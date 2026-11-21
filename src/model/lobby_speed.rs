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
//! - **1 Frame (rc.15, `docs/design/rc15-speed-frame.md`):** the waiting area's battle task entry runs paused
//!   (the register it loaded the paused word into, set for that call only) until a step press of the native button lets
//!   one entry run: one slow step, one scheduler pass. The paused word, the slow table and the rate stay as they are; the
//!   registry walker (room and mode objects) runs while paused and reads the global word, so the room stays at x1.
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
    /// rc.15 1 Frame: the waiting area's task entry runs paused (`entry_holds`) until a step press; no slot, no pass.
    Hold,
}

impl LobbyPlan {
    /// The plan for a lobby speed.
    pub const fn of(speed: Speed) -> LobbyPlan {
        match speed {
            Speed::Half => LobbyPlan::Slot(2),
            Speed::Quarter => LobbyPlan::Slot(4),
            Speed::TwoThirds => LobbyPlan::TwoThirds,
            Speed::OneAndHalf => LobbyPlan::ExtraPass,
            Speed::OneFrame => LobbyPlan::Hold,
            Speed::X1 => LobbyPlan::X1,
        }
    }

    /// The slot this plan holds: its first divisor, and whether the pace rewrites it. 2/3 starts at 1 (no slow): the
    /// pace's first period end asks for 2.
    pub const fn slot(self) -> Option<(u8, bool)> {
        match self {
            LobbyPlan::Slot(divisor) => Some((divisor, false)),
            LobbyPlan::TwoThirds => Some((1, true)),
            LobbyPlan::X1 | LobbyPlan::ExtraPass | LobbyPlan::Hold => None,
        }
    }
}

/// rc.15 1 Frame's step button, as Training's mode object reads it (`speed_frame.step_mask`, `FUN_0177fe20`): a fixed
/// controller-button mask per controller style 1..=7 (`speed_frame.step_buttons`, `main+0x45434f0`): ZL (`0x400`) on
/// styles 1, 2, 5 and 6, the raw L bit (`0x100`) on 3, 4 and the GameCube pad (7), whose L trigger it is.
pub const STEP_BUTTONS: [u32; 7] = [0x400, 0x400, 0x100, 0x100, 0x400, 0x400, 0x100];

/// The step button's raw bits for a controller style (none for a style the table doesn't hold).
pub const fn step_mask(style: u32) -> u32 {
    if style >= 1 && style as usize <= STEP_BUTTONS.len() {
        STEP_BUTTONS[style as usize - 1]
    } else {
        0
    }
}

/// The step button's raw bits Player 1's poll takes out of the controller itself and does not put back after the
/// mapping call: `sets` from all four button sets, `edges` from the two edge sets only. The game's own reader
/// (`FUN_0177e8f0`: Training's step at `.1556a78`/`.1556ae0`) runs after the mapping call, as its pause check does
/// (`.1558e20`), which the chord stage keeps from Plus the same way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepHide {
    pub sets: u32,
    pub edges: u32,
}

/// rc.15 review (tr-primary-2, 2026-11-21 16:55Z): holding the speed chord never steps a frame, so while the hold is
/// open the step button leaves all four sets. The game then rebuilds a press from the cleared current set at every
/// refresh (6e226733), so the poll the hold closes on takes that edge once more: a button still held is a hold, not a
/// press. A chord's own step button (`chord_bits`: Save State's ZL) loses its edges only: the detector needs its
/// current set to keep the chord.
pub const fn step_hide(style: u32, hold_open: bool, hold_closed: bool, chord_bits: u32) -> StepHide {
    let step = step_mask(style);
    StepHide {
        sets: if hold_open { step } else { 0 },
        edges: if hold_open || hold_closed { step } else { step & chord_bits },
    }
}

/// The lobby's step input by the native reader's rules (`.177ec20`: newly pressed is `just_down & current`, held is
/// `current`), on the sets as the poll found them, without what `hide` and the chord stage (`chord_bits`, the raw
/// buttons it keeps from the game on this poll) take: a chord's step button is neither a press nor a hold.
pub const fn step_input(style: u32, current: u32, just_down: u32, hide: StepHide, chord_bits: u32) -> (bool, bool) {
    let mask = step_mask(style);
    (just_down & current & mask & !hide.edges != 0, current & mask & !(hide.sets | chord_bits) != 0)
}

/// rc.15: every row 1 Frame relies on (group `speed_frame`, plus the `lobby_speed` entry rows the hold reads): all must
/// pass at boot or the bind offers no 1 Frame and the hold is not installed.
pub const FRAME_ROWS: [&str; 16] = [
    "speed_frame.entry_hold",
    "speed_frame.entry_skip",
    "speed_frame.entry_skip_last",
    "speed_frame.walker_paused",
    "speed_frame.pause_sync",
    "speed_frame.step_mode",
    "speed_frame.step_input",
    "speed_frame.step_hold",
    "speed_frame.step_repeat",
    "speed_frame.step_run",
    "speed_frame.step_pause",
    "speed_frame.step_mask",
    "speed_frame.step_buttons",
    "lobby_speed.paused_flag",
    "lobby_speed.task_entry",
    "lobby_speed.slow_step",
];

