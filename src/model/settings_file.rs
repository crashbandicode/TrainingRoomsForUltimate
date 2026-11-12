//! Boot-time load of `settings.json` as one storage transaction (SPEC-core-04 §3.2, decision S2;
//! review R1 of package 1). The runtime keeps only the call; the host tests run this same code on a
//! temporary directory.
//!
//! A file that does not read cleanly (not JSON, not UTF-8, a bad field, a repeated key) is recovered
//! field by field by `settings::parse_bytes`. Recovery then has to leave the player's disk in a state
//! the next boot reads back to the same settings, so it is a transaction, in this order:
//!
//! 1. the untouched bytes of the original are written to the first free `settings.invalid-<n>.json`
//!    and read back to prove they are identical;
//! 2. only then are the sanitized complete settings (both snapshots, saved defaults, unknown keys
//!    kept) written over the active path, and for the fork's old file name the old file is retired
//!    afterwards.
//!
//! Both writes are `store::write_replace`, the Store's one transactional write: the bytes go to
//! `<path>.tmp`, are synced, then replace the target (through a `.bak` where rename cannot replace,
//! as on FAT); the old file stays if anything fails. A failure of step 2 removes the backup of step
//! 1 again, so the original stays active and untouched and nothing is left behind; it is logged and
//! retried at the next boot, and the recovered settings are still what this boot uses. A crash inside
//! the replace fallback leaves `settings.json.bak`, which the next load moves back before it reads
//! anything.

use std::fs;
use std::io;
use std::path::Path;

use super::settings::{self, Settings};
use super::storage::{self, SettingsSource};
use super::store::write_replace;

/// The temporary file `write_replace` uses for the active file.
#[cfg(test)]
pub const TEMP_NAME: &str = "settings.json.tmp";
/// What the replace fallback leaves behind if the process dies inside it.
pub const BACKUP_NAME: &str = "settings.json.bak";

/// What boot got from the settings files.
#[derive(Clone, Debug)]
pub struct Boot {
    pub settings: Settings,
    /// One line per thing worth a diagnostics line, in order.
    pub notes: Vec<String>,
    /// A current or legacy settings file was present, including one that could not be read.
    /// False only when both names were absent. The Training intro uses this for an existing install.
    pub file_existed: bool,
}

impl Boot {
    fn factory(notes: Vec<String>, file_existed: bool) -> Self {
        Self { settings: Settings::factory(), notes, file_existed }
    }
}

fn exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
}

/// Load the settings of the data folder `dir`, recovering a file that does not read cleanly.
pub fn load(dir: &Path) -> Boot {
    let mut notes = Vec::new();
    let active = dir.join(storage::file::SETTINGS);
    let legacy = dir.join(storage::file::LEGACY_SETTINGS);
    restore_orphaned_backup(dir, &active, &mut notes);
    let (source, path) = match storage::pick_settings_source(exists(&active), exists(&legacy)) {
        SettingsSource::Current => (SettingsSource::Current, active.clone()),
        SettingsSource::LegacyName => (SettingsSource::LegacyName, legacy),
        SettingsSource::Missing => return Boot::factory(notes, false),
    };
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            notes.push(format!("[TrainingRooms] settings unreadable: {error}; factory settings"));
            return Boot::factory(notes, true);
        }
    };
    let loaded = settings::parse_bytes(&bytes);
    let issue_start = notes.len();
    for issue in &loaded.issues {
        notes.push(format!("[TrainingRooms] settings issue at={} reason={}", issue.at, issue.reason));
    }
    if loaded.corrupt() && !recover(dir, &path, source, &bytes, &loaded.settings, &mut notes) {
        // A wholly invalid file has no usable values. Report its parse/recovery failure
        // once at boot; a denied cleanup must not become a panic or a retry loop.
        if let Some(issue) = loaded.issues.iter().find(|issue| issue.at.is_empty()) {
            if let Some(failure) = notes.pop() {
                notes.truncate(issue_start);
                notes.push(format!("{failure}; invalid settings: {}; using factory settings", issue.reason));
            }
        }
    }
    Boot { settings: loaded.settings, notes, file_existed: true }
}

/// A `settings.json.bak` without a `settings.json` is a crash inside the replace fallback: the backup
/// is the last good file.
fn restore_orphaned_backup(dir: &Path, active: &Path, notes: &mut Vec<String>) {
    let backup = dir.join(BACKUP_NAME);
    if !exists(active) && exists(&backup) {
        match fs::rename(&backup, active) {
            Ok(()) => notes.push("[TrainingRooms] settings.json restored from an interrupted replace".to_owned()),
            Err(error) => notes.push(format!("[TrainingRooms] could not restore {BACKUP_NAME}: {error}")),
        }
    }
}

/// Steps 1 and 2 of the module comment. Never panics, never leaves a half-made backup.
fn recover(dir: &Path, path: &Path, source: SettingsSource, original: &[u8], recovered: &Settings, notes: &mut Vec<String>) -> bool {
    let abandon = |notes: &mut Vec<String>, what: String| {
        notes.push(format!("[TrainingRooms] settings recovery abandoned: {what}; the original file stays active"));
        false
    };

    let Some(backup) = storage::first_free_invalid_copy(|name| exists(&dir.join(name)))
        .and_then(storage::invalid_copy_name)
        .map(|name| dir.join(name.as_str()))
    else {
        return abandon(notes, "no free settings.invalid-<n>.json".to_owned());
    };
    let backed_up = write_replace(&backup, original).and_then(|()| match fs::read(&backup) {
        Ok(copy) if copy == original => Ok(()),
        Ok(_) => Err(io::Error::other("the backup does not read back identical")),
        Err(error) => Err(error),
    });
    if let Err(error) = backed_up {
        let _ = fs::remove_file(&backup);
        return abandon(notes, format!("could not keep {}: {error}", file_name(&backup)));
    }

    // Publish: the sanitized settings become `settings.json`.
    if let Err(error) = write_replace(&dir.join(storage::file::SETTINGS), recovered.to_json().as_bytes()) {
        let _ = fs::remove_file(&backup);
        return abandon(notes, format!("could not write {}: {error}", storage::file::SETTINGS));
    }
    if source == SettingsSource::LegacyName {
        // The fork's old file is retired only now that `settings.json` holds the recovered settings.
        if let Err(error) = fs::remove_file(path) {
            notes.push(format!("[TrainingRooms] could not retire {}: {error}", file_name(path)));
        }
    }
    notes.push(format!("[TrainingRooms] settings recovered; the original is kept as {}", file_name(&backup)));
    true
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Write `settings` as the active file: the Store's transactional write (the menu's writer, a later
/// package, calls this).
#[cfg(test)]
pub fn save(dir: &Path, settings: &Settings) -> io::Result<()> {
    write_replace(&dir.join(storage::file::SETTINGS), settings.to_json().as_bytes())
}
