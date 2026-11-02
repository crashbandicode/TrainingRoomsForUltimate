//! SPEC-core-08, the first-boot import on real directories (the copy loop, the receipt, N6, N8, the capture
//! staging rule, interrupted and failed copies, a second boot). The plan's own rules are in
//! `tests/migration.rs`.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use model::migration::{self, ImportResult};
use model::migration_run::{run_background, run_blocking, status_word, Roots, Stage};
use model::sha256::sha256_hex;
use serde_json::Value;

struct Tmp(PathBuf);

impl Tmp {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("training_rooms-import-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }

    fn fork(&self) -> PathBuf {
        self.0.join("TrainingModpack")
    }

    fn rooms(&self) -> PathBuf {
        self.0.join("TrainingRooms")
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn saves(player_kind: i64, cpu_kind: i64) -> String {
    let rec = |kind: i64| format!(r#"{{"x": 1.5, "y": 0.0, "percent": 3.0, "lr": 1.0, "situation_kind": 0, "state": "NoAction", "fighter_kind": {kind}, "charge": {{}}, "steve_state": null}}"#);
    format!(r#"{{"player": [{}, {}], "cpu": [{}, {}]}}"#, rec(player_kind), rec(-1), rec(cpu_kind), rec(-1))
}

/// The fork's folder as the card has it: settings with `character_item` = Player 1st Var., both INIs,
/// saves, a Mii file, a setup, a macro, two published captures and one unpublished staging folder.
fn fork_folder(t: &Tmp) {
    let f = t.fork();
    write(&f.join("training_modpack_menu.conf"), br#"{"menu": {"character_item": [0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], "update_policy": [1,0]}, "defaults_menu": {}}"#);
    write(&f.join("training_modpack.json"), br#"{"update":{"last_update_version":"x"}}"#);
    write(&f.join("button_config.ini"), b"save_state_save=zl+down\n");
    write(&f.join("replay_capture.ini"), b"enabled=on\n");
    write(&f.join("save_states.json"), saves(0x0e, 0x07).as_bytes());
    write(&f.join("mii-presets.json"), b"[]");
    write(&f.join("setups/a.json"), b"{\"a\":1}");
    write(&f.join("macros/m.json"), b"{\"m\":1}");
    write(&f.join("replay_captures/11.smd/11.tmrc"), &vec![7u8; 200_000]);
    write(&f.join("replay_captures/11.smd/manifest.json"), b"{}");
    write(&f.join("replay_captures/22.smd/timeline.jsonl"), b"{\"t\":0}\n");
    write(&f.join("replay_captures/.33.smd-0.tmp/manifest.json"), b"half");
    write(&f.join("replay_captures/.33.smd-0.tmp/timeline.jsonl"), b"half");
    write(&f.join("cycle_preload.ini"), b"cache");
    write(&f.join("practice-diagnostics.log"), b"log");
    write(&f.join("save_states.toml"), b"legacy");
}

fn tree(root: &Path) -> Vec<(String, String)> {
    fn walk(path: &Path, rel: &str, out: &mut Vec<(String, String)>) {
        let mut entries: Vec<_> = fs::read_dir(path).unwrap().map(|e| e.unwrap()).collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let child = if rel.is_empty() { e.file_name().to_string_lossy().into_owned() } else { format!("{rel}/{}", e.file_name().to_string_lossy()) };
            if e.file_type().unwrap().is_dir() {
                walk(&e.path(), &child, out);
            } else {
                out.push((child, sha256_hex(&fs::read(e.path()).unwrap())));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, "", &mut out);
    out
}

fn full_import(t: &Tmp) -> (model::migration_run::Receipt, Result<(), String>) {
    let roots_source = t.fork();
    let roots_dest = t.rooms();
    let roots = Roots { source: &roots_source, dest: &roots_dest };
    let mut log = |_: &str| {};
    match run_blocking(roots, 5, "0.1.0", &mut log) {
        Stage::Imported { blocking, marker } => {
            if blocking.pending_captures() == 0 {
                assert!(marker.is_some(), "no captures: the marker is written with the blocking part");
                let r = blocking.receipt.clone();
                (r, marker.unwrap())
            } else {
                assert!(marker.is_none(), "the marker waits for the background part");
                run_background(roots, blocking, &mut log)
            }
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn migration_plan_splits_blocking_and_background() {
    let t = Tmp::new();
    fork_folder(&t);
    let mut log = |_: &str| {};
    let (source, dest) = (t.fork(), t.rooms());
    let Stage::Imported { blocking, marker } = run_blocking(Roots { source: &source, dest: &dest }, 1, "0.1.0", &mut log) else { panic!() };
    // The small files are in place before the settings load; the captures are not copied yet; no marker.
    assert!(dest.join("settings.json").is_file() && dest.join("save_states.json").is_file());
    assert!(dest.join("button_config.ini").is_file() && dest.join("replay_capture.ini").is_file());
    assert!(dest.join("setups/a.json").is_file() && dest.join("macros/m.json").is_file() && dest.join("mii-presets.json").is_file());
    assert!(!dest.join("replay_captures").exists());
    assert_eq!(blocking.pending_captures(), 3, "two published bundles, three files");
    assert!(marker.is_none() && !dest.join("migration.json").exists());
    assert_eq!(status_word(&Stage::Imported { blocking, marker }), "done");
}

#[test]
fn migration_full_import_receipt_and_originals() {
    let t = Tmp::new();
    fork_folder(&t);
    let before = tree(&t.fork());
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(tree(&t.fork()), before, "the fork's folder is byte-identical afterwards");
    assert_eq!(receipt.result(), ImportResult::Done);
    assert_eq!(receipt.skipped_staging, 1, "the staging folder is counted once");
    // The staging folder was never copied, and nothing outside the whitelist was.
    let dest = tree(&t.rooms());
    assert!(!dest.iter().any(|(p, _)| p.contains(".tmp") || p == "cycle_preload.ini" || p == "training_modpack.json" || p.ends_with(".toml") || p.ends_with(".log")), "{dest:?}");
    for must in [
        "settings.json",
        "button_config.ini",
        "replay_capture.ini",
        "save_states.json",
        "mii-presets.json",
        "setups/a.json",
        "macros/m.json",
        "replay_captures/11.smd/11.tmrc",
        "replay_captures/11.smd/manifest.json",
        "replay_captures/22.smd/timeline.jsonl",
        "migration.json",
    ] {
        assert!(dest.iter().any(|(p, _)| p == must), "{must} missing in {dest:?}");
    }
    // The marker: the SPEC shape, the hashes of what was copied.
    let marker: Value = serde_json::from_slice(&fs::read(t.rooms().join("migration.json")).unwrap()).unwrap();
    for key in ["source", "at", "plugin", "result", "files", "skipped"] {
        assert!(marker.get(key).is_some(), "{key}");
    }
    assert_eq!(marker["source"], "sd:/ultimate/TrainingModpack");
    assert_eq!(marker["result"], "done");
    assert_eq!(marker["skipped_staging"], 1);
    let files = marker["files"].as_array().unwrap();
    assert_eq!(files.len(), 10);
    let entry = files.iter().find(|f| f["from"].as_str().unwrap().ends_with("/button_config.ini")).unwrap();
    assert_eq!(entry["sha256"], sha256_hex(b"save_state_save=zl+down\n"));
    assert!(entry["to"].as_str().unwrap().ends_with("TrainingRooms/button_config.ini"));
    // settings.json is the fork's training_modpack_menu.conf, byte for byte.
    assert_eq!(fs::read(t.rooms().join("settings.json")).unwrap(), fs::read(t.fork().join("training_modpack_menu.conf")).unwrap());
}

#[test]
fn migration_item_on_load_converts_player_records_in_the_copy() {
    let t = Tmp::new();
    fork_folder(&t);
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.converted, 1, "only the saved Daisy record");
    let copy: Value = serde_json::from_slice(&fs::read(t.rooms().join("save_states.json")).unwrap()).unwrap();
    // Player 1st Var. of Daisy: the Smile Turnip (choice 2), exact lifetime fields -1 so the timers are skipped.
    assert_eq!(copy["player"][0]["held_item"]["choice"], 2);
    assert_eq!(copy["player"][0]["held_item"]["lifetime_max"], -1);
    assert!(copy["player"][1].get("held_item").is_none_or(Value::is_null), "never-saved records get none");
    assert!(copy["cpu"][0].get("held_item").is_none(), "CPU records are untouched");
    let original: Value = serde_json::from_slice(&fs::read(t.fork().join("save_states.json")).unwrap()).unwrap();
    assert!(original["player"][0].get("held_item").is_none(), "the original has none");
    // Everything else is the fork's record, unchanged.
    let mut stripped = copy.clone();
    stripped["player"][0].as_object_mut().unwrap().remove("held_item");
    assert_eq!(stripped, original);
    // The settings copy still carries the retired key: it is ignored on load, never rewritten here.
    let marker: Value = serde_json::from_slice(&fs::read(t.rooms().join("migration.json")).unwrap()).unwrap();
    assert_eq!(marker["converted_item_snapshots"], 1);
}

#[test]
fn migration_item_on_load_never_overwrites_a_snapshot_or_an_existing_save() {
    let t = Tmp::new();
    fork_folder(&t);
    // A fork save that already has an `empty` snapshot keeps it.
    let with_item = saves(0x0e, 0x07).replacen("\"steve_state\": null", "\"steve_state\": null, \"held_item\": {\"state\": \"empty\"}", 1);
    write(&t.fork().join("save_states.json"), with_item.as_bytes());
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.converted, 0);
    let copy: Value = serde_json::from_slice(&fs::read(t.rooms().join("save_states.json")).unwrap()).unwrap();
    assert_eq!(copy["player"][0]["held_item"]["state"], "empty");
    // A destination that exists is kept as it is (and not converted: only the copy is converted).
    let t2 = Tmp::new();
    fork_folder(&t2);
    write(&t2.rooms().join("save_states.json"), b"{\"player\": [], \"cpu\": []}");
    let (receipt, marker) = full_import(&t2);
    marker.unwrap();
    assert_eq!(receipt.converted, 0);
    assert_eq!(fs::read(t2.rooms().join("save_states.json")).unwrap(), b"{\"player\": [], \"cpu\": []}");
    assert!(receipt.skipped.iter().any(|(p, r)| p == "save_states.json" && r == "exists"));
}

#[test]
fn migration_developer_ini_does_not_mark_imported() {
    let t = Tmp::new();
    fork_folder(&t);
    write(&t.rooms().join("developer.ini"), b"developer_page=on\n");
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.result(), ImportResult::Done, "the import ran completely");
    assert!(t.rooms().join("settings.json").is_file() && t.rooms().join("replay_captures/22.smd/timeline.jsonl").is_file());
    assert_eq!(fs::read(t.rooms().join("developer.ini")).unwrap(), b"developer_page=on\n", "left byte-identical");
    assert!(t.rooms().join("migration.json").is_file());
}

#[test]
fn migration_marker_blocks_a_second_boot() {
    let t = Tmp::new();
    fork_folder(&t);
    let (_, marker) = full_import(&t);
    marker.unwrap();
    let after_first = tree(&t.rooms());
    // The player deletes a copied file: nothing is re-imported while the marker exists.
    fs::remove_file(t.rooms().join("button_config.ini")).unwrap();
    let (source, dest) = (t.fork(), t.rooms());
    let stage = run_blocking(Roots { source: &source, dest: &dest }, 9, "0.1.0", &mut |_| {});
    assert!(matches!(stage, Stage::MarkerPresent));
    assert_eq!(status_word(&stage), "skipped");
    assert!(!t.rooms().join("button_config.ini").exists());
    assert_eq!(tree(&t.rooms()).len() + 1, after_first.len());
}

#[test]
fn migration_no_source_writes_a_none_marker() {
    let t = Tmp::new();
    let (source, dest) = (t.fork(), t.rooms());
    let stage = run_blocking(Roots { source: &source, dest: &dest }, 3, "0.1.0", &mut |_| {});
    assert_eq!(status_word(&stage), "none");
    let marker: Value = serde_json::from_slice(&fs::read(t.rooms().join("migration.json")).unwrap()).unwrap();
    assert_eq!((marker["result"].as_str(), marker["files"].as_array().map(Vec::len)), (Some("none"), Some(0)));
}

#[test]
fn migration_existing_destinations_are_never_overwritten() {
    let t = Tmp::new();
    fork_folder(&t);
    write(&t.rooms().join("button_config.ini"), b"mine\n");
    write(&t.rooms().join("setups/a.json"), b"mine");
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(fs::read(t.rooms().join("button_config.ini")).unwrap(), b"mine\n");
    assert_eq!(fs::read(t.rooms().join("setups/a.json")).unwrap(), b"mine");
    assert_eq!(receipt.skipped.iter().filter(|(_, r)| r == "exists").count(), 2);
}

#[test]
fn migration_captures_have_no_size_cap_and_the_rest_do() {
    let t = Tmp::new();
    fork_folder(&t);
    // 8 MiB + 1 under replay_captures/ is copied; the same size elsewhere is skipped.
    let big = vec![1u8; 8 * 1024 * 1024 + 1];
    write(&t.fork().join("replay_captures/44.smd/timeline.jsonl"), &big);
    write(&t.fork().join("macros/big.json"), &big);
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(fs::metadata(t.rooms().join("replay_captures/44.smd/timeline.jsonl")).unwrap().len(), big.len() as u64);
    assert!(!t.rooms().join("macros/big.json").exists());
    assert!(receipt.skipped.iter().any(|(p, r)| p == "macros/big.json" && r == "too_large"));
}

#[test]
fn migration_staging_folders_are_skipped_and_never_touched() {
    let t = Tmp::new();
    fork_folder(&t);
    write(&t.fork().join("replay_captures/.55.smd-12.tmp/manifest.json"), b"x");
    // The staging folders are made unreadable: if the import tried to read them it would report an error.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for name in [".33.smd-0.tmp", ".55.smd-12.tmp"] {
            fs::set_permissions(t.fork().join("replay_captures").join(name), fs::Permissions::from_mode(0o000)).unwrap();
        }
    }
    let (receipt, marker) = full_import(&t);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for name in [".33.smd-0.tmp", ".55.smd-12.tmp"] {
            fs::set_permissions(t.fork().join("replay_captures").join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    marker.unwrap();
    assert_eq!(receipt.skipped_staging, 2);
    assert_eq!(receipt.result(), ImportResult::Done);
    assert!(!t.rooms().join("replay_captures").join(".33.smd-0.tmp").exists());
    let m: Value = serde_json::from_slice(&fs::read(t.rooms().join("migration.json")).unwrap()).unwrap();
    assert_eq!(m["skipped_staging"], 2);
    assert!(migration::is_staging("replay_captures/.id.smd-3.tmp/x") && !migration::is_staging("replay_captures/id.smd/x"));
}

#[test]
fn migration_io_error_is_partial_and_the_rest_is_copied() {
    let t = Tmp::new();
    fork_folder(&t);
    // A directory sits on the destination's temp name: that one copy cannot be written.
    fs::create_dir_all(t.rooms().join("button_config.ini.part")).unwrap();
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.result(), ImportResult::Partial);
    assert!(receipt.skipped.iter().any(|(p, r)| p == "button_config.ini" && r.starts_with("io:")), "{:?}", receipt.skipped);
    assert!(!t.rooms().join("button_config.ini").exists(), "no truncated file");
    assert!(t.rooms().join("settings.json").is_file() && t.rooms().join("replay_captures/22.smd/timeline.jsonl").is_file());
    let m: Value = serde_json::from_slice(&fs::read(t.rooms().join("migration.json")).unwrap()).unwrap();
    assert_eq!(m["result"], "partial");
}

#[test]
fn migration_unreadable_source_file_is_partial() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let t = Tmp::new();
        fork_folder(&t);
        fs::set_permissions(t.fork().join("mii-presets.json"), fs::Permissions::from_mode(0o000)).unwrap();
        let (receipt, marker) = full_import(&t);
        fs::set_permissions(t.fork().join("mii-presets.json"), fs::Permissions::from_mode(0o644)).unwrap();
        marker.unwrap();
        if receipt.result() == ImportResult::Done {
            return; // running as a user that can read anything (root): the case cannot be made
        }
        assert_eq!(receipt.result(), ImportResult::Partial);
        assert!(!t.rooms().join("mii-presets.json").exists());
        assert!(t.rooms().join("settings.json").is_file());
    }
}

#[test]
fn migration_interrupted_background_copy_reruns_and_completes() {
    let t = Tmp::new();
    fork_folder(&t);
    let (source, dest) = (t.fork(), t.rooms());
    let roots = Roots { source: &source, dest: &dest };
    // First boot: the game quits during the background part: the marker is not written, and a half-written
    // `.part` is left behind.
    let Stage::Imported { blocking, .. } = run_blocking(roots, 1, "0.1.0", &mut |_| {}) else { panic!() };
    assert!(blocking.pending_captures() > 0);
    drop(blocking);
    write(&dest.join("replay_captures/11.smd/11.tmrc.part"), b"truncated");
    assert!(!dest.join("migration.json").exists());
    // Second boot: the blocking files exist (kept), the captures are copied, the leftover `.part` is replaced.
    let Stage::Imported { blocking, .. } = run_blocking(roots, 2, "0.1.0", &mut |_| {}) else { panic!() };
    assert!(blocking.receipt.skipped.iter().any(|(_, r)| r == "exists"), "the files copied the first time are kept");
    let (receipt, marker) = run_background(roots, blocking, &mut |_| {});
    marker.unwrap();
    assert_eq!(fs::read(dest.join("replay_captures/11.smd/11.tmrc")).unwrap(), vec![7u8; 200_000]);
    assert!(!dest.join("replay_captures/11.smd/11.tmrc.part").exists());
    assert!(dest.join("migration.json").is_file());
    assert!(receipt.files.iter().any(|f| f.from.ends_with("22.smd/timeline.jsonl")));
}

#[test]
fn migration_marker_write_failure_leaves_the_import_to_run_again() {
    let t = Tmp::new();
    fork_folder(&t);
    // A directory on the temp name of the marker makes its write fail.
    fs::create_dir_all(t.rooms().join("migration.json.tmp")).unwrap();
    let (_, marker) = full_import(&t);
    assert!(marker.is_err(), "the marker could not be written");
    assert!(!t.rooms().join("migration.json").exists());
    // Next boot (the obstacle is gone): the import runs again, keeping what exists, and finishes.
    fs::remove_dir_all(t.rooms().join("migration.json.tmp")).unwrap();
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert!(receipt.skipped.iter().any(|(_, r)| r == "exists"));
    assert!(t.rooms().join("migration.json").is_file());
}

#[test]
fn migration_receipt_hashes_the_converted_copy_separately() {
    let t = Tmp::new();
    fork_folder(&t);
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    let saves = receipt.files.iter().find(|f| f.from.ends_with("/save_states.json")).unwrap();
    assert_eq!(saves.source_sha256, sha256_hex(&fs::read(t.fork().join("save_states.json")).unwrap()));
    assert_eq!(saves.sha256, sha256_hex(&fs::read(t.rooms().join("save_states.json")).unwrap()));
    assert_ne!(saves.sha256, saves.source_sha256, "the copy carries the converted snapshot");
}
