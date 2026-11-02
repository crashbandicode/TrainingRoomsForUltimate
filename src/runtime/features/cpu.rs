//! CPU behaviours, the runtime half (package 2): the glue between the game and `model::cpu`.
//!
//! Contract (see `model::cpu::state`): every hook first reads what it needs from the game with no lock
//! held (a game call can re-enter one of these hooks), then takes the one `CPU` lock for the pure
//! decision, releases it, and applies the answer. Nothing here allocates in a hook (the counting
//! allocator's `HookScope` wraps every entry point).
//!
//! * `on_frame` wraps the fighter vtable's on-frame slot: after the game's own frame it registers each
//!   CPU-role body (the role table maps a module accessor to the body's state), ticks the clock once
//!   per simulation frame from the primary body, runs `Cpu::frame` and applies its answer.
//! * The hooks (`cpu_sdi_hook`, `cpu_stick_x_hook`, `cpu_stick_y_hook`, `cpu_clatter_start_hook`,
//!   `cpu_transition_term_hook`) answer from the role table by accessor pointer and call the original
//!   for every body that is not a CPU role.
//! * The game's constants (status kinds, terms, work ids) are asked from the game's own table by name,
//!   once, on the first frame after the table exists (`ensure_consts`).
//!
//! NOT YET SWITCH-RUN: compiled for the Switch, not run on Eden or the console.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use parking_lot::Mutex;

use crate::model::cpu::defence::SdiAnswer;
use crate::model::cpu::facts::{FrameFacts, FrameOut, TechFacts};
use crate::model::cpu::shield::{ButtonAnswer, OosFacts};
use crate::model::cpu::tech::{is_tech_situation, TechChange, TechRequest};
use crate::model::cpu::status;
use crate::model::cpu::hash::hash40;
use crate::model::cpu::ids::{self, Table, C, UNRESOLVED};
use crate::model::cpu::role::{is_cpu_role, FighterKey, Scene, LOBBY_CPU_ENTRY};
use crate::model::cpu::settings::CpuSettings;
use crate::model::cpu::state::Cpu;
use crate::model::cpu::status::Situation;
use crate::model::lra_reset::{self, HubStep};
use crate::model::settings::Snapshot;
use crate::model::sites_pkg2;
use crate::model::storage::FixedPath;
use crate::runtime::alloc::HookScope;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::reset_hub;
use crate::runtime::game::{self, natives};

/// The CPU behaviours' state; `None` until `init`.
static CPU: Mutex<Option<Cpu>> = Mutex::new(None);

// The few resolved constants a hook compares before it takes the lock.
static CONSTS_READY: AtomicBool = AtomicBool::new(false);
static CONST_ATTEMPTS: AtomicU32 = AtomicU32::new(0);
static NANA_KIND: AtomicI32 = AtomicI32::new(UNRESOLVED);
static SITUATION_CLIFF: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_CLIMB: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_CLIFF_ATTACK: AtomicI32 = AtomicI32::new(UNRESOLVED);
static FLAG_CATCH_CLIFF: AtomicI32 = AtomicI32::new(UNRESOLVED);
static FLAG_DISABLE_PENALTY: AtomicI32 = AtomicI32::new(UNRESOLVED);
/// The three `sys_clatter` effect attributes OR-ed, or 0 when one did not resolve.
static CLATTER_EFFECT_ATTRIBUTES: AtomicU32 = AtomicU32::new(0);
static STATUS_LAY_DOWN: AtomicI32 = AtomicI32::new(UNRESOLVED);
static WORK_CURSOR_OFFSET: AtomicI32 = AtomicI32::new(UNRESOLVED);
static QUAKE_KIND_NONE: AtomicI32 = AtomicI32::new(UNRESOLVED);
static WORK_VECOR_X: AtomicI32 = AtomicI32::new(UNRESOLVED);
static WORK_VECOR_Y: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_PASSIVE: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_PASSIVE_WALL: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_PASSIVE_CEIL: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_CONT_ESCAPE: AtomicI32 = AtomicI32::new(UNRESOLVED);
/// `phx::Hash40` of `just_shield_off`, the parry animation.
static PARRY_MOTION: AtomicU64 = AtomicU64::new(0);
static BUTTON_GUARD: AtomicI32 = AtomicI32::new(UNRESOLVED);
static BUTTON_GUARD_HOLD: AtomicI32 = AtomicI32::new(UNRESOLVED);
/// `phx::Hash40` of `common`, `shield_dec1` and `shield_recovery1` (the parameter hook compares them).
static PARAM_HASHES: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];
/// SPEC-cpu-04's hooks and natives were installed (the frame wrapper reads the tech facts only then).
static TECH_ON: AtomicBool = AtomicBool::new(false);
/// The five tech effects hidden while a tech animation plays (`phx::Hash40`), set with the constants.
static TECH_EFFECTS: [AtomicU64; 5] = [const { AtomicU64::new(0) }; 5];
/// The player entry's last seen x (an `f32`'s bits), for the In / Out rule.
static PLAYER_X: AtomicU32 = AtomicU32::new(0);

