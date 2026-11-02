//! The sites package 2 adds (SPEC-cpu-01 §4, SPEC-cpu-02 §4, SPEC-cpu-03 §4), checked against the owned
//! 13.0.5 ELF: each row's offset is the symbol it claims, every hooked thunk fits Skyline's 16-byte far
//! jump without touching the next function, no site or guard window is near a site of the CPU pilot
//! plugin, and the per-frame slot holds the original it replaces.

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::guard::{HookForm, Placement, SeamKind};
use model::sites::{self, Feature, Site};
use model::sites_pkg2;

/// (row, symbol) of every package-2 row that is a named function in the owned ELF.
const SYMBOLS: [(&str, &str); 42] = [
    ("lua_const_lookup", "_ZN3lib18lua_bind_get_valueIiEEbmRT_"),
    ("status_kind", "_ZN3app8lua_bind30StatusModule__status_kind_implEPNS_26BattleObjectModuleAccessorE"),
    ("prev_status_kind", "_ZN3app8lua_bind35StatusModule__prev_status_kind_implEPNS_26BattleObjectModuleAccessorEj"),
    ("situation_kind", "_ZN3app8lua_bind33StatusModule__situation_kind_implEPNS_26BattleObjectModuleAccessorE"),
    ("prev_situation_kind", "_ZN3app8lua_bind38StatusModule__prev_situation_kind_implEPNS_26BattleObjectModuleAccessorE"),
    ("change_status_request", "_ZN3app8lua_bind52StatusModule__change_status_request_from_script_implEPNS_26BattleObjectModuleAccessorEib"),
    ("motion_frame", "_ZN3app8lua_bind24MotionModule__frame_implEPNS_26BattleObjectModuleAccessorE"),
    ("pos_x", "_ZN3app8lua_bind25PostureModule__pos_x_implEPNS_26BattleObjectModuleAccessorE"),
    ("work_is_flag", "_ZN3app8lua_bind24WorkModule__is_flag_implEPNS_26BattleObjectModuleAccessorEi"),
    ("work_set_flag", "_ZN3app8lua_bind25WorkModule__set_flag_implEPNS_26BattleObjectModuleAccessorEbi"),
    ("sdi_hook", "_ZN3app8lua_bind59FighterControlModuleImpl__check_hit_stop_delay_command_implEPNS_26BattleObjectModuleAccessorERN3phx8Vector2fE"),
    ("stick_x_hook", "_ZN3app8lua_bind31ControlModule__get_stick_x_implEPNS_26BattleObjectModuleAccessorE"),
    ("stick_y_hook", "_ZN3app8lua_bind31ControlModule__get_stick_y_implEPNS_26BattleObjectModuleAccessorE"),
    ("clatter_start_hook", "_ZN3app8lua_bind33ControlModule__start_clatter_implEPNS_26BattleObjectModuleAccessorEfffaibb"),
    ("add_clatter_time", "_ZN3app8lua_bind36ControlModule__add_clatter_time_implEPNS_26BattleObjectModuleAccessorEfi"),
    ("effect_req_on_joint", "_ZN3app8lua_bind31EffectModule__req_on_joint_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40ES4_RKNS3_8Vector3fES7_fS7_S7_bjii"),
    ("transition_term_hook", "_ZN3app8lua_bind42WorkModule__is_enable_transition_term_implEPNS_26BattleObjectModuleAccessorEi"),
    ("change_status_force", "_ZN3app8lua_bind38StatusModule__change_status_force_implEPNS_26BattleObjectModuleAccessorEib"),
    ("visibility_set_whole", "_ZN3app8lua_bind32VisibilityModule__set_whole_implEPNS_26BattleObjectModuleAccessorEb"),
    ("effect_set_visible_kind", "_ZN3app8lua_bind35EffectModule__set_visible_kind_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40Eb"),
    ("motion_end_frame", "_ZN3app8lua_bind28MotionModule__end_frame_implEPNS_26BattleObjectModuleAccessorE"),
    ("damage_of", "_ZN3app8lua_bind25DamageModule__damage_implEPNS_26BattleObjectModuleAccessorEi"),
    ("work_get_float", "_ZN3app8lua_bind26WorkModule__get_float_implEPNS_26BattleObjectModuleAccessorEi"),
    ("work_set_float", "_ZN3app8lua_bind26WorkModule__set_float_implEPNS_26BattleObjectModuleAccessorEfi"),
    ("work_get_param_float", "_ZN3app8lua_bind32WorkModule__get_param_float_implEPNS_26BattleObjectModuleAccessorEmm"),
    ("motion_change_hook", "_ZN3app8lua_bind32MotionModule__change_motion_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40Effbfbb"),
    ("quake_pos_hook", "_ZN3app8lua_bind32CameraModule__req_quake_pos_implEPNS_26BattleObjectModuleAccessorEiRKN3phx8Vector3fE"),
    ("command_flag_hook", "_ZN3app8lua_bind40ControlModule__get_command_flag_cat_implEPNS_26BattleObjectModuleAccessorEi"),
    ("attack_air_kind_hook", "_ZN3app8lua_bind39ControlModule__get_attack_air_kind_implEPNS_26BattleObjectModuleAccessorE"),
    ("stick_dir_hook", "_ZN3app8lua_bind33ControlModule__get_stick_dir_implEPNS_26BattleObjectModuleAccessorE"),
    ("work_get_int", "_ZN3app8lua_bind24WorkModule__get_int_implEPNS_26BattleObjectModuleAccessorEi"),
    ("kinetic_sum_speed_y", "_ZN3app8lua_bind35KineticModule__get_sum_speed_y_implEPNS_26BattleObjectModuleAccessorEi"),
    ("pos_y", "_ZN3app8lua_bind25PostureModule__pos_y_implEPNS_26BattleObjectModuleAccessorE"),
    ("pos_z", "_ZN3app8lua_bind25PostureModule__pos_z_implEPNS_26BattleObjectModuleAccessorE"),
    ("effect_req", "_ZN3app8lua_bind22EffectModule__req_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40ERKNS3_8Vector3fES7_fjibi"),
    ("cancel_is_enable", "_ZN3app8lua_bind35CancelModule__is_enable_cancel_implEPNS_26BattleObjectModuleAccessorE"),
    ("motion_kind", "_ZN3app8lua_bind30MotionModule__motion_kind_implEPNS_26BattleObjectModuleAccessorE"),
    ("shield_param_hook", "_ZN3app8lua_bind32WorkModule__get_param_float_implEPNS_26BattleObjectModuleAccessorEmm"),
    ("check_button_on_hook", "_ZN3app8lua_bind35ControlModule__check_button_on_implEPNS_26BattleObjectModuleAccessorEi"),
    ("check_button_off_hook", "_ZN3app8lua_bind36ControlModule__check_button_off_implEPNS_26BattleObjectModuleAccessorEi"),
    ("stick_x_no_clamp_hook", "_ZN3app8lua_bind40ControlModule__get_stick_x_no_clamp_implEPNS_26BattleObjectModuleAccessorE"),
    ("stick_y_no_clamp_hook", "_ZN3app8lua_bind40ControlModule__get_stick_y_no_clamp_implEPNS_26BattleObjectModuleAccessorE"),
];

