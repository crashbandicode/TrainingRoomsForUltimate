//! The Saved layouts screen of Break the Targets (`artifacts/rc14-eggsercise/SPEC.md` §V2.5 "Saved layouts") on the menu worker: this
//! stage's five slots as a list panel, as the saved combos' list is (`ui::combos`). A loads a slot and Y deletes it; both go to the
//! game thread, which owns the live slots, through `btt::CMDS`. The slots shown are the file service's cache (`btt::CACHE`: the
//! stage and slots as last read or written).

use super::external;
use crate::engine::macro_ui::macro_panel::{LibraryRow, View};
use crate::model::btt::{layouts_footer, slot_label, SLOTS};
use crate::model::menu::table::Editor;
use crate::model::menu::Menu;
use crate::model::notify::DEFAULT_COLOR;
use crate::runtime::features::btt::{CACHE, CMDS};
use crate::runtime::features::notices;
use crate::runtime::macros::named_macros;

/// This screen's standing notice (free beside the Combos' 0x2A and 0x2B, Recording's 0x2C and Eggs-ercise's 0x2E).
const NOTICE_ID: u8 = 0x2D;

/// A result of the screen: one notice.
pub fn note(text: &str) {
    notices::upsert(NOTICE_ID, "Saved layouts", text, 180, DEFAULT_COLOR);
}

pub fn is_layouts(editor: Editor) -> bool {
    editor == Editor::Layouts
}

/// Whether the list is up.
#[derive(Default)]
pub struct Ui {
    up: bool,
}

impl Ui {
    /// The row was entered: the slot list opens (true), or the note says why it did not.
    pub fn open(&mut self) -> bool {
        let slots = CACHE.read().1;
        let rows = (0..SLOTS).map(|i| LibraryRow { id: i.to_string(), label: slot_label(i, slots[i].as_ref()) }).collect();
        named_macros::open_external_library(rows, 0, "Saved layouts", Some("delete".to_owned()));
        self.up = external::panel_up();
        if !self.up {
            note("Wait for the practice fighters to finish loading"); // restriction: combo-save-wait-load
        }
        self.up
    }

    /// Every worker poll: A (Load) and Y (Delete) on the list. Returns a note to show. A load turns Break the Targets On, so
    /// closing the menu opens it at Ready.
    pub fn tick(&mut self, menu: Option<&mut Menu>) -> Option<String> {
        if !self.up {
            return None;
        }
        // X (rename) means nothing here; it closes the panel like A and Y do.
        let _ = named_macros::take_list_rename();
        let (id, delete) = match (named_macros::take_list_choice(), named_macros::take_list_action()) {
            (Some(id), _) => (id, false),
            (_, Some(id)) => (id, true),
            _ => return None,
        };
        self.up = false;
        let slot = id.parse::<usize>().ok()?;
        let n = slot + 1;
        if CACHE.read().1.get(slot)?.is_none() {
            return Some(format!("Slot {n} is empty"));
        }
        CMDS.post((delete, slot));
        if delete {
            return Some(format!("Slot {n} deleted"));
        }
        if let Some(menu) = menu {
            menu.btt_on();
        }
        Some(format!("Slot {n} loaded: close the menu to start"))
    }

    /// The panel's frame: the list's own, with this screen's footer.
    pub fn view(&self) -> Option<View> {
        let mut view = external::panel_view_now()?;
        view.footer = layouts_footer(&view.footer);
        Some(view)
    }

    /// The panel left the screen (B, or the menu closed).
    pub fn leave_panel(&mut self) {
        if !named_macros::suspended() {
            self.up = false;
        }
    }
}
