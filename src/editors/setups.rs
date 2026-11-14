//! Worker-owned Setup drafts and close-time selection. Native validation and
//! transaction ownership remain in the states provider; files use SetupLibrary.
use crate::engine::macro_ui::macro_panel::LibraryRow;
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
    let keys = "A Open   X Rename   Y Make Load use it   B Back";
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

/// Which of the Setup library's panels is up: the list, one setup's own screen, or its delete question.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Pane {
    #[default]
    List,
    Own(String),
    Delete(String),
}

/// What a press on a panel's row leads to next.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Open(Pane),
    Choose(String, bool),
    Rename(String),
    /// A copy as a new file (Break the Targets' saved layouts; a setup's screen has no such row).
    Duplicate(String),
    Remove(String),
}

/// A row of a file's own screen or of its delete question: the panel's row id, its label, and what it does (the footer's second line).
type Row = (&'static str, &'static str, &'static str);

/// A file library's two panels past its list: the file's own screen and its delete question (the Setup library's, and Break
/// the Targets' saved layouts', btt-5).
pub struct Screens {
    own: &'static [Row],
    ask: &'static [Row],
}

pub const SETUP: Screens = Screens { own: &OWN, ask: &ASK };
pub const LAYOUT: Screens = Screens { own: &OWN, ask: &ASK }; // stub
const _: [&[Row]; 2] = [&LAYOUT_OWN, &LAYOUT_ASK];

const OWN: [Row; 4] = [
    ("restore", "Restore here", "Both fighters move there when you close the menu."),
    ("load", "Make Load use it", "Nobody moves. Load goes there until the bound save changes."),
    ("rename", "Rename", "Type a new name for this setup."),
    ("delete", "Delete", "Asks once. Your saves are not touched."),
];
const ASK: [Row; 2] = [
    ("keep", "Keep it", "The setup stays in the library."),
    ("delete", "Delete it", "Removes this setup's file. Your saves are not touched."),
];
const LAYOUT_OWN: [Row; 4] = [
    ("load", "Load", "Puts these targets on the stage. Close the menu to start."),
    ("rename", "Rename", "Type a new name for this layout."),
    ("duplicate", "Duplicate", "Saves a copy as a new layout, at the top of the list."),
    ("delete", "Delete", "Asks once."),
];
const LAYOUT_ASK: [Row; 2] = [("keep", "Keep it", "The layout stays."), ("delete", "Delete it", "Removes this layout's file.")];

impl Pane {
    fn table(&self, screens: &Screens) -> &'static [Row] {
        if matches!(self, Self::Delete(_)) {
            screens.ask
        } else {
            screens.own
        }
    }

    /// The panel's rows (a list's come from the library).
    pub fn rows(&self, screens: &Screens) -> Vec<LibraryRow> {
        self.table(screens).iter().map(|(id, label, _)| LibraryRow { id: (*id).into(), label: (*label).into() }).collect()
    }

    /// The footer's two lines: the file (`label`), then what the row under the cursor does.
    pub fn footer(&self, screens: &Screens, label: &str, selected: usize) -> String {
        let ask = if matches!(self, Self::Delete(_)) { "Delete? " } else { "" };
        format!("{ask}{label}\n{}", self.table(screens).get(selected).map_or("", |row| row.2))
    }

    /// A on the row `row` of this panel (a list's row is a file's id).
    pub fn leads_to(&self, row: &str) -> Step {
        match (self, row) {
            (Self::List, id) => Step::Open(Self::Own(id.into())),
            (Self::Own(id), "restore") => Step::Choose(id.clone(), true),
            (Self::Own(id), "load") => Step::Choose(id.clone(), false),
            (Self::Own(id), "rename") => Step::Rename(id.clone()),
            // stub
            (Self::Own(id), _) => Step::Open(Self::Delete(id.clone())),
            (Self::Delete(id), "delete") => Step::Remove(id.clone()),
            (Self::Delete(id), _) => Step::Open(Self::Own(id.clone())),
        }
    }

    /// B: the panel one step up, or none from the list (B closes it).
    pub fn back(&self) -> Option<Self> {
        match self {
            Self::List => None,
            Self::Own(_) => Some(Self::List),
            Self::Delete(id) => Some(Self::Own(id.clone())),
        }
    }
}

