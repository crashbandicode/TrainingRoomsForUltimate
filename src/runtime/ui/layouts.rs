//! The saved-file list screen on the menu worker, shared by Break the Targets' Saved layouts (btt-5,
//! `artifacts/rc14-btt-layouts/DESIGN.md`) and Parry practice's saved practices (`artifacts/rc15-parry/DESIGN.md` §5):
//! one `SavedList` per kind of file. The list is the files in Recording's order and look (`screens::hot_list_frame`: the
//! hot five in the green outline, the hairline under the fifth). A on a file opens its own screen (Load, Rename,
//! Duplicate, Delete) and Delete asks once: the Setup library's panes (`editors::setups::Pane`) on the library panel. Rename
//! uses the shared keyboard. The file work is done here, as the Setup library's is.

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
/// Name tokens of this screen: apart from the Setup library's revisions and the combos' high bit.
const TOKEN_BASE: u64 = 1 << 62;


pub fn is_layouts(editor: Editor) -> bool {
    matches!(editor, Editor::Layouts | Editor::ParrySaved | Editor::ParrySave)
}

fn root() -> &'static Path {
    Path::new(ROOT)
}

/// One kind of saved file the screen lists. Rows are (id, name, detail); every file operation returns its note.
pub trait SavedList {
    /// The panel's title, and the list's title above the rows.
    fn title(&self) -> &'static str;
    fn list_title(&self) -> String {
        self.title().to_owned()
    }
    /// The keyboard's title for a rename or a new name.
    fn naming(&self) -> &'static str;
    /// The files in the list's order, how many are the hot five, and how many files were skipped.
    fn saved_rows(&mut self, root: &Path) -> (Vec<(String, String, String)>, usize, usize);
    /// The list's two footer lines for the row under `cursor`.
    fn footer(&self, cursor: usize, skipped: usize) -> String;
    fn saved_rename(&mut self, root: &Path, id: &str, name: &str) -> Result<String, String>;
    fn saved_copy(&mut self, root: &Path, id: &str) -> Result<String, String>;
    fn saved_remove(&mut self, root: &Path, id: &str) -> Result<String, String>;
    fn saved_use(&mut self, root: &Path, id: &str, menu: Option<&mut Menu>) -> Result<String, String>;
    /// A new file named `name` (a "Save this …" row); `None` when the kind has no such row.
    fn saved_new(&mut self, _root: &Path, _name: &str) -> Option<Result<String, String>> {
        None
    }
    /// The default name the keyboard opens with for a new file.
    fn new_name(&mut self, _root: &Path) -> String {
        String::new()
    }
}

/// Break the Targets' Saved layouts: this stage's layout files.
#[derive(Default)]
struct BttLayouts {
    rows: Vec<(String, Layout)>,
}

impl SavedList for BttLayouts {
    fn title(&self) -> &'static str {
        "Saved layouts"
    }

    fn list_title(&self) -> String {
        format!("{} · {}", self.title(), stage_label(stage()))
    }

    fn naming(&self) -> &'static str {
        "Name layout"
    }

    fn saved_rows(&mut self, root: &Path) -> (Vec<(String, String, String)>, usize, usize) {
        let (all, skipped) = btt::all(root);
        let hot;
        (self.rows, hot) = btt::listed(&all, stage());
        (self.rows.iter().map(|(id, l)| (id.clone(), l.name.clone(), targets_label(l.targets.len()))).collect(), hot, skipped)
    }

    fn footer(&self, cursor: usize, skipped: usize) -> String {
        btt::list_footer(&self.rows, cursor, skipped)
    }

    fn saved_rename(&mut self, root: &Path, id: &str, name: &str) -> Result<String, String> {
        btt::use_file(root, id, Some(name)).map(|()| format!("Renamed to {}", name.trim())).map_err(|e| e.to_string())
    }

    fn saved_copy(&mut self, root: &Path, id: &str) -> Result<String, String> {
        btt::duplicate(root, id, &stage_label(stage())).map(|_| "Duplicated: the copy is at the top".to_owned()).map_err(|e| e.to_string())
    }

    fn saved_remove(&mut self, root: &Path, id: &str) -> Result<String, String> {
        let name = self.rows.iter().find(|r| r.0 == id).map(|r| r.1.name.clone()).unwrap_or_default();
        btt::library(root).delete(id).map(|()| format!("Deleted {name}")).map_err(|e| e.to_string())
    }

    /// Load stages the layout for Start, or puts a running course back at Ready.
    fn saved_use(&mut self, root: &Path, id: &str, menu: Option<&mut Menu>) -> Result<String, String> {
        let layout = self.rows.iter().find(|r| r.0 == id).map(|r| r.1.clone()).ok_or_else(|| format!("No layout {id}"))?;
        let hot = btt::five(&[(id.to_owned(), layout.clone())], 1)[0].ok_or_else(|| format!("No layout {id}"))?;
        CMDS.post(hot);
        let running = menu.is_some_and(|m| m.btt_running());
        let next = if running { "back to Ready with it" } else { "Start plays it" };
        btt::use_file(root, id, None).map(|()| format!("Loaded {}: {next}", layout.name)).map_err(|e| e.to_string())
    }
}

