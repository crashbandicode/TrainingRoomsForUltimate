//! SPEC-cpu-03 (CPU ledge options) and the decisions it carries: B6 (one state per fighter), B11
//! (a changed Ledge Options row re-rolls the held case at once), N3 (Ledge Delay is a checklist or a
//! range). Test names are the spec's §9 rows.

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

use model::cpu::counter::CounterId;
use model::cpu::facts::FrameFacts;
use model::cpu::ids::{Table, C};
use model::cpu::ledge::{option, BUFFER_PLAYBACK_FRAME, BUFFER_REGULAR_FRAME, BUFFER_TETHER_FRAME};
use model::cpu::playback::PlaybackState;
use model::cpu::role::FighterKey;
use model::cpu::settings::{CpuSettings, DelayRow};
use model::cpu::state::Cpu;
use model::cpu::status::Situation;
use model::lra_reset::HubStep;
use model::menu::reroll;
use model::settings::Snapshot;

// Synthetic status numbers (the real ones come from the game's table at run time).
const WAIT: i32 = 1;
const CLIFF_CATCH: i32 = 40;
const CLIFF_WAIT: i32 = 41;
const CLIFF_CLIMB: i32 = 42;
const CLIFF_ESCAPE: i32 = 43;
const CLIFF_JUMP1: i32 = 44;
const CLIFF_ATTACK: i32 = 45;
const TERM_CLIMB: i32 = 7;
const TERM_ATTACK: i32 = 8;

fn table() -> Table {
    Table::with(&[
        (C::StatusWait, WAIT),
        (C::StatusCliffCatch, CLIFF_CATCH),
        (C::StatusCliffWait, CLIFF_WAIT),
        (C::StatusCliffClimb, CLIFF_CLIMB),
        (C::StatusCliffEscape, CLIFF_ESCAPE),
        (C::StatusCliffJump1, CLIFF_JUMP1),
        (C::StatusCliffAttack, CLIFF_ATTACK),
        (C::TermContCliffClimb, TERM_CLIMB),
        (C::TermContCliffAttack, TERM_ATTACK),
    ])
}

fn settings(rows: &[(&str, Vec<u8>)]) -> CpuSettings {
    let mut snap = Snapshot::factory();
    for (id, row) in rows {
        snap.set(id, row).unwrap();
    }
    CpuSettings::from_snapshot(&snap)
}

fn ledge_row(at: &[usize]) -> Vec<u8> {
    let mut row = vec![0u8; 10];
    for i in at {
        row[*i] = 1;
    }
    row
}

fn delay_row(at: &[usize]) -> Vec<u8> {
    let mut row = vec![0u8; 31];
    for i in at {
        row[*i] = 1;
    }
    row
}

fn action_row(at: &[usize]) -> Vec<u8> {
    let mut row = vec![0u8; 30];
    for i in at {
        row[*i] = 1;
    }
    row
}

fn cpu_with(s: CpuSettings) -> (Cpu, FighterKey) {
    let mut cpu = Cpu::new(s, 77);
    cpu.ids = table();
    let key = FighterKey::new(1, false).unwrap();
    cpu.fighters[key.index()].pending = 0;
    // The dodge-staling flag was written on an earlier frame (the factory row is On: flag false), so a
    // frame's output here is only the ledge step's.
    cpu.fighters[key.index()].defence.stale_written = Some(false);
    (cpu, key)
}

fn hanging(status: i32, prev: i32, frame: i32, actionable: bool, catch: bool) -> FrameFacts {
    FrameFacts {
        status,
        prev_status: prev,
        situation: Situation::Cliff,
        motion_frame: frame,
        catch_cliff: catch,
        cont_cliff_attack: actionable,
        ..FrameFacts::default()
    }
}

fn grounded() -> FrameFacts {
    FrameFacts { status: WAIT, situation: Situation::Ground, ..FrameFacts::default() }
}

fn cpu03_case_lifetime() {
    // The case and delay are rolled when the CPU hangs, held for the whole grab, and cleared when it
    // leaves the ledge, so the next grab rolls anew.
    let s = settings(&[("ledge_state", ledge_row(&[option::NEUTRAL as usize, option::ROLL as usize]))]);
    let (mut cpu, key) = cpu_with(s);
    cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, false, false));
    let held = cpu.fighters[key.index()].ledge.case;
    assert!(held.is_some_and(|c| c == option::NEUTRAL || c == option::ROLL));
    assert_eq!(cpu.fighters[key.index()].ledge.delay, Some(0));
    for frame in 4..12 {
        cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, frame, false, false));
        assert_eq!(cpu.fighters[key.index()].ledge.case, held, "held while hanging");
    }
    cpu.frame(key, &grounded());
    assert_eq!(cpu.fighters[key.index()].ledge.case, None);
    assert_eq!(cpu.fighters[key.index()].ledge.delay, None);
    // Many grabs roll both options.
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..100 {
        cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, false, false));
        seen.insert(cpu.fighters[key.index()].ledge.case.unwrap());
        cpu.frame(key, &grounded());
    }
    assert_eq!(seen.len(), 2);
}

