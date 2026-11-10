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

/// Step 2b: the menu worker's roster requests (the recording host's switch, the combo panel's switch at menu close) and its read of
/// Player 1's fighter go through `roster::roster_requests`: the worker posts to a `DropRing` and reads a copy; the game frame
/// (`lobby::update`, after the settings re-plan, as the worker's loop ordered them) runs `character_roster::request_*_kind` and keeps
/// the copy current. No plugin thread reaches the roster's admission, so its locks (the role states, the Mii owners', the sources',
/// the planner's request stamps) stay game-side. The combo panel's answer now comes a game frame after the close: its only
/// consumers are the trace point and the refusal notice, made in `combos::service_switch_replies` each worker pass.
#[test]
fn the_menu_workers_roster_requests_run_on_the_game_frame() {
    let g = graph();
    let reach = reachable(&g, "worker::run");
    for game_side in [
        "character_roster::request_player_kind",
        "character_roster::request_cpu_kind",
        "character_roster::player_kind",
        "character_roster::queue_admitted",
    ] {
        assert!(!reach.contains(game_side), "the menu worker reaches {game_side} again");
    }
    for posted in ["roster_requests::request_player_kind", "roster_requests::request_player_kind_replied", "roster_requests::take_reply"] {
        assert!(reach.contains(posted), "the menu worker no longer posts through {posted}");
    }
    let lobby = include_str!("../src/runtime/lobby/mod.rs");
    ordered(body(lobby, "update"), &[
        "compat::menu::service_roster_change();",
        "roster::roster_requests::service();",
        "training::lobby::update();",
    ]);
    let combos = include_str!("../src/runtime/ui/combos.rs");
    let close = &combos[combos.find("pub fn after_close").unwrap()..];
    let close = &close[..close.find("\n    }\n").unwrap()];
    assert!(!close.contains("fighter_event") && !close.contains("notices::"), "after_close reads the roster's answer itself again");
    ordered(body(combos, "service_switch_replies"), &["roster::take_reply()", "live::fighter_event(event::REQUESTED", "notices::upsert(0x2A"]);
    let worker = include_str!("../src/runtime/ui/worker.rs");
    assert!(worker.contains("combos::service_switch_replies();"), "the worker no longer takes the roster's answers");
}

/// Step 2c: the combo helper's locks (`HELPER`, `RUN`, `PLAN`, `NAME`, `RENDERER`) are taken by game threads only, and the menu's
/// labels (`LABELS`) by the menu worker only. The worker posts what it changes (`STAGED`, `NAME_POSTED`, `PLAN_POST`) and reads
/// copies (`SAMPLE`, `PROGRESS`); each game thread that takes `PLAN` installs a posted plan first, and each that takes `NAME`
/// takes a posted name first, so it sees what the worker staged last, as when the worker wrote it under the lock: nothing lands
/// later. A staged target's label shows once the frame installed it (`STAGED_SEEN`), as when the frame wrote it. The game never
/// frees a plan: the replaced ones go back to the worker (`model::box_handoff`), which drops them each pass.
#[test]
fn the_combo_helpers_locks_are_game_side_and_the_worker_posts() {
    let g = graph();
    let worker_reach = reachable(&g, "worker::run");
    for game_side in ["combo::after_strip", "combo::session_clear", "combo::hud_tick", "combo::seq_view", "combo::note_step", "combo::take_posted_name"] {
        assert!(!worker_reach.contains(game_side), "the menu worker reaches {game_side}");
    }
    let combo = include_str!("../src/runtime/features/combo.rs");
    let game_locks = ["HELPER.", "RUN.", "PLAN.", "NAME.", "RENDERER."];
    for worker_side in ["set_target", "stage_target", "stage_sequence", "set_draft", "labels", "take_progress", "reap_plans", "last_combo", "strip_sample"] {
        let f = body(combo, worker_side);
        for lock in game_locks {
            assert!(!f.contains(lock), "the worker's {worker_side} takes {lock}:\n{f}");
        }
    }
    for game_side in ["after_strip", "session_clear", "hud_tick", "seq_view", "note_step"] {
        assert!(!body(combo, game_side).contains("LABELS"), "the game's {game_side} takes the menu's labels");
    }
    for game_side in ["after_strip", "session_clear", "seq_view"] {
        ordered(body(combo, game_side), &["PLAN.", "PLAN_POST.install("]);
    }
    ordered(body(combo, "after_strip"), &["NAME.lock()", "take_posted_name(", "*name = text"]);
    ordered(body(combo, "hud_tick"), &["NAME.try_lock()", "take_posted_name("]);
    ordered(body(combo, "labels"), &["settle()"]);
    ordered(body(combo, "stage_target"), &["settle()", "STAGED.publish(", "pending = Some((STAGED.generation()"]);
    ordered(body(combo, "note_step"), &["PROGRESS.write("]);
    let worker = include_str!("../src/runtime/ui/worker.rs");
    ordered(body(worker, "sync_sequence_drill"), &["combo::reap_plans();", "APPLIED.lock()"]);
}
