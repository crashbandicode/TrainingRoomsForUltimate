//! `save_states.json`: five slots per role (SPEC-state-01 §7). Read once, lazily; written whenever both
//! fighters finish a Save. The reader takes what it can: a missing or malformed file gives the defaults
//! of §7.3 (the caller logs the warning), a short array keeps its records and defaults the rest, unknown
//! keys are ignored, every marker comes back as `NoAction`.

use serde_json::Value;

use super::json::Writer;
use super::record::SavedState;

/// Slots per role.
pub const SLOTS: usize = 5;

/// Both roles' five records.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveFile {
    pub player: [SavedState; SLOTS],
    pub cpu: [SavedState; SLOTS],
}

/// Why a file was not used as it stood.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Defaulted {
    /// The bytes are not JSON at all.
    NotJson,
    /// JSON, but not an object with `player` and `cpu` arrays.
    WrongShape,
}

impl Default for SaveFile {
    fn default() -> Self {
        Self { player: std::array::from_fn(|_| SavedState::never_saved()), cpu: std::array::from_fn(|_| SavedState::never_saved()) }
    }
}

impl SaveFile {
    /// Boot/worker only: prepare the game machine's value records. The writer
    /// retains the complete file, including opaque imported Mii JSON. Native
    /// restoration uses the roster's typed identity; it never reads this JSON.
    /// Keeping it out of the game copy makes record copies and replacement
    /// independent of allocator work in a hook.
    pub fn into_game_records(mut self) -> Self {
        for record in self.player.iter_mut().chain(self.cpu.iter_mut()) {
            record.mii = None;
        }
        self
    }

    /// Read the file's bytes. `Err` is why it is not usable: the caller takes `SaveFile::default()`.
    pub fn parse(bytes: &[u8]) -> Result<Self, Defaulted> {
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else { return Err(Defaulted::NotJson) };
        let Some(object) = value.as_object() else { return Err(Defaulted::WrongShape) };
        if !object.contains_key("player") && !object.contains_key("cpu") {
            return Err(Defaulted::WrongShape);
        }
        let mut file = Self::default();
        for (key, records) in [("player", &mut file.player), ("cpu", &mut file.cpu)] {
            if let Some(items) = object.get(key).and_then(Value::as_array) {
                for (slot, item) in records.iter_mut().zip(items) {
                    *slot = SavedState::from_json(item);
                }
            }
        }
        Ok(file)
    }

    /// The pretty JSON the fork writes (`serde_json::to_string_pretty` of its slot arrays).
    pub fn to_pretty(&self) -> String {
        let mut out = Writer::new();
        out.begin_object();
        for (key, records) in [("player", &self.player), ("cpu", &self.cpu)] {
            out.key(key);
            out.begin_array();
            for record in records {
                out.item();
                record.write_json(&mut out);
            }
            out.end_array();
        }
        out.end_object();
        out.finish()
    }
}
