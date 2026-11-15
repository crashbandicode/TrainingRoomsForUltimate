//! Mash out: real CPU moves and the approved existing-frame native clock.
//! Hooks publish values only; the existing diagnostic worker owns solver, grading, bests and cache.

use super::{notices, recorder, states};
use crate::model::cpu::{hash::lua_bind_hash, role::LOBBY_CPU_ENTRY};
use crate::model::mashout::{
    admission::{Admission, UNAVAILABLE_NOTICE},
    moves::{Config, OrdinaryGrab},
    native::{Clock, Seed},
};
use crate::model::{notify::DEFAULT_COLOR, scene::Scene, settings::Snapshot};
use crate::runtime::{
    game::{self, natives},
    published, scene, state_natives,
};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use hos_lock::{Mailbox, Mutex, Published};

#[path = "mashout/reader.rs"]
mod reader;
#[path = "mashout/worker.rs"]
mod worker;

const NOTICE_ID: u8 = 0x30;
const NOTICE_RAISE: u8 = 4;
static READY: AtomicBool = AtomicBool::new(false);
static WANTED: AtomicBool = AtomicBool::new(false);
static CONFIG: Published<Option<Config>> = Published::new(None);
static START: AtomicBool = AtomicBool::new(false);
static GENERATION: AtomicU64 = AtomicU64::new(0);
static ARMED: AtomicU64 = AtomicU64::new(0);
static P1: AtomicUsize = AtomicUsize::new(0);
static THRESHOLD: AtomicU32 = AtomicU32::new(0);
static OWN_CPU: AtomicUsize = AtomicUsize::new(0);
static CPU_ENTRY: AtomicI32 = AtomicI32::new(-1);
static TEMP_KIND: AtomicU32 = AtomicU32::new(u32::MAX);
static FAULT: AtomicU64 = AtomicU64::new(0);
static SEEDS: Mailbox<(u64, Option<reader::Sample>), 4> = Mailbox::new();
static EVENTS: Mailbox<worker::Event, 128> = Mailbox::new();
static REPLIES: Mailbox<u64, 4> = Mailbox::new();

pub fn set_admission(on: bool) {
    READY.store(on, Ordering::Release);
}
fn admission() -> Admission {
    Admission::new(READY.load(Ordering::Acquire))
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
    ARMED.store(0, Ordering::Release);
    OWN_CPU.store(0, Ordering::Release);
    start();
}
pub fn service() {
    worker::service();
}

/// Roster requests are temporary; its normal completion must not persist the practice attacker as the user's choice.
pub(crate) fn temporary_cpu_selection(kind: u32) -> bool {
    TEMP_KIND.load(Ordering::Acquire) == kind
}

fn post(event: worker::Event, generation: u64) {
    if !EVENTS.post(event) {
        FAULT.store(generation, Ordering::Release);
    }
}

fn revoke() {
    ARMED.store(0, Ordering::Release);
}

#[derive(Clone, Copy)]
pub(crate) struct SeedToken {
    generation: u64,
    threshold: f32,
}