/// The scene the behaviours are live in (decision B4; the waiting room is package 6's).
static LOBBY: AtomicBool = AtomicBool::new(false);

/// Frames to keep asking for the constants before giving up (the table is built early; a game whose
/// table never answers must not cost a call per frame for ever).
const CONST_ATTEMPT_LIMIT: u32 = 900;

/// The diag build counts how often each hook ran, so the per-frame budget is measured, not estimated
/// (docs/implementation-packages.md, package 2). The shipped build compiles all of this away.
#[cfg(feature = "diag")]
mod counts {
    use std::sync::atomic::{AtomicU32, Ordering};

    pub const FRAME: usize = 0;
    pub const SDI: usize = 1;
    pub const STICK_X: usize = 2;
    pub const STICK_Y: usize = 3;
    pub const CLATTER_START: usize = 4;
    pub const TRANSITION_TERM: usize = 5;
    pub const MOTION_CHANGE: usize = 6;
    pub const QUAKE_POS: usize = 7;
    static COUNTS: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
    /// One line per this many primary-body frames.
    const REPORT_EVERY: u32 = 600;

    pub fn bump(which: usize) {
        COUNTS[which].fetch_add(1, Ordering::Relaxed);
    }

    /// Called once per primary-body frame: write and clear the counts every `REPORT_EVERY` frames.
    pub fn report_if_due() {
        if COUNTS[FRAME].load(Ordering::Relaxed) < REPORT_EVERY {
            return;
        }
        let take = |i: usize| COUNTS[i].swap(0, Ordering::Relaxed);
        crate::runtime::diag_log::diagnostic(format_args!(
            "[TrainingRooms] cpu_hook_counts frames={} sdi={} stick_x={} stick_y={} clatter_start={} transition_term={} motion_change={} quake_pos={} heap_allocations={}",
            take(FRAME),
            take(SDI),
            take(STICK_X),
            take(STICK_Y),
            take(CLATTER_START),
            take(TRANSITION_TERM),
            take(MOTION_CHANGE),
            take(QUAKE_POS),
            crate::runtime::alloc::hook_allocations()
        ));
    }
}

/// The hook indices, the same in both builds (the shipped `bump` ignores them).
mod counts_index {
    pub const SDI: usize = 1;
    pub const STICK_X: usize = 2;
    pub const STICK_Y: usize = 3;
    pub const CLATTER_START: usize = 4;
    pub const TRANSITION_TERM: usize = 5;
    pub const MOTION_CHANGE: usize = 6;
    pub const QUAKE_POS: usize = 7;
}

#[cfg(feature = "diag")]
fn bump(which: usize) {
    counts::bump(which);
}

#[cfg(not(feature = "diag"))]
#[inline(always)]
fn bump(_which: usize) {}

fn with<R>(f: impl FnOnce(&mut Cpu) -> R) -> Option<R> {
    CPU.lock().as_mut().map(f)
}

/// Create the state (boot, before any hook is installed).
pub fn init(settings: CpuSettings, seed: u64) {
    *CPU.lock() = Some(Cpu::new(settings, seed));
}

/// A menu commit or the first publish: the new settings and the menu outbox's re-roll bits
/// (decisions Q18 A, B11).
pub fn publish(snapshot: &Snapshot, rerolls: u8) {
    let settings = CpuSettings::from_snapshot(snapshot);
    with(|cpu| cpu.publish(settings, rerolls));
}

