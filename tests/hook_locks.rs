//! stream/sync (operator order 2026-11-02): a game thread never takes a lock that another thread can hold across real work.
//! The locks in `HOOK_LOCKS` are taken by game-thread hooks. This test follows the runtime's call graph (`tests/common/callgraph.rs`)
//! from every entry point that runs on a plugin thread (the menu worker, the writer threads, the loaders, the publishing path)
//! and fails if any function it reaches takes one of them. A plugin thread hands values to the game through
//! `hos_lock::Published`, atomics or a lock-free mailbox instead (IMPLEMENTATION-PROGRESS-sync.md, audit table).
//!
//! How the scan reads the code:
//! - **Lock sites:** a function takes a lock when its body has the static followed by `.lock(` / `.read(` / `.write(` /
//!   `.try_lock(` / `.try_read(` / `.try_write(`, or hands it by reference to a call (`lock_write(&MOTION)`, the carried code's
//!   helpers). The static is matched in its own file, or by `module::NAME` elsewhere.
//! - **Edges:** path calls (`module::f(..)`, `f(..)`, a function handed over by path) as the capability scan resolves them. A
//!   method call `.m(..)` is followed when exactly one runtime function is named `m`; when several are, the edge is not followed
//!   and the name is listed in the report (`AMBIGUOUS`), so nothing is dropped without a trace. Methods of model types and of the
//!   locks are not runtime functions and have no edge.
//! - **Limits:** calls through `dyn` objects, trait methods, closures stored and called later, and function pointers kept in data
//!   are not seen (the capability scan's `dynamic` edges are the place to add one); a closure body counts as part of the function
//!   that contains it; a static reached through a re-export is matched only through `ALIASES`. `diag`-only modules are not scanned
//!   (they never ship).
//!
//! `the_scan_catches_a_seeded_violation` runs the same scan on `tests/fixtures/hook_locks/`, a small runtime with one plugin thread
//! that takes a hook's lock through a method call and through a helper, and requires both to be reported.

#[path = "common/callgraph.rs"]
mod callgraph;

use std::collections::{BTreeMap, BTreeSet};
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
    // Macro Source = Named Library reserves the layer's buffers on the publishing thread: the allocation runs before `MOTION` is
    // taken, and the write lock only swaps the reserved buffers in (pkg4 manifest edit, stream/sync).
    ("named_macros::prepare_buffers", "MOTION", "write lock held for a buffer swap only; the allocation is outside"),
];

const LOCK_METHODS: &[&str] = &["lock", "read", "write", "try_lock", "try_read", "try_write"];

/// `(file, module name)`: a re-export a static is also reached by (`compat::common::menu` re-exports `ui::menu_app`'s statics).
const ALIASES: &[(&str, &str)] = &[("ui/menu_app.rs", "menu"), ("ui/menu_app.rs", "menu_app")];

/// Whether function `index` takes the lock `name` of `file` (see the module comment).
fn takes_lock(graph: &Graph, index: usize, file: &str, name: &str) -> bool {
    let f = &graph.funcs[index];
    let module = Path::new(file).file_stem().unwrap().to_string_lossy().into_owned();
    let toks = &f.body;
    let ident = |k: usize, s: &str| matches!(toks.get(k), Some(Tok::Ident(t)) if t == s);
    let punct = |k: usize, c: char| matches!(toks.get(k), Some(Tok::Punct(p)) if *p == c);
    for i in 0..toks.len() {
        if !ident(i, name) {
            continue;
        }
        let qualified = i >= 2
            && toks[i - 1] == Tok::PathSep
            && (ident(i - 2, &module) || ALIASES.iter().any(|(f, m)| *f == file && ident(i - 2, m)));
        if f.file != file && !qualified {
            continue;
        }
        let method = punct(i + 1, '.') && LOCK_METHODS.iter().any(|m| ident(i + 2, m)) && punct(i + 3, '(');
        let start = if qualified { i - 2 } else { i };
        let by_ref = start >= 1 && punct(start - 1, '&');
        if method || by_ref {
            return true;
        }
    }
    false
}

