//! Mash out: real CPU moves and the approved existing-frame native clock.
//! Hooks publish values only; the existing diagnostic worker owns solver, grading and bests.

use super::{notices, recorder, states};
use crate::model::cpu::{
    hash::lua_bind_hash,
    ids::{C, COUNT, NAMES as CPU_NAMES},
    role::LOBBY_CPU_ENTRY,
};
use crate::model::mashout::{
    admission::{faulted, Admission, UNAVAILABLE_NOTICE},
    entry,
    moves::{Config, Scenario},
    native::Clock,
};
use crate::model::{
    input_delay::MappedInputs,
    notify::DEFAULT_COLOR,
    recording::{inject_values, Inject},
    scene::Scene,
    settings::Snapshot,
};
use crate::runtime::{
    game::{self, natives},
    published, scene, state_natives,
};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use hos_lock::{Mailbox, Mutex, Published};

#[path = "mashout/clatter_set.rs"]
pub(crate) mod clatter_set;
#[path = "mashout/interposer.rs"]
mod interposer;
#[path = "mashout/reader.rs"]
mod reader;
#[path = "mashout/worker.rs"]
mod worker;

const NOTICE_ID: u8 = 0x30;
const NOTICE_RAISE: u8 = 4;
static READY: AtomicBool = AtomicBool::new(false);
static SLOTS_READY: AtomicBool = AtomicBool::new(false);
static WANTED: AtomicBool = AtomicBool::new(false);
static CONFIG: Published<Option<Config>> = Published::new(None);
static START: AtomicBool = AtomicBool::new(false);
static GENERATION: AtomicU64 = AtomicU64::new(0);
static ARMED: AtomicU64 = AtomicU64::new(0);
static P1: AtomicUsize = AtomicUsize::new(0);
static THRESHOLD: AtomicU32 = AtomicU32::new(0);
static OWN_CPU: AtomicUsize = AtomicUsize::new(0);
static DRIVE: AtomicU64 = AtomicU64::new(0);
static CPU_ENTRY: AtomicI32 = AtomicI32::new(-1);
static TEMP_KIND: AtomicU32 = AtomicU32::new(u32::MAX);
static FAULT: AtomicU64 = AtomicU64::new(0);
// Independent serial: a native restart within an attempt invalidates its earlier solver.
static EPOCH: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SeedOrigin {
    Export,
    ThrowClatterStart,
}
#[derive(Clone, Copy)]
struct NativeSeed {
    generation: u64,
    epoch: u64,
    origin: SeedOrigin,
    sample: Option<reader::Sample>,
}
static SEEDS: Mailbox<NativeSeed, 4> = Mailbox::new();
#[derive(Clone, Copy)]
struct NativeEnd {
    generation: u64,
    epoch: u64,
    accessor: usize,
    threshold: f32,
}
static ENDS: Mailbox<NativeEnd, 4> = Mailbox::new();
static EVENTS: Mailbox<worker::Event, 128> = Mailbox::new();
static REPLIES: Mailbox<u64, 4> = Mailbox::new();

pub fn set_admission(on: bool) {
    READY.store(on, Ordering::Release);
}
fn admission() -> Admission {
    Admission::new(READY.load(Ordering::Acquire) && SLOTS_READY.load(Ordering::Acquire))
}

pub fn publish(rows: &Snapshot) {
    CONFIG.publish(Config::from_snapshot(rows));
    WANTED.store(
        crate::model::practice_games::is_on(rows, "mashout"),
        Ordering::Release,
    );
}
pub fn start() {
    START.store(true, Ordering::Release);
}
pub fn reset() {
    clatter_set::release();
    ARMED.store(0, Ordering::Release);
    OWN_CPU.store(0, Ordering::Release);
    start();
}
pub fn service() {
    interposer::service();
    worker::service();
}

/// M3 Set observer owner site (O2/O3): game thread only, from `lobby::update` (which `cpu::on_frame` runs first, before
/// the original). C9 retract on scope loss first, then restore/verify (O4 order). Never from the diag drain tick.
pub(crate) fn owner_site() {
    clatter_set::scene(scene::current().is_some());
    clatter_set::service();
}

/// Roster requests are temporary; its normal completion must not persist the practice attacker as the user's choice.
pub(crate) fn temporary_cpu_selection(kind: u32) -> bool {
    TEMP_KIND.load(Ordering::Acquire) == kind
}

fn post(event: worker::Event, generation: u64) {
    if !EVENTS.post(event) {
        fault(generation);
    }
}

fn fault(generation: u64) {
    FAULT.fetch_max(generation, Ordering::AcqRel);
}

fn revoke() {
    clatter_set::release();
    ARMED.store(0, Ordering::Release);
    drive(MappedInputs::NEUTRAL);
}

// One coherent mapped sample, through the recorder writer already guarded for this CPU.
fn drive(value: MappedInputs) {
    let mut bytes = [0; 8];
    bytes[..4].copy_from_slice(&value.buttons.to_le_bytes());
    bytes[4..].copy_from_slice(&[
        value.lstick_x as u8,
        value.lstick_y as u8,
        value.rstick_x as u8,
        value.rstick_y as u8,
    ]);
    DRIVE.store(u64::from_le_bytes(bytes), Ordering::Release);
}
pub(crate) fn control() -> Inject {
    let bytes = DRIVE.load(Ordering::Acquire).to_le_bytes();
    inject_values(
        MappedInputs {
            buttons: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            lstick_x: bytes[4] as i8,
            lstick_y: bytes[5] as i8,
            rstick_x: bytes[6] as i8,
            rstick_y: bytes[7] as i8,
        },
        1,
    )
}

#[derive(Clone, Copy)]
pub(crate) struct SeedToken {
    generation: u64,
    threshold: f32,
}

/// Before role_of/CPU.lock in the existing clatter-start wrapper. No plugin lock or allocation.
pub(crate) unsafe fn seed_token(accessor: usize) -> Option<SeedToken> {
    if !READY.load(Ordering::Acquire)
        || !SLOTS_READY.load(Ordering::Acquire)
        || accessor == 0
        || P1.load(Ordering::Acquire) != accessor
    {
        return None;
    }
    let generation = ARMED.load(Ordering::Acquire);
    if generation == 0
        || scene::current().is_none()
        || natives::module_accessor(0) != Some(accessor)
    {
        return None;
    }
    let threshold = f32::from_bits(THRESHOLD.load(Ordering::Acquire));
    (ARMED.load(Ordering::Acquire) == generation).then_some(SeedToken {
        generation,
        threshold,
    })
}

/// Post-original, while the original's live BOMA is still valid. Copy initialized record0, including preserved latches.
pub(crate) unsafe fn seed_after(
    accessor: usize,
    token: SeedToken,
    source: i8,
    index: i32,
    c: bool,
    _d: bool,
) {
    if !READY.load(Ordering::Acquire)
        || !SLOTS_READY.load(Ordering::Acquire)
        || seed_token(accessor).is_none_or(|fresh| fresh.generation != token.generation)
    {
        return;
    }
    let sample = admission()
        .with(|| {
            // Stock common initializers use both d values (Egg/Ridley/Corrin use true).
            // The initialized record carries those latch/reload values; never substitute defaults.
            (source == -1 && index == 0 && !c)
                .then(|| reader::sample(accessor, token.threshold))
                .flatten()
                .filter(|s| s.record.source == -1 && s.record.remaining > 0.0)
        })
        .flatten();
    publish_seed(token.generation, SeedOrigin::Export, sample);
}

