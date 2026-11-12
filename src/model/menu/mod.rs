//! The menu model (SPEC-core-02, SPEC-core-03 input rules, SPEC-core-04 persistence of the last page,
//! SPEC-input-04 chord editing; layout `MENU-DRAFT.md` §2, draft 9): the pure state machine behind
//! the in-game menu. It draws nothing and touches no game memory. The input adapter feeds it
//! logical buttons (`Menu::press`); the renderer reads the page, the entries, the cursor and the
//! screen on top of the stack; what the menu asks the runtime to do (publish settings, re-roll a CPU
//! choice, write an INI file) is collected in an `Outbox` the runtime takes.
//!
//! Values live in the settings model (`model::settings`), one row per setting id; this module only
//! knows how each row is edited. Allocation happens on input events only (a screen pushed, a row
//! written), never per frame.

pub mod chord;
pub mod delay;
pub mod grid;
pub mod input;
pub mod options;
pub mod table;

use chord::Chord;
use delay::{DelayRange, DelaySpec};
use grid::{ConsoleMode, GridKind, GridLayout, Move, RosterUse};
use input::Button;
use options::{CHORD_ROWS, SITUATIONS, SWAP_GROUP_IDS};
use table::{Control, Editor, Page, Row, GROUPS, ROWS};

use super::settings::{self, Settings};

/// Everything the menu needs to know about the install and the console.
#[derive(Clone, Copy, Debug)]
pub struct Env {
    /// `developer.ini` has `developer_page=on` (N1).
    pub developer_page: bool,
    /// The standalone CPU pilot plugin is on the card (Q11).
    pub pilot_present: bool,
    /// The operation mode now (read again each time a grid opens).
    pub console: ConsoleMode,
    /// `button_config.ini`'s two menu shortcuts (N7).
    pub shortcuts: [Chord; 2],
    /// `replay_capture.ini` (Q7).
    pub replay_capture: bool,
    /// The combo helper's target and draft names (SPEC-input-07 §6), kept by the menu worker.
    pub combo_target: super::combo::Label,
    pub combo_draft: super::combo::Label,
    /// The build tag the page's key hints end with (R13): `frame::BUILD_TAG` on the console; empty adds nothing.
    pub build_tag: &'static str,
}

impl Default for Env {
    fn default() -> Env {
        Env {
            developer_page: false,
            pilot_present: false,
            console: ConsoleMode::Docked,
            shortcuts: [Chord::DEFAULT_SHORTCUT_1, Chord::NONE],
            // The file of record is the INI, which defaults to off when absent (lead ruling 2026-11-02);
        // the runtime sets this from it at boot.
        replay_capture: false,
            combo_target: super::combo::Label::EMPTY,
            combo_draft: super::combo::Label::EMPTY,
            build_tag: "",
        }
    }
}

/// CPU choices a menu change makes the CPU take again (Q18 A, B11). Bits of `Outbox::rerolls`.
pub mod reroll {
    /// SDI Direction changed: a new weighted roll at the next CPU frame.
    pub const SDI_DIRECTION: u8 = 1;
    /// Shield Tilt changed.
    pub const SHIELD_TILT: u8 = 2;
    /// Ledge Options changed: the held ledge case is cleared (B11); a Ledge Delay change is not here.
    pub const LEDGE_OPTIONS: u8 = 4;
    /// Throw Options changed: the held throw case is cleared (B11).
    pub const THROW_OPTIONS: u8 = 8;
}

/// The setting ids whose change re-rolls a CPU choice, with the bit each sets.
const REROLL_IDS: [(&str, u8); 4] = [
    ("sdi_state", reroll::SDI_DIRECTION),
    ("shield_tilt", reroll::SHIELD_TILT),
    ("ledge_state", reroll::LEDGE_OPTIONS),
    ("throw_state", reroll::THROW_OPTIONS),
];

/// The re-roll bits that replacing the whole settings `old` by `new` makes (the menu sets them one row at a
/// time in `put`; the diag build's settings reload replaces a whole snapshot, so it asks this).
pub fn rerolls_between(old: &settings::Snapshot, new: &settings::Snapshot) -> u8 {
    REROLL_IDS.iter().filter(|(id, _)| old.get(id) != new.get(id)).fold(0, |bits, (_, bit)| bits | bit)
}

/// What the menu asks the runtime to do. Taken with `Menu::take_outbox`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outbox {
    /// Any setting changed since the last take (the commit publishes before it queues the write).
    pub changed: bool,
    /// `reroll::*` bits.
    pub rerolls: u8,
    /// A confirmed change of a menu shortcut: write `button_config.ini` with these two chords and
    /// apply them at once (N7).
    pub button_config: Option<[Chord; 2]>,
    /// The Replay capture toggle changed: write `replay_capture.ini` (Q7).
    pub replay_capture: Option<bool>,
    /// A on a tile of the Give item grid: the item (index of `ITEM_NAMES`) to give Player 1 now. Taken at once with
    /// `Menu::take_give_item` (the request is made on Player 1's next game frame, not at close).
    pub give_item: Option<u8>,
    /// A, X or Y on a tile of the Lobby Stages grid (tile 0 is All): the stage editor (ditors::stages, which knows the carried
    /// stage catalog) applies it. Taken at once with Menu::take_stage_press.
    pub stage_press: Option<(u16, Button)>,
    /// A, X, Y, ZR on the portrait roster grid: the tile and the button; the roster editor (package 6's roster rules) applies it.
    /// Taken at once with Menu::take_roster_press.
    pub roster_press: Option<(u16, Button)>,
}

/// Why a row or entry is greyed (it shows dimmed with this reason and ignores A).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grey {
    /// Respawn on platform while Auto-load on death is On (N2): a save state loads instead.
    AutoLoadOn,
    /// A damage range while its damage setting is not Random Value.
    DamageNotRandom,
    /// A swap group while Group Swap is Off.
    GroupSwapOff,
    /// A swap group that needs the Ice Climbers pair bank, while it is Off.
    PairBankOff,
    /// The Ice Climbers pair bank while Ready Fighter Bodies is not 3.
    NeedsThreeBodies,
    /// The checklist of a delay row while a range is set.
    RangeSet,
    /// The range of a delay row while the checklist holds a value.
    ChecklistSet,
}

impl Grey {
    pub const fn text(self) -> &'static str {
        match self {
            Grey::AutoLoadOn => "Auto-load on death is On: a save state loads instead",
            Grey::DamageNotRandom => "Only used when damage on load is Random Value",
            Grey::GroupSwapOff => "Group Swap is Off",
            Grey::PairBankOff => "Ice Climbers pair bank is Off",
            Grey::NeedsThreeBodies => "Needs Ready Fighter Bodies set to 3",
            Grey::RangeSet => "A range is set: clear it first (Y)",
            Grey::ChecklistSet => "Values are ticked: clear them first (Y)",
        }
    }
}

/// An entry of a page's list: a group header or a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Group(usize),
    Row(usize),
    /// The last line of every page, *Reset page to defaults*: A asks to confirm, then restores the page from the
    /// saved defaults. It belongs to the page's last section (`usize` is that group).
    ResetPage(usize),
}

/// A row's value as the renderer shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowValue {
    Toggle(bool),
    /// The chosen option of a list, `None` when the array holds no selection.
    Choice(Option<usize>),
    /// The chosen number of a `Number` row.
    Number(Option<u16>),
    /// `ticked` of `total` entries.
    Checklist { ticked: usize, total: usize },
    /// A delay row: nothing set, `n selected`, or `min - max`.
    Delay(DelayView),
    Slider { lower: u8, upper: u8 },
    /// The lit chips of a direction grid, bit `i` for array index `i` (the nine dots of the row).
    Chips(u16),
    /// A menu shortcut's chord.
    Chord(Chord),
    /// *Situation overrides*: how many of the 13 situations have an action set.
    OverridesSet(usize),
    None,
}

/// A delay row's state for display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelayView {
    Empty,
    Ticked(usize),
    Range(DelayRange),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderState {
    LowerHover,
    UpperHover,
    LowerSelected,
    UpperSelected,
}

/// What a chord entry will write when confirmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChordTarget {
    /// One of the six rows of `CHORD_ROWS`.
    Row(u8),
    /// Menu shortcut 1 or 2.
    Shortcut(u8),
}

/// Separate physical capture from preview controls, so A/B/X/Y are capturable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChordCapture {
    #[default]
    Preview,
    /// The arming X and every button already held with it must first be released.
    Release,
    Buttons(Chord),
}

/// A screen on the sub-screen stack (the page itself is the root, below the stack).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// The popup list of a dropdown, Yes/No/Random or number row.
    List { row: usize, cursor: usize },
    /// A multi-select popup.
    Checklist { row: usize, cursor: usize },
    /// A delay row's popup (N3): cursor `0..len` is a checklist entry, `len` the Min handle, `len + 1`
    /// the Max handle; `editing` means the handle under the cursor is selected.
    Delay { row: usize, cursor: usize, editing: bool },
    Slider { row: usize, state: SliderState },
    /// A direction grid; `chip` is the array index of the focused chip.
    Directions { row: usize, chip: usize },
    SwapGroups { cursor: usize },
    /// *Situation overrides*: cursor `0` is the situation, `1..=30` an action.
    Overrides { situation: usize, cursor: usize },
    /// *Button chords*: the six chord rows.
    Chords { cursor: usize },
    /// The chord editor (SPEC-input-04): `captured` is what the next confirm writes.
    ChordEntry { target: ChordTarget, captured: Chord },
    /// The item or stage grid, its layout fixed when it opened.
    Grid { layout: GridLayout, page: usize, cell: usize },
    /// A sub-screen whose content belongs to a later package.
    External(Editor),
    /// *Reset page to defaults?*: A resets the page, B cancels.
    ConfirmReset,
}

