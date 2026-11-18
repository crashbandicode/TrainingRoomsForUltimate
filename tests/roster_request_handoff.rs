//! Execute the real CPU roster request routing and readiness functions in a host-only fixture.
use std::{fs, process::Command};

fn replace_once(template: String, marker: &str, replacement: &str) -> String {
    assert_eq!(
        template.matches(marker).count(),
        1,
        "unique fixture seam: {marker}"
    );
    template.replace(marker, replacement)
}

// Find the Rust function body without counting braces inside strings or comments.
fn block<'a>(source: &'a str, marker: &str) -> &'a str {
    assert_eq!(
        source.matches(marker).count(),
        1,
        "unique actual-runtime seam: {marker}"
    );
    let start = source.find(marker).unwrap();
    let opening = start + source[start..].find('{').unwrap();
    let bytes = source.as_bytes();
    let mut i = opening;
    let mut depth = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            i += 2;
            let mut comments = 1usize;
            while i < bytes.len() && comments != 0 {
                if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                    comments += 1;
                    i += 2;
                } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                    comments -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            continue;
        }
        if let Some(end) = raw_string_end(bytes, i) {
            i = end;
            continue;
        }
        if bytes[i] == b'"' {
            i = quoted_end(bytes, i, b'"');
            continue;
        }
        if bytes[i] == b'\'' {
            if let Some(end) = char_literal_end(bytes, i) {
                i = end;
                continue;
            }
        }
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("unterminated actual-runtime seam: {marker}");
}

fn quoted_end(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i = (i + 2).min(bytes.len());
        } else if bytes[i] == quote {
            return i + 1;
        } else {
            i += 1;
        }
    }
    bytes.len()
}

fn char_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    if i >= bytes.len() || bytes[i] == b'\n' || bytes[i] == b'\'' {
        return None;
    }
    if bytes[i] == b'\\' {
        i += 1;
        match *bytes.get(i)? {
            b'u' if bytes.get(i + 1) == Some(&b'{') => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'}' {
                    i += 1;
                }
                if i >= bytes.len() {
                    return None;
                }
                i += 1;
            }
            b'x' => i = (i + 3).min(bytes.len()),
            _ => i += 1,
        }
    } else {
        let width = match bytes[i] {
            b if b & 0x80 == 0 => 1,
            b if b & 0xe0 == 0xc0 => 2,
            b if b & 0xf0 == 0xe0 => 3,
            _ => 4,
        };
        i = (i + width).min(bytes.len());
    }
    (bytes.get(i) == Some(&b'\'')).then_some(i + 1)
}

fn raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    if bytes.get(i) == Some(&b'b') {
        i += 1;
    }
    if bytes.get(i) != Some(&b'r') {
        return None;
    }
    i += 1;
    let mut hashes = 0usize;
    while bytes.get(i) == Some(&b'#') {
        hashes += 1;
        i += 1;
    }
    if bytes.get(i) != Some(&b'"') {
        return None;
    }
    i += 1;
    while i < bytes.len() {
        if bytes[i] == b'"'
            && bytes
                .get(i + 1..i + 1 + hashes)
                .is_some_and(|tail| tail.iter().all(|b| *b == b'#'))
        {
            return Some(i + 1 + hashes);
        }
        i += 1;
    }
    None
}

#[test]
fn actual_cpu_request_handoff_and_reload_readiness() {
    let roster = include_str!("../src/runtime/roster/character_roster.rs");
    let cpu = include_str!("../src/runtime/mii/mii_ordinary_cpu.rs");
    let player = include_str!("../src/runtime/mii/mii_ordinary_player.rs");
    let saved_alpha = include_str!("../src/runtime/mii/mii_saved_alpha.rs");
    let model_player = include_str!("../src/model/mii/ordinary_player.rs");

    let queue = block(
        roster,
        "fn queue_admitted(role: Role, selection: Selection, restoring: bool, once: bool)",
    );
    let owned = block(
        roster,
        "fn queue_owned(role: Role, selection: Selection, restoring: bool, once: bool, owner: Requester)",
    );
    let request = block(roster, "pub fn request_cpu_kind(kind: u32)");
    let live_owner = block(roster, "fn live_owner(role: Role, kind: u32) -> Requester");
    let sync_selection = block(roster, "fn sync_selection(self, fighter: TrioFighter, owner: Requester)");
    let persists = block(model_player, "pub fn persists_selection(owner:Requester)->bool");
    let readiness = block(roster, "pub fn cpu_reload_ready() -> bool");
    let notice = block(roster, "pub(crate) fn notice_blocked() -> bool");
    let preload_pending = block(cpu, "pub(super) fn preload_pending()->bool");
    let intercept = block(
        cpu,
        "pub(super) fn intercept(role:Role,selection:Selection,user_request:bool,requester:Requester)->Option<bool>",
    );

    assert!(player.contains(
        "pub(super) fn intercept(role:Role,selection:Selection,user_request:bool)->Option<bool>"
    ));
    let player_stub = "pub fn intercept(_: Role, _: Selection, _: bool) -> Option<bool> { None }";
    let saved_stub = if saved_alpha.contains(
        "pub(super) fn intercept(roster_role: Role, selection: Selection) -> Option<bool>",
    ) {
        "pub fn intercept(_: Role, _: Selection) -> Option<bool> { None }"
    } else {
        assert!(saved_alpha
            .contains("pub(super) fn intercept(roster_role: Role, selection: Selection) -> bool"));
        "pub fn intercept(_: Role, _: Selection) -> bool { false }"
    };

    let mut fixture = FIXTURE.to_owned();
    fixture = replace_once(fixture, "// CPU_PRELOAD_PENDING", preload_pending);
    fixture = replace_once(fixture, "// CPU_INTERCEPT", intercept);
    fixture = replace_once(fixture, "// NOTICE_BLOCKED", notice);
    fixture = replace_once(fixture, "// QUEUE_ADMITTED", queue);
    fixture = replace_once(fixture, "// QUEUE_OWNED", owned);
    fixture = replace_once(fixture, "// LIVE_OWNER", live_owner);
    fixture = replace_once(fixture, "// SYNC_SELECTION", sync_selection);
    fixture = replace_once(fixture, "// PERSISTS_SELECTION", persists);
    fixture = replace_once(fixture, "// CPU_RELOAD_READY", readiness);
    fixture = replace_once(fixture, "// REQUEST_CPU_KIND", request);
    fixture = replace_once(fixture, "// PLAYER_INTERCEPT_STUB", player_stub);
    fixture = replace_once(fixture, "// SAVED_ALPHA_INTERCEPT_STUB", saved_stub);

    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "roster-request-handoff-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&dir).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(dir.clone());
    let input = dir.join("actual.rs");
    let binary = dir.join(format!("actual{}", std::env::consts::EXE_SUFFIX));
    fs::write(&input, fixture).unwrap();
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let compiled = Command::new(compiler)
        .args(["--edition=2021", "--test"])
        .arg(&input)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "actual roster fragments did not compile: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let run = Command::new(&binary)
        .args(["--test-threads=1", "--nocapture"])
        .output()
        .unwrap();
    print!("{}", String::from_utf8_lossy(&run.stdout));
    assert!(
        run.status.success(),
        "actual CPU request handoff regression failed:\n{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

const FIXTURE: &str = include_str!("support/roster_request_handoff_fixture.rs");
