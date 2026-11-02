//! Reads of the running game: the text base, the version, the 64 bytes at a site, and the one game
//! function every feature calls (the Training-mode predicate).
//!
//! NOT YET SWITCH-COMPILED.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use skyline::hooks::{getRegionAddress, Region};
use skyline::nn::oe::{DisplayVersion, GetDisplayVersion};

use crate::model::guard::GUARD_BYTES;
use crate::model::{sites, sites_pkg2};
use crate::model::version::BUILD_ID_SCAN_BYTES;

static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAINING_PREDICATE_USABLE: AtomicBool = AtomicBool::new(false);

/// The start of `main`'s `.text` (offsets in the site table are relative to it).
pub fn text_base() -> usize {
    let cached = BASE.load(Ordering::Relaxed);
    if cached != 0 {
        return cached;
    }
    // SAFETY: Skyline's region lookup has no preconditions.
    let base = unsafe { getRegionAddress(Region::Text) } as usize;
    BASE.store(base, Ordering::Relaxed);
    base
}

/// `nn::oe::DisplayVersion.name` (16 bytes, NUL padded).
pub fn display_version() -> [u8; 16] {
    let mut version = DisplayVersion { name: [0; 16] };
    // SAFETY: the function writes at most the 16-byte name into the struct we own.
    unsafe { GetDisplayVersion(&mut version) };
    version.name
}

/// The last page of `.rodata`, where 13.0.5 keeps the GNU build-id note (`.data` follows it).
pub fn rodata_tail() -> &'static [u8] {
    // SAFETY: the regions are mapped by the loader; the tail is the page before `.data`, clipped to
    // `.rodata`'s start so a layout with a short `.rodata` cannot read before it.
    unsafe {
        let rodata = getRegionAddress(Region::Rodata) as usize;
        let data = getRegionAddress(Region::Data) as usize;
        let start = data.saturating_sub(BUILD_ID_SCAN_BYTES).max(rodata);
        std::slice::from_raw_parts(start as *const u8, data - start)
    }
}

/// The 64 bytes now at `main+offset`.
pub fn read_window(offset: usize) -> [u8; GUARD_BYTES] {
    let mut out = [0u8; GUARD_BYTES];
    // SAFETY: only called for offsets of the site table, which lie inside `main`'s mapped image.
    unsafe { std::ptr::copy_nonoverlapping((text_base() + offset) as *const u8, out.as_mut_ptr(), GUARD_BYTES) };
    out
}

/// The pointer-sized value now at `main+offset` (a vtable slot), as an offset from `.text`.
pub fn read_slot_target(offset: usize) -> u64 {
    let base = text_base();
    // SAFETY: only called for the vtable slot row, inside `main`'s mapped `.data`.
    let value = unsafe { ((base + offset) as *const u64).read_volatile() };
    value.wrapping_sub(base as u64)
}

/// Record that the Training-mode predicate passed its 64-byte guard (it is a called function:
/// other plugins' hooks on it are fine).
pub fn allow_training_predicate() {
    TRAINING_PREDICATE_USABLE.store(true, Ordering::Release);
}

/// The game's constant table lookup `lib::lua_bind_get_value<int>(key, out)` (`main+0x3728e10`), asked
/// for one constant's key (`model::cpu::hash::lua_bind_hash` of its identifier). `None` when the game
/// does not know the key (or its table is not built yet).
///
/// # Safety
/// The `lua_const_lookup` site must have passed its guard.
pub unsafe fn lua_const(key: u64) -> Option<i32> {
    let lookup: extern "C" fn(u64, *mut i32) -> bool = std::mem::transmute(text_base() + sites_pkg2::LUA_CONST_LOOKUP_OFFSET);
    let mut value = 0i32;
    lookup(key, &mut value).then_some(value)
}

/// The `lua_bind` natives the CPU behaviours read and call (all take the fighter's
/// `BattleObjectModuleAccessor` first). Each is a guarded `Called` site of package 2's core.
///
/// # Safety
/// Every function below is only valid after the `CpuCore` sites passed their guards, with
/// `accessor` the module accessor of a live fighter.
pub mod natives {
    use super::{sites_pkg2, text_base};

    unsafe fn at<F: Copy>(offset: usize) -> F {
        debug_assert_eq!(std::mem::size_of::<F>(), std::mem::size_of::<usize>());
        std::mem::transmute_copy(&(text_base() + offset))
    }

