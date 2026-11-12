//! Bounded observations associated with native replay records, not the latest match.
//! The adapter supplies verified record/reset events and native serializer counts.
//! This module neither recognizes game modes nor decides whether a save succeeded.

use std::collections::VecDeque;
use std::sync::Arc;

use {crate::engine::macros::LiveFrame, crate::engine::macros::LiveSession, crate::engine::macros::LiveTrack, crate::engine::macros::MAX_FRAMES};

/// An incarnation changes even when the game resets/reuses the same native index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeRecordKey {
    pub generation: u64,
    pub index: u32,
}

/// Includes the active record. Older evicted records are unavailable, never rebound
/// to the newest capture. At most 4 * 8 * MAX_FRAMES samples are retained internally.
pub const MAX_RETAINED_CAPTURES: usize = 4;
const ENTRIES: usize = 8;

/// First worker transition only. Flags: gap=1, invalid sample=2, late start=4.
#[cfg(any(feature = "diag", test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FirstSeal {
    pub key: NativeRecordKey,
    pub reason: SealReason,
    pub flags: u8,
    pub entry: Option<i32>,
    /// Entry length, or total retained samples for a record-wide partial finish.
    pub frames: u32,
    pub first: Option<u32>,
    pub last: Option<u32>,
    pub next: Option<u32>,
}
#[cfg(any(feature = "diag", test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SealReason { Sample, Reservation, Limit, Explicit, PartialFinish }
#[cfg(any(feature = "diag", test))]
impl std::fmt::Display for FirstSeal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self.reason {
            SealReason::Sample => "sample", SealReason::Reservation => "reservation",
            SealReason::Limit => "limit", SealReason::Explicit => "explicit",
            SealReason::PartialFinish => "partial_finish",
        };
        write!(f, "record={}:{} reason={} flags={} entry={:?} frames={} first={:?} last={:?} next={:?}",
            self.key.generation, self.key.index, reason, self.flags, self.entry,
            self.frames, self.first, self.last, self.next)
    }
}

struct LiveRecord {
    session: LiveSession,
    last_tick: [Option<u32>; ENTRIES],
    sealed: [bool; ENTRIES],
}

struct Record {
    key: NativeRecordKey,
    live: Option<LiveRecord>,
    saved: Option<Arc<LiveSession>>,
}

pub struct LiveCapturePool {
    generation: u64,
    active: Option<NativeRecordKey>,
    records: VecDeque<Record>,
    #[cfg(any(feature = "diag", test))]
    first_seal: Option<FirstSeal>,
}

impl Default for LiveCapturePool {
    fn default() -> Self { Self::new() }
}

impl LiveCapturePool {
    pub const fn new() -> Self {
        Self { generation: 0, active: None, records: VecDeque::new(),
            #[cfg(any(feature = "diag", test))]
            first_seal: None,
        }
    }

    /// Drop only pool ownership. A save already bound to an Arc remains valid.
    pub fn clear(&mut self) {
        self.active = None;
        self.records.clear();
        #[cfg(any(feature = "diag", test))]
        { self.first_seal = None; }
    }

    pub fn start_record(&mut self, index: u32) -> NativeRecordKey {
        #[cfg(any(feature = "diag", test))]
        { self.first_seal = None; }
        self.finish_record();
        self.records.retain(|record| record.key.index != index);
        while self.records.len() >= MAX_RETAINED_CAPTURES {
            self.records.pop_front();
        }
        self.generation += 1;
        let key = NativeRecordKey { generation: self.generation, index };
        self.records.push_back(Record {
            key,
            live: Some(LiveRecord {
                session: LiveSession { tracks: Vec::with_capacity(ENTRIES),
                    truncated: false, first_counter: None, breaks: Vec::new() },
                last_tick: [None; ENTRIES],
                sealed: [false; ENTRIES],
            }),
            saved: None,
        });
        self.active = Some(key);
        key
    }

    /// Worker handoff: preserve the game owner's monotonic incarnation even
    /// after a dropped event retired a prior timeline.
    pub fn start_record_owned(&mut self, key: NativeRecordKey) -> bool {
        if key.generation <= self.generation { return false; }
        self.generation = key.generation - 1;
        self.start_record(key.index);
        true
    }

