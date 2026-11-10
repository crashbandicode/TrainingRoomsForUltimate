//! rc.6 menu-close freeze (primary 2026-11-09 12:28Z; sightings covrest-yoga-05 and rc6-bg-qp3): the static lock-order
//! audit. The game thread stopped at a plugin menu close twice, with no lines after `[Ui] press B`. A deadlock between
//! threads needs two locks taken in opposite orders, or one thread retaking a lock it already holds (hos-lock's locks
//! are not reentrant). This scan reads the runtime's call graph (`tests/common/callgraph.rs`, the same scanner and call
//! resolution as `tests/hook_locks.rs`) and:
//! - finds every blocking acquisition of a static lock (`NAME.lock()` / `.read()` / `.write()`, or a guard handed back
//!   by `lock_write(&NAME)` / `lock_read(&NAME)`); `try_*` never waits and is not an acquisition here;
//! - works out how long its guard is held: a guard bound by `let` until `drop(name)` or the end of its block, a
//!   temporary until the end of its statement (through a `match`/`if let` block it heads, edition 2021);
//! - records an order edge A -> B for every lock B taken while A is held, directly or through any call made then;
//! - names the threads that reach each edge: game-thread hooks (`#[skyline::hook]`) and every plugin thread's entry
//!   (`model::thread_stacks::THREADS`, as `hook_locks.rs` derives them, plus the publishing path).
//!
//! `cargo test --test lock_order -- --nocapture` prints the lock table and the edges. The test fails on a cycle in the
//! order graph (two locks taken in both orders), and on a lock retaken while held. Limits are `hook_locks.rs`'s: calls
//! through `dyn`, trait methods and stored closures are not seen; native locks (the game's own) are not seen.

#[path = "common/callgraph.rs"]
mod callgraph;

#[path = "../src/model/thread_stacks.rs"]
#[allow(dead_code)]
mod thread_stacks;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use callgraph::{Graph, Tok};

const LOCK_METHODS: &[&str] = &["lock", "read", "write"];
const GUARD_HELPERS: &[&str] = &["lock_write", "lock_read"];
const WAITS: &[&str] = &["sleep", "park", "park_timeout", "recv", "recv_timeout", "join", "wait", "wait_timeout", "wait_while"];
const PUBLISHING_ENTRIES: &[&str] = &["published::publish", "menu::apply_kept"];
const ALIASES: &[(&str, &str)] = &[("ui/menu_app.rs", "menu"), ("ui/menu_app.rs", "menu_app")];

/// `(function, via)` edges the scan reports that the code does not have, each with the reason (read in the source).
const NOT_AN_EDGE: &[(&str, &str, &str)] = &[(
    "named_macros::offer_external_name_titled",
    "setups::offer_name",
    "`lock_write(&UI).offer_name(..)` calls the model's `MacroUi::offer_name`; the scan resolves a method by its name, and \
     the only runtime receiver method of that name is `setups::Ui::offer_name`",
)];

fn punct(t: Option<&Tok>, c: char) -> bool {
    matches!(t, Some(Tok::Punct(p)) if *p == c)
}

fn ident(t: Option<&Tok>) -> Option<&str> {
    match t {
        Some(Tok::Ident(s)) => Some(s),
        _ => None,
    }
}

/// The runtime's call graph, a bare imported name resolved as Rust does (`Graph::precise_imports`): `lobby_persist`'s `restore` is
/// `model::lobby_persist::restore`, not `kirby_hats::restore` (which made `entry_applied` look like it waits on the source lock).
fn runtime_graph() -> Graph {
    let mut graph = Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[]);
    graph.precise_imports = true;
    graph
}

/// Every `static NAME: …Mutex<…>` / `…RwLock<…>` of `root` (the scan of `hook_locks.rs`).
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

/// Receivers named otherwise than their type's module (docs lane review R2, finding 1): the inline keyboard is held as `keyboard`
/// (`name_keyboard`, `native_keyboard`: `inline_keyboard::Keyboard`), the catalog request's `Broker` as `broker`.
const RECEIVER_MODULES: &[(&str, &str)] = &[("keyboard", "inline_keyboard"), ("broker", "mii_catalog_request")];

/// Ambiguous method calls left unfollowed although a runtime candidate reaches a lock, by `(method, receivers, why none of the runtime
/// candidates is meant)`; `?` is a receiver that is not an identifier (a call's result, a path). Docs lane review R2, finding 1.
const FOREIGN_RECEIVERS: &[(&str, &[&str], &str)] = &[
    ("read_value", &["MENU", "source"], "the compat Readable impls: TYPED_STATIC_CALLS follows MENU's; the others take no lock"),
    ("assign_value", &["target"], "the compat Assignable impls: TYPED_STATIC_CALLS follows MENU's (assign -> store)"),
    ("open", &["?", "keeper", "list"], PANELS),
    ("tick", &["?", "queue", "ui"], PANELS),
    ("press", &["editor", "list", "m", "menu"], PANELS),
    ("view", &["bundle", "editor", "extras", "ui"], PANELS),
    ("target", &["active", "cycle", "editor", "proof"], "combos::Ui::target is private to ui/combos.rs"),
    (
        "poll",
        &["?", "JOBS", "adapter", "clock", "detector", "gate", "guard", "model", "queue", "replies", "reply", "t"],
        "inline_keyboard::Keyboard is held only as `keyboard`, mii_catalog_request's private Broker only as `broker` \
         (RECEIVER_MODULES); these are engine exchanges and queues, input adapters, the model's catalog reply",
    ),
    ("close", &["queue"], "practice_trace_core's queue; inline_keyboard::Keyboard is held only as `keyboard`"),
];
const PANELS: &str = "combos::Ui and setups::Ui live only in the worker's state (`s.combos`, `s.setups`, `setups` in worker::close); \
                      these are std files, the model's menu and place keeper, engine editors, queues, MacroUi and a bank's bundle";

/// A call or reference in a body, at token `at`: `(segments, method)`, and for a method call the identifier before its `.` (the
/// receiver's last field or variable: `combos` in `s.combos.open()`).
struct Call {
    at: usize,
    segments: Vec<String>,
    method: bool,
    receiver: Option<String>,
}

/// `Graph::references` with positions (same rules).
fn calls(body: &[Tok]) -> Vec<Call> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < body.len() {
        if let Tok::Ident(first) = &body[i] {
            let mut method = i > 0 && punct(body.get(i - 1), '.');
            let mut segments = vec![first.clone()];
            if method && i > 1 && ident(body.get(i - 2)) == Some("self") {
                segments.insert(0, "Self".to_owned());
                method = false;
            }
            let mut k = i + 1;
            while k + 1 < body.len() && body[k] == Tok::PathSep {
                match &body[k + 1] {
                    Tok::Ident(next) => {
                        segments.push(next.clone());
                        k += 2;
                    }
                    Tok::Punct('<') => {
                        let mut depth = 0;
                        while k + 1 < body.len() {
                            if punct(body.get(k + 1), '<') {
                                depth += 1;
                            } else if punct(body.get(k + 1), '>') {
                                depth -= 1;
                                if depth == 0 {
                                    k += 1;
                                    break;
                                }
                            }
                            k += 1;
                        }
                        k += 1;
                    }
                    _ => break,
                }
            }
            let called = punct(body.get(k), '(');
            if (segments.len() == 1 && called) || segments.len() >= 2 || (method && called) {
                let receiver = if method && i > 1 { ident(body.get(i - 2)).map(str::to_owned) } else { None };
                out.push(Call { at: i, segments, method, receiver });
            }
            i = k.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

struct Ctx<'g> {
    graph: &'g Graph,
    by_name: BTreeMap<String, Vec<usize>>,
    locks: Vec<(String, String)>,
    /// `Graph::callees` per function: the scanner's own resolution (imports, aliases), which a path call must also pass.
    callees: Vec<BTreeSet<usize>>,
}

impl<'g> Ctx<'g> {
    fn new(graph: &'g Graph, locks: Vec<(String, String)>) -> Self {
        let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, f) in graph.funcs.iter().enumerate() {
            by_name.entry(f.name.clone()).or_default().push(i);
        }
        let callees = (0..graph.funcs.len()).map(|f| graph.callees(f)).collect();
        Ctx { graph, by_name, locks, callees }
    }

    /// The callees a call can mean (`hook_locks.rs` `edges`: a path call as `Graph::callees` resolves it, then by its
    /// module qualifier; a method only when one runtime receiver method has the name).
    fn resolve(&self, from: usize, call: &Call) -> Vec<usize> {
        let f = &self.graph.funcs[from];
        let Some(last) = call.segments.last() else { return Vec::new() };
        if LOCK_METHODS.contains(&last.as_str()) && call.method {
            return Vec::new();
        }
        let Some(all) = self.by_name.get(last) else { return Vec::new() };
        if call.method {
            let methods: Vec<usize> = all.iter().copied().filter(|&c| self.graph.funcs[c].has_receiver).collect();
            if methods.len() == 1 {
                return if methods[0] != from { methods } else { Vec::new() };
            }
            // Ambiguous: the receiver's field names its module (`s.combos.press()` is `ui/combos.rs`'s, not `ui/setups.rs`'s
            // `press`; docs lane review R1 addendum 2026-11-10 00:25Z), or `RECEIVER_MODULES` does. No match keeps it unfollowed,
            // which `every_unfollowed_ambiguous_call_that_reaches_a_lock_is_classified` checks.
            let Some(receiver) = call.receiver.as_deref() else { return Vec::new() };
            let module = RECEIVER_MODULES.iter().find(|(r, _)| *r == receiver).map_or(receiver, |(_, m)| *m);
            return methods.into_iter().filter(|&c| c != from && self.graph.funcs[c].module == module).collect();
        }
        let qualifier = call.segments.len().checked_sub(2).map(|k| call.segments[k].as_str());
        all.iter()
            .copied()
            .filter(|&c| c != from && self.callees[from].contains(&c))
            .filter(|&c| {
                let callee = &self.graph.funcs[c];
                match qualifier {
                    None => {
                        let local = all.iter().any(|&o| self.graph.funcs[o].file == f.file);
                        !local || callee.file == f.file
                    }
                    Some("self" | "Self" | "super" | "crate") => callee.file == f.file,
                    // A `#[path]` module declared under another name (`body_bank` is `native_body_bank.rs`).
                    Some(q) if self.graph.declared_module(&f.file, q).is_some() => self.graph.declared_module(&f.file, q) == Some(callee.module.as_str()),
                    Some(q) if q == callee.module => true,
                    Some(q) => !all.iter().any(|&o| self.graph.funcs[o].module == q),
                }
            })
            .collect()
    }

    /// The lock an identifier at `i` of function `f` names, if it is one of the static locks.
    fn lock_at(&self, f: usize, i: usize) -> Option<usize> {
        let func = &self.graph.funcs[f];
        let body = &func.body;
        let name = ident(body.get(i))?;
        let qualified = i >= 2 && body[i - 1] == Tok::PathSep;
        self.locks.iter().position(|(file, lock)| {
            if lock != name {
                return false;
            }
            let module = Path::new(file).file_stem().unwrap().to_string_lossy().into_owned();
            if qualified {
                let q = ident(body.get(i - 2)).unwrap_or("");
                q == module || ALIASES.iter().any(|(af, m)| af == file && *m == q)
            } else {
                func.file == *file
            }
        })
    }
}

