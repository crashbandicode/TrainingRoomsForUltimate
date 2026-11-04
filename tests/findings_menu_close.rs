//! Phase-23 Ally finding 2 (artifacts/phase23/BRIEF-FINDINGS-ALLY.md, EDEN-ALLY-REPORT.md): the menu
//! worker's close (`runtime::ui::worker::close`) publishes `menu.commit()` — the snapshot the menu
//! opened on — so rows adopted while the menu was open are reverted by every menu visit. The adopting
//! paths are decision D1's write-backs (`compat::menu` store → `ui::worker::adopt_typed`, "the rows
//! ... the settings the menu opens on") and, in the diag build, `diag_settings_reload`'s publish.
//! Decision D1 (docs/PACKAGE-6-PLAN.md "D1 accepted") says package 5's commit path *persists* a
//! write-back; SPEC-core-02 §6.7 re-synchronises the model from the committed settings at every
//! opening. The root-cause fix needs the operator's conflict policy (who wins when the open menu and
//! an adopted row changed the same row), so this regression test is `#[ignore]`d until that decision
//! (brief rule 3: a fix that changes designed behaviour goes to the operator first).

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::menu::{Env, Menu};
use model::settings::Settings;

/// A one-hot `TrioFighter` choice over the Mii-classes envelope (89 choices): `kind` is the selected
/// fighter kind (`0x01` Donkey Kong, `0x41` Inkling), serialized exactly as the kept key stores it.
fn choice(kind: usize) -> Vec<u8> {
    let mut values = vec![0u8; 89];
    values[kind] = 1;
    values
}

#[test]
#[ignore = "finding 2 (Ally batch): the close conflict policy is the operator's decision; unignore with the fix"]
fn a_menu_close_does_not_revert_a_row_adopted_while_it_was_open() {
    // The menu opens on the committed settings: the CPU's pick is Donkey Kong (the B4-3 row's body).
    let mut opened = Settings::factory();
    opened
        .menu
        .set_kept("cpu_character", serde_json::to_value(choice(0x01)).unwrap())
        .unwrap();
    let mut menu = Menu::new(opened.clone(), Env::default());

    // While the menu is open, a write-back adopts the fixture's CPU pick (Inkling, kind 0x41) into
    // the committed settings the menu opened on (`ui::worker::adopt_typed`, decision D1).
    let mut adopted = opened;
    adopted
        .menu
        .set_kept("cpu_character", serde_json::to_value(choice(0x41)).unwrap())
        .unwrap();

    // The close publishes exactly what the model commits (`worker::close`, `menu.commit()`).
    let committed = menu.commit().clone();
    assert_eq!(
        committed.menu.extra().get("cpu_character"),
        adopted.menu.extra().get("cpu_character"),
        "decision D1: a row adopted while the menu is open must survive the close commit"
    );
}
