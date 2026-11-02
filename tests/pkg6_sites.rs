//! The sites package 6 adds for the CPU-as-P1 fallback (`sites_pkg6.rs`), checked against the owned 13.0.5 ELF: each row is the
//! symbol it claims (a `lua_bind` thunk the carried `cpu_pilot` calls through `smash::`), every row's 64 bytes are the ELF's, no name
//! or offset repeats a row of another package, and no guard window touches a site of the standalone CPU pilot plugin.

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::guard::{SeamKind, GUARD_BYTES};
use model::sites_pkg6::SITES;
use model::{sites, sites_pkg2, sites_pkg3, sites_pkg4, sites_ui};

const SYMBOLS: [(&str, &str); 5] = [
    ("p1_attack_is_infliction", "_ZN3app8lua_bind39AttackModule__is_infliction_status_implEPNS_26BattleObjectModuleAccessorEj"),
    ("p1_manager_entry_count", "_ZN3app8lua_bind32FighterManager__entry_count_implEPNS_14FighterManagerE"),
    ("p1_manager_is_result_mode", "_ZN3app8lua_bind35FighterManager__is_result_mode_implEPNS_14FighterManagerE"),
    ("p1_manager_is_ready_go", "_ZN3app8lua_bind32FighterManager__is_ready_go_implEPNS_14FighterManagerE"),
    ("p1_entry_current_fighter_id", "_ZN3app8lua_bind37FighterEntry__current_fighter_id_implEPNS_12FighterEntryE"),
];

/// The standalone pilot's sites (`tests/cpu_sites.rs`).
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
    let Some(owned) = common::owned_main("pkg6_rows_are_the_symbols_they_name") else { return };
    assert_eq!(SYMBOLS.len(), SITES.len());
    for (row, symbol) in SYMBOLS {
        assert_eq!(owned.symbol(symbol), Some(find(row).offset), "{row} is not {symbol}");
    }
}

#[test]
fn every_row_matches_the_owned_elf() {
    let Some(owned) = common::owned_main("pkg6_every_row_matches_the_owned_elf") else { return };
    for site in &SITES {
        assert_eq!(site.kind, SeamKind::Called, "{} is a called function, never hooked", site.name);
        assert_eq!(owned.window(site.offset), site.guard, "{} at {:#x}", site.name, site.offset);
        assert_eq!(site.guard.len(), GUARD_BYTES);
        assert_eq!(site.feature, sites::Feature::P1Cpu);
    }
}

#[test]
fn no_name_or_offset_repeats_another_packages_row() {
    let others: Vec<(&str, usize)> = sites::BASE_SITES
        .iter()
        .chain(sites_pkg2::SITES_PKG2.iter())
        .chain(sites_pkg3::STATE_SITES.iter())
        .chain(sites_pkg4::SITES.iter())
        .chain(sites_ui::UI_SITES.iter())
        .map(|s| (s.name, s.offset))
        .collect();
    for site in &SITES {
        assert_eq!(SITES.iter().filter(|s| s.name == site.name).count(), 1, "{} twice in sites_pkg6", site.name);
        for (name, offset) in &others {
            assert_ne!(*name, site.name, "{} is also a row elsewhere", site.name);
            assert_ne!(*offset, site.offset, "{} repeats the offset of {name}", site.name);
        }
    }
}

#[test]
fn no_guard_window_touches_a_site_of_the_standalone_pilot() {
    for site in &SITES {
        for pilot in PILOT_SITES {
            let distance = site.offset.abs_diff(pilot);
            assert!(distance >= GUARD_BYTES, "{} at {:#x} is {distance:#x} from the pilot site {pilot:#x}", site.name, site.offset);
        }
    }
}