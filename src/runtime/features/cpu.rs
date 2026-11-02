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

use hos_lock::Mutex;

use crate::model::cpu::defence::SdiAnswer;
use crate::model::cpu::command::CmdFacts;
use crate::model::cpu::facts::{FrameFacts, FrameOut, TechFacts};
use crate::model::cpu::shield::{ButtonAnswer, OosFacts};
use crate::model::cpu::tech::{self, is_tech_situation, StageClass, TechChange, TechRequest};
use crate::model::cpu::status;
use crate::model::cpu::hash::hash40;
use crate::model::cpu::playback::PlaybackState;
use crate::model::cpu::ids::{self, Table, C, UNRESOLVED};
use crate::model::cpu::role::{is_cpu_role, FighterKey, Scene, LOBBY_CPU_ENTRY};
use crate::model::cpu::settings::CpuSettings;
use crate::model::cpu::state::Cpu;
use crate::model::cpu::status::Situation;
use crate::model::lra_reset::{self, HubStep};
use crate::model::settings::Snapshot;
use crate::model::sites_pkg2;
use crate::model::storage::FixedPath;
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{notices, recorder, reset_hub, states};
use crate::runtime::game::{self, natives};

/// The CPU behaviours' state; `None` until `init`.
static CPU: Mutex<Option<Cpu>> = Mutex::new(None);

// The few resolved constants a hook compares before it takes the lock.
static CONSTS_READY: AtomicBool = AtomicBool::new(false);
static CONST_ATTEMPTS: AtomicU32 = AtomicU32::new(0);
static NANA_KIND: AtomicI32 = AtomicI32::new(UNRESOLVED);
static SITUATION_CLIFF: AtomicI32 = AtomicI32::new(UNRESOLVED);
static SITUATION_GROUND: AtomicI32 = AtomicI32::new(UNRESOLVED);
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
static CAT1: AtomicI32 = AtomicI32::new(UNRESOLVED);
static CAT2: AtomicI32 = AtomicI32::new(UNRESOLVED);
static BUTTON_JUMP: AtomicI32 = AtomicI32::new(UNRESOLVED);
static BUTTON_FLICK_JUMP: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_CONT_DASH: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_CATCH_DASH: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_ATTACK_AIR: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_THROW_HI: AtomicI32 = AtomicI32::new(UNRESOLVED);
static WORK_INVALID_CATCH: AtomicI32 = AtomicI32::new(UNRESOLVED);
static WORK_RESERVE_DIVE: AtomicI32 = AtomicI32::new(UNRESOLVED);
static KINETIC_GRAVITY: AtomicI32 = AtomicI32::new(UNRESOLVED);
static EFFECT_CONCLUDE: AtomicI32 = AtomicI32::new(UNRESOLVED);
/// `phx::Hash40` of `dash`, `just_shield_off` (already above) and `sys_smash_flash_s`.
static DASH_MOTION: AtomicU64 = AtomicU64::new(0);
static FLASH_EFFECT: AtomicU64 = AtomicU64::new(0);
/// The mash hooks and natives were installed.
static TERM_ESCAPE_AIR: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_GUARD_ON: AtomicI32 = AtomicI32::new(UNRESOLVED);
static TERM_DOWN_STAND: AtomicI32 = AtomicI32::new(UNRESOLVED);
static MASH_ON: AtomicBool = AtomicBool::new(false);
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
static PLAYER_Y: AtomicU32 = AtomicU32::new(0);
static PLAYER_Z: AtomicU32 = AtomicU32::new(0);
/// The player entry has been seen this session (its position is valid for the distance triggers).
static PLAYER_VALID: AtomicBool = AtomicBool::new(false);

/// Diag: the save-state-loading fact the CPU last saw (logged on change).
#[cfg(feature = "diag")]
static LAST_SAVE_LOADING: AtomicBool = AtomicBool::new(false);

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
    pub const COMMAND_FLAG: usize = 8;
    pub const AIR_KIND: usize = 9;
    pub const STICK_DIR: usize = 10;
    static COUNTS: [AtomicU32; 11] = [const { AtomicU32::new(0) }; 11];
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
            "[TrainingRooms] cpu_hook_counts frames={} sdi={} stick_x={} stick_y={} clatter_start={} transition_term={} motion_change={} quake_pos={} command_flag={} air_kind={} stick_dir={} heap_allocations={}",
            take(FRAME),
            take(SDI),
            take(STICK_X),
            take(STICK_Y),
            take(CLATTER_START),
            take(TRANSITION_TERM),
            take(MOTION_CHANGE),
            take(QUAKE_POS),
            take(COMMAND_FLAG),
            take(AIR_KIND),
            take(STICK_DIR),
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
    pub const COMMAND_FLAG: usize = 8;
    pub const AIR_KIND: usize = 9;
    pub const STICK_DIR: usize = 10;
}

/// One diag-build line for an answer a behaviour gave (the shipped build compiles the arguments away).
/// The line is formatted on the caller's stack into the fixed ring: nothing allocates.
macro_rules! cpu_event {
    ($($arg:tt)*) => {{
        #[cfg(feature = "diag")]
        {
            diagnostic(format_args!("[TrainingRooms] cpu_event {}", format_args!($($arg)*)));
        }
    }};
}

/// Diag build: log an answer only when it differs from the last one logged for the same hook and body, so a
/// per-frame hook costs one line per change. `None` (the hook gave no answer) re-arms the line.
macro_rules! cpu_event_on_change {
    ($hook:expr, $body:expr, $value:expr, $($arg:tt)*) => {{
        #[cfg(feature = "diag")]
        {
            if seen($hook, $body, $value) {
                cpu_event!($($arg)*);
            }
        }
    }};
}

/// Diag build: the hook gave no answer for this body; the next answer is logged again.
macro_rules! cpu_event_rearm {
    ($hook:expr, $body:expr) => {{
        #[cfg(feature = "diag")]
        {
            let _ = seen($hook, $body, None);
        }
    }};
}

/// The hooks that log on change (an index into `LAST_SEEN`).
#[cfg(feature = "diag")]
mod ev {
    pub const AIRDODGE: usize = 0;
    pub const SDI_NONE: usize = 1;
    pub const TECH: usize = 2;
    pub const CMD1: usize = 4;
    pub const CMD2: usize = 5;
    pub const AIR_KIND: usize = 6;
    pub const STICK_DIR: usize = 7;
    pub const CROUCH: usize = 8;
    pub const GUARD_ON: usize = 9;
    pub const GUARD_OFF: usize = 10;
    pub const FULL_HOP: usize = 11;
    pub const TILT: usize = 12;
    pub const SHIELD_PARAM: usize = 13;
    pub const CLIMB_DENIED: usize = 14;
    pub const STATUS: usize = 15;
    pub const ROLE: usize = 16;
    pub const DI_CALL: usize = 17;
    pub const TECH_CALL: usize = 18;
    pub const OOS_CALL: usize = 19;
    pub const COUNT: usize = 20;
}

