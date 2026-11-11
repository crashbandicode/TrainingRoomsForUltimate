//! SPEC-core-04 (settings file) with decisions S1 and S2. Test names are the spec's §9 rows; the two
//! fixtures are real settings files of the earlier fork (`tests/fixtures/fork-menu-*.json`, see
//! `tests/fixtures/README.md` for their origin).

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::settings::{self, parse, CloseReason, CommitStep, Reason, Settings, Snapshot, WriteMode};
use serde_json::{json, Value};

const OLD: &str = include_str!("fixtures/fork-menu-switch-before21.json");
const NEW: &str = include_str!("fixtures/fork-menu-console-crash78.json");

fn array(len: usize, selected: usize) -> Value {
    Value::Array((0..len).map(|i| Value::from(u8::from(i == selected))).collect())
}

#[test]
fn core04_wire_shape_fixture() {
    // OnOff is [on, off]; Delay is 31 one-hot elements; sliders are [lower, upper].
    let s = Settings::factory();
    assert_eq!(s.menu.get("lra_reset").unwrap(), [1, 0]);
    assert_eq!(s.menu.get("stage_hazards").unwrap(), [0, 1]);
    assert_eq!(s.menu.get("input_delay").unwrap().len(), 31);
    assert_eq!(s.menu.get("input_delay").unwrap()[0], 1);
    // The real files carry the same shape for the three rows this build knows.
    for text in [OLD, NEW] {
        let file: Value = serde_json::from_str(text).unwrap();
        for snapshot in ["menu", "defaults_menu"] {
            assert_eq!(file[snapshot]["lra_reset"], json!([1, 0]), "{snapshot}");
            assert_eq!(file[snapshot]["stage_hazards"], json!([0, 1]), "{snapshot}");
            assert_eq!(file[snapshot]["input_delay"], array(31, 0), "{snapshot}");
            assert_eq!(file[snapshot]["save_damage_limits_cpu"], json!([0, 150]), "{snapshot}");
        }
        let loaded = parse(text);
        assert!(!loaded.corrupt(), "{:?}", loaded.issues);
        assert_eq!(loaded.settings.menu.input_delay(), 0);
        assert!(loaded.settings.menu.lra_reset());
        assert!(!loaded.settings.menu.stage_hazards());
    }
    // Each row's array has exactly the table's length.
    for def in settings::DEFS.iter() {
        assert_eq!(def.default.len(), def.len, "{}", def.id);
    }
}

#[test]
fn core04_roundtrip_factory() {
    let factory = Settings::factory();
    let text = factory.to_json();
    let loaded = parse(&text);
    assert!(!loaded.corrupt());
    assert_eq!(loaded.settings, factory);
    // Pretty-printed, for hand editing.
    assert!(text.contains("\n  \"menu\""), "{text}");
}

#[test]
fn core04_defaults_independent() {
    let mut s = Settings::factory();
    s.menu.set("input_delay", &array_u8(31, 5)).unwrap();
    s.menu.set("lra_reset", &[0, 1]).unwrap();
    assert_eq!(s.menu.input_delay(), 5);
    assert_eq!(s.defaults.input_delay(), 0);
    let loaded = parse(&s.to_json());
    assert_eq!(loaded.settings.menu.input_delay(), 5);
    assert!(!loaded.settings.menu.lra_reset());
    assert_eq!(loaded.settings.defaults.input_delay(), 0);
    assert!(loaded.settings.defaults.lra_reset());
    // Saving defaults copies the current settings over the defaults snapshot.
    let mut saved = loaded.settings;
    saved.defaults = saved.menu.clone();
    let again = parse(&saved.to_json());
    assert_eq!(again.settings.defaults.input_delay(), 5);
}

fn array_u8(len: usize, selected: usize) -> Vec<u8> {
    (0..len).map(|i| u8::from(i == selected)).collect()
}

#[test]
fn core04_unknown_keys_ignored() {
    // Keys the table does not know never change a known row.
    let loaded = parse(&json!({"menu": {"future_row": [1,0,0], "hud": [1,0], "input_delay": array(31, 2)}, "defaults_menu": {}, "future": 1}).to_string());
    assert!(!loaded.corrupt());
    assert_eq!(loaded.settings.menu.input_delay(), 2);
    assert_eq!(loaded.settings.menu.extra().get("future_row"), Some(&json!([1, 0, 0])), "kept, not applied");
    assert_eq!(loaded.settings.defaults.input_delay(), 0);
    // The retired keys are read and dropped.
    let loaded = parse(&json!({"menu": {"update_policy": [1,0,0], "menu_open_start_press": [1,0]}, "defaults_menu": {}, "update_policy": 1}).to_string());
    assert!(!loaded.corrupt());
    assert!(loaded.settings.menu.extra().is_empty());
    assert!(loaded.settings.extra.is_empty());
    assert!(!loaded.settings.to_json().contains("update_policy"));
}

