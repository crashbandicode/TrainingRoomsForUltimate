//! The frames a playing role hands from the recording worker to the hook that applies them.
//!
//! The worker runs on its own thread and is not tied to the game's frames: a tick can come late (it
//! reads the card) or twice between two game frames. One slot between it and the hook dropped a frame
//! when the worker wrote twice and left the AI's own controls in place when it wrote none. The worker
//! now fills this queue ahead of the hook, in file order, and the hook takes exactly one frame per game
//! frame, so the macro advances with the game and not with the worker. Fixed storage: the hooks take
//! from it without allocating.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Frames buffered ahead of the hook, about one second at 60 fps.
pub const AHEAD: usize = 64;

/// One Menu worker publishes and clears. One game-thread hook consumes each role.
pub struct FrameQueue {
    words: [AtomicU64; AHEAD],
    head: AtomicUsize,
    tail: AtomicUsize,
    generation_start: AtomicUsize,
}

impl FrameQueue {
    pub(crate) const fn new() -> Self {
        Self {
            words: [const { AtomicU64::new(0) }; AHEAD],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            generation_start: AtomicUsize::new(0),
        }
    }

    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let generation = self.generation_start.load(Ordering::Acquire);
        Self::live_head(head, generation) == self.tail.load(Ordering::Acquire)
    }

    /// The worker checks room before advancing playback. Only that worker fills it.
    pub fn room(&self) -> usize {
        let tail = self.tail.load(Ordering::Relaxed);
        AHEAD - tail.wrapping_sub(self.producer_head())
    }

    /// Start a new generation at the published tail. Only the hook writes head.
    pub fn clear(&self) {
        let tail = self.tail.load(Ordering::Relaxed);
        self.generation_start.store(tail, Ordering::Release);
    }

    pub fn generation(&self) -> usize {
        self.generation_start.load(Ordering::Acquire)
    }

    /// Append the run's next frame. A full queue keeps every published frame.
    pub fn push(&self, word: u64) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail.wrapping_sub(self.producer_head()) == AHEAD {
            return false;
        }
        self.words[tail % AHEAD].store(word, Ordering::Release);
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    /// Take the oldest live frame once. Retry only when a clear invalidates the slot.
    pub fn pop(&self) -> Option<u64> {
        self.pop_tagged().map(|(word, _)| word)
    }

    pub fn pop_tagged(&self) -> Option<(u64, usize)> {
        let head = self.head.load(Ordering::Relaxed);
        loop {
            let generation = self.generation_start.load(Ordering::Acquire);
            let at = Self::live_head(head, generation);
            if at == self.tail.load(Ordering::Acquire) {
                return None;
            }
            if let Some(word) = self.read_live(at, generation) {
                self.head.store(at.wrapping_add(1), Ordering::Release);
                return Some((word, generation));
            }
        }
    }

    fn producer_head(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let generation = self.generation_start.load(Ordering::Relaxed);
        Self::live_head(head, generation)
    }

    fn live_head(head: usize, generation: usize) -> usize {
        // Sequences are ordered across integer wrap. A generation lasts less than
        // half the sequence space on the 64-bit targets.
        if (generation.wrapping_sub(head) as isize) > 0 {
            generation
        } else {
            head
        }
    }

    fn read_live(&self, at: usize, generation: usize) -> Option<u64> {
        let word = self.words[at % AHEAD].load(Ordering::Acquire);
        // If this load sees a reused slot, it also sees the clear that preceded
        // that slot's Release store. Recheck before committing the pop.
        (self.generation_start.load(Ordering::Acquire) == generation).then_some(word)
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn frames_come_out_once_in_order() {
        let q = FrameQueue::new();
        for i in 0..10u64 {
            assert!(q.push(i + 100));
        }
        let out: Vec<u64> = std::iter::from_fn(|| q.pop()).collect();
        assert_eq!(out, (100..110).collect::<Vec<u64>>());
        assert_eq!(q.pop(), None);
    }

    fn a_full_queue_refuses_and_wraps_in_order() {
        let q = FrameQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.pop(), None);
        for i in 0..AHEAD as u64 {
            assert!(q.push(i));
        }
        assert_eq!(q.room(), 0);
        assert!(!q.push(999), "a full queue must keep every published frame");
        assert_eq!(q.pop(), Some(0));
        assert!(q.push(AHEAD as u64));
        let out: Vec<u64> = std::iter::from_fn(|| q.pop()).collect();
        assert_eq!(out, (1..=AHEAD as u64).collect::<Vec<_>>());
        assert!(q.is_empty());
    }

    fn clear_drops_the_buffered_frames() {
        let q = FrameQueue::new();
        for i in 0..AHEAD as u64 {
            assert!(q.push(i));
        }
        assert_eq!(q.pop(), Some(0));
        assert!(q.push(AHEAD as u64));
        let head = q.head.load(Ordering::Relaxed);
        q.clear();
        assert_eq!(q.head.load(Ordering::Relaxed), head);
        assert!(q.is_empty());
        assert_eq!(q.room(), AHEAD);
        for i in 0..AHEAD as u64 {
            assert!(q.push(100 + i), "the new generation can use the whole ring");
        }
        let out: Vec<u64> = std::iter::from_fn(|| q.pop()).collect();
        assert_eq!(out, (100..100 + AHEAD as u64).collect::<Vec<_>>());
        assert_eq!(q.pop(), None);
    }

    fn popped_frames_carry_the_generation_they_were_pushed_under() {
        let q = FrameQueue::new();
        let first = q.generation();
        assert!(q.push(10));
        assert_eq!(q.pop_tagged(), Some((10, first)));
        q.clear();
        let second = q.generation();
        assert_ne!(first, second);
        assert!(q.push(20));
        assert_eq!(q.pop_tagged(), Some((20, second)));
        assert_eq!(q.pop_tagged(), None);
    }

    fn an_interrupted_pop_rejects_the_reused_slot() {
        let q = FrameQueue::new();
        for i in 0..AHEAD as u64 {
            assert!(q.push(i));
        }
        let at = q.head.load(Ordering::Relaxed);
        let generation = q.generation_start.load(Ordering::Acquire);

        // The hook selected the old slot. The worker clears and reuses that exact slot
        // before the hook loads its payload.
        q.clear();
        assert!(q.push(900));
        assert_eq!(q.read_live(at, generation), None);
        assert_eq!(q.head.load(Ordering::Relaxed), at);
        assert_eq!(q.pop(), Some(900));
        assert_eq!(q.pop(), None);
    }

    fn the_producer_and_consumer_keep_every_frame_in_order() {
        use std::time::{Duration, Instant};

        const FRAMES: u64 = 30_000;
        let q = FrameQueue::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for i in 0..FRAMES {
                    while !q.push(i) {
                        assert!(Instant::now() < deadline, "producer stalled at {i}");
                        std::thread::yield_now();
                    }
                }
            });
            for i in 0..FRAMES {
                let frame = loop {
                    if let Some(frame) = q.pop() {
                        break frame;
                    }
                    assert!(Instant::now() < deadline, "consumer stalled at {i}");
                    std::thread::yield_now();
                };
                assert_eq!(frame, i);
            }
        });
        assert!(q.is_empty());
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("frames_come_out_once_in_order", frames_come_out_once_in_order as fn()),
            ("a_full_queue_refuses_and_wraps_in_order", a_full_queue_refuses_and_wraps_in_order as fn()),
            ("clear_drops_the_buffered_frames", clear_drops_the_buffered_frames as fn()),
            ("popped_frames_carry_the_generation_they_were_pushed_under", popped_frames_carry_the_generation_they_were_pushed_under as fn()),
            ("an_interrupted_pop_rejects_the_reused_slot", an_interrupted_pop_rejects_the_reused_slot as fn()),
            ("the_producer_and_consumer_keep_every_frame_in_order", the_producer_and_consumer_keep_every_frame_in_order as fn()),
        ]);
    }
    }
}
