//! Stage Tech practicer. UI/input/layout exchange only plain snapshots; the
//! launch adapter stays on the P1 game thread. No default hitlag resolver exists.
use super::{
    notices,
    tech_adapter::{Adapter, HitlagResolver},
    tech_surfaces::{self, Discovery, Found, NativeSource},
};
use crate::model::btt::{self as model, Pad, LOOKS};
use crate::model::chords::{field, Sample};
use crate::model::cpu::hash::hash40;
use crate::model::cpu::rng::Rng;
use crate::model::floor_hop::Floor;
use crate::model::panes::EggLook;
use crate::model::settings::Snapshot;
use crate::model::storage::FixedPath;
use crate::model::tech::{
    controller::{Controller, Event, Outcome, Setup},
    picker::{self, Pick, Picker},
    planner,
    practice_state::{self as state, View as StateView},
    session::{Bests, Length, Phase, Session},
    solver,
    surface::{Kind as SurfaceKind, MaterialDraws, MaterialHint, RememberedSpot, StageEpoch, Surfaces, MAX_LINES},
    wizard::{Action as WizardAction, Outcome as WizardOutcome, Step, Wizard},
    Vec2,
};
use crate::model::tech_launch::{DecisionSnapshot, FlightParams, Launcher};
use crate::model::practice_games::{self, Game};
use crate::model::tech_settings::{self, Config};
use crate::model::wizard::Kind;
use crate::runtime::game::natives;
use crate::runtime::{published, scene};
use hos_lock::{Mailbox, Mutex, Published};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::OnceLock;

const NOTICE: u8 = 0x32;
static CONFIG: Published<Option<Config>> = Published::new(None);
static OPEN: AtomicBool = AtomicBool::new(false);
static START: AtomicBool = AtomicBool::new(false);
static EDIT: AtomicBool = AtomicBool::new(false);
/// Training resets not answered yet, as two bits of one byte: `RESET_RUN` for the running Tech session, and
/// `RESET_OUTLINE` since the outline in `VIEW` was traced (the game may have rebuilt its collision).
static RESET: AtomicU8 = AtomicU8::new(0);
const RESET_RUN: u8 = 1;
const RESET_OUTLINE: u8 = 2;
static RUNNING: AtomicBool = AtomicBool::new(false);
static GUARD_PRESS: AtomicBool = AtomicBool::new(false);
static WIZARD_REQUEST: AtomicBool = AtomicBool::new(false);
static WIZARD_SKIP: AtomicBool = AtomicBool::new(false);
static BACK_MENU: AtomicBool = AtomicBool::new(false);
static HITLAG: OnceLock<HitlagResolver> = OnceLock::new();
static TUMBLE_IDS: OnceLock<[i32; 7]> = OnceLock::new();
static DECISIONS: Mailbox<DecisionSnapshot, 4> = Mailbox::new();
static GRADING_IDS: OnceLock<[i32; 11]> = OnceLock::new();
static OUTCOMES: Mailbox<Outcome, 4> = Mailbox::new();
struct ViewState {
    picker: Picker,
    surfaces: Surfaces,
    /// The walk that fills `surfaces`: with it the lines say whether they are all there.
    discovery: Discovery,
    range: Option<[f32; 4]>,
    start: bool,
    wizard: Option<Wizard>,
    wizard_config: Option<Config>,
    cancelled: bool,
    preview_params: Option<FlightParams>,
    preview_blocked: Option<bool>,
}
static VIEW: Mutex<ViewState> = Mutex::new(ViewState {
    picker: Picker::EMPTY,
    surfaces: Surfaces::EMPTY,
    discovery: Discovery::NEW,
    range: None,
    start: false,
    wizard: None,
    wizard_config: None,
    cancelled: false,
    preview_params: None,
    preview_blocked: None,
});
enum StartHint {
    Off,
    Pending(MaterialDraws),
    Visible(MaterialHint, u8),
}
struct Practice {
    adapter: Adapter,
    controller: Controller,
    config: Config,
    pending: bool,
    reroll: bool,
    sampled: Option<(Setup, picker::Spot, [f32; 2])>,
    approximate: bool,
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
    start_hint: StartHint,
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
    WIZARD_SKIP.store(
        tech_settings::selected(snapshot, "tech_wizard_skip") == 0,
        Ordering::Release,
    );
}
pub fn open_wizard() {
    WIZARD_REQUEST.store(true, Ordering::Release);
    published::WIZARDS.set_up(Kind::TechWizard, true);
    // Publish the complete request even if the game consumed menu_closed's
    // earlier restart between the worker's two calls.
    START.store(true, Ordering::Release);
}
pub fn wizard_active() -> bool {
    published::WIZARDS.is_up(Kind::TechWizard)
}
pub fn take_menu_request() -> bool {
    BACK_MENU.swap(false, Ordering::AcqRel)
}
fn release_camera_requests() {
    super::btt_camera::request(model::TECH_PLACEMENT_OWNER | model::TECH_CAMERA_OWNER, false);
}

