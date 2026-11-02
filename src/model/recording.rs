//! Input recording and playback (SPEC-input-01): five slots of 600 frames of P1's mapped input that
//! the CPU performs live while they are recorded and replays on demand. Pure: the runtime reads the
//! game, calls one of the three stage methods (`poll`, `control`, `post_frame`) and applies the answer.
//!
//! The recorder's three stages in one game frame (SPEC-input-01 §5.2):
//! 1. `poll`: P1's final mapped input (after input delay). Stores the frame when recording and makes
//!    P1's output neutral while the recorder owns P1 (lockout, standby, recording).
//! 2. `control`: the CPU's control frame, after the AI wrote its controls. Counts the lockout down,
//!    injects the current frame into the CPU, advances.
//! 3. `post_frame`: the end-of-run check, the loop, the crop, the mash queue pop.
//!
//! Decisions applied (docs/DECISIONS.md, the specs' *Decided* lines): B4 Record refused while an AI
//! drives P1; B5/N4 the setting `settings_override_playback`; B8 batch (full-range frame bounds, an
//! unrecorded or empty slot is refused, playback refused while recording, stop on reset, chords ignored
//! while a save state loads); per-slot facing and starting status (§6.1); the Recording Slot and the
//! duration are latched at the start of the lockout (SPEC-input-01 question 4).
//!
//! Frame bounds (question 1, B8): a recording of duration `L` stores frames `0..L` and playback plays
//! every stored frame, one per CPU control frame, so a slot of length `n` takes `n` control frames.

use super::cpu::ids::{Table, C, UNRESOLVED};
use super::cpu::playback::PlaybackState;
use super::cpu::rng::Rng;
use super::cpu::settings::Weights;
use super::input_delay::MappedInputs;
use super::settings::Snapshot;

pub const SLOTS: usize = 5;
/// Frames per slot: the largest Recording Duration.
pub const SLOT_FRAMES: usize = 600;
/// Frames of a named recording (	raining_mod_macros::MAX_FRAMES: ten minutes at 60 fps); the named capture lasts until the stop chord.
pub const NAMED_FRAMES: u16 = 36_000;
/// A slot's length before anything was recorded into it.
pub const INITIAL_LENGTH: u16 = 60;
/// Control frames of the lockout that follows the record chord (SPEC-input-01 §5.3).
pub const LOCKOUT_FRAMES: u16 = 30;
/// `ledge playback` buffer, and the one used while the CPU is still in `CLIFF_CATCH`.
pub const LEDGE_BUFFER: u8 = 5;
pub const CLIFF_CATCH_BUFFER: u8 = 4;
/// While the buffer is at most this many, the injected frame is neutral (a buffered press reads as a
/// press, not a hold).
pub const BUFFER_NEUTRAL_MAX: u8 = 3;
/// The CPU's cliff-wait rest timer is lifted to this while the recorder holds the CPU there.
pub const CLIFF_REST_TIMER: i32 = 200;
/// ... when it has fallen below this.
pub const CLIFF_REST_TIMER_LOW: i32 = 50;
/// The game's mapped stick range for full deflection, and the neutral threshold in stick units.
pub const STICK_FULL: f32 = 120.0;
pub const NEUTRAL_AXIS_LIMIT: i16 = 24;
/// Button bit that does not count against a neutral frame.
pub const FLICK_JUMP: u32 = 0x8000;

/// Whether `frame` is neutral: no buttons (or exactly FLICK_JUMP) and every stick axis below 0.2
/// (SPEC-input-01 §3.1).
pub fn is_neutral(frame: MappedInputs) -> bool {
    let axis = |v: i8| i16::from(v).abs() < NEUTRAL_AXIS_LIMIT;
    frame.buttons & !FLICK_JUMP == 0 && axis(frame.lstick_x) && axis(frame.lstick_y) && axis(frame.rstick_x) && axis(frame.rstick_y)
}

/// How the menu's Playback Hitstun Timing reads (`hitstun_playback`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitstunTiming {
    #[default]
    HitstunEnds,
    HitstopEnds,
    HitstopBegins,
}

/// The recording rows of the settings, as one `Copy` value the hooks read without allocating.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecorderSettings {
    pub slot: u8,
    /// Recording Trigger contains Button Combination / Save State Load.
    pub trigger_chord: bool,
    pub trigger_save_state: bool,
    pub duration: u16,
    pub crop: bool,
    pub playback_slots: Weights<5>,
    pub hitstun: HitstunTiming,
    pub save_state_slots: Weights<5>,
    pub mash_interrupt: bool,
    pub looping: bool,
    /// N4: the CPU settings override a playing recording (default Off).
    pub override_playback: bool,
}

