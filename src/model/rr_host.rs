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

#[cfg(target_os = "switch")]
pub(crate) fn capture_role() -> u8 {
    CAP_ROLE.load(Ordering::Acquire)
}

pub fn take_player_frame() -> Option<InputFrame> {
    take_frame(&PLAYER_FRAMES)
}

#[cfg(target_os = "switch")]
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
