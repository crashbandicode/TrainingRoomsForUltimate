//! The package-4 carried files are generated: `tools/carry_pkg4.py verify --dest-only` compares every carried file's sha256
//! with the manifest (`docs/provenance/pkg4-carry-manifest.json`), so a hand edit fails here. With the fork stage present,
//! `verify` also re-derives each file from the stage; that run is made by hand at each re-sync. The ledger table is generated
//! from the manifest and must be current.

use std::process::Command;

fn run(args: &[&str]) -> Option<std::process::Output> {
    let root = env!("CARGO_MANIFEST_DIR");
    Command::new("python3").current_dir(root).arg("-I").args(args).output().ok() // no python on this host: the check runs where there is one
}

#[test]
fn carried_files_match_the_manifest() {
    let Some(out) = run(&["tools/carry_pkg4.py", "verify", "--dest-only"]) else { return };
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}

#[test]
fn the_ledger_is_generated_from_the_manifest() {
    let root = env!("CARGO_MANIFEST_DIR");
    let before = std::fs::read_to_string(format!("{root}/tools/provenance-data/pkg4-carry-ledger.txt")).expect("ledger present");
    let Some(out) = run(&["tools/carry_pkg4.py", "ledger"]) else { return };
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let after = std::fs::read_to_string(format!("{root}/tools/provenance-data/pkg4-carry-ledger.txt")).unwrap();
    assert_eq!(before, after, "run `python3 -I tools/carry_pkg4.py ledger` and commit the result");
}

/// `tools/carry_edits.py` writes manifest edits from a base and a wanted file; its host tests replay every edit
/// through this tool's own `apply_edits`.
#[test]
fn the_edit_writer_passes_its_host_tests() {
    let Some(out) = run(&["tools/test_carry_edits.py"]) else { return };
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}
