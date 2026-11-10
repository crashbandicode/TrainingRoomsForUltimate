//! Measured stack growth fails the build (rc.6 cov-crop; primary 2026-11-10 02:59Z). `tools/stack_marks.py` holds every row of
//! `model::thread_stacks` to the largest `[ThreadStack] high_water` mark the diag build painted in Eden, kept in
//! `tests/fixtures/stack_marks/` with its run: a row whose `measured` is below one fails. `tr-menu`'s row said 30,848 B measured
//! while the runs painted 54,960 B, and the next combo crop overflowed its 56 KiB. The tool's own host tests run first. The
//! interpreter is `/usr/bin/python3`, as in `tests/size_budget.rs`.

use std::path::Path;
use std::process::{Command, Output};

fn python(args: &[&str]) -> Output {
    Command::new("/usr/bin/python3")
        .args(["-B", "-I"])
        .args(args)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .output()
        .unwrap_or_else(|error| panic!("the stack-marks gate needs /usr/bin/python3: {error}"))
}

fn text(out: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

#[test]
fn the_stack_marks_tool_passes_its_own_tests() {
    let out = python(&["tools/test_stack_marks.py"]);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn every_row_covers_the_largest_measured_mark() {
    let out = python(&["tools/stack_marks.py", "tests/fixtures/stack_marks"]);
    assert!(
        out.status.success(),
        "a measured high water passed its row: record the run in model::thread_stacks and size the stack by the rule\n{}",
        text(&out)
    );
}
