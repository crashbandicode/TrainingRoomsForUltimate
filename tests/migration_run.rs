//! SPEC-core-08, the first-boot import on real directories (the copy loop, the receipt, N6, N8, the capture
//! staging rule, interrupted and failed copies, a second boot). The plan's own rules are in
//! `tests/migration.rs`.

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
fn migration_cpu_item_selection_keeps_never_saved_player_absent() {
    let t = Tmp::new();
    fork_folder(&t);
    let mut settings: Value = serde_json::from_slice(&fs::read(t.fork().join("training_modpack_menu.conf")).unwrap()).unwrap();
    let mut selection = vec![0; 17];
    selection[9] = 1; // CPU 1st Var.; the same-slot R.O.B. resolves to Gyro, not the player's kind.
    settings["menu"]["character_item"] = serde_json::json!(selection);
    write(&t.fork().join("training_modpack_menu.conf"), &serde_json::to_vec(&settings).unwrap());
    let mut source: Value = serde_json::from_str(&saves(-1, 0x2d)).unwrap();
    source["player"][1]["fighter_kind"] = 7.into();
    source["cpu"][1]["fighter_kind"] = 0x2d.into();
    write(&t.fork().join("save_states.json"), &serde_json::to_vec(&source).unwrap());
    let original = tree(&t.fork());
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.converted, 1, "only the saved player, even though both CPU kinds resolve");
    let mut expected = source;
    expected["player"][1]["held_item"] = serde_json::json!({
        "state": "item", "choice": 29, "variation": 0,
        "lifetime": -1, "lifetime_max": -1, "fuse": -1, "fuse_max": -1
    });
    let copied: Value = serde_json::from_slice(&fs::read(t.rooms().join("save_states.json")).unwrap()).unwrap();
    assert_eq!(copied, expected, "never-saved player and every CPU record stay byte-semantically unchanged");
    assert_eq!(tree(&t.fork()), original, "source bytes untouched");
    let saved = tree(&t.rooms());
    let (source, dest) = (t.fork(), t.rooms());
    assert!(matches!(run_blocking(Roots { source: &source, dest: &dest }, 9, "0.1.0", &mut |_| {}), Stage::MarkerPresent));
    assert_eq!(tree(&dest), saved, "second boot does not rewrite imported data");
    let marker: Value = serde_json::from_slice(&fs::read(dest.join("migration.json")).unwrap()).unwrap();
    assert_eq!(marker["converted_item_snapshots"], 1);
}

