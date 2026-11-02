//! The first-boot import (SPEC-core-08), the part that touches files: it follows `migration::plan`,
//! copies each planned file through `<name>.part` and a rename, hashes what it copies, converts the
//! retired Item-on-load setting in the copied `save_states.json` once (decision N6), and writes
//! `migration.json` last. Used by the runtime on its boot thread and its background thread; host-tested on
//! real directories.
//!
//! Rules (SPEC-core-08 §4, review R1): the fork's folder is only ever read; an existing destination is
//! never overwritten; a copy is complete or absent (a leftover `.part` is overwritten next run); a read or
//! write error skips that one file and the result is `partial`; the marker is the only thing that means
//! "already imported" (a pre-installed `developer.ini` does not count and is never touched); the marker
//! itself is written through the store's atomic replace, and when that fails the import runs again next
//! boot (harmless: what exists is kept).
//!
//! Absent is not unreadable (review R2-8): a missing optional path is simply not imported, but a path
//! that is present and cannot be enumerated, stat-ed or typed (a directory that cannot be opened, an
//! entry whose metadata fails, a source folder that is not a folder) is an error. It goes to the
//! receipt's `unlisted` list and the log with its path and the error, the result is `partial`, the other
//! paths are still imported, and **no marker is written**, so the next boot tries again and the exists
//! rule keeps what was copied. (A file that was listed but fails while it is copied stays rule 4's
//! `partial` with its marker.)

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::migration::{self, Action, ImportResult, Outcome, Part, Plan, Skip, Source};
use super::sha256::{hex, Sha256};
use super::state::convert;
use super::storage::file;
use super::store::write_replace;

/// The copy buffer (SPEC-core-08 §4 rule 3).
pub const BUFFER_BYTES: usize = 64 * 1024;
/// The label written in the marker: the fork's folder on the card.
pub const SOURCE_LABEL: &str = "sd:/ultimate/TrainingModpack";
const DEST_LABEL: &str = "sd:/ultimate/TrainingRooms";

/// The two folders.
#[derive(Clone, Copy, Debug)]
pub struct Roots<'a> {
    pub source: &'a Path,
    pub dest: &'a Path,
}

/// One copied or skipped file, as the receipt records it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileResult {
    pub from: String,
    pub to: String,
    pub bytes: u64,
    /// The hash of the bytes written to the destination.
    pub sha256: String,
    /// The hash of the source bytes (differs only for the converted `save_states.json`).
    pub source_sha256: String,
}

/// What an import did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Receipt {
    pub files: Vec<FileResult>,
    /// `(path, reason)`: `exists`, `too_large`, `io:<code>`.
    pub skipped: Vec<(String, String)>,
    /// `(path, error)`: present paths that could not be listed, stat-ed or typed (R2-8). Any entry makes
    /// the result `partial` and withholds the marker, so the next boot imports again.
    pub unlisted: Vec<(String, String)>,
    /// How many capture staging folders were left alone.
    pub skipped_staging: usize,
    /// Player save-state records that got an item snapshot (N6).
    pub converted: usize,
    pub outcomes: Vec<Outcome>,
}

impl Receipt {
    pub fn result(&self) -> ImportResult {
        migration::result(&self.outcomes)
    }
}

/// How the blocking part ended.
#[derive(Debug)]
pub enum Stage {
    /// `migration.json` exists: nothing was done (`migration=skipped`).
    MarkerPresent,
    /// The source folder does not exist: the marker was written with `result: none`.
    NoSource { marker: Result<(), String> },
    /// The blocking files are copied; `background` still has the captures. When there are none the
    /// marker has already been written (`marker`).
    Imported { blocking: Blocking, marker: Option<Result<(), String>> },
}

/// What the blocking part leaves for the background part.
#[derive(Debug)]
pub struct Blocking {
    pub receipt: Receipt,
    background: Vec<(String, String)>,
    now: u64,
    plugin: String,
}

impl Blocking {
    /// How many capture files the background part will copy.
    pub fn pending_captures(&self) -> usize {
        self.background.len()
    }

    /// Whether some present path could not be listed: the marker is not written, the next boot imports
    /// again.
    pub fn marker_withheld(&self) -> bool {
        !self.receipt.unlisted.is_empty()
    }
}

fn io_reason(error: &io::Error) -> String {
    match error.raw_os_error() {
        Some(code) => format!("io:{code}"),
        None => format!("io:{:?}", error.kind()),
    }
}

