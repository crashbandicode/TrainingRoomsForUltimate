//! B4-3 (Surface lane `artifacts/phase23/LANE-SURFACE-REPORT.md` "B4-3 root cause"; Ally lane `LANE-ALLY-REPORT.md`
//! "Round 4" §3): the CPU Kirby hat request stuck forever because `kirby_hats::on_frame` ran only with Player 1's
//! body. It now runs once per Kirby with that Kirby's own body (`state_kirby_opff_hook`, the fork's call site);
//! the role match is `hat_role`. The call-site wiring itself is checked in `tests/kirby_hat_cpu.rs`.

use crate::engine::kirby::kirby_hat_cycle::{commit_ends_request, request_frame, REQUEST_FRAMES};

/// HAT-REQUEST-END R1 (operator option A, 2026-11-05): the request's clock runs every game frame (`kirby_hats::tick`),
/// so a request whose role's Kirby is never served still ends after 900 frames with "Kirby hat could not be loaded".
/// A save state restoring it holds the clock.
fn a_request_no_kirby_serves_ends_after_its_frames() {
    let mut age = 0;
    for frame in 1..=REQUEST_FRAMES {
        let (next, ended) = request_frame(age, false);
        assert_eq!((next, ended), (frame, false), "frame {frame}: still pending");
        age = next;
    }
    assert_eq!(request_frame(age, false), (REQUEST_FRAMES + 1, true), "frame 901 ends it");
    assert_eq!(request_frame(age, true), (age, false), "a restoring save state holds the clock");
    assert_eq!(request_frame(u16::MAX, false), (u16::MAX, true), "it can never wrap back to pending");
}

/// HAT-REQUEST-END R2: committing a non-Kirby fighter to the role ends its pending request (`fighter_committed` clears
/// it and releases its lease, as `clear_pending` does); a Kirby commit keeps it.
fn a_role_that_stops_being_kirby_ends_its_request() {
    assert!(!commit_ends_request(0x06), "Kirby keeps the request");
    for kind in [0x00, 0x05, 0x07, 0x4b, 0x58] {
        assert!(commit_ends_request(kind), "kind {kind:#x} ends it");
    }
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_request_no_kirby_serves_ends_after_its_frames", a_request_no_kirby_serves_ends_after_its_frames as fn()),
        ("a_role_that_stops_being_kirby_ends_its_request", a_role_that_stops_being_kirby_ends_its_request as fn()),
    ]);
}
