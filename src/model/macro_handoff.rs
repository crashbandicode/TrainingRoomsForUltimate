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

/// Copied Vault observations; native pointers never cross to the worker.
#[derive(Clone, Copy, Debug)]
pub enum VaultEvent {
    Begin { epoch: u64 },
    Sample { epoch: u64, entry: i32, sample: crate::engine::macros::ReplayFrame, quantized: bool },
    End { epoch: u64, truncated: bool },
}

/// Worker-owned archive of the latest and previous native Vault timelines.
pub struct VaultArchive {
    epoch: u64,
    pub session: crate::engine::macros::ReplaySession,
    pub previous: Option<crate::engine::macros::ReplaySession>,
}
impl Default for VaultArchive { fn default() -> Self { Self::new() } }
impl VaultArchive {
    pub const fn new() -> Self {
        Self { epoch: 0, session: crate::engine::macros::ReplaySession { tracks: Vec::new(), truncated: false }, previous: None }
    }
    pub fn note_loss(&mut self, last_lost_epoch: u64) {
        if last_lost_epoch != 0 && self.epoch <= last_lost_epoch { self.session.truncated = true; }
    }
    pub fn apply(&mut self, event: VaultEvent, last_lost_epoch: u64) {
        use crate::engine::macros::{ReplaySession, ReplayTrack, MAX_FRAMES};
        let epoch = match event {
            VaultEvent::Begin { epoch } | VaultEvent::Sample { epoch, .. } | VaultEvent::End { epoch, .. } => epoch,
        };
        if epoch == 0 || epoch < self.epoch { return; }
        if epoch != self.epoch {
            let previous = std::mem::replace(&mut self.session, ReplaySession::default());
            if previous.tracks.iter().any(|track| !track.frames.is_empty()) { self.previous = Some(previous); }
            self.epoch = epoch;
            self.session.truncated = !matches!(event, VaultEvent::Begin { .. });
        }
        // A failed post ends hook sampling immediately. Treat queued prefixes
        // predating a known loss conservatively; never label one complete.
        self.note_loss(last_lost_epoch);
        match event {
            VaultEvent::Begin { .. } => {},
            VaultEvent::End { truncated, .. } => self.session.truncated |= truncated,
            VaultEvent::Sample { entry, sample, quantized, .. } => {
                if !(0..8).contains(&entry) { self.session.truncated = true; return; }
                let tracks = &mut self.session.tracks;
                let index = match tracks.iter().position(|track| track.entry == entry) {
                    Some(index) => index,
                    None => {
                        tracks.push(ReplayTrack { entry, frames: Vec::new(), quantized_analog: false });
                        tracks.len() - 1
                    },
                };
                let track = &mut tracks[index];
                track.quantized_analog |= quantized;
                if track.frames.len() < MAX_FRAMES { track.frames.push(sample); }
                else { self.session.truncated = true; }
            },
        }
    }
}

const _: () = assert!(crate::engine::macros::MAX_RETAINED_CAPTURES == 4);

