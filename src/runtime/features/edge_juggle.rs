//! Native Training adapter for the edge and juggle games. No additional game hooks.
use std::fmt::Write;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

use hos_lock::{Mutex, Published};

use super::{cpu::konst, mashout::FloorRay, notices, practice, recorder, states};
use crate::model::chords::{field, logical as key, Sample};
use crate::model::cpu::{
    ids::{C, UNRESOLVED},
    rng::Rng,
};
use crate::model::edge_juggle::perfect::{
    self, AirPhysics, Correction, JumpTiming, LandingFacts, LandingOptions, MeteorBounce, Route,
    RouteFacts, ThreatClock,
};
use crate::model::edge_juggle::{
    Difficulty, Drill, Observation, Outcome, Phase, Reaction, Session, Setup, Stage, Wizard,
    WizardStep,
};
use crate::model::notify::DEFAULT_COLOR;
use crate::model::practice_games::best_time;
use crate::model::settings::Snapshot;
use crate::model::storage::FixedPath;
use crate::runtime::{
    game::{self, natives},
    published, scene, state_natives as state,
};

const NOTICE: u8 = 0x34;
const JUGGLE_PERCENT: u16 = 60;
type StageKey = (i32, u32);
type BestKey = (Drill, i32, u8, Difficulty);

#[derive(Clone, Copy)]
struct Config {
    edge_percent: u16,
    edge_limit: Option<u16>,
    juggle_limit: Option<u16>,
    difficulty: [Difficulty; 2],
    reaction: [Reaction; 2],
}
static CONFIG: Published<Config> = Published::new(Config {
    edge_percent: 100,
    edge_limit: Some(10),
    juggle_limit: Some(10),
    difficulty: [Difficulty::LevelNine; 2],
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
    kept: [Option<(StageKey, Setup)>; 2],
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
    difficulty: Difficulty,
    reaction: Reaction,
    threat: ThreatClock,
    stage_key: StageKey,
    stage: Stage,
    stage_floor_y: f32,
    wizard: Option<Wizard>,
    defaults: Setup,
    cursor: [f32; 2],
    confirm_player: bool,
    setup: Setup,
    rounds: Session,
    body: CpuBody,
    previous_damage: Option<f32>,
    damage_stop: bool,
    landing_draw: u32,
    jump_delay: u8,
    route_draw: u32,
    jump_timing: JumpTiming,
    plan_pending: bool,
    last_route: Option<Route>,
    guide: Option<RecoveryGuide>,
    last_cpu_status: i32,
    bounce: MeteorBounce,
    sdi_start: Option<u32>,
    player_ledge: Option<usize>,
    placement: Placement,
    notice: FixedPath<128>,
}
#[derive(Clone, Copy)]
enum RecoveryGuide {
    High {
        lip: [f32; 2],
        inward: f32,
    },
    Cross {
        exit: [f32; 2],
        dir: f32,
        band: [f32; 2],
    },
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

#[derive(Clone, Copy)]
enum Placement {
    ResetResources,
    Grounded,
    Airborne,
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
    SPECIAL_TABLE.store(special_table, Ordering::Release);
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
        difficulty: [Drill::GuardEdge, Drill::Juggler]
            .map(|drill| Difficulty::from_option(select(drill.difficulty_rows().difficulty))),
        reaction: [Drill::GuardEdge, Drill::Juggler].map(|drill| {
            let rows = drill.difficulty_rows();
            Reaction::from_settings(select(rows.reaction), select(rows.frames))
        }),
    });
}

