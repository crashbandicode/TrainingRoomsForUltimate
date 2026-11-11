//! A bounded channel with `std::sync::mpsc::sync_channel`'s surface whose non-blocking calls never wait on the other side.
//!
//! rc.8 "Add New…" hang (2026-11-11, worker A's finding A3): std's `try_recv` is not wait-free. When a sender has claimed
//! a slot (moved the tail) but not yet published it, std's array `start_recv` loops on `Backoff::spin_light` until the
//! sender's stamp lands, and `try_send` does the same on a receiver mid-read. The game's MainThread polled the Mii close
//! hand-off that way while it had preempted the menu worker inside that window on core 0; Horizon does not time-slice
//! equal or lower priorities back in, so the MainThread spun for good. Here a slot the other side holds is "not ready":
//! `try_recv` returns `Empty` and `try_send` returns `Full`, and the caller tries again on its next poll.
//!
//! - Each slot moves `EMPTY -> WRITING -> FULL -> EMPTY`; a sender claims an empty slot with one compare-exchange and the
//!   one receiver empties a full one. A post number per message keeps posting order: a message whose predecessor is
//!   still being written waits for a later call (never for the writer).
//! - One receiver ([`Receiver`] is not `Clone` or `Sync`); any number of senders ([`SyncSender`] is `Clone`).
//! - [`Receiver::recv`] blocks, for plugin threads only. It sleeps on a `hos_lock::Event` that posts set once a blocking
//!   receiver exists, so a channel only polled pays no supervisor call per post.
//! - Disconnection as std's: `try_send` after the receiver is gone returns `Disconnected`; `try_recv`/`recv` once every
//!   sender is gone and nothing is queued return `Disconnected`/`RecvError`.
//! - The slots are allocated once, in [`sync_channel`] (boot), as std's are; no call allocates afterwards.
use std::cell::{Cell, UnsafeCell};
use std::fmt;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering::{AcqRel, Acquire, Relaxed, Release, SeqCst}};
use std::sync::Arc;

const EMPTY: u8 = 0;
const WRITING: u8 = 1;
const FULL: u8 = 2;

struct Slot<T> {
    state: AtomicU8,
    value: UnsafeCell<MaybeUninit<(u64, T)>>,
}

struct Inner<T> {
    slots: Box<[Slot<T>]>,
    /// The next post number (senders).
    posted: AtomicU64,
    /// The next post number to deliver (written by the receiver only).
    next: AtomicU64,
    senders: AtomicUsize,
    receiver: AtomicBool,
    /// A receiver has blocked in `recv`: posts set `wake` from then on.
    blocking: AtomicBool,
    wake: hos_lock::Event,
}

// SAFETY: a slot's value is written only by the sender that moved it to WRITING, and read and emptied only by the one
// receiver while it is FULL; FULL is published with a release store and observed with an acquire (SeqCst) load, and only
// the receiver moves a FULL slot back to EMPTY. Values move between threads, so `T: Send` is required, as for std's.
unsafe impl<T: Send> Send for Inner<T> {}
unsafe impl<T: Send> Sync for Inner<T> {}

impl<T> Drop for Inner<T> {
    fn drop(&mut self) {
        for slot in self.slots.iter_mut() {
            if *slot.state.get_mut() == FULL {
                // SAFETY: the last owner; a FULL slot holds an initialized value nobody took.
                unsafe { slot.value.get_mut().assume_init_drop() };
            }
        }
    }
}

/// The sending half (any thread, any number of clones).
pub struct SyncSender<T> {
    inner: Arc<Inner<T>>,
}

/// The receiving half (one thread at a time).
pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
    _not_sync: PhantomData<Cell<()>>,
}

/// std's own error values (plain enums; no channel code), so callers that match on `std::sync::mpsc::TryRecvError` keep
/// working: `TrySendError::Full` when every slot is full or held, `TryRecvError::Empty` when nothing is ready (no message,
/// or the next one is still being written), the `Disconnected`s and `RecvError` as std's.
pub use std::sync::mpsc::{RecvError, TryRecvError, TrySendError};

