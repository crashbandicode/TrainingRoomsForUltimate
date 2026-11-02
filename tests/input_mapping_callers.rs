//! SPEC-input-03 §4: where the Input Delay hooks sit, and why. The game's final input mapping
//! (`main+0x1750dd0`) is hooked by nobody here: the CPU pilot plugin on the card owns its inner
//! points (`fim_valid` 0x1750df0, `fim_exit` 0x1751ce8), and an entry hook is not "after the
//! prologue". The delay is applied in the function's callers, right after the call returns, so it
//! comes after the function and the pilot's hook, which is the order the former entry hook had.
//!
//! Facts checked on the owned 13.0.5 ELF:
//! - the function has exactly two callers, both direct `bl`, and nothing else references it: no other
//!   direct branch, no materialised address (`adrp`+`add`, `adr`, `movz`/`movk`), no pointer in the file;
//! - caller A (`main+0x17546f4`) is a loop over the 8 slots: x20 is the slot, x23 + x22 the slot's 8-byte
//!   out record (the call's x2), none of them written between the `note` site and the call; the call
//!   returns to a `b` that leads to the increment (`apply` site);
//! - caller B (`main+0x1755250`) is a single-slot wrapper: x8 is the out record (written once, at its
//!   entry), x2 the slot, neither written between the `note` site and the call; the `apply` site is the
//!   instruction after the call;
//! - none of our four sites, nor its 64-byte guard window, is within 0x40 of a site of the CPU pilot
//!   (list copied from its `src/model.rs`, commit 0162eed) or overlaps the function itself.

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::sites;

const FUNCTION: usize = 0x175_0dd0;
const CALL_A: usize = 0x175_46f4;
const CALL_B: usize = 0x175_5250;

/// Offsets of every site of `ssbu-cpu-pilot` (commit 0162eed, `src/model.rs`).
const PILOT_SITES: [usize; 23] = [
    0x1750df0, 0x1751ce8, 0x2da180, 0x6b8400, 0x2d8d40, 0x375ef8c, 0x59970, 0x37a2270, 0x376309c, 0x1d59560,
    0x147fb64, 0x150ea38, 0x1d59dc4, 0x1d5b5a8, 0x1d5b650, 0x1d5a590, 0x1d5a840, 0x32f22c4, 0x376db18, 0x32f1630,
    0x30c770, 0x3119d0, 0x1671650,
];

const SITE_NAMES: [&str; 4] = ["input_caller_a_note", "input_caller_a_apply", "input_caller_b_note", "input_caller_b_apply"];

fn word(words: &[u32], offset: usize) -> u32 {
    words[offset / 4]
}

