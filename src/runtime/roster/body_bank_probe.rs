//! Private, one-entry ordinary-body reuse experiment; excluded from release builds.
//!
//! R3+X in native Training as Mario builds a Mario/Link/Samus bank. Later presses
//! request a native member switch. This fixture is deliberately fixed and runs
//! once per process; it is not the production cache policy or a capacity setting.
//! No body, entry, resource owner or controller is moved between owners. Native
//! loader/destructor calls remain responsible for construction and destruction.
use super::{aligned, capture_reload_pose, native_fighter_install, native_source,
    read, ready_forms, BASE, INSTALL, InstallToken};
use crate::runtime::compat::common::{self, game_mode, input::MappedInputs, menu};
use crate::engine::seams::native_fields as fields;
use crate::engine::preload::preload_metrics::{self, MemorySample};
use skyline::hooks::InlineCtx;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use crate::engine::swap::body_bank::{Members, Phase, BANK_FLAG, NATIVE_MEMBERS};
use crate::engine::swap::character_forms::EntryState;

const KINDS: [u32; NATIVE_MEMBERS] = [0, 2, 3];
static ENABLED: AtomicBool = AtomicBool::new(false);
static ENTRY: AtomicUsize = AtomicUsize::new(0);
static HELD: AtomicBool = AtomicBool::new(false);
static REQUEST: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Body { address: usize, id: u32 }

struct Probe {
    phase: Phase,
    token: Option<InstallToken>,
    bodies: [Body; NATIVE_MEMBERS],
    from: u8,
    target: u8,
    started: u64,
    acknowledged: bool,
    wait_logged: bool,
}
static STATE: Mutex<Probe> = Mutex::new(Probe {
    phase: Phase::Idle, token: None, bodies: [Body { address: 0, id: 0 }; NATIVE_MEMBERS],
    from: 0, target: 0, started: 0, acknowledged: false, wait_logged: false,
});

