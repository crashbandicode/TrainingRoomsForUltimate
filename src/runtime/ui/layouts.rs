//! The Saved layouts screen of Break the Targets (btt-5, `artifacts/rc14-btt-layouts/DESIGN.md`) on the menu worker. The list
//! is this stage's layout files in Recording's order and look (`btt::listed`, `screens::hot_list_frame`: the hot five in the green
//! outline, the hairline under the fifth). A on a layout opens its own screen (Load, Rename, Duplicate, Delete) and Delete asks
//! once: the Setup library's panes (`editors::setups::Pane`) on the library panel. Rename uses the shared keyboard. The file work
//! is done here, as the Setup library's is; a loaded layout goes to the game thread through `btt::CMDS`, and `btt::reread` has
//! the writer list the stage again for the chords.

use std::path::Path;

use super::external;
use crate::editors::setups::{Pane, Step, LAYOUT};
use crate::engine::macro_ui::macro_panel::{LibraryRow, View};
use crate::model::btt::{self, targets_label, Layout};
use crate::model::frame::Frame;
use crate::model::menu::{input::Button, table::Editor, Menu};
use crate::model::notify::DEFAULT_COLOR;
use crate::model::screens::hot_list_frame;
use crate::model::storage::ROOT;
use crate::runtime::features::btt::{reread, stage, stage_label, CMDS};
use crate::runtime::features::notices;
use crate::runtime::macros::named_macros;

/// This screen's standing notice (free beside the Combos' 0x2A and 0x2B, Recording's 0x2C and Eggs-ercise's 0x2E).
const NOTICE_ID: u8 = 0x2D;
const TITLE: &str = "Saved layouts";
/// Name tokens of this screen: apart from the Setup library's revisions and the combos' high bit.
const TOKEN_BASE: u64 = 1 << 62;

/// A result of the screen: one notice.
pub fn note(text: &str) {
    notices::upsert(NOTICE_ID, TITLE, text, 180, DEFAULT_COLOR);
}

pub fn is_layouts(editor: Editor) -> bool {
    editor == Editor::Layouts
}

fn root() -> &'static Path {
    Path::new(ROOT)
}

/// The pane up (`None`: the screen is closed), this stage's layouts in the list's order, how many are hot, the list's cursor,
/// the files skipped, and the rename the keyboard is open for (its token and the layout's id).
#[derive(Default)]
pub struct Ui {
    pane: Option<Pane>,
    rows: Vec<(String, Layout)>,
    hot: usize,
    cursor: usize,
    skipped: usize,
    naming: Option<(u64, String)>,
    serial: u64,
}

impl Ui {
    /// The row was entered: this stage's list opens (true), or the note says why it did not.
    pub fn open(&mut self) -> bool {
        self.cursor = 0;
        self.show(Pane::List).map_err(|why| note(&why)).is_ok()
    }

    /// Opens `pane` on the library panel; the list is read from the card again. The list is drawn as a list (`paint`), the
    /// panel under it keeps the layer open.
    fn show(&mut self, pane: Pane) -> Result<(), String> {
        let (title, rows) = match &pane {
            Pane::List => {
                let (all, skipped) = btt::all(root());
                (self.rows, self.hot) = btt::listed(&all, stage());
                (self.skipped, self.cursor) = (skipped, self.cursor.min(self.rows.len().saturating_sub(1)));
                let rows = self.rows.iter().map(|(id, l)| LibraryRow { id: id.clone(), label: l.name.clone() });
                (TITLE.to_owned(), rows.collect())
            }
            Pane::Own(id) | Pane::Delete(id) => (self.layout(id)?.name.clone(), pane.rows(&LAYOUT)),
        };
        named_macros::open_external_library(rows, 0, title, None);
        if !external::panel_up() {
            return Err("Wait for the practice fighters to finish loading".into()); // restriction: combo-save-wait-load
        }
        self.pane = Some(pane);
        Ok(())
    }

    fn layout(&self, id: &str) -> Result<&Layout, String> {
        self.rows.iter().find(|r| r.0 == id).map(|r| &r.1).ok_or_else(|| format!("No layout {id}"))
    }

    /// The list takes Up, Down, A and B itself (one row a step, as Recording's lists); a layout's own screen and its question
    /// are the panel's, and B there steps back one pane.
    pub fn press(&mut self, button: Button) {
        let last = self.rows.len().saturating_sub(1);
        match (&self.pane, button) {
            (Some(Pane::List), Button::Up) => self.cursor = self.cursor.saturating_sub(1),
            (Some(Pane::List), Button::Down) => self.cursor = (self.cursor + 1).min(last),
            (Some(Pane::List), Button::A) => {
                if let Some(Err(why)) = self.rows.get(self.cursor).map(|r| r.0.clone()).map(|id| self.show(Pane::Own(id))) {
                    note(&why);
                }
            }
            (Some(Pane::List), Button::B) | (None, _) => external::press_panel(button),
            (Some(Pane::List), _) => {}
            (Some(pane), Button::B) => {
                if let Some(Err(why)) = pane.back().map(|back| self.show(back)) {
                    note(&why);
                }
            }
            _ => external::press_panel(button),
        }
    }

