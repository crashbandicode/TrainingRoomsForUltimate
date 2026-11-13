//! SPEC-cpu-06 (aerial timing, throws and pummels, attack angle, fast fall, full hop, crouch) and the
//! decisions it carries: B5 / N4 (throws yield to a recording), B6, B8 (the angle only applies to
//! mash-driven attacks), B11 (a changed Throw Options row re-rolls the held case), N3 (delay ranges).
//! Test names are the spec's §9 rows; `cpu06_roll_points` is in `tests/cpu_core.rs`.

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::cpu::command::CmdFacts;
use model::cpu::facts::FrameFacts;
use model::cpu::ids::{self, Table, C};
use model::cpu::mash::Action;
use model::cpu::playback::PlaybackState;
use model::cpu::role::FighterKey;
use model::cpu::settings::{CpuSettings, DelayRow};
use model::cpu::state::Cpu;
use model::cpu::status::Situation;
use model::menu::reroll;
use model::settings::Snapshot;

fn c(i: usize) -> C {
    assert!(i < ids::COUNT);
    // SAFETY: `C` is a fieldless `repr(u16)` enum numbered from 0; every value below COUNT is one.
    unsafe { std::mem::transmute::<u16, C>(i as u16) }
}

fn table() -> Table {
    let all: Vec<(C, i32)> = (0..ids::COUNT).map(|i| (c(i), 1000 + i as i32)).collect();
    Table::with(&all)
}

fn settings(rows: &[(&str, Vec<u8>)]) -> CpuSettings {
    let mut snap = Snapshot::factory();
    for (id, row) in rows {
        snap.set(id, row).unwrap();
    }
    CpuSettings::from_snapshot(&snap)
}

fn delay(at: &[usize]) -> Vec<u8> {
    let mut row = vec![0u8; 31];
    for i in at {
        row[*i] = 1;
    }
    row
}

fn cpu_with(rows: &[(&str, Vec<u8>)]) -> (Cpu, FighterKey) {
    let mut cpu = Cpu::new(settings(rows), 66);
    cpu.ids = table();
    let key = FighterKey::new(1, false).unwrap();
    cpu.fighters[key.index()].pending = 0;
    cpu.fighters[key.index()].defence.stale_written = Some(false);
    (cpu, key)
}

fn st(c: C) -> i32 {
    table().get(c)
}

fn air(status: C) -> CmdFacts {
    CmdFacts { status: st(status), situation: Situation::Air, ..CmdFacts::default() }
}

fn ground(status: C) -> CmdFacts {
    CmdFacts { status: st(status), situation: Situation::Ground, ..CmdFacts::default() }
}

fn cpu06_aerial_delay_rules() {
    // A 10-frame delay counts only on frames where CONT_ATTACK_AIR is enabled; 0 and "already in
    // ATTACK_AIR" wait for nothing.
    let (mut cpu, key) = cpu_with(&[("aerial_delay", delay(&[10]))]);
    cpu.fighters[key.index()].mash.queue.push(Action::NAIR);
    cpu.fighters[key.index()].mash.rolled.aerial_delay = 10;
    let attack_n = st(C::FlagAttackN) as u32;
    // Before the aerial is possible: waits and does not count.
    for _ in 0..20 {
        cpu.tick_clock();
        let out = cpu.command_flags(key, false, &air(C::StatusFall));
        assert_eq!(out.bits & attack_n, 0);
    }
    assert_eq!(cpu.fighters[key.index()].counters.count(model::cpu::counter::CounterId::Aerial), 0);
    // Possible: the delay counts 10 frames, then the press comes out.
    let enabled = || CmdFacts { cont_attack_air: true, ..air(C::StatusFall) };
    let mut pressed_at = None;
    for n in 0..30 {
        cpu.tick_clock();
        let f = enabled();
        if cpu.command_flags(key, false, &f).bits & attack_n != 0 {
            pressed_at = Some(n);
            break;
        }
    }
    assert_eq!(pressed_at, Some(10));
    assert_eq!(cpu.fighters[key.index()].mash.queued_aerial, Some(Action::NAIR));
    assert_eq!(cpu.attack_air_kind(key), Some(st(C::AirKindN)));
    // A delay of 0 presses at once; already in ATTACK_AIR no wait.
    let (mut cpu, key) = cpu_with(&[]);
    cpu.fighters[key.index()].mash.queue.push(Action::FAIR);
    let f = CmdFacts { cont_attack_air: true, ..air(C::StatusFall) };
    assert_ne!(cpu.command_flags(key, false, &f).bits & st(C::FlagAttackN) as u32, 0);
    let (mut cpu, key) = cpu_with(&[]);
    cpu.fighters[key.index()].mash.queue.push(Action::FAIR);
    cpu.fighters[key.index()].mash.rolled.aerial_delay = 30;
    let f = air(C::StatusAttackAir);
    assert_ne!(cpu.command_flags(key, false, &f).bits & st(C::FlagAttackN) as u32, 0);
}

