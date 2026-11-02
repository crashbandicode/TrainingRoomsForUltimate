// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! [`Published`]: a value that a writer thread replaces and reader threads copy out without ever waiting on the
//! writer: two slots, the index of the current one, and a reader count per slot.
//!
//! - **Read** (`read`, `read_if_newer`): count in on the current slot, check it is still current, copy, count out.
//!   The check fails only if a writer switched slots between the two loads; the reader then tries the new slot.
//!   A writer switches at most once per value and must first wait for the other slot's readers to leave, so the
//!   retries are bounded by the number of values published meanwhile. A reader never waits for a writer.
//! - **Write** (`publish`): writers take turns on a lock, wait (sleeping, never spinning) until no reader is left on
//!   the slot that is not current, fill it, make it current and bump the generation. Only threads that may wait call
//!   it: a plugin's menu worker, loaders, boot, never a game hook.
//!
//! Fixed size, no heap: `T` is `Copy` and lives in the two slots.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering::{Acquire, Relaxed, Release, SeqCst}};

use crate::Mutex;

pub struct Published<T: Copy> {
    slots: [UnsafeCell<T>; 2],
    readers: [AtomicU32; 2],
    current: AtomicU32,
    generation: AtomicU32,
    writer: Mutex<()>,
}

// SAFETY: a slot is written only while it is not current and has no reader (see `publish`), and read only while it
// is current and counted; `T: Send` values move between threads by copy.
unsafe impl<T: Copy + Send> Sync for Published<T> {}

impl<T: Copy> Published<T> {
    pub const fn new(value: T) -> Self {
        Self {
            slots: [UnsafeCell::new(value), UnsafeCell::new(value)],
            readers: [AtomicU32::new(0), AtomicU32::new(0)],
            current: AtomicU32::new(0),
            generation: AtomicU32::new(0),
            writer: Mutex::new(()),
        }
    }

    /// The number of values published so far (0 before the first `publish`).
    #[inline]
    pub fn generation(&self) -> u32 {
        self.generation.load(Acquire)
    }

    /// A copy of the current value. Never waits.
    #[inline]
    pub fn read(&self) -> T {
        loop {
            let i = (self.current.load(Acquire) & 1) as usize;
            self.readers[i].fetch_add(1, SeqCst);
            if self.current.load(SeqCst) as usize == i {
                // SAFETY: slot `i` is current and counted: no writer touches it until the count drops.
                let value = unsafe { *self.slots[i].get() };
                self.readers[i].fetch_sub(1, Release);
                return value;
            }
            self.readers[i].fetch_sub(1, Release);
        }
    }

    /// The current value if it is newer than `seen` (which is then updated). One atomic load when nothing changed.
    #[inline]
    pub fn read_if_newer(&self, seen: &AtomicU32) -> Option<T> {
        let generation = self.generation();
        if generation == seen.load(Relaxed) {
            return None;
        }
        // The generation is bumped after the switch, so the value read now is at least as new as `generation`.
        let value = self.read();
        seen.store(generation, Relaxed);
        Some(value)
    }

    /// Make `value` the current one. May wait (sleeping): call it only where a thread may wait.
    pub fn publish(&self, value: T) {
        let _turn = self.writer.lock();
        let next = ((self.current.load(Relaxed) & 1) ^ 1) as usize;
        while self.readers[next].load(SeqCst) != 0 {
            // A reader counted on a slot that is no longer current: it leaves after one check or one copy.
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        // SAFETY: slot `next` is not current and has no reader; readers that count on it from now on find it is not
        // current and leave without reading. Writers take turns on `writer`.
        unsafe { *self.slots[next].get() = value };
        self.current.store(next as u32, SeqCst);
        self.generation.fetch_add(1, Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use std::sync::Arc;
    use std::thread;

    #[derive(Clone, Copy)]
    struct Wide([u64; 32]);

    #[test]
    fn two_writers_and_six_readers_never_tear_a_value() {
        let cell = Arc::new(Published::new(Wide([0; 32])));
        let stop = Arc::new(AtomicBool::new(false));
        let torn = Arc::new(AtomicUsize::new(0));
        let mut readers = Vec::new();
        for _ in 0..6 {
            let (cell, stop, torn) = (cell.clone(), stop.clone(), torn.clone());
            readers.push(thread::spawn(move || {
                let seen = AtomicU32::new(0);
                while !stop.load(SeqCst) {
                    let v = cell.read();
                    if v.0.iter().any(|x| *x != v.0[0]) {
                        torn.fetch_add(1, SeqCst);
                    }
                    if let Some(n) = cell.read_if_newer(&seen) {
                        if n.0.iter().any(|x| *x != n.0[0]) {
                            torn.fetch_add(1, SeqCst);
                        }
                    }
                }
            }));
        }
        let writers: Vec<_> = (0..2)
            .map(|_| {
                let cell = cell.clone();
                thread::spawn(move || {
                    for i in 0..1500u64 {
                        cell.publish(Wide([i; 32]));
                    }
                })
            })
            .collect();
        for w in writers {
            w.join().unwrap();
        }
        stop.store(true, SeqCst);
        for r in readers {
            r.join().unwrap();
        }
        assert_eq!(torn.load(SeqCst), 0);
        assert_eq!(cell.generation(), 3000);
    }

    #[test]
    fn one_writer_publishes_in_order_and_read_if_newer_reports_each_change_once() {
        let cell = Published::new(1u32);
        let seen = AtomicU32::new(0);
        assert_eq!(cell.read_if_newer(&seen), None);
        cell.publish(2);
        assert_eq!(cell.read_if_newer(&seen), Some(2));
        assert_eq!(cell.read_if_newer(&seen), None);
        cell.publish(3);
        cell.publish(4);
        assert_eq!(cell.read_if_newer(&seen), Some(4));
        assert_eq!(cell.read(), 4);
        assert_eq!(cell.readers[0].load(SeqCst) + cell.readers[1].load(SeqCst), 0);
    }

    #[test]
    fn a_single_writer_is_seen_in_order_by_concurrent_readers() {
        let cell = Arc::new(Published::new(Wide([0; 32])));
        let stop = Arc::new(AtomicBool::new(false));
        let readers: Vec<_> = (0..4)
            .map(|_| {
                let (cell, stop) = (cell.clone(), stop.clone());
                thread::spawn(move || {
                    let mut last = 0;
                    while !stop.load(SeqCst) {
                        let v = cell.read();
                        assert!(v.0.iter().all(|x| *x == v.0[0]), "torn");
                        assert!(v.0[0] >= last, "went backwards");
                        last = v.0[0];
                    }
                })
            })
            .collect();
        for i in 1..=2000u64 {
            cell.publish(Wide([i; 32]));
        }
        stop.store(true, SeqCst);
        for r in readers {
            r.join().unwrap();
        }
        assert_eq!(cell.read().0[0], 2000);
    }
}
