//! The Recording page's Combos group on the menu worker (SPEC-input-07 §4–§6): the draft name, *Save last combo*, the
//! saved combos (A target, X rename, Y delete) and the target. Files go through the generic store; names through the
//! named-macro panel's caller-owned naming (the setup library's path), so the keyboard has one owner.

use super::external;
use crate::engine::macro_ui::macro_panel::{LibraryRow, View, COMBO_KEYBOARD_TITLE};
use crate::model::combo_record::{clean_name, default_name, ComboLibrary, ComboRecord, DIR};
use crate::model::notify::DEFAULT_COLOR;
use crate::model::store::SkippedFile;
use crate::runtime::features::notices;
use crate::model::menu::{input::Button, table::Editor};
use crate::runtime::features::combo as live;
use crate::runtime::macros::named_macros;

/// Name tokens of this library: the high bit keeps them apart from the setup library's revisions.
const TOKEN_BASE: u64 = 1 << 63;

pub fn is_combo(editor: Editor) -> bool {
    matches!(editor, Editor::ComboName | Editor::ComboSave | Editor::ComboLibrary | Editor::ComboTarget)
}

fn library() -> ComboLibrary {
    ComboLibrary::new(format!("{}/{DIR}", crate::model::storage::ROOT))
}

/// The standing notice that names a combo file the plugin could not use (the Combos notes use `0x2A`).
const FILE_NOTICE: u8 = 0x2B;

/// One notice naming the first unusable file and why, and how many more there are: never a silent drop.
fn note_skipped(skipped: &[SkippedFile]) {
    if let Some(text) = crate::model::combo_record::skipped_note(skipped) {
        notices::upsert(FILE_NOTICE, "Saved combos", &text, 300, DEFAULT_COLOR);
    }
}

/// What a name the keyboard returns is for.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Naming {
    Draft,
    Rename(String),
}

/// What opening a row did.
pub enum Opened {
    /// A panel is up (the list, the name page).
    Panel,
    /// The action ran; the note says what happened and the row closes.
    Done(String),
}

#[derive(Default)]
pub struct Ui {
    active: Option<Editor>,
    naming: Option<(u64, Naming)>,
    serial: u64,
    draft: Option<String>,
    /// The target's id and name.
    target: Option<(String, String)>,
    listed: usize,
    loaded: bool,
}

impl Ui {
    /// The target `selected.txt` names, read once on the worker.
    fn load_target(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let library = library();
        if let Some(id) = library.selected() {
            match library.load(&id) {
                Ok(record) => self.apply_target(Some((id, record))),
                Err(why) => note_skipped(&[SkippedFile { file: format!("{id}.json"), reason: why.to_string() }]),
            }
        }
    }

    fn apply_target(&mut self, target: Option<(String, ComboRecord)>) {
        live::set_target(target.as_ref().map(|(_, r)| (r.combo, r.name.as_str())));
        self.target = target.map(|(id, r)| (id, r.name));
    }

    pub fn active_editor(&self) -> Option<Editor> {
        self.active
    }

    /// A row of the group was entered.
    pub fn open(&mut self, editor: Editor) -> Result<Opened, String> {
        self.load_target();
        if crate::runtime::ui::facts::fact_waiting_scene() {
            return Err("Combos work in Training".into()); // restriction: combos-training-only
        }
        let opened = match editor {
            Editor::ComboName => {
                let initial = self.draft.clone().unwrap_or_default();
                self.offer(Naming::Draft, &initial, "The name of the next saved combo");
                Opened::Panel
            }
            Editor::ComboSave => Opened::Done(self.save()?),
            Editor::ComboLibrary | Editor::ComboTarget => {
                self.show_list()?;
                Opened::Panel
            }
            _ => return Err("Not a combo row".into()),
        };
        self.active = matches!(opened, Opened::Panel).then_some(editor);
        Ok(opened)
    }

    fn offer(&mut self, naming: Naming, initial: &str, summary: &str) {
        self.serial += 1;
        let token = TOKEN_BASE | self.serial;
        self.naming = Some((token, naming));
        named_macros::offer_external_name_titled(token, COMBO_KEYBOARD_TITLE, initial, summary);
    }

    fn save(&mut self) -> Result<String, String> {
        let combo = live::last_combo().ok_or("No combo yet: land a hit and a follow-up first")?;
        let library = library();
        let name = match self.draft.take() {
            Some(name) => name,
            None => default_name(&library.list().map_err(|e| e.to_string())?.entries),
        };
        let recorded_on = live::p1_kind().map(named_macros::fighter_label);
        let record = ComboRecord::new(name, combo, recorded_on);
        let id = library.save_new(&record).map_err(|e| e.to_string())?;
        live::set_draft("");
        live::saved(&combo);
        let note = format!("Saved {} ({} links), now the target", record.name, combo.len());
        if let Err(why) = library.select(Some(&id)) {
            self.apply_target(Some((id, record)));
            return Ok(format!("{note}; selected.txt: {why}"));
        }
        self.apply_target(Some((id, record)));
        Ok(note)
    }

