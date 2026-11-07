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
use super::macros::{FrameSnapshot, FrameWindow, InputFrame, Library, MAX_FRAMES};
use super::playback::{Book, Playback, StartLink};
use super::rr_runtime::{
    chord_notice, civil_date, crop_combo_file, crop_macro_file, delete_save,
    drive_mash, drive_play, drive_step, duplicate_save, finish_recording, migrate_at_boot,
    offer_keyboard, push_frame, save_made_combo, start_save_notice, toggle_record, Capture,
    KeyboardOffer, Tape, EMPTY_COMBO, READ_WINDOW, RESIDENT_FRAMES,
};
use crate::model::combo::Combo;
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
static PLAYER_BITS: AtomicU64 = AtomicU64::new(0);
static PLAYER_READY: AtomicBool = AtomicBool::new(false);
static CPU_BITS: AtomicU64 = AtomicU64::new(0);
static CPU_READY: AtomicBool = AtomicBool::new(false);
static CURRENT_KIND: AtomicU32 = AtomicU32::new(u32::MAX);
static RUN_PLAYER: AtomicBool = AtomicBool::new(false);
static RUN_CPU: AtomicBool = AtomicBool::new(false);
static MACHINE: Mutex<Option<Playback>> = Mutex::new(None);
static RESIDENT: Mutex<Option<Resident>> = Mutex::new(None);
static MENU_EFFECT: Mutex<Effect> = Mutex::new(Effect::None);
static CROP: Mutex<Option<CropWindow>> = Mutex::new(None);
static DEFER_MIGRATE: AtomicBool = AtomicBool::new(false);
static ROLE_KIND: [AtomicI32; 2] = [const { AtomicI32::new(-1) }; 2];
static STRIP: Mutex<Option<Combo>> = Mutex::new(None);
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
static CPU_DRIVE: Mutex<CpuRoleDrive> = Mutex::new(CpuRoleDrive::idle());

struct MakeJob {
    source: String,
    ready: bool,
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
    pub notices: Vec<&'static str>,
    pub diags: Vec<String>,
    pub keyboard: Option<KeyboardOffer>,
    pub mismatch: Option<MismatchAsk>,
    /// Switch uses the existing exact-fighter route for this kind. Keep leaves it empty.
    pub switch_kind: Option<i32>,
    /// A combo records a fighter name. The wrapper resolves it to a kind.
    pub switch_name: Option<String>,
    pub switch_player: bool,
}

/// The existing Fighter Mismatch screen, named for the wrapper that opens it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MismatchAsk {
    pub origin: &'static str,
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
}

fn playing() -> bool {
    RUN_PLAYER.load(Ordering::Acquire) || RUN_CPU.load(Ordering::Acquire)
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
    let mut drive = lock(&CPU_DRIVE);
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
        let mut drive = lock(&CPU_DRIVE);
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
    lock(&CPU_DRIVE).control(ctx)
}

pub fn cpu_role_phase() -> Option<Phase> {
    if !CPU_ROLE_ON.load(Ordering::Acquire) {
        return None;
    }
    let drive = lock(&CPU_DRIVE);
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
    take_frame(&PLAYER_READY, &PLAYER_BITS)
}

#[cfg(any(test, target_os = "switch"))]
pub(crate) fn take_cpu_frame() -> Option<InputFrame> {
    take_frame(&CPU_READY, &CPU_BITS)
}

