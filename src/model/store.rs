//! Directory-backed record library: one compact JSON file per entry, named
//! `<id>.json`, in a caller-chosen directory. There is no entry-count limit.
//!
//! [`Store`] is generic over the stored [`Record`]. The portable practice setup
//! (`state::setup`) and, later, the named-macro library are instantiations that
//! reuse the id, size, version and atomic-write rules below instead of copying
//! them.
//!
//! Carried from the fork's `training_mod_macros/src/library.rs` (stage m5o,
//! see CARRY-OVER.md). Clean-room changes: the fork's `StoreError` became this
//! file's own [`StoreError`] (only the variants the store itself raises), the
//! macro-specific `Library` / `MacroEntry` / `Record for MacroRecording` were
//! left out (package 4), the `Header` derive in `decode` is a hand-read of
//! `schema_version` (this crate has no serde derive), and the id fallback slug
//! is `entry`.
//!
//! * Ids are stable and path-safe (`[a-z0-9_-]`, at most [`MAX_ID_CHARS`]). The
//!   id is the file stem; it never changes when the record is overwritten. A
//!   record whose files people share accepts other safe names too
//!   ([`Record::accepts_id`]); the ids the store makes are always slugs.
//! * Hidden files (a leading `.`) are never records.
//! * Updates write `<id>.json.tmp`, sync it, then rename it over the target, so
//!   a reader or an interrupted write never sees a partial file. Where rename
//!   cannot replace an existing file (FAT on Horizon is the concern), the old
//!   file is moved to `<id>.json.bak` first and put back if the swap fails.
//! * `list` never deletes or rewrites anything except restoring an orphaned
//!   `.bak` (left only by a crash inside that swap).

use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt, fs,
    io::{self, BufWriter, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::{atomic::{AtomicU64, Ordering}, Mutex, MutexGuard, TryLockError},
};

use serde::{de::DeserializeOwned, Serialize};

/// What the store itself can fail with: I/O, id, size, parse and version. A record's own error type
/// converts these (`Record::Error: From<StoreError>`).
#[derive(Debug)]
pub enum StoreError {
    /// The file's `schema_version` is readable but not one the record accepts.
    UnsupportedVersion { found: u32 },
    FileTooLarge { bytes: u64, max: u64 },
    InvalidId(String),
    NotFound(String),
    /// Every candidate id derived from the name is already taken.
    NoFreeId,
    Parse(serde_json::Error),
    Io(io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion { found } => write!(f, "unsupported schema version {found}"),
            Self::FileTooLarge { bytes, max } => write!(f, "file is {bytes} bytes (maximum {max})"),
            Self::InvalidId(id) => write!(f, "invalid id {id:?}"),
            Self::NotFound(id) => write!(f, "no entry with id {id:?}"),
            Self::NoFreeId => write!(f, "no free id for this name"),
            Self::Parse(err) => write!(f, "malformed file: {err}"),
            Self::Io(err) => write!(f, "file error: {err}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(err) => Some(err),
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse(err)
    }
}

/// Longest id accepted, in characters.
pub const MAX_ID_CHARS: usize = 48;
/// The file naming a store's selected record (`Store::select`).
pub const SELECTED: &str = "selected.txt";
/// Default largest file read: 8 MiB. Other record types set their own bound through
/// [`Record::MAX_FILE_BYTES`].
pub const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;

const SLUG_CHARS: usize = 40;
const MAX_SUFFIX: u32 = 9_999;

/// A value type a [`Store`] keeps as one JSON file per entry.
///
/// The store owns ids, atomic replacement, size bounds and the
/// version-versus-corruption distinction. The record owns its own validation,
/// including rejecting a `schema_version` outside [`Record::SCHEMA_VERSIONS`]:
/// the store's header check only classifies files that do not parse at all.
pub trait Record: Serialize + DeserializeOwned {
    /// Error reported by a store of this record. The store's own failures
    /// (`StoreError`: I/O, id, size, parse, version) must convert into it.
    type Error: From<StoreError> + fmt::Display;
    /// Listing row built from one validated record.
    type Summary;
    /// Schema versions this reader accepts. A file whose `schema_version`
    /// header is readable but outside this set is reported as
    /// `UnsupportedVersion` rather than as corruption.
    const SCHEMA_VERSIONS: &'static [u32];
    /// Largest file `load` and `list` will read.
    const MAX_FILE_BYTES: u64;
    /// Serialize to the temporary file instead of building a JSON-sized save buffer.
    const STREAM_WRITE: bool = false;
    /// A record with a file of its own beside the record's (a macro's positions): before the record is written under
    /// `id`, after it is in place, and after it is deleted. Default: nothing.
    fn before_write(&self, _dir: &Path, _id: &str) {}
    fn after_write(&self, _dir: &Path, _id: &str) {}
    fn after_delete(_dir: &Path, _id: &str) {}
    /// Everything the store requires before it writes or accepts a file.
    fn validate(&self) -> Result<(), Self::Error>;
    /// Display name: its slug seeds new ids and it orders listings.
    fn name(&self) -> &str;
    /// Listing row for the record stored under `id`.
    fn summary(&self, id: &str) -> Self::Summary;
    /// Whether `id` (a file name without `.json`) may name a record of this store. Default: [`MAX_ID_CHARS`] of
    /// `[a-z0-9_-]`, the ids the store itself makes. A store whose files people share may accept more.
    fn accepts_id(id: &str) -> bool {
        default_id(id)
    }
    /// Called with the file's id after a record was read and before it is validated: a record may take what its
    /// file left out from the file's name. Default: nothing.
    fn adopt(&mut self, _id: &str) {}
    /// A file that belongs to another record type in the same directory. `list` omits it instead of
    /// reporting it as corrupt. Default: every readable file is this record's.
    fn owns(_bytes: &[u8]) -> bool {
        true
    }
}

/// A `.json` file that could not be used, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedFile {
    pub file: String,
    pub reason: String,
}

