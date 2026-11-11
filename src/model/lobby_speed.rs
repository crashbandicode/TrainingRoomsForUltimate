//! rc.8 R2: the waiting-room practice's 2/3 and x1.5 (`docs/design/rc8-lobby-speed.md`; QCF `RESEARCH-rc8-speed.md`
//! section 9). Pure; `runtime::lobby::lobby_speed` and `runtime::features::game_speed` read and write the game.
//!
//! The lobby can't use Training's native 2/3 and x1.5: both raise the game's update rate (`[main+0x593b444]`,
//! `[main+0x523d004]`), which the Quickplay room's own timers count, so the room would run faster for the peer too
//! (Eden `qcf-spd04y`). The bind never writes those two words in the lobby. Instead:
//! - **2/3:** one slot of the slow table, whose divisor alternates 2 and 1 at the slow periods' ends: periods of 1 and 2
//!   updates, two logic frames per three updates. The update count, the rate and the room are untouched (`qcf-spd04y`).
//! - **x1.5:** on every other update, after the update's own scheduler pass, one more pass of the battle scheduler's
//!   tasks, minus the registry walker (which ticks the Quickplay room object and the mode object), plus the slow step's
//!   own two counter increments for that extra world step. The room stays at x1 (`qcf-spd05y`: 58 frames between the
//!   host's room hits against 59 at x1, the peer's match arrival unchanged).
//!
//! Facts (13.0.5, the owned main ELF; seam group `lobby_speed`, decoded by `lobby_speed_rows_tests`):
//! - The battle task entry `FUN_0135ca00` (every battle task class's update) runs the slow step, then the scheduler
//!   `FUN_0137b3f0(**(main+0x52c33d8), paused)`, then the class's own update (vtable `+0x50`); the waiting area's is
//!   `FUN_0151b150` (class `0x506d4d8`), which the bind hooks after its prologue.
//! - The slow state at `main+0x52c5184`: `+0` logic frames, `+4` the current divisor N, `+5` the target, `+6` the
//!   sub-counter, `+8` a second counter. With N = 1 each update ends a period (both counters += 1, the target latched);
//!   otherwise the sub-counter counts to N, and the period ends on the update that leaves it at 0.
//! - The scheduler: phases 0..=9, each a libc++ deque of 16-byte `shared_ptr<entry>` at `S + 0x30*phase` (begin `+0x60`,
//!   end `+0x68`, start `+0x78`, size `+0x80`; 256 elements per block). An entry's group is the `i32` at `+8` (masked
//!   while `S.u32[1 + group]` is non-zero, groups up to 9), its task at `+0x40`, the task's tick at its vtable's `+0x30`.
//!   The current phase is the `u32` at `S+0`: -1 for phases 0 and 9 and between passes, the phase number otherwise.
//! - Paused (`[main+0x52b92e8]` non-zero), the scheduler runs only the tasks marked to run while paused.

use super::game_speed::Speed;

/// What the waiting-room practice does for a speed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LobbyPlan {
    /// Nothing: no slot, no extra pass.
    X1,
    /// A slot with this constant divisor (1/2, 1/4).
    Slot(u8),
    /// A slot whose divisor the pace alternates (`paced_divisor`).
    TwoThirds,
    /// The extra scheduler pass on every other update.
    ExtraPass,
}

impl LobbyPlan {
    /// The plan for a lobby speed; a speed the lobby doesn't offer (1 Frame) is x1.
    pub const fn of(speed: Speed) -> LobbyPlan {
        match speed {
            Speed::Half => LobbyPlan::Slot(2),
            Speed::Quarter => LobbyPlan::Slot(4),
            Speed::TwoThirds => LobbyPlan::TwoThirds,
            Speed::OneAndHalf => LobbyPlan::ExtraPass,
            Speed::X1 | Speed::OneFrame => LobbyPlan::X1,
        }
    }

