//! The recording pop-up, the Recording page's library, and each file's screen.
//!
//! The host fills [`Catalog`] and reads [`Selection`]. Playback, the Switch keyboard, crop, and
//! Fighter Mismatch stay behind [`Effect`]. This module does not import `engine` (a by-path test of
//! `model.rs` has no engine module) and it does not touch `recording.rs` or `playback.rs`.

use super::combo::Combo;
use super::menu::input::Button;
use super::menu::table::{COMBOS_SEQ, SAVES_MACROS};
use super::numbered_picker::{self, CatalogTile, NumberKey, TileKind};
use super::ordered_ids;

/// How far ZL+Up / ZL+Down moves a section. The menu's section jump uses the same distance.
pub const PAGE: usize = 12;
/// Rows of one section the twelve row panes can show at once (two sections).
pub const VISIBLE: usize = 6;

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
    Passthrough,
    LeaveUp,
    LeaveDown,
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
    Keyboard { header: &'static str, draft: String },
    MakeCombo { source: String, chain: bool },
    Crop { source: String, combo: bool, start: u32, end: u32 },
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
    /// *+ Save last combo…*: the host saves the strip's latest exchange under `name` (empty: Combo N) and drills it.
    SaveLastCombo { name: String },
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

/// A combo file. `links` are the lines [`combo_link_lines`] produces: the move, then its events, then drift.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboFile {
    pub id: String,
    pub name: String,
    /// The fighter the file says it was recorded on (empty: not recorded).
    pub recorded_on: String,
    pub made_from: Option<String>,
    pub links: Vec<String>,
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
    Save,
    Macro,
    Combo,
    Sequence,
}

