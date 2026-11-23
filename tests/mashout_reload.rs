//! Execute the Switch-only runtime's actual reload/identity branches on controlled native bodies.
//! Regression: a generation-5 owned reload temporarily removed the CPU and falsely failed the run.
use std::{fs, process::Command};

fn block<'a>(source: &'a str, marker: &str) -> &'a str {
    assert_eq!(
        source.matches(marker).count(),
        1,
        "unique actual-runtime seam: {marker}"
    );
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
            let result = &source[start..=opening + offset];
            // These selected fragments have no braces in strings/comments; reject ambiguous extraction.
            for line in result.lines() {
                if let Some((_, tail)) = line.split_once('"') {
                    let text = tail.rsplit_once('"').unwrap().0;
                    assert!(!text.contains(['{', '}']));
                }
                if let Some((_, comment)) = line.split_once("//") {
                    assert!(!comment.contains(['{', '}']));
                }
            }
            return result;
        }
    }
    panic!("unterminated actual-runtime seam: {marker}");
}

#[test]
fn actual_runtime_reload_waits_without_weakening_active_attempt_identity() {
    let source = include_str!("../src/runtime/features/mashout.rs");
    let missing = block(
        source,
        "let Some(cpu) = natives::module_accessor(cpu_entry) else {",
    );
    let identity = block(
        source,
        "if matches!(game.step, Step::Held | Step::Entering | Step::Result)",
    );
    let reload = block(source, "Step::Reload => {");
    let choose = block(source, "if kind != request.attacker_kind {");
    let choose_gate = block(block(source, "Step::Choose => {"), "if !released");
    let release_start = source.find("let released =").unwrap();
    let release = &source[release_start..=release_start + source[release_start..].find(';').unwrap()];
    let restart = block(source, "if restart {");
    let stopped_owner = block(source, "if config.is_none() && crate::runtime::ui::menu_app::active() {");
    let running_owner = block(source, "else if !matches!(game.step, Step::Off | Step::Reload | Step::Restore)");
    let ownership = format!("{stopped_owner} {running_owner}");
    let release_first = "Step::Release if released => game.step(";
    let release_end = "Step::Retry if released => {";
    assert_eq!(source.matches(release_first).count(), 1);
    assert_eq!(source.matches(release_end).count(), 1);
    let release_arms = &source[source.find(release_first).unwrap()..source.find(release_end).unwrap()];
    let covered = block(source, "if covered && !matches!(game.step, Step::Held | Step::Result | Step::Restore)");
    let restore = block(source, "Step::Restore => {");
    let end = block(source, "fn end_practice(&mut self) {");
    assert!(source.find(missing).unwrap() < source.find(identity).unwrap());
    assert!(source.find(identity).unwrap() < source.find("SEEDS.drain(").unwrap());
    let fixture = FIXTURE
        .replace("// MISSING", &format!("{missing};"))
        .replace("// IDENTITY", identity)
        .replace("// RELOAD", reload)
        .replace("// CHOOSE", choose)
        .replace("// RELEASE_CHECK", release)
        .replace("// ADMISSION_GATE", choose_gate)
        .replace("// RESTART", restart)
        .replace("// OWNERSHIP", &ownership)
        .replace("// RELEASE_ARMS", release_arms)
        .replace("// COVERED", covered)
        .replace("// RESTORE", restore)
        .replace("// END_PRACTICE", end);
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("tr-mashout-reload-{}-{unique}", std::process::id()));
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
        "actual runtime fragments did not compile: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let run = Command::new(&binary)
        .args(["--test-threads=1", "--nocapture"])
        .output()
        .unwrap();
    print!("{}", String::from_utf8_lossy(&run.stdout));
    assert!(
        run.status.success(),
        "actual runtime regression failed:\n{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

const FIXTURE: &str = include_str!("support/mashout_reload_fixture.rs");
