//! The layout archive's working buffers (`runtime::ui::archive`), handed between the archive hook and the retire thread without a
//! lock. The hook runs on the game's task workers (`archive_alloc`: 112 of 124 calls off MainThread, hot-slots hook-thread table
//! 2026-11-09); a lock the retire thread can hold would let the game starve the holder while the hook waits for it (the rc.6 freeze
//! class, `tests/lock_order.rs`).
//!
//! Each buffer has one state word, and every step is one compare-exchange by the only side that may take it, so neither side waits:
//! `EMPTY` -(boot, `prepare`)-> `FREE` -(hook, `claim`)-> `BUSY` -(hook, `Claim::park`)-> `PARKED` -(retire thread, `retire_expired`
//! after the grace)-> `FREE`. A claim dropped without parking goes straight back to `FREE`. The bytes keep their boot capacity:
//! nothing here allocates after `prepare`.
use std::cell::UnsafeCell;
use std::sync::atomic::{
    AtomicU64, AtomicU8, AtomicUsize,
    Ordering::{Acquire, Relaxed, Release},
};

const EMPTY: u8 = 0;
const FREE: u8 = 1;
const BUSY: u8 = 2;
const PARKED: u8 = 3;
const RETIRING: u8 = 4;

struct Slot {
    state: AtomicU8,
    handed_out_ms: AtomicU64,
    tag: AtomicUsize,
    bytes: UnsafeCell<Vec<u8>>,
}

impl Slot {
    const fn new() -> Self {
        Self { state: AtomicU8::new(EMPTY), handed_out_ms: AtomicU64::new(0), tag: AtomicUsize::new(0), bytes: UnsafeCell::new(Vec::new()) }
    }

    fn take(&self, from: u8, to: u8) -> bool {
        self.state.compare_exchange(from, to, Acquire, Relaxed).is_ok()
    }
}

pub struct Slots<const N: usize> {
    slots: [Slot; N],
}

// SAFETY: a slot's bytes are touched only by the thread whose compare-exchange moved it to `BUSY` or `RETIRING` (`prepare` too), until
// that thread's release store moves it on; the atomics order those hand-overs.
unsafe impl<const N: usize> Sync for Slots<N> {}

impl<const N: usize> Default for Slots<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Slots<N> {
    pub const fn new() -> Self {
        Self { slots: [const { Slot::new() }; N] }
    }

    /// Boot: give every empty slot a buffer of `capacity` bytes.
    pub fn prepare(&self, capacity: usize) {
        for slot in &self.slots {
            if slot.take(EMPTY, RETIRING) {
                // SAFETY: this thread moved the slot to RETIRING; nobody else touches its bytes.
                unsafe { (*slot.bytes.get()).reserve_exact(capacity) };
                slot.state.store(FREE, Release);
            }
        }
    }

    /// A free buffer, empty, with its boot capacity; `None` when every buffer is out. Never waits.
    pub fn claim(&self) -> Option<Claim<'_, N>> {
        let index = self.slots.iter().position(|slot| slot.take(FREE, BUSY))?;
        let mut claim = Claim { slots: self, index };
        claim.bytes().clear();
        Some(claim)
    }

    /// Whether a handed-out buffer still waits for its grace.
    pub fn parked(&self) -> bool {
        self.slots.iter().any(|slot| slot.state.load(Acquire) == PARKED)
    }

    /// The retire thread: every parked buffer handed out at least `grace_ms` before `now_ms` goes back to `FREE`, after
    /// `released(tag, len, handed_out_ms)`. Never waits.
    pub fn retire_expired(&self, now_ms: u64, grace_ms: u64, mut released: impl FnMut(usize, usize, u64)) {
        for slot in &self.slots {
            if slot.state.load(Acquire) != PARKED || now_ms.saturating_sub(slot.handed_out_ms.load(Relaxed)) < grace_ms {
                continue;
            }
            if !slot.take(PARKED, RETIRING) {
                continue;
            }
            // SAFETY: this thread moved the slot to RETIRING; nobody else touches its bytes.
            let bytes = unsafe { &mut *slot.bytes.get() };
            let (tag, len, at) = (slot.tag.load(Relaxed), bytes.len(), slot.handed_out_ms.load(Relaxed));
            bytes.clear();
            slot.state.store(FREE, Release);
            released(tag, len, at);
        }
    }
}

