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
    pub const fn new() -> Self {
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

impl Default for FrameQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn frames_come_out_once_in_order() {
        let mut q = FrameQueue::new();
        for i in 0..10u64 {
            assert!(q.push(i + 100));
        }
        let out: Vec<u64> = std::iter::from_fn(|| q.pop()).collect();
        assert_eq!(out, (100..110).collect::<Vec<u64>>());
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn a_full_queue_refuses_and_keeps_every_frame() {
        let mut q = FrameQueue::new();
        for i in 0..AHEAD as u64 {
            assert!(q.push(i));
        }
        assert!(!q.push(999), "full: refused, not written over");
        assert_eq!(q.pop(), Some(0));
        assert!(q.push(7));
        let out: Vec<u64> = std::iter::from_fn(|| q.pop()).collect();
        assert_eq!(out.len(), AHEAD);
        assert_eq!(out.first(), Some(&1));
        assert_eq!(out.last(), Some(&7));
    }

    #[test]
    fn clear_drops_the_buffered_frames() {
        let mut q = FrameQueue::new();
        assert!(q.push(1));
        assert!(q.push(2));
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.room(), AHEAD);
        assert!(q.push(9));
        assert_eq!(q.pop(), Some(9));
    }
    }
}
