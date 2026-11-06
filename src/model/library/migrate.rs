//! Migration of today's recording slots and save indexes into named files.
//!
//! Numbered recording slots are not files (SPEC-input-01 §7 and the persistence list: slot contents
//! are not persisted). The caller passes the live slots beside the directory that already holds
//! `settings.json`, `save_states.json`, and `macros/`. The date is the day this function runs.

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::chain::is_chain_document;
use super::hot::RecordingIndex;
use super::save::NamedSave;
use super::{Error, Kind, LoopCount, Use, MACROS_DIR, SAVES_DIR};
use crate::engine::macros::{InputFrame, MacroRecording, StartState};
use crate::model::state::file::SaveFile;
use crate::model::store::{default_id, Record, Store, SELECTED};
use crate::model::version::SUPPORTED_VERSION;

pub use super::hot::MigrationReceipt;

/// One live recording slot. `None` in the array is a slot that was never recorded.
#[derive(Clone, Debug, PartialEq)]
pub struct LegacySlot {
    pub frames: Vec<InputFrame>,
    pub start: StartState,
}

/// Turns today's slots into named files. Empty slots are skipped. No chain and no sequence is written.
pub fn migrate(root: &Path, slots: &[Option<LegacySlot>; 5], date: &str) -> Result<MigrationReceipt, Error> {
    let mut index = RecordingIndex::load(root)?;
    if index.migrated() {
        return Ok(index.receipt().cloned().unwrap_or_default());
    }
    check_date(date)?;
    // These rows stay in the settings file and become nothing here.
    let _ = menu_row(root, "save_state_playback");
    let _ = menu_row(root, "record_trigger");

    let loops = loop_from(menu_row(root, "playback_loop").as_deref());
    let slot_index = save_slot(menu_row(root, "save_state_slot").as_deref());
    let saves = read_saves(root);

    let mut recording_ids = Vec::new();
    for (index_in_list, slot) in slots.iter().enumerate() {
        let Some(slot) = slot else { continue };
        if slot.frames.is_empty() {
            continue;
        }
        let name = format!("Slot {} ({date})", index_in_list + 1);
        let recording = MacroRecording::new(name, SUPPORTED_VERSION, slot.start, slot.frames.clone())
            .map_err(|err| Error::Record(err.to_string()))?;
        let id = place(&root.join(MACROS_DIR), &recording, true)?;
        recording_ids.push(id);
    }

    let mut save_ids = Vec::new();
    let mut bound = None;
    for index_in_list in 0..5 {
        let player = saves.player[index_in_list].clone();
        let cpu = saves.cpu[index_in_list].clone();
        if !player.is_saved() && !cpu.is_saved() {
            continue;
        }
        let name = format!("Slot {} ({date})", index_in_list + 1);
        let save = NamedSave::new(name, player, cpu)?;
        let id = place(&root.join(SAVES_DIR), &save, false)?;
        if index_in_list == slot_index {
            bound = Some(id.clone());
        }
        save_ids.push(id);
    }

    let active = selected_macro(root);
    seed(&mut index, Kind::Macro, &recording_ids)?;
    seed(&mut index, Kind::Save, &save_ids)?;
    let receipt = MigrationReceipt {
        recordings: recording_ids,
        saves: save_ids,
        bound_save: bound,
        active_player: active,
        loop_player: loops,
        loop_cpu: loops,
    };
    index.apply_migration(&receipt);
    index.save(root)?;
    Ok(receipt)
}

fn seed(index: &mut RecordingIndex, kind: Kind, ids: &[String]) -> Result<(), Error> {
    let count = ids.len() as u64;
    for (offset, id) in ids.iter().enumerate() {
        index.note_use(kind, id, Use::Recorded, count - offset as u64)?;
    }
    Ok(())
}

fn check_date(date: &str) -> Result<(), Error> {
    let bytes = date.as_bytes();
    let shaped = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes.iter().enumerate().all(|(i, byte)| i == 4 || i == 7 || byte.is_ascii_digit());
    if !shaped {
        return Err(Error::Invalid("bad date"));
    }
    let month: u8 = date[5..7].parse().unwrap_or(0);
    let day: u8 = date[8..10].parse().unwrap_or(0);
    if (1..=12).contains(&month) && (1..=31).contains(&day) {
        Ok(())
    } else {
        Err(Error::Invalid("bad date"))
    }
}

fn menu_row(root: &Path, id: &str) -> Option<Vec<u8>> {
    let bytes = fs::read(root.join("settings.json")).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    let items = value.get("menu")?.get(id)?.as_array()?;
    Some(items.iter().filter_map(|item| item.as_u64().and_then(|n| u8::try_from(n).ok())).collect())
}

/// OnOff is `[On, Off]`. A set first byte is On, which becomes infinite. Off, or a missing row, is 1.
fn loop_from(row: Option<&[u8]>) -> LoopCount {
    match row {
        Some(row) if row.first().is_some_and(|value| *value != 0) => LoopCount::Infinite,
        _ => LoopCount::Finite(1),
    }
}

/// First set byte, or 0 when the row is missing. That index is the bound save when it was written.
fn save_slot(row: Option<&[u8]>) -> usize {
    row.and_then(|row| row.iter().position(|value| *value != 0)).unwrap_or(0).min(4)
}

fn read_saves(root: &Path) -> SaveFile {
    match fs::read(root.join("save_states.json")) {
        Ok(bytes) => SaveFile::parse(&bytes).unwrap_or_default(),
        Err(_) => SaveFile::default(),
    }
}

fn selected_macro(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root.join(MACROS_DIR).join(SELECTED)).ok()?;
    let id = text.trim();
    if !default_id(id) {
        return None;
    }
    root.join(MACROS_DIR).join(format!("{id}.json")).is_file().then(|| id.to_owned())
}

fn place<R: Record>(dir: &Path, record: &R, skip_chains: bool) -> Result<String, Error> {
    if let Some(id) = find_named(dir, record.name(), skip_chains)? {
        return Ok(id);
    }
    Store::new(dir).save_new(record).map_err(|err| Error::Record(err.to_string()))
}

fn find_named(dir: &Path, name: &str, skip_chains: bool) -> Result<Option<String>, Error> {
    let read = match fs::read_dir(dir) {
        Ok(read) => read,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let mut paths = Vec::new();
    for item in read {
        paths.push(item?.path());
    }
    paths.sort();
    for path in paths {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else { continue };
        let Some(id) = file_name.strip_suffix(".json") else { continue };
        if !default_id(id) {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else { continue };
        if skip_chains && is_chain_document(&bytes) {
            continue;
        }
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else { continue };
        if value.get("name").and_then(Value::as_str) == Some(name) {
            return Ok(Some(id.to_owned()));
        }
    }
    Ok(None)
}