/// Game-owned record identities only. Frame storage belongs to LiveArchive.
pub struct LiveTokens {
    generation: u64,
    keys: [Option<crate::engine::macros::NativeRecordKey>; 4],
    active: Option<crate::engine::macros::NativeRecordKey>,
    sampled_keys: [bool; 4],
    last_tick: [Option<u32>; 8],
    sealed: [bool; 8],
}
impl Default for LiveTokens { fn default() -> Self { Self::new() } }
impl LiveTokens {
    pub const fn new() -> Self {
        Self { generation: 0, keys: [None; 4], active: None, sampled_keys: [false; 4], last_tick: [None; 8], sealed: [false; 8] }
    }
    pub fn clear(&mut self) { self.keys = [None; 4]; self.sampled_keys = [false; 4]; self.active = None; }
    pub fn active_key(&self) -> Option<crate::engine::macros::NativeRecordKey> { self.active }
    pub fn key_for_index(&self, index: u32) -> Option<crate::engine::macros::NativeRecordKey> {
        self.keys.iter().flatten().copied().find(|key| key.index == index)
    }
    pub fn invalidate_record(&mut self, index: u32) {
        if self.active.is_some_and(|key| key.index == index) { self.active = None; }
        let mut kept = [None; 4];
        let mut sampled = [false; 4];
        let mut at = 0;
        for (slot, value) in self.keys.iter().enumerate() {
            if let Some(key) = value.filter(|key| key.index != index) {
                kept[at] = Some(key); sampled[at] = self.sampled_keys[slot]; at += 1;
            }
        }
        self.keys = kept; self.sampled_keys = sampled;
    }
    pub fn start_record(&mut self, index: u32) -> crate::engine::macros::NativeRecordKey {
        self.finish();
        self.invalidate_record(index);
        self.generation += 1;
        let key = crate::engine::macros::NativeRecordKey { generation: self.generation, index };
        let at = self.keys.iter().position(Option::is_none).unwrap_or_else(|| {
            self.keys.rotate_left(1); self.sampled_keys.rotate_left(1); 3
        });
        self.keys[at] = Some(key);
        self.sampled_keys[at] = false;
        self.active = Some(key);
        self.last_tick = [None; 8]; self.sealed = [false; 8];
        key
    }
    pub fn has_samples(&self, key: crate::engine::macros::NativeRecordKey) -> bool {
        self.keys.iter().position(|value| *value == Some(key)).is_some_and(|slot| self.sampled_keys[slot])
    }
    pub fn finish(&mut self) {
        if let Some(key) = self.active {
            if !self.has_samples(key) { self.invalidate_record(key.index); }
        }
        self.active = None;
    }
    pub fn wants_sample(&self, key: crate::engine::macros::NativeRecordKey, entry: i32, tick: u32) -> bool {
        self.active == Some(key) && (0..8).contains(&entry)
            && !self.sealed[entry as usize] && self.last_tick[entry as usize] != Some(tick)
    }
    pub fn sampled(&mut self, key: crate::engine::macros::NativeRecordKey, entry: i32, tick: u32) {
        if self.active == Some(key) && (0..8).contains(&entry) {
            self.last_tick[entry as usize] = Some(tick);
            if let Some(slot) = self.keys.iter().position(|value| *value == Some(key)) { self.sampled_keys[slot] = true; }
        }
    }
    pub fn seal(&mut self, key: crate::engine::macros::NativeRecordKey, entry: i32) {
        if self.active == Some(key) && (0..8).contains(&entry) { self.sealed[entry as usize] = true; }
    }
}

