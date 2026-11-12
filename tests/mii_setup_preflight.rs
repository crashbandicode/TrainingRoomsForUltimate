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

// Value rules of the native Mii-setup adapter (WP-M2 step 2): the preflight that must
// pass before `0x67e7d0`/`0x67ead0` run for a bank-built Mii group, the outcome
// classification after them, and the interim canonical-before-mutation rule.
use crate::engine::mii::mii_setup::*;

fn good() -> Preflight {
    Preflight {
        fingerprints: true, owner: 0, kind: 0x4a,
        tokens: Tokens::from_entry_halfwords([0, 75, 125, 0, 37]),
        manager_present: true, record_count: 133, r_present: true, r_index: 0, fe_index: 0, d_gate: 0,
        other_r_indices: [None, Some(1), None, None, None, None, None, None],
        phase: EntryPhase::READY, seam: Seam::Plan, charinfo_published: true,
    }
}

fn the_canonical_shape_passes_and_every_clause_fails_closed_with_its_reason() {
    let g = good();
    assert_eq!(g.check(), Ok(()));
    assert_eq!(Preflight { fingerprints: false, ..g }.check(), Err("Mii setup: native fingerprints differ"));
    for kind in [0x47u32, 0x4b, 0x3, 0x4e] {
        assert_eq!(Preflight { kind, ..g }.check(), Err("Mii setup: kind is not a Mii class"), "kind {kind:#x}");
    }
    for kind in [0x48u32, 0x49, 0x4a] { assert_eq!(Preflight { kind, ..g }.check(), Ok(())); }
    assert_eq!(Preflight { owner: 8, ..g }.check(), Err("Mii setup: owner outside the native owner table"));
    assert_eq!(Preflight { manager_present: false, ..g }.check(), Err("Mii setup: manager singleton absent"));
    assert_eq!(Preflight { r_present: false, ..g }.check(), Err("Mii setup: resource owner absent"));
    assert_eq!(Preflight { record_count: 0, ..g }.check(), Err("Mii setup: record index outside the selector vector"));
    let last = Tokens { index: 132, ..g.tokens };
    assert_eq!(Preflight { tokens: last, r_index: 132, fe_index: 132, ..g }.check(), Ok(()));
    let past = Tokens { index: 133, ..g.tokens };
    assert_eq!(Preflight { tokens: past, r_index: 133, fe_index: 133, ..g }.check(),
        Err("Mii setup: record index outside the selector vector"));
    assert_eq!(Preflight { phase: EntryPhase::BANK_PARKED, ..g }.check(),
        Err("Mii setup: entry not Ready (plan needs loader 6/14, no request)"));
    assert_eq!(Preflight { d_gate: 1, ..g }.check(), Err("Mii setup: D rebuild gate set; canonical path owns this owner"));
    // The destructive precondition: `0x67e7d0` retires the record at `R+0x68`, then
    // publishes at `tokens[0]`; both must be this owner's assigned record (and FE agrees).
    assert_eq!(Preflight { r_index: 1, ..g }.check(), Err("Mii setup: R+0x68, FE+0x46 and the target index differ"));
    assert_eq!(Preflight { fe_index: 1, ..g }.check(), Err("Mii setup: R+0x68, FE+0x46 and the target index differ"));
    // Another live owner naming the target record would lose its pair to the retirement.
    let mut claimed = g.other_r_indices;
    claimed[4] = Some(0);
    assert_eq!(Preflight { other_r_indices: claimed, ..g }.check(), Err("Mii setup: another owner's resource names the target record"));
    // The owner's own entry in the table is ignored (it is `r_index`).
    let mut own = g.other_r_indices;
    own[0] = Some(0);
    assert_eq!(Preflight { other_r_indices: own, ..g }.check(), Ok(()));
    assert_eq!(Preflight { charinfo_published: false, ..g }.check(),
        Err("Mii setup: record CharInfo is not the published configuration"));
}

fn outcomes_require_the_pair_and_an_intact_payload_and_report_faces() {
    assert_eq!(classify((0x11f3e48000, 0x11f3e48100), (7, 8), true),
        Outcome::Prepared { pair: (0x11f3e48000, 0x11f3e48100), faces: (7, 8) });
    assert_eq!(classify((0, 0x11f3e48100), (7, 8), true), Outcome::Failed("Mii setup: native resource pair absent after setup"));
    assert_eq!(classify((0x11f3e48000, 0), (7, 8), true), Outcome::Failed("Mii setup: native resource pair absent after setup"));
    assert_eq!(classify((0x11f3e48000, 0x11f3e48100), (0, 0), true),
        Outcome::Prepared { pair: (0x11f3e48000, 0x11f3e48100), faces: (0, 0) });
    assert_eq!(classify((0x11f3e48000, 0x11f3e48100), (7, 8), false), Outcome::Failed("Mii setup: record CharInfo changed under setup"));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_canonical_shape_passes_and_every_clause_fails_closed_with_its_reason", the_canonical_shape_passes_and_every_clause_fails_closed_with_its_reason as fn()),
        ("outcomes_require_the_pair_and_an_intact_payload_and_report_faces", outcomes_require_the_pair_and_an_intact_payload_and_report_faces as fn()),
    ]);
}