fn cpu06_falling_aerial() {
    let (mut cpu, key) = cpu_with(&[]);
    cpu.fighters[key.index()].mash.queue.push(Action::NAIR);
    cpu.fighters[key.index()].mash.rolled.falling_aerial = true;
    let attack_n = st(C::FlagAttackN) as u32;
    let rising = air(C::StatusJumpAerial);
    assert_eq!(cpu.command_flags(key, false, &rising).bits & attack_n, 0, "withheld while rising");
    let falling = CmdFacts { falling: true, ..rising };
    assert_ne!(cpu.command_flags(key, false, &falling).bits & attack_n, 0, "pressed once the gravity speed is negative");
}

fn cpu06_full_hop_hold() {
    // A grounded aerial mash returns only the jump bit when a full hop was rolled; the jump buttons are
    // held during JUMP_SQUAT only, released never; a short hop does not hold.
    let (mut cpu, key) = cpu_with(&[]);
    cpu.fighters[key.index()].mash.queue.push(Action::UAIR);
    cpu.fighters[key.index()].mash.rolled.full_hop = true;
    let out = cpu.command_flags(key, false, &ground(C::StatusWait));
    assert_eq!(out.bits, st(C::FlagJumpButton) as u32);
    cpu.command_flags(key, false, &ground(C::StatusJumpSquat));
    assert_eq!(cpu.full_hop_button(key, true), Some(true));
    assert_eq!(cpu.full_hop_button(key, false), Some(false));
    cpu.fighters[key.index()].mash.rolled.full_hop = false;
    assert_eq!(cpu.full_hop_button(key, true), None);
    // Only in the squat, never during a playback.
    cpu.fighters[key.index()].mash.rolled.full_hop = true;
    cpu.command_flags(key, false, &ground(C::StatusWait));
    // (the seen status is only refreshed by frames; use one)
    cpu.frame(key, &FrameFacts { status: st(C::StatusWait), ..FrameFacts::default() });
    assert_eq!(cpu.full_hop_button(key, true), None);
    cpu.frame(key, &FrameFacts { status: st(C::StatusJumpSquat), ..FrameFacts::default() });
    assert_eq!(cpu.full_hop_button(key, true), Some(true));
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    assert_eq!(cpu.full_hop_button(key, true), None);
}

fn cpu06_fast_fall_rules() {
    let (mut cpu, key) = cpu_with(&[("fast_fall_delay", delay(&[5]))]);
    cpu.fighters[key.index()].mash.rolled.fast_fall = true;
    let fall = st(C::StatusFall);
    let frame = |cpu: &mut Cpu, falling: bool, status: i32, dive: bool| {
        cpu.tick_clock();
        cpu.frame(key, &FrameFacts { status, situation: Situation::Air, falling, reserve_dive: dive, ..FrameFacts::default() }).fast_fall
    };
    // The delay is rolled at the apex, every frame until the fall begins; nothing yet.
    assert!(!frame(&mut cpu, false, fall, false));
    assert_eq!(cpu.fighters[key.index()].mash.rolled.fast_fall_delay, 5);
    // Falling: waits 5 frames, then the flag and the flash once.
    let mut at = None;
    for n in 0..12 {
        if frame(&mut cpu, true, fall, false) {
            at = Some(n);
            break;
        }
    }
    assert_eq!(at, Some(5));
    // Already reserved: nothing. Wrong status: nothing. Playback: nothing.
    assert!(!frame(&mut cpu, true, fall, true));
    assert!(!frame(&mut cpu, true, st(C::StatusEscapeAir), false));
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    assert!(!frame(&mut cpu, true, fall, false));
    // N3: a set range is picked uniformly (fast fall delay range).
    let s = settings(&[("fast_fall_delay_range", vec![2, 4])]);
    assert_eq!(s.fast_fall_delay, DelayRow::Range { min: 2, max: 4, step: 1 });
}