    /// The slot this plan holds: its first divisor, and whether the pace rewrites it. 2/3 starts at 1 (no slow): the
    /// pace's first period end asks for 2.
    pub const fn slot(self) -> Option<(u8, bool)> {
        match self {
            LobbyPlan::Slot(divisor) => Some((divisor, false)),
            LobbyPlan::TwoThirds => Some((1, true)),
            LobbyPlan::X1 | LobbyPlan::ExtraPass => None,
        }
    }
}

/// 2/3's pace, read on the waiting area's update after its slow step: on an update that ended a period (`sub` 0), the
/// divisor for the period after the next one (the slow step latches the target only at a period's end): 2 after a
/// period of 1 (`current_n` 1), 1 after a period of 2. `None` inside a period.
pub const fn paced_divisor(current_n: u8, sub: u8) -> Option<u8> {
    if sub != 0 {
        return None;
    }
    Some(if current_n <= 1 { 2 } else { 1 })
}

/// x1.5's cadence: one extra pass on every other update.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cadence {
    odd: bool,
}

impl Cadence {
    pub const fn new() -> Self {
        Self { odd: false }
    }

    /// One waiting-area update that may run a pass: true on every second call.
    pub fn due(&mut self) -> bool {
        self.odd = !self.odd;
        !self.odd
    }

    pub fn reset(&mut self) {
        self.odd = false;
    }
}

/// The game's state on the waiting area's update, as the hook reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassNow {
    /// The practice's session is live (armed by Player 1's frame, cleared by the practice's teardown).
    pub live: bool,
    /// The lobby engine is active and the live mode words are the waiting room.
    pub in_practice: bool,
    /// The native waiting-room UI is busy.
    pub ui_busy: bool,
    /// `[main+0x52b92e8]`.
    pub paused: u32,
    /// The slow state's current and target divisors (`+4`, `+5`).
    pub current_n: u8,
    pub target_n: u8,
    /// The scheduler's current-phase word (`S+0`); -1 between passes.
    pub phase_word: u32,
}

/// Whether x1.5's extra pass may run on this update: the live practice, its UI free, the game not paused, no slow motion
/// of any kind (a slot, a hit's slow), and the scheduler between passes.
pub const fn pass_allowed(now: PassNow) -> bool {
    now.live && now.in_practice && !now.ui_busy && now.paused == 0 && now.current_n == 1 && now.target_n == 1 && now.phase_word == u32::MAX
}

/// The native addresses the runtime forms, each decoded from the guarded words by the host tests.
pub mod native {
    /// `main+0x52c33d8`: the word holding the scheduler's global (`lobby_speed.task_entry`); the scheduler is `**word`.
    pub const SCHED_GLOBAL: usize = 0x52c33d8;
    /// `main+0x52b92e8`: the paused word (`lobby_speed.paused_flag`).
    pub const PAUSED: usize = 0x52b92e8;
    /// `main+0x52c5184`: the slow state (`lobby_speed.slow_step`).
    pub const SLOW_STATE: usize = 0x52c5184;
    pub const LOGIC_FRAMES_AT: usize = 0;
    pub const CURRENT_N_AT: usize = 4;
    pub const TARGET_N_AT: usize = 5;
    pub const SUB_AT: usize = 6;
    pub const SECOND_COUNTER_AT: usize = 8;
    /// `main+0x1358970`: the registry walker task's tick (`lobby_speed.walker_tick`), the one task the pass skips.
    pub const WALKER_TICK: usize = 0x1358970;
    /// `main+0x151b170`: the hooked instruction in the waiting area's update (`lobby_speed.class_update`).
    pub const CLASS_UPDATE_SITE: usize = 0x151b170;
}

