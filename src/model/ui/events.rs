//! The existing UI input ring, carrying one complete physical poll per slot.
//! Its sole producer holds the input adapter's try-lock; its sole consumer is
//! the menu worker. Session cancellation invalidates generations, never writes
//! the ring as a second producer and cannot be dropped by a full ring.
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use super::adapter::Step;
use super::menu::{input::Button, Menu};

const BUTTONS: [Button; 12] = [Button::Up, Button::Right, Button::Down, Button::Left,
    Button::A, Button::B, Button::X, Button::Y, Button::L, Button::R, Button::ZL, Button::ZR];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Packet {
    pub held: u16,
    pub pressed: u16,
    navigation: u16,
}

impl Packet {
    pub fn from_step(step: &Step) -> Self {
        Self { held: step.held, pressed: step.pressed,
            navigation: step.presses().fold(0, |bits, b| bits | (1 << b as u8)) }
    }

    /// Capture consumes a whole poll before its preview/navigation controls.
    pub fn presses(self, menu: &mut Menu) -> impl Iterator<Item = Button> + use<> {
        let bits = if menu.chord_poll(self.held, self.pressed) { 0 } else { self.navigation };
        BUTTONS.into_iter().filter(move |b| bits & (1 << *b as u8) != 0)
    }

    fn encode(self) -> u64 {
        u64::from(self.held) | (u64::from(self.pressed) << 16) | (u64::from(self.navigation) << 32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event { Open, Poll(Packet) }

impl Event {
    fn encode(self) -> u64 { match self { Self::Open => 1 << 63, Self::Poll(p) => p.encode() } }
    fn decode(word: u64) -> Self {
        if word >> 63 != 0 { Self::Open } else {
            Self::Poll(Packet { held: word as u16, pressed: (word >> 16) as u16, navigation: (word >> 32) as u16 })
        }
    }
}

struct Slot { word: AtomicU64, generation: AtomicU32 }
impl Slot {
    const fn new() -> Self { Self { word: AtomicU64::new(0), generation: AtomicU32::new(0) } }
}

pub struct Ring<const N: usize> {
    slots: [Slot; N],
    head: AtomicUsize,
    tail: AtomicUsize,
    generation: AtomicU32,
}

impl<const N: usize> Ring<N> {
    pub const fn new() -> Self {
        assert!(N.is_power_of_two());
        Self { slots: [const { Slot::new() }; N], head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0), generation: AtomicU32::new(0) }
    }

    pub fn generation(&self) -> u32 { self.generation.load(Ordering::Acquire) }

    /// Any cancellation caller. The worker drops stale queued events even at capacity.
    pub fn reset(&self) { self.generation.fetch_add(1, Ordering::AcqRel); }

    /// Only the serialized input producer calls push. A false result is a
    /// discontinuity: cancel the transient menu, never confirm a partial poll.
    pub fn push(&self, generation: u32, event: Event) -> bool {
        if generation != self.generation() { return false; }
        let head = self.head.load(Ordering::Relaxed);
        if head.wrapping_sub(self.tail.load(Ordering::Acquire)) >= N { return false; }
        let slot = &self.slots[head % N];
        slot.word.store(event.encode(), Ordering::Relaxed);
        slot.generation.store(generation, Ordering::Relaxed);
        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    /// Only the worker pops. It rechecks generation before dispatch; a reset
    /// during push leaves an explicitly stale packet, not a torn physical poll.
    pub fn pop(&self) -> Option<(u32, Event)> {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail == self.head.load(Ordering::Acquire) { return None; }
        let slot = &self.slots[tail % N];
        let event = (slot.generation.load(Ordering::Relaxed), Event::decode(slot.word.load(Ordering::Relaxed)));
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(event)
    }
}

impl<const N: usize> Default for Ring<N> { fn default() -> Self { Self::new() } }
