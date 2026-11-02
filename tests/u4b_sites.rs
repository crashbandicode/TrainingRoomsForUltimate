//! Stream u4b's resident effect-texture helper (`src/runtime/ui/effect_texture.rs`, `src/model/effect_texture.rs`) against the owned 13.0.5
//! ELF: every row's 64 bytes, the manager slot the helper decodes, and every structure offset the walk relies on, checked against the
//! instructions of the game's own functions that read and write those structures.

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::effect_texture as e;
use model::sites_u4b::{self, SITES};
use model::sites_ui::decode_adrp_ldr;

fn words(window: &[u8; 64]) -> [u32; 16] {
    let mut out = [0u32; 16];
    for (i, w) in out.iter_mut().enumerate() {
        *w = u32::from_le_bytes([window[4 * i], window[4 * i + 1], window[4 * i + 2], window[4 * i + 3]]);
    }
    out
}

fn row(name: &str) -> [u32; 16] {
    words(&SITES.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("row {name}")).guard)
}

/// `movz wN, #imm` (and `movk wN, #imm, lsl 16`) pairs of a window as the 32-bit values they build, with the register.
fn built_values(w: &[u32; 16]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for pair in w.windows(2) {
        let (movz, movk) = (pair[0], pair[1]);
        let rd = movz & 0x1f;
        if movz & 0xffe0_0000 == 0x5280_0000 && movk & 0xffe0_0000 == 0x72a0_0000 && movk & 0x1f == rd {
            out.push((rd, ((movk >> 5) & 0xffff) << 16 | ((movz >> 5) & 0xffff)));
        }
    }
    out
}

/// `ldr xT, [xN, #off]` (unsigned offset, 64-bit).
fn ldr_x(rt: u32, rn: u32, off: u32) -> u32 {
    0xf940_0000 | ((off / 8) << 10) | (rn << 5) | rt
}

/// `ldr wT, [xN, #off]`.
fn ldr_w(rt: u32, rn: u32, off: u32) -> u32 {
    0xb940_0000 | ((off / 4) << 10) | (rn << 5) | rt
}

/// `str xT, [xN, #off]`.
fn str_x(rt: u32, rn: u32, off: u32) -> u32 {
    0xf900_0000 | ((off / 8) << 10) | (rn << 5) | rt
}

#[test]
fn u4b_rows_match_the_owned_elf() {
    let Some(main) = common::owned_main("u4b_rows_match_the_owned_elf") else { return };
    for site in &SITES {
        assert_eq!(main.window(site.offset), site.guard, "{}", site.name);
    }
}

#[test]
fn no_u4b_row_overlaps_another_or_a_u4_row() {
    for (i, a) in SITES.iter().enumerate() {
        for b in SITES.iter().skip(i + 1) {
            assert!(!a.overlaps(b), "{} overlaps {}", a.name, b.name);
        }
        for b in model::sites_u4::SITES.iter() {
            assert!(!a.overlaps(b), "{} overlaps {}", a.name, b.name);
        }
    }
}

#[test]
fn the_manager_slot_decodes_from_the_instance_words_and_is_the_games_symbol() {
    let w = row("effmgr_instance_words");
    let pc = sites_u4b::EFFMGR_INSTANCE_WORDS_OFFSET as u64;
    assert_eq!(decode_adrp_ldr(pc, w[0], w[1]), Some(e::MANAGER_SLOT as u64));
    // `ldr x0, [x8]` (the instance variable holds the manager), then `mov w1, #0x372; add x2, sp, #0x18; bl effect_load`.
    assert_eq!(w[2], 0xf940_0100);
    assert_eq!(w[3], 0x5280_6e41, "mov w1, #0x372");
    let Some(main) = common::owned_main("the_manager_slot_decodes_from_the_instance_words_and_is_the_games_symbol") else { return };
    let symbol = main.symbols.iter().find(|(name, _)| name == "_ZN3lib9SingletonINS_13EffectManagerEE9instance_E").expect("the instance symbol");
    assert_eq!(symbol.1, e::MANAGER_SLOT);
    assert!(e::MANAGER_SLOT > main.text.addr + main.text.size, "the slot is a data address");
}

