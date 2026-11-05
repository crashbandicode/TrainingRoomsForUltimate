//! The Recording page's Combos group on the menu worker (SPEC-input-07 §4–§6): the draft name, *Save last combo*, the
//! saved combos (A target, X rename, Y delete) and the target. Files go through the generic store; names through the
//! named-macro panel's caller-owned naming (the setup library's path), so the keyboard has one owner.
//!
//! Choosing a combo recorded on another fighter asks the named macros' Switch/Keep question about Player 1 (§4a): the
//! shared prompt with the `Combo` origin, the plan from `editors::combo_fighter`, and a Switch asked of the roster
//! loader's Player role when the menu closes (`after_close`).

use core::sync::atomic::{AtomicU64, Ordering};

use super::external;
use crate::editors::combo_fighter::{self as fighter, event, Plan};
use crate::engine::compat::menu::OnOff;
use crate::engine::macro_ui::macro_panel::FighterChoice;
use crate::engine::swap::trio::TrioFighter;
use crate::runtime::roster::character_roster as roster;
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

/// The fighter question's revision: a later choice, or the list's panel going away, retires an unanswered question
/// (`named_macros::fighter_prompt_stale`).
static PROMPT_REVISION: AtomicU64 = AtomicU64::new(1);

pub fn prompt_revision() -> u64 {
    PROMPT_REVISION.load(Ordering::Acquire)
}

