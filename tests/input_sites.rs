//! The sites package 4 adds (SPEC-input-01 §4, SPEC-input-05 §4), checked against the owned 13.0.5 ELF:
//! each Called row is the symbol it claims, every row's 64 bytes are the ELF's, the AI epilogue hook sits
//! on the one path that follows the controller write, and no site or guard window is near a site of the
//! CPU pilot plugin or of another row of this plugin.

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::guard::{SeamKind, GUARD_BYTES};
use model::sites;
use model::sites_pkg4::{self as pkg4, SITES};

const SYMBOLS: [(&str, &str); 6] = [
    ("posture_lr", "_ZN3app8lua_bind22PostureModule__lr_implEPNS_26BattleObjectModuleAccessorE"),
    ("stop_is_stop", "_ZN3app8lua_bind24StopModule__is_stop_implEPNS_26BattleObjectModuleAccessorE"),
    ("work_get_int", "_ZN3app8lua_bind24WorkModule__get_int_implEPNS_26BattleObjectModuleAccessorEi"),
    ("work_set_int", "_ZN3app8lua_bind24WorkModule__set_int_implEPNS_26BattleObjectModuleAccessorEii"),
    ("model_set_color_rgb", "_ZN3app8lua_bind31ModelModule__set_color_rgb_implEPNS_26BattleObjectModuleAccessorEfffNS_16MODEL_COLOR_TYPEE"),
    ("fighter_manager_get_entry", "_ZN3app8lua_bind38FighterManager__get_fighter_entry_implEPNS_14FighterManagerENS_14FighterEntryIDE"),
];

/// The pilot's sites (see `tests/cpu_sites.rs`).
const PILOT_SITES: [usize; 23] = [
    0x1750df0, 0x1751ce8, 0x2da180, 0x6b8400, 0x2d8d40, 0x375ef8c, 0x59970, 0x37a2270, 0x376309c, 0x1d59560,
    0x147fb64, 0x150ea38, 0x1d59dc4, 0x1d5b5a8, 0x1d5b650, 0x1d5a590, 0x1d5a840, 0x32f22c4, 0x376db18, 0x32f1630,
    0x30c770, 0x3119d0, 0x1671650,
];

fn find(name: &str) -> &'static model::sites::Site {
    SITES.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("no row {name}"))
}

#[test]
fn rows_are_the_symbols_they_name() {
    let Some(owned) = common::owned_main("rows_are_the_symbols_they_name") else { return };
    for (row, symbol) in SYMBOLS {
        assert_eq!(owned.symbol(symbol), Some(find(row).offset), "{row} is not {symbol}");
    }
}

#[test]
fn every_row_matches_the_owned_elf() {
    let Some(owned) = common::owned_main("every_row_matches_the_owned_elf") else { return };
    for site in &SITES {
        assert_eq!(owned.window(site.offset), site.guard, "{} at {:#x}", site.name, site.offset);
    }
    // Names are unique across both tables.
    for site in &SITES {
        assert!(sites::find(site.name).is_none(), "{} is also a row of sites.rs", site.name);
        assert_eq!(SITES.iter().filter(|s| s.name == site.name).count(), 1);
    }
}

#[test]
fn the_ai_epilogue_follows_the_controller_write() {
    let Some(owned) = common::owned_main("the_ai_epilogue_follows_the_controller_write") else { return };
    let word = |at: usize| {
        let w = owned.window(at);
        u32::from_le_bytes([w[0], w[1], w[2], w[3]])
    };
    // `ldr x8, [x19, #0x8]` (the CPU controller pointer of the AI object) twice before the writes of the
    // sticks and the clamped sticks, and `str w9, [x8, #0xc]` (the buttons) just before the epilogue.
    assert_eq!(word(0x2dbbac), 0xf940_0668);
    assert_eq!(word(0x2dbbb4), 0xf900_0d09, "str x9, [x8, #0x18]");
    assert_eq!(word(0x2dbbbc), 0xf900_0909, "str x9, [x8, #0x10]");
    assert_eq!(word(0x2dbbd0), 0xf900_2109, "str x9, [x8, #0x40]");
    assert_eq!(word(0x2dbbe0), 0xb900_0d09, "str w9, [x8, #0xc]");
    // The hooked word is the first of the epilogue (an `ldp d9, d8`, not PC-relative) and `x19` is
    // restored only by the `ldp x20, x19` after it, so the hook sees the AI object in x19.
    assert_eq!(word(pkg4::AI_EPILOGUE_OFFSET), 0x6d51_23e9);
    assert_eq!(word(0x2dbc4c), 0xa956_4ff4, "ldp x20, x19, [sp, #0x160]");
    assert_eq!(word(0x2dbc64), 0xd65f_03c0, "ret");
    // The function's entry word is the pilot's hook site; this hook is far from it.
    let entry = 0x2da180usize;
    assert!(pkg4::AI_EPILOGUE_OFFSET > entry + GUARD_BYTES);
}

