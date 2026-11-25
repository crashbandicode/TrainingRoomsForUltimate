//! The recording pop-up, the Recording page's library, and each file's screen.
//!
//! The host fills [`Catalog`] and reads [`Selection`]. Playback, the Switch keyboard, crop, and
//! Fighter Mismatch stay behind [`Effect`]. This module does not import `engine` (a by-path test of
//! `model.rs` has no engine module) and it does not touch `recording.rs` or `playback.rs`.

use super::combo::Combo;
use super::combo_record::is_drill_take_name;
use super::menu::input::Button;
use super::menu::table::{COMBOS_SEQ, SAVES_MACROS};
use super::menu::{jumped, jumped_within, stepped};
use super::numbered_picker::{self, CatalogTile, NumberKey, TileKind};
use super::ordered_ids;

const SAVE_ON: [u8; 4] = [242, 152, 72, 255];
const SAVE_OFF: [u8; 4] = [126, 132, 144, 255];

/// Player 1 or the CPU. The pop-up's role is who Record captures. The page has its own role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewRole {
    Player,
    Cpu,
}

impl ViewRole {
    pub const fn other(self) -> ViewRole {
        match self {
            ViewRole::Player => ViewRole::Cpu,
            ViewRole::Cpu => ViewRole::Player,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            ViewRole::Player => "Player 1",
            ViewRole::Cpu => "CPU",
        }
    }

    pub const fn diag(self) -> &'static str {
        match self {
            ViewRole::Player => "player",
            ViewRole::Cpu => "cpu",
        }
    }

    fn index(self) -> usize {
        match self {
            ViewRole::Player => 0,
            ViewRole::Cpu => 1,
        }
    }
}

/// Which screen a cursor belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Popup,
    Page,
}

/// What the Recording page should do with a button the library did not take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageHit {
    Consumed,
    Close,
    Passthrough,
    /// The page took the button but it does nothing there (the bridge state's `ack.result=ignored`).
    Ignored,
}

/// Loop counts X cycles: ∞, 1, 2, 3, 5, 10.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopMark {
    Infinite,
    Finite(u8),
}

impl Default for LoopMark {
    fn default() -> Self {
        LoopMark::Finite(1)
    }
}

impl LoopMark {
    pub const CYCLE: [LoopMark; 6] = [
        LoopMark::Infinite,
        LoopMark::Finite(1),
        LoopMark::Finite(2),
        LoopMark::Finite(3),
        LoopMark::Finite(5),
        LoopMark::Finite(10),
    ];

    pub const fn label(self) -> &'static str {
        match self {
            LoopMark::Infinite => "∞",
            LoopMark::Finite(1) => "1",
            LoopMark::Finite(2) => "2",
            LoopMark::Finite(3) => "3",
            LoopMark::Finite(5) => "5",
            LoopMark::Finite(10) => "10",
            LoopMark::Finite(_) => "1",
        }
    }

    fn next(self) -> LoopMark {
        let at = Self::CYCLE.iter().position(|mark| *mark == self).unwrap_or(1);
        Self::CYCLE[(at + 1) % Self::CYCLE.len()]
    }
}

/// The floppy chip. On is orange. Off is grey. The header is always On.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Floppy {
    On,
    Off,
}

impl Floppy {
    pub const fn color(self) -> [u8; 4] {
        match self {
            Floppy::On => SAVE_ON,
            Floppy::Off => SAVE_OFF,
        }
    }
}

/// What the host does after the menu. The menu does not play, crop, or open the keyboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    RecordDrill,
    Keyboard { header: &'static str, draft: String },
    MakeCombo { source: String, chain: bool },
    OpenCrop { source: String, combo: bool, player: bool },
    /// X on a drill that kept its take (F10): the host previews frames `0..frames` of macro `source` on Player 1.
    PreviewDrill { source: String, frames: u32 },
    DuplicateSave { source: String },
    DeleteSave { source: String },
    /// A chain the menu just named. The host writes it; the local row is what the menu test sees first.
    WriteChain { name: String, steps: Vec<String> },
    /// A sequence the menu just named. The host writes it.
    WriteSequence { name: String, combos: Vec<String> },
    RenameSave { id: String, name: String },
    RenameMacro { id: String, name: String },
    RenameChain { id: String, name: String },
    RenameCombo { id: String, name: String },
    RenameSequence { id: String, name: String },
    /// Delete on a macro, a chain, a combo or a sequence. The host removes the file.
    DeleteFile { kind: FileKind, id: String },
    /// Edit chain finished. The host writes the chain's new order.
    WriteChainSteps { id: String, steps: Vec<String> },
    /// Edit combos finished. The host writes the sequence's new order.
    WriteSequenceCombos { id: String, combos: Vec<String> },
    /// *+ Save last combo…*: the host saves the strip's latest exchange under `name` (empty: Combo N) and drills it.
    SaveLastCombo { name: String },
    /// Start save flipped on a macro's or a chain's screen. The host writes the switch into the file (Play reads it there).
    SetStartSave { id: String, chain: bool, on: bool },
}

/// The file kinds Delete removes through [`Effect::DeleteFile`]. A save state has its own effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Macro,
    Chain,
    Combo,
    Sequence,
}

/// One named save in the snapshot the host loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveFile {
    pub id: String,
    pub name: String,
    pub fighters: String,
}

/// A recording or a chain. A chain's start save is macro 1's; `start_save` on a chain is ignored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroFile {
    pub id: String,
    pub name: String,
    pub chain: bool,
    pub steps: Vec<String>,
    pub start_save: Option<String>,
    pub start_save_on: bool,
    pub recorded_on: String,
    /// Frames in a macro, or links in a combo's sibling. A chain's crop is not offered.
    pub span: u32,
}

/// A combo file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboFile {
    pub input_only: bool,
    pub id: String,
    pub name: String,
    /// The fighter the file says it was recorded on (empty: not recorded).
    pub recorded_on: String,
    pub made_from: Option<String>,
    /// The macro file that holds this directly recorded drill's owned take.
    pub take: Option<String>,
    pub combo: Box<Combo>,
    pub fighter: Option<super::move_names::FighterId>,
    /// Real links in the combo file. The crop handles step this, not the display lines.
    pub span: u32,
}

/// A sequence file. Combo ids, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceFile {
    pub id: String,
    pub name: String,
    pub combos: Vec<String>,
    pub streak: u32,
    pub best: u32,
}

/// The files and the hot lists. Hot ids are most recently used first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalog {
    pub saves: Vec<SaveFile>,
    pub macros: Vec<MacroFile>,
    pub combos: Vec<ComboFile>,
    pub sequences: Vec<SequenceFile>,
    pub hot_saves: Vec<String>,
    pub hot_macros: Vec<String>,
    pub hot_combos: Vec<String>,
    pub hot_sequences: Vec<String>,
}

/// What playback and record read. The menu updates it. The host stores it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub bound: Option<String>,
    pub active_player: Option<String>,
    pub active_cpu: Option<String>,
    pub loop_player: LoopMark,
    pub loop_cpu: LoopMark,
    pub drill: Option<String>,
    pub record_role: ViewRole,
    /// rc.12 TR1: the macro each Playback Slot 1-5 plays.
    pub slots: [Option<String>; 5],
}

/// One library row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowMark {
    pub name: String,
    pub green: bool,
    pub cursor: bool,
    pub hot: bool,
    pub floppy: Option<Floppy>,
    pub order: Option<u8>,
    pub chain_count: Option<u8>,
    /// The r8 mockup's line under the name (`RECORDING-REDESIGN-r8-popup-*.png`): a save's fighters, a macro's or a
    /// combo's "Recorded on …", a chain's or a sequence's steps. Empty for an action row.
    pub sub: String,
}

/// One section, header included.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionView {
    pub header: &'static str,
    pub header_floppy: Option<Floppy>,
    pub header_loop: Option<LoopMark>,
    pub hairline_after: Option<usize>,
    pub rows: Vec<RowMark>,
    pub cursor: usize,
    /// Files in the section (action rows apart), beside its header in the r8 mockup.
    pub files: usize,
    /// The card's catalog is still being read (menu-speed fix): the header says so instead of a file count.
    pub loading: bool,
}

/// One Connections line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnLine {
    pub text: String,
    pub floppy: Option<Floppy>,
}

/// The kind of library file a CONNECTIONS box names (its pill: SAVE STATE, MACRO, COMBO, SEQUENCE).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileTag {
    Drill,
    Save,
    Macro,
    Combo,
    Sequence,
}

/// One line of the r8 pop-up's CONNECTIONS column (`RECORDING-REDESIGN-r8-popup-*.png`): the row under the cursor in
/// its kind box, a heading over the files joined to it, each of those in its own box, and plain lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnKind {
    /// A colour-key chip in the shared side panel.
    Swatch([u8; 4]),
    Head(FileTag),
    Child(FileTag),
    Heading,
    #[default]
    Detail,
    /// A combo's link ("L1 jab → jab TRUE") and the line under it (its inputs and drift).
    Link,
    LinkSub,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnItem {
    pub kind: ConnKind,
    pub text: String,
}

/// One tile of the numbered picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickerRow {
    pub name: String,
    pub rank: u8,
    pub cursor: bool,
}

/// One row of a file screen. The Start save row of a macro or chain carries its floppy, the same
/// chip as that file's library row (RECORDING-REDESIGN, floppy chip, operator 2026-11-06).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRow {
    pub title: String,
    pub value: String,
    pub floppy: Option<Floppy>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ItemKind {
    Drills,
    RecordDrill,
    Save,
    Recording,
    Combo,
    Sequence,
    NewChain,
    NewSequence,
    /// *+ Save last combo…*, the first row of COMBOS (operator 2026-11-08).
    SaveLastCombo,
}

