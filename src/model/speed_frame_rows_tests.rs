//! rc.15 1 Frame: every offset and constant the runtime forms, decoded from the guard words of the `speed_frame` seam
//! rows (the rows equal the owned 13.0.5 ELF: `native_seams::tests::compared_rows_match_the_owned_elf`). Mounted from
//! `engine` (it needs both the rows and the models).

use crate::engine::seams::native_seams::rows_speed_frame::ROWS;
use crate::engine::seams::native_seams::{HookForm, Placement, SeamKind};
use crate::model::game_speed::native as speed;
use crate::model::lobby_speed::{native, step_mask, FRAME_ROWS, HOLD_RUN, STEP_BUTTONS};

fn row(label: &str) -> &'static crate::engine::seams::native_seams::Seam {
    crate::engine::seams::native_seams::SEAMS
        .iter()
        .flat_map(|table| table.iter())
        .find(|r| r.label == label)
        .unwrap_or_else(|| panic!("{label}"))
}

fn word(label: &str, n: usize) -> (usize, u32) {
    let r = row(label);
    (r.offset + 4 * n, u32::from_le_bytes(r.guard[4 * n..4 * n + 4].try_into().unwrap()))
}

fn uimm(w: u32) -> usize {
    assert_eq!(w & 0x3b00_0000, 0x3900_0000, "{w:#010x} is not a load/store with an unsigned offset");
    (((w >> 10) & 0xfff) << (w >> 30)) as usize
}

fn adrp(pc: usize, w: u32) -> usize {
    assert_eq!(w & 0x9f00_0000, 0x9000_0000, "{w:#010x} is not ADRP");
    let imm = (((w >> 29) & 3) | (((w >> 5) & 0x7_ffff) << 2)) as i64;
    ((pc as i64 & !0xfff) + ((imm << 43) >> 43) * 0x1000) as usize
}

fn add_imm(w: u32) -> usize {
    assert_eq!(w & 0x7f80_0000, 0x1100_0000, "{w:#010x} is not ADD (immediate)");
    ((w >> 10) & 0xfff) as usize
}

fn cmp_imm(w: u32) -> usize {
    assert_eq!(w & 0x7f80_001f, 0x7100_001f, "{w:#010x} is not CMP (immediate)");
    ((w >> 10) & 0xfff) as usize
}

fn bl(pc: usize, w: u32) -> usize {
    assert_eq!(w >> 26, 0b100101, "{w:#010x} is not BL");
    (pc as i64 + (((w & 0x03ff_ffff) as i64) << 38 >> 38) * 4) as usize
}

fn page_plus(label: &str, adrp_at: usize, access_at: usize) -> usize {
    let (pc, a) = word(label, adrp_at);
    adrp(pc, a) + uimm(word(label, access_at).1)
}

fn the_entry_hold_is_a_mid_inline_hook_after_the_paused_word_load() {
    const L: &str = "speed_frame.entry_hold";
    let r = row(L);
    assert_eq!(r.offset, native::ENTRY_HOLD_SITE);
    assert!(matches!(r.kind, SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid }));
    assert_eq!(word(L, 0).1, 0xf944_8908, "ldr x8, [x8, #0x910]: not PC-relative");
    assert_eq!(word(L, 1).1, 0xf940_0101, "ldr x1, [x8]");
    // The paused word into w22 two words before, and `this` in x20 (`lobby_speed.paused_flag`).
    let flag = row("lobby_speed.paused_flag");
    assert_eq!(flag.offset + 0x1c, native::ENTRY_HOLD_SITE - 8, "ldr w22 is at .135ca30");
    let ldr = u32::from_le_bytes(flag.guard[0x1c..0x20].try_into().unwrap());
    assert_eq!((ldr & 0x1f, uimm(ldr)), (22, speed::PAUSED - 0x52b_9000), "ldr w22, [x8, #0x2e8]");
    let mov = u32::from_le_bytes(flag.guard[0x10..0x14].try_into().unwrap());
    assert_eq!(mov, 0xaa00_03f4, "mov x20, x0: the entry's object");
    // w22 is read only by the slow step's skip and the scheduler's paused argument.
    assert_eq!(word("speed_frame.entry_skip", 8).1, 0x3500_0a96, "cbnz w22: past the slow step");
    assert_eq!(word("speed_frame.entry_skip_last", 6).1, 0x3500_0496, "cbnz w22 on the other path");
    let source = include_str!("../runtime/lobby/lobby_speed.rs");
    assert!(source.contains(&format!("#[skyline::hook(offset = {:#x}, inline)]", native::ENTRY_HOLD_SITE)));
}

