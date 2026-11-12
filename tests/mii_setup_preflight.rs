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

/// The same owner and record at Phase B: the bank's parked change, the identity installed.
fn phase_b() -> Preflight { Preflight { phase: EntryPhase::BANK_PARKED, seam: Seam::PhaseB, ..good() } }

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

fn the_cpu_shape_uses_its_own_record_and_refuses_the_ordinary_index_zero_claim() {
    // CPU1's assigned record is 1 (`plan2-mii-owner-loading01/REPORT.md` visit 1): the
    // ordinary FE/O index 0 is not evidence of its Mii record, and Player0 names 0.
    let cpu = Preflight {
        owner: 1, tokens: Tokens::from_entry_halfwords([1, 75, 125, 0, 37]), r_index: 1, fe_index: 1,
        other_r_indices: [Some(0), None, None, None, None, None, None, None], ..good()
    };
    assert_eq!(cpu.check(), Ok(()));
    let stale_r = Preflight { r_index: 0, ..cpu };
    assert_eq!(stale_r.check(), Err("Mii setup: R+0x68, FE+0x46 and the target index differ"));
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

fn the_interim_rule_routes_an_unproven_target_canonically_before_any_mutation() {
    assert!(admits_group_mutation(Ok(())));
    assert!(!admits_group_mutation(Err("Mii setup: D rebuild gate set; canonical path owns this owner")));
}

fn m3_phase_b_admits_the_bank_s_parked_change_where_idleness_could_never_hold() {
    // Surface s2 #175/#380 (M4b diag `c3f6951f…`): Phase B runs only after
    // `awaiting_completion` proved 3/0xc/0xff, changing 1, guard 0 and `+0x5933 == 0`
    // (the commit `0x6581f0` on the bank's own `+0x591a = via`). The old rule required
    // 6/14 there and refused every bank-built Mii group with "native loader not idle".
    let p = phase_b();
    assert_eq!(p.check(), Ok(()));
    assert_eq!(EntryPhase::BANK_PARKED, EntryPhase { outer: 3, loader: 0xc, pending: 0xff, changing: 1, guard: 0, acknowledged: 0 });
    let refused = Err("Mii setup: entry not parked in the bank's own change (3/0xc)");
    // Idle at Phase B is not the bank's change: nothing parked, nothing to build into.
    assert_eq!(Preflight { phase: EntryPhase::READY, ..p }.check(), refused);
    // Native's own state 6 is native's loader running (a Reload), never the bank's seam.
    assert_eq!(Preflight { phase: EntryPhase::RELOAD_STATE6, ..p }.check(), refused);
    // Acknowledged: native activation may already run; a queued request; a guard; a native
    // Reload written over the parked change (`0x653240`: loader 0, outer 3).
    let parked = EntryPhase::BANK_PARKED;
    for phase in [EntryPhase { acknowledged: 1, ..parked }, EntryPhase { pending: 0, ..parked },
        EntryPhase { guard: 1, ..parked }, EntryPhase { changing: 0, ..parked }, EntryPhase { loader: 0, ..parked },
        EntryPhase { outer: 6, ..parked }, EntryPhase { loader: 0xb, ..parked }] {
        assert_eq!(Preflight { phase, ..p }.check(), refused, "{phase:?}");
    }
    // Plan time: Ready only (the bank's `switch` accepts nothing else), whatever `+0x5933`
    // the previous change left.
    assert_eq!(good().check(), Ok(()));
    assert_eq!(Preflight { phase: EntryPhase { acknowledged: 0, ..EntryPhase::READY }, ..good() }.check(), Ok(()));
    for phase in [EntryPhase::BANK_PARKED, EntryPhase::RELOAD_STATE6, EntryPhase { pending: 2, ..EntryPhase::READY },
        EntryPhase { changing: 1, ..EntryPhase::READY }, EntryPhase { guard: 1, ..EntryPhase::READY }] {
        assert_eq!(Preflight { phase, ..good() }.check(), Err("Mii setup: entry not Ready (plan needs loader 6/14, no request)"), "{phase:?}");
    }
    // The ELF facts the phases encode.
    assert_eq!((COMMIT_CHANGE, RELOAD_LOADER, STATE6_BIND_CALL), (0x6581f0, 0x650010, 0x651444));
    assert_eq!(CHAIN_WORDS[10] & 0xfc00_0000, 0x9400_0000, "state 6 calls 0x67e7d0 itself");
    assert_eq!((FE_OUTER, FE_LOADER, FE_PENDING, FE_CHANGING, FE_GUARD, FE_ACKNOWLEDGED),
        (0x5920, 0x591c, 0x591a, 0x5934, 0x5935, 0x5933));
}

fn plan_and_phase_b_agree_on_every_clause_but_the_phase() {
    // What plan time admits, Phase B admits, and each refusal names the same clause, so a
    // Mii target the plan could have refused is never refused after the window detaches.
    let variants: [fn(Preflight) -> Preflight; 13] = [
        |p| p,
        |p| Preflight { fingerprints: false, ..p },
        |p| Preflight { kind: 0x4b, ..p },
        |p| Preflight { owner: 8, ..p },
        |p| Preflight { manager_present: false, ..p },
        |p| Preflight { r_present: false, ..p },
        |p| Preflight { record_count: 0, ..p },
        |p| Preflight { d_gate: 1, ..p },
        |p| Preflight { r_index: 5, ..p },
        |p| Preflight { fe_index: 5, ..p },
        |p| Preflight { other_r_indices: [None, Some(0), None, None, None, None, None, None], ..p },
        |p| Preflight { charinfo_published: false, ..p },
        |p| Preflight { kind: 0x48, tokens: Tokens { index: 132, ..p.tokens }, r_index: 132, fe_index: 132, ..p },
    ];
    for (i, v) in variants.iter().enumerate() {
        assert_eq!(v(good()).check(), v(phase_b()).check(), "variant {i}");
    }
    assert!(admits_group_mutation(good().check()) && phase_b().check().is_ok());
}

fn tokens_keep_the_native_halfword_order_and_the_fingerprints_are_the_listed_prologues() {
    let t = Tokens::from_entry_halfwords([3, 75, 125, 0, 37]);
    assert_eq!(t.halfwords(), [3, 75, 125, 0, 37]);
    assert_eq!(std::mem::size_of::<Tokens>(), 10);
    // Every called function starts with its `sub sp, sp, #imm` prologue word.
    assert_eq!(BIND_WORDS[0], 0xd10483ff);
    assert_eq!(FACES_WORDS[0], 0xd10343ff);
    assert_eq!(PUBLISH_WORDS[0], 0xd10743ff);
    // The chain ends in `bl 0x67e7d0` from `0x651444`: imm26 = (0x67e7d0 - 0x651444) / 4.
    let imm26 = (BIND_MII_RESOURCE as i64 - 0x651444i64) / 4;
    assert_eq!(CHAIN_WORDS[10], 0x9400_0000 | (imm26 as u32 & 0x03ff_ffff));
    assert_eq!(PAIR_STORE - PUBLISH_PAIR, 0x14ec);
    assert_eq!(CHAIN_GATE + 3 * 4, MANAGER_CHAIN);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_canonical_shape_passes_and_every_clause_fails_closed_with_its_reason", the_canonical_shape_passes_and_every_clause_fails_closed_with_its_reason as fn()),
        ("the_cpu_shape_uses_its_own_record_and_refuses_the_ordinary_index_zero_claim", the_cpu_shape_uses_its_own_record_and_refuses_the_ordinary_index_zero_claim as fn()),
        ("outcomes_require_the_pair_and_an_intact_payload_and_report_faces", outcomes_require_the_pair_and_an_intact_payload_and_report_faces as fn()),
        ("the_interim_rule_routes_an_unproven_target_canonically_before_any_mutation", the_interim_rule_routes_an_unproven_target_canonically_before_any_mutation as fn()),
        ("m3_phase_b_admits_the_bank_s_parked_change_where_idleness_could_never_hold", m3_phase_b_admits_the_bank_s_parked_change_where_idleness_could_never_hold as fn()),
        ("plan_and_phase_b_agree_on_every_clause_but_the_phase", plan_and_phase_b_agree_on_every_clause_but_the_phase as fn()),
        ("tokens_keep_the_native_halfword_order_and_the_fingerprints_are_the_listed_prologues", tokens_keep_the_native_halfword_order_and_the_fingerprints_are_the_listed_prologues as fn()),
    ]);
}
