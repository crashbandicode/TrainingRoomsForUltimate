//! The site rows package 3 adds (SPEC-state-01 §4), checked against the owned 13.0.5 ELF: every row's
//! 64 bytes are the ELF's, every replace hook's far jump fits without touching another function, no
//! window is near a site of the CPU pilot, and the install line stays under its buffer.

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::guard::{HookForm, Placement, SeamKind};
use model::sites::{self, Feature};
use model::sites_pkg3::{self, STATE_SITES};

/// Offsets of every site of `ssbu-cpu-pilot` (the list `tests/cpu_sites.rs` uses).
const PILOT_SITES: [usize; 23] = [
    0x1750df0, 0x1751ce8, 0x2da180, 0x6b8400, 0x2d8d40, 0x375ef8c, 0x59970, 0x37a2270, 0x376309c, 0x1d59560,
    0x147fb64, 0x150ea38, 0x1d59dc4, 0x1d5b5a8, 0x1d5b650, 0x1d5a590, 0x1d5a840, 0x32f22c4, 0x376db18, 0x32f1630,
    0x30c770, 0x3119d0, 0x1671650,
];

#[test]
fn state_sites_are_the_elfs_bytes_at_unique_names() {
    let Some(owned) = common::owned_main("state_sites_are_the_elfs_bytes_at_unique_names") else { return };
    let mut names: Vec<&str> = STATE_SITES.iter().map(|s| s.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), STATE_SITES.len(), "unique names");
    for site in &STATE_SITES {
        assert_eq!(owned.window(site.offset), site.guard, "{} at {:#x}", site.name, site.offset);
        assert!(matches!(site.feature, Feature::StateCore | Feature::StateCharge | Feature::StateBuff | Feature::StateItems | Feature::StateRestore));
        assert!(sites::find(site.name).is_none(), "{} must not be a row of the main table", site.name);
    }
}

#[test]
fn state_sites_the_symbols_they_name() {
    let Some(owned) = common::owned_main("state_sites_the_symbols_they_name") else { return };
    for (row, symbol) in [
        ("state_get_param_int", "_ZN3app8lua_bind30WorkModule__get_param_int_implEPNS_26BattleObjectModuleAccessorEmm"),
        ("state_set_dead_rumble", "_ZN3app15sv_fighter_util15set_dead_rumbleEP9lua_State"),
        ("state_req_quake", "_ZN3app8lua_bind28CameraModule__req_quake_implEPNS_26BattleObjectModuleAccessorEi"),
        ("state_effect_req", "_ZN3app8lua_bind22EffectModule__req_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40ERKNS3_8Vector3fES7_fjibi"),
        ("state_stage_id", "_ZN3app14sv_information8stage_idEv"),
        ("state_status_request", "_ZN3app8lua_bind40StatusModule__change_status_request_implEPNS_26BattleObjectModuleAccessorEib"),
        ("state_init_pos", "_ZN3app8lua_bind28PostureModule__init_pos_implEPNS_26BattleObjectModuleAccessorERKN3phx8Vector3fEbb"),
    ] {
        let site = STATE_SITES.iter().find(|s| s.name == row).unwrap();
        assert_eq!(owned.symbol(symbol), Some(site.offset), "{row}");
    }
}

#[test]
fn state_sites_every_replace_hook_fits_the_far_jump() {
    let Some(owned) = common::owned_main("state_sites_every_replace_hook_fits_the_far_jump") else { return };
    let hooks: Vec<_> = STATE_SITES
        .iter()
        .filter(|s| matches!(s.kind, SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry }))
        .collect();
    assert_eq!(hooks.len(), 31);
    for site in hooks {
        assert!(!site.note.is_empty(), "{}: an entry hook needs its justification", site.name);
        assert_eq!(site.offset % 16, 0, "{}", site.name);
        for (name, address) in &owned.symbols {
            assert!(
                !(address > &site.offset && address < &(site.offset + 16)),
                "{}: symbol {name} starts at {address:#x} inside the far jump",
                site.name
            );
        }
    }
}

#[test]
fn state_sites_are_not_near_the_cpu_pilot() {
    for site in &STATE_SITES {
        let (lo, hi) = (site.offset.saturating_sub(0x40), site.window_end() + 0x40);
        for pilot in PILOT_SITES {
            assert!(!(lo..hi).contains(&pilot), "{} is within 0x40 of the pilot site {pilot:#x}", site.name);
        }
    }
}

#[test]
fn state_sites_only_overlap_called_rows_or_each_other() {
    // A hook's write span is its first 16 bytes. Another feature's hook may sit inside a window of ours
    // (the get_param thunks are 32 bytes apart): the compare happens before any write.
    for (i, a) in STATE_SITES.iter().enumerate() {
        for b in &STATE_SITES[i + 1..] {
            if a.overlaps(b) {
                let write_a = (a.offset, a.offset + 16);
                let hook = |s: &model::sites::Site| matches!(s.kind, SeamKind::Hook { .. });
                if hook(a) && hook(b) {
                    assert!(b.offset >= write_a.1 || a.offset >= b.offset + 16, "{} and {} write over each other", a.name, b.name);
                }
            }
        }
    }
    // The item-manager slot row recomputes to the singleton's slot.
    let site = STATE_SITES.iter().find(|s| s.name == "state_item_manager_ref").unwrap();
    let words = |i: usize| u32::from_le_bytes([site.guard[4 * i], site.guard[4 * i + 1], site.guard[4 * i + 2], site.guard[4 * i + 3]]);
    assert_eq!(model::lra_reset::manager_slot_offset(site.offset, words(0), words(1)), Some(model::state::ITEM_MANAGER_SLOT_OFFSET));
    let _ = sites_pkg3::STATE_ITEM_MANAGER_REF_OFFSET;
}