/// Before role_of/CPU.lock in the existing clatter-start wrapper. No plugin lock or allocation.
pub(crate) unsafe fn seed_token(accessor: usize) -> Option<SeedToken> {
    if !READY.load(Ordering::Acquire) || accessor == 0 || P1.load(Ordering::Acquire) != accessor {
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
    d: bool,
) {
    if !READY.load(Ordering::Acquire)
        || seed_token(accessor).is_none_or(|fresh| fresh.generation != token.generation)
    {
        return;
    }
    let sample = admission()
        .with(|| {
            (source == -1 && index == 0 && !c && !d)
                .then(|| reader::sample(accessor, token.threshold))
                .flatten()
                .filter(|s| s.record.source == -1 && s.record.remaining > 0.0)
        })
        .flatten();
    if !SEEDS.post((token.generation, sample)) {
        FAULT.store(token.generation, Ordering::Release);
    }
}

/// Existing CPU hooks and AI epilogue yield only the freshly resolved, selected CPU body.
pub(crate) unsafe fn owns_cpu(accessor: usize) -> bool {
    if !READY.load(Ordering::Acquire)
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
    Restore,
}

struct Game {
    step: Step,
    generation: u64,
    config: Option<Config>,
    scene: Option<Scene>,
    requested: Option<OrdinaryGrab>,
    original: Option<u32>,
    p1: usize,
    cpu: usize,
    clock: Option<Clock>,
    saw_catch: bool,
    restore_requested: bool,
}
impl Game {
    const fn new() -> Self {
        Self {
            step: Step::Off,
            generation: 0,
            config: None,
            scene: None,
            requested: None,
            original: None,
            p1: 0,
            cpu: 0,
            clock: None,
            saw_catch: false,
            restore_requested: false,
        }
    }
    fn step(&mut self, next: Step, text: &'static str) {
        if self.step == next {
            return;
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
    fn retire(&mut self) {
        revoke();
        OWN_CPU.store(0, Ordering::Release);
        TEMP_KIND.store(u32::MAX, Ordering::Release);
        post(worker::Event::Cancel(self.generation), self.generation);
        *self = Self::new();
    }
}

// Only fighter callbacks take this lock. Neither the settings nor diagnostic worker touches it.
static GAME: Mutex<Game> = Mutex::new(Game::new());
const NAMES: [&str; 15] = [
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
];
static IDS: [AtomicI32; 15] = [const { AtomicI32::new(i32::MIN) }; 15];
fn ids() -> Option<[i32; 15]> {
    let mut values = [0; 15];
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
fn captured(status: i32, ids: &[i32; 15]) -> bool {
    ids[2..5].contains(&status) || ids[11..15].contains(&status)
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
    let mut game = GAME.lock();
    let facts = scene::facts();
    if facts.busy {
        revoke();
        OWN_CPU.store(0, Ordering::Release);
        return;
    }
    let Some(where_now) = scene::current() else {
        game.retire();
        return;
    };
    if game.scene.is_some_and(|old| old != where_now) {
        game.retire();
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
        if game.generation != 0 {
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
    let linked = captured(status, &ids);
    if restart {
        revoke();
        post(worker::Event::Cancel(game.generation), game.generation);
        game.config = config;
        game.clock = None;
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
    if FAULT.load(Ordering::Acquire) == game.generation && game.generation != 0 {
        game.fail("Observation queue full · no grade");
    }
    REPLIES.drain(|generation| {
        if generation == game.generation && game.step == Step::Result {
            game.step(
                if linked { Step::Release } else { Step::Choose },
                "Mash out · Setting up",
            );
        }
    });
    SEEDS.drain(|(generation, sample)| {
        if generation != game.generation { return; }
        if matches!(game.step, Step::Held | Step::Result) { game.fail("Capture restarted · no grade"); return; }
        if game.step != Step::Entering { return; }
        let Some(sample) = sample else { game.fail("Native capture seed unavailable · no grade"); return; };
        let cpu_status = natives::status_kind(cpu);
        if !linked || ![ids[1], ids[8], ids[9], ids[10]].contains(&cpu_status) || sample.record.automatic <= 0.0 {
            game.fail("Grab entry unconfirmed · no grade"); return;
        }
        let seed = sample.seed();
        let Ok(clock) = Clock::new(seed.record) else { game.fail("Invalid native capture seed · no grade"); return; };
        let Some(request) = game.requested else { return; };
        let cpu_body = natives::entry_active_body(natives::fighter_entry(cpu_entry));
        if cpu_body == 0 || ((cpu_body + 0xc) as *const u32).read() != request.attacker_kind {
            game.fail("Attacker changed · no grade"); return;
        }
        let key = worker::Key::new(request.attacker_kind, ((fighter + 0xc) as *const u32).read(), natives::damage(p1, 0), seed);
        game.clock = Some(clock);
        game.step = Step::Held;
        post(worker::Event::Begin(generation, key, seed), generation);
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::enqueue_diagnostic(format_args!("[Mashout] seed gen={generation} key={key:?} seed={seed:?} input={:?} history={} current={:#x} previous={:#x} alternate={:#x}", sample.input, sample.history, sample.current, sample.previous, sample.alternate));
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
            game.step = Step::Reload;
            post(
                worker::Event::Setup(game.generation, "Mash out · Loading attacker"),
                game.generation,
            );
            if kind != request.attacker_kind {
                if crate::runtime::roster::character_roster::cpu_reload_ready() {
                    TEMP_KIND.store(request.attacker_kind, Ordering::Release);
                    crate::runtime::roster::character_roster::request_cpu_kind(
                        request.attacker_kind,
                    );
                } else {
                    game.step = Step::Choose;
                }
            }
        }
        Step::Reload => {
            let body = natives::entry_active_body(natives::fighter_entry(cpu_entry));
            let wanted = game.requested.map(|r| r.attacker_kind);
            if body != 0 && wanted == Some(((body + 0xc) as *const u32).read()) {
                game.p1 = p1;
                game.cpu = cpu;
                CPU_ENTRY.store(cpu_entry, Ordering::Release);
                OWN_CPU.store(cpu, Ordering::Release);
                game.step(Step::Place, "Mash out · Setting up Grab");
            }
        }
        Step::Place if !linked && natives::situation_kind(p1) == ids[7] => {
            let position = state_natives::Vector3f {
                x: natives::pos_x(p1),
                y: natives::pos_y(p1),
                z: natives::pos_z(p1),
            };
            let facing = natives::lr(p1);
            if ![position.x, position.y, position.z, facing]
                .iter()
                .all(|v| v.is_finite())
                || facing == 0.0
            {
                game.fail("Position unavailable");
                return;
            }
            state_natives::clear_speed_all(p1);
            state_natives::status_request(p1, ids[0]);
            state_natives::clear_speed_all(cpu);
            // Initial standing-grab fixture: first Eden entry must prove its spacing/floor and real capture.
            state_natives::init_pos(
                cpu,
                &state_natives::Vector3f {
                    x: position.x + 6.0 * facing.signum(),
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
            let Some(threshold) = admission().with(|| reader::threshold(p1)).flatten() else {
                game.fail(UNAVAILABLE_NOTICE);
                return;
            };
            P1.store(p1, Ordering::Release);
            THRESHOLD.store(threshold.to_bits(), Ordering::Release);
            ARMED.store(game.generation, Ordering::Release);
            game.step(Step::Entering, "Mash out · Waiting for Grab");
            state_natives::status_request(cpu, ids[1]);
        }
        Step::Entering => {
            let status = natives::status_kind(cpu);
            game.saw_catch |= [ids[1], ids[8], ids[9], ids[10]].contains(&status);
            if game.saw_catch && status == ids[0] && !linked {
                game.fail("Grab did not capture · reopen menu to retry");
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
            if clock.observe(live.record, live.input, escaped).is_err() {
                #[cfg(feature = "diag")]
                crate::runtime::diag_log::enqueue_diagnostic(format_args!(
                    "[Mashout] mismatch gen={} t={} before={before:?} live={live:?}",
                    game.generation,
                    clock.elapsed()
                ));
                game.fail("Native tick mismatch · no grade");
                return;
            }
            if !linked && !escaped && !clock.exhausted() {
                game.fail("Capture interrupted · no grade");
                return;
            }
            if clock.last() != before || escaped {
                post(
                    worker::Event::Observe(game.generation, clock.elapsed(), clock.exhausted()),
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
            }
            game.clock = Some(clock);
        }
        Step::Result => post(
            worker::Event::Pulse(game.generation, !covered && !natives::is_stop(p1)),
            game.generation,
        ),
        Step::Failed => (),
        Step::Restore => {
            if game.original.is_none() {
                game.retire();
                return;
            }
            if !released {
                return;
            }
            revoke();
            OWN_CPU.store(0, Ordering::Release);
            let Some(original) = game.original else {
                game.retire();
                return;
            };
            let body = natives::entry_active_body(natives::fighter_entry(cpu_entry));
            if body == 0 {
                return;
            }
            if ((body + 0xc) as *const u32).read() == original {
                game.retire();
            } else if !game.restore_requested
                && crate::runtime::roster::character_roster::cpu_reload_ready()
            {
                TEMP_KIND.store(original, Ordering::Release);
                game.restore_requested =
                    crate::runtime::roster::character_roster::request_cpu_kind(original);
            }
        }
    }
}
