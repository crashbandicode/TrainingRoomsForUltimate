use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

use hos_lock::{Mailbox, Mutex};

use super::tech_surfaces::{self, Discovery, Found, NativeSource};
use super::{cpu::konst, notices, placement, practice, recorder, states};
use crate::engine::{library::Role, rr_host};
use crate::model::btt::Pad;
use crate::model::chords::{field, logical as key, Sample};
use crate::model::cpu::ids::{C, UNRESOLVED};
use crate::model::crosshair::{CpuPreview, Marks};
use crate::model::floor_hop::Floor;
use crate::model::notify::{Body, DEFAULT_COLOR};
use crate::model::record_place::{
    self, Act, Commit, End, Fighter, Kept, Leave, Live, Orders, Outcome, Place, Plan, Raw, Release, Seen, Setup, Shared,
    SharedStart, Shown, Status, Sync, Watch, Who, Wizard, CHECK_FRAMES, LIVE_FRAMES, READY_FRAMES,
};
use crate::model::state::{machine::StartWait, record::StartState};
use crate::model::storage::FixedPath;
use crate::model::tech::surface::{self, StageEpoch, Surfaces, MAX_LINES};
use crate::model::wizard::Kind;
use crate::runtime::{game::{self, natives}, published, scene, state_natives as state};

const NOTICE: u8 = 0x36;
/// Frames a save asked for may take to start its load before the picker lets the fighters go.
const SAVE_FRAMES: u16 = 180;
/// Frames a save's picker waits for the menu to close before the request is dropped.
const OPEN_FRAMES: u8 = 120;
type Stage = (i32, u32);

/// A save the worker asks the picker to open on (`open_save`): the worker's ticket for the visit, the save's name,
/// its two halves as the wizard shows them, and the macros that start from it.
#[derive(Clone, Copy)]
pub(crate) struct SaveOpen {
    pub ticket: u32,
    pub name: FixedPath<48>,
    pub saved: Setup,
    pub ledge: [bool; 2],
    pub edit: Who,
    pub shared: Shared,
    pub rule: SharedStart,
}

/// The saves asked for (worker to game), each with the session it was asked in, and the placements confirmed
/// (game to worker). No lock and no wait on either side.
static OPENS: Mailbox<(u32, SaveOpen), 2> = Mailbox::new();
static COMMITS: Mailbox<Commit, 2> = Mailbox::new();
/// Counts session clears: a save asked for in an earlier session is not opened in a later one.
static SESSION: AtomicU32 = AtomicU32::new(0);
/// The ticket of the visit whose save is with the worker (0: none), and of one that could not be written.
static SAVING: AtomicU32 = AtomicU32::new(0);
static FAILED: AtomicU32 = AtomicU32::new(0);
/// A Training reset rebuilt the stage's collision: the picker traces the floors again.
static RETRACE: AtomicBool = AtomicBool::new(false);
static OPEN: AtomicBool = AtomicBool::new(false);
static PAGES: AtomicBool = AtomicBool::new(false);
static HOLDING: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static WATCHING: AtomicBool = AtomicBool::new(false);
static LOAD_ACTIVE: AtomicBool = AtomicBool::new(false);
static CLEAR: AtomicBool = AtomicBool::new(false);
static CANCEL_LOAD: AtomicBool = AtomicBool::new(false);
static WAKE: AtomicU8 = AtomicU8::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static SHOWN: hos_lock::TripleBuffer<Shown> = hos_lock::TripleBuffer::new(Shown::IDLE);

struct Engine {
    kept: Kept,
    session: Option<Session>,
    targets: [Option<StartWait>; 2],
    shown: Shown,
    published: Shown,
    watch: Option<(Watch, bool)>,
    /// A save asked for while the menu was still closing, and the frames it has waited.
    asked: Option<(SaveOpen, u8)>,
    trace: Trace,
}

enum Session {
    Pages {
        wizard: Wizard,
        stage: Stage,
        recorded: Who,
        notice: FixedPath<224>,
        pad: Pad,
        /// Each fighter's own body at its spot (`placement::hold_at`).
        holds: [Option<CpuPreview>; 2],
        /// The save the wizard is opened on.
        save: Option<Visit>,
    },
    Start(Start),
}

/// A visit to a save: the worker's ticket, the save's name, and the frames since its save was asked for.
#[derive(Clone, Copy)]
struct Visit {
    ticket: u32,
    name: FixedPath<48>,
    saving: Option<u16>,
}

/// The stage's floors for the step between floors (operator 2026-11-25, PC1 B): Tech practice's surface walk, run
/// for the picker into its own lines.
struct Trace {
    surfaces: Surfaces,
    discovery: Discovery,
    found: Found,
}

impl Trace {
    const NEW: Self = Self { surfaces: Surfaces::EMPTY, discovery: Discovery::NEW, found: Found::More };

