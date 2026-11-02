//! Runs the Python host tests of the Eden analysis scripts (`tests/eden/test_analyze.py`): the delay
//! onset measured from screenshots (`eden/analyze_probe.py`, `analyze_step.py`) on synthetic shots with a
//! planted 5-frame shift and a no-shift case, and the console.log scan (`eden/tr_eden.py`). A missing
//! Python or Pillow fails the test unless `SKIP_EDEN_SCRIPT_TESTS=1`, which prints a SKIPPED line.

use std::process::Command;

#[test]
fn the_eden_analysis_scripts_pass_their_host_tests() {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/eden/test_analyze.py");
    let output = Command::new("/usr/bin/python3").args(["-I", script]).output();
    match output {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stderr);
            if text.contains("No module named 'PIL'") && std::env::var_os("SKIP_EDEN_SCRIPT_TESTS").is_some() {
                println!("SKIPPED the_eden_analysis_scripts_pass_their_host_tests: Pillow missing");
                return;
            }
            panic!("python tests failed:\n{text}");
        }
        Err(error) => {
            assert!(std::env::var_os("SKIP_EDEN_SCRIPT_TESTS").is_some(), "no /usr/bin/python3: {error}");
            println!("SKIPPED the_eden_analysis_scripts_pass_their_host_tests: {error}");
        }
    }
}
