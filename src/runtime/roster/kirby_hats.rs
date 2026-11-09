//! Shortcut-driven copy abilities. No game pointer is retained between callbacks.

use std::sync::atomic::{AtomicU8, Ordering};
use hos_lock::Mutex;

use smash::app::{self, lua_bind::*};
use smash::lib::lua_const::*;
use crate::runtime::compat::menu::MENU;
use crate::runtime::compat::sync::read;

use crate::runtime::compat::common::consts::FighterId;
use crate::runtime::compat::common::input::{Buttons, MappedInputs};
use crate::runtime::compat::common::{self, button_config, menu, try_get_module_accessor};
use crate::engine::kirby::kirby_hat_cycle::{
    commit_ends_request, hat_role, operator_clicks, request_frame, HatContinuation, HatCycle, HatSnapshot, ShortcutLatch,
};
use crate::runtime::lobby::training;
use crate::runtime::roster::character_roster::sources::{self, Lease};
use crate::engine::swap::source_lifetime::Holder;

#[derive(Clone, Copy)]
struct Pending {
    body_id: u32,
    kind: u32,
    age: u16,
    started: bool,
    cleared: bool,
    // Bookkeeping only; native copy state is revalidated on every retry.
    leases_released: bool,
    restoring: bool,
    lease: Option<Lease>,
}

struct State {
    cycle: HatCycle,
    pending: Option<Pending>,
    continuation: HatContinuation,
}

static STATES: [Mutex<State>; 2] = [
    Mutex::new(State {
        cycle: HatCycle::new(),
        pending: None,
        continuation: HatContinuation::new(),
    }),
    Mutex::new(State {
        cycle: HatCycle::new(),
        pending: None,
        continuation: HatContinuation::new(),
    }),
];
static SHORTCUT: Mutex<ShortcutLatch> = Mutex::new(ShortcutLatch::new());
/// The controller operating the CPU under native Behavior=Control has its own latch.
static OPERATOR_SHORTCUT: Mutex<ShortcutLatch> = Mutex::new(ShortcutLatch::new());
static PENDING_ROLES: AtomicU8 = AtomicU8::new(0);

fn clear_pending(role: usize, state: &mut State) {
    if let Some(pending) = state.pending {
        if let Some(lease) = pending.lease {
            if pending.started { sources::retain_hat(lease, pending.body_id); }
            else { sources::release(lease); }
        }
    }
    sources::cancel(role);
    state.pending = None;
    PENDING_ROLES.fetch_and(!(1 << role), Ordering::Release);
}

fn notice(role: usize, text: std::borrow::Cow<'static, str>) {
    training::ui::notifications::notification(["Kirby Hats", "CPU Kirby Hats"][role].into(), text, 120);
}

unsafe fn permitted() -> bool {
    common::is_training_mode()
        && common::is_ready_go()
        && !crate::runtime::roster::character_roster::busy()
        && !menu::captures_input()
        && !training::lobby::native_ui_busy()
        && !button_config::has_pending_request()
        && !training::save_states::is_loading()
        && !training::recording_or_playback()
        && !crate::runtime::compat::replay_preview::occupied()
}

unsafe fn role_body(role: usize) -> Option<*mut app::BattleObjectModuleAccessor> {
    let body = try_get_module_accessor(if role == 0 {
        FighterId::Player
    } else {
        FighterId::CPU
    })?;
    (app::utility::get_kind(&mut *body) == *FIGHTER_KIND_KIRBY).then_some(body)
}

/// The facts behind a failed `role_body`, read once at the failed chord for the one-line diagnostic
/// (glm-03: `role_body(1)` returned `None` for a CPU Kirby while the bank still called it ready, and
/// accessor-missing versus kind-mismatch was not split). `entry` is the id the lobby maps the role to,
/// `entry_ptr` the game's entry object, and `active_index`/`installed` the game's own active-cell
/// fields (`model::entry_cells` F11: `+0x5918`, `+0x4150`) at the moment of the read.
struct RoleLookup {
    accessor: Option<*mut app::BattleObjectModuleAccessor>,
    kind: Option<i32>,
    entry: i32,
    entry_ptr: usize,
    active_index: u8,
    installed: u64,
}