/// Offsets of every site of `ssbu-cpu-pilot` (commit 0162eed, `src/model.rs`; the list is the one
/// `tests/input_mapping_callers.rs` copied).
const PILOT_SITES: [usize; 23] = [
    0x1750df0, 0x1751ce8, 0x2da180, 0x6b8400, 0x2d8d40, 0x375ef8c, 0x59970, 0x37a2270, 0x376309c, 0x1d59560,
    0x147fb64, 0x150ea38, 0x1d59dc4, 0x1d5b5a8, 0x1d5b650, 0x1d5a590, 0x1d5a840, 0x32f22c4, 0x376db18, 0x32f1630,
    0x30c770, 0x3119d0, 0x1671650,
];

fn cpu_sites() -> impl Iterator<Item = &'static Site> {
    sites::SITES.iter().filter(|s| matches!(s.feature, Feature::CpuCore | Feature::CpuDefence | Feature::CpuLedge | Feature::CpuTech | Feature::CpuShield | Feature::CpuMash | Feature::CpuCommon))
}

#[test]
fn cpu_sites_are_the_symbols_they_name() {
    let Some(owned) = common::owned_main("cpu_sites_are_the_symbols_they_name") else { return };
    for (row, symbol) in SYMBOLS {
        let site = sites::find(row).unwrap_or_else(|| panic!("no row {row}"));
        assert_eq!(owned.symbol(symbol), Some(site.offset), "{row} is not {symbol}");
    }
    // The constant that `sites.rs` names for each is the row's offset.
    for (name, offset) in [
        ("cpu_on_frame_slot", sites_pkg2::CPU_ON_FRAME_SLOT_OFFSET),
        ("cpu_on_frame", sites_pkg2::CPU_ON_FRAME_OFFSET),
        ("lua_const_lookup", sites_pkg2::LUA_CONST_LOOKUP_OFFSET),
        ("status_kind", sites_pkg2::STATUS_KIND_OFFSET),
        ("prev_status_kind", sites_pkg2::PREV_STATUS_KIND_OFFSET),
        ("situation_kind", sites_pkg2::SITUATION_KIND_OFFSET),
        ("prev_situation_kind", sites_pkg2::PREV_SITUATION_KIND_OFFSET),
        ("change_status_request", sites_pkg2::CHANGE_STATUS_REQUEST_OFFSET),
        ("motion_frame", sites_pkg2::MOTION_FRAME_OFFSET),
        ("pos_x", sites_pkg2::POS_X_OFFSET),
        ("work_is_flag", sites_pkg2::WORK_IS_FLAG_OFFSET),
        ("work_set_flag", sites_pkg2::WORK_SET_FLAG_OFFSET),
        ("sdi_hook", sites_pkg2::SDI_HOOK_OFFSET),
        ("stick_x_hook", sites_pkg2::STICK_X_HOOK_OFFSET),
        ("stick_y_hook", sites_pkg2::STICK_Y_HOOK_OFFSET),
        ("clatter_start_hook", sites_pkg2::CLATTER_START_HOOK_OFFSET),
        ("add_clatter_time", sites_pkg2::ADD_CLATTER_TIME_OFFSET),
        ("effect_req_on_joint", sites_pkg2::EFFECT_REQ_ON_JOINT_OFFSET),
        ("transition_term_hook", sites_pkg2::TRANSITION_TERM_HOOK_OFFSET),
        ("change_status_force", sites_pkg2::CHANGE_STATUS_FORCE_OFFSET),
        ("visibility_set_whole", sites_pkg2::VISIBILITY_SET_WHOLE_OFFSET),
        ("effect_set_visible_kind", sites_pkg2::EFFECT_SET_VISIBLE_KIND_OFFSET),
        ("motion_end_frame", sites_pkg2::MOTION_END_FRAME_OFFSET),
        ("damage_of", sites_pkg2::DAMAGE_OF_OFFSET),
        ("work_get_float", sites_pkg2::WORK_GET_FLOAT_OFFSET),
        ("work_set_float", sites_pkg2::WORK_SET_FLOAT_OFFSET),
        ("work_get_param_float", sites_pkg2::WORK_GET_PARAM_FLOAT_OFFSET),
        ("motion_change_hook", sites_pkg2::MOTION_CHANGE_HOOK_OFFSET),
        ("quake_pos_hook", sites_pkg2::QUAKE_POS_HOOK_OFFSET),
        ("command_flag_hook", sites_pkg2::COMMAND_FLAG_HOOK_OFFSET),
        ("attack_air_kind_hook", sites_pkg2::ATTACK_AIR_KIND_HOOK_OFFSET),
        ("stick_dir_hook", sites_pkg2::STICK_DIR_HOOK_OFFSET),
        ("work_get_int", sites_pkg2::WORK_GET_INT_OFFSET),
        ("kinetic_sum_speed_y", sites_pkg2::KINETIC_SUM_SPEED_Y_OFFSET),
        ("pos_y", sites_pkg2::POS_Y_OFFSET),
        ("pos_z", sites_pkg2::POS_Z_OFFSET),
        ("effect_req", sites_pkg2::EFFECT_REQ_OFFSET),
        ("cancel_is_enable", sites_pkg2::CANCEL_IS_ENABLE_OFFSET),
        ("motion_kind", sites_pkg2::MOTION_KIND_OFFSET),
        ("shield_param_hook", sites_pkg2::SHIELD_PARAM_HOOK_OFFSET),
        ("check_button_on_hook", sites_pkg2::CHECK_BUTTON_ON_HOOK_OFFSET),
        ("check_button_off_hook", sites_pkg2::CHECK_BUTTON_OFF_HOOK_OFFSET),
        ("stick_x_no_clamp_hook", sites_pkg2::STICK_X_NO_CLAMP_HOOK_OFFSET),
        ("stick_y_no_clamp_hook", sites_pkg2::STICK_Y_NO_CLAMP_HOOK_OFFSET),
    ] {
        assert_eq!(sites::find(name).map(|s| s.offset), Some(offset), "{name}");
    }
}

