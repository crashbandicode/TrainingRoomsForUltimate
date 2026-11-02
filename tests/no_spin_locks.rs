//! The plugin's locks never spin (the menu-close hang, 2026-11-02; crates/hos-lock/README.md): parking_lot's parker
//! on the Switch target is a pure busy loop, so parking_lot, parking_lot_core and spin-lock crates stay out of the
//! build, and the code takes its locks from hos-lock. The state machines of hos-lock run here too, so the root
//! `cargo test` covers them.

#[path = "../crates/hos-lock/src/word.rs"]
#[allow(dead_code)]
mod word;

use std::fs;
use std::path::Path;

const BANNED_PACKAGES: &[&str] = &["parking_lot", "parking_lot_core", "spin", "spinning_top", "spin_sleep"];

#[test]
fn the_lock_file_has_no_spinning_lock_crate() {
    let lock = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock")).expect("Cargo.lock");
    let names: Vec<&str> =
        lock.lines().filter_map(|l| l.strip_prefix("name = \"")).filter_map(|l| l.strip_suffix('"')).collect();
    for banned in BANNED_PACKAGES {
        assert!(!names.contains(banned), "{banned} is in Cargo.lock");
    }
    assert!(names.contains(&"hos-lock"));
}

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_source_takes_a_lock_from_parking_lot() {
    let mut files = Vec::new();
    rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    let mut hits = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if !code.starts_with("//") && code.contains("parking_lot") {
                hits.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    assert!(hits.is_empty(), "parking_lot used in {hits:?}");
}