impl ItemKind {
    /// A row that does something instead of naming a file.
    const fn action(self) -> bool {
        matches!(self, ItemKind::Drills | ItemKind::RecordDrill | ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Item {
    kind: ItemKind,
    id: String,
}

#[derive(Clone, Copy, Debug)]
struct Place {
    drills: bool,
    parent_at: usize,
    section: u8,
    at: [usize; 2],
}

impl Place {
    const fn new() -> Place {
        Place { drills: false, parent_at: 0, section: 0, at: [0, 0] }
    }

    fn in_drills(self, group: u8, section: u8) -> bool {
        self.drills && group == COMBOS_SEQ && section == 1
    }
}

#[derive(Clone, Debug)]
struct FileScreen {
    id: String,
    kind: ItemKind,
    cursor: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickerKind {
    NewChain,
    NewSequence,
    EditChain,
    EditSequence,
}

#[derive(Clone, Debug)]
struct Picker {
    kind: PickerKind,
    cursor: usize,
    values: Vec<u8>,
    ids: Vec<String>,
    editing: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyKind {
    RenameSave,
    RenameMacro,
    RenameChain,
    RenameCombo,
    RenameSequence,
    NewChain,
    NewSequence,
    SaveLastCombo,
}

#[derive(Clone, Debug)]
struct Keyboard {
    kind: KeyKind,
    ids: Vec<String>,
    target: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Act {
    StartSave,
    SetActive,
    PlaybackSlot,
    Bind,
    Duplicate,
    Rename,
    Crop,
    MakeCombo,
    EditChain,
    EditCombos,
    Delete,
    Drill,
    DrillCombo,
    Info,
}

struct BuiltRow {
    title: String,
    value: String,
    act: Act,
    drill: Option<String>,
}

/// The pop-up and the Recording page.
#[derive(Clone, Debug)]
pub struct RecordingScreens {
    popup_open: bool,
    popup_role: ViewRole,
    page_role: ViewRole,
    popup_tab: u8,
    popup_at: [Place; 2],
    page_at: [Place; 2],
    file: Option<FileScreen>,
    picker: Option<Picker>,
    keyboard: Option<Keyboard>,
    overlay_face: Face,
    catalog: Catalog,
    /// Macros that drills own as their takes (F10), named with the take prefix and key, with their frame counts: taken out of the catalog's macros in
    /// `set_catalog`, so no list, picker, name or count shows them.
    takes: Vec<(String, u32)>,
    bound: Option<String>,
    active: [Option<String>; 2],
    loops: [LoopMark; 2],
    loop_edits: [Option<LoopMark>; 2],
    drill: Option<String>,
    slots: [Option<String>; 5],
    last_effect: Effect,
    selection_changed: bool,
    serial: u32,
    /// The worker's catalog is not built yet (menu-speed fix): the lists say so.
    loading: bool,
}

impl RecordingScreens {
    /// Screens open above this face's own list, one B each (the bridge snapshot's depth).
    pub fn depth(&self, face: Face, group: u8, on_library: bool) -> usize {
        let overlays = if self.overlay_face == face {
            usize::from(self.file.is_some()) + usize::from(self.picker.is_some()) + usize::from(self.keyboard.is_some())
        } else {
            0
        };
        let group = if face == Face::Popup { self.popup_group() } else { group };
        overlays + usize::from((face == Face::Popup || on_library) && self.place(face, group).drills)
    }

    /// The list the cursor rests in on `face` (the bridge snapshot): that face's open numbered picker or file screen,
    /// else, on the library (`on_library`; the pop-up always is), the current section of `group` (the pop-up uses its
    /// own group). `None`: the keyboard is up, the catalog is loading, the list is empty, or the page's cursor is not
    /// on a library group.
    pub fn focus_list(&self, face: Face, group: u8, on_library: bool) -> Option<super::menu::FocusList> {
        use super::menu::FocusList;
        let list = |id: String, count: usize, index: usize, value: String| {
            (count != 0).then(|| FocusList { id, count, index: index.min(count - 1), row: None, value })
        };
        if self.overlay_open() && self.overlay_face == face {
            if self.loading || self.keyboard.is_some() {
                return None;
            }
            if let Some(picker) = &self.picker {
                return list("recording/picker".into(), picker.ids.len(), picker.cursor, String::new());
            }
            if let Some(file) = &self.file {
                let rows = self.built_file_rows();
                let index = file.cursor.min(rows.len().saturating_sub(1));
                let value = rows.get(index).map_or_else(String::new, |row| row.value.clone());
                return list("recording/file".into(), rows.len(), index, value);
            }
            return None;
        }
        if !on_library {
            return None;
        }
        let group = if face == Face::Popup { self.popup_group() } else { group };
        let place = self.place(face, group);
        let section = place.section;
        let count = self.items(face, group, section).len();
        let name = if face == Face::Popup { "popup" } else { "page" };
        let suffix = if place.in_drills(group, section) { "/drills" } else { "" };
        list(format!("recording/{name}/{group}/{section}{suffix}"), count, place.at[usize::from(section)], String::new())
    }

    pub(crate) fn new() -> RecordingScreens {
        RecordingScreens {
            popup_open: false,
            popup_role: ViewRole::Player,
            page_role: ViewRole::Player,
            popup_tab: 0,
            popup_at: [Place::new(), Place::new()],
            page_at: [Place::new(), Place::new()],
            file: None,
            picker: None,
            keyboard: None,
            overlay_face: Face::Page,
            catalog: Catalog::default(),
            takes: Vec::new(),
            bound: None,
            active: [None, None],
            loops: [LoopMark::default(), LoopMark::default()],
            loop_edits: [None; 2],
            drill: None,
            slots: Default::default(),
            last_effect: Effect::None,
            selection_changed: false,
            serial: 0,
            loading: false,
        }
    }

    pub fn popup_open(&self) -> bool {
        self.popup_open
    }

    pub fn toggle_popup(&mut self) {
        self.popup_open = !self.popup_open;
        if !self.popup_open && self.overlay_face == Face::Popup {
            self.picker = None;
            self.keyboard = None;
        }
    }

    pub fn role(&self, face: Face) -> ViewRole {
        match face {
            Face::Popup => self.popup_role,
            Face::Page => self.page_role,
        }
    }

    pub fn page_role(&self) -> ViewRole {
        self.page_role
    }

    pub fn roles(&self) -> [ViewRole; 2] {
        [self.popup_role, self.page_role]
    }

    pub fn set_roles(&mut self, [popup, page]: [ViewRole; 2]) {
        self.set_popup_role(popup);
        self.page_role = page;
    }

    /// A file, picker or keyboard opened from the page owns its buttons; ZR does not change the role.
    pub fn page_overlay_open(&self) -> bool {
        self.overlay_open() && self.overlay_face == Face::Page
    }

    pub fn set_catalog(&mut self, mut catalog: Catalog) {
        self.takes.clear();
        catalog.macros.retain(|file| {
            if is_drill_take_name(&file.name) && catalog.combos.iter().any(|combo| combo.take.as_deref() == Some(file.id.as_str())) {
                self.takes.push((file.id.clone(), file.span));
                false
            } else {
                true
            }
        });
        self.catalog = catalog;
        self.loading = false;
        if self.file.as_ref().is_some_and(|file| !self.has_file(&file.id, file.kind)) {
            self.file = None;
        }
    }

    /// The menu worker has no catalog for the card yet (menu-speed fix): the lists say "loading" until `set_catalog`.
    pub fn set_loading(&mut self) {
        self.loading = true;
        self.picker = None;
        self.keyboard = None;
        self.last_effect = Effect::None;
        self.selection_changed = false;
    }

    pub fn take_selection_change(&mut self) -> bool {
        core::mem::take(&mut self.selection_changed)
    }

    fn has_file(&self, id: &str, kind: ItemKind) -> bool {
        match kind {
            ItemKind::Recording => self.macro_by_id(id).is_some(),
            ItemKind::Save => self.save_by_id(id).is_some(),
            ItemKind::Combo => self.combo_by_id(id).is_some(),
            ItemKind::Sequence => self.catalog.sequences.iter().any(|file| file.id == id),
            _ => false,
        }
    }

    pub fn loading(&self) -> bool {
        self.loading
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn selection(&self) -> Selection {
        Selection {
            bound: self.bound.clone(),
            active_player: self.active[0].clone(),
            active_cpu: self.active[1].clone(),
            loop_player: self.loops[0],
            loop_cpu: self.loops[1],
            drill: self.drill.clone(),
            record_role: self.popup_role,
            slots: self.slots.clone(),
        }
    }

    #[cfg(test)]
    pub fn last_effect(&self) -> &Effect {
        &self.last_effect
    }

    /// The host takes the effect once. A second read is [`Effect::None`].
    pub fn take_effect(&mut self) -> Effect {
        std::mem::replace(&mut self.last_effect, Effect::None)
    }

    /// The page's keyboard is up (a name is being asked for).
    pub fn naming(&self) -> bool { self.keyboard.is_some() }

    /// The keyboard was cancelled: nothing is made or renamed.
    pub fn cancel_name(&mut self) -> bool { self.keyboard.take().is_some() }

    pub fn loop_of(&self, role: ViewRole) -> LoopMark {
        self.loops[role.index()]
    }

    pub fn loop_label(&self, role: ViewRole) -> &'static str {
        self.loop_of(role).label()
    }

    /// The Options row's Left / Right. X does not call this.
    pub fn cycle_loop_row(&mut self, forward: bool) {
        let (at, len) = (self.loop_row_index(), LoopMark::CYCLE.len());
        self.set_loop_row(if forward { (at + 1) % len } else { (at + len - 1) % len });
    }

    /// Where the page role's count is in `LoopMark::CYCLE`, the order of the row's list (`LOOP_COUNTS`).
    pub fn loop_row_index(&self) -> usize {
        LoopMark::CYCLE.iter().position(|mark| *mark == self.loops[self.page_role.index()]).unwrap_or(1)
    }

    /// The row's list pick (and Left / Right): the page role's count becomes `LoopMark::CYCLE[index]`.
    pub fn set_loop_row(&mut self, index: usize) {
        let next = LoopMark::CYCLE.get(index).copied().unwrap_or_default();
        let role = self.page_role.index();
        if self.loops[role] != next {
            self.loops[role] = next;
            self.loop_edits[role] = Some(next);
        }
    }

    /// The row's reset (Reset page to defaults): both roles' counts go back to 1, the count of a new index. True when one changed.
    pub fn reset_loops(&mut self) -> bool {
        let was = core::mem::take(&mut self.loops);
        let changed = was != self.loops;
        if changed {
            self.loop_edits = self.loops.map(Some);
        }
        changed
    }

    pub fn persist_loop_edits(&mut self, publish: impl FnOnce([Option<LoopMark>; 2]) -> bool) {
        if self.loop_edits != [None; 2] && publish(self.loop_edits) {
            self.loop_edits = [None; 2];
        }
    }

    pub fn tab_names(&self) -> &'static [&'static str] {
        if self.popup_role == ViewRole::Cpu {
            &["Save states & Macros"]
        } else {
            &["Save states & Macros", "Combos & Sequences"]
        }
    }

    pub fn tab(&self) -> usize {
        if self.popup_role == ViewRole::Cpu { 0 } else { usize::from(self.popup_tab) }
    }

    pub fn file_open(&self) -> bool {
        self.file.is_some()
    }

    pub fn picker_open(&self) -> bool {
        self.picker.is_some()
    }

    pub fn file_cursor(&self) -> usize {
        self.file.as_ref().map(|file| file.cursor).unwrap_or(0)
    }

    /// The numbered picker. `rank` 0 is unnumbered. `values[0]` is the All flag and is not a row.
    pub fn picker_rows(&self) -> Vec<PickerRow> {
        let Some(picker) = &self.picker else { return Vec::new() };
        picker.ids.iter().enumerate().map(|(index, id)| PickerRow {
            name: self.name_of_id(id),
            rank: picker.values.get(index + 1).copied().unwrap_or(0),
            cursor: index == picker.cursor,
        }).collect()
    }

    fn name_of_id(&self, id: &str) -> String {
        for kind in [ItemKind::Save, ItemKind::Recording, ItemKind::Combo, ItemKind::Sequence] {
            if let Some(name) = self.slot_name(kind, id) {
                return name;
            }
        }
        id.to_string()
    }

    pub fn file_rows(&self) -> Vec<FileRow> {
        if self.loading {
            return vec![FileRow { floppy: None, title: "Loading…".to_owned(), value: String::new() }];
        }
        let floppy = self.file.as_ref().and_then(|file| self.row_floppy(&Item { kind: file.kind, id: file.id.clone() }));
        self.built_file_rows()
            .into_iter()
            .map(|row| FileRow { floppy: floppy.filter(|_| row.act == Act::StartSave), title: row.title, value: row.value })
            .collect()
    }

    pub fn file_crumb(&self) -> String {
        if self.loading {
            return "Recording".to_owned();
        }
        let Some(file) = &self.file else { return String::new() };
        let role = if self.page_role == ViewRole::Cpu { "CPU view" } else { "Player view" };
        let group = match file.kind {
            ItemKind::Combo if self.combo_by_id(&file.id).is_some_and(|f| f.input_only) => "Combos & Sequences  >  Drills",
            ItemKind::Combo | ItemKind::Sequence => "Combos & Sequences",
            _ => "Save states & Macros",
        };
        format!("Recording ({role})  >  {group}  >  {}", self.item_name(&Item { kind: file.kind, id: file.id.clone() }))
    }

    pub fn section(&self, face: Face, group: u8, section: u8) -> SectionView {
        self.section_view(face, group, section)
    }

    #[cfg(test)]
    pub fn section_cursor(&self, face: Face, group: u8, section: u8) -> usize {
        let place = self.place(face, group);
        place.at[usize::from(section)]
    }

    pub fn current_section(&self, face: Face, group: u8) -> u8 {
        self.place(face, group).section
    }

    pub fn connections(&self, face: Face, group: u8) -> Vec<ConnLine> {
        let Some(item) = self.focused(face, group) else {
            return Vec::new();
        };
        self.connection_lines(face, &item)
    }

    /// The r8 pop-up's CONNECTIONS of the row under the cursor, as boxes, headings and lines. The page keeps
    /// [`Self::connections`] in its help lines.
    pub fn connection_items(&self, face: Face, group: u8) -> Vec<ConnItem> {
        let Some(item) = self.focused(face, group) else {
            return Vec::new();
        };
        let line = |kind, text: String| ConnItem { kind, text };
        let mut out = Vec::new();
        match item.kind {
            ItemKind::Save => {
                let Some(save) = self.save_by_id(&item.id) else { return out };
                out.push(line(ConnKind::Head(FileTag::Save), save.name.clone()));
                let users: Vec<&MacroFile> = self
                    .catalog
                    .macros
                    .iter()
                    .filter(|file| {
                        if file.chain {
                            self.chain_start_id(file) == Some(item.id.as_str())
                        } else {
                            file.start_save.as_deref() == Some(item.id.as_str())
                        }
                    })
                    .collect();
                if !users.is_empty() {
                    out.push(line(ConnKind::Heading, "start save of".to_string()));
                    out.extend(users.iter().map(|file| line(ConnKind::Child(FileTag::Macro), file.name.clone())));
                }
                let bound = if self.bound.as_deref() == Some(item.id.as_str()) { "yes" } else { "no" };
                out.push(line(ConnKind::Detail, format!("Bound to Save / Load  {bound}")));
            }
            ItemKind::Recording => {
                let Some(file) = self.macro_by_id(&item.id) else { return out };
                out.push(line(ConnKind::Head(FileTag::Macro), file.name.clone()));
                let start = if file.chain { self.chain_start_id(file) } else { file.start_save.as_deref() };
                if let Some(name) = start.and_then(|id| self.save_name(id)) {
                    out.push(line(ConnKind::Heading, "starts from (warps there once, first)".to_string()));
                    let on = if file.start_save_on { "On" } else { "Off" };
                    out.push(line(ConnKind::Child(FileTag::Save), format!("{name} · {on}")));
                }
                if file.chain {
                    out.push(line(ConnKind::Heading, "macros in order".to_string()));
                    for (index, step) in file.steps.iter().enumerate() {
                        let name = self.macro_by_id(step).map_or(step.as_str(), |step| step.name.as_str());
                        out.push(line(ConnKind::Child(FileTag::Macro), format!("{} {name}", index + 1)));
                    }
                } else {
                    let chains: Vec<(String, usize)> = self
                        .catalog
                        .macros
                        .iter()
                        .filter(|chain| chain.chain)
                        .filter_map(|chain| chain.steps.iter().position(|step| *step == item.id).map(|at| (chain.name.clone(), at + 1)))
                        .collect();
                    if !chains.is_empty() {
                        out.push(line(ConnKind::Heading, "in chains".to_string()));
                        out.extend(chains.into_iter().map(|(name, at)| line(ConnKind::Child(FileTag::Macro), format!("{name} as {at}"))));
                    }
                }
                let combos: Vec<&ComboFile> =
                    self.catalog.combos.iter().filter(|combo| combo.made_from.as_deref() == Some(item.id.as_str())).collect();
                if !combos.is_empty() {
                    out.push(line(ConnKind::Heading, "combos made from it".to_string()));
                    out.extend(combos.iter().map(|combo| line(ConnKind::Child(if combo.input_only { FileTag::Drill } else { FileTag::Combo }), combo.name.clone())));
                }
                if !file.chain && !file.recorded_on.is_empty() {
                    let seconds = file.span as f32 / 60.0;
                    out.push(line(ConnKind::Detail, format!("Recorded on  {} · {seconds:.1} s · information only", file.recorded_on)));
                }
            }
            ItemKind::Combo => {
                let Some(combo) = self.combo_by_id(&item.id) else { return out };
                out.push(line(ConnKind::Head(if combo.input_only { FileTag::Drill } else { FileTag::Combo }), combo.name.clone()));
                if let Some(source) = combo.made_from.as_deref().and_then(|id| self.macro_by_id(id)) {
                    out.push(line(ConnKind::Heading, "made from".to_string()));
                    out.push(line(ConnKind::Child(FileTag::Macro), source.name.clone()));
                }
                let sequences: Vec<String> = self
                    .catalog
                    .sequences
                    .iter()
                    .filter_map(|sequence| {
                        sequence.combos.iter().position(|step| *step == item.id).map(|at| format!("{} #{}", sequence.name, at + 1))
                    })
                    .collect();
                if !sequences.is_empty() {
                    out.push(line(ConnKind::Heading, "in sequences".to_string()));
                    out.extend(sequences.into_iter().map(|text| line(ConnKind::Child(FileTag::Sequence), text)));
                }
                let links = combo.span;
                let units = if combo.input_only { "steps" } else { "links" };
                let lines = combo_link_lines(&combo.combo, super::move_names::Namer::recorded(combo.fighter));
                if !lines.is_empty() {
                    out.push(line(ConnKind::Heading, format!("Its {units} ({links})")));
                    out.extend(lines.into_iter().map(|text| {
                        line(if link_head(&text) { ConnKind::Link } else { ConnKind::LinkSub }, text)
                    }));
                }
                if !combo.recorded_on.is_empty() {
                    out.push(line(ConnKind::Detail, format!("Fighters  {}", combo.recorded_on)));
                }
            }
            ItemKind::Sequence => {
                let Some(sequence) = self.sequence_by_id(&item.id) else { return out };
                out.push(line(ConnKind::Head(FileTag::Sequence), sequence.name.clone()));
                if !sequence.combos.is_empty() {
                    out.push(line(ConnKind::Heading, "combos".to_string()));
                    for (index, step) in sequence.combos.iter().enumerate() {
                        let name = self.combo_by_id(step).map_or(step.as_str(), |combo| combo.name.as_str());
                        out.push(line(ConnKind::Child(FileTag::Combo), format!("{} {name}", index + 1)));
                    }
                }
                out.push(line(ConnKind::Detail, format!("streak {} · best {}", sequence.streak, sequence.best)));
            }
            _ => out.extend(self.connection_lines(face, &item).into_iter().map(|conn| line(ConnKind::Detail, conn.text))),
        }
        out
    }

    #[cfg(test)]
    pub fn set_cursor(&mut self, face: Face, group: u8, section: u8, index: usize) {
        let place = self.place_mut(face, group);
        let section = section.min(1);
        place.section = section;
        place.at[usize::from(section)] = index;
    }

    #[cfg(test)]
    pub fn set_file_cursor(&mut self, cursor: usize) {
        if let Some(file) = self.file.as_mut() {
            file.cursor = cursor;
        }
    }

    #[cfg(test)]
    pub fn set_combo(&mut self, id: &str, combo: Combo) {
        if let Some(file) = self.catalog.combos.iter_mut().find(|file| file.id == id) {
            *file.combo = combo;
        }
    }

    #[cfg(test)]
    pub fn has_chain_named(&self, name: &str) -> bool {
        self.catalog.macros.iter().any(|file| file.chain && file.name == name)
    }

    #[cfg(test)]
    pub fn picker_recording_ids(&self) -> Vec<String> {
        self.recording_ids()
    }

    #[cfg(test)]
    pub fn start_save_id(&self, id: &str) -> Option<&str> {
        self.macro_by_id(id).and_then(|file| file.start_save.as_deref())
    }

    /// Footer for a face without a focused page-library group.
    pub fn hint(&self, face: Face) -> &'static str {
        self.hint_at(face, None)
    }

    /// Footer for a face and its focused page-library group, including a drill's available preview.
    pub fn hint_at(&self, face: Face, page_group: Option<u8>) -> &'static str {
        if self.keyboard.is_some() {
            return "Name it, then confirm. Empty keeps the N name. B cancels.";
        }
        if self.picker.is_some() {
            return "A add number   R remove   X done   B back";
        }
        if self.file.is_some() && face == Face::Page {
            return "Up/Down move   Left/Right change   A do it   B back";
        }
        if face == Face::Popup {
            // The r8 footer: what A does in the section under the cursor, then the pop-up's keys. The CPU has one tab.
            let group = if self.tab() == 0 { SAVES_MACROS } else { COMBOS_SEQ };
            let cpu = self.popup_role == ViewRole::Cpu;
            match (group, self.current_section(face, group), cpu) {
                (SAVES_MACROS, 0, false) => "A bind   Left/Right section   ZL/L/R tab   ZR role   ZL+Up/Down page   B close",
                (SAVES_MACROS, 0, true) => "A bind   Left/Right section   ZR role   ZL+Up/Down page   B close",
                (SAVES_MACROS, _, false) => {
                    "A set active   X loop   Left/Right section   ZL/L/R tab   ZR role   ZL+Up/Down page   B close"
                }
                (SAVES_MACROS, _, true) => "A set active   X loop   Left/Right section   ZR role   ZL+Up/Down page   B close",
                _ if self.drill_take(face, group).is_some() =>
                    "A drill   X preview   Left/Right section   ZL/L/R tab   ZR role   ZL+Up/Down page   B close",
                _ => "A drill   Left/Right section   ZL/L/R tab   ZR role   ZL+Up/Down page   B close",
            }
        } else if page_group.is_some_and(|group| self.drill_take(Face::Page, group).is_some()) {
            "A open   X preview   ZR role   L/R page   B close"
        } else {
            "A open   X loop in MACROS   ZL+Up/Down section   ZR role   L/R page   B close"
        }
    }


    /// Adopt the recording index's current selection independently of the file catalog.
    pub fn adopt(&mut self, bound: Option<String>, active: [Option<String>; 2], loops: [LoopMark; 2], drill: Option<String>) {
        self.bound = bound;
        self.active = active;
        for (role, mark) in loops.into_iter().enumerate() {
            self.loops[role] = self.loop_edits[role].unwrap_or(mark);
        }
        self.drill = drill;
    }

    /// Adopt the current Playback Slots from the index.
    pub fn adopt_slots(&mut self, slots: [Option<String>; 5]) {
        self.slots = slots;
    }

    /// The Playback Slot (0-4) that plays `id`, if any.
    fn slot_of(&self, id: &str) -> Option<usize> {
        self.slots.iter().position(|held| held.as_deref() == Some(id))
    }

    /// Left/Right (and A) on a macro's Playback slot row: None, 1 .. 5 and round. The slot a macro moves to is taken from
    /// the macro that held it.
    fn step_slot(&mut self, id: &str, forward: bool) {
        self.selection_changed = true;
        let now = self.slot_of(id).map_or(0, |i| i + 1);
        let next = if forward { (now + 1) % 6 } else { (now + 5) % 6 };
        self.slots.iter_mut().filter(|held| held.as_deref() == Some(id)).for_each(|held| *held = None);
        if next > 0 {
            self.slots[next - 1] = Some(id.to_string());
        }
    }

    pub fn focused_file_title(&self) -> Option<String> {
        let cursor = self.file.as_ref()?.cursor;
        self.built_file_rows().get(cursor).map(|row| row.title.clone())
    }

    pub fn diag_line(&self, face: Face, page_group: u8) -> String {
        let role = self.role(face);
        let group = match face {
            Face::Popup => self.popup_group(),
            Face::Page => page_group,
        };
        let view = self.section(face, group, self.current_section(face, group));
        let hot = view.rows.iter().filter(|row| row.hot).count().min(5);
        let hair = view.hairline_after.unwrap_or(0);
        let floppy = match view.rows.iter().find(|row| row.cursor).and_then(|row| row.floppy) {
            Some(Floppy::On) => "on",
            _ => "off",
        };
        let header = if view.header_floppy == Some(Floppy::On) { "on" } else { "off" };
        let active = self.active[role.index()].as_deref().unwrap_or("");
        let group_name = if group == COMBOS_SEQ { "combos-sequences" } else { "saves-macros" };
        let tab = if self.tab() == 0 { "saves" } else { "combos" };
        let screen = match face {
            Face::Popup => "recording-popup",
            Face::Page => "recording-page",
        };
        format!(
            "screen={screen} role={} tab={tab} loop={} hot={hot} hairline={hair} floppy={floppy} header={header} active={active} group={group_name}",
            role.diag(),
            self.loop_of(role).label(),
        )
    }

    /// One pop-up button. Always consumed while the pop-up is open.
    pub fn press_popup(&mut self, button: Button) {
        self.press_popup_outcome(button);
    }

    /// `press_popup`, reporting whether the press acted (false: a closed popup, Y, or X outside the macros list).
    pub fn press_popup_outcome(&mut self, button: Button) -> bool {
        if !self.popup_open {
            return false;
        }
        if self.overlay_face == Face::Popup && self.overlay_open() {
            return self.press_overlay(button, Face::Popup);
        }
        let group = self.popup_group();
        match button {
            Button::B if self.place(Face::Popup, group).drills => self.toggle_drills(Face::Popup, group),
            Button::B => self.popup_open = false,
            Button::Y => return false,
            Button::ZR => self.set_popup_role(self.popup_role.other()),
            Button::ZL | Button::L | Button::R => self.toggle_tab(),
            Button::X => {
                if self.in_macros(Face::Popup, group) {
                    self.cycle_x(self.popup_role);
                } else if !self.preview_drill(Face::Popup, group) {
                    return false;
                }
            }
            Button::Up => self.nudge_popup(-1),
            Button::Down => self.nudge_popup(1),
            Button::JumpUp => self.page_section(Face::Popup, group, false),
            Button::JumpDown => self.page_section(Face::Popup, group, true),
            Button::Left | Button::Right => self.swap_section(Face::Popup, group),
            Button::A => self.activate(Face::Popup, group),
        }
        true
    }

    /// One Recording-page button. L, R, and B pass through unless a file screen is open.
    pub fn handle_page(&mut self, button: Button, on_library: bool, group: u8) -> PageHit {
        if self.page_overlay_open() {
            return if self.press_overlay(button, Face::Page) { PageHit::Consumed } else { PageHit::Ignored };
        }
        if button == Button::B && on_library && self.place(Face::Page, group).drills { self.toggle_drills(Face::Page, group); return PageHit::Consumed; }
        match button {
            Button::X => {
                if on_library && self.in_macros(Face::Page, group) {
                    self.cycle_x(self.page_role);
                    PageHit::Consumed
                } else if on_library && group == COMBOS_SEQ && self.preview_drill(Face::Page, group) {
                    PageHit::Consumed
                } else {
                    PageHit::Ignored
                }
            }
            Button::Y | Button::ZL => PageHit::Ignored,
            Button::ZR => {
                self.page_role = self.page_role.other();
                PageHit::Consumed
            }
            Button::L | Button::R | Button::B => PageHit::Passthrough,
            _ if !on_library => PageHit::Passthrough,
            Button::Up => self.nudge_page(group, -1),
            Button::Down => self.nudge_page(group, 1),
            Button::JumpUp | Button::JumpDown => {
                let down = button == Button::JumpDown;
                let section = usize::from(self.place(Face::Page, group).section);
                let len = self.items(Face::Page, group, section as u8).len();
                match jumped_within(self.place(Face::Page, group).at[section], len, down) {
                    Some(at) => { self.place_mut(Face::Page, group).at[section] = at; PageHit::Consumed }
                    None if self.leave_section(group, down) => PageHit::Consumed,
                    None => PageHit::Passthrough,
                }
            }
            Button::Left | Button::Right => {
                self.swap_section(Face::Page, group);
                PageHit::Consumed
            }
            Button::A => {
                self.activate(Face::Page, group);
                if self.last_effect == Effect::RecordDrill { PageHit::Close } else { PageHit::Consumed }
            }
        }
    }

    /// The keyboard confirmed. Empty on a rename keeps the previous name. Empty on a new file uses N.
    pub fn submit_name(&mut self, name: &str) {
        let Some(keyboard) = self.keyboard.take() else { return };
        let trimmed = name.trim();
        match keyboard.kind {
            KeyKind::RenameSave | KeyKind::RenameMacro | KeyKind::RenameChain | KeyKind::RenameCombo | KeyKind::RenameSequence => {
                if !trimmed.is_empty() {
                    if let Some(id) = keyboard.target {
                        self.rename_id(keyboard.kind, &id, trimmed);
                    }
                }
            }
            KeyKind::NewChain => self.create_chain(&keyboard.ids, trimmed),
            KeyKind::NewSequence => self.create_sequence(&keyboard.ids, trimmed),
            KeyKind::SaveLastCombo => self.last_effect = Effect::SaveLastCombo { name: trimmed.to_string() },
        }
        if !matches!(
            self.last_effect,
            Effect::WriteChain { .. }
                | Effect::WriteSequence { .. }
                | Effect::SaveLastCombo { .. }
                | Effect::RenameSave { .. }
                | Effect::RenameMacro { .. }
                | Effect::RenameChain { .. }
                | Effect::RenameCombo { .. }
                | Effect::RenameSequence { .. }
        ) {
            self.last_effect = Effect::None;
        }
    }

    fn overlay_open(&self) -> bool {
        self.keyboard.is_some() || self.picker.is_some() || self.file.is_some()
    }

    pub(crate) fn popup_accepts_place_first(&self) -> bool {
        self.popup_open && !(self.overlay_face == Face::Popup && self.overlay_open())
    }

    fn set_popup_role(&mut self, role: ViewRole) {
        self.popup_role = role;
        if role == ViewRole::Cpu {
            self.popup_tab = 0;
        }
    }

    fn toggle_tab(&mut self) {
        if self.popup_role == ViewRole::Player {
            self.popup_tab ^= 1;
        }
    }

    fn popup_group(&self) -> u8 {
        if self.tab() == 0 { SAVES_MACROS } else { COMBOS_SEQ }
    }

    fn which(group: u8) -> usize {
        if group == COMBOS_SEQ { 1 } else { 0 }
    }

    fn place(&self, face: Face, group: u8) -> Place {
        match face {
            Face::Popup => self.popup_at[self.tab()],
            Face::Page => self.page_at[Self::which(group)],
        }
    }

    fn place_mut(&mut self, face: Face, group: u8) -> &mut Place {
        match face {
            Face::Popup => &mut self.popup_at[self.tab()],
            Face::Page => &mut self.page_at[Self::which(group)],
        }
    }

    fn in_macros(&self, face: Face, group: u8) -> bool {
        group == SAVES_MACROS && self.place(face, group).section == 1
    }

    fn cycle_x(&mut self, role: ViewRole) {
        if self.loading {
            return;
        }
        self.selection_changed = true;
        let slot = role.index();
        self.loops[slot] = self.loops[slot].next();
    }

    fn nudge_popup(&mut self, delta: isize) {
        let group = self.popup_group();
        let section = usize::from(self.place(Face::Popup, group).section);
        let len = self.items(Face::Popup, group, section as u8).len();
        if len == 0 {
            return;
        }
        let at = self.place(Face::Popup, group).at[section];
        self.place_mut(Face::Popup, group).at[section] = stepped(at, len, delta > 0);
    }

    fn nudge_page(&mut self, group: u8, delta: isize) -> PageHit {
        let section = self.place(Face::Page, group).section;
        let len = self.items(Face::Page, group, section).len();
        let at = self.place(Face::Page, group).at[usize::from(section)];
        if len == 0 {
            self.swap_section(Face::Page, group);
        } else {
            self.place_mut(Face::Page, group).at[usize::from(section)] = stepped(at, len, delta > 0);
        }
        PageHit::Consumed
    }

    /// A jump at a page section's edge: on to the other section's near end. False at the two lists' outer end.
    fn leave_section(&mut self, group: u8, down: bool) -> bool {
        let section = self.place(Face::Page, group).section;
        if down == (section == 0) {
            let at = if down { 0 } else { self.items(Face::Page, group, 0).len().saturating_sub(1) };
            let place = self.place_mut(Face::Page, group);
            (place.section, place.at[usize::from(down)]) = (u8::from(down), at);
        }
        down == (section == 0)
    }

    fn swap_section(&mut self, face: Face, group: u8) {
        let from = self.place(face, group).section;
        let to = 1 - from;
        let len = self.items(face, group, to).len();
        let kept = self.place(face, group).at[usize::from(from)];
        let index = if len == 0 { 0 } else { kept.min(len - 1) };
        let place = self.place_mut(face, group);
        place.at[usize::from(to)] = index;
        place.section = to;
    }

    fn page_section(&mut self, face: Face, group: u8, forward: bool) {
        let section = usize::from(self.place(face, group).section);
        let len = self.items(face, group, section as u8).len();
        let at = self.place(face, group).at[section];
        self.place_mut(face, group).at[section] = jumped(at, len, forward);
    }

    fn toggle_drills(&mut self, face: Face, group: u8) {
        let place = self.place_mut(face, group); place.drills = !place.drills;
        core::mem::swap(&mut place.at[1], &mut place.parent_at); place.section = 1;
    }

    fn activate(&mut self, face: Face, group: u8) {
        let Some(item) = self.focused(face, group) else { return };
        match (face, item.kind) {
            (_, ItemKind::Drills) => self.toggle_drills(face, group),
            (_, ItemKind::RecordDrill) => { self.last_effect = Effect::RecordDrill; self.popup_open = false; }
            (Face::Popup, ItemKind::Save) => self.set_bound(&item.id, true),
            (Face::Popup, ItemKind::Recording) => self.set_active(self.popup_role, &item.id, true),
            (Face::Popup, ItemKind::Combo | ItemKind::Sequence) => self.set_drill(item.kind, item.id),
            (Face::Popup, ItemKind::NewChain) => self.open_picker(PickerKind::NewChain, Face::Popup, None),
            (Face::Popup, ItemKind::NewSequence) => self.open_picker(PickerKind::NewSequence, Face::Popup, None),
            (Face::Page, ItemKind::NewChain) => self.open_picker(PickerKind::NewChain, Face::Page, None),
            (Face::Page, ItemKind::NewSequence) => self.open_picker(PickerKind::NewSequence, Face::Page, None),
            (face, ItemKind::SaveLastCombo) => {
                self.overlay_face = face;
                self.open_keyboard(KeyKind::SaveLastCombo, "Name combo", Vec::new());
            }
            (Face::Page, kind) => self.open_file(item.id, kind),
        }
    }

    fn open_file(&mut self, id: String, kind: ItemKind) {
        if kind == ItemKind::Recording && self.macro_by_id(&id).is_some_and(|file| file.chain) {
            self.close_chain_gap(&id);
        }
        self.overlay_face = Face::Page;
        self.file = Some(FileScreen { id, kind, cursor: 0 });
        self.picker = None;
        self.keyboard = None;
    }

    fn close_chain_gap(&mut self, id: &str) {
        let known: Vec<String> = self.catalog.macros.iter().filter(|file| !file.chain).map(|file| file.id.clone()).collect();
        if let Some(chain) = self.catalog.macros.iter_mut().find(|file| file.chain && file.id == id) {
            ordered_ids::retain_existing(&mut chain.steps, |step| known.iter().any(|id| id == step));
        }
    }

    fn close_sequence_gap(&mut self, id: &str) {
        let known: Vec<String> = self.catalog.combos.iter().filter(|f| !f.input_only).map(|file| file.id.clone()).collect();
        if let Some(sequence) = self.catalog.sequences.iter_mut().find(|file| file.id == id) {
            ordered_ids::retain_existing(&mut sequence.combos, |step| known.iter().any(|id| id == step));
        }
    }

    /// One press on the open overlay (loading, keyboard, picker or file screen): true when it acted.
    fn press_overlay(&mut self, button: Button, face: Face) -> bool {
        if self.loading {
            if button == Button::B {
                self.file = None;
                return true;
            }
            return false;
        }
        if self.keyboard.is_some() {
            if button == Button::B {
                self.keyboard = None;
                return true;
            }
            return false;
        }
        if self.picker.is_some() {
            return self.press_picker(button, face);
        }
        self.press_file(button)
    }

    fn press_file(&mut self, button: Button) -> bool {
        let len = self.built_file_rows().len();
        match button {
            Button::B => self.file = None,
            Button::Up => {
                if let Some(file) = self.file.as_mut() {
                    file.cursor = stepped(file.cursor, len, false);
                }
            }
            Button::Down => {
                if let Some(file) = self.file.as_mut() {
                    file.cursor = stepped(file.cursor, len, true);
                }
            }
            Button::JumpUp | Button::JumpDown => {
                if let Some(file) = self.file.as_mut() {
                    file.cursor = jumped(file.cursor, len, button == Button::JumpDown);
                }
            }
            Button::Left => self.flip_switch(false),
            Button::Right => self.flip_switch(true),
            Button::A => self.do_file_row(),
            Button::X | Button::Y | Button::L | Button::R | Button::ZL | Button::ZR => return false,
        }
        true
    }

    fn flip_switch(&mut self, on: bool) {
        let cursor = self.file.as_ref().map(|file| file.cursor);
        let id = self.file.as_ref().map(|file| file.id.clone());
        let kind = self.file.as_ref().map(|file| file.kind);
        let rows = self.built_file_rows();
        let (Some(cursor), Some(id), Some(kind)) = (cursor, id, kind) else { return };
        let Some(act) = rows.get(cursor).map(|row| row.act) else { return };
        match act {
            Act::Bind => self.set_bound(&id, on),
            Act::SetActive => self.set_active(self.page_role, &id, on),
            Act::StartSave => self.set_start(kind, &id, on),
            Act::PlaybackSlot => self.step_slot(&id, on),
            _ => {}
        }
    }

    fn do_file_row(&mut self) {
        let cursor = self.file.as_ref().map(|file| file.cursor);
        let id = self.file.as_ref().map(|file| file.id.clone());
        let kind = self.file.as_ref().map(|file| file.kind);
        let rows = self.built_file_rows();
        let (Some(cursor), Some(id), Some(kind)) = (cursor, id, kind) else { return };
        let Some(row) = rows.get(cursor) else { return };
        let act = row.act;
        let drill = row.drill.clone();
        match act {
            Act::Bind => {
                let on = self.bound.as_deref() != Some(id.as_str());
                self.set_bound(&id, on);
            }
            Act::SetActive => {
                let on = self.active[self.page_role.index()].as_deref() != Some(id.as_str());
                self.set_active(self.page_role, &id, on);
            }
            Act::StartSave => {
                let on = !self.start_is_on(kind, &id);
                self.set_start(kind, &id, on);
            }
            Act::PlaybackSlot => self.step_slot(&id, true),
            Act::Duplicate => {
                self.duplicate_save(&id);
                if kind == ItemKind::Save {
                    self.last_effect = Effect::DuplicateSave { source: id };
                }
            }
            Act::Rename => self.open_rename(kind, &id),
            Act::Crop => self.open_crop(kind, id),
            Act::MakeCombo => {
                let chain = kind == ItemKind::Recording && self.macro_by_id(&id).is_some_and(|file| file.chain);
                self.last_effect = Effect::MakeCombo { source: id, chain };
            }
            Act::EditChain => self.open_picker(PickerKind::EditChain, Face::Page, Some(id)),
            Act::EditCombos => self.open_picker(PickerKind::EditSequence, Face::Page, Some(id)),
            Act::Delete => {
                let file = match kind {
                    ItemKind::Recording if self.macro_by_id(&id).is_some_and(|file| file.chain) => Some(FileKind::Chain),
                    ItemKind::Recording => Some(FileKind::Macro),
                    ItemKind::Combo => Some(FileKind::Combo),
                    ItemKind::Sequence => Some(FileKind::Sequence),
                    _ => None,
                };
                self.delete_file(kind, &id);
                if kind == ItemKind::Save {
                    self.last_effect = Effect::DeleteSave { source: id };
                } else if let Some(kind) = file {
                    self.last_effect = Effect::DeleteFile { kind, id };
                }
            }
            Act::Drill => self.set_drill(kind, id),
            Act::DrillCombo => {
                if let Some(combo) = drill {
                    self.set_drill(ItemKind::Combo, combo);
                }
            }
            Act::Info => {}
        }
    }

    fn open_crop(&mut self, kind: ItemKind, source: String) {
        self.last_effect = Effect::OpenCrop { source, combo: kind == ItemKind::Combo, player: self.page_role == ViewRole::Player };
    }

    fn set_bound(&mut self, id: &str, on: bool) {
        self.selection_changed = true;
        if on {
            self.bound = Some(id.to_string());
            self.note_hot_save(id);
        } else if self.bound.as_deref() == Some(id) {
            self.bound = None;
        }
    }

    fn set_active(&mut self, role: ViewRole, id: &str, on: bool) {
        self.selection_changed = true;
        let slot = role.index();
        if on {
            self.active[slot] = Some(id.to_string());
            self.note_hot_macro(id);
        } else if self.active[slot].as_deref() == Some(id) {
            self.active[slot] = None;
        }
    }

    /// A drill is a use (the index notes it): the file goes first in its kind's list now, as Set active and Bind do.
    fn set_drill(&mut self, kind: ItemKind, id: String) {
        self.selection_changed = true;
        let hot = if kind == ItemKind::Sequence { &mut self.catalog.hot_sequences } else { &mut self.catalog.hot_combos };
        hot.retain(|hot| *hot != id);
        hot.insert(0, id.clone());
        self.drill = Some(id);
    }

    fn set_start(&mut self, _kind: ItemKind, id: &str, on: bool) {
        if let Some(file) = self.catalog.macros.iter_mut().find(|file| file.id == id && file.start_save_on != on) {
            file.start_save_on = on;
            self.last_effect = Effect::SetStartSave { id: id.to_string(), chain: file.chain, on };
        }
    }

    fn start_is_on(&self, _kind: ItemKind, id: &str) -> bool {
        self.macro_by_id(id).is_some_and(|file| file.start_save_on)
    }

    fn duplicate_save(&mut self, id: &str) {
        let Some(source) = self.save_by_id(id).cloned() else { return };
        let n = unused_n("Save", &self.catalog.saves.iter().map(|file| file.name.clone()).collect::<Vec<_>>());
        self.serial += 1;
        let copy_id = format!("save-{}", self.serial);
        self.catalog.saves.push(SaveFile { id: copy_id.clone(), name: format!("Save {n}"), fighters: source.fighters });
        self.note_hot_save(&copy_id);
    }

    fn delete_file(&mut self, kind: ItemKind, id: &str) {
        self.slot_remove(kind, id);
        if self.bound.as_deref() == Some(id) {
            self.bound = None;
        }
        for slot in &mut self.active {
            if slot.as_deref() == Some(id) {
                *slot = None;
            }
        }
        if self.drill.as_deref() == Some(id) {
            self.drill = None;
        }
        self.slots.iter_mut().filter(|held| held.as_deref() == Some(id)).for_each(|held| *held = None);
        self.file = None;
    }

    fn open_rename(&mut self, kind: ItemKind, id: &str) {
        let (key, header) = match kind {
            ItemKind::Save => (KeyKind::RenameSave, "Name save state"),
            ItemKind::Recording if self.macro_by_id(id).is_some_and(|file| file.chain) => (KeyKind::RenameChain, "Name chain"),
            ItemKind::Recording => (KeyKind::RenameMacro, "Name macro"),
            ItemKind::Combo => (KeyKind::RenameCombo, if self.combo_by_id(id).is_some_and(|f| f.input_only) { "Name drill" } else { "Name combo" }),
            ItemKind::Sequence => (KeyKind::RenameSequence, "Name sequence"),
            _ => return,
        };
        let previous = self.item_name(&Item { kind, id: id.to_string() });
        self.keyboard = Some(Keyboard { kind: key, ids: Vec::new(), target: Some(id.to_string()) });
        self.last_effect = Effect::Keyboard { header, draft: previous };
    }

    fn rename_id(&mut self, kind: KeyKind, id: &str, name: &str) {
        let Some(item) = item_kind_of(kind) else { return };
        self.slot_set_name(item, id, name);
        self.last_effect = match kind {
            KeyKind::RenameSave => Effect::RenameSave { id: id.to_string(), name: name.to_string() },
            KeyKind::RenameMacro => Effect::RenameMacro { id: id.to_string(), name: name.to_string() },
            KeyKind::RenameChain => Effect::RenameChain { id: id.to_string(), name: name.to_string() },
            KeyKind::RenameCombo => Effect::RenameCombo { id: id.to_string(), name: name.to_string() },
            KeyKind::RenameSequence => Effect::RenameSequence { id: id.to_string(), name: name.to_string() },
            KeyKind::NewChain | KeyKind::NewSequence | KeyKind::SaveLastCombo => Effect::None,
        };
    }

    fn open_picker(&mut self, kind: PickerKind, face: Face, editing: Option<String>) {
        if let Some(id) = editing.as_deref().filter(|_| matches!(kind, PickerKind::EditSequence)) {
            self.close_sequence_gap(id);
        }
        let ids = match kind {
            PickerKind::NewChain | PickerKind::EditChain => self.recording_ids(),
            PickerKind::NewSequence | PickerKind::EditSequence => self.catalog.combos.iter().filter(|f| !f.input_only).map(|file| file.id.clone()).collect(),
        };
        let order = editing.as_ref().map(|id| self.existing_order(kind, id)).unwrap_or_default();
        let values = fill_ranks(&ids, &order);
        self.overlay_face = face;
        self.picker = Some(Picker { kind, cursor: 0, values, ids, editing });
    }

    fn existing_order(&self, kind: PickerKind, id: &str) -> Vec<String> {
        match kind {
            PickerKind::EditChain => self.macro_by_id(id).map(|file| file.steps.clone()).unwrap_or_default(),
            PickerKind::EditSequence => self.sequence_by_id(id).map(|file| file.combos.clone()).unwrap_or_default(),
            PickerKind::NewChain | PickerKind::NewSequence => Vec::new(),
        }
    }

    fn recording_ids(&self) -> Vec<String> {
        let tiles: Vec<CatalogTile<usize>> = self
            .catalog
            .macros
            .iter()
            .enumerate()
            .map(|(index, file)| CatalogTile { id: index, kind: if file.chain { TileKind::Chain } else { TileKind::Recording } })
            .collect();
        numbered_picker::chain_recordings(&tiles).map(|tile| self.catalog.macros[tile.id].id.clone()).collect()
    }

    fn press_picker(&mut self, button: Button, _face: Face) -> bool {
        match button {
            Button::B => self.picker = None,
            Button::X => self.finish_picker(),
            Button::A => {
                if let Some(picker) = self.picker.as_mut() {
                    let cursor = picker.cursor;
                    numbered_picker::apply_key(&mut picker.values, cursor, NumberKey::A);
                }
            }
            Button::R => {
                if let Some(picker) = self.picker.as_mut() {
                    let cursor = picker.cursor;
                    numbered_picker::apply_key(&mut picker.values, cursor, NumberKey::R);
                }
            }
            Button::Up => {
                if let Some(picker) = self.picker.as_mut() {
                    picker.cursor = stepped(picker.cursor, picker.ids.len(), false);
                }
            }
            Button::Down => {
                if let Some(picker) = self.picker.as_mut() {
                    picker.cursor = stepped(picker.cursor, picker.ids.len(), true);
                }
            }
            Button::JumpUp | Button::JumpDown => {
                if let Some(picker) = self.picker.as_mut() {
                    picker.cursor = jumped(picker.cursor, picker.ids.len(), button == Button::JumpDown);
                }
            }
            Button::Left | Button::Right | Button::L | Button::ZR | Button::ZL | Button::Y => {
                return false;
            }
        }
        true
    }

    fn finish_picker(&mut self) {
        let Some(picker) = self.picker.take() else { return };
        let ordered = ordered_ids::write_back(&picker.values, &picker.ids);
        match picker.kind {
            PickerKind::EditChain => {
                if let Some(id) = picker.editing {
                    if let Some(chain) = self.catalog.macros.iter_mut().find(|file| file.id == id) {
                        chain.steps = ordered.clone();
                        self.last_effect = Effect::WriteChainSteps { id, steps: ordered };
                    }
                }
            }
            PickerKind::EditSequence => {
                if let Some(id) = picker.editing {
                    if let Some(sequence) = self.catalog.sequences.iter_mut().find(|file| file.id == id) {
                        sequence.combos = ordered.clone();
                        self.last_effect = Effect::WriteSequenceCombos { id, combos: ordered };
                    }
                }
            }
            PickerKind::NewChain => self.open_keyboard(KeyKind::NewChain, "Name chain", ordered),
            PickerKind::NewSequence => self.open_keyboard(KeyKind::NewSequence, "Name sequence", ordered),
        }
    }

    fn open_keyboard(&mut self, kind: KeyKind, header: &'static str, ids: Vec<String>) {
        self.keyboard = Some(Keyboard { kind, ids, target: None });
        self.last_effect = Effect::Keyboard { header, draft: String::new() };
    }

    fn create_chain(&mut self, steps: &[String], name: &str) {
        let names: Vec<String> = self.catalog.macros.iter().filter(|file| file.chain).map(|file| file.name.clone()).collect();
        let n = unused_n("Chain", &names);
        let label = if name.is_empty() { format!("Chain {n}") } else { name.to_string() };
        self.last_effect = Effect::WriteChain { name: label.clone(), steps: steps.to_vec() };
        self.serial += 1;
        let id = format!("chain-{}", self.serial);
        self.catalog.macros.push(MacroFile {
            id: id.clone(),
            name: label,
            chain: true,
            steps: steps.to_vec(),
            start_save: None,
            start_save_on: true,
            recorded_on: String::new(),
            span: 0,
        });
        self.note_hot_macro(&id);
    }

    fn create_sequence(&mut self, combos: &[String], name: &str) {
        let names: Vec<String> = self.catalog.sequences.iter().map(|file| file.name.clone()).collect();
        let n = unused_n("Sequence", &names);
        let label = if name.is_empty() { format!("Sequence {n}") } else { name.to_string() };
        self.last_effect = Effect::WriteSequence { name: label.clone(), combos: combos.to_vec() };
        self.serial += 1;
        let id = format!("sequence-{}", self.serial);
        self.catalog.sequences.push(SequenceFile { id: id.clone(), name: label, combos: combos.to_vec(), streak: 0, best: 0 });
        self.catalog.hot_sequences.insert(0, id);
    }

    fn note_hot_save(&mut self, id: &str) {
        self.catalog.hot_saves.retain(|hot| hot != id);
        self.catalog.hot_saves.insert(0, id.to_string());
    }

    fn note_hot_macro(&mut self, id: &str) {
        self.catalog.hot_macros.retain(|hot| hot != id);
        self.catalog.hot_macros.insert(0, id.to_string());
    }

    /// The take the focused drill row can preview (F10).
    fn drill_take(&self, face: Face, group: u8) -> Option<(String, u32)> {
        let item = self.focused(face, group).filter(|item| item.kind == ItemKind::Combo)?;
        let drill = self.combo_by_id(&item.id).filter(|file| file.input_only)?;
        self.playable(drill)
    }

    /// The drill's owned take, when it has frames.
    fn playable(&self, drill: &ComboFile) -> Option<(String, u32)> {
        drill.take.as_ref().and_then(|id| self.takes.iter().find(|(take, frames)| take == id && *frames > 0)).cloned()
    }

    /// X on a drill row: post its preview. False when the row has no take to play.
    fn preview_drill(&mut self, face: Face, group: u8) -> bool {
        let Some((source, frames)) = self.drill_take(face, group) else { return false };
        self.last_effect = Effect::PreviewDrill { source, frames };
        true
    }

    fn focused(&self, face: Face, group: u8) -> Option<Item> {
        let place = self.place(face, group);
        let items = self.items(face, group, place.section);
        if items.is_empty() {
            return None;
        }
        let at = place.at[usize::from(place.section)].min(items.len() - 1);
        items.get(at).cloned()
    }

    fn items(&self, face: Face, group: u8, section: u8) -> Vec<Item> {
        if self.loading {
            return Vec::new();
        }
        let combos = group == COMBOS_SEQ;
        if !combos && section == 0 {
            return ordered(&self.catalog.saves, &self.catalog.hot_saves, |file| &file.id, |file| &file.name)
                .into_iter()
                .map(|file| Item { kind: ItemKind::Save, id: file.id.clone() })
                .collect();
        }
        if !combos {
            let mut rows: Vec<Item> = ordered(&self.catalog.macros, &self.catalog.hot_macros, |file| &file.id, |file| &file.name)
                .into_iter()
                .map(|file| Item { kind: ItemKind::Recording, id: file.id.clone() })
                .collect();
            rows.push(Item { kind: ItemKind::NewChain, id: String::new() });
            return rows;
        }
        if section == 0 {
            let mut rows: Vec<Item> = ordered(&self.catalog.sequences, &self.catalog.hot_sequences, |file| &file.id, |file| &file.name)
                .into_iter()
                .map(|file| Item { kind: ItemKind::Sequence, id: file.id.clone() })
                .collect();
            rows.push(Item { kind: ItemKind::NewSequence, id: String::new() });
            return rows;
        }
        let drills = self.place(face, group).drills;
        let mut rows = vec![Item { kind: if drills { ItemKind::RecordDrill } else { ItemKind::SaveLastCombo }, id: String::new() }];
        rows.extend(
            ordered(&self.catalog.combos, &self.catalog.hot_combos, |file| &file.id, |file| &file.name)
                .into_iter()
                .filter(|file| file.input_only == drills)
                .map(|file| Item { kind: ItemKind::Combo, id: file.id.clone() }),
        );
        if !drills { rows.push(Item { kind: ItemKind::Drills, id: String::new() }); }
        rows
    }

    fn section_view(&self, face: Face, group: u8, section: u8) -> SectionView {
        let items = self.items(face, group, section);
        let file_count = items.iter().filter(|item| !item.kind.action()).count();
        // Action rows that head the list (+ Save last combo…): the hot marks and the hairline count files only.
        let lead = items.iter().take_while(|item| item.kind.action()).count();
        let place = self.place(face, group);
        let cursor = place.at[usize::from(section)].min(items.len().saturating_sub(1));
        let focused = self.focused(face, group);
        let role = self.role(face);
        let header = if place.in_drills(group, section) { "DRILLS" } else { header_of(group, section) };
        let rows = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let file_index = index.saturating_sub(lead);
                self.row_mark(item, file_index, cursor == index && place.section == section, file_count, focused.as_ref(), role)
            })
            .collect();
        SectionView {
            header,
            header_floppy: (header == "SAVE STATES").then_some(Floppy::On),
            header_loop: (header == "MACROS").then_some(self.loop_of(role)),
            hairline_after: (file_count > 5).then_some(5 + lead),
            rows,
            cursor,
            files: file_count,
            loading: self.loading,
        }
    }

    fn row_mark(&self, item: &Item, index: usize, cursor: bool, file_count: usize, focused: Option<&Item>, role: ViewRole) -> RowMark {
        let hot = !item.kind.action() && index < 5 && file_count > 0 && self.is_hot(item);
        RowMark {
            name: self.item_name(item),
            green: self.is_green(item, role),
            cursor,
            hot,
            floppy: self.row_floppy(item),
            order: focused.and_then(|focus| self.order_badge(focus, item)),
            chain_count: self.macro_by_id(&item.id).filter(|file| file.chain).map(|file| file.steps.len() as u8),
            sub: self.row_sub(item),
        }
    }

    fn row_sub(&self, item: &Item) -> String {
        let recorded = |on: &str| if on.is_empty() { String::new() } else { format!("Recorded on {on}") };
        match item.kind {
            ItemKind::Save => self.save_by_id(&item.id).map(|file| file.fighters.clone()).unwrap_or_default(),
            ItemKind::Recording => match self.macro_by_id(&item.id) {
                Some(file) if file.chain => {
                    file.steps.iter().filter_map(|id| self.macro_by_id(id).map(|step| step.name.as_str())).collect::<Vec<_>>().join(" → ")
                }
                Some(file) => recorded(&file.recorded_on),
                None => String::new(),
            },
            ItemKind::Combo => self.combo_by_id(&item.id).map(|file| if file.input_only { format!("{} steps{}", file.span, if file.recorded_on.is_empty() { String::new() } else { format!(" · {}", recorded(&file.recorded_on)) }) } else { recorded(&file.recorded_on) }).unwrap_or_default(),
            ItemKind::Sequence => self
                .catalog
                .sequences
                .iter()
                .find(|file| file.id == item.id)
                .map(|file| {
                    file.combos
                        .iter()
                        .filter_map(|id| self.combo_by_id(id).map(|combo| combo.name.as_str()))
                        .collect::<Vec<_>>()
                        .join(" → ")
                })
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn is_hot(&self, item: &Item) -> bool {
        let hot = match item.kind {
            ItemKind::Save => &self.catalog.hot_saves,
            ItemKind::Recording => &self.catalog.hot_macros,
            ItemKind::Combo => &self.catalog.hot_combos,
            ItemKind::Sequence => &self.catalog.hot_sequences,
            _ => return false,
        };
        hot.iter().filter(|id| item.kind != ItemKind::Combo || self.combo_by_id(id).map(|f| f.input_only) == self.combo_by_id(&item.id).map(|f| f.input_only)).take(5).any(|id| id == &item.id)
    }

    fn is_green(&self, item: &Item, role: ViewRole) -> bool {
        match item.kind {
            ItemKind::Save => self.bound.as_deref() == Some(item.id.as_str()),
            ItemKind::Recording => self.active[role.index()].as_deref() == Some(item.id.as_str()),
            ItemKind::Combo | ItemKind::Sequence => self.drill.as_deref() == Some(item.id.as_str()),
            _ => false,
        }
    }

    fn row_floppy(&self, item: &Item) -> Option<Floppy> {
        let file = self.macro_by_id(&item.id)?;
        if item.kind != ItemKind::Recording {
            return None;
        }
        if file.chain {
            return self.chain_start_id(file).map(|_| if file.start_save_on { Floppy::On } else { Floppy::Off });
        }
        file.start_save.as_ref().map(|_| if file.start_save_on { Floppy::On } else { Floppy::Off })
    }

    fn order_badge(&self, focus: &Item, item: &Item) -> Option<u8> {
        if focus.kind == ItemKind::Recording && item.kind == ItemKind::Recording {
            if let Some(chain) = self.macro_by_id(&focus.id).filter(|file| file.chain) {
                return ordered_ids::order_of(&chain.steps, &item.id);
            }
            if let Some(chain) = self.macro_by_id(&item.id).filter(|file| file.chain) {
                return ordered_ids::order_of(&chain.steps, &focus.id);
            }
            return None;
        }
        if focus.kind == ItemKind::Sequence && item.kind == ItemKind::Combo {
            let sequence = self.sequence_by_id(&focus.id)?;
            return ordered_ids::order_of(&sequence.combos, &item.id);
        }
        if focus.kind == ItemKind::Combo && item.kind == ItemKind::Sequence {
            let sequence = self.sequence_by_id(&item.id)?;
            return ordered_ids::order_of(&sequence.combos, &focus.id);
        }
        None
    }

    fn item_name(&self, item: &Item) -> String {
        match item.kind {
            ItemKind::Drills => "DRILLS >".to_string(),
            ItemKind::RecordDrill => "+ Record drill…".to_string(),
            ItemKind::NewChain => "+ New chain".to_string(),
            ItemKind::NewSequence => "+ New sequence".to_string(),
            ItemKind::SaveLastCombo => "+ Save last combo…".to_string(),
            kind => self.slot_name(kind, &item.id).unwrap_or_default(),
        }
    }

    fn slot_name(&self, kind: ItemKind, id: &str) -> Option<String> {
        match kind {
            ItemKind::Save => self.save_by_id(id).map(|file| file.name.clone()),
            ItemKind::Recording => self.macro_by_id(id).map(|file| file.name.clone()),
            ItemKind::Combo => self.combo_by_id(id).map(|file| file.name.clone()),
            ItemKind::Sequence => self.sequence_by_id(id).map(|file| file.name.clone()),
            _ => None,
        }
    }

    fn slot_set_name(&mut self, kind: ItemKind, id: &str, name: &str) {
        match kind {
            ItemKind::Save => {
                if let Some(file) = self.catalog.saves.iter_mut().find(|file| file.id == id) {
                    file.name = name.to_string();
                }
            }
            ItemKind::Recording => {
                if let Some(file) = self.catalog.macros.iter_mut().find(|file| file.id == id) {
                    file.name = name.to_string();
                }
            }
            ItemKind::Combo => {
                if let Some(file) = self.catalog.combos.iter_mut().find(|file| file.id == id) {
                    file.name = name.to_string();
                }
            }
            ItemKind::Sequence => {
                if let Some(file) = self.catalog.sequences.iter_mut().find(|file| file.id == id) {
                    file.name = name.to_string();
                }
            }
            _ => {}
        }
    }

    fn slot_remove(&mut self, kind: ItemKind, id: &str) {
        match kind {
            ItemKind::Save => self.catalog.saves.retain(|file| file.id != id),
            ItemKind::Recording => self.catalog.macros.retain(|file| file.id != id),
            ItemKind::Combo => self.catalog.combos.retain(|file| file.id != id),
            ItemKind::Sequence => self.catalog.sequences.retain(|file| file.id != id),
            _ => {}
        }
    }

    fn connection_lines(&self, face: Face, item: &Item) -> Vec<ConnLine> {
        let role = self.role(face);
        match item.kind {
            ItemKind::Save => self.save_lines(&item.id),
            ItemKind::Recording => self.macro_lines(&item.id, role),
            ItemKind::Combo => self.combo_lines(&item.id),
            ItemKind::Sequence => self.sequence_lines(&item.id),
            ItemKind::Drills => vec![line("Input drills")],
            ItemKind::RecordDrill => vec![line("Record Player 1 directly as an input drill.")],
            ItemKind::NewChain => vec![line("A new chain")],
            ItemKind::NewSequence => vec![line("A new sequence")],
            ItemKind::SaveLastCombo => vec![line("Saves the strip's latest exchange as a combo and drills it. A names it.")],
        }
    }

    fn save_lines(&self, id: &str) -> Vec<ConnLine> {
        let Some(save) = self.save_by_id(id) else { return Vec::new() };
        let bound = self.bound.as_deref() == Some(id);
        let mut lines = vec![line(save.name.clone())];
        if !save.fighters.is_empty() {
            lines.push(line(format!("Fighters: {}", save.fighters)));
        }
        lines.push(line(format!("Bound to Save / Load: {}", if bound { "yes" } else { "no" })));
        if bound {
            lines.push(line("Load warps here"));
        }
        for file in &self.catalog.macros {
            let uses = if file.chain { self.chain_start_id(file).is_some_and(|save_id| save_id == id) } else { file.start_save.as_deref() == Some(id) };
            if uses {
                lines.push(self.recording_line(format!("Start save of {}", file.name), &file.id));
            }
        }
        lines
    }

    fn macro_lines(&self, id: &str, role: ViewRole) -> Vec<ConnLine> {
        let Some(file) = self.macro_by_id(id).cloned() else { return Vec::new() };
        let mut lines = Vec::new();
        if file.chain {
            match self.chain_start_name(&file) {
                Some(name) => lines.push(self.recording_line(format!("Starts from {name}"), id)),
                None => lines.push(line("No start save · plays from where the fighters are")),
            }
            lines.push(line(format!("Plays {} steps", file.steps.len())));
            for (index, step) in file.steps.iter().enumerate() {
                let name = self.macro_by_id(step).map(|file| file.name.clone()).unwrap_or_else(|| step.clone());
                lines.push(line(ordered_ids::step_line(index, &name)));
            }
            if self.active[role.index()].as_deref() == Some(id) {
                lines.push(line(format!("Loop {}", self.loop_of(role).label())));
            }
            return lines;
        }
        match file.start_save.as_ref().and_then(|save| self.save_name(save)) {
            Some(name) => lines.push(self.recording_line(format!("Starts from {name}"), id)),
            None => lines.push(line("No start save · plays from where the fighters are")),
        }
        for chain in self.catalog.macros.iter().filter(|chain| chain.chain) {
            if let Some(index) = chain.steps.iter().position(|step| step == id) {
                lines.push(line(format!("{} · step {}", chain.name, index + 1)));
            }
        }
        for combo in self.catalog.combos.iter().filter(|combo| combo.made_from.as_deref() == Some(id)) {
            lines.push(line(format!("Combo {}", combo.name)));
        }
        if !file.recorded_on.is_empty() {
            lines.push(line(format!("Recorded on {}", file.recorded_on)));
        }
        lines
    }

    fn combo_lines(&self, id: &str) -> Vec<ConnLine> {
        let Some(combo) = self.combo_by_id(id) else { return Vec::new() };
        let mut lines = Vec::new();
        if combo.input_only && self.playable(combo).is_none() {
            // restriction: drill-preview-rerecord
            lines.push(line("Re-record to preview"));
        }
        // First: the help joins these lines into one, and a long list of links must not push the fighter out (#49).
        if !combo.recorded_on.is_empty() {
            lines.push(line(format!("Recorded on {}", combo.recorded_on)));
        }
        if let Some(source) = combo.made_from.as_ref().and_then(|id| self.macro_by_id(id).map(|file| file.name.clone())) {
            lines.push(line(format!("from {source}")));
        }
        for link in combo_link_lines(&combo.combo, super::move_names::Namer::recorded(combo.fighter)) {
            lines.push(line(link));
        }
        for sequence in &self.catalog.sequences {
            if sequence.combos.iter().any(|step| step == id) {
                lines.push(line(format!("In sequence {}", sequence.name)));
            }
        }
        lines
    }

    fn sequence_lines(&self, id: &str) -> Vec<ConnLine> {
        let Some(sequence) = self.sequence_by_id(id) else { return Vec::new() };
        let mut lines = Vec::new();
        for (index, step) in sequence.combos.iter().enumerate() {
            let name = self.combo_by_id(step).map(|file| file.name.clone()).unwrap_or_else(|| step.clone());
            lines.push(line(ordered_ids::step_line(index, &name)));
        }
        lines.push(line(format!("streak {} · best {}", sequence.streak, sequence.best)));
        lines
    }

    fn built_file_rows(&self) -> Vec<BuiltRow> {
        let Some(file) = &self.file else { return Vec::new() };
        let id = file.id.as_str();
        match file.kind {
            ItemKind::Save => self.save_file_rows(id),
            ItemKind::Recording if self.macro_by_id(id).is_some_and(|m| m.chain) => self.chain_file_rows(id),
            ItemKind::Recording => self.macro_file_rows(id),
            ItemKind::Combo => self.combo_file_rows(id),
            ItemKind::Sequence => self.sequence_file_rows(id),
            _ => Vec::new(),
        }
    }

    fn save_file_rows(&self, id: &str) -> Vec<BuiltRow> {
        let on = self.bound.as_deref() == Some(id);
        vec![
            row("Bind to Save / Load", if on { "On" } else { "Off" }, Act::Bind),
            row("Duplicate", "", Act::Duplicate),
            row_rename(),
            row_delete(),
        ]
    }

    fn macro_file_rows(&self, id: &str) -> Vec<BuiltRow> {
        let mut rows = Vec::new();
        if let Some(file) = self.macro_by_id(id) {
            if let Some(save) = file.start_save.as_ref() {
                let name = self.save_name(save).unwrap_or_else(|| save.clone());
                rows.push(row("Start save", if file.start_save_on { format!("{name} · On") } else { "Off".into() }, Act::StartSave));
            }
        }
        rows.push(self.active_row(id));
        // rc.12 TR1: the Playback Slot (Mash, Followup, the overrides, Ledge Options) this macro plays as.
        let slot = self.slot_of(id).map_or_else(|| "None".to_string(), |i| format!("Slot {}", i + 1));
        rows.push(row("Playback slot", slot, Act::PlaybackSlot));
        rows.push(row_rename());
        rows.push(row("Crop", "", Act::Crop));
        rows.push(row("Make combo", "", Act::MakeCombo));
        rows.push(row_delete());
        rows
    }

    fn chain_file_rows(&self, id: &str) -> Vec<BuiltRow> {
        let mut rows = Vec::new();
        if let Some(file) = self.macro_by_id(id) {
            if let Some(name) = self.chain_start_name(file) {
                rows.push(row("Start save", if file.start_save_on { format!("{name} · On") } else { "Off".into() }, Act::StartSave));
            }
        }
        rows.push(self.active_row(id));
        rows.push(row("Edit chain", "", Act::EditChain));
        rows.push(row_rename());
        rows.push(row("Make combo", "", Act::MakeCombo));
        rows.push(row_delete());
        rows
    }

    fn combo_file_rows(&self, id: &str) -> Vec<BuiltRow> {
        let mut rows = vec![row("Drill", "This becomes the drill", Act::Drill)];
        if let Some(combo) = self.combo_by_id(id) {
            if let Some(source) = combo.made_from.as_ref().and_then(|id| self.macro_by_id(id).map(|file| file.name.clone())) {
                rows.push(row("Made from", format!("from {source}"), Act::Info));
            }
        }
        rows.push(row_rename());
        rows.push(row("Crop", "", Act::Crop));
        rows.push(row_delete());
        rows
    }

    fn sequence_file_rows(&self, id: &str) -> Vec<BuiltRow> {
        let mut rows = vec![row("Drill sequence", "This becomes the drill", Act::Drill)];
        if let Some(sequence) = self.sequence_by_id(id) {
            for (index, step) in sequence.combos.iter().enumerate() {
                let name = self.combo_by_id(step).map(|file| file.name.clone()).unwrap_or_else(|| step.clone());
                rows.push(BuiltRow {
                    title: ordered_ids::step_line(index, &name),
                    value: "A drills this combo".to_string(),
                    act: Act::DrillCombo,
                    drill: Some(step.clone()),
                });
            }
        }
        rows.push(row("Edit combos", "", Act::EditCombos));
        rows.push(row_rename());
        rows.push(row_delete());
        rows
    }

    fn active_row(&self, id: &str) -> BuiltRow {
        let on = self.active[self.page_role.index()].as_deref() == Some(id);
        let value = if on { format!("On · {}", self.page_role.label()) } else { "Off".to_string() };
        row("Set active", value, Act::SetActive)
    }

    /// A Connections line naming a recording, with that recording's floppy.
    fn recording_line(&self, text: String, id: &str) -> ConnLine {
        ConnLine { text, floppy: self.row_floppy(&Item { kind: ItemKind::Recording, id: id.to_string() }) }
    }

    fn macro_by_id(&self, id: &str) -> Option<&MacroFile> {
        self.catalog.macros.iter().find(|file| file.id == id)
    }

    fn save_by_id(&self, id: &str) -> Option<&SaveFile> {
        self.catalog.saves.iter().find(|file| file.id == id)
    }

    fn combo_by_id(&self, id: &str) -> Option<&ComboFile> {
        self.catalog.combos.iter().find(|file| file.id == id)
    }

    fn sequence_by_id(&self, id: &str) -> Option<&SequenceFile> {
        self.catalog.sequences.iter().find(|file| file.id == id)
    }

    fn save_name(&self, id: &str) -> Option<String> {
        self.save_by_id(id).map(|file| file.name.clone())
    }

    fn chain_start_id(&self, chain: &MacroFile) -> Option<&str> {
        let first = chain.steps.first()?;
        let recording = self.catalog.macros.iter().find(|file| file.id == *first && !file.chain)?;
        recording.start_save.as_deref()
    }

    fn chain_start_name(&self, chain: &MacroFile) -> Option<String> {
        self.save_name(self.chain_start_id(chain)?)
    }
}

/// The hot five: the five ids used last, newest first; an id breaks a tie. A sixth use pushes the oldest out. Recording's index
/// (`library::RecordingIndex`) and Break the Targets' saved layouts (`model::btt::listed`) both take theirs from here.
pub fn hot_five<'a>(uses: impl Iterator<Item = (&'a str, u64)>) -> Vec<String> {
    let mut rows: Vec<(&str, u64)> = uses.collect();
    rows.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(right.0)));
    rows.into_iter().take(5).map(|(id, _)| id.to_owned()).collect()
}