#[test]
fn migration_cpu_item_selection_survives_fresh_process() {
    const NAME: &str = "migration_cpu_item_selection_survives_fresh_process";
    const ROOT: &str = "TR_N6_PROCESS_ROOT";
    const PHASE: &str = "TR_N6_PROCESS_PHASE";
    let stamp = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos().to_string();
    if let Some(root) = std::env::var_os(ROOT) {
        let root = PathBuf::from(root);
        let (source, dest) = (root.join("TrainingModpack"), root.join("TrainingRooms"));
        let phase = std::env::var(PHASE).unwrap();
        let started = stamp();
        let expected: Value = serde_json::from_slice(&fs::read(root.join("expected.json")).unwrap()).unwrap();
        if phase == "save" {
            let Stage::Imported { blocking, marker } = run_blocking(Roots { source: &source, dest: &dest }, 16, "0.1.0", &mut |_| {}) else { panic!("import did not run") };
            assert_eq!(blocking.pending_captures(), 0);
            marker.unwrap().unwrap();
            assert_eq!(blocking.receipt.converted, 1);
            let actual: Value = serde_json::from_slice(&fs::read(dest.join("save_states.json")).unwrap()).unwrap();
            assert_eq!(actual, expected["copy"], "validate exact persisted fixture, not a recreated seed");
        } else {
            assert_eq!(phase, "restore");
            let (mut restored, outcome) = model::state::persist::load(&dest.join("save_states.json"));
            assert_eq!(outcome, model::state::persist::Outcome::Read);
            assert_eq!(restored.player[0].fighter_kind, -1);
            assert_eq!(restored.player[0].held_item, None);
            assert_eq!(restored.player[1].held_item, Some(model::state::held_item::HeldItem::Item {
                choice: 29, variation: 7, lifetime: -1, lifetime_max: -1, fuse: -1, fuse_max: -1
            }));
            assert!(restored.cpu.iter().all(|record| record.held_item.is_none()));
            // Continue with a different legacy variation: neither a conversion nor boot may overwrite it.
            let before = restored.clone();
            assert_eq!(model::state::convert::convert_file(&mut restored, 9), 0);
            assert_eq!(restored, before);
            assert!(matches!(run_blocking(Roots { source: &source, dest: &dest }, 17, "0.1.0", &mut |_| {}), Stage::MarkerPresent));
        }
        let source_hash = sha256_hex(&fs::read(source.join("save_states.json")).unwrap());
        assert_eq!(source_hash, expected["source_sha256"].as_str().unwrap());
        let receipt = serde_json::json!({
            "phase": phase, "pid": std::process::id(), "started_ns": started, "finished_ns": stamp(),
            "save_sha256": sha256_hex(&fs::read(dest.join("save_states.json")).unwrap()),
            "marker_sha256": sha256_hex(&fs::read(dest.join("migration.json")).unwrap()), "source_sha256": source_hash
        });
        write(&root.join(format!("{phase}-process.json")), &serde_json::to_vec(&receipt).unwrap());
        return;
    }
    let t = Tmp::new();
    let mut source: Value = serde_json::from_str(&saves(-1, 0x2d)).unwrap();
    source["player"][1]["fighter_kind"] = 7.into();
    source["cpu"][1]["fighter_kind"] = 0x2d.into();
    let bytes = serde_json::to_vec(&source).unwrap();
    write(&t.fork().join("save_states.json"), &bytes);
    let mut selection = vec![0; 17];
    selection[16] = 1;
    write(&t.fork().join("training_modpack_menu.conf"), &serde_json::to_vec(&serde_json::json!({"menu": {"character_item": selection}})).unwrap());
    source["player"][1]["held_item"] = serde_json::json!({"state":"item", "choice":29, "variation":7, "lifetime":-1, "lifetime_max":-1, "fuse":-1, "fuse_max":-1});
    write(&t.0.join("expected.json"), &serde_json::to_vec(&serde_json::json!({"copy": source, "source_sha256": sha256_hex(&bytes)})).unwrap());
    let exe = std::env::current_exe().unwrap();
    let launch = |phase| {
        let start = stamp();
        let result = std::process::Command::new(&exe).args(["--exact", NAME, "--nocapture"])
            .env(ROOT, &t.0).env(PHASE, phase).output().unwrap();
        assert!(result.status.success(), "{phase}: {}{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
        (start, stamp())
    };
    let (save_started, save_exited) = launch("save");
    let (restore_started, restore_exited) = launch("restore");
    let load = |phase| serde_json::from_slice::<Value>(&fs::read(t.0.join(format!("{phase}-process.json"))).unwrap()).unwrap();
    let (saved, restored) = (load("save"), load("restore"));
    assert_ne!(saved["pid"], restored["pid"]);
    assert!(save_exited.parse::<u128>().unwrap() <= restore_started.parse::<u128>().unwrap());
    for key in ["save_sha256", "marker_sha256", "source_sha256"] {
        assert_eq!(saved[key], restored[key], "durable {key} unchanged after fresh-process continuation");
    }
    println!("N6 fresh-process evidence {}", serde_json::json!({"save": saved, "restore": restored,
        "save_launch_ns":save_started, "save_exit_observed_ns":save_exited,
        "restore_launch_ns":restore_started, "restore_exit_observed_ns":restore_exited,
        "saver_success_before_restore_launch":true, "equal_persisted_hashes":true}));
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

#[cfg(unix)]
#[test]
fn core08_existing_dangling_destination_is_preserved_not_replaced() {
    let t = Tmp::new();
    fork_folder(&t);
    fs::create_dir_all(t.rooms()).unwrap();
    let target = t.rooms().join("button_config.ini");
    std::os::unix::fs::symlink("user-owned-missing.ini", &target).unwrap();
    let before = tree(&t.fork());
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(fs::read_link(&target).unwrap(), Path::new("user-owned-missing.ini"),
        "an occupied destination is not absent just because its referent is missing");
    assert!(receipt.skipped.iter().any(|(p, r)| p == "button_config.ini" && r == "exists"));
    assert_eq!(tree(&t.fork()), before);
}

#[cfg(unix)]
#[test]
fn n8_destination_appearing_after_blocking_is_preserved_at_publication() {
    let t = Tmp::new();
    write(&t.fork().join("replay_captures/11.smd/11.tmrc"), b"complete source capture");
    let (source, dest) = (t.fork(), t.rooms());
    let roots = Roots { source: &source, dest: &dest };
    let Stage::Imported { blocking, marker } = run_blocking(roots, 1, "0.1.0", &mut |_| {}) else { panic!() };
    assert!(marker.is_none() && blocking.pending_captures() == 1);
    let target = dest.join("replay_captures/11.smd/11.tmrc");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink("user-owned-missing.tmrc", &target).unwrap();
    let (receipt, marker) = run_background(roots, blocking, &mut |_| {});
    marker.unwrap();
    assert_eq!(fs::read_link(&target).unwrap(), Path::new("user-owned-missing.tmrc"));
    assert!(receipt.files.is_empty());
    assert_eq!(receipt.skipped, [("replay_captures/11.smd/11.tmrc".to_owned(), "exists".to_owned())]);
    assert!(!target.with_file_name("11.tmrc.part").exists());
    assert_eq!(fs::read(source.join("replay_captures/11.smd/11.tmrc")).unwrap(), b"complete source capture");
}

#[cfg(unix)]
#[test]
fn core08_existing_marker_entry_blocks_import_even_without_a_referent() {
    let t = Tmp::new();
    fork_folder(&t);
    fs::create_dir_all(t.rooms()).unwrap();
    let marker = t.rooms().join("migration.json");
    std::os::unix::fs::symlink("user-owned-marker.json", &marker).unwrap();
    let (source, dest) = (t.fork(), t.rooms());
    let stage = run_blocking(Roots { source: &source, dest: &dest }, 1, "0.1.0", &mut |_| {});
    assert!(matches!(stage, Stage::MarkerPresent));
    assert_eq!(fs::read_link(marker).unwrap(), Path::new("user-owned-marker.json"));
    assert!(!dest.join("settings.json").exists());
}

#[cfg(unix)]
#[test]
fn core08_marker_inspection_error_defers_import_without_calling_it_present() {
    let t = Tmp::new();
    fork_folder(&t);
    std::os::unix::fs::symlink("TrainingRooms", t.rooms()).unwrap();
    let (source, dest) = (t.fork(), t.rooms());
    let mut messages = Vec::new();
    let stage = run_blocking(Roots { source: &source, dest: &dest }, 1, "0.1.0", &mut |s| messages.push(s.to_owned()));
    assert_eq!(status_word(&stage), "partial");
    let Stage::Imported { blocking, marker } = stage else { panic!("inspection error must not be marker-present or no-source") };
    assert!(marker.unwrap().is_err() && blocking.marker_withheld());
    assert!(blocking.receipt.files.is_empty() && blocking.pending_captures() == 0);
    assert_eq!(blocking.receipt.unlisted[0].0, "migration.json");
    assert!(messages.iter().any(|s| s.contains("cannot inspect migration.json")));
    assert_eq!(fs::read_link(&dest).unwrap(), Path::new("TrainingRooms"));
}

#[cfg(unix)]
#[test]
fn core08_destination_inspection_error_is_partial_and_retryable() {
    let t = Tmp::new();
    write(&t.fork().join("macros/m.json"), b"macro");
    write(&t.fork().join("button_config.ini"), b"buttons");
    fs::create_dir_all(t.rooms()).unwrap();
    std::os::unix::fs::symlink("macros", t.rooms().join("macros")).unwrap();
    let (receipt, marker) = full_import(&t);
    assert!(marker.is_err());
    assert_eq!(receipt.result(), ImportResult::Partial);
    assert!(receipt.skipped.iter().any(|(path, reason)| path == "macros/m.json" && reason.starts_with("io:")));
    assert_eq!(fs::read(t.rooms().join("button_config.ini")).unwrap(), b"buttons");
    assert!(!t.rooms().join("migration.json").exists());
    fs::remove_file(t.rooms().join("macros")).unwrap();
    let (_, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(fs::read(t.rooms().join("macros/m.json")).unwrap(), b"macro");
}

#[cfg(unix)]
#[test]
fn core08_marker_appearing_during_background_is_not_replaced() {
    let t = Tmp::new();
    write(&t.fork().join("replay_captures/11.smd/11.tmrc"), b"capture");
    let (source, dest) = (t.fork(), t.rooms());
    let roots = Roots { source: &source, dest: &dest };
    let Stage::Imported { blocking, .. } = run_blocking(roots, 1, "0.1.0", &mut |_| {}) else { panic!() };
    fs::create_dir_all(&dest).unwrap();
    std::os::unix::fs::symlink("operator-marker.json", dest.join("migration.json")).unwrap();
    let (_, marker) = run_background(roots, blocking, &mut |_| {});
    marker.unwrap();
    assert_eq!(fs::read_link(dest.join("migration.json")).unwrap(), Path::new("operator-marker.json"));
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
    // A failed copy withholds the marker (lead ruling 2026-11-02): the next boot retries it.
    assert!(marker.is_err(), "a failed copy must not be marked as imported");
    assert!(!t.rooms().join("migration.json").exists());
    assert_eq!(receipt.result(), ImportResult::Partial);
    assert!(receipt.skipped.iter().any(|(p, r)| p == "button_config.ini" && r.starts_with("io:")), "{:?}", receipt.skipped);
    assert!(!t.rooms().join("button_config.ini").exists(), "no truncated file");
    assert!(t.rooms().join("settings.json").is_file() && t.rooms().join("replay_captures/22.smd/timeline.jsonl").is_file());
}

#[test]
fn migration_a_failed_copy_withholds_the_marker_and_the_second_run_completes() {
    let t = Tmp::new();
    fork_folder(&t);
    // save_states.json cannot be written (a directory on its temp name): the failure must not be lost for good.
    fs::create_dir_all(t.rooms().join("save_states.json.part")).unwrap();
    let (source, dest) = (t.fork(), t.rooms());
    let roots = Roots { source: &source, dest: &dest };
    let Stage::Imported { blocking, marker } = run_blocking(roots, 1, "0.1.0", &mut |_| {}) else { panic!() };
    assert!(blocking.marker_withheld());
    assert!(marker.is_none(), "captures are pending");
    let (receipt, marker) = run_background(roots, blocking, &mut |_| {});
    assert!(marker.is_err());
    assert_eq!(receipt.result(), ImportResult::Partial);
    assert!(!dest.join("migration.json").exists() && !dest.join("save_states.json").exists());
    assert!(dest.join("settings.json").is_file(), "the other files were copied");
    // Next boot, the obstacle gone: the missing file is copied (and converted), what exists is kept, the marker is written.
    fs::remove_dir_all(dest.join("save_states.json.part")).unwrap();
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.result(), ImportResult::Done);
    assert!(dest.join("save_states.json").is_file() && dest.join("migration.json").is_file());
    assert_eq!(receipt.converted, 1, "the retried save file is still converted once");
    assert!(receipt.skipped.iter().any(|(_, r)| r == "exists"), "what was copied the first time is kept");
}

#[test]
fn migration_deterministic_skips_still_allow_the_marker() {
    // Staging folders, an existing destination and a file over the cap are by design, not errors.
    let t = Tmp::new();
    fork_folder(&t);
    write(&t.rooms().join("button_config.ini"), b"mine\n");
    write(&t.fork().join("macros/big.json"), &vec![1u8; 8 * 1024 * 1024 + 1]);
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.skipped_staging, 1);
    assert!(receipt.skipped.iter().any(|(_, r)| r == "exists") && receipt.skipped.iter().any(|(_, r)| r == "too_large"));
    assert_eq!(receipt.result(), ImportResult::Done);
    assert!(t.rooms().join("migration.json").is_file());
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
        if receipt.result() == ImportResult::Done {
            marker.unwrap();
            return; // running as a user that can read anything (root): the case cannot be made
        }
        assert!(marker.is_err(), "an unreadable listed file withholds the marker");
        assert_eq!(receipt.result(), ImportResult::Partial);
        assert!(!t.rooms().join("mii-presets.json").exists() && !t.rooms().join("migration.json").exists());
        assert!(t.rooms().join("settings.json").is_file());
        // Readable again: the next boot copies it and completes.
        let (receipt, marker) = full_import(&t);
        marker.unwrap();
        assert_eq!(receipt.result(), ImportResult::Done);
        assert!(t.rooms().join("mii-presets.json").is_file());
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

// ---- R2-8: absent is not unreadable -----------------------------------------------------------------------

/// The marker is withheld, the result is partial, and every error names its path and the error.
fn assert_incomplete(receipt: &model::migration_run::Receipt, marker: &Result<(), String>, dest: &Path, path: &str) {
    assert_eq!(receipt.result(), ImportResult::Partial, "{receipt:?}");
    assert!(receipt.unlisted.iter().any(|(p, e)| p == path && e.starts_with("io:")), "{path}: {:?}", receipt.unlisted);
    assert!(marker.is_err(), "no marker for an incomplete import");
    assert!(!dest.join("migration.json").exists(), "migration.json must not exist");
}

#[test]
fn migration_r2_8_a_directory_that_cannot_be_enumerated_is_incomplete_not_absent() {
    let t = Tmp::new();
    fork_folder(&t);
    // `macros` exists but is not a folder: it cannot be enumerated (ENOTDIR, not NotFound).
    fs::remove_dir_all(t.fork().join("macros")).unwrap();
    write(&t.fork().join("macros"), b"not a folder");
    let mut log_lines = Vec::new();
    let (source, dest) = (t.fork(), t.rooms());
    let roots = Roots { source: &source, dest: &dest };
    let Stage::Imported { blocking, marker } = run_blocking(roots, 1, "0.1.0", &mut |l| log_lines.push(l.to_owned())) else { panic!() };
    assert!(marker.is_none(), "captures are pending");
    assert!(blocking.marker_withheld());
    let (receipt, marker) = run_background(roots, blocking, &mut |l| log_lines.push(l.to_owned()));
    assert_incomplete(&receipt, &marker, &dest, "macros");
    assert!(log_lines.iter().any(|l| l.contains("cannot list macros:") && l.contains("io:")), "{log_lines:?}");
    // The other paths were still imported.
    assert!(dest.join("settings.json").is_file() && dest.join("setups/a.json").is_file());
    assert!(dest.join("replay_captures/22.smd/timeline.jsonl").is_file());
    assert!(!dest.join("macros").exists());
    // Next boot, access restored: the missing content is imported, what exists is kept, the marker is written.
    fs::remove_file(t.fork().join("macros")).unwrap();
    write(&t.fork().join("macros/m.json"), b"{\"m\":1}");
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.result(), ImportResult::Done);
    assert!(receipt.skipped.iter().any(|(_, r)| r == "exists"));
    assert_eq!(fs::read(dest.join("macros/m.json")).unwrap(), b"{\"m\":1}");
    assert!(dest.join("migration.json").is_file());
}

#[cfg(unix)]
#[test]
fn migration_r2_8_a_permission_denied_directory_is_incomplete_and_retried() {
    use std::os::unix::fs::PermissionsExt;
    let t = Tmp::new();
    fork_folder(&t);
    let macros = t.fork().join("macros");
    fs::set_permissions(&macros, fs::Permissions::from_mode(0o000)).unwrap();
    // A privileged runner can read anything: this premise (an actual permission error) is checked, not assumed.
    let denied = fs::read_dir(&macros).is_err();
    let (receipt, marker) = full_import(&t);
    fs::set_permissions(&macros, fs::Permissions::from_mode(0o755)).unwrap();
    if denied {
        assert_incomplete(&receipt, &marker, &t.rooms(), "macros");
        assert!(!t.rooms().join("macros").exists());
        let (receipt, marker) = full_import(&t);
        marker.unwrap();
        assert_eq!(receipt.result(), ImportResult::Done);
        assert!(t.rooms().join("macros/m.json").is_file() && t.rooms().join("migration.json").is_file());
    } else {
        assert_eq!(receipt.result(), ImportResult::Done, "root can read it, nothing to report");
    }
}

#[cfg(unix)]
#[test]
fn migration_r2_8_a_metadata_failure_is_incomplete_not_absent() {
    let t = Tmp::new();
    fork_folder(&t);
    // A self-referencing link: stat follows it and fails with ELOOP (not NotFound), even for root.
    fs::remove_file(t.fork().join("button_config.ini")).unwrap();
    std::os::unix::fs::symlink("button_config.ini", t.fork().join("button_config.ini")).unwrap();
    let (receipt, marker) = full_import(&t);
    assert_incomplete(&receipt, &marker, &t.rooms(), "button_config.ini");
    assert!(!t.rooms().join("button_config.ini").exists());
    assert!(t.rooms().join("settings.json").is_file() && t.rooms().join("setups/a.json").is_file());
    // A link inside a listed folder fails the same way when the entry is stat-ed (the file type of a link is not a
    // file, so it is not imported, but the folder's own entries still are).
    fs::remove_file(t.fork().join("button_config.ini")).unwrap();
    write(&t.fork().join("button_config.ini"), b"save_state_save=zl+down\n");
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.result(), ImportResult::Done);
    assert_eq!(fs::read(t.rooms().join("button_config.ini")).unwrap(), b"save_state_save=zl+down\n");
}

