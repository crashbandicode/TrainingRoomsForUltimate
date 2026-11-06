//! A sequence is an ordered list of combo ids, plus the streak and the best streak.

use serde::{Deserialize, Serialize};

use super::{check_ids, check_label, Error};
use crate::model::combo_record::ComboLibrary;
use crate::model::store::Record;

const SCHEMA: u32 = 1;
const MAX_FILE_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sequence {
    pub schema_version: u32,
    pub name: String,
    pub combos: Vec<String>,
    #[serde(default)]
    pub streak: u32,
    #[serde(default)]
    pub best: u32,
}

impl Sequence {
    pub fn new(name: impl Into<String>, combos: Vec<String>) -> Result<Self, Error> {
        let sequence = Self {
            schema_version: SCHEMA,
            name: name.into().trim().to_owned(),
            combos,
            streak: 0,
            best: 0,
        };
        sequence.validate()?;
        Ok(sequence)
    }

    /// Combo ids that still open, in file order. A renamed combo keeps its id. A deleted one is skipped.
    pub fn present_ids(&self, combos: &ComboLibrary) -> Vec<String> {
        self.combos.iter().filter(|id| combos.load(id).is_ok()).cloned().collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceEntry {
    pub id: String,
    pub name: String,
    pub steps: usize,
}

impl Record for Sequence {
    type Error = Error;
    type Summary = SequenceEntry;
    const SCHEMA_VERSIONS: &'static [u32] = &[SCHEMA];
    const MAX_FILE_BYTES: u64 = MAX_FILE_BYTES;

    fn validate(&self) -> Result<(), Error> {
        if self.schema_version != SCHEMA {
            return Err(Error::Invalid("bad schema"));
        }
        check_label(&self.name)?;
        check_ids(&self.combos)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn summary(&self, id: &str) -> SequenceEntry {
        SequenceEntry { id: id.to_owned(), name: self.name.clone(), steps: self.combos.len() }
    }
}
