//! The pane contract of the plugin's own menu layout (SPEC-core-03 §4.2, SPEC-core-07 §6 "keep one table in
//! the model crate"): every pane the layout archive carries for the menu, by name, with its place and size.
//! The host builder (`tools/layout-builder`) builds the BFLYT from this table; the runtime finds its panes by
//! the same names and writes only text, colours, visibility and positions into them. Nothing here is
//! upstream's: the layout is flat (one container, no parts), with text boxes, rectangles and the
//! approved owned-game item pictures. Stage and fighter portraits are loaded at runtime.
//!
//! Units are the native canvas, 1920 x 1080, origin at the centre, y up (it maps to 1280 x 720).

use core::fmt;

/// A pane name (at most 24 bytes: the BFLYT limit).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Name {
    bytes: [u8; 24],
    len: u8,
}

impl Name {
    pub const fn new(text: &str) -> Name {
        let src = text.as_bytes();
        assert!(src.len() <= 24, "pane names are at most 24 bytes");
        let mut bytes = [0u8; 24];
        let mut i = 0;
        while i < src.len() {
            bytes[i] = src[i];
            i += 1;
        }
        Name { bytes, len: src.len() as u8 }
    }

    /// `prefix` followed by `index` as two decimal digits.
    pub const fn indexed(prefix: &str, index: usize) -> Name {
        assert!(index < 100);
        let p = prefix.as_bytes();
        assert!(p.len() + 2 <= 24);
        let mut bytes = [0u8; 24];
        let mut i = 0;
        while i < p.len() {
            bytes[i] = p[i];
            i += 1;
        }
        bytes[i] = b'0' + (index / 10) as u8;
        bytes[i + 1] = b'0' + (index % 10) as u8;
        Name { bytes, len: (p.len() + 2) as u8 }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).unwrap_or("")
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The layout the menu lives in, and the container the plugin adds under its root pane.
pub const LAYOUT_NAME: &str = "info_training";
pub const ROOT: Name = Name::new("TrModMenu");

/// Text alignment inside a text box (the box's own edge the text starts from).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// A container with no picture or text: visibility and position only.
    Null,
    /// A flat rectangle: the stock solid texture tinted by its vertex colours.
    Rect { color: [u8; 4] },
    /// A text box in the game's `nintendo64` font; `chars` is the buffer's capacity in UTF-16 units.
    Text { align: Align, font: (f32, f32), chars: u16, color: [u8; 4] },
}

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub name: Name,
    /// The pane it is a child of: `RootPane`, or a container of this table.
    pub parent: Name,
    pub kind: Kind,
    /// Centre of the pane, relative to the container.
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Starts visible (the container itself starts hidden).
    pub visible: bool,
}

// ---- geometry ------------------------------------------------------------------------------------------

pub const TABS: usize = 6;
pub const GROUPS: usize = 8;
/// Rows visible at once (the largest sub-screen list scrolls inside them).
pub const ROWS: usize = 12;

pub const PANEL_W: f32 = 1840.0;
pub const PANEL_H: f32 = 1000.0;
/// The recording pop-up's panel over the match. The r8 mockup (`docs/decisions/RECORDING-REDESIGN-r8-popup-*.png` on
/// `review/menu-proposal`) draws its tabs, rows, loop counts, highlight, help and hint inside the panel. The pop-up
/// lays those out on the page's geometry, so its panel is the page's.
pub const POPUP_W: f32 = PANEL_W;
pub const POPUP_H: f32 = PANEL_H;

pub const TAB_Y: f32 = 440.0;
pub const TAB_H: f32 = 64.0;
pub const TAB_BOX_W: f32 = 520.0;
pub const CRUMB_Y: f32 = 372.0;

pub const GROUP_X: f32 = -700.0;
pub const GROUP_W: f32 = 400.0;
pub const GROUP_ROW_H: f32 = 56.0;
pub const DIVIDER_X: f32 = -480.0;

pub const ROW_LEFT: f32 = -440.0;
pub const ROW_RIGHT: f32 = 880.0;
pub const ROW_LABEL_W: f32 = 760.0;
pub const ROW_VALUE_W: f32 = 540.0;
pub const ROW_H: f32 = 52.0;
pub const ROW_PITCH: f32 = 54.0;
pub const ROW_FIRST_Y: f32 = 310.0;

pub const HELP_X: f32 = 0.0;
pub const HELP_W: f32 = 1760.0;
pub const HELP_Y: [f32; 2] = [-372.0, -420.0];
pub const HINT_Y: f32 = -472.0;

/// The right-hand value column sits this much lower than its label (the game's font draws right-aligned text higher).
pub const VALUE_DROP: f32 = 8.0;
pub const FONT: (f32, f32) = (22.0, 34.0);
pub const FONT_SMALL: (f32, f32) = (18.0, 28.0);

/// An estimate of a text's width in layout units (the game's font is proportional; a pane is only
/// positioned from it, never clipped by it).
pub fn text_width(units: usize) -> f32 {
    units as f32 * FONT.0 * 0.56
}

pub const fn row_y(row: usize) -> f32 {
    ROW_FIRST_Y - ROW_PITCH * row as f32
}

pub const fn group_y(group: usize) -> f32 {
    ROW_FIRST_Y - GROUP_ROW_H * group as f32
}

// ---- the r8 recording pop-up's two columns -------------------------------------------------------------
//
// The mockup puts the lists on the left and CONNECTIONS on the right of one panel. The pop-up has no group column,
// so its list starts where the groups would be and stops short of the middle.

/// The pop-up's list column (rows, plates, outlines, cursor, marks).
pub const POP_ROW_LEFT: f32 = -860.0;
pub const POP_ROW_RIGHT: f32 = 60.0;
/// The CONNECTIONS column and the rule between the two.
pub const CONN_DIVIDER_X: f32 = 110.0;
pub const CONN_LEFT: f32 = 150.0;
pub const CONN_RIGHT: f32 = 880.0;
pub const CONN_PITCH: f32 = 42.0;
pub const CONN_LINE_H: f32 = 40.0;
/// A row with a line under it: the name rises and the line sits under it, both inside the row's plate.
pub const SUB_RISE: f32 = 11.0;
pub const SUB_DROP: f32 = 13.0;
pub const SUB_H: f32 = 26.0;
pub const FONT_SUBLINE: (f32, f32) = (15.0, 24.0);

/// The list column's left and right edge: the page's, or the pop-up's left column.
pub const fn row_span(popup: bool) -> (f32, f32) {
    if popup { (POP_ROW_LEFT, POP_ROW_RIGHT) } else { (ROW_LEFT, ROW_RIGHT) }
}

/// A row's label width in the column: the page's, or what the pop-up's narrower column leaves beside a short value.
pub const fn row_label_w(popup: bool) -> f32 {
    if popup { POP_ROW_RIGHT - POP_ROW_LEFT - POP_LABEL_INSET - POP_VALUE_W - 16.0 } else { ROW_LABEL_W }
}

pub const fn row_value_w(popup: bool) -> f32 {
    if popup { POP_VALUE_W } else { ROW_VALUE_W }
}

/// Where a row's name starts in its column: the page's at its left edge; the pop-up's after the order badge's room.
pub const fn row_label_left(popup: bool) -> f32 {
    if popup { POP_ROW_LEFT + POP_LABEL_INSET } else { ROW_LEFT }
}

pub const POP_LABEL_INSET: f32 = 24.0;
/// The pop-up's red order badges (RECORDING-REDESIGN.md "Red number, at the row's left edge"), one per list row of the
/// two sections (four rows each).
pub const BADGES: usize = 8;
pub const BADGE_W: f32 = 22.0;
pub const BADGE_X: f32 = POP_ROW_LEFT + 12.0;
/// A pop-up section header's name moves right of its kind's mark (the floppy, or a square).
pub const HEAD_INSET: f32 = 20.0;
pub const HEAD_MARK_X: f32 = POP_ROW_LEFT + 18.0;
pub const HEAD_SQ: f32 = 16.0;
/// The r8 kind colours: the CONNECTIONS pills and the section headers' squares.
pub const KIND_SAVE: [u8; 4] = [240, 150, 60, 255];
pub const KIND_MACRO: [u8; 4] = [92, 150, 240, 255];
pub const KIND_COMBO: [u8; 4] = [170, 120, 240, 255];
pub const KIND_SEQUENCE: [u8; 4] = [235, 105, 170, 255];
/// The pop-up's header band, and the fill of its active page tab.
pub const BAND: [u8; 4] = [30, 33, 40, 255];
pub const POP_VALUE_W: f32 = 200.0;

/// Centre of CONNECTIONS line `line`; the header sits on the first row's line.
pub const fn conn_y(line: usize) -> f32 {
    ROW_FIRST_Y - CONN_PITCH * (line + 1) as f32
}

/// A joined file's box, a heading and the tree's ticks start this far right of the head box.
pub const CONN_INDENT: f32 = 50.0;
/// The tree's stem, between the column's edge and a joined file's box.
pub const CONN_STEM_X: f32 = CONN_LEFT + 24.0;
pub const CONN_BOX_H: f32 = CONN_LINE_H - 4.0;
pub const PILL_H: f32 = 24.0;
pub const FONT_PILL: (f32, f32) = (14.0, 22.0);

/// A pill's width for a label of `units` UTF-16 units.
pub fn pill_w(units: usize) -> f32 {
    units as f32 * FONT_PILL.0 * 0.62 + 20.0
}

/// The r8 role tabs on the title line, right of "Recording": the plate's centre, one tab's width, the key caps.
pub const ROLE_X: f32 = 600.0;
pub const ROLE_TAB_W: f32 = 176.0;
pub const ROLE_H: f32 = 52.0;
pub const ROLE_CAP: f32 = 40.0;
pub const ROLE_CAP_X: [f32; 2] = [ROLE_X - ROLE_TAB_W - 40.0, ROLE_X + ROLE_TAB_W + 40.0];
pub const fn role_tab_x(tab: usize) -> f32 {
    ROLE_X + (tab as f32 - 0.5) * ROLE_TAB_W
}
/// In the pop-up the title line ("Recording" and the role tabs) is on top and the page tabs sit under it.
pub const POP_TAB_Y: f32 = CRUMB_Y + 4.0;