/// One line of the r8 pop-up's CONNECTIONS column (`RECORDING-REDESIGN-r8-popup-*.png`): the row under the cursor in
/// its kind box, a heading over the files joined to it, each of those in its own box, and plain lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnKind {
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
        matches!(self, ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Item {
    kind: ItemKind,
    id: String,
}

#[derive(Clone, Copy, Debug)]
struct Place {
    section: u8,
    at: [usize; 2],
}

impl Place {
    const fn new() -> Place {
        Place { section: 0, at: [0, 0] }
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

/// The crop screen's two handles. L and R move the focused one. A saves. B returns.
#[derive(Clone, Debug)]
struct CropDraft {
    source: String,
    combo: bool,
    span: u32,
    start: u32,
    end: u32,
    out: bool,
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
    bound: Option<String>,
    active: [Option<String>; 2],
    loops: [LoopMark; 2],
    drill: Option<String>,
    last_effect: Effect,
    serial: u32,
    crop: Option<CropDraft>,
    /// Online waiting room: the keyboard stays closed and an empty name keeps N.
    keyboard_blocked: bool,
}

impl Default for RecordingScreens {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordingScreens {
    pub fn new() -> RecordingScreens {
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
            bound: None,
            active: [None, None],
            loops: [LoopMark::default(), LoopMark::default()],
            drill: None,
            last_effect: Effect::None,
            serial: 0,
            crop: None,
            keyboard_blocked: false,
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
            self.crop = None;
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

    pub fn set_catalog(&mut self, catalog: Catalog) {
        self.catalog = catalog;
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
        }
    }

    pub fn last_effect(&self) -> &Effect {
        &self.last_effect
    }

    /// The host takes the effect once. A second read is [`Effect::None`].
    pub fn take_effect(&mut self) -> Effect {
        std::mem::replace(&mut self.last_effect, Effect::None)
    }

    pub fn loop_of(&self, role: ViewRole) -> LoopMark {
        self.loops[role.index()]
    }

    pub fn loop_label(&self, role: ViewRole) -> &'static str {
        self.loop_of(role).label()
    }

    /// The Options row's Left / Right. X does not call this.
    pub fn cycle_loop_row(&mut self, forward: bool) {
        let role = self.page_role.index();
        let marks = LoopMark::CYCLE;
        let at = marks.iter().position(|mark| *mark == self.loops[role]).unwrap_or(1);
        let next = if forward { (at + 1) % marks.len() } else { (at + marks.len() - 1) % marks.len() };
        self.loops[role] = marks[next];
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
        let floppy = self.file.as_ref().and_then(|file| self.row_floppy(&Item { kind: file.kind, id: file.id.clone() }));
        self.built_file_rows()
            .into_iter()
            .map(|row| FileRow { floppy: floppy.filter(|_| row.act == Act::StartSave), title: row.title, value: row.value })
            .collect()
    }

    pub fn file_crumb(&self) -> String {
        let Some(file) = &self.file else { return String::new() };
        let role = if self.page_role == ViewRole::Cpu { "CPU view" } else { "Player view" };
        let group = match file.kind {
            ItemKind::Combo | ItemKind::Sequence => "Combos & Sequences",
            _ => "Save states & Macros",
        };
        format!("Recording ({role})  >  {group}  >  {}", self.item_name(&Item { kind: file.kind, id: file.id.clone() }))
    }

    pub fn section(&self, face: Face, group: u8, section: u8) -> SectionView {
        self.section_view(face, group, section)
    }

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
                let Some(save) = self.catalog.saves.iter().find(|file| file.id == item.id) else { return out };
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
                    out.extend(combos.iter().map(|combo| line(ConnKind::Child(FileTag::Combo), combo.name.clone())));
                }
                if !file.chain && !file.recorded_on.is_empty() {
                    let seconds = file.span as f32 / 60.0;
                    out.push(line(ConnKind::Detail, format!("Recorded on  {} · {seconds:.1} s · information only", file.recorded_on)));
                }
            }
            ItemKind::Combo => {
                let Some(combo) = self.catalog.combos.iter().find(|file| file.id == item.id) else { return out };
                out.push(line(ConnKind::Head(FileTag::Combo), combo.name.clone()));
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
                let links = combo.links.iter().filter(|text| link_head(text)).count();
                if !combo.links.is_empty() {
                    out.push(line(ConnKind::Heading, format!("Its links ({links})")));
                    out.extend(combo.links.iter().map(|text| {
                        line(if link_head(text) { ConnKind::Link } else { ConnKind::LinkSub }, text.clone())
                    }));
                }
                if !combo.recorded_on.is_empty() {
                    out.push(line(ConnKind::Detail, format!("Fighters  {}", combo.recorded_on)));
                }
            }
            ItemKind::Sequence => {
                let Some(sequence) = self.catalog.sequences.iter().find(|file| file.id == item.id) else { return out };
                out.push(line(ConnKind::Head(FileTag::Sequence), sequence.name.clone()));
                if !sequence.combos.is_empty() {
                    out.push(line(ConnKind::Heading, "combos".to_string()));
                    for (index, step) in sequence.combos.iter().enumerate() {
                        let name = self.catalog.combos.iter().find(|combo| combo.id == *step).map_or(step.as_str(), |combo| combo.name.as_str());
                        out.push(line(ConnKind::Child(FileTag::Combo), format!("{} {name}", index + 1)));
                    }
                }
                out.push(line(ConnKind::Detail, format!("streak {} · best {}", sequence.streak, sequence.best)));
            }
            ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo => {
                out.extend(self.connection_lines(face, &item).into_iter().map(|conn| line(ConnKind::Detail, conn.text)));
            }
        }
        out
    }

    pub fn set_cursor(&mut self, face: Face, group: u8, section: u8, index: usize) {
        let place = self.place_mut(face, group);
        let section = section.min(1);
        place.section = section;
        place.at[usize::from(section)] = index;
    }

    pub fn set_file_cursor(&mut self, cursor: usize) {
        if let Some(file) = self.file.as_mut() {
            file.cursor = cursor;
        }
    }

    pub fn set_links(&mut self, id: &str, links: Vec<String>) {
        if let Some(combo) = self.catalog.combos.iter_mut().find(|file| file.id == id) {
            combo.links = links;
        }
    }

    pub fn has_chain_named(&self, name: &str) -> bool {
        self.catalog.macros.iter().any(|file| file.chain && file.name == name)
    }

    pub fn picker_recording_ids(&self) -> Vec<String> {
        self.recording_ids()
    }

    pub fn start_save_id(&self, id: &str) -> Option<&str> {
        self.macro_by_id(id).and_then(|file| file.start_save.as_deref())
    }

    pub fn hint(&self, face: Face) -> &'static str {
        if self.keyboard.is_some() {
            return "Name it, then confirm. Empty keeps the N name. B confirms empty.";
        }
        if self.picker.is_some() {
            return "A add number   R remove   B done";
        }
        if self.file.is_some() && face == Face::Page {
            return "Up/Down move   Left/Right On/Off   A do it   B back";
        }
        if face == Face::Popup {
            // The r8 footer: what A does in the section under the cursor, then the pop-up's keys. The CPU has one tab.
            let group = if self.tab() == 0 { SAVES_MACROS } else { COMBOS_SEQ };
            let cpu = self.popup_role == ViewRole::Cpu;
            match (group, self.current_section(face, group), cpu) {
                (SAVES_MACROS, 0, false) => "A bind   Left/Right section   ZL/ZR tab   L/R role   ZL+Up/Down page   B close",
                (SAVES_MACROS, 0, true) => "A bind   Left/Right section   L/R role   ZL+Up/Down page   B close",
                (SAVES_MACROS, _, false) => {
                    "A set active   X loop   Left/Right section   ZL/ZR tab   L/R role   ZL+Up/Down page   B close"
                }
                (SAVES_MACROS, _, true) => "A set active   X loop   Left/Right section   L/R role   ZL+Up/Down page   B close",
                _ => "A drill   Left/Right section   ZL/ZR tab   L/R role   ZL+Up/Down page   B close",
            }
        } else {
            "A open   X loop in MACROS   ZR role   L/R page   B close"
        }
    }

    pub fn set_keyboard_blocked(&mut self, blocked: bool) {
        self.keyboard_blocked = blocked;
    }

    pub fn naming(&self) -> bool {
        self.keyboard.is_some()
    }

    pub fn crop_open(&self) -> bool {
        self.crop.is_some()
    }

    /// The index's selection, with every catalog load (`CatalogWatch::take_load`: a new opening or a file write). The
    /// menu publishes its selection back to the index (`rr_host::publish_selection`), so it must never hold one older
    /// than the index: a model the pop-up kept alive took the index once, and a later Record made a new active file, so
    /// closing the menu put the old one back (Eden rc5-opus-final-trace2, 2026-11-08). The menu's own edits are already
    /// in the index, published with the press that made them.
    pub fn adopt(&mut self, bound: Option<String>, active: [Option<String>; 2], loops: [LoopMark; 2], drill: Option<String>) {
        self.bound = bound;
        self.active = active;
        self.loops = loops;
        self.drill = drill;
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
        let mut line = format!(
            "screen={screen} role={} tab={tab} loop={} hot={hot} hairline={hair} floppy={floppy} header={header} active={active} group={group_name}",
            role.diag(),
            self.loop_of(role).label(),
        );
        if let Some(crop) = &self.crop {
            line.push_str(&format!(
                " screen=crop source={} combo={} open=1 in={} out={}",
                crop.source,
                i32::from(crop.combo),
                crop.start,
                crop.end
            ));
        }
        line
    }

    /// One pop-up button. Always consumed while the pop-up is open.
    pub fn press_popup(&mut self, button: Button) {
        if !self.popup_open {
            return;
        }
        if self.crop.is_some() {
            self.press_crop(button);
            return;
        }
        if self.overlay_face == Face::Popup && self.overlay_open() {
            self.press_overlay(button, Face::Popup);
            return;
        }
        let group = self.popup_group();
        match button {
            Button::B => self.popup_open = false,
            Button::Y => {}
            Button::L => self.set_popup_role(ViewRole::Player),
            Button::R => self.set_popup_role(ViewRole::Cpu),
            Button::ZL | Button::ZR => self.toggle_tab(),
            Button::X => {
                if self.in_macros(Face::Popup, group) {
                    self.cycle_x(self.popup_role);
                }
            }
            Button::Up => self.nudge_popup(-1),
            Button::Down => self.nudge_popup(1),
            Button::JumpUp => self.page_section(Face::Popup, group, false),
            Button::JumpDown => self.page_section(Face::Popup, group, true),
            Button::Left | Button::Right => self.swap_section(Face::Popup, group),
            Button::A => self.activate(Face::Popup, group),
        }
    }

    /// One Recording-page button. L, R, and B pass through unless a file screen is open.
    pub fn handle_page(&mut self, button: Button, on_library: bool, group: u8) -> PageHit {
        if self.crop.is_some() {
            self.press_crop(button);
            return PageHit::Consumed;
        }
        if self.overlay_open() && self.overlay_face == Face::Page {
            self.press_overlay(button, Face::Page);
            return PageHit::Consumed;
        }
        match button {
            Button::X => {
                if on_library && self.in_macros(Face::Page, group) {
                    self.cycle_x(self.page_role);
                }
                PageHit::Consumed
            }
            Button::Y | Button::ZL => PageHit::Consumed,
            Button::ZR => {
                self.page_role = self.page_role.other();
                PageHit::Consumed
            }
            Button::L | Button::R | Button::B => PageHit::Passthrough,
            _ if !on_library => PageHit::Passthrough,
            Button::Up => self.nudge_page(group, -1),
            Button::Down => self.nudge_page(group, 1),
            Button::JumpUp => {
                self.page_section(Face::Page, group, false);
                PageHit::Consumed
            }
            Button::JumpDown => {
                self.page_section(Face::Page, group, true);
                PageHit::Consumed
            }
            Button::Left | Button::Right => {
                self.swap_section(Face::Page, group);
                PageHit::Consumed
            }
            Button::A => {
                self.activate(Face::Page, group);
                PageHit::Consumed
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
        let slot = role.index();
        self.loops[slot] = self.loops[slot].next();
    }

    fn nudge_popup(&mut self, delta: isize) {
        let group = self.popup_group();
        let section = usize::from(self.place(Face::Popup, group).section);
        let len = self.items(group, section as u8).len();
        if len == 0 {
            return;
        }
        let at = self.place(Face::Popup, group).at[section].min(len - 1);
        if delta < 0 && at == 0 || delta > 0 && at + 1 >= len {
            return;
        }
        self.place_mut(Face::Popup, group).at[section] = if delta < 0 { at - 1 } else { at + 1 };
    }

    fn nudge_page(&mut self, group: u8, delta: isize) -> PageHit {
        let section = self.place(Face::Page, group).section;
        let len = self.items(group, section).len();
        let at = self.place(Face::Page, group).at[usize::from(section)];
        let at = if len == 0 { 0 } else { at.min(len - 1) };
        let at_edge = len == 0 || (delta < 0 && at == 0) || (delta > 0 && at + 1 >= len);
        if !at_edge {
            self.place_mut(Face::Page, group).at[usize::from(section)] = if delta < 0 { at - 1 } else { at + 1 };
            return PageHit::Consumed;
        }
        if delta > 0 && section == 0 {
            self.place_mut(Face::Page, group).section = 1;
            self.place_mut(Face::Page, group).at[1] = 0;
            return PageHit::Consumed;
        }
        if delta < 0 && section == 1 {
            let upper = self.items(group, 0).len();
            self.place_mut(Face::Page, group).section = 0;
            self.place_mut(Face::Page, group).at[0] = upper.saturating_sub(1);
            return PageHit::Consumed;
        }
        if delta < 0 { PageHit::LeaveUp } else { PageHit::LeaveDown }
    }

    fn swap_section(&mut self, face: Face, group: u8) {
        let from = self.place(face, group).section;
        let to = 1 - from;
        let len = self.items(group, to).len();
        let kept = self.place(face, group).at[usize::from(from)];
        let index = if len == 0 { 0 } else { kept.min(len - 1) };
        let place = self.place_mut(face, group);
        place.at[usize::from(to)] = index;
        place.section = to;
    }

    fn page_section(&mut self, face: Face, group: u8, forward: bool) {
        let section = usize::from(self.place(face, group).section);
        let len = self.items(group, section as u8).len();
        let at = self.place(face, group).at[section];
        self.place_mut(face, group).at[section] = page_by(at, len, forward);
    }

    fn activate(&mut self, face: Face, group: u8) {
        let Some(item) = self.focused(face, group) else { return };
        match (face, item.kind) {
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
        let known: Vec<String> = self.catalog.combos.iter().map(|file| file.id.clone()).collect();
        if let Some(sequence) = self.catalog.sequences.iter_mut().find(|file| file.id == id) {
            ordered_ids::retain_existing(&mut sequence.combos, |step| known.iter().any(|id| id == step));
        }
    }

    fn press_overlay(&mut self, button: Button, face: Face) {
        if self.keyboard.is_some() {
            if button == Button::B {
                self.submit_name("");
            }
            return;
        }
        if self.picker.is_some() {
            self.press_picker(button, face);
            return;
        }
        self.press_file(button);
    }

    fn press_file(&mut self, button: Button) {
        let len = self.built_file_rows().len();
        match button {
            Button::B => self.file = None,
            Button::Up => {
                if let Some(file) = self.file.as_mut() {
                    if file.cursor > 0 {
                        file.cursor -= 1;
                    }
                }
            }
            Button::Down => {
                if let Some(file) = self.file.as_mut() {
                    if file.cursor + 1 < len {
                        file.cursor += 1;
                    }
                }
            }
            Button::JumpUp | Button::JumpDown => {
                if let Some(file) = self.file.as_mut() {
                    file.cursor = page_by(file.cursor, len, button == Button::JumpDown);
                }
            }
            Button::Left => self.flip_switch(false),
            Button::Right => self.flip_switch(true),
            Button::A => self.do_file_row(),
            Button::X | Button::Y | Button::L | Button::R | Button::ZL | Button::ZR => {}
        }
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
                self.delete_file(kind, &id);
                if kind == ItemKind::Save {
                    self.last_effect = Effect::DeleteSave { source: id };
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

    fn open_crop(&mut self, kind: ItemKind, id: String) {
        let combo = kind == ItemKind::Combo;
        let span = match kind {
            ItemKind::Combo => self.catalog.combos.iter().find(|file| file.id == id).map(|file| file.span).unwrap_or(0),
            ItemKind::Recording => self.macro_by_id(&id).map(|file| file.span).unwrap_or(0),
            _ => 0,
        };
        let span = span.max(1);
        self.crop = Some(CropDraft { source: id, combo, span, start: 0, end: span, out: false });
    }

    fn press_crop(&mut self, button: Button) {
        let Some(crop) = self.crop.as_mut() else { return };
        match button {
            Button::L | Button::R => {
                let delta = if button == Button::L { -1 } else { 1 };
                if crop.out {
                    let next = (crop.end as i32 + delta).clamp(crop.start as i32 + 1, crop.span as i32);
                    crop.end = next as u32;
                } else {
                    let next = (crop.start as i32 + delta).clamp(0, crop.end as i32 - 1);
                    crop.start = next as u32;
                }
            }
            Button::Up | Button::Down => crop.out = !crop.out,
            Button::A => {
                let Some(crop) = self.crop.take() else { return };
                self.last_effect = Effect::Crop {
                    source: crop.source,
                    combo: crop.combo,
                    start: crop.start,
                    end: crop.end,
                };
            }
            Button::B => self.crop = None,
            _ => {}
        }
    }

    pub fn crop_cursor(&self) -> usize {
        usize::from(self.crop.as_ref().is_some_and(|crop| crop.out))
    }

    pub fn crop_rows(&self) -> Vec<FileRow> {
        let Some(crop) = &self.crop else { return Vec::new() };
        vec![
            FileRow { title: "In".into(), value: format!("{}", crop.start), floppy: None },
            FileRow { title: "Out".into(), value: format!("{}", crop.end), floppy: None },
            FileRow { title: "Save".into(), value: "A saves a new file".into(), floppy: None },
        ]
    }

    fn set_bound(&mut self, id: &str, on: bool) {
        if on {
            self.bound = Some(id.to_string());
            self.note_hot_save(id);
        } else if self.bound.as_deref() == Some(id) {
            self.bound = None;
        }
    }

    fn set_active(&mut self, role: ViewRole, id: &str, on: bool) {
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
        let hot = if kind == ItemKind::Sequence { &mut self.catalog.hot_sequences } else { &mut self.catalog.hot_combos };
        hot.retain(|hot| *hot != id);
        hot.insert(0, id.clone());
        self.drill = Some(id);
    }

    fn set_start(&mut self, _kind: ItemKind, id: &str, on: bool) {
        if let Some(file) = self.catalog.macros.iter_mut().find(|file| file.id == id) {
            file.start_save_on = on;
        }
    }

    fn start_is_on(&self, _kind: ItemKind, id: &str) -> bool {
        self.macro_by_id(id).is_some_and(|file| file.start_save_on)
    }

    fn duplicate_save(&mut self, id: &str) {
        let Some(source) = self.catalog.saves.iter().find(|file| file.id == id).cloned() else { return };
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
        self.file = None;
    }

    fn open_rename(&mut self, kind: ItemKind, id: &str) {
        let (key, header) = match kind {
            ItemKind::Save => (KeyKind::RenameSave, "Name save state"),
            ItemKind::Recording if self.macro_by_id(id).is_some_and(|file| file.chain) => (KeyKind::RenameChain, "Name chain"),
            ItemKind::Recording => (KeyKind::RenameMacro, "Name macro"),
            ItemKind::Combo => (KeyKind::RenameCombo, "Name combo"),
            ItemKind::Sequence => (KeyKind::RenameSequence, "Name sequence"),
            ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo => return,
        };
        let previous = self.item_name(&Item { kind, id: id.to_string() });
        self.keyboard = Some(Keyboard { kind: key, ids: Vec::new(), target: Some(id.to_string()) });
        if self.keyboard_blocked {
            self.submit_name("");
            return;
        }
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
            PickerKind::NewSequence | PickerKind::EditSequence => self.catalog.combos.iter().map(|file| file.id.clone()).collect(),
        };
        let order = editing.as_ref().map(|id| self.existing_order(kind, id)).unwrap_or_default();
        let values = fill_ranks(&ids, &order);
        self.overlay_face = face;
        self.picker = Some(Picker { kind, cursor: 0, values, ids, editing });
    }

    fn existing_order(&self, kind: PickerKind, id: &str) -> Vec<String> {
        match kind {
            PickerKind::EditChain => self.macro_by_id(id).map(|file| file.steps.clone()).unwrap_or_default(),
            PickerKind::EditSequence => self.catalog.sequences.iter().find(|file| file.id == id).map(|file| file.combos.clone()).unwrap_or_default(),
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

    fn press_picker(&mut self, button: Button, _face: Face) {
        match button {
            Button::B => self.finish_picker(),
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
                    if picker.cursor > 0 {
                        picker.cursor -= 1;
                    }
                }
            }
            Button::Down => {
                if let Some(picker) = self.picker.as_mut() {
                    if !picker.ids.is_empty() && picker.cursor + 1 < picker.ids.len() {
                        picker.cursor += 1;
                    }
                }
            }
            Button::Left | Button::Right | Button::L | Button::ZR | Button::ZL | Button::X | Button::Y | Button::JumpUp | Button::JumpDown => {}
        }
    }

    fn finish_picker(&mut self) {
        let Some(picker) = self.picker.take() else { return };
        let ordered = ordered_ids::write_back(&picker.values, &picker.ids);
        match picker.kind {
            PickerKind::EditChain => {
                if let Some(id) = picker.editing {
                    if let Some(chain) = self.catalog.macros.iter_mut().find(|file| file.id == id) {
                        chain.steps = ordered;
                    }
                }
            }
            PickerKind::EditSequence => {
                if let Some(id) = picker.editing {
                    if let Some(sequence) = self.catalog.sequences.iter_mut().find(|file| file.id == id) {
                        sequence.combos = ordered;
                    }
                }
            }
            PickerKind::NewChain => self.open_keyboard(KeyKind::NewChain, "Name chain", ordered),
            PickerKind::NewSequence => self.open_keyboard(KeyKind::NewSequence, "Name sequence", ordered),
        }
    }

    fn open_keyboard(&mut self, kind: KeyKind, header: &'static str, ids: Vec<String>) {
        self.keyboard = Some(Keyboard { kind, ids, target: None });
        if self.keyboard_blocked {
            self.submit_name("");
            return;
        }
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

    fn focused(&self, face: Face, group: u8) -> Option<Item> {
        let place = self.place(face, group);
        let items = self.items(group, place.section);
        if items.is_empty() {
            return None;
        }
        let at = place.at[usize::from(place.section)].min(items.len() - 1);
        items.get(at).cloned()
    }

    fn items(&self, group: u8, section: u8) -> Vec<Item> {
        let combos = group == COMBOS_SEQ;
        if !combos && section == 0 {
            return self.ordered(&self.catalog.saves, &self.catalog.hot_saves, |file| &file.id, |file| &file.name)
                .into_iter()
                .map(|file| Item { kind: ItemKind::Save, id: file.id.clone() })
                .collect();
        }
        if !combos {
            let mut rows: Vec<Item> = self
                .ordered(&self.catalog.macros, &self.catalog.hot_macros, |file| &file.id, |file| &file.name)
                .into_iter()
                .map(|file| Item { kind: ItemKind::Recording, id: file.id.clone() })
                .collect();
            rows.push(Item { kind: ItemKind::NewChain, id: String::new() });
            return rows;
        }
        if section == 0 {
            let mut rows: Vec<Item> = self
                .ordered(&self.catalog.sequences, &self.catalog.hot_sequences, |file| &file.id, |file| &file.name)
                .into_iter()
                .map(|file| Item { kind: ItemKind::Sequence, id: file.id.clone() })
                .collect();
            rows.push(Item { kind: ItemKind::NewSequence, id: String::new() });
            return rows;
        }
        let mut rows = vec![Item { kind: ItemKind::SaveLastCombo, id: String::new() }];
        rows.extend(
            self.ordered(&self.catalog.combos, &self.catalog.hot_combos, |file| &file.id, |file| &file.name)
                .into_iter()
                .map(|file| Item { kind: ItemKind::Combo, id: file.id.clone() }),
        );
        rows
    }

    fn ordered<'a, T>(&'a self, files: &'a [T], hot: &[String], id: impl Fn(&T) -> &str, name: impl Fn(&T) -> &str) -> Vec<&'a T> {
        let _ = self;
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

    fn section_view(&self, face: Face, group: u8, section: u8) -> SectionView {
        let items = self.items(group, section);
        let file_count = items.iter().filter(|item| !item.kind.action()).count();
        // Action rows that head the list (+ Save last combo…): the hot marks and the hairline count files only.
        let lead = items.iter().take_while(|item| item.kind.action()).count();
        let place = self.place(face, group);
        let cursor = place.at[usize::from(section)];
        let focused = self.focused(face, group);
        let role = self.role(face);
        let header = header_of(group, section);
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
            ItemKind::Save => self.catalog.saves.iter().find(|file| file.id == item.id).map(|file| file.fighters.clone()).unwrap_or_default(),
            ItemKind::Recording => match self.macro_by_id(&item.id) {
                Some(file) if file.chain => {
                    file.steps.iter().filter_map(|id| self.macro_by_id(id).map(|step| step.name.as_str())).collect::<Vec<_>>().join(" → ")
                }
                Some(file) => recorded(&file.recorded_on),
                None => String::new(),
            },
            ItemKind::Combo => self.catalog.combos.iter().find(|file| file.id == item.id).map(|file| recorded(&file.recorded_on)).unwrap_or_default(),
            ItemKind::Sequence => self
                .catalog
                .sequences
                .iter()
                .find(|file| file.id == item.id)
                .map(|file| {
                    file.combos
                        .iter()
                        .filter_map(|id| self.catalog.combos.iter().find(|combo| &combo.id == id).map(|combo| combo.name.as_str()))
                        .collect::<Vec<_>>()
                        .join(" → ")
                })
                .unwrap_or_default(),
            ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo => String::new(),
        }
    }

    fn is_hot(&self, item: &Item) -> bool {
        let hot = match item.kind {
            ItemKind::Save => &self.catalog.hot_saves,
            ItemKind::Recording => &self.catalog.hot_macros,
            ItemKind::Combo => &self.catalog.hot_combos,
            ItemKind::Sequence => &self.catalog.hot_sequences,
            ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo => return false,
        };
        hot.iter().take(5).any(|id| id == &item.id)
    }

    fn is_green(&self, item: &Item, role: ViewRole) -> bool {
        match item.kind {
            ItemKind::Save => self.bound.as_deref() == Some(item.id.as_str()),
            ItemKind::Recording => self.active[role.index()].as_deref() == Some(item.id.as_str()),
            ItemKind::Combo | ItemKind::Sequence => self.drill.as_deref() == Some(item.id.as_str()),
            ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo => false,
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
            let sequence = self.catalog.sequences.iter().find(|file| file.id == focus.id)?;
            return ordered_ids::order_of(&sequence.combos, &item.id);
        }
        if focus.kind == ItemKind::Combo && item.kind == ItemKind::Sequence {
            let sequence = self.catalog.sequences.iter().find(|file| file.id == item.id)?;
            return ordered_ids::order_of(&sequence.combos, &focus.id);
        }
        None
    }

    fn item_name(&self, item: &Item) -> String {
        match item.kind {
            ItemKind::NewChain => "+ New chain".to_string(),
            ItemKind::NewSequence => "+ New sequence".to_string(),
            ItemKind::SaveLastCombo => "+ Save last combo…".to_string(),
            kind => self.slot_name(kind, &item.id).unwrap_or_default(),
        }
    }

    fn slot_name(&self, kind: ItemKind, id: &str) -> Option<String> {
        match kind {
            ItemKind::Save => self.catalog.saves.iter().find(|file| file.id == id).map(|file| file.name.clone()),
            ItemKind::Recording => self.macro_by_id(id).map(|file| file.name.clone()),
            ItemKind::Combo => self.catalog.combos.iter().find(|file| file.id == id).map(|file| file.name.clone()),
            ItemKind::Sequence => self.catalog.sequences.iter().find(|file| file.id == id).map(|file| file.name.clone()),
            ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo => None,
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
            ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo => {}
        }
    }

    fn slot_remove(&mut self, kind: ItemKind, id: &str) {
        match kind {
            ItemKind::Save => self.catalog.saves.retain(|file| file.id != id),
            ItemKind::Recording => self.catalog.macros.retain(|file| file.id != id),
            ItemKind::Combo => self.catalog.combos.retain(|file| file.id != id),
            ItemKind::Sequence => self.catalog.sequences.retain(|file| file.id != id),
            ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo => {}
        }
    }

    fn connection_lines(&self, face: Face, item: &Item) -> Vec<ConnLine> {
        let role = self.role(face);
        match item.kind {
            ItemKind::Save => self.save_lines(&item.id),
            ItemKind::Recording => self.macro_lines(&item.id, role),
            ItemKind::Combo => self.combo_lines(&item.id),
            ItemKind::Sequence => self.sequence_lines(&item.id),
            ItemKind::NewChain => vec![ConnLine { text: "A new chain".to_string(), floppy: None }],
            ItemKind::NewSequence => vec![ConnLine { text: "A new sequence".to_string(), floppy: None }],
            ItemKind::SaveLastCombo => vec![ConnLine {
                text: "Saves the strip's latest exchange as a combo and drills it. A names it.".to_string(),
                floppy: None,
            }],
        }
    }

    fn save_lines(&self, id: &str) -> Vec<ConnLine> {
        let Some(save) = self.catalog.saves.iter().find(|file| file.id == id) else { return Vec::new() };
        let bound = self.bound.as_deref() == Some(id);
        let mut lines = vec![ConnLine { text: save.name.clone(), floppy: None }];
        if !save.fighters.is_empty() {
            lines.push(ConnLine { text: format!("Fighters: {}", save.fighters), floppy: None });
        }
        lines.push(ConnLine {
            text: format!("Bound to Save / Load: {}", if bound { "yes" } else { "no" }),
            floppy: None,
        });
        if bound {
            lines.push(ConnLine { text: "Load warps here".to_string(), floppy: None });
        }
        for file in &self.catalog.macros {
            let uses = if file.chain { self.chain_start_id(file).is_some_and(|save_id| save_id == id) } else { file.start_save.as_deref() == Some(id) };
            if uses {
                lines.push(ConnLine { text: format!("Start save of {}", file.name), floppy: self.row_floppy(&Item { kind: ItemKind::Recording, id: file.id.clone() }) });
            }
        }
        lines
    }

    fn macro_lines(&self, id: &str, role: ViewRole) -> Vec<ConnLine> {
        let Some(file) = self.macro_by_id(id).cloned() else { return Vec::new() };
        let mut lines = Vec::new();
        if file.chain {
            match self.chain_start_name(&file) {
                Some(name) => lines.push(ConnLine { text: format!("Starts from {name}"), floppy: self.row_floppy(&Item { kind: ItemKind::Recording, id: id.to_string() }) }),
                None => lines.push(ConnLine { text: "No start save · plays from where the fighters are".to_string(), floppy: None }),
            }
            lines.push(ConnLine { text: format!("Plays {} steps", file.steps.len()), floppy: None });
            for (index, step) in file.steps.iter().enumerate() {
                let name = self.macro_by_id(step).map(|file| file.name.clone()).unwrap_or_else(|| step.clone());
                lines.push(ConnLine { text: ordered_ids::step_line(index, &name), floppy: None });
            }
            if self.active[role.index()].as_deref() == Some(id) {
                lines.push(ConnLine { text: format!("Loop {}", self.loop_of(role).label()), floppy: None });
            }
            return lines;
        }
        match file.start_save.as_ref().and_then(|save| self.save_name(save)) {
            Some(name) => lines.push(ConnLine { text: format!("Starts from {name}"), floppy: self.row_floppy(&Item { kind: ItemKind::Recording, id: id.to_string() }) }),
            None => lines.push(ConnLine { text: "No start save · plays from where the fighters are".to_string(), floppy: None }),
        }
        for chain in self.catalog.macros.iter().filter(|chain| chain.chain) {
            if let Some(index) = chain.steps.iter().position(|step| step == id) {
                lines.push(ConnLine { text: format!("{} · step {}", chain.name, index + 1), floppy: None });
            }
        }
        for combo in self.catalog.combos.iter().filter(|combo| combo.made_from.as_deref() == Some(id)) {
            lines.push(ConnLine { text: format!("Combo {}", combo.name), floppy: None });
        }
        if !file.recorded_on.is_empty() {
            lines.push(ConnLine { text: format!("Recorded on {}", file.recorded_on), floppy: None });
        }
        lines
    }

    fn combo_lines(&self, id: &str) -> Vec<ConnLine> {
        let Some(combo) = self.catalog.combos.iter().find(|file| file.id == id) else { return Vec::new() };
        let mut lines = Vec::new();
        // First: the help joins these lines into one, and a long list of links must not push the fighter out (#49).
        if !combo.recorded_on.is_empty() {
            lines.push(ConnLine { text: format!("Recorded on {}", combo.recorded_on), floppy: None });
        }
        if let Some(source) = combo.made_from.as_ref().and_then(|id| self.macro_by_id(id).map(|file| file.name.clone())) {
            lines.push(ConnLine { text: format!("from {source}"), floppy: None });
        }
        for link in &combo.links {
            lines.push(ConnLine { text: link.clone(), floppy: None });
        }
        for sequence in &self.catalog.sequences {
            if sequence.combos.iter().any(|step| step == id) {
                lines.push(ConnLine { text: format!("In sequence {}", sequence.name), floppy: None });
            }
        }
        lines
    }

    fn sequence_lines(&self, id: &str) -> Vec<ConnLine> {
        let Some(sequence) = self.catalog.sequences.iter().find(|file| file.id == id) else { return Vec::new() };
        let mut lines = Vec::new();
        for (index, step) in sequence.combos.iter().enumerate() {
            let name = self.catalog.combos.iter().find(|file| file.id == step.as_str()).map(|file| file.name.clone()).unwrap_or_else(|| step.clone());
            lines.push(ConnLine { text: ordered_ids::step_line(index, &name), floppy: None });
        }
        lines.push(ConnLine { text: format!("streak {} · best {}", sequence.streak, sequence.best), floppy: None });
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
            ItemKind::NewChain | ItemKind::NewSequence | ItemKind::SaveLastCombo => Vec::new(),
        }
    }

    fn save_file_rows(&self, id: &str) -> Vec<BuiltRow> {
        let on = self.bound.as_deref() == Some(id);
        vec![
            BuiltRow { title: "Bind to Save / Load".to_string(), value: if on { "On" } else { "Off" }.to_string(), act: Act::Bind, drill: None },
            BuiltRow { title: "Duplicate".to_string(), value: String::new(), act: Act::Duplicate, drill: None },
            row_rename(),
            row_delete(),
        ]
    }

    fn macro_file_rows(&self, id: &str) -> Vec<BuiltRow> {
        let mut rows = Vec::new();
        if let Some(file) = self.macro_by_id(id) {
            if let Some(save) = file.start_save.as_ref() {
                let name = self.save_name(save).unwrap_or_else(|| save.clone());
                rows.push(BuiltRow {
                    title: "Start save".to_string(),
                    value: if file.start_save_on { format!("{name} · On") } else { "Off".to_string() },
                    act: Act::StartSave,
                    drill: None,
                });
            }
        }
        rows.push(self.active_row(id));
        rows.push(row_rename());
        rows.push(BuiltRow { title: "Crop".to_string(), value: String::new(), act: Act::Crop, drill: None });
        rows.push(BuiltRow { title: "Make combo".to_string(), value: String::new(), act: Act::MakeCombo, drill: None });
        rows.push(row_delete());
        rows
    }

    fn chain_file_rows(&self, id: &str) -> Vec<BuiltRow> {
        let mut rows = Vec::new();
        if let Some(file) = self.macro_by_id(id) {
            if let Some(name) = self.chain_start_name(file) {
                rows.push(BuiltRow {
                    title: "Start save".to_string(),
                    value: if file.start_save_on { format!("{name} · On") } else { "Off".to_string() },
                    act: Act::StartSave,
                    drill: None,
                });
            }
        }
        rows.push(self.active_row(id));
        rows.push(BuiltRow { title: "Edit chain".to_string(), value: String::new(), act: Act::EditChain, drill: None });
        rows.push(row_rename());
        rows.push(BuiltRow { title: "Make combo".to_string(), value: String::new(), act: Act::MakeCombo, drill: None });
        rows.push(row_delete());
        rows
    }

    fn combo_file_rows(&self, id: &str) -> Vec<BuiltRow> {
        let mut rows = vec![BuiltRow { title: "Drill".to_string(), value: "This becomes the drill".to_string(), act: Act::Drill, drill: None }];
        if let Some(combo) = self.catalog.combos.iter().find(|file| file.id == id) {
            if let Some(source) = combo.made_from.as_ref().and_then(|id| self.macro_by_id(id).map(|file| file.name.clone())) {
                rows.push(BuiltRow { title: "Made from".to_string(), value: format!("from {source}"), act: Act::Info, drill: None });
            }
        }
        rows.push(row_rename());
        rows.push(BuiltRow { title: "Crop".to_string(), value: String::new(), act: Act::Crop, drill: None });
        rows.push(row_delete());
        rows
    }

    fn sequence_file_rows(&self, id: &str) -> Vec<BuiltRow> {
        let mut rows = vec![BuiltRow { title: "Drill sequence".to_string(), value: "This becomes the drill".to_string(), act: Act::Drill, drill: None }];
        if let Some(sequence) = self.catalog.sequences.iter().find(|file| file.id == id) {
            for (index, step) in sequence.combos.iter().enumerate() {
                let name = self.catalog.combos.iter().find(|file| file.id == step.as_str()).map(|file| file.name.clone()).unwrap_or_else(|| step.clone());
                rows.push(BuiltRow {
                    title: ordered_ids::step_line(index, &name),
                    value: "A drills this combo".to_string(),
                    act: Act::DrillCombo,
                    drill: Some(step.clone()),
                });
            }
        }
        rows.push(BuiltRow { title: "Edit combos".to_string(), value: String::new(), act: Act::EditCombos, drill: None });
        rows.push(row_rename());
        rows.push(row_delete());
        rows
    }

    fn active_row(&self, id: &str) -> BuiltRow {
        let on = self.active[self.page_role.index()].as_deref() == Some(id);
        let value = if on { format!("On · {}", self.page_role.label()) } else { "Off".to_string() };
        BuiltRow { title: "Set active".to_string(), value, act: Act::SetActive, drill: None }
    }

    fn macro_by_id(&self, id: &str) -> Option<&MacroFile> {
        self.catalog.macros.iter().find(|file| file.id == id)
    }

    fn save_name(&self, id: &str) -> Option<String> {
        self.catalog.saves.iter().find(|file| file.id == id).map(|file| file.name.clone())
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

fn header_of(group: u8, section: u8) -> &'static str {
    match (group == COMBOS_SEQ, section) {
        (false, 0) => "SAVE STATES",
        (false, _) => "MACROS",
        (true, 0) => "SEQUENCES",
        _ => "COMBOS",
    }
}

fn page_by(cursor: usize, len: usize, forward: bool) -> usize {
    if len == 0 {
        return 0;
    }
    let cursor = cursor.min(len - 1);
    if forward { (cursor + PAGE).min(len - 1) } else { cursor.saturating_sub(PAGE) }
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

fn row_rename() -> BuiltRow {
    BuiltRow { title: "Rename".to_string(), value: String::new(), act: Act::Rename, drill: None }
}

fn row_delete() -> BuiltRow {
    BuiltRow { title: "Delete".to_string(), value: String::new(), act: Act::Delete, drill: None }
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

fn unused_n(prefix: &str, names: &[String]) -> u32 {
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
    line.strip_prefix('L').and_then(|rest| rest.split_once(' ')).is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// One link's move line, then its input events, then drift when the file recorded a hold.
/// An older presses-only file shows its presses and no drift.
pub fn combo_link_lines(combo: &Combo) -> Vec<String> {
    combo.links().enumerate().flat_map(|(index, link)| link_panel_lines(index, link)).collect()
}

fn link_panel_lines(index: usize, link: &super::combo::LinkRec) -> Vec<String> {
    let word = if link.is_true() { "TRUE" } else { "FALSE" };
    let mut lines = vec![format!(
        "L{} {} → {} {word}",
        index + 1,
        link.from.name().to_ascii_lowercase(),
        link.to.name().to_ascii_lowercase()
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
        Kind::Press(Button::Attack) => "A".to_string(),
        Kind::Press(Button::Special) => "B".to_string(),
        Kind::Press(Button::Shield) => "shield".to_string(),
        Kind::Press(Button::Grab) => "grab".to_string(),
        Kind::Release(_) => "release".to_string(),
        Kind::CStick => with_dir(event.dir, "c-stick"),
        Kind::Flick(Flick::FastFall) => with_dir(event.dir, "flick"),
        Kind::Flick(Flick::Dash) => with_dir(event.dir, "dash"),
        Kind::Flick(Flick::TapJump) => with_dir(event.dir, "tap"),
    }
}

fn with_dir(dir: super::combo::Dir, word: &str) -> String {
    match dir_arrow(dir) {
        "" => word.to_string(),
        arrow => format!("{arrow} {word}"),
    }
}

fn dir_arrow(dir: super::combo::Dir) -> &'static str {
    use super::combo::Dir;
    match dir {
        Dir::Forward => "→",
        Dir::Back => "←",
        Dir::Up => "↑",
        Dir::Down => "↓",
        // The UI font maps the four cardinal arrows and not the diagonals.
        Dir::ForwardUp => "↑→",
        Dir::BackUp => "↑←",
        Dir::ForwardDown => "↓→",
        Dir::BackDown => "↓←",
        Dir::Neutral | Dir::Side => "",
    }
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
    use super::*;
    use crate::model::chords::Request;
    use crate::model::combo::{Combo, LinkRec};
    use crate::model::frame_strip::{Move, Verdict};
    use crate::model::menu::input::Button;
    use crate::model::menu::table::{Control, Page, GROUPS, ROWS};
    use crate::model::menu::{Env, Menu};
    use crate::model::settings::{self, Settings};

    fn link(from: Move, to: Move, tru: bool) -> LinkRec {
        LinkRec::presses(from, to, if tru { Verdict::True(None) } else { Verdict::Drop(1) }, None, &[])
    }

    #[test]
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
            combos: vec![ComboFile {
                id: "nairc".into(),
                name: "Nair loop combo".into(),
                recorded_on: String::new(),
                made_from: Some("nair".into()),
                links: combo_link_lines(&Combo::from_links([link(Move::Aerial, Move::Aerial, true)]).unwrap()),
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

    #[test]
    fn cpu_view_has_one_tab_and_sections_scroll_apart() {
        let mut screens = screens();
        open_popup(&mut screens);
        screens.press_popup(Button::R);
        assert_eq!(screens.role(Face::Popup), ViewRole::Cpu);
        assert_eq!(screens.tab_names(), ["Save states & Macros"]);
        let tab = screens.tab();
        screens.press_popup(Button::ZL);
        screens.press_popup(Button::ZR);
        assert_eq!(screens.tab(), tab, "CPU has no tab control");
        screens.set_cursor(Face::Popup, SAVES_MACROS, 1, 2);
        let lower = screens.section_cursor(Face::Popup, SAVES_MACROS, 1);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 0);
        screens.press_popup(Button::JumpDown);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), PAGE);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 1), lower);
        assert!(screens.section(Face::Popup, SAVES_MACROS, 0).header == "SAVE STATES");
        assert!(screens.section(Face::Popup, SAVES_MACROS, 1).header == "MACROS");
    }

    #[test]
    fn saves_are_a_section_under_both_roles() {
        let mut screens = screens();
        open_popup(&mut screens);
        assert_eq!(screens.tab_names().len(), 2);
        assert_eq!(screens.section(Face::Popup, SAVES_MACROS, 0).header, "SAVE STATES");
        screens.press_popup(Button::R);
        assert_eq!(screens.section(Face::Popup, SAVES_MACROS, 0).header, "SAVE STATES");
        assert_eq!(screens.tab_names().len(), 1);
    }

    #[test]
    fn connections_follow_the_focused_row() {
        let mut screens = screens();
        open_popup(&mut screens);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 0);
        let first = screens.connections(Face::Popup, SAVES_MACROS);
        assert!(first.iter().any(|line| line.text.contains("Corner")));
        assert!(first.iter().any(|line| line.text == "Bound to Save / Load: yes"));
        screens.press_popup(Button::Down);
        let second = screens.connections(Face::Popup, SAVES_MACROS);
        assert!(second.iter().any(|line| line.text.contains("Platform")));
        assert!(!second.iter().any(|line| line.text == "Bound to Save / Load: yes"));
    }

    #[test]
    fn page_a_opens_and_popup_a_acts() {
        let mut screens = screens();
        let bound = screens.selection().bound.clone();
        screens.set_cursor(Face::Page, SAVES_MACROS, 0, 0);
        assert_eq!(screens.handle_page(Button::A, true, SAVES_MACROS), PageHit::Consumed);
        assert!(screens.file_open());
        assert_eq!(screens.file_rows()[0].title, "Bind to Save / Load");
        assert_eq!(screens.selection().bound, bound);
        assert!(screens.hint(Face::Page).contains("A open") || screens.file_open());
        screens.handle_page(Button::B, true, SAVES_MACROS);
        open_popup(&mut screens);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 1);
        screens.press_popup(Button::A);
        assert!(!screens.file_open());
        assert_eq!(screens.selection().bound.as_deref(), Some("platform"));
    }

    #[test]
    fn a_view_shows_one_roles_active_macro() {
        let mut screens = screens();
        open_popup(&mut screens);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 1, 0);
        let player = screens.section(Face::Popup, SAVES_MACROS, 1);
        let nair = player.rows.iter().find(|row| row.name == "Nair loop").unwrap();
        let chain = player.rows.iter().find(|row| row.name == "Corner pressure").unwrap();
        assert!(nair.green);
        assert!(!chain.green);
        screens.press_popup(Button::R);
        let cpu = screens.section(Face::Popup, SAVES_MACROS, 1);
        let nair = cpu.rows.iter().find(|row| row.name == "Nair loop").unwrap();
        let chain = cpu.rows.iter().find(|row| row.name == "Corner pressure").unwrap();
        assert!(!nair.green);
        assert!(chain.green);
        assert_eq!(screens.page_role(), ViewRole::Player, "the page role is its own");
    }

    #[test]
    fn the_bound_save_is_marked_in_both_roles() {
        let mut screens = screens();
        open_popup(&mut screens);
        let player = screens.section(Face::Popup, SAVES_MACROS, 0);
        assert!(player.rows.iter().find(|row| row.name == "Corner").unwrap().green);
        screens.press_popup(Button::R);
        let cpu = screens.section(Face::Popup, SAVES_MACROS, 0);
        assert!(cpu.rows.iter().find(|row| row.name == "Corner").unwrap().green);
    }

    #[test]
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

    #[test]
    fn sequences_sit_above_combos() {
        let screens = screens();
        let upper = screens.section(Face::Page, COMBOS_SEQ, 0);
        let lower = screens.section(Face::Page, COMBOS_SEQ, 1);
        assert_eq!(upper.header, "SEQUENCES");
        assert_eq!(lower.header, "COMBOS");
        assert!(upper.rows.iter().any(|row| row.name == "Corner to kill"));
        assert!(lower.rows.iter().any(|row| row.name == "Nair loop combo"));
    }

    #[test]
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
        screens.press_popup(Button::ZR);
        assert_eq!(screens.tab(), 0);
        screens.press_popup(Button::L);
        assert_eq!(screens.role(Face::Popup), ViewRole::Player);
        screens.press_popup(Button::B);
        assert!(!screens.popup_open());
    }

    #[test]
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
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), len - 1 - PAGE);
        screens.set_cursor(Face::Page, SAVES_MACROS, 0, 0);
        screens.handle_page(Button::JumpDown, true, SAVES_MACROS);
        assert_eq!(screens.section_cursor(Face::Page, SAVES_MACROS, 0), 12);
    }

    #[test]
    fn b_closes_the_popup_without_changing_the_selection() {
        let mut screens = screens();
        open_popup(&mut screens);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 3);
        let bound = screens.selection().bound.clone();
        let active = screens.selection().active_player.clone();
        screens.press_popup(Button::B);
        assert!(!screens.popup_open());
        open_popup(&mut screens);
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), 3);
        assert_eq!(screens.selection().bound, bound);
        assert_eq!(screens.selection().active_player, active);
    }

    #[test]
    fn zr_switches_role_on_the_recording_page() {
        let mut menu = menu();
        to_recording(&mut menu);
        menu.recording_mut().set_catalog(example());
        let before = menu.page();
        menu.press(Button::ZR);
        assert_eq!(menu.page(), before);
        assert_eq!(menu.recording().page_role(), ViewRole::Cpu);
        menu.press(Button::ZR);
        assert_eq!(menu.recording().page_role(), ViewRole::Player);
    }

    #[test]
    fn l_and_r_still_change_the_menu_page() {
        let mut menu = menu();
        to_recording(&mut menu);
        menu.press(Button::R);
        assert_eq!(menu.page(), Page::FightersStages);
        menu.press(Button::L);
        assert_eq!(menu.page(), Page::Recording);
        menu.recording_mut().set_catalog(example());
        menu.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 0, 0);
        menu.press(Button::A);
        assert!(menu.recording().file_open());
        menu.press(Button::L);
        menu.press(Button::R);
        assert_eq!(menu.page(), Page::Recording, "a file screen keeps L and R");
    }

    #[test]
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
        loop {
            if menu.focused_row() == Some(mash) {
                break;
            }
            menu.press(Button::Down);
        }
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

    #[test]
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

    #[test]
    fn a_chain_screen_has_no_crop() {
        let mut screens = screens();
        let chain = screens.section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == "Corner pressure").unwrap();
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, chain);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        let titles: Vec<_> = screens.file_rows().into_iter().map(|row| row.title).collect();
        assert!(!titles.iter().any(|title| title == "Crop"));
        assert!(titles.iter().any(|title| title == "Edit chain"));
        screens.handle_page(Button::B, true, SAVES_MACROS);
        let macro_row = screens.section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == "Nair loop").unwrap();
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, macro_row);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        let titles: Vec<_> = screens.file_rows().into_iter().map(|row| row.title).collect();
        assert!(titles.iter().any(|title| title == "Crop"));
        assert!(!titles.iter().any(|title| title == "Edit chain"));
    }

    #[test]
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

    #[test]
    fn the_combo_panel_reads_the_file_links() {
        let combo = Combo::from_links([link(Move::Aerial, Move::Aerial, true), link(Move::Aerial, Move::Aerial, true)]).unwrap();
        let lines = combo_link_lines(&combo);
        assert_eq!(lines[0], "L1 aerial → aerial TRUE");
        let mut screens = screens();
        screens.set_links("nairc", lines);
        // Row 0 of COMBOS is + Save last combo…; the combo is row 1.
        screens.set_cursor(Face::Page, COMBOS_SEQ, 1, 1);
        let panel = screens.connections(Face::Page, COMBOS_SEQ);
        assert!(panel.iter().any(|line| line.text == "L1 aerial → aerial TRUE"));
    }

    #[test]
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
        let lines = combo_link_lines(&combo);
        assert_eq!(lines[0], "L1 aerial → aerial TRUE");
        assert_eq!(lines[1], "jump · release · ↓ flick · A");
        assert_eq!(lines[2], "drift →");
        assert_eq!(lines[3], "L2 aerial → aerial TRUE");
        assert_eq!(lines[4], "jump · release · ↓ flick · A");
        assert_eq!(lines[5], "drift ←");
        let mut screens = screens();
        screens.set_links("nairc", lines);
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
        let old_lines = combo_link_lines(&old);
        assert_eq!(old_lines, vec!["L1 aerial → aerial TRUE".to_string(), "A · jump".to_string()]);
        assert!(old_lines.iter().all(|line| !line.contains("drift") && !line.contains("release") && !line.contains("flick")));
    }

    #[test]
    fn player_text_says_sequence_and_the_helper_says_link() {
        assert!(GROUPS.iter().any(|group| group.title == "Combos & Sequences"));
        assert!(GROUPS.iter().all(|group| !group.title.to_ascii_lowercase().contains("conversion")));
        let screens = screens();
        let section = screens.section(Face::Page, COMBOS_SEQ, 0);
        assert!(section.rows.iter().any(|row| row.name == "+ New sequence"));
        assert_eq!(section.header, "SEQUENCES");
        let helper = ROWS.iter().find(|row| row.id == "combo_helper").unwrap();
        assert!(helper.help_lines().0.contains("link") || helper.help_lines().1.contains("link"));
        let phrases = ["+ New sequence", "Drill sequence", "SEQUENCES", "sequence"];
        assert!(phrases.iter().all(|phrase| !phrase.contains("conversion")));
    }

    #[test]
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

    #[test]
    fn the_start_save_row_of_a_file_screen_carries_the_rows_floppy() {
        let mut screens = screens();
        let index = screens.section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == "Nair loop").unwrap();
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, index);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        let rows = screens.file_rows();
        assert_eq!(rows[0].title, "Start save");
        assert_eq!(rows[0].floppy, Some(Floppy::On));
        assert!(rows[1..].iter().all(|row| row.floppy.is_none()));
        screens.handle_page(Button::Left, true, SAVES_MACROS);
        assert_eq!(screens.file_rows()[0].floppy, Some(Floppy::Off), "grey with the switch Off, like the list row");
        assert!(screens.crop_rows().iter().all(|row| row.floppy.is_none()));
    }

    #[test]
    fn y_does_nothing_on_the_new_screens() {
        let mut screens = screens();
        open_popup(&mut screens);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 2);
        let before = screens.selection();
        screens.press_popup(Button::Y);
        assert!(screens.popup_open());
        assert_eq!(screens.section_cursor(Face::Popup, SAVES_MACROS, 0), 2);
        assert_eq!(screens.selection().bound, before.bound);
        assert_eq!(screens.loop_of(ViewRole::Player), LoopMark::Finite(1));

        let mut menu = menu();
        to_recording(&mut menu);
        let mash = ROWS.iter().position(|row| row.id == "playback_mash").unwrap();
        for _ in 0..menu.entry_count(Page::Recording) + 8 {
            if menu.focused_row() == Some(mash) {
                break;
            }
            menu.press(Button::Down);
        }
        assert_eq!(menu.focused_row(), Some(mash));
        menu.press(Button::A);
        let changed = menu.settings().menu.get("playback_mash").unwrap().to_vec();
        menu.press(Button::Y);
        assert_eq!(menu.settings().menu.get("playback_mash").unwrap(), changed.as_slice());
    }

    #[test]
    fn new_chain_and_new_sequence_open_the_picker() {
        let mut screens = screens();
        let last = screens.section(Face::Page, SAVES_MACROS, 1).rows.len() - 1;
        assert_eq!(screens.section(Face::Page, SAVES_MACROS, 1).rows[last].name, "+ New chain");
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, last);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert!(screens.picker_open());
        assert!(screens.picker_recording_ids().iter().all(|id| id != "pressure"), "a chain is not a step");
        screens.handle_page(Button::B, true, SAVES_MACROS);
        assert!(matches!(screens.last_effect(), Effect::Keyboard { header: "Name chain", .. }));
        screens.submit_name("");
        assert!(screens.has_chain_named("Chain 1"));

        let last = screens.section(Face::Page, COMBOS_SEQ, 0).rows.len() - 1;
        assert_eq!(screens.section(Face::Page, COMBOS_SEQ, 0).rows[last].name, "+ New sequence");
        screens.set_cursor(Face::Page, COMBOS_SEQ, 0, last);
        screens.handle_page(Button::A, true, COMBOS_SEQ);
        assert!(screens.picker_open());
    }

    #[test]
    fn the_crop_screen_posts_a_new_range_and_b_returns() {
        let mut screens = screens();
        let row = screens.section(Face::Page, SAVES_MACROS, 1).rows.iter().position(|row| row.name == "Nair loop").unwrap();
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, row);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        let crop = screens.file_rows().iter().position(|row| row.title == "Crop").unwrap();
        screens.set_file_cursor(crop);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert!(screens.crop_open());
        assert!(matches!(screens.last_effect(), Effect::None));
        assert!(screens.diag_line(Face::Page, SAVES_MACROS).contains("screen=crop source=nair combo=0"));
        screens.handle_page(Button::R, true, SAVES_MACROS);
        screens.handle_page(Button::Down, true, SAVES_MACROS);
        screens.handle_page(Button::L, true, SAVES_MACROS);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        match screens.last_effect() {
            Effect::Crop { source, combo, start, end } => {
                assert_eq!((source.as_str(), *combo, *start, *end), ("nair", false, 1, 11));
            }
            other => panic!("crop effect, got {other:?}"),
        }
        assert!(!screens.crop_open());
        assert!(screens.catalog().macros.iter().any(|file| file.id == "nair" && file.name == "Nair loop"));
        let _ = screens.take_effect();

        screens.handle_page(Button::A, true, SAVES_MACROS);
        assert!(screens.crop_open());
        screens.handle_page(Button::B, true, SAVES_MACROS);
        assert!(!screens.crop_open());
        assert!(matches!(screens.take_effect(), Effect::None));
    }

    #[test]
    fn the_waiting_room_names_chain_n_without_a_keyboard() {
        let mut screens = screens();
        screens.set_keyboard_blocked(true);
        let last = screens.section(Face::Page, SAVES_MACROS, 1).rows.len() - 1;
        screens.set_cursor(Face::Page, SAVES_MACROS, 1, last);
        screens.handle_page(Button::A, true, SAVES_MACROS);
        screens.handle_page(Button::B, true, SAVES_MACROS);
        assert!(!screens.naming());
        assert!(screens.has_chain_named("Chain 1"));
        assert!(matches!(screens.last_effect(), Effect::WriteChain { .. }));
    }

    #[test]
    fn the_popup_diag_names_the_hot_boundary() {
        let mut screens = screens();
        open_popup(&mut screens);
        let line = screens.diag_line(Face::Popup, SAVES_MACROS);
        assert!(line.contains("screen=recording-popup"));
        assert!(line.contains("role=player"));
        assert!(line.contains("loop="));
    }

    #[test]
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

    #[test]
    fn the_recording_page_is_the_page_the_menu_shows() {
        assert_eq!(Page::Recording.title(), "Recording");
        let mut menu = menu();
        to_recording(&mut menu);
        assert_eq!(menu.page().title(), "Recording");
        let titles: Vec<_> = GROUPS.iter().filter(|group| group.page == Page::Recording).map(|group| group.title).collect();
        assert_eq!(titles, ["Save states & Macros", "Combos & Sequences", "Replays", "Options"]);
        assert!(matches!(ROWS.iter().find(|row| row.id == "playback_loop").unwrap().control, Control::Dropdown(labels) if labels.len() == 6));
    }

    /// Operator 2026-11-08 (primary inbox 13:08Z): "+ Save last combo…" heads the COMBOS list on the Recording page and in
    /// the pop-up's Combos & Sequences tab. A opens the keyboard titled for a combo; the combos keep their hot marks.
    #[test]
    fn save_last_combo_heads_the_combos_list_and_opens_the_keyboard() {
        for face in [Face::Page, Face::Popup] {
            let mut screens = screens();
            if face == Face::Popup {
                open_popup(&mut screens);
                screens.press_popup(Button::ZR);
            }
            let combos = screens.section(face, COMBOS_SEQ, 1);
            assert_eq!(combos.rows[0].name, "+ Save last combo…", "{face:?}");
            assert!(!combos.rows[0].hot && !combos.rows[0].green);
            assert_eq!(combos.rows[1].name, "Nair loop combo");
            assert!(combos.rows[1].hot, "a hot combo stays hot under the new row");
            screens.set_cursor(face, COMBOS_SEQ, 1, 0);
            let lines = screens.connections(face, COMBOS_SEQ);
            assert!(lines.iter().any(|line| line.text.contains("latest exchange")), "{lines:?}");
            match face {
                Face::Page => {
                    screens.handle_page(Button::A, true, COMBOS_SEQ);
                }
                Face::Popup => screens.press_popup(Button::A),
            }
            assert_eq!(screens.take_effect(), Effect::Keyboard { header: "Name combo", draft: String::new() }, "{face:?}");
        }
    }

    /// RECORDING.md: "The five most recently used of a kind are listed first, most recent first." A drill and a bind are
    /// uses (the index notes them); the live list follows at once, as Set active already does, not only after the model
    /// is rebuilt (Eden rc6-covrec-diag2 cov-combo-other-fighter: the pop-up kept Luigi first after Mario was drilled).
    #[test]
    fn a_drill_or_a_bind_in_the_popup_puts_that_file_first() {
        let mut screens = screens();
        let mut catalog = example();
        catalog.combos.push(ComboFile {
            id: "other".into(),
            name: "Another combo".into(),
            recorded_on: String::new(),
            made_from: None,
            links: Vec::new(),
            span: 1,
        });
        screens.set_catalog(catalog);
        open_popup(&mut screens);
        screens.press_popup(Button::ZR);
        // COMBOS: + Save last combo…, Nair loop combo (hot), Another combo.
        screens.set_cursor(Face::Popup, COMBOS_SEQ, 1, 2);
        screens.press_popup(Button::A);
        let combos = screens.section(Face::Popup, COMBOS_SEQ, 1);
        assert_eq!(combos.rows[1].name, "Another combo");
        assert!(combos.rows[1].hot && combos.rows[1].green);
        screens.press_popup(Button::ZR);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 2);
        let third = screens.section(Face::Popup, SAVES_MACROS, 0).rows[2].name.clone();
        screens.press_popup(Button::A);
        assert_eq!(screens.section(Face::Popup, SAVES_MACROS, 0).rows[0].name, third);
    }

    /// #49 in rc.6 terms: the COMBOS list replaces Saved combos, so a combo's CONNECTIONS name the fighter it was recorded
    /// on, first, as a macro's do (rc.5 fixes2 footer; r8 mockup "Fighters"). The help joins its lines into one, so the
    /// fighter leads and a long list of links cannot push it out.
    #[test]
    fn a_combos_connections_name_its_recorded_fighter_first() {
        let mut screens = screens();
        let mut catalog = example();
        catalog.combos[0].recorded_on = "Luigi".into();
        screens.set_catalog(catalog);
        screens.set_cursor(Face::Page, COMBOS_SEQ, 1, 1);
        let lines = screens.connections(Face::Page, COMBOS_SEQ);
        assert_eq!(lines.first().map(|line| line.text.as_str()), Some("Recorded on Luigi"), "{lines:?}");
    }

    /// Primary 2026-11-08 15:55Z: the r8 pop-up as drawn. Its CONNECTIONS are kind boxes in a tree
    /// (`RECORDING-REDESIGN-r8-popup-combo.png`, `-macro.png`, `-save.png`): the row under the cursor, a heading, the
    /// files joined to it, then plain lines.
    #[test]
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

    /// The r8 mockup counts each section's files beside its header ("SAVE STATES (7)"); action rows are not files.
    #[test]
    fn sections_count_their_files() {
        let screens = screens();
        let files = |group, section| screens.section(Face::Popup, group, section).files;
        assert_eq!((files(SAVES_MACROS, 0), files(SAVES_MACROS, 1)), (16, 4));
        assert_eq!((files(COMBOS_SEQ, 0), files(COMBOS_SEQ, 1)), (1, 1));
    }

    /// The r8 footer names what A does in the section under the cursor, then the pop-up's keys
    /// (`RECORDING-REDESIGN-r8-popup-save.png`: "A bind", `-macro.png`: "A set active   X loop", `-combo.png`: "A drill").
    #[test]
    fn the_popup_footer_is_the_r8_footer() {
        let mut screens = screens();
        open_popup(&mut screens);
        let keys = "   Left/Right section   ZL/ZR tab   L/R role   ZL+Up/Down page   B close";
        screens.set_cursor(Face::Popup, SAVES_MACROS, 0, 0);
        assert_eq!(screens.hint(Face::Popup), format!("A bind{keys}"));
        screens.set_cursor(Face::Popup, SAVES_MACROS, 1, 0);
        assert_eq!(screens.hint(Face::Popup), format!("A set active   X loop{keys}"));
        screens.press_popup(Button::ZR);
        screens.set_cursor(Face::Popup, COMBOS_SEQ, 1, 1);
        assert_eq!(screens.hint(Face::Popup), format!("A drill{keys}"));
        screens.press_popup(Button::R);
        assert_eq!(screens.role(Face::Popup), ViewRole::Cpu);
        screens.set_cursor(Face::Popup, SAVES_MACROS, 1, 0);
        assert_eq!(
            screens.hint(Face::Popup),
            "A set active   X loop   Left/Right section   L/R role   ZL+Up/Down page   B close",
            "the CPU has one tab"
        );
    }

    /// Primary 2026-11-08 (#49, #19): the r8 mockup's line under each row (`RECORDING-REDESIGN-r8-popup-combo.png`,
    /// `-macro.png`): "Recorded on …" under a combo and a macro, the fighters under a save, the steps under a chain and
    /// a sequence. An action row has none.
    #[test]
    fn rows_carry_the_r8_sublines() {
        let mut screens = screens();
        let mut catalog = example();
        catalog.combos[0].recorded_on = "Pikachu → Mario".into();
        screens.set_catalog(catalog);
        let row = |screens: &RecordingScreens, group: u8, section: u8, name: &str| {
            let view = screens.section(Face::Popup, group, section);
            view.rows.iter().find(|row| row.name == name).map(|row| row.sub.clone()).unwrap_or_else(|| panic!("{name}: {view:?}"))
        };
        assert_eq!(row(&screens, COMBOS_SEQ, 1, "Nair loop combo"), "Recorded on Pikachu → Mario");
        assert_eq!(row(&screens, COMBOS_SEQ, 1, "+ Save last combo…"), "");
        assert_eq!(row(&screens, COMBOS_SEQ, 0, "Corner to kill"), "Nair loop combo");
        assert_eq!(row(&screens, SAVES_MACROS, 0, "Corner"), "Pikachu · Mario");
        assert_eq!(row(&screens, SAVES_MACROS, 1, "Nair loop"), "Recorded on Pikachu");
        assert_eq!(row(&screens, SAVES_MACROS, 1, "Corner pressure"), "Nair loop → Dash grab → CPU throw");
        assert_eq!(row(&screens, SAVES_MACROS, 1, "Dash grab"), "", "nothing recorded on file");
    }

    /// The typed name goes to the host with the save; an empty name lets the host pick Combo N. Where no keyboard can
    /// open (an online room), A saves at once under Combo N.
    #[test]
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
        screens.set_keyboard_blocked(true);
        screens.handle_page(Button::A, true, COMBOS_SEQ);
        assert_eq!(screens.take_effect(), Effect::SaveLastCombo { name: String::new() });
    }
}
