//! The game natives the save states call (package 3). Each is a guarded `Called` row of
//! `model::sites_pkg3` (a `lua_bind` thunk: the module accessor first, then its arguments).
//!
//! # Safety
//! Every function is only valid after the `StateCore` sites passed their guards, with `accessor` the
//! module accessor of a live fighter (or the manager / article / item pointer the name says).
//!
//! NOT YET SWITCH-RUN.

use crate::model::sites_pkg3 as s;
use crate::runtime::game::{granted, text_base};

/// The native at `main+offset`, or `None` when its rows did not pass their guards at boot (`game::granted`):
/// a native that was not granted is never called, and the function that asked answers nothing (R2-1).
unsafe fn at<F: Copy>(offset: usize) -> Option<F> {
    debug_assert_eq!(std::mem::size_of::<F>(), std::mem::size_of::<usize>());
    if !granted(offset) {
        return None;
    }
    Some(std::mem::transmute_copy(&(text_base() + offset)))
}

/// `phx::Vector3f`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub unsafe fn pos_y(accessor: usize) -> f32 {
    at::<extern "C" fn(usize) -> f32>(s::STATE_POS_Y_OFFSET).map_or_else(Default::default, |f| f(accessor))
}

pub unsafe fn lr(accessor: usize) -> f32 {
    at::<extern "C" fn(usize) -> f32>(s::STATE_LR_OFFSET).map_or_else(Default::default, |f| f(accessor))
}

pub unsafe fn set_lr(accessor: usize, lr: f32) {
    at::<extern "C" fn(usize, f32)>(s::STATE_SET_LR_OFFSET).map_or_else(Default::default, |f| f(accessor, lr))
}

pub unsafe fn set_pos(accessor: usize, pos: &Vector3f) {
    at::<extern "C" fn(usize, *const Vector3f)>(s::STATE_SET_POS_OFFSET).map_or_else(Default::default, |f| f(accessor, pos))
}

/// `PostureModule::init_pos(pos, true, false)`: resets ground contacts and collision history at the
/// destination; the second flag stays false so linked objects are not translated.
pub unsafe fn init_pos(accessor: usize, pos: &Vector3f) {
    at::<extern "C" fn(usize, *const Vector3f, bool, bool)>(s::STATE_INIT_POS_OFFSET).map_or_else(Default::default, |f| f(accessor, pos, true, false))
}

pub unsafe fn clear_speed_all(accessor: usize) {
    at::<extern "C" fn(usize)>(s::STATE_CLEAR_SPEED_ALL_OFFSET).map_or_else(Default::default, |f| f(accessor))
}

pub unsafe fn heal(accessor: usize, amount: f32) {
    at::<extern "C" fn(usize, f32, i32)>(s::STATE_HEAL_OFFSET).map_or_else(Default::default, |f| f(accessor, amount, 0))
}

pub unsafe fn add_damage(accessor: usize, amount: f32) {
    at::<extern "C" fn(usize, f32, i32)>(s::STATE_ADD_DAMAGE_OFFSET).map_or_else(Default::default, |f| f(accessor, amount, 0))
}

pub unsafe fn set_sleep(accessor: usize, asleep: bool) {
    at::<extern "C" fn(usize, bool)>(s::STATE_SET_SLEEP_OFFSET).map_or_else(Default::default, |f| f(accessor, asleep))
}

pub unsafe fn stop_all_sound(accessor: usize) {
    at::<extern "C" fn(usize)>(s::STATE_STOP_ALL_SOUND_OFFSET).map_or_else(Default::default, |f| f(accessor))
}

pub unsafe fn pause_se_all(accessor: usize, paused: bool) {
    at::<extern "C" fn(usize, bool)>(s::STATE_PAUSE_SE_ALL_OFFSET).map_or_else(Default::default, |f| f(accessor, paused))
}

/// `SoundModule::play_se_no3d(hash, true, true)`.
pub unsafe fn play_se_no3d(accessor: usize, hash: u64) {
    at::<extern "C" fn(usize, u64, bool, bool) -> u64>(s::STATE_PLAY_SE_NO3D_OFFSET).map_or_else(Default::default, |f| f(accessor, hash, true, true));
}

pub unsafe fn stop_rumble(accessor: usize, all: bool) {
    at::<extern "C" fn(usize, bool)>(s::STATE_STOP_RUMBLE_OFFSET).map_or_else(Default::default, |f| f(accessor, all))
}

