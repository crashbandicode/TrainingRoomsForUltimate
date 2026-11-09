//! Private, default-off native AI-as-pad and command adapter, Training and exact audited online 1v1 scope only.
//! No operation-CPU change persists beyond the synchronous native initialization.
//! Inert when the standalone ssbu-cpu-pilot plugin is installed (see `init`).
use std::convert::TryFrom;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

// hos-lock's Mutex: std's boxes its pthread mutex at the first lock, which the hooks below take (no heap in hooks).
use hos_lock::Mutex;
use serde::Deserialize;
use serde_json::json;

use crate::engine::compat::fixed::{FixedList, FixedQueue};
use crate::engine::compat::json_line::{DebugStr, Json, Obj};
use skyline::hooks::{getRegionAddress, Region};
use smash::app::{lua_bind::*, BattleObjectModuleAccessor};
use smash::lib::lua_const::*;
// The AI-as-pad primitives (rank ladder, pad encoding, and the native
// qualify/ready/arm/think/release calls) come from the shared core library.
// This module keeps the online stage session, request.json, the menu session
// and the logging.
use ssbu_cpu_pilot_core::native::{self as core_native, Native};
use ssbu_cpu_pilot_core::{level as core_level, pad as core_pad};

use crate::runtime::compat::common::input::MappedInputs;

#[path = "../../model/p1/mod.rs"]
mod pilot_core;
use self::pilot_core::move_observer::{MoveObserver, Observation};
use crate::engine::p1_session::menu_session as menu_session;
use crate::engine::p1_session::stage_session as stage_session;

// Publication is lock-free: menu commits may hold the App lock while on_frame
// holds STATE and asks whether the menu captures input. Never acquire STATE here.
static MENU_DESIRED: AtomicU64 = AtomicU64::new(0);
pub(crate) fn menu_settings_changed(enabled: bool, selected: Option<u8>) {
    let level = menu_session::level(enabled, selected);
    let _ = MENU_DESIRED.fetch_update(Ordering::AcqRel, Ordering::Acquire, |previous| {
        Some(menu_session::update(previous, level))
    });
}

const ROOT: &str = "sd:/ultimate/TrainingRooms/cpu_pilot";
const REQUEST: &str = "sd:/ultimate/TrainingRooms/cpu_pilot/request.json";
static BASE: AtomicUsize = AtomicUsize::new(0);
static EMITTING: AtomicBool = AtomicBool::new(false);
static PENDING: AtomicU64 = AtomicU64::new(0);
// FIM observes owned slots only; it never advances the simulation clock.
static FIM_MASK: AtomicUsize = AtomicUsize::new(0);
static EMIT_SLOT: AtomicUsize = AtomicUsize::new(usize::MAX);
static EMIT_SCENE: AtomicU64 = AtomicU64::new(0);
static STATE: Mutex<State> = Mutex::new(State {
    polls: 0,
    active: None,
    pending_request: None,
    scene: None,
    result: false,
    last_gate: "",
    diagnostic_rows: 0,
    pair_logged: false,
    observed_binding: None,
    input_logged: false,
    menu: menu_session::Arbitration::new(),
});

// The fallback's file work runs on the heartbeat (`service`), never in a hook (no heap in hooks): the hook asks for a read of
// request.json at its old cadence and takes what the heartbeat parsed and planned; it posts its receipts to the heartbeat.
static READ_WANTED: AtomicBool = AtomicBool::new(false);
static POSTED: Mutex<Option<Posted>> = Mutex::new(None);
/// The run id of the last request posted: an old request cannot re-arm a new match (the heartbeat's own).
static SEEN: Mutex<Option<RunId>> = Mutex::new(None);
static OUTBOX: Mutex<FixedQueue<Outgoing, 8>> = Mutex::new(FixedQueue::new());

/// A request read, validated and planned off the hook.
struct Posted {
    request: RequestFacts,
    planned: Result<(u8, Option<u32>, Option<CommandRun>), &'static str>,
}

/// What the hook hands the heartbeat to write.
enum Outgoing {
    /// A request run's receipt, `armed` or its end; `trace` is the command trace at its end.
    Receipt { facts: ReceiptFacts, reason: &'static str, trace: Vec<TraceRow> },
    /// A request that ended before it armed; its planned command is freed off the hook.
    Rejected { request: RequestFacts, reason: &'static str, stage: Option<StageSession>, command: Option<CommandRun> },
    /// A planned command the hook did not take up.
    Discard(CommandRun),
}

fn post(outgoing: Outgoing) {
    let _ = OUTBOX.lock().push(outgoing);
}

/// A request's run id: `Request::kind` admits exactly 32 hex digits, kept without a heap string.
#[derive(Clone, Copy, PartialEq, Eq)]
struct RunId([u8; 32]);
impl RunId {
    fn of(run: &str) -> Option<Self> {
        run.as_bytes().try_into().ok().map(Self)
    }
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("")
    }
}
impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl serde::Serialize for RunId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// What the hook keeps of a request.
#[derive(Clone, Copy)]
struct RequestFacts {
    run: RunId,
    emit: bool,
    pilot: Option<pilot_core::config::JsonConfig>,
    scope: Scope,
    local_slot: Option<u8>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    run: String,
    rank: Option<u8>,
    frames: Option<u32>,
    emit: bool,
    pilot: Option<pilot_core::config::JsonConfig>,
    walk_direction: Option<WalkDirection>,
    #[serde(default)]
    scope: Scope,
    local_slot: Option<u8>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WalkDirection {
    Left,
    Right,
    AwayFromOpponent,
}
impl WalkDirection {
    fn axis(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
            // Resolved when the command begins, before publishing any input.
            Self::AwayFromOpponent => 0.0,
        }
    }

