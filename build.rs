//! The build tag the menu footer ends with (R13, `model::frame::BUILD_TAG`): the internal build name and the commit
//! being built. `TR_BUILD_SHA` is `git rev-parse --short=8 HEAD` run from this directory, with `-dirty` when a tracked
//! file differs from HEAD, or `unknown` without git or a repository (a source archive); the build never fails on it.

use std::path::Path;
use std::process::Command;

const BUILD_NAME: &str = "rc8";

/// The binary's inputs: an edit after a clean build reruns this script, so the tag gains `-dirty`.
const SOURCES: [&str; 6] = ["src", "assets", "crates", "third_party", "Cargo.toml", "Cargo.lock"];

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
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
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
}
