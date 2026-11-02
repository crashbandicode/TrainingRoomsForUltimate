//! Design C of the named-macro layer (SPEC-core-01 §4.3, lead ruling 2026-11-02): the panel model has one owner, the menu worker, and
//! the game-thread hooks never take its locks. What crosses between the two lives here, pure and host-tested:
//!
//! * `MacroQueries`: the `Copy` answers the hooks consult every frame (Macro Source, capture/playback state, panel visibility …),
//!   published by the worker after every model change and read wait-free by the hooks.
//! * `PlayBuffers<T>`: two frame buffers for named playback. The worker fills the one the hooks are not reading and publishes it; a
//!   hook reads one frame by value per call and never waits. The worker, which may sleep, waits for the readers of the old buffer to
//!   leave before it writes that buffer again.
//! * `CaptureBuffer<T>`: one frame buffer the hook owns between the start of a named capture and its end, and the worker owns
//!   otherwise (it prepares it before a capture and reads the frames after one). Ownership moves through one atomic; neither side waits.
//! * `LayerEvent`: what a game thread tells the worker (posted to a mailbox, drained by the worker in order).
//!
//! The buffers allocate only on the worker (`prepare`, `fill`); a hook's `push`/`frame_at` stay within the prepared capacity.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering::SeqCst};

/// The hook-side answers of the named layer (one snapshot, written by the worker after each model change).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MacroQueries {
    pub session: u64,
    /// Macro Source is Named Library.
    pub source_is_named: bool,
    /// A named macro is loaded and its fighter is ready (the playback chord may start it).
    pub playback_allowed: bool,
    pub playback_len: u32,
    pub playback_facing: f32,
    pub playback_status: i32,
    pub playback_generation: u32,
    pub loaded_matches: bool,
    pub placement: Option<(f32, f32)>,
    pub velocity: Option<(f32, f32)>,
    /// The loaded macro plays on the current CPU whatever its fighter (no recorded-identity gate).
    pub plays_current: bool,
    /// The loaded macro's recorded fighter kind (-1: none), for the restore's identity gate.
    pub loaded_kind: i32,
    /// The loaded macro was recorded in the air (its restore starts in Fall).
    pub restore_in_air: bool,
    /// Any panel of the layer is up (library, naming, clip editor, capture browser).
    pub panel_visible: bool,
    /// The clip editor is in the foreground (P1's controls are the editor's).
    pub clip_foreground: bool,
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

impl MacroQueries {
    /// Nothing loaded, nothing visible: what the hooks see before the worker's first publish.
    pub const IDLE: MacroQueries = MacroQueries {
        session: 0,
        source_is_named: false,
        playback_allowed: false,
        playback_len: 0,
        playback_facing: 1.0,
        playback_status: 0,
        playback_generation: 0,
        loaded_matches: false,
        placement: None,
        velocity: None,
        plays_current: false,
        loaded_kind: -1,
        restore_in_air: false,
        panel_visible: false,
        clip_foreground: false,
        replay_loading: false,
        browser_pending: false,
        browser_release: false,
        naming_pending: false,
        suspended: false,
        marker: None,
    };
}

/// One message from a game thread to the worker. `Copy` and small, so posting allocates nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerEvent {
    /// The named capture ended with `len` frames in the capture buffer (`crop`: Recording Crop was on).
    CaptureFinished { len: u32, crop: bool },
    /// A named playback or a clip preview ended.
    PlaybackEnded,
    /// The playback chord asked for the loaded named macro (the worker requests the restore).
    RequestPlayback,
    /// A save-state restore finished and the layer's follow-up ran on the game thread (the worker refreshes its view).
    RestoreDone,
    /// The session ended (leaving Training, a hot swap, a lobby change): close transient panels.
    SessionCleared,
    /// One game frame passed with a practice CPU on stage (the worker's character and keyboard services may run).
    FrameTick,
}

/// One bounded numbered-slot transfer. The worker constructs a request, a game
/// callback executes it against its Recorder, and the worker consumes the reply.
/// Both directions use the same fixed payload: no allocation occurs in execute.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlotTransfer {
    slot: u8,
    frames: [crate::model::input_delay::MappedInputs; crate::model::recording::SLOT_FRAMES],
    len: u16,
    origin: crate::model::recording::Origin,
    pub result: SlotResult,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotResult {
    ExportRequest,
    ImportRequest,
    Exported,
    Imported,
    ExportFailed(crate::model::recording::ExportRefusal),
    ImportFailed(crate::model::recording::ImportRefusal),
}