// ---- direction grid geometry ---------------------------------------------------------------------------

/// Three rows of chips fit the content area (the tile grid's 620 units, y 320 down to -300): the pitch is a third of it.
pub const CHIP_SIZE: f32 = 184.0;
pub const CHIP_PITCH: f32 = 206.0;

/// Centre of the chip at grid position (column, row) as in menu::CHIP_POS (columns 0-2 relative, 3 absolute).
pub const fn chip_xy(col: u8, row: u8) -> (f32, f32) {
    let x = -300.0 + CHIP_PITCH * col as f32 + if col == 3 { 70.0 } else { 0.0 };
    (x, 217.0 - CHIP_PITCH * row as f32)
}

// ---- names ---------------------------------------------------------------------------------------------
//
// The panes in tree (draw) order, back to front. An index is stable: the builder adds the panes in this
// order and the runtime keeps its resolved pointers in an array of the same order.

/// The menu's container, under `RootPane`.
pub const IDX_MENU: usize = 0;
pub const IDX_BG: usize = 1;
pub const IDX_TAB_HI: usize = IDX_BG + 1;
pub const IDX_TAB: usize = IDX_TAB_HI + 1;
pub const IDX_TITLE: usize = IDX_TAB + TABS;
pub const IDX_SEP: usize = IDX_TITLE + 1;
pub const IDX_CRUMB: usize = IDX_SEP + 1;
pub const IDX_DIVIDER: usize = IDX_CRUMB + 1;
pub const IDX_GROUP_HI: usize = IDX_DIVIDER + 1;
pub const IDX_GROUP: usize = IDX_GROUP_HI + 1;
pub const IDX_ROW_HI: usize = IDX_GROUP + GROUPS;
/// Recording chrome, behind the row text: a green plate per row, the hot-five hairline,
/// two section outlines (four edges each) and the white cursor (a left bar and four edges).
pub const CHROME_GREENS: usize = ROWS;
pub const CHROME_HAIR: usize = 1;
pub const CHROME_OUTLINE: usize = 8;
pub const CHROME_CURSOR: usize = 5;
pub const CHROME: usize = CHROME_GREENS + CHROME_HAIR + CHROME_OUTLINE + CHROME_CURSOR;
pub const IDX_GREEN: usize = IDX_ROW_HI + 1;
pub const IDX_HAIRLINE: usize = IDX_GREEN + CHROME_GREENS;
pub const IDX_OUTLINE: usize = IDX_HAIRLINE + CHROME_HAIR;
pub const IDX_CURSOR: usize = IDX_OUTLINE + CHROME_OUTLINE;
/// Row `i`'s label is `IDX_ROW + 2 * i` and its value `IDX_ROW + 2 * i + 1`.
pub const IDX_ROW: usize = IDX_CURSOR + CHROME_CURSOR;
pub const IDX_SEP_HELP: usize = IDX_ROW + 2 * ROWS;
pub const IDX_HELP: usize = IDX_SEP_HELP + 1;
pub const IDX_HINT: usize = IDX_HELP + 2;
/// The direction grid's chips (3 x 3 relative to the opponent plus the two absolute ones): a tile, a needle that
/// points the chip's way, a label and a weight badge each, and the focus ring.
pub const CHIPS: usize = 11;
pub const IDX_CHIP_HI: usize = IDX_HINT + 1;
pub const IDX_CHIP: usize = IDX_CHIP_HI + 1;
pub const IDX_CHIP_NEEDLE: usize = IDX_CHIP + CHIPS;
pub const IDX_CHIP_LABEL: usize = IDX_CHIP_NEEDLE + CHIPS;
pub const IDX_CHIP_BADGE: usize = IDX_CHIP_LABEL + CHIPS;
/// The item and stage grids' tiles (up to 5 x 8): a tile and a label each, and the focus ring. Their size and place
/// are set when a grid opens (the layout follows the console mode).
pub const TILES: usize = 88;
/// The tile name label's and the roster marks' (badges, Solo Popo "S") box heights; the renderer sets only widths.
pub const TILE_LABEL_H: f32 = 40.0;
pub const MARK_H: f32 = 28.0;
/// The height the grid's rows share (a tile is `(ROW_RIGHT - ROW_LEFT) / cols` by `TILE_AREA_H / rows`).
pub const TILE_AREA_H: f32 = 620.0;

/// A pane's box: centre and size in layout units (y up).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Area {
    pub fn overlaps(self, other: Area) -> bool {
        (self.x - other.x).abs() * 2.0 < self.w + other.w && (self.y - other.y).abs() * 2.0 < self.h + other.h
    }
}

/// A roster grid tile's parts relative to its cell centre: the portrait, the name label and the Solo Popo "S" marks
/// (Player left, CPU right). With the portrait drawn the name sits under it and the marks on the portrait's lower
/// corners, just above the name (SOLO-S-PLACEMENT, operator 2026-11-05: cover the icon partially, never the name);
/// without it the name is centred and the marks sit just under it.
pub fn roster_tile(cell_w: f32, cell_h: f32, pictured: bool) -> (Area, Area, [Area; 2]) {
    let size = (cell_h - 28.0).min(cell_w - 12.0);
    let icon = Area { x: 0.0, y: 10.0, w: size, h: size };
    let label = if pictured {
        Area { x: 0.0, y: 12.0 - cell_h / 2.0, w: cell_w - 6.0, h: TILE_LABEL_H }
    } else {
        Area { x: 0.0, y: 0.0, w: cell_w - 18.0, h: TILE_LABEL_H }
    };
    let y = label.y + [-1.0, 1.0][usize::from(pictured)] * (TILE_LABEL_H + MARK_H) / 2.0;
    (icon, label, [-36.0, 36.0].map(|x| Area { x, y, w: 40.0, h: MARK_H }))
}
pub const IDX_TILE_HI: usize = IDX_CHIP_BADGE + CHIPS;
pub const IDX_TILE: usize = IDX_TILE_HI + 1;
pub const IDX_TILE_LABEL: usize = IDX_TILE + TILES;
pub const ITEM_ICONS: usize = 38;
pub const IDX_ITEM_ICON: usize = IDX_TILE_LABEL + TILES;
/// The roster grid's per-role marks, Player block then CPU block (`Frame::tile_badges` etc.): the fork's corner
/// badges (`roster_p_NN` / `roster_c_NN`), All-stripes and the Solo Popo "S" markers. Menu panes: the renderer
/// draws them through `Panes::all`, so they sit inside `MENU_COUNT` (P0 2026-11-04: placed after the input log,
/// they were indexed past the resolved array).
pub const IDX_TILE_BADGE: usize = IDX_ITEM_ICON + ITEM_ICONS;
pub const IDX_TILE_STRIPE: usize = IDX_TILE_BADGE + 2 * TILES;
pub const IDX_TILE_SOLO: usize = IDX_TILE_STRIPE + 2 * TILES;
/// Floppy marks (`tr_icon_save`). Two section headers and four rows in each section.
pub const FLOPPIES: usize = 10;
/// Loop marks (`tr_icon_repeat`), one beside each section header. Only MACROS shows one.
pub const LOOPS: usize = 2;
pub const IDX_FLOPPY: usize = IDX_TILE_SOLO + 2;
pub const IDX_LOOP: usize = IDX_FLOPPY + FLOPPIES;
/// The r8 recording pop-up (`docs/decisions/RECORDING-REDESIGN-r8-popup-*.png` on `review/menu-proposal`; primary
/// 2026-11-08 #19 and #49): a line under each list row ("Recorded on …"), and the CONNECTIONS column right of the list
/// with its divider and header. Hidden until a recording screen draws them.
pub const SUBS: usize = ROWS;
pub const CONN_LINES: usize = 14;
pub const IDX_SUB: usize = IDX_LOOP + LOOPS;
pub const IDX_CONN_DIV: usize = IDX_SUB + SUBS;
pub const IDX_CONN_HEAD: usize = IDX_CONN_DIV + 1;
pub const IDX_CONN: usize = IDX_CONN_HEAD + 1;
/// The r8 CONNECTIONS tree (primary 2026-11-08 15:55Z, "as drawn"): a kind box per file (four edges, a pill and its
/// label), the head box and up to five joined files, and the tree's stem and one tick per joined file.
pub const CONN_BOXES: usize = 6;
pub const IDX_CONN_BOX: usize = IDX_CONN + CONN_LINES;
pub const IDX_CONN_PILL: usize = IDX_CONN_BOX + 4 * CONN_BOXES;
pub const IDX_CONN_PILL_TEXT: usize = IDX_CONN_PILL + CONN_BOXES;
pub const CONN_TREE: usize = CONN_BOXES;
pub const IDX_CONN_TREE: usize = IDX_CONN_PILL_TEXT + CONN_BOXES;
/// The r8 title's L/R role tabs: their plate, the lit half, "Player" and "CPU", and the L and R key caps.
pub const IDX_ROLE_BG: usize = IDX_CONN_TREE + CONN_TREE;
pub const IDX_ROLE_HI: usize = IDX_ROLE_BG + 1;
pub const IDX_ROLE: usize = IDX_ROLE_HI + 1;
pub const IDX_ROLE_CAP: usize = IDX_ROLE + 2;
pub const IDX_ROLE_KEY: usize = IDX_ROLE_CAP + 2;
/// The r8 lists (primary 2026-11-08 16:51Z): the two section headers' kind squares, the red order badges and their
/// numbers, and the four edges of each of the pop-up's two page tabs.
pub const IDX_HEAD_SQ: usize = IDX_ROLE_KEY + 2;
pub const IDX_BADGE: usize = IDX_HEAD_SQ + 2;
pub const IDX_BADGE_NUM: usize = IDX_BADGE + BADGES;
pub const TAB_BOXES: usize = 2;
pub const IDX_TAB_BOX: usize = IDX_BADGE_NUM + BADGES;
/// How many panes the menu itself owns and the runtime resolves at registration (`view::Panes::all`); every index
/// the renderer writes is below it. The panes after these (the notification panel and the frame strip) belong to the
/// input package, which finds them by name when it draws.
pub const MENU_COUNT: usize = IDX_TAB_BOX + 4 * TAB_BOXES;
/// The notification panel (SPEC-input-05): its own container under `RootPane`. Each of its texts (header, short
/// body, long body) has a dark plate drawn immediately before it. Written by the notices code of the input package.
pub const IDX_NOTICE: usize = MENU_COUNT;
pub const IDX_NOTICE_HEADER_PLATE: usize = IDX_NOTICE + 1;
pub const IDX_NOTICE_HEADER: usize = IDX_NOTICE + 2;
pub const IDX_NOTICE_TEXT_PLATE: usize = IDX_NOTICE + 3;
pub const IDX_NOTICE_TEXT: usize = IDX_NOTICE + 4;
pub const IDX_NOTICE_LONG_PLATE: usize = IDX_NOTICE + 5;
pub const IDX_NOTICE_LONG: usize = IDX_NOTICE + 6;
/// Full startup chord text plus its terminator; checked against notify::BODY_MAX.
pub const NOTICE_LONG_UNITS: u16 = 321;
/// The frame strip (SPEC-input-06, operator decision INPUT-VIZ A, 2026-11-05; it replaced the input log's 271 panes):
/// its own container `TrIv` under `RootPane` and the panes below it, all named `TrIv…`. The input package writes
/// them (`model::frame_strip_plan`).
pub const IDX_IV: usize = IDX_NOTICE_LONG + 1;
/// The combo bar under the strip (SPEC-input-07 §7, operator decision COMBO-HELPER A, 2026-11-05): its own container
/// `TrCb` inside `TrIv` (so it hides with the strip and adds no root), and panes named `TrCb…`.
pub const IDX_CB: usize = IDX_IV + iv::COUNT;
/// The hitbox visualizer's own orbs (`model::hitbox_orbs`, decision HB-OWN-ORBS): a container under `RootPane` (the
/// layout builder makes it RootPane's first child in both layouts, so every other pane draws over it) and `ORBS` pictures of the plugin's own disc texture (`tr_orb_disc`, drawn by the layout builder), tinted by
/// their vertex colours. 512 = Training's most fighter objects (two fighters, each Ice Climbers with Nana) times the
/// spheres one fighter holds (`hitbox::MAX_HELD`, 128); `tests/core06_orbs.rs` keeps the two equal (review HB-2).
pub const ORBS: usize = 512;
pub const IDX_ORBS: usize = IDX_CB + cb::COUNT;
pub const IDX_ORB: usize = IDX_ORBS + 1;
/// The orb panes' names: `tr_oa_00` .. `tr_of_11`, one prefix per hundred.
const ORB_PREFIXES: [&str; 6] = ["tr_oa_", "tr_ob_", "tr_oc_", "tr_od_", "tr_oe_", "tr_of_"];
/// How many panes the plugin adds to the layout.
pub const COUNT: usize = IDX_ORB + ORBS;

