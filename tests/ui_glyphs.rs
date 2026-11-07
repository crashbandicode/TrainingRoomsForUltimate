//! Every character the plugin can put on screen is one the game's font draws (rc.5 review: the Mii list's "✓ " marker,
//! U+2713, rendered as an empty box in `docs/media/mii-editor-list.gif`).
//!
//! The menu's text panes use the layout font `nintendo64` (`tools/layout-builder`), the game's Latin UI face. Its
//! character map was read from the stock data.arc (`artifacts/phase23/rc5-combined/glyphs/font_cmap.py`: the three
//! `nintendoP_RodinNTLG` weights, B003/EB003/UB003, 8,612 characters each): U+2713 and U+2714 are in none of the game's
//! 14 fonts. `FONT_SAFE` lists the non-ASCII characters this source uses that all three weights map, and only those.
//!
//! Scope: every string and char literal in `src/**/*.rs` outside `#[cfg(test)]` code (test-only files and
//! `#[cfg(test)] mod … { … }` blocks are skipped: their CJK and emoji are test inputs, never drawn). That is wider than the
//! UI strings (log lines are included), which keeps the rule simple: a log line can use the same characters.

use std::fs;
use std::path::{Path, PathBuf};

/// Non-ASCII characters in the cmap of every `nintendoP_RodinNTLG` weight in 13.0.5's data.arc (font_cmap.py), and used here.
const FONT_SAFE: &[char] = &[
    '\u{2026}', // … HORIZONTAL ELLIPSIS ("Add New…")
    '\u{00B7}', // · MIDDLE DOT (footers: "A Target · X Rename")
    '\u{03A9}', // Ω GREEK CAPITAL OMEGA (stage forms)
    '\u{2014}', // — EM DASH (the Mii list's title)
    '\u{00A7}', // § SECTION SIGN
    '\u{00E9}', // é
    '\u{2192}', // → RIGHTWARDS ARROW. All three nintendoP_RodinNTLG weights map it (bfotf-2c46f6bb68, bfotf-2d24073d3a, bfotf-2d58bc0c11).
    '\u{221E}', // ∞ INFINITY. The same three weights map it.
];

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// A file that only test builds include (`#[cfg(test)] #[path = "…_tests.rs"] mod …;` or `mod tests;`).
fn test_only(path: &Path) -> bool {
    let name = path.file_name().unwrap().to_string_lossy();
    name == "tests.rs" || name.ends_with("_tests.rs") || path.components().any(|c| c.as_os_str() == "tests")
}

/// The literals of `src` outside comments and `#[cfg(test)]` blocks: (line, text).
fn literals(src: &str) -> Vec<(usize, String)> {
    let b: Vec<char> = src.chars().collect();
    let (mut i, mut line, mut out) = (0, 1, Vec::new());
    let mut skip_depth: Option<usize> = None; // brace depth at which a #[cfg(test)] block ends
    let mut depth = 0usize;
    let mut pending_test = false;
    let starts = |i: usize, s: &str| s.chars().enumerate().all(|(k, c)| b.get(i + k) == Some(&c));
    while i < b.len() {
        let c = b[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if starts(i, "//") {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else if starts(i, "/*") {
            let mut nest = 0;
            while i < b.len() {
                if starts(i, "/*") {
                    nest += 1;
                    i += 2;
                } else if starts(i, "*/") {
                    nest -= 1;
                    i += 2;
                    if nest == 0 {
                        break;
                    }
                } else {
                    if b[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
        } else if starts(i, "#[cfg(test)]") {
            pending_test = true;
            i += "#[cfg(test)]".len();
        } else if c == '{' {
            depth += 1;
            if pending_test && skip_depth.is_none() {
                skip_depth = Some(depth);
            }
            pending_test = false;
            i += 1;
        } else if c == '}' {
            if skip_depth == Some(depth) {
                skip_depth = None;
            }
            depth = depth.saturating_sub(1);
            i += 1;
        } else if c == ';' {
            pending_test = false; // `#[cfg(test)] mod x;` or a cfg'd item without a body
            i += 1;
        } else if c == 'r' && (b.get(i + 1) == Some(&'"') || b.get(i + 1) == Some(&'#')) && !b.get(i.wrapping_sub(1)).is_some_and(|p| p.is_alphanumeric() || *p == '_') {
            let mut hashes = 0;
            let mut k = i + 1;
            while b.get(k) == Some(&'#') {
                hashes += 1;
                k += 1;
            }
            if b.get(k) != Some(&'"') {
                i += 1;
                continue;
            }
            let close: String = std::iter::once('"').chain(std::iter::repeat_n('#', hashes)).collect();
            let start_line = line;
            let mut text = String::new();
            k += 1;
            while k < b.len() && !starts(k, &close) {
                if b[k] == '\n' {
                    line += 1;
                }
                text.push(b[k]);
                k += 1;
            }
            if skip_depth.is_none() {
                out.push((start_line, text));
            }
            i = k + close.len();
        } else if c == '"' {
            let start_line = line;
            let mut text = String::new();
            let mut k = i + 1;
            while k < b.len() && b[k] != '"' {
                if b[k] == '\\' {
                    text.push(b[k]);
                    k += 1;
                }
                if b[k] == '\n' {
                    line += 1;
                }
                text.push(b[k]);
                k += 1;
            }
            if skip_depth.is_none() {
                out.push((start_line, text));
            }
            i = k + 1;
        } else if c == '\'' && b.get(i + 2) == Some(&'\'') {
            if skip_depth.is_none() {
                out.push((line, b[i + 1].to_string()));
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    out
}

#[test]
fn every_drawn_character_is_in_the_game_font() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut all = Vec::new();
    files(&root, &mut all);
    all.sort();
    let (mut bad, mut used) = (Vec::new(), Vec::new());
    for path in all.iter().filter(|p| !test_only(p)) {
        let src = fs::read_to_string(path).unwrap();
        for (line, text) in literals(&src) {
            used.extend(text.chars().filter(|c| FONT_SAFE.contains(c)));
            for ch in text.chars().filter(|c| !c.is_ascii() && !FONT_SAFE.contains(c)) {
                bad.push(format!("{}:{line}: U+{:04X} {ch:?} in {text:?}", path.strip_prefix(&root).unwrap().display(), ch as u32));
            }
        }
    }
    assert!(bad.is_empty(), "characters the game's font has no glyph for (it draws an empty box):\n{}", bad.join("\n"));
    // The list holds only what the source uses: a new character is proven against the font's cmap and added here.
    let unused: Vec<char> = FONT_SAFE.iter().copied().filter(|c| !used.contains(c)).collect();
    assert!(unused.is_empty(), "FONT_SAFE entries no literal uses: {unused:?}");
}

#[test]
fn the_scanner_sees_literals_and_skips_test_code() {
    let src = "fn a() { let s = \"✓ x\"; } // \"✓ comment\"\n#[cfg(test)]\nmod t { fn b() { let s = \"✓ test\"; } }\nfn c() { let r = r#\"Ω\"#; }\n";
    let got = literals(src);
    assert_eq!(got, vec![(1, "✓ x".to_string()), (4, "Ω".to_string())]);
}