/// A chosen combo waiting for the answer to its fighter question: nothing is written until the answer.
struct Asking {
    id: String,
    record: ComboRecord,
    fighter: TrioFighter,
    p1: Option<u32>,
    p1_label: String,
}

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
    asking: Option<Asking>,
    /// Switch was answered: P1's switch is asked for when the menu closes.
    switch_at_close: Option<TrioFighter>,
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
        let opened = match editor {
            Editor::ComboName => {
                if !name_keyboard_opens() {
                    // restriction: combo-name-online-default
                    Opened::Done("The next combo gets a Combo N name".into())
                } else {
                    let initial = self.draft.clone().unwrap_or_default();
                    self.offer(Naming::Draft, &initial, "The name of the next saved combo");
                    Opened::Panel
                }
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

    /// Training and a local-wireless waiting room may name a combo. An online waiting room may not:
    /// neither keyboard transport is shown safe there (`model::scene::combo_name_keyboard`).
    fn name_keyboard_opens() -> bool {
        let mode = crate::runtime::lobby::game_mode::current().map(|mode| mode.mode).unwrap_or(0);
        crate::model::scene::combo_name_keyboard(crate::runtime::ui::facts::fact_waiting_scene(), mode)
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
            // restriction: combo-save-wait-load
            return Err("Wait for the practice fighters to finish loading".into());
        }
        Ok(())
    }

    /// Every worker poll: the fighter question's answer, names the keyboard confirmed, and the list's A, X and Y.
    /// Returns a note to show.
    pub fn tick(&mut self) -> Option<String> {
        self.load_target();
        if let Some(answer) = named_macros::take_combo_fighter_choice() {
            // An answer with no question waiting belongs to a panel the menu already closed: nothing to apply.
            if let Some(asking) = self.asking.take() {
                return self.answered(asking, answer);
            }
        }
        if self.asking.is_some() && !external::panel_up() {
            // Retired unanswered (the scene or the selection moved on): nothing changed.
            self.asking = None;
            self.active = None;
        }
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
            return match self.choose(&id) {
                // The fighter question is up: the group keeps the panel until it is answered.
                Ok(None) => None,
                Ok(Some(note)) => {
                    self.active = None;
                    Some(note)
                }
                Err(why) => {
                    self.active = None;
                    Some(why)
                }
            };
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

    /// A in the list: the combo becomes the target, unless it was recorded on another fighter and the question opens
    /// first (`Ok(None)`; the answer arrives in `tick`).
    fn choose(&mut self, id: &str) -> Result<Option<String>, String> {
        let record = library().load(id).map_err(|e| e.to_string())?;
        let p1 = roster::player_kind();
        let p1_label = p1.map(|kind| named_macros::fighter_label(kind as i32));
        let prompt_enabled = crate::runtime::compat::sync::read(&crate::runtime::compat::menu::MENU).fighter_prompt == OnOff::ON;
        let recorded_on = record.recorded_on.clone();
        let plan = fighter::plan(recorded_on.as_deref(), p1.zip(p1_label.as_deref()), prompt_enabled);
        let recorded = recorded_on.as_deref().and_then(fighter::fighter_named).map(TrioFighter::kind);
        PROMPT_REVISION.fetch_add(1, Ordering::AcqRel);
        self.switch_at_close = None;
        let p1_label = p1_label.unwrap_or_default();
        let tail = match plan {
            Plan::Ask(wanted) => {
                if named_macros::open_combo_fighter_prompt(prompt_revision(), wanted.name().to_owned(), p1_label.clone()) {
                    live::fighter_event(event::ASKED, recorded, p1, 0);
                    self.asking = Some(Asking { id: id.to_owned(), record, fighter: wanted, p1, p1_label });
                    return Ok(None);
                }
                // The question could not open: the safe default, as Keep.
                live::fighter_event(event::KEPT, recorded, p1, 0);
                Some(fighter::answer_note(FighterChoice::Keep, wanted, &p1_label))
            }
            Plan::Keep(kept) => {
                live::fighter_event(event::KEPT, recorded, p1, kept as u32);
                fighter::kept_note(kept, recorded_on.as_deref().unwrap_or_default(), &p1_label)
            }
            Plan::Proceed => {
                live::fighter_event(event::PROCEED, recorded, p1, 0);
                None
            }
        };
        self.target(id, record, tail).map(Some)
    }

    /// The combo becomes the target: `selected.txt`, the helper, and the note (with `tail`).
    fn target(&mut self, id: &str, record: ComboRecord, tail: Option<String>) -> Result<String, String> {
        library().select(Some(id)).map_err(|e| e.to_string())?;
        let mut note = match &record.recorded_on {
            Some(fighter) => format!("Target: {} ({} links, recorded on {fighter})", record.name, record.combo.len()),
            None => format!("Target: {} ({} links)", record.name, record.combo.len()),
        };
        if let Some(tail) = tail {
            note.push_str("; ");
            note.push_str(&tail);
        }
        self.apply_target(Some((id.to_owned(), record)));
        Ok(note)
    }

    /// The answer to the fighter question. Cancel changes nothing and opens the list again; Keep and Switch make the
    /// combo the target, and Switch asks for P1's fighter when the menu closes.
    fn answered(&mut self, asking: Asking, answer: Option<FighterChoice>) -> Option<String> {
        let recorded = Some(asking.fighter.kind());
        let Some(choice) = answer else {
            live::fighter_event(event::CANCELLED, recorded, asking.p1, 0);
            if let Err(why) = self.show_list() {
                self.active = None;
                return Some(why);
            }
            return None;
        };
        let code = if choice == FighterChoice::Switch { event::SWITCH } else { event::KEEP };
        live::fighter_event(code, recorded, asking.p1, 0);
        self.active = None;
        let tail = fighter::answer_note(choice, asking.fighter, &asking.p1_label);
        match self.target(&asking.id, asking.record, Some(tail)) {
            Ok(note) => {
                if choice == FighterChoice::Switch {
                    self.switch_at_close = Some(asking.fighter);
                }
                Some(note)
            }
            Err(why) => Some(why),
        }
    }

    /// The menu closed (`worker::close_session`): P1's switch, when Switch was answered, goes to the roster loader now,
    /// so the loader's wait for the menu to release input starts here (the Practice Stage request's moment). Refusals
    /// carry the loader's own notice; this one names the fighter.
    pub fn after_close(&mut self) {
        let Some(wanted) = self.switch_at_close.take() else { return };
        let queued = roster::request_player_kind(wanted.kind());
        live::fighter_event(event::REQUESTED, Some(wanted.kind()), roster::player_kind(), u32::from(queued));
        if !queued {
            let note = format!("P1 was not switched to {}", wanted.name());
            notices::upsert(0x2A, "Combos", &note, 180, DEFAULT_COLOR);
        }
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
        if matches!(self.active, Some(Editor::ComboLibrary | Editor::ComboTarget)) && self.naming.is_none() && self.asking.is_none() {
            view.footer = "> marks the target.\nA Target · X Rename · Y Delete · B Return".into();
        }
        Some(view)
    }

    /// The panel closed (B, or the menu lost its layer): a pending name stays with the panel's own continuation.
    pub fn leave_panel(&mut self) {
        if !named_macros::suspended() {
            self.active = None;
            if self.asking.take().is_some() {
                PROMPT_REVISION.fetch_add(1, Ordering::AcqRel);
            }
        }
    }
}