    pub fn active_key(&self) -> Option<NativeRecordKey> { self.active }

    /// Existing worker consumes this after applying the event, before releasing its lock.
    /// Formatting happens only after that lock is released.
    #[cfg(any(feature = "diag", test))]
    pub fn take_first_seal(&mut self) -> Option<FirstSeal> { self.first_seal.take() }

    /// A native prefix reset replaces a slot without changing its numeric index.
    /// Previously bound immutable snapshots remain owned by their save operation.
    pub fn invalidate_record(&mut self, index: u32) {
        #[cfg(any(feature = "diag", test))]
        if self.first_seal.is_some_and(|note| note.key.index == index) { self.first_seal = None; }
        if self.active.is_some_and(|key| key.index == index) { self.active = None; }
        self.records.retain(|record| record.key.index != index);
    }

    pub fn key_for_index(&self, index: u32) -> Option<NativeRecordKey> {
        self.records.iter().find(|record| record.key.index == index).map(|record| record.key)
    }

    /// Skip redundant native getters for a repeated callback. A different count
    /// still reaches push(), which detects and seals gaps/backwards movement.
    pub fn wants_sample(&self, key: NativeRecordKey, entry: i32, tick: u32) -> bool {
        if self.active != Some(key) || !(0..ENTRIES as i32).contains(&entry) { return false; }
        self.records.back().and_then(|record| record.live.as_ref()).is_some_and(|live| {
            !live.sealed[entry as usize] && live.last_tick[entry as usize] != Some(tick)
        })
    }

    /// Reject one fighter's discontinuous sample without ending other tracks.
    /// A stale key cannot seal an entry in a newer native-record incarnation.
    pub fn seal_entry(&mut self, key: NativeRecordKey, entry: i32) {
        if self.active != Some(key) || !(0..ENTRIES as i32).contains(&entry) { return; }
        if let Some(live) = self.records.back_mut().and_then(|record| record.live.as_mut()) {
            #[cfg(any(feature = "diag", test))]
            observe_seal(&mut self.first_seal, live, key, SealReason::Explicit, 0, Some(entry), None);
            live.sealed[entry as usize] = true;
            live.session.truncated = true;
        }
    }

    pub fn push(&mut self, key: NativeRecordKey, entry: i32, tick: u32,
                sample: LiveFrame) -> bool {
        if self.active != Some(key) || !(0..ENTRIES as i32).contains(&entry) { return false; }
        let Some(live) = self.records.back_mut().and_then(|record| record.live.as_mut()) else {
            return false;
        };
        let slot = entry as usize;
        if live.sealed[slot] || live.last_tick[slot] == Some(tick) { return false; }
        let gap = match live.last_tick[slot] {
            Some(last) => last.checked_add(1) != Some(tick)
                && !(sample.is_valid() && resumes_after_break(live, slot, last, tick)),
            // The editor carries crop indexes across fighters. A late-starting
            // track must not silently shift the selected match time.
            None => live.session.first_counter.is_some_and(|first| first != tick),
        };
        if gap || !sample.is_valid() {
            #[cfg(any(feature = "diag", test))]
            observe_seal(&mut self.first_seal, live, key, SealReason::Sample,
                u8::from(gap) | (u8::from(!sample.is_valid()) << 1)
                    | (u8::from(gap && live.last_tick[slot].is_none()) << 2),
                Some(entry), Some(tick));
            live.sealed[slot] = true;
            live.session.truncated = true;
            return false;
        }
        let track_index = match live.session.tracks.iter().position(|track| track.entry == entry) {
            Some(index) => index,
            None => {
                let mut frames = Vec::new();
                // One bounded reservation when this fighter first appears. No
                // growth/allocation on subsequent simulation samples.
                if frames.try_reserve_exact(MAX_FRAMES).is_err() {
                    #[cfg(any(feature = "diag", test))]
                    observe_seal(&mut self.first_seal, live, key, SealReason::Reservation, 0, Some(entry), Some(tick));
                    live.sealed[slot] = true;
                    live.session.truncated = true;
                    return false;
                }
                live.session.tracks.push(LiveTrack { entry, frames });
                live.session.tracks.len() - 1
            }
        };
        if live.session.tracks[track_index].frames.len() == MAX_FRAMES {
            #[cfg(any(feature = "diag", test))]
            observe_seal(&mut self.first_seal, live, key, SealReason::Limit, 0, Some(entry), Some(tick));
            live.sealed[slot] = true;
            live.session.truncated = true;
            return false;
        }
        live.session.tracks[track_index].frames.push(sample);
        live.session.first_counter.get_or_insert(tick);
        live.last_tick[slot] = Some(tick);
        true
    }

