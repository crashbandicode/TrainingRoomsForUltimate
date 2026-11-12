//! rc.8 R2 lobby speed: every offset and constant the runtime forms, decoded from the guard words of the `lobby_speed`
//! seam rows (the rows equal the owned 13.0.5 ELF: `native_seams::tests::compared_rows_match_the_owned_elf`). Mounted
//! from `engine` (it needs both the rows and `model::lobby_speed`).

use crate::engine::seams::native_seams::rows_lobby_speed::ROWS;
use crate::engine::seams::native_seams::{HookForm, Placement, SeamKind};
use crate::model::lobby_speed::{native, phase_word, sched};

fn row(label: &str) -> &'static crate::engine::seams::native_seams::Seam {
    ROWS.iter().find(|r| r.label == label).unwrap_or_else(|| panic!("{label}"))
}

fn word(label: &str, n: usize) -> (usize, u32) {
    let r = row(label);
    (r.offset + 4 * n, u32::from_le_bytes(r.guard[4 * n..4 * n + 4].try_into().unwrap()))
}

/// LDR/STR/LDRSW (unsigned offset): the byte offset.
fn uimm(w: u32) -> usize {
    assert_eq!(w & 0x3b00_0000, 0x3900_0000, "{w:#010x} is not a load/store with an unsigned offset");
    let scale = if w & 0xff80_0000 == 0xb980_0000 { 2 } else { w >> 30 };
    (((w >> 10) & 0xfff) << scale) as usize
}

fn adrp(pc: usize, w: u32) -> usize {
    assert_eq!(w & 0x9f00_0000, 0x9000_0000, "{w:#010x} is not ADRP");
    let imm = (((w >> 29) & 3) | (((w >> 5) & 0x7_ffff) << 2)) as i64;
    let imm = (imm << 43) >> 43;
    ((pc as i64 & !0xfff) + imm * 0x1000) as usize
}

fn add_imm(w: u32) -> usize {
    assert_eq!(w & 0x7f80_0000, 0x1100_0000, "{w:#010x} is not ADD (immediate)");
    (((w >> 10) & 0xfff) << if (w >> 22) & 1 == 1 { 12 } else { 0 }) as usize
}

/// SUBS (immediate) into the zero register: `cmp wN|xN, #imm{, lsl #12}`.
fn cmp_imm(w: u32) -> usize {
    assert_eq!(w & 0x7f80_001f, 0x7100_001f, "{w:#010x} is not CMP (immediate)");
    (((w >> 10) & 0xfff) << if (w >> 22) & 1 == 1 { 12 } else { 0 }) as usize
}

fn bl(pc: usize, w: u32) -> usize {
    assert_eq!(w >> 26, 0b100101, "{w:#010x} is not BL");
    let imm = ((w & 0x03ff_ffff) as i64) << 38 >> 38;
    (pc as i64 + imm * 4) as usize
}

/// `adrp` + an unsigned-offset access on the same page: the address.
fn page_plus(label: &str, adrp_at: usize, access_at: usize) -> usize {
    let (pc, a) = word(label, adrp_at);
    adrp(pc, a) + uimm(word(label, access_at).1)
}

fn class_update_is_a_mid_inline_hook_on_mov_x0_x20_before_the_flow_check() {
    let r = row("lobby_speed.class_update");
    assert_eq!(r.offset, native::CLASS_UPDATE_SITE);
    assert!(matches!(r.kind, SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid }));
    assert_eq!(word("lobby_speed.class_update", 0).1, 0xaa14_03e0, "mov x0, x20");
    assert_eq!(word("lobby_speed.class_update", 1).1, 0xaa01_03f3, "mov x19, x1");
    let (pc, w) = word("lobby_speed.class_update", 2);
    assert_eq!(bl(pc, w), 0x14f_8940, "bl FUN_014f8940, the battle flow check");
    // The hook in the runtime source sits on the row.
    let source = include_str!("../runtime/lobby/lobby_speed.rs");
    assert!(source.contains(&format!("#[skyline::hook(offset = {:#x}, inline)]", native::CLASS_UPDATE_SITE)));
}

