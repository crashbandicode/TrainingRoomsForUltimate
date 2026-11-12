//! The build tag the menu footer ends with (R13, `model::frame::BUILD_TAG`): the internal build name and the commit
//! being built. `TR_BUILD_SHA` is `git rev-parse --short=8 HEAD` run from this directory, with `-dirty` when a tracked
//! file differs from HEAD, or `unknown` without git or a repository (a source archive); the build never fails on it.

use std::path::{Path, PathBuf};
use std::process::Command;

const BUILD_NAME: &str = "rc10";

/// The binary's inputs: an edit after a clean build reruns this script, so the tag gains `-dirty`.
const SOURCES: [&str; 7] = ["src", "data", "assets", "crates", "third_party", "Cargo.toml", "Cargo.lock"];

/// One git call's trimmed stdout, or `None` when git is missing or the call fails. No optional locks: a status here
/// never rewrites the index.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).env("GIT_OPTIONAL_LOCKS", "0").output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok().map(|text| text.trim().to_owned())
}

fn rerun_if_changed(path: &Path) {
    // A path that does not exist would rerun the script on every build.
    if path.exists() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn main() {
    // The package being built, as cargo sets it for this run (rc.9, TASK-rc9.md item 3): `env!` would freeze the path the
    // script was compiled in, and a target dir shared with another checkout would read that checkout's HEAD.
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR").map_or_else(|| PathBuf::from("."), PathBuf::from);
    let dir = manifest.as_path();
    rerun_if_changed(&dir.join("build.rs"));
    for source in SOURCES {
        rerun_if_changed(&dir.join(source));
    }
    let sha = git(dir, &["rev-parse", "--short=8", "HEAD"]).filter(|sha| sha.len() >= 8 && sha.bytes().all(|b| b.is_ascii_hexdigit()));
    let tag = match sha {
        Some(sha) => {
            // A new commit or checkout reruns it: HEAD, its reflog (written on every commit, also when the branch ref is
            // packed and has no file yet) and the ref. `--git-path` follows a worktree's `.git` file to its own HEAD and
            // to the common directory's refs.
            let head_ref = git(dir, &["symbolic-ref", "-q", "HEAD"]);
            for name in ["HEAD", "logs/HEAD", "packed-refs"].into_iter().chain(head_ref.as_deref()) {
                if let Some(path) = git(dir, &["rev-parse", "--git-path", name]) {
                    rerun_if_changed(&dir.join(path));
                }
            }
            // A status that fails cannot show the tree clean.
            let dirty = git(dir, &["status", "--porcelain", "--untracked-files=no"]).is_none_or(|changes| !changes.is_empty());
            if dirty {
                format!("{sha}-dirty")
            } else {
                sha
            }
        }
        None => "unknown".to_owned(),
    };
    println!("cargo:rustc-env=TR_BUILD_SHA={tag}");
    println!("cargo:rustc-env=TR_BUILD_NAME={BUILD_NAME}");
    guard_tables(dir);
}

/// The guard tables (rc.10 S1, `tools/rows_tsv.py`): each `data/tables/<name>.tsv` becomes the array literal
/// `$OUT_DIR/tables/<name>.rs` that its source file includes. Line 1 is `#! <Struct>` and the field names; a row is a
/// cell per field, the field's Rust value as written, except `guard`: 64 bytes as 128 hex digits. `# ` lines are
/// comments. A malformed file fails the build.
fn guard_tables(dir: &Path) {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("tables");
    std::fs::create_dir_all(&out).expect("tables dir");
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join("data/tables")).expect("data/tables").map(|e| e.expect("entry").path()).collect();
    files.sort();
    for file in files.iter().filter(|f| f.extension().is_some_and(|e| e == "tsv")) {
        let name = file.file_stem().and_then(|s| s.to_str()).expect("table name");
        let text = std::fs::read_to_string(file).expect("table file");
        let mut lines = text.lines();
        let head = lines.next().and_then(|l| l.strip_prefix("#! ")).unwrap_or_else(|| panic!("{name}: no `#! ` header"));
        let mut names = head.split('\t');
        let kind = names.next().expect("struct name");
        let names: Vec<&str> = names.collect();
        let mut code = String::from("[\n");
        for (number, line) in lines.enumerate().filter(|(_, l)| !l.starts_with("# ") && !l.is_empty()) {
            let cells: Vec<&str> = line.split('\t').collect();
            assert!(cells.len() == names.len(), "{name} line {}: {} cells for {} fields", number + 2, cells.len(), names.len());
            code.push_str(kind);
            code.push_str(" { ");
            for (field, cell) in names.iter().zip(cells) {
                code.push_str(field);
                code.push_str(": ");
                if *field == "guard" {
                    assert!(cell.len() == 128 && cell.bytes().all(|b| b.is_ascii_hexdigit()), "{name} line {}: guard is not 64 bytes of hex", number + 2);
                    code.push('[');
                    for byte in cell.as_bytes().chunks(2) {
                        code.push_str("0x");
                        code.push_str(std::str::from_utf8(byte).expect("hex"));
                        code.push(',');
                    }
                    code.push(']');
                } else {
                    code.push_str(cell);
                }
                code.push_str(", ");
            }
            code.push_str("},\n");
        }
        code.push_str("]\n");
        std::fs::write(out.join(format!("{name}.rs")), code).expect("write table");
    }
}

