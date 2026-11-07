//! Recording worker decisions the host can run. The Switch wrapper performs the notices,
//! the save-restore request, and the keyboard offer this module names.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use super::library::{
    Chain, LegacySlot, NamedSave, RecordingIndex, Role, StartSaveState, MACROS_DIR, SAVES_DIR,
};
use super::macros::{FrameSnapshot, InputFrame, Library, MAX_FRAMES};
use super::playback::{Book, Playback};
use super::rr_runtime::{
    chain_mismatch, chord_notice, civil_date, crop_combo_file, crop_macro_file, delete_save,
    drive_mash, drive_play, drive_step, duplicate_save, finish_recording, make_combo,
    migrate_at_boot, offer_keyboard, push_frame, start_save_notice, toggle_record, Capture,
    KeyboardOffer, Tape, READ_WINDOW,
    RESIDENT_FRAMES,
};
use crate::model::input_delay::MappedInputs;
use crate::model::recording_menu::Effect;
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
    if !MIGRATED.load(Ordering::Acquire) && migrate_at_boot(root, slots, date).is_ok() {
        MIGRATED.store(true, Ordering::Release);
        tick.diags.push("screen=migrate once=1".to_string());
    }
    let edge = RECORD_EDGE.swap(0, Ordering::AcqRel);
    let play_player = PLAY_PLAYER.swap(false, Ordering::AcqRel);
    let play_cpu = PLAY_CPU.swap(false, Ordering::AcqRel);
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
        let (role, frames) = taken_frames();
        let mut capture = Capture::idle();
        toggle_record(&mut capture, role);
        for frame in frames {
            push_frame(&mut capture, role, frame);
        }
        if let Ok(stop) = finish_recording(root, &capture, player.clone(), cpu.clone(), unix_now())
        {
            if let Some(text) = stop.notice() {
                tick.notices.push(text);
            }
        }
    }
    if !(play_player || play_cpu || mash || menu_pending() || playing()) {
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
        apply_menu(root, &book, online, &mut tick);
    }
    tick
}

fn play_role(root: &Path, machine: &mut Playback, book: &Book, role: Role, tick: &mut Tick) {
    show_chord_notice(root, tick);
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
    ask_mismatch(root, role, tick);
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

fn ask_mismatch(root: &Path, role: Role, tick: &mut Tick) {
    let current = CURRENT_KIND.load(Ordering::Acquire);
    if current == u32::MAX {
        return;
    }
    let Some(id) = RecordingIndex::load(root)
        .ok()
        .and_then(|index| index.active(role).map(str::to_owned))
    else {
        return;
    };
    let Ok(chain) = Store::<Chain>::new(root.join(MACROS_DIR)).load(&id) else {
        return;
    };
    let Some(first) = chain.recordings.first() else {
        return;
    };
    let Ok(recording) = Library::new(root.join(MACROS_DIR)).load(first) else {
        return;
    };
    let recorded = recording.start.fighter_kind.to_string();
    let now = (current as i32).to_string();
    let steps = [(first.as_str(), recorded.as_str())];
    if chain_mismatch(&steps, &now, true, true).is_some() {
        tick.diags.push("screen=mismatch ask=macro1".to_string());
    }
}

fn apply_menu(root: &Path, book: &Book, online: bool, tick: &mut Tick) {
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
        Effect::Crop { source, combo } => {
            tick.diags.push(format!(
                "screen=crop source={source} combo={}",
                i32::from(combo)
            ));
            let _ = write_crop(root, &source, combo);
        }
        Effect::MakeCombo { .. } => {
            if let Some(id) = make_combo(book).warp {
                tick.warp = Some(id);
            }
        }
        Effect::DuplicateSave { source } => {
            let _ = duplicate_save(root, &source, unix_now());
        }
        Effect::DeleteSave { source } => {
            let _ = delete_save(root, &source, unix_now());
        }
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
        let saved = macros.load(&listed.entries[0].id).unwrap();
        assert!(saved.start_save.is_some());
        assert_eq!(saved.frames[0].buttons, 4);
        assert_eq!(
            RecordingIndex::load(&root).unwrap().active(Role::Player),
            None
        );
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
        post_effect(Effect::Crop { source: macro_id.clone(), combo: false });
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
}
