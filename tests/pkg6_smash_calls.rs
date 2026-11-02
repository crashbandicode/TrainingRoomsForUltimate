//! The guarded-call cross-check (playbook "every called function guarded", lead ruling 2026-11-02): every game function the carried
//! runtime sources call through the `smash` crate (`WorkModule::get_int`, `FighterManager::...`, `utility::get_kind`, ...) must map
//! to a row of a site table that carries its 64-byte guard. The list of calls is generated from the sources at test time, so a
//! re-sync that adds a call fails here until the call is mapped to a guarded row (a new row in `sites_pkg6.rs` or an existing one).

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::collections::BTreeSet;
use std::path::Path;

use model::{sites, sites_pkg2, sites_pkg3, sites_pkg4, sites_pkg6, sites_ui};

/// `smash` call (as written in the source) -> the guarded row that covers it.
const COVERED: &[(&str, &str)] = &[
    ("AttackModule::is_infliction_status", "p1_attack_is_infliction"),
    ("DamageModule::damage", "damage_of"),
    ("PostureModule::pos_x", "pos_x"),
    ("PostureModule::pos_y", "pos_y"),
    ("StatusModule::status_kind", "status_kind"),
    ("StatusModule::prev_status_kind", "prev_status_kind"),
    ("WorkModule::get_int", "work_get_int"),
    ("FighterManager::entry_count", "p1_manager_entry_count"),
    ("FighterManager::get_fighter_entry", "fighter_manager_get_entry"),
    ("FighterManager::is_ready_go", "p1_manager_is_ready_go"),
    ("FighterManager::is_result_mode", "p1_manager_is_result_mode"),
    ("FighterEntry::current_fighter_id", "p1_entry_current_fighter_id"),
    ("utility::get_kind", "state_object_kind"),
    ("sv_battle_object::module_accessor", "state_object_module_accessor"),
];

fn all_rows() -> BTreeSet<&'static str> {
    sites::BASE_SITES
        .iter()
        .chain(sites_pkg2::SITES_PKG2.iter())
        .chain(sites_pkg3::STATE_SITES.iter())
        .chain(sites_pkg4::SITES.iter())
        .chain(sites_pkg6::SITES.iter())
        .chain(sites_ui::UI_SITES.iter())
        .map(|site| site.name)
        .collect()
}

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The `Type::function` calls of one source that name a game function: a path ending in `...Module`, `FighterManager`, `FighterEntry`,
/// `utility` or `sv_battle_object`, then `::name` and an opening parenthesis.
fn smash_calls(text: &str) -> BTreeSet<String> {
    let mut calls = BTreeSet::new();
    let owners = ["Module", "FighterManager", "FighterEntry", "utility", "sv_battle_object"];
    for line in text.lines() {
        let code = line.split("//").next().unwrap_or("");
        let bytes = code.as_bytes();
        let mut i = 0;
        while let Some(at) = code[i..].find("::") {
            let sep = i + at;
            let start = code[..sep].rfind(|c: char| !(c.is_alphanumeric() || c == '_')).map_or(0, |p| p + 1);
            let owner = &code[start..sep];
            let rest = &code[sep + 2..];
            let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
            let name = &rest[..end];
            let called = rest[end..].starts_with('(');
            if called && !name.is_empty() && owners.iter().any(|o| owner.ends_with(o)) && owner.chars().next().is_some_and(|c| c.is_alphabetic()) {
                // `Vec::new`-style std paths never end in these owners, so no further filter is needed.
                calls.insert(format!("{owner}::{name}"));
            }
            i = sep + 2;
            if i >= bytes.len() {
                break;
            }
        }
    }
    calls
}

#[test]
fn every_covered_row_exists() {
    let rows = all_rows();
    for (call, row) in COVERED {
        assert!(rows.contains(row), "{call} is mapped to {row}, which is not a row of any site table");
    }
}

#[test]
fn every_smash_call_of_the_carried_runtime_sources_is_covered_by_a_guarded_row() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    let covered: BTreeSet<&str> = COVERED.iter().map(|(call, _)| *call).collect();
    let mut scanned = 0;
    let mut calls = BTreeSet::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        // Only sources that use the `smash` crate; the other packages reach the game through guarded raw offsets.
        if !text.contains("smash::") {
            continue;
        }
        scanned += 1;
        for call in smash_calls(&text) {
            calls.insert((file.strip_prefix(&root).unwrap().display().to_string(), call));
        }
    }
    let unguarded: Vec<_> = calls.iter().filter(|(_, call)| !covered.contains(call.as_str())).collect();
    assert!(unguarded.is_empty(), "smash calls without a guarded row (add rows to sites_pkg6.rs and COVERED): {unguarded:?}");
    assert!(scanned >= 1, "the scan found no source that uses the smash crate");
}

#[test]
fn the_scanner_finds_the_calls_it_is_meant_to_find() {
    let source = "let x = WorkModule::get_int(m, id); // StatusModule::status_kind(m)\nlet y = smash::app::utility::get_kind(&mut *m);\nlet z = Vec::new();\nFighterManager::entry_count(manager)";
    let calls = smash_calls(source);
    assert!(calls.contains("WorkModule::get_int"));
    assert!(calls.contains("utility::get_kind"));
    assert!(calls.contains("FighterManager::entry_count"));
    assert!(!calls.contains("StatusModule::status_kind"), "a commented call is not a call");
    assert!(!calls.iter().any(|c| c.starts_with("Vec")));
}