/// rc.12 (the operator 2026-11-13: "it's supposed to do so at the peak of the hop not later"; terra-gif-1
/// fast-fall-boot2: the spark one frame before the landing): a hop stays in Jump or Jump Aerial past its apex, so the
/// fast fall comes on the first falling frame (plus the delay) in every airborne status that takes a dive.
fn cpu06_fast_fall_at_the_apex_of_a_hop() {
    for (status, delay_at) in [(C::StatusJump, 0usize), (C::StatusJumpAerial, 0), (C::StatusJump, 3), (C::StatusFallAerial, 2)] {
        let (mut cpu, key) = cpu_with(&[("fast_fall_delay", delay(&[delay_at]))]);
        cpu.fighters[key.index()].mash.rolled.fast_fall = true;
        let frame = |cpu: &mut Cpu, falling: bool| {
            cpu.tick_clock();
            cpu.frame(key, &FrameFacts { status: st(status), situation: Situation::Air, falling, ..FrameFacts::default() }).fast_fall
        };
        // Rising: nothing.
        for _ in 0..4 {
            assert!(!frame(&mut cpu, false), "{status:?} rising");
        }
        // The apex: the first falling frame, plus the delay.
        let at = (0..12).find(|_| frame(&mut cpu, true));
        assert_eq!(at, Some(delay_at), "{status:?} delay {delay_at}");
    }
}

fn cpu06_attack_angle() {
    // Up and Down values; neutral and an empty row leave the game's; not on the ledge; not during a
    // playback; the angle clears when the queue empties (decision B8).
    let (mut cpu, key) = cpu_with(&[]);
    cpu.fighters[key.index()].mash.rolled.attack_angle = Some(1);
    assert_eq!(cpu.attack_angle(key), Some(1.0));
    cpu.fighters[key.index()].mash.rolled.attack_angle = Some(2);
    assert_eq!(cpu.attack_angle(key), Some(-1.0));
    cpu.fighters[key.index()].mash.rolled.attack_angle = None;
    assert_eq!(cpu.attack_angle(key), None);
    cpu.fighters[key.index()].mash.rolled.attack_angle = Some(1);
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    assert_eq!(cpu.attack_angle(key), None);
    cpu.playback = PlaybackState::default();
    cpu.frame(key, &FrameFacts { situation: Situation::Cliff, ..FrameFacts::default() });
    assert_eq!(cpu.attack_angle(key), None, "not on the ledge");
    // The queue is empty on the next command query: the angle goes.
    cpu.command_flags(key, false, &ground(C::StatusWait));
    assert_eq!(cpu.fighters[key.index()].mash.rolled.attack_angle, None);
    // A mashed forward tilt with the angle Up: the game queries the commands more than once a frame, and the tilt's
    // status reads the angle after the query that completed the action. The angle holds while the body is in the
    // status the action completed in, and goes once the body leaves it (wa-menu7: the hook answered no angle).
    let (mut cpu, key) = cpu_with(&[]);
    cpu.fighters[key.index()].mash.queue.push(Action::F_TILT);
    cpu.fighters[key.index()].mash.rolled.attack_angle = Some(1);
    cpu.tick_clock();
    cpu.command_flags(key, false, &ground(C::StatusWait));
    cpu.tick_clock();
    cpu.command_flags(key, false, &ground(C::StatusAttackS3));
    assert!(cpu.fighters[key.index()].mash.queue.is_empty(), "the tilt's status completes the action");
    cpu.command_flags(key, false, &ground(C::StatusAttackS3));
    assert_eq!(cpu.attack_angle(key), Some(1.0), "the second query of the frame keeps the angle");
    for _ in 0..10 {
        cpu.tick_clock();
        cpu.command_flags(key, false, &ground(C::StatusAttackS3));
    }
    assert_eq!(cpu.attack_angle(key), Some(1.0), "held through the tilt");
    cpu.tick_clock();
    cpu.command_flags(key, false, &ground(C::StatusWait));
    assert_eq!(cpu.attack_angle(key), None, "gone once the body leaves the tilt");
    // A later native tilt, not from the mash, gets no angle (decision B8).
    cpu.command_flags(key, false, &ground(C::StatusAttackS3));
    assert_eq!(cpu.attack_angle(key), None, "a native tilt after the mash's");
}

fn cpu06_crouch_statuses() {
    let (mut cpu, key) = cpu_with(&[("crouch", vec![1, 0])]);
    for status in [C::StatusWait, C::StatusSquat, C::StatusSquatB, C::StatusSquatF, C::StatusSquatRv, C::StatusSquatWait] {
        cpu.frame(key, &FrameFacts { status: st(status), ..FrameFacts::default() });
        assert_eq!(cpu.crouch_stick(key), Some(-1.0), "{status:?}");
    }
    cpu.frame(key, &FrameFacts { status: st(C::StatusFall), ..FrameFacts::default() });
    assert_eq!(cpu.crouch_stick(key), None);
    let (mut cpu, key) = cpu_with(&[]);
    cpu.frame(key, &FrameFacts { status: st(C::StatusWait), ..FrameFacts::default() });
    assert_eq!(cpu.crouch_stick(key), None, "Crouch Off");
}