fn cpu03_option_status_table() {
    for (option, want) in [
        (option::NEUTRAL, CLIFF_CLIMB),
        (option::ROLL, CLIFF_ESCAPE),
        (option::JUMP, CLIFF_JUMP1),
        (option::ATTACK, CLIFF_ATTACK),
    ] {
        let s = settings(&[("ledge_state", ledge_row(&[option as usize]))]);
        let (mut cpu, key) = cpu_with(s);
        let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 30, true, false));
        assert_eq!(out.request_status, Some(want), "option {option}");
        assert!(out.playback.is_none());
    }
}

fn cpu03_buffer_frames() {
    let s = settings(&[("ledge_state", ledge_row(&[option::JUMP as usize]))]);
    // A regular grab (CLIFF_WAIT after CLIFF_CATCH, catch flag false): the request comes on frame 19,
    // the last unactionable frame, and not on 18 or 20.
    let (mut cpu, key) = cpu_with(s);
    for frame in [17, 18, 20] {
        let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, frame, false, false));
        assert_eq!(out.request_status, None, "regular grab, frame {frame}");
    }
    let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, BUFFER_REGULAR_FRAME, false, false));
    assert_eq!(out.request_status, Some(CLIFF_JUMP1));
    assert_eq!(BUFFER_REGULAR_FRAME, 19);
    // A regular grab whose catch flag is already true does not use the buffer frame.
    let (mut cpu, key) = cpu_with(s);
    assert_eq!(cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 19, false, true)).request_status, None);
    // A tether grab (no CLIFF_CATCH before, catch flag true): frame 18.
    let (mut cpu, key) = cpu_with(s);
    assert_eq!(cpu.frame(key, &hanging(CLIFF_WAIT, WAIT, 19, false, true)).request_status, None);
    assert_eq!(
        cpu.frame(key, &hanging(CLIFF_WAIT, WAIT, BUFFER_TETHER_FRAME, false, true)).request_status,
        Some(CLIFF_JUMP1)
    );
    assert_eq!(BUFFER_TETHER_FRAME, 18);
    // A delay of 10 never uses the buffer frames: it counts from actionability.
    let delayed = settings(&[("ledge_state", ledge_row(&[option::JUMP as usize])), ("ledge_delay", delay_row(&[1]))]);
    let (mut cpu, key) = cpu_with(delayed);
    assert_eq!(cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 19, false, false)).request_status, None);
    assert_eq!(BUFFER_PLAYBACK_FRAME, 13);
}

fn cpu03_delay_and_climb_denial() {
    // A 30-frame delay (row option 3 at step 10): nothing before actionability, then 30 ticks.
    let s = settings(&[
        ("ledge_state", ledge_row(&[option::ROLL as usize])),
        ("ledge_delay", delay_row(&[3])),
    ]);
    let (mut cpu, key) = cpu_with(s);
    // Before CONT_CLIFF_ATTACK nothing counts, even many frames.
    for frame in 3..40 {
        cpu.tick_clock();
        let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, frame, false, false));
        assert_eq!(out.request_status, None);
    }
    assert_eq!(cpu.fighters[key.index()].counters.count(CounterId::Ledge), 0);
    // Once actionable the delay counts: the request comes when 30 frames have passed.
    let mut requested_at = None;
    for n in 0..60 {
        cpu.tick_clock();
        let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 40 + n, true, false));
        if out.request_status.is_some() {
            requested_at = Some(n);
            break;
        }
    }
    assert_eq!(requested_at, Some(30));
    // The native climb is denied while the delay runs, and only then; other terms and other statuses
    // answer the original.
    let (mut cpu, key) = cpu_with(s);
    cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, false, false));
    assert!(!cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_CLIMB, true), "denied while delaying");
    assert!(cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_ATTACK, true), "other terms pass through");
    assert!(cpu.ledge_transition_term(key, WAIT, TERM_CLIMB, true), "other statuses pass through");
    assert!(!cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_CLIMB, false), "an original false stays false");
    // Zero delay: the climb is not denied.
    let (mut cpu, key) = cpu_with(settings(&[("ledge_state", ledge_row(&[option::ROLL as usize]))]));
    cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, false, false));
    assert!(cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_CLIMB, true));
    // Wait holds: no request however long, the climb stays denied, the case is kept.
    let (mut cpu, key) = cpu_with(settings(&[("ledge_state", ledge_row(&[option::WAIT as usize]))]));
    for frame in 0..200 {
        let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 20 + frame, true, false));
        assert_eq!(out.request_status, None);
    }
    assert_eq!(cpu.fighters[key.index()].ledge.case, Some(option::WAIT));
    assert!(!cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_CLIMB, true));
}

