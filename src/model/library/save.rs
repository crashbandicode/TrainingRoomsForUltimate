//! A named save: both fighters, the same fields as one index of `save_states.json`.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::{check_label, Error};
use crate::model::state::json::Writer;
use crate::model::state::record::SavedState;
use crate::model::store::Record;

const SCHEMA: u32 = 1;
const MAX_FILE_BYTES: u64 = 512 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct NamedSave {
    pub schema_version: u32,
    pub name: String,
    pub player: SavedState,
    pub cpu: SavedState,
}

impl NamedSave {
    pub fn new(name: impl Into<String>, player: SavedState, cpu: SavedState) -> Result<Self, Error> {
        let save = Self { schema_version: SCHEMA, name: name.into().trim().to_owned(), player, cpu };
        save.validate()?;
        Ok(save)
    }
}

fn state_json(state: &SavedState) -> Result<Value, String> {
    let mut writer = Writer::new();
    state.write_json(&mut writer);
    serde_json::from_str(&writer.finish()).map_err(|err| err.to_string())
}

impl Serialize for NamedSave {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = serde_json::json!({
            "schema_version": self.schema_version,
            "name": self.name,
            "player": state_json(&self.player).map_err(serde::ser::Error::custom)?,
            "cpu": state_json(&self.cpu).map_err(serde::ser::Error::custom)?,
        });
        value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NamedSave {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let object = value.as_object().ok_or_else(|| D::Error::custom("bad schema"))?;
        let schema_version = object
            .get("schema_version")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| D::Error::custom("bad schema"))?;
        let name = object.get("name").and_then(Value::as_str).unwrap_or("").to_owned();
        let player = SavedState::from_json(object.get("player").unwrap_or(&Value::Null));
        let cpu = SavedState::from_json(object.get("cpu").unwrap_or(&Value::Null));
        Ok(Self { schema_version, name, player, cpu })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedSaveEntry {
    pub id: String,
    pub name: String,
}

impl Record for NamedSave {
    type Error = Error;
    type Summary = NamedSaveEntry;
    const SCHEMA_VERSIONS: &'static [u32] = &[SCHEMA];
    const MAX_FILE_BYTES: u64 = MAX_FILE_BYTES;

    fn validate(&self) -> Result<(), Error> {
        if self.schema_version != SCHEMA {
            return Err(Error::Invalid("bad schema"));
        }
        check_label(&self.name)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn summary(&self, id: &str) -> NamedSaveEntry {
        NamedSaveEntry { id: id.to_owned(), name: self.name.clone() }
    }
}
