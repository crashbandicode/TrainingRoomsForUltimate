//! Live-consumer occupancy: who else uses a kind on a role's resource owner, and the conflict a
//! planned acquisition, release, build or target raises (WORKPLAN §2.5; SYNTHESIS agree-5; Fable
//! REDESIGN §2.3 graded rules; Astra REDESIGN §4.3 drain/suspend invariant; D-KIRBY1-ROB1-RCA §3;
//! D-HANG1-SEPARATION §1/§4). Values only; the adapter samples a [`KindOccupancy`] in the current
//! callback and hands it, with the [`Rules`], to the ledger as its `LiveConsumers`.
//!
//! Rules and their grades (a HYP rule is relaxed by a probe, never by a stall):
//! - `LiveCopySharesRegistry` (FACT, K1 §3.2–3.3): a Kirby copy registers the copied kind on
//!   Kirby's *own* R, and `0x17e8610(R, k)` with `R+0x4c` bit 2 releases R's G68 mirror for
//!   `(k, costume)`; acquiring or releasing the copied kind on that R is refused.
//! - `PrefetchDebtUnderLiveKirby` (FACT K1 §0/§5.1, Astra §4.3): no bank prefetch key may rest on
//!   R while a member whose row has `prefetch_while_live: false` (Kirby) is live — *suspend*:
//!   no acquisition while it is live; *drain*: a transition whose target is such a member must
//!   first return every prefetched key (the planner drains Prefetched groups to Planned before the
//!   commit, so a freshly live Kirby, hatless by `kirby_park_gate`, finds an empty debt).
//! - `HatSourceLive` (HYP, K1 §6 K2): a hat-source kind built into the incoming window while a copy
//!   exists; conservative on, inert until the adapter supplies hat-source kinds.
//! - `OtherRoleLive` (disproved for the mirror shape: D-HANG1-SEPARATION §1 S-A 3/3 completed; the
//!   hang is the parked never-activated Kirby, WP3/WP-K): off in `Rules::C116_1`.

use super::capabilities::{capabilities, form_class, Capabilities};
#[cfg(any(test, feature = "simulator"))]
use super::capabilities::{can_window_member_of, member_forms, pins_cell0, FormClass, Grouping};
use super::ledger::{Conflict, KindSet64, LiveConsumers, ReleaseFootprint, ResidentDependencies, Role, Use, NO_KIND};
#[cfg(any(test, feature = "simulator"))]
use super::ledger::{native_window_requests, orphan_keys, pair_rule, NativeCall, RFlags};
use crate::engine::swap::character_forms::Forms;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Fact,
    Hyp,
}

/// Which rules are on, and at what grade. `None` = off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    pub live_copy: Grade,
    pub prefetch_debt_under_live_kirby: Grade,
    pub hat_source: Option<Grade>,
    pub other_role_live: Option<Grade>,
}

impl Rules {
    /// C116.1: the two FACT rules, the K2 hypothesis conservative, the mirror rule off (disproved).
    pub const C116_1: Rules = Rules {
        live_copy: Grade::Fact,
        prefetch_debt_under_live_kirby: Grade::Fact,
        hat_source: Some(Grade::Hyp),
        other_role_live: None,
    };
}

impl Default for Rules {
    fn default() -> Self { Rules::C116_1 }
}

/// A value snapshot of every consumer of kinds in the scene, per role (index 0 Player, 1 CPU).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KindOccupancy {
    /// Each role's live kind (the body's `+0xc`, never `R+0x58`).
    pub live: [Option<u32>; 2],
    /// Bodies parked in the window or in Built groups, per role.
    pub parked: [KindSet64; 2],
    /// Kinds whose bank prefetch keys rest on the role's R (groups that `holds_keys()`).
    pub prefetched: [KindSet64; 2],
    /// `INT_COPY_CHARA` of a live Kirby with `FLAG_COPY`, per role.
    pub kirby_copy: [Option<u32>; 2],
    /// Helper slots leased as hat sources (`Consumer::Hat`); empty until the sources expose them.
    pub hat_sources: KindSet64,
    /// Warm helper slots; informational.
    pub helpers: KindSet64,
    /// A parked Kirby that was constructed and never activated (D-HANG1 S-B shape; WP3's destroy
    /// arm reads it, WP-K's K-R1 decides the native side).
    pub never_activated_kirby: [bool; 2],
}