#[derive(Clone, Copy)]
pub struct FixedReplayId { bytes: [u8; 96], len: u8 }
impl FixedReplayId {
    pub fn new(text: &str) -> Option<Self> {
        if text.is_empty() || text.len() > 96 || text.chars().any(|c| c.is_control() || c == '/' || c == char::from(92u8)) { return None; }
        let mut value = Self { bytes: [0; 96], len: text.len() as u8 };
        value.bytes[..text.len()].copy_from_slice(text.as_bytes());
        Some(value)
    }
    fn text(&self) -> &str { std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap() }
}
#[derive(Clone, Copy)]
pub struct FixedReplayTitle { units: [u16; 40], len: u8 }
impl FixedReplayTitle {
    pub fn new(units: &[u16]) -> Option<Self> {
        if units.is_empty() || units.len() > 40
            || char::decode_utf16(units.iter().copied()).any(|c| c.is_err() || c.is_ok_and(char::is_control)) { return None; }
        let mut value = Self { units: [0; 40], len: units.len() as u8 };
        value.units[..units.len()].copy_from_slice(units);
        Some(value)
    }
    fn text(&self) -> String { String::from_utf16(&self.units[..usize::from(self.len)]).unwrap() }
}
/// ReplayMiis is a fixed value but intentionally has a non-Copy public API.
/// Copy its proven entries for the mailbox; rebuild its portable codec on the worker.
#[derive(Clone, Copy)]
pub struct FixedReplayMiis([Option<crate::engine::mii::replay::RawMii>; 8]);
impl FixedReplayMiis {
    pub fn copied(value: &crate::engine::mii::replay::ReplayMiis) -> Self {
        Self(std::array::from_fn(|entry| value.entry(entry).copied()))
    }
    pub fn owned(&self) -> Option<crate::engine::mii::replay::ReplayMiis> {
        let mut bytes = vec![1, self.0.iter().flatten().count() as u8];
        for mii in self.0.iter().flatten() {
            bytes.extend([mii.entry(), mii.class().kind() as u8]);
            bytes.extend(mii.source_index().to_le_bytes());
            bytes.extend(mii.charinfo());
            let tokens = mii.tokens();
            for token in [tokens.outfit, tokens.hat, tokens.colour, tokens.voice] { bytes.extend(token.to_le_bytes()); }
            bytes.extend(tokens.specials);
        }
        crate::engine::mii::replay::ReplayMiis::from_bytes(&bytes).ok()
    }
}
#[derive(Clone, Copy)]
pub enum LiveEvent {
    Clear,
    Start { key: crate::engine::macros::NativeRecordKey },
    Invalidate { index: u32 },
    Finish { partial: bool },
    Seal { key: crate::engine::macros::NativeRecordKey, entry: i32 },
    Sample { key: crate::engine::macros::NativeRecordKey, entry: i32, tick: u32, sample: crate::engine::macros::LiveFrame },
    Bind { save: u64, key: crate::engine::macros::NativeRecordKey },
    Forget { save: u64 },
    Commit { save: u64, id: FixedReplayId, title: FixedReplayTitle, miis: bool },
}
pub struct LiveCommit {
    pub save: u64,
    pub needs_miis: bool,
    pub id: String,
    pub title: String,
    pub session: std::sync::Arc<crate::engine::macros::LiveSession>,
    pub miis: Option<crate::engine::mii::replay::ReplayMiis>,
}
pub struct LiveArchive {
    pool: crate::engine::macros::LiveCapturePool,
    bindings: std::collections::VecDeque<(u64, std::sync::Arc<crate::engine::macros::LiveSession>)>,
    last_loss: u64,
}
impl Default for LiveArchive { fn default() -> Self { Self::new() } }
impl LiveArchive {
    pub const fn new() -> Self {
        Self { pool: crate::engine::macros::LiveCapturePool::new(), bindings: std::collections::VecDeque::new(), last_loss: 0 }
    }
    /// A missing lifecycle/sample event invalidates uncertain associations. A later
    /// Start is independent; no suffix or reused native slot is rebound as complete.
    pub fn apply(&mut self, serial: u64, event: LiveEvent, last_lost_serial: u64) -> Option<LiveCommit> {
        if last_lost_serial > self.last_loss {
            self.pool.clear(); self.bindings.clear(); self.last_loss = last_lost_serial;
        }
        if serial <= self.last_loss { return None; }
        match event {
            LiveEvent::Clear => self.pool.clear(),
            LiveEvent::Start { key } => { self.pool.start_record_owned(key); },
            LiveEvent::Invalidate { index } => self.pool.invalidate_record(index),
            LiveEvent::Finish { partial } => if partial { self.pool.finish_partial(); } else { self.pool.finish_record(); },
            LiveEvent::Seal { key, entry } => self.pool.seal_entry(key, entry),
            LiveEvent::Sample { key, entry, tick, sample } => { self.pool.push(key, entry, tick, sample); },
            LiveEvent::Bind { save, key } => {
                self.bindings.retain(|(id, _)| *id != save);
                if let Some(session) = self.pool.snapshot(key) {
                    while self.bindings.len() >= crate::engine::macros::MAX_RETAINED_CAPTURES { self.bindings.pop_front(); }
                    self.bindings.push_back((save, session));
                }
            },
            LiveEvent::Forget { save } => self.bindings.retain(|(id, _)| *id != save),
            LiveEvent::Commit { save, id, title, miis } => {
                let at = self.bindings.iter().position(|(id, _)| *id == save)?;
                let (_, session) = self.bindings.remove(at)?;
                return Some(LiveCommit { save, needs_miis: miis, id: id.text().to_owned(), title: title.text(), session, miis: None });
            },
        }
        None
    }
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
    fn live_worker_handoff_preserves_bound_saves_across_reset_and_refuses_event_loss() {
        use super::*;
        use crate::engine::macros::{LiveFrame, LiveInput, StartState};
        let mut tokens = LiveTokens::new();
        let mut archive = LiveArchive::new();
        let sample = |buttons| LiveFrame { input: LiveInput { buttons, ..LiveInput::default() },
            start: StartState::neutral(8, 0, 1.0), x: 1.0, y: 2.0, position_z: 3.0, speed_z: 0.0, body: 99 };
        let id = FixedReplayId::new("Replay-001").unwrap();
        let title = FixedReplayTitle::new(&"Match title".encode_utf16().collect::<Vec<_>>()).unwrap();
        let first = tokens.start_record(0);
        archive.apply(1, LiveEvent::Start { key: first }, 0);
        archive.apply(2, LiveEvent::Sample { key: first, entry: 1, tick: 20, sample: sample(7) }, 0);
        tokens.sampled(first, 1, 20);
        tokens.finish();
        archive.apply(3, LiveEvent::Finish { partial: false }, 0);
        archive.apply(4, LiveEvent::Bind { save: 11, key: first }, 0);
        tokens.clear();
        archive.apply(5, LiveEvent::Clear, 0);
        let second = tokens.start_record(0);
        assert_ne!(first, second);
        archive.apply(6, LiveEvent::Start { key: second }, 0);
        archive.apply(7, LiveEvent::Sample { key: second, entry: 1, tick: 80, sample: sample(9) }, 0);
        archive.apply(8, LiveEvent::Finish { partial: true }, 0);
        archive.apply(9, LiveEvent::Bind { save: 12, key: second }, 0);
        let later = archive.apply(10, LiveEvent::Commit { save: 12, id, title, miis: false }, 0).unwrap();
        let earlier = archive.apply(11, LiveEvent::Commit { save: 11, id, title, miis: false }, 0).unwrap();
        assert_eq!(earlier.session.tracks[0].frames[0].input.buttons, 7);
        assert!(!earlier.session.truncated);
        assert_eq!(later.session.tracks[0].frames[0].input.buttons, 9);
        assert!(later.session.truncated);
        assert_eq!(earlier.id, "Replay-001");
        assert_eq!(earlier.title, "Match title");
        archive.apply(12, LiveEvent::Bind { save: 13, key: second }, 0);
        assert!(archive.apply(14, LiveEvent::Commit { save: 13, id, title, miis: false }, 13).is_none());
        // A subsequent native incarnation recovers independently; no lost prefix is rebound.
        let third = tokens.start_record(0);
        archive.apply(15, LiveEvent::Start { key: third }, 13);
        archive.apply(16, LiveEvent::Sample { key: third, entry: 1, tick: 90, sample: sample(15) }, 13);
        archive.apply(17, LiveEvent::Finish { partial: false }, 13);
        archive.apply(18, LiveEvent::Bind { save: 14, key: third }, 13);
        let recovered = archive.apply(19, LiveEvent::Commit { save: 14, id, title, miis: false }, 13).unwrap();
        assert_eq!(recovered.session.tracks[0].frames[0].input.buttons, 15);
        assert!(!recovered.session.truncated);
    }

