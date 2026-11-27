//! Reads of the running game: the text base, the version, the 64 bytes at a site, and the one game
//! function every feature calls (the Training-mode predicate).
//!
//! Part of the Switch build.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

use skyline::hooks::{getRegionAddress, Region};
use skyline::nn::oe::{DisplayVersion, GetDisplayVersion};

use crate::model::capabilities::Granted;
use crate::model::guard::{GuardOutcome, GUARD_BYTES};
use crate::model::{lra_reset, sites, sites_pkg2, sites_pkg4};
use crate::model::version::BUILD_ID_SCAN_BYTES;

static BASE: AtomicUsize = AtomicUsize::new(0);
/// The natives whose rows passed their guards (`model::capabilities::Granted`), set once by boot before any
/// hook is installed. Until then, and for any offset that was not granted, the native-call layer calls nothing
/// (R2-1).
static GRANTED: OnceLock<Granted> = OnceLock::new();
static TRAINING_PREDICATE_USABLE: AtomicBool = AtomicBool::new(false);
/// Which groups of the fighter natives passed the guards of the rows they call (set by boot).
static ENTRY_NATIVES_USABLE: AtomicBool = AtomicBool::new(false);
static OPERATOR_NATIVES_USABLE: AtomicBool = AtomicBool::new(false);
static CATEGORY_NATIVES_USABLE: AtomicBool = AtomicBool::new(false);

/// A group of `natives` that boot enables once its rows are usable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeGroup {
    /// `fighter_entry`, `entry_active_body`, `module_accessor`: row `fighter_manager_get_entry`.
    Entry,
    /// `is_operation_cpu`: rows `fighter_information` and `is_operation_cpu`.
    Operator,
    /// `is_fighter`: rows `utility_get_category` and `utility_get_kind`.
    Category,
}

/// Boot hands the native-call layer the set of natives it may call (once; a second call is ignored).
pub fn grant(granted: Granted) {
    let _ = GRANTED.set(granted);
}

/// Whether the native at `main+offset` may be called: its rows passed their guards at boot.
pub fn granted(offset: usize) -> bool {
    GRANTED.get().is_some_and(|set| set.allows(offset))
}

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