/// Never retained: the pointer is an equality/diagnostic token, read in the same callback.
unsafe fn role_lookup(role: usize) -> RoleLookup {
    let entry = crate::runtime::lobby::training::lobby::entry_id(if role == 0 {
        FighterId::Player
    } else {
        FighterId::CPU
    });
    let mut lookup = RoleLookup { accessor: None, kind: None, entry, entry_ptr: 0, active_index: 0, installed: 0 };
    let entry_ptr = crate::runtime::game::natives::fighter_entry(entry);
    if entry_ptr == 0 {
        return lookup;
    }
    lookup.entry_ptr = entry_ptr;
    lookup.installed = ((entry_ptr + 0x4150) as *const u64).read_volatile();
    lookup.active_index = ((entry_ptr + 0x5918) as *const u8).read_volatile();
    let accessor = crate::runtime::game::natives::fighter_accessor(
        crate::runtime::game::natives::entry_active_body(entry_ptr),
    );
    if accessor == 0 {
        return lookup;
    }
    let accessor = accessor as *mut app::BattleObjectModuleAccessor;
    lookup.kind = Some(app::utility::get_kind(&mut *accessor));
    lookup.accessor = Some(accessor);
    lookup
}

unsafe fn equipped(body: *mut app::BattleObjectModuleAccessor) -> Option<u32> {
    WorkModule::is_flag(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY)
        .then(|| WorkModule::get_int(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_INT_COPY_CHARA) as u32)
}

pub(crate) fn preload_pending() -> bool { PENDING_ROLES.load(Ordering::Acquire)!=0 }

/// Primary 2026-11-09 07:34Z (`character_roster::foreground`): a hat request waits for INSTALL alone while its source
/// load is queued (`sources::prepare` answered "not yet") and it holds no lease; before that it waits for its Kirby to
/// idle, after it for the native copy.
pub(crate) fn foreground_pending() -> bool {
    preload_pending() && STATES.iter().any(|s| s.lock().pending.is_some_and(|p| p.leases_released && p.lease.is_none() && !p.started))
}

/// Predict with a copy of the real hat cursor. No shortcut or equipped state changes.
pub(crate) unsafe fn preload_targets(menu: &crate::engine::compat::menu::TrainingModpackMenu,
    role: usize) -> [Option<crate::runtime::roster::cycle_preload::Target>;crate::runtime::roster::cycle_preload::MAX_TARGETS] {
    let Some(body)=role_body(role) else {return [None;crate::runtime::roster::cycle_preload::MAX_TARGETS];};
    let mut cycle={let state=STATES[role].lock();
        if state.pending.is_some() {return [None;crate::runtime::roster::cycle_preload::MAX_TARGETS];} state.cycle};
    let choices=menu.ordered_kirby_hat_kinds_fixed(if role==0 {
        crate::engine::swap::roster_grid::Role::Player
    } else {crate::engine::swap::roster_grid::Role::Cpu});
    let mut equipped=equipped(body);
    let mut targets=[None;crate::runtime::roster::cycle_preload::MAX_TARGETS];
    for target in &mut targets {
        let Some(kind)=cycle.target(&choices,equipped) else {break;};
        *target=sources::preload_hat_target(role,kind);
        cycle.commit(kind); equipped=Some(kind);
    }
    targets
}

/// Called only for P1, before the ordinary input-delay queue.
pub unsafe fn handle_input(out: &mut MappedInputs, left_click: bool, right_click: bool) {
    sample_shortcut(&SHORTCUT, out, left_click, right_click);
}

/// Called only for the controller operating the CPU under native Behavior=Control
/// (`practice::cpu_operator`). P1's CPU hat chord (Special plus right stick click), or
/// its own-fighter alias with the left stick, cycles the CPU's hat; never P1's.
pub unsafe fn handle_cpu_operator_input(out: &mut MappedInputs, left_click: bool, right_click: bool) {
    let (left, right) = operator_clicks(left_click, right_click);
    sample_shortcut(&OPERATOR_SHORTCUT, out, left, right);
}

