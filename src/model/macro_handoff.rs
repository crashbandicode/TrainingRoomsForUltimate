//! Design C of the named-macro layer (SPEC-core-01 §4.3, lead ruling 2026-11-02): the panel model has one owner, the menu worker, and
//! the game-thread hooks never take its locks. What crosses between the two lives here, pure and host-tested:
//!
//! * `MacroQueries`: the `Copy` answers the hooks consult every frame (panel and replay visibility …),
//!   published by the worker after every model change and read wait-free by the hooks.
//! * `PlayBuffers<T>`: two frame buffers for named playback. The worker fills the one the hooks are not reading and publishes it; a
//!   hook reads one frame by value per call and never waits. The worker, which may sleep, waits for the readers of the old buffer to
//!   leave before it writes that buffer again.
//! * `LayerEvent`: what a game thread tells the worker (posted to a mailbox, drained by the worker in order).
//!
//! The buffers allocate only on the worker (`prepare`, `fill`); a hook's `push`/`frame_at` stay within the prepared capacity.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering::SeqCst};

/// The hook-side answers of the named layer (one snapshot, written by the worker after each model change).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MacroQueries {
    pub session: u64,
    /// Any panel of the layer is up (library, naming, clip editor, capture browser).
    pub panel_visible: bool,
    /// The clip editor is in the foreground (P1's controls are the editor's).
    pub clip_foreground: bool,
    /// The editor draft exists, including parking in a pending Mii reload.
    pub clip_editor_open: bool,
    /// A clip preview plays: the fighters run although the editor's panel is up.
    pub clip_previewing: bool,
    pub replay_loading: bool,
    pub browser_pending: bool,
    pub browser_release: bool,
    /// A keyboard request is open or pending.
    pub naming_pending: bool,
    /// The layer is suspended (a waiting-room scene left with a draft).
    pub suspended: bool,
    /// The placement editor's cross marker, if it shows one (x, y).
    pub marker: Option<(f32, f32)>,
}

/// What one physical P1 mapping may do with the published editor snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipInputRoute { Pass, Consume, Forward }

/// The worker arms close ownership before publishing its closed editor query.
/// Read that query first: a new closed snapshot must not follow an old arm read.
pub fn route_clip_input(
    read_queries: impl FnOnce() -> MacroQueries,
    poll_close: impl FnOnce() -> bool,
) -> ClipInputRoute {
    let queries = read_queries();
    queries.clip_input_route(poll_close())
}

impl MacroQueries {
    pub fn clip_input_route(&self, close_owned: bool) -> ClipInputRoute {
        if close_owned { ClipInputRoute::Consume }
        else if self.clip_foreground || self.replay_loading { ClipInputRoute::Forward }
        else { ClipInputRoute::Pass }
    }
    /// Nothing loaded, nothing visible: what the hooks see before the worker's first publish.
    pub const IDLE: MacroQueries = MacroQueries {
        session: 0,
        panel_visible: false,
        clip_foreground: false,
        clip_editor_open: false,
        clip_previewing: false,
        replay_loading: false,
        browser_pending: false,
        browser_release: false,
        naming_pending: false,
        suspended: false,
        marker: None,
    };
}

/// Two frame buffers for playback: the hooks read the published one, the worker fills the other.
pub struct PlayBuffers<T> {
    slots: [UnsafeCell<Vec<T>>; 2],
    /// The published buffer (0 or 1); `NONE` before the first publish.
    active: AtomicU8,
    /// Its length (frames).
    len: AtomicUsize,
    /// Hooks reading each buffer right now.
    readers: [AtomicU32; 2],
    /// Bumped twice on every publish, odd while it switches (a hook can tell that a new macro replaced the one it started).
    generation: AtomicU32,
}

const NONE: u8 = u8::MAX;

// SAFETY: a slot is written only by the single worker while it is unpublished and has no reader (see `fill`); hooks only read.
unsafe impl<T: Send> Sync for PlayBuffers<T> {}

impl<T: Copy> PlayBuffers<T> {
    pub const fn new() -> Self {
        Self {
            slots: [UnsafeCell::new(Vec::new()), UnsafeCell::new(Vec::new())],
            active: AtomicU8::new(NONE),
            len: AtomicUsize::new(0),
            readers: [AtomicU32::new(0), AtomicU32::new(0)],
            generation: AtomicU32::new(0),
        }
    }

    /// Worker: reserve `capacity` frames in both buffers (boot or a menu commit; the worker may allocate).
    ///
    /// # Safety
    /// Only the worker calls `prepare`, `fill` and `retract` (one writer).
    pub unsafe fn prepare(&self, capacity: usize) {
        for index in 0..2u8 {
            if self.active.load(SeqCst) == index {
                continue;
            }
            self.wait_unread(index);
            let slot = &mut *self.slots[usize::from(index)].get();
            if slot.capacity() < capacity {
                slot.reserve_exact(capacity - slot.len());
            }
        }
    }

    /// Worker: copy `frames` into the unpublished buffer and publish it. Returns the new generation.
    ///
    /// # Safety
    /// As `prepare`.
    pub unsafe fn fill(&self, frames: &[T]) -> u32 {
        let current = self.active.load(SeqCst);
        let next = if current == 0 { 1 } else { 0 };
        self.wait_unread(next);
        let slot = &mut *self.slots[usize::from(next)].get();
        slot.clear();
        slot.extend_from_slice(frames);
        // The generation is odd while the published buffer switches (a reader comparing two generations sees the switch).
        self.generation.fetch_add(1, SeqCst);
        self.len.store(0, SeqCst);
        self.active.store(next, SeqCst);
        self.len.store(frames.len(), SeqCst);
        self.generation.fetch_add(1, SeqCst) + 1
    }

