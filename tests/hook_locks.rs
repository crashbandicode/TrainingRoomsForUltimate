//! stream/sync (operator order 2026-11-02): a game thread never takes a lock that another thread can hold across real work.
//! The locks in `HOOK_LOCKS` are taken by game-thread hooks. This test follows the runtime's call graph (`tests/common/callgraph.rs`)
//! from every entry point that runs on a plugin thread (the menu worker, the writer threads, the loaders, the publishing path)
//! and fails if any function it reaches takes one of them. A plugin thread hands values to the game through
//! `hos_lock::Published`, atomics or a lock-free mailbox instead (IMPLEMENTATION-PROGRESS-sync.md (removed after v0.2.0-rc.1; see that tag), audit table).
//!
//! How the scan reads the code:
//! - **Lock sites:** a function takes a lock when its body has the static followed by `.lock(` / `.read(` / `.write(` /
//!   `.try_lock(` / `.try_read(` / `.try_write(`, or hands it by reference to a call (`lock_write(&MOTION)`, the carried code's
//!   helpers). The static is matched in its own file, or by `module::NAME` elsewhere.
//! - **Edges:** path calls (`module::f(..)`, `f(..)`, a function handed over by path) as the capability scan resolves them. A
//!   method call `.m(..)` is followed when exactly one runtime method with a `self` receiver is named `m`; when several are, the edge is not followed
//!   and the name is listed in the report (`AMBIGUOUS`), so nothing is dropped without a trace. Methods of model types and of the
//!   locks are not runtime functions and have no edge.
//! - **Limits:** calls through `dyn` objects, trait methods, closures stored and called later, and function pointers kept in data
//!   are not seen (the capability scan's `dynamic` edges are the place to add one); a closure body counts as part of the function
//!   that contains it; a static reached through a re-export is matched only through `ALIASES`. `diag`-only modules are not scanned
//!   (they never ship).
//!
//! `the_scan_catches_a_seeded_violation` runs the same scan on `tests/fixtures/hook_locks/`, a small runtime with one plugin thread
//! that takes a hook's lock through a method call and through a helper, and requires both to be reported.

use crate::callgraph;
use crate::tech_callbacks;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use callgraph::{Graph, Tok};

/// `(file under src/runtime, static)`: locks game-thread hooks take.
const HOOK_LOCKS: &[(&str, &str)] = &[
    ("features/chords.rs", "STATE"),
    ("features/cpu.rs", "CPU"),
    ("features/states.rs", "STATE"),
    ("features/states/quick_reset.rs", "QUICK"),
    ("features/states/preview.rs", "CHECKPOINT"),
    ("features/states/setups.rs", "FACTS"),
    ("features/recorder.rs", "RNG"),
    ("features/input_display.rs", "TRACKER"),
    ("features/input_display.rs", "BACK"),
    ("features/input_display.rs", "IDS"),
    ("features/input_display.rs", "RENDERER"),
    ("features/combo.rs", "HELPER"),
    ("features/combo.rs", "NAME"),
    ("features/combo.rs", "PLAN"),
    ("features/combo.rs", "RUN"),
    ("features/combo.rs", "RENDERER"),
    ("features/input_delay.rs", "ADAPTER"),
    ("features/notices.rs", "QUEUE"),
    ("features/notices.rs", "BOOT_NOTICES"),
    ("features/hud_draw.rs", "CONTAINERS"),
    // The hitbox visualizer's own orbs: the fighter pass publishes on BOARD (game thread, under STORE), the layout tick
    // reads it under ORB_PANES (review HB-5, 2026-11-05).
    ("features/hitbox.rs", "STORE"),
    ("features/hitbox.rs", "BOARD"),
    ("features/hitbox.rs", "ORB_PANES"),
    // rc.14 Eggs-ercise: the run is stepped by the fighter pass and reset by the on-init wrapper (game thread); the layout
    // tick reads its egg. The menu worker reaches it only through atomics (`eggs_start`, `publish`).
    ("features/hitbox.rs", "RUN"),
    // rc.14 Break the Targets: P1's poll (the input thread), P1's pass (the game thread) and the layout tick take it; the menu worker
    // reaches it only through atomics and a mailbox.
    ("features/btt.rs", "BTT"),
    // Mash's lifecycle belongs only to fighter callbacks; its solver worker must never take it.
    ("features/mashout.rs", "GAME"),
    ("features/tech.rs", "VIEW"),
    ("compat/menu.rs", "EDITED"),
    ("ui/input.rs", "ADAPTER"),
    ("ui/menu_app.rs", "QUICK_MENU_APP"),
    ("macros/named_macros.rs", "PANEL_STOPS"),
    ("macros/replay_capture.rs", "VAULT_HOOK"),
    ("macros/replay_capture.rs", "CLOSE_INPUT"),
    ("macros/live_replay.rs", "CAPTURE"),
];

use crate::model::thread_stacks;

/// Thread entries that exist only under a non-shipped probe feature, in a block the call-graph scan does not resolve: skipped while the
/// graph has no such function (the scan still follows them when it does). Every other missing entry fails the test.
const FEATURE_ONLY_ENTRIES: &[(&str, &str)] = &[("practice_trace::writer", "console-practice-trace (diag probe feature) only, inside a cfg-gated block")];

/// Entry points on the publishing path that a plugin thread calls (not threads of their own).
const PUBLISHING_ENTRIES: &[&str] = &["published::publish", "menu::apply_kept", "mashout::service"];