/// The target of a direct branch word at `at`: `b`, `bl`, `b.cond`, `cbz`/`cbnz`, `tbz`/`tbnz`.
fn target(at: usize, w: u32) -> Option<usize> {
    let sext = |v: u32, bits: u32| (((v << (32 - bits)) as i32) >> (32 - bits)) as i64;
    let delta = if w & 0x7c00_0000 == 0x1400_0000 {
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
fn the_sites_are_in_the_callers_and_not_near_the_pilot() {
    let sites: Vec<_> = SITE_NAMES.iter().map(|n| sites::find(n).unwrap()).collect();
    assert_eq!(
        sites.iter().map(|s| s.offset).collect::<Vec<_>>(),
        [0x175_46e0, 0x175_4720, 0x175_523c, 0x175_5254]
    );
    for s in &sites {
        assert!(sites::feature_usable(sites::Feature::InputDelay, |_| true));
        assert!(!sites::feature_usable(sites::Feature::InputDelay, |x| x.name != s.name), "{}", s.name);
        // Not in the function, not near the pilot's sites, and the 64-byte window holds none of them.
        assert!(!(FUNCTION..FUNCTION + 0x1400).contains(&s.offset), "{} is outside the function", s.name);
        for pilot in PILOT_SITES {
            let (lo, hi) = (s.offset.saturating_sub(0x40), s.window_end() + 0x40);
            assert!(!(lo..hi).contains(&pilot), "{} is within 0x40 of the pilot site {pilot:#x}", s.name);
        }
    }
    // The old in-function sites are gone.
    assert!(sites::find("input_mapping_begin").is_none() && sites::find("input_mapping_end").is_none());
}

#[test]
fn the_function_has_two_callers_and_the_registers_are_still_ours_at_the_sites() {
    let Some(main) = common::owned_main("the_function_has_two_callers_and_the_registers_are_still_ours_at_the_sites") else {
        return;
    };
    let words = main.text_words();
    for s in SITE_NAMES.iter().map(|n| sites::find(n).unwrap()) {
        assert_eq!(main.window(s.offset), s.guard, "{}", s.name);
    }
    // Every direct branch to the function.
    let callers: Vec<usize> = (0..words.len() * 4).step_by(4).filter(|&at| target(at, word(&words, at)) == Some(FUNCTION)).collect();
    assert_eq!(callers, [CALL_A, CALL_B]);
    for at in [CALL_A, CALL_B] {
        assert_eq!(word(&words, at) & 0xfc00_0000, 0x9400_0000, "bl at {at:#x}");
    }
    // No materialised address of the function: adrp+add, adr, movz+movk.
    for (i, &w) in words.iter().enumerate() {
        let at = i * 4;
        let rel = |w: u32| {
            let imm = ((w >> 5) & 0x7_ffff) << 2 | ((w >> 29) & 3);
            (((imm << 11) as i32) >> 11) as i64
        };
        if w & 0x9f00_0000 == 0x9000_0000 && ((at & !0xfff) as i64 + (rel(w) << 12)) as usize == FUNCTION & !0xfff {
            let follows = (1..5).filter_map(|k| words.get(i + k)).any(|n| n & 0xffc0_0000 == 0x9100_0000 && (n >> 10) & 0xfff == (FUNCTION & 0xfff) as u32);
            assert!(!follows, "adrp+add of the function at {at:#x}");
        }
        if w & 0x9f00_0000 == 0x1000_0000 {
            assert_ne!((at as i64 + rel(w)) as usize, FUNCTION, "adr at {at:#x}");
        }
        if w & 0xff80_0000 == 0x5280_0000 && (w >> 5) & 0xffff == (FUNCTION & 0xffff) as u32 {
            let next = words.get(i + 1).copied().unwrap_or(0);
            assert!(!(next & 0xff80_0000 == 0x7280_0000 && (next >> 5) & 0xffff == (FUNCTION >> 16) as u32), "movz/movk at {at:#x}");
        }
    }
    let needle = (FUNCTION as u64).to_le_bytes();
    assert!(!main.bytes.windows(8).any(|w| w == needle), "a pointer to the function in the file");

    // Caller A: add x2, x23, x22 / mov x0, x21 / mov x1, x20 / and w4, w9, w8 / bl, then `b` to the increment.
    assert_eq!(
        [0x175_46e0, 0x175_46e4, 0x175_46e8, 0x175_46ec, 0x175_46f0].map(|a| word(&words, a)),
        [0x1a9f_17e9, 0x8b16_02e2, 0xaa15_03e0, 0xaa14_03e1, 0x0a08_0124],
        "cset w9, eq / add x2, x23, x22 / mov x0, x21 / mov x1, x20 / and w4, w9, w8"
    );
    assert_eq!(target(0x175_46f8, word(&words, 0x175_46f8)), Some(0x175_4720));
    assert_eq!(word(&words, 0x175_4720), 0x9100_0694, "add x20, x20, #1");
    // Caller B: mov x0, x9 / cset / and x1, x2, #0xffffffff / mov x2, x8 / and w4, w11, w10 / bl / orr w0, wzr, #1.
    assert_eq!(
        [0x175_523c, 0x175_5240, 0x175_5244, 0x175_5248, 0x175_524c, 0x175_5254].map(|a| word(&words, a)),
        [0xaa09_03e0, 0x1a9f_17eb, 0x9240_7c41, 0xaa08_03e2, 0x0a0a_0164, 0x3200_03e0]
    );
    // x8 (B's out record) is written once, by `mov x8, x1` at its entry; x2 (its slot) never before the call.
    let writes = |range: std::ops::Range<usize>, reg: u32| -> Vec<usize> {
        range.step_by(4).filter(|&a| {
            let w = word(&words, a);
            // Data-processing forms that write Rd (mov/orr/add/sub/and/csel/cset/ldr/ldrsw), not stores or compares.
            let is_store_or_branch = w & 0x3b00_0000 == 0x3900_0000 && w & 0x0040_0000 == 0 || w & 0x7c00_0000 == 0x1400_0000 || w & 0xff00_0010 == 0x5400_0000;
            let rd_writes = w & 0x1f == reg && !is_store_or_branch && w & 0x1f != 31;
            let flags_only = w & 0x7f20_001f == 0x7100_001f || w & 0x7f20_001f == 0x3100_001f || w & 0x7f20_001f == 0x6b00_001f; // cmp / cmn / subs xzr
            rd_writes && !flags_only && !matches!(w & 0xff00_0000, 0x3400_0000 | 0x3500_0000 | 0xb400_0000 | 0xb500_0000 | 0x3600_0000 | 0x3700_0000)
        }).collect()
    };
    assert_eq!(writes(0x175_5160..0x175_5254, 8), [0x175_516c], "x8 is written only by `mov x8, x1`");
    assert!(writes(0x175_5160..0x175_523c, 2).is_empty(), "x2 is not written before the note site");
    assert!(writes(0x175_46e0..0x175_46f4, 20).is_empty() && writes(0x175_46e0..0x175_46f4, 22).is_empty() && writes(0x175_46e0..0x175_46f4, 23).is_empty());
}
