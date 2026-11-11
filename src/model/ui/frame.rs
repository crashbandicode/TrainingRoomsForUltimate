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
    /// The r8 line under a recording row ("Recorded on …"); empty for any other row.
    pub sub: Text<64>,
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
    /// Floppy panes, then the two loop panes. Hidden until a recording screen places one.
    pub icons: [MarkIcon; super::panes::FLOPPIES + super::panes::LOOPS],
    /// The recording pop-up is the window. The background shrinks so the match HUD stays visible.
    pub popup: bool,
    /// Green plate behind a recording row (the active file or the bound save).
    pub green: [bool; ROW_LINES],
    /// White cursor border and left bar, instead of the dark highlight, on a recording screen.
    pub white_cursor: bool,
    /// Row under which the hot-five hairline is drawn, when that boundary is on screen.
    pub hairline: Option<u8>,
    /// Visible hot block of each section: first row slot and how many rows it covers.
    pub outline: [Option<(u8, u8)>; 2],
    /// A chain row. The link glyph is drawn when the layout has imported it.
    pub chain: [bool; ROW_LINES],
    /// The pop-up's CONNECTIONS column (r8 mockup): its header and lines, the row under the cursor first. Empty on the
    /// page, which keeps CONNECTIONS in the help lines.
    pub conn_head: Text<48>,
    pub conn: [Text<72>; super::panes::CONN_LINES],
    pub conn_tone: [Tone; super::panes::CONN_LINES],
    pub conn_count: u8,
    /// What each CONNECTIONS line is in the r8 tree: a kind box (with its pill), a heading, a link, a plain line.
    pub conn_kind: [super::recording_menu::ConnKind; super::panes::CONN_LINES],
    /// The pop-up's L/R role tabs in the title (r8): 0 Player, 1 CPU. `None` off the pop-up.
    pub role_tab: Option<u8>,
    /// The pop-up's section headers sit on a dark band (r8), with their kind's mark before the name: the floppy for
    /// SAVE STATES, a square for the others.
    pub band: [bool; ROW_LINES],
    pub head_tag: [Option<super::recording_menu::FileTag>; 2],
    /// The pop-up's red order badges at a row's left edge (r8 "Red number"): row slot and number.
    pub badges: [Option<(u8, u8)>; super::panes::BADGES],
    /// The pop-up draws its page tabs as outlined boxes, the active one in green (r8), not on the red accent bar.
    pub tab_boxes: bool,
}

/// One floppy or loop mark. `color` is sRGB; the renderer converts it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkIcon {
    pub on: bool,
    pub x: f32,
    pub y: f32,
    pub color: [u8; 4],
}

impl MarkIcon {
    pub const HIDDEN: MarkIcon = MarkIcon { on: false, x: 0.0, y: 0.0, color: [255, 255, 255, 255] };
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
        self.icons = [MarkIcon::HIDDEN; super::panes::FLOPPIES + super::panes::LOOPS];
        self.popup = false;
        self.green = [false; ROW_LINES];
        self.white_cursor = false;
        self.hairline = None;
        self.outline = [None; 2];
        self.chain = [false; ROW_LINES];
        self.conn_head.clear();
        self.conn.fill(Text::new());
        self.conn_tone = [Tone::Normal; super::panes::CONN_LINES];
        self.conn_count = 0;
        self.conn_kind = [super::recording_menu::ConnKind::Detail; super::panes::CONN_LINES];
        self.role_tab = None;
        self.band = [false; ROW_LINES];
        self.head_tag = [None; 2];
        self.badges = [None; super::panes::BADGES];
        self.tab_boxes = false;
    }

    pub const fn new() -> Frame {
        const EMPTY_ROW: RowLine = RowLine {
            label: Text::new(),
            value: Text::new(),
            sub: Text::new(),
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
            icons: [MarkIcon::HIDDEN; super::panes::FLOPPIES + super::panes::LOOPS],
            popup: false,
            green: [false; ROW_LINES],
            white_cursor: false,
            hairline: None,
            outline: [None; 2],
            chain: [false; ROW_LINES],
            conn_head: Text::new(),
            conn: [Text::new(); super::panes::CONN_LINES],
            conn_tone: [Tone::Normal; super::panes::CONN_LINES],
            conn_count: 0,
            conn_kind: [super::recording_menu::ConnKind::Detail; super::panes::CONN_LINES],
            role_tab: None,
            band: [false; ROW_LINES],
            head_tag: [None; 2],
            badges: [None; super::panes::BADGES],
            tab_boxes: false,
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Frame::new()
    }
}

pub const TITLE: &str = "TrainingRooms";