/// Package 6 sets this when the waiting room's practice is active.
pub fn set_lobby(on: bool) {
    LOBBY.store(on, Ordering::Release);
    with(|cpu| cpu.scene = if on { Scene::Lobby } else { Scene::Training });
}

/// One step of the Training reset hub (`reset_hub::run`), for every body.
pub fn hub_step(step: HubStep) {
    with(|cpu| cpu.hub_step(step));
}

fn player_x() -> f32 {
    f32::from_bits(PLAYER_X.load(Ordering::Relaxed))
}

/// The module accessor's body, if it is a registered CPU role.
fn role_of(accessor: usize) -> Option<FighterKey> {
    CPU.lock().as_ref().and_then(|cpu| cpu.roles.find(accessor))
}

/// Ask the game's table for every constant, once. Returns whether the table is ready. Called from the
/// frame wrapper outside its `HookScope` (it copies a table, no heap, but the log line formats text).
fn ensure_consts() -> bool {
    if CONSTS_READY.load(Ordering::Acquire) {
        return true;
    }
    if CONST_ATTEMPTS.fetch_add(1, Ordering::Relaxed) >= CONST_ATTEMPT_LIMIT {
        return false;
    }
    let mut table = Table::unresolved();
    // SAFETY: only reached from the frame wrapper, installed after `lua_const_lookup` passed its guard.
    let missing = table.resolve(|key| unsafe { game::lua_const(key) });
    if missing == ids::COUNT {
        return false; // the game's table is not built yet; ask again next frame
    }
    NANA_KIND.store(table.get(C::KindNana), Ordering::Release);
    SITUATION_CLIFF.store(table.get(C::SituationCliff), Ordering::Release);
    TERM_CLIMB.store(table.get(C::TermContCliffClimb), Ordering::Release);
    TERM_CLIFF_ATTACK.store(table.get(C::TermContCliffAttack), Ordering::Release);
    FLAG_CATCH_CLIFF.store(table.get(C::WorkFlagCatchCliff), Ordering::Release);
    FLAG_DISABLE_PENALTY.store(table.get(C::WorkFlagDisableEscapePenalty), Ordering::Release);
    WORK_VECOR_X.store(table.get(C::WorkFloatVecorCorrectStickX), Ordering::Release);
    WORK_VECOR_Y.store(table.get(C::WorkFloatVecorCorrectStickY), Ordering::Release);
    TERM_PASSIVE.store(table.get(C::TermPassive), Ordering::Release);
    TERM_PASSIVE_WALL.store(table.get(C::TermPassiveWall), Ordering::Release);
    TERM_PASSIVE_CEIL.store(table.get(C::TermPassiveCeil), Ordering::Release);
    TERM_CONT_ESCAPE.store(table.get(C::TermContEscape), Ordering::Release);
    PARRY_MOTION.store(hash40("just_shield_off"), Ordering::Release);
    BUTTON_GUARD.store(table.get(C::ButtonGuard), Ordering::Release);
    BUTTON_GUARD_HOLD.store(table.get(C::ButtonGuardHold), Ordering::Release);
    for (slot, name) in PARAM_HASHES.iter().zip(["common", "shield_dec1", "shield_recovery1"]) {
        slot.store(hash40(name), Ordering::Release);
    }
    STATUS_LAY_DOWN.store(table.get(C::StatusLayDown), Ordering::Release);
    WORK_CURSOR_OFFSET.store(table.get(C::WorkFloatCursorOffsetY), Ordering::Release);
    QUAKE_KIND_NONE.store(table.get(C::QuakeKindNone), Ordering::Release);
    for (slot, name) in TECH_EFFECTS.iter().zip(["sys_nopassive", "sys_down_smoke", "sys_passive", "sys_crown", "sys_crown_collision"]) {
        slot.store(hash40(name), Ordering::Release);
    }
    let attributes = [C::EffectNoJointScale, C::EffectFollow, C::EffectConcludeStatus];
    if attributes.iter().all(|c| table.resolved(*c)) {
        CLATTER_EFFECT_ATTRIBUTES.store(attributes.iter().fold(0u32, |mask, c| mask | table.get(*c) as u32), Ordering::Release);
    }
    if missing > 0 {
        let mut names = FixedPath::<400>::new();
        for name in table.missing().take(6) {
            let _ = std::fmt::Write::write_fmt(&mut names, format_args!("{name} "));
        }
        diagnostic(format_args!("[TrainingRooms] cpu constants missing={missing}: {}", names.as_str()));
    }
    with(|cpu| cpu.ids = table);
    CONSTS_READY.store(true, Ordering::Release);
    true
}

