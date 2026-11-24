//! This file runs the host tests of the counter and the gate (docs/SIZE-PLAN.md, `tools/test_size_gate.py`).
//! The gate itself (`tools/size_gate.py`) counts the working tree with the operator's rules (`tools/sloc.py`) and fails
//! above `size-budget.toml` or when that file records a raise without the lead's approval. It is a gate step of its
//! own, run on every gated head beside this suite (docs/BUILD.md, "Size gate"), and is no longer a second test here
//! because that step runs the same script on the same tree.
//!
//! The test shells out to Python on purpose: the count must be the primary's counter itself (sloc4), the one that
//! measured the fork, not a second implementation in Rust that could drift from it. A missing Python (or one
//! older than 3.11, without `tomllib`) fails this test; there is no skip switch, because a gate that can be
//! skipped is not a gate. The interpreter is `/usr/bin/python3`.

use std::process::Command;

fn python(script: &str) {
    let path = format!("{}/tools/{script}", env!("CARGO_MANIFEST_DIR"));
    let mut command = if let Some(lock) = std::env::var_os("TR_GATE_FREEZE_LOCK") {
        // The remote gate may pause the rest of the suite for an Eden load.
        // A size scan uses a hard wall-clock timeout and must finish first.
        let mut command = Command::new("/usr/bin/flock");
        command.arg("--shared").arg(lock).arg("/usr/bin/python3");
        command
    } else {
        Command::new("/usr/bin/python3")
    };
    let output = command
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
