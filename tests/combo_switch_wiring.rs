//! The runtime wiring of a combo's fighter question (`p23/combo-switch`, SPEC-input-07 §4a; operator 2026-11-05: "the
//! switch p1 to a combo's recorded fighter should follow the same logic that the replay macro uses"). The runtime is
//! Switch-only, so the call-graph scanner (`tests/common/callgraph.rs`) reads it; the plan and the shared prompt are
//! host-tested in `src/editors/combo_fighter.rs`.

#[path = "common/callgraph.rs"]
mod callgraph;

use std::path::Path;

use callgraph::{Graph, Tok};

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

/// Whether `key`'s body names `ident`.
fn names(g: &Graph, key: &str, ident: &str) -> bool {
    g.funcs[one(g, key)].body.iter().any(|t| matches!(t, Tok::Ident(s) if s == ident))
}

/// A in Saved combos: the plan reads P1's fighter from the roster loader and the named macros' *Ask before switching
/// fighter* setting (not Player Swapping, report §2a), and a mismatch opens the shared Fighter Mismatch prompt with the
/// `Combo` origin.
#[test]
fn a_chosen_combo_is_asked_about_through_the_shared_prompt() {
    let g = graph();
    assert!(calls(&g, "combos::tick", "combos::choose"));
    for fact in ["plan", "player_kind", "fighter_prompt", "fighter_label"] {
        assert!(names(&g, "combos::choose", fact), "choose reads {fact}");
    }
    assert!(!names(&g, "combos::choose", "player_swapping") && !names(&g, "combos::choose", "trio_practice"));
    assert!(calls(&g, "combos::choose", "named_macros::open_combo_fighter_prompt"));
    assert!(names(&g, "named_macros::open_combo_fighter_prompt", "open_fighter_prompt_for"));
    assert!(names(&g, "named_macros::open_combo_fighter_prompt", "Combo"));
    // Nothing is written before the answer: the target is set by `target`, which a question defers.
    assert!(calls(&g, "combos::choose", "combos::target") && calls(&g, "combos::answered", "combos::target"));
    assert!(names(&g, "combos::target", "select"));
}

/// The answer comes back through the panel's effect to the combos UI: Cancel opens the list again, Keep and Switch set
/// the target, and Switch is remembered for the menu's close.
#[test]
fn the_answer_reaches_the_combos_ui() {
    let g = graph();
    assert!(names(&g, "named_macros::apply_effect", "ComboFighter") && names(&g, "named_macros::apply_effect", "COMBO_FIGHTER"));
    assert!(names(&g, "named_macros::take_combo_fighter_choice", "COMBO_FIGHTER"));
    assert!(calls(&g, "combos::tick", "named_macros::take_combo_fighter_choice"));
    assert!(calls(&g, "combos::tick", "combos::answered"));
    assert!(calls(&g, "combos::answered", "combos::show_list"), "Cancel goes back to the list");
    assert!(names(&g, "combos::answered", "switch_at_close"));
    assert!(names(&g, "combos::choose", "switch_at_close"), "a later choice replaces an unsent switch");
}

/// Switch goes to the roster loader's Player role when the menu has closed: the Direct request a Player Character pick
/// makes, as one explicit load (`once`) that Player Swapping Off does not refuse, with no native call of its own.
#[test]
fn switch_asks_the_player_role_once_when_the_menu_closes() {
    let g = graph();
    assert!(calls(&g, "worker::close_session", "combos::after_close"));
    // The lock-free runtime (step 2b): the worker posts the switch; the game frame makes the roster's request.
    assert!(calls(&g, "combos::after_close", "roster_requests::request_player_kind_replied"));
    assert!(calls(&g, "roster_requests::service", "character_roster::request_player_kind"));
    assert!(calls(&g, "character_roster::request_player_kind", "character_roster::queue_admitted"));
    for ident in ["Player", "Direct", "pending"] {
        assert!(names(&g, "character_roster::request_player_kind", ident), "request_player_kind names {ident}");
    }
    assert!(!names(&g, "character_roster::request_player_kind", "enabled"), "the cycle setting does not refuse it");
    let text = include_str!("../src/runtime/roster/character_roster.rs");
    assert!(text.contains("queue_admitted(Role::Player, Selection::Direct(fighter), false, true);"));
    assert!(names(&g, "character_roster::player_kind", "observed"));
}

/// Player Swapping Off still keeps the plugin from changing P1 on its own: the cycle chord, the Player Character pick
/// and the waiting restore go through `queue` / `queue_request` (`once` false), both admission checks still read the
/// setting, the preload look-ahead still stops, and nothing but `request_player_kind` asks for a one-shot load.
#[test]
fn player_swapping_off_still_never_cycles_p1_by_itself() {
    let g = graph();
    let admitted = one(&g, "character_roster::queue_admitted");
    let mut callers: Vec<String> =
        (0..g.funcs.len()).filter(|&i| g.callees(i).contains(&admitted)).map(|i| g.funcs[i].key()).collect();
    callers.sort();
    assert_eq!(callers, ["character_roster::queue_request", "character_roster::request_player_kind"]);
    let text = include_str!("../src/runtime/roster/character_roster.rs");
    assert!(text.contains("    queue_admitted(role, selection, restoring, false);\n"), "queue_request is never once");
    assert!(calls(&g, "character_roster::queue", "character_roster::queue_request"));
    assert!(calls(&g, "character_roster::handle_input", "character_roster::queue"), "the P1 cycle chord");
    assert!(!calls(&g, "character_roster::handle_input", "character_roster::queue_admitted"));
    assert!(calls(&g, "character_roster::settings_changed", "character_roster::queue"), "the Player Character pick");
    assert!(!calls(&g, "character_roster::settings_changed", "character_roster::queue_admitted"));
    // Both admission checks read the setting and accept only an explicit one-shot in its place.
    assert!(text.contains("if !role.enabled() && !once && !matches!(selection, Selection::RefreshPartners) {"));
    assert!(text.contains("|| (!role.enabled() && !request.once && !matches!(request.selection, Selection::RefreshPartners))"));
    assert!(names(&g, "character_roster::preload_targets", "enabled"), "no look-ahead for a role whose cycle is off");
    assert!(text.contains("self == Self::Cpu || setting(&MENU).trio_practice == OnOff::ON"), "the setting itself is unchanged");
}

/// An unanswered question is retired when the selection moves on or the scene ends, like the other origins.
#[test]
fn a_stale_combo_question_is_retired() {
    let g = graph();
    assert!(names(&g, "named_macros::fighter_prompt_stale", "Combo"));
    assert!(names(&g, "named_macros::fighter_prompt_stale", "prompt_revision"));
    assert!(names(&g, "combos::prompt_revision", "PROMPT_REVISION"));
    assert!(names(&g, "combos::leave_panel", "PROMPT_REVISION"));
    assert!(names(&g, "combos::choose", "PROMPT_REVISION"));
}
