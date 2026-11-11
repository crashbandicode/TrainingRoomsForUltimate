//! rc.8 game speed (`docs/design/rc8-binds.md`): every offset the runtime forms, decoded from the guard words of the
//! `game_speed` seam rows (the rows equal the owned 13.0.5 ELF: `native_seams::tests::compared_rows_match_the_owned_elf`).
//! Mounted from `engine` (it needs both the rows and `model::game_speed`).

use crate::engine::seams::native_seams::rows_game_speed::ROWS;
use crate::model::game_speed::{fresh_table, native, table, SEAM_ROWS, SPEED_FIELD, SPEED_TEXT_PANE};

// --- the guard words ------------------------------------------------------------------------------------------------

fn row(label: &str) -> &'static crate::engine::seams::native_seams::Seam {
    ROWS.iter().find(|r| r.label == label).unwrap_or_else(|| panic!("{label}"))
}

fn word(guard: [u8; 64], n: usize) -> u32 {
    u32::from_le_bytes(guard[4 * n..4 * n + 4].try_into().unwrap())
}

/// LDR/STR (unsigned offset, any size): the byte offset.
fn uimm(w: u32) -> usize {
    assert_eq!(w & 0x3b00_0000, 0x3900_0000, "{w:#010x} is not a load/store with an unsigned offset");
    (((w >> 10) & 0xfff) << (w >> 30)) as usize
}

/// Whether a load (as opposed to a store) of `bytes` bytes.
fn load(w: u32, bytes: u32) -> bool {
    1 << (w >> 30) == bytes && (w >> 22) & 1 == 1
}

fn store(w: u32, bytes: u32) -> bool {
    1 << (w >> 30) == bytes && (w >> 22) & 3 == 0
}

/// LDRB/STRB/STR (pre-index, `[xN, #imm]!`): the offset.
fn pre_index(w: u32) -> usize {
    assert_eq!(w & 0x3b20_0c00, 0x3800_0c00, "{w:#010x} is not a pre-indexed load/store");
    ((w >> 12) & 0x1ff) as usize
}

fn add_imm(w: u32) -> usize {
    assert_eq!(w & 0x7f80_0000, 0x1100_0000, "{w:#010x} is not ADD (immediate)");
    (((w >> 10) & 0xfff) << if (w >> 22) & 1 == 1 { 12 } else { 0 }) as usize
}

fn adrp(pc: usize, w: u32) -> usize {
    assert_eq!(w & 0x9f00_0000, 0x9000_0000, "{w:#010x} is not ADRP");
    let imm = (((w >> 29) & 3) | (((w >> 5) & 0x7_ffff) << 2)) as i64;
    let imm = (imm << 43) >> 43;
    ((pc as i64 & !0xfff) + imm * 0x1000) as usize
}

fn bl(pc: usize, w: u32) -> usize {
    assert_eq!(w >> 26, 0b100101, "{w:#010x} is not BL");
    let imm = ((w & 0x03ff_ffff) as i64) << 38 >> 38;
    (pc as i64 + imm * 4) as usize
}

/// ORR (immediate): the bitmask the instruction moves into its register.
fn orr_imm(w: u32) -> u64 {
    let sf = w >> 31;
    assert_eq!(w & 0x7f80_0000, 0x3200_0000, "{w:#010x} is not ORR (immediate)");
    let (n, immr, imms) = ((w >> 22) & 1, (w >> 16) & 0x3f, (w >> 10) & 0x3f);
    let len = 31 - ((n << 6) | (!imms & 0x3f)).leading_zeros();
    let esize = 1u32 << len;
    let (s, r) = (imms & (esize - 1), immr & (esize - 1));
    let ones = if s + 1 == 64 { u64::MAX } else { (1u64 << (s + 1)) - 1 };
    let mask = if esize == 64 { u64::MAX } else { (1u64 << esize) - 1 };
    let elem = if r == 0 { ones } else { ((ones >> r) | (ones << (esize - r))) & mask };
    let width = if sf == 1 { 64 } else { 32 };
    (0..width / esize).fold(0u64, |out, i| out | elem << (i * esize))
}

/// MOVZ then MOVK (`lsl #16`): the 32-bit value.
fn movz_movk(lo: u32, hi: u32) -> u32 {
    assert_eq!(lo & 0x7f80_0000, 0x5280_0000, "{lo:#010x} is not MOVZ");
    assert_eq!(hi & 0x7f80_0000, 0x7280_0000, "{hi:#010x} is not MOVK");
    assert_eq!((hi >> 21) & 3, 1, "MOVK lsl #16");
    ((lo >> 5) & 0xffff) | ((hi >> 5) & 0xffff) << 16
}

