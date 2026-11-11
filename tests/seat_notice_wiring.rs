//! F9-SEAT-NOTICE (primary 2026-11-08 23:03Z, reading A; cov-rest's question of 16:25Z): when process memory refuses a
//! parked seat, the seat is held and retried, but the readiness notice counted only the plan's targets and the bank's
//! groups, so the hold showed only in a diag line. The two seats now count in the existing fighter status ("… waiting
//! for memory"), with no new text. The runtime is Switch-only, so the call-graph scanner (`tests/common/callgraph.rs`)
//! reads it; the count itself is host-tested in `parked_entries`
//! (`a_seat_held_for_memory_counts_in_the_fighter_status`).

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

/// Whether `key`'s body names `ident`.
fn names(g: &Graph, key: &str, ident: &str) -> bool {
    g.funcs[one(g, key)].body.iter().any(|t| matches!(t, Tok::Ident(s) if s == ident))
}

/// The loader callback notes each speculative load on a seat as held for memory or admitted, before the refusal returns.
#[test]
fn the_loader_notes_a_seat_refused_for_memory() {
    let g = graph();
    // rc.8 R5: `service`'s load is `service_load`, in its own frame.
    assert!(names(&g, "character_sources::service_load", "note_seat_memory"), "service notes the seat's admission");
    let text = include_str!("../src/runtime/roster/character_sources.rs");
    let note = text.find("f9_gate::note_seat_memory(slot, wanted.kind,").expect("the note in service");
    let refused = text.find("memory_refused(slot, wanted.kind, decision, memory_cost, \"preload\");").expect("the refusal");
    assert!(note < refused, "the note comes before the refusal's early return");
    assert!(names(&g, "f9_gate::reset", "note_seat_memory"), "a scene's reset clears the held seats");
}

/// The fighter status shown in the notice counts the seats: their targets as the loader sees them, the Ready source on
/// each entry, and a standing memory refusal.
#[test]
fn the_fighter_status_counts_the_two_seats() {
    let g = graph();
    assert!(names(&g, "cycle_preload::poll_status", "with_seats"), "the status adds the seats");
    assert!(names(&g, "cycle_preload::poll_status", "seat_states"));
    for ident in ["override_seat", "parked_seat", "seat_member", "seat_held", "seat_state", "training_cpu_quantity"] {
        assert!(names(&g, "character_sources::seat_states", ident), "seat_states reads {ident}");
    }
}
