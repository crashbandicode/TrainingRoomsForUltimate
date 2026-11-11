//! rc.8 R2: the lobby's 2/3 pace (against a model of the game's slow step), x1.5's cadence and gate, and the scheduler
//! walk (against a fake scheduler laid out as `FUN_0137b3f0` reads it).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use super::super::game_speed::{fresh_table, table, Held, LobbyNow, LobbySlot, LobbyStep, Speed};
use super::*;

// --- plans -----------------------------------------------------------------------------------------------------------

#[test]
fn each_lobby_speed_has_its_plan_and_none_raises_the_update_rate() {
    assert_eq!(LobbyPlan::of(Speed::X1), LobbyPlan::X1);
    assert_eq!(LobbyPlan::of(Speed::Half), LobbyPlan::Slot(2));
    assert_eq!(LobbyPlan::of(Speed::Quarter), LobbyPlan::Slot(4));
    assert_eq!(LobbyPlan::of(Speed::TwoThirds), LobbyPlan::TwoThirds);
    assert_eq!(LobbyPlan::of(Speed::OneAndHalf), LobbyPlan::ExtraPass);
    assert_eq!(LobbyPlan::of(Speed::OneFrame), LobbyPlan::X1, "1 Frame is not a lobby speed");
    assert_eq!(LobbyPlan::Slot(2).slot(), Some((2, false)));
    assert_eq!(LobbyPlan::TwoThirds.slot(), Some((1, true)), "2/3 starts with no slow; the pace asks for 2");
    assert_eq!(LobbyPlan::ExtraPass.slot(), None, "x1.5 holds no slot");
    assert_eq!(LobbyPlan::X1.slot(), None);
}

// --- 2/3: the pace against the game's slow step ----------------------------------------------------------------------

/// `FUN_0135ca00`'s slow step (`lobby_speed.slow_step`/`slow_period`) and `FUN_014a2cd0`'s max divisor, over a table.
struct Slow {
    table: [u8; table::SIZE],
    frames: u32,
    current: u8,
    target: u8,
    sub: u8,
}

impl Slow {
    fn new() -> Self {
        Self { table: fresh_table(), frames: 0, current: 1, target: 1, sub: 0 }
    }

    fn update(&mut self) {
        if self.table[table::DIRTY_AT] != 0 {
            let max = (0..table::SLOTS)
                .filter(|slot| self.table[table::SLOTS_AT + 2 * slot] != 0)
                .map(|slot| self.table[table::SLOTS_AT + 2 * slot + 1])
                .max()
                .unwrap_or(0);
            self.target = max.clamp(1, 100);
            self.table[table::DIRTY_AT] = 0;
        }
        if self.current == 1 {
            self.frames += 1;
            self.current = self.target;
            self.sub = 0;
        } else {
            self.sub += 1;
            if self.sub >= self.current {
                self.frames += 1;
                self.sub = 0;
                self.current = self.target;
            }
        }
    }
}

const TABLE: usize = 0x1_0000_1000;
const LIVE: LobbyNow = LobbyNow { in_practice: true, ui_busy: false, table: TABLE };

/// Updates where a logic frame ended, then the period lengths between them.
fn run(slow: &mut Slow, keeper: &mut LobbySlot, speed: Speed, updates: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for update in 0..updates {
        // Player 1's frame (the slot) runs inside the world's tasks, after the slow step; the pace on the class update.
        let before = slow.frames;
        slow.update();
        if slow.frames != before {
            ends.push(update);
            keeper.step(speed, LIVE, Some(&mut slow.table));
        }
        keeper.pace(slow.current, slow.sub, TABLE, Some(&mut slow.table));
    }
    ends.windows(2).map(|w| w[1] - w[0]).collect()
}