/// Where the menu was when it closed (`Menu::place`): kept by the runtime outside the model for the rest of the boot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    /// The page showing. A close commits it as `last_page` too; a session that ended without a close (a scene
    /// change, the renderer lost) did not, so the page is kept here as well.
    page: Page,
    cursor: [usize; 6],
    expanded: u32,
    /// The page and the sub-screen open on it, when that screen can be resumed.
    screen: Option<(Page, Screen)>,
}

/// The worker's memory of where the menu was, across the model drops of a close, a scene change or a Training reset,
/// for the rest of the boot. The worker drops its model whenever the input generation moves (every close and every
/// session clear) and builds or reopens one at the next opening; both go through here, so the save and the restore are
/// host-tested as the worker runs them (review MN-4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlaceKeeper {
    place: Option<Place>,
}

impl PlaceKeeper {
    pub const fn new() -> PlaceKeeper {
        PlaceKeeper { place: None }
    }

    /// The model is dropped: keep where it was. With no model (already dropped, or the menu never opened this
    /// session) the place kept before stays.
    pub fn drop_model(&mut self, menu: &mut Option<Menu>) {
        if let Some(m) = menu.take() {
            self.place = Some(m.place());
        }
    }

    /// An opening: the model still held is reopened with `settings`, else a new one is built; then it comes back to the
    /// place kept (`Menu::restore_place`, clamped to the table as it stands now). A model still held would have its
    /// own cursors replaced by the older place; no path reaches that today, since every close and session clear drops
    /// the model first (review MN-3, left as it is).
    pub fn open<'a>(&self, menu: &'a mut Option<Menu>, settings: Settings, env: Env) -> &'a mut Menu {
        let opened = match menu.take() {
            Some(mut m) => {
                m.reopen(settings, env);
                m
            }
            None => Menu::new(settings, env),
        };
        let m = menu.insert(opened);
        if let Some(place) = self.place {
            m.restore_place(place);
        }
        m
    }

    pub fn place(&self) -> Option<Place> {
        self.place
    }
}

/// The sub-screens the model owns entirely, which can be reopened as they were (a handle being dragged or a
/// range being edited is put down first). The chord editor (a capture in progress), the grids (their layout, the
/// textures and the requests belong to the runtime) and the external editors (the runtime's panels) are not
/// resumed: their parent row is focused instead.
fn resumable(screen: Screen) -> Option<Screen> {
    match screen {
        Screen::Delay { row, cursor, .. } => Some(Screen::Delay { row, cursor, editing: false }),
        Screen::Slider { row, state } => Some(Screen::Slider {
            row,
            state: match state {
                SliderState::LowerSelected | SliderState::LowerHover => SliderState::LowerHover,
                SliderState::UpperSelected | SliderState::UpperHover => SliderState::UpperHover,
            },
        }),
        Screen::List { .. }
        | Screen::Checklist { .. }
        | Screen::Directions { .. }
        | Screen::SwapGroups { .. }
        | Screen::Overrides { .. }
        | Screen::Chords { .. } => Some(screen),
        Screen::ChordEntry { .. } | Screen::Grid { .. } | Screen::External(_) | Screen::ConfirmReset => None,
    }
}

/// The delay popup's checklist is drawn as columns of this many boxes (mockup 19).
pub const DELAY_ROWS: usize = 4;

/// The situation overrides screen draws its 30 actions in three columns of this many boxes (mockup 06 shows 13 in two).
pub const OVERRIDE_COLUMN: usize = 10;

/// How far ZL + Up / Down moves in a popup list: one window of lines, the rows the sub-screens draw at once
/// (`ui::panes::ROWS`; a test keeps them equal).
pub const LIST_JUMP: usize = 12;

/// The section an entry belongs to.
fn group_of(entry: Entry) -> usize {
    match entry {
        Entry::Group(g) | Entry::ResetPage(g) => g,
        Entry::Row(r) => usize::from(ROWS[r].group),
    }
}

/// A list cursor after ZL + Up / Down: one window further, stopping at the first or last entry (no wrap).
fn jumped(cursor: usize, len: usize, forward: bool) -> usize {
    if forward { (cursor + LIST_JUMP).min(len.saturating_sub(1)) } else { cursor.saturating_sub(LIST_JUMP) }
}

/// The menu.
#[derive(Clone, Debug)]
pub struct Menu {
    settings: Settings,
    env: Env,
    page: Page,
    /// The focused entry of each page.
    cursor: [usize; 6],
    /// Bit `g` set: advanced group `g` is open.
    expanded: u32,
    stack: Vec<Screen>,
    outbox: Outbox,
    closed: bool,
    chord_capture: ChordCapture,
    chord_refusal: Option<super::ini_files::EditRefusal>,
    /// The row ids, kept keys and top-level keys the player changed this menu session
    /// (decision MENU-CLOSE): what the close commits on top of the live settings.
    edits: Vec<String>,
    /// The last confirmed page reset: the page and the rows it changed, for the runtime's log line.
    page_reset: Option<(Page, Vec<&'static str>)>,
    /// The recording pop-up and the Recording page's library. Not a settings row.
    recording: super::recording_menu::RecordingScreens,
}

const SLIDER_SLOW: u8 = 1;
const SLIDER_FAST: u8 = 10;
/// Faster step of a handle in a delay range, in option steps.
const RANGE_FAST: usize = 10;

impl Menu {
    /// Open the menu on `settings`: on the page last used (`last_page`), CPU when there is none or
    /// the saved page is not in the ring (the Developer page with the flag off), Q5.
    /// The recording pop-up and the Recording page's library.
    pub fn recording(&self) -> &super::recording_menu::RecordingScreens {
        &self.recording
    }

    /// The host loads a catalog here, and tests move the library cursor.
    pub fn recording_mut(&mut self) -> &mut super::recording_menu::RecordingScreens {
        &mut self.recording
    }

    /// A chord request. `RecordingPopup` toggles the pop-up. Other requests are not this screen's.
    pub fn on_request(&mut self, request: super::chords::Request) {
        if request == super::chords::Request::RecordingPopup {
            self.recording.toggle_popup();
        }
    }

    pub fn new(settings: Settings, env: Env) -> Menu {
        let mut menu = Menu {
            settings,
            env,
            page: Page::Cpu,
            cursor: [0; 6],
            expanded: 0,
            stack: Vec::new(),
            outbox: Outbox::default(),
            closed: false,
            chord_capture: ChordCapture::Preview,
            chord_refusal: None,
            edits: Vec::new(),
            page_reset: None,
            recording: super::recording_menu::RecordingScreens::new(),
        };
        menu.sync();
        menu
    }

    /// Open again on freshly committed settings (the fork's `open_settings_for_player`): the stack and
    /// the pending requests are dropped, the page is the one last used, cursors stay.
    pub fn reopen(&mut self, settings: Settings, env: Env) {
        self.settings = settings;
        self.env = env;
        self.stack.clear();
        self.outbox = Outbox::default();
        self.closed = false;
        self.chord_capture = ChordCapture::Preview;
        self.chord_refusal = None;
        self.edits.clear();
        self.sync();
    }

    /// Where the menu is: each page's focused entry, the open advanced groups and the sub-screen open on the page
    /// (the bottom of the stack, when it can be resumed). The runtime keeps it outside the model, which it drops at
    /// every close, so the next opening comes back here (`restore_place`).
    pub fn place(&self) -> Place {
        let screen = self.stack.first().copied().and_then(resumable);
        Place { page: self.page, cursor: self.cursor, expanded: self.expanded, screen: screen.map(|s| (self.page, s)) }
    }

    /// Come back to `place` after an opening: its page when that page is still in the ring (else the page `last_page`
    /// gave, Q5). Every cursor is clamped to its page as the table and the install stand now, so a stale index never
    /// points past the end; the sub-screen is reopened only on the same page, on the row that opens it, when that row
    /// is still visible and not greyed and the screen's own cursor is still inside it. Otherwise the parent row stays
    /// focused.
    pub fn restore_place(&mut self, place: Place) {
        self.cursor = place.cursor;
        self.expanded = place.expanded;
        if self.page_in_ring(place.page) {
            self.page = place.page;
        }
        let page = self.page;
        for other in Page::ALL {
            self.page = other;
            self.clamp_cursor();
        }
        self.page = page;
        self.stack.clear();
        if let Some((saved, screen)) = place.screen {
            if saved == page && self.resumes(screen) {
                self.stack.push(screen);
            }
        }
    }

