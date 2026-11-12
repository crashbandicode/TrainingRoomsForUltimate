//! Package 2 (CPU behaviour) site rows: every place in the 13.0.5 `main` the CPU behaviours hook, call or
//! rely on (SPEC-cpu-01 section 4 and the specs after it), each with its 64 guard bytes read from the owned ELF
//! (`tests/cpu_sites.rs` checks the symbol of every row and `core01_sites_match_owned_elf` the bytes).
//! Registered in `sites::SITES` by one line; the features are `Feature::Cpu*`.

use super::guard::{outcome_for, GuardOutcome, HookForm, Placement, SeamKind, Sharing, GUARD_BYTES};
use super::sites::{Feature, Site};

/// Package 2: the per-frame slot and its original, the constant table lookup, the natives the facts
/// are read through, and the hooks of SPEC-cpu-02 and SPEC-cpu-03 (each is a row of `SITES`).
pub const CPU_ON_FRAME_SLOT_OFFSET: usize = 0x4f8_0568;
pub const CPU_ON_FRAME_OFFSET: usize = 0x61_4630;
pub const LUA_CONST_LOOKUP_OFFSET: usize = 0x372_8e10;
pub const STATUS_KIND_OFFSET: usize = 0x208_7720;
pub const PREV_STATUS_KIND_OFFSET: usize = 0x208_7770;
pub const SITUATION_KIND_OFFSET: usize = 0x208_77a0;
pub const PREV_SITUATION_KIND_OFFSET: usize = 0x208_77b0;
pub const CHANGE_STATUS_REQUEST_OFFSET: usize = 0x208_76e0;
pub const MOTION_FRAME_OFFSET: usize = 0x205_cc70;
pub const POS_X_OFFSET: usize = 0x206_ae10;
pub const WORK_IS_FLAG_OFFSET: usize = 0x208_dc80;
pub const WORK_SET_FLAG_OFFSET: usize = 0x208_dcb0;
pub const SDI_HOOK_OFFSET: usize = 0x20a_eda0;
pub const STICK_X_HOOK_OFFSET: usize = 0x200_1260;
pub const STICK_Y_HOOK_OFFSET: usize = 0x200_1280;
pub const CLATTER_START_HOOK_OFFSET: usize = 0x200_1080;
pub const ADD_CLATTER_TIME_OFFSET: usize = 0x200_10d0;
pub const EFFECT_REQ_ON_JOINT_OFFSET: usize = 0x201_76f0;
pub const TRANSITION_TERM_HOOK_OFFSET: usize = 0x208_dd60;
pub const CHANGE_STATUS_FORCE_OFFSET: usize = 0x208_7780;
pub const VISIBILITY_SET_WHOLE_OFFSET: usize = 0x208_c0c0;
pub const EFFECT_SET_VISIBLE_KIND_OFFSET: usize = 0x201_7a50;
pub const MOTION_END_FRAME_OFFSET: usize = 0x205_ccd0;
pub const DAMAGE_OF_OFFSET: usize = 0x200_b700;
pub const WORK_GET_FLOAT_OFFSET: usize = 0x208_db50;
pub const WORK_SET_FLOAT_OFFSET: usize = 0x208_db60;
pub const WORK_GET_PARAM_FLOAT_OFFSET: usize = 0x208_de50;
pub const MOTION_CHANGE_HOOK_OFFSET: usize = 0x205_cb30;
pub const MOTION_KIND_OFFSET: usize = 0x205_cc20;
pub const COMMAND_FLAG_HOOK_OFFSET: usize = 0x200_1630;
pub const ATTACK_AIR_KIND_HOOK_OFFSET: usize = 0x200_1710;
pub const STICK_DIR_HOOK_OFFSET: usize = 0x200_1030;
pub const WORK_GET_INT_OFFSET: usize = 0x208_dbc0;
pub const KINETIC_SUM_SPEED_Y_OFFSET: usize = 0x204_3a10;
pub const POS_Y_OFFSET: usize = 0x206_ae40;
pub const POS_Z_OFFSET: usize = 0x206_ae70;
pub const EFFECT_REQ_OFFSET: usize = 0x201_76b0;
pub const CANCEL_IS_ENABLE_OFFSET: usize = 0x209_2860;
pub const QUAKE_POS_HOOK_OFFSET: usize = 0x1ff_0ac0;
pub const SHIELD_PARAM_HOOK_OFFSET: usize = 0x208_de50;
pub const CHECK_BUTTON_ON_HOOK_OFFSET: usize = 0x200_1430;
pub const CHECK_BUTTON_OFF_HOOK_OFFSET: usize = 0x200_1440;
pub const STICK_X_NO_CLAMP_HOOK_OFFSET: usize = 0x200_1600;
pub const STICK_Y_NO_CLAMP_HOOK_OFFSET: usize = 0x200_1610;
pub const UTILITY_GET_CATEGORY_OFFSET: usize = 0x15c_dfe0;
pub const UTILITY_GET_KIND_OFFSET: usize = 0x15c_dff0;

