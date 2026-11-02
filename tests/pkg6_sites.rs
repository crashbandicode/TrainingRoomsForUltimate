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

const SYMBOLS: [(&str, &str); 19] = [
    ("p1_attack_is_infliction", "_ZN3app8lua_bind39AttackModule__is_infliction_status_implEPNS_26BattleObjectModuleAccessorEj"),
    ("p1_manager_entry_count", "_ZN3app8lua_bind32FighterManager__entry_count_implEPNS_14FighterManagerE"),
    ("p1_manager_is_result_mode", "_ZN3app8lua_bind35FighterManager__is_result_mode_implEPNS_14FighterManagerE"),
    ("p1_manager_is_ready_go", "_ZN3app8lua_bind32FighterManager__is_ready_go_implEPNS_14FighterManagerE"),
    ("p1_entry_current_fighter_id", "_ZN3app8lua_bind37FighterEntry__current_fighter_id_implEPNS_12FighterEntryE"),
    ("roster_have_item_kind", "_ZN3app8lua_bind35ItemModule__get_have_item_kind_implEPNS_26BattleObjectModuleAccessorEi"),
    ("roster_posture_pos", "_ZN3app8lua_bind23PostureModule__pos_implEPNS_26BattleObjectModuleAccessorE"),
    ("roster_find_battle_object", "_ZN3app8lua_bind54BattleObjectManager__is_active_find_battle_object_implEPNS_19BattleObjectManagerEj"),
    ("roster_status_is_changing", "_ZN3app8lua_bind30StatusModule__is_changing_implEPNS_26BattleObjectModuleAccessorE"),
    ("probe_attack_data", "_ZN3app8lua_bind30AttackModule__attack_data_implEPNS_26BattleObjectModuleAccessorEib"),
    ("probe_attack_is_attack", "_ZN3app8lua_bind28AttackModule__is_attack_implEPNS_26BattleObjectModuleAccessorEib"),
    ("probe_attack_power_add_status", "_ZN3app8lua_bind35AttackModule__power_add_status_implEPNS_26BattleObjectModuleAccessorE"),
    ("probe_attack_power_mul", "_ZN3app8lua_bind28AttackModule__power_mul_implEPNS_26BattleObjectModuleAccessorE"),
    ("probe_attack_power_mul_2nd", "_ZN3app8lua_bind32AttackModule__power_mul_2nd_implEPNS_26BattleObjectModuleAccessorE"),
    ("probe_attack_power_mul_3rd", "_ZN3app8lua_bind32AttackModule__power_mul_3rd_implEPNS_26BattleObjectModuleAccessorE"),
    ("probe_attack_power_mul_4th", "_ZN3app8lua_bind32AttackModule__power_mul_4th_implEPNS_26BattleObjectModuleAccessorE"),
    ("probe_attack_power_mul_5th", "_ZN3app8lua_bind32AttackModule__power_mul_5th_implEPNS_26BattleObjectModuleAccessorE"),
    ("probe_attack_power_mul_status", "_ZN3app8lua_bind35AttackModule__power_mul_status_implEPNS_26BattleObjectModuleAccessorE"),
    ("probe_damage_is_damage_lock", "_ZN3app8lua_bind33DamageModule__is_damage_lock_implEPNS_26BattleObjectModuleAccessorE"),
];

/// The standalone pilot's sites (`tests/cpu_sites.rs`).
const PILOT_SITES: [usize; 23] = [
    0x1750df0, 0x1751ce8, 0x2da180, 0x6b8400, 0x2d8d40, 0x375ef8c, 0x59970, 0x37a2270, 0x376309c, 0x1d59560,
    0x147fb64, 0x150ea38, 0x1d59dc4, 0x1d5b5a8, 0x1d5b650, 0x1d5a590, 0x1d5a840, 0x32f22c4, 0x376db18, 0x32f1630,
    0x30c770, 0x3119d0, 0x1671650,
];

/// The relied-on game layouts (no symbol: instructions of the game that read or write the fields package 6 reads): F9's No. of
/// CPUs path (root load, reader, pause-menu writer) and F11's active-cell read.
const RELIED: [&str; 4] = ["roster_cpu_quantity_root", "roster_cpu_quantity_reader", "roster_cpu_quantity_writer", "entry_active_cell"];

fn find(name: &str) -> &'static model::sites::Site {
    SITES.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("no row {name}"))
}

#[test]
fn rows_are_the_symbols_they_name() {
    let Some(owned) = common::owned_main("pkg6_rows_are_the_symbols_they_name") else { return };
    assert_eq!(SYMBOLS.len(), SITES.len() - RELIED.len(), "every called row names its symbol");
    for (row, symbol) in SYMBOLS {
        assert_eq!(owned.symbol(symbol), Some(find(row).offset), "{row} is not {symbol}");
    }
}

#[test]
fn every_row_matches_the_owned_elf() {
    let Some(owned) = common::owned_main("pkg6_every_row_matches_the_owned_elf") else { return };
    for site in &SITES {
        let relied = RELIED.contains(&site.name);
        let kind = if relied { SeamKind::Relied } else { SeamKind::Called };
        assert_eq!(site.kind, kind, "{}: called functions and the relied-on game layouts of F9/F11, never hooked", site.name);
        assert_eq!(owned.window(site.offset), site.guard, "{} at {:#x}", site.name, site.offset);
        assert_eq!(site.guard.len(), GUARD_BYTES);
        let expected = if relied {
            sites::Feature::Roster
        } else if site.name.starts_with("p1_") {
            sites::Feature::P1Cpu
        } else if site.name.starts_with("roster_") {
            sites::Feature::Roster
        } else {
            sites::Feature::Probes
        };
        assert_eq!(site.feature, expected, "{}", site.name);
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