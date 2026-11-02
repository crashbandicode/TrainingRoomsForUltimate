//! A small source scanner for the runtime's call graph (review R2-1): it reads `src/runtime/**/*.rs` as text
//! (the runtime is Switch-only and cannot be compiled or run on the host), finds every function, its calls and
//! the game-offset constants it names, and answers "which sites can this function reach". It over-approximates
//! on purpose (a method call reaches every receiver function of that name; a bare name reaches the functions of that
//! name in its own file or in a file it imports from), so a missed edge is unlikely and an extra edge shows up
//! as an unexpected capability the table has to name.
//!
//! The scanner is a tool of the test, not of the plugin: the plugin carries only the *result* (the declared
//! closures in `model::capabilities`), and `tests/capabilities.rs` fails when the two disagree.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    Ident(String),
    Punct(char),
    PathSep,
}

/// One function (or macro-generated hook) of the runtime.
#[derive(Clone, Debug)]
pub struct Func {
    /// Path relative to `src/runtime`, e.g. `features/cpu.rs`.
    pub file: String,
    /// The innermost `mod` of the definition, else the file's stem (`cpu`; `mod.rs` takes its folder's name).
    pub module: String,
    pub name: String,
    /// A declared `self` receiver; free/associated functions cannot be dot-call targets.
    pub has_receiver: bool,
    /// The text of the attributes before it (`skyline::hook(offset = sites_pkg2::SDI_HOOK_OFFSET)`).
    pub attrs: String,
    pub body: Vec<Tok>,
}

impl Func {
    pub fn key(&self) -> String {
        format!("{}::{}", self.module, self.name)
    }

    /// The constant a `#[skyline::hook(offset = X)]` names.
    pub fn hook_offset_const(&self) -> Option<String> {
        let at = self.attrs.find("skyline::hook")?;
        let rest = &self.attrs[at..];
        let eq = rest.find("offset")?;
        let value = rest[eq + 6..].trim_start().strip_prefix('=')?.trim_start();
        let end = value.find([',', ')'])?;
        let name = value[..end].trim();
        Some(name.rsplit("::").next()?.to_owned())
    }
}

// ---- lexing ---------------------------------------------------------------------------------------------

/// Comments, strings and chars become spaces (newlines kept).
fn sanitize(src: &str) -> String {
    let b: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied().unwrap_or('\0');
        if c == '/' && next == '/' {
            while i < b.len() && b[i] != '\n' {
                out.push(' ');
                i += 1;
            }
        } else if c == '/' && next == '*' {
            let mut depth = 0;
            while i < b.len() {
                if b[i] == '/' && b.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if b[i] == '*' && b.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(b[i]));
                    i += 1;
                }
            }
        } else if c == 'r' && (next == '"' || (next == '#' && matches!(b.get(i + 2), Some('"') | Some('#')))) && (i == 0 || !(b[i - 1].is_alphanumeric() || b[i - 1] == '_')) {
            // raw string r"..." or r#"..."#
            let mut j = i + 1;
            let mut hashes = 0;
            while b.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            j += 1; // the quote
            loop {
                if j >= b.len() {
                    break;
                }
                if b[j] == '"' && (0..hashes).all(|k| b.get(j + 1 + k) == Some(&'#')) {
                    j += 1 + hashes;
                    break;
                }
                j += 1;
            }
            while i < j {
                out.push(blank(b[i]));
                i += 1;
            }
        } else if c == '"' {
            out.push(' ');
            i += 1;
            while i < b.len() && b[i] != '"' {
                if b[i] == '\\' {
                    out.push(' ');
                    i += 1;
                }
                if i < b.len() {
                    out.push(blank(b[i]));
                    i += 1;
                }
            }
            out.push(' ');
            i += 1;
        } else if c == '\'' {
            // A char literal ('x', '\n', '\u{..}') or a lifetime ('a).
            if next == '\\' {
                let mut j = i + 2;
                while j < b.len() && b[j] != '\'' {
                    j += 1;
                }
                while i <= j && i < b.len() {
                    out.push(' ');
                    i += 1;
                }
            } else if b.get(i + 2) == Some(&'\'') {
                out.push_str("   ");
                i += 3;
            } else {
                out.push(' ');
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

pub fn lex(src: &str) -> Vec<Tok> {
    let text = sanitize(src);
    let b: Vec<char> = text.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_') {
                i += 1;
            }
            toks.push(Tok::Ident(b[start..i].iter().collect()));
        } else if c.is_ascii_digit() {
            // A number (with a suffix and underscores) is one ident-like token.
            let start = i;
            while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_' || (b[i] == '.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit()))) {
                i += 1;
            }
            toks.push(Tok::Ident(b[start..i].iter().collect()));
        } else if c == ':' && b.get(i + 1) == Some(&':') {
            toks.push(Tok::PathSep);
            i += 2;
        } else {
            toks.push(Tok::Punct(c));
            i += 1;
        }
    }
    toks
}

