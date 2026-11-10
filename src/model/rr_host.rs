//! Recording worker decisions the host can run. The Switch wrapper performs the notices,
//! the save-restore request, and the keyboard offer this module names.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use super::library::{
    Chain, Drill, DrillKind, Kind, LegacySlot, LoopCount, NamedSave, RecordingIndex, Role, Sequence,
    StartSaveState, Use, MACROS_DIR, SAVES_DIR, SEQUENCES_DIR,
};
use super::frame_queue::FrameQueue;
use super::macros::{FrameSnapshot, FrameWindow, InputFrame, Library, MacroRecording, MomentumMode, Placement, MAX_FRAMES};
use super::playback::{Book, Playback, StartLink};
use super::rr_runtime::{
    chord_notice, civil_date, crop_combo_file, crop_macro_file, delete_save,
    drive_mash, drive_play, drive_step, duplicate_save, finish_recording, migrate_at_boot,
    offer_keyboard, push_frame, save_chord, save_made_combo, start_save_notice, toggle_record, Capture,
    KeyboardOffer, Tape, EMPTY_COMBO, READ_WINDOW, RESIDENT_FRAMES,
};
use crate::model::combo::Combo;
use crate::model::cpu::playback::PlaybackState;
use crate::model::input_delay::MappedInputs;
use crate::model::recording::{inject_values, ControlCtx, ControlOut, CpuRoleDrive, Inject, Phase};
use crate::model::recording_menu::{Effect, LoopMark, Selection, ViewRole};
use crate::model::state::record::SavedState;
use crate::model::store::Store;

const ROLE_PLAYER: u8 = 1;
const ROLE_CPU: u8 = 2;

static RECORD_ROLE: AtomicU8 = AtomicU8::new(ROLE_PLAYER);
static LAST_ROLE: AtomicU8 = AtomicU8::new(ROLE_PLAYER);
static CAP_ROLE: AtomicU8 = AtomicU8::new(0);
static CAP_LEN: AtomicUsize = AtomicUsize::new(0);
static CAP_FRAMES: [AtomicU64; MAX_FRAMES] = [const { AtomicU64::new(0) }; MAX_FRAMES];
static RECORD_EDGE: AtomicU8 = AtomicU8::new(0);
static PLAY_PLAYER: AtomicBool = AtomicBool::new(false);
static PLAY_CPU: AtomicBool = AtomicBool::new(false);
static MASH: AtomicBool = AtomicBool::new(false);
static MIGRATED: AtomicBool = AtomicBool::new(false);
/// Played frames, filled ahead by the worker and taken one per game frame by Player 1's poll and the
/// CPU's AI epilogue. One slot lost frames: the worker is not tied to the game's frames (Eden
/// `rc5-fid-measure1`, 2026-11-08: one CPU frame every three AI frames on a recorded play).
static PLAYER_FRAMES: hos_lock::Mutex<FrameQueue> = hos_lock::Mutex::new(FrameQueue::new());
static CPU_FRAMES: hos_lock::Mutex<FrameQueue> = hos_lock::Mutex::new(FrameQueue::new());
static CURRENT_KIND: AtomicU32 = AtomicU32::new(u32::MAX);
static RUN_PLAYER: AtomicBool = AtomicBool::new(false);
static RUN_CPU: AtomicBool = AtomicBool::new(false);
static MACHINE: Mutex<Option<Playback>> = Mutex::new(None);
static RESIDENT: Mutex<Option<Resident>> = Mutex::new(None);
static MENU_EFFECT: Mutex<Effect> = Mutex::new(Effect::None);
static CROP: Mutex<Option<CropWindow>> = Mutex::new(None);
static DEFER_MIGRATE: AtomicBool = AtomicBool::new(false);
static ROLE_KIND: [AtomicI32; 2] = [const { AtomicI32::new(-1) }; 2];
static STRIP: Mutex<(u32, Option<Combo>)> = Mutex::new((0, None));
static MAKE: Mutex<Option<MakeJob>> = Mutex::new(None);
static HELD: Mutex<Option<Role>> = Mutex::new(None);
static ANSWER: Mutex<Option<MismatchChoice>> = Mutex::new(None);
static LAST_DRILL: Mutex<Option<String>> = Mutex::new(None);
static DRILL_SEEN: AtomicBool = AtomicBool::new(false);
static DRILL_ASK: Mutex<Option<String>> = Mutex::new(None);
static PLAYER_LABEL: Mutex<String> = Mutex::new(String::new());
static LAST_ASK: Mutex<Option<MismatchAsk>> = Mutex::new(None);
static CATALOG_GEN: AtomicU64 = AtomicU64::new(0);
static CPU_ROLE_ON: AtomicBool = AtomicBool::new(false);
/// A Play warped to its start save and the restore has not landed yet. No role steps until it
/// does: a frame fed before the restore is undone by it. A CPU-role file begins on its first
/// press, so its opening action was always lost (Eden `rc5-opus-cpu2`, 2026-11-08).
static WARP_HOLD: AtomicBool = AtomicBool::new(false);
/// Both fighters at the last record chord that started a recording (`note_record_start`). Values only: the game thread
/// writes it, the worker takes it when the recording stops.
static RECORD_START: hos_lock::Mutex<Option<RecordStart>> = hos_lock::Mutex::new(None);
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
/// Edit placement's Keep: a file id and the offset Play uses for it instead of the file's.
static KEPT_PLACEMENT: Mutex<Option<(String, Placement)>> = Mutex::new(None);

/// The bound save as the save-state machine's active slot must hold it (RECORDING.md, "Save states only warp").
#[derive(Clone, Debug, PartialEq)]
struct BoundCopy {
    id: Option<String>,
    pair: Option<[SavedState; 2]>,
}

struct MakeJob {
    source: String,
    ready: bool,
    /// The strip's count of links captured when the source began playing.
    since: u32,
}

struct Resident {
    id: String,
    frames: Vec<InputFrame>,
    tape: Tape,
}

/// The crop handles. The menu names the file. The replay window names the frames.
#[derive(Clone, Debug)]
pub struct CropWindow {
    pub start: u32,
    pub end: u32,
    pub snapshot: FrameSnapshot,
    pub name: String,
}

/// What one worker tick asks the Switch wrapper to do.
#[derive(Clone, Debug, Default)]
pub struct Tick {
    pub warp: Option<String>,
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

pub fn pending() -> bool {
    RECORD_EDGE.load(Ordering::Acquire) != 0
        || PLAY_PLAYER.load(Ordering::Acquire)
        || PLAY_CPU.load(Ordering::Acquire)
        || MASH.load(Ordering::Acquire)
        || menu_pending()
        || playing()
        || waiting_work()
}

/// Work that waits on no chord: a drill question, a running Make combo, or an answer to the fighter question. The service
/// gate (`pending`) and the tick (`service_at`) read the same list; an answered question that `pending` left out was never
/// applied once migration had finished, so the held play never resumed (rc.5 Fighter Mismatch).
fn waiting_work() -> bool {
    lock(&DRILL_ASK).is_some() || lock(&MAKE).is_some() || lock(&ANSWER).is_some()
}

fn playing() -> bool {
    role_playing(Role::Player) || role_playing(Role::Cpu)
}

/// The machine still steps the role, or its hook still has frames of the run to play.
fn role_playing(role: Role) -> bool {
    run_flag(role).load(Ordering::Acquire) || !role_frames(role).lock().is_empty()
}

fn run_flag(role: Role) -> &'static AtomicBool {
    match role {
        Role::Player => &RUN_PLAYER,
        Role::Cpu => &RUN_CPU,
    }
}

pub fn set_record_role(role: Role) {
    let bits = match role {
        Role::Player => ROLE_PLAYER,
        Role::Cpu => ROLE_CPU,
    };
    RECORD_ROLE.store(bits, Ordering::Release);
}

/// The pop-up's role, posted by the menu worker before the record chord.
pub fn publish_menu_role(role: ViewRole) {
    set_record_role(match role {
        ViewRole::Cpu => Role::Cpu,
        ViewRole::Player => Role::Player,
    });
}

/// After the record chord updates `CAP_ROLE`. A CPU-role start enters lockout. A stop leaves it.
pub fn cpu_role_after_chord(facing: i8) {
    let mut drive = CPU_DRIVE.lock();
    let role = CAP_ROLE.load(Ordering::Acquire);
    if drive.active() && role == 0 {
        drive.stop();
        CPU_ROLE_ON.store(false, Ordering::Release);
    } else if role == ROLE_CPU && !drive.active() {
        drive.begin(facing);
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
        push_capture(
            ROLE_CPU,
            InputFrame {
                buttons: frame.buttons,
                lstick_x: frame.lstick_x,
                lstick_y: frame.lstick_y,
                rstick_x: frame.rstick_x,
                rstick_y: frame.rstick_y,
            },
        );
    }
    true
}

/// The CPU control frame of a CPU-role recording. `write` is what replaces the AI's controls.
pub fn cpu_role_control(ctx: &ControlCtx) -> ControlOut {
    if !CPU_ROLE_ON.load(Ordering::Acquire) {
        return ControlOut { write: None, hold_cliff_timer: false };
    }
    CPU_DRIVE.lock().control(ctx)
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
    inject_values(
        MappedInputs {
            buttons: frame.buttons,
            lstick_x: frame.lstick_x,
            lstick_y: frame.lstick_y,
            rstick_x: frame.rstick_x,
            rstick_y: frame.rstick_y,
        },
        1,
    )
}

/// The menu worker posts one effect. The recording worker applies it. A `None` does not clear a pending effect.
pub fn post_effect(effect: Effect) {
    if matches!(effect, Effect::None) {
        return;
    }
    *lock(&MENU_EFFECT) = effect;
}

pub fn set_current_fighter(kind: i32) {
    CURRENT_KIND.store(kind as u32, Ordering::Release);
}

pub fn set_crop_window(window: CropWindow) {
    *lock(&CROP) = Some(window);
}