fn cpu03_playback_paths() {
    let playback1 = option::PLAYBACK_FIRST as usize;
    // With Ledge Delay set (even "0"), a playback option starts early, on frame 13, while the CPU
    // cannot act yet.
    let with_delay_row = settings(&[("ledge_state", ledge_row(&[playback1])), ("ledge_delay", delay_row(&[0]))]);
    let (mut cpu, key) = cpu_with(with_delay_row);
    assert!(cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 12, false, false)).playback.is_none());
    // Until the playback starts the native climb is denied.
    assert!(!cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_CLIMB, true));
    let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 13, false, false));
    let start = out.playback.expect("early ledge playback");
    assert!(start.ledge && start.slot == 0);
    assert!(cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_CLIMB, true), "released once started");
    // With the row empty there is no early start; the normal playback starts when actionable.
    let empty_row = settings(&[("ledge_state", ledge_row(&[playback1 + 2]))]);
    let (mut cpu, key) = cpu_with(empty_row);
    assert!(cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 13, false, false)).playback.is_none());
    let normal = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 30, true, false)).playback.unwrap();
    assert!(!normal.ledge && normal.slot == 2);
    // A set range counts as a non-empty row for the early start (N3).
    let range = settings(&[("ledge_state", ledge_row(&[playback1])), ("ledge_delay_range", vec![0, 0])]);
    let (mut cpu, key) = cpu_with(range);
    assert!(cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 13, false, false)).playback.is_some());
}

fn cpu03_ledge_playback_frame_asks_for_the_slot_on_the_ledge() {
    let playback1 = option::PLAYBACK_FIRST as usize;
    let row = settings(&[("ledge_state", ledge_row(&[playback1 + 1])), ("ledge_delay", delay_row(&[0]))]);
    let (mut cpu, key) = cpu_with(row);
    let start = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 13, false, false)).playback.expect("early ledge playback");
    // Slot 2 (index 1) on the ledge.
    assert_eq!((start.slot, start.ledge), (1, true));
}

fn cpu03_post_getup_mash() {
    // Override of the performed kind first, else the mash action when the Ledge Option trigger is
    // ticked, else nothing.
    let roll_then_fsmash = settings(&[
        ("ledge_state", ledge_row(&[option::ROLL as usize])),
        ("ledge_roll_override", action_row(&[15])),
        ("mash_state", action_row(&[18])),
        ("mash_triggers", {
            let mut t = vec![0u8; 17];
            t[8] = 1;
            t
        }),
    ]);
    let (mut cpu, key) = cpu_with(roll_then_fsmash);
    cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 30, true, false));
    assert_eq!(cpu.fighters[key.index()].mash.queue.current().map(|a| a.0), Some(15), "the override wins");
    // No override for the kind: the trigger path buffers mash_state.
    let trigger_only = settings(&[
        ("ledge_state", ledge_row(&[option::ROLL as usize])),
        ("mash_state", action_row(&[18])),
        ("mash_triggers", {
            let mut t = vec![0u8; 17];
            t[8] = 1;
            t
        }),
    ]);
    let (mut cpu, key) = cpu_with(trigger_only);
    cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 30, true, false));
    assert_eq!(cpu.fighters[key.index()].mash.queue.current().map(|a| a.0), Some(18));
    // Neither: nothing is buffered.
    let none = settings(&[("ledge_state", ledge_row(&[option::ROLL as usize])), ("mash_state", action_row(&[18]))]);
    let none = CpuSettings { mash_triggers: 0, ..none };
    let (mut cpu, key) = cpu_with(none);
    let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 30, true, false));
    assert_eq!(out.request_status, Some(CLIFF_ESCAPE));
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
}

/// rc.12 TR2: the fork's lobby filter is gone (nothing set it); a Playback Slot option is held like any other case.
fn cpu03_a_playback_case_is_held() {
    let playback_only = settings(&[("ledge_state", ledge_row(&[option::PLAYBACK_FIRST as usize]))]);
    let (mut cpu, key) = cpu_with(playback_only);
    cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 14, false, false));
    assert_eq!(cpu.fighters[key.index()].ledge.case, Some(option::PLAYBACK_FIRST));
}