    fn resolve(self, local_x: f32, peer_x: f32) -> Self {
        match self {
            Self::AwayFromOpponent if local_x < peer_x => Self::Left,
            Self::AwayFromOpponent => Self::Right,
            fixed => fixed,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::AwayFromOpponent => "away_from_opponent",
        }
    }
}

enum RequestKind {
    Raw {
        rank: u8,
        frames: u32,
    },
    Command {
        config: pilot_core::Config,
        direction: WalkDirection,
    },
}

impl Request {
    fn kind(&self) -> Option<RequestKind> {
        if self.local_slot.is_some_and(|slot| slot > 7) {
            return None;
        }
        if self.run.len() != 32 || !self.run.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        match (self.rank, self.frames, self.pilot) {
            (Some(rank), Some(frames), None)
                if rank <= 100 && (1..=3600).contains(&frames) && self.walk_direction.is_none() =>
            {
                Some(RequestKind::Raw { rank, frames })
            }
            (None, None, Some(json)) if (1..=3600).contains(&json.guard_frames) => {
                let config = pilot_core::Config::try_from(json).ok()?;
                Some(RequestKind::Command {
                    config,
                    direction: self.walk_direction.unwrap_or(WalkDirection::AwayFromOpponent),
                })
            }
            _ => None,
        }
    }
}

struct State {
    polls: u32,
    active: Option<Run>,
    // Online only: validated requests must be visible before the phase-2 gate.
    pending_request: Option<RunPlan>,
    scene: Option<Scene>,
    result: bool,
    last_gate: &'static str,
    diagnostic_rows: u8,
    pair_logged: bool,
    observed_binding: Option<Binding>,
    input_logged: bool,
    menu: menu_session::Arbitration,
}

#[derive(Clone, Copy)]
enum RunSource {
    Request(RequestFacts),
    Menu { level: u8 },
}
impl RunSource {
    fn menu_level(&self) -> Option<u8> {
        match self {
            Self::Menu { level } => Some(*level),
            Self::Request(_) => None,
        }
    }
    fn emit(&self) -> bool {
        match self {
            Self::Request(request) => request.emit,
            Self::Menu { .. } => true,
        }
    }
}

struct RunPlan {
    source: RunSource,
    rank: u8,
    raw_frames: Option<u32>,
    command: Option<CommandRun>,
}

struct Run {
    source: RunSource,
    binding: Binding,
    observed_fim_mask: u8,
    arming_cpu_flags: ArmingFlags,
    stage_initialization: Option<StageSession>,
    native_rank: u8,
    raw_frames: Option<u32>,
    command: Option<CommandRun>,
    p1_id: u32,
    cpu_id: u32,
    ctrl: usize,
    ai: usize,
    old_rank: i32,
    random: [u8; 20],
    samples: u32,
    nonneutral: u32,
    first_outputs: FixedList<u64, 24>,
    start: Snapshot,
    end: Snapshot,
    flag_samples: u32,
    flags_zero: bool,
    last_ctrl_operation: u8,
    last_service_operation: u8,
    rank_restore_attempted: Option<bool>,
}

struct CommandRun {
    pilot: pilot_core::Pilot,
    observer: MoveObserver,
    tick: u64,
    direction: WalkDirection,
    first_command_frame: Option<u64>,
    last_status: Option<i32>,
    /// Reserved for `TRACE_LIMIT` rows when the request is planned (off the hook): a push never grows it.
    trace: Vec<TraceRow>,
    trace_dropped: u32,
}

const TRACE_LIMIT: usize = 128;

/// One command-trace row, kept as values; the receipt's JSON is built from it on the heartbeat.
#[derive(Clone, Copy)]
struct TraceRow {
    game_frame: u64,
    status: i32,
    prev0: i32,
    events: pilot_core::move_observer::Events,
    phase: pilot_core::Phase,
    output: pilot_core::Output,
    p1_damage: f32,
    cpu_damage: f32,
}
impl TraceRow {
    fn value(&self) -> serde_json::Value {
        let events = self.events;
        json!({"game_frame":self.game_frame,"status":self.status,
            "prev0":self.prev0,
            "active_family":events.active_family.map(|f| format!("{f:?}")),
            "completed":events.completed.map(|f| format!("{f:?}")),
            "interrupted":events.interrupted.map(|f| format!("{f:?}")),
            "phase":format!("{:?}",self.phase),
            "output":format!("{:?}",self.output),"p1_damage":self.p1_damage,"cpu_damage":self.cpu_damage})
    }
}

/// A request run's receipt facts (the run at arming or at its end), written to its file on the heartbeat.
#[derive(Clone, Copy)]
struct ReceiptFacts {
    request: RequestFacts,
    binding: Binding,
    observed_fim_mask: u8,
    arming_cpu_flags: ArmingFlags,
    native_rank: u8,
    stage_initialization: Option<StageSession>,
    samples: u32,
    nonneutral: u32,
    first_outputs: FixedList<u64, 24>,
    start: Snapshot,
    end: Snapshot,
    p1_id: u32,
    cpu_id: u32,
    flag_samples: u32,
    flags_zero: bool,
    last_ctrl_operation: u8,
    last_service_operation: u8,
    rank_restore_attempted: Option<bool>,
    command: Option<CommandFacts>,
}

#[derive(Clone, Copy)]
struct CommandFacts {
    elapsed_game_frames: u64,
    moves: u64,
    hits: u64,
    damage_dealt: f32,
    phase: pilot_core::Phase,
    tick: u64,
    first_command_frame: Option<u64>,
    direction: WalkDirection,
    trace_dropped: u32,
}

impl ReceiptFacts {
    fn of(run: &Run, request: RequestFacts) -> Self {
        Self {
            request,
            binding: run.binding,
            observed_fim_mask: run.observed_fim_mask,
            arming_cpu_flags: run.arming_cpu_flags,
            native_rank: run.native_rank,
            stage_initialization: run.stage_initialization,
            samples: run.samples,
            nonneutral: run.nonneutral,
            first_outputs: run.first_outputs,
            start: run.start,
            end: run.end,
            p1_id: run.p1_id,
            cpu_id: run.cpu_id,
            flag_samples: run.flag_samples,
            flags_zero: run.flags_zero,
            last_ctrl_operation: run.last_ctrl_operation,
            last_service_operation: run.last_service_operation,
            rank_restore_attempted: run.rank_restore_attempted,
            command: run.command.as_ref().map(|command| CommandFacts {
                elapsed_game_frames: command.pilot.elapsed_game_frames(),
                moves: command.pilot.completed_moves(),
                hits: command.pilot.damaging_hits(),
                damage_dealt: command.pilot.damage_dealt(),
                phase: command.pilot.phase(),
                tick: command.tick,
                first_command_frame: command.first_command_frame,
                direction: command.direction,
                trace_dropped: command.trace_dropped,
            }),
        }
    }
}

#[derive(Clone, Copy, serde::Serialize)]
struct Snapshot {
    p1_x: f32,
    p1_y: f32,
    p1_damage: f32,
    cpu_x: f32,
    cpu_y: f32,
    cpu_damage: f32,
}

// Required fields are rebound from the current controller/AI callback. The
// native() checks qualify each owner before use; do not retain these addresses.
#[inline]
unsafe fn read<T: Copy>(address: usize) -> T {
    crate::engine::seams::native_fields::read_unchecked(address, 0)
}
#[inline]
unsafe fn write<T: Copy>(address: usize, value: T) {
    crate::engine::seams::native_fields::write_unchecked(address, 0, value);
}
unsafe fn ptr(address: usize) -> Option<usize> {
    let value: usize = read(address);
    (value != 0 && value.is_multiple_of(8)).then_some(value)
}

// The qualification itself lives in the core library. It names the service and
// RNG pointer walks coarsely ("service", "rng_state"); keep the finer gate names
// this module has always logged.
unsafe fn native(entry: Entry, slot: u8) -> Result<Native, &'static str> {
    let base = BASE.load(Ordering::Acquire);
    core_native::resolve(base, entry.ctrl, entry.info, slot).map_err(|reason| match reason {
        "service" if ptr(base + core_native::offsets::AI_SERVICE).is_none() => "service_root",
        "rng_state" => match ptr(base + core_native::offsets::RNG_ROOT) {
            None => "rng_root",
            Some(root) if ptr(root).is_none() => "rng_owner",
            Some(_) => "rng_state",
        },
        other => other,
    })
}

// Never acquire STATE here, and never hold STAGE across a native call. The
// manager task may call hooked code; a STATE -> original -> STATE cycle deadlocks.
static STAGE: Mutex<Option<StageSession>> = Mutex::new(None);
static STAGE_GENERATION: AtomicU64 = AtomicU64::new(0);
static STAGE_CALLING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
enum StageIntent {
    None,
    Request,
    Menu,
}
impl StageIntent {
    fn enabled(self) -> bool {
        match self {
            Self::None => false,
            Self::Request => true,
            // Observe Off immediately, even before the next owned fighter tick.
            Self::Menu => (1..=9).contains(&((MENU_DESIRED.load(Ordering::Acquire) & 15) as u8)),
        }
    }
}

#[derive(Clone, Copy, serde::Serialize)]
struct StageSnapshot {
    phase: i32,
    d0: u8,
    d1: u8,
    valid: Option<u8>,
    stage_id: i32,
}
unsafe fn stage_snapshot(service: usize) -> StageSnapshot {
    StageSnapshot {
        phase: read(service + 0x1fc),
        d0: read(service + 0xd0),
        d1: read(service + 0xd1),
        valid: ptr(service + 0xc8).map(|stage| read(stage)),
        stage_id: read(service + 0x150),
    }
}

#[derive(Clone, Copy)]
struct StageSession {
    generation: u64,
    binding: Binding,
    service: usize,
    random_address: usize,
    random: [u8; 20],
    initial: StageSnapshot,
    last_phase: i32,
    progress: stage_session::Session,
    intent: StageIntent,
}
/// A stage session's evidence: written straight into a hook's diag line, or converted for a receipt on the heartbeat. The fields
/// are the sorted keys the fork's `json!` wrote.
#[derive(Clone, Copy)]
struct Evidence(StageSession);
impl serde::Serialize for Evidence {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = self.0;
        serde::Serialize::serialize(&Obj((
            ("activation_binding", s.binding),
            ("after_phase", s.last_phase),
            ("analysis_game_frames", s.progress.frames),
            ("before", s.initial),
            ("d0_restored_each_call", true),
            ("frame_limit", stage_session::LIMIT),
            ("shared_rng_restored_each_call", true),
            ("status", DebugStr(s.progress.status)),
        )), serializer)
    }
}

fn clear_stage() {
    STAGE_GENERATION.fetch_add(1, Ordering::AcqRel);
    *STAGE.lock() = None;
}

fn cancel_stage_initialization() {
    let mut guard = STAGE.lock();
    // Losing pilot identity must not expose the extra phase-2 manager body to
    // shared RNG. A timeout also remains refused until the actual scene ends.
    if let Some(session) = guard.as_mut() {
        if session.progress.status == stage_session::Status::Pending {
            session.progress.status = stage_session::Status::Cancelled;
        }
    }
}

fn stage_evidence(binding: Binding) -> Option<StageSession> {
    let stage = *STAGE.lock();
    stage.filter(|s| s.binding.scene == binding.scene)
}

unsafe fn random_address() -> Option<usize> {
    let base = BASE.load(Ordering::Acquire);
    ptr(ptr(ptr(base + 0x52c35b0)?)?).map(|state| state + 0x50)
}

unsafe fn service_address() -> Option<(usize, usize)> {
    let owner = ptr(BASE.load(Ordering::Acquire) + 0x52b6fd8)?;
    Some((owner, ptr(owner)?))
}

unsafe fn human_only(local: Entry, peer: Entry, service: usize) -> bool {
    [local, peer].iter().all(|entry| {
        (0..8).contains(&entry.slot)
            && read::<usize>(entry.ctrl + 0x60) == 0
            && read::<u8>(entry.ctrl + 0x70) == 0
            && read::<u8>(entry.info + 0x92) == 0
            && read::<u8>(service + 0x80 + entry.slot as usize) == 0
    })
}