/// The pane up (`None`: the screen is closed), the files in the list's order, how many are hot, the list's cursor, the files
/// skipped, the rename the keyboard is open for (its token and the file's id; an empty id is a new file), and the kind.
#[derive(Default)]
pub struct Ui {
    pane: Option<Pane>,
    rows: Vec<(String, String, String)>,
    hot: usize,
    cursor: usize,
    skipped: usize,
    naming: Option<(u64, String)>,
    serial: u64,
    kind: Option<Box<dyn SavedList>>,
}

impl Ui {
    /// A result of the screen: one notice, under the open list's title.
    pub fn note(&self, text: &str) {
        let title = self.kind.as_ref().map_or("Saved layouts", |k| k.title());
        notices::upsert(NOTICE_ID, title, text, 180, DEFAULT_COLOR);
    }

    /// The row was entered: its list opens (true), or the note says why it did not. A "Save this …" row opens the keyboard.
    pub fn open(&mut self, editor: Editor) -> bool {
        self.cursor = 0;
        let mut kind: Box<dyn SavedList> = match editor {
            Editor::Layouts => Box::new(BttLayouts::default()),
            _ => Box::new(super::parry_saved::ParryPractices::default()),
        };
        if editor == Editor::ParrySave {
            let name = kind.new_name(root());
            self.serial += 1;
            self.naming = Some((TOKEN_BASE | self.serial, String::new()));
            named_macros::offer_external_name_titled(TOKEN_BASE | self.serial, kind.naming(), &name, "Saving this practice");
            self.kind = Some(kind);
            return false;
        }
        self.kind = Some(kind);
        let shown = self.show(Pane::List);
        if let Err(why) = &shown {
            self.note(why);
        }
        shown.is_ok()
    }

    fn kind(&mut self) -> Result<&mut Box<dyn SavedList>, String> {
        self.kind.as_mut().ok_or_else(|| "No list open".to_owned())
    }

    /// Opens `pane` on the library panel; the list is read from the card again. The list is drawn as a list (`paint`), the
    /// panel under it keeps the layer open.
    fn show(&mut self, pane: Pane) -> Result<(), String> {
        let (title, rows) = match &pane {
            Pane::List => {
                let (rows, hot, skipped) = self.kind()?.saved_rows(root());
                (self.rows, self.hot, self.skipped) = (rows, hot, skipped);
                self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
                let title = self.kind()?.title().to_owned();
                (title, self.rows.iter().map(|(id, name, _)| LibraryRow { id: id.clone(), label: name.clone() }).collect())
            }
            Pane::Own(id) | Pane::Delete(id) => (self.row(id)?.1.clone(), pane.rows(&LAYOUT)),
        };
        named_macros::open_external_library(rows, 0, title, None);
        if !external::panel_up() {
            return Err("Wait for the practice fighters to finish loading".into()); // restriction: combo-save-wait-load
        }
        self.pane = Some(pane);
        Ok(())
    }

    fn row(&self, id: &str) -> Result<&(String, String, String), String> {
        self.rows.iter().find(|r| r.0 == id).ok_or_else(|| format!("No file {id}"))
    }

