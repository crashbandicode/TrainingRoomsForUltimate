//! The game natives the save states call (package 3). Each is a guarded `Called` row of
//! `model::sites_pkg3` (a `lua_bind` thunk: the module accessor first, then its arguments).
//!
//! # Safety
//! Every function is only valid after the `StateCore` sites passed their guards, with `accessor` the
//! module accessor of a live fighter (or the manager / article / item pointer the name says).
//!
//! NOT YET SWITCH-RUN.

use crate::model::sites_pkg3 as s;
use crate::runtime::game::text_base;

unsafe fn at<F: Copy>(offset: usize) -> F {
    debug_assert_eq!(std::mem::size_of::<F>(), std::mem::size_of::<usize>());
    std::mem::transmute_copy(&(text_base() + offset))
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
    at::<extern "C" fn(usize) -> f32>(s::STATE_POS_Y_OFFSET)(accessor)
}

pub unsafe fn lr(accessor: usize) -> f32 {
    at::<extern "C" fn(usize) -> f32>(s::STATE_LR_OFFSET)(accessor)
}

pub unsafe fn set_lr(accessor: usize, lr: f32) {
    at::<extern "C" fn(usize, f32)>(s::STATE_SET_LR_OFFSET)(accessor, lr)
}

pub unsafe fn set_pos(accessor: usize, pos: &Vector3f) {
    at::<extern "C" fn(usize, *const Vector3f)>(s::STATE_SET_POS_OFFSET)(accessor, pos)
}

/// `PostureModule::init_pos(pos, true, false)`: resets ground contacts and collision history at the
/// destination; the second flag stays false so linked objects are not translated.
pub unsafe fn init_pos(accessor: usize, pos: &Vector3f) {
    at::<extern "C" fn(usize, *const Vector3f, bool, bool)>(s::STATE_INIT_POS_OFFSET)(accessor, pos, true, false)
}

pub unsafe fn clear_speed_all(accessor: usize) {
    at::<extern "C" fn(usize)>(s::STATE_CLEAR_SPEED_ALL_OFFSET)(accessor)
}

pub unsafe fn heal(accessor: usize, amount: f32) {
    at::<extern "C" fn(usize, f32, i32)>(s::STATE_HEAL_OFFSET)(accessor, amount, 0)
}

pub unsafe fn add_damage(accessor: usize, amount: f32) {
    at::<extern "C" fn(usize, f32, i32)>(s::STATE_ADD_DAMAGE_OFFSET)(accessor, amount, 0)
}

pub unsafe fn set_sleep(accessor: usize, asleep: bool) {
    at::<extern "C" fn(usize, bool)>(s::STATE_SET_SLEEP_OFFSET)(accessor, asleep)
}

pub unsafe fn stop_all_sound(accessor: usize) {
    at::<extern "C" fn(usize)>(s::STATE_STOP_ALL_SOUND_OFFSET)(accessor)
}

pub unsafe fn pause_se_all(accessor: usize, paused: bool) {
    at::<extern "C" fn(usize, bool)>(s::STATE_PAUSE_SE_ALL_OFFSET)(accessor, paused)
}

/// `SoundModule::play_se_no3d(hash, true, true)`.
pub unsafe fn play_se_no3d(accessor: usize, hash: u64) {
    at::<extern "C" fn(usize, u64, bool, bool) -> u64>(s::STATE_PLAY_SE_NO3D_OFFSET)(accessor, hash, true, true);
}

pub unsafe fn stop_rumble(accessor: usize, all: bool) {
    at::<extern "C" fn(usize, bool)>(s::STATE_STOP_RUMBLE_OFFSET)(accessor, all)
}

pub unsafe fn slow_clear_whole(accessor: usize) {
    at::<extern "C" fn(usize)>(s::STATE_SLOW_CLEAR_WHOLE_OFFSET)(accessor)
}

pub unsafe fn camera_zoom_out(accessor: usize, kind: i32) {
    at::<extern "C" fn(usize, i32)>(s::STATE_CAMERA_ZOOM_OUT_OFFSET)(accessor, kind)
}

