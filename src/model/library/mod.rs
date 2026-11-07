//! Named saves, chains, sequences, and the recording index (recording redesign, file half).
//!
//! Content files stay where they are: recordings and chains in `macros/`, sequences in `sequences/`,
//! named saves in `saves/`. `recording-index.json` holds use, the hot five, loop counts, the bound
//! save, each role's active macro, and the drill. A content file is not rewritten because it was used.

mod chain;
mod hot;
mod migrate;
mod ordered;
mod save;
mod sequence;

use serde::{Deserialize, Serialize};

pub use chain::{Chain, ChainEntry};
pub use hot::{RecordingIndex, INDEX_FILE};
pub use migrate::{migrate, LegacySlot, MigrationReceipt};
pub use save::{NamedSave, NamedSaveEntry};
pub use sequence::{Sequence, SequenceEntry};

/// Directory of recordings and chains, under the TrainingRooms data folder.
pub const MACROS_DIR: &str = "macros";
/// Directory of named saves.
pub const SAVES_DIR: &str = "saves";
/// Directory of sequences.
pub const SEQUENCES_DIR: &str = "sequences";

/// A content kind the index keeps a hot list for. A chain is a macro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Macro,
    Save,
    Sequence,
    Combo,
}

/// Player 1 or the CPU. Each role has its own active macro and loop count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Player,
    Cpu,
}

/// How a file became used. Every one of these updates that kind's hot list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    Bound,
    SetActive,
    Played,
    Drilled,
    Recorded,
}

/// Loop counts the menu can land on. Anything else is not a count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopCount {
    Finite(u8),
    Infinite,
}

impl Default for LoopCount {
    fn default() -> Self {
        LoopCount::Finite(1)
    }
}

impl TryFrom<u8> for LoopCount {
    type Error = ();

    fn try_from(count: u8) -> Result<Self, ()> {
        match count {
            1 | 2 | 3 | 5 | 10 => Ok(LoopCount::Finite(count)),
            _ => Err(()),
        }
    }
}

impl Serialize for LoopCount {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            LoopCount::Infinite => serializer.serialize_str("infinite"),
            LoopCount::Finite(count) => serializer.serialize_u8(*count),
        }
    }
}

impl<'de> Deserialize<'de> for LoopCount {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(text) if text == "infinite" => Ok(LoopCount::Infinite),
            serde_json::Value::Number(number) => {
                let count = number.as_u64().and_then(|n| u8::try_from(n).ok()).ok_or_else(|| {
                    serde::de::Error::custom("bad loop")
                })?;
                LoopCount::try_from(count).map_err(|()| serde::de::Error::custom("bad loop"))
            }
            _ => Err(serde::de::Error::custom("bad loop")),
        }
    }
}

/// The one drill. A combo and a sequence replace each other.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Drill {
    pub kind: DrillKind,
    pub id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrillKind {
    Combo,
    Sequence,
}

/// What a recording's start-save id means. The sentence for the deleted case is a later slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartSaveState {
    /// The recording never had a start save.
    None,
    /// The id names a save file that is still there. The switch does not change this.
    Linked,
    /// The id is set and the file is gone. The switch does not change this.
    Deleted,
}

/// `on` is accepted so a caller can pass the switch. Off with the file present stays linked.
pub fn start_save_state(id: Option<&str>, on: bool, file_exists: bool) -> StartSaveState {
    let _switch = on;
    match id {
        None | Some("") => StartSaveState::None,
        Some(_) if file_exists => StartSaveState::Linked,
        Some(_) => StartSaveState::Deleted,
    }
}

/// Failures of the index, the new record types, and migration.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(&'static str),
    Record(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => write!(f, "i/o error: {err}"),
            Error::Json(err) => write!(f, "malformed file: {err}"),
            Error::Invalid(why) => f.write_str(why),
            Error::Record(why) => f.write_str(why),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

impl From<crate::model::store::StoreError> for Error {
    fn from(err: crate::model::store::StoreError) -> Self {
        Error::Record(err.to_string())
    }
}

pub(crate) fn check_label(name: &str) -> Result<(), Error> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 48 || name.chars().any(char::is_control) {
        return Err(Error::Invalid("bad name"));
    }
    Ok(())
}

pub(crate) fn check_ids(ids: &[String]) -> Result<(), Error> {
    if ids.iter().all(|id| crate::model::store::default_id(id)) {
        Ok(())
    } else {
        Err(Error::Invalid("bad id"))
    }
}

#[cfg(test)]
mod tests;
