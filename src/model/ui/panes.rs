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
pub const CLIP_STRIP: [f32; 4] = [0.0, -390.0, 1920.0, 300.0];
pub const CLIP_ROW_Y: [f32; 2] = [-268.0, -314.0];
pub const CLIP_BAR_Y: f32 = -362.0;
pub const CLIP_HELP_Y: [f32; 2] = [-408.0, -454.0];
pub const CLIP_HINT_Y: f32 = -500.0;
/// The strip's two text columns, centre x and width: the label starts at the timeline's left end and the value ends at its right end.
pub const CLIP_LABEL_BOX: (f32, f32) = (-450.0, 700.0);
pub const CLIP_VALUE_BOX: (f32, f32) = (360.0, 880.0);
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

/// Inner padding between a list's chrome (the outline's and the cursor's edges, the cursor's left bar) and the row text,
/// on both sides (rc.14 list-padding, GIF review of `8199c696`: the first letters ran under the box's left edge and "3
/// targets" touched its right). Every list row is placed through `row_label_box` and `row_value_box`, so no list loses it.
pub const ROW_PAD: f32 = 20.0;
/// The outline's and the cursor's edge width, and the white cursor's left bar: its centre inside the span's left edge and
/// its width.
pub const CHROME_EDGE: f32 = 2.0;
pub const CURSOR_BAR_X: f32 = 4.0;
pub const CURSOR_BAR_W: f32 = 6.0;

/// A row's label box in the column, centre x and width: the page's starts one pad inside the column's left edge; the
/// pop-up's starts past the order badge's room (wider than the pad) and leaves what its narrower column has beside a short
/// value. Both stop where the value's box starts, less a gap.
pub const fn row_label_box(popup: bool) -> (f32, f32) {
    let left = if popup { POP_ROW_LEFT + POP_LABEL_INSET } else { ROW_LEFT + ROW_PAD };
    let width = if popup { POP_ROW_RIGHT - POP_ROW_LEFT - POP_LABEL_INSET - POP_VALUE_W - 16.0 } else { ROW_LABEL_W - ROW_PAD };
    (left + width / 2.0, width)
}

/// A row's value box, centre x and width: the right-aligned text ends one pad inside the column's right edge, and the
/// box's left edge stays where it was.
pub const fn row_value_box(popup: bool) -> (f32, f32) {
    let width = (if popup { POP_VALUE_W } else { ROW_VALUE_W }) - ROW_PAD;
    (row_value_right(popup) - width / 2.0, width)
}

/// Where a row's value text ends; what sits left of it (a floppy, a loop mark) is placed from here.
pub const fn row_value_right(popup: bool) -> f32 {
    row_span(popup).1 - ROW_PAD
}

/// Room for the order badge, and for a section header's mark, before the names (r8: the names start past both).
pub const POP_LABEL_INSET: f32 = 42.0;
/// The pop-up's red order badges (RECORDING-REDESIGN.md "Red number, at the row's left edge"), one per list row of the
/// two sections (four rows each).
pub const BADGES: usize = 8;
pub const BADGE_W: f32 = 24.0;
pub const BADGE_X: f32 = POP_ROW_LEFT + 20.0;
pub const HEAD_MARK_X: f32 = POP_ROW_LEFT + 20.0;
pub const HEAD_SQ: f32 = 16.0;
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
pub const ROLE_CAP_ZR: f32 = 56.0;
pub const ROLE_CAP_X: [f32; 2] = [ROLE_X - ROLE_TAB_W - 40.0, ROLE_X + ROLE_TAB_W + 40.0];

/// Key caps beside the role tabs: L/R in the pop-up, ZR on the page, none while a file or picker owns the buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoleKeys {
    #[default]
    None,
    LR,
    ZR,
}

