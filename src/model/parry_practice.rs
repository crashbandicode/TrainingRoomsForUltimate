//! Parry practice's saved practices (`artifacts/rc15-parry/DESIGN.md` §5.1–5.2): the file record and the usable check.
//! Mounted in `engine` beside the library it names (a by-path test of `model.rs` has no `engine` module).

/// The saved practices' folder under the data folder.
pub const DIR: &str = "parry";

/// A saved practice, `parry/<id>.json` (DESIGN §5.1): references only. The macro is a library id; its start save is the
/// macro's own link (operator Q5); `set` is the session row's option (1 sets of ten, 2 free); `used` orders the hot five.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Practice {
    pub schema_version: u32,
    pub name: String,
    #[serde(rename = "macro")]
    pub macro_id: String,
    #[serde(rename = "loop")]
    pub loop_count: super::library::LoopCount,
    pub set: u8,
    #[serde(default)]
    pub used: u64,
}

impl crate::model::store::Record for Practice {
    type Error = crate::model::store::StoreError;
    type Summary = (String, Practice);
    const SCHEMA_VERSIONS: &'static [u32] = &[1];
    const MAX_FILE_BYTES: u64 = 4096;

    fn validate(&self) -> Result<(), crate::model::store::StoreError> {
        let name = self.name.trim();
        let named = !name.is_empty() && name.chars().count() <= 48 && !name.chars().any(char::is_control);
        if self.schema_version == 1 && named && !self.macro_id.is_empty() && (1..=2).contains(&self.set) {
            Ok(())
        } else {
            Err(crate::model::store::StoreError::Parse(<serde_json::Error as serde::de::Error>::custom("not a parry practice")))
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn summary(&self, id: &str) -> (String, Practice) {
        (id.to_owned(), self.clone())
    }
}

/// Whether a saved practice can start (DESIGN §5.2): its macro still opens and its start-save link is not deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Usable {
    Yes,
    MacroMissing,
    StartSaveMissing,
}

pub fn check(macro_opens: bool, start: super::library::StartSaveState) -> Usable {
    match (macro_opens, start) {
        (false, _) => Usable::MacroMissing,
        (true, super::library::StartSaveState::Deleted) => Usable::StartSaveMissing,
        _ => Usable::Yes,
    }
}

#[cfg(test)]
#[path = "parry_practice_tests.rs"]
mod tests;
