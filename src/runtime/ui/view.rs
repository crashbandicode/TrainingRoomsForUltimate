//! Draws one menu frame into the layout's panes (SPEC-core-03 §4.2): text, colours, positions of the
//! highlight bars. Runs inside the layout tick, only when the worker published a new frame.

use skyline::nn::ui2d::Pane;

use crate::model::frame::{Frame, Tone, View};
use crate::model::menu::options::DIRECTIONS;
use crate::model::menu::CHIP_POS;
use crate::model::screens::{TILE_GREYED, TILE_HIDDEN};
use crate::model::panes as p;

/// The fork's role colours (`src/training/ui/roster_grid.rs:9-10`): the roster grid's Player and CPU badges, stripes,
/// Solo Popo markers and cursor tint.
const PLAYER: [u8; 4] = [255, 83, 92, 255];
const CPU: [u8; 4] = [75, 164, 255, 255];
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

        // The overrides screen keeps its situation line as the list's first row.
        let list = matches!(frame.view, View::List | View::Overrides | View::Clip);
        self.draw_chips(frame);
        self.draw_tiles(frame);
        self.draw_delay(frame);
        self.draw_overrides(frame);
        self.draw_clip(frame);
        // Rows; on the direction grid the same panes carry the "Selected (weight)" panel at the right of the chips.
        let panel = if frame.view == View::Chips { selected_panel(frame) } else { [None; p::ROWS] };
        for (i, line) in panel.into_iter().enumerate() {
            if list && i < usize::from(frame.row_count) {
                let row = &frame.rows[i];
                self.place(p::row_label_index(i), p::ROW_LEFT + p::ROW_LABEL_W / 2.0, p::row_y(i), p::ROW_LABEL_W);
                self.place(p::row_value_index(i), p::ROW_RIGHT - p::ROW_VALUE_W / 2.0, p::row_y(i) - p::VALUE_DROP, p::ROW_VALUE_W);
                self.text(p::row_label_index(i), row.label.units(), tone_color(row.label_tone));
                self.text(p::row_value_index(i), row.value.units(), tone_color(row.value_tone));
            } else if frame.view == View::Delay && i < 4 {
                // The delay popup's four text lines: the checklist header and count, the range header and value, and the
                // two handle labels under their handles.
                let line = &frame.rows[i];
                let (lx, lw, ly, vx, vw, vy) = delay_text_slot(i, frame);
                self.place(p::row_label_index(i), lx, ly, lw);
                self.place(p::row_value_index(i), vx, vy, vw);
                self.text(p::row_label_index(i), line.label.units(), tone_color(line.label_tone));
                self.text(p::row_value_index(i), line.value.units(), tone_color(line.value_tone));
            } else if let Some((name, weight, tone)) = line {
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

        if frame.white_cursor {
            self.show(p::IDX_ROW_HI, false);
        }
        self.draw_chrome(frame);
        self.text(p::IDX_HELP, frame.help[0].units(), p::TEXT);
        self.text(p::IDX_HELP + 1, frame.help[1].units(), p::TEXT);
        self.text(p::IDX_HINT, frame.hint.units(), p::TEXT_DIM);
        self.draw_marks(frame);
    }

    /// Green row plates, the hot outline, the hairline and the white cursor. Hidden until a recording screen asks.
    unsafe fn draw_chrome(&self, frame: &Frame) {
        for index in p::IDX_GREEN..p::IDX_ROW {
            self.show(index, false);
        }
        for (i, on) in frame.green.iter().enumerate() {
            if *on {
                self.rect_at(
                    p::IDX_GREEN + i,
                    (p::ROW_LEFT + p::ROW_RIGHT) / 2.0,
                    p::row_y(i),
                    p::ROW_RIGHT - p::ROW_LEFT,
                    p::ROW_H,
                    p::GREEN_ROW,
                );
            }
        }
        if let Some(slot) = frame.hairline {
            let y = p::row_y(usize::from(slot)) - p::ROW_H / 2.0;
            self.rect_at(p::IDX_HAIRLINE, (p::ROW_LEFT + p::ROW_RIGHT) / 2.0, y, p::ROW_RIGHT - p::ROW_LEFT, 2.0, p::RULE);
        }
        for (n, block) in frame.outline.iter().enumerate() {
            if let Some((first, count)) = *block {
                self.stroke(p::IDX_OUTLINE + n * 4, first, count, p::TEXT_ON);
            }
        }
        if frame.white_cursor {
            if let Some(slot) = frame.focus {
                let x = p::ROW_LEFT + 4.0;
                self.rect_at(p::IDX_CURSOR, x, p::row_y(usize::from(slot)), 6.0, p::ROW_H, p::CURSOR_WHITE);
                self.stroke(p::IDX_CURSOR + 1, slot, 1, p::CURSOR_WHITE);
            }
        }
    }

    unsafe fn stroke(&self, index: usize, first: u8, count: u8, color: [u8; 4]) {
        let last = usize::from(first) + usize::from(count.saturating_sub(1));
        let top = p::row_y(usize::from(first)) + p::ROW_H / 2.0;
        let bottom = p::row_y(last) - p::ROW_H / 2.0;
        let y = (top + bottom) / 2.0;
        let h = (top - bottom).abs().max(p::ROW_H);
        let x = (p::ROW_LEFT + p::ROW_RIGHT) / 2.0;
        let w = p::ROW_RIGHT - p::ROW_LEFT;
        let edge = 2.0;
        self.rect_at(index, p::ROW_LEFT + edge / 2.0, y, edge, h, color);
        self.rect_at(index + 1, p::ROW_RIGHT - edge / 2.0, y, edge, h, color);
        self.rect_at(index + 2, x, top - edge / 2.0, w, edge, color);
        self.rect_at(index + 3, x, bottom + edge / 2.0, w, edge, color);
    }

    unsafe fn rect_at(&self, index: usize, x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) {
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

    /// Floppy and loop pictures, and the smaller pop-up background. Hidden marks stay hidden.
    unsafe fn draw_marks(&self, frame: &Frame) {
        for i in 0..p::FLOPPIES {
            let icon = frame.icons[i];
            let index = p::IDX_FLOPPY + i;
            self.show(index, icon.on);
            if icon.on {
                self.at(index, icon.x, icon.y);
                let pane = self.all[index];
                if !pane.is_null() {
                    pane::set_rect_color(pane, p::linear(icon.color));
                }
            }
        }
        for i in 0..p::LOOPS {
            let icon = frame.icons[p::FLOPPIES + i];
            let index = p::IDX_LOOP + i;
            self.show(index, icon.on);
            if icon.on {
                self.at(index, icon.x, icon.y);
                let pane = self.all[index];
                if !pane.is_null() {
                    pane::set_rect_color(pane, p::linear(icon.color));
                }
            }
        }
        let bg = self.all[p::IDX_BG];
        if !bg.is_null() {
            if frame.popup {
                pane::set_width(bg, 1280.0);
                pane::set_height(bg, 760.0);
            } else {
                pane::set_width(bg, p::PANEL_W);
                pane::set_height(bg, p::PANEL_H);
            }
        }
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
            // DI / SDI: the pointer carries the game's own effect texture (`chips`), turned to the chip's direction.
            let layer = if i == crate::runtime::ui::chips::NEUTRAL { None } else { crate::runtime::ui::chips::layer() };
            match (CHIP_ANGLE[i], layer) {
                (Some(angle), Some(layer)) => {
                    let at = crate::model::effect_chips::place(layer.visual, angle, x, y, p::CHIP_SIZE, layer.width, layer.height);
                    let tint = if weight != 0 { layer.tint } else { [layer.tint[0] / 2, layer.tint[1] / 2, layer.tint[2] / 2, 255] };
                    self.rect(p::IDX_CHIP_NEEDLE + i, at.x, at.y, at.w, at.h, tint);
                    pane::set_rot_z(self.all[p::IDX_CHIP_NEEDLE + i], at.rot);
                }
                (Some(angle), None) => {
                    let radians = angle.to_radians();
                    let (dx, dy) = (radians.cos(), radians.sin());
                    self.rect(p::IDX_CHIP_NEEDLE + i, x + dx * 34.0, y + 12.0 + dy * 34.0, 12.0, 52.0, if weight != 0 { p::TEXT } else { p::TEXT_DIM });
                    pane::set_rot_z(self.all[p::IDX_CHIP_NEEDLE + i], angle - 90.0);
                }
                (None, _) => {
                    self.rect(p::IDX_CHIP_NEEDLE + i, x, y + 12.0, 30.0, 30.0, if weight != 0 { p::TEXT } else { p::TEXT_DIM });
                    pane::set_rot_z(self.all[p::IDX_CHIP_NEEDLE + i], 0.0);
                }
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
        // One archive-owned picture per item, shared across all three handheld pages.
        for i in 0..p::ITEM_ICONS {
            self.hide(p::IDX_ITEM_ICON + i);
        }
        let shown = frame.view == View::Tiles;
        let (cols, rows) = (f32::from(frame.tile_cols.max(1)), f32::from(frame.tile_rows.max(1)));
        let (cell_w, cell_h) = ((p::ROW_RIGHT - p::ROW_LEFT) / cols, p::TILE_AREA_H / rows);
        let centre = |i: usize| {
            let per_row = usize::from(frame.tile_cols.max(1));
            let (c, r) = (i % per_row, i / per_row);
            // The roster's short last row sits centred under the full rows (the character select's own layout).
            let count = usize::from(frame.tile_count);
            let last_len = count - (count.saturating_sub(1) / per_row) * per_row;
            let shift = if frame.tile_centre_last && r == count.saturating_sub(1) / per_row { (per_row - last_len) as f32 / 2.0 } else { 0.0 };
            (p::ROW_LEFT + (c as f32 + shift + 0.5) * cell_w, 320.0 - (r as f32 + 0.5) * cell_h)
        };
        for i in 0..p::TILES {
            if !shown || i >= usize::from(frame.tile_count) || frame.tile_state[i] == TILE_HIDDEN {
                self.hide(p::IDX_TILE + i);
                self.hide(p::IDX_TILE_LABEL + i);
                continue;
            }
            let (x, y) = centre(i);
            if frame.tile_item_grid && usize::from(frame.tile_n[i]) < p::ITEM_ICONS {
                self.rect(p::IDX_TILE + i, x, y, cell_w - 10.0, cell_h - 8.0, p::HIGHLIGHT);
                let size = 64.0_f32.min(cell_h - 12.0).min(cell_w / 3.0);
                let left = x - cell_w / 2.0 + 10.0;
                self.rect(p::IDX_ITEM_ICON + usize::from(frame.tile_n[i]), left + size / 2.0, y, size, size, [255; 4]);
                let label_left = left + size + 8.0;
                let label_w = cell_w - size - 28.0;
                self.label(p::IDX_TILE_LABEL + i, frame.tiles[i].units(), label_left + label_w / 2.0, y, label_w, p::TEXT);
                continue;
            }
            // A roster tile the editing role cannot choose draws greyed (the fork's `roster_grid.rs` 197-206 tint).
            let greyed = frame.tile_roster_grid && frame.tile_state[i] == TILE_GREYED;
            if frame.tile_roster_grid && crate::runtime::ui::thumbs::is_bound(i) {
                let (icon, label, _) = p::roster_tile(cell_w, cell_h, true);
                let tint = if greyed { [110, 110, 110, 150] } else { [255, 255, 255, 255] };
                self.rect(p::IDX_TILE + i, x, y + icon.y, icon.w, icon.h, tint);
                self.label(p::IDX_TILE_LABEL + i, frame.tiles[i].units(), x, y + label.y, label.w, if greyed { p::TEXT_OFF } else { p::TEXT });
                continue;
            }
            if frame.tile_stage_grid && crate::runtime::ui::thumbs::is_bound(i) {
                // The tile shows its stage's own picture (108 x 90, aspect kept): docked tiles put it at the left with the
                // name beside it, handheld tiles (tall) put it over the name.
                let tall = cell_h >= 100.0;
                let (image_h, image_x, image_y) = if tall {
                    (cell_h - 48.0, x, y + 20.0)
                } else {
                    (cell_h - 10.0, x - cell_w / 2.0 + 8.0 + (cell_h - 10.0) * 0.6, y)
                };
                let image_w = image_h * 1.2;
                self.rect(p::IDX_TILE + i, image_x, image_y, image_w, image_h, [255, 255, 255, 255]);
                let (label_x, label_w, label_y) = if tall {
                    (x, cell_w - 18.0, y - cell_h / 2.0 + 20.0)
                } else {
                    let left = x - cell_w / 2.0 + 16.0 + image_w;
                    let width = cell_w - 24.0 - image_w;
                    (left + width / 2.0, width, y)
                };
                self.label(p::IDX_TILE_LABEL + i, frame.tiles[i].units(), label_x, label_y, label_w, p::TEXT);
                continue;
            }
            self.rect(p::IDX_TILE + i, x, y, cell_w - 10.0, cell_h - 8.0, p::HIGHLIGHT);
            self.label(p::IDX_TILE_LABEL + i, frame.tiles[i].units(), x, y, cell_w - 18.0, if greyed { p::TEXT_OFF } else { p::TEXT });
        }
        if shown && frame.tile_count > 0 {
            let (x, y) = centre(usize::from(frame.tile_focus));
            // The fork's cursor tint (roster_grid.rs:279-291): the active cell's highlight takes the
            // editing role's colour at alpha 80, so the same-role order badge stays legible over it.
            let [r, g, b, _] = [PLAYER, CPU][usize::from(frame.tile_cpu)];
            let tint = [p::ACCENT, [r, g, b, 80]][usize::from(frame.tile_roster_grid)];
            self.rect(p::IDX_TILE_HI, x, y, cell_w + 2.0, cell_h + 2.0, tint);
        } else {
            self.hide(p::IDX_TILE_HI);
        }
        self.draw_roster_badges(frame, &centre, (cell_w, cell_h));
    }

    /// The roster grid's per-role marks in the fork's geometry (`scripts/roster-grid/main.rs:299-348`), Player
    /// left red, CPU right blue, both roles at once: the All-stripes 4x86 at x±56, the order badges 52x28 at
    /// x±36/y+35, and the Solo Popo "S" 40x28 at x±36 on the Ice Climbers cell (kind 0x4b), off its name label
    /// (`panes::roster_tile`). Empty marks hide.
    unsafe fn draw_roster_badges(&self, frame: &Frame, centre: &dyn Fn(usize) -> (f32, f32), cell: (f32, f32)) {
        let roster = frame.view == View::Tiles && frame.tile_roster_grid;
        let count = usize::from(frame.tile_count);
        let ic = (0..count).find(|i| roster && frame.tile_n[*i] == 0x4b);
        for (r, (side, colour)) in [(-1.0, PLAYER), (1.0, CPU)].into_iter().enumerate() {
            for i in 0..p::TILES {
                let on = roster && i < count && frame.tile_state[i] != TILE_HIDDEN;
                let ((x, y), b, at) = (centre(i), &frame.tile_badges[r][i], r * p::TILES + i);
                let (stripe, badge) = (p::IDX_TILE_STRIPE + at, p::IDX_TILE_BADGE + at);
                self.rect(stripe, x + side * 56.0, y, 4.0, 86.0, colour);
                self.show(stripe, on && frame.tile_stripes[r][i]);
                self.label(badge, b.units(), x + side * 36.0, y + 35.0, 52.0, colour);
                self.show(badge, on && !b.is_empty());
            }
            let ((x, y), solo) = (centre(ic.unwrap_or(0)), p::IDX_TILE_SOLO + r);
            let pictured = ic.is_some_and(crate::runtime::ui::thumbs::is_bound);
            let mark = p::roster_tile(cell.0, cell.1, pictured).2[r];
            self.label(solo, &[83], x + mark.x, y + mark.y, mark.w, colour); // "S"
            self.show(solo, ic.is_some() && frame.tile_solo[r]);
        }
    }
}

// ---- the delay popup (mockup 19): a grid of boxes over the min-max range control ----------------------------
const DELAY_CELL_W: f32 = (p::ROW_RIGHT - p::ROW_LEFT) / 8.0;
const DELAY_FIRST_Y: f32 = 250.0;
const DELAY_PITCH: f32 = 58.0;
const DELAY_BAR_L: f32 = -300.0;
const DELAY_BAR_W: f32 = 1000.0;
const DELAY_BAR_Y: f32 = -95.0;
const DELAY_RANGE_HEAD_Y: f32 = -10.0;
const DELAY_HANDLE_LABEL_Y: f32 = -165.0;

/// The x of the handle standing on value index `index` of a set whose last index is `top`.
fn delay_handle_x(index: u8, top: u8) -> f32 {
    DELAY_BAR_L + DELAY_BAR_W * f32::from(index) / f32::from(top.max(1))
}

/// Label slot (x, width, y) and value slot (x, width, y) of the popup's text line `i`.
fn delay_text_slot(i: usize, frame: &Frame) -> (f32, f32, f32, f32, f32, f32) {
    let right = |y: f32| (p::ROW_RIGHT - 150.0, 300.0, y - p::VALUE_DROP);
    match i {
        0 => (p::ROW_LEFT + 300.0, 600.0, 312.0, right(312.0).0, right(312.0).1, right(312.0).2),
        1 => (p::ROW_LEFT + 300.0, 600.0, DELAY_RANGE_HEAD_Y, right(DELAY_RANGE_HEAD_Y).0, right(DELAY_RANGE_HEAD_Y).1, right(DELAY_RANGE_HEAD_Y).2),
        _ => {
            let at = if i == 2 { frame.range.min } else { frame.range.max };
            let x = delay_handle_x(at, frame.range.top);
            (x, 220.0, DELAY_HANDLE_LABEL_Y, x, 10.0, DELAY_HANDLE_LABEL_Y)
        }
    }
}

impl Panes {
    unsafe fn draw_delay(&self, frame: &Frame) {
        let shown = frame.view == View::Delay;
        if !shown {
            return;
        }
        let rows = usize::from(frame.tile_rows.max(1));
        for i in 0..p::TILES {
            if i >= usize::from(frame.tile_count) {
                continue; // draw_tiles hid it
            }
            let (c, r) = (i / rows, i % rows);
            let (x, y) = (p::ROW_LEFT + (c as f32 + 0.5) * DELAY_CELL_W, DELAY_FIRST_Y - r as f32 * DELAY_PITCH);
            let (fill, text) = match frame.tile_state[i] {
                1 => (p::ACCENT, p::TEXT),
                2 => (p::HIGHLIGHT, p::TEXT_OFF),
                _ => (p::HIGHLIGHT, p::TEXT),
            };
            self.rect(p::IDX_TILE + i, x, y, DELAY_CELL_W - 12.0, DELAY_PITCH - 10.0, fill);
            self.label(p::IDX_TILE_LABEL + i, frame.tiles[i].units(), x, y, DELAY_CELL_W - 20.0, text);
        }
        if frame.tile_focus != 255 && frame.tile_count > 0 {
            let i = usize::from(frame.tile_focus);
            let (c, r) = (i / rows, i % rows);
            self.rect(p::IDX_TILE_HI, p::ROW_LEFT + (c as f32 + 0.5) * DELAY_CELL_W, DELAY_FIRST_Y - r as f32 * DELAY_PITCH, DELAY_CELL_W - 2.0, DELAY_PITCH - 2.0, p::ACCENT_RING);
        } else {
            self.hide(p::IDX_TILE_HI);
        }
        // The range control: a track, the span between the handles, the two handles, a ring on the focused one.
        let range = frame.range;
        let (track, span, handle) = if range.grey { (p::HIGHLIGHT, p::HIGHLIGHT, p::TEXT_OFF) } else { ([70, 78, 92, 255], p::ACCENT, p::TEXT) };
        self.rect(p::IDX_CHIP, DELAY_BAR_L + DELAY_BAR_W / 2.0, DELAY_BAR_Y, DELAY_BAR_W, 8.0, track);
        let (lo, hi) = if range.set { (delay_handle_x(range.min, range.top), delay_handle_x(range.max, range.top)) } else { (DELAY_BAR_L, DELAY_BAR_L + DELAY_BAR_W) };
        if range.set {
            self.rect(p::IDX_CHIP + 1, (lo + hi) / 2.0, DELAY_BAR_Y, (hi - lo).max(4.0), 8.0, span);
        } else {
            self.hide(p::IDX_CHIP + 1);
        }
        let held = if range.editing { p::ACCENT } else { handle };
        self.rect(p::IDX_CHIP + 2, lo, DELAY_BAR_Y, 26.0, 44.0, if range.focus == 1 { held } else { handle });
        self.rect(p::IDX_CHIP + 3, hi, DELAY_BAR_Y, 26.0, 44.0, if range.focus == 2 { held } else { handle });
        match range.focus {
            1 => self.rect(p::IDX_CHIP_HI, lo, DELAY_BAR_Y, 44.0, 64.0, p::ACCENT_RING),
            2 => self.rect(p::IDX_CHIP_HI, hi, DELAY_BAR_Y, 44.0, 64.0, p::ACCENT_RING),
            _ => self.hide(p::IDX_CHIP_HI),
        }
        for i in 4..p::CHIPS {
            self.hide(p::IDX_CHIP + i);
        }
    }
}

// ---- the situation overrides screen (mockup 06): columns of action boxes under the situation line -------
const OVERRIDE_COL_W: f32 = 420.0;
const OVERRIDE_COL_X: [f32; 3] = [-230.0, 210.0, 650.0];
const OVERRIDE_FIRST_Y: f32 = 250.0;
const OVERRIDE_PITCH: f32 = 54.0;

impl Panes {
    unsafe fn draw_overrides(&self, frame: &Frame) {
        if frame.view != View::Overrides {
            return;
        }
        let rows = usize::from(frame.tile_rows.max(1));
        let at = |i: usize| (OVERRIDE_COL_X[(i / rows).min(2)], OVERRIDE_FIRST_Y - (i % rows) as f32 * OVERRIDE_PITCH);
        for i in 0..p::TILES {
            if i >= usize::from(frame.tile_count) {
                continue; // draw_tiles hid it
            }
            let (x, y) = at(i);
            let on = frame.tile_state[i] == 1;
            self.rect(p::IDX_TILE + i, x, y, OVERRIDE_COL_W - 12.0, OVERRIDE_PITCH - 12.0, if on { p::ACCENT } else { p::HIGHLIGHT });
            self.label(p::IDX_TILE_LABEL + i, frame.tiles[i].units(), x, y, OVERRIDE_COL_W - 24.0, p::TEXT);
        }
        if frame.tile_focus != 255 {
            let (x, y) = at(usize::from(frame.tile_focus));
            self.rect(p::IDX_TILE_HI, x, y, OVERRIDE_COL_W, OVERRIDE_PITCH - 4.0, p::ACCENT_RING);
        } else {
            self.hide(p::IDX_TILE_HI);
        }
    }
}
// ---- the clip editor (the replay window): its lines are the list rows, the timeline sits under them ------------------
const CLIP_BAR_L: f32 = -300.0;
const CLIP_BAR_W: f32 = 1000.0;
const CLIP_BAR_Y: f32 = -90.0;

/// The x of frame `at` of a capture of `total` frames on the timeline.
fn clip_x(at: u32, total: u32) -> f32 {
    CLIP_BAR_L + CLIP_BAR_W * at.min(total) as f32 / total.max(1) as f32
}

impl Panes {
    unsafe fn draw_clip(&self, frame: &Frame) {
        if frame.view != View::Clip {
            return;
        }
        use core::fmt::Write;
        let bar = frame.clip;
        let (start, end, now) = (clip_x(bar.start, bar.total), clip_x(bar.end, bar.total), clip_x(bar.cursor, bar.total));
        let held = |handle: u8| if bar.handle == handle { p::ACCENT_RING } else { p::TEXT };
        self.rect(p::IDX_CHIP, CLIP_BAR_L + CLIP_BAR_W / 2.0, CLIP_BAR_Y, CLIP_BAR_W, 8.0, [70, 78, 92, 255]);
        self.rect(p::IDX_CHIP + 1, (start + end) / 2.0, CLIP_BAR_Y, (end - start).max(4.0), 8.0, p::ACCENT);
        self.rect(p::IDX_CHIP + 2, start, CLIP_BAR_Y, 22.0, 44.0, held(1));
        self.rect(p::IDX_CHIP + 3, end, CLIP_BAR_Y, 22.0, 44.0, held(2));
        self.rect(p::IDX_CHIP + 4, now, CLIP_BAR_Y, 6.0, 96.0, if bar.playing { p::TEXT_ON } else { held(0) });
        for i in 5..p::CHIPS {
            self.hide(p::IDX_CHIP + i);
        }
        self.hide(p::IDX_CHIP_HI);
        for i in 0..p::CHIPS {
            self.hide(p::IDX_CHIP_BADGE + i);
            self.hide(p::IDX_CHIP_NEEDLE + i);
            if i >= 3 {
                self.hide(p::IDX_CHIP_LABEL + i);
            }
        }
        let marks = [(start, CLIP_BAR_Y - 52.0, bar.start, "in"), (end, CLIP_BAR_Y - 52.0, bar.end, "out"), (now, CLIP_BAR_Y + 70.0, bar.cursor, "now")];
        for (slot, (x, y, frames, name)) in marks.into_iter().enumerate() {
            let mut text = crate::model::frame::Text::<16>::new();
            let _ = write!(text, "{name} {frames}");
            self.label(p::IDX_CHIP_LABEL + slot, text.units(), x, y, 200.0, p::TEXT_DIM);
        }
    }
}
