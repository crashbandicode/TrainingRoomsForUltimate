//! CPU-role recording and Playback Slot control decisions.

use super::cpu::ids::{Table, C};
use super::input_delay::MappedInputs;
use super::settings::Snapshot;

pub const SLOTS: usize = 5;
/// Frames per slot: the largest Recording Duration.
pub const SLOT_FRAMES: usize = 600;
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
    pub trim: super::trim::Trim,
    pub indicator: bool,
    #[cfg(test)]
    pub delay: u16,
    pub place: bool,
    pub hitstun: HitstunTiming,
    pub mash_interrupt: bool,
}

impl RecorderSettings {
    pub const fn factory() -> Self {
        Self {
            trim: super::trim::Trim::End,
            indicator: true,
            #[cfg(test)]
            delay: 0,
            place: false,
            hitstun: HitstunTiming::HitstunEnds,
            mash_interrupt: true,
        }
    }

    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        let row = |id: &str| snapshot.get(id).unwrap_or(&[]);
        let first = |id: &str| row(id).iter().position(|v| *v != 0);
        let on = |id: &str| row(id).first().is_some_and(|v| *v != 0);
        let trim = super::trim::Trim::from_row(row("recording_crop"));
        Self {
            trim,
            indicator: on("playback_indicator"),
            #[cfg(test)]
            delay: 0,
            place: on("recording_place"),
            hitstun: match first("hitstun_playback") {
                Some(1) => HitstunTiming::HitstopEnds,
                Some(2) => HitstunTiming::HitstopBegins,
                _ => HitstunTiming::HitstunEnds,
            },
            mash_interrupt: on("playback_mash"),
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
}

/// Who holds the CPU's controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Possession {
    Player,
    Lockout,
    Standby,
    Cpu,
}

/// Light notice-body colours for the recording phases. Each clears 4.5:1 on `panes::NOTICE_PLATE` and has its phase's
/// tint hue: yellow, magenta, and the CPU's blue.
pub const LOCKOUT_COLOR: [u8; 4] = [255, 224, 112, 255];
pub const STANDBY_COLOR: [u8; 4] = [255, 156, 255, 255];
pub const RECORDING_COLOR: [u8; 4] = [148, 176, 255, 255];

/// The colour-blend tints of a recorded fighter (fix round 2026-11-23, F3). While its take records, a fighter has its
/// role's hue, the roster's scheme the HUD row's chip uses (`hud_row`): Player 1 red, the CPU blue. The CPU's two
/// phases before its take records keep hues of their own: lockout yellow (it was blue until then), standby magenta.
pub const PLAYER_TINT: [f32; 3] = [1.0, 0.0, 0.0];
pub const CPU_TINT: [f32; 3] = [0.0, 0.0, 1.0];
pub const LOCKOUT_TINT: [f32; 3] = [1.0, 1.0, 0.0];
pub const STANDBY_TINT: [f32; 3] = [1.0, 0.0, 1.0];

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
    /// and recording). Player 1's own take tints Player 1 (`PLAYER_TINT`, `rr_host::player_tint`).
    pub const fn tint(self) -> Option<[f32; 3]> {
        match self {
            Phase::Lockout => Some(LOCKOUT_TINT),
            Phase::Standby => Some(STANDBY_TINT),
            Phase::Recording => Some(CPU_TINT),
            _ => None,
        }
    }

    /// The notice body and its RGBA text colour; the header is `NOTICE_HEADER`.
    /// The colours are light enough to clear 4.5:1 on the dark notice plate. The hue
    /// stays each phase's tint: yellow, magenta, and blue for lockout, standby, and recording.
    pub const fn notice(self) -> Option<(&'static str, [u8; 4])> {
        match self {
            Phase::Lockout => Some(("Lockout", LOCKOUT_COLOR)),
            Phase::Standby => Some(("Standby", STANDBY_COLOR)),
            Phase::Recording => Some(("Recording", RECORDING_COLOR)),
            Phase::Playback => Some(("Playback", super::notify::DEFAULT_COLOR)),
            Phase::Idle => None,
        }
    }
}

/// The header every recorder notice carries; the recorder clears it before it posts the next phase.
pub const NOTICE_HEADER: &str = "Input Recording";

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