#[test]
fn two_thirds_paces_periods_of_one_and_two_updates() {
    let mut slow = Slow::new();
    let mut keeper = LobbySlot::new();
    let periods = run(&mut slow, &mut keeper, Speed::TwoThirds, 600);
    // After the first two periods the lengths alternate 1, 2: two logic frames per three updates.
    let steady = &periods[2..];
    assert!(steady.windows(2).all(|w| w[0] + w[1] == 3 && w[0] != w[1]), "{steady:?}");
    let frames = slow.frames as f64 / 600.0;
    assert!((frames - 2.0 / 3.0).abs() < 0.01, "{frames}");
    // The plugin holds exactly one slot, its divisor 1 or 2.
    let held = keeper.held().unwrap();
    let in_use: Vec<u8> = (0..table::SLOTS).filter(|s| slow.table[0x20 + 2 * s] == 1).map(|s| slow.table[0x21 + 2 * s]).collect();
    assert_eq!(in_use, [held.divisor]);
    assert!(held.divisor == 1 || held.divisor == 2);
}

#[test]
fn half_and_quarter_are_not_paced_and_two_thirds_yields_to_a_longer_slow() {
    for (speed, n) in [(Speed::Half, 2usize), (Speed::Quarter, 4)] {
        let mut slow = Slow::new();
        let mut keeper = LobbySlot::new();
        let periods = run(&mut slow, &mut keeper, speed, 400);
        assert!(periods[2..].iter().all(|p| *p == n), "{speed:?}: {periods:?}");
        assert!(matches!(keeper.pace(1, 0, TABLE, Some(&mut slow.table)), LobbyStep::Nothing));
    }
    // A hit's slow (the game's own slot, divisor 4) beside the paced slot: the largest divisor wins, as natively.
    let mut slow = Slow::new();
    slow.table[0x20] = 1;
    slow.table[0x21] = 4;
    let mut keeper = LobbySlot::new();
    let periods = run(&mut slow, &mut keeper, Speed::TwoThirds, 400);
    assert!(periods[2..].iter().all(|p| *p == 4), "{periods:?}");
}

#[test]
fn the_paced_slot_is_kept_through_its_divisors_and_freed_with_the_one_it_holds() {
    let mut t = fresh_table();
    let mut keeper = LobbySlot::new();
    let taken = Held { table: TABLE, slot: 0, divisor: 1 };
    assert_eq!(keeper.step(Speed::TwoThirds, LIVE, Some(&mut t)), LobbyStep::Taken(taken));
    assert_eq!((t[0x20], t[0x21], t[0x40]), (1, 1, 1), "taken as the game takes, with divisor 1");
    // Inside a period nothing is written; at a period's end after a period of 1, 2 is asked for.
    t[0x40] = 0;
    assert_eq!(keeper.pace(1, 1, TABLE, Some(&mut t)), LobbyStep::Nothing);
    let two = Held { divisor: 2, ..taken };
    assert_eq!(keeper.pace(1, 0, TABLE, Some(&mut t)), LobbyStep::Paced(two));
    assert_eq!((t[0x20], t[0x21], t[0x40]), (1, 2, 1), "the divisor rewritten, dirty set");
    assert_eq!(keeper.pace(1, 0, TABLE, Some(&mut t)), LobbyStep::Nothing, "already 2");
    // `step` keeps the paced slot whatever divisor it holds.
    assert_eq!(keeper.step(Speed::TwoThirds, LIVE, Some(&mut t)), LobbyStep::Nothing);
    // Another table, or a slot the game no longer holds as ours: no write.
    let snapshot = t;
    assert_eq!(keeper.pace(2, 0, TABLE + 0x100, Some(&mut t)), LobbyStep::Nothing);
    t[0x21] = 3;
    let changed = t;
    assert_eq!(keeper.pace(2, 0, TABLE, Some(&mut t)), LobbyStep::Nothing);
    assert_eq!(t, changed);
    t = snapshot;
    // A change of speed frees it with the divisor it holds now; 1/2 then takes a plain slot.
    assert_eq!(keeper.step(Speed::Half, LIVE, Some(&mut t)), LobbyStep::Freed(two));
    assert_eq!(t[0x20..0x22], [0, 1]);
    let half = Held { table: TABLE, slot: 0, divisor: 2 };
    assert_eq!(keeper.step(Speed::Half, LIVE, Some(&mut t)), LobbyStep::Taken(half));
    assert_eq!(keeper.pace(1, 0, TABLE, Some(&mut t)), LobbyStep::Nothing, "1/2 is never paced");
    // 1/2 to 2/3 (both hold a slot of divisor 2 at times): released, then taken paced.
    assert_eq!(keeper.step(Speed::TwoThirds, LIVE, Some(&mut t)), LobbyStep::Freed(half));
    assert_eq!(keeper.step(Speed::TwoThirds, LIVE, Some(&mut t)), LobbyStep::Taken(taken));
}

