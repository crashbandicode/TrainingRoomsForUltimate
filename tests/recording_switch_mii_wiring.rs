//! Switch on a library macro recorded on an exact Mii (primary 2026-11-08, #54 (a)): rr_host names the macro's
//! `playback_target()` (`Tick::switch_mii`, host-tested in `rr_host`), and the Switch wrapper hands it to the existing
//! replay-catalog request on the game callback, through the Mii machinery's API only. Eden `rc6-covrec-diag2`
//! cov-named-mii: Switch went to `request_cpu_kind` and loaded a default Mii of the class. The runtime is Switch-only,
//! so the call-graph scanner (`tests/common/callgraph.rs`) reads it.

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
fn an_exact_mii_switch_goes_to_the_replay_catalog_request() {
    let g = graph();
    assert!(names(&g, "recording_host::apply", "switch_mii"), "the wrapper reads the macro's target");
    assert!(names(&g, "recording_host::apply", "SWITCH_MII"));
    assert!(names(&g, "recording_host::service_switch_request", "SWITCH_MII"));
    assert!(names(&g, "recording_host::service_switch_request", "request_named"), "the existing replay-catalog request");
    assert!(names(&g, "recording_host::service_switch_request", "cpu_reload_ready"));
    assert!(!names(&g, "recording_host::service_switch_request", "request_cpu_kind"), "never the kind-only route");
    // The game callback that serves the named layer's request serves this one too.
    let text = include_str!("../src/runtime/macros/mod.rs");
    assert!(text.contains(
        "named_macros::service_character_request();\n            crate::runtime::features::recording_host::service_switch_request();"
    ));
}
