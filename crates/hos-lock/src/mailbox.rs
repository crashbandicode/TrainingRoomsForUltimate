// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! [`Mailbox`]: any thread posts small `Copy` messages to one consumer thread; nobody waits. `N` fixed slots, each claimed by a
//! compare-exchange (empty -> writing -> full -> reading -> empty), a sequence number per post so the consumer gets them in posting
//! order, and a counted drop when every slot is full. Fixed size, no heap: a game hook can post, and the consumer (a plugin thread)
//! drains.
//!
//! Contract: one consumer thread calls [`Mailbox::drain`]; any number of threads may post.

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering::{AcqRel, Acquire, Relaxed, Release}};

const EMPTY: u8 = 0;
const WRITING: u8 = 1;
const FULL: u8 = 2;

pub struct Mailbox<T: Copy, const N: usize> {
    slots: [UnsafeCell<MaybeUninit<(u64, T)>>; N],
    state: [AtomicU8; N],
    seq: AtomicU64,
    /// The next sequence number to deliver (the consumer's).
    next: AtomicU64,
    dropped: AtomicU32,
}

// SAFETY: a slot's cell is written only by the poster that moved it to WRITING, and read only by the one consumer while it is FULL;
// FULL is published with Release and observed with Acquire, and only the consumer moves a FULL slot back to EMPTY.
unsafe impl<T: Copy + Send, const N: usize> Sync for Mailbox<T, N> {}

impl<T: Copy, const N: usize> Default for Mailbox<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy, const N: usize> Mailbox<T, N> {
    pub const fn new() -> Self {
        Self {
            slots: [const { UnsafeCell::new(MaybeUninit::uninit()) }; N],
            state: [const { AtomicU8::new(EMPTY) }; N],
            seq: AtomicU64::new(0),
            next: AtomicU64::new(0),
            dropped: AtomicU32::new(0),
        }
    }

    /// Post `message` (any thread). Never waits; returns false and counts a drop when every slot is full.
    pub fn post(&self, message: T) -> bool {
        for (i, state) in self.state.iter().enumerate() {
            if state.compare_exchange(EMPTY, WRITING, Acquire, Relaxed).is_ok() {
                // The number is taken after the claim: a full mailbox uses none, so the consumer never waits on a gap that cannot fill.
                let seq = self.seq.fetch_add(1, AcqRel);
                // SAFETY: the slot is ours between WRITING and FULL.
                unsafe { (*self.slots[i].get()).write((seq, message)) };
                state.store(FULL, Release);
                return true;
            }
        }
        self.dropped.fetch_add(1, Relaxed);
        false
    }

    /// The consumer: hand the posted messages to `f` in posting order. A message whose predecessor is still being written stays in
    /// its slot until a later drain (never waits). Returns how many were delivered.
    pub fn drain(&self, mut f: impl FnMut(T)) -> usize {
        let mut full: [(u64, usize); N] = [(0, 0); N];
        let mut n = 0;
        for (i, state) in self.state.iter().enumerate() {
            if state.load(Acquire) == FULL {
                // SAFETY: a FULL slot is written and no poster touches it (posters claim EMPTY slots only); reading its sequence number
                // does not take it.
                full[n] = (unsafe { (*self.slots[i].get()).assume_init_ref().0 }, i);
                n += 1;
            }
        }
        let full = &mut full[..n];
        full.sort_unstable_by_key(|(seq, _)| *seq);
        let mut next = self.next.load(Relaxed);
        let mut delivered = 0;
        for (seq, i) in full.iter() {
            if *seq != next {
                break;
            }
            // SAFETY: FULL slot, see above; the consumer owns it until it stores EMPTY.
            let (_, message) = unsafe { (*self.slots[*i].get()).assume_init_read() };
            self.state[*i].store(EMPTY, Release);
            f(message);
            next += 1;
            delivered += 1;
        }
        self.next.store(next, Relaxed);
        delivered
    }
    /// Drops since the last call (a full mailbox).
    pub fn take_dropped(&self) -> u32 {
        self.dropped.swap(0, Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn messages_come_out_in_posting_order_and_a_full_box_counts_drops() {
        let mailbox: Mailbox<u32, 4> = Mailbox::new();
        for i in 0..4 {
            assert!(mailbox.post(i));
        }
        assert!(!mailbox.post(99));
        assert_eq!(mailbox.take_dropped(), 1);
        let mut got = Vec::new();
        assert_eq!(mailbox.drain(|m| got.push(m)), 4);
        assert_eq!(got, vec![0, 1, 2, 3]);
        assert_eq!(mailbox.drain(|_| panic!("empty")), 0);
    }

    #[test]
    fn many_posting_threads_lose_nothing_but_counted_drops() {
        let mailbox: Arc<Mailbox<(u8, u32), 8>> = Arc::new(Mailbox::new());
        let done = Arc::new(AtomicBool::new(false));
        let consumer = {
            let (mailbox, done) = (mailbox.clone(), done.clone());
            thread::spawn(move || {
                let mut received = [0u32; 4];
                let mut last = [None::<u32>; 4];
                loop {
                    let finished = done.load(SeqCst);
                    mailbox.drain(|(who, n)| {
                        // Per poster, messages arrive in its own order.
                        if let Some(previous) = last[who as usize] {
                            assert!(n > previous, "out of order for poster {who}");
                        }
                        last[who as usize] = Some(n);
                        received[who as usize] += 1;
                    });
                    if finished && mailbox.drain(|(who, _)| received[who as usize] += 1) == 0 {
                        return received;
                    }
                }
            })
        };
        let posters: Vec<_> = (0..4u8)
            .map(|who| {
                let mailbox = mailbox.clone();
                thread::spawn(move || {
                    let mut posted = 0u32;
                    for n in 0..5000u32 {
                        if mailbox.post((who, n)) {
                            posted += 1;
                        }
                    }
                    posted
                })
            })
            .collect();
        let posted: Vec<u32> = posters.into_iter().map(|p| p.join().unwrap()).collect();
        done.store(true, SeqCst);
        let received = consumer.join().unwrap();
        assert_eq!(received.to_vec(), posted, "every accepted post is received exactly once");
        let dropped = mailbox.take_dropped();
        assert_eq!(posted.iter().sum::<u32>() + dropped, 20_000);
    }
}