/// The first reason a recording cannot start, in the runtime's check order (`features/recorder.rs`
/// applies it to the live facts). B4 (SPEC-input-01 question 7, decided 2026-11-01): an AI-driven P1
/// is checked first — the pilot or the fallback drives entry 0 — then the missing fighters, then a
/// save state loading.
pub const fn record_refusal(
    p1_driven_by_ai: bool,
    entry0_present: bool,
    save_state_loading: bool,
) -> Option<RecordRefusal> {
    if p1_driven_by_ai {
        Some(RecordRefusal::P1DrivenByAi)
    } else if !entry0_present {
        Some(RecordRefusal::FightersMissing)
    } else if save_state_loading {
        Some(RecordRefusal::SaveStateLoading)
    } else {
        None
    }
}

/// A Record chord for the recording worker, which toggles a capture (`rr_host::on_chords`). A chord that would start one
/// asks `refusal` first, the same refusals as every other record start (B4 first); a chord that stops the running capture
/// always passes, and `refusal` is not asked. `Ok` is whether a Record goes to the worker; `Err` is the notice instead.
pub fn record_chord(
    record: bool,
    capturing: bool,
    refusal: impl FnOnce() -> Option<RecordRefusal>,
) -> Result<bool, RecordRefusal> {
    if !record || capturing {
        return Ok(record);
    }
    refusal().map_or(Ok(true), Err)
}

/// rc.9 C14: Play on Player 1 writes Player 1's input, as an AI driving P1 does (the standalone CPU pilot, or the game's
/// operator flag): its chord is refused then, with Record's B4 check (`p1_driven_by_ai`, asked only for a Play) and
/// notice style. `Ok` is whether the Play goes to the worker.
pub fn play_player_chord(play_player: bool, p1_driven_by_ai: impl FnOnce() -> bool) -> Result<bool, &'static str> {
    if play_player && p1_driven_by_ai() {
        Err(PLAY_PLAYER_AI_REFUSAL)
    } else {
        Ok(play_player)
    }
}

/// The refusal's notice, worded as Record's (`RecordRefusal::P1DrivenByAi`).
pub const PLAY_PLAYER_AI_REFUSAL: &str = "P1 is controlled by an AI: Play on Player 1 is off";

/// The standalone ssbu-cpu-pilot's state export, `ssbu_cpu_pilot_state_v1(out) -> bool` (ABI 1, its
/// `src/runtime.rs:510-538`, the same at 0162eed and 8b03e3b): `false` while the plugin is inactive.
pub type PilotStateExport = unsafe extern "C" fn(*mut PilotStateV1) -> bool;

/// The export's out-struct, field for field (`#[repr(C)]`, 24 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PilotStateV1 {
    pub abi_version: u32,
    pub settings: u32,
    /// 0 while P1 is the player, else the CPU level driving P1.
    pub armed_level: u8,
    pub reserved: [u8; 7],
    pub last_input: u64,
}

/// B4's `p1_driven_by_ai` for `record_refusal`: the game's operator flag on entry 0, TR's own fallback pilot (P1 CPU
/// Control), or the standalone pilot's own answer. Neither pilot drives P1 by flipping the flag (each arms the AI and
/// returns its decision as P1's mapped input, `core/src/native.rs:242`; the fallback leaves no operation-CPU change
/// behind), so the fallback says whether it is emitting P1's input now (`fallback`), and the standalone pilot's export
/// must answer and say `armed_level != 0`.
///
/// # Safety
/// `export` is the pilot's `ssbu_cpu_pilot_state_v1`, or another function with that ABI.
pub unsafe fn p1_driven_by_ai(operation_cpu: bool, fallback: bool, export: Option<PilotStateExport>) -> bool {
    let mut s = PilotStateV1::default();
    let answered = export.is_some_and(|f| unsafe { f(&mut s) });
    operation_cpu || fallback || (answered && s.abi_version >= 1 && s.armed_level != 0)
}

/// Why a playback was not started (decision B8: an unrecorded slot, or playback while recording).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayRefusal {
    /// A slot index of 5 or more.
    SlotOutOfRange,
    /// Nothing recorded in that slot, or its length is 0 (question 11, B8).
    SlotEmpty,
}

impl PlayRefusal {
    pub const fn text(self) -> &'static str {
        match self {
            PlayRefusal::SlotOutOfRange => "No such slot",
            // restriction: playback-slot-empty
            PlayRefusal::SlotEmpty => "This Playback Slot is empty: pick its macro on the Recording page",
        }
    }
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