    pub fn finish_record(&mut self) {
        let Some(key) = self.active.take() else { return };
        if let Some(record) = self.records.iter_mut().find(|record| record.key == key) {
            record.saved = record.live.take().filter(|live| !live.session.tracks.is_empty())
                .map(|mut live| {
                    for track in &mut live.session.tracks { track.frames.shrink_to_fit(); }
                    Arc::new(live.session)
                });
        }
        self.records.retain(|record| record.live.is_some() || record.saved.is_some());
    }

    /// Preserve a valid prefix when the native record itself becomes discontinuous.
    pub fn finish_partial(&mut self) {
        if let Some(_key) = self.active {
            if let Some(live) = self.records.back_mut().and_then(|record| record.live.as_mut()) {
                #[cfg(any(feature = "diag", test))]
                observe_seal(&mut self.first_seal, live, _key, SealReason::PartialFinish, 0, None, None);
                live.session.truncated = true;
            }
        }
        self.finish_record();
    }

    /// Called at the native indexed-copy boundary, separately from save success.
    /// The caller bounds outstanding saves; clones share immutable frame storage.
    pub fn snapshot(&mut self, key: NativeRecordKey) -> Option<Arc<LiveSession>> {
        if self.active == Some(key) { self.finish_record(); }
        self.records.iter().find(|record| record.key == key)?.saved.clone()
    }
}

/// A forward jump from `last` to `tick` is a phase break, not a discontinuity, when the
/// whole record went without live play in between: no admitted fighter was sampled at
/// any counter inside the gap (the TIME! to sudden-death transition, a pause), and this
/// fighter's track ends exactly where the timeline does. The first fighter to resume
/// records the break; the others must resume at the same counter and frame index.
/// Time is never compressed: the break keeps every later frame's true counter.
fn resumes_after_break(live: &mut LiveRecord, slot: usize, last: u32, tick: u32) -> bool {
    if tick <= last { return false; }
    let Some(len) = live.session.tracks.iter().find(|t| t.entry == slot as i32)
        .map(|t| t.frames.len() as u32) else { return false };
    if let Some(b) = live.session.breaks.last() {
        if b.counter == tick { return b.index == len; }
        if b.counter > last { return false; }
    }
    // Every admitted, unsealed fighter stopped at the same counter, and there are at least two:
    // a lone track cannot tell a stretch without play from its own dropped callbacks.
    let mut stopped_together = 0;
    for track in &live.session.tracks {
        let other = track.entry as usize;
        if live.sealed[other] { continue; }
        if live.last_tick[other] != Some(last) { return false; }
        stopped_together += 1;
    }
    let longest = live.session.tracks.iter().map(|t| t.frames.len() as u32).max().unwrap_or(0);
    if stopped_together < 2 || len != longest || live.session.breaks.len() >= crate::engine::macros::MAX_BREAKS
        || live.session.breaks.try_reserve(1).is_err() {
        return false;
    }
    live.session.breaks.push(crate::engine::macros::CaptureBreak { index: len, counter: tick });
    true
}

/// Pure worker facts, shared by every actual sealing branch. No native reads or allocation.
#[cfg(any(feature = "diag", test))]
fn observe_seal(out: &mut Option<FirstSeal>, live: &LiveRecord, key: NativeRecordKey,
                reason: SealReason, flags: u8, entry: Option<i32>, next: Option<u32>) {
    if live.session.truncated { return; }
    let frames = live.session.tracks.iter().filter(|t| entry.is_none_or(|e| e == t.entry))
        .map(|t| t.frames.len() as u32).sum();
    *out = Some(FirstSeal { key, reason, flags, entry, frames, first: live.session.first_counter,
        last: entry.and_then(|e| live.last_tick[e as usize]), next });
}