// ---- the per-frame dispatch ---------------------------------------------------------------------------

static ORIGINAL_FRAME: AtomicUsize = AtomicUsize::new(0);

type OnFrame = extern "C" fn(usize, usize, usize, usize, usize, usize, usize, usize) -> usize;

/// Fighter object layout (read from the owned 13.0.5 `main`, `tests/cpu_sites.rs` and the on-init
/// notes): the fighter kind at +0xc, the entry id at +0x10, the module accessor pointer at +0x20.
const FIGHTER_KIND_FIELD: usize = 0xc;
const FIGHTER_ENTRY_FIELD: usize = lra_reset::FIGHTER_ENTRY_ID_FIELD;
const FIGHTER_ACCESSOR_FIELD: usize = 0x20;

extern "C" fn on_frame(a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize, a7: usize) -> usize {
    // SAFETY: `ORIGINAL_FRAME` holds the guarded original on-frame address (set by `install_frame`).
    let original: OnFrame = unsafe { std::mem::transmute(ORIGINAL_FRAME.load(Ordering::Acquire)) };
    let result = original(a0, a1, a2, a3, a4, a5, a6, a7);
    let ready = ensure_consts();
    let _scope = HookScope::enter();
    if ready {
        // SAFETY: `a0` is the fighter object the game just ran a frame of.
        unsafe { fighter_frame(a0) };
    }
    result
}

/// The entry whose first body is the primary CPU body (the clock ticks from it).
fn primary_entry(lobby: bool) -> u8 {
    if lobby {
        LOBBY_CPU_ENTRY as u8
    } else {
        1
    }
}

unsafe fn fighter_frame(fighter: usize) {
    if fighter == 0 {
        return;
    }
    let entry = ((fighter + FIGHTER_ENTRY_FIELD) as *const i32).read();
    let accessor = ((fighter + FIGHTER_ACCESSOR_FIELD) as *const usize).read();
    if accessor == 0 || !lra_reset::entry_id_valid(entry) {
        return;
    }
    let training = game::is_training_mode();
    if entry == 0 {
        // The player entry: remember where it stands (a body's In / Out is relative to it).
        if training {
            PLAYER_X.store(natives::pos_x(accessor).to_bits(), Ordering::Relaxed);
        }
        return;
    }
    if !training {
        with(|cpu| {
            if cpu.roles.any() {
                cpu.clear_session();
            }
        });
        return;
    }
    let lobby = LOBBY.load(Ordering::Acquire);
    let scene = if lobby { Scene::Lobby } else { Scene::Training };
    if !is_cpu_role(scene, entry, reset_hub::cpu_operated(fighter)) {
        with(|cpu| cpu.roles.forget(accessor));
        return;
    }
    let kind = ((fighter + FIGHTER_KIND_FIELD) as *const i32).read();
    let partner = NANA_KIND.load(Ordering::Acquire) == kind;
    let Some(key) = FighterKey::new(entry, partner) else { return };

    // Read the facts with no lock held.
    let status = natives::status_kind(accessor);
    let prev_status = natives::prev_status_kind(accessor, 0);
    let situation = natives::situation_kind(accessor);
    let prev_situation = natives::prev_situation_kind(accessor);
    let motion_frame = natives::motion_frame(accessor) as i32;
    let cpu_x = natives::pos_x(accessor);
    let hanging = situation == SITUATION_CLIFF.load(Ordering::Acquire);
    let (catch_cliff, cont_cliff_attack) = if hanging {
        let flag = FLAG_CATCH_CLIFF.load(Ordering::Acquire);
        let term = TERM_CLIFF_ATTACK.load(Ordering::Acquire);
        (
            flag != UNRESOLVED && natives::is_flag(accessor, flag),
            term != UNRESOLVED && natives::is_enable_transition_term(accessor, term),
        )
    } else {
        (false, false)
    };

    let primary = !key.partner && key.entry == primary_entry(lobby);
    let tech = if TECH_ON.load(Ordering::Acquire) { tech_facts(accessor, status, primary) } else { TechFacts::default() };

    let out = with(|cpu| {
        let facts = FrameFacts {
            status,
            prev_status,
            situation: Situation::of(&cpu.ids, situation),
            prev_situation: Situation::of(&cpu.ids, prev_situation),
            motion_frame,
            player_x: player_x(),
            cpu_x,
            catch_cliff,
            cont_cliff_attack,
            tech,
        };
        cpu.roles.set(accessor, key);
        if primary {
            cpu.tick_clock();
            #[cfg(feature = "diag")]
            {
                counts::bump(counts::FRAME);
            }
        }
        cpu.frame(key, &facts)
    });
    if let Some(out) = out {
        apply(accessor, out);
    }
    #[cfg(feature = "diag")]
    counts::report_if_due();
}

