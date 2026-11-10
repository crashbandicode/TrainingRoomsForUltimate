//! The Give item request's hand-off (rc.6 `6ef998f7`; docs lane review R1, finding 3). The menu worker posts the item
//! (`ui::requests::give_item`); Player 1's next game frame queues it (`lobby::frame` -> `requests::take_give_item` ->
//! `character_items::request`), not the UI tick, which runs on the game's task workers. A request must not outlive its scene: the
//! roster's scene change (`character_items::scene_changed`) drops it, so a later scene's first frame neither grants an item nobody
//! asked for there nor shows "Wait for practice to be ready" in a waiting room. The runtime is Switch-only, so the call-graph
//! scanner reads it.

#[path = "common/callgraph.rs"]
mod callgraph;

use std::collections::BTreeSet;
use std::path::Path;

use callgraph::Graph;

fn graph() -> Graph {
    Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[])
}

fn one(g: &Graph, key: &str) -> usize {
    let found = g.find(key);
    assert_eq!(found.len(), 1, "{key}: {found:?}");
    found[0]
}

fn calls(g: &Graph, from: &str, to: &str) -> bool {
    g.callees(one(g, from)).contains(&one(g, to))
}

fn reachable(g: &Graph, key: &str) -> BTreeSet<String> {
    let mut pending = vec![one(g, key)];
    let mut seen = BTreeSet::new();
    let mut keys = BTreeSet::new();
    while let Some(index) = pending.pop() {
        if seen.insert(index) {
            keys.insert(g.funcs[index].key());
            pending.extend(g.callees(index));
        }
    }
    keys
}

#[test]
fn a_scene_change_drops_a_pending_give_item_request() {
    let g = graph();
    assert!(
        calls(&g, "character_items::scene_changed", "requests::take_give_item"),
        "a Give item request posted in one scene would be queued on the next scene's first frame"
    );
}

#[test]
fn the_request_is_queued_on_player_1s_frame_not_on_the_ui_tick() {
    let g = graph();
    assert!(calls(&g, "lobby::frame", "requests::take_give_item"), "Player 1's frame no longer takes the request");
    assert!(calls(&g, "lobby::frame", "character_items::request"), "Player 1's frame no longer queues the item");
    assert!(
        !reachable(&g, "tick::tick").contains("character_items::request"),
        "the UI tick (the game's task workers) reaches the item engine again"
    );
}