/// The combo bar's pane indices and geometry (canvas units; the bar's centre is the origin of its panes).
pub mod cb {
    /// Link cells and result pills.
    pub const CELLS: usize = 4;
    pub const PILLS: usize = 8;

    pub mod at {
        use super::{CELLS, PILLS};
        pub const CONTAINER: usize = 0;
        pub const BACKDROP: usize = 1;
        pub const HEADER: usize = 2;
        pub const STREAK: usize = 3;
        pub const RESULT: usize = 4;
        pub const REASON: usize = 5;
        pub const PILL: usize = 6;
        /// Per cell, one block of `CELLS` each: backdrop, title, verdict, offset, ruler, window, recorded and your press.
        pub const CELL: usize = PILL + PILLS;
        pub const TITLE: usize = CELL + CELLS;
        pub const VERDICT: usize = TITLE + CELLS;
        pub const OFFSET: usize = VERDICT + CELLS;
        pub const RULER: usize = OFFSET + CELLS;
        pub const WINDOW: usize = RULER + CELLS;
        pub const RECORDED: usize = WINDOW + CELLS;
        pub const YOURS: usize = RECORDED + CELLS;
        pub const COUNT: usize = YOURS + CELLS;
    }

    pub const COUNT: usize = at::COUNT;

    /// The bar's centre below the strip's, and its size (the mockup's 864 × 132).
    pub const BAR_Y: f32 = -184.0;
    pub const BAR_W: f32 = 864.0;
    pub const BAR_H: f32 = 132.0;
    pub const LEFT: f32 = -414.0;
    pub const HEADER_Y: f32 = 48.0;
    pub const FOOTER_Y: f32 = -48.0;
    pub const CELL_Y: f32 = 2.0;
    pub const CELL_W: f32 = 200.0;
    pub const CELL_H: f32 = 64.0;
    pub const CELL_PITCH: f32 = 208.0;
    pub const CELL_X0: f32 = -312.0;
    pub const TITLE_DY: f32 = 18.0;
    pub const VERDICT_DY: f32 = -2.0;
    pub const RULER_DY: f32 = -22.0;
    /// The ruler: frames from A₁, `RULER_FROM ..= RULER_FROM + RULER_FRAMES`, `PER_FRAME` units each.
    pub const RULER_FROM: i32 = -10;
    pub const RULER_FRAMES: i32 = 30;
    pub const PER_FRAME: f32 = 6.0;
    pub const RULER_W: f32 = RULER_FRAMES as f32 * PER_FRAME;
    pub const PILL_SIZE: f32 = 14.0;
    pub const PILL_X0: f32 = 166.0;
    pub const PILL_PITCH: f32 = 20.0;
    pub const STREAK_X: f32 = 370.0;
    pub const BACKDROP_COLOR: [u8; 4] = [14, 18, 27, 222];
    pub const CELL_COLOR: [u8; 4] = [30, 36, 48, 255];
    pub const TEXT_UNITS: u16 = 24;
}

/// The frame strip's pane indices and geometry (canvas units, the container's centre is the origin).
pub mod iv {
    /// Frames on screen, links in the LAST row, lane runs and run lengths per lane, chips.
    pub const WINDOW: usize = 60;
    pub const HISTORY: usize = 4;
    pub const RUNS: usize = 16;
    pub const RUN_LABELS: usize = 4;
    pub const CHIPS: usize = 8;

    /// The strip's pane indices (local: `IDX_IV + at::…` in the table).
    pub mod at {
        use super::{CHIPS, HISTORY, RUNS, RUN_LABELS, WINDOW};
        pub const CONTAINER: usize = 0;
        pub const BACKDROP: usize = 1;
        /// IN, P1, CPU.
        pub const LANE_LABEL: usize = 2;
        /// The readouts' divider and the LAST row's rule.
        pub const SEPARATOR: usize = 5;
        /// P1's runs, then the CPU's.
        pub const RUN: usize = 7;
        pub const RUN_LABEL: usize = RUN + 2 * RUNS;
        pub const CHIP_BOX: usize = RUN_LABEL + 2 * RUN_LABELS;
        pub const CHIP_GLYPH: usize = CHIP_BOX + CHIPS;
        pub const CHIP_SHAFT: usize = CHIP_GLYPH + CHIPS;
        pub const CHIP_HEAD: usize = CHIP_SHAFT + CHIPS;
        pub const CHIP_TICK: usize = CHIP_HEAD + CHIPS;
        pub const WINDOW_BAR: usize = CHIP_TICK + CHIPS;
        pub const WINDOW_TEXT: usize = WINDOW_BAR + 1;
        /// The link bracket's left leg, bottom and right leg, then the advantage bracket's.
        pub const BRACKET: usize = WINDOW_TEXT + 1;
        pub const BRACKET_TEXT: usize = BRACKET + 6;
        pub const NOW: usize = BRACKET_TEXT + 2;
        pub const NOW_TEXT: usize = NOW + 1;
        /// ADVANTAGE, LINK, INPUT: labels, values, sub-lines.
        pub const READ_LABEL: usize = NOW_TEXT + 1;
        pub const READ_VALUE: usize = READ_LABEL + 3;
        pub const READ_SUB: usize = READ_VALUE + 3;
        pub const LAST_TEXT: usize = READ_SUB + 3;
        pub const PILL: usize = LAST_TEXT + 1;
        pub const PILL_TEXT: usize = PILL + HISTORY;
        pub const MOVE_TEXT: usize = PILL_TEXT + HISTORY;
        /// One mark per frame of the stick row, under P1's lane (oldest at `STICK`).
        pub const STICK: usize = MOVE_TEXT + HISTORY;
        pub const COUNT: usize = STICK + WINDOW;
    }

    pub const COUNT: usize = at::COUNT;

