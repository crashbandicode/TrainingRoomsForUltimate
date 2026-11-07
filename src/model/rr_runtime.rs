//! Host decisions for the recording runtime. The hook only reads a filled window.
//! Card IO and allocation stay on the worker.

use std::path::Path;

use super::library::{
    migrate, Error, Kind, LegacySlot, LoopCount, NamedSave, RecordingIndex, Role,
    StartSaveState, Use, MACROS_DIR, SAVES_DIR,
};
use super::macro_ui::macro_panel::{fighter_gate, FighterGate, FighterOrigin};
use super::macros::{
    FrameSnapshot, FrameWindow, InputFrame, Library, MacroRecording, StartState, MAX_FRAMES,
};
use super::playback::{Book, Playback};
use crate::model::combo::{Combo, LinkRec};
use crate::model::combo_record::{ComboLibrary, ComboRecord};
use crate::model::state::record::SavedState;
use crate::model::store::Store;

/// One minute at 60 fps. Five hot macros stay resident.
pub const RESIDENT_FRAMES: usize = 3_600;
/// The worker reads this many frames past the resident minute.
pub const READ_WINDOW: usize = 600;
pub const MISSING_START_SAVE: &str = "No start save. This macro plays from where the fighters are.";
pub const CHORD_NOTICE: &str =
    "Record, Play on Player 1, Play on CPU, and the recording pop-up replace Input Record and Input Playback.";
pub const EMPTY_RECORDING: &str = "Recording was empty";

/// Frames the hook may index. The rest of a long file stays out of this struct until the worker reads a window.
#[derive(Clone, Debug)]
pub struct Tape {
    resident: Vec<InputFrame>,
    window: Vec<InputFrame>,
    window_start: usize,
    len: usize,
    reads: Vec<(usize, usize)>,
}

impl Tape {
    pub fn load(frames: &[InputFrame]) -> Self {
        let n = frames.len().min(RESIDENT_FRAMES);
        Self {
            resident: frames[..n].to_vec(),
            window: Vec::new(),
            window_start: 0,
            len: frames.len(),
            reads: Vec::new(),
        }
    }

    pub fn resident_len(&self) -> usize {
        self.resident.len()
    }

    pub fn window_len(&self) -> usize {
        self.window.len()
    }

    pub fn reads(&self) -> &[(usize, usize)] {
        &self.reads
    }

    /// Hook path. It does not read the card and it does not allocate.
    pub fn hook_frame(&self, index: usize) -> Option<InputFrame> {
        if index < self.resident.len() {
            return Some(self.resident[index]);
        }
        if index >= self.window_start && index < self.window_start + self.window.len() {
            return Some(self.window[index - self.window_start]);
        }
        None
    }

    /// Worker path. The closure is the only reader.
    pub fn fill_window(
        &mut self,
        index: usize,
        mut read: impl FnMut(usize, usize) -> Vec<InputFrame>,
    ) {
        if index < self.resident.len() || index >= self.len {
            return;
        }
        let start = RESIDENT_FRAMES + ((index - RESIDENT_FRAMES) / READ_WINDOW) * READ_WINDOW;
        let count = READ_WINDOW.min(self.len - start);
        if self.window_start == start && self.window.len() == count {
            return;
        }
        self.reads.push((start, count));
        self.window = read(start, count);
        self.window_start = start;
    }
}

/// The file cap stays 36,000. A length inside it is kept.
pub fn length_is_kept(frames: usize) -> bool {
    frames > 0 && frames <= MAX_FRAMES
}

#[derive(Clone, Debug)]
pub struct Capture {
    on: bool,
    role: Role,
    frames: Vec<InputFrame>,
}

impl Capture {
    pub fn idle() -> Self {
        Self {
            on: false,
            role: Role::Player,
            frames: Vec::new(),
        }
    }

    pub fn on(&self) -> bool {
        self.on
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn frames(&self) -> &[InputFrame] {
        &self.frames
    }
}

/// The pop-up role is who the next recording stores. Starting clears the buffer.
pub fn toggle_record(capture: &mut Capture, role: Role) -> bool {
    if capture.on {
        capture.on = false;
        false
    } else {
        capture.on = true;
        capture.role = role;
        capture.frames.clear();
        true
    }
}

pub fn push_frame(capture: &mut Capture, role: Role, frame: InputFrame) {
    if capture.on && capture.role == role {
        capture.frames.push(frame);
    }
}

pub enum RecordStop {
    Empty,
    Saved { id: String, start_save: String },
}

impl RecordStop {
    pub fn notice(&self) -> Option<&'static str> {
        match self {
            RecordStop::Empty => Some(EMPTY_RECORDING),
            RecordStop::Saved { .. } => None,
        }
    }