pub const fn role_caps(keys: RoleKeys) -> [Option<(&'static [u16], f32)>; 2] {
    match keys {
        RoleKeys::None => [None, None],
        RoleKeys::LR => [Some((&[76], ROLE_CAP)), Some((&[82], ROLE_CAP))],
        RoleKeys::ZR => [None, Some((&[90, 82], ROLE_CAP_ZR))],
    }
}

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
/// rc.13 E1 x1: a needle per layer of the game's DI effect, one block of `CHIPS` each, back to front glow, body,
/// streaks, core. The first block also carries the SDI burst and the arrows.
pub const CHIP_LAYERS: usize = 4;
pub const IDX_CHIP_NEEDLE: usize = IDX_CHIP + CHIPS;
pub const IDX_CHIP_LABEL: usize = IDX_CHIP_NEEDLE + CHIP_LAYERS * CHIPS;
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
    #[cfg(test)]
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
/// One picture pane per give-item tile (`tr_item_NN`), rc.13's Wario bike (38) included.
pub const ITEM_ICONS: usize = 39;
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
/// The notification panel (SPEC-input-05): its own container under `RootPane`. Since 2026-11-17 (`model::notice_box`) the
/// first plate is the one dark plate behind the header and the body, sized to them; the other two stay hidden. The
/// plates come first and the three texts after them: a plate drawn after a text covers it (Eden covrest-02,
/// 2026-11-08). Written by the notices code of the input package.
pub const IDX_NOTICE: usize = MENU_COUNT;
pub const IDX_NOTICE_HEADER_PLATE: usize = IDX_NOTICE + 1;
pub const IDX_NOTICE_TEXT_PLATE: usize = IDX_NOTICE + 2;
pub const IDX_NOTICE_LONG_PLATE: usize = IDX_NOTICE + 3;
pub const IDX_NOTICE_HEADER: usize = IDX_NOTICE + 4;
pub const IDX_NOTICE_TEXT: usize = IDX_NOTICE + 5;
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
/// layout builder makes it RootPane's first child in both layouts, so every other pane draws over it) and `ORBS` pictures
/// tinted by their vertex colours. Each orb is a pill (decision HB-PILLS): the first `CAPS` pictures take the plugin's
/// half-disc texture (`tr_orb_cap`, drawn by the layout builder), the `BANDS` after them its band (`tr_orb_band`). A pill
/// is two caps and a band, so 170 pills.
pub const ORBS: usize = 512;
pub const CAPS: usize = 340;
pub const BANDS: usize = ORBS - CAPS;
pub const IDX_ORBS: usize = IDX_CB + cb::COUNT;
pub const IDX_ORB: usize = IDX_ORBS + 1;
/// The orb panes' names: `tr_oa_00` .. `tr_of_11`, one prefix per hundred.
const ORB_PREFIXES: [&str; 6] = ["tr_oa_", "tr_ob_", "tr_oc_", "tr_od_", "tr_oe_", "tr_of_"];
/// The practice games' pictures (rc.14), under `RootPane` and drawn under the orbs, each in its own texture's colours
/// (`assets/egg-target/`, drawn by `tools/draw_egg_target.py`): `EGGS` targets (`tr_egg_00`.., `target.png`; the SPEC's
/// maximum of 10), `BURSTS` break pictures (`tr_eggcr_00`.., `cracked.png`; at most 2 breaks at once) and the placing
/// cursor (`tr_egg_cursor`, `cursor.png`), drawn above the targets.
pub const EGGS: usize = 10;
pub const BURSTS: usize = 2;
pub const EGG_PANES: usize = EGGS + BURSTS + 1;
pub const IDX_EGG: usize = IDX_ORB + ORBS;
/// Each `EggPicture`'s panes: (first, count) after `IDX_EGG`.
pub const EGG_POOLS: [(usize, usize); 3] = [(0, EGGS), (EGGS, BURSTS), (EGGS + BURSTS, 1)];
/// Compact Break the Targets HUD: container, plate, existing target picture, one-line text.
pub const IDX_BTT: usize = IDX_EGG + EGG_PANES;
/// How many panes the plugin adds to the layout.
#[cfg(test)]
pub const COUNT: usize = IDX_BTT + 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EggPicture {
    Whole,
    Cracked,
    Cursor,
}

/// One picture this frame: `scale` (its size over the pill's), `alpha` (the pane's) and which picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EggLook {
    pub scale: f32,
    pub alpha: u8,
    pub picture: EggPicture,
}

pub const EGG_WHOLE: EggLook = EggLook { scale: 1.0, alpha: 255, picture: EggPicture::Whole };

/// The combo bar's pane indices and geometry (canvas units; the bar's centre is the origin of its panes).
pub mod cb {
    /// Link cells and result pills.
    pub const CELLS: usize = 4;
    pub const PILLS: usize = 8;
    /// A drilled sequence's combos shown at once, and the arrows between them.
    pub const CHIPS: usize = 3;

