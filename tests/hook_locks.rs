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
    ("features/states/quick_reset.rs", "QUICK"),
    ("features/states/preview.rs", "CHECKPOINT"),
    ("features/states/setups.rs", "FACTS"),
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

#[path = "../src/model/thread_stacks.rs"]
#[allow(dead_code)]
mod thread_stacks;

/// Thread entries that exist only under a non-shipped probe feature, in a block the call-graph scan does not resolve: skipped while the
/// graph has no such function (the scan still follows them when it does). Every other missing entry fails the test.
const FEATURE_ONLY_ENTRIES: &[(&str, &str)] = &[("practice_trace::writer", "console-practice-trace (diag probe feature) only, inside a cfg-gated block")];

/// Entry points on the publishing path that a plugin thread calls (not threads of their own).
const PUBLISHING_ENTRIES: &[&str] = &["published::publish", "menu::apply_kept"];

/// `module::function` of every plugin thread's entry, derived from `model::thread_stacks::THREADS` (each row's `symbol`, its closure
/// suffix dropped), so a new or renamed thread cannot be missed; the diag-only threads (`runs` starts with `diag:`) live in modules the
/// scan does not read. Plus the publishing path.
fn plugin_thread_entries() -> Vec<String> {
    let diag_only = diag_only_modules(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"));
    let mut out: Vec<String> = PUBLISHING_ENTRIES.iter().map(|s| (*s).to_owned()).collect();
    for row in thread_stacks::THREADS.iter() {
        let path = row.symbol.split("::{").next().unwrap_or(row.symbol);
        if row.runs.starts_with("diag:") || path.split("::").any(|m| diag_only.contains(m)) {
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
    // Macro Source = Named Library reserves the layer's buffers on the publishing thread: the allocation runs before `MOTION` is
    // taken, and the write lock only swaps the reserved buffers in (pkg4 manifest edit, stream/sync).
    ("named_macros::prepare_buffers", "MOTION", "write lock held for a buffer swap only; the allocation is outside"),
];

/// A ratchet, not a tolerance (lead, 2026-11-02): violations whose fix is in progress elsewhere, each (function key, file::lock, owner,
/// the fix that removes it). The test fails on any violation not listed here, AND on a row that no longer occurs (delete it the moment the
/// fix lands), and prints the table every run. The release-candidate gate requires this table to be empty.
const PENDING: &[(&str, &str, &str, &str)] = &[(
    "recorder::rec",
    "features/recorder.rs::REC",
    "package 4 (a93ab192186b224a5)",
    "design C: the named layer's recorder calls on the menu worker (recorder::with_recorder) move to its published MacroQueries and mailbox",
)];

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
                q if q == callee.module => true,
                q => !by_name.get(&callee.name).is_some_and(|all| all.iter().any(|&o| graph.funcs[o].module == q)),
            }
        })
    };
    let mut out: BTreeSet<usize> = graph.callees(index).into_iter().filter(|&c| path_match(c)).collect();
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
        if starts.is_empty() && FEATURE_ONLY_ENTRIES.iter().any(|(e, _)| e == entry) {
            continue;
        }
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
    let entries = plugin_thread_entries();
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    let result = scan(&runtime, &entries, &refs, &[]);
    println!("{} static locks; reached from plugin threads:", locks.len());
    for line in &result.violations {
        println!("LOCK-REACH {line}");
    }
    println!("AMBIGUOUS {:?}", result.ambiguous);
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
