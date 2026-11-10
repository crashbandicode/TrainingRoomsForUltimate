//! The lock-free runtime (operator order 2026-11-10 00:44Z, `artifacts/phase23/TASK-OPUS-rc6-lockfree.md`): no plugin thread takes
//! a lock that a game entry point waits on. `tests/lock_order.rs` (`the_lockfree_inventory_only_shrinks`) counts what is left; this
//! pins each hand-off the steps made: what a plugin thread now posts, where the game frame takes it, and that what now lands a frame
//! later has no plugin-side reader of its outcome. The runtime is Switch-only, so the call-graph scanner reads it.

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

/// The body of `name` in `src`, from its `fn` line to the next line that is a lone `}` at column 0.
fn body<'a>(src: &'a str, name: &str) -> &'a str {
    let at = src.find(&format!("fn {name}(")).unwrap_or_else(|| panic!("fn {name}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

fn ordered(src: &str, steps: &[&str]) {
    let mut from = 0;
    for step in steps {
        let at = src[from..].find(step).unwrap_or_else(|| panic!("missing or out of order: {step}\n{src}"));
        from += at + step.len();
    }
}

/// Step 2a: a settings publish (the menu worker's commit, the diag reload) only notes the roster's re-plan and the preload
/// planner's reset; the game frame (`lobby::update`, the input poll and the fighter frame on MainThread) runs both, in the order the
/// publish made them, before the scene clock. No plugin thread reaches either, so the roster's, the planner's and the sources' locks
/// stay game-side. They land at the game's next poll instead of right after the publish: the worker never reads their outcome (the
/// roster's request and the planner's state are read by game hooks only; their notices go through the notice queue either way).
#[test]
fn a_settings_publish_hands_the_re_plans_to_the_game_frame() {
    let g = graph();
    for plugin in ["worker::run", "published::publish", "diag_reload::poll"] {
        if g.find(plugin).is_empty() {
            continue;
        }
        let reach = reachable(&g, plugin);
        for game_side in ["character_roster::settings_changed", "cycle_preload::settings_changed"] {
            assert!(!reach.contains(game_side), "{plugin} reaches {game_side}: a plugin thread takes the roster's locks again");
        }
    }
    let lobby = include_str!("../src/runtime/lobby/mod.rs");
    ordered(body(lobby, "update"), &[
        "cycle_preload_config::service();",
        "compat::menu::service_roster_change();",
        "training::lobby::update();",
    ]);
    let menu = include_str!("../src/runtime/compat/menu.rs");
    let note = body(menu, "note_roster_change");
    assert!(!note.contains(".lock()"), "the publish takes a lock to note the re-plan: {note}");
    ordered(body(menu, "service_roster_change"), &["ROSTER_CHANGE.read()", "compare_exchange", "settings_changed(&previous, None)"]);
    let config = include_str!("../src/runtime/roster/cycle_preload_config.rs");
    assert!(!body(config, "store").contains("cycle_preload::settings_changed"), "the publish resets the planner itself again");
    let worker = include_str!("../src/runtime/ui/worker.rs");
    assert!(!worker.contains("run_roster_change"), "the menu worker runs the roster's re-plan again");
}

/// The re-plan now runs in a game hook, so it allocates nothing (the playbook's hook rule): `settings_changed` and `locate` use the
/// fixed rosters (`roster_fixed`, `KindList`, `DebugIter`), never the `Vec` form.
#[test]
fn the_roster_re_plan_allocates_nothing() {
    let roster = include_str!("../src/runtime/roster/character_roster.rs");
    for name in ["settings_changed", "locate"] {
        let f = body(roster, name);
        for heap in ["roster(", "collect", "Vec", "format!", "to_string", "to_owned"] {
            let hits: Vec<&str> = f.match_indices(heap).filter(|(at, _)| heap != "roster(" || !f[..*at].ends_with('_')).map(|(_, s)| s).collect();
            assert!(hits.is_empty(), "{name} uses {heap} (a heap allocation in a game hook):\n{f}");
        }
    }
}
