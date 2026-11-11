//! rc.8 R2: the waiting-room practice's 2/3 and x1.5 (`model::lobby_speed`; design `docs/design/rc8-lobby-speed.md`).
//! Switch only.
//!
//! One inline hook, `lobby_speed.class_update` (`main+0x151b170`), in the waiting area's own update `FUN_0151b150`: the
//! battle task entry calls it after this update's slow step and scheduler pass, on the main thread. The callback hands
//! the update to `game_speed::lobby_update`, which paces 2/3's slot (the slot itself rides Player 1's frame, as 1/2 and
//! 1/4) and, for x1.5, runs [`extra_pass`] on every other update. It never changes the context, never writes the update
//! rate words (`[main+0x593b444]`, `[main+0x523d004]`), and takes no lock but `game_speed`'s `try_lock`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use skyline::hooks::InlineCtx;

use crate::model::hook_stats::{HookId, HookScope};
use crate::model::lobby_speed::{self as rules, native, Mem, Plan, Task};

/// The rows passed and the hook is installed.
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// Extra passes run (diag lines read it).
static PASSES: AtomicU32 = AtomicU32::new(0);

/// The seam rows this feature compares before it hooks; `game_speed`'s own rows (the slow table's steps) are its gate.
const ROWS: &[&str] = &[
    "lobby_speed.class_update",
    "lobby_speed.task_entry",
    "lobby_speed.paused_flag",
    "lobby_speed.slow_step",
    "lobby_speed.slow_period",
    "lobby_speed.sched_setup",
    "lobby_speed.sched_phase",
    "lobby_speed.sched_entry",
    "lobby_speed.sched_task",
    "lobby_speed.walker_tick",
];

#[skyline::hook(offset = 0x151b170, inline)]
unsafe fn class_update(_ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::LobbySpeed);
    crate::runtime::features::game_speed::lobby_update();
}

/// Once from `lobby::init`, after `game_speed::init`: compare every row, then hook (only with the speed bind's own rows
/// passed: the paced slot is its slot).
pub(crate) fn init() {
    let speed = crate::runtime::features::game_speed::installed();
    let rows = crate::runtime::seams::seam_install::gate("lobby_speed", ROWS, speed);
    if rows && speed {
        skyline::install_hooks!(class_update);
        INSTALLED.store(true, Ordering::Release);
    }
    crate::engine::compat::logging::diagnostic(format_args!(
        "[TrainingRooms] lobby_speed installed={} rows={rows} game_speed={speed}",
        INSTALLED.load(Ordering::Acquire)
    ));
}

/// Volatile reads of game memory for the scheduler walk (`plan_pass` checks every pointer before it follows it).
struct GameMem;

impl Mem for GameMem {
    fn u64(&self, at: usize) -> u64 {
        // SAFETY: `plan_pass` reads only at addresses it formed from pointers that passed `lobby_speed::object`, inside
        // the scheduler the game's own pass walked on this update (the main thread, between passes).
        unsafe { (at as *const u64).read_volatile() }
    }
    fn u32(&self, at: usize) -> u32 {
        // SAFETY: as `u64`.
        unsafe { (at as *const u32).read_volatile() }
    }
    fn i32(&self, at: usize) -> i32 {
        // SAFETY: as `u64`.
        unsafe { (at as *const i32).read_volatile() }
    }
}

/// The battle scheduler, or 0 (`**(main+0x52c33d8)`).
///
/// # Safety
/// `base` is main's base with `lobby_speed.task_entry` guarded.
pub(crate) unsafe fn scheduler(base: usize) -> usize {
    let global = ((base + native::SCHED_GLOBAL) as *const usize).read_volatile();
    if !rules::object(global) {
        return 0;
    }
    let s = (global as *const usize).read_volatile();
    if rules::object(s) {
        s
    } else {
        0
    }
}

/// The scheduler's current-phase word (`S+0`), `0` without a scheduler.
///
/// # Safety
/// As `scheduler`.
pub(crate) unsafe fn phase_word(s: usize) -> u32 {
    if s == 0 {
        0
    } else {
        (s as *const u32).read_volatile()
    }
}

/// x1.5's extra world step: one more pass of the scheduler's tasks, minus the registry walker (the Quickplay room object
/// and the mode object stay at x1), in the scheduler's own order and with its own phase word, then the slow step's two
/// counters for that step (`lobby_speed.slow_step`: what the native step does at x1). A walk that isn't whole (a pointer
/// that fails its check, more tasks than the buffer, no walker) runs nothing.
///
/// # Safety
/// The waiting area's update on the main thread after this update's scheduler pass (`class_update`), with
/// `lobby_speed::pass_allowed` true (not paused, no slow, the scheduler between passes); `base` is main's base with every
/// `lobby_speed` row guarded.
pub(crate) unsafe fn extra_pass(base: usize, s: usize) -> Plan {
    let mut tasks = [Task::default(); 96];
    let plan = rules::plan_pass(&GameMem, s, base + native::WALKER_TICK, &mut tasks);
    if !plan.runnable() {
        return plan;
    }
    for task in &tasks[..plan.tasks] {
        (s as *mut u32).write_volatile(rules::phase_word(usize::from(task.phase)));
        // SAFETY: the tick the scheduler itself calls for this task (`lobby_speed.sched_task`: `ldr x0,[entry,#0x40];
        // ldr x8,[x0]; ldr x8,[x8,#0x30]; blr x8`), with the task in x0, on the thread its pass runs on.
        let tick: unsafe extern "C" fn(usize) = std::mem::transmute(task.tick);
        tick(task.task);
    }
    (s as *mut u32).write_volatile(u32::MAX);
    let state = base + native::SLOW_STATE;
    for at in [native::LOGIC_FRAMES_AT, native::SECOND_COUNTER_AT] {
        let counter = (state + at) as *mut u32;
        counter.write_volatile(counter.read_volatile().wrapping_add(1));
    }
    PASSES.fetch_add(1, Ordering::Relaxed);
    plan
}

/// Extra passes run since boot (the diag build's lines).
pub(crate) fn passes() -> u32 {
    PASSES.load(Ordering::Relaxed)
}
