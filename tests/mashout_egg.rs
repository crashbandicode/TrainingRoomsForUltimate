//! Execute actual runtime entry, observer, seed and cancellation decisions against native-memory fixtures.
use std::{fs, process::Command};

fn block<'a>(source: &'a str, marker: &str) -> &'a str {
    assert_eq!(
        source.matches(marker).count(),
        1,
        "unique runtime seam: {marker}"
    );
    let start = source.find(marker).unwrap();
    // The seed closure destructures NativeSeed in its argument; that first brace is not its body.
    let opening = if marker == "SEEDS.drain(" {
        start + source[start..].find("| {").expect("seed closure body") + 2
    } else {
        start + source[start..].find('{').unwrap()
    };
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
    panic!("unterminated runtime seam: {marker}");
}

fn production(source: &str) -> &str {
    source.split("#[cfg(test)]").next().unwrap()
}

#[test]
fn actual_runtime_egg_preparation_and_native_seed() {
    let runtime = include_str!("../src/runtime/features/mashout.rs");
    let moves = include_str!("../src/model/mashout/moves.rs");
    let mapped = include_str!("../src/model/input_delay.rs");
    let linked_start = runtime
        .find("let scored = holding(")
        .or_else(|| runtime.find("let linked = holding("))
        .unwrap();
    let linked_end = runtime[linked_start..].find("if restart {").unwrap() + linked_start;
    let mut linked = runtime[linked_start..linked_end].to_string();
    if !linked.contains("let scored =") {
        linked.push_str("let scored = linked;\n");
    }
    let seed = format!("{});", block(runtime, "SEEDS.drain("));
    let mut fixture = include_str!("support/mashout_egg_fixture.rs").to_string();
    for (marker, value) in [
        (
            "// ENTRY",
            production(include_str!("../src/model/mashout/entry.rs")).to_string(),
        ),
        (
            "// NATIVE",
            production(include_str!("../src/model/mashout/native.rs")).to_string(),
        ),
        (
            "// READER",
            production(include_str!("../src/runtime/features/mashout/reader.rs")).to_string(),
        ),
        (
            "// CONSTANTS",
            production(include_str!("../src/model/cpu/ids.rs")).to_string(),
        ),
        (
            "// HASH",
            production(include_str!("../src/model/cpu/hash.rs")).to_string(),
        ),
        (
            "// HASH40",
            production(include_str!("../src/model/ui/hash40.rs")).to_string(),
        ),
        (
            "// MOVE_TYPES",
            moves[moves
                .find("#[derive(Clone, Copy, Debug, PartialEq, Eq)]")
                .unwrap()..moves.find("const fn entry(").unwrap()]
                .to_string(),
        ),
        (
            "// MAPPED",
            format!(
                "#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]\n{}\n{}",
                block(mapped, "pub struct MappedInputs"),
                block(mapped, "impl MappedInputs")
            ),
        ),
        (
            "// END_TYPE",
            format!(
                "#[derive(Clone, Copy)]\n{}",
                block(runtime, "struct NativeEnd")
            ),
        ),
        (
            "// END_TOKEN",
            block(runtime, "unsafe fn end_token(").to_string(),
        ),
        (
            "// END_AFTER",
            block(runtime, "unsafe fn end_after(").to_string(),
        ),
        ("// ARM", block(runtime, "unsafe fn arm(").to_string()),
        (
            "// TOKEN_TYPE",
            block(runtime, "pub(crate) struct SeedToken").to_string(),
        ),
        (
            "// NATIVE_SEED_TYPES",
            format!(
                "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n{}\n#[derive(Clone, Copy)]\n{}",
                block(runtime, "enum SeedOrigin"),
                block(runtime, "struct NativeSeed")
            ),
        ),
        (
            "// TOKEN",
            block(runtime, "pub(crate) unsafe fn seed_token(").to_string(),
        ),
        (
            "// AFTER",
            block(runtime, "pub(crate) unsafe fn seed_after(").to_string(),
        ),
        (
            "// THROW_AFTER",
            block(runtime, "pub(crate) unsafe fn throw_seed_after(").to_string(),
        ),
        (
            "// PUBLISH_SEED",
            block(runtime, "fn publish_seed(").to_string(),
        ),
        (
            "// START",
            block(
                runtime,
                if runtime.contains("if !pluck &&") {
                    "if !pluck &&"
                } else {
                    "if !entry::preparatory"
                },
            )
            .to_string(),
        ),
        (
            "// ENTERING",
            block(runtime, "Step::Entering => {").to_string(),
        ),
        ("// LINKS", linked),
        ("// SETTLE", block(runtime, "Step::Settle => {").to_string()),
        (
            "// PLACE",
            block(runtime, "Step::Place if !linked").to_string(),
        ),
        (
            "// BODY_GUARD",
            block(
                runtime,
                "if matches!(game.step, Step::Held | Step::Entering | Step::Result)",
            )
            .to_string(),
        ),
        ("// RELEASED", {
            let a = runtime.find("let released =").unwrap();
            runtime[a..=a + runtime[a..].find(';').unwrap()].to_string()
        }),
        ("// HELD", block(runtime, "Step::Held => {").replacen("Step::Held =>", "Step::Held if EXECUTE_HELD.load(Ordering::Relaxed) =>", 1)),
        ("// SEEDS", seed),
        ("// ENDS", format!("{});", block(runtime, "ENDS.drain("))),
        ("// RESTART", block(runtime, "if restart {").to_string()),
    ] {
        fixture = fixture.replace(&format!("{marker}\n"), &format!("{value}\n"));
    }
    run_fixture(fixture, "egg");
}

