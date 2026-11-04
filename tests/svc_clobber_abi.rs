//! Every inline `asm!` whose template contains an `svc` instruction must declare the full
//! `extern "C"` integer register set clobbered (`clobber_abi("C")`). The console kernel returns
//! from an SVC with x8–x17 zeroed as well as x0–x7, so a hand-listed `lateout("x2")…lateout("x7")`
//! lets the compiler keep a live value in x8–x17 across the SVC; on the console that value becomes
//! 0 and the next use faults (the v0.1.0 boot crash, `runtime::diag_log::console_write`).
//! Eden preserves x8–x17 across SVC returns, so only the console shows it.

use std::path::PathBuf;

fn rs_sources() -> Vec<PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(std::path::Path::new("src"), &mut out);
    out.sort();
    out
}

/// The balanced `asm!( … )` bodies of one source file, byte-based (UTF-8 safe: multi-byte
/// sequences never contain ASCII quote, brace or slash bytes, so scanning bytes is sound).
fn asm_blocks(source: &str) -> Vec<(usize, String)> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 5 <= bytes.len() {
        if !bytes[i..].starts_with(b"asm!(") {
            i += 1;
            continue;
        }
        if i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
            i += 1;
            continue;
        }
        let start = i;
        let mut depth = 1usize;
        let mut j = i + 5;
        let mut in_string = false;
        let mut block = Vec::new();
        while j < bytes.len() && depth > 0 {
            let b = bytes[j];
            if in_string {
                block.push(b);
                if b == b'"' {
                    in_string = false;
                }
                j += 1;
                continue;
            }
            match b {
                b'"' => {
                    in_string = true;
                    block.push(b);
                    j += 1;
                }
                b'(' => {
                    depth += 1;
                    block.push(b);
                    j += 1;
                }
                b')' => {
                    depth -= 1;
                    if depth > 0 {
                        block.push(b);
                    }
                    j += 1;
                }
                b'/' if j + 1 < bytes.len() && bytes[j + 1] == b'/' => {
                    while j < bytes.len() && bytes[j] != b'\n' {
                        j += 1;
                    }
                }
                _ => {
                    block.push(b);
                    j += 1;
                }
            }
        }
        out.push((start, String::from_utf8_lossy(&block).into_owned()));
        i = j;
    }
    out
}

#[test]
fn every_svc_asm_declares_the_c_abi_clobbers() {
    let mut failures = Vec::new();
    for path in rs_sources() {
        let Ok(source) = std::fs::read_to_string(&path) else { continue };
        for (offset, block) in asm_blocks(&source) {
            if !block.contains("svc") {
                continue;
            }
            let line = source[..offset].lines().count() + 1;
            if !block.contains("clobber_abi(\"C\")") {
                failures.push(format!(
                    "{}:{}: an asm! template contains `svc` but the block does not declare clobber_abi(\"C\"):\n{}",
                    path.display(),
                    line,
                    block
                ));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