pub unsafe fn camera_stop_quake(accessor: usize, kind: i32) {
    at::<extern "C" fn(usize, i32)>(s::STATE_CAMERA_STOP_QUAKE_OFFSET)(accessor, kind)
}

pub unsafe fn effect_kill_kind(accessor: usize, effect: u64) {
    at::<extern "C" fn(usize, u64, bool, bool)>(s::STATE_EFFECT_KILL_KIND_OFFSET)(accessor, effect, false, false)
}

pub unsafe fn effect_remove_screen(accessor: usize, effect: u64) {
    at::<extern "C" fn(usize, u64, i32)>(s::STATE_EFFECT_REMOVE_SCREEN_OFFSET)(accessor, effect, 0)
}

pub unsafe fn effect_remove_common(accessor: usize, effect: u64) {
    at::<extern "C" fn(usize, u64)>(s::STATE_EFFECT_REMOVE_COMMON_OFFSET)(accessor, effect)
}

pub unsafe fn article_is_exist(accessor: usize, article: i32) -> bool {
    at::<extern "C" fn(usize, i32) -> bool>(s::STATE_ARTICLE_IS_EXIST_OFFSET)(accessor, article)
}

/// The `Article*` of article `article`, or 0.
pub unsafe fn article_get(accessor: usize, article: i32) -> usize {
    at::<extern "C" fn(usize, i32) -> usize>(s::STATE_ARTICLE_GET_ARTICLE_OFFSET)(accessor, article)
}

pub unsafe fn article_object_id(article: usize) -> u32 {
    at::<extern "C" fn(usize) -> u32>(s::STATE_ARTICLE_OBJECT_ID_OFFSET)(article)
}

pub unsafe fn article_remove_object(accessor: usize, object_id: u32) {
    at::<extern "C" fn(usize, u32)>(s::STATE_ARTICLE_REMOVE_OBJECT_OFFSET)(accessor, object_id)
}

pub unsafe fn item_active_count(manager: usize) -> u64 {
    at::<extern "C" fn(usize) -> u64>(s::STATE_ITEM_ACTIVE_COUNT_OFFSET)(manager)
}

/// The `Item*` at `index` of the manager's active list, or 0.
pub unsafe fn item_active(manager: usize, index: u64) -> usize {
    at::<extern "C" fn(usize, u64) -> usize>(s::STATE_ITEM_ACTIVE_ITEM_OFFSET)(manager, index)
}

pub unsafe fn item_object_id(item: usize) -> u32 {
    at::<extern "C" fn(usize) -> u32>(s::STATE_ITEM_OBJECT_ID_OFFSET)(item)
}

pub unsafe fn item_remove(manager: usize, object_id: u32) {
    at::<extern "C" fn(usize, u32)>(s::STATE_ITEM_REMOVE_OFFSET)(manager, object_id)
}

pub unsafe fn work_get_int(accessor: usize, id: i32) -> i32 {
    at::<extern "C" fn(usize, i32) -> i32>(s::STATE_WORK_GET_INT_OFFSET)(accessor, id)
}

pub unsafe fn work_set_int(accessor: usize, value: i32, id: i32) {
    at::<extern "C" fn(usize, i32, i32)>(s::STATE_WORK_SET_INT_OFFSET)(accessor, value, id)
}

pub unsafe fn work_on_flag(accessor: usize, id: i32) {
    at::<extern "C" fn(usize, i32)>(s::STATE_WORK_ON_FLAG_OFFSET)(accessor, id)
}

pub unsafe fn work_off_flag(accessor: usize, id: i32) {
    at::<extern "C" fn(usize, i32)>(s::STATE_WORK_OFF_FLAG_OFFSET)(accessor, id)
}

/// `StatusModule::change_status_request(accessor, status, false)`: the placement handshake.
pub unsafe fn status_request(accessor: usize, status: i32) {
    at::<extern "C" fn(usize, i32, bool)>(s::STATE_STATUS_REQUEST_OFFSET)(accessor, status, false)
}