// Called only with a current validated online pair and a fully validated request
// or measured menu level. first_pair and DefaultOff never call start themselves.
unsafe fn stage_admission(
    binding: Binding,
    local: Entry,
    peer: Entry,
    n: Native,
    intent: StageIntent,
) -> Result<Option<StageSession>, &'static str> {
    if binding.scene.scope != Scope::LanOnline1v1 {
        return Ok(None);
    }
    let snapshot = stage_snapshot(n.service);
    let humans = human_only(local, peer, n.service);
    let mut guard = STAGE.lock();
    if let Some(session) = guard.as_mut() {
        if session.progress.status == stage_session::Status::Invalid
            || session.binding.scene != binding.scene
            || (session.progress.status != stage_session::Status::Ready
                && session.binding != binding)
            || session.service != n.service
            || session.random_address != n.random
            || session.initial.stage_id != snapshot.stage_id
        {
            return Err("stage_ai_identity_changed");
        }
        // Never suppress RNG draws belonging to a real native CPU/subfighter.
        if snapshot.d0 & 1 != 0 || !humans {
            *guard = None;
            return Ok(None);
        }
        session.intent = intent;
        if session.progress.status != stage_session::Status::Cancelled {
            return Ok(Some(*session));
        }
        *guard = None;
    }
    if snapshot.stage_id == -1 || snapshot.stage_id == 0x138 {
        return Ok(None);
    }
    if let Some(progress) =
        stage_session::Session::start(intent.enabled(), humans, snapshot.phase, snapshot.d0)
    {
        let session = StageSession {
            generation: STAGE_GENERATION.fetch_add(1, Ordering::AcqRel) + 1,
            binding,
            service: n.service,
            random_address: n.random,
            random: read(n.random),
            initial: snapshot,
            last_phase: snapshot.phase,
            progress,
            intent,
        };
        *guard = Some(session);
        return Ok(Some(session));
    }
    Ok(None)
}

unsafe fn stage_live(session: StageSession, svc: usize) -> Option<(usize, usize)> {
    if current_scene()? != session.binding.scene {
        return None;
    }
    let manager = manager()?;
    if FighterManager::is_result_mode(manager) || FighterManager::entry_count(manager) != 2 {
        return None;
    }
    let (owner, data) = service_address()?;
    let random = random_address()?;
    if owner != svc
        || data != session.service
        || random != session.random_address
        || read::<i32>(data + 0x150) != session.initial.stage_id
    {
        return None;
    }
    // The native manager keeps running after a stock/BOID or pilot-owner change.
    // Once initialized its RNG lease belongs to this scene/service, not a fighter.
    if session.progress.status == stage_session::Status::Ready {
        return Some((data, random));
    }
    let mask = FIM_MASK.load(Ordering::Acquire) as u8;
    let (binding, local, peer) = pair(session.binding.scene, mask, manager).ok()?;
    if binding != session.binding {
        return None;
    }
    let n = native(local, binding.slot).ok()?;
    (n.service == data && n.random == random && human_only(local, peer, data))
        .then_some((data, random))
}

fn invalidate_stage(generation: u64) {
    let mut guard = STAGE.lock();
    if let Some(session) = guard.as_mut().filter(|s| s.generation == generation) {
        session.progress.status = stage_session::Status::Invalid;
    }
    clear_output();
}

#[skyline::hook(offset = 0x30c710)]
unsafe fn ai_process_begin(svc: usize) {
    // A recursive call is already inside the outer call's RNG transaction.
    if STAGE_CALLING.swap(true, Ordering::AcqRel) {
        original!()(svc);
        return;
    }
    let session = *STAGE.lock();
    let Some(session) = session else {
        original!()(svc);
        STAGE_CALLING.store(false, Ordering::Release);
        return;
    };
    if matches!(
        session.progress.status,
        stage_session::Status::Failed
            | stage_session::Status::Cancelled
            | stage_session::Status::Invalid
    ) {
        original!()(svc);
        STAGE_CALLING.store(false, Ordering::Release);
        return;
    }
    let Some((service, random)) = stage_live(session, svc) else {
        invalidate_stage(session.generation);
        original!()(svc);
        STAGE_CALLING.store(false, Ordering::Release);
        return;
    };
    let saved_d0: u8 = read(service + 0xd0);
    if saved_d0 & 1 != 0 {
        clear_stage();
        original!()(svc);
        STAGE_CALLING.store(false, Ordering::Release);
        return;
    }
    let paused = menu_busy();
    let action = {
        let mut guard = STAGE.lock();
        guard
            .as_mut()
            .filter(|s| s.generation == session.generation)
            .map_or(stage_session::Action::Pass, |s| {
                s.progress.before(s.intent.enabled(), paused)
            })
    };
    if action == stage_session::Action::Pass {
        original!()(svc);
        STAGE_CALLING.store(false, Ordering::Release);
        return;
    }
    let shared: [u8; 20] = read(random);
    write(random, session.random);
    // Online d1 remains set, so native process_begin rechecks stage analysis
    // even after reaching phase 2. Maintain its AI-present input for every
    // leased manager call, including Ready/Isolate; restore before returning.
    write(service + 0xd0, saved_d0 | 1);
    original!()(svc);
    // Re-resolve owners after original: never dereference a cached service/RNG
    // after a transition. Restore only storage that is still the live singleton.
    let same_random = random_address() == Some(random);
    let private = same_random.then(|| read::<[u8; 20]>(random));
    if same_random {
        write(random, shared);
    }
    let same_service = service_address() == Some((svc, service));
    if same_service {
        write(service + 0xd0, saved_d0);
    }
    let phase = same_service.then(|| read::<i32>(service + 0x1fc));
    let mut completed = session;
    if phase == Some(2) && action == stage_session::Action::Initialize {
        completed.progress.status = stage_session::Status::Ready;
    }
    if same_service && same_random && stage_live(completed, svc).is_some() {
        let mut guard = STAGE.lock();
        if let Some(s) = guard
            .as_mut()
            .filter(|s| s.generation == session.generation)
        {
            s.random = private.unwrap();
            s.last_phase = phase.unwrap();
            if completed.progress.status == stage_session::Status::Ready {
                // Owner cancellation cannot undo the native phase-2 transition.
                s.progress.status = stage_session::Status::Ready;
            } else {
                s.progress.after(phase.unwrap());
            }
        }
    } else {
        invalidate_stage(session.generation);
    }
    STAGE_CALLING.store(false, Ordering::Release);
}

fn menu_busy() -> bool {
    crate::runtime::compat::common::menu::captures_input()
        || crate::runtime::compat::sync::read(&crate::runtime::lobby::training::ui::menu::VANILLA_MENU_ACTIVE)
        || crate::runtime::compat::replay_preview::occupied()
        || crate::runtime::lobby::training::recording_or_playback()
}

/// Whether `init` installed the pilot's hook (the main base is set only after the guards passed and the hook went in).
pub(crate) fn installed() -> bool {
    BASE.load(Ordering::Acquire) != 0
}

fn clear_output() {
    EMITTING.store(false, Ordering::Release);
    PENDING.store(0, Ordering::Release);
    EMIT_SLOT.store(usize::MAX, Ordering::Release);
    EMIT_SCENE.store(0, Ordering::Release);
}

#[derive(Clone, Copy, Debug, Default, Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Scope {
    #[default]
    OfflineTraining,
    #[serde(rename = "lan_online_1v1")]
    LanOnline1v1,
}

