//! Worker-owned Setup drafts and close-time selection. Native validation and
//! transaction ownership remain in the states provider; files use SetupLibrary.
use crate::engine::stages::lobby_stages::STAGES;
use crate::model::state::setup::{SetupEntry, SetupLibrary, SetupRecord};

/// A provider's answer: the value, or the refusal text.
pub type Res<T> = Result<T, &'static str>;

/// The notice after A or Y on a setup: nothing happens until the menu closes.
pub fn chosen(restore: bool) -> &'static str {
    if restore {
        "Setup chosen: both fighters move there when you close the menu"
    } else {
        "Setup chosen: Load will go there once you close the menu. Nobody moves"
    }
}

/// The list's two footer lines, the most the help panel draws: the cursor row, then the keys. The panel's page note stays only when it says something.
pub fn list_footer(footer: &str) -> String {
    let (row, rest) = footer.split_once('\n').unwrap_or((footer, ""));
    let note = rest.split("    ").next().filter(|note| *note != "page 1/1  skipped 0");
    let keys = "A Go there once   X Rename   Y Make Load use it   B Back";
    format!("{row}\n{keys}{}", note.map_or(String::new(), |note| format!("   {note}")))
}

/// The lobby catalog's name of the stage a setup was saved on.
pub fn stage_name(id: Option<i32>) -> &'static str {
    let found = STAGES.iter().find(|(stage, _)| Some(*stage as i32) == id);
    found.map_or("unknown stage", |(_, name)| *name)
}

/// A list row's full text, shown in the footer under the cursor: name, fighters, stage and where each fighter stands.
pub fn row_label(entry: &SetupEntry, fighter: impl Fn(i32) -> String) -> String {
    let (p1, cpu) = (fighter(entry.player_kind), fighter(entry.cpu_kind));
    let (stage, x) = (stage_name(entry.stage_id), (entry.player_x, entry.cpu_x));
    format!("{} · {p1} / {cpu} · {stage} · spots {} / {}", entry.name, x.0, x.1)
}

/// The naming keyboard's sub-line: whose name it asks for and the two fighters.
pub fn name_summary(renaming: bool, player: &str, cpu: &str) -> String {
    format!("{} · {player} / {cpu}", if renaming { "Renaming" } else { "From your bound save" })
}

/// Value-only boundary implemented by the native states facade.
pub trait Provider {
    type Target: Copy;
    type Ticket: Copy;
    fn snapshot(&mut self, target: Self::Target) -> Res<Self::Ticket>;
    fn validate(&mut self, target: Self::Target, record: &SetupRecord, restore: bool) -> Res<()>;
    fn bind(&mut self, target: Self::Target, record: &SetupRecord, restore: bool) -> Res<Self::Ticket>;
    fn poll(&mut self, ticket: Self::Ticket) -> Option<Res<Reply>>;
}

pub enum Reply {
    Snapshot(Box<SetupRecord>),
    Bound,
}

enum Job<K> {
    Snapshot { ticket: K, present: bool },
    Bind(K),
}

struct Selection<T> {
    target: T,
    record: Box<SetupRecord>,
    restore: bool,
}

/// Lives on the menu worker, outside the menu's disposable session. Accepted
/// tickets must be drained even after a panel, renderer or lobby disappears.
pub struct Setups<T, K> {
    job: Option<Job<K>>,
    draft: Option<Box<SetupRecord>>,
    selection: Option<Selection<T>>,
    name_revision: u64,
    /// The setup being renamed (its id): the confirmed name overwrites its file instead of making a new one.
    rename_id: Option<String>,
}

impl<T, K> Default for Setups<T, K> {
    fn default() -> Self {
        Self {
            job: None,
            draft: None,
            selection: None,
            name_revision: 0,
            rename_id: None,
        }
    }
}

impl<T: Copy, K: Copy> Setups<T, K> {
    pub fn watching(&self) -> bool {
        self.job.is_some()
    }

    pub fn draft(&self) -> Option<&SetupRecord> {
        self.draft.as_deref()
    }

    /// Identifies a naming result; an old keyboard cannot save a newer draft.
    pub fn name_revision(&self) -> u64 {
        self.name_revision
    }

