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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
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

    #[test]
    fn an_unchanged_value_is_not_rewritten() {
        let cell: SeqCell<2> = SeqCell::new();
        cell.write(&[1, 2]);
        let seq = cell.seq.load(Relaxed);
        cell.write(&[1, 2]);
        assert_eq!(cell.seq.load(Relaxed), seq);
        assert_eq!(cell.read(1), Some([1, 2]));
    }
}