#[test]
fn the_entry_layout_is_the_one_the_loader_writes() {
    let hash = built_values(&row("effmgr_entry_hash"));
    // The loader names the file entry `manager + 0x18000 + slot * 0xa8`: stride `mov w11, #0xa8` (not a movk pair), hash at +0x18008, refs at +0x18098.
    assert!(hash.contains(&(12, (e::ENTRY_BASE + e::ENTRY_HASH) as u32)), "{hash:?}");
    assert!(hash.contains(&(10, (e::ENTRY_BASE + 0x98) as u32)), "{hash:?}");
    let w = row("effmgr_entry_hash");
    assert_eq!(w[0], 0x5280_150b, "mov w11, #0xa8");
    assert_eq!(e::ENTRY_STRIDE, 0xa8);
    let data = built_values(&row("effmgr_entry_data"));
    assert!(data.contains(&(9, (e::ENTRY_BASE + e::ENTRY_DATA) as u32)), "{data:?}");
    let w = row("effmgr_entry_data");
    assert_eq!(w[0], 0x5280_1508, "mov w8, #0xa8");
    // The EFFN magic is compared bytewise: 'E' (0x45), 'F', 'F' (0x46), 'N' (0x4e).
    let magic: Vec<u32> = w.iter().filter(|x| **x & 0xffc0_03ff == 0x7100_011f).map(|x| (x >> 10) & 0xfff).collect();
    assert_eq!(&magic[..2], &[0x45, 0x46], "cmp w8, #'E'; cmp w8, #'F'");
    assert_eq!(&e::EFFN_MAGIC, b"EFFN");
}

#[test]
fn the_system_and_its_resources_are_where_the_walk_reads_them() {
    let base = built_values(&row("effmgr_system_base"));
    // `[sp+0x20] = manager + 0x193d8`, and the system is read at `+0xf8` from it: `manager + 0x194d0`.
    assert!(base.contains(&(8, 0x1_93d8)), "{base:?}");
    assert_eq!(0x1_93d8 + 0xf8, e::MANAGER_SYSTEM);
    let w = row("effmgr_system_base");
    // `ldrsh w8, [x11, #0xe]`: the EFFN block index.
    assert!(w.contains(&0x79c0_1d68), "ldrsh w8, [x11, #0xe]");
    assert_eq!(e::EFFN_BLOCK, 0xe);
    let w = row("effmgr_system_resources");
    let slots = built_values(&w);
    assert!(slots.contains(&(9, (e::ENTRY_BASE + e::ENTRY_RESOURCE) as u32)), "{slots:?}");
    assert!(w.contains(&ldr_x(8, 8, 0xf8)), "the system pointer");
    assert!(w.contains(&ldr_x(8, 8, e::SYSTEM_RESOURCES as u32)), "the resource array");
    // `ldr x8, [x8, w24, uxtw #3]`: index times 8.
    assert!(w.contains(&0xf878_5908), "ldr x8, [x8, w24, uxtw #3]");
    // `ldrb w9, [x8, #9]`: the resource's valid byte.
    assert!(w.contains(&0x3940_2509), "ldrb w9, [x8, #9]");
    assert_eq!(e::RESOURCE_VALID, 9);
}

