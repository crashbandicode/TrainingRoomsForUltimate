//! Stage Tech practicer. UI/input/layout exchange only plain snapshots; the
//! launch adapter stays on the P1 game thread. No default hitlag resolver exists.
use super::{
    notices,
    tech_adapter::{Adapter, HitlagResolver},
    tech_surfaces::{self, NativeSource, PlatformScan, ScanState},
};
use crate::model::btt::{Pad, LOOKS};
use crate::model::chords::{field, Sample};
use crate::model::cpu::hash::hash40;
use crate::model::cpu::rng::Rng;
use crate::model::panes::{EggLook, EggPicture, EGG_WHOLE};
use crate::model::settings::Snapshot;
use crate::model::storage::FixedPath;
use crate::model::tech::{
    controller::{Controller, Event, Outcome, Setup},
    discovery::Bootstrap,
    picker::{self, Pick, Picker},
    planner,
    session::{Bests, Length, Phase},
    solver,
    surface::{Kind, RememberedSpot, StageEpoch, Surfaces, Walk},
    Vec2,
};
use crate::model::tech_launch::{DecisionSnapshot, FlightParams, Launcher};
use crate::model::tech_settings::{self, Config};
use crate::runtime::game::natives;
use crate::runtime::{published, scene};
use hos_lock::{Mailbox, Mutex, Published};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

const NOTICE: u8 = 0x32;
static CONFIG: Published<Option<Config>> = Published::new(None);
static OPEN: AtomicBool = AtomicBool::new(false);
static START: AtomicBool = AtomicBool::new(false);
static EDIT: AtomicBool = AtomicBool::new(false);
static RESET: AtomicBool = AtomicBool::new(false);
static RUNNING: AtomicBool = AtomicBool::new(false);
static PLACING: AtomicBool = AtomicBool::new(false);
static GUARD_PRESS: AtomicBool = AtomicBool::new(false);
static HITLAG: OnceLock<HitlagResolver> = OnceLock::new();
static TUMBLE_IDS: OnceLock<[i32; 7]> = OnceLock::new();
static DECISIONS: Mailbox<DecisionSnapshot, 4> = Mailbox::new();
static GRADING_IDS: OnceLock<[i32; 11]> = OnceLock::new();
static OUTCOMES: Mailbox<Outcome, 4> = Mailbox::new();
struct ViewState {
    picker: Picker,
    surfaces: Surfaces,
    range: Option<[f32; 4]>,
    start: bool,
}
static VIEW: Mutex<ViewState> = Mutex::new(ViewState {
    picker: Picker::EMPTY,
    surfaces: Surfaces::EMPTY,
    range: None,
    start: false,
});
struct Practice {
    adapter: Adapter,
    controller: Controller,
    config: Config,
    pending: bool,
    reroll: bool,
    sampled: Option<(Setup, picker::Spot, [f32; 2])>,
    approximate: bool,
    scan: Option<PlatformScan>,
    scan_done: bool,
    bootstrap: Option<Bootstrap>,
    discovery_failed: bool,
    epoch: Option<StageEpoch>,
    fighter_kind: i32,
    rng: Rng,
    after_reaction: u16,
    planning_failed: bool,
    decision: Option<(DecisionSnapshot, u16)>,
    remembered: Option<RememberedSpot>,
    recovering: bool,
    recovery_error: Option<&'static str>,
    restart_pending: bool,
    retained_sample: Option<(Setup, [f32; 2])>,
}
thread_local! {
    static LIVE: RefCell<Option<Practice>> = const { RefCell::new(None) };
    static BESTS: RefCell<Bests> = const { RefCell::new(Bests::EMPTY) };
}

