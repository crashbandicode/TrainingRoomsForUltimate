//! rc.9 coverage (docs/COVERAGE-rc9.md): the runner of the table-driven tests. A module's cases are the test functions it
//! had; `tools/test_cut.py` lists them in one `#[test] fn case_table()`. Each case runs on its own thread, as the test
//! harness ran it (so thread-local state never leaks from one case into the next), every case runs even after one fails,
//! and the failure names each failed case.

use std::any::Any;

fn message(payload: &(dyn Any + Send)) -> &str {
    payload.downcast_ref::<&str>().copied().or_else(|| payload.downcast_ref::<String>().map(String::as_str)).unwrap_or("")
}

/// Runs every case; panics with the names and messages of the failed ones.
pub fn run(cases: &[(&'static str, fn())]) {
    let mut failed = Vec::new();
    for (name, case) in cases {
        let thread = std::thread::Builder::new().name((*name).to_owned()).spawn(*case).expect("a thread for the case");
        if let Err(payload) = thread.join() {
            failed.push(format!("{name}: {}", message(payload.as_ref())));
        }
    }
    assert!(failed.is_empty(), "{} of {} cases failed:\n{}", failed.len(), cases.len(), failed.join("\n"));
}