    /// Whether `screen` still belongs to the focused row and its cursor is inside it.
    fn resumes(&self, screen: Screen) -> bool {
        let Some(focused) = self.focused_row() else { return false };
        if self.grey(focused).is_some() {
            return false;
        }
        let control = ROWS[focused].control;
        let on_focused = |row: usize| row == focused;
        match screen {
            Screen::List { row, cursor } => {
                on_focused(row)
                    && matches!(control, Control::Dropdown(_) | Control::YesNoRandom | Control::Number { .. })
                    && cursor < self.option_count(row)
            }
            Screen::Checklist { row, cursor } => {
                on_focused(row) && matches!(control, Control::MultiSelect(_)) && cursor < self.option_count(row)
            }
            Screen::Delay { row, cursor, .. } => {
                on_focused(row)
                    && matches!(control, Control::MultiSelectRange { .. })
                    && cursor < self.get(ROWS[row].id).len() + 2
            }
            Screen::Slider { row, .. } => on_focused(row) && matches!(control, Control::Slider),
            Screen::Directions { row, chip } => {
                on_focused(row) && matches!(control, Control::DirectionGrid { .. }) && chip < CHIP_POS.len()
            }
            Screen::SwapGroups { cursor } => matches!(control, Control::SwapGroups) && cursor < SWAP_GROUP_IDS.len(),
            Screen::Overrides { situation, cursor } => {
                matches!(control, Control::Editor(Editor::SituationOverrides))
                    && situation < SITUATIONS.len()
                    && cursor <= options::ACTIONS.len()
            }
            Screen::Chords { cursor } => {
                matches!(control, Control::Editor(Editor::ButtonChords)) && cursor < CHORD_ROWS.len()
            }
            Screen::ChordEntry { .. } | Screen::Grid { .. } | Screen::External(_) | Screen::ConfirmReset => false,
        }
    }

    fn sync(&mut self) {
        let saved = self.settings.last_page().and_then(Page::from_id).filter(|page| self.page_in_ring(*page));
        self.page = saved.unwrap_or(Page::Cpu);
        self.normalize_delay_rows();
        self.clamp_cursor();
    }

    /// A delay row with a checklist and a range at once (a hand-edited file) keeps the checklist.
    fn normalize_delay_rows(&mut self) {
        for row in &ROWS {
            if let Control::MultiSelectRange { range_id, .. } = row.control {
                let ticked = self.get(row.id).iter().any(|v| *v != 0);
                if ticked && !self.get(range_id).is_empty() {
                    self.put(range_id, &[]);
                }
            }
        }
        // Loading is not a change the player made.
        self.outbox = Outbox::default();
        self.edits.clear();
    }

    // ---- pages and entries -------------------------------------------------------------------

    fn page_in_ring(&self, page: Page) -> bool {
        page != Page::Developer || self.env.developer_page
    }

    /// The pages in the ring now: five, or six with the Developer page enabled.
    pub fn ring(&self) -> impl Iterator<Item = Page> + '_ {
        Page::ALL.into_iter().filter(|page| self.page_in_ring(*page))
    }

    pub fn page(&self) -> Page {
        self.page
    }

    /// The Recording page group under the cursor, for the pop-up's sibling diag line.
    pub fn recording_page_group(&self) -> u8 {
        self.focus().map(group_of).unwrap_or(usize::from(table::SAVES_MACROS)) as u8
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    fn group_open(&self, group: usize) -> bool {
        !GROUPS[group].advanced || self.expanded & (1 << group) != 0
    }

    fn group_visible(&self, group: usize) -> bool {
        if group == usize::from(table::COMBOS_SEQ) && self.recording.page_role() == super::recording_menu::ViewRole::Cpu {
            return false;
        }
        if group == usize::from(table::SAVES_MACROS) || group == usize::from(table::COMBOS_SEQ) {
            return true;
        }
        ROWS.iter().any(|row| usize::from(row.group) == group && row.visible(self.env.pilot_present))
    }

    /// Visit the entries of `page` in order: a header for each group with a visible row, then its
    /// rows unless the group is collapsed; last, the page's *Reset page to defaults* line. `visit` returns `false`
    /// to stop.
    fn scan(&self, page: Page, mut visit: impl FnMut(usize, Entry) -> bool) {
        let mut n = 0;
        let mut last = None;
        for (g, group) in GROUPS.iter().enumerate() {
            if group.page != page || !self.group_visible(g) {
                continue;
            }
            last = Some(g);
            if !visit(n, Entry::Group(g)) {
                return;
            }
            n += 1;
            if !self.group_open(g) {
                continue;
            }
            for (r, row) in ROWS.iter().enumerate() {
                if usize::from(row.group) == g && row.visible(self.env.pilot_present) {
                    if !visit(n, Entry::Row(r)) {
                        return;
                    }
                    n += 1;
                }
            }
        }
        if let Some(g) = last {
            visit(n, Entry::ResetPage(g));
        }
    }

    /// Entries on `page` (headers included).
    pub fn entry_count(&self, page: Page) -> usize {
        let mut count = 0;
        self.scan(page, |n, _| {
            count = n + 1;
            true
        });
        count
    }

    pub fn entry(&self, page: Page, index: usize) -> Option<Entry> {
        let mut found = None;
        self.scan(page, |n, entry| {
            if n == index {
                found = Some(entry);
                false
            } else {
                true
            }
        });
        found
    }

    /// A header can be focused only when it opens or closes an advanced group.
    fn focusable(&self, entry: Entry) -> bool {
        match entry {
            Entry::Row(_) | Entry::ResetPage(_) => true,
            Entry::Group(g) => GROUPS[g].advanced || g == usize::from(table::SAVES_MACROS) || g == usize::from(table::COMBOS_SEQ),
        }
    }

    fn clamp_cursor(&mut self) {
        let page = self.page;
        let count = self.entry_count(page);
        let mut at = self.cursor[page.index()].min(count.saturating_sub(1));
        for _ in 0..count {
            if self.entry(page, at).is_some_and(|e| self.focusable(e)) {
                break;
            }
            at = (at + 1) % count;
        }
        self.cursor[page.index()] = at;
    }

    /// The focused entry of the current page.
    pub fn focus(&self) -> Option<Entry> {
        self.entry(self.page, self.cursor[self.page.index()])
    }

    /// The focused entry's index in the page's list.
    pub fn cursor(&self) -> usize {
        self.cursor[self.page.index()]
    }

    /// The focused row, if the focus is on a row.
    pub fn focused_row(&self) -> Option<usize> {
        match self.focus() {
            Some(Entry::Row(r)) => Some(r),
            _ => None,
        }
    }

    pub fn group_is_open(&self, group: usize) -> bool {
        self.group_open(group)
    }

    /// The Developer rows are inert while the page is hidden: a consumer asks for the effective
    /// value, `stored AND enabled` (SPEC-core-02 §2.8).
    #[cfg(test)]
    pub fn developer_effective(&self, id: &str) -> bool {
        self.env.developer_page && self.toggle_by_id(id)
    }

    // ---- reading values ----------------------------------------------------------------------

    fn get(&self, id: &str) -> &[u8] {
        self.settings.menu.get(id).unwrap_or(&[])
    }

    fn toggle_by_id(&self, id: &str) -> bool {
        self.get(id).first().is_some_and(|v| *v != 0)
    }

    /// The settings as they stand (what the commit publishes and writes).
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// The commit: record the page last used (Q5) and hand the settings out.
    pub fn commit(&mut self) -> &Settings {
        self.settings.set_last_page(self.page.id());
        &self.settings
    }

    /// The close commit (decision MENU-CLOSE, `review/menu-proposal` `9a9cc47`): the rows, kept keys
    /// and top-level keys the player changed this session keep the model's values — the player's
    /// edit wins a same-row conflict — and every other row keeps its live value in `live`, the
    /// write-backs and publishes adopted while the menu was open. `commit` stays the model's own
    /// snapshot; the close path commits onto the live settings.
    pub fn commit_onto(&mut self, live: &Settings) -> Settings {
        let mut merged = live.clone();
        for id in &self.edits {
            if let Some(elements) = self.settings.menu.get(id) {
                let elements = elements.to_vec();
                let _ = merged.menu.set(id, &elements);
            }
            if let Some(value) = self.settings.menu.extra().get(id).cloned() {
                let _ = merged.menu.set_kept(id, value);
            }
            if let Some(value) = self.settings.extra.get(id).cloned() {
                merged.extra.insert(id.clone(), value);
            }
        }
        // The saved defaults are the player's (`save_defaults`); adoption never touches them.
        merged.defaults = self.settings.defaults.clone();
        merged.set_last_page(self.page.id());
        merged
    }

    /// A row, kept key or top-level key the player changed this session (MENU-CLOSE).
    fn note_edit(&mut self, id: &str) {
        if !self.edits.iter().any(|edit| edit == id) {
            self.edits.push(id.to_owned());
        }
    }

    /// The JSON the commit writes (`{"menu": …, "defaults_menu": …}` and the other top-level keys).
    #[cfg(test)]
    pub fn serialized(&mut self) -> String {
        self.commit().to_json()
    }

    /// The value of a row for the renderer.
    pub fn value(&self, row: usize) -> RowValue {
        let row = &ROWS[row];
        let array = self.get(row.id);
        match row.control {
            Control::Toggle => RowValue::Toggle(array.first().is_some_and(|v| *v != 0)),
            Control::ReplayCapture => RowValue::Toggle(self.env.replay_capture),
            Control::Dropdown(_) | Control::Number { .. } => {
                let at = array.iter().position(|v| *v != 0);
                match row.control {
                    Control::Number { first, step } => RowValue::Number(at.map(|i| first + step * i as u16)),
                    _ => RowValue::Choice(at),
                }
            }
            Control::YesNoRandom => RowValue::Choice(yes_no_random(array)),
            Control::MultiSelect(labels) => {
                RowValue::Checklist { ticked: array.iter().filter(|v| **v != 0).count(), total: labels.len() }
            }
            Control::MultiSelectRange { first, step, range_id } => {
                RowValue::Delay(match self.range_of(range_id, first, step) {
                    Some(range) => DelayView::Range(range),
                    None => match array.iter().filter(|v| **v != 0).count() {
                        0 => DelayView::Empty,
                        n => DelayView::Ticked(n),
                    },
                })
            }
            Control::Slider => RowValue::Slider { lower: array.first().copied().unwrap_or(0), upper: array.get(1).copied().unwrap_or(0) },
            Control::DirectionGrid { .. } => {
                RowValue::Chips(array.iter().enumerate().fold(0u16, |mask, (i, v)| mask | (u16::from(*v != 0) << i)))
            }
            Control::SwapGroups => RowValue::Checklist {
                ticked: SWAP_GROUP_IDS.iter().filter(|id| self.toggle_by_id(id)).count(),
                total: SWAP_GROUP_IDS.len(),
            },
            Control::Editor(Editor::SituationOverrides) => RowValue::OverridesSet(
                SITUATIONS.iter().filter(|(_, id)| self.get(id).iter().any(|v| *v != 0)).count(),
            ),
            Control::Editor(Editor::ShortcutSlot(slot)) => RowValue::Chord(self.env.shortcuts[usize::from(slot & 1)]),
            Control::Editor(_) | Control::Info(_) => RowValue::None,
        }
    }

    fn range_of(&self, range_id: &str, first: u16, step: u16) -> Option<DelayRange> {
        match self.get(range_id) {
            [lo, hi] => Some(DelayRange { min: first + step * u16::from(*lo), max: first + step * u16::from(*hi), step }),
            _ => None,
        }
    }

    /// What a consumer reads from the delay row called `id` (SPEC-core-02 §2.8).
    pub fn delay_spec(&self, id: &str) -> Option<DelaySpec<'_>> {
        let row = ROWS.iter().find(|row| row.id == id)?;
        let Control::MultiSelectRange { first, step, range_id } = row.control else {
            return None;
        };
        if let Some(range) = self.range_of(range_id, first, step) {
            return Some(DelaySpec::Range(range));
        }
        let weights = self.get(id);
        if weights.iter().any(|v| *v != 0) {
            Some(DelaySpec::Set { weights, first, step })
        } else {
            Some(DelaySpec::Empty)
        }
    }

