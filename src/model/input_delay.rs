//! Input Delay (SPEC-input-03): P1's final mapped input is held back by 0 to 30 polls before the
//! input display and the recorder see it. A fixed ring of 31 eight-byte records in static data:
//! stepping it allocates nothing, so the hook can run it (playbook: no heap in hooks).
//!
//! The queue advances once per P1 poll of the game's final input mapping, not once per frame
//! (SPEC-input-03 §5.3, open question 1).

/// The largest delay in frames (the menu row has 31 options, `0..=30`).
pub const MAX_DELAY: u8 = 30;
/// Slots in the ring: a poll pushes one record before the oldest are dropped, so `MAX_DELAY + 1`.
const SLOTS: usize = MAX_DELAY as usize + 1;

/// The game's final mapped input record (SPEC-input-01 §3.1, 8 bytes). The all-zero record is
/// neutral: no buttons, both sticks centred.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct MappedInputs {
    pub buttons: u32,
    pub lstick_x: i8,
    pub lstick_y: i8,
    pub rstick_x: i8,
    pub rstick_y: i8,
}

const _: () = assert!(core::mem::size_of::<MappedInputs>() == 8);

impl MappedInputs {
    /// The record the fighter reads while the queue is still filling.
    pub const NEUTRAL: MappedInputs = MappedInputs { buttons: 0, lstick_x: 0, lstick_y: 0, rstick_x: 0, rstick_y: 0 };
}

/// P1's physical sticks, read from the game's raw `Controller` (SPEC-input-04 §3.1), in the record's stick unit. The
/// mapped record is not the physical stick: on the first frames of a C-stick attack the game writes the C-stick's
/// direction into the record's left stick and leaves its right stick 0 (Eden `rc5-opus-cstick1`, 2026-11-08). The
/// frame strip and the combo helper read these instead, so the stick row is the left stick and the C-stick arrow is
/// the C-stick (SPEC-input-06 §3.2, §3.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sticks {
    pub lx: i8,
    pub ly: i8,
    pub rx: i8,
    pub ry: i8,
}

/// The record's stick unit: full deflection is 120 (SPEC-input-01 §3.1, `recording::STICK_FULL`).
const STICK_UNIT: f32 = 120.0;

impl Sticks {
    pub const NEUTRAL: Sticks = Sticks { lx: 0, ly: 0, rx: 0, ry: 0 };

    /// The raw axes `[left x, left y, right x, right y]` (f32, ±1). `None` when one is not a stick value (not finite,
    /// or past ±1 by more than rounding): the layout guard, and the caller keeps the record's sticks.
    pub fn from_axes(axes: [f32; 4]) -> Option<Sticks> {
        if axes.iter().any(|v| !v.is_finite() || v.abs() > 1.001) {
            return None;
        }
        let unit = |v: f32| (v * STICK_UNIT).round() as i8;
        Some(Sticks { lx: unit(axes[0]), ly: unit(axes[1]), rx: unit(axes[2]), ry: unit(axes[3]) })
    }

    /// The record's own sticks, when the raw ones are not available.
    pub const fn of_record(record: MappedInputs) -> Sticks {
        Sticks { lx: record.lstick_x, ly: record.lstick_y, rx: record.rstick_x, ry: record.rstick_y }
    }

    /// What the strip and the combo helper read: the record's buttons with these sticks.
    pub const fn onto(self, record: MappedInputs) -> MappedInputs {
        MappedInputs { buttons: record.buttons, lstick_x: self.lx, lstick_y: self.ly, rstick_x: self.rx, rstick_y: self.ry }
    }

    /// What the strip and the combo helper read once the recorder has had P1's poll (`before` is the record it got,
    /// `after` the record it left). Play writes the played frame into the record, so the fighter acts on that frame:
    /// the strip reads it with its own sticks, and Make combo captures the macro's events as RECORDING.md says
    /// ("Make combo records the same events, because it plays into the same strip"). An untouched poll keeps the
    /// physical sticks.
    pub fn after_recorder(self, before: MappedInputs, after: MappedInputs) -> (MappedInputs, Sticks) {
        if after == before {
            (after, self)
        } else {
            (after, Sticks::of_record(after))
        }
    }
}

/// What owns P1's input for a poll. Anything other than gameplay empties the queue and makes that
/// poll neutral (SPEC-input-03 §5.1: "the queue is emptied whenever something other than gameplay
/// owns P1's input, and on session clear").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Owners(u8);