macro_rules! trace {
    ($($arg:tt)*) => { crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}

fn members() -> Members { Members::new(&KINDS).expect("ordinary bank probe fixture") }

unsafe fn permitted() -> bool {
    ENABLED.load(Ordering::Acquire) && common::is_native_training_mode()
        && matches!(game_mode::current(), Some(mode)
            if mode.initialized && mode.melee == 0x20 && mode.mode == 0x04040000)
        && common::is_ready_go() && !menu::captures_input()
        && !crate::runtime::lobby::training::save_states::is_loading()
        && !crate::runtime::macros::replay_capture::loading() && !crate::runtime::compat::replay_preview::occupied()
}

pub unsafe fn handle_input(out: &mut MappedInputs, right: bool, x: bool) {
    let chord = right && x;
    let held = HELD.swap(chord, Ordering::Relaxed);
    let edge = chord && !held;
    if chord && STATE.lock().unwrap().phase.accepts_input() && permitted() {
        *out = MappedInputs::empty();
        if edge { REQUEST.store(true, Ordering::Release); }
    }
}

/// The native caller owns this entry for the duration of this callback. The
/// numeric receipt is only a match; the complete descriptor must also match.
unsafe fn scoped(entry: *const u8) -> bool {
    aligned(entry) && entry as usize == ENTRY.load(Ordering::Acquire)
        && read::<u32>(entry, 0xe0) == 0 && read::<u32>(entry, 0x14) == 3
        && read::<u32>(entry, 0x28) == 1
        && KINDS.iter().enumerate().all(|(i, kind)| read::<u32>(entry, 0x18 + i * 4) == *kind)
}

unsafe fn snapshot(entry: *const u8) -> EntryState {
    EntryState { count: read(entry, 0x14),
        kinds: [read(entry, 0x18), read(entry, 0x1c), read(entry, 0x20)],
        actor_count: read(entry, 0x28), installed: read(entry, 0x4150),
        index: read(entry, 0x5918), previous_or_next: read(entry, 0x5919),
        pending: read(entry, 0x591a), flags: read(entry, 0x591f),
        outer: read(entry, 0x5920), loader: read(entry, 0x591c),
        changing: read(entry, 0x5934), guard: read(entry, 0x5935) }
}

/// Current-entry borrow only; validate each child before reading required fields.
/// `active=None` requires ALL bodies parked; no timeout can replace that proof.
unsafe fn bodies(entry: *const u8, active: Option<u8>) -> Option<[Body; NATIVE_MEMBERS]> {
    let mut observed = [Body::default(); NATIVE_MEMBERS];
    for (i, kind) in KINDS.iter().enumerate() {
        let body = fields::child(entry as usize, 0x4160 + i * 8)?;
        if fields::read::<usize>(body, 0)? != BASE.load(Ordering::Acquire) + 0x4f80210
            || fields::read::<u32>(body, 0xc)? != *kind
            || fields::read::<u64>(body, 0x10)? != 0
            || fields::read::<u8>(body, 0x3a)? != if active == Some(i as u8) { 4 } else { 3 }
            || fields::child(body, 0x20).is_none()
        { return None; }
        observed[i] = Body { address: body, id: fields::read(body, 8)? };
    }
    Some(observed)
}

unsafe fn selected_context(entry: *const u8, target: u8) -> bool {
    let Some(context) = fields::child(entry as usize, 0x4178) else { return false; };
    Some(context) == fields::child(entry as usize, 0x4188 + usize::from(target) * 8)
        && fields::read::<u32>(context, 0x6c) == Some(0)
        && fields::read::<u32>(context, 0x74) == Some(KINDS[usize::from(target)])
}

/// Runs in the existing entry pre-service callback, never from input/render hooks.
/// Returns only from plugin work; native dispatch still runs exactly once.
pub unsafe fn service(entry: *mut u8, owner: u32) -> bool {
    if owner != 0 || !ENABLED.load(Ordering::Acquire) { return false; }
    let phase = STATE.lock().unwrap().phase;
    if phase == Phase::Idle {
        if !REQUEST.load(Ordering::Acquire) || !permitted() || INSTALL.active() { return false; }
        if ready_forms(entry, owner).is_none_or(|(forms, kind)| kind != 0 || forms.count != 1) {
            REQUEST.store(false, Ordering::Release);
            trace!("[BodyBankProbe] refused: start native Training as Mario; fixture [Mario,Link,Samus]");
            return false;
        }
        if super::STATES.iter().any(|s| { let s=s.lock().unwrap(); s.active.is_some() || s.pending.is_some() }) {
            return false;
        }
        let Some(resource) = native_source::resource_owner(0) else { return false; };
        let Some(pose) = capture_reload_pose(entry, owner) else { return false; };
        let Some(token) = INSTALL.try_begin(true) else { return false; };
        let mut descriptor = native_fighter_install::Descriptor::copy_from(entry);
        descriptor.request_group(native_fighter_install::Group { count: 3, kinds: KINDS, actor_count: 1 });
        {
            let mut state = STATE.lock().unwrap();
            state.phase = Phase::Building; state.token = Some(token);
            state.started = preload_metrics::ticks();
        }
        ENTRY.store(entry as usize, Ordering::Release);
        REQUEST.store(false, Ordering::Release);
        trace!("[BodyBankProbe] build entry={:p} resource={:p} memory={:?}", entry, resource, MemorySample::capture());
        // State/identities are published before native re-entry. No mutex spans it.
        super::reload_at_pose(BASE.load(Ordering::Acquire), entry, &descriptor, pose);
        return true;
    }
    if phase == Phase::Retired || entry as usize != ENTRY.load(Ordering::Acquire) { return false; }
    // Native construction may temporarily hold the old descriptor. Only observe
    // it here; the hook-specific admission never treats that as the bank.
    if !scoped(entry) { return true; }
    let observation = snapshot(entry);
    let bank = members();
    if matches!(phase, Phase::Building | Phase::Changing) {
        if let Some(index) = bank.ready_index(observation) {
            if let Some(now) = bodies(entry, Some(index)) {
                let mut state = STATE.lock().unwrap();
                if state.phase == Phase::Changing && (index != state.target || state.bodies != now) {
                    // Native readiness plus current-body proofs allow releasing
                    // our gate, but changed identities are a FAILED reuse probe.
                    // Do not quietly adopt replacements as a successful switch.
                    trace!("[BodyBankProbe] FAILED reuse: native ready with changed identity/target; index={} expected={} now={:?}; retiring probe", index, state.target, now);
                    state.phase = Phase::Retired;
                    REQUEST.store(false, Ordering::Release);
                    if let Some(token) = state.token.take() { INSTALL.finish(token); }
                    return true;
                }
                let previous = state.phase;
                let elapsed = preload_metrics::ticks().wrapping_sub(state.started);
                state.bodies = now; state.phase = Phase::Ready;
                REQUEST.store(false, Ordering::Release);
                if let Some(token) = state.token.take() { INSTALL.finish(token); }
                trace!("[BodyBankProbe] ready previous={:?} index={} kind={} elapsed_us={:?} bodies={:?} memory={:?}",
                    previous, index, KINDS[index as usize], preload_metrics::micros(elapsed, preload_metrics::tick_hz()), now, MemorySample::capture());
                return true;
            }
        }
        let mut state = STATE.lock().unwrap();
        if state.phase == Phase::Changing && !state.acknowledged
            && bank.awaiting_completion(observation, state.target)
            && read::<u8>(entry, 0x5910) == state.from
            && read::<u8>(entry, 0x5933) == 0
            && bodies(entry, None) == Some(state.bodies)
            && selected_context(entry, state.target)
        {
            // Ordinary fighters have no PT/Aegis completion script. All bodies
            // are parked, identities/owner unchanged, selected context valid.
            // Release only native's completion gate; retain its case12 resource
            // checks and case13 activation/event/restore ordering.
            fields::write_unchecked(entry as usize, 0x5933, 1u8);
            state.acknowledged = true;
            trace!("[BodyBankProbe] parked completion acknowledged from={} to={}", state.from, state.target);
        }
        if !state.wait_logged && preload_metrics::ticks().wrapping_sub(state.started) > preload_metrics::tick_hz() * 10 {
            state.wait_logged = true;
            trace!("[BodyBankProbe] stalled phase={:?} state={:?}; no forced advance or cleanup", state.phase, observation);
        }
        return true;
    }
    if phase == Phase::Ready && REQUEST.load(Ordering::Acquire) && permitted() {
        let Some(index) = bank.ready_index(observation) else { return true; };
        let Some(now) = bodies(entry, Some(index)) else { return true; };
        let Some(pose) = capture_reload_pose(entry, owner) else { return true; };
        let mut state = STATE.lock().unwrap();
        if now != state.bodies { return true; }
        let Some(token) = INSTALL.try_begin(true) else { return true; };
        state.phase = Phase::Changing; state.token = Some(token);
        state.from = index; state.target = (index + 1) % NATIVE_MEMBERS as u8;
        state.started = preload_metrics::ticks(); state.acknowledged = false; state.wait_logged = false;
        let target = state.target;
        drop(state);
        REQUEST.store(false, Ordering::Release);
        pose.stage();
        // Exactly the native request fields. The engine's FighterManager commits
        // the change; no manual pumping or direct activation call.
        fields::write_unchecked(entry as usize, 0x5932, 0u8);
        fields::write_unchecked(entry as usize, 0x591a, target);
        trace!("[BodyBankProbe] request from={} to={} bodies={:?}", index, target, now);
    }
    true
}

pub unsafe fn on_destroy(entry: *const u8) {
    if entry as usize != ENTRY.load(Ordering::Acquire) { return; }
    let mut state = STATE.lock().unwrap();
    state.phase = Phase::Retired;
    REQUEST.store(false, Ordering::Release);
    if let Some(token) = state.token.take() { INSTALL.finish(token); }
    // Keep the immutable receipt for any native callbacks during destruction.
    // Context init/free remain native; there is no private free. A reused address
    // alone never qualifies: scoped() requires this experiment's full descriptor.
    trace!("[BodyBankProbe] retired entry={:p}; native retains all destruction ownership", entry);
}

#[skyline::hook(offset = 0x654dc0)]
unsafe fn prepare_models(entry: *mut u8) {
    original!()(entry);
    if scoped(entry) { fields::write_unchecked(entry as usize, 0x591f, BANK_FLAG); }
}

unsafe fn table(ctx: &mut InlineCtx, register: usize) {
    if scoped(ctx.registers[19].x() as *const u8) {
        ctx.registers[register].set_x(KINDS.as_ptr() as u64);
    }
}
#[skyline::hook(offset = 0x655484, inline)]
unsafe fn create_contexts(ctx: &mut InlineCtx) { table(ctx, 23); }
#[skyline::hook(offset = 0x64fbcc, inline)]
unsafe fn select_context(ctx: &mut InlineCtx) { table(ctx, 9); }
// Native init/free select the three-slot PT table for ordinary kinds. They use
// it only as non-sentinel gates and null-check each context. Keep those paths
// native: they visit every allocated context even when a later bank is smaller.

#[skyline::hook(offset = 0x17e5bc0)]
unsafe fn prepare_resources(resource: *mut u8) {
    original!()(resource);
    if !aligned(resource) || ENTRY.load(Ordering::Acquire) == 0
        || read::<u32>(resource, 0x54) != 0
        || read::<u8>(resource, 0x4c) & 7 != 1
        || !KINDS.iter().enumerate().all(|(i, kind)| read::<u32>(resource, 0x58 + i * 4) == *kind)
    { return; }
    // Reload state6 can select a DIFFERENT free resource owner. Match the live
    // native association here, never the pre-reload R receipt. Read-only checks
    // run inside its native worker before it marks task/resource completion.
    let base = BASE.load(Ordering::Acquire);
    let Some(pool) = fields::child(base, 0x5324680) else { return; };
    if fields::read::<usize>(pool, 0xe8) != Some(resource as usize)
        || fields::read::<u8>(pool, 0x128) != Some(0) { return; }
    let acquire: unsafe extern "C" fn(*mut u8, u32) =
        std::mem::transmute(base + 0x17e64d0);
    for &kind in KINDS.iter().skip(1) { acquire(resource, kind); }
    trace!("[BodyBankProbe] additional resource members acquired resource={:p}", resource);
}

pub unsafe fn install(base: usize) {
    // Seam rows shared with the bank (`native_seams::rows_a1`); WP-A1 shadow: the words still decide.
    let legacy = [(0x654dc0, 0xd10103ffu32), (0x17e5bc0, 0xd105c3ff),
        (0x655484, 0xb94002f4), (0x64fbcc, 0xb940012a)].iter().all(|&(offset, word)| {
        let stock = fields::read_unchecked::<u32>(base, offset) == word;
        if !stock { trace!("[BodyBankProbe] fingerprint mismatch +0x{:x}; disabled", offset); }
        stock
    });
    if !crate::runtime::seams::seam_install::gate("body_bank_probe", &["body_bank.prepare_models",
        "body_bank.prepare_resources", "body_bank.create_contexts", "body_bank.select_context",
        "group_swap.acquire_kind"], legacy) {
        return;
    }
    skyline::install_hooks!(prepare_models, prepare_resources, create_contexts, select_context);
    ENABLED.store(true, Ordering::Release);
    trace!("[BodyBankProbe] installed private ordinary-bank experiment; Training Mario R3+X; not production cache policy");
}