impl RecorderSettings {
    pub const fn factory() -> Self {
        Self {
            slot: 0,
            trigger_chord: true,
            trigger_save_state: false,
            duration: 150,
            crop: true,
            playback_slots: Weights([1, 0, 0, 0, 0]),
            hitstun: HitstunTiming::HitstunEnds,
            save_state_slots: Weights([0; 5]),
            mash_interrupt: true,
            looping: false,
            override_playback: false,
        }
    }

    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        let row = |id: &str| snapshot.get(id).unwrap_or(&[]);
        let first = |id: &str| row(id).iter().position(|v| *v != 0);
        let on = |id: &str| row(id).first().is_some_and(|v| *v != 0);
        let trigger = row("record_trigger");
        Self {
            slot: first("recording_slot").map_or(0, |i| i.min(SLOTS - 1) as u8),
            trigger_chord: trigger.first().is_some_and(|v| *v != 0),
            trigger_save_state: trigger.get(1).is_some_and(|v| *v != 0),
            // 60, 90 .. 600 frames (19 options, step 30).
            duration: first("recording_duration").map_or(150, |i| (60 + 30 * i.min(18)) as u16),
            crop: on("recording_crop"),
            playback_slots: Weights::from_row(row("playback_button_slots")),
            hitstun: match first("hitstun_playback") {
                Some(1) => HitstunTiming::HitstopEnds,
                Some(2) => HitstunTiming::HitstopBegins,
                _ => HitstunTiming::HitstunEnds,
            },
            save_state_slots: Weights::from_row(row("save_state_playback")),
            mash_interrupt: on("playback_mash"),
            looping: on("playback_loop"),
            override_playback: on("settings_override_playback"),
        }
    }
}

impl Default for RecorderSettings {
    fn default() -> Self {
        Self::factory()
    }
}

/// What the recorder is doing (SPEC-input-01 §5.3 "Recorder state").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    None,
    /// The lockout after the record chord.
    Pause,
    Record,
    Playback,
}

/// Who holds the CPU's controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Possession {
    Player,
    Lockout,
    Standby,
    Cpu,
}

/// The phase the player sees: the tint and the HUD notice (SPEC-input-01 §2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Lockout,
    Standby,
    Recording,
    Playback,
}

impl Phase {
    /// The colour-blend tint of the CPU model while the phase lasts (every frame of lockout, standby
    /// and recording).
    pub const fn tint(self) -> Option<[f32; 3]> {
        match self {
            Phase::Lockout => Some([0.0, 0.0, 1.0]),
            Phase::Standby => Some([1.0, 0.0, 1.0]),
            Phase::Recording => Some([1.0, 0.0, 0.0]),
            _ => None,
        }
    }

    /// The notice body and its RGBA text colour; the header is `NOTICE_HEADER`.
    pub const fn notice(self) -> Option<(&'static str, [u8; 4])> {
        match self {
            Phase::Lockout => Some(("Lockout", [8, 8, 200, 255])),
            Phase::Standby => Some(("Standby", [200, 8, 200, 255])),
            Phase::Recording => Some(("Recording", [200, 8, 8, 255])),
            Phase::Playback => Some(("Playback", [0, 0, 0, 255])),
            Phase::Idle => None,
        }
    }
}

/// The header every recorder notice carries; the recorder clears it before it posts the next phase.
pub const NOTICE_HEADER: &str = "Input Recording";

/// What the CPU was when a slot's recording began (the fork's export contract, SPEC-input-01 §9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StartSnapshot {
    pub fighter_kind: i32,
    pub costume: u32,
    pub situation: i32,
    pub speed_x: f32,
    pub speed_y: f32,
}

impl StartSnapshot {
    pub const NONE: StartSnapshot = StartSnapshot { fighter_kind: 0, costume: 0, situation: 0, speed_x: 0.0, speed_y: 0.0 };
}

/// One slot's origin: facing at the lockout and the status the first frame led to (§6.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Origin {
    /// +1 or -1 (the CPU's `lr`).
    pub facing: i8,
    /// The CPU's status on the poll after frame 0, or `UNRESOLVED` until captured.
    pub status: i32,
    pub start: StartSnapshot,
}

impl Origin {
    pub const NONE: Origin = Origin { facing: 1, status: UNRESOLVED, start: StartSnapshot::NONE };
}

/// Why a recording was not started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordRefusal {
    /// B4: an AI drives P1 (the pilot or the fallback).
    P1DrivenByAi,
    /// The CPU or P1 body is missing (hot swap, hat load).
    FightersMissing,
    /// A save state is loading (§6.4).
    SaveStateLoading,
    /// The CPU is not in a state the recorder can own (preview / capture owners, SPEC-input-01 §6.12).
    Busy,
}

