// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/test_table.rs"]
mod test_table;
#[allow(dead_code, unused_imports)]
#[path = "../src/model/mii/mod.rs"]
pub mod mii;
#[allow(unused_imports)]
mod engine {
    pub use super::mii;
}
#[allow(dead_code)]
mod panes {
    pub const TEXT: [u8; 4] = [236, 238, 242, 255]; // src/model/ui/panes.rs TEXT
}
#[allow(dead_code)]
#[path = "../src/model/notify.rs"]
mod notify;

// The CPU Character receipt follows who asked for the CPU change (ordinary_player::cpu_receipt): a practice game's
// own temporary fighter (Mash out's attacker and its restore) gets none, so nothing waits behind Mash out's standing
// notice to show over the drill (mashout-demo-devin TRIAGE-CPU-APPLIED-NOTICE.md; RED before the fix: 2 of the 3
// queue cases below drew "CPU fighter applied", triage/RED-run.log). The persisted choice follows the same owner.
use crate::engine::mii::ordinary_cpu::Request;
use crate::engine::mii::ordinary_player::{bank_notice, cpu_receipt, persists_selection, Requester};
use crate::notify::{Body, Queue, DEFAULT_COLOR};

const MASH: u8 = 1; // Mash out worker NOTICE_ID (standing, u16::MAX)
const CPU: u8 = 2; // "CPU Character" standing slot (character_roster.rs notify, 180 frames)

fn mash(q: &mut Queue, text: &str) {
    q.upsert(MASH, "Mash out", Body::copied(text), u16::MAX, DEFAULT_COLOR);
}

fn cpu(q: &mut Queue, original: bool, owner: Requester) {
    if let Some(text) = cpu_receipt(original, false, owner) {
        q.upsert(CPU, "CPU Character", Body::copied(text), 180, DEFAULT_COLOR);
    }
}

fn run(q: &mut Queue, frames: usize, menu_open: bool, draws: &mut Vec<(String, String)>) {
    for _ in 0..frames {
        if let Some(d) = q.tick(true, menu_open).draw {
            draws.push((d.header.to_string(), d.body.as_str().to_string()));
        }
    }
}

fn cpu_draws(draws: &[(String, String)]) -> Vec<String> {
    draws.iter().filter(|(h, _)| h == "CPU Character").map(|(_, b)| b.clone()).collect()
}

/// (b) demo -3 log #1104/#1779/#1784: the attacker load's receipt waited behind Mash out's notice and was drawn when
/// an Attacker-move change with Mash On restarted Mash out (its notice cleared).
fn practice_attacker_load_draws_nothing_after_an_attacker_change() {
    let (mut q, mut draws) = (Queue::new(), Vec::new());
    mash(&mut q, "Loading attacker");
    run(&mut q, 10, false, &mut draws);
    cpu(&mut q, false, Requester::Practice);
    mash(&mut q, "Mash! · 2f");
    run(&mut q, 600, false, &mut draws);
    run(&mut q, 120, true, &mut draws);
    q.clear_id(MASH);
    run(&mut q, 1, false, &mut draws);
    mash(&mut q, "Waiting for release");
    run(&mut q, 60, false, &mut draws);
    assert_eq!(cpu_draws(&draws), Vec::<String>::new());
}

/// (a) demo -2 log #1657..#1727: the restore's receipt stayed queued behind an open menu and the next attacker load
/// rewrote it in place, ahead of Mash out's notice.
fn practice_restore_and_reload_draw_nothing_off_then_on() {
    let (mut q, mut draws) = (Queue::new(), Vec::new());
    cpu(&mut q, true, Requester::Practice);
    run(&mut q, 30, false, &mut draws);
    run(&mut q, 90, true, &mut draws);
    mash(&mut q, "Loading attacker");
    cpu(&mut q, false, Requester::Practice);
    mash(&mut q, "Waiting for move");
    run(&mut q, 120, false, &mut draws);
    assert_eq!(cpu_draws(&draws), Vec::<String>::new());
}

/// A user's CPU Character pick still gets its receipt, and the user's restore its own.
fn a_user_cpu_choice_still_gets_its_receipt() {
    let (mut q, mut draws) = (Queue::new(), Vec::new());
    cpu(&mut q, false, Requester::User);
    run(&mut q, 5, false, &mut draws);
    assert_eq!(cpu_draws(&draws), vec!["CPU fighter applied".to_string()]);
    assert_eq!(cpu_receipt(true, false, Requester::User), Some("Original fighter restored"));
    assert_eq!(cpu_receipt(false, true, Requester::User), Some("CPU Mii configuration applied"));
}

/// Parity: the receipt and the persisted choice (`sync_selection`) follow one owner rule, for every receipt case, and
/// a user receipt is exactly the existing bank notice.
fn receipt_and_persisted_choice_follow_the_same_owner() {
    for owner in [Requester::User, Requester::Practice] {
        for original in [false, true] {
            for publish in [false, true] {
                let receipt = cpu_receipt(original, publish, owner);
                assert_eq!(receipt.is_some(), persists_selection(owner), "{owner:?} {original} {publish}");
                if owner == Requester::User {
                    assert_eq!(receipt, Some(bank_notice(false, publish, original)));
                }
            }
        }
    }
}

/// The owner travels with the request: a Direct request is the user's unless a practice game marks it its own.
fn the_request_carries_its_owner() {
    let direct = Request::direct(7, 0x42).unwrap();
    assert_eq!(direct.owner(), Requester::User);
    assert_eq!(Request::CYCLE.owner(), Requester::User);
    let practice = direct.owned(Requester::Practice);
    assert_eq!(practice.owner(), Requester::Practice);
    assert_eq!(practice.direct_kind(), Some(0x42));
    assert!(practice.current(7, None));
    assert_ne!(practice, direct);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("practice_attacker_load_draws_nothing_after_an_attacker_change", practice_attacker_load_draws_nothing_after_an_attacker_change as fn()),
        ("practice_restore_and_reload_draw_nothing_off_then_on", practice_restore_and_reload_draw_nothing_off_then_on as fn()),
        ("a_user_cpu_choice_still_gets_its_receipt", a_user_cpu_choice_still_gets_its_receipt as fn()),
        ("receipt_and_persisted_choice_follow_the_same_owner", receipt_and_persisted_choice_follow_the_same_owner as fn()),
        ("the_request_carries_its_owner", the_request_carries_its_owner as fn()),
    ]);
}
