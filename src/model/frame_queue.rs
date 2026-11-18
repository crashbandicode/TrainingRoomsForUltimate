//! The frames a playing role hands from the recording worker to the hook that applies them.
//!
//! The worker runs on its own thread and is not tied to the game's frames: a tick can come late (it
//! reads the card) or twice between two game frames. One slot between it and the hook dropped a frame
//! when the worker wrote twice and left the AI's own controls in place when it wrote none. The worker
//! now fills this queue ahead of the hook, in file order, and the hook takes exactly one frame per game
//! frame, so the macro advances with the game and not with the worker. Fixed storage: the hooks take
//! from it without allocating.

/// Frames buffered ahead of the hook, about one second at 60 fps.
pub const AHEAD: usize = 64;

/// A ring of packed frames (`rr_host`'s `pack`) in the order the run plays them.
#[derive(Clone, Copy, Debug)]
pub struct FrameQueue {
    words: [u64; AHEAD],
    head: usize,
    len: usize,
}

impl FrameQueue {
    pub(crate) const fn new() -> Self {
        Self { words: [0; AHEAD], head: 0, len: 0 }
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn room(&self) -> usize {
        AHEAD - self.len
    }

    /// A new run, or a stopped one: drop what is buffered.
    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }

    /// Append the run's next frame. `false` when the queue is full: a frame is never written over.
    pub fn push(&mut self, word: u64) -> bool {
        if self.len == AHEAD {
            return false;
        }
        self.words[(self.head + self.len) % AHEAD] = word;
        self.len += 1;
        true
    }

    /// The oldest frame, once.
    pub fn pop(&mut self) -> Option<u64> {
        if self.len == 0 {
            return None;
        }
        let word = self.words[self.head];
        self.head = (self.head + 1) % AHEAD;
        self.len -= 1;
        Some(word)
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn frames_come_out_once_in_order() {
        let mut q = FrameQueue::new();
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
            ("an_interrupted_pop_rejects_the_reused_slot", an_interrupted_pop_rejects_the_reused_slot as fn()),
            ("the_producer_and_consumer_keep_every_frame_in_order", the_producer_and_consumer_keep_every_frame_in_order as fn()),
        ]);
    }
    }
}