/// A list's order: the hot ids first, in their order (newest first), then the rest by name. Break the Targets' saved layouts
/// use it too (`model::btt::listed`), so both lists order the same way.
pub fn ordered<'a, T>(files: &'a [T], hot: &[String], id: impl Fn(&T) -> &str, name: impl Fn(&T) -> &str) -> Vec<&'a T> {
    let mut out: Vec<&T> = Vec::new();
    for hot_id in hot {
        if let Some(file) = files.iter().find(|file| id(file) == hot_id) {
            if !out.iter().any(|kept| id(kept) == hot_id) {
                out.push(file);
            }
        }
    }
    let mut rest: Vec<&T> = files.iter().filter(|file| !out.iter().any(|kept| id(kept) == id(file))).collect();
    rest.sort_by(|a, b| name(a).cmp(name(b)));
    out.extend(rest);
    out
}

fn header_of(group: u8, section: u8) -> &'static str {
    match (group == COMBOS_SEQ, section) {
        (false, 0) => "SAVE STATES",
        (false, _) => "MACROS",
        (true, 0) => "SEQUENCES",
        _ => "COMBOS",
    }
}

fn fill_ranks(ids: &[String], order: &[String]) -> Vec<u8> {
    let mut values = vec![0u8; ids.len() + 1];
    for (rank, step) in order.iter().enumerate() {
        if let Some(index) = ids.iter().position(|id| id == step) {
            values[index + 1] = (rank + 1) as u8;
        }
    }
    values
}

