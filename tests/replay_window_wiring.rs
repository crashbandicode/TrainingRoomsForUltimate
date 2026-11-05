//! Operator 2026-11-05: the fork's recording workflows ship again: "yes port enable the fork's Replay Window retry /
//! exact-Mii named macros", then "Switch CPU to recorded fighter, Setup library, Edit placement". The decisions are
//! host-tested in the carried models (`src/model/macro_ui/replay_target_tests.rs`, `macro_panel.rs`, `editors/setups.rs`);
//! the runtime is Switch-only, so the call-graph scanner (`tests/common/callgraph.rs`) checks that no RC gate is left and
//! that each menu row reaches its editor.

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

/// The identifier that follows `ident` within `window` tokens in `key`'s body.
fn followed_by(g: &Graph, key: &str, ident: &str, next: &str, window: usize) -> bool {
    let body = &g.funcs[one(g, key)].body;
    (0..body.len()).any(|k| matches!(&body[k], Tok::Ident(s) if s == ident)
        && body[k..].iter().take(window).any(|t| matches!(t, Tok::Ident(s) if s == next)))
}

#[test]
fn no_rc_gate_is_left() {
    let g = graph();
    for flag in ["ADVANCED_RECORDING_WORKFLOWS", "WORKFLOWS"] {
        assert!(!g.funcs.iter().any(|f| f.body.iter().any(|t| matches!(t, Tok::Ident(s) if s == flag))), "{flag}");
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    for file in ["macros/replay_capture.rs", "macros/named_macros.rs", "macros/diag.rs", "ui/setups.rs", "features/states/setups.rs"] {
        let text = std::fs::read_to_string(src.join(file)).unwrap();
        assert!(!text.contains("unavailable in this release"), "{file} still refuses a workflow");
    }
}

#[test]
fn the_replay_track_and_named_macros_switch_as_the_fork_did() {
    let g = graph();
    // The track's target is the host-tested model function; the retry resolves an unresolved Mii before the prompt.
    assert!(names(&g, "replay_capture::target_for", "clip_target"));
    assert!(names(&g, "replay_capture::gate_selection", "resolve"));
    // The fork's Switch offer: any exact target this build loads (an ordinary fighter, or a Mii with the loader).
    assert!(names(&g, "named_macros::load_context", "can_switch"));
    assert!(!names(&g, "named_macros::commit_selected", "playback_target"), "no RC refusal before the prompt");
}

#[test]
fn every_row_opens_its_editor() {
    let g = graph();
    assert!(followed_by(&g, "external::open_panel", "ReplayWindow", "open_editor", 12));
    assert!(followed_by(&g, "external::open_panel", "EditPlacement", "open_editor", 12));
    assert!(followed_by(&g, "worker::is_setup", "SetupSave", "SetupLibrary", 6), "both setup rows are the setups panel");
    assert!(names(&g, "setups::request_snapshot", "enqueue") && names(&g, "setups::request_bind", "enqueue"));
}