/// `module::function` of every plugin thread's entry, derived from `model::thread_stacks::THREADS` (each row's `symbol`, its closure
/// suffix dropped), so a new or renamed thread cannot be missed; the diag-only threads are identified by their stable IDs and live in modules the
/// scan does not read. Plus the publishing path.
fn plugin_thread_entries() -> Vec<String> {
    let diag_only = diag_only_modules(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"));
    let mut out: Vec<String> = PUBLISHING_ENTRIES.iter().map(|s| (*s).to_owned()).collect();
    for row in thread_stacks::THREADS.iter() {
        let path = row.symbol.split("::{").next().unwrap_or(row.symbol);
        if matches!(row.id, thread_stacks::ThreadId::StateDiag | thread_stacks::ThreadId::DiagReload | thread_stacks::ThreadId::MacroDiag)
            || path.split("::").any(|m| diag_only.contains(m))
        {
            continue;
        }
        let path = row.symbol.split("::{").next().unwrap_or(row.symbol);
        let segments: Vec<&str> = path.split("::").collect();
        let key = segments[segments.len().saturating_sub(2)..].join("::");
        if !out.contains(&key) {
            out.push(key);
        }
    }
    out
}
/// `(function key, lock)` reached from a plugin thread on purpose, with the reason it is not a hook waiting on real work.
const ALLOWED: &[(&str, &str, &str)] = &[
    ("setups::facts", "FACTS", "copies only the published value; no native call, allocation or STATE access under this lock"),
];

/// A ratchet, not a tolerance (lead, 2026-11-02): violations whose fix is in progress elsewhere, each (function key, file::lock, owner,
/// the fix that removes it). The test fails on any violation not listed here, AND on a row that no longer occurs (delete it the moment the
/// fix lands), and prints the table every run. The release-candidate gate requires this table to be empty.
const PENDING: &[(&str, &str, &str, &str)] = &[];

const LOCK_METHODS: &[&str] = &["lock", "read", "write", "try_lock", "try_read", "try_write"];

/// `(file, module name)`: a re-export a static is also reached by (`compat::common::menu` re-exports `ui::menu_app`'s statics).
const ALIASES: &[(&str, &str)] = &[("ui/menu_app.rs", "menu"), ("ui/menu_app.rs", "menu_app")];

/// Whether function `index` takes the lock `name` of `file` (see the module comment).
fn takes_lock(graph: &Graph, index: usize, file: &str, name: &str) -> bool {
    takes_lock_methods(graph, index, file, name, LOCK_METHODS, true)
}

fn takes_lock_methods(graph: &Graph, index: usize, file: &str, name: &str, methods: &[&str], by_reference: bool) -> bool {
    takes_lock_on_thread(graph, index, file, name, methods, by_reference, None)
}

fn takes_lock_on_thread(graph: &Graph, index: usize, file: &str, name: &str, methods: &[&str], by_reference: bool, plugin: Option<bool>) -> bool {
    let f = &graph.funcs[index];
    let module = Path::new(file).file_stem().unwrap().to_string_lossy().into_owned();
    let toks = &f.body;
    let ident = |k: usize, s: &str| matches!(toks.get(k), Some(Tok::Ident(t)) if t == s);
    let punct = |k: usize, c: char| matches!(toks.get(k), Some(Tok::Punct(p)) if *p == c);
    let excluded = plugin.and_then(|plugin| callgraph::thread_dispatch(toks).map(|(p, g)| if plugin { g } else { p }));
    for i in 0..toks.len() {
        if excluded.as_ref().is_some_and(|range| range.contains(&i)) { continue; }
        if !ident(i, name) {
            continue;
        }
        let qualified = i >= 2
            && toks[i - 1] == Tok::PathSep
            && (ident(i - 2, &module) || ALIASES.iter().any(|(f, m)| *f == file && ident(i - 2, m)));
        if f.file != file && !qualified {
            continue;
        }
        // `NAME.lock()`, or one lock of a static array, `NAME[role].lock()`.
        let mut dot = i + 1;
        if punct(dot, '[') {
            let mut depth = 0;
            while dot < toks.len() {
                if punct(dot, '[') {
                    depth += 1;
                } else if punct(dot, ']') {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                dot += 1;
            }
            dot += 1;
        }
        let method = punct(dot, '.') && methods.iter().any(|m| ident(dot + 1, m)) && punct(dot + 2, '(');
        let start = if qualified { i - 2 } else { i };
        let by_ref = by_reference && start >= 1 && punct(start - 1, '&');
        if method || by_ref {
            return true;
        }
    }
    false
}

/// The carried code's typed statics: a helper handed `&STATIC` (`read(&MENU)`, `setting(&MENU)`, `lock_write(&MENU)`,
/// `assign(&MENU, ..)`) calls the static's trait impl, which the text cannot show (a trait method). Each row is `(static, helper,
/// the impl function)`: `read`/`setting` and `lock_write` run `Menu::read_value` (the guard is built from it), the guard's drop and
/// `assign_value` run `store`. A function that branches on `on_plugin_thread()` (`callgraph::thread_dispatch`) is followed into its
/// plugin-thread branch only (this scan walks plugin threads). rc.6 QP freeze (`rc6-qpf3`, 2026-11-09): without these edges the
/// scan did not see the menu worker (`macros::port::worker_tick` -> `named_macros` -> `read(&MENU)` -> `current_menu`) take
/// `EDITED`; the game's MainThread starved the worker on core 0 while it held it, and the layout task on another core waited in
/// its hook for `EDITED`.
const TYPED_STATIC_CALLS: &[(&str, &str, &str)] = &[
    ("MENU", "read", "menu::read_value"),
    ("MENU", "setting", "menu::read_value"),
    ("MENU", "lock_write", "menu::read_value"),
    ("MENU", "lock_write", "menu::store"),
    ("MENU", "assign", "menu::store"),
];

/// The names `index` calls only inside the game-thread branch of an `on_plugin_thread()` dispatch: not followed on a plugin thread.
fn other_thread_branch_only(graph: &Graph, index: usize, plugin: bool) -> BTreeSet<String> {
    let body = &graph.funcs[index].body;
    let Some((p, g)) = callgraph::thread_dispatch(body) else { return BTreeSet::new() };
    let game = if plugin { g } else { p };
    let called = |k: usize| match (body.get(k), body.get(k + 1)) {
        (Some(Tok::Ident(name)), Some(Tok::Punct('('))) => Some(name.clone()),
        _ => None,
    };
    let inside: BTreeSet<String> = game.clone().filter_map(called).collect();
    let outside: BTreeSet<String> = (0..body.len()).filter(|k| !game.contains(k)).filter_map(called).collect();
    inside.difference(&outside).cloned().collect()
}

/// `TYPED_STATIC_CALLS` is what `compat/menu.rs` does: `Menu::read_value` sends a plugin thread to `plugin_view` and only a game
/// thread to `current_menu`; `plugin_view` never names `EDITED`, and `current_menu` is the one reader that does.
#[test]
fn menu_read_dispatches_on_the_thread() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let graph = Graph::scan(&runtime, &[]);
    let body = |key: &str| -> String {
        let found: Vec<usize> = graph.find(key).into_iter().filter(|&i| graph.funcs[i].file == "compat/menu.rs").collect();
        assert_eq!(found.len(), 1, "{key} in compat/menu.rs: {} functions", found.len());
        graph.funcs[found[0]].body.iter().map(|t| format!("{t:?}")).collect::<Vec<_>>().join(" ")
    };
    let read = body("menu::read_value");
    let (plugin, game) = (read.find("plugin_view").expect("read_value calls plugin_view"), read.find("current_menu").expect("read_value calls current_menu"));
    assert!(read.contains("on_plugin_thread") && read.find("on_plugin_thread").unwrap() < plugin && plugin < game,
            "read_value must take plugin threads to plugin_view and the rest to current_menu: {read}");
    assert!(!body("menu::plugin_view").contains("EDITED"), "plugin_view names EDITED");
    assert!(body("menu::current_menu").contains("EDITED"), "current_menu no longer takes EDITED: update TYPED_STATIC_CALLS");
    let runtime_files = std::fs::read_to_string(runtime.join("threads.rs")).unwrap();
    assert!(runtime_files.contains("pub fn on_plugin_thread() -> bool"), "threads::on_plugin_thread is gone");
}

/// The impl functions `from`'s body reaches through `helper(&STATIC ..)` / `helper(&path::STATIC ..)` (TYPED_STATIC_CALLS).
fn typed_static_callees(graph: &Graph, from: usize) -> BTreeSet<usize> {
    let toks = &graph.funcs[from].body;
    let ident = |k: usize| match toks.get(k) {
        Some(Tok::Ident(t)) => Some(t.as_str()),
        _ => None,
    };
    let punct = |k: usize, c: char| matches!(toks.get(k), Some(Tok::Punct(p)) if *p == c);
    let mut out = BTreeSet::new();
    for i in 0..toks.len() {
        let Some(helper) = ident(i) else { continue };
        if !TYPED_STATIC_CALLS.iter().any(|(_, h, _)| *h == helper) || !punct(i + 1, '(') || !punct(i + 2, '&') {
            continue;
        }
        // `&MENU` or `&a::b::MENU`: the last segment of the path after `&`.
        let mut k = i + 3;
        while ident(k).is_some() && toks.get(k + 1) == Some(&Tok::PathSep) {
            k += 2;
        }
        let Some(name) = ident(k) else { continue };
        for (static_name, h, callee) in TYPED_STATIC_CALLS {
            if *static_name == name && *h == helper {
                out.extend(graph.find(callee));
            }
        }
    }
    out
}

/// Callees of `index`: path calls, plus method calls whose name is unique among scanned receiver methods. Ambiguous method names
/// are recorded in `ambiguous` (name -> how many functions carry it).
fn edges(graph: &Graph, index: usize, by_name: &BTreeMap<String, Vec<usize>>, ambiguous: &mut BTreeMap<String, usize>) -> BTreeSet<usize> {
    edges_on_thread(graph, index, by_name, ambiguous, true)
}

fn edges_on_thread(graph: &Graph, index: usize, by_name: &BTreeMap<String, Vec<usize>>, ambiguous: &mut BTreeMap<String, usize>, plugin: bool) -> BTreeSet<usize> {
    let from = &graph.funcs[index];
    let refs = Graph::references(from);
    // A path edge keeps the module qualifier: `p1::publish(..)` reaches the `publish` of module `p1` only (the capability resolver's rule;
    // `self`/`Self`/`super`/`crate` mean this file; a qualifier that names no candidate's module, an alias, keeps every candidate).
    let path_match = |c: usize| {
        let callee = &graph.funcs[c];
        refs.iter().filter(|(_, method)| !method).any(|(segments, _)| {
            if segments.last() != Some(&callee.name) {
                return false;
            }
            let Some(qualifier) = segments.len().checked_sub(2).map(|k| segments[k].as_str()) else {
                // An unqualified call means this file's function when there is one (Rust's own rule: the local item shadows an
                // import); only when there is none can it be an imported one.
                let local = by_name.get(&callee.name).is_some_and(|all| all.iter().any(|&o| graph.funcs[o].file == from.file));
                return !local || callee.file == from.file;
            };
            match qualifier {
                "self" | "Self" | "super" | "crate" => callee.file == from.file,
                // A `#[path]` module declared under another name (`body_bank` is `native_body_bank.rs`).
                q if graph.declared_module(&from.file, q).is_some() => graph.declared_module(&from.file, q) == Some(callee.module.as_str()),
                q if q == callee.module => true,
                q => !by_name.get(&callee.name).is_some_and(|all| all.iter().any(|&o| graph.funcs[o].module == q)),
            }
        })
    };
    let mut out: BTreeSet<usize> = graph.callees(index).into_iter().filter(|&c| path_match(c)).collect();
    out.extend(typed_static_callees(graph, index).into_iter().filter(|&c| c != index));
    for (segments, method) in &refs {
        if !method {
            continue;
        }
        let Some(name) = segments.last() else { continue };
        if LOCK_METHODS.contains(&name.as_str()) {
            continue;
        }
        let methods: Vec<usize> = by_name.get(name).into_iter().flatten().copied()
            .filter(|&candidate| graph.funcs[candidate].has_receiver).collect();
        match methods.as_slice() {
            [only] if *only != index => {
                out.insert(*only);
            }
            many if many.len() > 1 => {
                ambiguous.insert(name.clone(), many.len());
            }
            _ => {}
        }
    }
    let game_only = other_thread_branch_only(graph, index, plugin);
    out.retain(|&c| !game_only.contains(&graph.funcs[c].name));
    out
}

struct Scan {
    violations: BTreeSet<String>,
    ambiguous: BTreeMap<String, usize>,
}

fn scan(
    runtime: &Path,
    entries: &[&str],
    locks: &[(&str, &str)],
    allowed: &[(&str, &str, &str)],
) -> Scan {
    let graph = Graph::scan(runtime, &[]);
    scan_graph(&graph, entries, locks, allowed)
}

fn scan_graph(
    graph: &Graph,
    entries: &[&str],
    locks: &[(&str, &str)],
    allowed: &[(&str, &str, &str)],
) -> Scan {
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, f) in graph.funcs.iter().enumerate() {
        by_name.entry(f.name.clone()).or_default().push(i);
    }
    let mut violations = BTreeSet::new();
    let mut ambiguous = BTreeMap::new();
    for entry in entries {
        let starts = graph.find(entry);
        if starts.is_empty() && FEATURE_ONLY_ENTRIES.iter().any(|(e, _)| e == entry) {
            continue;
        }
        assert!(!starts.is_empty(), "entry {entry} not found");
        let mut seen = BTreeSet::new();
        let mut stack: Vec<(usize, String)> = starts
            .into_iter()
            .map(|s| (s, (*entry).to_owned()))
            .collect();
        while let Some((i, via)) = stack.pop() {
            if !seen.insert(i) {
                continue;
            }
            let key = graph.funcs[i].key();
            for (file, lock) in locks {
                if takes_lock(graph, i, file, lock)
                    && !allowed.iter().any(|(f, l, _)| *f == key && l == lock)
                {
                    violations.insert(format!(
                        "{entry} -> ... -> {key} takes {file}::{lock} (called from {via})"
                    ));
                }
            }
            for callee in edges(graph, i, &by_name, &mut ambiguous) {
                stack.push((callee, key.clone()));
            }
        }
    }
    Scan {
        violations,
        ambiguous,
    }
}

