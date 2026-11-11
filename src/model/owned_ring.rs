//! [`OwnedRing`]: one producer at a time hands owned values (heap included) to one consumer thread through `N` preallocated
//! slots; neither ever waits (the lock-free runtime, `artifacts/phase23/TASK-OPUS-rc6-lockfree.md`: a game hook posts what a plugin
//! thread writes and frees). A Lamport ring: the producer fills `slots[tail % N]` and then publishes `tail` (Release); the consumer
//! moves `slots[head % N]` out and then publishes `head` (Release). Each slot is touched by one side at a time: the producer only
//! while `tail - head < N`, the consumer only while `head < tail`.
//!
//! A full ring refuses the new value and counts it ([`take_refused`](OwnedRing::take_refused)): the value comes back to the
//! producer. It cannot drop the oldest instead: that value is the consumer's to move out and free, and taking it back would free it
//! on the producer (a game hook) or race the consumer's move.
//!
//! Contract: producers take turns (the caller serialises them, e.g. under a lock only producers take); one consumer thread.
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering::{Acquire, Relaxed, Release}};

pub struct OwnedRing<T, const N: usize> {
    slots: [UnsafeCell<MaybeUninit<T>>; N],
    head: AtomicUsize,
    tail: AtomicUsize,
    refused: AtomicU32,
}

// SAFETY: a slot is written only by the producer while it is outside `head..tail` and read only by the consumer while it is inside;
// `tail` and `head` are published with Release and observed with Acquire, so each side sees the other's writes to the slots it
// hands over. Values move between threads, so `T: Send`.
unsafe impl<T: Send, const N: usize> Sync for OwnedRing<T, N> {}
unsafe impl<T: Send, const N: usize> Send for OwnedRing<T, N> {}

impl<T, const N: usize> Default for OwnedRing<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> OwnedRing<T, N> {
    pub const fn new() -> Self {
        Self {
            slots: [const { UnsafeCell::new(MaybeUninit::uninit()) }; N],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            refused: AtomicU32::new(0),
        }
    }

    /// The producer: queue `value`, or give it back (counted) when the ring is full. Never waits, allocates or frees.
    pub fn push(&self, value: T) -> Result<(), T> {
        let tail = self.tail.load(Relaxed);
        if tail.wrapping_sub(self.head.load(Acquire)) >= N {
            self.refused.fetch_add(1, Relaxed);
            return Err(value);
        }
        // SAFETY: `tail - head < N`: the slot is outside the consumer's range, and producers take turns.
        unsafe { (*self.slots[tail % N].get()).write(value) };
        self.tail.store(tail.wrapping_add(1), Release);
        Ok(())
    }

    /// The consumer: the oldest value queued, or `None`. Never waits.
    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Relaxed);
        if head == self.tail.load(Acquire) {
            return None;
        }
        // SAFETY: `head < tail`: the producer filled this slot and published it; only this consumer moves it out, once.
        let value = unsafe { (*self.slots[head % N].get()).assume_init_read() };
        self.head.store(head.wrapping_add(1), Release);
        Some(value)
    }

    /// Values refused on a full ring since the last call.
    pub fn take_refused(&self) -> u32 {
        self.refused.swap(0, Relaxed)
    }
}

impl<T, const N: usize> Drop for OwnedRing<T, N> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn values_come_out_in_order_and_a_full_ring_refuses_and_counts() {
        let ring: OwnedRing<Vec<u32>, 3> = OwnedRing::new();
        assert_eq!(ring.pop(), None);
        for n in 0..3 {
            assert!(ring.push(vec![n]).is_ok());
        }
        assert_eq!(ring.push(vec![9]), Err(vec![9]), "full: the value comes back");
        assert_eq!(ring.take_refused(), 1);
        assert_eq!(ring.take_refused(), 0);
        assert_eq!(ring.pop(), Some(vec![0]));
        assert!(ring.push(vec![3]).is_ok());
        let rest: Vec<Vec<u32>> = std::iter::from_fn(|| ring.pop()).collect();
        assert_eq!(rest, vec![vec![1], vec![2], vec![3]]);
    }

    /// Values left in the ring are dropped with it (once each).
    #[test]
    fn a_dropped_ring_drops_what_it_holds_once() {
        let counter = Arc::new(());
        {
            let ring: OwnedRing<Arc<()>, 4> = OwnedRing::new();
            for _ in 0..3 {
                ring.push(Arc::clone(&counter)).unwrap();
            }
            drop(ring.pop());
            assert_eq!(Arc::strong_count(&counter), 3);
        }
        assert_eq!(Arc::strong_count(&counter), 1);
    }

    /// A producer far faster than the consumer: every value arrives whole and in order, and each is either taken or refused.
    #[test]
    fn a_racing_producer_and_consumer_never_lose_or_tear_a_value() {
        const TOTAL: u64 = 100_000;
        let ring: Arc<OwnedRing<Box<[u64; 4]>, 8>> = Arc::new(OwnedRing::new());
        let producer = {
            let ring = Arc::clone(&ring);
            std::thread::spawn(move || {
                let mut refused = 0u64;
                for n in 1..=TOTAL {
                    if ring.push(Box::new([n, n ^ 0xffff, n, n.wrapping_mul(3)])).is_err() {
                        refused += 1;
                    }
                }
                refused
            })
        };
        let (mut taken, mut last) = (0u64, 0u64);
        loop {
            let finished = producer.is_finished();
            match ring.pop() {
                Some(v) => {
                    assert!(v[1] == v[0] ^ 0xffff && v[2] == v[0] && v[3] == v[0].wrapping_mul(3), "a torn value: {v:?}");
                    assert!(v[0] > last, "out of order: {} after {last}", v[0]);
                    last = v[0];
                    taken += 1;
                }
                None if finished => break,
                None => {}
            }
        }
        let refused = producer.join().unwrap();
        assert_eq!(taken + refused, TOTAL, "taken {taken} + refused {refused}");
        assert_eq!(u64::from(ring.take_refused()), refused);
    }
    }
}
