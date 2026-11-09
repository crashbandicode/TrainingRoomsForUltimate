//! Phase 2 N11, Lobby Items: the row, its settings, defaults and import (through the real menu and settings code), and the
//! runtime's wiring (the call graph of the Switch-only runtime: the hook writes only x8, the removal runs once from Player 1's
//! frame, and nothing outside the waiting scene is touched). The policy is host-tested beside it (`src/model/lobby_items.rs`).
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "common/callgraph.rs"]
mod callgraph;
#[path = "common/owned_elf.rs"]
mod owned_elf;

use std::path::Path;

use callgraph::{Graph, Tok};
use owned_elf::{branches_into, owned_elf, target, word};
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

// #62: the waiting area has a second item source, a timer-driven generator `F` (`main+0x1549930`) that the spawner's
// switch doesn't gate. It draws the item lottery (`bl 0x15dc450` at 0x15499b0), tests the verdict with
// `tbz w0,#0,0x15499fc` at 0x15499b4 and creates the item on a win (`bl 0x15beaa0` at 0x15499c8); 0x15499fc re-arms
// its timer either way. Eden `rc6-bg-qp5`: six items from `F` during a 61 s Off window.
const DROP_LOT: u64 = 0x15499b4;
const DROP_DECLINED: u64 = 0x15499fc;

const RUNTIME: &str = include_str!("../src/runtime/lobby/lobby_items.rs");
const SEAM_ROWS: &str = include_str!("../src/model/seams/native_seams/rows_lobby_items.rs");

/// The source text of `fn <name>` in the runtime file, up to its closing brace at column 0.
fn source_of(name: &str) -> &'static str {
    let start = RUNTIME.find(&format!("fn {name}(")).unwrap_or_else(|| panic!("fn {name} in lobby_items.rs"));
    let end = start + RUNTIME[start..].find("\n}\n").expect("closing brace");
    &RUNTIME[start..end]
}

#[test]
fn the_timed_drop_hook_declines_the_lottery_only_in_a_waiting_scene_with_the_row_off() {
    let g = graph();
    let hook = body(&g, "lobby_items::drop_lot");
    assert!(names(hook, "suppress") && names(hook, "waiting") && names(hook, "lobby_items"));
    assert!(names(hook, "LobbyItemsDrop"), "the callback is counted under its own hook id");
    for forbidden in ["write_volatile", "item_remove", "write", "set_w", "frame"] {
        assert!(!names(hook, forbidden), "the hook writes nothing but x0 ({forbidden})");
    }
    let text = source_of("drop_lot");
    assert_eq!(text.matches(".set_x(").count(), 1, "one register write");
    assert!(text.contains("registers[0].set_x(0)"), "Off hands the tbz a zero verdict: w0 = 0");
    assert!(RUNTIME.contains(&format!("#[skyline::hook(offset = {DROP_LOT:#x}, inline)]\nunsafe fn drop_lot(")));
    let init = body(&g, "lobby_items::init");
    assert!(names(init, "spawn_switch") && names(init, "drop_lot"), "init installs both hooks, or neither");
}

#[test]
fn the_timed_drop_rows_are_compared_before_the_hooks_install() {
    for (label, offset, kind) in [
        ("lobby_items.drop_lot", "0x15499b4", "SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid }"),
        ("lobby_items.drop_gate", "0x154997c", "SeamKind::Relied"),
    ] {
        let at = SEAM_ROWS.find(&format!("label: \"{label}\"")).unwrap_or_else(|| panic!("{label} has a seam row"));
        let row = &SEAM_ROWS[at..at + SEAM_ROWS[at..].find("note:").unwrap()];
        assert!(row.contains(&format!("offset: {offset},")) && row.contains(kind), "{label}: {row}");
        let rows = &RUNTIME[RUNTIME.find("const ROWS").unwrap()..];
        let rows = &rows[..rows.find("];").unwrap()];
        assert!(rows.contains(&format!("\"{label}\"")), "{label} is compared in lobby_items::init");
    }
}

#[test]
fn the_timed_generator_decides_on_its_lottery_at_the_hook_site() {
    let Some(d) = owned_elf() else { return };
    assert_eq!(target(0x15499b0, word(&d, 0x15499b0)), Some(0x15dc450), "bl 0x15dc450, the item lottery");
    let tbz = word(&d, DROP_LOT);
    assert_eq!(tbz & 0xfff8_001f, 0x3600_0000, "tbz w0, #0: the hook site tests bit 0 of the lottery's w0");
    assert_eq!(target(DROP_LOT, tbz), Some(DROP_DECLINED), "a zero verdict takes the declined path");
    // The creation call lies on the winning fall-through only, before the declined path.
    assert_eq!(target(0x15499c8, word(&d, 0x15499c8)), Some(0x15beaa0), "bl 0x15beaa0, the item creation");
    assert_eq!(word(&d, 0x15499c0), 0x5280_6d41, "mov w1, #0x36a: the creation's argument");
    // The declined path re-arms the timer: `stp x8, xzr, [x19, #0x38]` after the adrp/add of the next callback.
    assert_eq!(word(&d, 0x1549a04), 0xa903_fe68, "stp x8, xzr, [x19, #0x38]");
    // The relied gates: control = [[ItemManager+0xa0]], its byte +8, then the game-state byte.
    assert_eq!(word(&d, 0x154997c), 0xf940_5294, "ldr x20, [x20, #0xa0]");
    assert_eq!(word(&d, 0x1549984), 0x3940_2108, "ldrb w8, [x8, #0x8]");
}

#[test]
fn no_branch_lands_inside_the_timed_drop_hook_site() {
    let Some(d) = owned_elf() else { return };
    // Skyline's far form at an address 4 past an 8-byte boundary is 20 bytes (a leading nop).
    assert_eq!(DROP_LOT % 8, 4);
    let (hits, scanned) = branches_into(&d, DROP_LOT + 4..DROP_LOT + 20);
    assert!(scanned > 100_000, "the executable segment was found and scanned ({scanned} branches)");
    assert!(hits.is_empty(), "branches into the replaced words: {hits:x?}");
}