/// Result of scanning a store: good entries plus the files that were skipped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing<S> {
    /// Sorted by record name (case-insensitive), then id.
    pub entries: Vec<S>,
    pub skipped: Vec<SkippedFile>,
}

// Not derived: a derive would demand `S: Default`, which summaries do not have.
impl<S> Default for Listing<S> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            skipped: Vec::new(),
        }
    }
}

/// A [`Store::list`] in progress: the directory's names, read and decoded one per [`Self::step`].
pub struct ListCursor<R: Record> {
    store: Store<R>,
    names: std::vec::IntoIter<String>,
    entries: Vec<(String, String, R::Summary)>,
    skipped: Vec<SkippedFile>,
}

impl<R: Record> ListCursor<R> {
    /// Read and decode the next file (none is read for a name that is not a record). `false` when none is left.
    pub fn step(&mut self) -> bool {
        let Some(name) = self.names.next() else { return false };
        let Some(id) = name.strip_suffix(".json") else {
            return true;
        };
        let path = self.store.file(id, "json");
        let data = match check_id::<R>(id).and_then(|()| read_bounded(&path, R::MAX_FILE_BYTES)) {
            Ok(data) => data,
            Err(err) => {
                let reason = lift::<R>(err).to_string();
                self.skipped.push(SkippedFile { file: name, reason });
                return true;
            }
        };
        if !R::owns(&data) { return true; }
        match decode::<R>(id, &data) {
            Ok(rec) => self.entries.push((rec.name().to_lowercase(), id.to_owned(), rec.summary(id))),
            Err(err) => self.skipped.push(SkippedFile { file: name, reason: err.to_string() }),
        }
        true
    }

    /// The listing, sorted as [`Store::list`] sorts it (the names not stepped yet are not in it).
    pub fn finish(mut self) -> Listing<R::Summary> {
        self.entries.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        Listing { entries: self.entries.into_iter().map(|(_, _, summary)| summary).collect(), skipped: self.skipped }
    }
}

/// A record store rooted at a caller-supplied directory.
pub struct Store<R> {
    dir: PathBuf,
    record: PhantomData<fn() -> R>,
}

impl<R> Clone for Store<R> {
    fn clone(&self) -> Self {
        Self {
            dir: self.dir.clone(),
            record: PhantomData,
        }
    }
}

impl<R> fmt::Debug for Store<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Store").field("dir", &self.dir).finish()
    }
}

impl<R: Record> Store<R> {
    /// Does no I/O; the directory is created by the first save.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            record: PhantomData,
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Lists every valid record. A missing directory is an empty store.
    /// Corrupt or invalid files are reported in `skipped` and left untouched.
    /// Files are decoded one at a time; only each summary is retained.
    pub fn list(&self) -> Result<Listing<R::Summary>, R::Error> {
        #[cfg(test)]
        if let Some(action) = BEFORE_LIST.with(|next| next.borrow_mut().take()) {
            action();
        }
        let mut cursor = self.list_cursor()?;
        while cursor.step() {}
        Ok(cursor.finish())
    }

