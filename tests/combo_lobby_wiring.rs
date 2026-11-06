//! Lobby-practice combos (`p23/combos-lobby`). The runtime is Switch-only, so the call-graph scanner reads it.
//! The model is included by path: the crate is a cdylib, so this binary cannot link it.

#[path = "common/callgraph.rs"]
mod callgraph;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::collections::BTreeSet;
use std::path::Path;

use callgraph::{Graph, Tok};
use model::combo::{Aim, Combo, Helper, LinkRec, Settings, Step, STEPS};
use model::frame_strip::{Button, Move, Tracker, Verdict};
use model::scene::{combo_name_keyboard, scene, Facts};

fn graph() -> Graph {
    Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[])
}

fn one(g: &Graph, key: &str) -> usize {
    let found = g.find(key);
    assert_eq!(found.len(), 1, "{key}: {found:?}");
    found[0]
}

fn calls(g: &Graph, from: &str, to: &str) -> bool {
    let target = one(g, to);
    g.callees(one(g, from)).contains(&target)
}

fn names(g: &Graph, key: &str, ident: &str) -> bool {
    g.funcs[one(g, key)].body.iter().any(|t| matches!(t, Tok::Ident(s) if s == ident))
}

fn reachable(g: &Graph, key: &str) -> BTreeSet<String> {
    let mut pending = vec![one(g, key)];
    let mut seen = BTreeSet::new();
    while let Some(index) = pending.pop() {
        if seen.insert(index) {
            pending.extend(g.callees(index));
        }
    }
    seen.into_iter().map(|index| g.funcs[index].key()).collect()
}

fn reference() -> Combo {
    let mut steps = [None; STEPS];
    steps[0] = Some(Step { button: Button::Attack, aim: Aim::Up, offset: 1 });
    let link = LinkRec {
        from: Move::Jab,
        to: Move::UpTilt,
        verdict: Verdict::True(Some(-3)),
        window: Some((0, 4)),
        steps,
    };
    Combo::from_links([link, LinkRec { from: Move::UpTilt, ..link }]).unwrap()
}

/// The combo rows open in a waiting scene. Recording is the strip, which already
/// samples there. Save and the list are the rows. The waiting-scene refusal is the bug.
#[test]
fn combo_rows_open_in_waiting_room_practice() {
    let text = include_str!("../src/runtime/ui/combos.rs");
    assert!(!text.contains("Combos work in Training"), "open still refuses a waiting scene");
    let g = graph();
    assert!(!names(&g, "combos::open", "fact_waiting_scene"), "open still branches on the waiting scene");
    assert!(calls(&g, "combos::open", "combos::save"), "Save last combo is a combo row");
    assert!(calls(&g, "combos::open", "combos::show_list"), "the list is a combo row");
    assert!(names(&g, "combos::save", "save_new"), "save writes a combo file");
    assert!(names(&g, "combos::save", "default_name"), "an unnamed save still has a name");
    assert!(names(&g, "combos::save", "last_combo"), "save stores the combo the strip recorded");
    let lib = text.split("fn library()").nth(1).unwrap();
    assert!(lib.contains("storage::ROOT") && lib.contains("{DIR}"), "save writes under the plugin's combo directory");
}

/// The strip samples in lobby practice, the shared pane table carries the bar, and grading
/// is the same step Training uses.
#[test]
fn the_helper_grades_attempts_in_lobby_practice() {
    let lobby = Facts { native_training: false, lobby_active: true, busy: false };
    let training = Facts { native_training: true, lobby_active: false, busy: false };
    assert_eq!(scene(lobby).map(|s| format!("{s:?}")), Some("Lobby".into()));
    assert!(scene(training).is_some() && scene(lobby).is_some());
    assert!(model::panes::specs().iter().any(|spec| spec.name.as_str() == "TrCb"));
    let builder = include_str!("../tools/layout-builder/src/main.rs");
    assert!(builder.contains("let mut specs = panes::specs();"), "both layouts start from the same pane table");
    let g = graph();
    assert!(calls(&g, "cpu::fighter_frame", "input_display::sample_frame"));
    assert!(names(&g, "cpu::fighter_frame", "role_scene"));
    assert!(names(&g, "cpu::role_scene", "Lobby"), "lobby practice is a scene the strip samples");
    assert!(calls(&g, "input_display::sample_frame", "combo::after_strip"));
    assert!(names(&g, "input_display::sample", "is_stop"));
    assert!(names(&g, "input_display::sample", "attack_is_attack"));
    assert!(names(&g, "input_display::sample", "motion_frame"));
    assert!(names(&g, "input_display::reaction_left", "get_float"));
    assert!(calls(&g, "hud_draw::draw", "combo::hud_tick"), "the bar draws when its container is present");
    let settings = Settings { on: true, ..Settings::factory() };
    let tracker = Tracker::new();
    let mut in_training = Helper::new();
    let mut in_lobby = Helper::new();
    in_training.set_target(Some(reference()));
    in_lobby.set_target(Some(reference()));
    assert_eq!(in_training.step(&tracker, settings), in_lobby.step(&tracker, settings));
}

