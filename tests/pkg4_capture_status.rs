//! ui-16 regression: enabled next-record preference is not active-record arming.
#[test]
fn capture_acceptance_distinguishes_preference_from_native_record() {
    let output = std::process::Command::new("/usr/bin/python3")
        .args(["-I", "-B", concat!(env!("CARGO_MANIFEST_DIR"), "/tests/eden/test_capture_status.py")])
        .output()
        .expect("capture acceptance tests require Python");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
