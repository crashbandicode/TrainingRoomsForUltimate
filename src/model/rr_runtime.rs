//! Host decisions for the recording runtime. The hook only reads a filled window.
//! Card IO and allocation stay on the worker.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::model::recording_menu::{Catalog, ComboFile, LoopMark, MacroFile, RecordingScreens, SaveFile, SequenceFile};
use crate::model::store::{catalog_changes, try_recording_read, CatalogListing, CatalogSignatures, Record, Store, CATALOG_DIRS};

use super::library::{
    migrate, Chain, Error, Kind, LegacySlot, LoopCount, NamedSave, RecordingIndex, Role, Sequence,
    StartSaveState, Use, MACROS_DIR, SAVES_DIR,
};
#[cfg(test)]
use super::macro_ui::macro_panel::{fighter_gate, FighterGate, FighterOrigin};
use super::macros::{InputFrame, Library, MacroRecording, StartState};
#[cfg(test)]
use super::macros::MAX_FRAMES;
use super::playback::{Book, Playback};
use crate::model::combo::Combo;
use crate::model::combo_record::{clean_name, default_name, drill_take_name, is_drill_take_name, ComboLibrary, ComboRecord};
use crate::model::state::record::SavedState;

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
    let name = unused_name("Macro", &macro_names);
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
    RecordingIndex::update(root, |index| {
        index.note_use(Kind::Save, &start_id, Use::Recorded, at)?;
        index.note_use(Kind::Macro, &id, Use::Recorded, at.saturating_add(1))?;
        index.set_active(capture.role(), Some(&id), at.saturating_add(2))?;
        Ok(true)
    })?;
    Ok(RecordStop::Saved {
        id,
        start_save: start_id,
    })
}

/// A directly recorded drill's frames, as a macro named by its `take` (F10). The menu hides it and `delete_combo`
/// removes it with the drill. No start save, active role or recent-use mark. With `trim` the idle tail goes first;
/// `None` when no frame is left.
pub fn save_drill_take(root: &Path, mut frames: Vec<InputFrame>, start: StartState, trim: bool, at: u64) -> Result<Option<String>, Error> {
    if trim {
        super::macro_ui::macro_panel::crop_neutral_tail(&mut frames);
    }
    if frames.is_empty() {
        return Ok(None);
    }
    let macros = Library::new(root.join(MACROS_DIR));
    let name = drill_take_name(drill_take_key(&frames, at));
    let recording = MacroRecording::new(name, "13.0.5", start, frames).map_err(record)?;
    macros.save_new(&recording).map_err(record).map(Some)
}

