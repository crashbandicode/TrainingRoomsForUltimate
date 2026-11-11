//! Worker-owned Setup drafts and close-time selection. Native validation and
//! transaction ownership remain in the states provider; files use SetupLibrary.
use crate::model::state::setup::{SetupLibrary, SetupRecord};

/// Value-only boundary implemented by the native states facade.
pub trait Provider {
    type Target: Copy;
    type Ticket: Copy;
    fn snapshot(&mut self, target: Self::Target) -> Result<Self::Ticket, &'static str>;
    fn validate(
        &mut self,
        target: Self::Target,
        record: &SetupRecord,
        restore: bool,
    ) -> Result<(), &'static str>;
    fn bind(
        &mut self,
        target: Self::Target,
        record: &SetupRecord,
        restore: bool,
    ) -> Result<Self::Ticket, &'static str>;
    fn poll(&mut self, ticket: Self::Ticket) -> Option<Result<Reply, &'static str>>;
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
}

impl<T, K> Default for Setups<T, K> {
    fn default() -> Self {
        Self {
            job: None,
            draft: None,
            selection: None,
            name_revision: 0,
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
            return Err("A setup transaction is already in progress");
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
    pub fn close_menu<P: Provider<Target = T, Ticket = K>>(
        &mut self,
        provider: &mut P,
    ) -> Result<(), &'static str> {
        let Some(selected) = self.selection.as_ref() else {
            return Ok(());
        };
        if self.job.is_some() {
            return Err("A setup transaction is already in progress");
        }
        let ticket = provider.bind(selected.target, &selected.record, selected.restore)?;
        self.job = Some(Job::Bind(ticket));
        self.selection = None;
        Ok(())
    }

    /// Poll independently of panel visibility. A cancelled snapshot is retired
    /// without reopening its naming screen; errors still reach the caller.
    pub fn poll<P: Provider<Target = T, Ticket = K>>(
        &mut self,
        provider: &mut P,
    ) -> Option<Result<bool, &'static str>> {
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

    /// Cancel exports nothing. A provider ticket is deliberately not dropped.
    pub fn cancel_name(&mut self) {
        self.draft = None;
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
    /// attempted name. Successful save creates a new file, never an overwrite.
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
        let id = library.save_new(draft).map_err(|e| e.to_string())?;
        self.cancel_name();
        Ok(id)
    }
}

