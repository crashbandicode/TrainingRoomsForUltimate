//! The sites the hitbox visualizer (SPEC-core-06) hooks, calls or relies on, in a table of their own (stream rules: one
//! file per stream, registered by one line). Same row type and the same 64-byte guard rule as `sites.rs`; the bytes were
//! read from the owned 13.0.5 ELF by `tools/gen_hitbox_sites.py` and `tests/core06_hitbox.rs` re-checks every row against
//! it. Offsets are from the start of `.text`.
//!
//! Seven hooks (the `ATTACK` and `CATCH` animation commands, `GrabModule::set_rebound` and, rc.14 wind-articles, the four
//! `AREA_WIND_2ND*` commands), eleven called natives of the
//! attack, damage, effect and motion modules, the Lua state's accessor getter, two relied-on windows (the Lua stack reads the plugin
//! repeats from the game's own command code), three relied-on windows of HitModule (the hurtbox tables, decision HB1), two of the
//! DamageModule's damage record (spots-3) and,
//! for the own orbs (decision HB-OWN-ORBS), the joint-position thunk, the camera projection and its screen-size window. The
//! effect natives remain for articles' spheres.
//! Every row is a capability (`model::capabilities`); the natives the
//! feature shares with other packages (`status_kind`, the constant lookup, `set_visible_kind`) stay in those packages' tables
//! and enter the closure through `CROSS_NEEDS`.

use super::guard::{HookForm, Placement, SeamKind};
use super::sites::{Feature, Site};

pub const HITBOX_ATTACK_OFFSET: usize = 0x228_4b20;
pub const HITBOX_CATCH_OFFSET: usize = 0x22a_c290;
pub const HITBOX_REBOUND_OFFSET: usize = 0x202_7bb0;
pub const HITBOX_IS_ATTACK_OFFSET: usize = 0x1fd_01c0;
pub const HITBOX_ATTACK_DATA_OFFSET: usize = 0x1fd_0520;
/// `AttackModule::power_mul_status(accessor)`: the module's status-scoped damage multiplier (the smash charge; module `+0xe0`).
pub const HITBOX_POWER_MUL_OFFSET: usize = 0x1fd_0550;
/// `DamageModule::reaction(accessor, index)`: the knockback the defender received (spots-3).
pub const HITBOX_DAMAGE_REACTION_OFFSET: usize = 0x200_b710;
/// spots-3, relied on: the fighter DamageModule's reaction getter (vtable slot 0xd0: the damage records from module `+0x18`, 0x1c0 bytes
/// each, the knockback at `+0x10`) and the hit handler's copy of the hit's id block beside the knockback it stores (`damage_record`).
pub const DAMAGE_RECORD_OFFSET: usize = 0x3f_fa10;
pub const DAMAGE_HIT_IDS_OFFSET: usize = 0x40_2f9c;
/// spots-3, relied on: the hit's damage transaction adding the hit's damage into record `+0x8` (`DAMAGE_DEALT_OFFSET`).
pub const DAMAGE_DEALT_OFFSET: usize = 0x40_28e0;