pub unsafe fn slow_clear_whole(accessor: usize) {
    at::<extern "C" fn(usize)>(s::STATE_SLOW_CLEAR_WHOLE_OFFSET).map_or_else(Default::default, |f| f(accessor))
}

pub unsafe fn camera_zoom_out(accessor: usize, kind: i32) {
    at::<extern "C" fn(usize, i32)>(s::STATE_CAMERA_ZOOM_OUT_OFFSET).map_or_else(Default::default, |f| f(accessor, kind))
}

pub unsafe fn camera_stop_quake(accessor: usize, kind: i32) {
    at::<extern "C" fn(usize, i32)>(s::STATE_CAMERA_STOP_QUAKE_OFFSET).map_or_else(Default::default, |f| f(accessor, kind))
}

pub unsafe fn effect_kill_kind(accessor: usize, effect: u64) {
    at::<extern "C" fn(usize, u64, bool, bool)>(s::STATE_EFFECT_KILL_KIND_OFFSET).map_or_else(Default::default, |f| f(accessor, effect, false, false))
}

pub unsafe fn effect_remove_screen(accessor: usize, effect: u64) {
    at::<extern "C" fn(usize, u64, i32)>(s::STATE_EFFECT_REMOVE_SCREEN_OFFSET).map_or_else(Default::default, |f| f(accessor, effect, 0))
}

pub unsafe fn effect_remove_common(accessor: usize, effect: u64) {
    at::<extern "C" fn(usize, u64)>(s::STATE_EFFECT_REMOVE_COMMON_OFFSET).map_or_else(Default::default, |f| f(accessor, effect))
}

pub unsafe fn article_is_exist(accessor: usize, article: i32) -> bool {
    at::<extern "C" fn(usize, i32) -> bool>(s::STATE_ARTICLE_IS_EXIST_OFFSET).map_or_else(Default::default, |f| f(accessor, article))
}

/// The `Article*` of article `article`, or 0.
pub unsafe fn article_get(accessor: usize, article: i32) -> usize {
    at::<extern "C" fn(usize, i32) -> usize>(s::STATE_ARTICLE_GET_ARTICLE_OFFSET).map_or_else(Default::default, |f| f(accessor, article))
}

pub unsafe fn article_object_id(article: usize) -> u32 {
    at::<extern "C" fn(usize) -> u32>(s::STATE_ARTICLE_OBJECT_ID_OFFSET).map_or_else(Default::default, |f| f(article))
}

pub unsafe fn article_remove_object(accessor: usize, object_id: u32) {
    at::<extern "C" fn(usize, u32)>(s::STATE_ARTICLE_REMOVE_OBJECT_OFFSET).map_or_else(Default::default, |f| f(accessor, object_id))
}

pub unsafe fn item_active_count(manager: usize) -> u64 {
    at::<extern "C" fn(usize) -> u64>(s::STATE_ITEM_ACTIVE_COUNT_OFFSET).map_or_else(Default::default, |f| f(manager))
}

/// The `Item*` at `index` of the manager's active list, or 0.
pub unsafe fn item_active(manager: usize, index: u64) -> usize {
    at::<extern "C" fn(usize, u64) -> usize>(s::STATE_ITEM_ACTIVE_ITEM_OFFSET).map_or_else(Default::default, |f| f(manager, index))
}

pub unsafe fn item_object_id(item: usize) -> u32 {
    at::<extern "C" fn(usize) -> u32>(s::STATE_ITEM_OBJECT_ID_OFFSET).map_or_else(Default::default, |f| f(item))
}

pub unsafe fn item_remove(manager: usize, object_id: u32) {
    at::<extern "C" fn(usize, u32)>(s::STATE_ITEM_REMOVE_OFFSET).map_or_else(Default::default, |f| f(manager, object_id))
}

pub unsafe fn work_get_int(accessor: usize, id: i32) -> i32 {
    at::<extern "C" fn(usize, i32) -> i32>(s::STATE_WORK_GET_INT_OFFSET).map_or_else(Default::default, |f| f(accessor, id))
}

pub unsafe fn work_set_int(accessor: usize, value: i32, id: i32) {
    at::<extern "C" fn(usize, i32, i32)>(s::STATE_WORK_SET_INT_OFFSET).map_or_else(Default::default, |f| f(accessor, value, id))
}

