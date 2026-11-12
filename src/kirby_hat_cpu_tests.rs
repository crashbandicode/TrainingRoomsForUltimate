//! B4-3 (Surface lane `artifacts/phase23/LANE-SURFACE-REPORT.md` "B4-3 root cause"; Ally lane `LANE-ALLY-REPORT.md`
//! "Round 4" §3): the CPU Kirby hat request stuck forever because `kirby_hats::on_frame` ran only with Player 1's
//! body. It now runs once per Kirby with that Kirby's own body (`state_kirby_opff_hook`, the fork's call site);
//! the role match is `hat_role`. The call-site wiring itself is checked in `tests/kirby_hat_cpu.rs`.

use crate::engine::kirby::kirby_hat_cycle::{commit_ends_request, hat_role, request_frame, ShortcutLatch, REQUEST_FRAMES};

/// Stand-ins for two module accessors: equality is all `hat_role` reads.
const P1_KIRBY: usize = 0x1000;
const CPU_KIRBY: usize = 0x2000;

/// The right-click chord requests role 1 (CPU) when the CPU's body is a Kirby, and the CPU Kirby's own frame
/// callback then serves that role: the request, then the role-1 match that lets the pending request age and start.
/// P1's Kirby still serves role 0.
fn a_cpu_kirby_body_serves_the_cpu_request_and_p1_still_serves_its_own() {
    let roles = [Some(P1_KIRBY), Some(CPU_KIRBY)];
    let mut latch = ShortcutLatch::new();
    let (request, suppress) = latch.sample(true, true, false, true, roles.map(|r| r.is_some()));
    assert_eq!(request, Some(1), "Special + right stick click requests the CPU's hat");
    assert!(suppress);
    assert_eq!(hat_role(CPU_KIRBY, roles), Some(1), "the CPU Kirby's own callback serves role 1");
    let mut latch = ShortcutLatch::new();
    assert_eq!(latch.sample(true, true, true, false, [true, true]).0, Some(0), "Special + left click requests P1's");
    assert_eq!(hat_role(P1_KIRBY, roles), Some(0), "P1's Kirby still serves role 0");
    // A body that is neither role's Kirby (a helper, or a role that is not a Kirby) serves no request.
    assert_eq!(hat_role(0x3000, roles), None);
    assert_eq!(hat_role(CPU_KIRBY, [Some(P1_KIRBY), None]), None, "the CPU is not a Kirby");
    // The defect, as wiring: one callback per frame with Player 1's body never reached role 1; one per Kirby does.
    let served = |bodies: &[usize]| bodies.iter().filter_map(|b| hat_role(*b, roles)).collect::<Vec<_>>();
    assert_eq!(served(&[P1_KIRBY]), [0]);
    assert_eq!(served(&[P1_KIRBY, CPU_KIRBY]), [0, 1]);
}

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

/// A normally progressing request is unchanged: the old clock advanced once per `on_frame` call on the role's own
/// Kirby, which runs once per game frame, so a request served every frame has the same age frame by frame and is
/// confirmed long before the timeout, as before.
fn a_request_served_every_frame_ages_as_before() {
    let (mut old, mut new) = (0u16, 0u16);
    for _ in 0..120 {
        old = old.saturating_add(1); // the old rule: one step per served on_frame
        let (age, ended) = request_frame(new, false);
        assert!(!ended);
        new = age;
        assert_eq!(new, old);
    }
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
        ("a_cpu_kirby_body_serves_the_cpu_request_and_p1_still_serves_its_own", a_cpu_kirby_body_serves_the_cpu_request_and_p1_still_serves_its_own as fn()),
        ("a_request_no_kirby_serves_ends_after_its_frames", a_request_no_kirby_serves_ends_after_its_frames as fn()),
        ("a_request_served_every_frame_ages_as_before", a_request_served_every_frame_ages_as_before as fn()),
        ("a_role_that_stops_being_kirby_ends_its_request", a_role_that_stops_being_kirby_ends_its_request as fn()),
    ]);
}
