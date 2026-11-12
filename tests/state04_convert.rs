//! SPEC-state-04 §9 / SPEC-core-08 §9, decision N6: the one-time conversion of the retired
//! `character_item` setting into held-item snapshots of older saves.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::state::convert::{self, convert_file, item_for, setting_index, snapshot_for};
use model::state::file::SaveFile;
use model::state::held_item::{HeldItem, ITEM_NAMES};
use model::state::persist;
use model::state::record::SavedState;
use serde_json::json;

fn item(choice: u8, variation: i32) -> Option<HeldItem> {
    Some(HeldItem::Item { choice, variation, lifetime: -1, lifetime_max: -1, fuse: -1, fuse_max: -1 })
}

fn record(kind: i32) -> SavedState {
    SavedState { fighter_kind: kind, ..SavedState::never_saved() }
}

fn state04_setting_to_variation() {
    // Player 1st..8th resolve against the player's kind; CPU 1st..8th against the CPU's kind.
    assert_eq!(snapshot_for(1, 0x0e, 7), item(2, 0), "Daisy 1st: Smile Turnip");
    assert_eq!(snapshot_for(4, 0x0e, 7), item(5, 7), "Daisy 4th: Stitch-Face Turnip");
    assert_eq!(snapshot_for(9, 7, 0x2d), item(29, 0), "CPU 1st with a R.O.B. CPU");
    assert_eq!(snapshot_for(16, 7, 0x2d), item(29, 7), "CPU 8th: Gyro 8P");
    assert_eq!(snapshot_for(0, 0x0e, 0x0e), None);
    assert_eq!(snapshot_for(17, 0x0e, 0x0e), None);
}

fn state04_table_lookup() {
    // Every entry of §2.1 names an item of the fork's list with a variation the list accepts.
    let kinds = [0x54, 0x3a, 0x0e, 0x27, 0x45, 0x02, 0x31, 0x37, 0x0d, 0x44, 0x2d, 0x38, 0x10, 0x43, 0x22, 0x2e, 0x30, 0x17];
    let mut total = 0;
    for kind in kinds {
        for variation in 0..8 {
            if let Some((choice, v)) = item_for(kind, variation) {
                total += 1;
                assert!(usize::from(choice) < ITEM_NAMES.len());
                assert!(model::state::held_item::valid_variation(usize::from(choice), v), "kind {kind:#x} #{variation}");
                assert!(item(choice, v).unwrap().valid());
            }
        }
        assert_eq!(item_for(kind, 8), None, "out of range -> no item");
    }
    assert_eq!(total, 1 + 1 + 6 + 2 + 1 + 2 + 1 + 8 + 6 + 1 + 8 + 2 + 1 + 1 + 1 + 1 + 1 + 1);
    assert_eq!(item_for(7, 0), None, "Fox has no entry");
    // The item names agree with the fighters of the table (a spot check of the fork's list).
    assert!(ITEM_NAMES[2].starts_with("Daisy") && ITEM_NAMES[22].starts_with("Peach"));
    assert!(ITEM_NAMES[29].starts_with("R.O.B.") && ITEM_NAMES[37].starts_with("Young Link"));
}

fn state04_convert_setting_to_snapshot() {
    let mut file = SaveFile::default();
    file.player[0] = record(0x0e);
    file.cpu[0] = record(0x2d);
    file.player[1] = record(0x02);
    file.cpu[1] = record(0x07);
    // Player 1st Var.
    assert_eq!(convert_file(&mut file, 1), 2);
    assert_eq!(file.player[0].held_item, item(2, 0));
    assert_eq!(file.player[1].held_item, item(11, 0), "Link's Remote Bomb");
    // CPU-role records are untouched.
    assert_eq!(file.cpu[0].held_item, None);
    assert_eq!(file.cpu[1].held_item, None);
}

fn state04_convert_no_entry_leaves_absent() {
    let mut file = SaveFile::default();
    file.player[0] = record(0x07); // Fox: no entry
    file.player[1] = record(-1); // never saved
    file.player[2] = record(0x0e);
    assert_eq!(convert_file(&mut file, 3), 1, "Daisy 3rd");
    assert_eq!(file.player[0].held_item, None);
    assert_eq!(file.player[1].held_item, None);
    assert_eq!(file.player[2].held_item, item(4, 6));
    // The 7th Daisy variation does not exist.
    let mut other = SaveFile::default();
    other.player[0] = record(0x0e);
    assert_eq!(convert_file(&mut other, 7), 0);
    // A CPU entry whose CPU kind has no entry converts nothing.
    let mut cpu_fox = SaveFile::default();
    cpu_fox.player[0] = record(0x0e);
    cpu_fox.cpu[0] = record(0x07);
    assert_eq!(convert_file(&mut cpu_fox, 9), 0);
}

fn state04_convert_never_overwrites() {
    let mut file = SaveFile::default();
    for (slot, existing) in [Some(HeldItem::Empty), Some(HeldItem::Unsupported), item(25, 7)].into_iter().enumerate() {
        file.player[slot] = SavedState { held_item: existing, ..record(0x0e) };
    }
    assert_eq!(convert_file(&mut file, 1), 0);
    assert_eq!(file.player[0].held_item, Some(HeldItem::Empty));
    assert_eq!(file.player[1].held_item, Some(HeldItem::Unsupported));
    assert_eq!(file.player[2].held_item, item(25, 7));
}