    pub fn cursor(&self) -> Option<&str> {
        match self {
            RecordStop::Saved { id, .. } => Some(id),
            RecordStop::Empty => None,
        }
    }
}

/// Stop. An empty buffer writes nothing. A recording writes `"<name> · start"`, stores that id, and
/// becomes most recently used. The active macro is left as it was.
pub fn finish_recording(
    root: &Path,
    capture: &Capture,
    player: SavedState,
    cpu: SavedState,
    at: u64,
) -> Result<RecordStop, Error> {
    if capture.frames.is_empty() {
        return Ok(RecordStop::Empty);
    }
    let macros = Library::new(root.join(MACROS_DIR));
    let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
    let macro_names = macros
        .list()
        .map_err(record)?
        .entries
        .into_iter()
        .map(|entry| entry.name)
        .collect::<Vec<_>>();
    let name = unused_label("Macro", &macro_names);
    let start_name = format!("{name} · start");
    let kind = player.fighter_kind;
    let start = NamedSave::new(start_name, player, cpu)?;
    let start_id = saves.save_new(&start)?;
    let mut recording = MacroRecording::new(
        &name,
        "13.0.5",
        StartState::neutral(kind, 0, 1.0),
        capture.frames.clone(),
    )
    .map_err(record)?;
    recording.set_start_save(Some(&start_id)).map_err(record)?;
    let id = macros.save_new(&recording).map_err(record)?;
    let mut index = RecordingIndex::load(root)?;
    index.note_use(Kind::Save, &start_id, Use::Recorded, at)?;
    index.note_use(Kind::Macro, &id, Use::Recorded, at.saturating_add(1))?;
    index.save(root)?;
    Ok(RecordStop::Saved {
        id,
        start_save: start_id,
    })
}

pub fn drive_play<'a>(machine: &mut Playback, role: Role, book: &'a Book) -> super::playback::Effect<'a> {
    machine.play_chord(role, book)
}

pub fn drive_step<'a>(
    machine: &mut Playback,
    role: Role,
    book: &'a Book,
) -> Option<super::playback::Played<'a>> {
    machine.step(role, book)
}

pub fn drive_mash(machine: &mut Playback, role: Role, interrupt: bool) -> bool {
    machine.mash(role, interrupt)
}

pub fn drive_load<'a>(
    machine: &Playback,
    book: &'a Book,
    settings: &crate::model::state::settings::StateSettings,
) -> Option<super::playback::DeathWarp<'a>> {
    machine.load(book, settings)
}

pub fn drive_auto_load<'a>(
    machine: &Playback,
    book: &'a Book,
    settings: &crate::model::state::settings::StateSettings,
) -> Option<super::playback::DeathWarp<'a>> {
    machine.auto_load_on_death(book, settings)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainAsk {
    pub step: usize,
    pub recorded: String,
    pub current: String,
    pub origin: FighterOrigin,
    pub switch_fighter: String,
}

