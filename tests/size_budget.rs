//! Code-size gate (docs/SIZE-PLAN.md): runs the host tests of the counter and the gate
//! (`tools/test_size_gate.py`), then `tools/size_gate.py`, which counts the working tree with the operator's
//! rules (`tools/sloc.py`) and fails above `size-budget.toml` or when that file records a raise without the
//! lead's approval.
//!
//! The gate shells out to Python on purpose: the count must be the operator's counter itself, the one that
//! measured the fork, not a second implementation in Rust that could drift from it. A missing Python (or one
//! older than 3.11, without `tomllib`) fails this test; there is no skip switch, because a gate that can be
//! skipped is not a gate. The interpreter is `/usr/bin/python3`, as in `tests/eden_scripts.rs`.

use std::process::Command;

fn python(script: &str) {
    let path = format!("{}/tools/{script}", env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("/usr/bin/python3")
        .args(["-I", &path])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap_or_else(|error| panic!("the size gate needs /usr/bin/python3: {error}"));
    let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{script} failed:\n{text}");
    println!("{text}");
}

#[test]
fn the_counter_and_the_gate_pass_their_host_tests() {
    python("test_size_gate.py");
}

#[test]
fn the_implementation_is_within_its_size_budget() {
    python("size_gate.py");
}