    pub const STRIP_Y: f32 = 420.0;
    pub const STRIP_W: f32 = 1296.0;
    pub const STRIP_H: f32 = 216.0;
    /// The left edge of the oldest frame's cell, the distance between frames and a cell's width.
    pub const FIRST_X: f32 = -564.0;
    pub const PITCH: f32 = 13.5;
    pub const CELL_W: f32 = 12.0;
    pub const NOW_X: f32 = FIRST_X + WINDOW as f32 * PITCH + 3.0;
    pub const LABEL_X: f32 = -606.0;
    pub const IN_Y: f32 = 73.5;
    /// P1 sits just under the input ticks. The lane is shorter than it was so a stick row fits between it and the CPU
    /// lane without moving the brackets, the LAST row or the combo bar.
    pub const P1_Y: f32 = 27.0;
    pub const CPU_Y: f32 = -21.0;
    pub const LANE_H: f32 = 16.0;
    /// Centre of the stick row, in the gap between the two lanes.
    pub const STICK_Y: f32 = 3.0;
    pub const CHIP: f32 = 30.0;
    pub const TICK_Y: f32 = 47.25;
    pub const TICK_H: f32 = 22.5;
    pub const WINDOW_Y: f32 = 44.25;
    pub const WINDOW_H: f32 = 7.5;
    pub const LEG_Y: f32 = -39.0;
    pub const LEG_H: f32 = 9.0;
    pub const BAR_Y: f32 = -43.5;
    pub const BRACKET_TEXT_Y: f32 = -57.0;
    pub const READ_X: f32 = 276.0;
    pub const READ_LABEL_Y: [f32; 3] = [90.0, 34.5, -21.0];
    pub const READ_VALUE_Y: [f32; 3] = [64.5, 9.0, -46.5];
    pub const READ_SUB_X: f32 = READ_X + 75.0;
    pub const DIVIDER_X: f32 = 262.5;
    pub const RULE_Y: f32 = -70.5;
    pub const LAST_Y: f32 = -87.0;
    pub const PILL_W: f32 = 96.0;
    pub const PILL_STRIDE: f32 = 300.0;
    pub const PILL_X0: f32 = FIRST_X + PILL_W / 2.0;
    pub const BACKDROP_COLOR: [u8; 4] = [14, 18, 27, 222];
    /// The text boxes' capacity in UTF-16 units.
    pub const TEXT_UNITS: u16 = 24;
    /// Small labels, the readouts' sub-lines, the readouts' values.
    pub const FONT_TAG: (f32, f32) = (14.0, 22.0);
    pub const FONT_SUB: (f32, f32) = (16.0, 26.0);
    pub const FONT_VALUE: (f32, f32) = (26.0, 40.0);
}

/// The frame strip's pane names.
const fn iv_name(local: usize) -> Name {
    use iv::at::*;
    use iv::{CHIPS, RUNS, RUN_LABELS, WINDOW};
    match local {
        CONTAINER => Name::new("TrIv"),
        BACKDROP => Name::new("TrIvBg"),
        i if i < SEPARATOR => Name::indexed("TrIvLab", i - LANE_LABEL),
        i if i < RUN => Name::indexed("TrIvSep", i - SEPARATOR),
        i if i < RUN_LABEL => Name::indexed("TrIvRun", i - RUN),
        i if i < CHIP_BOX => Name::indexed("TrIvRunN", i - RUN_LABEL),
        i if i < WINDOW_BAR => {
            let n = (i - CHIP_BOX) % CHIPS;
            match (i - CHIP_BOX) / CHIPS {
                0 => Name::indexed("TrIvChB", n),
                1 => Name::indexed("TrIvChG", n),
                2 => Name::indexed("TrIvChS", n),
                3 => Name::indexed("TrIvChH", n),
                _ => Name::indexed("TrIvChT", n),
            }
        }
        WINDOW_BAR => Name::new("TrIvWin"),
        WINDOW_TEXT => Name::new("TrIvWinT"),
        i if i < BRACKET_TEXT => Name::indexed("TrIvBr", i - BRACKET),
        i if i < NOW => Name::indexed("TrIvBrT", i - BRACKET_TEXT),
        NOW => Name::new("TrIvNow"),
        NOW_TEXT => Name::new("TrIvNowT"),
        i if i < READ_VALUE => Name::indexed("TrIvRl", i - READ_LABEL),
        i if i < READ_SUB => Name::indexed("TrIvRv", i - READ_VALUE),
        i if i < LAST_TEXT => Name::indexed("TrIvRs", i - READ_SUB),
        LAST_TEXT => Name::new("TrIvLast"),
        i if i < PILL_TEXT => Name::indexed("TrIvPill", i - PILL),
        i if i < MOVE_TEXT => Name::indexed("TrIvPillT", i - PILL_TEXT),
        i if i < STICK => Name::indexed("TrIvMove", i - MOVE_TEXT),
        i if i < COUNT => Name::indexed("TrIvSk", i - STICK),
        _ => {
            let _ = (RUNS, RUN_LABELS, WINDOW);
            panic!("no such frame strip pane")
        }
    }
}

/// A hidden text box of the frame strip.
const fn iv_text(name: Name, [x, y, w]: [f32; 3], align: Align, font: (f32, f32)) -> Spec {
    Spec { visible: false, ..under(text(name, [x, y, w, 30.0], align, font, iv::TEXT_UNITS, TEXT), Name::new("TrIv")) }
}

/// The frame strip's panes: kind, place and size, all hidden until the input package draws them.
const fn iv_spec(local: usize, name: Name) -> Spec {
    use iv::at::*;
    use iv::{
        BACKDROP_COLOR, BAR_Y, BRACKET_TEXT_Y, CELL_W, CHIP, DIVIDER_X, FONT_SUB, FONT_TAG, FONT_VALUE, IN_Y, LABEL_X,
        LANE_H, LAST_Y, LEG_H, LEG_Y, NOW_X, PILL_W, READ_SUB_X, READ_X, RULE_Y, STICK_Y, STRIP_H, STRIP_W, STRIP_Y,
        TEXT_UNITS, TICK_H, TICK_Y, WINDOW_H, WINDOW_Y,
    };
    let parent = Name::new("TrIv");
    match local {
        CONTAINER => Spec { name, parent: ROOT_PANE, kind: Kind::Null, x: 0.0, y: STRIP_Y, w: STRIP_W, h: STRIP_H, visible: false },
        BACKDROP => under(rect(name, [0.0, 0.0, STRIP_W, STRIP_H], BACKDROP_COLOR, true), parent),
        i if i < SEPARATOR => iv_text(name, [LABEL_X, 0.0, 60.0], Align::Center, FONT_TAG),
        SEPARATOR => under(rect(name, [DIVIDER_X, 19.5, 1.5, 147.0], RULE, false), parent),
        i if i < RUN => under(rect(name, [0.0, RULE_Y, STRIP_W - 36.0, 1.5], RULE, false), parent),
        i if i < RUN_LABEL => under(rect(name, [0.0, 0.0, CELL_W, LANE_H], TEXT, false), parent),
        i if i < CHIP_BOX => iv_text(name, [0.0, 0.0, 60.0], Align::Center, FONT_TAG),
        i if i < CHIP_GLYPH => under(rect(name, [0.0, IN_Y, CHIP, CHIP], TEXT, false), parent),
        i if i < CHIP_SHAFT => Spec { visible: false, ..under(text(name, [0.0, IN_Y, CHIP, CHIP], Align::Center, FONT_SUB, 2, TEXT), parent) },
        i if i < CHIP_HEAD => under(rect(name, [0.0, IN_Y, 4.5, 14.0], TEXT, false), parent),
        i if i < CHIP_TICK => under(rect(name, [0.0, IN_Y, 9.0, 9.0], TEXT, false), parent),
        i if i < WINDOW_BAR => under(rect(name, [0.0, TICK_Y, 1.5, TICK_H], TEXT, false), parent),
        WINDOW_BAR => under(rect(name, [0.0, WINDOW_Y, 1.5, WINDOW_H], TEXT, false), parent),
        WINDOW_TEXT => iv_text(name, [0.0, WINDOW_Y, 120.0], Align::Center, FONT_TAG),
        i if i < BRACKET_TEXT && (i - BRACKET) % 3 == 1 => under(rect(name, [0.0, BAR_Y, 3.0, 3.0], TEXT, false), parent),
        i if i < BRACKET_TEXT => under(rect(name, [0.0, LEG_Y, 3.0, LEG_H], TEXT, false), parent),
        i if i < NOW => iv_text(name, [0.0, BRACKET_TEXT_Y, 120.0], Align::Center, FONT_TAG),
        NOW => under(rect(name, [NOW_X, 32.25, 1.5, 121.5], TEXT, false), parent),
        NOW_TEXT => iv_text(name, [NOW_X - 24.0, 84.0, 48.0], Align::Center, FONT_TAG),
        i if i < READ_VALUE => iv_text(name, [READ_X + 60.0, 0.0, 120.0], Align::Left, FONT_TAG),
        i if i < READ_SUB => Spec { visible: false, ..under(text(name, [READ_X + 30.0, 0.0, 60.0, 40.0], Align::Left, FONT_VALUE, TEXT_UNITS, TEXT), parent) },
        i if i < LAST_TEXT => iv_text(name, [READ_SUB_X + 90.0, 0.0, 180.0], Align::Left, FONT_SUB),
        LAST_TEXT => iv_text(name, [LABEL_X, LAST_Y, 60.0], Align::Center, FONT_TAG),
        i if i < PILL_TEXT => under(rect(name, [0.0, LAST_Y, PILL_W, 24.0], TEXT, false), parent),
        i if i < MOVE_TEXT => iv_text(name, [0.0, LAST_Y, PILL_W], Align::Center, FONT_TAG),
        i if i < STICK => iv_text(name, [0.0, LAST_Y, 200.0], Align::Left, FONT_TAG),
        i if i < COUNT => Spec {
            visible: false,
            ..under(text(name, [0.0, STICK_Y, 16.0, 18.0], Align::Center, FONT_TAG, 2, TEXT), parent)
        },
        _ => {
            let _ = TEXT_UNITS;
            panic!("no such frame strip pane")
        }
    }
}

/// The combo bar's pane names.
const fn cb_name(local: usize) -> Name {
    use cb::at::*;
    use cb::CELLS;
    match local {
        CONTAINER => Name::new("TrCb"),
        BACKDROP => Name::new("TrCbBg"),
        HEADER => Name::new("TrCbHead"),
        STREAK => Name::new("TrCbStrk"),
        RESULT => Name::new("TrCbRes"),
        REASON => Name::new("TrCbWhy"),
        i if i < CELL => Name::indexed("TrCbPill", i - PILL),
        i if i < COUNT => {
            let n = (i - CELL) % CELLS;
            match (i - CELL) / CELLS {
                0 => Name::indexed("TrCbCell", n),
                1 => Name::indexed("TrCbTit", n),
                2 => Name::indexed("TrCbVer", n),
                3 => Name::indexed("TrCbOff", n),
                4 => Name::indexed("TrCbRul", n),
                5 => Name::indexed("TrCbWin", n),
                6 => Name::indexed("TrCbRec", n),
                _ => Name::indexed("TrCbYou", n),
            }
        }
        _ => panic!("no such combo bar pane"),
    }
}

