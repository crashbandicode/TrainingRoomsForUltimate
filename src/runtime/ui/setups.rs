//! Setup screens on the menu worker. The states facade owns native work.
use super::external;
use crate::editors::setups::{Provider, Reply, Setups};
use crate::engine::macro_ui::macro_panel::{LibraryRow, View};
use crate::model::menu::{input::Button, table::Editor, Menu};
use crate::model::state::setup::{SetupLibrary, SetupRecord};
use crate::runtime::features::states::setups as native;
use crate::runtime::macros::named_macros;

struct Native;
impl Provider for Native {
    type Target = native::Target;
    type Ticket = native::Ticket;
    fn snapshot(&mut self, target: Self::Target) -> Result<Self::Ticket, &'static str> {
        native::request_snapshot(target)
    }
    fn validate(
        &mut self,
        target: Self::Target,
        record: &SetupRecord,
        restore: bool,
    ) -> Result<(), &'static str> {
        native::validate_selection(target, record, restore)
    }
    fn bind(
        &mut self,
        target: Self::Target,
        record: &SetupRecord,
        restore: bool,
    ) -> Result<Self::Ticket, &'static str> {
        native::request_bind(target, record, restore)
    }
    fn poll(&mut self, ticket: Self::Ticket) -> Option<Result<Reply, &'static str>> {
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
            Editor::SetupLibrary => self.show_library()?,
            _ => return Err("Not a setup screen".into()),
        }
        self.target = Some(target);
        self.active = Some(editor);
        Ok(())
    }

    fn show_library(&mut self) -> Result<(), String> {
        let listing = library().list().map_err(|e| e.to_string())?;
        self.library_count = listing.entries.len();
        let rows = listing
            .entries
            .into_iter()
            .map(|entry| LibraryRow {
                id: entry.id,
                label: format!(
                    "{} · {} / {}",
                    entry.name,
                    named_macros::fighter_label(entry.player_kind),
                    named_macros::fighter_label(entry.cpu_kind)
                ),
            })
            .collect();
        named_macros::open_external_library(
            rows,
            listing.skipped.len(),
            "Setup library",
            Some("bind only".into()),
        );
        if !external::panel_up() {
            return Err("Wait for the practice fighters to finish loading".into());
        }
        Ok(())
    }

    fn offer_name(&mut self) {
        let Some(record) = self.model.draft() else {
            return;
        };
        let token = self.model.name_revision();
        let slot = self.target.map_or(0, |target| target.slot + 1);
        let summary = format!(
            "Save State Slot {slot} · {} / {}",
            named_macros::fighter_label(record.player.fighter_kind),
            named_macros::fighter_label(record.cpu.fighter_kind)
        );
        self.name_token = Some(token);
        named_macros::offer_external_name(token, &record.name, &summary);
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
                    match self.model.save_name(token, &confirmed.name, &library()) {
                        Ok(_) => {
                            self.name_token = None;
                            self.active = None;
                            return Some(format!("Saved setup {}", confirmed.name.trim()));
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
            let choice = named_macros::take_list_choice()
                .map(|id| (id, true))
                .or_else(|| named_macros::take_list_action().map(|id| (id, false)));
            if let Some((id, restore)) = choice {
                let result = (|| {
                    let target = self.target.ok_or("This setup list has closed")?;
                    let record = library().load(&id).map_err(|e| e.to_string())?;
                    self.model
                        .choose(&mut Native, target, record, restore)
                        .map_err(str::to_owned)
                })();
                match result {
                    Ok(()) => {
                        self.active = None;
                        return Some(
                            if restore {
                                "Setup will bind and restore when the menu closes"
                            } else {
                                "Setup will bind when the menu closes"
                            }
                            .into(),
                        );
                    }
                    Err(why) => {
                        if let Err(reopen) = self.show_library() {
                            return Some(format!("{why}; {reopen}"));
                        }
                        return Some(why);
                    }
                }
            }
        }
        None
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
        if self.active == Some(Editor::SetupLibrary)
            && self.library_count == 0
            && matches!(button, Button::A | Button::Y)
        {
            return;
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
                labels: vec!["Reading the selected saved setup…".into()],
                footer: "B Cancel".into(),
            });
        }
        let mut view = external::panel_view_now()?;
        if self.active == Some(Editor::SetupLibrary) {
            if self.library_count == 0 {
                view.labels = vec!["No setups saved".into()];
                view.footer = "Save a setup from a numbered slot first. B Return".into();
            } else {
                view.footer = format!(
                    "{}
A Bind and restore on menu close · Y Bind only · B Return",
                    view.footer.split("    ").next().unwrap_or("")
                );
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