/// Macro 1 only. A later step never asks and never names a fighter to switch to.
pub fn chain_mismatch(
    steps: &[(&str, &str)],
    current: &str,
    prompt_on: bool,
    switchable: bool,
) -> Option<ChainAsk> {
    let (_, fighter) = *steps.first()?;
    if fighter.is_empty() || fighter == current {
        return None;
    }
    if fighter_gate(prompt_on, switchable, false) != FighterGate::Prompt {
        return None;
    }
    Some(ChainAsk {
        step: 0,
        recorded: fighter.to_string(),
        current: current.to_string(),
        origin: FighterOrigin::Named,
        switch_fighter: fighter.to_string(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardOffer {
    pub header: &'static str,
    pub draft: String,
}

/// Online waiting rooms do not open the keyboard. The caller keeps the N name.
pub fn keyboard_opens(online: bool) -> bool {
    let mode = if online { 0x0701_0000 } else { 0 };
    crate::model::scene::combo_name_keyboard(true, mode)
}

pub fn offer_keyboard(header: &'static str, draft: &str, online: bool) -> Option<KeyboardOffer> {
    if !keyboard_opens(online) {
        return None;
    }
    Some(KeyboardOffer {
        header,
        draft: draft.to_string(),
    })
}

pub fn crop_macro(
    frames: &[InputFrame],
    start: u32,
    end: u32,
    snapshot: &FrameSnapshot,
    name: &str,
) -> Result<MacroRecording, Error> {
    let mut window = FrameWindow::new(frames.len() as u32).map_err(record)?;
    window.set_in(start);
    window.set_out(end);
    window
        .crop(frames, snapshot, name, "13.0.5")
        .map_err(record)
}

pub fn crop_macro_file(
    root: &Path,
    id: &str,
    start: u32,
    end: u32,
    snapshot: &FrameSnapshot,
    name: &str,
) -> Result<String, Error> {
    let library = Library::new(root.join(MACROS_DIR));
    let original = library.load(id).map_err(record)?;
    let cropped = crop_macro(&original.frames, start, end, snapshot, name)?;
    library.save_new(&cropped).map_err(record)
}

pub fn crop_combo(combo: &Combo, start: u32, end: u32, name: &str) -> Result<ComboRecord, Error> {
    let links: Vec<LinkRec> = combo.links().cloned().collect();
    let mut window = FrameWindow::new(links.len() as u32).map_err(record)?;
    window.set_in(start);
    window.set_out(end);
    let sliced = links[window.start() as usize..window.end() as usize].to_vec();
    let next = Combo::from_links(sliced).ok_or(Error::Invalid("bad crop"))?;
    Ok(ComboRecord::new(name.to_string(), next, None))
}

pub fn crop_combo_file(
    root: &Path,
    id: &str,
    start: u32,
    end: u32,
    name: &str,
) -> Result<String, Error> {
    let library = ComboLibrary::new(root.join("combos"));
    let original = library.load(id).map_err(record)?;
    let cropped = crop_combo(&original.combo, start, end, name)?;
    library.save_new(&cropped).map_err(record)
}

/// Replay export is a cropped recording with no start save.
pub fn replay_export(
    frames: &[InputFrame],
    start: u32,
    end: u32,
    snapshot: &FrameSnapshot,
    name: &str,
) -> Result<MacroRecording, Error> {
    crop_macro(frames, start, end, snapshot, name)
}

#[derive(Clone, Debug)]
pub struct MakeComboPass {
    pub role: Role,
    pub strip: bool,
    pub drill: Option<super::library::Drill>,
    pub warp: Option<String>,
    pub buttons: Vec<u32>,
}

/// One Player pass from the chain's start save. The drill id is not written.
pub fn make_combo(book: &Book) -> MakeComboPass {
    let drill = book.index.drill().cloned();
    let mut once = book.clone();
    once.index.set_loop(Role::Player, LoopCount::Finite(1));
    let mut machine = Playback::new();
    let effect = machine.play_chord(Role::Player, &once);
    let mut buttons = Vec::new();
    for _ in 0..64 {
        match machine.step(Role::Player, &once) {
            Some(played) => buttons.push(played.frame.buttons),
            None => break,
        }
    }
    MakeComboPass {
        role: Role::Player,
        strip: true,
        drill,
        warp: effect.warp.map(str::to_owned),
        buttons,
    }
}

pub fn start_save_notice(state: StartSaveState) -> Option<&'static str> {
    match state {
        StartSaveState::Deleted => {
            // restriction: macro-start-save-missing
            Some(MISSING_START_SAVE)
        }
        StartSaveState::None | StartSaveState::Linked => None,
    }
}

pub fn chord_notice(index: &mut RecordingIndex) -> Option<&'static str> {
    if index.chord_notice_seen() {
        return None;
    }
    index.mark_chord_notice_seen();
    Some(CHORD_NOTICE)
}

/// An empty chain or sequence adds no sentence.
pub fn notice_for_empty_chain_or_sequence() -> Option<&'static str> {
    None
}

pub fn duplicate_save(root: &Path, id: &str, at: u64) -> Result<String, Error> {
    let store = Store::<NamedSave>::new(root.join(SAVES_DIR));
    let source = store.load(id)?;
    let names = store
        .list()?
        .entries
        .into_iter()
        .map(|entry| entry.name)
        .collect::<Vec<_>>();
    let copy = NamedSave::new(unused_label("Save", &names), source.player, source.cpu)?;
    let copy_id = store.save_new(&copy)?;
    let mut index = RecordingIndex::load(root)?;
    index.note_use(Kind::Save, &copy_id, Use::Recorded, at)?;
    index.save(root)?;
    Ok(copy_id)
}

pub fn delete_save(root: &Path, id: &str, at: u64) -> Result<(), Error> {
    let store = Store::<NamedSave>::new(root.join(SAVES_DIR));
    store.delete(id)?;
    let mut index = RecordingIndex::load(root)?;
    if index.bound() == Some(id) {
        index.bind_save(None, at)?;
    }
    index.save(root)?;
    Ok(())
}

/// The bound file is overwritten. With nothing bound, the lowest free `Save N` is created and bound.
pub fn save_chord(
    root: &Path,
    player: SavedState,
    cpu: SavedState,
    at: u64,
) -> Result<String, Error> {
    let store = Store::<NamedSave>::new(root.join(SAVES_DIR));
    let mut index = RecordingIndex::load(root)?;
    if let Some(id) = index.bound().map(str::to_owned) {
        if let Ok(mut existing) = store.load(&id) {
            existing.player = player;
            existing.cpu = cpu;
            store.overwrite(&id, &existing)?;
            index.note_use(Kind::Save, &id, Use::Bound, at)?;
            index.save(root)?;
            return Ok(id);
        }
    }
    let names = store
        .list()?
        .entries
        .into_iter()
        .map(|entry| entry.name)
        .collect::<Vec<_>>();
    let save = NamedSave::new(unused_label("Save", &names), player, cpu)?;
    let id = store.save_new(&save)?;
    index.bind_save(Some(&id), at)?;
    index.save(root)?;
    Ok(id)
}

pub fn migrate_at_boot(
    root: &Path,
    slots: &[Option<LegacySlot>; 5],
    date: &str,
) -> Result<super::library::MigrationReceipt, Error> {
    migrate(root, slots, date)
}

/// Civil date of a unix second, `YYYY-MM-DD`. The worker passes the clock it has.
pub fn civil_date(unix_secs: u64) -> String {
    let z = (unix_secs / 86_400) as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn unused_label(prefix: &str, names: &[String]) -> String {
    let mut n = 1u32;
    loop {
        let label = format!("{prefix} {n}");
        if !names.iter().any(|name| name == &label) {
            return label;
        }
        n += 1;
    }
}

fn record(err: impl ToString) -> Error {
    Error::Record(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::library::{start_save_state, Drill, DrillKind};
    use super::*;
    use crate::model::combo::{Aim, Event, LinkRec};
    use crate::model::frame_strip::{Button, Move, Verdict};
    use crate::model::menu::input::Button as MenuButton;
    use crate::model::menu::table::{COMBOS_SEQ, SAVES_MACROS};
    use crate::model::recording_menu::{
        Catalog, ComboFile, Effect, Face, MacroFile, RecordingScreens, SaveFile, SequenceFile,
    };
    use crate::model::state::settings::StateSettings;

    fn frame(buttons: u32) -> InputFrame {
        InputFrame {
            buttons,
            ..InputFrame::default()
        }
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("tr-rr7-{tag}-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn state(kind: i32) -> SavedState {
        let mut state = SavedState::never_saved();
        state.fighter_kind = kind;
        state
    }

    fn recording(name: &str, buttons: &[u32], save: Option<&str>) -> MacroRecording {
        let mut recording = MacroRecording::new(
            name,
            "13.0.5",
            StartState::neutral(8, 0, 1.0),
            buttons.iter().copied().map(frame).collect(),
        )
        .unwrap();
        if let Some(id) = save {
            recording.set_start_save(Some(id)).unwrap();
        }
        recording
    }

    fn link(from: Move, to: Move) -> LinkRec {
        LinkRec::presses(
            from,
            to,
            Verdict::True(None),
            None,
            &[Event::press(Button::Jump, Aim::Neutral, 2)],
        )
    }

    #[test]
    fn record_follows_the_popup_role() {
        let mut capture = Capture::idle();
        assert!(toggle_record(&mut capture, Role::Player));
        push_frame(&mut capture, Role::Player, frame(1));
        push_frame(&mut capture, Role::Cpu, frame(9));
        assert_eq!(
            capture
                .frames()
                .iter()
                .map(|frame| frame.buttons)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert!(!toggle_record(&mut capture, Role::Player));
        assert!(toggle_record(&mut capture, Role::Cpu));
        push_frame(&mut capture, Role::Player, frame(1));
        push_frame(&mut capture, Role::Cpu, frame(9));
        assert_eq!(capture.role(), Role::Cpu);
        assert_eq!(
            capture
                .frames()
                .iter()
                .map(|frame| frame.buttons)
                .collect::<Vec<_>>(),
            vec![9]
        );
    }

    #[test]
    fn record_stores_the_start_save_id() {
        let root = scratch("record");
        let mut index = RecordingIndex::fresh();
        index.set_active(Role::Player, Some("old"), 1).unwrap();
        index.save(&root).unwrap();
        let mut capture = Capture::idle();
        toggle_record(&mut capture, Role::Player);
        push_frame(&mut capture, Role::Player, frame(4));
        let stop = finish_recording(&root, &capture, state(13), state(7), 5).unwrap();
        let RecordStop::Saved {
            ref id,
            ref start_save,
        } = stop
        else {
            panic!("empty")
        };
        let id = id.clone();
        let start_save = start_save.clone();
        assert_eq!(stop.cursor(), Some(id.as_str()));
        let macros = Library::new(root.join(MACROS_DIR));
        let saved = macros.load(&id).unwrap();
        assert_eq!(saved.name, "Macro 1");
        assert_eq!(saved.start_save.as_deref(), Some(start_save.as_str()));
        let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
        assert_eq!(saves.load(&start_save).unwrap().name, "Macro 1 · start");
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.active(Role::Player), Some("old"));
        assert_eq!(
            index.hot(Kind::Macro).first().map(String::as_str),
            Some(id.as_str())
        );
    }

    #[test]
    fn a_long_macro_plays_from_a_600_frame_window() {
        assert!(length_is_kept(MAX_FRAMES));
        assert!(!length_is_kept(MAX_FRAMES + 1));
        let frames: Vec<InputFrame> = (0..4_300).map(|index| frame(index as u32)).collect();
        let mut tape = Tape::load(&frames);
        assert_eq!(tape.resident_len(), RESIDENT_FRAMES);
        assert_eq!(tape.hook_frame(3_599).unwrap().buttons, 3_599);
        assert!(tape.reads().is_empty());
        assert!(tape.hook_frame(3_600).is_none());
        tape.fill_window(3_600, |start, count| frames[start..start + count].to_vec());
        assert_eq!(tape.reads(), &[(3_600, 600)]);
        assert_eq!(tape.window_len(), 600);
        assert_eq!(tape.hook_frame(3_600).unwrap().buttons, 3_600);
        assert_eq!(tape.hook_frame(4_199).unwrap().buttons, 4_199);
        tape.fill_window(4_100, |start, count| frames[start..start + count].to_vec());
        assert_eq!(tape.reads().len(), 1);
        tape.fill_window(4_200, |start, count| frames[start..start + count].to_vec());
        assert_eq!(tape.reads(), &[(3_600, 600), (4_200, 100)]);
        assert_eq!(tape.hook_frame(4_200).unwrap().buttons, 4_200);
        assert_eq!(tape.resident_len(), RESIDENT_FRAMES);
    }

    #[test]
    fn a_macro_with_no_start_save_does_not_warp() {
        let mut index = RecordingIndex::fresh();
        index.set_active(Role::Player, Some("bare"), 1).unwrap();
        let mut book = Book::new(index);
        book.add_recording("bare", recording("Bare", &[3, 4], None));
        let mut machine = Playback::new();
        let effect = drive_play(&mut machine, Role::Player, &book);
        assert!(effect.started);
        assert!(effect.warp.is_none());
        assert_eq!(machine.warps(Role::Player), 0);
        assert_eq!(
            drive_step(&mut machine, Role::Player, &book)
                .unwrap()
                .frame
                .buttons,
            3
        );
        assert!(start_save_notice(start_save_state(None, true, false)).is_none());
    }

    #[test]
    fn the_play_chords_drive_the_role_machines() {
        let mut index = RecordingIndex::fresh();
        index.set_active(Role::Player, Some("p"), 1).unwrap();
        index.set_active(Role::Cpu, Some("c"), 2).unwrap();
        index.set_loop(Role::Player, LoopCount::Finite(2));
        index.bind_save(Some("slot"), 3).unwrap();
        let mut book = Book::new(index);
        book.add_recording("p", recording("P", &[1], Some("start-p")));
        book.add_recording("c", recording("C", &[2], None));
        book.add_save("start-p");
        book.add_save("slot");
        let mut machine = Playback::new();
        let player = drive_play(&mut machine, Role::Player, &book);
        assert_eq!(player.warp, Some("start-p"));
        assert_eq!(machine.warps(Role::Player), 1);
        let cpu = drive_play(&mut machine, Role::Cpu, &book);
        assert!(cpu.warp.is_none());
        assert!(machine.playing(Role::Player) && machine.playing(Role::Cpu));
        assert_eq!(
            drive_step(&mut machine, Role::Player, &book)
                .unwrap()
                .frame
                .buttons,
            1
        );
        assert_eq!(
            drive_step(&mut machine, Role::Player, &book)
                .unwrap()
                .frame
                .buttons,
            1
        );
        assert_eq!(machine.warps(Role::Player), 1);
        assert!(drive_mash(&mut machine, Role::Player, true));
        assert!(!machine.playing(Role::Player));
        assert!(machine.playing(Role::Cpu));
        let settings = StateSettings {
            autoload: true,
            ..StateSettings::default()
        };
        let loaded = drive_load(&machine, &book, &settings).unwrap();
        assert_eq!(loaded.save_id, "slot");
        assert!(machine.playing(Role::Cpu));
        let death = drive_auto_load(&machine, &book, &settings).unwrap();
        assert_eq!(death.save_id, "slot");
        assert!(machine.playing(Role::Cpu));
    }

    #[test]
    fn a_chain_asks_fighter_mismatch_for_macro_1_only() {
        let ask =
            chain_mismatch(&[("m1", "Pikachu"), ("m2", "Mario")], "Mario", true, true).unwrap();
        assert_eq!(ask.step, 0);
        assert_eq!(ask.switch_fighter, "Pikachu");
        assert_eq!(ask.recorded, "Pikachu");
        assert_eq!(ask.origin, FighterOrigin::Named);
        assert!(
            chain_mismatch(&[("m1", "Mario"), ("m2", "Pikachu")], "Mario", true, true).is_none()
        );
        assert!(chain_mismatch(&[("m1", "Pikachu")], "Mario", false, true).is_none());
    }

    #[test]
    fn rename_uses_the_switch_keyboard_headers() {
        let mut screens = RecordingScreens::new();
        screens.set_catalog(header_catalog());
        assert_header(&mut screens, SAVES_MACROS, 0, "Slot", "Name save state");
        assert_header(&mut screens, SAVES_MACROS, 1, "Nair", "Name macro");
        assert_header(&mut screens, SAVES_MACROS, 1, "Corner", "Name chain");
        assert_header(&mut screens, COMBOS_SEQ, 1, "One", "Name combo");
        assert_header(&mut screens, COMBOS_SEQ, 0, "Seq", "Name sequence");
        screens.submit_name("");
        assert_eq!(screens.catalog().sequences[0].name, "Seq");
        assert!(!keyboard_opens(true));
        assert!(offer_keyboard("Name macro", "Macro 1", true).is_none());
        assert_eq!(
            offer_keyboard("Name macro", "Macro 1", false)
                .unwrap()
                .header,
            "Name macro"
        );
        close_overlay(&mut screens);
        open_row(&mut screens, SAVES_MACROS, 1, "+ New chain");
        screens.handle_page(MenuButton::B, true, SAVES_MACROS);
        screens.submit_name("");
        assert!(screens.has_chain_named("Chain 1"));
    }

    #[test]
    fn crop_writes_a_new_file_and_leaves_the_original() {
        let root = scratch("crop");
        let macros = Library::new(root.join(MACROS_DIR));
        let source = recording("Nair", &[1, 2, 3, 4], Some("keep"));
        let id = macros.save_new(&source).unwrap();
        let before = std::fs::read(macros.dir().join(format!("{id}.json"))).unwrap();
        let snapshot = FrameSnapshot {
            frame: 1,
            state: StartState::neutral(8, 0, 1.0),
        };
        let copy = crop_macro_file(&root, &id, 1, 3, &snapshot, "Nair crop").unwrap();
        let after = std::fs::read(macros.dir().join(format!("{id}.json"))).unwrap();
        assert_eq!(before, after);
        assert_ne!(copy, id);
        let cropped = macros.load(&copy).unwrap();
        assert_eq!(cropped.frames.len(), 2);
        assert_eq!(cropped.frames[0].buttons, 2);
        assert!(cropped.start_save.is_none());
        assert_eq!(macros.load(&id).unwrap().frames.len(), 4);

        let combos = ComboLibrary::new(root.join("combos"));
        let combo = Combo::from_links([
            link(Move::Aerial, Move::SideTilt),
            link(Move::SideTilt, Move::NeutralSpecial),
        ])
        .unwrap();
        let combo_id = combos
            .save_new(&ComboRecord::new("Loop".into(), combo, None))
            .unwrap();
        let combo_before = std::fs::read(combos.dir().join(format!("{combo_id}.json"))).unwrap();
        let cropped_id = crop_combo_file(&root, &combo_id, 0, 1, "Loop crop").unwrap();
        let combo_after = std::fs::read(combos.dir().join(format!("{combo_id}.json"))).unwrap();
        assert_eq!(combo_before, combo_after);
        assert_eq!(combos.load(&cropped_id).unwrap().combo.links().count(), 1);
        assert_eq!(combos.load(&combo_id).unwrap().combo.links().count(), 2);
    }

    #[test]
    fn replay_export_has_no_start_save() {
        let frames = vec![frame(1), frame(2), frame(3)];
        let snapshot = FrameSnapshot {
            frame: 0,
            state: StartState::neutral(8, 0, 1.0),
        };
        let exported = replay_export(&frames, 0, 2, &snapshot, "Clip").unwrap();
        assert!(exported.start_save.is_none());
        assert_eq!(exported.frames.len(), 2);
    }

    #[test]
    fn make_combo_on_a_chain_plays_one_pass_on_player_1() {
        let mut index = RecordingIndex::fresh();
        index.set_active(Role::Player, Some("pressure"), 1).unwrap();
        index.set_loop(Role::Player, LoopCount::Infinite);
        index
            .set_drill(
                Some(Drill {
                    kind: DrillKind::Combo,
                    id: "drill-1".into(),
                }),
                2,
            )
            .unwrap();
        let mut book = Book::new(index);
        book.add_recording("nair", recording("Nair", &[5], Some("nair-start")));
        book.add_recording("dash", recording("Dash", &[6], Some("dash-start")));
        book.add_save("nair-start");
        book.add_save("dash-start");
        let mut chain =
            super::super::library::Chain::new("Corner", vec!["nair".into(), "dash".into()])
                .unwrap();
        chain.set_start_save_on(true);
        book.add_chain("pressure", chain);
        let pass = make_combo(&book);
        assert_eq!(pass.role, Role::Player);
        assert!(pass.strip);
        assert_eq!(pass.warp.as_deref(), Some("nair-start"));
        assert_eq!(pass.buttons, vec![5, 6]);
        assert_eq!(pass.drill.unwrap().id, "drill-1");
        assert_eq!(book.index.drill().unwrap().id, "drill-1");
        assert_eq!(book.index.loop_count(Role::Player), LoopCount::Infinite);
    }

    #[test]
    fn the_missing_start_save_sentence_is_only_for_a_deleted_link() {
        assert_eq!(
            start_save_notice(start_save_state(Some("gone"), true, false)),
            Some(MISSING_START_SAVE)
        );
        assert!(start_save_notice(start_save_state(None, true, false)).is_none());
        assert!(start_save_notice(start_save_state(Some("kept"), false, true)).is_none());
        let source = include_str!("rr_runtime.rs");
        let restrictions = include_str!("../../docs/RESTRICTIONS.md");
        assert!(source.contains("restriction: macro-start-save-missing"));
        assert!(restrictions.contains("| `macro-start-save-missing` |"));
        assert!(
            restrictions.contains("No start save. This macro plays from where the fighters are.")
        );
        assert!(restrictions.contains("the connected start file was deleted"));
    }

    #[test]
    fn the_chord_notice_shows_once() {
        let mut index = RecordingIndex::fresh();
        assert_eq!(chord_notice(&mut index), Some(CHORD_NOTICE));
        assert!(index.chord_notice_seen());
        assert!(chord_notice(&mut index).is_none());
    }

    #[test]
    fn the_empty_recording_notice_stays() {
        let root = scratch("empty");
        let capture = Capture::idle();
        let stop = finish_recording(&root, &capture, state(1), state(2), 1).unwrap();
        assert_eq!(stop.notice(), Some("Recording was empty"));
        assert!(stop.cursor().is_none());
        assert!(!root.join(MACROS_DIR).exists());
    }

    #[test]
    fn silent_when_a_chain_or_a_sequence_is_empty() {
        assert!(notice_for_empty_chain_or_sequence().is_none());
    }

    #[test]
    fn deleting_the_bound_save_clears_the_binding() {
        let root = scratch("unbind");
        let id = save_chord(&root, state(3), state(4), 1).unwrap();
        assert_eq!(
            RecordingIndex::load(&root).unwrap().bound(),
            Some(id.as_str())
        );
        delete_save(&root, &id, 2).unwrap();
        assert!(RecordingIndex::load(&root).unwrap().bound().is_none());
        let next = save_chord(&root, state(8), state(9), 3).unwrap();
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.bound(), Some(next.as_str()));
        let saved = Store::<NamedSave>::new(root.join(SAVES_DIR))
            .load(&next)
            .unwrap();
        assert_eq!(saved.name, "Save 1");
        assert_eq!(saved.player.fighter_kind, 8);
    }

    #[test]
    fn duplicate_copies_the_save_and_leaves_it_bound() {
        let root = scratch("dup");
        let id = save_chord(&root, state(3), state(4), 1).unwrap();
        let copy = duplicate_save(&root, &id, 2).unwrap();
        assert_ne!(copy, id);
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.bound(), Some(id.as_str()));
        assert_eq!(
            index.hot(Kind::Save).first().map(String::as_str),
            Some(copy.as_str())
        );
        let store = Store::<NamedSave>::new(root.join(SAVES_DIR));
        let original = store.load(&id).unwrap();
        let copied = store.load(&copy).unwrap();
        assert_eq!(original.player.fighter_kind, copied.player.fighter_kind);
        assert_eq!(copied.name, "Save 2");
        assert_eq!(original.name, "Save 1");
    }

    #[test]
    fn migration_runs_once_at_boot() {
        assert_eq!(civil_date(0), "1970-01-01");
        let root = scratch("boot");
        let date = "2026-11-06";
        let mut slots: [Option<LegacySlot>; 5] = [None, None, None, None, None];
        slots[0] = Some(LegacySlot {
            frames: vec![frame(6)],
            start: StartState::neutral(1, 0, 1.0),
        });
        let receipt = migrate_at_boot(&root, &slots, date).unwrap();
        assert_eq!(receipt.recordings.len(), 1);
        let saved = Library::new(root.join(MACROS_DIR))
            .load(&receipt.recordings[0])
            .unwrap();
        assert_eq!(saved.name, format!("Slot 1 ({date})"));
        let before = snapshot(&root);
        slots[1] = Some(LegacySlot {
            frames: vec![frame(7)],
            start: StartState::neutral(2, 0, 1.0),
        });
        let again = migrate_at_boot(&root, &slots, date).unwrap();
        assert_eq!(again.recordings, receipt.recordings);
        assert_eq!(snapshot(&root), before);
    }

    fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, Vec<u8>)>) {
            let Ok(read) = std::fs::read_dir(dir) else {
                return;
            };
            let mut paths: Vec<_> = read.map(|entry| entry.unwrap().path()).collect();
            paths.sort();
            for path in paths {
                if path.is_dir() {
                    walk(&path, root, out);
                } else {
                    let rel = path
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    out.push((rel, std::fs::read(&path).unwrap()));
                }
            }
        }
        walk(root, root, &mut out);
        out.sort();
        out
    }

    fn header_catalog() -> Catalog {
        Catalog {
            saves: vec![SaveFile {
                id: "slot".into(),
                name: "Slot".into(),
                fighters: String::new(),
            }],
            macros: vec![
                MacroFile {
                    id: "nair".into(),
                    name: "Nair".into(),
                    chain: false,
                    steps: Vec::new(),
                    start_save: None,
                    start_save_on: true,
                    recorded_on: String::new(),
                },
                MacroFile {
                    id: "pressure".into(),
                    name: "Corner".into(),
                    chain: true,
                    steps: vec!["nair".into()],
                    start_save: None,
                    start_save_on: true,
                    recorded_on: String::new(),
                },
            ],
            combos: vec![ComboFile {
                id: "c1".into(),
                name: "One".into(),
                made_from: None,
                links: Vec::new(),
            }],
            sequences: vec![SequenceFile {
                id: "seq".into(),
                name: "Seq".into(),
                combos: vec!["c1".into()],
                streak: 0,
                best: 0,
            }],
            ..Catalog::default()
        }
    }

    fn assert_header(
        screens: &mut RecordingScreens,
        group: u8,
        section: u8,
        name: &str,
        header: &str,
    ) {
        close_overlay(screens);
        open_row(screens, group, section, name);
        let rename = screens
            .file_rows()
            .iter()
            .position(|row| row.title == "Rename")
            .unwrap();
        screens.set_file_cursor(rename);
        screens.handle_page(MenuButton::A, true, group);
        match screens.last_effect() {
            Effect::Keyboard { header: got, .. } => assert_eq!(*got, header),
            other => panic!("{other:?}"),
        }
    }

    fn open_row(screens: &mut RecordingScreens, group: u8, section: u8, name: &str) {
        let view = screens.section(Face::Page, group, section);
        let index = view.rows.iter().position(|row| row.name == name).unwrap();
        screens.set_cursor(Face::Page, group, section, index);
        screens.handle_page(MenuButton::A, true, group);
    }

    fn close_overlay(screens: &mut RecordingScreens) {
        for _ in 0..4 {
            if screens.picker_open() || screens.file_open() {
                screens.handle_page(MenuButton::B, true, SAVES_MACROS);
            }
        }
    }
}