/// The three chords, consumed by the AI hook. This only stores flags.
pub fn on_chords(record: bool, play_player: bool, play_cpu: bool) {
    if record {
        let role = RECORD_ROLE.load(Ordering::Acquire);
        if CAP_ROLE.load(Ordering::Acquire) == 0 {
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

pub fn note_mash() {
    MASH.store(true, Ordering::Release);
}

/// What the CPU's behaviours see of playback (B5 / N4, Mash interrupts playback): the old recorder's state, with a
/// library macro playing on the CPU counted as a running playback.
pub fn cpu_playback(recorder: PlaybackState) -> PlaybackState {
    PlaybackState {
        running: recorder.running || role_playing(Role::Cpu),
        ..recorder
    }
}

/// The save-state restore a Play's warp asked for has placed both fighters (or the warp was
/// not requested after all). The held runs step from their first frame on the next tick.
pub fn warp_landed() {
    WARP_HOLD.store(false, Ordering::Release);
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
}

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
    *RECORD_START.lock() = Some(start);
}

/// PLAY-MOMENTUM (primary 2026-11-08 23:03Z, reading (b)): what the role's run owes its first input, taken once by the
/// game frame that injected that input. `None` when nothing is owed (no warp, or already given).
pub fn take_start_speed(role: Role) -> Option<StartSpeed> {
    let bit = 1u8 << role_index(role);
    // While the warp's restore is pending no input is fed, and the restore would undo a speed set now.
    if START_OWED.load(Ordering::Acquire) & bit == 0 || WARP_HOLD.load(Ordering::Acquire) {
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
    CAP_LEN.store(len + 1, Ordering::Release);
}

#[cfg(any(test, target_os = "switch"))]
pub(crate) fn capture_role() -> u8 {
    CAP_ROLE.load(Ordering::Acquire)
}

pub fn take_player_frame() -> Option<InputFrame> {
    take_frame(&PLAYER_FRAMES)
}

#[cfg(any(test, target_os = "switch"))]
pub(crate) fn take_cpu_frame() -> Option<InputFrame> {
    take_frame(&CPU_FRAMES)
}

/// Player 1's poll. Injects one posted frame, then stores the output when Player 1 is the recorded role. Returns whether
/// a frame of a Player run was injected (its first one may owe a start speed, `take_start_speed`).
///
/// # Safety
/// `out` is null or a live mapped-input record for this poll.
pub unsafe fn on_player_poll(out: *mut MappedInputs) -> bool {
    if out.is_null() {
        return false;
    }
    let injected = take_player_frame();
    if let Some(frame) = injected {
        (*out).buttons = frame.buttons;
        (*out).lstick_x = frame.lstick_x;
        (*out).lstick_y = frame.lstick_y;
        (*out).rstick_x = frame.rstick_x;
        (*out).rstick_y = frame.rstick_y;
    }
    if CAP_ROLE.load(Ordering::Acquire) == ROLE_PLAYER {
        let mapped = *out;
        push_capture(
            ROLE_PLAYER,
            InputFrame {
                buttons: mapped.buttons,
                lstick_x: mapped.lstick_x,
                lstick_y: mapped.lstick_y,
                rstick_x: mapped.rstick_x,
                rstick_y: mapped.rstick_y,
            },
        );
    }
    injected.is_some()
}

pub fn today() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => civil_date(duration.as_secs()),
        Err(_) => "1970-01-01".to_string(),
    }
}

pub fn service_at(
    root: &Path,
    slots: &[Option<LegacySlot>; 5],
    date: &str,
    player: SavedState,
    cpu: SavedState,
    online: bool,
) -> Tick {
    let mut tick = Tick::default();
    apply_waiting_answer(&mut tick);
    if !DEFER_MIGRATE.load(Ordering::Acquire)
        && !MIGRATED.load(Ordering::Acquire)
        && migrate_at_boot(root, slots, date).is_ok()
    {
        MIGRATED.store(true, Ordering::Release);
        BOUND_DIRTY.store(true, Ordering::Release);
        tick.diags.push("screen=migrate once=1".to_string());
    }
    let edge = RECORD_EDGE.swap(0, Ordering::AcqRel);
    let mut play_player = PLAY_PLAYER.swap(false, Ordering::AcqRel);
    let mut play_cpu = PLAY_CPU.swap(false, Ordering::AcqRel);
    let mash = MASH.swap(false, Ordering::AcqRel);
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
    if edge == 2 {
        let role_now = if RECORD_ROLE.load(Ordering::Acquire) == ROLE_CPU {
            "cpu"
        } else {
            "player"
        };
        tick.diags
            .push(format!("screen=recording role={role_now} recording=0"));
        let (role, frames) = taken_frames();
        let mut capture = Capture::idle();
        toggle_record(&mut capture, role);
        for frame in frames {
            push_frame(&mut capture, role, frame);
        }
        // The start save is where Record started, and the macro keeps the recorded role's speed then (PLAY-MOMENTUM).
        // Without a posted start (only a host test drives the chord without the game thread) the worker's live pose
        // stands in, at rest, as before.
        let (player, cpu, speeds) = match RECORD_START.lock().take() {
            Some(start) => (start.poses[0].saved(), start.poses[1].saved(), start.speeds),
            None => (player.clone(), cpu.clone(), [(0.0, 0.0); 2]),
        };
        if let Ok(stop) = finish_recording(root, &capture, player, cpu, speeds[role_index(role)], unix_now())
        {
            bump_catalog();
            if let Some(text) = stop.notice() {
                tick.notices.push(text);
            }
        }
    }
    // A drill question and a finished Make combo are work even when no chord is waiting.
    let busy = play_player
        || play_cpu
        || mash
        || menu_pending()
        || playing()
        || waiting_work();
    if !busy {
        return tick;
    }
    let Ok(book) = load_book(root) else {
        return tick;
    };
    let mut machine = lock(&MACHINE);
    let machine = machine.get_or_insert_with(Playback::new);
    if mash {
        // The mash is the CPU's (Mash Toggles), so it stops the macro the CPU plays; Player 1's keeps playing
        // (RECORDING.md: "stops the macro that role is playing"; primary 2026-11-08 13:05Z).
        drive_mash(machine, Role::Cpu, true);
        RUN_CPU.store(false, Ordering::Release);
        drop_frames(Role::Cpu);
        tick.diags.push("screen=playback mash=1".to_string());
    }
    release_held(&mut play_player, &mut play_cpu);
    if play_player {
        play_role(root, machine, &book, Role::Player, &mut tick);
    } else if RUN_PLAYER.load(Ordering::Acquire) {
        step_role(root, machine, &book, Role::Player);
    }
    if play_cpu {
        play_role(root, machine, &book, Role::Cpu, &mut tick);
    } else if RUN_CPU.load(Ordering::Acquire) {
        step_role(root, machine, &book, Role::Cpu);
    }
    if menu_pending() {
        apply_menu(root, machine, &book, online, &mut tick);
    }
    ask_drill(root, &mut tick);
    finish_make(root, &mut tick);
    tick
}

/// A record or a menu effect while the old slot image is still filling. Migration stays pending.
pub fn service_pending(root: &Path, player: SavedState, cpu: SavedState, online: bool) -> Tick {
    DEFER_MIGRATE.store(true, Ordering::Release);
    let empty = [None, None, None, None, None];
    let tick = service_at(root, &empty, "", player, cpu, online);
    DEFER_MIGRATE.store(false, Ordering::Release);
    tick
}

pub fn set_role_fighter(role: Role, kind: i32) {
    ROLE_KIND[role_index(role)].store(kind, Ordering::Release);
    if role == Role::Player {
        CURRENT_KIND.store(kind as u32, Ordering::Release);
    }
}

/// The strip's last combo and its count of links captured (`combo::Helper::seen`), from the recording worker's tick.
pub fn set_strip_sample((seen, combo): (u32, Option<Combo>)) {
    *lock(&STRIP) = (seen, combo);
}

pub fn set_fighter_label(label: &str) {
    *lock(&PLAYER_LABEL) = label.to_string();
}

pub fn answer_mismatch(choice: MismatchChoice) {
    *lock(&ANSWER) = Some(choice);
}

pub fn catalog_generation() -> u64 {
    CATALOG_GEN.load(Ordering::Acquire)
}

fn bump_catalog() {
    CATALOG_GEN.fetch_add(1, Ordering::Release);
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

/// What Edit placement edits (primary 2026-11-08 #70 (a)): the page role's active macro, the file Play plays, and the
/// position its start save gives that role. Play warps there and adds the offset (`warp_pair`).
#[derive(Clone, Debug, PartialEq)]
pub struct PlacementTarget {
    pub role: Role,
    pub id: String,
    pub recording: MacroRecording,
    pub anchor: (f32, f32),
}

/// The game's `SITUATION_KIND_AIR` (ground 0, cliff 1).
const SITUATION_AIR: i32 = 2;

pub const PLACE_NO_MACRO: &str = "Make a macro active for this role first";
pub const PLACE_NO_START: &str = "Link a start save to the macro first";

pub fn placement_target(root: &Path) -> Result<PlacementTarget, &'static str> {
    let role = if RECORD_ROLE.load(Ordering::Acquire) == ROLE_CPU { Role::Cpu } else { Role::Player };
    let index = RecordingIndex::load(root).map_err(|_| PLACE_NO_MACRO)?;
    let active = index.active(role).ok_or(PLACE_NO_MACRO)?.to_owned();
    // A chain warps to macro 1's start save and plays with macro 1's placement (`play_placement`), so it edits macro 1.
    let (id, chain_on) = match Store::<Chain>::new(root.join(MACROS_DIR)).load(&active) {
        Ok(chain) => (chain.recordings.first().cloned().ok_or(PLACE_NO_MACRO)?, Some(chain.start_save_on)),
        Err(_) => (active, None),
    };
    let recording = Library::new(root.join(MACROS_DIR)).load(&id).map_err(|_| PLACE_NO_MACRO)?;
    let start_on = chain_on.unwrap_or(recording.start_save_on);
    let save = recording
        .start_save
        .as_deref()
        .filter(|_| start_on)
        .and_then(|save| Store::<NamedSave>::new(root.join(SAVES_DIR)).load(save).ok())
        .ok_or(PLACE_NO_START)?;
    let at = match role {
        Role::Player => &save.player,
        Role::Cpu => &save.cpu,
    };
    let anchor = (at.x, at.y);
    let mut recording = recording;
    if let Some(placement) = kept_placement(&id) {
        recording.placement = placement;
    }
    Ok(PlacementTarget { role, id, recording, anchor })
}

/// Keep on Edit placement: Play uses this offset for that file until the game closes. The file is unchanged.
pub fn keep_placement(id: &str, placement: Placement) {
    *lock(&KEPT_PLACEMENT) = Some((id.to_owned(), placement));
}

/// Write over put an offset into the file: a kept one for that file no longer applies.
pub fn placement_written(id: &str) {
    let mut kept = lock(&KEPT_PLACEMENT);
    if kept.as_ref().is_some_and(|(kept, _)| kept == id) {
        *kept = None;
    }
}

fn kept_placement(id: &str) -> Option<Placement> {
    lock(&KEPT_PLACEMENT).as_ref().filter(|(kept, _)| kept == id).map(|(_, placement)| *placement)
}

/// The placement a Play of `id` warps with: a kept one, else the file's (a chain: macro 1's, whose start save it is).
fn play_placement(book: &Book, id: &str) -> Placement {
    let first = book.first_recording(id).unwrap_or(id);
    kept_placement(first).unwrap_or_else(|| book.placement(first))
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
        }
    }
    pair
}

