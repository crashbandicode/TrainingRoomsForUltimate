//! The menu as one frame of text: everything the renderer writes into the layout's panes, built from the
//! menu model (`model::menu`) with no allocation in the frame itself (fixed UTF-16 buffers), so the
//! renderer's tick only copies. SPEC-core-03 §2.2 (page, footer, key hints), §2.5 (two-line help panel).
//!
//! The model decides what is on screen and where the cursor is; this module only says how it reads.

use core::fmt::{self, Write};

use super::menu::table::{Control, Editor, Info, Row, GROUPS, ROWS};
use super::menu::{Entry, Grey, Menu, RowValue};
use super::panes::{GROUPS as GROUP_LINES, ROWS as ROW_LINES, TABS};

/// Fixed-capacity UTF-16 text: what a text pane's buffer holds. Writing past the capacity cuts the text.
#[derive(Clone, Copy)]
pub struct Text<const N: usize> {
    units: [u16; N],
    len: u16,
}

impl<const N: usize> Text<N> {
    pub const fn new() -> Text<N> {
        Text { units: [0; N], len: 0 }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn units(&self) -> &[u16] {
        &self.units[..usize::from(self.len)]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// One UTF-16 unit as is (a private-use key glyph of the game's font).
    pub fn push_unit(&mut self, unit: u16) {
        if usize::from(self.len) < N {
            self.units[usize::from(self.len)] = unit;
            self.len += 1;
        }
    }

    pub fn string_lossy(&self) -> String {
        String::from_utf16_lossy(self.units())
    }
}

impl<const N: usize> Default for Text<N> {
    fn default() -> Self {
        Text::new()
    }
}

impl<const N: usize> PartialEq for Text<N> {
    fn eq(&self, other: &Self) -> bool {
        self.units() == other.units()
    }
}

impl<const N: usize> fmt::Debug for Text<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.string_lossy())
    }
}

impl<const N: usize> Write for Text<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for unit in s.encode_utf16() {
            self.push_unit(unit);
        }
        Ok(())
    }
}

/// How a line is coloured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Normal,
    /// Unfocused chrome: inactive tabs and groups, hints.
    Dim,
    /// A switch that is on.
    On,
    /// A switch that is off, or a row greyed by its dependency.
    Off,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RowLine {
    pub label: Text<80>,
    pub value: Text<72>,
    pub label_tone: Tone,
    pub value_tone: Tone,
}

/// The effect visual of the direction chips.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChipVisual {
    /// The clean room's own arrow chips (Airdodge Direction, Shield Tilt; and DI / SDI while the game's visual is not available).
    #[default]
    None,
    /// DI Direction: the game's `sys_vector`.
    Di,
    /// SDI Direction: the game's `sys_hitstop_spark`.
    Sdi,
}

/// What fills the area right of the divider.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    /// The rows of the page or a popup.
    #[default]
    List,
    /// The direction grid's chips.
    Chips,
    /// The item or stage grid's tiles.
    Tiles,
    /// The delay popup (N3, mockup 19): the checklist as a grid of boxes over the min-max range control.
    Delay,
    /// The situation overrides screen (mockup 06): the situation line over two columns of action boxes.
    Overrides,
    /// The replay window's clip editor: its text lines over a timeline (the crop window and the playhead).
    Clip,
}

/// The delay popup's range control: two handles on a bar over the row's value set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RangeView {
    /// Both handles are set (else the bar is empty).
    pub set: bool,
    pub min: u8,
    pub max: u8,
    /// The last value index (the bar's scale).
    pub top: u8,
    /// The control is greyed (the checklist holds a value).
    pub grey: bool,
    /// Which handle the cursor is on: 0 none (the cursor is on the checklist), 1 Min, 2 Max.
    pub focus: u8,
    pub editing: bool,
}

/// The clip editor's timeline (frames of the capture: the whole length, the crop window and the playhead).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClipBar {
    pub total: u32,
    pub start: u32,
    pub end: u32,
    pub cursor: u32,
    /// Which handle the Left / Right keys move: 0 the playhead, 1 the start, 2 the end.
    pub handle: u8,
    pub playing: bool,
}