    /// [`Self::list`] a file at a time (the menu worker's catalog build reads one file per pass): the directory is read
    /// here, each [`ListCursor::step`] reads and decodes one file, and [`ListCursor::finish`] sorts.
    pub fn list_cursor(&self) -> Result<ListCursor<R>, R::Error> {
        let mut skipped = Vec::new();
        #[cfg(test)]
        count_directory_read();
        let read_dir = match fs::read_dir(&self.dir) {
            Ok(read_dir) => read_dir,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(ListCursor { store: self.clone(), names: Vec::new().into_iter(), entries: Vec::new(), skipped })
            }
            Err(err) => return Err(lift::<R>(err.into())),
        };
        let mut names = Vec::new();
        for item in read_dir {
            match item {
                // A hidden file is never a record (a card written from a Mac holds `._<name>` resource forks).
                Ok(item) if item.path().is_file() && !item.file_name().to_string_lossy().starts_with('.') => {
                    names.push(item.file_name().to_string_lossy().into_owned())
                }
                Ok(_) => {}
                Err(err) => skipped.push(SkippedFile {
                    file: String::new(),
                    reason: format!("unreadable directory entry: {err}"),
                }),
            }
        }
        self.restore_orphaned_backups(&mut names);
        names.sort_unstable();
        Ok(ListCursor { store: self.clone(), names: names.into_iter(), entries: Vec::new(), skipped })
    }

    /// Read once and validate the owning type. Invalid data is stable for the
    /// catalog signature; an I/O error leaves the entry eligible to retry.
    pub fn load_with_sibling<S: Record>(&self, id: &str) -> io::Result<Option<Result<R::Summary, S>>> {
        if !R::accepts_id(id) && !S::accepts_id(id) {
            return Ok(None);
        }
        let Some(data) = read_for_catalog(&self.file(id, "json"), R::MAX_FILE_BYTES.max(S::MAX_FILE_BYTES))? else {
            return Ok(None);
        };
        Ok(if R::accepts_id(id) && data.len() as u64 <= R::MAX_FILE_BYTES && R::owns(&data) {
            decode::<R>(id, &data).ok().map(|r| Ok(r.summary(id)))
        } else if S::accepts_id(id) && data.len() as u64 <= S::MAX_FILE_BYTES {
            decode::<S>(id, &data).ok().map(Err)
        } else {
            None
        })
    }

    pub fn load_for_catalog(&self, id: &str) -> io::Result<Option<R>> {
        if !R::accepts_id(id) {
            return Ok(None);
        }
        Ok(read_for_catalog(&self.file(id, "json"), R::MAX_FILE_BYTES)?
            .and_then(|data| decode::<R>(id, &data).ok()))
    }

    /// Loads and fully validates the record with this id.
    pub fn load(&self, id: &str) -> Result<R, R::Error> {
        check_id::<R>(id).map_err(lift::<R>)?;
        let data = read_bounded(&self.file(id, "json"), R::MAX_FILE_BYTES)
            .map_err(|err| match err {
                StoreError::Io(inner) if inner.kind() == io::ErrorKind::NotFound => {
                    StoreError::NotFound(id.to_owned())
                }
                other => other,
            })
            .map_err(lift::<R>)?;
        decode::<R>(id, &data)
    }

    /// Saves as a brand-new file and returns its id. The id comes from the
    /// name (`my-drill`, then `my-drill-2`, ...) and never matches an existing
    /// file, good or corrupt, so nothing already on disk can be replaced.
    pub fn save_new(&self, rec: &R) -> Result<String, R::Error> {
        let bytes = encode(rec)?;
        fs::create_dir_all(&self.dir).map_err(|err| lift::<R>(err.into()))?;
        let _guard = catalog_folder(&self.dir).and_then(|(root, _)| recording_write_guard(root));
        let id = self.free_id(rec.name()).map_err(lift::<R>)?;
        self.write_atomic(&id, rec, bytes)?;
        Ok(id)
    }

    /// Replaces the existing record `id` with `rec`. The file must already
    /// exist; on any failure the previous contents are left intact.
    pub fn overwrite(&self, id: &str, rec: &R) -> Result<(), R::Error> {
        check_id::<R>(id).map_err(lift::<R>)?;
        let bytes = encode(rec)?;
        let _guard = catalog_folder(&self.dir).and_then(|(root, _)| recording_write_guard(root));
        let exists = self
            .file(id, "json")
            .try_exists()
            .map_err(|err| lift::<R>(err.into()))?;
        if !exists {
            return Err(lift::<R>(StoreError::NotFound(id.to_owned())));
        }
        self.write_atomic(id, rec, bytes)
    }

    /// Removes the record `id` (and a leftover backup of it).
    pub fn delete(&self, id: &str) -> Result<(), R::Error> {
        check_id::<R>(id).map_err(lift::<R>)?;
        let _guard = catalog_folder(&self.dir).and_then(|(root, _)| recording_write_guard(root));
        match fs::remove_file(self.file(id, "json")) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Err(lift::<R>(StoreError::NotFound(id.to_owned()))),
            Err(err) => return Err(lift::<R>(err.into())),
        }
        let _ = fs::remove_file(self.file(id, "json.bak"));
        R::after_delete(&self.dir, id);
        note_catalog_write(&self.dir, id, None);
        Ok(())
    }

    /// The id in `selected.txt`, when it names a valid id (the file may name a record since deleted).
    pub fn selected(&self) -> Option<String> {
        let text = fs::read_to_string(self.dir.join(SELECTED)).ok()?;
        let id = text.trim();
        check_id::<R>(id).is_ok().then(|| id.to_owned())
    }

    /// Writes `selected.txt` atomically; `None` removes it.
    pub fn select(&self, id: Option<&str>) -> Result<(), R::Error> {
        let path = self.dir.join(SELECTED);
        let Some(id) = id else {
            return match fs::remove_file(&path) {
                Err(err) if err.kind() != io::ErrorKind::NotFound => Err(lift::<R>(err.into())),
                _ => Ok(()),
            };
        };
        check_id::<R>(id).map_err(lift::<R>)?;
        fs::create_dir_all(&self.dir).map_err(|err| lift::<R>(err.into()))?;
        write_replace(&path, id.as_bytes()).map_err(|err| lift::<R>(err.into()))
    }

    fn file(&self, id: &str, extension: &str) -> PathBuf {
        self.dir.join(format!("{id}.{extension}"))
    }

    fn free_id(&self, name: &str) -> Result<String, StoreError> {
        let base = slug(name);
        for n in 1..=MAX_SUFFIX {
            let id = if n == 1 {
                base.clone()
            } else {
                format!("{base}-{n}")
            };
            if !self.file(&id, "json").try_exists()? && !self.file(&id, "json.bak").try_exists()? {
                return Ok(id);
            }
        }
        Err(StoreError::NoFreeId)
    }

    fn write_atomic(&self, id: &str, rec: &R, bytes: Option<Vec<u8>>) -> Result<(), R::Error> {
        rec.before_write(&self.dir, id);
        let path = self.file(id, "json");
        let size = if let Some(bytes) = bytes {
            write_replace(&path, &bytes).map_err(|e| lift::<R>(e.into()))?;
            bytes.len() as u64
        } else if R::STREAM_WRITE {
            replace_with(&path, |tmp| {
                let mut writer = BufWriter::new(fs::File::create(tmp)?);
                serde_json::to_writer(&mut writer, rec)?;
                writer.flush()?;
                writer.get_ref().sync_all()?;
                Ok(writer.get_ref().metadata()?.len())
            }).map_err(|e| lift::<R>(e.into()))?
        } else { unreachable!("non-streaming records always have encoded bytes") };
        rec.after_write(&self.dir, id);
        note_catalog_write(&self.dir, id, Some(size));
        Ok(())
    }

    /// A `.json.bak` without its `.json` means a crash inside the fallback swap;
    /// the backup is the last good copy, so move it back.
    fn restore_orphaned_backups(&self, names: &mut Vec<String>) {
        let orphans: Vec<String> = names
            .iter()
            .filter_map(|name| name.strip_suffix(".json.bak"))
            .filter(|id| check_id::<R>(id).is_ok() && !names.contains(&format!("{id}.json")))
            .map(str::to_owned)
            .collect();
        if orphans.is_empty() {
            return;
        }
        let _guard = catalog_folder(&self.dir).and_then(|(root, _)| recording_write_guard(root));
        for id in orphans {
            let target = self.file(&id, "json");
            if matches!(target.try_exists(), Ok(false)) {
                let _ = fs::rename(self.file(&id, "json.bak"), &target);
            }
            if target.is_file() {
                names.push(format!("{id}.json"));
            }
        }
    }
}

