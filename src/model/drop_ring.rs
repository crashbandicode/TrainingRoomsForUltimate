//! [`DropRing`]: one producer thread hands small values (`W` words) to one consumer thread through `N` preallocated slots; neither
//! ever waits. When the ring is full the producer drops the oldest value and counts it, so the newest requests always get through
//! (the lock-free runtime, `artifacts/phase23/TASK-OPUS-rc6-lockfree.md`: "a full queue drops the oldest and counts it, never
//! blocks").
//!
//! `head` is the next value to take and `tail` the next to fill; both only grow. The producer fills `slots[tail % N]` and then
//! publishes `tail` (Release). On a full ring it first moves `head` past the oldest value with a compare-exchange, the same step
//! the consumer takes to claim it, so exactly one of them gets each value. The consumer copies `slots[head % N]` and then claims it
//! with that compare-exchange; if the producer dropped it meanwhile (and may be overwriting the slot), the claim fails and the copy,
//! possibly torn, is thrown away. The words are atomics, so a torn copy is never undefined behaviour.
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::{AcqRel, Acquire, Relaxed, Release}};

pub struct DropRing<const W: usize, const N: usize> {
    slots: [[AtomicU64; W]; N],
    head: AtomicU64,
    tail: AtomicU64,
    dropped: AtomicU32,
}

impl<const W: usize, const N: usize> Default for DropRing<W, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const N: usize> DropRing<W, N> {
    pub const fn new() -> Self {
        Self {
            slots: [const { [const { AtomicU64::new(0) }; W] }; N],
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            dropped: AtomicU32::new(0),
        }
    }

    /// The producer (one thread): queue `value`; on a full ring the oldest value is dropped and counted. Never waits.
    pub fn push(&self, value: [u64; W]) {
        let tail = self.tail.load(Relaxed);
        loop {
            let head = self.head.load(Acquire);
            if tail - head < N as u64 {
                break;
            }
            if self.head.compare_exchange(head, head + 1, AcqRel, Acquire).is_ok() {
                self.dropped.fetch_add(1, Relaxed);
                break;
            }
        }
        for (word, v) in self.slots[(tail % N as u64) as usize].iter().zip(value) {
            word.store(v, Relaxed);
        }
        self.tail.store(tail + 1, Release);
    }

    /// The consumer (one thread): the oldest value still queued, or `None`. Never waits.
    pub fn pop(&self) -> Option<[u64; W]> {
        loop {
            let head = self.head.load(Acquire);
            if head == self.tail.load(Acquire) {
                return None;
            }
            let slot = &self.slots[(head % N as u64) as usize];
            let value: [u64; W] = std::array::from_fn(|i| slot[i].load(Relaxed));
            // The claim (Release orders the copy before it): fails only when the producer dropped this value meanwhile.
            if self.head.compare_exchange(head, head + 1, AcqRel, Acquire).is_ok() {
                return Some(value);
            }
        }
    }

    /// Values dropped on a full ring since the last call.
    pub fn take_dropped(&self) -> u32 {
        self.dropped.swap(0, Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn values_come_out_in_order_and_a_full_ring_drops_the_oldest() {
        let ring: DropRing<2, 4> = DropRing::new();
        assert_eq!(ring.pop(), None);
        for n in 0..6u64 {
            ring.push([n, n * 10]);
        }
        assert_eq!(ring.take_dropped(), 2, "two oldest dropped");
        let got: Vec<u64> = std::iter::from_fn(|| ring.pop()).map(|v| v[0]).collect();
        assert_eq!(got, vec![2, 3, 4, 5], "the newest four, oldest first");
        assert_eq!(ring.pop(), None);
        ring.push([9, 90]);
        assert_eq!(ring.pop(), Some([9, 90]));
    }

    /// A producer far faster than the consumer: every value the consumer takes is whole (both words agree), in increasing order,
    /// and every value is either taken or counted as dropped.
    #[test]
    fn a_racing_producer_and_consumer_never_tear_or_lose_a_value() {
        let ring: Arc<DropRing<4, 3>> = Arc::new(DropRing::new());
        const TOTAL: u64 = 200_000;
        let producer = {
            let ring = Arc::clone(&ring);
            std::thread::spawn(move || {
                for n in 1..=TOTAL {
                    ring.push([n, n ^ 0xffff, n, n.wrapping_mul(3)]);
                }
            })
        };
        let mut taken = 0u64;
        let mut last = 0u64;
        let mut dropped = 0u64;
        loop {
            // Sampled before the pop: a finished producer and an empty ring mean nothing more comes.
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
        dropped += u64::from(ring.take_dropped());
        producer.join().unwrap();
        assert_eq!(taken + dropped, TOTAL, "taken {taken} + dropped {dropped}");
        assert_eq!(last, TOTAL, "the newest value always gets through");
    }
}