fn is_punct(t: &Tok, c: char) -> bool {
    matches!(t, Tok::Punct(p) if *p == c)
}

fn ident(t: &Tok) -> Option<&str> {
    match t {
        Tok::Ident(s) => Some(s),
        _ => None,
    }
}

/// Index of the token closing the group opened at `open` (`{`, `(`, `[`).
fn matching(toks: &[Tok], open: usize) -> usize {
    let (o, c) = match &toks[open] {
        Tok::Punct('{') => ('{', '}'),
        Tok::Punct('(') => ('(', ')'),
        Tok::Punct('[') => ('[', ']'),
        other => panic!("not an opening token: {other:?}"),
    };
    let mut depth = 0;
    for (i, t) in toks.iter().enumerate().skip(open) {
        if is_punct(t, o) {
            depth += 1;
        } else if is_punct(t, c) {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
    }
    toks.len() - 1
}

fn tokens_text(toks: &[Tok]) -> String {
    let mut out = String::new();
    for t in toks {
        match t {
            Tok::Ident(s) => {
                out.push_str(s);
                out.push(' ');
            }
            Tok::Punct(c) => out.push(*c),
            Tok::PathSep => out.push_str("::"),
        }
    }
    out.replace(" ::", "::").replace(":: ", "::")
}

// ---- items ----------------------------------------------------------------------------------------------

struct Parser<'a> {
    file: &'a str,
    funcs: Vec<Func>,
    /// Inside a `macro_rules!` body: its `{}` blocks hold the generated items, so they are entered, not skipped.
    in_macro: bool,
}

