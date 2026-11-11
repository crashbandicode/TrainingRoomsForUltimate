//! rc.8 "Add New…" hang (worker A's finding A3, 2026-11-11): the game's MainThread spun inside std's `sync_channel`
//! `try_recv` (the Mii close hand-off, polled by the native service), waiting for the menu worker to finish a `try_send`
//! the MainThread had preempted on core 0. std's mpsc/mpmc channels spin on a peer that is mid-operation in both
//! directions (`start_recv` on a claimed, unpublished slot; `start_send` on a slot still being read), and a game thread
//! reaches every channel the plugin has, so the plugin's non-test code uses none of them: `model::nowait_channel` has the
//! same surface and returns "not ready" instead.
//!
//! `tests/hook_locks.rs` covers named locks; this covers the channel class in the source, and
//! `tools/game_thread_wait_check.py` covers the built ELF (everything a hook reaches, dependencies included).
//!
//! `GAME_THREAD_SCAN_ROOT=<repository>` scans another checkout (the proof run against `eca14e4b`, which must fail).
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// What may not appear in non-test code, and why.
const FORBIDDEN: &[(&str, &str)] = &[
    ("sync::mpsc", "std's mpsc channel spins on a peer mid-operation (use model::nowait_channel)"),
    ("sync::mpmc", "std's mpmc channel spins on a peer mid-operation (use model::nowait_channel)"),
    ("crossbeam", "crossbeam's channels and backoff spin on a peer mid-operation"),
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The text with comments, string and char literals blanked (same length, newlines kept), so line numbers still hold.
fn code_only(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = b.to_vec();
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for c in &mut out[from..to] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
    };
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            let end = b[i..].iter().position(|&c| c == b'\n').map_or(b.len(), |p| i + p);
            blank(&mut out, i, end);
            i = end;
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            let (mut depth, mut j) = (1, i + 2);
            while j < b.len() && depth > 0 {
                if b[j] == b'/' && b.get(j + 1) == Some(&b'*') {
                    depth += 1;
                    j += 2;
                } else if b[j] == b'*' && b.get(j + 1) == Some(&b'/') {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            blank(&mut out, i, j);
            i = j;
        } else if b[i] == b'r' && (b.get(i + 1) == Some(&b'"') || b.get(i + 1) == Some(&b'#'))
            && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
        {
            let hashes = b[i + 1..].iter().take_while(|&&c| c == b'#').count();
            if b.get(i + 1 + hashes) != Some(&b'"') {
                i += 1;
                continue;
            }
            let close: Vec<u8> = std::iter::once(b'"').chain(std::iter::repeat_n(b'#', hashes)).collect();
            let start = i + 2 + hashes;
            let end = b[start..].windows(close.len()).position(|w| w == close.as_slice()).map_or(b.len(), |p| start + p + close.len());
            blank(&mut out, i, end);
            i = end;
        } else if b[i] == b'"' {
            let mut j = i + 1;
            while j < b.len() && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            blank(&mut out, i, (j + 1).min(b.len()));
            i = j + 1;
        } else if b[i] == b'\'' {
            // A char literal ('x', '\n', '\u{..}'); otherwise a lifetime.
            let end = if b.get(i + 1) == Some(&b'\\') {
                b[i + 2..].iter().position(|&c| c == b'\'').map(|p| i + 2 + p + 1)
            } else {
                let len = text[i + 1..].chars().next().map_or(1, char::len_utf8);
                (b.get(i + 1 + len) == Some(&b'\'')).then_some(i + 2 + len)
            };
            match end {
                Some(end) => {
                    blank(&mut out, i, end);
                    i = end;
                }
                None => i += 1,
            }
        } else {
            i += 1;
        }
    }
    String::from_utf8(out).expect("blanking keeps UTF-8")
}

/// Whether the attribute text (inside `#[...]`) gates on test alone: `cfg(test)` or `cfg(all(test, ...))`.
fn test_only(attr: &str) -> bool {
    let a: String = attr.chars().filter(|c| !c.is_whitespace()).collect();
    a == "cfg(test)" || a.starts_with("cfg(all(test,") || a.starts_with("cfg(all(test)")
}

/// The code (from `code_only`) with every test-only item blanked: the attribute, any attributes after it, and the item up
/// to its body's closing brace or its `;`.
fn without_tests(code: &str) -> String {
    let b = code.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    while let Some(p) = code[i..].find("#[") {
        let at = i + p;
        let Some(close) = matching(b, at + 1, b'[', b']') else { break };
        if !test_only(&code[at + 2..close]) {
            i = close + 1;
            continue;
        }
        let mut j = close + 1;
        loop {
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            if code[j..].starts_with("#[") {
                let Some(end) = matching(b, j + 1, b'[', b']') else { break };
                j = end + 1;
            } else {
                break;
            }
        }
        let brace = code[j..].find('{').map(|p| j + p);
        let semi = code[j..].find(';').map(|p| j + p);
        let end = match (brace, semi) {
            (Some(br), Some(s)) if s < br => s + 1,
            (Some(br), _) => matching(b, br, b'{', b'}').map_or(b.len(), |e| e + 1),
            (None, Some(s)) => s + 1,
            (None, None) => b.len(),
        };
        for c in &mut out[at..end] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
        i = end;
    }
    String::from_utf8(out).expect("blanking keeps UTF-8")
}