/// Only the guarded native call-site observer uses this origin. It copies the
/// post-call record before the caller's next instruction, including carried latches.
pub(crate) unsafe fn throw_seed_after(control: usize) {
    let accessor = P1.load(Ordering::Acquire);
    let Some(token) = seed_token(accessor) else {
        return;
    };
    if reader::control(accessor) != Some(control) {
        return;
    }
    let sample = reader::sample(accessor, token.threshold)
        .filter(|s| s.record.source == -1 && s.record.remaining > 0.0);
    if seed_token(accessor).is_none_or(|fresh| fresh.generation != token.generation) {
        return;
    }
    publish_seed(token.generation, SeedOrigin::ThrowClatterStart, sample);
}

/// A native end is a distinct outcome only when it clears a positive live hold.
unsafe fn end_token(control: usize, index: i32) -> Option<NativeEnd> {
    let accessor = P1.load(Ordering::Acquire);
    let token = seed_token(accessor)?;
    if index != 0
        || reader::control(accessor) != Some(control)
        || reader::sample(accessor, token.threshold)?.record.remaining <= 0.0
    {
        return None;
    }
    Some(NativeEnd {
        generation: token.generation,
        epoch: EPOCH.load(Ordering::Acquire),
        accessor,
        threshold: token.threshold,
    })
}
unsafe fn end_after(control: usize, token: NativeEnd) {
    if EPOCH.load(Ordering::Acquire) != token.epoch
        || seed_token(token.accessor).is_none_or(|fresh| fresh.generation != token.generation)
        || reader::control(token.accessor) != Some(control)
        || reader::sample(token.accessor, token.threshold)
            .is_none_or(|sample| sample.record.remaining != 0.0)
    {
        return;
    }
    if !ENDS.post(token) {
        fault(token.generation);
    }
}

fn publish_seed(generation: u64, origin: SeedOrigin, sample: Option<reader::Sample>) {
    let epoch = EPOCH.fetch_add(1, Ordering::AcqRel) + 1;
    if !SEEDS.post(NativeSeed {
        generation,
        epoch,
        origin,
        sample,
    }) {
        fault(generation);
    }
}

/// Existing CPU hooks and AI epilogue yield only the freshly resolved, selected CPU body.
pub(crate) unsafe fn owns_cpu(accessor: usize) -> bool {
    if !READY.load(Ordering::Acquire)
        || !SLOTS_READY.load(Ordering::Acquire)
        || accessor == 0
        || OWN_CPU.load(Ordering::Acquire) != accessor
    {
        return false;
    }
    let entry = CPU_ENTRY.load(Ordering::Acquire);
    scene::current().is_some() && entry >= 0 && natives::module_accessor(entry) == Some(accessor)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Off,
    Choose,
    Reload,
    Place,
    Settle,
    Entering,
    Held,
    Result,
    Failed,
    Release,
    Retry,
    Restore,
}

struct Game {
    step: Step,
    generation: u64,
    epoch: u64,
    config: Option<Config>,
    scene: Option<Scene>,
    requested: Option<Scenario>,
    original: Option<u32>,
    p1: usize,
    cpu: usize,
    clock: Option<Clock>,
    /// M3 Set observer consumer for a 113/116 attempt (armed at Settle).
    writes: Option<crate::model::mashout::clatter::Writes>,
    /// An observed write was applied this attempt: no grade until G2 replays them in the bounds.
    applied: bool,
    saw_catch: bool,
    followed: bool,
    restore_requested: bool,
    trolley: Option<entry::TrolleyEntry>,
    hero: Option<entry::HeroEntry>,
    trail_magic: Option<entry::TrailMagicEntry>,
    /// P1's damage bits when the first preparatory magic cast began (Sora116 setup backstop).
    prep_damage: Option<u32>,
}
impl Game {
    const fn new() -> Self {
        Self {
            step: Step::Off,
            generation: 0,
            epoch: 0,
            config: None,
            scene: None,
            requested: None,
            original: None,
            p1: 0,
            cpu: 0,
            clock: None,
            writes: None,
            applied: false,
            saw_catch: false,
            followed: false,
            restore_requested: false,
            trolley: None,
            hero: None,
            trail_magic: None,
            prep_damage: None,
        }
    }
    fn step(&mut self, next: Step, text: &'static str) {
        if self.step == next {
            return;
        }
        if !matches!(next, Step::Entering | Step::Held) {
            // O4: every exit from an attempt closes the M3 observer gate (restore at the next owner-site call).
            clatter_set::release();
        }
        self.step = next;
        post(worker::Event::Setup(self.generation, text), self.generation);
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::enqueue_diagnostic(format_args!(
            "[Mashout] step gen={} step={next:?}",
            self.generation
        ));
    }
    fn fail(&mut self, reason: &'static str) {
        revoke();
        self.clock = None;
        self.step = Step::Failed;
        post(
            worker::Event::Failed(self.generation, reason),
            self.generation,
        );
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::enqueue_diagnostic(format_args!(
            "[Mashout] unscored gen={} reason={reason}",
            self.generation
        ));
    }
    fn end_practice(&mut self) {
        revoke();
        OWN_CPU.store(0, Ordering::Release);
        TEMP_KIND.store(u32::MAX, Ordering::Release);
        post(worker::Event::Cancel(self.generation), self.generation);
        *self = Self::new();
    }
}

// Only fighter callbacks take this lock. Neither the settings nor diagnostic worker touches it.
static GAME: Mutex<Game> = Mutex::new(Game::new());
const NAMES: [&str; 16] = [
    "FIGHTER_STATUS_KIND_WAIT",
    "FIGHTER_STATUS_KIND_CATCH",
    "FIGHTER_STATUS_KIND_CAPTURE_PULLED",
    "FIGHTER_STATUS_KIND_CAPTURE_WAIT",
    "FIGHTER_STATUS_KIND_CAPTURE_DAMAGE",
    "FIGHTER_STATUS_KIND_CAPTURE_CUT",
    "FIGHTER_STATUS_KIND_CAPTURE_JUMP",
    "SITUATION_KIND_GROUND",
    "FIGHTER_STATUS_KIND_CATCH_PULL",
    "FIGHTER_STATUS_KIND_CATCH_WAIT",
    "FIGHTER_STATUS_KIND_CATCH_ATTACK",
    "FIGHTER_STATUS_KIND_CAPTURE_PULLED_YOSHI",
    "FIGHTER_STATUS_KIND_CAPTURE_WAIT_YOSHI",
    "FIGHTER_STATUS_KIND_CAPTURE_DAMAGE_YOSHI",
    "FIGHTER_STATUS_KIND_CAPTURE_PULLED_PICKEL",
    "FIGHTER_STATUS_KIND_OTTOTTO_WAIT",
];
static IDS: [AtomicI32; NAMES.len()] = [const { AtomicI32::new(i32::MIN) }; NAMES.len()];
fn ids() -> Option<[i32; NAMES.len()]> {
    let mut values = [0; NAMES.len()];
    for (index, name) in NAMES.iter().enumerate() {
        let mut value = IDS[index].load(Ordering::Acquire);
        if value == i32::MIN {
            value = unsafe { game::lua_const(lua_bind_hash(name)) }?;
            IDS[index].store(value, Ordering::Release);
        }
        values[index] = value;
    }
    Some(values)
}
static STATE_IDS: [AtomicI32; COUNT] = [const { AtomicI32::new(i32::MIN) }; COUNT];
fn state_id(c: C) -> Option<i32> {
    let slot = &STATE_IDS[c as usize];
    let mut value = slot.load(Ordering::Acquire);
    if value == i32::MIN {
        value = unsafe { game::lua_const(lua_bind_hash(CPU_NAMES[c as usize])) }?;
        slot.store(value, Ordering::Release);
    }
    Some(value)
}
fn holding(request: Option<Scenario>, status: i32) -> bool {
    request.is_some_and(|r| entry::holding(r.state, |c| state_id(c) == Some(status)))
}
/// Native init_panel moves this roll from KIND into PREV_KIND, then clears KIND.
/// INIT also clears FIX before ACMD fixes it; that fresh edge makes PREV current.
unsafe fn judge_choice(cpu: usize, saw_unfixed: &mut bool) -> Result<Option<bool>, &'static str> {
    if !game::granted(crate::model::sites_pkg2::WORK_GET_INT_OFFSET)
        || !game::granted(crate::model::sites_pkg2::WORK_IS_FLAG_OFFSET)
    {
        // restriction: M3-READER-GUARD
        return Err("Mash out unavailable");
    }
    let Some(((special, fixed), kind)) = state_id(C::StatusSpecialS)
        .zip(state_id(C::JudgeFixed)).zip(state_id(C::JudgeKind))
    else {
        // restriction: M3-READER-GUARD
        return Err("Mash out unavailable");
    };
    if natives::status_kind(cpu) != special { return Ok(None); }
    let fixed = natives::is_flag(cpu, fixed);
    *saw_unfixed |= !fixed;
    entry::judge_outcome(*saw_unfixed, fixed, natives::get_int(cpu, kind))
}

