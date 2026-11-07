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

pub const fn row_y(row: usize) -> f32 {
    ROW_FIRST_Y - ROW_PITCH * row as f32
}

pub const fn group_y(group: usize) -> f32 {
    ROW_FIRST_Y - GROUP_ROW_H * group as f32
}

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
/// Row `i`'s label is `IDX_ROW + 2 * i` and its value `IDX_ROW + 2 * i + 1`.
pub const IDX_ROW: usize = IDX_ROW_HI + 1;
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
/// How many panes the menu itself owns and the runtime resolves at registration (`view::Panes::all`); every index
/// the renderer writes is below it. The panes after these (the notification panel and the frame strip) belong to the
/// input package, which finds them by name when it draws.
pub const MENU_COUNT: usize = IDX_LOOP + LOOPS;
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
        use super::{CHIPS, HISTORY, RUNS, RUN_LABELS};
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
        pub const COUNT: usize = MOVE_TEXT + HISTORY;
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
    pub const P1_Y: f32 = 21.75;
    pub const CPU_Y: f32 = -14.25;
    pub const LANE_H: f32 = 28.5;
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
    use iv::{CHIPS, HISTORY, RUNS, RUN_LABELS};
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
        i if i < MOVE_TEXT + HISTORY => Name::indexed("TrIvMove", i - MOVE_TEXT),
        _ => {
            let _ = (RUNS, RUN_LABELS);
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
        LANE_H, LAST_Y, LEG_H, LEG_Y, NOW_X, PILL_W, READ_SUB_X, READ_X, RULE_Y, STRIP_H, STRIP_W, STRIP_Y, TEXT_UNITS,
        TICK_H, TICK_Y, WINDOW_H, WINDOW_Y,
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
        _ => iv_text(name, [0.0, LAST_Y, 200.0], Align::Left, FONT_TAG),
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
        i if i >= IDX_LOOP && i < MENU_COUNT => Name::indexed("tr_loop_", i - IDX_LOOP),
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
        i if i >= IDX_FLOPPY && i < MENU_COUNT => Spec { visible: false, ..rect(name, [0.0, 0.0, 28.0, 28.0], [255, 255, 255, 255], false) },
        i if i >= IDX_TILE_BADGE && i < IDX_FLOPPY => Spec { visible: false, ..text(name, [0.0, 0.0, 52.0, MARK_H], Align::Center, FONT_SMALL, 4, TEXT) },
        IDX_TAB_HI => rect(name, [-740.0, TAB_Y, 200.0, TAB_H], ACCENT, true),
        IDX_TITLE => text(name, [600.0, TAB_Y, 560.0, TAB_H], Align::Right, FONT_SMALL, 40, TEXT_DIM),
        IDX_SEP => rect(name, [0.0, TAB_Y - 52.0, 1760.0, 3.0], RULE, true),
        IDX_CRUMB => text(name, [0.0, CRUMB_Y, 1760.0, 56.0], Align::Left, FONT, 120, TEXT),
        IDX_DIVIDER => rect(name, [DIVIDER_X, 20.0, 3.0, 650.0], RULE, true),
        IDX_GROUP_HI => rect(name, [GROUP_X, group_y(0), GROUP_W, GROUP_ROW_H], HIGHLIGHT, true),
        IDX_ROW_HI => rect(name, [(ROW_LEFT + ROW_RIGHT) / 2.0, row_y(0), ROW_RIGHT - ROW_LEFT + 30.0, ROW_H], HIGHLIGHT, true),
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
