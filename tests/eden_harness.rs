//! Runs the Python host tests of the Eden harness (`tests/eden/test_harness.py`, `eden/harness/`): the claim
//! ledger parser on real ledger lines (stamps with and without seconds, every agent's owners, per-host claims,
//! fairness, the imgui-nvn-sol interleave rule), the parameter set, `gv.py` prep/restore on a synthetic profile,
//! the console.log scan verdict, the route's step generation and gating on a scripted fake instance, and the Ally
//! adapter's log wait. No Eden is started. A missing Python fails the test unless `SKIP_EDEN_SCRIPT_TESTS=1`.

use std::process::Command;

#[test]
fn the_eden_harness_passes_its_host_tests() {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/eden/test_harness.py");
    match Command::new("/usr/bin/python3").args(["-I", script]).output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => panic!("python tests failed:\n{}", String::from_utf8_lossy(&out.stderr)),
        Err(error) => {
            assert!(std::env::var_os("SKIP_EDEN_SCRIPT_TESTS").is_some(), "no /usr/bin/python3: {error}");
            println!("SKIPPED the_eden_harness_passes_its_host_tests: {error}");
        }
    }
}