#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
struct Scene {
    scope: Scope,
    initialized: bool,
    melee: u8,
    mode: u32,
}
impl Scene {
    fn key(self) -> u64 {
        1 | (u64::from(self.scope == Scope::LanOnline1v1) << 1)
            | (u64::from(self.initialized) << 2)
            | (u64::from(self.melee) << 3)
            | (u64::from(self.mode) << 11)
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
struct Binding {
    scene: Scene,
    slot: u8,
    local_entry: i32,
    peer_entry: i32,
    local_boid: u32,
    peer_boid: u32,
    local_kind: i32,
    peer_kind: i32,
    ctrl: usize,
    ai: usize,
}

#[derive(Clone, Copy)]
struct Entry {
    index: i32,
    boid: u32,
    kind: i32,
    slot: i32,
    mirror_slot: i32,
    ctrl: usize,
    info: usize,
    module: *mut BattleObjectModuleAccessor,
}

unsafe fn current_scene() -> Option<Scene> {
    let mode = crate::runtime::lobby::game_mode::current();
    if crate::runtime::compat::common::is_native_training_mode() {
        return Some(Scene {
            scope: Scope::OfflineTraining,
            initialized: mode.is_some_and(|m| m.initialized),
            melee: mode.map_or(0, |m| m.melee),
            mode: mode.map_or(0, |m| m.mode),
        });
    }
    let mode = mode?;
    (mode.initialized && mode.melee == 0x0e && mode.mode == 0x07010101).then_some(Scene {
        scope: Scope::LanOnline1v1,
        initialized: true,
        melee: mode.melee,
        mode: mode.mode,
    })
}

unsafe fn manager() -> Option<*mut smash::app::FighterManager> {
    let address = crate::runtime::compat::sync::read(&crate::runtime::compat::common::FIGHTER_MANAGER_ADDR);
    if address == 0 || !address.is_multiple_of(8) {
        return None;
    }
    ptr(address).map(|p| p as *mut smash::app::FighterManager)
}

unsafe fn entry(
    manager: *mut smash::app::FighterManager,
    index: i32,
) -> Result<Option<Entry>, &'static str> {
    if !(0..8).contains(&index) {
        return Err("entry_index_bounds");
    }
    let fighter =
        FighterManager::get_fighter_entry(manager, smash::app::FighterEntryID(index)) as usize;
    if fighter == 0 {
        return Ok(None);
    }
    if !fighter.is_multiple_of(8) {
        return Err("entry_pointer_alignment");
    }
    let boid = FighterEntry::current_fighter_id(fighter as *mut smash::app::FighterEntry) as u32;
    let module = smash::app::sv_battle_object::module_accessor(boid);
    if module.is_null() {
        return Err("current_object_missing");
    }
    if (*module).battle_object_id != boid
        || !crate::runtime::compat::common::is_fighter(&*module)
        || WorkModule::get_int(module, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) != index
    {
        return Err("entry_object_mismatch");
    }
    let (ctrl, info) = core_native::controller(BASE.load(Ordering::Acquire), fighter)?;
    Ok(Some(Entry {
        index,
        boid,
        kind: smash::app::utility::get_kind(&mut *module),
        slot: read(ctrl + 8),
        mirror_slot: read(ctrl + 0x6c),
        ctrl,
        info,
        module,
    }))
}

unsafe fn pair(
    scene: Scene,
    mask: u8,
    manager: *mut smash::app::FighterManager,
) -> Result<(Binding, Entry, Entry), &'static str> {
    // Preserve the practice loader exclusion used by the original common
    // accessor; a valid-looking old object is not permission to race a swap.
    if crate::runtime::roster::character_roster::busy() {
        return Err("character_roster_loading");
    }
    let (local, peer, slot) = if scene.scope == Scope::OfflineTraining {
        (
            entry(manager, 0)?.ok_or("offline_entry0_missing")?,
            entry(manager, 1)?.ok_or("offline_entry1_missing")?,
            0,
        )
    } else {
        if mask.count_ones() != 1 {
            return Err("fim_slot_not_unique");
        }
        if FighterManager::entry_count(manager) != 2 {
            return Err("online_not_1v1");
        }
        let slot = mask.trailing_zeros() as u8;
        let mut entries: [Option<Entry>; 2] = [None, None];
        let mut count = 0;
        for index in 0..8 {
            if let Some(value) = entry(manager, index)? {
                if count == 2 {
                    return Err("online_extra_entry");
                }
                entries[count] = Some(value);
                count += 1;
            }
        }
        if count != 2 {
            return Err("online_entry_pair_missing");
        }
        let first = entries[0].ok_or("online_entry_pair_missing")?;
        let second = entries[1].ok_or("online_entry_pair_missing")?;
        let first_matches = first.slot == i32::from(slot);
        let second_matches = second.slot == i32::from(slot);
        if first_matches == second_matches {
            return Err("local_slot_not_unique_entry");
        }
        let (local, peer) = if first_matches {
            (first, second)
        } else {
            (second, first)
        };
        (local, peer, slot)
    };
    if local.boid == peer.boid {
        return Err("shared_battle_object");
    }
    if local.slot != i32::from(slot) || local.mirror_slot != i32::from(slot) {
        return Err("local_controller_slot_mismatch");
    }
    let ai = ptr(local.ctrl + 0x58).ok_or("local_ai_pointer")?;
    if read::<i32>(ai + 0x160) != i32::from(slot) {
        return Err("local_ai_slot_mismatch");
    }
    Ok((
        Binding {
            scene,
            slot,
            local_entry: local.index,
            peer_entry: peer.index,
            local_boid: local.boid,
            peer_boid: peer.boid,
            local_kind: local.kind,
            peer_kind: peer.kind,
            ctrl: local.ctrl,
            ai,
        },
        local,
        peer,
    ))
}

// Diagnostics are inside the existing default-off cpu-pilot-probe feature.
// State changes only, <=32 rows per scene, no per-frame filesystem writes.
// The line is typed JSON written into the diag ring (`Json`): no `serde_json::Value`, no heap in the hook.
fn gate<D: serde::Serialize>(state: &mut State, name: &'static str, detail: impl FnOnce() -> D) {
    // Reserve one row for the first valid pair snapshot; no JSON
    // formatting occurs for an unchanged gate or an exhausted cap.
    if name == state.last_gate || state.diagnostic_rows >= 31 {
        return;
    }
    state.last_gate = name;
    state.diagnostic_rows += 1;
    crate::engine::compat::logging::diagnostic(format_args!(
        "[CpuPilot] admission {}",
        Json(Obj((("detail", detail()), ("gate", name))))
    ));
}

/// An entry's controller and AI reads, kept as values (the keys the fork's `json!` wrote, sorted).
#[derive(Clone, Copy, serde::Serialize)]
struct EntryDetails {
    ai: Option<usize>,
    ai_flags: Option<u32>,
    ai_mode: Option<u8>,
    ai_rank: Option<i32>,
    ai_slot: Option<i32>,
    boid: u32,
    ctrl: usize,
    ctrl_loaded: u8,
    ctrl_operation: u8,
    entry: i32,
    info_operation: u8,
    kind: i32,
    mirror_slot: i32,
    slot: i32,
}

/// The pair's details at arming, for a request run's receipts.
#[derive(Clone, Copy, serde::Serialize)]
struct ArmingFlags {
    local: EntryDetails,
    peer: EntryDetails,
    service_slot_operation: u8,
}

unsafe fn entry_details(value: Entry) -> EntryDetails {
    let ai = ptr(value.ctrl + 0x58);
    EntryDetails {
        ai,
        ai_flags: ai.map(|ai| read::<u32>(ai + 0xc44)),
        ai_mode: ai.map(|ai| read::<u8>(ai + 0xc08)),
        ai_rank: ai.map(|ai| read::<i32>(ai + 0xc0c)),
        ai_slot: ai.map(|ai| read::<i32>(ai + 0x160)),
        boid: value.boid,
        ctrl: value.ctrl,
        ctrl_loaded: read::<u8>(value.ctrl + 0x69),
        ctrl_operation: read::<u8>(value.ctrl + 0x70),
        entry: value.index,
        info_operation: read::<u8>(value.info + 0x92),
        kind: value.kind,
        mirror_slot: value.mirror_slot,
        slot: value.slot,
    }
}

unsafe fn pair_diagnostic(
    state: &mut State,
    binding: Binding,
    local: Entry,
    peer: Entry,
    manager: *mut smash::app::FighterManager,
    callback_boid: u32,
) {
    if state.pair_logged {
        return;
    }
    state.pair_logged = true;
    if state.diagnostic_rows >= 32 {
        return;
    }
    state.diagnostic_rows += 1;
    let base = BASE.load(Ordering::Acquire);
    let service = ptr(base + 0x52b6fd8).and_then(|p| ptr(p));
    let legacy_player =
        crate::runtime::compat::common::try_get_module_accessor(crate::runtime::compat::consts::FighterId::Player)
            .map(|p| (*p).battle_object_id);
    let legacy_cpu = crate::runtime::compat::common::try_get_module_accessor(crate::runtime::compat::consts::FighterId::CPU)
        .map(|p| (*p).battle_object_id);
    let value = Obj((
        ("binding", binding),
        ("callback_boid", callback_boid),
        ("configured_css_entry1", Obj((
            ("byte78", read::<u8>(base + 0x52c5730 + 0x260 + 0x78)),
            ("byte79", read::<u8>(base + 0x52c5730 + 0x260 + 0x79)),
        ))),
        ("entry_count", FighterManager::entry_count(manager)),
        ("legacy_accessor_pair", Obj((("cpu", legacy_cpu), ("player", legacy_player)))),
        ("level1", "refused_unmeasured"),
        ("level9", "unmeasured_mode0_candidate100_not_admitted"),
        ("local", entry_details(local)),
        ("manager_ready_go", FighterManager::is_ready_go(manager)),
        ("manager_result", FighterManager::is_result_mode(manager)),
        ("observed_fim_mask", FIM_MASK.load(Ordering::Acquire)),
        ("peer", entry_details(peer)),
        ("read_only", true),
        ("service_mode", service.map(|p| read::<i32>(p + 0x1fc))),
        ("service_slot_operation", service.map(|p| read::<u8>(p + 0x80 + usize::from(binding.slot & 7)))),
        ("stage_ai", service.map(|p| stage_snapshot(p))),
    ));
    crate::engine::compat::logging::diagnostic(format_args!("[CpuPilot] first_pair {}", Json(value)));
}