    #[test]
    fn live_tokens_keep_bounded_incarnations_and_skip_duplicate_or_sealed_samples() {
        use super::LiveTokens;
        let mut tokens = LiveTokens::new();
        let first = tokens.start_record(0);
        assert!(tokens.wants_sample(first, 1, 0));
        tokens.sampled(first, 1, 0);
        assert!(!tokens.wants_sample(first, 1, 0));
        assert!(tokens.wants_sample(first, 1, 1));
        tokens.seal(first, 1);
        assert!(!tokens.wants_sample(first, 1, 1));
        tokens.finish();
        for index in 1..=4 {
            let key = tokens.start_record(index);
            tokens.sampled(key, 0, 0);
            tokens.finish();
        }
        assert!(tokens.key_for_index(0).is_none());
        assert!(tokens.key_for_index(1).is_some());
        let old = tokens.key_for_index(4).unwrap();
        let replacement = tokens.start_record(4);
        assert_ne!(replacement, old);
        assert!(!tokens.has_samples(replacement));
        tokens.finish();
        assert!(tokens.key_for_index(4).is_none());
        assert!(tokens.key_for_index(1).is_some());
        tokens.clear();
        assert!(tokens.key_for_index(1).is_none());
        assert!(tokens.start_record(1).generation > replacement.generation);
    }