fn holding() -> CmdFacts {
    CmdFacts { cont_throw_hi: true, ..ground(C::StatusCatchWait) }
}

fn cpu06_throw_state_machine() {
    // Up throw after 30 frames, pummelling from frame 10 while it waits; the throw bits come on the
    // category-2 query; the mash after the throw ignores the triggers.
    let mut mash = vec![0u8; 30];
    mash[18] = 1;
    let mut throw = vec![0u8; 5];
    throw[3] = 1;
    let (mut cpu, key) = cpu_with(&[
        ("throw_state", throw),
        ("throw_delay", delay(&[6])),   // 30 frames at step 5
        ("pummel_delay", delay(&[2])),  // 10 frames
        ("mash_state", mash),
        ("mash_triggers", vec![0; 17]),
    ]);
    let hold = holding();
    let catch_attack = st(C::StatusCatchAttack);
    let mut pummels = 0;
    let mut thrown_at = None;
    for n in 0..60 {
        cpu.tick_clock();
        let out = cpu.command_flags(key, true, &hold);
        if out.request_status == Some(catch_attack) {
            pummels += 1;
            assert!(n >= 10, "no pummel before its delay");
        }
        if out.bits == st(C::FlagThrowHi) as u32 {
            thrown_at = Some(n);
            break;
        }
    }
    assert_eq!(thrown_at, Some(30));
    assert!(pummels > 0);
    assert_eq!(cpu.fighters[key.index()].mash.queue.current().map(|a| a.0), Some(18), "the mash after the throw ignores triggers");
    // Not holding anyone: everything is forgotten.
    cpu.command_flags(key, true, &ground(C::StatusWait));
    assert_eq!(cpu.fighters[key.index()].throw, model::cpu::throw::Throw::new());
    // Case None with no Pummel Delay: the CPU just holds; a missing CONT_THROW_HI: nothing.
    let (mut cpu, key) = cpu_with(&[]);
    let hold = holding();
    for _ in 0..40 {
        cpu.tick_clock();
        let out = cpu.command_flags(key, true, &hold);
        assert_eq!((out.bits, out.request_status), (0, None));
    }
    let (mut cpu, key) = cpu_with(&[("throw_state", vec![0, 1, 0, 0, 0])]);
    let no_term = CmdFacts { cont_throw_hi: false, ..holding() };
    assert_eq!(cpu.command_flags(key, true, &no_term).bits, 0);
    // Pummelling is off by default (an empty Pummel Delay row): a long delay never pummels.
    let (mut cpu, key) = cpu_with(&[("throw_state", vec![0, 1, 0, 0, 0]), ("throw_delay", delay(&[20]))]);
    let hold = holding();
    for _ in 0..40 {
        cpu.tick_clock();
        assert_eq!(cpu.command_flags(key, true, &hold).request_status, None);
    }
}

fn cpu06_throw_none_pummels_until_the_grab_ends() {
    // Operator 2026-11-13 (TR4): with Throw None (the default) a Pummel Delay still pummels. The CPU never throws, so
    // it pummels every Pummel Delay frames, as it does while waiting out a Throw Delay, until the game ends the grab
    // (the held fighter breaks free).
    let (mut cpu, key) = cpu_with(&[("pummel_delay", delay(&[2]))]); // 10 frames
    let hold = holding();
    let catch_attack = st(C::StatusCatchAttack);
    let (mut first, mut last, mut pummels) = (None, 0, 0);
    for n in 0..200 {
        cpu.tick_clock();
        let out = cpu.command_flags(key, true, &hold);
        assert_eq!(out.bits, 0, "Throw None never throws");
        if out.request_status == Some(catch_attack) {
            assert!(n >= 10, "no pummel before its delay");
            first.get_or_insert(n);
            (last, pummels) = (n, pummels + 1);
        }
    }
    assert!(first.is_some(), "Throw None with a Pummel Delay pummels");
    assert!(pummels >= 15 && last >= 180, "it keeps pummelling while it holds: {pummels}, the last at {last}");
    // The grab ended: the next one starts over.
    cpu.command_flags(key, true, &ground(C::StatusWait));
    assert_eq!(cpu.fighters[key.index()].throw, model::cpu::throw::Throw::new());
}

