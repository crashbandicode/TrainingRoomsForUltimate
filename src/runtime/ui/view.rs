//! Draws one menu frame into the layout's panes (SPEC-core-03 §4.2): text, colours, positions of the
//! highlight bars. Runs inside the layout tick, only when the worker published a new frame.

use skyline::nn::ui2d::Pane;

use crate::model::frame::{Frame, Tone, View};
use crate::model::menu::options::DIRECTIONS;
use crate::model::menu::CHIP_POS;
use crate::model::panes as p;
use crate::runtime::ui::pane;

/// The resolved panes of the menu in the layout now showing.
pub struct Panes {
    pub container: *mut Pane,
    pub all: [*mut Pane; p::MENU_COUNT],
}

// SAFETY: the pointers are only dereferenced on the UI job thread, inside the tick, while the layout lives.
unsafe impl Send for Panes {}

fn tone_color(tone: Tone) -> [u8; 4] {
    match tone {
        Tone::Normal => p::TEXT,
        Tone::Dim => p::TEXT_DIM,
        Tone::On => p::TEXT_ON,
        Tone::Off => p::TEXT_OFF,
    }
}

/// An estimate of a text's width in layout units (the game's font is proportional; a pane is only
/// positioned from it, never clipped by it).
fn text_width(units: usize) -> f32 {
    units as f32 * p::FONT.0 * 0.56
}

impl Panes {
    unsafe fn text(&self, index: usize, units: &[u16], color: [u8; 4]) {
        let pane = self.all[index];
        if pane.is_null() {
            return;
        }
        pane::set_text(pane, units);
        pane::set_text_color(pane, p::linear(color));
    }

    /// Position and width of a text pane (the row panes serve the list and the direction grid's panel).
    unsafe fn place(&self, index: usize, x: f32, y: f32, w: f32) {
        let pane = self.all[index];
        if !pane.is_null() {
            pane::set_pos(pane, x, y);
            pane::set_width(pane, w);
        }
    }

    unsafe fn at(&self, index: usize, x: f32, y: f32) {
        let pane = self.all[index];
        if !pane.is_null() {
            pane::set_pos(pane, x, y);
        }
    }

    unsafe fn show(&self, index: usize, shown: bool) {
        let pane = self.all[index];
        if !pane.is_null() {
            pane::set_visible(pane, shown);
        }
    }

    /// Write a frame.
    pub unsafe fn draw(&self, frame: &Frame) {
        // Tabs: the active one on the accent bar.
        let mut left = -840.0;
        for i in 0..p::TABS {
            let index = p::tab_index(i);
            if i >= usize::from(frame.tab_count) {
                self.text(index, &[], p::TEXT_DIM);
                continue;
            }
            let units = frame.tabs[i].units();
            let width = text_width(units.len());
            let active = i == usize::from(frame.tab_active);
            self.text(index, units, if active { p::TEXT } else { p::TEXT_DIM });
            self.at(index, left + p::TAB_BOX_W / 2.0, p::TAB_Y);
            if active {
                self.at(p::IDX_TAB_HI, left + width / 2.0, p::TAB_Y);
                let hi = self.all[p::IDX_TAB_HI];
                if !hi.is_null() {
                    pane::set_width(hi, width + 36.0);
                }
            }
            left += width + 64.0;
        }
        self.text(p::IDX_TITLE, frame.title.units(), p::TEXT_DIM);
        self.text(p::IDX_CRUMB, frame.crumb.units(), p::TEXT);

        // Groups.
        for i in 0..p::GROUPS {
            let active = i == usize::from(frame.group_active);
            if i < usize::from(frame.group_count) {
                self.text(p::group_index(i), frame.groups[i].units(), if active { p::TEXT } else { p::TEXT_DIM });
            } else {
                self.text(p::group_index(i), &[], p::TEXT_DIM);
            }
        }
        self.show(p::IDX_GROUP_HI, frame.group_count > 0);
        self.at(p::IDX_GROUP_HI, p::GROUP_X, p::group_y(usize::from(frame.group_active)));

        let list = frame.view == View::List;
        self.draw_chips(frame);
        self.draw_tiles(frame);
        // Rows; on the direction grid the same panes carry the "Selected (weight)" panel at the right of the chips.
        let panel = if frame.view == View::Chips { selected_panel(frame) } else { [None; p::ROWS] };
        for i in 0..p::ROWS {
            if list && i < usize::from(frame.row_count) {
                let row = &frame.rows[i];
                self.place(p::row_label_index(i), p::ROW_LEFT + p::ROW_LABEL_W / 2.0, p::row_y(i), p::ROW_LABEL_W);
                self.place(p::row_value_index(i), p::ROW_RIGHT - p::ROW_VALUE_W / 2.0, p::row_y(i) - p::VALUE_DROP, p::ROW_VALUE_W);
                self.text(p::row_label_index(i), row.label.units(), tone_color(row.label_tone));
                self.text(p::row_value_index(i), row.value.units(), tone_color(row.value_tone));
            } else if let Some((name, weight, tone)) = panel[i] {
                self.place(p::row_label_index(i), PANEL_X + PANEL_NAME_W / 2.0, p::row_y(i), PANEL_NAME_W);
                self.place(p::row_value_index(i), PANEL_X + PANEL_NAME_W + PANEL_WEIGHT_W / 2.0, p::row_y(i) - p::VALUE_DROP, PANEL_WEIGHT_W);
                self.text(p::row_label_index(i), name.units(), tone_color(tone));
                self.text(p::row_value_index(i), weight.units(), tone_color(tone));
            } else {
                self.text(p::row_label_index(i), &[], p::TEXT);
                self.text(p::row_value_index(i), &[], p::TEXT);
            }
        }
        match frame.focus.filter(|_| list) {
            Some(row) => {
                self.show(p::IDX_ROW_HI, true);
                self.at(p::IDX_ROW_HI, (p::ROW_LEFT + p::ROW_RIGHT) / 2.0, p::row_y(usize::from(row)));
            }
            None => self.show(p::IDX_ROW_HI, false),
        }

        self.text(p::IDX_HELP, frame.help[0].units(), p::TEXT);
        self.text(p::IDX_HELP + 1, frame.help[1].units(), p::TEXT);
        self.text(p::IDX_HINT, frame.hint.units(), p::TEXT_DIM);
    }
}

