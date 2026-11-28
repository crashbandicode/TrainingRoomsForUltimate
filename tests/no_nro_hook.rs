//! TrainingRooms uses no NRO-load provider (order of 2026-11-27): it sees module loads at its own
//! `nro_loaded` site, so `libnro_hook.nro` is not a dependency. This keeps source and manifests free
//! of the provider's names, comments included.

use std::fs;
use std::path::{Path, PathBuf};

const PROVIDER_NAMES: &[&str] =
    &["add_nro_load_hook", "add_nro_unload_hook", "skyline::nro", "NroInfo", "libnro_hook", "no_nro_hook", "nro-load provider"];

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
fn nothing_in_the_plugin_names_the_nro_hook_provider() {
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
    assert!(hits.is_empty(), "the nro-hook provider is named in {hits:?}");
}
