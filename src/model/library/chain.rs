//! A chain is an ordered list of recording ids in `macros/`, distinguished by `kind`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Error;
use crate::model::ordered_ids::IdList;
use crate::model::store::Record;

const SCHEMA: u32 = 1;
const MAX_FILE_BYTES: u64 = 64 * 1024;

fn switch_on() -> bool {
    true
}

fn switch_is_on(on: &bool) -> bool {
    *on
}

/// `macros/<id>.json` whose `kind` is `chain`. Recordings omit these; the chain list reports the rest.
pub(crate) fn is_chain_document(bytes: &[u8]) -> bool {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|value| value.get("kind").and_then(Value::as_str).map(|kind| kind == "chain"))
        .unwrap_or(false)
}

/// Ordered recording ids and its own start-save switch. It does not store a save id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chain {
    pub schema_version: u32,
    pub kind: String,
    pub name: String,
    pub recordings: Vec<String>,
    #[serde(default = "switch_on", skip_serializing_if = "switch_is_on")]
    pub start_save_on: bool,
}

impl Chain {
    pub fn new(name: impl Into<String>, recordings: Vec<String>) -> Result<Self, Error> {
        let chain = Self {
            schema_version: SCHEMA,
            kind: "chain".to_owned(),
            name: name.into().trim().to_owned(),
            recordings,
            start_save_on: true,
        };
        chain.validate()?;
        Ok(chain)
    }

    pub fn set_start_save_on(&mut self, on: bool) {
        self.start_save_on = on;
    }
}

pub type ChainEntry = super::ordered::OrderedEntry;

impl IdList for Chain {
    fn ids(&self) -> &[String] {
        &self.recordings
    }

    fn ids_mut(&mut self) -> &mut Vec<String> {
        &mut self.recordings
    }
}

impl Record for Chain {
    type Error = Error;
    type Summary = ChainEntry;
    const SCHEMA_VERSIONS: &'static [u32] = &[SCHEMA];
    const MAX_FILE_BYTES: u64 = MAX_FILE_BYTES;

    fn validate(&self) -> Result<(), Error> {
        super::ordered::validate_ordered(
            self.schema_version == SCHEMA && self.kind == "chain",
            &self.name,
            &self.recordings,
        )
    }

    fn owns(bytes: &[u8]) -> bool {
        is_chain_document(bytes)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn summary(&self, id: &str) -> ChainEntry {
        super::ordered::summary(id, &self.name, self.recordings.len())
    }
}