pub unsafe fn stage_id() -> i32 {
    at::<extern "C" fn() -> i32>(s::STATE_STAGE_ID_OFFSET)()
}

/// The ItemManager singleton, read through the slot `manager_slot_offset` recomputed from the guarded
/// instruction pair; 0 when it does not exist yet.
pub unsafe fn item_manager() -> usize {
    ((text_base() + crate::model::state::ITEM_MANAGER_SLOT_OFFSET) as *const usize).read_volatile()
}

// ---- milestone 2: charges and buffs ----------------------------------------------------------------

/// `EffectModule::req_common(accessor, effect, 0.0)`: the generic `charge_max` flash.
pub unsafe fn effect_req_common(accessor: usize, effect: u64) {
    at::<extern "C" fn(usize, u64, f32) -> u64>(s::STATE_EFFECT_REQ_COMMON_OFFSET)(accessor, effect, 0.0);
}

/// `EffectModule::req_follow(accessor, effect, joint, pos, rot, 1.0, false, attribute, 0, lifetime, 0, 0,
/// false, false)`: the handle of the effect (through the save states' own hook: a pass-through for
/// everything but `sys_score_aura`).
pub unsafe fn effect_req_follow(accessor: usize, effect: u64, joint: u64, pos: &Vector3f, rot: &Vector3f, attribute: u32, lifetime: i32) -> u64 {
    type ReqFollow = extern "C" fn(usize, u64, u64, *const Vector3f, *const Vector3f, f32, bool, u32, i32, i32, i32, i32, bool, bool) -> u64;
    at::<ReqFollow>(s::STATE_EFFECT_REQ_FOLLOW_OFFSET)(accessor, effect, joint, pos, rot, 1.0, false, attribute, 0, lifetime, 0, 0, false, false)
}

/// `EffectModule::req(accessor, effect, pos, rot, 1.0, 0, -1, false, 0)`: Min Min's arm effect.
pub unsafe fn effect_req(accessor: usize, effect: u64) -> u64 {
    type Req = extern "C" fn(usize, u64, *const Vector3f, *const Vector3f, f32, u32, i32, bool, i32) -> u64;
    let zero = Vector3f { x: 0.0, y: 0.0, z: 0.0 };
    at::<Req>(s::STATE_EFFECT_REQ_OFFSET)(accessor, effect, &zero, &zero, 1.0, 0, -1, false, 0)
}

pub unsafe fn effect_kill(accessor: usize, handle: u32) {
    at::<extern "C" fn(usize, u32, bool, bool)>(s::STATE_EFFECT_KILL_OFFSET)(accessor, handle, false, true)
}

pub unsafe fn effect_set_scale_one(accessor: usize, handle: u32) {
    let one = Vector3f { x: 1.0, y: 1.0, z: 1.0 };
    at::<extern "C" fn(usize, u32, *const Vector3f)>(s::STATE_EFFECT_SET_SCALE_OFFSET)(accessor, handle, &one)
}

/// `FighterUtil::set_face_motion_by_priority(accessor, facial, motion)`.
pub unsafe fn face_motion(accessor: usize, facial: i32, motion: u64) {
    at::<extern "C" fn(usize, i32, u64)>(s::STATE_FACE_MOTION_OFFSET)(accessor, facial, motion)
}

pub unsafe fn article_generate_enable(accessor: usize, article: i32) {
    at::<extern "C" fn(usize, i32, bool, i32)>(s::STATE_ARTICLE_GENERATE_ENABLE_OFFSET)(accessor, article, false, -1)
}

pub unsafe fn article_set_visibility(accessor: usize, article: i32, name: u64, group: u64) {
    at::<extern "C" fn(usize, i32, u64, u64, i32)>(s::STATE_ARTICLE_SET_VISIBILITY_OFFSET)(accessor, article, name, group, 0)
}

pub unsafe fn motion_set_rate(accessor: usize, rate: f32) {
    at::<extern "C" fn(usize, f32)>(s::STATE_MOTION_SET_RATE_OFFSET)(accessor, rate)
}