fn hud_borrow_contract(hud: &[Tok], seq: &[Tok]) -> bool {
    let at = |body: &[Tok], pattern: &str| {
        let pattern = callgraph::lex(pattern);
        body.windows(pattern.len()).position(|w| w == pattern)
    };
    let Some(start) = at(hud, "HELPER.try_lock() else { return 0") else {
        return false;
    };
    let Some(draw) = at(hud, "panes.draw(&helper") else {
        return false;
    };
    let Some(end) = at(hud, "drop(helper)") else {
        return false;
    };
    let Some(apply) = at(hud, "apply(") else {
        return false;
    };
    start < draw && draw < end && end < apply
        && at(hud, "RENDERER.try_lock() else { return 0").is_some()
        && at(hud, "NAME.try_lock()").is_some()
        && at(seq, "PLAN.try_lock()?").is_some() && at(seq, "RUN.try_lock()?").is_some()
        && hud[start..end].iter().chain(seq).all(|t| !matches!(t, Tok::Ident(s) if ["lock", "read", "write", "Box", "Vec", "String", "alloc", "after_strip", "session_clear", "set_target"].contains(&s.as_str())))
}

#[test]
fn combo_hud_has_no_wait_or_native_apply_under_helper_and_seeds_fail() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut graph = Graph::scan(&runtime, &[]);
    let hud = graph.find("combo::hud_tick")[0];
    let seq = graph.find("combo::seq_view")[0];
    assert!(hud_borrow_contract(
        &graph.funcs[hud].body,
        &graph.funcs[seq].body
    ));
    let mut bad_hud = graph.funcs[hud].body.clone();
    let at = bad_hud
        .windows(3)
        .position(|w| w == callgraph::lex("RENDERER.try_lock"))
        .unwrap()
        + 2;
    bad_hud[at] = Tok::Ident("lock".into());
    assert!(
        !hud_borrow_contract(&bad_hud, &graph.funcs[seq].body),
        "blocking RENDERER seed escaped"
    );
    for method in ["RUN", "PLAN"] {
        let mut bad = graph.funcs[seq].body.clone();
        let at = bad
            .windows(3)
            .position(|w| w == callgraph::lex(&format!("{method}.try_lock")))
            .unwrap()
            + 2;
        bad[at] = Tok::Ident("lock".into());
        assert!(
            !hud_borrow_contract(&graph.funcs[hud].body, &bad),
            "blocking {method} seed escaped"
        );
    }
    // Exercise the ownership scanner with a plugin entry reaching a newly inventoried successful try.
    graph.funcs[hud].body =
        callgraph::lex("NAME.try_lock(); PLAN.try_lock(); RUN.try_lock(); RENDERER.try_lock();");
    let result = scan_graph(&graph, &["combo::hud_tick"], HOOK_LOCKS, &[]);
    for name in ["NAME", "PLAN", "RUN", "RENDERER"] {
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.contains(&format!("features/combo.rs::{name}"))),
            "try ownership seed {name} missed"
        );
    }
}

