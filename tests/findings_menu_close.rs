//! Decision MENU-CLOSE (`review/menu-proposal` `9a9cc47`, docs/DECISIONS.md): closing the menu
//! commits only the rows the player changed in this menu session (the player's edit wins a
//! same-row conflict); every other row keeps its live value — the D1 write-backs and publishes
//! adopted while the menu was open (Ally batch finding 2). The close path is
//! `runtime::ui::worker::close` → `Menu::commit_onto(&live)`.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::menu::input::Button;
use model::menu::table::{ROWS};
use model::menu::{Env, Entry, Menu};
use model::settings::Settings;

/// A one-hot `TrioFighter` choice over the Mii-classes envelope (89 choices): `kind` is the selected
/// fighter kind (`0x01` Donkey Kong, `0x41` Inkling), serialized exactly as the kept key stores it.
fn choice(kind: usize) -> Vec<u8> {
    let mut values = vec![0u8; 89];
    values[kind] = 1;
    values
}

fn row_of(key: &str) -> usize {
    ROWS.iter().position(|row| row.id == key).unwrap_or_else(|| panic!("no row {key}"))
}

/// Focus a row: change page, open its advanced group, move down to it (the core02 helpers).
fn goto(m: &mut Menu, key: &str) {
    let row = row_of(key);
    let page = ROWS[row].page();
    for _ in 0..6 {
        if m.page() == page {
            break;
        }
        m.press(Button::R);
    }
    assert_eq!(m.page(), page, "page of {key} is in the ring");
    let group = usize::from(ROWS[row].group);
    if !m.group_is_open(group) {
        for _ in 0..=m.entry_count(m.page()) {
            if m.focus() == Some(Entry::Group(group)) {
                break;
            }
            m.press(Button::Down);
        }
        m.press(Button::A);
    }
    for _ in 0..=m.entry_count(m.page()) {
        if m.focus() == Some(Entry::Row(row)) {
            return;
        }
        m.press(Button::Down);
    }
    panic!("could not focus {key}");
}

/// MENU-CLOSE: a D1 write-back (or an adopted publish) changed a row while the menu was open; the
/// close keeps it instead of reverting to the open-time snapshot.
#[test]
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

    // The close commits onto the live settings (`worker::close`).
    let committed = menu.commit_onto(&adopted);
    assert_eq!(
        committed.menu.extra().get("cpu_character"),
        adopted.menu.extra().get("cpu_character"),
        "decision MENU-CLOSE: a row adopted while the menu is open must survive the close commit"
    );
}

/// MENU-CLOSE: when the player and the game changed the same row, the player's edit wins.
#[test]
fn a_player_edit_wins_a_same_row_conflict() {
    let opened = Settings::factory();
    let factory_hud = opened.menu.get("hud").unwrap().to_vec();
    let mut menu = Menu::new(opened.clone(), Env::default());

    // The player toggles the HUD row in this session (the model's own input path).
    goto(&mut menu, "hud");
    menu.press(Button::A);
    let player_value = menu.settings().menu.get("hud").unwrap().to_vec();
    assert_ne!(player_value, factory_hud, "the toggle changed the row");

    // While the menu was open the game adopted the row back to the factory value.
    let mut adopted = opened;
    adopted.menu.set("hud", &factory_hud).unwrap();

    let committed = menu.commit_onto(&adopted);
    assert_eq!(
        committed.menu.get("hud").unwrap(),
        player_value.as_slice(),
        "decision MENU-CLOSE: the player's edit wins a same-row conflict"
    );
}

/// MENU-CLOSE: a close with no edits leaves the live settings as they are — nothing is reverted and
/// nothing the game adopted is lost. (`last_page` still records the page left on: Q5, SPEC-core-04.)
#[test]
fn a_close_with_no_edits_leaves_the_live_settings_unchanged() {
    let mut live = Settings::factory();
    live.menu
        .set_kept("cpu_roster", serde_json::json!({"a": 1}))
        .unwrap();
    live.extra.insert("stage_preferences".into(), serde_json::json!({"direct_stage": 7}));
    let mut menu = Menu::new(live.clone(), Env::default());

    let committed = menu.commit_onto(&live);
    assert_eq!(committed.menu, live.menu, "no session edits: the rows are the live ones");
    assert_eq!(committed.defaults, live.defaults, "no session edits: the defaults are the live ones");
    // `last_page` still records the page the menu was left on (Q5, SPEC-core-04 §3.1); every other
    // top-level key is the live one.
    let mut extra = live.extra.clone();
    extra.remove("last_page");
    let mut committed_extra = committed.extra.clone();
    committed_extra.remove("last_page");
    assert_eq!(committed_extra, extra, "no session edits: the top-level keys are the live ones");
}

/// The operator's console finding (release v0.1.0, `artifacts/release-v0.1.0/p1-cycle-evidence/`):
/// the fork-settings import adopted `trio_practice` ON after the menu's open-time copy was made;
/// the release's close then reverted it to the copy (OFF) and every P1 cycle logged
/// "request ignored role=Player selection=Cycle: this role's roster cycle is off"
/// (`TrainingRooms__practice-diagnostics.log` #144; `TrainingRooms__settings.json`:
/// `trio_practice [0,1]`). MENU-CLOSE (`87a5105`): the close commits only the rows the player
/// changed, so the adopted row survives — and the P1 cycle's role gate
/// (`character_roster.rs:245`: `trio_practice == On`) is open again.
#[test]
fn an_adopted_trio_practice_survives_a_no_edit_close_and_the_p1_cycle_is_accepted() {
    // The menu opens on the settings where Player Swapping is OFF (the open-time copy).
    let mut opened = Settings::factory();
    opened.menu.set("trio_practice", &[0, 1]).unwrap();
    let mut menu = Menu::new(opened.clone(), Env::default());

    // While the menu is open, the first-boot fork import adopts trio_practice ON into the live
    // settings (`ui::worker::adopt_typed`, decision D1).
    let mut adopted = opened;
    adopted.menu.set("trio_practice", &[1, 0]).unwrap();

    // Open → close without touching the row: the close commits onto the live settings.
    let committed = menu.commit_onto(&adopted);
    assert_eq!(
        committed.menu.get("trio_practice").unwrap(),
        &[1, 0][..],
        "decision MENU-CLOSE: the adopted Player Swapping must survive a no-edit close"
    );
    // The P1 cycle's role gate reads exactly this row (the fork's `Role::enabled`,
    // `character_roster.rs:245`): ON means the next cycle request is not "ignored ... cycle is off".
    assert!(
        committed.menu.get("trio_practice").unwrap().first() == Some(&1),
        "the P1 cycle's role gate is open again"
    );
}
