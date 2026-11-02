//! stream/sync (operator order 2026-11-02): a game thread never takes a lock that another thread can hold across real work.
//! The locks below are taken by game-thread hooks; this test follows the runtime's call graph (`tests/common/callgraph.rs`)
//! from every entry point that runs on a plugin thread (the menu worker, the writer threads, the loaders, the publishing
//! path) and fails if any function it reaches names one of them. A plugin thread hands values to the game through
//! `hos_lock::Published`, atomics or a lock-free mailbox instead (IMPLEMENTATION-PROGRESS-sync.md, audit table).
//!
//! The graph is the capability scan's, followed along path calls only (`module::f(..)`, `f(..)`, a function handed over by path):
//! a method call there reaches every function of that name in the runtime (`.with(` on a ring would reach `cpu::with`), which made
//! every entry reach every lock. The runtime's functions call each other by path; methods are the model's and the locks' own. An
//! entry in `ALLOWED` names the function, the lock and why it is not a wait of a hook on real work.

#[path = "common/callgraph.rs"]
mod callgraph;

use std::collections::BTreeSet;
use std::path::Path;

use callgraph::{Graph, Tok};

/// `(file under src/runtime, static)`: locks game-thread hooks take.
const HOOK_LOCKS: &[(&str, &str)] = &[
    ("features/chords.rs", "STATE"),
    ("features/cpu.rs", "CPU"),
    ("features/states.rs", "STATE"),
    ("features/recorder.rs", "REC"),
    ("features/recorder.rs", "RNG"),
    ("features/input_display.rs", "LOG"),
    ("features/input_display.rs", "RENDERER"),
    ("features/input_delay.rs", "ADAPTER"),
    ("features/notices.rs", "QUEUE"),
    ("features/hud_draw.rs", "CONTAINERS"),
    ("compat/menu.rs", "EDITED"),
    ("ui/input.rs", "ADAPTER"),
    ("ui/menu_app.rs", "QUICK_MENU_APP"),
    ("macros/named_macros.rs", "MOTION"),
    ("macros/named_macros.rs", "LEGACY"),
];

/// `module::function` of every entry point that runs on a plugin thread (or is the publishing path a plugin thread calls).
const PLUGIN_THREAD_ENTRIES: &[&str] = &[
    "worker::run",
    "writer::run",
    "published::publish",
    "menu::apply_kept",
    "states::start_writer",
    "diag_log::start_drain_thread",
    "import::start",
    "import::spawn_background",
    "replay_store::start_writer",
    "replay_store::fallback_catalog",
];

/// `(function key, lock)` reached from a plugin thread on purpose, with the reason it is not a hook waiting on real work.
const ALLOWED: &[(&str, &str, &str)] = &[
    // The typed menu's game-side edit is read on the publishing thread to give package 6's re-plan the previous view; every
    // taker holds `EDITED` for one copy, nothing else.
    ("menu::current_menu", "EDITED", "held for one copy by every taker"),
];

/// The callees of `index` that its body names by path (method-call edges dropped, see the module comment).
fn path_callees(graph: &Graph, index: usize) -> Vec<usize> {
    let names: BTreeSet<String> =
        Graph::references(&graph.funcs[index]).into_iter().filter(|(_, method)| !method).filter_map(|(segments, _)| segments.last().cloned()).collect();
    graph.callees(index).into_iter().filter(|&c| names.contains(&graph.funcs[c].name)).collect()
}
fn names_lock(graph: &Graph, index: usize, file: &str, name: &str) -> bool {
    let f = &graph.funcs[index];
    let module = Path::new(file).file_stem().unwrap().to_string_lossy().into_owned();
    let toks = &f.body;
    for (i, t) in toks.iter().enumerate() {
        if matches!(t, Tok::Ident(s) if s == name) {
            let qualified = i >= 2 && toks[i - 1] == Tok::PathSep && matches!(&toks[i - 2], Tok::Ident(m) if *m == module);
            if f.file == file || qualified {
                return true;
            }
        }
    }
    false
}

#[test]
fn no_plugin_thread_reaches_a_lock_a_hook_takes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let graph = Graph::scan(&root.join("src/runtime"), &[]);
    let mut violations = BTreeSet::new();
    for entry in PLUGIN_THREAD_ENTRIES {
        let starts = graph.find(entry);
        assert!(!starts.is_empty(), "entry {entry} not found in the runtime");
        let mut seen = BTreeSet::new();
        let mut stack: Vec<(usize, String)> = starts.into_iter().map(|s| (s, (*entry).to_owned())).collect();
        while let Some((i, via)) = stack.pop() {
            if !seen.insert(i) {
                continue;
            }
            for (file, lock) in HOOK_LOCKS {
                if names_lock(&graph, i, file, lock) {
                    let key = graph.funcs[i].key();
                    if !ALLOWED.iter().any(|(f, l, _)| *f == key && l == lock) {
                        violations.insert(format!("{entry} -> ... -> {key} names {file}::{lock} (via {via})"));
                    }
                }
            }
            for callee in path_callees(&graph, i) {
                stack.push((callee, graph.funcs[i].key()));
            }
        }
    }
    assert!(violations.is_empty(), "plugin threads reach locks hooks take:\n{}", violations.into_iter().collect::<Vec<_>>().join("\n"));
}

/// Every `static NAME: …Mutex<…>` / `…RwLock<…>` of the runtime (ours and carried package-6 code, std or hos-lock).
fn static_locks(root: &Path) -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                for line in std::fs::read_to_string(&path).unwrap().lines() {
                    let line = line.trim_start();
                    let rest = line.strip_prefix("pub ").or_else(|| line.strip_prefix("pub(crate) ")).unwrap_or(line);
                    let Some(rest) = rest.strip_prefix("static ") else { continue };
                    let Some((name, ty)) = rest.split_once(':') else { continue };
                    if ty.contains("Mutex<") || ty.contains("RwLock<") {
                        out.push((rel.clone(), name.trim().to_owned()));
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

/// The audit's raw material (IMPLEMENTATION-PROGRESS-sync.md): which static locks a plugin-thread entry reaches, and through which
/// function. Prints; never fails (`cargo test --test hook_locks -- --nocapture`).
#[test]
fn report_locks_reached_from_plugin_threads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let graph = Graph::scan(&root, &[]);
    let locks = static_locks(&root);
    let mut lines = BTreeSet::new();
    for entry in PLUGIN_THREAD_ENTRIES {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<usize> = graph.find(entry);
        while let Some(i) = stack.pop() {
            if !seen.insert(i) {
                continue;
            }
            for (file, lock) in &locks {
                if names_lock(&graph, i, file, lock) {
                    lines.insert(format!("LOCK-REACH {file}::{lock} <- {} (entry {entry})", graph.funcs[i].key()));
                }
            }
            stack.extend(path_callees(&graph, i));
        }
    }
    println!("{} static locks; reached from plugin threads:", locks.len());
    for line in lines {
        println!("{line}");
    }
}
