//! P1-CYCLE: every runtime walker of the Player cycle walks the one list (`engine::swap::cycle_members`) with the
//! admission the Player Mii owner publishes, and the menu worker reads that publication. The runtime is Switch-only,
//! so the call-graph scanner reads it. The list itself is host-tested in `src/editors/player_cycle_tests.rs`.

#[path = "common/callgraph.rs"]
mod callgraph;

use std::path::Path;

use callgraph::{Graph, Tok};

fn graph() -> Graph {
    Graph::scan(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"),
        &[],
    )
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
    g.funcs[one(g, key)]
        .body
        .iter()
        .any(|t| matches!(t, Tok::Ident(s) if s == ident))
}

const PLAYER: &str = include_str!("../src/runtime/mii/mii_ordinary_player.rs");
const PRELOAD: &str = include_str!("../src/runtime/roster/cycle_preload.rs");
const BODY_BANK: &str = include_str!("../src/runtime/roster/native_body_bank.rs");

/// The Player Mii owner's cycle walks `cycle_members` with the admission it computes under the same lock as the
/// request, never the unfiltered roster.
#[test]
fn the_player_mii_owner_walks_the_one_list_with_its_admission() {
    let g = graph();
    assert!(calls(
        &g,
        "mii_ordinary_player::begin",
        "mii_ordinary_player::cycle_admission"
    ));
    assert!(
        names(&g, "mii_ordinary_player::begin", "cycle_members")
            && names(&g, "mii_ordinary_player::begin", "admitted")
    );
    assert!(PLAYER.contains(
        "next_qualified_player_with_ice_climbers(&crate::engine::swap::cycle_members::cycle_members(menu.player_roster,admitted),"
    ));
    assert!(
        !PLAYER.contains("next_qualified_player_with_ice_climbers(&menu.player_roster.fighters()"),
        "the unfiltered roster is gone"
    );
    assert!(names(
        &g,
        "mii_ordinary_player::cycle_admission",
        "player_admission"
    ));
}

/// The admission the menu shows is published every Player frame from the same function, and cleared at a scene change.
#[test]
fn the_player_owner_publishes_the_admission_it_walks() {
    let g = graph();
    assert!(calls(
        &g,
        "mii_ordinary_player::service",
        "mii_ordinary_player::publish_admission"
    ));
    assert!(calls(
        &g,
        "mii_ordinary_player::service",
        "mii_ordinary_player::cycle_admission"
    ));
    assert!(calls(
        &g,
        "mii_ordinary_player::scene_changed",
        "mii_ordinary_player::publish_admission"
    ));
    assert!(calls(
        &g,
        "character_roster::player_cycle_admission",
        "mii_ordinary_player::published_admission"
    ));
    assert!(calls(
        &g,
        "character_roster::cycle_facts",
        "character_roster::player_cycle_admission"
    ));
}

/// Cycle Preload and the body bank's window plan walk the same list with the published Player admission.
#[test]
fn the_player_lanes_of_the_other_walkers_use_the_one_list() {
    let g = graph();
    assert!(
        names(&g, "cycle_preload::mixed_targets", "cycle_members")
            && names(&g, "cycle_preload::mixed_targets", "player_cycle_admission")
    );
    assert!(
        !PRELOAD.contains("ALL_MII"),
        "the Player lane no longer admits every Mii class"
    );
    assert!(
        names(&g, "native_body_bank::plan_budget", "cycle_members")
            && names(
                &g,
                "native_body_bank::plan_budget",
                "player_cycle_admission"
            )
    );
    assert!(!BODY_BANK.contains("ALL_MII"));
    let editor = include_str!("../src/editors/roster.rs");
    assert!(editor.contains("(RosterUse::Cycles, Role::Player) => self.player_cycle[k],"));
    let worker = include_str!("../src/runtime/ui/worker.rs");
    assert!(worker.contains("(tiles.player_cycle, tiles.cpu_cycle, tiles.slot_zero) ="));
}
