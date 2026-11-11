//! The lock-free runtime (operator order 2026-11-10 00:44Z, `artifacts/phase23/TASK-OPUS-rc6-lockfree.md`): no plugin thread takes
//! a lock that a game entry point waits on. `tests/lock_order.rs` (`the_lockfree_inventory_only_shrinks`) counts what is left; this
//! pins each hand-off the steps made: what a plugin thread now posts, where the game frame takes it, and that what now lands a frame
//! later has no plugin-side reader of its outcome. The runtime is Switch-only, so the call-graph scanner reads it.

// rc.9 test cut: the call-graph scanner's own tests run once, in `tests/hook_locks.rs`.
macro_rules! callgraph_self_tests {
    ($($body:tt)*) => {};
}
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

/// Step 2d: the recording worker's named load and bound save reach the save states through `model::box_handoff`: the worker posts
/// a box (and drops the boxes the game emptied, each pass and at each post); game threads take the value out under a lock no
/// plugin thread takes (`NAMED_TAKE`, `STATE`) and give the box back, never freeing it. A take happens where the lock's take did
/// (the player's frame step; the session clear discards), so nothing lands later.
#[test]
fn the_save_states_take_the_workers_posts_without_its_locks() {
    let g = graph();
    let worker_reach = reachable(&g, "worker::run");
    for game_side in ["states::on_fighter_frame", "states::apply_bound_post", "states::clear_session", "states::take_pair"] {
        assert!(!worker_reach.contains(game_side), "the menu worker reaches {game_side}");
    }
    assert!(worker_reach.contains("states::reap_posts"), "the worker no longer drops the emptied boxes");
    let states = include_str!("../src/runtime/features/states.rs");
    for worker_side in ["request_named_load", "post_bound", "reap_posts"] {
        let f = body(states, worker_side);
        assert!(!f.contains(".lock()"), "the worker's {worker_side} takes a lock:\n{f}");
    }
    ordered(body(states, "request_named_load"), &["NAMED_LOAD.post(Some(Box::new(pair)))"]);
    ordered(body(states, "post_bound"), &["BOUND_POST.post(Some(Box::new("]);
    ordered(body(states, "on_fighter_frame"), &["apply_bound_post();", "NAMED_TAKE.lock()", "NAMED_LOAD.take(take_pair)", "begin_temporary(pair)"]);
    ordered(body(states, "apply_bound_post"), &["BOUND_POST.pending()", "with(|state|", "setup_busy()", "BOUND_POST.take("]);
    ordered(body(states, "clear_session"), &["NAMED_TAKE.lock()", "NAMED_LOAD.discard()"]);
    ordered(body(states, "take_pair"), &["core::mem::replace(pair, [SavedState::never_saved(), SavedState::never_saved()])"]);
    let host = include_str!("../src/runtime/features/recording_host.rs");
    ordered(body(host, "service"), &["states::reap_posts();", "let root"]);
}

/// Step 2e: the facts the setups transactions check (`setups::FACTS`) are a triple buffer the game writes (its writers take
/// turns on `FACTS_WRITER`, which no plugin thread takes) and the menu worker reads: the worker never takes a lock the P1 callback
/// waits on, and reads the facts the game wrote last, as under the lock. The worker is the buffer's one reader: `facts` is called
/// only by the transactions' entry points, and those only by the worker's Setups panel (`ui/setups.rs`, `native::`); the game's
/// functions (`service`, the clears) only write. (The coarse call graph here would follow `prepared.validate(..)` in `service` to the
/// panel's `impl Provider` method of that name: `lock_order`'s `PROVIDER`, not a call.)
#[test]
fn the_setups_facts_are_a_buffer_the_worker_reads() {
    let setups = include_str!("../src/runtime/features/states/setups.rs");
    let reads = |f: &str| f.match_indices("facts()").any(|(at, _)| !f[..at].ends_with(|c: char| c == '_' || c == '.' || c.is_alphanumeric()));
    let readers: Vec<&str> = ["open", "validate_selection", "request_snapshot", "request_bind", "service", "clear_facts", "retire_session", "write_facts", "poll"]
        .into_iter()
        .filter(|name| reads(body(setups, name)))
        .collect();
    assert_eq!(readers, ["open", "validate_selection", "request_snapshot", "request_bind"], "the facts buffer's readers");
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let g = graph();
    let callers: BTreeSet<String> = g
        .funcs
        .iter()
        .filter(|f| {
            let body = std::fs::read_to_string(runtime.join(&f.file)).unwrap_or_default();
            ["open(", "validate_selection(", "request_snapshot(", "request_bind("].iter().any(|n| body.contains(&format!("native::{n}")))
                && f.file != "features/states/setups.rs"
        })
        .map(|f| f.file.clone())
        .collect();
    assert_eq!(callers, BTreeSet::from(["ui/setups.rs".to_owned()]), "who calls the transactions' entry points");
    for game in ["states::on_fighter_frame", "states::clear_session", "states::retire_setup_session"] {
        assert!(reachable(&g, game).contains("setups::write_facts"), "{game} no longer writes the facts through write_facts");
    }
    ordered(body(setups, "write_facts"), &["FACTS_WRITER.lock()", "FACTS.write(facts)"]);
    ordered(body(setups, "facts"), &["FACTS.read().0"]);
    ordered(body(setups, "clear_facts"), &["write_facts(None)"]);
    ordered(body(setups, "service"), &["write_facts(current)", "JOBS.take_game()"]);
    assert!(!setups.contains("FACTS.lock()"), "a lock on the facts again");
}
/// Step 3: the heartbeat (`tr-heartbeat`: the drain, and the steps registered in `diag_log::TICKS`) takes no lock a game entry
/// waits on. The CPU-as-P1 fallback's receipts come through `model::owned_ring::OwnedRing`: the hook's posts take turns on
/// `OUTBOX_TURN` (which the heartbeat never takes) and the heartbeat is the ring's one consumer, moving each receipt out and freeing
/// it off the hook, as before. A full ring refuses the newest and counts it, as the fixed queue did.
#[test]
fn the_heartbeat_takes_the_hooks_receipts_without_a_lock() {
    let pilot = include_str!("../src/runtime/p1/cpu_pilot.rs");
    for heartbeat in ["service", "next_receipt", "write_next"] {
        let f = body(pilot, heartbeat);
        assert!(!f.contains("OUTBOX_TURN") && !f.contains("OUTBOX.lock()"), "the heartbeat's {heartbeat} takes the outbox's lock:\n{f}");
    }
    ordered(body(pilot, "post"), &["OUTBOX_TURN.lock()", "OUTBOX.push(outgoing)"]);
    ordered(body(pilot, "service"), &["while write_next() {}", "OUTBOX.take_refused()"]);
    ordered(body(pilot, "next_receipt"), &["OUTBOX.pop()"]);
    let g = graph();
    let heartbeat = reachable(&g, "cpu_pilot::service");
    assert!(!heartbeat.contains("cpu_pilot::post"), "the heartbeat posts to its own outbox");
}

