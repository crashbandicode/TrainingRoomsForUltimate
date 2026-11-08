//! #54, TASK-OPUS-rc6-open-fixes item 2 (primary 2026-11-08 23:03Z, reading B): after a Switch the CPU holds what the
//! install wrote (`mii_install::installable`), so every comparison of a recorded exact-Mii target with the CPU uses the
//! target as Switch installs it (`mii_install::installed_target`), exactly. The runtime is Switch-only, so the call-graph
//! scanner (`tests/common/callgraph.rs`) reads it.

#[path = "common/callgraph.rs"]
mod callgraph;

use std::path::Path;

use callgraph::{Graph, Tok};

fn graph() -> Graph {
    Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[])
}

fn names(g: &Graph, key: &str, ident: &str) -> bool {
    let found = g.find(key);
    assert_eq!(found.len(), 1, "{key}: {found:?}");
    g.funcs[found[0]].body.iter().any(|t| matches!(t, Tok::Ident(s) if s == ident))
}

#[test]
fn a_recorded_mii_is_compared_as_switch_installs_it() {
    let g = graph();
    // The fighter prompt (library load, the clip editor's track) and the readiness of a named or clip Switch.
    assert!(names(&g, "named_macros::recorded_target_matches", "installed"), "the prompt's match");
    assert!(names(&g, "mii_ordinary_cpu::replay_ready", "installed_target"), "the Switch's readiness");
    // The installed form is worked out on the game callback, where the CPU catalog is read; the worker reads a copy.
    assert!(names(&g, "identity::service", "installed_target"));
    assert!(names(&g, "mii_replay::installed_target", "replay_catalog"));
    assert!(names(&g, "mii_replay::installed_target", "installed_target"));
}
