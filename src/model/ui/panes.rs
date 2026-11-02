//! The pane contract of the plugin's own menu layout (SPEC-core-03 §4.2, SPEC-core-07 §6 "keep one table in
//! the model crate"): every pane the layout archive carries for the menu, by name, with its place and size.
//! The host builder (`tools/layout-builder`) builds the BFLYT from this table; the runtime finds its panes by
//! the same names and writes only text, colours, visibility and positions into them. Nothing here is
//! upstream's: the layout is flat (one container, no parts) and draws only text boxes and flat rectangles,
//! so the archive needs no new texture.
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
    /// A flat rectangle: the stock solid texture tinted by its vertex colours.
    Rect { color: [u8; 4] },
    /// A text box in the game's `nintendo64` font; `chars` is the buffer's capacity in UTF-16 units.
    Text { align: Align, font: (f32, f32), chars: u16, color: [u8; 4] },
}

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub name: Name,
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

pub const FONT: (f32, f32) = (22.0, 34.0);
pub const FONT_SMALL: (f32, f32) = (18.0, 28.0);

pub const fn row_y(row: usize) -> f32 {
    ROW_FIRST_Y - ROW_PITCH * row as f32
}

pub const fn group_y(group: usize) -> f32 {
    ROW_FIRST_Y - GROUP_ROW_H * group as f32
}

// ---- names ---------------------------------------------------------------------------------------------
//
// The panes in tree (draw) order, back to front. An index is stable: the builder adds the panes in this
// order and the runtime keeps its resolved pointers in an array of the same order.

pub const IDX_BG: usize = 0;
pub const IDX_TAB_HI: usize = 1;
pub const IDX_TAB: usize = 2;
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
/// How many panes the menu adds under the container.
pub const COUNT: usize = IDX_HINT + 1;

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
        i if i >= IDX_TAB && i < IDX_TAB + TABS => Name::indexed("tr_tab_", i - IDX_TAB),
        i if i >= IDX_GROUP && i < IDX_GROUP + GROUPS => Name::indexed("tr_grp_", i - IDX_GROUP),
        i if i >= IDX_ROW && i < IDX_ROW + 2 * ROWS => {
            if (i - IDX_ROW) % 2 == 0 {
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

pub const PANEL: [u8; 4] = [8, 10, 14, 236];
pub const ACCENT: [u8; 4] = [176, 48, 40, 255];
pub const HIGHLIGHT: [u8; 4] = [38, 44, 56, 255];
pub const RULE: [u8; 4] = [60, 66, 78, 255];
pub const TEXT: [u8; 4] = [236, 238, 242, 255];
pub const TEXT_DIM: [u8; 4] = [150, 156, 168, 255];
pub const TEXT_ON: [u8; 4] = [110, 210, 160, 255];
pub const TEXT_OFF: [u8; 4] = [110, 116, 128, 255];

const fn rect(name: Name, x: f32, y: f32, w: f32, h: f32, color: [u8; 4], visible: bool) -> Spec {
    Spec { name, kind: Kind::Rect { color }, x, y, w, h, visible }
}

#[allow(clippy::too_many_arguments)]
const fn text(name: Name, x: f32, y: f32, w: f32, h: f32, align: Align, font: (f32, f32), chars: u16, color: [u8; 4]) -> Spec {
    Spec { name, kind: Kind::Text { align, font, chars, color }, x, y, w, h, visible: true }
}

/// Pane `index`'s kind and place.
pub const fn spec(index: usize) -> Spec {
    let name = name_of(index);
    match index {
        IDX_BG => rect(name, 0.0, 0.0, PANEL_W, PANEL_H, PANEL, true),
        IDX_TAB_HI => rect(name, -740.0, TAB_Y, 200.0, TAB_H, ACCENT, true),
        IDX_TITLE => text(name, 600.0, TAB_Y, 560.0, TAB_H, Align::Right, FONT_SMALL, 40, TEXT_DIM),
        IDX_SEP => rect(name, 0.0, TAB_Y - 52.0, 1760.0, 3.0, RULE, true),
        IDX_CRUMB => text(name, 0.0, CRUMB_Y, 1760.0, 56.0, Align::Left, FONT, 120, TEXT),
        IDX_DIVIDER => rect(name, DIVIDER_X, 20.0, 3.0, 650.0, RULE, true),
        IDX_GROUP_HI => rect(name, GROUP_X, group_y(0), GROUP_W, GROUP_ROW_H, HIGHLIGHT, true),
        IDX_ROW_HI => rect(name, (ROW_LEFT + ROW_RIGHT) / 2.0, row_y(0), ROW_RIGHT - ROW_LEFT + 30.0, ROW_H, HIGHLIGHT, true),
        IDX_SEP_HELP => rect(name, 0.0, HELP_Y[0] + 48.0, 1760.0, 3.0, RULE, true),
        IDX_HINT => text(name, HELP_X, HINT_Y, HELP_W, 44.0, Align::Left, FONT_SMALL, 160, TEXT_DIM),
        i if i == IDX_HELP => text(name, HELP_X, HELP_Y[0], HELP_W, 48.0, Align::Left, FONT_SMALL, 160, TEXT),
        i if i == IDX_HELP + 1 => text(name, HELP_X, HELP_Y[1], HELP_W, 48.0, Align::Left, FONT_SMALL, 160, TEXT),
        i if i >= IDX_TAB && i < IDX_TAB + TABS => {
            text(name, -840.0 + TAB_BOX_W / 2.0, TAB_Y, TAB_BOX_W, TAB_H, Align::Left, FONT, 40, TEXT_DIM)
        }
        i if i >= IDX_GROUP && i < IDX_GROUP + GROUPS => {
            text(name, GROUP_X, group_y(i - IDX_GROUP), GROUP_W, GROUP_ROW_H, Align::Left, FONT, 48, TEXT_DIM)
        }
        i => {
            let row = (i - IDX_ROW) / 2;
            if (i - IDX_ROW) % 2 == 0 {
                text(name, ROW_LEFT + ROW_LABEL_W / 2.0, row_y(row), ROW_LABEL_W, ROW_H, Align::Left, FONT, 80, TEXT)
            } else {
                text(name, ROW_RIGHT - ROW_VALUE_W / 2.0, row_y(row), ROW_VALUE_W, ROW_H, Align::Right, FONT, 72, TEXT)
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
        for s in specs() {
            assert!(s.x - s.w / 2.0 >= -PANEL_W / 2.0 - 1.0 && s.x + s.w / 2.0 <= PANEL_W / 2.0 + 1.0, "{:?} x", s.name);
            assert!(s.y - s.h / 2.0 >= -PANEL_H / 2.0 - 1.0 && s.y + s.h / 2.0 <= PANEL_H / 2.0 + 1.0, "{:?} y", s.name);
        }
    }

    #[test]
    fn the_rows_do_not_run_into_the_help_panel() {
        assert!(row_y(ROWS - 1) - ROW_H / 2.0 > HELP_Y[0] + 48.0 + 4.0);
    }
}