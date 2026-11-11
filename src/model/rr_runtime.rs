//! Host decisions for the recording runtime. The hook only reads a filled window.
//! Card IO and allocation stay on the worker.

use std::path::Path;

use super::library::{
    migrate, Chain, Error, Kind, LegacySlot, LoopCount, NamedSave, RecordingIndex, Role, Sequence,
    StartSaveState, Use, MACROS_DIR, SAVES_DIR, SEQUENCES_DIR,
};
use super::macro_ui::macro_panel::{fighter_gate, FighterGate, FighterOrigin};
use super::macros::{
    FrameSnapshot, FrameWindow, InputFrame, Library, MacroRecording, StartState, MAX_FRAMES,
};
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

/// Stop. An empty buffer writes nothing. A recording writes `"<name> · start"` from `player` and `cpu` (where Record
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
    let mut recording = MacroRecording::new(&name, "13.0.5", start_state, capture.frames.clone()).map_err(record)?;
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

