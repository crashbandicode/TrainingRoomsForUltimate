//! The sites package 5 (menu rendering and assets) hooks or relies on, in a table of their own so the
//! package's rows do not collide with the others' edits of `sites.rs` (stream rules). Same row type and
//! the same 64-byte guard rule; the bytes were read from the owned 13.0.5 ELF
//! (`tests/ui_sites.rs` re-checks every row against it). Offsets are from the start of `.text`.
//!
//! Layout archive hook, the two UI anchors and the manager global: SPEC-core-05 §4.1.

use super::guard::{HookForm, Placement, SeamKind};
use super::sites::{Feature, Site};

pub const UI_ARCHIVE_ALLOC_OFFSET: usize = 0x377_4704;
pub const UI_PASS_CALCULATE_OFFSET: usize = 0x375_efa8;
pub const UI_REGISTER_CALCULATE_OFFSET: usize = 0x378_4ad4;
pub const UI_MANAGER_GLOBAL_OFFSET: usize = 0x375_e280;
/// The game's pane text setter (pane, text, units, flags).
pub const UI_SET_TEXT_OFFSET: usize = 0x37a_2270;
/// The game's `app::fighter::is_ready_go().
pub const UI_IS_READY_GO_OFFSET: usize = 0x15c_e620;

/// The five words the two anchors pin (SPEC-core-05 §4.1): the instructions the hook sits on and the
/// four after it, up to the holder layout's virtual Calculate (`blr x8`).
#[cfg(test)]
pub const UI_PASS_PIN: [u32; 5] = [0xf942_2ee0, 0xf940_0008, 0xf940_4d08, 0xaa13_03e1, 0x2a1f_03e2];
#[cfg(test)]
pub const UI_REGISTER_PIN: [u32; 5] = [0xf942_2ea0, 0xf940_0008, 0xf940_4d08, 0x3200_03e2, 0xaa13_03e1];

/// The manager's global slot, as the `adrp`/`ldr` words decode (`main+0x593ba90`, `.bss`).
#[cfg(test)]
pub const UI_MANAGER_SLOT_OFFSET: usize = 0x593_ba90;
/// `ui_manager_global`'s two words.
#[cfg(test)]
pub const UI_MANAGER_ADRP_LDR: [u32; 2] = [0xb001_0efc, 0xf945_4b88];

pub static UI_SITES: [Site; 6] = include!(concat!(env!("OUT_DIR"), "/tables/sites_ui.UI_SITES.rs"));

/// The absolute address an `adrp xN, page` followed by `ldr xM, [xN, #off]` (64-bit, unsigned offset)
/// names, for the instruction pair at `pc`; `None` when the two words are not that pair or the registers
/// differ. Position independent: the same decode serves any load address (SPEC-core-05 §4.1: the pass's
/// manager global is read from these words, never from the entry, which another plugin may have hooked).
pub fn decode_adrp_ldr(pc: u64, adrp: u32, ldr: u32) -> Option<u64> {
    if adrp & 0x9f00_0000 != 0x9000_0000 || ldr & 0xffc0_0000 != 0xf940_0000 {
        return None;
    }
    let rd = adrp & 0x1f;
    let rn = (ldr >> 5) & 0x1f;
    if rd != rn {
        return None;
    }
    let imm = (((adrp >> 5) & 0x7_ffff) << 2) | ((adrp >> 29) & 3);
    let imm = ((imm << 11) as i32 >> 11) as i64;
    let page = (pc & !0xfff).wrapping_add_signed(imm << 12);
    Some(page.wrapping_add(u64::from((ldr >> 10) & 0xfff) * 8))
}

/// Whether the five words at an anchor are the pinned ones.
#[cfg(test)]
pub fn pin_matches(words: [u32; 5], pin: [u32; 5]) -> bool {
    words == pin
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn the_manager_slot_decodes_from_the_pass_words() {
        let slot = decode_adrp_ldr(UI_MANAGER_GLOBAL_OFFSET as u64, UI_MANAGER_ADRP_LDR[0], UI_MANAGER_ADRP_LDR[1]);
        assert_eq!(slot, Some(UI_MANAGER_SLOT_OFFSET as u64));
    }

    fn the_decode_refuses_other_words_and_other_registers() {
        assert_eq!(decode_adrp_ldr(0, 0xd503_201f, UI_MANAGER_ADRP_LDR[1]), None);
        assert_eq!(decode_adrp_ldr(0, UI_MANAGER_ADRP_LDR[0], 0xd503_201f), None);
        // ldr x8, [x27, ...] instead of [x28, ...]
        assert_eq!(decode_adrp_ldr(0, UI_MANAGER_ADRP_LDR[0], (UI_MANAGER_ADRP_LDR[1] & !0x3e0) | (27 << 5)), None);
    }

    fn every_row_is_a_hook_or_a_relied_word_of_the_ui_feature() {
        for site in &UI_SITES {
            assert_eq!(site.feature, Feature::Ui);
        }
        assert_eq!(UI_SITES.iter().filter(|s| matches!(s.kind, SeamKind::Hook { .. })).count(), 3);
        assert_eq!(UI_SITES.iter().filter(|s| matches!(s.kind, SeamKind::Called)).count(), 2);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_manager_slot_decodes_from_the_pass_words", the_manager_slot_decodes_from_the_pass_words as fn()),
            ("the_decode_refuses_other_words_and_other_registers", the_decode_refuses_other_words_and_other_registers as fn()),
            ("every_row_is_a_hook_or_a_relied_word_of_the_ui_feature", every_row_is_a_hook_or_a_relied_word_of_the_ui_feature as fn()),
        ]);
    }
    }
}