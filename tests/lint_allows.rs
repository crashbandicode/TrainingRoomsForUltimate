//! Lint suppressions are workarounds (playbook WORKFLOW "Root cause, or the operator's approval"): every
//! `allow(` / `#[expect(` in `src/` and `crates/` (vendored `third_party/` excluded) must be in `KNOWN` below, file
//! by file and lint by lint with its exact count, and `Cargo.toml` may set no lint to `allow`. A new suppression
//! fails this test; so does a `KNOWN` row whose suppressions are gone, so the table only shrinks.
//!
//! No row has a recorded operator approval today (2026-11-02). The rows below are the remaining carried ones
//! (decision D9 accepted them narrowly for byte-identical carried code; the lead takes each to the operator or
//! has it removed at the root with a recorded carry edit). A row gains `approved` only with the operator's
//! approval and its date.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// (file, lint, count, status).
const KNOWN: &[(&str, &str, usize, &str)] = &[
    ("src/model/macro_ui/macro_panel.rs", "clippy::large_enum_variant", 1, "pending: carried (pkg4 manifest), D9"),
    ("src/runtime/macros/replay_store.rs", "dead_code", 1, "pending: carried (pkg4 manifest), only caller behind `deluxe-sdk` (off, L1)"),
    ("src/runtime/mii/mii_backend.rs", "dead_code", 2, "pending: carried (pkg6 manifest), D9, feature-gated callers"),
    ("src/runtime/mii/mii_editor_bridge/draft.rs", "clippy::large_enum_variant", 1, "pending: carried (pkg6 manifest), D9"),
    ("src/runtime/mii/mii_hud_portrait.rs", "clippy::large_enum_variant", 1, "pending: carried (pkg6 manifest), D9"),
    ("src/runtime/mii/mii_ordinary_native.rs", "dead_code", 1, "pending: carried (pkg6 manifest), D9, feature-gated callers"),
    ("src/runtime/mii/native_keyboard/inline_keyboard.rs", "clippy::large_enum_variant", 1, "pending: carried (pkg6 manifest), D9"),
    ("src/runtime/mii/native_keyboard/inline_keyboard.rs", "dead_code", 4, "pending: carried (pkg6 manifest), D9, feature-gated callers"),
    ("src/runtime/p1/cpu_pilot.rs", "dead_code", 1, "pending: carried (pkg6 manifest), D9, path-included pilot core"),
    ("src/runtime/roster/character_roster.rs", "clippy::large_enum_variant", 1, "pending: carried (pkg6 manifest), D9"),
    ("src/runtime/roster/character_sources.rs", "dead_code", 1, "pending: carried (pkg6 manifest), D9, feature-gated caller"),
    ("src/runtime/roster/native_fighter_install.rs", "dead_code", 2, "pending: carried (pkg6 manifest), D9, feature-gated callers"),
    ("src/runtime/roster/native_source_capacity.rs", "dead_code", 1, "pending: carried (pkg6 manifest), D9, feature-gated callers"),
];

fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() && name != "third_party" && name != "target" {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The lints named by every suppression attribute in `text`, comment lines skipped. An attribute may span lines
/// (`cfg_attr(…,\n allow(dead_code))`), so the scan runs over the whole text, not line by line.
fn suppressed_lints(text: &str) -> Vec<String> {
    let code: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .map(|line| match line.find("// ") {
            Some(at) if !line[..at].contains('"') => &line[..at],
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut lints = Vec::new();
    for marker in ["allow(", "#[expect(", "#![expect("] {
        let mut rest = code.as_str();
        while let Some(at) = rest.find(marker) {
            let preceded_by_ident = marker == "allow(" && rest[..at].chars().last().is_some_and(|c| c.is_alphanumeric() || c == '_');
            let body = &rest[at + marker.len()..];
            let end = body.find(')').unwrap_or(body.len());
            if !preceded_by_ident {
                lints.extend(body[..end].split(',').map(|lint| lint.trim().to_owned()).filter(|lint| !lint.is_empty()));
            }
            rest = &body[end..];
        }
    }
    lints
}

#[test]
fn every_lint_suppression_is_known_and_every_known_one_still_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    sources(&root.join("crates"), &mut files);
    let mut found: BTreeMap<(String, String), usize> = BTreeMap::new();
    for file in &files {
        let rel = file.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        for lint in suppressed_lints(&fs::read_to_string(file).unwrap()) {
            *found.entry((rel.clone(), lint)).or_default() += 1;
        }
    }
    let known: BTreeMap<(String, String), usize> =
        KNOWN.iter().map(|(file, lint, count, _)| (((*file).to_owned(), (*lint).to_owned()), *count)).collect();
    let mut problems = Vec::new();
    for ((file, lint), count) in &found {
        match known.get(&(file.clone(), lint.clone())) {
            None => problems.push(format!("new suppression: {file}: allow({lint}) x{count} (fix the root cause, or take it to the lead)")),
            Some(k) if k != count => problems.push(format!("{file}: allow({lint}) x{count}, KNOWN says x{k}: update the row (it only shrinks)")),
            _ => {}
        }
    }
    for (file, lint) in known.keys() {
        if !found.contains_key(&(file.clone(), lint.clone())) {
            problems.push(format!("{file}: allow({lint}) is gone: delete its KNOWN row"));
        }
    }
    assert!(problems.is_empty(), "lint suppressions:\n{}", problems.join("\n"));
}

#[test]
fn cargo_toml_sets_no_lint_to_allow() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let mut in_lints = false;
    for line in manifest.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            in_lints = line.contains("lints");
        } else if in_lints {
            assert!(!line.contains("\"allow\""), "Cargo.toml lint set to allow: {line}");
        }
    }
}

#[test]
fn the_scanner_finds_every_form() {
    let text = "#[allow(dead_code)]\n#![allow(clippy::a, clippy::b)]\n#[cfg_attr(\n    not(x),\n    allow(dead_code)\n)]\n// #[allow(not_counted)]\nfn f() { shallow(1); } // allow(nor_this)\n#[expect(unused)]\n";
    assert_eq!(suppressed_lints(text), ["dead_code", "clippy::a", "clippy::b", "dead_code", "unused"]);
}
