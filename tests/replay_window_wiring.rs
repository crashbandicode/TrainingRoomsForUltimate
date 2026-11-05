//! Operator 2026-11-05: "yes port enable the fork's Replay Window retry / exact-Mii named macros". The decisions are host-tested
//! in `src/model/macro_ui/workflows_tests.rs`; the runtime is Switch-only, so the call-graph scanner (`tests/common/callgraph.rs`)
//! checks that every place the RC flag used to gate now asks those decisions, and that the menu row reaches the window.

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

fn names(g: &Graph, key: &str, ident: &str) -> bool {
    g.funcs[one(g, key)].body.iter().any(|t| matches!(t, Tok::Ident(s) if s == ident))
}

#[test]
fn the_rc_flag_is_gone_and_every_gate_asks_the_release_workflows() {
    let g = graph();
    assert!(!g.funcs.iter().any(|f| f.body.iter().any(|t| matches!(t, Tok::Ident(s) if s == "ADVANCED_RECORDING_WORKFLOWS"))));
    // The replay window: its opening and its actions (the track choice among them).
    for key in ["replay_capture::open_editor", "replay_capture::on_action"] {
        assert!(names(&g, key, "WORKFLOWS") && names(&g, key, "replay_window"), "{key}");
    }
    // The track's target and its Switch, and the CPU request the Switch makes.
    assert!(names(&g, "replay_capture::target_for", "clip_target"));
    assert!(names(&g, "replay_capture::gate_for", "clip_switchable"));
    assert!(names(&g, "replay_capture::service_reload_request", "may_load_on_cpu"));
    // The retry: an unresolved Mii track is resolved before the prompt.
    assert!(names(&g, "replay_capture::gate_selection", "resolve"));
    // Named macros: the load, the Switch offer, the CPU request, Keep.
    assert!(names(&g, "named_macros::commit_selected", "named_load_refusal"));
    assert!(names(&g, "named_macros::load_context", "may_load_on_cpu"));
    assert!(names(&g, "named_macros::service_character_request", "may_load_on_cpu"));
    assert!(names(&g, "named_macros::submit_loaded_character", "exact_mii_named"));
    assert!(names(&g, "named_macros::publish_queries", "keep_allowed"));
    // What stays closed.
    assert!(names(&g, "named_macros::open_editor", "placement"));
    for key in ["setups::request_snapshot", "setups::request_bind"] {
        assert!(names(&g, key, "setups"), "{key}");
    }
}

#[test]
fn the_replay_window_row_opens_the_window() {
    let g = graph();
    let body = &g.funcs[one(&g, "external::open_panel")].body;
    let at = (0..body.len())
        .find(|&k| matches!(&body[k], Tok::Ident(s) if s == "ReplayWindow"))
        .expect("external::open_panel handles Editor::ReplayWindow");
    let next = body[at..].iter().take(12).any(|t| matches!(t, Tok::Ident(s) if s == "open_editor"));
    assert!(next, "Editor::ReplayWindow => replay_capture::open_editor()");
}
