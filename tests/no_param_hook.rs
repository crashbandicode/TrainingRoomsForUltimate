//! TrainingRooms uses no parameter-hook provider (order of 2026-11-27): the shield damage multiplier is read from the
//! game's own `CommonParams` object (`cpu_common_params_hook`; SPEC-cpu-05 question 6), so `libparam_hook.nro` is not a
//! dependency. The release NRO of `dc5302f1` imports none of that plugin's exports and holds none of its names. This
//! keeps the source and the manifests that way, comments included: a comment that said the multiplier needed the
//! plugin is how the install pages came to ask for it.

use std::fs;
use std::path::{Path, PathBuf};

/// The provider's export, the Skyline smash bindings that register with it, its file and crate names, and the
/// phrase the stale comment used.
const PROVIDER_NAMES: &[&str] =
    &["add_param_load_hook", "params_main", "smash::params", "ParamsInfo", "libparam_hook", "params-hook", "param-hook plugin"];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn nothing_in_the_plugin_names_the_param_hook_provider() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![root.join("Cargo.toml"), root.join("Cargo.lock")];
    rust_files(&root.join("src"), &mut files);
    rust_files(&root.join("crates"), &mut files);
    let mut hits = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        for (n, line) in text.lines().enumerate() {
            for name in PROVIDER_NAMES {
                if line.contains(name) {
                    hits.push(format!("{}:{} {name}", file.strip_prefix(root).unwrap().display(), n + 1));
                }
            }
        }
    }
    assert!(hits.is_empty(), "the param-hook provider is named in {hits:?}");
}