/// The scheduler's layout (`lobby_speed.sched_*`).
pub mod sched {
    pub const PHASES: usize = 10;
    pub const STRIDE: usize = 0x30;
    pub const BEGIN_AT: usize = 0x60;
    pub const END_AT: usize = 0x68;
    pub const START_AT: usize = 0x78;
    pub const SIZE_AT: usize = 0x80;
    pub const PER_BLOCK: usize = 256;
    pub const ELEMENT: usize = 16;
    pub const ENTRY_GROUP_AT: usize = 8;
    pub const ENTRY_TASK_AT: usize = 0x40;
    pub const TICK_SLOT: usize = 0x30;
    /// Masks: `S.u32[1 + group]` for groups up to 9.
    pub const MASKS_AT: usize = 4;
    pub const MASKED_MAX: i32 = 9;
    /// No walk reads more elements than this per phase (the census had 56 in all).
    pub const MAX_PER_PHASE: usize = 256;
    /// The pass's task buffer: the most tasks one phase may hand it.
    pub const PHASE_TASKS: usize = 96;
}

/// The phase word the scheduler stores before a phase's tasks: -1 for phases 0 and 9, the number otherwise.
pub const fn phase_word(phase: usize) -> u32 {
    if phase == 0 || phase >= 9 {
        u32::MAX
    } else {
        phase as u32
    }
}

/// Reads of game memory: a fake in tests, volatile reads on the Switch.
pub trait Mem {
    fn u64(&self, at: usize) -> u64;
    fn u32(&self, at: usize) -> u32;
    fn i32(&self, at: usize) -> i32;
}

/// The end of the address space a process maps (as `native_fields::USER_ADDRESS_END`).
#[cfg(target_os = "switch")]
const ADDRESS_END: usize = 1 << 39;
#[cfg(not(target_os = "switch"))]
const ADDRESS_END: usize = 1 << 48;

/// A structural pointer check: non-null, 8-byte aligned, inside the address space.
pub const fn object(address: usize) -> bool {
    address != 0 && address.is_multiple_of(8) && address < ADDRESS_END
}

/// One task of the extra pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Task {
    pub phase: u8,
    pub task: usize,
    pub tick: usize,
}

impl Task {
    pub const EMPTY: Task = Task { phase: 0, task: 0, tick: 0 };
}

/// What a walk found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// Tasks to tick, in the scheduler's order (written to `out` when the walk has one).
    pub tasks: usize,
    /// Tasks whose tick is the skipped one (the registry walker).
    pub skipped: usize,
    /// Masked entries.
    pub masked: usize,
    /// Entries, tasks or vtables that failed the pointer check, and elements past `out` or `MAX_PER_PHASE`: the walk
    /// was not the scheduler's own, so the pass doesn't run.
    pub broken: usize,
    /// The most tasks to tick in one phase.
    pub widest: usize,
}

impl Plan {
    /// The pass runs only on a whole, clean walk with the walker found once, and no phase wider than the buffer.
    pub const fn runnable(&self) -> bool {
        self.broken == 0 && self.skipped == 1 && self.tasks > 0 && self.widest <= sched::PHASE_TASKS
    }
}

/// Walk the phases `phases` of the scheduler `s` as `FUN_0137b3f0` does unpaused (every element of each deque, masked
/// groups left out), minus the task whose tick is `skip_tick`. With `out`, list each task to tick there (an element
/// past its end is broken); without, only count. Nothing is called.
pub fn plan_phases(
    mem: &impl Mem,
    s: usize,
    phases: core::ops::Range<usize>,
    skip_tick: usize,
    mut out: Option<&mut [Task]>,
) -> Plan {
    let mut plan = Plan::default();
    if !object(s) {
        plan.broken += 1;
        return plan;
    }
    for phase in phases.start..phases.end.min(sched::PHASES) {
        let before = plan.tasks;
        walk_phase(mem, s, phase, skip_tick, out.as_deref_mut(), &mut plan);
        plan.widest = plan.widest.max(plan.tasks - before);
    }
    plan
}

/// The whole walk (phases 0..=9), listed into `out`.
pub fn plan_pass(mem: &impl Mem, s: usize, skip_tick: usize, out: &mut [Task]) -> Plan {
    plan_phases(mem, s, 0..sched::PHASES, skip_tick, Some(out))
}