/// Menu CPU-role record. The numbered slot table holds 600 frames; a menu file holds up to
/// `macros::MAX_FRAMES`, so this streams the stored frames out one at a time.
///
/// A press while the phase is still lockout is neutralised and dropped. The idle baseline
/// (`FLICK_JUMP`) during standby is injected and not stored. The first non-neutral poll ends
/// standby and is the first stored frame, which is also the frame the CPU performs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpuRoleDrive {
    mode: Mode,
    possession: Possession,
    lockout: u16,
    live: MappedInputs,
}

/// The phase a mode and possession show.
const fn phase_of(mode: Mode, possession: Possession) -> Phase {
    match (mode, possession) {
        (Mode::Pause, _) => Phase::Lockout,
        (Mode::Record, Possession::Standby) => Phase::Standby,
        (Mode::Record, _) => Phase::Recording,
        (Mode::None, _) => Phase::Idle,
    }
}

impl CpuRoleDrive {
    pub const fn idle() -> Self {
        Self { mode: Mode::None, possession: Possession::Player, lockout: 0, live: MappedInputs::NEUTRAL }
    }

    pub const fn active(&self) -> bool {
        !matches!(self.mode, Mode::None)
    }

    pub const fn phase(&self) -> Phase {
        phase_of(self.mode, self.possession)
    }

    #[cfg(test)]
    pub const fn possession(&self) -> Possession {
        self.possession
    }

    /// The record chord.
    pub fn begin(&mut self) {
        self.mode = Mode::Pause;
        self.possession = Possession::Lockout;
        self.lockout = LOCKOUT_FRAMES;
        self.live = MappedInputs::NEUTRAL;
    }

    pub fn begin_placed(&mut self) {
        self.mode = Mode::Record;
        self.possession = Possession::Cpu;
        self.lockout = 0;
        self.live = MappedInputs::NEUTRAL;
    }

    pub fn stop(&mut self) {
        *self = Self::idle();
    }

    /// P1's poll. Neutralises P1 while this drive owns the CPU. `Some` is the frame to store.
    pub fn poll(&mut self, out: &mut MappedInputs) -> Option<MappedInputs> {
        match (self.mode, self.possession) {
            (Mode::Pause, _) => {
                *out = MappedInputs::NEUTRAL;
                None
            }
            (Mode::Record, Possession::Standby) => {
                self.live = *out;
                let stored = if !is_neutral(*out) {
                    self.possession = Possession::Cpu;
                    Some(*out)
                } else {
                    None
                };
                *out = MappedInputs::NEUTRAL;
                stored
            }
            (Mode::Record, Possession::Cpu) => {
                let stored = *out;
                self.live = stored;
                *out = MappedInputs::NEUTRAL;
                Some(stored)
            }
            _ => None,
        }
    }

    /// The CPU control frame after the AI wrote its controls.
    pub fn control(&mut self) -> ControlOut {
        let mut out = ControlOut { write: None, hold_cliff_timer: false };
        match self.mode {
            Mode::Pause => {
                out.hold_cliff_timer = true;
                self.lockout = self.lockout.saturating_sub(1);
                if self.lockout == 0 {
                    self.mode = Mode::Record;
                    self.possession = Possession::Standby;
                }
            }
            Mode::Record => {
                if matches!(self.possession, Possession::Standby | Possession::Cpu) {
                    out.hold_cliff_timer = self.possession == Possession::Standby;
                    out.write = Some(inject_values(self.live, 1));
                }
            }
            Mode::None => {}
        }
        out
    }
}

/// What the mash queue and the CPU's state say when the recorder is idle (§5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MashCtx {
    /// The slot of the queue head when it is a Playback Slot action.
    pub head_slot: Option<u8>,
    pub in_hitstun: bool,
    pub in_hitstop: bool,
    /// The mapped term is enabled, or, for any other start, the fighter can act (the runtime asks
    /// only when `mash_needs_term` says so).
    pub term_enabled: bool,
}

/// How a playback begins; the facing of the CPU is read now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayStart {
    pub slot: u8,
    pub frames: u16,
}

/// One Playback Slot run on the CPU: the AI hook starts it, takes one frame per control frame, and ends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotRun { slot: u8, frame: u16, length: u16, buffer: u8, mirror: i8, generation: u32 }