/// `left_click` requests role 0 (P1) and `right_click` role 1 (CPU).
unsafe fn sample_shortcut(latch: &Mutex<ShortcutLatch>, out: &mut MappedInputs, left_click: bool, right_click: bool) {
    let special = out
        .buttons
        .intersects(Buttons::SPECIAL | Buttons::SPECIAL_RAW | Buttons::SPECIAL_RAW2);
    let neutral = out.lstick_x.unsigned_abs() < 20 && out.lstick_y.unsigned_abs() < 20;
    let mut shortcut = latch.lock();
    let mut bodies = [None; 2];
    if shortcut.would_request(special, neutral, left_click, right_click) && permitted() {
        let role = usize::from(right_click);
        bodies[role] = role_body(role);
        if bodies[role].is_none() {
            // The chord was eligible and permitted but the role had no Kirby body: say which case it
            // is (glm-03 follow-up), once per chord attempt (the latch edges this branch).
            let lookup = role_lookup(role);
            match lookup.accessor {
                Some(accessor) => crate::engine::compat::logging::diagnostic(format_args!(
                    "[KirbyHats] role_body role={} case=kind kind_seen=0x{:x} accessor=0x{:x} entry={} entry_ptr=0x{:x} active_index={} installed={}",
                    role, lookup.kind.unwrap_or(0) as u32, accessor as usize, lookup.entry,
                    lookup.entry_ptr, lookup.active_index, lookup.installed
                )),
                None => crate::engine::compat::logging::diagnostic(format_args!(
                    "[KirbyHats] role_body role={} case=accessor entry={} entry_ptr=0x{:x} active_index={} installed={}",
                    role, lookup.entry, lookup.entry_ptr, lookup.active_index, lookup.installed
                )),
            }
        }
    }
    let (request, suppress) = shortcut.sample(
        special,
        neutral,
        left_click,
        right_click,
        bodies.map(|body| body.is_some()),
    );
    drop(shortcut);
    if suppress {
        out.buttons
            .remove(Buttons::SPECIAL | Buttons::SPECIAL_RAW | Buttons::SPECIAL_RAW2);
    }
    let Some(role) = request else {
        return;
    };
    let body = bodies[role].expect("eligible Kirby body");
    let settings = read(&MENU);
    let choices = settings.ordered_kirby_hat_kinds_fixed(if role == 0 {
        crate::engine::swap::roster_grid::Role::Player
    } else {
        crate::engine::swap::roster_grid::Role::Cpu
    });
    let mut state = STATES[role].lock();
    if state.pending.is_some() {
        return;
    }
    let Some(kind) = state.cycle.target(&choices, equipped(body)) else {
        drop(state);
        notice(role, "Select a Kirby hat first".into());
        return;
    };
    state.pending = Some(Pending {
        body_id: (*body).battle_object_id,
        kind,
        age: 0,
        started: false,
        cleared: false,
        leases_released: false,
        restoring: false,
        lease: None,
    });
    PENDING_ROLES.fetch_or(1 << role, Ordering::Release);
    sources::pause_preload();
    if let Some(target)=sources::preload_hat_target(role,kind) {
        crate::runtime::roster::cycle_preload::request(role,true,target);
    }
}

