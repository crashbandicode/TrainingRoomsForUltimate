//! Phase 2 N11, Lobby Items: the row, its settings, defaults and import (through the real menu and settings code), and the
//! runtime's wiring (the call graph of the Switch-only runtime: the hook writes only x8, the removal runs once from Player 1's
//! frame, and nothing outside the waiting scene is touched). The policy is host-tested beside it (`src/model/lobby_items.rs`).
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "common/callgraph.rs"]
mod callgraph;

use std::path::Path;

use callgraph::{Graph, Tok};
use model::menu::{input::Button, table::{Control, Page, ROWS}, Entry, Env, Menu};
use model::settings::{parse, Settings};

#[test]
fn the_row_is_a_default_on_toggle_after_stage_hazards() {
    let settings = Settings::factory();
    assert!(settings.menu.lobby_items() && settings.defaults.lobby_items(), "default On: the native behaviour");
    let row = ROWS.iter().position(|row| row.id == "lobby_items").unwrap();
    assert_eq!(ROWS[row].title, "Lobby Items");
    assert_eq!(ROWS[row - 1].id, "stage_hazards");
    assert_eq!(ROWS[row].page(), Page::FightersStages);
    assert_eq!(ROWS[row].group, ROWS[row - 1].group, "Fighters & Stages > Stages");
    assert!(matches!(ROWS[row].control, Control::Toggle));
}

#[test]
fn older_files_import_on_and_the_value_round_trips_on_its_own() {
    for text in [r#"{"menu":{}}"#, r#"{"menu":{"stage_hazards":[0,1]}}"#, r#"{"menu":{"lobby_items":[0,0]}}"#] {
        let imported = parse(text);
        assert!(!imported.corrupt(), "{text}");
        assert!(imported.settings.menu.lobby_items(), "{text}: imports On");
    }
    assert!(!parse(r#"{"menu":{"lobby_items":[0,1]}}"#).settings.menu.lobby_items());
    let mut settings = Settings::factory();
    settings.menu.set("lobby_items", &[0, 1]).unwrap();
    assert!(!settings.menu.lobby_items() && settings.defaults.lobby_items());
    let loaded = parse(&settings.to_json());
    assert!(!loaded.corrupt());
    assert!(!loaded.settings.menu.lobby_items() && loaded.settings.defaults.lobby_items());
    settings.defaults = settings.menu.clone();
    assert!(!parse(&settings.to_json()).settings.defaults.lobby_items(), "saved as the defaults too");
    settings.menu = model::settings::Snapshot::factory();
    assert!(settings.menu.lobby_items() && !settings.defaults.lobby_items(), "a page reset restores the factory On");
}

#[test]
fn the_menu_toggles_it_with_a_and_leaves_the_other_rules() {
    let mut menu = Menu::new(Settings::factory(), Env::default());
    let row = ROWS.iter().position(|row| row.id == "lobby_items").unwrap();
    for _ in 0..6 {
        if menu.page() == Page::FightersStages {
            break;
        }
        menu.press(Button::R);
    }
    assert_eq!(menu.page(), Page::FightersStages);
    let focus = |menu: &mut Menu, entry| {
        for _ in 0..=menu.entry_count(menu.page()) {
            if menu.focus() == Some(entry) {
                return;
            }
            menu.press(Button::Down);
        }
        panic!("could not focus {entry:?}");
    };
    let group = usize::from(ROWS[row].group);
    if !menu.group_is_open(group) {
        focus(&mut menu, Entry::Group(group));
        menu.press(Button::A);
    }
    focus(&mut menu, Entry::Row(row));
    let hazards = menu.settings().menu.stage_hazards();
    menu.press(Button::A);
    assert!(!menu.settings().menu.lobby_items());
    assert!(menu.settings().defaults.lobby_items());
    menu.press(Button::A);
    assert!(menu.settings().menu.lobby_items());
    assert_eq!(menu.settings().menu.stage_hazards(), hazards, "an unrelated rule is unchanged");
}

fn graph() -> Graph {
    Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[])
}

fn body<'a>(g: &'a Graph, key: &str) -> &'a [Tok] {
    let found = g.find(key);
    assert_eq!(found.len(), 1, "{key}: {found:?}");
    &g.funcs[found[0]].body
}

fn names(body: &[Tok], ident: &str) -> bool {
    body.iter().any(|t| matches!(t, Tok::Ident(s) if s == ident))
}

#[test]
fn the_hook_only_zeroes_x8_in_a_waiting_scene_with_the_row_off() {
    let g = graph();
    let hook = body(&g, "lobby_items::spawn_switch");
    assert!(names(hook, "suppress") && names(hook, "waiting") && names(hook, "lobby_items"));
    assert!(names(hook, "set_x"), "it writes a register");
    for forbidden in ["write_volatile", "item_remove", "write", "set_w"] {
        assert!(!names(hook, forbidden), "the hook writes nothing but x8 ({forbidden})");
    }
    let waiting = body(&g, "lobby_items::waiting");
    assert!(names(waiting, "qualified_waiting") && names(waiting, "initialized"));
}

#[test]
fn the_removal_runs_from_player_ones_frame_through_the_gate() {
    let g = graph();
    let frame = body(&g, "lobby_items::frame");
    for ident in ["step", "item_gate_ready", "busy", "collect", "item_remove"] {
        assert!(names(frame, ident), "{ident}");
    }
    assert!(names(body(&g, "lobby::frame"), "frame"));
    let init = body(&g, "lobby_items::init");
    assert!(names(init, "gate") && names(init, "granted") && names(init, "install_hooks"));
    // The spawner hook never removes: removing inside the item manager's update is out.
    assert!(!names(body(&g, "lobby_items::spawn_switch"), "frame"));
}