/// A resumable names-and-sizes scan. Ignored entries count toward the same
/// budget as visible records, so a folder of non-records also yields to input.
pub struct CatalogListing {
    entries: Option<fs::ReadDir>,
    files: Option<BTreeMap<String, u64>>,
}

impl CatalogListing {
    pub fn open(path: &Path) -> Self {
        #[cfg(test)]
        count_directory_read();
        match fs::read_dir(path) {
            Ok(entries) => Self { entries: Some(entries), files: Some(BTreeMap::new()) },
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                Self { entries: None, files: Some(BTreeMap::new()) }
            }
            Err(_) => Self { entries: None, files: None },
        }
    }

    /// Advance at most four directory entries. The caller holds the read guard.
    pub fn step(&mut self, recover: Option<fn(&str) -> bool>) -> bool {
        for _ in 0..4 {
            let Some(entry) = self.entries.as_mut().and_then(Iterator::next) else { return false };
            if entry.and_then(|entry| self.add(entry, recover)).is_err() {
                self.files = None;
                return false;
            }
        }
        true
    }

    fn add(&mut self, entry: fs::DirEntry, recover: Option<fn(&str) -> bool>) -> io::Result<()> {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            return Ok(());
        }
        let (id, path) = if let Some(id) = name.strip_suffix(".json") {
            (id, entry.path())
        } else if let Some(id) = name.strip_suffix(".json.bak").filter(|id| recover.is_some_and(|accepts| accepts(id))) {
            let backup = entry.path();
            let target = backup.with_extension("");
            if !target.try_exists()? {
                fs::rename(backup, &target)?;
            }
            (id, target)
        } else {
            return Ok(());
        };
        #[cfg(test)]
        count_catalog_metadata();
        match fs::metadata(path) {
            Ok(meta) if meta.is_file() => {
                self.files.as_mut().unwrap().insert(id.to_owned(), meta.len());
            }
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
        Ok(())
    }

    pub fn finish(self) -> Option<BTreeMap<String, u64>> {
        self.files
    }
}