impl SlotTransfer {
    pub fn export(slot: usize) -> Self {
        Self {
            slot: u8::try_from(slot).unwrap_or(u8::MAX),
            frames: [crate::model::input_delay::MappedInputs::NEUTRAL; crate::model::recording::SLOT_FRAMES],
            len: 0,
            origin: crate::model::recording::Origin::NONE,
            result: SlotResult::ExportRequest,
        }
    }

    pub fn import(slot: usize, frames: &[crate::model::input_delay::MappedInputs], origin: crate::model::recording::Origin)
        -> Result<Self, crate::model::recording::ImportRefusal>
    {
        use crate::model::recording::{ImportRefusal, SLOTS, SLOT_FRAMES};
        if slot >= SLOTS { return Err(ImportRefusal::SlotOutOfRange); }
        if frames.is_empty() { return Err(ImportRefusal::Empty); }
        if frames.len() > SLOT_FRAMES { return Err(ImportRefusal::TooLong(frames.len())); }
        let mut transfer = Self::export(slot);
        transfer.frames[..frames.len()].copy_from_slice(frames);
        transfer.len = frames.len() as u16;
        transfer.origin = origin;
        transfer.result = SlotResult::ImportRequest;
        Ok(transfer)
    }

    pub fn execute(&mut self, recorder: &mut crate::model::recording::Recorder) {
        match self.result {
            SlotResult::ExportRequest => match recorder.export_slot(self.slot) {
                Ok((frames, origin)) => {
                    self.frames[..frames.len()].copy_from_slice(frames);
                    self.len = frames.len() as u16;
                    self.origin = origin;
                    self.result = SlotResult::Exported;
                }
                Err(error) => self.result = SlotResult::ExportFailed(error),
            },
            SlotResult::ImportRequest => {
                self.result = match recorder.import_slot(self.slot, &self.frames[..usize::from(self.len)], self.origin) {
                    Ok(()) => SlotResult::Imported,
                    Err(error) => SlotResult::ImportFailed(error),
                };
            }
            _ => {}
        }
    }

    pub fn exported(&self) -> Option<(&[crate::model::input_delay::MappedInputs], crate::model::recording::Origin)> {
        (self.result == SlotResult::Exported).then(|| (&self.frames[..usize::from(self.len)], self.origin))
    }
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

/// Who owns the capture buffer.
const WORKER_READY: u8 = 0;
const HOOK: u8 = 1;
const WORKER_FULL: u8 = 2;
const WORKER_BUSY: u8 = 3;
const HOOK_BUSY: u8 = 4;

/// The named capture's frame buffer: the hook's from `begin` to `finish`, the worker's otherwise.
pub struct CaptureBuffer<T> {
    frames: UnsafeCell<Vec<T>>,
    owner: AtomicU8,
    /// Frames stored by the hook (published at `finish`).
    len: AtomicUsize,
}

// SAFETY: every frame access holds WORKER_BUSY or HOOK_BUSY exclusively. Ownership
// transitions use compare-exchange; competing hook calls fail immediately instead of aliasing.
unsafe impl<T: Send> Sync for CaptureBuffer<T> {}

impl<T: Copy + Default> CaptureBuffer<T> {
    pub const fn new() -> Self {
        Self { frames: UnsafeCell::new(Vec::new()), owner: AtomicU8::new(WORKER_READY), len: AtomicUsize::new(0) }
    }

    /// Worker: reserve `capacity` frames (only while the worker owns an empty buffer). False when the hook or a pending result holds it.
    pub fn prepare(&self, capacity: usize) -> bool {
        if self.owner.compare_exchange(WORKER_READY, WORKER_BUSY, SeqCst, SeqCst).is_err() {
            return false;
        }
        // SAFETY: WORKER_BUSY: only this worker call touches the buffer.
        let frames = unsafe { &mut *self.frames.get() };
        if frames.capacity() < capacity {
            frames.reserve_exact(capacity - frames.len());
        }
        self.owner.store(WORKER_READY, SeqCst);
        true
    }

    /// Hook: take the buffer for a capture (empty, prepared). False when the worker has not taken the last result yet.
    pub fn begin(&self) -> bool {
        if self.owner.compare_exchange(WORKER_READY, HOOK_BUSY, SeqCst, SeqCst).is_err() {
            return false;
        }
        // SAFETY: HOOK_BUSY excludes every other accessor while clearing the buffer.
        unsafe { (*self.frames.get()).clear() };
        self.len.store(0, SeqCst);
        self.owner.store(HOOK, SeqCst);
        true
    }