/// Where the "Selected (weight)" panel of the direction grids sits: right of the chips (they end at x = 480).
const PANEL_X: f32 = 520.0;
const PANEL_NAME_W: f32 = 240.0;
const PANEL_WEIGHT_W: f32 = 110.0;

type PanelLine = Option<(crate::model::frame::Text<24>, crate::model::frame::Text<8>, Tone)>;

/// The panel's lines (mockup 09): a header, then each chip with a weight in chip order, `xN` at the right;
/// "none selected" while the grid is empty.
fn selected_panel(frame: &Frame) -> [PanelLine; p::ROWS] {
    use core::fmt::Write;
    let mut lines: [PanelLine; p::ROWS] = [None; p::ROWS];
    let mut put = |at: usize, name: &str, weight: Option<u8>, tone: Tone| {
        if let Some(slot) = lines.get_mut(at) {
            let mut n = crate::model::frame::Text::<24>::new();
            let mut w = crate::model::frame::Text::<8>::new();
            let _ = n.write_str(name);
            if let Some(weight) = weight {
                let _ = write!(w, "x{weight}");
            }
            *slot = Some((n, w, tone));
        }
    };
    put(0, "Selected (weight)", None, Tone::Normal);
    let mut next = 1;
    for (i, weight) in frame.chip_weights.iter().enumerate().take(p::CHIPS) {
        if *weight != 0 {
            put(next, DIRECTIONS[i], Some(*weight), Tone::On);
            next += 1;
        }
    }
    if next == 1 {
        put(1, "none selected", None, Tone::Dim);
    }
    lines
}
/// Where a chip's pointer stub sits and how far it is turned: the direction each chip stands for, in degrees
/// (counter-clockwise from the right; In / Out are the screen's left / right here, the menu mirrors them
/// with the opponent), `None` for Neutral, which has no pointer.
const CHIP_ANGLE: [Option<f32>; 11] = [
    Some(0.0),
    Some(45.0),
    Some(90.0),
    Some(135.0),
    Some(180.0),
    Some(225.0),
    Some(270.0),
    Some(315.0),
    None,
    Some(180.0),
    Some(0.0),
];

impl Panes {
    unsafe fn rect(&self, index: usize, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
        let pane = self.all[index];
        if pane.is_null() {
            return;
        }
        pane::set_visible(pane, true);
        pane::set_pos(pane, x, y);
        pane::set_width(pane, w);
        pane::set_height(pane, h);
        pane::set_rect_color(pane, p::linear(color));
    }

    unsafe fn hide(&self, index: usize) {
        let pane = self.all[index];
        if !pane.is_null() {
            pane::set_visible(pane, false);
        }
    }