    fn show_list(&mut self) -> Result<(), String> {
        let listing = library().list().map_err(|e| e.to_string())?;
        self.listed = listing.entries.len();
        let shared = listing.entries.iter().filter(|e| e.shared()).count();
        live::listed(self.listed, listing.skipped.len(), shared);
        note_skipped(&listing.skipped);
        if self.listed == 0 {
            return Err("No combos saved yet: Save last combo first".into());
        }
        let target = self.target.as_ref().map(|(id, _)| id.as_str());
        let rows = listing
            .entries
            .into_iter()
            .map(|e| LibraryRow {
                label: format!("{}{}", if Some(e.id.as_str()) == target { "> " } else { "" }, e.label()),
                id: e.id,
            })
            .collect();
        named_macros::open_external_library(rows, listing.skipped.len(), "Saved combos", Some("delete".into()));
        if !external::panel_up() {
            return Err("Wait for the practice fighters to finish loading".into()); // restriction: combo-save-wait-load
        }
        Ok(())
    }

    /// Every worker poll: names the keyboard confirmed, and the list's A, X and Y. Returns a note to show.
    pub fn tick(&mut self) -> Option<String> {
        self.load_target();
        if let Some((token, _)) = self.naming {
            if let Some(confirmed) = named_macros::take_confirmed_name() {
                if confirmed.context.token == token {
                    let (_, naming) = self.naming.take()?;
                    self.active = None;
                    return Some(self.named(naming, &confirmed.name));
                }
            }
            if !named_macros::external_name_open(token) {
                // Cancelled (B) or discarded: the previous name stays.
                self.naming = None;
                if self.active == Some(Editor::ComboName) {
                    self.active = None;
                }
            }
        }
        if !matches!(self.active, Some(Editor::ComboLibrary | Editor::ComboTarget)) {
            return None;
        }
        if let Some(id) = named_macros::take_list_choice() {
            self.active = None;
            return Some(self.choose(&id).unwrap_or_else(|why| why));
        }
        if let Some(id) = named_macros::take_list_action() {
            self.active = None;
            return Some(self.delete(&id).unwrap_or_else(|why| why));
        }
        if let Some(id) = named_macros::take_list_rename() {
            return match library().load(&id) {
                Ok(record) => {
                    self.offer(Naming::Rename(id), &record.name, "Rename the saved combo");
                    None
                }
                Err(why) => {
                    self.active = None;
                    Some(why.to_string())
                }
            };
        }
        None
    }

    fn named(&mut self, naming: Naming, typed: &str) -> String {
        match naming {
            Naming::Draft => {
                self.draft = clean_name(typed).or(self.draft.take());
                live::set_draft(self.draft.as_deref().unwrap_or(""));
                match &self.draft {
                    Some(name) => format!("The next combo is saved as {name}"),
                    None => "The next combo gets a Combo N name".into(),
                }
            }
            Naming::Rename(id) => {
                let Some(name) = clean_name(typed) else { return "Name unchanged".into() };
                let library = library();
                let result = library.load(&id).and_then(|mut record| {
                    record.name = name.clone();
                    library.overwrite(&id, &record).map(|()| record)
                });
                match result {
                    Ok(record) => {
                        if self.target.as_ref().is_some_and(|(target, _)| *target == id) {
                            self.apply_target(Some((id, record)));
                        }
                        format!("Renamed to {name}")
                    }
                    Err(why) => why.to_string(),
                }
            }
        }
    }

    fn choose(&mut self, id: &str) -> Result<String, String> {
        let library = library();
        let record = library.load(id).map_err(|e| e.to_string())?;
        library.select(Some(id)).map_err(|e| e.to_string())?;
        let note = match &record.recorded_on {
            Some(fighter) => format!("Target: {} ({} links, recorded on {fighter})", record.name, record.combo.len()),
            None => format!("Target: {} ({} links)", record.name, record.combo.len()),
        };
        self.apply_target(Some((id.to_owned(), record)));
        Ok(note)
    }

    fn delete(&mut self, id: &str) -> Result<String, String> {
        let library = library();
        let name = library.load(id).map(|r| r.name).unwrap_or_else(|_| id.to_owned());
        library.delete(id).map_err(|e| e.to_string())?;
        if self.target.as_ref().is_some_and(|(target, _)| target == id) {
            let _ = library.select(None);
            self.apply_target(None);
        }
        Ok(format!("Deleted {name}"))
    }

    pub fn press(&mut self, button: Button) {
        if matches!(self.active, Some(Editor::ComboLibrary | Editor::ComboTarget))
            && self.listed == 0
            && matches!(button, Button::A | Button::X | Button::Y)
        {
            return;
        }
        external::press_panel(button);
    }

    pub fn panel_up(&self) -> bool {
        !named_macros::suspended() && (self.active.is_some() || self.naming.is_some()) && external::panel_up()
    }

    pub fn view(&self) -> Option<View> {
        let mut view = external::panel_view_now()?;
        if matches!(self.active, Some(Editor::ComboLibrary | Editor::ComboTarget)) && self.naming.is_none() {
            view.footer = "> marks the target.\nA Target · X Rename · Y Delete · B Return".into();
        }
        Some(view)
    }

    /// The panel closed (B, or the menu lost its layer): a pending name stays with the panel's own continuation.
    pub fn leave_panel(&mut self) {
        if !named_macros::suspended() {
            self.active = None;
        }
    }
}
