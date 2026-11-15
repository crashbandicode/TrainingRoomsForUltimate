//! Docs gates (ordered 2026-11-15): `tools/docs_check.py` reads `README.md` and `docs/**/*.md` and fails on a
//! relative link or image that points nowhere, an `#anchor` the forge would not find, a page no link reaches, a
//! `docs/CURRENT-RELEASE.md` without its one `Current release: rc.N` line, an entry point that does not name that release,
//! a "newest" or "current" sentence that names another one, and a quoted notice or refusal text that is not a string literal
//! of the code the release build compiles (test, simulator and diag-only code do not count). The rules, the forge's slug rules and the allowlists (each entry with its reason) are at the top of that file.
//!
//! The fixtures (`tools/test_docs_check.py`, `tools/test_check_external_links.py`) prove each rule fails on a seeded
//! break and passes on the clean case; the external-link script's tests use a server on 127.0.0.1 only. The network check
//! itself (`tools/check_external_links.py`) is a release step and is not run here (docs/BUILD.md, "Release checklist").
//!
//! Like the size gate (`tests/size_budget.rs`) these shell out to `/usr/bin/python3` and have no skip switch. Neither this
//! file nor the scripts are counted by the size gate (`tools/sloc.py` leaves `tests/` out and counts Rust only).

use std::process::Command;

fn python(script: &str) {
    let path = format!("{}/tools/{script}", env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("/usr/bin/python3")
        .args(["-I", &path])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap_or_else(|error| panic!("the docs gates need /usr/bin/python3: {error}"));
    let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{script} failed:\n{text}");
}

#[test]
fn the_docs_checks_fail_on_every_seeded_break() {
    python("test_docs_check.py");
}

#[test]
fn the_external_link_script_reports_every_status_it_knows() {
    python("test_check_external_links.py");
}

#[test]
fn the_docs_have_no_broken_link_anchor_orphan_or_stale_release() {
    python("docs_check.py");
}
