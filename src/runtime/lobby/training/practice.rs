//! Current native entry ownership for save/record/preview. No retained pointers
//! are dereferenced: each callback resolves the manager and current body again.
use hos_lock::Mutex;
use smash::{app::{self, lua_bind::*, BattleObjectModuleAccessor}, lib::lua_const::*};
use {crate::runtime::compat::common, crate::engine::seams::native_fields};
use super::practice_policy::{self, Context, Entries, EntryLease};

static OWNED: Mutex<EntryLease> = Mutex::new(EntryLease::new());

pub fn context() -> Option<Context> {
    let mode = common::game_mode::current();
    practice_policy::context(unsafe { common::is_native_training_mode() },
        mode.is_some_and(|m| m.initialized), mode.map_or(0, |m| m.melee),
        mode.map_or(0, |m| m.mode), super::lobby::active())
}

/// Fresh ownership context without waiting on a game-side menu edit.
/// Native Training does not depend on a waiting-room menu request.
pub fn try_context() -> Result<Option<Context>, crate::runtime::compat::menu::MenuBusy> {
    if unsafe { common::is_native_training_mode() } {
        return Ok(Some(Context::Training));
    }
    let mode = common::game_mode::current();
    Ok(practice_policy::context(false, mode.is_some_and(|m| m.initialized),
        mode.map_or(0, |m| m.melee), mode.map_or(0, |m| m.mode), super::lobby::try_active()?))
}

pub fn enabled() -> bool {
    enabled_in(context())
}
fn enabled_in(current_context: Option<Context>) -> bool {
    current_context.is_some() && !crate::runtime::roster::character_roster::busy() && !super::lobby::native_ui_busy()
}

pub unsafe fn entry(role: usize) -> Option<usize> {
    if !enabled() { return None; }
    let context = context()?;
    let index = if role == 0 { 0 } else { context.cpu_entry() };
    native_entry(index)
}

/// Native Training panels also hold extra CPUs. They have entry receipts for
/// releasing that stop, but never acquire either save/record slot.
pub unsafe fn panel_entry_in(current_context: Option<Context>, module: &BattleObjectModuleAccessor) -> Option<usize> {
    if !enabled_in(current_context) || !common::is_fighter(module) { return None; }
    let context = current_context?;
    let index = WorkModule::get_int(module as *const _ as *mut BattleObjectModuleAccessor,
        *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if !(0..8).contains(&index)
        || (context == Context::Waiting && context.role(index).is_none()) { return None; }
    native_entry(index)
}

unsafe fn native_entry(index: i32) -> Option<usize> {
    let manager = native_fields::child(crate::runtime::compat::sync::read(&common::FIGHTER_MANAGER_ADDR), 0)?;
    let entry = FighterManager::get_fighter_entry(manager as *mut app::FighterManager,
        app::FighterEntryID(index)) as usize;
    (native_fields::object(entry)
        && native_fields::read::<u32>(entry, 0xe0) == Some(index as u32)).then_some(entry)
}

unsafe fn entries(current_context: Option<Context>) -> Option<Entries> {
    if !enabled_in(current_context) { return None; }
    let context = current_context?;
    Some(Entries { context, player: native_entry(0)?, cpu: native_entry(context.cpu_entry())? })
}

pub unsafe fn acquire() -> bool {
    acquire_in(context())
}

/// Same lease acquisition using the caller's freshly resolved eligibility.
pub unsafe fn acquire_in(current_context: Option<Context>) -> bool {
    let Some(current) = entries(current_context) else { return false; };
    if common::try_get_module_accessor(crate::runtime::compat::consts::FighterId::Player).is_none()
        || common::try_get_module_accessor(crate::runtime::compat::consts::FighterId::CPU).is_none() { return false; }
    OWNED.lock().acquire(current)
}

pub fn forget() { OWNED.lock().retire(); }

/// Whether a practice session (the entry lease) is held now.
pub fn owned() -> bool { OWNED.lock().active() }

/// Invoked before consumer callbacks and before native entry destruction.
pub fn retire() {
    let owned = OWNED.lock().retire();
    if owned || crate::engine::rr_host::previewing() || super::recording_or_playback() {
        super::clear_session_state(false);
    }
}

/// Once per game frame, from the P1 body's frame callback: retire a lease whose
/// entries were replaced and resume suspended waiting drafts. The two native entry
/// resolutions below are the only per-frame cost; immediate retirement on entry
/// destruction or scene change does not wait for this.
pub unsafe fn refresh() {
    if !enabled() { return; }
    if !OWNED.lock().active() && !super::named_macros::suspended() { return; }
    let current = entries(context());
    let replaced = OWNED.lock().replaced(current);
    if replaced { retire(); }
    if current.is_some() { super::named_macros::resume_waiting(); }
}

pub fn entry_departed(entry: usize, owner: u32) {
    crate::runtime::features::cpu::entry_departed(owner);
    super::named_macros::forget_stopped_entry(entry);
    super::tech::forget_hidden_entry(owner as usize);
    let owned = OWNED.lock().owns(entry);
    if owned || owner == 0 {
        if super::lobby::is_waiting_scene() { super::named_macros::suspend_waiting(); }
        if owner == 0 { depart_session(); } else { retire(); }
    }
}

pub fn scene_departed() {
    super::named_macros::forget_stopped_scene();
    super::tech::forget_hidden_scene();
    depart_session();
}

fn depart_session() {
    practice_policy::depart_session(
        || { OWNED.lock().retire(); },
        || super::clear_session_state(false),
    );
}

pub unsafe fn role(module: &BattleObjectModuleAccessor) -> Option<usize> {
    role_in(context(), module)
}
pub unsafe fn role_in(current_context: Option<Context>, module: &BattleObjectModuleAccessor) -> Option<usize> {
    if !enabled_in(current_context) || !common::is_fighter(module) { return None; }
    current_context?.role(WorkModule::get_int(module as *const _ as *mut BattleObjectModuleAccessor,
        *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID))
}

/// The controller mapping that operates the Training CPU under native CPU
/// Behavior=Control: the CPU entry's own mapping index, only while a valid native
/// FighterInformation positively reports a controller operator. P1, waiting scenes,
/// an AI-owned CPU and every unqualified read refuse.
pub fn cpu_operator(player_idx: i32) -> bool {
    player_idx == crate::runtime::compat::consts::FighterId::CPU as i32
        && context() == Some(Context::Training)
        && common::try_get_module_accessor(crate::runtime::compat::consts::FighterId::CPU)
            .is_some_and(|cpu| common::operation_cpu(unsafe { &mut *cpu }) == Some(false))
}
