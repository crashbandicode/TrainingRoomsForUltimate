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