/// A channel of `capacity` slots (at least one).
pub fn sync_channel<T>(capacity: usize) -> (SyncSender<T>, Receiver<T>) {
    assert!(capacity > 0, "a nowait channel needs at least one slot");
    let slots = (0..capacity)
        .map(|_| Slot { state: AtomicU8::new(EMPTY), value: UnsafeCell::new(MaybeUninit::uninit()) })
        .collect();
    let inner = Arc::new(Inner {
        slots,
        posted: AtomicU64::new(0),
        next: AtomicU64::new(0),
        senders: AtomicUsize::new(1),
        receiver: AtomicBool::new(true),
        blocking: AtomicBool::new(false),
        wake: hos_lock::Event::new(),
    });
    (SyncSender { inner: Arc::clone(&inner) }, Receiver { inner, _not_sync: PhantomData })
}

impl<T> SyncSender<T> {
    /// Any thread: hand `value` over, or give it back. Never waits on the receiver or another sender.
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        let Some(writing) = self.claim() else {
            return Err(if self.inner.receiver.load(Acquire) { TrySendError::Full(value) } else { TrySendError::Disconnected(value) });
        };
        writing.publish(value);
        Ok(())
    }

    /// Claim an empty slot (the first half of `try_send`): `None` when the receiver is gone or every slot is full or held.
    fn claim(&self) -> Option<Writing<'_, T>> {
        if !self.inner.receiver.load(Acquire) {
            return None;
        }
        let slot = self.inner.slots.iter().find(|slot| slot.state.compare_exchange(EMPTY, WRITING, Acquire, Relaxed).is_ok())?;
        Some(Writing { inner: &self.inner, slot, published: false })
    }
}

/// A claimed slot between the claim and the publication (the window std's receiver spins on).
struct Writing<'a, T> {
    inner: &'a Inner<T>,
    slot: &'a Slot<T>,
    published: bool,
}

impl<T> Writing<'_, T> {
    fn publish(self, value: T) {
        let number = self.take_number();
        self.fill(number, value);
    }

    /// The post number, taken after the claim: a refused post uses none, so the receiver never waits on a gap that
    /// cannot fill. A claim that took a number must be filled.
    fn take_number(&self) -> u64 {
        self.inner.posted.fetch_add(1, AcqRel)
    }

    fn fill(mut self, number: u64, value: T) {
        // SAFETY: the slot is this sender's between WRITING and FULL.
        unsafe { (*self.slot.value.get()).write((number, value)) };
        self.slot.state.store(FULL, SeqCst);
        self.published = true;
        if self.inner.blocking.load(SeqCst) {
            self.inner.wake.set();
        }
    }
}

impl<T> Drop for Writing<'_, T> {
    fn drop(&mut self) {
        if !self.published {
            // Claimed and never written: give the slot back.
            self.slot.state.store(EMPTY, Release);
        }
    }
}