#[test]
fn core04_unknown_keys_kept_on_rewrite() {
    // Rows this build has no table row for (the fork's rosters, hats and Mii picks until package 6) are
    // kept verbatim and written back; dropping them would erase the player's settings.
    for text in [OLD, NEW] {
        let original: Value = serde_json::from_str(text).unwrap();
        let loaded = parse(text);
        let rewritten: Value = serde_json::from_str(&loaded.settings.to_json()).unwrap();
        for snapshot in ["menu", "defaults_menu"] {
            let before = original[snapshot].as_object().unwrap();
            let after = rewritten[snapshot].as_object().unwrap();
            for (key, value) in before {
                if settings::RETIRED.contains(&key.as_str()) {
                    assert!(!after.contains_key(key), "{key} should be dropped");
                } else {
                    assert_eq!(after.get(key), Some(value), "{snapshot}.{key}");
                }
            }
        }
    }
    // The newer file carries rows of the fork that are not arrays of bytes; they survive too.
    let loaded = parse(NEW);
    assert!(loaded.settings.menu.extra().contains_key("player_roster"));
}

#[test]
fn core04_missing_keys_default() {
    let loaded = parse("{}");
    assert!(!loaded.corrupt());
    assert_eq!(loaded.settings, Settings::factory());
    let loaded = parse(&json!({"menu": {"lra_reset": [0, 1]}}).to_string());
    assert!(!loaded.corrupt());
    assert!(!loaded.settings.menu.lra_reset());
    assert!(loaded.settings.defaults.lra_reset());
    assert_eq!(loaded.settings.menu.input_delay(), 0);
    // A shorter array leaves the tail off; a longer one loses its extra elements.
    let loaded = parse(&json!({"menu": {"input_delay": [0, 0, 1], "stage_hazards": [1, 0, 1, 1]}}).to_string());
    assert!(!loaded.corrupt());
    assert_eq!(loaded.settings.menu.input_delay(), 2);
    assert_eq!(loaded.settings.menu.get("input_delay").unwrap().len(), 31);
    assert_eq!(loaded.settings.menu.get("stage_hazards").unwrap(), [1, 0]);
}

#[test]
fn core04_corrupt_file_policy() {
    // Not JSON: factory settings, one issue, the caller keeps a renamed copy.
    for text in ["", "{", "not json", "{\"menu\": {\"input_delay\": [1,", "\u{0}\u{0}"] {
        let loaded = parse(text);
        assert!(loaded.corrupt(), "{text:?}");
        assert!(matches!(loaded.issues[0].reason, Reason::Syntax(_)), "{text:?}: {:?}", loaded.issues);
        assert_eq!(loaded.settings, Settings::factory());
    }
    assert_eq!(parse("[1,2]").issues[0].reason, Reason::NotAnObject);
    assert_eq!(parse("42").issues[0].reason, Reason::NotAnObject);
    // One bad field: only that field takes its factory value (field by field).
    let text = json!({"menu": {"input_delay": array(31, 7), "lra_reset": "off", "stage_hazards": [1, 0]},
                      "defaults_menu": {"stage_hazards": [1, 0]}})
    .to_string();
    let loaded = parse(&text);
    let at: Vec<_> = loaded.issues.iter().map(|i| (i.at.as_str(), i.reason.clone())).collect();
    assert_eq!(at, [("menu.lra_reset", Reason::NotAnArray)]);
    assert_eq!(loaded.settings.menu.input_delay(), 7);
    assert!(loaded.settings.menu.lra_reset(), "bad field takes its factory value");
    assert!(loaded.settings.menu.stage_hazards());
    assert!(loaded.settings.defaults.stage_hazards());
    // Each rejection reason.
    let reasons = |value: Value| parse(&json!({"menu": {"lra_reset": value}}).to_string()).issues.first().map(|i| i.reason.clone());
    assert_eq!(reasons(json!(null)), Some(Reason::NotAnArray));
    assert_eq!(reasons(json!([1, -1])), Some(Reason::ElementNotAByte));
    assert_eq!(reasons(json!([1, 256])), Some(Reason::ElementNotAByte));
    assert_eq!(reasons(json!([1, 0.5])), Some(Reason::ElementNotAByte));
    assert_eq!(reasons(json!([1, "0"])), Some(Reason::ElementNotAByte));
    assert_eq!(reasons(json!([2, 0])), Some(Reason::ElementAboveMax));
    assert_eq!(reasons(json!([1, 0])), None);
    // A snapshot that is not an object.
    let loaded = parse(&json!({"menu": [1], "defaults_menu": 3}).to_string());
    assert_eq!(loaded.issues.iter().map(|i| i.at.as_str()).collect::<Vec<_>>(), ["menu", "defaults_menu"]);
    assert_eq!(loaded.settings, Settings::factory());
}

