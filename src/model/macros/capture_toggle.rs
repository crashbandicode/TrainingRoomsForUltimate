//! Runtime capture preference and native-record ownership behind the Deluxe
//! Plugins-page toggle.
//!
//! One packed atomic word holds the next-record preference (`desired`), the
//! latched phase of the record in progress and adapter readiness. The overlay
//! draw thread (`current`/`request`) and the game thread (record start/end)
//! therefore never observe a torn combination and share no mutex: every
//! transition is one compare-exchange on that word.
//!
//! Semantics (revision 28 plan §2.4, §8 Q1/Q2, §9 R1/R2):
//! * A toggle never discards captured data. OFF governs new records only; a
//!   record that started armed keeps capturing and its bound save still
//!   publishes.
//! * Between records a request applies immediately. During a record a request
//!   that differs from the latched arm state is latched as pending; it takes
//!   effect when the owning native record ends, and `current` keeps reporting
//!   the latched value meanwhile. Nothing here performs I/O or logging; the
//!   adapter reports an applied pending request from the game thread.
//! * [`RecordOwnership`] tracks the accepted native record (state token, index,
//!   epoch) independently of the capture pool, so an unarmed record ends on the
//!   same native events as an armed one, and an event for an unrelated or
//!   older owner cannot end a newer record.

use std::sync::atomic::{AtomicU32, Ordering};

/// `request`/`current` choice index for capture off.
pub const CAPTURE_OFF: usize = 0;
/// `request`/`current` choice index for capture on.
pub const CAPTURE_ON: usize = 1;

const DESIRED: u32 = 1 << 0;
const PHASE_SHIFT: u32 = 1;
const PHASE_MASK: u32 = 0b11 << PHASE_SHIFT;
const READY: u32 = 1 << 3;

/// Latched phase of the native record in progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordPhase {
    /// No owned native record; `current` reports the preference.
    Idle,
    /// A native record is in progress and capture was off when it started.
    Unarmed,
    /// A native record is in progress and is being captured.
    Armed,
}

impl RecordPhase {
    fn from_bits(bits: u32) -> Self {
        match bits {
            1 => RecordPhase::Unarmed,
            2 => RecordPhase::Armed,
            _ => RecordPhase::Idle,
        }
    }

    fn bits(self) -> u32 {
        match self {
            RecordPhase::Idle => 0,
            RecordPhase::Unarmed => 1,
            RecordPhase::Armed => 2,
        }
    }

    /// Arm state of the record in progress; `None` when idle.
    pub fn armed(self) -> Option<bool> {
        match self {
            RecordPhase::Idle => None,
            RecordPhase::Unarmed => Some(false),
            RecordPhase::Armed => Some(true),
        }
    }
}

/// Answer to a draw-thread request, mirroring the host's `REQUEST_*` codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToggleRequest {
    /// The preference now governs the next record, or equals the record in progress.
    Applied,
    /// Latched; takes effect when the owned native record ends.
    Pending,
    /// Not a capture choice index. Nothing changed.
    Rejected,
    /// The adapter is not ready (guards, writer or hooks failed). Nothing changed.
    Unavailable,
}

/// One coherent reading of the packed word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToggleSnapshot {
    pub desired: bool,
    pub phase: RecordPhase,
    pub ready: bool,
}

/// Outcome of ending an owned record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordEnded {
    /// Arm state the record ran with.
    pub armed: bool,
    /// Preference at the moment the record ended.
    pub desired: bool,
}

impl RecordEnded {
    /// True when a request latched during the record now takes effect.
    pub fn pending_applied(self) -> bool {
        self.armed != self.desired
    }
}

fn choice(on: bool) -> usize {
    if on { CAPTURE_ON } else { CAPTURE_OFF }
}

/// The packed toggle word. Safe to call from any thread; never blocks.
#[derive(Debug)]
pub struct CaptureToggle {
    word: AtomicU32,
}

impl Default for CaptureToggle {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureToggle {
    pub const fn new() -> Self {
        Self { word: AtomicU32::new(0) }
    }

    fn decode(word: u32) -> ToggleSnapshot {
        ToggleSnapshot {
            desired: word & DESIRED != 0,
            phase: RecordPhase::from_bits((word & PHASE_MASK) >> PHASE_SHIFT),
            ready: word & READY != 0,
        }
    }