/// The facts only the tech specs read. The cursor offset is the primary body's; the lay-down lock-out
/// parameters are read only in that status.
unsafe fn tech_facts(accessor: usize, status: i32, primary: bool) -> TechFacts {
    let mut tech = TechFacts { end_frame: natives::motion_end_frame(accessor) as i32, ..TechFacts::default() };
    let cursor = WORK_CURSOR_OFFSET.load(Ordering::Acquire);
    if primary && cursor != UNRESOLVED {
        tech.cursor_offset = natives::get_float(accessor, cursor);
    }
    if status == STATUS_LAY_DOWN.load(Ordering::Acquire) {
        let common = hash40("common");
        tech.damage = natives::damage(accessor, 0);
        tech.laydown_base = natives::get_param_float(accessor, common, hash40("laydown_no_action_frame"));
        tech.laydown_max = natives::get_param_float(accessor, common, hash40("laydown_no_action_frame_max"));
        tech.laydown_damage_max = natives::get_param_float(accessor, common, hash40("laydown_damage_max"));
    }
    tech
}

/// Do what the frame asked, with no lock held.
unsafe fn apply(accessor: usize, out: FrameOut) {
    if let Some(value) = out.stale_flag {
        let flag = FLAG_DISABLE_PENALTY.load(Ordering::Acquire);
        if flag != UNRESOLVED {
            natives::set_flag(accessor, value, flag);
        }
    }
    if let Some(delta) = out.clatter {
        natives::add_clatter_time(accessor, delta);
        let attributes = CLATTER_EFFECT_ATTRIBUTES.load(Ordering::Acquire);
        if attributes != 0 {
            natives::req_on_joint(accessor, hash40("sys_clatter"), hash40("hip"), 1.0, attributes);
        }
    }
    if let Some(status) = out.request_status {
        natives::change_status_request(accessor, status);
    }
    if let Some(status) = out.force_status {
        natives::change_status_force(accessor, status);
    }
    let cursor = WORK_CURSOR_OFFSET.load(Ordering::Acquire);
    if let (Some(value), true) = (out.hide.cursor, cursor != UNRESOLVED) {
        natives::set_float(accessor, value, cursor);
    }
    if let Some(visible) = out.hide.model_visible {
        natives::set_whole(accessor, visible);
    }
    if out.hide.hide_effects {
        for effect in &TECH_EFFECTS {
            natives::set_visible_kind(accessor, effect.load(Ordering::Acquire), false);
        }
    }
    // `out.playback` waits for the input-recording package: there is no recorder to ask yet.
    #[cfg(feature = "diag")]
    if let Some(start) = out.playback {
        diagnostic(format_args!("[TrainingRooms] cpu playback request slot={} ledge={}", start.slot, start.ledge));
    }
}

/// Replace the on-frame vtable slot with the wrapper. Only after the slot and the original passed
/// their guards (`boot`).
pub fn install_frame() {
    let base = game::text_base();
    ORIGINAL_FRAME.store(base + sites_pkg2::CPU_ON_FRAME_OFFSET, Ordering::Release);
    let slot = (base + sites_pkg2::CPU_ON_FRAME_SLOT_OFFSET) as *mut usize;
    // SAFETY: the slot is a writable `.data` pointer that held the original's address.
    unsafe { slot.write_volatile(on_frame as *const () as usize) };
}

// ---- SPEC-cpu-02 hooks ----------------------------------------------------------------------------------

