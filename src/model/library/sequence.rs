//! A sequence is an ordered list of combo ids, plus the streak and the best streak.

use serde::{Deserialize, Serialize};

use super::Error;
#[cfg(test)]
use crate::model::combo_record::ComboLibrary;
#[cfg(test)]
use crate::model::ordered_ids::IdList;
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
    #[cfg(test)]
    pub fn present_ids(&self, combos: &ComboLibrary) -> Vec<String> {
        self.combos.iter().filter(|id| combos.load(id).is_ok()).cloned().collect()
    }
}

pub type SequenceEntry = super::ordered::OrderedEntry;

#[cfg(test)]
impl IdList for Sequence {
    fn ids(&self) -> &[String] {
        &self.combos
    }

    fn ids_mut(&mut self) -> &mut Vec<String> {
        &mut self.combos
    }
}

impl Record for Sequence {
    type Error = Error;
    type Summary = SequenceEntry;
    const SCHEMA_VERSIONS: &'static [u32] = &[SCHEMA];
    const MAX_FILE_BYTES: u64 = MAX_FILE_BYTES;

    fn validate(&self) -> Result<(), Error> {
        super::ordered::validate_ordered_combos(self.schema_version == SCHEMA, &self.name, &self.combos)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn summary(&self, id: &str) -> SequenceEntry {
        super::ordered::summary(id, &self.name, self.combos.len())
    }
}