    fn encode(snapshot: ToggleSnapshot) -> u32 {
        (if snapshot.desired { DESIRED } else { 0 })
            | (snapshot.phase.bits() << PHASE_SHIFT)
            | (if snapshot.ready { READY } else { 0 })
    }

    /// Compare-exchange loop. `step` sees the latest word and returns the next
    /// snapshot plus the value to hand back once the exchange succeeds, so a
    /// competing transition can never be answered from a stale reading.
    fn transition<R>(&self, step: impl Fn(ToggleSnapshot) -> (ToggleSnapshot, R)) -> R {
        let mut current = self.word.load(Ordering::Acquire);
        loop {
            let (next, result) = step(Self::decode(current));
            match self.word.compare_exchange_weak(
                current,
                Self::encode(next),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return result,
                Err(actual) => current = actual,
            }
        }
    }

    pub fn snapshot(&self) -> ToggleSnapshot {
        Self::decode(self.word.load(Ordering::Acquire))
    }

    /// Next-record preference (also the value the INI should hold).
    pub fn desired(&self) -> bool {
        self.snapshot().desired
    }

    pub fn is_ready(&self) -> bool {
        self.snapshot().ready
    }

    pub fn phase(&self) -> RecordPhase {
        self.snapshot().phase
    }

    /// Cheap per-fighter early-out: true only while an armed record is in progress.
    pub fn is_armed(&self) -> bool {
        self.phase() == RecordPhase::Armed
    }

    /// Startup seed from the INI. Leaves phase and readiness untouched.
    pub fn seed(&self, desired: bool) {
        self.transition(|mut snapshot| {
            snapshot.desired = desired;
            (snapshot, ())
        })
    }

    /// Call once the native guards, the writer and the hooks have all succeeded.
    pub fn set_ready(&self) {
        self.transition(|mut snapshot| {
            snapshot.ready = true;
            (snapshot, ())
        })
    }

    /// Draw thread. `None` (shown as `Unavailable`) until ready; otherwise the
    /// latched arm state while recording, else the preference.
    pub fn current_choice(&self) -> Option<usize> {
        let snapshot = self.snapshot();
        snapshot
            .ready
            .then(|| choice(snapshot.phase.armed().unwrap_or(snapshot.desired)))
    }

    /// Draw thread. Stores the preference and answers in the same exchange.
    pub fn request_choice(&self, choice: usize) -> ToggleRequest {
        let wanted = match choice {
            CAPTURE_OFF => false,
            CAPTURE_ON => true,
            _ => return ToggleRequest::Rejected,
        };
        self.transition(|mut snapshot| {
            if !snapshot.ready {
                return (snapshot, ToggleRequest::Unavailable);
            }
            snapshot.desired = wanted;
            let result = match snapshot.phase.armed() {
                Some(armed) if armed != wanted => ToggleRequest::Pending,
                _ => ToggleRequest::Applied,
            };
            (snapshot, result)
        })
    }

    /// Game thread, after native accepted a segment-0 start. Latches the
    /// current preference as the record's arm state and returns it. A phase
    /// still latched from a missed end is replaced; end it first to report it.
    pub fn begin_record(&self) -> bool {
        self.transition(|mut snapshot| {
            snapshot.phase = if snapshot.desired { RecordPhase::Armed } else { RecordPhase::Unarmed };
            (snapshot, snapshot.desired)
        })
    }

    /// Game thread, when the owned native record ends. `None` when idle.
    pub fn end_record(&self) -> Option<RecordEnded> {
        self.transition(|mut snapshot| match snapshot.phase.armed() {
            Some(armed) => {
                snapshot.phase = RecordPhase::Idle;
                (snapshot, Some(RecordEnded { armed, desired: snapshot.desired }))
            }
            None => (snapshot, None),
        })
    }
}

/// The accepted native record, identified by its borrowed state token, its
/// native pool index and the adapter epoch at acceptance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordOwner {
    pub state: usize,
    pub index: u32,
    pub epoch: u64,
}

/// Game-thread ownership ledger, kept under the adapter's mutex and separate
/// from the capture pool so unarmed records are tracked too. Every `release_*`
/// result that is `Some` must be followed by [`CaptureToggle::end_record`].
#[derive(Debug, Default)]
pub struct RecordOwnership {
    owner: Option<RecordOwner>,
}

