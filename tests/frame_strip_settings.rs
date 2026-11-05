//! SPEC-input-06 §3.1: the Input Display row (`input_display_strip`: Off, Frame strip), the migration from the retired
//! input-log rows, the defaults import and a settings round trip.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::frame_strip::Settings as Strip;
use model::settings::{parse, Settings};
use serde_json::{json, Value};

fn file(menu: Value, defaults: Value) -> String {
    json!({ "menu": menu, "defaults_menu": defaults }).to_string()
}

#[test]
fn the_factory_value_is_the_frame_strip() {
    let s = Settings::factory();
    assert_eq!(s.menu.get("input_display_strip").unwrap(), [0, 1]);
    assert!(Strip::from_snapshot(&s.menu).on);
    assert!(s.menu.get("input_display").is_none() && s.menu.get("input_display_status").is_none());
}

#[test]
fn an_older_file_migrates_off_to_off_and_every_display_mode_to_the_strip() {
    for (old, strip) in [([1, 0, 0, 0], [1, 0]), ([0, 1, 0, 0], [0, 1]), ([0, 0, 1, 0], [0, 1]), ([0, 0, 0, 1], [0, 1])] {
        let loaded = parse(&file(json!({ "input_display": old, "input_display_status": [0, 1] }), json!({ "input_display": old })));
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        assert_eq!(loaded.settings.menu.get("input_display_strip").unwrap(), strip, "menu {old:?}");
        assert_eq!(loaded.settings.defaults.get("input_display_strip").unwrap(), strip, "saved defaults {old:?}");
        assert_eq!(Strip::from_snapshot(&loaded.settings.menu).on, strip == [0, 1]);
    }
}

#[test]
fn the_new_row_wins_over_the_old_one_and_a_bad_old_value_keeps_the_default() {
    let loaded = parse(&file(json!({ "input_display_strip": [1, 0], "input_display": [0, 1, 0, 0] }), json!({})));
    assert_eq!(loaded.settings.menu.get("input_display_strip").unwrap(), [1, 0]);
    // An element above 1 was refused by the base too (its factory value, Smash Inputs, showed the display).
    let loaded = parse(&file(json!({ "input_display": [0, 2, 0, 0] }), json!({ "input_display": "Off" })));
    assert_eq!(loaded.settings.menu.get("input_display_strip").unwrap(), [0, 1], "not a row: the default");
    assert_eq!(loaded.settings.defaults.get("input_display_strip").unwrap(), [0, 1]);
}

/// Review IV-3: the base padded or cut the old array to its four options (`parse_row`) and read the first option set,
/// so a short or hand-edited Off loaded as Off; it must stay Off (it came back On before the fix).
#[test]
fn an_older_file_is_read_as_the_base_read_it() {
    for (old, strip) in [
        (json!([1, 0, 0]), [1, 0]),
        (json!([1]), [1, 0]),
        (json!([1, 1, 0, 0]), [1, 0]),
        (json!([1, 0, 0, 0, 0]), [1, 0]),
        (json!([0, 0, 1]), [0, 1]),
        (json!([0, 0, 0, 0]), [0, 1]),
        (json!([]), [0, 1]),
    ] {
        let loaded = parse(&file(json!({ "input_display": old }), json!({ "input_display": old })));
        assert_eq!(loaded.settings.menu.get("input_display_strip").unwrap(), strip, "menu {old}");
        assert_eq!(loaded.settings.defaults.get("input_display_strip").unwrap(), strip, "saved defaults {old}");
    }
}

#[test]
fn a_round_trip_writes_the_new_row_and_drops_the_retired_ones() {
    let loaded = parse(&file(json!({ "input_display": [1, 0, 0, 0], "input_display_status": [1, 0] }), json!({})));
    let written: Value = serde_json::from_str(&loaded.settings.to_json()).unwrap();
    assert_eq!(written["menu"]["input_display_strip"], json!([1, 0]));
    for key in ["input_display", "input_display_status"] {
        assert!(written["menu"].get(key).is_none(), "{key} is retired");
        assert!(written.get(key).is_none());
    }
    let again = parse(&loaded.settings.to_json());
    assert!(again.issues.is_empty());
    assert_eq!(again.settings.menu.get("input_display_strip").unwrap(), [1, 0]);
}