#[test]
fn game_speed_rows_decode_the_native_offsets() {
    let labels: Vec<_> = ROWS.iter().map(|r| r.label).collect();
    assert_eq!(labels, SEAM_ROWS, "the runtime gates every row of the group");
    for r in ROWS {
        assert_eq!(r.group, "game_speed");
    }
    // The pause menu: the root and the chain the apply follows, the field it reads, the field the HUD reads.
    let chain = row("game_speed.training_chain");
    let at = |r: &crate::engine::seams::native_seams::Seam, n: usize| (r.offset + 4 * n, word(r.guard, n));
    let (pc, w) = at(chain, 0);
    assert_eq!(adrp(pc, w) + uimm(word(chain.guard, 2)), native::TRAINING_ROOT);
    assert!(load(word(chain.guard, 1), 4) && uimm(word(chain.guard, 1)) == 0x194, "the applied value x25+0x194");
    let value = row("game_speed.training_value");
    let hops = [
        uimm(word(chain.guard, 4)),
        uimm(word(chain.guard, 5)),
        uimm(word(chain.guard, 6)),
        uimm(word(value.guard, 1)),
        uimm(word(value.guard, 2)),
        uimm(word(value.guard, 3)),
        uimm(word(value.guard, 4)),
    ];
    assert_eq!(hops, native::PANEL_CHAIN);
    assert_eq!(uimm(word(value.guard, 0)), 0xb38);
    assert!(load(word(value.guard, 5), 4));
    assert_eq!(uimm(word(value.guard, 5)), SPEED_FIELD);
    let hud = row("game_speed.hud_icon");
    assert_eq!((uimm(word(hud.guard, 1)), uimm(word(hud.guard, 2))), (0x80, SPEED_FIELD));
    // The setter's unchanged-value path: the pane, the store, the call with (menu, value, pane).
    let path = row("game_speed.speed_store_text");
    assert_eq!(add_imm(word(path.guard, 0)), SPEED_TEXT_PANE);
    assert!(store(word(path.guard, 6), 4));
    assert_eq!(uimm(word(path.guard, 6)), SPEED_FIELD);
    assert_eq!(word(path.guard, 5), 0xaa15_03e2, "mov x2, x21: the pane is the third argument");
    let (pc, w) = at(path, 7);
    assert_eq!(bl(pc, w), native::SPEED_TEXT_FN);
    assert_eq!(row("game_speed.speed_text").offset, native::SPEED_TEXT_FN);
    // The slow table: its global, the slots, the divisor beside in_use, the dirty byte.
    let rate = row("game_speed.rate_request");
    let (pc, w) = at(rate, 2);
    assert_eq!(adrp(pc, w) + uimm(word(rate.guard, 3)), native::SLOW_TABLE);
    assert_eq!(uimm(word(rate.guard, 4)), table::DIRTY_AT);
    let (pc, w) = at(rate, 6);
    assert_eq!(bl(pc, w), row("game_speed.max_divisor").offset);
    let max = row("game_speed.max_divisor");
    assert_eq!((uimm(word(max.guard, 0)), uimm(word(max.guard, 3))), (table::SLOTS_AT, table::SLOTS_AT + 1));
    let free_row = row("game_speed.slot_free");
    let (pc, w) = at(free_row, 4);
    assert_eq!(adrp(pc, w) + uimm(word(free_row.guard, 5)), native::SLOW_TABLE);
    assert_eq!(pre_index(word(free_row.guard, 8)), table::SLOTS_AT);
    let zero = word(free_row.guard, 10);
    assert!(store(zero, 1) && uimm(zero) == 0 && zero & 0x1f == 0x1f, "strb wzr, [x10]: in_use 0");
    assert_eq!(orr_imm(word(free_row.guard, 11)), 1);
    assert_eq!(uimm(word(free_row.guard, 12)), table::SLOTS_AT + 1);
    assert_eq!(uimm(word(free_row.guard, 13)), table::DIRTY_AT);
    let take_row = row("game_speed.slot_take");
    assert_eq!(orr_imm(word(take_row.guard, 0)), 1, "in_use 1");
    assert_eq!(orr_imm(word(take_row.guard, 3)), 4, "1/4's divisor");
    assert_eq!(uimm(word(take_row.guard, 4)), table::SLOTS_AT + 1);
    assert_eq!(uimm(word(take_row.guard, 5)), table::DIRTY_AT);
    // A new table: the create writes every slot {0, 1} and dirty 1, as `fresh_table`.
    let create = row("game_speed.table_create");
    let pattern = orr_imm(word(create.guard, 0)).to_le_bytes();
    assert_eq!(pattern, fresh_table()[table::SLOTS_AT..table::SLOTS_AT + 8]);
    assert_eq!(pre_index(word(create.guard, 4)), table::SLOTS_AT, "str x9, [x10, #0x20]!");
    let tail = movz_movk(word(create.guard, 10), word(create.guard, 11)).to_le_bytes();
    assert_eq!(uimm_stur(word(create.guard, 15)), 0x3e);
    assert_eq!(tail[..3], fresh_table()[0x3e..=table::DIRTY_AT], "slot 15 {{0, 1}} and dirty 1");
    // The teardown stores 0 into the global.
    let destroy = row("game_speed.table_destroy");
    let (pc, w) = at(destroy, 0);
    assert_eq!(adrp(pc, w) + uimm(word(destroy.guard, 1)), native::SLOW_TABLE);
    assert!(store(word(destroy.guard, 11), 8) && word(destroy.guard, 11) & 0x1f == 0x1f, "str xzr");
    assert_eq!(uimm(word(destroy.guard, 11)), native::SLOW_TABLE & 0xfff);
}

/// STUR (unscaled): the signed offset.
fn uimm_stur(w: u32) -> usize {
    assert_eq!(w & 0x3b20_0c00, 0x3800_0000, "{w:#010x} is not STUR/LDUR");
    ((w >> 12) & 0x1ff) as usize
}
