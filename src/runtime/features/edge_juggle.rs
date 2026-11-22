//! Native Training adapter for the edge and juggle games. No additional game hooks.
use std::fmt::Write;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

use hos_lock::{Mutex, Published};

use super::{cpu::konst, mashout::FloorRay, notices, placement, practice, recorder, states};
use crate::model::btt::Pad;
use crate::model::chords::{field, logical as key, Sample};
use crate::model::crosshair::Marks;
use crate::model::cpu::{
    ids::{C, UNRESOLVED},
    rng::Rng,
    role::FighterKey,
};
use crate::model::edge_juggle::perfect::game_watch::{self, OilInputs, OilStall};
use crate::model::edge_juggle::perfect::recovery::{
    self, Guarder, Hold, HoldFacts, LedgeWatch, Reach, Zone,
};
use crate::model::edge_juggle::perfect::rule_counts::{Counters, PerfectRule};
use crate::model::edge_juggle::perfect::{
    self, AirPhysics, Correction, LandingFacts, LandingOptions, MeteorBounce, MeteorWindow,
    PlanHeight, PlanInput, Route, RouteFacts, ThreatClock,
};
use crate::model::edge_juggle::picker::{self, DrillId, Mode, Picker};
use crate::model::edge_juggle::{
    CpuObject, Drill, Observation, Outcome, Phase, PlacedPlayer, Placement, PlacementAction, PlacementFrame,
    Reaction, Session, Setup, Stage, Wizard, WizardStep,
};

#[path = "edge_juggle/zoom.rs"]
mod zoom;
#[path = "edge_juggle/practice_cpu.rs"]
pub(in crate::runtime) mod practice_cpu;
use crate::model::notify::DEFAULT_COLOR;
use crate::model::practice_games::best_time;
use crate::model::settings::Snapshot;
use crate::model::storage::FixedPath;
use crate::runtime::{
    game::{self, natives},
    published, scene, state_natives as state,
};

#[path = "edge_juggle/niche_pad.rs"]
mod niche_pad;
#[path = "edge_juggle/startup_dodge.rs"]
mod startup_dodge;

const NOTICE: u8 = 0x34;
const JUGGLE_PERCENT: u16 = 60;
type StageKey = (i32, u32);
type BestKey = (Drill, i32, u8, Mode);

#[derive(Clone, Copy)]
struct Opponent {
    fighter: Option<u32>,
    mode: Mode,
    window: u8,
}

impl Opponent {
    const DEFAULT: Self = Self {
        fighter: Picker::DEFAULT.fighter,
        mode: Mode::Cpu(Picker::DEFAULT.level),
        window: picker::DEFAULT_WINDOW,
    };
}

static OPPONENT: Published<Opponent> = Published::new(Opponent::DEFAULT);

#[derive(Clone, Copy)]
struct Config {
    edge_percent: u16,
    edge_limit: Option<u16>,
    juggle_limit: Option<u16>,
    opponent: [Opponent; 2],
    reaction: [Reaction; 2],
}
static CONFIG: Published<Config> = Published::new(Config {
    edge_percent: 100,
    edge_limit: Some(10),
    juggle_limit: Some(10),
    opponent: [Opponent::DEFAULT; 2],
    reaction: [Reaction::Human; 2],
});
static INSTALLED: AtomicBool = AtomicBool::new(false);
static P1_BUTTONS: AtomicU32 = AtomicU32::new(0);
static START: AtomicU8 = AtomicU8::new(0);
static ENGINE_ACTIVE: AtomicBool = AtomicBool::new(false);
static RESTART: AtomicBool = AtomicBool::new(false);
static WIZARD: AtomicBool = AtomicBool::new(false);
static OPEN: AtomicBool = AtomicBool::new(false);
static BACK: AtomicBool = AtomicBool::new(false);

struct Engine {
    run: Option<Run>,
    kept: [Option<(StageKey, Setup, bool)>; 2],
    bests: [Option<(BestKey, u32)>; 24],
    streaks: [Option<(BestKey, u32)>; 24],
    held_cpu: Option<CpuBody>,
    rng: Rng,
}
static ENGINE: Mutex<Engine> = Mutex::new(Engine {
    run: None,
    kept: [None; 2],
    bests: [None; 24],
    streaks: [None; 24],
    held_cpu: None,
    rng: Rng::new(0),
});

struct Run {
    drill: Drill,
    opponent: Opponent,
    zoom: zoom::NativeAttempt,
    reaction: Reaction,
    threat: ThreatClock,
    stage_key: StageKey,
    stage: Stage,
    stage_floor_y: f32,
    wizard: Option<Wizard>,
    defaults: Setup,
    confirm_player: bool,
    setup: Setup,
    rounds: Session,
    body: CpuBody,
    previous_damage: Option<f32>,
    damage_stop: bool,
    landing_draw: u32,
    jump_delay: u8,
    route_draw: u32,
    hold: Hold,
    zone: Zone,
    ledge_watch: LedgeWatch,
    plan_pending: bool,
    route_watch: perfect::RouteWatch,
    player_reach: f32,
    player_height: PlanHeight,
    route_history: recovery::RouteHistory,
    guide: Option<RecoveryGuide>,
    niche: Option<niche_pad::NicheGuide>,
    oil_stall: OilStall,
    oil_draw: [u32; 2],
    last_cpu_status: i32,
    bounce: MeteorBounce,
    sdi_start: Option<u32>,
    player_ledge: Option<usize>,
    placement: Placement,
    /// The point a watched start write wrote for Player 1; None when the write found it idle.
    settle: Option<[f32; 2]>,
    notice: FixedPath<128>,
}
#[derive(Clone, Copy)]
struct RecoveryGuide {
    lip: [f32; 2],
    inward: f32,
}

/// Equality tokens from the current entry. Only a fresh body may be accessed.
#[derive(Clone, Copy, PartialEq, Eq)]
struct CpuBody {
    accessor: usize,
    object_id: u32,
}

impl CpuBody {
    unsafe fn current() -> Option<Self> {
        let accessor = natives::module_accessor(1)?;
        natives::is_fighter(accessor).then(|| Self {
            accessor,
            object_id: natives::battle_object_id(accessor),
        })
    }
}

struct Bodies {
    p1: usize,
    cpu: usize,
}

static SPECIAL_TABLE: AtomicBool = AtomicBool::new(false);
static RECOVERY_BASE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn install(dependencies: bool) -> bool {
    // Surface verification may ask for a memory map. Run it before the game callbacks use its result.
    let installed = dependencies
        && super::cpu_level::install_drill_behavior()
        && unsafe { super::tech_surfaces::init() };
    if installed
        && crate::runtime::seams::seam_install::gate(
            "edge_juggle",
            &[
                "edge_juggle.special_available_snapshot",
                "edge_juggle.special_available",
                "edge_juggle.escape_slide_distance",
                "edge_juggle.escape_cancel_frame",
            ],
            false,
        )
    {
        RECOVERY_BASE.store(game::text_base(), Ordering::Release);
    }
    let special_table = installed
        && crate::runtime::seams::seam_install::gate(
            "edge_juggle",
            &["edge_juggle.special_height", "edge_juggle.special_length"],
            false,
        );
    startup_dodge::install(installed);
    SPECIAL_TABLE.store(special_table, Ordering::Release);
    if installed {
        zoom::install();
    }
    INSTALLED.store(installed, Ordering::Release);
    installed
}

pub fn init(seed: u64) {
    ENGINE.lock().rng = Rng::new(seed);
}

pub fn publish(snapshot: &Snapshot) {
    let select = |id| crate::model::tech_settings::selected(snapshot, id);
    let limit = |row| {
        [Some(10), Some(20), None]
            .get(select(row))
            .copied()
            .flatten()
    };
    CONFIG.publish(Config {
        edge_percent: select("edge_cpu_percent").min(300) as u16,
        edge_limit: limit("edge_attempts"),
        juggle_limit: limit("juggle_attempts"),
        opponent: [Drill::GuardEdge, Drill::Juggler].map(|drill| {
            let picker = Picker::read(snapshot, drill);
            Opponent {
                fighter: picker.fighter,
                mode: picker.mode(),
                window: (select(picker::rows(drill).window).min(29) + 1) as u8,
            }
        }),
        reaction: [Drill::GuardEdge, Drill::Juggler].map(|drill| {
            let rows = drill.difficulty_rows();
            Reaction::from_settings(select(rows.reaction), select(rows.frames))
        }),
    });
}

pub fn start(edit: bool) {
    if let Some(drill) = wanted() {
        OPPONENT.publish(CONFIG.read().opponent[drill as usize]);
    }
    super::cpu_level::drill_started();
    P1_BUTTONS.store(0, Ordering::Release);
    RESTART.store(false, Ordering::Release);
    START.store(if edit { 2 } else { 1 }, Ordering::Release);
}

pub fn menu_opened() {
    OPEN.store(true, Ordering::Release);
    notices::clear(NOTICE);
}

pub fn menu_closed() {
    OPEN.store(false, Ordering::Release);
}

pub fn restart() {
    if practice::active().and_then(Drill::from_game).is_some() {
        RESTART.store(true, Ordering::Release);
    }
}

pub fn take_menu_request() -> bool {
    BACK.swap(false, Ordering::AcqRel)
}

pub fn wizard_active() -> bool {
    WIZARD.load(Ordering::Acquire) && wanted().is_some()
}

pub(super) fn placement_active() -> bool {
    !OPEN.load(Ordering::Acquire) && wizard_active()
}

pub fn allowed(drill: Drill) -> bool {
    if !INSTALLED.load(Ordering::Acquire) || !scene::is_native_training() {
        return false;
    }
    if super::recording_host::cpu_playback_blocks_drill() {
        notices::refuse("Stop CPU playback before starting this drill"); // restriction: edge-juggle-cpu-playback-busy
        return false;
    }
    !matches!(CONFIG.read().opponent[drill as usize].mode, Mode::Drill(DrillId::Zoom))
        || zoom::installed()
}

fn wanted() -> Option<Drill> {
    practice::active()
        .and_then(Drill::from_game)
        .filter(|_| scene::is_native_training())
}

pub(super) fn running() -> bool {
    wanted().is_some()
}

pub(super) fn stock_level() -> Option<u8> {
    running().then(|| OPPONENT.read().mode.stock_level())
}