    /// The list takes Up, Down, A and B itself (one row a step, as Recording's lists); a file's own screen and its question
    /// are the panel's, and B there steps back one pane.
    pub fn press(&mut self, button: Button) {
        match (&self.pane, button) {
            (Some(Pane::List), Button::Up) => self.cursor = crate::model::menu::stepped(self.cursor, self.rows.len(), false),
            (Some(Pane::List), Button::Down) => self.cursor = crate::model::menu::stepped(self.cursor, self.rows.len(), true),
            (Some(Pane::List), Button::A) => {
                if let Some(Err(why)) = self.rows.get(self.cursor).map(|r| r.0.clone()).map(|id| self.show(Pane::Own(id))) {
                    self.note(&why);
                }
            }
            (Some(Pane::List), Button::B) | (None, _) => external::press_panel(button),
            (Some(Pane::List), _) => {}
            (Some(pane), Button::B) => {
                if let Some(Err(why)) = pane.back().map(|back| self.show(back)) {
                    self.note(&why);
                }
            }
            _ => external::press_panel(button),
        }
    }

    /// Every worker poll: a confirmed name, and A on a file's own screen or its question. Returns a note to show.
    pub fn tick(&mut self, menu: Option<&mut Menu>) -> Option<String> {
        if let Some((token, id)) = self.naming.clone() {
            if let Some(name) = named_macros::take_confirmed_name_for(token) {
                self.naming = None;
                let kind = self.kind.as_mut()?;
                let done = if id.is_empty() { kind.saved_new(root(), &name)? } else { kind.saved_rename(root(), &id, &name) };
                return Some(self.changed(done));
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
            Step::Choose(id, _) => {
                let done = self.kind.as_mut()?.saved_use(root(), &id, menu);
                Some(self.changed(done))
            }
            Step::Rename(id) => {
                let (name, detail) = self.row(&id).ok().map(|r| (r.1.clone(), r.2.clone()))?;
                self.serial += 1;
                self.naming = Some((TOKEN_BASE | self.serial, id));
                let title = self.kind.as_ref()?.naming();
                named_macros::offer_external_name_titled(TOKEN_BASE | self.serial, title, &name, &format!("Renaming · {detail}"));
                None
            }
            Step::Duplicate(id) => {
                self.cursor = 0;
                let done = self.kind.as_mut()?.saved_copy(root(), &id);
                self.back_to_list(done)
            }
            Step::Remove(id) => {
                // Both SavedList impls delete through their Store (tests/lock_order.rs RECEIVER_MODULES).
                let saved_kind = self.kind.as_mut()?;
                let done = saved_kind.saved_remove(root(), &id);
                self.back_to_list(done)
            }
        }
    }

    /// A file changed: the chords' five are listed again; the note is the result or why it failed.
    fn changed(&self, result: Result<String, String>) -> String {
        reread();
        result.unwrap_or_else(|why| why)
    }

    fn back_to_list(&mut self, result: Result<String, String>) -> Option<String> {
        let text = self.changed(result);
        Some(self.show(Pane::List).err().map_or(text.clone(), |why| format!("{text}; {why}")))
    }

    /// The list, drawn like the Recording lists, when it is the pane up; false for the other panes (the panel's own frame).
    pub fn paint(&self, menu: &Menu, frame: &mut Frame) -> bool {
        let Some(kind) = self.kind.as_ref() else { return false };
        if self.pane != Some(Pane::List) || named_macros::suspended() {
            return false;
        }
        let rows: Vec<(String, String)> = self.rows.iter().map(|(_, name, detail)| (name.clone(), detail.clone())).collect();
        let footer = kind.footer(self.cursor, self.skipped);
        hot_list_frame(menu, &kind.list_title(), &footer, &rows, self.cursor, self.hot, frame);
        true
    }

    /// A file's own screen or its question: the panel's frame, with the file and what the row under the cursor does.
    pub fn view(&self) -> Option<View> {
        let mut view = external::panel_view_now()?;
        if let Some(pane @ (Pane::Own(id) | Pane::Delete(id))) = &self.pane {
            let label = self.row(id).map(|r| format!("{} · {}", r.1, r.2)).unwrap_or_default();
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
