//! Host decisions for the recording runtime. The hook only reads a filled window.
//! Card IO and allocation stay on the worker.

use std::path::Path;

use super::library::{
    migrate, Chain, Error, Kind, LegacySlot, LoopCount, NamedSave, RecordingIndex, Role, Sequence,
    StartSaveState, Use, MACROS_DIR, SAVES_DIR, SEQUENCES_DIR,
};
#[cfg(test)]
use super::macro_ui::macro_panel::{fighter_gate, FighterGate, FighterOrigin};
use super::macros::{FrameSnapshot, FrameWindow, InputFrame, Library, MacroRecording, StartState};
#[cfg(test)]
use super::macros::MAX_FRAMES;
use super::playback::{Book, Playback};
use crate::model::combo::{Combo, LinkRec};
use crate::model::combo_record::{clean_name, default_name, ComboLibrary, ComboRecord};
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
/// The sentence Save last combo already uses when the strip has no link.
pub const EMPTY_COMBO: &str = "No combo yet: land a hit and a follow-up first.";

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

    #[cfg(test)]
    pub fn resident_len(&self) -> usize {
        self.resident.len()
    }

    #[cfg(test)]
    pub fn window_len(&self) -> usize {
        self.window.len()
    }

    #[cfg(test)]
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
#[cfg(test)]
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

    pub fn role(&self) -> Role {
        self.role
    }

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn cursor(&self) -> Option<&str> {
        match self {
            RecordStop::Saved { id, .. } => Some(id),
            RecordStop::Empty => None,
        }
    }
}

/// Stop. With `trim` (Recording › Options › Crop trailing neutral, rc.12 TR7) the idle frames at the end go first. An
/// empty buffer writes nothing. A recording writes `"<name> · start"` from `player` and `cpu` (where Record
/// started), stores that id, becomes most recently used, and becomes that role's active macro. A CPU-role recording names
/// the CPU's fighter, so Play on CPU compares the fighter who was being recorded. The macro's start state keeps the
/// recorded role's facing and its `speed` when Record started (x, y), which Play applies with "Keep recorded speed"
/// (PLAY-MOMENTUM).
pub fn finish_recording(
    root: &Path,
    capture: &Capture,
    player: SavedState,
    cpu: SavedState,
    speed: (f32, f32),
    at: u64,
    trim: bool,
) -> Result<RecordStop, Error> {
    let mut frames = capture.frames.clone();
    if trim {
        super::macro_ui::macro_panel::crop_neutral_tail(&mut frames);
    }
    if frames.is_empty() {
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
    let recorded = match capture.role() {
        Role::Cpu => &cpu,
        Role::Player => &player,
    };
    let start_state = StartState {
        situation: recorded.situation_kind,
        speed_x: speed.0,
        speed_y: speed.1,
        ..StartState::neutral(recorded.fighter_kind, 0, recorded.lr)
    };
    let start = NamedSave::new(start_name, player, cpu)?;
    let start_id = saves.save_new(&start)?;
    let mut recording = MacroRecording::new(&name, "13.0.5", start_state, frames).map_err(record)?;
    recording.set_start_save(Some(&start_id)).map_err(record)?;
    let id = macros.save_new(&recording).map_err(record)?;
    let mut index = RecordingIndex::load(root)?;
    index.note_use(Kind::Save, &start_id, Use::Recorded, at)?;
    index.note_use(Kind::Macro, &id, Use::Recorded, at.saturating_add(1))?;
    index.set_active(capture.role(), Some(&id), at.saturating_add(2))?;
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

#[cfg(test)]
pub fn drive_load<'a>(
    machine: &Playback,
    book: &'a Book,
    settings: &crate::model::state::settings::StateSettings,
) -> Option<super::playback::DeathWarp<'a>> {
    machine.load(book, settings)
}

#[cfg(test)]
pub fn drive_auto_load<'a>(
    machine: &Playback,
    book: &'a Book,
    settings: &crate::model::state::settings::StateSettings,
) -> Option<super::playback::DeathWarp<'a>> {
    machine.auto_load_on_death(book, settings)
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainAsk {
    pub step: usize,
    pub recorded: String,
    pub current: String,
    pub origin: FighterOrigin,
    pub switch_fighter: String,
}