fn cpu03_per_fighter_state() {
    // B6: Popo and Nana each roll and hold their own case and delay.
    let s = settings(&[("ledge_state", ledge_row(&[option::NEUTRAL as usize, option::ROLL as usize, option::WAIT as usize]))]);
    let (mut cpu, popo) = cpu_with(s);
    let nana = FighterKey::new(1, true).unwrap();
    cpu.fighters[nana.index()].pending = 0;
    let mut differ = false;
    for _ in 0..50 {
        cpu.frame(popo, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, false, false));
        cpu.frame(nana, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, false, false));
        differ |= cpu.fighters[popo.index()].ledge.case != cpu.fighters[nana.index()].ledge.case;
        // Popo leaves the ledge; Nana's case stays held.
        cpu.frame(popo, &grounded());
        assert!(cpu.fighters[nana.index()].ledge.case.is_some());
        cpu.frame(nana, &grounded());
    }
    assert!(differ, "independent rolls differ somewhere in 50 grabs");
}

fn cpu03_ledge_delay_range_pick() {
    // N3: a set range is picked uniformly among its step values once per grab.
    let s = settings(&[("ledge_state", ledge_row(&[option::ROLL as usize])), ("ledge_delay_range", vec![3, 6])]);
    assert_eq!(s.ledge_delay, DelayRow::Range { min: 30, max: 60, step: 10 });
    let (mut cpu, key) = cpu_with(s);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..200 {
        cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, false, false));
        seen.insert(cpu.fighters[key.index()].ledge.delay.unwrap());
        cpu.frame(key, &grounded());
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), vec![30, 40, 50, 60]);
    // The delay is held for the whole grab.
    cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, false, false));
    let held = cpu.fighters[key.index()].ledge.delay;
    for frame in 4..30 {
        cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, frame, false, false));
        assert_eq!(cpu.fighters[key.index()].ledge.delay, held);
    }
}

fn cpu03_range_counts_as_nonempty_row() {
    // See cpu03_playback_paths: the early playback buffer needs a set Ledge Delay row, and a set
    // range is one.
    let range = settings(&[("ledge_delay_range", vec![0, 0])]);
    assert!(range.ledge_delay.is_set());
    assert!(!settings(&[]).ledge_delay.is_set());
}

fn cpu03_ledge_options_change_rerolls_held_case() {
    // B11: the CPU is parked on the ledge with Wait; the menu changes the row to Roll; it rolls from
    // the new set during the same hang, with no second grab, and the held delay is unchanged.
    let wait = settings(&[("ledge_state", ledge_row(&[option::WAIT as usize])), ("ledge_delay", delay_row(&[2]))]);
    let (mut cpu, key) = cpu_with(wait);
    for frame in 3..50 {
        let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, frame, true, false));
        assert_eq!(out.request_status, None);
    }
    let delay = cpu.fighters[key.index()].ledge.delay;
    assert_eq!(delay, Some(20));
    let roll = settings(&[("ledge_state", ledge_row(&[option::ROLL as usize])), ("ledge_delay", delay_row(&[2]))]);
    cpu.publish(roll, reroll::LEDGE_OPTIONS);
    assert_eq!(cpu.fighters[key.index()].ledge.case, None, "cleared at once");
    assert_eq!(cpu.fighters[key.index()].ledge.delay, delay);
    // The next frame holds Roll and (the delay counting from now) requests it after the held delay.
    let mut requested = None;
    for n in 0..40 {
        cpu.tick_clock();
        let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 60 + n, true, false));
        if out.request_status.is_some() {
            requested = Some(n);
            assert_eq!(out.request_status, Some(CLIFF_ESCAPE));
            break;
        }
    }
    assert_eq!(requested, Some(20));
}

fn cpu03_ledge_options_emptied_clears_case() {
    let roll = settings(&[("ledge_state", ledge_row(&[option::ROLL as usize])), ("ledge_delay", delay_row(&[5]))]);
    let (mut cpu, key) = cpu_with(roll);
    cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, false, false));
    assert!(cpu.fighters[key.index()].ledge.case.is_some());
    assert!(!cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_CLIMB, true));
    // The edit empties the row: the case clears, the step gate stops the feature, the climb is no
    // longer denied.
    let emptied = settings(&[("ledge_state", ledge_row(&[]))]);
    cpu.publish(emptied, reroll::LEDGE_OPTIONS);
    assert_eq!(cpu.fighters[key.index()].ledge.case, None);
    let out = cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 4, true, false));
    assert_eq!(out, Default::default());
    assert!(cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_CLIMB, true));
}