impl<T> Clone for SyncSender<T> {
    fn clone(&self) -> Self {
        self.inner.senders.fetch_add(1, Relaxed);
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T> Drop for SyncSender<T> {
    fn drop(&mut self) {
        if self.inner.senders.fetch_sub(1, AcqRel) == 1 {
            // A receiver blocked in `recv` now sees the disconnection.
            self.inner.wake.set();
        }
    }
}

impl<T> Receiver<T> {
    /// The receiver: the next message in posting order, or `Empty` when there is none or it is still being written. Never
    /// waits on a sender.
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let inner = &*self.inner;
        // Read before the scan: once no sender is left none can come back, and every post happened before the last
        // sender's release, so a scan after this load sees all of them.
        let gone = inner.senders.load(Acquire) == 0;
        let next = inner.next.load(Relaxed);
        let mut held = false;
        for slot in inner.slots.iter() {
            match slot.state.load(SeqCst) {
                FULL => {
                    // SAFETY: a FULL slot is initialized and no sender touches it; reading its number does not take it.
                    if unsafe { (*slot.value.get()).assume_init_ref().0 } == next {
                        // SAFETY: as above; the receiver owns a FULL slot until it stores EMPTY.
                        let (_, value) = unsafe { (*slot.value.get()).assume_init_read() };
                        slot.state.store(EMPTY, Release);
                        inner.next.store(next + 1, Relaxed);
                        return Ok(value);
                    }
                    held = true;
                }
                WRITING => held = true,
                _ => {}
            }
        }
        if gone && !held { Err(TryRecvError::Disconnected) } else { Err(TryRecvError::Empty) }
    }

    /// Plugin threads only: the next message, sleeping until one is posted; `RecvError` once every sender is gone and
    /// nothing is queued. Never call it on a game thread.
    pub fn recv(&self) -> Result<T, RecvError> {
        if !self.inner.blocking.load(Relaxed) {
            self.inner.blocking.store(true, SeqCst);
        }
        loop {
            match self.try_recv() {
                Ok(value) => return Ok(value),
                Err(TryRecvError::Disconnected) => return Err(RecvError),
                Err(TryRecvError::Empty) => self.inner.wake.wait(None),
            }
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.inner.receiver.store(false, Release);
    }
}

impl<T> fmt::Debug for SyncSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SyncSender { .. }")
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Receiver { .. }")
    }
}


