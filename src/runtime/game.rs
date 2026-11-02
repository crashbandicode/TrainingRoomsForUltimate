//! Reads of the running game: the text base, the version, the 64 bytes at a site, and the one game
//! function every feature calls (the Training-mode predicate).
//!
//! NOT YET SWITCH-COMPILED.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

use skyline::hooks::{getRegionAddress, Region};
use skyline::nn::oe::{DisplayVersion, GetDisplayVersion};

use crate::model::capabilities::Granted;
use crate::model::guard::GUARD_BYTES;
use crate::model::{lra_reset, sites, sites_pkg2, sites_pkg4, sites_pkg6};
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
    /// `fighter_entry`, `entry_body`, `module_accessor`: row `fighter_manager_get_entry`.
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
        lra_reset, sites, sites_pkg2, sites_pkg4, sites_pkg6, text_base, CATEGORY_NATIVES_USABLE, ENTRY_NATIVES_USABLE, OPERATOR_NATIVES_USABLE,
    };
    use crate::model::cpu::ids::UNRESOLVED;
    use crate::runtime::features::cpu;

    /// `FighterEntry` (the entry object): its first body (the fighter object) pointer.
    pub const ENTRY_BODY_FIELD: usize = crate::model::entry_cells::BODY_FIELD;
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

    pub unsafe fn set_float(accessor: usize, value: f32, id: i32) {
        at::<extern "C" fn(usize, f32, i32)>(sites_pkg2::WORK_SET_FLOAT_OFFSET).map_or_else(Default::default, |f| f(accessor, value, id))
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

    /// The entry object's first body (the fighter object), or 0.
    pub unsafe fn entry_body(entry_ptr: usize) -> usize {
        if entry_ptr == 0 {
            return 0;
        }
        ((entry_ptr + ENTRY_BODY_FIELD) as *const usize).read()
    }

    /// F11: the entry's running body (the fighter object), or 0: the window cell the game marks active (`+0x5918`, counted by
    /// `+0x4150`; `model::entry_cells::active_cell`), which a body-bank group swap or a native Pokemon Trainer switch changes, so it
    /// is the body the fighter actually is. Without the guarded layout (`sites_pkg6::entry_active_cell`, checked at boot) it is the
    /// first body, as `entry_body`. Game thread only: it reads the live entry; worker and menu threads read published facts.
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

    /// Whether the accessor belongs to a fighter (its battle object id is in the fighter category).
    pub unsafe fn is_fighter(accessor: usize) -> bool {
        if accessor == 0 || !CATEGORY_NATIVES_USABLE.load(Ordering::Acquire) {
            return false;
        }
        let id = ((accessor + ACCESSOR_OBJECT_ID_FIELD) as *const u32).read();
        at::<extern "C" fn(u32) -> i32>(sites_pkg2::UTILITY_GET_CATEGORY_OFFSET).map_or_else(Default::default, |f| f(id)) == CATEGORY_FIGHTER
    }

    /// `BattleObjectManager::is_active_find_battle_object(manager, object id)` (row `roster_find_battle_object`): the active battle
    /// object with `object_id`, 0 for an unknown or inactive id, a missing manager or a call that was not granted.
    pub unsafe fn find_battle_object(object_id: u32) -> usize {
        let Some(find) = at::<extern "C" fn(usize, u32) -> usize>(sites_pkg6::ROSTER_FIND_BATTLE_OBJECT_OFFSET) else {
            return 0;
        };
        let manager = ((text_base() + sites_pkg6::BATTLE_OBJECT_MANAGER_SLOT_OFFSET) as *const usize).read_volatile();
        if manager == 0 {
            return 0;
        }
        find(manager, object_id)
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