impl RecordOwnership {
    pub const fn new() -> Self {
        Self { owner: None }
    }

    #[cfg(test)]
    pub fn owner(&self) -> Option<RecordOwner> {
        self.owner
    }

    pub fn owns_state(&self, state: usize) -> bool {
        self.owner.is_some_and(|owner| owner.state == state)
    }

    pub fn owns_record(&self, state: usize, index: u32) -> bool {
        self.owner.is_some_and(|owner| owner.state == state && owner.index == index)
    }

    /// Accepted segment-0 start. Returns a previous owner that never saw its
    /// end; the caller ends that one before latching the new record.
    pub fn begin(&mut self, owner: RecordOwner) -> Option<RecordOwner> {
        self.owner.replace(owner)
    }

    /// Nonzero segment on the owned state: the same record continues under a
    /// new native index and epoch. False (and no change) for any other state.
    pub fn continue_segment(&mut self, state: usize, index: u32, epoch: u64) -> bool {
        match &mut self.owner {
            Some(owner) if owner.state == state => {
                owner.index = index;
                owner.epoch = epoch;
                true
            }
            _ => false,
        }
    }

    /// Stop/reset instructions and serializer exhaustion: keyed by state token.
    pub fn release_state(&mut self, state: usize) -> Option<RecordOwner> {
        self.owns_state(state).then(|| self.owner.take()).flatten()
    }

    /// FRAM publication and slot replacement/import/reset: keyed by state and index.
    pub fn release_record(&mut self, state: usize, index: u32) -> Option<RecordOwner> {
        self.owns_record(state, index).then(|| self.owner.take()).flatten()
    }