/// Callees of `index`: path calls, plus method calls whose name is unique among the scanned functions. Ambiguous method names
/// are recorded in `ambiguous` (name -> how many functions carry it).
fn edges(graph: &Graph, index: usize, by_name: &BTreeMap<String, Vec<usize>>, ambiguous: &mut BTreeMap<String, usize>) -> BTreeSet<usize> {
    let refs = Graph::references(&graph.funcs[index]);
    let path_names: BTreeSet<&String> = refs.iter().filter(|(_, method)| !method).filter_map(|(s, _)| s.last()).collect();
    let mut out: BTreeSet<usize> = graph.callees(index).into_iter().filter(|&c| path_names.contains(&graph.funcs[c].name)).collect();
    for (segments, method) in &refs {
        if !method {
            continue;
        }
        let Some(name) = segments.last() else { continue };
        if LOCK_METHODS.contains(&name.as_str()) {
            continue;
        }
        match by_name.get(name).map(Vec::as_slice) {
            Some([only]) if *only != index => {
                out.insert(*only);
            }
            Some(many) if many.len() > 1 => {
                ambiguous.insert(name.clone(), many.len());
            }
            _ => {}
        }
    }
    out
}

struct Scan {
    violations: BTreeSet<String>,
    ambiguous: BTreeMap<String, usize>,
}

fn scan(runtime: &Path, entries: &[&str], locks: &[(&str, &str)], allowed: &[(&str, &str, &str)]) -> Scan {
    let graph = Graph::scan(runtime, &[]);
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, f) in graph.funcs.iter().enumerate() {
        by_name.entry(f.name.clone()).or_default().push(i);
    }
    let mut violations = BTreeSet::new();
    let mut ambiguous = BTreeMap::new();
    for entry in entries {
        let starts = graph.find(entry);
        assert!(!starts.is_empty(), "entry {entry} not found under {}", runtime.display());
        let mut seen = BTreeSet::new();
        let mut stack: Vec<(usize, String)> = starts.into_iter().map(|s| (s, (*entry).to_owned())).collect();
        while let Some((i, via)) = stack.pop() {
            if !seen.insert(i) {
                continue;
            }
            let key = graph.funcs[i].key();
            for (file, lock) in locks {
                if takes_lock(&graph, i, file, lock) && !allowed.iter().any(|(f, l, _)| *f == key && l == lock) {
                    violations.insert(format!("{entry} -> ... -> {key} takes {file}::{lock} (called from {via})"));
                }
            }
            for callee in edges(&graph, i, &by_name, &mut ambiguous) {
                stack.push((callee, key.clone()));
            }
        }
    }
    Scan { violations, ambiguous }
}

#[test]
fn no_plugin_thread_reaches_a_lock_a_hook_takes() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let result = scan(&runtime, PLUGIN_THREAD_ENTRIES, HOOK_LOCKS, ALLOWED);
    println!("AMBIGUOUS method names not followed (name: functions): {:?}", result.ambiguous);
    assert!(
        result.violations.is_empty(),
        "plugin threads reach locks hooks take:\n{}",
        result.violations.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn the_scan_catches_a_seeded_violation() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hook_locks");
    let result = scan(&fixture, &["worker::run"], &[("features/chords.rs", "STATE"), ("macros/named_macros.rs", "MOTION")], &[]);
    let text = result.violations.iter().cloned().collect::<Vec<_>>().join("\n");
    assert!(text.contains("chords::refresh_rows takes features/chords.rs::STATE"), "method-call edge and lock site missed:\n{text}");
    assert!(text.contains("named_macros::reserve takes macros/named_macros.rs::MOTION"), "helper by reference missed:\n{text}");
    assert!(!text.contains("on_poll"), "a hook-only function was reported:\n{text}");
    assert_eq!(result.ambiguous.get("tick"), Some(&2), "ambiguous names must be listed: {:?}", result.ambiguous);
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

/// The audit's raw material (IMPLEMENTATION-PROGRESS-sync.md): every static lock (ours and carried package-6 code, std or hos-lock)
/// a plugin-thread entry reaches, with the function that takes it. Prints; never fails
/// (`cargo test --test hook_locks report -- --nocapture`).
#[test]
fn report_locks_reached_from_plugin_threads() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let locks = static_locks(&runtime);
    let refs: Vec<(&str, &str)> = locks.iter().map(|(f, n)| (f.as_str(), n.as_str())).collect();
    let result = scan(&runtime, PLUGIN_THREAD_ENTRIES, &refs, &[]);
    println!("{} static locks; reached from plugin threads:", locks.len());
    for line in &result.violations {
        println!("LOCK-REACH {line}");
    }
    println!("AMBIGUOUS {:?}", result.ambiguous);
}