#[test]
fn cpu_sites_the_frame_slot_holds_the_original_it_replaces() {
    let Some(owned) = common::owned_main("cpu_sites_the_frame_slot_holds_the_original_it_replaces") else { return };
    let slot = sites::find("cpu_on_frame_slot").unwrap();
    // The slot is a relocated pointer: the file holds zero (and carries no relocation table), so the
    // original's address cannot be read statically. The row records it (`expected_slot_target`, the
    // seam table's value) and boot compares the live slot with it (`Site::slot_outcome`) before the
    // wrapper replaces it.
    assert_eq!(owned.window(slot.offset), [0u8; 64]);
    assert_eq!(slot.expected_slot_target(), Some(sites_pkg2::CPU_ON_FRAME_OFFSET as u64));
    assert_eq!(slot.slot_outcome(sites_pkg2::CPU_ON_FRAME_OFFSET as u64), model::guard::GuardOutcome::Stock);
    assert!(!slot.slot_outcome(0x123456).usable());
    // The function the slot points at is the row the wrapper calls.
    assert_eq!(sites::find("cpu_on_frame").map(|s| s.offset), Some(sites_pkg2::CPU_ON_FRAME_OFFSET));
    assert!(matches!(slot.kind, SeamKind::Hook { form: HookForm::VtableSlot, placement: Placement::Data }));
}