/// `FighterControlModuleImpl::check_hit_stop_delay_command(accessor, out)`: the SDI answer for a CPU
/// body (decision N9: unset means no SDI, the original is not called).
#[skyline::hook(offset = sites_pkg2::SDI_HOOK_OFFSET)]
pub unsafe fn cpu_sdi_hook(accessor: usize, out: *mut [f32; 2]) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::SDI);
    if let Some(key) = role_of(accessor) {
        let cpu_x = natives::pos_x(accessor);
        match with(|cpu| cpu.sdi_query(key, player_x(), cpu_x)) {
            Some(SdiAnswer::Input(x, y)) => {
                *out = [x, y];
                return 1;
            }
            Some(SdiAnswer::NoInput) => return 0,
            Some(SdiAnswer::Native) | None => {}
        }
    }
    call_original!(accessor, out)
}

fn air_dodge_stick(accessor: usize) -> Option<(f32, f32)> {
    let key = role_of(accessor)?;
    // SAFETY: `accessor` is a CPU body the frame wrapper registered; the native is a guarded site.
    let cpu_x = unsafe { natives::pos_x(accessor) };
    with(|cpu| cpu.air_dodge_stick(key, player_x(), cpu_x)).flatten()
}

/// `ControlModule::get_stick_x(accessor)`: the air-dodge direction out of hitstun.
#[skyline::hook(offset = sites_pkg2::STICK_X_HOOK_OFFSET)]
pub unsafe fn cpu_stick_x_hook(accessor: usize) -> f32 {
    let _scope = HookScope::enter();
    bump(counts_index::STICK_X);
    let original = call_original!(accessor);
    air_dodge_stick(accessor).map_or(original, |(x, _)| x)
}

/// `ControlModule::get_stick_y(accessor)`: the same pick as x for the episode. Crouch (SPEC-cpu-06)
/// answers after this one when it is built.
#[skyline::hook(offset = sites_pkg2::STICK_Y_HOOK_OFFSET)]
pub unsafe fn cpu_stick_y_hook(accessor: usize) -> f32 {
    let _scope = HookScope::enter();
    bump(counts_index::STICK_Y);
    let original = call_original!(accessor);
    air_dodge_stick(accessor).map_or(original, |(_, y)| y)
}

/// `ControlModule::start_clatter(accessor, initial, auto, manual, a, b, c, d)`: remember the manual
/// recovery rate as the clatter step; the original always runs.
#[skyline::hook(offset = sites_pkg2::CLATTER_START_HOOK_OFFSET)]
pub unsafe fn cpu_clatter_start_hook(
    accessor: usize,
    initial: f32,
    auto_rate: f32,
    manual_rate: f32,
    a: i8,
    b: i32,
    c: bool,
    d: bool,
) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::CLATTER_START);
    if let Some(key) = role_of(accessor) {
        with(|cpu| cpu.clatter_start(key, manual_rate));
    }
    call_original!(accessor, initial, auto_rate, manual_rate, a, b, c, d)
}

// ---- SPEC-cpu-03 hook -----------------------------------------------------------------------------------

/// `WorkModule::is_enable_transition_term(accessor, term)`: the native automatic ledge climb is denied
/// while a CPU body waits, delays or waits for its playback. Every other query is the original's.
#[skyline::hook(offset = sites_pkg2::TRANSITION_TERM_HOOK_OFFSET)]
pub unsafe fn cpu_transition_term_hook(accessor: usize, term: i32) -> bool {
    let _scope = HookScope::enter();
    bump(counts_index::TRANSITION_TERM);
    let original = call_original!(accessor, term);
    if !original || term != TERM_CLIMB.load(Ordering::Acquire) {
        return original;
    }
    let Some(key) = role_of(accessor) else { return original };
    let status = natives::status_kind(accessor);
    with(|cpu| cpu.ledge_transition_term(key, status, term, original)).unwrap_or(original)
}

// ---- SPEC-cpu-04 hooks ----------------------------------------------------------------------------------

/// Whether the SPEC-cpu-04 hooks and natives are installed (the frame wrapper reads their facts then).
pub fn set_tech_on(on: bool) {
    TECH_ON.store(on, Ordering::Release);
}