/// A listing of the source folder: the files found, and the present paths that could not be examined
/// (`(path, error)`, R2-8). Only `NotFound` means absent.
struct Listing {
    files: Vec<(String, u64)>,
    unlisted: Vec<(String, String)>,
}

fn error_text(error: &io::Error) -> String {
    format!("{} ({})", io_reason(error), error)
}

/// Every file of the imported names inside `source` (relative, `/` separated), with its size.
fn list_source(source: &Path) -> Listing {
    let mut listing = Listing { files: Vec::new(), unlisted: Vec::new() };
    for name in [file::LEGACY_SETTINGS, "button_config.ini", "replay_capture.ini", "save_states.json", "mii-presets.json"] {
        match fs::metadata(source.join(name)) {
            Ok(meta) if meta.is_file() => listing.files.push((name.to_owned(), meta.len())),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => listing.unlisted.push((name.to_owned(), error_text(&error))),
        }
    }
    for dir in ["setups", "macros", "replay_captures"] {
        walk(&source.join(dir), dir, &mut listing);
    }
    listing
}

fn walk(path: &Path, rel: &str, out: &mut Listing) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return,
        Err(error) => {
            out.unlisted.push((rel.to_owned(), error_text(&error)));
            return;
        }
    };
    let mut names = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => names.push(entry),
            Err(error) => out.unlisted.push((format!("{rel}/"), error_text(&error))),
        }
    }
    names.sort_by_key(|e| e.file_name());
    for entry in names {
        let child = format!("{rel}/{}", entry.file_name().to_string_lossy());
        let kind = match entry.file_type() {
            Ok(kind) => kind,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                out.unlisted.push((child, error_text(&error)));
                continue;
            }
        };
        if kind.is_dir() {
            if migration::is_staging(&child) {
                // A staging folder is listed as one marker entry for the plan to count; nothing in it
                // is opened, listed or read.
                out.files.push((format!("{child}/"), 0));
                continue;
            }
            walk(&entry.path(), &child, out);
        } else if kind.is_file() {
            match entry.metadata() {
                Ok(meta) => out.files.push((child, meta.len())),
                // Gone between the listing and the stat: nothing to import.
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => out.unlisted.push((child, error_text(&error))),
            }
        }
    }
}
/// Copy `from` to `to` through `<to>.part` and a rename, hashing both sides. `transform` may change the
/// bytes (the converted `save_states.json`); it needs the whole file in memory (the blocking part is
/// capped at 8 MiB). Never overwrites: if `to` exists when the copy is done it is kept.
fn copy_file(from: &Path, to: &Path, transform: Option<&dyn Fn(Vec<u8>) -> Vec<u8>>) -> io::Result<Option<(u64, String, String)>> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut part = to.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let mut input = fs::File::open(from)?;
    let mut source_hash = Sha256::new();
    let mut dest_hash = Sha256::new();
    let mut bytes = 0u64;
    {
        let mut output = fs::File::create(&part)?;
        let result = (|| -> io::Result<()> {
            if let Some(transform) = transform {
                let mut all = Vec::new();
                input.read_to_end(&mut all)?;
                source_hash.update(&all);
                let changed = transform(all);
                dest_hash.update(&changed);
                bytes = changed.len() as u64;
                output.write_all(&changed)?;
            } else {
                let mut buffer = vec![0u8; BUFFER_BYTES];
                loop {
                    let n = input.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    source_hash.update(&buffer[..n]);
                    dest_hash.update(&buffer[..n]);
                    output.write_all(&buffer[..n])?;
                    bytes += n as u64;
                }
            }
            output.sync_all()
        })();
        if let Err(error) = result {
            drop(output);
            let _ = fs::remove_file(&part);
            return Err(error);
        }
    }
    if to.exists() {
        // Another writer got there first: keep theirs.
        let _ = fs::remove_file(&part);
        return Ok(None);
    }
    if let Err(error) = fs::rename(&part, to) {
        let _ = fs::remove_file(&part);
        return Err(error);
    }
    Ok(Some((bytes, hex(&source_hash.finish()), hex(&dest_hash.finish()))))
}