/// Which build this is (R13): the crate version, the internal build name and the commit's short sha (`-dirty` with
/// uncommitted changes, `unknown` without git), set by `build.rs` at compile time.
pub const BUILD_TAG: &str = concat!(env!("CARGO_PKG_VERSION"), " · ", env!("TR_BUILD_NAME"), " ", env!("TR_BUILD_SHA"));

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
/// The page's last line, and the title of its confirmation.
pub const RESET_LABEL: &str = "Reset page to defaults";
const HINT_PAGE: &str = "L/R page   Up/Down move   ZL+Up/Down section   Left/Right change   A open   X save defaults   Y reset row   B close";

fn group_of(entry: Entry) -> usize {
    match entry {
        Entry::Group(g) | Entry::ResetPage(g) => g,
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
    if r.id == "playback_loop" {
        let _ = write!(out, "<  {}  >", menu.recording().loop_label(menu.recording().page_role()));
        return;
    }
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
        let _ = write!(frame.groups[at], "{}  ({})", GROUPS[g].title, group_rows(menu, g, pilot));
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
            Some(Entry::ResetPage(_)) => {
                let _ = line.label.write_str(RESET_LABEL);
                let _ = line.value.write_str(">");
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
        None if matches!(menu.focus(), Some(Entry::ResetPage(_))) => {
            let _ = frame.help[0].write_str("Puts every setting of this page back to your saved defaults (X saves them).");
            let _ = frame.help[1].write_str("A asks to confirm first.");
        }
        None => {
            let _ = frame.help[0].write_str("A opens or closes this section.");
        }
    }
    let _ = frame.hint.write_str(HINT_PAGE);
    paint_recording(menu, frame);
    // The build tag ends the page's own key hints, after them, in the same dim line (R13); the recording pop-up keeps its
    // r8 footer.
    let tag = menu.env().build_tag;
    if !frame.popup && !tag.is_empty() {
        let _ = write!(frame.hint, "      {tag}");
    }
}

/// The count beside a group title: the rows it lists. A recording library group lists files, not
/// settings rows (it has none), so it counts the catalog's files.
fn group_rows(menu: &Menu, g: usize, pilot: bool) -> usize {
    use super::menu::table::{COMBOS_SEQ, SAVES_MACROS};
    let files = menu.recording().catalog();
    match u8::try_from(g) {
        Ok(SAVES_MACROS) => files.saves.len() + files.macros.len(),
        Ok(COMBOS_SEQ) => files.combos.len() + files.sequences.len(),
        _ => ROWS.iter().filter(|row| usize::from(row.group) == g && row.visible(pilot)).count(),
    }
}

/// Rows of one section the twelve row panes can show under a header.
const SECTION_ROWS: usize = 4;

fn paint_recording(menu: &Menu, frame: &mut Frame) {
    use super::menu::table::{Page, COMBOS_SEQ, SAVES_MACROS};
    use super::recording_menu::Face;
    let rec = menu.recording();
    let popup = rec.popup_open();
    let on_page = menu.page() == Page::Recording && menu.screen().is_none();
    if !popup && !on_page {
        return;
    }
    let face = if popup { Face::Popup } else { Face::Page };
    if popup {
        frame.popup = true;
        frame.group_count = 0;
        frame.tabs.fill(Text::new());
        frame.tab_count = 0;
        for (i, name) in rec.tab_names().iter().enumerate().take(TABS) {
            let _ = frame.tabs[i].write_str(name);
            frame.tab_count = (i + 1) as u8;
        }
        frame.tab_active = rec.tab() as u8;
        // The r8 title: "Recording", with the role in the L/R tabs beside it.
        let _ = frame.crumb.write_str("Recording");
        frame.role_tab = Some(u8::from(rec.role(face) == super::recording_menu::ViewRole::Cpu));
        // The page tabs are outlined boxes under the title line; the red tab is the role's.
        frame.tab_boxes = true;
    }
    // The recording hint replaces the page hint page_frame wrote; appended, the two ran together.
    frame.hint.clear();
    let _ = frame.hint.write_str(rec.hint(face));
    let group_now = menu.focus().map(group_of);
    let on_library = group_now.is_some_and(|g| g == usize::from(SAVES_MACROS) || g == usize::from(COMBOS_SEQ));
    if !(popup || on_library || rec.file_open() || rec.picker_open()) {
        return;
    }
    let group = if popup {
        if rec.tab() == 0 { SAVES_MACROS } else { COMBOS_SEQ }
    } else {
        group_now.unwrap_or(usize::from(SAVES_MACROS)) as u8
    };
    frame.rows.fill(RowLine::default());
    frame.row_count = 0;
    frame.focus = None;
    frame.white_cursor = true;
    if rec.picker_open() {
        paint_picker(rec, frame);
    } else if rec.crop_open() {
        paint_crop(rec, frame);
    } else if rec.file_open() && face == Face::Page {
        paint_file(rec, frame);
    } else {
        paint_sections(rec, frame, face, group);
    }
    let lines = if popup { Vec::new() } else { rec.connections(face, group) };
    if popup {
        // The r8 pop-up has no help panel: the page row's help under it described a row the pop-up does not show.
        frame.help[0].clear();
        frame.help[1].clear();
        // Its own column, right of the list: the header, then the row under the cursor and the files joined to it in
        // kind boxes, the headings over them, and plain lines.
        let items = rec.connection_items(face, group);
        if !items.is_empty() {
            let _ = frame.conn_head.write_str("CONNECTIONS  of the row under the cursor");
        }
        for (slot, item) in items.iter().take(super::panes::CONN_LINES).enumerate() {
            use super::recording_menu::ConnKind;
            let _ = frame.conn[slot].write_str(&item.text);
            frame.conn_kind[slot] = item.kind;
            frame.conn_tone[slot] = match item.kind {
                ConnKind::Heading | ConnKind::LinkSub => Tone::Dim,
                _ => Tone::Normal,
            };
            frame.conn_count = (slot + 1) as u8;
        }
    } else if !lines.is_empty() {
        frame.help[0].clear();
        frame.help[1].clear();
        let _ = frame.help[0].write_str("CONNECTIONS");
        let mut joined = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                joined.push_str(" | ");
            }
            if joined.len() + line.text.len() > 150 {
                break;
            }
            joined.push_str(&line.text);
        }
        let _ = frame.help[1].write_str(&joined);
    }
}

