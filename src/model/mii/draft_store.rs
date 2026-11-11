//! Bounded, mod-owned preset persistence. Native Mii saves are never a destination.
//! One menu/filesystem owner must serialize access to this directory.
use std::{fs, io::{self, Read, Write}, path::{Path, PathBuf}};
use {crate::engine::mii::draft::{Draft, Library, Saved}, crate::engine::mii::inventory::Snapshot};

pub const MAX_FILE_BYTES: u64 = 1024 * 1024;
const FILE: &str = "mii-presets.json";

pub struct Store {
    dir: PathBuf,
    library: Library,
}

impl Store {
    /// A missing file is an empty library. Invalid/oversized files are errors and
    /// remain untouched. Recover a validated backup only if its destination is absent.
    pub fn open(mod_directory: impl Into<PathBuf>) -> io::Result<Self> {
        let dir = mod_directory.into();
        let dest = dir.join(FILE);
        let library = match read_library(&dest) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let backup = dir.join(format!("{FILE}.bak"));
                match read_library(&backup) {
                    Ok(value) => { fs::rename(&backup, &dest)?; value }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Library::default(),
                    Err(error) => return Err(error),
                }
            }
            Err(error) => return Err(error),
        };
        Ok(Self { dir, library })
    }

    pub fn library(&self) -> &Library { &self.library }

    /// Commit on disk before publishing the new in-memory library. Duplicate
    /// configurations preserve their first name and do not write any file.
    pub fn save(&mut self, draft: &Draft, name: &str, native: &Snapshot) -> io::Result<Saved> {
        let mut next = self.library.clone();
        let outcome = next.save(draft, name, native)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        self.commit(next, outcome)
    }

    pub fn import(&mut self, configuration: crate::engine::mii::lifetime::Configuration, name: &str, native: &Snapshot) -> io::Result<Saved> {
        let mut next = self.library.clone();
        let outcome = next.import(configuration, name, native)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        self.commit(next, outcome)
    }

    fn commit(&mut self, next: Library, outcome: Saved) -> io::Result<Saved> {
        if !matches!(outcome, Saved::Created(_)) { return Ok(outcome); }
        let bytes = serde_json::to_vec(&next).map_err(invalid_data)?;
        if bytes.len() as u64 > MAX_FILE_BYTES { return Err(invalid_data("Mii preset file is too large")); }
        fs::create_dir_all(&self.dir)?;
        let tmp = self.dir.join(format!("{FILE}.tmp"));
        let dest = self.dir.join(FILE);
        // This scratch file belongs only to this mod store. A previous interrupted
        // scratch write was never committed and is not a valid library to recover.
        let mut file = fs::File::create(&tmp)?;
        let written = file.write_all(&bytes).and_then(|()| file.sync_all());
        drop(file);
        let result = written.and_then(|()| replace_file(&tmp, &dest));
        if result.is_err() { let _ = fs::remove_file(&tmp); }
        result?;
        self.library = next;
        Ok(outcome)
    }
}

fn invalid_data(error: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

fn read_library(path: &Path) -> io::Result<Library> {
    // Pinned Skyline open() loses NotFound; metadata() preserves its typed result.
    if fs::metadata(path)?.len() > MAX_FILE_BYTES { return Err(invalid_data("Mii preset file is too large")); }
    let file = fs::File::open(path)?;
    // Bound the read itself as well as metadata; file growth cannot bypass the cap.
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES { return Err(invalid_data("Mii preset file is too large")); }
    serde_json::from_slice(&bytes).map_err(invalid_data)
}

/// Same rename/backup strategy used by the macro library for Horizon filesystems
/// whose rename cannot replace a file. An interrupted fallback keeps the old .bak.
fn replace_file(tmp: &Path, dest: &Path) -> io::Result<()> {
    let first = match fs::rename(tmp, dest) { Ok(()) => return Ok(()), Err(error) => error };
    if !dest.try_exists()? { return Err(first); }
    replace_via_backup(tmp, dest)
}

fn replace_via_backup(tmp: &Path, dest: &Path) -> io::Result<()> {
    let backup = dest.with_file_name(format!("{FILE}.bak"));
    match fs::remove_file(&backup) {
        Ok(()) => {},
        Err(error) if error.kind() == io::ErrorKind::NotFound => {},
        Err(error) => return Err(error),
    }
    fs::rename(dest, &backup)?;
    if let Err(error) = fs::rename(tmp, dest) {
        // If restoration itself fails, leave the old backup for validated open().
        let _ = fs::rename(&backup, dest);
        return Err(error);
    }
    let _ = fs::remove_file(&backup);
    Ok(())
}