/// Every row's outcome against the 64 bytes now at its offset (an install's phase one: before anything is written).
pub fn outcomes<const N: usize>(rows: &[sites::Site; N]) -> [GuardOutcome; N] {
    std::array::from_fn(|i| {
        let site = &rows[i];
        if site.compared() { site.outcome(&read_window(site.offset)) }
        else { site.slot_outcome(read_slot_target(site.offset)) }
    })
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

/// Record that every row a group of fighter natives calls or relies on passed its 64-byte guard. Until
/// then those natives answer "nothing" (0, `None`, false).
pub fn allow_natives(group: NativeGroup) {
    match group {
        NativeGroup::Entry => &ENTRY_NATIVES_USABLE,
        NativeGroup::Operator => &OPERATOR_NATIVES_USABLE,
        NativeGroup::Category => &CATEGORY_NATIVES_USABLE,
    }
    .store(true, Ordering::Release);
}

/// The game's constant table lookup `lib::lua_bind_get_value<int>(key, out)` (`main+0x3728e10`), asked
/// for one constant's key (`model::cpu::hash::lua_bind_hash` of its identifier). `None` when the game
/// does not know the key (or its table is not built yet).
///
/// # Safety
/// The `lua_const_lookup` site must have passed its guard.
pub unsafe fn lua_const(key: u64) -> Option<i32> {
    if !granted(sites_pkg2::LUA_CONST_LOOKUP_OFFSET) {
        return None;
    }
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
    use std::sync::atomic::Ordering;

    use super::{
        lra_reset, sites, sites_pkg2, sites_pkg4, text_base, CATEGORY_NATIVES_USABLE, ENTRY_NATIVES_USABLE, OPERATOR_NATIVES_USABLE,
    };
    use crate::model::cpu::ids::UNRESOLVED;
    use crate::runtime::features::cpu;

    /// The fighter object: its `BattleObjectModuleAccessor` pointer.
    pub const FIGHTER_ACCESSOR_FIELD: usize = 0x20;
    /// The accessor: its battle object id (`utility::get_kind`'s `ldr w0, [x0, #8]`, guarded by the
    /// `utility_get_kind` row).
    pub const ACCESSOR_OBJECT_ID_FIELD: usize = 0x8;
    /// `utility::get_category` of a fighter's battle object id.
    const CATEGORY_FIGHTER: i32 = 0;

    /// The native at `main+offset`, or `None` when its rows did not pass their guards at boot (`granted`): a
    /// native that was not granted is never called, and the function that asked answers nothing (R2-1).
    unsafe fn at<F: Copy>(offset: usize) -> Option<F> {
        debug_assert_eq!(std::mem::size_of::<F>(), std::mem::size_of::<usize>());
        if !super::granted(offset) {
            return None;
        }
        Some(std::mem::transmute_copy(&(text_base() + offset)))
    }

    /// Native two-register pair return (AAPCS64 x0/x1), not a hidden sret.
    pub use crate::runtime::features::tech_hitlag_read::ParamPair as HitlagParamPair;
    pub unsafe fn tech_hitlag_param_pair(object: usize, hash: u64) -> Option<HitlagParamPair> {
        Some(at::<extern "C" fn(usize,u64)->HitlagParamPair>(crate::model::sites_tech_hitlag::PAIR_OFFSET)?(object,hash))
    }
    pub unsafe fn tech_hitlag_param_float(pair: &HitlagParamPair) -> Option<f32> {
        Some(at::<extern "C" fn(*const HitlagParamPair)->f32>(crate::model::sites_tech_hitlag::FLOAT_OFFSET)?(pair))
    }
    pub unsafe fn tech_hitlag_param_int(pair: &HitlagParamPair) -> Option<i32> {
        Some(at::<extern "C" fn(*const HitlagParamPair)->i32>(crate::model::sites_tech_hitlag::INT_OFFSET)?(pair))
    }
    pub unsafe fn tech_hitlag_k_lookup(map: usize, hash: u64) -> Option<f32> {
        Some(at::<extern "C" fn(usize,u64)->f32>(crate::model::sites_tech_hitlag::K_OFFSET)?(map,hash))
    }
    pub unsafe fn tech_magnitude(kb: f32) -> Option<f32> {
        let f = at::<extern "C" fn(usize,usize,i32,i32,f32,f32,f32)->f32>(crate::model::sites_tech_launch::MAGNITUDE_OFFSET)?;
        Some(f(0,0,0,0,kb,0.0,0.0))
    }
    pub unsafe fn tech_reaction(kb: f32) -> Option<f32> {
        Some(at::<extern "C" fn(f32)->f32>(crate::model::sites_tech_launch::REACTION_OFFSET)?(kb))
    }
    pub unsafe fn tech_cos(angle: f32) -> Option<f32> {
        Some(at::<extern "C" fn(f32)->f32>(crate::model::sites_tech_launch::COS_OFFSET)?(angle))
    }
    pub unsafe fn tech_sin(angle: f32) -> Option<f32> {
        Some(at::<extern "C" fn(f32)->f32>(crate::model::sites_tech_launch::SIN_OFFSET)?(angle))
    }
    pub unsafe fn tech_overwrite_reaction(accessor: usize, frames: f32) -> Option<()> {
        at::<extern "C" fn(usize,f32)>(crate::model::sites_tech_launch::OVERWRITE_REACTION_OFFSET)?(accessor,frames); Some(())
    }
    pub unsafe fn tech_is_damage(accessor: usize) -> Option<bool> {
        Some(at::<extern "C" fn(usize)->bool>(crate::model::sites_tech_launch::IS_DAMAGE_OFFSET)?(accessor))
    }
    pub unsafe fn tech_in_damage_stop(accessor: usize) -> Option<bool> {
        let offset = crate::model::sites_tech_surfaces::DAMAGE_STOP_OFFSET;
        Some(at::<extern "C" fn(usize) -> bool>(offset)?(accessor))
    }
    pub unsafe fn tech_passive_ground_touch(accessor: usize, direction: u32) -> Option<bool> {
        let offset = crate::model::sites_tech_surfaces::PASSIVE_GROUND_OFFSET;
        Some(at::<extern "C" fn(usize, u32) -> bool>(offset)?(accessor, direction))
    }
    pub unsafe fn tech_energy(accessor: usize, id: i32) -> Option<usize> {
        Some(at::<extern "C" fn(usize,i32)->usize>(crate::model::sites_tech_launch::GET_ENERGY_OFFSET)?(accessor,id))
    }
    pub unsafe fn tech_speed(energy: usize) -> Option<(f32,f32)> {
        let x = at::<extern "C" fn(usize)->f32>(crate::model::sites_tech_launch::SPEED_X_OFFSET)?(energy);
        let y = at::<extern "C" fn(usize)->f32>(crate::model::sites_tech_launch::SPEED_Y_OFFSET)?(energy);
        Some((x,y))
    }

    pub unsafe fn status_kind(accessor: usize) -> i32 {
        at::<extern "C" fn(usize) -> i32>(sites_pkg2::STATUS_KIND_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    /// `StatusModule::prev_status_kind(accessor, index)`.
    pub unsafe fn prev_status_kind(accessor: usize, index: u32) -> i32 {
        at::<extern "C" fn(usize, u32) -> i32>(sites_pkg2::PREV_STATUS_KIND_OFFSET).map_or_else(Default::default, |f| f(accessor, index))
    }

    pub unsafe fn situation_kind(accessor: usize) -> i32 {
        at::<extern "C" fn(usize) -> i32>(sites_pkg2::SITUATION_KIND_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    pub unsafe fn prev_situation_kind(accessor: usize) -> i32 {
        at::<extern "C" fn(usize) -> i32>(sites_pkg2::PREV_SITUATION_KIND_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    pub unsafe fn motion_frame(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg2::MOTION_FRAME_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    /// `MotionModule::motion_kind(accessor)` as the `phx::Hash40` value.
    pub unsafe fn motion_kind(accessor: usize) -> u64 {
        at::<extern "C" fn(usize) -> u64>(sites_pkg2::MOTION_KIND_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    /// `MotionModule::is_anim_resource(accessor, motion)`: whether the fighter has an animation of that name. A
    /// row that was not granted answers no.
    pub unsafe fn is_anim_resource(accessor: usize, motion: u64) -> bool {
        at::<extern "C" fn(usize, u64) -> bool>(sites_pkg2::MOTION_IS_ANIM_RESOURCE_OFFSET).is_some_and(|f| f(accessor, motion))
    }

    /// `run_lines`: the game runs the animation's script lines up to `frame`, as it does for its own calls. False
    /// starts the animation at `frame` and runs none of them (the native's last argument, true).
    /// Returns whether the guarded native was called, not whether the game accepted the motion.
    pub unsafe fn change_motion(accessor: usize, motion: u64, frame: f32, rate: f32, run_lines: bool) -> bool {
        if let Some(f) = at::<extern "C" fn(usize, u64, f32, f32, bool, f32, bool, bool) -> u64>(sites_pkg2::MOTION_CHANGE_HOOK_OFFSET) {
            f(accessor, motion, frame, rate, false, 0.0, false, !run_lines);
            true
        } else { false }
    }

    pub unsafe fn is_enable_cancel(accessor: usize) -> bool {
        at::<extern "C" fn(usize) -> bool>(sites_pkg2::CANCEL_IS_ENABLE_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    /// `app::sv_information::stage_id()`: the native stage id, -1 when there is none (also when its row was not
    /// granted).
    pub unsafe fn stage_id() -> i32 {
        at::<extern "C" fn() -> i32>(sites_pkg2::STAGE_ID_CALL_OFFSET).map_or(-1, |f| f())
    }

    /// The form field of the native stage table's record for `stage_id` (0 the stage, 1 its Omega form, 2 its
    /// Battlefield form); `None` for an id outside the table, or when the table's row was not granted (R2-1).
    pub unsafe fn stage_form(stage_id: i32) -> Option<u32> {
        if !super::granted(sites_pkg2::STAGE_TABLE_OFFSET) || !(0..crate::model::cpu::tech::stage::STAGE_COUNT).contains(&stage_id) {
            return None;
        }
        let record = text_base() + sites_pkg2::STAGE_TABLE_OFFSET + stage_id as usize * sites_pkg2::STAGE_RECORD_STRIDE;
        Some(((record + sites_pkg2::STAGE_RECORD_FORM) as *const u64).read() as u32)
    }

    /// The camera manager's `change_active_camera(manager, mode, 0, &functor, 0)`, called as the game's own callers call it
    /// (priority 0, flag 0, an in-place no-op functor: word 0 its vtable, word 4 itself, destroyed through the vtable after the call):
    /// whether the game changed its mode. `None` when the row was not granted, or the vtable's destroy slots do not point into `.text`
    /// (the vtable is relocated at load and is zero in the owned ELF, so only the running game can show it).
    pub unsafe fn change_active_camera(manager: usize, mode: i32) -> Option<bool> {
        if !super::granted(sites_pkg2::CHANGE_ACTIVE_CAMERA_OFFSET) {
            return None;
        }
        let vtable = text_base() + sites_pkg2::CAMERA_FUNCTOR_VTABLE;
        let text_end = skyline::hooks::getRegionAddress(skyline::hooks::Region::Rodata) as usize;
        let read = |p: usize| (p as *const usize).read_volatile();
        // A pointer into `.text`, 4-aligned: what a function slot holds.
        let code = |p: usize| p >= text_base() && p < text_end && p & 3 == 0;
        if !(code(read(vtable + 0x20)) && code(read(vtable + 0x28))) {
            return None;
        }
        #[repr(C, align(16))]
        struct Functor([usize; 6]);
        let mut functor = Functor([0; 6]);
        let this = std::ptr::addr_of_mut!(functor) as usize;
        (functor.0[0], functor.0[4]) = (vtable, this);
        let changed = at::<extern "C" fn(usize, u32, i32, usize, u32) -> bool>(sites_pkg2::CHANGE_ACTIVE_CAMERA_OFFSET)?(manager, mode as u32, 0, this, 0);
        // As every game caller does: destroy the callable the call left (in place through slot 0x20, on the heap through 0x28).
        let live = std::ptr::read_volatile(&functor.0[4]);
        if live != 0 && live & 7 == 0 {
            let slot = read(read(live) + [0x28, 0x20][usize::from(live == this)]);
            if code(slot) {
                let destroy: extern "C" fn(usize) = std::mem::transmute(slot);
                destroy(live);
            }
        }
        Some(changed)
    }

    pub unsafe fn pos_y(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg2::POS_Y_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    pub unsafe fn pos_z(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg2::POS_Z_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    pub unsafe fn get_int(accessor: usize, id: i32) -> i32 {
        at::<extern "C" fn(usize, i32) -> i32>(sites_pkg2::WORK_GET_INT_OFFSET).map_or_else(Default::default, |f| f(accessor, id))
    }

    /// `KineticModule::get_sum_speed_y(accessor, energy id)`.
    pub unsafe fn sum_speed_y(accessor: usize, energy: i32) -> f32 {
        at::<extern "C" fn(usize, i32) -> f32>(sites_pkg2::KINETIC_SUM_SPEED_Y_OFFSET).map_or_else(Default::default, |f| f(accessor, energy))
    }

    /// `EffectModule::req(accessor, effect, pos, rot, scale, tail.0, tail.1, tail.2, tail.3)`: `tail` is the native's last four
    /// arguments in the symbol's order (`u32, i32, bool, i32`). The effect handle, 0 when the row was not granted.
    pub unsafe fn effect_req(accessor: usize, effect: u64, pos: &[f32; 3], rot: &[f32; 3], scale: f32, tail: (u32, i32, bool, i32)) -> u64 {
        type Req = extern "C" fn(usize, u64, *const [f32; 3], *const [f32; 3], f32, u32, i32, bool, i32) -> u64;
        at::<Req>(sites_pkg2::EFFECT_REQ_OFFSET).map_or_else(Default::default, |f| f(accessor, effect, pos, rot, scale, tail.0, tail.1, tail.2, tail.3))
    }

    pub unsafe fn pos_x(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg2::POS_X_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    pub unsafe fn is_flag(accessor: usize, id: i32) -> bool {
        at::<extern "C" fn(usize, i32) -> bool>(sites_pkg2::WORK_IS_FLAG_OFFSET).map_or_else(Default::default, |f| f(accessor, id))
    }

    pub unsafe fn set_flag(accessor: usize, value: bool, id: i32) {
        at::<extern "C" fn(usize, bool, i32)>(sites_pkg2::WORK_SET_FLAG_OFFSET).map_or_else(Default::default, |f| f(accessor, value, id))
    }

    /// `StatusModule::change_status_request_from_script(accessor, status, true)`.
    pub unsafe fn change_status_request(accessor: usize, status: i32) {
        at::<extern "C" fn(usize, i32, bool)>(sites_pkg2::CHANGE_STATUS_REQUEST_OFFSET).map_or_else(Default::default, |f| f(accessor, status, true))
    }

    /// `WorkModule::is_enable_transition_term(accessor, term)`: the function the ledge hook wraps (the
    /// call goes through the hook, which answers the original for any term it does not own).
    pub unsafe fn is_enable_transition_term(accessor: usize, term: i32) -> bool {
        at::<extern "C" fn(usize, i32) -> bool>(sites_pkg2::TRANSITION_TERM_HOOK_OFFSET).map_or_else(Default::default, |f| f(accessor, term))
    }

    /// `StatusModule::change_status_force(accessor, status, block)` (the CPU behaviours pass `true`; the save states' buffs need
    /// `false` for Wii Fit).
    pub unsafe fn change_status_force(accessor: usize, status: i32, block: bool) {
        at::<extern "C" fn(usize, i32, bool)>(sites_pkg2::CHANGE_STATUS_FORCE_OFFSET).map_or_else(Default::default, |f| f(accessor, status, block))
    }

    pub unsafe fn set_whole(accessor: usize, visible: bool) {
        at::<extern "C" fn(usize, bool)>(sites_pkg2::VISIBILITY_SET_WHOLE_OFFSET).map_or_else(Default::default, |f| f(accessor, visible))
    }

    /// `EffectModule::set_visible_kind(accessor, Hash40, visible)`.
    pub unsafe fn set_visible_kind(accessor: usize, effect: u64, visible: bool) {
        at::<extern "C" fn(usize, u64, bool)>(sites_pkg2::EFFECT_SET_VISIBLE_KIND_OFFSET).map_or_else(Default::default, |f| f(accessor, effect, visible))
    }

    pub unsafe fn motion_end_frame(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg2::MOTION_END_FRAME_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    /// `DamageModule::damage(accessor, index)`.
    pub unsafe fn damage(accessor: usize, index: i32) -> f32 {
        at::<extern "C" fn(usize, i32) -> f32>(sites_pkg2::DAMAGE_OF_OFFSET).map_or_else(Default::default, |f| f(accessor, index))
    }

    pub unsafe fn get_float(accessor: usize, id: i32) -> f32 {
        at::<extern "C" fn(usize, i32) -> f32>(sites_pkg2::WORK_GET_FLOAT_OFFSET).map_or_else(Default::default, |f| f(accessor, id))
    }

    /// Concrete ControlModule ABI, after the caller verifies its vtable/dispatch slot.
    pub unsafe fn tech_guard_count(module: usize, button: i32) -> Option<u8> {
        at::<extern "C" fn(usize, i32) -> u8>(crate::model::sites_tech_launch::GUARD_COUNT_OFFSET).map(|f| f(module, button))
    }
    pub unsafe fn tech_guard_previous(module: usize, button: i32) -> Option<u8> {
        at::<extern "C" fn(usize, i32) -> u8>(crate::model::sites_tech_launch::GUARD_PREVIOUS_OFFSET).map(|f| f(module, button))
    }
    pub unsafe fn tech_stick_x(module: usize) -> Option<f32> {
        at::<extern "C" fn(usize) -> f32>(crate::model::sites_tech_launch::STICK_X_OFFSET).map(|f| f(module))
    }
    pub unsafe fn tech_guard_trigger(module: usize, button: i32) -> Option<bool> {
        at::<extern "C" fn(usize, i32) -> bool>(crate::model::sites_tech_launch::GUARD_TRIGGER_OFFSET).map(|f| f(module, button))
    }

    pub unsafe fn set_float(accessor: usize, value: f32, id: i32) {
        at::<extern "C" fn(usize, f32, i32)>(sites_pkg2::WORK_SET_FLOAT_OFFSET).map_or_else(Default::default, |f| f(accessor, value, id))
    }

    /// One guarded utility::get_kind boundary for Developer and save-state callers.
    /// Absence remains distinct so each consumer keeps its existing refusal default.
    pub unsafe fn fighter_kind(accessor: usize) -> Option<i32> {
        at::<extern "C" fn(usize) -> i32>(sites_pkg2::UTILITY_GET_KIND_OFFSET).map(|f| f(accessor))
    }

    /// Owned ELF 1655610: const accessor argument, inverted flag bit0 result.
    pub unsafe fn backshield_visible(accessor: usize) -> bool {
        at::<extern "C" fn(usize) -> bool>(crate::model::developer_debug::BACKSHIELD_VISIBLE_OFFSET).is_some_and(|f| f(accessor))
    }

    /// `WorkModule::get_param_float(accessor, param type hash, param hash)`.
    pub unsafe fn get_param_float(accessor: usize, kind: u64, name: u64) -> f32 {
        at::<extern "C" fn(usize, u64, u64) -> f32>(sites_pkg2::WORK_GET_PARAM_FLOAT_OFFSET).map_or_else(Default::default, |f| f(accessor, kind, name))
    }

    /// `ControlModule::add_clatter_time(accessor, delta, 0)`.
    pub unsafe fn add_clatter_time(accessor: usize, delta: f32) {
        at::<extern "C" fn(usize, f32, i32)>(sites_pkg2::ADD_CLATTER_TIME_OFFSET).map_or_else(Default::default, |f| f(accessor, delta, 0))
    }

    /// `EffectModule::req_on_joint(accessor, effect, joint, pos, rot, scale, pos_rand, rot_rand, true,
    /// attribute, 0, 0)` with zero offsets and rotations: the `sys_clatter` flash (SPEC-cpu-02 §4).
    pub unsafe fn req_on_joint(accessor: usize, effect: u64, joint: u64, scale: f32, attribute: u32) {
        type ReqOnJoint =
            extern "C" fn(usize, u64, u64, *const [f32; 3], *const [f32; 3], f32, *const [f32; 3], *const [f32; 3], bool, u32, i32, i32) -> u64;
        let zero = [0.0f32; 3];
        at::<ReqOnJoint>(sites_pkg2::EFFECT_REQ_ON_JOINT_OFFSET).map_or_else(Default::default, |f| f(accessor, effect, joint, &zero, &zero, scale, &zero, &zero, true, attribute, 0, 0));
    }

    /// `PostureModule::lr(accessor)`: the facing, +1.0 or -1.0.
    pub unsafe fn lr(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(sites_pkg4::POSTURE_LR_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    /// `StopModule::is_stop(accessor)`: the body is in hitstop.
    pub unsafe fn is_stop(accessor: usize) -> bool {
        at::<extern "C" fn(usize) -> bool>(sites_pkg4::STOP_IS_STOP_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    /// `StopModule::set_hit_stop_frame(accessor, frames, damage)`: the body is in hitstop for `frames` ticks, as after a hit.
    pub unsafe fn set_hit_stop_frame(accessor: usize, frames: i32, damage: bool) -> Option<()> {
        at::<extern "C" fn(usize, i32, bool)>(crate::model::sites_hitbox::HITBOX_HIT_STOP_OFFSET)?(accessor, frames, damage);
        Some(())
    }

    /// `WorkModule::set_int(accessor, value, id)`.
    pub unsafe fn set_int(accessor: usize, value: i32, id: i32) {
        at::<extern "C" fn(usize, i32, i32)>(sites_pkg4::WORK_SET_INT_OFFSET).map_or_else(Default::default, |f| f(accessor, value, id))
    }

    /// `ModelModule::set_color_rgb(accessor, r, g, b, MODEL_COLOR_TYPE)`.
    pub unsafe fn set_color_rgb(accessor: usize, r: f32, g: f32, b: f32, kind: i32) {
        at::<extern "C" fn(usize, f32, f32, f32, i32)>(sites_pkg4::MODEL_SET_COLOR_RGB_OFFSET).map_or_else(Default::default, |f| f(accessor, r, g, b, kind))
    }

    // ---- fighters by entry id -------------------------------------------------------------------------
    //
    // The functions below answer nothing (0, `None`, false) until boot has called `allow_natives` for their
    // group after the guards of the rows they use passed (`NativeGroup`). They read the live game on every
    // call and keep no pointer.

    /// The `FighterManager` singleton, or 0.
    unsafe fn fighter_manager() -> usize {
        ((text_base() + lra_reset::FIGHTER_MANAGER_SLOT_OFFSET) as *const usize).read_volatile()
    }

    /// `FighterManager::get_fighter_entry(manager, entry id)`: the entry object of a fighter entry
    /// (0 is player 1; 0 when the entry does not exist or the natives are not usable).
    pub unsafe fn fighter_entry(entry: i32) -> usize {
        if !ENTRY_NATIVES_USABLE.load(Ordering::Acquire) || !lra_reset::entry_id_valid(entry) {
            return 0;
        }
        let manager = fighter_manager();
        if manager == 0 {
            return 0;
        }
        at::<extern "C" fn(usize, i32) -> usize>(sites_pkg4::FIGHTER_MANAGER_GET_ENTRY_OFFSET).map_or_else(Default::default, |f| f(manager, entry))
    }

    /// F11: the entry's running body (the fighter object), or 0: the window cell the game marks active (`+0x5918`, counted by
    /// `+0x4150`; `model::entry_cells::active_cell`), which a body-bank group swap or a native Pokemon Trainer switch changes, so it
    /// is the body the fighter actually is. Without the guarded layout (`sites_pkg6::entry_active_cell`, checked at boot) it is the
    /// first body (window cell 0). Every fighter identity reads this one: the first body is not the fighter on screen after a
    /// live swap (CODE-FINDINGS #25). Game thread only: it reads the live entry; worker and menu threads read published facts.
    pub(crate) unsafe fn entry_active_body(entry_ptr: usize) -> usize {
        crate::model::entry_cells::read_active_body(
            entry_ptr,
            ACTIVE_CELL_GUARDED.load(Ordering::Acquire),
            |address| (address as *const u64).read(),
            |address| (address as *const u8).read(),
        )
    }

    /// F11: the layout `entry_active_body` relies on passed its guard (set once at boot).
    pub(crate) static ACTIVE_CELL_GUARDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    /// The fighter object's `BattleObjectModuleAccessor`, or 0.
    pub unsafe fn fighter_accessor(fighter: usize) -> usize {
        if fighter == 0 {
            return 0;
        }
        ((fighter + FIGHTER_ACCESSOR_FIELD) as *const usize).read()
    }

    /// The module accessor of the running body of a fighter entry (`None` when the entry or its body does not exist): the game's
    /// active cell (F11, `entry_active_body`), so a body-bank group swap or a Pokemon Trainer switch is followed.
    pub unsafe fn module_accessor(entry: i32) -> Option<usize> {
        let accessor = fighter_accessor(entry_active_body(fighter_entry(entry)));
        (accessor != 0).then_some(accessor)
    }

    /// The full object ID of a live module accessor.
    pub unsafe fn battle_object_id(accessor: usize) -> u32 {
        ((accessor + ACCESSOR_OBJECT_ID_FIELD) as *const u32).read()
    }

    /// Whether the accessor belongs to a fighter (its battle object id is in the fighter category).
    pub unsafe fn is_fighter(accessor: usize) -> bool {
        if accessor == 0 || !CATEGORY_NATIVES_USABLE.load(Ordering::Acquire) {
            return false;
        }
        let id = battle_object_id(accessor);
        at::<extern "C" fn(u32) -> i32>(sites_pkg2::UTILITY_GET_CATEGORY_OFFSET).map_or_else(Default::default, |f| f(id)) == CATEGORY_FIGHTER
    }

    /// Whether the body is on the ground (`StatusModule::situation_kind` is `SITUATION_KIND_GROUND`);
    /// false until the CPU runtime has resolved the game's constants.
    pub unsafe fn is_grounded(accessor: usize) -> bool {
        let ground = cpu::situation_ground();
        accessor != 0 && ground != UNRESOLVED && situation_kind(accessor) == ground
    }

    /// `FighterInformation::is_operation_cpu` of an entry: the game's own operator flag.
    pub unsafe fn is_operation_cpu(entry: i32) -> bool {
        if !OPERATOR_NATIVES_USABLE.load(Ordering::Acquire) || !lra_reset::entry_id_valid(entry) {
            return false;
        }
        let manager = fighter_manager();
        if manager == 0 {
            return false;
        }
        let information = at::<extern "C" fn(usize, i32) -> usize>(sites::FIGHTER_INFORMATION_OFFSET).map_or_else(Default::default, |f| f(manager, entry));
        if information == 0 {
            return false;
        }
        at::<extern "C" fn(usize) -> bool>(sites::IS_OPERATION_CPU_OFFSET).map_or_else(Default::default, |f| f(information))
    }

    // ---- the hitbox visualizer's natives (SPEC-core-06 section 4; rows of `sites_hitbox`) -------------------------------

    /// Current combat ownership of a live item or weapon.
    pub unsafe fn team_owner_id(accessor: usize) -> Option<u32> {
        at::<extern "C" fn(usize) -> u32>(crate::model::sites_hitbox::HITBOX_TEAM_OWNER_ID_OFFSET).map(|call| call(accessor))
    }
    //
    // Each answers nothing (0, false) until its row was granted, like the rest of this layer.

    /// `app::sv_system::battle_object_module_accessor(lua_State*)`: the accessor of the object running an animation command, or 0.
    pub unsafe fn lua_accessor(lua_state: usize) -> usize {
        at::<extern "C" fn(usize) -> usize>(crate::model::sites_hitbox::HITBOX_ACCESSOR_OFFSET).map_or_else(Default::default, |f| f(lua_state))
    }

    /// `AttackModule::is_attack(accessor, slot, false)`.
    pub unsafe fn attack_is_attack(accessor: usize, slot: i32) -> bool {
        at::<extern "C" fn(usize, i32, bool) -> bool>(crate::model::sites_hitbox::HITBOX_IS_ATTACK_OFFSET).map_or_else(Default::default, |f| f(accessor, slot, false))
    }

    /// Number of normal AttackModule records on a live fighter accessor.
    pub unsafe fn fighter_attack_count(accessor: usize) -> Option<i32> {
        use crate::model::sites_mashout::{CLIMBER_ATTACK_COUNT, SITES};
        let read_count = at::<extern "C" fn(usize) -> u64>(CLIMBER_ATTACK_COUNT)?;
        let site = SITES.iter().find(|site| site.offset == CLIMBER_ATTACK_COUNT)?;
        if super::read_window(site.offset) != site.guard { return None; }
        let module = ((accessor + 0xa0) as *const usize).read();
        if module == 0 || !module.is_multiple_of(8)
            || (module as *const usize).read() != super::text_base() + 0x4f6a5b8
        {
            return None;
        }
        let count = read_count(module);
        (1..=16).contains(&count).then_some(count as i32)
    }

    /// `AttackModule::attack_data(accessor, slot, false)`: the slot's `AttackData` (layout in `model::hitbox::layout`), or 0.
    pub unsafe fn attack_data(accessor: usize, slot: i32) -> usize {
        at::<extern "C" fn(usize, i32, bool) -> usize>(crate::model::sites_hitbox::HITBOX_ATTACK_DATA_OFFSET).map_or_else(Default::default, |f| f(accessor, slot, false))
    }

    /// `AttackModule::power_mul_status(accessor)`: the module's status-scoped damage multiplier (module `+0xe0`): 1.0, or the smash charge the game's
    /// common script set (`sub_set_attack_s4_power_mul`); `get_power` multiplies every slot's damage by it. 0.0 until the row was granted, which the
    /// strip reads as a hit it cannot attribute.
    pub unsafe fn attack_power_mul(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(crate::model::sites_hitbox::HITBOX_POWER_MUL_OFFSET).map_or_else(Default::default, |f| f(accessor))
    }

    /// `DamageModule::reaction(accessor, 0)`: the knockback the fighter received, as the game's hit handler stored it for the strongest hit of
    /// the frame (spots-3). 0.0 until the row was granted, which the strip reads as no reading.
    pub unsafe fn damage_reaction(accessor: usize) -> f32 {
        at::<extern "C" fn(usize, i32) -> f32>(crate::model::sites_hitbox::HITBOX_DAMAGE_REACTION_OFFSET).map_or_else(Default::default, |f| f(accessor, 0))
    }

    /// The fighter's weight (`WorkModule::get_param_float(accessor, hash40("weight"), 0)`), 0.0 when the row was not granted.
    pub unsafe fn weight(accessor: usize) -> f32 {
        get_param_float(accessor, crate::model::cpu::hash::hash40("weight"), 0)
    }

    /// `EffectModule::req_follow(accessor, effect, joint, pos, rot, scale, tail.0, ..., tail.7)`: `tail` is the native's last eight
    /// arguments in the symbol's order (`bool, u32, i32, i32, i32, i32, bool, bool`; the second is the effect attribute, the fourth the
    /// lifetime). The effect's handle (an `unsigned int`, as `kill` and `is_exist_effect` take it), 0 when the game made none or the
    /// row was not granted. The one wrapper of this native (tests/native_wrappers.rs): the hitbox spheres pass
    /// `(true, attribute, 0, 0, 0, 0, true, true)` at their scale, the save states' charge effects `(false, attribute, 0, lifetime, 0,
    /// 0, false, false)` at scale 1.0. The save states' own hook of this native sees both (a pass-through for all but `sys_score_aura`).
    pub unsafe fn effect_req_follow(
        accessor: usize,
        effect: u64,
        joint: u64,
        pos: &[f32; 3],
        rot: &[f32; 3],
        scale: f32,
        tail: (bool, u32, i32, i32, i32, i32, bool, bool),
    ) -> u32 {
        type ReqFollow = extern "C" fn(usize, u64, u64, *const [f32; 3], *const [f32; 3], f32, bool, u32, i32, i32, i32, i32, bool, bool) -> u32;
        at::<ReqFollow>(crate::model::sites_hitbox::HITBOX_REQ_FOLLOW_OFFSET).map_or_else(Default::default, |f| {
            f(accessor, effect, joint, pos, rot, scale, tail.0, tail.1, tail.2, tail.3, tail.4, tail.5, tail.6, tail.7)
        })
    }

    /// `EffectModule::kill(accessor, handle, false, true)`: ends one effect by its handle (never by kind). The one wrapper of this
    /// native: the hitbox spheres and Min Min's arm effect in the save states.
    pub unsafe fn effect_kill(accessor: usize, handle: u32) {
        at::<extern "C" fn(usize, u32, bool, bool)>(crate::model::sites_hitbox::HITBOX_KILL_OFFSET).map_or_else(Default::default, |f| f(accessor, handle, false, true))
    }

    /// `ModelModule::joint_global_position_with_offset(accessor, joint, offset, out, false)`: the world position of `offset`
    /// in the bone's frame (`None` when the row was not granted). The guarded thunk (`main+0x2057640`) jumps to the
    /// ModelModule's vtable slot `0x158` with the flag masked to one bit (`and w4, w4, #1`). The plugin passes `false`, which
    /// is meant to read the joint matrix without updating the model first. That rests on the implementation behind slot
    /// `0x158`, not read yet: a follow-up Eden tracepoint on the thunk's `br x5` reads its address (review HB-3).
    pub unsafe fn joint_position(accessor: usize, joint: u64, offset: &[f32; 3]) -> Option<[f32; 3]> {
        type JointPosition = extern "C" fn(usize, u64, *const [f32; 4], *mut [f32; 4], bool);
        let local = Vector4 { v: [offset[0], offset[1], offset[2], 0.0] };
        let mut out = Vector4 { v: [0.0; 4] };
        at::<JointPosition>(crate::model::sites_hitbox::ORB_JOINT_POSITION_OFFSET).map(|f| {
            f(accessor, joint, &local.v, &mut out.v, false);
            [out.v[0], out.v[1], out.v[2]]
        })
    }

    /// The game's world-to-screen projection with the render camera (`(out_x*, out_y*, pos&, false)`): pixels, y down
    /// (`None` when the row was not granted).
    pub unsafe fn world_to_screen(pos: &[f32; 3]) -> Option<[f32; 2]> {
        type Project = extern "C" fn(*mut f32, *mut f32, *const [f32; 4], bool);
        let at_pos = Vector4 { v: [pos[0], pos[1], pos[2], 0.0] };
        let (mut x, mut y) = (0.0f32, 0.0f32);
        at::<Project>(crate::model::sites_hitbox::ORB_WORLD_TO_SCREEN_OFFSET).map(|f| {
            f(&mut x, &mut y, &at_pos.v, false);
            [x, y]
        })
    }

    /// The two screen size words that projection maps with (read only when its relied row was granted).
    pub unsafe fn screen_size_words() -> Option<[f32; 2]> {
        use crate::model::sites_hitbox::{ORB_SCREEN_H_WORD, ORB_SCREEN_SIZE_OFFSET, ORB_SCREEN_W_WORD};
        super::granted(ORB_SCREEN_SIZE_OFFSET).then(|| {
            [((text_base() + ORB_SCREEN_W_WORD) as *const f32).read_volatile(), ((text_base() + ORB_SCREEN_H_WORD) as *const f32).read_volatile()]
        })
    }

    /// The stage's camera limit range (left, right, top, bottom): the four floats `app::camera::get_camera_limit_range` returns from
    /// the camera manager (`**(main+0x52b8f00)`); `None` when its relied row was not granted or the manager is not built yet.
    pub unsafe fn camera_limit_range() -> Option<[f32; 4]> {
        use crate::model::sites_hitbox::{CAMERA_LIMIT_FIELD, CAMERA_LIMIT_RANGE_OFFSET};
        let holder = (text_base() + sites_pkg2::CAMERA_MANAGER_SLOT_OFFSET) as *const *const usize;
        let holder = super::granted(CAMERA_LIMIT_RANGE_OFFSET).then(|| holder.read_volatile())?;
        let manager = (!holder.is_null()).then(|| holder.read_volatile())?;
        (manager != 0).then(|| ((manager + CAMERA_LIMIT_FIELD) as *const [f32; 4]).read_unaligned())
    }

    /// A `phx::Vector3f` as the game reads it (16 bytes, 16-aligned).
    #[repr(C, align(16))]
    struct Vector4 {
        v: [f32; 4],
    }
}

/// The game's `app::smashball::is_training_mode()`; false when its guard did not pass.
pub fn is_training_mode() -> bool {
    if !TRAINING_PREDICATE_USABLE.load(Ordering::Acquire) || !granted(sites::IS_TRAINING_MODE_OFFSET) {
        return false;
    }
    // SAFETY: the site was guarded against the 13.0.5 bytes; the function takes no arguments and
    // returns a bool.
    let predicate: extern "C" fn() -> bool =
        unsafe { std::mem::transmute(text_base() + sites::IS_TRAINING_MODE_OFFSET) };
    predicate()
}

/// The game's `app::fighter::is_ready_go()`; false when its guard did not pass.
pub fn is_ready_go() -> bool {
    if !READY_GO_USABLE.load(Ordering::Acquire) || !granted(crate::model::sites_ui::UI_IS_READY_GO_OFFSET) {
        return false;
    }
    // SAFETY: the site was guarded against the 13.0.5 bytes; the function takes no arguments and returns a bool.
    let predicate: extern "C" fn() -> bool = unsafe { std::mem::transmute(text_base() + crate::model::sites_ui::UI_IS_READY_GO_OFFSET) };
    predicate()
}

pub fn allow_ready_go() {
    READY_GO_USABLE.store(true, Ordering::Release);
}

static READY_GO_USABLE: AtomicBool = AtomicBool::new(false);