/// The naming keyboard's sub-line: whose name it asks for (a rename, where the fighters stand, or the bound save) and the two
/// fighters.
pub fn name_summary(renaming: bool, here: bool, player: &str, cpu: &str) -> String {
    let from = if renaming { "Renaming" } else if here { "Where they stand now" } else { "From your bound save" };
    format!("{from} · {player} / {cpu}")
}

/// Value-only boundary implemented by the native states facade.
pub trait Provider {
    type Target: Copy;
    type Ticket: Copy;
    /// `here`: where both fighters stand now (Save setup here); otherwise the bound save.
    fn snapshot(&mut self, target: Self::Target, here: bool) -> Res<Self::Ticket>;
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
        here: bool,
    ) -> Result<(), &'static str> {
        if self.job.is_some() {
            return Err("A setup is already being saved or restored");
        }
        let ticket = provider.snapshot(target, here)?;
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
    use crate::engine::macro_ui::macro_panel::MacroUi;
    use crate::model::state::{record::SavedState, setup::FighterSetup};
    use crate::model::store::Record;
    type Target = (u64, usize, bool);
    #[derive(Default)]
    struct Fake {
        requested: Vec<(Target, bool)>,
        bound: Vec<(Target, bool, String)>,
        reply: Option<Result<Reply, &'static str>>,
        refuse: bool,
    }
    impl Provider for Fake {
        type Target = Target;
        type Ticket = u32;
        fn snapshot(&mut self, target: Target, here: bool) -> Result<u32, &'static str> {
            self.requested.push((target, here));
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
        ui.begin_save(&mut provider, (2, 4, true), false).unwrap();
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

    /// Save setup here asks for a snapshot of where the fighters stand and keeps it as a new file. Nothing binds, so the bound
    /// save (the slot Load reads) is never written; Save setup from bound save still asks for the bound save.
    fn save_setup_here_reads_the_fighters_and_never_writes_the_bound_save() {
        let dir = std::env::temp_dir().join(format!("tr-setup-here-{}", std::process::id()));
        let library = SetupLibrary::new(dir.join("setups"));
        let mut ui = Setups::default();
        let mut provider = Fake::default();
        ui.begin_save(&mut provider, (2, 0, true), true).unwrap();
        let mut here = record();
        here.player.x = 40.0;
        provider.reply = Some(Ok(Reply::Snapshot(Box::new(here))));
        assert_eq!(ui.poll(&mut provider), Some(Ok(true)));
        let id = ui.save_name(ui.name_revision(), "Ledge trap", &library).unwrap();
        assert_eq!(library.load(&id).unwrap().player.x, 40.0);
        ui.begin_save(&mut provider, (2, 0, true), false).unwrap();
        assert_eq!(provider.requested, vec![((2, 0, true), true), ((2, 0, true), false)]);
        assert!(provider.bound.is_empty(), "nothing binds: the bound save is not written");
        std::fs::remove_dir_all(dir).unwrap();
        // The game side reads both fighters with Save's own reader and never calls Save's slot writers; only a bind writes.
        let game = include_str!("../runtime/features/states/setups.rs");
        assert!(game.contains("[super::read_record(&raws[0]), super::read_record(&raws[1])]"));
        for writer in ["capture(", "hand_to_writer", "save_captured", "request_save"] {
            assert!(!game.contains(writer), "{writer}");
        }
        assert_eq!(game.matches("writing = true").count(), 1, "only a bind hands the slot to the writer");
    }

    /// The help panel draws two footer lines: the cursor row, then the keys. The page note appears only when it says something.
    fn the_list_footer_shows_the_keys_in_two_lines_and_a_page_note_only_when_it_matters() {
        let row = |label: &str| LibraryRow { id: "corner".into(), label: label.into() };
        let footer = |rows: Vec<LibraryRow>, skipped: usize| {
            let mut ui = MacroUi::new();
            ui.open_library_forward(rows, skipped, "Setup library", Some("bind only".into()));
            list_footer(&ui.view().unwrap().footer)
        };
        let keys = "A Open   X Rename   Y Make Load use it   B Back";
        assert_eq!(footer(vec![row("Corner trap · Mario / Little Mac")], 0), format!("Corner trap · Mario / Little Mac\n{keys}"));
        assert_eq!(footer(vec![row("Corner trap")], 2), format!("Corner trap\n{keys}   page 1/1  skipped 2"));
        assert_eq!(footer((0..40).map(|_| row("Setup")).collect(), 0), format!("Setup\n{keys}   page 1/2  skipped 0"));
    }

    /// A on a row opens that setup's own screen; its rows restore it, make Load use it, rename it, or ask once before deleting it; B
    /// steps back one panel.
    fn a_setups_own_screen_walks_from_the_list_to_its_actions_and_back() {
        let (own, ask) = (Pane::Own("corner-trap".into()), Pane::Delete("corner-trap".into()));
        assert_eq!(Pane::List.leads_to("corner-trap"), Step::Open(own.clone()));
        assert_eq!(own.leads_to("restore"), Step::Choose("corner-trap".into(), true));
        assert_eq!(own.leads_to("load"), Step::Choose("corner-trap".into(), false));
        assert_eq!(own.leads_to("rename"), Step::Rename("corner-trap".into()));
        assert_eq!(own.leads_to("delete"), Step::Open(ask.clone()));
        assert_eq!(ask.leads_to("keep"), Step::Open(own.clone()));
        assert_eq!(ask.leads_to("delete"), Step::Remove("corner-trap".into()));
        assert_eq!((Pane::List.back(), own.back(), ask.back()), (None, Some(Pane::List), Some(own)));
    }

    /// The rows are labelled in words, the cursor starts on the safe row of the question, and the footer says what the row under the
    /// cursor does.
    fn the_own_screen_rows_and_footers_say_what_each_row_does() {
        let labels = |pane: &Pane| pane.rows(&SETUP).into_iter().map(|row| row.label).collect::<Vec<_>>();
        let (own, ask) = (Pane::Own("x".into()), Pane::Delete("x".into()));
        assert_eq!(labels(&own), ["Restore here", "Make Load use it", "Rename", "Delete"]);
        assert_eq!(labels(&ask), ["Keep it", "Delete it"]);
        let row = "Corner trap · Mario / Little Mac · Training · spots -40 / 35";
        assert_eq!(own.footer(&SETUP, row, 1), format!("{row}\nNobody moves. Load goes there until the bound save changes."));
        assert!(own.footer(&SETUP, row, 0).ends_with("Both fighters move there when you close the menu."));
        assert!(ask.footer(&SETUP, row, 0).starts_with("Delete? Corner trap"), "{}", ask.footer(&SETUP, row, 0));
        assert_eq!(own.footer(&SETUP, row, 9), format!("{row}\n"), "a cursor off the rows explains nothing");
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

    /// Break the Targets' saved layouts (btt-5) walk the same panes with their own rows: A on a layout opens Load, Rename,
    /// Duplicate and Delete; Delete asks once (Keep it first); B steps back one pane. The Setup library's rows are unchanged.
    fn layout_own_screen_walks_load_rename_duplicate_delete() {
        let (own, ask) = (Pane::Own("ledge-trap".into()), Pane::Delete("ledge-trap".into()));
        let labels = |pane: &Pane, screens: &Screens| pane.rows(screens).into_iter().map(|row| row.label).collect::<Vec<_>>();
        assert_eq!(labels(&own, &LAYOUT), ["Load", "Rename", "Duplicate", "Delete"]);
        assert_eq!(labels(&ask, &LAYOUT), ["Keep it", "Delete it"]);
        assert_eq!(labels(&own, &SETUP), ["Restore here", "Make Load use it", "Rename", "Delete"]);
        assert_eq!(Pane::List.leads_to("ledge-trap"), Step::Open(own.clone()));
        assert_eq!(own.leads_to("load"), Step::Choose("ledge-trap".into(), false));
        assert_eq!(own.leads_to("rename"), Step::Rename("ledge-trap".into()));
        assert_eq!(own.leads_to("duplicate"), Step::Duplicate("ledge-trap".into()));
        assert_eq!(own.leads_to("delete"), Step::Open(ask.clone()));
        assert_eq!((ask.leads_to("keep"), ask.leads_to("delete")), (Step::Open(own.clone()), Step::Remove("ledge-trap".into())));
        assert_eq!((own.back(), ask.back()), (Some(Pane::List), Some(own.clone())));
        let row = "Ledge trap · 3 targets";
        assert_eq!(own.footer(&LAYOUT, row, 0), format!("{row}\nPuts these targets on the stage. Close the menu to start."));
        assert_eq!(ask.footer(&LAYOUT, row, 1), format!("Delete? {row}\nRemoves this layout's file."));
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
        assert_eq!(name_summary(false, false, "Mario", "Little Mac"), "From your bound save · Mario / Little Mac");
        assert_eq!(name_summary(false, true, "Mario", "Little Mac"), "Where they stand now · Mario / Little Mac");
        assert_eq!(name_summary(true, false, "Mario", "Little Mac"), "Renaming · Mario / Little Mac");
        let ui = include_str!("../runtime/ui/setups.rs");
        assert!(ui.contains("name_summary(") && ui.contains("Reading your save…") && ui.contains("Reading where they stand…"));
        assert!(!ui.contains("Save State Slot {slot}") && !ui.contains("Reading the selected saved setup"));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_list_footer_shows_the_keys_in_two_lines_and_a_page_note_only_when_it_matters", the_list_footer_shows_the_keys_in_two_lines_and_a_page_note_only_when_it_matters as fn()),
            ("a_setups_own_screen_walks_from_the_list_to_its_actions_and_back", a_setups_own_screen_walks_from_the_list_to_its_actions_and_back as fn()),
            ("the_own_screen_rows_and_footers_say_what_each_row_does", the_own_screen_rows_and_footers_say_what_each_row_does as fn()),
            ("renaming_a_setup_overwrites_its_file_and_keeps_its_id", renaming_a_setup_overwrites_its_file_and_keeps_its_id as fn()),
            ("layout_own_screen_walks_load_rename_duplicate_delete", layout_own_screen_walks_load_rename_duplicate_delete as fn()),
            ("the_chosen_notes_say_what_will_happen_in_plain_words", the_chosen_notes_say_what_will_happen_in_plain_words as fn()),
            ("a_row_names_the_fighters_the_stage_and_both_spots", a_row_names_the_fighters_the_stage_and_both_spots as fn()),
            ("a_saved_stage_id_has_the_lobby_catalogs_name", a_saved_stage_id_has_the_lobby_catalogs_name as fn()),
            ("the_setup_screens_name_the_bound_save_not_a_slot", the_setup_screens_name_the_bound_save_not_a_slot as fn()),
            ("save_setup_here_reads_the_fighters_and_never_writes_the_bound_save", save_setup_here_reads_the_fighters_and_never_writes_the_bound_save as fn()),
            ("library_selection_freezes_target_and_only_actual_close_binds_once", library_selection_freezes_target_and_only_actual_close_binds_once as fn()),
            ("invalid_name_and_file_failure_keep_draft_cancel_exports_nothing", invalid_name_and_file_failure_keep_draft_cancel_exports_nothing as fn()),
        ]);
    }
    }
}