/// Native owned-cart read only; the move creates and ejects its own article.
unsafe fn trolley_input(cpu: usize, status: i32, actor: &mut Option<entry::TrolleyEntry>)
    -> Result<MappedInputs, ()>
{
    use crate::model::sites_pkg3 as sites;
    if ![
        sites::STATE_ARTICLE_IS_EXIST_OFFSET, sites::STATE_ARTICLE_GET_ARTICLE_OFFSET,
        sites::STATE_ARTICLE_OBJECT_ID_OFFSET, sites::STATE_OBJECT_IS_ACTIVE_OFFSET,
        sites::STATE_OBJECT_MODULE_ACCESSOR_OFFSET, sites::STATE_LINK_PARENT_OBJECT_ID_OFFSET,
        crate::model::sites_pkg2::WORK_GET_INT_OFFSET,
    ].into_iter().all(game::granted) { return Err(()); }
    let [Some(special), Some(failed), Some(ride), Some(drive), Some(jump), Some(article),
        Some(link), Some(counter)] = [
        C::StatusSpecialS, C::PickelTrolleyFailed, C::PickelTrolleyRide,
        C::PickelTrolleyDrive, C::PickelTrolleyJump, C::PickelTrolleyArticle,
        C::PickelTrolleyLink, C::PickelTrolleyJumpDisable,
    ].map(state_id) else { return Err(()); };
    let phase = if status == failed { entry::TrolleyStatus::Failed }
        else if status == ride { entry::TrolleyStatus::Ride }
        else if status == drive { entry::TrolleyStatus::Drive }
        else if status == jump { entry::TrolleyStatus::Jump }
        else if status == special { entry::TrolleyStatus::Special }
        else { entry::TrolleyStatus::Other };
    let mut cart = None;
    let mut jump_disable = None;
    if state_natives::article_is_exist(cpu, article) {
        let object = state_natives::article_get(cpu, article);
        if object == 0 { return Err(()); }
        let object_id = state_natives::article_object_id(object);
        if !state_natives::object_is_active(object_id) { return Err(()); }
        let accessor = state_natives::object_accessor(object_id);
        if accessor == 0 { return Err(()); }
        if phase == entry::TrolleyStatus::Drive {
            if state_natives::link_parent_object_id(cpu, link) as u32 != object_id {
                return Err(());
            }
            jump_disable = Some(natives::get_int(accessor, counter));
        }
        if state_natives::article_get(cpu, article) != object
            || !state_natives::object_is_active(object_id)
            || state_natives::object_accessor(object_id) != accessor
        { return Err(()); }
        cart = Some(entry::OwnedTrolley { cpu, article: object, object_id, accessor });
    }
    if actor.is_none() {
        *actor = entry::TrolleyEntry::new(cpu);
    }
    actor
        .as_mut()
        .ok_or(())?
        .observe(
            phase,
            entry::TrolleyRead {
                readers_ready: true,
                cpu,
                cart,
                jump_disable,
            },
        )
        .map_err(|_| ())
}

unsafe fn trail_magic_kind(cpu: usize) -> Result<entry::TrailMagicKind, ()> {
    if !game::granted(crate::model::sites_pkg2::WORK_GET_INT_OFFSET) {
        return Err(());
    }
    let [Some(work), Some(fire), Some(blizzard), Some(thunder)] = [
        C::TrailMagicKind,
        C::TrailMagicFire,
        C::TrailMagicBlizzard,
        C::TrailMagicThunder,
    ]
    .map(state_id) else {
        return Err(());
    };
    match natives::get_int(cpu, work) {
        kind if kind == fire => Ok(entry::TrailMagicKind::Fire),
        kind if kind == blizzard => Ok(entry::TrailMagicKind::Blizzard),
        kind if kind == thunder => Ok(entry::TrailMagicKind::Thunder),
        _ => Err(()),
    }
}

/// Whether a preparatory Fire or Thunder (cloud) article is still live, through the guarded article reader.
unsafe fn trail_prep_article_live(cpu: usize) -> Result<bool, ()> {
    if !game::granted(crate::model::sites_pkg3::STATE_ARTICLE_IS_EXIST_OFFSET) {
        return Err(());
    }
    let [Some(fire), Some(cloud)] = [C::TrailArticleFire, C::TrailArticleCloud].map(state_id) else {
        return Err(());
    };
    Ok(state_natives::article_is_exist(cpu, fire) || state_natives::article_is_exist(cpu, cloud))
}

fn trail_magic_status(status: i32, wait: i32) -> Result<entry::TrailMagicStatus, ()> {
    let [Some(special), Some(n1), Some(n1_shoot), Some(n1_end), Some(n3)] = [
        C::StatusSpecialN,
        C::TrailSpecialN1,
        C::TrailSpecialN1Shoot,
        C::TrailSpecialN1End,
        C::TrailSpecialN3,
    ]
    .map(state_id) else {
        return Err(());
    };
    Ok(if status == wait {
        entry::TrailMagicStatus::Wait
    } else if status == special {
        entry::TrailMagicStatus::SpecialN
    } else if status == n1 {
        entry::TrailMagicStatus::N1
    } else if status == n1_shoot {
        entry::TrailMagicStatus::N1Shoot
    } else if status == n1_end {
        entry::TrailMagicStatus::N1End
    } else if status == n3 {
        entry::TrailMagicStatus::N3
    } else {
        entry::TrailMagicStatus::Other
    })
}