fn the_walker_skips_an_object_only_on_the_global_paused_word() {
    const L: &str = "speed_frame.walker_paused";
    assert_eq!(page_plus(L, 4, 5), speed::PAUSED, "ldr w8, [main+0x52b92e8]");
    assert_eq!(word(L, 7).1, 0x3940_2e88, "ldrb w8, [x20, #11]: the object's paused-skip byte");
}

fn the_training_step_is_the_mode_objects_and_its_button_is_zl() {
    assert_eq!(cmp_imm(word("speed_frame.step_mode", 3).1), crate::model::game_speed::Speed::OneFrame.value() as usize);
    const I: &str = "speed_frame.step_input";
    let (pc, a) = word(I, 4);
    assert_eq!(adrp(pc, a) + add_imm(word(I, 5).1), 0x509_ebd0, "the button functor's vtable");
    assert_eq!(word(I, 12).1, 0x2a1f_03e3, "mov w3, wzr: newly pressed");
    let (pc, w) = word(I, 14);
    assert_eq!(bl(pc, w), 0x177_e8f0);
    assert_eq!(word("speed_frame.step_hold", 0).1, 0x3200_03e3, "orr w3, wzr, #1: held");
    assert_eq!(cmp_imm(word("speed_frame.step_repeat", 7).1), usize::from(HOLD_RUN), "cmp w8, #18");
    // The functor's mask by controller style, from the table.
    const M: &str = "speed_frame.step_mask";
    let (pc, a) = word(M, 11);
    assert_eq!(adrp(pc, a) + add_imm(word(M, 12).1), native::STEP_BUTTONS_AT);
    assert_eq!(row("speed_frame.step_buttons").offset, native::STEP_BUTTONS_AT);
    let table = &row("speed_frame.step_buttons").guard;
    for (i, mask) in STEP_BUTTONS.iter().enumerate() {
        assert_eq!(u32::from_le_bytes(table[4 * i..4 * i + 4].try_into().unwrap()), *mask, "style {}", i + 1);
        assert_eq!(step_mask(i as u32 + 1), *mask);
    }
    assert_eq!(STEP_BUTTONS[0], crate::model::chords::bit::ZL, "ZL on a Pro Controller");
    assert_eq!(STEP_BUTTONS[6], crate::model::chords::bit::L, "the GameCube L trigger's raw bit");
}

fn the_resume_is_the_mode_objects_step_run() {
    const R: &str = "speed_frame.step_run";
    assert_eq!(page_plus(R, 0, 1), speed::PAUSED, "ldr w8, [main+0x52b92e8]");
    assert_eq!(word(R, 3).1, 0x3610_00c8, "tbz w8, #2: Training's pause bit");
    assert_eq!(page_plus(R, 4, 5), speed::STEP_BITS, "ldr w9, [main+0x52b92ec]");
    assert_eq!(word(R, 6).1, 0x121d_7929, "and w9, w9, #0xfffffffb");
    let (pc, w) = word(R, 8);
    assert_eq!(bl(pc, w), speed::PAUSE_SYNC_FN);
    assert_eq!(row("speed_frame.pause_sync").offset, speed::PAUSE_SYNC_FN);
    assert!(matches!(row("speed_frame.pause_sync").kind, SeamKind::Called));
    const P: &str = "speed_frame.step_pause";
    assert_eq!(word(P, 3).1, 0x321e_0129, "orr w9, w9, #0x4");
    let (pc, w) = word(P, 5);
    assert_eq!(bl(pc, w), speed::PAUSE_SYNC_FN);
}

fn every_speed_frame_row_is_compared_and_gated() {
    assert_eq!(ROWS.len(), 13);
    for r in ROWS {
        assert_eq!(r.group, "speed_frame", "{}", r.label);
        assert!(r.compared(), "{}", r.label);
        assert!(FRAME_ROWS.contains(&r.label), "{} is in the gate", r.label);
    }
    for label in FRAME_ROWS {
        let _ = row(label);
    }
}

/// This module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_entry_hold_is_a_mid_inline_hook_after_the_paused_word_load", the_entry_hold_is_a_mid_inline_hook_after_the_paused_word_load as fn()),
        ("the_walker_skips_an_object_only_on_the_global_paused_word", the_walker_skips_an_object_only_on_the_global_paused_word as fn()),
        ("the_training_step_is_the_mode_objects_and_its_button_is_zl", the_training_step_is_the_mode_objects_and_its_button_is_zl as fn()),
        ("the_resume_is_the_mode_objects_step_run", the_resume_is_the_mode_objects_step_run as fn()),
        ("every_speed_frame_row_is_compared_and_gated", every_speed_frame_row_is_compared_and_gated as fn()),
    ]);
}
