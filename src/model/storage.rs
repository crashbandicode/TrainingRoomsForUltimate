//! Where the plugin keeps its files (SPEC-core-08, SPEC-core-01 §4.3/§7, SPEC-core-04 §4).
//!
//! Everything lives under `sd:/ultimate/TrainingRooms/` (decision 2026-11-01); the earlier fork's
//! `sd:/ultimate/TrainingModpack/` is only ever read, by the one-time import. Paths are built in
//! fixed stack buffers, so building one allocates nothing (hooks and the boot thread).

use core::fmt::{self, Write};

/// The plugin's data folder.
pub const ROOT: &str = "sd:/ultimate/TrainingRooms";
/// The earlier fork's data folder: read by the import, never written.
pub const FORK_ROOT: &str = "sd:/ultimate/TrainingModpack";

/// File names inside `ROOT`.
pub mod file {
    /// The player's settings and saved defaults (decision S1: renamed from the fork's file).
    pub const SETTINGS: &str = "settings.json";
    /// The fork's settings file name. The fork's `training_modpack.json` is its updater record
    /// (`{"update":{"last_update_version":…}}`), not the settings; the settings file of the fork,
    /// JSON despite the extension, is `training_modpack_menu.conf` (SPEC-core-04 §4).
    pub const LEGACY_SETTINGS: &str = "training_modpack_menu.conf";
    /// The install line, and the sparse event lines of a `diag` build.
    pub const EVENTS_LOG: &str = "events.log";
    pub const DIAGNOSTICS: &str = "practice-diagnostics.log";
    pub const DIAGNOSTICS_PREVIOUS: &str = "practice-diagnostics.previous.log";
    /// Gate lines (`[PracticeRecording]`, and the other lines a row waits for). Not rotated.
    pub const GATES: &str = "practice-gates.log";
    /// Decision N1: `developer_page=on` shows the Developer page.
    pub const DEVELOPER_INI: &str = "developer.ini";
    /// The import's marker and receipt (SPEC-core-08 §2).
    pub const MIGRATION: &str = "migration.json";
    /// The save-state slots (SPEC-state-01 §7): five per role, the fork's file name.
    pub const SAVE_STATES: &str = "save_states.json";
    /// The folder of the portable setup library (`<id>.json` per setup).
    pub const SETUPS_DIR: &str = "setups";
}

/// Full paths the diagnostics file uses (also built by `FixedPath::in_root`; a test keeps them equal).
pub const EVENTS_LOG_PATH: &str = "sd:/ultimate/TrainingRooms/events.log";
pub const DIAGNOSTICS_PATH: &str = "sd:/ultimate/TrainingRooms/practice-diagnostics.log";
pub const DIAGNOSTICS_PREVIOUS_PATH: &str = "sd:/ultimate/TrainingRooms/practice-diagnostics.previous.log";
pub const GATES_PATH: &str = "sd:/ultimate/TrainingRooms/practice-gates.log";

/// A path built in a fixed buffer. `N` is the capacity in bytes.
#[derive(Clone, Copy)]
pub struct FixedPath<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> FixedPath<N> {
    pub const fn new() -> Self {
        Self { buf: [0; N], len: 0 }
    }

    /// `dir/name`, or `None` when it does not fit.
    pub fn join(dir: &str, name: &str) -> Option<Self> {
        let mut path = Self::new();
        write!(path, "{dir}/{name}").ok()?;
        Some(path)
    }

    /// `name` inside the data folder.
    pub fn in_root(name: &str) -> Option<Self> {
        Self::join(ROOT, name)
    }

    pub fn as_str(&self) -> &str {
        // Only `write_str` fills the buffer, and it copies whole `&str`s, so this is valid UTF-8.
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

impl<const N: usize> Default for FixedPath<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Write for FixedPath<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > N {
            return Err(fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl<const N: usize> fmt::Debug for FixedPath<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

/// Buffer size for any path of this plugin.
pub const PATH_CAPACITY: usize = 128;
pub type Path = FixedPath<PATH_CAPACITY>;

/// How many renamed copies of a corrupt settings file are kept before the oldest number is reused.
pub const INVALID_COPIES_MAX: u32 = 99;

/// The file name of the `n`th kept copy of a corrupt settings file (decision S2), `n` from 1.
pub fn invalid_copy_name(n: u32) -> Option<FixedPath<48>> {
    let mut name = FixedPath::<48>::new();
    write!(name, "settings.invalid-{n}.json").ok()?;
    Some(name)
}

/// The first free `n` in `1..=INVALID_COPIES_MAX` for the kept copy, given a way to ask whether a
/// name exists. `None` when every number is taken (the file is then left where it is).
pub fn first_free_invalid_copy(mut exists: impl FnMut(&str) -> bool) -> Option<u32> {
    (1..=INVALID_COPIES_MAX).find(|n| invalid_copy_name(*n).is_some_and(|name| !exists(name.as_str())))
}

/// Which file the loader reads (decision S1: `settings.json`, "the loader also accepts the old name
/// once").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsSource {
    /// `settings.json`.
    Current,
    /// The fork's file name inside `ROOT` (a hand-copied file): read once, then the next save writes
    /// `settings.json`.
    LegacyName,
    /// Neither exists: factory settings.
    Missing,
}

pub const fn pick_settings_source(has_settings: bool, has_legacy_name: bool) -> SettingsSource {
    if has_settings {
        SettingsSource::Current
    } else if has_legacy_name {
        SettingsSource::LegacyName
    } else {
        SettingsSource::Missing
    }
}

/// The diagnostics file's size limit; at the limit it is renamed to the `.previous` file and a new
/// one starts (SPEC-core-01 §7: two files of at most 128 KiB).
pub const DIAGNOSTIC_LIMIT: u64 = 128 * 1024;

/// Whether appending `incoming` bytes to a file that already holds `bytes` rotates it first.
pub const fn should_rotate(bytes: u64, incoming: usize) -> bool {
    bytes + incoming as u64 > DIAGNOSTIC_LIMIT
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn paths_join_in_a_fixed_buffer() {
        assert_eq!(Path::in_root("settings.json").unwrap().as_str(), "sd:/ultimate/TrainingRooms/settings.json");
        assert!(FixedPath::<8>::join("sd:/ultimate", "x").is_none());
    }

    #[test]
    fn the_first_free_copy_number_is_used() {
        assert_eq!(first_free_invalid_copy(|name| name == "settings.invalid-1.json"), Some(2));
    }
    }
}