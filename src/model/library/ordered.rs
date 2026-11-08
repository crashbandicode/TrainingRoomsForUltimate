//! Shared validation and summary for an ordered list of ids.
//!
//! The documents stay different. A chain still has `kind` and `start_save_on`. A sequence still has
//! `streak` and `best`. The JSON keys stay `recordings` and `combos`.

use super::{check_combo_ids, check_ids, check_label, Error};

/// What a chain list and a sequence list both report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderedEntry {
    pub id: String,
    pub name: String,
    pub steps: usize,
}

/// Schema, then the label, then the ids. The error strings stay `bad schema`, `bad name`, and `bad id`.
pub fn validate_ordered(schema_ok: bool, name: &str, ids: &[String]) -> Result<(), Error> {
    if !schema_ok {
        return Err(Error::Invalid("bad schema"));
    }
    check_label(name)?;
    check_ids(ids)
}

/// A sequence: the same checks, with combo ids (`check_combo_ids`).
pub fn validate_ordered_combos(schema_ok: bool, name: &str, ids: &[String]) -> Result<(), Error> {
    if !schema_ok {
        return Err(Error::Invalid("bad schema"));
    }
    check_label(name)?;
    check_combo_ids(ids)
}

pub fn summary(id: &str, name: &str, steps: usize) -> OrderedEntry {
    OrderedEntry {
        id: id.to_owned(),
        name: name.to_owned(),
        steps,
    }
}