/// The key in a take's name: FNV-1a 64 over the take's frames (per frame the buttons little-endian, then the four
/// stick bytes) and then `at` little-endian. Two takes share a key only with the same frames saved in the same second.
fn drill_take_key(frames: &[InputFrame], at: u64) -> u64 {
    frames.iter().flat_map(|frame| frame.buttons.to_le_bytes().into_iter().chain([
        frame.lstick_x as u8, frame.lstick_y as u8, frame.rstick_x as u8, frame.rstick_y as u8,
    ])).chain(at.to_le_bytes()).fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
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

/// A kept file owned by the shared clip editor until the keyboard confirms it.
pub enum ClipFile { Macro(Box<MacroRecording>), Combo(Box<ComboRecord>) }

/// Always creates a fresh id; empty keyboard text chooses the next unused number.
pub fn save_clip_file(root: &Path, file: ClipFile, typed: &str, follow: Option<(Role, &str)>) -> Result<String, Error> {
    let kind = match &file { ClipFile::Macro(_) => Kind::Macro, ClipFile::Combo(_) => Kind::Combo };
    let id = match file {
        ClipFile::Macro(mut recording) => {
            let library = Library::new(root.join(MACROS_DIR));
            let names: Vec<_> = library.list().map_err(record)?.entries.into_iter().map(|entry| entry.name).collect();
            recording.name = if typed.trim().is_empty() { unused_name("Macro", &names) } else { typed.trim().to_owned() };
            library.save_new(&recording).map_err(record)
        }
        ClipFile::Combo(mut recording) => {
            let library = ComboLibrary::new(root.join("combos"));
            let names: Vec<_> = library.list().map_err(record)?.entries.into_iter().map(|entry| entry.name).collect();
            recording.name = if typed.trim().is_empty() { unused_name(recording.unit_kind().name_prefix(), &names) } else { typed.trim().to_owned() };
            library.save_new(&recording).map_err(record)
        }
    }?;
    super::rr_host::note_hot(root, kind, &id, follow);
    Ok(id)
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
pub fn save_made_combo(root: &Path, combo: &Combo, source: &str, recorded_on: Option<String>, at: u64) -> Result<String, Error> {
    save_combo(root, combo, source, recorded_on, None, at)
}

/// A directly recorded input drill (F10): saved as Drill N like a made combo, with the macro file that holds its
/// take named in the drill's `take`. No source: `made_from` stays empty.
/// If the drill file cannot be written the take is deleted, so no take is left under MACROS. Once the drill file
/// exists both files stay, also when the index update then fails.
pub fn save_drill_with_take(root: &Path, combo: &Combo, recorded_on: Option<String>, take: String, at: u64) -> Result<String, Error> {
    save_combo(root, combo, "", recorded_on, Some(take), at)
}

fn save_combo(root: &Path, combo: &Combo, source: &str, recorded_on: Option<String>, take: Option<String>, at: u64) -> Result<String, Error> {
    let library = ComboLibrary::new(root.join("combos"));
    let cleanup = || {
        if let Some(take) = take.as_deref() {
            let _ = Library::new(root.join(MACROS_DIR)).delete(take);
        }
    };
    let entries = library.list().map_err(|err| { cleanup(); record(err) })?.entries;
    let prefix = if combo.input_only { "Drill" } else { "Combo" };
    let name = crate::model::store::lowest_free(|n| format!("{prefix} {n}"), entries.iter().map(|e| e.name.as_str()));
    let mut made = ComboRecord::new(name, *combo, recorded_on);
    if !source.is_empty() { made = made.with_made_from(source); }
    if let Some(take) = take.as_deref() { made = made.with_take(take); }
    let id = library.save_new(&made).map_err(|err| { cleanup(); record(err) })?;
    RecordingIndex::update(root, |index| {
        index.note_use(Kind::Combo, &id, Use::Recorded, at)?;
        Ok(true)
    })?;
    Ok(id)
}

/// Delete a combo or drill file. A drill's take (F10) goes with it, unless another drill or combo the library reads
/// still names it; a file the library skips is not a drill and keeps no take. That is settled before the drill file
/// is touched: when the folder cannot be read nothing is deleted. Only a macro with a take's name is ever deleted:
/// a file that names any other macro leaves it alone.
pub fn delete_combo(root: &Path, id: &str) -> bool {
    let combos = ComboLibrary::new(root.join("combos"));
    let take = combos.load(id).ok().and_then(|file| file.take);
    let mut goes = take.is_some();
    if let Some(take) = take.as_deref() {
        let Ok(listing) = combos.list() else { return false };
        for entry in listing.entries.iter().filter(|entry| entry.id != id) {
            let Ok(file) = combos.load(&entry.id) else { return false };
            goes &= file.take.as_deref() != Some(take);
        }
    }
    if combos.delete(id).is_err() {
        return false;
    }
    if let Some(take) = take.filter(|_| goes) {
        let macros = Library::new(root.join(MACROS_DIR));
        if macros.load(&take).is_ok_and(|file| is_drill_take_name(&file.name)) {
            let _ = macros.delete(&take);
        }
    }
    true
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
    RecordingIndex::update(root, |index| {
        index.note_use(Kind::Combo, &id, Use::Recorded, at)?;
        index.set_drill(Some(super::library::Drill { kind: super::library::DrillKind::Combo, id: id.clone() }), at)?;
        Ok(true)
    })?;
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
    let copy = NamedSave::new(unused_name("Save", &names), source.player, source.cpu)?;
    let copy_id = store.save_new(&copy)?;
    RecordingIndex::update(root, |index| {
        index.note_use(Kind::Save, &copy_id, Use::Recorded, at)?;
        Ok(true)
    })?;
    Ok(copy_id)
}

pub fn delete_save(root: &Path, id: &str, at: u64) -> Result<(), Error> {
    let store = Store::<NamedSave>::new(root.join(SAVES_DIR));
    store.delete(id)?;
    RecordingIndex::update(root, |index| {
        let was_bound = index.bound() == Some(id);
        if was_bound {
            index.bind_save(None, at)?;
        }
        Ok(was_bound)
    })?;
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
    let bound = {
        let _guard = crate::model::store::recording_write_guard(root);
        RecordingIndex::load(root)?.bound().map(str::to_owned)
    };
    if let Some(id) = bound.as_deref() {
        if let Ok(mut existing) = store.load(id) {
            existing.player = player;
            existing.cpu = cpu;
            store.overwrite(id, &existing)?;
            RecordingIndex::update(root, |index| {
                index.note_use(Kind::Save, id, Use::Bound, at)?;
                Ok(true)
            })?;
            return Ok(id.to_owned());
        }
    }
    let names = store
        .list()?
        .entries
        .into_iter()
        .map(|entry| entry.name)
        .collect::<Vec<_>>();
    let save = NamedSave::new(unused_name("Save", &names), player, cpu)?;
    let id = store.save_new(&save)?;
    RecordingIndex::update(root, |index| {
        if index.bound() == bound.as_deref() {
            index.bind_save(Some(&id), at)?;
        } else {
            index.note_use(Kind::Save, &id, Use::Recorded, at)?;
        }
        Ok(true)
    })?;
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
    format!("{prefix} {}", crate::model::recording_menu::unused_n(prefix, names))
}

/// A catalog's entries are the files only. Hot lists and current selection are
/// applied separately from a successfully read recording index.
pub fn apply_catalog(catalog: &Catalog, index: &RecordingIndex, screens: &mut RecordingScreens) {
    let mut catalog = catalog.clone();
    catalog.hot_macros = index.hot(Kind::Macro).to_vec();
    catalog.hot_saves = index.hot(Kind::Save).to_vec();
    catalog.hot_combos = [false, true]
        .into_iter()
        .flat_map(|input| index.hot_where(Kind::Combo, |id| {
            catalog.combos.iter().any(|file| file.id == id && file.input_only == input)
        }))
        .collect();
    catalog.hot_sequences = index.hot(Kind::Sequence).to_vec();
    screens.set_catalog(catalog);
}

pub fn apply_index(index: &RecordingIndex, screens: &mut RecordingScreens) {
    let mark = |role| match index.loop_count(role) {
        LoopCount::Infinite => LoopMark::Infinite,
        LoopCount::Finite(n) => LoopMark::Finite(n),
    };
    screens.adopt(
        index.bound().map(str::to_owned),
        [index.active(Role::Player).map(str::to_owned), index.active(Role::Cpu).map(str::to_owned)],
        [mark(Role::Player), mark(Role::Cpu)],
        index.drill().map(|drill| drill.id.clone()),
    );
    screens.adopt_slots(index.slots().clone());
}

#[cfg(test)]
pub fn load_catalog(root: &Path) -> Result<Catalog, Error> {
    let mut cache = CatalogCache::new(root);
    cache.want(0);
    while cache.step() {}
    let index = RecordingIndex::load(root)?;
    let mut screens = RecordingScreens::new();
    apply_catalog(cache.ready(0).unwrap(), &index, &mut screens);
    Ok(screens.catalog().clone())
}

enum CatalogEntry {
    Macro(MacroFile),
    Save(SaveFile),
    Combo(ComboFile),
    Sequence(SequenceFile),
}

fn catalog_entry(root: &Path, folder: usize, id: String) -> std::io::Result<Option<CatalogEntry>> {
    let dir = root.join(CATALOG_DIRS[folder]);
    let entry = match folder {
        0 => {
            let Some(file) = Library::new(dir).load_with_sibling::<Chain>(&id)? else { return Ok(None) };
            CatalogEntry::Macro(match file {
                Ok(file) => MacroFile {
                    id,
                    name: file.name,
                    chain: false,
                    steps: Vec::new(),
                    start_save: file.start_save,
                    start_save_on: file.start_save_on,
                    recorded_on: fighter_name(file.start.fighter_kind),
                    span: file.frames.len() as u32,
                },
                Err(file) => MacroFile {
                    id,
                    name: file.name,
                    chain: true,
                    steps: file.recordings,
                    start_save: None,
                    start_save_on: file.start_save_on,
                    recorded_on: String::new(),
                    span: 0,
                },
            })
        }
        1 => {
            let Some(save) = Store::<NamedSave>::new(dir).load_for_catalog(&id)? else { return Ok(None) };
            let fighters = [&save.player, &save.cpu]
                .into_iter()
                .filter(|state| state.is_saved())
                .map(|state| fighter_name(state.fighter_kind))
                .filter(|name| !name.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            CatalogEntry::Save(SaveFile { id, name: save.name, fighters })
        }
        2 => {
            let Some(file) = ComboLibrary::new(dir).load_for_catalog(&id)? else { return Ok(None) };
            CatalogEntry::Combo(ComboFile {
                id,
                name: file.name,
                input_only: file.combo.input_only,
                recorded_on: file.recorded_on.unwrap_or_default(),
                made_from: file.made_from,
                take: file.take,
                links: crate::model::recording_menu::combo_link_lines(&file.combo),
                span: file.combo.len() as u32,
            })
        }
        _ => {
            let Some(file) = Store::<Sequence>::new(dir).load_for_catalog(&id)? else { return Ok(None) };
            CatalogEntry::Sequence(SequenceFile {
                id,
                name: file.name,
                combos: file.combos,
                streak: file.streak,
                best: file.best,
            })
        }
    };
    Ok(Some(entry))
}

fn accepts_macro_backup(id: &str) -> bool {
    MacroRecording::accepts_id(id) || Chain::accepts_id(id)
}

struct CatalogFile {
    size: u64,
    entry: Option<CatalogEntry>,
    retry: bool,
}

/// A names-and-sizes check is staged until all four folders are listed.
#[derive(Default)]
struct FolderCheck {
    at: usize,
    listing: Option<CatalogListing>,
    changes: CatalogSignatures,
}

pub struct CatalogCache {
    root: PathBuf,
    generation: Option<u64>,
    ready: Option<(u64, Catalog)>,
    files: BTreeMap<(usize, String), CatalogFile>,
    pending: BTreeSet<(usize, String)>,
    /// New entries finish their cold read before repeatedly rewritten entries.
    revised: BTreeSet<(usize, String)>,
    cold: Option<FolderCheck>,
    cold_writes: CatalogSignatures,
    check: Option<FolderCheck>,
    external: CatalogSignatures,
    external_changed: bool,
}

impl Default for CatalogCache {
    fn default() -> Self {
        Self::new(Path::new(crate::model::storage::ROOT))
    }
}

impl CatalogCache {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            generation: None,
            ready: None,
            files: BTreeMap::new(),
            pending: BTreeSet::new(),
            revised: BTreeSet::new(),
            cold: Some(FolderCheck::default()),
            cold_writes: CatalogSignatures::new(),
            check: None,
            external: CatalogSignatures::new(),
            external_changed: false,
        }
    }

    pub fn want(&mut self, generation: u64) {
        if self.generation == Some(generation) {
            return;
        }
        self.generation = Some(generation);
        self.ready = None;
        let external = core::mem::take(&mut self.external);
        self.apply_changes(external);
        // Committed writes also refresh an entry whose JSON length did not change.
        let writes = catalog_changes(&self.root);
        if self.cold.is_some() {
            self.cold_writes.extend(writes);
        } else {
            self.apply_changes(writes);
            if self.check.is_some() {
                self.check();
            }
        }
    }

    fn apply_changes(&mut self, changes: CatalogSignatures) {
        for (key, size) in changes {
            if let Some(size) = size {
                if let Some(file) = self.files.get_mut(&key) {
                    file.size = size;
                    if !self.pending.contains(&key) {
                        self.revised.insert(key);
                    }
                } else {
                    self.files.insert(key.clone(), CatalogFile { size, entry: None, retry: false });
                    self.pending.insert(key);
                }
            } else {
                self.files.remove(&key);
                self.pending.remove(&key);
                self.revised.remove(&key);
            }
        }
    }

    /// One listing batch or one payload read between input passes.
    // Keep catalog scratch space out of the menu worker's other call paths.
    #[inline(never)]
    pub fn step(&mut self) -> bool {
        let Some(generation) = self.generation else { return false };
        if self.ready.is_some() {
            return self.check_step();
        }
        if let Some(mut cold) = self.cold.take() {
            if self.scan_step(&mut cold, true) {
                self.cold = Some(cold);
            } else {
                // Writes observed during a cold scan win over its directory snapshots.
                cold.changes.extend(core::mem::take(&mut self.cold_writes));
                self.apply_changes(cold.changes);
            }
            return true;
        }
        if let Some(key) = self.pending.pop_first().or_else(|| self.revised.pop_first()) {
            let outcome = catalog_entry(&self.root, key.0, key.1.clone());
            let file = self.files.get_mut(&key).unwrap();
            file.retry = outcome.is_err();
            file.entry = outcome.ok().flatten();
            return true;
        }
        let mut catalog = Catalog::default();
        for file in self.files.values().filter_map(|file| file.entry.as_ref()) {
            match file {
                CatalogEntry::Macro(file) => catalog.macros.push(file.clone()),
                CatalogEntry::Save(file) => catalog.saves.push(file.clone()),
                CatalogEntry::Combo(file) => catalog.combos.push(file.clone()),
                CatalogEntry::Sequence(file) => catalog.sequences.push(file.clone()),
            }
        }
        catalog.macros.sort_by_cached_key(|file| (file.chain, file.name.to_lowercase(), file.id.clone()));
        catalog.saves.sort_by_cached_key(|file| (file.name.to_lowercase(), file.id.clone()));
        catalog.combos.sort_by_cached_key(|file| (file.name.to_lowercase(), file.id.clone()));
        catalog.sequences.sort_by_cached_key(|file| (file.name.to_lowercase(), file.id.clone()));
        self.ready = Some((generation, catalog));
        self.check.is_some()
    }

    #[inline(never)]
    fn start_listing(&self, folder: usize) -> CatalogListing {
        CatalogListing::open(&self.root.join(CATALOG_DIRS[folder]))
    }

    fn scan_step(&self, check: &mut FolderCheck, cold: bool) -> bool {
        let Some(guard) = try_recording_read() else { return true };
        let Some(listing) = check.listing.as_mut() else {
            check.listing = Some(self.start_listing(check.at));
            return true;
        };
        let recover = cold.then_some(match check.at {
            0 => accepts_macro_backup as fn(&str) -> bool,
            1 => NamedSave::accepts_id,
            2 => ComboRecord::accepts_id,
            _ => Sequence::accepts_id,
        });
        if listing.step(recover) {
            return true;
        }
        drop(guard);
        let listing = check.listing.take().unwrap();
        self.finish_listing(check, listing);
        check.at += 1;
        check.at < CATALOG_DIRS.len()
    }

    #[inline(never)]
    fn finish_listing(&self, check: &mut FolderCheck, listing: CatalogListing) {
        let Some(files) = listing.finish() else { return };
        for key in self.files.keys().filter(|key| key.0 == check.at) {
            if !files.contains_key(&key.1) {
                check.changes.insert(key.clone(), None);
            }
        }
        for (id, size) in files {
            let key = (check.at, id);
            if self.files.get(&key).is_none_or(|file| file.size != size || file.retry) {
                check.changes.insert(key, Some(size));
            }
        }
    }

    /// Queue an external names-and-sizes check even if the cold catalog is building.
    pub fn check(&mut self) {
        self.check = Some(FolderCheck::default());
    }

    fn check_step(&mut self) -> bool {
        let Some(mut check) = self.check.take() else { return false };
        if self.scan_step(&mut check, false) {
            self.check = Some(check);
        } else if !check.changes.is_empty() {
            self.external = check.changes;
            self.external_changed = true;
        }
        true
    }

    pub fn take_external_change(&mut self) -> bool {
        core::mem::take(&mut self.external_changed)
    }

    pub fn ready(&self, generation: u64) -> Option<&Catalog> {
        self.ready.as_ref().filter(|(ready, _)| *ready == generation).map(|(_, catalog)| catalog)
    }
}

/// Menu state from the index. An unsuccessful read leaves the requested revision unready.
#[derive(Default)]
pub struct MenuIndex {
    seen: Option<(u64, u64)>,
    index: Option<RecordingIndex>,
}

impl MenuIndex {
    /// Returns true when this pass took the index read slot, even if a writer held the card.
    // Keep JSON decoding in its own frame for the menu worker's stack bound.
    #[inline(never)]
    pub fn step(&mut self, root: &Path, version: (u64, u64)) -> bool {
        if self.ready(version).is_some() {
            return false;
        }
        let Some(_guard) = crate::model::store::try_recording_read() else { return true };
        self.seen = Some(version);
        self.index = RecordingIndex::load(root).ok();
        true
    }

    pub fn ready(&self, version: (u64, u64)) -> Option<&RecordingIndex> {
        self.index.as_ref().filter(|_| self.seen == Some(version))
    }
}


// The fighter table is the switch (and test) roster. A host library build has no names.
#[cfg(any(test, target_os = "switch"))]
pub(super) fn fighter_name(kind: i32) -> String {
    if kind < 0 {
        return String::new();
    }
    crate::engine::swap::trio::TrioFighter::from_kind(kind as u32)
        .map(|fighter| fighter.name().to_owned())
        .unwrap_or_default()
}

#[cfg(not(any(test, target_os = "switch")))]
pub(super) fn fighter_name(_kind: i32) -> String {
    String::new()
}

fn record(err: impl ToString) -> Error {
    Error::Record(err.to_string())
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::super::library::{Drill, DrillKind, INDEX_FILE, SEQUENCES_DIR};
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

    fn a_failed_drill_save_deletes_its_take() {
        let root = scratch("failed-drill-save");
        std::fs::write(root.join("combos"), b"not a directory").unwrap();
        let macros = Library::new(root.join(MACROS_DIR));
        let take = save_drill_take(&root, vec![frame(1), frame(0)], StartState::neutral(8, 0, 1.0), false, 1).unwrap().unwrap();
        assert_eq!(macros.list().unwrap().entries.len(), 1);
        let mut step = LinkRec::INPUT;
        step.events[0] = Some(Event::press(Button::Jump, Aim::Neutral, 0));
        let mut combo = Combo::from_links([step]).unwrap();
        combo.input_only = true;
        assert!(save_drill_with_take(&root, &combo, None, take, 1).is_err());
        assert!(macros.list().unwrap().entries.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    fn input_drill() -> Combo {
        let mut step = LinkRec::INPUT;
        step.events[0] = Some(Event::press(Button::Jump, Aim::Neutral, 0));
        let mut combo = Combo::from_links([step]).unwrap();
        combo.input_only = true;
        combo
    }

    fn an_index_failure_after_the_drill_file_keeps_the_pair() {
        let root = scratch("drill-index-failure");
        std::fs::create_dir(root.join(INDEX_FILE)).unwrap();
        let first = vec![frame(1), frame(0)];
        let second = vec![frame(2), frame(0)];
        let take1 = save_drill_take(&root, first.clone(), StartState::neutral(8, 0, 1.0), false, 1).unwrap().unwrap();
        assert!(save_drill_with_take(&root, &input_drill(), None, take1.clone(), 1).is_err());
        let combos = ComboLibrary::new(root.join("combos"));
        let macros = Library::new(root.join(MACROS_DIR));
        let entries = combos.list().unwrap().entries;
        assert_eq!(entries.len(), 1);
        let drill1 = entries[0].id.clone();
        assert_eq!(combos.load(&drill1).unwrap().take.as_deref(), Some(take1.as_str()));
        assert_eq!(macros.load(&take1).unwrap().frames, first);
        std::fs::remove_dir(root.join(INDEX_FILE)).unwrap();
        let take2 = save_drill_take(&root, second.clone(), StartState::neutral(8, 0, 1.0), false, 1).unwrap().unwrap();
        let drill2 = save_drill_with_take(&root, &input_drill(), None, take2.clone(), 1).unwrap();
        assert_ne!(take1, take2);
        assert!(delete_combo(&root, &drill1));
        assert!(macros.load(&take1).is_err());
        assert_eq!(macros.load(&take2).unwrap().frames, second);
        assert_eq!(combos.load(&drill2).unwrap().take.as_deref(), Some(take2.as_str()));
    }

    fn a_drill_file_from_another_card_neither_plays_nor_deletes_the_local_take() {
        let a = scratch("drill-card-a");
        let b = scratch("drill-card-b");
        let first = vec![frame(1), frame(0)];
        let second = vec![frame(2), frame(0)];
        let take_a = save_drill_take(&a, first.clone(), StartState::neutral(8, 0, 1.0), false, 1).unwrap().unwrap();
        let drill_a = save_drill_with_take(&a, &input_drill(), None, take_a.clone(), 1).unwrap();
        let take_b = save_drill_take(&b, second, StartState::neutral(8, 0, 1.0), false, 1).unwrap().unwrap();
        let drill_b = save_drill_with_take(&b, &input_drill(), None, take_b.clone(), 1).unwrap();
        assert_ne!(take_a, take_b);
        std::fs::copy(b.join("combos").join(format!("{drill_b}.json")), a.join("combos").join("copied.json")).unwrap();
        let combos = ComboLibrary::new(a.join("combos"));
        let macros = Library::new(a.join(MACROS_DIR));
        assert_eq!(combos.load("copied").unwrap().take.as_deref(), Some(take_b.as_str()));
        assert!(macros.load(&take_b).is_err());
        assert!(delete_combo(&a, "copied"));
        assert_eq!(macros.load(&take_a).unwrap().frames, first);
        assert_eq!(combos.load(&drill_a).unwrap().take.as_deref(), Some(take_a.as_str()));
    }

    fn a_take_two_drill_files_name_survives_the_first_delete() {
        let root = scratch("drill-shared-take");
        let take = save_drill_take(&root, vec![frame(1), frame(0)], StartState::neutral(8, 0, 1.0), false, 1).unwrap().unwrap();
        let drill = save_drill_with_take(&root, &input_drill(), None, take.clone(), 1).unwrap();
        std::fs::copy(root.join("combos").join(format!("{drill}.json")), root.join("combos").join("copied.json")).unwrap();
        let macros = Library::new(root.join(MACROS_DIR));
        assert!(delete_combo(&root, "copied"));
        assert!(macros.load(&take).is_ok());
        assert!(delete_combo(&root, &drill));
        assert!(macros.load(&take).is_err());
    }

    fn a_skipped_combo_file_does_not_keep_a_deleted_drills_take() {
        let root = scratch("drill-skipped-combo");
        let take = save_drill_take(&root, vec![frame(1), frame(0)], StartState::neutral(8, 0, 1.0), false, 1).unwrap().unwrap();
        let drill = save_drill_with_take(&root, &input_drill(), None, take, 1).unwrap();
        let typo = root.join("combos").join("typo.json");
        std::fs::write(&typo, br#"{"schema_version":1,"name":"Typo","links":[{"from":"aerial","to":"aeria","verdict":{"kind":"true","link":-2},"steps":[{"button":"jump","aim":"neutral","offset":2}]}]}"#).unwrap();
        let combos = ComboLibrary::new(root.join("combos"));
        let skipped = combos.list().unwrap().skipped;
        assert!(skipped.iter().any(|file| file.file == "typo.json" && file.reason.contains("bad combo field: to")));
        assert!(delete_combo(&root, &drill));
        assert!(combos.load(&drill).is_err());
        assert!(Library::new(root.join(MACROS_DIR)).list().unwrap().entries.is_empty());
        assert!(typo.is_file());
    }

    fn a_combo_folder_that_cannot_be_read_refuses_the_delete_and_keeps_the_pair() {
        let root = scratch("drill-unreadable-combos");
        let take = save_drill_take(&root, vec![frame(1), frame(0)], StartState::neutral(8, 0, 1.0), false, 1).unwrap().unwrap();
        let drill = save_drill_with_take(&root, &input_drill(), None, take.clone(), 1).unwrap();
        let dir = root.join("combos");
        let held = root.join("combos-held");
        let hook_dir = dir.clone();
        let hook_held = held.clone();
        crate::model::store::before_next_store_list(move || {
            std::fs::rename(&hook_dir, &hook_held).unwrap();
            std::fs::write(&hook_dir, b"not a directory").unwrap();
        });
        assert!(!delete_combo(&root, &drill));
        assert!(held.join(format!("{drill}.json")).is_file());
        let macros = Library::new(root.join(MACROS_DIR));
        assert!(macros.load(&take).is_ok());
        std::fs::remove_file(&dir).unwrap();
        std::fs::rename(&held, &dir).unwrap();
        assert_eq!(ComboLibrary::new(dir).load(&drill).unwrap().take.as_deref(), Some(take.as_str()));
        assert!(macros.load(&take).is_ok());
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
        close_overlay(&mut screens);
        open_row(&mut screens, SAVES_MACROS, 1, "+ New chain");
        screens.handle_page(MenuButton::X, true, SAVES_MACROS);
        screens.submit_name("");
        assert!(screens.has_chain_named("Chain 1"));
    }

    /// Operator 2026-11-15: Quickplay's room opens the name keyboard like a local-wireless room, so the menu has no
    /// blocked mode any more. Rename, a new chain and Save last combo each ask for a name; nothing is saved or renamed
    /// before the keyboard answers.
    fn an_online_room_opens_every_name_keyboard() {
        let mut screens = RecordingScreens::new();
        screens.set_catalog(header_catalog());
        assert_header(&mut screens, SAVES_MACROS, 1, "Nair", "Name macro");
        assert_eq!(screens.take_effect(), Effect::Keyboard { header: "Name macro", draft: "Nair".into() });
        close_overlay(&mut screens);
        open_row(&mut screens, SAVES_MACROS, 1, "+ New chain");
        screens.handle_page(MenuButton::X, true, SAVES_MACROS);
        assert!(matches!(screens.take_effect(), Effect::Keyboard { header: "Name chain", .. }));
        assert!(!screens.has_chain_named("Chain 1"), "no chain before the name is answered");
        screens.submit_name("");
        assert!(screens.has_chain_named("Chain 1"));
        let _ = screens.take_effect();
        screens.set_cursor(Face::Page, COMBOS_SEQ, 1, 0);
        screens.handle_page(MenuButton::A, true, COMBOS_SEQ);
        assert!(matches!(screens.take_effect(), Effect::Keyboard { header: "Name combo", .. }));
    }

    fn file_clip_save_as_new_preserves_original() {
        use crate::engine::macro_ui::clip_panel::{ClipAction, ClipEditor, ClipEffect};
        let root = scratch("shared-clip");
        let macros = Library::new(root.join(MACROS_DIR));
        let source = recording("Original", &[1, 2, 3, 4], Some("keep"));
        let id = macros.save_new(&source).unwrap();
        let path = macros.dir().join(format!("{id}.json"));
        let before = std::fs::read(&path).unwrap();
        let mut editor = ClipEditor::macro_file(source, false).unwrap();
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(1));
        let ClipEffect::Export(cut) = editor.on_action(ClipAction::Confirm) else { panic!("macro export"); };
        let new_id = save_clip_file(&root, ClipFile::Macro(cut), "Kept", None).unwrap();
        assert_ne!(id, new_id);
        assert_eq!(std::fs::read(path).unwrap(), before);
        let kept = macros.load(&new_id).unwrap();
        assert_eq!(kept.name, "Kept");
        assert_eq!(kept.frames.iter().map(|f| f.buttons).collect::<Vec<_>>(), vec![2, 3, 4]);
        assert_eq!(kept.start_save.as_deref(), Some("keep"));

        let combos = ComboLibrary::new(root.join("combos"));
        let source = ComboRecord::new("Original".into(), Combo::from_links([
            link(Move::Aerial, Move::SideTilt), link(Move::SideTilt, Move::NeutralSpecial),
        ]).unwrap(), None);
        let id = combos.save_new(&source).unwrap();
        let path = combos.dir().join(format!("{id}.json"));
        let before = std::fs::read(&path).unwrap();
        let mut editor = ClipEditor::combo_file(source).unwrap();
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(1));
        let ClipEffect::ExportCombo(cut) = editor.on_action(ClipAction::Confirm) else { panic!("combo export"); };
        let new_id = save_clip_file(&root, ClipFile::Combo(cut), "Last link", None).unwrap();
        assert_ne!(id, new_id);
        assert_eq!(std::fs::read(path).unwrap(), before);
        let kept = combos.load(&new_id).unwrap();
        assert_eq!(kept.name, "Last link");
        assert_eq!(kept.combo.links().count(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A crop follows its source only when that source is the role's active macro.
    fn a_saved_crop_of_the_roles_active_macro_becomes_active() {
        let root = scratch("crop-active");
        let macros = Library::new(root.join(MACROS_DIR));
        let source = recording("Source", &[1, 2, 3, 4], None);
        let source_id = macros.save_new(&source).unwrap();
        let other_id = macros.save_new(&recording("Other", &[5], None)).unwrap();
        let cpu_id = macros.save_new(&recording("CPU", &[6], None)).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&source_id), 1).unwrap();
        index.set_active(Role::Cpu, Some(&cpu_id), 2).unwrap();
        index.save(&root).unwrap();
        let cut = || {
            let mut cropped = source.clone();
            cropped.frames = source.frames[1..3].to_vec();
            ClipFile::Macro(Box::new(cropped))
        };
        let new_id = save_clip_file(&root, cut(), "Crop", Some((Role::Player, &source_id))).unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.active(Role::Player), Some(new_id.as_str()));
        assert_eq!(index.active(Role::Cpu), Some(cpu_id.as_str()));
        index.set_active(Role::Player, Some(&other_id), 3).unwrap();
        index.save(&root).unwrap();
        save_clip_file(&root, cut(), "Other crop", Some((Role::Player, &source_id))).unwrap();
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.active(Role::Player), Some(other_id.as_str()));
        assert_eq!(index.active(Role::Cpu), Some(cpu_id.as_str()));
        save_clip_file(&root, cut(), "No follow", None).unwrap();
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.active(Role::Player), Some(other_id.as_str()));
        assert_eq!(index.active(Role::Cpu), Some(cpu_id.as_str()));
    }

    fn drill_units_share_clip_editor_and_save_same_kind() {
        use crate::engine::macro_ui::{clip_panel::{clip_help, ClipAction, ClipContext, ClipEditor, ClipEffect}, macro_panel::{MacroUi, NameContext, Effect as NameEffect, CLIP_NAME_TOKEN}};
        use crate::model::{combo::InputCapture, combo_record::UnitKind, frame_strip::Snap};
        let mut capture = InputCapture::new();
        for (frame, buttons) in [(0, 1), (2, 0), (23, 2), (25, 0), (46, 1), (48, 0)] {
            capture.step(Snap { buttons, facing: 1, ..Snap::empty(frame) });
        }
        let source = ComboRecord::new("Drill 1".into(), capture.combo, Some("Mario".into())).with_made_from("macro-7");
        let root = scratch("drill-clip");
        let library = ComboLibrary::new(root.join("combos"));
        let original = library.save_new(&source).unwrap();
        let path = root.join("combos").join(format!("{original}.json"));
        let bytes = std::fs::read(&path).unwrap();
        let mut editor = ClipEditor::combo_file(library.load(&original).unwrap()).unwrap();
        assert_eq!(editor.view().context, ClipContext::Combo { unit: UnitKind::Step });
        assert_eq!(editor.view().total, 3);
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(1));
        editor.on_action(ClipAction::SelectOut);
        editor.on_action(ClipAction::Nudge(-1));
        assert_eq!((editor.view().start, editor.view().end), (1, 2));
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::None));
        let help = clip_help(&editor.view(), "ZL", "ZL", 20);
        assert!(!help.controls.contains("X preview"));
        assert!(help.follow.contains("steps"));
        let mut naming = MacroUi::new();
        naming.offer_name(NameContext { token: CLIP_NAME_TOKEN, title: "Name drill" }, "Drill 1", "Save as a new file");
        let NameEffect::NameConfirmed { name, .. } = naming.confirm_native_name(String::new()) else { panic!("empty drill name"); };
        let ClipEffect::ExportCombo(kept) = editor.on_action(ClipAction::Confirm) else { panic!("drill export"); };
        let id = save_clip_file(&root, ClipFile::Combo(kept), &name, None).unwrap();
        let saved = library.load(&id).unwrap();
        assert_eq!(saved.name, "Drill 2");
        assert_eq!((saved.unit_kind(), saved.unit_count()), (UnitKind::Step, 1));
        assert_eq!(saved.combo.get(0).unwrap().events().next().unwrap().offset, 0);
        assert_eq!(saved.made_from, source.made_from);
        assert_eq!(saved.recorded_on, source.recorded_on);
        assert_ne!(id, original);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(matches!(editor.on_action(ClipAction::Cancel), ClipEffect::Close));
        std::fs::remove_dir_all(root).unwrap();
    }

    fn file_clip_empty_name_uses_unused_number() {
        use crate::engine::macro_ui::{clip_panel::{ClipAction, ClipEditor, ClipEffect}, macro_panel::{MacroUi, NameContext, Effect as NameEffect, CLIP_NAME_TOKEN}};
        let mut naming = MacroUi::new();
        naming.offer_name(NameContext { token: CLIP_NAME_TOKEN, title: "Name macro" }, "Original", "Save as a new file");
        let NameEffect::NameConfirmed { name, .. } = naming.confirm_native_name(String::new()) else { panic!("empty name confirmation"); };
        let root = scratch("clip-name");
        let source = recording("Macro 1", &[1, 2], None);
        let macros = Library::new(root.join(MACROS_DIR));
        macros.save_new(&source).unwrap();
        let mut editor = ClipEditor::macro_file(source, false).unwrap();
        let ClipEffect::Export(kept) = editor.on_action(ClipAction::Confirm) else { panic!("macro export"); };
        let id = save_clip_file(&root, ClipFile::Macro(kept), &name, None).unwrap();
        assert_eq!(macros.load(&id).unwrap().name, "Macro 2");
        let combos = ComboLibrary::new(root.join("combos"));
        let source = ComboRecord::new("Combo 1".into(), Combo::from_links([link(Move::Aerial, Move::SideTilt)]).unwrap(), None);
        combos.save_new(&source).unwrap();
        let mut editor = ClipEditor::combo_file(source).unwrap();
        let ClipEffect::ExportCombo(kept) = editor.on_action(ClipAction::Confirm) else { panic!("combo export"); };
        let id = save_clip_file(&root, ClipFile::Combo(kept), &name, None).unwrap();
        assert_eq!(combos.load(&id).unwrap().name, "Combo 2");
        std::fs::remove_dir_all(root).unwrap();
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
            combos: vec![ComboFile { input_only: false, take: None,
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
                screens.set_catalog(load_catalog(&root).unwrap());
                apply_index(&RecordingIndex::load(&root).unwrap(), screens);
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

    fn a_slot_assignment_survives_close_reopen_and_next_press() {
        let root = library_card("slot-reopen", 1);
        let mut cache = CatalogCache::new(&root);
        cache.want(0);
        while cache.step() {}
        let mut index_watch = MenuIndex::default();
        let mut open = |cache: &CatalogCache| {
            let version = (0, crate::model::store::index_generation());
            index_watch.step(&root, version);
            let index = index_watch.ready(version).unwrap();
            let mut menu = RecordingScreens::new();
            apply_catalog(cache.ready(0).unwrap(), index, &mut menu);
            apply_index(index, &mut menu);
            menu
        };
        let mut menu = open(&cache);
        open_row(&mut menu, SAVES_MACROS, 1, "Take 0");
        let slot = menu.file_rows().iter().position(|row| row.title == "Playback slot").unwrap();
        menu.set_file_cursor(slot);
        menu.handle_page(MenuButton::Right, true, SAVES_MACROS);
        let assigned = menu.selection().slots[0].clone();
        assert!(assigned.is_some());
        assert!(menu.take_selection_change(), "assigning a slot requests an index write");
        super::super::rr_host::publish_selection(&root, &menu.selection(), None);
        assert_eq!(RecordingIndex::load(&root).unwrap().slots()[0], assigned);
        drop(menu);
        let mut reopened = open(&cache);
        reopened.handle_page(MenuButton::Down, true, SAVES_MACROS);
        assert!(!reopened.take_selection_change(), "navigation leaves selection untouched");
        assert_eq!(RecordingIndex::load(&root).unwrap().slots()[0], assigned, "another button must not undo Slot 1");
        assert_eq!(reopened.selection().slots[0], assigned);
    }

    fn a_superseded_catalog_cannot_open_a_file_action() {
        let root = library_card("obsolete-file", 1);
        let loaded = load_catalog(&root).unwrap();
        let mut screens = RecordingScreens::new();
        screens.set_catalog(loaded);
        open_row(&mut screens, SAVES_MACROS, 1, "Take 0");
        let rename = screens.file_rows().iter().position(|row| row.title == "Rename").unwrap();
        screens.set_file_cursor(rename);
        screens.set_loading();
        screens.handle_page(MenuButton::A, true, SAVES_MACROS);
        assert!(!matches!(screens.take_effect(), Effect::Keyboard { .. }), "a stale file action must not open the rename keyboard");
    }

    fn an_index_read_failure_stays_unready_and_retries() {
        let root = library_card("bad-index-retry", 1);
        let mut index = MenuIndex::default();
        assert!(index.step(&root, (0, 0)));
        assert!(index.ready((0, 0)).is_some());
        let path = root.join(super::super::library::INDEX_FILE);
        std::fs::write(&path, b"invalid json").unwrap();
        assert!(index.step(&root, (1, 0)));
        assert!(index.ready((1, 0)).is_none(), "the previous index must not be relabelled");
        assert!(index.ready((0, 0)).is_none());
        RecordingIndex::fresh().save(&root).unwrap();
        assert!(index.step(&root, (1, 0)), "a readable index retries at the same revision");
        assert!(index.ready((1, 0)).is_some());
    }

    fn streak_saves_do_not_restart_a_populated_catalog_build() {
        let root = library_card("streak-churn", 60);
        let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
        for i in 0..30 {
            let save = NamedSave::new(format!("Save {i}"), state(8), state(7)).unwrap();
            saves.save_new(&save).unwrap();
        }
        let sequences = Store::<Sequence>::new(root.join(SEQUENCES_DIR));
        let sequence = sequences.save_new(&Sequence::new("Drill", vec!["combo-1".into()]).unwrap()).unwrap();
        let mut cache = CatalogCache::new(&root);
        let mut streak = 0;
        let reads_before = crate::model::store::file_reads();
        let mut generation = crate::model::store::catalog_generation();
        let mut writes = 0;
        let done = (0..140).any(|pass| {
            if pass % 10 == 0 {
                // Alternate one-digit values: a byte-size signature alone cannot see these rewrites.
                streak = if streak == 1 { 2 } else { 1 };
                assert!(super::super::rr_host::save_sequence_progress(&root, &sequence, streak, streak));
                let current = crate::model::store::catalog_generation();
                assert!(current > generation, "each real sequence save invalidates the catalog");
                generation = current;
                writes += 1;
            }
            cache.want(generation);
            let _ = cache.step();
            cache.ready(generation).is_some()
        });
        assert!(done, "60 macros and 30 saves must complete in 140 passes despite streak writes");
        let catalog = cache.ready(generation).unwrap();
        assert!(crate::model::store::file_reads() - reads_before <= 91 + 2 * writes,
            "generation churn does not re-read already decoded macros or saves");
        assert_eq!((catalog.macros.len(), catalog.saves.len()), (60, 30));
        assert_eq!((catalog.sequences[0].streak, catalog.sequences[0].best), (streak, streak));
    }

    /// A card with `files` recordings and its index, as the menu lists it.
    fn library_card(tag: &str, files: usize) -> std::path::PathBuf {
        let root = scratch(tag);
        RecordingIndex::load(&root).unwrap().save(&root).unwrap();
        let macros = Library::new(root.join(MACROS_DIR));
        for i in 0..files {
            macros.save_new(&recording(&format!("Take {i}"), &[0, 1, 0], None)).unwrap();
        }
        root
    }

    /// The worker can drain input between every pair of payload reads.
    fn a_stepped_catalog_build_is_load_catalog_a_file_at_a_time() {
        let root = library_card("build-steps", 4);
        Store::<Chain>::new(root.join(MACROS_DIR))
            .save_new(&Chain::new("Chain", vec!["take-0".into()]).unwrap()).unwrap();
        let mut cache = CatalogCache::new(&root);
        cache.want(0);
        let initial = crate::model::store::file_reads();
        loop {
            let before = crate::model::store::file_reads();
            let busy = cache.step();
            assert!(crate::model::store::file_reads() - before <= 1);
            if !busy { break; }
        }
        assert_eq!(crate::model::store::file_reads() - initial, 5);
        let catalog = cache.ready(0).unwrap();
        assert_eq!(catalog.macros.len(), 5);
        assert!(catalog.macros[4].chain, "the same folder also lists chains");
        assert_eq!(catalog, &load_catalog(&root).unwrap());
    }

    /// A superseded generation remains unavailable while the changed files are merged.
    fn the_catalog_cache_serves_only_a_fresh_catalog() {
        let root = library_card("cache-fresh", 2);
        let mut cache = CatalogCache::new(&root);
        assert!(cache.ready(0).is_none(), "nothing before a build");
        cache.want(0);
        assert!(cache.ready(0).is_none(), "not before the build ends");
        while cache.step() {}
        assert_eq!(cache.ready(0), Some(&load_catalog(&root).unwrap()));
        assert!(!cache.step(), "built: no more work until the generation moves");
        // TR saves a file: generation 1. The old catalog is not served for it.
        Library::new(root.join(MACROS_DIR)).save_new(&recording("Take new", &[0, 2, 0], None)).unwrap();
        cache.want(1);
        assert!(cache.ready(1).is_none());
        assert!(cache.step());
        // Another save mid-build: generation 2 retains unchanged entries and adds the new file.
        Library::new(root.join(MACROS_DIR)).save_new(&recording("Take newer", &[0, 4, 0], None)).unwrap();
        cache.want(2);
        while cache.step() {}
        assert!(cache.ready(1).is_none(), "a superseded generation is never served");
        let fresh = cache.ready(2).expect("generation 2 built");
        assert_eq!(fresh, &load_catalog(&root).unwrap());
        assert_eq!(fresh.macros.len(), 4);
        // Asking again for the generation it has costs nothing.
        cache.want(2);
        assert!(!cache.step());
    }

    /// Files copied onto the card outside TR (FTP, DBI): the check after a menu open sees an add, a delete or a rename by
    /// names and sizes and asks for a rebuild; an unchanged card asks for none and reads no file.
    fn the_check_after_a_menu_open_sees_files_copied_outside_tr() {
        let root = library_card("outside", 2);
        let mut cache = CatalogCache::new(&root);
        cache.want(0);
        while cache.step() {}
        let reads = crate::model::store::file_reads();
        cache.check();
        let mut steps = 0;
        while cache.step() {
            steps += 1;
        }
        assert!(steps >= 4, "a folder per step: {steps}");
        assert!(!cache.take_external_change(), "an unchanged card asks for no rebuild");
        assert_eq!(crate::model::store::file_reads(), reads, "the check reads no file");
        assert!(cache.ready(0).is_some(), "the kept catalog stays");
        let macros = root.join(MACROS_DIR);
        let first = load_catalog(&root).unwrap().macros[0].id.clone();
        let mut generation = 0;
        for change in ["add", "delete", "rename"] {
            match change {
                "add" => {
                    std::fs::copy(macros.join(format!("{first}.json")), macros.join("copied-in.json")).unwrap();
                }
                "delete" => std::fs::remove_file(macros.join("copied-in.json")).unwrap(),
                _ => std::fs::rename(macros.join(format!("{first}.json")), macros.join("renamed-in.json")).unwrap(),
            }
            let before = crate::model::store::file_reads();
            cache.check();
            while cache.step() {}
            assert_eq!(crate::model::store::file_reads(), before, "{change}: list without payload reads");
            assert!(cache.take_external_change(), "{change}: the check asks for a rebuild");
            assert!(!cache.take_external_change(), "{change}: once");
            // The worker moves the catalog generation; the rebuild is the card as it is now.
            generation += 1;
            cache.want(generation);
            while cache.step() {}
            assert_eq!(crate::model::store::file_reads() - before, usize::from(change != "delete"), "{change}");
            assert_eq!(cache.ready(generation), Some(&load_catalog(&root).unwrap()), "{change}");
            cache.check();
            while cache.step() {}
            assert!(!cache.take_external_change(), "{change}: the rebuilt catalog's own folders are not a change");
        }
        assert!(cache.ready(generation).unwrap().macros.iter().any(|file| file.id == "renamed-in"));
    }


    fn save_chord_preserves_a_slot_written_after_its_initial_index_read() {
        use crate::model::store::before_next_store_list;

        let root = library_card("save-chord-slot-race", 1);
        let path = root.clone();
        before_next_store_list(move || {
            let index = RecordingIndex::load(&path).unwrap();
            assert_eq!(index.bound(), None, "the save has not committed");
            let mut screens = RecordingScreens::new();
            apply_index(&index, &mut screens);
            screens.adopt_slots([Some("take-0".into()), None, None, None, None]);
            super::super::rr_host::publish_selection(&path, &screens.selection(), None);
            assert_eq!(RecordingIndex::load(&path).unwrap().slots()[0].as_deref(), Some("take-0"));
        });

        let id = save_chord(&root, state(8), state(7), 240).unwrap();
        let index = RecordingIndex::load(&root).unwrap();
        assert_eq!(index.bound(), Some(id.as_str()));
        assert!(Store::<NamedSave>::new(root.join(SAVES_DIR)).load(&id).is_ok());
        assert_eq!(index.slots()[0].as_deref(), Some("take-0"), "the accepted slot must survive the Save chord");
    }

    fn discovery_yields_between_small_batches_and_keeps_menu_input_available() {
        use crate::model::menu::table::Page;
        use crate::model::menu::{Env, Menu};
        use crate::model::settings::Settings;
        use crate::model::store::catalog_metadata_reads;

        let root = library_card("bounded-discovery", 60);
        let mut cache = CatalogCache::new(&root);
        cache.want(0);
        while cache.step() {}
        let mut menu = Menu::new(Settings::factory(), Env::default());
        while menu.page() != Page::Recording { menu.press(MenuButton::R); }
        let index = RecordingIndex::load(&root).unwrap();
        apply_catalog(cache.ready(0).unwrap(), &index, menu.recording_mut());
        let before = catalog_metadata_reads();
        cache.check();
        assert!(cache.step(), "open the first discovery folder");
        assert!(catalog_metadata_reads() - before <= 4, "one input drain cannot wait behind an entire folder");
        assert!(cache.step(), "read the first discovery batch");
        assert!(catalog_metadata_reads() - before <= 4, "one batch stats at most four entries");
        assert!(cache.ready(0).is_some(), "an unchanged open continues with the current catalog");
        menu.press(MenuButton::R);
        assert_ne!(menu.page(), Page::Recording, "page navigation remains available during the scan");

        let mut steps = 2;
        loop {
            let last = catalog_metadata_reads();
            let work = cache.step();
            assert!(catalog_metadata_reads() - last <= 4, "metadata calls per drain remain bounded");
            if !work { break; }
            steps += 1;
            assert!(steps <= 80);
        }
        assert_eq!(catalog_metadata_reads() - before, 60);
        assert!(steps >= 15);
        assert!(!cache.take_external_change(), "the check found the same names and sizes");
    }

    fn a_failed_payload_retries_without_a_signature_change() {
        use crate::model::store::{fail_next_payload_read, file_reads};

        let root = library_card("payload-retry", 1);
        let entry = root.join(MACROS_DIR).join("take-0.json");
        let contents = std::fs::read(&entry).unwrap();
        // A second bad file has a stable name and size but is permanently invalid.
        std::fs::write(root.join(MACROS_DIR).join("zzz-bad.json"), b"not json").unwrap();
        let mut cache = CatalogCache::new(&root);
        cache.want(0);
        fail_next_payload_read();
        for _ in 0..20 {
            if !cache.step() { break; }
        }
        assert!(cache.ready(0).is_some(), "payload failures must not stall the catalog");
        let before = file_reads();

        cache.check();
        for _ in 0..20 {
            let step = cache.step();
            if cache.take_external_change() {
                cache.want(1);
            } else if !step {
                break;
            }
        }
        let catalog = cache.ready(1).or_else(|| cache.ready(0)).expect("retry completed");
        assert_eq!(catalog.macros.iter().map(|file| file.id.as_str()).collect::<Vec<_>>(), ["take-0"]);
        assert_eq!(file_reads() - before, 1, "retry the I/O failure only");
        assert_eq!(std::fs::read(entry).unwrap(), contents, "the same file recovered");
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_failed_drill_save_deletes_its_take", a_failed_drill_save_deletes_its_take as fn()),
            ("an_index_failure_after_the_drill_file_keeps_the_pair", an_index_failure_after_the_drill_file_keeps_the_pair as fn()),
            ("a_drill_file_from_another_card_neither_plays_nor_deletes_the_local_take", a_drill_file_from_another_card_neither_plays_nor_deletes_the_local_take as fn()),
            ("a_take_two_drill_files_name_survives_the_first_delete", a_take_two_drill_files_name_survives_the_first_delete as fn()),
            ("a_skipped_combo_file_does_not_keep_a_deleted_drills_take", a_skipped_combo_file_does_not_keep_a_deleted_drills_take as fn()),
            ("a_combo_folder_that_cannot_be_read_refuses_the_delete_and_keeps_the_pair", a_combo_folder_that_cannot_be_read_refuses_the_delete_and_keeps_the_pair as fn()),
            ("save_chord_preserves_a_slot_written_after_its_initial_index_read", save_chord_preserves_a_slot_written_after_its_initial_index_read as fn()),
            ("discovery_yields_between_small_batches_and_keeps_menu_input_available", discovery_yields_between_small_batches_and_keeps_menu_input_available as fn()),
            ("a_failed_payload_retries_without_a_signature_change", a_failed_payload_retries_without_a_signature_change as fn()),
            ("a_slot_assignment_survives_close_reopen_and_next_press", a_slot_assignment_survives_close_reopen_and_next_press as fn()),
            ("a_superseded_catalog_cannot_open_a_file_action", a_superseded_catalog_cannot_open_a_file_action as fn()),
            ("an_index_read_failure_stays_unready_and_retries", an_index_read_failure_stays_unready_and_retries as fn()),
            ("streak_saves_do_not_restart_a_populated_catalog_build", streak_saves_do_not_restart_a_populated_catalog_build as fn()),
            ("the_check_after_a_menu_open_sees_files_copied_outside_tr", the_check_after_a_menu_open_sees_files_copied_outside_tr as fn()),
            ("a_stepped_catalog_build_is_load_catalog_a_file_at_a_time", a_stepped_catalog_build_is_load_catalog_a_file_at_a_time as fn()),
            ("the_catalog_cache_serves_only_a_fresh_catalog", the_catalog_cache_serves_only_a_fresh_catalog as fn()),
            ("record_follows_the_popup_role", record_follows_the_popup_role as fn()),
            ("a_long_macro_plays_from_a_600_frame_window", a_long_macro_plays_from_a_600_frame_window as fn()),
            ("the_play_chords_drive_the_role_machines", the_play_chords_drive_the_role_machines as fn()),
            ("a_chain_asks_fighter_mismatch_for_macro_1_only", a_chain_asks_fighter_mismatch_for_macro_1_only as fn()),
            ("rename_uses_the_switch_keyboard_headers", rename_uses_the_switch_keyboard_headers as fn()),
            ("an_online_room_opens_every_name_keyboard", an_online_room_opens_every_name_keyboard as fn()),
            ("a_saved_crop_of_the_roles_active_macro_becomes_active", a_saved_crop_of_the_roles_active_macro_becomes_active as fn()),
            ("file_clip_save_as_new_preserves_original", file_clip_save_as_new_preserves_original as fn()),
            ("file_clip_empty_name_uses_unused_number", file_clip_empty_name_uses_unused_number as fn()),
            ("drill_units_share_clip_editor_and_save_same_kind", drill_units_share_clip_editor_and_save_same_kind as fn()),
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