    /// Every worker poll: a confirmed rename, and A on a layout's own screen or its question. Returns a note to show.
    pub fn tick(&mut self, menu: Option<&mut Menu>) -> Option<String> {
        if let Some((token, id)) = self.naming.clone() {
            if let Some(name) = named_macros::take_confirmed_name_for(token) {
                self.naming = None;
                return Some(self.changed(btt::use_file(root(), &id, Some(&name)).map(|()| format!("Renamed to {}", name.trim()))));
            }
            if !named_macros::external_name_open(token) {
                self.naming = None;
            }
            return None;
        }
        let pane = self.pane.clone().filter(|p| *p != Pane::List)?;
        // X and Y mean nothing on these panes; A's choice closed the panel.
        let _ = (named_macros::take_list_rename(), named_macros::take_list_action());
        let row = named_macros::take_list_choice()?;
        self.pane = None;
        match pane.leads_to(&row) {
            Step::Open(next) => self.show(next).err(),
            Step::Choose(id, _) => self.load(&id, menu),
            Step::Rename(id) => {
                let layout = self.layout(&id).ok()?.clone();
                self.serial += 1;
                self.naming = Some((TOKEN_BASE | self.serial, id));
                named_macros::offer_external_name_titled(TOKEN_BASE | self.serial, "Name layout", &layout.name, &format!("Renaming · {}", targets_label(layout.targets.len())));
                None
            }
            Step::Duplicate(id) => {
                self.cursor = 0;
                let made = btt::duplicate(root(), &id, &stage_label(stage())).map(|_| "Duplicated: the copy is at the top".to_owned());
                self.back_to_list(made)
            }
            Step::Remove(id) => {
                let name = self.layout(&id).map(|l| l.name.clone()).unwrap_or_default();
                self.back_to_list(btt::library(root()).delete(&id).map(|()| format!("Deleted {name}")))
            }
        }
    }

    /// Load: the game thread takes the targets, the file is used (top of the five), and Break the Targets turns On, so closing
    /// the menu opens it at Ready.
    fn load(&mut self, id: &str, menu: Option<&mut Menu>) -> Option<String> {
        let layout = self.layout(id).ok()?.clone();
        let hot = btt::five(&[(id.to_owned(), layout.clone())], 1)[0]?;
        CMDS.post(hot);
        if let Some(menu) = menu {
            menu.btt_on();
        }
        let used = btt::use_file(root(), id, None).map(|()| format!("Loaded {}: close the menu to start", layout.name));
        Some(self.changed(used))
    }

    /// A file changed: the chords' five are listed again; the note is the result or why it failed.
    fn changed(&self, result: Result<String, impl ToString>) -> String {
        reread();
        result.unwrap_or_else(|why| why.to_string())
    }

    fn back_to_list(&mut self, result: Result<String, impl ToString>) -> Option<String> {
        let text = self.changed(result);
        Some(self.show(Pane::List).err().map_or(text.clone(), |why| format!("{text}; {why}")))
    }

    /// The list, drawn like the Recording lists, when it is the pane up; false for the other panes (the panel's own frame).
    pub fn paint(&self, menu: &Menu, frame: &mut Frame) -> bool {
        if self.pane != Some(Pane::List) || named_macros::suspended() {
            return false;
        }
        let rows: Vec<(String, String)> = self.rows.iter().map(|(_, l)| (l.name.clone(), targets_label(l.targets.len()))).collect();
        let footer = btt::list_footer(&self.rows, self.cursor, self.skipped);
        hot_list_frame(menu, &format!("{TITLE} · {}", stage_label(stage())), &footer, &rows, self.cursor, self.hot, frame);
        true
    }

    /// A layout's own screen or its question: the panel's frame, with the layout and what the row under the cursor does.
    pub fn view(&self) -> Option<View> {
        let mut view = external::panel_view_now()?;
        if let Some(pane @ (Pane::Own(id) | Pane::Delete(id))) = &self.pane {
            let label = self.layout(id).map(|l| format!("{} · {}", l.name, targets_label(l.targets.len()))).unwrap_or_default();
            view.footer = pane.footer(&LAYOUT, &label, view.selected);
        }
        Some(view)
    }

    /// The panel left the screen (B, or the menu closed).
    pub fn leave_panel(&mut self) {
        if !named_macros::suspended() {
            self.pane = None;
        }
    }
}