/// Switch/Keep uses the prompt and the Player loader the waiting room already uses for a character change.
#[test]
fn combo_switch_prompt_works_in_lobby_practice() {
    let text = include_str!("../src/runtime/ui/combos.rs");
    assert!(!text.contains("Combos work in Training"), "the prompt is unreachable while open refuses a waiting scene");
    let g = graph();
    assert!(calls(&g, "combos::choose", "named_macros::open_combo_fighter_prompt"));
    assert!(calls(&g, "combos::after_close", "character_roster::request_player_kind"));
    let roster = include_str!("../src/runtime/roster/character_roster.rs");
    assert!(
        roster.contains("cfg!(feature = \"waiting-roster-reload\") && lobby::active()"),
        "P1's loader admits an active lobby"
    );
    let cargo = include_str!("../Cargo.toml");
    let default = cargo.lines().find(|line| line.trim_start().starts_with("default = ")).unwrap();
    assert!(default.contains("enhancements-alpha"));
    assert!(cargo.contains("waiting-roster-reload"), "the shipped features include the lobby loader");
}

/// The menu path writes the plugin's combo files. It does not reach a save-state request.
/// The strip's only such request is the helper's existing Load save state, already taken from the lobby sample.
#[test]
fn combos_write_no_game_state_online() {
    let g = graph();
    for entry in ["combos::open", "combos::save", "combos::show_list", "combos::offer", "combos::name_keyboard_opens"] {
        let reached = reachable(&g, entry);
        for writer in ["request_load", "request_save"] {
            assert!(
                !reached.iter().any(|key| key.ends_with(&format!("::{writer}"))),
                "{entry} reaches {writer}: {reached:?}"
            );
        }
    }
    assert!(names(&g, "combos::save", "save_new"));
    assert!(names(&g, "combos::library", "ROOT"));
    let strip = reachable(&g, "input_display::sample_frame");
    let writers: Vec<_> = strip
        .iter()
        .filter(|key| key.ends_with("::request_load") || key.ends_with("::request_save"))
        .cloned()
        .collect();
    assert_eq!(writers, ["states::request_load"], "the strip's only save-state request is the helper's existing load");
    assert!(strip.iter().any(|key| key == "combo::after_strip"));
}

/// An online waiting room does not open a combo keyboard. Local wireless and Training still do.
#[test]
fn online_lobby_combo_naming_does_not_open_the_keyboard() {
    assert!(!combo_name_keyboard(true, 0x0701_0100));
    assert!(!combo_name_keyboard(true, 0x0701_0400));
    assert!(!combo_name_keyboard(true, 0x0701_0200), "the whole online waiting family");
    assert!(combo_name_keyboard(true, 0x0201_0000), "local wireless keeps the inline keyboard");
    assert!(combo_name_keyboard(false, 0x0701_0100), "Training is not a waiting scene");
    assert!(combo_name_keyboard(false, 0x0404_0000));
    let g = graph();
    assert!(calls(&g, "combos::open", "combos::name_keyboard_opens"));
    assert!(names(&g, "combos::name_keyboard_opens", "combo_name_keyboard"));
    assert!(names(&g, "combos::name_keyboard_opens", "fact_waiting_scene"));
    let text = include_str!("../src/runtime/ui/combos.rs");
    let open = text.split("pub fn open").nth(1).unwrap().split("fn name_keyboard_opens").next().unwrap();
    assert!(open.contains("if !Self::name_keyboard_opens()"));
    assert!(!open.contains("offer_external_name_titled"), "the closed path does not open a keyboard");
    assert!(open.contains("The next combo gets a Combo N name"));
}

/// The saved-combo list still waits until the practice panel is up. The inbox approved keeping it.
#[test]
fn saved_combos_wait_for_the_practice_fighters() {
    let text = include_str!("../src/runtime/ui/combos.rs");
    let list = text.split("fn show_list").nth(1).unwrap();
    assert!(list.contains("// restriction: combo-save-wait-load"));
    assert!(list.contains("Wait for the practice fighters to finish loading"));
    assert!(names(&graph(), "combos::show_list", "panel_up"));
    let registry = include_str!("../docs/RESTRICTIONS.md");
    assert!(registry.contains("combo-save-wait-load"));
    let row = registry.lines().find(|line| line.contains("combos-training-only")).expect("the row stays");
    assert!(row.contains("retired"), "{row}");
    assert!(row.contains("superseded by combos-lobby in rc.5"), "{row}");
}