/// From this many polls of the step button held, the world runs while it is held (`speed_frame.step_repeat`:
/// `cmp w8,#18; b.lo` on the mode object's held count).
pub const HOLD_RUN: u8 = 18;

/// rc.15 1 Frame's step input on Player 1's poll, the Training mode object's rule (`speed_frame.step_input`/`step_hold`):
/// a new press lets one task entry run; held, a count goes up each poll and from `HOLD_RUN` on every entry runs; let go,
/// the count goes to 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStep {
    held_for: u8,
    due: bool,
}

impl FrameStep {
    pub const fn new() -> Self {
        Self { held_for: 0, due: false }
    }

    /// One P1 poll. `pressed`/`held`: the step button newly pressed (`just_down & current`, as the native press check)
    /// or held. `blocked`: the speed hold is open, an input owner holds the poll or the waiting-room UI is busy: no step,
    /// and the count starts again.
    pub fn poll(&mut self, pressed: bool, held: bool, blocked: bool) {
        if blocked {
            self.held_for = 0;
        } else if pressed {
            self.held_for = 0;
            self.due = true;
        } else if held {
            self.held_for = self.held_for.saturating_add(1);
            if self.held_for >= HOLD_RUN {
                self.due = true;
            }
        } else {
            self.held_for = 0;
        }
    }

    /// The task entry runs one update unheld if a step is due; the step is used up.
    pub fn take(&mut self) -> bool {
        core::mem::take(&mut self.due)
    }

    /// The due bit of `bits`.
    pub const DUE: u32 = 1 << 8;

    /// The step state as one word (the runtime keeps it in an atomic) and back.
    pub const fn bits(self) -> u32 {
        self.held_for as u32 | if self.due { Self::DUE } else { 0 }
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self { held_for: bits as u8, due: bits & Self::DUE != 0 }
    }
}

/// The battle task entry's call, as the 1 Frame hold (`speed_frame.entry_hold`, `main+0x135ca38`) sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryNow {
    /// The practice is live (armed by Player 1's frame, cleared by its teardown) and its speed is 1 Frame.
    pub armed: bool,
    /// The lobby engine is active and the live mode words are the waiting room.
    pub in_practice: bool,
    /// The entry's object is the waiting area's battle task (vtable `main+0x506d4d8`), not a match's or Training's.
    pub waiting_area: bool,
    /// The paused word the entry just loaded into w22 (`[main+0x52b92e8]`).
    pub paused: u32,
    /// The slow state's current and target divisors (`+4`, `+5`): the hold starts only between slow periods of 1, so a
    /// slot left from the speed before (or a hit's slow) first ends its period and every step is one whole logic frame.
    pub current_n: u8,
    pub target_n: u8,
}

/// Whether the hold may use up a due step on this entry: the waiting area's entry of the live practice at 1 Frame, the
/// game not paused already, and no slow period running.
pub const fn entry_takes_step(now: EntryNow) -> bool {
    now.armed && now.in_practice && now.waiting_area && now.paused == 0 && now.current_n == 1 && now.target_n == 1
}

/// Whether this entry runs paused (w22 = 1): an entry `entry_takes_step` admits with no step due (`step_due` is the
/// step this entry used up). The entry then skips its slow step and runs the scheduler's run-while-paused tasks (the
/// registry walker, so the room objects, among them), as native 1 Frame does, while the paused word itself stays 0.
pub const fn entry_holds(now: EntryNow, step_due: bool) -> bool {
    entry_takes_step(now) && !step_due
}

/// Whether 1 Frame's hold is armed: the practice live at 1 Frame. Every reset (a practice start or end, the session
/// clear, a scene change, a match found) leaves the practice not live or its speed x1, so the hold is never armed in a
/// match.
pub const fn armed(live: bool, wanted: Speed) -> bool {
    live && matches!(LobbyPlan::of(wanted), LobbyPlan::Hold)
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
    #[cfg(test)]
    pub const CLASS_UPDATE_SITE: usize = 0x151b170;
    /// rc.15 `main+0x135ca38`: the hooked instruction in the battle task entry, right after it loads the paused word into
    /// w22 (`speed_frame.entry_hold`).
    #[cfg(test)]
    pub const ENTRY_HOLD_SITE: usize = 0x135ca38;
    /// rc.15 `main+0x506d4d8`: the waiting area's battle task class (the census `qcf-spd05y`, `class_vt`; its `+0x50` is
    /// the update `lobby_speed.class_update` hooks).
    pub const WAITING_AREA_VTABLE: usize = 0x506d4d8;
    /// rc.15 `main+0x45434f0`: the step button table (`speed_frame.step_buttons`).
    #[cfg(test)]
    pub const STEP_BUTTONS_AT: usize = 0x45434f0;
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
#[cfg(test)]
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