/// `MotionModule::change_motion(accessor, motion, ...)`: the tech and getup animations follow the held
/// roll directions (SPEC-cpu-04 section 5, "Motion").
#[skyline::hook(offset = sites_pkg2::MOTION_CHANGE_HOOK_OFFSET)]
pub unsafe fn cpu_motion_change_hook(accessor: usize, motion: u64, a: f32, b: f32, c: bool, d: f32, e: bool, f: bool) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::MOTION_CHANGE);
    let motion = role_of(accessor).and_then(|key| with(|cpu| cpu.substitute_motion(key, motion))).unwrap_or(motion);
    call_original!(accessor, motion, a, b, c, d, e, f)
}

/// `CameraModule::req_quake_pos(accessor, kind, pos)`: no shake for a CPU in the missed-tech status
/// while Hide Tech Animations is On (SPEC-cpu-04 section 4).
#[skyline::hook(offset = sites_pkg2::QUAKE_POS_HOOK_OFFSET)]
pub unsafe fn cpu_quake_pos_hook(accessor: usize, kind: i32, pos: *const [f32; 3]) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::QUAKE_POS);
    let none = QUAKE_KIND_NONE.load(Ordering::Acquire);
    let kind = match role_of(accessor) {
        Some(_) if none != UNRESOLVED => {
            let status = natives::status_kind(accessor);
            if with(|cpu| cpu.quake_suppressed(status)).unwrap_or(false) {
                none
            } else {
                kind
            }
        }
        _ => kind,
    };
    call_original!(accessor, kind, pos)
}

// ---- SPEC-cpu-05 hooks ----------------------------------------------------------------------------------

fn is_guard_button(button: i32) -> bool {
    button != UNRESOLVED && (button == BUTTON_GUARD.load(Ordering::Acquire) || button == BUTTON_GUARD_HOLD.load(Ordering::Acquire))
}

fn guard_button_answer(accessor: usize, button: i32, on_query: bool) -> Option<bool> {
    if !is_guard_button(button) {
        return None;
    }
    let key = role_of(accessor)?;
    match with(|cpu| cpu.guard_button(key, on_query))? {
        ButtonAnswer::Held(value) => Some(value),
        ButtonAnswer::Original => None,
    }
}

/// `ControlModule::check_button_on(accessor, button)`: the guard buttons read as held while a CPU
/// body should hold its shield.
#[skyline::hook(offset = sites_pkg2::CHECK_BUTTON_ON_HOOK_OFFSET)]
pub unsafe fn cpu_check_button_on_hook(accessor: usize, button: i32) -> bool {
    let _scope = HookScope::enter();
    let original = call_original!(accessor, button);
    guard_button_answer(accessor, button, true).unwrap_or(original)
}

/// `ControlModule::check_button_off(accessor, button)`: the guard buttons do not read as released.
#[skyline::hook(offset = sites_pkg2::CHECK_BUTTON_OFF_HOOK_OFFSET)]
pub unsafe fn cpu_check_button_off_hook(accessor: usize, button: i32) -> bool {
    let _scope = HookScope::enter();
    let original = call_original!(accessor, button);
    guard_button_answer(accessor, button, false).unwrap_or(original)
}

fn shield_tilt(accessor: usize) -> Option<(f32, f32)> {
    let key = role_of(accessor)?;
    // SAFETY: `accessor` is a CPU body the frame wrapper registered; the native is a guarded site.
    let cpu_x = unsafe { natives::pos_x(accessor) };
    with(|cpu| cpu.shield_tilt_stick(key, player_x(), cpu_x)).flatten()
}

/// `ControlModule::get_stick_x_no_clamp(accessor)`: the shield tilt.
#[skyline::hook(offset = sites_pkg2::STICK_X_NO_CLAMP_HOOK_OFFSET)]
pub unsafe fn cpu_stick_x_no_clamp_hook(accessor: usize) -> f32 {
    let _scope = HookScope::enter();
    let original = call_original!(accessor);
    shield_tilt(accessor).map_or(original, |(x, _)| x)
}

/// `ControlModule::get_stick_y_no_clamp(accessor)`: the shield tilt.
#[skyline::hook(offset = sites_pkg2::STICK_Y_NO_CLAMP_HOOK_OFFSET)]
pub unsafe fn cpu_stick_y_no_clamp_hook(accessor: usize) -> f32 {
    let _scope = HookScope::enter();
    let original = call_original!(accessor);
    shield_tilt(accessor).map_or(original, |(_, y)| y)
}

