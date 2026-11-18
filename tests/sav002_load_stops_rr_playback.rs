//! SAV-002 (p3-rec-xps-6 rows 20/21): "Load only warps. It doesn't start a recording, and it stops one that is playing"
//! (docs/guide/save-states.md:17). A Recording-page macro plays in `model::rr_host` (its own runs, RUN_PLAYER / RUN_CPU),
//! not in the old `features::recorder` machine; the load path stopped only the latter (`recorder_stop mode=0`, already idle)
//! and the Loop-∞ take kept playing after the load.

fn source(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path)).unwrap()
}

/// The body of `pub fn on_load_request()` in features/recorder.rs.
fn on_load_request_body() -> String {
    let code = source("src/runtime/features/recorder.rs");
    let start = code.find("pub fn on_load_request()").expect("on_load_request");
    let body = &code[start..];
    let end = body.find("\n}\n").expect("end of on_load_request");
    body[..end].to_string()
}

#[test]
fn a_load_request_stops_the_recording_hosts_playback() {
    let body = on_load_request_body();
    assert!(body.contains("note_load()"),
        "a save-state load must stop rr_host playback (the Recording page's macros), not only the old recorder:\n{body}");
}