/// Service from Kirby's native frame callback while that body is owned by the game: called once per Kirby with
/// that Kirby's own body (`features::states::state_kirby_opff_hook`, the fork's `SITE_KIRBY_OPFF` call), so the
/// CPU role's Kirby reaches its own pending request.
pub unsafe fn on_frame(body: *mut app::BattleObjectModuleAccessor) {
    let waiting = training::lobby::active() && training::lobby::is_waiting_scene();
    if (PENDING_ROLES.load(Ordering::Acquire) == 0 && !waiting)
        || body.is_null() || !common::is_training_mode() {
        return;
    }
    let Some(role) = hat_role(body, [role_body(0), role_body(1)]) else { return };
    // Resolve roster ownership before the hat mutex: roster completion can clear
    // this role's memory. Never restore onto the temporary character-select body.
    let settled = waiting && common::is_ready_go()
        && crate::runtime::roster::character_roster::waiting_choice_settled(role);
    let mut state = STATES[role].lock();
    if state.pending.is_none() && settled {
        if state.continuation.pending() {
            if !permitted() { return; }
            let snapshot = state.continuation.take_restore().expect("pending continuation");
            state.cycle = snapshot.cycle;
            if let Some(kind) = snapshot.equipped {
                state.pending = Some(Pending {
                    body_id: (*body).battle_object_id, kind,
                    age: 0, started: false, cleared: false, leases_released: false,
                    restoring: false, lease: None,
                });
                PENDING_ROLES.fetch_or(1 << role, Ordering::Release);
                crate::engine::compat::logging::diagnostic(format_args!(
                    "[KirbyHats] lobby continuation role={} kind=0x{:x} body=0x{:x}",
                    role, kind, (*body).battle_object_id));
            }
        } else if !WorkModule::is_flag(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY_ON_START) {
            let snapshot = HatSnapshot { cycle: state.cycle, equipped: equipped(body) };
            state.continuation.observe(snapshot);
        }
    }
    let Some(mut pending) = state.pending else {
        return;
    };
    if pending.restoring && pending.body_id == (*body).battle_object_id
        && training::save_states::is_loading() { return; }
    let identity_changed = pending.body_id != (*body).battle_object_id;
    if identity_changed || !permitted() {
        clear_pending(role, &mut state);
        return;
    }
    if pending.started
        && equipped(body) == Some(pending.kind)
        && !WorkModule::is_flag(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY_ON_START)
    {
        state.cycle.commit(pending.kind);
        if settled {
            let snapshot = HatSnapshot { cycle: state.cycle, equipped: Some(pending.kind) };
            state.continuation.observe(snapshot);
        }
        if let Some(lease) = pending.lease.take() {
            sources::retain_hat(lease, pending.body_id);
        }
        state.pending = Some(pending);
        clear_pending(role, &mut state);
        drop(state);
        crate::runtime::roster::cycle_preload::completed(role,true,pending.kind);
        crate::engine::compat::logging::diagnostic(format_args!(
            "[KirbyHats] confirmed role={} kind=0x{:x} body=0x{:x}",
            role, pending.kind, pending.body_id
        ));
        let name = crate::engine::swap::trio::TrioFighter::from_kind(pending.kind)
            .map(|f| f.name())
            .unwrap_or("Kirby hat");
        notice(role, name.into());
        return;
    }
    if !pending.started
        && !StopModule::is_stop(body)
        && !StatusModule::is_changing(body)
        && matches!(StatusModule::status_kind(body), kind if kind == *FIGHTER_STATUS_KIND_WAIT || kind == *FIGHTER_STATUS_KIND_FALL)
    {
        if cfg!(feature = "managed-sources") && !pending.cleared {
            pending.cleared = true;
            state.pending = Some(pending);
            drop(state);
            // No state mutex is held across native cleanup or its listeners.
            if let Err(reason) = sources::clear_hat_for_switch(body) {
                clear_pending(role, &mut STATES[role].lock());
                notice(role, reason.into());
            }
            return;
        }
        let slot = if cfg!(feature = "managed-sources") {
            if let Err(reason) = sources::finish_hat_clear(body, pending.leases_released) {
                clear_pending(role, &mut state);
                drop(state);
                notice(role, reason.into());
                return;
            }
            pending.leases_released = true;
            match sources::prepare(role, pending.kind,
                Holder { category: 1, id: u64::from(pending.body_id) }) {
                Ok(Some(lease)) => {
                    pending.lease = Some(lease);
                    Some(lease.slot as i32)
                }
                Ok(None) => { state.pending = Some(pending); return; }
                Err(reason) => {
                    clear_pending(role, &mut state);
                    drop(state);
                    notice(role, reason.into());
                    return;
                }
            }
        } else {
        let source = IntoIterator::into_iter([FighterId::Player, FighterId::CPU])
            .filter_map(try_get_module_accessor)
            .find(|source| app::utility::get_kind(&mut **source) as u32 == pending.kind);
        let slot = source.map(|source|
            WorkModule::get_int(source, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID));
        slot
        };
        let Some(slot) = slot else {
            clear_pending(role, &mut state);
            drop(state);
            notice(role, "This hat's resources are not loaded".into());
            return;
        };
        pending.started = true;
        state.pending = Some(pending);
        drop(state);
        crate::engine::compat::logging::diagnostic(format_args!(
            "[KirbyHats] request role={} kind=0x{:x} source_slot={} body=0x{:x}",
            role, pending.kind, slot, pending.body_id
        ));
        training::charge::setup_copy_ability(body, slot, pending.kind as i32);
        return;
    }
    state.pending = Some(pending);
}