pub(super) fn cpu_selection() -> Option<u32> {
    let drill = wanted()?;
    if START.load(Ordering::Acquire) == 0 {
        if let Some(engine) = ENGINE.try_lock() {
            if let Some(run) = engine.run.as_ref().filter(|run| run.drill == drill) {
                return run.opponent.fighter;
            }
        }
    }
    OPPONENT.read().fighter
}

pub unsafe fn controls_cpu(accessor: usize) -> bool {
    if !INSTALLED.load(Ordering::Acquire) {
        return false;
    }
    running()
        && !published::HOT_SWAP_IN_PROGRESS.load(Ordering::Acquire)
        && natives::module_accessor(1) == Some(accessor)
}

pub(super) fn suspends_cpu_settings(key: FighterKey) -> bool {
    let active = INSTALLED.load(Ordering::Acquire)
        && !published::HOT_SWAP_IN_PROGRESS.load(Ordering::Acquire)
        && running();
    crate::model::edge_juggle::suspends_cpu_settings(active, key)
}

/// The final mapped pad read by P1, after the existing delay and playback stages.
pub(super) fn observe_player_pad(buttons: u32) {
    if running() {
        P1_BUTTONS.store(buttons, Ordering::Release);
    }
}

pub(super) fn may_request_player_height() -> bool {
    ENGINE_ACTIVE.load(Ordering::Acquire)
        && !OPEN.load(Ordering::Acquire)
        && !published::HOT_SWAP_IN_PROGRESS.load(Ordering::Acquire)
        && wanted() == Some(Drill::GuardEdge)
}

/// Issued only to the primary P1 frame of a live Perfect recovery plan.
pub(super) struct LivePerfect {
    player: usize,
    frame: u32,
}

impl LivePerfect {
    pub(super) fn player(&self) -> usize {
        self.player
    }

    pub(super) fn record_height(self, height: Option<f32>) {
        let Some(mut engine) = ENGINE.try_lock() else { return };
        if let Some(run) = engine.run.as_mut().filter(|run| {
            run.rounds.phase == Phase::Live && run.plan_pending
        }) {
            run.player_height.record(self.frame, height);
        }
    }
}

pub(super) unsafe fn player_height_request(player: usize) -> Option<LivePerfect> {
    if !INSTALLED.load(Ordering::Acquire)
        || states::is_loading()
        || natives::module_accessor(0) != Some(player)
        || !natives::is_fighter(player)
    {
        return None;
    }
    let mut engine = ENGINE.try_lock()?;
    let run = engine.run.as_mut().filter(|run| {
        wanted() == Some(run.drill)
            && run.opponent.mode == Mode::Perfect
            && run.rounds.phase == Phase::Live
            && run.wizard.is_none()
            && run.plan_pending
    })?;
    let cpu = run.body.accessor;
    let actionable = CpuBody::current() == Some(run.body) && airborne_recovery(cpu, natives::status_kind(cpu));
    let frame = run.rounds.frames;
    run.player_height.request(frame, actionable).then_some(LivePerfect { player, frame })
}

fn falling(status: i32) -> bool {
    status != UNRESOLVED
        && [C::StatusFall, C::StatusFallAerial, C::StatusDamageFall]
            .map(konst)
            .contains(&status)
}

unsafe fn airborne_recovery(cpu: usize, status: i32) -> bool {
    let air = konst(C::SituationAir);
    status != UNRESOLVED
        && air != UNRESOLVED
        && (falling(status) || status == konst(C::StatusJumpAerial))
        && natives::situation_kind(cpu) == air
}

/// The stock values seen before the controller write in the existing AI epilogue.
#[derive(Clone, Copy)]
pub(super) struct StockPad {
    pub buttons: u32,
    pub stick: [f32; 2],
    pub clamped_x: f32,
}

static PLANNER_EVENTS: AtomicU32 = AtomicU32::new(0);

/// Release trace boundary for the route taken after a complete plan.
#[inline(never)]
#[no_mangle]
pub extern "C" fn planner_chose(route: u32, fighter: u32, crossable: u32, row: u32) {
    std::hint::black_box((route, fighter, crossable, row));
    PLANNER_EVENTS.fetch_add(1, Ordering::Relaxed);
}

/// A single rejected plan sample; the adjacent stock AI trace identifies the fighter.
#[inline(never)]
#[no_mangle]
pub extern "C" fn planner_unavailable(input: PlanInput) {
    std::hint::black_box(input);
    PLANNER_EVENTS.fetch_add(1, Ordering::Relaxed);
}

pub static PERFECT_CORRECTIONS: Counters = Counters::new();

/// Plain release trace boundary for actual controller edits. No I/O in the AI hook.
#[inline(never)]
#[no_mangle]
pub extern "C" fn perfect_rule_fired(rule: PerfectRule, cpu_frame: u32, trigger_frame: u32, detail: u32) {
    std::hint::black_box((rule, cpu_frame, trigger_frame, detail));
    PERFECT_CORRECTIONS.record(rule);
}

/// Read by the release tracer at the two boundaries of a scored attempt.
#[inline(never)]
#[no_mangle]
pub extern "C" fn edge_juggle_attempt(meta: u64, clock_damage: u64, cpu: u64, p1: u64) {
    std::hint::black_box((meta, clock_damage, cpu, p1));
}

unsafe fn trace_attempt(
    run: &Run,
    before: Phase,
    bodies: Bodies,
    kind: i32,
    damage: Option<f32>,
) {
    let Some((ordinal, event)) = run.rounds.trace_event(before) else {
        return;
    };
    use crate::model::sites_pkg2 as sites;
    if !game::granted(sites::POS_X_OFFSET) || !game::granted(sites::POS_Y_OFFSET) {
        return;
    }
    let pack = |low, high| u64::from(low) | (u64::from(high) << 32);
    let position = |accessor| pack(natives::pos_x(accessor).to_bits(), natives::pos_y(accessor).to_bits());
    let meta = u64::from(ordinal)
        | (u64::from(event) << 16)
        | (u64::from(run.drill == Drill::Juggler) << 24)
        | (u64::from(run.opponent.mode == Mode::Perfect) << 25)
        | (u64::from(run.stage_key.1 & 0xf) << 28)
        | (u64::from(kind as u16) << 32)
        | (u64::from(run.stage_key.0 as u16) << 48);
    edge_juggle_attempt(
        meta,
        pack(damage.unwrap_or(f32::NAN).to_bits(), run.rounds.frames),
        position(bodies.cpu),
        position(bodies.p1),
    );
}

impl StockPad {
    fn observe(
        self,
        edit: Correction,
        rule: PerfectRule,
        run: &Run,
        trigger: u32,
        detail: u32,
    ) -> Correction {
        let next_buttons = (self.buttons & !edit.release) | edit.press;
        let changed_stick = edit.stick.is_some_and(|[x, y]| {
            x.to_bits() != self.stick[0].to_bits()
                || x.to_bits() != self.clamped_x.to_bits()
                || y != self.stick[1]
        });
        if next_buttons != self.buttons || changed_stick {
            perfect_rule_fired(rule, run.rounds.frames, trigger, detail);
        }
        edit
    }
}

pub(super) unsafe fn ai_correction(ai: usize, accessor: usize, stock: StockPad) -> Correction {
    let buttons = stock.buttons;
    if !INSTALLED.load(Ordering::Acquire)
        || OPEN.load(Ordering::Acquire)
        || published::HOT_SWAP_IN_PROGRESS.load(Ordering::Acquire)
        || states::is_loading()
        || START.load(Ordering::Acquire) != 0
    {
        return Correction::default();
    }
    let Some(mut engine) = ENGINE.try_lock() else {
        return Correction::default();
    };
    let Some(run) = engine.run.as_mut().filter(|run| wanted() == Some(run.drill)) else {
        return Correction::default();
    };
    use crate::model::sites_pkg2 as sites;
    if run.body.accessor != accessor
        || natives::battle_object_id(accessor) != run.body.object_id
        || ![
            sites::STATUS_KIND_OFFSET,
            sites::POS_X_OFFSET,
            sites::POS_Y_OFFSET,
        ]
        .into_iter()
        .all(game::granted)
    {
        return Correction::default();
    }
    // A live editor gets its neutral pad from the shared placement session.
    if run.wizard.is_some() {
        return Correction::default();
    }
    if let Some(hold) = run.rounds.phase.held_pad(buttons) {
        return hold;
    }
    if run.rounds.phase != Phase::Live {
        return Correction::default();
    }
    match run.opponent.mode {
        Mode::Drill(DrillId::Zoom) => return run.zoom.zoom_pad(accessor, buttons),
        Mode::Cpu(_) => return Correction::default(),
        Mode::Perfect => {}
    }
    let status = natives::status_kind(accessor);
    let Some(p1) = natives::module_accessor(0) else {
        return Correction::default();
    };
    if run.drill == Drill::Juggler {
        let reflect = konst(C::StatusDamageFlyReflectD);
        let previous = std::mem::replace(&mut run.last_cpu_status, status);
        let bounce_started = status != UNRESOLVED
            && status == reflect
            && previous != UNRESOLVED
            && previous != reflect;
        if bounce_started || run.bounce.awaiting_dodge() {
            let escape = konst(C::TermContEscapeAir);
            let air = konst(C::SituationAir);
            let continued = status != UNRESOLVED
                && (status == reflect || airborne_recovery(accessor, status));
            let window = if !continued || escape == UNRESOLVED || air == UNRESOLVED {
                MeteorWindow::Interrupted
            } else if natives::situation_kind(accessor) == air
                && natives::is_enable_transition_term(accessor, escape)
            {
                MeteorWindow::Actionable
            } else {
                MeteorWindow::Waiting
            };
            let correction = run.bounce.step(
                bounce_started,
                window,
                natives::status_kind(p1) == konst(C::StatusWait)
                    && P1_BUTTONS.load(Ordering::Acquire) == 0,
                buttons,
            );
            if correction != Correction::default() {
                return stock.observe(
                    correction, PerfectRule::Meteor, run, 0, run.reaction.frames().into(),
                );
            }
        }
    }
    let position = [natives::pos_x(accessor), natives::pos_y(accessor)];
    if run.drill == Drill::GuardEdge {
        if run.niche.as_ref().is_some_and(niche_pad::NicheGuide::using_bomb) {
            return niche_correction(run, ai, accessor, p1, position, status, stock);
        }
        // Hit off-stage: survive first, otherwise bend the launch at the nearer ledge.
        // J3's counter stays the Juggler's; this edit has no trace tag until one is agreed.
        let [left, right] = run.stage.ledges;
        if position[0] < left[0] || position[0] > right[0] {
            let ledge = if position[0] < left[0] { left } else { right };
            let anchor = recovery::di_anchor(position, ledge);
            let correction = air_hit_recovery(run, anchor, accessor, position, status);
            if correction != Correction::default() {
                return correction;
            }
        }
        if run.niche.is_some() {
            return niche_correction(run, ai, accessor, p1, position, status, stock);
        }
        if run.oil_stall.active() {
            // Transition terms may be unavailable in the active stance.
            if recovery_snapshot(ai).is_some_and(|(snapshot, available)| {
                !available || special_reach(ai, snapshot).is_some()
            }) {
                if let Some(pad) = run.oil_stall.sustain(status, run.rounds.frames) {
                    return stock.observe(pad, PerfectRule::Niche, run, 0, game_watch::ROW.into());
                }
                run.plan_pending = true;
            } else {
                run.oil_stall.cancel();
                return Correction::default();
            }
        }
        if !airborne_recovery(accessor, status) {
            return Correction::default();
        }
        return recovery_pad(run, p1, ai, accessor, position, stock);
    }
    let player = [natives::pos_x(p1), natives::pos_y(p1)];
    let correction = air_hit_recovery(run, player, accessor, position, status);
    if correction != Correction::default() {
        return stock.observe(correction, PerfectRule::HitRecovery, run, 0, run.reaction.frames().into());
    }
    let [left, right] = run.stage.ledges;
    if position[0] <= left[0] || position[0] >= right[0] {
        return Correction::default();
    }
    let air = konst(C::SituationAir);
    if air != UNRESOLVED && natives::situation_kind(accessor) == air {
        let correction = landing_dodge(run, p1, accessor, status, buttons);
        if correction != Correction::default() {
            return stock.observe(
                correction, PerfectRule::ReactDodge, run, run.threat.trigger_frame(), run.reaction.frames().into(),
            );
        }
    }
    if !falling(status) {
        return Correction::default();
    }
    let facts = LandingFacts {
        cpu: position,
        player: [natives::pos_x(p1), natives::pos_y(p1)],
        main_floor: run.stage.ledges.map(|point| point[0]),
        stock_stick_y: stock.stick[1],
        stock_buttons: buttons,
    };
    let options = landing_options(run, p1, accessor, facts);
    let edit = perfect::landing_variant(facts, options, run.landing_draw);
    stock.observe(edit, PerfectRule::Landing, run, 0, run.reaction.frames().into())
}

