//! Review R2 of package 1: the native pause is an input owner. These tests drive the REAL ownership
//! producer, `InputAdapter::pause_begin` / `pause_end` (the code the runtime's two `StatePause` hooks
//! call) and the REAL poll (`InputAdapter::poll`, which reads the owner mask itself) on a mask the
//! test does not hand-fill: nothing here passes an `Owners` value to the queue.

use std::sync::atomic::{AtomicU8, Ordering};

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::input_adapter::{owners, InputAdapter, PauseReport};
use model::input_delay::{Clear, MappedInputs, Owners};

/// A record carrying the poll number, so every output names the poll it came from. Gameplay
/// records have `lstick_x` = 50 (the player is holding the stick), menu records `lstick_y` = 100
/// (the pause menu's stick/d-pad navigation).
fn game(n: u32) -> MappedInputs {
    MappedInputs { buttons: n, lstick_x: 50, ..MappedInputs::NEUTRAL }
}

fn menu(n: u32) -> MappedInputs {
    MappedInputs { buttons: 0x8000_0000 | n, lstick_y: 100, ..MappedInputs::NEUTRAL }
}

/// The game polls P1 `from..from+count` times with `make` records at `delay`.
fn polls(a: &mut InputAdapter, mask: &AtomicU8, delay: u8, from: u32, count: u32, make: fn(u32) -> MappedInputs) -> Vec<MappedInputs> {
    (from..from + count).map(|n| a.poll(mask, delay, make(n)).0).collect()
}

#[test]
fn a_pause_empties_the_queue_owns_the_input_and_nothing_old_reaches_gameplay() {
    let mask = AtomicU8::new(0);
    let mut a = InputAdapter::new();
    let delay = 5;
    // Gameplay: the queue fills (poll 1..=12), the stick held.
    let before = polls(&mut a, &mask, delay, 1, 12, game);
    assert_eq!(before[11], game(7), "steady state: 5 polls late");
    assert_eq!(a.queued(), 5);

    // The native pause begins: the producer takes the input and the queue is empty at once.
    assert_eq!(a.pause_begin(&mask), 5);
    assert_eq!(owners(&mask), Owners::VANILLA_PAUSE);
    assert_eq!(a.queued(), 0, "emptied at takeover, before any poll");

    // The pause menu is used: its input is polled, nothing of it reaches the game, the queue stays empty.
    let paused = polls(&mut a, &mask, delay, 100, 20, menu);
    assert!(paused.iter().all(|r| *r == MappedInputs::NEUTRAL), "{paused:?}");
    assert_eq!(a.queued(), 0);

    // Resume: the owner is released, the first `delay` gameplay polls are neutral, then gameplay input
    // arrives `delay` polls late. No pre-pause record (game(8..=12)) and no menu record comes out.
    let report = a.pause_end(&mask, delay);
    assert_eq!(owners(&mask), Owners::NONE);
    assert_eq!(report, PauseReport { queued_at_begin: 5, owned_polls: 20, owned_non_neutral: 0 });
    let mut after = Vec::new();
    let mut checks = Vec::new();
    for n in 200..212 {
        let (out, check) = a.poll(&mask, delay, game(n));
        after.push(out);
        checks.extend(check);
    }
    assert!(after[..5].iter().all(|r| *r == MappedInputs::NEUTRAL), "{after:?}");
    assert_eq!(after[5], game(200), "the first post-resume record, 5 polls late");
    for out in &after {
        assert!(out.lstick_y != 100 && !(8..=12).contains(&out.buttons), "a pre-pause or menu record leaked: {out:?}");
    }
    assert_eq!(checks.len(), 1);
    assert_eq!((checks[0].delay, checks[0].leaked), (5, 0));
}