fn reset_admission(state: &mut State, scene: Option<Scene>, result: bool) {
    if state.scene == scene && state.result == result {
        return;
    }
    clear_output();
    FIM_MASK.store(0, Ordering::Release);
    reject_pending(state, "scene_exit");
    clear_stage();
    if let Some(run) = state.active.take() {
        unsafe {
            finish(run, None, "scene_exit");
        }
    }
    state.scene = scene;
    state.result = result;
    state.polls = 0;
    state.last_gate = "";
    state.diagnostic_rows = 0;
    state.pair_logged = false;
    state.observed_binding = None;
    state.input_logged = false;
    // seen intentionally survives: an old request cannot re-arm a new match.
}

unsafe fn snapshot(
    p1: &mut BattleObjectModuleAccessor,
    cpu: *mut BattleObjectModuleAccessor,
) -> Snapshot {
    Snapshot {
        p1_x: PostureModule::pos_x(p1),
        p1_y: PostureModule::pos_y(p1),
        p1_damage: DamageModule::damage(p1, 0),
        cpu_x: PostureModule::pos_x(cpu),
        cpu_y: PostureModule::pos_y(cpu),
        cpu_damage: DamageModule::damage(cpu, 0),
    }
}

unsafe fn result_mode() -> Option<bool> {
    manager().map(|manager| FighterManager::is_result_mode(manager))
}

unsafe fn measured_ranks(level: u8) -> Result<[u8; 9], &'static str> {
    if !(1..=9).contains(&level) {
        return Err("level_out_of_range");
    }
    // run05 cpu-level1-mode0 / cpu-level9-mode0: native CPU behavior mode0,
    // controller CPU flag1, CSS+79 and live AI rank agree at0 and100.
    // Native Training Stop mode1 is not difficulty evidence (it forces100).
    // The core library owns the 13.0.5 ladder: level1 is zero and the native
    // table stores levels2 through9. Any other table refuses every level.
    core_level::ranks(core_native::rank_table(BASE.load(Ordering::Acquire)))
        .ok_or("rank_table_mismatch")
}

unsafe fn plan(
    kind: RequestKind,
    run_id: &str,
) -> Result<(u8, Option<u32>, Option<CommandRun>), &'static str> {
    match kind {
        RequestKind::Raw { rank, frames } => Ok((rank, Some(frames), None)),
        RequestKind::Command { config, direction } => {
            let _ = run_id; // Caller logs bounded admission failures with run identity.
            let ranks = measured_ranks(config.level)?;
            let pilot = pilot_core::Pilot::new(config, Some(ranks))?;
            Ok((
                pilot.rank(),
                None,
                Some(CommandRun {
                    pilot,
                    observer: MoveObserver::default(),
                    tick: 0,
                    direction,
                    first_command_frame: None,
                    last_status: None,
                    trace: Vec::with_capacity(TRACE_LIMIT),
                    trace_dropped: 0,
                }),
            ))
        }
    }
}

fn command_metrics(command: &CommandFacts, trace: &[TraceRow]) -> serde_json::Value {
    json!({"elapsed_game_frames":command.elapsed_game_frames,
        "moves":command.moves,"hits":command.hits,
        "damage_dealt":command.damage_dealt,"phase":format!("{:?}",command.phase),
        "simulation_tick":command.tick,"first_command_frame":command.first_command_frame,
        "walk_direction":command.direction.name(),
        "trace":trace.iter().map(TraceRow::value).collect::<Vec<_>>(),"trace_limit":TRACE_LIMIT,"trace_dropped":command.trace_dropped})
}

/// A run's receipt. A menu run writes its diagnostic line here; a request run's receipt goes to the heartbeat (`service` writes
/// the file and the line), with the command trace when the run has ended.
fn publish(run: &Run, reason: &'static str, trace: Vec<TraceRow>) {
    let request = match run.source {
        RunSource::Request(request) => request,
        RunSource::Menu { level } => {
            // Menu sessions are not synthetic automation requests or receipts.
            // Only arming/termination write a diagnostic; never a per-frame log.
            crate::engine::compat::logging::diagnostic(format_args!(
                "[CpuPilot] menu_control event={reason} level={level} rank={} binding={:?} rank_restore_attempted={:?}",
                run.native_rank, run.binding, run.rank_restore_attempted
            ));
            return;
        }
    };
    post(Outgoing::Receipt { facts: ReceiptFacts::of(run, request), reason, trace });
}

/// A request run's receipt as the fork wrote it (on the heartbeat).
#[inline(never)] // stream/stack: the receipt's locals live only while it is built, not under the file write
fn receipt_value(run: &ReceiptFacts, reason: &str, trace: &[TraceRow]) -> serde_json::Value {
    let request = run.request;
    json!({"run":request.run,"event":reason,
        "scope":request.scope,"binding":run.binding,"observed_fim_mask":run.observed_fim_mask,
        "arming_cpu_flags":run.arming_cpu_flags,"emit":request.emit,"native_rank":run.native_rank,
        "stage_initialization":run.stage_initialization.map(Evidence),
        "samples":run.samples,"nonneutral":run.nonneutral,"first_outputs":&*run.first_outputs,
        "start_x":run.start.p1_x,"end_x":run.end.p1_x,
        "start_damage":run.start.p1_damage,"end_damage":run.end.p1_damage,
        "start":run.start,"end":run.end,"p1_id":run.p1_id,"cpu_id":run.cpu_id,
        "ending_snapshot_source":"last_valid_bound_pair_sample",
        "operation_cpu_persisted":if run.flag_samples > 0 {Some(!run.flags_zero)} else {None},
        "operation_flags_samples":run.flag_samples,"ctrl_operation":run.last_ctrl_operation,
        "service_operation":run.last_service_operation,
        "fighter_info_operation":0,"fighter_info_operation_source":"native_gate_read_validated_zero",
        "operation_observation_scope":"gate samples only; not continuous observation",
        "shared_rng_restored_each_call":true,"pilot":request.pilot,
        "rank_restore_attempted":run.rank_restore_attempted,
        "termination_class":if reason == "replaced" {"cancel"} else {reason},
        "command":run.command.as_ref().map(|command| command_metrics(command, trace))})
}

#[inline(never)] // stream/stack, as `receipt_value`
fn rejected_value(request: RequestFacts, reason: &str, stage: Option<StageSession>) -> serde_json::Value {
    json!({"run":request.run,"event":reason,"scope":request.scope,
        "armed":false,"samples":0,"stage_initialization":stage.map(Evidence)})
}

fn write_receipt(run: RunId, reason: &str, value: &serde_json::Value) {
    let path = format!("{ROOT}/{run}-{reason}.json");
    if let Err(error) = fs::write(path, value.to_string()) {
        crate::engine::compat::logging::diagnostic(format_args!("[CpuPilot] result_write_failed {error}"));
    }
    crate::engine::compat::logging::diagnostic(format_args!("[CpuPilot] {value}"));
}

/// The fallback's file work, on the heartbeat (every 50 ms; never in a hook): the receipts the hook posted, then a read of
/// request.json when the hook asked for one (its old cadence: a scene's first admitted frame, then every 60th).
pub(crate) fn service() {
    if !installed() {
        return;
    }
    while write_next() {}
    let dropped = OUTBOX.lock().take_refused();
    if dropped > 0 {
        crate::engine::compat::logging::diagnostic(format_args!("[CpuPilot] receipts_dropped={dropped} (outbox full)"));
    }
    if READ_WANTED.swap(false, Ordering::AcqRel) {
        read_request();
    }
}

/// Write the oldest receipt the hook posted; false when there is none.
fn write_next() -> bool {
    let Some((run, reason, value)) = next_receipt() else { return false };
    write_receipt(run, reason, &value);
    true
}

/// The oldest receipt the hook posted, as its JSON; what it held is freed here, off the hook.
#[inline(never)] // stream/stack: a receipt's facts live only in this frame, never under the file write
fn next_receipt() -> Option<(RunId, &'static str, serde_json::Value)> {
    loop {
        let next = OUTBOX.lock().pop();
        match next? {
            Outgoing::Receipt { facts, reason, trace } => {
                return Some((facts.request.run, reason, receipt_value(&facts, reason, &trace)));
            }
            Outgoing::Rejected { request, reason, stage, command } => {
                drop(command);
                return Some((request.run, reason, rejected_value(request, reason, stage)));
            }
            Outgoing::Discard(command) => drop(command),
        }
    }
}