unsafe fn landing_options(run: &Run, p1: usize, cpu: usize, facts: LandingFacts) -> LandingOptions {
    if run.landing_draw < 6 || facts.stock_buttons & !perfect::FLICK_JUMP != 0 {
        return LandingOptions::default();
    }
    let mut out = LandingOptions::default();
    let [x, y] = facts.cpu;
    let Some(player_reach) = fighter_height(p1, facts.player[1]) else {
        return out;
    };
    out.player_in_reach = (x - facts.player[0]).abs() <= player_reach;
    match run.landing_draw {
        6 if !out.player_in_reach => {
            let gravity = konst(C::KineticGravity);
            let dive = konst(C::WorkFlagReserveDive);
            let Some(ray) = FloorRay::new(p1) else {
                return out;
            };
            out.fast_fall = gravity != UNRESOLVED
                && dive != UNRESOLVED
                && natives::sum_speed_y(cpu, gravity) < 0.0
                && !natives::is_flag(cpu, dive)
                && ray.floor(x, [y, run.stage.blast()[3]], true).is_some();
        }
        7 if out.player_in_reach => {
            let term = konst(C::TermContJumpAerial);
            let kind = natives::fighter_kind(cpu).unwrap_or(UNRESOLVED);
            let air = air_physics(cpu).with_fly_jump(kind, remaining_jumps(cpu));
            out.jump = term != UNRESOLVED
                && natives::is_enable_transition_term(cpu, term)
                && air.valid()
                && y + air.jump_speed.powi(2) / (2.0 * air.gravity) < run.stage.blast()[2];
        }
        8 => {
            let [left, right] = facts.main_floor;
            let away = if x >= facts.player[0] {
                (x + right) * 0.5
            } else {
                (x + left) * 0.5
            };
            let Some(ray) = FloorRay::new(p1) else {
                return out;
            };
            let Some(floor) = ray.floor(away, [y, run.stage.blast()[3]], true) else {
                return out;
            };
            let Some(height) = floor.height(away) else {
                return out;
            };
            let Some((vx, vy)) = super::recording_host::fighter_speed(cpu) else {
                return out;
            };
            if floor.flags & 0x2000 != 0
                && (away - facts.player[0]).abs() > player_reach
                && air_physics(cpu).reaches_with_jump(
                    facts.cpu,
                    [vx, vy],
                    [away, height],
                    90,
                    run.stage.blast()[3],
                )
            {
                out.platform_x = Some(away);
            }
        }
        _ => {}
    }
    out
}

/// DI and SDI against `player`: the juggler, or the point opposite the stage for an edge return.
unsafe fn air_hit_recovery(
    run: &mut Run,
    player: [f32; 2],
    cpu: usize,
    position: [f32; 2],
    status: i32,
) -> Correction {
    use crate::model::cpu::{
        defence::{self, direction},
        settings::Strength,
    };
    let hitlag = natives::tech_in_damage_stop(cpu) == Some(true);
    let damage = [
        C::StatusDamage,
        C::StatusDamageFly,
        C::StatusDamageFlyRoll,
        C::StatusDamageFlyMeteor,
        C::StatusDamageFlyReflectD,
        C::StatusDamageFlyReflectLr,
        C::StatusDamageFlyReflectU,
        C::StatusDamageFall,
    ];
    if !hitlag {
        run.sdi_start = None;
    }
    if !hitlag && (status == UNRESOLVED || !damage.map(konst).contains(&status)) {
        return Correction::default();
    }
    let air = konst(C::SituationAir);
    if air == UNRESOLVED || natives::situation_kind(cpu) != air {
        return Correction::default();
    }
    let (x, y) = super::recording_host::fighter_speed(cpu).unwrap_or_default();
    let reaction = konst(C::WorkFloatDamageReactionFrames);
    let frames = if reaction == UNRESOLVED {
        0.0
    } else {
        natives::get_float(cpu, reaction).max(0.0)
    };
    if !hitlag && frames == 0.0 {
        return Correction::default();
    }
    let Some([stick_x, stick_y]) =
        perfect::hit_recovery(position, player, [x, y], frames, run.stage.blast())
    else {
        return Correction::default();
    };
    let mut dir = match [stick_x, stick_y] {
        [_, 1.0] => direction::UP,
        [_, -1.0] => direction::DOWN,
        [1.0, _] => direction::RIGHT,
        [-1.0, _] => direction::LEFT,
        _ => direction::NEUTRAL,
    };
    if hitlag {
        let frame = run.rounds.frames;
        let since = *run.sdi_start.get_or_insert(frame);
        if defence::sdi_period(Strength::High).is_some_and(|period| (frame - since) % period != 0) {
            dir = direction::NEUTRAL;
        }
    }
    let (x, y) = defence::stick_vector(dir, player[0], position[0]);
    Correction {
        stick: Some([x, y]),
        ..Correction::default()
    }
}

/// P1's current attack: its motion once the status shows it, the raw input before that.
unsafe fn player_attack(p1: usize) -> (Option<u64>, i32) {
    let input = P1_BUTTONS.load(Ordering::Acquire);
    let status = natives::status_kind(p1);
    let attack = perfect::attack_key(status, input, || natives::motion_kind(p1), konst);
    (attack, status)
}

unsafe fn landing_dodge(run: &mut Run, p1: usize, cpu: usize, cpu_status: i32, buttons: u32) -> Correction {
    let (attack, status) = player_attack(p1);
    if attack.is_none() && status != konst(C::StatusWait) {
        run.threat = ThreatClock::default();
        return Correction::default();
    }
    let escape = konst(C::TermContEscapeAir);
    let can_dodge = escape != UNRESOLVED && natives::is_enable_transition_term(cpu, escape);
    let contact = if attack.is_some() && can_dodge {
        let Some(contact) = attack_reaches_core(p1, cpu) else {
            return Correction::default();
        };
        contact || (falling(cpu_status) && startup_dodge::covers_contact(p1, cpu, status))
    } else {
        false
    };
    run.threat.observe(
        attack,
        run.rounds.frames,
        run.reaction.frames(),
        contact,
        can_dodge,
        buttons,
    )
}

unsafe fn air_physics(cpu: usize) -> AirPhysics {
    use crate::model::hash40::hash40;
    let read = |name| natives::get_param_float(cpu, hash40(name), 0);
    AirPhysics::from_params(perfect::NativeAir {
        air_speed_x_stable: read("air_speed_x_stable"),
        air_accel_x_add: read("air_accel_x_add"),
        air_accel_x_mul: read("air_accel_x_mul"),
        air_accel_y: read("air_accel_y"),
        air_speed_y_stable: read("air_speed_y_stable"),
        jump_aerial_y: read("jump_aerial_y"),
    })
}

unsafe fn remaining_jumps(cpu: usize) -> Option<u8> {
    use crate::model::hash40::hash40;
    let id = konst(C::WorkIntJumpCount);
    if id == UNRESOLVED || !game::granted(crate::model::sites_pkg2::WORK_GET_INT_OFFSET) {
        return None;
    }
    let max = state::get_param_int(cpu, hash40("jump_count_max"), 0);
    let spent = natives::get_int(cpu, id);
    if max <= 0 || spent < 0 {
        return None;
    }
    u8::try_from(max.saturating_sub(spent).max(0)).ok()
}

/// Test the live attack volume against the CPU's current and projected torso axis.
unsafe fn attack_reaches_core(p1: usize, cpu: usize) -> Option<bool> {
    use crate::model::{eggs::Hit, hash40::hash40};
    use perfect::Capsule;

    let (hits, count) = super::hitbox::object_hits(p1);
    if count == 0 {
        return Some(false);
    }
    let hip = natives::joint_position(cpu, hash40("hip"), &[0.0; 3])?;
    let head = natives::joint_position(cpu, hash40("head"), &[0.0; 3])?;
    if !hip.into_iter().chain(head).all(f32::is_finite) || hip == head {
        return None;
    }
    let (px, py) = super::recording_host::fighter_speed(p1)?;
    let (cx, cy) = super::recording_host::fighter_speed(cpu)?;
    let core = [hip, head];
    let future = core.map(|[x, y, z]| [x + cx - px, y + cy - py, z]);
    Some(hits[..count].iter().any(|&Hit(a, b, radius, _)| {
        let attack = Capsule { ends: [a, b], radius };
        attack.touches_segment(core) || attack.touches_segment(future)
    }))
}

