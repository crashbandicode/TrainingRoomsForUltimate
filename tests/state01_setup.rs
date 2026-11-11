//! The portable setup library (SPEC-state-01 §6.9, the fork's `practice_setup` file format). The tests
//! are the unit tests of the fork's `training_mod_consts/src/practice_setup.rs` (stage m5o, ours-new;
//! CARRY-OVER.md) ported to this crate's types, plus the filesystem rules (two loads, a failed write).

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use model::state::held_item::HeldItem;
use model::state::record::{Charge, Marker, Robin, Steve};
use model::state::setup::*;
use model::store::StoreError;
use serde_json::{json, Value};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!("training_rooms-setup-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

fn exact_mii() -> Value {
    json!({"canonical": {"class": "Brawler"}, "charinfo": [7, 7, 7], "tags": [1, 2, 14000000000000000000u64]})
}

fn charge() -> Charge {
    Charge { int_x: Some(3), float_y: Some(1.5), has_charge: Some(true), ..Charge::default() }
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

fn player() -> FighterSetup {
    FighterSetup {
        fighter_kind: 0x07,
        x: -12.5,
        y: 0.0,
        percent: 42.5,
        lr: -1.0,
        situation_kind: 0,
        charge: charge(),
        steve_state: None,
        held_item: Some(HeldItem::Item { choice: 25, variation: 7, lifetime: 100, lifetime_max: 600, fuse: -1, fuse_max: -1 }),
        mii: None,
    }
}

fn cpu() -> FighterSetup {
    FighterSetup {
        fighter_kind: 0x48,
        x: 30.0,
        y: 12.0,
        percent: 0.0,
        lr: 1.0,
        situation_kind: 2,
        charge: Charge::default(),
        steve_state: None,
        held_item: None,
        mii: Some(exact_mii()),
    }
}

fn setup(name: &str) -> SetupRecord {
    SetupRecord::new(name, "13.0.5", Some(0x40), player(), cpu()).unwrap()
}

#[test]
fn json_is_versioned_values_only_and_round_trips() {
    let record = setup("Fox ledge drill");
    let value: Value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["schema_version"], json!(1));
    assert_eq!(value["name"], json!("Fox ledge drill"));
    assert_eq!(value["stage_id"], json!(0x40));
    for role in ["player", "cpu"] {
        let half = value[role].as_object().unwrap();
        assert!(!half.contains_key("state"), "{role}: no restore machine state");
        for key in ["fighter_kind", "x", "y", "percent", "lr", "situation_kind", "charge"] {
            assert!(half.contains_key(key), "{role}.{key}");
        }
    }
    assert_eq!(value["player"]["held_item"]["state"], json!("item"));
    assert!(value["player"].get("mii").is_none(), "absent identity is omitted, not null");
    assert!(value["player"].get("steve_state").is_none());
    assert!(value["cpu"]["mii"].is_object());
    assert!(value["cpu"].get("held_item").is_none());
    let bytes = serde_json::to_vec(&record).unwrap();
    assert!((bytes.len() as u64) < MAX_SETUP_FILE_BYTES / 8, "{} bytes", bytes.len());
    assert_eq!(serde_json::from_slice::<SetupRecord>(&bytes).unwrap(), record);
    // The u64 Mii tag is not narrowed.
    assert!(String::from_utf8(bytes).unwrap().contains("14000000000000000000"));
    // Absent optional fields read back as None rather than failing.
    let mut trimmed = value.clone();
    trimmed.as_object_mut().unwrap().remove("stage_id");
    let parsed: SetupRecord = serde_json::from_value(trimmed).unwrap();
    assert_eq!(parsed.stage_id, None);
    assert_eq!(parsed.player, record.player);
    assert_eq!(parsed.cpu, record.cpu);
}

#[test]
fn every_mii_class_requires_capture_time_identity_for_portable_setups() {
    for kind in 0x48..=0x4a {
        for role in [Role::Player, Role::Cpu] {
            let missing = FighterSetup { fighter_kind: kind, mii: None, ..player() };
            assert!(matches!(
                missing.validate(role),
                Err(SetupError::InvalidFighter { why: "exact Mii identity missing; save this slot again", .. })
            ));
        }
    }
    assert!(player().validate(Role::Player).is_ok());
    assert!(cpu().validate(Role::Cpu).is_ok());
}

#[test]
fn library_round_trip_listing_and_stable_ids() {
    let tmp = TempDir::new();
    let dir = tmp.path().join("setups");
    let record = setup("Fox ledge drill");
    let id = SetupLibrary::new(&dir).save_new(&record).unwrap();
    assert_eq!(id, "fox-ledge-drill");
    // A new instance on the same path, as after a game restart.
    let reopened = SetupLibrary::new(&dir);
    assert_eq!(reopened.load(&id).unwrap(), record);
    let listing: SetupListing = reopened.list().unwrap();
    assert_eq!(
        listing.entries,
        vec![SetupEntry {
            id: id.clone(),
            name: "Fox ledge drill".to_owned(),
            game_version: "13.0.5".to_owned(),
            player_kind: 0x07,
            cpu_kind: 0x48,
            stage_id: Some(0x40),
            exact_mii: [false, true],
        }]
    );
    assert!(listing.skipped.is_empty());
    assert_eq!(file_names(&dir), ["fox-ledge-drill.json"], "no temp or backup left");
    // A rename through overwrite keeps the id and both fighters.
    let mut renamed = reopened.load(&id).unwrap();
    renamed.set_name("  Fox ledge drill v2 ").unwrap();
    reopened.overwrite(&id, &renamed).unwrap();
    let after = reopened.load(&id).unwrap();
    assert_eq!(after.name, "Fox ledge drill v2");
    assert_eq!(after.player, record.player);
    assert_eq!(after.cpu, record.cpu);
    assert_eq!(reopened.save_new(&record).unwrap(), "fox-ledge-drill-2");
}

fn case(mutate: fn(&mut SetupRecord), role: Role, why: &'static str) -> (fn(&mut SetupRecord), Role, &'static str) {
    (mutate, role, why)
}

fn steve() -> Steve {
    Steve::START
}

#[test]
fn fighter_validation_names_the_role_and_rejects_bad_values() {
    let cases = vec![
        case(|s| s.cpu.fighter_kind = -1, Role::Cpu, "fighter kind"),
        case(|s| s.player.fighter_kind = MAX_FIGHTER_KIND + 1, Role::Player, "fighter kind"),
        case(|s| s.player.lr = 0.5, Role::Player, "facing"),
        case(|s| s.cpu.lr = f32::NAN, Role::Cpu, "facing"),
        case(|s| s.player.percent = -0.5, Role::Player, "damage"),
        case(|s| s.cpu.percent = 1000.5, Role::Cpu, "damage"),
        case(|s| s.player.x = f32::NAN, Role::Player, "x position"),
        case(|s| s.cpu.y = f32::INFINITY, Role::Cpu, "y position"),
        case(|s| s.player.charge.float_x = Some(f32::NAN), Role::Player, "float_x"),
        case(
            |s| s.cpu.charge = Charge { robin: Some(Robin { red_book: f32::NAN, ..robin().robin.unwrap() }), ..Charge::default() },
            Role::Cpu,
            "red book",
        ),
        case(|s| s.player.steve_state = Some(Steve { mat_wood: -1, ..steve() }), Role::Player, "material count"),
        case(|s| s.player.steve_state = Some(Steve { axe_durability: f32::NAN, ..steve() }), Role::Player, "durability"),
        case(|s| s.player.steve_state = Some(Steve { pick_mat: 200, ..steve() }), Role::Player, "material kind"),
        case(
            |s| {
                s.player.held_item =
                    Some(HeldItem::Item { choice: 11, variation: 3, lifetime: 0, lifetime_max: 0, fuse: -1, fuse_max: -1 })
            },
            Role::Player,
            "held item",
        ),
    ];
    for (mutate, role, why) in cases {
        let mut record = setup("Case");
        mutate(&mut record);
        let err = record.validate().unwrap_err();
        assert!(matches!(err, SetupError::InvalidFighter { role: found, .. } if found == role), "{why}: {err}");
        let text = err.to_string();
        assert!(text.contains(why) && text.contains(&role.to_string()), "{text}");
    }
    let mut steve_player = player();
    steve_player.fighter_kind = 0x58;
    steve_player.steve_state = Some(steve());
    steve_player.charge = robin();
    assert!(steve_player.validate(Role::Player).is_ok());
    let mut mii_player = player();
    mii_player.fighter_kind = 0x48;
    mii_player.mii = Some(exact_mii());
    assert!(mii_player.validate(Role::Player).is_ok());
}

#[test]
fn record_validation_covers_schema_name_version_and_stage() {
    let mut record = setup("Ok");
    record.schema_version = 2;
    let err = record.validate().unwrap_err();
    assert!(matches!(err, SetupError::Store(StoreError::UnsupportedVersion { found: 2 })));
    assert_eq!(err.to_string(), "unsupported setup schema version 2 (supported: 1)");
    for (name, why) in [("   ", "empty"), ("x\ny", "control")] {
        assert!(matches!(
            SetupRecord::new(name, "13.0.5", None, player(), cpu()),
            Err(SetupError::Invalid(found)) if found.contains(why)
        ));
    }
    let long = "a".repeat(MAX_NAME_CHARS + 1);
    assert!(matches!(SetupRecord::new(long.as_str(), "13.0.5", None, player(), cpu()), Err(SetupError::Invalid("name is too long"))));
    let version = "v".repeat(MAX_GAME_VERSION_BYTES + 1);
    assert!(matches!(SetupRecord::new("Ok", version.as_str(), None, player(), cpu()), Err(SetupError::Invalid("game version is too long"))));
    assert!(matches!(SetupRecord::new("Ok", "13.0.5", Some(-1), player(), cpu()), Err(SetupError::Invalid("stage id is negative"))));
    // Trimming, a 48-scalar name, an empty version and no stage are fine.
    let padded = format!("  {}  ", "é".repeat(MAX_NAME_CHARS));
    let mut ok = SetupRecord::new(padded, "", None, player(), cpu()).unwrap();
    assert_eq!(ok.name.chars().count(), MAX_NAME_CHARS);
    assert_eq!(ok.stage_id, None);
    assert!(matches!(ok.set_name("  "), Err(SetupError::Invalid("name is empty"))));
    assert_eq!(ok.name.chars().count(), MAX_NAME_CHARS, "a rejected rename keeps the name");
    assert_eq!(ok.role(Role::Cpu).fighter_kind, 0x48);
    assert_eq!(ok.role(Role::Player).fighter_kind, 0x07);
}

#[test]
fn store_reports_version_size_and_corruption_and_touches_nothing() {
    let tmp = TempDir::new();
    let library = SetupLibrary::new(tmp.path());
    let good_id = library.save_new(&setup("Good")).unwrap();
    let mut future = serde_json::to_value(setup("Future")).unwrap();
    future["schema_version"] = json!(2);
    let bad: Vec<(&str, Vec<u8>)> = vec![
        ("future.json", serde_json::to_vec(&future).unwrap()),
        ("big.json", vec![b' '; MAX_SETUP_FILE_BYTES as usize + 1]),
        ("garbage.json", b"{ not json".to_vec()),
        ("Bad Name.json", serde_json::to_vec(&setup("Bad")).unwrap()),
    ];
    for (name, bytes) in &bad {
        fs::write(tmp.path().join(name), bytes).unwrap();
    }
    assert!(matches!(library.load("future"), Err(SetupError::Store(StoreError::UnsupportedVersion { found: 2 }))));
    assert!(matches!(library.load("big"), Err(SetupError::Store(StoreError::FileTooLarge { max: MAX_SETUP_FILE_BYTES, .. }))));
    assert!(matches!(library.load("garbage"), Err(SetupError::Store(StoreError::Parse(_)))));
    assert!(matches!(library.load("nothing"), Err(SetupError::Store(StoreError::NotFound(_)))));
    assert_eq!(library.load("nothing").unwrap_err().to_string(), "no setup with id \"nothing\"");
    let listing = library.list().unwrap();
    let ids: Vec<&str> = listing.entries.iter().map(|entry| entry.id.as_str()).collect();
    assert_eq!(ids, [good_id.as_str()]);
    let reason = |file: &str| listing.skipped.iter().find(|s| s.file == file).unwrap().reason.clone();
    assert_eq!(reason("future.json"), "unsupported setup schema version 2 (supported: 1)");
    assert!(reason("big.json").contains("bytes"), "{}", reason("big.json"));
    assert!(reason("garbage.json").starts_with("malformed setup file"), "{}", reason("garbage.json"));
    assert!(reason("Bad Name.json").starts_with("invalid setup id"), "{}", reason("Bad Name.json"));
    for (name, bytes) in &bad {
        assert_eq!(&fs::read(tmp.path().join(name)).unwrap(), bytes, "{name}");
    }
    assert_eq!(library.load(&good_id).unwrap(), setup("Good"));
}

#[test]
fn overwrite_failure_keeps_the_previous_setup_and_a_second_load_reads_it() {
    let tmp = TempDir::new();
    let library = SetupLibrary::new(tmp.path());
    let id = library.save_new(&setup("Keep")).unwrap();
    let path = tmp.path().join(format!("{id}.json"));
    let before = fs::read(&path).unwrap();
    // Occupy the temp path with a directory so the new data cannot be written.
    fs::create_dir(tmp.path().join(format!("{id}.json.tmp"))).unwrap();
    assert!(matches!(library.overwrite(&id, &setup("Changed")), Err(SetupError::Store(StoreError::Io(_)))));
    assert_eq!(fs::read(&path).unwrap(), before);
    // A fresh library over the same folder (the next boot) reads the previous setup.
    assert_eq!(SetupLibrary::new(tmp.path()).load(&id).unwrap(), setup("Keep"));
    // An invalid replacement never reaches the disk either.
    let mut broken = setup("Broken");
    broken.player.lr = 0.0;
    assert!(matches!(library.overwrite(&id, &broken), Err(SetupError::InvalidFighter { role: Role::Player, .. })));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(matches!(library.overwrite("ghost", &setup("Ghost")), Err(SetupError::Store(StoreError::NotFound(_)))));
}

#[test]
fn a_setup_binds_to_a_slot_as_values_only() {
    let record = setup("Bind");
    let saved = record.player.to_saved(record.stage_id);
    assert_eq!(saved.marker, Marker::NoAction);
    assert_eq!(saved.saved_stage, Some(0x40));
    assert_eq!(FighterSetup::of_saved(&saved), record.player);
    let cpu_saved = record.cpu.to_saved(record.stage_id);
    assert_eq!(cpu_saved.mii, Some(exact_mii()));
}
