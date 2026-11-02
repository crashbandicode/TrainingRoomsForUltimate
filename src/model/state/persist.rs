//! Reading and writing `save_states.json` on the filesystem (SPEC-state-01 §7.1). The rule (review R1):
//! the active file is never renamed away before a durable replacement exists. A file that does not read
//! cleanly is *copied* aside once (`save_states.invalid-<n>.json`) and the active file stays where it is
//! until a Save replaces it atomically (`store::write_replace`: temp file, sync, replace; the old file
//! is retired only after the new one is in place, and stays on any failure).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::file::{Defaulted, SaveFile};
use crate::model::store::write_replace;

/// Largest file read: five slots per role with Mii identities is far below this.
pub const MAX_BYTES: u64 = 1024 * 1024;

/// How a load went.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Read as it stood.
    Read,
    /// No file: every record takes the defaults (not a fault).
    Missing,
    /// Unusable: the defaults are in use, a copy of the file was kept at the path given (if the copy
    /// worked), and the file itself is untouched.
    Defaulted { why: Defaulted, kept: Option<PathBuf> },
    /// The file could not be read (I/O error or too large); defaults in use, file untouched.
    Unreadable,
}

fn invalid_copy(path: &Path, n: u32) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!("{stem}.invalid-{n}.json"))
}

/// Read the file (lazily, the first time the slots are touched).
pub fn load(path: &Path) -> (SaveFile, Outcome) {
    let bytes = match fs::metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return (SaveFile::default(), Outcome::Missing),
        Err(_) => return (SaveFile::default(), Outcome::Unreadable),
        Ok(meta) if meta.len() > MAX_BYTES => return (SaveFile::default(), Outcome::Unreadable),
        Ok(_) => match fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => return (SaveFile::default(), Outcome::Unreadable),
        },
    };
    match SaveFile::parse(&bytes) {
        Ok(file) => (file, Outcome::Read),
        Err(why) => {
            // Keep one copy of what was there (a second boot over the same bytes reuses it); never
            // overwrite an earlier copy, never move the file.
            let mut kept = None;
            for copy in (1..=99).map(|n| invalid_copy(path, n)) {
                match fs::read(&copy) {
                    Ok(old) if old == bytes => {
                        kept = Some(copy);
                        break;
                    }
                    Ok(_) => continue,
                    Err(_) => {
                        kept = fs::write(&copy, &bytes).ok().map(|()| copy);
                        break;
                    }
                }
            }
            (SaveFile::default(), Outcome::Defaulted { why, kept })
        }
    }
}

/// Write the whole slot table. On `Err` the previous file is exactly as it was.
pub fn save(path: &Path, file: &SaveFile) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_replace(path, file.to_pretty().as_bytes())
}
