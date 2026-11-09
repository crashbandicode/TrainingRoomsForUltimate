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

/// A call or reference in a body, at token `at`: `(segments, method)`.
struct Call {
    at: usize,
    segments: Vec<String>,
    method: bool,
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
                out.push(Call { at: i, segments, method });
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
            return if methods.len() == 1 && methods[0] != from { methods } else { Vec::new() };
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
        let Some(name) = ident(body.get(i)) else { return None };
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
    bound: bool,
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
                out.push(Take { lock, at: i, end, bound });
            }
        }
    }
    for i in 0..body.len() {
        let Some(lock) = ctx.lock_at(f, i) else { continue };
        let method = punct(body.get(i + 1), '.')
            && ident(body.get(i + 2)).is_some_and(|m| LOCK_METHODS.contains(&m))
            && punct(body.get(i + 3), '(');
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
        let call_close = if helper { i + 1 } else { i + 4 };
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
        let (end, bound) = if let Some(close) = closure_end(body, s, at) {
            (close, false)
        } else if let Some(open) = condition_end(body, s, at) {
            (open, false)
        } else if let Some(name) = bound_name {
            let block = block_end(body, at);
            let dropped = (at..block).find(|&j| {
                ident(body.get(j)) == Some("drop") && punct(body.get(j + 1), '(') && ident(body.get(j + 2)) == Some(name.as_str())
            });
            (dropped.unwrap_or(block), true)
        } else if else_bound {
            (block_end(body, at), true)
        } else {
            (statement_end(body, s, at, arm), false)
        };
        out.push(Take { lock, at, end, bound });
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

fn plugin_entries() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = PUBLISHING_ENTRIES.iter().map(|e| ((*e).to_owned(), format!("publish({e})"))).collect();
    for row in thread_stacks::THREADS.iter() {
        let path = row.symbol.split("::{").next().unwrap_or(row.symbol);
        let segments: Vec<&str> = path.split("::").collect();
        let key = segments[segments.len().saturating_sub(2)..].join("::");
        let name = row.name.to_owned();
        if !out.iter().any(|(k, _)| *k == key) {
            out.push((key, name));
        }
    }
    out
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
        for f in 0..n {
            let add: BTreeSet<usize> = callees[f].iter().flat_map(|&c| s.reach[c].iter().copied()).collect();
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
    for (key, name) in plugin_entries() {
        let found = graph.find(&key);
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
    let graph = Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[]);
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
}

#[test]
fn no_two_locks_are_taken_in_both_orders_and_no_lock_is_retaken_while_held() {
    let graph = Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[]);
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
        for f in 0..n {
            let add: BTreeSet<usize> = callees[f].iter().flat_map(|&c| s.reach[c].iter().copied()).collect();
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