    pub mod at {
        use super::{CELLS, CHIPS, PILLS};
        pub const CONTAINER: usize = 0;
        pub const BACKDROP: usize = 1;
        pub const HEADER: usize = 2;
        pub const STREAK: usize = 3;
        pub const RESULT: usize = SEQ_BACKDROP + 1;
        pub const REASON: usize = RESULT + 1;
        pub const PILL: usize = STREAK + 1;
        /// Per cell, one block of `CELLS` each: backdrop, title, verdict, offset, ruler, window, recorded and your press.
        pub const CELL: usize = PILL + PILLS;
        pub const TITLE: usize = CELL + CELLS;
        pub const VERDICT: usize = TITLE + CELLS;
        pub const OFFSET: usize = VERDICT + CELLS;
        pub const RULER: usize = OFFSET + CELLS;
        pub const WINDOW: usize = RULER + CELLS;
        pub const RECORDED: usize = WINDOW + CELLS;
        pub const YOURS: usize = RECORDED + CELLS;
        /// A drilled sequence (docs/RECORDING.md § Sequences and the one drill; mockup `r8-sequence-bar`): the band
        /// above the bar for the sequence header and the row of its combos.
        pub const SEQ_BACKDROP: usize = YOURS + CELLS;
        /// The header: the SEQUENCE pill and its word, the name, "combo k of n ·", the combo, "· N links",
        /// "· tolerance ±1f", and on the right "streak N" and "· best M".
        pub const SEQ_PILL: usize = REASON + 1;
        pub const SEQ_PILL_TEXT: usize = SEQ_PILL + 1;
        pub const SEQ_NAME: usize = SEQ_PILL_TEXT + 1;
        pub const SEQ_STEP: usize = SEQ_NAME + 1;
        pub const SEQ_LINKS: usize = SEQ_STEP + 1;
        pub const SEQ_TOLERANCE: usize = SEQ_LINKS + 1;
        pub const SEQ_STREAK: usize = SEQ_TOLERANCE + 1;
        pub const SEQ_BEST: usize = SEQ_STREAK + 1;
        /// The footer's start: "Ryu BnB landed ·", and the current combo's name before the helper's own result.
        pub const SEQ_LANDED: usize = SEQ_BEST + 1;
        /// Per combo chip, one block of `CHIPS` each: outline, fill, text, and the two strokes of the check mark
        /// (the game's fonts have no ✓).
        pub const SEQ_CHIP_EDGE: usize = SEQ_LANDED + 1;
        pub const SEQ_CHIP_FILL: usize = SEQ_CHIP_EDGE + CHIPS;
        pub const SEQ_CHIP_TEXT: usize = SEQ_CHIP_FILL + CHIPS;
        pub const SEQ_CHECK_SHORT: usize = SEQ_CHIP_TEXT + CHIPS;
        pub const SEQ_CHECK_LONG: usize = SEQ_CHECK_SHORT + CHIPS;
        /// The arrows between the chips.
        pub const COUNT: usize = SEQ_CHECK_LONG + CHIPS;
    }

    pub const COUNT: usize = at::COUNT;

    /// Sequence mode (mockup `r8-sequence-bar`): the bar is as wide as the strip, with a band above the combo bar's
    /// rows. The container moves down by the band, so the bar's top stays under the strip.
    pub const SEQ_PILL_COLOR: [u8; 4] = [236, 108, 176, 255];
    pub const SEQ_PILL_TEXT_COLOR: [u8; 4] = [24, 18, 30, 255];
    pub const SEQ_CHIP_COLOR: [u8; 4] = [32, 38, 52, 255];

    /// The bar's centre below the strip's, and its size (the mockup's 864 × 132).
    pub const BACKDROP_COLOR: [u8; 4] = super::iv::BACKDROP_COLOR;
    pub const CELL_COLOR: [u8; 4] = [30, 36, 48, 255];

    /// A line's capacity in UTF-16 units, by its pane: the card's table, read by its native boxes and by its pane
    /// cache. The reason, the link titles and the marks under a step carry sentences longer than a short line.
    pub const fn units(local: usize) -> u16 {
        use at::*;
        match local {
            REASON => super::iv::TEXT_UNITS,
            i if (i >= TITLE && i < VERDICT) || (i >= OFFSET && i < RULER) => super::iv::TEXT_UNITS,
            _ => super::iv::SHORT_UNITS,
        }
    }

    /// The card's lines that are longer than a short line.
    pub const LONG_LINES: usize = super::long_lines(false);
}

#[path = "strip_layout/mod.rs"]
pub mod strip_layout;

/// The frame strip's pane indices and geometry (canvas units, the container's centre is the origin).
pub mod iv {
    /// Frames on screen, links in the LAST row, lane runs and run lengths per lane, chips.
    pub const TAGS: usize = 8;
    pub const WINDOW: usize = 60;
    pub const HISTORY: usize = 4;
    pub const RUNS: usize = 16;
    pub const RUN_LABELS: usize = 4;
    pub const CHIPS: usize = 8;

