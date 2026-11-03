//! Pending creation is an owned intent, never a zero/partially populated row.
use super::{inventory::{self, MiiClass}, value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Intent {
    NativeDefault(MiiClass),
    Ready,
}

/// Fixed owned storage, exposed as a row only after the intent becomes Ready.
/// The zeroed backing of NativeDefault is not a valid or partially valid row.
#[derive(Clone, Copy, Debug)]
pub(super) struct Draft {
    intent: Intent,
    storage: value::Row,
}

impl PartialEq for Draft {
    fn eq(&self, other: &Self) -> bool {
        self.intent == other.intent && self.row() == other.row()
    }
}
impl Eq for Draft {}

impl Draft {
    pub(super) fn native_default(class: MiiClass) -> Self {
        Self { intent: Intent::NativeDefault(class), storage: [0; core::mem::size_of::<value::Row>()] }
    }

    pub(super) fn ready(row: value::Row) -> Self {
        Self { intent: Intent::Ready, storage: row }
    }

    pub(super) fn pending_class(&self) -> Option<MiiClass> {
        match self.intent { Intent::NativeDefault(class) => Some(class), Intent::Ready => None }
    }

    pub(super) fn validate(&self) -> Result<(), &'static str> {
        match self.row() {
            None => Ok(()),
            Some(row) => value::configuration(row).map(|_| ()),
        }
    }

    pub(super) fn row(&self) -> Option<&value::Row> {
        match self.intent { Intent::Ready => Some(&self.storage), Intent::NativeDefault(_) => None }
    }

    pub(super) fn hash(&self) -> u64 {
        self.row().map_or(0, |row| inventory::fnv1a64(row))
    }

    /// Refine only the matching pending intent. A completed value never selects
    /// a new default on resume, even if native defaults or the catalog change.
    pub(super) fn materialize(&mut self, class: MiiClass, row: value::Row) -> Result<(), &'static str> {
        if self.pending_class() != Some(class) { return Err("default_intent_changed"); }
        if row[..12] != [0; 12] { return Err("default_identity_not_zero"); }
        let config = value::configuration(&row)?;
        if config.canonical().class != class { return Err("default_class_mismatch"); }
        *self = Self::ready(row);
        Ok(())
    }
}