/// A hidden text box of the combo bar; `x` is the box's centre.
const fn cb_text(name: Name, [x, y, w]: [f32; 3], align: Align, font: (f32, f32)) -> Spec {
    Spec { visible: false, ..under(text(name, [x, y, w, 26.0], align, font, cb::TEXT_UNITS, TEXT), Name::new("TrCb")) }
}

/// The combo bar's panes: kind, place and size, all hidden until the helper draws them.
const fn cb_spec(local: usize, name: Name) -> Spec {
    use cb::at::*;
    use cb::{
        BAR_H, BAR_W, BAR_Y, CELLS, CELL_COLOR, CELL_H, CELL_W, CELL_Y, FOOTER_Y, HEADER_Y, LEFT, PILL_SIZE, RULER_DY,
        RULER_W, STREAK_X, TITLE_DY, VERDICT_DY,
    };
    use iv::{FONT_SUB, FONT_TAG};
    let parent = Name::new("TrCb");
    match local {
        CONTAINER => Spec { name, parent: Name::new("TrIv"), kind: Kind::Null, x: 0.0, y: BAR_Y, w: BAR_W, h: BAR_H, visible: false },
        BACKDROP => Spec { visible: false, ..under(rect(name, [0.0, 0.0, BAR_W, BAR_H], cb::BACKDROP_COLOR, true), parent) },
        HEADER => cb_text(name, [LEFT + 180.0, HEADER_Y, 360.0], Align::Left, FONT_SUB),
        STREAK => cb_text(name, [STREAK_X, HEADER_Y, 120.0], Align::Center, FONT_TAG),
        RESULT => cb_text(name, [LEFT + 120.0, FOOTER_Y, 240.0], Align::Left, FONT_SUB),
        REASON => cb_text(name, [LEFT + 260.0 + 200.0, FOOTER_Y, 400.0], Align::Left, FONT_SUB),
        i if i < CELL => under(rect(name, [0.0, HEADER_Y, PILL_SIZE, PILL_SIZE], TEXT, false), parent),
        i if i < TITLE => under(rect(name, [0.0, CELL_Y, CELL_W, CELL_H], CELL_COLOR, false), parent),
        i if i < VERDICT => cb_text(name, [0.0, CELL_Y + TITLE_DY, CELL_W - 8.0], Align::Center, FONT_TAG),
        i if i < OFFSET => cb_text(name, [0.0, CELL_Y + VERDICT_DY, 96.0], Align::Center, FONT_TAG),
        i if i < RULER => cb_text(name, [0.0, CELL_Y + VERDICT_DY, 100.0], Align::Center, FONT_TAG),
        i if i < WINDOW => under(rect(name, [0.0, CELL_Y + RULER_DY, RULER_W, 3.0], RULE, false), parent),
        i if i < RECORDED => under(rect(name, [0.0, CELL_Y + RULER_DY, 6.0, 9.0], TEXT, false), parent),
        i if i < YOURS => under(rect(name, [0.0, CELL_Y + RULER_DY, 4.0, 16.0], TEXT, false), parent),
        i if i < COUNT => under(rect(name, [0.0, CELL_Y + RULER_DY, 6.0, 12.0], TEXT, false), parent),
        _ => {
            let _ = CELLS;
            panic!("no such combo bar pane")
        }
    }
}

/// A roster mark's name: one prefix per block (Player and CPU badges, Player and CPU stripes, the Solo markers) and the
/// tile as two digits. `name2` writes its second number as one digit, so it cannot name 88 tiles.
const fn mark_name(n: usize) -> Name {
    let prefix = match n / TILES {
        0 => "tr_rb0_",
        1 => "tr_rb1_",
        2 => "tr_rs0_",
        3 => "tr_rs1_",
        _ => "tr_rp_",
    };
    Name::indexed(prefix, n % TILES)
}

pub const fn tab_index(i: usize) -> usize {
    IDX_TAB + i
}
pub const fn group_index(i: usize) -> usize {
    IDX_GROUP + i
}
pub const fn row_label_index(i: usize) -> usize {
    IDX_ROW + 2 * i
}
pub const fn row_value_index(i: usize) -> usize {
    IDX_ROW + 2 * i + 1
}

/// The name of pane `index` (`< COUNT`).
pub const fn name_of(index: usize) -> Name {
    match index {
        i if i >= IDX_ITEM_ICON && i < IDX_ITEM_ICON + ITEM_ICONS => Name::indexed("tr_item_", i - IDX_ITEM_ICON),
        IDX_ORBS => Name::new("TrOrbs"),
        i if i >= IDX_ORB && i < IDX_ORB + ORBS => Name::indexed(ORB_PREFIXES[(i - IDX_ORB) / 100], (i - IDX_ORB) % 100),
        IDX_MENU => ROOT,
        IDX_NOTICE => Name::new("TrModDisp0"),
        IDX_NOTICE_HEADER_PLATE => Name::new("TrModDisp0HdrPlate"),
        IDX_NOTICE_HEADER => Name::new("TrModDisp0Header"),
        IDX_NOTICE_TEXT_PLATE => Name::new("TrModDisp0TxtPlate"),
        IDX_NOTICE_TEXT => Name::new("TrModDisp0Txt"),
        IDX_NOTICE_LONG_PLATE => Name::new("TrModDisp0LongPlate"),
        IDX_NOTICE_LONG => Name::new("TrModDisp0Long"),
        i if i >= IDX_IV && i < IDX_CB => iv_name(i - IDX_IV),
        i if i >= IDX_CB && i < IDX_ORBS => cb_name(i - IDX_CB),
        IDX_CHIP_HI => Name::new("tr_ch_hi"),
        IDX_TILE_HI => Name::new("tr_ti_hi"),
        IDX_BG => Name::new("tr_bg"),
        IDX_TAB_HI => Name::new("tr_tab_hi"),
        IDX_TITLE => Name::new("tr_title"),
        IDX_SEP => Name::new("tr_sep"),
        IDX_CRUMB => Name::new("tr_crumb"),
        IDX_DIVIDER => Name::new("tr_div"),
        IDX_GROUP_HI => Name::new("tr_grp_hi"),
        IDX_ROW_HI => Name::new("tr_row_hi"),
        i if i >= IDX_GREEN && i < IDX_HAIRLINE => Name::indexed("tr_grn_", i - IDX_GREEN),
        IDX_HAIRLINE => Name::new("tr_hair"),
        i if i >= IDX_OUTLINE && i < IDX_CURSOR => Name::indexed("tr_out_", i - IDX_OUTLINE),
        i if i >= IDX_CURSOR && i < IDX_ROW => Name::indexed("tr_cur_", i - IDX_CURSOR),
        IDX_SEP_HELP => Name::new("tr_sep_help"),
        i if i == IDX_HELP => Name::new("tr_help_0"),
        i if i == IDX_HELP + 1 => Name::new("tr_help_1"),
        IDX_HINT => Name::new("tr_hint"),
        i if i >= IDX_CHIP && i < IDX_CHIP + CHIPS => Name::indexed("tr_ch_", i - IDX_CHIP),
        i if i >= IDX_CHIP_NEEDLE && i < IDX_CHIP_NEEDLE + CHIPS => Name::indexed("tr_cn_", i - IDX_CHIP_NEEDLE),
        i if i >= IDX_CHIP_LABEL && i < IDX_CHIP_LABEL + CHIPS => Name::indexed("tr_cl_", i - IDX_CHIP_LABEL),
        i if i >= IDX_CHIP_BADGE && i < IDX_CHIP_BADGE + CHIPS => Name::indexed("tr_cb_", i - IDX_CHIP_BADGE),
        i if i >= IDX_TILE && i < IDX_TILE + TILES => Name::indexed("tr_ti_", i - IDX_TILE),
        i if i >= IDX_TILE_LABEL && i < IDX_TILE_LABEL + TILES => Name::indexed("tr_tl_", i - IDX_TILE_LABEL),
        i if i >= IDX_FLOPPY && i < IDX_LOOP => Name::indexed("tr_floppy_", i - IDX_FLOPPY),
        i if i >= IDX_LOOP && i < IDX_SUB => Name::indexed("tr_loop_", i - IDX_LOOP),
        i if i >= IDX_SUB && i < IDX_CONN_DIV => Name::indexed("tr_sub_", i - IDX_SUB),
        IDX_CONN_DIV => Name::new("tr_conn_div"),
        IDX_CONN_HEAD => Name::new("tr_conn_head"),
        i if i >= IDX_CONN && i < IDX_CONN_BOX => Name::indexed("tr_conn_", i - IDX_CONN),
        i if i >= IDX_CONN_BOX && i < IDX_CONN_PILL => Name::indexed("tr_cbx_", i - IDX_CONN_BOX),
        i if i >= IDX_CONN_PILL && i < IDX_CONN_PILL_TEXT => Name::indexed("tr_cpl_", i - IDX_CONN_PILL),
        i if i >= IDX_CONN_PILL_TEXT && i < IDX_CONN_TREE => Name::indexed("tr_cpt_", i - IDX_CONN_PILL_TEXT),
        i if i >= IDX_CONN_TREE && i < IDX_ROLE_BG => Name::indexed("tr_ctr_", i - IDX_CONN_TREE),
        IDX_ROLE_BG => Name::new("tr_role_bg"),
        IDX_ROLE_HI => Name::new("tr_role_hi"),
        i if i >= IDX_ROLE && i < IDX_ROLE_CAP => Name::indexed("tr_role_", i - IDX_ROLE),
        i if i >= IDX_ROLE_CAP && i < IDX_ROLE_KEY => Name::indexed("tr_rcap_", i - IDX_ROLE_CAP),
        i if i >= IDX_ROLE_KEY && i < IDX_HEAD_SQ => Name::indexed("tr_rkey_", i - IDX_ROLE_KEY),
        i if i >= IDX_HEAD_SQ && i < IDX_BADGE => Name::indexed("tr_hsq_", i - IDX_HEAD_SQ),
        i if i >= IDX_BADGE && i < IDX_BADGE_NUM => Name::indexed("tr_obg_", i - IDX_BADGE),
        i if i >= IDX_BADGE_NUM && i < IDX_TAB_BOX => Name::indexed("tr_obn_", i - IDX_BADGE_NUM),
        i if i >= IDX_TAB_BOX && i < MENU_COUNT => Name::indexed("tr_tbx_", i - IDX_TAB_BOX),
        i if i >= IDX_TILE_BADGE && i < IDX_FLOPPY => mark_name(i - IDX_TILE_BADGE),
        i if i >= IDX_TAB && i < IDX_TAB + TABS => Name::indexed("tr_tab_", i - IDX_TAB),
        i if i >= IDX_GROUP && i < IDX_GROUP + GROUPS => Name::indexed("tr_grp_", i - IDX_GROUP),
        i if i >= IDX_ROW && i < IDX_ROW + 2 * ROWS => {
            if (i - IDX_ROW).is_multiple_of(2) {
                Name::indexed("tr_rl_", (i - IDX_ROW) / 2)
            } else {
                Name::indexed("tr_rv_", (i - IDX_ROW) / 2)
            }
        }
        _ => panic!("no such pane"),
    }
}