#[test]
fn migration_r2_8_a_source_that_is_not_a_folder_is_incomplete_not_none() {
    let t = Tmp::new();
    write(&t.fork(), b"a file where the fork's folder should be");
    let (source, dest) = (t.fork(), t.rooms());
    let stage = run_blocking(Roots { source: &source, dest: &dest }, 4, "0.1.0", &mut |_| {});
    let Stage::Imported { blocking, marker } = &stage else { panic!("{stage:?}") };
    assert!(matches!(marker, Some(Err(_))));
    assert_eq!(blocking.receipt.unlisted.len(), 1);
    assert_eq!(status_word(&stage), "partial");
    assert!(!dest.join("migration.json").exists(), "an unusable source is not 'no source'");
}

#[test]
fn migration_r2_8_absent_optional_paths_are_still_absent() {
    let t = Tmp::new();
    // Only a source folder with one file: every other optional path is NotFound and means nothing.
    write(&t.fork().join("button_config.ini"), b"x\n");
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.result(), ImportResult::Done);
    assert!(receipt.unlisted.is_empty());
    assert!(t.rooms().join("migration.json").is_file());
}

#[test]
fn migration_r2_8_a_partial_import_keeps_what_it_copied_and_the_next_boot_completes_it() {
    let t = Tmp::new();
    fork_folder(&t);
    fs::remove_dir_all(t.fork().join("setups")).unwrap();
    write(&t.fork().join("setups"), b"x");
    let (receipt, marker) = full_import(&t);
    assert_incomplete(&receipt, &marker, &t.rooms(), "setups");
    let after_first = tree(&t.rooms());
    assert!(after_first.iter().any(|(p, _)| p == "settings.json") && after_first.iter().any(|(p, _)| p.starts_with("replay_captures/")));
    // The status word on the install line says partial.
    let (source, dest) = (t.fork(), t.rooms());
    let stage = run_blocking(Roots { source: &source, dest: &dest }, 9, "0.1.0", &mut |_| {});
    assert_eq!(status_word(&stage), "partial", "a second run is not MarkerPresent: there is no marker");
    // Fix the source: the third boot finishes and writes the marker.
    fs::remove_file(t.fork().join("setups")).unwrap();
    write(&t.fork().join("setups/a.json"), b"{\"a\":1}");
    let (receipt, marker) = full_import(&t);
    marker.unwrap();
    assert_eq!(receipt.result(), ImportResult::Done);
    assert!(t.rooms().join("setups/a.json").is_file() && t.rooms().join("migration.json").is_file());
}
