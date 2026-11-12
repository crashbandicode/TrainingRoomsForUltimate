//! The sites the hitbox visualizer (SPEC-core-06) hooks, calls or relies on, in a table of their own (stream rules: one
//! file per stream, registered by one line). Same row type and the same 64-byte guard rule as `sites.rs`; the bytes were
//! read from the owned 13.0.5 ELF by `tools/gen_hitbox_sites.py` and `tests/core06_hitbox.rs` re-checks every row against
//! it. Offsets are from the start of `.text`.
//!
//! Three hooks (the `ATTACK` and `CATCH` animation commands and `GrabModule::set_rebound`), nine called natives of the
//! attack, effect and motion modules, the Lua state's accessor getter, two relied-on windows (the Lua stack reads the plugin
//! repeats from the game's own command code), three relied-on windows of HitModule (the hurtbox tables, decision HB1) and,
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
pub const HITBOX_REQ_FOLLOW_OFFSET: usize = 0x201_7730;
pub const HITBOX_KILL_OFFSET: usize = 0x201_7840;
pub const HITBOX_IS_EXIST_OFFSET: usize = 0x201_7a20;
pub const HITBOX_RGB_LAST_OFFSET: usize = 0x201_7c80;
pub const HITBOX_RATE_LAST_OFFSET: usize = 0x201_7ce0;
pub const HITBOX_SLEEP_EFFECT_OFFSET: usize = 0x206_9140;
pub const HITBOX_ACCESSOR_OFFSET: usize = 0x227_daf0;
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
/// The three hooks' rows (the other rows are called or relied on, never written).
pub const HOOK_ROWS: [&str; 3] = ["hitbox_attack", "hitbox_catch", "hitbox_set_rebound"];

pub static SITES: [Site; 24] = include!(concat!(env!("OUT_DIR"), "/tables/sites_hitbox.SITES.rs"));
