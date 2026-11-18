//! The menu as one frame of text: everything the renderer writes into the layout's panes, built from the
//! menu model (`model::menu`) with no allocation in the frame itself (fixed UTF-16 buffers), so the
//! renderer's tick only copies. SPEC-core-03 §2.2 (page, footer, key hints), §2.5 (two-line help panel).
//!
//! The model decides what is on screen and where the cursor is; this module only says how it reads.

use core::fmt::{self, Write};
use super::recording_menu::{Face, Floppy, RecordingScreens};

use super::menu::table::{Control, Editor, Info, GROUPS, ROWS};
use super::menu::{group_of, Entry, Grey, Menu, RowValue, LOOP_ROW};
use super::panes::{text_width, GROUPS as GROUP_LINES, ROWS as ROW_LINES, TABS};

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
    /// A dialog (a confirmation, the Fighter Mismatch question, a panel): drawn in a box sized to it (`dialog_box`).
    pub dialog: bool,
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
        self.dialog = false;
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
            dialog: false,
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

impl Frame {
    /// A dialog's box (2026-11-17: every dialog takes only the space it needs), for a dialog (`dialog`) or the recording
    /// pop-up (`popup`): the background `[x, y, w, h]` around the crumb, the content and the help and hint lines, and the
    /// help lines' and hint's y, which follow the content instead of the page's foot. `None` for a page (the full panel).
    pub fn dialog_box(&self) -> Option<([f32; 4], [f32; 2], f32)> {
        use super::notice_box::measure;
        use super::panes as p;
        const LEFT: f32 = -880.0;
        const PAD: f32 = 24.0;
        (self.dialog || self.popup || self.view == View::Delay).then_some(())?;
        // The menu fonts render wider than the notice-calibrated estimate: ~6 % for prose, ~14 % for a 48-W crumb (Eden,
        // notice-compact-72451433 and -9f90e741), so a dialog measures its text with a 1.2 margin.
        let width = |text: &[u16], font: (f32, f32)| 1.2 * measure(text.iter().copied(), font.0).0;
        let (mut right, mut bottom) = if self.view == View::Tiles {
            let (cols, rows) = (usize::from(self.tile_cols.max(1)), usize::from(self.tile_rows.max(1)));
            // A panel's page carries all its cells; the empty ones draw nothing, so the box ends at the last label.
            let count = self.tiles[..usize::from(self.tile_count)].iter().rposition(|t| !t.units().is_empty()).map_or(0, |i| i + 1);
            let cell_w = (p::ROW_RIGHT - p::ROW_LEFT) / cols as f32;
            (p::ROW_LEFT + count.min(cols) as f32 * cell_w, 320.0 - count.div_ceil(cols) as f32 * p::TILE_AREA_H / rows as f32)
        } else if self.view == View::Delay {
            // The delay pop-up: eight tile columns across the rows and the range bar, its handle labels lowest (y -165,
            // the view's DELAY_HANDLE_LABEL_Y).
            (p::ROW_RIGHT, -165.0 - p::ROW_H / 2.0)
        } else {
            let (x, w) = p::row_label_box(self.popup);
            let rows = &self.rows[..usize::from(self.row_count)];
            let widest = rows.iter().map(|r| width(r.label.units(), p::FONT)).fold(0.0, f32::max);
            // A row's value is right-aligned in the value column (Chord Entry's chord), so a valued row spans to it.
            let valued = rows.iter().any(|r| !r.value.units().is_empty());
            (if valued { p::row_value_right(self.popup) } else { x - w / 2.0 + widest }, p::row_y(rows.len().max(1) - 1) - p::ROW_H / 2.0)
        };
        // Every pop-up draws its role tabs and L/R key caps (x ≤ 836), the row bands and the CONNECTIONS divider (review 2);
        // its connection lines end above the divider's foot and left of CONN_RIGHT, so the box spans to both.
        let div = p::spec(p::IDX_CONN_DIV);
        if self.popup {
            (right, bottom) = (right.max(p::CONN_RIGHT), bottom.min(div.y - div.h / 2.0));
        }
        right = right.max(LEFT + width(self.crumb.units(), p::FONT));
        let (mut y, mut help_y) = (bottom - 8.0, p::HELP_Y);
        for (at, line) in help_y.iter_mut().zip(&self.help).filter(|(_, line)| !line.units().is_empty()) {
            (y, *at) = (y - 48.0, y - 48.0);
            right = right.max(LEFT + width(line.units(), p::FONT_SMALL));
        }
        let hint_y = y - 52.0;
        right = right.max(LEFT + width(self.hint.units(), p::FONT_SMALL)) + PAD;
        let top = if self.popup { p::TAB_Y } else { p::CRUMB_Y } + 28.0 + PAD;
        let (left, bottom) = (LEFT - PAD, hint_y - 22.0 - PAD);
        Some(([(left + right) / 2.0, (top + bottom) / 2.0, right - left, top - bottom], help_y, hint_y))
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
    if r.id == LOOP_ROW {
        let _ = write!(out, "<  {}  >", menu.recording().loop_label(menu.recording().page_role()));
        return;
    }
    let cycles = matches!(r.control, Control::Dropdown(_) | Control::Number { .. } | Control::YesNoRandom);
    if cycles {
        let _ = out.write_str("<  ");
    }
    match menu.value(row) {
        RowValue::Toggle(on) => _ = out.write_str(if on { "[ON]" } else { "[OFF]" }),
        RowValue::Choice(at) => {
            let label = match (r.control, at) {
                (Control::Dropdown(_), Some(i)) => {
                    let (name, qualifier) = menu.dropdown_label(row, i).unwrap_or(("-", ""));
                    let _ = out.write_str(name);
                    if !qualifier.is_empty() { let _ = write!(out, " · {qualifier}"); }
                    ""
                },
                (Control::YesNoRandom, Some(i)) => super::menu::options::YES_NO_RANDOM.get(i).copied().unwrap_or("-"),
                _ => "-",
            };
            let _ = out.write_str(label);
        }
        RowValue::Number(Some(n)) => _ = write!(out, "{n}"),
        RowValue::Number(None) => _ = out.write_str("-"),
        RowValue::Checklist { ticked, total } => _ = write!(out, "{ticked} of {total}  >"),
        RowValue::Delay(view) => {
            use super::menu::DelayView;
            let _ = match view {
                DelayView::Empty => out.write_str("none  >"),
                DelayView::Ticked(n) => write!(out, "{n} selected  >"),
                DelayView::Range(range) => write!(out, "{} - {}  >", range.min, range.max),
            };
        }
        RowValue::Slider { lower, upper } => {
            if let Some(spec) = super::tech_settings::scale(r.id) {
                let _ = spec.write(lower, out);
                let _ = out.write_str(" - ");
                let _ = spec.write(upper, out);
                let _ = out.write_str("  >");
            } else { let _ = write!(out, "{lower} - {upper}  >"); }
        }
        RowValue::Chips(mask) => _ = write!(out, "{} selected  >", mask.count_ones()),
        RowValue::Chord(chord) => _ = write!(out, "{chord}  >"),
        RowValue::OverridesSet(n) => _ = write!(out, "{n} of 13 set  >"),
        RowValue::None => {
            let _ = match r.control {
                Control::Info(Info::PermanentShortcut) => out.write_str("B + D-pad Up"),
                Control::Info(Info::PilotControl) => out.write_str("CPU pilot plugin"),
                Control::Editor(Editor::ComboTarget) => named(out, menu.env().combo_target.as_str()),
                Control::Editor(Editor::ComboName) => named(out, menu.env().combo_draft.as_str()),
                Control::Game(game) | Control::Start(game) if menu.running() == Some(game) => out.write_str("Running  >"),
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
    match (menu.value(row), ROWS[row].control) {
        (RowValue::Toggle(true), _) => Tone::On,
        (RowValue::Toggle(false), _) => Tone::Off,
        // rc.15 practice menu: "Running" in green.
        (_, Control::Game(game) | Control::Start(game)) if menu.running() == Some(game) => Tone::On,
        _ => Tone::Normal,
    }
}

/// A row's label: its title, except that a game's Start row reads Stop while that game runs.
pub fn row_title(menu: &Menu, row: usize) -> &'static str {
    match ROWS[row].control {
        Control::Start(game) if menu.running() == Some(game) => "Stop",
        _ => ROWS[row].title,
    }
}

/// The two help lines of a row: its own, the running game's for a game's rows while it runs, and a greyed row's reason
/// on the second line.
fn write_help(menu: &Menu, row: usize, help: &mut [Text<160>; 2]) {
    help.iter_mut().for_each(Text::clear);
    let r = &ROWS[row];
    let _ = match r.control {
        Control::Game(game) if menu.running() == Some(game) => {
            write!(help[0], "{} is running. A opens it to stop it or change it.", game.title())
                .and_then(|()| help[1].write_str("Closing the menu (B) leaves it running."))
        }
        Control::Start(game) if menu.running() == Some(game) => {
            write!(help[0], "{} is running. A stops it, and the row reads Start.", game.title())
                .and_then(|()| help[1].write_str("Changed settings apply at the next Start. B leaves it running."))
        }
        _ => {
            let (one, two) = r.help_lines();
            help[0].write_str(one).and_then(|()| help[1].write_str(two))
        }
    };
    if let Some(grey) = menu.grey(row) {
        help[1].clear();
        let _ = help[1].write_str(Grey::text(grey));
    }
}

/// One row of a list of rows, as the page and a game's sub-screen draw it.
fn row_line(menu: &Menu, row: usize, line: &mut RowLine) {
    let _ = line.label.write_str(row_title(menu, row));
    if ROWS[row].advanced {
        let _ = line.label.write_str("  (advanced)");
    }
    write_value(menu, row, &mut line.value);
    line.value_tone = value_tone(menu, row);
    line.label_tone = if menu.grey(row).is_some() { Tone::Off } else { Tone::Normal };
}

/// The rows of a game's sub-screen (`Screen::Rows`): the focused one is `cursor`. The help panel is that row's.
pub(crate) fn rows_lines(menu: &Menu, group: u8, cursor: usize, frame: &mut Frame) {
    let total = menu.sub_rows(group).count();
    let first = super::screens::window_start(cursor, total);
    for (slot, row) in menu.sub_rows(group).skip(first).take(ROW_LINES).enumerate() {
        row_line(menu, row, &mut frame.rows[slot]);
        frame.row_count += 1;
    }
    frame.focus = Some((cursor.saturating_sub(first)) as u8);
    if let Some(row) = menu.sub_rows(group).nth(cursor) {
        write_help(menu, row, &mut frame.help);
    }
}

/// One tab of the tab row as the renderer places it: where its text starts (layout units), its estimated text width, and
/// whether it is the active one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabSlot {
    pub left: f32,
    pub width: f32,
    pub active: bool,
}

impl Frame {
    /// The tab row: a slot for each tab the frame names. A frame that names none (the clip editor is drawn alone, with no
    /// menu chrome) has none, so the renderer draws no label and no active tab's box.
    pub fn tab_slots(&self) -> [Option<TabSlot>; TABS] {
        let mut slots = [None; TABS];
        let mut left = -840.0;
        for (i, slot) in slots.iter_mut().enumerate().take(usize::from(self.tab_count)) {
            let width = text_width(self.tabs[i].units().len());
            *slot = Some(TabSlot { left, width, active: i == usize::from(self.tab_active) });
            left += width + if self.tab_boxes { 84.0 } else { 64.0 };
        }
        slots
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
            Some(Entry::Row(r)) => row_line(menu, *r, line),
            Some(Entry::ResetPage(_)) => {
                let _ = line.label.write_str(RESET_LABEL);
                let _ = line.value.write_str(">");
            }
            None => {}
        }
        frame.row_count += 1;
    }
    frame.focus = focus_line.map(|f| (f - first) as u8);

    match menu.focused_row() {
        Some(row) => write_help(menu, row, &mut frame.help),
        None if matches!(menu.focus(), Some(Entry::ResetPage(_))) => {
            let _ = frame.help[0].write_str("Puts every setting of this page back to your saved defaults (X saves them).");
            let _ = frame.help[1].write_str("A asks first. On Fighters & Stages, both fighter picks reset and load on close.");
        }
        None => _ = frame.help[0].write_str("A opens or closes this section."),
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

fn paint_file(rec: &RecordingScreens, frame: &mut Frame) {
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
            let x = super::panes::row_value_right(false) - super::panes::text_width(row.value.encode_utf16().count()) - 22.0;
            place_icon(frame, 2, x, super::panes::row_y(slot), floppy);
        }
    }
    let _ = frame.crumb.write_str(&rec.file_crumb());
}

fn paint_picker(rec: &RecordingScreens, frame: &mut Frame) {
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

fn paint_sections(rec: &RecordingScreens, frame: &mut Frame, face: Face, group: u8) {
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
                "DRILLS" => Some(FileTag::Drill),
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
            let right = super::panes::row_value_right(face == super::recording_menu::Face::Popup);
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
                Some(order) => _ = write!(frame.rows[slot].label, "{order} {}", row.name),
                None => _ = frame.rows[slot].label.write_str(&row.name),
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

fn place_icon(frame: &mut Frame, index: usize, x: f32, y: f32, floppy: Floppy) {
    place_icon_color(frame, index, x, y, floppy.color());
}

fn place_icon_color(frame: &mut Frame, index: usize, x: f32, y: f32, color: [u8; 4]) {
    if let Some(slot) = frame.icons.get_mut(index) {
        *slot = MarkIcon { on: true, x, y, color };
    }
}

fn p_row_x(popup: bool) -> f32 {
    super::panes::row_value_right(popup) - 40.0
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
    lib_unit_tests! {
    use super::*;
    use crate::model::menu::input::Button;
    use crate::model::menu::Env;
    use crate::model::settings::Settings;

    fn menu() -> Menu {
        Menu::new(Settings::factory(), Env::default())
    }

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
        let text_left = super::super::panes::row_value_right(false) - super::super::panes::text_width(value.encode_utf16().count());
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
        let combo = |id: &str| ComboFile { input_only: false, id: id.into(), name: id.into(), recorded_on: String::new(), made_from: None, links: Vec::new(), span: 1 };
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

    /// Primary 2026-11-08 15:55Z, the r8 pop-up as drawn (`RECORDING-REDESIGN-r8-popup-combo.png`): the title
    /// "Recording" with the L/R role tabs, each section header with its file count, CONNECTIONS as kind boxes in a tree,
    /// and the r8 footer.
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

    fn the_fade_is_logarithmic_from_255_to_0_over_15_frames() {
        assert_eq!(fade_alpha(0), 255);
        assert_eq!(fade_alpha(15), 0);
        assert_eq!(fade_alpha(99), 0);
        let curve: Vec<u8> = (1..15).map(fade_alpha).collect();
        assert!(curve.windows(2).all(|w| w[0] >= w[1]), "{curve:?}");
        assert!(curve[0] > 240 && curve[13] < 70);
    }

    /// R13: the crate version, the build name and the commit's short sha (`git rev-parse --short=8`: 8 hex digits, more only if
    /// 8 are ambiguous), `-dirty` with uncommitted changes, or `unknown` without git.
    fn the_build_tag_reads_version_name_and_short_sha() {
        let rest = BUILD_TAG.strip_prefix(env!("CARGO_PKG_VERSION")).expect(BUILD_TAG);
        let sha = rest.strip_prefix(" · rc15 ").expect(BUILD_TAG);
        let hex = sha.strip_suffix("-dirty").unwrap_or(sha);
        assert!(sha == "unknown" || (hex.len() >= 8 && hex.bytes().all(|b| b.is_ascii_hexdigit())), "{BUILD_TAG}");
    }

    /// R13: the tag ends the page's key hints, after them, on the page and on the Recording page; the pop-up and a
    /// sub-screen keep their own keys, and a menu without a tag keeps the hint as it was.
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

    /// rc.14 recfix (Eden rc.13 still `p2-replay-editor`): the renderer draws the active tab's box from the frame's tab
    /// slots, one slot per tab the frame names and exactly one active, the pop-up's box tabs spaced wider than the page's.
    fn the_tab_row_has_a_slot_per_named_tab_and_one_active() {
        let mut f = Frame::new();
        page_frame(&menu(), &mut f);
        let (slots, named) = (f.tab_slots(), usize::from(f.tab_count));
        assert!(named >= 5);
        assert!(slots[..named].iter().all(Option::is_some) && slots[named..].iter().all(Option::is_none));
        assert_eq!(slots.iter().flatten().filter(|slot| slot.active).count(), 1);
        let (a, b) = (slots[0].unwrap(), slots[1].unwrap());
        assert_eq!(b.left, a.left + a.width + 64.0);
        f.tab_boxes = true;
        assert_eq!(f.tab_slots()[1].unwrap().left, a.left + a.width + 84.0);
    }

    /// rc.14 list-padding (GIF review of `8199c696`, Saved layouts): the cursor's left bar and the outline's edges ran
    /// through the first letter of every row, and "3 targets" touched the outline's right edge. Every list places its
    /// text through `row_label_box` and `row_value_box`; on both faces the text is at least `GAP` layout units (pixels of
    /// the 1920 x 1080 layout) from the innermost chrome edge on each side, and the two boxes do not overlap.
    fn list_text_keeps_a_gap_to_the_outline_and_the_cursor_on_both_sides() {
        use super::super::panes::{row_label_box, row_span, row_value_box, CHROME_EDGE, CURSOR_BAR_W, CURSOR_BAR_X};
        const GAP: f32 = 12.0;
        for popup in [false, true] {
            let (left, right) = row_span(popup);
            let ((lx, lw), (vx, vw)) = (row_label_box(popup), row_value_box(popup));
            let (label_left, label_right) = (lx - lw / 2.0, lx + lw / 2.0);
            let (value_left, value_right) = (vx - vw / 2.0, vx + vw / 2.0);
            let chrome_left = left + CHROME_EDGE.max(CURSOR_BAR_X + CURSOR_BAR_W / 2.0);
            let chrome_right = right - CHROME_EDGE;
            assert!(label_left - chrome_left >= GAP, "popup={popup}: the label starts {} units from the cursor's bar", label_left - chrome_left);
            assert!(chrome_right - value_right >= GAP, "popup={popup}: the value ends {} units from the outline", chrome_right - value_right);
            assert!(label_right < value_left, "popup={popup}: the label's box runs into the value's");
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("list_text_keeps_a_gap_to_the_outline_and_the_cursor_on_both_sides", list_text_keeps_a_gap_to_the_outline_and_the_cursor_on_both_sides as fn()),
            ("the_tab_row_has_a_slot_per_named_tab_and_one_active", the_tab_row_has_a_slot_per_named_tab_and_one_active as fn()),
            ("recording_paint_uses_a_green_plate_a_white_cursor_and_the_visible_hot_rows", recording_paint_uses_a_green_plate_a_white_cursor_and_the_visible_hot_rows as fn()),
            ("a_macro_screen_draws_the_floppy_before_the_start_save_name", a_macro_screen_draws_the_floppy_before_the_start_save_name as fn()),
            ("the_popup_has_the_r8_title_counts_boxes_and_footer", the_popup_has_the_r8_title_counts_boxes_and_footer as fn()),
            ("the_fade_is_logarithmic_from_255_to_0_over_15_frames", the_fade_is_logarithmic_from_255_to_0_over_15_frames as fn()),
            ("the_build_tag_reads_version_name_and_short_sha", the_build_tag_reads_version_name_and_short_sha as fn()),
            ("the_page_footer_ends_with_the_build_tag", the_page_footer_ends_with_the_build_tag as fn()),
            ("the_build_tag_fits_every_page_footer", the_build_tag_fits_every_page_footer as fn()),
        ]);
    }
    }
}