/// The `character_item` index in the fork's settings file, when it names an item (N6).
/// An unreadable settings file (not an absent one) is an error: the conversion would be lost for good once
/// the marker exists.
fn item_setting(source: &Path) -> io::Result<Option<usize>> {
    let bytes = match fs::read(source.join(file::LEGACY_SETTINGS)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let index = serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(|value| convert::setting_index(value.get("menu")?.get("character_item")?));
    Ok(index)
}

/// The source folder: absent (`None`), listed, or present but unusable (a file, or a stat error): then
/// nothing can be listed and the path is reported.
fn examine_source(source: &Path) -> Option<Listing> {
    match fs::metadata(source) {
        Ok(meta) if meta.is_dir() => Some(list_source(source)),
        Ok(_) => Some(Listing { files: Vec::new(), unlisted: vec![(".".to_owned(), "not a folder".to_owned())] }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => Some(Listing { files: Vec::new(), unlisted: vec![(".".to_owned(), error_text(&error))] }),
    }
}

fn withheld_reason(unlisted: usize) -> String {
    format!("{unlisted} present path(s) could not be listed; the import runs again next boot")
}

fn record(receipt: &mut Receipt, action: &Action<'_>, result: io::Result<Option<(u64, String, String)>>) {
    match result {
        Ok(Some((bytes, source_sha256, sha256))) => {
            receipt.files.push(FileResult {
                from: format!("{SOURCE_LABEL}/{}", action.from),
                to: format!("{DEST_LABEL}/{}", action.to),
                bytes,
                sha256,
                source_sha256,
            });
            receipt.outcomes.push(Outcome::Copied);
        }
        Ok(None) => {
            receipt.skipped.push((action.from.to_owned(), Skip::Exists.word().to_owned()));
            receipt.outcomes.push(Outcome::Skipped(Skip::Exists));
        }
        Err(error) => {
            receipt.skipped.push((action.from.to_owned(), io_reason(&error)));
            receipt.outcomes.push(Outcome::IoError);
        }
    }
}

/// The blocking part: the small files, before the settings load. `now` is a tick for the marker.
pub fn run_blocking(roots: Roots<'_>, now: u64, plugin: &str, log: &mut dyn FnMut(&str)) -> Stage {
    if roots.dest.join(file::MIGRATION).exists() {
        return Stage::MarkerPresent;
    }
    let mut listing = examine_source(roots.source);
    let unlisted = listing.as_mut().map(|l| std::mem::take(&mut l.unlisted)).unwrap_or_default();
    let sources: Option<Vec<Source<'_>>> = listing.as_ref().map(|l| l.files.iter().map(|(rel, bytes)| Source { rel, bytes: *bytes }).collect());
    let destination_exists = |to: &str| roots.dest.join(to).exists();
    let plan = migration::plan(false, sources.as_deref(), destination_exists);
    let actions = match plan {
        Plan::MarkerPresent => return Stage::MarkerPresent,
        Plan::NoSource => {
            let blocking = Blocking { receipt: Receipt::default(), background: Vec::new(), now, plugin: plugin.to_owned() };
            let marker = write_marker(roots.dest, &blocking).map_err(|e| e.to_string());
            log("[Migration] no source folder; marker written with result none");
            return Stage::NoSource { marker };
        }
        Plan::Copy(actions) => actions,
    };
    let mut receipt = Receipt::default();
    for (path, error) in unlisted {
        log(&format!("[Migration] cannot list {path}: {error}; the import stays incomplete and runs again next boot"));
        receipt.skipped.push((path.clone(), error.clone()));
        receipt.outcomes.push(Outcome::IoError);
        receipt.unlisted.push((path, error));
    }
    let mut background = Vec::new();
    let mut staging_folders: Vec<String> = Vec::new();
    // Only a settings file the listing found is read for the conversion (a source that is not a folder was
    // reported above).
    let has_settings = listing.as_ref().is_some_and(|l| l.files.iter().any(|(rel, _)| rel == file::LEGACY_SETTINGS));
    let item_index = match if has_settings { item_setting(roots.source) } else { Ok(None) } {
        Ok(index) => index,
        Err(error) => {
            let error = error_text(&error);
            log(&format!("[Migration] cannot read {}: {error}; the import stays incomplete and runs again next boot", file::LEGACY_SETTINGS));
            receipt.skipped.push((file::LEGACY_SETTINGS.to_owned(), error.clone()));
            receipt.outcomes.push(Outcome::IoError);
            receipt.unlisted.push((file::LEGACY_SETTINGS.to_owned(), error));
            None
        }
    };
    for action in &actions {
        if action.skip == Some(Skip::Staging) {
            // `replay_captures/.<id>.smd-N.tmp/…`: count the folder once, read nothing.
            if let Some(folder) = action.from.split('/').nth(1) {
                if !staging_folders.iter().any(|f| f == folder) {
                    staging_folders.push(folder.to_owned());
                }
            }
            continue;
        }
        if let Some(skip) = action.skip {
            receipt.skipped.push((action.from.to_owned(), skip.word().to_owned()));
            receipt.outcomes.push(Outcome::Skipped(skip));
            log(&format!("[Migration] skipped {} reason={}", action.from, skip.word()));
            continue;
        }
        if action.part == Part::Background {
            background.push((action.from.to_owned(), action.to.to_owned()));
            continue;
        }
        // N6: the copied `save_states.json` gets item snapshots from the old setting, once.
        let converted = std::cell::Cell::new(0usize);
        let convert_saves = |bytes: Vec<u8>| -> Vec<u8> {
            let Some(index) = item_index else { return bytes };
            let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) else { return bytes };
            let n = convert::convert_json(&mut value, index);
            converted.set(n);
            if n == 0 {
                return bytes;
            }
            serde_json::to_vec_pretty(&value).unwrap_or(bytes)
        };
        let transform: Option<&dyn Fn(Vec<u8>) -> Vec<u8>> = (action.from == "save_states.json").then_some(&convert_saves);
        let result = copy_file(&roots.source.join(action.from), &roots.dest.join(action.to), transform);
        let copied = matches!(result, Ok(Some(_)));
        record(&mut receipt, action, result);
        if copied {
            receipt.converted += converted.get();
            log(&format!("[Migration] copied {} -> {} (converted records: {})", action.from, action.to, converted.get()));
        }
    }
    receipt.skipped_staging = staging_folders.len();
    let blocking = Blocking { receipt, background, now, plugin: plugin.to_owned() };
    let marker = blocking.background.is_empty().then(|| write_marker_unless_withheld(roots.dest, &blocking));
    Stage::Imported { blocking, marker }
}

/// The background part: copy the captures (no size cap), then write the marker.
pub fn run_background(roots: Roots<'_>, mut blocking: Blocking, log: &mut dyn FnMut(&str)) -> (Receipt, Result<(), String>) {
    let pending = std::mem::take(&mut blocking.background);
    for (from, to) in &pending {
        let action = Action { from, to, part: Part::Background, skip: None };
        let result = copy_file(&roots.source.join(from), &roots.dest.join(to), None);
        if let Ok(Some((bytes, _, hash))) = &result {
            log(&format!("[Migration] copied {from} bytes={bytes} sha256={hash}"));
        }
        record(&mut blocking.receipt, &action, result);
    }
    let marker = write_marker_unless_withheld(roots.dest, &blocking);
    (blocking.receipt, marker)
}

/// The marker, unless a present path could not be listed (R2-8): then `Err` and no file, like a marker
/// that could not be written, and the next boot imports again.
fn write_marker_unless_withheld(dest: &Path, blocking: &Blocking) -> Result<(), String> {
    if blocking.marker_withheld() {
        return Err(withheld_reason(blocking.receipt.unlisted.len()));
    }
    write_marker(dest, blocking).map_err(|e| e.to_string())
}

/// `migration.json` (SPEC-core-08 §4 rule 5), written through the atomic replace.
fn write_marker(dest: &Path, blocking: &Blocking) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    let receipt = &blocking.receipt;
    let files: Vec<Value> = receipt
        .files
        .iter()
        .map(|f| json!({"from": f.from, "to": f.to, "bytes": f.bytes, "sha256": f.sha256, "source_sha256": f.source_sha256, "result": "copied"}))
        .collect();
    let skipped: Vec<Value> = receipt.skipped.iter().map(|(path, reason)| json!({"path": path, "reason": reason})).collect();
    let marker = json!({
        "source": SOURCE_LABEL,
        "at": blocking.now,
        "plugin": blocking.plugin,
        "result": receipt.result().word(),
        "files": files,
        "skipped": skipped,
        "skipped_staging": receipt.skipped_staging,
        "converted_item_snapshots": receipt.converted,
    });
    let text = serde_json::to_string_pretty(&marker).map_err(io::Error::other)?;
    write_replace(&dest.join(file::MIGRATION), format!("{text}\n").as_bytes())
}

/// The install line's `migration=` word for a finished blocking part.
pub fn status_word(stage: &Stage) -> &'static str {
    match stage {
        Stage::MarkerPresent => "skipped",
        Stage::NoSource { marker: Ok(()) } => "none",
        Stage::NoSource { marker: Err(_) } => "skipped",
        Stage::Imported { blocking, .. } if blocking.marker_withheld() => "partial",
        Stage::Imported { marker: Some(Err(_)), .. } => "skipped",
        Stage::Imported { blocking, .. } => blocking.receipt.result().word(),
    }
}