    /// The number of options of a list-like row.
    pub fn option_count(&self, row: usize) -> usize {
        match ROWS[row].control {
            Control::Dropdown(labels) | Control::MultiSelect(labels) => labels.len(),
            Control::YesNoRandom => 3,
            Control::Number { .. } => self.get(ROWS[row].id).len(),
            Control::MultiSelectRange { .. } => self.get(ROWS[row].id).len(),
            _ => 0,
        }
    }

    /// Why the row is greyed, if it is.
    pub fn grey(&self, row: usize) -> Option<Grey> {
        match ROWS[row].id {
            "respawn_invincibility" if self.toggle_by_id("save_state_autoload") => Some(Grey::AutoLoadOn),
            "save_damage_limits_player" if self.get("save_damage_player").get(2).copied().unwrap_or(0) == 0 => {
                Some(Grey::DamageNotRandom)
            }
            "save_damage_limits_cpu" if self.get("save_damage_cpu").get(2).copied().unwrap_or(0) == 0 => {
                Some(Grey::DamageNotRandom)
            }
            _ => None,
        }
    }

    /// Why a swap-group entry is greyed. Group Swap is the master switch of the group entries; the
    /// pair groups also need the pair bank; the pair bank needs three bodies (MENU-DRAFT §1 notes).
    pub fn swap_entry_grey(&self, entry: usize) -> Option<Grey> {
        let on = |id: &str| self.toggle_by_id(id);
        match entry {
            0 if self.get("cycle_preload_bank_members").get(2).copied().unwrap_or(0) == 0 => Some(Grey::NeedsThreeBodies),
            2..=6 if !on("cycle_preload_group_swap") => Some(Grey::GroupSwapOff),
            3 | 5 if !on("cycle_preload_ic_pair_bank") => Some(Grey::PairBankOff),
            _ => None,
        }
    }

    /// For a delay popup: the checklist is greyed while a range is set, the range while any value is
    /// ticked. Returns `(checklist, range)`.
    pub fn delay_grey(&self, row: usize) -> (Option<Grey>, Option<Grey>) {
        let Control::MultiSelectRange { range_id, .. } = ROWS[row].control else {
            return (None, None);
        };
        let ticked = self.get(ROWS[row].id).iter().any(|v| *v != 0);
        let ranged = !self.get(range_id).is_empty();
        (ranged.then_some(Grey::RangeSet), ticked.then_some(Grey::ChecklistSet))
    }

    // ---- writing values ----------------------------------------------------------------------

    /// Replace a row; notes the change, and the re-rolls it causes (Q18 A, B11).
    fn put(&mut self, id: &str, elements: &[u8]) {
        let unchanged = self.get(id) == elements;
        if unchanged || self.settings.menu.set(id, elements).is_err() {
            return;
        }
        self.outbox.changed = true;
        self.note_edit(id);
        for (reroll_id, bit) in REROLL_IDS {
            if reroll_id == id {
                self.outbox.rerolls |= bit;
            }
        }
    }

    fn put_one_hot(&mut self, id: &str, index: usize) {
        let mut row = vec![0u8; self.get(id).len()];
        if let Some(slot) = row.get_mut(index) {
            *slot = 1;
        }
        self.put(id, &row);
    }

    /// Take what the menu asked the runtime to do.
    pub fn take_outbox(&mut self) -> Outbox {
        core::mem::take(&mut self.outbox)
    }

    #[cfg(test)]
    pub fn outbox(&self) -> &Outbox {
        &self.outbox
    }

    /// The player's saved defaults, replaced by `save_defaults` (the current values are copied).
    pub fn save_defaults(&mut self) {
        if self.settings.defaults != self.settings.menu {
            self.settings.defaults = self.settings.menu.clone();
            self.outbox.changed = true;
        }
    }

    /// Bulk update from a map of id to array (SPEC-core-02 §5, the fork's `update_from_vec` rule):
    /// each id present replaces its row wholly, so a shorter array leaves the appended options off;
    /// an id missing is left as it is; an id that is not a row is ignored. A delay row's checklist
    /// and range are one row: updating either clears the other first.
    pub fn bulk_update(&mut self, entries: &[(&str, &[u8])]) {
        for (id, array) in entries {
            let Some((_, def)) = settings::def(id) else { continue };
            // The array is read like a file's row: padded with zeros or cut to the row's length, so a
            // shorter one leaves the appended options off; a bad one (a slider without two elements)
            // is refused and the row stays as it was.
            let value = serde_json::Value::Array(array.iter().map(|b| serde_json::Value::from(*b)).collect());
            let Ok(parsed) = settings::parse_row(def, &value) else { continue };
            if def.kind == settings::Kind::Range {
                // The range arrives: the checklist of its row is cleared first.
                let row = ROWS.iter().find(|row| matches!(row.control, Control::MultiSelectRange { range_id, .. } if range_id == *id));
                if let Some(row) = row {
                    let zeros = vec![0u8; self.get(row.id).len()];
                    self.put(row.id, &zeros);
                }
            } else if let Some(Control::MultiSelectRange { range_id, .. }) =
                ROWS.iter().find(|row| row.id == *id).map(|row| row.control)
            {
                self.put(range_id, &[]);
            }
            self.put(id, &parsed);
        }
    }

    // ---- the sub-screen stack ----------------------------------------------------------------

    /// The screen on top of the stack, `None` when the page itself is showing.
    pub fn screen(&self) -> Option<&Screen> {
        self.stack.last()
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// The console's operation mode changed (docked or undocked). A grid already open keeps the
    /// layout it opened with; the next one opened takes the new mode (SPEC-core-03 §2.5).
    /// The combo helper's target and draft names changed while the menu is open (SPEC-input-07 §6).
    pub fn set_combo_labels(&mut self, target: super::combo::Label, draft: super::combo::Label) {
        self.env.combo_target = target;
        self.env.combo_draft = draft;
    }

    pub fn set_console(&mut self, mode: ConsoleMode) {
        self.env.console = mode;
    }

    /// Whether the footer offers *Clear* (Y) on the screen showing: the delay popup, a direction
    /// grid and the chord editor clear their value; a plain checklist has no clear.
    #[cfg(test)]
    pub fn clear_hint(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(Screen::Delay { .. } | Screen::Directions { .. } | Screen::ChordEntry { .. })
        )
    }