fn read_for_catalog(path: &Path, max: u64) -> io::Result<Option<Vec<u8>>> {
    match read_bounded(path, max) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(StoreError::Io(err)) => Err(err),
        Err(_) => Ok(None),
    }
}

/// Lift a store failure into the record's error type.
fn lift<R: Record>(err: StoreError) -> R::Error {
    err.into()
}

/// The two record types sharing `macros/` need only the tag to identify which decoder owns a file.
pub(crate) fn is_chain_document(bytes: &[u8]) -> bool {
    #[derive(serde::Deserialize)]
    struct Header { kind: Option<String> }
    serde_json::from_slice::<Header>(bytes).is_ok_and(|header| header.kind.as_deref() == Some("chain"))
}

fn encode<R: Record>(rec: &R) -> Result<Option<Vec<u8>>, R::Error> {
    rec.validate()?;
    if R::STREAM_WRITE { Ok(None) } else { serde_json::to_vec(rec).map(Some).map_err(|err| lift::<R>(err.into())) }
}

#[cfg(test)]
thread_local! {
    static FILE_READS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    static INDEX_READS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    static DIRECTORY_READS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    static METADATA_READS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    static FAIL_PAYLOAD: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    static BEFORE_LIST: core::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { core::cell::RefCell::new(None) };
}

/// Files this thread has read through a store (tests: a check that must read none).
#[cfg(test)]
pub(crate) fn file_reads() -> usize {
    FILE_READS.with(|reads| reads.get())
}

#[cfg(test)]
pub(crate) fn count_index_read() {
    INDEX_READS.with(|reads| reads.set(reads.get() + 1));
}

#[cfg(test)]
pub fn index_reads() -> usize {
    INDEX_READS.with(core::cell::Cell::get)
}

#[cfg(test)]
fn count_directory_read() {
    DIRECTORY_READS.with(|reads| reads.set(reads.get() + 1));
}

#[cfg(test)]
pub fn directory_reads() -> usize {
    DIRECTORY_READS.with(core::cell::Cell::get)
}

#[cfg(test)]
pub(crate) fn before_next_store_list(action: impl FnOnce() + 'static) {
    BEFORE_LIST.with(|next| next.replace(Some(Box::new(action))));
}

#[cfg(test)]
pub(crate) fn fail_next_payload_read() {
    FAIL_PAYLOAD.with(|fail| fail.set(true));
}

#[cfg(test)]
fn count_catalog_metadata() {
    METADATA_READS.with(|reads| reads.set(reads.get() + 1));
}

#[cfg(test)]
pub(crate) fn catalog_metadata_reads() -> usize {
    METADATA_READS.with(core::cell::Cell::get)
}

/// Reads a whole file after checking its size against `max`.
fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>, StoreError> {
    payload_read()?;
    check_len(fs::metadata(path)?.len(), max)?;
    Ok(fs::read(path)?)
}

fn payload_read() -> Result<(), StoreError> {
    #[cfg(test)]
    FILE_READS.with(|reads| reads.set(reads.get() + 1));
    #[cfg(test)]
    if FAIL_PAYLOAD.with(|fail| fail.replace(false)) {
        return Err(io::Error::from(io::ErrorKind::Interrupted).into());
    }
    Ok(())
}

