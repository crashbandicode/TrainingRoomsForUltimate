//! R13: `build.rs` embeds the build name and the commit being built (`model::frame::BUILD_TAG`). Cargo reruns the script
//! on a new commit, so the sha is what `git rev-parse --short=8 HEAD` gives for this tree now, with `-dirty` or not, or
//! `unknown` where git or the repository is missing.

use std::process::Command;

#[test]
fn the_build_tag_is_the_commit_being_built() {
    assert_eq!(env!("TR_BUILD_NAME"), "rc12");
    let tag = env!("TR_BUILD_SHA");
    let head = Command::new("git")
        .args(["-C", env!("CARGO_MANIFEST_DIR"), "rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned());
    match head {
        Some(head) => assert_eq!(tag.strip_suffix("-dirty").unwrap_or(tag), head, "TR_BUILD_SHA={tag}"),
        None => assert_eq!(tag, "unknown"),
    }
}