/// Player 1's poll. Injects one posted frame, then stores the output when Player 1 is the recorded role.
///
/// # Safety
/// `out` is null or a live mapped-input record for this poll.
pub unsafe fn on_player_poll(out: *mut MappedInputs) {
    if out.is_null() {
        return;
    }
    if let Some(frame) = take_player_frame() {
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
        if let Ok(stop) = finish_recording(root, &capture, player.clone(), cpu.clone(), unix_now())
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
        || lock(&DRILL_ASK).is_some()
        || lock(&MAKE).is_some()
        || (lock(&HELD).is_some() && lock(&ANSWER).is_some());
    if !busy {
        return tick;
    }
    let Ok(book) = load_book(root) else {
        return tick;
    };
    let mut machine = lock(&MACHINE);
    let machine = machine.get_or_insert_with(Playback::new);
    if mash {
        drive_mash(machine, Role::Player, true);
        drive_mash(machine, Role::Cpu, true);
        RUN_PLAYER.store(false, Ordering::Release);
        RUN_CPU.store(false, Ordering::Release);
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

pub fn set_strip_sample(combo: Option<Combo>) {
    *lock(&STRIP) = combo;
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
            Some(MismatchChoice::Cancel) => return,
            Some(MismatchChoice::Keep) => {}
            Some(MismatchChoice::Switch) => {
                tick.switch_kind = Some(ask.recorded_kind);
                tick.switch_player = role == Role::Player;
            }
        }
    }
    let effect = drive_play(machine, role, book);
    let word = if role == Role::Player {
        "player"
    } else {
        "cpu"
    };
    if let Some(id) = effect.warp {
        tick.warp = Some(id.to_owned());
        tick.diags
            .push(format!("screen=playback role={word} warp=1"));
    }
    note_missing_start(root, book, role, tick);
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

fn step_role(root: &Path, machine: &mut Playback, book: &Book, role: Role) {
    match drive_step(machine, role, book) {
        Some(played) => publish(role, resident_frame(root, &played)),
        None => run_flag(role).store(false, Ordering::Release),
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
    let (ready, bits) = match role {
        Role::Player => (&PLAYER_READY, &PLAYER_BITS),
        Role::Cpu => (&CPU_READY, &CPU_BITS),
    };
    bits.store(pack(frame), Ordering::Release);
    ready.store(true, Ordering::Release);
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
    }
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
        StartLink::Linked(id) => tick.warp = Some(id.to_string()),
        StartLink::None => {}
    }
    let effect = machine.play_file(Role::Player, book, source);
    if effect.started {
        RUN_PLAYER.store(true, Ordering::Release);
        step_role(root, machine, book, Role::Player);
    }
    *lock(&MAKE) = Some(MakeJob { source: source.to_string(), ready: true });
    tick.diags.push(format!("screen=make-combo source={source} playing={}", i32::from(effect.started)));
    if online {
        tick.keyboard = None;
    }
}

fn finish_make(root: &Path, tick: &mut Tick) {
    let ready = lock(&MAKE).as_ref().is_some_and(|job| job.ready);
    if !ready || RUN_PLAYER.load(Ordering::Acquire) {
        return;
    }
    let Some(job) = lock(&MAKE).take() else {
        return;
    };
    let sample = lock(&STRIP).take();
    let Some(combo) = sample.filter(|combo| !combo.is_empty()) else {
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

fn take_frame(ready: &AtomicBool, bits: &AtomicU64) -> Option<InputFrame> {
    if ready.swap(false, Ordering::AcqRel) {
        Some(unpack(bits.load(Ordering::Acquire)))
    } else {
        None
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
    PLAYER_READY.store(false, Ordering::Release);
    CPU_READY.store(false, Ordering::Release);
    CURRENT_KIND.store(u32::MAX, Ordering::Release);
    RUN_PLAYER.store(false, Ordering::Release);
    RUN_CPU.store(false, Ordering::Release);
    *lock(&MACHINE) = None;
    *lock(&RESIDENT) = None;
    *lock(&MENU_EFFECT) = Effect::None;
    *lock(&CROP) = None;
    *lock(&STRIP) = None;
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
    *lock(&CPU_DRIVE) = CpuRoleDrive::idle();
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
        assert!(take_player_frame().is_none());
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
        let _ = service_at(&root, &none(), "2026-11-07", state(13), state(7), false);
        for (index, expected) in jab.iter().enumerate() {
            if index > 0 {
                let _ = service_at(&root, &none(), "2026-11-07", state(13), state(7), false);
            }
            let frame = take_cpu_frame().expect("play on cpu posts the stored frame");
            assert_eq!(frame.buttons, *expected);
            assert_eq!(playback_controller(frame).buttons, *expected);
        }
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
        set_strip_sample(Some(sample_combo()));
        post_effect(Effect::MakeCombo { source: source.clone(), chain: false });
        let playing = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert_eq!(playing.warp.as_deref(), Some(save_id.as_str()));
        assert_eq!(take_player_frame().expect("source frame").buttons, 11);
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
        set_strip_sample(Some(crate::model::combo::Combo::default()));
        post_effect(Effect::MakeCombo { source: source.clone(), chain: false });
        let _ = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        let empty = service_at(&root, &none(), "2026-11-06", state(8), state(7), false);
        assert!(empty.diags.iter().any(|line| line == "screen=notice kind=combo-empty"));
        assert!(empty.notices.contains(&EMPTY_COMBO));
        assert!(crate::model::combo_record::ComboLibrary::new(root.join("combos")).list().unwrap().entries.is_empty());

        reset_host();
        let gone = one_macro(&root, "Gone", 4, 8, Some("deleted-save"));
        set_strip_sample(Some(sample_combo()));
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
}