#[test]
fn actual_runtime_worker_native_epoch_admission_and_bests() {
    let runtime = include_str!("../src/runtime/features/mashout.rs");
    let run = include_str!("../src/model/mashout/run.rs");
    let practice = include_str!("../src/model/practice_games.rs");
    let mut fixture = include_str!("support/mashout_worker_fixture.rs").to_owned();
    for (marker, source) in [
        (
            "// WORKER",
            include_str!("../src/runtime/features/mashout/worker.rs").to_owned(),
        ),
        (
            "// NATIVE",
            production(include_str!("../src/model/mashout/native.rs")).to_owned(),
        ),
        (
            "// GRADE",
            production(include_str!("../src/model/mashout/grade.rs")).to_owned(),
        ),
        (
            "// RUN",
            run.split("#[cfg(test)]\n#[path = \"session_tests.rs\"]")
                .next()
                .unwrap()
                .to_owned(),
        ),
        (
            "// STORAGE",
            include_str!("../src/model/storage.rs")
                .split("#[cfg(test)]\nmod tests")
                .next()
                .unwrap()
                .to_owned(),
        ),
        (
            "// PRACTICE",
            [
                "pub fn record_time",
                "pub fn best_time",
                "pub fn result_tick",
            ]
            .map(|m| block(practice, m))
            .join("\n"),
        ),
        (
            "// FAULTED",
            block(
                include_str!("../src/model/mashout/admission.rs"),
                "pub const fn faulted",
            )
            .to_owned(),
        ),
        (
            "// SEED_ORIGIN",
            format!(
                "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n{}",
                block(runtime, "enum SeedOrigin")
            ),
        ),
    ] {
        fixture = fixture.replace(&format!("{marker}\n"), &format!("{source}\n"));
    }
    run_fixture(fixture, "worker");
}

#[test]
fn actual_interposer_install_restore_and_foreign_slot_refusal() {
    let source = include_str!("../src/runtime/features/mashout/interposer.rs");
    let mut fixture = include_str!("support/mashout_interposer_fixture.rs").to_owned();
    for (marker, replacement) in [
        (
            "// SERVICE",
            block(source, "pub(super) fn service()").replace("pub(super)", "pub"),
        ),
        (
            "// READER_TABLE",
            block(source, "pub(super) unsafe fn reader_table(").replace("pub(super)", "pub"),
        ),
        ("// TARGETS", block(source, "fn targets()").to_owned()),
        (
            "// STARTED",
            block(source, "unsafe extern \"C\" fn started(").to_owned(),
        ),
        (
            "// END",
            block(source, "unsafe extern \"C\" fn end(").to_owned(),
        ),
    ] {
        fixture = fixture.replace(marker, &replacement);
    }
    run_fixture(
        fixture
            .replace("mashout::throw_seed_after", "throw_seed_after")
            .replace("mashout::end_token", "end_token")
            .replace("mashout::end_after", "end_after"),
        "interposer",
    );
}

fn run_fixture(fixture: String, label: &str) {
    // Inner module docs need their original position before imports.
    let fixture = fixture.replace("//!", "//");
    let dir = std::env::temp_dir().join(format!("tr-mashout-{label}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(dir.clone());
    let input = dir.join("actual.rs");
    let binary = dir.join("actual");
    fs::write(&input, fixture).unwrap();
    let compiled = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2021", "--test"])
        .arg(&input)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    if !compiled.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&compiled.stderr));
    }
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let run = Command::new(&binary)
        .args(["--test-threads=1", "--nocapture"])
        .output()
        .unwrap();
    print!("{}", String::from_utf8_lossy(&run.stdout));
    assert!(
        run.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}
