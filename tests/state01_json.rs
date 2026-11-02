//! SPEC-state-01 §7 `save_states.json`: the record shape, the fork's real files, tolerance, and the
//! filesystem rules (two boots, a write failure). The fixtures and the shape tests are the fork's
//! `training_mod_consts/tests/json_ported_fixtures.rs` and `json_save_state_shape.rs` (stage m5o,
//! ours-new; CARRY-OVER.md) ported to this crate's hand-written reader and writer: the same files,
//! the same assertions, the same byte-for-byte round trip.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use model::state::file::{Defaulted, SaveFile, SLOTS};
use model::state::held_item::HeldItem;
use model::state::persist::{self, Outcome};
use model::state::record::{Charge, Marker, Robin, SavedState, Steve};
use serde_json::Value;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!("training_rooms-state01-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ported-json")
}

fn robin() -> Charge {
    Charge {
        robin: Some(Robin {
            thunder_kind: 2,
            yellow_book: 1,
            red_book: 0.5,
            green_book: 3,
            purple_book: 0,
            levin_charge: 4,
            levin_recharge: 0,
            has_levin: true,
        }),
        ..Charge::default()
    }
}

fn captured(saved_stage: Option<i32>) -> SavedState {
    SavedState {
        x: -8.116991,
        y: 0.010259508,
        percent: 42.5,
        lr: -1.0,
        situation_kind: 2,
        marker: Marker::NoAction,
        fighter_kind: 8,
        charge: robin(),
        steve: Some(Steve::START),
        held_item: Some(HeldItem::Item { choice: 25, variation: 7, lifetime: 500, lifetime_max: 1400, fuse: -1, fuse_max: 0 }),
        saved_stage,
        mii: None,
    }
}

/// An opaque Mii identity whose third tag exceeds `i64::MAX` (the tag the TOML writers could not hold).
fn mii() -> Value {
    serde_json::from_str(
        r#"{"canonical": {"class": "Brawler"}, "charinfo": [7, 7, 7], "tags": [4505798650110000, 1000000000000, 14000000000000000000]}"#,
    )
    .unwrap()
}

#[test]
fn state01_record_json_shape() {
    let mut file = SaveFile::default();
    file.player[0] = SavedState { mii: Some(mii()), ..captured(Some(95)) };
    file.player[1] = captured(Some(0));
    file.player[2] = SavedState {
        charge: Charge { int_x: Some(3), float_y: Some(1.5), has_charge: Some(true), ..Charge::default() },
        steve: None,
        held_item: Some(HeldItem::Empty),
        saved_stage: Some(0x1f),
        ..captured(None)
    };
    file.player[3] = SavedState { held_item: Some(HeldItem::Unsupported), ..SavedState::never_saved() };
    file.cpu[0] = SavedState { saved_stage: Some(95), mii: Some(mii()), ..SavedState::never_saved() };
    file.cpu[4] = captured(Some(-1));

    let text = file.to_pretty();
    assert!(text.contains("14000000000000000000"), "the full u64 tag survives:\n{text}");
    assert!(text.contains("\"saved_stage\": 95"));
    assert!(text.contains("\"state\": \"item\""));
    let back = SaveFile::parse(text.as_bytes()).expect("parses back");
    assert_eq!(back, file);
    // The generic tree keeps the tag unsigned.
    let value: Value = serde_json::from_str(&text).unwrap();
    let tag = &value["player"][0]["mii"]["tags"][2];
    assert_eq!(tag.as_u64(), Some(14_000_000_000_000_000_000));
    assert_eq!(tag.as_i64(), None);
}

#[test]
fn state01_legacy_shaped_json_loads_with_defaults() {
    let legacy = r#"{"player": [
        {"x": -8.116991, "y": 0.010259508, "percent": 0.0, "lr": 1.0, "situation_kind": 0,
         "state": "NoAction", "fighter_kind": 8, "charge": {}, "steve_state": null},
        {"x": 0.0, "y": 0.0, "percent": 0.0, "lr": 1.0, "situation_kind": 0, "state": "NoAction",
         "fighter_kind": -1, "charge": {}, "steve_state": {"mat_g1": 36, "mat_wood": 18, "mat_stone": 0,
         "mat_iron": 3, "mat_gold": 0, "mat_redstone": 2, "mat_diamond": 0, "sword_mat": "\u0001",
         "sword_durability": 25.0, "axe_mat": "\u0001", "axe_durability": 70.0, "pick_mat": "\u0001",
         "pick_durability": 70.0, "shovel_mat": "\u0001", "shovel_durability": 70.0}}],
      "cpu": [
        {"x": 1.5, "y": 2.5, "percent": 10.0, "lr": -1.0, "situation_kind": 2, "state": "KillPlayer",
         "fighter_kind": 3, "charge": {"int_x": 3, "float_y": 1.5, "has_charge": true}, "steve_state": null,
         "held_item": {"state": "item", "choice": 25, "variation": 7, "lifetime": 500, "lifetime_max": 1400,
         "fuse": -1, "fuse_max": 0}}]}"#;
    let file = SaveFile::parse(legacy.as_bytes()).expect("legacy-shaped JSON");
    // Short arrays keep their records and default the rest.
    assert!(file.player[2..].iter().all(|r| *r == SavedState::never_saved()));
    assert!(file.player.iter().chain(&file.cpu).all(|s| s.saved_stage.is_none() && s.mii.is_none()));
    assert_eq!(file.player[0].x, -8.116991);
    assert_eq!(file.player[0].charge, Charge::default());
    assert_eq!(file.player[0].steve, None);
    assert_eq!(file.player[0].held_item, None);
    assert_eq!(file.player[1].steve, Some(Steve::START));
    // The persisted in-progress marker never resumes a load in a new process.
    assert_eq!(file.cpu[0].marker, Marker::NoAction);
    assert_eq!(file.cpu[0].charge, Charge { int_x: Some(3), float_y: Some(1.5), has_charge: Some(true), ..Charge::default() });
    assert_eq!(
        file.cpu[0].held_item,
        Some(HeldItem::Item { choice: 25, variation: 7, lifetime: 500, lifetime_max: 1400, fuse: -1, fuse_max: 0 })
    );
    let rewritten = file.to_pretty();
    assert_eq!(SaveFile::parse(rewritten.as_bytes()).unwrap(), file);
}

