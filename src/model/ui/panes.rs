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

pub const BG: Name = Name::new("tr_bg");
pub const TITLE: Name = Name::new("tr_title");
pub const TAB_HI: Name = Name::new("tr_tab_hi");
pub const CRUMB: Name = Name::new("tr_crumb");
pub const GROUP_HI: Name = Name::new("tr_grp_hi");
pub const DIVIDER: Name = Name::new("tr_div");
pub const ROW_HI: Name = Name::new("tr_row_hi");
pub const SEP: Name = Name::new("tr_sep");
pub const HELP: [Name; 2] = [Name::new("tr_help_0"), Name::new("tr_help_1")];
pub const HINT: Name = Name::new("tr_hint");

pub const fn tab(i: usize) -> Name {
    Name::indexed("tr_tab_", i)
}
pub const fn group(i: usize) -> Name {
    Name::indexed("tr_grp_", i)
}
pub const fn row_label(i: usize) -> Name {
    Name::indexed("tr_rl_", i)
}
pub const fn row_value(i: usize) -> Name {
    Name::indexed("tr_rv_", i)
}

// ---- colours (RGBA) ------------------------------------------------------------------------------------

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

const fn text(name: Name, x: f32, y: f32, w: f32, h: f32, align: Align, font: (f32, f32), chars: u16, color: [u8; 4]) -> Spec {
    Spec { name, kind: Kind::Text { align, font, chars, color }, x, y, w, h, visible: true }
}

/// Every pane of the menu, in tree (draw) order: back to front.
pub fn specs() -> Vec<Spec> {
    let mut all = Vec::new();
    all.push(rect(BG, 0.0, 0.0, PANEL_W, PANEL_H, PANEL, true));
    all.push(rect(TAB_HI, -840.0 + 100.0, TAB_Y, 200.0, TAB_H, ACCENT, true));
    for i in 0..TABS {
        all.push(text(tab(i), -840.0 + TAB_BOX_W / 2.0, TAB_Y, TAB_BOX_W, TAB_H, Align::Left, FONT, 40, TEXT_DIM));
    }
    all.push(text(TITLE, 600.0, TAB_Y, 560.0, TAB_H, Align::Right, FONT_SMALL, 40, TEXT_DIM));
    all.push(rect(SEP, 0.0, TAB_Y - 52.0, 1760.0, 3.0, RULE, true));
    all.push(text(CRUMB, 0.0, CRUMB_Y, 1760.0, 56.0, Align::Left, FONT, 120, TEXT));
    all.push(rect(DIVIDER, DIVIDER_X, 20.0, 3.0, 650.0, RULE, true));
    all.push(rect(GROUP_HI, GROUP_X, group_y(0), GROUP_W, GROUP_ROW_H, HIGHLIGHT, true));
    for i in 0..GROUPS {
        all.push(text(group(i), GROUP_X, group_y(i), GROUP_W, GROUP_ROW_H, Align::Left, FONT, 48, TEXT_DIM));
    }
    all.push(rect(ROW_HI, (ROW_LEFT + ROW_RIGHT) / 2.0, row_y(0), ROW_RIGHT - ROW_LEFT + 30.0, ROW_H, HIGHLIGHT, true));
    for i in 0..ROWS {
        all.push(text(row_label(i), ROW_LEFT + ROW_LABEL_W / 2.0, row_y(i), ROW_LABEL_W, ROW_H, Align::Left, FONT, 80, TEXT));
        all.push(text(row_value(i), ROW_RIGHT - ROW_VALUE_W / 2.0, row_y(i), ROW_VALUE_W, ROW_H, Align::Right, FONT, 72, TEXT));
    }
    all.push(rect(Name::new("tr_sep_help"), 0.0, HELP_Y[0] + 48.0, 1760.0, 3.0, RULE, true));
    for (i, name) in HELP.iter().enumerate() {
        all.push(text(*name, HELP_X, HELP_Y[i], HELP_W, 48.0, Align::Left, FONT_SMALL, 160, TEXT));
    }
    all.push(text(HINT, HELP_X, HINT_Y, HELP_W, 44.0, Align::Left, FONT_SMALL, 160, TEXT_DIM));
    all
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
        assert_ne!(ROOT, BG);
    }

    #[test]
    fn indexed_names_have_two_digits() {
        assert_eq!(tab(3).as_str(), "tr_tab_03");
        assert_eq!(row_value(11).as_str(), "tr_rv_11");
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