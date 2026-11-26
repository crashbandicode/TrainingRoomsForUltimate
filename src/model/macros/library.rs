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

use super::recording::{MacroError, MacroRecording, Placement, MII_SCHEMA_VERSION, SCHEMA_VERSION};

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
#[derive(Clone, Debug, PartialEq)]
pub struct MacroEntry {
    pub id: String,
    pub name: String,
    pub fighter_kind: i32,
    pub costume: u32,
    pub frame_count: usize,
    pub trim: crate::model::trim::Trim,
    pub placement: Placement,
    pub start_save: Option<String>,
    pub start_save_on: bool,
}

impl Store<MacroRecording> {
    /// Loads a macro for the clip editor: the macro, and its positions when the file beside it binds it
    /// (`positions`). Playback, the list and every other load read the macro only (`load`).
    pub fn load_edited(&self, id: &str) -> Result<MacroRecording, MacroError> {
        let mut recording = self.load(id)?;
        if let Some((source, track)) = super::positions::read_with_source(self.dir(), id, &recording.frames) {
            recording.set_sourced_track(source, track);
        }
        Ok(recording)
    }
}

/// The named-macro library: the original API, now on the generic store.
pub type Library = Store<MacroRecording>;

impl Record for MacroRecording {
    type Error = MacroError;
    type Summary = MacroEntry;
    const SCHEMA_VERSIONS: &'static [u32] = &[SCHEMA_VERSION, MII_SCHEMA_VERSION];
    const MAX_FILE_BYTES: u64 = MAX_FILE_BYTES;
    const STREAM_READ: bool = true;
    fn summary_name(summary: &MacroEntry) -> &str { &summary.name }
    fn owns_reader(reader: impl std::io::Read) -> serde_json::Result<bool> {
        #[derive(serde::Deserialize)]
        struct Header { kind: Option<String> }
        Ok(serde_json::from_reader::<_, Header>(reader)?.kind.as_deref() != Some("chain"))
    }

    /// A new take or crop under the id: the positions file there goes first, so an interrupted save never leaves an
    /// earlier take's positions bound to the new macro. The same take written again (a rename, a placement, Trim)
    /// carries no positions in memory and keeps the file that binds its inputs.
    fn before_write(&self, dir: &std::path::Path, id: &str) {
        if self.track.is_some() || !super::positions::binds(dir, id, &self.frames) {
            super::positions::remove(dir, id);
        }
    }

    /// The positions go beside the macro once it is in place. A failed write leaves a macro without positions.
    fn after_write(&self, dir: &std::path::Path, id: &str) {
        if let Some(track) = &self.track {
            if super::positions::write(dir, id, &self.frames, track, self.track_source).is_err() {
                super::positions::remove(dir, id);
            }
        }
    }

    fn after_delete(dir: &std::path::Path, id: &str) {
        super::positions::remove(dir, id);
    }

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
            trim: self.trim,
            placement: self.placement,
            start_save: self.start_save.clone(),
            start_save_on: self.start_save_on,
        }
    }
}

#[cfg(test)]
mod tests;