/// Macro 1 only. A later step never asks and never names a fighter to switch to.
#[cfg(test)]
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
#[cfg(test)]
pub fn replay_export(
    frames: &[InputFrame],
    start: u32,
    end: u32,
    snapshot: &FrameSnapshot,
    name: &str,
) -> Result<MacroRecording, Error> {
    crop_macro(frames, start, end, snapshot, name)
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub struct MakeComboPass {
    pub role: Role,
    pub strip: bool,
    pub drill: Option<super::library::Drill>,
    pub warp: Option<String>,
    pub buttons: Vec<u32>,
}

/// The strip's combo, named Combo N, marked made from `source`. The drill is not written.
pub fn save_made_combo(root: &Path, combo: &Combo, source: &str, at: u64) -> Result<String, Error> {
    let library = ComboLibrary::new(root.join("combos"));
    let entries = library.list().map_err(record)?.entries;
    let made = ComboRecord::new(default_name(&entries), *combo, None).with_made_from(source);
    let id = library.save_new(&made).map_err(record)?;
    let mut index = RecordingIndex::load(root)?;
    index.note_use(Kind::Combo, &id, Use::Recorded, at)?;
    index.save(root)?;
    Ok(id)
}

/// *+ Save last combo…* (operator 2026-11-08; rc.4 COMBO-HELPER): the strip's latest exchange, named `name` (empty:
/// Combo N), recorded on Player 1's fighter, hot, and the drill (RECORDING.md: "makes it the drill").
pub fn save_last_combo(root: &Path, combo: &Combo, name: &str, recorded_on: Option<String>, at: u64) -> Result<String, Error> {
    let library = ComboLibrary::new(root.join("combos"));
    let name = match clean_name(name) {
        Some(name) => name,
        None => default_name(&library.list().map_err(record)?.entries),
    };
    let id = library.save_new(&ComboRecord::new(name, *combo, recorded_on)).map_err(record)?;
    let mut index = RecordingIndex::load(root)?;
    index.note_use(Kind::Combo, &id, Use::Recorded, at)?;
    index.set_drill(Some(super::library::Drill { kind: super::library::DrillKind::Combo, id: id.clone() }), at)?;
    index.save(root)?;
    Ok(id)
}

/// One Player pass from the chain's start save. The drill id is not written.
#[cfg(test)]
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

pub fn unused_name(prefix: &str, names: &[String]) -> String {
    unused_label(prefix, names)
}

/// The card, as the pop-up lists it. Hot lists and the selection come from the index.
pub struct CatalogLoad {
    pub catalog: crate::model::recording_menu::Catalog,
    pub bound: Option<String>,
    pub active: [Option<String>; 2],
    pub loops: [crate::model::recording_menu::LoopMark; 2],
    pub drill: Option<String>,
    /// rc.12 TR1: the macro of each Playback Slot 1-5.
    pub slots: [Option<String>; 5],
}

pub fn load_catalog(root: &Path) -> Result<CatalogLoad, Error> {
    use crate::model::recording_menu::{Catalog, ComboFile, LoopMark, MacroFile, SaveFile, SequenceFile};
    let index = RecordingIndex::load(root)?;
    let mut catalog = Catalog::default();
    let macros = Library::new(root.join(MACROS_DIR));
    if let Ok(list) = macros.list() {
        for entry in list.entries {
            let Ok(file) = macros.load(&entry.id) else { continue };
            catalog.macros.push(MacroFile {
                id: entry.id,
                name: file.name,
                chain: false,
                steps: Vec::new(),
                start_save: file.start_save,
                start_save_on: file.start_save_on,
                recorded_on: fighter_name(file.start.fighter_kind),
                span: file.frames.len() as u32,
            });
        }
    }
    let chains = Store::<Chain>::new(root.join(MACROS_DIR));
    if let Ok(list) = chains.list() {
        for entry in list.entries {
            let Ok(file) = chains.load(&entry.id) else { continue };
            catalog.macros.push(MacroFile {
                id: entry.id,
                name: file.name,
                chain: true,
                steps: file.recordings,
                start_save: None,
                start_save_on: file.start_save_on,
                recorded_on: String::new(),
                span: 0,
            });
        }
    }
    let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
    if let Ok(list) = saves.list() {
        for entry in list.entries {
            let fighters = saves.load(&entry.id).ok().map(|save| {
                let mut parts = Vec::new();
                if save.player.is_saved() {
                    parts.push(fighter_name(save.player.fighter_kind));
                }
                if save.cpu.is_saved() {
                    parts.push(fighter_name(save.cpu.fighter_kind));
                }
                parts.retain(|part| !part.is_empty());
                parts.join(" · ")
            });
            catalog.saves.push(SaveFile { id: entry.id, name: entry.name, fighters: fighters.unwrap_or_default() });
        }
    }
    let combos = ComboLibrary::new(root.join("combos"));
    if let Ok(list) = combos.list() {
        for entry in list.entries {
            let record = combos.load(&entry.id).ok();
            let links = record
                .as_ref()
                .map(|file| crate::model::recording_menu::combo_link_lines(&file.combo))
                .unwrap_or_default();
            catalog.combos.push(ComboFile {
                id: entry.id,
                name: entry.name,
                recorded_on: record.as_ref().and_then(|file| file.recorded_on.clone()).unwrap_or_default(),
                made_from: record.and_then(|file| file.made_from),
                links,
                span: entry.links as u32,
            });
        }
    }
    let sequences = Store::<Sequence>::new(root.join(SEQUENCES_DIR));
    if let Ok(list) = sequences.list() {
        for entry in list.entries {
            let Ok(file) = sequences.load(&entry.id) else { continue };
            catalog.sequences.push(SequenceFile {
                id: entry.id,
                name: file.name,
                combos: file.combos,
                streak: file.streak,
                best: file.best,
            });
        }
    }
    catalog.hot_macros = index.hot(Kind::Macro).to_vec();
    catalog.hot_saves = index.hot(Kind::Save).to_vec();
    catalog.hot_combos = index.hot(Kind::Combo).to_vec();
    catalog.hot_sequences = index.hot(Kind::Sequence).to_vec();
    let mark = |count: LoopCount| match count {
        LoopCount::Infinite => LoopMark::Infinite,
        LoopCount::Finite(n) => LoopMark::Finite(n),
    };
    Ok(CatalogLoad {
        catalog,
        bound: index.bound().map(str::to_owned),
        active: [
            index.active(Role::Player).map(str::to_owned),
            index.active(Role::Cpu).map(str::to_owned),
        ],
        loops: [mark(index.loop_count(Role::Player)), mark(index.loop_count(Role::Cpu))],
        drill: index.drill().map(|drill| drill.id.clone()),
        slots: index.slots().clone(),
    })
}

// The fighter table is the switch (and test) roster. A host library build has no names.
#[cfg(any(test, target_os = "switch"))]
fn fighter_name(kind: i32) -> String {
    if kind < 0 {
        return String::new();
    }
    crate::engine::swap::trio::TrioFighter::from_kind(kind as u32)
        .map(|fighter| fighter.name().to_owned())
        .unwrap_or_default()
}

#[cfg(not(any(test, target_os = "switch")))]
fn fighter_name(_kind: i32) -> String {
    String::new()
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
    lib_unit_tests! {
    use super::super::library::{Drill, DrillKind};
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

    fn the_chord_notice_shows_once() {
        let mut index = RecordingIndex::fresh();
        assert_eq!(chord_notice(&mut index), Some(CHORD_NOTICE));
        assert!(index.chord_notice_seen());
        assert!(chord_notice(&mut index).is_none());
    }

    fn the_empty_recording_notice_stays() {
        let root = scratch("empty");
        let capture = Capture::idle();
        let stop = finish_recording(&root, &capture, state(1), state(2), (0.0, 0.0), 1, true).unwrap();
        assert_eq!(stop.notice(), Some("Recording was empty"));
        assert!(stop.cursor().is_none());
        assert!(!root.join(MACROS_DIR).exists());
    }

    /// rc.12 TR7 (operator 2026-11-13, B "make Record trim"): with Recording › Options › Crop trailing neutral On, Record
    /// drops the idle frames at the end (no button but a flick-jump bit, sticks inside the dead zone); Off keeps every
    /// frame; a take that is all idle is empty.
    fn record_trims_the_neutral_tail() {
        let root = scratch("trim");
        let take = |frames: &[InputFrame]| {
            let mut capture = Capture::idle();
            toggle_record(&mut capture, Role::Player);
            frames.iter().for_each(|frame| push_frame(&mut capture, Role::Player, *frame));
            capture
        };
        let idle = [frame(0x8000), InputFrame { lstick_x: 20, rstick_y: -23, ..frame(0) }, frame(0)];
        let mut frames = vec![frame(5), InputFrame { lstick_x: 90, ..frame(0) }];
        frames.extend_from_slice(&idle);
        let saved = |stop: RecordStop| match stop {
            RecordStop::Saved { id, .. } => Library::new(root.join(MACROS_DIR)).load(&id).unwrap().frames.len(),
            RecordStop::Empty => 0,
        };
        assert_eq!(saved(finish_recording(&root, &take(&frames), state(1), state(2), (0.0, 0.0), 1, true).unwrap()), 2);
        assert_eq!(saved(finish_recording(&root, &take(&frames), state(1), state(2), (0.0, 0.0), 5, false).unwrap()), 5);
        let stop = finish_recording(&root, &take(&idle), state(1), state(2), (0.0, 0.0), 9, true).unwrap();
        assert_eq!(stop.notice(), Some("Recording was empty"));
    }

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
                    span: 0,
                },
                MacroFile {
                    id: "pressure".into(),
                    name: "Corner".into(),
                    chain: true,
                    steps: vec!["nair".into()],
                    start_save: None,
                    start_save_on: true,
                    recorded_on: String::new(),
                    span: 0,
                },
            ],
            combos: vec![ComboFile {
                id: "c1".into(),
                name: "One".into(),
                recorded_on: String::new(),
                made_from: None,
                links: Vec::new(),
                span: 0,
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

    /// Eden rc5-opus-final-trace2 (2026-11-08): Record on CPU made `macro-1` the CPU's active file, then a menu open and
    /// close wrote `active.cpu = null` back. The pop-up had kept its model alive with the selection it took before the
    /// recording. The worker's steps (`worker::refresh_recording`, `post_recording_effect`) on a scratch card.
    fn a_menu_close_after_record_on_cpu_keeps_the_new_active_file() {
        use crate::model::recording_menu::{CatalogWatch, RecordingScreens};
        use super::super::rr_host::publish_selection;

        let root = scratch("menu-adopt");
        RecordingIndex::load(&root).unwrap().save(&root).unwrap();
        let refresh = |screens: &mut RecordingScreens, watch: CatalogWatch, generation: u64| {
            let (watch, due) = watch.take_load(generation);
            if due {
                let loaded = load_catalog(&root).unwrap();
                screens.set_catalog(loaded.catalog);
                screens.adopt(loaded.bound, loaded.active, loaded.loops, loaded.drill);
            }
            watch
        };
        // The pop-up: its model loads generation 0, and its press publishes the selection.
        let mut screens = RecordingScreens::new();
        let watch = refresh(&mut screens, CatalogWatch::default(), 0);
        publish_selection(&root, &screens.selection(), None);
        // Record on CPU stops: the new file is the CPU's active one, and the generation moves.
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Cpu, Some("macro-1"), 10).unwrap();
        index.save(&root).unwrap();
        // The menu opens on the same model and closes: the worker refreshes, then publishes.
        refresh(&mut screens, watch, 1);
        publish_selection(&root, &screens.selection(), None);
        let after = RecordingIndex::load(&root).unwrap();
        assert_eq!(after.active(Role::Cpu), Some("macro-1"), "the menu published a selection older than the index");
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("record_follows_the_popup_role", record_follows_the_popup_role as fn()),
            ("a_long_macro_plays_from_a_600_frame_window", a_long_macro_plays_from_a_600_frame_window as fn()),
            ("the_play_chords_drive_the_role_machines", the_play_chords_drive_the_role_machines as fn()),
            ("a_chain_asks_fighter_mismatch_for_macro_1_only", a_chain_asks_fighter_mismatch_for_macro_1_only as fn()),
            ("rename_uses_the_switch_keyboard_headers", rename_uses_the_switch_keyboard_headers as fn()),
            ("crop_writes_a_new_file_and_leaves_the_original", crop_writes_a_new_file_and_leaves_the_original as fn()),
            ("replay_export_has_no_start_save", replay_export_has_no_start_save as fn()),
            ("make_combo_on_a_chain_plays_one_pass_on_player_1", make_combo_on_a_chain_plays_one_pass_on_player_1 as fn()),
            ("the_chord_notice_shows_once", the_chord_notice_shows_once as fn()),
            ("the_empty_recording_notice_stays", the_empty_recording_notice_stays as fn()),
            ("record_trims_the_neutral_tail", record_trims_the_neutral_tail as fn()),
            ("migration_runs_once_at_boot", migration_runs_once_at_boot as fn()),
            ("a_menu_close_after_record_on_cpu_keeps_the_new_active_file", a_menu_close_after_record_on_cpu_keeps_the_new_active_file as fn()),
        ]);
    }
    }
}