    /// Hook: the capture is running.
    pub fn capturing(&self) -> bool {
        self.owner.load(SeqCst) == HOOK
    }

    /// Hook: store frame `index` (frames in between are default-filled). False when not capturing or past the prepared capacity
    /// (the hook never grows the buffer).
    pub fn store(&self, index: usize, frame: T) -> bool {
        if self.owner.compare_exchange(HOOK, HOOK_BUSY, SeqCst, SeqCst).is_err() {
            return false;
        }
        // SAFETY: HOOK_BUSY is held exclusively for this bounded write.
        let frames = unsafe { &mut *self.frames.get() };
        if index >= frames.capacity() {
            self.owner.store(HOOK, SeqCst);
            return false;
        }
        if frames.len() <= index {
            frames.resize(index + 1, T::default());
        }
        frames[index] = frame;
        self.len.store(frames.len(), SeqCst);
        self.owner.store(HOOK, SeqCst);
        true
    }

    /// Hook: frame `index` was stored.
    pub fn has_frame(&self, index: usize) -> bool {
        self.owner.load(SeqCst) == HOOK && index < self.len.load(SeqCst)
    }

    /// Hook: frame `index` of the running capture.
    pub fn frame_at(&self, index: usize) -> Option<T> {
        if self.owner.compare_exchange(HOOK, HOOK_BUSY, SeqCst, SeqCst).is_err() {
            return None;
        }
        // SAFETY: HOOK_BUSY excludes writes and ownership transitions until the copy completes.
        let frame = unsafe { (&*self.frames.get()).get(index).copied() };
        self.owner.store(HOOK, SeqCst);
        frame
    }

    /// Hook: the capture ended; the buffer goes to the worker with its frames. Returns the frame count (None: not capturing).
    pub fn finish(&self) -> Option<usize> {
        self.owner.compare_exchange(HOOK, HOOK_BUSY, SeqCst, SeqCst).ok()?;
        let len = self.len.load(SeqCst);
        self.owner.store(WORKER_FULL, SeqCst);
        Some(len)
    }

    /// Hook: the capture was cancelled (a load request, a stop): the buffer goes back to the worker empty.
    pub fn cancel(&self) {
        if self.owner.compare_exchange(HOOK, WORKER_BUSY, SeqCst, SeqCst).is_ok() {
            self.len.store(0, SeqCst);
            self.owner.store(WORKER_READY, SeqCst);
        }
    }

