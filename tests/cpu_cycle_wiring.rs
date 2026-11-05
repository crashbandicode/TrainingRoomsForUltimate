//! P-CYCLE (operator 2026-11-05: "mii gunner is in the cycle somehow for cpu even though he's not listed in the menu"):
//! every runtime walker of a cycle walks the one list (`engine::swap::cycle_members`), the CPU Mii owner publishes the
//! admission it walks, and the menu worker reads that publication and the runtime's slot 0. The runtime is Switch-only,
//! so the call-graph scanner (`tests/common/callgraph.rs`) reads it; the list itself is host-tested in
//! `src/model/swap/cycle_members_tests.rs`.

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

const CPU_OWNER: &str = include_str!("../src/runtime/mii/mii_ordinary_cpu.rs");
const PRELOAD: &str = include_str!("../src/runtime/roster/cycle_preload.rs");
const BODY_BANK: &str = include_str!("../src/runtime/roster/native_body_bank.rs");
const ROSTER: &str = include_str!("../src/runtime/roster/character_roster.rs");

/// The CPU Mii owner's cycle walks `cycle_members` with the admission it computes from its own state under the same
/// lock as the rest of the request, never the unfiltered roster.
#[test]
fn the_cpu_mii_owner_walks_the_one_list_with_its_admission() {
    let g = graph();
    assert!(calls(&g, "mii_ordinary_cpu::begin", "mii_ordinary_cpu::cycle_admission"));
    assert!(names(&g, "mii_ordinary_cpu::begin", "cycle_members") && names(&g, "mii_ordinary_cpu::begin", "admitted"));
    assert!(CPU_OWNER.contains("mii_cycle::next_qualified_cpu(&crate::engine::swap::cycle_members::cycle_members(menu.cpu_roster,admitted),"));
    assert!(!CPU_OWNER.contains("next_qualified_cpu(&menu.cpu_roster.fighters()"), "the unfiltered roster is gone");
    assert!(names(&g, "mii_ordinary_cpu::cycle_admission", "cpu_admission"));
}

/// The admission the menu shows is published every CPU frame from the same function, and cleared at a scene change.
#[test]
fn the_owner_publishes_the_admission_it_walks() {
    let g = graph();
    assert!(calls(&g, "mii_ordinary_cpu::service", "mii_ordinary_cpu::publish_admission"));
    assert!(calls(&g, "mii_ordinary_cpu::service", "mii_ordinary_cpu::cycle_admission"));
    assert!(calls(&g, "mii_ordinary_cpu::scene_changed", "mii_ordinary_cpu::publish_admission"));
    assert!(calls(&g, "character_roster::cpu_cycle_admission", "mii_ordinary_cpu::published_admission"));
}

/// Cycle Preload and the body bank's window plan walk the same list with the published CPU admission; the plain loader's
/// `roster_fixed` walks it with no Mii (it cannot load one).
#[test]
fn every_other_walker_uses_the_one_list() {
    let g = graph();
    assert!(names(&g, "cycle_preload::mixed_targets", "cycle_members") && names(&g, "cycle_preload::mixed_targets", "cpu_cycle_admission"));
    assert!(!PRELOAD.contains("menu.cpu_roster.fighters_excluding_fixed(None)"));
    assert!(names(&g, "native_body_bank::plan_budget", "cycle_members") && names(&g, "native_body_bank::plan_budget", "cpu_cycle_admission"));
    assert!(!BODY_BANK.contains("menu.cpu_roster.fighters_excluding_fixed(None)"));
    assert!(names(&g, "character_roster::roster_fixed", "cycle_members_excluding") && names(&g, "character_roster::roster_fixed", "NO_MII"));
    assert!(!ROSTER.contains("filter(|f| !(0x48..=0x4a).contains(&f.kind()))"), "the private Mii filter is the one list's now");
}

/// The menu reads what cycles: the CPU cycle's admission and each role's slot 0, from atomics (never an owner).
#[test]
fn the_menu_reads_the_runtime_admission_and_slot_zero() {
    let g = graph();
    assert!(calls(&g, "worker::mii_tiles", "character_roster::cycle_facts"));
    assert!(calls(&g, "character_roster::cycle_facts", "character_roster::cpu_cycle_admission"));
    assert!(calls(&g, "character_roster::cycle_facts", "character_roster::original_kind"));
    let editor = include_str!("../src/editors/roster.rs");
    assert!(editor.contains("let original = self.mii.slot_zero[r].or_else("), "the 0 is the runtime's original");
    assert!(editor.contains("(RosterUse::Cycles, Role::Cpu) => self.cpu_cycle[k],"));
    assert!(editor.contains("let members = cycle_members(roster_of(values), admitted);"));
}