#[cfg(feature = "diag")]
static LAST_SEEN: [[AtomicU64; 16]; ev::COUNT] = [const { [const { AtomicU64::new(u64::MAX) }; 16] }; ev::COUNT];

/// True when `value` is an answer and differs from the one logged last for (`hook`, `body`).
#[cfg(feature = "diag")]
fn seen(hook: usize, body: usize, value: Option<u64>) -> bool {
    let new = value.unwrap_or(u64::MAX);
    let old = LAST_SEEN[hook][body & 15].swap(new, Ordering::Relaxed);
    value.is_some() && old != new
}

#[cfg(feature = "diag")]
fn bump(which: usize) {
    counts::bump(which);
}

#[cfg(not(feature = "diag"))]
#[inline(always)]
fn bump(_which: usize) {}

/// The last published settings generation the CPU adopted (only read and written under `CPU`).
static ADOPTED: AtomicU32 = AtomicU32::new(0);

/// Run `f` on the CPU state; a newly published value (`published::Live`) is adopted first. `CPU` is taken by game threads only
/// (stream/sync): the menu worker publishes through `published`, never through this lock.
fn with<R>(f: impl FnOnce(&mut Cpu) -> R) -> Option<R> {
    let mut guard = CPU.lock();
    let cpu = guard.as_mut()?;
    if let Some(live) = crate::runtime::published::live_if_newer(&ADOPTED) {
        cpu.publish(live.cpu, crate::runtime::published::take_rerolls());
    }
    Some(f(cpu))
}

/// Create the state (boot, before any hook is installed).
pub fn init(settings: CpuSettings, seed: u64) {
    *CPU.lock() = Some(Cpu::new(settings, seed));
}

/// The CPU takes the choices of the `reroll::*` bits again (package 6: the waiting room's practice start re-rolls every cached choice
/// from the live menu).
pub fn request_rerolls(rerolls: u8) {
    with(|cpu| cpu.request_rerolls(rerolls));
}

/// A practice body departed (package 6): its hidden-tech record is dropped.
pub fn forget_hidden_entry(entry: usize) {
    with(|cpu| cpu.forget_hidden_entry(entry));
}

/// The practice scene ended (package 6): no hidden-tech record survives it.
pub fn forget_hidden_scene() {
    with(Cpu::forget_hidden);
}
/// Package 6 sets this when the waiting room's practice is active.
pub fn set_lobby(on: bool) {
    LOBBY.store(on, Ordering::Release);
    crate::runtime::scene::set_lobby_active(on);
    with(|cpu| cpu.scene = if on { Scene::Lobby } else { Scene::Training });
}

/// One step of the Training reset hub (`reset_hub::run`), for every body.
pub fn hub_step(step: HubStep) {
    with(|cpu| cpu.hub_step(step));
}

fn player_pos() -> (f32, f32, f32) {
    (
        f32::from_bits(PLAYER_X.load(Ordering::Relaxed)),
        f32::from_bits(PLAYER_Y.load(Ordering::Relaxed)),
        f32::from_bits(PLAYER_Z.load(Ordering::Relaxed)),
    )
}

fn player_pos_valid() -> bool {
    PLAYER_VALID.load(Ordering::Relaxed)
}

fn player_x() -> f32 {
    f32::from_bits(PLAYER_X.load(Ordering::Relaxed))
}

/// The constant table once the game's table has answered (package 4's recorder reads its own ids).
pub fn ids() -> Option<Table> {
    if !CONSTS_READY.load(Ordering::Acquire) {
        return None;
    }
    CPU.lock().as_ref().map(|cpu| cpu.ids.clone())
}

/// `SITUATION_KIND_GROUND` once the game's constants are resolved, else `UNRESOLVED`.
pub fn situation_ground() -> i32 {
    SITUATION_GROUND.load(Ordering::Acquire)
}

/// The entry of the primary CPU body for the current scene.
pub fn primary_entry_now() -> u8 {
    primary_entry(LOBBY.load(Ordering::Acquire))
}

/// The playback slot (0..=4) at the head of body key's mash queue, when the head is a Playback Slot.
pub fn mash_head(key: FighterKey) -> Option<u8> {
    with(|cpu| cpu.fighters[key.index()].mash.queue.current().and_then(|a| a.playback_slot())).flatten()
}

/// The recorder is done with the head (the playback it asked for ended or was refused).
pub fn mash_pop(key: FighterKey) {
    with(|cpu| {
        cpu.fighters[key.index()].mash.queue.complete();
    });
}

