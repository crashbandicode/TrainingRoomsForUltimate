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

    const fn from_raw(bytes: [u8; 24], len: u8) -> Name {
        Name { bytes, len }
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
pub const IDX_TILE_HI: usize = IDX_CHIP_BADGE + CHIPS;
pub const IDX_TILE: usize = IDX_TILE_HI + 1;
pub const IDX_TILE_LABEL: usize = IDX_TILE + TILES;
pub const ITEM_ICONS: usize = 38;
pub const IDX_ITEM_ICON: usize = IDX_TILE_LABEL + TILES;
/// How many panes the menu itself owns and the runtime resolves at registration; the panes after these (the
/// notification panel and the input log) belong to the input package, which finds them by name when it draws.
pub const MENU_COUNT: usize = IDX_NOTICE;
/// The notification panel (SPEC-input-05): its own container under `RootPane`, a backing rectangle, a header and a
/// text box. Written by the notices code of the input package.
pub const IDX_NOTICE: usize = IDX_ITEM_ICON + ITEM_ICONS;
pub const IDX_NOTICE_BASE: usize = IDX_NOTICE + 1;
pub const IDX_NOTICE_HEADER: usize = IDX_NOTICE + 2;
pub const IDX_NOTICE_TEXT: usize = IDX_NOTICE + 3;
pub const IDX_NOTICE_LONG: usize = IDX_NOTICE + 4;
/// Full startup chord text plus its terminator; checked against notify::BODY_MAX.
pub const NOTICE_LONG_UNITS: u16 = 321;
/// The input log (SPEC-input-02, lead ruling 2026-11-02: text slots, no new textures): a column container, 15 row
/// containers, and in each row 5 slots. A slot is a text box (a button glyph of the game's font, coloured) and two
/// rectangles (the shaft and the head of the plugin's own arrow for a stick direction), plus the held-frames text
/// (4 units) and the status text (48 units) of the row. Names are unique across the layout:
/// `TrModInputLog` / `TrModInputLog<rr>` / `ILG_<rr>_<s>` / `ILA_<rr>_<s>` / `ILH_<rr>_<s>` / `ILF_<rr>` / `ILS_<rr>`.
pub const LOG_ROWS: usize = 15;
pub const LOG_SLOTS: usize = 5;
/// A slot's x in its row, and the head of its arrow: its distance from the shaft's centre along the arrow.
pub const LOG_SLOT_X0: f32 = -200.0;
pub const LOG_SLOT_PITCH: f32 = 54.0;
pub const LOG_HEAD_OFFSET: f32 = 16.0;
pub const IDX_LOG: usize = IDX_NOTICE_LONG + 1;
pub const IDX_LOG_ROW: usize = IDX_LOG + 1;
pub const IDX_LOG_GLYPH: usize = IDX_LOG_ROW + LOG_ROWS;
pub const IDX_LOG_SHAFT: usize = IDX_LOG_GLYPH + LOG_ROWS * LOG_SLOTS;
pub const IDX_LOG_HEAD: usize = IDX_LOG_SHAFT + LOG_ROWS * LOG_SLOTS;
pub const IDX_LOG_FRAME: usize = IDX_LOG_HEAD + LOG_ROWS * LOG_SLOTS;
pub const IDX_LOG_STATUS: usize = IDX_LOG_FRAME + LOG_ROWS;
/// The roster grid's per-role marks, Player block then CPU block (`Frame::tile_badges` etc.): the fork's corner
/// badges (`roster_p_NN` / `roster_c_NN`), All-stripes and the Solo Popo "S" markers. Appended after the input
/// log, so no existing index moves.
pub const IDX_TILE_BADGE: usize = IDX_LOG_STATUS + LOG_ROWS;
pub const IDX_TILE_STRIPE: usize = IDX_TILE_BADGE + 2 * TILES;
pub const IDX_TILE_SOLO: usize = IDX_TILE_STRIPE + 2 * TILES;
/// How many panes the plugin adds to the layout.
pub const COUNT: usize = IDX_TILE_SOLO + 2;

pub const fn log_slot(row: usize, slot: usize) -> usize {
    row * LOG_SLOTS + slot
}

/// A name with two index numbers: `prefix` + row + `_` + slot.
const fn name2(prefix: &str, row: usize, slot: usize) -> Name {
    let a = Name::indexed(prefix, row);
    let mut bytes = [0u8; 24];
    let len = a.len as usize;
    let mut i = 0;
    while i < len {
        bytes[i] = a.bytes[i];
        i += 1;
    }
    bytes[i] = b'_';
    bytes[i + 1] = b'0' + slot as u8;
    Name::from_raw(bytes, (len + 2) as u8)
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
        IDX_MENU => ROOT,
        IDX_NOTICE => Name::new("TrModDisp0"),
        IDX_NOTICE_BASE => Name::new("TrModDisp0Base"),
        IDX_NOTICE_HEADER => Name::new("TrModDisp0Header"),
        IDX_NOTICE_TEXT => Name::new("TrModDisp0Txt"),
        IDX_NOTICE_LONG => Name::new("TrModDisp0Long"),
        IDX_LOG => Name::new("TrModInputLog"),
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
        i if i >= IDX_LOG_ROW && i < IDX_LOG_ROW + LOG_ROWS => Name::indexed("TrModInputLog", i - IDX_LOG_ROW),
        i if i >= IDX_LOG_GLYPH && i < IDX_LOG_GLYPH + LOG_ROWS * LOG_SLOTS => {
            let n = i - IDX_LOG_GLYPH;
            name2("ILG_", n / LOG_SLOTS, n % LOG_SLOTS)
        }
        i if i >= IDX_LOG_SHAFT && i < IDX_LOG_SHAFT + LOG_ROWS * LOG_SLOTS => {
            let n = i - IDX_LOG_SHAFT;
            name2("ILA_", n / LOG_SLOTS, n % LOG_SLOTS)
        }
        i if i >= IDX_LOG_HEAD && i < IDX_LOG_HEAD + LOG_ROWS * LOG_SLOTS => {
            let n = i - IDX_LOG_HEAD;
            name2("ILH_", n / LOG_SLOTS, n % LOG_SLOTS)
        }
        i if i >= IDX_LOG_FRAME && i < IDX_LOG_FRAME + LOG_ROWS => Name::indexed("ILF_", i - IDX_LOG_FRAME),
        i if i >= IDX_LOG_STATUS && i < IDX_LOG_STATUS + LOG_ROWS => Name::indexed("ILS_", i - IDX_LOG_STATUS),
        i if i >= IDX_CHIP && i < IDX_CHIP + CHIPS => Name::indexed("tr_ch_", i - IDX_CHIP),
        i if i >= IDX_CHIP_NEEDLE && i < IDX_CHIP_NEEDLE + CHIPS => Name::indexed("tr_cn_", i - IDX_CHIP_NEEDLE),
        i if i >= IDX_CHIP_LABEL && i < IDX_CHIP_LABEL + CHIPS => Name::indexed("tr_cl_", i - IDX_CHIP_LABEL),
        i if i >= IDX_CHIP_BADGE && i < IDX_CHIP_BADGE + CHIPS => Name::indexed("tr_cb_", i - IDX_CHIP_BADGE),
        i if i >= IDX_TILE && i < IDX_TILE + TILES => Name::indexed("tr_ti_", i - IDX_TILE),
        i if i >= IDX_TILE_LABEL && i < IDX_TILE_LABEL + TILES => Name::indexed("tr_tl_", i - IDX_TILE_LABEL),
        i if i >= IDX_TILE_BADGE && i < COUNT => name2("tr_rm", (i - IDX_TILE_BADGE) / TILES, i % TILES),
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
/// Notices keep their specified black/custom body colors on an opaque light panel.
pub const NOTICE_BACKGROUND: [u8; 4] = [255, 255, 255, 255];
pub const NOTICE_HEADER: [u8; 4] = [38, 44, 56, 255];

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

/// Pane `index`'s kind and place.
pub const fn spec(index: usize) -> Spec {
    let name = name_of(index);
    match index {
        i if i >= IDX_ITEM_ICON && i < IDX_ITEM_ICON + ITEM_ICONS => rect(name, [0.0, 0.0, 64.0, 64.0], [255; 4], false),
        IDX_MENU => Spec { name, parent: ROOT_PANE, kind: Kind::Null, x: 0.0, y: 0.0, w: 1920.0, h: 1080.0, visible: false },
        IDX_NOTICE => Spec { name, parent: ROOT_PANE, kind: Kind::Null, x: 0.0, y: 0.0, w: 1920.0, h: 1080.0, visible: false },
        IDX_NOTICE_BASE => under(rect(name, [0.0, 405.0, 1000.0, 150.0], NOTICE_BACKGROUND, true), name_of(IDX_NOTICE)),
        IDX_NOTICE_HEADER => under(text(name, [0.0, 445.0, 960.0, 50.0], Align::Center, FONT_SMALL, 24, NOTICE_HEADER), name_of(IDX_NOTICE)),
        IDX_NOTICE_TEXT => under(text(name, [0.0, 390.0, 960.0, 70.0], Align::Center, FONT, 48, TEXT), name_of(IDX_NOTICE)),
        IDX_NOTICE_LONG => under(text(name, [0.0, 325.0, 960.0, 180.0], Align::Center, (14.0, 24.0),
            NOTICE_LONG_UNITS, TEXT), name_of(IDX_NOTICE)),
        IDX_BG => rect(name, [0.0, 0.0, PANEL_W, PANEL_H], PANEL, true),
        IDX_LOG => Spec { name, parent: ROOT_PANE, kind: Kind::Null, x: -700.0, y: 380.0, w: 520.0, h: 800.0, visible: false },
        i if i >= IDX_LOG_ROW && i < IDX_LOG_ROW + LOG_ROWS => {
            Spec { name, parent: name_of(IDX_LOG), kind: Kind::Null, x: 0.0, y: -52.5 * (i - IDX_LOG_ROW) as f32, w: 520.0, h: 50.0, visible: true }
        }
        i if i >= IDX_LOG_GLYPH && i < IDX_LOG_GLYPH + LOG_ROWS * LOG_SLOTS => {
            let n = i - IDX_LOG_GLYPH;
            under(Spec { visible: false, ..text(name, [LOG_SLOT_X0 + LOG_SLOT_PITCH * (n % LOG_SLOTS) as f32, 0.0, 50.0, 48.0], Align::Center, FONT, 4, TEXT) }, name_of(IDX_LOG_ROW + n / LOG_SLOTS))
        }
        i if i >= IDX_LOG_SHAFT && i < IDX_LOG_SHAFT + LOG_ROWS * LOG_SLOTS => {
            let n = i - IDX_LOG_SHAFT;
            under(rect(name, [LOG_SLOT_X0 + LOG_SLOT_PITCH * (n % LOG_SLOTS) as f32, 0.0, 8.0, 36.0], TEXT, false), name_of(IDX_LOG_ROW + n / LOG_SLOTS))
        }
        i if i >= IDX_LOG_HEAD && i < IDX_LOG_HEAD + LOG_ROWS * LOG_SLOTS => {
            let n = i - IDX_LOG_HEAD;
            under(rect(name, [LOG_SLOT_X0 + LOG_SLOT_PITCH * (n % LOG_SLOTS) as f32, LOG_HEAD_OFFSET, 18.0, 18.0], TEXT, false), name_of(IDX_LOG_ROW + n / LOG_SLOTS))
        }
        i if i >= IDX_LOG_FRAME && i < IDX_LOG_FRAME + LOG_ROWS => {
            under(text(name, [130.0, 0.0, 70.0, 48.0], Align::Right, FONT_SMALL, 4, TEXT), name_of(IDX_LOG_ROW + (i - IDX_LOG_FRAME)))
        }
        i if i >= IDX_LOG_STATUS && i < IDX_LOG_STATUS + LOG_ROWS => {
            under(text(name, [330.0, 0.0, 200.0, 48.0], Align::Left, FONT_SMALL, 48, TEXT_DIM), name_of(IDX_LOG_ROW + (i - IDX_LOG_STATUS)))
        }
        IDX_CHIP_HI => rect(name, [0.0, 0.0, CHIP_SIZE + 16.0, CHIP_SIZE + 16.0], ACCENT, false),
        IDX_TILE_HI => rect(name, [0.0, 0.0, 100.0, 60.0], ACCENT, false),
        i if i >= IDX_CHIP && i < IDX_CHIP + CHIPS => rect(name, [0.0, 0.0, CHIP_SIZE, CHIP_SIZE], HIGHLIGHT, false),
        i if i >= IDX_CHIP_NEEDLE && i < IDX_CHIP_NEEDLE + CHIPS => rect(name, [0.0, 0.0, 10.0, 84.0], TEXT, false),
        i if i >= IDX_CHIP_LABEL && i < IDX_CHIP_LABEL + CHIPS => Spec { visible: false, ..text(name, [0.0, 0.0, CHIP_SIZE, 44.0], Align::Center, FONT_SMALL, 16, TEXT) },
        i if i >= IDX_CHIP_BADGE && i < IDX_CHIP_BADGE + CHIPS => Spec { visible: false, ..text(name, [0.0, 0.0, 60.0, 44.0], Align::Center, FONT, 4, TEXT) },
        i if i >= IDX_TILE && i < IDX_TILE + TILES => rect(name, [0.0, 0.0, 100.0, 60.0], HIGHLIGHT, false),
        i if i >= IDX_TILE_LABEL && i < IDX_TILE_LABEL + TILES => Spec { visible: false, ..text(name, [0.0, 0.0, 100.0, 40.0], Align::Center, FONT_SMALL, 28, TEXT) },
        i if i >= IDX_TILE_STRIPE && i < IDX_TILE_SOLO => Spec { visible: false, ..rect(name, [0.0, 0.0, 4.0, 86.0], TEXT, false) },
        i if i >= IDX_TILE_BADGE => Spec { visible: false, ..text(name, [0.0, 0.0, 52.0, 28.0], Align::Center, FONT_SMALL, 4, TEXT) },
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
            for b in &all[i + 1..] {
                assert_ne!(a.name, b.name, "{:?} is listed twice", a.name);
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
