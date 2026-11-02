//! SPEC-input-03 §4: where the Input Delay hooks sit in the game's final input mapping, and why. The
//! function was hooked at its entry; it is now hooked after its prologue (playbook "Hook after the
//! prologue"). Facts checked on the owned 13.0.5 ELF:
//!
//! - `begin` (`main+0x1750dfc`, `mov x19, x2`): x2 is the out record and x23 the sign-extended player
//!   index (`sxtw x23, w1` at `0x1750df0`); it comes after the checks that leave the function for
//!   index -1 and for 8 or more.
//! - `end` (`main+0x1751ce8`): the function's one epilogue and its one `ret`; the final `bl 0x1752870`
//!   that completes the record is the instruction before it.
//! - every branch into the epilogue starts after `begin`, except the one that rejects index -1 (the
//!   path on which `begin` never runs and so leaves no note).

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::sites;

const FUNCTION: usize = 0x175_0dd0;
/// The function ends before the next one (`0x17521f0`, called three times from it); its out-of-line
/// tails (`0x17520c0` and others) sit after the epilogue.
const FUNCTION_END: usize = 0x175_21f0;

fn word(words: &[u32], offset: usize) -> u32 {
    words[offset / 4]
}

/// The target of a direct branch word at `at`, if it is `b`, `b.cond`, `cbz/cbnz` or `tbz/tbnz`.
fn target(at: usize, w: u32) -> Option<usize> {
    let sext = |v: u32, bits: u32| (((v << (32 - bits)) as i32) >> (32 - bits)) as i64;
    let delta = if w & 0xfc00_0000 == 0x1400_0000 {
        sext(w & 0x03ff_ffff, 26) * 4
    } else if w & 0xff00_0010 == 0x5400_0000 || w & 0x7e00_0000 == 0x3400_0000 {
        sext((w >> 5) & 0x7_ffff, 19) * 4
    } else if w & 0x7e00_0000 == 0x3600_0000 {
        sext((w >> 5) & 0x3fff, 14) * 4
    } else {
        return None;
    };
    usize::try_from(at as i64 + delta).ok()
}

#[test]
fn input_mapping_sites_are_after_the_prologue_and_at_the_one_epilogue() {
    let begin = sites::find("input_mapping_begin").unwrap();
    let end = sites::find("input_mapping_end").unwrap();
    assert_eq!((begin.offset, end.offset), (0x175_0dfc, 0x175_1ce8));
    let g = |s: &sites::Site, i: usize| u32::from_le_bytes([s.guard[4 * i], s.guard[4 * i + 1], s.guard[4 * i + 2], s.guard[4 * i + 3]]);
    // The hooked instructions.
    assert_eq!(g(begin, 0), 0xaa02_03f3, "mov x19, x2");
    assert_eq!(g(end, 0), 0xa947_7bfd, "ldp x29, x30, [sp, #0x70]");
    // Neither hook is at the function entry (`sub sp, sp, #0x80` at 0x1750dd0).
    assert!(begin.offset > FUNCTION + 8 * 4, "after the prologue's stores and frame setup");
    // The sites of a feature are all required.
    assert!(sites::feature_usable(sites::Feature::InputDelay, |_| true));
    assert!(!sites::feature_usable(sites::Feature::InputDelay, |s| s.name != "input_mapping_begin"));

    let Some(main) = common::owned_main("input_mapping_sites_are_after_the_prologue_and_at_the_one_epilogue") else {
        return;
    };
    let words = main.text_words();
    // Prologue and the sign extension that precede `begin`.
    assert_eq!(word(&words, FUNCTION), 0xd102_03ff, "sub sp, sp, #0x80");
    assert_eq!(word(&words, 0x175_0df0), 0x9340_7c37, "sxtw x23, w1");
    assert_eq!(word(&words, 0x175_0dfc), 0xaa02_03f3);
    // The call that completes the record, right before the epilogue, and the one `ret`.
    let bl = word(&words, 0x175_1ce4);
    assert_eq!(bl & 0xfc00_0000, 0x9400_0000, "bl");
    assert_eq!(0x175_1ce4 + ((((bl & 0x03ff_ffff) << 6) as i32 >> 6) as i64 * 4) as usize, 0x175_2870);
    let rets: Vec<usize> = (FUNCTION..FUNCTION_END).step_by(4).filter(|&at| word(&words, at) == 0xd65f_03c0).collect();
    assert_eq!(rets, [0x175_1cfc], "the function has one ret, in the epilogue");
    // Every branch into the epilogue: all start after `begin`, but the one that rejects index -1.
    let into: Vec<usize> = (FUNCTION..FUNCTION_END)
        .step_by(4)
        .filter(|&at| target(at, word(&words, at)) == Some(0x175_1ce8))
        .collect();
    assert_eq!(into, [0x175_0dec, 0x175_0e04, 0x175_0e34, 0x175_0ea4, 0x175_20c0], "{into:x?}");
    let before_begin: Vec<_> = into.iter().filter(|&&at| at < begin.offset).collect();
    assert_eq!(before_begin, [&0x175_0dec], "only the index check leaves before `begin`");
}