/// The recorder's state, for the CPU specs that yield to a playback (decisions B5 / N4).
pub fn set_playback(state: PlaybackState) {
    with(|cpu| cpu.playback = state);
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
    SITUATION_GROUND.store(table.get(C::SituationGround), Ordering::Release);
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
    TERM_ESCAPE_AIR.store(table.get(C::TermContEscapeAir), Ordering::Release);
    TERM_GUARD_ON.store(table.get(C::TermContGuardOn), Ordering::Release);
    TERM_DOWN_STAND.store(table.get(C::TermDownStand), Ordering::Release);
    CAT1.store(table.get(C::CatCommand1), Ordering::Release);
    CAT2.store(table.get(C::CatCommand2), Ordering::Release);
    BUTTON_JUMP.store(table.get(C::ButtonJump), Ordering::Release);
    BUTTON_FLICK_JUMP.store(table.get(C::ButtonFlickJump), Ordering::Release);
    TERM_CONT_DASH.store(table.get(C::TermContDash), Ordering::Release);
    TERM_CATCH_DASH.store(table.get(C::TermContCatchDash), Ordering::Release);
    TERM_ATTACK_AIR.store(table.get(C::TermContAttackAir), Ordering::Release);
    TERM_THROW_HI.store(table.get(C::TermContThrowHi), Ordering::Release);
    WORK_INVALID_CATCH.store(table.get(C::WorkIntInvalidCatchFrame), Ordering::Release);
    WORK_RESERVE_DIVE.store(table.get(C::WorkFlagReserveDive), Ordering::Release);
    KINETIC_GRAVITY.store(table.get(C::KineticGravity), Ordering::Release);
    EFFECT_CONCLUDE.store(table.get(C::EffectConcludeStatus), Ordering::Release);
    DASH_MOTION.store(hash40("dash"), Ordering::Release);
    FLASH_EFFECT.store(hash40("sys_smash_flash_s"), Ordering::Release);
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
    #[cfg(feature = "diag")]
    {
        // The resolved table by name, so a log's status, term and work ids can be read by name.
        let mut line = FixedPath::<700>::new();
        let mut count = 0;
        for (name, value) in table.resolved_entries() {
            let _ = std::fmt::Write::write_fmt(&mut line, format_args!("{name}={value} "));
            count += 1;
            if count % 6 == 0 {
                diagnostic(format_args!("[TrainingRooms] cpu_ids {}", line.as_str()));
                line = FixedPath::new();
            }
        }
        if count % 6 != 0 {
            diagnostic(format_args!("[TrainingRooms] cpu_ids {}", line.as_str()));
        }
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
    // Package 4, S4-4: the capture observers and the clip preview see the state the game is about to advance.
    unsafe { crate::runtime::macros::frame_before(a0) };
    let result = original(a0, a1, a2, a3, a4, a5, a6, a7);
    unsafe { crate::runtime::macros::frame_after(a0) };
    let ready = ensure_consts();
    let _scope = HookScope::enter_hook(HookId::CpuFrame);
    if ready {
        // SAFETY: `a0` is the fighter object the game just ran a frame of.
        unsafe { fighter_frame(a0) };
        // Package 3: the save-state step runs on the same per-fighter callback, after the CPU's.
        crate::runtime::features::states::frame(a0);
        // Package 6: the CPU-as-P1 fallback's step (nothing when the pilot is absent-and-not-installed or present).
        unsafe { crate::runtime::p1::frame(a0) };
        // Package 6: Player 1's body refreshes the waiting-room practice entry lease, once per game frame.
        unsafe { crate::runtime::lobby::frame(a0) };
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
    // The shared scene wrapper (pkg6 D4): native Training or an active lobby practice, never during a hot swap.
    let scene_now = crate::runtime::scene::current();
    let training = scene_now.is_some();
    if entry == 0 {
        // Also outside Training: leaving it restores the game's multiplier.
        damage_multiplier_frame(training);
        // The player entry: remember where it stands (a body's In / Out is relative to it).
        if training {
            frame_advantage(accessor);
            PLAYER_X.store(natives::pos_x(accessor).to_bits(), Ordering::Relaxed);
            mash_player_position(accessor);
        }
        return;
    }
    if !training {
        let cleared = with(|cpu| {
            let any = cpu.roles.any();
            if any {
                cpu.clear_session();
            }
            any
        });
        // Package 4's session state (recorder, chords, notices) goes with it; outside the CPU lock.
        if cleared == Some(true) {
            recorder::session_clear();
        }
        return;
    }
    let scene = scene_now.unwrap_or(Scene::Training);
    let lobby = scene == Scene::Lobby;
    if !is_cpu_role(scene, entry, reset_hub::cpu_operated(fighter)) {
        with(|cpu| cpu.roles.forget(accessor));
        return;
    }
    let kind = ((fighter + FIGHTER_KIND_FIELD) as *const i32).read();
    let partner = NANA_KIND.load(Ordering::Acquire) == kind;
    let Some(key) = FighterKey::new(entry, partner) else { return };

    // Read the facts with no lock held.
    let save_loading = states::is_loading();
    let status = natives::status_kind(accessor);
    let prev_status = natives::prev_status_kind(accessor, 0);
    let situation = natives::situation_kind(accessor);
    let prev_situation = natives::prev_situation_kind(accessor);
    let motion_frame = natives::motion_frame(accessor) as i32;
    let cpu_x = natives::pos_x(accessor);
    let (catch_cliff, cont_cliff_attack) = ledge_facts(accessor, situation);
    let (falling, reserve_dive) = mash_facts(accessor);
    let primary = !key.partner && key.entry == primary_entry(lobby);
    let tech = tech_facts(accessor, status, primary);

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
            falling,
            reserve_dive,
        };
        cpu.roles.set(accessor, key);
        cpu.fighters[key.index()].kind = kind;
        // A save state loading is package 3's fact (`states::is_loading`, the load machine's own flag).
        cpu.save_loading = save_loading;
        #[cfg(feature = "diag")]
        if LAST_SAVE_LOADING.swap(save_loading, Ordering::AcqRel) != save_loading {
            cpu_event!("save_loading={save_loading}");
        }
        if primary {
            cpu.tick_clock();
            #[cfg(feature = "diag")]
            {
                counts::bump(counts::FRAME);
            }
        }
        cpu.frame(key, &facts)
    });
    cpu_event_on_change!(
        ev::ROLE,
        key.index(),
        Some(accessor as u64),
        "role entry={} partner={} kind={kind} lobby={lobby} accessor={accessor:#x}",
        key.entry,
        key.partner
    );
    cpu_event_on_change!(
        ev::STATUS,
        key.index(),
        Some((u64::from(status as u32) << 32) | u64::from(situation as u32)),
        "status entry={} partner={} status={status} prev={prev_status} situation={situation} motion_frame={motion_frame}",
        key.entry,
        key.partner
    );
    if let Some(out) = out {
        apply(accessor, out);
    }
    #[cfg(feature = "diag")]
    counts::report_if_due();
}

// ---- the per-feature facts and appliers of the frame wrapper (R2-1) -----------------------------------------
//
// The frame wrapper belongs to the CPU core and reads only the core's natives itself. Everything another
// feature needs (the ledge, mash, tech and defence natives) is in a function that checks its feature's own
// install flag first, so a feature whose rows were rejected is never reached from the wrapper
// (`model::capabilities`: these functions are the declared boundaries of `tests/capabilities.rs`).

/// SPEC-cpu-03's hook was installed (the wrapper reads the ledge facts only then).
static LEDGE_ON: AtomicBool = AtomicBool::new(false);
/// SPEC-cpu-02's hooks and natives were installed (the wrapper applies the clatter only then).
static DEFENCE_ON: AtomicBool = AtomicBool::new(false);

pub fn set_ledge_on(on: bool) {
    LEDGE_ON.store(on, Ordering::Release);
}

pub fn set_defence_on(on: bool) {
    DEFENCE_ON.store(on, Ordering::Release);
}

/// The ledge facts: a hanging fighter's catch-cliff flag and whether the cliff attack transition is open.
unsafe fn ledge_facts(accessor: usize, situation: i32) -> (bool, bool) {
    if !LEDGE_ON.load(Ordering::Acquire) || situation != SITUATION_CLIFF.load(Ordering::Acquire) {
        return (false, false);
    }
    let flag = FLAG_CATCH_CLIFF.load(Ordering::Acquire);
    let term = TERM_CLIFF_ATTACK.load(Ordering::Acquire);
    (
        flag != UNRESOLVED && natives::is_flag(accessor, flag),
        term != UNRESOLVED && natives::is_enable_transition_term(accessor, term),
    )
}