fn walk_phase(mem: &impl Mem, s: usize, phase: usize, skip_tick: usize, mut out: Option<&mut [Task]>, plan: &mut Plan) {
    let deque = s + sched::STRIDE * phase;
    let begin = mem.u64(deque + sched::BEGIN_AT) as usize;
    let end = mem.u64(deque + sched::END_AT) as usize;
    if begin == end {
        return;
    }
    let start = mem.u64(deque + sched::START_AT) as usize;
    let size = mem.u64(deque + sched::SIZE_AT) as usize;
    if !object(begin) || size > sched::MAX_PER_PHASE {
        plan.broken += 1;
        return;
    }
    for index in start..start + size {
        let block = mem.u64(begin + (index / sched::PER_BLOCK) * 8) as usize;
        if !object(block) {
            plan.broken += 1;
            return;
        }
        let entry = mem.u64(block + (index % sched::PER_BLOCK) * sched::ELEMENT) as usize;
        if !object(entry) {
            plan.broken += 1;
            continue;
        }
        let group = mem.i32(entry + sched::ENTRY_GROUP_AT);
        if group <= sched::MASKED_MAX {
            let mask_at = (s + sched::MASKS_AT).wrapping_add((group as isize * 4) as usize);
            if mem.u32(mask_at) != 0 {
                plan.masked += 1;
                continue;
            }
        }
        let task = mem.u64(entry + sched::ENTRY_TASK_AT) as usize;
        let vtable = if object(task) { mem.u64(task) as usize } else { 0 };
        let tick = if object(vtable) { mem.u64(vtable + sched::TICK_SLOT) as usize } else { 0 };
        if tick == 0 || !tick.is_multiple_of(4) {
            plan.broken += 1;
            continue;
        }
        if tick == skip_tick {
            plan.skipped += 1;
            continue;
        }
        if let Some(out) = out.as_deref_mut() {
            if plan.tasks == out.len() {
                plan.broken += 1;
                continue;
            }
            out[plan.tasks] = Task { phase: phase as u8, task, tick };
        }
        plan.tasks += 1;
    }
}

/// How an extra pass went.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pass {
    /// The whole walk, checked before any tick: nothing runs unless it is `runnable`.
    pub survey: Plan,
    /// Tasks ticked.
    pub ticked: usize,
    /// Every phase ran. The slow counters advance only then.
    pub whole: bool,
}

/// x1.5's extra world step, the scheduler's way (the docs lane's R2 review, finding 1). The whole walk is checked first,
/// without a buffer; then each phase is read again right before its ticks, as `FUN_0137b3f0` does (a tick may change a
/// later phase's deque), and its tasks go to `tick` in order. The pass stops, not whole, when a phase's second read is
/// not clean or `live` turns false between two ticks (the practice's teardown began inside a tick). `buf` holds one
/// phase.
pub fn run_pass(
    mem: &impl Mem,
    s: usize,
    skip_tick: usize,
    buf: &mut [Task],
    live: impl Fn() -> bool,
    mut tick: impl FnMut(Task),
) -> Pass {
    let survey = plan_phases(mem, s, 0..sched::PHASES, skip_tick, None);
    let mut pass = Pass { survey, ticked: 0, whole: false };
    if !survey.runnable() || survey.widest > buf.len() {
        return pass;
    }
    for phase in 0..sched::PHASES {
        let plan = plan_phases(mem, s, phase..phase + 1, skip_tick, Some(&mut *buf));
        if plan.broken != 0 {
            return pass;
        }
        for task in &buf[..plan.tasks] {
            if !live() {
                return pass;
            }
            tick(*task);
            pass.ticked += 1;
        }
    }
    pass.whole = true;
    pass
}

#[cfg(test)]
#[path = "lobby_speed_tests.rs"]
mod tests;