/// One blocking acquisition: the lock, where, and the token range its guard is held over.
#[derive(Clone, Debug)]
struct Take {
    lock: usize,
    at: usize,
    end: usize,
}

/// Start of the statement holding token `k` (just after a `;`, `{`, `}` or `=>` at its own depth).
fn statement_start(body: &[Tok], k: usize) -> usize {
    let mut depth = 0i32;
    let mut i = k;
    while i > 0 {
        let t = &body[i - 1];
        match t {
            Tok::Punct(')' | ']') => depth += 1,
            Tok::Punct('(' | '[') => depth -= 1,
            Tok::Punct('}') if depth <= 0 => return i,
            Tok::Punct('}') => depth += 1,
            Tok::Punct('{') if depth <= 0 => return i,
            Tok::Punct('{') => depth -= 1,
            Tok::Punct(';') if depth <= 0 => return i,
            Tok::Punct('>') if depth <= 0 && i >= 2 && punct(body.get(i - 2), '=') => return i,
            _ => {}
        }
        i -= 1;
    }
    0
}

/// End of the statement starting at `s` that holds token `k`: a `;` at its depth, a `,` ending a match arm, or the
/// close of its enclosing block. A `{…}` the statement opens (a `match`/`if let` it heads) is part of it.
fn statement_end(body: &[Tok], s: usize, k: usize, arm: bool) -> usize {
    let mut depth = 0i32;
    for (i, t) in body.iter().enumerate().skip(s) {
        match t {
            Tok::Punct('(' | '[' | '{') => depth += 1,
            Tok::Punct(')' | ']') => depth -= 1,
            Tok::Punct('}') => {
                depth -= 1;
                if depth < 0 && i > k {
                    return i;
                }
                // A block statement (`if let … {…}`, `match … {…}`) ends with its block unless `else` or a method
                // chain continues it.
                if depth == 0 && i > k && !matches!(body.get(i + 1), Some(Tok::Ident(e)) if e == "else")
                    && !punct(body.get(i + 1), '.') && !punct(body.get(i + 1), '?')
                {
                    return i;
                }
            }
            Tok::Punct(';') if depth <= 0 && i > k => return i,
            Tok::Punct(',') if arm && depth <= 0 && i > k => return i,
            _ => {}
        }
    }
    body.len()
}

/// A take inside a closure written without braces (`.filter(|i| LOCK.lock().f(*i))`): its temporaries end with the
/// closure's expression (the `)` or `,` that closes it).
fn closure_end(body: &[Tok], s: usize, at: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut j = at;
    let close_bar = loop {
        if j <= s {
            return None;
        }
        j -= 1;
        match &body[j] {
            Tok::Punct(')' | ']' | '}') => depth += 1,
            Tok::Punct('(' | '[' | '{') => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
            }
            Tok::Punct('|') if depth == 0 => break j,
            _ => {}
        }
    };
    let open_bar = if close_bar > 0 && punct(body.get(close_bar - 1), '|') {
        close_bar - 1
    } else {
        (s..close_bar).rev().find(|&k| punct(body.get(k), '|'))?
    };
    let before = open_bar.checked_sub(1).and_then(|k| body.get(k));
    let opens = punct(before, '(') || punct(before, ',') || punct(before, '=') || ident(before) == Some("move");
    if !opens {
        return None;
    }
    let mut depth = 0i32;
    (at..body.len()).find(|&i| {
        match &body[i] {
            Tok::Punct('(' | '[' | '{') => depth += 1,
            Tok::Punct(')' | ']' | '}') => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            Tok::Punct(',' | ';') if depth == 0 => return true,
            _ => {}
        }
        false
    })
}

/// A plain `if`/`while` condition (not `if let`/`while let`) drops its temporaries before the block it guards: the `{`
/// that ends the condition of the statement starting at `s`, when the take at `at` is inside it.
fn condition_end(body: &[Tok], s: usize, at: usize) -> Option<usize> {
    let head = if ident(body.get(s)) == Some("else") { s + 1 } else { s };
    if !matches!(ident(body.get(head)), Some("if" | "while")) || ident(body.get(head + 1)) == Some("let") {
        return None;
    }
    let mut depth = 0i32;
    let open = (head..body.len()).find(|&j| {
        match &body[j] {
            Tok::Punct('(' | '[') => depth += 1,
            Tok::Punct(')' | ']') => depth -= 1,
            Tok::Punct('{') if depth == 0 => return true,
            _ => {}
        }
        false
    })?;
    (at < open).then_some(open)
}