/// Save writes the one bound save, or a new bound Save N when none is bound (`rr_runtime::save_chord`). Worker side.
pub fn write_saved(root: &Path) -> bool {
    let Some([player, cpu]) = lock(&SAVED_PAIR).take() else {
        return false;
    };
    let Ok(id) = save_chord(root, player.clone(), cpu.clone(), unix_now()) else {
        return false;
    };
    *lock(&BOUND_APPLIED) = Some(BoundCopy { id: Some(id), pair: Some([player, cpu]) });
    bump_catalog();
    true
}

/// The menu's selection is what Play, Record, and the loop read. The index is that selection.
pub fn publish_selection(root: &Path, selection: &Selection, drill_kind: Option<DrillKind>) {
    let Ok(mut index) = RecordingIndex::load(root) else {
        return;
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
    let _ = index.save(root);
    BOUND_DIRTY.store(true, Ordering::Release);
}

/// What the grade helper should do when the recording drill id changes.
/// The same id is not applied again: applying a target clears the open attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradeFollow<'a> {
    Same,
    Clear,
    Install(&'a str),
}

/// The helper's target follows the recording drill. `applied` is the id already
/// given to the helper. `None` clears it. A different id is installed once.
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
pub fn start_sequence_drill(root: &Path, id: &str) -> Option<crate::model::drill::SequenceDrill> {
    let sequence = Store::<Sequence>::new(root.join(SEQUENCES_DIR)).load(id).ok()?;
    let combos = crate::model::combo_record::ComboLibrary::new(root.join("combos"));
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
            && self.steps.iter().zip(&other.steps).all(|(a, b)| a.id == b.id && a.combo == b.combo)
    }
}

