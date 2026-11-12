//! [`SeqCell`]: a small value one writer at a time replaces and any thread reads without a lock (a sequence lock over atomic words).
//! The writer never waits; a reader retries while a write is in progress and gives up after `tries` attempts instead of waiting.
//! rc.6 freeze class (`tests/lock_order.rs`): a plugin thread reads what game hooks keep under their own lock without taking it
//! (`native_body_bank`'s ready observation, read by the menu worker's roster re-plan).
use std::sync::atomic::{
    fence, AtomicU32, AtomicU64,
    Ordering::{Acquire, Relaxed, Release},
};

pub struct SeqCell<const W: usize> {
    seq: AtomicU32,
    words: [AtomicU64; W],
}

impl<const W: usize> Default for SeqCell<W> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize> SeqCell<W> {
    pub const fn new() -> Self {
        Self { seq: AtomicU32::new(0), words: [const { AtomicU64::new(0) }; W] }
    }

    /// Replace the value (all words zero at first). One writer at a time: the caller serialises writers (a lock it holds). Writes
    /// nothing when the value is unchanged.
    pub fn write(&self, value: &[u64; W]) {
        if self.words.iter().zip(value).all(|(word, v)| word.load(Relaxed) == *v) {
            return;
        }
        let seq = self.seq.load(Relaxed);
        self.seq.store(seq.wrapping_add(1), Relaxed);
        fence(Release);
        for (word, v) in self.words.iter().zip(value) {
            word.store(*v, Relaxed);
        }
        self.seq.store(seq.wrapping_add(2), Release);
    }

    /// The value, read whole; `None` when a write was in progress on every one of `tries` attempts.
    pub fn read(&self, tries: usize) -> Option<[u64; W]> {
        for _ in 0..tries {
            let before = self.seq.load(Acquire);
            if before & 1 == 1 {
                std::hint::spin_loop();
                continue;
            }
            let value: [u64; W] = std::array::from_fn(|i| self.words[i].load(Relaxed));
            fence(Acquire);
            if self.seq.load(Relaxed) == before {
                return Some(value);
            }
        }
        None
    }

    /// [`read`](Self::read) for up to `rounds` rounds of 64 tries, yielding the thread between rounds so a writer that was
    /// preempted mid-write can finish (never a lock). `None` only when every round met a write in progress.
    pub fn read_yielding(&self, rounds: usize) -> Option<[u64; W]> {
        for _ in 0..rounds {
            if let Some(value) = self.read(64) {
                return Some(value);
            }
            std::thread::yield_now();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use std::sync::Arc;

    fn a_reader_sees_whole_values_only() {
        let cell: Arc<SeqCell<8>> = Arc::new(SeqCell::new());
        assert_eq!(cell.read(1), Some([0; 8]));
        let writer = {
            let cell = Arc::clone(&cell);
            std::thread::spawn(move || {
                for n in 1..50_000u64 {
                    cell.write(&[n; 8]);
                }
            })
        };
        let mut seen = 0;
        while !writer.is_finished() {
            if let Some(value) = cell.read(1_000) {
                assert!(value.iter().all(|w| *w == value[0]), "a torn read: {value:?}");
                seen += 1;
            }
        }
        writer.join().unwrap();
        assert_eq!(cell.read(1), Some([49_999; 8]));
        assert!(seen > 0);
    }

    fn a_yielding_reader_outlasts_a_write_left_open() {
        let cell: Arc<SeqCell<2>> = Arc::new(SeqCell::new());
        cell.write(&[7, 7]);
        // A writer preempted between its marks: the sequence stays odd until it runs again.
        let seq = cell.seq.load(Relaxed);
        cell.seq.store(seq + 1, Relaxed);
        assert_eq!(cell.read(64), None, "a plain read gives up");
        let writer = {
            let cell = Arc::clone(&cell);
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(20));
                cell.words[0].store(8, Relaxed);
                cell.words[1].store(8, Relaxed);
                cell.seq.store(seq + 2, Release);
            })
        };
        assert_eq!(cell.read_yielding(1_000_000), Some([8, 8]));
        writer.join().unwrap();
    }

    fn an_unchanged_value_is_not_rewritten() {
        let cell: SeqCell<2> = SeqCell::new();
        cell.write(&[1, 2]);
        let seq = cell.seq.load(Relaxed);
        cell.write(&[1, 2]);
        assert_eq!(cell.seq.load(Relaxed), seq);
        assert_eq!(cell.read(1), Some([1, 2]));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_reader_sees_whole_values_only", a_reader_sees_whole_values_only as fn()),
            ("a_yielding_reader_outlasts_a_write_left_open", a_yielding_reader_outlasts_a_write_left_open as fn()),
            ("an_unchanged_value_is_not_rewritten", an_unchanged_value_is_not_rewritten as fn()),
        ]);
    }
    }
}