impl SlotRun {
    pub const fn idle() -> Self {
        Self { slot: 0, frame: 0, length: 0, buffer: 0, mirror: 1, generation: 0 }
    }

    pub const fn active(&self) -> bool { self.length != 0 }
    pub const fn slot(&self) -> u8 { self.slot }
    pub const fn length(&self) -> u16 { self.length }
    pub const fn generation(&self) -> u32 { self.generation }
    #[cfg(test)]
    pub const fn frame(&self) -> u16 { self.frame }

    /// Facing is latched at the start; ledge playback buffers the first input.
    pub fn start(slot: u8, length: u16, recorded_facing: i8, facing: i8, ledge: Option<bool>, generation: u32) -> Self {
        Self { slot, frame: 0, length, buffer: match ledge {
            Some(true) => CLIFF_CATCH_BUFFER, Some(false) => LEDGE_BUFFER, None => 0,
        }, mirror: recorded_facing * facing, generation }
    }

    pub fn control(&mut self, shulk_dial: bool, frame_at: impl FnOnce(u16) -> Option<MappedInputs>) -> Option<Inject> {
        if !self.active() { return None; }
        let Some(frame) = frame_at(self.frame.min(self.length - 1)) else {
            self.stop();
            return None;
        };
        let frame = if self.buffer > 0 && self.buffer <= BUFFER_NEUTRAL_MAX { MappedInputs::NEUTRAL } else { frame };
        let out = inject_values(frame, if shulk_dial { 1 } else { self.mirror });
        if self.buffer > 0 { self.buffer -= 1; } else { self.frame = self.frame.saturating_add(1); }
        Some(out)
    }

    pub fn post(&mut self) -> Option<u8> {
        if !self.active() || self.frame < self.length || self.buffer > 0 { return None; }
        let slot = self.slot;
        self.stop();
        Some(slot)
    }

    pub fn stop(&mut self) { *self = Self::idle(); }
}

/// The mapped transition term for a slot's starting status; ranges are inclusive at both ends.
/// Anything else has no single term: wait until the fighter can act (`cpu::actionable`, SPEC-cpu-07 §3).
/// Upstream's side-special placeholder is not enabled for every fighter that can act: Greninja answers
/// no while idle and after hitstun, Mario answers yes while idle (F8 trace 2026-11-23).
pub fn term_for_status(t: &Table, status: i32) -> Option<C> {
    if t.is(C::StatusAttackAir, status) {
        Some(C::TermContAttackAir)
    } else if t.within(C::StatusEscapeAir, C::StatusEscapeAirSlide, status) {
        Some(C::TermContEscapeAir)
    } else if t.is(C::StatusSpecialHi, status) {
        Some(C::TermContSpecialHi)
    } else if t.is(C::StatusJumpSquat, status) {
        Some(C::TermContJumpSquat)
    } else if t.is(C::StatusJumpAerial, status) {
        Some(C::TermContJumpAerial)
    } else if t.is(C::StatusEscape, status) {
        Some(C::TermContEscape)
    } else if t.within(C::StatusEscapeF, C::StatusEscapeB, status) {
        Some(C::TermContEscapeF)
    } else if t.is(C::StatusCatch, status) {
        Some(C::TermContCatch)
    } else {
        None
    }
}

pub fn mash_needs_term(idle: bool, s: &RecorderSettings, head: Option<u8>, in_hitstun: bool) -> bool {
    idle && head.is_some() && !(in_hitstun && s.hitstun != HitstunTiming::HitstunEnds)
}

pub fn mash_should_start(idle: bool, s: &RecorderSettings, c: &MashCtx) -> Option<u8> {
    let slot = c.head_slot?;
    if !idle { return None; }
    let now = if c.in_hitstun {
        match s.hitstun {
            HitstunTiming::HitstopBegins => true,
            HitstunTiming::HitstopEnds => !c.in_hitstop,
            HitstunTiming::HitstunEnds => c.term_enabled,
        }
    } else { c.term_enabled };
    now.then_some(slot)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportRefusal {
    SlotOutOfRange,
    TooLong(usize),
}

impl ImportRefusal {
    pub fn message(self) -> String {
        match self {
            ImportRefusal::SlotOutOfRange => "No such Playback Slot. Nothing was changed.".to_owned(),
            ImportRefusal::TooLong(n) => format!("This macro is {n} frames; a Playback Slot holds {SLOT_FRAMES}. Crop it first."),
        }
    }
}