// ---- colours (RGBA) ------------------------------------------------------------------------------------
//
// The layout engine treats vertex and text colours as linear light and encodes them for the screen, so the
// sRGB colours below are converted with linear before they are written (builder and runtime alike).

/// The linear-light bytes that show as srgb on the screen (alpha is unchanged).
pub fn linear(srgb: [u8; 4]) -> [u8; 4] {
    let channel = |v: u8| {
        let s = f32::from(v) / 255.0;
        let l = if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) };
        (l * 255.0 + 0.5) as u8
    };
    [channel(srgb[0]), channel(srgb[1]), channel(srgb[2]), srgb[3]]
}

pub const PANEL: [u8; 4] = [8, 10, 14, 255];
pub const ACCENT: [u8; 4] = [176, 48, 40, 255];
pub const HIGHLIGHT: [u8; 4] = [38, 44, 56, 255];
/// The focus ring of a box or handle that is itself on the accent.
pub const ACCENT_RING: [u8; 4] = [240, 240, 240, 255];
pub const RULE: [u8; 4] = [60, 66, 78, 255];
pub const TEXT: [u8; 4] = [236, 238, 242, 255];
pub const TEXT_DIM: [u8; 4] = [150, 156, 168, 255];
pub const TEXT_ON: [u8; 4] = [110, 210, 160, 255];
/// The green row plate. Light text stays readable on it.
pub const GREEN_ROW: [u8; 4] = [16, 72, 48, 255];
pub const CURSOR_WHITE: [u8; 4] = [255, 255, 255, 255];
pub const TEXT_OFF: [u8; 4] = [110, 116, 128, 255];
/// Opaque dark plate behind each notice text. A white plate disappeared on a bright stage; black body text
/// on this plate is the UI20 failure (about 1.06:1), so the body colours are the light ones.
pub const NOTICE_PLATE: [u8; 4] = PANEL;
/// Padding, in canvas units, on every side of a notice text when its plate is sized.
pub const NOTICE_PLATE_PAD: f32 = 16.0;
pub const NOTICE_HEADER: [u8; 4] = TEXT;

/// Which notice texts are shown. Each plate uses the same flag as its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoticeFaces {
    pub header: bool,
    pub body: bool,
    pub long_body: bool,
}

/// `shown` is the notice container. A long body uses the long text and hides the short one.
pub const fn notice_faces(shown: bool, long_body: bool) -> NoticeFaces {
    NoticeFaces { header: shown, body: shown && !long_body, long_body: shown && long_body }
}

const fn padded([x, y, w, h]: [f32; 4]) -> [f32; 4] {
    [x, y, w + 2.0 * NOTICE_PLATE_PAD, h + 2.0 * NOTICE_PLATE_PAD]
}

const NOTICE_HEADER_BOX: [f32; 4] = [0.0, 445.0, 960.0, 50.0];
const NOTICE_TEXT_BOX: [f32; 4] = [0.0, 390.0, 960.0, 70.0];
const NOTICE_LONG_BOX: [f32; 4] = [0.0, 325.0, 960.0, 180.0];

/// `[x, y, w, h]`: a pane's place and size, the order `Spec` keeps them in.
const fn rect(name: Name, [x, y, w, h]: [f32; 4], color: [u8; 4], visible: bool) -> Spec {
    Spec { name, parent: ROOT, kind: Kind::Rect { color }, x, y, w, h, visible }
}

const fn text(name: Name, [x, y, w, h]: [f32; 4], align: Align, font: (f32, f32), chars: u16, color: [u8; 4]) -> Spec {
    Spec { name, parent: ROOT, kind: Kind::Text { align, font, chars, color }, x, y, w, h, visible: true }
}

const ROOT_PANE: Name = Name::new("RootPane");

const fn under(mut spec: Spec, parent: Name) -> Spec {
    spec.parent = parent;
    spec
}

/// Centre of `TrModDisp0`, below the combo bar. The header box is the highest notice text
/// (local y 445, height 50). Sixteen units are the plate padding `p23/notice-plates` draws
/// around that text. The combo bar's bottom is `STRIP_Y + BAR_Y - BAR_H / 2`, and 8 units stay clear of it.
pub const NOTICE_ORIGIN_Y: f32 = {
    let header_top = 445.0 + 50.0 / 2.0;
    let plate_pad = 16.0;
    let bar_bottom = iv::STRIP_Y + cb::BAR_Y - cb::BAR_H / 2.0;
    let gap = 8.0;
    bar_bottom - gap - (header_top + plate_pad)
};