/// A Connections line with no floppy.
fn line(text: impl Into<String>) -> ConnLine {
    ConnLine { text: text.into(), floppy: None }
}

/// A file row that drills nothing.
fn row(title: impl Into<String>, value: impl Into<String>, act: Act) -> BuiltRow {
    BuiltRow { title: title.into(), value: value.into(), act, drill: None }
}

fn row_rename() -> BuiltRow {
    row("Rename", "", Act::Rename)
}

fn row_delete() -> BuiltRow {
    row("Delete", "", Act::Delete)
}

fn item_kind_of(kind: KeyKind) -> Option<ItemKind> {
    match kind {
        KeyKind::RenameSave => Some(ItemKind::Save),
        KeyKind::RenameMacro | KeyKind::RenameChain => Some(ItemKind::Recording),
        KeyKind::RenameCombo => Some(ItemKind::Combo),
        KeyKind::RenameSequence => Some(ItemKind::Sequence),
        KeyKind::NewChain | KeyKind::NewSequence | KeyKind::SaveLastCombo => None,
    }
}

pub(crate) fn unused_n(prefix: &str, names: &[String]) -> u32 {
    let mut n = 1u32;
    loop {
        let label = format!("{prefix} {n}");
        if !names.iter().any(|name| name == &label) {
            return n;
        }
        n += 1;
    }
}