fn the_task_entry_runs_the_scheduler_then_the_class_update() {
    const L: &str = "lobby_speed.task_entry";
    assert_eq!(page_plus(L, 2, 3), native::SCHED_GLOBAL, "ldr x8, [main+0x52c33d8]");
    assert_eq!(word(L, 4).1, 0xf940_0100, "ldr x0, [x8]: the scheduler is **word");
    let (pc, w) = word(L, 7);
    assert_eq!(bl(pc, w), 0x137_b3f0, "bl FUN_0137b3f0");
    assert_eq!(word(L, 8).1, 0xf940_0288, "ldr x8, [x20]");
    assert_eq!(uimm(word(L, 9).1), 0x50, "the class's update slot");
    assert_eq!(word(L, 12).1, 0xd63f_0100, "blr x8");
}

fn the_paused_word_is_the_one_the_entry_reads() {
    const L: &str = "lobby_speed.paused_flag";
    assert_eq!(page_plus(L, 6, 7), native::PAUSED, "ldr w22, [main+0x52b92e8]");
    assert_eq!(word(L, 7).1 & 0x1f, 22, "into w22, the scheduler's paused argument");
}

fn the_slow_state_offsets_come_from_the_slow_step() {
    const L: &str = "lobby_speed.slow_step";
    assert_eq!(page_plus(L, 0, 1), native::SLOW_STATE + native::CURRENT_N_AT, "ldrb w8, [current N]");
    assert_eq!(cmp_imm(word(L, 2).1), 1, "cmp w8, #1: one update per period");
    let (pc, a) = word(L, 4);
    assert_eq!(adrp(pc, a) + add_imm(word(L, 5).1), native::SLOW_STATE, "add x8, x8, #0x184");
    assert_eq!(uimm(word(L, 6).1), native::TARGET_N_AT, "ldrb w9, [x8, #5]: the target");
    assert_eq!(uimm(word(L, 10).1), native::LOGIC_FRAMES_AT, "ldr w10, [x8]: the logic frames");
    assert_eq!(uimm(word(L, 11).1), native::SECOND_COUNTER_AT, "ldr w11, [x8, #8]: the second counter");
    assert_eq!(uimm(word(L, 13).1), native::CURRENT_N_AT, "strb w9, [x8, #4]: the target latched");
    assert_eq!(uimm(word(L, 15).1), native::SUB_AT, "strh wzr, [x8, #6]: the sub-counters");
    const P: &str = "lobby_speed.slow_period";
    assert_eq!(page_plus(P, 0, 1), native::SLOW_STATE + native::SUB_AT, "ldrb w10, [the sub-counter]");
    assert_eq!(uimm(word(P, 12).1), native::SUB_AT, "strb wzr, [x9, #6]: the period ends at 0");
    assert_eq!(uimm(word(P, 13).1), native::CURRENT_N_AT, "strb w8, [x9, #4]: the target latched at the end");
}