/// Camera requests and the game-frame applier stay lock-free even when only game threads reach them.
/// Use the production reach scanner; CAMERA is also named explicitly so the seeded lock is seen after it becomes a Cell.
#[test]
fn btt_camera_callbacks_take_no_lock_and_camera_lock_seeds_fail() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut graph = Graph::scan(&runtime, &[]);
    // A scene read imports the engine's read_unchecked as `read`, not the compat menu's typed read.
    graph.precise_imports = true;
    let entries = ["btt_camera::request", "btt_camera::interrupt_placement", "btt_camera::placing", "btt_camera::placement_pad", "btt_camera::update_placement", "btt_camera::frame"];
    let mut locks = static_locks(&runtime);
    let camera = ("features/btt_camera.rs".to_owned(), "CAMERA".to_owned());
    if !locks.contains(&camera) {
        locks.push(camera);
    }
    let refs: Vec<(&str, &str)> = locks.iter().map(|(file, name)| (file.as_str(), name.as_str())).collect();
    let result = scan_graph(&graph, &entries, &refs, &[]);
    assert!(result.violations.is_empty(), "camera callbacks reach locks: {:?}", result.violations);
    for entry in entries {
        let found = graph.find(entry);
        assert_eq!(found.len(), 1, "camera callback {entry}");
        let at = found[0];
        let original = graph.funcs[at].body.clone();
        graph.funcs[at].body.extend(callgraph::lex("CAMERA.lock();"));
        let seeded = scan_graph(&graph, &[entry], &refs, &[]);
        assert!(seeded.violations.iter().any(|v| v.contains(&format!("{entry} takes features/btt_camera.rs::CAMERA"))),
                "blocking camera seed escaped {entry}: {:?}", seeded.violations);
        graph.funcs[at].body = original;
    }
}

#[test]
fn no_plugin_thread_reaches_a_lock_a_hook_takes() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let entries = plugin_thread_entries();
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    println!("plugin-thread entries: {entries:?}");
    let result = scan(&runtime, &entries, HOOK_LOCKS, ALLOWED);
    println!("AMBIGUOUS method names not followed (name: functions): {:?}", result.ambiguous);
    for (key, lock, owner, fix) in PENDING {
        println!("PENDING {key} takes {lock}: owner {owner}; fix: {fix}");
    }
    let pending_hit = |v: &String| PENDING.iter().any(|(key, lock, _, _)| v.contains(&format!("{key} takes {lock} ")));
    let unexpected: Vec<&String> = result.violations.iter().filter(|v| !pending_hit(v)).collect();
    assert!(unexpected.is_empty(), "plugin threads reach locks hooks take:\n{}", unexpected.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
    let gone: Vec<&str> = PENDING
        .iter()
        .filter(|(key, lock, _, _)| !result.violations.iter().any(|v| v.contains(&format!("{key} takes {lock} "))))
        .map(|(key, _, _, _)| *key)
        .collect();
    assert!(gone.is_empty(), "PENDING rows that no longer occur (delete them): {gone:?}");
}

#[test]
fn the_scan_catches_a_seeded_violation() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hook_locks");
    let result = scan(&fixture, &["worker::run"], &[("features/chords.rs", "STATE"), ("macros/named_macros.rs", "MOTION")], &[]);
    let text = result.violations.iter().cloned().collect::<Vec<_>>().join("\n");
    assert!(text.contains("chords::refresh_rows takes features/chords.rs::STATE"), "method-call edge and lock site missed:\n{text}");
    assert!(text.contains("named_macros::reserve takes macros/named_macros.rs::MOTION"), "helper by reference missed:\n{text}");
    assert!(!text.contains("on_poll"), "a hook-only function was reported:\n{text}");
    assert!(!text.contains("named_macros::store"), "a local call or an atomic method resolved to another module's function:\n{text}");
    assert!(!text.contains("free_only"), "a model method resolved to a same-named free function:\n{text}");
    assert_eq!(result.ambiguous.get("tick"), Some(&2), "ambiguous names must be listed: {:?}", result.ambiguous);
}

#[test]
fn a_typed_static_read_reaches_its_impl() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hook_locks");
    let result = scan(&fixture, &["worker::reads_menu"], &[("compat/menu.rs", "EDITED")], &[]);
    assert!(result.violations.iter().any(|line| line.contains("menu::plugin_view takes compat/menu.rs::EDITED")),
            "read(&MENU) not followed to the impl: {:?}", result.violations);
}

/// Unlike HOOK_LOCKS (plugin/hook ownership), the native throw observer may take no lock at all, including a try.
/// Reuse the runtime graph and its edge resolver; keep unresolved native/model/crate references visible in the report.
struct ObserverScan {
    reached: BTreeSet<usize>,
    violations: BTreeSet<String>,
    ambiguous: BTreeMap<String, usize>,
    boundaries: BTreeSet<String>,
}

/// The scanner has no receiver types: an atomic's `.load()` otherwise reaches the UI's `Layout::load`.
/// Mask only methods on these concrete, source-checked external statics, keeping calls on other receivers visible.
fn throw_observer_graph(runtime: &Path) -> Graph {
    let mut graph = Graph::scan(runtime, &[]);
    graph.precise_imports = true;
    let receivers = [
        ("features/mashout.rs", "READY", "AtomicBool"),
        ("features/mashout.rs", "SLOTS_READY", "AtomicBool"),
        ("features/mashout.rs", "EPOCH", "AtomicU64"),
        ("features/mashout/interposer.rs", "INSTALLED", "AtomicBool"),
        ("features/mashout/interposer.rs", "ORIGINAL_END", "AtomicUsize"),
        ("features/mashout.rs", "P1", "AtomicUsize"),
        ("features/mashout.rs", "ARMED", "AtomicU64"),
        ("features/mashout.rs", "THRESHOLD", "AtomicU32"),
        ("game.rs", "BASE", "AtomicUsize"),
        ("game.rs", "GRANTED", "OnceLock<Granted>"),
        ("game.rs", "TRAINING_PREDICATE_USABLE", "AtomicBool"),
        ("game.rs", "ENTRY_NATIVES_USABLE", "AtomicBool"),
        ("game.rs", "ACTIVE_CELL_GUARDED", "std::sync::atomic::AtomicBool"),
        ("lobby/game_mode.rs", "BASE", "AtomicUsize"),
        ("scene.rs", "LOBBY_ACTIVE", "AtomicBool"),
        ("scene.rs", "HOT_SWAP_IN_PROGRESS", "AtomicBool"),
    ];
    for (file, receiver, ty) in receivers {
        let declaration = if receiver == "HOT_SWAP_IN_PROGRESS" { "published.rs" } else { file };
        let source = std::fs::read_to_string(runtime.join(declaration)).unwrap();
        assert!(source.contains(&format!("static {receiver}: {ty}")), "external receiver changed: {file}::{receiver}");
        for f in graph.funcs.iter_mut().filter(|f| f.file == file) {
            for at in 2..f.body.len() {
                if f.body[at - 2] == Tok::Ident(receiver.into()) && f.body[at - 1] == Tok::Punct('.') {
                    if let Tok::Ident(method) = &mut f.body[at] {
                        if (ty.starts_with("Atomic") || ty.ends_with("AtomicBool")) && matches!(method.as_str(), "load" | "store" | "fetch_max")
                            || ty == "OnceLock<Granted>" && method == "get"
                        {
                            *method = format!("external_{receiver}_{method}");
                        }
                    }
                }
            }
        }
    }
    // Admission::with is a model method, not diag_log::Probe::with. Mask that exact receiver expression only;
    // the closure body remains in the graph. Any different `.with()` still goes through normal resolution.
    let pattern = callgraph::lex("admission().with(");
    for f in graph.funcs.iter_mut().filter(|f| f.file == "features/mashout.rs") {
        for at in 0..f.body.len().saturating_sub(pattern.len()) + 1 {
            if f.body[at..].starts_with(&pattern) {
                f.body[at + 4] = Tok::Ident("external_Admission_with".into());
            }
        }
    }
    graph
}