/// A link's move line ("L1 jab → jab TRUE"), not the events or drift line under it.
fn link_head(line: &str) -> bool {
    line.strip_prefix(['L','S']).and_then(|rest| rest.split_once(' ')).is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// One link's move line, then its input events, then drift when the file recorded a hold.
/// An older presses-only file shows its presses and no drift.
pub fn combo_link_lines(combo: &Combo, namer: super::move_names::Namer<'_>) -> Vec<String> {
    combo.links().enumerate().flat_map(|(index, link)| {
        if combo.input_only { let mut lines = vec![format!("S{} Step {}", index + 1, index + 1)]; if let Some(events) = event_line(link) { lines.push(events); } lines } else { link_panel_lines(combo, index, link, namer) }
    }).collect()
}

fn link_panel_lines(combo: &Combo, index: usize, link: &super::combo::LinkRec, namer: super::move_names::Namer<'_>) -> Vec<String> {
    let word = if link.is_true() { "TRUE" } else { "FALSE" };
    let mut from = String::new();
    let mut to = String::new();
    let [first, follow] = combo.descs(index).unwrap_or([link.desc(0), link.desc(1)]);
    let _ = first.write(&mut from, false, namer);
    let _ = follow.write(&mut to, false, namer);
    let mut lines = vec![format!(
        "L{} {} → {} {word}",
        index + 1,
        if namer.is_on() { from } else { from.to_ascii_lowercase() },
        if namer.is_on() { to } else { to.to_ascii_lowercase() }
    )];
    if let Some(events) = event_line(link) {
        lines.push(events);
    }
    if let Some(drift) = drift_line(link.drift) {
        lines.push(drift);
    }
    lines
}

fn event_line(link: &super::combo::LinkRec) -> Option<String> {
    let parts: Vec<String> = link.events().map(event_token).collect();
    if parts.is_empty() { None } else { Some(parts.join(" · ")) }
}

fn event_token(event: super::combo::Event) -> String {
    use super::combo::{Flick, Kind};
    use super::frame_strip::Button;
    match event.kind {
        Kind::Press(Button::Jump) => "jump".to_string(),
        Kind::Press(Button::Attack) => with_dir(event.dir, "A"),
        Kind::Press(Button::Special) => with_dir(event.dir, "B"),
        Kind::Press(Button::Shield) => "shield".to_string(),
        Kind::Press(Button::Grab) => "grab".to_string(),
        Kind::Release(_) => "release".to_string(),
        Kind::CStick => with_dir(event.dir, "c-stick"),
        Kind::Flick(Flick::FastFall) => with_dir(event.dir, "flick"),
        Kind::Flick(Flick::Dash) => with_dir(event.dir, "dash"),
        Kind::Flick(Flick::TapJump) => with_dir(event.dir, "tap"),
        Kind::Aim(part) => {
            let arrow = dir_arrow(event.dir);
            format!("{} {}", if part == 1 { "1st" } else { "2nd" }, if arrow.is_empty() { "none" } else { arrow })
        }
    }
}

fn with_dir(dir: super::combo::Dir, word: &str) -> String {
    match dir_arrow(dir) {
        "" => word.to_string(),
        arrow => format!("{arrow} {word}"),
    }
}

fn dir_arrow(dir: super::combo::Dir) -> &'static str {
    // The UI font maps the four cardinal arrows and not the diagonals.
    super::action::arrow(dir)
}

fn drift_line(dir: Option<super::combo::Dir>) -> Option<String> {
    let arrow = dir_arrow(dir?);
    if arrow.is_empty() { None } else { Some(format!("drift {arrow}")) }
}

/// Which catalog generation the open menu already loaded.
///
/// Closing the menu drops the model, and the catalog with it. A file write is
/// not the only reason to load: the next open has an empty catalog even when
/// the generation did not change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogWatch {
    seen: u64,
}

impl Default for CatalogWatch {
    fn default() -> Self {
        // Distinct from generation 0, so the first open loads.
        Self { seen: u64::MAX }
    }
}

impl CatalogWatch {
    pub fn on_menu_dropped(self) -> Self {
        Self { seen: u64::MAX }
    }

    pub fn generation(self) -> Option<u64> {
        (self.seen != u64::MAX).then_some(self.seen)
    }

    /// `true` once, when `generation` is not the one already given to the open menu.
    pub fn take_load(self, generation: u64) -> (Self, bool) {
        if self.seen == generation {
            (self, false)
        } else {
            (Self { seen: generation }, true)
        }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::chords::Request;
    use crate::model::combo::{Combo, LinkRec};
    use crate::model::frame_strip::{Move, Verdict};
    use crate::model::menu::input::Button;
    use crate::model::menu::table::{Page, ROWS};
    use crate::model::menu::{Env, Menu};
    use crate::model::settings::{self, Settings};

    fn link(from: Move, to: Move, tru: bool) -> LinkRec {
        LinkRec::presses(from, to, if tru { Verdict::True(None) } else { Verdict::Drop(1) }, None, &[])
    }

    /// Menu speed: a menu opened before the worker's catalog is built lists no files and says so in each header; the
    /// catalog's arrival ends it. A model that was never told is not loading (tests, the pop-up's own model).
    fn the_lists_say_loading_until_the_catalog_arrives() {
        let mut screens = RecordingScreens::new();
        assert!(!screens.loading());
        assert!(!screens.section(Face::Page, SAVES_MACROS, 1).loading);
        screens.set_loading();
        assert!(screens.loading());
        for (group, section) in [(SAVES_MACROS, 0), (SAVES_MACROS, 1), (COMBOS_SEQ, 0), (COMBOS_SEQ, 1)] {
            let view = screens.section(Face::Page, group, section);
            assert!(view.loading && view.files == 0, "{group}/{section}");
            assert!(screens.section(Face::Popup, group, section).loading);
        }
        screens.set_catalog(example());
        assert!(!screens.loading());
        let view = screens.section(Face::Page, SAVES_MACROS, 0);
        assert!(!view.loading && view.files > 0);
    }

    fn a_reopened_menu_loads_the_catalog_without_a_new_file() {
        let opened = CatalogWatch::default();
        let (loaded, due) = opened.take_load(3);
        assert!(due);
        let (held, due) = loaded.take_load(3);
        assert!(!due);
        let (reopened, due) = held.on_menu_dropped().take_load(3);
        assert!(due);
        let (held, due) = reopened.take_load(3);
        assert!(!due);
        let (_, due) = held.take_load(4);
        assert!(due);
    }

    fn example() -> Catalog {
        let mut saves = Vec::new();
        for index in 0..16 {
            saves.push(SaveFile { id: format!("s{index}"), name: format!("Save {index}"), fighters: String::new() });
        }
        saves[0] = SaveFile { id: "corner".into(), name: "Corner".into(), fighters: "Pikachu · Mario".into() };
        saves[1] = SaveFile { id: "platform".into(), name: "Platform".into(), fighters: String::new() };
        saves[2] = SaveFile { id: "nair-start".into(), name: "Nair loop · start".into(), fighters: String::new() };
        Catalog {
            saves,
            macros: vec![
                MacroFile {
                    id: "nair".into(),
                    name: "Nair loop".into(),
                    chain: false,
                    steps: Vec::new(),
                    start_save: Some("nair-start".into()),
                    start_save_on: true,
                    recorded_on: "Pikachu".into(),
                    span: 12,
                },
                MacroFile {
                    id: "dash".into(),
                    name: "Dash grab".into(),
                    chain: false,
                    steps: Vec::new(),
                    start_save: None,
                    start_save_on: true,
                    recorded_on: String::new(),
                    span: 0,
                },
                MacroFile {
                    id: "throw".into(),
                    name: "CPU throw".into(),
                    chain: false,
                    steps: Vec::new(),
                    start_save: None,
                    start_save_on: true,
                    recorded_on: String::new(),
                    span: 0,
                },
                MacroFile {
                    id: "pressure".into(),
                    name: "Corner pressure".into(),
                    chain: true,
                    steps: vec!["nair".into(), "dash".into(), "throw".into()],
                    start_save: None,
                    start_save_on: true,
                    recorded_on: String::new(),
                    span: 0,
                },
            ],
            combos: vec![ComboFile { input_only: false, take: None,
                id: "nairc".into(),
                name: "Nair loop combo".into(),
                recorded_on: String::new(),
                made_from: Some("nair".into()),
                combo: Box::new(Combo::from_links([link(Move::Aerial, Move::Aerial, true)]).unwrap()),
                fighter: None,
                span: 1,
            }],
            sequences: vec![SequenceFile {
                id: "corner-seq".into(),
                name: "Corner to kill".into(),
                combos: vec!["nairc".into()],
                streak: 2,
                best: 6,
            }],
            hot_saves: vec!["corner".into(), "platform".into(), "nair-start".into()],
            hot_macros: vec!["nair".into(), "pressure".into(), "dash".into(), "throw".into()],
            hot_combos: vec!["nairc".into()],
            hot_sequences: vec!["corner-seq".into()],
        }
    }

    fn screens() -> RecordingScreens {
        let mut screens = RecordingScreens::new();
        screens.set_catalog(example());
        screens.bound = Some("corner".into());
        screens.active[0] = Some("nair".into());
        screens.active[1] = Some("pressure".into());
        screens
    }

    fn open_popup(screens: &mut RecordingScreens) {
        screens.popup_open = true;
    }

    fn menu() -> Menu {
        Menu::new(Settings::factory(), Env::default())
    }

    fn to_recording(menu: &mut Menu) {
        menu.press(Button::R);
        menu.press(Button::R);
        assert_eq!(menu.page(), Page::Recording);
    }

    fn one_save_is_bound_and_the_page_opens_it() {
        let mut screens = screens();
        open_popup(&mut screens);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 1);
        screens.press_popup(Button::A);
        screens.press_popup(Button::B);
        assert_eq!(screens.selection().bound.as_deref(), Some("platform"));
        assert!(!screens.section(Face::Page, SAVES_MACROS, 0).rows.iter().find(|row| row.name == "Corner").unwrap().green);
        // The bind was a use: Platform is now the most recently used save, the first row.
        assert_eq!(screens.section(Face::Page, SAVES_MACROS, 0).rows[0].name, "Platform");
        screens.set_cursor(Face::Page, SAVES_MACROS, 0, 0);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert_eq!(screens.file_rows()[0].value, "On");
        screens.handle_page(Button::Left, true, SAVES_MACROS);
        assert_eq!(screens.selection().bound, None);
        screens.handle_page(Button::Right, true, SAVES_MACROS);
        assert_eq!(screens.selection().bound.as_deref(), Some("platform"));
    }