impl RecordRefusal {
    /// The HUD notice's text.
    pub const fn text(self) -> &'static str {
        match self {
            RecordRefusal::P1DrivenByAi => "P1 is controlled by an AI: recording is off",
            RecordRefusal::FightersMissing => "Both fighters must be present",
            RecordRefusal::SaveStateLoading => "Wait for the save state to load",
            RecordRefusal::Busy => "Recording is busy",
        }
    }
}

/// Why a playback was not started (decision B8: an unrecorded slot, or playback while recording).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayRefusal {
    /// The lockout is running.
    Lockout,
    /// A recording is running (question 17, B8).
    Recording,
    /// A slot index of 5 or more.
    SlotOutOfRange,
    /// Nothing recorded in that slot, or its length is 0 (question 11, B8).
    SlotEmpty,
    FightersMissing,
}

impl PlayRefusal {
    pub const fn text(self) -> &'static str {
        match self {
            PlayRefusal::Lockout => "Wait for the lockout to end",
            PlayRefusal::Recording => "Finish the recording first",
            PlayRefusal::SlotOutOfRange => "No such slot",
            PlayRefusal::SlotEmpty => "Record into this slot first",
            PlayRefusal::FightersMissing => "Both fighters must be present",
        }
    }
}

/// The CPU facts the poll stage needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PollFacts {
    /// The CPU's current status kind (read only when `wants_cpu_status`).
    pub cpu_status: i32,
}

/// The values the CPU's controller gets for one frame (SPEC-input-01 §4): the buttons, the raw sticks
/// and the clamped sticks. The right stick and the rumble bytes are never part of it (rule R6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inject {
    pub buttons: u32,
    pub stick_x: f32,
    pub stick_y: f32,
    pub clamped_x: f32,
    pub clamped_y: f32,
}

impl Inject {
    pub const NEUTRAL: Inject = Inject { buttons: 0, stick_x: 0.0, stick_y: 0.0, clamped_x: 0.0, clamped_y: 0.0 };
}

fn clamp1(v: f32) -> f32 {
    v.clamp(-1.0, 1.0)
}

/// A recorded frame as the CPU's controller values (SPEC-input-01 §5.3 "Injection", question 2 with
/// one scale, 1/120, the game's own full-deflection unit): horizontal inputs are mirrored by `mirror`
/// (+1 or -1); the clamped fields drop values below 0.2.
pub fn inject_values(frame: MappedInputs, mirror: i8) -> Inject {
    let m = f32::from(mirror);
    let x = clamp1(m * f32::from(frame.lstick_x) / STICK_FULL);
    let y = clamp1(f32::from(frame.lstick_y) / STICK_FULL);
    let dead = |v: f32| if v.abs() < 0.2 { 0.0 } else { v };
    Inject { buttons: frame.buttons, stick_x: x, stick_y: y, clamped_x: dead(x), clamped_y: dead(y) }
}

/// What `control` asks the runtime to do with the CPU's controller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlOut {
    /// Write these values (the whole recorder-owned set), or leave the AI's controls alone.
    pub write: Option<Inject>,
    /// The lockout or standby holds the CPU on the ledge: check the cliff-wait rest timer.
    pub hold_cliff_timer: bool,
}

/// The CPU facts the control stage needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlCtx {
    /// The CPU's `lr` now (+1 or -1).
    pub facing: i8,
    /// The CPU is Shulk and his Monado dial is open: no mirroring then.
    pub shulk_dial: bool,
}

/// What the post-frame stage found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PostOut {
    /// A recording or playback reached its last frame.
    pub ended: Option<Ended>,
    /// The mash queue head is a Playback Slot: pop it (mash `reset`), the playback it asked for is over.
    pub pop_mash: bool,
    /// Loop: a new playback of this slot started (the runtime re-reads facing; it was read from
    /// `PostCtx::facing`).
    pub restarted: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// The recording into `slot` finished; its stored length is `frames` (after the crop).
    Recorded { slot: u8, frames: u16 },
    Played { slot: u8 },
    /// A named capture reached its maximum length (the layer keeps the frames).
    NamedCaptured { frames: u16 },
    /// A named playback (or a clip preview) played to its end.
    NamedPlayed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PostCtx {
    /// The CPU's `lr` now (a loop restart re-reads it).
    pub facing: i8,
    /// The mash queue's head is a Playback Slot action.
    pub queue_head_is_playback: bool,
}

/// What the mash queue and the CPU's state say when the recorder is idle (§5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MashCtx {
    /// The slot of the queue head when it is a Playback Slot action.
    pub head_slot: Option<u8>,
    pub in_hitstun: bool,
    pub in_hitstop: bool,
    /// The transition term of `Recorder::term_for_slot(head)` is enabled on the CPU (the runtime asks
    /// only when `Recorder::mash_needs_term` says so).
    pub term_enabled: bool,
}