fn the_scheduler_layout_comes_from_its_walk() {
    const S: &str = "lobby_speed.sched_setup";
    assert_eq!(word(S, 2).1, 0x1280_0015, "mov w21, #-1");
    assert_eq!(word(S, 3).1, 0x321c_07f6, "orr w22, wzr, #0x30");
    assert_eq!(sched::STRIDE, 0x30);
    const P: &str = "lobby_speed.sched_phase";
    assert_eq!(cmp_imm(word(P, 2).1), 8, "phases 1..=8 store their number, 0 and 9 store -1");
    for (phase, expected) in [(0, u32::MAX), (1, 1), (8, 8), (9, u32::MAX)] {
        assert_eq!(phase_word(phase), expected);
    }
    assert_eq!(word(P, 6).1, 0xb900_0268, "str w8, [x19]: the phase word at S+0");
    assert_eq!(uimm(word(P, 7).1), sched::START_AT, "ldr x9, [x10, #0x78]");
    assert_eq!(word(P, 8).1, 0xd345_fd28, "lsr x8, x9, #5");
    assert_eq!(word(P, 9).1, 0x927d_dd0b, "and x11, x8, #0x7fffffffffffff8: (index / 256) * 8");
    let ldp = word(P, 10).1;
    assert_eq!(ldp & 0xffc0_0000, 0xa940_0000, "ldp x8, x10, [x10, #imm]");
    assert_eq!((((ldp >> 15) & 0x7f) * 8) as usize, sched::BEGIN_AT);
    assert_eq!(sched::END_AT, sched::BEGIN_AT + 8);
    const E: &str = "lobby_speed.sched_entry";
    assert_eq!(uimm(word(E, 1).1), sched::SIZE_AT, "ldr x10, [x10, #0x80]");
    assert_eq!(word(E, 14).1, 0xf940_0308, "ldr x8, [x24]: the entry");
    assert_eq!(uimm(word(E, 15).1), sched::ENTRY_GROUP_AT, "ldrsw x9, [x8, #8]: the group");
    const T: &str = "lobby_speed.sched_task";
    assert_eq!(cmp_imm(word(T, 0).1), sched::MASKED_MAX as usize, "cmp w9, #9");
    assert_eq!(word(T, 2).1, 0x8b09_0a69, "add x9, x19, x9, lsl #2");
    assert_eq!(uimm(word(T, 3).1), sched::MASKS_AT, "ldr w9, [x9, #4]: the group's mask");
    assert_eq!(uimm(word(T, 5).1), sched::ENTRY_TASK_AT, "ldr x0, [x8, #0x40]: the task");
    assert_eq!(word(T, 7).1, 0xf940_0008, "ldr x8, [x0]: its vtable");
    assert_eq!(uimm(word(T, 8).1), sched::TICK_SLOT, "ldr x8, [x8, #0x30]: its tick");
    assert_eq!(word(T, 9).1, 0xd63f_0100, "blr x8 with the task in x0");
    assert_eq!(add_imm(word(T, 11).1), sched::ELEMENT, "add x24, x24, #16");
    assert_eq!(cmp_imm(word(T, 13).1), sched::PER_BLOCK * sched::ELEMENT, "cmp x8, #0x1000: a block");
}

fn the_skipped_tick_is_the_registry_walker() {
    const L: &str = "lobby_speed.walker_tick";
    assert_eq!(row(L).offset, native::WALKER_TICK);
    assert_eq!(page_plus(L, 4, 5), 0x52c_3910, "ldr x8, [main+0x52c3910]: the registry");
    let (pc, a) = word(L, 6);
    assert_eq!(adrp(pc, a) + add_imm(word(L, 7).1), 0x506_21a0, "the inner lambda that ticks each registry object");
}

fn every_lobby_speed_row_is_compared_and_scoped_to_the_waiting_room() {
    use crate::engine::seams::native_seams::Scopes;
    assert_eq!(ROWS.len(), 10);
    for r in ROWS {
        assert_eq!(r.group, "lobby_speed", "{}", r.label);
        assert!(r.compared(), "{}", r.label);
        assert_eq!(r.scopes, Scopes::LOBBY_WAITING, "{}", r.label);
        assert_eq!(r.feature, "", "{}", r.label);
    }
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("class_update_is_a_mid_inline_hook_on_mov_x0_x20_before_the_flow_check", class_update_is_a_mid_inline_hook_on_mov_x0_x20_before_the_flow_check as fn()),
        ("the_task_entry_runs_the_scheduler_then_the_class_update", the_task_entry_runs_the_scheduler_then_the_class_update as fn()),
        ("the_paused_word_is_the_one_the_entry_reads", the_paused_word_is_the_one_the_entry_reads as fn()),
        ("the_slow_state_offsets_come_from_the_slow_step", the_slow_state_offsets_come_from_the_slow_step as fn()),
        ("the_scheduler_layout_comes_from_its_walk", the_scheduler_layout_comes_from_its_walk as fn()),
        ("the_skipped_tick_is_the_registry_walker", the_skipped_tick_is_the_registry_walker as fn()),
        ("every_lobby_speed_row_is_compared_and_scoped_to_the_waiting_room", every_lobby_speed_row_is_compared_and_scoped_to_the_waiting_room as fn()),
    ]);
}