/// `WorkModule::get_param_float(accessor, kind, name)`: shield decay and regeneration for a CPU body
/// while the decay override is active; every other parameter is the original's (the first compare
/// is on the parameter type hash, so the common case costs two integer compares).
#[skyline::hook(offset = sites_pkg2::SHIELD_PARAM_HOOK_OFFSET)]
pub unsafe fn cpu_shield_param_hook(accessor: usize, kind: u64, name: u64) -> f32 {
    let _scope = HookScope::enter();
    let original = call_original!(accessor, kind, name);
    if kind != PARAM_HASHES[0].load(Ordering::Acquire) {
        return original;
    }
    let decay = name == PARAM_HASHES[1].load(Ordering::Acquire);
    if !decay && name != PARAM_HASHES[2].load(Ordering::Acquire) {
        return original;
    }
    role_of(accessor).and_then(|key| with(|cpu| cpu.shield_param(key, decay)).flatten()).unwrap_or(original)
}

// ---- the `common` script module's hooks (called from `cpu_common`) -------------------------------------

/// The module accessor of a script object (`L2CFighterBase`): the pointer at +0x40, which the owned
/// 13.0.5 `common` module loads from its first argument before every `lua_bind` call (checked on
/// `correctDamageVectorCommon` and `sub_guard_cont`). A pointer that is no registered CPU body does
/// nothing, so a wrong offset can only make the hooks inert.
pub(crate) unsafe fn script_accessor(fighter: usize) -> usize {
    if fighter == 0 {
        return 0;
    }
    ((fighter + 0x40) as *const usize).read()
}

/// DI: before the game corrects a damage vector, write the DI stick floats for a CPU body.
pub(crate) unsafe fn di_before(accessor: usize) {
    let Some(key) = role_of(accessor) else { return };
    let (x, y) = (WORK_VECOR_X.load(Ordering::Acquire), WORK_VECOR_Y.load(Ordering::Acquire));
    if x == UNRESOLVED || y == UNRESOLVED {
        return;
    }
    let cpu_x = natives::pos_x(accessor);
    if let Some(Some((vx, vy))) = with(|cpu| cpu.di_correction(key, player_x(), cpu_x)) {
        natives::set_float(accessor, vx, x);
        natives::set_float(accessor, vy, y);
    }
}

/// Tech: the status a CPU body's script is about to change to, or the one to request instead. `passive`
/// asks the script's own "is passive enabled".
pub(crate) unsafe fn tech_request(accessor: usize, requested: i32, passive: impl FnOnce() -> bool) -> Option<i32> {
    let key = role_of(accessor)?;
    if !with(|cpu| is_tech_situation(&cpu.ids, requested))? {
        return None;
    }
    let term = |id: &AtomicI32| {
        let id = id.load(Ordering::Acquire);
        id != UNRESOLVED && natives::is_enable_transition_term(accessor, id)
    };
    let request = TechRequest {
        requested,
        second_prev: natives::prev_status_kind(accessor, 1),
        passive: term(&TERM_PASSIVE),
        passive_wall: term(&TERM_PASSIVE_WALL),
        passive_ceil: term(&TERM_PASSIVE_CEIL),
        script_passive_enabled: passive(),
    };
    match with(|cpu| cpu.tech_status_change(key, &request))? {
        TechChange::Status(status) => Some(status),
        TechChange::Unchanged => None,
    }
}

/// Out of shield: after the script continuation, the dodge or roll status to change to, if any.
pub(crate) unsafe fn oos_after(accessor: usize) -> Option<i32> {
    let key = role_of(accessor)?;
    let prev = natives::prev_status_kind(accessor, 0);
    let status_now = natives::status_kind(accessor);
    let motion = natives::motion_kind(accessor);
    let term = TERM_CONT_ESCAPE.load(Ordering::Acquire);
    let cont_escape = term != UNRESOLVED && natives::is_enable_transition_term(accessor, term);
    let parry = PARRY_MOTION.load(Ordering::Acquire);
    with(|cpu| {
        let facts = OosFacts {
            left_shieldstun: cpu.ids.is(C::StatusGuardDamage, prev),
            shielding: status::shielding(&cpu.ids, status_now),
            in_parry: motion == parry,
            cont_escape,
        };
        cpu.out_of_shield(key, &facts).escape_status
    })
    .flatten()
}
