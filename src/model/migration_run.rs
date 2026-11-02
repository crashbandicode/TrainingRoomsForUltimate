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
}

fn io_reason(error: &io::Error) -> String {
    match error.raw_os_error() {
        Some(code) => format!("io:{code}"),
        None => format!("io:{:?}", error.kind()),
    }
}

/// Every file of the imported names inside `source` (relative, `/` separated), with its size.
fn list_source(source: &Path) -> io::Result<Vec<(String, u64)>> {
    let mut out = Vec::new();
    for name in [file::LEGACY_SETTINGS, "button_config.ini", "replay_capture.ini", "save_states.json", "mii-presets.json"] {
        if let Ok(meta) = fs::metadata(source.join(name)) {
            if meta.is_file() {
                out.push((name.to_owned(), meta.len()));
            }
        }
    }
    for dir in ["setups", "macros", "replay_captures"] {
        walk(&source.join(dir), dir, &mut out)?;
    }
    Ok(out)
}

fn walk(path: &Path, rel: &str, out: &mut Vec<(String, u64)>) -> io::Result<()> {
    let Ok(entries) = fs::read_dir(path) else { return Ok(()) };
    let mut names: Vec<_> = entries.filter_map(Result::ok).collect();
    names.sort_by_key(|e| e.file_name());
    for entry in names {
        let child = format!("{rel}/{}", entry.file_name().to_string_lossy());
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            if migration::is_staging(&child) {
                // A staging folder is listed as one marker entry for the plan to count; nothing in it
                // is opened, listed or read.
                out.push((format!("{child}/"), 0));
                continue;
            }
            walk(&entry.path(), &child, out)?;
        } else if kind.is_file() {
            out.push((child, entry.metadata().map(|m| m.len()).unwrap_or(0)));
        }
    }
    Ok(())
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
fn item_setting(source: &Path) -> Option<usize> {
    let bytes = fs::read(source.join(file::LEGACY_SETTINGS)).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    convert::setting_index(value.get("menu")?.get("character_item")?)
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
    let listing = if roots.source.is_dir() { list_source(roots.source).ok() } else { None };
    let sources: Option<Vec<Source<'_>>> = listing.as_ref().map(|l| l.iter().map(|(rel, bytes)| Source { rel, bytes: *bytes }).collect());
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
    let mut background = Vec::new();
    let mut staging_folders: Vec<String> = Vec::new();
    let item_index = item_setting(roots.source);
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
    let marker = blocking.background.is_empty().then(|| write_marker(roots.dest, &blocking).map_err(|e| e.to_string()));
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
    let marker = write_marker(roots.dest, &blocking).map_err(|e| e.to_string());
    (blocking.receipt, marker)
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
        Stage::Imported { marker: Some(Err(_)), .. } => "skipped",
        Stage::Imported { blocking, .. } => blocking.receipt.result().word(),
    }
}