fn check_len(bytes: u64, max: u64) -> Result<(), StoreError> {
    if bytes > max {
        return Err(StoreError::FileTooLarge { bytes, max });
    }
    Ok(())
}

fn decode<R: Record>(id: &str, data: &[u8]) -> Result<R, R::Error> {
    match serde_json::from_slice::<R>(data) {
        Ok(mut rec) => {
            rec.adopt(id);
            rec.validate()?;
            Ok(rec)
        }
        Err(err) => {
            // A newer schema may not fit this layout; call that a version
            // problem rather than corruption.
            #[derive(serde::Deserialize)]
            struct Header { schema_version: u32 }
            let header = serde_json::from_slice::<Header>(data).ok().map(|h| h.schema_version);
            match header {
                Some(found) if !R::SCHEMA_VERSIONS.contains(&found) => {
                    Err(lift::<R>(StoreError::UnsupportedVersion { found }))
                }
                _ => Err(lift::<R>(err.into())),
            }
        }
    }
}

/// Write `bytes` to `path` so a reader or an interrupted write never sees a partial file: the bytes go
/// to `<path>.tmp`, are synced, then replace the target (through a `.bak` where rename cannot
/// replace). The old file is retired only after the new one is in place; on any failure it stays.
/// Added for package 3's `save_states.json` (same rule as the store's own writes).
pub fn write_replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    replace_with(path, |tmp| write_synced(tmp, bytes))
}

pub(crate) fn replace_with<T>(path: &Path, write: impl FnOnce(&Path) -> io::Result<T>) -> io::Result<T> {
    let tmp = with_suffix(path, ".tmp");
    let result = write(&tmp).and_then(|value| replace_file(&tmp, path).map(|()| value));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn replace_file(tmp: &Path, dest: &Path) -> io::Result<()> {
    let first = match fs::rename(tmp, dest) {
        Ok(()) => return Ok(()),
        Err(err) => err,
    };
    if dest.try_exists().unwrap_or(false) {
        replace_via_backup(tmp, dest)
    } else {
        Err(first)
    }
}

/// Fallback for filesystems whose rename will not replace an existing file.
fn replace_via_backup(tmp: &Path, dest: &Path) -> io::Result<()> {
    let backup = with_suffix(dest, ".bak");
    let _ = fs::remove_file(&backup);
    fs::rename(dest, &backup)?;
    if let Err(err) = fs::rename(tmp, dest) {
        let _ = fs::rename(&backup, dest);
        return Err(err);
    }
    let _ = fs::remove_file(&backup);
    Ok(())
}

pub(crate) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut os: OsString = path.as_os_str().to_owned();
    os.push(suffix);
    PathBuf::from(os)
}

/// `make(n)` with the lowest `n` from 1 that no name in `taken` uses, ignoring case: "Combo N", "Layout N · Battlefield".
pub fn lowest_free<'a>(make: impl Fn(u32) -> String, taken: impl Iterator<Item = &'a str> + Clone) -> String {
    (1..).map(make).find(|name| !taken.clone().any(|t| t.eq_ignore_ascii_case(name))).unwrap_or_default()
}

/// The ids the store itself makes: [`MAX_ID_CHARS`] of `[a-z0-9_-]` (`slug`, with a `-N` suffix).
pub fn default_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_CHARS
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

fn check_id<R: Record>(id: &str) -> Result<(), StoreError> {
    if R::accepts_id(id) {
        Ok(())
    } else {
        Err(StoreError::InvalidId(id.to_owned()))
    }
}

/// Lowercase ASCII slug of a display name, `entry` if nothing survives.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= SLUG_CHARS {
            break;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("entry");
    }
    out
}

// Recording catalog invalidation happens at the committed Store operation,
// including the standalone macro and combo editors. Other stores are independent.
pub const CATALOG_DIRS: [&str; 4] = ["macros", "saves", "combos", "sequences"];
pub type CatalogSignatures = BTreeMap<(usize, String), Option<u64>>;
static CATALOG_WRITES: Mutex<BTreeMap<PathBuf, CatalogSignatures>> = Mutex::new(BTreeMap::new());
static RECORDING_WRITE: Mutex<()> = Mutex::new(());
static CATALOG_GEN: AtomicU64 = AtomicU64::new(0);
static INDEX_GEN: AtomicU64 = AtomicU64::new(0);

fn recording_root(root: &Path) -> bool {
    cfg!(test) || root == Path::new(crate::model::storage::ROOT)
}