/// Olimar's Pikmin line through the Save-State helpers: respawn `line` first when given, then read it.
/// None, with nothing called, when a native or constant those helpers use is unavailable.
unsafe fn olimar_pikmin(cpu: usize, line: Option<[Option<i32>; 3]>) -> Option<[Option<i32>; 3]> {
    use crate::model::state::ids::{Table, C as S, NAMES as STATE_NAMES};
    use crate::model::{sites_pkg2 as p2, sites_pkg3 as p3, sites_pkg4 as p4};
    use crate::runtime::features::state_restore;
    if ![
        p3::STATE_WORK_GET_INT64_OFFSET, p3::STATE_OBJECT_IS_ACTIVE_OFFSET,
        p3::STATE_OBJECT_MODULE_ACCESSOR_OFFSET, p3::STATE_ARTICLE_REMOVE_EXIST_OFFSET,
        p3::STATE_ARTICLE_ACTIVE_NUM_OFFSET, p3::STATE_ARTICLE_GENERATE_OFFSET,
        p3::STATE_STATUS_REQUEST_OFFSET, p2::WORK_GET_INT_OFFSET, p4::WORK_SET_INT_OFFSET,
    ].into_iter().all(game::granted) { return None; }
    let mut values = [(S::OlimarHold0, 0); 9];
    for (slot, c) in values.iter_mut().zip([
        S::OlimarHold0, S::OlimarHold1, S::OlimarHold2, S::PikminVariation, S::ArticleOlimarPikmin,
        S::ArticleAllTarget, S::OlimarPreVariation, S::OlimarBeforePreVariation, S::PikminStatusAirFollow,
    ]) {
        *slot = (c, game::lua_const(lua_bind_hash(STATE_NAMES[c as usize]))?);
    }
    let ids = Table::with(&values);
    if let Some(line) = line {
        state_restore::olimar_respawn(cpu, &ids, &line);
    }
    Some(state_restore::olimar_line(cpu, &ids))
}

unsafe fn arm(p1: usize, generation: u64) -> bool {
    let Some(threshold) = admission().with(|| reader::threshold(p1)).flatten() else {
        return false;
    };
    P1.store(p1, Ordering::Release);
    THRESHOLD.store(threshold.to_bits(), Ordering::Release);
    ARMED.store(generation, Ordering::Release);
    true
}

