//! Original probes for the Local Wireless round-handshake policy, through its
//! real entry points. Oracle: run124's three guest samples while the editor
//! owned the scene (ack 0,1,1 / resolved 0,1,1, deque unchanged) and the
//! audited native consumer predicate. Addresses are identity tokens only.
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[allow(dead_code, unused_imports)]
#[path = "../src/model/mii/mod.rs"]
pub mod mii;
#[allow(unused_imports)]
mod engine {
    pub use super::mii;
}

use crate::engine::mii::ldn_handshake::{
    interruption, Handshake, Identity, Interruption, Observation, Sample,
};

const SERVICE: usize = 0x0011_dc81_0000;
const GENERATION: u64 = 7;

fn sample(service: usize, generation: u64, ack: u32, resolved: u8) -> Sample {
    Sample { identity: Some(Identity { service, generation }), ack, resolved }
}

fn guest(ack: u32, resolved: u8) -> Sample { sample(SERVICE, GENERATION, ack, resolved) }

fn unbound(ack: u32, resolved: u8) -> Sample { Sample { identity: None, ack, resolved } }

fn abort_is_never_begin_and_does_not_disarm_the_session() {
    let mut handshake = Handshake::new();
    // Resolved with an empty mask before any baseline: stale, not an abort edge.
    assert_eq!(handshake.observe(guest(0, 1)), Observation::StaleHigh);
    assert!(guest(0, 1).aborted() && !guest(0, 1).committed());
    assert_eq!(handshake.observe(guest(0, 0)), Observation::Baseline);
    assert_eq!(handshake.observe(guest(0, 1)), Observation::Aborted);
    assert_eq!(handshake.observe(guest(0, 1)), Observation::Aborted);
    assert!(handshake.armed() && !handshake.fired());
    // The round resets and resolves again with an acknowledgement.
    assert_eq!(handshake.observe(guest(0, 0)), Observation::Waiting);
    assert_eq!(handshake.observe(guest(1, 1)), Observation::Committed);
    // A sequential read that saw the resolved byte before the mask must not
    // permanently disarm a session whose next read shows the committed round.
    let mut racing = Handshake::new();
    assert_eq!(racing.observe(guest(0, 0)), Observation::Baseline);
    assert_eq!(racing.observe(guest(0, 1)), Observation::Aborted);
    assert_eq!(racing.observe(guest(1, 1)), Observation::Committed);
}

fn arm_records_only_a_genuine_zero_baseline_and_never_fires() {
    let mut handshake = Handshake::new();
    assert!(!handshake.arm(guest(1, 1)));
    assert!(!handshake.arm(guest(0, 1)));
    assert!(!handshake.arm(unbound(0, 0)));
    assert!(!handshake.armed());
    // An acknowledged but unresolved round is still a resolved-zero baseline.
    assert!(handshake.arm(guest(1, 0)));
    assert!(handshake.armed() && !handshake.fired());
    // Arming again with a high reading keeps the existing baseline.
    assert!(handshake.arm(guest(1, 1)));
    assert!(handshake.armed());
    assert_eq!(handshake.observe(guest(1, 1)), Observation::Committed);
    // A fired detector never re-arms.
    assert!(!handshake.arm(guest(0, 0)));
    assert!(handshake.fired());
}

fn queued_match_and_remote_join_keep_priority_over_the_handshake() {
    let mut handshake = Handshake::new();
    assert_eq!(handshake.observe(guest(0, 0)), Observation::Baseline);
    // A queued match-start (kind 6) already pending wins even on the committed frame.
    assert_eq!(interruption(true, Some(6), &mut handshake, guest(1, 1)), Some(Interruption::Network(6)));
    // A remote Join notice with its sender slot likewise keeps the existing path.
    let remote_join = 1 | (3_u64 << 48);
    let mut joined = Handshake::new();
    assert_eq!(joined.observe(guest(0, 0)), Observation::Baseline);
    assert_eq!(interruption(true, Some(remote_join), &mut joined, guest(0, 0)),
        Some(Interruption::Network(remote_join)));
    // Nothing queued and the round unresolved: the editor keeps running.
    let mut quiet = Handshake::new();
    assert_eq!(interruption(true, Some(0), &mut quiet, guest(0, 0)), None);
    assert_eq!(interruption(true, Some(0), &mut quiet, guest(1, 0)), None);
    // Nothing queued and the round committed: the typed handshake interruption, once.
    assert_eq!(interruption(true, Some(0), &mut quiet, guest(1, 1)), Some(Interruption::Handshake));
    assert_eq!(interruption(true, Some(0), &mut quiet, guest(1, 1)), None);
    // An abort with nothing queued never interrupts.
    let mut aborted = Handshake::new();
    assert_eq!(interruption(true, Some(0), &mut aborted, guest(0, 0)), None);
    assert_eq!(interruption(true, Some(0), &mut aborted, guest(0, 1)), None);
    // No service at all: no decision and no baseline.
    let mut missing = Handshake::new();
    assert_eq!(interruption(true, None, &mut missing, unbound(1, 1)), None);
    assert!(!missing.armed());
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("abort_is_never_begin_and_does_not_disarm_the_session", abort_is_never_begin_and_does_not_disarm_the_session as fn()),
        ("arm_records_only_a_genuine_zero_baseline_and_never_fires", arm_records_only_a_genuine_zero_baseline_and_never_fires as fn()),
        ("queued_match_and_remote_join_keep_priority_over_the_handshake", queued_match_and_remote_join_keep_priority_over_the_handshake as fn()),
    ]);
}