impl Parser<'_> {
    fn items(&mut self, toks: &[Tok], module: &str) {
        let mut i = 0;
        let mut attrs = String::new();
        while i < toks.len() {
            // Attributes.
            if is_punct(&toks[i], '#') && toks.get(i + 1).is_some_and(|t| is_punct(t, '[') || is_punct(t, '!')) {
                let open = if is_punct(&toks[i + 1], '!') { i + 2 } else { i + 1 };
                let close = matching(toks, open);
                attrs.push_str(&tokens_text(&toks[open + 1..close]));
                attrs.push(' ');
                i = close + 1;
                continue;
            }
            match ident(&toks[i]) {
                Some("mod") if toks.get(i + 2).is_some_and(|t| is_punct(t, '{')) => {
                    let name = ident(&toks[i + 1]).unwrap_or("?").to_owned();
                    let close = matching(toks, i + 2);
                    // `#[cfg(test)] mod tests` is not part of the shipped runtime.
                    if !attrs.contains("cfg(test)") {
                        self.items(&toks[i + 3..close], &name);
                    }
                    i = close + 1;
                    attrs.clear();
                }
                Some("impl") | Some("trait") | Some("extern") if self.block_follows(toks, i) => {
                    let open = self.block_open(toks, i);
                    let close = matching(toks, open);
                    self.items(&toks[open + 1..close], module);
                    i = close + 1;
                    attrs.clear();
                }
                Some("macro_rules") => {
                    // macro_rules ! name { ... }
                    let open = (i..toks.len()).find(|&k| is_punct(&toks[k], '{')).unwrap_or(toks.len() - 1);
                    let close = matching(toks, open);
                    let outer = std::mem::replace(&mut self.in_macro, true);
                    self.items(&toks[open + 1..close], module);
                    self.in_macro = outer;
                    i = close + 1;
                    attrs.clear();
                }
                Some("fn") if toks.get(i + 1).is_some_and(|t| matches!(t, Tok::Ident(_)) || is_punct(t, '$')) => {
                    let (name_at, name) = if is_punct(&toks[i + 1], '$') {
                        (i + 2, format!("${}", ident(&toks[i + 2]).unwrap_or("?")))
                    } else {
                        (i + 1, ident(&toks[i + 1]).unwrap_or("?").to_owned())
                    };
                    // The body: the first `{` before a `;` outside every group.
                    let mut k = name_at + 1;
                    let mut body = None;
                    let mut has_receiver = false;
                    while k < toks.len() {
                        if is_punct(&toks[k], '(') || is_punct(&toks[k], '[') {
                            if is_punct(&toks[k], '(') {
                                has_receiver |= toks[k + 1..matching(toks, k)].iter()
                                    .take_while(|t| !is_punct(t, ',') && !is_punct(t, ':') && **t != Tok::PathSep)
                                    .any(|t| ident(t) == Some("self"));
                            }
                            k = matching(toks, k) + 1;
                        } else if is_punct(&toks[k], '{') {
                            body = Some((k, matching(toks, k)));
                            break;
                        } else if is_punct(&toks[k], ';') {
                            break;
                        } else {
                            k += 1;
                        }
                    }
                    match body {
                        Some((open, close)) => {
                            let inner = &toks[open + 1..close];
                            self.funcs.push(Func { file: self.file.to_owned(), module: module.to_owned(), name, has_receiver, attrs: attrs.clone(), body: inner.to_vec() });
                            // Nested items (a local `fn`, a `mod`) are found too; the outer function keeps them.
                            self.items(inner, module);
                            i = close + 1;
                        }
                        None => i = k + 1,
                    }
                    attrs.clear();
                }
                Some("const") | Some("static") | Some("struct") | Some("enum") | Some("type") | Some("use") | Some("pub") | Some("unsafe") | Some("async") | Some("unsafe_extern") => {
                    // `pub`, `unsafe` and friends precede the item; `use`, `static`, `const`, `struct`, `enum`, `type`
                    // statements end at `;` or a `{}` block that holds no functions of interest.
                    match ident(&toks[i]) {
                        Some("pub") | Some("unsafe") | Some("async") => {
                            if toks.get(i + 1).is_some_and(|t| is_punct(t, '(')) {
                                i = matching(toks, i + 1) + 1;
                            } else {
                                i += 1;
                            }
                        }
                        _ => {
                            i = self.skip_statement(toks, i);
                            attrs.clear();
                        }
                    }
                }
                _ => {
                    if is_punct(&toks[i], '{') {
                        let close = matching(toks, i);
                        if self.in_macro {
                            self.items(&toks[i + 1..close], module);
                        }
                        i = close + 1;
                        attrs.clear();
                    } else if is_punct(&toks[i], ';') {
                        i += 1;
                        attrs.clear();
                    } else {
                        // A macro invocation at item level (`prob_hooks! { ... }`) or anything else: step on.
                        i += 1;
                    }
                }
            }
        }
    }

    /// Whether an `impl`/`trait`/`extern` at `i` opens a `{}` block of items (not `extern "C" fn`, not `extern crate`).
    fn block_follows(&self, toks: &[Tok], i: usize) -> bool {
        let mut k = i + 1;
        while k < toks.len() {
            match &toks[k] {
                Tok::Punct('{') => return true,
                Tok::Punct(';') => return false,
                Tok::Ident(s) if s == "fn" => return false,
                Tok::Punct('(') | Tok::Punct('[') => k = matching(toks, k) + 1,
                _ => k += 1,
            }
        }
        false
    }

    fn block_open(&self, toks: &[Tok], i: usize) -> usize {
        let mut k = i + 1;
        loop {
            match &toks[k] {
                Tok::Punct('{') => return k,
                Tok::Punct('(') | Tok::Punct('[') => k = matching(toks, k) + 1,
                _ => k += 1,
            }
        }
    }

    fn skip_statement(&self, toks: &[Tok], i: usize) -> usize {
        let mut k = i;
        while k < toks.len() {
            match &toks[k] {
                Tok::Punct(';') => return k + 1,
                Tok::Punct('{') => {
                    // `struct S { .. }` / `enum E { .. }` end at the block; `static X: T = { .. };` ends at `;`.
                    let close = matching(toks, k);
                    if toks.get(close + 1).is_some_and(|t| is_punct(t, ';')) {
                        return close + 2;
                    }
                    let head_is_value = toks[i..k].iter().any(|t| is_punct(t, '='));
                    if !head_is_value {
                        return close + 1;
                    }
                    k = close + 1;
                }
                Tok::Punct('(') | Tok::Punct('[') => k = matching(toks, k) + 1,
                _ => k += 1,
            }
        }
        k
    }
}