impl Owners {
    /// Gameplay owns the input.
    #[cfg(test)]
    pub const NONE: Owners = Owners(0);
    /// The plugin's menu or a modal panel captured the input, or closed within the last 15 frames.
    pub const MENU: Owners = Owners(1);
    /// The named stop chord was consumed this poll.
    pub const STOP_CHORD: Owners = Owners(2);
    /// The native waiting-room UI became busy.
    pub const NATIVE_UI_BUSY: Owners = Owners(4);
    /// The replay editor owns the poll.
    pub const REPLAY_EDITOR: Owners = Owners(8);
    /// The game's own pause menu is open (SPEC-input-03 question 3, recommended).
    pub const VANILLA_PAUSE: Owners = Owners(16);
    /// The replay editor is parked while a load it asked for runs (`replay_capture::loading()`: a recorded Mii's
    /// import, then the CPU's install). P1 stays neutral, but the gameplay gates do not wait for it (#68).
    pub const REPLAY_LOADING: Owners = Owners(32);

    /// The bits, for an atomic that other packages set (menu, chords, native UI, replay editor).
    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn from_bits(bits: u8) -> Owners {
        Owners(bits)
    }

    #[cfg(test)]
    pub const fn with(self, other: Owners) -> Owners {
        Owners(self.0 | other.0)
    }

    pub const fn contains(self, other: Owners) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn any(self) -> bool {
        self.0 != 0
    }

    /// What the carried gameplay gates (`menu::captures_input`) wait for: every owner but the stop chord (a poll the
    /// chord consumed) and the replay editor's own load. The fork's `captures_input` was the menu or a visible panel;
    /// the Replay window waits in its load for the very work those gates hold (#68).
    pub const fn holds_gameplay(self) -> bool {
        self.0 & !(Self::STOP_CHORD.0 | Self::REPLAY_LOADING.0) != 0
    }
}

/// Why the queue was emptied outside a poll.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clear {
    /// Scene change, hot swap or a lobby-practice change (SPEC-input-03 §5.1 "session clear").
    Session,
    /// A save state was loaded (decision B8, SPEC-input-03 question 2): a load repositions the
    /// fighters, so presses queued before it are never what the player meant.
    SaveStateLoad,
    /// Training was reset with L+R+A or by the game (decision B8; the reset hub's last step).
    TrainingReset,
    /// The game's own pause began or ended (SPEC-input-03 §5.1, question 3): whatever was queued
    /// before the pause, and whatever the pause menu's input produced, never reaches gameplay.
    NativePause,
}

/// The delay queue.
#[derive(Clone, Copy, Debug)]
pub struct DelayQueue {
    ring: [MappedInputs; SLOTS],
    /// The physical sticks of the same poll as `ring`'s record, so they come out with it.
    sticks: [Sticks; SLOTS],
    /// Index of the oldest record.
    head: usize,
    len: usize,
}

impl Default for DelayQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl DelayQueue {
    pub const fn new() -> Self {
        Self { ring: [MappedInputs::NEUTRAL; SLOTS], sticks: [Sticks::NEUTRAL; SLOTS], head: 0, len: 0 }
    }

    #[cfg(test)]
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Empty the queue; the next `delay` polls output neutral. Returns how many records it held
    /// (the diag build's hub line reports it).
    pub fn clear(&mut self, _why: Clear) -> usize {
        let held = self.len;
        self.head = 0;
        self.len = 0;
        held
    }

    fn push_newest(&mut self, record: MappedInputs, sticks: Sticks) {
        let at = (self.head + self.len) % SLOTS;
        self.ring[at] = record;
        self.sticks[at] = sticks;
        self.len += 1;
    }

    fn pop_oldest(&mut self) {
        self.head = (self.head + 1) % SLOTS;
        self.len -= 1;
    }

    /// One P1 poll with delay `delay` (clamped to `MAX_DELAY`) and the record the game just mapped.
    /// Returns the record the game should see (SPEC-input-03 §5).
    ///
    /// 0. Delay 0 empties the queue and passes `actual` through at once: whatever was queued at a
    ///    higher delay is never played, so the transition poll already returns the current record
    ///    (SPEC-input-03 §5: "setting the delay to 0 empties the queue on the next poll and passes
    ///    input through unchanged"; review R3).
    /// 1. `actual` is the poll's record.
    /// 2. Fewer than `delay` queued: neutral. Otherwise the oldest queued record.
    /// 3. `actual` is queued as the newest.
    /// 4. The oldest records are dropped until at most `delay` remain.
    ///
    /// Lowering a nonzero delay from `a` to `b` follows steps 1 to 4 as they are: this poll still
    /// plays the oldest record (`a` polls old), the `a - b` records between it and the new steady
    /// state are discarded unplayed, and from the next poll the output is `b` polls old, so the
    /// output jumps forward in time by `a - b` frames (SPEC-input-03 §5, "Decreasing d").
    #[cfg(test)]
    pub fn step(&mut self, delay: u8, actual: MappedInputs) -> MappedInputs {
        self.step_with(delay, actual, Sticks::of_record(actual)).0
    }

