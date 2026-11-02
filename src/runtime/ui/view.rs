//! Draws one menu frame into the layout's panes (SPEC-core-03 §4.2): text, colours, positions of the
//! highlight bars. Runs inside the layout tick, only when the worker published a new frame.

use skyline::nn::ui2d::Pane;

use crate::model::frame::{Frame, Tone};
use crate::model::panes as p;
use crate::runtime::ui::pane;

/// The resolved panes of the menu in the layout now showing.
pub struct Panes {
    pub container: *mut Pane,
    pub all: [*mut Pane; p::COUNT],
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
    units as f32 * p::FONT.0 * 1.12
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
                    pane::set_width(hi, width + 44.0);
                }
            }
            left += width + 70.0;
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

        // Rows.
        for i in 0..p::ROWS {
            if i < usize::from(frame.row_count) {
                let row = &frame.rows[i];
                self.text(p::row_label_index(i), row.label.units(), tone_color(row.label_tone));
                self.text(p::row_value_index(i), row.value.units(), tone_color(row.value_tone));
            } else {
                self.text(p::row_label_index(i), &[], p::TEXT);
                self.text(p::row_value_index(i), &[], p::TEXT);
            }
        }
        match frame.focus {
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