/// R9 with R2: a match found from the waiting room at 2/3 (no busy edge; the practice P1's destroy is the session clear):
/// the release frees the slot with the divisor the pace left, and the match's table holds no slot of the plugin.
#[test]
fn match_found_at_two_thirds_frees_the_paced_slot_before_the_drop() {
    for divisor_at_the_drop in [1u8, 2] {
        let mut t = fresh_table();
        t[0x20] = 1;
        t[0x21] = 3;
        let mut keeper = LobbySlot::new();
        keeper.step(Speed::TwoThirds, LIVE, Some(&mut t));
        if divisor_at_the_drop == 2 {
            keeper.pace(1, 0, TABLE, Some(&mut t));
        }
        let held = keeper.held().unwrap();
        assert_eq!(held.divisor, divisor_at_the_drop);
        assert_eq!(keeper.release(TABLE, Some(&mut t)), LobbyStep::Freed(held));
        assert_eq!((t[0x22], t[0x23]), (0, 1));
        assert_eq!((t[0x20], t[0x21]), (1, 3), "the game's slot is untouched");
        assert_eq!(keeper.pace(1, 0, TABLE, Some(&mut t)), LobbyStep::Nothing, "nothing left to pace");
        let matched = LobbyNow { in_practice: false, ..LIVE };
        assert_eq!(keeper.step(Speed::TwoThirds, matched, Some(&mut t)), LobbyStep::Nothing, "never taken outside");
    }
}

#[test]
fn paced_divisor_alternates_at_period_ends_only() {
    assert_eq!(paced_divisor(1, 0), Some(2));
    assert_eq!(paced_divisor(2, 0), Some(1));
    assert_eq!(paced_divisor(4, 0), Some(1));
    assert_eq!(paced_divisor(2, 1), None);
}

// --- x1.5: cadence and gate ------------------------------------------------------------------------------------------

#[test]
fn the_extra_pass_runs_on_every_other_update_and_restarts_after_a_reset() {
    let mut cadence = Cadence::new();
    let due: Vec<bool> = (0..6).map(|_| cadence.due()).collect();
    assert_eq!(due, [false, true, false, true, false, true]);
    cadence.due();
    cadence.reset();
    assert!(!cadence.due());
    assert!(cadence.due());
}

#[test]
fn the_extra_pass_needs_the_live_practice_unpaused_with_no_slow_and_between_passes() {
    let ok = PassNow { live: true, in_practice: true, ui_busy: false, paused: 0, current_n: 1, target_n: 1, phase_word: u32::MAX };
    assert!(pass_allowed(ok));
    for (now, why) in [
        (PassNow { live: false, ..ok }, "the practice's teardown began (session clear)"),
        (PassNow { in_practice: false, ..ok }, "not the waiting room's practice"),
        (PassNow { ui_busy: true, ..ok }, "the waiting-room UI is busy"),
        (PassNow { paused: 1, ..ok }, "paused"),
        (PassNow { current_n: 2, ..ok }, "a slow period"),
        (PassNow { target_n: 4, ..ok }, "a slow is coming (a hit, a slot not yet freed)"),
        (PassNow { phase_word: 3, ..ok }, "inside a scheduler pass"),
    ] {
        assert!(!pass_allowed(now), "{why}");
    }
}

// --- the scheduler walk ----------------------------------------------------------------------------------------------

#[derive(Default)]
struct Fake {
    bytes: RefCell<HashMap<usize, u8>>,
}

