//! The small INI files this package owns: `button_config.ini` (SPEC-input-04 §7, draft 9 N7) and the
//! file transaction every plugin-written small file follows (review R1: never lose the old content,
//! never leave a half file).
//!
//! Reading is fail-closed (review R4, SPEC-input-04 §7): a missing, unreadable, oversized or
//! invalid-UTF-8 file gives the defaults; a bad value falls back per slot (`MenuShortcuts::parse`).
//! Writing is a transaction: a temporary name in the same folder is written and synced, then renamed
//! over the file; where the filesystem's rename will not replace a file (FAT on Horizon) the old file
//! moves to `<name>.bak` first and goes back if the swap fails. A failed write leaves the old file as
//! it was and removes the temporary one.
//!
//! The helper is the same rule the shared `store` uses for its JSON entries; it is written here for
//! files that are not store entries and is to be replaced by the storage transaction of the review's
//! fix stream (r1fix) when that lands on the integration branch.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use super::menu::chord::Chord;
use super::menu_shortcuts::{self, MenuShortcuts, DEFAULT_INI, DEFAULT_SHORTCUTS};

/// What reading a small file gave.
#[derive(Debug, PartialEq, Eq)]
pub enum FileRead {
    Missing,
    /// The whole file, within the cap.
    Bytes(Vec<u8>),
    /// More than the cap, or an I/O error half way: nothing is used.
    Rejected(Reject),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reject {
    TooLarge,
    Unreadable,
    NotUtf8,
}

/// Read at most `max` bytes of `path`; one byte more than `max` is read to tell an oversized file from
/// one of exactly `max`.
pub fn read_capped(path: &Path, max: usize) -> FileRead {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return FileRead::Missing,
        Err(_) => return FileRead::Rejected(Reject::Unreadable),
    };
    let mut bytes = Vec::new();
    if file.take(max as u64 + 1).read_to_end(&mut bytes).is_err() {
        return FileRead::Rejected(Reject::Unreadable);
    }
    if bytes.len() > max {
        return FileRead::Rejected(Reject::TooLarge);
    }
    FileRead::Bytes(bytes)
}

/// The shortcuts a read gave: defaults for anything but a clean UTF-8 file.
pub fn shortcuts_from(read: &FileRead) -> MenuShortcuts {
    match read {
        FileRead::Bytes(bytes) => match std::str::from_utf8(bytes) {
            Ok(text) => MenuShortcuts::parse(text),
            Err(_) => DEFAULT_SHORTCUTS,
        },
        _ => DEFAULT_SHORTCUTS,
    }
}

/// Read `button_config.ini` at `path` (SPEC-input-04 §7). The file is created with the default text
/// when it is absent (the only write at boot); a failure to create it is returned for the diagnostics
/// and the defaults are used.
pub fn load_shortcuts(path: &Path) -> (MenuShortcuts, Option<io::Error>) {
    let read = read_capped(path, menu_shortcuts::MAX_BYTES);
    if read == FileRead::Missing {
        let created = write_atomic(path, DEFAULT_INI.as_bytes()).err();
        return (DEFAULT_SHORTCUTS, created);
    }
    (shortcuts_from(&read), None)
}

/// The N7 write: the two slots replaced in the file's own text (comments and unknown lines kept), as
/// a transaction. `Ok` means the new text is on the file; on `Err` the old file is untouched and the
/// caller keeps the change in memory for the session (SPEC-input-04 question 6 (c)).
pub fn write_shortcuts(path: &Path, shortcuts: MenuShortcuts) -> io::Result<()> {
    let original = match read_capped(path, menu_shortcuts::MAX_BYTES) {
        FileRead::Bytes(bytes) => String::from_utf8(bytes).unwrap_or_else(|_| DEFAULT_INI.to_owned()),
        _ => DEFAULT_INI.to_owned(),
    };
    write_atomic(path, shortcuts.to_ini_text(&original).as_bytes())
}

/// Why the chord editor refuses a chord (SPEC-input-04 question 6 (b), the recommendation: refuse the
/// permanent chord and a duplicate slot; allow overlap with the six chord rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditRefusal {
    /// The chord equals the permanent `B + D-pad Up`.
    Permanent,
    /// The chord equals the other shortcut slot.
    SameAsOther,
}

impl EditRefusal {
    pub const fn text(self) -> &'static str {
        match self {
            EditRefusal::Permanent => "B + D-pad Up always opens the menu",
            EditRefusal::SameAsOther => "The other shortcut already uses this",
        }
    }
}

/// Check an edit of slot `slot` to `chord` against the rules above. An empty chord disables the slot
/// and is always allowed.
pub fn check_edit(current: MenuShortcuts, slot: usize, chord: Chord) -> Result<MenuShortcuts, EditRefusal> {
    if !chord.is_empty() {
        if chord == menu_shortcuts::DEFAULT_OPEN_MENU_CONFIG {
            return Err(EditRefusal::Permanent);
        }
        if current.slots()[1 - (slot & 1)] == chord {
            return Err(EditRefusal::SameAsOther);
        }
    }
    Ok(current.with_slot(slot & 1, chord))
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut os: OsString = path.as_os_str().to_owned();
    os.push(suffix);
    PathBuf::from(os)
}

/// Write `bytes` to `path` as a transaction (see the module doc): the Store's one write, `store::write_replace`.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    super::store::write_replace(path, bytes)
}

/// A `.bak` without its file means a crash inside the swap above: put it back.
pub fn restore_orphaned_backup(path: &Path) {
    let backup = with_suffix(path, ".bak");
    if backup.exists() && !path.exists() {
        let _ = fs::rename(backup, path);
    }
}
