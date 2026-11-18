//! FS-027 / FS-031 (phase3-sol SOL-R27, SOL-R28): a Kirby hat and a common-item grant were accepted inside a named Player take.
//! Every "busy" guard asks `recorder::recording_or_playback()`, which reads only the old recorder machine's phase; the
//! Recording page's takes and macros run in `model::rr_host` (capture role, runs and queues), so the guards read idle.

fn source(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path)).unwrap()
}

#[test]
fn the_busy_predicate_sees_the_recording_hosts_take_and_playback() {
    let code = source("src/runtime/features/recorder.rs");
    let start = code.find("pub fn recording_or_playback()").expect("recording_or_playback");
    let body = &code[start..start + code[start..].find("\n}\n").expect("end")];
    assert!(body.contains("recording_host::") || body.contains("rr_host::"),
        "recording_or_playback must also report a Recording-page take or playback (rr_host):\n{body}");
}