fn kind_row(kind: u32) -> Capabilities { capabilities(form_class(kind, Forms::starting(kind))) }

impl KindOccupancy {
    /// The role's live member suspends bank prefetch (its row: `prefetch_while_live: false`).
    pub fn prefetch_suspended(&self, role: Role) -> bool {
        self.live[role.index()].is_some_and(|kind| !kind_row(kind).prefetch_while_live)
    }

    /// Prefetch debt that must be drained before `target` may become live.
    pub fn debt_blocks_target(&self, role: Role, target: u32) -> bool {
        !kind_row(target).prefetch_while_live && !self.prefetched[role.index()].is_empty()
    }

    /// The conflict `usage` of `kind` by `role` raises, if any (first matching rule, FACT rules first).
    pub fn conflicts(&self, role: Role, kind: u32, usage: Use, rules: &Rules) -> Option<Conflict> {
        let i = role.index();
        if self.kirby_copy[i] == Some(kind) && matches!(usage, Use::Acquire | Use::Release) {
            return Some(Conflict::LiveCopySharesRegistry);
        }
        if usage == Use::Acquire && self.prefetch_suspended(role) {
            return Some(Conflict::PrefetchDebtUnderLiveKirby);
        }
        if usage == Use::TargetKind && self.debt_blocks_target(role, kind) {
            return Some(Conflict::PrefetchDebtUnderLiveKirby);
        }
        if rules.hat_source.is_some() && usage == Use::Build && self.kirby_copy[i].is_some() && self.hat_sources.contains(kind) {
            return Some(Conflict::HatSourceLive);
        }
        if rules.other_role_live.is_some() && matches!(usage, Use::Build | Use::TargetKind) && self.live[1 - i] == Some(kind) {
            return Some(Conflict::OtherRoleLive);
        }
        None
    }
}

/// The ledger's `LiveConsumers`: an occupancy snapshot under a rule set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Occupancy {
    pub kinds: KindOccupancy,
    pub rules: Rules,
}

impl LiveConsumers for Occupancy {
    fn conflict(&self, role: Role, kind: u32, usage: Use) -> Option<Conflict> {
        self.kinds.conflicts(role, kind, usage, &self.rules)
    }
}

// ----- Kirby residency bookkeeping (WORKPLAN §2.14 step 2; D-HANG1 ≡ D-KIRBY1 as one class) ------

/// The G68 `(kind, costume)` registry records a Kirby body of a role references through its copy:
/// the copied kind of a live copy (FACT K1 §3.3: the copy registers it on Kirby's own R). A hatless
/// Kirby, parked or live, references none (`prepare_tuple`/`release_tuple` return early for kind 6,
/// K1 §3.1); a parked never-activated Kirby's copy-tuple *list* is the destroy-side hazard WP3's
/// `DestroyPlan` carries (K-R1 decides the native initialiser).
pub fn kirby_registry_of(copy: Option<u32>) -> KindSet64 {
    let mut set = KindSet64::default();
    if let Some(kind) = copy { set.insert(kind); }
    set
}

/// What a role's resident bodies still depend on in R after their keys are gone: every parked and
/// live body kind (the NRO tree, GROUP-SWAP-IMPL §14.3) plus the Kirby copy's kind (its registry
/// record).
pub fn resident_dependencies(body_kinds: impl IntoIterator<Item = u32>, kirby_copy: Option<u32>) -> ResidentDependencies {
    let mut kinds = KindSet64::default();
    for kind in body_kinds {
        if kind != NO_KIND { kinds.insert(kind); }
    }
    for kind in kirby_registry_of(kirby_copy).iter() { kinds.insert(kind); }
    ResidentDependencies { kinds }
}

/// A release footprint is admitted only when none of the registry records it would touch is one a
/// resident Kirby copy references (K1 §3.2: `0x17e8610` releases R's mirror for `(kind, costume)`).
pub fn footprint_admitted(footprint: &ReleaseFootprint, kirby_registry: KindSet64) -> Result<(), Conflict> {
    if footprint.kind_registry.iter().any(|record| kirby_registry.contains(record.kind)) {
        Err(Conflict::LiveCopySharesRegistry)
    } else {
        Ok(())
    }
}