    /// `step`, with the poll's physical sticks carried beside the record and returned with it.
    pub fn step_with(&mut self, delay: u8, actual: MappedInputs, sticks: Sticks) -> (MappedInputs, Sticks) {
        let delay = usize::from(delay.min(MAX_DELAY));
        if delay == 0 {
            self.head = 0;
            self.len = 0;
            return (actual, sticks);
        }
        let out = if self.len < delay {
            (MappedInputs::NEUTRAL, Sticks::NEUTRAL)
        } else {
            (self.ring[self.head], self.sticks[self.head])
        };
        self.push_newest(actual, sticks);
        while self.len > delay {
            self.pop_oldest();
        }
        out
    }

    /// A poll while something other than gameplay owns P1's input: the queue is emptied and the
    /// poll outputs neutral (SPEC-input-03 §5.1). Otherwise `step`.
    #[cfg(test)]
    pub fn poll(&mut self, delay: u8, owners: Owners, actual: MappedInputs) -> MappedInputs {
        self.poll_with(delay, owners, actual, Sticks::of_record(actual)).0
    }

    /// `poll`, with the poll's physical sticks.
    pub fn poll_with(&mut self, delay: u8, owners: Owners, actual: MappedInputs, sticks: Sticks) -> (MappedInputs, Sticks) {
        if owners.any() {
            self.clear(Clear::Session);
            return (MappedInputs::NEUTRAL, Sticks::NEUTRAL);
        }
        self.step_with(delay, actual, sticks)
    }
}

/// The stages of one P1 poll of the final input mapping, in order (SPEC-input-03 §5.2, SPEC-input-01
/// §5.1): the menu and the chords read the raw controller before the delay, so they are never
/// delayed; the input display and the recorder run after it and see delayed input.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollStage {
    MenuInput,
    ChordDetection,
    InputDelay,
    InputDisplay,
    Recorder,
}

#[cfg(test)]
pub const POLL_ORDER: [PollStage; 5] =
    [PollStage::MenuInput, PollStage::ChordDetection, PollStage::InputDelay, PollStage::InputDisplay, PollStage::Recorder];