/// The mash's facts: whether the fighter is falling and has reserved a dive.
unsafe fn mash_facts(accessor: usize) -> (bool, bool) {
    if !MASH_ON.load(Ordering::Acquire) {
        return (false, false);
    }
    let gravity = KINETIC_GRAVITY.load(Ordering::Acquire);
    let dive = WORK_RESERVE_DIVE.load(Ordering::Acquire);
    (
        gravity != UNRESOLVED && natives::sum_speed_y(accessor, gravity) < 0.0,
        dive != UNRESOLVED && natives::is_flag(accessor, dive),
    )
}

/// The player's y and z for the mash's distance triggers.
unsafe fn mash_player_position(accessor: usize) {
    if !MASH_ON.load(Ordering::Acquire) {
        return;
    }
    PLAYER_Y.store(natives::pos_y(accessor).to_bits(), Ordering::Relaxed);
    PLAYER_Z.store(natives::pos_z(accessor).to_bits(), Ordering::Relaxed);
    PLAYER_VALID.store(true, Ordering::Relaxed);
}

/// The facts only the tech specs read. The cursor offset is the primary body's; the lay-down lock-out
/// parameters are read only in that status.
unsafe fn tech_facts(accessor: usize, status: i32, primary: bool) -> TechFacts {
    if !TECH_ON.load(Ordering::Acquire) {
        return TechFacts::default();
    }
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

/// Do what the frame asked, with no lock held: the core's own requests, then each feature's, each behind its
/// own install flag.
unsafe fn apply(accessor: usize, out: FrameOut) {
    if let Some(value) = out.stale_flag {
        let flag = FLAG_DISABLE_PENALTY.load(Ordering::Acquire);
        if flag != UNRESOLVED {
            natives::set_flag(accessor, value, flag);
        }
        cpu_event!("stale_flag value={value} resolved={}", flag != UNRESOLVED);
    }
    apply_defence(accessor, &out);
    if let Some(status) = out.request_status {
        cpu_event!("request_status status={status} from={}", natives::status_kind(accessor));
        natives::change_status_request(accessor, status);
    }
    apply_mash(accessor, &out);
    apply_tech(accessor, &out);
    // A playback request goes straight to the recorder (package 4's `request_playback`): the one path.
    if let Some(start) = out.playback {
        recorder::request_playback(start);
        cpu_event!("playback_request slot={} ledge={}", start.slot, start.ledge);
    }
}

/// SPEC-cpu-02: the clatter.
unsafe fn apply_defence(accessor: usize, out: &FrameOut) {
    if !DEFENCE_ON.load(Ordering::Acquire) {
        return;
    }
    if let Some(delta) = out.clatter {
        cpu_event!("clatter delta={delta:.3} status={}", natives::status_kind(accessor));
        natives::add_clatter_time(accessor, delta);
        let attributes = CLATTER_EFFECT_ATTRIBUTES.load(Ordering::Acquire);
        if attributes != 0 {
            natives::req_on_joint(accessor, hash40("sys_clatter"), hash40("hip"), 1.0, attributes);
        }
    }
}

/// SPEC-cpu-01: the fast fall.
unsafe fn apply_mash(accessor: usize, out: &FrameOut) {
    if !MASH_ON.load(Ordering::Acquire) {
        return;
    }
    if out.fast_fall {
        cpu_event!("fast_fall from={}", natives::status_kind(accessor));
        let dive = WORK_RESERVE_DIVE.load(Ordering::Acquire);
        if dive != UNRESOLVED {
            natives::set_flag(accessor, true, dive);
        }
        let attribute = EFFECT_CONCLUDE.load(Ordering::Acquire);
        if attribute != UNRESOLVED {
            let pos = [natives::pos_x(accessor), natives::pos_y(accessor), 0.0];
            natives::effect_req(accessor, FLASH_EFFECT.load(Ordering::Acquire), &pos, 2.0, attribute);
        }
    }
}

/// SPEC-cpu-04: the forced status, the cursor and the hidden model and effects of a tech animation.
unsafe fn apply_tech(accessor: usize, out: &FrameOut) {
    if !TECH_ON.load(Ordering::Acquire) {
        return;
    }
    if let Some(status) = out.force_status {
        cpu_event!("force_status status={status} from={}", natives::status_kind(accessor));
        natives::change_status_force(accessor, status);
    }
    let cursor = WORK_CURSOR_OFFSET.load(Ordering::Acquire);
    if let (Some(value), true) = (out.hide.cursor, cursor != UNRESOLVED) {
        cpu_event!("hide cursor={value:.1}");
        natives::set_float(accessor, value, cursor);
    }
    if let Some(visible) = out.hide.model_visible {
        cpu_event!("hide model_visible={visible}");
        natives::set_whole(accessor, visible);
    }
    if out.hide.hide_effects {
        cpu_event!("hide effects status={}", natives::status_kind(accessor));
        for effect in &TECH_EFFECTS {
            natives::set_visible_kind(accessor, effect.load(Ordering::Acquire), false);
        }
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
    let _scope = HookScope::enter_hook(HookId::CpuSdi);
    bump(counts_index::SDI);
    if let Some(key) = role_of(accessor) {
        let cpu_x = natives::pos_x(accessor);
        match with(|cpu| cpu.sdi_query(key, player_x(), cpu_x)) {
            Some(SdiAnswer::Input(x, y)) => {
                *out = [x, y];
                cpu_event_rearm!(ev::SDI_NONE, key.index());
                cpu_event!("sdi_input entry={} x={x:.3} y={y:.3}", key.entry);
                return 1;
            }
            Some(SdiAnswer::NoInput) => {
                cpu_event_on_change!(ev::SDI_NONE, key.index(), Some(0), "sdi_none entry={}", key.entry);
                return 0;
            }
            Some(SdiAnswer::Native) | None => {}
        }
    }
    call_original!(accessor, out)
}

fn air_dodge_stick(accessor: usize) -> Option<(f32, f32)> {
    let key = role_of(accessor)?;
    // SAFETY: `accessor` is a CPU body the frame wrapper registered; the native is a guarded site.
    let cpu_x = unsafe { natives::pos_x(accessor) };
    let answer = with(|cpu| cpu.air_dodge_stick(key, player_x(), cpu_x)).flatten();
    #[cfg(feature = "diag")]
    match answer {
        Some((x, y)) => cpu_event_on_change!(
            ev::AIRDODGE,
            key.index(),
            Some((u64::from(x.to_bits()) << 32) | u64::from(y.to_bits())),
            "airdodge_stick entry={} x={x:.3} y={y:.3} player_x={:.1} cpu_x={cpu_x:.1}",
            key.entry,
            player_x()
        ),
        None => cpu_event_rearm!(ev::AIRDODGE, key.index()),
    }
    answer
}

/// `ControlModule::get_stick_x(accessor)`: the air-dodge direction out of hitstun.
#[skyline::hook(offset = sites_pkg2::STICK_X_HOOK_OFFSET)]
pub unsafe fn cpu_stick_x_hook(accessor: usize) -> f32 {
    let _scope = HookScope::enter_hook(HookId::CpuStickX);
    bump(counts_index::STICK_X);
    let original = call_original!(accessor);
    air_dodge_stick(accessor).map_or(original, |(x, _)| x)
}

/// `ControlModule::get_stick_y(accessor)`: the same pick as x for the episode. Crouch (SPEC-cpu-06)
/// answers after this one when it is built.
#[skyline::hook(offset = sites_pkg2::STICK_Y_HOOK_OFFSET)]
pub unsafe fn cpu_stick_y_hook(accessor: usize) -> f32 {
    let _scope = HookScope::enter_hook(HookId::CpuStickY);
    bump(counts_index::STICK_Y);
    let original = call_original!(accessor);
    if let Some((_, y)) = air_dodge_stick(accessor) {
        return y;
    }
    // Crouch answers when the air-dodge direction does not (SPEC-cpu-06 section 4).
    let Some(key) = role_of(accessor) else { return original };
    let crouch = with(|cpu| cpu.crouch_stick(key)).flatten();
    #[cfg(feature = "diag")]
    match crouch {
        Some(y) => cpu_event_on_change!(ev::CROUCH, key.index(), Some(u64::from(y.to_bits())), "crouch entry={} y={y:.3}", key.entry),
        None => cpu_event_rearm!(ev::CROUCH, key.index()),
    }
    crouch.unwrap_or(original)
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
    let _scope = HookScope::enter_hook(HookId::CpuClatterStart);
    bump(counts_index::CLATTER_START);
    if let Some(key) = role_of(accessor) {
        with(|cpu| cpu.clatter_start(key, manual_rate));
        cpu_event!("clatter_start entry={} manual_rate={manual_rate:.3} initial={initial:.1}", key.entry);
    }
    call_original!(accessor, initial, auto_rate, manual_rate, a, b, c, d)
}

// ---- SPEC-cpu-03 hook -----------------------------------------------------------------------------------

/// `WorkModule::is_enable_transition_term(accessor, term)`: the native automatic ledge climb is denied
/// while a CPU body waits, delays or waits for its playback. Every other query is the original's.
#[skyline::hook(offset = sites_pkg2::TRANSITION_TERM_HOOK_OFFSET)]
pub unsafe fn cpu_transition_term_hook(accessor: usize, term: i32) -> bool {
    let _scope = HookScope::enter_hook(HookId::CpuTransitionTerm);
    bump(counts_index::TRANSITION_TERM);
    let original = call_original!(accessor, term);
    if !original || term != TERM_CLIMB.load(Ordering::Acquire) {
        return original;
    }
    let Some(key) = role_of(accessor) else { return original };
    let status = natives::status_kind(accessor);
    let answer = with(|cpu| cpu.ledge_transition_term(key, status, term, original)).unwrap_or(original);
    if answer {
        cpu_event_rearm!(ev::CLIMB_DENIED, key.index());
    } else {
        cpu_event_on_change!(ev::CLIMB_DENIED, key.index(), Some(1), "climb_denied entry={} status={status}", key.entry);
    }
    answer
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
    let _scope = HookScope::enter_hook(HookId::CpuMotionChange);
    bump(counts_index::MOTION_CHANGE);
    let asked = motion;
    let motion = role_of(accessor).and_then(|key| with(|cpu| cpu.substitute_motion(key, motion))).unwrap_or(motion);
    if motion != asked {
        cpu_event!("motion_substitute from={asked:#x} to={motion:#x} status={}", natives::status_kind(accessor));
    }
    call_original!(accessor, motion, a, b, c, d, e, f)
}

/// `CameraModule::req_quake_pos(accessor, kind, pos)`: no shake for a CPU in the missed-tech status
/// while Hide Tech Animations is On (SPEC-cpu-04 section 4).
#[skyline::hook(offset = sites_pkg2::QUAKE_POS_HOOK_OFFSET)]
pub unsafe fn cpu_quake_pos_hook(accessor: usize, kind: i32, pos: *const [f32; 3]) -> u64 {
    let _scope = HookScope::enter_hook(HookId::CpuQuakePos);
    bump(counts_index::QUAKE_POS);
    let none = QUAKE_KIND_NONE.load(Ordering::Acquire);
    let kind = match role_of(accessor) {
        Some(_) if none != UNRESOLVED => {
            let status = natives::status_kind(accessor);
            if with(|cpu| cpu.quake_suppressed(status)).unwrap_or(false) {
                cpu_event!("quake_suppressed status={status} kind={kind}");
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

/// The game's `CommonParams` object, captured when the game fills it from its parameter file (0 until then).
static COMMON_PARAMS: AtomicUsize = AtomicUsize::new(0);

/// Inline hook on the `mov x19, x0` of the function that fills `CommonParams`: x0 is the object.
#[skyline::hook(offset = sites_pkg2::COMMON_PARAMS_FILL_OFFSET, inline)]
pub unsafe fn cpu_common_params_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let _scope = HookScope::enter();
    let params = ctx.registers[0].x() as usize;
    COMMON_PARAMS.store(params, Ordering::Release);
    cpu_event!("common_params object={params:#x}");
}

// ---- SPEC-cpu-04 fixed camera ---------------------------------------------------------------------------

/// The camera class of the stage the game is on now.
unsafe fn stage_class_now() -> StageClass {
    let id = natives::stage_id();
    tech::stage_class(id, natives::stage_form(id))
}

/// Write the fixed centre into the camera manager (`manager + 0xbd0`, three floats of the game's vector).
unsafe fn write_camera_centre(manager: usize, centre: [f32; 3]) {
    ((manager + sites_pkg2::CAMERA_CENTRE_FIELD) as *mut [f32; 3]).write(centre);
}

/// Inline hook right after the prologue of the camera manager's change-active-camera function (x0 the manager,
/// w1 the requested mode): for the fixed mode in Training, the stage's centre (or the remembered default) goes
/// into the manager before the game acts.
#[skyline::hook(offset = sites_pkg2::CAMERA_MODE_HOOK_OFFSET, inline)]
pub unsafe fn cpu_camera_mode_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let _scope = HookScope::enter();
    let mode = ctx.registers[1].w() as i32;
    if mode != tech::CAMERA_MODE_FIXED || crate::runtime::scene::current().is_none() {
        return;
    }
    let manager = ctx.registers[0].x() as usize;
    let class = stage_class_now();
    let centre = with(|cpu| cpu.camera_mode(class, mode)).flatten();
    cpu_event!("camera_mode mode={mode} class={class:?} stage={} write={centre:?}", natives::stage_id());
    if let (Some(centre), true) = (centre, manager != 0) {
        write_camera_centre(manager, centre);
    }
}

/// The Training fixed-camera setup (`values` first field: the centre the game uses). The centre is remembered as
/// the stage default; after the game copied it into the manager, the stage's value replaces it (Hide Tech
/// Animations On) so the caller's own struct is never written.
#[skyline::hook(offset = sites_pkg2::FIXED_CAMERA_SETUP_HOOK_OFFSET)]
pub unsafe fn cpu_camera_setup_hook(unused: usize, values: *const [f32; 3]) {
    let _scope = HookScope::enter();
    call_original!(unused, values);
    if values.is_null() || crate::runtime::scene::current().is_none() {
        return;
    }
    let game = values.read();
    let class = stage_class_now();
    let centre = with(|cpu| cpu.camera_setup(class, game)).flatten();
    cpu_event!("camera_setup game={game:?} class={class:?} stage={} write={centre:?}", natives::stage_id());
    let slot = (game::text_base() + sites_pkg2::CAMERA_MANAGER_SLOT_OFFSET) as *const *const usize;
    let manager = if slot.read().is_null() { 0 } else { slot.read().read() };
    if let (Some(centre), true) = (centre, manager != 0) {
        write_camera_centre(manager, centre);
    }
}

/// Once per frame (the player entry's frame): the shield damage multiplier, 0.0 while Training is active and
/// the shield is Infinite, otherwise the game's own value, also outside Training (SPEC-cpu-05 section 4).
unsafe fn damage_multiplier_frame(training: bool) {
    let params = COMMON_PARAMS.load(Ordering::Acquire);
    if params == 0 {
        return;
    }
    let field = (params + sites_pkg2::SHIELD_DAMAGE_MUL_FIELD) as *mut f32;
    let current = field.read_volatile();
    if let Some(Some(want)) = with(|cpu| cpu.damage_multiplier(training, current)) {
        if want != current {
            field.write_volatile(want);
            cpu_event!("shield_damage_mul write={want:.3} was={current:.3} training={training}");
        }
    }
}

fn is_guard_button(button: i32) -> bool {
    button != UNRESOLVED && (button == BUTTON_GUARD.load(Ordering::Acquire) || button == BUTTON_GUARD_HOLD.load(Ordering::Acquire))
}

fn is_jump_button(button: i32) -> bool {
    button != UNRESOLVED && (button == BUTTON_JUMP.load(Ordering::Acquire) || button == BUTTON_FLICK_JUMP.load(Ordering::Acquire))
}

/// The shield's answer for the guard buttons, else the full hop's for the jump buttons.
fn guard_button_answer(accessor: usize, button: i32, on_query: bool) -> Option<bool> {
    let guard = is_guard_button(button);
    if !guard && !is_jump_button(button) {
        return None;
    }
    let key = role_of(accessor)?;
    if guard {
        let answer = match with(|cpu| cpu.guard_button(key, on_query))? {
            ButtonAnswer::Held(value) => Some(value),
            ButtonAnswer::Original => None,
        };
        #[cfg(feature = "diag")]
        match answer {
            Some(value) => cpu_event_on_change!(
                if on_query { ev::GUARD_ON } else { ev::GUARD_OFF },
                key.index(),
                Some(u64::from(value)),
                "guard_button entry={} on_query={on_query} held={value}",
                key.entry
            ),
            None => cpu_event_rearm!(if on_query { ev::GUARD_ON } else { ev::GUARD_OFF }, key.index()),
        }
        answer
    } else {
        let answer = with(|cpu| cpu.full_hop_button(key, on_query)).flatten();
        #[cfg(feature = "diag")]
        match answer {
            Some(value) => cpu_event_on_change!(ev::FULL_HOP, key.index(), Some(u64::from(value)), "full_hop_button entry={} on_query={on_query} held={value}", key.entry),
            None => cpu_event_rearm!(ev::FULL_HOP, key.index()),
        }
        answer
    }
}

/// `ControlModule::check_button_on(accessor, button)`: the guard buttons read as held while a CPU
/// body should hold its shield.
#[skyline::hook(offset = sites_pkg2::CHECK_BUTTON_ON_HOOK_OFFSET)]
pub unsafe fn cpu_check_button_on_hook(accessor: usize, button: i32) -> bool {
    let _scope = HookScope::enter_hook(HookId::CpuCheckButtonOn);
    let original = call_original!(accessor, button);
    guard_button_answer(accessor, button, true).unwrap_or(original)
}

/// `ControlModule::check_button_off(accessor, button)`: the guard buttons do not read as released.
#[skyline::hook(offset = sites_pkg2::CHECK_BUTTON_OFF_HOOK_OFFSET)]
pub unsafe fn cpu_check_button_off_hook(accessor: usize, button: i32) -> bool {
    let _scope = HookScope::enter_hook(HookId::CpuCheckButtonOff);
    let original = call_original!(accessor, button);
    guard_button_answer(accessor, button, false).unwrap_or(original)
}

fn shield_tilt(accessor: usize) -> Option<(f32, f32)> {
    let key = role_of(accessor)?;
    // SAFETY: `accessor` is a CPU body the frame wrapper registered; the native is a guarded site.
    let cpu_x = unsafe { natives::pos_x(accessor) };
    let tilt = with(|cpu| cpu.shield_tilt_stick(key, player_x(), cpu_x)).flatten();
    #[cfg(feature = "diag")]
    match tilt {
        Some((x, y)) => cpu_event_on_change!(
            ev::TILT,
            key.index(),
            Some((u64::from(x.to_bits()) << 32) | u64::from(y.to_bits())),
            "shield_tilt entry={} x={x:.3} y={y:.3} player_x={:.1} cpu_x={cpu_x:.1}",
            key.entry,
            player_x()
        ),
        None => cpu_event_rearm!(ev::TILT, key.index()),
    }
    tilt
}

/// `ControlModule::get_stick_x_no_clamp(accessor)`: the shield tilt.
#[skyline::hook(offset = sites_pkg2::STICK_X_NO_CLAMP_HOOK_OFFSET)]
pub unsafe fn cpu_stick_x_no_clamp_hook(accessor: usize) -> f32 {
    let _scope = HookScope::enter_hook(HookId::CpuStickXNoClamp);
    let original = call_original!(accessor);
    shield_tilt(accessor).map_or(original, |(x, _)| x)
}

/// `ControlModule::get_stick_y_no_clamp(accessor)`: the shield tilt.
#[skyline::hook(offset = sites_pkg2::STICK_Y_NO_CLAMP_HOOK_OFFSET)]
pub unsafe fn cpu_stick_y_no_clamp_hook(accessor: usize) -> f32 {
    let _scope = HookScope::enter_hook(HookId::CpuStickYNoClamp);
    let original = call_original!(accessor);
    shield_tilt(accessor).map_or(original, |(_, y)| y)
}

/// `WorkModule::get_param_float(accessor, kind, name)`: shield decay and regeneration for a CPU body
/// while the decay override is active; every other parameter is the original's (the first compare
/// is on the parameter type hash, so the common case costs two integer compares).
#[skyline::hook(offset = sites_pkg2::SHIELD_PARAM_HOOK_OFFSET)]
pub unsafe fn cpu_shield_param_hook(accessor: usize, kind: u64, name: u64) -> f32 {
    let _scope = HookScope::enter_hook(HookId::CpuShieldParam);
    let original = call_original!(accessor, kind, name);
    if kind != PARAM_HASHES[0].load(Ordering::Acquire) {
        return original;
    }
    let decay = name == PARAM_HASHES[1].load(Ordering::Acquire);
    if !decay && name != PARAM_HASHES[2].load(Ordering::Acquire) {
        return original;
    }
    let Some(key) = role_of(accessor) else { return original };
    let value = with(|cpu| cpu.shield_param(key, decay)).flatten();
    #[cfg(feature = "diag")]
    match value {
        Some(v) => cpu_event_on_change!(
            ev::SHIELD_PARAM,
            key.index(),
            Some((u64::from(decay) << 32) | u64::from(v.to_bits())),
            "shield_param entry={} decay={decay} value={v:.4} original={original:.4}",
            key.entry
        ),
        None => cpu_event_rearm!(ev::SHIELD_PARAM, key.index()),
    }
    value.unwrap_or(original)
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
    cpu_event_on_change!(ev::DI_CALL, 0, Some(accessor as u64), "di_before accessor={accessor:#x} role={}", role_of(accessor).is_some());
    let Some(key) = role_of(accessor) else { return };
    let (x, y) = (WORK_VECOR_X.load(Ordering::Acquire), WORK_VECOR_Y.load(Ordering::Acquire));
    if x == UNRESOLVED || y == UNRESOLVED {
        return;
    }
    let cpu_x = natives::pos_x(accessor);
    if let Some(Some((vx, vy))) = with(|cpu| cpu.di_correction(key, player_x(), cpu_x)) {
        natives::set_float(accessor, vx, x);
        natives::set_float(accessor, vy, y);
        cpu_event!("di entry={} x={vx:.3} y={vy:.3} player_x={:.1} cpu_x={cpu_x:.1}", key.entry, player_x());
    }
}

/// Tech: the status a CPU body's script is about to change to, or the one to request instead. `passive`
/// asks the script's own "is passive enabled".
pub(crate) unsafe fn tech_request(accessor: usize, requested: i32, passive: impl FnOnce() -> bool) -> Option<i32> {
    cpu_event_on_change!(ev::TECH_CALL, 0, Some((accessor as u64) ^ ((requested as u32 as u64) << 48)), "tech_call accessor={accessor:#x} requested={requested} role={}", role_of(accessor).is_some());
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
        TechChange::Status(status) => {
            cpu_event!(
                "tech requested={requested} answer={status} passive={} wall={} ceil={} script_passive={}",
                request.passive,
                request.passive_wall,
                request.passive_ceil,
                request.script_passive_enabled
            );
            Some(status)
        }
        TechChange::Unchanged => {
            cpu_event_on_change!(
                ev::TECH,
                key.index(),
                Some(requested as u32 as u64),
                "tech_unchanged requested={requested} passive={} wall={} ceil={} script_passive={}",
                request.passive,
                request.passive_wall,
                request.passive_ceil,
                request.script_passive_enabled
            );
            None
        }
    }
}

/// Out of shield: after the script continuation, the dodge or roll status to change to, if any.
pub(crate) unsafe fn oos_after(accessor: usize) -> Option<i32> {
    cpu_event_on_change!(ev::OOS_CALL, 0, Some(accessor as u64), "oos_call accessor={accessor:#x} role={}", role_of(accessor).is_some());
    let key = role_of(accessor)?;
    let prev = natives::prev_status_kind(accessor, 0);
    let status_now = natives::status_kind(accessor);
    let motion = natives::motion_kind(accessor);
    let term = TERM_CONT_ESCAPE.load(Ordering::Acquire);
    let cont_escape = term != UNRESOLVED && natives::is_enable_transition_term(accessor, term);
    let parry = PARRY_MOTION.load(Ordering::Acquire);
    let escape = with(|cpu| {
        let facts = OosFacts {
            left_shieldstun: cpu.ids.is(C::StatusGuardDamage, prev),
            shielding: status::shielding(&cpu.ids, status_now),
            in_parry: motion == parry,
            cont_escape,
        };
        cpu.out_of_shield(key, &facts).escape_status
    })
    .flatten();
    #[cfg(feature = "diag")]
    if let Some(status) = escape {
        cpu_event!("oos_escape entry={} status={status} from={status_now} prev={prev}", key.entry);
    }
    escape
}

// ---- SPEC-cpu-01 and SPEC-cpu-06 hooks ------------------------------------------------------------------

/// Whether the mash hooks and natives are installed.
pub fn set_mash_on(on: bool) {
    MASH_ON.store(on, Ordering::Release);
}

unsafe fn term_on(accessor: usize, id: &AtomicI32) -> bool {
    let id = id.load(Ordering::Acquire);
    id != UNRESOLVED && natives::is_enable_transition_term(accessor, id)
}

/// Everything a command query reads from the game, with no lock held.
unsafe fn command_facts(accessor: usize, key: FighterKey) -> CmdFacts {
    let (px, py, pz) = (player_pos().0, player_pos().1, player_pos().2);
    let (cx, cy, cz) = (natives::pos_x(accessor), natives::pos_y(accessor), natives::pos_z(accessor));
    let distance = if player_pos_valid() { ((cx - px).powi(2) + (cy - py).powi(2) + (cz - pz).powi(2)).sqrt() } else { f32::INFINITY };
    let invalid = WORK_INVALID_CATCH.load(Ordering::Acquire);
    let gravity = KINETIC_GRAVITY.load(Ordering::Acquire);
    let motion = natives::motion_kind(accessor);
    let situation = natives::situation_kind(accessor);
    let status = natives::status_kind(accessor);
    let mut facts = CmdFacts {
        status,
        situation: crate::model::cpu::status::Situation::Other,
        motion_frame: natives::motion_frame(accessor) as i32,
        motion_is_dash: motion == DASH_MOTION.load(Ordering::Acquire),
        in_parry: motion == PARRY_MOTION.load(Ordering::Acquire),
        invalid_catch_frame: if invalid != UNRESOLVED { natives::get_int(accessor, invalid) } else { 0 },
        cont_dash: term_on(accessor, &TERM_CONT_DASH),
        cont_catch_dash: term_on(accessor, &TERM_CATCH_DASH),
        cont_attack_air: term_on(accessor, &TERM_ATTACK_AIR),
        cont_throw_hi: term_on(accessor, &TERM_THROW_HI),
        kind: UNRESOLVED,
        falling: gravity != UNRESOLVED && natives::sum_speed_y(accessor, gravity) < 0.0,
        distance,
        prev_status: natives::prev_status_kind(accessor, 0),
    };
    // The situation and the fighter kind are classified under the lock (they need the table).
    with(|cpu| {
        facts.situation = crate::model::cpu::status::Situation::of(&cpu.ids, situation);
        facts.kind = cpu.fighters[key.index()].kind;
    });
    facts
}

/// `ControlModule::get_command_flag_cat(accessor, category)`: the mash's bits (category 1) or the throw
/// bits (category 2) of a CPU body are ORed into the game's answer.
#[skyline::hook(offset = sites_pkg2::COMMAND_FLAG_HOOK_OFFSET)]
pub unsafe fn cpu_command_flag_hook(accessor: usize, category: i32) -> u32 {
    let _scope = HookScope::enter();
    bump(counts_index::COMMAND_FLAG);
    let original = call_original!(accessor, category);
    let cat2 = category == CAT2.load(Ordering::Acquire);
    if category == UNRESOLVED || (!cat2 && category != CAT1.load(Ordering::Acquire)) {
        return original;
    }
    let Some(key) = role_of(accessor) else { return original };
    if !with(|cpu| cpu.wants_commands(key, cat2)).unwrap_or(false) {
        return original;
    }
    let facts = command_facts(accessor, key);
    let Some(out) = with(|cpu| cpu.command_flags(key, cat2, &facts)) else { return original };
    if let Some(status) = out.request_status {
        cpu_event!("command_request_status entry={} status={status} from={}", key.entry, natives::status_kind(accessor));
        natives::change_status_request(accessor, status);
    }
    if out.stop_playback {
        cpu_event!("playback_stop entry={}", key.entry);
        // Playback Mash Interrupt: the running playback stops (package 4's recorder).
        recorder::interrupt_playback();
    }
    #[cfg(feature = "diag")]
    {
        let (queued, latched) = with(|cpu| cpu.mash_debug(key)).unwrap_or((false, false));
        let status = natives::status_kind(accessor);
        let state = (u64::from(status as u32) << 20) ^ (u64::from(out.bits) << 2) ^ (u64::from(queued) << 1) ^ u64::from(latched);
        cpu_event_on_change!(
            if cat2 { ev::CMD2 } else { ev::CMD1 },
            key.index(),
            Some(state),
            "command_query entry={} cat={} status={status} bits={:#x} request={:?} queued={queued} latched={latched} situation={:?}",
            key.entry,
            if cat2 { 2 } else { 1 },
            out.bits,
            out.request_status,
            facts.situation
        );
    }
    original | out.bits
}

/// `ControlModule::get_attack_air_kind(accessor)`: the queued aerial's kind for a CPU body.
#[skyline::hook(offset = sites_pkg2::ATTACK_AIR_KIND_HOOK_OFFSET)]
pub unsafe fn cpu_attack_air_kind_hook(accessor: usize) -> i32 {
    let _scope = HookScope::enter();
    bump(counts_index::AIR_KIND);
    let original = call_original!(accessor);
    let Some(key) = role_of(accessor) else { return original };
    let kind = with(|cpu| cpu.attack_air_kind(key)).flatten();
    #[cfg(feature = "diag")]
    match kind {
        Some(k) => cpu_event_on_change!(ev::AIR_KIND, key.index(), Some(k as u32 as u64), "attack_air_kind entry={} kind={k} original={original}", key.entry),
        None => cpu_event_rearm!(ev::AIR_KIND, key.index()),
    }
    kind.unwrap_or(original)
}

/// `ControlModule::get_stick_dir(accessor)`: the attack angle of a CPU body.
#[skyline::hook(offset = sites_pkg2::STICK_DIR_HOOK_OFFSET)]
pub unsafe fn cpu_stick_dir_hook(accessor: usize) -> f32 {
    let _scope = HookScope::enter();
    bump(counts_index::STICK_DIR);
    let original = call_original!(accessor);
    let Some(key) = role_of(accessor) else { return original };
    let angle = with(|cpu| cpu.attack_angle(key)).flatten();
    #[cfg(feature = "diag")]
    match angle {
        Some(a) => cpu_event_on_change!(ev::STICK_DIR, key.index(), Some(u64::from(a.to_bits())), "stick_dir entry={} angle={a:.3} original={original:.3}", key.entry),
        None => cpu_event_rearm!(ev::STICK_DIR, key.index()),
    }
    angle.unwrap_or(original)
}

/// Whether a fighter can act now: any of the five transition terms or a cancel (SPEC-cpu-07 §3).
unsafe fn actionable(accessor: usize) -> bool {
    term_on(accessor, &TERM_ESCAPE_AIR)
        || term_on(accessor, &TERM_ATTACK_AIR)
        || term_on(accessor, &TERM_GUARD_ON)
        || term_on(accessor, &TERM_CONT_ESCAPE)
        || term_on(accessor, &TERM_DOWN_STAND)
        || natives::is_enable_cancel(accessor)
}

/// Frame advantage, on the player entry's frame: reads both fighters only while the row is On.
unsafe fn frame_advantage(player: usize) {
    if !MASH_ON.load(Ordering::Acquire) {
        return;
    }
    // The replay editor or a replay capture loading holds the screen: the measurement clears and waits (SPEC-cpu-07 §3).
    if recorder::replay_editor_occupied() || recorder::replay_capture_loading() {
        with(|cpu| cpu.frame_advantage(false, false));
        return;
    }
    let lobby = crate::runtime::scene::current() == Some(Scene::Lobby);
    let primary = FighterKey::new(i32::from(primary_entry(lobby)), false);
    let Some(cpu_accessor) = primary.and_then(|key| CPU.lock().as_ref().and_then(|c| c.roles.accessor_of(key))) else { return };
    if !with(|cpu| cpu.settings.frame_advantage && cpu.settings.mash_state.is_empty()).unwrap_or(false) {
        // Off, or a mashing CPU: the state clears and nothing is read.
        with(|cpu| cpu.frame_advantage(false, false));
        return;
    }
    let (p, c) = (actionable(player), actionable(cpu_accessor));
    if let Some(Some(value)) = with(|cpu| cpu.frame_advantage(p, c)) {
        // The notification (SPEC-cpu-07 §2): package 4's notices show it, clearing every other one.
        notices::post_frame_advantage(value);
        cpu_event!("frame_advantage value={value}");
    }
}
