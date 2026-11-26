//! The CPU Character expiry notice follows the request's owner (Astra review of 9cf8c0cf, MED 1): Mash out's own
//! request (its Mash Off restore of the original CPU) that expires against a new menu epoch is quiet, as the operator
//! ruled for that restore (11-17 ~22:20Z "1. yes quiet"); the user's own expired pick still draws "Character choice
//! expired; select it again". Runs the ACTUAL expiry branch of src/runtime/mii/mii_ordinary_cpu.rs `begin_prepare` in a
//! host fixture (RED before the fix: the Practice request drew the notice, run rc15-cpu-receipt-xps-2 #4505).
use std::{fs, process::Command};

const CPU: &str = include_str!("../src/runtime/mii/mii_ordinary_cpu.rs");
const PLAYER_MODEL: &str = include_str!("../src/model/mii/ordinary_player.rs");

fn block<'a>(source: &'a str, marker: &str) -> &'a str {
    assert_eq!(source.matches(marker).count(), 1, "unique actual-runtime seam: {marker}");
    let start = source.find(marker).unwrap();
    let opening = start + source[start..].find('{').unwrap();
    let mut depth = 0;
    for (offset, byte) in source.as_bytes()[opening..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => (),
        }
        if depth == 0 {
            return &source[start..=opening + offset];
        }
    }
    panic!("unterminated actual-runtime seam: {marker}");
}

const FIXTURE: &str = r#"
#![allow(dead_code, unused_variables)]
use std::sync::Mutex;
static NOTICES: Mutex<Vec<String>> = Mutex::new(Vec::new());
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role { Player, Cpu }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Requester { User, Practice }
#[derive(Clone, Copy)]
struct Request { direct: Option<u32>, owner: Requester }
impl Request {
    fn direct_kind(self) -> Option<u32> { self.direct }
    fn owner(self) -> Requester { self.owner }
}
fn request_current(_: Request, _: ()) -> bool { false }
fn clear_request(_: Request) {}
fn note(_: &str) {}
fn notify(_: Role, message: std::borrow::Cow<'static, str>) { NOTICES.lock().unwrap().push(message.into_owned()); }
// MODEL_FNS
fn actual_expiry(request: Request) -> bool {
    let active = ();
    // EXPIRY
    true
}
fn drawn(owner: Requester) -> Vec<String> {
    NOTICES.lock().unwrap().clear();
    let returned = actual_expiry(Request { direct: Some(0x58), owner });
    assert!(!returned, "the expired request must stop begin_prepare");
    NOTICES.lock().unwrap().clone()
}
#[test]
fn a_practice_request_expires_quietly() {
    assert_eq!(drawn(Requester::Practice), Vec::<String>::new(), "Mash out's own expired request drew a CPU notice");
}
#[test]
fn the_users_own_expired_pick_still_draws_its_notice() {
    assert_eq!(drawn(Requester::User), vec!["Character choice expired; select it again".to_string()]);
}
"#;

#[test]
fn actual_cpu_expiry_notice_follows_the_request_owner() {
    let expiry = block(CPU, "if !request_current(request,active) {");
    let model = if PLAYER_MODEL.contains("pub fn expiry_notice(") {
        block(PLAYER_MODEL, "pub fn expiry_notice(").to_string()
    } else {
        String::new()
    };
    let fixture = FIXTURE.replace("// EXPIRY", expiry).replace("// MODEL_FNS", &model);
    let dir = crate::test_scratch::Scratch::new("cpu-expiry-owner");
    let input = dir.join("actual.rs");
    let binary = dir.join("actual");
    fs::write(&input, fixture).unwrap();
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let compiled = Command::new(compiler).args(["--edition=2021", "--test"]).arg(&input).arg("-o").arg(&binary)
        .output().unwrap();
    assert!(compiled.status.success(), "fragments did not compile: {}", String::from_utf8_lossy(&compiled.stderr));
    let run = Command::new(&binary).args(["--test-threads=1"]).output().unwrap();
    print!("{}", String::from_utf8_lossy(&run.stdout));
    let _ = fs::remove_dir_all(&dir);
    assert!(run.status.success(), "actual runtime regression failed:\n{}", String::from_utf8_lossy(&run.stdout));
}
