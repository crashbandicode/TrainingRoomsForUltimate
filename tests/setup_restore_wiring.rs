//! A setup restore and the bound save (primary 2026-11-08, #69 (b): "Setups stay a separate library from save states",
//! LANE-GROK-RECORDING-REDESIGN-REPORT.md:322). Eden `rc6-covrec-diag2` cov-setup-restore: restoring a setup moved the
//! fighters and also wrote them into the bound save file, because the state writer handed the setup's slot to
//! `rr_host::save_captured` as if Save had captured it. The runtime is Switch-only, so the call-graph scanner
//! (`tests/common/callgraph.rs`) reads it; the worker's side is host-tested in `rr_host`
//! (`a_setup_restore_leaves_the_bound_save_and_the_slot_returns_to_it`).

#[path = "common/callgraph.rs"]
mod callgraph;

use std::path::Path;

use callgraph::{Graph, Tok};

fn graph() -> Graph {
    Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[])
}

fn body(g: &Graph, key: &str) -> Vec<String> {
    let found = g.find(key);
    assert_eq!(found.len(), 1, "{key}: {found:?}");
    g.funcs[found[0]]
        .body
        .iter()
        .filter_map(|t| match t {
            Tok::Ident(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// Only Save's own records reach the bound save. A setup bind tells the worker that the slot must get the bound save
/// back; its slot is never handed to `save_captured`.
#[test]
fn a_setup_restore_writes_no_save_file() {
    let g = graph();
    let take = body(&g, "states::take_captured");
    assert!(take.iter().any(|t| t == "setup_restored"), "a setup bind asks for the bound save again");
    assert!(take.iter().any(|t| t == "save_captured"), "Save still writes the bound save");
    let text = include_str!("../src/runtime/features/states.rs");
    let start = text.find("fn take_captured(").expect("take_captured");
    let end = start + text[start..].find("\npub fn start_writer").expect("start_writer follows");
    let take_text = &text[start..end];
    assert!(!take_text.contains("binding.slot()"), "the setup's slot is not a Save");
    // The pair Save captured is taken before a later setup bind changes the writer's copy of that slot.
    let pair = take_text.find("let saved_pair").expect("the Save pair is taken first");
    let trailing = take_text.find("if let Some(binding) = binding").expect("a setup bind after every record");
    assert!(pair < trailing, "the Save pair is read before the trailing setup bind is applied");
}