pub unsafe fn work_on_flag(accessor: usize, id: i32) {
    at::<extern "C" fn(usize, i32)>(s::STATE_WORK_ON_FLAG_OFFSET).map_or_else(Default::default, |f| f(accessor, id))
}

pub unsafe fn work_off_flag(accessor: usize, id: i32) {
    at::<extern "C" fn(usize, i32)>(s::STATE_WORK_OFF_FLAG_OFFSET).map_or_else(Default::default, |f| f(accessor, id))
}

/// `StatusModule::change_status_request(accessor, status, false)`: the placement handshake.
pub unsafe fn status_request(accessor: usize, status: i32) {
    at::<extern "C" fn(usize, i32, bool)>(s::STATE_STATUS_REQUEST_OFFSET).map_or_else(Default::default, |f| f(accessor, status, false))
}

pub unsafe fn stage_id() -> i32 {
    at::<extern "C" fn() -> i32>(s::STATE_STAGE_ID_OFFSET).map_or_else(Default::default, |f| f())
}

/// The ItemManager singleton, read through the slot `manager_slot_offset` recomputed from the guarded
/// instruction pair; 0 when it does not exist yet.
pub unsafe fn item_manager() -> usize {
    ((text_base() + crate::model::state::ITEM_MANAGER_SLOT_OFFSET) as *const usize).read_volatile()
}

// ---- milestone 2: charges and buffs ----------------------------------------------------------------

/// `EffectModule::req_common(accessor, effect, 0.0)`: the generic `charge_max` flash.
pub unsafe fn effect_req_common(accessor: usize, effect: u64) {
    at::<extern "C" fn(usize, u64, f32) -> u64>(s::STATE_EFFECT_REQ_COMMON_OFFSET).map_or_else(Default::default, |f| f(accessor, effect, 0.0));
}

/// `EffectModule::req_follow(accessor, effect, joint, pos, rot, 1.0, false, attribute, 0, lifetime, 0, 0,
/// false, false)`: the handle of the effect (through the save states' own hook: a pass-through for
/// everything but `sys_score_aura`).
pub unsafe fn effect_req_follow(accessor: usize, effect: u64, joint: u64, pos: &Vector3f, rot: &Vector3f, attribute: u32, lifetime: i32) -> u64 {
    type ReqFollow = extern "C" fn(usize, u64, u64, *const Vector3f, *const Vector3f, f32, bool, u32, i32, i32, i32, i32, bool, bool) -> u64;
    at::<ReqFollow>(s::STATE_EFFECT_REQ_FOLLOW_OFFSET).map_or_else(Default::default, |f| f(accessor, effect, joint, pos, rot, 1.0, false, attribute, 0, lifetime, 0, 0, false, false))
}

/// `EffectModule::req(accessor, effect, pos, rot, 1.0, 0, -1, false, 0)`: Min Min's arm effect.
pub unsafe fn effect_req(accessor: usize, effect: u64) -> u64 {
    type Req = extern "C" fn(usize, u64, *const Vector3f, *const Vector3f, f32, u32, i32, bool, i32) -> u64;
    let zero = Vector3f { x: 0.0, y: 0.0, z: 0.0 };
    at::<Req>(s::STATE_EFFECT_REQ_OFFSET).map_or_else(Default::default, |f| f(accessor, effect, &zero, &zero, 1.0, 0, -1, false, 0))
}

pub unsafe fn effect_kill(accessor: usize, handle: u32) {
    at::<extern "C" fn(usize, u32, bool, bool)>(s::STATE_EFFECT_KILL_OFFSET).map_or_else(Default::default, |f| f(accessor, handle, false, true))
}

pub unsafe fn effect_set_scale_one(accessor: usize, handle: u32) {
    let one = Vector3f { x: 1.0, y: 1.0, z: 1.0 };
    at::<extern "C" fn(usize, u32, *const Vector3f)>(s::STATE_EFFECT_SET_SCALE_OFFSET).map_or_else(Default::default, |f| f(accessor, handle, &one))
}

/// `FighterUtil::set_face_motion_by_priority(accessor, facial, motion)`.
pub unsafe fn face_motion(accessor: usize, facial: i32, motion: u64) {
    at::<extern "C" fn(usize, i32, u64)>(s::STATE_FACE_MOTION_OFFSET).map_or_else(Default::default, |f| f(accessor, facial, motion))
}