impl Fake {
    fn put(&self, at: usize, value: u64, len: usize) {
        let mut bytes = self.bytes.borrow_mut();
        for i in 0..len {
            bytes.insert(at + i, (value >> (8 * i)) as u8);
        }
    }
    fn get(&self, at: usize, len: usize) -> u64 {
        let bytes = self.bytes.borrow();
        (0..len).fold(0, |v, i| v | u64::from(*bytes.get(&(at + i)).unwrap_or(&0)) << (8 * i))
    }
}

impl Mem for Fake {
    fn u64(&self, at: usize) -> u64 {
        self.get(at, 8)
    }
    fn u32(&self, at: usize) -> u32 {
        self.get(at, 4) as u32
    }
    fn i32(&self, at: usize) -> i32 {
        self.get(at, 4) as u32 as i32
    }
}

const S: usize = 0x10_0000;
const WALKER: usize = 0x7135_8970;

/// A scheduler with `phases[p]` = the (group, tick) of each entry, phase 3's deque starting at element 254 so it spans two
/// blocks. Each entry, task and vtable lives at its own address.
fn scheduler(phases: &[&[(i32, usize)]], masks: &[(usize, u32)]) -> Fake {
    let m = Fake::default();
    let mut next = 0x20_0000usize;
    let mut alloc = |size: usize| {
        let at = next;
        next += (size + 0xf) & !0xf;
        at
    };
    for &(group, mask) in masks {
        m.put(S + 4 + 4 * group, u64::from(mask), 4);
    }
    for (phase, entries) in phases.iter().enumerate() {
        let deque = S + 0x30 * phase;
        if entries.is_empty() {
            continue;
        }
        let start = if phase == 3 { 254 } else { 0 };
        let blocks = (start + entries.len()).div_ceil(256);
        let map = alloc(8 * blocks);
        for b in 0..blocks {
            let block = alloc(0x1000);
            m.put(map + 8 * b, block as u64, 8);
        }
        m.put(deque + 0x60, map as u64, 8);
        m.put(deque + 0x68, (map + 8 * blocks) as u64, 8);
        m.put(deque + 0x78, start as u64, 8);
        m.put(deque + 0x80, entries.len() as u64, 8);
        for (i, &(group, tick)) in entries.iter().enumerate() {
            let index = start + i;
            let block = m.u64(map + 8 * (index / 256)) as usize;
            let (entry, task, vtable) = (alloc(0x50), alloc(0x10), alloc(0x40));
            m.put(block + 16 * (index % 256), entry as u64, 8);
            m.put(entry + 8, group as u32 as u64, 4);
            m.put(entry + 0x40, task as u64, 8);
            m.put(task, vtable as u64, 8);
            m.put(vtable + 0x30, tick as u64, 8);
        }
    }
    m
}

const fn tick(n: usize) -> usize {
    0x71_0000_0000 + 0x100 * n
}

/// Ten tasks over phases 0, 1, 3, 5 and 9, the walker in phase 1; with groups 2 and 12 masked (`scheduler(&SAMPLE,
/// MASKS)`), phase 3 lists three.
const SAMPLE: [&[(i32, usize)]; 10] = [
    &[(9, tick(1)), (0, tick(2))],
    &[(8, tick(3)), (8, WALKER), (4, tick(4))],
    &[],
    &[(3, tick(5)), (12, tick(6)), (2, tick(7)), (0, tick(8))],
    &[],
    &[(5, tick(9))],
    &[],
    &[],
    &[],
    &[(6, tick(10))],
];
const MASKS: &[(usize, u32)] = &[(2, 1), (12, 1)];

/// The entry, task and vtable of element `i` of phase `phase` in a `scheduler` fake.
fn element(m: &Fake, phase: usize, i: usize) -> (usize, usize, usize) {
    let deque = S + 0x30 * phase;
    let (map, index) = (m.u64(deque + 0x60) as usize, m.u64(deque + 0x78) as usize + i);
    let block = m.u64(map + 8 * (index / 256)) as usize;
    let entry = m.u64(block + 16 * (index % 256)) as usize;
    let task = m.u64(entry + 0x40) as usize;
    (entry, task, m.u64(task) as usize)
}