    /// The strip's pane indices (local: `IDX_IV + at::…` in the table).
    pub mod at {
        use super::{CHIPS, HISTORY, RUNS, RUN_LABELS, TAGS, WINDOW};
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
        pub const TAG_BOX: usize = STICK + WINDOW;
        pub const TAG_TEXT: usize = TAG_BOX + TAGS;
        pub const COUNT: usize = TAG_TEXT + TAGS;
    }

    pub const COUNT: usize = at::COUNT;

    pub use super::strip_layout::strip::*;
    pub const BACKDROP_COLOR: [u8; 4] = [14, 18, 27, 222];
    /// A long line's capacity in UTF-16 units: the longest line a plan writes (the card's reason, 37 units) fits, so
    /// it is cut neither by its box nor by `frame_strip_plan::Text`. Which lines are long is `units` here and in `cb`.
    pub const TEXT_UNITS: u16 = 40;
    /// Every other line's capacity.
    pub const SHORT_UNITS: u16 = 24;

    /// A line's capacity in UTF-16 units, by its pane: the strip's table, read by its native boxes and by its pane
    /// cache. The readouts' sub-lines carry the folded link and Parry's grade words; a chip and a stick mark are one glyph.
    pub const fn units(local: usize) -> u16 {
        use at::*;
        match local {
            i if (i >= CHIP_GLYPH && i < CHIP_SHAFT) || (i >= STICK && i < TAG_BOX) => 2,
            i if i >= READ_SUB && i < LAST_TEXT => TEXT_UNITS,
            _ => SHORT_UNITS,
        }
    }