fn scan_throw_observer(graph: &Graph, entry: &str, locks: &[(&str, &str)]) -> ObserverScan {
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, f) in graph.funcs.iter().enumerate() {
        by_name.entry(f.name.clone()).or_default().push(i);
    }
    let mut result = ObserverScan {
        reached: BTreeSet::new(),
        violations: BTreeSet::new(),
        ambiguous: BTreeMap::new(),
        boundaries: BTreeSet::new(),
    };
    let mut pending = graph.find(entry);
    assert_eq!(pending.len(), 1, "unique observer entry {entry} must exist");
    while let Some(i) = pending.pop() {
        if !result.reached.insert(i) {
            continue;
        }
        let f = &graph.funcs[i];
        let key = f.key();
        for (file, lock) in locks {
            if takes_lock(graph, i, file, lock) {
                result.violations.insert(format!("{entry} -> {key} takes {file}::{lock}"));
            }
        }
        for (at, token) in f.body.iter().enumerate() {
            let Tok::Ident(name) = token else { continue };
            let name = name.as_str();
            // All lock/try methods are rejected even on an uninventoried receiver. A bare read/write can be
            // a raw-pointer load or Published read; RwLock read/write are checked by the full static inventory above.
            if matches!(name, "lock" | "try_lock" | "try_read" | "try_write")
                && f.body.get(at + 1) == Some(&Tok::Punct('('))
            {
                result.violations.insert(format!("{entry} -> {key} lock operation {name}"));
            }
        }
        // This is a source-level admission ratchet, not an allocator proof for external/native code. Catch the
        // usual heap-owning constructors, macros, conversions and growth operations wherever the graph reaches.
        for token in &f.body {
            if let Tok::Ident(name) = token {
                if ["Box", "Vec", "String", "VecDeque", "HashMap", "HashSet", "BTreeMap", "BTreeSet", "Rc", "Arc", "CString",
                    "alloc", "alloc_zeroed", "realloc", "__rust_alloc", "malloc", "calloc", "vec", "format",
                    "to_owned", "to_string", "to_vec", "into_boxed_slice", "collect", "reserve", "reserve_exact", "push", "push_str"]
                    .contains(&name.as_str())
                {
                    result.violations.insert(format!("{entry} -> {key} allocation operation {name}"));
                }
            }
        }
        let mut next = edges(graph, i, &by_name, &mut result.ambiguous);
        // edges() is normally used for plugin threads and prunes the game branch. This observer is native: follow
        // both branches, and explicit dynamic edges, should either be introduced into its reach later.
        if callgraph::thread_dispatch(&f.body).is_some() {
            next.extend(graph.callees(i));
        }
        for (from, to) in &graph.dynamic {
            if *from == key {
                next.extend(graph.find(to));
            }
        }
        for (segments, method) in Graph::references(f) {
            let name = segments.last().unwrap();
            if !next.iter().any(|&callee| graph.funcs[callee].name == *name) {
                result.boundaries.insert(format!("{key}: {}{}", if method { "." } else { "" }, segments.join("::")));
            }
        }
        pending.extend(next);
    }
    result
}

fn throw_observer_entries(graph: &Graph) -> Vec<String> {
    let mut entries = vec!["mashout::throw_seed_after".to_owned()];
    for name in ["started", "end"] {
        let wrappers: Vec<_> = graph.funcs.iter().filter(|f|
            f.file == "features/mashout/interposer.rs" && f.name == name).collect();
        assert_eq!(wrappers.len(), 1, "the native slot bridge must exist exactly once");
        entries.push(wrappers[0].key());
    }
    entries
}

#[test]
fn native_throw_observer_reach_has_no_lock_or_allocation() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let graph = throw_observer_graph(&runtime);
    let locks = static_locks(&runtime);
    let locks: Vec<_> = locks.iter().map(|(f, n)| (f.as_str(), n.as_str())).collect();
    let observer = graph.find("mashout::throw_seed_after");
    assert_eq!(observer.len(), 1);
    for entry in throw_observer_entries(&graph) {
        let result = scan_throw_observer(&graph, &entry, &locks);
        let reached: Vec<_> = result.reached.iter().map(|&i| format!("{} ({})", graph.funcs[i].key(), graph.funcs[i].file)).collect();
        println!("THROW-OBSERVER {entry}: {reached:?}");
        println!("THROW-BOUNDARIES unresolved native/model/crate references: {:?}", result.boundaries);
        println!("THROW-LIMITS: external native functions, model/crate method bodies, dyn/trait calls and stored function pointers/closures require separate review; supply known runtime pointer edges in Graph::dynamic");
        assert!(result.ambiguous.is_empty(), "observer method edges need explicit resolution: {:?}", result.ambiguous);
        if !entry.ends_with("::end") {
            assert!(result.reached.contains(&observer[0]), "{entry} must reach the actual observer");
        }
        assert!(result.reached.iter().any(|&i| graph.funcs[i].file == "features/mashout/reader.rs" && graph.funcs[i].name == "sample"),
            "{entry} must reach the real Control reader");
        assert!(result.violations.is_empty(), "native throw observer reaches lock/allocation operations:\n{}", result.violations.into_iter().collect::<Vec<_>>().join("\n"));
    }
}

/// M3 Set observer (O9/C12): the qualified native path (`clatter_set::qualified`, entered from the naked +0x90
/// wrapper) reaches no lock and no allocation. Only atomic/volatile method names inside clatter_set.rs are masked
/// (the scanner has no receiver types); every other call stays visible.
#[test]
fn native_clatter_set_observer_reach_has_no_lock_or_allocation() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut graph = throw_observer_graph(&runtime);
    for f in graph.funcs.iter_mut().filter(|f| f.file == "features/mashout/clatter_set.rs") {
        for at in 1..f.body.len() {
            if f.body[at - 1] == Tok::Punct('.') {
                if let Tok::Ident(method) = &mut f.body[at] {
                    if matches!(method.as_str(), "load" | "store" | "swap" | "fetch_add" | "compare_exchange" | "read_volatile") {
                        *method = format!("external_atomic_{method}");
                    }
                }
            }
        }
    }
    let locks = static_locks(&runtime);
    let locks: Vec<_> = locks.iter().map(|(f, n)| (f.as_str(), n.as_str())).collect();
    for name in ["qualified", "qualified_add"] {
        let entry = graph.funcs.iter().find(|f| f.file == "features/mashout/clatter_set.rs" && f.name == name)
            .expect("each qualified path (Set, Add) must exist").key();
        let result = scan_throw_observer(&graph, &entry, &locks);
        let reached: Vec<_> = result.reached.iter().map(|&i| graph.funcs[i].key()).collect();
        println!("CLATTER-SET-OBSERVER {entry}: {reached:?}; boundaries {:?}", result.boundaries);
        assert!(result.ambiguous.is_empty(), "observer method edges need explicit resolution: {:?}", result.ambiguous);
        assert!(result.violations.is_empty(), "M3 observer reaches lock/allocation operations:\n{}",
            result.violations.into_iter().collect::<Vec<_>>().join("\n"));
    }
}

/// Every function a plugin-thread entry reaches (the same edges as the lock scan).
fn plugin_reach(graph: &Graph, entries: &[&str]) -> BTreeSet<usize> {
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, f) in graph.funcs.iter().enumerate() {
        by_name.entry(f.name.clone()).or_default().push(i);
    }
    let mut ambiguous = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut stack: Vec<usize> = entries.iter().flat_map(|e| graph.find(e)).collect();
    while let Some(i) = stack.pop() {
        if seen.insert(i) {
            stack.extend(edges(graph, i, &by_name, &mut ambiguous));
        }
    }
    seen
}

