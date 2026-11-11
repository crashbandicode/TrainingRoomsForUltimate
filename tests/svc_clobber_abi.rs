//! Every inline `asm!` whose template contains an `svc` instruction must declare the full
//! `extern "C"` integer register set clobbered (`clobber_abi("C")`). The console kernel returns
//! from an SVC with x8–x17 zeroed as well as x0–x7, so a hand-listed `lateout("x2")…lateout("x7")`
//! lets the compiler keep a live value in x8–x17 across the SVC; on the console that value becomes
//! 0 and the next use faults (the v0.1.0 boot crash, `runtime::diag_log::console_write`).
//! Eden preserves x8–x17 across SVC returns, so only the console shows it.
//!
//! rc.7 (primary 2026-11-11 02:05Z): this gate read only `src/`, so `crates/hos-lock`'s two arbiter calls (`svc 0x34`,
//! `svc 0x35`, hand-listed x0–x7) passed it, and the interim rc.7 `b3678dfe` crashed at boot on the operator's Switch
//! (Atmosphere: Data Abort at 0, PC `+0xf9abc` in `ui::worker::run`, `svc #0x34` then `ldaxr wzr, [x10]`). It now reads
//! every crate we build: `src/`, `crates/` and the vendored `third_party/`, `asm!` and `global_asm!` alike; and it fails
//! on any `svc` in a hand-written `.S`/`.s` (such a call must save what it needs by hand; none does today, so one
//! appearing needs its own audit). `tools/svc_clobber_check.py` checks the built `.so` instruction by instruction.

use std::fs;
use std::path::{Path, PathBuf};

const ROOTS: &[&str] = &["src", "crates", "third_party"];

fn files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name != "target" && !name.starts_with('.') {
                files(&path, out);
            }
        } else if matches!(path.extension().and_then(|e| e.to_str()), Some("rs" | "S" | "s")) {
            out.push(path);
        }
    }
}

/// The text of each `asm!(…)`/`global_asm!(…)` invocation outside line comments: from the opening parenthesis to its
/// match, string literals (with escapes) skipped while counting.
fn asm_invocations(text: &str) -> Vec<(usize, String)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find("asm!") {
        let start = from + at;
        from = start + 4;
        let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        if text[line_start..start].contains("//") {
            continue;
        }
        let Some(open) = text[from..].find(|c: char| !c.is_whitespace()).map(|i| from + i) else { break };
        if bytes[open] != b'(' {
            continue;
        }
        let (mut depth, mut i, mut in_str) = (0usize, open, false);
        while i < bytes.len() {
            let c = bytes[i];
            if in_str {
                if c == b'\\' {
                    i += 1;
                } else if c == b'"' {
                    in_str = false;
                }
            } else if c == b'"' {
                in_str = true;
            } else if c == b'(' {
                depth += 1;
            } else if c == b')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            i += 1;
        }
        let line = text[..start].matches('\n').count() + 1;
        out.push((line, text[open..=i.min(bytes.len() - 1)].to_string()));
        from = i;
    }
    out
}

/// An `svc` instruction inside one of the invocation's string literals.
fn template_has_svc(body: &str) -> bool {
    let mut rest = body;
    while let Some(q) = rest.find('"') {
        let after = &rest[q + 1..];
        let Some(end) = after.find('"') else { break };
        let literal = &after[..end];
        if literal.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).any(|word| word == "svc") {
            return true;
        }
        rest = &after[end + 1..];
    }
    false
}

/// An `svc` instruction in a hand-written assembly file (comments and the text after `//`/`;`/`@` ignored).
fn asm_file_svc_lines(text: &str) -> Vec<usize> {
    text.lines().enumerate().filter_map(|(n, line)| {
        let code = line.split("//").next().unwrap_or("").split(';').next().unwrap_or("").split('@').next().unwrap_or("");
        code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).any(|w| w == "svc").then_some(n + 1)
    }).collect()
}

#[test]
fn every_svc_asm_declares_the_c_abi_clobbers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut all = Vec::new();
    for r in ROOTS {
        files(&root.join(r), &mut all);
    }
    assert!(all.len() > 100, "scanned only {} files", all.len());
    let mut svc_sites = 0;
    let mut missing = Vec::new();
    for path in &all {
        let Ok(text) = fs::read_to_string(path) else { continue };
        let rel = path.strip_prefix(root).unwrap_or(path).display().to_string();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            for (line, body) in asm_invocations(&text) {
                if template_has_svc(&body) {
                    svc_sites += 1;
                    if !body.contains("clobber_abi(\"C\")") {
                        missing.push(format!("{rel}:{line}: inline svc without clobber_abi(\"C\")"));
                    }
                }
            }
        } else {
            for line in asm_file_svc_lines(&text) {
                missing.push(format!("{rel}:{line}: svc in hand-written assembly (save x8–x17 by hand and audit it)"));
            }
        }
    }
    // hos-lock's two arbiter calls and the six in src/ as of rc.7 (2026-11-11): a drop means the scan lost them.
    assert!(svc_sites >= 8, "found only {svc_sites} inline svc sites");
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}

#[test]
fn the_scan_sees_a_missing_clobber() {
    let bad = "fn f() { unsafe { core::arch::asm!(\"svc #0x34\", inout(\"x0\") 1u64 => _, options(nostack)); } }";
    let good = "fn f() { unsafe { core::arch::asm!(\"svc #0x34\", inout(\"x0\") 1u64 => _, clobber_abi(\"C\"), options(nostack)); } }";
    let commented = "// core::arch::asm!(\"svc #0x34\")\nfn f() {}";
    let not_svc = "fn f() { unsafe { core::arch::asm!(\"mov {0}, sp\", out(reg) _); } }";
    for (src, svc, ok) in [(bad, true, false), (good, true, true), (not_svc, false, false)] {
        let found = asm_invocations(src);
        assert_eq!(found.len(), 1, "{src}");
        assert_eq!(template_has_svc(&found[0].1), svc, "{src}");
        if svc {
            assert_eq!(found[0].1.contains("clobber_abi(\"C\")"), ok, "{src}");
        }
    }
    assert!(asm_invocations(commented).is_empty());
    assert_eq!(asm_file_svc_lines("    stp x29, x30, [sp, #-16]!\n    svc #0x27\n// svc in a comment\n"), vec![2]);
}