fn cpu03_playback_yield_and_reset() {
    // While another playback runs the feature yields entirely (no request, no denial).
    let s = settings(&[("ledge_state", ledge_row(&[option::WAIT as usize]))]);
    let (mut cpu, key) = cpu_with(s);
    cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, false, false));
    assert!(!cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_CLIMB, true));
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    assert_eq!(cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 30, true, false)), Default::default());
    assert!(cpu.ledge_transition_term(key, CLIFF_WAIT, TERM_CLIMB, true), "yielded: native behaviour");
    // The reset bundle resets the delay and its counter, not the case.
    let s = settings(&[("ledge_state", ledge_row(&[option::ROLL as usize])), ("ledge_delay", delay_row(&[4]))]);
    let (mut cpu, key) = cpu_with(s);
    cpu.frame(key, &hanging(CLIFF_WAIT, CLIFF_CATCH, 3, true, false));
    cpu.tick_clock();
    cpu.tick_clock();
    assert_eq!(cpu.fighters[key.index()].counters.count(CounterId::Ledge), 2);
    let case = cpu.fighters[key.index()].ledge.case;
    cpu.hub_step(HubStep::FrameCounters);
    cpu.hub_step(HubStep::Ledge);
    assert_eq!(cpu.fighters[key.index()].counters.count(CounterId::Ledge), 0);
    assert_eq!(cpu.fighters[key.index()].ledge.delay, None);
    assert_eq!(cpu.fighters[key.index()].ledge.case, case);
}

fn cpu03_whole_settings_replacement_yields_the_menus_reroll_bits() {
    // The diag build replaces the whole settings from a file; the bits are the ones the menu sets row by
    // row: Ledge Options (B11), SDI Direction, Shield Tilt and Throw Options, but not Ledge Delay.
    let base = Snapshot::factory();
    assert_eq!(model::menu::rerolls_between(&base, &base), 0);
    let mut next = Snapshot::factory();
    next.set("ledge_state", &ledge_row(&[option::ROLL as usize])).unwrap();
    assert_eq!(model::menu::rerolls_between(&base, &next), reroll::LEDGE_OPTIONS);
    let mut delayed = next.clone();
    delayed.set("ledge_delay", &delay_row(&[10])).unwrap();
    assert_eq!(model::menu::rerolls_between(&next, &delayed), 0, "a Ledge Delay change is not a re-roll");
    let mut both = delayed.clone();
    both.set("ledge_state", &ledge_row(&[])).unwrap();
    both.set("sdi_state", &{
        let mut row = base.get("sdi_state").unwrap().to_vec();
        row[1] = 1;
        row
    })
    .unwrap();
    assert_eq!(model::menu::rerolls_between(&delayed, &both), reroll::LEDGE_OPTIONS | reroll::SDI_DIRECTION);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("cpu03_case_lifetime", cpu03_case_lifetime as fn()),
        ("cpu03_option_status_table", cpu03_option_status_table as fn()),
        ("cpu03_buffer_frames", cpu03_buffer_frames as fn()),
        ("cpu03_delay_and_climb_denial", cpu03_delay_and_climb_denial as fn()),
        ("cpu03_playback_paths", cpu03_playback_paths as fn()),
        ("cpu03_ledge_playback_frame_asks_for_the_slot_on_the_ledge", cpu03_ledge_playback_frame_asks_for_the_slot_on_the_ledge as fn()),
        ("cpu03_post_getup_mash", cpu03_post_getup_mash as fn()),
        ("cpu03_a_playback_case_is_held", cpu03_a_playback_case_is_held as fn()),
        ("cpu03_per_fighter_state", cpu03_per_fighter_state as fn()),
        ("cpu03_ledge_delay_range_pick", cpu03_ledge_delay_range_pick as fn()),
        ("cpu03_range_counts_as_nonempty_row", cpu03_range_counts_as_nonempty_row as fn()),
        ("cpu03_ledge_options_change_rerolls_held_case", cpu03_ledge_options_change_rerolls_held_case as fn()),
        ("cpu03_ledge_options_emptied_clears_case", cpu03_ledge_options_emptied_clears_case as fn()),
        ("cpu03_playback_yield_and_reset", cpu03_playback_yield_and_reset as fn()),
        ("cpu03_whole_settings_replacement_yields_the_menus_reroll_bits", cpu03_whole_settings_replacement_yields_the_menus_reroll_bits as fn()),
    ]);
}