fn catalog_folder(dir: &Path) -> Option<(&Path, usize)> {
    let root = dir.parent().filter(|path| recording_root(path))?;
    let name = dir.file_name()?.to_str()?;
    CATALOG_DIRS.iter().position(|&folder| folder == name).map(|folder| (root, folder))
}

pub fn recording_write_guard(root: &Path) -> Option<MutexGuard<'static, ()>> {
    recording_root(root).then(|| RECORDING_WRITE.lock().unwrap_or_else(|err| err.into_inner()))
}

/// The background worker can defer a read while an atomic replacement is in progress.
pub fn try_recording_read() -> Option<MutexGuard<'static, ()>> {
    if cfg!(test) {
        return Some(RECORDING_WRITE.lock().unwrap_or_else(|err| err.into_inner()));
    }
    match RECORDING_WRITE.try_lock() {
        Ok(lock) => Some(lock),
        Err(TryLockError::Poisoned(err)) => Some(err.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    }
}

fn note_catalog_write(dir: &Path, id: &str, size: Option<u64>) {
    let Some((root, folder)) = catalog_folder(dir) else { return };
    let mut writes = CATALOG_WRITES.lock().unwrap_or_else(|err| err.into_inner());
    writes.entry(root.to_path_buf()).or_default().insert((folder, id.to_owned()), size);
    CATALOG_GEN.fetch_add(1, Ordering::Release);
    drop(writes);
    #[cfg(target_os = "switch")]
    if folder == 1 {
        crate::engine::rr_host::invalidate_bound();
    }
    wake_catalog();
}

pub fn catalog_changes(root: &Path) -> CatalogSignatures {
    CATALOG_WRITES.lock().unwrap_or_else(|err| err.into_inner()).remove(root).unwrap_or_default()
}

pub fn catalog_generation() -> u64 {
    CATALOG_GEN.load(Ordering::Acquire)
}

pub fn advance_catalog() {
    CATALOG_GEN.fetch_add(1, Ordering::Release);
    wake_catalog();
}

pub fn index_generation() -> u64 {
    INDEX_GEN.load(Ordering::Acquire)
}

pub fn note_index_write(root: &Path) {
    if recording_root(root) {
        INDEX_GEN.fetch_add(1, Ordering::Release);
        wake_catalog();
    }
}