fn state04_cpu_selection_requires_a_saved_player_in_both_representations() {
    // CPU-selected item kind does not make an absent player record a saved one (core-08 §4 N6).
    // Literal oracle: CPU R.O.B. first/eighth variations are Gyro choices29/variation0 or7.
    for (index, variation) in [(9, 0), (16, 7)] {
        let mut file = SaveFile::default();
        for slot in 0..5 {
            file.cpu[slot] = record(0x2d);
        }
        file.player[1] = record(7);
        file.player[2] = SavedState { held_item: Some(HeldItem::Empty), ..record(7) };
        file.player[3] = SavedState { held_item: Some(HeldItem::Unsupported), ..record(7) };
        file.player[4] = SavedState { held_item: item(25, 7), ..record(7) };
        let mut json: serde_json::Value = serde_json::from_str(&file.to_pretty()).unwrap();
        let before = json.clone();
        assert_eq!(convert::convert_json(&mut json, index), 1, "never-saved player must remain absent");
        assert_eq!(convert_file(&mut file, index), 1);
        assert_eq!(file.player[0].held_item, None);
        assert_eq!(file.player[1].held_item, item(29, variation));
        assert_eq!(json, serde_json::from_str::<serde_json::Value>(&file.to_pretty()).unwrap(), "JSON import and typed converter agree");
        assert_eq!(json["cpu"], before["cpu"], "CPU-role records never receive the item");
        assert_eq!(convert::convert_json(&mut json, index), 0, "idempotent even without a marker");
        assert_eq!(snapshot_for(index, -1, 0x2d), None, "the shared snapshot constructor owns target validity");
    }
}

fn state04_cpu_selection_does_not_invent_an_absent_or_invalid_player_kind() {
    for player in [json!({}), json!({"fighter_kind": null}), json!({"fighter_kind": "7"}), json!({"fighter_kind": -1}), json!({"fighter_kind": 2147483648_i64})] {
        let mut file = json!({"player": [player], "cpu": [{"fighter_kind": 0x2d}]});
        let original = file.clone();
        assert_eq!(convert::convert_json(&mut file, 9), 0);
        assert_eq!(file, original, "invalid/absent player is not rewritten");
    }
    let mut valid = json!({"player": [{"fighter_kind": 7, "held_item": null}], "cpu": [{"fighter_kind": 0x2d}]});
    assert_eq!(convert::convert_json(&mut valid, 9), 1, "null snapshot on an actual save is convertible");
    assert_eq!(valid["player"][0]["held_item"]["choice"], 29);
}

fn state04_setting_retired() {
    // The old key is a one-hot array; None and a missing key convert nothing.
    let mut one_hot = vec![0; convert::SETTING_OPTIONS];
    assert_eq!(setting_index(&json!(one_hot)), None);
    one_hot[3] = 1;
    assert_eq!(setting_index(&json!(one_hot)), Some(3));
    assert_eq!(setting_index(&json!("x")), None);
    assert_eq!(setting_index(&json!([1, 0, 0])), None, "option 0 is None");
    // The settings model keeps no such row and ignores the key on load.
    assert!(model::settings::def("character_item").is_none());
    assert!(model::settings::RETIRED.contains(&"character_item"));
}

fn state04_convert_runs_on_the_copy_and_is_durable() {
    // Filesystem: the copied file is converted and rewritten atomically; a second boot reads the
    // snapshot, and a failed write leaves the copy as it was.
    let dir = std::env::temp_dir().join(format!("training_rooms-convert-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("save_states.json");
    let mut file = SaveFile::default();
    file.player[0] = record(0x0e);
    persist::save(&path, &file).unwrap();
    let before = std::fs::read(&path).unwrap();

    let (mut copy, _) = persist::load(&path);
    assert_eq!(convert_file(&mut copy, 1), 1);
    std::fs::create_dir(dir.join("save_states.json.tmp")).unwrap();
    assert!(persist::save(&path, &copy).is_err(), "a failed write");
    assert_eq!(std::fs::read(&path).unwrap(), before, "the copy is as it was");
    std::fs::remove_dir(dir.join("save_states.json.tmp")).unwrap();
    persist::save(&path, &copy).unwrap();
    let (second_boot, _) = persist::load(&path);
    assert_eq!(second_boot.player[0].held_item, item(2, 0));
    // The conversion is idempotent: a re-run (which the marker prevents) would change nothing.
    let mut again = second_boot.clone();
    assert_eq!(convert_file(&mut again, 1), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("state04_setting_to_variation", state04_setting_to_variation as fn()),
        ("state04_table_lookup", state04_table_lookup as fn()),
        ("state04_convert_setting_to_snapshot", state04_convert_setting_to_snapshot as fn()),
        ("state04_convert_no_entry_leaves_absent", state04_convert_no_entry_leaves_absent as fn()),
        ("state04_convert_never_overwrites", state04_convert_never_overwrites as fn()),
        ("state04_cpu_selection_requires_a_saved_player_in_both_representations", state04_cpu_selection_requires_a_saved_player_in_both_representations as fn()),
        ("state04_cpu_selection_does_not_invent_an_absent_or_invalid_player_kind", state04_cpu_selection_does_not_invent_an_absent_or_invalid_player_kind as fn()),
        ("state04_setting_retired", state04_setting_retired as fn()),
        ("state04_convert_runs_on_the_copy_and_is_durable", state04_convert_runs_on_the_copy_and_is_durable as fn()),
    ]);
}