/// Pane `index`'s kind and place.
pub const fn spec(index: usize) -> Spec {
    let name = name_of(index);
    match index {
        i if i >= IDX_ITEM_ICON && i < IDX_ITEM_ICON + ITEM_ICONS => rect(name, [0.0, 0.0, 64.0, 64.0], [255; 4], false),
        IDX_ORBS => Spec { name, parent: ROOT_PANE, kind: Kind::Null, x: 0.0, y: 0.0, w: 1920.0, h: 1080.0, visible: false },
        i if i >= IDX_ORB && i < IDX_ORB + ORBS => under(rect(name, [0.0, 0.0, 64.0, 64.0], [255; 4], false), name_of(IDX_ORBS)),
        IDX_MENU => Spec { name, parent: ROOT_PANE, kind: Kind::Null, x: 0.0, y: 0.0, w: 1920.0, h: 1080.0, visible: false },
        IDX_NOTICE => Spec { name, parent: ROOT_PANE, kind: Kind::Null, x: 0.0, y: NOTICE_ORIGIN_Y, w: 1920.0, h: 1080.0, visible: false },
        IDX_NOTICE_HEADER_PLATE => under(rect(name, padded(NOTICE_HEADER_BOX), NOTICE_PLATE, true), name_of(IDX_NOTICE)),
        IDX_NOTICE_HEADER => under(text(name, NOTICE_HEADER_BOX, Align::Center, FONT_SMALL, 24, NOTICE_HEADER), name_of(IDX_NOTICE)),
        IDX_NOTICE_TEXT_PLATE => under(rect(name, padded(NOTICE_TEXT_BOX), NOTICE_PLATE, true), name_of(IDX_NOTICE)),
        IDX_NOTICE_TEXT => under(text(name, NOTICE_TEXT_BOX, Align::Center, FONT, 48, TEXT), name_of(IDX_NOTICE)),
        IDX_NOTICE_LONG_PLATE => under(rect(name, padded(NOTICE_LONG_BOX), NOTICE_PLATE, false), name_of(IDX_NOTICE)),
        IDX_NOTICE_LONG => under(Spec { visible: false, ..text(name, NOTICE_LONG_BOX, Align::Center, (14.0, 24.0),
            NOTICE_LONG_UNITS, TEXT) }, name_of(IDX_NOTICE)),
        IDX_BG => rect(name, [0.0, 0.0, PANEL_W, PANEL_H], PANEL, true),
        i if i >= IDX_IV && i < IDX_CB => iv_spec(i - IDX_IV, name),
        i if i >= IDX_CB && i < IDX_ORBS => cb_spec(i - IDX_CB, name),
        IDX_CHIP_HI => rect(name, [0.0, 0.0, CHIP_SIZE + 16.0, CHIP_SIZE + 16.0], ACCENT, false),
        IDX_TILE_HI => rect(name, [0.0, 0.0, 100.0, 60.0], ACCENT, false),
        i if i >= IDX_CHIP && i < IDX_CHIP + CHIPS => rect(name, [0.0, 0.0, CHIP_SIZE, CHIP_SIZE], HIGHLIGHT, false),
        i if i >= IDX_CHIP_NEEDLE && i < IDX_CHIP_NEEDLE + CHIPS => rect(name, [0.0, 0.0, 10.0, 84.0], TEXT, false),
        i if i >= IDX_CHIP_LABEL && i < IDX_CHIP_LABEL + CHIPS => Spec { visible: false, ..text(name, [0.0, 0.0, CHIP_SIZE, 44.0], Align::Center, FONT_SMALL, 16, TEXT) },
        i if i >= IDX_CHIP_BADGE && i < IDX_CHIP_BADGE + CHIPS => Spec { visible: false, ..text(name, [0.0, 0.0, 60.0, 44.0], Align::Center, FONT, 4, TEXT) },
        i if i >= IDX_TILE && i < IDX_TILE + TILES => rect(name, [0.0, 0.0, 100.0, 60.0], HIGHLIGHT, false),
        i if i >= IDX_TILE_LABEL && i < IDX_TILE_LABEL + TILES => Spec { visible: false, ..text(name, [0.0, 0.0, 100.0, TILE_LABEL_H], Align::Center, FONT_SMALL, 28, TEXT) },
        i if i >= IDX_TILE_STRIPE && i < IDX_TILE_SOLO => Spec { visible: false, ..rect(name, [0.0, 0.0, 4.0, 86.0], TEXT, false) },
        i if i >= IDX_FLOPPY && i < IDX_SUB => Spec { visible: false, ..rect(name, [0.0, 0.0, 28.0, 28.0], [255, 255, 255, 255], false) },
        i if i >= IDX_SUB && i < IDX_CONN_DIV => Spec {
            visible: false,
            ..text(name, [row_label_left(true) + row_label_w(true) / 2.0, row_y(i - IDX_SUB) - SUB_DROP, row_label_w(true), SUB_H],
                Align::Left, FONT_SUBLINE, 64, TEXT_DIM)
        },
        IDX_CONN_DIV => Spec { visible: false, ..rect(name, [CONN_DIVIDER_X, 15.0, 3.0, 640.0], RULE, false) },
        IDX_CONN_HEAD => Spec {
            visible: false,
            ..text(name, [(CONN_LEFT + CONN_RIGHT) / 2.0, ROW_FIRST_Y, CONN_RIGHT - CONN_LEFT, CONN_LINE_H], Align::Left, FONT_SMALL, 48, TEXT_DIM)
        },
        i if i >= IDX_CONN && i < IDX_CONN_BOX => Spec {
            visible: false,
            ..text(name, [(CONN_LEFT + CONN_RIGHT) / 2.0, conn_y(i - IDX_CONN), CONN_RIGHT - CONN_LEFT, CONN_LINE_H], Align::Left, FONT_SMALL, 72, TEXT)
        },
        i if i >= IDX_CONN_BOX && i < IDX_CONN_PILL_TEXT => {
            Spec { visible: false, ..rect(name, [(CONN_LEFT + CONN_RIGHT) / 2.0, conn_y(0), 8.0, 8.0], TEXT, false) }
        }
        i if i >= IDX_CONN_PILL_TEXT && i < IDX_CONN_TREE => Spec {
            visible: false,
            ..text(name, [CONN_LEFT + 80.0, conn_y(0), 160.0, PILL_H + 4.0], Align::Center, FONT_PILL, 12, PANEL)
        },
        i if i >= IDX_CONN_TREE && i < IDX_ROLE_BG => Spec { visible: false, ..rect(name, [CONN_STEM_X, conn_y(1), 2.0, 2.0], RULE, false) },
        IDX_ROLE_BG => Spec { visible: false, ..rect(name, [ROLE_X, TAB_Y, 2.0 * ROLE_TAB_W + 8.0, ROLE_H + 8.0], HIGHLIGHT, false) },
        IDX_ROLE_HI => Spec { visible: false, ..rect(name, [role_tab_x(0), TAB_Y, ROLE_TAB_W, ROLE_H], ACCENT, false) },
        i if i >= IDX_ROLE && i < IDX_ROLE_CAP => Spec {
            visible: false,
            ..text(name, [role_tab_x(i - IDX_ROLE), TAB_Y, ROLE_TAB_W, ROLE_H], Align::Center, FONT, 8, TEXT)
        },
        i if i >= IDX_ROLE_CAP && i < IDX_ROLE_KEY => {
            Spec { visible: false, ..rect(name, [ROLE_CAP_X[i - IDX_ROLE_CAP], TAB_Y, ROLE_CAP, ROLE_CAP], RULE, false) }
        }
        i if i >= IDX_ROLE_KEY && i < IDX_HEAD_SQ => Spec {
            visible: false,
            ..text(name, [ROLE_CAP_X[i - IDX_ROLE_KEY], TAB_Y, ROLE_CAP, ROLE_CAP], Align::Center, FONT_SMALL, 2, TEXT)
        },
        i if i >= IDX_HEAD_SQ && i < IDX_BADGE => {
            Spec { visible: false, ..rect(name, [HEAD_MARK_X, row_y(0), HEAD_SQ, HEAD_SQ], KIND_SEQUENCE, false) }
        }
        i if i >= IDX_BADGE && i < IDX_BADGE_NUM => {
            Spec { visible: false, ..rect(name, [BADGE_X, row_y(1), BADGE_W, BADGE_W], ACCENT, false) }
        }
        i if i >= IDX_BADGE_NUM && i < IDX_TAB_BOX => Spec {
            visible: false,
            ..text(name, [BADGE_X, row_y(1), BADGE_W + 8.0, BADGE_W + 6.0], Align::Center, FONT_PILL, 2, TEXT)
        },
        i if i >= IDX_TAB_BOX && i < MENU_COUNT => Spec { visible: false, ..rect(name, [0.0, POP_TAB_Y, 2.0, 2.0], RULE, false) },
        i if i >= IDX_TILE_BADGE && i < IDX_FLOPPY => Spec { visible: false, ..text(name, [0.0, 0.0, 52.0, MARK_H], Align::Center, FONT_SMALL, 4, TEXT) },
        IDX_TAB_HI => rect(name, [-740.0, TAB_Y, 200.0, TAB_H], ACCENT, true),
        IDX_TITLE => text(name, [600.0, TAB_Y, 560.0, TAB_H], Align::Right, FONT_SMALL, 40, TEXT_DIM),
        IDX_SEP => rect(name, [0.0, TAB_Y - 52.0, 1760.0, 3.0], RULE, true),
        IDX_CRUMB => text(name, [0.0, CRUMB_Y, 1760.0, 56.0], Align::Left, FONT, 120, TEXT),
        IDX_DIVIDER => rect(name, [DIVIDER_X, 20.0, 3.0, 650.0], RULE, true),
        IDX_GROUP_HI => rect(name, [GROUP_X, group_y(0), GROUP_W, GROUP_ROW_H], HIGHLIGHT, true),
        IDX_ROW_HI => rect(name, [(ROW_LEFT + ROW_RIGHT) / 2.0, row_y(0), ROW_RIGHT - ROW_LEFT + 30.0, ROW_H], HIGHLIGHT, true),
        i if i >= IDX_GREEN && i < IDX_ROW => rect(name, [0.0, 0.0, 8.0, 8.0], GREEN_ROW, false),
        IDX_SEP_HELP => rect(name, [0.0, HELP_Y[0] + 48.0, 1760.0, 3.0], RULE, true),
        IDX_HINT => text(name, [HELP_X, HINT_Y, HELP_W, 44.0], Align::Left, FONT_SMALL, 160, TEXT_DIM),
        i if i == IDX_HELP => text(name, [HELP_X, HELP_Y[0], HELP_W, 48.0], Align::Left, FONT_SMALL, 160, TEXT),
        i if i == IDX_HELP + 1 => text(name, [HELP_X, HELP_Y[1], HELP_W, 48.0], Align::Left, FONT_SMALL, 160, TEXT),
        i if i >= IDX_TAB && i < IDX_TAB + TABS => {
            text(name, [-840.0 + TAB_BOX_W / 2.0, TAB_Y, TAB_BOX_W, TAB_H], Align::Left, FONT, 40, TEXT_DIM)
        }
        i if i >= IDX_GROUP && i < IDX_GROUP + GROUPS => {
            text(name, [GROUP_X, group_y(i - IDX_GROUP), GROUP_W, GROUP_ROW_H], Align::Left, FONT, 48, TEXT_DIM)
        }
        i => {
            let row = (i - IDX_ROW) / 2;
            if (i - IDX_ROW).is_multiple_of(2) {
                text(name, [ROW_LEFT + ROW_LABEL_W / 2.0, row_y(row), ROW_LABEL_W, ROW_H], Align::Left, FONT, 80, TEXT)
            } else {
                text(name, [ROW_RIGHT - ROW_VALUE_W / 2.0, row_y(row), ROW_VALUE_W, ROW_H], Align::Right, FONT, 72, TEXT)
            }
        }
    }
}

