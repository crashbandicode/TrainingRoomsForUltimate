//! Combined Single values must have one meaning in the menu, defaults and runtime gate.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::compat::menu::{OnOff, TrainingModpackMenu};
use model::menu::input::Button;
use model::menu::table::ROWS;
use model::menu::{Entry, Env, Menu, RowValue};
use model::settings::{self, Kind, Settings, Snapshot};
use serde_json::json;

fn focus(menu: &mut Menu, target: Entry) {
    for _ in 0..=menu.entry_count(menu.page()) {
        if menu.focus() == Some(target) {
            return;
        }
        menu.press(Button::Down);
    }
    panic!("cannot focus {target:?}");
}

fn swapping(menu: &mut Menu) -> usize {
    let row = ROWS.iter().position(|row| row.id == "trio_practice").unwrap();
    for _ in 0..6 {
        if menu.page() == ROWS[row].page() {
            break;
        }
        menu.press(Button::R);
    }
    assert_eq!(menu.page(), ROWS[row].page());
    let group = usize::from(ROWS[row].group);
    if !menu.group_is_open(group) {
        focus(menu, Entry::Group(group));
        menu.press(Button::A);
    }
    focus(menu, Entry::Row(row));
    row
}

fn agrees(menu: &Menu, row: usize, on: bool) {
    assert_eq!(menu.value(row), RowValue::Toggle(on));
    assert_eq!(menu.settings().menu.get("trio_practice").unwrap(), if on { &[1, 0] } else { &[0, 1] });
    // This is the exact equality used by character_roster::Role::enabled.
    assert_eq!(TrainingModpackMenu::from_snapshot(&menu.settings().menu).trio_practice == OnOff::ON, on);
}

#[test]
fn boot_load_repairs_both_snapshots_and_the_exact_player_cycle_gate() {
    let loaded = settings::parse(r#"{"menu":{"trio_practice":[1,1]},"defaults_menu":{"trio_practice":[1,1]}}"#);
    assert!(loaded.issues.is_empty());
    assert_eq!(loaded.settings.defaults.get("trio_practice").unwrap(), &[1, 0]);
    let mut menu = Menu::new(loaded.settings, Env::default());
    let row = swapping(&mut menu);
    agrees(&menu, row, true);
    let live = menu.settings().clone();
    let committed = menu.commit_onto(&live);
    let reloaded = settings::parse(&committed.to_json());
    assert_eq!(TrainingModpackMenu::from_snapshot(&reloaded.settings.menu).trio_practice, OnOff::ON);
}

#[test]
fn reset_page_from_combined_saved_defaults_keeps_the_display_and_gate_on() {
    let loaded = settings::parse(r#"{"menu":{"trio_practice":[0,1]},"defaults_menu":{"trio_practice":[1,1]}}"#);
    let mut menu = Menu::new(loaded.settings, Env::default());
    let row = swapping(&mut menu);
    agrees(&menu, row, false);
    let page = menu.page();
    focus(&mut menu, Entry::ResetPage(page));
    menu.press(Button::A);
    menu.press(Button::A);
    assert!(menu.screen().is_none());
    agrees(&menu, row, true);
    let live = menu.settings().clone();
    let committed = menu.commit_onto(&live);
    assert_eq!(TrainingModpackMenu::from_snapshot(&committed.menu).trio_practice, OnOff::ON);
}

#[test]
fn toggle_save_defaults_and_reopen_keep_single_values_canonical() {
    let loaded = settings::parse(r#"{"menu":{"trio_practice":[1,1]}}"#);
    let mut menu = Menu::new(loaded.settings, Env::default());
    let row = swapping(&mut menu);
    agrees(&menu, row, true);
    menu.press(Button::A);
    agrees(&menu, row, false);
    menu.press(Button::A);
    agrees(&menu, row, true);
    menu.press(Button::X);
    assert_eq!(menu.settings().defaults.get("trio_practice").unwrap(), &[1, 0]);
    let saved = settings::parse(&menu.settings().to_json()).settings;
    menu.reopen(saved, Env::default());
    let row = swapping(&mut menu);
    agrees(&menu, row, true);
}

#[test]
fn every_single_load_and_set_keep_the_first_option_and_allow_none() {
    for def in settings::DEFS.iter().filter(|def| def.kind == Kind::Single) {
        for first in 0..def.len {
            let mut combined = vec![0; def.len];
            combined[first..].fill(1);
            let mut expected = vec![0; def.len];
            expected[first] = 1;
            let mut snapshot = Snapshot::factory();
            snapshot.set(def.id, &combined).unwrap();
            assert_eq!(snapshot.get(def.id).unwrap(), expected, "set {}", def.id);
            let loaded = settings::parse(&json!({"menu": {(def.id): combined.clone()}, "defaults_menu": {(def.id): combined}}).to_string());
            assert!(loaded.issues.is_empty());
            for snapshot in [&loaded.settings.menu, &loaded.settings.defaults] {
                assert_eq!(snapshot.get(def.id).unwrap(), expected, "load {}", def.id);
            }
        }
        let mut snapshot = Snapshot::factory();
        snapshot.set(def.id, &vec![0; def.len]).unwrap();
        assert!(snapshot.get(def.id).unwrap().iter().all(|v| *v == 0), "none {}", def.id);
    }
}

#[test]
fn weighted_random_and_multiple_chords_are_not_single_selections() {
    let mut settings = Settings::factory();
    for id in ["falling_aerials", "full_hop", "fast_fall"] {
        settings.menu.set(id, &[1, 1]).unwrap();
    }
    settings.menu.set("player_cycle", &[0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]).unwrap();
    settings.menu.set("shield_tilt", &[0; 11]).unwrap();
    let loaded = settings::parse(&settings.to_json());
    assert_eq!(loaded.settings, settings);
    for id in ["falling_aerials", "full_hop", "fast_fall"] {
        assert_eq!(loaded.settings.menu.get(id).unwrap(), &[1, 1]);
    }
    assert_eq!(loaded.settings.menu.get("player_cycle").unwrap().iter().sum::<u8>(), 2);
}