    /// One frame of the walk, on Player 1's pass. A new stage, form or Training session starts it again.
    unsafe fn step(&mut self, p1: usize, stage: Stage, range: [f32; 4]) {
        if RETRACE.swap(false, Ordering::AcqRel) {
            *self = Self::NEW;
        }
        let Some(training) = super::cpu::tech_player_receipt().ok().flatten() else { return };
        if self.surfaces.bind(StageEpoch { stage: stage.0, form: stage.1, training }) {
            (self.discovery, self.found) = (Discovery::NEW, Found::More);
        }
        if self.found == Found::More {
            self.found = match NativeSource::new(p1) {
                Some(mut source) => self.discovery.step(&mut source, range, &mut self.surfaces),
                None => Found::Failed,
            };
        }
    }
}

/// The traced floor lines, once the outline and its platform scan are both done. None before that, or after a
/// broken walk: the picker then stands its fighter on the floor under a free crosshair.
fn traced_floors(trace: &Trace, floors: &mut [Floor; MAX_LINES]) -> usize {
    if trace.found != Found::Done {
        return 0;
    }
    let mut count = 0;
    for line in trace.surfaces.lines().filter(|line| line.kind() == Some(surface::Kind::Floor)) {
        floors[count] = line.ends.map(|end| [end.x, end.y]);
        count += 1;
    }
    count
}

struct Start {
    sync: Sync,
    kind: StartKind,
    stage: Option<Stage>,
    orders: Orders,
    seen: [Option<Seen>; 2],
    status: [i32; 2],
    raw: [[Option<Raw>; 3]; 2],
    slept: [bool; 2],
    live_given: bool,
    idle_frames: u16,
    checked: bool,
    notice: FixedPath<224>,
}

enum StartKind {
    Record { setup: Setup, stage: Stage, recorded: Who },
    Load,
}

static ENGINE: Mutex<Engine> = Mutex::new(Engine {
    kept: Kept::new(),
    session: None,
    targets: [None; 2],
    shown: Shown::IDLE,
    published: Shown::IDLE,
    watch: None,
    asked: None,
    trace: Trace::NEW,
});