#[test]
fn the_walk_follows_the_scheduler_and_leaves_out_the_walker_and_masked_groups() {
    let phases: [&[(i32, usize)]; 10] = [
        &[(9, tick(1)), (0, tick(2))],
        &[(8, tick(3)), (8, WALKER), (4, tick(4))],
        &[],
        &[(3, tick(5)), (12, tick(6)), (2, tick(7)), (0, tick(8))],
        &[],
        &[(5, tick(9))],
        &[],
        &[],
        &[],
        &[(6, tick(10))],
    ];
    // Group 2 masked (a slow's in-between update would mask world groups); group 12 has no mask word (> 9).
    let m = scheduler(&phases, &[(2, 1), (12, 1)]);
    let mut out = [Task::default(); 16];
    let plan = plan_pass(&m, S, WALKER, &mut out);
    assert_eq!(plan, Plan { tasks: 9, skipped: 1, masked: 1, broken: 0, widest: 3 });
    assert!(plan.runnable());
    let got: Vec<(u8, usize)> = out[..plan.tasks].iter().map(|t| (t.phase, t.tick)).collect();
    assert_eq!(
        got,
        [(0, tick(1)), (0, tick(2)), (1, tick(3)), (1, tick(4)), (3, tick(5)), (3, tick(6)), (3, tick(8)), (5, tick(9)), (9, tick(10))],
        "in the scheduler's order, across phase 3's two blocks"
    );
    for t in &out[..plan.tasks] {
        assert_eq!(m.u64(m.u64(t.task) as usize + 0x30) as usize, t.tick, "the task's own tick");
    }
}

#[test]
fn a_walk_that_is_not_whole_or_has_no_walker_is_not_run() {
    let phases: [&[(i32, usize)]; 10] = [&[(0, tick(1)), (8, WALKER)], &[], &[], &[], &[], &[], &[], &[], &[], &[]];
    let m = scheduler(&phases, &[]);
    // No room for the tasks.
    let mut none: [Task; 0] = [];
    let plan = plan_pass(&m, S, WALKER, &mut none);
    assert_eq!((plan.tasks, plan.broken), (0, 1));
    assert!(!plan.runnable());
    // The walker not found (another tick skipped).
    let mut out = [Task::default(); 4];
    assert!(!plan_pass(&m, S, tick(99), &mut out).runnable());
    // No scheduler.
    assert_eq!(plan_pass(&m, 0, WALKER, &mut out), Plan { tasks: 0, skipped: 0, masked: 0, broken: 1, widest: 0 });
    // A null entry, a task without a vtable, a tick that is not code: broken, not run.
    let broken = scheduler(&phases, &[]);
    let block = broken.u64(broken.u64(S + 0x60) as usize) as usize;
    broken.put(block, 0, 8);
    let plan = plan_pass(&broken, S, WALKER, &mut out);
    assert_eq!((plan.broken, plan.runnable()), (1, false));
    let broken = scheduler(&phases, &[]);
    let entry = broken.u64(block) as usize;
    let task = broken.u64(entry + 0x40) as usize;
    broken.put(task, 0, 8);
    assert!(!plan_pass(&broken, S, WALKER, &mut out).runnable());
    let broken = scheduler(&phases, &[]);
    let vtable = broken.u64(task) as usize;
    broken.put(vtable + 0x30, 0x71_0000_0002, 8);
    assert!(!plan_pass(&broken, S, WALKER, &mut out).runnable());
    // A deque larger than any the scheduler holds.
    let huge = scheduler(&phases, &[]);
    huge.put(S + 0x80, 100_000, 8);
    assert!(!plan_pass(&huge, S, WALKER, &mut out).runnable());
}

// --- the pass (the docs lane's R2 review, findings 1 and 2) ----------------------------------------------------------

