//! The newest-state slot and the transactional JSON write behind the Mii editor bridge's trace (lead ruling 2026-11-02 on
//! `mii-editor-last.json`, review R1/R5): a game callback only copies its latest state into a slot (no allocation, no file I/O); a
//! writer thread takes the slot and writes the file through `model::store::write_replace`, so a reader or an interrupted write never
//! sees a partial file and a failed write leaves the old file in place. A newer state replaces an older unwritten one (coalesced).
//! Written for this repository, not carried.

use std::io;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;

/// The newest unwritten state of one kind.
pub struct LatestSlot<T> {
    value: Mutex<Option<T>>,
}

impl<T> LatestSlot<T> {
    pub const fn new() -> Self {
        Self { value: Mutex::new(None) }
    }

    /// Store `value`, replacing an unwritten older one.
    pub fn put(&self, value: T) {
        if let Ok(mut slot) = self.value.lock() {
            *slot = Some(value);
        }
    }

    /// Take the newest state, if any.
    pub fn take(&self) -> Option<T> {
        self.value.lock().ok().and_then(|mut slot| slot.take())
    }

    /// Give a state back after a failed write, unless a newer one arrived meanwhile.
    pub fn restore(&self, value: T) {
        if let Ok(mut slot) = self.value.lock() {
            slot.get_or_insert(value);
        }
    }
}

impl<T> Default for LatestSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// `value` as pretty JSON, written to `path` transactionally.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    crate::model::store::write_replace(path, &bytes)
}

/// One writer-thread turn: write the newest state of `slot` to `path`. `Ok(false)` when the slot was empty; on a failed write the
/// state goes back into the slot (unless a newer one is there) and the error is returned, with the old file untouched.
pub fn flush<T: Serialize>(slot: &LatestSlot<T>, path: &Path) -> io::Result<bool> {
    flush_with(slot, |value| write_json(path, value))
}

/// [`flush`] with the writer given as a function (the carried bridge's `write_trace` logs its event line and then calls
/// [`write_json`]).
pub fn flush_with<T>(slot: &LatestSlot<T>, write: impl FnOnce(&T) -> io::Result<()>) -> io::Result<bool> {
    let Some(value) = slot.take() else { return Ok(false) };
    match write(&value) {
        Ok(()) => Ok(true),
        Err(error) => {
            slot.restore(value);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    struct Dir(std::path::PathBuf);
    impl Dir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("tr-trace-slot-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn read(path: &Path) -> Value {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn the_newest_state_replaces_the_older_ones_and_one_write_carries_it() {
        let dir = Dir::new("coalesce");
        let path = dir.0.join("trace.json");
        let slot = LatestSlot::new();
        for n in 1..=3 {
            slot.put(json!({ "events": n }));
        }
        assert!(flush(&slot, &path).unwrap());
        assert_eq!(read(&path), json!({ "events": 3 }));
        assert!(!flush(&slot, &path).unwrap(), "nothing is left to write");
    }

    #[test]
    fn two_loads_see_the_first_then_the_replacement_and_never_a_temporary() {
        let dir = Dir::new("two-loads");
        let path = dir.0.join("trace.json");
        let slot = LatestSlot::new();
        slot.put(json!({ "events": 1 }));
        flush(&slot, &path).unwrap();
        assert_eq!(read(&path), json!({ "events": 1 }));
        slot.put(json!({ "events": 2 }));
        flush(&slot, &path).unwrap();
        assert_eq!(read(&path), json!({ "events": 2 }));
        let names: Vec<_> = std::fs::read_dir(&dir.0).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        assert_eq!(names, ["trace.json"]);
    }

    #[test]
    fn a_failed_write_keeps_the_old_file_and_the_state_for_the_next_turn() {
        let dir = Dir::new("failed");
        let path = dir.0.join("trace.json");
        let slot = LatestSlot::new();
        slot.put(json!({ "events": 1 }));
        flush(&slot, &path).unwrap();
        // the target's directory goes away: the temporary file cannot be created
        let missing = dir.0.join("gone").join("trace.json");
        slot.put(json!({ "events": 2 }));
        assert!(flush(&slot, &missing).is_err());
        assert_eq!(read(&path), json!({ "events": 1 }), "the old file is untouched");
        // the state was given back, so the next turn (to a writable place) writes it
        assert!(flush(&slot, &path).unwrap());
        assert_eq!(read(&path), json!({ "events": 2 }));
    }

    #[test]
    fn a_state_that_arrived_during_a_failed_write_is_not_overwritten_by_the_older_one() {
        let slot = LatestSlot::new();
        slot.put(1);
        let old = slot.take().unwrap();
        slot.put(2);
        slot.restore(old);
        assert_eq!(slot.take(), Some(2));
    }
}