/// How a playback begins; the facing of the CPU is read now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayStart {
    pub slot: u8,
    pub frames: u16,
}

/// The recorder.
#[derive(Clone, Debug)]
pub struct Recorder {
    slots: [[MappedInputs; SLOT_FRAMES]; SLOTS],
    len: [u16; SLOTS],
    recorded: [bool; SLOTS],
    origin: [Origin; SLOTS],
    mode: Mode,
    possession: Possession,
    /// The next frame to inject (and, recording, the frame the next poll stores).
    frame: u16,
    lockout: u16,
    buffer: u8,
    /// The current run's length in frames.
    length: u16,
    record_slot: u8,
    play_slot: u8,
    recorded_facing: i8,
    current_facing: i8,
    starting_status: i32,
    /// The slot's status was taken after frame 0.
    status_captured: bool,
    /// Latched at the lockout: crop is read at the end, the duration was latched with `length`.
    mash_started: bool,
    /// The frame source is the named layer's buffer (SPEC-input-01 section 9), not a numbered slot.
    named: bool,
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Recorder {
    pub const fn new() -> Self {
        Self {
            slots: [[MappedInputs::NEUTRAL; SLOT_FRAMES]; SLOTS],
            len: [INITIAL_LENGTH; SLOTS],
            recorded: [false; SLOTS],
            origin: [Origin::NONE; SLOTS],
            mode: Mode::None,
            possession: Possession::Player,
            frame: 0,
            lockout: 0,
            buffer: 0,
            length: INITIAL_LENGTH,
            record_slot: 0,
            play_slot: 0,
            recorded_facing: 1,
            current_facing: 1,
            starting_status: UNRESOLVED,
            status_captured: false,
            mash_started: false,
            named: false,
        }
    }

    // ---- queries (SPEC-input-01 §5.6 and the CPU specs' interface) -----------------------------

    /// The frame source of the run is the named layer's buffer.
    pub const fn is_named(&self) -> bool {
        self.named
    }

    pub const fn mode(&self) -> Mode {
        self.mode
    }

    pub const fn possession(&self) -> Possession {
        self.possession
    }

    pub const fn frame(&self) -> u16 {
        self.frame
    }

    pub const fn length(&self) -> u16 {
        self.length
    }

    pub const fn record_slot(&self) -> u8 {
        self.record_slot
    }

    pub const fn play_slot(&self) -> u8 {
        self.play_slot
    }

    pub fn slot_len(&self, slot: u8) -> u16 {
        self.len.get(usize::from(slot)).copied().unwrap_or(0)
    }

    pub fn is_recorded(&self, slot: u8) -> bool {
        self.recorded.get(usize::from(slot)).copied().unwrap_or(false)
    }

    pub fn origin(&self, slot: u8) -> Option<Origin> {
        self.origin.get(usize::from(slot)).copied()
    }

    /// The stored frames of `slot` (its length), for export and the clip layer.
    pub fn frames(&self, slot: u8) -> Option<&[MappedInputs]> {
        let i = usize::from(slot);
        self.slots.get(i).map(|s| &s[..usize::from(self.len[i]).min(SLOT_FRAMES)])
    }

    /// `is_playback()` of the CPU specs: the recorder holds the CPU in Record or Playback.
    pub const fn is_playback(&self) -> bool {
        matches!(self.mode, Mode::Record | Mode::Playback)
    }

    pub const fn is_recording(&self) -> bool {
        matches!(self.mode, Mode::Record)
    }

    /// Lockout or standby.
    pub const fn is_standby(&self) -> bool {
        matches!(self.possession, Possession::Lockout | Possession::Standby)
    }

    /// The summary the CPU behaviours read (`Cpu::playback`); `queued` is the mash queue's.
    pub fn playback_state(&self, queued: bool) -> PlaybackState {
        PlaybackState {
            running: self.is_playback(),
            recording: self.is_recording(),
            standby: self.is_standby(),
            queued,
        }
    }

    /// The phase for the tint and the notice.
    pub const fn phase(&self) -> Phase {
        match (self.mode, self.possession) {
            (Mode::Pause, _) => Phase::Lockout,
            (Mode::Record, Possession::Standby) => Phase::Standby,
            (Mode::Record, _) => Phase::Recording,
            (Mode::Playback, _) => Phase::Playback,
            (Mode::None, _) => Phase::Idle,
        }
    }

    /// The poll stage reads the CPU's status only on the poll after frame 0.
    pub const fn wants_cpu_status(&self) -> bool {
        matches!(self.mode, Mode::Record) && matches!(self.possession, Possession::Cpu) && self.frame == 1 && !self.status_captured
    }

    // ---- starting and stopping -----------------------------------------------------------------

    /// The record chord or the save-state trigger. `facing` is the CPU's `lr`. The slot and duration
    /// are latched here (question 4).
    pub fn start_recording(&mut self, s: &RecorderSettings, facing: i8, start: StartSnapshot) {
        let slot = s.slot.min(SLOTS as u8 - 1);
        let i = usize::from(slot);
        self.slots[i] = [MappedInputs::NEUTRAL; SLOT_FRAMES];
        let duration = s.duration.clamp(1, SLOT_FRAMES as u16);
        self.len[i] = duration;
        self.recorded[i] = false;
        self.origin[i] = Origin { facing, status: UNRESOLVED, start };
        self.record_slot = slot;
        self.length = duration;
        self.mode = Mode::Pause;
        self.possession = Possession::Lockout;
        self.frame = 0;
        self.lockout = LOCKOUT_FRAMES;
        self.buffer = 0;
        self.recorded_facing = facing;
        self.current_facing = facing;
        self.starting_status = UNRESOLVED;
        self.status_captured = false;
        self.mash_started = false;
        self.named = false;
    }

    /// The named layer's record chord (SPEC-input-01 section 9 start_named_lockout): the same lockout, then the capture runs from
    /// frame 0 with the CPU possessed at once (no standby) for up to NAMED_FRAMES, until the stop chord. The frames live in the
    /// layer's buffer: poll_named hands each one out, control_named reads it back.
    pub fn start_named_lockout(&mut self, facing: i8) {
        self.mode = Mode::Pause;
        self.possession = Possession::Lockout;
        self.frame = 0;
        self.lockout = LOCKOUT_FRAMES;
        self.buffer = 0;
        self.length = NAMED_FRAMES;
        self.recorded_facing = facing;
        self.current_facing = facing;
        self.starting_status = UNRESOLVED;
        self.status_captured = true;
        self.mash_started = false;
        self.named = true;
    }

    /// The named layer's playback (playback_named, playback_preview): rames frames from the layer's buffer, mirrored by the
    /// recorded acing against the CPU's current_facing now. Refused during the lockout and a recording like any playback.
    pub fn start_named_playback(&mut self, frames: u16, recorded_facing: i8, status: i32, current_facing: i8) -> Result<PlayStart, PlayRefusal> {
        match self.mode {
            Mode::Pause => return Err(PlayRefusal::Lockout),
            Mode::Record => return Err(PlayRefusal::Recording),
            _ => {}
        }
        if frames == 0 || frames > NAMED_FRAMES {
            return Err(PlayRefusal::SlotEmpty);
        }
        self.length = frames;
        self.mode = Mode::Playback;
        self.possession = Possession::Player;
        self.frame = 0;
        self.buffer = 0;
        self.current_facing = current_facing;
        self.recorded_facing = recorded_facing;
        self.starting_status = status;
        self.named = true;
        Ok(PlayStart { slot: 0, frames })
    }

    /// The named stop chord: the capture ends now; returns the frames the layer holds (the frame index of the next poll).
    pub fn finish_named_capture(&mut self) -> Option<u16> {
        if !(self.named && self.mode == Mode::Record) {
            return None;
        }
        let frames = self.frame.min(NAMED_FRAMES);
        self.stop();
        Some(frames)
    }

    /// Begin playing `slot`; `facing` is the CPU's `lr` now.
    pub fn start_playback(&mut self, slot: u8, facing: i8) -> Result<PlayStart, PlayRefusal> {
        match self.mode {
            Mode::Pause => return Err(PlayRefusal::Lockout),
            Mode::Record => return Err(PlayRefusal::Recording),
            _ => {}
        }
        let i = usize::from(slot);
        if i >= SLOTS {
            return Err(PlayRefusal::SlotOutOfRange);
        }
        if !self.recorded[i] || self.len[i] == 0 || usize::from(self.len[i]) > SLOT_FRAMES {
            return Err(PlayRefusal::SlotEmpty);
        }
        self.length = self.len[i];
        self.play_slot = slot;
        self.mode = Mode::Playback;
        self.possession = Possession::Player;
        self.frame = 0;
        self.buffer = 0;
        self.current_facing = facing;
        self.recorded_facing = self.origin[i].facing;
        self.starting_status = self.origin[i].status;
        self.named = false;
        Ok(PlayStart { slot, frames: self.length })
    }

    /// The ledge feature's playback: the buffer is 5 frames, 4 when the CPU is still in `CLIFF_CATCH`.
    pub fn playback_ledge(&mut self, slot: u8, facing: i8, cliff_catch: bool) -> Result<PlayStart, PlayRefusal> {
        let start = self.start_playback(slot, facing)?;
        self.buffer = if cliff_catch { CLIFF_CATCH_BUFFER } else { LEDGE_BUFFER };
        Ok(start)
    }

    /// `stop_playback`: the state returns to None and the owner counters reset (decision B8: also
    /// buffer and lockout, §5.3). Called on a save-state load request, a mash interrupt, the session
    /// end, a hot swap, a Training reset and the named layer.
    pub fn stop(&mut self) {
        self.mode = Mode::None;
        self.possession = Possession::Player;
        self.frame = 0;
        self.buffer = 0;
        self.lockout = 0;
        self.mash_started = false;
        self.named = false;
    }

    // ---- the poll stage ------------------------------------------------------------------------

    /// P1's mapped input for this poll (after input delay). Stores the frame when recording; makes
    /// P1's output neutral while the recorder owns P1.
    pub fn poll(&mut self, out: &mut MappedInputs, facts: PollFacts) {
        match (self.mode, self.possession) {
            (Mode::Pause, _) => *out = MappedInputs::NEUTRAL,
            (Mode::Record, Possession::Standby) => {
                let i = usize::from(self.record_slot);
                self.slots[i][0] = *out;
                if !is_neutral(*out) {
                    self.possession = Possession::Cpu;
                    self.recorded[i] = true;
                }
                *out = MappedInputs::NEUTRAL;
            }
            (Mode::Record, Possession::Cpu) => {
                let i = usize::from(self.record_slot);
                if self.frame == 1 && !self.status_captured {
                    // The status the first frame led to: the CPU has performed frame 0 since.
                    self.origin[i].status = facts.cpu_status;
                    self.status_captured = true;
                }
                if self.frame < self.length {
                    self.slots[i][usize::from(self.frame)] = *out;
                }
                *out = MappedInputs::NEUTRAL;
            }
            _ => {}
        }
    }

    /// The poll stage of a named run: the frame the layer stores at the returned index (the capture's, from frame 0) while P1's
    /// output is made neutral. During the lockout only the neutralising happens.
    pub fn poll_named(&mut self, out: &mut MappedInputs) -> Option<(u16, MappedInputs)> {
        match (self.mode, self.possession) {
            (Mode::Pause, _) => {
                *out = MappedInputs::NEUTRAL;
                None
            }
            (Mode::Record, Possession::Cpu) if self.named => {
                let stored = (self.frame, *out);
                *out = MappedInputs::NEUTRAL;
                Some(stored)
            }
            _ => None,
        }
    }

    // ---- the control stage ---------------------------------------------------------------------

    /// One CPU control frame, after the AI wrote its controls.
    pub fn control(&mut self, c: &ControlCtx) -> ControlOut {
        self.control_from(c, &|_| None)
    }

    /// control of a named run: the frame at an index comes from the layer's buffer (None: not captured yet, the CPU gets neutral
    /// controls and the clock waits, SPEC-input-01 section 9 has_captured_frame).
    pub fn control_named(&mut self, c: &ControlCtx, frame: &dyn Fn(u16) -> Option<MappedInputs>) -> ControlOut {
        self.control_from(c, frame)
    }

    fn control_from(&mut self, c: &ControlCtx, named: &dyn Fn(u16) -> Option<MappedInputs>) -> ControlOut {
        let mut out = ControlOut { write: None, hold_cliff_timer: false };
        match self.mode {
            Mode::Pause => {
                out.hold_cliff_timer = true;
                if self.lockout > 0 {
                    self.lockout -= 1;
                }
                if self.lockout == 0 {
                    self.mode = Mode::Record;
                    // A named capture possesses the CPU at once: its frame 0 is the first poll after the lockout.
                    self.possession = if self.named { Possession::Cpu } else { Possession::Standby };
                }
            }
            Mode::Record => {
                let i = usize::from(self.record_slot);
                if self.possession == Possession::Standby {
                    out.hold_cliff_timer = true;
                    out.write = Some(self.injected(self.slots[i][0], c));
                } else if self.named {
                    // The poll of this frame stored it a moment ago; before the first poll there is nothing to perform.
                    match named(self.frame) {
                        Some(frame) => {
                            out.write = Some(self.injected(frame, c));
                            self.advance();
                        }
                        None => out.write = Some(Inject::NEUTRAL),
                    }
                } else {
                    out.write = Some(self.injected(self.slots[i][usize::from(self.frame.min(self.length - 1))], c));
                    self.advance();
                }
            }
            Mode::Playback => {
                let i = usize::from(self.play_slot);
                let at = self.frame.min(self.length.saturating_sub(1));
                let frame = if self.named { named(at).unwrap_or(MappedInputs::NEUTRAL) } else { self.slots[i][usize::from(at)] };
                out.write = Some(self.injected(frame, c));
                if self.buffer > 0 {
                    self.buffer -= 1;
                } else {
                    self.advance();
                }
            }
            Mode::None => {}
        }
        out
    }

    fn advance(&mut self) {
        if self.frame < self.length {
            self.frame += 1;
        }
    }

    fn injected(&self, frame: MappedInputs, c: &ControlCtx) -> Inject {
        // Record mirrors against the facing at the lockout, which is the CPU's own: 1.
        let mirror = if c.shulk_dial { 1 } else { self.recorded_facing * c.facing };
        let frame = if self.buffer > 0 && self.buffer <= BUFFER_NEUTRAL_MAX { MappedInputs::NEUTRAL } else { frame };
        inject_values(frame, mirror)
    }

    // ---- the post-frame stage ------------------------------------------------------------------

    /// The end-of-run check, the crop, the loop (SPEC-input-01 §5.3 "End detection").
    pub fn post_frame(&mut self, s: &RecorderSettings, c: &PostCtx) -> PostOut {
        let mut out = PostOut::default();
        if !matches!(self.mode, Mode::Record | Mode::Playback) || self.frame < self.length || self.buffer > 0 {
            return out;
        }
        self.possession = Possession::Player;
        out.pop_mash = c.queue_head_is_playback;
        let was = self.mode;
        if self.named {
            // The layer owns the frames; a capture that reached its maximum ends here like a stop chord.
            let frames = self.frame.min(NAMED_FRAMES);
            out.ended = Some(if was == Mode::Record { Ended::NamedCaptured { frames } } else { Ended::NamedPlayed });
            self.frame = 0;
            self.mode = Mode::None;
            self.named = false;
            return out;
        }
        if was == Mode::Record {
            let i = usize::from(self.record_slot);
            let mut n = self.frame;
            if s.crop && n > 0 {
                while n > 0 && is_neutral(self.slots[i][usize::from(n) - 1]) {
                    n -= 1;
                }
            }
            self.len[i] = n;
            self.length = n;
            self.recorded[i] = n > 0;
            out.ended = Some(Ended::Recorded { slot: self.record_slot, frames: n });
        } else {
            out.ended = Some(Ended::Played { slot: self.play_slot });
        }
        self.frame = 0;
        self.mode = Mode::None;
        if was == Mode::Playback && s.looping {
            let slot = self.play_slot;
            if self.start_playback(slot, c.facing).is_ok() {
                out.restarted = Some(slot);
            }
        }
        out
    }

    // ---- triggers ------------------------------------------------------------------------------

    /// A weighted random slot of Playback Button Slots (the plugin's own generator, question 12).
    pub fn pick_chord_slot(s: &RecorderSettings, rng: &mut Rng) -> Option<u8> {
        s.playback_slots.pick(rng).map(|i| i as u8)
    }

    /// Both chords passed in one poll: playback wins and the record request is dropped (§2.2).
    pub fn resolve_chords(record: bool, playback: bool) -> ChordAction {
        if playback {
            ChordAction::Playback
        } else if record {
            ChordAction::Record
        } else {
            ChordAction::None
        }
    }

    /// Both fighters finished restoring after a save-state load (§6.4): record if Recording Trigger
    /// contains Save State Load; else play a weighted random Save State Playback slot unless the saved
    /// situation is the ledge (ledge playbacks are a ledge option).
    pub fn on_state_restored(s: &RecorderSettings, saved_on_ledge: bool, rng: &mut Rng) -> LoadAction {
        if s.trigger_save_state {
            LoadAction::Record
        } else if !saved_on_ledge {
            s.save_state_slots.pick(rng).map_or(LoadAction::None, |i| LoadAction::Play(i as u8))
        } else {
            LoadAction::None
        }
    }

    // ---- the mash trigger (§5.4) ---------------------------------------------------------------

    /// The transition term that says the CPU can act again, derived from the slot's starting status.
    /// Ranges are inclusive at both ends (question 3 recommendation; upstream's half-open ranges left
    /// `ESCAPE_AIR_SLIDE` and `ESCAPE_B` in the "anything else" case). Anything else falls to the
    /// side-special term, upstream's acknowledged placeholder (open: needs a design decision).
    pub fn term_for_status(t: &Table, status: i32) -> C {
        if status == UNRESOLVED {
            return C::TermContSpecialS;
        }
        if t.is(C::StatusAttackAir, status) {
            C::TermContAttackAir
        } else if t.within(C::StatusEscapeAir, C::StatusEscapeAirSlide, status) {
            C::TermContEscapeAir
        } else if t.is(C::StatusSpecialHi, status) {
            C::TermContSpecialHi
        } else if t.is(C::StatusJumpSquat, status) {
            C::TermContJumpSquat
        } else if t.is(C::StatusJumpAerial, status) {
            C::TermContJumpAerial
        } else if t.is(C::StatusEscape, status) {
            C::TermContEscape
        } else if t.within(C::StatusEscapeF, C::StatusEscapeB, status) {
            C::TermContEscapeF
        } else if t.is(C::StatusCatch, status) {
            C::TermContCatch
        } else {
            C::TermContSpecialS
        }
    }

    /// The starting status of `slot` (from its origin), for `term_for_status`.
    pub fn starting_status_of(&self, slot: u8) -> i32 {
        self.origin.get(usize::from(slot)).map_or(UNRESOLVED, |o| o.status)
    }

    /// Whether the mash trigger evaluation needs the transition term of the queue head asked from the
    /// game (only then is the call made): the recorder is idle, the head is a playback, and the timing
    /// depends on the term (every case but hitstun with a hitstop timing).
    pub fn mash_needs_term(&self, s: &RecorderSettings, head: Option<u8>, in_hitstun: bool) -> bool {
        self.mode == Mode::None && head.is_some() && !(in_hitstun && s.hitstun != HitstunTiming::HitstunEnds)
    }
    /// Whether a playback the mash queue asked for starts now (§5.4 table).
    pub fn mash_should_start(&self, s: &RecorderSettings, c: &MashCtx) -> Option<u8> {
        let slot = c.head_slot?;
        if self.mode != Mode::None {
            return None;
        }
        let now = if c.in_hitstun {
            match s.hitstun {
                HitstunTiming::HitstopBegins => true,
                HitstunTiming::HitstopEnds => !c.in_hitstop,
                HitstunTiming::HitstunEnds => c.term_enabled,
            }
        } else {
            c.term_enabled
        };
        now.then_some(slot)
    }

    /// Mark that the last start came from the mash queue (so the runtime pops the head at the end).
    pub fn note_mash_started(&mut self) {
        self.mash_started = true;
    }

    /// Mash Interrupt: stop a playback when a mash trigger fires (`playback_mash`); never while a
    /// recording or the lockout owns the CPU.
    pub fn interrupt_on_mash(&mut self, s: &RecorderSettings) -> bool {
        if s.mash_interrupt && self.mode == Mode::Playback {
            self.stop();
            true
        } else {
            false
        }
    }

    // ---- the named layer's contract (§9) -------------------------------------------------------

    /// `import_named_slot`: replace one slot with `frames` (1..=600) and its origin, or change nothing.
    pub fn import_slot(&mut self, slot: u8, frames: &[MappedInputs], origin: Origin) -> Result<(), ImportRefusal> {
        let i = usize::from(slot);
        if i >= SLOTS {
            return Err(ImportRefusal::SlotOutOfRange);
        }
        if frames.is_empty() {
            return Err(ImportRefusal::Empty);
        }
        if frames.len() > SLOT_FRAMES {
            return Err(ImportRefusal::TooLong(frames.len()));
        }
        self.slots[i] = [MappedInputs::NEUTRAL; SLOT_FRAMES];
        self.slots[i][..frames.len()].copy_from_slice(frames);
        self.len[i] = frames.len() as u16;
        self.recorded[i] = true;
        self.origin[i] = origin;
        Ok(())
    }

    /// `export_named`: the frames and the origin of a recorded slot.
    pub fn export_slot(&self, slot: u8) -> Result<(&[MappedInputs], Origin), ExportRefusal> {
        let i = usize::from(slot);
        if i >= SLOTS {
            return Err(ExportRefusal::SlotOutOfRange);
        }
        if !self.recorded[i] || self.len[i] == 0 || usize::from(self.len[i]) > SLOT_FRAMES {
            return Err(ExportRefusal::NotRecorded);
        }
        Ok((&self.slots[i][..usize::from(self.len[i])], self.origin[i]))
    }

    /// Change the current facing (the clip layer sets it at a restored pose).
    pub fn set_current_facing(&mut self, facing: i8) {
        self.current_facing = facing;
    }

    pub const fn current_facing(&self) -> i8 {
        self.current_facing
    }

    pub const fn mash_started(&self) -> bool {
        self.mash_started
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChordAction {
    None,
    Record,
    Playback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadAction {
    None,
    Record,
    Play(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportRefusal {
    SlotOutOfRange,
    Empty,
    TooLong(usize),
}

impl ImportRefusal {
    pub fn message(self) -> String {
        match self {
            ImportRefusal::SlotOutOfRange => "Record Slot is out of range. Nothing was changed.".to_owned(),
            ImportRefusal::Empty => "Recording is empty. Nothing was changed.".to_owned(),
            ImportRefusal::TooLong(n) => format!("Recording is {n} frames; a Record Slot holds {SLOT_FRAMES}. Nothing was changed."),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportRefusal {
    SlotOutOfRange,
    NotRecorded,
}

impl ExportRefusal {
    pub const fn message(self) -> &'static str {
        match self {
            ExportRefusal::SlotOutOfRange => "Record Slot is out of range",
            ExportRefusal::NotRecorded => "Record into this slot first",
        }
    }
}
