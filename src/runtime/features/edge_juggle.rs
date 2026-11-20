//! Native Training adapter for the edge and juggle games. No additional game hooks.
use std::fmt::Write;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use hos_lock::{Mutex, Published};

use super::{cpu::konst, mashout::FloorRay, notices, practice, recorder, states};
use crate::model::chords::{field, logical as key, Sample};
use crate::model::cpu::ids::{C, UNRESOLVED};
use crate::model::edge_juggle::{
    Difficulty, Drill, Observation, Outcome, Phase, Session, Setup, Stage, Wizard, WizardStep,
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
}
static CONFIG: Published<Config> = Published::new(Config {
    edge_percent: 100,
    edge_limit: Some(10),
    juggle_limit: Some(10),
    difficulty: [Difficulty::LevelNine; 2],
});
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
}
static ENGINE: Mutex<Engine> = Mutex::new(Engine {
    run: None,
    kept: [None; 2],
    bests: [None; 24],
    streaks: [None; 24],
    held_cpu: None,
});

struct Run {
    drill: Drill,
    difficulty: Difficulty,
    stage_key: StageKey,
    stage: Stage,
    wizard: Option<Wizard>,
    defaults: Setup,
    cursor: [f32; 2],
    confirm_player: bool,
    setup: Setup,
    rounds: Session,
    body: CpuBody,
    previous_damage: Option<f32>,
    waiting_for_fall: bool,
    notice: FixedPath<128>,
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
        difficulty: [Drill::GuardEdge, Drill::Juggler].map(|drill| {
            Difficulty::from_option(select(drill.difficulty_rows().difficulty))
        }),
    });
}

pub fn start(edit: bool) {
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
    scene::is_native_training()
}

fn wanted() -> Option<Drill> {
    practice::active()
        .and_then(Drill::from_game)
        .filter(|_| scene::is_native_training())
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
    if let Some(wizard) = run.wizard {
        let Setup {
            area,
            cpu: [x, y],
            p1: [px, py],
            percent,
        } = wizard.setup;
        let _ = match wizard.step {
            WizardStep::Area => write!(
                text,
                "1/4 · Area · {} · D-pad choose · A next",
                area.title()
            ),
            WizardStep::Cpu => write!(
                text,
                "{}/4 · CPU ({x:.0}, {y:.0}) · Stick move · A next",
                if run.drill == Drill::GuardEdge { 2 } else { 1 }
            ),
            WizardStep::Percent => write!(text, "2/4 · {percent}% · D-pad change · A next"),
            WizardStep::Player => write!(text, "3/4 · P1 ({px:.0}, {py:.0}) · Stick move · A next"),
            WizardStep::Preview => write!(
                text,
                "4/4 · CPU ({x:.0}, {y:.0}) · {percent}% · P1 ({px:.0}, {py:.0}) · A start"
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
        notices::upsert(
            NOTICE,
            run.drill.game().title(),
            text.as_str(),
            180,
            DEFAULT_COLOR,
        );
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
            run.waiting_for_fall = false;
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
            stage,
            stage_key,
            wizard: editing.then_some(wizard),
            setup,
            defaults: wizard.setup,
            cursor: wizard.setup.p1,
            confirm_player: false,
            rounds: Session::new(drill, limit),
            body,
            previous_damage: None,
            waiting_for_fall: false,
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
            run.waiting_for_fall = false;
        }
    }
    let Engine {
        run: Some(run),
        bests,
        streaks,
        held_cpu,
        ..
    } = &mut *engine
    else {
        return;
    };
    if run.body != body {
        run.body = body;
        *held_cpu = None;
        run.rounds.restart_drill();
        run.waiting_for_fall = false;
        run.previous_damage = None;
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
    let obs = Observation {
        dead_or_rebirthing,
        grounded: natives::situation_kind(cpu) == konst(C::SituationGround),
        cliff_catch: status == konst(C::StatusCliffCatch),
        hit: damage
            .zip(run.previous_damage)
            .is_some_and(|(now, before)| now > before),
    };
    run.previous_damage = damage;
    if run.rounds.phase == Phase::Place {
        if dead_or_rebirthing {
            run.waiting_for_fall = false;
        } else if run.waiting_for_fall && status == konst(C::StatusFall) {
            let [x, y] = run.setup.cpu;
            state::set_sleep(cpu, true);
            state::clear_speed_all(cpu);
            state::set_pos(cpu, &state::Vector3f { x, y, z: 0.0 });
            *held_cpu = Some(body);
            run.rounds.placed();
            run.waiting_for_fall = false;
        } else {
            run.waiting_for_fall = try_place(&Bodies { p1, cpu }, drill, run.stage, run.setup);
        }
    } else {
        let before = run.rounds.phase;
        run.rounds.advance_attempt(obs);
        if matches!(before, Phase::Ready(_)) && run.rounds.phase == Phase::Live {
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