/// The plan of sequence `id`, read from the card. A deleted combo is skipped. None when the file does not load.
pub fn sequence_plan(root: &Path, id: &str) -> Option<SequencePlan> {
    let sequence = Store::<Sequence>::new(root.join(SEQUENCES_DIR)).load(id).ok()?;
    let combos = crate::model::combo_record::ComboLibrary::new(root.join("combos"));
    let steps = sequence
        .combos
        .iter()
        .filter_map(|combo_id| {
            let record = combos.load(combo_id).ok()?;
            Some(SequenceStep { id: combo_id.clone(), name: record.name, combo: record.combo })
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
    let saved = store.overwrite(id, &sequence).is_ok();
    if saved {
        bump_catalog();
    }
    saved
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
    show_chord_notice(root, tick);
    if let Some(ask) = mismatch_for(book, role) {
        match lock(&ANSWER).take() {
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
                tick.diags.push(format!("screen=mismatch ask={} answer=keep", ask.origin));
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
    if !machine.playing(role) && !role_frames(role).lock().is_empty() {
        // The machine has handed over its last frames but the hook is still playing them: the chord stops
        // that run, as it does while the machine still steps.
        drop_frames(role);
        run_flag(role).store(false, Ordering::Release);
        return;
    }
    let effect = drive_play(machine, role, book);
    let word = if role == Role::Player {
        "player"
    } else {
        "cpu"
    };
    let mut owed = None;
    if let Some(id) = effect.warp {
        tick.warp = Some(id.to_owned());
        if let Some(active) = book.index.active(role) {
            let placement = play_placement(book, active);
            tick.warp_place = Some((role, placement));
            owed = play_start_speed(book, active, placement).map(|speed| (speed, placement.momentum));
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
        WARP_HOLD.store(true, Ordering::Release);
    }
    note_missing_start(root, book, role, tick);
    if effect.stopped || effect.started {
        drop_frames(role);
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
        step_role(root, machine, book, role);
    }
}

/// Fill the role's queue ahead of its hook, in file order. The hook takes one frame per game frame, so a
/// late or doubled worker tick neither drops nor repeats one.
fn step_role(root: &Path, machine: &mut Playback, book: &Book, role: Role) {
    if WARP_HOLD.load(Ordering::Acquire) {
        return;
    }
    while role_frames(role).lock().room() > 0 {
        match drive_step(machine, role, book) {
            Some(played) => publish(role, resident_frame(root, &played)),
            None => {
                run_flag(role).store(false, Ordering::Release);
                return;
            }
        }
    }
}

fn resident_frame(root: &Path, played: &super::playback::Played) -> InputFrame {
    if played.frame_index < RESIDENT_FRAMES {
        return played.frame;
    }
    let mut slot = lock(&RESIDENT);
    let same = slot
        .as_ref()
        .is_some_and(|item| item.id == played.recording_id);
    if !same {
        let Ok(recording) = Library::new(root.join(MACROS_DIR)).load(played.recording_id) else {
            return played.frame;
        };
        let tape = Tape::load(&recording.frames);
        *slot = Some(Resident {
            id: played.recording_id.to_owned(),
            frames: recording.frames,
            tape,
        });
    }
    let index = played.frame_index;
    let missing = slot
        .as_ref()
        .is_some_and(|item| item.tape.hook_frame(index).is_none());
    if missing {
        let item = slot.as_mut().expect("resident");
        let start = RESIDENT_FRAMES + ((index - RESIDENT_FRAMES) / READ_WINDOW) * READ_WINDOW;
        if start < item.frames.len() {
            let count = READ_WINDOW.min(item.frames.len() - start);
            let window = item.frames[start..start + count].to_vec();
            item.tape.fill_window(index, |_, _| window.clone());
        }
    }
    slot.as_ref()
        .and_then(|item| item.tape.hook_frame(index))
        .unwrap_or(played.frame)
}

fn publish(role: Role, frame: InputFrame) {
    role_frames(role).lock().push(pack(frame));
}

/// A run started, stopped or was cut by a mash: what it buffered is not played, and a start speed it still owed is not
/// given.
fn drop_frames(role: Role) {
    role_frames(role).lock().clear();
    owe_start_speed(role, None);
}

fn note_missing_start(root: &Path, book: &Book, role: Role, tick: &mut Tick) {
    let Some(id) = book.index.active(role) else {
        return;
    };
    let Ok(recording) = Library::new(root.join(MACROS_DIR)).load(id) else {
        return;
    };
    let Some(save) = recording.start_save.as_deref() else {
        return;
    };
    if Store::<NamedSave>::new(root.join(SAVES_DIR))
        .load(save)
        .is_ok()
    {
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
    let Ok(record) = crate::model::combo_record::ComboLibrary::new(root.join("combos")).load(&id) else {
        return;
    };
    let Some(recorded) = record.recorded_on.filter(|name| !name.is_empty()) else {
        return;
    };
    let current = lock(&PLAYER_LABEL).clone();
    if current.is_empty() || current == recorded {
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

fn apply_menu(root: &Path, machine: &mut Playback, book: &Book, online: bool, tick: &mut Tick) {
    let effect = std::mem::replace(&mut *lock(&MENU_EFFECT), Effect::None);
    match effect {
        Effect::None => {}
        Effect::Keyboard { header, draft } => {
            if let Some(offer) = offer_keyboard(header, &draft, online) {
                tick.diags
                    .push(format!("screen=rename header={}", offer.header));
                tick.keyboard = Some(offer);
            }
        }
        Effect::Crop { source, combo, start, end } => {
            if write_crop_range(root, &source, combo, start, end).is_some() {
                tick.diags.push(format!(
                    "screen=crop source={source} combo={} saved=1",
                    i32::from(combo)
                ));
                bump_catalog();
            }
        }
        Effect::MakeCombo { source, chain } => {
            begin_make(root, machine, book, &source, chain, online, tick)
        }
        Effect::DuplicateSave { source } => {
            if duplicate_save(root, &source, unix_now()).is_ok() {
                bump_catalog();
            }
        }
        Effect::DeleteSave { source } => {
            if delete_save(root, &source, unix_now()).is_ok() {
                bump_catalog();
            }
        }
        Effect::WriteChain { name, steps } => {
            if let Ok(chain) = Chain::new(name, steps) {
                if Store::<super::library::Chain>::new(root.join(MACROS_DIR)).save_new(&chain).is_ok() {
                    bump_catalog();
                }
            }
        }
        Effect::WriteSequence { name, combos } => {
            if let Ok(sequence) = Sequence::new(name, combos) {
                if Store::<Sequence>::new(root.join(SEQUENCES_DIR)).save_new(&sequence).is_ok() {
                    bump_catalog();
                }
            }
        }
        Effect::RenameSave { id, name } => rename_save(root, &id, name),
        Effect::RenameMacro { id, name } => rename_macro(root, &id, name),
        Effect::RenameChain { id, name } => rename_chain(root, &id, name),
        Effect::RenameCombo { id, name } => rename_combo(root, &id, name),
        Effect::RenameSequence { id, name } => rename_sequence(root, &id, name),
        Effect::DeleteFile { kind, id } => {
            if delete_file(root, kind, &id) {
                bump_catalog();
            }
        }
        Effect::WriteChainSteps { id, steps } => {
            let store = Store::<Chain>::new(root.join(MACROS_DIR));
            if let Ok(mut chain) = store.load(&id) {
                chain.recordings = steps;
                if store.overwrite(&id, &chain).is_ok() {
                    bump_catalog();
                }
            }
        }
        Effect::WriteSequenceCombos { id, combos } => {
            let store = Store::<Sequence>::new(root.join(SEQUENCES_DIR));
            if let Ok(mut sequence) = store.load(&id) {
                sequence.combos = combos;
                if store.overwrite(&id, &sequence).is_ok() {
                    bump_catalog();
                }
            }
        }
        Effect::SaveLastCombo { name } => save_last_combo(root, &name, tick),
    }
}

/// *+ Save last combo…* (operator 2026-11-08; rc.4 COMBO-HELPER): the strip's latest exchange becomes a combo file
/// named `name` (empty: Combo N), recorded on Player 1's fighter, hot, and the drill (RECORDING.md).
fn save_last_combo(root: &Path, name: &str, tick: &mut Tick) {
    let Some(combo) = lock(&STRIP).1.filter(|combo| !combo.is_empty()) else {
        tick.notices.push(EMPTY_COMBO);
        tick.diags.push("screen=notice kind=combo-empty".to_string());
        return;
    };
    let label = lock(&PLAYER_LABEL).clone();
    let recorded_on = (!label.is_empty()).then_some(label);
    if let Ok(id) = super::rr_runtime::save_last_combo(root, &combo, name, recorded_on, unix_now()) {
        tick.diags.push(format!("screen=save-last-combo saved={id} links={}", combo.len()));
        bump_catalog();
    }
}

/// Delete on a macro, a chain, a combo or a sequence: the file goes, and the index no longer names it as a role's
/// active macro or as the drill. A sequence that lists a deleted combo skips it.
pub fn delete_file(root: &Path, kind: crate::model::recording_menu::FileKind, id: &str) -> bool {
    use crate::model::recording_menu::FileKind;
    let deleted = match kind {
        FileKind::Macro => Library::new(root.join(MACROS_DIR)).delete(id).is_ok(),
        FileKind::Chain => Store::<Chain>::new(root.join(MACROS_DIR)).delete(id).is_ok(),
        FileKind::Combo => crate::model::combo_record::ComboLibrary::new(root.join("combos")).delete(id).is_ok(),
        FileKind::Sequence => Store::<Sequence>::new(root.join(SEQUENCES_DIR)).delete(id).is_ok(),
    };
    if !deleted {
        return false;
    }
    if let Ok(mut index) = RecordingIndex::load(root) {
        let at = unix_now();
        let mut changed = false;
        for role in [Role::Player, Role::Cpu] {
            if index.active(role) == Some(id) {
                changed |= index.set_active(role, None, at).is_ok();
            }
        }
        if index.drill().is_some_and(|drill| drill.id == id) {
            changed |= index.set_drill(None, at).is_ok();
        }
        if changed {
            let _ = index.save(root);
        }
    }
    true
}

fn rename_save(root: &Path, id: &str, name: String) {
    let store = Store::<NamedSave>::new(root.join(SAVES_DIR));
    let Ok(mut save) = store.load(id) else { return };
    save.name = name;
    if store.overwrite(id, &save).is_ok() {
        bump_catalog();
    }
}

fn rename_macro(root: &Path, id: &str, name: String) {
    let store = Library::new(root.join(MACROS_DIR));
    let Ok(mut file) = store.load(id) else { return };
    file.name = name;
    if store.overwrite(id, &file).is_ok() {
        bump_catalog();
    }
}

fn rename_chain(root: &Path, id: &str, name: String) {
    let store = Store::<Chain>::new(root.join(MACROS_DIR));
    let Ok(mut file) = store.load(id) else { return };
    file.name = name;
    if store.overwrite(id, &file).is_ok() {
        bump_catalog();
    }
}

fn rename_combo(root: &Path, id: &str, name: String) {
    let store = crate::model::combo_record::ComboLibrary::new(root.join("combos"));
    let Ok(mut file) = store.load(id) else { return };
    file.name = name;
    if store.overwrite(id, &file).is_ok() {
        bump_catalog();
    }
}

fn rename_sequence(root: &Path, id: &str, name: String) {
    let store = Store::<Sequence>::new(root.join(SEQUENCES_DIR));
    let Ok(mut file) = store.load(id) else { return };
    file.name = name;
    if store.overwrite(id, &file).is_ok() {
        bump_catalog();
    }
}

fn begin_make(
    root: &Path,
    machine: &mut Playback,
    book: &Book,
    source: &str,
    chain: bool,
    online: bool,
    tick: &mut Tick,
) {
    let _ = chain;
    match book.start_link(source) {
        StartLink::Deleted => {
            if let Some(text) = start_save_notice(super::library::StartSaveState::Deleted) {
                tick.notices.push(text);
                tick.diags
                    .push("screen=notice kind=macro-start-save-missing".to_string());
            }
        }
        StartLink::Linked(id) => {
            tick.warp = Some(id.to_string());
            tick.warp_place = Some((Role::Player, play_placement(book, source)));
            WARP_HOLD.store(true, Ordering::Release);
        }
        StartLink::None => {}
    }
    let effect = machine.play_file(Role::Player, book, source);
    if effect.started {
        drop_frames(Role::Player);
        RUN_PLAYER.store(true, Ordering::Release);
        step_role(root, machine, book, Role::Player);
    }
    let since = lock(&STRIP).0;
    *lock(&MAKE) = Some(MakeJob { source: source.to_string(), ready: true, since });
    tick.diags.push(format!("screen=make-combo source={source} playing={}", i32::from(effect.started)));
    if online {
        tick.keyboard = None;
    }
}

fn finish_make(root: &Path, tick: &mut Tick) {
    let ready = lock(&MAKE).as_ref().is_some_and(|job| job.ready);
    if !ready || role_playing(Role::Player) {
        return;
    }
    let Some(job) = lock(&MAKE).take() else {
        return;
    };
    // Only an exchange the strip captured while the source played: a playback that lands no hit leaves the count
    // where it was, and the earlier exchange the strip still shows is not this file's.
    let (seen, sample) = {
        let mut strip = lock(&STRIP);
        (strip.0, strip.1.take())
    };
    let Some(combo) = sample.filter(|combo| seen != job.since && !combo.is_empty()) else {
        tick.notices.push(EMPTY_COMBO);
        tick.diags.push("screen=notice kind=combo-empty".to_string());
        return;
    };
    if save_made_combo(root, &combo, &job.source, unix_now()).is_ok() {
        tick.diags.push(format!("screen=make-combo source={} saved=1", job.source));
        bump_catalog();
    }
}

fn write_crop_range(root: &Path, source: &str, combo: bool, start: u32, end: u32) -> Option<String> {
    if combo {
        let library = crate::model::combo_record::ComboLibrary::new(root.join("combos"));
        let original = library.load(source).ok()?;
        let span = original.combo.len() as u32;
        let (start, end) = clamp_range(start, end, span)?;
        let names = library.list().ok()?.entries.into_iter().map(|entry| entry.name).collect::<Vec<_>>();
        let name = super::rr_runtime::unused_name("Combo", &names);
        let id = crop_combo_file(root, source, start, end, &name).ok()?;
        note_hot(root, Kind::Combo, &id);
        return Some(id);
    }
    let library = Library::new(root.join(MACROS_DIR));
    let original = library.load(source).ok()?;
    let span = original.frames.len() as u32;
    let (start, end) = clamp_range(start, end, span)?;
    let snapshot = super::macros::FrameSnapshot { frame: start, state: original.start };
    let names = library.list().ok()?.entries.into_iter().map(|entry| entry.name).collect::<Vec<_>>();
    let name = super::rr_runtime::unused_name("Macro", &names);
    let id = crop_macro_file(root, source, start, end, &snapshot, &name).ok()?;
    note_hot(root, Kind::Macro, &id);
    Some(id)
}

fn clamp_range(start: u32, end: u32, span: u32) -> Option<(u32, u32)> {
    let mut window = FrameWindow::new(span).ok()?;
    window.set_in(start);
    window.set_out(end);
    Some((window.start(), window.end()))
}

fn note_hot(root: &Path, kind: Kind, id: &str) {
    let Ok(mut index) = RecordingIndex::load(root) else {
        return;
    };
    if index.note_use(kind, id, Use::Recorded, unix_now()).is_ok() {
        let _ = index.save(root);
    }
}

pub fn write_crop(root: &Path, source: &str, combo: bool) -> Option<String> {
    let window = lock(&CROP).take()?;
    if combo {
        crop_combo_file(root, source, window.start, window.end, &window.name).ok()
    } else {
        crop_macro_file(
            root,
            source,
            window.start,
            window.end,
            &window.snapshot,
            &window.name,
        )
        .ok()
    }
}

fn show_chord_notice(root: &Path, tick: &mut Tick) {
    let Ok(mut index) = RecordingIndex::load(root) else {
        return;
    };
    let Some(text) = chord_notice(&mut index) else {
        return;
    };
    if index.save(root).is_ok() {
        tick.notices.push(text);
        tick.diags.push("screen=notice kind=chord-once".to_string());
    }
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
fn take_frame(frames: &hos_lock::Mutex<FrameQueue>) -> Option<InputFrame> {
    if WARP_HOLD.load(Ordering::Acquire) {
        return None;
    }
    frames.lock().pop().map(unpack)
}

fn role_frames(role: Role) -> &'static hos_lock::Mutex<FrameQueue> {
    match role {
        Role::Player => &PLAYER_FRAMES,
        Role::Cpu => &CPU_FRAMES,
    }
}

fn menu_pending() -> bool {
    !matches!(&*lock(&MENU_EFFECT), Effect::None)
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
fn reset_host() {
    RECORD_ROLE.store(ROLE_PLAYER, Ordering::Release);
    LAST_ROLE.store(ROLE_PLAYER, Ordering::Release);
    CAP_ROLE.store(0, Ordering::Release);
    CAP_LEN.store(0, Ordering::Release);
    RECORD_EDGE.store(0, Ordering::Release);
    PLAY_PLAYER.store(false, Ordering::Release);
    PLAY_CPU.store(false, Ordering::Release);
    MASH.store(false, Ordering::Release);
    MIGRATED.store(false, Ordering::Release);
    PLAYER_FRAMES.lock().clear();
    CPU_FRAMES.lock().clear();
    CURRENT_KIND.store(u32::MAX, Ordering::Release);
    RUN_PLAYER.store(false, Ordering::Release);
    RUN_CPU.store(false, Ordering::Release);
    *lock(&MACHINE) = None;
    *lock(&RESIDENT) = None;
    *lock(&MENU_EFFECT) = Effect::None;
    *lock(&CROP) = None;
    *lock(&STRIP) = (0, None);
    *lock(&MAKE) = None;
    *lock(&HELD) = None;
    *lock(&ANSWER) = None;
    *lock(&LAST_ASK) = None;
    *lock(&LAST_DRILL) = None;
    *lock(&DRILL_ASK) = None;
    *lock(&PLAYER_LABEL) = String::new();
    DRILL_SEEN.store(false, Ordering::Release);
    DEFER_MIGRATE.store(false, Ordering::Release);
    CATALOG_GEN.store(0, Ordering::Release);
    CPU_ROLE_ON.store(false, Ordering::Release);
    WARP_HOLD.store(false, Ordering::Release);
    *RECORD_START.lock() = None;
    *START_SPEED.lock() = [None; 2];
    START_OWED.store(0, Ordering::Release);
    *CPU_DRIVE.lock() = CpuRoleDrive::idle();
    BOUND_DIRTY.store(true, Ordering::Release);
    *lock(&BOUND_APPLIED) = None;
    *lock(&SAVED_PAIR) = None;
    SLOT_RESYNC.store(false, Ordering::Release);
    *lock(&KEPT_PLACEMENT) = None;
    for slot in &ROLE_KIND {
        slot.store(-1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::super::macros::StartState;
    use super::*;
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

    fn state(kind: i32) -> SavedState {
        let mut state = SavedState::never_saved();
        state.fighter_kind = kind;
        state
    }

    fn none() -> [Option<LegacySlot>; 5] {
        [None, None, None, None, None]
    }

    #[test]
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
            false,
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
        let tick = service_at(&root, &none(), "2026-11-06", player, cpu, false);
        assert_eq!(tick.warp.as_deref(), Some(save_id.as_str()));
        assert!(tick
            .diags
            .iter()
            .any(|line| line == "screen=playback role=player warp=1"));
        assert!(take_player_frame().is_none(), "no frame before the warp's restore lands");
        warp_landed();
        let _ = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert_eq!(take_player_frame().expect("player frame").buttons, 4);
        let tick = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert_eq!(take_player_frame().expect("next frame").buttons, 5);
        assert!(tick.diags.iter().all(|line| !line.contains("warp=1")));
        note_mash();
        let tick = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(tick
            .diags
            .iter()
            .any(|line| line == "screen=playback mash=1"));
        // The CPU's mash stops only the CPU's macro: Player 1's keeps playing (primary 2026-11-08 13:05Z, #22 (a)).
        assert_eq!(take_player_frame().expect("Player 1 plays on after the CPU's mash").buttons, 6);
    }

    #[test]
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
        let _ = service_at(&root, &none(), "2026-11-06", state(13), state(7), false);
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

    #[test]
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
        let first = service_at(&root, &slots, "2026-11-06", state(1), state(1), false);
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
        let second = service_at(&root, &slots, "2026-11-06", state(1), state(1), false);
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

    #[test]
    fn the_worker_crop_writes_a_new_file() {
        let _hold = hold();
        reset_host();
        let root = scratch("crop");
        let recording = super::super::macros::MacroRecording::new(
            "Nair",
            "13.0.5",
            StartState::neutral(8, 0, 1.0),
            vec![
                InputFrame {
                    buttons: 1,
                    ..InputFrame::default()
                },
                InputFrame {
                    buttons: 2,
                    ..InputFrame::default()
                },
                InputFrame {
                    buttons: 3,
                    ..InputFrame::default()
                },
                InputFrame {
                    buttons: 4,
                    ..InputFrame::default()
                },
            ],
        )
        .unwrap();
        let id = Library::new(root.join(MACROS_DIR))
            .save_new(&recording)
            .unwrap();
        let before = std::fs::read(root.join(MACROS_DIR).join(format!("{id}.json"))).unwrap();
        set_crop_window(CropWindow {
            start: 1,
            end: 3,
            snapshot: FrameSnapshot {
                frame: 1,
                state: StartState::neutral(8, 0, 1.0),
            },
            name: "Nair crop".into(),
        });
        let copy = write_crop(&root, &id, false).unwrap();
        let after = std::fs::read(root.join(MACROS_DIR).join(format!("{id}.json"))).unwrap();
        assert_eq!(before, after);
        assert_ne!(copy, id);
        assert_eq!(
            Library::new(root.join(MACROS_DIR))
                .load(&copy)
                .unwrap()
                .frames
                .len(),
            2
        );
    }

    #[test]
    fn a_posted_menu_effect_reaches_the_worker() {
        let _hold = hold();
        reset_host();
        let root = scratch("posted");
        let _ = service_at(&root, &none(), "2026-11-06", state(1), state(1), false);
        let id = super::super::rr_runtime::save_chord(&root, state(3), state(4), 1).unwrap();
        post_effect(Effect::DuplicateSave { source: id.clone() });
        let _ = service_at(&root, &none(), "2026-11-06", state(3), state(4), false);
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
        set_crop_window(CropWindow {
            start: 0,
            end: 1,
            snapshot: FrameSnapshot { frame: 0, state: StartState::neutral(8, 0, 1.0) },
            name: "Nair crop".into(),
        });
        post_effect(Effect::Crop { source: macro_id.clone(), combo: false, start: 0, end: 1 });
        let tick = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(tick.diags.iter().any(|line| line.starts_with("screen=crop source=")));
        assert_eq!(Library::new(root.join(MACROS_DIR)).list().unwrap().entries.len(), 2);
        post_effect(Effect::Keyboard { header: "Name macro", draft: "Nair".into() });
        let named = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert_eq!(named.keyboard.as_ref().map(|offer| offer.header), Some("Name macro"));
        post_effect(Effect::DeleteSave { source: id });
        let _ = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(RecordingIndex::load(&root).unwrap().bound().is_none());
    }

    #[test]
    fn an_empty_posted_record_keeps_the_empty_notice() {
        let _hold = hold();
        reset_host();
        let root = scratch("empty");
        on_chords(true, false, false);
        on_chords(true, false, false);
        let tick = service_at(&root, &none(), "2026-11-06", state(1), state(1), false);
        assert!(Library::new(root.join(MACROS_DIR))
            .list()
            .unwrap()
            .entries
            .is_empty());
        assert!(tick
            .notices
            .contains(&super::super::rr_runtime::EMPTY_RECORDING));
    }

    #[test]
    fn the_cpu_inject_slot_starts_empty() {
        let _hold = hold();
        reset_host();
        assert_eq!(capture_role(), 0);
        assert!(take_cpu_frame().is_none());
    }

    #[test]
    fn a_frame_past_the_resident_minute_comes_from_the_window() {
        let _hold = hold();
        reset_host();
        let root = scratch("window");
        let frames: Vec<InputFrame> = (0..4_300)
            .map(|index| InputFrame {
                buttons: index as u32,
                ..InputFrame::default()
            })
            .collect();
        let recording = super::super::macros::MacroRecording::new(
            "Long",
            "13.0.5",
            StartState::neutral(1, 0, 1.0),
            frames,
        )
        .unwrap();
        let id = Library::new(root.join(MACROS_DIR))
            .save_new(&recording)
            .unwrap();
        let played = crate::engine::playback::Played {
            recording_id: id.as_str(),
            frame_index: RESIDENT_FRAMES,
            frame: InputFrame::default(),
        };
        assert_eq!(
            resident_frame(&root, &played).buttons,
            RESIDENT_FRAMES as u32
        );
        let later = crate::engine::playback::Played {
            recording_id: id.as_str(),
            frame_index: RESIDENT_FRAMES + 50,
            frame: InputFrame::default(),
        };
        assert_eq!(
            resident_frame(&root, &later).buttons,
            (RESIDENT_FRAMES as u32) + 50
        );
        assert_eq!(lock(&RESIDENT).as_ref().unwrap().tape.reads().len(), 1);
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

    #[test]
    fn record_follows_the_published_cpu_role() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-role");
        set_record_role(Role::Cpu);
        on_chords(true, false, false);
        let start = service_at(&root, &none(), "2026-11-06", state(13), state(7), false);
        assert!(start.diags.iter().any(|line| line == "screen=recording role=cpu recording=1"));
        push_capture(ROLE_CPU, InputFrame { buttons: 9, ..InputFrame::default() });
        push_capture(ROLE_PLAYER, InputFrame { buttons: 1, ..InputFrame::default() });
        on_chords(true, false, false);
        let stop = service_at(&root, &none(), "2026-11-06", state(13), state(7), false);
        assert!(stop.diags.iter().any(|line| line == "screen=recording role=cpu recording=0"));
        let macros = Library::new(root.join(MACROS_DIR));
        let listed = macros.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let saved = macros.load(&listed.entries[0].id).unwrap();
        assert_eq!(saved.frames[0].buttons, 9);
        let save_id = saved.start_save.expect("start save");
        let save = Store::<NamedSave>::new(root.join(SAVES_DIR)).load(&save_id).unwrap();
        assert_eq!(save.cpu.fighter_kind, 7);
        assert!(crate::engine::rr_runtime::load_catalog(&root).unwrap().catalog.macros.iter().any(|file| file.id == listed.entries[0].id));
    }

    fn press(buttons: u32) -> MappedInputs {
        MappedInputs { buttons, ..MappedInputs::NEUTRAL }
    }

    fn cpu_menu_role() {
        let mut screens = crate::model::recording_menu::RecordingScreens::new();
        screens.toggle_popup();
        screens.press_popup(crate::model::menu::input::Button::R);
        assert_eq!(screens.selection().record_role, ViewRole::Cpu);
        publish_menu_role(screens.selection().record_role);
    }

    fn player_menu_role() {
        let mut screens = crate::model::recording_menu::RecordingScreens::new();
        screens.toggle_popup();
        screens.press_popup(crate::model::menu::input::Button::L);
        assert_eq!(screens.selection().record_role, ViewRole::Player);
        publish_menu_role(screens.selection().record_role);
    }

    fn drive_cpu(buttons: u32) -> Option<u32> {
        let mut out = press(buttons);
        assert!(cpu_role_poll(&mut out));
        assert_eq!(out, MappedInputs::NEUTRAL);
        cpu_role_control(&ControlCtx { facing: 1, shulk_dial: false }).write.map(|write| write.buttons)
    }

    #[test]
    fn cpu_role_lockout_or_standby_press_is_not_stored() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-lockout");
        cpu_menu_role();
        on_chords(true, false, false);
        cpu_role_after_chord(1);
        let _ = service_at(&root, &none(), "2026-11-07", state(13), state(7), false);
        for _ in 0..crate::model::recording::LOCKOUT_FRAMES {
            assert!(drive_cpu(1).is_none());
        }
        assert_eq!(cpu_role_phase(), Some(Phase::Standby));
        for _ in 0..4 {
            assert_eq!(drive_cpu(0x8000), Some(0x8000));
            assert_eq!(cpu_role_phase(), Some(Phase::Standby));
        }
        on_chords(true, false, false);
        cpu_role_after_chord(1);
        let _stopped = service_at(&root, &none(), "2026-11-07", state(13), state(7), false);
        assert!(Library::new(root.join(MACROS_DIR)).list().unwrap().entries.is_empty());
    }

    #[test]
    fn cpu_role_record_stores_the_jab_and_plays_it_on_the_cpu() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-jab");
        cpu_menu_role();
        on_chords(true, false, false);
        cpu_role_after_chord(1);
        let started = service_at(&root, &none(), "2026-11-07", state(13), state(7), false);
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
        cpu_role_after_chord(1);
        assert!(cpu_role_phase().is_none());
        let stopped = service_at(&root, &none(), "2026-11-07", state(13), state(7), false);
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
            },
            None,
        );
        on_chords(false, false, true);
        let played = service_at(&root, &none(), "2026-11-07", state(13), state(7), false);
        if played.warp.is_some() {
            assert!(take_cpu_frame().is_none(), "no frame before the warp's restore lands");
            warp_landed();
            let _ = service_at(&root, &none(), "2026-11-07", state(13), state(7), false);
        }
        for (index, expected) in jab.iter().enumerate() {
            if index > 0 {
                let _ = service_at(&root, &none(), "2026-11-07", state(13), state(7), false);
            }
            let frame = take_cpu_frame().expect("play on cpu posts the stored frame");
            assert_eq!(frame.buttons, *expected);
            assert_eq!(playback_controller(frame).buttons, *expected);
        }
    }

    /// Eden `rc5-opus-cpu2` (2026-11-08): a CPU-role file begins on its first press (the jab).
    /// Play on CPU warped to the start save and fed frame 0 at once; the restore landed four
    /// frames later and undid the jab, so the CPU never acted. Frame 0 waits for the restore.
    #[test]
    fn a_warped_cpu_play_feeds_its_first_frame_after_the_restore() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-warp");
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
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
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert_eq!(tick.warp.as_deref(), Some(save_id.as_str()));
        assert!(take_cpu_frame().is_none(), "frame 0 must not be fed before the restore");
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(take_cpu_frame().is_none(), "the run holds while the restore is pending");

        warp_landed();
        let mut fed = Vec::new();
        for _ in 0..jab.len() {
            let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
            fed.push(take_cpu_frame().expect("a frame after the restore").buttons);
        }
        assert_eq!(fed, jab, "the whole file, from its first press, reaches the CPU");
    }

    #[test]
    fn a_cpu_play_without_a_start_save_feeds_at_once() {
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-nowarp");
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        let frames = vec![InputFrame { buttons: 33_281, ..InputFrame::default() }];
        let recording =
            super::super::macros::MacroRecording::new("Jab", "13.0.5", StartState::neutral(7, 0, 1.0), frames).unwrap();
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 1).unwrap();
        index.save(&root).unwrap();
        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(tick.warp.is_none());
        assert_eq!(take_cpu_frame().expect("no warp, no wait").buttons, 33_281);
    }

    /// Eden `rc6-covrec-diag1` (2026-11-08): Mash interrupts playback On, the CPU played its active macro,
    /// Player 1's jab put it in hitstun, and no `screen=playback mash=1` came. The CPU's mash read only the
    /// old recorder's playback, which was idle, so it never asked the library run to stop.
    #[test]
    fn a_library_macro_playing_on_the_cpu_is_the_playback_the_mash_and_covered_settings_see() {
        use crate::model::cpu::playback::covered_setting_applies;
        let _hold = hold();
        reset_host();
        let root = scratch("cpu-mash");
        let idle = PlaybackState::default();
        active_plain_macro(&root, Role::Cpu, 30);
        assert!(!cpu_playback(idle).running, "nothing plays yet");
        on_chords(false, false, true);
        tick(&root);
        assert!(take_cpu_frame().is_some());
        let seen = cpu_playback(idle);
        assert!(seen.possesses_cpu(), "the CPU's library run is a running playback: {seen:?}");
        assert!(!covered_setting_applies(seen, false));
        let recording = PlaybackState { recording: true, ..idle };
        assert!(cpu_playback(recording).recording, "the recorder's own state is kept");
        note_mash();
        tick(&root);
        assert!(take_cpu_frame().is_none(), "the mash stopped the CPU's run");
        assert!(!cpu_playback(idle).running);
    }

    /// A file of `n` distinct frames (buttons 1..=n), active on `role`, with no start save.
    fn active_plain_macro(root: &Path, role: Role, n: u32) -> Vec<u32> {
        let _ = service_at(root, &none(), "2026-11-08", state(8), state(7), false);
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
        let _ = service_at(root, &none(), "2026-11-08", state(8), state(7), false);
    }

    /// Eden `rc5-fid-measure1` (2026-11-08): the worker's ticks are not the game's frames. Two ticks
    /// between two AI frames overwrote a frame in the single slot, and an AI frame with no tick before it
    /// got no frame, so the AI's own controls stood inside the run. Every file frame reaches the CPU
    /// once, in order, one per AI frame, whatever the worker's timing.
    #[test]
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
    #[test]
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

    /// The Play chord again stops the run: nothing buffered is fed after the stop.
    #[test]
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
    #[test]
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

    #[test]
    fn cpu_role_player_path_still_stores_the_polls() {
        let _hold = hold();
        reset_host();
        let root = scratch("player-jab");
        player_menu_role();
        on_chords(true, false, false);
        cpu_role_after_chord(1);
        assert!(cpu_role_phase().is_none());
        let _ = service_at(&root, &none(), "2026-11-07", state(8), state(7), false);
        let frames = [1u32, 36_872, 36_872, 36_872, 33_281];
        for buttons in frames {
            let mut out = press(buttons);
            unsafe { on_player_poll(&mut out) };
            assert_eq!(out.buttons, buttons);
        }
        on_chords(true, false, false);
        cpu_role_after_chord(1);
        let _ = service_at(&root, &none(), "2026-11-07", state(8), state(7), false);
        let macros = Library::new(root.join(MACROS_DIR));
        let listed = macros.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let saved = macros.load(&listed.entries[0].id).unwrap();
        let buttons: Vec<u32> = saved.frames.iter().map(|frame| frame.buttons).collect();
        assert_eq!(buttons, frames);
    }

    #[test]
    fn a_record_writes_before_migration_when_the_slot_image_is_absent() {
        let _hold = hold();
        reset_host();
        let root = scratch("pending");
        on_chords(true, false, false);
        push_capture(ROLE_PLAYER, InputFrame { buttons: 4, ..InputFrame::default() });
        on_chords(true, false, false);
        let tick = service_pending(&root, state(8), state(7), false);
        assert!(!migrated());
        assert!(tick.diags.iter().any(|line| line.contains("recording=0")));
        assert_eq!(Library::new(root.join(MACROS_DIR)).list().unwrap().entries.len(), 1);
        let mut slots = none();
        slots[0] = Some(LegacySlot {
            frames: vec![InputFrame { buttons: 2, ..InputFrame::default() }],
            start: StartState::neutral(3, 0, 1.0),
        });
        let migrated_tick = service_at(&root, &slots, "2026-11-06", state(8), state(7), false);
        assert!(migrated());
        assert!(migrated_tick.diags.iter().any(|line| line == "screen=migrate once=1"));
    }

    #[test]
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
        set_strip_sample((0, Some(sample_combo())));
        post_effect(Effect::MakeCombo { source: source.clone(), chain: false });
        let playing = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert_eq!(playing.warp.as_deref(), Some(save_id.as_str()));
        assert!(take_player_frame().is_none(), "the source waits for its start save's restore");
        warp_landed();
        let fed = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(fed.diags.iter().all(|line| !line.contains("saved=1")), "the combo is made after the source has played");
        assert_eq!(take_player_frame().expect("source frame").buttons, 11);
        set_strip_sample((1, Some(sample_combo())));
        let saved = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
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

    #[test]
    fn make_combo_failures_notice_and_skip_the_keyboard_online() {
        let _hold = hold();
        reset_host();
        let root = scratch("make-fail");
        let source = one_macro(&root, "Bare", 3, 8, None);
        set_strip_sample((0, Some(crate::model::combo::Combo::default())));
        post_effect(Effect::MakeCombo { source: source.clone(), chain: false });
        let _ = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert_eq!(take_player_frame().expect("the source plays").buttons, 3);
        let empty = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(empty.diags.iter().any(|line| line == "screen=notice kind=combo-empty"));
        assert!(empty.notices.contains(&EMPTY_COMBO));
        assert!(crate::model::combo_record::ComboLibrary::new(root.join("combos")).list().unwrap().entries.is_empty());

        reset_host();
        let gone = one_macro(&root, "Gone", 4, 8, Some("deleted-save"));
        set_strip_sample((0, Some(sample_combo())));
        post_effect(Effect::MakeCombo { source: gone, chain: false });
        let missing = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(missing.warp.is_none());
        assert!(missing.diags.iter().any(|line| line == "screen=notice kind=macro-start-save-missing"));
        assert_eq!(take_player_frame().expect("plays anyway").buttons, 4);

        reset_host();
        let present = one_macro(&root, "Later", 6, 8, None);
        let chain = Chain::new("Gap", vec!["missing-macro".into(), present]).unwrap();
        let chain_id = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&chain).unwrap();
        post_effect(Effect::MakeCombo { source: chain_id, chain: true });
        let gap = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(gap.warp.is_none());
        assert_eq!(take_player_frame().expect("skips the missing step").buttons, 6);

        reset_host();
        post_effect(Effect::Keyboard { header: "Name macro", draft: "Online".into() });
        let online = service_at(&root, &none(), "2026-11-06", state(8), state(7), true);
        assert!(online.keyboard.is_none());
        assert!(online.diags.iter().all(|line| !line.contains("screen=rename")));
    }

    /// Eden rc6-seq-explore1 `seq-make` (2026-11-08 12:28Z, primary inbox 13:31Z): a played macro that lands no hit was
    /// saved as the strip's PREVIOUS exchange. RECORDING.md: Make combo records the exchange the file plays. Only a
    /// link the strip captured after Make combo began counts; none gives the empty notice and no file.
    #[test]
    fn make_combo_with_no_hit_saves_nothing_not_the_previous_exchange() {
        let _hold = hold();
        reset_host();
        let root = scratch("make-stale");
        let source = one_macro(&root, "Whiff", 3, 8, None);
        set_strip_sample((4, Some(sample_combo())));
        post_effect(Effect::MakeCombo { source, chain: false });
        let _ = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert_eq!(take_player_frame().expect("the source plays").buttons, 3);
        set_strip_sample((4, Some(sample_combo())));
        let done = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(done.diags.iter().any(|line| line == "screen=notice kind=combo-empty"), "{:?}", done.diags);
        assert!(done.diags.iter().all(|line| !line.contains("saved=1")));
        assert!(crate::model::combo_record::ComboLibrary::new(root.join("combos")).list().unwrap().entries.is_empty());
    }

    /// Operator 2026-11-08 (primary inbox 13:08Z): *+ Save last combo…* saves the latest exchange as a combo file, as
    /// rc.4's COMBO-HELPER did (`combos/<id>.json` on the generic store): named as typed or Combo N, recorded on Player
    /// 1's fighter, hot, and the drill (RECORDING.md). With no link on the strip it says so and writes nothing.
    #[test]
    fn save_last_combo_writes_the_latest_exchange_names_it_and_drills_it() {
        let _hold = hold();
        reset_host();
        let root = scratch("save-last");
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        set_fighter_label("Mario");
        let combos = crate::model::combo_record::ComboLibrary::new(root.join("combos"));
        set_strip_sample((0, None));
        post_effect(Effect::SaveLastCombo { name: String::new() });
        let empty = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(empty.diags.iter().any(|line| line == "screen=notice kind=combo-empty"), "{:?}", empty.diags);
        assert!(empty.notices.contains(&EMPTY_COMBO));
        assert!(combos.list().map(|listed| listed.entries.is_empty()).unwrap_or(true));
        set_strip_sample((3, Some(sample_combo())));
        post_effect(Effect::SaveLastCombo { name: "Ryu BnB".into() });
        let saved = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(saved.diags.iter().any(|line| line.starts_with("screen=save-last-combo saved=")), "{:?}", saved.diags);
        let listed = combos.list().unwrap();
        assert_eq!(listed.entries.len(), 1);
        let id = listed.entries[0].id.clone();
        let file = combos.load(&id).unwrap();
        assert_eq!((file.name.as_str(), file.combo, file.recorded_on.as_deref()), ("Ryu BnB", sample_combo(), Some("Mario")));
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.drill().map(|drill| (drill.kind, drill.id.as_str())), Some((DrillKind::Combo, id.as_str())));
        assert_eq!(index.hot(Kind::Combo).first(), Some(&id));
        post_effect(Effect::SaveLastCombo { name: String::new() });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        let names: Vec<String> = combos.list().unwrap().entries.into_iter().map(|entry| entry.name).collect();
        assert!(names.iter().any(|name| name == "Combo 1"), "{names:?}");
    }

    #[test]
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
        let held = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(held.diags.iter().any(|line| line.contains("screen=mismatch ask=macro ")));
        assert!(take_player_frame().is_none());
        answer_mismatch(MismatchChoice::Keep);
        let kept = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(kept.switch_kind.is_none());
        assert_eq!(take_player_frame().expect("keep plays").buttons, 15);

        reset_host();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&first), 3).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Player, 8);
        on_chords(false, true, false);
        let _ = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        answer_mismatch(MismatchChoice::Switch);
        let switched = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert_eq!(switched.switch_kind, Some(13));
        assert!(switched.switch_player);
        assert_eq!(take_player_frame().expect("switch plays").buttons, 15);

        reset_host();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&chain_id), 4).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Player, 8);
        on_chords(false, true, false);
        let chain_ask = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(chain_ask.diags.iter().any(|line| line.contains("ask=macro1")));
        answer_mismatch(MismatchChoice::Cancel);
        let cancelled = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(take_player_frame().is_none());
        assert!(cancelled.switch_kind.is_none());
    }

    #[test]
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
        let held = service_pending(&root, state(0), state(0), false);
        assert!(held.diags.iter().any(|line| line.contains("screen=mismatch ask=macro1 rows=")), "{:?}", held.diags);
        assert!(held.mismatch.is_some());
        assert!(!pending(), "a question waiting for its answer is not work");
        answer_mismatch(MismatchChoice::Keep);
        assert!(pending(), "the answer must reach the tick");
        let kept = service_pending(&root, state(0), state(0), false);
        assert!(kept.diags.iter().any(|line| line == "screen=mismatch ask=macro1 answer=keep"));
        assert!(kept.switch_kind.is_none());
        assert_eq!(take_cpu_frame().expect("keep plays macro 1 on the current fighter").buttons, 15);

        reset_host();
        set_role_fighter(Role::Cpu, 0);
        on_chords(false, false, true);
        let _ = service_pending(&root, state(0), state(0), false);
        answer_mismatch(MismatchChoice::Switch);
        assert!(pending());
        let switched = service_pending(&root, state(0), state(0), false);
        assert!(switched.diags.iter().any(|line| line == "screen=mismatch ask=macro1 answer=switch"));
        assert_eq!(switched.switch_kind, Some(13));
        assert!(!switched.switch_player, "the CPU's chain switches the CPU");

        reset_host();
        set_role_fighter(Role::Cpu, 0);
        on_chords(false, false, true);
        let _ = service_pending(&root, state(0), state(0), false);
        set_role_fighter(Role::Cpu, 13);
        answer_mismatch(MismatchChoice::Keep);
        let _ = service_pending(&root, state(13), state(13), false);
        assert!(lock(&ANSWER).is_none(), "an answer the fighters no longer need is dropped");
    }

    #[test]
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
            let tick = service_pending(&root, state(2), state(2), false);
            let ask = tick.mismatch.expect("the recorded fighter is asked about");
            assert_eq!((ask.role, ask.recorded_kind, ask.current_kind), (side, 1, 2), "{role:?}");
        }
    }

    #[test]
    fn a_changed_combo_drill_asks_and_switch_names_the_recorded_fighter() {
        let _hold = hold();
        reset_host();
        let root = scratch("drill");
        let record = crate::model::combo_record::ComboRecord::new("Drill".into(), sample_combo(), Some("Ryu".into()));
        let id = crate::model::combo_record::ComboLibrary::new(root.join("combos")).save_new(&record).unwrap();
        note_drill(Some("seed".into()));
        note_drill(Some(id));
        set_fighter_label("Mario");
        let asked = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(asked.diags.iter().any(|line| line.contains("screen=mismatch ask=combo")));
        assert!(asked.switch_name.is_none());
        answer_mismatch(MismatchChoice::Switch);
        let switched = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert_eq!(switched.switch_name.as_deref(), Some("Ryu"));
        assert!(switched.switch_player);
    }

    #[test]
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
        };
        publish_selection(&root, &selection, None);
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.loop_count(Role::Player), LoopCount::Finite(5));
        assert_eq!(index.loop_count(Role::Cpu), LoopCount::Infinite);
    }

    #[test]
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

    #[test]
    fn a_sequence_plan_stages_its_combos_in_order_and_skips_a_deleted_one() {
        use crate::model::frame_strip::Move;
        let root = scratch("seq-plan");
        let ryu = one_combo(&root, "Ryu BnB", Move::Jab);
        let nair = one_combo(&root, "Nair loop combo", Move::Aerial);
        let id = one_sequence(&root, "Corner to kill", &[ryu.clone(), "deleted-combo".into(), nair.clone()]);
        let plan = sequence_plan(&root, &id).expect("the sequence file loads");
        assert_eq!(plan.name, "Corner to kill");
        assert_eq!(plan.steps.iter().map(|s| (s.id.as_str(), s.name.as_str())).collect::<Vec<_>>(), [
            (ryu.as_str(), "Ryu BnB"),
            (nair.as_str(), "Nair loop combo")
        ]);
        assert_eq!(plan.steps[0].combo.get(0).map(|l| l.from), Some(Move::Jab), "the combo the helper grades");
        assert_eq!((plan.streak, plan.best), (0, 0));
        assert!(sequence_plan(&root, "no-such-sequence").is_none());
    }

    #[test]
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

    #[test]
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
        let catalog = super::super::rr_runtime::load_catalog(&root).unwrap().catalog;
        let shown = catalog.sequences.iter().find(|s| s.id == id).unwrap();
        assert_eq!((shown.streak, shown.best), (4, 9), "the Connections panel reads the file");
        assert!(!save_sequence_progress(&root, "no-such-sequence", 1, 1));
    }

    #[test]
    fn a_rename_or_a_saved_streak_keeps_the_plan_and_a_new_order_does_not() {
        use crate::model::frame_strip::Move;
        let root = scratch("seq-same");
        let ryu = one_combo(&root, "Ryu BnB", Move::Jab);
        let nair = one_combo(&root, "Nair loop combo", Move::Aerial);
        let id = one_sequence(&root, "Corner to kill", &[ryu.clone(), nair.clone()]);
        let plan = sequence_plan(&root, &id).unwrap();
        rename_combo(&root, &ryu, "Ryu opener".into());
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

    #[test]
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

    #[test]
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

    #[test]
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
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        let file = Store::<Sequence>::new(root.join(SEQUENCES_DIR)).load(&id).unwrap();
        assert_eq!(file.combos, [nair.clone(), ryu.clone()], "Edit combos is saved");
        post_effect(Effect::DeleteFile { kind: FileKind::Combo, id: ryu.clone() });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(crate::model::combo_record::ComboLibrary::new(root.join("combos")).load(&ryu).is_err(), "the combo is gone");
        let plan = sequence_plan(&root, &id).unwrap();
        assert_eq!(plan.steps.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), [nair.as_str()], "the sequence skips it");
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_drill(Some(Drill { kind: DrillKind::Sequence, id: id.clone() }), 1).unwrap();
        index.save(&root).unwrap();
        post_effect(Effect::DeleteFile { kind: FileKind::Sequence, id: id.clone() });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(sequence_plan(&root, &id).is_none(), "the sequence is gone");
        assert!(RecordingIndex::load(&root).unwrap().drill().is_none(), "and is no longer the drill");
        let chain_macro = one_macro(&root, "Jab", 1, 8, None);
        let chain = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&Chain::new("Chain 1", vec![chain_macro.clone()]).unwrap()).unwrap();
        post_effect(Effect::WriteChainSteps { id: chain.clone(), steps: vec![] });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(Store::<Chain>::new(root.join(MACROS_DIR)).load(&chain).unwrap().recordings.is_empty(), "Edit chain is saved");
        post_effect(Effect::DeleteFile { kind: FileKind::Chain, id: chain.clone() });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(Store::<Chain>::new(root.join(MACROS_DIR)).load(&chain).is_err(), "the chain is gone");
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&chain_macro), 2).unwrap();
        index.save(&root).unwrap();
        post_effect(Effect::DeleteFile { kind: FileKind::Macro, id: chain_macro.clone() });
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(Library::new(root.join(MACROS_DIR)).load(&chain_macro).is_err(), "the macro is gone");
        assert!(RecordingIndex::load(&root).unwrap().active(Role::Player).is_none(), "and is no longer active");
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
    #[test]
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
        };
        publish_selection(&root, &selection, None);
        assert_eq!(bound_sync(&root).unwrap().unwrap()[0].x, 40.0, "A on Right binds Right for Load");
        save_captured([at_x(5.0), at_x(6.0)]);
        assert!(write_saved(&root));
        let written = saves.load(&right).unwrap();
        assert_eq!((written.player.x, written.cpu.x), (5.0, 6.0), "Save writes the bound file");
        assert!(bound_sync(&root).is_none(), "the slot already holds what Save wrote");
        publish_selection(&root, &Selection { bound: None, ..selection }, None);
        assert_eq!(bound_sync(&root), Some(None), "nothing bound empties the slot");
        save_captured([at_x(1.0), at_x(2.0)]);
        assert!(write_saved(&root));
        let made = RecordingIndex::load(&root).unwrap().bound().unwrap().to_owned();
        assert_eq!(saves.load(&made).unwrap().name, "Save 1");
        assert_eq!(saves.load(&made).unwrap().player.x, 1.0);
    }

    /// Primary 2026-11-08 #70 (a): Edit placement edits the page role's active macro, the file Play plays. Eden
    /// rc6-covrec-diag2 cov-edit-placement refused with a macro active, and Play never read a placement. The anchor is
    /// where the start save puts the role; Play's warp adds the file's offset (or a kept one) to that role only.
    #[test]
    fn edit_placement_targets_the_active_macro_and_play_warps_with_its_offset() {
        let _hold = hold();
        reset_host();
        let root = scratch("placement");
        migrate_at_boot(&root, &none(), "2026-11-08").unwrap();
        set_record_role(Role::Cpu);
        assert_eq!(placement_target(&root), Err(PLACE_NO_MACRO));
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
        assert_eq!(placement_target(&root), Err(PLACE_NO_START), "no warp, no anchor");
        recording.set_start_save(Some(&start)).unwrap();
        recording.set_placement(Placement { delta_x: 1.5, delta_y: 0.0, ..Placement::default() }).unwrap();
        let id = macros.save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&id), 2).unwrap();
        index.save(&root).unwrap();
        let target = placement_target(&root).expect("the CPU's active macro");
        assert_eq!((target.role, target.id.as_str(), target.anchor), (Role::Cpu, id.as_str(), (30.0, 2.0)));
        assert_eq!(target.recording.placement.delta_x, 1.5);

        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert_eq!(tick.warp.as_deref(), Some(start.as_str()));
        assert_eq!(tick.warp_place.map(|(role, place)| (role, place.delta_x)), Some((Role::Cpu, 1.5)));
        let pair = warp_pair(saves.load(&start).unwrap(), tick.warp_place);
        assert_eq!((pair[0].x, pair[1].x, pair[1].y), (-30.0, 31.5, 2.0), "only the playing role moves");
        let lifted = warp_pair(
            saves.load(&start).unwrap(),
            Some((Role::Cpu, Placement { delta_x: 0.0, delta_y: 3.0, ..Placement::default() })),
        );
        assert_eq!((lifted[1].y, lifted[1].situation_kind), (5.0, SITUATION_AIR), "a raised start is in the air");

        keep_placement(&id, Placement { delta_x: -2.0, delta_y: 0.0, ..Placement::default() });
        warp_landed();
        on_chords(false, false, true);
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        on_chords(false, false, true);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert_eq!(tick.warp_place.map(|(_, place)| place.delta_x), Some(-2.0), "Keep wins until the game closes");
        assert_eq!(macros.load(&id).unwrap().placement.delta_x, 1.5, "Keep leaves the file alone");

        // A chain warps to macro 1's start save and plays with macro 1's placement, so Edit placement edits macro 1.
        let chain = Chain::new("Pressure", vec![id.clone(), loose.clone()]).unwrap();
        let chain_id = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&chain).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&chain_id), 3).unwrap();
        index.save(&root).unwrap();
        let target = placement_target(&root).expect("a chain edits its macro 1");
        assert_eq!((target.id.as_str(), target.anchor), (id.as_str(), (30.0, 2.0)));
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
    #[test]
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
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(tick.mismatch.is_some(), "the recorded fighter is asked about");
        answer_mismatch(MismatchChoice::Switch);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert_eq!(tick.switch_kind, Some(0x48));
        assert_eq!(tick.switch_mii, Some(crate::engine::mii::replay::Target::mii(configuration)), "the recorded Mii");

        reset_host();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some(&plain), 2).unwrap();
        index.save(&root).unwrap();
        set_role_fighter(Role::Cpu, 7);
        on_chords(false, false, true);
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        answer_mismatch(MismatchChoice::Switch);
        let tick = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert_eq!((tick.switch_kind, tick.switch_mii), (Some(1), None), "any other fighter keeps the kind route");
    }

    /// Primary 2026-11-08 #69 (b): "Setups stay a separate library from save states"
    /// (LANE-GROK-RECORDING-REDESIGN-REPORT.md:322). Eden rc6-covrec-diag2 cov-setup-restore: the restore moved the
    /// fighters and also wrote them into the bound save. A restore only moves the fighters: the worker writes nothing,
    /// the bound file keeps its pair, and the active slot gets the bound save back, so Load still warps to it.
    #[test]
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
        assert!(!write_saved(&root), "a setup restore hands the worker nothing to write");
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
        }, None);
        assert_eq!(bound_sync(&root), Some(None), "nothing bound");
        setup_restored();
        assert_eq!(bound_sync(&root), Some(None), "with nothing bound the restored slot is emptied again");
        assert_eq!(RecordingIndex::load(&root).unwrap().bound(), None, "and no save is made for it");
    }

    /// Primary 2026-11-08 23:03Z, SETUP-Y-BIND (cov-rec #69 (b)): Y in the Setup library binds a setup without
    /// restoring it. The setup stays in the active slot, so Load returns to it, until the next bound-save change. It
    /// never writes the bound save file. A (bind and restore) still gives the slot the bound save back.
    #[test]
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
        assert!(!write_saved(&root), "Y hands the worker nothing to write");
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
        StartPose { fighter_kind: kind, situation_kind: 0, x, y: 0.0, percent: 0.0, lr }
    }

    /// RECORDING-REDESIGN "Start save: the save state this macro starts at", and PLAY-MOMENTUM (primary 2026-11-08
    /// 23:03Z, reading (b)): the start save is where Record started, not where it stopped (the worker wrote the live pose
    /// of the stop), and the macro keeps the recorded role's speed and facing at that moment (it stored zero and 1.0).
    #[test]
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
        let _ = service_at(&root, &none(), "2026-11-08", stopped, state(7), false);
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
        let _ = service_at(&root, &none(), "2026-11-08", state(13), state(7), false);
        let cpu = macros.list().unwrap().entries.iter().map(|e| macros.load(&e.id).unwrap()).find(|r| r.start.fighter_kind == 7)
            .expect("the CPU's macro");
        assert_eq!((cpu.start.speed_x, cpu.start.speed_y, cpu.start.facing), (2.5, -1.0, -1.0));
        let save = saves.load(cpu.start_save.as_deref().unwrap()).unwrap();
        assert_eq!((save.player.x, save.cpu.x), (-5.0, 5.0));
    }

    /// Plays the Player's active macro (recorded with `start`, kept `placement` if any) once and returns the start speed
    /// the role gets on the game frame that takes its first input, checking it is given only once.
    fn momentum_play(tag: &str, start: StartState, placement: Option<Placement>, start_save: bool) -> Option<StartSpeed> {
        reset_host();
        let root = scratch(tag);
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        let save = Store::<NamedSave>::new(root.join(SAVES_DIR)).save_new(&NamedSave::new("Run", state(8), state(7)).unwrap()).unwrap();
        let frames = vec![InputFrame { buttons: 4, ..InputFrame::default() }, InputFrame { buttons: 5, ..InputFrame::default() }];
        let mut recording = MacroRecording::new("Run", "13.0.5", start, frames).unwrap();
        if start_save {
            recording.set_start_save(Some(&save)).unwrap();
        }
        let id = Library::new(root.join(MACROS_DIR)).save_new(&recording).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&id), 1).unwrap();
        index.save(&root).unwrap();
        if let Some(placement) = placement {
            keep_placement(&id, placement);
        }
        on_chords(false, true, false);
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
        assert!(take_start_speed(Role::Player).is_none(), "{tag}: nothing before the run's first input");
        warp_landed();
        let _ = service_at(&root, &none(), "2026-11-08", state(8), state(7), false);
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
    #[test]
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
}