    /// Worker: nothing is published any more (the macro was unloaded).
    ///
    /// # Safety
    /// As `prepare`.
    pub unsafe fn retract(&self) {
        self.generation.fetch_add(1, SeqCst);
        self.active.store(NONE, SeqCst);
        self.len.store(0, SeqCst);
        self.generation.fetch_add(1, SeqCst);
    }

    /// The worker's wait (it may sleep): no hook is reading buffer `index`.
    fn wait_unread(&self, index: u8) {
        while self.readers[usize::from(index)].load(SeqCst) != 0 {
            std::thread::yield_now();
        }
    }

    /// Hook: the published macro's length (0 when none).
    pub fn len(&self) -> usize {
        if self.active.load(SeqCst) == NONE {
            0
        } else {
            self.len.load(SeqCst)
        }
    }

    /// Hook: no macro is published.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Hook: the publish counter.
    pub fn generation(&self) -> u32 {
        self.generation.load(SeqCst)
    }

    /// Hook: frame `index` of the published macro, by value. Never waits: a publish that races the read is seen on the retry, and the
    /// worker does not write a buffer while this reader is counted on it.
    pub fn frame_at(&self, index: usize) -> Option<T> {
        for _ in 0..2 {
            let active = self.active.load(SeqCst);
            if active == NONE {
                return None;
            }
            let readers = &self.readers[usize::from(active)];
            readers.fetch_add(1, SeqCst);
            if self.active.load(SeqCst) != active {
                // The worker published the other buffer in between: it may write this one once we leave.
                readers.fetch_sub(1, SeqCst);
                continue;
            }
            // SAFETY: `active` is published and counted as read, so the worker does not write it (`fill` writes the other one and waits
            // for this count before it would write this one).
            let frame = unsafe { (&*self.slots[usize::from(active)].get()).get(index).copied() };
            readers.fetch_sub(1, SeqCst);
            return frame;
        }
        None
    }
}

impl<T: Copy> Default for PlayBuffers<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::{MacroQueries, PlayBuffers};

    fn the_idle_snapshot_has_nothing_loaded_or_visible() {
        let idle = MacroQueries::IDLE;
        assert!(!idle.panel_visible && !idle.clip_foreground && !idle.naming_pending);
    }

    fn a_published_macro_is_read_by_value_and_a_new_one_replaces_it() {
        let buffers = PlayBuffers::<u32>::new();
        assert_eq!((buffers.len(), buffers.frame_at(0)), (0, None));
        // SAFETY: this test is the only writer.
        unsafe {
            buffers.prepare(8);
            let g1 = buffers.fill(&[1, 2, 3]);
            assert_eq!((buffers.len(), buffers.frame_at(2), buffers.frame_at(3)), (3, Some(3), None));
            let g2 = buffers.fill(&[9; 5]);
            assert!(g2 > g1);
            assert_eq!((buffers.len(), buffers.frame_at(4)), (5, Some(9)));
            buffers.retract();
        }
        assert_eq!((buffers.len(), buffers.frame_at(0)), (0, None));
    }

    fn readers_never_see_a_torn_buffer_while_the_worker_republishes() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, Barrier};
        let buffers = Arc::new(PlayBuffers::<u64>::new());
        let stop = Arc::new(AtomicBool::new(false));
        let start = Arc::new(Barrier::new(4));
        // SAFETY: the main thread is the only writer.
        unsafe { buffers.prepare(256) };
        let readers: Vec<_> = (0..3)
            .map(|_| {
                let (buffers, stop, start) = (buffers.clone(), stop.clone(), start.clone());
                std::thread::spawn(move || {
                    let mut seen = 0u64;
                    start.wait();
                    loop {
                        let finished = stop.load(Ordering::Acquire);
                        // Every buffer the worker publishes holds one value in every frame: two reads of one macro agree.
                        let generation = buffers.generation();
                        if generation % 2 == 1 {
                            continue;
                        }
                        if let (Some(a), Some(b)) = (buffers.frame_at(0), buffers.frame_at(255)) {
                            if buffers.generation() == generation {
                                assert_eq!(a, b, "a frame of an older fill leaked into the published buffer");
                            }
                            seen += 1;
                        }
                        // A reader can be descheduled for the whole publication burst.
                        // Still sample the final stable publication before honoring stop;
                        // otherwise scheduler timing makes this test vacuous (seen == 0).
                        if finished { break; }
                    }
                    seen
                })
            })
            .collect();
        start.wait();
        for value in 0..2000u64 {
            // SAFETY: the only writer.
            unsafe { buffers.fill(&[value; 256]) };
        }
        stop.store(true, Ordering::Release);
        for reader in readers {
            assert!(reader.join().unwrap() > 0, "each reader must inspect a published buffer");
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_idle_snapshot_has_nothing_loaded_or_visible", the_idle_snapshot_has_nothing_loaded_or_visible as fn()),
            ("a_published_macro_is_read_by_value_and_a_new_one_replaces_it", a_published_macro_is_read_by_value_and_a_new_one_replaces_it as fn()),
            ("readers_never_see_a_torn_buffer_while_the_worker_republishes", readers_never_see_a_torn_buffer_while_the_worker_republishes as fn()),
        ]);
    }
    }
}