#[test]
fn the_ai_epilogue_is_the_only_return_of_the_function() {
    let Some(owned) = common::owned_main("the_ai_epilogue_is_the_only_return_of_the_function") else { return };
    let words = owned.text_words();
    let (start, end) = (0x2da180 / 4, 0x2dccb0 / 4);
    let rets = (start..end).filter(|i| words[*i] == 0xd65f_03c0).count();
    assert_eq!(rets, 1, "one `ret` between the AI function's entry and the next function");
    // No branch lands in the epilogue's words (so every path reaches the hook by falling into it).
    let target = |i: usize| -> Option<usize> {
        let w = words[i];
        let imm26 = |w: u32| (((w & 0x03ff_ffff) as i32) << 6 >> 6) as isize;
        let imm19 = |w: u32| ((((w >> 5) & 0x7_ffff) as i32) << 13 >> 13) as isize;
        let delta = if w & 0xfc00_0000 == 0x1400_0000 {
            imm26(w)
        } else if w & 0xff00_0010 == 0x5400_0000 || w & 0x7e00_0000 == 0x3400_0000 {
            imm19(w)
        } else {
            return None;
        };
        usize::try_from(i as isize + delta).ok()
    };
    for i in start..end {
        if let Some(t) = target(i) {
            let epilogue = pkg4::AI_EPILOGUE_OFFSET / 4;
            assert!(!(epilogue..=0x2dbc64 / 4).contains(&t), "word {:#x} branches into the epilogue", i * 4);
        }
    }
}

#[test]
fn no_row_window_is_near_a_pilot_site_or_another_row() {
    for site in &SITES {
        for pilot in PILOT_SITES {
            assert!(
                site.offset + GUARD_BYTES <= pilot || pilot + GUARD_BYTES <= site.offset,
                "{} at {:#x} is within 0x40 of the pilot site {pilot:#x}",
                site.name,
                site.offset
            );
        }
    }
    // Hook rows must not overlap another HOOK or each other's first words; reads of pinned words
    // (Relied rows) deliberately overlap their hook, which is the point of the pin.
    for (i, a) in SITES.iter().enumerate() {
        for b in &SITES[i + 1..] {
            let hooks = matches!(a.kind, SeamKind::Hook { .. }) && matches!(b.kind, SeamKind::Hook { .. });
            // The two Combo Display observers are 0x3c apart: their windows overlap, which boot's two
            // phases (compare every window, then write) handle (SPEC-core-01 §6 "Overlapping thunks").
            let allowed = a.name == "combo_display_a" && b.name == "combo_display_b";
            if hooks && !allowed {
                assert!(!a.overlaps(b), "{} and {} overlap", a.name, b.name);
            }
        }
    }
    // Against the whole of sites.rs: no package-1/2 hook word window overlaps one of ours.
    for a in &SITES {
        for b in sites::SITES.iter() {
            if matches!(a.kind, SeamKind::Hook { .. }) && matches!(b.kind, SeamKind::Hook { .. }) {
                assert!(!a.overlaps(b), "{} and {} overlap", a.name, b.name);
            }
        }
    }
}

#[test]
fn combo_display_pins_are_the_specs_words() {
    let Some(owned) = common::owned_main("combo_display_pins_are_the_specs_words") else { return };
    let words = |at: usize| {
        let w = owned.window(at);
        [0, 4, 8, 12].map(|o| u32::from_le_bytes([w[o], w[o + 1], w[o + 2], w[o + 3]]))
    };
    assert_eq!(words(0x13e5498), [0x396d_b114, 0x6b14_013f, 0x5400_02c0, 0x3400_0234]);
    assert_eq!(words(0x13e54d4), [0x396d_b108, 0x3908_c268, 0x3200_03e8, 0x3908_be68]);
}

#[test]
fn every_feature_has_rows() {
    assert_eq!(pkg4::sites_of(sites::Feature::Recording).count(), 7);
    assert_eq!(pkg4::sites_of(sites::Feature::Notices).count(), 5);
}