#[test]
fn counting_and_listing_walks_agree_and_a_phase_wider_than_the_buffer_is_not_run() {
    let m = scheduler(&SAMPLE, MASKS);
    let mut out = [Task::default(); 16];
    assert_eq!(plan_phases(&m, S, 0..sched::PHASES, WALKER, None), plan_pass(&m, S, WALKER, &mut out));
    assert_eq!(plan_phases(&m, S, 3..4, WALKER, None), Plan { tasks: 3, skipped: 0, masked: 1, broken: 0, widest: 3 });
    let wide: Vec<(i32, usize)> = (0..=sched::PHASE_TASKS).map(|n| (0, tick(100 + n))).collect();
    let phases: [&[(i32, usize)]; 10] = [&[(8, WALKER)], &wide, &[], &[], &[], &[], &[], &[], &[], &[]];
    let survey = plan_phases(&scheduler(&phases, &[]), S, 0..sched::PHASES, WALKER, None);
    assert_eq!((survey.broken, survey.widest), (0, sched::PHASE_TASKS + 1));
    assert!(!survey.runnable(), "one phase holds more tasks than the pass's buffer");
}

#[test]
fn the_pass_ticks_every_task_in_the_schedulers_order_and_is_whole() {
    let m = scheduler(&SAMPLE, MASKS);
    let mut out = [Task::default(); 16];
    let plan = plan_pass(&m, S, WALKER, &mut out);
    let mut buf = [Task::EMPTY; sched::PHASE_TASKS];
    let mut ticked = Vec::new();
    let pass = run_pass(&m, S, WALKER, &mut buf, || true, |t| ticked.push(t));
    assert_eq!(pass, Pass { survey: plan, ticked: 9, whole: true });
    assert_eq!(ticked, out[..9]);
}

#[test]
fn each_phase_is_read_again_right_before_its_ticks() {
    // Phase 0's first tick gives phase 3's first task another tick: phase 3 ticks the new one.
    let m = scheduler(&SAMPLE, MASKS);
    let (_, _, vtable) = element(&m, 3, 0);
    let mut buf = [Task::EMPTY; sched::PHASE_TASKS];
    let mut ticked = Vec::new();
    let pass = run_pass(&m, S, WALKER, &mut buf, || true, |t| {
        if t.tick == tick(1) {
            m.put(vtable + 0x30, tick(50) as u64, 8);
        }
        ticked.push((t.phase, t.tick));
    });
    assert!(pass.whole);
    assert!(ticked.contains(&(3, tick(50))) && !ticked.contains(&(3, tick(5))), "{ticked:?}");
    // Phase 0's first tick frees phase 3's second task: phases 0 and 1 ran, phase 3 is not ticked, the pass not whole.
    let m = scheduler(&SAMPLE, MASKS);
    let (_, task, _) = element(&m, 3, 1);
    let mut ticked = Vec::new();
    let pass = run_pass(&m, S, WALKER, &mut buf, || true, |t| {
        if t.tick == tick(1) {
            m.put(task, 0, 8);
        }
        ticked.push(t.tick);
    });
    assert_eq!((pass.ticked, pass.whole), (4, false));
    assert_eq!(ticked, [tick(1), tick(2), tick(3), tick(4)]);
}

#[test]
fn the_pass_stops_between_ticks_once_the_practice_is_no_longer_live() {
    let m = scheduler(&SAMPLE, MASKS);
    let live = Cell::new(true);
    let mut buf = [Task::EMPTY; sched::PHASE_TASKS];
    let mut ticked = Vec::new();
    // Phase 1's first task runs the practice's teardown (its session clear drops the live flag).
    let pass = run_pass(&m, S, WALKER, &mut buf, || live.get(), |t| {
        ticked.push(t.tick);
        if t.tick == tick(3) {
            live.set(false);
        }
    });
    assert_eq!((pass.ticked, pass.whole), (3, false));
    assert_eq!(ticked, [tick(1), tick(2), tick(3)]);
    // No walker: nothing ticks. A buffer narrower than the widest phase: nothing ticks.
    let mut calls = 0;
    let pass = run_pass(&m, S, tick(99), &mut buf, || true, |_| calls += 1);
    assert_eq!((pass.ticked, pass.whole), (0, false));
    let mut narrow = [Task::EMPTY; 2];
    let pass = run_pass(&m, S, WALKER, &mut narrow, || true, |_| calls += 1);
    assert_eq!((pass.ticked, pass.whole, calls), (0, false, 0));
}