/// `FighterSpecializer_Jack::add_rebel_gauge(accessor, FighterEntryID(entry), amount)`.
pub unsafe fn jack_add_rebel(accessor: usize, entry: i32, amount: f32) {
    at::<extern "C" fn(usize, i32, f32)>(s::STATE_JACK_ADD_REBEL_OFFSET)(accessor, entry, amount)
}

/// The game's add-limit routine `(limit, fighter object, 0)`.
pub unsafe fn cloud_add_limit(fighter: usize, limit: f32) {
    at::<extern "C" fn(f32, usize, i32)>(s::STATE_CLOUD_ADD_LIMIT_OFFSET)(limit, fighter, 0)
}

pub unsafe fn visibility_set_status_default(accessor: usize, a: u64, b: u64) {
    at::<extern "C" fn(usize, u64, u64)>(s::STATE_VISIBILITY_SET_STATUS_DEFAULT_OFFSET)(accessor, a, b)
}

pub unsafe fn visibility_set(accessor: usize, a: u64, b: u64) {
    at::<extern "C" fn(usize, u64, u64)>(s::STATE_VISIBILITY_SET_OFFSET)(accessor, a, b)
}

/// `MotionModule::add_motion_partial(DRAGON, motion, 0.0, 1.0, false, false, 0.0, true, true, false)`.
pub unsafe fn motion_add_partial_dragon(accessor: usize, part: i32, motion: u64) {
    type Partial = extern "C" fn(usize, i32, u64, f32, f32, bool, bool, f32, bool, bool, bool);
    at::<Partial>(s::STATE_MOTION_ADD_PARTIAL_OFFSET)(accessor, part, motion, 0.0, 1.0, false, false, 0.0, true, true, false)
}

/// `StatusModule::change_status_force(accessor, status, block)` (the CPU core's native always passes
/// `true`; the buffs need `false` for Wii Fit).
pub unsafe fn status_force(accessor: usize, status: i32, block: bool) {
    at::<extern "C" fn(usize, i32, bool)>(crate::model::sites::CHANGE_STATUS_FORCE_OFFSET)(accessor, status, block)
}

/// `WorkModule::get_param_int(accessor, kind, name)` through the save states' hook (a pass-through for
/// anything but a death parameter).
pub unsafe fn get_param_int(accessor: usize, kind: u64, name: u64) -> i32 {
    at::<extern "C" fn(usize, u64, u64) -> i32>(s::STATE_GET_PARAM_INT_OFFSET)(accessor, kind, name)
}

// ---- milestone 3: items and the fighter-specific restores ------------------------------------------

/// `ItemModule::have_item(accessor, kind, variation, 0, false, false)`.
pub unsafe fn have_item(accessor: usize, kind: i32, variation: i32) {
    at::<extern "C" fn(usize, i32, i32, i32, bool, bool)>(s::STATE_HAVE_ITEM_OFFSET)(accessor, kind, variation, 0, false, false)
}

pub unsafe fn article_generate(accessor: usize, article: i32, arg: i32) {
    at::<extern "C" fn(usize, i32, bool, i32)>(s::STATE_ARTICLE_GENERATE_OFFSET)(accessor, article, false, arg)
}

pub unsafe fn article_remove_exist(accessor: usize, article: i32, target: i32) {
    at::<extern "C" fn(usize, i32, i32)>(s::STATE_ARTICLE_REMOVE_EXIST_OFFSET)(accessor, article, target)
}

pub unsafe fn article_active_num(accessor: usize, article: i32) -> i32 {
    at::<extern "C" fn(usize, i32) -> i32>(s::STATE_ARTICLE_ACTIVE_NUM_OFFSET)(accessor, article)
}

pub unsafe fn work_get_int64(accessor: usize, id: i32) -> i64 {
    at::<extern "C" fn(usize, i32) -> i64>(s::STATE_WORK_GET_INT64_OFFSET)(accessor, id)
}

pub unsafe fn sub_material_num(accessor: usize, material: i32) {
    at::<extern "C" fn(usize, i32, i32)>(s::STATE_SUB_MATERIAL_NUM_OFFSET)(accessor, material, 0)
}

