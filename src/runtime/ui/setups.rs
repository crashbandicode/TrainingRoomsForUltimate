//! Setup screens on the menu worker. The states facade owns native work.
use super::external;
use crate::editors::setups::row_label;
use crate::editors::setups::{chosen, list_footer, name_summary, Pane, Provider, Reply, Res, Setups, Step};
use crate::engine::macro_ui::macro_panel::{LibraryRow, View};
use crate::model::menu::{input::Button, table::Editor, Menu};
use crate::model::state::setup::{SetupLibrary, SetupRecord};
use crate::model::store::Record;
use crate::runtime::features::states::setups as native;
use crate::runtime::macros::named_macros;

struct Native;
impl Provider for Native {
    type Target = native::Target;
    type Ticket = native::Ticket;
    fn snapshot(&mut self, target: Self::Target) -> Res<Self::Ticket> {
        native::request_snapshot(target)
    }
    fn validate(&mut self, target: Self::Target, record: &SetupRecord, restore: bool) -> Res<()> {
        native::validate_selection(target, record, restore)
    }
    fn bind(&mut self, target: Self::Target, record: &SetupRecord, restore: bool) -> Res<Self::Ticket> {
        native::request_bind(target, record, restore)
    }
    fn poll(&mut self, ticket: Self::Ticket) -> Option<Res<Reply>> {
        native::poll(ticket).map(|reply| {
            reply.map(|reply| match reply {
                native::Outcome::Snapshot(record) => Reply::Snapshot(Box::new(record)),
                native::Outcome::Bound => Reply::Bound,
            })
        })
    }
}

#[derive(Default)]
pub struct Ui {
    model: Setups<native::Target, native::Ticket>,
    active: Option<Editor>,
    target: Option<native::Target>,
    name_token: Option<u64>,
    library_count: usize,
    pane: Pane,
    /// The open setup's row text, for its own screen's footer.
    label: String,
}

fn library() -> SetupLibrary {
    SetupLibrary::new(format!("{}/setups", crate::model::storage::ROOT))
}

impl Ui {
    pub fn watching(&self) -> bool {
        self.model.watching() || self.name_token.is_some()
    }

    pub fn open(&mut self, editor: Editor, menu: &Menu) -> Result<(), String> {
        // A same-lobby parked naming session belongs to its original slot.
        if editor == Editor::SetupSave
            && self
                .name_token
                .is_some_and(named_macros::external_name_open)
        {
            self.active = Some(editor);
            return Ok(());
        }
        let settings = &menu.settings().menu;
        let slot = settings
            .get("save_state_slot")
            .and_then(|v| v.iter().position(|n| *n != 0))
            .ok_or("Choose a Save State Slot first")?;
        let enabled = settings
            .get("save_state_enable")
            .and_then(|v| v.first())
            .is_some_and(|n| *n != 0);
        let target = native::open(slot, enabled)?;
        match editor {
            Editor::SetupSave => self.model.begin_save(&mut Native, target)?,
            Editor::SetupLibrary => self.show(Pane::List)?,
            _ => return Err("Not a setup screen".into()),
        }
        self.target = Some(target);
        self.active = Some(editor);
        Ok(())
    }

    /// Opens `pane` as the layer's panel: the list of setups, or one setup's own screen or delete question.
    fn show(&mut self, pane: Pane) -> Result<(), String> {
        let (title, rows, skipped): (_, Vec<LibraryRow>, _) = match &pane {
            Pane::List => {
                let listing = library().list().map_err(|e| e.to_string())?;
                self.library_count = listing.entries.len();
                let entries = listing.entries.into_iter();
                let rows = entries.map(|entry| LibraryRow {
                    label: row_label(&entry, named_macros::fighter_label),
                    id: entry.id,
                });
                ("Setup library".to_owned(), rows.collect(), listing.skipped.len())
            }
            Pane::Own(id) | Pane::Delete(id) => {
                let record = library().load(id).map_err(|e| e.to_string())?;
                self.label = row_label(&record.summary(id), named_macros::fighter_label);
                (record.name, pane.rows(), 0)
            }
        };
        let alternate = (pane == Pane::List).then(|| "bind only".to_owned());
        named_macros::open_external_library(rows, skipped, title, alternate);
        if !external::panel_up() {
            return Err("Wait for the practice fighters to finish loading".into());
        }
        self.pane = pane;
        Ok(())
    }

    fn offer_name(&mut self) {
        let Some(record) = self.model.draft() else {
            return;
        };
        let token = self.model.name_revision();
        let summary = name_summary(
            self.model.renaming(),
            &named_macros::fighter_label(record.player.fighter_kind),
            &named_macros::fighter_label(record.cpu.fighter_kind),
        );
        self.name_token = Some(token);
        named_macros::offer_external_name(token, &record.name, &summary);
    }

    /// X on a row: the setup's name goes through the keyboard, as a new setup's does; the panel X closed is gone, so the naming
    /// page is the open screen.
    fn begin_rename(&mut self, id: String) -> Option<String> {
        let record = match library().load(&id) {
            Ok(record) => record,
            Err(why) => return Some(why.to_string()),
        };
        self.model.begin_rename(id, record);
        self.active = Some(Editor::SetupSave);
        self.offer_name();
        None
    }

    pub fn active_editor(&self) -> Option<Editor> {
        self.active
    }