    unsafe fn label(&self, index: usize, units: &[u16], x: f32, y: f32, w: f32, color: [u8; 4]) {
        let pane = self.all[index];
        if pane.is_null() {
            return;
        }
        pane::set_visible(pane, true);
        pane::set_pos(pane, x, y);
        pane::set_width(pane, w);
        pane::set_text(pane, units);
        pane::set_text_color(pane, p::linear(color));
    }

    unsafe fn draw_chips(&self, frame: &Frame) {
        let shown = frame.view == View::Chips;
        for i in 0..p::CHIPS {
            if !shown {
                for base in [p::IDX_CHIP, p::IDX_CHIP_NEEDLE, p::IDX_CHIP_LABEL, p::IDX_CHIP_BADGE] {
                    self.hide(base + i);
                }
                continue;
            }
            let (col, row) = CHIP_POS[i];
            let (x, y) = p::chip_xy(col, row);
            let weight = frame.chip_weights[i];
            let tile = if weight != 0 { [58, 66, 84, 255] } else { p::HIGHLIGHT };
            self.rect(p::IDX_CHIP + i, x, y, p::CHIP_SIZE, p::CHIP_SIZE, tile);
            match CHIP_ANGLE[i] {
                Some(angle) => {
                    let radians = angle.to_radians();
                    let (dx, dy) = (radians.cos(), radians.sin());
                    self.rect(p::IDX_CHIP_NEEDLE + i, x + dx * 34.0, y + 12.0 + dy * 34.0, 12.0, 52.0, if weight != 0 { p::TEXT } else { p::TEXT_DIM });
                    pane::set_rot_z(self.all[p::IDX_CHIP_NEEDLE + i], angle - 90.0);
                }
                None => self.rect(p::IDX_CHIP_NEEDLE + i, x, y + 12.0, 30.0, 30.0, if weight != 0 { p::TEXT } else { p::TEXT_DIM }),
            }
            let mut name = crate::model::frame::Text::<16>::new();
            let _ = core::fmt::Write::write_str(&mut name, DIRECTIONS[i]);
            self.label(p::IDX_CHIP_LABEL + i, name.units(), x, y - 70.0, p::CHIP_SIZE, if weight != 0 { p::TEXT } else { p::TEXT_DIM });
            if weight != 0 {
                let mut badge = crate::model::frame::Text::<4>::new();
                let _ = core::fmt::Write::write_fmt(&mut badge, format_args!("x{weight}"));
                self.label(p::IDX_CHIP_BADGE + i, badge.units(), x + 60.0, y + 66.0, 60.0, p::TEXT_ON);
            } else {
                self.hide(p::IDX_CHIP_BADGE + i);
            }
        }
        if shown {
            let (col, row) = CHIP_POS[usize::from(frame.chip_focus).min(p::CHIPS - 1)];
            let (x, y) = p::chip_xy(col, row);
            // The ring sits behind the tiles: it is a larger rectangle at the same place.
            self.rect(p::IDX_CHIP_HI, x, y, p::CHIP_SIZE + 16.0, p::CHIP_SIZE + 16.0, p::ACCENT);
        } else {
            self.hide(p::IDX_CHIP_HI);
        }
    }

    unsafe fn draw_tiles(&self, frame: &Frame) {
        let shown = frame.view == View::Tiles;
        let (cols, rows) = (f32::from(frame.tile_cols.max(1)), f32::from(frame.tile_rows.max(1)));
        let (cell_w, cell_h) = ((p::ROW_RIGHT - p::ROW_LEFT) / cols, 620.0 / rows);
        let centre = |i: usize| {
            let (c, r) = (i % usize::from(frame.tile_cols.max(1)), i / usize::from(frame.tile_cols.max(1)));
            (p::ROW_LEFT + (c as f32 + 0.5) * cell_w, 320.0 - (r as f32 + 0.5) * cell_h)
        };
        for i in 0..p::TILES {
            if !shown || i >= usize::from(frame.tile_count) {
                self.hide(p::IDX_TILE + i);
                self.hide(p::IDX_TILE_LABEL + i);
                continue;
            }
            let (x, y) = centre(i);
            self.rect(p::IDX_TILE + i, x, y, cell_w - 10.0, cell_h - 8.0, p::HIGHLIGHT);
            self.label(p::IDX_TILE_LABEL + i, frame.tiles[i].units(), x, y, cell_w - 18.0, p::TEXT);
        }
        if shown && frame.tile_count > 0 {
            let (x, y) = centre(usize::from(frame.tile_focus));
            self.rect(p::IDX_TILE_HI, x, y, cell_w + 2.0, cell_h + 2.0, p::ACCENT);
        } else {
            self.hide(p::IDX_TILE_HI);
        }
    }
}