/// Every pane of the menu, in tree (draw) order: back to front.
pub fn specs() -> Vec<Spec> {
    (0..COUNT).map(spec).collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_fit_the_bflyt_limit() {
        let all = specs();
        for (i, a) in all.iter().enumerate() {
            assert!(a.name.as_bytes().len() <= 24 && !a.name.as_bytes().is_empty());
            // The layout builder and the resolver read names as text: every byte must be printable ASCII
            // (2026-11-04: the roster marks' one-digit `name2` slot produced bytes above '9', invalid text).
            assert!(a.name.as_bytes().iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_'), "pane {i} has a non-text name {:?}", a.name.as_bytes());
            for b in &all[i + 1..] {
                assert_ne!(a.name.as_str(), b.name.as_str(), "{:?} is listed twice", a.name);
            }
        }
        assert_ne!(ROOT, name_of(IDX_BG));
        assert_eq!(all.len(), COUNT);
        for i in 0..COUNT {
            assert_eq!(all.iter().filter(|s| s.name == name_of(i)).count(), 1, "{i}");
        }
        for i in IDX_ITEM_ICON..IDX_ITEM_ICON + ITEM_ICONS {
            assert!(i < MENU_COUNT, "item pictures must be resolved in the menu pointer array");
            assert_eq!(spec(i).parent, ROOT);
            assert!(!spec(i).visible);
        }
        // P0 2026-11-04: the roster grid's marks are drawn through the menu pointer array too.
        for i in IDX_TILE_BADGE..IDX_TILE_SOLO + 2 {
            assert!(i < MENU_COUNT, "roster mark {i} must be resolved in the menu pointer array");
            assert_eq!(spec(i).parent, ROOT, "resolved under the menu container, by name");
            assert!(!spec(i).visible);
        }
        for i in IDX_FLOPPY..MENU_COUNT {
            assert!(i < MENU_COUNT, "recording mark {i} must be resolved in the menu pointer array");
            assert_eq!(spec(i).parent, ROOT, "resolved under the menu container, by name");
            assert!(!spec(i).visible);
        }
        assert_eq!(IDX_NOTICE, MENU_COUNT, "the input package's panes follow the menu's block");
    }

    /// Eden rc6-covrec-diag1 (2026-11-08): the pop-up's black panel was narrower and shorter than what the pop-up
    /// draws, so the row highlight and the MACROS loop count ran onto the stage, and the tabs, help and hint sat
    /// outside it. Primary 2026-11-08 13:05Z: the r8 mockup decides, and there everything is inside the panel.
    #[test]
    fn the_popup_panel_encloses_its_tabs_rows_help_and_hint() {
        let (half_w, half_h) = (POPUP_W / 2.0, POPUP_H / 2.0);
        let drawn = [IDX_TAB_HI, IDX_TAB, IDX_TITLE, IDX_SEP, IDX_CRUMB, IDX_ROW_HI, IDX_ROW, IDX_ROW + 1,
            IDX_ROW + 2 * (ROWS - 1), IDX_ROW + 2 * (ROWS - 1) + 1, IDX_SEP_HELP, IDX_HELP, IDX_HELP + 1, IDX_HINT];
        for index in drawn {
            let s = spec(index);
            assert!(s.x - s.w / 2.0 >= -half_w && s.x + s.w / 2.0 <= half_w, "{} leaves the panel sideways", s.name.as_str());
            assert!(s.y - s.h / 2.0 >= -half_h && s.y + s.h / 2.0 <= half_h, "{} leaves the panel", s.name.as_str());
        }
        // A loop count and a floppy are drawn left of the row's right end (`frame::p_row_x`), inside the highlight.
        assert!(ROW_RIGHT <= half_w);
    }

    #[test]
    fn the_pane_table_fits_with_the_stick_row() {
        let walk = include_str!("../../runtime/ui/pane.rs");
        let limit = walk
            .lines()
            .find(|line| line.contains("const WALK_LIMIT"))
            .and_then(|line| line.split('=').nth(1))
            .and_then(|rest| rest.trim().trim_end_matches(';').parse::<usize>().ok())
            .expect("WALK_LIMIT");
        assert_eq!(limit, 4096);
        assert_eq!(iv::at::COUNT - iv::at::STICK, iv::WINDOW, "one stick pane per frame");
        assert_eq!(iv::COUNT, 181);
        // The merged table already contains the stick row, the twelve recording icons and the
        // recording chrome (green plates, hairline, outlines, white cursor).
        assert_eq!(FLOPPIES + LOOPS, 12);
        assert_eq!(CHROME, 26);
        assert_eq!(name_of(IDX_FLOPPY).as_str(), "tr_floppy_00");
        assert_eq!(name_of(IDX_LOOP - 1).as_str(), "tr_floppy_09");
        assert_eq!(name_of(IDX_LOOP).as_str(), "tr_loop_00");
        assert_eq!(name_of(IDX_SUB - 1).as_str(), "tr_loop_01");
        // The r8 pop-up's sublines and CONNECTIONS column (primary 2026-11-08 #19, #49).
        let r8 = SUBS + 2 + CONN_LINES;
        assert_eq!(r8, 28);
        assert_eq!(name_of(IDX_SUB).as_str(), "tr_sub_00");
        assert_eq!(name_of(IDX_CONN_BOX - 1).as_str(), "tr_conn_13");
        // The r8 tree (boxes, pills, stem and ticks) and the role tabs (primary 2026-11-08 15:55Z).
        let r8_drawn = 4 * CONN_BOXES + 2 * CONN_BOXES + CONN_TREE + 8;
        assert_eq!(r8_drawn, 50);
        assert_eq!(name_of(IDX_ROLE_KEY + 1).as_str(), "tr_rkey_01");
        // The header squares, the order badges and the boxed page tabs (primary 2026-11-08 16:51Z).
        let r8_lists = 2 + 2 * BADGES + 4 * TAB_BOXES;
        assert_eq!(r8_lists, 26);
        assert_eq!(name_of(IDX_HEAD_SQ).as_str(), "tr_hsq_00");
        assert_eq!(name_of(IDX_BADGE).as_str(), "tr_obg_00");
        assert_eq!(name_of(IDX_BADGE_NUM + BADGES - 1).as_str(), "tr_obn_07");
        assert_eq!(name_of(MENU_COUNT - 1).as_str(), "tr_tbx_07");
        assert!(COUNT < limit, "plugin table {COUNT} exceeds the walk {limit}");
        assert_eq!(COUNT, 1424 + CHROME + r8 + r8_drawn + r8_lists);
        assert_eq!(limit - COUNT, 2672 - CHROME - r8 - r8_drawn - r8_lists);
        let strip_tree = iv::COUNT + cb::COUNT;
        assert!(strip_tree <= limit, "the strip walk, combo bar included, is {strip_tree}");
        let first = spec(IDX_IV + iv::at::STICK);
        let last = spec(IDX_IV + iv::at::COUNT - 1);
        assert_eq!(first.name.as_str(), "TrIvSk00");
        assert_eq!(last.name.as_str(), "TrIvSk59");
        assert_eq!(first.parent.as_str(), "TrIv");
        assert_eq!(last.parent.as_str(), "TrIv");
        assert!(!first.visible);
    }

    /// The r8 mockup (`RECORDING-REDESIGN-r8-popup-combo.png`): the lists on the left, CONNECTIONS on the right, a rule
    /// between them, everything inside the panel and above the help; a row's subline stays inside its plate.
    #[test]
    fn the_popup_has_the_r8_two_columns() {
        const { assert!(POP_ROW_RIGHT < CONN_DIVIDER_X && CONN_DIVIDER_X < CONN_LEFT, "the list, the rule, CONNECTIONS") };
        const { assert!(POP_ROW_LEFT >= -POPUP_W / 2.0 && CONN_RIGHT <= POPUP_W / 2.0) };
        let help_top = HELP_Y[0] + 48.0;
        for index in [IDX_CONN_HEAD, IDX_CONN, IDX_CONN + CONN_LINES - 1, IDX_CONN_DIV] {
            let s = spec(index);
            assert!(s.x - s.w / 2.0 >= CONN_DIVIDER_X - 2.0 && s.x + s.w / 2.0 <= CONN_RIGHT, "{:?} is in the right column", s.name);
            assert!(s.y - s.h / 2.0 > help_top && s.y + s.h / 2.0 <= ROW_FIRST_Y + ROW_H, "{:?} is above the help", s.name);
        }
        for row in 0..ROWS {
            let sub = spec(IDX_SUB + row);
            assert!(sub.x + sub.w / 2.0 <= POP_ROW_RIGHT, "a subline stays in the list column");
            let plate_bottom = row_y(row) - ROW_H / 2.0;
            assert!(sub.y - SUB_H / 2.0 >= plate_bottom - 2.0, "row {row}'s subline stays on its plate");
        }
        assert!(row_label_left(true) + row_label_w(true) <= POP_ROW_RIGHT - row_value_w(true), "the name ends before the value");
        // The role tabs and their key caps sit on the title line inside the panel, right of "Recording".
        for index in [IDX_ROLE_BG, IDX_ROLE_HI, IDX_ROLE, IDX_ROLE + 1, IDX_ROLE_CAP, IDX_ROLE_CAP + 1] {
            let s = spec(index);
            assert!(s.x - s.w / 2.0 >= 0.0 && s.x + s.w / 2.0 <= POPUP_W / 2.0, "{:?} sits right of the title", s.name);
            assert!((s.y - TAB_Y).abs() < 1.0, "{:?} is on the title line", s.name);
        }
        // The page tabs go under the title line and stay clear of the first row.
        assert!(POP_TAB_Y - TAB_H / 2.0 >= row_y(0) + ROW_H / 2.0 && POP_TAB_Y + TAB_H / 2.0 <= TAB_Y - ROLE_H / 2.0);
        // A box's pill and a joined file's indent fit the column.
        assert!(CONN_LEFT + CONN_INDENT + 8.0 + pill_w(10) + 120.0 <= CONN_RIGHT, "a SAVE STATE pill and a name fit");
        assert!(row_label_left(false) + row_label_w(false) <= ROW_RIGHT - row_value_w(false) + 1.0, "the page is unchanged");
        // A badge sits at the row's left edge, before the name; a header's mark sits before the header's name.
        const { assert!(BADGE_X + BADGE_W / 2.0 <= POP_ROW_LEFT + POP_LABEL_INSET && BADGE_X - BADGE_W / 2.0 >= POP_ROW_LEFT) };
        const { assert!(HEAD_MARK_X + 14.0 <= POP_ROW_LEFT + POP_LABEL_INSET + HEAD_INSET && HEAD_MARK_X - 14.0 >= POP_ROW_LEFT) };
        for index in [IDX_HEAD_SQ, IDX_BADGE, IDX_BADGE_NUM] {
            assert!(!spec(index).visible, "{:?} is hidden until drawn", spec(index).name);
        }
    }

    #[test]
    fn the_linear_conversion_darkens_and_keeps_alpha() {
        assert_eq!(linear([0, 255, 128, 77]), [0, 255, 55, 77]);
        assert!(linear([8, 10, 14, 236])[0] <= 1);
    }

    #[test]
    fn indexed_names_have_two_digits() {
        assert_eq!(name_of(tab_index(3)).as_str(), "tr_tab_03");
        assert_eq!(name_of(row_value_index(11)).as_str(), "tr_rv_11");
    }

    #[test]
    fn every_pane_lies_inside_the_panel() {
        for s in specs().into_iter().filter(|s| s.parent == ROOT) {
            assert!(s.x - s.w / 2.0 >= -PANEL_W / 2.0 - 1.0 && s.x + s.w / 2.0 <= PANEL_W / 2.0 + 1.0, "{:?} x", s.name);
            assert!(s.y - s.h / 2.0 >= -PANEL_H / 2.0 - 1.0 && s.y + s.h / 2.0 <= PANEL_H / 2.0 + 1.0, "{:?} y", s.name);
        }
    }

    #[test]
    fn the_rows_do_not_run_into_the_help_panel() {
        assert!(row_y(ROWS - 1) - ROW_H / 2.0 > HELP_Y[0] + 48.0 + 4.0);
    }
}