    /// The strip's lines that are longer than a short line.
    pub const LONG_LINES: usize = super::long_lines(true);
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
        i if i < TAG_BOX => Name::indexed("TrIvSk", i - STICK),
        i if i < TAG_TEXT => Name::indexed("TrIvTagB", i - TAG_BOX),
        i if i < COUNT => Name::indexed("TrIvTagT", i - TAG_TEXT),
        _ => {
            let _ = (RUNS, RUN_LABELS, WINDOW);
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
        SEQ_BACKDROP => Name::new("TrCbSqBg"),
        SEQ_PILL => Name::new("TrCbSqPill"),
        SEQ_PILL_TEXT => Name::new("TrCbSqKind"),
        SEQ_NAME => Name::new("TrCbSqName"),
        SEQ_STEP => Name::new("TrCbSqStep"),
        SEQ_LINKS => Name::new("TrCbSqLinks"),
        SEQ_TOLERANCE => Name::new("TrCbSqTol"),
        SEQ_STREAK => Name::new("TrCbSqStrk"),
        SEQ_BEST => Name::new("TrCbSqBest"),
        SEQ_LANDED => Name::new("TrCbSqDone"),
        i if i >= SEQ_CHIP_EDGE && i < COUNT => {
            let n = (i - SEQ_CHIP_EDGE) % cb::CHIPS;
            match (i - SEQ_CHIP_EDGE) / cb::CHIPS {
                0 => Name::indexed("TrCbSqEdge", n),
                1 => Name::indexed("TrCbSqFill", n),
                2 => Name::indexed("TrCbSqChip", n),
                3 => Name::indexed("TrCbSqTick", n),
                _ => Name::indexed("TrCbSqCheck", n),
            }
        }
        i if i < SEQ_BACKDROP => {
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
const fn cb_text(name: Name, size: [f32; 4], align: Align, font: (f32, f32)) -> Spec {
    hidden(under(text(name, size, align, font, iv::SHORT_UNITS, TEXT), Name::new("TrCb")))
}

/// The combo bar's panes: kind, place and size, all hidden until the helper draws them.
const fn cb_spec(layout: &strip_layout::LayoutSpec, local: usize, name: Name) -> Spec {
    use cb::at::*;
    use cb::{CELL_COLOR, SEQ_CHIP_COLOR, SEQ_PILL_COLOR, SEQ_PILL_TEXT_COLOR};
    use iv::{FONT_SUB, FONT_TAG};
    let parent = Name::new("TrCb");
    let (place, h) = layout.card.at_rest(local, &layout.strip);
    let size = [place.x, place.y, place.w, h];
    match local {
        CONTAINER => container(name, Name::new("TrIv"), size),
        BACKDROP | SEQ_BACKDROP => hidden(under(rect(name, size, cb::BACKDROP_COLOR, true), parent)),
        HEADER | RESULT | REASON | SEQ_NAME | SEQ_LANDED =>
            cb_text(name, size, Align::Left, FONT_SUB),
        STREAK | SEQ_STEP | SEQ_TOLERANCE => cb_text(name, size, Align::Right, FONT_TAG),
        SEQ_LINKS => cb_text(name, size, Align::Left, FONT_TAG),
        SEQ_STREAK | SEQ_BEST => cb_text(name, size, Align::Left, FONT_SUB),
        SEQ_PILL => under(rect(name, size, SEQ_PILL_COLOR, false), parent),
        SEQ_PILL_TEXT => hidden(under(text(name, size, Align::Center, FONT_TAG, iv::SHORT_UNITS, SEQ_PILL_TEXT_COLOR), parent)),
        i if i < CELL => under(rect(name, size, TEXT, false), parent),
        i if i < TITLE => under(rect(name, size, CELL_COLOR, false), parent),
        i if i < RULER => cb_text(name, size, Align::Center, FONT_TAG),
        i if i < WINDOW => under(rect(name, size, RULE, false), parent),
        i if i < SEQ_BACKDROP || (SEQ_CHIP_EDGE <= i && i < SEQ_CHIP_FILL)
            || (SEQ_CHECK_SHORT <= i && i < COUNT) => under(rect(name, size, TEXT, false), parent),
        i if i < SEQ_CHIP_TEXT => under(rect(name, size, SEQ_CHIP_COLOR, false), parent),
        i if i < SEQ_CHECK_SHORT => cb_text(name, size, Align::Left, FONT_SUB),
        _ => panic!("no such combo bar pane"),
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
        i if i >= IDX_EGG && i < IDX_EGG + EGGS => Name::indexed("tr_egg_", i - IDX_EGG),
        i if i >= IDX_EGG + EGGS && i < IDX_EGG + EGGS + BURSTS => Name::indexed("tr_eggcr_", i - IDX_EGG - EGGS),
        i if i == IDX_EGG + EGGS + BURSTS => Name::new("tr_egg_cursor"),
        IDX_BTT => Name::new("TrBtt"),
        i if i == IDX_BTT + 1 => Name::new("TrBttPlate"),
        i if i == IDX_BTT + 2 => Name::new("TrBttChip"),
        i if i == IDX_BTT + 3 => Name::new("TrBttText"),
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
        i if i >= IDX_CHIP_NEEDLE && i < IDX_CHIP_LABEL => Name::indexed("tr_cn_", i - IDX_CHIP_NEEDLE),
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
pub const NOTICE_HEADER: [u8; 4] = TEXT;
/// The compact notice box (2026-11-17, `model::notice_box`): the plate's top-left corner, 16 px from the left and 8 px
/// from the top at 1280 × 720 (the BTT chip's corner); its widest, 8 px short of the frame strip; and its two fonts.
pub const NOTICE_LEFT: f32 = -936.0;
pub const NOTICE_TOP: f32 = 528.0;
pub const NOTICE_MAX_W: f32 = -iv::STRIP_W / 2.0 - 12.0 - NOTICE_LEFT;
/// Lobby practice (`Scene::Lobby`, the Local Wireless waiting room): the native "Leave Battle" panel (top left) and the
/// "Waiting…" / "L+R Start" pill (top right) reach down to y ≈ 447, so the frame strip, the BTT chip and the notice
/// anchor move down by this there and end 15 units under them (2026-11-17). Training is unchanged.
pub const LOBBY_DROP: f32 = strip_layout::C.lobby_drop;
pub const NOTICE_HEADER_FONT: (f32, f32) = (15.0, 24.0);
pub const NOTICE_BODY_FONT: (f32, f32) = (17.0, 26.0);
/// Where the notice panes start, before the HUD draw sizes them to the notice.
const NOTICE_BOX: [f32; 4] = [NOTICE_LEFT + NOTICE_MAX_W / 2.0, NOTICE_TOP - 40.0, NOTICE_MAX_W, 80.0];

/// `[x, y, w, h]`: a pane's place and size, the order `Spec` keeps them in.
const fn rect(name: Name, [x, y, w, h]: [f32; 4], color: [u8; 4], visible: bool) -> Spec {
    Spec { name, parent: ROOT, kind: Kind::Rect { color }, x, y, w, h, visible }
}

const fn text(name: Name, [x, y, w, h]: [f32; 4], align: Align, font: (f32, f32), chars: u16, color: [u8; 4]) -> Spec {
    Spec { name, parent: ROOT, kind: Kind::Text { align, font, chars, color }, x, y, w, h, visible: true }
}

const ROOT_PANE: Name = Name::new("RootPane");

const fn container(name: Name, parent: Name, [x, y, w, h]: [f32; 4]) -> Spec {
    Spec { name, parent, kind: Kind::Null, x, y, w, h, visible: false }
}

const fn under(mut spec: Spec, parent: Name) -> Spec {
    spec.parent = parent;
    spec
}

const fn hidden(spec: Spec) -> Spec {
    Spec { visible: false, ..spec }
}

/// Top-left compact HUD, in canvas units; the text and target chip stay on one line.
pub fn btt_hud_box(body: &str, strip: bool, bar: bool) -> [f32; 4] {
    let wide = body.chars().count() > 18;
    let width = if wide { (body.chars().count() as f32 * 13.0 + 64.0).clamp(600.0, 1000.0) } else { 276.0 };
    let y = if wide && strip {
        let spec = &strip_layout::C.strip;
        let [_, y, _, h] = spec.bounds(true, bar);
        spec.origin_y + y - h / 2.0 - 32.0
    } else {
        504.0
    };
    [-936.0, y, width, 48.0]
}

/// Archive placement for a strip or card pane, using the same spec as the live HUD. A text box is declared as long
/// as its line's capacity (`iv::units`, `cb::units`).
pub const fn strip_pane_spec(layout: &strip_layout::LayoutSpec, index: usize) -> Spec {
    let name = name_of(index);
    if index < IDX_CB {
        lined(strip_layout::strip::iv_spec(&layout.strip, index - IDX_IV, name), iv::units(index - IDX_IV))
    } else {
        lined(cb_spec(layout, index - IDX_CB, name), cb::units(index - IDX_CB))
    }
}

/// A text box declared `units` UTF-16 units long; any other pane as it is.
const fn lined(mut spec: Spec, units: u16) -> Spec {
    if let Kind::Text { align, font, color, .. } = spec.kind {
        spec.kind = Kind::Text { align, font, chars: units, color };
    }
    spec
}

/// How many panes of a bar carry a long line by its capacity table: the strip's (`strip`) or the card's.
const fn long_lines(strip: bool) -> usize {
    let count = if strip { iv::COUNT } else { cb::COUNT };
    let mut long = 0;
    let mut local = 0;
    while local < count {
        let units = if strip { iv::units(local) } else { cb::units(local) };
        if units > iv::SHORT_UNITS {
            long += 1;
        }
        local += 1;
    }
    long
}

/// Pane `index`'s kind and place.
pub const fn spec(index: usize) -> Spec {
    let name = name_of(index);
    match index {
        i if i >= IDX_ITEM_ICON && i < IDX_ITEM_ICON + ITEM_ICONS => rect(name, [0.0, 0.0, 64.0, 64.0], [255; 4], false),
        IDX_ORBS => container(name, ROOT_PANE, [0.0, 0.0, 1920.0, 1080.0]),
        i if i >= IDX_ORB && i < IDX_ORB + ORBS => under(rect(name, [0.0, 0.0, 64.0, 64.0], [255; 4], false), name_of(IDX_ORBS)),
        i if i >= IDX_EGG && i < IDX_EGG + EGG_PANES => under(rect(name, [0.0, 0.0, 64.0, 64.0], [255; 4], false), ROOT_PANE),
        IDX_BTT => container(name, ROOT_PANE, [-936.0, 504.0, 1000.0, 48.0]),
        i if i == IDX_BTT + 1 => under(rect(name, [138.0, 0.0, 276.0, 48.0], PANEL, true), name_of(IDX_BTT)),
        i if i == IDX_BTT + 2 => under(rect(name, [26.0, 0.0, 32.0, 32.0], [255; 4], true), name_of(IDX_BTT)),
        i if i == IDX_BTT + 3 => under(text(name, [158.0, 0.0, 212.0, 40.0], Align::Left, (13.0, 24.0), 96, TEXT), name_of(IDX_BTT)),
        IDX_MENU => container(name, ROOT_PANE, [0.0, 0.0, 1920.0, 1080.0]),
        IDX_NOTICE => container(name, ROOT_PANE, [0.0, 0.0, 1920.0, 1080.0]),
        IDX_NOTICE_HEADER_PLATE => under(rect(name, NOTICE_BOX, NOTICE_PLATE, true), name_of(IDX_NOTICE)),
        IDX_NOTICE_HEADER => under(text(name, NOTICE_BOX, Align::Left, NOTICE_HEADER_FONT, 24, NOTICE_HEADER), name_of(IDX_NOTICE)),
        IDX_NOTICE_TEXT_PLATE => under(rect(name, NOTICE_BOX, NOTICE_PLATE, false), name_of(IDX_NOTICE)),
        IDX_NOTICE_TEXT => under(text(name, NOTICE_BOX, Align::Left, NOTICE_BODY_FONT, 48, TEXT), name_of(IDX_NOTICE)),
        IDX_NOTICE_LONG_PLATE => under(rect(name, NOTICE_BOX, NOTICE_PLATE, false), name_of(IDX_NOTICE)),
        IDX_NOTICE_LONG => under(hidden(text(name, NOTICE_BOX, Align::Left, NOTICE_BODY_FONT, NOTICE_LONG_UNITS, TEXT)), name_of(IDX_NOTICE)),
        IDX_BG => rect(name, [0.0, 0.0, PANEL_W, PANEL_H], PANEL, true),
        i if i >= IDX_IV && i < IDX_ORBS => strip_pane_spec(&strip_layout::C, i),
        IDX_CHIP_HI => rect(name, [0.0, 0.0, CHIP_SIZE + 16.0, CHIP_SIZE + 16.0], ACCENT, false),
        IDX_TILE_HI => rect(name, [0.0, 0.0, 100.0, 60.0], ACCENT, false),
        i if i >= IDX_CHIP && i < IDX_CHIP + CHIPS => rect(name, [0.0, 0.0, CHIP_SIZE, CHIP_SIZE], HIGHLIGHT, false),
        i if i >= IDX_CHIP_NEEDLE && i < IDX_CHIP_LABEL => rect(name, [0.0, 0.0, 10.0, 84.0], TEXT, false),
        i if i >= IDX_CHIP_LABEL && i < IDX_CHIP_LABEL + CHIPS => hidden(text(name, [0.0, 0.0, CHIP_SIZE, 44.0], Align::Center, FONT_SMALL, 16, TEXT)),
        i if i >= IDX_CHIP_BADGE && i < IDX_CHIP_BADGE + CHIPS => hidden(text(name, [0.0, 0.0, 60.0, 44.0], Align::Center, FONT, 4, TEXT)),
        i if i >= IDX_TILE && i < IDX_TILE + TILES => rect(name, [0.0, 0.0, 100.0, 60.0], HIGHLIGHT, false),
        i if i >= IDX_TILE_LABEL && i < IDX_TILE_LABEL + TILES => hidden(text(name, [0.0, 0.0, 100.0, TILE_LABEL_H], Align::Center, FONT_SMALL, 28, TEXT)),
        i if i >= IDX_TILE_STRIPE && i < IDX_TILE_SOLO => hidden(rect(name, [0.0, 0.0, 4.0, 86.0], TEXT, false)),
        i if i >= IDX_FLOPPY && i < IDX_SUB => hidden(rect(name, [0.0, 0.0, 28.0, 28.0], [255, 255, 255, 255], false)),
        i if i >= IDX_SUB && i < IDX_CONN_DIV => {
            let (x, w) = row_label_box(true);
            hidden(text(name, [x, row_y(i - IDX_SUB) - SUB_DROP, w, SUB_H], Align::Left, FONT_SUBLINE, 64, TEXT_DIM))
        }
        IDX_CONN_DIV => hidden(rect(name, [CONN_DIVIDER_X, 15.0, 3.0, 640.0], RULE, false)),
        IDX_CONN_HEAD => hidden(text(name, [(CONN_LEFT + CONN_RIGHT) / 2.0, ROW_FIRST_Y, CONN_RIGHT - CONN_LEFT, CONN_LINE_H], Align::Left, FONT_SMALL, 48, TEXT_DIM)),
        i if i >= IDX_CONN && i < IDX_CONN_BOX => hidden(text(name, [(CONN_LEFT + CONN_RIGHT) / 2.0, conn_y(i - IDX_CONN), CONN_RIGHT - CONN_LEFT, CONN_LINE_H], Align::Left, FONT_SMALL, 72, TEXT)),
        i if i >= IDX_CONN_BOX && i < IDX_CONN_PILL_TEXT => {
            hidden(rect(name, [(CONN_LEFT + CONN_RIGHT) / 2.0, conn_y(0), 8.0, 8.0], TEXT, false))
        }
        i if i >= IDX_CONN_PILL_TEXT && i < IDX_CONN_TREE => hidden(text(name, [CONN_LEFT + 80.0, conn_y(0), 160.0, PILL_H + 4.0], Align::Center, FONT_PILL, 12, PANEL)),
        i if i >= IDX_CONN_TREE && i < IDX_ROLE_BG => hidden(rect(name, [CONN_STEM_X, conn_y(1), 2.0, 2.0], RULE, false)),
        IDX_ROLE_BG => hidden(rect(name, [ROLE_X, TAB_Y, 2.0 * ROLE_TAB_W + 8.0, ROLE_H + 8.0], HIGHLIGHT, false)),
        IDX_ROLE_HI => hidden(rect(name, [role_tab_x(0), TAB_Y, ROLE_TAB_W, ROLE_H], ACCENT, false)),
        i if i >= IDX_ROLE && i < IDX_ROLE_CAP => hidden(text(name, [role_tab_x(i - IDX_ROLE), TAB_Y, ROLE_TAB_W, ROLE_H], Align::Center, FONT, 8, TEXT)),
        i if i >= IDX_ROLE_CAP && i < IDX_ROLE_KEY => {
            hidden(rect(name, [ROLE_CAP_X[i - IDX_ROLE_CAP], TAB_Y, ROLE_CAP, ROLE_CAP], RULE, false))
        }
        i if i >= IDX_ROLE_KEY && i < IDX_HEAD_SQ => hidden(text(name, [ROLE_CAP_X[i - IDX_ROLE_KEY], TAB_Y, ROLE_CAP, ROLE_CAP], Align::Center, FONT_SMALL, 2, TEXT)),
        i if i >= IDX_HEAD_SQ && i < IDX_BADGE => {
            hidden(rect(name, [HEAD_MARK_X, row_y(0), HEAD_SQ, HEAD_SQ], KIND_SEQUENCE, false))
        }
        i if i >= IDX_BADGE && i < IDX_BADGE_NUM => {
            hidden(rect(name, [BADGE_X, row_y(1), BADGE_W, BADGE_W], ACCENT, false))
        }
        i if i >= IDX_BADGE_NUM && i < IDX_TAB_BOX => hidden(text(name, [BADGE_X, row_y(1), BADGE_W + 8.0, BADGE_W + 6.0], Align::Center, FONT_PILL, 2, TEXT)),
        i if i >= IDX_TAB_BOX && i < MENU_COUNT => hidden(rect(name, [0.0, POP_TAB_Y, 2.0, 2.0], RULE, false)),
        i if i >= IDX_TILE_BADGE && i < IDX_FLOPPY => hidden(text(name, [0.0, 0.0, 52.0, MARK_H], Align::Center, FONT_SMALL, 4, TEXT)),
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
                let (x, w) = row_label_box(false);
                text(name, [x, row_y(row), w, ROW_H], Align::Left, FONT, 80, TEXT)
            } else {
                let (x, w) = row_value_box(false);
                text(name, [x, row_y(row), w, ROW_H], Align::Right, FONT, 72, TEXT)
            }
        }
    }
}

/// Every pane of the menu, in tree (draw) order: back to front.
#[cfg(test)]
pub fn specs() -> Vec<Spec> {
    (0..COUNT).map(spec).collect()
}

#[cfg(test)]
mod role_tests {
    lib_unit_tests! {
    use super::*;

    fn role_caps_reuse_the_two_cap_panes() {
        assert_eq!(role_caps(RoleKeys::None), [None, None]);
        assert_eq!(role_caps(RoleKeys::LR), [Some((&[76][..], ROLE_CAP)), Some((&[82][..], ROLE_CAP))]);
        assert_eq!(role_caps(RoleKeys::ZR), [None, Some((&[90, 82][..], ROLE_CAP_ZR))]);
        for keys in [RoleKeys::None, RoleKeys::LR, RoleKeys::ZR] {
            for (i, cap) in role_caps(keys).iter().enumerate() {
                let Kind::Text { chars, .. } = spec(IDX_ROLE_KEY + i).kind else { panic!("key pane is text") };
                assert_eq!(chars, 2);
                if let Some((text, _)) = cap {
                    assert!(text.len() <= usize::from(chars));
                }
            }
        }
        let (center, width) = (ROLE_CAP_X[1], ROLE_CAP_ZR);
        assert!(center - width / 2.0 >= ROLE_X + ROLE_TAB_W + 4.0);
        assert!(center + width / 2.0 <= PANEL_W / 2.0);
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[("role_caps_reuse_the_two_cap_panes", role_caps_reuse_the_two_cap_panes as fn())]);
    }
    }
}