/// The defender's damage record 0, read raw (`DAMAGE_RECORD_OFFSET`, `DAMAGE_HIT_IDS_OFFSET`, `DAMAGE_DEALT_OFFSET`): the accessor's
/// DamageModule, its record array, and in the record the attacker's battle object id (the hit's `+0x24`), the attack slot (the hit's `+0x32`,
/// the id the handler passes to `AttackModule::attack_data`) and the frame's damage before the 1-on-1 ratio (`+0x8`; Eden: the percent's rise
/// over it is 1.2000 on 93 hits).
pub mod damage_record {
    pub const MODULE: usize = 0xa8;
    pub const RECORDS: usize = 0x18;
    pub const DEALT: usize = 0x8;
    pub const ATTACKER: usize = 0x44;
    pub const SLOT: usize = 0x52;
}
pub const HITBOX_REQ_FOLLOW_OFFSET: usize = 0x201_7730;
pub const HITBOX_KILL_OFFSET: usize = 0x201_7840;
pub const HITBOX_IS_EXIST_OFFSET: usize = 0x201_7a20;
pub const HITBOX_RGB_LAST_OFFSET: usize = 0x201_7c80;
pub const HITBOX_RATE_LAST_OFFSET: usize = 0x201_7ce0;
pub const HITBOX_SLEEP_EFFECT_OFFSET: usize = 0x206_9140;
pub const HITBOX_ACCESSOR_OFFSET: usize = 0x227_daf0;
/// `ArticleModule::get_article_from_no(accessor, kind, ordinal)`: nullable `Article*`, not an accessor.
pub const HITBOX_ARTICLE_FROM_NO_OFFSET: usize = 0x209_2e10;
pub const HITBOX_TEAM_OWNER_ID_OFFSET: usize = 0x208_a990;
#[cfg(test)]
pub const HITBOX_ATTACK_STACK_OFFSET: usize = 0x228_4cd0;
#[cfg(test)]
pub const HITBOX_CATCH_STACK_OFFSET: usize = 0x22a_c2ec;
/// Hurtboxes (decision HB1): the `HitModule` thunk, its group/part tables, its part update.
#[cfg(test)]
pub const HITBOX_HURT_MODULE_OFFSET: usize = 0x203_db10;
#[cfg(test)]
pub const HITBOX_HURT_GROUPS_OFFSET: usize = 0x46_9af0;
#[cfg(test)]
pub const HITBOX_HURT_PART_OFFSET: usize = 0x46_8490;
/// Own orbs (decision HB-OWN-ORBS; `model::hitbox_orbs`): the bone transform, the projection and its screen size words.
pub const ORB_JOINT_POSITION_OFFSET: usize = 0x205_7640;
pub const ORB_WORLD_TO_SCREEN_OFFSET: usize = 0x134_1280;
pub const ORB_SCREEN_SIZE_OFFSET: usize = 0x134_15f0;
/// The screen size words that projection reads (`.bss`, offsets from `.text`).
pub const ORB_SCREEN_W_WORD: usize = 0x744_701c;
pub const ORB_SCREEN_H_WORD: usize = 0x744_7020;
/// Eggs-ercise's hitlag (SPEC V2.1): `StopModule::set_hit_stop_frame`, a lua_bind thunk.
pub const HITBOX_HIT_STOP_OFFSET: usize = 0x208_9260;
/// Eggs-ercise's stage range (V2.2): the accessor `app::camera::get_camera_limit_range` returns the camera manager's
/// (`sites_pkg2::CAMERA_MANAGER_SLOT_OFFSET`) four floats at `CAMERA_LIMIT_FIELD`; the plugin reads them, never calls it.
pub const CAMERA_LIMIT_RANGE_OFFSET: usize = 0x15c_b9b0;
pub const CAMERA_LIMIT_FIELD: usize = 0xc20;
/// rc.14 wind-articles: the four wind-area animation commands (`app::sv_animcmd::AREA_WIND_2ND`, `_arg10`, `_RAD`, `_RAD_arg9`). Each
/// takes the running object's AreaModule (accessor `+0xc0`) and adds a wind record; an article that runs one is tracked so its area
/// is drawn (`legend::wind_tracks`).
pub const AREA_WIND_OFFSET: usize = 0x22a_98c0;
pub const AREA_WIND_ARG10_OFFSET: usize = 0x22a_9960;
pub const AREA_WIND_RAD_OFFSET: usize = 0x22a_8e90;
pub const AREA_WIND_RAD_ARG9_OFFSET: usize = 0x22a_8f30;
/// The seven hooks' rows (the other rows are called or relied on, never written).
pub const HOOK_ROWS: [&str; 7] =
    ["hitbox_attack", "hitbox_catch", "hitbox_set_rebound", "hitbox_area_wind", "hitbox_area_wind_arg10", "hitbox_area_wind_rad", "hitbox_area_wind_rad_arg9"];

pub static SITES: [Site; 37] = include!(concat!(env!("OUT_DIR"), "/tables/sites_hitbox.SITES.rs"));
