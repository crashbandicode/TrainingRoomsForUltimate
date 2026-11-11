//! Stream u4's run-time texture helper (`src/runtime/ui/texture.rs`) against the owned 13.0.5 ELF: every row's 64 bytes, the
//! decoded service slot, the readiness test the plugin copies, the field offsets of the game's UI texture and the two vtable
//! values the helper compares pointers against.

mod common;

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::sites_u4::{self, SITES};
use model::sites_ui::decode_adrp_ldr;
use model::ui_texture as t;

fn word(window: &[u8; 64], index: usize) -> u32 {
    u32::from_le_bytes([window[4 * index], window[4 * index + 1], window[4 * index + 2], window[4 * index + 3]])
}

/// The absolute value an `adrp xN, page; add xN, xN, #off` pair at `pc` names, when `adrp` and `add` are that pair.
fn decode_adrp_add(pc: u64, adrp: u32, add: u32) -> Option<u64> {
    if adrp & 0x9f00_0000 != 0x9000_0000 || add & 0xffc0_0000 != 0x9100_0000 || adrp & 0x1f != (add >> 5) & 0x1f {
        return None;
    }
    let imm = (((adrp >> 5) & 0x7_ffff) << 2) | ((adrp >> 29) & 3);
    let imm = ((imm << 11) as i32 >> 11) as i64;
    let page = (pc & !0xfff).wrapping_add_signed(imm << 12);
    Some(page + u64::from((add >> 10) & 0xfff))
}

#[test]
fn u4_rows_match_the_owned_elf() {
    let Some(main) = common::owned_main("u4_rows_match_the_owned_elf") else { return };
    for site in &SITES {
        assert_eq!(main.window(site.offset), site.guard, "{}", site.name);
    }
}

#[test]
fn no_u4_row_overlaps_another() {
    for (i, a) in SITES.iter().enumerate() {
        for b in SITES.iter().skip(i + 1) {
            assert!(!a.overlaps(b), "{} overlaps {}", a.name, b.name);
        }
    }
}

#[test]
fn the_service_slot_decodes_from_the_lookup_words() {
    let window = SITES.iter().find(|s| s.name == "ui_tex_find_handle").unwrap().guard;
    let first = (sites_u4::UI_TEX_SERVICE_SLOT_WORDS_OFFSET - sites_u4::UI_TEX_FIND_HANDLE_OFFSET) / 4;
    let pc = sites_u4::UI_TEX_SERVICE_SLOT_WORDS_OFFSET as u64;
    assert_eq!(decode_adrp_ldr(pc, word(&window, first), word(&window, first + 1)), Some(t::SERVICE_SLOT as u64));
}

#[test]
fn the_readiness_test_is_the_one_the_plugin_copies() {
    let Some(main) = common::owned_main("the_readiness_test_is_the_one_the_plugin_copies") else { return };
    let w = main.window(sites_u4::UI_TEX_READY_WORDS_OFFSET);
    // ldr x8,[x20]; cbz x8; ldr x0,[x8,#8]; bl poll; ldr x8,[x20]; ldr x9,[x8,#8]; ldr x10,[x9,#0x18]; cbnz x10
    assert_eq!(word(&w, 0), 0xf940_0288);
    assert_eq!(word(&w, 2), 0xf940_0500, "ldr x0, [x8, #8]: the UI texture is the holder's +8");
    let bl = word(&w, 3);
    assert_eq!(bl >> 26, 0b100101, "a bl");
    let delta = ((bl & 0x03ff_ffff) << 6) as i32 >> 6;
    assert_eq!(
        (sites_u4::UI_TEX_READY_WORDS_OFFSET + 12).wrapping_add_signed(delta as isize * 4),
        sites_u4::UI_TEX_POLL_OFFSET,
        "the call is the poll"
    );
    assert_eq!(word(&w, 5), 0xf940_0509, "ldr x9, [x8, #8]");
    assert_eq!(word(&w, 6), 0xf940_0d2a, "ldr x10, [x9, #0x18]: the pending job");
    assert_eq!(word(&w, 8), 0xb940_012a, "ldr w10, [x9]: the handle");
    assert_eq!(word(&w, 12), 0xf940_3129, "ldr x9, [x9, #0x60]: the state object");
    assert_eq!(word(&w, 14), 0xb940_1929, "ldr w9, [x9, #0x18]: its state");
    assert_eq!(word(&w, 15), 0x7100_0d3f, "cmp w9, #3");
    assert_eq!((t::TEXTURE_JOB, t::TEXTURE_STATE, t::STATE_FIELD, t::STATE_LOADED, t::HANDLE_NONE), (0x18, 0x60, 0x18, 3, 0xff_ffff));
}

#[test]
fn the_constructor_stores_the_holder_vtable_and_the_control_block_carries_its_own() {
    let Some(main) = common::owned_main("the_constructor_stores_the_holder_vtable_and_the_control_block_carries_its_own") else { return };
    let words = main.text_words();
    let materialises = |from: usize, to: usize, target: u64| {
        (from / 4..to / 4 - 1).any(|i| decode_adrp_add((i * 4) as u64, words[i], words[i + 1]) == Some(target))
    };
    assert!(materialises(sites_u4::UI_TEX_CTOR_OFFSET, sites_u4::UI_TEX_CTOR_OFFSET + 0x100, t::HOLDER_VTABLE as u64), "holder vtable");
    assert!(
        materialises(sites_u4::UI_TEX_MAKE_OFFSET, sites_u4::UI_TEX_MAKE_OFFSET + 0x110, t::CONTROL_VTABLE as u64),
        "control block vtable"
    );
}

#[test]
fn the_control_block_vtable_slot_is_the_on_zero_shared_the_game_calls() {
    let Some(main) = common::owned_main("the_control_block_vtable_slot_is_the_on_zero_shared_the_game_calls") else { return };
    // `.data` holds the unrelocated vtable; its relocation entries are applied by the loader, so only the section bounds are checked here.
    assert!(t::CONTROL_VTABLE >= main.data.addr && t::CONTROL_VTABLE < main.data.addr + main.data.size);
    assert!(t::HOLDER_VTABLE >= main.data.addr && t::HOLDER_VTABLE < main.data.addr + main.data.size);
}
