//! A whole settings file sent by the test-only hardware bridge (state export `REQ_SET_APPLY`). It is accepted only when it
//! reads with no issue, so a malformed or invalid request changes nothing; boot keeps its own recovery (`parse_bytes`
//! repairs what it can and the caller keeps the bad file).

use super::settings::{self, Loaded};

/// The parsed file, or the number of issues that refuse it.
pub fn accept(bytes: &[u8]) -> Result<Loaded, usize> {
    let loaded = settings::parse_bytes(bytes);
    if loaded.corrupt() { Err(loaded.issues.len()) } else { Ok(loaded) }
}

/// The live-restore readback: the committed menu rows are the file's.
pub fn matches(bytes: &[u8], committed: Option<&settings::Snapshot>) -> bool {
    accept(bytes).is_ok_and(|loaded| committed == Some(&loaded.settings.menu))
}

#[cfg(test)]
#[path = "settings_request_tests.rs"]
mod tests;
