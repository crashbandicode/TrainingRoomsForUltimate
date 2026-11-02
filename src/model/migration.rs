//! The first-boot import (SPEC-core-08): what is copied from the earlier fork's
//! `sd:/ultimate/TrainingModpack/` into `sd:/ultimate/TrainingRooms/`, and when. This is the pure
//! plan: given the listing of the source folder, which destinations exist and whether the marker
//! exists, it says which files to copy, which to skip and why, and which part of the import each
//! belongs to. The runtime does the copying (fixed 64 KiB buffer, `.part` then rename).
//!
//! Only `migration.json` means "already imported": any other file in the new root, `developer.ini`
//! in particular (a pre-installed one is the operator's own), changes nothing here (operator rule
//! 2026-11-02).
//!
//! Not in this module yet: the item-on-load conversion (N6; it needs the save-state records of
//! SPEC-state-01/-04), the receipt's sha256 and the copy loop.

use super::storage::file;

/// The largest file the blocking part copies (SPEC-core-08 §4 rule 3).
pub const MAX_COPY_BYTES: u64 = 8 * 1024 * 1024;

/// The fork's settings file in its folder. The spec text names `training_modpack.json` here; that
/// file is the fork's updater record, and copying it as `settings.json` would load factory settings
/// over the player's own. The settings are in this one (JSON despite the extension).
pub const SETTINGS_SOURCE: &str = file::LEGACY_SETTINGS;

/// Which part of the import a file belongs to (SPEC-core-08 §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// Small files, finished before the settings load.
    Blocking,
    /// `replay_captures/`: its own thread, never joined by the boot.
    Background,
}

/// Where a source file goes, or `None` when it is not imported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Route<'a> {
    pub to: &'a str,
    pub part: Part,
    /// `replay_captures/` has no size cap (N8).
    pub capped: bool,
}

const SAME_NAME_FILES: [&str; 4] = ["button_config.ini", "replay_capture.ini", "save_states.json", "mii-presets.json"];
const SAME_NAME_DIRS: [&str; 2] = ["setups/", "macros/"];
const CAPTURES_DIR: &str = "replay_captures/";

/// The route of a file named relative to the source folder (`setups/a.json`).
pub fn route(rel: &str) -> Option<Route<'_>> {
    if rel == SETTINGS_SOURCE {
        return Some(Route { to: file::SETTINGS, part: Part::Blocking, capped: true });
    }
    if SAME_NAME_FILES.contains(&rel) {
        return Some(Route { to: rel, part: Part::Blocking, capped: true });
    }
    let inside = |dir: &str| rel.len() > dir.len() && rel.starts_with(dir) && !rel[dir.len()..].contains('/');
    if SAME_NAME_DIRS.iter().any(|dir| inside(dir)) {
        return Some(Route { to: rel, part: Part::Blocking, capped: true });
    }
    if inside(CAPTURES_DIR) {
        return Some(Route { to: rel, part: Part::Background, capped: false });
    }
    None
}

/// The temporary name a capture is written under before its rename.
pub fn part_name(to: &str, out: &mut impl core::fmt::Write) -> core::fmt::Result {
    write!(out, "{to}.part")
}

/// A file in the source folder.
#[derive(Clone, Copy, Debug)]
pub struct Source<'a> {
    pub rel: &'a str,
    pub bytes: u64,
}

/// Why a file was not copied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skip {
    /// The destination exists: a hand-copied folder or another version wins.
    Exists,
    TooLarge,
}

impl Skip {
    pub const fn word(self) -> &'static str {
        match self {
            Skip::Exists => "exists",
            Skip::TooLarge => "too_large",
        }
    }
}

/// One planned file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub part: Part,
    pub skip: Option<Skip>,
}

/// What the import does this boot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan<'a> {
    /// `migration.json` exists: nothing is imported (`migration=skipped reason=marker_present`).
    MarkerPresent,
    /// The source folder does not exist: write the marker with `result: none` and no files.
    NoSource,
    Copy(Vec<Action<'a>>),
}

/// Plan the import. `source` is `None` when the source folder does not exist.
pub fn plan<'a>(
    marker_exists: bool,
    source: Option<&[Source<'a>]>,
    destination_exists: impl Fn(&str) -> bool,
) -> Plan<'a> {
    if marker_exists {
        return Plan::MarkerPresent;
    }
    let Some(source) = source else {
        return Plan::NoSource;
    };
    let mut actions = Vec::new();
    for file in source {
        let Some(route) = route(file.rel) else {
            continue;
        };
        let skip = if destination_exists(route.to) {
            Some(Skip::Exists)
        } else if route.capped && file.bytes > MAX_COPY_BYTES {
            Some(Skip::TooLarge)
        } else {
            None
        };
        actions.push(Action { from: file.rel, to: route.to, part: route.part, skip });
    }
    Plan::Copy(actions)
}

/// What happened to one file of the plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Copied,
    Skipped(Skip),
    /// A read or write error (`reason=io:<code>`).
    IoError,
}

/// The receipt's `result` (SPEC-core-08 §4 rule 5): `partial` when any file failed, `none` when
/// nothing was copied and nothing failed, else `done`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportResult {
    Done,
    Partial,
    Nothing,
}

impl ImportResult {
    pub const fn word(self) -> &'static str {
        match self {
            ImportResult::Done => "done",
            ImportResult::Partial => "partial",
            ImportResult::Nothing => "none",
        }
    }
}

pub fn result(outcomes: &[Outcome]) -> ImportResult {
    if outcomes.contains(&Outcome::IoError) {
        ImportResult::Partial
    } else if outcomes.contains(&Outcome::Copied) {
        ImportResult::Done
    } else {
        ImportResult::Nothing
    }
}

/// The marker is written after the last file of both parts (SPEC-core-08 §5): the blocking part
/// never writes it, so a game that quits during the background copy repeats the import.
pub const fn marker_written_after(blocking_done: bool, background_done: bool) -> bool {
    blocking_done && background_done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marker_alone_blocks_a_rerun() {
        assert_eq!(plan(true, None, |_| true), Plan::MarkerPresent);
        assert_eq!(plan(false, None, |_| true), Plan::NoSource);
    }

    #[test]
    fn routes_rename_the_settings_file() {
        assert_eq!(route("training_modpack_menu.conf").map(|r| r.to), Some("settings.json"));
        assert!(route("training_modpack.json").is_none());
    }
}