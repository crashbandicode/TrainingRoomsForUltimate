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
//!   no acquisition of a fighter kind while it is live (the bank's object kinds are no fighter a copy
//!   takes: primary 2026-11-09 05:38Z); *drain*: a transition whose target is such a member must
//!   first return every prefetched key (the planner drains Prefetched groups to Planned before the
//!   commit, so a freshly live Kirby, hatless by `kirby_park_gate`, finds an empty debt).
//! - `HatSourceLive` (HYP, K1 §6 K2): a hat-source kind built into the incoming window while a copy
//!   exists; conservative on, inert until the adapter supplies hat-source kinds.
//! - `OtherRoleLive` (disproved for the mirror shape: D-HANG1-SEPARATION §1 S-A 3/3 completed; the
//!   hang is the parked never-activated Kirby, WP3/WP-K): off in `Rules::C116_1`.
//!
//! rc.7 F1 (`Rules::OWNED_G68`): the bank owns the G68 step of its releases (`runtime::roster::g68_step`;
//! `request_bit::Policy::Owned` models it), so `LiveCopySharesRegistry` and `PrefetchDebtUnderLiveKirby` have no
//! hazard left to guard and raise nothing; the other rules stand.

use super::capabilities::{bank_object_kind, capabilities, form_class, Capabilities};
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
    /// rc.7 F1 (the D-KIRBY1 root fix, `runtime::roster::g68_step`): the bank owns the G68 step of the
    /// releases it causes, so a release of the bank's keys never reaches a copy's package
    /// (`request_bit::Policy::Owned`). `live_copy` and `prefetch_debt_under_live_kirby` then raise nothing.
    pub g68_owned: bool,
}

impl Rules {
    /// C116.1: the two FACT rules, the K2 hypothesis conservative, the mirror rule off (disproved).
    pub const C116_1: Rules = Rules {
        live_copy: Grade::Fact,
        prefetch_debt_under_live_kirby: Grade::Fact,
        hat_source: Some(Grade::Hyp),
        other_role_live: None,
        g68_owned: false,
    };
    /// rc.7: C116.1 with the G68 step owned (both D-KIRBY1 rules have no hazard left to guard).
    pub const OWNED_G68: Rules = Rules { g68_owned: true, ..Rules::C116_1 };
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
        // rc.7 F1: with the G68 step owned, neither D-KIRBY1 rule has a hazard to guard.
        if !rules.g68_owned {
            if self.kirby_copy[i] == Some(kind) && matches!(usage, Use::Acquire | Use::Release) {
                return Some(Conflict::LiveCopySharesRegistry);
            }
            // The suspend covers the fighter kinds a copy can share on R; the bank's object kinds are none (a detached
            // PT group's object re-arm runs under a live Kirby: primary 2026-11-09 05:38Z, `object_rearm_admitted`).
            if usage == Use::Acquire && self.prefetch_suspended(role) && !bank_object_kind(kind) {
                return Some(Conflict::PrefetchDebtUnderLiveKirby);
            }
            if usage == Use::TargetKind && self.debt_blocks_target(role, kind) {
                return Some(Conflict::PrefetchDebtUnderLiveKirby);
            }
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