/// Read, validate and plan a new request (one whose run id differs from the last one posted) for the hook to take.
#[inline(never)] // stream/stack, as `receipt_value`
fn read_request() {
    let Some((request, kind)) = request() else { return };
    let Some(run) = RunId::of(&request.run) else { return };
    {
        let mut seen = SEEN.lock();
        if *seen == Some(run) {
            return;
        }
        *seen = Some(run);
    }
    let facts = RequestFacts {
        run,
        emit: request.emit,
        pilot: request.pilot,
        scope: request.scope,
        local_slot: request.local_slot,
    };
    // SAFETY: `plan` reads the game's rank table under the main base `init` validated (static data, any thread).
    let planned = unsafe { plan(kind, &request.run) };
    let replaced = POSTED.lock().replace(Posted { request: facts, planned });
    drop(replaced);
}

unsafe fn finish(mut run: Run, native: Option<Native>, reason: &'static str) {
    clear_output();
    if let Some(command) = run.command.as_mut() {
        if reason == "identity_changed" || reason == "scene_exit" || reason == "match_ended" {
            command.pilot.step(pilot_core::Frame {
                game_frame: command.tick,
                identity_matches: reason != "identity_changed",
                match_ended: reason != "identity_changed",
                ..pilot_core::Frame::default()
            });
        }
    }
    if let Some(n) =
        native.filter(|n| n.ctrl == run.ctrl && n.ai == run.ai && n.slot == run.binding.slot)
    {
        // Restores the saved rank and leaves the shared AI stream unchanged.
        core_native::release(BASE.load(Ordering::Acquire), n, run.old_rank);
        run.rank_restore_attempted = Some(true);
    } else {
        run.rank_restore_attempted = Some(false);
    }
    let trace = run.command.as_mut().map_or_else(Vec::new, |command| std::mem::take(&mut command.trace));
    publish(&run, reason, trace);
}

fn pause(run: &mut Run) {
    clear_output();
    if let Some(command) = run.command.as_mut() {
        command.pilot.step(pilot_core::Frame {
            game_frame: command.tick,
            identity_matches: true,
            active: false,
            ..pilot_core::Frame::default()
        });
    }
}