fn paint_crop(rec: &super::recording_menu::RecordingScreens, frame: &mut Frame) {
    let rows = rec.crop_rows();
    let cursor = rec.crop_cursor();
    for (slot, row) in rows.iter().enumerate().take(ROW_LINES) {
        let _ = frame.rows[slot].label.write_str(&row.title);
        let _ = frame.rows[slot].value.write_str(&row.value);
        frame.row_count += 1;
        if slot == cursor {
            frame.focus = Some(slot as u8);
        }
    }
    let _ = frame.crumb.write_str("Crop");
}

fn paint_file(rec: &super::recording_menu::RecordingScreens, frame: &mut Frame) {
    let rows = rec.file_rows();
    let cursor = rec.file_cursor();
    let start = window_start(cursor, rows.len(), ROW_LINES);
    for (offset, row) in rows.iter().enumerate().skip(start).take(ROW_LINES) {
        let slot = offset - start;
        let _ = frame.rows[slot].label.write_str(&row.title);
        let _ = frame.rows[slot].value.write_str(&row.value);
        frame.row_count += 1;
        if offset == cursor {
            frame.focus = Some(slot as u8);
        }
        // Before the start save's name: the value is right-aligned, so the chip sits left of its text.
        if let Some(floppy) = row.floppy {
            let x = super::panes::ROW_RIGHT - super::panes::text_width(row.value.encode_utf16().count()) - 22.0;
            place_icon(frame, 2, x, super::panes::row_y(slot), floppy);
        }
    }
    let _ = frame.crumb.write_str(&rec.file_crumb());
}

fn paint_picker(rec: &super::recording_menu::RecordingScreens, frame: &mut Frame) {
    let rows = rec.picker_rows();
    let cursor = rows.iter().position(|row| row.cursor).unwrap_or(0);
    let start = window_start(cursor, rows.len(), ROW_LINES);
    for (offset, row) in rows.iter().enumerate().skip(start).take(ROW_LINES) {
        let slot = offset - start;
        let _ = if row.rank == 0 {
            frame.rows[slot].label.write_str(&row.name)
        } else {
            write!(frame.rows[slot].label, "{} {}", row.rank, row.name)
        };
        frame.row_count += 1;
        if row.cursor {
            frame.focus = Some(slot as u8);
        }
    }
}