/// M3 Set observer O2/O3 (review B1): the Set slot's writers (`clatter_set::{service, scene, arm}`) run only on the game
/// thread — reached from the owner sites `cpu::on_frame` (and `lobby::update`), never from any plugin-thread entry
/// (diag_log's drain tick runs `mashout::service`).
#[test]
fn clatter_set_writers_run_only_at_game_thread_owner_sites() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let graph = Graph::scan(&runtime, &[]);
    let writers: Vec<usize> = ["service", "scene", "arm"].iter().map(|name| {
        let found: Vec<usize> = graph.funcs.iter().enumerate()
            .filter(|(_, f)| f.file == "features/mashout/clatter_set.rs" && f.name == *name).map(|(i, _)| i).collect();
        assert_eq!(found.len(), 1, "clatter_set::{name} must exist exactly once");
        found[0]
    }).collect();
    let entries = plugin_thread_entries();
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    let plugin = plugin_reach(&graph, &entries);
    let leaked: Vec<String> = writers.iter().filter(|w| plugin.contains(w)).map(|&w| graph.funcs[w].key()).collect();
    assert!(leaked.is_empty(), "plugin threads reach the Set slot writers: {leaked:?}");
    let game = plugin_reach(&graph, &["cpu::on_frame"]);
    let missing: Vec<String> = writers.iter().filter(|w| !game.contains(w)).map(|&w| graph.funcs[w].key()).collect();
    assert!(missing.is_empty(), "cpu::on_frame must reach every Set slot writer: {missing:?}");
    // O4 (review (c)): every attempt exit closes the gate. Each exit path runs one of these, and each releases.
    for (file, name) in [("features/mashout.rs", "revoke"), ("features/mashout.rs", "reset"), ("features/mashout.rs", "step"),
                         ("features/mashout/clatter_set.rs", "retract_common")] {
        let f = graph.funcs.iter().find(|f| f.file == file && f.name == name).unwrap_or_else(|| panic!("{file}::{name}"));
        assert!(f.body.windows(3).any(|w| w[0] == Tok::Ident("release".into()) && w[1] == Tok::Punct('(')
            || w[1] == Tok::Ident("release".into()) && w[2] == Tok::Punct('(')), "{name} must call clatter_set::release");
    }
    // L2b (2): restore()'s only callers are service() (the owner site) and release(), where it sits behind the
    // owner-thread guard (`owner == current_thread()` before the call). Removing the guard fails this.
    // clatter_set::restore is private, so only its own file can call it; other modules' `restore`s are unrelated.
    let source = std::fs::read_to_string(runtime.join("features/mashout/clatter_set.rs")).unwrap();
    assert!(source.contains("\nfn restore()"), "clatter_set::restore must stay private");
    let callers: Vec<String> = graph.funcs.iter().filter(|f| f.file == "features/mashout/clatter_set.rs" && f.name != "restore"
        && f.body.windows(2).any(|w| w[0] == Tok::Ident("restore".into()) && w[1] == Tok::Punct('(')))
        .map(|f| format!("{}::{}", f.file, f.name)).collect();
    assert_eq!(callers, ["features/mashout/clatter_set.rs::release", "features/mashout/clatter_set.rs::service"],
        "restore must be reached only from release (guarded) and service");
    let release = graph.funcs.iter().find(|f| f.file == "features/mashout/clatter_set.rs" && f.name == "release").unwrap();
    let at = |name: &str| release.body.iter().position(|t| *t == Tok::Ident(name.into()));
    assert!(matches!((at("current_thread"), at("restore")), (Some(g), Some(r)) if g < r),
        "release must check the owner thread before restoring");
    let fail = graph.funcs.iter().find(|f| f.file == "features/mashout.rs" && f.name == "fail").unwrap();
    assert!(fail.body.contains(&Tok::Ident("revoke".into())), "Game::fail must revoke (and so release)");
    let lobby = plugin_reach(&graph, &["lobby::update"]);
    for name in ["service", "scene"] {
        assert!(lobby.iter().any(|&i| graph.funcs[i].file == "features/mashout/clatter_set.rs" && graph.funcs[i].name == name),
            "lobby::update must run clatter_set::{name} (restore/retract outside Training)");
    }
}

#[test]
fn native_throw_observer_scan_catches_transitive_lock_and_allocation_seeds() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut graph = throw_observer_graph(&runtime);
    let helper: Vec<_> = graph.funcs.iter().enumerate()
        .filter(|(_, f)| f.file == "features/mashout/reader.rs" && f.name == "sample").map(|(i, _)| i).collect();
    assert_eq!(helper.len(), 1);
    let helper = helper[0];
    let original = graph.funcs[helper].body.clone();
    let entries = throw_observer_entries(&graph);
    for (seed, expected) in [
        ("UNINVENTORIED.lock();", "lock operation lock"),
        ("UNINVENTORIED.try_lock();", "lock operation try_lock"),
        ("let heap = Vec::with_capacity(1);", "allocation operation Vec"),
        ("let heap = format!(\"seed\");", "allocation operation format"),
    ] {
        graph.funcs[helper].body = original.clone();
        graph.funcs[helper].body.extend(callgraph::lex(seed));
        for entry in &entries {
            let result = scan_throw_observer(&graph, entry, &[]);
            assert!(result.violations.iter().any(|v| v.contains("reader::sample") && v.contains(expected)),
                "transitive {seed} seed escaped {entry}: {:?}", result.violations);
        }
    }
}

#[test]
fn direct_free_function_calls_still_reach_the_hook_lock() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hook_locks");
    let result = scan(&fixture, &["worker::direct"], &[("features/chords.rs", "STATE")], &[]);
    assert!(result.violations.iter().any(|line| line.contains("chords::free_only takes features/chords.rs::STATE")));
}

/// Every `static NAME: …Mutex<…>` / `…RwLock<…>` of the runtime (ours and carried package-6 code, std or hos-lock).
fn static_locks(root: &Path) -> Vec<(String, String)> {
    static_types(root, &["Mutex<", "RwLock<"])
}

fn static_types(root: &Path, types: &[&str]) -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, types: &[&str], out: &mut Vec<(String, String)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, root, types, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                for line in std::fs::read_to_string(&path).unwrap().lines() {
                    let line = line.trim_start();
                    let rest = line.strip_prefix("pub ").or_else(|| line.strip_prefix("pub(crate) ")).unwrap_or(line);
                    let Some(rest) = rest.strip_prefix("static ") else { continue };
                    let Some((name, ty)) = rest.split_once(':') else { continue };
                    if types.iter().any(|ty_name| ty.contains(ty_name)) {
                        out.push((rel.clone(), name.trim().to_owned()));
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, types, &mut out);
    out
}

/// The staged Tech pass is a hook root before the launch binding exists. Walk
/// its helpers too: a hidden blocking initializer is as unsafe as VIEW.lock().
fn non_switch_only(f: &callgraph::Func, runtime: &Path) -> bool {
    // The shared lexer removes string contents from attributes. Require both
    // its exact negative target predicate and the original attached predicate;
    // unsupported cfg syntax remains audited rather than silently skipped.
    let attrs: String = f.attrs.split_whitespace().collect();
    if !attrs.contains("cfg(not(target_os=))") { return false; }
    let source = std::fs::read_to_string(runtime.join(&f.file)).unwrap();
    let compact: String = source.split_whitespace().collect();
    ["", "pub", "pub(crate)", "unsafe", "pubunsafe", "pub(crate)unsafe"].iter().any(|prefix| {
        compact.contains(&format!("#[cfg(not(target_os=\"switch\"))]{prefix}fn{}(", f.name))
    })
}

fn tech_frame_waits(graph: &Graph, runtime: &Path) -> BTreeSet<String> {
    tech_frame_waits_with_callbacks(graph,runtime,tech_callbacks::EDGES)
}
fn tech_frame_waits_with_callbacks(graph: &Graph, runtime: &Path, callbacks:&[(&str,&str)]) -> BTreeSet<String> {
    game_pass_waits(graph, runtime, "tech::frame", callbacks, false)
}