    /// The menu was closed (B on the page); the runtime commits.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// The row whose help text the two-line panel shows: the focused row of the page, or the row a
    /// popup or grid belongs to.
    pub fn help_row(&self) -> Option<&'static Row> {
        let row = match self.stack.last() {
            None => self.focused_row()?,
            Some(Screen::List { row, .. })
            | Some(Screen::Checklist { row, .. })
            | Some(Screen::Delay { row, .. })
            | Some(Screen::Slider { row, .. })
            | Some(Screen::Directions { row, .. }) => *row,
            Some(_) => return None,
        };
        Some(&ROWS[row])
    }

    // ---- input -------------------------------------------------------------------------------

    /// Whether what is showing takes ZL + Up / Down as jumps: a page (the section jump) and the popup lists (a window).
    /// Everywhere else (the slider, the direction grid, the roster and stage grids, the external panels, a handle being
    /// edited, the reset confirmation) ZL is its own button and Up / Down move as usual; the input adapter asks this
    /// (through the worker) before it turns a direction into a jump (review MN-1).
    pub fn takes_jumps(&self) -> bool {
        match self.stack.last() {
            None => true,
            Some(Screen::List { .. } | Screen::Checklist { .. } | Screen::SwapGroups { .. } | Screen::Overrides { .. } | Screen::Chords { .. }) => true,
            Some(Screen::Delay { editing, .. }) => !editing,
            Some(_) => false,
        }
    }

    /// One logical button press (an accepted press edge, or a stick repeat).
    pub fn press(&mut self, button: Button) {
        if matches!(self.screen(), Some(Screen::ChordEntry { .. })) && self.chord_capture != ChordCapture::Preview {
            return;
        }
        // A jump that reaches a screen without jumps (the screen changed under a held ZL) is the plain direction.
        let button = match button {
            Button::JumpUp if !self.takes_jumps() => Button::Up,
            Button::JumpDown if !self.takes_jumps() => Button::Down,
            other => other,
        };
        if self.recording.popup_open() {
            self.recording.press_popup(button);
            return;
        }
        match self.stack.last().copied() {
            None => self.press_page(button),
            Some(Screen::List { row, cursor }) => self.press_list(button, row, cursor),
            Some(Screen::Checklist { row, cursor }) => self.press_checklist(button, row, cursor),
            Some(Screen::Delay { row, cursor, editing }) => self.press_delay(button, row, cursor, editing),
            Some(Screen::Slider { row, state }) => self.press_slider(button, row, state),
            Some(Screen::Directions { row, chip }) => self.press_directions(button, row, chip),
            Some(Screen::SwapGroups { cursor }) => self.press_swap_groups(button, cursor),
            Some(Screen::Overrides { situation, cursor }) => self.press_overrides(button, situation, cursor),
            Some(Screen::Chords { cursor }) => self.press_chords(button, cursor),
            Some(Screen::ChordEntry { target, captured }) => self.press_chord_entry(button, target, captured),
            Some(Screen::Grid { layout, page, cell }) => self.press_grid(button, layout, page, cell),
            Some(Screen::External(_)) => {
                if button == Button::B {
                    self.stack.pop();
                }
            }
            Some(Screen::ConfirmReset) => match button {
                Button::A => {
                    self.stack.pop();
                    self.reset_page();
                }
                Button::B => {
                    self.stack.pop();
                }
                _ => {}
            },
        }
    }

    fn top(&mut self, screen: Screen) {
        if let Some(top) = self.stack.last_mut() {
            *top = screen;
        }
    }

    fn move_cursor(&mut self, forward: bool) {
        let page = self.page;
        let count = self.entry_count(page);
        if count == 0 {
            return;
        }
        let mut at = self.cursor[page.index()];
        for _ in 0..count {
            at = if forward { (at + 1) % count } else { (at + count - 1) % count };
            if self.entry(page, at).is_some_and(|e| self.focusable(e)) {
                break;
            }
        }
        self.cursor[page.index()] = at;
    }

    fn change_page(&mut self, forward: bool) {
        let ring: Vec<Page> = self.ring().collect();
        let Some(at) = ring.iter().position(|p| *p == self.page) else { return };
        let next = if forward { (at + 1) % ring.len() } else { (at + ring.len() - 1) % ring.len() };
        self.page = ring[next];
        self.clamp_cursor();
    }

    fn press_page(&mut self, button: Button) {
        if self.page == Page::Recording {
            let group = self.focus().map(group_of);
            let on_library = group.is_some_and(|g| g == usize::from(table::SAVES_MACROS) || g == usize::from(table::COMBOS_SEQ));
            let which = group.unwrap_or(usize::from(table::SAVES_MACROS)) as u8;
            match self.recording.handle_page(button, on_library, which) {
                super::recording_menu::PageHit::Consumed => {
                    self.clamp_cursor();
                    return;
                }
                super::recording_menu::PageHit::LeaveUp => {
                    self.move_cursor(false);
                    return;
                }
                super::recording_menu::PageHit::LeaveDown => {
                    self.move_cursor(true);
                    return;
                }
                super::recording_menu::PageHit::Passthrough => {}
            }
        }
        match button {
            Button::Up => self.move_cursor(false),
            Button::Down => self.move_cursor(true),
            Button::L => self.change_page(false),
            Button::R => self.change_page(true),
            Button::B => self.closed = true,
            Button::X => self.save_defaults(),
            Button::Y => {
                if let Some(row) = self.focused_row().and_then(|index| ROWS.get(index)) { self.reset_setting(row); }
            }
            Button::A => match self.focus() {
                Some(Entry::Group(g)) => {
                    self.expanded ^= 1 << g;
                    self.clamp_cursor();
                }
                Some(Entry::Row(row)) => self.activate(row),
                Some(Entry::ResetPage(_)) => self.stack.push(Screen::ConfirmReset),
                None => {}
            },
            Button::Left | Button::Right => {
                if let Some(row) = self.focused_row() {
                    self.cycle(row, button == Button::Right);
                }
            }
            Button::JumpUp => self.jump_section(false),
            Button::JumpDown => self.jump_section(true),
            // ZR no longer resets the page (it is the role switch inside the roster grids, easy to hit by
            // accident): the page's last line does, after a confirmation.
            Button::ZL | Button::ZR => {}
        }
    }

    /// ZL + Up / Down on a page: the cursor goes to the first focusable entry of the previous or next section (a
    /// group: its first row, or the header of a collapsed advanced group). It does not wrap: on the first or the
    /// last section the cursor stays, unlike Up / Down, so the edge of the page is felt.
    fn jump_section(&mut self, forward: bool) {
        let page = self.page;
        let Some(current) = self.focus().map(group_of) else { return };
        let (mut before, mut after, mut seen, mut last) = (None, None, false, None);
        self.scan(page, |n, entry| {
            let group = group_of(entry);
            if !self.focusable(entry) || last == Some(group) {
                return true;
            }
            // `n` is the first focusable entry of `group`.
            last = Some(group);
            if group == current {
                seen = true;
            } else if !seen {
                before = Some(n);
            } else {
                after = Some(n);
                return false;
            }
            true
        });
        if let Some(at) = if forward { after } else { before } {
            self.cursor[page.index()] = at;
        }
    }

    /// A on a row of the page.
    fn activate(&mut self, row: usize) {
        if self.grey(row).is_some() {
            return;
        }
        let r = &ROWS[row];
        match r.control {
            Control::Toggle => {
                let on = self.toggle_by_id(r.id);
                self.put(r.id, if on { &[0, 1] } else { &[1, 0] });
            }
            Control::ReplayCapture => {
                self.env.replay_capture = !self.env.replay_capture;
                self.outbox.replay_capture = Some(self.env.replay_capture);
            }
            Control::Dropdown(_) | Control::YesNoRandom | Control::Number { .. } => {
                let cursor = match r.control {
                    Control::YesNoRandom => yes_no_random(self.get(r.id)),
                    _ => self.get(r.id).iter().position(|v| *v != 0),
                }
                .unwrap_or(0);
                self.stack.push(Screen::List { row, cursor });
            }
            Control::MultiSelect(_) => self.stack.push(Screen::Checklist { row, cursor: 0 }),
            Control::MultiSelectRange { .. } => self.stack.push(Screen::Delay { row, cursor: 0, editing: false }),
            Control::Slider => self.stack.push(Screen::Slider { row, state: SliderState::LowerHover }),
            Control::DirectionGrid { .. } => self.stack.push(Screen::Directions { row, chip: DIRECTION_START }),
            Control::SwapGroups => self.stack.push(Screen::SwapGroups { cursor: 0 }),
            Control::Editor(editor) => self.open_editor(editor),
            Control::Info(_) => {}
        }
    }

    fn open_editor(&mut self, editor: Editor) {
        let screen = match editor {
            Editor::SituationOverrides => Screen::Overrides { situation: 0, cursor: 0 },
            Editor::ButtonChords => Screen::Chords { cursor: 0 },
            Editor::ShortcutSlot(slot) => {
                let slot = slot & 1;
                Screen::ChordEntry { target: ChordTarget::Shortcut(slot), captured: self.env.shortcuts[usize::from(slot)] }
            }
            // The console mode is read now and the layout stays until the grid closes.
            Editor::GiveItem => Screen::Grid { layout: GridLayout::new(GridKind::Items, self.env.console), page: 0, cell: 0 },
            Editor::StageGrid => Screen::Grid { layout: GridLayout::new(GridKind::Stages, self.env.console), page: 0, cell: 0 },
            Editor::ChangeStage => Screen::Grid { layout: GridLayout::new(GridKind::StageChange, self.env.console), page: 0, cell: 0 },
            Editor::PortraitPlayer => self.roster_screen(RosterUse::PlayerPick),
            Editor::PortraitCpu => self.roster_screen(RosterUse::CpuPick),
            Editor::CharacterCycles => self.roster_screen(RosterUse::Cycles),
            Editor::KirbyHats => self.roster_screen(RosterUse::Hats),
            other => Screen::External(other),
        };
        self.stack.push(screen);
    }

    /// Left / Right on a row of the page: a dropdown cycles in place and wraps (Q2); a number steps
    /// and stops at its ends.
    fn cycle(&mut self, row: usize, forward: bool) {
        if self.grey(row).is_some() {
            return;
        }
        let r = &ROWS[row];
        if r.id == "playback_loop" {
            self.recording.cycle_loop_row(forward);
            return;
        }
        match r.control {
            Control::Dropdown(labels) => {
                let len = labels.len();
                let at = self.get(r.id).iter().position(|v| *v != 0);
                let next = match at {
                    None => 0,
                    Some(i) if forward => (i + 1) % len,
                    Some(i) => (i + len - 1) % len,
                };
                self.put_one_hot(r.id, next);
            }
            Control::YesNoRandom => {
                let next = match yes_no_random(self.get(r.id)) {
                    None => 0,
                    Some(i) if forward => (i + 1) % 3,
                    Some(i) => (i + 2) % 3,
                };
                self.put_yes_no_random(r.id, next);
            }
            Control::Number { .. } => {
                let len = self.get(r.id).len();
                let at = self.get(r.id).iter().position(|v| *v != 0).unwrap_or(0);
                let next = if forward { (at + 1).min(len - 1) } else { at.saturating_sub(1) };
                self.put_one_hot(r.id, next);
            }
            _ => {}
        }
    }

    fn put_yes_no_random(&mut self, id: &str, choice: usize) {
        match choice {
            0 => self.put(id, &[1, 0]),
            1 => self.put(id, &[0, 1]),
            _ => self.put(id, &[1, 1]),
        }
    }

    /// The confirmed *Reset page to defaults* restores the page's saved defaults and notes which rows changed
    /// (`take_page_reset`). INI-held shortcuts and replay capture are not in the settings snapshots and stay unchanged.
    fn reset_page(&mut self) {
        let mut changed = Vec::new();
        for row in &ROWS {
            if row.page() == self.page && row.visible(self.env.pilot_present) && self.reset_setting(row) {
                changed.push(if row.id.is_empty() { row.title } else { row.id });
            }
        }
        self.page_reset = Some((self.page, changed));
    }

    /// The page reset that ran since the last take: the page and the rows it changed (ids, or titles for rows
    /// without one).
    pub fn take_page_reset(&mut self) -> Option<(Page, Vec<&'static str>)> {
        self.page_reset.take()
    }

    /// One visible row can own several stored keys (range/checklist, groups,
    /// overrides, chords); a focused-row reset restores that complete value. True when a value changed.
    fn reset_setting(&mut self, row: &Row) -> bool {
        let mut changed = false;
        if matches!(row.control, Control::SwapGroups) {
            for id in SWAP_GROUP_IDS { changed |= self.reset_row(id); }
        }
        if matches!(row.control, Control::Editor(Editor::SituationOverrides)) {
            for (_, id) in SITUATIONS { changed |= self.reset_row(id); }
        }
        if matches!(row.control, Control::Editor(Editor::ButtonChords)) {
            for (_, id) in CHORD_ROWS { changed |= self.reset_row(id); }
        }
        // One row, two kept keys. The row id is the Player key; the CPU key is the sibling.
        if matches!(row.control, Control::Editor(Editor::CharacterCycles)) {
            changed |= self.reset_row("cpu_roster");
        }
        if matches!(row.control, Control::Editor(Editor::KirbyHats)) {
            changed |= self.reset_row("cpu_kirby_hats");
        }
        changed |= self.reset_row(row.id);
        if let Control::MultiSelectRange { range_id, .. } = row.control {
            changed |= self.reset_row(range_id);
        }
        changed
    }

    fn reset_row(&mut self, id: &str) -> bool {
        if id.is_empty() {
            return false;
        }
        if let Some(saved) = self.settings.defaults.get(id).map(<[u8]>::to_vec) {
            let changed = saved != self.get(id);
            self.put(id, &saved);
            return changed;
        }
        // A kept key is not a DEFS row, so `Snapshot::get` never sees it. The defaults snapshot
        // holds the player's saved value when they stored one (`save_defaults` copies `extra`).
        // Otherwise the factory is what a missing key already means.
        self.reset_kept(id)
    }

    // Choice counts: 89 (86 fighters and forms, 3 Mii classes). Lobby stages: STAGES.len() == 117.
    const CYCLE_CHOICES: usize = 89;
    const LOBBY_STAGES: usize = 117;

    /// Factory JSON of a kept key. A missing key already means this. `None` when this menu does not own it.
    fn kept_page_default(id: &str) -> Option<serde_json::Value> {
        let values = match id {
            "player_roster" | "cpu_roster" => Self::bytes(Self::CYCLE_CHOICES + 1, &[(0, 1)]),
            "player_kirby_hats" | "cpu_kirby_hats" => Self::bytes(Self::CYCLE_CHOICES + 1, &[(1, 1)]),
            "player_character" | "cpu_character" => Self::bytes(Self::CYCLE_CHOICES, &[(0, 1)]),
            "lobby_stages" => Self::bytes(Self::LOBBY_STAGES + 1, &[(1, 1)]),
            _ => return None,
        };
        Some(serde_json::Value::Array(values))
    }

    fn bytes(len: usize, set: &[(usize, u8)]) -> Vec<serde_json::Value> {
        let mut values = vec![serde_json::Value::from(0u8); len];
        for (index, byte) in set {
            values[*index] = serde_json::Value::from(*byte);
        }
        values
    }

    fn reset_kept(&mut self, id: &str) -> bool {
        let saved = self.settings.defaults.extra().get(id).cloned();
        let target = saved.clone().or_else(|| Self::kept_page_default(id));
        let Some(target) = target else { return false };
        // Absence is already the factory. Writing it would mark a reset that changed nothing.
        if self.kept(id).is_none() && saved.is_none() {
            return false;
        }
        if self.kept(id) == Some(&target) {
            return false;
        }
        self.set_kept(id, target);
        true
    }

    fn press_list(&mut self, button: Button, row: usize, cursor: usize) {
        let len = self.option_count(row);
        match button {
            Button::Up => self.top(Screen::List { row, cursor: (cursor + len - 1) % len }),
            Button::Down => self.top(Screen::List { row, cursor: (cursor + 1) % len }),
            Button::JumpUp | Button::JumpDown => {
                self.top(Screen::List { row, cursor: jumped(cursor, len, button == Button::JumpDown) });
            }
            Button::A => {
                match ROWS[row].control {
                    Control::YesNoRandom => self.put_yes_no_random(ROWS[row].id, cursor),
                    _ => self.put_one_hot(ROWS[row].id, cursor),
                }
                self.stack.pop();
            }
            Button::B => {
                self.stack.pop();
            }
            _ => {}
        }
    }

    /// A tick: set to 1 when off; clear when on (an older weight is cleared too).
    fn toggle_entry(&mut self, id: &str, index: usize) {
        let mut array = self.get(id).to_vec();
        let Some(slot) = array.get_mut(index) else { return };
        *slot = u8::from(*slot == 0);
        self.put(id, &array);
    }

    /// ZL: tick all, or clear all when every entry is already ticked.
    fn all_or_none(&mut self, id: &str) {
        let mut array = self.get(id).to_vec();
        let all = array.iter().all(|v| *v != 0);
        for slot in &mut array {
            *slot = if all { 0 } else { (*slot).max(1) };
        }
        self.put(id, &array);
    }

    fn press_checklist(&mut self, button: Button, row: usize, cursor: usize) {
        let len = self.option_count(row);
        let id = ROWS[row].id;
        match button {
            Button::Up => self.top(Screen::Checklist { row, cursor: (cursor + len - 1) % len }),
            Button::Down => self.top(Screen::Checklist { row, cursor: (cursor + 1) % len }),
            Button::JumpUp | Button::JumpDown => {
                self.top(Screen::Checklist { row, cursor: jumped(cursor, len, button == Button::JumpDown) });
            }
            Button::A => self.toggle_entry(id, cursor),
            Button::ZL => self.all_or_none(id),
            Button::B => {
                self.stack.pop();
            }
            _ => {}
        }
    }

    fn press_delay(&mut self, button: Button, row: usize, cursor: usize, editing: bool) {
        let Control::MultiSelectRange { range_id, .. } = ROWS[row].control else { return };
        let id = ROWS[row].id;
        let len = self.get(id).len();
        let (checklist_grey, range_grey) = self.delay_grey(row);
        let on_handle = cursor >= len;
        let screen = |cursor: usize, editing: bool| Screen::Delay { row, cursor, editing };
        match button {
            Button::Up | Button::Down => {
                let forward = button == Button::Down;
                if editing {
                    let amount = RANGE_FAST;
                    self.move_handle(range_id, cursor - len, if forward { -(amount as isize) } else { amount as isize });
                } else {
                    let count = len + 2;
                    let next = if forward { (cursor + 1) % count } else { (cursor + count - 1) % count };
                    self.top(screen(next, false));
                }
            }
            // The checklist entries and the two handles as one list; a handle being edited ignores the jump.
            Button::JumpUp | Button::JumpDown if !editing => {
                self.top(screen(jumped(cursor, len + 2, button == Button::JumpDown), false));
            }
            Button::Left | Button::Right => {
                let forward = button == Button::Right;
                if editing {
                    self.move_handle(range_id, cursor - len, if forward { 1 } else { -1 });
                } else if on_handle {
                    self.top(screen(if cursor == len { len + 1 } else { len }, false));
                } else {
                    // The checklist is drawn as columns of DELAY_ROWS boxes: Left / Right step a column, staying put at an edge.
                    let next = if forward { cursor + DELAY_ROWS } else { cursor.wrapping_sub(DELAY_ROWS) };
                    if next < len {
                        self.top(screen(next, false));
                    }
                }
            }
            Button::A if on_handle => {
                if range_grey.is_some() {
                    return;
                }
                if editing {
                    self.top(screen(cursor, false));
                } else {
                    if self.get(range_id).is_empty() {
                        self.put(range_id, &[0, 0]);
                    }
                    self.top(screen(cursor, true));
                }
            }
            Button::A => {
                if checklist_grey.is_none() {
                    self.toggle_entry(id, cursor);
                }
            }
            Button::Y => {
                // Clear the active control: a set range is removed, else the checklist is emptied.
                if checklist_grey.is_some() {
                    self.put(range_id, &[]);
                    self.top(screen(cursor, false));
                } else if range_grey.is_some() {
                    let zeros = vec![0u8; len];
                    self.put(id, &zeros);
                }
            }
            Button::ZL => {
                if checklist_grey.is_none() {
                    self.all_or_none(id);
                }
            }
            Button::B => {
                if editing {
                    self.top(screen(cursor, false));
                } else {
                    self.stack.pop();
                }
            }
            _ => {}
        }
    }

    /// Move one handle (`0` Min, `1` Max) of a delay range by `delta` option steps; Min never passes
    /// Max and Max never passes Min, both stay inside the row's value set.
    fn move_handle(&mut self, range_id: &str, handle: usize, delta: isize) {
        let [lo, hi] = match self.get(range_id) {
            [lo, hi] => [*lo as isize, *hi as isize],
            _ => return,
        };
        let top = isize::from(settings::RANGE_INDEX_MAX);
        let (lo, hi) = if handle == 0 { ((lo + delta).clamp(0, hi), hi) } else { (lo, (hi + delta).clamp(lo, top)) };
        self.put(range_id, &[lo as u8, hi as u8]);
    }

    fn press_slider(&mut self, button: Button, row: usize, state: SliderState) {
        let id = ROWS[row].id;
        let [lower, upper] = match self.get(id) {
            [lo, hi] => [*lo, *hi],
            _ => return,
        };
        let selected = matches!(state, SliderState::LowerSelected | SliderState::UpperSelected);
        let lower_handle = matches!(state, SliderState::LowerHover | SliderState::LowerSelected);
        let screen = |state: SliderState| Screen::Slider { row, state };
        let adjust = |delta: i16| -> [u8; 2] {
            let apply = |v: u8, lo: i16, hi: i16| (i16::from(v) + delta).clamp(lo, hi) as u8;
            if lower_handle {
                [apply(lower, 0, i16::from(upper)), upper]
            } else {
                [lower, apply(upper, i16::from(lower), i16::from(settings::SLIDER_MAX))]
            }
        };
        match button {
            Button::A => self.top(screen(match state {
                SliderState::LowerHover => SliderState::LowerSelected,
                SliderState::UpperHover => SliderState::UpperSelected,
                SliderState::LowerSelected => SliderState::LowerHover,
                SliderState::UpperSelected => SliderState::UpperHover,
            })),
            Button::B => {
                if selected {
                    self.top(screen(if lower_handle { SliderState::LowerHover } else { SliderState::UpperHover }));
                } else {
                    self.stack.pop();
                }
            }
            Button::Up if selected => self.put(id, &adjust(i16::from(SLIDER_FAST))),
            Button::Down if selected => self.put(id, &adjust(-i16::from(SLIDER_FAST))),
            Button::Right if selected => self.put(id, &adjust(i16::from(SLIDER_SLOW))),
            Button::Left if selected => self.put(id, &adjust(-i16::from(SLIDER_SLOW))),
            Button::Left | Button::Right if !selected => self.top(screen(if lower_handle {
                SliderState::UpperHover
            } else {
                SliderState::LowerHover
            })),
            _ => {}
        }
    }

    fn press_directions(&mut self, button: Button, row: usize, chip: usize) {
        let single = matches!(ROWS[row].control, Control::DirectionGrid { single: true });
        let id = ROWS[row].id;
        match button {
            Button::Up | Button::Down | Button::Left | Button::Right => {
                let direction = match button {
                    Button::Up => Move::Up,
                    Button::Down => Move::Down,
                    Button::Left => Move::Left,
                    _ => Move::Right,
                };
                self.top(Screen::Directions { row, chip: chip_step(chip, direction) });
            }
            Button::A => {
                let mut array = self.get(id).to_vec();
                if array.len() <= chip {
                    return;
                }
                if single {
                    let was = array[chip] != 0;
                    array.iter_mut().for_each(|v| *v = 0);
                    array[chip] = u8::from(!was);
                } else {
                    // One weight per press; past the largest weight the chip clears again.
                    array[chip] = (array[chip] + 1) % (settings::WEIGHT_MAX + 1);
                }
                self.put(id, &array);
            }
            Button::Y => {
                let mut array = self.get(id).to_vec();
                if single {
                    array.iter_mut().for_each(|v| *v = 0);
                } else if let Some(slot) = array.get_mut(chip) {
                    *slot = 0;
                }
                self.put(id, &array);
            }
            Button::ZL => {
                let mut array = self.get(id).to_vec();
                let all = !single && array.iter().all(|v| *v != 0);
                for slot in &mut array {
                    *slot = if single || all { 0 } else { (*slot).max(1) };
                }
                self.put(id, &array);
            }
            Button::B => {
                self.stack.pop();
            }
            _ => {}
        }
    }

    fn press_swap_groups(&mut self, button: Button, cursor: usize) {
        let len = SWAP_GROUP_IDS.len();
        match button {
            Button::Up => self.top(Screen::SwapGroups { cursor: (cursor + len - 1) % len }),
            Button::Down => self.top(Screen::SwapGroups { cursor: (cursor + 1) % len }),
            Button::JumpUp | Button::JumpDown => {
                self.top(Screen::SwapGroups { cursor: jumped(cursor, len, button == Button::JumpDown) });
            }
            Button::A => {
                if self.swap_entry_grey(cursor).is_none() {
                    let id = SWAP_GROUP_IDS[cursor];
                    let on = self.toggle_by_id(id);
                    self.put(id, if on { &[0, 1] } else { &[1, 0] });
                }
            }
            Button::B => {
                self.stack.pop();
            }
            _ => {}
        }
    }

    fn press_overrides(&mut self, button: Button, situation: usize, cursor: usize) {
        let len = options::ACTIONS.len() + 1;
        let id = SITUATIONS[situation].1;
        let screen = |situation: usize, cursor: usize| Screen::Overrides { situation, cursor };
        match button {
            Button::Up => self.top(screen(situation, (cursor + len - 1) % len)),
            Button::Down => self.top(screen(situation, (cursor + 1) % len)),
            Button::JumpUp | Button::JumpDown => {
                self.top(screen(situation, jumped(cursor, len, button == Button::JumpDown)));
            }
            Button::Left if cursor == 0 => {
                self.top(screen((situation + SITUATIONS.len() - 1) % SITUATIONS.len(), 0));
            }
            Button::Right if cursor == 0 => self.top(screen((situation + 1) % SITUATIONS.len(), 0)),
            // The actions are drawn in columns of OVERRIDE_COLUMN (mockup 06): Left / Right step a column.
            Button::Left | Button::Right if cursor > 0 => {
                let at = cursor - 1;
                let next = if button == Button::Right { at + OVERRIDE_COLUMN } else { at.wrapping_sub(OVERRIDE_COLUMN) };
                if next < options::ACTIONS.len() {
                    self.top(screen(situation, next + 1));
                }
            }
            Button::A if cursor > 0 => self.toggle_entry(id, cursor - 1),
            Button::ZL if cursor > 0 => self.all_or_none(id),
            Button::B => {
                self.stack.pop();
            }
            _ => {}
        }
    }

    fn press_chords(&mut self, button: Button, cursor: usize) {
        let len = CHORD_ROWS.len();
        match button {
            Button::Up => self.top(Screen::Chords { cursor: (cursor + len - 1) % len }),
            Button::Down => self.top(Screen::Chords { cursor: (cursor + 1) % len }),
            Button::JumpUp | Button::JumpDown => {
                self.top(Screen::Chords { cursor: jumped(cursor, len, button == Button::JumpDown) });
            }
            Button::A => {
                let captured = Chord::from_row(self.get(CHORD_ROWS[cursor].1));
                self.stack.push(Screen::ChordEntry { target: ChordTarget::Row(cursor as u8), captured });
            }
            Button::B => {
                self.stack.pop();
            }
            _ => {}
        }
    }

    /// The input adapter's chord capture (hold one button, press the others, SPEC-input-04 §2.2):
    /// replaces what the editor will write. Ignored outside a chord editor.
    pub fn chord_entry(&mut self, chord: Chord) {
        if let Some(Screen::ChordEntry { target, .. }) = self.stack.last().copied() {
            self.top(Screen::ChordEntry { target, captured: chord });
        }
    }

    pub fn chord_capture(&self) -> ChordCapture { self.chord_capture }

    pub fn chord_refusal(&self) -> Option<super::ini_files::EditRefusal> { self.chord_refusal }

    /// Consume capture before processing navigation from the same physical poll.
    /// X arms first even when A/B/Y arrive in that poll; those are data, not an
    /// accidental confirmation/cancellation of the previous preview.
    pub fn chord_poll(&mut self, held: u16, pressed: u16) -> bool {
        if !matches!(self.screen(), Some(Screen::ChordEntry { .. })) {
            self.chord_capture = ChordCapture::Preview;
            self.chord_refusal = None;
            return false;
        }
        match self.chord_capture {
            ChordCapture::Preview if pressed & (1 << 2) != 0 => {
                self.chord_refusal = None;
                self.chord_capture = ChordCapture::Release;
                true
            }
            ChordCapture::Preview => false,
            ChordCapture::Release => {
                if held == 0 { self.chord_capture = ChordCapture::Buttons(Chord::NONE); }
                true
            }
            ChordCapture::Buttons(seen) => {
                let next = Chord(seen.0 | held);
                if !next.is_empty() {
                    self.chord_entry(next);
                    self.chord_capture = if held == 0 { ChordCapture::Preview } else { ChordCapture::Buttons(next) };
                }
                true
            }
        }
    }

    /// Only INI shortcut confirmation is immediate; settings still commit on close.
    pub fn take_button_config(&mut self) -> Option<[Chord; 2]> { self.outbox.button_config.take() }

    /// The runtime's authoritative slots after applying/refusing a confirmed edit.
    pub fn shortcuts_applied(&mut self, shortcuts: [Chord; 2]) { self.env.shortcuts = shortcuts; }

    fn press_chord_entry(&mut self, button: Button, target: ChordTarget, captured: Chord) {
        self.chord_refusal = None;
        match button {
            // Confirm.
            Button::A => {
                if self.confirm_chord(target, captured) { self.stack.pop(); }
            }
            Button::X => self.chord_capture = ChordCapture::Release,
            // Empty the chord (a menu shortcut with an empty chord is disabled).
            Button::Y => self.top(Screen::ChordEntry { target, captured: Chord::NONE }),
            Button::B => {
                self.stack.pop();
            }
            _ => {}
        }
    }

    fn confirm_chord(&mut self, target: ChordTarget, chord: Chord) -> bool {
        match target {
            ChordTarget::Row(i) => {
                let id = CHORD_ROWS[usize::from(i)].1;
                self.put(id, &chord.to_row());
            }
            ChordTarget::Shortcut(slot) => {
                let slot = usize::from(slot & 1);
                let [first, second] = self.env.shortcuts;
                if let Err(why) = super::ini_files::check_edit(super::menu_shortcuts::MenuShortcuts::new(first, second), slot, chord) {
                    self.chord_refusal = Some(why);
                    return false;
                }
                if self.env.shortcuts[slot] != chord {
                    self.env.shortcuts[slot] = chord;
                    self.outbox.button_config = Some(self.env.shortcuts);
                }
            }
        }
        true
    }

    fn press_grid(&mut self, button: Button, layout: GridLayout, page: usize, cell: usize) {
        let screen = |page: usize, cell: usize| Screen::Grid { layout, page, cell };
        match button {
            Button::Up => self.top(screen(page, layout.step(page, cell, Move::Up))),
            Button::Down => self.top(screen(page, layout.step(page, cell, Move::Down))),
            Button::Left => self.top(screen(page, layout.step(page, cell, Move::Left))),
            Button::Right => self.top(screen(page, layout.step(page, cell, Move::Right))),
            Button::L | Button::R if !matches!(layout.kind, GridKind::Roster(_)) => {
                let pages = layout.pages();
                let next = if button == Button::R { (page + 1) % pages } else { (page + pages - 1) % pages };
                let cell = cell.min(layout.page_len(next).saturating_sub(1));
                self.top(screen(next, cell));
            }
            // A, X, Y on the Lobby Stages or Change Stage grid: the stage editor applies them (rotation order, direct stage, form).
            Button::A | Button::X | Button::Y if matches!(layout.kind, GridKind::Stages | GridKind::StageChange) => {
                if let Some(tile) = layout.tile(page, cell) {
                    self.outbox.stage_press = u16::try_from(tile).ok().map(|tile| (tile, button));
                }
            }
            // The roster editor owns pick/rank, form preview, Solo Popo, removal and role switching.
            Button::A | Button::X | Button::Y | Button::R | Button::ZR if matches!(layout.kind, GridKind::Roster(_)) => {
                if let Some(tile) = layout.tile(page, cell) {
                    self.outbox.roster_press = u16::try_from(tile).ok().map(|tile| (tile, button));
                }
            }
            // A on the Give item grid: give the item now (every press, the already-chosen item included).
            Button::A if layout.kind == GridKind::Items => {
                if let Some(item) = layout.tile(page, cell) {
                    self.outbox.give_item = u8::try_from(item).ok();
                }
            }
            Button::B => {
                self.stack.pop();
            }
            _ => {}
        }
    }

    /// The item the player chose on the Give item grid since the last call, if any.
    pub fn take_give_item(&mut self) -> Option<u8> {
        self.outbox.give_item.take()
    }

    /// The press on the Lobby Stages or Change Stage grid since the last call, if any: (tile, button), tile 0 being All.
    pub fn take_stage_press(&mut self) -> Option<(u16, Button)> {
        self.outbox.stage_press.take()
    }

    /// The roster grid's press since the last call, if any.
    pub fn take_roster_press(&mut self) -> Option<(u16, Button)> {
        self.outbox.roster_press.take()
    }

    /// The portrait roster grid (one page, the same in both console modes).
    fn roster_screen(&self, use_: RosterUse) -> Screen {
        Screen::Grid { layout: GridLayout::new(GridKind::Roster(use_), self.env.console), page: 0, cell: 0 }
    }

    /// A kept key of the menu snapshot (a non-array row: the rosters, the hats, the stage order).
    pub fn kept(&self, key: &str) -> Option<&serde_json::Value> {
        self.settings.menu.extra().get(key)
    }

    /// Replace a kept key of the menu snapshot (an editor's write); notes the change for the commit.
    pub fn set_kept(&mut self, key: &str, value: serde_json::Value) {
        if self.kept(key) != Some(&value) && self.settings.menu.set_kept(key, value).is_ok() {
            self.outbox.changed = true;
            self.note_edit(key);
        }
    }

    /// A top-level key of the settings file kept as it is (stage_preferences, mii_preferences).
    pub fn top_extra(&self, key: &str) -> Option<&serde_json::Value> {
        self.settings.extra.get(key)
    }

    /// Replace a top-level kept key (an editor's write); notes the change for the commit.
    pub fn set_top_extra(&mut self, key: &str, value: serde_json::Value) {
        if self.top_extra(key) != Some(&value) {
            self.settings.extra.insert(key.to_owned(), value);
            self.outbox.changed = true;
            self.note_edit(key);
        }
    }
}

