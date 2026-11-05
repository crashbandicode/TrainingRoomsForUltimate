//! P-TIMER (timer-recording truncation, `docs/DECISIONS.md` DEFERRED-LIST item 7): the fix is in the capture model
//! (`LiveCapturePool::push` keeps a record going across a stretch with no live play, as a `CaptureBreak`), the archive
//! and the clip editor, all host-tested beside them. The runtime is Switch-only, so the call-graph scanner checks that
//! the adapter it relies on is unchanged: sampling still happens only in a live scene, every sample still goes through
//! the pool, and the adapter writes no break of its own.

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
fn sampling_stays_gated_on_a_live_scene_and_goes_through_the_pool() {
    let g = graph();
    assert!(names(&g, "live_replay::live_scene", "is_ready_go"), "no sample outside READY..GO play");
    assert!(names(&g, "live_replay::before_frame", "live_scene") && names(&g, "live_replay::before_frame", "wants_sample"));
    assert!(names(&g, "live_replay::after_frame", "live_scene") && names(&g, "live_replay::after_frame", "push"));
}

#[test]
fn the_runtime_records_no_break_of_its_own() {
    let g = graph();
    for f in &g.funcs {
        assert!(!f.body.iter().any(|t| matches!(t, Tok::Ident(s) if s == "CaptureBreak" || s == "breaks")),
            "{}::{}: breaks belong to the capture model", f.file, f.name);
    }
}