/// Step 5a: the inline keyboard's callback signals (`inline_keyboard::SIGNALS`) are taken only with `try_lock`, by the menu worker's
/// keyboard and by the Mii editor's (game hooks) alike: no thread ever waits on them. The keyboard lease already made the takes
/// exclusive (every take is on the lease owner's thread, inside its calls), so a `try_lock` always succeeds there; if one ever
/// failed, a callback counts a missed signal, a poll returns no new signals this pass (they wait for the next), and an open fails
/// (the lease is released).
#[test]
fn the_keyboard_signals_are_only_ever_tried() {
    let keyboard = include_str!("../src/runtime/mii/native_keyboard/inline_keyboard.rs");
    assert!(!keyboard.contains("SIGNALS.lock()"), "a blocking take of the keyboard's signals again");
    for callback in ["changed", "moved", "entered", "cancelled", "initialized", "finished"] {
        let f = body(keyboard, callback);
        assert!(f.contains("signals(") || f.contains("SIGNALS.try_lock()"), "the {callback} callback no longer tries the signals:\n{f}");
    }
    ordered(body(keyboard, "signals"), &["SIGNALS.try_lock()", "MISSED.fetch_add("]);
    ordered(&keyboard[keyboard.find("pub(crate) unsafe fn open(").unwrap()..], &["Lease::acquire()?", "SIGNALS.try_lock()", "return None"]);
    ordered(&keyboard[keyboard.find("pub(crate) unsafe fn poll(").unwrap()..], &["sdk_calc(", "SIGNALS.try_lock()"]);
}

/// Step 5c: the menu worker sleeps on a `hos_lock::Event`, not std's parker. Game hooks wake it (`worker::wake`, 21 call paths from
/// `cpu::on_frame`, the input poll, the AI epilogue): std's parker on the Skyline target is pthread's, whose `unpark` takes the
/// parker's mutex, which the parked worker holds around its condition-variable wait and after every wake, a plugin-held lock no
/// scan of this crate sees. Setting the event is an atomic swap and the arbiter's signal: it never waits. No runtime code parks.
#[test]
fn the_worker_sleeps_on_an_event_that_never_makes_a_waker_wait() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let g = graph();
    let files: BTreeSet<String> = g.funcs.iter().map(|f| f.file.clone()).collect();
    for file in files {
        // The code only: a comment may name the parker it replaced.
        let text: String = std::fs::read_to_string(runtime.join(&file))
            .unwrap_or_default()
            .lines()
            .map(|line| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        for parker in ["thread::park", ".unpark()", "park_timeout("] {
            assert!(!text.contains(parker), "{file} uses std's parker ({parker})");
        }
    }
    let worker = include_str!("../src/runtime/ui/worker.rs");
    ordered(body(worker, "wake"), &["WAKE.set()"]);
    ordered(body(worker, "run"), &["WAKE.wait(Some(POLL))", "WAKE.wait(None)"]);
    assert!(worker.contains("static WAKE: hos_lock::Event"), "the worker's wake is not the event");
}