    #[test]
    fn replay_identity_handoff_keeps_unicode_and_exact_mii_payload() {
        use super::{FixedReplayId, FixedReplayMiis, FixedReplayTitle};
        assert!(FixedReplayId::new("../bad").is_none());
        assert!(FixedReplayId::new("bad
name").is_none());
        assert!(FixedReplayTitle::new(&[0xd800]).is_none());
        assert!(FixedReplayTitle::new(&[10]).is_none());
        let title = "Mario 🥊".encode_utf16().collect::<Vec<_>>();
        assert_eq!(FixedReplayTitle::new(&title).unwrap().text(), "Mario 🥊");
        let mut bytes = vec![0u8; 106];
        bytes[0] = 1; bytes[1] = 1; bytes[2] = 0; bytes[3] = 0x48;
        bytes[4..6].copy_from_slice(&321u16.to_le_bytes());
        for (i, byte) in bytes[6..94].iter_mut().enumerate() { *byte = i as u8; }
        bytes[94..96].copy_from_slice(&1u16.to_le_bytes());
        bytes[98..100].copy_from_slice(&4u16.to_le_bytes());
        bytes[100..102].copy_from_slice(&37u16.to_le_bytes());
        bytes[102..106].copy_from_slice(&[1, 0, 0, 0]);
        let original = crate::engine::mii::replay::ReplayMiis::from_bytes(&bytes).unwrap();
        assert_eq!(FixedReplayMiis::copied(&original).owned().unwrap(), original);
    }

    #[test]
    fn vault_events_preserve_tracks_previous_and_partial_identity() {
        use super::{VaultArchive, VaultEvent};
        use crate::engine::macros::{InputFrame, ReplayFrame, StartState};
        let sample = ReplayFrame { input: InputFrame { buttons: 7, ..InputFrame::default() },
            start: StartState::neutral(8, 0, 1.0), x: 12.0, y: 4.0 };
        let mut archive = VaultArchive::new();
        archive.apply(VaultEvent::Begin { epoch: 1 }, 0);
        archive.apply(VaultEvent::Sample { epoch: 1, entry: 3, sample, quantized: true }, 0);
        archive.apply(VaultEvent::End { epoch: 1, truncated: false }, 0);
        assert!(!archive.session.truncated);
        assert_eq!(archive.session.tracks[0].entry, 3);
        assert!(archive.session.tracks[0].quantized_analog);
        assert_eq!(archive.session.tracks[0].frames[0].input.buttons, 7);
        assert_eq!(archive.session.tracks[0].frames[0].x, 12.0);
        archive.apply(VaultEvent::Begin { epoch: 2 }, 0);
        archive.apply(VaultEvent::Sample { epoch: 2, entry: 1, sample, quantized: false }, 2);
        assert!(archive.session.truncated);
        assert_eq!(archive.previous.as_ref().unwrap().tracks[0].entry, 3);
        archive.apply(VaultEvent::Sample { epoch: 1, entry: 4, sample, quantized: false }, 2);
        assert_eq!(archive.session.tracks.len(), 1);
        // Missing Begin is visibly partial, and the previous capture remains its own values.
        archive.apply(VaultEvent::Sample { epoch: 3, entry: 0, sample, quantized: false }, 2);
        assert!(archive.session.truncated);
        assert!(archive.previous.as_ref().unwrap().truncated);
        archive.apply(VaultEvent::Begin { epoch: 4 }, 2);
        archive.apply(VaultEvent::Sample { epoch: 4, entry: 8, sample, quantized: false }, 2);
        assert!(archive.session.truncated);
        assert!(archive.session.tracks.is_empty());
    }

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
        use std::sync::Arc;
        let buffers = Arc::new(PlayBuffers::<u64>::new());
        let stop = Arc::new(AtomicBool::new(false));
        // SAFETY: the main thread is the only writer.
        unsafe { buffers.prepare(256) };
        let readers: Vec<_> = (0..3)
            .map(|_| {
                let (buffers, stop) = (buffers.clone(), stop.clone());
                std::thread::spawn(move || {
                    let mut seen = 0u64;
                    while !stop.load(Ordering::Relaxed) {
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
                    }
                    seen
                })
            })
            .collect();
        for value in 0..2000u64 {
            // SAFETY: the only writer.
            unsafe { buffers.fill(&[value; 256]) };
        }
        stop.store(true, Ordering::Relaxed);
        let seen: u64 = readers.into_iter().map(|r| r.join().unwrap()).sum();
        assert!(seen > 0);
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