fn saved_fixtures() -> Vec<PathBuf> {
    let mut names: Vec<PathBuf> = fs::read_dir(fixtures())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .filter(|p| p.file_name().unwrap().to_string_lossy().contains("save_states"))
        .collect();
    names.sort();
    names
}

fn imported_text(path: &Path) -> String {
    let name = format!("{}.imported.json", path.file_stem().unwrap().to_string_lossy());
    fs::read_to_string(fixtures().join("expected-a-import").join(name)).unwrap()
}

#[test]
fn state01_ported_files_load_and_rewrite_as_the_fork_wrote_them() {
    let names = saved_fixtures();
    assert!(names.len() >= 5, "{names:?}");
    let mut with_items = 0;
    for path in names {
        let converted = fs::read_to_string(&path).unwrap();
        let ported = SaveFile::parse(converted.as_bytes()).unwrap_or_else(|e| panic!("{}: {:?}", path.display(), e));
        let expected = imported_text(&path);
        let imported = SaveFile::parse(expected.as_bytes()).unwrap();
        assert_eq!(ported, imported, "{}: converter output differs from the toml import", path.display());
        // Byte for byte what the fork's writer produced for the same slots.
        assert_eq!(ported.to_pretty(), expected, "{}", path.display());
        assert!(!converted.contains("null"), "{}", path.display());
        for slot in ported.player.iter().chain(ported.cpu.iter()) {
            assert!(slot.saved_stage.is_none() && slot.mii.is_none(), "{}", path.display());
            slot.charge.validate().unwrap();
            if let Some(steve) = slot.steve {
                steve.validate().unwrap();
                assert_eq!(steve.sword_mat, 1, "{}", path.display());
            }
            if let Some(item) = slot.held_item {
                with_items += 1;
                assert!(item.valid(), "{}", path.display());
            }
        }
    }
    assert!(with_items >= 2, "fixtures with a held_item table were expected");
}

#[test]
fn state01_damaged_fields_do_not_discard_adjacent_slots() {
    for damaged in [
        r#""damaged""#,
        r#"{"state": "unknown"}"#,
        r#"{"state": "item", "choice": 25, "variation": 0, "lifetime": 0, "lifetime_max": 0, "fuse": -1, "fuse_max": 0}"#,
        r#"{"state": "item", "choice": 11, "variation": 0, "lifetime": -1, "lifetime_max": -1, "fuse": 1801, "fuse_max": 1800}"#,
        r#"{"state": "item", "choice": 999}"#,
    ] {
        let text = format!(
            r#"{{"player": [{{"fighter_kind": 1}}, {{"fighter_kind": 2, "held_item": {damaged}}}, {{"fighter_kind": 3, "held_item": {{"state": "empty"}}}}]}}"#
        );
        let file = SaveFile::parse(text.as_bytes()).unwrap();
        assert_eq!(file.player[0].held_item, None);
        assert_eq!(file.player[1].held_item, Some(HeldItem::Unsupported));
        assert_eq!(file.player[2].held_item, Some(HeldItem::Empty));
        assert_eq!(file.player[1].fighter_kind, 2);
    }
    // A JSON null is an absent snapshot, not a damaged item.
    let file = SaveFile::parse(br#"{"player": [{"held_item": null}]}"#).unwrap();
    assert_eq!(file.player[0].held_item, None);
}

#[test]
fn state01_file_missing_defaults_and_unusable_files_report_why() {
    assert_eq!(SaveFile::parse(b"{ not json").unwrap_err(), Defaulted::NotJson);
    assert_eq!(SaveFile::parse(b"[1, 2]").unwrap_err(), Defaulted::WrongShape);
    assert_eq!(SaveFile::parse(b"{\"other\": 1}").unwrap_err(), Defaulted::WrongShape);
    let defaults = SaveFile::default();
    assert_eq!(defaults.player.len(), SLOTS);
    assert!(defaults.cpu.iter().all(|r| r.fighter_kind == -1 && r.lr == 1.0 && r.steve == Some(Steve::START)));
    // A bad Steve record (material kind out of the fork's 0..=15 bound) is dropped, not trusted.
    let bad = br#"{"player": [{"steve_state": {"mat_g1": 1, "mat_wood": 1, "mat_stone": 1, "mat_iron": 1, "mat_gold": 1,
        "mat_redstone": 1, "mat_diamond": 1, "sword_mat": "\u0010", "sword_durability": 1.0, "axe_mat": "\u0001",
        "axe_durability": 1.0, "pick_mat": "\u0001", "pick_durability": 1.0, "shovel_mat": "\u0001", "shovel_durability": 1.0}}]}"#;
    assert_eq!(SaveFile::parse(bad).unwrap().player[0].steve, None);
}