#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use std::sync::Barrier;
    use std::time::{Duration, Instant};

    #[test]
    fn delivers_in_posting_order_up_to_capacity_and_refuses_the_rest() {
        let (tx, rx) = sync_channel(2);
        assert!(tx.try_send(1).is_ok());
        assert!(tx.try_send(2).is_ok());
        assert!(matches!(tx.try_send(3), Err(TrySendError::Full(3))));
        assert_eq!(rx.try_recv(), Ok(1));
        assert!(tx.try_send(4).is_ok());
        assert_eq!(rx.try_recv(), Ok(2));
        assert_eq!(rx.try_recv(), Ok(4));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }

    /// The rc.8 hang's interleaving: a sender stopped between its claim and its publication. std's `try_recv` spins here
    /// until the sender runs again; this one says "not ready" at once, every time, and delivers after the publication.
    #[test]
    fn a_writer_paused_mid_write_leaves_the_reader_not_ready_and_never_spinning() {
        let (tx, rx) = sync_channel(2);
        let writing = tx.claim().expect("an empty slot");
        let started = Instant::now();
        for _ in 0..100_000 {
            assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
        }
        // 100,000 polls finish in milliseconds; a poll that waited for the writer would never return.
        assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
        writing.publish(7);
        assert_eq!(rx.try_recv(), Ok(7));
    }

    /// A later message waits behind an earlier one still being written; neither side waits on the other.
    #[test]
    fn a_message_behind_an_unpublished_predecessor_waits_for_a_later_poll() {
        let (tx, rx) = sync_channel(3);
        let first = tx.claim().expect("slot");
        // Claimed but not numbered: another sender's message takes number 0 and is delivered past it.
        assert!(tx.try_send(20).is_ok());
        assert_eq!(rx.try_recv(), Ok(20));
        // Numbered (1) but not written: the next message (2) waits for it, and the receiver is told "not ready".
        let number = first.take_number();
        assert!(tx.try_send(30).is_ok());
        for _ in 0..1_000 {
            assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
        }
        first.fill(number, 10);
        assert_eq!(rx.try_recv(), Ok(10));
        assert_eq!(rx.try_recv(), Ok(30));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn a_full_slot_the_receiver_has_not_emptied_makes_the_sender_refuse_not_wait() {
        let (tx, rx) = sync_channel(1);
        assert!(tx.try_send(1).is_ok());
        let started = Instant::now();
        for n in 0..100_000 {
            assert!(matches!(tx.try_send(n), Err(TrySendError::Full(_))));
        }
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(rx.try_recv(), Ok(1));
        assert!(tx.try_send(2).is_ok());
    }

    #[test]
    fn a_claim_dropped_unwritten_frees_its_slot() {
        let (tx, rx) = sync_channel(1);
        drop(tx.claim().expect("slot"));
        assert!(tx.try_send(5).is_ok());
        assert_eq!(rx.try_recv(), Ok(5));
    }

    #[test]
    fn disconnection_follows_std() {
        let (tx, rx) = sync_channel::<u8>(1);
        let tx2 = tx.clone();
        assert!(tx.try_send(1).is_ok());
        drop(tx);
        drop(tx2);
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
        assert_eq!(rx.recv(), Err(RecvError));
        let (tx, rx) = sync_channel::<u8>(1);
        drop(rx);
        assert!(matches!(tx.try_send(9), Err(TrySendError::Disconnected(9))));
    }

    #[test]
    fn queued_values_are_dropped_with_the_channel() {
        let value = Arc::new(());
        let (tx, rx) = sync_channel(2);
        assert!(tx.try_send(Arc::clone(&value)).is_ok());
        assert!(tx.try_send(Arc::clone(&value)).is_ok());
        assert_eq!(Arc::strong_count(&value), 3);
        drop((tx, rx));
        assert_eq!(Arc::strong_count(&value), 1);
    }

    #[test]
    fn a_blocking_receiver_wakes_for_each_post_and_for_the_last_sender() {
        let (tx, rx) = sync_channel(2);
        let reader = std::thread::spawn(move || {
            let mut got = Vec::new();
            while let Ok(value) = rx.recv() {
                got.push(value);
            }
            got
        });
        for n in 0..200u32 {
            let mut value = n;
            loop {
                match tx.try_send(value) {
                    Ok(()) => break,
                    Err(TrySendError::Full(back)) => {
                        value = back;
                        std::thread::yield_now();
                    }
                    Err(TrySendError::Disconnected(_)) => panic!("receiver gone"),
                }
            }
        }
        drop(tx);
        assert_eq!(reader.join().unwrap(), (0..200).collect::<Vec<_>>());
    }

    #[test]
    fn many_senders_and_one_polling_receiver_lose_and_duplicate_nothing() {
        const SENDERS: u32 = 4;
        const EACH: u32 = 5_000;
        let (tx, rx) = sync_channel(3);
        let start = Arc::new(Barrier::new(SENDERS as usize + 1));
        let threads: Vec<_> = (0..SENDERS)
            .map(|s| {
                let (tx, start) = (tx.clone(), Arc::clone(&start));
                std::thread::spawn(move || {
                    start.wait();
                    for n in 0..EACH {
                        let mut value = s * EACH + n;
                        loop {
                            match tx.try_send(value) {
                                Ok(()) => break,
                                Err(TrySendError::Full(back)) => {
                                    value = back;
                                    std::thread::yield_now();
                                }
                                Err(TrySendError::Disconnected(_)) => panic!("receiver gone"),
                            }
                        }
                    }
                })
            })
            .collect();
        drop(tx);
        start.wait();
        let mut seen = vec![false; (SENDERS * EACH) as usize];
        let mut last = vec![None::<u32>; SENDERS as usize];
        let mut count = 0;
        loop {
            match rx.try_recv() {
                Ok(value) => {
                    assert!(!seen[value as usize], "duplicate {value}");
                    seen[value as usize] = true;
                    // Each sender's own messages arrive in its posting order.
                    let sender = (value / EACH) as usize;
                    assert!(last[sender].is_none_or(|previous| previous < value), "sender {sender} out of order");
                    last[sender] = Some(value);
                    count += 1;
                }
                Err(TryRecvError::Empty) => std::thread::yield_now(),
                Err(TryRecvError::Disconnected) => break,
            }
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(count, SENDERS * EACH);
    }
    }
}
