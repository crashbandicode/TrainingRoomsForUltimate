//! F6 (the first-boot group-build hitch): the model of the diag split (`model::build_split`) and the script that turns its log lines
//! into the evidence table (`tools/f6_split_table.py`, with its Python host tests on synthetic lines).

#[path = "../src/model/build_split.rs"]
#[allow(dead_code)]
mod build_split;

use build_split::{Gaps, Split, Step, Work};
use std::process::Command;

#[test]
fn the_split_table_script_passes_its_host_tests() {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tools/test_f6_split_table.py");
    let out = Command::new("/usr/bin/python3").args(["-I", script]).output().expect("/usr/bin/python3");
    assert!(out.status.success(), "python tests failed:\n{}", String::from_utf8_lossy(&out.stderr));
}

/// The line the runtime writes is the line the script parses (one format, two readers).
#[test]
fn the_runtime_line_format_is_the_one_the_script_reads() {
    let mut s = Split::start(0);
    s.mark(Step::Plugin, 100);
    s.mark(Step::Control, 60_100);
    s.mark(Step::Allocate, 61_000);
    let mut g = Gaps::arm(0);
    g.note(Work::Setup);
    g.observe(77_000);
    let log = format!(
        "1 [BuildSplit] setup owner=0 group=1 kinds=[2d, 2c] build_of_boot=1 carried_setup_us=60100 {s}\n\
         2 [GroupSwap] swap owner=0 kind=0x2d request_to_ready_us=2000000 bodies=[] max_tick_gap_us=77000 ticks=1\n\
         3 [BuildSplit] gaps owner=0 kind=0x2d {g}\n"
    );
    let dir = std::env::temp_dir().join(format!("f6-split-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("diag.log");
    let json = dir.join("out.json");
    std::fs::write(&path, log).unwrap();
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tools/f6_split_table.py");
    let out = Command::new("/usr/bin/python3")
        .args(["-I", script, path.to_str().unwrap(), "--json", json.to_str().unwrap()])
        .output()
        .expect("/usr/bin/python3");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let parsed = std::fs::read_to_string(&json).unwrap();
    for needle in ["\"control_64f820\": [\n    60000,\n    1\n   ]", "\"setup_us\": 60100", "\"max_tick_gap_us\": 77000", "\"after\": \"setup\""] {
        assert!(parsed.contains(needle), "{needle} not in {parsed}");
    }
    std::fs::remove_dir_all(&dir).ok();
}