fn game_pass_waits(
    graph: &Graph, runtime: &Path, root: &str, callbacks: &[(&str, &str)], reject_syscalls: bool,
) -> BTreeSet<String> {
    let roots = graph.find(root);
    assert_eq!(roots.len(), 1, "{root} must remain an explicit audit root");
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, f) in graph.funcs.iter().enumerate() {
        by_name.entry(f.name.clone()).or_default().push(i);
    }
    let locks = static_locks(runtime);
    let cells = static_types(runtime, &["OnceLock<", "OnceCell<"]);
    let mut stack: Vec<_> = roots.into_iter().map(|i| (i, String::new())).collect();
    let mut seen = BTreeSet::new();
    let mut violations = BTreeSet::new();
    while let Some((i, via)) = stack.pop() {
        if !seen.insert(i) { continue; }
        if non_switch_only(&graph.funcs[i], runtime) { continue; }
        let key = graph.funcs[i].key();
        let path = if via.is_empty() { key.clone() } else { format!("{via} -> {key}") };
        for (file, name) in &locks {
            if takes_lock_on_thread(graph, i, file, name, &["lock", "read", "write"], true, Some(false)) {
                violations.insert(format!("{path} blocks on {file}::{name}"));
            }
        }
        for (file, name) in &cells {
            if takes_lock_on_thread(graph, i, file, name, &["set", "get_or_init", "get_or_try_init", "wait"], false, Some(false)) {
                violations.insert(format!("{path} initializes or waits on {file}::{name}"));
            }
        }
        // Same explicit allocation spellings covered by the existing HUD audit.
        let plugin_range = callgraph::thread_dispatch(&graph.funcs[i].body).map(|(p, _)| p);
        if graph.funcs[i].body.iter().enumerate().any(|(at, t)| !plugin_range.as_ref().is_some_and(|p| p.contains(&at))
            && matches!(t, Tok::Ident(s) if ["Box", "Vec", "String", "alloc"].contains(&s.as_str()))) {
            violations.insert(format!("{path} reaches an allocation spelling"));
        }
        if reject_syscalls && graph.funcs[i].body.iter().any(|token|
            matches!(token, Tok::Ident(name) if ["asm", "svc", "syscall"].contains(&name.as_str()))
        ) {
            violations.insert(format!("{path} reaches a syscall"));
        }
        stack.extend(edges_on_thread(graph, i, &by_name, &mut BTreeMap::new(), false).into_iter().map(|next| (next, path.clone())));
        stack.extend(callbacks.iter().filter(|(from,_)| *from==key).flat_map(|(_,to)| graph.find(to))
            .map(|next| (next,path.clone())));
    }
    violations
}

#[test]
fn edge_juggle_passes_are_nonblocking() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut graph = Graph::scan(&runtime, &[]);
    graph.precise_imports = true;
    for root in ["edge_juggle::frame", "edge_juggle::poll", "edge_juggle::look", "edge_juggle::ai_correction", "tech_surfaces::frame_read_line", "tech_surfaces::frame_next_line"] {
        let violations = game_pass_waits(&graph, &runtime, root, &[], true);
        assert!(violations.is_empty(), "{root}: {violations:#?}");
    }
    let root = "edge_juggle::frame";
    let frame = graph.find(root)[0];
    graph.funcs[frame].body = callgraph::lex("ENGINE.lock();");
    assert!(game_pass_waits(&graph, &runtime, root, &[], true)
        .iter().any(|entry| entry.contains("ENGINE")));
}

#[test]
fn tech_frame_has_no_blocking_lock_initialization_or_allocation() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut graph = Graph::scan(&runtime, &[]);
    graph.precise_imports = true;
    let violations = tech_frame_waits(&graph, &runtime);
    assert!(violations.is_empty(), "Tech frame hook can wait or allocate:\n{}", violations.into_iter().collect::<Vec<_>>().join("\n"));
}

#[test]
fn tech_frame_audit_rejects_reachable_lock_initialization_and_allocation_seeds() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut graph = Graph::scan(&runtime, &[]);
    graph.precise_imports = true;
    let frame = graph.find("tech::frame")[0];
    let helper = graph.find("tech::still_tumbling")[0];
    // Remove the binding path: the root itself must retain this coverage.
    graph.funcs[frame].body = callgraph::lex("still_tumbling(0);");
    for (seed, expected) in [
        ("VIEW.lock();", "blocks on features/tech.rs::VIEW"),
        ("TUMBLE_IDS.set([0; 7]);", "initializes or waits on features/tech.rs::TUMBLE_IDS"),
        ("let allocated = Vec::new();", "reaches an allocation spelling"),
    ] {
        graph.funcs[helper].body = callgraph::lex(seed);
        let found = tech_frame_waits(&graph, &runtime);
        assert!(found.iter().any(|v| v.contains(expected)), "reachable seed escaped: {seed}: {found:?}");
    }
    graph.funcs[helper].body = callgraph::lex("VIEW.try_lock(); TUMBLE_IDS.get();");
    assert!(tech_frame_waits(&graph, &runtime).is_empty(), "nonblocking access was rejected");
}

#[test]
fn tech_frame_audit_follows_game_branch_and_switch_implementation() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut graph = Graph::scan(&runtime, &[]);
    graph.precise_imports = true;
    let frame = graph.find("tech::frame")[0];
    let helper = graph.find("tech::still_tumbling")[0];
    graph.funcs[frame].body = callgraph::lex("still_tumbling(0);");
    graph.funcs[helper].body = callgraph::lex("if threads::on_plugin_thread() { VIEW.lock(); } else { VIEW.try_lock(); }");
    assert!(tech_frame_waits(&graph, &runtime).is_empty(), "plugin branch was walked as a game hook");
    graph.funcs[helper].body = callgraph::lex("if threads::on_plugin_thread() { VIEW.try_lock(); } else { VIEW.lock(); }");
    assert!(tech_frame_waits(&graph, &runtime).iter().any(|v| v.contains("blocks on features/tech.rs::VIEW")), "game branch wait escaped");
    // Use real target attributes, not a function-name exemption. A wait seeded
    // into the host fallback is irrelevant; the Switch implementation must fail.
    let variants = graph.find("clock::uptime_ms");
    assert_eq!(variants.len(), 2);
    let host = *variants.iter().find(|&&i| non_switch_only(&graph.funcs[i], &runtime)).unwrap();
    let switch = *variants.iter().find(|&&i| i != host).unwrap();
    graph.funcs[frame].body = callgraph::lex("clock::uptime_ms();");
    graph.funcs[host].body = callgraph::lex("tech::VIEW.lock();");
    graph.funcs[switch].body.clear();
    assert!(tech_frame_waits(&graph, &runtime).is_empty(), "host-only fallback was walked as Switch code");
    graph.funcs[switch].body = callgraph::lex("tech::VIEW.lock();");
    assert!(tech_frame_waits(&graph, &runtime).iter().any(|v| v.contains("blocks on features/tech.rs::VIEW")), "Switch wait escaped");
}

/// The audit's raw material (IMPLEMENTATION-PROGRESS-sync.md (removed after v0.2.0-rc.1; see that tag)): every static lock (ours and carried package-6 code, std or hos-lock)
/// a plugin-thread entry reaches, with the function that takes it. Prints; never fails
/// (`cargo test --test all -- hook_locks::report_locks_reached_from_plugin_threads --exact --nocapture`).
#[test]
fn report_locks_reached_from_plugin_threads() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let locks = static_locks(&runtime);
    let refs: Vec<(&str, &str)> = locks.iter().map(|(f, n)| (f.as_str(), n.as_str())).collect();
    let entries = plugin_thread_entries();
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    let result = scan(&runtime, &entries, &refs, &[]);
    println!("{} static locks; reached from plugin threads:", locks.len());
    for line in &result.violations {
        println!("LOCK-REACH {line}");
    }
    println!("AMBIGUOUS {:?}", result.ambiguous);
}