/// The close of the block that holds token `k`.
fn block_end(body: &[Tok], k: usize) -> usize {
    let mut depth = 0i32;
    for (i, t) in body.iter().enumerate().skip(k) {
        match t {
            Tok::Punct('{') => depth += 1,
            Tok::Punct('}') => {
                depth -= 1;
                if depth < 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    body.len()
}

/// The locks a function hands out by reference (`state_for(role)` returns `&STATES[i]`): the static locks of its
/// file its body names after a `&`.
fn accessor_locks(ctx: &Ctx, g: usize) -> Vec<usize> {
    let func = &ctx.graph.funcs[g];
    let body = &func.body;
    (1..body.len()).filter(|&i| punct(body.get(i - 1), '&')).filter_map(|i| ctx.lock_at(g, i)).collect::<BTreeSet<_>>().into_iter().collect()
}

/// `accessor(…).lock()`: the call before `.lock(` at `dot` (the `.`), resolved in the same file, and the locks it hands out.
fn accessor_take(ctx: &Ctx, f: usize, dot: usize) -> Vec<usize> {
    let body = &ctx.graph.funcs[f].body;
    if dot == 0 || !punct(body.get(dot - 1), ')') {
        return Vec::new();
    }
    let mut depth = 0;
    let mut open = dot - 1;
    loop {
        match &body[open] {
            Tok::Punct(')') => depth += 1,
            Tok::Punct('(') => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        if open == 0 {
            return Vec::new();
        }
        open -= 1;
    }
    let Some(name) = open.checked_sub(1).and_then(|k| ident(body.get(k))) else { return Vec::new() };
    let file = &ctx.graph.funcs[f].file;
    ctx.by_name
        .get(name)
        .into_iter()
        .flatten()
        .filter(|&&g| ctx.graph.funcs[g].file == *file)
        .flat_map(|&g| accessor_locks(ctx, g))
        .collect()
}

/// The token that ends the name at `i`: the `]` of an index right after it (`NAME[role]`), else `i` itself.
fn index_end(body: &[Tok], i: usize) -> usize {
    if !punct(body.get(i + 1), '[') {
        return i;
    }
    let mut depth = 0;
    for (k, t) in body.iter().enumerate().skip(i + 1) {
        match t {
            Tok::Punct('[') => depth += 1,
            Tok::Punct(']') => {
                depth -= 1;
                if depth == 0 {
                    return k;
                }
            }
            _ => {}
        }
    }
    i
}

fn takes(ctx: &Ctx, f: usize) -> Vec<Take> {
    let body = &ctx.graph.funcs[f].body;
    let mut out = Vec::new();
    // `accessor(…).lock()` (character_roster's `state_for(role).lock()`): a take of each lock the accessor hands out,
    // held over its statement.
    for i in 1..body.len() {
        if punct(body.get(i), '.') && ident(body.get(i + 1)).is_some_and(|m| LOCK_METHODS.contains(&m)) && punct(body.get(i + 2), '(') {
            for lock in accessor_take(ctx, f, i) {
                let s = statement_start(body, i);
                let arm = s >= 2 && punct(body.get(s - 1), '>') && punct(body.get(s - 2), '=');
                let lets = ident(body.get(s)) == Some("let");
                let bound = lets && punct(body.get(i + 4), ';');
                let name_at = if ident(body.get(s + 1)) == Some("mut") { s + 2 } else { s + 1 };
                let end = if let Some(close) = closure_end(body, s, i) {
                    close
                } else if let Some(open) = condition_end(body, s, i) {
                    open
                } else if bound {
                    let block = block_end(body, i);
                    let name = ident(body.get(name_at)).unwrap_or("");
                    (i..block)
                        .find(|&j| ident(body.get(j)) == Some("drop") && punct(body.get(j + 1), '(') && ident(body.get(j + 2)) == Some(name))
                        .unwrap_or(block)
                } else {
                    statement_end(body, s, i, arm)
                };
                out.push(Take { lock, at: i, end });
            }
        }
    }
    for i in 0..body.len() {
        let Some(lock) = ctx.lock_at(f, i) else { continue };
        // `NAME.lock()`, or one lock of a static array, `NAME[role].lock()` (native_body_bank's `BANKS`).
        let dot = index_end(body, i) + 1;
        let method = punct(body.get(dot), '.')
            && ident(body.get(dot + 1)).is_some_and(|m| LOCK_METHODS.contains(&m))
            && punct(body.get(dot + 2), '(');
        let head = if i >= 2 && body[i - 1] == Tok::PathSep { i - 2 } else { i };
        let helper = head >= 3
            && punct(body.get(head - 1), '&')
            && punct(body.get(head - 2), '(')
            && ident(body.get(head - 3)).is_some_and(|h| GUARD_HELPERS.contains(&h));
        if !method && !helper {
            continue;
        }
        let at = if helper { head - 3 } else { head };
        let s = statement_start(body, at);
        let arm = s >= 2 && punct(body.get(s - 1), '>') && punct(body.get(s - 2), '=');
        // `let [mut] name = NAME.lock()[.unwrap()|.expect(..)];` binds the guard.
        let call_close = if helper { i + 1 } else { dot + 3 };
        let mut after = call_close + 1;
        if punct(body.get(after), '.') && matches!(ident(body.get(after + 1)), Some("unwrap" | "expect")) {
            let open = after + 2;
            let mut depth = 0;
            let mut j = open;
            while j < body.len() {
                if punct(body.get(j), '(') {
                    depth += 1;
                } else if punct(body.get(j), ')') {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                j += 1;
            }
            after = j + 1;
        }
        let lets = ident(body.get(s)) == Some("let");
        let mut name_at = s + 1;
        if ident(body.get(name_at)) == Some("mut") {
            name_at += 1;
        }
        let simple = lets && ident(body.get(name_at)).is_some() && punct(body.get(name_at + 1), '=') && name_at + 2 == at;
        let bound_name = (simple && punct(body.get(after), ';')).then(|| ident(body.get(name_at)).unwrap().to_owned());
        // `let Ok(mut g) = NAME.lock() else {…};` binds it too.
        let else_bound = lets && ident(body.get(after)) == Some("else");
        let end = if let Some(close) = closure_end(body, s, at) {
            close
        } else if let Some(open) = condition_end(body, s, at) {
            open
        } else if let Some(name) = bound_name {
            let block = block_end(body, at);
            let dropped = (at..block).find(|&j| {
                ident(body.get(j)) == Some("drop") && punct(body.get(j + 1), '(') && ident(body.get(j + 2)) == Some(name.as_str())
            });
            dropped.unwrap_or(block)
        } else if else_bound {
            block_end(body, at)
        } else {
            statement_end(body, s, at, arm)
        };
        out.push(Take { lock, at, end });
    }
    out
}

#[derive(Default)]
struct Scan {
    /// Per function: its acquisitions.
    takes: Vec<Vec<Take>>,
    /// Per function: every lock it can take, itself or through calls.
    reach: Vec<BTreeSet<usize>>,
    /// Per function: the roles (thread entries) that reach it.
    roles: Vec<BTreeSet<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Edge {
    held: usize,
    taken: usize,
    func: String,
    via: String,
}

/// The plugin threads' entries: `(key, thread, module path)`. A `THREADS` row's symbol gives the module path (`macros::diag` for
/// `macros::diag::start::{closure#0}`); the publishers have none.
fn plugin_entries() -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> =
        PUBLISHING_ENTRIES.iter().map(|e| ((*e).to_owned(), format!("publish({e})"), String::new())).collect();
    for row in thread_stacks::THREADS.iter() {
        let path = row.symbol.split("::{").next().unwrap_or(row.symbol);
        let segments: Vec<&str> = path.split("::").collect();
        let key = segments[segments.len().saturating_sub(2)..].join("::");
        let module = segments[..segments.len().saturating_sub(1)].join("::");
        let name = row.name.to_owned();
        if !out.iter().any(|(k, _, _)| *k == key) {
            out.push((key, name, module));
        }
    }
    out
}

/// The functions an entry names: with a module path, the functions of that name defined on it (`practice_trace::writer` is
/// `practice_trace.rs`'s `mod enabled`'s; `macros::diag::start` is not `threads.rs`'s own `mod diag`: docs lane review R2, finding
/// 5); without one, the key's matches.
fn entry_starts(graph: &Graph, key: &str, module: &str) -> Vec<usize> {
    if module.is_empty() {
        return graph.find(key);
    }
    let name = key.rsplit("::").next().unwrap_or(key);
    (0..graph.funcs.len()).filter(|&f| graph.funcs[f].name == name && on_module_path(&graph.funcs[f], module)).collect()
}

/// `func` is defined on `module`: its file's path, with or without its inline `mod`, and `module` agree segment by segment, one a
/// suffix of the other (the rows name `states`, the file is `features/states.rs`).
fn on_module_path(func: &callgraph::Func, module: &str) -> bool {
    let file: Vec<&str> = func.file.trim_end_matches(".rs").trim_end_matches("/mod").split('/').collect();
    let mut inner = file.clone();
    if inner.last() != Some(&func.module.as_str()) {
        inner.push(func.module.as_str());
    }
    let want: Vec<&str> = module.split("::").collect();
    [file, inner].iter().any(|have| have.ends_with(&want) || want.ends_with(have))
}

/// Each plugin thread's entry functions (`plugin_entries`, resolved by `entry_starts`): empty when one resolves to nothing
/// (`every_plugin_thread_entry_resolves`).
fn plugin_entry_funcs(graph: &Graph) -> Vec<(Vec<usize>, String)> {
    plugin_entries().into_iter().map(|(key, name, module)| (entry_starts(graph, &key, &module), name)).collect()
}

fn scan(graph: &Graph) -> (Ctx<'_>, Scan) {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let ctx = Ctx::new(graph, static_locks(&runtime));
    let n = graph.funcs.len();
    let mut s = Scan { takes: (0..n).map(|f| takes(&ctx, f)).collect(), ..Scan::default() };
    let callees: Vec<BTreeSet<usize>> =
        (0..n).map(|f| calls(&graph.funcs[f].body).iter().flat_map(|c| ctx.resolve(f, c)).collect()).collect();
    s.reach = s.takes.iter().map(|t| t.iter().map(|t| t.lock).collect()).collect();
    loop {
        let mut changed = false;
        for (f, out) in callees.iter().enumerate() {
            let add: BTreeSet<usize> = out.iter().flat_map(|&c| s.reach[c].iter().copied()).collect();
            let before = s.reach[f].len();
            s.reach[f].extend(add);
            changed |= s.reach[f].len() != before;
        }
        if !changed {
            break;
        }
    }
    s.roles = vec![BTreeSet::new(); n];
    let mut entries: Vec<(Vec<usize>, String)> = Vec::new();
    for (i, f) in graph.funcs.iter().enumerate() {
        if f.attrs.contains("skyline::hook") {
            entries.push((vec![i], "game-hook".to_owned()));
        }
    }
    for (key, name, module) in plugin_entries() {
        let found = entry_starts(graph, &key, &module);
        if !found.is_empty() {
            entries.push((found, name));
        }
    }
    for (starts, role) in entries {
        let mut stack = starts;
        let mut seen = BTreeSet::new();
        while let Some(f) = stack.pop() {
            if !seen.insert(f) {
                continue;
            }
            s.roles[f].insert(role.clone());
            stack.extend(callees[f].iter().copied());
        }
    }
    (ctx, s)
}

fn edges(ctx: &Ctx, s: &Scan) -> (BTreeSet<Edge>, Vec<String>) {
    let mut out = BTreeSet::new();
    let mut waits = Vec::new();
    for (f, takes) in s.takes.iter().enumerate() {
        let func = &ctx.graph.funcs[f];
        let body_calls = calls(&func.body);
        for t in takes {
            for other in takes.iter().filter(|o| o.at > t.at && o.at < t.end) {
                out.insert(Edge { held: t.lock, taken: other.lock, func: func.key(), via: "direct".to_owned() });
            }
            for c in body_calls.iter().filter(|c| c.at > t.at + 1 && c.at < t.end) {
                let last = c.segments.last().map(String::as_str).unwrap_or("");
                if WAITS.contains(&last) {
                    waits.push(format!("{} holds {} across {}", func.key(), lock_name(ctx, t.lock), c.segments.join("::")));
                }
                for callee in ctx.resolve(f, c) {
                    let via = ctx.graph.funcs[callee].key();
                    if NOT_AN_EDGE.iter().any(|(at, v, _)| *at == func.key() && *v == via) {
                        continue;
                    }
                    for &b in &s.reach[callee] {
                        out.insert(Edge { held: t.lock, taken: b, func: func.key(), via: via.clone() });
                    }
                }
            }
        }
    }
    (out, waits)
}

fn lock_name(ctx: &Ctx, lock: usize) -> String {
    let (file, name) = &ctx.locks[lock];
    format!("{file}::{name}")
}

/// Cycles of the order graph between distinct locks (each pair of locks with a path both ways, by Tarjan's SCCs).
fn cycles(edges: &BTreeSet<Edge>, n: usize) -> Vec<Vec<usize>> {
    let mut adj = vec![BTreeSet::new(); n];
    for e in edges.iter().filter(|e| e.held != e.taken) {
        adj[e.held].insert(e.taken);
    }
    struct T<'a> {
        adj: &'a [BTreeSet<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        out: Vec<Vec<usize>>,
    }
    fn visit(t: &mut T, v: usize) {
        t.index[v] = Some(t.next);
        t.low[v] = t.next;
        t.next += 1;
        t.stack.push(v);
        t.on[v] = true;
        for w in t.adj[v].clone() {
            if t.index[w].is_none() {
                visit(t, w);
                t.low[v] = t.low[v].min(t.low[w]);
            } else if t.on[w] {
                t.low[v] = t.low[v].min(t.index[w].unwrap());
            }
        }
        if Some(t.low[v]) == t.index[v] {
            let mut scc = Vec::new();
            while let Some(w) = t.stack.pop() {
                t.on[w] = false;
                scc.push(w);
                if w == v {
                    break;
                }
            }
            if scc.len() > 1 {
                scc.sort();
                t.out.push(scc);
            }
        }
    }
    let mut t = T { adj: &adj, index: vec![None; n], low: vec![0; n], on: vec![false; n], stack: Vec::new(), next: 0, out: Vec::new() };
    for v in 0..n {
        if t.index[v].is_none() {
            visit(&mut t, v);
        }
    }
    t.out
}

fn roles_of(ctx: &Ctx, s: &Scan, key: &str) -> BTreeSet<String> {
    ctx.graph.funcs.iter().enumerate().filter(|(_, f)| f.key() == key).flat_map(|(i, _)| s.roles[i].iter().cloned()).collect()
}

#[test]
fn report_the_lock_table_and_the_order_edges() {
    let graph = runtime_graph();
    let (ctx, s) = scan(&graph);
    // The lock table: each lock, the roles that take it (and from which functions).
    let mut table: BTreeMap<usize, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    for (f, takes) in s.takes.iter().enumerate() {
        for t in takes {
            for role in &s.roles[f] {
                table.entry(t.lock).or_default().entry(role.clone()).or_default().insert(graph.funcs[f].key());
            }
        }
    }
    println!("LOCKS {} static locks, {} taken somewhere", ctx.locks.len(), table.len());
    for (lock, roles) in &table {
        let shared = roles.contains_key("game-hook") && roles.len() > 1;
        println!("LOCK {} shared_with_game={shared}", lock_name(&ctx, *lock));
        for (role, funcs) in roles {
            println!("  {role}: {}", funcs.iter().cloned().collect::<Vec<_>>().join(", "));
        }
    }
    let (edges, waits) = edges(&ctx, &s);
    for e in &edges {
        println!(
            "EDGE {} -> {} in {} via {} roles={:?}",
            lock_name(&ctx, e.held),
            lock_name(&ctx, e.taken),
            e.func,
            e.via,
            roles_of(&ctx, &s, &e.func)
        );
    }
    for w in &waits {
        println!("WAIT {w}");
    }
    // The menu-close window (both freezes: `[Ui] press B`, then nothing): every lock the worker's close path can take,
    // and the game-thread functions that take the same lock.
    for entry in WINDOW {
        let starts = graph.find(entry);
        let reached: BTreeSet<usize> = starts.iter().flat_map(|&f| s.reach[f].iter().copied()).collect();
        println!("WINDOW {entry}: {} locks", reached.len());
        for lock in reached {
            let game: Vec<String> = table.get(&lock).and_then(|r| r.get("game-hook")).map(|f| f.iter().cloned().collect()).unwrap_or_default();
            println!("  WINDOW-LOCK {} game_takers={}", lock_name(&ctx, lock), if game.is_empty() { "-".to_owned() } else { game.join(", ") });
        }
    }
}

/// The menu close on the worker: the press that starts it, its two paths (with and without a Mii catalog), the publish,
/// the typed write-back, and the settings writer the close wakes.
const WINDOW: &[&str] =
    &["worker::begin_close", "worker::retry_close", "worker::close", "published::publish", "menu::apply_kept", "writer::run"];

#[test]
fn no_two_locks_are_taken_in_both_orders_and_no_lock_is_retaken_while_held() {
    let graph = runtime_graph();
    let (ctx, s) = scan(&graph);
    let (edges, _) = edges(&ctx, &s);
    let mut problems = Vec::new();
    for scc in cycles(&edges, ctx.locks.len()) {
        let names: Vec<String> = scc.iter().map(|&l| lock_name(&ctx, l)).collect();
        let inside: Vec<String> = edges
            .iter()
            .filter(|e| e.held != e.taken && scc.contains(&e.held) && scc.contains(&e.taken))
            .map(|e| format!("{} -> {} in {} via {}", lock_name(&ctx, e.held), lock_name(&ctx, e.taken), e.func, e.via))
            .collect();
        problems.push(format!("cycle {names:?}:\n    {}", inside.join("\n    ")));
    }
    for e in edges.iter().filter(|e| e.held == e.taken) {
        problems.push(format!("{} retaken while held in {} via {}", lock_name(&ctx, e.held), e.func, e.via));
    }
    assert!(problems.is_empty(), "lock-order problems:\n{}", problems.join("\n"));
}

// ---- rc.6 QP freeze, the class (primary 2026-11-09 19:07Z) ----------------------------------------------------------------
//
// `rc6-qpf3` (Eden's GDB stub on a frozen b): the game's MainThread busy-joined the UI task group on core 0; the group's task on
// TaskWorker1 waited in our hook for `compat::menu::EDITED`; `tr-menu`, the menu worker, held it on core 0 and never ran again.
// Any lock a game-thread hook takes with a blocking call, and a plugin thread can hold, is the same hazard: the game can starve
// the holder while it waits on the hook. A plugin thread holds a lock it takes blocking or with a successful `try_*`; a game-side
// `try_*` never waits and is not a hazard.

const TRY_METHODS: &[&str] = &["try_lock", "try_read", "try_write"];

/// The carried code's typed statics (`hook_locks.rs` `TYPED_STATIC_CALLS`): a helper handed `&STATIC` runs the static's trait
/// impl, which the text does not show. `read`/`setting` and `lock_write` run `Menu::read_value` (the guard is built from it), the
/// guard's drop and `assign_value` run `store`. A function that branches on `on_plugin_thread()` (`callgraph::thread_dispatch`)
/// is followed into the walking thread's branch only.
const TYPED_STATIC_CALLS: &[(&str, &str, &str)] = &[
    ("MENU", "read", "menu::read_value"),
    ("MENU", "setting", "menu::read_value"),
    ("MENU", "lock_write", "menu::read_value"),
    ("MENU", "lock_write", "menu::store"),
    ("MENU", "assign", "menu::store"),
];

/// Shared locks the code shares on purpose, each with the reason it is not the hazard (read in the source). Empty: every row
/// needs the reason a starved plugin holder cannot stall a game hook that waits for it.
const SHARED_ON_PURPOSE: &[(&str, &str)] = &[];

/// Game entry points installed without `#[skyline::hook]`, walked as game hooks (docs lane review R1, 2026-11-09 23:23Z, finding 2):
/// a vtable slot (`cpu::on_frame`, original `main+0x614630`), `A64HookFunction` replacements (`cpu_common`), inline-hook and
/// trampoline targets (`native_service`, `native_body_bank::prepare_*`, `native_unlink_*`, `mii_ordinary_gate`, `seam_drill`),
/// the game's on-init callback (`reset_hub::on_init`), Skyline's NRO-load hooks, and the exported trace control.
/// `every_function_handed_to_the_game_is_listed` keeps this complete.
const GAME_ENTRIES: &[&str] = &[
    "cpu::on_frame",
    "cpu_common::di_hook",
    "cpu_common::change_status_hook",
    "cpu_common::sub_guard_cont_hook",
    "cpu_common::on_nro_load",
    "roster::on_nro_load",
    "reset_hub::on_init",
    "native_service::training_mod_native_service_dispatch",
    "mii_ordinary_gate::mii_ordinary_gate_decide",
    "native_body_bank::prepare_call",
    "native_body_bank::prepare_clone_into",
    "native_body_bank::prepare_clone_heap",
    "native_unlink_install::leave",
    "native_unlink_install::recursive_last_guard",
    "native_unlink_install::sister_first_guard",
    "native_unlink_install::sister_last_guard",
    "native_unlink_install::header_first_guard",
    "native_unlink_install::header_last_guard",
    "native_unlink_install::vector_first_guard",
    "native_unlink_install::vector_last_guard",
    "native_unlink_probe::born",
    "native_unlink_probe::cleared",
    "native_unlink_probe::sister_cleared",
    "native_unlink_probe::header_cleared",
    "native_unlink_probe::vector_cleared",
    "native_unlink_probe::enter",
    "native_unlink_probe::header_enter",
    "native_unlink_probe::header_leave",
    "practice_trace::training_modpack_trace_control_v1",
    "enabled::list_step",
    "enabled::list_callback_enter",
    "seam_drill::inline_0",
    "seam_drill::inline_1",
    "seam_drill::inline_2",
    "seam_drill::inline_3",
    "probe::mounted",
];

/// Functions whose address is handed out (`extern "C"`, `as *const ()`) that are not game entry points, and why.
const NOT_GAME_ENTRIES: &[(&str, &str)] = &[
    ("imp::detached", "a plugin thread's start (runtime::threads); its thread is walked from model::thread_stacks"),
    ("imp::joined", "a plugin thread's start (runtime::threads); its thread is walked from model::thread_stacks"),
    ("inline_keyboard::changed", KEYBOARD_CALLBACK),
    ("inline_keyboard::moved", KEYBOARD_CALLBACK),
    ("inline_keyboard::entered", KEYBOARD_CALLBACK),
    ("inline_keyboard::cancelled", KEYBOARD_CALLBACK),
    ("inline_keyboard::initialized", KEYBOARD_CALLBACK),
    ("inline_keyboard::finished", KEYBOARD_CALLBACK),
    ("combo::tr_seq_step_v1", "a trace-point anchor our own code calls (its callers are walked)"),
    ("combo::tr_seq_plan_v1", "a trace-point anchor our own code calls (its callers are walked)"),
    ("combo::tr_seq_saved_v1", "a trace-point anchor our own code calls (its callers are walked)"),
    ("combo::tr_combo_link_v1", "a trace-point anchor our own code calls (its callers are walked)"),
    ("combo::tr_combo_attempt_v1", "a trace-point anchor our own code calls (its callers are walked)"),
    ("combo::tr_combo_saved_v1", "a trace-point anchor our own code calls (its callers are walked)"),
    ("combo::tr_combo_listed_v1", "a trace-point anchor our own code calls (its callers are walked)"),
    ("combo::tr_combo_fighter_v1", "a trace-point anchor our own code calls (its callers are walked)"),
    ("notices::tr_intro_posted_v1", "a trace-point anchor our own code calls (its callers are walked)"),
    ("worker::tr_cycle_badges_v1", "a trace-point anchor our own code calls (its callers are walked)"),
];

/// Listed entries the call graph does not contain, and why (docs lane review R2, finding 5: a key that resolves to nothing is not
/// skipped silently). Every other `GAME_ENTRIES` key and every plugin entry resolves (`every_listed_entry_resolves`).
const UNSCANNED_ENTRIES: &[(&str, &str)] = &[
    ("probe::mounted", "ui/probe.rs is a `#[cfg(feature = \"diag\")]` module: Eden builds only, not scanned (R2 finding 6)"),
    ("ui::probe::install", "the tr-probe thread: ui/probe.rs is a `#[cfg(feature = \"diag\")]` module, not scanned"),
    ("macros::diag::start", "the tr-macro-diag thread: macros/diag.rs is a `#[cfg(feature = \"diag\")]` module, not scanned"),
];

/// Callback registrations the text cannot follow: `(registrar's module, registrar, the function that calls what is registered)`.
/// `hud::register(draw)` puts a draw into the table `hud::run` calls on the UI pass (`anchors::ui_pass_calculate`, the game's task
/// workers); `diag_log::set_tick(step)` puts a step into the table the heartbeat thread runs. The edges come from the registration
/// calls themselves, so a new draw or tick is followed without a list.
const REGISTRARS: &[(&str, &str, &str)] =
    &[("hud", "register", "hud::run"), ("diag_log", "set_tick", "diag_log::start_drain_thread")];

/// Calls from model code back into the runtime (the model is not scanned): `capabilities.rs` declares the same ones
/// (`the_callback_edges_cover_the_capability_scan`).
const MODEL_CALLBACKS: &[(&str, &str)] = &[
    ("hitbox::frame", "hitbox::sphere_alive"),
    ("hitbox::frame", "hitbox::sphere_kill_own"),
    ("hitbox::frame", "hitbox::sphere_spawn"),
    ("hitbox::hitbox_rebound_hook", "hitbox::sphere_alive"),
    ("hitbox::hitbox_rebound_hook", "hitbox::sphere_kill_own"),
    ("hitbox::frame", "hitbox::active"),
    ("hitbox::frame", "hitbox::data"),
    ("hitbox::frame", "hitbox::read_u64"),
    ("hitbox::frame", "hitbox::read_f32"),
];

const KEYBOARD_CALLBACK: &str = "the inline keyboard's SDK callback: the SDK runs it synchronously on the thread that calls into \
                                 the keyboard (inline_keyboard.rs), so it is walked from there (KEYBOARD_CALLBACKS), not as an entry";

/// The inline keyboard's SDK callbacks, run synchronously inside the SDK calls of `Keyboard::open` (Appear), `poll` (Calc) and
/// `close` (Finalize) on the keyboard's owner thread: `tr-menu` for the menu's keyboard (`native_keyboard`), a game hook for the Mii
/// editor's (`name_keyboard`).
const KEYBOARD_CALLBACKS: &[&str] = &[
    "inline_keyboard::changed",
    "inline_keyboard::moved",
    "inline_keyboard::entered",
    "inline_keyboard::cancelled",
    "inline_keyboard::initialized",
    "inline_keyboard::finished",
];

/// `(caller, callee)` for every callback the text cannot follow: the registrations' (`REGISTRARS`), `MODEL_CALLBACKS` and the
/// keyboard's (`KEYBOARD_CALLBACKS`).
fn callback_edges(graph: &Graph) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (module, registrar, runner) in REGISTRARS {
        let runners = graph.find(runner);
        for f in 0..graph.funcs.len() {
            let body = &graph.funcs[f].body;
            for i in 2..body.len() {
                if ident(body.get(i)) != Some(registrar) || body[i - 1] != Tok::PathSep || ident(body.get(i - 2)) != Some(module) {
                    continue;
                }
                if !punct(body.get(i + 1), '(') {
                    continue;
                }
                // The argument: a path up to `)` or `,`; its last two segments name the registered function.
                let mut k = i + 2;
                let mut segments = Vec::new();
                while let Some(t) = body.get(k) {
                    match t {
                        Tok::Ident(s) => segments.push(s.clone()),
                        Tok::PathSep => {}
                        _ => break,
                    }
                    k += 1;
                }
                let key = segments[segments.len().saturating_sub(2)..].join("::");
                let targets = graph.find(&key);
                assert_eq!(targets.len(), 1, "{module}::{registrar}({key}) in {}: {} functions", graph.funcs[f].key(), targets.len());
                out.extend(runners.iter().map(|&r| (r, targets[0])));
            }
        }
    }
    for (from, to) in MODEL_CALLBACKS {
        for a in graph.find(from) {
            out.extend(graph.find(to).into_iter().map(|b| (a, b)));
        }
    }
    for from in ["inline_keyboard::open", "inline_keyboard::poll", "inline_keyboard::close"] {
        for a in graph.find(from) {
            out.extend(KEYBOARD_CALLBACKS.iter().flat_map(|to| graph.find(to)).map(|b| (a, b)));
        }
    }
    out
}

/// The runs that measured the hooks' threads (primary 2026-11-09 19:44Z/20:32Z, OPERATOR "measure first, fix exactly"; playbook
/// CHECKLIST M row, main 87b3de0): hot-slots' two trace points per hook on its game address, a `thread == main` counter and a
/// `thread != main` record point, on `b04856b2` diag (`artifacts/phase23/hook-threads/TABLE.md`).
/// The runs of hot-slots' first entry-point session: the mii plan (CPU and P1 cycles, CPU switches, Mii groups, the pause) and
/// four native L+R+A resets, on `72344fd9`'s diag build (`9b1d2d99`).
const ENTRY_RUNS: &str = "final-yoga-ht-mii-reset-72344fd9";
/// The runs that reached the Mii editor hooks (hot-slots, TABLE.md 2026-11-10 01:57Z): its combined boot on the fix build
/// `ead60d8c` (`72344fd9` + this branch: Training, the CPU to Kirby, the Mii editor rows with the name keyboard) and the two LDN
/// pair sessions, whose guest opens the editor.
const EDITOR_RUNS: &str =
    "final-yoga-ht-fixcheck-ead60d8c-r2 + final-yoga-ht-pair-72344fd9 + final-yoga-ht-pair-b04856b2-r2 (a and b, LDN pairs: the guest editor)";
const MEASURED_RUNS: &str = "final-yoga-ht-mii-b04856b2 + final-yoga-ht-kr-b04856b2 + final-yoga-ht-pair-b04856b2-r2 (a and b, an LDN \
                             pair: the waiting rooms and a match) + final-yoga-ht-reset-b04856b2 (four native resets)";

/// Game hooks measured on MainThread only: `(hook, calls, calls on MainThread, runs)`, the runs' exact counters together. A hook that runs
/// on MainThread cannot deadlock this way: while it waits, MainThread is not busy-joining a task group, so the core it leaves free
/// lets the starved holder run (`rc6-qpf3`'s cycle needs the waiter on a task worker). A lock leaves the failing set only when every
/// hook that waits on it is here; a hook seen on a worker thread, or never reached, never is.
const MEASURED_ON_MAIN_THREAD: &[(&str, u64, u64, &str)] = &[
    ("input_delay::caller_a_note", 73_937, 73_937, MEASURED_RUNS),
    ("input_delay::caller_a_apply", 358_224, 358_224, MEASURED_RUNS),
    ("input_delay::caller_b_note", 914, 914, MEASURED_RUNS),
    ("input_delay::caller_b_apply", 914, 914, MEASURED_RUNS),
    ("character_lifecycle::destruction_trace", 30, 30, MEASURED_RUNS),
    ("character_lifecycle::refresh_entry_portrait", 8, 8, MEASURED_RUNS),
    ("character_lifecycle::refresh_ordinal_portrait", 32, 32, MEASURED_RUNS),
    ("states::state_kirby_opff_hook", 3_001, 3_001, MEASURED_RUNS),
    ("native_source_capacity::owner_cleanup_complete", 30, 30, MEASURED_RUNS),
    ("native_source_capacity::listener_removed", 30, 30, MEASURED_RUNS),
    ("native_source_capacity::actor_cleanup", 97, 97, MEASURED_RUNS),
    ("native_source_capacity::pause_state_exit", 2, 2, MEASURED_RUNS),
    ("mii_hud_portrait::pipeline_before_tick", 93_041, 93_041, MEASURED_RUNS),
    ("mii_hud_portrait::pipeline_after_tick", 93_041, 93_041, MEASURED_RUNS),
    // hot-slots, the R1 entry points (TABLE.md; inbox 2026-11-10 00:31Z).
    ("cpu::on_frame", 45_743, 45_743, ENTRY_RUNS),
    ("native_service::training_mod_native_service_dispatch", 92_724, 92_724, ENTRY_RUNS),
    ("mii_ordinary_gate::mii_ordinary_gate_decide", 26, 26, ENTRY_RUNS),
    // hot-slots' combined boot (primary 2026-11-10 01:16Z): the Mii editor hooks that wait on inline_keyboard SIGNALS.
    ("mii_editor_bridge::edit_frame", 6_067, 6_067, EDITOR_RUNS),
    ("mii_editor_bridge::editor_parent", 7_275, 7_275, EDITOR_RUNS),
    ("mii_editor_bridge::editor_release", 9, 9, EDITOR_RUNS),
    ("name_keyboard::name_frame", 843, 843, EDITOR_RUNS),
];

/// The impl functions `f` runs through `helper(&STATIC ..)`, with the token position of the helper.
fn typed_static_callees(graph: &Graph, f: usize) -> Vec<(usize, usize)> {
    let body = &graph.funcs[f].body;
    let mut out = Vec::new();
    for i in 0..body.len() {
        let Some(helper) = ident(body.get(i)) else { continue };
        if !TYPED_STATIC_CALLS.iter().any(|(_, h, _)| *h == helper) || !punct(body.get(i + 1), '(') || !punct(body.get(i + 2), '&') {
            continue;
        }
        let mut k = i + 3;
        while ident(body.get(k)).is_some() && body.get(k + 1) == Some(&Tok::PathSep) {
            k += 2;
        }
        let Some(name) = ident(body.get(k)) else { continue };
        for (s, h, callee) in TYPED_STATIC_CALLS {
            if *s == name && *h == helper {
                out.extend(graph.find(callee).into_iter().filter(|&c| c != f).map(|c| (i, c)));
            }
        }
    }
    out
}

/// Whether token `at` of `f` runs on the walking thread: outside a `thread_dispatch` branch, or in its own branch.
fn on_this_thread(graph: &Graph, f: usize, at: usize, plugin: bool) -> bool {
    match callgraph::thread_dispatch(&graph.funcs[f].body) {
        Some((p, g)) => !(if plugin { g.contains(&at) } else { p.contains(&at) }),
        None => true,
    }
}

/// The locks `f` takes with `try_*` (`NAME.try_lock()`): never a wait, but a successful one holds the lock.
fn try_takes(ctx: &Ctx, f: usize) -> BTreeSet<usize> {
    let body = &ctx.graph.funcs[f].body;
    (0..body.len())
        .filter(|&i| {
            let dot = index_end(body, i) + 1;
            punct(body.get(dot), '.') && ident(body.get(dot + 1)).is_some_and(|m| TRY_METHODS.contains(&m)) && punct(body.get(dot + 2), '(')
        })
        .filter_map(|i| ctx.lock_at(f, i))
        .collect()
}

/// Per lock: the game-hook functions that take it blocking, and the plugin-thread functions (with their thread) that hold it.
/// Per function: the plugin threads that reach it.
struct Shared {
    game_waits: BTreeMap<usize, BTreeSet<String>>,
    /// Per lock: the hooks (keys) whose reach waits on it.
    game_hooks: BTreeMap<usize, BTreeSet<String>>,
    plugin_holds: BTreeMap<usize, BTreeSet<String>>,
    plugin_reached: BTreeMap<usize, BTreeSet<String>>,
}

/// `(from, token, to)` for a runtime function handed over by bare name as a value (`.and_then(stage_evidence)`,
/// `service(waiting_name_allowed)`): `calls` keeps a value use only when it is path-qualified. It runs on the thread that hands it
/// over (a synchronous callback) or is a registration whose runner `REGISTRARS` names, so the edge is from the function that hands
/// it over (docs lane review R2, 2026-11-10). A same-file function only, and not a name the body binds itself.
fn value_edges(graph: &Graph) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for (f, func) in graph.funcs.iter().enumerate() {
        let b = &func.body;
        for i in 1..b.len() {
            let Some(name) = ident(b.get(i)) else { continue };
            let next_ok = matches!(b.get(i + 1), Some(Tok::Punct(',' | ')' | ';' | ']' | '}'))) || ident(b.get(i + 1)) == Some("as");
            if !next_ok || !matches!(b.get(i - 1), Some(Tok::Punct('(' | ',' | '=' | '[' | '{' | '&'))) {
                continue;
            }
            let bound = (0..b.len().saturating_sub(1)).any(|k| {
                matches!((&b[k], &b[k + 1]), (Tok::Ident(l), Tok::Ident(m)) if (l == "let" || l == "mut" || l == "for") && m == name)
                    || matches!((&b[k], &b[k + 1]), (Tok::Punct('|' | '(' | ','), Tok::Ident(m))
                        if m == name && matches!(b.get(k + 2), Some(Tok::Punct(':' | '|' | ','))))
            });
            if bound {
                continue;
            }
            for (to, target) in graph.funcs.iter().enumerate() {
                if to != f && target.name == name && target.file == func.file && !target.has_receiver {
                    out.push((f, i, to));
                }
            }
        }
    }
    out
}

fn shared_locks(ctx: &Ctx, s: &Scan) -> Shared {
    let graph = ctx.graph;
    let n = graph.funcs.len();
    // Per function: (token position, callee) for path and method calls and typed-static helpers.
    let mut callees: Vec<Vec<(usize, usize)>> = (0..n)
        .map(|f| {
            let mut out: Vec<(usize, usize)> =
                calls(&graph.funcs[f].body).iter().flat_map(|c| ctx.resolve(f, c).into_iter().map(move |t| (c.at, t))).collect();
            out.extend(typed_static_callees(graph, f));
            out
        })
        .collect();
    for (from, to) in callback_edges(graph) {
        callees[from].push((0, to));
    }
    for (from, at, to) in value_edges(graph) {
        callees[from].push((at, to));
    }
    let walk = |starts: Vec<usize>, plugin: bool| -> BTreeSet<usize> {
        let mut seen = BTreeSet::new();
        let mut stack = starts;
        while let Some(f) = stack.pop() {
            if seen.insert(f) {
                stack.extend(callees[f].iter().filter(|(at, _)| on_this_thread(graph, f, *at, plugin)).map(|(_, c)| *c));
            }
        }
        seen
    };
    let mut hooks: Vec<usize> = (0..n).filter(|&f| graph.funcs[f].attrs.contains("skyline::hook")).collect();
    hooks.extend(GAME_ENTRIES.iter().flat_map(|key| graph.find(key)));
    let mut out = Shared {
        game_waits: BTreeMap::new(),
        game_hooks: BTreeMap::new(),
        plugin_holds: BTreeMap::new(),
        plugin_reached: BTreeMap::new(),
    };
    for hook in hooks {
        for f in walk(vec![hook], false) {
            for t in s.takes[f].iter().filter(|t| on_this_thread(graph, f, t.at, false)) {
                out.game_waits.entry(t.lock).or_default().insert(format!("{} (hook {})", graph.funcs[f].key(), graph.funcs[hook].key()));
                out.game_hooks.entry(t.lock).or_default().insert(graph.funcs[hook].key());
            }
        }
    }
    for (key, name, module) in plugin_entries() {
        let starts = entry_starts(graph, &key, &module);
        for f in walk(starts, true) {
            out.plugin_reached.entry(f).or_default().insert(name.clone());
            let held: BTreeSet<usize> = s.takes[f]
                .iter()
                .filter(|t| on_this_thread(graph, f, t.at, true))
                .map(|t| t.lock)
                .chain(try_takes(ctx, f))
                .collect();
            for lock in held {
                out.plugin_holds.entry(lock).or_default().insert(format!("{} ({name})", graph.funcs[f].key()));
            }
        }
    }
    out
}

#[test]
fn no_lock_a_game_hook_waits_on_is_held_by_a_plugin_thread() {
    let graph = runtime_graph();
    let (ctx, s) = scan(&graph);
    let shared = shared_locks(&ctx, &s);
    let measured = |hook: &str| MEASURED_ON_MAIN_THREAD.iter().any(|(h, _, _, _)| *h == hook);
    let mut problems = Vec::new();
    let mut used = BTreeSet::new();
    for (lock, waits) in &shared.game_waits {
        let Some(holds) = shared.plugin_holds.get(lock) else { continue };
        let name = lock_name(&ctx, *lock);
        if SHARED_ON_PURPOSE.iter().any(|(l, _)| *l == name) {
            println!("SHARED-ON-PURPOSE {name}");
            continue;
        }
        let hooks = &shared.game_hooks[lock];
        used.extend(hooks.iter().filter(|h| measured(h)).cloned());
        let open: Vec<&String> = hooks.iter().filter(|h| !measured(h)).collect();
        if open.is_empty() {
            let runs: BTreeSet<&str> =
                MEASURED_ON_MAIN_THREAD.iter().filter(|(h, _, _, _)| hooks.contains(*h)).map(|(_, _, _, runs)| *runs).collect();
            println!("MEASURED {name}: every waiting hook ran on MainThread only ({runs:?}): {hooks:?}");
            continue;
        }
        problems.push(format!(
            "{name}\n    hooks not measured MainThread-only: {}\n    game hooks wait: {}\n    plugin threads hold: {}",
            open.iter().map(|h| h.as_str()).collect::<Vec<_>>().join(", "),
            waits.iter().cloned().collect::<Vec<_>>().join(", "),
            holds.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    for (hook, calls, on_main, _) in MEASURED_ON_MAIN_THREAD {
        assert_eq!(calls, on_main, "{hook}: {calls} calls but {on_main} on MainThread: it ran on a worker thread; fix its locks");
        assert!(*calls > 0, "{hook}: never reached; an unreached hook is not measured");
        let found = graph.find(hook);
        assert!(
            GAME_ENTRIES.contains(hook) || found.iter().any(|&f| graph.funcs[f].attrs.contains("skyline::hook")),
            "{hook} is not a game hook any more"
        );
        assert!(used.contains(*hook), "{hook} waits on no shared lock now: delete its MEASURED_ON_MAIN_THREAD row");
    }
    for p in &problems {
        println!("SHARED {p}");
    }
    assert!(problems.is_empty(), "locks a game hook waits on that a plugin thread can hold ({}):\n{}", problems.len(), problems.join("\n"));
}

/// The lock-free runtime (operator order 2026-11-10 00:44Z, `artifacts/phase23/TASK-OPUS-rc6-lockfree.md`): no plugin thread ever
/// takes a lock that any game entry point can wait on. The inventory is every such lock with no exception at all (no
/// `SHARED_ON_PURPOSE`, no `MEASURED_ON_MAIN_THREAD`), over the diag build (the Eden runs' build: its own modules, threads and hooks
/// too), every `#[skyline::hook]` and `GAME_ENTRIES`, the callback tables and bare-name function values. It is the work queue:
/// `LOCKFREE_INVENTORY_MAX` only goes down, step by step, to 0.
const LOCKFREE_INVENTORY_MAX: usize = 1;

/// The diag build's call graph (`runtime_graph` plus the `#[cfg(feature = "diag")]` modules).
fn diag_runtime_graph() -> Graph {
    let mut graph = Graph::scan_with_diag(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"));
    graph.precise_imports = true;
    graph
}

/// `(lock, the game entries that wait on it, the plugin functions and threads that hold it)`, with no exception.
fn lockfree_inventory(ctx: &Ctx, shared: &Shared) -> Vec<(String, BTreeSet<String>, BTreeSet<String>)> {
    shared
        .game_waits
        .keys()
        .filter_map(|lock| {
            let holds = shared.plugin_holds.get(lock)?;
            Some((lock_name(ctx, *lock), shared.game_hooks[lock].clone(), holds.clone()))
        })
        .collect()
}

#[test]
fn the_lockfree_inventory_only_shrinks() {
    let graph = diag_runtime_graph();
    let (ctx, s) = scan(&graph);
    let shared = shared_locks(&ctx, &s);
    let items = lockfree_inventory(&ctx, &shared);
    println!("LOCKFREE-INVENTORY {}", items.len());
    for (lock, entries, holds) in &items {
        let threads: BTreeSet<&str> = holds.iter().filter_map(|h| h.split_once(" (").and_then(|(_, t)| t.strip_suffix(')'))).collect();
        println!("LOCKFREE {lock}\n    entries wait: {}\n    threads hold: {}\n    held in: {}",
            entries.iter().cloned().collect::<Vec<_>>().join(", "),
            threads.into_iter().collect::<Vec<_>>().join(", "),
            holds.iter().cloned().collect::<Vec<_>>().join(", "));
    }
    assert!(items.len() <= LOCKFREE_INVENTORY_MAX, "the lock-free inventory grew: {} > {LOCKFREE_INVENTORY_MAX}", items.len());
}

/// Every plugin thread's entry resolves to the function its `model::thread_stacks` symbol names, in the diag build (no thread is
/// skipped silently: `macros::diag::start` once resolved to `threads.rs`'s `mod diag`, and `practice_trace::writer`, inside
/// `mod enabled`, to nothing; docs lane review R2), and in the shipped build every thread but the diag build's own.
#[test]
fn every_plugin_thread_entry_resolves() {
    let diag = diag_runtime_graph();
    for (found, name) in plugin_entry_funcs(&diag) {
        assert!(!found.is_empty(), "{name}: its entry resolves to no function in the diag build");
    }
    let shipped = runtime_graph();
    let missing: BTreeSet<String> =
        plugin_entry_funcs(&shipped).into_iter().filter(|(found, _)| found.is_empty()).map(|(_, name)| name).collect();
    let diag_only: BTreeSet<String> = ["tr-probe", "tr-macro-diag"].iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(missing, diag_only, "the shipped build lacks only the diag build's own threads");
}

/// The saved-Mii catalog read is the one plugin-thread job behind the Training-entry freeze (`yoga-i3-final`, primary 2026-11-09
/// 19:30Z/19:44Z): the writer (`replay-capture-save`) or the fallback reader (`mii-catalog`) reads the files and sends the owned
/// snapshot back through `model::catalog_reply` (a `try_send`, no lock). The game thread polls it in the roster callback
/// (`mii_catalog_request::service` -> `poll` -> `apply`) and hands it to the role states; `menu_handoff::service` (game thread
/// only) opens, closes and retires requests. So the broker and its consumers must stay off plugin threads: these functions take
/// the locks the game's Mii and Training hooks take (`mii_auto_init::STATE`, `mii_saved_alpha::ALPHA`, the bridge's `STATE`).
const CATALOG_GAME_SIDE: &[&str] = &[
    "mii_catalog_request::service",
    "mii_catalog_request::apply",
    "mii_catalog_request::abandon_auto",
    "mii_catalog_request::request",
    "mii_catalog_request::close_opening",
    "mii_catalog_request::retire",
    "mii_catalog_request::retain_editor",
    "mii_auto_init::deliver",
    "mii_auto_init::abandon",
    "mii_auto_init::reset",
    "mii_auto_init::cancel",
    "mii_saved_alpha::accept_catalog",
    "mii_editor_bridge::catalog_read",
];

#[test]
fn the_mii_catalog_reply_is_consumed_on_the_game_thread_only() {
    let graph = runtime_graph();
    let (ctx, s) = scan(&graph);
    let shared = shared_locks(&ctx, &s);
    let mut problems = Vec::new();
    for key in CATALOG_GAME_SIDE {
        let found = graph.find(key);
        assert!(!found.is_empty(), "{key} is gone: update CATALOG_GAME_SIDE");
        for f in found {
            if let Some(threads) = shared.plugin_reached.get(&f) {
                problems.push(format!("{key} is reached from {}", threads.iter().cloned().collect::<Vec<_>>().join(", ")));
            }
        }
    }
    assert!(problems.is_empty(), "the catalog's game-side consumers run on a plugin thread:\n{}", problems.join("\n"));
}

/// The functions of `src/runtime` whose address the plugin hands to the game or to a native caller: every `extern "C"` /
/// `extern "Rust"` function without `#[skyline::hook]`, and every function of a file taken there as `name as *const ()`. Keys are
/// the call graph's (`enabled::list_step` for a function in `practice_trace.rs`'s `mod enabled`); a file the graph does not scan
/// (diag-only) gives `stem::name`.
fn handed_out(graph: &Graph) -> BTreeSet<String> {
    fn walk(dir: &Path, root: &Path, graph: &Graph, out: &mut BTreeSet<String>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, root, graph, out);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
            let module = if stem == "mod" { path.parent().unwrap().file_name().unwrap().to_string_lossy().into_owned() } else { stem };
            let mut insert = |name: &str| {
                let keys: Vec<String> = graph.funcs.iter().filter(|f| f.file == rel && f.name == name).map(|f| f.key()).collect();
                if keys.is_empty() {
                    out.insert(format!("{module}::{name}"));
                }
                out.extend(keys);
            };
            let text = std::fs::read_to_string(&path).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            let defined = |name: &str| text.contains(&format!("fn {name}("));
            for (i, line) in lines.iter().enumerate() {
                for abi in ["extern \"C\" fn ", "extern \"Rust\" fn "] {
                    if let Some(at) = line.find(abi) {
                        let name: String = line[at + abi.len()..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                        let attrs = lines[i.saturating_sub(3)..i].join(" ");
                        if !name.is_empty() && !attrs.contains("skyline::hook") {
                            insert(&name);
                        }
                    }
                }
                let mut rest = *line;
                while let Some(at) = rest.find(" as *const ()") {
                    let name: String =
                        rest[..at].chars().rev().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<Vec<_>>().into_iter().rev().collect();
                    if !name.is_empty() && defined(&name) {
                        insert(&name);
                    }
                    rest = &rest[at + 1..];
                }
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut out = BTreeSet::new();
    walk(&root, &root, graph, &mut out);
    out
}

/// The ambiguous method calls `Ctx::resolve` leaves unfollowed while a runtime candidate reaches a lock: `(method, receiver)` (`?`
/// when the receiver is not an identifier) to the functions that make them.
fn unfollowed_ambiguous(ctx: &Ctx, s: &Scan) -> BTreeMap<(String, String), BTreeSet<String>> {
    let graph = ctx.graph;
    let mut out: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for f in 0..graph.funcs.len() {
        for call in calls(&graph.funcs[f].body) {
            let Some(method) = call.segments.last().filter(|_| call.method) else { continue };
            if LOCK_METHODS.contains(&method.as_str()) || !ctx.resolve(f, &call).is_empty() {
                continue;
            }
            let Some(all) = ctx.by_name.get(method) else { continue };
            let methods: Vec<usize> = all.iter().copied().filter(|&m| graph.funcs[m].has_receiver).collect();
            if methods.len() < 2 || !methods.iter().any(|&m| m != f && !s.reach[m].is_empty()) {
                continue;
            }
            let receiver = call.receiver.clone().unwrap_or_else(|| "?".to_owned());
            out.entry((method.clone(), receiver)).or_default().insert(graph.funcs[f].key());
        }
    }
    out
}

/// R1 finding 2's guard: every function handed to the game is a listed entry point (walked as a game hook) or listed as not one,
/// with its reason, and every listed name is still handed out.
#[test]
fn every_function_handed_to_the_game_is_listed() {
    let handed = handed_out(&runtime_graph());
    for known in ["cpu::on_frame", "cpu_common::on_nro_load", "native_service::training_mod_native_service_dispatch", "imp::detached"] {
        assert!(handed.contains(known), "the scan of handed-out functions misses {known}: {handed:?}");
    }
    let listed: BTreeSet<String> =
        GAME_ENTRIES.iter().map(|k| (*k).to_owned()).chain(NOT_GAME_ENTRIES.iter().map(|(k, _)| (*k).to_owned())).collect();
    let missing: Vec<&String> = handed.difference(&listed).collect();
    let stale: Vec<&String> = listed.difference(&handed).collect();
    assert!(missing.is_empty(), "handed to the game but on neither GAME_ENTRIES nor NOT_GAME_ENTRIES: {missing:?}");
    assert!(stale.is_empty(), "listed but no longer handed out: {stale:?}");
}

/// R2 finding 5's guard: every game entry and every plugin thread's entry resolves to a function on its module path, or is listed in
/// `UNSCANNED_ENTRIES` (and does not resolve). A key that resolves to nothing would drop its whole reach from the check.
#[test]
fn every_listed_entry_resolves() {
    let graph = runtime_graph();
    let unscanned = |key: &str| UNSCANNED_ENTRIES.iter().any(|(k, _)| *k == key);
    let mut keys: Vec<(String, Vec<usize>)> = GAME_ENTRIES.iter().map(|k| ((*k).to_owned(), graph.find(k))).collect();
    for (key, _, module) in plugin_entries() {
        let starts = entry_starts(&graph, &key, &module);
        keys.push((if module.is_empty() { key } else { format!("{module}::{}", key.rsplit("::").next().unwrap()) }, starts));
    }
    let mut problems = Vec::new();
    for (key, found) in &keys {
        match (found.is_empty(), unscanned(key)) {
            (true, false) => problems.push(format!("{key} resolves to no function on its module path")),
            (false, true) => problems.push(format!("{key} resolves now: delete its UNSCANNED_ENTRIES row")),
            _ => {}
        }
    }
    for (key, _) in UNSCANNED_ENTRIES {
        if !keys.iter().any(|(k, _)| k == key) {
            problems.push(format!("{key} is no longer an entry: delete its UNSCANNED_ENTRIES row"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// R2 finding 1's guard (docs lane, 2026-11-10 00:36Z): `Ctx::resolve` follows an ambiguous method only by its receiver, so every
/// call it leaves unfollowed while a runtime candidate reaches a lock is classified in `FOREIGN_RECEIVERS`. A new one fails here
/// until it is: a real receiver goes into `RECEIVER_MODULES`, a foreign one into `FOREIGN_RECEIVERS` with the reason.
#[test]
fn every_unfollowed_ambiguous_call_that_reaches_a_lock_is_classified() {
    let graph = runtime_graph();
    let (ctx, s) = scan(&graph);
    let found = unfollowed_ambiguous(&ctx, &s);
    let classified = |m: &str, r: &str| FOREIGN_RECEIVERS.iter().any(|(method, receivers, _)| *method == m && receivers.contains(&r));
    let missing: Vec<String> =
        found.iter().filter(|((m, r), _)| !classified(m, r)).map(|((m, r), callers)| format!(".{r}.{m}() in {callers:?}")).collect();
    assert!(
        missing.is_empty(),
        "ambiguous calls left unfollowed whose runtime candidates reach a lock; name the receiver's module (RECEIVER_MODULES) or \
         classify it (FOREIGN_RECEIVERS):\n{}",
        missing.join("\n")
    );
    for (method, receivers, _) in FOREIGN_RECEIVERS {
        for r in *receivers {
            assert!(found.contains_key(&((*method).to_owned(), (*r).to_owned())), ".{r}.{method}() is not left unfollowed now: delete it");
        }
    }
    for (receiver, module) in RECEIVER_MODULES {
        assert!(graph.funcs.iter().any(|f| f.module == *module && f.has_receiver), "RECEIVER_MODULES: {receiver} -> {module} has no methods");
    }
}

/// R1 finding 1: the callback edges are followed (the UI pass reaches the HUD draws, the heartbeat reaches its ticks), and they cover
/// every edge `tests/capabilities.rs` declares for its own scan.
#[test]
fn the_callback_edges_cover_the_capability_scan() {
    let graph = runtime_graph();
    let edges: BTreeSet<(String, String)> =
        callback_edges(&graph).into_iter().map(|(a, b)| (graph.funcs[a].key(), graph.funcs[b].key())).collect();
    for want in [("hud::run", "hud_draw::draw"), ("hud::run", "hitbox::draw_orbs"), ("diag_log::start_drain_thread", "p1::service")] {
        assert!(edges.contains(&(want.0.to_owned(), want.1.to_owned())), "{want:?} not followed: {edges:?}");
    }
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/capabilities.rs")).unwrap();
    let start = text.find("fn new() -> Analysis").expect("capabilities.rs Analysis::new");
    let end = start + text[start..].find("let consts").expect("capabilities.rs Analysis::new end");
    let ends: BTreeSet<&str> = edges.iter().flat_map(|(a, b)| [a.as_str(), b.as_str()]).collect();
    for literal in text[start..end].split('"').skip(1).step_by(2).filter(|s| s.contains("::") && !s.contains(' ')) {
        assert!(ends.contains(literal), "capabilities.rs declares {literal} in an edge this check does not follow");
    }
}

/// A bare name a `use` imports from outside the runtime is that item (`runtime_graph`): `lobby_persist::entry_applied` calls the
/// model's `restore`, so it reaches neither `kirby_hats::restore` nor `character_items::restore`, which the over-approximating graph
/// (`Graph::scan` as the capability scan uses it) follows.
#[test]
fn an_imported_model_function_is_not_a_runtime_call() {
    let precise = runtime_graph();
    let loose = Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[]);
    let reaches = |graph: &Graph, to: &str| {
        let from = graph.find("lobby_persist::entry_applied");
        assert_eq!(from.len(), 1);
        let callees = graph.callees(from[0]);
        graph.find(to).iter().any(|c| callees.contains(c))
    };
    for to in ["kirby_hats::restore", "character_items::restore"] {
        assert!(reaches(&loose, to), "{to}: the control lost its edge");
        assert!(!reaches(&precise, to), "{to}: an imported model function resolved to a runtime one");
    }
}

/// R1 addendum (docs lane, 2026-11-10 00:25Z): a method name several runtime types define (`open`, `tick`, `press`, `view` on
/// `ui/combos.rs` and `ui/setups.rs`) resolves by the receiver's field, so the menu worker reaches the combo panel and what it
/// locks (`combo::set_target`, `combo::last_combo`).
#[test]
fn an_ambiguous_method_resolves_by_its_receivers_field() {
    let graph = runtime_graph();
    let (ctx, _) = scan(&graph);
    let worker = graph.find("worker::sync_panels");
    assert_eq!(worker.len(), 1);
    let keys: BTreeSet<String> = calls(&graph.funcs[worker[0]].body)
        .iter()
        .filter(|c| c.receiver.as_deref() == Some("combos"))
        .flat_map(|c| ctx.resolve(worker[0], c))
        .map(|t| graph.funcs[t].key())
        .collect();
    assert!(keys.contains("combos::open") && keys.contains("combos::tick"), "s.combos.open()/.tick() not followed: {keys:?}");
    assert!(!keys.iter().any(|k| k.starts_with("setups::")), "a combos call resolved to the setups panel: {keys:?}");
}

/// Two scan blind spots (rc.6, hot-slots hook-thread table 2026-11-09: `allocation_trace` runs on the game's task workers): a call
/// through a `#[path]` module declared under another name (`character_roster::body_bank::creation` is `native_body_bank.rs`), and
/// one lock of a static array (`BANKS[role].lock()`). Both are followed now.
#[test]
fn a_path_module_call_and_an_indexed_take_are_seen() {
    assert_eq!(
        callgraph::path_modules("#[cfg(feature = \"x\")]\n#[path = \"native_body_bank.rs\"]\npub(crate) mod body_bank;\n#[path = \"../mii/a.rs\"] mod a;\n"),
        [("body_bank".to_owned(), "native_body_bank".to_owned())]
    );
    let graph = runtime_graph();
    let (ctx, s) = scan(&graph);
    let one = |key: &str| {
        let found = graph.find(key);
        assert_eq!(found.len(), 1, "{key}");
        found[0]
    };
    let (hook, creation) = (one("character_lifecycle::allocation_trace"), one("native_body_bank::creation"));
    assert!(graph.callees(hook).contains(&creation), "allocation_trace -> body_bank::creation not followed");
    let taken: Vec<String> = s.takes[creation].iter().map(|t| lock_name(&ctx, t.lock)).collect();
    assert!(taken.iter().any(|l| l.ends_with("native_body_bank.rs::BANKS")), "BANKS[role].lock() not seen: {taken:?}");
}

/// The class check on a fixture: a hook that waits on a lock a plugin thread holds (blocking, and with a `try_lock`), a hook
/// that only `try_lock`s one, and a typed static read whose impl differs by thread.
#[test]
fn the_shared_lock_check_sees_a_seeded_hazard() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lock_order_shared");
    let graph = Graph::scan(&dir, &[]);
    let ctx = Ctx::new(&graph, static_locks(&dir));
    let s = Scan { takes: (0..graph.funcs.len()).map(|f| takes(&ctx, f)).collect(), ..Scan::default() };
    let shared = shared_locks(&ctx, &s);
    let name = |l: &usize| ctx.locks[*l].1.clone();
    let hazards: BTreeSet<String> =
        shared.game_waits.keys().filter(|l| shared.plugin_holds.contains_key(l)).map(name).collect();
    assert!(hazards.contains("HELD"), "a blocking take on both sides: {hazards:?}");
    assert!(hazards.contains("TRIED"), "a plugin try_lock holds it too: {hazards:?}");
    assert!(!hazards.contains("POLLED"), "a game-side try_lock never waits: {hazards:?}");
    assert!(hazards.contains("EDITED"), "the game-thread impl of a typed read waits on EDITED, the plugin one holds it: {hazards:?}");
    assert!(!hazards.contains("PUBLISHING"), "only the plugin impl takes PUBLISHING: {hazards:?}");
    assert!(hazards.contains("ROLES"), "one lock of a static array, taken by index on both sides: {hazards:?}");
}

/// The scan on a small fixture: a guard bound by `let` and held over a call, a temporary held over its statement, a
/// guard dropped before the call, and an inversion between two threads.
#[test]
fn the_scan_sees_an_inversion_a_relock_and_a_dropped_guard() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lock_order");
    let graph = Graph::scan(&dir, &[]);
    let ctx = Ctx::new(&graph, static_locks(&dir));
    let n = graph.funcs.len();
    let mut s = Scan { takes: (0..n).map(|f| takes(&ctx, f)).collect(), ..Scan::default() };
    let callees: Vec<BTreeSet<usize>> =
        (0..n).map(|f| calls(&graph.funcs[f].body).iter().flat_map(|c| ctx.resolve(f, c)).collect()).collect();
    s.reach = s.takes.iter().map(|t| t.iter().map(|t| t.lock).collect()).collect();
    for _ in 0..n {
        for (f, out) in callees.iter().enumerate() {
            let add: BTreeSet<usize> = out.iter().flat_map(|&c| s.reach[c].iter().copied()).collect();
            s.reach[f].extend(add);
        }
    }
    let (edges, waits) = edges(&ctx, &s);
    let text: Vec<String> =
        edges.iter().map(|e| format!("{}->{} {} {}", ctx.locks[e.held].1, ctx.locks[e.taken].1, e.func, e.via)).collect();
    let has = |want: &str| text.iter().any(|t| t == want);
    assert!(has("A->B worker::close_like publish::publish_like"), "bound guard over a call: {text:?}");
    assert!(has("B->A hook::on_frame direct"), "temporary over its match: {text:?}");
    assert!(has("A->A worker::relock worker::helper_takes_a"), "relock through a call: {text:?}");
    assert!(!text.iter().any(|t| t.contains("worker::dropped_first")), "a dropped guard holds nothing: {text:?}");
    assert!(!text.iter().any(|t| t.contains("worker::statement_only")), "a temporary ends with its statement: {text:?}");
    assert!(!text.iter().any(|t| t.contains("worker::closure_only")), "a closure's temporary ends with it: {text:?}");
    assert!(!text.iter().any(|t| t.contains("worker::condition_only")), "an `if` condition drops its temporaries: {text:?}");
    assert!(has("A->B worker::if_let_holds publish::publish_like"), "`if let` holds its scrutinee (2021): {text:?}");
    assert!(!text.iter().any(|t| t.contains("worker::block_statement_ends")), "a block statement ends with its block: {text:?}");
    assert_eq!(cycles(&edges, ctx.locks.len()).len(), 1, "{text:?}");
    assert!(waits.iter().any(|w| w.contains("worker::sleeps_holding")), "{waits:?}");
}