/// HAT-REQUEST-END R1 (operator option A, 2026-11-05): once per game frame (Player 1's frame step), every pending
/// request ages whether or not its role's Kirby reached `on_frame`, so it always ends: confirmed there, or here with
/// "Kirby hat could not be loaded". A save state restoring the request holds its age, as `on_frame` holds it.
pub fn tick() {
    if PENDING_ROLES.load(Ordering::Acquire) == 0 {
        return;
    }
    for (role, state) in STATES.iter().enumerate() {
        let mut state = state.lock();
        let Some(pending) = state.pending.as_mut() else { continue };
        let expired;
        (pending.age, expired) = request_frame(pending.age, pending.restoring && training::save_states::is_loading());
        if expired {
            clear_pending(role, &mut state);
            drop(state);
            notice(role, "Kirby hat could not be loaded".into());
        }
    }
}

pub fn cancel_pending() {
    for (role, state) in STATES.iter().enumerate() {
        let mut state = state.lock();
        clear_pending(role, &mut state);
        drop(state);
    }
}

fn invalidate_runtime(same_lobby: Option<bool>) {
    for state in &STATES {
        let mut state = state.lock();
        state.pending = None;
        if let Some(same_lobby) = same_lobby {
            state.cycle = HatCycle::new();
            state.continuation.scene_changed(same_lobby);
        }
        drop(state);
    }
    *SHORTCUT.lock() = ShortcutLatch::default();
    *OPERATOR_SHORTCUT.lock() = ShortcutLatch::default();
    PENDING_ROLES.store(0, Ordering::Release);
}

/// Uses the roster's already classified scene boundary. Runtime leases/pointers
/// are always discarded, even when value-only hat intent survives a match.
pub fn scene_changed(same_lobby: bool) {
    invalidate_runtime(Some(same_lobby));
}

/// Entry destruction precedes knowing whether this is a match or a room exit.
pub fn practice_departed(waiting: bool) {
    invalidate_runtime(if waiting { None } else { Some(false) });
}

pub fn fresh_lobby() { invalidate_runtime(Some(false)); }

/// An accepted swap away from Kirby must not restore an obsolete hat later, and ends the role's pending request
/// (HAT-REQUEST-END R2, operator option A, 2026-11-05): its lease is released as `clear_pending` does.
pub fn fighter_committed(role: usize, kind: u32) {
    if !commit_ends_request(kind) { return; }
    let mut state = STATES[role].lock();
    state.continuation.clear();
    state.cycle = HatCycle::new();
    clear_pending(role, &mut state);
}

/// Restore uses the same asynchronous source acquisition as the shortcut. The
/// native save loader may finish first; an opponent slot is never hard-coded.
pub unsafe fn restore(body: *mut app::BattleObjectModuleAccessor, kind: u32) {
    let role = if role_body(0) == Some(body) { 0 }
        else if role_body(1) == Some(body) { 1 } else { return; };
    let mut state = STATES[role].lock();
    clear_pending(role, &mut state);
    state.pending = Some(Pending { body_id: (*body).battle_object_id, kind,
        age: 0, started: false, cleared: false, leases_released: false,
        restoring: true, lease: None,
    });
    PENDING_ROLES.fetch_or(1 << role, Ordering::Release);
}