/// `sv_battle_object::module_accessor(object id)`.
pub unsafe fn object_accessor(object_id: u32) -> usize {
    at::<extern "C" fn(u32) -> usize>(s::STATE_OBJECT_MODULE_ACCESSOR_OFFSET)(object_id)
}

pub unsafe fn object_is_active(object_id: u32) -> bool {
    at::<extern "C" fn(u32) -> bool>(s::STATE_OBJECT_IS_ACTIVE_OFFSET)(object_id)
}

pub unsafe fn sora_change_magic(fighter: usize) {
    at::<extern "C" fn(usize)>(s::STATE_SORA_CHANGE_MAGIC_OFFSET)(fighter)
}

pub unsafe fn is_have_item(accessor: usize) -> bool {
    at::<extern "C" fn(usize, i32) -> bool>(s::STATE_IS_HAVE_ITEM_OFFSET)(accessor, 0)
}

pub unsafe fn get_have_item_id(accessor: usize) -> i32 {
    at::<extern "C" fn(usize, i32) -> i32>(s::STATE_GET_HAVE_ITEM_ID_OFFSET)(accessor, 0)
}

pub unsafe fn remove_item(accessor: usize) {
    at::<extern "C" fn(usize, i32)>(s::STATE_REMOVE_ITEM_OFFSET)(accessor, 0)
}

pub unsafe fn set_have_item_action(accessor: usize, action: i32, power: f32) {
    at::<extern "C" fn(usize, i32, f32, i32)>(s::STATE_SET_HAVE_ITEM_ACTION_OFFSET)(accessor, action, power, 0)
}

/// `utility::get_category(object id)`.
pub unsafe fn object_category(object_id: u32) -> i32 {
    at::<extern "C" fn(u32) -> i32>(s::STATE_OBJECT_CATEGORY_OFFSET)(object_id)
}

/// `utility::get_kind(accessor)`: the kind of the fighter or item behind an accessor.
pub unsafe fn object_kind(accessor: usize) -> i32 {
    at::<extern "C" fn(usize) -> i32>(s::STATE_OBJECT_KIND_OFFSET)(accessor)
}

/// A turnip's power getter: `peach` or `daisy`, face `n` in 1..=8.
pub unsafe fn turnip_power(peach: bool, n: u8) -> f32 {
    let first = if peach { 0x1668D90 } else { 0x165BF40 };
    at::<extern "C" fn() -> f32>(first + 0x20 * (usize::from(n) - 1))()
}

// ---- Pokemon Trainer and Kirby ----------------------------------------------------------------------

/// `LinkModule::get_parent_object_id(accessor, link)`: a Pokemon's trainer object id (or -1).
pub unsafe fn link_parent_object_id(accessor: usize, link: i32) -> i32 {
    at::<extern "C" fn(usize, i32) -> i32>(s::STATE_LINK_PARENT_OBJECT_ID_OFFSET)(accessor, link)
}

pub unsafe fn set_situation_kind(accessor: usize, situation: i32) {
    at::<extern "C" fn(usize, i32, bool)>(s::STATE_SET_SITUATION_KIND_OFFSET)(accessor, situation, true)
}

pub unsafe fn article_set_rate(accessor: usize, article: i32, rate: f32) {
    at::<extern "C" fn(usize, i32, f32)>(s::STATE_ARTICLE_SET_RATE_OFFSET)(accessor, article, rate)
}

pub unsafe fn article_hide_whole(accessor: usize, article: i32, target: i32) {
    at::<extern "C" fn(usize, i32, bool, i32)>(s::STATE_ARTICLE_SET_VISIBILITY_WHOLE_OFFSET)(accessor, article, false, target)
}

/// `FighterSpecializer_Kirby::copy_setup(kirby, opponent entry, copied kind, true, false)`.
pub unsafe fn kirby_copy_setup(accessor: usize, opponent_entry: i32, copied_kind: i32) {
    at::<extern "C" fn(usize, i32, i32, bool, bool)>(s::STATE_KIRBY_COPY_SETUP_OFFSET)(accessor, opponent_entry, copied_kind, true, false)
}