fn cpu06_throw_options_change_rerolls_held_case() {
    // B11: the CPU holds someone with Back Throw held; the menu changes the row to Forward Throw; the
    // case is cleared at once and rolled again in the same hold; the held delays are unchanged.
    let (mut cpu, key) = cpu_with(&[("throw_state", vec![0, 0, 1, 0, 0]), ("throw_delay", delay(&[4]))]);
    let hold = holding();
    for _ in 0..5 {
        cpu.tick_clock();
        cpu.command_flags(key, true, &hold);
    }
    assert_eq!(cpu.fighters[key.index()].throw.case, Some(2));
    let delay_before = cpu.fighters[key.index()].throw.delay;
    cpu.publish(settings(&[("throw_state", vec![0, 1, 0, 0, 0]), ("throw_delay", delay(&[4]))]), reroll::THROW_OPTIONS);
    assert_eq!(cpu.fighters[key.index()].throw.case, None);
    assert_eq!(cpu.fighters[key.index()].throw.delay, delay_before);
    let mut bits = 0;
    for _ in 0..40 {
        cpu.tick_clock();
        bits = cpu.command_flags(key, true, &hold).bits;
        if bits != 0 {
            break;
        }
    }
    assert_eq!(bits, st(C::FlagThrowF) as u32, "the new option comes out in the same hold");
}

fn cpu06_throw_yields_to_playback() {
    let mut throw = vec![0u8; 5];
    throw[1] = 1;
    let (mut cpu, key) = cpu_with(&[("throw_state", throw.clone())]);
    let hold = holding();
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    assert_eq!(cpu.command_flags(key, true, &hold).bits, 0, "the recording wins by default");
    cpu.playback = PlaybackState::default();
    assert_ne!(cpu.command_flags(key, true, &hold).bits, 0);
    let (mut cpu, key) = cpu_with(&[("throw_state", throw), ("settings_override_playback", vec![1, 0])]);
    let hold = holding();
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    assert_ne!(cpu.command_flags(key, true, &hold).bits, 0, "Settings override playback On: the throw settings win");
}

fn cpu06_range_picks_and_exclusive_rows() {
    // N3: ranges for the four delay rows of this spec; a set Pummel Delay range turns pummelling on.
    let s = settings(&[
        ("aerial_delay_range", vec![5, 15]),
        ("throw_delay_range", vec![4, 8]),
        ("pummel_delay_range", vec![0, 2]),
    ]);
    assert_eq!(s.aerial_delay, DelayRow::Range { min: 5, max: 15, step: 1 });
    assert_eq!(s.throw_delay, DelayRow::Range { min: 20, max: 40, step: 5 });
    assert_eq!(s.pummel_delay, DelayRow::Range { min: 0, max: 10, step: 5 });
    assert!(s.pummel_delay.is_set());
    assert!(!settings(&[]).pummel_delay.is_set());
}

fn cpu06_per_fighter_state() {
    // B6: Popo's held throw does not leak into Nana's.
    let (mut cpu, popo) = cpu_with(&[("throw_state", vec![0, 1, 0, 0, 0])]);
    let nana = FighterKey::new(1, true).unwrap();
    cpu.command_flags(popo, true, &holding());
    assert!(cpu.fighters[popo.index()].throw.case.is_some());
    assert!(cpu.fighters[nana.index()].throw.case.is_none());
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("cpu06_aerial_delay_rules", cpu06_aerial_delay_rules as fn()),
        ("cpu06_falling_aerial", cpu06_falling_aerial as fn()),
        ("cpu06_full_hop_hold", cpu06_full_hop_hold as fn()),
        ("cpu06_fast_fall_rules", cpu06_fast_fall_rules as fn()),
        ("cpu06_fast_fall_at_the_apex_of_a_hop", cpu06_fast_fall_at_the_apex_of_a_hop as fn()),
        ("cpu06_attack_angle", cpu06_attack_angle as fn()),
        ("cpu06_crouch_statuses", cpu06_crouch_statuses as fn()),
        ("cpu06_throw_state_machine", cpu06_throw_state_machine as fn()),
        ("cpu06_throw_none_pummels_until_the_grab_ends", cpu06_throw_none_pummels_until_the_grab_ends as fn()),
        ("cpu06_throw_options_change_rerolls_held_case", cpu06_throw_options_change_rerolls_held_case as fn()),
        ("cpu06_throw_yields_to_playback", cpu06_throw_yields_to_playback as fn()),
        ("cpu06_range_picks_and_exclusive_rows", cpu06_range_picks_and_exclusive_rows as fn()),
        ("cpu06_per_fighter_state", cpu06_per_fighter_state as fn()),
    ]);
}