/// One frame of the menu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub view: View,
    /// Weight of each chip (array index order of options::DIRECTIONS) and the focused chip.
    pub chip_weights: [u8; super::panes::CHIPS],
    pub chip_focus: u8,
    /// Which of the game's own effect visuals the chips draw (SPEC-core-07 section 4.6): DI and SDI only, the other grids keep the arrows.
    pub chip_visual: ChipVisual,
    /// The tile grid: its geometry, the labels of the tiles on this page and the focused tile.
    pub tile_cols: u8,
    pub tile_rows: u8,
    pub tiles: [Text<28>; super::panes::TILES],
    pub tile_count: u8,
    pub tile_focus: u8,
    /// The stage grid's tiles: tile number per tile (0 = All, n = catalog stage n - 1; `u16::MAX` = none), whether this is a
    /// stage grid at all, and a signature that changes when the page or the layout changes (thumbnails are requested per page).
    pub tile_n: [u16; super::panes::TILES],
    pub tile_stage_grid: bool,
    pub tile_item_grid: bool,
    pub tile_roster_grid: bool,
    pub tile_cpu: bool,
    /// A short last row is centred under the full ones (the portrait roster's last row of 9 under 13 columns).
    pub tile_centre_last: bool,
    pub tile_sig: u32,
    /// Delay popup: per tile 0 clear, 1 ticked, 2 greyed (tile_focus 255 = the cursor is on a range handle).
    pub tile_state: [u8; super::panes::TILES],
    /// The roster grid's per-role marks, `[Player, CPU]`: the fork's corner badges (the order number, `0` or
    /// `All`), All-stripes (the cell is in that role's All order) and Solo Popo "S" markers (the Ice Climbers cell).
    pub tile_badges: [[Text<4>; super::panes::TILES]; 2],
    pub tile_stripes: [[bool; super::panes::TILES]; 2],
    pub tile_solo: [bool; 2],
    pub range: RangeView,
    pub clip: ClipBar,
    pub title: Text<40>,
    pub tabs: [Text<40>; TABS],
    pub tab_count: u8,
    pub tab_active: u8,
    pub crumb: Text<120>,
    pub groups: [Text<48>; GROUP_LINES],
    pub group_count: u8,
    pub group_active: u8,
    pub rows: [RowLine; ROW_LINES],
    pub row_count: u8,
    /// The focused line of `rows`, if any (a popup's cursor too).
    pub focus: Option<u8>,
    pub help: [Text<160>; 2],
    pub hint: Text<160>,
}

impl Frame {
    /// Reuse a frame in place. Constructing a whole temporary on each nested
    /// screen path exhausted the menu thread's stack when the roster grew.
    pub fn reset(&mut self) {
        self.view = View::List;
        self.chip_weights.fill(0);
        self.chip_focus = 0;
        self.chip_visual = ChipVisual::None;
        self.tile_cols = 0;
        self.tile_rows = 0;
        self.tiles.fill(Text::new());
        self.tile_count = 0;
        self.tile_focus = 0;
        self.tile_n.fill(u16::MAX);
        self.tile_stage_grid = false;
        self.tile_item_grid = false;
        self.tile_roster_grid = false;
        self.tile_cpu = false;
        self.tile_centre_last = false;
        self.tile_sig = 0;
        self.tile_state.fill(0);
        self.tile_badges = [[Text::new(); super::panes::TILES]; 2];
        self.tile_stripes = [[false; super::panes::TILES]; 2];
        self.tile_solo = [false; 2];
        self.range = RangeView::default();
        self.clip = ClipBar::default();
        self.title.clear();
        self.tabs.fill(Text::new());
        self.tab_count = 0;
        self.tab_active = 0;
        self.crumb.clear();
        self.groups.fill(Text::new());
        self.group_count = 0;
        self.group_active = 0;
        self.rows.fill(RowLine::default());
        self.row_count = 0;
        self.focus = None;
        self.help.fill(Text::new());
        self.hint.clear();
    }