/// Where the focus starts in a direction grid: the Neutral chip.
pub const DIRECTION_START: usize = 8;

/// Position (column, row) of each chip, by array index: In · Neutral · Out across and Up · - · Down
/// down as 3 x 3 (In on the left), then the two absolute chips in a column of their own.
pub const CHIP_POS: [(u8, u8); 11] = [
    (2, 1), // Out
    (2, 0), // Up Out
    (1, 0), // Up
    (0, 0), // Up In
    (0, 1), // In
    (0, 2), // Down In
    (1, 2), // Down
    (2, 2), // Down Out
    (1, 1), // Neutral
    (3, 0), // Left (absolute)
    (3, 1), // Right (absolute)
];

fn chip_at(col: u8, row: u8) -> Option<usize> {
    CHIP_POS.iter().position(|pos| *pos == (col, row))
}

/// Move between chips; rows and columns wrap and empty cells are skipped.
fn chip_step(chip: usize, direction: Move) -> usize {
    let (mut col, mut row) = CHIP_POS[chip];
    for _ in 0..4 {
        match direction {
            Move::Left => col = (col + 3) % 4,
            Move::Right => col = (col + 1) % 4,
            Move::Up => row = (row + 2) % 3,
            Move::Down => row = (row + 1) % 3,
        }
        if let Some(next) = chip_at(col, row) {
            return next;
        }
    }
    chip
}

/// The Yes / No / Random choice of a weighted `[True, False]` array: both set is Random.
fn yes_no_random(array: &[u8]) -> Option<usize> {
    match (array.first().copied().unwrap_or(0) != 0, array.get(1).copied().unwrap_or(0) != 0) {
        (true, false) => Some(0),
        (false, true) => Some(1),
        (true, true) => Some(2),
        (false, false) => None,
    }
}