#[test]
fn the_resource_keeps_its_textures_and_their_views_and_slots_where_the_walk_reads_them() {
    let w = row("vfx_texture_table");
    assert!(w.contains(&ldr_x(8, 19, e::RESOURCE_TEXTURES as u32)), "ldr x8, [x19, #0xa8]: the texture file");
    assert!(w.contains(&ldr_x(9, 8, 0x38)), "the table at file + 0x38 (its +4 is the count)");
    assert!(w.contains(&ldr_w(9, 9, 4)), "ldr w9, [x9, #4]");
    assert!(w.contains(&ldr_x(8, 8, e::FILE_TEXTURES as u32)), "the texture pointer array at file + 0x28");
    assert!(w.contains(&0xb900_a269), "str w9, [x19, #0xa0]: the count");
    assert_eq!((e::RESOURCE_TEXTURE_COUNT, e::RESOURCE_TEXTURES), (0xa0, 0xa8));
    let w = row("vfx_texture_slot");
    assert!(w.contains(&ldr_x(1, 24, e::BRTI_VIEW as u32)), "ldr x1, [x24, #0x88]: the texture view is the registration's second argument");
    assert!(w.contains(&str_x(8, 24, e::BRTI_SLOT as u32)), "str x8, [x24, #0x90]: the slot");
    // `ldrsw x8, [x19, #0xa0]`: the loop bound is the resource's texture count.
    assert!(w.contains(&(0xb980_0000 | ((e::RESOURCE_TEXTURE_COUNT as u32 / 4) << 10) | (19 << 5) | 8)), "the loop bound is the resource's count");
}

#[test]
fn the_emitters_keep_their_binary_and_the_slots_of_their_samplers_where_the_walk_reads_them() {
    let w = row("vfx_emitter_table");
    assert!(w.contains(&ldr_x(8, 8, 0x18)), "ldr x8, [x8, #0x18]: the emitter array of a set");
    assert!(w.contains(&ldr_w(10, 10, e::SET_EMITTER_COUNT as u32)), "ldr w10, [x10, #4]: the emitter count of a set");
    // `orr w24, wzr, #0x38` is the set stride; the emitter stride is `add x26, x26, #0x3e0` (outside this window); the three slots are 0x48, 0x50, 0x58.
    assert_eq!((e::SET_STRIDE, e::EMITTER_STRIDE, e::EMITTER_SLOTS, e::RESOURCE_SETS, e::RESOURCE_SET_COUNT), (0x38, 0x3e0, [0x48, 0x50, 0x58], 0x58, 0x4c));
    assert!(w.contains(&0xf800_0e99) || w.iter().any(|x| x & 0xffe0_0c00 == 0xf800_0c00), "str x25, [x20, #0x48]!: the first slot of an emitter");
    let w = row("vfx_emitter_sampler");
    assert!(w.contains(&0xf85c_8288), "ldur x8, [x20, #-0x38]: the emitter's binary is at emitter + 0x10 (x20 = emitter + 0x48)");
    assert!(w.contains(&ldr_x(2, 8, 0x9f0)), "the first sampler id at emitter binary + 0x9f0");
    assert!(w.contains(&ldr_x(2, 8, 0xa10)), "the second at + 0xa10");
    assert!(w.contains(&ldr_x(2, 8, 0xa30)), "the third at + 0xa30");
    assert_eq!(e::EMTR_BYTES, 0xac0);
    assert_eq!((e::emtr::SAMPLERS, e::emtr::SAMPLER_STRIDE), (0x9f0, 0x20));
    assert_eq!(e::EMITTER_EMTR, 0x10);
}

#[test]
fn the_registration_row_is_the_function_the_game_registers_texture_views_with() {
    let Some(main) = common::owned_main("the_registration_row_is_the_function_the_game_registers_texture_views_with") else { return };
    let w = main.text_words();
    // The effect manager passes it (as `adrp`/`add`) to the particle system's registration (`bl 0x9ab70` follows in the same function).
    let target = sites_u4b::UI_REGISTER_VIEW_OFFSET as u64;
    let near = |from: usize, to: usize| {
        (from / 4..to / 4 - 1).any(|i| {
            let (adrp, add) = (w[i], w[i + 1]);
            if adrp & 0x9f00_0000 != 0x9000_0000 || add & 0xffc0_0000 != 0x9100_0000 || adrp & 0x1f != (add >> 5) & 0x1f {
                return false;
            }
            let imm = (((adrp >> 5) & 0x7_ffff) << 2) | ((adrp >> 29) & 3);
            let imm = ((imm << 11) as i32 >> 11) as i64;
            let page = (((i * 4) as u64) & !0xfff).wrapping_add_signed(imm << 12);
            page + u64::from((add >> 10) & 0xfff) == target
        })
    };
    assert!(near(0x3561500, 0x3561560), "the loader's `register` argument");
}