/// Core supplies its verified product resolver; never install a fixture/default.
/// Call before the first Tech session. Rebinding is rejected by OnceLock.
pub fn bind_hitlag(resolver: HitlagResolver) -> Result<(), HitlagResolver> {
    HITLAG.set(resolver)
}
pub fn publish(snapshot: &Snapshot) {
    CONFIG.publish(Some(tech_settings::read(snapshot)));
}
pub fn menu_opened() {
    // SAFETY: atomic owner-bit release; every placing exit also releases bit 2.
    unsafe { super::btt_camera::request(2, false) };
    GUARD_PRESS.store(false, Ordering::Release);
    PLACING.store(false, Ordering::Release);
    OPEN.store(true, Ordering::Release);
    notices::clear(NOTICE);
    notices::raise(notices::RAISE_TECH, false);
}
pub fn menu_closed(edit: bool) {
    OPEN.store(false, Ordering::Release);
    EDIT.store(edit, Ordering::Release);
    START.store(true, Ordering::Release);
}
pub fn reset() {
    RESET.store(true, Ordering::Release);
}
pub fn running() -> bool {
    RUNNING.load(Ordering::Acquire)
}
pub fn placing() -> bool {
    PLACING.load(Ordering::Acquire)
}
/// Native decision seam posts the exact attempt-tagged snapshot, before status
/// mutation. ContactReceipt alone deliberately cannot synthesize this snapshot.
fn observe_decision(snapshot: DecisionSnapshot) -> bool {
    DECISIONS.post(snapshot)
}
pub fn observe(outcome: Outcome) -> bool {
    OUTCOMES.post(outcome)
}
/// Additional proven native guard edge. The live native reader supplies availability;
/// neither raw shoulder input nor a trigger-count decrease proves a new press.
pub fn guard_press() {
    if !OPEN.load(Ordering::Acquire)
        && !scene::facts().busy
        && !published::input_owners().covers_strip()
    {
        GUARD_PRESS.store(true, Ordering::Release);
    }
}
fn kind(config: Config) -> Option<Kind> {
    match config.surface {
        1 => Some(Kind::Floor),
        2 => Some(Kind::Wall),
        3 => Some(Kind::Ceiling),
        _ => None,
    }
}
fn length(config: Config) -> Length {
    match config.session {
        1 => Length::Attempts(20),
        2 => Length::Attempts(10),
        3 => Length::Frames(3600),
        4 => Length::Frames(10800),
        _ => Length::Free,
    }
}
fn draw(rng: &mut Rng, n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let threshold = n.wrapping_neg() % n;
    loop {
        let v = rng.next_u32();
        if v >= threshold {
            return v % n;
        }
    }
}
/// Owned P1 poll, shared BTT logical controls. Only the editor consumes input.
pub unsafe fn poll(controller: usize, sample: Sample) -> bool {
    if !placing() || OPEN.load(Ordering::Acquire) {
        return false;
    }
    let Some(config) = CONFIG.read() else {
        return false;
    };
    let Some(mut view) = VIEW.try_lock() else {
        return true;
    };
    let Some(range) = view.range else {
        return true;
    };
    let stick = [0, 1].map(|i| ((controller + field::STICKS[i]) as *const f32).read_volatile());
    let ViewState {
        picker,
        surfaces,
        start,
        ..
    } = &mut *view;
    if picker.feed(
        Pad {
            stick,
            held: sample.held,
            pressed: sample.pressed,
            save: false,
            load: false,
        },
        surfaces,
        range,
        kind(config),
    ) == Pick::Start
    {
        *start = true;
    }
    true
}
/// BTT's existing egg/cursor pane pool draws the selected point and its normal.
pub fn look(active: bool) -> [Option<([f32; 2], EggLook)>; LOOKS] {
    let mut out = [None; LOOKS];
    if !active || !running() {
        return out;
    }
    let Some(view) = VIEW.try_lock() else {
        return out;
    };
    let spot = if placing() {
        view.picker.hovered
    } else {
        view.picker.selected
    };
    if let Some(spot) = spot {
        out[0] = Some((
            [spot.point.x, spot.point.y],
            EggLook {
                scale: 0.3,
                ..EGG_WHOLE
            },
        ));
        if placing() {
            let tip = crate::model::tech::add(
                spot.point,
                crate::model::tech::scale(spot.line.normal, 5.),
            );
            let side = Vec2 {
                x: -spot.line.normal.y,
                y: spot.line.normal.x,
            };
            let back =
                crate::model::tech::add(tip, crate::model::tech::scale(spot.line.normal, -1.5));
            let points = [
                crate::model::tech::add(
                    spot.point,
                    crate::model::tech::scale(spot.line.normal, 2.5),
                ),
                tip,
                crate::model::tech::add(back, side),
                crate::model::tech::add(back, crate::model::tech::scale(side, -1.)),
            ];
            for (slot, p) in out[1..5].iter_mut().zip(points) {
                *slot = Some((
                    [p.x, p.y],
                    EggLook {
                        scale: 0.12,
                        ..EGG_WHOLE
                    },
                ));
            }
        }
    }
    if placing() {
        out[LOOKS - 1] = Some((
            [view.picker.cursor.x, view.picker.cursor.y],
            EggLook {
                picture: EggPicture::Cursor,
                ..EGG_WHOLE
            },
        ));
    }
    out
}
/// Initialize from the CPU resolver's complete status array, in `tech_init::STATUS_IDS` order.
/// # Safety
/// Only the non-hook heartbeat may call this, after guarded game-table resolution.
pub unsafe fn init(ids: [i32; 18]) -> Option<()> {
    if ids.contains(&crate::model::cpu::ids::UNRESOLVED) {
        return None;
    }
    if !tech_surfaces::init() {
        return None;
    }
    let tumble: [i32; 7] = ids[..7].try_into().ok()?;
    let grading: [i32; 11] = ids[7..].try_into().ok()?;
    GRADING_IDS.set(grading).ok()?;
    super::tech_adapter::bind_decision_observer(observe_decision).ok()?;
    TUMBLE_IDS.set(tumble).ok()
}
unsafe fn still_tumbling(accessor: usize) -> Option<bool> {
    Some(TUMBLE_IDS.get()?.contains(&natives::status_kind(accessor)))
}
fn notice(run: &Practice, text: Option<&str>) {
    use core::fmt::Write;
    let mut message = FixedPath::<192>::new();
    let best = BESTS.with(|bests| {
        bests
            .borrow_mut()
            .record(run.fighter_kind, run.controller.session.summary.best)
    });
    if let Some(text) = text {
        let _ = message.write_str(text);
    } else if matches!(run.controller.session.phase, Phase::Off | Phase::Done) {
        let _ = run.controller.session.summary.write(&mut message);
        let _ = write!(message, " · fighter best {best}");
    } else if placing() {
        let _ = message.write_str("Choose a surface · A sets · X starts");
    } else {
        if let Some(grade) = run.controller.session.result {
            let _ = grade.write(&mut message);
        } else {
            let _ = message.write_str("Tech the selected surface");
        }
        if let Some(direction) = run.controller.direction {
            let _ = write!(message, " · {}", direction.label());
        }
        let _ = message.write_str(" · ");
        let _ = run.controller.session.write_progress(&mut message);
        if run.approximate {
            let _ = message.write_str(" · Auto approximate");
        }
    }
    notices::raise(notices::RAISE_TECH, true);
    notices::upsert(
        NOTICE,
        "Tech Practice",
        message.as_str(),
        u16::MAX,
        crate::model::notify::DEFAULT_COLOR,
    );
}
/// Discard borrowed identities once, then resolve the same geometry after the
/// bounded walk. A second invalid result is a specific failure, never a retry loop.
fn rebuild(run: &mut Practice, view: &mut ViewState) {
    if run.recovering {
        // restriction: tech-surface-unavailable-after-rebuild
        let message = "Selected surface unavailable after rebuild";
        run.recovery_error = Some(message);
        notice(run, Some(message));
        return;
    }
    let sampled = run.sampled.take();
    run.remembered = sampled.map(|(_, spot, _)| spot)
        .or_else(|| (run.config.surface == 4).then_some(view.picker.selected).flatten())
        .and_then(|spot| RememberedSpot::capture(spot.line, spot.point));
    run.retained_sample = sampled.map(|(setup, _, percent)| (setup, percent));
    run.recovering = true;
    run.scan = None;
    run.scan_done = false;
    run.bootstrap = None;
    view.surfaces.discard();
    view.picker.selected = None;
    view.picker.hovered = None;
    notice(run, Some("Finding stage surfaces…"));
}