/// The names of `#[cfg(feature = "diag")] mod name;` declarations in a source text (read from the raw text: the lexer blanks
/// string literals, and the feature name is one).
fn lex_attrs(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut gated = false;
    for line in src.lines() {
        let line = line.trim();
        if line == "#[cfg(feature = \"diag\")]" {
            gated = true;
        } else if gated && line.starts_with("#[") {
            // another attribute between the cfg and the item
        } else {
            if gated {
                let decl = line.trim_start_matches("pub(crate) ").trim_start_matches("pub ");
                if let Some(name) = decl.strip_prefix("mod ").and_then(|rest| rest.strip_suffix(';')) {
                    out.push(name.trim().to_owned());
                }
            }
            gated = false;
        }
    }
    out
}
// ---- the graph ------------------------------------------------------------------------------------------

pub struct Graph {
    pub funcs: Vec<Func>,
    /// `file -> (alias -> module name)` from `use ... as alias;`.
    aliases: BTreeMap<String, BTreeMap<String, String>>,
    /// `file -> names it imports by `use`` (any identifier in a `use` statement).
    imports: BTreeMap<String, BTreeSet<String>>,
    by_name: BTreeMap<String, Vec<usize>>,
    /// Extra edges for calls the text cannot show (a function pointer table): `(from key, to key)`.
    pub dynamic: Vec<(String, String)>,
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn module_of_file(rel: &str) -> String {
    let path = Path::new(rel);
    let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
    if stem == "mod" {
        path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or(stem)
    } else {
        stem
    }
}

impl Graph {
    /// `skip_dirs`: folders (relative to `runtime_dir`) that are not scanned: package 6's own seam-guarded areas.
    pub fn scan(runtime_dir: &Path, skip_dirs: &[&str]) -> Graph {
        let mut files = Vec::new();
        rust_files(runtime_dir, &mut files);
        files.retain(|path| {
            let rel = path.strip_prefix(runtime_dir).unwrap().to_string_lossy().replace('\\', "/");
            !skip_dirs.iter().any(|dir| rel.starts_with(&format!("{dir}/")))
        });
        // Modules declared `#[cfg(feature = "diag")] mod x;` are Eden-only code: never in the shipped build, so not scanned.
        let diag_only: Vec<PathBuf> = files
            .iter()
            .flat_map(|path| {
                let toks = lex_attrs(&fs::read_to_string(path).unwrap());
                let dir = path.parent().unwrap().to_path_buf();
                let own = dir.join(path.file_stem().unwrap());
                toks.into_iter().flat_map(move |name| [dir.join(format!("{name}.rs")), dir.join(&name).join("mod.rs"), own.join(format!("{name}.rs")), own.join(&name).join("mod.rs")]).collect::<Vec<_>>()
            })
            .collect();
        files.retain(|path| !diag_only.contains(path));
        let mut funcs = Vec::new();
        let mut aliases: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut imports: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for path in files {
            let rel = path.strip_prefix(runtime_dir).unwrap().to_string_lossy().replace('\\', "/");
            let toks = lex(&fs::read_to_string(&path).unwrap());
            // use statements: aliases and imported names.
            let mut i = 0;
            while i < toks.len() {
                if ident(&toks[i]) == Some("use") {
                    let end = (i..toks.len()).find(|&k| is_punct(&toks[k], ';')).unwrap_or(toks.len() - 1);
                    let stmt = &toks[i + 1..end];
                    for (k, t) in stmt.iter().enumerate() {
                        if let Some(name) = ident(t) {
                            imports.entry(rel.clone()).or_default().insert(name.to_owned());
                            if name == "as" {
                                if let (Some(Tok::Ident(alias)), true) = (stmt.get(k + 1), k > 0) {
                                    if let Some(Tok::Ident(target)) = stmt.get(k - 1) {
                                        aliases.entry(rel.clone()).or_default().insert(alias.clone(), target.clone());
                                    }
                                }
                            }
                        }
                    }
                    i = end + 1;
                } else {
                    i += 1;
                }
            }
            let mut parser = Parser { file: &rel, funcs: Vec::new(), in_macro: false };
            parser.items(&toks, &module_of_file(&rel));
            funcs.extend(parser.funcs);
        }
        // `prob_hooks! { name = OFFSET, n; ... }`: the macro's template becomes one hook function per line.
        let templates: Vec<Func> = funcs.iter().filter(|f| f.name.starts_with('$') && f.attrs.contains("skyline::hook")).cloned().collect();
        for template in templates {
            let file = template.file.clone();
            let toks = lex(&fs::read_to_string(runtime_dir.join(&file)).unwrap());
            // Find `prob_hooks ! {` invocations (not the definition) and read `name = CONST , literal ;`.
            for i in 0..toks.len() {
                if ident(&toks[i]) == Some("prob_hooks") && toks.get(i + 1).is_some_and(|t| is_punct(t, '!')) && toks.get(i + 2).is_some_and(|t| is_punct(t, '{')) && i > 0 && ident(&toks[i - 1]) != Some("macro_rules") {
                    let close = matching(&toks, i + 2);
                    let mut k = i + 3;
                    while k + 2 < close {
                        if let (Some(name), true, Some(offset)) = (ident(&toks[k]), is_punct(&toks[k + 1], '='), ident(&toks[k + 2])) {
                            let mut f = template.clone();
                            f.name = name.to_owned();
                            f.attrs = format!("skyline::hook(offset = {offset})");
                            funcs.push(f);
                            k += 3;
                        } else {
                            k += 1;
                        }
                    }
                }
            }
        }
        funcs.retain(|f| !f.name.starts_with('$'));
        let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, f) in funcs.iter().enumerate() {
            by_name.entry(f.name.clone()).or_default().push(i);
        }
        Graph { funcs, aliases, imports, by_name, dynamic: Vec::new() }
    }

