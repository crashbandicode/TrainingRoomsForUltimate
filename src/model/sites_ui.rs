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

/// The five words the two anchors pin (SPEC-core-05 §4.1): the instructions the hook sits on and the
/// four after it, up to the holder layout's virtual Calculate (`blr x8`).
pub const UI_PASS_PIN: [u32; 5] = [0xf942_2ee0, 0xf940_0008, 0xf940_4d08, 0xaa13_03e1, 0x2a1f_03e2];
pub const UI_REGISTER_PIN: [u32; 5] = [0xf942_2ea0, 0xf940_0008, 0xf940_4d08, 0x3200_03e2, 0xaa13_03e1];

/// The manager's global slot, as the `adrp`/`ldr` words decode (`main+0x593ba90`, `.bss`).
pub const UI_MANAGER_SLOT_OFFSET: usize = 0x593_ba90;
/// `ui_manager_global`'s two words.
pub const UI_MANAGER_ADRP_LDR: [u32; 2] = [0xb001_0efc, 0xf945_4b88];

pub static UI_SITES: [Site; 4] = [    Site {
        name: "ui_archive_alloc",
        feature: Feature::Ui,
        offset: 0x3774704,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0xe3, 0xe6, 0x06, 0x94, 0xa0, 0x05, 0x00, 0xb4, 0xe1, 0x03, 0x15, 0xaa, 0xe2, 0x03, 0x17, 0xaa,
            0xc0, 0xb6, 0x00, 0xf9, 0x72, 0x2e, 0x09, 0x94, 0xc1, 0xb6, 0x40, 0xf9, 0xc2, 0x57, 0x00, 0xd0,
            0x42, 0x54, 0x39, 0x91, 0xe0, 0x03, 0x16, 0xaa, 0xe3, 0x03, 0x1f, 0xaa, 0xe4, 0x03, 0x1f, 0xaa,
            0xe5, 0x03, 0x1f, 0xaa, 0xc1, 0xba, 0x00, 0xf9, 0xb9, 0x36, 0x23, 0x97, 0xc0, 0xad, 0x00, 0x36,
        ],
        note: "Inline hook on the `bl` to the allocator inside the UI holder constructor (`main+0x3773770`), after the layout archive was decompressed: x21 is the decompressed bytes, x1, x23 and x24 their size (`mov x1, x24` precedes the call). The hook replaces x21 with the plugin's archive and the three size registers with its size; the game then allocates that size and copies from x21 (SPEC-core-05 §4.1). Only the `info_training` archive is replaced.",
    },
    Site {
        name: "ui_pass_calculate",
        feature: Feature::Ui,
        offset: 0x375efa8,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0xe0, 0x2e, 0x42, 0xf9, 0x08, 0x00, 0x40, 0xf9, 0x08, 0x4d, 0x40, 0xf9, 0xe1, 0x03, 0x13, 0xaa,
            0xe2, 0x03, 0x1f, 0x2a, 0x00, 0x01, 0x3f, 0xd6, 0xe0, 0x03, 0x13, 0xaa, 0x4f, 0x97, 0x23, 0x97,
            0xe0, 0x03, 0x13, 0xaa, 0xe1, 0x03, 0x1f, 0x2a, 0x5c, 0x97, 0x23, 0x97, 0xe8, 0x3a, 0x46, 0xb9,
            0x08, 0x05, 0x00, 0x11, 0x94, 0x22, 0x00, 0x91, 0x9f, 0x02, 0x15, 0xeb, 0x08, 0x05, 0x00, 0x12,
        ],
        note: "Inline anchor inside the UI manager's per-frame pass (`main+0x375e260`), on the instruction before the holder's virtual Calculate (`ldr x0, [x23, #0x458]` starts the five-word pin): the plugin's per-layout tick runs here, after the holder's animation update and before its Calculate, on the UI job thread (SPEC-core-05 §4.1, §5).",
    },
    Site {
        name: "ui_register_calculate",
        feature: Feature::Ui,
        offset: 0x3784ad4,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0xa0, 0x2e, 0x42, 0xf9, 0x08, 0x00, 0x40, 0xf9, 0x08, 0x4d, 0x40, 0xf9, 0xe2, 0x03, 0x00, 0x32,
            0xe1, 0x03, 0x13, 0xaa, 0x00, 0x01, 0x3f, 0xd6, 0xe0, 0x03, 0x13, 0xaa, 0x84, 0x00, 0x23, 0x97,
            0xe0, 0x03, 0x13, 0xaa, 0xe1, 0x03, 0x1f, 0x2a, 0x91, 0x00, 0x23, 0x97, 0xa8, 0x7a, 0x47, 0x39,
            0x08, 0x75, 0x1d, 0x12, 0x89, 0x06, 0x1f, 0x12, 0x28, 0x01, 0x08, 0x2a, 0xa8, 0x7a, 0x07, 0x39,
        ],
        note: "Inline anchor inside the UI manager's insert (`main+0x37849c0`), before the initial Calculate of a newly registered holder (x21): the registration tick, on the main thread (SPEC-core-05 §4.1).",
    },
    Site {
        name: "ui_manager_global",
        feature: Feature::Ui,
        offset: 0x375e280,
        kind: SeamKind::Relied,
        guard: [
            0xfc, 0x0e, 0x01, 0xb0, 0x88, 0x4b, 0x45, 0xf9, 0x09, 0x01, 0x40, 0xf9, 0x3f, 0xe1, 0x10, 0x39,
            0x13, 0x01, 0x40, 0xf9, 0x68, 0xea, 0x50, 0x39, 0x28, 0x02, 0x00, 0x34, 0x74, 0x02, 0x05, 0x91,
            0xe0, 0x03, 0x14, 0xaa, 0xc7, 0x8d, 0x09, 0x94, 0x75, 0x5a, 0x56, 0xa9, 0xbf, 0x02, 0x16, 0xeb,
            0xe0, 0x00, 0x00, 0x54, 0xa1, 0x86, 0x40, 0xf8, 0xe0, 0x03, 0x13, 0xaa, 0xf1, 0x9a, 0x00, 0x94,
        ],
        note: "Relied-on `adrp x28, ...; ldr x8, [x28, #0xa90]` in the UI manager's per-frame pass: the plugin decodes the manager's global slot (`main+0x593ba90`) from these two words (position independent) and never writes it. The pass's own entry words are not relied on: another plugin may have hooked them (SPEC-core-05 §6).",
    },
];

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
pub fn pin_matches(words: [u32; 5], pin: [u32; 5]) -> bool {
    words == pin
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manager_slot_decodes_from_the_pass_words() {
        let slot = decode_adrp_ldr(UI_MANAGER_GLOBAL_OFFSET as u64, UI_MANAGER_ADRP_LDR[0], UI_MANAGER_ADRP_LDR[1]);
        assert_eq!(slot, Some(UI_MANAGER_SLOT_OFFSET as u64));
    }

    #[test]
    fn the_decode_refuses_other_words_and_other_registers() {
        assert_eq!(decode_adrp_ldr(0, 0xd503_201f, UI_MANAGER_ADRP_LDR[1]), None);
        assert_eq!(decode_adrp_ldr(0, UI_MANAGER_ADRP_LDR[0], 0xd503_201f), None);
        // ldr x8, [x27, ...] instead of [x28, ...]
        assert_eq!(decode_adrp_ldr(0, UI_MANAGER_ADRP_LDR[0], (UI_MANAGER_ADRP_LDR[1] & !0x3e0) | (27 << 5)), None);
    }

    #[test]
    fn every_row_is_a_hook_or_a_relied_word_of_the_ui_feature() {
        for site in &UI_SITES {
            assert_eq!(site.feature, Feature::Ui);
        }
        assert_eq!(UI_SITES.iter().filter(|s| matches!(s.kind, SeamKind::Hook { .. })).count(), 3);
    }
}