/// One claimed buffer, exclusive until it is parked or dropped (dropped: straight back to `FREE`).
pub struct Claim<'a, const N: usize> {
    slots: &'a Slots<N>,
    index: usize,
}

impl<const N: usize> Claim<'_, N> {
    pub fn bytes(&mut self) -> &mut Vec<u8> {
        // SAFETY: the slot is BUSY and this claim moved it there.
        unsafe { &mut *self.slots.slots[self.index].bytes.get() }
    }

    /// Hand the buffer out: it stays as it is until `retire_expired` sees its grace over.
    pub fn park(self, now_ms: u64, tag: usize) {
        let slot = &self.slots.slots[self.index];
        slot.handed_out_ms.store(now_ms, Relaxed);
        slot.tag.store(tag, Relaxed);
        slot.state.store(PARKED, Release);
        std::mem::forget(self);
    }
}

impl<const N: usize> Drop for Claim<'_, N> {
    fn drop(&mut self) {
        self.bytes().clear();
        self.slots.slots[self.index].state.store(FREE, Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_buffer_goes_out_parks_for_its_grace_and_comes_back() {
        let slots: Slots<2> = Slots::new();
        assert!(slots.claim().is_none(), "no buffer before boot");
        slots.prepare(64);
        let mut first = slots.claim().unwrap();
        first.bytes().extend_from_slice(&[7; 32]);
        let pointer = first.bytes().as_ptr();
        assert!(first.bytes().capacity() >= 64);
        first.park(1_000, 1);
        assert!(slots.parked());
        let second = slots.claim().unwrap();
        assert!(slots.claim().is_none(), "both buffers are out");
        drop(second);
        let again = slots.claim().unwrap();
        drop(again);
        slots.retire_expired(5_999, 5_000, |_, _, _| panic!("early"));
        let mut seen = Vec::new();
        slots.retire_expired(6_000, 5_000, |tag, len, at| seen.push((tag, len, at)));
        assert_eq!(seen, [(1, 32, 1_000)]);
        assert!(!slots.parked());
        let mut reused = [slots.claim().unwrap(), slots.claim().unwrap()];
        assert!(reused.iter_mut().any(|claim| claim.bytes().as_ptr() == pointer), "the same boot buffer comes back");
        assert!(reused.iter_mut().all(|claim| claim.bytes().is_empty()));
    }

    #[test]
    fn the_hook_and_the_retire_thread_never_share_a_buffer() {
        use std::sync::Arc;
        let slots: Arc<Slots<2>> = Arc::new(Slots::new());
        slots.prepare(16);
        let retire = {
            let slots = Arc::clone(&slots);
            std::thread::spawn(move || {
                let mut released = 0;
                for now in 0..20_000u64 {
                    slots.retire_expired(now, 1, |_, len, _| {
                        assert_eq!(len, 16);
                        released += 1;
                    });
                }
                released
            })
        };
        let mut handed = 0;
        for now in 0..20_000u64 {
            if let Some(mut claim) = slots.claim() {
                assert!(claim.bytes().is_empty());
                claim.bytes().extend_from_slice(&[1; 16]);
                claim.park(now, 0);
                handed += 1;
            }
        }
        let released = retire.join().unwrap();
        slots.retire_expired(u64::MAX, 0, |_, _, _| {});
        assert!(handed > 0 && released <= handed);
        let all: Vec<_> = (0..2).filter_map(|_| slots.claim()).collect();
        assert_eq!(all.len(), 2, "every buffer is free again");
    }
}
