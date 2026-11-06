//! Directory-backed record library: one compact JSON file per entry, named
//! `<id>.json`, in a caller-chosen directory. There is no entry-count limit.
//!
//! [`Store`] is generic over the stored [`Record`]. [`Library`] is its
//! named-macro instantiation and keeps the original API; the portable practice
//! setup in `training_mod_consts` is a second instantiation that reuses the id,
//! size, version and atomic-write rules below instead of copying them.
//!
//! * Ids are stable and path-safe (`[a-z0-9_-]`, at most [`MAX_ID_CHARS`]). The
//!   id is the file stem; it never changes when the record is overwritten.
//! * Updates write `<id>.json.tmp`, sync it, then rename it over the target, so
//!   a reader or an interrupted write never sees a partial file. Where rename
//!   cannot replace an existing file (FAT on Horizon is the concern), the old
//!   file is moved to `<id>.json.bak` first and put back if the swap fails.
//! * `list` never deletes or rewrites anything except restoring an orphaned
//!   `.bak` (left only by a crash inside that swap).

use crate::model::store::{Record, Store, StoreError, MAX_FILE_BYTES};

use super::recording::{MacroError, MacroRecording, MII_SCHEMA_VERSION, SCHEMA_VERSION};

/// The store's own failures become the macro crate's error, variant for variant (the carried code matches on `MacroError`).
impl From<StoreError> for MacroError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::UnsupportedVersion { found } => Self::UnsupportedVersion { found },
            StoreError::FileTooLarge { bytes, max } => Self::FileTooLarge { bytes, max },
            StoreError::InvalidId(id) => Self::InvalidId(id),
            StoreError::NotFound(id) => Self::NotFound(id),
            StoreError::NoFreeId => Self::NoFreeId,
            StoreError::Parse(err) => Self::Parse(err),
            StoreError::Io(err) => Self::Io(err),
        }
    }
}

/// Summary of one loadable macro, enough to fill a selection dropdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroEntry {
    pub id: String,
    pub name: String,
    pub fighter_kind: i32,
    pub costume: u32,
    pub frame_count: usize,
}

/// The named-macro library: the original API, now on the generic store.
pub type Library = Store<MacroRecording>;

impl Record for MacroRecording {
    type Error = MacroError;
    type Summary = MacroEntry;
    const SCHEMA_VERSIONS: &'static [u32] = &[SCHEMA_VERSION, MII_SCHEMA_VERSION];
    const MAX_FILE_BYTES: u64 = MAX_FILE_BYTES;

    fn validate(&self) -> Result<(), MacroError> {
        MacroRecording::validate(self)
    }

    fn owns(bytes: &[u8]) -> bool {
        !super::recording::is_chain_document(bytes)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn summary(&self, id: &str) -> MacroEntry {
        MacroEntry {
            id: id.to_owned(),
            name: self.name.clone(),
            fighter_kind: self.start.fighter_kind,
            costume: self.start.costume,
            frame_count: self.frames.len(),
        }
    }
}

#[cfg(test)]
mod tests;