pub fn menu_opened() {
    release_camera_requests();
    GUARD_PRESS.store(false, Ordering::Release);
    published::WIZARDS.set_up(Kind::TechPlacing, false);
    published::WIZARDS.set_up(Kind::TechWizard, false);
    OPEN.store(true, Ordering::Release);
    notices::clear(NOTICE);
}
/// The menu closed: a close alone starts nothing (rc.15 practice menu, Q1).
pub fn menu_closed() {
    OPEN.store(false, Ordering::Release);
}
/// Start (the menu worker, `practice::start`): `edit`, it was Tech point (the surface placing view).
pub fn start(edit: bool) {
    EDIT.store(edit, Ordering::Release);
    START.store(true, Ordering::Release);
}
/// A Start is on its way to the next frame.
pub fn starting() -> bool {
    START.load(Ordering::Acquire)
}
pub fn reset() {
    RESET.store(RESET_RUN | RESET_OUTLINE, Ordering::Release);
}
pub fn running() -> bool {
    RUNNING.load(Ordering::Acquire)
}
pub fn placing() -> bool {
    published::WIZARDS.is_up(Kind::TechPlacing)
}
/// Native decision seam posts the exact attempt-tagged snapshot, before status
/// mutation. ContactReceipt alone deliberately cannot synthesize this snapshot.
fn observe_decision(snapshot: DecisionSnapshot) -> bool {
    DECISIONS.post(snapshot)
}
fn kind(config: Config) -> Option<SurfaceKind> {
    match config.surface {
        1 => Some(SurfaceKind::Floor),
        2 => Some(SurfaceKind::Wall),
        3 => Some(SurfaceKind::Ceiling),
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
unsafe fn plan_context(
    view: &ViewState,
    spot: picker::Spot,
    config: Config,
    at: usize,
    multiplier: f32,
) -> Option<planner::Context<'_>> {
    let gravity = natives::get_param_float(at, hash40("air_accel_y"), 0);
    let cap = natives::get_param_float(at, hash40("air_speed_y_stable"), 0);
    if !gravity.is_finite() || gravity <= 0. || !cap.is_finite() || cap <= 0. {
        return None;
    }
    Some(planner::Context {
        surfaces: &view.surfaces,
        spot,
        params: FlightParams {
            brake: 0.051,
            gravity,
            fall_speed_cap: cap,
        },
        blast: solver::editor_bounds(tech_surfaces::blast_bounds()?),
        view: if config.camera == 1 {
            Some(solver::editor_bounds(view.range?))
        } else {
            None
        },
        damage_multiplier: Some(multiplier),
    })
}
fn wizard_poll(view: &mut ViewState, sample: Sample, stick: [f32; 2], range: [f32; 4]) {
    use crate::model::chords::logical::{A, B, PLUS, X};
    let Some(wizard) = view.wizard.as_mut() else {
        return;
    };
    let pad = Pad {
        stick,
        held: sample.held,
        pressed: sample.pressed,
        save: false,
        load: false,
    };
    if sample.pressed != 0 || stick != [0., 0.] {
        view.preview_blocked = None;
    }
    if wizard.step == Step::Spot {
        // X belongs to Defaults here, rather than the full editor's Start.
        let mut picker_pad = pad;
        picker_pad.pressed &= !(1 << X);
        view.picker.feed(picker_pad, &view.surfaces, range, None);
        if let Some(spot) = view.picker.hovered {
            wizard.set_spot(spot);
        }
    } else {
        let motion = view.picker.placing_motion(&pad);
        wizard.adjust(motion[0], motion[1]);
    }
    // One action per edge, with Back taking priority over confirmation.
    let action = [
        (B, WizardAction::Back),
        (X, WizardAction::Defaults),
        (PLUS, WizardAction::Start),
        (A, WizardAction::Next),
    ]
    .into_iter()
    .find_map(|(key, action)| (sample.pressed & (1 << key) != 0).then_some(action));
    match action.map(|a| wizard.action(a)) {
        Some(WizardOutcome::Start) => {
            view.wizard_config = wizard.config();
            view.picker.selected = wizard.selected;
            view.start = true;
            published::WIZARDS.set_up(Kind::TechWizard, false);
        }
        Some(WizardOutcome::Cancelled) => {
            view.cancelled = true;
            published::WIZARDS.set_up(Kind::TechWizard, false);
            BACK_MENU.store(true, Ordering::Release);
        }
        _ => {}
    }
}

/// The wizard uses the shared notice writer and compact HUD. Measurement,
/// wrapping and placement belong to the common renderer and existing panes.
fn wizard_notice(
    run: &Practice,
    wizard: &Wizard,
    blocked: Option<bool>,
    hint: Option<MaterialHint>,
) {
    use core::fmt::Write;
    let mut text = FixedPath::<256>::new();
    let _ = writeln!(text, "{}", wizard.step.title());
    let _ = match wizard.step {
        Step::Spot => text.write_str("Move the cursor onto a surface"),
        Step::Angle => write!(
            text,
            "Surface angle {:.0}° · world {:.1}°\n← → adjust",
            wizard.degrees,
            wizard.world_angle().unwrap_or(0.)
        ),
        Step::Distance => write!(
            text,
            "Distance {:.0} units from surface\n← → adjust",
            wizard.distance
        ),
        Step::Hit => write!(
            text,
            "{} Speed {:.0} KB\n{} Hit damage {:.0}% added\n↑ ↓ choose · ← → adjust",
            if wizard.hit_focus { " " } else { ">" },
            wizard.speed,
            if wizard.hit_focus { ">" } else { " " },
            wizard.hit
        ),
        Step::Preview => write!(
            text,
            "{:.0}° · {:.0} units · {:.0} KB · +{:.0}%\n{}",
            wizard.degrees,
            wizard.distance,
            wizard.speed,
            wizard.hit,
            match blocked {
                // restriction: tech-wizard-preview-blocked
                Some(true) => "Start blocked / outside bounds — adjust values",
                Some(false) => "Dotted path is approximate",
                // restriction: tech-wizard-preview-unavailable
                None => "Preview unavailable",
            }
        ),
    };
    if let Some(hint) = hint {
        let _ = write!(text, "\n{}", hint.text());
    }
    let _ = text.write_str(if wizard.step == Step::Preview {
        "\nA / + start · B back"
    } else {
        "\nA next · B back\nX defaults → preview · + start"
    });
    notice(run, Some(text.as_str()));
}
/// Owned P1 poll, shared BTT logical controls. Only the editor consumes input.
pub unsafe fn poll(controller: usize, sample: Sample) -> bool {
    if WIZARD_REQUEST.load(Ordering::Acquire) && !OPEN.load(Ordering::Acquire) {
        return true;
    }
    if !placing() || OPEN.load(Ordering::Acquire) {
        return wizard_active() && !OPEN.load(Ordering::Acquire);
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
    if wizard_active() {
        wizard_poll(&mut view, sample, stick, range);
        return true;
    }
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
    if !active || !running() {
        return [None; LOOKS];
    }
    let Some(view) = VIEW.try_lock() else {
        return [None; LOOKS];
    };
    let placing = placing();
    let wizard = view.wizard.as_ref().filter(|_| wizard_active());
    let spot = match wizard {
        Some(wizard) => wizard.selected,
        None if placing => view.picker.hovered,
        None => view.picker.selected,
    };
    let cursor =
        (placing && wizard.is_none_or(|w| w.step == Step::Spot)).then_some(view.picker.cursor);
    crate::model::tech::wizard_marks::marks(
        spot,
        cursor,
        wizard,
        view.preview_params,
        view.preview_blocked == Some(false),
    )
}
/// The stage's traced floors for the placement picker's step between floors, from the one outline Tech practice
/// traces: no second copy is kept. The picker is open only while no practice game runs, so Tech is not walking
/// the outline then. When it is not done for this stage, form and Training session, or a Training reset has passed
/// since it was traced, the picker walks it here, one step a frame; Tech traces again at its next Start, as it
/// always does. `found`: the picker's own note of how its walk went. Returns the floors written, 0 until the walk
/// is done; `None` when the outline or the receipt is in use this frame, which is never waited for. The walk's
/// native calls are Tech practice's: where its guards did not pass, no floor is given.
pub(super) unsafe fn floors(
    p1: usize,
    (stage, form): (i32, u32),
    range: [f32; 4],
    found: &mut Found,
    out: &mut [Floor; MAX_LINES],
) -> Option<usize> {
    if !super::tech_adapter::tech_ready() {
        return Some(0);
    }
    let mut view = VIEW.try_lock()?;
    let view = &mut *view;
    let Some(training) = super::cpu::tech_player_receipt().ok()? else {
        return Some(0);
    };
    let epoch = StageEpoch { stage, form, training };
    let stale = RESET.fetch_and(!RESET_OUTLINE, Ordering::AcqRel) & RESET_OUTLINE != 0;
    if stale || view.surfaces.epoch() != Some(epoch) {
        view.surfaces = Surfaces::EMPTY;
        Surfaces::bind(&mut view.surfaces, epoch);
        view.discovery = Discovery::NEW;
        *found = Found::More;
    }
    *found = if view.discovery.walked(&view.surfaces) {
        Found::Done
    } else if *found == Found::Failed {
        Found::Failed
    } else {
        match NativeSource::new(p1) {
            Some(mut source) => view.discovery.walk_step(&mut source, range, &mut view.surfaces),
            None => Found::Failed,
        }
    };
    if *found != Found::Done {
        return Some(0);
    }
    let mut count = 0;
    for line in view.surfaces.lines().filter(|line| line.kind() == Some(SurfaceKind::Floor)) {
        out[count] = line.ends.map(|end| [end.x, end.y]);
        count += 1;
    }
    Some(count)
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
    super::tech_decision::init_passive_speed();
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
        if let StartHint::Visible(hint, _) = run.start_hint {
            let _ = write!(message, "\n{}", hint.text());
        }
    }
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
    run.remembered = sampled
        .map(|(_, spot, _)| spot)
        .or_else(|| {
            (run.config.surface == 4)
                .then_some(view.picker.selected)
                .flatten()
        })
        .and_then(|spot| RememberedSpot::capture(spot.line, spot.point));
    run.retained_sample = sampled.map(|(setup, _, percent)| (setup, percent));
    if let StartHint::Pending(draws) = &mut run.start_hint {
        *draws = MaterialDraws::default();
    }
    run.recovering = true;
    view.discovery = Discovery::NEW;
    view.surfaces.discard();
    view.picker.selected = None;
    view.picker.hovered = None;
    if let Some(wizard) = &mut view.wizard {
        wizard.selected = None;
    }
    notice(run, Some("Finding stage surfaces…"));
}

/// Update and sample on the same game thread with the caller's VIEW borrow.
fn with_practice_frame(
    view: &mut ViewState,
    open: bool,
    update: impl FnOnce(&mut Option<Practice>, &mut ViewState),
) {
    let capture = crate::runtime::bridge_state::PracticeCapture::of(Game::Tech);
    LIVE.with(|cell| {
        let mut slot = cell.borrow_mut();
        update(&mut slot, view);
        if !START.load(Ordering::Acquire) {
            if let Some(run) = slot.as_ref() {
                capture.submit(|| {
                    let phase = if view.cancelled {
                        StateView::Cancelled
                    } else if open {
                        StateView::Paused
                    } else if run.recovery_error.is_some() || run.discovery_failed || run.planning_failed {
                        StateView::Failed
                    } else if !view.discovery.walked(&view.surfaces) {
                        StateView::Discovering
                    } else if let Some(wizard) = view.wizard.as_ref().filter(|_| wizard_active()) {
                        StateView::Wizard(wizard, view.preview_blocked)
                    } else if placing() {
                        StateView::Placing(&view.picker)
                    } else {
                        StateView::Session
                    };
                    let session = if run.restart_pending { &Session::IDLE } else { &run.controller.session };
                    state::project_tech_state(session, phase)
                });
            }
        }
    });
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
        release_camera_requests();
        return;
    }
    let Some(mut view) = VIEW.try_lock() else {
        return;
    };
    if crate::model::scene::scene(facts).is_none() {
        {
            view.picker.enter(-1, Vec2::default());
            view.surfaces = Surfaces::EMPTY;
            view.discovery = Discovery::NEW;
            view.range = None;
            view.start = false;
            view.wizard = None;
            view.wizard_config = None;
            view.preview_params = None;
            view.preview_blocked = None;
        }
        LIVE.with(|slot| {
            if let Some(mut run) = slot.borrow_mut().take() {
                run.controller.stop(&mut run.adapter);
            }
        });
        RUNNING.store(false, Ordering::Release);
        published::WIZARDS.set_up(Kind::TechPlacing, false);
        published::WIZARDS.set_up(Kind::TechWizard, false);
        notices::clear(NOTICE);
        drop(view);
        release_camera_requests();
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
    let Some(mut config) = CONFIG.read() else {
        return;
    };
    // rc.15 practice menu: the session while Tech practice is the running game, Off otherwise.
    let stored = u8::try_from(config.session).unwrap_or(u8::MAX);
    config.session = usize::from(practice_games::effective(Game::Tech, stored, super::practice::active()));
    let stop = config.session == 0;
    let open = OPEN.load(Ordering::Acquire);
    // The running session's camera (latched at its Start, like every other setting), the published one before a run.
    let mut camera = config.camera;
    with_practice_frame(&mut view, open, |slot, view| {
        if stop {
            RESET.fetch_and(!RESET_RUN, Ordering::Release);
            view.start = false;
            if let Some(run) = slot.as_mut() {
                run.controller.stop(&mut run.adapter);
                if !open {
                    notice(run, None);
                }
            }
            *slot = None;
            RUNNING.store(false, Ordering::Release);
            published::WIZARDS.set_up(Kind::TechPlacing, false);
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
                start_hint: StartHint::Off,
            });
        }
        let run = slot.as_mut().unwrap();
        camera = run.config.camera;
        RUNNING.store(true, Ordering::Release);
        // rc.15 practice menu (Q1, tr-review-astra-3 16:37Z): a menu visit suspends the session, it never stops it. The
        // attempt in flight is cut and the fighter released; after B the session goes on with its count, streak and
        // clock. Only Start restarts it and only Stop ends it.
        if open {
            if let Event::NeedSetup { reroll } = run.controller.pause_for_menu(&mut run.adapter) {
                run.pending = true;
                run.reroll = reroll;
                run.decision = None;
                if reroll {
                    run.sampled = None;
                }
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
        let reset = RESET.fetch_and(!RESET_RUN, Ordering::AcqRel) & RESET_RUN != 0;
        let requested = START.swap(false, Ordering::AcqRel);
        if requested {
            view.cancelled = false;
            view.wizard_config = None;
            view.wizard = if WIZARD_REQUEST.swap(false, Ordering::AcqRel) {
                Some(Wizard::begin(config, WIZARD_SKIP.load(Ordering::Acquire)))
            } else {
                None
            };
        }
        if requested || reset {
            run.restart_pending = true;
            run.recovery_error = None;
            run.recovering = false;
        }
        if run.epoch != Some(epoch) || reset {
            let interrupted = view.wizard.as_mut().is_some_and(|wizard| {
                wizard.wizard_rebind_geometry(requested, run.epoch.is_some(), reset)
            });
            if wizard_active() && interrupted {
                // A draft never retains a borrowed line across collision or
                // scene lifetimes. Return to the full menu on interruption.
                view.wizard = None;
                view.wizard_config = None;
                view.cancelled = true;
                published::WIZARDS.set_up(Kind::TechWizard, false);
                BACK_MENU.store(true, Ordering::Release);
            }
            // A native Training reset can rebuild collision without changing
            // stage/form/scene identity. Only world geometry survives it.
            run.remembered = (run.config.surface == 4
                && view
                    .surfaces
                    .epoch()
                    .is_some_and(|old| old.stage == stage && old.form == form))
            .then(|| {
                view.picker
                    .selected
                    .and_then(|s| RememberedSpot::capture(s.line, s.point))
            })
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
            published::WIZARDS.set_up(Kind::TechPlacing, false);
            view.discovery = Discovery::NEW;
            RESET.fetch_and(!RESET_OUTLINE, Ordering::Release);
            run.discovery_failed = false;
            view.preview_params = None;
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
        if view.cancelled {
            run.pending = false;
            if run.controller.session.phase != Phase::Off {
                run.controller.stop(&mut run.adapter);
            }
            published::WIZARDS.set_up(Kind::TechPlacing, false);
            return;
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
        let Some(mut source) = NativeSource::new(at) else {
            run.discovery_failed = true;
            notice(run, Some("Surface discovery incomplete"));
            return;
        };
        view.range = Some(range);
        match view.discovery.walk_step(&mut source, range, &mut view.surfaces) {
            Found::Done => {}
            Found::More => {
                notice(run, Some("Finding stage surfaces…"));
                return;
            }
            Found::Failed => {
                run.discovery_failed = true;
                notice(run, Some("Surface discovery incomplete"));
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
            if run.controller.session.phase != Phase::Off {
                run.controller.stop(&mut run.adapter);
            }
            view.start = false;
            let Some(fighter_kind) = natives::fighter_kind(at) else {
                return;
            };
            run.fighter_kind = fighter_kind;
            run.config = view.wizard_config.unwrap_or(config);
            camera = run.config.camera;
            run.restart_pending = false;
            run.controller.start(length(run.config));
            run.start_hint = StartHint::Pending(MaterialDraws::default());
            run.decision = None;
            run.pending = true;
            run.reroll = true;
            run.sampled = None;
            run.planning_failed = false;
            let placing = EDIT.swap(false, Ordering::AcqRel)
                || wizard_active()
                || config.surface == 4 && view.picker.selected.is_none();
            published::WIZARDS.set_up(Kind::TechPlacing, placing);
        }
        if placing() {
            if !view.start {
                if wizard_active() {
                    if let Some(wizard) = &view.wizard {
                        let preview = (|| {
                            if wizard.step != Step::Preview {
                                return None;
                            }
                            let (draft, spot) = (wizard.config()?, wizard.selected?);
                            if !spot.current(&mut source) {
                                return None;
                            }
                            let multiplier = super::tech_hitlag::live_multiplier(at).ok()?;
                            let context = plan_context(view, spot, draft, at, multiplier)?;
                            let params = context.params;
                            let blocked = planner::plan(draft, context, &mut |_| 0).is_none();
                            let mut draws = MaterialDraws::default();
                            if !blocked {
                                draws.observe(source.damaging_floor(spot.line));
                            }
                            Some((blocked, params, draws.hint(draft)))
                        })();
                        wizard_notice(
                            run,
                            wizard,
                            preview.map(|p| p.0),
                            preview.and_then(|p| p.2),
                        );
                        view.preview_blocked = preview.map(|p| p.0);
                        view.preview_params = preview.map(|p| p.1);
                    }
                } else {
                    notice(run, None);
                }
                return;
            }
            view.start = false;
            if let Some(draft) = view.wizard_config {
                run.config = draft;
            }
            published::WIZARDS.set_up(Kind::TechPlacing, false);
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
        let guard_press =
            super::tech_decision::guard_event(at).map(|native| native || posted_press);
        if let Ok(event) =
            Controller::tick(&mut run.controller, &mut run.adapter, false, guard_press)
        {
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
                let spot = if run.config.surface == 4 {
                    if view
                        .picker
                        .selected
                        .is_some_and(|s| !s.current(&mut source))
                    {
                        rebuild(run, view);
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
                        &mut |source, line| {
                            if let StartHint::Pending(draws) = &mut run.start_hint {
                                draws.observe(source.damaging_floor(line));
                            }
                        },
                    )
                };
                let Some(spot) = spot else {
                    if run.config.surface == 4 {
                        notice(run, Some("Choose a stage surface"));
                    } else {
                        rebuild(run, view);
                    }
                    return;
                };
                view.picker.selected = Some(spot);
                let Some(context) = plan_context(view, spot, run.config, at, multiplier) else {
                    return;
                };
                let result = planner::plan(run.config, context, &mut |n| draw(&mut run.rng, n));
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
                    rebuild(run, view);
                    return;
                }
                let start_hint = if let StartHint::Pending(mut draws) = run.start_hint {
                    if run.config.surface == 4 {
                        draws.observe(source.damaging_floor(spot.line));
                    }
                    Some(draws.hint(run.config))
                } else {
                    None
                };
                if matches!(
                    run.controller.place(&mut run.adapter, setup),
                    Ok(Event::Placed)
                ) {
                    if let Some(hint) = start_hint {
                        run.start_hint = hint.map_or(
                            StartHint::Off,
                            |hint| StartHint::Visible(hint, 90),
                        );
                    }
                    run.pending = false;
                    run.decision = None;
                    run.after_reaction = 0;
                    run.recovering = false;
                }
            }
        }
        notice(run, None);
        if let StartHint::Visible(hint, left) = run.start_hint {
            run.start_hint = if left > 1 {
                StartHint::Visible(hint, left - 1)
            } else {
                StartHint::Off
            };
        }
    });
    drop(view);
    let admitted = running() && !open && !published::input_owners().covers_strip();
    super::btt_camera::request(model::TECH_PLACEMENT_OWNER, admitted && placing());
    super::btt_camera::request(model::TECH_CAMERA_OWNER, admitted && camera == 1);
}