    pub fn begin_save<P: Provider<Target = T, Ticket = K>>(
        &mut self,
        provider: &mut P,
        target: T,
    ) -> Result<(), &'static str> {
        if self.job.is_some() {
            return Err("A setup is already being saved or restored");
        }
        let ticket = provider.snapshot(target)?;
        self.cancel_name();
        self.job = Some(Job::Snapshot {
            ticket,
            present: true,
        });
        Ok(())
    }

    /// Library A/Y validate now, but never bind until the actual menu closes.
    /// Returning from the library preserves this selection and frozen target.
    pub fn choose<P: Provider<Target = T, Ticket = K>>(
        &mut self,
        provider: &mut P,
        target: T,
        record: SetupRecord,
        restore: bool,
    ) -> Result<(), &'static str> {
        provider.validate(target, &record, restore)?;
        self.selection = Some(Selection {
            target,
            record: Box::new(record),
            restore,
        });
        Ok(())
    }

    /// On refusal the selection stays available for the caller to retry or discard.
    /// Once accepted, the provider owns it and repeated close cannot post it.
    pub fn close_menu<P: Provider<Target = T, Ticket = K>>(&mut self, provider: &mut P) -> Res<()> {
        let Some(selected) = self.selection.as_ref() else {
            return Ok(());
        };
        if self.job.is_some() {
            return Err("A setup is already being saved or restored");
        }
        let ticket = provider.bind(selected.target, &selected.record, selected.restore)?;
        self.job = Some(Job::Bind(ticket));
        self.selection = None;
        Ok(())
    }

    /// Poll independently of panel visibility. A cancelled snapshot is retired
    /// without reopening its naming screen; errors still reach the caller.
    pub fn poll<P: Provider<Target = T, Ticket = K>>(&mut self, provider: &mut P) -> Option<Res<bool>> {
        let (ticket, present) = match self.job.as_ref()? {
            Job::Snapshot { ticket, present } => (*ticket, *present),
            Job::Bind(ticket) => (*ticket, false),
        };
        let reply = provider.poll(ticket)?;
        let snapshot = matches!(self.job, Some(Job::Snapshot { .. }));
        self.job = None;
        Some(match reply {
            Ok(Reply::Snapshot(record)) if snapshot => {
                if present {
                    self.name_revision = self.name_revision.wrapping_add(1);
                    self.draft = Some(record);
                }
                Ok(present)
            }
            Ok(Reply::Bound) if !snapshot => Ok(false),
            Ok(_) => Err("Unexpected setup transaction reply"),
            Err(why) => Err(why),
        })
    }

    pub fn renaming(&self) -> bool {
        self.rename_id.is_some()
    }

    /// The setup `id`, as loaded, is the draft; the name the player confirms overwrites its file.
    pub fn begin_rename(&mut self, id: String, record: SetupRecord) {
        self.cancel_name();
        self.draft = Some(Box::new(record));
        self.rename_id = Some(id);
    }

    /// Cancel exports nothing. A provider ticket is deliberately not dropped.
    pub fn cancel_name(&mut self) {
        self.draft = None;
        self.rename_id = None;
        self.name_revision = self.name_revision.wrapping_add(1);
        if let Some(Job::Snapshot { present, .. }) = &mut self.job {
            *present = false;
        }
    }

    /// Only a real lobby retirement discards the pending selection. Same-lobby
    /// suspension does not call this: the frozen target and name remain intact.
    pub fn retire_session(&mut self) {
        self.cancel_name();
        self.selection = None;
    }

    /// Validation and I/O failures retain the draft, including the last valid
    /// attempted name. A save creates a new file, never an overwrite; a rename overwrites its own file.
    pub fn save_name(
        &mut self,
        revision: u64,
        name: &str,
        library: &SetupLibrary,
    ) -> Result<String, String> {
        if revision != self.name_revision {
            return Err("This setup naming session has ended".into());
        }
        let draft = self.draft.as_mut().ok_or("No setup draft to save")?;
        draft.set_name(name).map_err(|e| e.to_string())?;
        let saved = match &self.rename_id {
            Some(id) => library.overwrite(id, draft).map(|()| id.clone()),
            None => library.save_new(draft),
        };
        let id = saved.map_err(|e| e.to_string())?;
        self.cancel_name();
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::macro_ui::macro_panel::{LibraryRow, MacroUi};
    use crate::model::state::{record::SavedState, setup::FighterSetup};
    use crate::model::store::Record;
    type Target = (u64, usize, bool);
    #[derive(Default)]
    struct Fake {
        requested: Vec<Target>,
        bound: Vec<(Target, bool, String)>,
        reply: Option<Result<Reply, &'static str>>,
        refuse: bool,
    }
    impl Provider for Fake {
        type Target = Target;
        type Ticket = u32;
        fn snapshot(&mut self, target: Target) -> Result<u32, &'static str> {
            self.requested.push(target);
            Ok(7)
        }
        fn validate(&mut self, _: Target, _: &SetupRecord, _: bool) -> Result<(), &'static str> {
            if self.refuse {
                Err("busy")
            } else {
                Ok(())
            }
        }
        fn bind(
            &mut self,
            target: Target,
            record: &SetupRecord,
            restore: bool,
        ) -> Result<u32, &'static str> {
            self.validate(target, record, restore)?;
            self.bound.push((target, restore, record.name.clone()));
            Ok(9)
        }
        fn poll(&mut self, _: u32) -> Option<Result<Reply, &'static str>> {
            self.reply.take()
        }
    }
    fn record() -> SetupRecord {
        let mut saved = SavedState::never_saved();
        saved.fighter_kind = 0;
        let fighter = FighterSetup::of_saved(&saved);
        SetupRecord::new("Setup", "13.0.5", Some(1), fighter.clone(), fighter).unwrap()
    }

    fn library_selection_freezes_target_and_only_actual_close_binds_once() {
        for restore in [false, true] {
            let mut ui = Setups::default();
            let mut provider = Fake::default();
            let target = (12, 3, restore);
            ui.choose(&mut provider, target, record(), restore).unwrap();
            // A panel return/same-lobby suspension performs no commit or reset.
            assert!(provider.bound.is_empty());
            provider.refuse = true;
            assert_eq!(ui.close_menu(&mut provider), Err("busy"));
            assert!(provider.bound.is_empty());
            provider.refuse = false;
            ui.close_menu(&mut provider).unwrap();
            ui.close_menu(&mut provider).unwrap();
            assert_eq!(provider.bound, vec![(target, restore, "Setup".into())]);
            ui.retire_session();
            assert!(ui.watching());
            provider.reply = Some(Ok(Reply::Bound));
            assert_eq!(ui.poll(&mut provider), Some(Ok(false)));
            assert!(!ui.watching());
        }
    }

    fn invalid_name_and_file_failure_keep_draft_cancel_exports_nothing() {
        let mut ui = Setups::default();
        let mut provider = Fake::default();
        ui.begin_save(&mut provider, (2, 4, true)).unwrap();
        provider.reply = Some(Ok(Reply::Snapshot(Box::new(record()))));
        assert_eq!(ui.poll(&mut provider), Some(Ok(true)));
        let token = ui.name_revision();
        let dir = std::env::temp_dir().join(format!("tr-setup-ui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let blocker = dir.join("not-a-directory");
        std::fs::write(&blocker, b"keep").unwrap();
        let library = SetupLibrary::new(&blocker);
        assert!(ui.save_name(token, "  ", &library).is_err());
        assert_eq!(ui.draft().unwrap().name, "Setup");
        assert!(ui.save_name(token, "Kept draft", &library).is_err());
        assert_eq!(ui.draft().unwrap().name, "Kept draft");
        assert_eq!(std::fs::read(&blocker).unwrap(), b"keep");
        ui.cancel_name();
        assert!(ui
            .save_name(
                token,
                "Late confirmation",
                &SetupLibrary::new(dir.join("setups"))
            )
            .is_err());
        assert!(!dir.join("setups").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// The help panel draws two footer lines: the cursor row, then the keys. The page note appears only when it says something.
    fn the_list_footer_shows_the_keys_in_two_lines_and_a_page_note_only_when_it_matters() {
        let row = |label: &str| LibraryRow { id: "corner".into(), label: label.into() };
        let footer = |rows: Vec<LibraryRow>, skipped: usize| {
            let mut ui = MacroUi::new();
            ui.open_library_forward(rows, skipped, "Setup library", Some("bind only".into()));
            list_footer(&ui.view().unwrap().footer)
        };
        let keys = "A Go there once   X Rename   Y Make Load use it   B Back";
        assert_eq!(footer(vec![row("Corner trap · Mario / Little Mac")], 0), format!("Corner trap · Mario / Little Mac\n{keys}"));
        assert_eq!(footer(vec![row("Corner trap")], 2), format!("Corner trap\n{keys}   page 1/1  skipped 2"));
        assert_eq!(footer((0..40).map(|_| row("Setup")).collect(), 0), format!("Setup\n{keys}   page 1/2  skipped 0"));
    }

    /// X (or Rename on a setup's own screen): the setup's record is the draft and the confirmed name overwrites its file, so the id
    /// and every other field stay; a cancelled rename changes nothing and leaves no rename behind.
    fn renaming_a_setup_overwrites_its_file_and_keeps_its_id() {
        let dir = std::env::temp_dir().join(format!("tr-setup-rename-{}", std::process::id()));
        let library = SetupLibrary::new(dir.join("setups"));
        let id = library.save_new(&record()).unwrap();
        let mut ui = Setups::<Target, u32>::default();
        ui.begin_rename(id.clone(), library.load(&id).unwrap());
        assert!(ui.renaming());
        let token = ui.name_revision();
        assert_eq!(ui.save_name(token, "Corner trap", &library).unwrap(), id);
        assert!(!ui.renaming());
        assert_eq!(library.load(&id).unwrap().name, "Corner trap");
        assert_eq!(library.list().unwrap().entries.len(), 1, "a rename writes no second file");
        ui.begin_rename(id.clone(), library.load(&id).unwrap());
        ui.cancel_name();
        assert!(!ui.renaming() && ui.save_name(ui.name_revision(), "Never", &library).is_err());
        assert_eq!(library.load(&id).unwrap().name, "Corner trap");
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn the_chosen_notes_say_what_will_happen_in_plain_words() {
        let (go, load) = (chosen(true), chosen(false));
        assert!(go.contains("both fighters move there") && go.contains("close the menu"), "{go}");
        assert!(load.contains("Load will go there") && load.contains("Nobody moves"), "{load}");
        assert!(!go.contains("bind") && !load.contains("bind"));
    }

    fn a_row_names_the_fighters_the_stage_and_both_spots() {
        let entry = SetupEntry { id: "corner-trap".into(), name: "Corner trap".into(), player_kind: 0, cpu_kind: 0x34, stage_id: Some(308), player_x: -40, cpu_x: 35 };
        let fighter = |kind: i32| if kind == 0 { "Mario".to_owned() } else { "Little Mac".to_owned() };
        assert_eq!(row_label(&entry, fighter), "Corner trap · Mario / Little Mac · Training · spots -40 / 35");
        let mut saved = SavedState::never_saved();
        saved.fighter_kind = 0;
        saved.x = -39.6;
        let player = FighterSetup::of_saved(&saved);
        let record = SetupRecord::new("Setup", "13.0.5", Some(0), player.clone(), player).unwrap();
        assert_eq!((record.summary("setup").player_x, record.summary("setup").cpu_x), (-40, -40), "the spots round to whole units");
    }

    fn a_saved_stage_id_has_the_lobby_catalogs_name() {
        assert_eq!((stage_name(Some(308)), stage_name(Some(0))), ("Training", "Battlefield"));
        assert_eq!((stage_name(None), stage_name(Some(-5))), ("unknown stage", "unknown stage"));
    }

    /// The naming keyboard says where the name goes (a rename, or a new setup from the bound save), never a numbered slot the player
    /// no longer sees. The runtime screens are Switch-only, so the reading screen's words are read from the source.
    fn the_setup_screens_name_the_bound_save_not_a_slot() {
        assert_eq!(name_summary(false, "Mario", "Little Mac"), "From your bound save · Mario / Little Mac");
        assert_eq!(name_summary(true, "Mario", "Little Mac"), "Renaming · Mario / Little Mac");
        let ui = include_str!("../runtime/ui/setups.rs");
        assert!(ui.contains("name_summary(") && ui.contains("Reading your save…"));
        assert!(!ui.contains("Save State Slot {slot}") && !ui.contains("Reading the selected saved setup"));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_list_footer_shows_the_keys_in_two_lines_and_a_page_note_only_when_it_matters", the_list_footer_shows_the_keys_in_two_lines_and_a_page_note_only_when_it_matters as fn()),
            ("renaming_a_setup_overwrites_its_file_and_keeps_its_id", renaming_a_setup_overwrites_its_file_and_keeps_its_id as fn()),
            ("the_chosen_notes_say_what_will_happen_in_plain_words", the_chosen_notes_say_what_will_happen_in_plain_words as fn()),
            ("a_row_names_the_fighters_the_stage_and_both_spots", a_row_names_the_fighters_the_stage_and_both_spots as fn()),
            ("a_saved_stage_id_has_the_lobby_catalogs_name", a_saved_stage_id_has_the_lobby_catalogs_name as fn()),
            ("the_setup_screens_name_the_bound_save_not_a_slot", the_setup_screens_name_the_bound_save_not_a_slot as fn()),
            ("library_selection_freezes_target_and_only_actual_close_binds_once", library_selection_freezes_target_and_only_actual_close_binds_once as fn()),
            ("invalid_name_and_file_failure_keep_draft_cancel_exports_nothing", invalid_name_and_file_failure_keep_draft_cancel_exports_nothing as fn()),
        ]);
    }
    }
}
