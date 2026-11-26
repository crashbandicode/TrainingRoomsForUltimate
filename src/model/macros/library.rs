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
use super::recording::Placement;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, Visitor};

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

// Feed the normal macro decoder its normal fields, but skip position rows when only
// making a menu entry. Their shape and values are checked when the macro is opened.
struct HeaderFields<'a, A> { map: A, track: &'a mut Option<bool> }

impl<'de, A: MapAccess<'de>> MapAccess<'de> for HeaderFields<'_, A> {
    type Error = A::Error;
    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> {
        while let Some(key) = self.map.next_key::<String>()? {
            if key != "track" { return seed.deserialize(key.into_deserializer()).map(Some); }
            if self.track.is_some() { return Err(de::Error::duplicate_field("track")); }
            *self.track = Some(self.map.next_value::<Option<de::IgnoredAny>>()?.is_some());
        }
        Ok(None)
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Self::Error> {
        self.map.next_value_seed(seed)
    }
}

struct MacroHeader;
impl<'de> Visitor<'de> for MacroHeader {
    type Value = (MacroRecording, bool);
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("a named macro") }
    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let mut track = None;
        let fields = de::value::MapAccessDeserializer::new(HeaderFields { map, track: &mut track });
        let rec = serde::Deserialize::deserialize(fields)?;
        Ok((rec, track.unwrap_or(false)))
    }
}

/// A macro read without its position rows, for the list and for playback. The file's version is still checked against
/// the presence of the key; the rows' shape and values are checked when the macro is opened whole.
fn read_header(reader: &mut dyn std::io::Read) -> serde_json::Result<Result<MacroRecording, MacroError>> {
    let mut de = serde_json::Deserializer::from_reader(reader);
    // A `track` key (a file of a build that kept positions in the macro) is skipped, here as in a whole load.
    let (rec, _) = serde::Deserializer::deserialize_map(&mut de, MacroHeader)?;
    de.end()?;
    Ok(rec.validate_header().and_then(|()| {
        rec.validate_frame_count()?;
        Ok(rec)
    }))
}

impl Store<MacroRecording> {
    /// Loads a macro for the clip editor: the macro, and its positions when the file beside it binds it
    /// (`positions`). Playback, the list and every other load read the macro only (`load`).
    pub fn load_edited(&self, id: &str) -> Result<MacroRecording, MacroError> {
        self.load(id)
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
    fn read_summary(id: &str, reader: &mut dyn std::io::Read) -> serde_json::Result<Result<MacroEntry, MacroError>> {
        Ok(read_header(reader)?.map(|rec| rec.summary(id)))
    }
    fn summary_name(summary: &MacroEntry) -> &str { &summary.name }
    fn stream_write(&self) -> bool { self.track.is_some() }
    fn owns_reader(reader: impl std::io::Read) -> serde_json::Result<bool> {
        #[derive(serde::Deserialize)]
        struct Header { kind: Option<String> }
        Ok(serde_json::from_reader::<_, Header>(reader)?.kind.as_deref() != Some("chain"))
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
