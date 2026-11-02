//! The package-5 sites (SPEC-core-05 §4.1) against the owned 13.0.5 ELF: every row's 64 bytes, the two
//! anchors' five-word pins, the archive hook's neighbourhood (the allocator call and the copy that
//! follows it), and the manager slot decoded from the pass words.

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::sites_ui::{self, UI_SITES};

fn word(window: &[u8; 64], index: usize) -> u32 {
    u32::from_le_bytes([window[4 * index], window[4 * index + 1], window[4 * index + 2], window[4 * index + 3]])
}

#[test]
fn ui_rows_match_the_owned_elf() {
    let Some(main) = common::owned_main("ui_rows_match_the_owned_elf") else { return };
    for site in &UI_SITES {
        assert_eq!(main.window(site.offset), site.guard, "{}", site.name);
    }
}

#[test]
fn the_anchor_pins_and_the_manager_slot_are_what_the_spec_says() {
    let Some(main) = common::owned_main("the_anchor_pins_and_the_manager_slot_are_what_the_spec_says") else { return };
    for (offset, pin) in [
        (sites_ui::UI_PASS_CALCULATE_OFFSET, sites_ui::UI_PASS_PIN),
        (sites_ui::UI_REGISTER_CALCULATE_OFFSET, sites_ui::UI_REGISTER_PIN),
    ] {
        let window = main.window(offset);
        assert!(sites_ui::pin_matches(std::array::from_fn(|i| word(&window, i)), pin), "{offset:#x}");
        // The word after the pin is `blr x8`, the holder layout's virtual Calculate.
        assert_eq!(word(&window, 5), 0xd63f_0100, "{offset:#x} is followed by blr x8");
    }
    let window = main.window(sites_ui::UI_MANAGER_GLOBAL_OFFSET);
    assert_eq!([word(&window, 0), word(&window, 1)], sites_ui::UI_MANAGER_ADRP_LDR);
    assert_eq!(
        sites_ui::decode_adrp_ldr(sites_ui::UI_MANAGER_GLOBAL_OFFSET as u64, word(&window, 0), word(&window, 1)),
        Some(sites_ui::UI_MANAGER_SLOT_OFFSET as u64)
    );
}

#[test]
fn the_archive_hook_is_the_allocator_call_followed_by_the_copy() {
    let Some(main) = common::owned_main("the_archive_hook_is_the_allocator_call_followed_by_the_copy") else { return };
    let window = main.window(sites_ui::UI_ARCHIVE_ALLOC_OFFSET);
    // bl <allocator>; cbz x0, ...; mov x1, x21 (the decompressed bytes); mov x2, x23 (the size)
    assert_eq!(word(&window, 0) >> 26, 0b100101, "a bl");
    assert_eq!(word(&window, 1) & 0xff00_001f, 0xb400_0000, "cbz x0");
    assert_eq!(word(&window, 2), 0xaa15_03e1, "mov x1, x21");
    assert_eq!(word(&window, 3), 0xaa17_03e2, "mov x2, x23");
    // The instruction before the call sets the size argument: mov x1, x24.
    let before = main.window(sites_ui::UI_ARCHIVE_ALLOC_OFFSET - 4);
    assert_eq!(word(&before, 0), 0xaa18_03e1, "mov x1, x24 precedes the call");
}

#[test]
fn no_ui_site_window_overlaps_another_or_a_pilot_site() {
    const PILOT_SITES: [usize; 23] = [
        0x1750df0, 0x1751ce8, 0x2da180, 0x6b8400, 0x2d8d40, 0x375ef8c, 0x59970, 0x37a2270, 0x376309c, 0x1d59560,
        0x147fb64, 0x150ea38, 0x1d59dc4, 0x1d5b5a8, 0x1d5b650, 0x1d5a590, 0x1d5a840, 0x32f22c4, 0x376db18, 0x32f1630,
        0x30c770, 0x3119d0, 0x1671650,
    ];
    for (i, a) in UI_SITES.iter().enumerate() {
        for b in UI_SITES.iter().skip(i + 1) {
            assert!(!a.overlaps(b), "{} overlaps {}", a.name, b.name);
        }
        // A function we only call may be a site of the pilot too (both call it; either may be hooked by others).
        if matches!(a.kind, model::guard::SeamKind::Called) {
            continue;
        }
        for pilot in PILOT_SITES {
            // The pilot's `ui_pass` word at 0x375ef8c is 0x1c before the pass anchor and outside its window.
            assert!(!(a.offset..a.window_end()).contains(&pilot), "{} window holds the pilot site {pilot:#x}", a.name);
        }
    }
}