unsafe fn fighter_height(fighter: usize, foot: f32) -> Option<f32> {
    let [_, head, _] =
        natives::joint_position(fighter, crate::model::hash40::hash40("head"), &[0.0; 3])?;
    let height = head - foot;
    (height.is_finite() && height > 0.0).then_some(height)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MeasuredReach {
    NessNs1,
    LucasLx,
}

unsafe fn route_guide(
    run: &mut Run,
    p1: usize,
    cpu: usize,
    air: AirPhysics,
    pos: [f32; 2],
    speed: [f32; 2],
    special: Option<perfect::SpecialReach>,
    player_top: f32,
    measured: Option<MeasuredReach>,
) -> Option<RecoveryGuide> {
    let read_outline = || {
        let (midpoint, limits) =
            perfect::outline_probe(natives::camera_limit_range()?, run.stage.blast())?;
        let ray = FloorRay::new(p1)?;
        super::tech_surfaces::training_outline(&ray, midpoint, limits)
    };
    let Some((outline, live_ledges)) = read_outline() else {
        planner_unavailable(PlanInput::StageOutline);
        return None;
    };
    let [left, right] = if run.stage_key == (89, 0) {
        let Some(ledges) = live_ledges else {
            planner_unavailable(PlanInput::StageOutline);
            return None;
        };
        ledges
    } else {
        run.stage.ledges
    };
    let player = [natives::pos_x(p1), natives::pos_y(p1)];
    let player_reach = player_top - player[1];
    if !player_reach.is_finite() || player_reach <= 0.0 {
        planner_unavailable(PlanInput::PlayerHeight);
        return None;
    }
    run.player_reach = player_reach;
    let Some((body, jumps)) = fighter_height(cpu, pos[1])
        .and_then(|body| remaining_jumps(cpu).map(|jumps| (body, jumps)))
    else {
        planner_unavailable(PlanInput::FighterGeometry);
        return None;
    };
    let bottom = run.stage.blast()[3];
    let jump_rise = if jumps == 0 {
        0.0
    } else {
        air.jump_speed.powi(2) / (2.0 * air.gravity)
    };
    let [special_horizontal, special_rise] = special.map_or([0.0; 2], |reach| [reach.horizontal, reach.rise]);
    let fighter = natives::fighter_kind(cpu).unwrap_or(UNRESOLVED);
    let facts = RouteFacts {
        fighter,
        stage: run.stage_key,
        position: pos,
        player,
        player_reach,
        ledges: [left, right],
        blast: run.stage.blast(),
        outline: outline.segments(),
        body_clearance: body,
        ledge_distance: body,
        fall_budget: air.terminal_speed,
        reach: [
            air.horizontal_speed * 90.0 + special_horizontal,
            (jump_rise + special_rise + speed[1].max(0.0)).max(f32::EPSILON),
        ],
    };
    if measured == Some(MeasuredReach::NessNs1) {
        if let Some(guide) = niche_pad::NicheGuide::prepare_ns1(cpu, p1, &facts, run.rounds.frames, speed, jumps) {
            let row = guide.selected_niche_row();
            run.route_history.planned(row.route);
            planner_chose(route_code(row.route), fighter as u32, 0, row.id.into());
            run.niche = Some(guide);
        } else {
            planner_chose(4, fighter as u32, 0, 0);
        }
        return None;
    }
    let [near, high, low] = perfect::possible_routes(&facts);
    let (near_ledge, inward) = if pos[0] <= left[0].midpoint(right[0]) {
        (left, 1.0)
    } else {
        (right, -1.0)
    };
    let lip = [near_ledge[0], left[1].max(right[1]) + body];
    let high = high.filter(|_| {
        air.high_route_input(pos, speed, lip, inward, bottom, jumps != 0)
            .is_some()
    });
    let band = facts.crossing_band();
    let crossable = u32::from(band.is_some());
    let zone = guarder_zone(p1, player, player_reach, near_ledge, pos);
    run.zone = zone;
    run.route_watch.set_guarder_zone(zone);
    run.hold.replan();
    let reach = Reach::trusted(air, jumps != 0, special);
    if fighter == game_watch::KIND && special.is_some() {
        let [pick, timing] = run.oil_draw;
        run.oil_stall.arm(zone, pick, timing);
    }
    let deadline = reach.deadline(pos, speed, near_ledge, bottom);
    let mut row = None;
    if fighter != UNRESOLVED {
        let inputs = niche_pad::RowInputs {
            frame: run.rounds.frames,
            zone,
            draw: run.route_draw,
            short: recovery::short(reach, pos, speed, near_ledge, bottom),
            speed,
            air,
            jumps,
            special,
            lucas_lx: measured == Some(MeasuredReach::LucasLx),
        };
        row = niche_pad::NicheGuide::prepare_niche(cpu, p1, fighter, &facts, band, inputs);
    }
    // An extension row is a need. A crossing row is one candidate of the roll.
    let needed = row.as_ref().is_some_and(|(guide, _)| guide.selected_niche_row().route == Route::Near);
    let crossing = row
        .as_ref()
        .filter(|_| !needed)
        .map(|(guide, margin)| recovery::niche_plan(Route::Opposite, *margin, guide.exit_point()));
    let options = [near, high, low, crossing];
    let last = run.route_history.previous();
    let plan = if needed {
        None
    } else if let Some(crossing_frames) = row.as_ref().and_then(|(guide, _)| guide.crossing_frames()) {
        use crate::model::hash40::hash40;
        let far_x = if inward > 0.0 { right[0] } else { left[0] };
        recovery::mix_measured_crossing(
            options,
            zone,
            last,
            run.route_draw,
            recovery::CrossingRace {
                near_x: near_ledge[0],
                far_x,
                guarder_x: player[0],
                guarder_run_speed: natives::get_param_float(p1, hash40("run_speed_max"), 0),
                guarder_facing: natives::lr(p1),
                crossing_frames,
            },
        )
    } else {
        recovery::mix(options, zone, last, run.route_draw)
    };
    let chosen = plan.map(|plan| plan.route);
    let guide = row.and_then(|(guide, _)| {
        if needed || chosen == Some(Route::Opposite) {
            Some(guide)
        } else if perfect::pikachu::fake_out_selected(chosen, zone, last, run.route_draw) {
            guide.fake_out(pos, speed)
        } else {
            None
        }
    });
    if let Some(guide) = guide {
        let row = guide.selected_niche_row();
        if !guide.awaiting_crossing_commit() {
            run.route_history.planned(row.route);
            planner_chose(route_code(row.route), fighter as u32, crossable, row.id.into());
        }
        run.niche = Some(guide);
        return None;
    }
    let Some(plan) = plan.filter(|_| measured.is_none()) else {
        planner_chose(4, fighter as u32, crossable, 0);
        return None;
    };
    planner_chose(route_code(plan.route), fighter as u32, crossable, 0);
    run.route_history.planned(plan.route);
    run.jump_delay = recovery::planned_delay(plan.route, zone, deadline, run.route_draw);
    (plan.route == Route::High).then_some(RecoveryGuide { lip, inward })
}

/// Where P1 stands against the fighter's near ledge. `reach` is P1's sampled height.
unsafe fn guarder_zone(p1: usize, player: [f32; 2], reach: f32, ledge: [f32; 2], cpu: [f32; 2]) -> Zone {
    let situation = natives::situation_kind(p1);
    let is = |kind| {
        let kind = konst(kind);
        kind != UNRESOLVED && situation == kind
    };
    recovery::zone(
        Guarder {
            position: player,
            hanging: is(C::SituationCliff),
            airborne: is(C::SituationAir),
            reach: reach * 1.5,
        },
        ledge,
        cpu,
    )
}

/// P1's live attack volume lies within one body of the ledge the fighter is about to grab.
unsafe fn ledge_hot(run: &mut Run, p1: usize, [x, y]: [f32; 2]) -> bool {
    use crate::model::eggs::Hit;
    let attack = player_attack(p1).0.filter(|_| run.zone == Zone::Ledge);
    let body = run.player_reach;
    let volume = attack.is_some() && body.is_finite() && body > 0.0 && {
        let (hits, count) = super::hitbox::object_hits(p1);
        hits[..count].iter().any(|&Hit(a, b, radius, _)| {
            perfect::Capsule { ends: [a, b], radius: radius + body * 0.5 }
                .touches_segment([[x, y, 0.0], [x, y - body, 0.0]])
        })
    };
    run.ledge_watch.hot(attack, volume, run.rounds.frames, run.reaction.frames())
}

fn route_code(route: Route) -> u32 {
    match route {
        Route::Near => 0,
        Route::Opposite => 1,
        Route::High => 2,
        Route::LateLow => 3,
    }
}

unsafe fn niche_correction(
    run: &mut Run,
    ai: usize,
    cpu: usize,
    p1: usize,
    position: [f32; 2],
    status: i32,
    stock: StockPad,
) -> Correction {
    let Some(guide) = run.niche.as_mut() else {
        return Correction::default();
    };
    if guide.accepts_form(None) {
        let kind = natives::fighter_kind(cpu);
        if !((kind == Some(0x5a) && niche_pad::pyra_swapping(status))
            || kind.is_some_and(|kind| guide.accepts_form(Some(kind))))
        {
            run.niche = None;
        }
        return Correction::default();
    }
    let awaiting_commit = guide.awaiting_crossing_commit();
    if let Some(edit) = guide.correct_niche_pad(ai, cpu, p1, position, status, run.rounds.frames) {
        if guide.awaiting_crossing_commit() {
            return stock.observe(edit, PerfectRule::Route, run, 0, 3);
        }
        if awaiting_commit {
            let row = guide.selected_niche_row();
            run.route_history.planned(row.route);
            planner_chose(route_code(row.route), row.fighter as u32, 1, row.id.into());
        }
        let id = guide.correction_row();
        return stock.observe(edit, PerfectRule::Niche, run, 0, id.into());
    }
    run.niche = None;
    run.plan_pending |= awaiting_commit;
    Correction::default()
}

unsafe fn player_in_lag(accessor: usize) -> bool {
    let status = natives::status_kind(accessor);
    status != UNRESOLVED
        && [C::StatusLandingAttackAir, C::StatusLandingDamageLight]
            .map(konst)
            .contains(&status)
        && !natives::is_enable_cancel(accessor)
}

/// The AI epilogue owns the AI and its current self snapshot for this call.
unsafe fn recovery_snapshot(ai: usize) -> Option<(usize, bool)> {
    use crate::engine::seams::native_fields as field;
    let base = RECOVERY_BASE.load(Ordering::Acquire);
    if base == 0 {
        return None;
    }
    let snapshot = field::child(ai, 0x168)?;
    let available: extern "C" fn(usize, u32) -> bool = std::mem::transmute(base + 0x358c20);
    Some((snapshot, available(snapshot, 0)))
}

unsafe fn special_reach(ai: usize, snapshot: usize) -> Option<perfect::SpecialReach> {
    let (_, reach) = read_special_reach(ai, snapshot)?;
    reach.valid().then_some(reach)
}

unsafe fn read_special_reach(ai: usize, snapshot: usize) -> Option<(i32, perfect::SpecialReach)> {
    use crate::engine::seams::native_fields as field;
    if !SPECIAL_TABLE.load(Ordering::Acquire) {
        return None;
    }
    let variant = field::read_unchecked::<i32>(snapshot, 0x240);
    if !(0..=3).contains(&variant) {
        return None;
    }
    let at = variant as usize * std::mem::size_of::<f32>();
    let reach = perfect::SpecialReach {
        horizontal: field::read_unchecked(ai, 0xae0 + at),
        rise: field::read_unchecked(ai, 0xad0 + at),
    };
    Some((variant, reach))
}

unsafe fn last_dodge_reach(snapshot: usize, cpu: usize) -> perfect::DodgeReach {
    use crate::engine::seams::native_fields as field;
    let distance = field::read_unchecked::<f32>(snapshot, 0x230)
        * natives::get_param_float(cpu, 0x1d_fb01_ad43, 0);
    let cancel_frames = field::read_unchecked(snapshot, 0x234);
    perfect::DodgeReach {
        distance,
        cancel_frames,
    }
}

unsafe fn recovery_pad(
    run: &mut Run,
    p1: usize,
    ai: usize,
    cpu: usize,
    pos: [f32; 2],
    stock: StockPad,
) -> Correction {
    let buttons = stock.buttons;
    let jump = konst(C::TermContJumpAerial);
    let jump_transition = jump != UNRESOLVED && natives::is_enable_transition_term(cpu, jump);
    let jumps = remaining_jumps(cpu);
    let kind = natives::fighter_kind(cpu).unwrap_or(UNRESOLVED);
    let physics = air_physics(cpu).with_fly_jump(kind, jumps);
    let speed = match physics.plan_velocity(super::recording_host::fighter_speed(cpu)) {
        Ok(speed) => speed,
        Err(input) => {
            if run.plan_pending && run.player_height.report_unavailable() {
                planner_unavailable(input);
            }
            return Correction::default();
        }
    };
    let [vx, vy] = speed;
    let can_jump = recovery::recovery_jump_available(jumps, jump_transition);
    let recovery = recovery_snapshot(ai);
    let [left, right] = run.stage.ledges;
    let guarder = (run.guide.is_some() || !run.plan_pending && run.player_reach > 0.0)
        .then(|| [natives::pos_x(p1), natives::pos_y(p1)]);
    let blocked = run.guide.zip(guarder).is_some_and(|(guide, player)| {
        perfect::player_on_path(player, [pos, guide.lip], run.player_reach)
    });
    run.plan_pending |= run
        .route_watch
        .observe(jumps.zip(recovery.map(|(_, available)| available)), blocked);
    if !run.plan_pending {
        let near = if pos[0] < left[0] {
            Some(left)
        } else {
            (pos[0] > right[0]).then_some(right)
        };
        if let Some((ledge, player)) = near.zip(guarder.filter(|p| p.iter().all(|x| x.is_finite()))) {
            let zone = guarder_zone(p1, player, run.player_reach, ledge, pos);
            run.plan_pending = run.route_watch.observe_guarder(zone, run.rounds.frames);
        }
    }
    let envelope = recovery
        .filter(|&(_, available)| available)
        .and_then(|(snapshot, _)| read_special_reach(ai, snapshot));
    let special = envelope.filter(|&(_, reach)| reach.valid()).map(|(_, reach)| reach);
    let measured = envelope.and_then(|(variant, reach)| {
        if perfect::ness::measured_envelope(kind, variant, reach) {
            Some(MeasuredReach::NessNs1)
        } else if perfect::lucas::measured_envelope(kind, variant, reach) {
            Some(MeasuredReach::LucasLx)
        } else {
            None
        }
    });
    if run.plan_pending {
        run.guide = None;
        if let Some(player_height) = run.player_height.take() {
            run.plan_pending = false;
            let plan = match (player_height, recovery, special) {
                (None, _, _) => Err(PlanInput::PlayerHeight),
                (Some(_), None, _) => Err(PlanInput::Snapshot),
                (Some(_), Some((_, true)), None) if measured.is_none() => Err(PlanInput::SpecialEnvelope),
                (Some(top), Some(_), reach) => Ok((top, reach)),
            };
            match plan {
                Ok((top, reach)) => {
                    run.guide = route_guide(run, p1, cpu, physics, pos, speed, reach, top, measured);
                    if run.niche.is_some() {
                        return niche_correction(run, ai, cpu, p1, pos, natives::status_kind(cpu), stock);
                    }
                }
                Err(input) => planner_unavailable(input),
            }
        }
    }
    if kind == perfect::ness::KIND
        && special.is_none()
        && !matches!(recovery, Some((_, false)))
    {
        return Correction::default();
    }
    if kind == game_watch::KIND && run.oil_stall.due(run.rounds.frames) {
        let ledge = if pos[0] < left[0] { Some(left) } else { (pos[0] > right[0]).then_some(right) };
        if let (Some(ledge), Some(special)) = (ledge, special) {
            let player = guarder.unwrap_or_else(|| [natives::pos_x(p1), natives::pos_y(p1)]);
            let inward = (ledge[0] - pos[0]).signum();
            let air = konst(C::SituationAir);
            let can_start = air != UNRESOLVED && natives::situation_kind(cpu) == air
                && natives::lr(cpu) == inward
                && niche_pad::niche_start_enabled(cpu, perfect::niche::Move::Down, true)
                && niche_pad::niche_start_enabled(cpu, perfect::niche::Move::Rise, true);
            let zone = guarder_zone(p1, player, run.player_reach, ledge, pos);
            if let Some(pad) = run.oil_stall.begin_oil_panic(OilInputs {
                frame: run.rounds.frames,
                status: natives::status_kind(cpu),
                position: pos,
                velocity: speed,
                ledge,
                player,
                player_reach: run.player_reach,
                zone,
                reach: Reach::trusted(physics, false, Some(special)),
                bottom: run.stage.blast()[3],
                can_start,
            }) {
                planner_chose(route_code(Route::Near), kind as u32, 0, game_watch::ROW.into());
                run.route_history.planned(Route::Near);
                return stock.observe(pad, PerfectRule::Niche, run, 0, game_watch::ROW.into());
            }
        }
    }
    let attack = buttons & perfect::ATTACK != 0;
    let can_punish = attack && player_in_lag(p1);
    if let Some(RecoveryGuide { lip, inward }) = run.guide {
        if !can_punish {
            if let Some(mut correction) = physics.high_route_input(
                pos,
                speed,
                lip,
                inward,
                run.stage.blast()[3],
                can_jump,
            ) {
                let detour = perfect::avoid_detour(buttons, true, false, inward).release;
                if buttons & !(perfect::JUMP | perfect::FLICK_JUMP | detour) == 0 {
                    stock.observe(correction, PerfectRule::Route, run, 0, 2);
                    correction.release = detour;
                    stock.observe(
                        Correction { release: detour, ..Correction::default() },
                        PerfectRule::Detour, run, 0, 0,
                    );
                    return correction;
                }
            }
        }
        run.guide = None;
    }
    let ledge = if pos[0] < left[0] {
        left
    } else if pos[0] > right[0] {
        right
    } else {
        run.hold = Hold::default();
        return Correction::default();
    };
    let special = recovery::stock_timing_reach(kind, special);
    if jumps == Some(0) {
        let bottom = run.stage.blast()[3];
        let coasts = physics.reaches_with_jump(pos, speed, ledge, u8::MAX, bottom);
        if let Some((snapshot, false)) = recovery {
            let escape = konst(C::TermContEscapeAir);
            if buttons & !(perfect::GUARD | perfect::FLICK_JUMP) == 0
                && escape != UNRESOLVED
                && natives::is_enable_transition_term(cpu, escape)
            {
                let reach = last_dodge_reach(snapshot, cpu);
                let pad = perfect::distance_dodge(pos, ledge, reach, true, coasts);
                if pad != Correction::default() {
                    return stock.observe(pad, PerfectRule::DistanceDodge, run, 0, 0);
                }
            }
        } else if let Some(reach) =
            special.filter(|_| !coasts && buttons & !perfect::FLICK_JUMP == 0)
        {
            let term = konst(C::TermContSpecialHi);
            if term != UNRESOLVED && natives::is_enable_transition_term(cpu, term) {
                let next = [pos[0] + vx, pos[1] + vy];
                let pad = reach.timed_input(pos, next, ledge);
                if pad != Correction::default() {
                    return stock.observe(pad, PerfectRule::Reach, run, 0, 0);
                }
            }
        }
    }
    if attack && !can_punish {
        let delay = if can_jump { 0 } else { 90 };
        let reachable = physics.reaches_with_jump(pos, speed, ledge, delay, run.stage.blast()[3]);
        let correction =
            perfect::avoid_detour(buttons, reachable, false, (ledge[0] - pos[0]).signum());
        if correction != Correction::default() {
            return stock.observe(correction, PerfectRule::Detour, run, 0, 0);
        }
    }
    // The stock jump, then the stock up special, wait for the planned frame while the fighter
    // still returns one frame later. Each frame re-checks from the live position.
    let bottom = run.stage.blast()[3];
    let term = konst(C::TermContSpecialHi);
    if !recovery::generic_timing_allowed(kind, special, matches!(recovery, Some((_, false)))) {
        return Correction::default();
    }
    let alone = Reach::trusted(physics, false, special);
    // An off-stage guarder: the jump stays in reserve while the up special alone returns.
    let reserve = can_jump
        && matches!(run.zone, Zone::OffHigh | Zone::OffLow)
        && alone.returns(pos, speed, ledge, 1, bottom);
    let (button, reach, planned) = if reserve {
        (perfect::JUMP, alone, u8::MAX)
    } else if can_jump {
        (perfect::JUMP, Reach::trusted(physics, true, special), run.jump_delay)
    } else if jumps == Some(0)
        && special.is_some()
        && term != UNRESOLVED
        && natives::is_enable_transition_term(cpu, term)
    {
        (perfect::SPECIAL, alone, run.jump_delay)
    } else {
        run.hold = Hold::default();
        return Correction::default();
    };
    let wait_drift = reach.hold_stick(pos, speed, ledge);
    let can_wait = wait_drift.is_some() && reach.returns(pos, speed, ledge, 1, bottom);
    if !can_wait && can_jump && !reserve && !run.hold.waiting() {
        let correction = reach.jump_at_horizontal_limit(pos, speed, ledge, bottom, buttons);
        if correction != Correction::default() {
            return stock.observe(correction, PerfectRule::Reach, run, 0, 0);
        }
    }
    let hot = can_wait && ledge_hot(run, p1, ledge);
    // The delay applies only to the chosen resource.
    let frame = run.rounds.frames;
    let left = jumps.unwrap_or(0);
    let mut correction =
        run.hold.step(HoldFacts { frame, buttons, button, jumps: left, planned, can_wait, hot });
    let inward = (ledge[0] - pos[0]).signum();
    if let Some(x) = wait_drift.filter(|_| correction.release != 0 || run.hold.waiting()) {
        // Level stick while the up special is withheld: no tap jump, no fast fall.
        let y = if correction.release & perfect::SPECIAL != 0 { 0.0 } else { stock.stick[1] };
        correction.stick = Some([x, y]);
    } else if correction.press == perfect::JUMP {
        correction.stick = Some([wait_drift.unwrap_or(-inward), stock.stick[1]]);
    } else if correction.press == perfect::SPECIAL && buttons & perfect::SPECIAL == 0 {
        correction.stick = Some([0.0, 1.0]);
    }
    let rule = if button == perfect::JUMP { PerfectRule::Jump } else { PerfectRule::Reach };
    stock.observe(correction, rule, run, 0, u32::from(planned))
}

fn axis(pressed: u16, negative: u8, positive: u8) -> i16 {
    i16::from(pressed & (1 << positive) != 0) - i16::from(pressed & (1 << negative) != 0)
}

/// P1's raw poll. The next fighter frame resolves the floor under the new cursor.
pub unsafe fn poll(controller: usize, sample: Sample) -> bool {
    if !wizard_active() {
        return false;
    }
    let Some(mut engine) = ENGINE.try_lock() else {
        return true;
    };
    let Engine {
        run: Some(run),
        kept,
        ..
    } = &mut *engine
    else {
        return true;
    };
    let Some(wizard) = &mut run.wizard else {
        return true;
    };
    let pressed = sample.pressed;
    let key_down = |k| pressed & (1u16 << k) != 0;
    let horizontal = axis(pressed, key::DPAD_LEFT, key::DPAD_RIGHT);
    let vertical = axis(pressed, key::DPAD_DOWN, key::DPAD_UP);
    let stick = [0, 1].map(|i| ((controller + field::STICKS[i]) as *const f32).read_volatile());
    let pad = Pad { stick, held: sample.held, pressed, save: false, load: false };

    if key_down(key::B) {
        run.confirm_player = false;
        if !wizard.back() {
            practice::finish(run.drill.game());
            BACK.store(true, Ordering::Release);
        }
        return true;
    }
    if key_down(key::X) {
        if run.drill == Drill::GuardEdge && wizard.step == WizardStep::Cpu {
            wizard.set_side_default(run.stage, false);
        } else {
            wizard.back_to_defaults(run.defaults);
        }
        run.confirm_player = false;
        return true;
    }
    if key_down(key::Y) && run.drill == Drill::GuardEdge && wizard.step == WizardStep::Cpu {
        wizard.set_side_default(run.stage, true);
        return true;
    }
    match wizard.step {
        WizardStep::Cpu => wizard.steer(run.stage, &pad),
        WizardStep::Percent => wizard.change_percent(horizontal + vertical * 10),
        WizardStep::Player => {
            wizard.steer(run.stage, &pad);
            run.confirm_player |= key_down(key::A);
            return true;
        }
        WizardStep::Preview => {}
    }
    if key_down(key::A) || wizard.step == WizardStep::Preview && key_down(key::PLUS) {
        if wizard.step == WizardStep::Cpu && !wizard.accepts_cpu(run.stage) {
            notices::refuse("Pick a point off the stage"); // restriction: edge-juggle-start-over-stage
            return true;
        }
        if wizard.step == WizardStep::Preview {
            run.setup = wizard.setup;
            kept[run.drill as usize] = Some((run.stage_key, run.setup, wizard.unmoved()));
            run.wizard = None;
            WIZARD.store(false, Ordering::Release);
        } else {
            wizard.advance();
        }
    }
    true
}

/// Borrowed only on the P1 frame. Wider or sloped main floors can have adjacent exact endpoints.
unsafe fn ledge(
    ray: &FloorRay,
    mut floor: crate::model::mashout::entry::PlacementFloor,
    right: bool,
    limits: [f32; 2],
) -> Option<[f32; 2]> {
    let dir = if right { 1.0 } else { -1.0 };
    for _ in 0..crate::model::tech::surface::MAX_LINES {
        let [a, b] = floor.ends;
        let end = if (a[0] < b[0]) == right { b } else { a };
        let Some(next) = ray.floor(end[0] + dir, limits, false) else {
            return Some(end);
        };
        if !next.ends.contains(&end) || next.ends == floor.ends {
            return Some(end);
        }
        floor = next;
    }
    None
}

pub(super) unsafe fn stage_floor(
    ray: &FloorRay,
    [left, right, top, bottom]: [f32; 4],
) -> Option<(perfect::Segment, perfect::Point)> {
    let x = left.midpoint(right);
    let limits = [top, bottom];
    let floor = ray.floor(x, limits, false)?;
    let ledges = [
        ledge(ray, floor, false, limits)?,
        ledge(ray, floor, true, limits)?,
    ];
    Some((ledges, [x, floor.height(x)?]))
}

unsafe fn setup_stage(p1: usize) -> Option<(Stage, [f32; 2])> {
    let bounds = natives::camera_limit_range()?;
    let ray = FloorRay::new(p1)?;
    let (ledges, start) = stage_floor(&ray, bounds)?;
    let blast = super::tech_surfaces::training_blast_bounds()?;
    Some((Stage::new(blast, bounds, ledges)?, start))
}

unsafe fn snap_player(p1: usize, drill: Drill, stage: Stage, [x, y]: [f32; 2]) -> Option<[f32; 2]> {
    let platforms = drill == Drill::Juggler;
    let x = if platforms { x } else { x.clamp(stage.ledges[0][0], stage.ledges[1][0]) };
    placement::floor_under(p1, [x, y], platforms)
}

/// Give the native fighter scripts one grounded transition before a new fall.
unsafe fn reset_recovery_resources(p1: usize, cpu: usize, stage: Stage) -> bool {
    use crate::model::sites_pkg3 as s;
    let [ground, wait] = [C::SituationGround, C::StatusWait].map(konst);
    if [ground, wait].contains(&UNRESOLVED)
        || ![
            s::STATE_CLEAR_SPEED_ALL_OFFSET,
            s::STATE_INIT_POS_OFFSET,
            s::STATE_SET_SITUATION_KIND_OFFSET,
            s::STATE_STATUS_REQUEST_OFFSET,
        ]
        .into_iter()
        .all(game::granted)
    {
        return false;
    }
    let [left, right] = stage.ledges;
    let cursor = [left[0].midpoint(right[0]), left[1].midpoint(right[1])];
    let Some([x, y]) = snap_player(p1, Drill::GuardEdge, stage, cursor) else {
        return false;
    };
    state::clear_speed_all(cpu);
    state::init_pos(cpu, &state::Vector3f { x, y, z: 0.0 });
    state::set_situation_kind(cpu, ground);
    state::status_request(cpu, wait);
    true
}

unsafe fn placed_player(p1: usize) -> PlacedPlayer {
    let wait = konst(C::StatusWait);
    PlacedPlayer {
        hit_freeze: natives::tech_in_damage_stop(p1) == Some(true),
        wait: wait != UNRESOLVED && natives::status_kind(p1) == wait,
    }
}

/// Player 1 stands on its written start: the written x, on the floor the write found (the first floor under its feet
/// has the written height there). Another x is off the start. `None`: the floor could not be read, which shows
/// nothing; a Player 1 on another floor at the same x is then missed, as before this check existed.
unsafe fn on_start(p1: usize, [x, y]: [f32; 2]) -> Option<bool> {
    if natives::pos_x(p1) != x {
        return Some(false);
    }
    let [_, _, _, bottom] = natives::camera_limit_range()?;
    let floor = FloorRay::new(p1)?.floor(x, [natives::pos_y(p1) + 1.0, bottom], true)?;
    Some(floor.height(x)? == y)
}

unsafe fn try_place(bodies: &Bodies, drill: Drill, stage_key: StageKey, stage: Stage, setup: Setup) -> Option<[f32; 2]> {
    let Some([x, y]) = snap_player(bodies.p1, drill, stage, setup.p1) else {
        return None;
    };
    let [wait, fall, ground, air] = [
        C::StatusWait,
        C::StatusFall,
        C::SituationGround,
        C::SituationAir,
    ]
    .map(konst);
    if [wait, fall, ground, air].contains(&UNRESOLVED) {
        return None;
    }
    use crate::model::sites_pkg3 as s;
    if ![
        s::STATE_CLEAR_SPEED_ALL_OFFSET,
        s::STATE_INIT_POS_OFFSET,
        s::STATE_SET_POS_OFFSET,
        s::STATE_SET_LR_OFFSET,
        s::STATE_SET_SITUATION_KIND_OFFSET,
        s::STATE_STATUS_REQUEST_OFFSET,
        s::STATE_SET_SLEEP_OFFSET,
        s::STATE_HEAL_OFFSET,
        s::STATE_ADD_DAMAGE_OFFSET,
    ]
    .into_iter()
    .all(game::granted)
    {
        return None;
    }
    let jump_count = konst(C::WorkIntJumpCount);
    if jump_count == UNRESOLVED || !game::granted(crate::model::sites_pkg4::WORK_SET_INT_OFFSET) {
        return None;
    }
    let [cx, _] = setup.cpu;
    let damage = natives::damage(bodies.cpu, 0);
    if !damage.is_finite() {
        return None;
    }
    let facing = if cx < x { -1.0 } else { 1.0 };
    placement::stand(bodies.p1, [x, y], facing);
    let cpu_facing = crate::model::edge_juggle::recovery_away_facing(
        drill, stage_key, stage, setup, natives::fighter_kind(bodies.cpu),
    ).unwrap_or(-facing);
    placement::lift(bodies.cpu, setup.cpu, cpu_facing);
    natives::set_int(bodies.cpu, drill.placed_jump_count(), jump_count);
    state::heal(bodies.cpu, -damage);
    state::add_damage(bodies.cpu, f32::from(setup.percent));
    placement::fall(bodies.cpu);
    Some([x, y])
}

fn post_notice(run: &mut Run, best: Option<u32>, best_streak: Option<u32>) {
    let Session {
        phase,
        attempts,
        secured,
        streak,
        frames,
        hits,
        ..
    } = run.rounds;
    let mut text = FixedPath::<128>::new();
    let time = frames as f32 / 60.0;
    let best_streak = best_streak.unwrap_or(0);
    let outcome = match phase {
        Phase::Result(result, _) => match result {
            Outcome::ZoomWin => "Win",
            Outcome::ZoomMiss => "Miss",
            Outcome::Secured => "Secured",
            Outcome::RecoveredAtLedge => "Recovered at the ledge",
            Outcome::RecoveredOnStage => "Recovered on stage",
            Outcome::Unresolved => "Unresolved",
            Outcome::Landed => "Landed",
            Outcome::GrabbedLedge => "Ledge",
            Outcome::KnockedOut => "KO",
        },
        Phase::Done => "Set complete · L+R+A restarts",
        _ => "",
    };
    let mut header = run.drill.game().title();
    if let Some(wizard) = run.wizard {
        let Setup {
            cpu: [x, y],
            p1: [px, py],
            percent,
        } = wizard.setup;
        let number = match wizard.step {
            WizardStep::Cpu => 0,
            WizardStep::Percent => 1,
            WizardStep::Player => if run.drill == Drill::GuardEdge { 1 } else { 2 },
            WizardStep::Preview => if run.drill == Drill::GuardEdge { 2 } else { 3 },
        };
        header = match run.drill {
            Drill::GuardEdge => ["Edge · 1/3", "Edge · 2/3", "Edge · 3/3"][number],
            Drill::Juggler => [
                "Juggle · 1/4",
                "Juggle · 2/4",
                "Juggle · 3/4",
                "Juggle · 4/4",
            ][number],
        };
        let _ = match wizard.step {
            WizardStep::Cpu if run.drill == Drill::Juggler => write!(
                text,
                "CPU x {x:.0} · Height {:+.0} · Stick or D-pad move · A next",
                y - run.stage_floor_y,
            ),
            WizardStep::Cpu => write!(text, "CPU ({x:.0}, {y:.0}) · Stick or D-pad move · A next · X default start · Y other side"),
            WizardStep::Percent => write!(text, "{percent}% · D-pad change · A next"),
            WizardStep::Player => write!(text, "P1 ({px:.0}, {py:.0}) · Stick or D-pad move · A next"),
            WizardStep::Preview => write!(
                text,
                "CPU ({x:.0}, {y:.0}) · {percent}% · P1 ({px:.0}, {py:.0}) · A/+ start"
            ),
        };
    } else if let Phase::Ready(left) = phase {
        let _ = write!(
            text,
            "Ready · {}",
            left.div_ceil(run.drill.ready_frames() / 3)
        );
    } else if matches!(run.opponent.mode, Mode::Drill(DrillId::Zoom)) {
        let _ = write!(text, "{outcome}  Zoom wins {secured}/{attempts} · 8 MP minimum before cast");
    } else if run.drill == Drill::GuardEdge {
        let _ = write!(
            text,
            "{outcome}  {secured}/{attempts} · Streak {streak} · Best {best_streak}"
        );
        if matches!(phase, Phase::Result(Outcome::Secured, _)) {
            let _ = write!(text, " · {time:.2} s");
        }
        if let Some(best) = best {
            let _ = write!(text, " · Fastest {:.2} s", best as f32 / 60.0);
        }
    } else {
        let _ = write!(text, "{outcome}  Airtime {time:.2} s · Hits {hits}");
        if let Some(best) = best {
            let _ = write!(text, " · Best {:.2} s", best as f32 / 60.0);
        }
    }
    if run.notice.as_str() != text.as_str() || frames % 30 == 1 && phase == Phase::Live {
        notices::upsert(NOTICE, header, text.as_str(), u16::MAX, DEFAULT_COLOR);
        run.notice = text;
    }
}

impl Engine {
    unsafe fn wake_cpu(&mut self, current: Option<CpuBody>) {
        let held = self.held_cpu.take();
        if let Some(current) = current.filter(|&body| held == Some(body)) {
            state::set_sleep(current.accessor, false);
        }
    }
}

/// One step per primary P1 fighter frame, with current owned accessors and no waits.
pub unsafe fn frame(fighter: usize) {
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let drill = wanted();
    if drill.is_none() && !ENGINE_ACTIVE.load(Ordering::Acquire) {
        return;
    }
    let Some(mut engine) = ENGINE.try_lock() else {
        return;
    };
    if !scene::is_native_training() {
        if let Some(old) = engine.run.take() {
            practice::finish(old.drill.game());
        }
        engine.held_cpu = None;
        ENGINE_ACTIVE.store(false, Ordering::Release);
        WIZARD.store(false, Ordering::Release);
        return;
    }
    if published::HOT_SWAP_IN_PROGRESS.load(Ordering::Acquire) {
        return;
    }
    if fighter == 0 {
        return;
    }
    let Some(p1) =
        natives::module_accessor(0).filter(|&p1| p1 == natives::fighter_accessor(fighter))
    else {
        return;
    };
    let body = CpuBody::current();
    let owns_setup = || {
        super::cpu_level::drill_applied()
            && natives::is_operation_cpu(1)
            && !OPEN.load(Ordering::Acquire)
            && !states::is_loading()
            && !recorder::busy_for_drill()
            && !published::input_owners().covers_strip()
    };
    if body.is_none()
        && drill == Some(Drill::GuardEdge)
        && START.load(Ordering::Acquire) == 0
        && !RESTART.load(Ordering::Acquire)
    {
        if let Some(run) = engine.run.as_mut().filter(|run| {
            run.rounds.phase == Phase::Live
                && run.niche.as_ref().is_some_and(|guide| guide.waiting_for_form(run.rounds.frames))
        }) {
            if owns_setup() {
                // The native swap can leave entry 1 between objects for a frame.
                // The attempt clock continues and no saved accessor is dereferenced.
                run.rounds.advance_attempt(Observation::default());
                return;
            }
        }
    }
    let Some((drill, body)) = drill.zip(body) else {
        engine.wake_cpu(body);
        if engine.run.take().is_some() {
            ENGINE_ACTIVE.store(false, Ordering::Release);
            WIZARD.store(false, Ordering::Release);
            notices::clear(NOTICE);
        }
        return;
    };
    let cpu = body.accessor;
    if !owns_setup() {
        engine.wake_cpu(Some(body));
        if let Some(run) = &mut engine.run {
            run.rounds.discard_attempt();
            run.placement = Placement::ResetResources;
            run.notice = FixedPath::new();
        }
        return;
    }
    let start = START.load(Ordering::Acquire);
    let Some(kind) = natives::fighter_kind(cpu) else {
        return;
    };
    let owned = engine.run.as_ref().is_some_and(|run| run.body == body);
    if CpuObject::sighted(owned, kind, natives::status_kind(cpu)) == CpuObject::Arriving {
        // The swap makes the replacement active before its first status. No run binds it, no
        // request is consumed for it and nothing is asked of it. A live measured swap keeps
        // its clock, as for the empty slot.
        if let Some(run) = engine.run.as_mut().filter(|run| {
            run.rounds.phase == Phase::Live
                && run.niche.as_ref().is_some_and(|guide| guide.waiting_for_form(run.rounds.frames))
        }) {
            run.rounds.advance_attempt(Observation::default());
        }
        return;
    }
    let active_run = engine.run.as_ref().filter(|run| start == 0 && run.drill == drill);
    let opponent = active_run.map_or_else(|| OPPONENT.read(), |run| run.opponent);
    if practice_cpu::switching() || opponent.fighter.is_some_and(|wanted| !practice_cpu::answers(wanted, kind as u32)) {
        engine.wake_cpu(Some(body));
        return;
    }
    let id = natives::stage_id();
    let Some(form) = natives::stage_form(id) else {
        return;
    };
    let stage_key = (id, form);
    if engine.run.as_ref().is_some_and(|run| run.wizard.is_some() && run.stage_key != stage_key) {
        super::btt_camera::interrupt_placement();
    }
    if start != 0
        || engine
            .run
            .as_ref()
            .is_none_or(|run| run.drill != drill || run.stage_key != stage_key)
    {
        let Some((stage, p1_start)) = setup_stage(p1) else {
            return;
        };
        engine.wake_cpu(Some(body));
        let config = CONFIG.read();
        let (percent, limit) = match drill {
            Drill::GuardEdge => (config.edge_percent, config.edge_limit),
            Drill::Juggler => (JUGGLE_PERCENT, config.juggle_limit),
        };
        let Some(wizard) = Wizard::new(drill, stage, p1_start, percent, stage_key, natives::fighter_kind(cpu)) else {
            return;
        };
        let last = engine.kept[drill as usize]
            .filter(|(stage, ..)| *stage == stage_key)
            .map(|(_, setup, unmoved)| wizard.remembered(stage, setup, unmoved));
        let editing = start == 2 || last.is_none();
        let mut setup = last.unwrap_or(wizard.setup);
        if drill == Drill::GuardEdge {
            setup.percent = percent;
        }
        engine.run = Some(Run {
            drill,
            opponent,
            zoom: zoom::NativeAttempt::default(),
            reaction: config.reaction[drill as usize],
            threat: ThreatClock::default(),
            stage,
            stage_floor_y: p1_start[1],
            stage_key,
            wizard: editing.then_some(wizard),
            setup,
            defaults: wizard.setup,
            confirm_player: false,
            rounds: Session::new(drill, limit),
            body,
            previous_damage: None,
            damage_stop: false,
            landing_draw: 5,
            jump_delay: 0,
            route_draw: 0,
            hold: Hold::default(),
            zone: Zone::Far,
            ledge_watch: LedgeWatch::default(),
            plan_pending: true,
            route_watch: perfect::RouteWatch::default(),
            player_reach: 0.0,
            player_height: PlanHeight::default(),
            route_history: recovery::RouteHistory::default(),
            guide: None,
            niche: None,
            oil_stall: OilStall::default(),
            oil_draw: [0; 2],
            last_cpu_status: UNRESOLVED,
            bounce: MeteorBounce::default(),
            sdi_start: None,
            player_ledge: None,
            placement: Placement::ResetResources,
            settle: None,
            notice: FixedPath::new(),
        });
        let _ = START.compare_exchange(start, 0, Ordering::AcqRel, Ordering::Acquire);
        ENGINE_ACTIVE.store(true, Ordering::Release);
        WIZARD.store(editing, Ordering::Release);
    }
    if RESTART.swap(false, Ordering::AcqRel) {
        engine.wake_cpu(Some(body));
        if let Some(run) = &mut engine.run {
            run.rounds.restart_drill();
            run.placement = Placement::ResetResources;
        }
    }
    let Engine {
        run: Some(run),
        bests,
        streaks,
        held_cpu,
        rng,
        ..
    } = &mut *engine
    else {
        return;
    };
    let measured_swap = run.rounds.phase == Phase::Live
        && kind == perfect::pyra::MYTHRA
        && game::granted(crate::model::sites_pkg2::STATUS_KIND_OFFSET)
        && run.niche.as_mut().is_some_and(|guide| {
            guide.follow_measured_form(run.rounds.frames, kind, natives::status_kind(cpu))
        });
    let legacy_swap = run.rounds.phase == Phase::Live
        && run.body != body
        && run.niche.as_ref().is_some_and(|guide| guide.accepts_form(Some(kind)));
    if measured_swap || legacy_swap {
        run.body = body;
        if legacy_swap {
            run.niche = None;
        }
        run.guide = None;
        run.previous_damage = None;
        run.damage_stop = false;
        run.player_ledge = None;
        run.plan_pending = true;
        run.route_watch = perfect::RouteWatch::default();
        run.player_reach = 0.0;
        run.player_height = PlanHeight::default();
        run.hold = Hold::default();
        run.zone = Zone::Far;
        run.ledge_watch = LedgeWatch::default();
        run.jump_delay = 0;
    } else if run.body != body {
        run.body = body;
        run.route_history = recovery::RouteHistory::default();
        run.oil_stall = OilStall::default();
        *held_cpu = None;
        run.rounds.restart_drill();
        run.placement = Placement::ResetResources;
        run.previous_damage = None;
        run.damage_stop = false;
        run.player_ledge = None;
    }
    let bucket = if drill == Drill::GuardEdge { run.stage.bucket(run.setup.cpu) } else { 0 };
    let key = (drill, kind, bucket, run.opponent.mode);
    if let Some(wizard) = &mut run.wizard {
        if wizard.step == WizardStep::Player {
            let confirm = std::mem::take(&mut run.confirm_player);
            let placed = wizard.player_floor(run.stage, wizard.cursor(), confirm, |point| {
                snap_player(p1, drill, run.stage, point)
            });
            if let Some(p1) = placed {
                wizard.setup.p1 = p1;
                if confirm {
                    wizard.advance();
                }
            } else if confirm {
                notices::refuse("Pick a point above a floor"); // restriction: edge-juggle-player-no-floor
            }
        }
        post_notice(run, best_time(bests, key), best_time(streaks, key));
        return;
    }
    if !game::granted(crate::model::sites_pkg2::STATUS_KIND_OFFSET)
        || !game::granted(crate::model::sites_pkg2::SITUATION_KIND_OFFSET)
    {
        return;
    }
    let status = natives::status_kind(cpu);
    if kind == 0x5a
        && run.niche.as_ref().is_some_and(|guide| guide.accepts_form(None))
        && !niche_pad::pyra_swapping(status)
    {
        run.niche = None;
    }
    let dead_or_rebirthing = status == konst(C::StatusDead) || status == konst(C::StatusRebirth);
    let damage = Some(natives::damage(cpu, 0)).filter(|d| d.is_finite());
    let damage_stop = natives::tech_in_damage_stop(cpu) == Some(true);
    let side = |x| run.stage.side(x);
    let player_ledge =
        (natives::situation_kind(p1) == konst(C::SituationCliff)).then(|| side(natives::pos_x(p1)));
    let cpu_ledge = side(natives::pos_x(cpu));
    let situation = natives::situation_kind(cpu);
    let trump = drill == Drill::GuardEdge
        && (run.player_ledge == Some(cpu_ledge) || player_ledge == Some(cpu_ledge));
    let obs = Observation {
        dead_or_rebirthing,
        grounded: situation == konst(C::SituationGround),
        cliff_catch: status == konst(C::StatusCliffCatch) && !trump,
        hit: damage_stop && !run.damage_stop
            || damage
                .zip(run.previous_damage)
                .is_some_and(|(now, before)| now > before),
    };
    if obs.hit && run.rounds.phase == Phase::Live {
        run.landing_draw = rng.below(9);
        run.route_draw = rng.next_u32();
        run.plan_pending = true;
    }
    run.previous_damage = damage;
    run.damage_stop = damage_stop;
    run.player_ledge = player_ledge;
    if run.rounds.phase == Phase::Place {
        let [wait, fall, ground, air] = [
            C::StatusWait,
            C::StatusFall,
            C::SituationGround,
            C::SituationAir,
        ]
        .map(konst);
        if [wait, fall, ground, air].contains(&UNRESOLVED) {
            return;
        }
        let frame = PlacementFrame {
            dead_or_rebirthing,
            grounded: situation == ground,
            airborne: situation == air,
            wait: status == wait,
            fall: status == fall,
        };
        match run.placement.action(frame) {
            PlacementAction::RestartPlacement => run.placement = Placement::ResetResources,
            PlacementAction::ResetResources => {
                if reset_recovery_resources(p1, cpu, run.stage) {
                    run.placement = Placement::Grounded;
                }
            }
            PlacementAction::PlaceStart => {
                // Read before the write: its Wait request is not Player 1's status yet. Without the position
                // rows the start cannot be judged, so it is not watched.
                use crate::model::sites_pkg2 as sites;
                let watch = placed_player(p1).watch() && game::granted(sites::POS_X_OFFSET) && game::granted(sites::POS_Y_OFFSET);
                if let Some(at) = try_place(&Bodies { p1, cpu }, drill, run.stage_key, run.stage, run.setup) {
                    run.settle = watch.then_some(at);
                    run.placement = Placement::Airborne;
                }
            }
            PlacementAction::AwaitNativeWait => {}
            action @ (PlacementAction::RequestFall | PlacementAction::HoldStart) => {
                let falling = action == PlacementAction::HoldStart;
                let watched = run.settle.map(|at| (placed_player(p1), on_start(p1, at), run.rounds.player_rewrites()));
                let (rewrite, ready) = PlacedPlayer::before_ready(watched, falling, drill);
                if let (true, Some(at)) = (rewrite, run.settle) {
                    placement::stand(p1, at, if run.setup.cpu[0] < at[0] { -1.0 } else { 1.0 });
                    run.rounds.player_rewritten();
                }
                if falling {
                    placement::hold(cpu, run.setup.cpu);
                    *held_cpu = Some(body);
                } else {
                    state::status_request(cpu, fall);
                }
                if ready {
                    run.rounds.placed();
                    run.zoom = zoom::NativeAttempt::default();
                    run.placement = Placement::ResetResources;
                }
            }
        }
    } else {
        let before = run.rounds.phase;
        if matches!(before, Phase::Ready(_)) && *held_cpu == Some(body) {
            placement::hold(cpu, run.setup.cpu);
        }
        let zoom = matches!(run.opponent.mode, Mode::Drill(DrillId::Zoom));
        if before == Phase::Live && zoom {
            let read = run.zoom.observe_zoom(p1, cpu, status, obs);
            run.rounds.advance_zoom(&mut run.zoom.attempt, obs, read, run.opponent.window);
        } else {
            run.rounds.advance_attempt(obs);
        }
        if !zoom {
            trace_attempt(run, before, Bodies { p1, cpu }, kind, damage);
        }
        if before == Phase::Live && matches!(run.rounds.phase, Phase::Result(..)) {
            run.route_history.complete();
        }
        if matches!(before, Phase::Ready(_)) && run.rounds.phase == Phase::Live {
            run.route_history.start();
            if kind == game_watch::KIND && drill == Drill::GuardEdge && run.opponent.mode == Mode::Perfect {
                run.oil_stall.next_attempt();
                run.oil_draw = [rng.next_u32(), rng.next_u32()];
            }
            run.landing_draw = rng.below(9);
            run.jump_delay = 0;
            run.route_draw = rng.next_u32();
            run.plan_pending = true;
            run.route_watch = perfect::RouteWatch::default();
            run.player_reach = 0.0;
            run.player_height = PlanHeight::default();
            run.guide = None;
            run.niche = None;
            run.hold = Hold::default();
            run.zone = Zone::Far;
            run.ledge_watch = LedgeWatch::default();
            run.last_cpu_status = UNRESOLVED;
            run.bounce = MeteorBounce::default();
            run.threat = ThreatClock::default();
            run.sdi_start = None;
            if held_cpu.take() == Some(body) {
                state::set_sleep(cpu, false);
            }
        }
        if run.rounds.record_result(bests, key).is_some() && drill == Drill::GuardEdge {
            crate::model::practice_games::record_max(streaks, key, u32::from(run.rounds.streak));
        }
    }
    post_notice(run, best_time(bests, key), best_time(streaks, key));
}
pub fn look(active: bool) -> Marks {
    let empty = Marks { looks: [None; super::btt::LOOKS], pointer: None };
    if !active || !wizard_active() {
        return empty;
    }
    let Some(engine) = ENGINE.try_lock() else {
        return empty;
    };
    let Some(wizard) = engine.run.as_ref().and_then(|run| run.wizard) else {
        return empty;
    };
    wizard.marks()
}