/// Existing P1 per-frame path. Entry0's current active body only, independent of frame-strip settings.
pub unsafe fn frame(fighter: usize) {
    if !READY.load(Ordering::Acquire) || fighter == 0 {
        return;
    }
    if ((fighter + crate::model::lra_reset::FIGHTER_ENTRY_ID_FIELD) as *const i32).read() != 0 {
        return;
    }
    let entry = natives::fighter_entry(0);
    if natives::entry_active_body(entry) != fighter {
        return;
    }
    let Some(mut game) = GAME.try_lock() else {
        fault(GENERATION.load(Ordering::Acquire));
        revoke();
        OWN_CPU.store(0, Ordering::Release);
        return;
    };
    // Disabling restores the native slots immediately, but must still restore the
    // temporary CPU through the existing frame path.
    if WANTED.load(Ordering::Acquire) && !SLOTS_READY.load(Ordering::Acquire) {
        revoke();
        OWN_CPU.store(0, Ordering::Release);
        return;
    }
    let facts = scene::facts();
    if facts.busy {
        revoke();
        OWN_CPU.store(0, Ordering::Release);
        return;
    }
    let Some(where_now) = scene::current() else {
        game.end_practice();
        return;
    };
    if game.scene.is_some_and(|old| old != where_now) {
        game.end_practice();
    }
    game.scene = Some(where_now);
    let Some(ids) = ids() else {
        return;
    };
    let p1 = natives::fighter_accessor(fighter);
    let cpu_entry = if where_now == Scene::Training {
        1
    } else {
        LOBBY_CPU_ENTRY
    };
    let Some(cpu) = natives::module_accessor(cpu_entry) else {
        revoke();
        OWN_CPU.store(0, Ordering::Release);
        // Our loader removes the old body before publishing its replacement.
        // Setup and restore wait with inputs revoked until the real body returns.
        if game.generation != 0 && !matches!(game.step, Step::Reload | Step::Restore) {
            game.fail("Practice CPU changed · no grade");
        }
        return;
    };
    if p1 == 0 {
        revoke();
        OWN_CPU.store(0, Ordering::Release);
        return;
    }
    let config = CONFIG.read().filter(|_| WANTED.load(Ordering::Acquire));
    let restart = START.swap(false, Ordering::AcqRel) || game.config != config;
    let status = natives::status_kind(p1);
    let scored = holding(game.requested, status)
        || game.requested.is_some_and(|r| {
            r.plan == crate::model::mashout::moves::Plan::Grab && status == ids[14]
        });
    let linked = scored
        || game
            .requested
            .is_some_and(|r| entry::preparatory_holding(r.plan, |c| state_id(c) == Some(status)));
    if restart {
        revoke();
        post(worker::Event::Cancel(game.generation), game.generation);
        game.generation = GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
        game.config = config;
        game.clock = None;
        game.hero = None;
        game.trail_magic = None;
        game.prep_damage = None;
        game.restore_requested = false;
        game.step(
            if linked {
                Step::Release
            } else if config.is_some() {
                Step::Choose
            } else {
                Step::Restore
            },
            "Mash out · Waiting for release",
        );
    }
    if game.step == Step::Off && config.is_some() {
        game.config = config;
        game.step(Step::Choose, "Mash out · Setting up");
    }
    if matches!(game.step, Step::Held | Step::Entering | Step::Result)
        && (game.p1 != p1 || game.cpu != cpu)
    {
        game.fail("Fighter changed · no grade");
    }
    if !matches!(game.step, Step::Off | Step::Reload | Step::Restore) {
        CPU_ENTRY.store(cpu_entry, Ordering::Release);
        OWN_CPU.store(cpu, Ordering::Release);
    }
    if states::is_loading() || recorder::recording_or_playback() {
        revoke();
        OWN_CPU.store(0, Ordering::Release);
        game.clock = None;
        game.step(
            if config.is_some() {
                Step::Choose
            } else {
                Step::Restore
            },
            "Mash out · Waiting for recording or load",
        );
        return;
    }
    let covered = published::input_owners().covers_strip();
    if faulted(game.generation, FAULT.load(Ordering::Acquire)) {
        // restriction: MASH-M1-MISMATCH
        game.fail("Observation incomplete · no grade");
    }
    REPLIES.drain(|generation| {
        if generation == game.generation && game.step == Step::Result {
            game.step(
                if linked { Step::Release } else { Step::Choose },
                "Mash out · Setting up",
            );
        }
    });
    if game.step == Step::Entering {
        if game.requested.is_some_and(|r| r.plan == crate::model::mashout::moves::Plan::Judge) {
            match judge_choice(cpu, &mut game.followed) {
                Err(_) => { game.fail(UNAVAILABLE_NOTICE); return; }
                Ok(Some(false)) => {
                    // A positively observed other roll is uncounted. Keep this selected
                    // move through native release, including when catalog Random chose it.
                    revoke();
                    game.clock = None;
                    game.step(Step::Retry, "Mash out · Waiting for release");
                    #[cfg(feature = "diag")]
                    crate::runtime::diag_log::enqueue_diagnostic(format_args!(
                        "[Mashout] outcome=other gen={} move=111", game.generation
                    ));
                    return;
                }
                _ => (),
            }
        }
        let Some(request) = game.requested else { return; };
        if request.plan == crate::model::mashout::moves::Plan::Hero {
            let [Some(brave), Some(special_lw), Some(select), Some(cancel), Some(start), Some(failure),
                Some(window_open), Some(ice_sword), Some(index), Some(active), Some(window),
                Some(command1), Some(command2), Some(command3), Some(command4), Some(success)] = [
                C::KindBrave, C::StatusSpecialLw, C::HeroSelect, C::HeroCancel, C::HeroStart,
                C::HeroFailure, C::HeroWindowOpen, C::HeroIceSwordCommand, C::HeroSelectIndex,
                C::HeroActiveCommand, C::HeroWindowState, C::HeroCommand1, C::HeroCommand2,
                C::HeroCommand3, C::HeroCommand4, C::HeroSuccessSp,
            ].map(state_id) else {
                game.fail(UNAVAILABLE_NOTICE);
                return;
            };
            let body = natives::entry_active_body(natives::fighter_entry(cpu_entry));
            if body == 0
                || ((body + 0xc) as *const u32).read() != request.attacker_kind
                || request.attacker_kind != brave as u32
            {
                game.fail("Attacker changed · no grade");
                return;
            }
            let cpu_status = natives::status_kind(cpu);
            let status = if cpu_status == special_lw { entry::HeroStatus::SpecialLw }
                else if cpu_status == select { entry::HeroStatus::Select }
                else if cpu_status == cancel { entry::HeroStatus::Cancel }
                else if cpu_status == start { entry::HeroStatus::Start }
                else if cpu_status == failure { entry::HeroStatus::Failure }
                else { entry::HeroStatus::Other };
            let reads_int = status == entry::HeroStatus::Select || status == entry::HeroStatus::Start;
            if reads_int && !game::granted(crate::model::sites_pkg2::WORK_GET_INT_OFFSET) {
                game.fail(UNAVAILABLE_NOTICE);
                return;
            }
            if status == entry::HeroStatus::Start
                && !game::granted(crate::model::sites_pkg2::WORK_IS_FLAG_OFFSET)
            {
                game.fail(UNAVAILABLE_NOTICE);
                return;
            }
            let mut read = entry::HeroRead {
                readers_ready: true,
                cpu,
                status,
                ice_sword_command: ice_sword,
                window_state: None,
                index: None,
                commands: None,
                success_sp: None,
                active_command: None,
            };
            if status == entry::HeroStatus::Select {
                let window_state = natives::get_int(cpu, window);
                read.window_state = Some(window_state);
                if window_state == window_open {
                    read.index = Some(natives::get_int(cpu, index));
                    read.commands = Some([
                        natives::get_int(cpu, command1),
                        natives::get_int(cpu, command2),
                        natives::get_int(cpu, command3),
                        natives::get_int(cpu, command4),
                    ]);
                }
            }
            if status == entry::HeroStatus::Start {
                read.success_sp = Some(natives::is_flag(cpu, success));
                // Native START consumes pending DECIDE_COMMAND; ACTIVE_COMMAND is its current command.
                read.active_command = Some(natives::get_int(cpu, active));
            }
            let Some(mut actor) = game.hero.or_else(|| entry::HeroEntry::new(cpu)) else {
                game.fail(UNAVAILABLE_NOTICE);
                return;
            };
            match actor.observe(read) {
                Err(_) => { game.fail(UNAVAILABLE_NOTICE); return; }
                Ok((input, outcome)) => {
                    // The existing setup coverage gate replaces blocked inputs with
                    // neutral later in this frame. Keep the actor uncommitted until
                    // its proposed edge can actually be sent, but still drain seeds/ends.
                    if covered && input != MappedInputs::NEUTRAL {
                        drive(MappedInputs::NEUTRAL);
                    } else {
                        game.hero = Some(actor);
                        drive(input);
                        match outcome {
                            entry::HeroOutcome::Pending => (),
                            entry::HeroOutcome::Confirmed => game.followed = true,
                            entry::HeroOutcome::Cancelled => {
                                revoke();
                                game.clock = None;
                                game.step(Step::Retry, "Mash out · Waiting for release");
                                #[cfg(feature = "diag")]
                                crate::runtime::diag_log::enqueue_diagnostic(format_args!(
                                    "[Mashout] outcome=other gen={} move=110", game.generation
                                ));
                                return;
                            }
                            entry::HeroOutcome::Rejected => {
                                game.fail("Move entry unconfirmed · no grade");
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
    SEEDS.drain(|NativeSeed { generation, epoch, origin, sample }| {
        if generation != game.generation { return; }
        if game.requested.is_some_and(|r| r.plan == crate::model::mashout::moves::Plan::IceMagic)
            && game.trail_magic.is_some_and(|actor| actor.preparing())
        { return; }
        let late_throw = origin == SeedOrigin::ThrowClatterStart
            && game.step == Step::Held
            && game.requested.is_some_and(|r| r.state == crate::model::mashout::moves::State::Bury
                && r.plan == crate::model::mashout::moves::Plan::DownThrow);
        // restriction: M3-T2-NATIVE-EPOCH — only the observed639d44 origin may replace a held seed.
        if matches!(game.step, Step::Held | Step::Result) && !late_throw {
            game.fail("Capture restarted · no grade"); return;
        }
        if !late_throw && game.step != Step::Entering { return; }
        if origin == SeedOrigin::ThrowClatterStart && !late_throw {
            game.fail("Capture restarted · no grade"); return;
        }
        // restriction: M3-READER-GUARD
        let Some(sample) = sample else { game.fail("Native capture seed unavailable · no grade"); return; };
        if game.requested.is_some_and(|r| r.plan == crate::model::mashout::moves::Plan::Judge)
            && judge_choice(cpu, &mut game.followed) != Ok(Some(true))
        {
            game.fail("Move entry unconfirmed · no grade"); return;
        }
        if game.requested.is_some_and(|r| r.plan == crate::model::mashout::moves::Plan::Hero)
            && !game.hero.is_some_and(|actor| actor.confirmed())
        {
            game.fail("Move entry unconfirmed · no grade"); return;
        }
        if game.requested.is_some_and(|r| r.plan == crate::model::mashout::moves::Plan::Trolley)
            && state_id(C::ClatterCatchedPickelTrolley) != Some(status)
        {
            game.fail("Move entry unconfirmed · no grade"); return;
        }
        if game.requested.is_some_and(|r| r.plan == crate::model::mashout::moves::Plan::Cargo)
            && !entry::cargo_holding(|c| state_id(c) == Some(status))
        {
            game.fail("Move entry unconfirmed · no grade"); return;
        }
        let ordinary_catch = game.requested.is_some_and(|r| r.plan == crate::model::mashout::moves::Plan::Grab);
        let cpu_status = natives::status_kind(cpu);
        if !scored || (ordinary_catch && ![ids[1], ids[8], ids[9], ids[10]].contains(&cpu_status)) {
            game.fail("Move entry unconfirmed · no grade"); return;
        }
        // Cargo's native shoulder EXEC (common0x25b878/0x25b894) later rewrites automatic and
        // manual decay; until that order against the first eligible tick is proven and a
        // Cargo grading rule is reviewed, no Cargo seed may begin a grade (OPUS-REVIEW-84e9b33b §1).
        if sample.record.automatic == 0.0
            || game.requested.is_some_and(|r| r.plan == crate::model::mashout::moves::Plan::Cargo)
        {
            // restriction: MASH-M1-MISMATCH
            game.fail("Native tick observation unavailable for this hold"); return;
        }
        let seed = sample.seed();
        let Ok(clock) = Clock::new(seed.record) else { game.fail("Invalid native capture seed · no grade"); return; };
        let Some(request) = game.requested else { return; };
        let cpu_body = natives::entry_active_body(natives::fighter_entry(cpu_entry));
        if cpu_body == 0 || ((cpu_body + 0xc) as *const u32).read() != request.attacker_kind {
            game.fail("Attacker changed · no grade"); return;
        }
        let key = worker::Key::new(request.id, request.attacker_kind, ((fighter + 0xc) as *const u32).read(), natives::damage(p1, 0), seed, origin);
        drive(MappedInputs::NEUTRAL);
        game.clock = Some(clock);
        game.epoch = epoch;
        game.step = Step::Held;
        post(worker::Event::Begin(generation, epoch, key, seed, origin), generation);
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::enqueue_diagnostic(format_args!("[Mashout] seed gen={generation} key={key:?} seed={seed:?} input={:?} history={} current={:#x} previous={:#x} alternate={:#x} epoch={epoch} origin={origin:?}", sample.input, sample.history, sample.current, sample.previous, sample.alternate));
    });
    ENDS.drain(|ended| {
        if ended.generation == game.generation
            && ended.epoch == game.epoch
            && ended.accessor == p1
            && game.step == Step::Held
            && ![ids[5], ids[6]].contains(&status)
            && !game.clock.is_some_and(|clock| clock.exhausted())
            && !faulted(game.generation, FAULT.load(Ordering::Acquire))
        {
            revoke();
            game.clock = None;
            game.step = Step::Result;
            post(
                worker::Event::NativeEnd(game.generation, game.epoch),
                game.generation,
            );
            #[cfg(feature = "diag")]
            crate::runtime::diag_log::enqueue_diagnostic(format_args!(
                "[Mashout] native_end gen={} epoch={} outcome=ended_before_escape",
                game.generation, game.epoch
            ));
        }
    });
    let released = status == ids[0]
        && natives::status_kind(cpu) == ids[0]
        && natives::situation_kind(p1) == ids[7]
        && natives::situation_kind(cpu) == ids[7]
        && !natives::is_stop(p1)
        && !natives::is_stop(cpu);
    // Observe a live hold even under menus: only an actual native decrease counts.
    // Setup and the result delay wait for the same input owners as the other games.
    if covered && !matches!(game.step, Step::Held | Step::Result) {
        drive(MappedInputs::NEUTRAL);
        return;
    }
    match game.step {
        Step::Off => (),
        Step::Release if released => game.step(
            if config.is_some() {
                Step::Choose
            } else {
                Step::Restore
            },
            "Mash out · Setting up",
        ),
        Step::Release => (),
        Step::Retry if released => {
            post(worker::Event::Cancel(game.generation), game.generation);
            game.generation = GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
            game.saw_catch = false;
            game.followed = false;
            game.trolley = None;
            game.hero = None;
            game.trail_magic = None;
            game.prep_damage = None;
            game.step(Step::Reload, "Mash out · Setting up");
        }
        Step::Retry => (),
        Step::Choose => {
            if !released {
                return;
            }
            let Some(config) = config else {
                return;
            };
            let roll = skyline::nn::os::GetSystemTick() as u64;
            game.requested = config.resolve(roll, roll.rotate_left(17));
            let Some(request) = game.requested else {
                // restriction: MASH-M1-MISMATCH
                game.fail("Attacker unavailable");
                return;
            };
            let body = natives::entry_active_body(natives::fighter_entry(cpu_entry));
            if body == 0 {
                return;
            }
            let kind = ((body + 0xc) as *const u32).read();
            game.original.get_or_insert(kind);
            game.generation = GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
            game.p1 = p1;
            game.cpu = cpu;
            game.saw_catch = false;
            game.followed = false;
            game.trolley = None;
            game.hero = None;
            game.trail_magic = None;
            game.prep_damage = None;
            revoke();
            game.step = Step::Reload;
            post(
                worker::Event::Setup(game.generation, "Mash out · Loading attacker"),
                game.generation,
            );
            if kind != request.attacker_kind {
                if crate::runtime::roster::character_roster::cpu_reload_ready() {
                    TEMP_KIND.store(request.attacker_kind, Ordering::Release);
                    if !crate::runtime::roster::character_roster::request_cpu_kind(
                        request.attacker_kind,
                    ) {
                        // restriction: MASH-M1-MISMATCH
                        game.fail("Attacker unavailable");
                    }
                } else {
                    game.step = Step::Choose;
                }
            }
        }
        Step::Reload => {
            let body = natives::entry_active_body(natives::fighter_entry(cpu_entry));
            let wanted = game.requested.map(|r| r.attacker_kind);
            if body != 0 && crate::runtime::roster::character_roster::cpu_reload_ready() {
                if wanted != Some(((body + 0xc) as *const u32).read()) {
                    // restriction: MASH-M1-MISMATCH
                    game.fail("Attacker unavailable");
                    return;
                }
                game.p1 = p1;
                game.cpu = cpu;
                CPU_ENTRY.store(cpu_entry, Ordering::Release);
                OWN_CPU.store(cpu, Ordering::Release);
                game.step(Step::Place, "Mash out · Setting up move");
            }
        }
        Step::Place if !linked && natives::situation_kind(p1) == ids[7] => {
            let position = state_natives::Vector3f {
                x: natives::pos_x(p1),
                y: natives::pos_y(p1),
                z: natives::pos_z(p1),
            };
            let mut facing = natives::lr(p1);
            if ![position.x, position.y, position.z, facing]
                .iter()
                .all(|v| v.is_finite())
                || facing == 0.0
            {
                // restriction: MASH-M1-MISMATCH
                game.fail("Position unavailable");
                return;
            }
            state_natives::clear_speed_all(p1);
            state_natives::status_request(p1, ids[0]);
            state_natives::clear_speed_all(cpu);
            let Some(request) = game.requested else {
                return;
            };
            let drag = request.plan == crate::model::mashout::moves::Plan::Drag;
            if drag || request.plan == crate::model::mashout::moves::Plan::Flip {
                // Camera centre is a direction reference, never proof of collision floor/runway.
                // Flip faces P1 from the stage interior: P1's own facing is incidental after an
                // attempt and once put the measured 41-unit CPU off the stage (GEN11-DIAGNOSIS).
                let Some([left, right, _, _]) = natives::camera_limit_range()
                    .filter(|r| r.iter().all(|v| v.is_finite()) && r[0] < r[1])
                else {
                    // restriction: MASH-RIDLEY-FIXTURE
                    game.fail("Stage direction unavailable · no grade");
                    return;
                };
                let centre = left + (right - left) * 0.5;
                if position.x != centre {
                    let outward = (position.x - centre).signum();
                    facing = if drag { outward } else { -outward };
                }
            }
            let spacing = entry::placement_spacing(request);
            state_natives::init_pos(
                cpu,
                &state_natives::Vector3f {
                    x: position.x + spacing * facing.signum(),
                    ..position
                },
            );
            state_natives::set_lr(cpu, -facing.signum());
            state_natives::status_request(cpu, ids[0]);
            game.step(Step::Settle, "Mash out · Waiting for grounded fighters");
        }
        Step::Place => (),
        Step::Settle => {
            if status != ids[0]
                || natives::status_kind(cpu) != ids[0]
                || natives::situation_kind(cpu) != ids[7]
                || natives::situation_kind(p1) != ids[7]
                || natives::is_stop(p1)
                || natives::is_stop(cpu)
            {
                return;
            }
            let Some(request) = game.requested else {
                return;
            };
            // Olimar's ordinary grab is set up with exactly one Pikmin through the Save-State
            // respawn (OPUS-DESIGN-REVIEW-M3-row40-delegates §3 F1), replacing the real Pluck.
            if entry::one_pikmin_grab(request) && !game.followed {
                if olimar_pikmin(cpu, Some(entry::ONE_PIKMIN)).is_none() {
                    game.fail(UNAVAILABLE_NOTICE);
                    return;
                }
                drive(MappedInputs::NEUTRAL);
                game.saw_catch = false;
                game.step(Step::Entering, "Mash out · Waiting for move");
                return;
            }
            let mut preparing = false;
            if request.plan == crate::model::mashout::moves::Plan::IceMagic {
                let kind = match trail_magic_kind(cpu) {
                    Ok(kind) => kind,
                    Err(()) => {
                        game.fail(UNAVAILABLE_NOTICE);
                        return;
                    }
                };
                match game.trail_magic {
                    None if kind == entry::TrailMagicKind::Blizzard => (),
                    None => match entry::TrailMagicEntry::new(kind) {
                        Ok(actor) => {
                            game.trail_magic = Some(actor);
                            game.prep_damage = Some(natives::damage(p1, 0).to_bits());
                            preparing = true;
                        }
                        Err(entry::TrailMagicError::Unavailable) => {
                            game.fail(UNAVAILABLE_NOTICE);
                            return;
                        }
                    },
                    Some(actor) if actor.prepared() && kind == entry::TrailMagicKind::Blizzard => {
                        // Backstop (OPUS-REVIEW-sora116-45c1f9d6 F2 (a)): the preparation must have left P1's
                        // damage bit-equal and no live Fire/Thunder article before the Blizzard cast arms.
                        let Ok(live) = trail_prep_article_live(cpu) else {
                            game.fail(UNAVAILABLE_NOTICE);
                            return;
                        };
                        if live || game.prep_damage != Some(natives::damage(p1, 0).to_bits()) {
                            // restriction: mashout-sora-prep
                            game.fail("Setup hit you · no grade");
                            return;
                        }
                        game.trail_magic = None;
                        game.prep_damage = None;
                    }
                    Some(mut actor) => {
                        if actor.begin_thunder(kind).is_err() {
                            game.fail(UNAVAILABLE_NOTICE);
                            return;
                        }
                        game.trail_magic = Some(actor);
                        preparing = true;
                    }
                }
                if preparing {
                    // OPUS-REVIEW-sora116-45c1f9d6 F2 (b): preparatory casts face away from P1; the existing
                    // Place faces the CPU back toward P1 before the Blizzard cast.
                    let away = natives::pos_x(cpu) - natives::pos_x(p1);
                    if !away.is_finite() || away == 0.0 {
                        // restriction: MASH-M1-MISMATCH
                        game.fail("Position unavailable");
                        return;
                    }
                    state_natives::set_lr(cpu, away.signum());
                }
            }
            let Some(start) = entry::start(request).and_then(|name| game::lua_const(lua_bind_hash(name)))
            else {
                // restriction: MASH-M1-MISMATCH
                game.fail("Move sequence unavailable");
                return;
            };
            if !preparing && !entry::preparatory(request.plan) && !arm(p1, game.generation) {
                game.fail(UNAVAILABLE_NOTICE);
                return;
            }
            if preparing {
                revoke();
            }
            drive(entry::controls(request.plan, false));
            game.writes = None;
            game.applied = false;
            if matches!(request.id, 113 | 116 | 117) {
                // M3 Set observer (O5): installed and read back here, at Settle, until this attempt's exit.
                // SAFETY: p1 is the live P1 BOMA; +0x48 is its ControlModule (lua_bind ldr x0,[x0,#72]).
                let control = unsafe { *((p1 + 0x48) as *const usize) };
                let generation = game.generation as u32;
                let armed = clatter_set::arm(generation, request.id, control).ok();
                game.writes = armed.and_then(|g| crate::model::mashout::clatter::Writes::new(g, request.id));
                if game.writes.is_none() {
                    // restriction: MASH-NATIVE-SLOT
                    game.fail(UNAVAILABLE_NOTICE);
                    return;
                }
            }
            game.step(Step::Entering, "Mash out · Waiting for move");
            state_natives::status_request(cpu, start);
            #[cfg(feature = "diag")]
            crate::runtime::diag_log::enqueue_diagnostic(format_args!(
                "[Mashout] entry gen={} move={} state={:?} attacker={} plan={:?} variant={}",
                game.generation,
                request.id,
                request.state,
                request.attacker_kind,
                request.plan,
                request.grab
            ));
        }
        Step::Entering => {
            let cpu_status = natives::status_kind(cpu);
            let Some(request) = game.requested else {
                return;
            };
            if entry::one_pikmin_grab(request) && !game.followed {
                drive(MappedInputs::NEUTRAL);
                // Two settled reads must each show exactly one Pikmin before the configured Catch arms.
                if released {
                    let line = olimar_pikmin(cpu, None);
                    #[cfg(feature = "diag")]
                    crate::runtime::diag_log::enqueue_diagnostic(format_args!(
                        "[Mashout] olimar gen={} line={line:?}", game.generation
                    ));
                    if !line.is_some_and(entry::one_pikmin) {
                        game.fail(UNAVAILABLE_NOTICE);
                        return;
                    }
                    if game.saw_catch {
                        game.followed = true;
                        game.saw_catch = false;
                        game.step(Step::Place, "Mash out · Setting up move");
                    } else {
                        game.saw_catch = true;
                    }
                }
                return;
            }
            if request.plan == crate::model::mashout::moves::Plan::IceMagic {
                drive(MappedInputs::NEUTRAL);
                if let Some(mut actor) = game.trail_magic {
                    let kind = match trail_magic_kind(cpu) {
                        Ok(kind) => kind,
                        Err(()) => {
                            game.fail(UNAVAILABLE_NOTICE);
                            return;
                        }
                    };
                    let phase = match trail_magic_status(cpu_status, ids[0]) {
                        Ok(phase) => phase,
                        Err(()) => {
                            game.fail(UNAVAILABLE_NOTICE);
                            return;
                        }
                    };
                    match actor.observe(phase, kind, released) {
                        Ok(entry::TrailMagicProgress::Pending) => {
                            game.trail_magic = Some(actor);
                            return;
                        }
                        Ok(entry::TrailMagicProgress::StartThunder) => {
                            game.trail_magic = Some(actor);
                            game.step(Step::Settle, "Mash out · Waiting for grounded fighters");
                            return;
                        }
                        Ok(entry::TrailMagicProgress::Prepared) => {
                            game.trail_magic = Some(actor);
                            game.step(Step::Place, "Mash out · Setting up move");
                            return;
                        }
                        Err(entry::TrailMagicError::Unavailable) => {
                            game.fail(UNAVAILABLE_NOTICE);
                            return;
                        }
                    }
                }
            }
            if request.plan != crate::model::mashout::moves::Plan::Hero {
                if request.plan != crate::model::mashout::moves::Plan::IceMagic {
                    drive(entry::controls(request.plan, game.followed));
                }
            }
            game.saw_catch |= cpu_status != ids[0];
            if request.plan == crate::model::mashout::moves::Plan::Roller {
                let mut input = MappedInputs::NEUTRAL;
                input.lstick_x = if natives::lr(cpu) > 0.0 { 120 } else { -120 };
                drive(input);
            }
            if request.plan == crate::model::mashout::moves::Plan::Trolley {
                match trolley_input(cpu, cpu_status, &mut game.trolley) {
                    Ok(input) => drive(input),
                    Err(()) => { game.fail(UNAVAILABLE_NOTICE); return; }
                }
            }
            if request.plan == crate::model::mashout::moves::Plan::Pin && !game.followed {
                let Some((special, attack)) =
                    state_id(C::StatusSpecialS).zip(state_id(C::KamuiPinAttack))
                else {
                    game.fail(UNAVAILABLE_NOTICE);
                    return;
                };
                if !game::granted(crate::model::sites_pkg2::WORK_IS_FLAG_OFFSET) {
                    game.fail(UNAVAILABLE_NOTICE);
                    return;
                }
                // Native Kamui d1f74 opens the attack window; 1d410 consumes an
                // ordinary trigger there. Never request the spear or victim status.
                if cpu_status == special && natives::is_flag(cpu, attack) {
                    game.followed = true;
                    drive(entry::controls(request.plan, true));
                }
            }
            if request.plan == crate::model::mashout::moves::Plan::Freeze && !game.followed {
                let Some((hold, no_bang)) =
                    state_id(C::LucasFreezeHold).zip(state_id(C::LucasFreezeNoBang))
                else {
                    game.fail(UNAVAILABLE_NOTICE);
                    return;
                };
                // Native Lucas1f9c0: Special release shoots only after NOBANG_TIME<=0.
                // Observe HOLD and the game's counter; never count callbacks or wall time.
                if cpu_status == hold && natives::get_int(cpu, no_bang) <= 0 {
                    game.followed = true;
                    drive(entry::controls(request.plan, true));
                }
            }
            if request.plan == crate::model::mashout::moves::Plan::AirSideSpecial
                && !game.followed
                && state_id(C::SituationAir) == Some(natives::situation_kind(cpu))
            {
                let Some(special) = state_id(C::StatusSpecialS) else {
                    // restriction: MASH-M1-MISMATCH
                    game.fail("Move status unavailable");
                    return;
                };
                game.followed = true;
                state_natives::status_request(cpu, special);
            }
            if !game.followed
                && ((entry::preparatory_grab(request.plan) && cpu_status == ids[9])
                    || entry::preparatory_holding(request.plan, |c| state_id(c) == Some(status)))
            {
                if !arm(p1, game.generation) {
                    game.fail(UNAVAILABLE_NOTICE);
                    return;
                }
                game.followed = true;
                drive(entry::controls(request.plan, true));
            }
            if request.plan == crate::model::mashout::moves::Plan::Cargo
                && game.followed
                && cpu_status == ids[9]
            {
                // Native CatchCont0xca4d0 takes Cat2 THROW_F only from CATCH_WAIT; any
                // later attacker status (SHOULDER_START onward) keeps the neutral above.
                let Some(input) = entry::cargo_throw(natives::lr(cpu)) else {
                    game.fail(UNAVAILABLE_NOTICE);
                    return;
                };
                drive(input);
            }
            if game.saw_catch && cpu_status == ids[0] && !linked {
                game.fail("Move did not capture · reopen menu to retry");
            }
        }
        Step::Held => {
            let Some(mut clock) = game.clock else {
                return;
            };
            let before = clock.last();
            let Some(live) = admission()
                .with(|| reader::threshold(p1).and_then(|threshold| reader::sample(p1, threshold)))
                .flatten()
            else {
                game.fail(UNAVAILABLE_NOTICE);
                return;
            };
            let escaped = [ids[5], ids[6]].contains(&status);
            // M3 G1: this frame's published Set writes (seq order, validated) before the tick check.
            let mut writes = [crate::model::mashout::clatter::ClatterWrite {
                generation: 0, seq: 0, slot: crate::model::mashout::clatter::Slot::Set, tag: 0,
                old: 0, operand: 0, new: 0, frame: 0, d_max: 0,
            }; clatter_set::RING];
            let mut count = 0;
            let mut refused = false;
            let generation = game.generation as u32;
            if let Some(consumer) = game.writes.as_mut() {
                while count < clatter_set::RING {
                    let seq = consumer.next_seq();
                    match consumer.accept(clatter_set::poll(generation, seq)) {
                        Ok(Some(write)) => {
                            writes[count] = write;
                            count += 1;
                        }
                        Ok(None) => break,
                        Err(_) => {
                            refused = true;
                            break;
                        }
                    }
                }
            }
            game.applied |= count > 0;
            if refused || clock.observe_writes(live.record, live.input, escaped, &writes[..count]).is_err() {
                #[cfg(feature = "diag")]
                crate::runtime::diag_log::enqueue_diagnostic(format_args!(
                    "[Mashout] mismatch gen={} t={} before={before:?} live={live:?}",
                    game.generation,
                    clock.elapsed()
                ));
                game.fail("Native tick mismatch · no grade");
                return;
            }
            if game.applied && (escaped || clock.exhausted()) {
                // restriction: MASH-M1-MISMATCH — observed writes kept the clock exact, but the F/T bounds don't
                // replay them yet (G2), so the attempt stays unscored.
                game.fail("Native tick mismatch · no grade");
                return;
            }
            if !linked && !escaped && !clock.exhausted() {
                let cpu_status = natives::status_kind(cpu);
                if game.requested.is_some_and(|r| entry::native_landing_end(
                    r.plan,
                    |c| state_id(c) == Some(status),
                    |c| state_id(c) == Some(cpu_status),
                )) {
                    // The same informational outcome as an observed native end_clatter.
                    revoke();
                    game.clock = None;
                    game.step = Step::Result;
                    post(worker::Event::NativeEnd(game.generation, game.epoch), game.generation);
                    #[cfg(feature = "diag")]
                    crate::runtime::diag_log::enqueue_diagnostic(format_args!(
                        "[Mashout] native_end gen={} epoch={} outcome=ended_before_escape source=landing t={}",
                        game.generation, game.epoch, clock.elapsed()
                    ));
                    return;
                }
                game.fail("Capture interrupted · no grade");
                return;
            }
            if clock.last() != before || escaped {
                post(
                    worker::Event::Observe(
                        game.generation,
                        game.epoch,
                        clock.elapsed(),
                        clock.exhausted(),
                    ),
                    game.generation,
                );
                #[cfg(feature = "diag")]
                crate::runtime::diag_log::enqueue_diagnostic(format_args!(
                    "[Mashout] tick gen={} t={} before={before:?} live={live:?} exhausted={}",
                    game.generation,
                    clock.elapsed(),
                    clock.exhausted()
                ));
            }
            if clock.exhausted() {
                revoke();
                game.step = Step::Result;
            } else if !game.followed
                && game.requested.is_some_and(|r| r.plan == crate::model::mashout::moves::Plan::UpSpecial)
                && !natives::is_stop(cpu)
            {
                // Native Dedede279d0: Special triggers TURN only when the game's term permits it.
                // Wait through hitstop so the input edge reaches the live native move loop.
                game.followed = true;
                drive(entry::controls(crate::model::mashout::moves::Plan::UpSpecial, true));
            }
            game.clock = Some(clock);
        }
        Step::Result => post(
            worker::Event::Pulse(
                game.generation,
                game.epoch,
                !covered && !natives::is_stop(p1),
            ),
            game.generation,
        ),
        Step::Failed => (),
        Step::Restore => {
            if game.original.is_none() {
                game.end_practice();
                return;
            }
            // Restore owns no practice input, including while the original body is pending.
            revoke();
            OWN_CPU.store(0, Ordering::Release);
            // Primary-approved Restore-only resting state: grounded teeter is not a capture.
            if linked
                || status != ids[0]
                || ![ids[0], ids[15]].contains(&natives::status_kind(cpu))
                || natives::situation_kind(p1) != ids[7]
                || natives::situation_kind(cpu) != ids[7]
                || natives::is_stop(p1)
                || natives::is_stop(cpu)
                || !crate::runtime::roster::character_roster::cpu_reload_ready()
            {
                return;
            }
            let Some(original) = game.original else {
                game.end_practice();
                return;
            };
            let body = natives::entry_active_body(natives::fighter_entry(cpu_entry));
            if body == 0 {
                return;
            }
            if ((body + 0xc) as *const u32).read() == original {
                game.end_practice();
            } else if !game.restore_requested {
                TEMP_KIND.store(original, Ordering::Release);
                game.restore_requested =
                    crate::runtime::roster::character_roster::request_cpu_kind(original);
            }
        }
    }
}