// ----- the ledger-driven policy for the replay harness (WP2's `native_model::Policy`) ------------

/// What the bank does once its acquisitions and releases go through the ledger with these rules:
/// the "new policy" of the fixture replays (WORKPLAN §2.5 tests). `kirby` is the Kirby row in force
/// (the shipped one by default; the §5.3 fallback row flips it), so the K-row decision is replayed
/// on the model before Eden.
#[cfg(any(test, feature = "simulator"))]
#[derive(Clone, Copy, Debug)]
pub struct LedgerPolicy {
    pub kirby: Capabilities,
    pub rules: Rules,
}

#[cfg(any(test, feature = "simulator"))]
impl Default for LedgerPolicy {
    fn default() -> Self { LedgerPolicy { kirby: capabilities(FormClass::Kirby), rules: Rules::C116_1 } }
}

#[cfg(any(test, feature = "simulator"))]
impl LedgerPolicy {
    fn row(&self, kind: u32) -> Capabilities {
        let class = form_class(kind, Forms::starting(kind));
        if class == FormClass::Kirby { self.kirby } else { capabilities(class) }
    }

    /// The row of a window member at cells 1–2: its single-body member form (Solo for `0x4b`; the
    /// pair is never a member there).
    fn member_row(&self, kind: u32) -> Capabilities {
        let class = form_class(kind, member_forms(kind));
        if class == FormClass::Kirby { self.kirby } else { capabilities(class) }
    }
}

#[cfg(any(test, feature = "simulator"))]
impl super::native_model::Policy for LedgerPolicy {
    fn name(&self) -> &'static str { "ledger" }

    /// The window the planner builds: cell 0 is the target; cells 1–2 keep a member only when its
    /// row admits that cell (`can_window_member_of`). The pair's Nana and the whole native groups
    /// are kept as native shapes.
    fn plan_window(&self, requested: [u32; 3]) -> [u32; 3] {
        let mut out = requested;
        if requested[0] == NO_KIND { return out; }
        if matches!(self.row(requested[0]).grouping, Grouping::OwnGroupOnly(_)) { return out; }
        let pair = pair_rule(requested);
        for (cell, slot) in out.iter_mut().enumerate().skip(1) {
            let kind = *slot;
            if kind == NO_KIND || (cell == 1 && pair) { continue; }
            if !can_window_member_of(&self.member_row(kind), cell) { *slot = NO_KIND; }
        }
        // Layout (`CellRule::PinCell0`, `Members::laid_out`): the first pinned member (Kirby, Solo
        // Popo) takes cell 0; a later pinned one waits for the next window (`Members::from_forms`);
        // the others keep their order. The target then activates through its own cell.
        let mut pinned = None;
        for (cell, slot) in out.iter_mut().enumerate() {
            if *slot == NO_KIND || !pins_cell0(*slot) { continue; }
            if pinned.is_some() { *slot = NO_KIND; } else { pinned = Some(cell); }
        }
        if let Some(p) = pinned.filter(|p| *p != 0) {
            let kind = out[p];
            out.copy_within(0..p, 1);
            out[0] = kind;
        }
        out
    }

    /// Every window kind native did not request itself (`0x17e5bc0` model) is the hook's, once.
    fn supplemental(&self, cells: [u32; 3], count: usize) -> Vec<u32> {
        let native: Vec<u32> = native_window_requests(cells, RFlags::TRAINING).iter().map(|r| r.kind).collect();
        cells.iter().take(count).copied().filter(|kind| *kind != NO_KIND && !native.contains(kind)).collect()
    }

    fn pair_refs(&self, cells: [u32; 3], count: usize) -> Option<u32> {
        pair_rule(cells).then_some(count as u32)
    }

    fn release_prefetched(&self, kind: u32, flags: RFlags) -> Vec<NativeCall> {
        let mut calls: Vec<NativeCall> = orphan_keys(kind, flags).iter(kind).map(NativeCall::ReleaseKey).collect();
        calls.push(NativeCall::ReleaseKind { kind });
        calls
    }