/// Whether the delay stage acts on a poll: only P1's (`player_idx == 0`) while Training (or an
/// active practice session) is live (SPEC-input-03 §4).
pub const fn applies(player_idx: i32, training_live: bool) -> bool {
    player_idx == 0 && training_live
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn rec(n: u32) -> MappedInputs {
        MappedInputs { buttons: n, ..MappedInputs::NEUTRAL }
    }

    fn two_polls_late() {
        let mut q = DelayQueue::new();
        let out: Vec<_> = (1..=5).map(|n| q.step(2, rec(n)).buttons).collect();
        assert_eq!(out, [0, 0, 1, 2, 3]);
    }

    /// #68 (cov-rec diag2/diag3, 2026-11-08): the Replay window's Mii track stayed on "Reading recorded Mii..." for 15 s.
    /// The window waits in `replay_capture::loading()` for the capture's Mii import, the import's service waits until
    /// `menu::captures_input()` is false, and that read the replay editor's owner bit, which `sync_owner` set for the
    /// load itself. The fork's `captures_input` was the menu or a visible panel; a parked editor waiting on its load was
    /// neither. The load keeps P1 neutral (an owner) but is not one the gameplay gates wait for.
    fn a_replay_load_keeps_p1_neutral_but_does_not_hold_the_gameplay_gates() {
        assert!(Owners::REPLAY_LOADING.any(), "P1 stays neutral while the editor waits");
        let mut q = DelayQueue::new();
        q.step(3, rec(1));
        assert_eq!(q.poll(3, Owners::REPLAY_LOADING, rec(2)), MappedInputs::NEUTRAL);
        assert!(!Owners::REPLAY_LOADING.holds_gameplay());
        assert!(!Owners::STOP_CHORD.with(Owners::REPLAY_LOADING).holds_gameplay());
        assert!(!Owners::NONE.holds_gameplay());
        for owner in [Owners::MENU, Owners::NATIVE_UI_BUSY, Owners::REPLAY_EDITOR, Owners::VANILLA_PAUSE] {
            assert!(owner.holds_gameplay(), "{owner:?}");
            assert!(owner.with(Owners::REPLAY_LOADING).holds_gameplay(), "{owner:?}");
        }
        let common = include_str!("../runtime/compat/common.rs");
        assert!(common.contains("published::input_owners().holds_gameplay()"));
        let macros = include_str!("../runtime/macros/mod.rs");
        assert!(macros.contains("let loading = replay_capture::loading();"));
        assert!(macros.contains("Owners::REPLAY_LOADING"));
        let recorder = include_str!("../runtime/features/recorder.rs");
        assert!(recorder.contains("owners.contains(Owners::REPLAY_LOADING)"), "the chords stay modal while it loads");
    }

    fn an_owner_empties_the_queue() {
        let mut q = DelayQueue::new();
        q.step(3, rec(1));
        assert_eq!(q.poll(3, Owners::MENU, rec(2)), MappedInputs::NEUTRAL);
        assert!(q.is_empty());
    }

    fn left(x: i8) -> Sticks {
        Sticks { lx: x, ..Sticks::NEUTRAL }
    }

    fn the_physical_sticks_come_out_with_their_own_record() {
        let mut q = DelayQueue::new();
        let out: Vec<_> = (1..=5).map(|n| q.poll_with(2, Owners::NONE, rec(n), left(n as i8 * 10))).collect();
        assert_eq!(out.iter().map(|(r, _)| r.buttons).collect::<Vec<_>>(), [0, 0, 1, 2, 3]);
        assert_eq!(out.iter().map(|(_, s)| s.lx).collect::<Vec<_>>(), [0, 0, 10, 20, 30], "the same delay as the record");
        assert_eq!(q.poll_with(2, Owners::MENU, rec(6), left(60)), (MappedInputs::NEUTRAL, Sticks::NEUTRAL));
        assert!(q.is_empty());
        assert_eq!(q.poll_with(0, Owners::NONE, rec(7), left(70)), (rec(7), left(70)), "delay 0 passes both through");
    }

    fn raw_axes_are_record_units_and_a_value_that_is_not_a_stick_is_refused() {
        assert_eq!(Sticks::from_axes([0.0, 0.6, 1.0, -1.0]), Some(Sticks { lx: 0, ly: 72, rx: 120, ry: -120 }));
        assert_eq!(Sticks::from_axes([f32::NAN, 0.0, 0.0, 0.0]), None);
        assert_eq!(Sticks::from_axes([0.0, 0.0, 1.5, 0.0]), None);
        let record = MappedInputs { buttons: 0x80a1, lstick_x: 100, lstick_y: 0, rstick_x: 0, rstick_y: 0 };
        let onto = Sticks { lx: 0, ly: 72, rx: 120, ry: 0 }.onto(record);
        assert_eq!((onto.buttons, onto.lstick_x, onto.lstick_y, onto.rstick_x), (0x80a1, 0, 72, 120));
        assert_eq!(Sticks::of_record(record).onto(record), record);
    }

    /// Eden rc6-covrec-diag1 (2026-11-08): Make combo saved its combo with no input events. The strip read P1's record
    /// before Play wrote the played frame into it, so it saw the idle controller. The display now reads the record
    /// the recorder left: a played jab is a press with the played sticks; an untouched poll keeps the physical sticks.
    fn the_display_reads_the_played_frame_and_keeps_the_physical_sticks_otherwise() {
        let physical = Sticks { lx: 0, ly: 72, rx: 0, ry: 0 };
        let idle = MappedInputs { buttons: 0x8000, lstick_x: 0, lstick_y: 0, rstick_x: 0, rstick_y: 0 };
        let played = MappedInputs { buttons: 0x8201, lstick_x: 60, lstick_y: 0, rstick_x: 0, rstick_y: 0 };
        let (record, sticks) = physical.after_recorder(idle, played);
        assert_eq!(sticks.onto(record), played, "the strip reads the played frame");
        let (record, sticks) = physical.after_recorder(idle, idle);
        assert_eq!(sticks.onto(record), physical.onto(idle), "an untouched poll keeps the physical sticks");
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("two_polls_late", two_polls_late as fn()),
            ("a_replay_load_keeps_p1_neutral_but_does_not_hold_the_gameplay_gates", a_replay_load_keeps_p1_neutral_but_does_not_hold_the_gameplay_gates as fn()),
            ("an_owner_empties_the_queue", an_owner_empties_the_queue as fn()),
            ("the_physical_sticks_come_out_with_their_own_record", the_physical_sticks_come_out_with_their_own_record as fn()),
            ("raw_axes_are_record_units_and_a_value_that_is_not_a_stick_is_refused", raw_axes_are_record_units_and_a_value_that_is_not_a_stick_is_refused as fn()),
            ("the_display_reads_the_played_frame_and_keeps_the_physical_sticks_otherwise", the_display_reads_the_played_frame_and_keeps_the_physical_sticks_otherwise as fn()),
        ]);
    }
    }
}