#[test]
fn cpu_sites_every_replace_hook_is_a_thunk_that_fits_the_far_jump() {
    let Some(owned) = common::owned_main("cpu_sites_every_replace_hook_is_a_thunk_that_fits_the_far_jump") else { return };
    let hooks: Vec<_> = cpu_sites()
        .filter(|s| matches!(s.kind, SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry }))
        .collect();
    assert_eq!(hooks.len(), 15, "sdi, stick x/y, start_clatter, transition term, change_motion, req_quake_pos, shield param, button on/off, stick x/y no clamp, command flag, attack air kind, stick dir");
    for site in hooks {
        assert!(!site.note.is_empty(), "{}: an entry hook needs its justification", site.name);
        // A 16-aligned entry: Skyline's far jump is 16 bytes there (20 only at an address 4 past an
        // 8-byte boundary), so it ends at the first byte of the next 16.
        assert_eq!(site.offset % 16, 0, "{}", site.name);
        // No other function starts inside the 16 bytes (the hook would overwrite it).
        for (name, address) in &owned.symbols {
            assert!(
                !(address > &site.offset && address < &(site.offset + 16)),
                "{}: symbol {name} starts at {address:#x} inside the far jump",
                site.name
            );
        }
        // The thunk ends in an indirect or direct branch inside its first 32 bytes (a leaf thunk).
        let (chunks, _) = site.guard[..32].as_chunks::<4>();
        let words: Vec<u32> = chunks.iter().map(|w| u32::from_le_bytes(*w)).collect();
        let branch = |w: u32| w & 0xffff_fc1f == 0xd61f_0000 || w & 0xfc00_0000 == 0x1400_0000;
        assert!(words.iter().any(|w| branch(*w)), "{} is not a leaf thunk", site.name);
        assert!(!words.contains(&0xa9bf_7bfd), "{} has a prologue (stp x29, x30, [sp, #-16]!)", site.name);
    }
}

#[test]
fn cpu_sites_are_not_near_the_cpu_pilot() {
    for site in cpu_sites() {
        if !site.compared() {
            continue;
        }
        let (lo, hi) = (site.offset.saturating_sub(0x40), site.window_end() + 0x40);
        for pilot in PILOT_SITES {
            assert!(!(lo..hi).contains(&pilot), "{} is within 0x40 of the pilot site {pilot:#x}", site.name);
        }
    }
}

#[test]
fn cpu_sites_features_need_the_core() {
    assert!(sites::feature_usable(Feature::CpuLedge, |_| true));
    assert!(!sites::feature_usable(Feature::CpuLedge, |s| s.name != "status_kind"), "ledge needs the core natives");
    assert!(!sites::feature_usable(Feature::CpuDefence, |s| s.name != "lua_const_lookup"));
    assert!(!sites::feature_usable(Feature::CpuDefence, |s| s.name != "add_clatter_time"));
    assert!(!sites::feature_usable(Feature::CpuTech, |s| s.name != "pos_x"), "tech needs the core natives");
    assert!(!sites::feature_usable(Feature::CpuTech, |s| s.name != "visibility_set_whole"));
    assert!(!sites::feature_usable(Feature::CpuShield, |s| s.name != "pos_x"), "shield needs the core natives");
    assert!(!sites::feature_usable(Feature::CpuShield, |s| s.name != "check_button_off_hook"));
    // The core does not need the features' hooks.
    assert!(sites::feature_usable(Feature::CpuCore, |s| s.feature == Feature::CpuCore || s.feature == Feature::Training));
    assert!(!sites::feature_usable(Feature::CpuCore, |s| s.name != "cpu_on_frame_slot"));
}

#[test]
fn cpu_sites_overlapping_windows_are_all_our_own() {
    // Boot compares every window before it writes any (sites::BOOT_PHASES); windows of CPU rows may
    // overlap each other (the thunks are 16 bytes apart) but only with each other or with Called rows.
    let mut pairs = 0;
    for (a, b) in sites::overlapping_pairs() {
        let (a, b) = (&sites::SITES[a], &sites::SITES[b]);
        let cpu = |s: &Site| matches!(s.feature, Feature::CpuCore | Feature::CpuDefence | Feature::CpuLedge | Feature::CpuTech | Feature::CpuShield | Feature::CpuMash | Feature::CpuCommon);
        if cpu(a) || cpu(b) {
            pairs += 1;
            assert!(cpu(a) && cpu(b), "{} overlaps {} (a site of another feature)", a.name, b.name);
        }
    }
    assert!(pairs > 0, "the stick, clatter and term thunks do overlap each other");
}