    /// A panel return does not discard a selected binding. A suspended naming
    /// draft remains owned by the existing external-name continuation.
    pub fn leave_panel(&mut self) {
        if !named_macros::suspended() {
            self.active = None;
        }
    }

    /// Called every worker poll, even when the menu has closed or reset.
    // Finish Setup snapshot/JSON scratch before sync_panels enters capture I/O.
    #[inline(never)]
    pub fn tick(&mut self) -> Option<String> {
        if self.target.is_some_and(|target| {
            native::open(target.slot, target.enabled)
                .is_ok_and(|current| current.session != target.session)
        }) {
            self.retire_session();
        }
        if let Some(reply) = self.model.poll(&mut Native) {
            match reply {
                Ok(true) => self.offer_name(),
                Ok(false) => {}
                Err(why) => {
                    self.active = None;
                    return Some(why.into());
                }
            }
        }
        if let Some(token) = self.name_token {
            if let Some(confirmed) = named_macros::take_confirmed_name() {
                if confirmed.context.token == token {
                    let verb = if self.model.renaming() { "Renamed to" } else { "Saved setup" };
                    match self.model.save_name(token, &confirmed.name, &library()) {
                        Ok(_) => {
                            self.name_token = None;
                            self.active = None;
                            return Some(format!("{verb} {}", confirmed.name.trim()));
                        }
                        Err(why) => {
                            self.offer_name();
                            return Some(why);
                        }
                    }
                }
            }
            if !named_macros::external_name_open(token) {
                self.model.cancel_name();
                self.name_token = None;
                self.active = None;
            }
        }
        if self.active == Some(Editor::SetupLibrary) {
            if let Some(id) = named_macros::take_list_rename() {
                return self.begin_rename(id);
            }
            // A opens the row (a setup, or one of its screen's rows); Y on a list row makes Load use that setup.
            return match (named_macros::take_list_choice(), named_macros::take_list_action()) {
                (Some(row), _) => self.run(self.pane.leads_to(&row)),
                (None, Some(id)) => self.run(Step::Choose(id, false)),
                (None, None) => None,
            };
        }
        None
    }

    /// Does what a press on a panel's row led to; the text is the notice.
    fn run(&mut self, step: Step) -> Option<String> {
        match step {
            Step::Open(pane) => self.show(pane).err(),
            Step::Rename(id) => self.begin_rename(id),
            Step::Remove(id) => {
                self.active = None;
                Some(library().delete(&id).map_or_else(|e| e.to_string(), |()| "Setup deleted".into()))
            }
            Step::Choose(id, restore) => {
                let result = (|| {
                    let target = self.target.ok_or("This setup list has closed")?;
                    let record = library().load(&id).map_err(|e| e.to_string())?;
                    self.model.choose(&mut Native, target, record, restore).map_err(str::to_owned)
                })();
                if result.is_ok() {
                    self.active = None;
                    return Some(chosen(restore).into());
                }
                let why = result.err()?;
                // Refused: the panel that asked comes back, and the notice says why.
                match self.show(self.pane.clone()) {
                    Err(reopen) => Some(format!("{why}; {reopen}")),
                    Ok(()) => Some(why),
                }
            }
        }
    }

    pub fn press(&mut self, button: Button) {
        if self.active == Some(Editor::SetupSave)
            && self.name_token.is_none()
            && self.model.watching()
        {
            if button == Button::B {
                self.model.cancel_name();
                self.active = None;
            }
            return;
        }
        if self.active == Some(Editor::SetupLibrary) {
            if let (Button::B, Some(pane)) = (button, self.pane.back()) {
                let _ = self.show(pane);
                return;
            }
            if self.library_count == 0 && matches!(button, Button::A | Button::Y) {
                return;
            }
        }
        external::press_panel(button);
    }

    pub fn panel_up(&self) -> bool {
        !named_macros::suspended()
            && self.active.is_some()
            && (external::panel_up()
                || (self.active == Some(Editor::SetupSave)
                    && self.name_token.is_none()
                    && self.model.watching()))
    }

    pub fn view(&self) -> Option<View> {
        if named_macros::suspended() {
            return None;
        }
        if self.active == Some(Editor::SetupSave)
            && self.name_token.is_none()
            && self.model.watching()
        {
            return Some(View {
                title: "Save setup to library".into(),
                selected: 0,
                labels: vec!["Reading your save…".into()],
                footer: "B Cancel".into(),
            });
        }
        let mut view = external::panel_view_now()?;
        if self.active == Some(Editor::SetupLibrary) {
            if self.pane != Pane::List {
                view.footer = self.pane.footer(&self.label, view.selected);
            } else if self.library_count == 0 {
                view.labels = vec!["No setups saved".into()];
                view.footer = "Save one with Save setup to library first. B Return".into();
            } else {
                view.footer = list_footer(&view.footer);
            }
        }
        Some(view)
    }

    pub fn close_menu(&mut self) -> Result<(), &'static str> {
        let result = self.model.close_menu(&mut Native);
        // Actual close ends this unaccepted UI selection even if validation
        // refused. Accepted jobs survive retire_session and continue to poll.
        self.retire_session();
        result
    }

    /// A true session end; accepted provider tickets are still polled by tick.
    pub fn retire_session(&mut self) {
        self.model.retire_session();
        self.active = None;
        self.target = None;
        self.name_token = None;
    }
}