    pub const fn new() -> Frame {
        const EMPTY_ROW: RowLine = RowLine {
            label: Text::new(),
            value: Text::new(),
            label_tone: Tone::Normal,
            value_tone: Tone::Normal,
        };
        Frame {
            view: View::List,
            chip_weights: [0; super::panes::CHIPS],
            chip_focus: 0,
            chip_visual: ChipVisual::None,
            tile_cols: 0,
            tile_rows: 0,
            tiles: [Text::new(); super::panes::TILES],
            tile_count: 0,
            tile_focus: 0,
            tile_n: [u16::MAX; super::panes::TILES],
            tile_stage_grid: false,
            tile_item_grid: false,
            tile_roster_grid: false,
            tile_cpu: false,
            tile_centre_last: false,
            tile_sig: 0,
            tile_state: [0; super::panes::TILES],
            tile_badges: [[Text::new(); super::panes::TILES]; 2],
            tile_stripes: [[false; super::panes::TILES]; 2],
            tile_solo: [false; 2],
            range: RangeView { set: false, min: 0, max: 0, top: 0, grey: false, focus: 0, editing: false },
            clip: ClipBar { total: 0, start: 0, end: 0, cursor: 0, handle: 0, playing: false },
            title: Text::new(),
            tabs: [Text::new(); TABS],
            tab_count: 0,
            tab_active: 0,
            crumb: Text::new(),
            groups: [Text::new(); GROUP_LINES],
            group_count: 0,
            group_active: 0,
            rows: [EMPTY_ROW; ROW_LINES],
            row_count: 0,
            focus: None,
            help: [Text::new(); 2],
            hint: Text::new(),
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Frame::new()
    }
}

pub const TITLE: &str = "TrainingRooms";

/// Real frames of discarded input and fade after the menu closes (SPEC-core-03 §2.3).
pub const CLOSE_FRAMES: u32 = 15;

/// The panel's alpha count real frames after a close: 255 / log10(16) * log10(remaining + 1) with remaining = 15 - count (saturating), 255 before the first frame, 0 when none is left (SPEC-core-03 §5).
pub fn fade_alpha(count: u32) -> u8 {
    let remaining = CLOSE_FRAMES.saturating_sub(count);
    if remaining >= CLOSE_FRAMES {
        255
    } else if remaining == 0 {
        0
    } else {
        (255.0 / 16f32.log10() * ((remaining + 1) as f32).log10()).clamp(0.0, 255.0) as u8
    }
}
const HINT_PAGE: &str = "L/R page   Up/Down move   Left/Right change   A open   X save defaults   Y reset row   ZR reset page   B close";

fn group_of(entry: Entry) -> usize {
    match entry {
        Entry::Group(g) => g,
        Entry::Row(r) => usize::from(ROWS[r].group),
    }
}

/// A name row's value (the combo target, the draft): the name, or "none".
fn named(out: &mut Text<72>, name: &str) -> fmt::Result {
    if name.is_empty() {
        out.write_str("none  >")
    } else {
        write!(out, "{name}  >")
    }
}

/// How a row's value reads on the page (the mockups' right-hand column).
pub fn write_value(menu: &Menu, row: usize, out: &mut Text<72>) {
    let r = &ROWS[row];
    let cycles = matches!(r.control, Control::Dropdown(_) | Control::Number { .. } | Control::YesNoRandom);
    if cycles {
        let _ = out.write_str("<  ");
    }
    match menu.value(row) {
        RowValue::Toggle(on) => {
            let _ = out.write_str(if on { "[ON]" } else { "[OFF]" });
        }
        RowValue::Choice(at) => {
            let label = match (r.control, at) {
                (Control::Dropdown(labels), Some(i)) => labels.get(i).copied().unwrap_or("-"),
                (Control::YesNoRandom, Some(i)) => super::menu::options::YES_NO_RANDOM.get(i).copied().unwrap_or("-"),
                _ => "-",
            };
            let _ = out.write_str(label);
        }
        RowValue::Number(Some(n)) => {
            let _ = write!(out, "{n}");
        }
        RowValue::Number(None) => {
            let _ = out.write_str("-");
        }
        RowValue::Checklist { ticked, total } => {
            let _ = write!(out, "{ticked} of {total}  >");
        }
        RowValue::Delay(view) => {
            use super::menu::DelayView;
            let _ = match view {
                DelayView::Empty => out.write_str("none  >"),
                DelayView::Ticked(n) => write!(out, "{n} selected  >"),
                DelayView::Range(range) => write!(out, "{} - {}  >", range.min, range.max),
            };
        }
        RowValue::Slider { lower, upper } => {
            let _ = write!(out, "{lower} - {upper}  >");
        }
        RowValue::Chips(mask) => {
            let _ = write!(out, "{} selected  >", mask.count_ones());
        }
        RowValue::Chord(chord) => {
            let _ = write!(out, "{chord}  >");
        }
        RowValue::OverridesSet(n) => {
            let _ = write!(out, "{n} of 13 set  >");
        }
        RowValue::None => {
            let _ = match r.control {
                Control::Info(Info::PermanentShortcut) => out.write_str("B + D-pad Up"),
                Control::Info(Info::PilotControl) => out.write_str("CPU pilot plugin"),
                Control::Editor(Editor::ComboTarget) => named(out, menu.env().combo_target.as_str()),
                Control::Editor(Editor::ComboName) => named(out, menu.env().combo_draft.as_str()),
                _ => out.write_str(">"),
            };
        }
    }
    if cycles {
        let _ = out.write_str("  >");
    }
}

fn value_tone(menu: &Menu, row: usize) -> Tone {
    if menu.grey(row).is_some() {
        return Tone::Off;
    }
    match menu.value(row) {
        RowValue::Toggle(true) => Tone::On,
        RowValue::Toggle(false) => Tone::Off,
        _ => Tone::Normal,
    }
}

/// The page view: tabs, the groups of the page on the left, the active group's rows on the right,
/// the focused row's two help lines and the key hints.
pub fn page_frame(menu: &Menu, frame: &mut Frame) {
    frame.reset();
    let _ = frame.title.write_str(TITLE);
    for (i, page) in menu.ring().take(TABS).enumerate() {
        let _ = frame.tabs[i].write_str(page.title());
        frame.tab_count = (i + 1) as u8;
        if page == menu.page() {
            frame.tab_active = i as u8;
        }
    }

    // The groups of the page, with how many rows each has now.
    let page = menu.page();
    let active = menu.focus().map(group_of);
    let pilot = menu.env().pilot_present;
    for n in 0..menu.entry_count(page) {
        let Some(Entry::Group(g)) = menu.entry(page, n) else { continue };
        let at = usize::from(frame.group_count);
        if at >= GROUP_LINES {
            break;
        }
        let rows = ROWS.iter().filter(|row| usize::from(row.group) == g && row.visible(pilot)).count();
        let _ = write!(frame.groups[at], "{}  ({rows})", GROUPS[g].title);
        if Some(g) == active {
            frame.group_active = at as u8;
        }
        frame.group_count += 1;
    }

    // The entries of the active group; the focus is one of them.
    let mut lines: [Option<Entry>; 40] = [None; 40];
    let mut count = 0;
    let mut focus_line = None;
    for n in 0..menu.entry_count(page) {
        let Some(entry) = menu.entry(page, n) else { continue };
        if Some(group_of(entry)) != active {
            continue;
        }
        // A section header is a line of the right-hand list only for a collapsible group.
        if let Entry::Group(g) = entry {
            if !GROUPS[g].advanced {
                continue;
            }
        }
        if count < lines.len() {
            if n == menu.cursor() {
                focus_line = Some(count);
            }
            lines[count] = Some(entry);
            count += 1;
        }
    }
    let first = match focus_line {
        Some(f) if f >= ROW_LINES => f + 1 - ROW_LINES,
        _ => 0,
    };
    for (slot, entry) in lines[first..count.min(first + ROW_LINES)].iter().enumerate() {
        let line = &mut frame.rows[slot];
        match entry {
            Some(Entry::Group(g)) => {
                let marker = if menu.group_is_open(*g) { "v" } else { ">" };
                let _ = write!(line.label, "{marker} {} (advanced)", GROUPS[*g].title);
                line.label_tone = Tone::Dim;
            }
            Some(Entry::Row(r)) => {
                let row: &Row = &ROWS[*r];
                let _ = line.label.write_str(row.title);
                if row.advanced {
                    let _ = line.label.write_str("  (advanced)");
                }
                write_value(menu, *r, &mut line.value);
                line.value_tone = value_tone(menu, *r);
                line.label_tone = if menu.grey(*r).is_some() { Tone::Off } else { Tone::Normal };
            }
            None => {}
        }
        frame.row_count += 1;
    }
    frame.focus = focus_line.map(|f| (f - first) as u8);

    match menu.help_row() {
        Some(row) => {
            let (one, two) = row.help_lines();
            let _ = frame.help[0].write_str(one);
            let _ = frame.help[1].write_str(two);
            if let Some(grey) = menu.focused_row().and_then(|r| menu.grey(r)) {
                frame.help[1].clear();
                let _ = frame.help[1].write_str(Grey::text(grey));
            }
        }
        None => {
            let _ = frame.help[0].write_str("A opens or closes this section.");
        }
    }
    let _ = frame.hint.write_str(HINT_PAGE);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::menu::input::Button;
    use crate::model::menu::Env;
    use crate::model::settings::Settings;

    fn menu() -> Menu {
        Menu::new(Settings::factory(), Env::default())
    }

    #[test]
    fn the_first_open_shows_cpu_with_its_groups_and_the_first_rows() {
        let m = menu();
        let mut f = Frame::new();
        page_frame(&m, &mut f);
        assert_eq!(f.tabs[0].string_lossy(), "CPU");
        assert_eq!(f.tab_count, 5);
        assert_eq!(f.tab_active, 0);
        assert_eq!(f.group_count, 6);
        assert_eq!(f.groups[0].string_lossy(), "Reactions  (4)");
        assert_eq!(f.group_active, 0);
        assert_eq!(f.rows[0].label.string_lossy(), "Mash");
        assert_eq!(f.row_count, 4);
        assert_eq!(f.focus, Some(0));
        assert!(!f.help[0].is_empty());
    }

    #[test]
    fn moving_down_into_the_next_group_switches_the_group_and_its_rows() {
        let mut m = menu();
        for _ in 0..4 {
            m.press(Button::Down);
        }
        let mut f = Frame::new();
        page_frame(&m, &mut f);
        assert_eq!(f.group_active, 1);
        assert_eq!(f.rows[0].label.string_lossy(), "Airdodge Direction");
        assert_eq!(f.focus, Some(0));
        assert_eq!(f.row_count, 6);
    }

    #[test]
    fn a_toggle_and_a_dropdown_read_like_the_mockup() {
        let mut m = menu();
        for _ in 0..4 {
            m.press(Button::Down);
        }
        let mut f = Frame::new();
        page_frame(&m, &mut f);
        let dropdown = (0..f.row_count as usize).find(|i| f.rows[*i].label.string_lossy() == "SDI Strength").unwrap();
        assert!(f.rows[dropdown].value.string_lossy().starts_with("<  "));
        let toggle = (0..f.row_count as usize).find(|i| f.rows[*i].label.string_lossy() == "Dodge Staling").unwrap();
        assert!(matches!(f.rows[toggle].value_tone, Tone::On | Tone::Off));
    }

    #[test]
    fn every_page_of_the_ring_fits_the_row_budget_of_the_layout() {
        for developer in [false, true] {
            let mut m = Menu::new(Settings::factory(), Env { developer_page: developer, ..Env::default() });
            for _ in 0..8 {
                let page = m.page();
                for _ in 0..m.entry_count(page) + 1 {
                    let mut f = Frame::new();
                    page_frame(&m, &mut f);
                    let expected = {
                        let active = m.focus().map(group_of);
                        (0..m.entry_count(page))
                            .filter_map(|n| m.entry(page, n))
                            .filter(|e| Some(group_of(*e)) == active && !matches!(e, Entry::Group(g) if !GROUPS[*g].advanced))
                            .count()
                    };
                    assert!(expected <= ROW_LINES, "{page:?}: {expected} lines in one group");
                    assert!(f.group_count as usize <= GROUP_LINES);
                    m.press(Button::Down);
                }
                m.press(Button::R);
            }
        }
    }

    #[test]
    fn the_fade_is_logarithmic_from_255_to_0_over_15_frames() {
        assert_eq!(fade_alpha(0), 255);
        assert_eq!(fade_alpha(15), 0);
        assert_eq!(fade_alpha(99), 0);
        let curve: Vec<u8> = (1..15).map(fade_alpha).collect();
        assert!(curve.windows(2).all(|w| w[0] >= w[1]), "{curve:?}");
        assert!(curve[0] > 240 && curve[13] < 70);
    }

    #[test]
    fn text_is_cut_at_its_capacity_never_past_it() {
        let mut t: Text<4> = Text::new();
        let _ = t.write_str("abcdefgh");
        assert_eq!(t.string_lossy(), "abcd");
    }
}