fn matching(b: &[u8], open_at: usize, open: u8, close: u8) -> Option<usize> {
    let mut depth = 0usize;
    for (k, &c) in b.iter().enumerate().skip(open_at) {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(k);
            }
        }
    }
    None
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// Every forbidden use in non-test code under the repository's `src/` and `crates/*/src/`: `file:line: what (why)`.
fn violations(root: &Path) -> (usize, Vec<String>) {
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    if let Ok(crates) = fs::read_dir(root.join("crates")) {
        for krate in crates.flatten() {
            rust_files(&krate.path().join("src"), &mut files);
        }
    }
    // The test-only `#[path]` attributes keep their file names in the raw text, so read them before blanking strings.
    let mut test_files = BTreeSet::new();
    let mut stripped = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).expect("readable source");
        let raw = without_tests_paths(&text, file);
        test_files.extend(raw);
        let code = without_tests(&code_only(&text));
        stripped.push((normalize(file), code));
    }
    let mut out = Vec::new();
    for (file, code) in &stripped {
        if test_files.iter().any(|t| file == t || (t.ends_with("mod.rs") && file.starts_with(t.parent().unwrap()))) {
            continue;
        }
        let flat: String = code.chars().filter(|c| !c.is_whitespace()).collect();
        for (pattern, why) in FORBIDDEN {
            for (n, line) in code.lines().enumerate() {
                let line_flat: String = line.chars().filter(|c| !c.is_whitespace()).collect();
                if line_flat.contains(pattern) {
                    out.push(format!("{}:{}: {} ({why})", rel(root, file), n + 1, pattern));
                }
            }
        }
        // `use std::sync::{.., mpsc, ..}` (or `mpmc`), possibly across lines.
        let mut from = 0;
        while let Some(p) = flat[from..].find("std::sync::{") {
            let open = from + p + "std::sync::".len();
            let close = matching(flat.as_bytes(), open, b'{', b'}').unwrap_or(flat.len() - 1);
            let group = &flat[open..=close];
            for name in ["mpsc", "mpmc"] {
                let hit = group.match_indices(name).any(|(k, _)| {
                    let before = group.as_bytes()[k - 1];
                    let after = group.as_bytes().get(k + name.len()).copied().unwrap_or(b'}');
                    (before == b'{' || before == b',') && !(after.is_ascii_alphanumeric() || after == b'_')
                });
                if hit {
                    out.push(format!("{}: std::sync::{{.., {name}, ..}} ({})", rel(root, file), FORBIDDEN[0].1));
                }
            }
            from = close + 1;
        }
    }
    (files.len(), out)
}

/// The files the test-only `#[path]` modules of `text` name (read from the raw text: the path is a string literal).
fn without_tests_paths(text: &str, file: &Path) -> BTreeSet<PathBuf> {
    // Blank only comments (keep strings) so the attribute's path survives, then walk the gated items as `without_tests`.
    let mut found = BTreeSet::new();
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if !t.starts_with("#[") || !test_only(t.trim_start_matches("#[").trim_end_matches(']')) {
            continue;
        }
        for next in lines.iter().skip(i + 1).take(4) {
            let n: String = next.chars().filter(|c| !c.is_whitespace()).collect();
            if let Some(rest) = n.strip_prefix("#[path=\"") {
                if let Some(path) = rest.split('"').next() {
                    found.insert(normalize(&file.parent().unwrap().join(path)));
                }
            } else if !n.starts_with("#[") {
                break;
            }
        }
    }
    found
}

fn rel(root: &Path, file: &Path) -> String {
    file.strip_prefix(normalize(root)).unwrap_or(file).display().to_string()
}

fn scan_root() -> PathBuf {
    std::env::var_os("GAME_THREAD_SCAN_ROOT").map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}

#[test]
fn no_plugin_code_uses_a_channel_that_spins_on_a_peer() {
    let root = scan_root();
    let (files, found) = violations(&root);
    println!("scanned {files} files under {}", root.display());
    for line in &found {
        println!("{line}");
    }
    assert!(files > 100, "the scan found only {files} files under {}", root.display());
    assert!(found.is_empty(), "{} use(s) of a channel that spins on a peer:\n{}", found.len(), found.join("\n"));
}

#[test]
fn the_scanner_sees_through_comments_strings_and_test_modules() {
    let code = "// std::sync::mpsc in a comment\nlet s = \"std::sync::mpsc\";\n#[cfg(test)]\nmod tests {\n    use std::sync::mpsc;\n}\n";
    let stripped = without_tests(&code_only(code));
    assert!(!stripped.contains("mpsc"), "{stripped}");
    assert_eq!(stripped.lines().count(), code.lines().count());
    let live = "use std::sync::{\n    atomic::AtomicU8,\n    mpsc::{self, Receiver},\n};\n";
    let flat: String = without_tests(&code_only(live))
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert!(flat.contains("{atomic::AtomicU8,mpsc::"), "{flat}");
    let gated_any = "#[cfg(any(feature = \"diag\", test))]\nuse std::sync::mpsc;\n";
    assert!(without_tests(&code_only(gated_any)).contains("mpsc"));
}