fn wake_catalog() {
    #[cfg(target_os = "switch")]
    crate::runtime::wake_recording_menu();
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use std::sync::atomic::{AtomicU32, Ordering};

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::*;

    /// A toy record: `{"schema_version": 1, "name": ".."}`, hand-written serde (no derive here).
    #[derive(Debug, PartialEq)]
    struct Note {
        version: u32,
        name: String,
    }

    impl Serialize for Note {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serde_json::json!({"schema_version": self.version, "name": self.name}).serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Note {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let value = serde_json::Value::deserialize(deserializer)?;
            let version = value.get("schema_version").and_then(serde_json::Value::as_u64);
            let name = value.get("name").and_then(serde_json::Value::as_str);
            match (version, name) {
                (Some(version), Some(name)) => Ok(Note { version: version as u32, name: name.to_owned() }),
                _ => Err(serde::de::Error::custom("not a note")),
            }
        }
    }

    impl Record for Note {
        type Error = StoreError;
        type Summary = String;
        const SCHEMA_VERSIONS: &'static [u32] = &[1];
        const MAX_FILE_BYTES: u64 = 256;

        fn validate(&self) -> Result<(), StoreError> {
            if self.version == 1 {
                Ok(())
            } else {
                Err(StoreError::UnsupportedVersion { found: self.version })
            }
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn summary(&self, id: &str) -> String {
            format!("{id}:{}", self.name)
        }
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let path = std::env::temp_dir()
                .join(format!("training_rooms-store-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn note(name: &str) -> Note {
        Note { version: 1, name: name.to_owned() }
    }

    fn ids_come_from_names_and_never_reuse_a_file() {
        let tmp = TempDir::new();
        let store = Store::<Note>::new(tmp.0.join("notes"));
        assert_eq!(store.save_new(&note("Fox ledge drill")).unwrap(), "fox-ledge-drill");
        assert_eq!(store.save_new(&note("Fox ledge drill")).unwrap(), "fox-ledge-drill-2");
        assert_eq!(store.save_new(&note("!!!")).unwrap(), "entry");
        assert_eq!(store.load("fox-ledge-drill-2").unwrap(), note("Fox ledge drill"));
        assert!(matches!(store.load("nothing"), Err(StoreError::NotFound(_))));
        assert!(matches!(store.load("Bad Id"), Err(StoreError::InvalidId(_))));
    }

    fn overwrite_is_atomic_and_needs_an_existing_file() {
        let tmp = TempDir::new();
        let store = Store::<Note>::new(&tmp.0);
        let id = store.save_new(&note("Keep")).unwrap();
        store.overwrite(&id, &note("Changed")).unwrap();
        assert_eq!(store.load(&id).unwrap().name, "Changed");
        assert!(matches!(store.overwrite("ghost", &note("Ghost")), Err(StoreError::NotFound(_))));
        // A directory squatting on the temp path makes the write fail and leaves the old file.
        fs::create_dir(tmp.0.join(format!("{id}.json.tmp"))).unwrap();
        let before = fs::read(tmp.0.join(format!("{id}.json"))).unwrap();
        assert!(matches!(store.overwrite(&id, &note("Lost")), Err(StoreError::Io(_))));
        assert_eq!(fs::read(tmp.0.join(format!("{id}.json"))).unwrap(), before);
        // An invalid record never reaches the disk.
        assert!(matches!(
            store.overwrite(&id, &Note { version: 2, name: "x".into() }),
            Err(StoreError::UnsupportedVersion { found: 2 })
        ));
    }

    fn listing_reports_version_size_and_corruption_and_touches_nothing() {
        let tmp = TempDir::new();
        let store = Store::<Note>::new(&tmp.0);
        let good = store.save_new(&note("Good")).unwrap();
        let bad: Vec<(&str, Vec<u8>)> = vec![
            ("future.json", br#"{"schema_version": 2, "extra": true}"#.to_vec()),
            ("big.json", vec![b' '; 257]),
            ("garbage.json", b"{ not json".to_vec()),
            ("Bad Name.json", br#"{"schema_version": 1, "name": "x"}"#.to_vec()),
        ];
        for (name, bytes) in &bad {
            fs::write(tmp.0.join(name), bytes).unwrap();
        }
        assert!(matches!(store.load("future"), Err(StoreError::UnsupportedVersion { found: 2 })));
        assert!(matches!(store.load("big"), Err(StoreError::FileTooLarge { max: 256, .. })));
        assert!(matches!(store.load("garbage"), Err(StoreError::Parse(_))));
        let listing = store.list().unwrap();
        assert_eq!(listing.entries, vec![format!("{good}:Good")]);
        let reason = |file: &str| listing.skipped.iter().find(|s| s.file == file).unwrap().reason.clone();
        assert_eq!(reason("future.json"), "unsupported schema version 2");
        assert!(reason("big.json").contains("bytes"));
        assert!(reason("garbage.json").starts_with("malformed file"));
        assert!(reason("Bad Name.json").starts_with("invalid id"));
        for (name, bytes) in &bad {
            assert_eq!(&fs::read(tmp.0.join(name)).unwrap(), bytes, "{name}");
        }
    }

    fn a_missing_directory_is_an_empty_store_and_an_orphaned_backup_is_restored() {
        let tmp = TempDir::new();
        let missing = Store::<Note>::new(tmp.0.join("nope"));
        assert!(missing.list().unwrap().entries.is_empty());
        let store = Store::<Note>::new(&tmp.0);
        fs::write(tmp.0.join("lost.json.bak"), serde_json::to_vec(&note("Lost")).unwrap()).unwrap();
        let listing = store.list().unwrap();
        assert_eq!(listing.entries, vec!["lost:Lost".to_owned()]);
        assert!(tmp.0.join("lost.json").exists() && !tmp.0.join("lost.json.bak").exists());
    }

    fn listing_is_sorted_by_name_ignoring_case() {
        let tmp = TempDir::new();
        let store = Store::<Note>::new(&tmp.0);
        store.save_new(&note("beta")).unwrap();
        store.save_new(&note("Alpha")).unwrap();
        assert_eq!(store.list().unwrap().entries, vec!["alpha:Alpha".to_owned(), "beta:beta".to_owned()]);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("ids_come_from_names_and_never_reuse_a_file", ids_come_from_names_and_never_reuse_a_file as fn()),
            ("overwrite_is_atomic_and_needs_an_existing_file", overwrite_is_atomic_and_needs_an_existing_file as fn()),
            ("listing_reports_version_size_and_corruption_and_touches_nothing", listing_reports_version_size_and_corruption_and_touches_nothing as fn()),
            ("a_missing_directory_is_an_empty_store_and_an_orphaned_backup_is_restored", a_missing_directory_is_an_empty_store_and_an_orphaned_backup_is_restored as fn()),
            ("listing_is_sorted_by_name_ignoring_case", listing_is_sorted_by_name_ignoring_case as fn()),
        ]);
    }
    }
}