    /// Record interface constructed, cleared or destroyed.
    pub fn release_all(&mut self) -> Option<RecordOwner> {
        self.owner.take()
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use {crate::engine::macros::LiveCapturePool, crate::engine::macros::LiveFrame, crate::engine::macros::LiveInput, crate::engine::macros::LiveSession, crate::engine::macros::StartState};
    use std::sync::Arc;

    fn sample(buttons: u32) -> LiveFrame {
        LiveFrame { input: LiveInput { buttons, ..LiveInput::default() },
            start: StartState::neutral(8, 0, 1.0), x: 0.0, y: 0.0,
            position_z: 0.0, speed_z: 0.0, body: 1 }
    }

    /// Host model of the native adapter's lifecycle hooks (`src/live_replay.rs`):
    /// the same ownership, pool and toggle calls in the same order, without
    /// native memory. `state` is the borrowed native state token.
    struct Sim {
        toggle: CaptureToggle,
        owner: RecordOwnership,
        pool: LiveCapturePool,
        state: usize,
        epoch: u64,
        ended: Vec<RecordEnded>,
    }

    impl Sim {
        fn new(seed: bool) -> Self {
            let toggle = CaptureToggle::new();
            toggle.seed(seed);
            toggle.set_ready();
            Self { toggle, owner: RecordOwnership::new(), pool: LiveCapturePool::new(),
                state: 0, epoch: 0, ended: Vec::new() }
        }

        fn release(&mut self, released: Option<RecordOwner>) {
            if released.is_some() {
                if let Some(ended) = self.toggle.end_record() { self.ended.push(ended); }
            }
        }

        fn reset_pool(&mut self, state: usize) {
            self.epoch += 1;
            self.state = state;
            self.pool.clear();
            let released = self.owner.release_all();
            self.release(released);
        }

        /// `start_recording` hook. `None` when native rejected the start.
        fn start(&mut self, state: usize, index: u32, segment: u32, accepted: bool) -> Option<bool> {
            if !accepted { return None; }
            if self.state != state { self.reset_pool(state); }
            self.epoch += 1;
            if segment == 0 {
                let displaced = self.owner.begin(RecordOwner { state, index, epoch: self.epoch });
                self.release(displaced);
                let armed = self.toggle.begin_record();
                if armed { self.pool.start_record(index); } else { self.pool.invalidate_record(index); }
                Some(armed)
            } else {
                self.pool.finish_partial();
                self.owner.continue_segment(state, index, self.epoch);
                self.toggle.phase().armed()
            }
        }

        fn push(&mut self, entry: i32, tick: u32) -> bool {
            let Some(key) = self.pool.active_key() else { return false };
            self.pool.push(key, entry, tick, sample(tick))
        }

        /// `publish_record_frames` hook (FRAM publication for `index`).
        fn publish(&mut self, state: usize, index: u32, recording: bool) {
            if self.state != state || !recording || !self.owner.owns_record(state, index) { return; }
            self.stop(state, false);
        }

        /// Inline stop/reset instructions for `state`.
        fn stop(&mut self, state: usize, partial: bool) {
            if self.state != state { return; }
            self.epoch += 1;
            if partial { self.pool.finish_partial(); } else { self.pool.finish_record(); }
            let released = self.owner.release_state(state);
            self.release(released);
        }

        /// `serialize_frame` post-original observation: `(index, recording)` or none.
        fn serializer(&mut self, state: usize, observed: Option<(u32, bool)>) {
            if self.state != state { return; }
            match observed {
                Some((index, true)) if self.owner.owns_record(state, index) => {}
                _ => {
                    self.pool.finish_partial();
                    let released = self.owner.release_state(state);
                    self.release(released);
                }
            }
        }

        /// `replace_record` / `reset_current_record` / `import_record` for one slot.
        fn invalidate(&mut self, state: usize, index: u32) {
            if self.state != state { return; }
            self.epoch += 1;
            self.pool.invalidate_record(index);
            let released = self.owner.release_record(state, index);
            self.release(released);
        }

        /// `copy_record`: native indexed copy binds the snapshot for a save.
        fn copy(&mut self, index: u32) -> Option<Arc<LiveSession>> {
            let key = self.pool.key_for_index(index)?;
            if self.pool.active_key() == Some(key) { self.pool.finish_partial(); }
            self.pool.snapshot(key)
        }

        fn current(&self) -> Option<usize> { self.toggle.current_choice() }
        fn request(&self, choice: usize) -> ToggleRequest { self.toggle.request_choice(choice) }
    }

    fn unarmed_record_cannot_inherit_an_older_armed_key_for_its_slot() {
        for with_replacement_hook in [true, false] {
            let mut sim = Sim::new(true);
            assert_eq!(sim.start(7, 3, 0, true), Some(true));
            assert!(sim.push(0, 1));
            sim.stop(7, false);
            assert!(sim.pool.key_for_index(3).is_some());
            assert_eq!(sim.request(CAPTURE_OFF), ToggleRequest::Applied);
            if with_replacement_hook { sim.invalidate(7, 3); }
            assert_eq!(sim.ended.len(), 1);
            assert_eq!(sim.start(7, 3, 0, true), Some(false));
            assert!(sim.pool.key_for_index(3).is_none());
            assert!(sim.copy(3).is_none());
            assert_eq!(sim.owner.owner().map(|owner| owner.index), Some(3));
        }
    }

    fn nonzero_segment_continues_ownership_and_keeps_the_pending_latch() {
        let mut sim = Sim::new(true);
        assert_eq!(sim.start(7, 0, 0, true), Some(true));
        let first = sim.pool.active_key().unwrap();
        assert!(sim.push(0, 5));
        // Segment continuation: same record, new native index, separate timeline.
        assert_eq!(sim.start(7, 1, 1, true), Some(true));
        assert!(sim.pool.active_key().is_none());
        assert!(sim.pool.snapshot(first).unwrap().truncated);
        assert_eq!(sim.owner.owner().map(|owner| owner.index), Some(1));
        assert_eq!(sim.request(CAPTURE_OFF), ToggleRequest::Pending);
        sim.publish(7, 0, true);
        assert!(sim.ended.is_empty(), "the old index is no longer the owner");
        sim.serializer(7, Some((1, true)));
        assert!(sim.ended.is_empty());
        sim.publish(7, 1, true);
        assert_eq!(sim.ended, vec![RecordEnded { armed: true, desired: false }]);
        assert_eq!(sim.current(), Some(CAPTURE_OFF));
        // A continuation for a state nobody owns is not adopted.
        let mut idle = Sim::new(true);
        idle.state = 9;
        assert_eq!(idle.start(9, 2, 1, true), None);
        assert!(idle.owner.owner().is_none());
    }

    fn request_racing_the_start_latch_governs_that_record() {
        let toggle = CaptureToggle::new();
        toggle.seed(false);
        toggle.set_ready();
        // Native accepted the start; the controller edge lands before the game
        // thread latches: the answer is APPLIED and the record really is armed.
        assert_eq!(toggle.request_choice(CAPTURE_ON), ToggleRequest::Applied);
        assert!(toggle.begin_record());
        assert_eq!(toggle.current_choice(), Some(CAPTURE_ON));
        // The edge lands after the latch: PENDING, latched value shown, applied at the end.
        assert_eq!(toggle.request_choice(CAPTURE_OFF), ToggleRequest::Pending);
        assert_eq!(toggle.current_choice(), Some(CAPTURE_ON));
        assert!(toggle.is_armed());
        assert_eq!(toggle.end_record(), Some(RecordEnded { armed: true, desired: false }));
        assert_eq!(toggle.current_choice(), Some(CAPTURE_OFF));
        assert_eq!(toggle.end_record(), None);
        assert!(!toggle.begin_record());
        assert_eq!(toggle.end_record(), Some(RecordEnded { armed: false, desired: false }));
    }

    fn unavailable_until_ready_and_foreign_choices_are_rejected() {
        let toggle = CaptureToggle::new();
        toggle.seed(true);
        assert_eq!(toggle.current_choice(), None);
        assert_eq!(toggle.request_choice(CAPTURE_OFF), ToggleRequest::Unavailable);
        assert!(toggle.desired(), "an unavailable request changes nothing");
        assert!(!toggle.is_ready());
        toggle.set_ready();
        assert_eq!(toggle.request_choice(2), ToggleRequest::Rejected);
        assert_eq!(toggle.request_choice(usize::MAX), ToggleRequest::Rejected);
        assert_eq!(toggle.current_choice(), Some(CAPTURE_ON));
        assert_eq!(toggle.request_choice(CAPTURE_OFF), ToggleRequest::Applied);
        assert_eq!(toggle.snapshot(), ToggleSnapshot { desired: false, phase: RecordPhase::Idle, ready: true });
    }

    fn concurrent_requests_and_record_boundaries_stay_coherent() {
        let toggle = Arc::new(CaptureToggle::new());
        toggle.seed(false);
        toggle.set_ready();
        let ui = {
            let toggle = Arc::clone(&toggle);
            std::thread::spawn(move || {
                let (mut applied, mut pending) = (0u32, 0u32);
                for edge in 0..20_000usize {
                    match toggle.request_choice(edge & 1) {
                        ToggleRequest::Applied => applied += 1,
                        ToggleRequest::Pending => pending += 1,
                        other => panic!("ready toggle answered {other:?}"),
                    }
                }
                (applied, pending)
            })
        };
        let game = {
            let toggle = Arc::clone(&toggle);
            std::thread::spawn(move || {
                let mut pending_applied = 0u32;
                for _ in 0..5_000 {
                    let armed = toggle.begin_record();
                    // Only the game thread changes the phase, so the latched
                    // value is what the row shows until this record ends.
                    assert_eq!(toggle.phase().armed(), Some(armed));
                    assert_eq!(toggle.current_choice(), Some(choice(armed)));
                    std::hint::spin_loop();
                    let ended = toggle.end_record().expect("a latched record ends exactly once");
                    assert_eq!(ended.armed, armed);
                    pending_applied += u32::from(ended.pending_applied());
                    assert_eq!(toggle.phase(), RecordPhase::Idle);
                }
                pending_applied
            })
        };
        let (applied, pending) = ui.join().unwrap();
        assert_eq!(applied + pending, 20_000);
        let _ = game.join().unwrap();
        let settled = toggle.snapshot();
        assert_eq!(settled.phase, RecordPhase::Idle);
        assert!(settled.ready);
        assert_eq!(toggle.current_choice(), Some(choice(settled.desired)));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("unarmed_record_cannot_inherit_an_older_armed_key_for_its_slot", unarmed_record_cannot_inherit_an_older_armed_key_for_its_slot as fn()),
            ("nonzero_segment_continues_ownership_and_keeps_the_pending_latch", nonzero_segment_continues_ownership_and_keeps_the_pending_latch as fn()),
            ("request_racing_the_start_latch_governs_that_record", request_racing_the_start_latch_governs_that_record as fn()),
            ("unavailable_until_ready_and_foreign_choices_are_rejected", unavailable_until_ready_and_foreign_choices_are_rejected as fn()),
            ("concurrent_requests_and_record_boundaries_stay_coherent", concurrent_requests_and_record_boundaries_stay_coherent as fn()),
        ]);
    }
    }
}