#[test]
fn takeover_empties_the_queue_even_when_polls_stop_while_paused() {
    // The mapping poll may not run at all during the pause: the queue must already be empty.
    let mask = AtomicU8::new(0);
    let mut a = InputAdapter::new();
    polls(&mut a, &mask, 4, 1, 10, game);
    assert_eq!(a.queued(), 4);
    a.pause_begin(&mask);
    assert_eq!(a.queued(), 0, "no poll has run since the pause began");
    // No poll during the pause; the pause ends. The next polls are neutral for `delay` polls.
    a.pause_end(&mask, 4);
    let after = polls(&mut a, &mask, 4, 50, 6, game);
    assert!(after[..4].iter().all(|r| *r == MappedInputs::NEUTRAL));
    assert_eq!(after[4], game(50));
}

#[test]
fn the_unwired_producer_lets_the_stale_and_menu_records_through() {
    // What the review found: the mask is never set and nothing clears at takeover. The same sequence
    // as above, with the producer calls left out, plays the pre-pause record after the resume (and
    // the menu's records while paused). The test shows the scenario can fail.
    let mask = AtomicU8::new(0);
    let mut a = InputAdapter::new();
    polls(&mut a, &mask, 5, 1, 12, game);
    let paused = polls(&mut a, &mask, 5, 100, 20, menu);
    assert!(paused.iter().any(|r| r.lstick_y == 100), "menu records come out of the queue");
    let after = polls(&mut a, &mask, 5, 200, 6, game);
    assert!(after.iter().any(|r| r.lstick_y == 100), "a stale menu record plays after the resume: {after:?}");
}

#[test]
fn other_owners_still_work_beside_the_pause_and_survive_its_release() {
    let mask = AtomicU8::new(0);
    let mut a = InputAdapter::new();
    // The menu (another package) owns the input while the game pauses and resumes.
    mask.fetch_or(Owners::MENU.bits(), Ordering::AcqRel);
    a.pause_begin(&mask);
    assert!(owners(&mask).contains(Owners::MENU) && owners(&mask).contains(Owners::VANILLA_PAUSE));
    a.pause_end(&mask, 3);
    assert_eq!(owners(&mask), Owners::MENU, "only the pause bit is released");
    // Polls while the menu owns: neutral, queue empty, and the resume window does not count them.
    assert_eq!(a.poll(&mask, 3, game(1)).0, MappedInputs::NEUTRAL);
    mask.fetch_and(!Owners::MENU.bits(), Ordering::AcqRel);
    let outs: Vec<_> = (0..4).map(|n| a.poll(&mask, 3, game(10 + n))).collect();
    assert!(outs[..3].iter().all(|(r, _)| *r == MappedInputs::NEUTRAL));
    assert_eq!(outs[2].1.map(|c| c.leaked), Some(0), "the resume check ends after `delay` gameplay polls");
    assert_eq!(outs[3].0, game(10));
}

#[test]
fn delay_zero_and_repeated_transitions_are_harmless() {
    let mask = AtomicU8::new(0);
    let mut a = InputAdapter::new();
    // Delay 0: pass-through, a pause still owns and releases cleanly, no resume window.
    assert_eq!(a.poll(&mask, 0, game(1)).0, game(1));
    a.pause_begin(&mask);
    assert_eq!(a.poll(&mask, 0, game(2)).0, MappedInputs::NEUTRAL);
    a.pause_end(&mask, 0);
    let (out, check) = a.poll(&mask, 0, game(3));
    assert_eq!((out, check), (game(3), None));
    // A begin without an end, begin twice, end twice: the bit is a bit, not a counter.
    a.pause_begin(&mask);
    a.pause_begin(&mask);
    assert_eq!(owners(&mask), Owners::VANILLA_PAUSE);
    a.pause_end(&mask, 2);
    a.pause_end(&mask, 2);
    assert_eq!(owners(&mask), Owners::NONE);
    // The hub's clear is independent of the owner mask.
    polls(&mut a, &mask, 2, 1, 5, game);
    assert_eq!(a.clear(Clear::TrainingReset), 2);
}