#[cfg(test)]
#[path = "worker_first_seal_tests.rs"]
mod worker_first_seal_tests;

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use {crate::engine::macros::LiveInput, crate::engine::macros::StartState};

    fn sample(buttons: u32) -> LiveFrame {
        LiveFrame { input: LiveInput { buttons, ..LiveInput::default() },
            start: StartState::neutral(8, 0, 1.0), x: 0.0, y: 0.0,
            position_z: 0.0, speed_z: 0.0, body: 1 }
    }

    fn duplicates_do_not_advance_and_gaps_never_compress_time() {
        for bad_tick in [9, 6] {
            let mut pool = LiveCapturePool::new();
            let key = pool.start_record(0);
            assert!(pool.push(key, 0, 7, sample(1)));
            assert!(!pool.wants_sample(key, 0, 7));
            assert!(!pool.push(key, 0, 7, sample(99)));
            assert!(pool.wants_sample(key, 0, bad_tick));
            assert!(!pool.push(key, 0, bad_tick, sample(2)));
            assert!(!pool.push(key, 0, 8, sample(3)));
            assert!(pool.push(key, 1, 7, sample(4)));
            assert!(pool.push(key, 1, 8, sample(5)));
            let saved = pool.snapshot(key).unwrap();
            assert!(saved.truncated);
            assert_eq!(saved.tracks[0].frames.len(), 1);
            assert_eq!(saved.tracks[1].frames.len(), 2);
        }
    }

    fn frame_limit_stops_one_track_without_stopping_other_fighters() {
        let mut pool = LiveCapturePool::new();
        let key = pool.start_record(0);
        assert!(pool.push(key, 1, 0, sample(41)));
        for tick in 0..MAX_FRAMES as u32 {
            assert!(pool.push(key, 0, tick, sample(tick)));
        }
        assert!(!pool.push(key, 0, MAX_FRAMES as u32, sample(99)));
        assert!(pool.push(key, 1, 1, sample(42)));
        let saved = pool.snapshot(key).unwrap();
        assert!(saved.truncated);
        assert_eq!(saved.tracks[1].frames.len(), MAX_FRAMES);
        assert_eq!(saved.tracks[0].frames.len(), 2);
    }

    fn a_fighter_resuming_elsewhere_or_backwards_seals_and_breaks_are_bounded() {
        let mut pool = LiveCapturePool::new();
        let key = pool.start_record(0);
        for tick in 0..4 {
            for entry in 0..2 { assert!(pool.push(key, entry, tick, sample(tick))); }
        }
        assert!(pool.push(key, 0, 20, sample(20)));
        assert!(!pool.push(key, 1, 21, sample(21)), "not the break's counter");
        assert!(!pool.push(key, 0, 19, sample(19)), "backwards");
        let saved = pool.snapshot(key).unwrap();
        assert!(saved.truncated);
        assert_eq!(saved.breaks.len(), 1);

        let mut pool = LiveCapturePool::new();
        let key = pool.start_record(1);
        let mut tick = 0;
        for _ in 0..=crate::engine::macros::MAX_BREAKS {
            for entry in 0..2 { assert!(pool.push(key, entry, tick, sample(tick))); }
            tick += 5;
        }
        assert!(!pool.push(key, 0, tick, sample(tick)), "one break more than MAX_BREAKS seals");
        let saved = pool.snapshot(key).unwrap();
        assert!(saved.truncated);
        assert_eq!(saved.breaks.len(), crate::engine::macros::MAX_BREAKS);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("duplicates_do_not_advance_and_gaps_never_compress_time", duplicates_do_not_advance_and_gaps_never_compress_time as fn()),
            ("frame_limit_stops_one_track_without_stopping_other_fighters", frame_limit_stops_one_track_without_stopping_other_fighters as fn()),
            ("a_fighter_resuming_elsewhere_or_backwards_seals_and_breaks_are_bounded", a_fighter_resuming_elsewhere_or_backwards_seals_and_breaks_are_bounded as fn()),
        ]);
    }
    }
}