pub fn start(edit: bool) {
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

pub fn allowed() -> bool {
    INSTALLED.load(Ordering::Acquire) && scene::is_native_training()
}

fn wanted() -> Option<Drill> {
    practice::active()
        .and_then(Drill::from_game)
        .filter(|_| scene::is_native_training())
}

pub(super) fn running() -> bool {
    wanted().is_some()
}

pub unsafe fn controls_cpu(accessor: usize) -> bool {
    if !INSTALLED.load(Ordering::Acquire) {
        return false;
    }
    running()
        && !published::HOT_SWAP_IN_PROGRESS.load(Ordering::Acquire)
        && natives::module_accessor(1) == Some(accessor)
}

/// The final mapped pad read by P1, after the existing delay and playback stages.
pub(super) fn observe_player_pad(buttons: u32) {
    if running() {
        P1_BUTTONS.store(buttons, Ordering::Release);
    }
}

pub unsafe fn ai_correction(ai: usize, accessor: usize, buttons: u32, stick_y: f32) -> Correction {
    if !INSTALLED.load(Ordering::Acquire) {
        return Correction::default();
    }
    let Some(mut engine) = ENGINE.try_lock() else {
        return Correction::default();
    };
    let Some(run) = engine.run.as_mut().filter(|run| {
        run.difficulty == Difficulty::Perfect
            && run.rounds.phase == Phase::Live
            && run.wizard.is_none()
    }) else {
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
    let status = natives::status_kind(accessor);
    let Some(p1) = natives::module_accessor(0) else {
        return Correction::default();
    };
    if run.drill == Drill::Juggler && status != UNRESOLVED {
        let reflect = konst(C::StatusDamageFlyReflectD);
        let previous = std::mem::replace(&mut run.last_cpu_status, status);
        let bounce = status == reflect
            && [konst(C::StatusDamageFlyMeteor), reflect].contains(&previous)
            && previous != UNRESOLVED;
        let correction = run.bounce.step(
            bounce,
            natives::status_kind(p1) == konst(C::StatusWait)
                && P1_BUTTONS.load(Ordering::Acquire) == 0,
            buttons,
        );
        if correction != Correction::default() {
            return correction;
        }
    }
    let falling = status != UNRESOLVED
        && [C::StatusFall, C::StatusFallAerial, C::StatusDamageFall]
            .map(konst)
            .contains(&status);
    let position = [natives::pos_x(accessor), natives::pos_y(accessor)];
    if run.drill == Drill::GuardEdge {
        if !falling && status != konst(C::StatusJumpAerial) {
            run.jump_timing = JumpTiming::default();
            return Correction::default();
        }
        return recovery_pad(run, p1, ai, accessor, position, buttons, stick_y);
    }
    let correction = air_hit_recovery(run, p1, accessor, position, status);
    if correction != Correction::default() {
        return correction;
    }
    let [left, right] = run.stage.ledges;
    if position[0] <= left[0] || position[0] >= right[0] {
        return Correction::default();
    }
    let air = konst(C::SituationAir);
    if air != UNRESOLVED && natives::situation_kind(accessor) == air {
        let correction = landing_dodge(run, p1, accessor, buttons);
        if correction != Correction::default() {
            return correction;
        }
    }
    if !falling {
        return Correction::default();
    }
    let facts = LandingFacts {
        cpu: position,
        player: [natives::pos_x(p1), natives::pos_y(p1)],
        main_floor: run.stage.ledges.map(|point| point[0]),
        stock_stick_y: stick_y,
        stock_buttons: buttons,
    };
    let options = landing_options(run, p1, accessor, facts);
    perfect::landing_variant(facts, options, run.landing_draw)
}

unsafe fn landing_options(run: &Run, p1: usize, cpu: usize, facts: LandingFacts) -> LandingOptions {
    if run.landing_draw < 6 || facts.stock_buttons & !perfect::FLICK_JUMP != 0 {
        return LandingOptions::default();
    }
    let mut out = LandingOptions::default();
    let [x, y] = facts.cpu;
    let player_reach = body_top(p1, facts.player[1]).unwrap_or(f32::INFINITY);
    let player_near = (x - facts.player[0]).abs() <= player_reach;
    match run.landing_draw {
        6 if !player_near => {
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
        7 if player_near => {
            let term = konst(C::TermContJumpAerial);
            let air = air_physics(cpu);
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

unsafe fn air_hit_recovery(
    run: &mut Run,
    p1: usize,
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
    let player = [natives::pos_x(p1), natives::pos_y(p1)];
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

unsafe fn landing_dodge(run: &mut Run, p1: usize, cpu: usize, buttons: u32) -> Correction {
    const ATTACK_INPUT: u64 = u64::MAX;
    let input = P1_BUTTONS.load(Ordering::Acquire);
    let attacks = [
        C::StatusAttack,
        C::StatusAttackDash,
        C::StatusAttackS3,
        C::StatusAttackHi3,
        C::StatusAttackLw3,
        C::StatusAttackAir,
        C::StatusAttackHi4Start,
        C::StatusAttackLw4Start,
        C::StatusAttackS4Start,
        C::StatusAttackHi4,
        C::StatusAttackLw4,
        C::StatusAttackS4,
        C::StatusSpecialN,
        C::StatusSpecialS,
        C::StatusSpecialHi,
        C::StatusSpecialLw,
    ];
    let status = natives::status_kind(p1);
    let motion = (status != UNRESOLVED && attacks.map(konst).contains(&status))
        .then(|| natives::motion_kind(p1))
        .filter(|&motion| motion != 0);
    let attack = motion.or_else(|| (input & 3 != 0).then_some(ATTACK_INPUT));
    if attack.is_none() && status != konst(C::StatusWait) {
        run.threat = ThreatClock::default();
        return Correction::default();
    }
    let escape = konst(C::TermContEscapeAir);
    let can_dodge = escape != UNRESOLVED && natives::is_enable_transition_term(cpu, escape);
    let contact = if attack.is_some() && can_dodge {
        let (px, py) = super::recording_host::fighter_speed(p1).unwrap_or_default();
        let (cx, cy) = super::recording_host::fighter_speed(cpu).unwrap_or_default();
        let Some(contact) = super::hitbox::attack_contact(p1, cpu, [cx - px, cy - py]) else {
            return Correction::default();
        };
        contact
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
    AirPhysics {
        horizontal_speed: read("air_speed_x_stable"),
        horizontal_acceleration: read("air_accel_x_add"),
        gravity: read("air_accel_y"),
        terminal_speed: read("air_speed_y_stable"),
        jump_speed: read("jump_aerial_y"),
    }
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

unsafe fn body_top(fighter: usize, foot: f32) -> Option<f32> {
    let mut highest = foot;
    super::hitbox::fighter_hurt_capsules(fighter, |part, _| {
        highest = highest.max(part.ends[0][1].max(part.ends[1][1]) + part.radius);
    });
    (highest.is_finite() && highest > foot).then_some(highest - foot)
}

unsafe fn route_guide(
    run: &mut Run,
    p1: usize,
    cpu: usize,
    air: AirPhysics,
    pos: [f32; 2],
    speed: [f32; 2],
    special: Option<perfect::SpecialReach>,
) -> Option<RecoveryGuide> {
    let [left, right] = run.stage.ledges;
    let (midpoint, limits) =
        perfect::outline_probe(natives::camera_limit_range()?, run.stage.blast())?;
    let ray = FloorRay::new(p1)?;
    let outline = super::tech_surfaces::training_outline(&ray, midpoint, limits)?;
    let player = [natives::pos_x(p1), natives::pos_y(p1)];
    let player_reach = body_top(p1, player[1])?;
    let body = body_top(cpu, pos[1])?;
    let jumps = remaining_jumps(cpu)?;
    let bottom = run.stage.blast()[3];
    let jump_rise = if jumps == 0 {
        0.0
    } else {
        air.jump_speed.powi(2) / (2.0 * air.gravity)
    };
    let [special_horizontal, special_rise] =
        special.map_or([0.0; 2], |reach| [reach.horizontal, reach.rise]);
    let facts = RouteFacts {
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
    let [near, high, low, opposite] = perfect::possible_routes(&facts);
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
    let band = perfect::corridor(facts.outline, facts.ledges, bottom, body, facts.fall_budget);
    let opposite = opposite.filter(|plan| {
        band.is_some_and(|band| air.crossing(pos, speed, plan.waypoints[2], band, jumps))
    });
    let plan = perfect::choose_route([near, high, low, opposite], run.last_route, run.route_draw)?;
    run.last_route = Some(plan.route);
    match plan.route {
        Route::Opposite => {
            let exit = plan.waypoints[2];
            let dir = (exit[0] - pos[0]).signum();
            Some(RecoveryGuide::Cross {
                exit,
                dir,
                band: band?,
            })
        }
        Route::High => Some(RecoveryGuide::High { lip, inward }),
        Route::LateLow => {
            run.jump_delay = run.jump_delay.max(1);
            None
        }
        Route::Near => None,
    }
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
    reach.valid().then_some(reach)
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
    buttons: u32,
    stick_y: f32,
) -> Correction {
    let jump = konst(C::TermContJumpAerial);
    let can_jump = jump != UNRESOLVED && natives::is_enable_transition_term(cpu, jump);
    let physics = air_physics(cpu);
    let Some((vx, vy)) = super::recording_host::fighter_speed(cpu).filter(|_| physics.valid())
    else {
        run.jump_timing = JumpTiming::default();
        return Correction::default();
    };
    let speed = [vx, vy];
    let jumps = remaining_jumps(cpu);
    let recovery = if run.plan_pending || jumps == Some(0) {
        recovery_snapshot(ai)
    } else {
        None
    };
    let special = recovery.and_then(|(snapshot, available)| {
        if available {
            special_reach(ai, snapshot)
        } else {
            None
        }
    });
    if run.plan_pending {
        run.plan_pending = false;
        run.guide = route_guide(run, p1, cpu, physics, pos, speed, special);
    }
    let attack = buttons & perfect::ATTACK != 0;
    let can_punish = attack && player_in_lag(p1);
    if let Some(guide) = run.guide {
        match guide {
            RecoveryGuide::High { lip, inward } if !can_punish => {
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
                        correction.release = detour;
                        return correction;
                    }
                }
                run.guide = None;
            }
            RecoveryGuide::Cross { exit, dir, band }
                if (exit[0] - pos[0]) * dir > 0.0
                    && physics.crossing(pos, speed, exit, band, u8::from(can_jump)) =>
            {
                let release = (buttons & perfect::JUMP)
                    | perfect::avoid_detour(buttons, true, can_punish, dir).release;
                return Correction {
                    stick: Some([dir, 0.0]),
                    release,
                    ..Correction::default()
                };
            }
            RecoveryGuide::Cross { exit, dir, .. }
                if (exit[0] - pos[0]) * dir <= 0.0 && can_jump =>
            {
                run.guide = None;
                // Continue to the far ledge from outside the stage.
                return Correction {
                    stick: Some([-dir, 0.0]),
                    press: perfect::JUMP,
                    ..Correction::default()
                };
            }
            _ => run.guide = None,
        }
    }
    let [left, right] = run.stage.ledges;
    let ledge = if pos[0] < left[0] {
        left
    } else if pos[0] > right[0] {
        right
    } else {
        run.jump_timing = JumpTiming::default();
        return Correction::default();
    };
    if !can_jump && jumps == Some(0) {
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
                    return pad;
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
                    return pad;
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
            return correction;
        }
    }
    let mut correction = run
        .jump_timing
        .step(buttons, can_jump, run.jump_delay, |delay| {
            physics.reaches_with_jump(pos, speed, ledge, delay, run.stage.blast()[3])
        });
    if run.jump_timing.waiting() || correction.press != 0 {
        correction.stick = Some([(ledge[0] - pos[0]).signum(), stick_y]);
    }
    correction
}

fn axis(pressed: u16, negative: u8, positive: u8) -> i16 {
    i16::from(pressed & (1 << positive) != 0) - i16::from(pressed & (1 << negative) != 0)
}

fn moved(point: [f32; 2], stick: [f32; 2]) -> [f32; 2] {
    [0, 1].map(|i| {
        point[i]
            + if stick[i].abs() > 0.3 {
                2.0 * stick[i]
            } else {
                0.0
            }
    })
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

    if key_down(key::B) {
        run.confirm_player = false;
        if !wizard.back() {
            practice::finish(run.drill.game());
            BACK.store(true, Ordering::Release);
        }
        return true;
    }
    if key_down(key::X) {
        wizard.setup = run.defaults;
        run.cursor = wizard.setup.p1;
        run.confirm_player = false;
        wizard.step = WizardStep::Preview;
        return true;
    }
    match wizard.step {
        WizardStep::Area => {
            let delta = horizontal + vertical * 2;
            if delta != 0 {
                wizard.change_area(run.stage, i32::from(delta));
            }
        }
        WizardStep::Cpu => wizard.move_cpu(run.stage, moved(wizard.setup.cpu, stick)),
        WizardStep::Percent => wizard.change_percent(horizontal + vertical * 10),
        WizardStep::Player => {
            if let Some(cursor) = run.stage.camera_point(moved(run.cursor, stick)) {
                run.cursor = cursor;
            }
            run.confirm_player |= key_down(key::A);
            return true;
        }
        WizardStep::Preview => {}
    }
    if key_down(key::A) || wizard.step == WizardStep::Preview && key_down(key::PLUS) {
        if wizard.step == WizardStep::Preview {
            run.setup = wizard.setup;
            kept[run.drill as usize] = Some((run.stage_key, run.setup));
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

unsafe fn setup_stage(p1: usize) -> Option<(Stage, [f32; 2])> {
    let bounds @ [left, right, top, bottom] = natives::camera_limit_range()?;
    let x = left.midpoint(right);
    let ray = FloorRay::new(p1)?;
    let limits = [top, bottom];
    let floor = ray.floor(x, limits, false)?;
    let blast = super::tech_surfaces::training_blast_bounds()?;
    let stage = Stage::new(
        blast,
        bounds,
        [
            ledge(&ray, floor, false, limits)?,
            ledge(&ray, floor, true, limits)?,
        ],
    )?;
    Some((stage, [x, floor.height(x)?]))
}

unsafe fn snap_player(p1: usize, drill: Drill, stage: Stage, [x, y]: [f32; 2]) -> Option<[f32; 2]> {
    let [_, _, top, bottom] = natives::camera_limit_range()?;
    let ray = FloorRay::new(p1)?;
    let platforms = drill == Drill::Juggler;
    let x = if platforms {
        x
    } else {
        x.clamp(stage.ledges[0][0], stage.ledges[1][0])
    };
    let top = if platforms { (y + 1.0).min(top) } else { top };
    let floor = ray.floor(x, [top, bottom], platforms)?;
    Some([x, floor.height(x)?])
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

unsafe fn try_place(bodies: &Bodies, drill: Drill, stage: Stage, setup: Setup) -> bool {
    let Some([x, y]) = snap_player(bodies.p1, drill, stage, setup.p1) else {
        return false;
    };
    let [wait, fall, ground, air] = [
        C::StatusWait,
        C::StatusFall,
        C::SituationGround,
        C::SituationAir,
    ]
    .map(konst);
    if [wait, fall, ground, air].contains(&UNRESOLVED) {
        return false;
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
        return false;
    }
    let jump_count = konst(C::WorkIntJumpCount);
    if jump_count == UNRESOLVED || !game::granted(crate::model::sites_pkg4::WORK_SET_INT_OFFSET) {
        return false;
    }
    let [cx, cy] = setup.cpu;
    let damage = natives::damage(bodies.cpu, 0);
    if !damage.is_finite() {
        return false;
    }
    let facing = if cx < x { -1.0 } else { 1.0 };
    state::clear_speed_all(bodies.p1);
    state::init_pos(bodies.p1, &state::Vector3f { x, y, z: 0.0 });
    state::set_lr(bodies.p1, facing);
    state::set_situation_kind(bodies.p1, ground);
    state::status_request(bodies.p1, wait);
    state::clear_speed_all(bodies.cpu);
    state::set_pos(
        bodies.cpu,
        &state::Vector3f {
            x: cx,
            y: cy,
            z: 0.0,
        },
    );
    state::set_lr(bodies.cpu, -facing);
    state::set_situation_kind(bodies.cpu, air);
    natives::set_int(bodies.cpu, 0, jump_count);
    state::heal(bodies.cpu, -damage);
    state::add_damage(bodies.cpu, f32::from(setup.percent));
    state::status_request(bodies.cpu, fall);
    true
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
            area,
            cpu: [x, y],
            p1: [px, py],
            percent,
        } = wizard.setup;
        let number = match wizard.step {
            WizardStep::Area => 0,
            WizardStep::Cpu => usize::from(run.drill == Drill::GuardEdge),
            WizardStep::Percent => 1,
            WizardStep::Player => 2,
            WizardStep::Preview => 3,
        };
        header = [
            ["Edge · 1/4", "Edge · 2/4", "Edge · 3/4", "Edge · 4/4"],
            [
                "Juggle · 1/4",
                "Juggle · 2/4",
                "Juggle · 3/4",
                "Juggle · 4/4",
            ],
        ][run.drill as usize][number];
        let _ = match wizard.step {
            WizardStep::Area => write!(text, "{} · D-pad choose · A next", area.title()),
            WizardStep::Cpu if run.drill == Drill::Juggler => write!(
                text,
                "CPU x {x:.0} · Height {:+.0} · Stick move · A next",
                y - run.stage_floor_y,
            ),
            WizardStep::Cpu => write!(text, "CPU ({x:.0}, {y:.0}) · Stick move · A next"),
            WizardStep::Percent => write!(text, "{percent}% · D-pad change · A next"),
            WizardStep::Player => write!(text, "P1 ({px:.0}, {py:.0}) · Stick move · A next"),
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
        notices::upsert(NOTICE, header, text.as_str(), 180, DEFAULT_COLOR);
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
    if !natives::is_operation_cpu(1)
        || OPEN.load(Ordering::Acquire)
        || states::is_loading()
        || recorder::recording_or_playback()
        || published::input_owners().covers_strip()
    {
        engine.wake_cpu(Some(body));
        if let Some(run) = &mut engine.run {
            run.rounds.discard_attempt();
            run.placement = Placement::ResetResources;
            run.notice = FixedPath::new();
        }
        return;
    }
    let id = natives::stage_id();
    let Some(form) = natives::stage_form(id) else {
        return;
    };
    let stage_key = (id, form);
    let start = START.load(Ordering::Acquire);
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
        let Some(wizard) = Wizard::new(drill, stage, p1_start, percent) else {
            return;
        };
        let last = engine.kept[drill as usize]
            .filter(|(stage, _)| *stage == stage_key)
            .map(|(_, setup)| setup);
        let editing = start == 2 || last.is_none();
        let mut setup = last.unwrap_or(wizard.setup);
        if drill == Drill::GuardEdge {
            setup.percent = percent;
        }
        engine.run = Some(Run {
            drill,
            difficulty: config.difficulty[drill as usize],
            reaction: config.reaction[drill as usize],
            threat: ThreatClock::default(),
            stage,
            stage_floor_y: p1_start[1],
            stage_key,
            wizard: editing.then_some(wizard),
            setup,
            defaults: wizard.setup,
            cursor: wizard.setup.p1,
            confirm_player: false,
            rounds: Session::new(drill, limit),
            body,
            previous_damage: None,
            damage_stop: false,
            landing_draw: 5,
            jump_delay: 0,
            route_draw: 0,
            jump_timing: JumpTiming::default(),
            plan_pending: true,
            last_route: None,
            guide: None,
            last_cpu_status: UNRESOLVED,
            bounce: MeteorBounce::default(),
            sdi_start: None,
            player_ledge: None,
            placement: Placement::ResetResources,
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
    if run.body != body {
        run.body = body;
        *held_cpu = None;
        run.rounds.restart_drill();
        run.placement = Placement::ResetResources;
        run.previous_damage = None;
        run.damage_stop = false;
        run.player_ledge = None;
    }
    let Some(kind) = natives::fighter_kind(cpu) else {
        return;
    };
    let key = (drill, kind, run.setup.area as u8, run.difficulty);
    if let Some(wizard) = &mut run.wizard {
        if wizard.step == WizardStep::Player {
            if let Some(p1) = snap_player(p1, drill, run.stage, run.cursor) {
                wizard.setup.p1 = p1;
                if run.confirm_player {
                    wizard.advance();
                }
            }
            run.confirm_player = false;
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
    let dead_or_rebirthing = status == konst(C::StatusDead) || status == konst(C::StatusRebirth);
    let damage = Some(natives::damage(cpu, 0)).filter(|d| d.is_finite());
    let damage_stop = natives::tech_in_damage_stop(cpu) == Some(true);
    let [left, right] = run.stage.ledges;
    let side = |x| usize::from(x > left[0].midpoint(right[0]));
    let player_ledge =
        (natives::situation_kind(p1) == konst(C::SituationCliff)).then(|| side(natives::pos_x(p1)));
    let cpu_ledge = side(natives::pos_x(cpu));
    let trump = drill == Drill::GuardEdge
        && (run.player_ledge == Some(cpu_ledge) || player_ledge == Some(cpu_ledge));
    let obs = Observation {
        dead_or_rebirthing,
        grounded: natives::situation_kind(cpu) == konst(C::SituationGround),
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
        if dead_or_rebirthing {
            run.placement = Placement::ResetResources;
        } else {
            match run.placement {
                Placement::ResetResources => {
                    if reset_recovery_resources(p1, cpu, run.stage) {
                        run.placement = Placement::Grounded;
                    }
                }
                Placement::Grounded if obs.grounded => {
                    if try_place(&Bodies { p1, cpu }, drill, run.stage, run.setup) {
                        run.placement = Placement::Airborne;
                    }
                }
                Placement::Grounded => run.placement = Placement::ResetResources,
                Placement::Airborne if status == konst(C::StatusFall) => {
                    let [x, y] = run.setup.cpu;
                    state::set_sleep(cpu, true);
                    state::clear_speed_all(cpu);
                    state::set_pos(cpu, &state::Vector3f { x, y, z: 0.0 });
                    *held_cpu = Some(body);
                    run.rounds.placed();
                    run.placement = Placement::ResetResources;
                }
                Placement::Airborne => {
                    state::status_request(cpu, konst(C::StatusFall));
                }
            }
        }
    } else {
        let before = run.rounds.phase;
        run.rounds.advance_attempt(obs);
        if matches!(before, Phase::Ready(_)) && run.rounds.phase == Phase::Live {
            run.landing_draw = rng.below(9);
            run.jump_delay = rng.below(9) as u8;
            run.route_draw = rng.next_u32();
            run.plan_pending = true;
            run.guide = None;
            run.jump_timing = JumpTiming::default();
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
pub fn look(active: bool) -> [Option<([f32; 2], crate::model::panes::EggLook)>; super::btt::LOOKS] {
    use crate::model::panes::{EggLook, EggPicture, EGG_WHOLE};
    let mut marks = [None; super::btt::LOOKS];
    if !active || !wizard_active() {
        return marks;
    }
    let Some(engine) = ENGINE.try_lock() else {
        return marks;
    };
    let Some(wizard) = engine.run.as_ref().and_then(|run| run.wizard) else {
        return marks;
    };
    marks[0] = Some((
        wizard.setup.cpu,
        EggLook {
            scale: 0.3,
            ..EGG_WHOLE
        },
    ));
    if matches!(wizard.step, WizardStep::Player | WizardStep::Preview) {
        marks[super::btt::LOOKS - 1] = Some((
            wizard.setup.p1,
            EggLook {
                picture: EggPicture::Cursor,
                ..EGG_WHOLE
            },
        ));
    }
    marks
}
