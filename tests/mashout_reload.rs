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
    assert!(source.find(missing).unwrap() < source.find(identity).unwrap());
    assert!(source.find(identity).unwrap() < source.find("SEEDS.drain(").unwrap());
    let fixture = FIXTURE
        .replace("// MISSING", &format!("{missing};"))
        .replace("// IDENTITY", identity)
        .replace("// RELOAD", reload);
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
    assert!(
        run.status.success(),
        "actual runtime regression failed:\n{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

const FIXTURE: &str = include_str!("support/mashout_reload_fixture.rs");