#[test]
fn state01_file_roundtrip_marker_reset() {
    let mut file = SaveFile::default();
    file.player[1] = captured(Some(3));
    file.player[1].marker = Marker::PosMove;
    let back = SaveFile::parse(file.to_pretty().as_bytes()).unwrap();
    assert_eq!(back.player[1].marker, Marker::NoAction);
    assert!(file.to_pretty().contains("\"state\": \"PosMove\""), "the marker is part of the written record");
}

// ---- the filesystem: two boots, a corrupt file, a write failure --------------------------------------

fn saved_file() -> SaveFile {
    let mut file = SaveFile::default();
    file.player[0] = captured(Some(5));
    file.cpu[0] = SavedState { x: 30.0, fighter_kind: 7, ..SavedState::never_saved() };
    file
}

#[test]
fn state01_two_boots_read_what_the_first_wrote() {
    let tmp = TempDir::new();
    let path = tmp.0.join("TrainingRooms").join("save_states.json");
    let (first, outcome) = persist::load(&path);
    assert_eq!(outcome, Outcome::Missing);
    assert_eq!(first, SaveFile::default());
    persist::save(&path, &saved_file()).unwrap();
    // A fresh process: reads it back, byte for byte the same table.
    let (second, outcome) = persist::load(&path);
    assert_eq!(outcome, Outcome::Read);
    assert_eq!(second, saved_file());
    let names: Vec<String> = fs::read_dir(path.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["save_states.json"], "no temp or backup left");
}

#[test]
fn state01_a_corrupt_file_is_copied_not_moved_and_survives_every_boot() {
    let tmp = TempDir::new();
    let path = tmp.0.join("save_states.json");
    fs::write(&path, b"{ not json").unwrap();
    for boot in 0..3 {
        let (file, outcome) = persist::load(&path);
        assert_eq!(file, SaveFile::default(), "boot {boot}");
        let Outcome::Defaulted { why: Defaulted::NotJson, kept: Some(copy) } = outcome else { panic!("boot {boot}: {outcome:?}") };
        assert_eq!(fs::read(&copy).unwrap(), b"{ not json");
        assert_eq!(fs::read(&path).unwrap(), b"{ not json", "boot {boot}: the active file is where it was");
    }
    let copies = fs::read_dir(&tmp.0).unwrap().count();
    assert_eq!(copies, 2, "the same bytes are kept once");
    // A Save replaces the active file atomically and the copy stays.
    persist::save(&path, &saved_file()).unwrap();
    assert_eq!(persist::load(&path).0, saved_file());
    assert_eq!(fs::read(tmp.0.join("save_states.invalid-1.json")).unwrap(), b"{ not json");
}

#[test]
fn state01_a_failed_write_leaves_the_previous_file_and_the_next_load_reads_it() {
    let tmp = TempDir::new();
    let path = tmp.0.join("save_states.json");
    persist::save(&path, &saved_file()).unwrap();
    let before = fs::read(&path).unwrap();
    // A directory on the temp path makes the new bytes impossible to write.
    fs::create_dir(tmp.0.join("save_states.json.tmp")).unwrap();
    let mut changed = saved_file();
    changed.player[2] = captured(Some(9));
    assert!(persist::save(&path, &changed).is_err());
    assert_eq!(fs::read(&path).unwrap(), before, "the active file is untouched");
    let (loaded, outcome) = persist::load(&path);
    assert_eq!((outcome, loaded), (Outcome::Read, saved_file()));
    // Once the obstacle is gone the same Save goes through.
    fs::remove_dir(tmp.0.join("save_states.json.tmp")).unwrap();
    persist::save(&path, &changed).unwrap();
    assert_eq!(persist::load(&path).0, changed);
}

#[test]
fn state01_an_oversize_or_unreadable_file_is_left_alone() {
    let tmp = TempDir::new();
    let path = tmp.0.join("save_states.json");
    fs::write(&path, vec![b' '; persist::MAX_BYTES as usize + 1]).unwrap();
    let (file, outcome) = persist::load(&path);
    assert_eq!((outcome, file), (Outcome::Unreadable, SaveFile::default()));
    assert_eq!(fs::metadata(&path).unwrap().len(), persist::MAX_BYTES + 1);
}
