//! The carried files are generated: `tools/carry_pkg6.py verify --dest-only` compares every carried file's sha256
//! with the manifest, so a hand edit fails here. With the fork stage present (`--stage`), `verify` also re-derives
//! each file from the stage; that run is made by hand at each re-sync.

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