    pub fn find(&self, key: &str) -> Vec<usize> {
        self.funcs.iter().enumerate().filter(|(_, f)| f.key() == key).map(|(i, _)| i).collect()
    }

    /// The functions a path or a method call in `from` can mean.
    fn resolve(&self, from: &Func, segments: &[String], method: bool) -> Vec<usize> {
        let Some(last) = segments.last() else { return Vec::new() };
        let Some(candidates) = self.by_name.get(last) else { return Vec::new() };
        if method {
            return candidates.iter().copied().filter(|&index| self.funcs[index].has_receiver).collect();
        }
        if segments.len() >= 2 {
            let mut module = segments[segments.len() - 2].as_str();
            let alias = self.aliases.get(&from.file).and_then(|a| a.get(module));
            if let Some(target) = alias {
                module = target;
            }
            if module == "self" || module == "Self" || module == "super" || module == "crate" {
                return candidates.iter().copied().filter(|&c| self.funcs[c].file == from.file).collect();
            }
            return candidates.iter().copied().filter(|&c| self.funcs[c].module == module).collect();
        }
        let local: Vec<usize> = candidates.iter().copied().filter(|&c| self.funcs[c].file == from.file).collect();
        if !local.is_empty() {
            return local;
        }
        if self.imports.get(&from.file).is_some_and(|i| i.contains(last)) {
            return candidates.clone();
        }
        Vec::new()
    }