    fn prefetch_admitted(&self, live: Option<u32>) -> bool {
        let occupancy = KindOccupancy { live: [live, None], ..KindOccupancy::default() };
        occupancy.conflicts(Role::Player, NO_KIND, Use::Acquire, &self.rules).is_none()
    }

    fn returns_keys_on_live_kirby(&self) -> bool { true }

    fn key_release_admitted(&self, kind: u32, copy: Option<u32>) -> bool {
        let occupancy = KindOccupancy { kirby_copy: [copy, None], ..KindOccupancy::default() };
        occupancy.conflicts(Role::Player, kind, Use::Release, &self.rules).is_none()
    }

    fn clears_routes(&self) -> bool { true }

    fn on_prequeue_refusal(&self, plain_kind: bool) -> super::native_model::Refusal {
        if plain_kind { super::native_model::Refusal::NativeContinue } else { super::native_model::Refusal::Epilogue }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::capabilities::{CellRule, Qualification};
    use super::super::ledger::{release_kind_footprint, OwnerState, KIRBY, NANA, POPO};
    use super::super::native_model::{kind_table, parse_fixture, replay, Event, Fault, G68Hazard, OldPolicy, Policy};

    fn occupancy() -> KindOccupancy { KindOccupancy::default() }

    #[test]
    fn conflicts_encode_the_kirby_drain_and_suspend_rules_and_the_live_copy_guard() {
        let rules = Rules::C116_1;
        // D-KIRBY1 S2: Kirby live and copying Mario on P1's R.
        let mut occ = occupancy();
        occ.live[0] = Some(KIRBY);
        occ.kirby_copy[0] = Some(0x00);
        assert_eq!(occ.conflicts(Role::Player, 0x00, Use::Release, &rules), Some(Conflict::LiveCopySharesRegistry));
        assert_eq!(occ.conflicts(Role::Player, 0x00, Use::Acquire, &rules), Some(Conflict::LiveCopySharesRegistry));
        assert_eq!(occ.conflicts(Role::Player, 0x07, Use::Release, &rules), None, "another kind's key may go");
        assert_eq!(occ.conflicts(Role::Player, 0x07, Use::Acquire, &rules), Some(Conflict::PrefetchDebtUnderLiveKirby), "suspend while Kirby is live");
        assert_eq!(occ.conflicts(Role::Cpu, 0x07, Use::Acquire, &rules), None, "the other role's R is not suspended");
        // Drain: a Kirby target with prefetch debt on R conflicts until the debt is returned.
        let mut occ = occupancy();
        occ.live[0] = Some(0x07);
        occ.prefetched[0].insert(0x00);
        assert_eq!(occ.conflicts(Role::Player, KIRBY, Use::TargetKind, &rules), Some(Conflict::PrefetchDebtUnderLiveKirby));
        assert_eq!(occ.conflicts(Role::Player, 0x08, Use::TargetKind, &rules), None, "an ordinary target needs no drain");
        occ.prefetched[0] = KindSet64::default();
        assert_eq!(occ.conflicts(Role::Player, KIRBY, Use::TargetKind, &rules), None);
        assert!(!occ.prefetch_suspended(Role::Player));
        // K2 (HYP): the hat source built into the window while a copy exists.
        let mut occ = occupancy();
        occ.kirby_copy[0] = Some(0x00);
        occ.hat_sources.insert(0x00);
        assert_eq!(occ.conflicts(Role::Player, 0x00, Use::Build, &rules), Some(Conflict::HatSourceLive));
        let relaxed = Rules { hat_source: None, ..rules };
        assert_eq!(occ.conflicts(Role::Player, 0x00, Use::Build, &relaxed), None);
        // The mirror rule is off: D-HANG1-SEPARATION S-A completed 3/3.
        let mut occ = occupancy();
        occ.live[1] = Some(0x1d);
        assert_eq!(occ.conflicts(Role::Player, 0x1d, Use::TargetKind, &rules), None);
        let mirror = Rules { other_role_live: Some(Grade::Hyp), ..rules };
        assert_eq!(occ.conflicts(Role::Player, 0x1d, Use::TargetKind, &mirror), Some(Conflict::OtherRoleLive));
        // Through the ledger's trait.
        let consumers = Occupancy { kinds: KindOccupancy { live: [Some(KIRBY), None], ..occupancy() }, rules };
        assert_eq!(consumers.conflict(Role::Player, 0x07, Use::Acquire), Some(Conflict::PrefetchDebtUnderLiveKirby));
    }

    #[test]
    fn footprint_admission_keeps_a_copied_kinds_registry_and_residents_name_the_copy() {
        let state = OwnerState { task_pending: false, loads_completed: true, costume: 0, map_nodes: 2 };
        let footprint = release_kind_footprint(0x00, RFlags::TRAINING, state, 2);
        assert_eq!(footprint_admitted(&footprint, kirby_registry_of(Some(0x00))), Err(Conflict::LiveCopySharesRegistry));
        assert_eq!(footprint_admitted(&footprint, kirby_registry_of(Some(0x07))), Ok(()));
        assert_eq!(footprint_admitted(&footprint, kirby_registry_of(None)), Ok(()));
        let not_loaded = release_kind_footprint(0x00, RFlags::TRAINING, OwnerState { loads_completed: false, ..state }, 2);
        assert_eq!(footprint_admitted(&not_loaded, kirby_registry_of(Some(0x00))), Ok(()), "no mirror release before the loads completed");
        let deps = resident_dependencies([0x07, KIRBY, 0x08, NO_KIND], Some(0x00));
        assert_eq!(deps.kinds.iter().collect::<Vec<_>>(), vec![0x00, KIRBY, 0x07, 0x08]);
        assert!(kirby_registry_of(None).is_empty());
    }

    /// The `body_bank` wrappers WP4 re-pointed at the ledger data equal the bodies they replaced
    /// (kept here as the oracle) on every window shape the adapters ask about; the one intended
    /// difference is named.
    #[test]
    fn consts_wrappers_equal_the_old_bodies_they_now_derive_from_the_ledger() {
        use crate::engine::swap::body_bank::{orphan_object_keys, pair_resource_references, supplemental_acquisitions, Members, Special, AEGIS_OBJECT_KIND, NATIVE_MEMBERS};
        fn old_supplemental(members: Members) -> ([u32; NATIVE_MEMBERS], usize) {
            let first_extra = if members.kinds()[0] == 0x4b { 2 } else { 1 };
            let mut kinds = [u32::MAX; NATIVE_MEMBERS];
            let mut count = 0;
            for kind in members.kinds().iter().take(members.count()).skip(first_extra) {
                kinds[count] = *kind;
                count += 1;
            }
            (kinds, count)
        }
        fn old_pair_refs(members: Members) -> Option<u32> { members.is_pair().then_some(members.count() as u32) }
        fn old_orphans(kind: u32) -> ([u32; 2], usize) {
            match kind { AEGIS_OBJECT_KIND => ([AEGIS_OBJECT_KIND * 42 + 6, u32::MAX], 1), _ => ([u32::MAX; 2], 0) }
        }
        let singles = [0x00u32, 0x02, 0x03, 0x06, 0x07, 0x08, 0x2d];
        let mut shapes = Vec::new();
        // Rotations are laid out (`Members::laid_out`, CellRule::PinCell0: Kirby to cell 0); the pair-lead
        // window never takes Kirby as its extra.
        for &a in &singles {
            shapes.push(Members::laid_out(&[a]).unwrap());
            for &b in &singles {
                if b == a { continue; }
                shapes.push(Members::laid_out(&[a, b]).unwrap());
                for &c in &singles {
                    if c == a || c == b { continue; }
                    shapes.push(Members::laid_out(&[a, b, c]).unwrap());
                }
            }
            if let Some(lead) = Members::pair_lead(a) { shapes.push(lead); }
        }
        shapes.push(Members::new(&[POPO]).unwrap());
        shapes.push(Members::from_forms(Forms::solo_popo(), &[0x02], 1).unwrap());
        shapes.push(Members::from_forms(Forms::solo_popo(), &[0x02, 0x07], 2).unwrap());
        shapes.push(Members::pair_group());
        for window in shapes {
            assert_eq!(supplemental_acquisitions(window), old_supplemental(window), "{window:?}");
            assert_eq!(pair_resource_references(window), old_pair_refs(window), "{window:?}");
        }
        // The intended difference: a native group's window is requested whole by `0x17e5bc0`
        // (members and object kind), so nothing is supplemental; the old rule would have requested
        // the second and third forms again (never reached: a native group is never a window build).
        for group in [Members::native_group(0x24).unwrap(), Members::native_group(0x5a).unwrap()] {
            assert_eq!(supplemental_acquisitions(group), ([u32::MAX; NATIVE_MEMBERS], 0), "{group:?}");
            assert_eq!(pair_resource_references(group), old_pair_refs(group));
            assert!(matches!(group.special(), Special::Native(_)));
        }
        for kind in (0u32..0x80).chain([u32::MAX]) {
            assert_eq!(orphan_object_keys(kind), old_orphans(kind), "{kind:#x}");
        }
    }

    fn fixture(text: &str, name: &str) -> Vec<Event> {
        parse_fixture(text).unwrap_or_else(|e| panic!("{}: {}", name, e))
    }

    fn real_faults(faults: Vec<Fault>, name: &str, policy: &str) -> Vec<Fault> {
        let bad: Vec<&Fault> = faults.iter().filter(|f| matches!(f, Fault::Expectation { .. } | Fault::Malformed { .. })).collect();
        assert!(bad.is_empty(), "{} {}: calibration or fixture errors {:?}", name, policy, bad);
        faults
    }

    #[test]
    fn the_ledger_policy_reproduces_no_catalogue_fault_on_the_wp4_fixtures() {
        let table = kind_table();
        let policy = LedgerPolicy::default();
        for (name, text, old_first) in [
            ("d-sg1-aegis-orphan", include_str!("../../../../tests/fixtures/swap/d-sg1-aegis-orphan.jsonl"), Fault::LoaderRefused7 { role: 0, residual: vec![0x1338] }),
            ("ic-b119-state11", include_str!("../../../../tests/fixtures/swap/ic-b119-state11.jsonl"), Fault::State11Stall { role: 0 }),
            // The recorded window holds Kirby at cell 1 (the old code's rotation-order layout); the
            // ledger policy lays it out at cell 0 (`CellRule::PinCell0`), so neither the mirror release
            // under a live copy nor the cell-1 unload spin (`KirbyCopyListAtNonZeroCell`) remains.
            ("d-kirby1-k1", include_str!("../../../../tests/fixtures/swap/d-kirby1-k1.jsonl"), Fault::Spin { role: 0, hazard: G68Hazard::MirrorReleaseUnderLiveCopy }),
        ] {
            let events = fixture(text, name);
            let old = real_faults(replay(&events, &OldPolicy, &table), name, "old");
            assert_eq!(old.first(), Some(&old_first), "{}: the recorded fault reproduces under the old policy", name);
            let new = real_faults(replay(&events, &policy, &table), name, "ledger");
            assert!(new.is_empty(), "{} under the ledger policy: {:?}", name, new);
        }
        for (name, text) in [
            ("solo-stall-double-link", include_str!("../../../../tests/fixtures/swap/solo-stall-double-link.jsonl")),
            ("defect2-lobby-cpu-aegis-prefetch", include_str!("../../../../tests/fixtures/swap/defect2-lobby-cpu-aegis-prefetch.jsonl")),
        ] {
            let events = fixture(text, name);
            let old = real_faults(replay(&events, &OldPolicy, &table), name, "old");
            assert!(matches!(old.first(), Some(Fault::LoaderRefused7 { .. })), "{}: {:?}", name, old);
            let new = real_faults(replay(&events, &policy, &table), name, "ledger");
            assert!(new.is_empty(), "{} under the ledger policy: {:?}", name, new);
        }
    }

    #[test]
    fn the_kirby_row_decides_the_s_b_replay_on_the_model() {
        let table = kind_table();
        let events = fixture(include_str!("../../../../tests/fixtures/swap/d-hang1-s-b.jsonl"), "d-hang1-s-b");
        // Shipped row (`CellRule::PinCell0`): the recorded [5, 6, 7] window is laid out [6, 5, 7] with
        // Yoshi active at cell 1, Kirby is sub 0 of its entry, and the teardown to Chrom completes
        // (RE-KIRBY-UNLOAD §0/§6: S-K, Kirby at sub 0, completes). The old and recipe policies keep
        // Kirby at cell 1 and spin (WP2's own tests).
        let policy = LedgerPolicy::default();
        assert_eq!(policy.plan_window([0x05, KIRBY, 0x07]), [KIRBY, 0x05, 0x07]);
        assert_eq!(policy.plan_window([0x05, 0x07, KIRBY]), [KIRBY, 0x05, 0x07]);
        assert_eq!(policy.plan_window([KIRBY, 0x05, 0x07]), [KIRBY, 0x05, 0x07]);
        assert_eq!(policy.plan_window([0x05, KIRBY, NO_KIND]), [KIRBY, 0x05, NO_KIND]);
        // Row 4: an upcoming Solo Popo is pinned the same way; with both pinned kinds the later
        // one in rotation order waits for the next window.
        assert_eq!(policy.plan_window([0x05, POPO, 0x07]), [POPO, 0x05, 0x07]);
        assert_eq!(policy.plan_window([0x05, 0x07, POPO]), [POPO, 0x05, 0x07]);
        assert_eq!(policy.plan_window([0x05, KIRBY, POPO]), [KIRBY, 0x05, NO_KIND]);
        assert_eq!(policy.plan_window([0x05, POPO, KIRBY]), [POPO, 0x05, NO_KIND]);
        assert_eq!(policy.plan_window([KIRBY, POPO, 0x07]), [KIRBY, NO_KIND, 0x07]);
        let shipped = real_faults(replay(&events, &policy, &table), "d-hang1-s-b", "ledger/shipped");
        assert!(shipped.is_empty(), "{:?}", shipped);
        // WORKPLAN §5.3 fallback row: Kirby canonical-only; the 7a candidate: active only. Either keeps
        // Kirby out of the parked cells, and the Reload completes on the model too.
        for kirby in [
            Capabilities { qualification: Qualification::Gated("D-HANG1/D-KIRBY1 class unproven"), ..capabilities(FormClass::Kirby) },
            Capabilities { window_cell: CellRule::ActiveOnly, ..capabilities(FormClass::Kirby) },
        ] {
            let policy = LedgerPolicy { kirby, ..LedgerPolicy::default() };
            assert_eq!(policy.plan_window([0x05, KIRBY, 0x07]), [0x05, NO_KIND, 0x07]);
            let faults = real_faults(replay(&events, &policy, &table), "d-hang1-s-b", "ledger/fallback");
            assert!(faults.is_empty(), "{:?}", faults);
        }
        // Native shapes pass through the planner untouched.
        let policy = LedgerPolicy::default();
        assert_eq!(policy.plan_window([POPO, NANA, 0x0a]), [POPO, NANA, 0x0a]);
        assert_eq!(policy.plan_window([0x24, 0x25, 0x26]), [0x24, 0x25, 0x26]);
        assert_eq!(policy.plan_window([0x07, 0x08, 0x09]), [0x07, 0x08, 0x09]);
        assert_eq!(policy.supplemental([POPO, 0x03, 0x07], 3), vec![0x07], "SOLO-STALL: Link is native's request");
        assert_eq!(policy.supplemental([0x07, 0x08, 0x09], 3), vec![0x08, 0x09]);
        assert_eq!(policy.supplemental([POPO, NANA, 0x0a], 3), vec![0x0a]);
        assert_eq!(policy.pair_refs([POPO, NANA, 0x02], 3), Some(3));
        assert_eq!(policy.pair_refs([POPO, 0x03, 0x07], 3), None);
        assert!(!policy.prefetch_admitted(Some(KIRBY)) && policy.prefetch_admitted(Some(0x07)) && policy.prefetch_admitted(None));
        assert!(!policy.key_release_admitted(0x00, Some(0x00)) && policy.key_release_admitted(0x00, Some(0x07)));
    }
}