    fn popup_buttons_match_q11() {
        let mut screens = screens();
        open_popup(&mut screens);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 2);
        screens.press_popup(Button::Right);
        assert_eq!(screens.current_section(Face::Popup, SAVES_MACROS), 1);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 1), 2);
        let before = screens.section_cursor(Face::Popup, SAVES_MACROS, 1);
        screens.press_popup(Button::Up);
        assert_eq!(screens.current_section(Face::Popup, SAVES_MACROS), 1);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 1), before - 1);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 0);
        screens.press_popup(Button::Up);
        assert_eq!(screens.tab(), 0, "Up does not enter the tab row");
        screens.press_popup(Button::ZL);
        assert_eq!(screens.tab(), 1);
        screens.press_popup(Button::L);
        assert_eq!(screens.tab(), 0);
        screens.press_popup(Button::ZR);
        assert_eq!(screens.role(Face::Popup), ViewRole::Cpu);
        screens.press_popup(Button::ZR);
        assert_eq!(screens.role(Face::Popup), ViewRole::Player);
        screens.press_popup(Button::B);
        assert!(!screens.popup_open());
    }

    fn popup_zr_toggles_the_role_like_the_page() {
        for waiting_room in [false, true] {
            for presses in [0, 1, 2, 3] {
                let env = Env { waiting_room, ..Env::default() };
                let mut page = Menu::new(Settings::factory(), env);
                let mut popup = Menu::new(Settings::factory(), env);
                to_recording(&mut page);
                popup.on_request(Request::RecordingPopup);
                let page_popup = page.recording().role(Face::Popup);
                let popup_page = popup.recording().role(Face::Page);
                for _ in 0..presses {
                    page.press(Button::ZR);
                    popup.press(Button::ZR);
                }
                assert_eq!(page.recording().role(Face::Page), popup.recording().role(Face::Popup));
                assert_eq!(page.recording().role(Face::Popup), page_popup);
                assert_eq!(popup.recording().role(Face::Page), popup_page);
            }
        }
    }

    fn popup_l_r_zl_switch_tabs_and_never_set_the_role() {
        for waiting_room in [false, true] {
            let mut menu = Menu::new(Settings::factory(), Env { waiting_room, ..Env::default() });
            menu.on_request(Request::RecordingPopup);
            for button in [Button::L, Button::R, Button::ZL] {
                menu.press(button);
                assert_eq!((menu.recording().tab(), menu.recording().role(Face::Popup)), (1, ViewRole::Player));
                menu.press(button);
                assert_eq!((menu.recording().tab(), menu.recording().role(Face::Popup)), (0, ViewRole::Player));
            }
            menu.press(Button::ZR);
            assert_eq!((menu.recording().tab(), menu.recording().role(Face::Popup)), (0, ViewRole::Cpu));
            for button in [Button::L, Button::R, Button::ZL] {
                menu.press(button);
                assert_eq!((menu.recording().tab(), menu.recording().role(Face::Popup)), (0, ViewRole::Cpu));
            }
            menu.press(Button::ZR);
            assert_eq!((menu.recording().tab(), menu.recording().role(Face::Popup)), (0, ViewRole::Player));
        }
    }

    fn popup_footer_names_zr_role_and_tab_keys() {
        let mut s = screens();
        open_popup(&mut s);
        let mut drill = s.catalog.combos[0].clone();
        drill.id = "preview-drill".into();
        drill.input_only = true;
        drill.take = Some("preview-take".into());
        s.catalog.combos.push(drill);
        s.takes.push(("preview-take".into(), 25));
        for (tab, section, preview) in [(0, 0, false), (0, 1, false), (1, 0, false),
            (1, 1, false), (1, 1, true)] {
            s.popup_tab = tab;
            let group = if tab == 0 { SAVES_MACROS } else { COMBOS_SEQ };
            s.set_cursor(Face::Popup, group, section, 0);
            if preview {
                s.place_mut(Face::Popup, group).drills = true;
                let at = s.items(Face::Popup, group, section).iter()
                    .position(|item| item.id == "preview-drill").unwrap();
                s.set_cursor(Face::Popup, group, section, at);
            }
            let hint = s.hint(Face::Popup);
            assert!(hint.contains("ZL/L/R tab   ZR role"), "{hint}");
            assert!(!hint.contains("L/R role"), "{hint}");
            if preview { assert!(hint.contains("X preview"), "{hint}"); }
        }
        s.set_popup_role(ViewRole::Cpu);
        for section in [0, 1] {
            s.set_cursor(Face::Popup, SAVES_MACROS, section, 0);
            let hint = s.hint(Face::Popup);
            assert!(hint.contains("ZR role"), "{hint}");
            assert!(!hint.contains("tab"), "{hint}");
            assert!(!hint.contains("L/R role"), "{hint}");
        }
    }

    fn paging_moves_one_section_by_twelve() {
        let mut screens = screens();
        open_popup(&mut screens);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 1, 1);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 0);
        screens.press_popup(Button::JumpDown);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), 12);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 1), 1);
        screens.press_popup(Button::JumpDown);
        let len = screens.section(Face::Popup, SAVES_MACROS, 0).rows.len();
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), len - 1);
        screens.press_popup(Button::JumpUp);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), len - 1 - crate::model::menu::LIST_JUMP);
        screens.set_cursor(Face::Page, SAVES_MACROS, 0, 0);
        screens.handle_page(Button::JumpDown, true, SAVES_MACROS);
        assert_eq!(screens.section_cursor(Face::Page, SAVES_MACROS, 0), 12);
    }

    /// The pop-up wraps inside its section; page jumps at an edge go on to the other section.
    fn library_lists_wrap_at_their_edges() {
        let mut screens = screens();
        open_popup(&mut screens);
        let len = screens.section(Face::Popup, SAVES_MACROS, 0).rows.len();
        assert!(len > crate::model::menu::LIST_JUMP + 1);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, len - 1);
        screens.press_popup(Button::Down);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), 0, "Down wraps to the first entry");
        screens.press_popup(Button::Up);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), len - 1, "Up wraps to the last entry");
        assert_eq!(screens.current_section(Face::Popup, SAVES_MACROS), 0);
        screens.press_popup(Button::JumpDown);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), 0, "ZL + Down wraps from the last entry");
        screens.press_popup(Button::JumpUp);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), len - 1, "ZL + Up wraps from the first entry");
        let page = screens.section(Face::Page, SAVES_MACROS, 0).rows.len();
        screens.set_cursor(Face::Page, SAVES_MACROS, 0, page - 1);
        screens.handle_page(Button::JumpDown, true, SAVES_MACROS);
        assert_eq!((screens.current_section(Face::Page, SAVES_MACROS),
            screens.section_cursor(Face::Page, SAVES_MACROS, 1)), (1, 0));
        screens.handle_page(Button::JumpUp, true, SAVES_MACROS);
        assert_eq!(screens.current_section(Face::Page, SAVES_MACROS), 0);
        assert_eq!(screens.section_cursor(Face::Page, SAVES_MACROS, 0), page - 1);
    }

    fn long_catalog() -> Catalog {
        let mut catalog = example();
        catalog.saves.truncate(3);
        for i in 0..45 {
            catalog.macros.push(MacroFile {
                id: format!("long-{i}"), name: format!("Long {i}"), chain: false, steps: Vec::new(),
                start_save: None, start_save_on: true, recorded_on: String::new(), span: 0,
            });
        }
        catalog
    }

    fn recording_list_menu() -> Menu {
        let mut menu = menu();
        to_recording(&mut menu);
        assert_eq!(menu.focus(), Some(crate::model::menu::Entry::Group(usize::from(SAVES_MACROS))));
        menu.recording_mut().set_catalog(long_catalog());
        menu
    }

    fn f9_new_menu(group: u8, section: u8, label: &str) -> Menu {
        let mut m = recording_list_menu();
        for _ in 0..20 {
            if m.focus() == Some(crate::model::menu::Entry::Group(usize::from(group))) { break; }
            m.press(Button::JumpDown);
        }
        assert_eq!(m.focus(), Some(crate::model::menu::Entry::Group(usize::from(group))));
        let rows = m.recording().section(Face::Page, group, section).rows;
        let index = rows.iter().position(|row| row.name == label).unwrap();
        m.recording_mut().set_cursor(Face::Page, group, section, index);
        m
    }

    fn f9_b_backs_out_of_a_new_picker() {
        for (group, section, label) in [
            (SAVES_MACROS, 1, "+ New chain"), (COMBOS_SEQ, 0, "+ New sequence"),
        ] {
          for numbered in [false, true] {
            let mut m = f9_new_menu(group, section, label);
            let count = (m.recording().catalog().macros.len(), m.recording().catalog().sequences.len());
            let expected_list = m.focus_list().unwrap();
            let expected_hint = m.recording().hint(Face::Page);
            m.press(Button::A);
            assert!(m.recording().picker_open());
            if numbered { m.press(Button::A); }
            assert_eq!(m.recording().hint(Face::Page), "A add number   R remove   X done   B back");
            m.press(Button::B);
            assert!(!m.recording().picker_open());
            assert!(!m.recording().naming());
            assert!(!matches!(m.recording_mut().take_effect(), Effect::Keyboard { .. }));
            assert_eq!((m.recording().catalog().macros.len(), m.recording().catalog().sequences.len()), count);
            assert_eq!(m.focus_list(), Some(expected_list));
            assert_eq!(m.recording().hint(Face::Page), expected_hint);
          }
        }
        for (group, section, label, tab) in [
            (SAVES_MACROS, 1, "+ New chain", 0), (COMBOS_SEQ, 0, "+ New sequence", 1),
        ] {
            for numbered in [false, true] {
                let mut m = f9_new_menu(group, section, label);
                m.recording_mut().popup_open = true;
                m.recording_mut().popup_tab = tab;
                let index = m.recording().section(Face::Popup, group, section).rows.iter()
                    .position(|row| row.name == label).unwrap();
                m.recording_mut().set_cursor(Face::Popup, group, section, index);
                m.press(Button::A);
                if numbered { m.press(Button::A); }
                assert_eq!(m.recording().hint(Face::Popup), "A add number   R remove   X done   B back");
                m.press(Button::B);
                assert!(!m.recording().picker_open());
                assert!(!m.recording().naming());
                assert!(!matches!(m.recording_mut().take_effect(), Effect::Keyboard { .. }));
            }
        }
    }

    fn f9_x_is_done_and_asks_the_name() {
        for (group, section, label, header) in [
            (SAVES_MACROS, 1, "+ New chain", "Name chain"),
            (COMBOS_SEQ, 0, "+ New sequence", "Name sequence"),
        ] {
            for face in [Face::Page, Face::Popup] {
                for numbered in [false, true] {
                    let mut m = f9_new_menu(group, section, label);
                    if face == Face::Popup {
                        m.recording_mut().popup_open = true;
                        m.recording_mut().popup_tab = u8::from(group == COMBOS_SEQ);
                        let index = m.recording().section(face, group, section).rows.iter()
                            .position(|row| row.name == label).unwrap();
                        m.recording_mut().set_cursor(face, group, section, index);
                    }
                    let counts = (m.recording().catalog().macros.len(), m.recording().catalog().sequences.len());
                    m.press(Button::A);
                    if numbered { m.press(Button::A); }
                    m.press(Button::X);
                    assert!(matches!(m.recording_mut().take_effect(), Effect::Keyboard { header: h, .. } if h == header));
                    assert!(m.recording().naming());
                    m.recording_mut().submit_name("");
                    let after = (m.recording().catalog().macros.len(), m.recording().catalog().sequences.len());
                    if group == SAVES_MACROS { assert_eq!(after, (counts.0 + 1, counts.1)); }
                    else { assert_eq!(after, (counts.0, counts.1 + 1)); }
                }
            }
        }
    }

    fn f9_b_keeps_the_old_chain_on_edit() {
        for (group, section, label, edit) in [
            (SAVES_MACROS, 1, "Corner pressure", "Edit chain"),
            (COMBOS_SEQ, 0, "Corner to kill", "Edit combos"),
        ] {
            let mut m = f9_new_menu(group, section, label);
            let old_steps = m.recording().catalog().macros.iter().find(|f| f.id == "pressure").unwrap().steps.clone();
            let old_combos = m.recording().catalog().sequences[0].combos.clone();
            m.press(Button::A);
            let index = m.recording().file_rows().iter().position(|row| row.title == edit).unwrap();
            m.recording_mut().set_file_cursor(index);
            m.press(Button::A);
            m.press(Button::R);
            m.press(Button::A);
            m.press(Button::R);
            m.press(Button::B);
            assert!(!m.recording().picker_open());
            assert!(!matches!(m.recording_mut().take_effect(), Effect::WriteChainSteps { .. } | Effect::WriteSequenceCombos { .. }));
            assert_eq!(m.recording().catalog().macros.iter().find(|f| f.id == "pressure").unwrap().steps, old_steps);
            assert_eq!(m.recording().catalog().sequences[0].combos, old_combos);
        }
    }

    fn f9_a_cancelled_name_makes_nothing() {
        let mut m = f9_new_menu(SAVES_MACROS, 1, "+ New chain");
        let count = m.recording().catalog().macros.len();
        m.press(Button::A); m.press(Button::A); m.press(Button::X);
        let _ = m.recording_mut().take_effect();
        assert!(m.recording_mut().cancel_name());
        assert!(!m.recording().naming());
        assert_eq!(m.recording().catalog().macros.len(), count);
        m.press(Button::A); m.press(Button::A); m.press(Button::X);
        let _ = m.recording_mut().take_effect();
        m.press(Button::B);
        assert!(!m.recording().naming());
        assert_eq!(m.recording().catalog().macros.len(), count);
        let before = m.recording().section_cursor(Face::Page, SAVES_MACROS, 1);
        m.press(Button::Down);
        assert_ne!(m.recording().section_cursor(Face::Page, SAVES_MACROS, 1), before);
        let old = m.recording().catalog().macros[0].name.clone();
        let index = m.recording().section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == old).unwrap();
        m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 1, index);
        m.press(Button::A);
        let rename = m.recording().file_rows().iter().position(|row| row.title == "Rename").unwrap();
        m.recording_mut().set_file_cursor(rename);
        m.press(Button::A);
        assert!(m.recording().naming());
        assert_eq!(m.recording().hint(Face::Page), "Name it, then confirm. Empty keeps the N name. B cancels.");
        assert!(m.recording_mut().cancel_name());
        assert!(!m.recording_mut().cancel_name());
        assert_eq!(m.recording().catalog().macros[0].name, old);
        m.press(Button::A);
        assert!(m.recording().naming());
        m.press(Button::B);
        assert!(!m.recording().naming());
        assert_eq!(m.recording().catalog().macros[0].name, old);
    }

    fn f9_x_finishes_an_edit() {
        let mut m = f9_new_menu(SAVES_MACROS, 1, "+ New chain");
        let index = m.recording().section(Face::Page, SAVES_MACROS, 1).rows.iter()
            .position(|row| row.name == "Corner pressure").unwrap();
        m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 1, index);
        m.press(Button::A);
        let edit = m.recording().file_rows().iter().position(|row| row.title == "Edit chain").unwrap();
        m.recording_mut().set_file_cursor(edit);
        m.press(Button::A);
        for _ in 0..m.recording().picker_rows().len() { m.press(Button::R); m.press(Button::Down); }
        assert!(m.recording().picker.as_ref().unwrap().values.iter().all(|&value| value == 0));
        m.press(Button::X);
        assert!(matches!(m.recording_mut().take_effect(), Effect::WriteChainSteps { steps, .. } if steps.is_empty()));
    }

    fn f2_recording_page_list_wraps() {
        use crate::model::menu::Entry;
        for group in [SAVES_MACROS, COMBOS_SEQ] {
            for section in [0, 1] {
                let mut m = recording_list_menu();
                for _ in 0..16 {
                    if m.focus() == Some(Entry::Group(usize::from(group))) { break; }
                    m.press(Button::JumpDown);
                }
                assert_eq!(m.focus(), Some(Entry::Group(usize::from(group))));
                let len = m.recording().section(Face::Page, group, section).rows.len();
                assert!(len > 0);
                m.recording_mut().set_cursor(Face::Page, group, section, 0);
                m.press(Button::Up);
                assert_eq!(m.focus(), Some(Entry::Group(usize::from(group))));
                assert_eq!((m.recording().current_section(Face::Page, group),
                    m.recording().section_cursor(Face::Page, group, section)), (section, len - 1));
                m.press(Button::Down);
                assert_eq!(m.focus(), Some(Entry::Group(usize::from(group))));
                assert_eq!((m.recording().current_section(Face::Page, group),
                    m.recording().section_cursor(Face::Page, group, section)), (section, 0));
            }
        }
        let mut m = recording_list_menu();
        let len = m.recording().section(Face::Page, SAVES_MACROS, 0).rows.len();
        m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 0, len - 1);
        m.press(Button::Down);
        assert_eq!(m.focus(), Some(Entry::Group(usize::from(SAVES_MACROS))));
        assert_eq!((m.recording().current_section(Face::Page, SAVES_MACROS),
            m.recording().section_cursor(Face::Page, SAVES_MACROS, 0)), (0, 0));
    }

    fn f2_recording_empty_saves_moves_to_macros() {
        use crate::model::menu::Entry;
        for button in [Button::Up, Button::Down] {
            let mut m = recording_list_menu();
            let mut catalog = long_catalog();
            catalog.saves.clear();
            catalog.hot_saves.clear();
            m.recording_mut().set_catalog(catalog);
            assert_eq!(m.recording().section(Face::Page, SAVES_MACROS, 0).rows.len(), 0);
            m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 0, 0);
            m.press(button);
            assert_eq!(m.focus(), Some(Entry::Group(usize::from(SAVES_MACROS))));
            assert_eq!((m.recording().current_section(Face::Page, SAVES_MACROS),
                m.recording().section_cursor(Face::Page, SAVES_MACROS, 1)), (1, 0));
        }
    }

    fn f2_recording_hold_wraps() {
        use crate::model::adapter::{bit, Adapter};
        let mut m = recording_list_menu();
        let len = m.recording().section(Face::Page, SAVES_MACROS, 1).rows.len();
        m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 1, len - 2);
        let mut adapter = Adapter::new();
        assert_eq!(adapter_poll(&mut m, &mut adapter, 1 << bit::DPAD_DOWN), [Button::Down]);
        for _ in 1..=29 { adapter_poll(&mut m, &mut adapter, 1 << bit::DPAD_DOWN); }
        assert_eq!(m.recording().current_section(Face::Page, SAVES_MACROS), 1);
        assert_eq!(m.recording().section_cursor(Face::Page, SAVES_MACROS, 1), 1);
    }

    fn f2_recording_file_wrap() {
        for section in [0, 1] {
            let mut m = recording_list_menu();
            m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, section, 0);
            m.press(Button::A);
            let len = m.recording().file_rows().len();
            assert!(len > 1);
            assert_eq!(m.recording().file_cursor(), 0);
            m.press(Button::Up);
            assert_eq!(m.recording().file_cursor(), len - 1);
            m.press(Button::Down);
            assert_eq!(m.recording().file_cursor(), 0);
        }
    }

    fn f2_recording_picker_wrap() {
        let mut m = recording_list_menu();
        let last = m.recording().section(Face::Page, SAVES_MACROS, 1).rows.len() - 1;
        m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 1, last);
        m.press(Button::A);
        assert!(m.recording().picker_open());
        let len = m.recording().picker_rows().len();
        assert!(len > 1);
        m.press(Button::Up);
        assert_eq!(m.recording().picker_rows().iter().position(|row| row.cursor), Some(len - 1));
        m.press(Button::Down);
        assert_eq!(m.recording().picker_rows().iter().position(|row| row.cursor), Some(0));
    }

    fn bridge_recording_list_tracks_jumps_and_the_popup() {
        use crate::model::bridge_state::list_view;
        let mut m = menu();
        to_recording(&mut m);
        let mut catalog = example();
        catalog.saves.truncate(14);
        m.recording_mut().set_catalog(catalog);
        m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 0, 0);
        let expected = format!("recording/page/{SAVES_MACROS}/0");
        let view = list_view(&m);
        assert_eq!((view.id.as_str(), view.count, view.index), (expected.as_str(), 14, Some(0)));
        m.press(Button::Down);
        assert_eq!(list_view(&m).index, Some(1));
        m.press(Button::Up);
        m.press(Button::JumpDown);
        assert_eq!(list_view(&m).index, Some(12));
        m.press(Button::JumpDown);
        assert_eq!(list_view(&m).index, Some(13));
        m.press(Button::JumpDown);
        let view = list_view(&m);
        assert_eq!((view.id, view.index), (format!("recording/page/{SAVES_MACROS}/1"), Some(0)));
        m.recording_mut().toggle_popup();
        let popup = list_view(&m);
        assert_eq!(popup.id, format!("recording/popup/{SAVES_MACROS}/0"));
        assert_eq!((popup.count, popup.index), (14, Some(0)));
    }

    fn bridge_publishes_the_popup_owner_and_not_the_panel_or_external_screen() {
        use crate::model::bridge_state::{published_list, ListView};
        use crate::model::flow::{model_takes_presses, Phase};
        use crate::model::menu::{Screen, table::Editor};
        let mut m = menu();
        let mut catalog = example();
        catalog.saves.truncate(14);
        m.recording_mut().set_catalog(catalog);
        m.on_request(Request::RecordingPopup);
        let takes = model_takes_presses(false, true, Phase::Menu, false);
        let view = published_list(Some(&m), takes);
        assert_eq!((view.id, view.count, view.index), (format!("recording/popup/{SAVES_MACROS}/0"), 14, Some(0)));
        m.press(Button::Down);
        assert_eq!(published_list(Some(&m), takes).index, Some(1));
        assert_eq!(published_list(Some(&m), model_takes_presses(true, false, Phase::Panel, false)), ListView::default());
        assert_eq!(published_list(Some(&m), model_takes_presses(true, false, Phase::Menu, true)), ListView::default());
        assert_eq!(published_list(None, true), ListView::default());

        let mut external = menu();
        while external.page() != Page::FightersStages { external.press(Button::R); }
        for _ in 0..=external.entry_count(Page::FightersStages) {
            if external.help_row().is_some_and(|row| row.id == "mii_brawler_config") { break; }
            external.press(Button::Down);
        }
        assert!(external.help_row().is_some_and(|row| row.id == "mii_brawler_config"));
        external.press(Button::A);
        assert!(matches!(external.screen(), Some(Screen::External(Editor::MiiList(0)))));
        assert_eq!(published_list(Some(&external), model_takes_presses(true, false, Phase::Menu, false)), ListView::default());
    }

    fn recording_page_jump_goes_on_past_a_section() {
        use crate::model::menu::Entry;
        let mut m = recording_list_menu();
        let saves = m.recording().section(Face::Page, SAVES_MACROS, 0).rows.len();
        let macros = m.recording().section(Face::Page, SAVES_MACROS, 1).rows.len();
        assert!(saves > 1 && macros > 13);
        m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 0, 0);
        m.press(Button::JumpDown);
        assert_eq!((m.recording().current_section(Face::Page, SAVES_MACROS),
            m.recording().section_cursor(Face::Page, SAVES_MACROS, 0)), (0, saves - 1));
        m.press(Button::JumpDown);
        assert_eq!((m.recording().current_section(Face::Page, SAVES_MACROS),
            m.recording().section_cursor(Face::Page, SAVES_MACROS, 1)), (1, 0));
        let mut expected = 0;
        while expected < macros - 1 {
            expected = (expected + 12).min(macros - 1);
            m.press(Button::JumpDown);
            assert_eq!(m.recording().section_cursor(Face::Page, SAVES_MACROS, 1), expected);
        }
        m.press(Button::JumpDown);
        assert_eq!(m.focus(), Some(Entry::Group(usize::from(COMBOS_SEQ))));
        let mut m = recording_list_menu();
        m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 1, 0);
        m.press(Button::JumpUp);
        assert_eq!((m.recording().current_section(Face::Page, SAVES_MACROS),
            m.recording().section_cursor(Face::Page, SAVES_MACROS, 0)), (0, saves - 1));
        m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 0, 0);
        let last_group = match m.entry(Page::Recording, m.entry_count(Page::Recording) - 1) {
            Some(Entry::ResetPage(group)) => group,
            other => panic!("last entry: {other:?}"),
        };
        let first = (0..m.entry_count(Page::Recording)).filter_map(|i| m.entry(Page::Recording, i))
            .find(|entry| matches!(entry, Entry::Row(row) if usize::from(ROWS[*row].group) == last_group))
            .expect("last group's first focusable row");
        m.press(Button::JumpUp);
        assert_eq!(m.focus(), Some(first));
    }

    fn recording_picker_takes_the_list_jump() {
        let mut m = recording_list_menu();
        let last = m.recording().section(Face::Page, SAVES_MACROS, 1).rows.len() - 1;
        m.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 1, last);
        assert!(m.press_outcome(Button::A));
        assert!(m.recording().picker_open());
        let len = m.recording().picker_rows().len();
        assert!(len > 13);
        let cursor = |m: &Menu| m.recording().picker_rows().iter().position(|r| r.cursor).unwrap();
        assert_eq!(cursor(&m), 0);
        let mut expected = 0;
        while expected < len - 1 {
            expected = (expected + 12).min(len - 1);
            assert!(m.press_outcome(Button::JumpDown));
            assert_eq!(cursor(&m), expected);
        }
        assert!(m.press_outcome(Button::JumpDown));
        assert_eq!(cursor(&m), 0);
        assert!(m.press_outcome(Button::JumpUp));
        assert_eq!(cursor(&m), len - 1);
        while expected > 0 {
            expected = expected.saturating_sub(12);
            assert!(m.press_outcome(Button::JumpUp));
            assert_eq!(cursor(&m), expected);
        }
        assert!(m.press_outcome(Button::JumpUp));
        assert_eq!(cursor(&m), len - 1);
    }

    fn zl_down_reaches_the_recording_lists_through_the_adapter() {
        use crate::model::adapter::{bit, Adapter};
        for face in [Face::Page, Face::Popup] {
            let mut m = recording_list_menu();
            if face == Face::Popup { m.on_request(Request::RecordingPopup); }
            m.recording_mut().set_cursor(face, SAVES_MACROS, 1, 0);
            let mut a = Adapter::new();
            assert_eq!(adapter_poll(&mut m, &mut a, 1 << bit::ZL), []);
            assert_eq!(adapter_poll(&mut m, &mut a, (1 << bit::ZL) | (1 << bit::DPAD_DOWN)), [Button::JumpDown]);
            assert_eq!(m.recording().section_cursor(face, SAVES_MACROS, 1), 12, "{face:?}");
        }
    }

    fn adapter_poll(menu: &mut Menu, adapter: &mut crate::model::adapter::Adapter, bits: u32) -> Vec<Button> {
        use crate::model::adapter::Style;
        use crate::model::menu::chord::Chord;
        adapter.set_jumps(menu.takes_jumps());
        let presses: Vec<_> = adapter.poll(bits, Style::Pro, true, true, [Chord::NONE; 2]).presses().collect();
        for button in &presses { menu.press(*button); }
        presses
    }

    fn holding_down_on_the_recording_lists_quickens() {
        use crate::model::adapter::{bit, Adapter};
        for face in [Face::Page, Face::Popup] {
            let mut m = recording_list_menu();
            if face == Face::Popup { m.on_request(Request::RecordingPopup); }
            m.recording_mut().set_cursor(face, SAVES_MACROS, 1, 0);
            assert!(m.recording().section(face, SAVES_MACROS, 1).rows.len() >= 40);
            let mut a = Adapter::new();
            assert_eq!(adapter_poll(&mut m, &mut a, 1 << bit::DPAD_DOWN), [Button::Down]);
            for _ in 1..=120 { adapter_poll(&mut m, &mut a, 1 << bit::DPAD_DOWN); }
            assert_eq!(m.recording().section_cursor(face, SAVES_MACROS, 1), 19, "{face:?}");
        }
    }

    fn x_cycles_the_loop_only_in_macros() {
        let mut screens = screens();
        open_popup(&mut screens);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 1, 0);
        assert_eq!(screens.loop_of(ViewRole::Player), LoopMark::Finite(1));
        for (presses, mark) in [(1, LoopMark::Finite(2)), (2, LoopMark::Finite(3)), (3, LoopMark::Finite(5)), (4, LoopMark::Finite(10)), (5, LoopMark::Infinite), (6, LoopMark::Finite(1))] {
            let mut step = screens.clone();
            for _ in 0..presses {
                step.press_popup(Button::X);
            }
            assert_eq!(step.loop_of(ViewRole::Player), mark);
        }
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 0);
        screens.press_popup(Button::X);
        assert_eq!(screens.loop_of(ViewRole::Player), LoopMark::Finite(1));
        screens.press_popup(Button::ZL);
        screens.press_popup(Button::X);
        assert_eq!(screens.loop_of(ViewRole::Player), LoopMark::Finite(1));
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, 0);
        screens.handle_page(Button::X, true, SAVES_MACROS);
        assert_eq!(screens.loop_of(ViewRole::Player), LoopMark::Finite(2));
        screens.handle_page(Button::A, true, SAVES_MACROS);
        let during_file = screens.loop_of(ViewRole::Player);
        screens.handle_page(Button::X, true, SAVES_MACROS);
        assert_eq!(screens.loop_of(ViewRole::Player), during_file);

        let mut menu = menu();
        to_recording(&mut menu);
        let mash = ROWS.iter().position(|row| row.id == "playback_mash").unwrap();
        for _ in 0..200 {
            if menu.focused_row() == Some(mash) { break; }
            menu.press(if menu.focused_row().is_none() { Button::JumpDown } else { Button::Down });
        }
        assert_eq!(menu.focused_row(), Some(mash));
        menu.press(Button::A);
        let changed = menu.settings().menu.get("playback_mash").unwrap().to_vec();
        assert_ne!(menu.settings().defaults.get("playback_mash").unwrap(), changed.as_slice());
        menu.press(Button::X);
        assert_ne!(menu.settings().defaults.get("playback_mash").unwrap(), changed.as_slice(), "X does not save defaults");
        menu.press(Button::L);
        menu.press(Button::L);
        menu.press(Button::X);
        assert_eq!(menu.settings().defaults.get("playback_mash").unwrap(), changed.as_slice());
    }

    /// rc.12 TR1 (operator 2026-11-13, "Fix them, it should work"): a macro's page has a Playback slot row (not a
    /// chain's). Left/Right step None, Slot 1 .. 5 and round, A steps on; a macro moved onto a slot takes it from the
    /// macro that held it; Delete empties the deleted macro's slot. The selection carries the slots to the index.
    fn a_macro_plays_as_a_playback_slot() {
        fn open(screens: &mut RecordingScreens, name: &str) {
            screens.file = None;
            let at = screens.section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == name).unwrap();
            screens.set_cursor(Face::Page, SAVES_MACROS, 1, at);
            screens.handle_page(Button::A, true, SAVES_MACROS);
        }
        fn slot_row(screens: &mut RecordingScreens) -> String {
            let row = screens.file_rows().iter().position(|row| row.title == "Playback slot").expect("a Playback slot row");
            screens.set_file_cursor(row);
            screens.file_rows()[row].value.clone()
        }
        let mut screens = screens();
        open(&mut screens, "Dash grab");
        assert_eq!(slot_row(&mut screens), "None");
        screens.handle_page(Button::Right, true, SAVES_MACROS);
        screens.handle_page(Button::Right, true, SAVES_MACROS);
        assert_eq!(slot_row(&mut screens), "Slot 2");
        screens.handle_page(Button::Left, true, SAVES_MACROS);
        assert_eq!(slot_row(&mut screens), "Slot 1");
        screens.handle_page(Button::Left, true, SAVES_MACROS);
        screens.handle_page(Button::Left, true, SAVES_MACROS);
        assert_eq!(slot_row(&mut screens), "Slot 5", "Left from None goes round to Slot 5");
        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert_eq!(slot_row(&mut screens), "None", "A steps on, round to None");
        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert_eq!(screens.selection().slots, [Some("dash".to_string()), None, None, None, None]);
        open(&mut screens, "Nair loop");
        slot_row(&mut screens);
        screens.handle_page(Button::Right, true, SAVES_MACROS);
        assert_eq!(screens.selection().slots[0].as_deref(), Some("nair"), "Nair loop takes Slot 1 from Dash grab");
        assert!(!screens.selection().slots.iter().any(|held| held.as_deref() == Some("dash")));
        screens.handle_page(Button::Right, true, SAVES_MACROS);
        open(&mut screens, "Dash grab");
        slot_row(&mut screens);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert_eq!(screens.selection().slots, [Some("dash".to_string()), Some("nair".to_string()), None, None, None]);
        screens.delete_file(ItemKind::Recording, "nair");
        assert_eq!(screens.selection().slots, [Some("dash".to_string()), None, None, None, None], "Delete empties its slot");
        open(&mut screens, "Corner pressure");
        assert!(!screens.file_rows().iter().any(|row| row.title == "Playback slot"), "a chain has no slot");
    }

    fn set_active_is_one_row_for_the_role_in_view() {
        let mut screens = screens();
        let dash = screens.section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == "Dash grab").unwrap();
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, dash);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        let titles: Vec<_> = screens.file_rows().iter().map(|row| row.title.clone()).collect();
        assert_eq!(titles.iter().filter(|title| title.as_str() == "Set active").count(), 1);
        let row = screens.file_rows().iter().position(|row| row.title == "Set active").unwrap();
        screens.set_file_cursor(row);
        screens.handle_page(Button::Right, true, SAVES_MACROS);
        assert_eq!(screens.file_rows().iter().find(|row| row.title == "Set active").unwrap().value, "On · Player 1");
        assert_eq!(screens.selection().active_player.as_deref(), Some("dash"));
        assert_eq!(screens.selection().active_cpu.as_deref(), Some("pressure"));
        screens.handle_page(Button::Left, true, SAVES_MACROS);
        assert_eq!(screens.selection().active_player, None);
        screens.handle_page(Button::Left, true, SAVES_MACROS);
        assert_eq!(screens.selection().active_cpu.as_deref(), Some("pressure"));
    }

    fn retired_rows_leave_the_menu_and_old_settings_still_load() {
        for id in ["recording_slot", "record_trigger", "recording_duration", "playback_button_slots", "save_state_playback", "macro_source", "save_state_slot", "randomize_slots"] {
            assert!(ROWS.iter().all(|row| row.id != id), "{id} left the menu");
            assert!(settings::def(id).is_some(), "{id} stays in the factory");
        }
        for title in ["Named library", "Save slot to library...", "Record Slot", "Randomize Slots"] {
            assert!(ROWS.iter().all(|row| row.title != title), "{title}");
        }
        let loaded = settings::parse(r#"{"menu":{"recording_slot":[0,1,0,0,0]}}"#);
        assert_eq!(loaded.settings.menu.get("recording_slot"), Some(&[0, 1, 0, 0, 0][..]));
    }

    fn the_combo_panel_lists_each_links_events_and_drift() {
        use crate::model::combo::{retain, Dir, Event, Flick, Kind};
        use crate::model::frame_strip::Button;
        let nair = |drift| {
            let mut link = LinkRec::presses(Move::Aerial, Move::Aerial, Verdict::True(None), None, &[]);
            link.presses_only = false;
            link.events = retain([
                Event::press(Button::Jump, crate::model::combo::Aim::Neutral, 0),
                Event { kind: Kind::Release(Button::Jump), dir: Dir::Neutral, offset: 3 },
                Event { kind: Kind::Flick(Flick::FastFall), dir: Dir::Down, offset: 8 },
                Event::press(Button::Attack, crate::model::combo::Aim::Neutral, 10),
            ]);
            link.drift = drift;
            link
        };
        let combo = Combo::from_links([nair(Some(Dir::Forward)), nair(Some(Dir::Back))]).unwrap();
        let lines = combo_link_lines(&combo, crate::model::move_names::Namer::OFF);
        assert_eq!(lines[0], "L1 aerial → aerial TRUE");
        assert_eq!(lines[1], "jump · release · ↓ flick · A");
        assert_eq!(lines[2], "drift →");
        assert_eq!(lines[3], "L2 aerial → aerial TRUE");
        assert_eq!(lines[4], "jump · release · ↓ flick · A");
        assert_eq!(lines[5], "drift ←");
        let mut screens = screens();
        screens.set_combo("nairc", combo);
        // Row 0 of COMBOS is + Save last combo…; the combo is row 1.
        screens.set_cursor(Face::Page, COMBOS_SEQ, 1, 1);
        let panel = screens.connections(Face::Page, COMBOS_SEQ);
        let text: Vec<&str> = panel.iter().map(|line| line.text.as_str()).collect();
        assert!(text.contains(&"jump · release · ↓ flick · A"));
        assert!(text.contains(&"drift →"));
        assert!(text.contains(&"drift ←"));

        let old = Combo::from_links([LinkRec::presses(
            Move::Aerial,
            Move::Aerial,
            Verdict::True(None),
            None,
            &[
                Event::press(Button::Attack, crate::model::combo::Aim::Side, 1),
                Event::press(Button::Jump, crate::model::combo::Aim::Up, 4),
            ],
        )])
        .unwrap();
        let old_lines = combo_link_lines(&old, crate::model::move_names::Namer::OFF);
        assert_eq!(old_lines, vec!["L1 aerial → aerial TRUE".to_string(), "A · jump".to_string()]);
        assert!(old_lines.iter().all(|line| !line.contains("drift") && !line.contains("release") && !line.contains("flick")));
    }

    fn the_floppy_is_grey_when_the_switch_is_off() {
        let mut screens = screens();
        let row = screens.section(Face::Page, SAVES_MACROS, 1).rows.into_iter().find(|row| row.name == "Nair loop").unwrap();
        assert_eq!(row.floppy, Some(Floppy::On));
        assert_eq!(Floppy::On.color(), [242, 152, 72, 255]);
        let index = screens.section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == "Nair loop").unwrap();
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, index);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        screens.handle_page(Button::Left, true, SAVES_MACROS);
        screens.handle_page(Button::B, true, SAVES_MACROS);
        let row = screens.section(Face::Page, SAVES_MACROS, 1).rows.into_iter().find(|row| row.name == "Nair loop").unwrap();
        assert_eq!(row.floppy, Some(Floppy::Off));
        assert_eq!(row.floppy.unwrap().color(), [126, 132, 144, 255]);
        assert_eq!(screens.start_save_id("nair"), Some("nair-start"));
        assert_eq!(screens.section(Face::Page, SAVES_MACROS, 0).header_floppy, Some(Floppy::On));
    }

    fn y_in_the_popup_new_chain_picker_is_ignored_and_a_picker_press_is_handled() {
        let mut screens = screens();
        open_popup(&mut screens);
        let last = screens.section(Face::Popup, SAVES_MACROS, 1).rows.len() - 1;
        assert_eq!(screens.section(Face::Popup, SAVES_MACROS, 1).rows[last].name, "+ New chain");
        screens.set_cursor(Face::Popup, SAVES_MACROS, 1, last);
        assert!(screens.press_popup_outcome(Button::A), "A opens the picker");
        assert!(screens.picker_open());
        assert!(!screens.press_popup_outcome(Button::Y), "the picker does nothing with Y");
        assert!(screens.picker_open());
        assert!(screens.press_popup_outcome(Button::A), "A on a picker row toggles it");
    }

    fn new_chain_and_new_sequence_open_the_picker() {
        let mut screens = screens();
        let last = screens.section(Face::Page, SAVES_MACROS, 1).rows.len() - 1;
        assert_eq!(screens.section(Face::Page, SAVES_MACROS, 1).rows[last].name, "+ New chain");
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, last);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert!(screens.picker_open());
        assert!(screens.picker_recording_ids().iter().all(|id| id != "pressure"), "a chain is not a step");
        screens.handle_page(Button::X, true, SAVES_MACROS);
        assert!(matches!(screens.last_effect(), Effect::Keyboard { header: "Name chain", .. }));
        screens.submit_name("");
        assert!(screens.has_chain_named("Chain 1"));

        let last = screens.section(Face::Page, COMBOS_SEQ, 0).rows.len() - 1;
        assert_eq!(screens.section(Face::Page, COMBOS_SEQ, 0).rows[last].name, "+ New sequence");
        screens.set_cursor(Face::Page, COMBOS_SEQ, 0, last);
        screens.handle_page(Button::A, true, COMBOS_SEQ);
        assert!(screens.picker_open());
    }

    fn crop_actions_open_the_shared_clip_editor() {
        let mut screens = screens();
        let row = screens.section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == "Nair loop").unwrap();
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, row);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        let crop = screens.file_rows().iter().position(|row| row.title == "Crop").unwrap();
        screens.set_file_cursor(crop);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert_eq!(screens.take_effect(), Effect::OpenCrop { source: "nair".into(), combo: false, player: true });
        screens.handle_page(Button::B, true, SAVES_MACROS);
        screens.handle_page(Button::ZR, true, SAVES_MACROS);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        screens.set_file_cursor(crop);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert_eq!(screens.take_effect(), Effect::OpenCrop { source: "nair".into(), combo: false, player: false });
        // The same action dispatches the combo, without an intermediate editor or write.
        screens.open_crop(ItemKind::Combo, "combo".into());
        assert_eq!(screens.take_effect(), Effect::OpenCrop { source: "combo".into(), combo: true, player: false });
        assert!(screens.catalog().macros.iter().any(|file| file.id == "nair" && file.name == "Nair loop"));
    }

    fn recording_popup_request_opens_the_popup() {
        let mut menu = menu();
        assert!(!menu.recording().popup_open());
        menu.on_request(Request::RecordingPopup);
        assert!(menu.recording().popup_open());
        menu.on_request(Request::RecordingPopup);
        assert!(!menu.recording().popup_open());
        menu.on_request(Request::PlayPlayer);
        assert!(!menu.recording().popup_open());
    }

    // ---- rc.6: sequences are drilled, edited and deleted on the card ------------------------------------------------

    /// The example catalog with a second combo, Ryu BnB, first in Corner to kill.
    fn two_combo_screens() -> RecordingScreens {
        let mut screens = screens();
        screens.catalog.combos.insert(
            0,
            ComboFile { input_only: false, take: None,
                id: "ryu".into(),
                name: "Ryu BnB".into(),
                recorded_on: String::new(),
                made_from: None,
                combo: Box::new(Combo::from_links([link(Move::Jab, Move::Aerial, true)]).unwrap()),
                fighter: None,
                span: 1,
            },
        );
        screens.catalog.sequences[0].combos = vec!["ryu".into(), "nairc".into()];
        screens
    }

    fn open_sequence(screens: &mut RecordingScreens) {
        let row = screens.section(Face::Page, COMBOS_SEQ, 0).rows.iter().position(|row| row.name == "Corner to kill").unwrap();
        screens.set_cursor(Face::Page, COMBOS_SEQ, 0, row);
        screens.handle_page(Button::A, true, COMBOS_SEQ);
        assert!(screens.file_open());
    }

    fn press_file_row(screens: &mut RecordingScreens, title: &str) {
        let row = screens.file_rows().iter().position(|row| row.title == title).unwrap_or_else(|| panic!("no {title:?}"));
        screens.set_file_cursor(row);
        screens.handle_page(Button::A, true, COMBOS_SEQ);
    }

    fn a_numbered_row_drills_that_combo_and_drill_sequence_drills_the_sequence() {
        let mut screens = two_combo_screens();
        open_sequence(&mut screens);
        let titles: Vec<_> = screens.file_rows().into_iter().map(|row| row.title).collect();
        assert_eq!(&titles[..3], ["Drill sequence", "1 Ryu BnB", "2 Nair loop combo"]);
        press_file_row(&mut screens, "2 Nair loop combo");
        assert_eq!(screens.selection().drill.as_deref(), Some("nairc"), "the combo alone is the drill");
        assert_eq!(screens.catalog().sequences[0].combos, ["ryu", "nairc"], "the sequence stays");
        assert!(matches!(screens.last_effect(), Effect::None), "nothing is written to the sequence");
        press_file_row(&mut screens, "Drill sequence");
        assert_eq!(screens.selection().drill.as_deref(), Some("corner-seq"), "the sequence replaces the combo");
    }

    fn edit_combos_writes_the_new_order_and_r_removes_the_last_number() {
        let mut screens = two_combo_screens();
        open_sequence(&mut screens);
        press_file_row(&mut screens, "Edit combos");
        assert!(screens.picker_open());
        let ranks = |s: &RecordingScreens| s.picker_rows().iter().map(|row| (row.name.clone(), row.rank)).collect::<Vec<_>>();
        assert_eq!(ranks(&screens), [("Ryu BnB".to_string(), 1), ("Nair loop combo".to_string(), 2)]);
        // R on Ryu BnB takes its number off; Nair loop combo closes up to 1. A gives Ryu BnB the next number.
        screens.handle_page(Button::R, true, COMBOS_SEQ);
        assert_eq!(ranks(&screens), [("Ryu BnB".to_string(), 0), ("Nair loop combo".to_string(), 1)]);
        screens.handle_page(Button::A, true, COMBOS_SEQ);
        screens.handle_page(Button::X, true, COMBOS_SEQ);
        assert_eq!(
            screens.last_effect(),
            &Effect::WriteSequenceCombos { id: "corner-seq".into(), combos: vec!["nairc".into(), "ryu".into()] }
        );
        assert_eq!(screens.catalog().sequences[0].combos, ["nairc", "ryu"]);
        // R on every number leaves an empty sequence, which drills nothing. It is still written.
        let _ = screens.take_effect();
        press_file_row(&mut screens, "Edit combos");
        screens.handle_page(Button::R, true, COMBOS_SEQ);
        screens.handle_page(Button::Down, true, COMBOS_SEQ);
        screens.handle_page(Button::R, true, COMBOS_SEQ);
        assert!(ranks(&screens).iter().all(|(_, rank)| *rank == 0), "R removed the last number");
        screens.handle_page(Button::X, true, COMBOS_SEQ);
        assert_eq!(screens.last_effect(), &Effect::WriteSequenceCombos { id: "corner-seq".into(), combos: vec![] });
    }

    fn delete_posts_the_file_for_the_host_to_remove() {
        let mut screens = two_combo_screens();
        open_sequence(&mut screens);
        screens.drill = Some("corner-seq".into());
        press_file_row(&mut screens, "Delete");
        assert_eq!(screens.last_effect(), &Effect::DeleteFile { kind: FileKind::Sequence, id: "corner-seq".into() });
        assert!(screens.catalog().sequences.is_empty());
        assert_eq!(screens.selection().drill, None, "a deleted drill is no longer drilled");
        let row = screens.section(Face::Page, COMBOS_SEQ, 1).rows.iter().position(|row| row.name == "Ryu BnB").unwrap();
        screens.set_cursor(Face::Page, COMBOS_SEQ, 1, row);
        screens.handle_page(Button::A, true, COMBOS_SEQ);
        press_file_row(&mut screens, "Delete");
        assert_eq!(screens.last_effect(), &Effect::DeleteFile { kind: FileKind::Combo, id: "ryu".into() });
        let chain = screens.section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == "Corner pressure").unwrap();
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, chain);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        let delete = screens.file_rows().iter().position(|row| row.title == "Delete").unwrap();
        screens.set_file_cursor(delete);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert_eq!(screens.last_effect(), &Effect::DeleteFile { kind: FileKind::Chain, id: "pressure".into() });
    }

    /// Primary 2026-11-08 15:55Z: the r8 pop-up as drawn. Its CONNECTIONS are kind boxes in a tree
    /// (`RECORDING-REDESIGN-r8-popup-combo.png`, `-macro.png`, `-save.png`): the row under the cursor, a heading, the
    /// files joined to it, then plain lines.
    fn the_popup_connections_are_the_r8_boxes() {
        use super::{ConnItem as I, ConnKind as K, FileTag as T};
        let item = |kind, text: &str| I { kind, text: text.to_string() };
        let mut screens = screens();
        screens.set_cursor(Face::Popup, COMBOS_SEQ, 1, 1);
        let combo = screens.connection_items(Face::Popup, COMBOS_SEQ);
        assert_eq!(
            combo[..6],
            [
                item(K::Head(T::Combo), "Nair loop combo"),
                item(K::Heading, "made from"),
                item(K::Child(T::Macro), "Nair loop"),
                item(K::Heading, "in sequences"),
                item(K::Child(T::Sequence), "Corner to kill #1"),
                item(K::Heading, "Its links (1)"),
            ]
        );
        assert_eq!(combo[6].kind, K::Link, "{combo:?}");
        assert!(combo[6..].iter().all(|line| matches!(line.kind, K::Link | K::LinkSub)), "{combo:?}");

        screens.set_cursor(Face::Popup, SAVES_MACROS, 1, 0);
        assert_eq!(
            screens.connection_items(Face::Popup, SAVES_MACROS),
            [
                item(K::Head(T::Macro), "Nair loop"),
                item(K::Heading, "starts from (warps there once, first)"),
                item(K::Child(T::Save), "Nair loop · start · On"),
                item(K::Heading, "in chains"),
                item(K::Child(T::Macro), "Corner pressure as 1"),
                item(K::Heading, "combos made from it"),
                item(K::Child(T::Combo), "Nair loop combo"),
                item(K::Detail, "Recorded on  Pikachu · 0.2 s · information only"),
            ]
        );

        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 2);
        assert_eq!(
            screens.connection_items(Face::Popup, SAVES_MACROS),
            [
                item(K::Head(T::Save), "Nair loop · start"),
                item(K::Heading, "start save of"),
                item(K::Child(T::Macro), "Nair loop"),
                item(K::Child(T::Macro), "Corner pressure"),
                item(K::Detail, "Bound to Save / Load  no"),
            ]
        );

        screens.set_cursor(Face::Popup, COMBOS_SEQ, 0, 0);
        assert_eq!(
            screens.connection_items(Face::Popup, COMBOS_SEQ),
            [
                item(K::Head(T::Sequence), "Corner to kill"),
                item(K::Heading, "combos"),
                item(K::Child(T::Combo), "1 Nair loop combo"),
                item(K::Detail, "streak 2 · best 6"),
            ]
        );
    }

    /// The typed name goes to the host with the save; an empty name lets the host pick Combo N.
    fn save_last_combo_posts_the_typed_name_or_combo_n() {
        let mut screens = screens();
        screens.set_cursor(Face::Page, COMBOS_SEQ, 1, 0);
        screens.handle_page(Button::A, true, COMBOS_SEQ);
        let _ = screens.take_effect();
        screens.submit_name("  Ryu BnB ");
        assert_eq!(screens.take_effect(), Effect::SaveLastCombo { name: "Ryu BnB".into() });
        screens.handle_page(Button::A, true, COMBOS_SEQ);
        let _ = screens.take_effect();
        screens.submit_name("");
        assert_eq!(screens.take_effect(), Effect::SaveLastCombo { name: String::new() });
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    fn drills_are_a_filtered_combo_sublist_on_both_faces() {
        for face in [Face::Page,Face::Popup] {
            let mut screens=RecordingScreens::new(); let mut cat=example(); let mut drill=cat.combos[0].clone();
            drill.id="input-drill".into(); drill.name="Movement".into(); drill.input_only=true; cat.combos.push(drill); screens.set_catalog(cat);
            screens.popup_open=true; screens.popup_tab=1;
            let root=screens.section(face,COMBOS_SEQ,1); assert!(root.rows.iter().any(|r| r.name=="DRILLS >")); assert!(!root.rows.iter().any(|r| r.name=="Movement"));
            screens.place_mut(face,COMBOS_SEQ).at[1]=2; screens.toggle_drills(face,COMBOS_SEQ);
            let drills=screens.section(face,COMBOS_SEQ,1); assert_eq!((drills.header,drills.files),("DRILLS",1));
            assert_eq!(drills.rows[0].name,"+ Record drill…"); assert_eq!(drills.rows[1].name,"Movement");
            assert!(!drills.rows[0].hot && !drills.rows[0].green);
            if face==Face::Page { assert_eq!(screens.handle_page(Button::B,true,COMBOS_SEQ),PageHit::Consumed); } else { screens.press_popup(Button::B); assert!(screens.popup_open); }
            assert!(!screens.place(face,COMBOS_SEQ).drills); assert_eq!(screens.place(face,COMBOS_SEQ).at[1],2);
            screens.toggle_drills(face,COMBOS_SEQ); screens.place_mut(face,COMBOS_SEQ).at[1]=0;
            if face==Face::Page { assert_eq!(screens.handle_page(Button::A,true,COMBOS_SEQ),PageHit::Close); } else { screens.press_popup(Button::A); assert!(!screens.popup_open); }
            assert_eq!(screens.take_effect(),Effect::RecordDrill);
            screens.open_picker(PickerKind::NewSequence,Face::Page,None); assert!(screens.picker.as_ref().unwrap().ids.iter().all(|id| id!="input-drill"));
        }
    }

    fn bridge_drills_menu(face: Face) -> Menu {
        let mut m = f9_new_menu(COMBOS_SEQ, 1, "DRILLS >");
        let mut catalog = example();
        let mut drill = catalog.combos[0].clone();
        drill.id = "input-drill".into();
        drill.name = "Movement".into();
        drill.input_only = true;
        catalog.combos.push(drill);
        m.recording_mut().set_catalog(catalog);
        if face == Face::Popup {
            m.on_request(Request::RecordingPopup);
            m.press(Button::ZL);
            let rows = m.recording().section(face, COMBOS_SEQ, 1).rows;
            let at = rows.iter().position(|row| row.name == "DRILLS >").unwrap();
            m.recording_mut().set_cursor(face, COMBOS_SEQ, 1, at);
        }
        m
    }

    fn the_drills_sublist_is_its_own_list_one_level_down_on_both_faces() {
        use crate::model::bridge_state::list_view;
        for face in [Face::Page, Face::Popup] {
            let mut m = bridge_drills_menu(face);
            let name = if face == Face::Popup { "popup" } else { "page" };
            let outer = format!("recording/{name}/{COMBOS_SEQ}/1");
            let before = list_view(&m);
            assert_eq!((m.depth(), before.id.as_str()), (0, outer.as_str()));
            m.press(Button::A);
            let drills = list_view(&m);
            assert_eq!((m.depth(), drills.id.as_str(), drills.index, drills.count),
                (1, format!("{outer}/drills").as_str(), Some(0), 2));
            m.press(Button::Down);
            assert_eq!(list_view(&m).index, Some(1));
            m.press(Button::B);
            let after = list_view(&m);
            assert_eq!((m.depth(), after.id, after.index), (0, outer, before.index));
            assert!(!m.is_closed());
            if face == Face::Popup { assert!(m.recording().popup_open()); }
        }
    }

    fn a_recording_screen_with_no_row_publishes_no_list_not_the_page_under_it() {
        use crate::model::bridge_state::{list_view, ListView};
        for (group, section, label) in [
            (SAVES_MACROS, 1, "+ New chain"), (COMBOS_SEQ, 0, "+ New sequence"),
        ] {
            let mut m = f9_new_menu(group, section, label);
            let mut catalog = long_catalog();
            if group == SAVES_MACROS {
                catalog.macros.clear();
            } else {
                catalog.combos.clear();
            }
            m.recording_mut().set_catalog(catalog);
            let rows = m.recording().section(Face::Page, group, section).rows;
            let index = rows.iter().position(|row| row.name == label).unwrap();
            m.recording_mut().set_cursor(Face::Page, group, section, index);
            let id = format!("recording/page/{group}/{section}");
            assert_eq!((m.depth(), list_view(&m).id), (0, id.clone()));
            m.press(Button::A);
            assert!(m.recording().picker_open());
            assert_eq!(list_view(&m), ListView::default(), "{label}");
            assert_eq!(m.depth(), 1);
            m.press(Button::B);
            assert!(!m.recording().picker_open());
            assert_eq!((m.depth(), list_view(&m).id), (0, id));
            assert!(!m.is_closed());
        }

        let mut keyboard = f9_new_menu(SAVES_MACROS, 1, "+ New chain");
        keyboard.press(Button::A);
        assert_eq!((keyboard.depth(), list_view(&keyboard).id), (1, "recording/picker".into()));
        keyboard.press(Button::A);
        keyboard.press(Button::X);
        assert!(keyboard.recording().naming());
        assert!(!keyboard.recording().picker_open());
        assert_eq!(list_view(&keyboard), ListView::default());
        assert_eq!(keyboard.depth(), 1);
        keyboard.press(Button::B);
        assert!(!keyboard.recording().naming());
        assert_eq!((keyboard.depth(), list_view(&keyboard).id), (0, format!("recording/page/{SAVES_MACROS}/1")));
        assert!(!keyboard.is_closed());

        let mut file = f9_new_menu(SAVES_MACROS, 0, "Corner");
        file.press(Button::A);
        assert_eq!((file.depth(), list_view(&file).id), (1, "recording/file".into()));
        file.recording_mut().set_loading();
        assert!(file.recording().page_overlay_open());
        assert_eq!(list_view(&file), ListView::default());
        assert_eq!(file.depth(), 1);
        file.press(Button::B);
        assert!(!file.recording().page_overlay_open());
        assert_eq!(file.depth(), 0);
        assert!(!file.is_closed());

        let mut ordinary = recording_list_menu();
        for _ in 0..500 {
            if ordinary.help_row().is_some_and(|row| row.id == "recording_crop") { break; }
            ordinary.press(if list_view(&ordinary).id.starts_with("recording/page/") { Button::JumpDown } else { Button::Down });
        }
        assert!(ordinary.help_row().is_some_and(|row| row.id == "recording_crop"));
        let page = list_view(&ordinary);
        assert_eq!(page.id, "page");
        assert!(page.index.is_some());
        assert_eq!(ordinary.depth(), 0);
        assert!(!ordinary.recording().page_overlay_open());
    }

    fn every_b_that_goes_back_takes_one_off_depth_on_the_recording_lists() {
        use crate::model::bridge_state::list_view;
        use std::collections::BTreeSet;
        use std::io::Write;
        let buttons = [Button::A, Button::B, Button::Up, Button::Down, Button::Left, Button::Right,
            Button::JumpDown, Button::JumpUp, Button::X, Button::ZR];
        for face in [Face::Page, Face::Popup] {
            let mut m = bridge_drills_menu(face);
            let prefix: &[Button] = if face == Face::Page {
                &[Button::A, Button::Down, Button::A, Button::B, Button::Left, Button::A,
                  Button::X, Button::B, Button::B]
            } else {
                &[Button::A, Button::Left, Button::Down, Button::A, Button::X, Button::B, Button::B]
            };
            let mut seed = 0x7a19_42d3u32;
            let mut ids = BTreeSet::new();
            let mut deepest = 0;
            let mut keyboard = false;
            let mut history = Vec::new();
            for step in 0..6000 {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let button = prefix.get(step).copied().unwrap_or(buttons[((seed >> 16) as usize) % buttons.len()]);
                history.push(button);
                m.press(button);
                m.recording_mut().take_effect();
                if m.is_closed() || (face == Face::Popup && !m.recording().popup_open()) {
                    m = bridge_drills_menu(face);
                    history.clear();
                    continue;
                }
                if m.recording().popup_open() || (m.page() == Page::Recording && m.screen().is_none()) {
                    let id = list_view(&m).id;
                    assert!(!(m.recording().page_overlay_open() && id == "page"), "{face:?} {history:?}");
                    if !id.is_empty() { ids.insert(id); }
                    keyboard |= m.recording().naming();
                    deepest = deepest.max(m.depth());
                    let mut copy = m.clone();
                    let depth = copy.depth();
                    for remaining in (0..depth).rev() {
                        copy.press(Button::B);
                        assert_eq!(copy.depth(), remaining, "{face:?} {history:?}");
                        assert!(!copy.is_closed(), "{face:?} {history:?}");
                        if face == Face::Popup { assert!(copy.recording().popup_open(), "{history:?}"); }
                    }
                    copy.press(Button::B);
                    if face == Face::Popup {
                        assert!(!copy.recording().popup_open(), "{face:?} {history:?}");
                    } else {
                        assert!(copy.is_closed(), "{face:?} {history:?}");
                    }
                }
            }
            writeln!(std::io::stderr(), "recording walk {face:?}: ids={ids:?} depth={deepest} keyboard={keyboard}").unwrap();
            assert!(ids.contains("recording/picker"), "{face:?}: {ids:?}");
            if face == Face::Page { assert!(ids.contains("recording/file"), "{ids:?}"); }
            assert!(ids.iter().any(|id| id.ends_with("/drills")), "{face:?}: {ids:?}");
            assert!(keyboard && deepest >= 2, "{face:?}: keyboard={keyboard}, depth={deepest}");
        }
    }

    /// Review 2 (notice-compact R2): the real empty-library pop-up's compact background holds every element the view
    /// draws in a pop-up: the role key caps, the row band, the CONNECTIONS divider, and the help and hint lines.
    fn popup_background_holds_its_chrome() {
        use crate::model::panes as p;
        let mut m = menu();
        open_popup(m.recording_mut());
        let mut f = crate::model::frame::Frame::new();
        crate::model::screens::frame_of(&m, &mut f, |_| "");
        assert!(f.popup && f.conn_count == 0, "the empty-library default pop-up");
        let ([x, y, w, h], help_y, hint_y) = f.dialog_box().expect("the pop-up is sized");
        let [left, bottom, right, top] = [x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0];
        let div = p::spec(p::IDX_CONN_DIV);
        assert!(right >= p::ROLE_CAP_X[1] + p::ROLE_CAP / 2.0 && right >= p::row_span(true).1, "{right}: caps, rows");
        assert!(bottom <= div.y - div.h / 2.0 && bottom <= hint_y - 22.0 && help_y[0] < div.y - div.h / 2.0, "{bottom}");
        assert!(left <= p::row_span(true).0 && top >= p::TAB_Y + 28.0 && w <= p::PANEL_W, "{:?}", [left, top, w]);
        // A pop-up with connections: its last line still ends above the divider's foot, inside the same box.
        assert!(p::conn_y(p::CONN_LINES - 1) - p::CONN_LINE_H / 2.0 >= div.y - div.h / 2.0 && right >= p::CONN_RIGHT);
    }

    fn x_previews_a_drill_that_has_its_take_on_both_faces() {
        let macro_file = |id: &str, chain: bool, span| MacroFile {
            id: id.into(), name: id.into(), chain, steps: Vec::new(), start_save: None,
            start_save_on: true, recorded_on: String::new(), span,
        };
        let combo = |id: &str, input_only: bool, made_from: Option<&str>, take: Option<&str>| ComboFile {
            input_only, id: id.into(), name: id.into(), recorded_on: String::new(),
            made_from: made_from.map(str::to_string), take: take.map(str::to_string), combo: Box::new(Combo::EMPTY), fighter: None, span: 1,
        };
        for face in [Face::Page, Face::Popup] {
            let mut screens = RecordingScreens::new();
            let mut own_take = macro_file("t1", false, 25);
            own_take.name = crate::model::combo_record::drill_take_name(1);
            let mut bare_take = macro_file("bare", false, 25);
            bare_take.name = "Drill take".into();
            screens.set_catalog(Catalog {
                macros: vec![macro_file("take", false, 40), own_take, bare_take, macro_file("ch", true, 40)],
                combos: vec![combo("d-own", true, None, Some("t1")), combo("d-made", true, Some("take"), None),
                    combo("d-old", true, None, None), combo("d-gone", true, Some("nope"), None),
                    combo("d-chain", true, Some("ch"), None), combo("d-lost", true, None, Some("missing")),
                    combo("d-fake", true, None, Some("take")),
                    combo("d-bare", true, None, Some("bare")),
                    combo("hit", false, Some("take"), None)],
                ..Catalog::default()
            });
            screens.popup_open = true;
            assert!(screens.catalog().macros.iter().all(|file| file.id != "t1"));
            assert!(screens.catalog().macros.iter().any(|file| file.id == "take"));
            assert!(screens.catalog().macros.iter().any(|file| file.id == "bare"));
            let macros = screens.items(face, SAVES_MACROS, 1);
            assert!(macros.iter().all(|item| item.id != "t1"));
            assert!(macros.iter().any(|item| item.id == "take"));
            assert!(macros.iter().any(|item| item.id == "bare"));
            screens.open_picker(PickerKind::NewChain, face, None);
            let picker = screens.picker.as_ref().unwrap();
            assert!(picker.ids.iter().all(|id| id != "t1"));
            assert!(picker.ids.iter().any(|id| id == "take"));
            screens.picker = None;
            screens.popup_tab = 1;
            let place = screens.place_mut(face, COMBOS_SEQ);
            place.section = 1;
            place.drills = true;
            let focus = |screens: &mut RecordingScreens, id: &str| {
                let at = screens.items(face, COMBOS_SEQ, 1).iter().position(|item| item.id == id).unwrap();
                screens.place_mut(face, COMBOS_SEQ).at[1] = at;
            };
            focus(&mut screens, "d-own");
            assert_eq!(screens.hint_at(face, Some(COMBOS_SEQ)), if face == Face::Page {
                "A open   X preview   ZR role   L/R page   B close"
            } else {
                "A drill   X preview   Left/Right section   ZL/L/R tab   ZR role   ZL+Up/Down page   B close"
            });
            assert!(!screens.combo_lines("d-own").iter().any(|line| line.text == "Re-record to preview"));
            if face == Face::Page {
                assert_eq!(screens.handle_page(Button::X, true, COMBOS_SEQ), PageHit::Consumed);
            } else {
                assert!(screens.press_popup_outcome(Button::X));
            }
            assert_eq!(screens.take_effect(), Effect::PreviewDrill { source: "t1".into(), frames: 25 });
            for id in ["d-made", "d-old", "d-gone", "d-chain", "d-lost", "d-fake", "d-bare", ""] {
                focus(&mut screens, id);
                if !id.is_empty() {
                    assert_eq!(screens.combo_lines(id).first().map(|line| line.text.as_str()), Some("Re-record to preview"));
                }
                if id == "d-made" {
                    if face == Face::Page {
                        assert_eq!(screens.hint_at(face, Some(COMBOS_SEQ)), screens.hint(Face::Page));
                    } else {
                        assert_eq!(screens.hint_at(face, Some(COMBOS_SEQ)),
                            "A drill   Left/Right section   ZL/L/R tab   ZR role   ZL+Up/Down page   B close");
                    }
                }
                if face == Face::Page {
                    assert_eq!(screens.handle_page(Button::X, true, COMBOS_SEQ), PageHit::Ignored);
                } else {
                    assert!(!screens.press_popup_outcome(Button::X));
                }
                assert_eq!(screens.take_effect(), Effect::None);
            }
            screens.place_mut(face, COMBOS_SEQ).drills = false;
            focus(&mut screens, "hit");
            assert!(!screens.combo_lines("hit").iter().any(|line| line.text == "Re-record to preview"));
            if face == Face::Page {
                assert_eq!(screens.handle_page(Button::X, true, COMBOS_SEQ), PageHit::Ignored);
            } else {
                assert!(!screens.press_popup_outcome(Button::X));
            }
            assert_eq!(screens.take_effect(), Effect::None);
            screens.popup_tab = 0;
            screens.place_mut(face, SAVES_MACROS).section = 1;
            let before = screens.loop_of(screens.role(face));
            if face == Face::Page {
                assert_eq!(screens.handle_page(Button::X, true, SAVES_MACROS), PageHit::Consumed);
            } else {
                assert!(screens.press_popup_outcome(Button::X));
            }
            assert_ne!(screens.loop_of(screens.role(face)), before);
            assert_eq!(screens.take_effect(), Effect::None);
        }
    }

    fn a_cropped_made_drill_has_no_preview() {
        use crate::model::combo::{Aim, Combo, Event, LinkRec};
        use crate::model::combo_record::ComboRecord;
        use crate::model::frame_strip::Button as GameButton;

        let mut step = LinkRec::INPUT;
        step.events[0] = Some(Event::press(GameButton::Jump, Aim::Neutral, 0));
        let mut inputs = Combo::from_links([step]).unwrap();
        inputs.input_only = true;
        let crop = ComboRecord::new("Drill".into(), inputs, None)
            .with_made_from("source")
            .crop_units(0, 1).unwrap();
        assert_eq!(crop.made_from.as_deref(), Some("source"), "the crop keeps its provenance");
        let mut screens = RecordingScreens::new();
        screens.set_catalog(Catalog {
            macros: vec![MacroFile { id: "source".into(), name: "Source".into(), chain: false, steps: Vec::new(),
                start_save: None, start_save_on: true, recorded_on: String::new(), span: 40 }],
            combos: vec![ComboFile { id: "crop".into(), name: crop.name, input_only: true, recorded_on: String::new(),
                made_from: crop.made_from, take: crop.take, combo: Box::new(Combo::EMPTY), fighter: None, span: 1 }],
            ..Catalog::default()
        });
        let place = screens.place_mut(Face::Page, COMBOS_SEQ);
        place.section = 1;
        place.drills = true;
        let at = screens.items(Face::Page, COMBOS_SEQ, 1).iter().position(|item| item.id == "crop").unwrap();
        screens.place_mut(Face::Page, COMBOS_SEQ).at[1] = at;
        assert_eq!(screens.combo_lines("crop").first().map(|line| line.text.as_str()), Some("Re-record to preview"));
        assert_eq!(screens.handle_page(Button::X, true, COMBOS_SEQ), PageHit::Ignored);
        assert_eq!(screens.take_effect(), Effect::None);
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("f9_b_backs_out_of_a_new_picker", f9_b_backs_out_of_a_new_picker as fn()),
            ("f9_x_is_done_and_asks_the_name", f9_x_is_done_and_asks_the_name as fn()),
            ("f9_b_keeps_the_old_chain_on_edit", f9_b_keeps_the_old_chain_on_edit as fn()),
            ("f9_a_cancelled_name_makes_nothing", f9_a_cancelled_name_makes_nothing as fn()),
            ("f9_x_finishes_an_edit", f9_x_finishes_an_edit as fn()),
            ("f2_recording_page_list_wraps", f2_recording_page_list_wraps as fn()),
            ("f2_recording_empty_saves_moves_to_macros", f2_recording_empty_saves_moves_to_macros as fn()),
            ("f2_recording_hold_wraps", f2_recording_hold_wraps as fn()),
            ("f2_recording_file_wrap", f2_recording_file_wrap as fn()),
            ("f2_recording_picker_wrap", f2_recording_picker_wrap as fn()),
            ("popup_background_holds_its_chrome", popup_background_holds_its_chrome as fn()),
            ("drills_are_a_filtered_combo_sublist_on_both_faces",drills_are_a_filtered_combo_sublist_on_both_faces as fn()),
            ("the_drills_sublist_is_its_own_list_one_level_down_on_both_faces", the_drills_sublist_is_its_own_list_one_level_down_on_both_faces as fn()),
            ("a_recording_screen_with_no_row_publishes_no_list_not_the_page_under_it", a_recording_screen_with_no_row_publishes_no_list_not_the_page_under_it as fn()),
            ("every_b_that_goes_back_takes_one_off_depth_on_the_recording_lists", every_b_that_goes_back_takes_one_off_depth_on_the_recording_lists as fn()),
            ("x_previews_a_drill_that_has_its_take_on_both_faces", x_previews_a_drill_that_has_its_take_on_both_faces as fn()),
            ("a_cropped_made_drill_has_no_preview", a_cropped_made_drill_has_no_preview as fn()),
            ("a_reopened_menu_loads_the_catalog_without_a_new_file", a_reopened_menu_loads_the_catalog_without_a_new_file as fn()),
            ("the_lists_say_loading_until_the_catalog_arrives", the_lists_say_loading_until_the_catalog_arrives as fn()),
            ("one_save_is_bound_and_the_page_opens_it", one_save_is_bound_and_the_page_opens_it as fn()),
            ("popup_buttons_match_q11", popup_buttons_match_q11 as fn()),
            ("popup_zr_toggles_the_role_like_the_page", popup_zr_toggles_the_role_like_the_page as fn()),
            ("popup_l_r_zl_switch_tabs_and_never_set_the_role", popup_l_r_zl_switch_tabs_and_never_set_the_role as fn()),
            ("popup_footer_names_zr_role_and_tab_keys", popup_footer_names_zr_role_and_tab_keys as fn()),
            ("paging_moves_one_section_by_twelve", paging_moves_one_section_by_twelve as fn()),
            ("library_lists_wrap_at_their_edges", library_lists_wrap_at_their_edges as fn()),
            ("holding_down_on_the_recording_lists_quickens", holding_down_on_the_recording_lists_quickens as fn()),
            ("recording_page_jump_goes_on_past_a_section", recording_page_jump_goes_on_past_a_section as fn()),
            ("bridge_recording_list_tracks_jumps_and_the_popup", bridge_recording_list_tracks_jumps_and_the_popup as fn()),
            ("bridge_publishes_the_popup_owner_and_not_the_panel_or_external_screen", bridge_publishes_the_popup_owner_and_not_the_panel_or_external_screen as fn()),
            ("recording_picker_takes_the_list_jump", recording_picker_takes_the_list_jump as fn()),
            ("zl_down_reaches_the_recording_lists_through_the_adapter", zl_down_reaches_the_recording_lists_through_the_adapter as fn()),
            ("x_cycles_the_loop_only_in_macros", x_cycles_the_loop_only_in_macros as fn()),
            ("set_active_is_one_row_for_the_role_in_view", set_active_is_one_row_for_the_role_in_view as fn()),
            ("a_macro_plays_as_a_playback_slot", a_macro_plays_as_a_playback_slot as fn()),
            ("retired_rows_leave_the_menu_and_old_settings_still_load", retired_rows_leave_the_menu_and_old_settings_still_load as fn()),
            ("the_combo_panel_lists_each_links_events_and_drift", the_combo_panel_lists_each_links_events_and_drift as fn()),
            ("the_floppy_is_grey_when_the_switch_is_off", the_floppy_is_grey_when_the_switch_is_off as fn()),
            ("new_chain_and_new_sequence_open_the_picker", new_chain_and_new_sequence_open_the_picker as fn()),
            ("y_in_the_popup_new_chain_picker_is_ignored_and_a_picker_press_is_handled", y_in_the_popup_new_chain_picker_is_ignored_and_a_picker_press_is_handled as fn()),
            ("crop_actions_open_the_shared_clip_editor", crop_actions_open_the_shared_clip_editor as fn()),
            ("recording_popup_request_opens_the_popup", recording_popup_request_opens_the_popup as fn()),
            ("a_numbered_row_drills_that_combo_and_drill_sequence_drills_the_sequence", a_numbered_row_drills_that_combo_and_drill_sequence_drills_the_sequence as fn()),
            ("edit_combos_writes_the_new_order_and_r_removes_the_last_number", edit_combos_writes_the_new_order_and_r_removes_the_last_number as fn()),
            ("delete_posts_the_file_for_the_host_to_remove", delete_posts_the_file_for_the_host_to_remove as fn()),
            ("the_popup_connections_are_the_r8_boxes", the_popup_connections_are_the_r8_boxes as fn()),
            ("save_last_combo_posts_the_typed_name_or_combo_n", save_last_combo_posts_the_typed_name_or_combo_n as fn()),
        ]);
    }
    }
}