pub unsafe fn article_generate_enable(accessor: usize, article: i32) {
    at::<extern "C" fn(usize, i32, bool, i32)>(s::STATE_ARTICLE_GENERATE_ENABLE_OFFSET).map_or_else(Default::default, |f| f(accessor, article, false, -1))
}

pub unsafe fn article_set_visibility(accessor: usize, article: i32, name: u64, group: u64) {
    at::<extern "C" fn(usize, i32, u64, u64, i32)>(s::STATE_ARTICLE_SET_VISIBILITY_OFFSET).map_or_else(Default::default, |f| f(accessor, article, name, group, 0))
}

pub unsafe fn motion_set_rate(accessor: usize, rate: f32) {
    at::<extern "C" fn(usize, f32)>(s::STATE_MOTION_SET_RATE_OFFSET).map_or_else(Default::default, |f| f(accessor, rate))
}

/// `FighterSpecializer_Jack::add_rebel_gauge(accessor, FighterEntryID(entry), amount)`.
pub unsafe fn jack_add_rebel(accessor: usize, entry: i32, amount: f32) {
    at::<extern "C" fn(usize, i32, f32)>(s::STATE_JACK_ADD_REBEL_OFFSET).map_or_else(Default::default, |f| f(accessor, entry, amount))
}

/// The game's add-limit routine `(limit, fighter object, 0)`.
pub unsafe fn cloud_add_limit(fighter: usize, limit: f32) {
    at::<extern "C" fn(f32, usize, i32)>(s::STATE_CLOUD_ADD_LIMIT_OFFSET).map_or_else(Default::default, |f| f(limit, fighter, 0))
}

pub unsafe fn visibility_set_status_default(accessor: usize, a: u64, b: u64) {
    at::<extern "C" fn(usize, u64, u64)>(s::STATE_VISIBILITY_SET_STATUS_DEFAULT_OFFSET).map_or_else(Default::default, |f| f(accessor, a, b))
}

pub unsafe fn visibility_set(accessor: usize, a: u64, b: u64) {
    at::<extern "C" fn(usize, u64, u64)>(s::STATE_VISIBILITY_SET_OFFSET).map_or_else(Default::default, |f| f(accessor, a, b))
}

/// `MotionModule::add_motion_partial(DRAGON, motion, 0.0, 1.0, false, false, 0.0, true, true, false)`.
pub unsafe fn motion_add_partial_dragon(accessor: usize, part: i32, motion: u64) {
    type Partial = extern "C" fn(usize, i32, u64, f32, f32, bool, bool, f32, bool, bool, bool);
    at::<Partial>(s::STATE_MOTION_ADD_PARTIAL_OFFSET).map_or_else(Default::default, |f| f(accessor, part, motion, 0.0, 1.0, false, false, 0.0, true, true, false))
}

/// `StatusModule::change_status_force(accessor, status, block)` (the CPU core's native always passes
/// `true`; the buffs need `false` for Wii Fit).
pub unsafe fn status_force(accessor: usize, status: i32, block: bool) {
    at::<extern "C" fn(usize, i32, bool)>(crate::model::sites_pkg2::CHANGE_STATUS_FORCE_OFFSET).map_or_else(Default::default, |f| f(accessor, status, block))
}

/// `WorkModule::get_param_int(accessor, kind, name)` through the save states' hook (a pass-through for
/// anything but a death parameter).
pub unsafe fn get_param_int(accessor: usize, kind: u64, name: u64) -> i32 {
    at::<extern "C" fn(usize, u64, u64) -> i32>(s::STATE_GET_PARAM_INT_OFFSET).map_or_else(Default::default, |f| f(accessor, kind, name))
}

// ---- milestone 3: the fighter-specific restores ------------------------------------------------------
// The held item is package 6's item engine (`roster::character_items`, reached through `features::item_bridge`): its `ItemModule`
// natives (`have_item`, `is_have_item`, `get_have_item_id`, `remove_item`, `set_have_item_action`, `utility::get_category`, the turnip
// power getters) are called there, behind the rows `state_*` / `utility_get_category` that `roster::CALLED_ROWS` lists; the
// duplicates this module held for a save-state item restore of its own were removed (docs/provenance/pkg6-carry-ledger.md, "Deleted
// clean-room natives").

