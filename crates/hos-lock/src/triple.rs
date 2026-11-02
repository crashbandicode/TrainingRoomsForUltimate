// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! [`TripleBuffer`]: one writer thread hands values to one reader thread; neither ever waits. Three slots: the writer fills its back
//! slot and swaps it with the middle one (marked fresh); the reader swaps its front slot with the middle one when that is fresh. Each
//! side touches only its own slot plus one atomic swap, so a game hook can be the writer (or the reader) without waiting on the other
//! thread. Fixed size, no heap: `T` is `Copy`.
//!
//! Contract: exactly one thread writes and exactly one thread reads (a second writer or reader would share a slot). The reader always
//! gets the latest complete value; intermediate values may be skipped.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU8, Ordering::{AcqRel, Acquire, Relaxed}};

const FRESH: u8 = 0b100;
const INDEX: u8 = 0b011;

pub struct TripleBuffer<T: Copy> {
    slots: [UnsafeCell<T>; 3],
    /// The middle slot's index, plus `FRESH` when the writer put a value there the reader has not taken.
    middle: AtomicU8,
    /// The writer's slot (touched by the writer only).
    back: AtomicU8,
    /// The reader's slot (touched by the reader only).
    front: AtomicU8,
}

// SAFETY: the writer only writes `slots[back]`, the reader only reads `slots[front]`, and the middle slot changes hands through one
// atomic swap (Release on publish, Acquire on take), so no slot is written while read. One writer and one reader (the contract).
unsafe impl<T: Copy + Send> Sync for TripleBuffer<T> {}

impl<T: Copy> TripleBuffer<T> {
    pub const fn new(value: T) -> Self {
        Self {
            slots: [UnsafeCell::new(value), UnsafeCell::new(value), UnsafeCell::new(value)],
            middle: AtomicU8::new(1),
            back: AtomicU8::new(2),
            front: AtomicU8::new(0),
        }
    }

    /// The writer: hand `value` over. Never waits.
    pub fn write(&self, value: T) {
        let back = self.back.load(Relaxed) & INDEX;
        // SAFETY: `back` is the writer's own slot.
        unsafe { *self.slots[back as usize].get() = value };
        let previous = self.middle.swap(back | FRESH, AcqRel);
        self.back.store(previous & INDEX, Relaxed);
    }

    /// The reader: the latest value handed over, and whether it is new since the previous read. Never waits.
    pub fn read(&self) -> (T, bool) {
        let mut fresh = false;
        if self.middle.load(Acquire) & FRESH != 0 {
            let front = self.front.load(Relaxed) & INDEX;
            let previous = self.middle.swap(front, AcqRel);
            self.front.store(previous & INDEX, Relaxed);
            fresh = true;
        }
        let front = self.front.load(Relaxed) & INDEX;
        // SAFETY: `front` is the reader's own slot.
        (unsafe { *self.slots[front as usize].get() }, fresh)
    }

    /// The reader, without a copy: `f` sees the latest value by reference (the reader's own slot) and whether it is new. Never waits.
    /// `f` must not read this buffer again (the slot it borrows would change hands).
    pub fn with_latest<R>(&self, f: impl FnOnce(&T, bool) -> R) -> R {
        let mut fresh = false;
        if self.middle.load(Acquire) & FRESH != 0 {
            let front = self.front.load(Relaxed) & INDEX;
            let previous = self.middle.swap(front, AcqRel);
            self.front.store(previous & INDEX, Relaxed);
            fresh = true;
        }
        let front = self.front.load(Relaxed) & INDEX;
        // SAFETY: `front` is the reader's own slot; the writer never touches it, and it changes hands only on the reader's next read.
        f(unsafe { &*self.slots[front as usize].get() }, fresh)
    }

    /// The reader: is a new value waiting (no slot changes hands)?
    pub fn has_new(&self) -> bool {
        self.middle.load(Acquire) & FRESH != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
    use std::sync::Arc;
    use std::thread;

    #[derive(Clone, Copy)]
    struct Wide([u64; 24]);

    #[test]
    fn the_reader_gets_whole_values_in_order_and_the_last_one() {
        let buffer = Arc::new(TripleBuffer::new(Wide([0; 24])));
        let done = Arc::new(AtomicBool::new(false));
        let reader = {
            let (buffer, done) = (buffer.clone(), done.clone());
            thread::spawn(move || {
                let mut last = 0;
                let mut fresh_reads = 0u32;
                loop {
                    let finished = done.load(SeqCst);
                    let (v, fresh) = buffer.read();
                    assert!(v.0.iter().all(|x| *x == v.0[0]), "torn value");
                    assert!(v.0[0] >= last, "went backwards");
                    last = v.0[0];
                    fresh_reads += u32::from(fresh);
                    if finished {
                        // One more read after the writer finished: the last value is there.
                        let (v, _) = buffer.read();
                        return (v.0[0], fresh_reads);
                    }
                }
            })
        };
        for i in 1..=20_000u64 {
            buffer.write(Wide([i; 24]));
        }
        done.store(true, SeqCst);
        let (last, fresh_reads) = reader.join().unwrap();
        assert_eq!(last, 20_000);
        assert!(fresh_reads > 0);
    }

    #[test]
    fn a_read_without_a_new_value_repeats_the_last_one() {
        let buffer = TripleBuffer::new(7u32);
        assert_eq!(buffer.read(), (7, false));
        buffer.write(8);
        assert!(buffer.has_new());
        assert_eq!(buffer.read(), (8, true));
        assert_eq!(buffer.read(), (8, false));
        buffer.write(9);
        buffer.write(10);
        assert_eq!(buffer.read(), (10, true));
        buffer.write(11);
        assert_eq!(buffer.with_latest(|v, fresh| (*v, fresh)), (11, true));
        assert_eq!(buffer.with_latest(|v, fresh| (*v, fresh)), (11, false));
    }
}