    pub unsafe fn status_kind(accessor: usize) -> i32 {
        at::<extern "C" fn(usize) -> i32>(sites_pkg2::STATUS_KIND_OFFSET)(accessor)
    }

    /// `StatusModule::prev_status_kind(accessor, index)`.
    pub unsafe fn prev_status_kind(accessor: usize, index: u32) -> i32 {
        at::<extern "C" fn(usize, u32) -> i32>(sites_pkg2::PREV_STATUS_KIND_OFFSET)(accessor, index)
    }

    pub unsafe fn situation_kind(accessor: usize) -> i32 {
        at::<extern "C" fn(usize) -> i32>(sites_pkg2::SITUATION_KIND_OFFSET)(accessor)
    }

    pub unsafe fn prev_situation_kind(accessor: usize) -> i32 {
        at::<extern "C" fn(usize) -> i32>(sites_pkg2::PREV_SITUATION_KIND_OFFSET)(accessor)
    }

    pub unsafe fn motion_frame(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg2::MOTION_FRAME_OFFSET)(accessor)
    }

    /// `MotionModule::motion_kind(accessor)` as the `phx::Hash40` value.
    pub unsafe fn motion_kind(accessor: usize) -> u64 {
        at::<extern "C" fn(usize) -> u64>(sites_pkg2::MOTION_KIND_OFFSET)(accessor)
    }

    pub unsafe fn is_enable_cancel(accessor: usize) -> bool {
        at::<extern "C" fn(usize) -> bool>(sites_pkg2::CANCEL_IS_ENABLE_OFFSET)(accessor)
    }

    /// `app::sv_information::stage_id()`: the native stage id, -1 when there is none.
    pub unsafe fn stage_id() -> i32 {
        at::<extern "C" fn() -> i32>(sites_pkg2::STAGE_ID_CALL_OFFSET)()
    }

    /// The form field of the native stage table's record for `stage_id` (0 the stage, 1 its Omega form, 2 its
    /// Battlefield form); `None` for an id outside the table.
    pub unsafe fn stage_form(stage_id: i32) -> Option<u32> {
        if !(0..crate::model::cpu::tech::stage::STAGE_COUNT).contains(&stage_id) {
            return None;
        }
        let record = text_base() + sites_pkg2::STAGE_TABLE_OFFSET + stage_id as usize * sites_pkg2::STAGE_RECORD_STRIDE;
        Some(((record + sites_pkg2::STAGE_RECORD_FORM) as *const u64).read() as u32)
    }

    pub unsafe fn pos_y(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg2::POS_Y_OFFSET)(accessor)
    }

    pub unsafe fn pos_z(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg2::POS_Z_OFFSET)(accessor)
    }

    pub unsafe fn get_int(accessor: usize, id: i32) -> i32 {
        at::<extern "C" fn(usize, i32) -> i32>(sites_pkg2::WORK_GET_INT_OFFSET)(accessor, id)
    }

    /// `KineticModule::get_sum_speed_y(accessor, energy id)`.
    pub unsafe fn sum_speed_y(accessor: usize, energy: i32) -> f32 {
        at::<extern "C" fn(usize, i32) -> f32>(sites_pkg2::KINETIC_SUM_SPEED_Y_OFFSET)(accessor, energy)
    }

    /// `EffectModule::req(accessor, effect, pos, rot, scale, 0, 0, true, attribute)`.
    pub unsafe fn effect_req(accessor: usize, effect: u64, pos: &[f32; 3], scale: f32, attribute: i32) {
        type Req = extern "C" fn(usize, u64, *const [f32; 3], *const [f32; 3], f32, u32, i32, bool, i32) -> u64;
        let zero = [0.0f32; 3];
        at::<Req>(sites_pkg2::EFFECT_REQ_OFFSET)(accessor, effect, pos, &zero, scale, 0, 0, true, attribute);
    }

    pub unsafe fn pos_x(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg2::POS_X_OFFSET)(accessor)
    }

    pub unsafe fn is_flag(accessor: usize, id: i32) -> bool {
        at::<extern "C" fn(usize, i32) -> bool>(sites_pkg2::WORK_IS_FLAG_OFFSET)(accessor, id)
    }

    pub unsafe fn set_flag(accessor: usize, value: bool, id: i32) {
        at::<extern "C" fn(usize, bool, i32)>(sites_pkg2::WORK_SET_FLAG_OFFSET)(accessor, value, id)
    }

    /// `StatusModule::change_status_request_from_script(accessor, status, true)`.
    pub unsafe fn change_status_request(accessor: usize, status: i32) {
        at::<extern "C" fn(usize, i32, bool)>(sites_pkg2::CHANGE_STATUS_REQUEST_OFFSET)(accessor, status, true)
    }

    /// `WorkModule::is_enable_transition_term(accessor, term)`: the function the ledge hook wraps (the
    /// call goes through the hook, which answers the original for any term it does not own).
    pub unsafe fn is_enable_transition_term(accessor: usize, term: i32) -> bool {
        at::<extern "C" fn(usize, i32) -> bool>(sites_pkg2::TRANSITION_TERM_HOOK_OFFSET)(accessor, term)
    }

    /// `StatusModule::change_status_force(accessor, status, true)`.
    pub unsafe fn change_status_force(accessor: usize, status: i32) {
        at::<extern "C" fn(usize, i32, bool)>(sites_pkg2::CHANGE_STATUS_FORCE_OFFSET)(accessor, status, true)
    }

    pub unsafe fn set_whole(accessor: usize, visible: bool) {
        at::<extern "C" fn(usize, bool)>(sites_pkg2::VISIBILITY_SET_WHOLE_OFFSET)(accessor, visible)
    }

    /// `EffectModule::set_visible_kind(accessor, Hash40, visible)`.
    pub unsafe fn set_visible_kind(accessor: usize, effect: u64, visible: bool) {
        at::<extern "C" fn(usize, u64, bool)>(sites_pkg2::EFFECT_SET_VISIBLE_KIND_OFFSET)(accessor, effect, visible)
    }

    pub unsafe fn motion_end_frame(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg2::MOTION_END_FRAME_OFFSET)(accessor)
    }

    /// `DamageModule::damage(accessor, index)`.
    pub unsafe fn damage(accessor: usize, index: i32) -> f32 {
        at::<extern "C" fn(usize, i32) -> f32>(sites_pkg2::DAMAGE_OF_OFFSET)(accessor, index)
    }

    pub unsafe fn get_float(accessor: usize, id: i32) -> f32 {
        at::<extern "C" fn(usize, i32) -> f32>(sites_pkg2::WORK_GET_FLOAT_OFFSET)(accessor, id)
    }

    pub unsafe fn set_float(accessor: usize, value: f32, id: i32) {
        at::<extern "C" fn(usize, f32, i32)>(sites_pkg2::WORK_SET_FLOAT_OFFSET)(accessor, value, id)
    }

    /// `WorkModule::get_param_float(accessor, param type hash, param hash)`.
    pub unsafe fn get_param_float(accessor: usize, kind: u64, name: u64) -> f32 {
        at::<extern "C" fn(usize, u64, u64) -> f32>(sites_pkg2::WORK_GET_PARAM_FLOAT_OFFSET)(accessor, kind, name)
    }

    /// `ControlModule::add_clatter_time(accessor, delta, 0)`.
    pub unsafe fn add_clatter_time(accessor: usize, delta: f32) {
        at::<extern "C" fn(usize, f32, i32)>(sites_pkg2::ADD_CLATTER_TIME_OFFSET)(accessor, delta, 0)
    }

    /// `EffectModule::req_on_joint(accessor, effect, joint, pos, rot, scale, pos_rand, rot_rand, true,
    /// attribute, 0, 0)` with zero offsets and rotations: the `sys_clatter` flash (SPEC-cpu-02 §4).
    pub unsafe fn req_on_joint(accessor: usize, effect: u64, joint: u64, scale: f32, attribute: u32) {
        type ReqOnJoint =
            extern "C" fn(usize, u64, u64, *const [f32; 3], *const [f32; 3], f32, *const [f32; 3], *const [f32; 3], bool, u32, i32, i32) -> u64;
        let zero = [0.0f32; 3];
        at::<ReqOnJoint>(sites_pkg2::EFFECT_REQ_ON_JOINT_OFFSET)(accessor, effect, joint, &zero, &zero, scale, &zero, &zero, true, attribute, 0, 0);
    }
}

/// The game's `app::smashball::is_training_mode()`; false when its guard did not pass.
pub fn is_training_mode() -> bool {
    if !TRAINING_PREDICATE_USABLE.load(Ordering::Acquire) {
        return false;
    }
    // SAFETY: the site was guarded against the 13.0.5 bytes; the function takes no arguments and
    // returns a bool.
    let predicate: extern "C" fn() -> bool =
        unsafe { std::mem::transmute(text_base() + sites::IS_TRAINING_MODE_OFFSET) };
    predicate()
}