fn paint_sections(rec: &super::recording_menu::RecordingScreens, frame: &mut Frame, face: super::recording_menu::Face, group: u8) {
    use super::recording_menu::FileTag;
    let popup = face == super::recording_menu::Face::Popup;
    let mut badges = 0;
    for section in 0..2u8 {
        let view = rec.section(face, group, section);
        let base = usize::from(section) * (SECTION_ROWS + 1);
        if base >= ROW_LINES {
            break;
        }
        // The r8 header carries its file count ("SAVE STATES (7)").
        let _ = write!(frame.rows[base].label, "{}  ({})", view.header, view.files);
        frame.rows[base].label_tone = Tone::Dim;
        frame.row_count = (base + 1) as u8;
        // In the pop-up it sits on a dark band with its kind's mark before the name (r8): the floppy for SAVE STATES.
        if popup {
            frame.band[base] = true;
            frame.head_tag[usize::from(section)] = match view.header {
                "SAVE STATES" => Some(FileTag::Save),
                "MACROS" => Some(FileTag::Macro),
                "COMBOS" => Some(FileTag::Combo),
                "SEQUENCES" => Some(FileTag::Sequence),
                _ => None,
            };
        }
        if let Some(floppy) = view.header_floppy {
            let x = if popup { super::panes::HEAD_MARK_X } else { p_row_x(false) };
            place_icon(frame, usize::from(section), x, super::panes::row_y(base), floppy);
        }
        if let Some(mark) = view.header_loop {
            // Left of the right-aligned count, as a start save's floppy sits left of its name (`paint_file`).
            let right = super::panes::row_span(face == super::recording_menu::Face::Popup).1;
            let x = right - super::panes::text_width(mark.label().encode_utf16().count()) - 22.0;
            place_icon_color(frame, super::panes::FLOPPIES + usize::from(section), x, super::panes::row_y(base), [236, 238, 242, 255]);
            let _ = write!(frame.rows[base].value, "{}", mark.label());
        }
        let start = window_start(view.cursor, view.rows.len(), SECTION_ROWS);
        let mut hot_lo: Option<usize> = None;
        let mut hot_hi: Option<usize> = None;
        for (offset, row) in view.rows.iter().enumerate().skip(start).take(SECTION_ROWS) {
            let slot = base + 1 + (offset - start);
            if slot >= ROW_LINES {
                break;
            }
            match row.order {
                // The pop-up's red badge at the row's left edge (r8); the page keeps the number before the name.
                Some(order) if popup && badges < super::panes::BADGES => {
                    frame.badges[badges] = Some((slot as u8, order));
                    badges += 1;
                    let _ = frame.rows[slot].label.write_str(&row.name);
                }
                Some(order) => {
                    let _ = write!(frame.rows[slot].label, "{order} {}", row.name);
                }
                None => {
                    let _ = frame.rows[slot].label.write_str(&row.name);
                }
            }
            if let Some(count) = row.chain_count {
                let _ = write!(frame.rows[slot].value, "{count}");
            }
            frame.rows[slot].label_tone = Tone::Normal;
            let _ = frame.rows[slot].sub.write_str(&row.sub);
            frame.green[slot] = row.green;
            frame.chain[slot] = row.chain_count.is_some();
            if row.hot {
                hot_lo.get_or_insert(slot);
                hot_hi = Some(slot);
            }
            if view.hairline_after == Some(5) && offset + 1 == 5 {
                frame.hairline = Some(slot as u8);
            }
            frame.row_count = (slot + 1) as u8;
            if row.cursor {
                frame.focus = Some(slot as u8);
            }
            if let Some(floppy) = row.floppy {
                let icon = 2 + usize::from(section) * SECTION_ROWS + (offset - start);
                place_icon(frame, icon, p_row_x(face == super::recording_menu::Face::Popup), super::panes::row_y(slot), floppy);
            }
        }
        if let (Some(lo), Some(hi)) = (hot_lo, hot_hi) {
            frame.outline[usize::from(section)] = Some((lo as u8, (hi - lo + 1) as u8));
        }
    }
}

fn place_icon(frame: &mut Frame, index: usize, x: f32, y: f32, floppy: super::recording_menu::Floppy) {
    place_icon_color(frame, index, x, y, floppy.color());
}

fn place_icon_color(frame: &mut Frame, index: usize, x: f32, y: f32, color: [u8; 4]) {
    if let Some(slot) = frame.icons.get_mut(index) {
        *slot = MarkIcon { on: true, x, y, color };
    }
}

fn p_row_x(popup: bool) -> f32 {
    super::panes::row_span(popup).1 - 40.0
}

