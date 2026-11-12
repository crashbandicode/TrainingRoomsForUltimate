//! rc.9 coverage (docs/COVERAGE-rc9.md): the runner of the table-driven tests. A module's cases are the test functions it
//! had; `tools/test_cut.py` lists them in one `#[test] fn case_table()`. Each case runs on its own thread, as the test
//! harness ran it (so thread-local state never leaks from one case into the next), every case runs even after one fails,
//! and the failure names each failed case.
//!
//! For per-case coverage (`tools/case_cover.py`): `TR_TABLE_LIST=1` prints `TABLE-CASE <file> <test thread> <case>` for
//! every row and runs none; `TR_TABLE_CASE=<case>` runs only the rows of that name.

use std::any::Any;

fn message(payload: &(dyn Any + Send)) -> &str {
    payload.downcast_ref::<&str>().copied().or_else(|| payload.downcast_ref::<String>().map(String::as_str)).unwrap_or("")
}

/// Runs every case; panics with the names and messages of the failed ones.
#[track_caller]
pub fn run(cases: &[(&'static str, fn())]) {
    let only = std::env::var("TR_TABLE_CASE").ok();
    if std::env::var_os("TR_TABLE_LIST").is_some() {
        let file = std::panic::Location::caller().file();
        let table = std::thread::current().name().unwrap_or("?").to_owned();
        for (name, _) in cases {
            println!("TABLE-CASE\t{file}\t{table}\t{name}");
        }
        return;
    }
    let mut failed = Vec::new();
    for (name, case) in cases.iter().filter(|(name, _)| only.as_deref().is_none_or(|only| only == *name)) {
        let thread = std::thread::Builder::new().name((*name).to_owned()).spawn(*case).expect("a thread for the case");
        if let Err(payload) = thread.join() {
            failed.push(format!("{name}: {}", message(payload.as_ref())));
        }
    }
    assert!(failed.is_empty(), "{} of {} cases failed:\n{}", failed.len(), cases.len(), failed.join("\n"));
}