    /// Worker: read the finished capture's frames, then the buffer is ready for the next capture. None when no result is waiting.
    pub fn take<R>(&self, read: impl FnOnce(&[T]) -> R) -> Option<R> {
        if self.owner.compare_exchange(WORKER_FULL, WORKER_BUSY, SeqCst, SeqCst).is_err() {
            return None;
        }
        // SAFETY: WORKER_BUSY: only this worker call touches the buffer.
        let frames = unsafe { &mut *self.frames.get() };
        let out = read(&frames[..self.len.load(SeqCst).min(frames.len())]);
        frames.clear();
        self.len.store(0, SeqCst);
        self.owner.store(WORKER_READY, SeqCst);
        Some(out)
    }
}

impl<T: Copy + Default> Default for CaptureBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn capture_access_is_exclusive_even_when_hook_calls_overlap() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        static ENTERED: AtomicBool = AtomicBool::new(false);
        static RELEASE: AtomicBool = AtomicBool::new(false);
        #[derive(Clone, Copy, Debug, PartialEq)]
        struct Frame(u32);
        impl Default for Frame {
            fn default() -> Self {
                ENTERED.store(true, Ordering::Release);
                while !RELEASE.load(Ordering::Acquire) { std::thread::yield_now(); }
                Self(0)
            }
        }
        let buffer = Arc::new(super::CaptureBuffer::<Frame>::new());
        assert!(buffer.prepare(2));
        assert!(buffer.begin());
        let writer = Arc::clone(&buffer);
        let thread = std::thread::spawn(move || writer.store(0, Frame(7)));
        while !ENTERED.load(Ordering::Acquire) { std::thread::yield_now(); }
        // A callback retiring or reading the capture cannot alias the active write.
        let begin = buffer.begin();
        let finish = buffer.finish();
        let read = buffer.frame_at(0);
        let write = buffer.store(1, Frame(9));
        buffer.cancel();
        RELEASE.store(true, Ordering::Release);
        assert!(thread.join().unwrap());
        assert!(!begin);
        assert_eq!(finish, None);
        assert_eq!(read, None);
        assert!(!write);
        assert_eq!(buffer.finish(), Some(1));
        assert_eq!(buffer.take(|frames| frames.to_vec()), Some(vec![Frame(7)]));
    }

    use super::{CaptureBuffer, MacroQueries, PlayBuffers};

    #[test]
    fn numbered_slot_transfer_preserves_frames_origin_and_refusals() {
        use super::{SlotTransfer, SlotResult};
        use crate::model::recording::{Recorder, Origin, ImportRefusal, ExportRefusal, SLOT_FRAMES};
        use crate::model::input_delay::MappedInputs;
        let mut recorder = Recorder::new();
        let mut empty = SlotTransfer::export(2);
        empty.execute(&mut recorder);
        assert_eq!(empty.result, SlotResult::ExportFailed(ExportRefusal::NotRecorded));
        let frame = MappedInputs { lstick_x: -91, rstick_y: 37, ..MappedInputs::NEUTRAL };
        let frames = [frame, MappedInputs::NEUTRAL, frame];
        let origin = Origin { facing: -1, status: 42, ..Origin::NONE };
        let mut import = SlotTransfer::import(2, &frames, origin).unwrap();
        import.execute(&mut recorder);
        assert_eq!(import.result, SlotResult::Imported);
        assert_eq!(SlotTransfer::import(2, &vec![frame; SLOT_FRAMES + 1], origin),
            Err(ImportRefusal::TooLong(SLOT_FRAMES + 1)));
        let mut export = SlotTransfer::export(2);
        export.execute(&mut recorder);
        assert_eq!(export.exported(), Some((frames.as_slice(), origin)));
        let mut invalid = SlotTransfer::export(usize::MAX);
        invalid.execute(&mut recorder);
        assert_eq!(invalid.result, SlotResult::ExportFailed(ExportRefusal::SlotOutOfRange));
    }

    #[test]
    fn the_idle_snapshot_has_nothing_loaded_or_visible() {
        let idle = MacroQueries::IDLE;
        assert!(!idle.source_is_named && !idle.playback_allowed && !idle.panel_visible && idle.loaded_kind == -1);
    }

    #[test]
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

    #[test]
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

    #[test]
    fn a_slow_worker_keeps_its_capture_until_it_has_copied_the_result() {
        use std::sync::{Arc, Barrier};
        let capture = Arc::new(CaptureBuffer::<u16>::new());
        assert!(capture.prepare(4));
        assert!(capture.begin());
        assert!(capture.store(0, 42));
        assert_eq!(capture.finish(), Some(1));
        let entered = Arc::new(Barrier::new(2));
        let leave = Arc::new(Barrier::new(2));
        let worker = {
            let (capture, entered, leave) = (capture.clone(), entered.clone(), leave.clone());
            std::thread::spawn(move || capture.take(|frames| {
                entered.wait();
                leave.wait();
                frames.to_vec()
            }))
        };
        entered.wait();
        // The game continues while the worker is delayed: it cannot overwrite
        // the finished recording or mistake it for a running capture.
        assert!(!capture.begin());
        assert!(!capture.store(0, 99));
        assert_eq!(capture.frame_at(0), None);
        capture.cancel();
        leave.wait();
        assert_eq!(worker.join().unwrap(), Some(vec![42]));
        assert!(capture.begin());
        assert!(capture.store(0, 7));
        assert_eq!(capture.finish(), Some(1));
        assert_eq!(capture.take(<[u16]>::to_vec), Some(vec![7]));
    }

    #[test]
    fn the_capture_buffer_moves_hook_to_worker_and_back_without_growing_in_the_hook() {
        let capture = CaptureBuffer::<u16>::new();
        assert!(!capture.store(0, 1), "not capturing yet");
        assert!(capture.prepare(4));
        assert!(capture.begin());
        assert!(!capture.prepare(8), "the hook owns it");
        assert!(capture.store(0, 5) && capture.store(2, 7));
        assert!(!capture.store(4, 9), "past the prepared capacity: refused, never grown in the hook");
        assert_eq!((capture.has_frame(1), capture.frame_at(1), capture.frame_at(2)), (true, Some(0), Some(7)));
        assert_eq!(capture.finish(), Some(3));
        assert!(!capture.begin(), "the worker has not taken the result");
        assert_eq!(capture.take(<[u16]>::to_vec), Some(vec![5, 0, 7]));
        assert_eq!(capture.take(<[u16]>::to_vec), None);
        assert!(capture.begin());
        capture.cancel();
        assert_eq!(capture.finish(), None);
        assert!(capture.begin(), "a cancelled capture returns the buffer ready");
    }
}