fn window_start(cursor: usize, len: usize, shown: usize) -> usize {
    if len <= shown {
        0
    } else {
        cursor.saturating_sub(shown / 2).min(len - shown)
    }
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
        // rc.6: the CPU page's seventh group is Level (CPU Level), after Player 1 control.
        assert_eq!(f.group_count, 7);
        assert_eq!(f.groups[0].string_lossy(), "Reactions  (4)");
        assert_eq!(f.groups[6].string_lossy(), "Level  (1)");
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
    fn recording_paint_uses_a_green_plate_a_white_cursor_and_the_visible_hot_rows() {
        use crate::model::menu::table::SAVES_MACROS;
        use crate::model::recording_menu::{Catalog, Face, LoopMark, MacroFile};
        let mut menu = menu();
        menu.press(Button::R);
        menu.press(Button::R);
        let mut macros = Vec::new();
        for index in 0..6 {
            macros.push(MacroFile {
                id: format!("m{index}"),
                name: format!("Macro {index}"),
                chain: false,
                steps: Vec::new(),
                start_save: None,
                start_save_on: true,
                recorded_on: String::new(),
                span: 4,
            });
        }
        macros.push(MacroFile {
            id: "ch".into(),
            name: "Chain".into(),
            chain: true,
            steps: vec!["m0".into(), "m1".into()],
            start_save: None,
            start_save_on: true,
            recorded_on: String::new(),
            span: 0,
        });
        menu.recording_mut().set_catalog(Catalog {
            macros,
            hot_macros: vec!["m0".into(), "m1".into(), "ch".into(), "m2".into(), "m3".into()],
            ..Catalog::default()
        });
        menu.recording_mut().adopt(
            None,
            [Some("m2".into()), None],
            [LoopMark::Finite(1), LoopMark::Finite(1)],
            None,
        );
        menu.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 1, 4);
        let mut frame = Frame::new();
        page_frame(&menu, &mut frame);
        assert!(frame.white_cursor);
        assert!(frame.hairline.is_some(), "the fifth hot file is in this window");
        assert!(frame.outline[1].is_some());
        assert!(frame.green.iter().any(|on| *on));
        assert!(frame.chain.iter().any(|on| *on));
        let green = frame.green.iter().position(|on| *on).unwrap();
        assert_eq!(frame.rows[green].label_tone, Tone::Normal);
        assert_eq!(frame.rows[green].label.string_lossy(), "Macro 2");
    }

    /// GIF evidence `macro-screen-1140.png` (2026-11-08): the macro's own screen drew "Macro 1 · start · On" and no
    /// floppy. RECORDING-REDESIGN puts the chip before the start save's name on that row.
    #[test]
    fn a_macro_screen_draws_the_floppy_before_the_start_save_name() {
        use crate::model::menu::table::SAVES_MACROS;
        use crate::model::recording_menu::{Catalog, Face, Floppy, MacroFile, SaveFile};
        let mut menu = menu();
        menu.press(Button::R);
        menu.press(Button::R);
        menu.recording_mut().set_catalog(Catalog {
            saves: vec![SaveFile { id: "m1-start".into(), name: "Macro 1 · start".into(), fighters: String::new() }],
            macros: vec![MacroFile {
                id: "m1".into(),
                name: "Jab".into(),
                chain: false,
                steps: Vec::new(),
                start_save: Some("m1-start".into()),
                start_save_on: true,
                recorded_on: "Mario".into(),
                span: 25,
            }],
            hot_saves: vec!["m1-start".into()],
            hot_macros: vec!["m1".into()],
            ..Catalog::default()
        });
        menu.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 1, 0);
        menu.press(Button::A);
        assert!(menu.recording().file_open(), "A opens the macro's own screen");
        let mut frame = Frame::new();
        page_frame(&menu, &mut frame);
        assert_eq!(frame.rows[0].label.string_lossy(), "Start save");
        let value = frame.rows[0].value.string_lossy();
        assert_eq!(value, "Macro 1 · start · On");
        let chip = frame.icons[2];
        assert!(chip.on, "the Start save row has its floppy");
        assert_eq!(chip.color, Floppy::On.color());
        assert_eq!(chip.y, super::super::panes::row_y(0));
        let text_left = super::super::panes::ROW_RIGHT - super::super::panes::text_width(value.encode_utf16().count());
        assert!(chip.x < text_left, "the chip sits before the name, not over it");
        assert!(frame.icons.iter().enumerate().all(|(i, icon)| i == 2 || !icon.on), "one chip on a file screen");
    }

    fn recording_menu_with_files() -> Menu {
        use crate::model::recording_menu::{Catalog, ComboFile, MacroFile, SaveFile, SequenceFile};
        let mut menu = menu();
        menu.press(Button::R);
        menu.press(Button::R);
        let save = SaveFile { id: "s1".into(), name: "Macro 1 · start".into(), fighters: String::new() };
        let jab = MacroFile {
            id: "m1".into(),
            name: "Jab".into(),
            chain: false,
            steps: Vec::new(),
            start_save: Some("s1".into()),
            start_save_on: true,
            recorded_on: "Mario".into(),
            span: 25,
        };
        let combo = |id: &str| ComboFile { id: id.into(), name: id.into(), recorded_on: String::new(), made_from: None, links: Vec::new(), span: 1 };
        let sequence = SequenceFile { id: "q1".into(), name: "Sequence 1".into(), combos: vec!["c1".into()], streak: 0, best: 0 };
        menu.recording_mut().set_catalog(Catalog {
            saves: vec![save],
            macros: vec![jab],
            combos: vec![combo("c1"), combo("c2")],
            sequences: vec![sequence],
            hot_saves: vec!["s1".into()],
            hot_macros: vec!["m1".into()],
            hot_combos: vec!["c1".into(), "c2".into()],
            hot_sequences: vec!["q1".into()],
        });
        menu
    }

    /// Primary 2026-11-08 (#19, #49): the r8 pop-up (`RECORDING-REDESIGN-r8-popup-combo.png`) has a line under each
    /// row and CONNECTIONS in its own column on the right. Eden rc6-covrec-diag2: the pop-up drew one line per row and
    /// joined CONNECTIONS into the help line at the bottom.
    #[test]
    fn the_popup_draws_sublines_and_a_connections_column() {
        use crate::model::menu::table::COMBOS_SEQ;
        use crate::model::recording_menu::Face;
        let mut menu = recording_menu_with_files();
        let mut catalog = menu.recording().catalog().clone();
        catalog.combos[0].recorded_on = "Pikachu → Mario".into();
        menu.recording_mut().set_catalog(catalog);
        menu.recording_mut().toggle_popup();
        menu.recording_mut().press_popup(Button::ZR);
        let combos = menu.recording().section(Face::Popup, COMBOS_SEQ, 1);
        let at = combos.rows.iter().position(|row| row.name == "c1").expect("c1 listed");
        menu.recording_mut().set_cursor(Face::Popup, COMBOS_SEQ, 1, at);
        let mut frame = Frame::new();
        page_frame(&menu, &mut frame);
        assert!(frame.popup);
        let slot = (0..usize::from(frame.row_count)).find(|i| frame.rows[*i].label.string_lossy() == "c1").expect("c1 drawn");
        assert_eq!(frame.rows[slot].sub.string_lossy(), "Recorded on Pikachu → Mario");
        let header = (0..usize::from(frame.row_count)).find(|i| frame.rows[*i].label.string_lossy().starts_with("COMBOS")).expect("header");
        assert!(frame.rows[header].sub.is_empty(), "a header has no subline");
        assert_eq!(frame.conn_head.string_lossy(), "CONNECTIONS  of the row under the cursor");
        let lines: Vec<String> = (0..usize::from(frame.conn_count)).map(|i| frame.conn[i].string_lossy()).collect();
        assert_eq!(lines.first().map(String::as_str), Some("c1"), "{lines:?}");
        assert!(lines.iter().any(|line| line == "Sequence 1 #1"), "{lines:?}");
        assert_eq!(lines.last().map(String::as_str), Some("Fighters  Pikachu → Mario"), "{lines:?}");
        assert!(!frame.help[0].string_lossy().contains("CONNECTIONS"), "the help keeps its own lines");
    }

    /// Primary 2026-11-08 15:55Z, the r8 pop-up as drawn (`RECORDING-REDESIGN-r8-popup-combo.png`): the title
    /// "Recording" with the L/R role tabs, each section header with its file count, CONNECTIONS as kind boxes in a tree,
    /// and the r8 footer.
    #[test]
    fn the_popup_has_the_r8_title_counts_boxes_and_footer() {
        use crate::model::menu::table::COMBOS_SEQ;
        use crate::model::recording_menu::{ConnKind, Face, FileTag};
        let mut menu = recording_menu_with_files();
        menu.recording_mut().toggle_popup();
        menu.recording_mut().press_popup(Button::ZR);
        menu.recording_mut().set_cursor(Face::Popup, COMBOS_SEQ, 1, 1);
        let mut frame = Frame::new();
        page_frame(&menu, &mut frame);
        assert_eq!(frame.crumb.string_lossy(), "Recording");
        assert_eq!(frame.role_tab, Some(0), "Player 1's tab is lit");
        let labels: Vec<String> = (0..usize::from(frame.row_count)).map(|i| frame.rows[i].label.string_lossy()).collect();
        assert!(labels.contains(&"SEQUENCES  (1)".to_string()) && labels.contains(&"COMBOS  (2)".to_string()), "{labels:?}");
        assert_eq!(frame.conn_kind[0], ConnKind::Head(FileTag::Combo));
        assert_eq!(frame.conn[0].string_lossy(), "c1");
        assert_eq!(frame.conn_kind[1], ConnKind::Heading);
        assert_eq!(frame.conn_kind[2], ConnKind::Child(FileTag::Sequence));
        assert!(frame.hint.string_lossy().starts_with("A drill   Left/Right section"));
        menu.recording_mut().press_popup(Button::R);
        let mut frame = Frame::new();
        page_frame(&menu, &mut frame);
        assert_eq!(frame.role_tab, Some(1), "R lights the CPU's tab");
        let mut page = Frame::new();
        menu.recording_mut().toggle_popup();
        page_frame(&menu, &mut page);
        assert_eq!(page.role_tab, None, "the page has no role tabs in its title");
    }

    /// Primary 2026-11-08 16:51Z, the r8 pop-up's remaining parts (`RECORDING-REDESIGN-r8-popup-combo.png`, `-macro.png`;
    /// RECORDING-REDESIGN.md "How to read a list"): each section header on a dark band with its kind's mark (the floppy
    /// for SAVE STATES, a square for the others), the red order badge at a row's left edge instead of a number before
    /// the name, and the page tabs as outlined boxes. A drilled sequence is a green row in the hot outline and wears the
    /// badge of the combo under the cursor. The page keeps its number before the name.
    #[test]
    fn the_popup_draws_header_bands_order_badges_and_boxed_tabs() {
        use crate::model::menu::table::COMBOS_SEQ;
        use crate::model::recording_menu::{Face, FileTag, LoopMark};
        let mut menu = recording_menu_with_files();
        menu.recording_mut().adopt(None, [None, None], [LoopMark::Finite(1), LoopMark::Finite(1)], Some("q1".into()));
        menu.recording_mut().toggle_popup();
        menu.recording_mut().press_popup(Button::ZR);
        let combos = menu.recording().section(Face::Popup, COMBOS_SEQ, 1);
        let at = combos.rows.iter().position(|row| row.name == "c1").expect("c1 listed");
        menu.recording_mut().set_cursor(Face::Popup, COMBOS_SEQ, 1, at);
        let mut frame = Frame::new();
        page_frame(&menu, &mut frame);
        assert_eq!(frame.band.iter().filter(|on| **on).count(), 2, "{:?}", frame.band);
        assert!(frame.band[0] && frame.band[SECTION_ROWS + 1], "both headers sit on a band");
        assert_eq!(frame.head_tag, [Some(FileTag::Sequence), Some(FileTag::Combo)]);
        let labels: Vec<String> = (0..usize::from(frame.row_count)).map(|i| frame.rows[i].label.string_lossy()).collect();
        let seq = labels.iter().position(|label| label.ends_with("Sequence 1")).expect("the sequence row");
        assert_eq!(labels[seq], "Sequence 1", "no number before the name in the pop-up");
        assert_eq!(frame.badges.iter().flatten().copied().collect::<Vec<_>>(), [(seq as u8, 1)], "c1 is its combo 1");
        assert!(frame.green[seq], "the drilled sequence is green");
        assert!(frame.outline[0].is_some_and(|(first, count)| usize::from(first) <= seq && seq < usize::from(first + count)));
        assert!(frame.tab_boxes);
        menu.recording_mut().press_popup(Button::ZL);
        let mut frame = Frame::new();
        page_frame(&menu, &mut frame);
        assert_eq!(frame.head_tag, [Some(FileTag::Save), Some(FileTag::Macro)]);
        menu.recording_mut().press_popup(Button::ZR);
        menu.recording_mut().toggle_popup();
        menu.recording_mut().set_cursor(Face::Page, COMBOS_SEQ, 1, at);
        let mut page = Frame::new();
        page_frame(&menu, &mut page);
        assert!(!page.tab_boxes && page.band.iter().all(|on| !on) && page.badges.iter().all(Option::is_none));
        assert!(page.head_tag.iter().all(Option::is_none));
    }

    /// Eden rc6-covrec-diag3 `cov-popup-r8` (2026-11-08): the MACROS header's loop count "1" was hidden under the loop
    /// icon; both sat at the column's right edge and the icon draws on top. Primary 13:05Z (#19): the loop count sits
    /// inside the panel, as in the r8 mockup. The icon goes left of the count, on the page and in the pop-up.
    #[test]
    fn the_loop_icon_sits_left_of_the_macros_loop_count() {
        use crate::model::menu::table::SAVES_MACROS;
        use crate::model::recording_menu::Face;
        for popup in [false, true] {
            let mut menu = recording_menu_with_files();
            menu.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 1, 0);
            if popup {
                menu.recording_mut().toggle_popup();
            }
            let mut frame = Frame::new();
            page_frame(&menu, &mut frame);
            let header = (0..usize::from(frame.row_count)).find(|i| frame.rows[*i].label.string_lossy().starts_with("MACROS")).expect("MACROS");
            let count = frame.rows[header].value.string_lossy();
            assert!(!count.is_empty(), "the header carries the loop count");
            let icon = frame.icons[super::super::panes::FLOPPIES + 1];
            assert!(icon.on, "the loop icon is drawn");
            let text_left = super::super::panes::row_span(popup).1 - super::super::panes::text_width(count.encode_utf16().count());
            assert!(icon.x + 14.0 <= text_left, "popup={popup}: the icon ({}) covers the count from {text_left}", icon.x);
        }
    }

    /// Eden `rc5-opus-floppy1` (2026-11-08): `Save states & Macros (0)` beside an open macro. The count was of settings
    /// rows, and a library group has none; it lists files.
    #[test]
    fn a_recording_library_group_counts_its_files() {
        let menu = recording_menu_with_files();
        let mut frame = Frame::new();
        page_frame(&menu, &mut frame);
        let groups: Vec<String> = (0..usize::from(frame.group_count)).map(|i| frame.groups[i].string_lossy()).collect();
        assert!(groups.contains(&"Save states & Macros  (2)".to_string()), "{groups:?}");
        assert!(groups.contains(&"Combos & Sequences  (3)".to_string()), "{groups:?}");
        let options = groups.iter().find(|line| line.starts_with("Options")).expect("Options group");
        assert_ne!(options, "Options  (0)", "settings groups still count their rows");
    }

    /// The same shot's footer read `B closeUp/Down move`: the recording hint was appended to the page hint.
    #[test]
    fn the_recording_hint_replaces_the_page_hint() {
        use crate::model::menu::table::SAVES_MACROS;
        use crate::model::recording_menu::Face;
        let mut menu = recording_menu_with_files();
        let mut frame = Frame::new();
        page_frame(&menu, &mut frame);
        assert_eq!(frame.hint.string_lossy(), menu.recording().hint(Face::Page));
        menu.recording_mut().set_cursor(Face::Page, SAVES_MACROS, 1, 0);
        menu.press(Button::A);
        assert!(menu.recording().file_open());
        let mut frame = Frame::new();
        page_frame(&menu, &mut frame);
        assert_eq!(frame.hint.string_lossy(), "Up/Down move   Left/Right On/Off   A do it   B back");
        let mut other = Menu::new(Settings::factory(), Env::default());
        other.press(Button::Down);
        let mut frame = Frame::new();
        page_frame(&other, &mut frame);
        assert_eq!(frame.hint.string_lossy(), HINT_PAGE, "pages without the recording screens keep the page hint");
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

    /// R13: the crate version, `rc8` and the commit's short sha (`git rev-parse --short=8`: 8 hex digits, more only if
    /// 8 are ambiguous), `-dirty` with uncommitted changes, or `unknown` without git.
    #[test]
    fn the_build_tag_reads_version_name_and_short_sha() {
        let rest = BUILD_TAG.strip_prefix(env!("CARGO_PKG_VERSION")).expect(BUILD_TAG);
        let sha = rest.strip_prefix(" · rc8 ").expect(BUILD_TAG);
        let hex = sha.strip_suffix("-dirty").unwrap_or(sha);
        assert!(sha == "unknown" || (hex.len() >= 8 && hex.bytes().all(|b| b.is_ascii_hexdigit())), "{BUILD_TAG}");
    }

    /// R13: the tag ends the page's key hints, after them, on the page and on the Recording page; the pop-up and a
    /// sub-screen keep their own keys, and a menu without a tag keeps the hint as it was.
    #[test]
    fn the_page_footer_ends_with_the_build_tag() {
        use crate::model::recording_menu::Face;
        let mut m = Menu::new(Settings::factory(), Env { build_tag: BUILD_TAG, ..Env::default() });
        let mut f = Frame::new();
        page_frame(&m, &mut f);
        assert_eq!(f.hint.string_lossy(), format!("{HINT_PAGE}      {BUILD_TAG}"));
        assert!(!f.help[0].is_empty(), "the help lines are kept");
        m.press(Button::R);
        m.press(Button::R);
        page_frame(&m, &mut f);
        assert_eq!(f.hint.string_lossy(), format!("{}      {BUILD_TAG}", m.recording().hint(Face::Page)));
        m.recording_mut().toggle_popup();
        page_frame(&m, &mut f);
        assert_eq!(f.hint.string_lossy(), m.recording().hint(Face::Popup), "the pop-up keeps its r8 footer");
        let mut c = Menu::new(Settings::factory(), Env { build_tag: BUILD_TAG, ..Env::default() });
        c.press(Button::Up);
        c.press(Button::A);
        crate::model::screens::frame_of(&c, &mut f, |_| "Stage");
        assert_eq!(f.hint.string_lossy(), "A reset   B cancel", "a sub-screen keeps its own keys");
        page_frame(&menu(), &mut f);
        assert_eq!(f.hint.string_lossy(), HINT_PAGE);
    }

    /// R13: with the longest tag a build makes (`-dirty`), every page's hint line fits its pane (`panes::spec`): the tag
    /// is not cut at the pane's capacity, and the line's width at the pane's font (`text_width`'s 0.56 em) stays inside
    /// the pane.
    #[test]
    fn the_build_tag_fits_every_page_footer() {
        use super::super::panes::{spec, Kind, IDX_HINT};
        const LONGEST: &str = concat!(env!("CARGO_PKG_VERSION"), " · ", env!("TR_BUILD_NAME"), " 0123abcd-dirty");
        let pane = spec(IDX_HINT);
        let Kind::Text { font, chars, .. } = pane.kind else { panic!("the hint pane is a text box") };
        for tag in [BUILD_TAG, LONGEST] {
            for developer in [false, true] {
                let mut m = Menu::new(Settings::factory(), Env { developer_page: developer, build_tag: tag, ..Env::default() });
                for _ in 0..8 {
                    let page = m.page();
                    for _ in 0..m.entry_count(page) + 1 {
                        let mut f = Frame::new();
                        page_frame(&m, &mut f);
                        let hint = f.hint.string_lossy();
                        assert!(hint.ends_with(tag), "{page:?}: {hint}");
                        let units = f.hint.units().len();
                        assert!(units <= usize::from(chars), "{page:?}: {units} units");
                        assert!(units as f32 * font.0 * 0.56 <= pane.w, "{page:?}: {hint}");
                        m.press(Button::Down);
                    }
                    m.press(Button::R);
                }
            }
        }
    }
}