struct Publishing(hos_lock::MutexGuard<'static, Engine>);

impl core::ops::Deref for Publishing {
    type Target = Engine;
    fn deref(&self) -> &Engine { &self.0 }
}

impl core::ops::DerefMut for Publishing {
    fn deref_mut(&mut self) -> &mut Engine { &mut self.0 }
}

impl Drop for Publishing {
    fn drop(&mut self) { publish(&mut self.0); }
}

fn lock_engine() -> Option<Publishing> {
    ENGINE.try_lock().map(Publishing)
}

fn publish(engine: &mut Engine) {
    let mut shown = engine.shown;
    match &engine.session {
        None => shown.stage = record_place::Stage::Idle,
        Some(Session::Pages { wizard, recorded, .. }) => {
            shown.stage = record_place::Stage::Pages;
            shown.page = wizard.page;
            shown.edit = wizard.edit;
            shown.recorded = *recorded;
            shown.plans = wizard.plans();
        }
        Some(Session::Start(start)) => {
            shown.stage = record_place::Stage::Start;
            shown.recorded = match start.kind {
                StartKind::Record { recorded, .. } => recorded,
                StartKind::Load => Who::P1,
            };
            shown.plans = [*start.sync.plan(Who::P1), *start.sync.plan(Who::Cpu)];
        }
    }
    if shown != engine.published {
        SHOWN.write(shown);
        engine.published = shown;
    }
    engine.shown = shown;
}

pub fn shown() -> Shown {
    SHOWN.read().0
}

pub fn install(dependencies: bool) -> bool {
    states::set_start_states(dependencies);
    INSTALLED.store(dependencies, Ordering::Release);
    dependencies
}

pub(super) fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

impl Start {
    fn new(plans: [Plan; 2], count: u16, kind: StartKind, stage: Option<Stage>) -> Self {
        Self {
            sync: Sync::new(plans, count),
            kind,
            stage,
            orders: Orders { acts: [Act::None; 2], live: false, count: None, end: None },
            seen: [None; 2],
            status: [UNRESOLVED; 2],
            raw: [[None; 3]; 2],
            slept: [false; 2],
            live_given: false,
            idle_frames: 0,
            checked: false,
            notice: FixedPath::new(),
        }
    }
}

fn announce(up: bool) {
    published::WIZARDS.set_up(Kind::RecordPlacing, up);
}

fn stop_watch(engine: &mut Engine) {
    engine.watch = None;
    WATCHING.store(false, Ordering::Release);
}

fn clear_engine(engine: &mut Engine) {
    if CLEAR.swap(false, Ordering::AcqRel) {
        stop_watch(engine);
        engine.session = None;
        engine.targets = [None; 2];
        engine.asked = None;
        engine.kept = Kept::new();
        CANCEL_LOAD.store(false, Ordering::Release);
        WAKE.store(0, Ordering::Release);
    } else if CANCEL_LOAD.swap(false, Ordering::AcqRel) {
        if let Some(Session::Start(Start { kind: StartKind::Load, slept, .. })) = &engine.session {
            let mask = u8::from(slept[0]) | u8::from(slept[1]) << 1;
            WAKE.fetch_or(mask, Ordering::AcqRel);
            engine.session = None;
            if LOAD_ACTIVE.swap(false, Ordering::AcqRel) {
                rr_host::abort_warp();
            }
            HOLDING.store(false, Ordering::Release);
            ACTIVE.store(false, Ordering::Release);
        }
        engine.targets = [None; 2];
    }
}

/// The picker may open: none is open, and no practice game runs (that is refused).
fn may_open() -> bool {
    if ACTIVE.load(Ordering::Acquire) {
        return false;
    }
    if practice::active().is_some() {
        notices::refuse(record_place::PRACTICE_ACTIVE); // restriction: record-place-practice
        return false;
    }
    true
}

pub(super) fn open() {
    if may_open() {
        OPEN.store(true, Ordering::Release);
    }
}

/// Change placement: the worker asks the picker to open on a save it read. Worker thread.
pub(crate) fn open_save(open: SaveOpen) {
    if INSTALLED.load(Ordering::Acquire) && may_open() && OPENS.post((SESSION.load(Ordering::Acquire), open)) {
        OPEN.store(true, Ordering::Release);
    }
}

/// The newest save asked for in this session. Player 1's pass only: it is the one reader.
fn take_open() -> Option<SaveOpen> {
    let (session, mut newest) = (SESSION.load(Ordering::Acquire), None);
    OPENS.drain(|(asked_in, open)| {
        if asked_in == session {
            newest = Some(open);
        }
    });
    newest
}

/// A placement the picker confirmed, for the worker that writes the save. Worker thread: it is the one reader.
pub(crate) fn take_commit() -> Option<Commit> {
    let mut newest = None;
    COMMITS.drain(|commit| newest = Some(commit));
    newest
}

/// The picker still waits for the save of this visit: only then may the worker ask for its load.
pub(crate) fn waiting(ticket: u32) -> bool {
    let saving = SAVING.load(Ordering::Acquire);
    record_place::accepts((saving != 0).then_some(saving), ticket)
}

/// The save of this visit could not be written: its picker puts both fighters back and says so.
pub(crate) fn save_failed(ticket: u32) {
    FAILED.store(ticket, Ordering::Release);
}

/// A Training reset: the stage's collision may be rebuilt, so an open picker traces its floors again.
pub(crate) fn retrace() {
    RETRACE.store(true, Ordering::Release);
}

pub fn wizard_active() -> bool {
    PAGES.load(Ordering::Acquire) || HOLDING.load(Ordering::Acquire)
}

pub(super) fn placement_active() -> bool {
    PAGES.load(Ordering::Acquire) && !published::input_owners().covers_strip()
}

pub(super) fn holding() -> bool {
    HOLDING.load(Ordering::Acquire)
}

fn post_text(old: &mut FixedPath<224>, header: &'static str, text: FixedPath<224>) {
    if old.as_str() != text.as_str() {
        notices::upsert(NOTICE, header, text.as_str(), u16::MAX, DEFAULT_COLOR);
        *old = text;
    }
}

unsafe fn wake(slept: [bool; 2]) {
    if !slept[0] && !slept[1] {
        return;
    }
    let bodies = [natives::module_accessor(0), recorder::cpu_body().map(|body| body.accessor)];
    for i in 0..2 {
        if slept[i] {
            if let Some(body) = bodies[i] {
                state::set_sleep(body, false);
            }
        }
    }
}

unsafe fn leave(engine: &mut Engine, how: Leave) {
    if let Some(session) = engine.session.take() {
        match session {
            Session::Pages { holds, .. } => {
                super::btt_camera::interrupt_placement();
                let_go(holds, how);
            }
            Session::Start(start) => {
                wake(start.slept);
                if matches!(start.kind, StartKind::Load) {
                    rr_host::abort_warp();
                    states::finish_start();
                }
            }
        }
    }
    engine.targets = [None; 2];
    engine.asked = None;
    SAVING.store(0, Ordering::Release);
    OPEN.store(false, Ordering::Release);
    PAGES.store(false, Ordering::Release);
    HOLDING.store(false, Ordering::Release);
    ACTIVE.store(false, Ordering::Release);
    LOAD_ACTIVE.store(false, Ordering::Release);
    notices::clear(NOTICE);
    announce(false);
}

/// The picker lets go of the bodies it held (`record_place::release`). A body that is still the one it took is
/// woken, and after a cancel put back where it stood, unless a save state is loading. Any other body is left alone.
unsafe fn let_go(holds: [Option<CpuPreview>; 2], how: Leave) {
    let bodies = [natives::module_accessor(0), recorder::cpu_body().map(|body| body.accessor)];
    let loading = states::is_loading();
    for (hold, body) in holds.into_iter().zip(bodies) {
        let (Some(hold), Some(body)) = (hold, body) else { continue };
        let release = record_place::release(how, natives::battle_object_id(body) == hold.body(), loading);
        if release != Release::Nothing && hold.asleep {
            state::set_sleep(body, false);
        }
        if release == Release::PutBack {
            placement::restore(body, hold.origin());
        }
    }
}

unsafe fn end(engine: &mut Engine, start: Start, outcome: End) {
    engine.shown.ends += 1;
    engine.shown.end = Some((matches!(start.kind, StartKind::Record { .. }), outcome, start.live_given));
    engine.shown.frames = start.raw;
    if outcome == End::Started && start.live_given {
        engine.watch = Some((Watch::new(), matches!(start.kind, StartKind::Record { .. })));
        WATCHING.store(true, Ordering::Release);
    }
    let states = [start.sync.plan(Who::P1).state, start.sync.plan(Who::Cpu).state];
    let kind = match &start.kind {
        StartKind::Record { .. } => "record",
        StartKind::Load => "load",
    };
    let (result, who) = match outcome {
        End::Started => ("started", "-"),
        End::Refused(Who::P1) => ("refused", "p1"),
        End::Refused(Who::Cpu) => ("refused", "cpu"),
    };
    crate::runtime::diag_log::diagnostic(format_args!(
        "[TrainingRooms] record_place kind={kind} end={result} who={who} p1={} cpu={} p1_status={} cpu_status={}",
        states[0].key(), states[1].key(), start.status[0], start.status[1]
    ));
    wake(start.slept);
    notices::clear(NOTICE);
    match outcome {
        End::Started => match start.kind {
            StartKind::Record { setup, stage, .. } => engine.kept.put(stage, setup),
            StartKind::Load => states::finish_start(),
        },
        End::Refused(who) => {
            let mut text = FixedPath::<224>::new();
            match start.kind {
                StartKind::Record { .. } => {
                    let _ = record_place::write_refusal(true, who, states[who.index()], &mut text);
                    notices::post_body("Input Recording", Body::copied(text.as_str()), 120, DEFAULT_COLOR); // restriction: record-place-start
                }
                StartKind::Load => {
                    rr_host::abort_warp();
                    let _ = record_place::write_refusal(false, who, states[who.index()], &mut text);
                    notices::post_body("Input Recording", Body::copied(text.as_str()), 120, DEFAULT_COLOR); // restriction: record-place-load
                    states::finish_start();
                }
            }
        }
    }
    engine.targets = [None; 2];
    PAGES.store(false, Ordering::Release);
    HOLDING.store(false, Ordering::Release);
    ACTIVE.store(false, Ordering::Release);
    LOAD_ACTIVE.store(false, Ordering::Release);
    announce(false);
}

unsafe fn give_live(start: &Start) {
    match &start.kind {
        StartKind::Record { recorded, .. } => {
            let mut poses = [rr_host::live_pose(0), rr_host::live_pose(1)];
            for who in [Who::P1, Who::Cpu] {
                let plan = start.sync.plan(who);
                let pose = &mut poses[who.index()];
                pose.situation_kind = konst(match plan.place {
                    Place::OnStage => C::SituationGround,
                    Place::OffStage => C::SituationAir,
                });
                pose.x = plan.at[0];
                pose.y = plan.at[1];
                pose.percent = plan.percent.unwrap_or(pose.percent);
                pose.start_state = plan.state;
            }
            rr_host::set_record_role(if *recorded == Who::Cpu { Role::Cpu } else { Role::Player });
            rr_host::note_record_start(rr_host::RecordStart { poses, speeds: [(0.0, 0.0); 2] });
            rr_host::on_chords(true, false, false);
            rr_host::cpu_role_begin_placed();
            notices::clear(NOTICE);
            announce(false);
        }
        StartKind::Load => rr_host::start_gate(false),
    }
}

pub unsafe fn poll(controller: usize, sample: Sample, block: bool) -> bool {
    if !INSTALLED.load(Ordering::Acquire) { return false; }
    if !ACTIVE.load(Ordering::Acquire) {
        return false;
    }
    let Some(mut engine) = lock_engine() else {
        return HOLDING.load(Ordering::Acquire) || PAGES.load(Ordering::Acquire) && !block;
    };
    clear_engine(&mut engine);
    let Some(session) = engine.session.take() else { return false };
    match session {
        Session::Pages { wizard, stage, recorded, notice, mut pad, holds, save } => {
            if !block {
                pad.pressed |= sample.pressed;
                pad.held = sample.held;
                pad.stick = [0, 1].map(|i| ((controller + field::STICKS[i]) as *const f32).read_volatile());
            }
            engine.session = Some(Session::Pages { wizard, stage, recorded, notice, pad, holds, save });
            !block
        }
        Session::Start(mut start) => {
            if !block && !start.live_given && matches!(start.kind, StartKind::Record { .. })
                && sample.pressed & (1u16 << key::B) != 0
            {
                engine.session = Some(Session::Start(start));
                leave(&mut engine, Leave::Other);
                return true;
            }
            let ready = matches!(start.kind, StartKind::Record { .. }) || rr_host::start_ready();
            start.orders = start.sync.tick(std::mem::take(&mut start.seen), ready);
            start.idle_frames = 0;
            if let (Some(count), StartKind::Record { .. }) = (start.orders.count, &start.kind) {
                let mut text = FixedPath::<224>::new();
                let _ = record_place::write_ready(count, &mut text);
                post_text(&mut start.notice, text);
            }
            if start.orders.live {
                HOLDING.store(false, Ordering::Release);
                start.live_given = true;
                give_live(&start);
            }
            if let Some(outcome) = start.orders.end {
                let hide = !start.live_given;
                end(&mut engine, start, outcome);
                return hide;
            }
            let hide = !start.live_given;
            engine.session = Some(Session::Start(start));
            hide
        }
    }
}

unsafe fn stage() -> Option<Stage> {
    if !INSTALLED.load(Ordering::Acquire) { return None; }
    let id = natives::stage_id();
    Some((id, natives::stage_form(id)?))
}

fn fit([x, y]: [f32; 2], [left, right, top, bottom]: [f32; 4]) -> Option<[f32; 2]> {
    (x.is_finite() && y.is_finite() && left <= right && bottom <= top)
        .then(|| [x.clamp(left, right), y.clamp(bottom, top)])
}

fn granted(standby: i32) -> bool {
    if !INSTALLED.load(Ordering::Acquire) { return false; }
    use crate::model::{sites_pkg2 as s2, sites_pkg3 as s3, sites_pkg4 as s4};
    let constants = [
        C::StatusWait, C::StatusFall, C::StatusDamageFall, C::StatusFallSpecial,
        C::StatusTreadDamageAir, C::StatusDown, C::StatusSlip, C::StatusDead,
        C::StatusRebirth, C::SituationGround, C::SituationAir, C::FlagDownUp,
        C::WorkIntJumpCount,
    ];
    standby != UNRESOLVED && constants.into_iter().all(|id| konst(id) != UNRESOLVED)
        && [
            s3::STATE_CLEAR_SPEED_ALL_OFFSET, s3::STATE_INIT_POS_OFFSET, s3::STATE_SET_POS_OFFSET,
            s3::STATE_SET_LR_OFFSET, s3::STATE_SET_SITUATION_KIND_OFFSET, s3::STATE_STATUS_REQUEST_OFFSET,
            s3::STATE_SET_SLEEP_OFFSET, s3::STATE_HEAL_OFFSET, s3::STATE_ADD_DAMAGE_OFFSET,
            s4::WORK_SET_INT_OFFSET, s4::POSTURE_LR_OFFSET, s2::STATUS_KIND_OFFSET,
            s2::SITUATION_KIND_OFFSET, s2::POS_X_OFFSET, s2::POS_Y_OFFSET, s2::DAMAGE_OF_OFFSET,
            s2::WORK_IS_FLAG_OFFSET, s2::WORK_SET_FLAG_OFFSET, s2::MOTION_FRAME_OFFSET,
            s2::CHANGE_STATUS_FORCE_OFFSET, s2::MOTION_KIND_OFFSET, s2::MOTION_CHANGE_HOOK_OFFSET,
        ].into_iter().all(game::granted)
}

struct DownAccess(usize);

impl record_place::DownBody for DownAccess {
    fn force_down(&mut self) {
        unsafe { natives::change_status_force(self.0, konst(C::StatusDown), false) }
    }

    fn face(&self) -> Option<(bool, u64)> {
        unsafe {
            (natives::status_kind(self.0) == konst(C::StatusDown))
                .then(|| (natives::is_flag(self.0, konst(C::FlagDownUp)), natives::motion_kind(self.0)))
        }
    }

    fn set_face(&mut self, up: bool, motion: u64) {
        unsafe {
            natives::set_flag(self.0, up, konst(C::FlagDownUp));
            natives::change_motion(self.0, motion, 0.0, 1.0);
        }
    }
}

unsafe fn act(body: usize, plan: Plan, action: Act, slept: &mut bool) -> bool {
    let lr = natives::lr(body);
    match action {
        Act::None => {}
        Act::Write => {
            let damage = natives::damage(body, 0);
            if plan.percent.is_some() && !damage.is_finite() {
                return false;
            }
            match plan.place {
                Place::OnStage => placement::stand(body, plan.at, lr),
                Place::OffStage => {
                    placement::lift(body, plan.at, lr);
                    natives::set_int(body, 1, konst(C::WorkIntJumpCount));
                    placement::fall(body);
                }
            }
            if let Some(percent) = plan.percent {
                state::heal(body, -damage);
                state::add_damage(body, percent);
            }
        }
        Act::Hold => {
            placement::hold(body, plan.at);
            *slept = true;
        }
        Act::Request => {
            match plan.place {
                Place::OnStage => {
                    if plan.state == StartState::Standard {
                        placement::stand(body, plan.at, lr);
                    } else {
                        placement::ground(body, plan.at, lr);
                    }
                }
                Place::OffStage => {
                    placement::lift(body, plan.at, lr);
                    state::set_sleep(body, false);
                    *slept = false;
                }
            }
            match plan.state {
                StartState::Standard => {}
                StartState::MissedTechUp | StartState::MissedTechDown => {
                    record_place::enter_down(&mut DownAccess(body), record_place::down_face(plan.state).unwrap());
                }
                _ => natives::change_status_force(body, konst(record_place::first_status(plan.state, plan.place)), false),
            }
        }
    }
    true
}

unsafe fn read_raw(accessor: usize) -> Raw {
    Raw {
        status: natives::status_kind(accessor),
        situation: natives::situation_kind(accessor),
        at: [natives::pos_x(accessor), natives::pos_y(accessor)],
        damage: natives::damage(accessor, 0),
        lr: natives::lr(accessor),
        motion_frame: natives::motion_frame(accessor),
        face_up: natives::is_flag(accessor, konst(C::FlagDownUp)),
        motion: natives::motion_kind(accessor),
    }
}

pub unsafe fn frame(fighter: usize) {
    if !INSTALLED.load(Ordering::Acquire) { return; }
    if fighter == 0 || !OPEN.load(Ordering::Acquire) && !ACTIVE.load(Ordering::Acquire)
        && !CANCEL_LOAD.load(Ordering::Acquire) && !WATCHING.load(Ordering::Acquire)
        && WAKE.load(Ordering::Acquire) == 0 {
        return;
    }
    let Some(p1) = natives::module_accessor(0) else { return };
    let accessor = natives::fighter_accessor(fighter);
    let cpu = recorder::cpu_body();
    let who = if accessor == p1 {
        Who::P1
    } else if cpu.is_some_and(|body| body.fighter == fighter) {
        Who::Cpu
    } else {
        return;
    };
    let standby = if ACTIVE.load(Ordering::Acquire) && !PAGES.load(Ordering::Acquire) {
        states::standby_status()
    } else {
        UNRESOLVED
    };
    let Some(mut engine) = lock_engine() else { return };
    clear_engine(&mut engine);
    if who == Who::P1 {
        let waking = WAKE.swap(0, Ordering::AcqRel);
        if waking != 0 {
            wake([waking & 1 != 0, waking & 2 != 0]);
        }
        let leaving = engine.session.as_ref().is_some_and(|session| {
            let (before_live, record, old_stage) = match session {
                Session::Pages { stage, .. } => (true, true, Some(*stage)),
                Session::Start(start) => (!start.live_given, matches!(start.kind, StartKind::Record { .. }), start.stage),
            };
            before_live && (scene::current().is_none()
                || published::HOT_SWAP_IN_PROGRESS.load(Ordering::Acquire)
                || record && (published::input_owners().covers_strip() || states::is_loading())
                || stage() != old_stage || cpu.is_none() || practice::active().is_some())
        });
        if leaving {
            leave(&mut engine, Leave::Other);
            return;
        }
        if OPEN.swap(false, Ordering::AcqRel) && engine.session.is_none() && scene::current().is_some() {
            let asked = take_open().map(|open| (open, 0)).or(engine.asked.take());
            let covered = published::input_owners().covers_strip() || states::is_loading();
            if let Some((open, waited)) = asked.filter(|_| covered) {
                // The menu that asked is still closing: the save waits for it, for a moment.
                if waited < OPEN_FRAMES {
                    engine.asked = Some((open, waited + 1));
                    OPEN.store(true, Ordering::Release);
                }
            } else if let (Some(cpu), Some(stage), Some(camera)) = (cpu, stage(), natives::camera_limit_range()) {
                let blast = tech_surfaces::training_blast_bounds();
                let reach = |place| record_place::reach(place, camera, blast);
                let opened = match asked {
                    Some((open, _)) => {
                        // A save's spots are shown fitted to this stage; the file keeps its own until one is edited.
                        let fitted = open.saved.map(|fighter| fit(fighter.at, reach(fighter.place)).map(|at| Fighter { at, ..fighter }));
                        let visit = Visit { ticket: open.ticket, name: open.name, saving: None };
                        match fitted {
                            [Some(first), Some(second)] => Some((
                                Wizard::for_save([first, second], open.ledge, open.edit).sharing(open.shared, open.rule),
                                Some(visit),
                            )),
                            _ => None,
                        }
                    }
                    None => {
                        let live = [p1, cpu.accessor].map(|body| {
                            let at = [natives::pos_x(body), natives::pos_y(body)];
                            (at, natives::damage(body, 0), placement::floor_under(body, at, true))
                        });
                        record_place::defaults(live, |at| fit(at, reach(Place::OffStage)))
                            .map(|defaults| (Wizard::new(defaults, engine.kept.get(stage)), None))
                    }
                };
                if let Some((mut wizard, save)) = opened {
                    let [left, right, top, _] = camera;
                    wizard.set_home(placement::floor_under(p1, [left.midpoint(right), top], false));
                    let recorded = if rr_host::record_role() == Role::Cpu { Who::Cpu } else { Who::P1 };
                    stop_watch(&mut engine);
                    engine.trace = Trace::NEW;
                    engine.session = Some(Session::Pages {
                        wizard, stage, recorded, notice: FixedPath::new(), pad: Pad::default(), holds: [None; 2], save,
                    });
                    ACTIVE.store(true, Ordering::Release);
                    PAGES.store(true, Ordering::Release);
                    announce(true);
                }
            }
        }
    }
    let sample = engine.watch.as_mut().and_then(|(watch, record)| watch.step(who).map(|i| (i, *record)));
    if let Some((i, record)) = sample {
        let tick = if record { rr_host::captured_frames() } else { rr_host::played_frames() };
        engine.shown.live[who.index()][i] = Some(Live { frames: LIVE_FRAMES[i], tick, raw: read_raw(accessor) });
    }
    if engine.watch.as_ref().is_some_and(|(watch, _)| watch.done()) {
        stop_watch(&mut engine);
    }
    let Some(session) = engine.session.take() else { return };
    match session {
        Session::Pages { mut wizard, stage, recorded, mut notice, mut pad, mut holds, mut save } => {
            if who == Who::P1 {
                let camera = natives::camera_limit_range();
                if let Some(camera) = camera {
                    engine.trace.step(p1, stage, camera);
                }
                let saving = match &mut save {
                    Some(Visit { ticket, saving: Some(frames), .. }) => {
                        *frames = frames.saturating_add(1);
                        Some((*ticket, *frames > SAVE_FRAMES))
                    }
                    _ => None,
                };
                if let Some((ticket, late)) = saving {
                    // The save is with the worker: both fighters wait at their spots for its load.
                    pad.pressed = 0;
                    let failed = record_place::accepts(Some(ticket), FAILED.swap(0, Ordering::AcqRel));
                    if failed || late {
                        let name = save.map_or(FixedPath::new(), |visit| visit.name);
                        engine.session = Some(Session::Pages { wizard, stage, recorded, notice, pad, holds, save });
                        if !failed {
                            leave(&mut engine, Leave::Other);
                            return;
                        }
                        // The file could not be written: both fighters go back, as for B, and the notice says so.
                        leave(&mut engine, Leave::Cancel);
                        let mut text = FixedPath::<224>::new();
                        let _ = record_place::write_not_saved(name.as_str(), &mut text);
                        notices::post_body("Input Recording", Body::copied(text.as_str()), 120, DEFAULT_COLOR); // restriction: place-save-write
                        return;
                    }
                } else {
                    let edited = wizard.setup[wizard.edit.index()];
                    let blast = tech_surfaces::training_blast_bounds();
                    let mut floors = [[[0.0; 2]; 2]; MAX_LINES];
                    let count = traced_floors(&engine.trace, &mut floors);
                    let outcome = wizard.step_on(&pad,
                        |at| camera.and_then(|camera| fit(at, record_place::reach(edited.place, camera, blast))),
                        |at| placement::floor_under(p1, at, true), &floors[..count]);
                    pad.pressed = 0;
                    match outcome {
                        Outcome::Closed => {
                            engine.session = Some(Session::Pages { wizard, stage, recorded, notice, pad, holds, save });
                            leave(&mut engine, Leave::Cancel);
                            return;
                        }
                        Outcome::Start => match &mut save {
                            Some(visit) => {
                                // A save: the worker writes the file and asks for one load of it.
                                visit.saving = Some(0);
                                SAVING.store(visit.ticket, Ordering::Release);
                                if !COMMITS.post(wizard.commit(visit.ticket, stage.0)) {
                                    FAILED.store(visit.ticket, Ordering::Release);
                                }
                                crate::runtime::ui::worker::wake();
                            }
                            None => {
                                // A Record start places both fighters itself: a body held asleep is woken for it.
                                let_go(holds, Leave::Other);
                                let setup = wizard.setup;
                                let start = Start::new(wizard.plans(), READY_FRAMES,
                                    StartKind::Record { setup, stage, recorded }, Some(stage));
                                stop_watch(&mut engine);
                                engine.shown.live = [[None; 2]; 2];
                                PAGES.store(false, Ordering::Release);
                                HOLDING.store(true, Ordering::Release);
                                engine.session = Some(Session::Start(start));
                                return;
                            }
                        },
                        Outcome::Open => {
                            let mut text = FixedPath::<224>::new();
                            let (header, name) = match &save {
                                Some(visit) => ("Change placement", visit.name.as_str()),
                                None => ("Recording", ""),
                            };
                            let _ = wizard.write_text(recorded, name, &mut text);
                            post_text(&mut notice, header, text);
                        }
                    }
                }
            }
            // The fighter's own body stands on its floor point, or is held at its spot in the air.
            let fighter = wizard.setup[who.index()];
            placement::hold_at(&mut holds[who.index()], accessor, natives::battle_object_id(accessor), fighter.at,
                fighter.place == Place::OnStage);
            engine.session = Some(Session::Pages { wizard, stage, recorded, notice, pad, holds, save });
        }
        Session::Start(mut start) => {
            if who == Who::P1 {
                start.idle_frames = start.idle_frames.saturating_add(1);
                if start.idle_frames > CHECK_FRAMES {
                    end(&mut engine, start, End::Refused(Who::P1));
                    return;
                }
            }
            if !start.checked {
                if !granted(standby) {
                    end(&mut engine, start, End::Refused(Who::P1));
                    return;
                }
                start.checked = true;
            }
            let i = who.index();
            let before_status = natives::status_kind(accessor);
            let usable = ![konst(C::StatusDead), konst(C::StatusRebirth), standby].contains(&before_status);
            let plan = *start.sync.plan(who);
            let action = start.orders.acts[i];
            if usable && !act(accessor, plan, action, &mut start.slept[i]) {
                end(&mut engine, start, End::Refused(who));
                return;
            }
            let requested = usable && action == Act::Request;
            if requested {
                start.orders.acts[i] = Act::None;
            }
            let raw = read_raw(accessor);
            let raw_status = raw.status;
            let situation = raw.situation;
            let standard = konst(record_place::first_status(StartState::Standard, plan.place));
            let expected = konst(record_place::first_status(plan.state, plan.place));
            let status = if raw_status == standard {
                Status::Standard
            } else if plan.state != StartState::Standard && raw_status == expected {
                Status::State
            } else {
                Status::Other
            };
            start.status[i] = raw_status;
            start.raw[i].rotate_left(1);
            start.raw[i][2] = Some(raw);
            start.seen[i] = Some(Seen {
                usable,
                requested,
                status,
                grounded: if situation == konst(C::SituationGround) { Some(true) }
                    else if situation == konst(C::SituationAir) { Some(false) } else { None },
                face_up: raw.face_up,
                motion: raw.motion,
                at: raw.at,
                damage: raw.damage,
            });
            engine.session = Some(Session::Start(start));
        }
    }
}

pub fn look(active: bool) -> Marks {
    let empty = Marks { looks: [None; crate::model::btt::LOOKS], pointer: None };
    if !active || !PAGES.load(Ordering::Acquire) {
        return empty;
    }
    let Some(engine) = ENGINE.try_lock() else { return empty };
    match &engine.session {
        Some(Session::Pages { wizard, holds, .. }) => {
            // A fighter whose own body is at its spot gets no mark. The game shows an out-of-view body in its
            // magnifier; without the game HUD the magnifier is hidden too, so the border pointer stays.
            let held = [holds[0].is_some(), holds[1].is_some()];
            wizard.marks(held).shown(held[wizard.edit.index()] && published::hud())
        }
        _ => empty,
    }
}

pub(super) fn load_target(role: usize, target: StartWait) {
    if !INSTALLED.load(Ordering::Acquire) { return; }
    if role >= 2 {
        return;
    }
    let Some(mut engine) = lock_engine() else { return };
    clear_engine(&mut engine);
    if matches!(engine.session, Some(Session::Pages { .. } | Session::Start(Start { kind: StartKind::Record { .. }, .. }))) {
        // A load places the fighters itself: the picker only wakes the bodies it held.
        unsafe { leave(&mut engine, Leave::Other) };
    }
    if engine.session.is_some() {
        return;
    }
    engine.targets[role] = Some(target);
    let [Some(p1), Some(cpu)] = engine.targets else { return };
    let plans = [p1, cpu].map(|target| Plan {
        place: if target.grounded { Place::OnStage } else { Place::OffStage },
        at: [target.x, target.y],
        state: target.state,
        percent: None,
    });
    let current_stage = unsafe { stage() };
    engine.targets = [None; 2];
    stop_watch(&mut engine);
    engine.shown.live = [[None; 2]; 2];
    engine.session = Some(Session::Start(Start::new(plans, 0, StartKind::Load, current_stage)));
    ACTIVE.store(true, Ordering::Release);
    LOAD_ACTIVE.store(true, Ordering::Release);
    HOLDING.store(true, Ordering::Release);
    rr_host::start_gate(true);
    rr_host::warp_landed();
}

pub(super) fn load_cancelled() -> bool {
    let dropped = LOAD_ACTIVE.swap(false, Ordering::AcqRel);
    CANCEL_LOAD.store(true, Ordering::Release);
    if dropped {
        HOLDING.store(false, Ordering::Release);
        ACTIVE.store(false, Ordering::Release);
    }
    if let Some(mut engine) = lock_engine() {
        clear_engine(&mut engine);
        engine.targets = [None; 2];
    }
    dropped
}

pub(super) fn session_clear() {
    SESSION.fetch_add(1, Ordering::AcqRel);
    SAVING.store(0, Ordering::Release);
    WATCHING.store(false, Ordering::Release);
    OPEN.store(false, Ordering::Release);
    PAGES.store(false, Ordering::Release);
    HOLDING.store(false, Ordering::Release);
    ACTIVE.store(false, Ordering::Release);
    if LOAD_ACTIVE.swap(false, Ordering::AcqRel) {
        rr_host::abort_warp();
    }
    CLEAR.store(true, Ordering::Release);
    announce(false);
    if let Some(mut engine) = lock_engine() {
        clear_engine(&mut engine);
    }
}