/// P1 post-native pass. The caller supplies the live fighter; no pointer survives
/// this call. Ice Climbers' secondary body is excluded by the native partner id.
pub unsafe fn frame(fighter: usize) {
    if !super::tech_adapter::tech_ready() {
        return;
    }
    if !super::cpu::TECH_INIT.ready() || TUMBLE_IDS.get().is_none() {
        return;
    }
    let facts = scene::facts();
    if facts.busy {
        super::btt_camera::request(2, false);
        return;
    }
    let Some(mut view) = VIEW.try_lock() else {
        return;
    };
    if crate::model::scene::scene(facts).is_none() {
        {
            view.picker.enter(-1, Vec2::default());
            view.surfaces = Surfaces::EMPTY;
            view.range = None;
            view.start = false;
        }
        LIVE.with(|slot| {
            if let Some(mut run) = slot.borrow_mut().take() {
                run.controller.stop(&mut run.adapter);
            }
        });
        RUNNING.store(false, Ordering::Release);
        PLACING.store(false, Ordering::Release);
        notices::clear(NOTICE);
        notices::raise(notices::RAISE_TECH, false);
        drop(view);
        super::btt_camera::request(2, false);
        return;
    }
    let entry = ((fighter + crate::model::lra_reset::FIGHTER_ENTRY_ID_FIELD) as *const i32).read();
    if entry != 0 {
        return;
    }
    let at = natives::fighter_accessor(fighter);
    if at == 0 || natives::module_accessor(0) != Some(at) {
        return;
    }
    // The same owned-player receipt gate used by the adapter excludes nonowners.
    let Some(identity) = super::cpu::tech_player_receipt().ok().flatten() else {
        return;
    };
    let Some(config) = CONFIG.read() else {
        return;
    };
    let stop = config.session == 0;
    let open = OPEN.load(Ordering::Acquire);
    LIVE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if stop {
            view.start = false;
            if let Some(run) = slot.as_mut() {
                run.controller.stop(&mut run.adapter);
                if !open {
                    notice(run, None);
                }
            }
            *slot = None;
            RUNNING.store(false, Ordering::Release);
            PLACING.store(false, Ordering::Release);
            return;
        }
        if slot.is_none() {
            let Some(hitlag) = HITLAG.get().copied() else {
                return;
            };
            let Some(fighter_kind) = natives::fighter_kind(at) else {
                return;
            };
            *slot = Some(Practice {
                adapter: Adapter::new(hitlag),
                controller: Controller::IDLE,
                config,
                pending: false,
                reroll: true,
                sampled: None,
                approximate: false,
                scan: None,
                scan_done: false,
                bootstrap: None,
                discovery_failed: false,
                epoch: None,
                fighter_kind,
                rng: Rng::new(Rng::seed_from(
                    skyline::nn::os::GetSystemTick() as u64,
                    at as u64,
                )),
                after_reaction: 0,
                planning_failed: false,
                decision: None,
                remembered: None,
                recovering: false,
                recovery_error: None,
                restart_pending: false,
                retained_sample: None,
            });
        }
        let run = slot.as_mut().unwrap();
        RUNNING.store(true, Ordering::Release);
        if open {
            if !matches!(run.controller.session.phase, Phase::Off | Phase::Done) {
                run.controller.stop(&mut run.adapter);
            }
            return;
        }
        if facts.busy || published::input_owners().covers_strip() {
            GUARD_PRESS.store(false, Ordering::Release);
            return;
        }
        let stage = natives::stage_id();
        let Some(form) = natives::stage_form(stage) else {
            return;
        };
        let epoch = StageEpoch {
            stage,
            form,
            training: identity,
        };
        let reset = RESET.swap(false, Ordering::AcqRel);
        if START.swap(false, Ordering::AcqRel) || reset {
            run.restart_pending = true;
            run.recovery_error = None;
            run.recovering = false;
        }
        if run.epoch != Some(epoch) || reset {
            // A native Training reset can rebuild collision without changing
            // stage/form/scene identity. Only world geometry survives it.
            run.remembered = (config.surface == 4
                && view.surfaces.epoch().is_some_and(|old| old.stage == stage && old.form == form))
                .then(|| view.picker.selected.and_then(|s| RememberedSpot::capture(s.line, s.point)))
                .flatten();
            run.recovering = false;
            run.recovery_error = None;
            run.controller.stop(&mut run.adapter);
            run.epoch = Some(epoch);
            run.sampled = None;
            run.retained_sample = None;
            run.decision = None;
            run.pending = false;
            run.planning_failed = false;
            PLACING.store(false, Ordering::Release);
            run.scan = None;
            run.scan_done = false;
            run.bootstrap = None;
            run.discovery_failed = false;
            view.picker.enter(
                stage,
                Vec2 {
                    x: natives::pos_x(at),
                    y: natives::pos_y(at),
                },
            );
            view.start = false;
            view.surfaces = Surfaces::EMPTY;
            Surfaces::bind(&mut view.surfaces, epoch);
        }
        if let Some(message) = run.recovery_error {
            notice(run, Some(message));
            return;
        }
        if run.discovery_failed {
            notice(run, Some("Surface discovery incomplete"));
            return;
        }
        let Some(range) = natives::camera_limit_range() else {
            run.discovery_failed = true;
            notice(run, Some("Surface discovery incomplete"));
            return;
        };
        if run.scan.is_none() {
            run.scan = PlatformScan::new(range);
        }
        let Some(mut source) = NativeSource::new(at) else {
            run.discovery_failed = true;
            notice(run, Some("Surface discovery incomplete"));
            return;
        };
        if run.bootstrap.is_none() {
            run.bootstrap = Bootstrap::new(range, source.touched());
        }
        {
            view.range = Some(range);
            if (view.surfaces.state != Walk::Closed || !run.scan_done)
                && !run.bootstrap.as_mut().is_some_and(Bootstrap::tick)
            {
                run.discovery_failed = true;
                notice(run, Some("Surface discovery incomplete"));
                return;
            }
            if view.surfaces.state == Walk::Broken && view.surfaces.lines().next().is_none() {
                match run
                    .bootstrap
                    .as_mut()
                    .ok_or(())
                    .and_then(|search| source.find_seed(search))
                {
                    Ok(Some((seed, end))) => view.surfaces.begin(seed, end),
                    Ok(None) => {}
                    Err(()) => {
                        run.discovery_failed = true;
                        notice(run, Some("Surface discovery incomplete"));
                        return;
                    }
                }
            }
            if view.surfaces.state == Walk::More {
                view.surfaces.step(&mut source);
            } else if view.surfaces.state == Walk::Closed && !run.scan_done {
                if let Some(scan) = run.scan.as_mut() {
                    match scan.step(&mut source, &mut view.surfaces) {
                        ScanState::More => {}
                        ScanState::Done => run.scan_done = true,
                        ScanState::Broken | ScanState::Full => {
                            run.discovery_failed = true;
                            notice(run, Some("Surface discovery incomplete"));
                            return;
                        }
                    }
                }
            }
            if matches!(view.surfaces.state, Walk::Full)
                || (view.surfaces.state == Walk::Broken && view.surfaces.lines().next().is_some())
            {
                run.discovery_failed = true;
                notice(run, Some("Surface discovery incomplete"));
                return;
            }
            if view.surfaces.state != Walk::Closed || !run.scan_done {
                notice(run, Some("Finding stage surfaces…"));
                return;
            }
        }
        {
            let ViewState {
                picker, surfaces, ..
            } = &mut *view;
            Picker::restore(picker, surfaces);
        }
        if let Some(remembered) = run.remembered.take() {
            match remembered.resolve(&view.surfaces) {
                Ok((line, point)) => {
                    let spot = picker::Spot { line, point };
                    view.picker.selected = Some(spot);
                    if let Some((setup, percent)) = run.retained_sample.take() {
                        run.sampled = Some((setup, spot, percent));
                    }
                }
                Err(error) => {
                    view.picker.selected = None;
                    run.recovery_error = Some(error.message());
                    notice(run, Some(error.message()));
                    return;
                }
            }
        }
        if run.restart_pending || run.controller.session.phase == Phase::Off {
            run.restart_pending = false;
            if run.controller.session.phase != Phase::Off {
                run.controller.stop(&mut run.adapter);
            }
            view.start = false;
            let Some(fighter_kind) = natives::fighter_kind(at) else {
                return;
            };
            run.fighter_kind = fighter_kind;
            run.config = config;
            run.controller.start(length(config));
            run.decision = None;
            run.pending = true;
            run.reroll = true;
            run.sampled = None;
            run.planning_failed = false;
            PLACING.store(
                EDIT.swap(false, Ordering::AcqRel)
                    || config.surface == 4 && view.picker.selected.is_none(),
                Ordering::Release,
            );
        }
        if placing() {
            if !view.start {
                notice(run, None);
                return;
            }
            view.start = false;
            PLACING.store(false, Ordering::Release);
        }
        let Ok(attempt) = run.adapter.attempt() else {
            return;
        };
        // Copy retained decision values, including unconfirmed native entry.
        // Busy defers before recorder consumption or contact-clock advancement.
        let Ok(raw) = run.adapter.pending_decision() else {
            return;
        };
        if run
            .decision
            .is_some_and(|(s, _)| !run.controller.accepts_snapshot(s.attempt))
        {
            run.decision = None;
        }
        DECISIONS.drain(|snapshot| {
            if snapshot.attempt == attempt.id {
                run.controller.retain_snapshot(&mut run.decision, snapshot);
            }
        });
        if let Some(snapshot) = raw.filter(|s| s.attempt == attempt.id) {
            run.controller.retain_snapshot(&mut run.decision, snapshot);
        }
        if let Some((snapshot, age)) = run.decision {
            let event = GRADING_IDS.get().map_or(Event::None, |ids| {
                run.controller.retained_snapshot(snapshot, *ids, age)
            });
            if event == Event::Graded {
                run.decision = None;
            }
        }
        OUTCOMES.drain(|outcome| {
            run.controller.outcome(outcome);
        });
        let posted_press = GUARD_PRESS.swap(false, Ordering::AcqRel);
        let guard_press = super::tech_decision::guard_event(at).map(|native| native || posted_press);
        if let Ok(event) = Controller::tick(
            &mut run.controller,
            &mut run.adapter,
            false,
            guard_press,
        ) {
            if let Some((snapshot, age)) = run.decision {
                run.decision = Some((snapshot, age.saturating_add(1)));
            }
            if event == Event::Summary {
                run.pending = false;
                run.planning_failed = false;
            }
            if let Event::NeedSetup { reroll } = event {
                run.pending = true;
                run.reroll = reroll;
                if reroll {
                    run.sampled = None;
                }
            }
        }
        if run.controller.session.phase == Phase::Flight {
            let reaction =
                super::tech_native::id("FIGHTER_INSTANCE_WORK_ID_FLOAT_DAMAGE_REACTION_FRAME")
                    .map(|id| natives::get_float(at, id));
            if reaction.is_some_and(|r| r.is_finite() && r <= 0.) {
                run.after_reaction = run.after_reaction.saturating_add(1);
            } else {
                run.after_reaction = 0;
            }
            let left_tumble = run.adapter.attempt().ok().is_some_and(|attempt| {
                attempt.state == crate::model::tech_launch::AttemptState::Flying
            }) && still_tumbling(at) == Some(false);
            if (run.after_reaction > 120 || left_tumble) && !run.controller.awaiting_outcome() {
                match Controller::abandon(&mut run.controller) {
                    Event::NeedSetup { reroll } => {
                        run.pending = true;
                        run.reroll = reroll;
                    }
                    Event::Summary => run.adapter.restore(),
                    _ => {}
                }
            }
        }
        if run.planning_failed {
            notice(
                run,
                Some("Approximate aim has no solution for these values"),
            );
            return;
        }
        if run.pending {
            let Ok(multiplier) = super::tech_hitlag::live_multiplier(at) else {
                // restriction: tech-hit-damage-multiplier-unavailable
                notice(run, Some("Hit damage unavailable"));
                return;
            };
            if run.reroll && run.sampled.is_none() {
                let Some(blast) = tech_surfaces::blast_bounds() else {
                    return;
                };
                let gravity = natives::get_param_float(at, hash40("air_accel_y"), 0);
                let cap = natives::get_param_float(at, hash40("air_speed_y_stable"), 0);
                if !gravity.is_finite() || gravity <= 0. || !cap.is_finite() || cap <= 0. {
                    return;
                }
                let spot = if run.config.surface == 4 {
                    if view.picker.selected.is_some_and(|s| !s.current(&mut source)) {
                        rebuild(run, &mut view);
                        return;
                    }
                    view.picker.selected
                } else {
                    picker::random_spot(
                        &view.surfaces,
                        &mut source,
                        kind(run.config),
                        range,
                        &mut |n| draw(&mut run.rng, n),
                    )
                };
                let Some(spot) = spot else {
                    if run.config.surface == 4 {
                        notice(run, Some("Choose a stage surface"));
                    } else {
                        rebuild(run, &mut view);
                    }
                    return;
                };
                view.picker.selected = Some(spot);
                let result = planner::plan(
                    run.config,
                    planner::Context {
                        surfaces: &view.surfaces,
                        spot,
                        params: FlightParams {
                            brake: 0.051,
                            gravity,
                            fall_speed_cap: cap,
                        },
                        blast: solver::editor_bounds(blast),
                        view: (run.config.camera == 1).then(|| solver::editor_bounds(range)),
                        damage_multiplier: Some(multiplier),
                    },
                    &mut |n| draw(&mut run.rng, n),
                );
                let Some(plan) = result else {
                    run.planning_failed = true;
                    notice(
                        run,
                        Some("Approximate aim has no solution for these values"),
                    );
                    return;
                };
                run.sampled = Some((plan.setup, spot, plan.sampled_percent));
                run.approximate = plan.approximate;
            }
            if let Some((mut setup, spot, [percent, visible_hit_damage])) = run.sampled {
                let Some(base) = planner::base_hit_damage(visible_hit_damage, Some(multiplier))
                else {
                    // restriction: tech-hit-damage-multiplier-unavailable
                    notice(run, Some("Hit damage unavailable"));
                    return;
                };
                setup.launch.hit_damage = base;
                setup.launch.percent = percent + base * multiplier;
                if !spot.current(&mut source) {
                    rebuild(run, &mut view);
                    return;
                }
                if matches!(
                    run.controller.place(&mut run.adapter, setup),
                    Ok(Event::Placed)
                ) {
                    run.pending = false;
                    run.decision = None;
                    run.after_reaction = 0;
                    run.recovering = false;
                }
            }
        }
        notice(run, None);
    });
    drop(view);
    super::btt_camera::request(
        2,
        running()
            && !open
            && !published::input_owners().covers_strip()
            && (placing() || config.camera == 1),
    );
}
