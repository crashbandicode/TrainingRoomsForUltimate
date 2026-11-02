//! The dependency lock keeps one copy of the Skyline runtime crates (package 6, lead ruling 2026-11-02): `skyline_smash` names the
//! skyline-rs and nnsdk-rs git sources, this plugin names crates.io `skyline =0.6.0`; the manifest's `[patch]` entries point the git
//! sources at the registry crates so a duplicate runtime crate (double statics, double size) cannot appear. A second copy of `skyline`,
//! `skyline_macro` or `nnsdk`, or any of them from a git source, fails here.

const LOCK: &str = include_str!("../Cargo.lock");

fn packages(name: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut current: Option<String> = None;
    let mut version = String::new();
    for line in LOCK.lines() {
        if let Some(n) = line.strip_prefix("name = \"") {
            current = Some(n.trim_end_matches('"').to_owned());
        } else if let Some(v) = line.strip_prefix("version = \"") {
            version = v.trim_end_matches('"').to_owned();
        } else if let Some(s) = line.strip_prefix("source = \"") {
            if current.as_deref() == Some(name) {
                found.push((version.clone(), s.trim_end_matches('"').to_owned()));
            }
        }
    }
    found
}

#[test]
fn one_copy_of_each_skyline_runtime_crate_and_all_from_the_registry() {
    for name in ["skyline", "skyline_macro", "nnsdk"] {
        let copies = packages(name);
        assert_eq!(copies.len(), 1, "{name} appears {} times in Cargo.lock: {copies:?}", copies.len());
        assert!(copies[0].1.starts_with("registry+"), "{name} must come from the registry: {copies:?}");
    }
    assert_eq!(packages("skyline")[0].0, "0.6.0", "the pinned Skyline");
}

#[test]
fn the_patch_entries_that_unify_them_are_in_the_manifest() {
    let manifest = include_str!("../Cargo.toml");
    for source in ["https://github.com/ultimate-research/skyline-rs", "https://github.com/ultimate-research/nnsdk-rs"] {
        assert!(manifest.contains(&format!("[patch.\"{source}\"]")), "missing [patch] for {source}");
    }
}