pub unsafe fn article_generate(accessor: usize, article: i32, arg: i32) {
    at::<extern "C" fn(usize, i32, bool, i32)>(s::STATE_ARTICLE_GENERATE_OFFSET).map_or_else(Default::default, |f| f(accessor, article, false, arg))
}

pub unsafe fn article_remove_exist(accessor: usize, article: i32, target: i32) {
    at::<extern "C" fn(usize, i32, i32)>(s::STATE_ARTICLE_REMOVE_EXIST_OFFSET).map_or_else(Default::default, |f| f(accessor, article, target))
}

pub unsafe fn article_active_num(accessor: usize, article: i32) -> i32 {
    at::<extern "C" fn(usize, i32) -> i32>(s::STATE_ARTICLE_ACTIVE_NUM_OFFSET).map_or_else(Default::default, |f| f(accessor, article))
}

pub unsafe fn work_get_int64(accessor: usize, id: i32) -> i64 {
    at::<extern "C" fn(usize, i32) -> i64>(s::STATE_WORK_GET_INT64_OFFSET).map_or_else(Default::default, |f| f(accessor, id))
}

pub unsafe fn sub_material_num(accessor: usize, material: i32) {
    at::<extern "C" fn(usize, i32, i32)>(s::STATE_SUB_MATERIAL_NUM_OFFSET).map_or_else(Default::default, |f| f(accessor, material, 0))
}

/// `sv_battle_object::module_accessor(object id)`.
pub unsafe fn object_accessor(object_id: u32) -> usize {
    at::<extern "C" fn(u32) -> usize>(s::STATE_OBJECT_MODULE_ACCESSOR_OFFSET).map_or_else(Default::default, |f| f(object_id))
}

pub unsafe fn object_is_active(object_id: u32) -> bool {
    at::<extern "C" fn(u32) -> bool>(s::STATE_OBJECT_IS_ACTIVE_OFFSET).map_or_else(Default::default, |f| f(object_id))
}

pub unsafe fn sora_change_magic(fighter: usize) {
    at::<extern "C" fn(usize)>(s::STATE_SORA_CHANGE_MAGIC_OFFSET).map_or_else(Default::default, |f| f(fighter))
}

/// `utility::get_kind(accessor)`: the kind of the fighter or item behind an accessor.
pub unsafe fn object_kind(accessor: usize) -> i32 {
    at::<extern "C" fn(usize) -> i32>(s::STATE_OBJECT_KIND_OFFSET).map_or_else(Default::default, |f| f(accessor))
}

// ---- Pokemon Trainer and Kirby ----------------------------------------------------------------------

/// `LinkModule::get_parent_object_id(accessor, link)`: a Pokemon's trainer object id (or -1).
pub unsafe fn link_parent_object_id(accessor: usize, link: i32) -> i32 {
    at::<extern "C" fn(usize, i32) -> i32>(s::STATE_LINK_PARENT_OBJECT_ID_OFFSET).map_or_else(Default::default, |f| f(accessor, link))
}

pub unsafe fn set_situation_kind(accessor: usize, situation: i32) {
    at::<extern "C" fn(usize, i32, bool)>(s::STATE_SET_SITUATION_KIND_OFFSET).map_or_else(Default::default, |f| f(accessor, situation, true))
}

pub unsafe fn article_set_rate(accessor: usize, article: i32, rate: f32) {
    at::<extern "C" fn(usize, i32, f32)>(s::STATE_ARTICLE_SET_RATE_OFFSET).map_or_else(Default::default, |f| f(accessor, article, rate))
}

pub unsafe fn article_hide_whole(accessor: usize, article: i32, target: i32) {
    at::<extern "C" fn(usize, i32, bool, i32)>(s::STATE_ARTICLE_SET_VISIBILITY_WHOLE_OFFSET).map_or_else(Default::default, |f| f(accessor, article, false, target))
}

/// `FighterSpecializer_Kirby::copy_setup(kirby, opponent entry, copied kind, true, false)`.
pub unsafe fn kirby_copy_setup(accessor: usize, opponent_entry: i32, copied_kind: i32) {
    at::<extern "C" fn(usize, i32, i32, bool, bool)>(s::STATE_KIRBY_COPY_SETUP_OFFSET).map_or_else(Default::default, |f| f(accessor, opponent_entry, copied_kind, true, false))
}
