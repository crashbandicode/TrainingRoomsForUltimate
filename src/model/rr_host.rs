//! Recording worker decisions the host can run. The Switch wrapper performs the notices,
//! the save-restore request, and the keyboard offer this module names.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use super::library::{
    Chain, Drill, DrillKind, Kind, LegacySlot, LoopCount, NamedSave, RecordingIndex, Role, Sequence,
    StartSaveState, Use, MACROS_DIR, SAVES_DIR, SEQUENCES_DIR,
};
use super::frame_queue::FrameQueue;
use super::macro_ui::clip_marks::{Seen, Span, Watch, WATCH_SPANS};
use super::macro_ui::play_track::PLAY;
use super::macro_ui::scrub_track::TAKE;
use super::macros::{InputFrame, Library, MacroRecording, MomentumMode, Placement, ReplayFrame, StartState, MAX_FRAMES};
use super::playback::{Book, Playback, StartLink};
use super::rr_runtime::{
    chord_notice, civil_date, delete_save,
    drive_mash, drive_play, drive_step, duplicate_save, finish_recording, migrate_at_boot,
    push_frame, save_chord, save_made_combo, start_save_notice, toggle_record, unused_name, Capture,
    KeyboardOffer, EMPTY_COMBO,
};
use crate::model::action::Late;
use crate::model::combo::{CaptureOwner, Combo, InputCapture};
use crate::model::frame_strip::Snap;
use crate::model::combo_record::{ComboLibrary, ComboRecord};
use crate::model::cpu::playback::PlaybackState;
use crate::model::input_delay::MappedInputs;
#[cfg(any(test, target_os = "switch"))]
use crate::model::hud_row::Take;
use crate::model::recording::{inject_values, ControlOut, CpuRoleDrive, Inject, Phase};
use crate::model::recording::{ImportRefusal, PlayRefusal, PlayStart, SlotRun, SLOTS, SLOT_FRAMES};
use crate::model::record_place::{Commit, Shared};
use crate::model::recording_menu::{Effect, FileKind, LoopMark, Selection, ViewRole};
use crate::model::state::record::{SavedState, StartState as PlacementStartState};
use crate::model::store::{self, Record, Store};
use crate::model::trim::Trim;

const ROLE_PLAYER: u8 = 1;
const ROLE_CPU: u8 = 2;
pub use super::macro_ui::clip_marks::CAPTURE_RUN;

static SLOT_SEQ: [AtomicU32; SLOTS] = [const { AtomicU32::new(0) }; SLOTS];
static SLOT_TAPES: [crate::model::macro_handoff::PlayBuffers<InputFrame>; SLOTS] =
    [const { crate::model::macro_handoff::PlayBuffers::new() }; SLOTS];
static SLOT_META: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];
static SLOT_PREPARED: [AtomicBool; SLOTS] = [const { AtomicBool::new(false) }; SLOTS];
static SLOT_RUN: hos_lock::Mutex<SlotRun> = hos_lock::Mutex::new(SlotRun::idle());
static SLOT_RUNNING: AtomicBool = AtomicBool::new(false);
static SLOT_OWED: AtomicU8 = AtomicU8::new(0);
static SLOT_REFUSAL: AtomicBool = AtomicBool::new(false);

pub fn set_slot(slot: usize, recording: Option<&MacroRecording>) -> Result<(), ImportRefusal> {
    if slot >= SLOTS { return Err(ImportRefusal::SlotOutOfRange); }
    if let Some(recording) = recording {
        let len = recording.trim.window(&recording.frames).len();
        if len > SLOT_FRAMES { return Err(ImportRefusal::TooLong(len)); }
    }
    SLOT_SEQ[slot].fetch_add(1, Ordering::SeqCst);
    let result = set_slot_data(slot, recording);
    SLOT_SEQ[slot].fetch_add(1, Ordering::SeqCst);
    result
}

fn set_slot_data(slot: usize, recording: Option<&MacroRecording>) -> Result<(), ImportRefusal> {
    if slot >= SLOTS { return Err(ImportRefusal::SlotOutOfRange); }
    if let Some(recording) = recording {
        let len = recording.trim.window(&recording.frames).len();
        if len > SLOT_FRAMES { return Err(ImportRefusal::TooLong(len)); }
    }
    let tape = &SLOT_TAPES[slot];
    // SAFETY: the recording worker is the single slot publisher.
    unsafe {
        if !SLOT_PREPARED[slot].load(Ordering::Acquire) {
            tape.prepare(SLOT_FRAMES);
            SLOT_PREPARED[slot].store(true, Ordering::Release);
        }
        if let Some(recording) = recording.filter(|r| !r.trim.window(&r.frames).is_empty()) {
            let facing = u64::from(recording.start.facing < 0.0);
            let status = u64::from(recording.start.status as u32);
            SLOT_META[slot].store(status | facing << 32, Ordering::SeqCst);
            tape.fill(&recording.frames[recording.trim.window(&recording.frames)]);
        } else {
            tape.retract();
        }
    }
    Ok(())
}

fn slot_snapshot(slot: usize) -> Option<(u32, u64, usize, u32)> {
    let before = SLOT_SEQ[slot].load(Ordering::SeqCst);
    if before & 1 != 0 { return None; }
    let meta = SLOT_META[slot].load(Ordering::SeqCst);
    let len = SLOT_TAPES[slot].len();
    let generation = SLOT_TAPES[slot].generation();
    (SLOT_SEQ[slot].load(Ordering::SeqCst) == before).then_some((before, meta, len, generation))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotTake { seq: u32, status: i32 }

impl SlotTake {
    pub fn status(&self) -> i32 { self.status }
}

pub fn slot_take(slot: u8) -> Option<SlotTake> {
    let at = usize::from(slot);
    if at >= SLOTS { return Some(SlotTake { seq: 0, status: crate::model::cpu::ids::UNRESOLVED }); }
    let (seq, meta, len, _) = slot_snapshot(at)?;
    let status = if len == 0 { crate::model::cpu::ids::UNRESOLVED } else { meta as u32 as i32 };
    Some(SlotTake { seq, status })
}

fn start_slot_snapshot(slot: u8, facing: i8, ledge: Option<bool>, meta: u64, len: usize, generation: u32)
    -> Result<Option<PlayStart>, PlayRefusal>
{
    if len == 0 { return Err(PlayRefusal::SlotEmpty); }
    let recorded_facing = if meta & (1 << 32) != 0 { -1 } else { 1 };
    *SLOT_RUN.lock() = SlotRun::start(slot, len as u16, recorded_facing, facing, ledge, generation);
    SLOT_RUNNING.store(true, Ordering::Release);
    Ok(Some(PlayStart { slot, frames: len as u16 }))
}

pub fn slot_start_mash(slot: u8, take: SlotTake, facing: i8) -> Result<Option<PlayStart>, PlayRefusal> {
    SLOT_OWED.store(0, Ordering::Release);
    let at = usize::from(slot);
    if at >= SLOTS { return Err(PlayRefusal::SlotOutOfRange); }
    let Some((seq, meta, len, generation)) = slot_snapshot(at) else { return Ok(None) };
    if seq != take.seq { return Ok(None); }
    start_slot_snapshot(slot, facing, None, meta, len, generation)
}

pub fn slot_start_ledge(slot: u8, facing: i8, ledge: Option<bool>) -> Result<Option<PlayStart>, PlayRefusal> {
    SLOT_OWED.store(0, Ordering::Release);
    let at = usize::from(slot);
    if at >= SLOTS { return Err(PlayRefusal::SlotOutOfRange); }
    let Some((_, meta, len, generation)) = slot_snapshot(at) else {
        let buffer = match ledge { None => 0, Some(false) => 1, Some(true) => 2 };
        let request = (slot + 1) | (u8::from(facing < 0) << 3) | (buffer << 4);
        SLOT_OWED.store(request, Ordering::Release);
        return Ok(None);
    };
    start_slot_snapshot(slot, facing, ledge, meta, len, generation)
}

/// A ledge playback was asked for and waits for its slot's publication. Atomic read (the transition-term hook asks it).
pub fn slot_owed() -> bool { SLOT_OWED.load(Ordering::Acquire) != 0 }

pub fn slot_running() -> bool {
    SLOT_RUNNING.load(Ordering::Acquire) || slot_owed()
}

pub fn slot_stop() -> bool {
    let owed = SLOT_OWED.swap(0, Ordering::AcqRel) != 0;
    SLOT_REFUSAL.store(false, Ordering::Release);
    let mut run = SLOT_RUN.lock();
    let running = run.active() || owed;
    run.stop();
    SLOT_RUNNING.store(false, Ordering::Release);
    running
}

pub fn slot_post() -> Option<u8> {
    let mut run = SLOT_RUN.lock();
    let ended = run.post();
    SLOT_RUNNING.store(run.active(), Ordering::Release);
    ended
}

pub fn slot_phase() -> (Phase, u16) {
    let run = SLOT_RUN.lock();
    if run.active() { (Phase::Playback, run.length()) } else { (Phase::Idle, 0) }
}

pub fn cpu_idle() -> bool { !slot_running() && !cpu_playback_blocks_drill() }

pub fn take_slot_refusal() -> Option<PlayRefusal> {
    SLOT_REFUSAL.swap(false, Ordering::AcqRel).then_some(PlayRefusal::SlotEmpty)
}

pub fn slot_control(shulk_dial: bool) -> Option<Inject> {
    let request = SLOT_OWED.load(Ordering::Acquire);
    if request != 0 {
        let slot = (request & 7) - 1;
        let facing = if request & 8 != 0 { -1 } else { 1 };
        let ledge = match request >> 4 { 1 => Some(false), 2 => Some(true), _ => None };
        let (_, meta, len, generation) = slot_snapshot(usize::from(slot))?;
        SLOT_OWED.store(0, Ordering::Release);
        if len == 0 {
            SLOT_REFUSAL.store(true, Ordering::Release);
            return None;
        }
        start_slot_snapshot(slot, facing, ledge, meta, len, generation).ok()??;
    }
    let mut run = SLOT_RUN.lock();
    if !run.active() { return None; }
    let tape = &SLOT_TAPES[usize::from(run.slot())];
    let generation = run.generation();
    let out = run.control(shulk_dial, |index| {
        if tape.generation() != generation { return None; }
        let frame = tape.frame_at(usize::from(index));
        (tape.generation() == generation).then_some(frame).flatten().map(to_mapped)
    });
    SLOT_RUNNING.store(run.active(), Ordering::Release);
    out
}

static RECORD_ROLE: AtomicU8 = AtomicU8::new(ROLE_PLAYER);
/// Ask before switching fighter (published with the settings): Off keeps the current fighter for every question here.
static ASK_FIGHTER: AtomicBool = AtomicBool::new(true);
static LAST_ROLE: AtomicU8 = AtomicU8::new(ROLE_PLAYER);
static CAP_ROLE: AtomicU8 = AtomicU8::new(0);
static CAP_LEN: AtomicUsize = AtomicUsize::new(0);
static PLAYED: AtomicU32 = AtomicU32::new(0);
static FED: [AtomicU64; 2] = [const { AtomicU64::new(0) }; 2];
static RUN_GEN: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static RUNS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static CAP_FRAMES: [AtomicU64; MAX_FRAMES] = [const { AtomicU64::new(0) }; MAX_FRAMES];
// Final converted event records only. CAP_FRAMES stays exclusively owned by ordinary Record mechanics.
static INPUT_MODE: AtomicU8 = AtomicU8::new(0); // bit1 Make, bit2 direct Record; either may overlap another take
static INPUT_EPOCH: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
static INPUT_POLLS: [AtomicU32; 2] = [const { AtomicU32::new(0) }; 2];
#[derive(Clone, Copy)]
struct InputTake { epoch: u32, polls: u32, capture: InputCapture, contact: bool }
impl InputTake { const EMPTY: Self = Self { epoch: 0, polls: 0, capture: InputCapture::new(), contact: false }; }
// Game threads alone take this lock; the worker reads the published result, never the writer lock.
static INPUT_STATE: hos_lock::Mutex<[InputTake; 2]> = hos_lock::Mutex::new([InputTake::EMPTY; 2]);
static INPUT_RESULT: [hos_lock::TripleBuffer<InputTake>; 2] = [const { hos_lock::TripleBuffer::new(InputTake::EMPTY) }; 2];
static MAKE_ROLE: AtomicU8 = AtomicU8::new(0);
static MAKE_CPU_FRAME: AtomicU64 = AtomicU64::new(0);
static DRILL_ARMED: AtomicBool = AtomicBool::new(false);
static SAVED_DRILL: Mutex<Option<String>> = Mutex::new(None);
pub fn arm_drill_record(on: bool) { DRILL_ARMED.store(on, Ordering::Release); }
pub fn take_saved_drill() -> Option<String> { lock(&SAVED_DRILL).take() }
pub fn input_observation_wanted() -> bool { INPUT_MODE.load(Ordering::Acquire) != 0 }
fn begin_input(mode: u8) {
    let at = mode.trailing_zeros() as usize;
    INPUT_MODE.fetch_and(!mode, Ordering::AcqRel); INPUT_POLLS[at].store(0, Ordering::Release);
    INPUT_EPOCH[at].fetch_add(1, Ordering::AcqRel); INPUT_MODE.fetch_or(mode, Ordering::Release);
}
pub fn observe_input(snap: Snap, late: Late, window: u8, contact: bool) {
    if !input_observation_wanted() { return; }
    let mut state = INPUT_STATE.lock();
    for (at, take) in state.iter_mut().enumerate() {
        if INPUT_MODE.load(Ordering::Acquire) & (1 << at) == 0 { continue; }
        let epoch = INPUT_EPOCH[at].load(Ordering::Acquire); let polls = INPUT_POLLS[at].load(Ordering::Acquire);
        if polls == 0 { continue; }
        if take.epoch != epoch { *take = InputTake { epoch, ..InputTake::EMPTY }; }
        if take.polls != polls { take.capture.set_window(window); take.capture.step_late(snap, late); take.polls = polls; }
        take.contact |= contact; INPUT_RESULT[at].write(*take);
    }
}
fn converted_input(mode: u8) -> Option<InputTake> {
    let at = mode.trailing_zeros() as usize; let polls = INPUT_POLLS[at].load(Ordering::Acquire);
    if polls == 0 { return Some(InputTake::EMPTY); }
    let take = INPUT_RESULT[at].read().0;
    (take.epoch == INPUT_EPOCH[at].load(Ordering::Acquire) && take.polls == polls).then_some(take)
}
// restriction: drill-capacity
pub const DRILL_CAPACITY: &str = "Drill too long: keep it to 16 steps";
// restriction: combo-capacity
pub const COMBO_CAPACITY: &str = "Combo too long: keep it to 16 links";
fn save_input(root: &Path, source: &str, activate: bool, spans: &[Span], tick: &mut Tick) {
    let mode = if activate { 2 } else { 1 };
    let take = converted_input(mode).expect("observed take").capture; INPUT_MODE.fetch_and(!mode, Ordering::AcqRel);
    if take.combo.overflow { tick.notices.push(DRILL_CAPACITY); return; }
    if take.combo.is_empty() { tick.notices.push(super::rr_runtime::EMPTY_RECORDING); return; }
    let posted = if activate { take_record_start() } else { None };
    let start = posted.map(|p| p.start);
    let recorded_on = start.map(|s| super::rr_runtime::fighter_name(s.poses[0].fighter_kind)).filter(|s| !s.is_empty());
    let fighter = posted.and_then(|p| p.owner.fighter());
    // F10: a directly recorded drill owns the macro file holding its take.
    let kept = if activate { keep_drill_take(root, start, spans, fighter) } else { None };
    let saved = match kept {
        Some(kept) => super::rr_runtime::save_drill_with_take(root, &take.combo, recorded_on, fighter, kept, unix_now()),
        None => save_made_combo(root, &take.combo, source, recorded_on, fighter, unix_now()),
    };
    if let Ok(id) = saved {
        if activate {
            let _ = RecordingIndex::update(root, |index| {
                index.set_drill(Some(Drill { kind: DrillKind::Combo, id: id.clone() }), unix_now())?;
                Ok(true)
            });
            *lock(&SAVED_DRILL) = Some(id.clone());
        }
        tick.diags.push(format!("screen=input-drill saved={id} steps={}", take.combo.len()));
    }
}

/// The frames of a direct drill take as a macro (`save_drill_take`), with the start the Record chord posted. `None`
/// when nothing could be kept: the drill is saved all the same, without a preview.
fn keep_drill_take(root: &Path, start: Option<RecordStart>, spans: &[Span], fighter: Option<crate::model::move_names::FighterId>) -> Option<String> {
    let (_, frames) = taken_frames();
    let state = start.map_or_else(StartState::default, |s| StartState {
        situation: s.poses[0].situation_kind,
        speed_x: s.speeds[0].0,
        speed_y: s.speeds[0].1,
        ..StartState::neutral(s.poses[0].fighter_kind, 0, s.poses[0].lr)
    });
    super::rr_runtime::save_drill_take_of(root, frames, state, record_trim(), unix_now(), spans, fighter).ok().flatten()
}

static RECORD_EDGE: AtomicU8 = AtomicU8::new(0);
static PLAY_PLAYER: AtomicBool = AtomicBool::new(false);
static PLAY_CPU: AtomicBool = AtomicBool::new(false);
static RESET_CPU: AtomicBool = AtomicBool::new(false);
static MASH: AtomicBool = AtomicBool::new(false);
/// SAV-002: a save-state load stops both roles' runs (save-states.md: "Load only warps … it stops one that is playing").
static LOAD: AtomicBool = AtomicBool::new(false);
static MIGRATED: AtomicBool = AtomicBool::new(false);
/// Played frames, filled ahead by the worker and taken one per game frame by Player 1's poll and the
/// CPU's AI epilogue. One slot lost frames: the worker is not tied to the game's frames (Eden
/// `rc5-fid-measure1`, 2026-11-08: one CPU frame every three AI frames on a recorded play).
static PLAYER_FRAMES: FrameQueue = FrameQueue::new();
static CPU_FRAMES: FrameQueue = FrameQueue::new();
static CURRENT_KIND: AtomicU32 = AtomicU32::new(u32::MAX);
static RUN_PLAYER: AtomicBool = AtomicBool::new(false);
static RUN_CPU: AtomicBool = AtomicBool::new(false);
/// Parry practice is on (operator Q3, 2026-11-17: a CPU mash does not stop the parry replay).
static PARRY: AtomicBool = AtomicBool::new(false);

/// The settings publish: Parry practice On or Off.
pub fn set_parry(on: bool) {
    PARRY.store(on, Ordering::Release);
    // Parry Off ends the practice: no pass stays owed for a later On (tr-review-astra-2, 703e3f8d finding 1).
    PARRY_PASSES.fetch_min(if on { u32::MAX } else { 0 }, Ordering::AcqRel);
}

/// The CPU plays a library run now (the parry pass the detector arms on).
pub fn cpu_run_active() -> bool {
    RUN_CPU.load(Ordering::Acquire)
}

/// Parry passes still to play after the current one (`u32::MAX`: the Loop is infinite), and the detector has an attempt open.
static PARRY_PASSES: AtomicU32 = AtomicU32::new(0);
static PARRY_OPEN: AtomicBool = AtomicBool::new(false);

/// Play on CPU with Parry practice On and no CPU macro: how to record one (the existing recording flow).
// restriction: parry-practice-no-recording
pub const PARRY_NO_RECORDING: &str = "No CPU recording yet: open the Recording pop-up (ZR+Right) and choose CPU, then ZR+Down records and ZR+Down stops";

// restriction: parry-practice-start-save-missing
pub const PARRY_START_SAVE_MISSING: &str = "Can't start: its start save was deleted";
// restriction: parry-practice-macro-missing
pub const PARRY_MACRO_MISSING: &str = "Can't start: its macro was deleted";

/// The game thread: an attempt is open, so the next pass waits (`parry::busy`).
pub fn set_parry_open(open: bool) {
    PARRY_OPEN.store(open, Ordering::Release);
}

/// Parry practice (DESIGN §2.3): Play on CPU plays the active macro once from its start save (Make combo's play-once
/// `play_file`), and the Loop's count becomes passes, each warped again. A second Play stops it, as Play does.
fn parry_play<'a>(machine: &mut Playback, role: Role, book: &'a Book, next: bool, tick: &mut Tick) -> Option<super::playback::Effect<'a>> {
    if role != Role::Cpu || !PARRY.load(Ordering::Acquire) {
        return None;
    }
    if !next {
        let between = PARRY_PASSES.swap(0, Ordering::AcqRel) > 0;
        if machine.playing(role) {
            return None;
        }
        if between {
            return Some(super::playback::Effect::default());
        }
    }
    let id = book.index.active(role)?;
    // A saved practice follows its macro's start-save link (operator Q5); a deleted link or macro cannot start.
    if !book.has_plan(id) {
        PARRY_PASSES.store(0, Ordering::Release);
        tick.notices.push(PARRY_MACRO_MISSING);
        return Some(super::playback::Effect::default());
    }
    if matches!(book.start_link(id), StartLink::Deleted) {
        PARRY_PASSES.store(0, Ordering::Release);
        tick.notices.push(PARRY_START_SAVE_MISSING);
        return Some(super::playback::Effect::default());
    }
    if !next {
        let passes = match book.index.loop_count(role) {
            LoopCount::Infinite => u32::MAX,
            LoopCount::Finite(n) => u32::from(n).saturating_sub(1),
        };
        PARRY_PASSES.store(passes, Ordering::Release);
    }
    Some(machine.play_file(role, book, id))
}

/// The next parry pass: the last one ended, none is open, and passes are left.
fn parry_next_pass(start: impl FnOnce()) {
    let left = PARRY_PASSES.load(Ordering::Acquire);
    if !PARRY.load(Ordering::Acquire) || left == 0 || RUN_CPU.load(Ordering::Acquire) || PARRY_OPEN.load(Ordering::Acquire) {
        return;
    }
    if !role_frames(Role::Cpu).is_empty() {
        return;
    }
    start();
    if left != u32::MAX {
        // Keep the credit visible to Mash until the worker has published the new run.
        // Off can clear credits during preparation; spending one must not restore them.
        PARRY_PASSES.fetch_min(left - 1, Ordering::Release);
    }
}
/// A hook cut invalidates queued frames. Only the worker clears the queue.
static CUT_PLAYER: AtomicBool = AtomicBool::new(false);
static CUT_CPU: AtomicBool = AtomicBool::new(false);
static MACHINE: Mutex<Option<Playback>> = Mutex::new(None);
static BOOK: Mutex<Option<Box<HeldBook>>> = Mutex::new(None);
type MenuEffect = (Option<(u64, u64)>, Effect);
static MENU_EFFECT: Mutex<Option<MenuEffect>> = Mutex::new(None);
static PREVIEW: AtomicU8 = AtomicU8::new(0);
static ACTION_WATCH: hos_lock::Mutex<Watch> = hos_lock::Mutex::new(Watch::new());
static PREVIEW_ASK: Mutex<PreviewRequests> = Mutex::new(PreviewRequests { stop: false, start: None });
static DEFER_MIGRATE: AtomicBool = AtomicBool::new(false);
static ROLE_KIND: [AtomicI32; 2] = [const { AtomicI32::new(-1) }; 2];
static STRIP: Mutex<(u32, Option<Combo>, CaptureOwner)> = Mutex::new((0, None, CaptureOwner::UNKNOWN));
static MAKE: Mutex<Option<MakeJob>> = Mutex::new(None);
static HELD: Mutex<Option<Role>> = Mutex::new(None);
static ANSWER: Mutex<Option<MismatchChoice>> = Mutex::new(None);
static LAST_DRILL: Mutex<Option<String>> = Mutex::new(None);
static DRILL_SEEN: AtomicBool = AtomicBool::new(false);
static DRILL_ASK: Mutex<Option<String>> = Mutex::new(None);
static PLAYER_LABEL: Mutex<String> = Mutex::new(String::new());
static LAST_ASK: Mutex<Option<MismatchAsk>> = Mutex::new(None);
static CPU_ROLE_ON: AtomicBool = AtomicBool::new(false);
/// A Play warped to its start save and the restore has not landed yet. No role steps until it
/// does: a frame fed before the restore is undone by it. A CPU-role file begins on its first
/// press, so its opening action was always lost (Eden `rc5-opus-cpu2`, 2026-11-08).
static WARP_HOLD: AtomicBool = AtomicBool::new(false);
static START_GATE: AtomicBool = AtomicBool::new(false);
#[derive(Clone, Copy)]
struct PostedStart {
    seq: u64,
    start: RecordStart,
    owner: CaptureOwner,
    /// The CPU's fighter, for a CPU-role take.
    cpu: CaptureOwner,
}

/// One game-thread writer (chord or placement poll), one recording-worker reader.
static RECORD_STARTS: hos_lock::TripleBuffer<PostedStart> =
    hos_lock::TripleBuffer::new(PostedStart { seq: 0, start: RecordStart::EMPTY, owner: CaptureOwner::UNKNOWN, cpu: CaptureOwner::UNKNOWN });
static START_SEQ: AtomicU64 = AtomicU64::new(0);
static START_USED: AtomicU64 = AtomicU64::new(0);
/// Per role (Player, CPU): the start speed a Play's warp owes the run's first input (`take_start_speed`).
static START_SPEED: hos_lock::Mutex<[Option<StartSpeed>; 2]> = hos_lock::Mutex::new([None; 2]);
/// The roles with a `START_SPEED` entry, so the per-frame check takes no lock while nothing is owed.
static START_OWED: AtomicU8 = AtomicU8::new(0);
/// First locked from the AI epilogue (`cpu_role_after_chord`), before any worker touches it.
/// Switch `std::sync::Mutex` allocates a OnceBox on that first lock, and the hook counter charges
/// the one allocation to `recorder_ai` (`runtime::roster::build_split`, `docs/provenance/pkg6-owner-locks.md`).
/// `hos_lock` is a const word and allocates nothing.
static CPU_DRIVE: hos_lock::Mutex<CpuRoleDrive> = hos_lock::Mutex::new(CpuRoleDrive::idle());
/// The binding or a library file changed since the save-state machine last took the bound save.
static BOUND_DIRTY: AtomicBool = AtomicBool::new(true);
/// What the machine's active slot was last given: the bound id and its pair.
static BOUND_APPLIED: Mutex<Option<BoundCopy>> = Mutex::new(None);
/// A Save the state writer captured, for the worker to write into the bound save.
static SAVED_PAIR: Mutex<Option<[SavedState; 2]>> = Mutex::new(None);
/// A setup restore moved the fighters through the active slot: the slot is given the bound save again.
static SLOT_RESYNC: AtomicBool = AtomicBool::new(false);

/// The bound save as the save-state machine's active slot must hold it (RECORDING.md, "Save states only warp").
#[derive(Clone, Debug, PartialEq)]
struct BoundCopy {
    id: Option<String>,
    pair: Option<[SavedState; 2]>,
}

struct MakeJob {
    source: String,
    ready: bool,
    role: Role,
    drill: Option<String>,
    /// The strip's count of links captured when the source began playing.
    since: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MakeAsk { pub source: String, pub role: Role, pub drill: Option<String> }

static MAKE_ASK: Mutex<Option<MakeAsk>> = Mutex::new(None);
pub fn request_make(ask: MakeAsk) { *lock(&MAKE_ASK) = Some(ask); }
pub fn make_actor_cpu() -> bool { MAKE_ROLE.load(Ordering::Acquire) == ROLE_CPU }
pub fn make_cpu_inputs() -> MappedInputs { to_mapped(unpack(MAKE_CPU_FRAME.load(Ordering::Acquire))) }

/// What the worker keeps between passes: the index and each playing file with its start save. Read again at every
/// start and after an in-mod write; ordinary playback passes do not read the held files.
struct HeldBook {
    root: PathBuf,
    version: (u64, u64),
    book: Book,
    saves: Vec<(String, NamedSave)>,
    sources: [Option<Source>; 2],
}

/// What a role's run plays: the file it started on, and whether the run follows the role's active file (a chord's run
/// does; a preview or Make combo plays the one file it named).
#[derive(Clone)]
struct Source { id: String, active: bool }

enum PreviewWindow {
    Exact(core::ops::Range<usize>),
    Drill { frames: u32, trim: Trim },
}

struct PreviewStart { source: String, role: Role, window: PreviewWindow, playback: bool }
#[derive(Default)]
struct PreviewRequests {
    stop: bool,
    start: Option<PreviewStart>,
}
const PREVIEW_REQUESTED: u8 = 3; // Owns the preview, but neither fighter receives this request's frames yet.

pub const CLIP_ID: &str = "*clip";
static PREVIEW_CLIP: Mutex<Option<MacroRecording>> = Mutex::new(None);
static PREVIEW_AWAY: AtomicU8 = AtomicU8::new(0);
/// Why the game could not start a clip preview (`preview_refused`). One game-thread writer (the frame that took the
/// ask), one recording-worker reader; neither waits.
static PREVIEW_REFUSALS: hos_lock::TripleBuffer<(u32, &'static str)> = hos_lock::TripleBuffer::new((0, ""));
static REFUSAL_SEQ: AtomicU32 = AtomicU32::new(0);
static REFUSAL_SEEN: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeWarp {
    Start { air: bool, player: bool },
    Return,
}

/// What one worker tick asks the Switch wrapper to do.
#[derive(Clone, Debug, Default)]
pub struct Tick {
    pub warp: Option<String>,
    pub warp_pair: Option<Box<[SavedState; 2]>>,
    /// A Vault clip's preview: the game keeps both fighters as Save reads them when it first starts and loads that pair at the start and for the return.
    pub home: Option<HomeWarp>,
    /// The playing role and the placement of the file that warps (`warp_pair`).
    pub warp_place: Option<(Role, Placement)>,
    pub notices: Vec<&'static str>,
    pub diags: Vec<String>,
    pub keyboard: Option<KeyboardOffer>,
    pub mismatch: Option<MismatchAsk>,
    /// Switch uses the existing exact-fighter route for this kind. Keep leaves it empty.
    pub switch_kind: Option<i32>,
    /// Switch for a CPU macro recorded on an exact Mii: the macro's `playback_target()`, for the existing replay-catalog
    /// request (primary 2026-11-08 #54 (a)). The wrapper uses it instead of `switch_kind`.
    pub switch_mii: Option<crate::engine::mii::replay::Target>,
    /// A combo records a fighter name. The wrapper resolves it to a kind.
    pub switch_name: Option<String>,
    pub switch_player: bool,
    /// The full save pair is needed only on a picker open, not on every recording tick's stack.
    pub place: Option<Box<PlaceOpen>>,
}

/// The existing Fighter Mismatch screen, named for the wrapper that opens it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MismatchAsk {
    pub origin: &'static str,
    /// The side that plays; the screen names its current fighter with this role.
    pub role: ViewRole,
    pub recorded_kind: i32,
    pub current_kind: i32,
    pub recorded_label: String,
    pub current_label: String,
}

/// The answer the prompt returns. Cancel plays nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MismatchChoice {
    Keep,
    Switch,
    Cancel,
}

pub fn migrated() -> bool {
    MIGRATED.load(Ordering::Acquire)
}

/// The clip editor's preview of a saved macro: frames `start..end` of `source` play once on `role` through the role's
/// Play queue, warped to the macro's start save. Running from this call until the run ends or `stop_preview`.
pub fn request_preview(source: &str, role: Role, start: u32, end: u32) {
    queue_preview(PreviewStart { source: source.to_owned(), role, window: PreviewWindow::Exact(start as usize..end as usize), playback: false });
}

fn queue_preview(start: PreviewStart) {
    lock(&PREVIEW_ASK).start = Some(start);
    let _ = PREVIEW.compare_exchange(0, PREVIEW_REQUESTED, Ordering::AcqRel, Ordering::Acquire);
}

/// F10: a drill's take previews on Player 1 (`request_preview`), unless Player 1 has a Play queued or running:
/// that run is not replaced. True when the preview was asked for.
pub fn request_drill_preview(source: &str, frames: u32, trim: Trim) -> bool {
    if player_playback_blocks_preview() {
        return false;
    }
    queue_preview(PreviewStart { source: source.to_owned(), role: Role::Player, window: PreviewWindow::Drill { frames, trim }, playback: false });
    true
}

pub fn request_clip_preview(recording: MacroRecording, role: Role) {
    let len = recording.frames.len();
    *lock(&PREVIEW_CLIP) = Some(recording);
    queue_preview(PreviewStart { source: CLIP_ID.to_owned(), role, window: PreviewWindow::Exact(0..len), playback: false });
}

pub fn request_clip_playback(recording: MacroRecording, role: Role) {
    let len = recording.frames.len();
    *lock(&PREVIEW_CLIP) = Some(recording);
    queue_preview(PreviewStart { source: CLIP_ID.to_owned(), role, window: PreviewWindow::Exact(0..len), playback: true });
}

/// A Vault clip's preview has the fighters away from where it found them until the return lands.
pub fn preview_away() -> bool { PREVIEW_AWAY.load(Ordering::Acquire) != 0 }

/// The session ended (leaving Training, a hot swap, a lobby change): a Vault clip's fighters are not put back. Atomic write (a game thread calls it).
pub fn drop_preview_home() { PREVIEW_AWAY.store(0, Ordering::Release); }

/// The game could not keep or load the home pair for a starting clip preview: the preview does not play. Game thread.
pub fn preview_refused(text: &'static str) {
    let seq = REFUSAL_SEQ.fetch_add(1, Ordering::AcqRel) + 1;
    PREVIEW_REFUSALS.write((seq, text));
    abort_warp();
}

/// A preview owns the stage: its run, or the return of a Vault clip's fighters. Atomic reads (a hook may call it).
pub fn previewing() -> bool { preview_running() || preview_away() }

/// Stop a running preview (nothing otherwise): the rest of its frames is dropped.
pub fn stop_preview() {
    if previewing() {
        let mut asks = lock(&PREVIEW_ASK);
        asks.start = None;
        asks.stop = true;
    }
}

/// A preview was asked for and has not ended. Atomic read (a hook may call it).
pub fn preview_running() -> bool { PREVIEW.load(Ordering::Acquire) != 0 }

pub fn preview_role() -> Option<Role> {
    match PREVIEW.load(Ordering::Acquire) {
        ROLE_PLAYER => Some(Role::Player),
        ROLE_CPU => Some(Role::Cpu),
        _ => None,
    }
}

pub fn watch_feed(run: u32, index: u32, seen: Seen) {
    if let Some(mut watch) = ACTION_WATCH.try_lock() {
        watch.feed(run, index, seen);
    }
}

pub fn watch_copy(out: &mut [Span; WATCH_SPANS]) -> (u32, usize) {
    let watch = ACTION_WATCH.lock();
    let spans = watch.spans();
    out[..spans.len()].copy_from_slice(spans);
    (watch.run(), spans.len())
}

pub fn watch_clear() {
    *ACTION_WATCH.lock() = Watch::new();
}

/// The spans the watch logged for the take that just closed, taken once. A preview's log is not a take's.
fn take_capture_spans() -> ([Span; WATCH_SPANS], usize) {
    let mut spans = [Span::default(); WATCH_SPANS];
    let (run, count) = watch_copy(&mut spans);
    watch_clear();
    (spans, if run == CAPTURE_RUN { count } else { 0 })
}

/// FS-027 / FS-031: a Recording-page take is open, or its Record edge waits for the next service. The busy guards
/// (`recorder::recording_or_playback`) read this, because the old recorder machine never sees rr_host's takes. Playback is
/// left out on purpose: a fighter-mismatch Switch is admitted while its play has started, and must not refuse itself.
pub fn taking() -> bool {
    CAP_ROLE.load(Ordering::Acquire) != 0 || RECORD_EDGE.load(Ordering::Acquire) != 0
}


pub fn pending() -> bool {
    RECORD_EDGE.load(Ordering::Acquire) != 0
        || PLAY_PLAYER.load(Ordering::Acquire)
        || PLAY_CPU.load(Ordering::Acquire)
        || RESET_CPU.load(Ordering::Acquire)
        || MASH.load(Ordering::Acquire)
        || LOAD.load(Ordering::Acquire)
        || menu_pending()
        || playing()
        || waiting_work()
}

/// Work that waits on no chord: a drill question, a running Make combo, or an answer to the fighter question. The service
/// gate (`pending`) and the tick (`service_at`) read the same list; an answered question that `pending` left out was never
/// applied once migration had finished, so the held play never resumed (rc.5 Fighter Mismatch).
fn waiting_work() -> bool {
    previewing() || REFUSAL_SEQ.load(Ordering::Acquire) != REFUSAL_SEEN.load(Ordering::Acquire)
        || lock(&DRILL_ASK).is_some() || lock(&MAKE).is_some() || lock(&MAKE_ASK).is_some() || lock(&ANSWER).is_some()
}

fn playing() -> bool {
    role_playing(Role::Player) || role_playing(Role::Cpu)
}

/// The machine still steps the role, or its hook still has live frames to play.
fn role_playing(role: Role) -> bool {
    if role == Role::Cpu && RESET_CPU.load(Ordering::Acquire) {
        return false;
    }
    run_flag(role).load(Ordering::Acquire) || !cut_flag(role).load(Ordering::Acquire) && !role_frames(role).is_empty()
}

/// A queued Play, running Recording-page CPU macro, or its still-live buffered tail.
pub fn cpu_playback_blocks_drill() -> bool {
    !RESET_CPU.load(Ordering::Acquire) && (PLAY_CPU.load(Ordering::Acquire) || role_playing(Role::Cpu))
}

/// A queued Play, a running Recording-page run on Player 1, or its still-live buffered tail.
pub fn player_playback_blocks_preview() -> bool {
    PLAY_PLAYER.load(Ordering::Acquire) || role_playing(Role::Player)
}

fn run_flag(role: Role) -> &'static AtomicBool {
    match role {
        Role::Player => &RUN_PLAYER,
        Role::Cpu => &RUN_CPU,
    }
}

fn cut_flag(role: Role) -> &'static AtomicBool {
    match role {
        Role::Player => &CUT_PLAYER,
        Role::Cpu => &CUT_CPU,
    }
}

/// A hook's cut: atomics only (no lock); the worker's next tick clears the queue (`clear_cut`).
fn cut(role: Role) {
    cut_flag(role).store(true, Ordering::Release);
    run_flag(role).store(false, Ordering::Release);
}

/// Worker side: the cut run's queue and owed start speed go, then its frames count again.
fn clear_cut(role: Role) {
    drop_frames(role);
    cut_flag(role).store(false, Ordering::Release);
}

pub fn set_record_role(role: Role) {
    let bits = match role {
        Role::Player => ROLE_PLAYER,
        Role::Cpu => ROLE_CPU,
    };
    RECORD_ROLE.store(bits, Ordering::Release);
}

pub fn record_role() -> Role {
    if RECORD_ROLE.load(Ordering::Acquire) == ROLE_CPU { Role::Cpu } else { Role::Player }
}

/// The pop-up's role, posted by the menu worker before the record chord.
pub fn publish_fighter_prompt(on: bool) {
    ASK_FIGHTER.store(on, Ordering::Release);
}

pub fn publish_menu_role(role: ViewRole) {
    set_record_role(match role {
        ViewRole::Cpu => Role::Cpu,
        ViewRole::Player => Role::Player,
    });
}

/// Boot: the Record chord records on the pop-up role saved on the card before any menu or pop-up press.
pub fn adopt_saved_role(settings: &crate::model::settings::Settings) {
    publish_menu_role(settings.recording_roles()[0]);
}

/// After the record chord updates `CAP_ROLE`. A CPU-role start enters lockout. A stop leaves it.
pub fn cpu_role_after_chord() {
    let mut drive = CPU_DRIVE.lock();
    let role = CAP_ROLE.load(Ordering::Acquire);
    if drive.active() && role == 0 {
        drive.stop();
        CPU_ROLE_ON.store(false, Ordering::Release);
    } else if role == ROLE_CPU && !drive.active() {
        drive.begin();
        CPU_ROLE_ON.store(true, Ordering::Release);
    }
}

pub fn cpu_role_begin_placed() {
    let mut drive = CPU_DRIVE.lock();
    if CAP_ROLE.load(Ordering::Acquire) == ROLE_CPU && !drive.active() {
        drive.begin_placed();
        CPU_ROLE_ON.store(true, Ordering::Release);
    }
}

/// P1's poll while a CPU-role recording owns the controller. `false` leaves the player path alone.
pub fn cpu_role_poll(out: &mut MappedInputs) -> bool {
    if !CPU_ROLE_ON.load(Ordering::Acquire) {
        return false;
    }
    let stored = {
        let mut drive = CPU_DRIVE.lock();
        if !drive.active() {
            return false;
        }
        drive.poll(out)
    };
    if let Some(frame) = stored {
        push_capture(ROLE_CPU, to_frame(frame));
    }
    true
}

/// A controller record as a recorded frame: the same five fields, field for field.
pub fn to_frame(m: MappedInputs) -> InputFrame {
    InputFrame { buttons: m.buttons, lstick_x: m.lstick_x, lstick_y: m.lstick_y, rstick_x: m.rstick_x, rstick_y: m.rstick_y }
}

/// A recorded frame as a controller record (the inverse of [`to_frame`]).
pub fn to_mapped(f: InputFrame) -> MappedInputs {
    MappedInputs { buttons: f.buttons, lstick_x: f.lstick_x, lstick_y: f.lstick_y, rstick_x: f.rstick_x, rstick_y: f.rstick_y }
}

/// The CPU control frame of a CPU-role recording. `write` is what replaces the AI's controls.
pub fn cpu_role_control() -> ControlOut {
    if !CPU_ROLE_ON.load(Ordering::Acquire) {
        return ControlOut { write: None, hold_cliff_timer: false };
    }
    CPU_DRIVE.lock().control()
}

pub fn cpu_role_phase() -> Option<Phase> {
    if !CPU_ROLE_ON.load(Ordering::Acquire) {
        return None;
    }
    let drive = CPU_DRIVE.lock();
    drive.active().then(|| drive.phase())
}

/// Play on CPU: the posted frame as the CPU controller word (`inject_values`, mirror +1).
pub fn playback_controller(frame: InputFrame) -> Inject {
    inject_values(to_mapped(frame), 1)
}

/// Resolve the Recording page's contribution to the CPU controller.
#[cfg(any(test, target_os = "switch"))]
pub(crate) fn cpu_controller(take_frame: impl FnOnce() -> Option<InputFrame>) -> Option<Inject> {
    if RESET_CPU.load(Ordering::Acquire) {
        return None;
    }
    // Sample ownership before the pop: the worker can enqueue the tail and end the run concurrently.
    let running = role_playing(Role::Cpu);
    take_frame().map(|frame| {
        slot_stop();
        playback_controller(frame)
    }).or_else(|| {
        (running && !CUT_CPU.load(Ordering::Acquire)).then_some(Inject::NEUTRAL)
    })
}

/// The menu worker posts one effect. The recording worker applies it. A `None` does not clear a pending effect.
#[cfg(test)]
pub fn post_effect(effect: Effect) {
    post_effect_at(effect, None);
}

pub fn post_effect_at(effect: Effect, expected: Option<(u64, u64)>) {
    if expected.is_some_and(|version| version != recording_version()) {
        return;
    }
    if effect == Effect::RecordDrill { arm_drill_record(true); return; }
    if !matches!(effect, Effect::None) {
        *lock(&MENU_EFFECT) = Some((expected, effect));
    }
}

/// The three chords, consumed by the AI hook. This only stores flags.
pub fn on_chords(record: bool, play_player: bool, play_cpu: bool) {
    // The game thread posts flags instead of reading the card or waiting on the worker.
    if record {
        let direct = DRILL_ARMED.swap(false, Ordering::AcqRel);
        let role = if direct { ROLE_PLAYER } else { RECORD_ROLE.load(Ordering::Acquire) };
        if CAP_ROLE.load(Ordering::Acquire) == 0 {
            if direct { begin_input(2); }
            CAP_LEN.store(0, Ordering::Relaxed);
            LAST_ROLE.store(role, Ordering::Relaxed);
            CAP_ROLE.store(role, Ordering::Release);
            RECORD_EDGE.store(1, Ordering::Release);
        } else {
            CAP_ROLE.store(0, Ordering::Release);
            RECORD_EDGE.store(2, Ordering::Release);
        }
    }
    if play_player {
        PLAY_PLAYER.store(true, Ordering::Release);
    }
    if play_cpu {
        PLAY_CPU.store(true, Ordering::Release);
    }
}

/// REC-027: the CPU's run stops at once, so the CPU's next frame sees no playback and its mash acts inside the trigger;
/// the tick still drives the machine's mash (`MASH`).
pub fn note_mash() {
    MASH.store(true, Ordering::Release);
    cut(Role::Cpu);
}

#[cfg(any(test, target_os = "switch"))]
pub fn clear_playback(cause: crate::model::input_delay::Clear) {
    // SaveStateLoad also runs this hub during a macro's own start warp.
    if cause == crate::model::input_delay::Clear::TrainingReset {
        PLAY_CPU.store(false, Ordering::Release);
        RESET_CPU.store(true, Ordering::Release);
    }
}

/// A save-state load was accepted. A Play's own warp (`WARP_HOLD`) is the start of that playback, not a stop.
/// Both runs stop at once, as for a mash (`note_mash`); the tick still drives the machine (`LOAD`).
pub fn note_load() {
    if WARP_HOLD.load(Ordering::Acquire) {
        return;
    }
    LOAD.store(true, Ordering::Release);
    cut(Role::Player);
    cut(Role::Cpu);
}

/// What the CPU's behaviours see of playback (B5 / N4, Mash interrupts playback): the old recorder's state, with a
/// library macro playing on the CPU counted as a running playback. A parry practice plays pass by pass: while passes
/// are owed its playback runs between them too, so a configured Mash asks it to stop (Parry ignores that) instead of
/// queueing an action on top of the replay (Eden accept-8, 26c93533). Game hooks read this using atomics.
pub fn cpu_playback(queued: bool) -> PlaybackState {
    let parry_owed = PARRY.load(Ordering::Acquire) && PARRY_PASSES.load(Ordering::Acquire) > 0
        && !RESET_CPU.load(Ordering::Acquire);
    let owed = slot_owed();
    PlaybackState { running: SLOT_RUNNING.load(Ordering::Acquire) || owed || role_playing(Role::Cpu) || parry_owed,
        owed, recording: false, standby: false, queued }
}

pub fn playback_roles() -> [bool; 2] {
    [role_playing(Role::Player), role_playing(Role::Cpu) || SLOT_RUNNING.load(Ordering::Acquire)]
}

/// The save-state restore a Play's warp asked for has placed both fighters (or the warp was
/// not requested after all). The held runs step from their first frame on the next tick.
pub fn warp_landed() {
    WARP_HOLD.store(false, Ordering::Release);
}

pub fn start_gate(up: bool) {
    if up {
        PLAYED.store(0, Ordering::Release);
    }
    START_GATE.store(up, Ordering::Release);
}

pub fn captured_frames() -> u32 {
    CAP_LEN.load(Ordering::Acquire) as u32
}

pub fn played_frames() -> u32 {
    PLAYED.load(Ordering::Acquire)
}

/// Frames of the role's current (or last) library run that its hook has taken: zero when a run is loaded, then one per delivered frame. Atomic read; slot runs are not counted.
pub fn fed_frames(role: Role) -> u32 {
    let index = role_index(role);
    let generation = RUN_GEN[index].load(Ordering::Acquire);
    let fed = FED[index].load(Ordering::Acquire);
    if (fed >> 32) as u32 == generation { fed as u32 } else { 0 }
}

fn mark_run(role: Role) {
    let runs = &RUNS[role_index(role)];
    runs.fetch_add(1, Ordering::AcqRel);
    RUN_GEN[role_index(role)].store(role_frames(role).generation() as u32, Ordering::Release);
    runs.fetch_add(1, Ordering::AcqRel);
}

/// The role's library run number (loads since boot) and the frames of that run its hook has taken, read together; `None` while a load stores.
pub fn fed_run(role: Role) -> Option<(u32, u32)> {
    let runs = &RUNS[role_index(role)];
    (0..4).find_map(|_| {
        let before = runs.load(Ordering::Acquire);
        let frames = fed_frames(role);
        (before & 1 == 0 && runs.load(Ordering::Acquire) == before).then_some((before >> 1, frames))
    })
}

pub fn start_gated() -> bool {
    START_GATE.load(Ordering::Acquire)
}

pub fn start_ready() -> bool {
    !(RUN_PLAYER.load(Ordering::Acquire) && PLAYER_FRAMES.is_empty())
        && !(RUN_CPU.load(Ordering::Acquire) && CPU_FRAMES.is_empty())
}

pub fn abort_warp() {
    LOAD.store(true, Ordering::Release);
    cut(Role::Player);
    cut(Role::Cpu);
    WARP_HOLD.store(false, Ordering::Release);
    start_gate(false);
}

/// One fighter at a record chord: the live values the start save keeps (`StartPose::saved`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StartPose {
    pub fighter_kind: i32,
    pub situation_kind: i32,
    pub x: f32,
    pub y: f32,
    pub percent: f32,
    pub lr: f32,
    pub start_state: PlacementStartState,
}

/// A role never seen: kind −1.
const UNSEEN: StartPose = StartPose { fighter_kind: -1, situation_kind: 0, x: 0.0, y: 0.0, percent: 0.0, lr: 1.0, start_state: PlacementStartState::Standard };

impl StartPose {
    /// The saved record of this pose; a fighter never seen (kind −1) is a never-saved record.
    pub fn saved(self) -> SavedState {
        let mut state = SavedState::never_saved();
        if self.fighter_kind < 0 {
            return state;
        }
        state.fighter_kind = self.fighter_kind;
        state.situation_kind = self.situation_kind;
        state.x = self.x;
        state.y = self.y;
        state.percent = self.percent;
        state.lr = self.lr;
        state.start_state = self.start_state;
        state
    }
}

/// Both fighters when Record started, with each one's summed speed (world units per frame, x then y). The start save is
/// this pair ("Start save: the save state this macro starts at"); the macro keeps its recorded role's speed (PLAY-MOMENTUM).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecordStart {
    pub poses: [StartPose; 2],
    pub speeds: [(f32, f32); 2],
}

impl RecordStart {
    const EMPTY: Self = Self { poses: [UNSEEN; 2], speeds: [(0.0, 0.0); 2] };

    /// Both role bodies as the game held them at the Record chord (slot 0 Player 1, 1 the CPU): each one's pose and
    /// summed speed, read from the body then. Never the frame-published pose (`live_pose`): that is the role's last
    /// published frame, which may be another fighter's. A role without a body is a never-seen pose (kind −1) at rest.
    pub fn at_chord(bodies: [Option<(StartPose, (f32, f32))>; 2]) -> Self {
        let bodies = bodies.map(|body| body.unwrap_or((UNSEEN, (0.0, 0.0))));
        Self { poses: bodies.map(|body| body.0), speeds: bodies.map(|body| body.1) }
    }
}

/// Each role's fighter as its latest Training frame ran it (`fighter_frame`; kind −1 before one ran): the worker's
/// live pose. Atomics only, so the worker never calls a native.
static LIVE_KIND: [AtomicI32; 2] = [const { AtomicI32::new(-1) }; 2];
static LIVE_SIT: [AtomicI32; 2] = [const { AtomicI32::new(0) }; 2];
/// x, y, percent and facing, as `f32` bits.
static LIVE_F32: [[AtomicU32; 4]; 2] = [const { [const { AtomicU32::new(0) }; 4] }; 2];

/// Whether the save-state step of a fighter's frame runs: Save States is On, a temporary load is under way, or this
/// body just took a posted named load (a Play's, a Make combo's or a crop preview's warp to a start save). The named
/// load is taken whatever Save States says: behind the Off return it was never taken and the run it held never started.
pub fn save_state_step(enabled: bool, temporary: bool, take_named: impl FnOnce() -> bool) -> bool {
    let named = take_named();
    enabled || temporary || named
}

/// One fighter's frame in Training, in the order the frame runs it (`states::on_fighter_frame`): the role's live pose
/// is published for the worker first, whatever Save States says, and only then does Save States Off end the frame. `live`
/// is the role's slot and pose, `None` for a body that is not the role's own (Nana). Returns whether the save-state step runs
/// (`save_states`: Save States is On, or a named load holds it open). Combined run rc15-ib-1f7e0174: the pose was
/// published after the Off return, so with Save States Off the worker's live pose named the previous CPU.
pub fn fighter_frame(live: Option<(usize, StartPose)>, save_states: bool) -> bool {
    if let Some((slot, pose)) = live {
        if let (Some(kind), Some(sit), Some(cells)) = (LIVE_KIND.get(slot), LIVE_SIT.get(slot), LIVE_F32.get(slot)) {
            kind.store(pose.fighter_kind, Ordering::Release);
            sit.store(pose.situation_kind, Ordering::Release);
            for (cell, value) in cells.iter().zip([pose.x, pose.y, pose.percent, pose.lr]) {
                cell.store(value.to_bits(), Ordering::Release);
            }
        }
    }
    save_states
}

/// A fighter's live pose (`fighter_frame`). Atomics only, no allocation; a slot never seen is kind −1.
pub fn live_pose(slot: usize) -> StartPose {
    let (Some(kind), Some(sit), Some(cells)) = (LIVE_KIND.get(slot), LIVE_SIT.get(slot), LIVE_F32.get(slot)) else { return UNSEEN };
    let kind = kind.load(Ordering::Acquire);
    if kind < 0 {
        return UNSEEN;
    }
    let [x, y, percent, lr] = [0, 1, 2, 3].map(|i: usize| f32::from_bits(cells[i].load(Ordering::Acquire)));
    StartPose { fighter_kind: kind, situation_kind: sit.load(Ordering::Acquire), x, y, percent, lr, start_state: PlacementStartState::Standard }
}

/// The speed a Play gives the playing role on its first input after the warp: the macro's recorded start speed with
/// "Keep recorded speed", zero with "Start at rest". `facing` is the facing it was recorded with; x is applied relative
/// to it, as the recorder's inputs are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StartSpeed {
    pub speed: (f32, f32),
    pub facing: f32,
}

/// The game thread, at a record chord that starts a recording (`capture_role() == 0`), before `on_chords`: both fighters'
/// live pose and speeds. Values only; no allocation.
pub fn note_record_start(start: RecordStart) {
    let fighter = crate::model::move_names::recorded_identity(crate::model::move_names::PUBLISHED.player(), start.poses[0].fighter_kind);
    post_record_start(start, fighter);
}

/// Reads a role's live fighter identity on the game thread. The runtime installs it once; set once, read without a lock.
static FIGHTER_READER: std::sync::OnceLock<fn(Role) -> Option<crate::model::move_names::FighterId>> = std::sync::OnceLock::new();
pub fn set_fighter_reader(reader: fn(Role) -> Option<crate::model::move_names::FighterId>) {
    let _ = FIGHTER_READER.set(reader);
}
fn read_fighter(role: Role) -> Option<crate::model::move_names::FighterId> {
    FIGHTER_READER.get().and_then(|reader| reader(role))
}

/// Every take start is posted here: the Record chord, and placement Ready through `note_record_start`. Each role's
/// fighter identity is read here, so every take carries the one of the role that recorded it.
pub fn post_record_start(start: RecordStart, fighter: Option<crate::model::move_names::FighterId>) {
    let seq = START_SEQ.fetch_add(1, Ordering::AcqRel) + 1;
    let fighter = fighter.or_else(|| read_fighter(Role::Player));
    RECORD_STARTS.write(PostedStart { seq, start, owner: CaptureOwner::new(start.poses[0].fighter_kind, fighter),
        cpu: CaptureOwner::new(start.poses[1].fighter_kind, read_fighter(Role::Cpu)) });
}

fn take_record_start() -> Option<PostedStart> {
    let posted = RECORD_STARTS.read().0;
    (START_USED.swap(posted.seq, Ordering::AcqRel) != posted.seq).then_some(posted)
}

/// PLAY-MOMENTUM (primary 2026-11-08 23:03Z, reading (b)): what the role's run owes its first input, taken once by the
/// game frame that injected that input. `None` when nothing is owed (no warp, or already given).
pub fn take_start_speed(role: Role) -> Option<StartSpeed> {
    let bit = 1u8 << role_index(role);
    // While the warp's restore is pending no input is fed, and the restore would undo a speed set now.
    if START_OWED.load(Ordering::Acquire) & bit == 0 || WARP_HOLD.load(Ordering::Acquire)
        || cut_flag(role).load(Ordering::Acquire) {
        return None;
    }
    START_OWED.fetch_and(!bit, Ordering::AcqRel);
    START_SPEED.lock()[role_index(role)].take()
}

fn owe_start_speed(role: Role, owed: Option<StartSpeed>) {
    let bit = 1u8 << role_index(role);
    START_SPEED.lock()[role_index(role)] = owed;
    if owed.is_some() {
        START_OWED.fetch_or(bit, Ordering::AcqRel);
    } else {
        START_OWED.fetch_and(!bit, Ordering::AcqRel);
    }
}

/// The start speed a Play of `id` owes after its warp: the recorded start speed of the file (a chain: macro 1, whose start
/// save it warps to) under the placement it warps with.
fn play_start_speed(book: &Book, id: &str, placement: Placement) -> Option<StartSpeed> {
    let first = book.first_recording(id).unwrap_or(id);
    let start = book.start_state(first)?;
    let speed = match placement.momentum {
        MomentumMode::PreserveRecorded => (start.speed_x, start.speed_y),
        MomentumMode::AtRest => (0.0, 0.0),
    };
    Some(StartSpeed { speed, facing: start.facing })
}

/// One captured frame. `side` is 1 for Player 1 and 2 for the CPU. A mismatch is ignored.
pub fn push_capture(side: u8, frame: InputFrame) {
    if CAP_ROLE.load(Ordering::Acquire) != side {
        return;
    }
    let len = CAP_LEN.load(Ordering::Relaxed);
    if len >= MAX_FRAMES {
        return;
    }
    CAP_FRAMES[len].store(pack(frame), Ordering::Relaxed);
    TAKE.push(len, usize::from(side == ROLE_CPU));
    CAP_LEN.store(len + 1, Ordering::Release);
}

#[cfg(any(test, target_os = "switch"))]
pub(crate) fn capture_role() -> u8 {
    CAP_ROLE.load(Ordering::Acquire)
}

/// The active take's role for the bridge, independent of the saved preference.
#[cfg(any(test, target_os = "switch"))]
pub(crate) fn capture_role_key() -> &'static str {
    match capture_role() {
        ROLE_PLAYER => ViewRole::Player.diag(),
        ROLE_CPU => ViewRole::Cpu.diag(),
        _ => "",
    }
}

/// The open take for the HUD's row (`hud_row`), independent of the saved preference.
#[cfg(any(test, target_os = "switch"))]
pub(crate) fn capture_take() -> Option<Take> {
    match capture_role() {
        ROLE_PLAYER => Some(Take::Player),
        ROLE_CPU => Some(Take::Cpu),
        _ => None,
    }
}

/// Player 1's fighter tint for this frame: the Player role's while Player 1's take is open, from the Record chord
/// that opens it to the one that closes it (Player 1's take has no lockout or standby). The CPU's take tints the CPU
/// by its phase instead (`cpu_role_phase`, `Phase::tint`).
#[cfg(any(test, target_os = "switch"))]
pub(crate) fn player_tint() -> Option<[f32; 3]> {
    (capture_take() == Some(Take::Player)).then_some(crate::model::recording::PLAYER_TINT)
}

pub fn take_player_frame() -> Option<InputFrame> {
    take_frame(Role::Player, || {})
}

#[cfg(test)]
pub(crate) fn take_cpu_frame() -> Option<InputFrame> {
    take_cpu_frame_seeded(|| {})
}

#[cfg(any(test, target_os = "switch"))]
pub(crate) fn take_cpu_frame_seeded(seed: impl FnOnce()) -> Option<InputFrame> {
    let frame = take_frame(Role::Cpu, seed);
    if let Some(frame) = frame.filter(|_| make_actor_cpu() && INPUT_MODE.load(Ordering::Acquire) & 1 != 0) {
        MAKE_CPU_FRAME.store(pack(frame), Ordering::Release);
        INPUT_POLLS[0].fetch_add(1, Ordering::Release);
    }
    frame
}

/// Player 1's poll. Injects one posted frame, then stores the output when Player 1 is the recorded role. Returns whether
/// a frame of a Player run was injected (its first one may owe a start speed, `take_start_speed`).
///
/// # Safety
/// `out` is null or a live mapped-input record for this poll.
pub unsafe fn on_player_poll(out: *mut MappedInputs) -> bool {
    on_player_poll_seeded(out, || {})
}

/// # Safety
/// `out` is null or the live mapped-input record for this poll.
pub unsafe fn on_player_poll_seeded(out: *mut MappedInputs, seed: impl FnOnce()) -> bool {
    if out.is_null() {
        return false;
    }
    let delay_before = DELAY_OWED[role_index(Role::Player)].load(Ordering::Acquire);
    // The fighter's poll takes the first input after the worker has published it.
    let injected = take_frame(Role::Player, seed);
    if let Some(frame) = injected {
        *out = to_mapped(frame);
    } else if DELAY_OWED[role_index(Role::Player)].load(Ordering::Acquire) < delay_before {
        *out = to_mapped(InputFrame::default());
    }
    if CAP_ROLE.load(Ordering::Acquire) == ROLE_PLAYER {
        push_capture(ROLE_PLAYER, to_frame(*out));
    }
    if injected.is_some() && MAKE_ROLE.load(Ordering::Acquire) == ROLE_PLAYER && INPUT_MODE.load(Ordering::Acquire) & 1 != 0 { INPUT_POLLS[0].fetch_add(1, Ordering::Release); }
    if CAP_ROLE.load(Ordering::Acquire) == ROLE_PLAYER && INPUT_MODE.load(Ordering::Acquire) & 2 != 0 { INPUT_POLLS[1].fetch_add(1, Ordering::Release); }
    injected.is_some()
}

pub fn today() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => civil_date(duration.as_secs()),
        Err(_) => "1970-01-01".to_string(),
    }
}

/// A macro stop and an input-drill stop are alternatives. The drill's deep
/// save path must not reserve the macro capture, spans and two saved fighters.
#[inline(never)]
fn save_stopped_macro(root: &Path, player: SavedState, cpu: SavedState, tick: &mut Tick) {
    let role_now = if RECORD_ROLE.load(Ordering::Acquire) == ROLE_CPU { "cpu" } else { "player" };
    tick.diags.push(format!("screen=recording role={role_now} recording=0"));
    let (spans, count) = take_capture_spans();
    let (role, frames) = taken_frames();
    let track = TAKE.take(frames.len());
    let mut capture = Capture::idle();
    toggle_record(&mut capture, role);
    capture.set_track(track);
    for frame in frames {
        push_frame(&mut capture, role, frame);
    }
    capture.set_spans(&spans[..count]);
    // The Record chord owns the start pose and speed. Host callers without
    // a posted start use the two live poses, at rest.
    let posted = take_record_start();
    capture.set_fighter(posted.and_then(|p| if role == Role::Cpu { p.cpu } else { p.owner }.fighter()));
    let (player, cpu, speeds) = match posted.map(|p| p.start) {
        Some(start) => (start.poses[0].saved(), start.poses[1].saved(), start.speeds),
        None => (player, cpu, [(0.0, 0.0); 2]),
    };
    if let Ok(stop) = finish_recording(root, &capture, player, cpu, speeds[role_index(role)], unix_now(), record_trim()) {
        if let Some(text) = stop.notice() {
            tick.notices.push(text);
        }
    }
}

pub fn service_at(
    root: &Path,
    slots: &[Option<LegacySlot>; 5],
    date: &str,
    player: SavedState,
    cpu: SavedState,
) -> Tick {
    let mut tick = Tick::default();
    TAKE.service(CAP_ROLE.load(Ordering::Acquire) != 0, CAP_LEN.load(Ordering::Acquire));
    apply_waiting_answer(&mut tick);
    if !DEFER_MIGRATE.load(Ordering::Acquire)
        && !MIGRATED.load(Ordering::Acquire)
        && migrate_at_boot(root, slots, date).is_ok()
    {
        MIGRATED.store(true, Ordering::Release);
        BOUND_DIRTY.store(true, Ordering::Release);
        tick.diags.push("screen=migrate once=1".to_string());
    }
    if RECORD_EDGE.load(Ordering::Acquire) == 2 && INPUT_MODE.load(Ordering::Acquire) & 2 != 0 && converted_input(2).is_none() { return tick; }
    let edge = RECORD_EDGE.swap(0, Ordering::AcqRel);
    let mut play_player = PLAY_PLAYER.swap(false, Ordering::AcqRel);
    let mut play_cpu = PLAY_CPU.swap(false, Ordering::AcqRel);
    if RESET_CPU.load(Ordering::Acquire) {
        play_cpu = false;
        // Keep the hook gated until the worker has retired the run and everything that could resume it.
        if let Some(machine) = lock(&MACHINE).as_mut() {
            drive_mash(machine, Role::Cpu, true);
        }
        RUN_CPU.store(false, Ordering::Release);
        PARRY_PASSES.store(0, Ordering::Release);
        clear_cut(Role::Cpu);
        if make_actor_cpu() {
            if let Some(job) = lock(&MAKE).take() {
                if job.drill.is_some() { let _ = Library::new(root.join(MACROS_DIR)).delete(&job.source); }
            }
            lock(&MAKE_ASK).take();
            MAKE_ROLE.store(0, Ordering::Release);
            INPUT_MODE.fetch_and(!1, Ordering::AcqRel);
        }
        let mut held = lock(&HELD);
        if *held == Some(Role::Cpu) {
            *held = None;
            lock(&ANSWER).take();
            lock(&LAST_ASK).take();
        }
        RESET_CPU.store(false, Ordering::Release);
    }
    let mash = MASH.swap(false, Ordering::AcqRel);
    let load = LOAD.swap(false, Ordering::AcqRel);
    if edge == 1 {
        let role = if LAST_ROLE.load(Ordering::Acquire) == ROLE_CPU {
            "cpu"
        } else {
            "player"
        };
        tick.diags
            .push(format!("screen=recording role={role} recording=1"));
        show_chord_notice(root, &mut tick);
    }
    if edge == 2 && INPUT_MODE.load(Ordering::Acquire) & 2 != 0 {
        let (spans, count) = take_capture_spans();
        save_input(root, "", true, &spans[..count], &mut tick); let _ = taken_frames(); let _ = take_record_start();
    } else if edge == 2 {
        save_stopped_macro(root, player, cpu, &mut tick);
    }
    // A drill question and a finished Make combo are work even when no chord is waiting.
    let busy = play_player
        || play_cpu
        || mash
        || load
        || menu_pending()
        || playing()
        || PARRY_PASSES.load(Ordering::Acquire) > 0
        || waiting_work();
    if !busy {
        return tick;
    }
    let mut held = lock(&BOOK);
    if !refresh_held(&mut held, root, [play_player, play_cpu]) {
        // Before the refusal is taken: taking one clears `PREVIEW`, which names the role to retire.
        retire_preview_run();
        take_preview_refusal(&mut tick);
        PREVIEW.store(0, Ordering::Release);
        *lock(&PREVIEW_ASK) = PreviewRequests::default();
        drop_preview_home();
        return tick;
    }
    let held = held.as_mut().expect("held book").as_mut();
    held.book.remove_recording(CLIP_ID);
    if let Some(clip) = lock(&PREVIEW_CLIP).as_ref() { held.book.add_recording(CLIP_ID, clip.clone()); }
    let book = &held.book;
    let mut machine = lock(&MACHINE);
    let machine = machine.get_or_insert_with(Playback::new);
    // restriction: parry-mash-ignored
    if mash && !PARRY.load(Ordering::Acquire) {
        // The mash is the CPU's (Mash Toggles), so it stops the macro the CPU plays; Player 1's keeps playing
        // (RECORDING.md: "stops the macro that role is playing"; primary 2026-11-08 13:05Z).
        drive_mash(machine, Role::Cpu, true);
        RUN_CPU.store(false, Ordering::Release);
        clear_cut(Role::Cpu);
        tick.diags.push("screen=playback mash=1".to_string());
    }
    if load {
        // The load starts nothing either: a Play press waiting in this tick goes too.
        (play_player, play_cpu) = (false, false);
        for role in [Role::Player, Role::Cpu] {
            drive_mash(machine, role, true);
            run_flag(role).store(false, Ordering::Release);
            clear_cut(role);
        }
        tick.diags.push("screen=playback load=1".to_string());
    }
    release_held(&mut play_player, &mut play_cpu);
    if play_player {
        play_role(root, machine, book, Role::Player, &mut tick);
    } else if RUN_PLAYER.load(Ordering::Acquire) {
        step_role(machine, book, Role::Player);
    }
    if play_cpu {
        play_role(root, machine, book, Role::Cpu, &mut tick);
    } else if RUN_CPU.load(Ordering::Acquire) {
        step_role(machine, book, Role::Cpu);
    }
    parry_next_pass(|| {
        play_role_with(root, machine, book, Role::Cpu, &mut tick, true);
        if machine.playing(Role::Cpu) {
            held.sources[role_index(Role::Cpu)] = book.index.active(Role::Cpu)
                .map(|id| Source { id: id.to_owned(), active: true });
        }
    });
    if menu_pending() {
        apply_menu(root, machine, book, &mut tick);
    }
    if let Some(ask) = lock(&MAKE_ASK).take() {
        begin_make(machine, book, ask, &mut tick);
    }
    drive_preview(machine, book, &mut tick);
    ask_drill(root, &mut tick);
    finish_make(root, &mut tick);
    hold_warp_pair(held, &mut tick);
    tick
}

/// A record or a menu effect while the old slot image is still filling. Migration stays pending.
pub fn service_pending(root: &Path, player: SavedState, cpu: SavedState) -> Tick {
    DEFER_MIGRATE.store(true, Ordering::Release);
    let empty = [None, None, None, None, None];
    let tick = service_at(root, &empty, "", player, cpu);
    DEFER_MIGRATE.store(false, Ordering::Release);
    tick
}

/// The worker's pass once migration has finished: what waits is serviced, and nothing else.
pub fn worker_tick(root: &Path, player: SavedState, cpu: SavedState) -> Option<Tick> {
    if !pending() {
        return None;
    }
    Some(service_pending(root, player, cpu))
}

pub fn set_role_fighter(role: Role, kind: i32) {
    ROLE_KIND[role_index(role)].store(kind, Ordering::Release);
    if role == Role::Player {
        CURRENT_KIND.store(kind as u32, Ordering::Release);
    }
}

/// Record-time trim last published by Recording › Options (factory End).
static RECORD_TRIM: AtomicU8 = AtomicU8::new(crate::model::trim::Trim::End as u8);
static PLAYBACK_INDICATOR: AtomicBool = AtomicBool::new(true);
static PLAYBACK_DELAY: AtomicU16 = AtomicU16::new(0);
static DELAY_OWED: [AtomicU16; 2] = [const { AtomicU16::new(0) }; 2];

pub fn set_record_trim(trim: crate::model::trim::Trim) {
    RECORD_TRIM.store(trim as u8, Ordering::Release);
}

fn record_trim() -> crate::model::trim::Trim {
    crate::model::trim::Trim::from_index(RECORD_TRIM.load(Ordering::Acquire))
}

pub fn set_playback_indicator(on: bool) {
    PLAYBACK_INDICATOR.store(on, Ordering::Release);
}

pub fn set_playback_delay(frames: u16) {
    PLAYBACK_DELAY.store(frames, Ordering::Release);
}

fn owe_delay(role: Role) {
    DELAY_OWED[role_index(role)].store(PLAYBACK_DELAY.load(Ordering::Acquire), Ordering::Release);
}

#[cfg(test)]
pub fn drop_cached_book() -> Option<usize> {
    let held = lock(&BOOK).take()?;
    let inline = std::mem::size_of_val(&held);
    drop(held);
    Some(inline)
}

pub fn playback_indicator_enabled() -> bool {
    PLAYBACK_INDICATOR.load(Ordering::Acquire)
}

/// The strip's last combo and its count of links captured (`combo::Helper::seen`), from the recording worker's tick.
pub fn set_strip_sample(sample: (u32, Option<Combo>, CaptureOwner)) {
    *lock(&STRIP) = sample;
}

pub fn set_fighter_label(label: &str) {
    *lock(&PLAYER_LABEL) = label.to_string();
}

pub fn answer_mismatch(choice: MismatchChoice) {
    *lock(&ANSWER) = Some(choice);
}

pub fn catalog_generation() -> u64 {
    store::catalog_generation()
}

pub fn recording_version() -> (u64, u64) {
    (catalog_generation(), store::index_generation())
}

pub fn play_sampling() -> bool { PLAY.sampling() }
pub fn play_allocated() -> bool { PLAY.allocated() }
pub fn play_live(slot: usize, frame: &ReplayFrame) { PLAY.live(slot, frame); }
pub fn play_seed(slot: usize, frame: Option<ReplayFrame>) { PLAY.seed(slot, frame); }
pub fn take_generated() -> bool { PLAY.take_written() }

#[inline(never)]
pub fn play_track_service(root: &Path) {
    use super::macro_ui::play_track::PlayTrack;
    use super::macros::positions::{self, Source};
    if PLAY_PLAYER.load(Ordering::Acquire) || PLAY_CPU.load(Ordering::Acquire) || lock(&PREVIEW_ASK).start.is_some() {
        return;
    }
    PLAY.ready();
    let Some((slot, generation)) = PLAY.open_run() else { return };
    let role = if slot == 0 { Role::Player } else { Role::Cpu };
    let over = !role_playing(role) || RUN_GEN[slot].load(Ordering::SeqCst) != generation;
    if !over || PLAY.busy() != 0 { return; }
    let Some((open, mut rows)) = PLAY.end() else { return };
    if open.version != recording_version() { return; }
    let dir = root.join(MACROS_DIR);
    let Ok(recording) = Library::new(&dir).load(open.id.as_str()) else { return };
    let same = |played: &[InputFrame]| positions::same_inputs(open.id.as_str(), played, &recording.frames);
    let matches = match open.retired_inputs.as_deref() {
        Some(played) => same(played),
        None => lock(&BOOK).as_ref().and_then(|held| held.book.inputs(open.id.as_str())).is_some_and(same),
    };
    if !matches { return; }
    if open.pass { rows.truncate(rows.len().min(recording.trim.window(&recording.frames).end)); }
    let existing = positions::check(&dir, open.id.as_str(), &recording.frames);
    let (write, done) = PlayTrack::write_wanted(existing, rows.len(), recording.frames.len());
    if write {
        if positions::write(&dir, open.id.as_str(), &recording.frames, &rows, Source::Generated).is_err() { return; }
        PLAY.note_written();
    }
    if done { PLAY.note_done(open.id.as_str(), open.version); }
}

#[cfg(any(test, target_os = "switch"))]
pub(crate) fn bump_catalog() {
    store::advance_catalog();
    invalidate_bound();
}

pub(crate) fn invalidate_bound() {
    BOUND_DIRTY.store(true, Ordering::Release);
}

/// What the save-state machine's active slot must hold now, when that changed since the last call or a setup restore
/// used the slot (`setup_restored`): the bound save's pair, or `Some(None)` (an empty slot) when nothing is bound. Load and Auto-load warp to that slot, so they follow
/// the binding (pop-up A on a save, the Bind row, Delete) and the bound file. Nothing before the library is built: the
/// legacy slot stays until migration binds the slot the chords used.
pub fn bound_sync(root: &Path) -> Option<Option<[SavedState; 2]>> {
    if !BOUND_DIRTY.swap(false, Ordering::AcqRel) {
        return None;
    }
    let index = match RecordingIndex::load(root) {
        Ok(index) if index.migrated() => index,
        _ => {
            BOUND_DIRTY.store(true, Ordering::Release);
            return None;
        }
    };
    let id = index.bound().map(str::to_owned);
    let pair = id
        .as_deref()
        .and_then(|id| Store::<NamedSave>::new(root.join(SAVES_DIR)).load(id).ok())
        .map(|save| [save.player, save.cpu]);
    let copy = BoundCopy { id, pair };
    let mut applied = lock(&BOUND_APPLIED);
    if !SLOT_RESYNC.swap(false, Ordering::AcqRel) && applied.as_ref() == Some(&copy) {
        return None;
    }
    let pair = copy.pair.clone();
    *applied = Some(copy);
    Some(pair)
}

/// The state writer has a Save's two records for the active slot. The worker writes them into the bound save.
/// No bound-save write waits for the worker (the bridge's fixture restore waits for this).
pub fn bound_settled() -> bool {
    pair_settled() && !BOUND_DIRTY.load(Ordering::Acquire)
}

/// No captured pair waits for the worker and none is being written.
pub fn pair_settled() -> bool {
    !BOUND_WRITING.load(Ordering::Acquire) && lock(&SAVED_PAIR).is_none()
}

/// The worker is writing a captured pair into the bound save (set before the pair is taken).
static BOUND_WRITING: AtomicBool = AtomicBool::new(false);

pub fn save_captured(pair: [SavedState; 2]) {
    *lock(&SAVED_PAIR) = Some(pair);
}

/// The state writer applied a setup restore (or a setup bound with Y) to the active slot. Setups stay a separate
/// library from save states (LANE-GROK-RECORDING-REDESIGN-REPORT.md:322; primary 2026-11-08 #69 (b)): a restore only
/// moves the fighters, so nothing is written into the bound save. The slot gets the bound save back; the post waits
/// until the restore has landed (`states::apply_bound_post`).
pub fn setup_restored() {
    SLOT_RESYNC.store(true, Ordering::Release);
    BOUND_DIRTY.store(true, Ordering::Release);
}

/// The state writer applied a setup bind from the Setup library to the active slot. A (`restore`) moves the fighters
/// and the slot then gets the bound save back (`setup_restored`). Y binds without moving anyone: the setup stays in
/// the slot, so Load returns to it, until the next bound-save change (primary 2026-11-08 23:03Z, SETUP-Y-BIND;
/// cov-rec #69 (b)). Neither writes the bound save file.
pub fn setup_bound(restore: bool) {
    if restore {
        setup_restored();
    }
}

/// The game's `SITUATION_KIND_AIR` (ground 0, cliff 1).
const SITUATION_AIR: i32 = 2;

pub const PLACE_NO_MACRO: &str = "Make a macro active for this role first";
pub const PLACE_NO_START: &str = "Link a start save to the macro first";

/// The save to open and the macro to relink on Copy. Reading the save belongs to `place_open`.
fn active_placement_ids(root: &Path, role: Role) -> Result<(String, String), &'static str> {
    let index = RecordingIndex::load(root).map_err(|_| PLACE_NO_MACRO)?;
    let active = index.active(role).ok_or(PLACE_NO_MACRO)?.to_owned();
    // A chain warps to macro 1's start save and plays with macro 1's placement (`play_placement`), so it edits macro 1.
    let (id, chain_on) = match Store::<Chain>::new(root.join(MACROS_DIR)).load(&active) {
        Ok(chain) => (chain.recordings.first().cloned().ok_or(PLACE_NO_MACRO)?, Some(chain.start_save_on)),
        Err(_) => (active, None),
    };
    let recording = Library::new(root.join(MACROS_DIR)).load(&id).map_err(|_| PLACE_NO_MACRO)?;
    let start_on = chain_on.unwrap_or(recording.start_save_on);
    let save = recording.start_save.filter(|_| start_on).ok_or(PLACE_NO_START)?;
    Ok((save, id))
}

/// The placement a Play of `id` warps with: the file's own (a chain: macro 1's, whose start save it is).
fn play_placement(book: &Book, id: &str) -> Placement {
    let first = book.first_recording(id).unwrap_or(id);
    book.placement(first)
}

/// The pair a Play warps to: the start save, with the file's placement added to the playing role's saved position
/// (`Placement::apply` on a fresh anchor every warp, so repeated trials never add it twice). A raised start on the
/// ground is in the air, as the named layer's `restore_in_air` had it.
pub fn warp_pair(save: NamedSave, place: Option<(Role, Placement)>) -> [SavedState; 2] {
    let mut pair = [save.player, save.cpu];
    if let Some((role, placement)) = place {
        let record = &mut pair[role_index(role)];
        (record.x, record.y) = placement.apply((record.x, record.y));
        if placement.delta_y != 0.0 && record.situation_kind == 0 {
            record.situation_kind = SITUATION_AIR;
            if matches!(record.start_state, PlacementStartState::MissedTechUp | PlacementStartState::MissedTechDown | PlacementStartState::Trip) {
                record.start_state = PlacementStartState::Standard;
            }
        }
    }
    pair
}

/// Change placement: a save the placement picker is asked to open on, as the worker read it. `save` is its file,
/// `relink` the macro a copy is linked to (the macro whose screen asked, a chain's macro 1); the picker hands both
/// back with what it confirmed (`commit_placement`).
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceOpen {
    pub save: String,
    pub relink: Option<String>,
    pub name: String,
    pub pair: [SavedState; 2],
    /// The fighter the picker edits first.
    pub edit: Role,
    pub shared: Shared,
}

/// What a confirmed placement did to its save, for the Switch wrapper.
#[derive(Clone, Debug, PartialEq)]
pub enum Placed {
    /// Nothing was written: the save, its copy or the macro's link could not be read or written. `name`: the
    /// save's, when it could be read.
    Refused { ticket: u32, name: String },
    /// The save holds the placement (or nothing was edited): the pair to load once, and the notice.
    Saved { ticket: u32, pair: Box<[SavedState; 2]>, notice: String },
}

/// The save `save` as the placement picker opens on it, with the macro files that start from it counted.
#[inline(never)]
fn place_open(root: &Path, save: String, relink: Option<String>, edit: Role) -> Option<Box<PlaceOpen>> {
    let file = Store::<NamedSave>::new(root.join(SAVES_DIR)).load(&save).ok()?;
    let shared = Shared { macros: macros_starting_from(root, &save), from_macro: relink.is_some() };
    Some(Box::new(PlaceOpen { save, relink, name: file.name, pair: [file.player, file.cpu], edit, shared }))
}

/// How many macro files start from the save `save`. The worker reads the macro folder once when Change placement
/// is asked for: the book it holds for Play has only the files that play. A chain is not a macro file (it starts
/// from its macro 1's save), so it is not counted; a file that does not read as a macro is passed over.
/// Kept out of line, as the held book's reader is: a loaded macro must not sit in `service_at`'s frame.
#[inline(never)]
fn macros_starting_from(root: &Path, save: &str) -> u8 {
    let library = Library::new(root.join(MACROS_DIR));
    let Ok(folder) = std::fs::read_dir(root.join(MACROS_DIR)) else { return 0 };
    let count = folder
        .flatten()
        .filter_map(|item| item.file_name().to_str().and_then(|name| name.strip_suffix(".json")).map(str::to_owned))
        .filter(|id| library.load(id).is_ok_and(|file| file.start_save.as_deref() == Some(save)))
        .count();
    u8::try_from(count).unwrap_or(u8::MAX)
}

/// The picker confirmed a placement on the save its visit opened: the worker writes it. Only what was edited is
/// written, field by field (`record_place::patch`); with nothing edited no file is written and none is copied. A
/// copy is a new save, written before `relink` is linked to it; if that link cannot be written the copy is
/// removed again, so the old link stands. No macro file is touched otherwise: a macro's offset stays its own.
pub fn commit_placement(root: &Path, save: &str, relink: Option<&str>, commit: Commit) -> Placed {
    let ticket = commit.ticket;
    let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
    let Ok(mut file) = saves.load(save) else { return Placed::Refused { ticket, name: String::new() } };
    let name = file.name.clone();
    if !commit.edits.iter().any(|edit| edit.any()) {
        return Placed::Saved { ticket, pair: Box::new([file.player, file.cpu]), notice: format!("Nothing changed · {name}") };
    }
    crate::model::record_place::patch([&mut file.player, &mut file.cpu], &commit.setup, commit.edits, commit.stage);
    let notice = if commit.copy {
        match copy_placed(root, &saves, &file, relink) {
            Some(copy) => format!("Saved a copy · {copy}"),
            None => return Placed::Refused { ticket, name },
        }
    } else {
        if saves.overwrite(save, &file).is_err() {
            return Placed::Refused { ticket, name };
        }
        invalidate_bound();
        match commit.macros {
            0 => format!("Saved · {name}"),
            1 => format!("Saved · {name} · 1 macro starts here"),
            macros => format!("Saved · {name} · {macros} macros start here"),
        }
    };
    Placed::Saved { ticket, pair: Box::new([file.player, file.cpu]), notice }
}

/// The placed pair as a new save, and `relink` linked to it; the copy's name. `None`: nothing is left behind.
fn copy_placed(root: &Path, saves: &Store<NamedSave>, placed: &NamedSave, relink: Option<&str>) -> Option<String> {
    let names = saves.list().ok()?.entries.into_iter().map(|entry| entry.name).collect::<Vec<_>>();
    let copy = NamedSave::new(unused_name("Save", &names), placed.player.clone(), placed.cpu.clone()).ok()?;
    let id = saves.save_new(&copy).ok()?;
    if let Some(recording) = relink {
        let macros = Library::new(root.join(MACROS_DIR));
        let linked = macros.load(recording).ok().and_then(|mut file| {
            // Relink the store-issued id without enabling a standalone start the chain's first macro had disabled.
            file.start_save = Some(id.clone());
            macros.overwrite(recording, &file).ok()
        });
        if linked.is_none() {
            let _ = saves.delete(&id);
            return None;
        }
    }
    let _ = RecordingIndex::update(root, |index| {
        index.note_use(Kind::Save, &id, Use::Recorded, unix_now())?;
        Ok(true)
    });
    Some(copy.name)
}

/// Save writes the one bound save, or a new bound Save N when none is bound (`rr_runtime::save_chord`). Worker side.
/// `after_take` runs once the pair is taken and before it is written (a test holds the worker there;
/// the diag build can slow the write there). `BOUND_WRITING` covers it.
pub fn write_saved_with(root: &Path, after_take: impl FnOnce()) -> bool {
    BOUND_WRITING.store(true, Ordering::Release);
    let wrote = write_saved_pair(root, after_take);
    BOUND_WRITING.store(false, Ordering::Release);
    wrote
}

fn write_saved_pair(root: &Path, after_take: impl FnOnce()) -> bool {
    let Some([player, cpu]) = lock(&SAVED_PAIR).take() else {
        return false;
    };
    after_take();
    let Ok(id) = save_chord(root, player.clone(), cpu.clone(), unix_now()) else {
        return false;
    };
    *lock(&BOUND_APPLIED) = Some(BoundCopy { id: Some(id), pair: Some([player, cpu]) });
    invalidate_bound();
    true
}

/// Commit Options Loop edits independently of file selection. File actions still need both revisions.
pub fn take_menu_effect(
    root: &Path,
    screens: &mut crate::model::recording_menu::RecordingScreens,
    observed: Option<(u64, u64)>,
) -> Option<(Effect, bool, (u64, u64))> {
    screens.persist_loop_edits(|edits| write_menu_loops(root, edits));
    let effect = screens.take_effect();
    let changed = screens.take_selection_change();
    let version = recording_version();
    if screens.loading() || observed != Some(version) {
        screens.set_loading();
        return None;
    }
    Some((effect, changed, version))
}

// Keep index decoding off the menu input stack until a Loop edit actually needs it.
#[inline(never)]
fn write_menu_loops(root: &Path, edits: [Option<LoopMark>; 2]) -> bool {
    RecordingIndex::update(root, |index| {
        let mut changed = false;
        for (role, mark) in [Role::Player, Role::Cpu].into_iter().zip(edits) {
            if let Some(count) = mark.map(loop_count).filter(|&count| count != index.loop_count(role)) {
                index.set_loop(role, count);
                changed = true;
            }
        }
        Ok(changed)
    }).is_ok()
}

/// The menu's selection is what Play, Record, and the loop read. The index is that selection.
#[cfg(test)]
pub fn publish_selection(root: &Path, selection: &Selection, drill_kind: Option<DrillKind>) {
    publish_selection_at(root, selection, drill_kind, None);
}

pub fn publish_selection_at(
    root: &Path,
    selection: &Selection,
    drill_kind: Option<DrillKind>,
    expected: Option<(u64, u64)>,
) -> bool {
    if expected.is_some_and(|version| version != recording_version()) {
        return false;
    }
    let Ok(mut index) = RecordingIndex::load(root) else {
        return false;
    };
    let at = unix_now();
    let _ = index.set_active(Role::Player, selection.active_player.as_deref(), at);
    let _ = index.set_active(Role::Cpu, selection.active_cpu.as_deref(), at.saturating_add(1));
    let _ = index.bind_save(selection.bound.as_deref(), at.saturating_add(2));
    index.set_loop(Role::Player, loop_count(selection.loop_player));
    index.set_loop(Role::Cpu, loop_count(selection.loop_cpu));
    if let (Some(id), Some(kind)) = (selection.drill.as_deref(), drill_kind) {
        let _ = index.set_drill(Some(Drill { kind, id: id.to_string() }), at.saturating_add(3));
    }
    let held = index.slots().clone();
    let slots_changed = index.set_slots(&selection.slots).is_ok() && index.slots() != &held;
    if !matches!(index.save_at(root, expected), Ok(true)) {
        return false;
    }
    if slots_changed {
        note_slots_changed(true);
    }
    invalidate_bound();
    true
}

/// rc.12 TR1: the index's Playback Slots changed (or must be copied again): the worker copies them into the game
/// thread's slots (`recording_host::sync_slots`). 1: copy quietly (boot, a new session, a delete); 2: say what was
/// copied (the menu's change). Starts at 1: the first service after migration copies them.
static SLOTS_DIRTY: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

pub fn note_slots_changed(announce: bool) {
    SLOTS_DIRTY.fetch_max(if announce { 2 } else { 1 }, Ordering::AcqRel);
}

/// Whether the slots are to be copied, and whether to say so; once.
pub fn take_slots_dirty() -> Option<bool> {
    match SLOTS_DIRTY.swap(0, Ordering::AcqRel) {
        0 => None,
        n => Some(n == 2),
    }
}

/// What the worker copies into the game thread's Playback Slots: each slot whose wanted macro is not the one it holds,
/// and the wanted macro (`None` empties the slot).
pub fn slot_changes(wanted: &[Option<String>; 5], held: &[Option<String>; 5]) -> Vec<(usize, Option<String>)> {
    wanted.iter().zip(held).enumerate().filter(|(_, (w, h))| w != h).map(|(i, (w, _))| (i, w.clone())).collect()
}

/// What the grade helper should do when the recording drill id changes.
/// The same id is not applied again: applying a target clears the open attempt.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradeFollow<'a> {
    Same,
    Clear,
    Install(&'a str),
}

/// The helper's target follows the recording drill. `applied` is the id already
/// given to the helper. `None` clears it. A different id is installed once.
#[cfg(test)]
pub fn follow_grade<'a>(applied: Option<&str>, drill: Option<&'a str>) -> GradeFollow<'a> {
    if applied == drill {
        GradeFollow::Same
    } else if let Some(id) = drill {
        GradeFollow::Install(id)
    } else {
        GradeFollow::Clear
    }
}

/// A sequence drill starts at step 1 over the sequence's combos that still open, going on from the streak and best
/// saved in its file. None when the file does not load.
#[cfg(test)]
pub fn start_sequence_drill(root: &Path, id: &str) -> Option<crate::model::drill::SequenceDrill> {
    let sequence = Store::<Sequence>::new(root.join(SEQUENCES_DIR)).load(id).ok()?;
    let combos = ComboLibrary::new(root.join("combos"));
    let present = sequence.present_ids(&combos);
    let ids = present.iter().map(String::as_str).collect::<Vec<_>>();
    Some(crate::model::drill::SequenceDrill::resume(id, &ids, sequence.streak, sequence.best))
}

/// One combo of a drilled sequence: its id, its name and the links the helper grades.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceStep {
    pub id: String,
    pub name: String,
    pub combo: Combo,
    pub fighter: Option<crate::model::move_names::FighterId>,
}

/// What the frame hook drills for a sequence: its name, the combos that still open in file order, and the streak
/// and best saved in its file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequencePlan {
    pub id: String,
    pub name: String,
    pub steps: Vec<SequenceStep>,
    pub streak: u32,
    pub best: u32,
}

impl SequencePlan {
    /// The same combos in the same order: a drill in progress goes on. A rename or a new streak is not a change.
    pub fn same_steps(&self, other: &SequencePlan) -> bool {
        self.id == other.id
            && self.steps.len() == other.steps.len()
            && self.steps.iter().zip(&other.steps).all(|(a, b)| a.id == b.id && a.combo == b.combo && a.fighter == b.fighter)
    }
}

/// The plan of sequence `id`, read from the card. A deleted combo is skipped. None when the file does not load.
pub fn sequence_plan(root: &Path, id: &str) -> Option<SequencePlan> {
    let sequence = Store::<Sequence>::new(root.join(SEQUENCES_DIR)).load(id).ok()?;
    let combos = ComboLibrary::new(root.join("combos"));
    let steps = sequence
        .combos
        .iter()
        .filter_map(|combo_id| {
            let record = combos.load(combo_id).ok()?;
            if record.combo.input_only { return None; }
            Some(SequenceStep { id: combo_id.clone(), name: record.name, combo: record.combo, fighter: record.fighter })
        })
        .collect();
    Some(SequencePlan { id: id.to_owned(), name: sequence.name, steps, streak: sequence.streak, best: sequence.best })
}

/// The drilled sequence's streak and best go to its file. Nothing else in the file changes.
pub fn save_sequence_progress(root: &Path, id: &str, streak: u32, best: u32) -> bool {
    let store = Store::<Sequence>::new(root.join(SEQUENCES_DIR));
    let Ok(mut sequence) = store.load(id) else { return false };
    if (sequence.streak, sequence.best) == (streak, best) {
        return true;
    }
    sequence.streak = streak;
    sequence.best = best;
    store.overwrite(id, &sequence).is_ok()
}

/// What the frame hook should drill when the menu's one drill changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrillFollow<'a> {
    /// Already drilled, or a drill whose file the catalog does not list yet.
    Same,
    Clear,
    Combo(&'a str),
    Sequence(&'a str),
}

/// The one drill: a combo replaces a drilled sequence and a sequence replaces a drilled combo. `applied` is what the
/// frame hook drills now. An id the catalog does not list yet is retried on a later post.
pub fn follow_drill<'a>(
    applied: Option<(DrillKind, &str)>,
    kind: Option<DrillKind>,
    drill: Option<&'a str>,
) -> DrillFollow<'a> {
    match (drill, kind) {
        (None, _) if applied.is_none() => DrillFollow::Same,
        (None, _) => DrillFollow::Clear,
        (Some(_), None) => DrillFollow::Same,
        (Some(id), Some(kind)) if applied == Some((kind, id)) => DrillFollow::Same,
        (Some(id), Some(DrillKind::Combo)) => DrillFollow::Combo(id),
        (Some(id), Some(DrillKind::Sequence)) => DrillFollow::Sequence(id),
    }
}

/// What the helper grades for the recording drill. A combo drill grades itself. A sequence drill grades its current
/// step, but only when `sequence` is the drill of that same id. No drill, or an unknown kind, grades nothing.
#[cfg(test)]
pub fn drill_grade_target<'a>(
    kind: Option<DrillKind>,
    drill: Option<&'a str>,
    sequence: Option<&'a crate::model::drill::SequenceDrill>,
) -> Option<&'a str> {
    match kind {
        Some(DrillKind::Combo) => drill,
        Some(DrillKind::Sequence) => match (drill, sequence) {
            (Some(id), Some(sequence)) if sequence.id() == id => sequence.target(),
            _ => None,
        },
        None => None,
    }
}

/// The drill changed. A combo recorded on another fighter asks; the drill itself stays.
pub fn note_drill(id: Option<String>) {
    let mut last = lock(&LAST_DRILL);
    if !DRILL_SEEN.swap(true, Ordering::AcqRel) {
        *last = id;
        return;
    }
    if *last == id {
        return;
    }
    *last = id.clone();
    *lock(&DRILL_ASK) = id;
}

fn loop_count(mark: LoopMark) -> LoopCount {
    match mark {
        LoopMark::Infinite => LoopCount::Infinite,
        LoopMark::Finite(count) => LoopCount::try_from(count).unwrap_or(LoopCount::Finite(1)),
    }
}

fn role_index(role: Role) -> usize {
    match role {
        Role::Player => 0,
        Role::Cpu => 1,
    }
}

fn role_kind(role: Role) -> i32 {
    let kind = ROLE_KIND[role_index(role)].load(Ordering::Acquire);
    if kind >= 0 {
        kind
    } else if role == Role::Player {
        let current = CURRENT_KIND.load(Ordering::Acquire);
        if current == u32::MAX { -1 } else { current as i32 }
    } else {
        -1
    }
}

fn open_play_track(book: &Book, role: Role, id: &str, limit: usize, pass: bool) {
    let kind = role_kind(role);
    if id == CLIP_ID || book.is_chain(id) || kind < 0 || book.recorded_fighter(id) != Some(kind) { return; }
    let Some(frames) = book.frame_count(id) else { return };
    let slot = role_index(role);
    PLAY.begin(id, slot, RUN_GEN[slot].load(Ordering::SeqCst), recording_version(), limit.min(frames) as u32, pass);
}

fn apply_waiting_answer(tick: &mut Tick) {
    if lock(&HELD).is_some() {
        return;
    }
    let Some(choice) = lock(&ANSWER).take() else {
        return;
    };
    let Some(ask) = lock(&LAST_ASK).take() else {
        return;
    };
    if choice == MismatchChoice::Switch {
        tick.switch_name = Some(ask.recorded_label);
        tick.switch_player = true;
    }
}

fn release_held(play_player: &mut bool, play_cpu: &mut bool) {
    let Some(role) = *lock(&HELD) else {
        return;
    };
    if lock(&ANSWER).is_none() {
        return;
    }
    *lock(&HELD) = None;
    match role {
        Role::Player => *play_player = true,
        Role::Cpu => *play_cpu = true,
    }
}

fn play_role(root: &Path, machine: &mut Playback, book: &Book, role: Role, tick: &mut Tick) {
    play_role_with(root, machine, book, role, tick, false);
}

fn play_role_with(root: &Path, machine: &mut Playback, book: &Book, role: Role, tick: &mut Tick, next: bool) {
    if !book.index.chord_notice_seen() { show_chord_notice(root, tick); }
    if let Some(ask) = mismatch_for(book, role) {
        // Ask before switching fighter Off (operator 2026-11-13, TR6): the current fighter stays, as Keep, unasked.
        let asking = ASK_FIGHTER.load(Ordering::Acquire);
        let answer = lock(&ANSWER).take().filter(|_| asking);
        match answer.or((!asking).then_some(MismatchChoice::Keep)) {
            None => {
                tick.diags.push(format!(
                    "screen=mismatch ask={} rows=Keep current fighter|Switch to recorded fighter",
                    ask.origin
                ));
                *lock(&LAST_ASK) = Some(ask.clone());
                tick.mismatch = Some(ask);
                *lock(&HELD) = Some(role);
                return;
            }
            Some(MismatchChoice::Cancel) => {
                tick.diags.push(format!("screen=mismatch ask={} answer=cancel", ask.origin));
                return;
            }
            Some(MismatchChoice::Keep) => {
                let off = if asking { "" } else { " prompt=off" };
                let line = format!("screen=mismatch ask={}{off} answer=keep", ask.origin);
                tick.diags.push(line);
            }
            Some(MismatchChoice::Switch) => {
                tick.diags.push(format!("screen=mismatch ask={} answer=switch", ask.origin));
                tick.switch_kind = Some(ask.recorded_kind);
                tick.switch_player = role == Role::Player;
                // Only the CPU loads an exact Mii (the ordinary CPU Mii loader); Player 1 keeps the kind route.
                if role == Role::Cpu {
                    tick.switch_mii = book.index.active(role).and_then(|id| book.exact_mii_target(id));
                }
            }
        }
    } else {
        // The fighters match now: an answer to an earlier question applies to nothing and must not answer the next one.
        lock(&ANSWER).take();
    }
    if !machine.playing(role) && !role_frames(role).is_empty() {
        // The machine has handed over its last frames but the hook is still playing them: the chord stops
        // that run, as it does while the machine still steps.
        drop_frames(role);
        run_flag(role).store(false, Ordering::Release);
        // A parry practice stops with it: no pass stays owed for `parry_next_pass` (tr-review-astra-2, finding 1).
        if role == Role::Cpu {
            PARRY_PASSES.store(0, Ordering::Release);
        }
        return;
    }
    if role == Role::Cpu && PARRY.load(Ordering::Acquire) && book.index.active(role).is_none() {
        tick.notices.push(PARRY_NO_RECORDING);
    }
    let effect = match parry_play(machine, role, book, next, tick) {
        Some(effect) => effect,
        None => drive_play(machine, role, book),
    };
    let word = if role == Role::Player {
        "player"
    } else {
        "cpu"
    };
    let mut owed = None;
    if let Some(id) = effect.warp {
        if let Some(active) = book.index.active(role) {
            let placement = play_placement(book, active);
            owed = name_warp(book, role, active, id, tick).map(|speed| (speed, placement.momentum));
        } else {
            tick.warp = Some(id.to_owned());
            WARP_HOLD.store(true, Ordering::Release);
        }
        tick.diags
            .push(format!("screen=playback role={word} warp=1"));
        if let Some((speed, mode)) = owed {
            let mode = match mode {
                MomentumMode::PreserveRecorded => "keep",
                MomentumMode::AtRest => "rest",
            };
            tick.diags.push(format!(
                "screen=playback role={word} momentum={mode} speed_x={} speed_y={} facing={}",
                speed.speed.0, speed.speed.1, speed.facing
            ));
        }
    }
    // A parry refusal and the deleted-save notice share Recording's standing notice: the refusal stays up.
    if !tick.notices.contains(&PARRY_START_SAVE_MISSING) {
        note_missing_start(book, role, tick);
    }
    if effect.stopped || effect.started {
        drop_frames(role);
    }
    if effect.started {
        mark_run(role);
        if let Some(id) = book.index.active(role).filter(|id| book.plays_from_zero(id)) {
            open_play_track(book, role, id, usize::MAX, true);
        }
        if !next { owe_delay(role); }
    }
    // PLAY-MOMENTUM: the warp's start speed waits for the run's first input (after `drop_frames`, which clears it).
    if let Some((speed, _)) = owed {
        owe_start_speed(role, Some(speed));
    }
    if effect.stopped {
        run_flag(role).store(false, Ordering::Release);
    }
    if effect.started {
        run_flag(role).store(true, Ordering::Release);
    }
    if run_flag(role).load(Ordering::Acquire) {
        step_role(machine, book, role);
    }
}

/// Fill the role's queue ahead of its hook, in file order. The hook takes one frame per game frame, so a
/// late or doubled worker tick neither drops nor repeats one.
fn step_role(machine: &mut Playback, book: &Book, role: Role) {
    if WARP_HOLD.load(Ordering::Acquire) {
        return;
    }
    // A cut run (its flag cleared by a mash) stops here: `publish` refuses its frames, so an endless take would never
    // fill the queue (Astra 09:48Z).
    while run_flag(role).load(Ordering::Acquire) && role_frames(role).room() > 0 {
        match drive_step(machine, role, book) {
            Some(played) => publish(role, played.frame),
            None => {
                run_flag(role).store(false, Ordering::Release);
                return;
            }
        }
    }
}

#[cfg(test)]
thread_local! {
    static POLL_ON_PUBLISH: core::cell::Cell<Option<fn(Role)>> = const { core::cell::Cell::new(None) };
    static BEFORE_TAKE: core::cell::Cell<Option<Box<dyn FnOnce()>>> = const { core::cell::Cell::new(None) };
}

/// A cut clears the run flag. If it races this publish, the hook's cut flag still
/// suppresses the old frame until the worker can clear the generation.
fn publish(role: Role, frame: InputFrame) {
    if run_flag(role).load(Ordering::Acquire) {
        // The worker checked room before advancing playback. The hook can only free more room.
        let queued = role_frames(role).push(pack(frame));
        debug_assert!(queued, "replay advanced without room for its frame");
        #[cfg(test)]
        if let Some(poll) = POLL_ON_PUBLISH.take() { poll(role); }
    }
}

/// A run started, stopped or was cut by a mash: what it buffered is not played, and a start speed it still owed is not
/// given.
fn drop_frames(role: Role) {
    role_frames(role).clear();
    owe_start_speed(role, None);
    DELAY_OWED[role_index(role)].store(0, Ordering::Release);
}

fn note_missing_start(book: &Book, role: Role, tick: &mut Tick) {
    let Some(id) = book.index.active(role) else {
        return;
    };
    if !book.missing_recording_save(id) {
        return;
    }
    if let Some(text) = start_save_notice(StartSaveState::Deleted) {
        tick.notices.push(text);
        tick.diags
            .push("screen=notice kind=macro-start-save-missing".to_string());
    }
}

fn mismatch_for(book: &Book, role: Role) -> Option<MismatchAsk> {
    let current = role_kind(role);
    if current < 0 {
        return None;
    }
    let id = book.index.active(role)?;
    let recorded = book.recorded_fighter(id)?;
    if recorded < 0 || recorded == current {
        return None;
    }
    let origin = if book.is_chain(id) { "macro1" } else { "macro" };
    Some(MismatchAsk {
        origin,
        role: match role {
            Role::Player => ViewRole::Player,
            Role::Cpu => ViewRole::Cpu,
        },
        recorded_kind: recorded,
        current_kind: current,
        recorded_label: recorded.to_string(),
        current_label: current.to_string(),
    })
}

fn ask_drill(root: &Path, tick: &mut Tick) {
    let Some(id) = lock(&DRILL_ASK).take() else {
        return;
    };
    let Ok(record) = ComboLibrary::new(root.join("combos")).load(&id) else {
        return;
    };
    let Some(recorded) = record.recorded_on.filter(|name| !name.is_empty()) else {
        return;
    };
    let current = lock(&PLAYER_LABEL).clone();
    if current.is_empty() || current == recorded {
        return;
    }
    if !ASK_FIGHTER.load(Ordering::Acquire) {
        // TR6: with Ask before switching fighter Off the drill stays on Player 1's fighter, unasked.
        tick.diags.push("screen=mismatch ask=combo prompt=off answer=keep".to_string());
        return;
    }
    tick.diags.push(
        "screen=mismatch ask=combo rows=Keep current fighter|Switch to recorded fighter".to_string(),
    );
    let ask = MismatchAsk {
        origin: "combo",
        role: ViewRole::Player,
        recorded_kind: -1,
        current_kind: role_kind(Role::Player),
        recorded_label: recorded,
        current_label: current,
    };
    *lock(&LAST_ASK) = Some(ask.clone());
    tick.mismatch = Some(ask);
}

fn apply_menu(root: &Path, machine: &mut Playback, book: &Book, tick: &mut Tick) {
    let Some((expected, effect)) = lock(&MENU_EFFECT).take() else { return };
    if expected.is_some_and(|version| version != recording_version()) {
        return;
    }
    match effect {
        Effect::None | Effect::RecordDrill | Effect::ListRecordings | Effect::CancelRecording | Effect::OpenRecording { .. } => {}
        Effect::ChangePlacement { save, relink } => tick.place = place_open(root, save, relink, record_role()),
        // The Replays row: the start save of the pop-up role's active macro (a chain: its macro 1's).
        Effect::ChangeActivePlacement => {
            let role = record_role();
            match active_placement_ids(root, role) {
                Ok((save, id)) => {
                    tick.place = place_open(root, save, Some(id), role);
                    if tick.place.is_none() { tick.notices.push(PLACE_NO_START); }
                }
                Err(refusal) => tick.notices.push(refusal),
            }
        },
        Effect::Keyboard { header, draft } => {
            tick.diags.push(format!("screen=rename header={header}"));
            tick.keyboard = Some(KeyboardOffer { header, draft });
        }
        Effect::OpenCrop { .. } => {} // The menu worker opens the shared clip editor.
        Effect::PreviewDrill { .. } => {} // The menu worker starts the preview (F10).
        Effect::MakeCombo { source, chain } => {
            let _ = chain;
            begin_make(machine, book, MakeAsk { source, role: Role::Player, drill: None }, tick)
        }
        Effect::DuplicateSave { source } => {
            let _ = duplicate_save(root, &source, unix_now());
        }
        Effect::DeleteSave { source } => {
            if delete_save(root, &source, unix_now()).is_ok() {
                invalidate_bound();
            }
        }
        Effect::WriteChain { name, steps } => save_new_record(root, MACROS_DIR, Chain::new(name, steps).ok()),
        Effect::WriteSequence { name, combos } => save_new_record(root, SEQUENCES_DIR, Sequence::new(name, combos).ok()),
        Effect::RenameSave { id, name } => {
            rename::<NamedSave>(root, SAVES_DIR, &id, |f| f.name = name)
        }
        Effect::RenameMacro { id, name } => {
            rename::<MacroRecording>(root, MACROS_DIR, &id, |f| f.name = name)
        }
        Effect::RenameChain { id, name } => {
            rename::<Chain>(root, MACROS_DIR, &id, |f| f.name = name)
        }
        Effect::RenameCombo { id, name } => {
            rename::<ComboRecord>(root, "combos", &id, |f| f.name = name)
        }
        Effect::RenameSequence { id, name } => {
            rename::<Sequence>(root, SEQUENCES_DIR, &id, |f| f.name = name)
        }
        Effect::DeleteFile { kind, id } => {
            delete_file(root, kind, &id);
        }
        Effect::WriteChainSteps { id, steps } => rename::<Chain>(root, MACROS_DIR, &id, |f| f.recordings = steps),
        Effect::WriteSequenceCombos { id, combos } => rename::<Sequence>(root, SEQUENCES_DIR, &id, |f| f.combos = combos),
        Effect::SaveLastCombo { name } => save_last_combo(root, &name, tick),
        Effect::SetStartSave { id, chain: true, on } => rename::<Chain>(root, MACROS_DIR, &id, |f| f.start_save_on = on),
        Effect::SetStartSave { id, chain: false, on } => rename::<MacroRecording>(root, MACROS_DIR, &id, |f| f.start_save_on = on),
        Effect::SetTrim { id, kind: FileKind::Macro, trim } => {
            rename::<MacroRecording>(root, MACROS_DIR, &id, |file| file.trim = trim);
            note_slots_changed(false);
        }
        Effect::SetTrim { id, kind: FileKind::Combo, trim } => {
            let drills = ComboLibrary::new(root.join("combos"));
            if let Ok(mut file) = drills.load(&id) {
                if file.combo.input_only {
                    file.trim = trim;
                    let _ = drills.overwrite(&id, &file);
                }
            }
        }
        Effect::SetTrim { .. } => {}
        // The macro's own screen (operator 2026-11-25, PC6 A): each row writes its one field into the macro's file.
        Effect::SetMomentum { id, rest } => {
            let momentum = if rest { MomentumMode::AtRest } else { MomentumMode::PreserveRecorded };
            rename::<MacroRecording>(root, MACROS_DIR, &id, |file| file.placement.momentum = momentum);
        }
        Effect::ClearOffset { id } => {
            rename::<MacroRecording>(root, MACROS_DIR, &id, |file| (file.placement.delta_x, file.placement.delta_y) = (0.0, 0.0));
        }
    }
}

/// *+ Save last combo…* (operator 2026-11-08; rc.4 COMBO-HELPER): the strip's latest exchange becomes a combo file
/// named `name` (empty: Combo N), recorded on Player 1's fighter, hot, and the drill (RECORDING.md).
fn save_last_combo(root: &Path, name: &str, tick: &mut Tick) {
    let (_, combo, owner) = *lock(&STRIP);
    let Some(combo) = combo.filter(|combo| !combo.is_empty()) else {
        tick.notices.push(EMPTY_COMBO);
        tick.diags.push("screen=notice kind=combo-empty".to_string());
        return;
    };
    if combo.overflow { tick.notices.push(COMBO_CAPACITY); return; }
    let label = super::rr_runtime::fighter_name(owner.kind());
    let recorded_on = (!label.is_empty()).then_some(label);
    if let Ok(id) = super::rr_runtime::save_last_combo(root, &combo, name, recorded_on, owner.fighter(), unix_now()) {
        tick.diags.push(format!("screen=save-last-combo saved={id} links={}", combo.len()));
    }
}

/// Delete on a macro, a chain, a combo or a sequence: the file goes, and the index no longer names it as a role's
/// active macro or as the drill. A sequence that lists a deleted combo skips it.
pub fn delete_file(root: &Path, kind: crate::model::recording_menu::FileKind, id: &str) -> bool {
    use crate::model::recording_menu::FileKind;
    let deleted = match kind {
        FileKind::Macro => Library::new(root.join(MACROS_DIR)).delete(id).is_ok(),
        FileKind::Chain => Store::<Chain>::new(root.join(MACROS_DIR)).delete(id).is_ok(),
        FileKind::Combo => super::rr_runtime::delete_combo(root, id),
        FileKind::Sequence => Store::<Sequence>::new(root.join(SEQUENCES_DIR)).delete(id).is_ok(),
        // A saved capture's files. Captures are in no index, so nothing else is touched.
        FileKind::Capture => {
            return id.strip_prefix("saved:")
                .is_some_and(|id| super::macros::CaptureArchive::new(root.join("replay_captures")).delete(id).unwrap_or(false));
        }
    };
    if !deleted {
        return false;
    }
    let mut slots_changed = false;
    let updated = RecordingIndex::update(root, |index| {
        let at = unix_now();
        let mut changed = false;
        for role in [Role::Player, Role::Cpu] {
            if index.active(role) == Some(id) {
                index.set_active(role, None, at)?;
                changed = true;
            }
        }
        if index.drill().is_some_and(|drill| drill.id == id) {
            index.set_drill(None, at)?;
            changed = true;
        }
        slots_changed = index.clear_slot_of(id);
        Ok(changed || slots_changed)
    });
    if updated.is_ok() && slots_changed {
        note_slots_changed(false);
    }
    true
}

fn save_new_record<R: Record>(root: &Path, dir: &str, file: Option<R>) {
    if let Some(file) = file {
        let _ = Store::<R>::new(root.join(dir)).save_new(&file);
    }
}

/// Rename on a save, a macro, a chain, a combo or a sequence, or a macro's or chain's Start save switch: the file is
/// rewritten under its id.
fn rename<R: Record>(root: &Path, dir: &str, id: &str, set: impl FnOnce(&mut R)) {
    let store = Store::<R>::new(root.join(dir));
    let Ok(mut file) = store.load(id) else { return };
    set(&mut file);
    let _ = store.overwrite(id, &file);
}

fn begin_make(
    machine: &mut Playback,
    book: &Book,
    ask: MakeAsk,
    tick: &mut Tick,
) {
    let _ = warp_to_start(book, ask.role, &ask.source, tick);
    MAKE_CPU_FRAME.store(0, Ordering::Release);
    MAKE_ROLE.store(if ask.role == Role::Player { ROLE_PLAYER } else { ROLE_CPU }, Ordering::Release);
    begin_input(1);
    let started = start_run(machine, book, ask.role, &ask.source, None, None, true);
    let since = lock(&STRIP).0;
    tick.diags.push(format!("screen=make-combo source={} role={} playing={}", ask.source,
        if ask.role == Role::Player { "player" } else { "cpu" }, i32::from(started)));
    *lock(&MAKE) = Some(MakeJob { source: ask.source, role: ask.role, drill: ask.drill, ready: true, since });
}

fn name_warp(book: &Book, role: Role, source: &str, save: &str, tick: &mut Tick) -> Option<StartSpeed> {
    let placement = play_placement(book, source);
    tick.warp = Some(save.to_owned());
    tick.warp_place = Some((role, placement));
    WARP_HOLD.store(true, Ordering::Release);
    play_start_speed(book, source, placement)
}

fn warp_to_start(book: &Book, role: Role, source: &str, tick: &mut Tick) -> Option<StartSpeed> {
    match book.start_link(source) {
        StartLink::Deleted => {
            if let Some(text) = start_save_notice(super::library::StartSaveState::Deleted) {
                tick.notices.push(text);
                tick.diags
                    .push("screen=notice kind=macro-start-save-missing".to_string());
            }
            None
        }
        StartLink::Linked(id) => name_warp(book, role, source, id, tick),
        StartLink::None => None,
    }
}

fn start_run(
    machine: &mut Playback,
    book: &Book,
    role: Role,
    source: &str,
    window: Option<PreviewWindow>,
    owed: Option<StartSpeed>,
    delayed: bool,
) -> bool {
    let (effect, track_end) = match window {
        Some(PreviewWindow::Exact(frames)) => {
            let end = (frames.start == 0).then_some(frames.end);
            (machine.play_range(role, book, source, frames), end)
        }
        Some(PreviewWindow::Drill { frames, trim }) => {
            let range = book.drill_preview_range(source, frames as usize, trim);
            (machine.play_range(role, book, source, range), None)
        }
        None => (machine.play_file(role, book, source), None),
    };
    if effect.started {
        drop_frames(role);
        if delayed { owe_delay(role); }
        mark_run(role);
        // A poll can consume input zero as soon as step_role publishes it.
        if let Some(end) = track_end { open_play_track(book, role, source, end, false); }
        owe_start_speed(role, owed);
        run_flag(role).store(true, Ordering::Release);
        step_role(machine, book, role);
    }
    effect.started
}

fn take_preview_refusal(tick: &mut Tick) {
    let (seq, text) = PREVIEW_REFUSALS.read().0;
    if REFUSAL_SEEN.swap(seq, Ordering::AcqRel) != seq {
        tick.notices.push(text);
        PREVIEW_AWAY.store(0, Ordering::Release);
        PREVIEW.store(0, Ordering::Release);
        lock(&PREVIEW_CLIP).take();
        *lock(&PREVIEW_ASK) = PreviewRequests::default();
    }
}

/// The card's recordings cannot be read while a preview is asked for or running: the preview is over, so its role's
/// run ends and its queued frames go, as after a Stop. Nothing is handed over once `preview_running` is false and
/// nothing resumes when the card reads again.
fn retire_preview_run() {
    let role = match PREVIEW.load(Ordering::Acquire) {
        ROLE_PLAYER => Role::Player,
        ROLE_CPU => Role::Cpu,
        _ => return,
    };
    if let Some(machine) = lock(&MACHINE).as_mut() {
        drive_mash(machine, role, true);
    }
    run_flag(role).store(false, Ordering::Release);
    clear_cut(role);
}

fn drive_preview(machine: &mut Playback, book: &Book, tick: &mut Tick) {
    take_preview_refusal(tick);
    let role = preview_role();
    let (stop, queued) = {
        let mut asks = lock(&PREVIEW_ASK);
        (core::mem::take(&mut asks.stop), asks.start.is_some())
    };
    if stop {
        if let Some(role) = role {
            drive_mash(machine, role, true);
            run_flag(role).store(false, Ordering::Release);
            clear_cut(role);
        }
        PREVIEW.store(if queued { PREVIEW_REQUESTED } else { 0 }, Ordering::Release);
        if PREVIEW_AWAY.compare_exchange(1, 2, Ordering::AcqRel, Ordering::Acquire).is_ok() {
            tick.home = Some(HomeWarp::Return);
            WARP_HOLD.store(true, Ordering::Release);
            return;
        }
    }
    if PREVIEW_AWAY.load(Ordering::Acquire) == 2 {
        if WARP_HOLD.load(Ordering::Acquire) { return; }
        drop_preview_home();
        if !queued { lock(&PREVIEW_CLIP).take(); }
    }
    let start = lock(&PREVIEW_ASK).start.take();
    match start {
        Some(PreviewStart { source, role, window, playback }) => {
            PREVIEW.store(if role == Role::Player { ROLE_PLAYER } else { ROLE_CPU }, Ordering::Release);
            let owed = if source == CLIP_ID {
                PREVIEW_AWAY.store(1, Ordering::Release);
                tick.home = Some(HomeWarp::Start {
                    air: book.start_state(CLIP_ID).is_some_and(|start| start.situation == SITUATION_AIR),
                    player: role == Role::Player,
                });
                WARP_HOLD.store(true, Ordering::Release);
                play_start_speed(book, CLIP_ID, Placement::default())
            } else {
                warp_to_start(book, role, &source, tick)
            };
            if !start_run(machine, book, role, &source, Some(window), owed, playback) {
                PREVIEW.store(0, Ordering::Release);
            }
        }
        None if role.is_some_and(|role| !role_playing(role)) => PREVIEW.store(0, Ordering::Release),
        None => {}
    }
}

fn finish_make(root: &Path, tick: &mut Tick) {
    let ready = lock(&MAKE).as_ref().is_some_and(|job| job.ready && !role_playing(job.role));
    if !ready || converted_input(1).is_none() {
        return;
    }
    let Some(job) = lock(&MAKE).take() else {
        return;
    };
    MAKE_ROLE.store(0, Ordering::Release);
    if let Some(name) = job.drill {
        let take = converted_input(1).expect("observed take").capture;
        INPUT_MODE.fetch_and(!1, Ordering::AcqRel);
        if take.combo.overflow || take.combo.is_empty() {
            let _ = Library::new(root.join(MACROS_DIR)).delete(&job.source);
            tick.notices.push(if take.combo.overflow { DRILL_CAPACITY } else { super::rr_runtime::EMPTY_RECORDING });
        } else if super::rr_runtime::save_drill_with_take_named(root, &take.combo, None, None, job.source, &name, unix_now()).is_ok() {
            tick.diags.push("screen=input-drill saved=1".into());
        }
        return;
    }
    // Only an exchange the strip captured while the source played: a playback that lands no hit leaves the count
    // where it was, and the earlier exchange the strip still shows is not this file's.
    let (seen, sample) = {
        let mut strip = lock(&STRIP);
        (strip.0, strip.1.take())
    };
    let Some(combo) = sample.filter(|combo| seen != job.since && !combo.is_empty()) else {
        if !converted_input(1).unwrap().contact && !converted_input(1).unwrap().capture.combo.is_empty() { save_input(root, &job.source, false, &[], tick); return; }
        INPUT_MODE.fetch_and(!1, Ordering::AcqRel);
        tick.notices.push(EMPTY_COMBO);
        tick.diags.push("screen=notice kind=combo-empty".to_string());
        return;
    };
    INPUT_MODE.fetch_and(!1, Ordering::AcqRel);
    if combo.overflow { tick.notices.push(COMBO_CAPACITY); return; }
    if save_made_combo(root, &combo, &job.source, None, None, unix_now()).is_ok() {
        tick.diags.push(format!("screen=make-combo source={} saved=1", job.source));
    }
}

pub(crate) fn note_hot(root: &Path, kind: Kind, id: &str, follow: Option<(Role, &str)>) {
    let _ = RecordingIndex::update(root, |index| {
        let at = unix_now();
        index.note_use(kind, id, Use::Recorded, at)?;
        if let Some((role, _)) = follow.filter(|(role, source)| kind == Kind::Macro && index.active(*role) == Some(*source)) {
            index.set_active(role, Some(id), at)?;
        }
        Ok(true)
    });
}

fn show_chord_notice(root: &Path, tick: &mut Tick) {
    let mut text = None;
    if RecordingIndex::update(root, |index| {
        text = chord_notice(index);
        Ok(text.is_some())
    }).is_ok() {
        let Some(text) = text else { return };
        tick.notices.push(text);
        tick.diags.push("screen=notice kind=chord-once".to_string());
    }
}

/// Parry practice's saved list (menu worker, DESIGN §5.2): whether macro `id` still opens, and its start-save link.
pub fn parry_check(root: &Path, id: &str) -> (bool, super::library::StartSaveState) {
    use super::library::StartSaveState;
    let Ok(book) = load_book(root) else { return (false, StartSaveState::None) };
    let start = match book.start_link(id) {
        StartLink::Deleted => StartSaveState::Deleted,
        StartLink::Linked(_) => StartSaveState::Linked,
        StartLink::None => StartSaveState::None,
    };
    (book.has_plan(id), start)
}

/// Save this practice: the CPU's active macro and the CPU's Loop (`None` with no CPU macro).
pub fn parry_wiring(root: &Path) -> Option<(String, LoopCount)> {
    let index = RecordingIndex::load(root).ok()?;
    Some((index.active(Role::Cpu)?.to_owned(), index.loop_count(Role::Cpu)))
}

/// Use a saved practice: its macro becomes the CPU's active macro and its Loop the CPU's (the existing operations).
pub fn parry_use(root: &Path, id: &str, count: LoopCount) -> Result<(), String> {
    RecordingIndex::update(root, |index| {
        index.set_active(Role::Cpu, Some(id), unix_now())?;
        index.set_loop(Role::Cpu, count);
        Ok(true)
    }).map(|_| ()).map_err(|e| e.to_string())
}

fn read_held(root: &Path, mut sources: [Option<Source>; 2], roles: [bool; 2], starts: [bool; 2]) -> Result<HeldBook, ()> {
    let version = recording_version();
    let index = RecordingIndex::load(root).map_err(|_| ())?;
    let library = Library::new(root.join(MACROS_DIR));
    let chain_store = Store::<Chain>::new(root.join(MACROS_DIR));
    let save_store = Store::<NamedSave>::new(root.join(SAVES_DIR));
    let mut recordings = BTreeMap::<String, MacroRecording>::new();
    let mut chains = BTreeMap::<String, Chain>::new();
    let mut firsts = Vec::<String>::new();
    let mut seen_files = BTreeSet::new();
    for role in [Role::Player, Role::Cpu] {
        let slot = role_index(role);
        if !roles[slot] { continue; }
        if starts[slot] && sources[slot].is_none() {
            sources[slot] = index.active(role).map(|id| Source { id: id.to_owned(), active: true });
        }
        let source = sources[slot].as_ref();
        let current = source.is_none_or(|source| source.active).then_some(index.active(role)).flatten();
        for id in [source.map(|source| source.id.as_str()), current].into_iter().flatten() {
            if id == CLIP_ID || !seen_files.insert(id.to_owned()) { continue; }
            if !recordings.contains_key(id) && !chains.contains_key(id) {
                if let Ok(recording) = library.load(id) {
                    recordings.insert(id.to_owned(), recording);
                } else if let Ok(chain) = chain_store.load(id) {
                    for step in &chain.recordings {
                        if let std::collections::btree_map::Entry::Vacant(entry) = recordings.entry(step.clone()) {
                            if let Ok(recording) = library.load(step) {
                                entry.insert(recording);
                            }
                        }
                    }
                    chains.insert(id.to_owned(), chain);
                }
            }
            if let Some(chain) = chains.get(id) {
                if let Some(first) = chain.recordings.first() { firsts.push(first.clone()); }
            } else {
                firsts.push(id.to_owned());
            }
        }
    }
    let mut book = Book::new(index);
    let mut saves = Vec::new();
    let mut seen = BTreeSet::new();
    for first in firsts {
        let Some(save) = recordings.get(&first).and_then(|recording| recording.start_save.as_deref()) else { continue };
        if !seen.insert(save.to_owned()) { continue; }
        if let Ok(named) = save_store.load(save) {
            book.add_save(save);
            saves.push((save.to_owned(), named));
        }
    }
    for (id, recording) in recordings { book.add_recording(id, recording); }
    for (id, chain) in chains { book.add_chain(id, chain); }
    Ok(HeldBook { root: root.to_path_buf(), version, book, saves, sources })
}

/// Reload on a start, a root or mod-write generation change, or when no value is held.
fn held_needs_read(held: Option<&HeldBook>, root: &Path, starts: [bool; 2]) -> bool {
    held.is_none_or(|held| held.root != root || held.version != recording_version() || starts[0] || starts[1])
}

/// Reads the held files again when a role starts, the root changes, the mod wrote the library, or nothing is held.
/// Kept out of line: the reader's locals (the book being built, a loaded macro, chain and save) must not sit in
/// `service_at`'s frame, which is on the menu worker's deepest path (`thread_stacks`).
#[inline(never)]
fn refresh_held(held: &mut Option<Box<HeldBook>>, root: &Path, chords: [bool; 2]) -> bool {
    let mut starts = chords;
    if lock(&ANSWER).is_some() {
        if let Some(role) = *lock(&HELD) { starts[role_index(role)] = true; }
    }
    let preview = lock(&PREVIEW_ASK).start.as_ref().map(|start| (start.role, start.source.clone()));
    let make = lock(&MAKE_ASK).as_ref().map(|ask| (ask.role, ask.source.clone()));
    let menu_make = lock(&MENU_EFFECT).as_ref().and_then(|(_, effect)| match effect {
        Effect::MakeCombo { source, .. } => Some((Role::Player, source.clone())),
        _ => None,
    });
    for (role, _) in [&menu_make, &make, &preview].into_iter().flatten() {
        starts[role_index(*role)] = true;
    }
    if !held_needs_read(held.as_deref(), root, starts) { return true; }
    let in_use = [
        starts[0] || role_playing(Role::Player),
        starts[1] || role_playing(Role::Cpu) || PARRY_PASSES.load(Ordering::Acquire) > 0,
    ];
    let mut sources = held.as_ref().map_or_else(|| [None, None], |held| held.sources.clone());
    for role in [Role::Player, Role::Cpu] {
        let slot = role_index(role);
        if starts[slot] || !in_use[slot] { sources[slot] = None; }
    }
    for (role, source) in [menu_make, make, preview].into_iter().flatten() {
        sources[role_index(role)] = Some(Source { id: source, active: false });
    }
    match read_held(root, sources, in_use, starts) {
        Ok(next) => {
            if let Some(old) = held { PLAY.retire_book(&mut old.book); }
            *held = Some(Box::new(next));
            true
        }
        Err(()) => false,
    }
}

#[inline(never)]
fn hold_warp_pair(held: &HeldBook, tick: &mut Tick) {
    tick.warp_pair = tick.warp.as_deref().and_then(|id| held.saves.iter().find(|(save, _)| save == id))
        .map(|(_, save)| Box::new(warp_pair(save.clone(), tick.warp_place)));
}

fn load_book(root: &Path) -> Result<Book, ()> {
    let index = RecordingIndex::load(root).map_err(|_| ())?;
    let mut book = Book::new(index);
    let macros = Library::new(root.join(MACROS_DIR));
    if let Ok(list) = macros.list() {
        for entry in list.entries {
            if let Ok(recording) = macros.load(&entry.id) {
                book.add_recording(entry.id, recording);
            }
        }
    }
    let chains = Store::<Chain>::new(root.join(MACROS_DIR));
    if let Ok(list) = chains.list() {
        for entry in list.entries {
            if let Ok(chain) = chains.load(&entry.id) {
                book.add_chain(entry.id, chain);
            }
        }
    }
    let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
    if let Ok(list) = saves.list() {
        for entry in list.entries {
            book.add_save(entry.id);
        }
    }
    Ok(book)
}

fn taken_frames() -> (Role, Vec<InputFrame>) {
    let role = if LAST_ROLE.load(Ordering::Acquire) == ROLE_CPU {
        Role::Cpu
    } else {
        Role::Player
    };
    let len = CAP_LEN.swap(0, Ordering::AcqRel);
    let frames = (0..len)
        .map(|index| unpack(CAP_FRAMES[index].load(Ordering::Acquire)))
        .collect();
    (role, frames)
}

/// One frame for this game frame. Nothing while a warp's restore is pending: a frame fed before the
/// restore is undone by it, so the run waits where it is.
fn take_frame(role: Role, seed: impl FnOnce()) -> Option<InputFrame> {
    #[cfg(test)]
    if let Some(publish) = BEFORE_TAKE.take() { publish(); }
    pop_frame(role, |generation| PLAY.poll(|| credit_frame(role, generation, seed))).map(|(frame, _)| frame)
}

fn pop_frame(role: Role, taken: impl FnOnce(usize)) -> Option<(InputFrame, usize)> {
    // A pending restore must land before an input or it would undo that input.
    if WARP_HOLD.load(Ordering::Acquire) || start_gated() || cut_flag(role).load(Ordering::Acquire)
        || role == Role::Cpu && RESET_CPU.load(Ordering::Acquire)
    {
        return None;
    }
    // Count the role's polls on the game thread so the delay is exact in game frames.
    if DELAY_OWED[role_index(role)].fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1)).is_ok() {
        return None;
    }
    role_frames(role).pop_tagged_with(taken).map(|(frame, generation)| {
        PLAYED.fetch_add(1, Ordering::AcqRel);
        (unpack(frame), generation)
    })
}

fn credit_frame(role: Role, generation: usize, seed: impl FnOnce()) {
    let count = &FED[role_index(role)];
    let previous = count.load(Ordering::Relaxed);
    let generation = generation as u32;
    let next = if (previous >> 32) as u32 == generation {
        (previous as u32).wrapping_add(1)
    } else {
        1
    };
    count.store((u64::from(generation) << 32) | u64::from(next), Ordering::Release);
    PLAY.fed(role_index(role), generation, next, seed);
}

fn role_frames(role: Role) -> &'static FrameQueue {
    match role {
        Role::Player => &PLAYER_FRAMES,
        Role::Cpu => &CPU_FRAMES,
    }
}

fn menu_pending() -> bool {
    lock(&MENU_EFFECT).is_some()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn pack(frame: InputFrame) -> u64 {
    frame.buttons as u64
        | ((frame.lstick_x as u8 as u64) << 32)
        | ((frame.lstick_y as u8 as u64) << 40)
        | ((frame.rstick_x as u8 as u64) << 48)
        | ((frame.rstick_y as u8 as u64) << 56)
}

fn unpack(bits: u64) -> InputFrame {
    InputFrame {
        buttons: bits as u32,
        lstick_x: (bits >> 32) as u8 as i8,
        lstick_y: (bits >> 40) as u8 as i8,
        rstick_x: (bits >> 48) as u8 as i8,
        rstick_y: (bits >> 56) as u8 as i8,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}

#[cfg(test)]
pub(crate) fn reset_host() {
    PLAY.reset();
    slot_stop();
    for tape in &SLOT_TAPES {
        // SAFETY: host tests serialize the single publisher.
        unsafe { tape.retract(); }
    }
    for seq in &SLOT_SEQ { seq.store(0, Ordering::Release); }
    INPUT_MODE.store(0, Ordering::Release); DRILL_ARMED.store(false, Ordering::Release); for p in &INPUT_POLLS { p.store(0, Ordering::Release); } for e in &INPUT_EPOCH { e.store(0, Ordering::Release); } *INPUT_STATE.lock() = [InputTake::EMPTY; 2]; for p in &INPUT_RESULT { p.write(InputTake::EMPTY); } *lock(&SAVED_DRILL) = None;
    SLOTS_DIRTY.store(1, Ordering::Release);
    RECORD_TRIM.store(crate::model::trim::Trim::End as u8, Ordering::Release);
    PLAYBACK_INDICATOR.store(true, Ordering::Release);
    PLAYBACK_DELAY.store(0, Ordering::Release);
    for delay in &DELAY_OWED { delay.store(0, Ordering::Release); }
    RECORD_ROLE.store(ROLE_PLAYER, Ordering::Release);
    ASK_FIGHTER.store(true, Ordering::Release);
    LAST_ROLE.store(ROLE_PLAYER, Ordering::Release);
    CAP_ROLE.store(0, Ordering::Release);
    CAP_LEN.store(0, Ordering::Release);
    PLAYED.store(0, Ordering::Release);
    for count in &FED { count.store(0, Ordering::Release); }
    for generation in &RUN_GEN { generation.store(0, Ordering::Release); }
    for runs in &RUNS { runs.store(0, Ordering::Release); }
    RECORD_EDGE.store(0, Ordering::Release);
    PLAY_PLAYER.store(false, Ordering::Release);
    PLAY_CPU.store(false, Ordering::Release);
    RESET_CPU.store(false, Ordering::Release);
    MASH.store(false, Ordering::Release);
    PARRY.store(false, Ordering::Release);
    PARRY_PASSES.store(0, Ordering::Release);
    PARRY_OPEN.store(false, Ordering::Release);
    LOAD.store(false, Ordering::Release);
    CUT_PLAYER.store(false, Ordering::Release);
    CUT_CPU.store(false, Ordering::Release);
    MIGRATED.store(false, Ordering::Release);
    PLAYER_FRAMES.clear();
    CPU_FRAMES.clear();
    CURRENT_KIND.store(u32::MAX, Ordering::Release);
    RUN_PLAYER.store(false, Ordering::Release);
    RUN_CPU.store(false, Ordering::Release);
    *lock(&MACHINE) = None;
    *lock(&BOOK) = None;
    *lock(&MENU_EFFECT) = None;
    PREVIEW.store(0, Ordering::Release);
    *lock(&PREVIEW_ASK) = PreviewRequests::default();
    PREVIEW_AWAY.store(0, Ordering::Release);
    REFUSAL_SEEN.store(REFUSAL_SEQ.load(Ordering::Acquire), Ordering::Release);
    *lock(&PREVIEW_CLIP) = None;
    *lock(&STRIP) = (0, None, CaptureOwner::UNKNOWN);
    *lock(&MAKE) = None;
    *lock(&MAKE_ASK) = None;
    MAKE_ROLE.store(0, Ordering::Release);
    MAKE_CPU_FRAME.store(0, Ordering::Release);
    *lock(&HELD) = None;
    *lock(&ANSWER) = None;
    *lock(&LAST_ASK) = None;
    *lock(&LAST_DRILL) = None;
    *lock(&DRILL_ASK) = None;
    *lock(&PLAYER_LABEL) = String::new();
    DRILL_SEEN.store(false, Ordering::Release);
    DEFER_MIGRATE.store(false, Ordering::Release);
    CPU_ROLE_ON.store(false, Ordering::Release);
    WARP_HOLD.store(false, Ordering::Release);
    START_GATE.store(false, Ordering::Release);
    START_USED.store(RECORD_STARTS.read().0.seq, Ordering::Release);
    *START_SPEED.lock() = [None; 2];
    START_OWED.store(0, Ordering::Release);
    *CPU_DRIVE.lock() = CpuRoleDrive::idle();
    BOUND_DIRTY.store(true, Ordering::Release);
    *lock(&BOUND_APPLIED) = None;
    *lock(&SAVED_PAIR) = None;
    SLOT_RESYNC.store(false, Ordering::Release);
    for slot in ROLE_KIND.iter().chain(&LIVE_KIND) {
        slot.store(-1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::super::macros::StartState;
    use super::*;
    use crate::model::bridge_events::{Fed, FedSeen};
    use std::process::Command;
    use std::sync::Mutex as TestMutex;

    static TEST_LOCK: TestMutex<()> = TestMutex::new(());

    fn hold() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|err| err.into_inner())
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("tr-rr7-host-{tag}-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_recording_host_passes_its_selected_reaction_window() {
        use crate::model::combo::Kind;
        use crate::model::frame_strip::Button;
        let _hold = hold();
        for (delay, window, expected) in [
            (1, 0, Kind::Release(Button::Attack)), (1, 1, Kind::LetGo(Button::Attack)),
            (2, 1, Kind::Release(Button::Attack)), (2, 2, Kind::LetGo(Button::Attack)),
        ] {
            reset_host();
            CAP_ROLE.store(ROLE_PLAYER, Ordering::Release);
            begin_input(2);
            for frame in 1..=6 {
                let mut mapped = MappedInputs { buttons: if frame < 3 { 1 } else { 0 }, ..MappedInputs::NEUTRAL };
                unsafe { on_player_poll(&mut mapped); }
                let late = if frame == 3 + delay { Late { turned: Some(Some(1)), ..Late::NONE } }
                    else { Late::NONE };
                observe_input(Snap { buttons: mapped.buttons, ..Snap::empty(frame) }, late, window, false);
            }
            let take = converted_input(2).unwrap();
            let release = take.capture.combo.links().flat_map(crate::model::combo::LinkRec::events)
                .find(|event| matches!(event.kind, Kind::Release(_) | Kind::LetGo(_))).unwrap();
            assert_eq!(release.kind, expected, "delay {delay}, window {window}");
        }
        reset_host();
    }

    fn state(kind: i32) -> SavedState {
        let mut state = SavedState::never_saved();
        state.fighter_kind = kind;
        state
    }

    fn none() -> [Option<LegacySlot>; 5] {
        [None, None, None, None, None]
    }

    fn slot_macro(frames: Vec<InputFrame>, facing: f32, status: i32) -> MacroRecording {
        let mut start = StartState::neutral(7, 0, facing);
        start.status = status;
        MacroRecording::new("Slot", "13.0.5", start, frames).unwrap()
    }

    fn slot_playback(queued: bool) -> PlaybackState {
        cpu_playback(queued)
    }

    fn start_slot(slot: u8, facing: i8) -> Result<Option<PlayStart>, PlayRefusal> {
        slot_start_mash(slot, slot_take(slot).unwrap(), facing)
    }

    fn an_owed_slot_does_not_show_the_playback_indicator() {
        let _hold = hold();
        reset_host();
        SLOT_SEQ[0].store(1, Ordering::Release);
        assert_eq!(slot_start_ledge(0, 1, None), Ok(None));
        assert!(slot_owed());
        assert!(!SLOT_RUNNING.load(Ordering::Acquire));
        assert_eq!(playback_roles(), [false, false]);
        SLOT_SEQ[0].store(0, Ordering::Release);
        slot_stop();
    }

    fn a_playback_slot_run_plays_its_macro_through_the_cpu_pad() {
        let _hold = hold();
        reset_host();
        let frames = vec![
            InputFrame { buttons: 1, lstick_x: -80, ..InputFrame::default() },
            InputFrame { buttons: 2, lstick_x: 40, ..InputFrame::default() },
            InputFrame { buttons: 3, lstick_x: 90, ..InputFrame::default() },
        ];
        let macro_ = slot_macro(frames.clone(), -1.0, 31);
        set_slot(0, Some(&macro_)).unwrap();
        assert_eq!(slot_take(0).unwrap().status(), 31);
        assert_eq!(start_slot(0, 1), Ok(Some(PlayStart { slot: 0, frames: 3 })));
        assert_eq!(slot_phase(), (Phase::Playback, 3));
        assert!(slot_playback(true).running);
        for (i, frame) in frames.iter().enumerate() {
            assert_eq!(slot_control(false), Some(inject_values(to_mapped(*frame), -1)));
            assert_eq!(slot_post(), if i == 2 { Some(0) } else { None });
        }
        assert!(!slot_playback(true).running);
        assert_eq!(slot_phase(), (Phase::Idle, 0));
        assert_eq!(slot_post(), None);
    }

    fn a_play_on_cpu_frame_ends_a_slot_run() {
        let _hold = hold();
        reset_host();
        let macro_ = slot_macro(vec![InputFrame::default(); 3], 1.0, 7);
        set_slot(0, Some(&macro_)).unwrap();
        start_slot(0, 1).unwrap().unwrap();
        let frame = InputFrame { buttons: 42, ..InputFrame::default() };
        assert_eq!(cpu_controller(|| Some(frame)), Some(inject_values(to_mapped(frame), 1)));
        assert!(!slot_running());
        assert!(slot_control(false).is_none());
    }

    fn a_slot_waits_for_a_running_cpu_macro() {
        let _hold = hold();
        reset_host();
        assert!(cpu_idle());
        RUN_CPU.store(true, Ordering::Release);
        assert!(!cpu_idle());
        RUN_CPU.store(false, Ordering::Release);
        assert!(cpu_idle());
    }

    fn a_changed_slot_ends_its_run() {
        let _hold = hold();
        reset_host();
        let old = slot_macro(vec![InputFrame { buttons: 1, ..InputFrame::default() }; 3], 1.0, 7);
        let new = slot_macro(vec![InputFrame { buttons: 2, ..InputFrame::default() }; 3], -1.0, 9);
        set_slot(0, Some(&old)).unwrap();
        start_slot(0, 1).unwrap().unwrap();
        assert_eq!(slot_control(false).unwrap().buttons, 1);
        set_slot(0, Some(&new)).unwrap();
        assert!(slot_control(false).is_none());
        assert!(!slot_running());
    }

    fn an_empty_or_long_slot_is_refused() {
        let _hold = hold();
        reset_host();
        assert_eq!(start_slot(0, 1), Err(PlayRefusal::SlotEmpty));
        assert_eq!(start_slot(5, 1), Err(PlayRefusal::SlotOutOfRange));
        let old = slot_macro(vec![InputFrame { buttons: 6, ..InputFrame::default() }], 1.0, 7);
        let long = slot_macro(vec![InputFrame::default(); 601], -1.0, 9);
        set_slot(0, Some(&old)).unwrap();
        assert_eq!(set_slot(0, Some(&long)), Err(ImportRefusal::TooLong(601)));
        assert_eq!(set_slot(5, Some(&old)), Err(ImportRefusal::SlotOutOfRange));
        assert_eq!(start_slot(0, 1), Ok(Some(PlayStart { slot: 0, frames: 1 })));
        assert_eq!(slot_control(false).unwrap().buttons, 6);
    }

    fn the_r4b_holds_turn_three_times_in_a_slot() {
        let _hold = hold();
        reset_host();
        let frames = [-80, 80, -80, 80].into_iter()
            .flat_map(|x| std::iter::repeat_n(InputFrame { lstick_x: x, ..InputFrame::default() }, 60))
            .collect();
        let macro_ = slot_macro(frames, -1.0, 7);
        set_slot(0, Some(&macro_)).unwrap();
        start_slot(0, -1).unwrap().unwrap();
        let mut facing = -1;
        let mut turns = 0;
        for i in 0..240 {
            let sign = if slot_control(false).unwrap().stick_x < 0.0 { -1 } else { 1 };
            if sign != facing { turns += 1; facing = sign; }
            assert_eq!(slot_post(), if i == 239 { Some(0) } else { None });
        }
        assert_eq!(turns, 3);
    }

    fn a_slot_start_defers_while_its_slot_is_published() {
        let _hold = hold();
        reset_host();
        let old = slot_macro(vec![InputFrame { buttons: 1, lstick_x: 80, ..InputFrame::default() }], 1.0, 11);
        let new = slot_macro(vec![InputFrame { buttons: 2, lstick_x: 80, ..InputFrame::default() }], -1.0, 22);
        set_slot(0, Some(&old)).unwrap();
        let take = slot_take(0).unwrap();
        SLOT_SEQ[0].fetch_add(1, Ordering::AcqRel);
        assert_eq!(slot_start_mash(0, take, 1), Ok(None));
        assert_eq!(slot_take(0), None);
        assert!(!slot_running());
        SLOT_META[0].store(22 | (1_u64 << 32), Ordering::SeqCst);
        assert_eq!(SLOT_TAPES[0].frame_at(0), Some(old.frames[0]));
        assert_eq!(slot_start_mash(0, take, 1), Ok(None));
        assert_eq!(slot_take(0), None);
        set_slot_data(0, Some(&new)).unwrap();
        assert_eq!(slot_start_mash(0, take, 1), Ok(None));
        SLOT_SEQ[0].fetch_add(1, Ordering::Release);
        assert_eq!(slot_take(0).unwrap().status(), 22);
        assert_eq!(slot_start_mash(0, take, 1), Ok(None));
        assert_eq!(start_slot(0, 1), Ok(Some(PlayStart { slot: 0, frames: 1 })));
        assert_eq!(slot_control(false), Some(inject_values(to_mapped(new.frames[0]), -1)));
    }

    fn a_replaced_take_is_not_started_under_the_old_takes_gate() {
        use crate::model::cpu::ids::{C, Table};
        use crate::model::recording::{mash_should_start, term_for_status, MashCtx, RecorderSettings};
        let _hold = hold();
        reset_host();
        let table = Table::with(&[(C::StatusAttackAir, 40)]);
        let old = slot_macro(vec![InputFrame { buttons: 1, ..InputFrame::default() }], 1.0, 1);
        let new = slot_macro(vec![InputFrame { buttons: 2, lstick_x: 80, ..InputFrame::default() }], -1.0, 40);
        set_slot(0, Some(&old)).unwrap();
        let take = slot_take(0).unwrap();
        assert_eq!(term_for_status(&table, take.status()), None);
        let settings = RecorderSettings::factory();
        let gate = |term_enabled| mash_should_start(true, &settings, &MashCtx {
            head_slot: Some(0), in_hitstun: false, in_hitstop: false, term_enabled,
        });
        assert_eq!(gate(true), Some(0));
        set_slot(0, Some(&new)).unwrap();
        assert_eq!(slot_start_mash(0, take, 1), Ok(None));
        assert!(!slot_running());
        assert_eq!(slot_control(false), None);
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_take(0), None);
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        let take = slot_take(0).unwrap();
        assert_eq!(take.status(), 40);
        assert_eq!(term_for_status(&table, take.status()), Some(C::TermContAttackAir));
        assert_eq!(gate(false), None);
        assert!(!slot_running());
        assert_eq!(slot_control(false), None);
        assert_eq!(gate(true), Some(0));
        assert_eq!(slot_start_mash(0, take, 1), Ok(Some(PlayStart { slot: 0, frames: 1 })));
        assert_eq!(slot_control(false), Some(inject_values(to_mapped(new.frames[0]), -1)));
    }

    fn one_ledge_request_through_a_publication_starts_exactly_one_run_with_its_buffer() {
        let _hold = hold();
        reset_host();
        let frames = vec![
            InputFrame { buttons: 0x10, ..InputFrame::default() },
            InputFrame { buttons: 0x20, ..InputFrame::default() },
        ];
        let macro_ = slot_macro(frames, 1.0, 7);
        set_slot(0, Some(&macro_)).unwrap();
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_start_ledge(0, 1, Some(false)), Ok(None));
        assert!(slot_owed());
        assert!(slot_running() && cpu_playback(false).running && !cpu_idle());
        assert!(cpu_playback(false).owed);
        assert_eq!(slot_control(false), None);
        assert!(cpu_playback(false).owed);
        assert_eq!(take_slot_refusal(), None);
        set_slot_data(0, Some(&macro_)).unwrap();
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        for (at, buttons) in [0x10, 0x10, 0, 0, 0, 0x10, 0x20].into_iter().enumerate() {
            assert_eq!(slot_control(false).unwrap().buttons, buttons);
            assert!(!slot_owed());
            assert!(!cpu_playback(false).owed);
            assert_eq!(slot_post(), if at == 6 { Some(0) } else { None });
        }
        assert_eq!(slot_control(false), None);
        assert_eq!(slot_post(), None);
        assert!(!slot_running());
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_start_ledge(0, -1, None), Ok(None));
        assert!(cpu_playback(false).owed);
        assert!(slot_stop());
        assert!(!slot_owed());
        assert!(!cpu_playback(false).owed);
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_control(false), None);
        assert!(!slot_running());
        assert!(!cpu_playback(false).owed);
        assert_eq!(take_slot_refusal(), None);
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_start_ledge(0, 1, Some(true)), Ok(None));
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        for (at, buttons) in [0x10, 0, 0, 0, 0x10, 0x20].into_iter().enumerate() {
            assert_eq!(slot_control(false).unwrap().buttons, buttons);
            assert_eq!(slot_post(), if at == 5 { Some(0) } else { None });
        }
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_start_ledge(0, 1, None), Ok(None));
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_control(false).unwrap().buttons, 0x10);
        assert!(slot_stop());
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_start_ledge(0, 1, Some(true)), Ok(None));
        set_slot_data(0, None).unwrap();
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_control(false), None);
        assert!(!slot_running());
        assert!(!slot_owed());
        assert!(!cpu_playback(false).owed);
        assert_eq!(take_slot_refusal(), Some(PlayRefusal::SlotEmpty));
        assert_eq!(take_slot_refusal(), None);
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_start_ledge(0, 1, Some(false)), Ok(None));
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_control(false), None);
        slot_stop();
        assert_eq!(take_slot_refusal(), None);
    }

    fn a_deferred_ledge_request_denies_an_immediate_climb_query() {
        use crate::model::cpu::{facts::FrameFacts, ids::{C, Table}, ledge::{option, BUFFER_PLAYBACK_FRAME},
            role::FighterKey, settings::CpuSettings, state::Cpu, status::Situation};
        use crate::model::settings::Snapshot;
        let _hold = hold();
        reset_host();
        let mut settings = Snapshot::factory();
        let mut ledge = vec![0; 10];
        ledge[option::PLAYBACK_FIRST as usize] = 1;
        settings.set("ledge_state", &ledge).unwrap();
        let mut delay = vec![0; 31];
        delay[0] = 1;
        settings.set("ledge_delay", &delay).unwrap();
        let mut cpu = Cpu::new(CpuSettings::from_snapshot(&settings), 77);
        cpu.ids = Table::with(&[(C::StatusCliffWait, 41), (C::StatusCliffCatch, 40), (C::TermContCliffClimb, 7)]);
        let key = FighterKey::new(1, false).unwrap();
        cpu.fighters[key.index()].pending = 0;
        let request = cpu.frame(key, &FrameFacts { status: 41, prev_status: 40, situation: Situation::Cliff,
            motion_frame: BUFFER_PLAYBACK_FRAME, ..FrameFacts::default() }).playback.unwrap();
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
        assert_eq!(slot_start_ledge(request.slot, 1, Some(false)), Ok(None));
        assert!(!cpu.ledge_transition_term(key, 41, 7, true, slot_owed()));
        assert!(cpu.ledge_transition_term(key, 41, 7, true, false));
        // The AI frame caches the owed start; a cancel before the next one must release the climb at once.
        cpu.playback = cpu_playback(false);
        assert!(cpu.playback.owed);
        assert!(!cpu.ledge_transition_term(key, 41, 7, true, slot_owed()));
        slot_stop();
        assert!(cpu.playback.owed && !slot_owed());
        assert!(cpu.ledge_transition_term(key, 41, 7, true, slot_owed()));
        SLOT_SEQ[0].fetch_add(1, Ordering::SeqCst);
    }

    fn a_late_credit_of_the_old_run_is_not_counted_for_the_new_one() {
        let _hold = hold();
        for role in [Role::Player, Role::Cpu] {
            reset_host();
            let root = scratch("fed-generation");
            tick(&root);
            let frames = (1..=5).map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
            let kind = if role == Role::Player { 8 } else { 7 };
            let recording = MacroRecording::new("Fed", "13.0.5", StartState::neutral(kind, 0, 1.0), frames).unwrap();
            let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
            let take = || if role == Role::Player { take_player_frame() } else { take_cpu_frame() };
            request_preview(&id, role, 0, 3);
            tick(&root);
            assert_eq!(fed_frames(role), 0);
            assert_eq!(take().map(|frame| frame.buttons), Some(1));
            assert_eq!(fed_frames(role), 1);
            let (old, generation) = pop_frame(role, |_| {}).unwrap();
            assert_eq!(old.buttons, 2);
            request_preview(&id, role, 2, 5);
            tick(&root);
            assert_eq!(fed_frames(role), 0);
            credit_frame(role, generation, || {});
            assert_eq!(fed_frames(role), 0);
            for (count, buttons) in (1..=3).zip(3..=5) {
                assert_eq!(take().map(|frame| frame.buttons), Some(buttons));
                assert_eq!(fed_frames(role), count);
            }
            assert!(take().is_none());
            tick(&root);
            assert_eq!(fed_frames(role), 3);
        }
    }

    fn a_roles_fed_count_is_the_frames_its_hook_took() {
        let _hold = hold();
        for role in [Role::Player, Role::Cpu] {
            reset_host();
            let other = if role == Role::Player { Role::Cpu } else { Role::Player };
            let slot = slot_macro(vec![InputFrame { buttons: 9, ..InputFrame::default() }], 1.0, 1);
            set_slot(0, Some(&slot)).unwrap();
            start_slot(0, 1).unwrap().unwrap();
            assert_eq!(slot_control(false).unwrap().buttons, 9);
            assert_eq!(slot_post(), Some(0));
            assert_eq!(fed_frames(role), 0);
            assert_eq!(fed_frames(other), 0);

            let root = scratch("fed-frames");
            tick(&root);
            let frames = (1..=5).map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
            let kind = if role == Role::Player { 8 } else { 7 };
            let recording = MacroRecording::new("Fed", "13.0.5", StartState::neutral(kind, 0, 1.0), frames).unwrap();
            let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
            let take = || if role == Role::Player { take_player_frame() } else { take_cpu_frame() };

            request_preview(&id, role, 0, 5);
            tick(&root);
            assert_eq!(fed_frames(role), 0);
            for expected in 1..=3 {
                assert_eq!(take().map(|frame| frame.buttons), Some(expected));
                assert_eq!(fed_frames(role), expected);
                assert_eq!(fed_frames(other), 0);
            }
            for expected in 4..=5 {
                assert_eq!(take().map(|frame| frame.buttons), Some(expected));
            }
            assert_eq!(fed_frames(role), 5);
            assert!(take().is_none());
            tick(&root);
            assert!(!preview_running());
            assert!(take().is_none());
            assert_eq!(fed_frames(role), 5);

            request_preview(&id, role, 1, 4);
            tick(&root);
            assert_eq!(fed_frames(role), 0);
            for expected in 2..=4 {
                assert_eq!(take().map(|frame| frame.buttons), Some(expected));
            }
            assert_eq!(fed_frames(role), 3);
            tick(&root);
            assert!(!preview_running());
            assert!(take().is_none());
            assert_eq!(fed_frames(role), 3);

            request_preview(&id, role, 0, 5);
            tick(&root);
            assert_eq!(fed_frames(role), 0);
            assert_eq!(take().map(|frame| frame.buttons), Some(1));
            assert_eq!(fed_frames(role), 1);
            stop_preview();
            tick(&root);
            assert!(!preview_running());
            assert!(take().is_none());
            assert_eq!(fed_frames(role), 1);
            assert_eq!(fed_frames(other), 0);

            if role == Role::Player {
                RecordingIndex::update(&root, |index| { index.set_active(role, Some(&id), 1)?; Ok(true) }).unwrap();
                on_chords(false, true, false);
                tick(&root);
                assert_eq!(fed_frames(role), 0);
                for expected in 1..=5 {
                    assert_eq!(take().map(|frame| frame.buttons), Some(expected));
                    assert_eq!(fed_frames(role), expected);
                }
                assert!(take().is_none());
                assert_eq!(fed_frames(role), 5);
            }
        }
    }

    fn a_named_load_is_taken_with_save_states_off() {
        let calls = core::cell::Cell::new(0);
        let take = |result| {
            calls.set(calls.get() + 1);
            result
        };
        assert!(save_state_step(false, false, || take(true)));
        assert!(!save_state_step(false, false, || take(false)));
        assert!(save_state_step(true, false, || take(false)));
        assert!(save_state_step(false, true, || take(false)));
        assert_eq!(calls.get(), 4);
    }

    /// A preview feeds only the selected range to its role after the start save lands.
    fn a_crop_preview_plays_its_window_on_the_role_through_the_play_queue() {
        linked_crop_preview(true);
    }

    fn a_linked_crop_preview_starts_with_save_states_off() {
        linked_crop_preview(false);
    }

    fn land_pair(pair: [SavedState; 2], enabled: bool) -> bool {
        use crate::model::cpu::rng::Rng;
        use crate::model::state::machine::{FrameIn, Machine, SituationIds, StatusNow};
        use crate::model::state::settings::StateSettings;

        let mut posted = Some(pair);
        let mut restore = Machine::default();
        let settings = StateSettings { enable: enabled, ..StateSettings::default() };
        let mut rng = Rng::new(7);
        let mut restored = false;
        for status in [StatusNow::Other, StatusNow::Wait] {
            for entry in [0, 1] {
                let temporary = restore.temporary_active();
                if fighter_frame(None, save_state_step(enabled, temporary, || {
                    posted.take().is_some_and(|pair| restore.begin_temporary(pair))
                })) {
                    let input = FrameIn {
                        entry, nana: false, popo: false, kind: if entry == 0 { 8 } else { 7 },
                        dead: false, status, prev_rebirth: false, standby: false, stage_id: 0x40,
                        situation: SituationIds { ground: 0, air: 2, cliff: 1 },
                        save_chord: false, load_chord: false, suspend_autoload: false,
                        kirby: false, pokemon: false, buffable: false,
                    };
                    if restore.frame(&input, &settings, &mut rng).restored.is_some() {
                        assert!(!restored);
                        restored = true;
                        warp_landed();
                    }
                }
            }
        }
        restored && posted.is_none() && !restore.temporary_active()
    }

    fn linked_crop_preview(enabled: bool) {
        let _hold = hold();
        for role in [Role::Player, Role::Cpu] {
            reset_host();
            let root = scratch("preview-role");
            service_at(&root, &none(), "2026-11-23", state(8), state(7));
            let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR))
                .save_new(&NamedSave::new("Start", state(8), state(7)).unwrap()).unwrap();
            let frames = (1..=6).map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
            let kind = if role == Role::Player { 8 } else { 7 };
            let mut recording = super::super::macros::MacroRecording::new("Take", "13.0.5", StartState::neutral(kind, 0, 1.0), frames).unwrap();
            recording.set_start_save(Some(&save_id)).unwrap();
            recording.set_start_save_on(true);
            let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
            let index = RecordingIndex::load(&root).unwrap();
            assert!(index.active(Role::Player).is_none() && index.active(Role::Cpu).is_none());
            request_preview(&id, role, 1, 4);
            assert!(preview_running() && pending());
            let tick = service_at(&root, &none(), "2026-11-23", state(8), state(7));
            assert_eq!(tick.warp.as_deref(), Some(save_id.as_str()));
            assert_eq!(tick.warp_place.map(|p| p.0), Some(role));
            assert!(take_player_frame().is_none() && take_cpu_frame().is_none());
            let save = Store::<NamedSave>::new(root.join(SAVES_DIR)).load(&save_id).unwrap();
            let restored = land_pair([save.player, save.cpu], enabled);
            let mut taken = Vec::new();
            for _ in 0..6 {
                service_at(&root, &none(), "2026-11-23", state(8), state(7));
                let (frame, other) = if role == Role::Player {
                    (take_player_frame(), take_cpu_frame())
                } else {
                    (take_cpu_frame(), take_player_frame())
                };
                assert!(other.is_none());
                match frame {
                    Some(frame) => taken.push(frame.buttons),
                    None => break,
                }
            }
            assert_eq!(taken, [2, 3, 4]);
            assert!(restored);
            service_at(&root, &none(), "2026-11-23", state(8), state(7));
            assert!(!preview_running());
            let index = RecordingIndex::load(&root).unwrap();
            assert!(index.active(Role::Player).is_none() && index.active(Role::Cpu).is_none());
        }
    }

    fn an_unreadable_index_refuses_a_new_preview_but_does_not_prevent_a_vault_return() {
        let _hold = hold();
        reset_host();
        let root = scratch("preview-bad-index");
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        let id = one_macro(&root, "Real take", 7, 7, None);
        std::fs::write(root.join(super::super::library::INDEX_FILE), "{").unwrap();
        let reads = store::index_reads();
        request_preview(&id, Role::Cpu, 0, 1);
        assert!(preview_running());
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert_eq!(store::index_reads(), reads + 1);
        assert!(!preview_running() && lock(&PREVIEW_ASK).start.is_none());
        assert!(take_cpu_frame().is_none());

        reset_host();
        let root = scratch("vault-preview-bad-index");
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        let clip = MacroRecording::new("Clip", "13.0.5", StartState::neutral(7, 0, 1.0), vec![InputFrame::default(); 5]).unwrap();
        // Exclude mod writes from parallel library tests: the only change here is external.
        let _no_mod_write = store::recording_write_guard(&root).unwrap();
        request_clip_preview(clip, Role::Cpu);
        let tick = service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(preview_away());
        assert_eq!(tick.home, Some(HomeWarp::Start { air: false, player: false }));
        assert!(land_pair([state(8), state(7)], false));
        std::fs::write(root.join(super::super::library::INDEX_FILE), "{").unwrap();
        let reads = store::index_reads();
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(take_cpu_frame().is_some() && !CPU_FRAMES.is_empty());
        stop_preview();
        let returning = service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert_eq!(returning.home, Some(HomeWarp::Return));
        assert!(!preview_running() && preview_away() && CPU_FRAMES.is_empty());
        assert!(take_cpu_frame().is_none());
        assert!(land_pair([state(8), state(7)], false));
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(!previewing() && !preview_away());
        assert_eq!(store::index_reads(), reads);
    }

    fn a_session_end_drops_the_preview_home() {
        let _hold = hold();
        reset_host();
        let root = scratch("vault-preview-session-end");
        let mut player = state(8); player.x = 10.0;
        let mut cpu = state(7); cpu.x = 30.0;
        service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
        let clip = MacroRecording::new("Clip", "13.0.5", StartState::neutral(7, 0, 1.0), vec![InputFrame::default(); 5]).unwrap();
        request_clip_preview(clip.clone(), Role::Cpu);
        let tick = service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
        assert_eq!(tick.home, Some(HomeWarp::Start { air: false, player: false }));
        assert!(land_pair([player.clone(), cpu.clone()], false));
        service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
        assert!(take_cpu_frame().is_some());
        drop_preview_home();
        assert!(!preview_away() && preview_running());
        stop_preview();
        let tick = service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
        assert!(tick.home.is_none());
        assert!(!previewing() && !pending());

        player.x = 90.0; cpu.x = 99.0;
        request_clip_preview(clip, Role::Cpu);
        let tick = service_at(&root, &none(), "2026-11-23", player, cpu);
        assert_eq!(tick.home, Some(HomeWarp::Start { air: false, player: false }));
    }

    fn a_crop_preview_owes_the_start_speed_as_play_does() {
        let _hold = hold();
        for role in [Role::Player, Role::Cpu] {
            reset_host();
            let root = scratch("preview-speed");
            service_at(&root, &none(), "2026-11-23", state(8), state(7));
            let save = NamedSave::new("Start", state(8), state(7)).unwrap();
            let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&save).unwrap();
            let kind = if role == Role::Player { 8 } else { 7 };
            let start = StartState { speed_x: 2.0, speed_y: -1.0, ..StartState::neutral(kind, 0, -1.0) };
            let mut recording = MacroRecording::new("Run", "13.0.5", start, vec![InputFrame::default(); 3]).unwrap();
            recording.set_start_save(Some(&save_id)).unwrap();
            recording.set_start_save_on(true);
            let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
            let mut index = RecordingIndex::load(&root).unwrap();
            index.set_active(role, Some(&id), 1).unwrap();
            index.save(&root).unwrap();
            on_chords(false, role == Role::Player, role == Role::Cpu);
            let tick = service_at(&root, &none(), "2026-11-23", state(8), state(7));
            assert_eq!(tick.warp.as_deref(), Some(save_id.as_str()));
            assert!(land_pair([save.player.clone(), save.cpu.clone()], true));
            service_at(&root, &none(), "2026-11-23", state(8), state(7));
            let ordinary = take_start_speed(role);
            assert_eq!(ordinary, Some(StartSpeed { speed: (2.0, -1.0), facing: -1.0 }));
            reset_host();
            request_preview(&id, role, 0, 3);
            let tick = service_at(&root, &none(), "2026-11-23", state(8), state(7));
            assert_eq!(tick.warp.as_deref(), Some(save_id.as_str()));
            assert!(land_pair([save.player, save.cpu], true));
            service_at(&root, &none(), "2026-11-23", state(8), state(7));
            assert_eq!(take_start_speed(role), ordinary);
        }
    }

    fn a_vault_clip_preview_plays_on_the_cpu_and_puts_both_fighters_back() {
        let _hold = hold();
        reset_host();
        let root = scratch("vault-preview");
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        let start = StartState { speed_x: 1.5, speed_y: 0.0, ..StartState::neutral(7, 0, -1.0) };
        let frames: Vec<_> = (1..=5).map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
        let clip = MacroRecording::new("Clip", "13.0.5", start, frames.clone()).unwrap();
        let mut player = state(8); player.x = 10.0;
        let mut cpu = state(7); cpu.x = 30.0;
        let home = [player.clone(), cpu.clone()];
        request_clip_preview(clip.clone(), Role::Cpu);
        assert!(previewing());
        let tick = service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
        assert!(previewing());
        assert!(tick.warp.is_none());
        assert_eq!(tick.home, Some(HomeWarp::Start { air: false, player: false }));
        assert!(take_player_frame().is_none() && take_cpu_frame().is_none());
        assert!(land_pair(home.clone(), false));
        let mut received = Vec::new();
        for _ in 0..6 {
            service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
            assert!(take_player_frame().is_none());
            if let Some(frame) = take_cpu_frame() { received.push(frame); }
        }
        assert_eq!(received, frames);
        assert_eq!(take_start_speed(Role::Cpu), Some(StartSpeed { speed: (1.5, 0.0), facing: -1.0 }));
        service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
        assert!(!preview_running() && preview_away() && previewing());
        player.x = 90.0; cpu.x = 99.0;
        request_clip_preview(clip.clone(), Role::Cpu);
        let tick = service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
        assert_eq!(tick.home, Some(HomeWarp::Start { air: false, player: false }));
        assert!(land_pair(home.clone(), false));
        stop_preview();
        let tick = service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
        assert_eq!(tick.home, Some(HomeWarp::Return));
        assert!(preview_away());
        assert!(previewing());
        stop_preview();
        let tick = service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
        assert_eq!(tick.home, None);
        assert!(land_pair(home, false));
        assert!(previewing());
        service_at(&root, &none(), "2026-11-23", player.clone(), cpu.clone());
        assert!(!preview_away() && !previewing() && !pending());

        let mut air_clip = clip;
        air_clip.start.situation = SITUATION_AIR;
        request_clip_preview(air_clip, Role::Cpu);
        let tick = service_at(&root, &none(), "2026-11-23", player, cpu);
        assert_eq!(tick.home, Some(HomeWarp::Start { air: true, player: false }));
    }

    fn a_refused_vault_preview_says_why_and_ends() {
        let _hold = hold();
        reset_host();
        let root = scratch("vault-preview-refused");
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        let clip = MacroRecording::new("Clip", "13.0.5", StartState::neutral(7, 0, 1.0), vec![InputFrame::default(); 5]).unwrap();
        request_clip_preview(clip, Role::Cpu);
        let tick = service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert_eq!(tick.home, Some(HomeWarp::Start { air: false, player: false }));
        assert!(take_cpu_frame().is_none());
        let text = "Let the CPU settle to idle and leave the ledge before previewing";
        preview_refused(text);
        let tick = service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(tick.notices.contains(&text));
        assert!(take_cpu_frame().is_none());
        assert!(!previewing() && !pending());

        let second = "Second notice";
        preview_refused(second);
        assert!(pending());
        let tick = service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(tick.notices.contains(&second));
        assert!(take_cpu_frame().is_none());
        assert!(!previewing() && !pending());
        let tick = service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(tick.notices.is_empty());

        reset_host();
        let root = scratch("vault-preview-refused-bad-index");
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        let clip = MacroRecording::new("Clip", "13.0.5", StartState::neutral(7, 0, 1.0), vec![InputFrame::default(); 5]).unwrap();
        request_clip_preview(clip, Role::Cpu);
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        preview_refused(text);
        std::fs::write(root.join(super::super::library::INDEX_FILE), "{").unwrap();
        let tick = service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(tick.notices.contains(&text));
        assert!(take_cpu_frame().is_none());
        assert!(!previewing() && !pending());
    }

    fn a_cpu_crop_preview_holds_right_whatever_the_facing() {
        let _hold = hold();
        for start in [0, 6] {
            reset_host();
            let root = scratch("preview-right");
            service_at(&root, &none(), "2026-11-23", state(8), state(7));
            let frame = InputFrame { lstick_x: 80, ..InputFrame::default() };
            let recording = MacroRecording::new("Right", "13.0.5", StartState::neutral(7, 0, -1.0), vec![frame; 63]).unwrap();
            let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
            request_preview(&id, Role::Cpu, start, 63);
            let expected = playback_controller(frame).stick_x;
            assert!(expected > 0.0);
            for _ in start..63 {
                service_at(&root, &none(), "2026-11-23", state(8), state(7));
                assert_eq!(cpu_controller(take_cpu_frame).unwrap().stick_x, expected);
                assert!(take_player_frame().is_none());
            }
        }
    }

    /// Stopping a preview discards its unplayed inputs.
    fn stopping_a_crop_preview_drops_the_rest() {
        let _hold = hold();
        reset_host();
        let root = scratch("preview-stop");
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        let frames = (1..=6).map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
        let recording = super::super::macros::MacroRecording::new("Take", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        request_preview(&id, Role::Cpu, 0, 6);
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert_eq!(take_cpu_frame().map(|frame| frame.buttons), Some(1));
        stop_preview();
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(take_cpu_frame().is_none());
        assert!(!preview_running());
    }

    /// A missing preview source finishes without a warp.
    fn a_crop_preview_of_a_missing_macro_ends_at_once() {
        let _hold = hold();
        reset_host();
        let root = scratch("preview-missing");
        request_preview("nope", Role::Player, 0, 3);
        let tick = service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(!preview_running());
        assert!(tick.warp.is_none());
    }

    fn posted_chords_reach_the_role_machines() {
        let _hold = hold();
        reset_host();
        let root = scratch("play");
        let player = state(8);
        let cpu = state(7);
        let _ = service_at(
            &root,
            &none(),
            "2026-11-06",
            player.clone(),
            cpu.clone(),
        );
        let save = NamedSave::new("Nair start", player.clone(), cpu.clone()).unwrap();
        let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR))
            .save_new(&save)
            .unwrap();
        let mut recording = super::super::macros::MacroRecording::new(
            "Nair",
            "13.0.5",
            StartState::neutral(8, 0, 1.0),
            vec![
                InputFrame {
                    buttons: 4,
                    ..InputFrame::default()
                },
                InputFrame {
                    buttons: 5,
                    ..InputFrame::default()
                },
                InputFrame {
                    buttons: 6,
                    ..InputFrame::default()
                },
            ],
        )
        .unwrap();
        recording.set_start_save(Some(&save_id)).unwrap();
        let id = Library::new(root.join(MACROS_DIR))
            .save_new(&recording)
            .unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&id), 1).unwrap();
        index.save(&root).unwrap();
        on_chords(false, true, false);
        let tick = service_at(&root, &none(), "2026-11-06", player, cpu);
        assert_eq!(tick.warp.as_deref(), Some(save_id.as_str()));
        assert!(tick
            .diags
            .iter()
            .any(|line| line == "screen=playback role=player warp=1"));
        assert!(take_player_frame().is_none(), "no frame before the warp's restore lands");
        warp_landed();
        let _ = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert_eq!(take_player_frame().expect("player frame").buttons, 4);
        let tick = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert_eq!(take_player_frame().expect("next frame").buttons, 5);
        assert!(tick.diags.iter().all(|line| !line.contains("warp=1")));
        note_mash();
        let tick = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(tick
            .diags
            .iter()
            .any(|line| line == "screen=playback mash=1"));
        // The CPU's mash stops only the CPU's macro: Player 1's keeps playing (primary 2026-11-08 13:05Z, #22 (a)).
        assert_eq!(take_player_frame().expect("Player 1 plays on after the CPU's mash").buttons, 6);
    }

    fn a_posted_record_stores_the_start_save() {
        let _hold = hold();
        reset_host();
        let root = scratch("rec");
        on_chords(true, false, false);
        push_capture(
            ROLE_PLAYER,
            InputFrame {
                buttons: 4,
                ..InputFrame::default()
            },
        );
        on_chords(true, false, false);
        let _ = service_at(&root, &none(), "2026-11-06", state(13), state(7));
        let macros = Library::new(root.join(MACROS_DIR));
        let listed = macros.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let id = listed.entries[0].id.clone();
        let saved = macros.load(&id).unwrap();
        assert!(saved.start_save.is_some());
        assert_eq!(saved.frames[0].buttons, 4);
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.active(Role::Player), Some(id.as_str()));
        assert_eq!(index.active(Role::Cpu), None);
    }

    fn migration_from_the_worker_runs_once() {
        let _hold = hold();
        reset_host();
        let root = scratch("mig");
        let mut slots = none();
        slots[0] = Some(LegacySlot {
            frames: vec![InputFrame {
                buttons: 1,
                ..InputFrame::default()
            }],
            start: StartState::neutral(3, 0, 1.0),
        });
        let first = service_at(&root, &slots, "2026-11-06", state(1), state(1));
        assert!(first
            .diags
            .iter()
            .any(|line| line == "screen=migrate once=1"));
        assert!(RecordingIndex::load(&root).unwrap().migrated());
        assert_eq!(
            Library::new(root.join(MACROS_DIR))
                .list()
                .unwrap()
                .entries
                .len(),
            1
        );
        slots[1] = Some(LegacySlot {
            frames: vec![InputFrame {
                buttons: 2,
                ..InputFrame::default()
            }],
            start: StartState::neutral(4, 0, 1.0),
        });
        let second = service_at(&root, &slots, "2026-11-06", state(1), state(1));
        assert!(second
            .diags
            .iter()
            .all(|line| line != "screen=migrate once=1"));
        assert_eq!(
            Library::new(root.join(MACROS_DIR))
                .list()
                .unwrap()
                .entries
                .len(),
            1
        );
    }

    fn a_posted_menu_effect_reaches_the_worker() {
        let _hold = hold();
        reset_host();
        let root = scratch("posted");
        let _ = service_at(&root, &none(), "2026-11-06", state(1), state(1));
        let id = super::super::rr_runtime::save_chord(&root, state(3), state(4), 1).unwrap();
        post_effect(Effect::DuplicateSave { source: id.clone() });
        let _ = service_at(&root, &none(), "2026-11-06", state(3), state(4));
        let listed = Store::<NamedSave>::new(root.join(SAVES_DIR)).list().unwrap();
        assert_eq!(listed.entries.len(), 2);
        assert_eq!(RecordingIndex::load(&root).unwrap().bound(), Some(id.as_str()));
        let recording = super::super::macros::MacroRecording::new(
            "Nair",
            "13.0.5",
            StartState::neutral(8, 0, 1.0),
            vec![
                InputFrame { buttons: 1, ..InputFrame::default() },
                InputFrame { buttons: 2, ..InputFrame::default() },
            ],
        )
        .unwrap();
        let macro_id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        post_effect(Effect::OpenCrop { source: macro_id.clone(), combo: false, player: false });
        let tick = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(tick.diags.iter().all(|line| !line.starts_with("screen=crop source=")), "opening the editor never writes a file");
        assert_eq!(Library::new(root.join(MACROS_DIR)).list().unwrap().entries.len(), 1);
        assert_eq!(Library::new(root.join(MACROS_DIR)).load(&macro_id).unwrap().frames, recording.frames);
        post_effect(Effect::Keyboard { header: "Name macro", draft: "Nair".into() });
        let named = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert_eq!(named.keyboard.as_ref().map(|offer| offer.header), Some("Name macro"));
        post_effect(Effect::DeleteSave { source: id });
        let _ = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(RecordingIndex::load(&root).unwrap().bound().is_none());
    }

    fn an_empty_posted_record_keeps_the_empty_notice() {
        let _hold = hold();
        reset_host();
        let root = scratch("empty");
        on_chords(true, false, false);
        on_chords(true, false, false);
        let tick = service_at(&root, &none(), "2026-11-06", state(1), state(1));
        assert!(Library::new(root.join(MACROS_DIR))
            .list()
            .unwrap()
            .entries
            .is_empty());
        assert!(tick
            .notices
            .contains(&super::super::rr_runtime::EMPTY_RECORDING));
    }

    fn the_cpu_inject_slot_starts_empty() {
        let _hold = hold();
        reset_host();
        assert_eq!(capture_role(), 0);
        assert!(take_cpu_frame().is_none());
    }

    fn one_macro(root: &std::path::Path, name: &str, buttons: u32, kind: i32, save: Option<&str>) -> String {
        let mut recording = super::super::macros::MacroRecording::new(
            name,
            "13.0.5",
            StartState::neutral(kind, 0, 1.0),
            vec![InputFrame { buttons, ..InputFrame::default() }],
        )
        .unwrap();
        if let Some(id) = save {
            recording.set_start_save(Some(id)).unwrap();
        }
        Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap()
    }

    fn sample_combo() -> crate::model::combo::Combo {
        crate::model::combo::Combo::from_links([crate::model::combo::LinkRec::presses(
            crate::model::frame_strip::Move::Aerial,
            crate::model::frame_strip::Move::Aerial,
            crate::model::frame_strip::Verdict::True(None),
            None,
            &[],
        )])
        .unwrap()
    }

    fn assert_cpu_capture_saved(root: &Path) {
        let start = service_at(root, &none(), "2026-11-06", state(13), state(7));
        assert!(start.diags.iter().any(|line| line == "screen=recording role=cpu recording=1"));
        push_capture(ROLE_CPU, InputFrame { buttons: 9, ..InputFrame::default() });
        push_capture(ROLE_PLAYER, InputFrame { buttons: 1, ..InputFrame::default() });
        on_chords(true, false, false);
        let stop = service_at(root, &none(), "2026-11-06", state(13), state(7));
        assert!(stop.diags.iter().any(|line| line == "screen=recording role=cpu recording=0"));
        let macros = Library::new(root.join(MACROS_DIR));
        let listed = macros.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let saved = macros.load(&listed.entries[0].id).unwrap();
        assert_eq!(saved.frames[0].buttons, 9);
        let save_id = saved.start_save.expect("start save");
        let save = Store::<NamedSave>::new(root.join(SAVES_DIR)).load(&save_id).unwrap();
        assert_eq!(save.cpu.fighter_kind, 7);
        assert!(crate::engine::rr_runtime::load_catalog(root).unwrap().macros.iter().any(|file| file.id == listed.entries[0].id));
    }

    fn record_follows_the_published_cpu_role() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-role");
        set_record_role(Role::Cpu);
        on_chords(true, false, false);
        assert_cpu_capture_saved(&root);
    }

    fn a_cold_start_records_on_the_saved_popup_role() {
        let _hold = hold();
        reset_host();
        let root = scratch("cold-role");
        let loaded = crate::model::settings::parse(r#"{"menu":{},"defaults_menu":{},"recording_roles":{"popup":"cpu","page":"player"}}"#);
        adopt_saved_role(&loaded.settings);
        assert_eq!(capture_role_key(), "");
        on_chords(true, false, false);
        assert_eq!(capture_role(), ROLE_CPU);
        assert_eq!(capture_role_key(), "cpu");
        assert_cpu_capture_saved(&root);
        assert_eq!(capture_role_key(), "");

        reset_host();
        adopt_saved_role(&loaded.settings);
        let old = crate::model::settings::parse(r#"{"menu":{},"defaults_menu":{}}"#);
        adopt_saved_role(&old.settings);
        on_chords(true, false, false);
        assert_eq!(capture_role(), ROLE_PLAYER);
        assert_eq!(capture_role_key(), "player");
    }

    fn the_open_take_is_the_capture_role() {
        let _hold = hold();
        reset_host();
        assert_eq!(capture_take(), None);
        on_chords(true, false, false);
        assert_eq!(capture_take(), Some(Take::Player));
        assert_eq!(capture_take().map_or("", Take::key), capture_role_key());
        on_chords(true, false, false);
        assert_eq!(capture_take(), None);
        reset_host();
        set_record_role(Role::Cpu);
        on_chords(true, false, false);
        assert_eq!(capture_take(), Some(Take::Cpu));
        assert_eq!(capture_take().map_or("", Take::key), capture_role_key());
        reset_host();
    }

    fn player_one_is_tinted_only_during_its_own_take() {
        let _hold = hold();
        reset_host();
        assert_eq!(player_tint(), None);
        on_chords(true, false, false);
        assert_eq!(player_tint(), Some(crate::model::recording::PLAYER_TINT));
        on_chords(true, false, false);
        assert_eq!(player_tint(), None);
        reset_host();
        set_record_role(Role::Cpu);
        on_chords(true, false, false);
        assert_eq!(capture_take(), Some(Take::Cpu));
        assert_eq!(player_tint(), None);
        reset_host();
    }

    fn press(buttons: u32) -> MappedInputs {
        MappedInputs { buttons, ..MappedInputs::NEUTRAL }
    }

    fn cpu_menu_role() {
        let mut screens = crate::model::recording_menu::RecordingScreens::new();
        screens.toggle_popup();
        screens.press_popup(crate::model::menu::input::Button::ZR);
        assert_eq!(screens.selection().record_role, ViewRole::Cpu);
        publish_menu_role(screens.selection().record_role);
    }

    fn player_menu_role() {
        let mut screens = crate::model::recording_menu::RecordingScreens::new();
        screens.toggle_popup();
        screens.press_popup(crate::model::menu::input::Button::ZR);
        screens.press_popup(crate::model::menu::input::Button::ZR);
        assert_eq!(screens.selection().record_role, ViewRole::Player);
        publish_menu_role(screens.selection().record_role);
    }

    fn drive_cpu(buttons: u32) -> Option<u32> {
        let mut out = press(buttons);
        assert!(cpu_role_poll(&mut out));
        assert_eq!(out, MappedInputs::NEUTRAL);
        cpu_role_control().write.map(|write| write.buttons)
    }

    fn f4_cpu_role_record_write_matches_playback_of_the_stored_frame() {
        let _hold = hold();
        reset_host();
        let root = scratch("f4-cpu-stick");
        cpu_menu_role();
        on_chords(true, false, false);
        cpu_role_after_chord();
        let _ = service_at(&root, &none(), "2026-11-07", state(13), state(7));
        for _ in 0..crate::model::recording::LOCKOUT_FRAMES {
            assert!(drive_cpu(1).is_none());
        }
        assert_eq!(cpu_role_phase(), Some(Phase::Standby));
        assert_eq!(drive_cpu(0x8000), Some(0x8000));
        let input = MappedInputs { buttons: 42, lstick_x: 80, ..MappedInputs::NEUTRAL };
        let mut player = input;
        assert!(cpu_role_poll(&mut player));
        assert_eq!(player, MappedInputs::NEUTRAL);
        assert_eq!(CAP_LEN.load(Ordering::Acquire), 1);
        let stored = unpack(CAP_FRAMES[0].load(Ordering::Acquire));
        assert_eq!(stored, to_frame(input));
        let write = cpu_role_control().write.unwrap();
        assert_eq!(write.stick_x, playback_controller(stored).stick_x);
    }

    /// A take with the role set to CPU while an AI writes Player 1's record: the record the poll is handed drives the
    /// CPU and fills the take, and Player 1 is left neutral, as for a human.
    fn a_cpu_role_take_forwards_the_given_record_to_the_cpu_and_leaves_player_one_neutral() {
        let _hold = hold();
        reset_host();
        let root = scratch("ai-cpu-take");
        cpu_menu_role();
        on_chords(true, false, false);
        cpu_role_after_chord();
        let _ = service_at(&root, &none(), "2026-11-25", state(13), state(7));
        for _ in 0..crate::model::recording::LOCKOUT_FRAMES {
            assert!(drive_cpu(1).is_none());
        }
        let given = [(42u32, 80i8, -30i8), (7, -100, 100), (36_872, 0, 0)];
        for (index, (buttons, x, y)) in given.into_iter().enumerate() {
            let input = MappedInputs { buttons, lstick_x: x, lstick_y: y, ..MappedInputs::NEUTRAL };
            let mut player = input;
            assert!(cpu_role_poll(&mut player));
            assert_eq!(player, MappedInputs::NEUTRAL, "Player 1 stands neutral");
            let write = cpu_role_control().write.expect("the CPU is driven");
            let expected = playback_controller(to_frame(input));
            assert_eq!((write.buttons, write.stick_x), (expected.buttons, expected.stick_x));
            assert_eq!(unpack(CAP_FRAMES[index].load(Ordering::Acquire)), to_frame(input));
        }
        assert_eq!(CAP_LEN.load(Ordering::Acquire), 3);
        on_chords(true, false, false);
        cpu_role_after_chord();
        let _ = service_at(&root, &none(), "2026-11-25", state(13), state(7));
        let macros = Library::new(root.join(MACROS_DIR));
        let listed = macros.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let saved = macros.load(&listed.entries[0].id).unwrap();
        let stored: Vec<_> = saved.frames.iter().map(|f| (f.buttons, f.lstick_x, f.lstick_y)).collect();
        assert_eq!(stored, given);
    }

    fn cpu_role_lockout_or_standby_press_is_not_stored() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-lockout");
        cpu_menu_role();
        on_chords(true, false, false);
        cpu_role_after_chord();
        let _ = service_at(&root, &none(), "2026-11-07", state(13), state(7));
        for _ in 0..crate::model::recording::LOCKOUT_FRAMES {
            assert!(drive_cpu(1).is_none());
        }
        assert_eq!(cpu_role_phase(), Some(Phase::Standby));
        for _ in 0..4 {
            assert_eq!(drive_cpu(0x8000), Some(0x8000));
            assert_eq!(cpu_role_phase(), Some(Phase::Standby));
        }
        on_chords(true, false, false);
        cpu_role_after_chord();
        let _stopped = service_at(&root, &none(), "2026-11-07", state(13), state(7));
        assert!(Library::new(root.join(MACROS_DIR)).list().unwrap().entries.is_empty());
    }

    fn cpu_role_record_stores_the_jab_and_plays_it_on_the_cpu() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-jab");
        cpu_menu_role();
        on_chords(true, false, false);
        cpu_role_after_chord();
        let started = service_at(&root, &none(), "2026-11-07", state(13), state(7));
        assert!(started.diags.iter().any(|line| line.contains("role=cpu") && line.contains("recording=1")));
        for _ in 0..crate::model::recording::LOCKOUT_FRAMES {
            assert!(drive_cpu(1).is_none());
        }
        for _ in 0..3 {
            assert_eq!(drive_cpu(0x8000), Some(0x8000));
        }
        let jab = [36_872u32, 36_872, 36_872, 33_281];
        let mut drove = Vec::new();
        for buttons in jab {
            drove.push(drive_cpu(buttons).expect("the player's frame drives the cpu"));
        }
        assert_eq!(drove, jab);
        assert_eq!(cpu_role_phase(), Some(Phase::Recording));
        on_chords(true, false, false);
        cpu_role_after_chord();
        assert!(cpu_role_phase().is_none());
        let stopped = service_at(&root, &none(), "2026-11-07", state(13), state(7));
        assert!(stopped.diags.iter().any(|line| line.contains("recording=0")));
        let macros = Library::new(root.join(MACROS_DIR));
        let listed = macros.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let saved = macros.load(&listed.entries[0].id).unwrap();
        let buttons: Vec<u32> = saved.frames.iter().map(|frame| frame.buttons).collect();
        assert_eq!(buttons, jab);
        assert!(!buttons.contains(&1));
        assert!(!buttons.contains(&0x8000));
        publish_selection(
            &root,
            &Selection {
                bound: None,
                active_player: None,
                active_cpu: Some(listed.entries[0].id.clone()),
                loop_player: LoopMark::Finite(1),
                loop_cpu: LoopMark::Finite(1),
                drill: None,
                record_role: ViewRole::Cpu,
                slots: Default::default(),
            },
            None,
        );
        on_chords(false, false, true);
        let played = service_at(&root, &none(), "2026-11-07", state(13), state(7));
        if played.warp.is_some() {
            assert!(take_cpu_frame().is_none(), "no frame before the warp's restore lands");
            warp_landed();
            let _ = service_at(&root, &none(), "2026-11-07", state(13), state(7));
        }
        for (index, expected) in jab.iter().enumerate() {
            if index > 0 {
                let _ = service_at(&root, &none(), "2026-11-07", state(13), state(7));
            }
            let frame = take_cpu_frame().expect("play on cpu posts the stored frame");
            assert_eq!(frame.buttons, *expected);
            assert_eq!(playback_controller(frame).buttons, *expected);
        }
    }

    /// Eden `rc5-opus-cpu2` (2026-11-08): a CPU-role file begins on its first press (the jab).
    /// Play on CPU warped to the start save and fed frame 0 at once; the restore landed four
    /// frames later and undid the jab, so the CPU never acted. Frame 0 waits for the restore.
    fn a_warped_cpu_play_feeds_its_first_frame_after_the_restore() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-warp");
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        let save = NamedSave::new("Jab start", state(8), state(7)).unwrap();
        let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&save).unwrap();
        let jab = [33_281u32, 33_281, 0x8000, 0x8000];
        let frames = jab.iter().map(|&buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
        let mut recording =
            super::super::macros::MacroRecording::new("Jab", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        recording.set_start_save(Some(&save_id)).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.save(&root).unwrap();

        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert_eq!(tick.warp.as_deref(), Some(save_id.as_str()));
        let stock_ai = Inject { buttons: 0x8004, stick_x: 1.0, clamped_x: 1.0, ..Inject::NEUTRAL };
        assert_eq!(
            cpu_controller(take_cpu_frame).unwrap_or(stock_ai),
            Inject::NEUTRAL,
            "restore pending: the CPU sees neutral, not its stock jump"
        );
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert_eq!(cpu_controller(take_cpu_frame), Some(Inject::NEUTRAL));

        clear_playback(crate::model::input_delay::Clear::SaveStateLoad);
        warp_landed();
        assert_eq!(
            cpu_controller(take_cpu_frame).unwrap_or(stock_ai),
            Inject::NEUTRAL,
            "restore committed: neutral lasts until the worker queues frame zero"
        );
        let mut fed = Vec::new();
        for _ in 0..jab.len() {
            let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
            fed.push(cpu_controller(take_cpu_frame).expect("a frame after the restore").buttons);
        }
        assert_eq!(fed, jab, "the whole file, from its first press, reaches the CPU");
        assert_eq!(cpu_controller(take_cpu_frame).unwrap_or(stock_ai), stock_ai, "stock control returns after the tail");
    }

    #[test]
    fn training_reset_releases_cpu_with_an_empty_queue() {
        let _hold = hold();
        reset_host();
        let root = scratch("reset-cpu-empty");
        active_plain_macro(&root, Role::Cpu, 4);
        on_chords(false, false, true);
        WARP_HOLD.store(true, Ordering::Release);
        tick(&root);
        assert!(CPU_FRAMES.is_empty());
        assert_eq!(cpu_controller(take_cpu_frame), Some(Inject::NEUTRAL));

        clear_playback(crate::model::input_delay::Clear::TrainingReset);
        let stock_ai = Inject { buttons: 0x8004, stick_x: 1.0, clamped_x: 1.0, ..Inject::NEUTRAL };
        assert_eq!(cpu_controller(take_cpu_frame).unwrap_or(stock_ai), stock_ai);
        warp_landed();
        tick(&root);
        assert_eq!(cpu_controller(take_cpu_frame), None, "pending frame zero stays canceled");
    }

    #[test]
    fn training_reset_discards_the_buffered_cpu_tail_and_pending_play() {
        let _hold = hold();
        reset_host();
        let root = scratch("reset-cpu-tail");
        active_plain_macro(&root, Role::Cpu, 4);
        on_chords(false, false, true);
        tick(&root);
        assert_eq!(cpu_controller(take_cpu_frame).unwrap().buttons, 1);
        assert!(!CPU_FRAMES.is_empty());

        on_chords(false, false, true);
        clear_playback(crate::model::input_delay::Clear::TrainingReset);
        assert_eq!(cpu_controller(take_cpu_frame), None, "no buffered frame reaches the next think");
        tick(&root);
        assert!(CPU_FRAMES.is_empty());
        tick(&root);
        assert_eq!(cpu_controller(take_cpu_frame), None, "the old Play cannot restart after reset");

        on_chords(false, false, true);
        tick(&root);
        assert_eq!(cpu_controller(take_cpu_frame).unwrap().buttons, 1, "a new Play starts normally");
    }

    #[test]
    fn parry_practice_delays_its_first_pass_only() {
        let _hold = hold();
        reset_host();
        let root = scratch("parry-delay");
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        let save = NamedSave::new("Parry start", state(8), state(7)).unwrap();
        let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&save).unwrap();
        let frames = vec![InputFrame { buttons: 2, ..InputFrame::default() }; 2];
        let mut recording = super::super::macros::MacroRecording::new("Fireball", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        recording.set_start_save(Some(&save_id)).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.set_loop(Role::Cpu, LoopCount::Finite(2));
        index.save(&root).unwrap();
        set_parry(true);
        set_playback_delay(10);
        on_chords(false, false, true);
        assert_eq!(service_at(&root, &none(), "2026-11-18", state(8), state(7)).warp.as_deref(), Some(save_id.as_str()));
        warp_landed();
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        for _ in 0..10 { assert_eq!(take_cpu_frame(), None); }
        assert_eq!(take_cpu_frame().map(|frame| frame.buttons), Some(2));
        assert_eq!(take_cpu_frame().map(|frame| frame.buttons), Some(2));
        assert_eq!(service_at(&root, &none(), "2026-11-18", state(8), state(7)).warp.as_deref(), Some(save_id.as_str()));
        warp_landed();
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert_eq!(take_cpu_frame().map(|frame| frame.buttons), Some(2));
        reset_host();
    }

    /// Parry practice (DESIGN rev 2 §2.3): each Loop pass is one play-once run warped to the macro's start save, and
    /// the next pass starts only after the last one ended and no attempt is open.
    fn parry_each_pass_warps_to_the_start_save() {
        let _hold = hold();
        reset_host();
        let root = scratch("parry-passes");
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        let save = NamedSave::new("Parry start", state(8), state(7)).unwrap();
        let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&save).unwrap();
        let frames = vec![InputFrame { buttons: 2, ..InputFrame::default() }; 2];
        let mut recording = super::super::macros::MacroRecording::new("Fireball", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        recording.set_start_save(Some(&save_id)).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.set_loop(Role::Cpu, LoopCount::Finite(2));
        index.save(&root).unwrap();
        set_parry(true);
        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert_eq!(tick.warp.as_deref(), Some(save_id.as_str()), "the first pass warps");
        warp_landed();
        let mut warps = 0;
        for _ in 0..8 {
            let tick = service_at(&root, &none(), "2026-11-18", state(8), state(7));
            warps += usize::from(tick.warp.is_some());
            if tick.warp.is_some() {
                warp_landed();
            }
            let _ = take_cpu_frame();
        }
        assert_eq!(warps, 1, "Loop 2: one more pass, warped again");
        assert_eq!(PARRY_PASSES.load(Ordering::Acquire), 0);
        reset_host();
    }

    /// tr-review-astra-2 finding 1 (70bb16e9): a second Play while the last pass's frames are still queued stops the
    /// practice; it must not leave a pass owed that `parry_next_pass` then starts.
    fn parry_stop_during_buffered_playback_owes_no_pass() {
        let _hold = hold();
        reset_host();
        let root = scratch("parry-stop-buffered");
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        let save = NamedSave::new("Parry start", state(8), state(7)).unwrap();
        let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&save).unwrap();
        let frames = vec![InputFrame { buttons: 2, ..InputFrame::default() }; 2];
        let mut recording = super::super::macros::MacroRecording::new("Short", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        recording.set_start_save(Some(&save_id)).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.set_loop(Role::Cpu, LoopCount::Finite(2));
        index.save(&root).unwrap();
        set_parry(true);
        on_chords(false, false, true);
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        warp_landed();
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert!(!role_frames(Role::Cpu).is_empty(), "the short pass is queued for the hook");
        assert!(!RUN_CPU.load(Ordering::Acquire), "the machine has handed over its last frames");
        on_chords(false, false, true);
        for _ in 0..4 {
            let tick = service_at(&root, &none(), "2026-11-18", state(8), state(7));
            assert!(tick.warp.is_none(), "the stop must not warp into the next pass");
        }
        assert!(!RUN_CPU.load(Ordering::Acquire) && role_frames(Role::Cpu).is_empty());
        assert_eq!(PARRY_PASSES.load(Ordering::Acquire), 0, "no pass is owed after the stop");
        reset_host();
    }

    /// tr-review-astra-2 703e3f8d finding 1: turning Parry Off with a pass still owed ends the practice; turning it On
    /// again without Play must not resume the old passes.
    fn parry_off_cancels_owed_passes() {
        let _hold = hold();
        reset_host();
        let root = scratch("parry-off-owed");
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        let save = NamedSave::new("Parry start", state(8), state(7)).unwrap();
        let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&save).unwrap();
        let frames = vec![InputFrame { buttons: 2, ..InputFrame::default() }; 2];
        let mut recording = super::super::macros::MacroRecording::new("Short", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        recording.set_start_save(Some(&save_id)).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.set_loop(Role::Cpu, LoopCount::Infinite);
        index.save(&root).unwrap();
        set_parry(true);
        on_chords(false, false, true);
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        warp_landed();
        set_parry(false);
        for _ in 0..4 {
            let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
            let _ = take_cpu_frame();
        }
        set_parry(true);
        for _ in 0..4 {
            let tick = service_at(&root, &none(), "2026-11-18", state(8), state(7));
            assert!(tick.warp.is_none(), "Parry On again must not resume the old practice");
        }
        assert!(!RUN_CPU.load(Ordering::Acquire) && role_frames(Role::Cpu).is_empty());
        assert_eq!(PARRY_PASSES.load(Ordering::Acquire), 0, "no pass is owed once Parry was Off");
        reset_host();
    }

    /// tr-review-astra-3 / the primary 18:19Z (Eden accept-8 on 26c93533): a parry practice plays pass by pass, and between
    /// two passes (the last pass's frames are played, the next one waits for the open attempt) the CPU saw no playback.
    /// A configured Mash then queued its action on top of the replay instead of asking the playback to stop, and the
    /// queued action blocked every later request. Owed passes are part of the running CPU playback.
    fn parry_passes_owed_are_a_running_cpu_playback() {
        let _hold = hold();
        reset_host();
        let root = scratch("parry-mash-view");
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        let save = NamedSave::new("Parry start", state(8), state(7)).unwrap();
        let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&save).unwrap();
        let frames = vec![InputFrame { buttons: 2, ..InputFrame::default() }];
        let mut recording = super::super::macros::MacroRecording::new("Reticle", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        recording.set_start_save(Some(&save_id)).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.set_loop(Role::Cpu, LoopCount::Finite(2));
        index.save(&root).unwrap();
        set_parry(true);
        on_chords(false, false, true);
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        warp_landed();
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert!(take_cpu_frame().is_some(), "pass 1's only frame");
        set_parry_open(true);
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert!(!RUN_CPU.load(Ordering::Acquire) && role_frames(Role::Cpu).is_empty(), "between the passes");
        assert_eq!(PARRY_PASSES.load(Ordering::Acquire), 1, "pass 2 is owed");
        assert!(cpu_playback(false).running, "the CPU still sees the practice's playback running");
        set_parry(false);
        assert!(!cpu_playback(false).running, "Parry Off ends it");
        reset_host();
    }

    /// The primary 21:31Z (Eden accept-8 r05): Mash Always + Jump with Mash interrupts playback On, during a parry
    /// practice. The CPU's mash asks the playback to stop and queues nothing, both while a pass plays and between passes.
    fn parry_pass_mash_asks_to_stop_and_queues_nothing() {
        use crate::model::cpu::mash::{Action, BufferContext, Buffered, Mash};
        use crate::model::cpu::rng::Rng;
        use crate::model::cpu::settings::CpuSettings;
        let _hold = hold();
        reset_host();
        let mut snap = crate::model::settings::Snapshot::factory();
        let mut always = vec![0u8; 17];
        always[16] = 1;
        let mut jump = vec![0u8; 30];
        jump[usize::from(Action::JUMP.0)] = 1;
        snap.set("mash_triggers", &always).unwrap();
        snap.set("mash_state", &jump).unwrap();
        snap.set("playback_mash", &[1, 0]).unwrap();
        let s = CpuSettings::from_snapshot(&snap);
        assert!(s.playback_mash, "Mash interrupts playback is On");
        let mash_now = || {
            let mut mash = Mash::new();
            let ctx = BufferContext { playback: cpu_playback(false) };
            let out = mash.buffer_menu_mash(&s, &mut Rng::new(7), ctx, Some(Action::JUMP));
            (out, mash.queue.is_empty())
        };
        let root = scratch("parry-mash-stop");
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        let save = NamedSave::new("Parry start", state(8), state(7)).unwrap();
        let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&save).unwrap();
        let frames = vec![InputFrame { buttons: 2, ..InputFrame::default() }; 3];
        let mut recording = super::super::macros::MacroRecording::new("Reticle", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        recording.set_start_save(Some(&save_id)).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.set_loop(Role::Cpu, LoopCount::Finite(2));
        index.save(&root).unwrap();
        set_parry(true);
        on_chords(false, false, true);
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        warp_landed();
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert!(take_cpu_frame().is_some(), "pass 1 plays");
        assert_eq!(mash_now(), (Buffered::StopPlayback, true), "during pass 1");
        while take_cpu_frame().is_some() {}
        set_parry_open(true);
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert_eq!(PARRY_PASSES.load(Ordering::Acquire), 1, "pass 2 is owed");
        assert_eq!(mash_now(), (Buffered::StopPlayback, true), "between the passes");

        set_parry_open(false);
        let mut during_restart = None;
        parry_next_pass(|| {
            // The native consumer can query the Mash view while the Menu worker prepares the warp.
            during_restart = Some(mash_now());
            RUN_CPU.store(true, Ordering::Release);
        });
        let owed_after_start = PARRY_PASSES.load(Ordering::Acquire);
        let after_start = mash_now();
        RUN_CPU.store(false, Ordering::Release);
        let running_after_end = cpu_playback(false).running;
        reset_host();

        assert_eq!(during_restart, Some((Buffered::StopPlayback, true)), "during final pass preparation");
        assert_eq!(owed_after_start, 0, "the credit is spent after starting");
        assert_eq!(after_start, (Buffered::StopPlayback, true), "during the final pass");
        assert!(!running_after_end, "ordinary Mash may resume after the last pass");
    }

    fn parry_restart_does_not_restore_credits_canceled_by_off() {
        let _hold = hold();
        reset_host();
        set_parry(true);
        PARRY_PASSES.store(3, Ordering::Release);

        parry_next_pass(|| set_parry(false));
        set_parry(true);
        let owed_after_on = PARRY_PASSES.load(Ordering::Acquire);
        reset_host();

        assert_eq!(owed_after_on, 0);
    }

    /// Parry practice's refusals (approved rows parry-practice-no-recording and parry-practice-start-save-missing).
    fn parry_refuses_without_a_recording_or_its_start_save() {
        let _hold = hold();
        reset_host();
        let root = scratch("parry-refuse");
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        set_parry(true);
        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert!(tick.notices.contains(&PARRY_NO_RECORDING), "{:?}", tick.notices);
        let save = NamedSave::new("Gone", state(8), state(7)).unwrap();
        let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
        let save_id = saves.save_new(&save).unwrap();
        let mut recording = super::super::macros::MacroRecording::new("Jab", "13.0.5", StartState::neutral(7, 0, 1.0), vec![InputFrame::default()]).unwrap();
        recording.set_start_save(Some(&save_id)).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.save(&root).unwrap();
        saves.delete(&save_id).unwrap();
        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert_eq!(tick.notices.last(), Some(&PARRY_START_SAVE_MISSING), "the refusal is the notice left up");
        assert!(tick.warp.is_none() && !RUN_CPU.load(Ordering::Acquire), "nothing plays");
        reset_host();
    }

    fn a_cpu_play_without_a_start_save_feeds_at_once() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-nowarp");
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        let frames = vec![InputFrame { buttons: 33_281, ..InputFrame::default() }];
        let recording =
            super::super::macros::MacroRecording::new("Jab", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.save(&root).unwrap();
        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(tick.warp.is_none());
        assert_eq!(take_cpu_frame().expect("no warp, no wait").buttons, 33_281);
    }

    #[test]
    fn a_drill_yields_to_a_running_library_cpu_macro_until_it_is_stopped() {
        let _hold = hold();
        reset_host();
        let root = scratch("drill-library-loop");
        active_plain_macro(&root, Role::Cpu, 4);
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_loop(Role::Cpu, LoopCount::Infinite);
        index.save(&root).unwrap();

        on_chords(false, false, true);
        assert!(cpu_playback_blocks_drill(), "a pending Play has priority over drill Start");
        tick(&root);
        assert_eq!(slot_phase(), (Phase::Idle, 0));
        assert!(!taking(), "fighter-mismatch busy excludes library playback");
        assert!(RUN_CPU.load(Ordering::Acquire));
        assert!(cpu_playback_blocks_drill(), "Start must yield while a library macro owns the CPU");
        assert!(!crate::model::edge_juggle::drill_owns_pad(true, cpu_playback_blocks_drill()));
        assert_eq!(take_cpu_frame().map(|frame| frame.buttons), Some(1));

        // Since Start was refused, the ordinary Play on CPU chord can stop the loop.
        on_chords(false, false, true);
        tick(&root);
        assert!(!cpu_playback_blocks_drill());
        assert!(crate::model::edge_juggle::drill_owns_pad(true, cpu_playback_blocks_drill()));
        tick(&root);
        assert!(take_cpu_frame().is_none(), "no stopped macro can replay after the drill ends");
        reset_host();
    }

    #[test]
    fn a_drill_yields_until_the_library_cpu_macros_buffered_tail_drains() {
        let _hold = hold();
        reset_host();
        let root = scratch("drill-library-tail");
        active_plain_macro(&root, Role::Cpu, 4);
        on_chords(false, false, true);
        tick(&root);
        assert!(!RUN_CPU.load(Ordering::Acquire), "the entire take has been handed to the hook");
        assert!(!role_frames(Role::Cpu).is_empty(), "the buffered tail is still live");
        assert!(!taking());

        for buttons in 1..=4 {
            assert!(cpu_playback_blocks_drill(), "the last frame also belongs to the playback");
            assert!(!crate::model::edge_juggle::drill_owns_pad(true, cpu_playback_blocks_drill()));
            assert_eq!(take_cpu_frame().map(|frame| frame.buttons), Some(buttons));
        }
        assert!(!cpu_playback_blocks_drill(), "an empty queue admits placement and the drill pad");
        assert!(crate::model::edge_juggle::drill_owns_pad(true, cpu_playback_blocks_drill()));
        tick(&root);
        assert!(take_cpu_frame().is_none(), "the buffered tail drained exactly once");
        reset_host();
    }

    /// Eden `rc6-covrec-diag1` (2026-11-08): Mash interrupts playback On, the CPU played its active macro,
    /// Player 1's jab put it in hitstun, and no `screen=playback mash=1` came. The CPU's mash read only the
    /// old recorder's playback, which was idle, so it never asked the library run to stop.
    fn a_library_macro_playing_on_the_cpu_is_the_playback_the_mash_and_covered_settings_see() {
        use crate::model::cpu::playback::covered_setting_applies;
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-mash");
        active_plain_macro(&root, Role::Cpu, 30);
        assert!(!cpu_playback(false).running, "nothing plays yet");
        on_chords(false, false, true);
        tick(&root);
        assert!(take_cpu_frame().is_some());
        let seen = cpu_playback(false);
        assert!(seen.possesses_cpu(), "the CPU's library run is a running playback: {seen:?}");
        assert!(!covered_setting_applies(seen, false));
        assert!(!cpu_playback(false).recording, "slot playback does not report a recording");
        note_mash();
        tick(&root);
        assert!(take_cpu_frame().is_none(), "the mash stopped the CPU's run");
        assert!(!cpu_playback(false).running);
    }

    /// SAV-002 (p3-rec-xps-6 rows 20/21): a save-state load stops both roles' runs and starts nothing; a Play's own warp
    /// (WARP_HOLD) is that playback's start and is not stopped.
    fn a_load_stops_both_runs_but_not_a_plays_own_warp() {
        let _hold = hold();
        reset_host();
        let root = scratch("load-stops");
        active_plain_macro(&root, Role::Cpu, 30);
        on_chords(false, false, true);
        tick(&root);
        assert!(take_cpu_frame().is_some(), "the CPU's run plays");
        note_load();
        // p3f-verify-yoga-3: the stop waited for the next service tick (~2 s), and the take kept playing after Load.
        assert!(take_cpu_frame().is_none() && !playing(), "the load stops the runs at once, before the tick");
        let t = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(t.diags.iter().any(|l| l == "screen=playback load=1"), "{:?}", t.diags);
        assert!(take_cpu_frame().is_none(), "the load stopped the CPU's run");
        assert!(!cpu_playback(false).running && !playing());
        // A Play pressed with the load in the same tick starts nothing.
        on_chords(false, false, true);
        note_load();
        tick(&root);
        assert!(!playing(), "the load starts nothing");
        // The Play's own warp: WARP_HOLD is up while its restore runs, so that load does not stop it.
        WARP_HOLD.store(true, Ordering::Release);
        note_load();
        assert!(!LOAD.load(Ordering::Acquire), "a Play's own warp is not a stop");
        WARP_HOLD.store(false, Ordering::Release);
    }

    /// REC-027 (p3-rec-xps-7 06e/06f): "the macro stops and the CPU does the mash action" (menu/recording.md:305). The
    /// CPU's mash asks for the stop and waits for the playback to be gone before it acts; rr_host only stopped the run at
    /// its next service tick, ~1 s later, so the Hitstun trigger had passed and no Mash Jump came. The CPU must see its
    /// macro gone at once, before the tick.
    fn the_mash_stop_is_seen_by_the_cpu_before_the_next_tick() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-mash-now");
        active_plain_macro(&root, Role::Cpu, 30);
        on_chords(false, false, true);
        tick(&root);
        assert!(cpu_playback(false).running, "the CPU's run plays");
        note_mash();
        assert!(!cpu_playback(false).running, "the CPU sees its macro stopped at once, so its mash can act in the trigger");
        assert!(take_cpu_frame().is_none(), "no further macro frame reaches the CPU");
        // Astra 08:38Z: a worker already inside step_role publishes its frame after the mash cut the run.
        publish(Role::Cpu, InputFrame { buttons: 99, ..InputFrame::default() });
        assert!(take_cpu_frame().is_none() && !cpu_playback(false).running, "an in-flight producer cannot refill a cut run");
    }

    /// A live frame for either role reaches the hook on this tick while the worker holds the playback machine.
    fn the_frame_pop_never_waits_and_keeps_the_frame_on_contention() {
        let _hold = hold();
        reset_host();
        let frame = InputFrame { buttons: 2, ..InputFrame::default() };
        let (held_tx, held_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _machine = lock(&MACHINE);
            for role in [Role::Player, Role::Cpu] {
                run_flag(role).store(true, Ordering::Release);
                publish(role, frame);
            }
            held_tx.send(()).unwrap();
            let _ = release_rx.recv_timeout(std::time::Duration::from_secs(10));
        });
        held_rx.recv().unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let hook = std::thread::spawn(move || tx.send((take_player_frame(), take_cpu_frame())).unwrap());
        let got = rx.recv_timeout(std::time::Duration::from_secs(2));
        let _ = release_tx.send(());
        worker.join().unwrap();
        hook.join().unwrap();

        assert_eq!(got, Ok((Some(frame), Some(frame))), "both pending frames must reach this tick");
        assert_eq!((take_player_frame(), take_cpu_frame()), (None, None), "each frame was consumed once");
        reset_host();
    }

    /// note_mash and note_load run in game hooks. A cut must return while the recording
    /// worker holds the start-speed lock.
    fn the_hook_side_cuts_never_wait_on_worker_locks() {
        let _hold = hold();
        reset_host();
        let (held_tx, held_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let worker = std::thread::spawn(move || {
            let _speed = START_SPEED.lock();
            let _ = held_tx.send(());
            let _ = release_rx.recv_timeout(std::time::Duration::from_secs(10));
        });
        held_rx.recv().unwrap();
        RUN_CPU.store(true, Ordering::Release);
        RUN_PLAYER.store(true, Ordering::Release);
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let hook = std::thread::spawn(move || {
            note_mash();
            note_load();
            let _ = done_tx.send(());
        });
        let returned = done_rx.recv_timeout(std::time::Duration::from_secs(2)).is_ok();
        let _ = release_tx.send(());
        let _ = worker.join();
        let _ = hook.join();
        assert!(returned, "a hook-side cut waited on a lock the worker held");
        assert!(!RUN_CPU.load(Ordering::Acquire) && !RUN_PLAYER.load(Ordering::Acquire), "both runs are cut");
        assert!(take_cpu_frame().is_none() && take_player_frame().is_none() && !playing(), "and nothing of them plays");
    }

    /// Astra 09:48Z: the worker inside `step_role` when a mash cuts an endless (Loop ∞) CPU run must return. `publish`
    /// refuses a cut run's frames, so the queue never fills and an endless take never ends by itself.
    fn a_cut_endless_run_does_not_spin_the_worker() {
        let _hold = hold();
        reset_host();
        let root = scratch("cut-endless");
        active_plain_macro(&root, Role::Cpu, 30);
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_loop(Role::Cpu, LoopCount::Infinite);
        index.save(&root).unwrap();
        on_chords(false, false, true);
        tick(&root);
        assert!(take_cpu_frame().is_some(), "the endless CPU run plays");
        note_mash();
        // The worker's next fill, as the loop it was in continues after the cut.
        let book = load_book(&root).unwrap();
        let mut machine = lock(&MACHINE).take().expect("the machine holds the CPU run");
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            step_role(&mut machine, &book, Role::Cpu);
            let _ = tx.send(());
        });
        let returned = rx.recv_timeout(std::time::Duration::from_secs(5)).is_ok();
        assert!(returned, "step_role spun on a cut endless run");
        let _ = worker.join();
        assert!(take_cpu_frame().is_none(), "nothing of the cut run is fed");
    }

    /// A file of `n` distinct frames (buttons 1..=n), active on `role`, with no start save.
    fn active_plain_macro(root: &Path, role: Role, n: u32) -> Vec<u32> {
        let _ = service_at(root, &none(), "2026-11-08", state(8), state(7));
        let buttons: Vec<u32> = (1..=n).collect();
        let frames = buttons.iter().map(|&buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
        let recording =
            super::super::macros::MacroRecording::new("Run", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(root).unwrap();
        index.set_active(role, Some(&id), 1).unwrap();
        index.save(root).unwrap();
        buttons
    }

    fn tick(root: &Path) {
        play_track_service(root);
        let _ = service_at(root, &none(), "2026-11-08", state(8), state(7));
    }

    fn generated_fixture(tag: &str, frames: u32) -> (PathBuf, String, MacroRecording) {
        reset_host();
        let root = scratch(tag);
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        let inputs = (1..=frames).map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
        let recording = MacroRecording::new("Old", "13.0.5", StartState::neutral(7, 0, 1.0), inputs).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Cpu, 7);
        (root, id, recording)
    }

    fn generated_sample(role: Role, index: u32) {
        let sample = |kind, x| ReplayFrame {
            input: InputFrame::default(), start: StartState::neutral(kind, 0, 1.0), x, y: 0.0,
        };
        let slot = role_index(role);
        play_live(slot, &sample(7, index as f32));
        play_live(1 - slot, &sample(8, index as f32 + 100.0));
    }

    fn generated_take(role: Role, index: u32) -> Option<InputFrame> {
        generated_sample(role, index);
        match role { Role::Player => take_player_frame(), Role::Cpu => take_cpu_frame() }
    }

    fn generated_play(root: &Path, role: Role) {
        on_chords(false, role == Role::Player, role == Role::Cpu);
        tick(root);
    }

    fn generated_rows(root: &Path, id: &str, recording: &MacroRecording) -> Option<Vec<[ReplayFrame; 2]>> {
        super::super::macros::positions::read(&root.join(MACROS_DIR), id, &recording.frames)
    }

    fn a_poll_that_straddles_generation_arm_seeds_the_first_input_once() {
        let _hold = hold();
        for role in [Role::Cpu, Role::Player] { straddle_arm(role); }
    }

    fn straddle_arm(role: Role) {
        let (root, id, recording) = generated_fixture("generated-seed-at-take", 3);
        if role == Role::Player {
            let mut index = RecordingIndex::load(&root).unwrap();
            index.set_active(role, Some(&id), 1).unwrap();
            index.save(&root).unwrap();
            set_role_fighter(role, 7);
            set_playback_delay(2);
        }
        let publish_root = root.clone();
        BEFORE_TAKE.set(Some(Box::new(move || generated_play(&publish_root, role))));
        let seeds = std::cell::Cell::new(0);
        let seed = || {
            seeds.set(seeds.get() + 1);
            generated_sample(role, 42);
        };
        let poll = || match role {
            Role::Cpu => take_cpu_frame_seeded(seed),
            Role::Player => {
                let mut out = MappedInputs::default();
                unsafe { on_player_poll_seeded(&mut out, seed) }.then(|| to_frame(out))
            }
        };
        if role == Role::Player {
            for _ in 0..2 { assert_eq!(poll(), None); }
            assert_eq!(seeds.get(), 0, "a delay poll is not input zero");
        }
        for input in &recording.frames { assert_eq!(poll(), Some(*input)); }
        tick(&root);
        let rows = generated_rows(&root, &id, &recording).unwrap();
        assert_eq!((seeds.get(), rows[0][0].x, rows[0][1].x), (1, 42.0, 142.0));
        std::fs::remove_dir_all(root).unwrap();
    }

    fn a_generated_run_with_no_partner_does_not_keep_the_last_runs_fighter() {
        let _hold = hold();
        let (root, first_id, first) = generated_fixture("generated-missing-partner", 3);
        let sample = |kind| ReplayFrame { input: InputFrame::default(), start: StartState::neutral(kind, 0, 1.0), x: 0.0, y: 0.0 };
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        play_seed(1, Some(sample(7)));
        play_seed(0, Some(sample(8)));
        assert_eq!(take_cpu_frame(), Some(first.frames[0]));
        generated_play(&root, Role::Cpu);
        tick(&root);
        assert_eq!(generated_rows(&root, &first_id, &first).unwrap()[0][1].start.fighter_kind, 8);

        let second = MacroRecording::new("Next", "13.0.5", StartState::neutral(7, 0, 1.0), first.frames.clone()).unwrap();
        let second_id = Library::new(root.join(MACROS_DIR)).save_new(&second).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&second_id), 1).unwrap();
        index.save(&root).unwrap();
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        play_seed(1, Some(sample(7)));
        play_seed(0, None);
        assert_eq!(take_cpu_frame(), Some(second.frames[0]));
        generated_play(&root, Role::Cpu);
        tick(&root);
        assert_eq!(generated_rows(&root, &second_id, &second).unwrap()[0][1].start.fighter_kind, -1);
    }

    fn playback_over_existing_sidecar(source: crate::engine::macros::positions::Source, damaged: bool) -> bool {
        use crate::engine::macros::positions;
        let (root, id, recording) = generated_fixture("generated-body", 605);
        let dir = root.join(MACROS_DIR);
        let path = positions::path(&dir, &id);
        let frame = ReplayFrame { input: InputFrame::default(), start: recording.start, x: -22.0, y: 20.0 };
        let old = vec![[frame, frame]; 601];
        positions::write(&dir, &id, &recording.frames, &old, source).unwrap();
        if damaged {
            let mut bytes = std::fs::read(&path).unwrap();
            let partner_x = 40 + 600 * 72 + 36 + 28;
            bytes[partner_x..partner_x + 4].copy_from_slice(&f32::NAN.to_le_bytes());
            std::fs::write(&path, bytes).unwrap();
        }
        assert_eq!(positions::kept(&dir, &id, &recording.frames), Some((source, 601)));
        assert_eq!(generated_rows(&root, &id, &recording).is_none(), damaged);
        let bytes = std::fs::read(&path).unwrap();
        tick(&root);
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        for i in 0..2 { assert_eq!(generated_take(Role::Cpu, i), Some(recording.frames[i as usize])); }
        generated_play(&root, Role::Cpu);
        tick(&root);
        let correct = if damaged {
            positions::source(&dir, &id, &recording.frames) == Some(positions::Source::Generated)
                && generated_rows(&root, &id, &recording).is_some_and(|rows| {
                    rows.len() == 2 && (rows[1][0].x, rows[1][1].x) == (1.0, 101.0)
                })
        } else {
            std::fs::read(&path).unwrap() == bytes && generated_rows(&root, &id, &recording) == Some(old)
        };
        std::fs::remove_dir_all(root).unwrap();
        correct
    }

    fn bad_sidecar_bodies_are_replaced_after_playback() {
        use crate::engine::macros::positions::Source;
        let _hold = hold();
        let repaired = [Source::Generated, Source::Recorded].map(|source| playback_over_existing_sidecar(source, true));
        assert_eq!(repaired, [true, true], "a nonfinite partner row in chunk two must not protect either source");
    }

    fn valid_sidecars_win_over_a_shorter_playback() {
        use crate::engine::macros::positions::Source;
        let _hold = hold();
        for source in [Source::Generated, Source::Recorded] {
            assert!(playback_over_existing_sidecar(source, false), "a valid {source:?} file changed");
        }
    }

    fn an_old_macro_played_once_from_zero_gets_generated_positions() {
        use crate::engine::macro_ui::{clip_panel::{ClipAction, ClipEditor}, scrub_track::Pos};
        use crate::engine::macros::positions::{self, Source};
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-full", 5);
        tick(&root);
        generated_play(&root, Role::Cpu);
        for (i, input) in recording.frames.iter().enumerate() {
            assert_eq!(generated_take(Role::Cpu, i as u32), Some(*input));
        }
        tick(&root);
        let rows = generated_rows(&root, &id, &recording).expect("generated positions");
        assert_eq!(rows.len(), 5);
        for (i, pair) in rows.iter().enumerate() {
            assert_eq!((pair[0].x, pair[1].x), (i as f32, i as f32 + 100.0));
        }
        assert_eq!(positions::source(&root.join(MACROS_DIR), &id, &recording.frames), Some(Source::Generated));
        let loaded = Library::new(root.join(MACROS_DIR)).load_edited(&id).unwrap();
        let mut editor = ClipEditor::macro_file(loaded, false).unwrap();
        editor.on_action(ClipAction::SelectIn);
        assert_eq!(editor.scrub().1, Pos::Held);
    }

    fn a_stopped_playback_writes_the_inputs_that_were_consumed() {
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-stop", 3);
        tick(&root);
        generated_play(&root, Role::Cpu);
        assert_eq!(generated_take(Role::Cpu, 0), Some(recording.frames[0]));
        generated_play(&root, Role::Cpu);
        tick(&root);
        assert_eq!(generated_rows(&root, &id, &recording).unwrap().len(), 1);
        super::super::macros::positions::remove(&root.join(MACROS_DIR), &id);
        request_preview(&id, Role::Cpu, 0, 3);
        tick(&root);
        assert_eq!(generated_take(Role::Cpu, 0), Some(recording.frames[0]));
        stop_preview();
        tick(&root);
        tick(&root);
        assert_eq!(generated_rows(&root, &id, &recording).unwrap().len(), 1);
    }

    fn an_immediate_poll_of_the_first_preview_input_gets_its_positions() {
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-first-poll", 5);
        tick(&root);
        request_preview(&id, Role::Cpu, 0, 1);
        POLL_ON_PUBLISH.set(Some(|role| {
            assert_eq!(generated_take(role, 0).map(|frame| frame.buttons), Some(1));
        }));
        tick(&root);
        assert_eq!(fed_frames(Role::Cpu), 1);
        tick(&root);
        let positions = generated_rows(&root, &id, &recording).expect("the first input of X was consumed");
        assert_eq!((positions.len(), positions[0][0].x, positions[0][1].x), (1, 0.0, 100.0));
        std::fs::remove_dir_all(root).unwrap();
    }

    fn an_editor_preview_from_zero_binds_the_whole_macro() {
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-preview", 5);
        tick(&root);
        request_preview(&id, Role::Cpu, 0, 1);
        tick(&root);
        assert_eq!(generated_take(Role::Cpu, 0), Some(recording.frames[0]));
        tick(&root);
        assert_eq!(generated_rows(&root, &id, &recording).unwrap().len(), 1);
    }

    fn delay_polls_and_a_stop_before_the_first_input_keep_nothing() {
        use crate::engine::macros::positions;
        let _hold = hold();
        let (root, id, _) = generated_fixture("generated-delay", 5);
        tick(&root);
        set_playback_delay(3);
        let writes = positions::writes();
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        for i in 0..3 {
            assert_eq!(generated_take(Role::Cpu, i), None);
            tick(&root);
        }
        generated_play(&root, Role::Cpu);
        tick(&root);
        assert_eq!(positions::writes(), writes);
        assert!(!positions::path(&root.join(MACROS_DIR), &id).exists());
    }

    fn a_longer_generated_track_replaces_a_shorter_one_only() {
        use crate::engine::macros::positions;
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-longer", 5);
        tick(&root);
        generated_play(&root, Role::Cpu);
        for i in 0..2 { assert!(generated_take(Role::Cpu, i).is_some()); }
        generated_play(&root, Role::Cpu);
        tick(&root);
        assert_eq!(generated_rows(&root, &id, &recording).unwrap().len(), 2);
        generated_play(&root, Role::Cpu);
        for i in 0..5 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        assert_eq!(generated_rows(&root, &id, &recording).unwrap().len(), 5);
        let writes = positions::writes();
        generated_play(&root, Role::Cpu);
        assert!(generated_take(Role::Cpu, 0).is_some());
        generated_play(&root, Role::Cpu);
        tick(&root);
        assert_eq!(generated_rows(&root, &id, &recording).unwrap().len(), 5);
        assert_eq!(positions::writes(), writes);
    }

    fn a_generated_track_never_replaces_a_recorded_one() {
        use crate::engine::macros::positions::{self, Source};
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-recorded", 5);
        let dir = root.join(MACROS_DIR);
        let frame = ReplayFrame { input: InputFrame::default(), start: recording.start, x: 99.0, y: 3.0 };
        positions::write(&dir, &id, &recording.frames, &[[frame, frame]; 5], Source::Recorded).unwrap();
        let original = std::fs::read(positions::path(&dir, &id)).unwrap();
        let writes = positions::writes();
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        for i in 0..5 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        assert_eq!(std::fs::read(positions::path(&dir, &id)).unwrap(), original);
        assert_eq!(positions::writes(), writes);
        generated_play(&root, Role::Cpu);
        assert!(!play_sampling());
    }

    /// A positions file a take left short (SP-17) heals through use: a playback that covers more frames replaces it.
    fn a_short_recorded_track_heals_on_a_full_playback() {
        use crate::engine::macros::positions::{self, Source};
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-heal", 5);
        let dir = root.join(MACROS_DIR);
        let frame = ReplayFrame { input: InputFrame::default(), start: recording.start, x: 99.0, y: 3.0 };
        positions::write(&dir, &id, &recording.frames, &[[frame, frame]; 3], Source::Recorded).unwrap();
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        for i in 0..5 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        assert_eq!(positions::kept(&dir, &id, &recording.frames), Some((Source::Generated, 5)));
        assert_eq!(generated_rows(&root, &id, &recording).unwrap()[4][0].x, 4.0);
    }

    fn an_adopted_generated_track_is_cropped_as_generated() {
        use crate::engine::macro_ui::clip_panel::{ClipAction, ClipEditor, ClipEffect};
        use crate::engine::macros::positions::{self, Source};
        let _hold = hold();
        for source in [Source::Generated, Source::Recorded] {
            let (root, id, recording) = generated_fixture("generated-adoption", 5);
            let dir = root.join(MACROS_DIR);
            let library = Library::new(&dir);
            let mut editor = ClipEditor::macro_file(library.load(&id).unwrap(), false).unwrap();
            let rows: Vec<[ReplayFrame; 2]> = (0..5).map(|i| {
                let sample = |kind, x| ReplayFrame { input: InputFrame::default(), start: StartState::neutral(kind, 0, 1.0), x, y: 0.0 };
                [sample(7, i as f32), sample(8, i as f32 + 100.0)]
            }).collect();
            if source == Source::Recorded {
                positions::write(&dir, &id, &recording.frames, &rows, source).unwrap();
            }
            request_preview(&id, Role::Cpu, 0, 5);
            tick(&root);
            assert!(play_sampling());
            for (i, input) in recording.frames.iter().enumerate() {
                assert_eq!(generated_take(Role::Cpu, i as u32), Some(*input));
            }
            tick(&root);
            assert_eq!(positions::source(&dir, &id, &recording.frames), Some(source));
            assert!(editor.adopt_positions(library.load_edited(&id).unwrap()));
            editor.on_action(ClipAction::SelectIn);
            editor.on_action(ClipAction::Nudge(1));
            let ClipEffect::Export(export) = editor.on_action(ClipAction::Confirm) else { panic!("macro crop") };
            assert_eq!(&export.frames, &recording.frames[1..]);
            let new_id = library.save_new(&export).unwrap();
            assert_eq!(positions::source(&dir, &new_id, &export.frames), Some(source));
            assert_eq!(positions::read(&dir, &new_id, &export.frames), Some(rows[1..].to_vec()));
        }
    }

    fn the_play_chord_pass_opens_and_writes_no_positions_file() {
        use crate::engine::macros::positions;
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-chord", 5);
        let before = (positions::reads(), positions::heads(), positions::writes());
        generated_play(&root, Role::Cpu);
        assert!(play_sampling(), "the first chord opens the sampler");
        assert_eq!((positions::reads(), positions::heads(), positions::writes()), before);
        for (i, frame) in recording.frames.iter().enumerate() {
            assert_eq!(generated_take(Role::Cpu, i as u32), Some(*frame));
        }
        tick(&root);
        assert_eq!(generated_rows(&root, &id, &recording).unwrap().len(), 5);

        let (root, id, recording) = generated_fixture("generated-stop-chord", 5);
        let before = (positions::reads(), positions::heads(), positions::writes());
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        assert_eq!((positions::reads(), positions::heads(), positions::writes()), before);
        assert_eq!(generated_take(Role::Cpu, 0), Some(recording.frames[0]));
        tick(&root);
        assert_eq!(positions::writes(), before.2);
        assert_eq!(generated_take(Role::Cpu, 1), Some(recording.frames[1]));
        tick(&root);
        assert_eq!(positions::writes(), before.2);
        generated_play(&root, Role::Cpu);
        assert_eq!(positions::writes(), before.2, "the stop chord writes no positions");
        tick(&root);
        assert_eq!(positions::writes(), before.2 + 1);
        assert_eq!(generated_rows(&root, &id, &recording).unwrap().len(), 2);
    }

    fn nothing_is_allocated_for_generation_before_the_first_generating_play() {
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-lazy", 5);
        assert!(!play_allocated());
        for _ in 0..3 {
            tick(&root);
            assert!(!play_allocated(), "idle passes do not allocate");
        }
        set_role_fighter(Role::Cpu, 8);
        publish_fighter_prompt(false);
        generated_play(&root, Role::Cpu);
        assert!(!play_allocated(), "another fighter does not generate");
        for i in 0..5 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        assert!(!play_allocated());

        set_role_fighter(Role::Cpu, 7);
        let chain = Chain::new("Chain", vec![id.clone()]).unwrap();
        let chain_id = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&chain).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&chain_id), 1).unwrap();
        index.save(&root).unwrap();
        generated_play(&root, Role::Cpu);
        assert!(!play_allocated(), "a chain does not generate");
        for i in 0..5 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        assert!(!play_allocated());

        request_preview(&id, Role::Cpu, 1, 4);
        tick(&root);
        assert!(!play_allocated(), "a preview from In 1 does not generate");
        for i in 1..4 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        assert!(!play_allocated());

        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.save(&root).unwrap();
        generated_play(&root, Role::Cpu);
        assert!(play_allocated());
        assert!(play_sampling());
        for (i, frame) in recording.frames.iter().enumerate() {
            assert_eq!(generated_take(Role::Cpu, i as u32), Some(*frame));
        }
        tick(&root);
        assert!(play_allocated());
        assert_eq!(generated_rows(&root, &id, &recording).unwrap().len(), 5);
    }

    fn a_library_write_during_the_run_drops_its_track() {
        use crate::engine::macros::positions;
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-version", 5);
        tick(&root);
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        for i in 0..2 { assert!(generated_take(Role::Cpu, i).is_some()); }
        Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        for i in 2..5 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        assert!(!positions::path(&root.join(MACROS_DIR), &id).exists());
    }

    fn a_same_size_external_edit_before_settle_cannot_adopt_the_previous_runs_positions() {
        let _hold = hold();
        for stop in [false, true] { external_edit_before_settle(stop); }
    }

    fn external_edit_before_settle(stop: bool) {
        use crate::engine::macros::positions;
        let (root, id, recording) = generated_fixture("generated-external-edit", 3);
        let dir = root.join(MACROS_DIR);
        generated_play(&root, Role::Cpu);
        for (i, input) in recording.frames.iter().take(if stop { 2 } else { 3 }).enumerate() {
            assert_eq!(generated_take(Role::Cpu, i as u32), Some(*input));
        }
        // Same external-edit contract as playback_immediate: no store event and no file-size change.
        let path = dir.join(format!("{id}.json"));
        let mut bytes = std::fs::read(&path).unwrap();
        let field = b"\"buttons\":1";
        let at = bytes.windows(field.len()).position(|s| s == field).unwrap() + field.len() - 1;
        bytes[at] = b'9';
        std::fs::write(&path, bytes).unwrap();
        let changed = Library::new(&dir).load(&id).unwrap();
        assert_eq!(changed.frames[0].buttons, 9);
        if stop { generated_play(&root, Role::Cpu); } // The stop chord refreshes the held book before settle.
        tick(&root);
        assert!(!positions::path(&dir, &id).exists(), "the worker must not sign the old poses with the new inputs");
        generated_play(&root, Role::Cpu);
        for (i, input) in changed.frames.iter().enumerate() {
            assert_eq!(generated_take(Role::Cpu, 10 + i as u32), Some(*input));
        }
        tick(&root);
        assert_eq!(generated_rows(&root, &id, &changed).unwrap()[0][0].x, 10.0);
        std::fs::remove_dir_all(root).unwrap();
    }

    fn a_recorded_file_written_before_the_worker_drains_is_kept() {
        use crate::engine::macros::positions::{self, Source};
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-race", 5);
        tick(&root);
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        for i in 0..2 { assert!(generated_take(Role::Cpu, i).is_some()); }
        generated_play(&root, Role::Cpu);
        let dir = root.join(MACROS_DIR);
        let frame = ReplayFrame { input: InputFrame::default(), start: recording.start, x: 99.0, y: 0.0 };
        positions::write(&dir, &id, &recording.frames, &[[frame, frame]; 5], Source::Recorded).unwrap();
        let original = std::fs::read(positions::path(&dir, &id)).unwrap();
        tick(&root);
        assert_eq!(std::fs::read(positions::path(&dir, &id)).unwrap(), original);
    }

    fn only_a_single_macro_from_input_zero_on_its_own_fighter_generates() {
        use crate::engine::macros::positions;
        let _hold = hold();
        for kind in 0..7 {
            let (root, id, mut recording) = generated_fixture("generated-exclusions", 5);
            match kind {
                0 => {
                    let chain = Chain::new("Chain", vec![id.clone()]).unwrap();
                    let chain_id = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&chain).unwrap();
                    let mut index = RecordingIndex::load(&root).unwrap();
                    index.set_active(Role::Cpu, Some(&chain_id), 1).unwrap();
                    index.save(&root).unwrap();
                }
                1 => {
                    recording.frames[0] = InputFrame::default();
                    recording.trim = Trim::Start;
                    Library::new(root.join(MACROS_DIR)).overwrite(&id, &recording).unwrap();
                }
                5 => { set_role_fighter(Role::Cpu, 8); publish_fighter_prompt(false); }
                6 => set_role_fighter(Role::Cpu, -1),
                _ => {}
            }
            tick(&root);
            match kind {
                0 | 1 | 5 | 6 => generated_play(&root, Role::Cpu),
                2 => { request_preview(&id, Role::Cpu, 1, 4); tick(&root); }
                3 => { request_clip_preview(recording.clone(), Role::Cpu); tick(&root); }
                4 => {
                    set_role_fighter(Role::Player, 7);
                    assert!(request_drill_preview(&id, 5, Trim::Off));
                    tick(&root);
                }
                _ => unreachable!(),
            }
            assert!(!play_sampling(), "excluded start {kind}");
            let role = if kind == 4 { Role::Player } else { Role::Cpu };
            assert!(role_playing(role), "excluded run {kind} actually started");
            if kind != 3 { assert!(generated_take(role, 0).is_some(), "excluded run {kind} fed an input"); }
            tick(&root);
            assert!(!positions::path(&root.join(MACROS_DIR), &id).exists());
        }
    }

    fn a_loop_and_a_second_role_never_join_one_track() {
        use crate::engine::macros::positions;
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-loop", 3);
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_loop(Role::Cpu, LoopCount::Finite(2));
        index.save(&root).unwrap();
        tick(&root);
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        for i in 0..6 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        let rows = generated_rows(&root, &id, &recording).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.iter().map(|row| row[0].x as u32).collect::<Vec<_>>(), vec![0, 1, 2]);

        let (root, cpu, recording) = generated_fixture("generated-two-roles", 3);
        let player = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&player), 1).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Player, 7);
        tick(&root);
        on_chords(false, true, true);
        tick(&root);
        for i in 0..3 {
            assert!(generated_take(Role::Player, i).is_some());
            assert!(generated_take(Role::Cpu, i).is_some());
        }
        tick(&root);
        assert_eq!(generated_rows(&root, &player, &recording).unwrap().len(), 3);
        assert!(!positions::path(&root.join(MACROS_DIR), &cpu).exists());
    }

    fn a_finished_write_is_announced_once() {
        let _hold = hold();
        let (root, _, _) = generated_fixture("generated-notice", 3);
        tick(&root);
        generated_play(&root, Role::Cpu);
        let at_first_play = recording_version();
        for i in 0..3 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        assert!(take_generated());
        assert!(!take_generated());
        let after_settle = recording_version();
        generated_play(&root, Role::Cpu);
        assert!(!play_sampling(), "a done macro armed again: library version at the first Play {at_first_play:?}, after the settle {after_settle:?}, at the second Play {:?}", recording_version());
        for i in 0..3 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        assert!(!take_generated());
    }

    fn save_to_same_library_from_another_thread(root: &Path) {
        let root = root.to_path_buf();
        std::thread::spawn(move || {
            let inputs = vec![InputFrame::default(); 3];
            let other = MacroRecording::new("Other", "13.0.5", StartState::neutral(7, 0, 1.0), inputs).unwrap();
            Library::new(root.join(MACROS_DIR)).save_new(&other).unwrap();
        }).join().unwrap();
    }

    fn another_threads_save_to_the_same_library_rearms_a_done_macro() {
        let _hold = hold();
        let (root, _, _) = generated_fixture("generated-foreign-done", 3);
        tick(&root);
        generated_play(&root, Role::Cpu);
        for i in 0..3 { assert!(generated_take(Role::Cpu, i).is_some()); }
        tick(&root);
        assert!(take_generated());
        assert!(!take_generated());
        let before = recording_version();
        save_to_same_library_from_another_thread(&root);
        assert_ne!(recording_version(), before);
        generated_play(&root, Role::Cpu);
        assert!(play_sampling(), "a same-library save invalidates the done cache");
    }

    fn another_threads_save_to_the_same_library_drops_an_open_run() {
        use crate::engine::macros::positions;
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-foreign-open", 3);
        tick(&root);
        generated_play(&root, Role::Cpu);
        assert!(play_sampling());
        let writes = positions::writes();
        assert_eq!(generated_take(Role::Cpu, 0), Some(recording.frames[0]));
        let before = recording_version();
        save_to_same_library_from_another_thread(&root);
        assert_ne!(recording_version(), before);
        for i in 1..3 { assert_eq!(generated_take(Role::Cpu, i), Some(recording.frames[i as usize])); }
        tick(&root);
        assert!(!positions::path(&root.join(MACROS_DIR), &id).exists());
        assert_eq!(positions::writes(), writes);
    }

    fn the_last_input_is_credited_before_the_worker_drains() {
        use crate::engine::macros::positions;
        let _hold = hold();
        let (root, id, recording) = generated_fixture("generated-last-input", 2);
        tick(&root);
        generated_play(&root, Role::Cpu);
        assert_eq!(generated_take(Role::Cpu, 0), Some(recording.frames[0]));
        generated_sample(Role::Cpu, 1);
        let writes = positions::writes();
        let open = play_sampling();
        PLAY.poll(|| {
            let (frame, generation) = pop_frame(Role::Cpu, |_| {}).unwrap();
            assert_eq!(frame, recording.frames[1]);
            play_track_service(&root);
            assert_eq!(positions::writes(), writes);
            if open { assert!(play_sampling()); }
            credit_frame(Role::Cpu, generation, || {});
        });
        tick(&root);
        assert_eq!(generated_rows(&root, &id, &recording).map(|rows| rows.len()), Some(2));
        assert!(open);
    }

    /// Eden `rc5-fid-measure1` (2026-11-08): the worker's ticks are not the game's frames. Two ticks
    /// between two AI frames overwrote a frame in the single slot, and an AI frame with no tick before it
    /// got no frame, so the AI's own controls stood inside the run. Every file frame reaches the CPU
    /// once, in order, one per AI frame, whatever the worker's timing.
    fn cpu_frames_reach_the_ai_once_each_whatever_the_worker_timing() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-timing");
        let file = active_plain_macro(&root, Role::Cpu, 8);
        on_chords(false, false, true);
        tick(&root);
        tick(&root);
        let mut fed = Vec::new();
        // AI frames with the worker ticking twice, not at all, once, not at all...
        for ticks in [0usize, 0, 2, 1, 0, 0, 1, 3] {
            for _ in 0..ticks {
                tick(&root);
            }
            fed.push(take_cpu_frame().map(|frame| frame.buttons));
        }
        assert_eq!(fed, file.iter().map(|&b| Some(b)).collect::<Vec<_>>(), "one frame per AI frame, none lost");
        tick(&root);
        assert!(take_cpu_frame().is_none(), "the run is over");
    }

    /// The same transfer for Player 1's poll.
    fn player_frames_reach_the_poll_once_each_whatever_the_worker_timing() {
        let _hold = hold();
        reset_host();
        let root = scratch("player-timing");
        let file = active_plain_macro(&root, Role::Player, 6);
        on_chords(false, true, false);
        tick(&root);
        tick(&root);
        tick(&root);
        let mut fed = Vec::new();
        for ticks in [0usize, 0, 1, 0, 2, 0] {
            for _ in 0..ticks {
                tick(&root);
            }
            fed.push(take_player_frame().map(|frame| frame.buttons));
        }
        assert_eq!(fed, file.iter().map(|&b| Some(b)).collect::<Vec<_>>());
    }

    fn x_during_a_play_on_player_one_starts_no_drill_preview() {
        let _hold = hold();
        reset_host();
        let root = scratch("drill-preview-play-busy");
        let file = active_plain_macro(&root, Role::Player, 70);
        let take = super::super::rr_runtime::save_drill_take(
            &root, vec![InputFrame { buttons: 90, ..InputFrame::default() }],
            StartState::neutral(8, 0, 1.0), crate::model::trim::Trim::Off, 1, &[],
        ).unwrap().unwrap();
        on_chords(false, true, false);
        assert!(!request_drill_preview(&take, 1, Trim::Off), "a queued Play also blocks the preview");
        tick(&root);
        tick(&root);
        tick(&root);
        assert!(RUN_PLAYER.load(Ordering::Acquire));
        assert_eq!(take_player_frame().map(|frame| frame.buttons), Some(file[0]));
        assert!(!request_drill_preview(&take, 1, Trim::Off));
        assert!(!preview_running());
        tick(&root);
        assert_eq!(take_player_frame().map(|frame| frame.buttons), Some(file[1]));
        on_chords(false, true, false);
        tick(&root);
        assert!(!player_playback_blocks_preview());
        assert!(request_drill_preview(&take, 1, Trim::Off));
        assert!(preview_running());
    }

    /// The Play chord again stops the run: nothing buffered is fed after the stop.
    fn a_stopped_cpu_run_feeds_nothing_more() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-stop");
        let _file = active_plain_macro(&root, Role::Cpu, 8);
        on_chords(false, false, true);
        tick(&root);
        assert_eq!(take_cpu_frame().map(|frame| frame.buttons), Some(1));
        on_chords(false, false, true);
        tick(&root);
        assert!(take_cpu_frame().is_none(), "stopped");
    }

    /// A Play that warps holds the other role's buffered frames too, until the restore lands.
    fn a_warp_holds_the_buffered_cpu_frames_until_the_restore() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-hold");
        let _file = active_plain_macro(&root, Role::Cpu, 8);
        on_chords(false, false, true);
        tick(&root);
        assert_eq!(take_cpu_frame().map(|frame| frame.buttons), Some(1));
        WARP_HOLD.store(true, Ordering::Release);
        tick(&root);
        assert!(take_cpu_frame().is_none(), "a frame fed before the restore is undone by it");
        warp_landed();
        tick(&root);
        assert_eq!(take_cpu_frame().map(|frame| frame.buttons), Some(2), "the run goes on where it was");
    }

    fn cpu_role_player_path_still_stores_the_polls() {
        let _hold = hold();
        reset_host();
        let root = scratch("player-jab");
        player_menu_role();
        on_chords(true, false, false);
        cpu_role_after_chord();
        assert!(cpu_role_phase().is_none());
        let _ = service_at(&root, &none(), "2026-11-07", state(8), state(7));
        let frames = [1u32, 36_872, 36_872, 36_872, 33_281];
        for buttons in frames {
            let mut out = press(buttons);
            unsafe { on_player_poll(&mut out) };
            assert_eq!(out.buttons, buttons);
        }
        on_chords(true, false, false);
        cpu_role_after_chord();
        let _ = service_at(&root, &none(), "2026-11-07", state(8), state(7));
        let macros = Library::new(root.join(MACROS_DIR));
        let listed = macros.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let saved = macros.load(&listed.entries[0].id).unwrap();
        let buttons: Vec<u32> = saved.frames.iter().map(|frame| frame.buttons).collect();
        assert_eq!(buttons, frames);
    }

    fn a_record_writes_before_migration_when_the_slot_image_is_absent() {
        let _hold = hold();
        reset_host();
        let root = scratch("pending");
        on_chords(true, false, false);
        push_capture(ROLE_PLAYER, InputFrame { buttons: 4, ..InputFrame::default() });
        on_chords(true, false, false);
        let tick = service_pending(&root, state(8), state(7));
        assert!(!migrated());
        assert!(tick.diags.iter().any(|line| line.contains("recording=0")));
        assert_eq!(Library::new(root.join(MACROS_DIR)).list().unwrap().entries.len(), 1);
        let mut slots = none();
        slots[0] = Some(LegacySlot {
            frames: vec![InputFrame { buttons: 2, ..InputFrame::default() }],
            start: StartState::neutral(3, 0, 1.0),
        });
        let migrated_tick = service_at(&root, &slots, "2026-11-06", state(8), state(7));
        assert!(migrated());
        assert!(migrated_tick.diags.iter().any(|line| line == "screen=migrate once=1"));
    }

    fn make_combo_plays_the_named_file_and_keeps_the_drill() {
        let _hold = hold();
        reset_host();
        let root = scratch("make");
        let player = state(8);
        let cpu = state(7);
        let save_id = Store::<NamedSave>::new(root.join(SAVES_DIR))
            .save_new(&NamedSave::new("Start", player, cpu).unwrap())
            .unwrap();
        let source = one_macro(&root, "Source", 11, 8, Some(&save_id));
        let other = one_macro(&root, "Other", 22, 8, None);
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&other), 1).unwrap();
        index.set_drill(Some(Drill { kind: DrillKind::Combo, id: "kept-drill".into() }), 2).unwrap();
        index.save(&root).unwrap();
        // An earlier exchange is on the strip; the source's own link arrives while it plays.
        set_strip_sample((0, Some(sample_combo()), CaptureOwner::UNKNOWN));
        post_effect(Effect::MakeCombo { source: source.clone(), chain: false });
        let playing = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert_eq!(playing.warp.as_deref(), Some(save_id.as_str()));
        assert!(take_player_frame().is_none(), "the source waits for its start save's restore");
        warp_landed();
        let fed = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(fed.diags.iter().all(|line| !line.contains("saved=1")), "the combo is made after the source has played");
        assert_eq!(take_player_frame().expect("source frame").buttons, 11);
        set_strip_sample((1, Some(sample_combo()), CaptureOwner::UNKNOWN));
        let saved = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(saved.diags.iter().any(|line| line.contains("screen=make-combo") && line.contains("saved=1")));
        let combos = crate::model::combo_record::ComboLibrary::new(root.join("combos"));
        let listed = combos.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let made = combos.load(&listed.entries[0].id).unwrap();
        assert_eq!(made.made_from.as_deref(), Some(source.as_str()));
        assert!(made.name.starts_with("Combo "));
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.drill().map(|drill| drill.id.as_str()), Some("kept-drill"));
        assert_eq!(index.active(Role::Player), Some(other.as_str()));
        assert!(index.hot(Kind::Combo).contains(&listed.entries[0].id));
    }

    /// Operator 2026-11-15: the name keyboard opens in Quickplay's room too (it replaced `combo-name-online-default`).
    fn make_combo_failures_notice_and_open_the_keyboard_online() {
        let _hold = hold();
        reset_host();
        let root = scratch("make-fail");
        let source = one_macro(&root, "Bare", 3, 8, None);
        set_strip_sample((0, Some(crate::model::combo::Combo::default()), CaptureOwner::UNKNOWN));
        post_effect(Effect::MakeCombo { source: source.clone(), chain: false });
        let _ = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert_eq!(take_player_frame().expect("the source plays").buttons, 3);
        let empty = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(empty.diags.iter().any(|line| line == "screen=notice kind=combo-empty"));
        assert!(empty.notices.contains(&EMPTY_COMBO));
        assert!(crate::model::combo_record::ComboLibrary::new(root.join("combos")).list().unwrap().entries.is_empty());

        reset_host();
        let gone = one_macro(&root, "Gone", 4, 8, Some("deleted-save"));
        set_strip_sample((0, Some(sample_combo()), CaptureOwner::UNKNOWN));
        post_effect(Effect::MakeCombo { source: gone, chain: false });
        let missing = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(missing.warp.is_none());
        assert!(missing.diags.iter().any(|line| line == "screen=notice kind=macro-start-save-missing"));
        assert_eq!(take_player_frame().expect("plays anyway").buttons, 4);

        reset_host();
        let present = one_macro(&root, "Later", 6, 8, None);
        let chain = Chain::new("Gap", vec!["missing-macro".into(), present]).unwrap();
        let chain_id = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&chain).unwrap();
        post_effect(Effect::MakeCombo { source: chain_id, chain: true });
        let gap = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(gap.warp.is_none());
        assert_eq!(take_player_frame().expect("skips the missing step").buttons, 6);

        reset_host();
        post_effect(Effect::Keyboard { header: "Name macro", draft: "Online".into() });
        let online = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        let offer = online.keyboard.expect("an online room offers the name keyboard");
        assert_eq!((offer.header, offer.draft.as_str()), ("Name macro", "Online"));
        assert!(online.diags.iter().any(|line| line == "screen=rename header=Name macro"));
    }

    /// Eden rc6-seq-explore1 `seq-make` (2026-11-08 12:28Z, primary inbox 13:31Z): a played macro that lands no hit was
    /// saved as the strip's PREVIOUS exchange. RECORDING.md: Make combo records the exchange the file plays. Only a
    /// link the strip captured after Make combo began counts; none gives the empty notice and no file.
    fn make_combo_no_hit_saves_input_drill_not_previous_exchange() {
        let _hold = hold(); reset_host(); let root = scratch("make-no-hit");
        let source = one_macro(&root,"Whiff",3,8,None);
        set_strip_sample((4,Some(sample_combo()), CaptureOwner::UNKNOWN));
        post_effect(Effect::MakeCombo { source: source.clone(), chain:false });
        service_at(&root,&none(),"2026-11-15",state(8),state(7));
        let mut input=MappedInputs::NEUTRAL; assert!(unsafe { on_player_poll(&mut input) });
        assert_eq!(input.buttons,3);
        // A completed poll is acknowledged by the guarded fighter observation before the worker converts it.
        let pending=service_at(&root,&none(),"2026-11-15",state(8),state(7)); assert!(pending.diags.iter().all(|l| !l.contains("saved=")));
        observe_input(Snap { buttons:input.buttons, facing:-1, ..Snap::empty(1) },Late::NONE,crate::model::combo::Settings::factory().tolerance,false);
        set_strip_sample((4,Some(sample_combo()), CaptureOwner::UNKNOWN));
        let done=service_at(&root,&none(),"2026-11-15",state(8),state(7));
        assert!(!done.notices.contains(&EMPTY_COMBO));
        let lib=ComboLibrary::new(root.join("combos")); let entries=lib.list().unwrap().entries;
        assert_eq!(entries.len(),1); let file=lib.load(&entries[0].id).unwrap();
        assert_eq!(file.name,"Drill 1"); assert!(file.combo.input_only); assert_eq!(file.combo.len(),1);
        assert_eq!(file.made_from.as_deref(),Some(source.as_str())); assert!(RecordingIndex::load(&root).unwrap().drill().is_none());
    }

    fn a_direct_drill_keeps_its_take_as_a_macro_without_a_save_or_role_change() {
        let _hold=hold(); reset_host(); let root=scratch("direct-drill"); set_record_role(Role::Cpu); set_record_trim(crate::model::trim::Trim::Off);
        post_effect(Effect::RecordDrill); note_record_start(RecordStart { poses:[pose(0,0.0,1.0),pose(7,5.0,-1.0)], speeds:[(0.0,0.0);2] }); on_chords(true,false,false); assert_eq!(capture_role(),ROLE_PLAYER);
        for (f,b) in [1,0].into_iter().enumerate() { let mut input=MappedInputs { buttons:b,..MappedInputs::NEUTRAL }; unsafe { on_player_poll(&mut input); } observe_input(Snap { buttons:b, facing:1,..Snap::empty(f as u32+1) },Late::NONE,crate::model::combo::Settings::factory().tolerance,false); }
        on_chords(true,false,false); service_at(&root,&none(),"2026-11-15",state(8),state(7)); set_record_trim(crate::model::trim::Trim::End);
        let id=take_saved_drill().unwrap(); assert!(take_saved_drill().is_none());
        assert_eq!(RecordingIndex::load(&root).unwrap().drill().unwrap().id,id);
        let file=ComboLibrary::new(root.join("combos")).load(&id).unwrap(); assert_eq!(file.recorded_on.as_deref(),Some("Mario"));
        let macros=Library::new(root.join(MACROS_DIR)); let entries=macros.list().unwrap().entries;
        assert_eq!(entries.len(),1, "the drill keeps its take");
        let take=macros.load(&entries[0].id).unwrap();
        assert!(crate::model::combo_record::is_drill_take_name(&take.name));
        assert_eq!(take.frames.iter().map(|f| f.buttons).collect::<Vec<_>>(),[1,0]);
        assert!(take.start_save.is_none());
        assert_eq!(file.take.as_deref(),Some(entries[0].id.as_str()));
        assert_eq!(file.made_from, None);
        assert!(RecordingIndex::load(&root).unwrap().active(Role::Player).is_none());
        assert!(Store::<NamedSave>::new(root.join(SAVES_DIR)).list().unwrap().entries.is_empty());
        on_chords(true,false,false); assert_eq!(capture_role(),ROLE_CPU,"one-shot does not change ordinary Record role");
        CAP_ROLE.store(0,Ordering::Release);
    }

    fn a_recorded_drills_take_previews_on_player_one_through_the_play_queue() {
        let _hold = hold();
        reset_host();
        let root = scratch("direct-drill-preview");
        set_record_role(Role::Cpu);
        set_record_trim(crate::model::trim::Trim::Off);
        post_effect(Effect::RecordDrill);
        let first = crate::model::move_names::FighterId { chara: crate::model::hash40::hash40("ui_chara_miiswordsman"), specials: [0, 1, 2, 0] };
        post_record_start(RecordStart { poses: [pose(0x49, 0.0, 1.0), pose(7, 5.0, -1.0)], speeds: [(0.0, 0.0); 2] }, Some(first));
        crate::model::move_names::PUBLISHED.set_player(Some((0x49, crate::model::move_names::FighterId { specials: [2, 0, 1, 2], ..first })));
        on_chords(true, false, false);
        for (f, b) in [1, 0].into_iter().enumerate() {
            let mut input = MappedInputs { buttons: b, ..MappedInputs::NEUTRAL };
            unsafe { on_player_poll(&mut input); }
            observe_input(Snap { buttons: b, facing: 1, ..Snap::empty(f as u32 + 1) }, Late::NONE, crate::model::combo::Settings::factory().tolerance, false);
        }
        on_chords(true, false, false);
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        set_record_trim(crate::model::trim::Trim::End);
        let drill = ComboLibrary::new(root.join("combos")).load(&take_saved_drill().unwrap()).unwrap();
        assert_eq!(drill.fighter, Some(first), "even a later Mii of the same kind cannot replace the recorded loadout");
        crate::model::move_names::PUBLISHED.set_player(None);
        let source = drill.take.unwrap();
        request_preview(&source, Role::Player, 0, 2);
        assert!(preview_running());
        let mut frames = Vec::new();
        for _ in 0..3 {
            service_at(&root, &none(), "2026-11-23", state(8), state(7));
            assert!(take_cpu_frame().is_none());
            if let Some(frame) = take_player_frame() { frames.push(frame.buttons); }
        }
        assert_eq!(frames, [1, 0]);
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(take_player_frame().is_none() && !preview_running());
    }

    fn fed_poll(seen: &mut FedSeen, lines: &mut Vec<(u32, String)>, frame: &mut u32, hides: bool) -> Option<u32> {
        let ((run_p1, p1), (run_cpu, cpu)) = (fed_run(Role::Player).unwrap(), fed_run(Role::Cpu).unwrap());
        let at = *frame;
        seen.sample(Fed { p1, cpu, run: [run_p1, run_cpu], preview: preview_running(), hides }, |text| { lines.push((at, text.to_owned())); 0 });
        *frame += 1;
        take_player_frame().map(|taken| taken.buttons)
    }

    fn a_drill_previews_fed_count_and_its_fed_line_follow_each_frame_handed_out() {
        const N: u32 = 6;
        let _hold = hold();
        reset_host();
        let root = scratch("drill-fed");
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        let frames = (1..=N).map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
        let take = super::super::rr_runtime::save_drill_take(&root, frames, StartState::neutral(8, 0, 1.0), crate::model::trim::Trim::Off, 1, &[]).unwrap().unwrap();
        let (mut seen, mut lines, mut frame) = (FedSeen::default(), Vec::new(), 100);

        assert!(request_drill_preview(&take, N, Trim::Off));
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        let run1 = fed_run(Role::Player).unwrap().0;
        assert_eq!(fed_run(Role::Player), Some((run1, 0)));
        assert_eq!(fed_frames(Role::Player), 0);
        for expected in 1..=N {
            assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, true), Some(expected));
            assert_eq!(fed_frames(Role::Player), expected);
        }
        assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, true), None);
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(!preview_running());
        assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, false), None);
        assert_eq!(fed_frames(Role::Player), N);

        assert!(request_drill_preview(&take, N, Trim::Off));
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        let run2 = fed_run(Role::Player).unwrap().0;
        assert_eq!(run2, run1 + 1);
        assert_eq!(fed_frames(Role::Player), 0);
        for expected in 1..=3 {
            assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, true), Some(expected));
        }
        stop_preview();
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(!preview_running());
        assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, false), None);
        assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, false), None);
        assert_eq!(fed_frames(Role::Player), 3);
        assert_eq!(fed_frames(Role::Cpu), 0);

        let line = |p1: u32, run: u32, on: bool| format!("fed p1={p1} cpu=0 run.p1={run} run.cpu=0 preview={0} hides={0}", u8::from(on));
        let mut expected: Vec<(u32, String)> = (0..=N).map(|k| (100 + k, line(k, run1, true))).collect();
        expected.push((101 + N, line(N, run1, false)));
        expected.extend((0..3).map(|k| (102 + N + k, line(k, run2, true))));
        expected.push((105 + N, line(3, run2, false)));
        assert_eq!(lines, expected);
    }

    fn an_ordinary_plays_load_and_every_frame_show_on_the_fed_line() {
        let _hold = hold();
        reset_host();
        let root = scratch("play-fed");
        tick(&root);
        let frames = (1..=5).map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
        let recording = MacroRecording::new("Fed", "13.0.5", StartState::neutral(8, 0, 1.0), frames).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        RecordingIndex::update(&root, |index| { index.set_active(Role::Player, Some(&id), 1)?; Ok(true) }).unwrap();
        let (mut seen, mut lines, mut frame) = (FedSeen::default(), Vec::new(), 200);
        assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, false), None);
        assert!(lines.is_empty());
        assert_eq!(fed_run(Role::Player), Some((0, 0)));

        on_chords(false, true, false);
        tick(&root);
        assert_eq!(fed_run(Role::Player), Some((1, 0)));
        for expected in 1..=5 {
            assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, false), Some(expected));
        }
        assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, false), None);

        tick(&root);
        on_chords(false, true, false);
        tick(&root);
        assert_eq!(fed_run(Role::Player), Some((2, 0)));
        assert_eq!(take_player_frame().map(|taken| taken.buttons), Some(1));
        for expected in 2..=5 {
            assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, false), Some(expected));
        }
        assert_eq!(fed_poll(&mut seen, &mut lines, &mut frame, false), None);

        let line = |p1: u32, run: u32| format!("fed p1={p1} cpu=0 run.p1={run} run.cpu=0 preview=0 hides=0");
        let mut expected: Vec<(u32, String)> = (201..=206).map(|at| (at, line(at - 201, 1))).collect();
        expected.extend((207..=211).map(|at| (at, line(at - 206, 2))));
        assert_eq!(lines, expected);
    }

    fn a_count_is_not_paired_with_a_run_number_while_a_load_stores() {
        let _hold = hold();
        reset_host();
        assert_eq!(fed_run(Role::Cpu), Some((0, 0)));
        RUNS[role_index(Role::Cpu)].fetch_add(1, Ordering::AcqRel);
        assert_eq!(fed_run(Role::Cpu), None);
        assert_eq!(fed_run(Role::Player), Some((0, 0)));
        RUNS[role_index(Role::Cpu)].fetch_add(1, Ordering::AcqRel);
        assert_eq!(fed_run(Role::Cpu), Some((1, 0)));
    }

    fn a_stop_while_the_index_is_unreadable_retires_the_preview_run() {
        let _hold = hold();
        for stop in [true, false] {
            reset_host();
            let root = scratch(if stop { "drill-stop-bad-index" } else { "drill-bad-index" });
            service_at(&root, &none(), "2026-11-23", state(8), state(7));
            let frames = (1..=80).map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
            let take = super::super::rr_runtime::save_drill_take(&root, frames, StartState::neutral(8, 0, 1.0), crate::model::trim::Trim::Off, 1, &[]).unwrap().unwrap();
            let no_mod_write = store::recording_write_guard(&root).unwrap();
            assert!(request_drill_preview(&take, 80, Trim::Off));
            service_at(&root, &none(), "2026-11-23", state(8), state(7));
            assert_eq!(take_player_frame().unwrap().buttons, 1);
            assert!(preview_running());
            assert!(RUN_PLAYER.load(Ordering::Acquire));
            assert!(!PLAYER_FRAMES.is_empty());

            let index = root.join(super::super::library::INDEX_FILE);
            let held = root.join("index-held.json");
            let had_index = index.is_file();
            if had_index { std::fs::rename(&index, &held).unwrap(); }
            std::fs::create_dir(&index).unwrap();
            let reads = store::index_reads();
            // Cross the prefilled queue boundary while the index remains unreadable.
            for expected in 2..=65 {
                service_at(&root, &none(), "2026-11-23", state(8), state(7));
                assert_eq!(take_player_frame().map(|frame| frame.buttons), Some(expected));
            }
            assert!(preview_running() && !PLAYER_FRAMES.is_empty());
            assert_eq!(store::index_reads(), reads);
            if stop {
                stop_preview();
            } else {
                drop(no_mod_write);
                let generation = catalog_generation();
                one_macro(&root, "Changed library", 7, 8, None);
                assert_ne!(catalog_generation(), generation);
            }
            service_at(&root, &none(), "2026-11-23", state(8), state(7));
            assert_eq!(store::index_reads(), reads + usize::from(!stop));
            assert!(!preview_running());
            assert!(PLAYER_FRAMES.is_empty());
            assert!(take_player_frame().is_none());
            assert!(!RUN_PLAYER.load(Ordering::Acquire));

            std::fs::remove_dir(&index).unwrap();
            if had_index { std::fs::rename(&held, &index).unwrap(); }
            for _ in 0..3 {
                service_at(&root, &none(), "2026-11-23", state(8), state(7));
                assert!(take_player_frame().is_none());
                assert!(!preview_running());
                assert!(!player_playback_blocks_preview());
            }
        }
    }

    fn deleting_a_drill_deletes_its_take() {
        let _hold = hold();
        reset_host();
        let root = scratch("delete-direct-drill");
        set_record_trim(crate::model::trim::Trim::Off);
        post_effect(Effect::RecordDrill);
        note_record_start(RecordStart { poses: [pose(0, 0.0, 1.0), pose(7, 5.0, -1.0)], speeds: [(0.0, 0.0); 2] });
        on_chords(true, false, false);
        for (f, b) in [1, 0].into_iter().enumerate() {
            let mut input = MappedInputs { buttons: b, ..MappedInputs::NEUTRAL };
            unsafe { on_player_poll(&mut input); }
            observe_input(Snap { buttons: b, facing: 1, ..Snap::empty(f as u32 + 1) }, Late::NONE, crate::model::combo::Settings::factory().tolerance, false);
        }
        on_chords(true, false, false);
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        set_record_trim(crate::model::trim::Trim::End);
        let id = take_saved_drill().unwrap();
        let macros = Library::new(root.join(MACROS_DIR));
        let combo = ComboLibrary::new(root.join("combos")).load(&id).unwrap().combo;
        assert_eq!(macros.list().unwrap().entries.len(), 1);
        post_effect(Effect::DeleteFile { kind: crate::model::recording_menu::FileKind::Combo, id: id.clone() });
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(ComboLibrary::new(root.join("combos")).load(&id).is_err());
        assert!(macros.list().unwrap().entries.is_empty());

        let ordinary = one_macro(&root, "Ordinary macro", 1, 8, None);
        let fake = crate::model::combo_record::ComboRecord::new("Other drill".into(), combo, None).with_take(&ordinary);
        let fake_id = ComboLibrary::new(root.join("combos")).save_new(&fake).unwrap();
        post_effect(Effect::DeleteFile { kind: crate::model::recording_menu::FileKind::Combo, id: fake_id.clone() });
        service_at(&root, &none(), "2026-11-23", state(8), state(7));
        assert!(ComboLibrary::new(root.join("combos")).load(&fake_id).is_err());
        assert!(macros.load(&ordinary).is_ok());
    }


    fn make_preserves_an_open_record_prefix_and_physical_cstick() {
        let _hold=hold(); reset_host(); let root=scratch("overlap-prefix");
        let source=one_macro(&root,"Whiff",3,8,None);
        on_chords(true,false,false);
        for b in [4,0] { let mut input=MappedInputs { buttons:b,..MappedInputs::NEUTRAL }; unsafe { on_player_poll(&mut input); } }
        post_effect(Effect::MakeCombo { source:source.clone(),chain:false });
        service_at(&root,&none(),"2026-11-15",state(8),state(7));
        let mut input=MappedInputs::NEUTRAL; unsafe { on_player_poll(&mut input); } observe_input(Snap { buttons:input.buttons,facing:1,..Snap::empty(3) },Late::NONE,crate::model::combo::Settings::factory().tolerance,false);
        on_chords(true,false,false); service_at(&root,&none(),"2026-11-15",state(8),state(7));
        let lib=Library::new(root.join(MACROS_DIR)); let saved=lib.list().unwrap().entries.into_iter().find(|e| e.id!=source).unwrap();
        assert_eq!(lib.load(&saved.id).unwrap().frames.iter().map(|f|f.buttons).collect::<Vec<_>>(),vec![4,0,3]);
        assert_eq!(ComboLibrary::new(root.join("combos")).list().unwrap().entries.len(),1);
        reset_host(); post_effect(Effect::RecordDrill); on_chords(true,false,false);
        let mut mapped=MappedInputs { buttons:crate::model::frame_strip::CSTICK_ON,lstick_x:127,rstick_x:127,..MappedInputs::NEUTRAL };
        unsafe { on_player_poll(&mut mapped); } observe_input(Snap { buttons:mapped.buttons,rstick_x:127,facing:1,..Snap::empty(10) },Late::NONE,crate::model::combo::Settings::factory().tolerance,false);
        on_chords(true,false,false); service_at(&root,&none(),"2026-11-15",state(8),state(7));
        let drill=ComboLibrary::new(root.join("combos")).load(&take_saved_drill().unwrap()).unwrap();
        let events=drill.combo.get(0).unwrap().events().collect::<Vec<_>>(); assert_eq!(events.len(),1); assert_eq!(events[0].kind,crate::model::combo::Kind::CStick);
    }

    fn direct_drill_overflow_saves_nothing_and_reopening_cancels_arm() {
        let _hold=hold(); reset_host(); let root=scratch("drill-overflow");
        post_effect(Effect::RecordDrill); arm_drill_record(false); on_chords(true,false,false);
        assert!(!input_observation_wanted()); on_chords(true,false,false); service_at(&root,&none(),"2026-11-15",state(8),state(7));
        post_effect(Effect::RecordDrill); on_chords(true,false,false);
        for f in 0..=crate::model::combo::LINKS*crate::model::combo::STEPS {
            let mut input=MappedInputs { buttons:if f%2==0 {1} else {0},..MappedInputs::NEUTRAL }; unsafe { on_player_poll(&mut input); }
            observe_input(Snap { buttons:input.buttons, facing:1,..Snap::empty(f as u32) },Late::NONE,crate::model::combo::Settings::factory().tolerance,false);
        }
        on_chords(true,false,false); let done=service_at(&root,&none(),"2026-11-15",state(8),state(7)); assert!(done.notices.contains(&DRILL_CAPACITY));
        assert!(ComboLibrary::new(root.join("combos")).list().unwrap().entries.is_empty()); assert!(!input_observation_wanted());
        let mut overflow=sample_combo(); for _ in 0..crate::model::combo::LINKS { overflow.push(*overflow.get(0).unwrap()); }
        set_strip_sample((100,Some(overflow), CaptureOwner::UNKNOWN)); post_effect(Effect::SaveLastCombo { name:String::new() });
        let tick=service_at(&root,&none(),"2026-11-15",state(8),state(7)); assert!(tick.notices.contains(&COMBO_CAPACITY));
        assert!(ComboLibrary::new(root.join("combos")).list().unwrap().entries.is_empty());

    }

    /// Operator 2026-11-08 (primary inbox 13:08Z): *+ Save last combo…* saves the latest exchange as a combo file, as
    /// rc.4's COMBO-HELPER did (`combos/<id>.json` on the generic store): named as typed or Combo N, recorded on Player
    /// 1's fighter, hot, and the drill (RECORDING.md). With no link on the strip it says so and writes nothing.
    fn save_last_combo_writes_the_latest_exchange_names_it_and_drills_it() {
        let _hold = hold();
        reset_host();
        let root = scratch("save-last");
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        use crate::model::move_names::{FighterId, PUBLISHED};
        let recorded = FighterId { chara: crate::model::hash40::hash40("ui_chara_pikachu"), specials: [0; 4] };
        let later = FighterId { chara: crate::model::hash40::hash40("ui_chara_fox"), specials: [0; 4] };
        set_fighter_label("Pikachu");
        let combos = crate::model::combo_record::ComboLibrary::new(root.join("combos"));
        set_strip_sample((0, None, CaptureOwner::UNKNOWN));
        post_effect(Effect::SaveLastCombo { name: String::new() });
        let empty = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(empty.diags.iter().any(|line| line == "screen=notice kind=combo-empty"), "{:?}", empty.diags);
        assert!(empty.notices.contains(&EMPTY_COMBO));
        assert!(combos.list().map(|listed| listed.entries.is_empty()).unwrap_or(true));
        PUBLISHED.set_player(Some((8, recorded)));
        set_strip_sample((3, Some(sample_combo()), CaptureOwner::new(8, Some(recorded))));
        PUBLISHED.set_player(Some((7, later)));
        set_fighter_label("Fox");
        post_effect(Effect::SaveLastCombo { name: "Ryu BnB".into() });
        let saved = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(saved.diags.iter().any(|line| line.starts_with("screen=save-last-combo saved=")), "{:?}", saved.diags);
        let listed = combos.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let id = listed.entries[0].id.clone();
        let file = combos.load(&id).unwrap();
        assert_eq!(file.fighter, Some(recorded), "a later P1 cannot rename the retained exchange");
        assert_eq!((file.name.as_str(), file.combo, file.recorded_on.as_deref()), ("Ryu BnB", sample_combo(), Some("Pikachu")));
        PUBLISHED.set_player(None);
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.drill().map(|drill| (drill.kind, drill.id.as_str())), Some((DrillKind::Combo, id.as_str())));
        assert_eq!(index.hot(Kind::Combo).first(), Some(&id));
        post_effect(Effect::SaveLastCombo { name: String::new() });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        let names: Vec<String> = combos.list().unwrap().entries.into_iter().map(|entry| entry.name).collect();
        assert!(names.iter().any(|name| name == "Combo 1"), "{names:?}");
    }

    /// A take of Player 1 holds the records its polls were handed, whoever wrote them (the pad, a pilot), unchanged and
    /// in order; the poll leaves the record as it was; the stop saves the take.
    fn a_player_take_stores_the_record_the_poll_was_given() {
        let _hold = hold();
        reset_host();
        let root = scratch("ai-take");
        player_menu_role();
        on_chords(true, false, false);
        let _ = service_at(&root, &none(), "2026-11-25", state(8), state(7));
        let given = [(4u32, 90i8, -20i8), (0, 0, 0), (36_872, -127, 127), (1, 5, 0)];
        for (buttons, x, y) in given {
            let mut out = MappedInputs { buttons, lstick_x: x, lstick_y: y, ..MappedInputs::NEUTRAL };
            assert!(!unsafe { on_player_poll(&mut out) }, "nothing is injected during a take");
            assert_eq!((out.buttons, out.lstick_x, out.lstick_y), (buttons, x, y), "the record is left as given");
        }
        assert_eq!(captured_frames(), 4);
        on_chords(true, false, false);
        let stop = service_at(&root, &none(), "2026-11-25", state(8), state(7));
        assert!(stop.diags.iter().any(|line| line == "screen=recording role=player recording=0"));
        assert!(!taking());
        let macros = Library::new(root.join(MACROS_DIR));
        let listed = macros.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let saved = macros.load(&listed.entries[0].id).unwrap();
        let stored: Vec<_> = saved.frames.iter().map(|f| (f.buttons, f.lstick_x, f.lstick_y)).collect();
        assert_eq!(stored, given);
    }
    fn a_take_is_busy_from_its_record_edge_until_its_stop_is_serviced() {
        let _hold = hold();
        reset_host();
        let root = scratch("taking");
        assert!(!taking());
        on_chords(true, false, false);
        assert!(taking(), "the Record edge is busy before its service");
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert!(taking(), "an open take is busy");
        on_chords(true, false, false);
        assert!(taking(), "the stop edge is busy until it is serviced");
        let _ = service_at(&root, &none(), "2026-11-18", state(8), state(7));
        assert!(!taking(), "a serviced stop is idle");
        on_chords(false, true, false);
        assert!(!taking(), "a play is not a take: its own fighter Switch must stay admissible");
    }
    fn mismatch_holds_playback_until_keep_or_switch() {
        let _hold = hold();
        reset_host();
        let root = scratch("mismatch");
        let first = one_macro(&root, "Recorded", 15, 13, None);
        let later = one_macro(&root, "Later", 16, 1, None);
        let chain = Chain::new("Mix", vec![first.clone(), later]).unwrap();
        let chain_id = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&chain).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&first), 1).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Player, 8);
        on_chords(false, true, false);
        let held = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(held.diags.iter().any(|line| line.contains("screen=mismatch ask=macro ")));
        assert!(take_player_frame().is_none());
        answer_mismatch(MismatchChoice::Keep);
        let kept = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(kept.switch_kind.is_none());
        assert_eq!(take_player_frame().expect("keep plays").buttons, 15);

        reset_host();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&first), 3).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Player, 8);
        on_chords(false, true, false);
        let _ = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        answer_mismatch(MismatchChoice::Switch);
        let switched = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert_eq!(switched.switch_kind, Some(13));
        assert!(switched.switch_player);
        assert_eq!(take_player_frame().expect("switch plays").buttons, 15);

        reset_host();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&chain_id), 4).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Player, 8);
        on_chords(false, true, false);
        let chain_ask = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(chain_ask.diags.iter().any(|line| line.contains("ask=macro1")));
        answer_mismatch(MismatchChoice::Cancel);
        let cancelled = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(take_player_frame().is_none());
        assert!(cancelled.switch_kind.is_none());
    }

    fn an_answered_mismatch_wakes_the_service_gate_and_plays() {
        // rc.5: after migration the Switch wrapper ticks only while `pending()`; the answer was not in it, so the held
        // play never resumed. Each answer is logged once, and an answer the fighters no longer need is dropped.
        let _hold = hold();
        reset_host();
        let root = scratch("mismatch-gate");
        let first = one_macro(&root, "Recorded", 15, 13, None);
        let later = one_macro(&root, "Later", 16, 0, None);
        let chain = Chain::new("Mix", vec![first, later]).unwrap();
        let chain_id = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&chain).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&chain_id), 1).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Cpu, 0);
        on_chords(false, false, true);
        assert!(pending());
        let held = service_pending(&root, state(0), state(0));
        assert!(held.diags.iter().any(|line| line.contains("screen=mismatch ask=macro1 rows=")), "{:?}", held.diags);
        assert!(held.mismatch.is_some());
        assert!(!pending(), "a question waiting for its answer is not work");
        answer_mismatch(MismatchChoice::Keep);
        assert!(pending(), "the answer must reach the tick");
        let kept = service_pending(&root, state(0), state(0));
        assert!(kept.diags.iter().any(|line| line == "screen=mismatch ask=macro1 answer=keep"));
        assert!(kept.switch_kind.is_none());
        assert_eq!(take_cpu_frame().expect("keep plays macro 1 on the current fighter").buttons, 15);

        reset_host();
        set_role_fighter(Role::Cpu, 0);
        on_chords(false, false, true);
        let _ = service_pending(&root, state(0), state(0));
        answer_mismatch(MismatchChoice::Switch);
        assert!(pending());
        let switched = service_pending(&root, state(0), state(0));
        assert!(switched.diags.iter().any(|line| line == "screen=mismatch ask=macro1 answer=switch"));
        assert_eq!(switched.switch_kind, Some(13));
        assert!(!switched.switch_player, "the CPU's chain switches the CPU");

        reset_host();
        set_role_fighter(Role::Cpu, 0);
        on_chords(false, false, true);
        let _ = service_pending(&root, state(0), state(0));
        set_role_fighter(Role::Cpu, 13);
        answer_mismatch(MismatchChoice::Keep);
        let _ = service_pending(&root, state(13), state(13));
        assert!(lock(&ANSWER).is_none(), "an answer the fighters no longer need is dropped");
    }

    fn the_mismatch_ask_names_the_side_that_plays() {
        // hot-slots 2026-11-10 18:58Z (`cov-mismatch-cancel`): Play on Player 1 (Donkey Kong 2) with a Mario (1) macro
        // asked about Player 1's fighter, but the screen called it the CPU's. The ask carries the playing side.
        let _hold = hold();
        for (role, side, chords) in [
            (Role::Player, ViewRole::Player, (false, true, false)),
            (Role::Cpu, ViewRole::Cpu, (false, false, true)),
        ] {
            reset_host();
            let root = scratch("mismatch-side");
            let id = one_macro(&root, "Mario macro", 15, 1, None);
            let mut index = RecordingIndex::load(&root).unwrap();
            index.set_active(role, Some(&id), 1).unwrap();
            index.save(&root).unwrap();
            set_role_fighter(role, 2);
            on_chords(chords.0, chords.1, chords.2);
            let tick = service_pending(&root, state(2), state(2));
            let ask = tick.mismatch.expect("the recorded fighter is asked about");
            assert_eq!((ask.role, ask.recorded_kind, ask.current_kind), (side, 1, 2), "{role:?}");
        }
    }

    fn with_ask_before_switching_off_a_play_chord_keeps_the_fighter() {
        // Operator 2026-11-13 (TR6): every fighter question follows Ask before switching fighter. Off: a Play chord on a
        // macro or a chain recorded on another fighter plays on the current one, as the Keep answer does, unasked.
        let _hold = hold();
        reset_host();
        let root = scratch("ask-off");
        let first = one_macro(&root, "Recorded", 15, 13, None);
        let later = one_macro(&root, "Later", 16, 0, None);
        let chain = Chain::new("Mix", vec![first.clone(), later]).unwrap();
        let chain_id = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&chain).unwrap();
        for (id, origin) in [(&first, "macro"), (&chain_id, "macro1")] {
            reset_host();
            publish_fighter_prompt(false);
            let mut index = RecordingIndex::load(&root).unwrap();
            index.set_active(Role::Player, Some(id), 1).unwrap();
            index.save(&root).unwrap();
            set_role_fighter(Role::Player, 8);
            on_chords(false, true, false);
            let tick = service_at(&root, &none(), "2026-11-06", state(8), state(7));
            assert!(tick.mismatch.is_none(), "{origin}: nothing asks");
            let kept = format!("screen=mismatch ask={origin} prompt=off answer=keep");
            assert!(tick.diags.contains(&kept), "{origin}: {:?}", tick.diags);
            assert!(tick.switch_kind.is_none(), "{origin}");
            assert_eq!(take_player_frame().expect("plays on the current fighter").buttons, 15, "{origin}");
        }
        reset_host();
    }

    fn with_ask_before_switching_off_a_changed_drill_keeps_the_fighter() {
        // TR6: a combo drill recorded on another fighter stays on Player 1's fighter, unasked, with the row Off.
        let _hold = hold();
        reset_host();
        publish_fighter_prompt(false);
        let root = scratch("drill-ask-off");
        let record = crate::model::combo_record::ComboRecord::new("Drill".into(), sample_combo(), Some("Ryu".into()));
        let id = crate::model::combo_record::ComboLibrary::new(root.join("combos")).save_new(&record).unwrap();
        note_drill(Some("seed".into()));
        note_drill(Some(id));
        set_fighter_label("Mario");
        let tick = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(tick.mismatch.is_none(), "nothing asks");
        assert!(tick.diags.iter().any(|line| line == "screen=mismatch ask=combo prompt=off answer=keep"), "{:?}", tick.diags);
        assert!(tick.switch_name.is_none());
        reset_host();
    }

    fn a_changed_combo_drill_asks_and_switch_names_the_recorded_fighter() {
        let _hold = hold();
        reset_host();
        let root = scratch("drill");
        let record = crate::model::combo_record::ComboRecord::new("Drill".into(), sample_combo(), Some("Ryu".into()));
        let id = crate::model::combo_record::ComboLibrary::new(root.join("combos")).save_new(&record).unwrap();
        note_drill(Some("seed".into()));
        note_drill(Some(id));
        set_fighter_label("Mario");
        let asked = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert!(asked.diags.iter().any(|line| line.contains("screen=mismatch ask=combo")));
        assert!(asked.switch_name.is_none());
        answer_mismatch(MismatchChoice::Switch);
        let switched = service_at(&root, &none(), "2026-11-06", state(8), state(7));
        assert_eq!(switched.switch_name.as_deref(), Some("Ryu"));
        assert!(switched.switch_player);
    }

    fn rc5wire_publish_selection_writes_the_loop() {
        let _hold = hold();
        reset_host();
        let root = scratch("loop");
        let selection = Selection {
            bound: None,
            active_player: None,
            active_cpu: None,
            loop_player: LoopMark::Finite(5),
            loop_cpu: LoopMark::Infinite,
            drill: None,
            record_role: crate::model::recording_menu::ViewRole::Player,
            slots: Default::default(),
        };
        publish_selection(&root, &selection, None);
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.loop_count(Role::Player), LoopCount::Finite(5));
        assert_eq!(index.loop_count(Role::Cpu), LoopCount::Infinite);
    }

    fn a_changed_drill_installs_the_grade_target_once() {
        assert_eq!(follow_grade(None, None), GradeFollow::Same);
        assert_eq!(follow_grade(None, Some("combo-2")), GradeFollow::Install("combo-2"));
        assert_eq!(follow_grade(Some("combo-2"), Some("combo-2")), GradeFollow::Same);
        assert_eq!(follow_grade(Some("combo-2"), Some("combo-1")), GradeFollow::Install("combo-1"));
        assert_eq!(follow_grade(Some("combo-2"), None), GradeFollow::Clear);
    }

    // ---- rc.6: sequence drilling (docs/RECORDING.md § Sequences and the one drill) ---------------------------------

    fn one_combo(root: &std::path::Path, name: &str, from: crate::model::frame_strip::Move) -> String {
        use crate::model::combo::LinkRec;
        use crate::model::frame_strip::{Move, Verdict};
        let combo = Combo::from_links([LinkRec::presses(from, Move::Aerial, Verdict::True(None), None, &[])]).unwrap();
        crate::model::combo_record::ComboLibrary::new(root.join("combos"))
            .save_new(&crate::model::combo_record::ComboRecord::new(name.into(), combo, None))
            .unwrap()
    }

    fn one_sequence(root: &std::path::Path, name: &str, combos: &[String]) -> String {
        Store::<Sequence>::new(root.join(SEQUENCES_DIR)).save_new(&Sequence::new(name, combos.to_vec()).unwrap()).unwrap()
    }

    fn a_sequence_plan_stages_its_combos_in_order_and_skips_a_deleted_one() {
        use crate::model::frame_strip::Move;
        let root = scratch("seq-plan");
        let ryu = one_combo(&root, "Ryu BnB", Move::Jab);
        let nair = one_combo(&root, "Nair loop combo", Move::Aerial);
        let library = ComboLibrary::new(root.join("combos"));
        let owner = crate::model::move_names::FighterId { chara: crate::model::hash40::hash40("ui_chara_ryu"), specials: [0; 4] };
        let mut record = library.load(&ryu).unwrap();
        record.fighter = Some(owner);
        library.overwrite(&ryu, &record).unwrap();
        let id = one_sequence(&root, "Corner to kill", &[ryu.clone(), "deleted-combo".into(), nair.clone()]);
        let plan = sequence_plan(&root, &id).expect("the sequence file loads");
        assert_eq!(plan.name, "Corner to kill");
        assert_eq!(plan.steps.iter().map(|s| s.fighter).collect::<Vec<_>>(), [Some(owner), None]);
        assert_eq!(plan.steps.iter().map(|s| (s.id.as_str(), s.name.as_str())).collect::<Vec<_>>(), [
            (ryu.as_str(), "Ryu BnB"),
            (nair.as_str(), "Nair loop combo")
        ]);
        assert_eq!(plan.steps[0].combo.get(0).map(|l| l.from), Some(Move::Jab), "the combo the helper grades");
        assert_eq!((plan.streak, plan.best), (0, 0));
        assert!(sequence_plan(&root, "no-such-sequence").is_none());
    }

    fn a_sequence_longer_than_sixteen_combos_drills_every_combo() {
        use crate::model::frame_strip::Move;
        let root = scratch("seq-long");
        let ids: Vec<String> = (0..20).map(|n| one_combo(&root, &format!("Combo {n}"), Move::Jab)).collect();
        let id = one_sequence(&root, "Long", &ids);
        assert_eq!(sequence_plan(&root, &id).unwrap().steps.len(), 20);
        let mut drill = start_sequence_drill(&root, &id).unwrap();
        assert_eq!(drill.step(), (1, 20));
        let clean = crate::model::combo::Summary {
            outcome: crate::model::combo::Outcome::Completed,
            graded: 1,
            true_links: 1,
            total: 1,
            clean: true,
            fault: None,
        };
        for id in &ids[1..] {
            assert_eq!(drill.note_summary(&clean), Some(id.as_str()));
        }
        assert_eq!(drill.note_summary(&clean), Some(ids[0].as_str()));
        assert_eq!((drill.streak(), drill.best()), (1, 1), "combo 20 finished the sequence");
    }

    fn streak_and_best_are_saved_to_the_sequence_file_and_the_next_drill_goes_on_from_them() {
        use crate::model::frame_strip::Move;
        let _hold = hold();
        let root = scratch("seq-progress");
        let ryu = one_combo(&root, "Ryu BnB", Move::Jab);
        let id = one_sequence(&root, "Corner to kill", std::slice::from_ref(&ryu));
        let before = catalog_generation();
        assert!(save_sequence_progress(&root, &id, 4, 9));
        assert!(catalog_generation() > before, "the menu reloads the catalog");
        let file = Store::<Sequence>::new(root.join(SEQUENCES_DIR)).load(&id).unwrap();
        assert_eq!((file.streak, file.best, file.combos.clone()), (4, 9, vec![ryu.clone()]));
        let drill = start_sequence_drill(&root, &id).unwrap();
        assert_eq!((drill.streak(), drill.best()), (4, 9), "drilling it again goes on from the file");
        let plan = sequence_plan(&root, &id).unwrap();
        assert_eq!((plan.streak, plan.best), (4, 9));
        let catalog = super::super::rr_runtime::load_catalog(&root).unwrap();
        let shown = catalog.sequences.iter().find(|s| s.id == id).unwrap();
        assert_eq!((shown.streak, shown.best), (4, 9), "the Connections panel reads the file");
        assert!(!save_sequence_progress(&root, "no-such-sequence", 1, 1));
    }

    fn a_rename_or_a_saved_streak_keeps_the_plan_and_a_new_order_does_not() {
        use crate::model::frame_strip::Move;
        let root = scratch("seq-same");
        let ryu = one_combo(&root, "Ryu BnB", Move::Jab);
        let nair = one_combo(&root, "Nair loop combo", Move::Aerial);
        let id = one_sequence(&root, "Corner to kill", &[ryu.clone(), nair.clone()]);
        let plan = sequence_plan(&root, &id).unwrap();
        rename::<ComboRecord>(&root, "combos", &ryu, |f| f.name = "Ryu opener".into());
        assert!(save_sequence_progress(&root, &id, 2, 3));
        let renamed = sequence_plan(&root, &id).unwrap();
        assert_eq!(renamed.steps[0].name, "Ryu opener");
        assert!(plan.same_steps(&renamed) && plan != renamed);
        let store = Store::<Sequence>::new(root.join(SEQUENCES_DIR));
        let mut file = store.load(&id).unwrap();
        file.combos = vec![nair, ryu];
        store.overwrite(&id, &file).unwrap();
        assert!(!plan.same_steps(&sequence_plan(&root, &id).unwrap()));
    }

    fn the_one_drill_replaces_a_combo_with_a_sequence_and_back() {
        use DrillKind::{Combo as C, Sequence as S};
        assert_eq!(follow_drill(None, None, None), DrillFollow::Same);
        assert_eq!(follow_drill(None, Some(S), Some("seq-1")), DrillFollow::Sequence("seq-1"));
        assert_eq!(follow_drill(Some((S, "seq-1")), Some(S), Some("seq-1")), DrillFollow::Same);
        assert_eq!(follow_drill(Some((S, "seq-1")), Some(C), Some("combo-2")), DrillFollow::Combo("combo-2"), "a combo replaces it");
        assert_eq!(follow_drill(Some((C, "combo-2")), Some(S), Some("seq-1")), DrillFollow::Sequence("seq-1"), "and back");
        assert_eq!(follow_drill(Some((C, "combo-2")), Some(S), Some("seq-2")), DrillFollow::Sequence("seq-2"));
        assert_eq!(follow_drill(Some((S, "seq-1")), None, Some("sequence-9")), DrillFollow::Same, "not listed yet: retried");
        assert_eq!(follow_drill(Some((S, "seq-1")), None, None), DrillFollow::Clear);
    }

    fn a_numbered_row_drills_that_combo_alone_with_its_own_target_and_the_sequence_file_stays() {
        use crate::model::frame_strip::Move;
        let _hold = hold();
        reset_host();
        let root = scratch("seq-row");
        let ryu = one_combo(&root, "Ryu BnB", Move::Jab);
        let nair = one_combo(&root, "Nair loop combo", Move::Aerial);
        let id = one_sequence(&root, "Corner to kill", &[ryu.clone(), nair.clone()]);
        assert!(save_sequence_progress(&root, &id, 4, 9));
        let path = root.join(SEQUENCES_DIR).join(format!("{id}.json"));
        let before = std::fs::read(&path).unwrap();
        // The menu's numbered row makes the combo the drill (recording_menu tests); the index and the hook follow it.
        let selection = Selection {
            bound: None,
            active_player: None,
            active_cpu: None,
            loop_player: LoopMark::Finite(1),
            loop_cpu: LoopMark::Finite(1),
            drill: Some(nair.clone()),
            record_role: crate::model::recording_menu::ViewRole::Player,
            slots: Default::default(),
        };
        publish_selection(&root, &selection, Some(DrillKind::Combo));
        assert_eq!(
            follow_drill(Some((DrillKind::Sequence, id.as_str())), Some(DrillKind::Combo), selection.drill.as_deref()),
            DrillFollow::Combo(nair.as_str())
        );
        let drill = RecordingIndex::load(&root).unwrap().drill().cloned();
        assert_eq!(drill, Some(Drill { kind: DrillKind::Combo, id: nair.clone() }));
        assert_eq!(std::fs::read(&path).unwrap(), before, "the sequence file stays as it was");
    }

    fn delete_and_edit_reach_the_card() {
        use crate::model::frame_strip::Move;
        use crate::model::recording_menu::FileKind;
        let _hold = hold();
        reset_host();
        let root = scratch("seq-delete");
        let ryu = one_combo(&root, "Ryu BnB", Move::Jab);
        let nair = one_combo(&root, "Nair loop combo", Move::Aerial);
        let id = one_sequence(&root, "Corner to kill", &[ryu.clone(), nair.clone()]);
        post_effect(Effect::WriteSequenceCombos { id: id.clone(), combos: vec![nair.clone(), ryu.clone()] });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        let file = Store::<Sequence>::new(root.join(SEQUENCES_DIR)).load(&id).unwrap();
        assert_eq!(file.combos, [nair.clone(), ryu.clone()], "Edit combos is saved");
        post_effect(Effect::DeleteFile { kind: FileKind::Combo, id: ryu.clone() });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(crate::model::combo_record::ComboLibrary::new(root.join("combos")).load(&ryu).is_err(), "the combo is gone");
        let plan = sequence_plan(&root, &id).unwrap();
        assert_eq!(plan.steps.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), [nair.as_str()], "the sequence skips it");
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_drill(Some(Drill { kind: DrillKind::Sequence, id: id.clone() }), 1).unwrap();
        index.save(&root).unwrap();
        post_effect(Effect::DeleteFile { kind: FileKind::Sequence, id: id.clone() });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(sequence_plan(&root, &id).is_none(), "the sequence is gone");
        assert!(RecordingIndex::load(&root).unwrap().drill().is_none(), "and is no longer the drill");
        let chain_macro = one_macro(&root, "Jab", 1, 8, None);
        let chain = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&Chain::new("Chain 1", vec![chain_macro.clone()]).unwrap()).unwrap();
        post_effect(Effect::WriteChainSteps { id: chain.clone(), steps: vec![] });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(Store::<Chain>::new(root.join(MACROS_DIR)).load(&chain).unwrap().recordings.is_empty(), "Edit chain is saved");
        post_effect(Effect::DeleteFile { kind: FileKind::Chain, id: chain.clone() });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(Store::<Chain>::new(root.join(MACROS_DIR)).load(&chain).is_err(), "the chain is gone");
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&chain_macro), 2).unwrap();
        index.save(&root).unwrap();
        // rc.12 TR1: the menu's Playback Slots reach the index (a macro holds one slot), the worker is told to copy them,
        // and copies only the slots that differ from what it gave them; a deleted macro empties its slot.
        let other = one_macro(&root, "Dash", 1, 8, None);
        let _ = take_slots_dirty();
        let selection = Selection {
            bound: None,
            active_player: Some(chain_macro.clone()),
            active_cpu: None,
            loop_player: LoopMark::Finite(1),
            loop_cpu: LoopMark::Finite(1),
            drill: None,
            record_role: ViewRole::Player,
            slots: [Some(chain_macro.clone()), None, Some(other.clone()), None, Some(chain_macro.clone())],
        };
        publish_selection(&root, &selection, None);
        let wanted = RecordingIndex::load(&root).unwrap().slots().clone();
        assert_eq!(wanted, [None, None, Some(other.clone()), None, Some(chain_macro.clone())], "one slot per macro: the last");
        assert_eq!(take_slots_dirty(), Some(true), "the menu's change is announced");
        assert_eq!(take_slots_dirty(), None);
        let held = [Some(other.clone()), None, Some(other.clone()), None, None];
        assert_eq!(slot_changes(&wanted, &held), [(0, None), (4, Some(chain_macro.clone()))]);
        publish_selection(&root, &selection, None);
        assert_eq!(take_slots_dirty(), None, "the same slots again copy nothing");
        post_effect(Effect::DeleteFile { kind: FileKind::Macro, id: chain_macro.clone() });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(Library::new(root.join(MACROS_DIR)).load(&chain_macro).is_err(), "the macro is gone");
        assert!(RecordingIndex::load(&root).unwrap().active(Role::Player).is_none(), "and is no longer active");
        assert_eq!(RecordingIndex::load(&root).unwrap().slots(), &[None, None, Some(other), None, None], "nor in a slot");
        assert_eq!(take_slots_dirty(), Some(false), "copied again, quietly");
    }

    /// Astra v2r 1: the worker holds a taken pair (its bound save not written yet) for longer than the restore's six
    /// settle samples; every sample must see it pending, and the pair is settled only once its write is done. RED before
    /// 7651c6f2: with the pair taken, `SAVED_PAIR` was empty and nothing else marked the write.
    fn a_taken_pair_stays_pending_until_its_write_is_done() {
        let _hold = hold();
        reset_host();
        let root = scratch("held-pair");
        migrate_at_boot(&root, &none(), "2026-11-18").unwrap();
        save_captured([at_x(3.0), at_x(4.0)]);
        assert!(!pair_settled(), "a captured pair waits for the worker");
        let (taken_tx, taken_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let worker_root = root.clone();
        let worker = std::thread::spawn(move || {
            write_saved_with(&worker_root, move || {
                taken_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
        });
        taken_rx.recv().unwrap();
        for sample in 0..6 {
            assert!(!pair_settled(), "settle sample {sample}: the taken pair is still being written");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        release_tx.send(()).unwrap();
        assert!(worker.join().unwrap(), "the held write completes");
        assert!(pair_settled(), "written: settled");
        let made = RecordingIndex::load(&root).unwrap().bound().unwrap().to_owned();
        assert_eq!(Store::<NamedSave>::new(root.join(SAVES_DIR)).load(&made).unwrap().player.x, 3.0);
    }

    fn at_x(x: f32) -> SavedState {
        let mut state = state(0);
        state.x = x;
        state
    }

    /// RECORDING.md, "Save states only warp": Save writes the one bound save; Load warps both fighters to that save; A
    /// on a save binds it to Save and Load. Eden rc6-covrec-diag1 cov-bind-load (2026-11-08): with Left bound, Load went
    /// to the legacy slot, A on Right changed nothing, and Save wrote save_states.json. The machine's active slot is
    /// the bound save: this is what the worker gives it, and what Save writes back.
    fn the_active_slot_follows_the_bound_save_and_save_writes_it() {
        let _hold = hold();
        reset_host();
        let root = scratch("bound");
        migrate_at_boot(&root, &none(), "2026-11-08").unwrap();
        let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
        let left = saves.save_new(&NamedSave::new("Left", at_x(-70.0), at_x(-40.0)).unwrap()).unwrap();
        let right = saves.save_new(&NamedSave::new("Right", at_x(40.0), at_x(70.0)).unwrap()).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.bind_save(Some(&left), 1).unwrap();
        index.save(&root).unwrap();
        let first = bound_sync(&root).expect("the first sync").expect("Left is bound");
        assert_eq!((first[0].x, first[1].x), (-70.0, -40.0));
        assert!(bound_sync(&root).is_none(), "nothing changed");
        let selection = Selection {
            bound: Some(right.clone()),
            active_player: None,
            active_cpu: None,
            loop_player: LoopMark::Finite(1),
            loop_cpu: LoopMark::Finite(1),
            drill: None,
            record_role: ViewRole::Player,
            slots: Default::default(),
        };
        publish_selection(&root, &selection, None);
        assert_eq!(bound_sync(&root).unwrap().unwrap()[0].x, 40.0, "A on Right binds Right for Load");
        save_captured([at_x(5.0), at_x(6.0)]);
        assert!(write_saved_with(&root, || {}));
        let written = saves.load(&right).unwrap();
        assert_eq!((written.player.x, written.cpu.x), (5.0, 6.0), "Save writes the bound file");
        assert!(bound_sync(&root).is_none(), "the slot already holds what Save wrote");
        publish_selection(&root, &Selection { bound: None, ..selection }, None);
        assert_eq!(bound_sync(&root), Some(None), "nothing bound empties the slot");
        save_captured([at_x(1.0), at_x(2.0)]);
        assert!(write_saved_with(&root, || {}));
        let made = RecordingIndex::load(&root).unwrap().bound().unwrap().to_owned();
        assert_eq!(saves.load(&made).unwrap().name, "Save 1");
        assert_eq!(saves.load(&made).unwrap().player.x, 1.0);
    }

    /// Primary 2026-11-08 #70 (a): Edit placement edits the Recording pop-up role's active macro, the file Play plays. Eden
    /// rc6-covrec-diag2 cov-edit-placement refused with a macro active, and Play never read a placement. The anchor is
    /// where the start save puts the role; Play's warp adds the file's offset (or a kept one) to that role only.
    fn edit_placement_targets_the_active_macro_and_play_warps_with_its_offset() {
        let _hold = hold();
        reset_host();
        let root = scratch("placement");
        migrate_at_boot(&root, &none(), "2026-11-08").unwrap();
        set_record_role(Role::Cpu);
        assert_eq!(active_placement_ids(&root, Role::Cpu), Err(PLACE_NO_MACRO));
        let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
        let mut cpu = at_x(30.0);
        cpu.fighter_kind = 7;
        cpu.y = 2.0;
        let start = saves.save_new(&NamedSave::new("Start", at_x(-30.0), cpu).unwrap()).unwrap();
        let macros = Library::new(root.join(MACROS_DIR));
        let mut recording =
            MacroRecording::new("Nair", "13.0.5", StartState::neutral(7, 0, 1.0), vec![InputFrame::default()]).unwrap();
        let loose = macros.save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&loose), 1).unwrap();
        index.save(&root).unwrap();
        assert_eq!(active_placement_ids(&root, Role::Cpu), Err(PLACE_NO_START));
        recording.set_start_save(Some(&start)).unwrap();
        recording.set_placement(Placement { delta_x: 1.5, delta_y: 0.0, ..Placement::default() }).unwrap();
        let id = macros.save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 2).unwrap();
        index.save(&root).unwrap();
        assert_eq!(active_placement_ids(&root, Role::Cpu), Ok((start.clone(), id.clone())));

        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert_eq!(tick.warp.as_deref(), Some(start.as_str()));
        assert_eq!(tick.warp_place.map(|(role, place)| (role, place.delta_x)), Some((Role::Cpu, 1.5)));
        let pair = warp_pair(saves.load(&start).unwrap(), tick.warp_place);
        assert_eq!((pair[0].x, pair[1].x, pair[1].y), (-30.0, 31.5, 2.0), "only the playing role moves");
        let lifted = warp_pair(
            saves.load(&start).unwrap(),
            Some((Role::Cpu, Placement { delta_x: 0.0, delta_y: 3.0, ..Placement::default() })),
        );
        assert_eq!((lifted[1].y, lifted[1].situation_kind), (5.0, SITUATION_AIR), "a raised start is in the air");

        // A chain warps to macro 1's start save and plays with macro 1's placement, so Edit placement edits macro 1.
        let chain = Chain::new("Pressure", vec![id.clone(), loose.clone()]).unwrap();
        let chain_id = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&chain).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&chain_id), 3).unwrap();
        index.save(&root).unwrap();
        assert_eq!(active_placement_ids(&root, Role::Cpu), Ok((start, id)));
    }

    /// A save named Jab start (Player 1 at x 10, the CPU at x 50 with 6.3 %), and two macros that start from it;
    /// the first has an offset of +5.
    fn placement_library(root: &Path) -> (String, [String; 2]) {
        let mut cpu = at_x(50.0);
        (cpu.percent, cpu.fighter_kind) = (6.3, 7);
        let save = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&NamedSave::new("Jab start", at_x(10.0), cpu).unwrap()).unwrap();
        let macros = Library::new(root.join(MACROS_DIR));
        let ids = ["Nair", "Fair"].map(|name| {
            let mut recording = MacroRecording::new(name, "13.0.5", StartState::neutral(7, 0, 1.0), vec![InputFrame::default()]).unwrap();
            recording.set_start_save(Some(&save)).unwrap();
            if name == "Nair" {
                recording.set_placement(Placement { delta_x: 5.0, delta_y: 0.0, ..Placement::default() }).unwrap();
            }
            macros.save_new(&recording).unwrap()
        });
        (save, ids)
    }

    #[test]
    fn replays_opens_a_start_save_without_reading_it_twice() {
        let _hold = hold();
        reset_host();
        let root = scratch("replays-open-reads");
        let (save, macros) = placement_library(&root);
        let chain = Store::<Chain>::new(root.join(MACROS_DIR))
            .save_new(&Chain::new("Pressure", macros.to_vec()).unwrap()).unwrap();
        RecordingIndex::update(&root, |index| { index.set_active(Role::Cpu, Some(&chain), 1)?; Ok(true) }).unwrap();
        set_record_role(Role::Cpu);
        let book = Book::new(RecordingIndex::load(&root).unwrap());
        let open = |effect| {
            let mut tick = Tick::default();
            let before = store::file_reads();
            post_effect(effect);
            apply_menu(&root, &mut Playback::new(), &book, &mut tick);
            (tick, store::file_reads() - before)
        };
        let (direct, direct_reads) = open(Effect::ChangePlacement { save, relink: Some(macros[0].clone()) });
        let (active, active_reads) = open(Effect::ChangeActivePlacement);
        assert!(direct.place.is_some());
        assert_eq!(active.place, direct.place, "same pair, editing role, shared count and relink target");
        // Both paths read the save and count its links. Resolving an active chain
        // additionally needs the chain and its first macro, not a second save read.
        assert!(active_reads <= direct_reads + 2, "direct={direct_reads}; active chain={active_reads}");

        let library = Library::new(root.join(MACROS_DIR));
        let mut first = library.load(&macros[0]).unwrap();
        first.set_start_save(Some("missing")).unwrap();
        library.overwrite(&macros[0], &first).unwrap();
        let (missing, _) = open(Effect::ChangeActivePlacement);
        assert!(missing.place.is_none());
        assert_eq!(missing.notices, [PLACE_NO_START]);
    }

    /// What the picker confirms on `pair` after `edit`: the fields that differ, as the wizard reports them.
    fn placed(pair: &[SavedState; 2], copy: bool, macros: u8, edit: impl FnOnce(&mut crate::model::record_place::Setup)) -> Commit {
        use crate::model::record_place::{saved_fighter, Edits};
        let was = [saved_fighter(&pair[0]).0, saved_fighter(&pair[1]).0];
        let mut setup = was;
        edit(&mut setup);
        let edits = [0, 1].map(|i| Edits {
            spot: setup[i].place != was[i].place || setup[i].at != was[i].at,
            state: setup[i].state != was[i].state,
            percent: setup[i].percent != was[i].percent,
        });
        Commit { ticket: 7, copy, setup, edits, stage: 31, macros }
    }

    fn saved_pair(root: &Path, id: &str) -> [SavedState; 2] {
        let save = Store::<NamedSave>::new(root.join(SAVES_DIR)).load(id).unwrap();
        [save.player, save.cpu]
    }

    fn file_bytes(root: &Path, dir: &str, id: &str) -> Vec<u8> {
        std::fs::read(root.join(dir).join(format!("{id}.json"))).unwrap()
    }

    /// Operator 2026-11-25: Change placement "should also edit on the save state itself". Only the edited half is
    /// written, field by field; no macro file is touched, so Play still adds a macro's offset to the new spot.
    fn a_placement_save_rewrites_only_the_edited_half_of_the_file() {
        let _hold = hold();
        reset_host();
        let root = scratch("place-change");
        migrate_at_boot(&root, &none(), "2026-11-25").unwrap();
        let (save, macros) = placement_library(&root);
        let before = macros.clone().map(|id| file_bytes(&root, MACROS_DIR, &id));
        let pair = saved_pair(&root, &save);
        let commit = placed(&pair, false, 2, |setup| setup[1].at = [40.0, 0.0]);
        let done = commit_placement(&root, &save, Some(&macros[0]), commit);
        let Placed::Saved { ticket, pair: loads, notice } = done else { panic!("{done:?}") };
        assert_eq!((ticket, notice.as_str()), (7, "Saved · Jab start · 2 macros start here"));
        let written = saved_pair(&root, &save);
        assert_eq!(written[0], pair[0], "the half that was not edited is as saved");
        assert_eq!((written[1].x, written[1].saved_stage), (40.0, Some(31)));
        // rv-edge-juggle-sol 15:54:53Z: a spot edit keeps the file's own damage, 6.3 and not 6.
        assert_eq!(written[1].percent.to_bits(), pair[1].percent.to_bits());
        assert_eq!(*loads, written, "the pair to load once is the file's");
        assert_eq!(macros.clone().map(|id| file_bytes(&root, MACROS_DIR, &id)), before, "no macro file is touched");
        let nair = Library::new(root.join(MACROS_DIR)).load(&macros[0]).unwrap();
        let named = NamedSave::new("Jab start", written[0].clone(), written[1].clone()).unwrap();
        assert_eq!(warp_pair(named, Some((Role::Player, nair.placement)))[0].x, 15.0, "Play still adds the offset");
        // One macro reads in the singular; a save no macro starts from says nothing of macros.
        let one = commit_placement(&root, &save, None, placed(&written, false, 1, |setup| setup[0].percent = 20));
        assert!(matches!(&one, Placed::Saved { notice, .. } if notice == "Saved · Jab start · 1 macro starts here"), "{one:?}");
        let none = commit_placement(&root, &save, None, placed(&written, false, 0, |setup| setup[0].percent = 30));
        assert!(matches!(&none, Placed::Saved { notice, .. } if notice == "Saved · Jab start"), "{none:?}");
        assert_eq!(saved_pair(&root, &save)[0].percent, 30.0);
        // A save that is gone refuses, by its ticket.
        assert_eq!(commit_placement(&root, "no-such-save", None, placed(&pair, false, 0, |setup| setup[0].percent = 1)),
            Placed::Refused { ticket: 7, name: String::new() });
    }

    fn a_save_with_nothing_edited_writes_no_file() {
        let _hold = hold();
        reset_host();
        let root = scratch("place-none");
        migrate_at_boot(&root, &none(), "2026-11-25").unwrap();
        let (save, macros) = placement_library(&root);
        let before = file_bytes(&root, SAVES_DIR, &save);
        let pair = saved_pair(&root, &save);
        // Also when the answer was "copy": with nothing to write there is nothing to copy.
        for copy in [false, true] {
            let done = commit_placement(&root, &save, Some(&macros[0]), placed(&pair, copy, 2, |_| {}));
            let Placed::Saved { pair: loads, notice, .. } = done else { panic!("{done:?}") };
            assert_eq!((notice.as_str(), *loads == pair), ("Nothing changed · Jab start", true));
        }
        assert_eq!(file_bytes(&root, SAVES_DIR, &save), before);
        assert_eq!(Store::<NamedSave>::new(root.join(SAVES_DIR)).list().unwrap().entries.len(), 1);
        assert_eq!(Library::new(root.join(MACROS_DIR)).load(&macros[0]).unwrap().start_save.as_deref(), Some(save.as_str()));
    }

    fn the_bound_copy_follows_a_changed_placement() {
        let _hold = hold();
        reset_host();
        let root = scratch("place-bound");
        migrate_at_boot(&root, &none(), "2026-11-25").unwrap();
        let (save, _) = placement_library(&root);
        RecordingIndex::update(&root, |index| { index.bind_save(Some(&save), 1)?; Ok(true) }).unwrap();
        invalidate_bound();
        assert_eq!(bound_sync(&root).unwrap().unwrap()[1].x, 50.0);
        assert!(bound_sync(&root).is_none());
        let pair = saved_pair(&root, &save);
        assert!(matches!(commit_placement(&root, &save, None, placed(&pair, false, 0, |setup| setup[1].at = [40.0, 0.0])), Placed::Saved { .. }));
        assert_eq!(bound_sync(&root).unwrap().unwrap()[1].x, 40.0, "Load goes to the new spot");
    }

    /// Operator 2026-11-25, PC5: "copy it for this macro". The copy is a new save; it is written before the macro is
    /// linked to it, and the save and the other macro are not touched.
    fn a_copy_is_written_before_the_macro_is_relinked() {
        let _hold = hold();
        reset_host();
        let root = scratch("place-copy");
        migrate_at_boot(&root, &none(), "2026-11-25").unwrap();
        let (save, macros) = placement_library(&root);
        let before = (file_bytes(&root, SAVES_DIR, &save), file_bytes(&root, MACROS_DIR, &macros[1]));
        let pair = saved_pair(&root, &save);
        let done = commit_placement(&root, &save, Some(&macros[0]), placed(&pair, true, 2, |setup| setup[1].at = [40.0, 0.0]));
        let Placed::Saved { pair: loads, notice, .. } = done else { panic!("{done:?}") };
        assert_eq!(notice, "Saved a copy · Save 1");
        let library = Library::new(root.join(MACROS_DIR));
        let copy = library.load(&macros[0]).unwrap().start_save.expect("linked");
        assert_ne!(copy, save);
        assert_eq!((saved_pair(&root, &copy)[1].x, loads[1].x), (40.0, 40.0));
        assert_eq!(library.load(&macros[0]).unwrap().placement.delta_x, 5.0, "the macro keeps its offset");
        assert_eq!((file_bytes(&root, SAVES_DIR, &save), file_bytes(&root, MACROS_DIR, &macros[1])), before);
        // From the save's own screen a copy is a new save that no macro uses.
        let own = commit_placement(&root, &save, None, placed(&pair, true, 2, |setup| setup[0].at = [0.0, 0.0]));
        assert!(matches!(&own, Placed::Saved { notice, .. } if notice == "Saved a copy · Save 2"), "{own:?}");
        assert_eq!(Store::<NamedSave>::new(root.join(SAVES_DIR)).list().unwrap().entries.len(), 3);
        assert_eq!(library.load(&macros[1]).unwrap().start_save.as_deref(), Some(save.as_str()));
        assert_eq!(library.load(&macros[0]).unwrap().start_save.as_deref(), Some(copy.as_str()));
    }

    fn a_chain_copy_keeps_the_first_macros_own_start_save_switch() {
        let _hold = hold();
        reset_host();
        let root = scratch("place-chain-copy-switch");
        migrate_at_boot(&root, &none(), "2026-11-25").unwrap();
        let (save, macros) = placement_library(&root);
        let library = Library::new(root.join(MACROS_DIR));
        let mut first = library.load(&macros[0]).unwrap();
        first.set_start_save_on(false);
        library.overwrite(&macros[0], &first).unwrap();
        let chain = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&Chain::new("Pressure", macros.to_vec()).unwrap()).unwrap();
        RecordingIndex::update(&root, |index| { index.set_active(Role::Cpu, Some(&chain), 1)?; Ok(true) }).unwrap();
        set_record_role(Role::Cpu);
        post_effect(Effect::ChangeActivePlacement);
        let open = service_at(&root, &none(), "2026-11-25", state(8), state(7)).place.unwrap();
        assert_eq!(open.relink.as_ref(), Some(&macros[0]), "the chain uses macro 1 even with its own switch Off");
        assert_eq!(open.save, save);
        let commit = placed(&open.pair, true, 2, |setup| setup[1].at = [40.0, 0.0]);
        assert!(matches!(commit_placement(&root, &open.save, open.relink.as_deref(), commit), Placed::Saved { .. }));
        let written = library.load(&macros[0]).unwrap();
        let copy = written.start_save.clone().unwrap();
        assert_ne!(copy, save);
        assert_eq!(saved_pair(&root, &copy)[1].x, 40.0);
        assert_eq!(active_placement_ids(&root, Role::Cpu), Ok((copy.clone(), macros[0].clone())));
        first.start_save = Some(copy);
        assert_eq!(written.start_save_on, first.start_save_on, "copy must not turn standalone warps On");
        assert_eq!(written, first, "the new start-save id is the only macro edit");
        RecordingIndex::update(&root, |index| { index.set_active(Role::Cpu, Some(&macros[0]), 2)?; Ok(true) }).unwrap();
        assert_eq!(active_placement_ids(&root, Role::Cpu).unwrap_err(), PLACE_NO_START);
    }

    /// rv-edge-juggle-sol 16:38:12Z: the second write fails (a directory stands where the macro's temporary file
    /// goes). The old link stays, the copy does not, and the picker is told the save was not written.
    fn a_failed_relink_leaves_the_old_link_and_no_copy() {
        let _hold = hold();
        reset_host();
        let root = scratch("place-relink");
        migrate_at_boot(&root, &none(), "2026-11-25").unwrap();
        let (save, macros) = placement_library(&root);
        let before = (file_bytes(&root, SAVES_DIR, &save), macros.clone().map(|id| file_bytes(&root, MACROS_DIR, &id)));
        std::fs::create_dir(root.join(MACROS_DIR).join(format!("{}.json.tmp", macros[0]))).unwrap();
        let pair = saved_pair(&root, &save);
        let done = commit_placement(&root, &save, Some(&macros[0]), placed(&pair, true, 2, |setup| setup[1].at = [40.0, 0.0]));
        assert_eq!(done, Placed::Refused { ticket: 7, name: "Jab start".into() });
        assert_eq!(Store::<NamedSave>::new(root.join(SAVES_DIR)).list().unwrap().entries.len(), 1, "no copy is left");
        assert_eq!((file_bytes(&root, SAVES_DIR, &save), macros.clone().map(|id| file_bytes(&root, MACROS_DIR, &id))), before);
        let library = Library::new(root.join(MACROS_DIR));
        for id in &macros {
            assert_eq!(library.load(id).unwrap().start_save.as_deref(), Some(save.as_str()));
        }
    }

    /// Operator 2026-11-25, PC2: the Replays row "opens the SAME picker", on the start save of the pop-up role's
    /// active macro (a chain: its macro 1's). The save's own screen and the pop-up open it with no macro to relink.
    fn the_replays_row_opens_the_active_macros_start_save() {
        use crate::model::record_place::Shared;
        let _hold = hold();
        reset_host();
        let root = scratch("place-open");
        migrate_at_boot(&root, &none(), "2026-11-25").unwrap();
        set_record_role(Role::Cpu);
        post_effect(Effect::ChangeActivePlacement);
        let tick = service_at(&root, &none(), "2026-11-25", state(8), state(7));
        assert_eq!((tick.place, tick.notices), (None, vec![PLACE_NO_MACRO]));
        let (save, macros) = placement_library(&root);
        // The count is of macro files: a chain that uses one of them is not a third.
        let chain = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&Chain::new("Pressure", macros.to_vec()).unwrap()).unwrap();
        RecordingIndex::update(&root, |index| { index.set_active(Role::Cpu, Some(&chain), 1)?; Ok(true) }).unwrap();
        post_effect(Effect::ChangeActivePlacement);
        let tick = service_at(&root, &none(), "2026-11-25", state(8), state(7));
        let open = PlaceOpen {
            save: save.clone(),
            relink: Some(macros[0].clone()),
            name: "Jab start".into(),
            pair: saved_pair(&root, &save),
            edit: Role::Cpu,
            shared: Shared { macros: 2, from_macro: true },
        };
        assert_eq!(tick.place.as_deref(), Some(&open));
        post_effect(Effect::ChangePlacement { save: save.clone(), relink: None });
        let tick = service_at(&root, &none(), "2026-11-25", state(8), state(7));
        assert_eq!(tick.place.as_deref(), Some(&PlaceOpen { relink: None, shared: Shared { macros: 2, from_macro: false }, ..open.clone() }));
        post_effect(Effect::ChangePlacement { save: save.clone(), relink: Some(macros[1].clone()) });
        let tick = service_at(&root, &none(), "2026-11-25", state(8), state(7));
        assert_eq!(tick.place.as_deref(), Some(&PlaceOpen { relink: Some(macros[1].clone()), ..open }));
        post_effect(Effect::ChangePlacement { save: "no-such-save".into(), relink: None });
        assert_eq!(service_at(&root, &none(), "2026-11-25", state(8), state(7)).place, None);
    }

    /// Operator 2026-11-25, PC6 A: "Momentum row; an Offset row shows an old offset, and A clears it". Each row
    /// writes its one field into the macro's file; the menu's catalog carries both for the rows.
    fn the_momentum_and_offset_rows_write_only_their_field() {
        use crate::model::recording_menu::MacroPlace;
        let _hold = hold();
        reset_host();
        let root = scratch("place-rows");
        migrate_at_boot(&root, &none(), "2026-11-25").unwrap();
        let (save, macros) = placement_library(&root);
        let library = Library::new(root.join(MACROS_DIR));
        let before = library.load(&macros[0]).unwrap();
        let other = file_bytes(&root, MACROS_DIR, &macros[1]);
        let places = super::super::rr_runtime::load_catalog(&root).unwrap().macro_places;
        assert_eq!(places.get(&macros[0]), Some(&MacroPlace { offset: [5.0f32.to_bits(), 0.0f32.to_bits()], rest: false }));
        assert_eq!(places.get(&macros[1]), Some(&MacroPlace::default()));
        post_effect(Effect::SetMomentum { id: macros[0].clone(), rest: true });
        let _ = service_at(&root, &none(), "2026-11-25", state(8), state(7));
        let mut rest = library.load(&macros[0]).unwrap();
        assert_eq!(rest.placement, Placement { momentum: MomentumMode::AtRest, ..before.placement });
        rest.placement = before.placement;
        assert_eq!(rest, before, "nothing else of the file changes");
        post_effect(Effect::ClearOffset { id: macros[0].clone() });
        let _ = service_at(&root, &none(), "2026-11-25", state(8), state(7));
        let mut cleared = library.load(&macros[0]).unwrap();
        assert_eq!(cleared.placement, Placement { momentum: MomentumMode::AtRest, ..Placement::default() }, "Momentum stays");
        cleared.placement = before.placement;
        assert_eq!(cleared, before, "inputs, start save and name are as they were");
        assert_eq!(file_bytes(&root, MACROS_DIR, &macros[1]), other, "the other macro's file is untouched");
        let places = super::super::rr_runtime::load_catalog(&root).unwrap().macro_places;
        assert_eq!(places.get(&macros[0]), Some(&MacroPlace { offset: [0, 0], rest: true }));
        // Play after the clear warps to the save's own spot, at rest.
        RecordingIndex::update(&root, |index| { index.set_active(Role::Cpu, Some(&macros[0]), 1)?; Ok(true) }).unwrap();
        set_record_role(Role::Cpu);
        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-25", state(8), state(7));
        assert_eq!(tick.warp.as_deref(), Some(save.as_str()));
        assert_eq!(tick.warp_place.map(|(role, place)| (role, place.delta_x, place.momentum)), Some((Role::Cpu, 0.0, MomentumMode::AtRest)));
    }

    fn exact_brawler() -> crate::engine::mii::lifetime::Configuration {
        use crate::engine::mii::inventory::{classify, MiiClass, RawSlot, TaggedId, CHARINFO_LEN, TAG_CHARA, TAG_HAT, TAG_OUTFIT};
        let saved = classify(&RawSlot {
            index: 0, slot_id: 13231, name_len: 1,
            name_units: [65, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            charinfo: [7; CHARINFO_LEN], colour: 4, voice: 37, specials: [1, 0, 0, 0],
            tag_outfit: TaggedId((u64::from(TAG_OUTFIT) << 56) | (1 << 40) | 0x1b_fef7_a5f7),
            tag_hat: TaggedId((u64::from(TAG_HAT) << 56) | 0x0f_a9ef_a5ea),
            tag_chara: TaggedId((u64::from(TAG_CHARA) << 56) | MiiClass::Brawler.chara_hash40()),
        }).unwrap();
        crate::engine::mii::lifetime::Configuration::from_saved(&saved).unwrap()
    }

    /// Primary 2026-11-08 #54 (a): Switch on a CPU macro recorded on an exact Mii hands the macro's `playback_target()`
    /// to the existing replay-catalog request. Eden rc6-covrec-diag2 cov-named-mii: Switch loaded a default Mii of the
    /// class (`Direct(TrioFighter(74))`, outfit 69 / hat 0 / colour 2 / specials 0000), not the recorded one.
    fn switch_on_an_exact_mii_macro_asks_for_that_mii() {
        let _hold = hold();
        reset_host();
        let root = scratch("exact-mii");
        migrate_at_boot(&root, &none(), "2026-11-08").unwrap();
        let configuration = exact_brawler();
        let mut recording =
            MacroRecording::new("Brawler", "13.0.5", StartState::neutral(0x48, 0, 1.0), vec![InputFrame::default()]).unwrap();
        recording.set_mii(configuration).unwrap();
        let macros = Library::new(root.join(MACROS_DIR));
        let id = macros.save_new(&recording).unwrap();
        let plain = macros
            .save_new(&MacroRecording::new("Mario", "13.0.5", StartState::neutral(1, 0, 1.0), vec![InputFrame::default()]).unwrap())
            .unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Cpu, 7);
        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(tick.mismatch.is_some(), "the recorded fighter is asked about");
        answer_mismatch(MismatchChoice::Switch);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert_eq!(tick.switch_kind, Some(0x48));
        assert_eq!(tick.switch_mii, Some(crate::engine::mii::replay::Target::mii(configuration)), "the recorded Mii");

        reset_host();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&plain), 2).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Cpu, 7);
        on_chords(false, false, true);
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        answer_mismatch(MismatchChoice::Switch);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert_eq!((tick.switch_kind, tick.switch_mii), (Some(1), None), "any other fighter keeps the kind route");
    }

    /// Primary 2026-11-08 #69 (b): "Setups stay a separate library from save states"
    /// (LANE-GROK-RECORDING-REDESIGN-REPORT.md:322). Eden rc6-covrec-diag2 cov-setup-restore: the restore moved the
    /// fighters and also wrote them into the bound save. A restore only moves the fighters: the worker writes nothing,
    /// the bound file keeps its pair, and the active slot gets the bound save back, so Load still warps to it.
    fn a_setup_restore_leaves_the_bound_save_and_the_slot_returns_to_it() {
        let _hold = hold();
        reset_host();
        let root = scratch("setup-restore");
        migrate_at_boot(&root, &none(), "2026-11-08").unwrap();
        let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
        let bound = saves.save_new(&NamedSave::new("Bound", at_x(-70.0), at_x(-40.0)).unwrap()).unwrap();
        let before = saves.load(&bound).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.bind_save(Some(&bound), 1).unwrap();
        index.save(&root).unwrap();
        assert!(bound_sync(&root).is_some(), "the first sync");
        assert!(bound_sync(&root).is_none(), "nothing changed");
        setup_restored();
        assert!(!write_saved_with(&root, || {}), "a setup restore hands the worker nothing to write");
        assert_eq!(saves.load(&bound).unwrap(), before, "the bound save file is untouched");
        let again = bound_sync(&root).expect("the slot is given the bound save again").expect("Bound is bound");
        assert_eq!((again[0].x, again[1].x), (-70.0, -40.0));
        assert!(bound_sync(&root).is_none(), "once per restore");
        publish_selection(&root, &Selection {
            bound: None,
            active_player: None,
            active_cpu: None,
            loop_player: LoopMark::Finite(1),
            loop_cpu: LoopMark::Finite(1),
            drill: None,
            record_role: ViewRole::Player,
            slots: Default::default(),
        }, None);
        assert_eq!(bound_sync(&root), Some(None), "nothing bound");
        setup_restored();
        assert_eq!(bound_sync(&root), Some(None), "with nothing bound the restored slot is emptied again");
        assert_eq!(RecordingIndex::load(&root).unwrap().bound(), None, "and no save is made for it");
    }

    /// Primary 2026-11-08 23:03Z, SETUP-Y-BIND (cov-rec #69 (b)): Y in the Setup library binds a setup without
    /// restoring it. The setup stays in the active slot, so Load returns to it, until the next bound-save change. It
    /// never writes the bound save file. A (bind and restore) still gives the slot the bound save back.
    fn a_setup_bound_with_y_stays_loads_target_until_the_bound_save_changes() {
        let _hold = hold();
        reset_host();
        let root = scratch("setup-y");
        migrate_at_boot(&root, &none(), "2026-11-08").unwrap();
        let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
        let bound = saves.save_new(&NamedSave::new("Bound", at_x(-70.0), at_x(-40.0)).unwrap()).unwrap();
        let other = saves.save_new(&NamedSave::new("Other", at_x(10.0), at_x(20.0)).unwrap()).unwrap();
        let before = saves.load(&bound).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.bind_save(Some(&bound), 1).unwrap();
        index.save(&root).unwrap();
        assert!(bound_sync(&root).is_some(), "the first sync");

        setup_bound(false);
        assert!(bound_sync(&root).is_none(), "Y: the setup stays in the slot, so Load returns to it");
        assert!(!write_saved_with(&root, || {}), "Y hands the worker nothing to write");
        // A write elsewhere in the library (the menu close's index write) is not a bound-save change.
        bump_catalog();
        assert!(bound_sync(&root).is_none(), "the bound save did not change, so the setup stays");
        assert!(bound_sync(&root).is_none(), "Load twice: still the setup");
        assert_eq!(saves.load(&bound).unwrap(), before, "the bound save file is untouched");

        let selection = Selection {
            bound: Some(other.clone()),
            active_player: None,
            active_cpu: None,
            loop_player: LoopMark::Finite(1),
            loop_cpu: LoopMark::Finite(1),
            drill: None,
            record_role: ViewRole::Player,
            slots: Default::default(),
        };
        publish_selection(&root, &selection, None);
        let next = bound_sync(&root).expect("binding another save is a bound-save change").expect("Other is bound");
        assert_eq!((next[0].x, next[1].x), (10.0, 20.0), "Load goes to the newly bound save");

        setup_bound(true);
        let back = bound_sync(&root).expect("A: the slot gets the bound save back").expect("Other is bound");
        assert_eq!((back[0].x, back[1].x), (10.0, 20.0));
        assert_eq!(saves.load(&bound).unwrap(), before, "no save file was written");
    }

    fn pose(kind: i32, x: f32, lr: f32) -> StartPose {
        StartPose { fighter_kind: kind, situation_kind: 0, x, y: 0.0, percent: 0.0, lr, start_state: PlacementStartState::Standard }
    }

    /// tr-primary-2 14:33:53Z and 14:51:40Z: a take keeps the fighter of the role that recorded it, read where every take
    /// start is posted: the Record chord, and placement Ready through `note_record_start`.
    #[test]
    fn a_take_keeps_the_fighter_of_the_role_that_recorded_it() {
        use crate::model::move_names::FighterId;
        static WHO: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];
        fn reader(role: Role) -> Option<FighterId> {
            let chara = WHO[role_index(role)].load(Ordering::Acquire);
            (chara != 0).then_some(FighterId { chara, specials: [0, 1, 2, 0] })
        }
        let _hold = hold();
        set_fighter_reader(reader);
        let who = |chara| FighterId { chara, specials: [0, 1, 2, 0] };
        let saved = |root: &Path| {
            let macros = Library::new(root.join(MACROS_DIR));
            let id = macros.list().unwrap().entries[0].id.clone();
            let json: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(MACROS_DIR).join(format!("{id}.json"))).unwrap()).unwrap();
            (macros.load(&id).unwrap().recorded_fighter, json.get("recorded_fighter").cloned())
        };
        let take = |tag: &str, role: Role, side: u8, post: &dyn Fn(RecordStart)| {
            reset_host();
            let root = scratch(tag);
            set_record_role(role);
            post(RecordStart { poses: [pose(0, 0.0, 1.0), pose(8, 5.0, -1.0)], speeds: [(0.0, 0.0); 2] });
            on_chords(true, false, false);
            push_capture(side, InputFrame { buttons: 513, ..InputFrame::default() });
            on_chords(true, false, false);
            let _ = service_at(&root, &none(), "2026-11-25", live_pose(0).saved(), live_pose(1).saved());
            saved(&root)
        };
        WHO[0].store(0xa1, Ordering::Release);
        WHO[1].store(0xb2, Ordering::Release);
        // Placement Ready starts a CPU-role take through `note_record_start`: the CPU's identity, in the combo file's key.
        let (fighter, key) = take("rec-fighter-cpu", Role::Cpu, ROLE_CPU, &note_record_start);
        assert_eq!(fighter, Some(who(0xb2)));
        assert_eq!(key, Some(serde_json::json!({"ui_chara": "b2", "specials": [0, 1, 2, 0]})));
        // A Player take: the identity the chord read, and the reader's when the caller had none.
        let given = FighterId { chara: 0xc3, specials: [0; 4] };
        assert_eq!(take("rec-fighter-p1", Role::Player, ROLE_PLAYER, &|start| post_record_start(start, Some(given))).0, Some(given));
        assert_eq!(take("rec-fighter-p1-read", Role::Player, ROLE_PLAYER, &|start| post_record_start(start, None)).0, Some(who(0xa1)));
        // A directly recorded drill's take is Player 1's.
        reset_host();
        let root = scratch("rec-fighter-drill");
        set_record_role(Role::Player);
        set_record_trim(crate::model::trim::Trim::Off);
        post_effect(Effect::RecordDrill);
        post_record_start(RecordStart { poses: [pose(0, 0.0, 1.0), pose(8, 5.0, -1.0)], speeds: [(0.0, 0.0); 2] }, None);
        on_chords(true, false, false);
        for (f, b) in [1, 0].into_iter().enumerate() {
            let mut input = MappedInputs { buttons: b, ..MappedInputs::NEUTRAL };
            unsafe { on_player_poll(&mut input); }
            observe_input(Snap { buttons: b, facing: 1, ..Snap::empty(f as u32 + 1) }, Late::NONE, crate::model::combo::Settings::factory().tolerance, false);
        }
        on_chords(true, false, false);
        service_at(&root, &none(), "2026-11-25", state(0), state(8));
        set_record_trim(crate::model::trim::Trim::End);
        assert!(take_saved_drill().is_some());
        assert_eq!(saved(&root).0, Some(who(0xa1)));
        // No identity to read: the take has no key, as a take saved before it existed.
        WHO[0].store(0, Ordering::Release);
        WHO[1].store(0, Ordering::Release);
        assert_eq!(take("rec-fighter-none", Role::Cpu, ROLE_CPU, &note_record_start), (None, None));
        reset_host();
    }

    #[test]
    fn a_record_start_is_posted_without_a_lock() {
        let source = include_str!("rr_host.rs");
        let body = source.split("pub fn note_record_start(").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(!body.contains("lock("));
        assert!(!source.contains(concat!("static RECORD_", "START:")));
    }

    #[test]
    fn the_worker_uses_the_last_posted_start_once() {
        let _hold = hold();
        reset_host();
        let root = scratch("posted-start-once");
        for x in [1.0, 2.0] {
            note_record_start(RecordStart { poses: [pose(13, x, 1.0), pose(7, 0.0, -1.0)], speeds: [(0.0, 0.0); 2] });
        }
        for x in [40.0, 50.0] {
            on_chords(true, false, false);
            push_capture(ROLE_PLAYER, InputFrame { buttons: 4, ..InputFrame::default() });
            on_chords(true, false, false);
            let mut live = state(13);
            live.x = x;
            service_at(&root, &none(), "2026-11-22", live, state(7));
        }
        let macros = Library::new(root.join(MACROS_DIR));
        let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
        let mut positions: Vec<_> = macros.list().unwrap().entries.iter().map(|entry| {
            let recording = macros.load(&entry.id).unwrap();
            saves.load(recording.start_save.as_deref().unwrap()).unwrap().player.x
        }).collect();
        positions.sort_by(f32::total_cmp);
        assert_eq!(positions, [2.0, 50.0]);
    }

    #[test]
    fn five_starts_without_a_worker_tick_keep_the_newest() {
        let _hold = hold();
        reset_host();
        let root = scratch("five-posted-starts");
        for i in 1..=5 {
            note_record_start(RecordStart { poses: [pose(13, i as f32, 1.0), pose(7, 0.0, -1.0)], speeds: [(0.0, 0.0); 2] });
            on_chords(true, false, false);
            push_capture(ROLE_PLAYER, InputFrame { buttons: i, ..InputFrame::default() });
            on_chords(true, false, false);
        }
        service_at(&root, &none(), "2026-11-22", state(13), state(7));
        let macros = Library::new(root.join(MACROS_DIR));
        let entries = macros.list().unwrap().entries;
        assert_eq!(entries.len(), 1);
        let recording = macros.load(&entries[0].id).unwrap();
        let save = Store::<NamedSave>::new(root.join(SAVES_DIR)).load(recording.start_save.as_deref().unwrap()).unwrap();
        assert_eq!(save.player.x, 5.0);
    }

    fn placed_save(tag: &str) -> NamedSave {
        use crate::model::state::record::StartState as PlacedState;
        reset_host();
        let root = scratch(tag);
        let mut p1 = pose(13, -30.0, -1.0);
        p1.y = 4.0;
        p1.percent = 80.0;
        p1.situation_kind = 0;
        p1.start_state = PlacedState::MissedTechUp;
        let mut cpu = pose(7, 40.0, 1.0);
        cpu.y = -8.0;
        cpu.percent = 45.0;
        cpu.situation_kind = SITUATION_AIR;
        cpu.start_state = PlacedState::Tumble;
        note_record_start(RecordStart { poses: [p1, cpu], speeds: [(0.0, 0.0); 2] });
        on_chords(true, false, false);
        push_capture(ROLE_PLAYER, InputFrame { buttons: 4, ..InputFrame::default() });
        on_chords(true, false, false);
        service_at(&root, &none(), "2026-11-22", state(13), state(7));
        let macros = Library::new(root.join(MACROS_DIR));
        let recording = macros.load(&macros.list().unwrap().entries[0].id).unwrap();
        Store::<NamedSave>::new(root.join(SAVES_DIR)).load(recording.start_save.as_deref().unwrap()).unwrap()
    }

    #[test]
    fn placed_start_becomes_the_start_save() {
        use crate::model::state::record::StartState as PlacedState;
        let _hold = hold();
        let saved = placed_save("placed-save");
        assert_eq!((saved.player.x, saved.player.y, saved.player.percent, saved.player.situation_kind, saved.player.start_state),
                   (-30.0, 4.0, 80.0, 0, PlacedState::MissedTechUp));
        assert_eq!((saved.cpu.x, saved.cpu.y, saved.cpu.percent, saved.cpu.situation_kind, saved.cpu.start_state),
                   (40.0, -8.0, 45.0, SITUATION_AIR, PlacedState::Tumble));
    }

    #[test]
    fn placed_start_keeps_each_fighters_facing() {
        let _hold = hold();
        let saved = placed_save("placed-facing");
        assert_eq!((saved.player.lr, saved.cpu.lr), (-1.0, 1.0));
    }

    #[test]
    fn the_start_gate_keeps_every_playback_frame_back() {
        let _hold = hold();
        reset_host();
        let p1 = InputFrame { buttons: 1, ..InputFrame::default() };
        let cpu = InputFrame { buttons: 2, ..InputFrame::default() };
        assert!(PLAYER_FRAMES.push(pack(p1)));
        assert!(CPU_FRAMES.push(pack(cpu)));
        start_gate(true);
        assert!(start_gated());
        assert_eq!(take_player_frame(), None);
        assert_eq!(take_cpu_frame(), None);
        start_gate(false);
        assert!(!start_gated());
        assert_eq!(take_player_frame(), Some(p1));
        assert_eq!(take_cpu_frame(), Some(cpu));
    }

    #[test]
    fn played_frames_count_from_the_start_gate() {
        let _hold = hold();
        reset_host();
        let frame = InputFrame { buttons: 1, ..InputFrame::default() };
        assert!(PLAYER_FRAMES.push(pack(frame)));
        assert!(CPU_FRAMES.push(pack(frame)));
        start_gate(true);
        assert_eq!(played_frames(), 0);
        assert_eq!(take_player_frame(), None);
        assert_eq!(take_cpu_frame(), None);
        assert_eq!(played_frames(), 0);
        start_gate(false);
        WARP_HOLD.store(true, Ordering::Release);
        assert_eq!(take_player_frame(), None);
        assert_eq!(played_frames(), 0);
        WARP_HOLD.store(false, Ordering::Release);
        assert_eq!(take_player_frame(), Some(frame));
        assert_eq!(played_frames(), 1);
        assert_eq!(take_cpu_frame(), Some(frame));
        assert_eq!(played_frames(), 2);
        assert_eq!(take_cpu_frame(), None);
        assert_eq!(played_frames(), 2);
        start_gate(true);
        assert_eq!(played_frames(), 0);
    }

    #[test]
    fn captured_frames_follows_push_capture() {
        let _hold = hold();
        reset_host();
        assert_eq!(captured_frames(), 0);
        CAP_ROLE.store(ROLE_PLAYER, Ordering::Release);
        push_capture(ROLE_CPU, InputFrame::default());
        assert_eq!(captured_frames(), 0);
        push_capture(ROLE_PLAYER, InputFrame::default());
        assert_eq!(captured_frames(), 1);
        push_capture(ROLE_PLAYER, InputFrame::default());
        assert_eq!(captured_frames(), 2);
    }

    #[test]
    fn start_ready_waits_for_a_running_roles_first_frame() {
        let _hold = hold();
        reset_host();
        assert!(start_ready());
        RUN_PLAYER.store(true, Ordering::Release);
        assert!(!start_ready());
        assert!(PLAYER_FRAMES.push(pack(InputFrame::default())));
        assert!(start_ready());
        RUN_CPU.store(true, Ordering::Release);
        assert!(!start_ready());
        assert!(CPU_FRAMES.push(pack(InputFrame::default())));
        assert!(start_ready());
    }

    #[test]
    fn abort_warp_retires_the_waiting_play_before_the_gate_opens() {
        let _hold = hold();
        reset_host();
        let root = scratch("abort-warp");
        service_at(&root, &none(), "2026-11-22", state(8), state(7));
        let save = Store::<NamedSave>::new(root.join(SAVES_DIR))
            .save_new(&NamedSave::new("Run", state(8), state(7)).unwrap()).unwrap();
        let mut start = StartState::neutral(8, 0, 1.0);
        start.speed_x = 1.5;
        let mut recording = MacroRecording::new("Run", "13.0.5", start,
            vec![InputFrame { buttons: 4, ..InputFrame::default() }]).unwrap();
        recording.set_start_save(Some(&save)).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        RecordingIndex::update(&root, |index| { index.set_active(Role::Player, Some(&id), 1)?; Ok(true) }).unwrap();
        on_chords(false, true, false);
        assert_eq!(service_at(&root, &none(), "2026-11-22", state(8), state(7)).warp.as_deref(), Some(save.as_str()));
        start_gate(true);
        warp_landed();
        service_at(&root, &none(), "2026-11-22", state(8), state(7));
        assert!(!PLAYER_FRAMES.is_empty());
        assert!(CPU_FRAMES.push(pack(InputFrame { buttons: 9, ..InputFrame::default() })));
        assert!(START_OWED.load(Ordering::Acquire) != 0);
        assert!(take_player_frame().is_none());
        abort_warp();
        assert!(!start_gated());
        assert!(take_player_frame().is_none());
        assert!(take_cpu_frame().is_none());
        assert!(take_start_speed(Role::Player).is_none());
        service_at(&root, &none(), "2026-11-22", state(8), state(7));
        assert!(take_player_frame().is_none());
        assert!(take_cpu_frame().is_none());
        assert!(take_start_speed(Role::Player).is_none());
    }

    #[test]
    fn a_raised_ground_start_plays_as_standard() {
        use crate::model::state::record::StartState as PlacedState;
        for start_state in [PlacedState::MissedTechUp, PlacedState::MissedTechDown, PlacedState::Trip] {
            let mut player = state(8);
            player.start_state = start_state;
            let pair = warp_pair(NamedSave::new("Run", player.clone(), state(7)).unwrap(),
                Some((Role::Player, Placement { delta_y: 3.0, ..Placement::default() })));
            assert_eq!(pair[0].situation_kind, SITUATION_AIR);
            assert_eq!(pair[0].start_state, PlacedState::Standard);
            let flat = warp_pair(NamedSave::new("Run", player, state(7)).unwrap(),
                Some((Role::Player, Placement { delta_x: 3.0, ..Placement::default() })));
            assert_eq!(flat[0].start_state, start_state);
        }
    }

    /// RECORDING-REDESIGN "Start save: the save state this macro starts at", and PLAY-MOMENTUM (primary 2026-11-08
    /// 23:03Z, reading (b)): the start save is where Record started, not where it stopped (the worker wrote the live pose
    /// of the stop), and the macro keeps the recorded role's speed and facing at that moment (it stored zero and 1.0).
    fn a_recording_keeps_where_and_how_fast_it_started() {
        let _hold = hold();
        reset_host();
        let root = scratch("rec-start");
        set_record_role(Role::Player);
        note_record_start(RecordStart { poses: [pose(13, -30.0, -1.0), pose(7, 40.0, 1.0)], speeds: [(-1.75, 0.0), (9.0, 9.0)] });
        on_chords(true, false, false);
        push_capture(ROLE_PLAYER, InputFrame { buttons: 4, ..InputFrame::default() });
        on_chords(true, false, false);
        let mut stopped = state(13);
        stopped.x = 25.0;
        let _ = service_at(&root, &none(), "2026-11-08", stopped, state(7));
        let macros = Library::new(root.join(MACROS_DIR));
        let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
        let id = macros.list().unwrap().entries[0].id.clone();
        let recording = macros.load(&id).unwrap();
        assert_eq!((recording.start.fighter_kind, recording.start.speed_x, recording.start.speed_y), (13, -1.75, 0.0));
        assert_eq!(recording.start.facing, -1.0, "the recorded role's facing, which the speed is relative to");
        let save = saves.load(recording.start_save.as_deref().expect("a start save")).unwrap();
        assert_eq!((save.player.x, save.player.lr, save.cpu.x), (-30.0, -1.0, 40.0), "the start save is where Record started");

        // A CPU-role recording keeps the CPU's speed.
        set_record_role(Role::Cpu);
        note_record_start(RecordStart { poses: [pose(13, -5.0, 1.0), pose(7, 5.0, -1.0)], speeds: [(9.0, 9.0), (2.5, -1.0)] });
        on_chords(true, false, false);
        push_capture(ROLE_CPU, InputFrame { buttons: 8, ..InputFrame::default() });
        on_chords(true, false, false);
        let _ = service_at(&root, &none(), "2026-11-08", state(13), state(7));
        let cpu = macros.list().unwrap().entries.iter().map(|e| macros.load(&e.id).unwrap()).find(|r| r.start.fighter_kind == 7)
            .expect("the CPU's macro");
        assert_eq!((cpu.start.speed_x, cpu.start.speed_y, cpu.start.facing), (2.5, -1.0, -1.0));
        let save = saves.load(cpu.start_save.as_deref().unwrap()).unwrap();
        assert_eq!((save.player.x, save.cpu.x), (-5.0, 5.0));
    }

    /// Combined run rc15-ib-1f7e0174-yoga-main-1 (rv-integration REVIEW FAIL 2026-11-23 03:32:38Z): with Save States Off
    /// the fighter frame returned before it published the role's live pose, so the worker's live pose stayed the previous
    /// CPU's (Inkling) while the live CPU was Pikachu. The frame publishes whatever Save States says.
    fn the_live_pose_is_published_with_save_states_off() {
        let _hold = hold();
        reset_host();
        assert!(fighter_frame(Some((0, pose(0, 11.3, -1.0))), true));
        assert!(fighter_frame(Some((1, StartPose { percent: 72.0, ..pose(52, 16.5, 1.0) })), true));
        let mario = StartPose { percent: 84.3, ..pose(0, 79.9, 1.0) };
        let pikachu = pose(8, 47.95, 1.0);
        assert!(!fighter_frame(Some((0, mario)), false), "Save States Off: the save-state step does not run");
        assert!(!fighter_frame(Some((1, pikachu)), false));
        assert!(!fighter_frame(None, false), "a body that is not the role's own (Nana) publishes nothing");
        assert_eq!([live_pose(0), live_pose(1)], [mario, pikachu], "the live pose is published whatever Save States says");
    }

    /// rv-preview-vis REVIEW FAIL 2026-11-23 04:24:30Z on the same defect: a published pose is the role's last published
    /// frame, not the fighter standing there now. Record reads both role bodies at its chord (`RecordStart::at_chord`), so
    /// a CPU take names the live CPU in the macro's start and in its start save even while the published pose is still
    /// the previous CPU's.
    fn a_take_starts_at_the_bodies_read_at_the_chord() {
        let _hold = hold();
        reset_host();
        let root = scratch("rec-chord-bodies");
        // The published poses: Mario, and the previous CPU (Inkling) where it last stood.
        fighter_frame(Some((0, pose(0, 11.3, -1.0))), true);
        fighter_frame(Some((1, StartPose { percent: 72.0, ..pose(52, 16.5, 1.0) })), true);
        // The CPU is Pikachu now and no frame of it has published. The Record chord reads the two bodies.
        let mario = StartPose { percent: 84.3, ..pose(0, 79.9, 1.0) };
        let pikachu = pose(8, 47.95, -1.0);
        set_record_role(Role::Cpu);
        note_record_start(RecordStart::at_chord([Some((mario, (0.0, 0.0))), Some((pikachu, (0.4, 0.0)))]));
        on_chords(true, false, false);
        push_capture(ROLE_CPU, InputFrame { buttons: 513, ..InputFrame::default() });
        on_chords(true, false, false);
        // The worker's pass still holds the published poses.
        let _ = service_at(&root, &none(), "2026-11-23", live_pose(0).saved(), live_pose(1).saved());
        let macros = Library::new(root.join(MACROS_DIR));
        let recording = macros.load(&macros.list().unwrap().entries[0].id).unwrap();
        assert_eq!((recording.start.fighter_kind, recording.start.facing, recording.start.speed_x), (8, -1.0, 0.4), "the macro starts on the live CPU, not the published one");
        let save = Store::<NamedSave>::new(root.join(SAVES_DIR)).load(recording.start_save.as_deref().expect("a start save")).unwrap();
        assert_eq!((save.cpu.fighter_kind, save.cpu.x, save.cpu.percent, save.cpu.lr), (8, 47.95, 0.0, -1.0), "the start save keeps the live CPU and its pose");
        assert_eq!((save.player.fighter_kind, save.player.x, save.player.percent), (0, 79.9, 84.3), "and the live Player 1");
        let absent = RecordStart::at_chord([None, None]);
        assert_eq!((absent.poses[0].fighter_kind, absent.poses[1].fighter_kind, absent.speeds), (-1, -1, [(0.0, 0.0); 2]), "a role without a body is a never-seen pose at rest");
    }

    /// SP-17 (dup-audit, 2026-11-26 12:45:51Z): takes of 1,553 and 3,253 frames kept 1,200 frames of positions. The
    /// worker's pass reached the take's blocks only while other work was pending. This drives the pass the worker
    /// runs and nothing else, and reads the positions file back.
    fn a_long_take_keeps_its_positions_on_the_workers_pass_alone() {
        use crate::engine::macro_ui::scrub_track::TAKE;
        use crate::engine::macros::positions;
        const FRAMES: usize = 1_553;
        let _hold = hold();
        reset_host();
        let root = scratch("long-take");
        let _ = service_at(&root, &none(), "2026-11-26", state(8), state(7));
        let pass = || worker_tick(&root, state(8), state(7));
        let at = |index: usize| ReplayFrame { input: InputFrame::default(), start: StartState::neutral(8, 0, 1.0), x: index as f32, y: 2.0 };
        set_record_role(Role::Player);
        on_chords(true, false, false);
        assert!(pass().is_some(), "the Record edge is serviced");
        for index in 0..FRAMES {
            TAKE.live(0, &at(index));
            push_capture(ROLE_PLAYER, InputFrame { buttons: (index % 7 + 1) as u32, ..InputFrame::default() });
            // Two game frames to a worker pass. Nothing else waits, so the pass has no tick.
            if index % 2 == 1 {
                assert!(!pending(), "a take alone is not pending work (frame {index})");
                assert!(pass().is_none());
            }
        }
        on_chords(true, false, false);
        assert!(pass().is_some(), "the stop is serviced");
        let dir = root.join(MACROS_DIR);
        let macros = Library::new(&dir);
        let id = macros.list().unwrap().entries[0].id.clone();
        let recording = macros.load(&id).unwrap();
        assert_eq!(recording.frames.len(), FRAMES);
        let rows = positions::read(&dir, &id, &recording.frames).expect("a positions file");
        assert_eq!(rows.len(), FRAMES, "the positions stop before the take does");
        assert_eq!((rows[0][0].x, rows[1_200][0].x, rows[FRAMES - 1][0].x), (0.0, 1_200.0, (FRAMES - 1) as f32));
        let host = include_str!("../runtime/features/recording_host.rs");
        assert!(host.contains("if let Some(tick) = rr_host::worker_tick(root, player, cpu) {") && !host.contains("rr_host::pending()"),
            "the worker's pass has a pending test of its own in front of the take");
    }

    /// The runtime's side of the two tests above: the fighter frame hands its live pose to `fighter_frame` before the
    /// Save States Off return and keeps no pose of its own; Record reads the two role bodies the chord step resolved,
    /// not a published pose or accessor.
    fn the_runtime_publishes_before_the_off_return_and_record_reads_the_bodies() {
        let body = |src: &'static str, name: &str| {
            let at = src.find(name).unwrap_or_else(|| panic!("{name}"));
            &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
        };
        let states = include_str!("../runtime/features/states.rs");
        let frame = body(states, "pub unsafe fn on_fighter_frame(");
        let publish = frame.find("rr_host::fighter_frame(").expect("the frame publishes its live pose");
        assert!(publish < frame.find("Refused::Off").expect("Save States Off"), "the Off return comes before the publication");
        assert!(!states.contains("LIVE_KIND"), "the save states keep a live pose of their own");
        let host = include_str!("../runtime/features/recording_host.rs");
        let start = body(host, "unsafe fn record_start(");
        assert!(start.contains("rr_host::RecordStart::at_chord(") && start.contains("recorder::body_of(0)"), "Record does not read the bodies:\n{start}");
        assert!(!start.contains("live_pose") && !start.contains("fighter_accessor("), "Record reads a published pose or accessor:\n{start}");
        let recorder = include_str!("../runtime/features/recorder.rs");
        assert!(body(recorder, "unsafe fn chord_step(").contains("recording_host::on_chords(body, "), "the chord step drops the CPU body it resolved");
    }

    /// Plays the Player's active macro (recorded with `start`, kept `placement` if any) once and returns the start speed
    /// the role gets on the game frame that takes its first input, checking it is given only once.
    fn momentum_play(tag: &str, start: StartState, placement: Option<Placement>, start_save: bool) -> Option<StartSpeed> {
        reset_host();
        let root = scratch(tag);
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        let save = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&NamedSave::new("Run", state(8), state(7)).unwrap()).unwrap();
        let frames = vec![InputFrame { buttons: 4, ..InputFrame::default() }, InputFrame { buttons: 5, ..InputFrame::default() }];
        let mut recording = MacroRecording::new("Run", "13.0.5", start, frames).unwrap();
        if start_save {
            recording.set_start_save(Some(&save)).unwrap();
        }
        if let Some(placement) = placement {
            recording.set_placement(placement).unwrap();
        }
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&id), 1).unwrap();
        index.save(&root).unwrap();
        on_chords(false, true, false);
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert!(take_start_speed(Role::Player).is_none(), "{tag}: nothing before the run's first input");
        warp_landed();
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7));
        assert_eq!(take_player_frame().expect("the first input").buttons, 4);
        let given = take_start_speed(Role::Player);
        assert!(take_start_speed(Role::Player).is_none(), "{tag}: once per Play");
        assert_eq!(take_player_frame().expect("the second input").buttons, 5);
        assert!(take_start_speed(Role::Player).is_none(), "{tag}: not again on a later frame");
        given
    }

    /// PLAY-MOMENTUM (primary 2026-11-08 23:03Z, reading (b); cov-rec #70): Play applies the macro's recorded start
    /// speeds when the Placement screen's Momentum key reads "Keep recorded speed", and starts at rest with "Start at rest"
    /// (an explicit zero, as the named layer's `ready_velocity`: drift during the restore is not kept). It goes with the
    /// warp to the start save, as the placement offset does; with no start save Play plays from where the fighters are.
    fn play_gives_the_recorded_start_speed_only_with_momentum_kept() {
        let _hold = hold();
        let mut running = StartState::neutral(8, 0, -1.0);
        running.speed_x = 1.6;
        running.speed_y = 0.4;
        assert_eq!(momentum_play("momentum-keep", running, None, true), Some(StartSpeed { speed: (1.6, 0.4), facing: -1.0 }));
        let rest = Placement { momentum: super::super::macros::MomentumMode::AtRest, ..Placement::default() };
        assert_eq!(momentum_play("momentum-rest", running, Some(rest), true), Some(StartSpeed { speed: (0.0, 0.0), facing: -1.0 }));
        assert_eq!(momentum_play("momentum-nowarp", running, None, false), None, "no start save, no warp: nothing is set");
    }

    /// rc.14 recfix: a macro's or a chain's screen, Start save, Left or Right. The switch goes into the file (the worker's
    /// steps: the press, `post_effect(take_effect())`, `service_at`), a reload of the catalog reads it back, and Play reads
    /// it from the file: Off plays from where the fighters stand, On warps to the start save first. Eden rc.13 gif-lane-4:
    /// Play with the switch Off still warped, because the menu only changed its own copy of the catalog.
    fn the_start_save_switch_reaches_the_file_and_the_play() {
        use crate::model::menu::input::Button as MenuButton;
        use crate::model::menu::table::SAVES_MACROS;
        use crate::model::recording_menu::{Face, RecordingScreens};
        let _hold = hold();
        for chain in [false, true] {
            let root = scratch(if chain { "start-switch-chain" } else { "start-switch-macro" });
            let work = |screens: &mut RecordingScreens| {
                post_effect(screens.take_effect());
                let _ = service_at(&root, &none(), "2026-11-14", state(8), state(7));
            };
            let reload = |screens: &mut RecordingScreens| screens.set_catalog_bundle(super::super::rr_runtime::load_catalog(&root).unwrap());
            let file_switch = |id: &str| match chain {
                true => Store::<Chain>::new(root.join(MACROS_DIR)).load(id).unwrap().start_save_on,
                false => Library::new(root.join(MACROS_DIR)).load(id).unwrap().start_save_on,
            };
            let play = |warp: Option<&str>, tag: &str| {
                on_chords(false, true, false);
                let tick = service_at(&root, &none(), "2026-11-14", state(8), state(7));
                assert_eq!(tick.warp.as_deref(), warp, "{tag}: Play warps only with the switch On");
                assert_eq!(tick.diags.iter().any(|line| line == "screen=playback role=player warp=1"), warp.is_some(), "{tag}");
                if warp.is_none() {
                    assert_eq!(take_player_frame().expect("plays at once").buttons, 4, "{tag}");
                }
            };
            reset_host();
            let _ = service_at(&root, &none(), "2026-11-14", state(8), state(7));
            let save = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&NamedSave::new("Run", state(8), state(7)).unwrap()).unwrap();
            let step = one_macro(&root, "Run", 4, 8, Some(&save));
            let id = match chain {
                true => Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&Chain::new("Chain 1", vec![step]).unwrap()).unwrap(),
                false => step,
            };
            let mut index = RecordingIndex::load(&root).unwrap();
            index.set_active(Role::Player, Some(&id), 1).unwrap();
            index.save(&root).unwrap();
            let mut screens = RecordingScreens::new();
            reload(&mut screens);
            let at = screens.section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == if chain { "Chain 1" } else { "Run" }).unwrap();
            screens.set_cursor(Face::Page, SAVES_MACROS, 1, at);
            screens.handle_page(MenuButton::A, true, SAVES_MACROS);
            assert_eq!(screens.file_rows()[0].title, "Start save");
            assert!(file_switch(&id), "a new file starts On");

            screens.handle_page(MenuButton::Left, true, SAVES_MACROS);
            work(&mut screens);
            assert!(!file_switch(&id), "Off is written to the file");
            reload(&mut screens);
            assert_eq!(screens.file_rows()[0].value, "Off", "and survives a reload of the catalog");
            play(None, "Off");

            reset_host();
            let _ = service_at(&root, &none(), "2026-11-14", state(8), state(7));
            screens.handle_page(MenuButton::Right, true, SAVES_MACROS);
            work(&mut screens);
            assert!(file_switch(&id), "On is written to the file");
            reload(&mut screens);
            assert_eq!(screens.file_rows()[0].value, "Run · On", "and survives a reload of the catalog");
            play(Some(&save), "On");
        }
    }

    fn queued_rename_checks_the_originating_index_revision() {
        let _guard = hold();
        reset_host();
        let root = scratch("rename-index-version");
        // Hold off unrelated test writes while advancing only the index revision.
        let _write = store::recording_write_guard(&root);
        let version = recording_version();
        let offer = || Effect::Keyboard { header: "Rename macro", draft: "Old".into() };
        post_effect_at(offer(), Some(version));
        store::note_index_write(&root);
        assert_eq!(recording_version().0, version.0, "an index-only change");
        assert_ne!(recording_version().1, version.1);
        let book = Book::new(RecordingIndex::load(&root).unwrap());
        let mut machine = Playback::new();
        let mut tick = Tick::default();
        apply_menu(&root, &mut machine, &book, &mut tick);
        assert!(tick.keyboard.is_none(), "an obsolete Rename must not offer a keyboard");
        post_effect_at(offer(), Some(recording_version()));
        apply_menu(&root, &mut machine, &book, &mut tick);
        assert_eq!(tick.keyboard.unwrap().header, "Rename macro");
    }

    fn vault_make(root: &Path, entry: i32, role: Role, kind: crate::model::recording_menu::Make) -> MakeAsk {
        use crate::engine::macro_ui::clip_panel::{ClipAction, ClipEditor, ClipEffect};
        use crate::engine::macros::{ReplayFrame, ReplaySession, ReplayTrack};
        let track = |entry: i32, base: u32| ReplayTrack {
            fighter: None,
            entry, quantized_analog: false,
            frames: (0..3).map(|i| ReplayFrame { input: InputFrame { buttons: base + i, ..InputFrame::default() },
                start: StartState::neutral(entry + 7, 0, 1.0), x: 0.0, y: 0.0 }).collect(),
        };
        let session = ReplaySession { tracks: vec![track(0, 1), track(1, 101)], truncated: false, breaks: Vec::new() };
        let mut editor = ClipEditor::new(session, vec![0]).unwrap();
        assert!(editor.select_entry(entry));
        let ClipEffect::Export(recording) = editor.on_action(ClipAction::Confirm) else { panic!("expected exported clip") };
        let made = super::super::rr_runtime::generate(root, *recording, "Typed", kind, role, 1).unwrap();
        let super::super::rr_runtime::Generated::Pass(ask) = made else { panic!("expected pass") };
        ask
    }

    fn consume_make(root: &Path, role: Role, hit: bool) -> Vec<u32> {
        let mut received = Vec::new();
        for frame in 1..=8 {
            service_at(root, &none(), "2026-11-24", state(8), state(7));
            let got = if role == Role::Player {
                assert!(take_cpu_frame().is_none());
                let mut input = MappedInputs::NEUTRAL;
                unsafe { on_player_poll(&mut input) }.then_some(to_frame(input))
            } else {
                assert!(take_player_frame().is_none());
                take_cpu_frame()
            };
            if let Some(input) = got {
                received.push(input.buttons);
                observe_input(Snap { buttons: input.buttons, facing: 1, ..Snap::empty(frame) }, Late::NONE, crate::model::combo::Settings::factory().tolerance, hit);
                if hit { set_strip_sample((1, Some(sample_combo()), CaptureOwner::UNKNOWN)); }
            }
        }
        received
    }

    fn capture_session() -> crate::engine::macros::ReplaySession {
        use crate::engine::macros::{ReplayFrame, ReplaySession, ReplayTrack};
        let track = |entry: i32, base: u32| ReplayTrack {
            fighter: None,
            entry, quantized_analog: false,
            frames: (0..3).map(|i| ReplayFrame { input: InputFrame { buttons: base + i, ..InputFrame::default() },
                start: StartState::neutral(entry + 7, 0, 1.0), x: 0.0, y: 0.0 }).collect(),
        };
        ReplaySession { tracks: vec![track(0, 1), track(1, 101)], truncated: false, breaks: Vec::new() }
    }

    fn the_host_deletes_a_saved_capture_through_the_file_delete() {
        use crate::model::recording_menu::FileKind;
        let _hold = hold();
        reset_host();
        let root = scratch("capture-delete");
        // The folder the runtime's archive reads (`replay_store::archive`).
        assert!(include_str!("../runtime/macros/replay_store.rs").contains("\"{}/replay_captures\""));
        let archive = crate::engine::macros::CaptureArchive::new(root.join("replay_captures"));
        let id = archive.save("native", "Match", "13.0.5", &capture_session()).unwrap();
        let kept = archive.save("other", "Other", "13.0.5", &capture_session()).unwrap();
        post_effect(Effect::DeleteFile { kind: FileKind::Capture, id: format!("saved:{id}") });
        service_at(&root, &none(), "2026-11-25", state(8), state(7));
        let left = archive.list().unwrap().entries;
        assert_eq!(left.iter().map(|entry| entry.id.as_str()).collect::<Vec<_>>(), [kept.as_str()]);
        // A Vault capture of this boot has no file, and a bare id is not a saved capture's: nothing is removed.
        for id in ["latest".to_owned(), "previous".to_owned(), kept.clone()] {
            post_effect(Effect::DeleteFile { kind: FileKind::Capture, id });
            service_at(&root, &none(), "2026-11-25", state(8), state(7));
            assert_eq!(archive.list().unwrap().entries.len(), 1);
        }
    }

    fn a_macro_made_from_a_capture_outlives_its_delete() {
        use crate::engine::macro_ui::clip_panel::{ClipAction, ClipEditor, ClipEffect};
        use crate::model::recording_menu::{FileKind, Make};
        let _hold = hold();
        reset_host();
        let root = scratch("capture-made");
        let archive = crate::engine::macros::CaptureArchive::new(root.join("replay_captures"));
        let id = archive.save("native", "Match", "13.0.5", &capture_session()).unwrap();
        let mut editor = ClipEditor::new(archive.load(&id).unwrap().session, vec![0]).unwrap();
        assert!(editor.select_entry(0));
        let ClipEffect::Export(recording) = editor.on_action(ClipAction::Confirm) else { panic!("expected exported clip") };
        let made = super::super::rr_runtime::generate(&root, *recording, "From capture", Make::Macro, Role::Player, 1).unwrap();
        let super::super::rr_runtime::Generated::Macro(made) = made else { panic!("expected a macro") };
        // One pass first: the host's own first pass settles the index (its migration mark).
        service_at(&root, &none(), "2026-11-25", state(8), state(7));
        let before = RecordingIndex::load(&root).unwrap();
        post_effect(Effect::DeleteFile { kind: FileKind::Capture, id: format!("saved:{id}") });
        service_at(&root, &none(), "2026-11-25", state(8), state(7));
        assert!(archive.list().unwrap().entries.is_empty(), "the capture is gone");
        assert_eq!(RecordingIndex::load(&root).unwrap(), before, "captures are not in the index: the delete changes nothing in it");
        let kept = Library::new(root.join(MACROS_DIR)).load(&made).unwrap();
        assert_eq!(kept.name, "From capture");
        assert_eq!(kept.frames.iter().map(|frame| frame.buttons).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(RecordingIndex::load(&root).unwrap().active(Role::Player), Some(made.as_str()), "and it is still the active pick");
    }

    fn each_source_and_target_makes_a_combo_on_that_role() {
        let _hold = hold();
        for entry in [0, 1] {
            for role in [Role::Player, Role::Cpu] {
                reset_host();
                let root = scratch("vault-combo");
                let ask = vault_make(&root, entry, role, crate::model::recording_menu::Make::Combo);
                let source = ask.source.clone();
                request_make(ask);
                let base = if entry == 0 { 1 } else { 101 };
                assert_eq!(consume_make(&root, role, true), [base, base + 1, base + 2]);
                let lib = crate::model::combo_record::ComboLibrary::new(root.join("combos"));
                let list = lib.list().unwrap().entries;
                assert_eq!(list.len(), 1);
                let combo = lib.load(&list[0].id).unwrap();
                assert!(!combo.combo.input_only);
                assert_eq!(combo.made_from.as_deref(), Some(source.as_str()));
            }
        }
    }

    fn each_source_and_target_makes_a_drill_on_that_role() {
        let _hold = hold();
        for entry in [0, 1] {
            for role in [Role::Player, Role::Cpu] {
                reset_host();
                let root = scratch("vault-drill");
                let ask = vault_make(&root, entry, role, crate::model::recording_menu::Make::Drill);
                let source = ask.source.clone();
                request_make(ask);
                let base = if entry == 0 { 1 } else { 101 };
                assert_eq!(consume_make(&root, role, false), [base, base + 1, base + 2]);
                let lib = crate::model::combo_record::ComboLibrary::new(root.join("combos"));
                let list = lib.list().unwrap().entries;
                assert_eq!(list.len(), 1);
                let drill = lib.load(&list[0].id).unwrap();
                assert!(drill.combo.input_only);
                assert_eq!(drill.name, "Typed");
                assert_eq!(drill.take.as_deref(), Some(source.as_str()));
            }
        }
    }

    fn make_drill_saves_a_drill_even_when_the_pass_hits() {
        let _hold = hold();
        reset_host();
        let root = scratch("vault-drill-hit");
        let ask = vault_make(&root, 0, Role::Player, crate::model::recording_menu::Make::Drill);
        request_make(ask);
        assert_eq!(consume_make(&root, Role::Player, true), [1, 2, 3]);
        let lib = crate::model::combo_record::ComboLibrary::new(root.join("combos"));
        let list = lib.list().unwrap().entries;
        assert_eq!(list.len(), 1);
        assert!(lib.load(&list[0].id).unwrap().combo.input_only);
    }

    fn a_cpu_pass_gives_the_strip_the_cpu_as_the_acting_fighter() {
        let _hold = hold();
        reset_host();
        let root = scratch("vault-cpu-actor");
        let ask = vault_make(&root, 1, Role::Cpu, crate::model::recording_menu::Make::Combo);
        request_make(ask);
        service_at(&root, &none(), "2026-11-24", state(8), state(7));
        assert!(make_actor_cpu());
        assert_eq!(crate::model::frame_strip::acting(true), [1, 0]);
        let frame = take_cpu_frame().unwrap();
        assert_eq!(make_cpu_inputs(), to_mapped(frame));
        observe_input(Snap { buttons: frame.buttons, facing: 1, ..Snap::empty(1) }, Late::NONE, crate::model::combo::Settings::factory().tolerance, false);
        consume_make(&root, Role::Cpu, false);
        assert!(!make_actor_cpu());
        assert_eq!(crate::model::frame_strip::acting(false), [0, 1]);
        let ask = vault_make(&root, 0, Role::Player, crate::model::recording_menu::Make::Combo);
        request_make(ask);
        service_at(&root, &none(), "2026-11-24", state(8), state(7));
        assert!(!make_actor_cpu());
        assert!(include_str!("../runtime/features/input_display.rs").contains("frame_strip::acting("));
    }

    #[test]
    fn a_tap_then_x_before_service_retires_the_old_preview_before_restarting() {
        let _hold = hold();
        for (old, next) in [(Role::Player, Role::Cpu), (Role::Cpu, Role::Player)] {
            reset_host();
            let root = scratch("preview-restart");
            let tick = || service_at(&root, &none(), "2026-11-25", state(8), state(7));
            let clip = |buttons: u32| MacroRecording::new("Clip", "13.0.5", StartState::neutral(7, 0, 1.0),
                vec![InputFrame { buttons, ..InputFrame::default() }; 8]).unwrap();
            let take = |role| if role == Role::Player { take_player_frame() } else { take_cpu_frame() };
            request_clip_preview(clip(11), old);
            assert_eq!(tick().home, Some(HomeWarp::Start { air: false, player: old == Role::Player }));
            assert!(land_pair([state(8), state(7)], false));
            tick();
            assert_eq!(take(old).unwrap().buttons, 11);
            // Two controller events can be drained before the recording worker services its requests.
            stop_preview();
            request_clip_preview(clip(23), next);
            assert_eq!(tick().home, Some(HomeWarp::Return), "lost the stop on {old:?}");
            assert!(take(old).is_none() && take(next).is_none());
            assert_eq!(tick().home, None, "start overtook the unfinished return");
            assert!(land_pair([state(8), state(7)], false));
            let mut home = tick().home;
            if home.is_none() { home = tick().home; }
            assert_eq!(home, Some(HomeWarp::Start { air: false, player: next == Role::Player }));
            assert!(land_pair([state(8), state(7)], false));
            tick();
            assert_eq!(take(next).unwrap().buttons, 23);
            assert!(take(old).is_none(), "old fighter still got a frame");
            stop_preview();
            assert_eq!(tick().home, Some(HomeWarp::Return));
            request_clip_preview(clip(33), old);
            assert_eq!(tick().home, None);
            stop_preview(); // Closing the editor must cancel the deferred X as well.
            assert!(land_pair([state(8), state(7)], false));
            assert_eq!(tick().home, None);
            assert!(!previewing() && !pending());
        }
    }

    fn a_vault_clip_previews_on_player_one_or_the_cpu() {
        let _hold = hold();
        for role in [Role::Player, Role::Cpu] {
            reset_host();
            let root = scratch("vault-role-preview");
            let mut start = StartState::neutral(7, 0, 1.0);
            start.situation = SITUATION_AIR;
            let frames = vec![InputFrame { buttons: 101, ..InputFrame::default() }];
            let recording = MacroRecording::new("Clip", "13.0.5", start, frames.clone()).unwrap();
            request_clip_preview(recording, role);
            let tick = service_at(&root, &none(), "2026-11-24", state(8), state(7));
            assert_eq!(tick.home, Some(HomeWarp::Start { air: true, player: role == Role::Player }));
            assert_eq!(crate::model::state::preview::playing_index(role == Role::Player), usize::from(role == Role::Cpu));
            assert!(land_pair([state(8), state(7)], false));
            service_at(&root, &none(), "2026-11-24", state(8), state(7));
            let (player, cpu) = (take_player_frame(), take_cpu_frame());
            assert_eq!(if role == Role::Player { player } else { cpu }, Some(frames[0]));
            assert_eq!(if role == Role::Player { cpu } else { player }, None);
        }
    }

    #[test]
    fn a_vault_clip_preview_is_exact_and_shows_its_role() {
        let _hold = hold();
        for role in [Role::Player, Role::Cpu] {
            reset_host();
            let root = scratch("vault-trim-preview");
            let tick = || service_at(&root, &none(), "2026-11-25", state(8), state(7));
            let frames: Vec<_> = [0, 1, 2, 0].into_iter()
                .map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
            let mut clip = MacroRecording::new("Clip", "13.0.5", StartState::neutral(7, 0, 1.0), frames.clone()).unwrap();
            clip.trim = Trim::Both;
            request_clip_preview(clip, role);
            assert_eq!(tick().home, Some(HomeWarp::Start { air: false, player: role == Role::Player }));
            assert!(land_pair([state(8), state(7)], false));
            for expected in &frames {
                tick();
                assert_eq!(playback_roles(), [role == Role::Player, role == Role::Cpu]);
                let (player, cpu) = (take_player_frame(), take_cpu_frame());
                assert_eq!(if role == Role::Player { player } else { cpu }, Some(*expected));
                assert_eq!(if role == Role::Player { cpu } else { player }, None);
            }
            tick();
            assert_eq!(playback_roles(), [false, false]);
            assert!(!preview_running() && preview_away());

            let idle = MacroRecording::new("Idle", "13.0.5", StartState::neutral(7, 0, 1.0), vec![InputFrame::default(); 4]).unwrap();
            let id = Library::new(root.join(MACROS_DIR)).save_new(&idle).unwrap();
            stop_preview();
            assert!(request_drill_preview(&id, 4, Trim::Both));
            assert_eq!(tick().home, Some(HomeWarp::Return));
            assert!(land_pair([state(8), state(7)], false));
            tick();
            tick();
            assert!(!preview_running() && !preview_away());
            assert_eq!(playback_roles(), [false, false]);
            assert!(take_player_frame().is_none() && take_cpu_frame().is_none());
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("an_owed_slot_does_not_show_the_playback_indicator", an_owed_slot_does_not_show_the_playback_indicator as fn()),
            ("the_host_deletes_a_saved_capture_through_the_file_delete", the_host_deletes_a_saved_capture_through_the_file_delete as fn()),
            ("a_macro_made_from_a_capture_outlives_its_delete", a_macro_made_from_a_capture_outlives_its_delete as fn()),
            ("each_source_and_target_makes_a_combo_on_that_role", each_source_and_target_makes_a_combo_on_that_role as fn()),
            ("each_source_and_target_makes_a_drill_on_that_role", each_source_and_target_makes_a_drill_on_that_role as fn()),
            ("make_drill_saves_a_drill_even_when_the_pass_hits", make_drill_saves_a_drill_even_when_the_pass_hits as fn()),
            ("a_cpu_pass_gives_the_strip_the_cpu_as_the_acting_fighter", a_cpu_pass_gives_the_strip_the_cpu_as_the_acting_fighter as fn()),
            ("a_vault_clip_previews_on_player_one_or_the_cpu", a_vault_clip_previews_on_player_one_or_the_cpu as fn()),
            ("a_playback_slot_run_plays_its_macro_through_the_cpu_pad", a_playback_slot_run_plays_its_macro_through_the_cpu_pad as fn()),
            ("a_play_on_cpu_frame_ends_a_slot_run", a_play_on_cpu_frame_ends_a_slot_run as fn()),
            ("a_slot_waits_for_a_running_cpu_macro", a_slot_waits_for_a_running_cpu_macro as fn()),
            ("a_changed_slot_ends_its_run", a_changed_slot_ends_its_run as fn()),
            ("an_empty_or_long_slot_is_refused", an_empty_or_long_slot_is_refused as fn()),
            ("the_r4b_holds_turn_three_times_in_a_slot", the_r4b_holds_turn_three_times_in_a_slot as fn()),
            ("a_slot_start_defers_while_its_slot_is_published", a_slot_start_defers_while_its_slot_is_published as fn()),
            ("a_replaced_take_is_not_started_under_the_old_takes_gate", a_replaced_take_is_not_started_under_the_old_takes_gate as fn()),
            ("one_ledge_request_through_a_publication_starts_exactly_one_run_with_its_buffer", one_ledge_request_through_a_publication_starts_exactly_one_run_with_its_buffer as fn()),
            ("a_deferred_ledge_request_denies_an_immediate_climb_query", a_deferred_ledge_request_denies_an_immediate_climb_query as fn()),
            ("a_roles_fed_count_is_the_frames_its_hook_took", a_roles_fed_count_is_the_frames_its_hook_took as fn()),
            ("a_count_is_not_paired_with_a_run_number_while_a_load_stores", a_count_is_not_paired_with_a_run_number_while_a_load_stores as fn()),
            ("a_late_credit_of_the_old_run_is_not_counted_for_the_new_one", a_late_credit_of_the_old_run_is_not_counted_for_the_new_one as fn()),
            ("a_crop_preview_owes_the_start_speed_as_play_does", a_crop_preview_owes_the_start_speed_as_play_does as fn()),
            ("a_vault_clip_preview_plays_on_the_cpu_and_puts_both_fighters_back", a_vault_clip_preview_plays_on_the_cpu_and_puts_both_fighters_back as fn()),
            ("a_cpu_crop_preview_holds_right_whatever_the_facing", a_cpu_crop_preview_holds_right_whatever_the_facing as fn()),
            ("a_named_load_is_taken_with_save_states_off", a_named_load_is_taken_with_save_states_off as fn()),
            ("a_linked_crop_preview_starts_with_save_states_off", a_linked_crop_preview_starts_with_save_states_off as fn()),
            ("an_unreadable_index_refuses_a_new_preview_but_does_not_prevent_a_vault_return", an_unreadable_index_refuses_a_new_preview_but_does_not_prevent_a_vault_return as fn()),
            ("a_session_end_drops_the_preview_home", a_session_end_drops_the_preview_home as fn()),
            ("a_refused_vault_preview_says_why_and_ends", a_refused_vault_preview_says_why_and_ends as fn()),
            ("a_crop_preview_plays_its_window_on_the_role_through_the_play_queue", a_crop_preview_plays_its_window_on_the_role_through_the_play_queue as fn()),
            ("stopping_a_crop_preview_drops_the_rest", stopping_a_crop_preview_drops_the_rest as fn()),
            ("a_crop_preview_of_a_missing_macro_ends_at_once", a_crop_preview_of_a_missing_macro_ends_at_once as fn()),
            ("queued_rename_checks_the_originating_index_revision", queued_rename_checks_the_originating_index_revision as fn()),
            ("posted_chords_reach_the_role_machines", posted_chords_reach_the_role_machines as fn()),
            ("a_taken_pair_stays_pending_until_its_write_is_done", a_taken_pair_stays_pending_until_its_write_is_done as fn()),
            ("the_start_save_switch_reaches_the_file_and_the_play", the_start_save_switch_reaches_the_file_and_the_play as fn()),
            ("a_posted_record_stores_the_start_save", a_posted_record_stores_the_start_save as fn()),
            ("migration_from_the_worker_runs_once", migration_from_the_worker_runs_once as fn()),
            ("a_posted_menu_effect_reaches_the_worker", a_posted_menu_effect_reaches_the_worker as fn()),
            ("an_empty_posted_record_keeps_the_empty_notice", an_empty_posted_record_keeps_the_empty_notice as fn()),
            ("the_cpu_inject_slot_starts_empty", the_cpu_inject_slot_starts_empty as fn()),
            ("record_follows_the_published_cpu_role", record_follows_the_published_cpu_role as fn()),
            ("a_cold_start_records_on_the_saved_popup_role", a_cold_start_records_on_the_saved_popup_role as fn()),
            ("the_open_take_is_the_capture_role", the_open_take_is_the_capture_role as fn()),
            ("player_one_is_tinted_only_during_its_own_take", player_one_is_tinted_only_during_its_own_take as fn()),
            ("cpu_role_lockout_or_standby_press_is_not_stored", cpu_role_lockout_or_standby_press_is_not_stored as fn()),
            ("cpu_role_record_stores_the_jab_and_plays_it_on_the_cpu", cpu_role_record_stores_the_jab_and_plays_it_on_the_cpu as fn()),
            ("f4_cpu_role_record_write_matches_playback_of_the_stored_frame", f4_cpu_role_record_write_matches_playback_of_the_stored_frame as fn()),
            ("a_warped_cpu_play_feeds_its_first_frame_after_the_restore", a_warped_cpu_play_feeds_its_first_frame_after_the_restore as fn()),
            ("a_cpu_play_without_a_start_save_feeds_at_once", a_cpu_play_without_a_start_save_feeds_at_once as fn()),
            ("parry_each_pass_warps_to_the_start_save", parry_each_pass_warps_to_the_start_save as fn()),
            ("parry_stop_during_buffered_playback_owes_no_pass", parry_stop_during_buffered_playback_owes_no_pass as fn()),
            ("parry_off_cancels_owed_passes", parry_off_cancels_owed_passes as fn()),
            ("parry_passes_owed_are_a_running_cpu_playback", parry_passes_owed_are_a_running_cpu_playback as fn()),
            ("parry_pass_mash_asks_to_stop_and_queues_nothing", parry_pass_mash_asks_to_stop_and_queues_nothing as fn()),
            ("parry_restart_does_not_restore_credits_canceled_by_off", parry_restart_does_not_restore_credits_canceled_by_off as fn()),
            ("parry_refuses_without_a_recording_or_its_start_save", parry_refuses_without_a_recording_or_its_start_save as fn()),
            ("a_library_macro_playing_on_the_cpu_is_the_playback_the_mash_and_covered_settings_see", a_library_macro_playing_on_the_cpu_is_the_playback_the_mash_and_covered_settings_see as fn()),
            ("a_load_stops_both_runs_but_not_a_plays_own_warp", a_load_stops_both_runs_but_not_a_plays_own_warp as fn()),
            ("the_mash_stop_is_seen_by_the_cpu_before_the_next_tick", the_mash_stop_is_seen_by_the_cpu_before_the_next_tick as fn()),
            ("a_cut_endless_run_does_not_spin_the_worker", a_cut_endless_run_does_not_spin_the_worker as fn()),
            ("the_hook_side_cuts_never_wait_on_worker_locks", the_hook_side_cuts_never_wait_on_worker_locks as fn()),
            ("the_frame_pop_never_waits_and_keeps_the_frame_on_contention", the_frame_pop_never_waits_and_keeps_the_frame_on_contention as fn()),
            ("cpu_frames_reach_the_ai_once_each_whatever_the_worker_timing", cpu_frames_reach_the_ai_once_each_whatever_the_worker_timing as fn()),
            ("player_frames_reach_the_poll_once_each_whatever_the_worker_timing", player_frames_reach_the_poll_once_each_whatever_the_worker_timing as fn()),
            ("a_stopped_cpu_run_feeds_nothing_more", a_stopped_cpu_run_feeds_nothing_more as fn()),
            ("a_warp_holds_the_buffered_cpu_frames_until_the_restore", a_warp_holds_the_buffered_cpu_frames_until_the_restore as fn()),
            ("cpu_role_player_path_still_stores_the_polls", cpu_role_player_path_still_stores_the_polls as fn()),
            ("a_record_writes_before_migration_when_the_slot_image_is_absent", a_record_writes_before_migration_when_the_slot_image_is_absent as fn()),
            ("make_combo_plays_the_named_file_and_keeps_the_drill", make_combo_plays_the_named_file_and_keeps_the_drill as fn()),
            ("make_combo_failures_notice_and_open_the_keyboard_online", make_combo_failures_notice_and_open_the_keyboard_online as fn()),
            ("a_direct_drill_keeps_its_take_as_a_macro_without_a_save_or_role_change",a_direct_drill_keeps_its_take_as_a_macro_without_a_save_or_role_change as fn()),
            ("deleting_a_drill_deletes_its_take",deleting_a_drill_deletes_its_take as fn()),
            ("a_recorded_drills_take_previews_on_player_one_through_the_play_queue",a_recorded_drills_take_previews_on_player_one_through_the_play_queue as fn()),
            ("a_drill_previews_fed_count_and_its_fed_line_follow_each_frame_handed_out", a_drill_previews_fed_count_and_its_fed_line_follow_each_frame_handed_out as fn()),
            ("an_ordinary_plays_load_and_every_frame_show_on_the_fed_line", an_ordinary_plays_load_and_every_frame_show_on_the_fed_line as fn()),
            ("a_stop_while_the_index_is_unreadable_retires_the_preview_run", a_stop_while_the_index_is_unreadable_retires_the_preview_run as fn()),
            ("x_during_a_play_on_player_one_starts_no_drill_preview",x_during_a_play_on_player_one_starts_no_drill_preview as fn()),
            ("make_preserves_an_open_record_prefix_and_physical_cstick",make_preserves_an_open_record_prefix_and_physical_cstick as fn()),
            ("direct_drill_overflow_saves_nothing_and_reopening_cancels_arm",direct_drill_overflow_saves_nothing_and_reopening_cancels_arm as fn()),
            ("make_combo_no_hit_saves_input_drill_not_previous_exchange", make_combo_no_hit_saves_input_drill_not_previous_exchange as fn()),
            ("save_last_combo_writes_the_latest_exchange_names_it_and_drills_it", save_last_combo_writes_the_latest_exchange_names_it_and_drills_it as fn()),
            ("a_player_take_stores_the_record_the_poll_was_given", a_player_take_stores_the_record_the_poll_was_given as fn()),
            ("a_cpu_role_take_forwards_the_given_record_to_the_cpu_and_leaves_player_one_neutral", a_cpu_role_take_forwards_the_given_record_to_the_cpu_and_leaves_player_one_neutral as fn()),
            ("a_take_is_busy_from_its_record_edge_until_its_stop_is_serviced", a_take_is_busy_from_its_record_edge_until_its_stop_is_serviced as fn()),
            ("mismatch_holds_playback_until_keep_or_switch", mismatch_holds_playback_until_keep_or_switch as fn()),
            ("an_answered_mismatch_wakes_the_service_gate_and_plays", an_answered_mismatch_wakes_the_service_gate_and_plays as fn()),
            ("the_mismatch_ask_names_the_side_that_plays", the_mismatch_ask_names_the_side_that_plays as fn()),
            ("a_changed_combo_drill_asks_and_switch_names_the_recorded_fighter", a_changed_combo_drill_asks_and_switch_names_the_recorded_fighter as fn()),
            ("with_ask_before_switching_off_a_play_chord_keeps_the_fighter", with_ask_before_switching_off_a_play_chord_keeps_the_fighter as fn()),
            ("with_ask_before_switching_off_a_changed_drill_keeps_the_fighter", with_ask_before_switching_off_a_changed_drill_keeps_the_fighter as fn()),
            ("rc5wire_publish_selection_writes_the_loop", rc5wire_publish_selection_writes_the_loop as fn()),
            ("a_changed_drill_installs_the_grade_target_once", a_changed_drill_installs_the_grade_target_once as fn()),
            ("a_sequence_plan_stages_its_combos_in_order_and_skips_a_deleted_one", a_sequence_plan_stages_its_combos_in_order_and_skips_a_deleted_one as fn()),
            ("a_sequence_longer_than_sixteen_combos_drills_every_combo", a_sequence_longer_than_sixteen_combos_drills_every_combo as fn()),
            ("streak_and_best_are_saved_to_the_sequence_file_and_the_next_drill_goes_on_from_them", streak_and_best_are_saved_to_the_sequence_file_and_the_next_drill_goes_on_from_them as fn()),
            ("a_rename_or_a_saved_streak_keeps_the_plan_and_a_new_order_does_not", a_rename_or_a_saved_streak_keeps_the_plan_and_a_new_order_does_not as fn()),
            ("the_one_drill_replaces_a_combo_with_a_sequence_and_back", the_one_drill_replaces_a_combo_with_a_sequence_and_back as fn()),
            ("a_numbered_row_drills_that_combo_alone_with_its_own_target_and_the_sequence_file_stays", a_numbered_row_drills_that_combo_alone_with_its_own_target_and_the_sequence_file_stays as fn()),
            ("delete_and_edit_reach_the_card", delete_and_edit_reach_the_card as fn()),
            ("the_active_slot_follows_the_bound_save_and_save_writes_it", the_active_slot_follows_the_bound_save_and_save_writes_it as fn()),
            ("edit_placement_targets_the_active_macro_and_play_warps_with_its_offset", edit_placement_targets_the_active_macro_and_play_warps_with_its_offset as fn()),
            ("a_placement_save_rewrites_only_the_edited_half_of_the_file", a_placement_save_rewrites_only_the_edited_half_of_the_file as fn()),
            ("a_save_with_nothing_edited_writes_no_file", a_save_with_nothing_edited_writes_no_file as fn()),
            ("the_bound_copy_follows_a_changed_placement", the_bound_copy_follows_a_changed_placement as fn()),
            ("a_copy_is_written_before_the_macro_is_relinked", a_copy_is_written_before_the_macro_is_relinked as fn()),
            ("a_chain_copy_keeps_the_first_macros_own_start_save_switch", a_chain_copy_keeps_the_first_macros_own_start_save_switch as fn()),
            ("a_failed_relink_leaves_the_old_link_and_no_copy", a_failed_relink_leaves_the_old_link_and_no_copy as fn()),
            ("the_replays_row_opens_the_active_macros_start_save", the_replays_row_opens_the_active_macros_start_save as fn()),
            ("the_momentum_and_offset_rows_write_only_their_field", the_momentum_and_offset_rows_write_only_their_field as fn()),
            ("switch_on_an_exact_mii_macro_asks_for_that_mii", switch_on_an_exact_mii_macro_asks_for_that_mii as fn()),
            ("a_setup_restore_leaves_the_bound_save_and_the_slot_returns_to_it", a_setup_restore_leaves_the_bound_save_and_the_slot_returns_to_it as fn()),
            ("a_setup_bound_with_y_stays_loads_target_until_the_bound_save_changes", a_setup_bound_with_y_stays_loads_target_until_the_bound_save_changes as fn()),
            ("a_recording_keeps_where_and_how_fast_it_started", a_recording_keeps_where_and_how_fast_it_started as fn()),
            ("the_live_pose_is_published_with_save_states_off", the_live_pose_is_published_with_save_states_off as fn()),
            ("a_take_starts_at_the_bodies_read_at_the_chord", a_take_starts_at_the_bodies_read_at_the_chord as fn()),
            ("a_long_take_keeps_its_positions_on_the_workers_pass_alone", a_long_take_keeps_its_positions_on_the_workers_pass_alone as fn()),
            ("the_runtime_publishes_before_the_off_return_and_record_reads_the_bodies", the_runtime_publishes_before_the_off_return_and_record_reads_the_bodies as fn()),
            ("play_gives_the_recorded_start_speed_only_with_momentum_kept", play_gives_the_recorded_start_speed_only_with_momentum_kept as fn()),
        ]);
    }

    mod generation {
        use super::*;

        /// The library version is process-wide, so no other test may write a library while these cases run.
        /// The rows run exactly as in the module's table, each on its own thread.
        #[test]
        fn case_table() {
            if std::env::var_os("TR_GENERATION_TABLE").is_none() {
                let name = format!("{}::case_table", module_path!().split_once("::").unwrap().1);
                let status = Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", &name, "--nocapture"])
                    .env("TR_GENERATION_TABLE", "1")
                    .status().unwrap();
                assert!(status.success(), "the generation cases failed in their own process");
            } else {
                crate::test_table::run(&[
                    ("an_old_macro_played_once_from_zero_gets_generated_positions", super::an_old_macro_played_once_from_zero_gets_generated_positions as fn()),
                    ("a_stopped_playback_writes_the_inputs_that_were_consumed", super::a_stopped_playback_writes_the_inputs_that_were_consumed as fn()),
                    ("an_editor_preview_from_zero_binds_the_whole_macro", super::an_editor_preview_from_zero_binds_the_whole_macro as fn()),
                    ("delay_polls_and_a_stop_before_the_first_input_keep_nothing", super::delay_polls_and_a_stop_before_the_first_input_keep_nothing as fn()),
                    ("a_longer_generated_track_replaces_a_shorter_one_only", super::a_longer_generated_track_replaces_a_shorter_one_only as fn()),
                    ("a_generated_track_never_replaces_a_recorded_one", super::a_generated_track_never_replaces_a_recorded_one as fn()),
                    ("a_short_recorded_track_heals_on_a_full_playback", super::a_short_recorded_track_heals_on_a_full_playback as fn()),
                    ("a_generated_run_with_no_partner_does_not_keep_the_last_runs_fighter", super::a_generated_run_with_no_partner_does_not_keep_the_last_runs_fighter as fn()),
                    ("an_adopted_generated_track_is_cropped_as_generated", super::an_adopted_generated_track_is_cropped_as_generated as fn()),
                    ("the_play_chord_pass_opens_and_writes_no_positions_file", super::the_play_chord_pass_opens_and_writes_no_positions_file as fn()),
                    ("nothing_is_allocated_for_generation_before_the_first_generating_play", super::nothing_is_allocated_for_generation_before_the_first_generating_play as fn()),
                    ("a_library_write_during_the_run_drops_its_track", super::a_library_write_during_the_run_drops_its_track as fn()),
                    ("a_recorded_file_written_before_the_worker_drains_is_kept", super::a_recorded_file_written_before_the_worker_drains_is_kept as fn()),
                    ("only_a_single_macro_from_input_zero_on_its_own_fighter_generates", super::only_a_single_macro_from_input_zero_on_its_own_fighter_generates as fn()),
                    ("a_loop_and_a_second_role_never_join_one_track", super::a_loop_and_a_second_role_never_join_one_track as fn()),
                    ("a_finished_write_is_announced_once", super::a_finished_write_is_announced_once as fn()),
                    ("the_last_input_is_credited_before_the_worker_drains", super::the_last_input_is_credited_before_the_worker_drains as fn()),
                    ("bad_sidecar_bodies_are_replaced_after_playback", super::bad_sidecar_bodies_are_replaced_after_playback as fn()),
                    ("valid_sidecars_win_over_a_shorter_playback", super::valid_sidecars_win_over_a_shorter_playback as fn()),
                    ("an_immediate_poll_of_the_first_preview_input_gets_its_positions", super::an_immediate_poll_of_the_first_preview_input_gets_its_positions as fn()),
                    ("a_same_size_external_edit_before_settle_cannot_adopt_the_previous_runs_positions", super::a_same_size_external_edit_before_settle_cannot_adopt_the_previous_runs_positions as fn()),
                    ("a_poll_that_straddles_generation_arm_seeds_the_first_input_once", super::a_poll_that_straddles_generation_arm_seeds_the_first_input_once as fn()),
                    ("another_threads_save_to_the_same_library_rearms_a_done_macro", super::another_threads_save_to_the_same_library_rearms_a_done_macro as fn()),
                    ("another_threads_save_to_the_same_library_drops_an_open_run", super::another_threads_save_to_the_same_library_drops_an_open_run as fn()),
                ]);
            }
        }
    }
    }
}