    /// Every call or reference in a function body: `(path segments, is a method call)`.
    pub fn references(f: &Func) -> Vec<(Vec<String>, bool)> {
        let toks = &f.body;
        let mut out = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            if let Tok::Ident(first) = &toks[i] {
                let mut method = i > 0 && is_punct(&toks[i - 1], '.');
                let mut segments = vec![first.clone()];
                // `self.name(` is a method of this file's own type (a receiver that is `self` is not another module's).
                let on_self = method && i > 1 && matches!(&toks[i - 2], Tok::Ident(s) if s == "self");
                if on_self {
                    segments.insert(0, "Self".to_owned());
                    method = false;
                }
                let mut k = i + 1;
                while k + 1 < toks.len() && toks[k] == Tok::PathSep {
                    match &toks[k + 1] {
                        Tok::Ident(next) => {
                            segments.push(next.clone());
                            k += 2;
                        }
                        // turbofish `::<T>`: skip the angle group.
                        Tok::Punct('<') => {
                            let mut depth = 0;
                            while k + 1 < toks.len() {
                                if is_punct(&toks[k + 1], '<') {
                                    depth += 1;
                                } else if is_punct(&toks[k + 1], '>') {
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
                let called = toks.get(k).is_some_and(|t| is_punct(t, '('));
                // A bare name counts as a call only when called; a qualified path counts as a reference (a
                // callback handed over: `ui::hud::register(hud_draw::draw)`), a method only when called.
                if (segments.len() == 1 && called) || (segments.len() >= 2) || (method && called) {
                    out.push((segments, method));
                }
                i = k.max(i + 1);
            } else {
                i += 1;
            }
        }
        out
    }

    pub fn callees(&self, index: usize) -> BTreeSet<usize> {
        let from = &self.funcs[index];
        let mut out = BTreeSet::new();
        for (segments, method) in Self::references(from) {
            for target in self.resolve(from, &segments, method) {
                if target != index {
                    out.insert(target);
                }
            }
        }
        for (a, b) in &self.dynamic {
            if *a == from.key() {
                out.extend(self.find(b));
            }
        }
        out
    }

    /// Every constant named `*_OFFSET` in a function body (the game offsets it uses), by last path segment.
    pub fn offset_consts(f: &Func) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for (segments, _) in Self::references(f) {
            if let Some(last) = segments.last() {
                if last.ends_with("_OFFSET") {
                    out.insert(last.clone());
                }
            }
        }
        // A bare constant (not followed by `(`) is not in `references`; scan the tokens as well.
        for t in &f.body {
            if let Tok::Ident(s) = t {
                if s.ends_with("_OFFSET") && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
                    out.insert(s.clone());
                }
            }
        }
        out
    }
}

// ---- constants of the model ------------------------------------------------------------------------------

/// `NAME -> value` of every `const NAME: usize = <number>;` in the model sources.
pub fn model_offsets(model_dir: &Path) -> BTreeMap<String, usize> {
    let mut files = Vec::new();
    rust_files(model_dir, &mut files);
    let mut out = BTreeMap::new();
    for path in files {
        let toks = lex(&fs::read_to_string(&path).unwrap());
        for i in 0..toks.len() {
            if ident(&toks[i]) == Some("const") {
                if let (Some(name), true, Some("usize"), true, Some(value)) = (
                    toks.get(i + 1).and_then(ident),
                    toks.get(i + 2).is_some_and(|t| is_punct(t, ':')),
                    toks.get(i + 3).and_then(ident),
                    toks.get(i + 4).is_some_and(|t| is_punct(t, '=')),
                    toks.get(i + 5).and_then(ident),
                ) {
                    if toks.get(i + 6).is_some_and(|t| is_punct(t, ';')) {
                        let digits = value.replace('_', "");
                        let parsed = match digits.strip_prefix("0x") {
                            Some(hex) => usize::from_str_radix(hex, 16).ok(),
                            None => digits.parse::<usize>().ok(),
                        };
                        if let Some(v) = parsed {
                            out.insert(name.to_owned(), v);
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod receiver_tests {
    use super::*;

    fn fixture() -> Graph {
        let mut parser = Parser { file: "fixture.rs", funcs: Vec::new(), in_macro: false };
        parser.items(&lex(r#"
            fn entry(value: &Runtime) { value.finish(); }
            fn direct() { finish(); }
            fn qualified() { fixture::finish(); }
            fn finish() { unrelated_native(); }
            fn unrelated_native() {}
            fn real_native() {}
            fn other_native() {}
            fn associated_native() {}
            impl Runtime { fn finish(&self) { real_native(); } }
            impl Other { fn finish(self: &Self) { other_native(); } }
            impl Associated { fn finish() { associated_native(); } }
        "#), "fixture");
        let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, func) in parser.funcs.iter().enumerate() {
            by_name.entry(func.name.clone()).or_default().push(index);
        }
        Graph { funcs: parser.funcs, aliases: BTreeMap::new(), imports: BTreeMap::new(), by_name, dynamic: Vec::new() }
    }

    fn reachable_names(graph: &Graph, entry: &str) -> BTreeSet<String> {
        let mut pending = graph.find(&format!("fixture::{entry}"));
        assert_eq!(pending.len(), 1);
        let mut seen = BTreeSet::new();
        while let Some(index) = pending.pop() {
            if seen.insert(index) {
                pending.extend(graph.callees(index));
            }
        }
        seen.into_iter().map(|index| graph.funcs[index].name.clone()).collect()
    }

    #[test]
    fn dot_call_excludes_unrelated_free_and_associated_functions() {
        let graph = fixture();
        let reachable = reachable_names(&graph, "entry");
        assert!(!reachable.contains("unrelated_native"));
        assert!(!reachable.contains("associated_native"));
    }

    #[test]
    fn dot_call_retains_all_ambiguous_receivers_and_their_native_dependencies() {
        let graph = fixture();
        let entry = graph.find("fixture::entry")[0];
        let targets = graph.callees(entry);
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|&index| graph.funcs[index].has_receiver));
        let reachable = reachable_names(&graph, "entry");
        assert!(reachable.contains("real_native"));
        assert!(reachable.contains("other_native"));
    }

    #[test]
    fn direct_and_qualified_calls_still_reach_free_functions() {
        let graph = fixture();
        for entry in ["direct", "qualified"] {
            assert!(reachable_names(&graph, entry).contains("unrelated_native"), "{entry}");
        }
    }
}