fn request() -> Option<(Request, RequestKind)> {
    let mut bytes = Vec::new();
    File::open(REQUEST)
        .ok()?
        .take(4097)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 4096 {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let object = value.as_object()?;
    // Presence matters too: explicit nulls cannot disguise a mixed request.
    if object.contains_key("pilot") {
        if object.contains_key("rank") || object.contains_key("frames") {
            return None;
        }
    } else if object.contains_key("walk_direction") {
        return None;
    }
    let request: Request = serde_json::from_value(value).ok()?;
    let kind = request.kind()?;
    Some((request, kind))
}

fn reject_pending(state: &mut State, reason: &'static str) {
    if let Some(RunPlan {
        source: RunSource::Request(request),
        command,
        ..
    }) = state.pending_request.take()
    {
        let stage = state.observed_binding.and_then(stage_evidence);
        post(Outgoing::Rejected { request, reason, stage, command });
    }
}

unsafe fn poll_request(state: &mut State, binding: Binding, n: Native) {
    state.polls = state.polls.wrapping_add(1);
    if state.polls % 60 == 1 {
        // The heartbeat reads the file (`service`); the hook takes what it planned.
        READ_WANTED.store(true, Ordering::Release);
    }
    let Some(Posted { request, planned }) = POSTED.try_lock().and_then(|mut posted| posted.take()) else {
        return;
    };
    if request.scope != binding.scene.scope
        || request.local_slot.is_some_and(|slot| slot != binding.slot)
    {
        if let Ok((_, _, Some(command))) = planned {
            post(Outgoing::Discard(command));
        }
        gate(
            state,
            "request_scope_or_slot_mismatch",
            || Obj((("binding", binding), ("expected_slot", request.local_slot), ("run", request.run), ("scope", request.scope))),
        );
    } else if let Ok((rank, raw_frames, command)) = planned {
        if let Some(previous) = state.active.take() {
            finish(previous, Some(n), "replaced");
        }
        reject_pending(state, "replaced");
        state.menu.request_started();
        state.pending_request = Some(RunPlan {
            source: RunSource::Request(request),
            rank,
            raw_frames,
            command,
        });
    } else {
        gate(
            state,
            "command_level_or_table_refused",
            || Obj((
                ("level", request.pilot.map(|p| p.level)),
                ("rank_mapping", "verified_css_mode0_endpoints_and_native_ladder"),
                ("run", request.run),
            )),
        );
    }
}

unsafe fn sample_flags(run: &mut Run, n: Native) {
    run.last_ctrl_operation = read(n.ctrl + 0x70);
    run.last_service_operation = read(n.service + 0x80 + usize::from(n.slot & 7));
    run.flags_zero &= run.last_ctrl_operation == 0 && run.last_service_operation == 0;
    // native() already read and validated fighter-info operation +0x92 as zero.
    run.flag_samples = run.flag_samples.saturating_add(1);
}

unsafe fn think(run: &mut Run, n: Native) -> u64 {
    // One AI decision on the run's private stream. The core call preserves all
    // mutable controller input fields (including clamped rumble) and the shared
    // stream.
    let decision = core_native::think(BASE.load(Ordering::Relaxed), n, &mut run.random);
    core_pad::pack(decision)
}

unsafe fn command_output(
    command: &mut CommandRun,
    module: &mut BattleObjectModuleAccessor,
    end: Snapshot,
) -> pilot_core::Output {
    command.tick = command.tick.saturating_add(1);
    let status = StatusModule::status_kind(module);
    let events = command.observer.step(Observation {
        game_frame: command.tick,
        status,
        prev0: StatusModule::prev_status_kind(module, 0),
        p1_damage: end.p1_damage,
        native_interrupted: None,
    });
    let previous_phase = command.pilot.phase();
    let output = command.pilot.step(pilot_core::Frame {
        game_frame: command.tick,
        identity_matches: true,
        active: true,
        match_ended: false,
        completed_moves: u32::from(events.completed.is_some()),
        infliction: AttackModule::is_infliction_status(module, *COLLISION_KIND_MASK_HIT),
        target_damage: end.cpu_damage,
        p1_lost_stock: status == *FIGHTER_STATUS_KIND_DEAD
            || status == *FIGHTER_STATUS_KIND_REBIRTH,
    });
    let completed_command = previous_phase != pilot_core::Phase::Done(pilot_core::End::Completed)
        && command.pilot.phase() == pilot_core::Phase::Done(pilot_core::End::Completed);
    if (matches!(output, pilot_core::Output::Command(_)) || completed_command)
        && command.first_command_frame.is_none()
    {
        command.first_command_frame = Some(command.tick);
        // Choose once at activation, after AI combat has moved both fighters.
        // Re-evaluating every frame could reverse us while passing the peer.
        command.direction = command.direction.resolve(end.p1_x, end.cpu_x);
    }
    if command.last_status != Some(status)
        || events.completed.is_some()
        || events.interrupted.is_some()
        || previous_phase != command.pilot.phase()
    {
        if command.trace.len() < TRACE_LIMIT {
            command.trace.push(TraceRow {
                game_frame: command.tick,
                status,
                prev0: StatusModule::prev_status_kind(module, 0),
                events,
                phase: command.pilot.phase(),
                output,
                p1_damage: end.p1_damage,
                cpu_damage: end.cpu_damage,
            });
        } else {
            command.trace_dropped = command.trace_dropped.saturating_add(1);
        }
    }
    command.last_status = Some(status);
    output
}

// Shared original native initialization for request and continuous menu owners.
unsafe fn arm(
    plan: RunPlan,
    pair: (Binding, Entry, Entry, u8),
    n: Native,
    module: &mut BattleObjectModuleAccessor,
) -> Run {
    let RunPlan {
        source,
        rank,
        raw_frames,
        command,
    } = plan;
    let (binding, local, peer, mask) = pair;
    let cpu = peer.module;
    let p1_id = binding.local_boid;
    let cpu_id = binding.peer_boid;
    // Initializes the human slot's loaded scripts at `rank` on a private copy of
    // the AI stream; the shared stream is put back before it returns.
    let (old_rank, random) = core_native::arm(BASE.load(Ordering::Acquire), n, rank);
    let start = snapshot(module, cpu);
    let mut run = Run {
        source,
        binding,
        observed_fim_mask: mask,
        arming_cpu_flags: ArmingFlags {
            local: entry_details(local),
            peer: entry_details(peer),
            service_slot_operation: read::<u8>(n.service + 0x80 + usize::from(n.slot & 7)),
        },
        stage_initialization: stage_evidence(binding),
        native_rank: rank,
        raw_frames,
        command,
        p1_id,
        cpu_id,
        ctrl: n.ctrl,
        ai: n.ai,
        old_rank,
        random,
        samples: 0,
        nonneutral: 0,
        first_outputs: FixedList::new(0),
        start,
        end: start,
        flag_samples: 0,
        flags_zero: true,
        last_ctrl_operation: 0,
        last_service_operation: 0,
        rank_restore_attempted: None,
    };
    sample_flags(&mut run, n);
    publish(&run, "armed", Vec::new());
    run
}

pub(crate) unsafe fn on_frame(module: &mut BattleObjectModuleAccessor) {
    if BASE.load(Ordering::Acquire) == 0 {
        return;
    }
    let scene = current_scene();
    let mut state = STATE.lock();
    let result = scene.is_some() && result_mode() == Some(true);
    reset_admission(&mut state, scene, result);
    let Some(scene) = scene else {
        gate(
            &mut state,
            "scope_exit",
            || Obj((("mode", DebugStr(crate::runtime::lobby::game_mode::current())),)),
        );
        return;
    };
    if result {
        gate(&mut state, "manager_result", || Obj((("scene", scene),)));
        return;
    }
    if !crate::runtime::compat::common::is_fighter(module) {
        return;
    }
    // Nana shares Popo's entry/controller; only the primary object owns a run.
    // Skip before pair lookup and gates to avoid alternating duplicate log writes.
    if smash::app::utility::get_kind(module) == *FIGHTER_KIND_NANA {
        return;
    }
    if menu_busy() {
        if let Some(run) = state.active.as_mut() {
            pause(run);
        } else {
            clear_output();
        }
        gate(&mut state, "menu_busy", || Obj((("scene", scene),)));
        return;
    }
    let Some(manager) = manager() else {
        clear_output();
        reject_pending(&mut state, "identity_changed");
        cancel_stage_initialization();
        if let Some(run) = state.active.take() {
            finish(run, None, "identity_changed");
        }
        gate(&mut state, "manager_pointer", || Obj((("scene", scene),)));
        return;
    };
    let mask = FIM_MASK.load(Ordering::Acquire) as u8;
    let pair = pair(scene, mask, manager);
    let (binding, local, peer) = match pair {
        Ok(pair) => pair,
        Err(reason) => {
            clear_output();
            reject_pending(&mut state, "identity_changed");
            cancel_stage_initialization();
            if let Some(run) = state.active.take() {
                finish(run, None, "identity_changed");
            }
            gate(
                &mut state,
                reason,
                || Obj((("fim_mask", mask), ("scene", scene))),
            );
            return;
        }
    };
    if state
        .observed_binding
        .is_some_and(|previous| previous != binding)
    {
        // A new object/slot/pointer pair must acquire fresh ownership evidence,
        // even if result/scope transitions happened between observed callbacks.
        if scene.scope == Scope::OfflineTraining {
            state.scene = None;
            reset_admission(&mut state, Some(scene), false);
        } else {
            clear_output();
            FIM_MASK.store(0, Ordering::Release);
            reject_pending(&mut state, "identity_changed");
            cancel_stage_initialization();
            if let Some(run) = state.active.take() {
                finish(run, None, "identity_changed");
            }
            state.observed_binding = None;
        }
        gate(
            &mut state,
            "pair_identity_changed",
            || Obj((("binding", binding),)),
        );
        return;
    }
    state.observed_binding = Some(binding);
    // First pair diagnostic is read-only and precedes any request/native setup.
    pair_diagnostic(
        &mut state,
        binding,
        local,
        peer,
        manager,
        module.battle_object_id,
    );
    let native = native(local, binding.slot);
    if let Some(run) = state.active.take() {
        if run.binding != binding
            || !native
                .as_ref()
                .is_ok_and(|n| n.ctrl == run.ctrl && n.ai == run.ai && n.slot == run.binding.slot)
        {
            reject_pending(&mut state, "identity_changed");
            cancel_stage_initialization();
            finish(run, native.ok(), "identity_changed");
            gate(
                &mut state,
                "binding_changed",
                || Obj((("binding", binding), ("fim_mask", mask))),
            );
            return;
        }
        state.active = Some(run);
    }
    // All fighters can call this hook: only the exact owned current object advances.
    if module.battle_object_id != binding.local_boid {
        if WorkModule::get_int(module, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID)
            == binding.local_entry
        {
            gate(
                &mut state,
                "current_object_filter",
                || Obj((("actual", module.battle_object_id), ("expected", binding.local_boid))),
            );
        }
        return;
    }
    let n = match native {
        Ok(n) => n,
        Err(reason) => {
            clear_output();
            reject_pending(&mut state, "identity_changed");
            cancel_stage_initialization();
            gate(&mut state, reason, || Obj((("binding", binding),)));
            return;
        }
    };
    let request_active = state.pending_request.is_some()
        || state
            .active
            .as_ref()
            .is_some_and(|run| matches!(&run.source, RunSource::Request(_)));
    let desired_level = state
        .menu
        .observe(MENU_DESIRED.load(Ordering::Acquire), request_active);
    if state.active.as_ref().is_some_and(|run| {
        run.source.menu_level().is_some() && run.source.menu_level() != desired_level
    }) {
        let previous = state.active.take().unwrap();
        finish(
            previous,
            Some(n),
            if desired_level.is_some() {
                "menu_level_changed"
            } else {
                "menu_off"
            },
        );
    }
    // Named SDK query is the same predicate used by common::is_ready_go, with
    // a freshly validated manager instead of an unchecked global dereference.
    if !FighterManager::is_ready_go(manager) {
        if let Some(run) = state.active.as_mut() {
            pause(run);
        } else {
            clear_output();
        }
        gate(
            &mut state,
            "manager_not_ready_go",
            || Obj((("binding", binding),)),
        );
        return;
    }
    if scene.scope == Scope::LanOnline1v1 {
        // A validated request can now authorize initialization while phase is 1.
        // GO/menu/identity guards still precede polling, as in the original path.
        poll_request(&mut state, binding, n);
        let request_owner = state.pending_request.is_some()
            || state
                .active
                .as_ref()
                .is_some_and(|run| matches!(&run.source, RunSource::Request(_)));
        let menu_level = state
            .menu
            .observe(MENU_DESIRED.load(Ordering::Acquire), request_owner);
        let intent = if request_owner {
            StageIntent::Request
        } else if let Some(level) = menu_level {
            match measured_ranks(level) {
                Ok(_) => StageIntent::Menu,
                Err(reason) => {
                    gate(&mut state, reason, || Obj((("menu_level", level),)));
                    StageIntent::None
                }
            }
        } else {
            StageIntent::None
        };
        match stage_admission(binding, local, peer, n, intent) {
            Ok(Some(session)) if session.progress.status == stage_session::Status::Failed => {
                clear_output();
                reject_pending(&mut state, "stage_ai_not_ready");
                gate(&mut state, "stage_ai_not_ready", || Evidence(session));
                return;
            }
            Err(reason) => {
                reject_pending(&mut state, reason);
                if let Some(run) = state.active.take() {
                    finish(run, None, "identity_changed");
                }
                clear_stage();
                clear_output();
                gate(&mut state, reason, || Obj((("binding", binding),)));
                return;
            }
            Ok(Some(session)) if session.progress.status == stage_session::Status::Pending => {
                clear_output();
                gate(&mut state, "stage_ai_pending", || Evidence(session));
                return;
            }
            // ready() below remains the sole native AI readiness gate. This
            // snapshot is carried into the armed and final request receipts.
            _ => {}
        }
    }
    let cpu = peer.module;
    // Native AI may be inactive at DEAD/REBIRTH. Let an already commanding
    // pilot observe its terminal stock loss before the AI-ready gate pauses it.
    // Pair/object identity, menu, result and ready-go guards have all passed.
    let status = StatusModule::status_kind(module);
    if status == *FIGHTER_STATUS_KIND_DEAD || status == *FIGHTER_STATUS_KIND_REBIRTH {
        let completed = if let Some(run) = state.active.as_mut() {
            if let Some(command) = run
                .command
                .as_mut()
                .filter(|command| command.pilot.phase() == pilot_core::Phase::Commanding)
            {
                run.end = snapshot(module, cpu);
                command_output(command, module, run.end);
                command.pilot.phase() == pilot_core::Phase::Done(pilot_core::End::Completed)
            } else {
                false
            }
        } else {
            false
        };
        if completed {
            let run = state.active.take().unwrap();
            finish(run, Some(n), "completed");
            return;
        }
    }
    // The core ready gate is the sole native AI readiness check.
    if let Err(reason) = core_native::ready(n) {
        if let Some(run) = state.active.as_mut() {
            pause(run);
        } else {
            clear_output();
        }
        gate(&mut state, reason, || Obj((("binding", binding),)));
        return;
    }
    gate(
        &mut state,
        "admission_ready",
        || Obj((("binding", binding), ("fim_mask", mask), ("stage_initialization", stage_evidence(binding).map(Evidence)))),
    );
    if scene.scope == Scope::OfflineTraining {
        // Preserve offline request polling after every original ready gate.
        poll_request(&mut state, binding, n);
    }
    if let Some(plan) = state.pending_request.take() {
        state.active = Some(arm(plan, (binding, local, peer, mask), n, module));
    }
    // Requests are polled first and inhibit menu ownership at acceptance. The
    // local desired value predates polling, so check arbitration again here.
    if state.active.is_none() {
        if let Some(level) = state
            .menu
            .observe(MENU_DESIRED.load(Ordering::Acquire), false)
        {
            match measured_ranks(level) {
                Ok(ranks) => {
                    state.active = Some(arm(
                        RunPlan {
                            source: RunSource::Menu { level },
                            rank: ranks[usize::from(level - 1)],
                            raw_frames: None,
                            command: None,
                        },
                        (binding, local, peer, mask),
                        n,
                        module,
                    ));
                }
                Err(reason) => gate(&mut state, reason, || Obj((("menu_level", level),))),
            }
        }
    }
    let Some(run) = state.active.as_mut() else {
        return;
    };
    run.end = snapshot(module, cpu);
    sample_flags(run, n);
    let output = if let Some(command) = run.command.as_mut() {
        command_output(command, module, run.end)
    } else {
        pilot_core::Output::Ai
    };
    let decision = match output {
        pilot_core::Output::Ai => think(run, n),
        pilot_core::Output::Command(pilot_core::Command::WalkOff) => {
            let x = run.command.as_ref().unwrap().direction.axis();
            core_pad::pack(core_pad::from_ai(
                0,
                x,
                if crate::runtime::compat::common::is_grounded(module) {
                    0.0
                } else {
                    -1.0
                },
            ))
        }
        pilot_core::Output::Pad | pilot_core::Output::Command(pilot_core::Command::Release) => {
            let run = state.active.take().unwrap();
            finish(run, Some(n), "completed");
            return;
        }
    };
    run.samples = run.samples.saturating_add(1);
    if decision & !0x8000 != 0 {
        run.nonneutral = run.nonneutral.saturating_add(1);
        if run.first_outputs.len() < 24 {
            run.first_outputs.push(decision);
        }
    }
    // Recheck ownership after native think; FIM can observe another slot concurrently.
    if run.binding.scene.scope == Scope::LanOnline1v1
        && FIM_MASK.load(Ordering::Acquire) != 1usize << run.binding.slot
    {
        let run = state.active.take().unwrap();
        finish(run, Some(n), "identity_changed");
        return;
    }
    EMIT_SLOT.store(usize::from(run.binding.slot), Ordering::Release);
    EMIT_SCENE.store(run.binding.scene.key(), Ordering::Release);
    PENDING.store(decision, Ordering::Release);
    EMITTING.store(run.source.emit(), Ordering::Release);
    if run.raw_frames.is_some_and(|frames| run.samples >= frames) {
        let run = state.active.take().unwrap();
        finish(run, Some(n), "completed");
    }
}

pub(crate) unsafe fn input(player_idx: i32, out: *mut MappedInputs) {
    if BASE.load(Ordering::Acquire) == 0 {
        return;
    }
    let scene = current_scene();
    let result = scene.is_some() && result_mode() == Some(true);
    let mut state_guard = STATE.try_lock();
    if let Some(state) = state_guard.as_mut() {
        reset_admission(state, scene, result);
    }
    // Observe only exact online battle scope, never waiting practice or results.
    if scene.is_some_and(|s| s.scope == Scope::LanOnline1v1)
        && !result
        && (0..8).contains(&player_idx)
    {
        FIM_MASK.fetch_or(1usize << player_idx, Ordering::AcqRel);
    } else if scene.is_none() || result || scene.is_some_and(|s| s.scope != Scope::LanOnline1v1) {
        FIM_MASK.store(0, Ordering::Release);
    }
    if let Some(state) = state_guard.as_mut() {
        // This proves FIM reaches the feature even if an upstream fighter-hook
        // owner guard returns before this module's on_frame admission checks.
        if scene.is_some() && !state.input_logged {
            state.input_logged = true;
            if state.diagnostic_rows < 31 {
                state.diagnostic_rows += 1;
                crate::engine::compat::logging::diagnostic(format_args!(
                    "[CpuPilot] first_input scene={scene:?} slot={player_idx} fim_mask={}",
                    FIM_MASK.load(Ordering::Acquire)
                ));
            }
        }
    }
    drop(state_guard);
    let Some(scene) = scene.filter(|_| !result) else {
        clear_output();
        return;
    };
    let mask = FIM_MASK.load(Ordering::Acquire);
    if scene.scope == Scope::LanOnline1v1 && mask.count_ones() != 1 {
        clear_output();
        if let Some(mut state) = STATE.try_lock() {
            reject_pending(&mut state, "identity_changed");
            cancel_stage_initialization();
            if let Some(run) = state.active.take() {
                finish(run, None, "identity_changed");
            }
        }
        return;
    }
    if menu_busy() {
        clear_output();
        if let Some(mut state) = STATE.try_lock() {
            if let Some(run) = state.active.as_mut() {
                pause(run);
            }
        }
        return;
    }
    if EMIT_SCENE.load(Ordering::Acquire) != scene.key() {
        clear_output();
        return;
    }
    if !(0..8).contains(&player_idx) || EMIT_SLOT.load(Ordering::Acquire) != player_idx as usize {
        return;
    }
    if scene.scope == Scope::LanOnline1v1 && mask != 1usize << player_idx {
        clear_output();
        return;
    }
    if EMITTING.load(Ordering::Acquire) {
        const _: () = assert!(std::mem::size_of::<MappedInputs>() == 8);
        *out = std::mem::transmute::<u64, MappedInputs>(PENDING.load(Ordering::Acquire));
    }
}

/// Seam-table rows (`native_seams::rows_a1`, group `cpu_pilot`): the one hook this module installs and
/// the three game functions ssbu-cpu-pilot-core's `native` calls (think, set_rank, AI reset). The core
/// library leaves the version and 64-byte guards to its host (`core/src/native.rs`); the version guard
/// runs in `main` before `init` (WP-B). Shadow until the group is flipped: the body checks in `init`
/// still decide.
const SEAMS: &[&str] = &[
    "cpu_pilot.ai_process_begin",
    "cpu_pilot.ai_think",
    "cpu_pilot.ai_set_rank",
    "cpu_pilot.ai_reset",
];

// The standalone libssbu_cpu_pilot.nro exports its state when it is installed.
// It then owns P1, so this module must not drive P1 a second time.
fn standalone_pilot_export() -> Option<usize> {
    let mut address = 0usize;
    unsafe {
        skyline::nn::ro::LookupSymbol(&mut address, b"ssbu_cpu_pilot_state_v1\0".as_ptr());
    }
    (address != 0).then_some(address)
}

pub(crate) fn init() {
    let base = unsafe { getRegionAddress(Region::Text) as usize };
    crate::engine::compat::logging::diagnostic(format_args!("[CpuPilot] main_base {base:#x}"));
    // Coexistence: with the standalone plugin installed, install no pilot hook.
    // BASE stays zero, so on_frame and input never act and P1 is driven once.
    if let Some(address) = standalone_pilot_export() {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[CpuPilot] standalone_pilot_present ssbu_cpu_pilot_state_v1={address:#x}; fork pilot hooks not installed"
        ));
        return;
    }
    // Body bytes remain stable even after the existing set_cpu_controls hook installs.
    let expected: &[(usize, &[u8])] = &[
        (0x6b8420, &[0xa8, 0x03, 0x00, 0xb4, 0x95, 0x2e, 0x40, 0xf9]),
        (0x2da1a0, &[0xf6, 0x57, 0x15, 0xa9, 0xf4, 0x4f, 0x16, 0xa9]),
        // Owned 13.0.5 ELF, process_begin +0x30; beyond hook-patched entry bytes.
        (
            0x30c740,
            &[
                0x08, 0x24, 0x40, 0x39, 0x68, 0x94, 0x02, 0x34, 0x08, 0x00, 0x40, 0xf9, 0x09, 0xfd,
                0x41, 0xb9,
            ],
        ),
    ];
    let legacy = expected.iter().all(|&(offset, bytes)| {
        let stock =
            unsafe { std::slice::from_raw_parts((base + offset) as *const u8, bytes.len()) } == bytes;
        if !stock {
            crate::engine::compat::logging::diagnostic(format_args!(
                "[CpuPilot] unsupported_native_body {offset:#x}"
            ));
        }
        stock
    });
    // The 64-byte guards of the hooked site and of every game function the core library calls
    // (C116.1 seam table; shadow until the `cpu_pilot` group is flipped, then they decide).
    if !crate::runtime::seams::seam_install::gate("cpu_pilot", SEAMS, legacy) {
        if legacy {
            crate::engine::compat::logging::diagnostic(format_args!("[CpuPilot] seam guard refused"));
        }
        return;
    }
    if fs::create_dir_all(ROOT).is_err() {
        return;
    }
    BASE.store(base, Ordering::Release);
    skyline::install_hooks!(ai_process_begin);
    crate::engine::compat::logging::diagnostic(format_args!("[CpuPilot] offline_probe_ready"));
}
