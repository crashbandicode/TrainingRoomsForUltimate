//! Execute actual runtime entry, observer, seed and cancellation decisions against native-memory fixtures.
use std::{fs, process::Command};

fn block<'a>(source: &'a str, marker: &str) -> &'a str {
    assert_eq!(
        source.matches(marker).count(),
        1,
        "unique runtime seam: {marker}"
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
        ("// ARM", block(runtime, "unsafe fn arm(").to_string()),
        (
            "// TOKEN_TYPE",
            block(runtime, "pub(crate) struct SeedToken").to_string(),
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
            "// START",
            block(runtime, "if !entry::preparatory").to_string(),
        ),
        (
            "// ENTERING",
            block(runtime, "Step::Entering => {").to_string(),
        ),
        ("// LINKS", linked),
        ("// SEEDS", seed),
        ("// RESTART", block(runtime, "if restart {").to_string()),
    ] {
        fixture = fixture.replace(&format!("{marker}\n"), &format!("{value}\n"));
    }
    // Inner module docs need their original position before imports.
    fixture = fixture.replace("//!", "//");
    let dir = std::env::temp_dir().join(format!("tr-mashout-egg-{}", std::process::id()));
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
