//! The carried files are generated: `tools/carry_pkg6.py verify --dest-only` compares every carried file's sha256
//! with the manifest, so a hand edit fails here. The gate also runs full `verify` against the fork stage on this
//! machine before each test step. The ledger is generated from the manifest and compared here.

use std::process::Command;

#[test]
fn carried_files_match_the_manifest() {
    let root = env!("CARGO_MANIFEST_DIR");
    let out = match Command::new("python3").current_dir(root).args(["-I", "tools/carry_pkg6.py", "verify", "--dest-only"]).output() {
        Ok(out) => out,
        Err(_) => return, // no python on this host: the same check runs on the hosts that have it
    };
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}

#[test]
fn the_ledger_is_generated_from_the_manifest() {
    let root = env!("CARGO_MANIFEST_DIR");
    let path = format!("{root}/docs/provenance/pkg6-carry-ledger.md");
    let before = std::fs::read_to_string(&path).expect("ledger present");
    let out = match Command::new("python3").current_dir(root).args(["-I", "tools/carry_pkg6.py", "ledger"]).output() {
        Ok(out) => out,
        Err(_) => return, // no python on this host: the same check runs on the hosts that have it
    };
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let after = std::fs::read_to_string(&path).unwrap();
    if before != after {
        std::fs::write(&path, &before).unwrap(); // leave the tree as it was found
    }
    assert_eq!(before, after, "run `python3 -I tools/carry_pkg6.py ledger` and commit the result");
}

#[test]
fn the_carry_tools_pass_their_host_tests() {
    let root = env!("CARGO_MANIFEST_DIR");
    let out = match Command::new("python3").current_dir(root).args(["-I", "tools/test_carry_tools.py"]).output() {
        Ok(out) => out,
        Err(_) => return,
    };
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}