#[test]
fn core04_non_utf8_file_is_corrupt() {
    let loaded = settings::parse_bytes(&[0x7b, 0xff, 0xfe, 0x7d]);
    assert!(loaded.corrupt());
    assert_eq!(loaded.settings, Settings::factory());
    assert!(!settings::parse_bytes(b"{}").corrupt());
}

#[test]
fn core04_duplicate_key_error() {
    let loaded = parse(r#"{"menu": {"lra_reset": [0, 1], "lra_reset": [0, 1], "input_delay": [0,1]}}"#);
    assert!(loaded.corrupt());
    assert_eq!(loaded.issues, [settings::Issue { at: "menu.lra_reset".into(), reason: Reason::DuplicateKey }]);
    assert!(loaded.settings.menu.lra_reset(), "a doubled key has no value to trust: factory");
    assert_eq!(loaded.settings.menu.input_delay(), 1, "the rest of the file still loads");
    let loaded = parse(r#"{"menu": {}, "menu": {}}"#);
    assert_eq!(loaded.issues[0].reason, Reason::DuplicateKey);
    let loaded = parse(r#"{"menu": {"mash_state": [1], "mash_state": [0]}}"#);
    assert_eq!(loaded.issues[0].at, "menu.mash_state");
    // A repeated key this build has no row for is reported too.
    let loaded = parse(r#"{"menu": {"future_row": [1], "future_row": [0]}}"#);
    assert_eq!(loaded.issues[0].at, "menu.future_row");
}

#[test]
fn core04_import_upstream_file() {
    // The fork's file, including the retired keys, loads unchanged; the older one still has
    // `menu_open_start_press` (retired, dropped) and loads clean.
    let loaded = parse(OLD);
    assert!(!loaded.corrupt());
    assert!(OLD.contains("menu_open_start_press") || NEW.contains("menu_open_start_press"));
    assert!(!loaded.settings.to_json().contains("menu_open_start_press"));
    let loaded = parse(NEW);
    assert!(!loaded.corrupt());
}

#[test]
fn core04_publish_order() {
    // The globals features read are replaced before the write is queued.
    assert_eq!(settings::COMMIT_ORDER, [CommitStep::Publish, CommitStep::QueueWrite]);
}

#[test]
fn core04_writer_fallback() {
    assert_eq!(settings::write_mode(true), WriteMode::Thread);
    assert_eq!(settings::write_mode(false), WriteMode::Synchronous);
    // SPEC-core-04 as amended 2026-11-02: the writer and loader stacks are the measured thread table's rows (tests/thread_stacks.rs
    // checks each covers its measured need); here only that both threads have one.
    const { assert!(settings::WRITER_STACK_BYTES >= 16 * 1024) };
    const { assert!(settings::LOADER_STACK_BYTES >= 16 * 1024) };
}

#[test]
fn core04_no_write_on_unplug() {
    assert!(settings::writes_on_close(CloseReason::Closed));
    assert!(!settings::writes_on_close(CloseReason::AllControllersUnplugged));
}

#[test]
fn core04_decoders() {
    let mut s = Snapshot::factory();
    // An empty single-choice selection: Input Delay is 0, the OnOff rows read Off.
    s.set("input_delay", &[0; 31]).unwrap();
    s.set("lra_reset", &[0, 0]).unwrap();
    s.set("stage_hazards", &[0, 0]).unwrap();
    assert_eq!(s.input_delay(), 0);
    assert!(!s.lra_reset());
    assert!(!s.stage_hazards());
    // Every delay option decodes to its own index.
    for d in 0..=30usize {
        s.set("input_delay", &array_u8(31, d)).unwrap();
        assert_eq!(usize::from(s.input_delay()), d);
    }
    assert_eq!(s.set("nope", &[1]), Err(Reason::UnknownId));
    assert_eq!(settings::def("input_delay").map(|(i, _)| i), Some(settings::INPUT_DELAY));
}

#[test]
fn core04_slider_rows() {
    let def = settings::Def { id: "save_damage_limits_cpu", kind: settings::Kind::Slider, len: 2, default: &[0, 150] };
    assert_eq!(settings::parse_row(&def, &json!([10, 20])), Ok(vec![10, 20]));
    assert_eq!(settings::parse_row(&def, &json!([0, 150])), Ok(vec![0, 150]));
    assert_eq!(settings::parse_row(&def, &json!([30, 20])), Err(Reason::BadSliderRange));
    assert_eq!(settings::parse_row(&def, &json!([0, 151])), Err(Reason::ElementAboveMax));
    assert_eq!(settings::parse_row(&def, &json!([0])), Err(Reason::BadSliderLength));
    assert_eq!(settings::parse_row(&def, &json!([0, 1, 2])), Err(Reason::BadSliderLength));
    let weighted = settings::Def { id: "mash_state", kind: settings::Kind::Weighted, len: 3, default: &[0, 0, 0] };
    assert_eq!(settings::parse_row(&weighted, &json!([5, 2, 0])), Ok(vec![5, 2, 0]));
    assert_eq!(settings::parse_row(&weighted, &json!([6, 0, 0])), Err(Reason::ElementAboveMax));
}

#[test]
fn core04_draft9_fields_default() {
    // The draft 9 rows are absent in an older file and take their factory value, clean.
    let loaded = parse(&json!({"menu": {"lra_reset": [1, 0]}}).to_string());
    assert!(!loaded.corrupt());
    for id in ["respawn_invincibility", "settings_override_playback", "dev_reload_config", "dev_debug_hooks"] {
        assert_eq!(loaded.settings.menu.get(id), Some(&[0u8, 1][..]), "{id} is Off");
    }
    for id in ["aerial_delay_range", "fast_fall_delay_range", "throw_delay_range", "pummel_delay_range", "ledge_delay_range", "oos_offset_range", "reaction_time_range"] {
        assert_eq!(loaded.settings.menu.get(id), Some(&[][..]), "{id} has no range");
    }
    // Present, they load into their rows; a range is absent from the file again while empty.
    let loaded = parse(&json!({"menu": {"respawn_invincibility": [1, 0], "aerial_delay_range": [2, 9]}}).to_string());
    assert!(!loaded.corrupt());
    assert_eq!(loaded.settings.menu.get("respawn_invincibility"), Some(&[1u8, 0][..]));
    assert_eq!(loaded.settings.menu.get("aerial_delay_range"), Some(&[2u8, 9][..]));
    assert!(loaded.settings.menu.extra().is_empty());
    let json: Value = serde_json::from_str(&loaded.settings.to_json()).unwrap();
    assert_eq!(json["menu"]["aerial_delay_range"], json!([2, 9]));
    assert!(json["menu"].get("fast_fall_delay_range").is_none());
    // A bad range (reversed, one element, above the largest option index) defaults, and is reported.
    for bad in [json!([9, 2]), json!([1]), json!([1, 31])] {
        let loaded = parse(&json!({"menu": {"aerial_delay_range": bad}}).to_string());
        assert_eq!(loaded.issues.len(), 1, "{bad}");
        assert_eq!(loaded.settings.menu.get("aerial_delay_range"), Some(&[][..]));
    }
}

#[test]
fn core04_character_item_retired() {
    let loaded = parse(&json!({"menu": {"character_item": [0, 1, 0]}, "defaults_menu": {"character_item": [1]}}).to_string());
    assert!(!loaded.corrupt());
    assert!(loaded.settings.menu.extra().get("character_item").is_none());
    assert!(!loaded.settings.to_json().contains("character_item"));
}

#[test]
fn core04_per_field_default_on_bad_field() {
    let text = json!({"menu": {"input_delay": "x", "lra_reset": [0, 1], "stage_hazards": [9, 9]}}).to_string();
    let loaded = parse(&text);
    assert_eq!(loaded.issues.len(), 2);
    assert_eq!(loaded.settings.menu.input_delay(), 0, "bad field defaults");
    assert!(!loaded.settings.menu.lra_reset(), "the good field still loads");
    assert!(!loaded.settings.menu.stage_hazards(), "bad field defaults (Off)");
}
