//! Host tests of the version guard `src/model/version.rs` (playbook `#[path]` pattern): the
//! supported game identity and the guard's comparison. Carried over from the earlier fork's
//! `training_mod_consts/tests/model_facade.rs` (stage m5o, ours-new); see CARRY-OVER.md.

#[path = "../src/model/version.rs"]
#[allow(dead_code, unused_imports)]
mod model;

use model::{find_gnu_build_id, gnu_build_id_at, verdict, Hex, Printable, Verdict};

fn note(id: &[u8]) -> Vec<u8> {
    let mut n = Vec::new();
    n.extend_from_slice(&4u32.to_le_bytes());
    n.extend_from_slice(&(id.len() as u32).to_le_bytes());
    n.extend_from_slice(&3u32.to_le_bytes());
    n.extend_from_slice(b"GNU\0");
    n.extend_from_slice(id);
    n
}

/// The 13.0.5 shape: a one-page tail ending at `.data`, the note at 0xf3c, zero padding after it.
fn tail_1305(id: &[u8]) -> Vec<u8> {
    let mut tail = vec![0x5au8; 0x1000];
    let n = note(id);
    tail[0xf3c..0xf3c + n.len()].copy_from_slice(&n);
    tail[0xf3c + n.len()..].fill(0);
    tail
}

#[test]
fn the_build_id_constants_agree() {
    assert_eq!(Hex(&model::MAIN_BUILD_ID_BYTES).to_string(), model::MAIN_BUILD_ID);
    assert_eq!(model::SUPPORTED_VERSION, "13.0.5");
    // The note sits in the last page of 13.0.5 .rodata (.data starts at 0x4f17000).
    assert_eq!(0x4f17000 - model::MAIN_BUILD_ID_NOTE_OFFSET, 0x1000 - 0xf3c);
    const { assert!(model::BUILD_ID_SCAN_BYTES >= 0x1000 - 0xf3c) };
}

#[test]
fn a_gnu_build_id_note_is_parsed_and_anything_else_is_not() {
    let n = note(&model::MAIN_BUILD_ID_BYTES);
    assert_eq!(gnu_build_id_at(&n), Some(&model::MAIN_BUILD_ID_BYTES[..]));
    let mut wrong_type = n.clone();
    wrong_type[8] = 1;
    assert_eq!(gnu_build_id_at(&wrong_type), None);
    let mut wrong_name = n.clone();
    wrong_name[12] = b'X';
    assert_eq!(gnu_build_id_at(&wrong_name), None);
    let mut too_long = n.clone();
    too_long[4] = 33;
    assert_eq!(gnu_build_id_at(&too_long), None);
    assert_eq!(gnu_build_id_at(&n[..n.len() - 1]), None, "a truncated descriptor is not a note");
    assert_eq!(gnu_build_id_at(&[]), None);
}

#[test]
fn the_guard_finds_the_note_at_the_end_of_rodata() {
    let tail = tail_1305(&model::MAIN_BUILD_ID_BYTES);
    let (at, id) = find_gnu_build_id(&tail).expect("note found");
    assert_eq!(at, 0xf3c);
    assert_eq!(id, model::MAIN_BUILD_ID_BYTES);
    // Another build: same layout, other ID.
    let other = [0x11u8; 20];
    assert_eq!(find_gnu_build_id(&tail_1305(&other)).map(|(_, id)| id), Some(&other[..]));
    // No note at all, and tails too short to hold one.
    assert_eq!(find_gnu_build_id(&[0u8; 0x1000]), None);
    assert_eq!(find_gnu_build_id(&[0u8; 16]), None);
    assert_eq!(find_gnu_build_id(&[]), None);
}

#[test]
fn the_verdict_needs_both_the_version_and_the_build_id() {
    let id = &model::MAIN_BUILD_ID_BYTES[..];
    assert_eq!(verdict(b"13.0.5", Some(id)), Verdict::Supported);
    assert_eq!(verdict(b"13.0.4", Some(id)), Verdict::UnsupportedVersion);
    assert_eq!(verdict(b"", Some(id)), Verdict::UnsupportedVersion);
    assert_eq!(verdict(b"13.0.5", Some(&[0u8; 20])), Verdict::UnsupportedBuild);
    assert_eq!(verdict(b"13.0.5", Some(&id[..19])), Verdict::UnsupportedBuild);
    assert_eq!(verdict(b"13.0.5", None), Verdict::BuildIdMissing);
    assert_eq!(Verdict::Supported.result(), "installed");
    for v in [Verdict::UnsupportedVersion, Verdict::UnsupportedBuild, Verdict::BuildIdMissing] {
        assert_eq!(v.result(), "unsupported_version");
        assert_ne!(v.reason(), "ok");
    }
}

#[test]
fn display_values_are_log_safe() {
    let mut name = [0u8; 16];
    name[..6].copy_from_slice(b"13.0.5");
    assert_eq!(model::display_version(&name), b"13.0.5");
    assert_eq!(model::display_version(b"no-nul-16-bytes!"), b"no-nul-16-bytes!");
    assert_eq!(Printable(b"13.0.5").to_string(), "13.0.5");
    assert_eq!(Printable(b"1 \x01").to_string(), "1??");
    assert_eq!(Printable(b"").to_string(), "none");
    assert_eq!(Hex(&[]).to_string(), "none");
    assert_eq!(Hex(&[0x0a, 0xff]).to_string(), "0aff");
}

/// FACT check against the owned ELF (`<smash-training-lobby>/research`, one level above this worktree).
/// A missing ELF fails the test (WORKAROUND-DECISIONS row 32); a build without the research tree
/// opts out explicitly with `SKIP_OWNED_ELF_FACT=1`, which prints a visible SKIPPED line.
#[test]
fn the_owned_1305_elf_carries_the_build_id_where_the_model_says() {
    use std::io::{Read, Seek, SeekFrom};
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../research/game-13.0.5/main-13.0.5.elf");
    let Ok(mut elf) = std::fs::File::open(path) else {
        assert!(
            std::env::var_os("SKIP_OWNED_ELF_FACT").is_some(),
            "owned 13.0.5 ELF missing at {}: link ../research, or set SKIP_OWNED_ELF_FACT=1 to skip",
            path
        );
        println!("SKIPPED the_owned_1305_elf_carries_the_build_id_where_the_model_says: {} absent", path);
        return;
    };
    // .rodata: vaddr 0x39c9000 at file offset 0x39ca000 (section header of the owned ELF).
    let file_offset = (model::MAIN_BUILD_ID_NOTE_OFFSET - 0x39c9000 + 0x39ca000) as u64;
    let mut buf = [0u8; 36];
    elf.seek(SeekFrom::Start(file_offset)).unwrap();
    elf.read_exact(&mut buf).unwrap();
    assert_eq!(gnu_build_id_at(&buf), Some(&model::MAIN_BUILD_ID_BYTES[..]));
}