/// The static locks taken more than once in one statement of `src`: each `NAME.lock()` guard is a temporary that lives to
/// the end of its statement, and neither hos-lock lock is reentrant, so the second take waits on the first for ever
/// (review IV-1, 2026-11-05: the frame strip's diag line took `IDS` four times). `names` are the locks `src` declares; a
/// statement ends at `;`, `{` or `}`, and a match arm (`=>`) is its own temporary scope. Text-level, so
/// `cfg(feature = "diag")` code is read too.
fn relocks_in_one_statement(src: &str, names: &BTreeSet<String>) -> Vec<String> {
    let toks = callgraph::lex(src);
    let ident = |k: usize, s: &str| matches!(toks.get(k), Some(Tok::Ident(t)) if t == s);
    let mut out = Vec::new();
    let mut taken: BTreeMap<String, usize> = BTreeMap::new();
    for i in 0..toks.len() {
        match &toks[i] {
            Tok::Punct(';' | '{' | '}') => taken.clear(),
            Tok::Punct('>') if i > 0 && toks[i - 1] == Tok::Punct('=') => taken.clear(),
            Tok::Ident(name)
                if names.contains(name)
                    && matches!(toks.get(i + 1), Some(Tok::Punct('.')))
                    && LOCK_METHODS.iter().any(|m| ident(i + 2, m))
                    && matches!(toks.get(i + 3), Some(Tok::Punct('('))) =>
            {
                let n = taken.entry(name.clone()).or_default();
                *n += 1;
                if *n == 2 {
                    out.push(name.clone());
                }
            }
            _ => {}
        }
    }
    out
}

#[test]
fn no_statement_takes_the_same_lock_twice() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut by_file: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (file, name) in static_locks(&runtime) {
        by_file.entry(file).or_default().insert(name);
    }
    let mut found = Vec::new();
    for (file, names) in &by_file {
        let src = std::fs::read_to_string(runtime.join(file)).unwrap();
        for name in relocks_in_one_statement(&src, names) {
            found.push(format!("{file}: {name}"));
        }
    }
    assert!(by_file.get("features/input_display.rs").is_some_and(|n| n.contains("IDS")), "the scan sees the strip's locks");
    assert!(found.is_empty(), "a statement takes a lock it already holds (it waits on itself): {found:?}");
}

#[test]
fn the_relock_scan_catches_the_strip_line_and_passes_separate_statements() {
    let names: BTreeSet<String> = ["IDS".to_owned()].into();
    // Review IV-1 as committed in 5ff2e91.
    let bad = r#"fn ids() { diagnostic(format_args!("{} {}", IDS.lock().as_ref().map_or(0, |t| t.get(C::A)),
        IDS.lock().as_ref().map_or(0, |t| t.get(C::B)))); }"#;
    assert_eq!(relocks_in_one_statement(bad, &names), ["IDS"]);
    let good = "fn ids() { let a = IDS.lock().as_ref().map_or(0, |t| t.get(C::A)); *IDS.lock() = None; if x { IDS.lock(); } }";
    assert!(relocks_in_one_statement(good, &names).is_empty());
    let read_twice = "fn f() { g(IDS.read().len(), IDS.try_read().is_some()); }";
    assert_eq!(relocks_in_one_statement(read_twice, &names), ["IDS"]);
    // Each match arm drops its temporaries (replay_capture's `read_capture_source`).
    let arms = "fn f() { let x = match id { a => (IDS.lock().unwrap().p.clone(), 1), b => (IDS.lock().unwrap().p.clone(), 2) }; }";
    assert!(relocks_in_one_statement(arms, &names).is_empty());
}

/// Modules declared under `#[cfg(feature = "diag")]` (`mod x;` on the line after the attribute): Eden-only code the scan does not read.
fn diag_only_modules(runtime: &Path) -> BTreeSet<String> {
    fn walk(dir: &Path, out: &mut BTreeSet<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                let lines: Vec<&str> = text.lines().map(str::trim).collect();
                for pair in lines.windows(2) {
                    if pair[0] == "#[cfg(feature = \"diag\")]" {
                        let decl = pair[1].trim_start_matches("pub(crate) ").trim_start_matches("pub ");
                        if let Some(name) = decl.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
                            out.insert(name.trim().to_owned());
                        }
                    }
                }
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(runtime, &mut out);
    out
}

#[test]
fn tech_callbacks_match_production_binding_and_model_calls() {
    let root=Path::new(env!("CARGO_MANIFEST_DIR"));
    let graph=Graph::scan(&root.join("src/runtime"),&[]);
    for &(from,to) in tech_callbacks::EDGES {
        assert_eq!(graph.find(from).len(),1,"callback source {from}");
        assert_eq!(graph.find(to).len(),1,"callback target {to}");
    }
    let compact=|s:String| s.split_whitespace().collect::<String>();
    let ui=compact(std::fs::read_to_string(root.join("src/runtime/features/tech.rs")).unwrap());
    let controller=compact(std::fs::read_to_string(root.join("src/model/tech/controller.rs")).unwrap());
    let cpu=compact(std::fs::read_to_string(root.join("src/runtime/features/cpu.rs")).unwrap());
    let adapter=compact(std::fs::read_to_string(root.join("src/runtime/features/tech_adapter.rs")).unwrap());
    for call in ["Controller::tick(&mutrun.controller,&mutrun.adapter,", "run.controller.place(&mutrun.adapter,setup)", "run.controller.stop(&mutrun.adapter)", "Adapter::new(hitlag)", "HITLAG.get().copied()", "HITLAG.set(resolver)"] {
        assert!(ui.contains(call),"production callback mapping changed: {call}");
    }
    for call in ["launcher.place(setup.launch)?", "launcher.attempt()", "launcher.restore()", "launcher.launch()?"] {
        assert!(controller.contains(call),"model callback changed: {call}");
    }
    assert!(cpu.contains("super::tech::bind_hitlag(super::tech_hitlag::PROVIDER)"));
    let native=compact(std::fs::read_to_string(root.join("src/runtime/features/tech_hitlag.rs")).unwrap());
    assert!(native.contains("pubconstPROVIDER:tech_adapter::HitlagResolver=resolve;"));
    assert!(adapter.contains("(self.hitlag)(accessor,owned.launch.hit_damage)?"));
}
#[test]
fn tech_frame_audit_follows_model_launcher_and_injected_provider_callbacks() {
    let runtime=Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut graph=Graph::scan(&runtime,&[]);graph.precise_imports=true;
    let resolver=graph.find("tech_hitlag::resolve")[0];
    let original=graph.funcs[resolver].body.clone();
    graph.funcs[resolver].body=callgraph::lex("tech_adapter::OWNED.lock();");
    // Negative controls reproduce both missing links; production uses the complete table.
    assert!(tech_frame_waits_with_callbacks(&graph,&runtime,&[]).is_empty());
    assert!(tech_frame_waits_with_callbacks(&graph,&runtime,&[("tech_adapter::launch","tech_hitlag::resolve")]).is_empty());
    let found=tech_frame_waits(&graph,&runtime);
    assert!(found.iter().any(|v|v.contains("tech_adapter::launch -> tech_hitlag::resolve blocks on features/tech_adapter.rs::OWNED")),"provider blocking seed escaped: {found:?}");
    for callback in ["tech_adapter::place","tech_adapter::attempt","tech_adapter::restore","tech_adapter::launch"] {
        let target=graph.find(callback)[0];
        let old=graph.funcs[target].body.clone();
        graph.funcs[target].body=callgraph::lex("OWNED.lock();");
        assert!(tech_frame_waits(&graph,&runtime).iter().any(|v|v.contains(&format!("{callback} blocks on features/tech_adapter.rs::OWNED"))),"model callback seed escaped: {callback}");
        graph.funcs[target].body=old;
    }
    graph.funcs[resolver].body=callgraph::lex("tech_adapter::OWNED.try_lock();");
    assert!(tech_frame_waits(&graph,&runtime).is_empty());
    graph.funcs[resolver].body=original;
    assert!(tech_frame_waits(&graph,&runtime).is_empty());
}