/// The game's `CommonParams` filler (`from_prc(this, params)`, a 70 KB function that stores each field the parameter
/// file names into `this`): the site is its `mov x19, x0`, where x0 is the object. 13.0.5 stores `shield_damage_mul` at
/// `this + 0x174` (`shield_dec1` at 0x16c, `shield_recovery1` at 0x170, `shield_damage_add` at 0x178). The object is
/// allocated by the factory at main+0x736920 (0xc58 bytes); Eden confirms it is the live one (diag line
/// `common_params`).
pub const COMMON_PARAMS_FILL_OFFSET: usize = 0x73_6ac0;
pub const SHIELD_DAMAGE_MUL_FIELD: usize = 0x174;

/// The native stage table (SPEC-cpu-04 camera): 364 records of 72 bytes indexed by stage id.
pub const STAGE_TABLE_OFFSET: usize = 0x454_a7e8;
pub const STAGE_RECORD_STRIDE: usize = 72;
/// The form field of a record: 0 the stage, 1 its Omega form, 2 its Battlefield form.
pub const STAGE_RECORD_FORM: usize = 8;
pub const STAGE_ID_CALL_OFFSET: usize = 0x227_ebb0;
pub const CAMERA_MODE_HOOK_OFFSET: usize = 0x4e_e498;
pub const FIXED_CAMERA_SETUP_HOOK_OFFSET: usize = 0x315_8e00;
/// The camera manager: the setup function loads `**(main + 0x52b8f00)` (the spec's `0x52b6f00` is a different slot;
/// the 64 bytes of the setup row contain the `adrp`/`ldr` that read this one) and stores the centre at `+0xbd0`
/// (`str q0, [manager, #0xbd0]`). The mode hook gets the manager as x0.
pub const CAMERA_MANAGER_SLOT_OFFSET: usize = 0x52b_8f00;
pub const CAMERA_CENTRE_FIELD: usize = 0xbd0;

pub static SITES_PKG2: [Site; 51] = include!(concat!(env!("OUT_DIR"), "/tables/sites_pkg2.SITES_PKG2.rs"));

/// A site in the `common` script module (`lua2cpp_common.nro`), found by symbol when that module loads and
/// guarded against the 64 bytes of the owned 13.0.5 module (`tests/cpu_common.rs` reads them from
/// `research/game-13.0.5/nro/lua2cpp_common.nro`, extracted from `data.arc`; see
/// `IMPLEMENTATION-PROGRESS-pkg2.md (removed after v0.2.0-rc.1; see that tag)`).
#[derive(Clone, Copy, Debug)]
pub struct CommonSite {
    pub name: &'static str,
    /// The mangled symbol the loader's `LookupSymbol` answers.
    pub symbol: &'static str,
    /// A replace hook at the entry (otherwise a called function).
    pub hooked: bool,
    pub guard: [u8; GUARD_BYTES],
    pub note: &'static str,
}

impl CommonSite {
    pub const fn kind(&self) -> SeamKind {
        if self.hooked {
            SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry }
        } else {
            SeamKind::Called
        }
    }

    /// The outcome for the 64 bytes now at the function in the loaded module.
    pub fn outcome(&self, actual: &[u8; GUARD_BYTES]) -> GuardOutcome {
        outcome_for(&self.guard, actual, self.kind(), Sharing::None)
    }
}

pub static COMMON_SITES: [CommonSite; 4] = include!(concat!(env!("OUT_DIR"), "/tables/sites_pkg2.COMMON_SITES.rs"));
