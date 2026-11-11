//! The transition coordinator (WORKPLAN §2.6 5b/5c; SYNTHESIS agree-6; Astra §6 state shape; Fable
//! §2.4 typestate carrying Astra's progress-path invariant).
//!
//! Contract: every refusal decidable from the preflight snapshot happens **before** the first native
//! effect and returns the lease (INSTALL) with nothing to undo. After `commit` there is no `refuse`:
//! the only exits are `settle` (full proof) or an explicit `Recovery` chosen at preflight, and the
//! lease leaves the transaction exactly once, inside the `Outcome`. The invariant (Astra §6.3): INSTALL
//! held ⇒ an identified transaction owns it **and names its progress path** — native ticks admitted
//! (`NativeContinue`, D-PTA1BC-FIX §4), our own rollback (`RollbackBuilt`, the Phase B failure arm), or
//! entry destruction only (`AwaitDestruction`: loader 7 unpublished has no alias to tick, D-SG1 §2.6 /
//! DEFECT #2; an exact-Mii late refusal keeps today's fail-closed path, Astra §6.3). Nothing here frees
//! the lease on a timeout, forces a loader byte or invents a reset (WORKPLAN §5.4).
//!
//! Facts the states encode: the native tick protocol is today's [`Gate`] (`enter`/`leave`/
//! `may_publish`/`publish`/`can_complete`; `crate::engine::mii::ordinary_player`), which the transaction
//! now owns so that `Gate::refuse` is reachable only through a `Recovery`. `enter` with `loader >= 7`
//! and nothing published is native's own refusal at state 6 (`E+0x591c = 7`, no alias, no publication,
//! SOLO-STALL-RCA §1): the adapter must not call the original service for that entry again (a null
//! alias), so the recovery is destruction. A prequeue mismatch of a plain-kind track continues natively
//! (the staged descriptor is what state 6 installs; `D-PTA1-RCA §8.2 option 1`); a Mii-payload track has
//! no proven continuation (`Recovery::AwaitDestruction` with its reason, the explicit exclusion).

use core::marker::PhantomData;

use crate::engine::mii::identity::{verify, ExpectedMutation, Identity, Mismatch};
use crate::engine::mii::ordinary_player::{Gate, Phase, Ticket};

#[derive(Debug)]
pub struct Preflight;
#[derive(Debug)]
pub struct Committed;
#[derive(Debug)]
pub struct Recovering;

/// The compensation a committed transaction takes when its proof fails. Chosen at preflight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// Continue the native epilogue; native installs the staged descriptor; at loader 14 the
    /// transaction settles unowned (authority dropped, nothing published). Native ticks admitted.
    NativeContinue,
    /// The Phase B failure arm: built cells destroyed, the saved window restored, a known member
    /// activates natively (`RecoveredKnownMember`; not the original member, Astra §6.2).
    RollbackBuilt,
    /// No native progress path exists (loader 7 unpublished: native refused the request before
    /// anything was published, DEFECT #2's shape): the lease is owned until the entry's destruction,
    /// and the native tick for the entry stays suppressed. No plan selects it for a late mismatch
    /// since row 7 (forward continuation everywhere); it remains the named shape of that stall.
    AwaitDestruction,
}
impl Recovery {
    pub const fn admits_native_ticks(self) -> bool {
        matches!(self, Recovery::NativeContinue | Recovery::RollbackBuilt)
    }
    pub const fn progress(self) -> Progress {
        match self {
            Recovery::NativeContinue => Progress::NativeTicks,
            Recovery::RollbackBuilt => Progress::Rollback,
            Recovery::AwaitDestruction => Progress::DestructionOnly,
        }
    }
}

/// What can still move a transaction that holds the lease.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Native service ticks for the entry are admitted and lead to Ready.
    NativeTicks,
    /// Our own compensation completes it.
    Rollback,
    /// Only the entry's destruction ends it (reported, never hidden).
    DestructionOnly,
}

/// The recoveries a transaction may take, fixed before the first native effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveryPlan {
    /// A prequeue or Ready proof mismatch after commit.
    pub late_mismatch: Recovery,
    /// A failed Phase B build (verification failed after construction).
    pub build_failure: Recovery,
}
impl RecoveryPlan {
    /// Plain-kind track (no Mii payload, no same-class proof): D-PTA1b generalised.
    pub const PLAIN_KIND: RecoveryPlan = RecoveryPlan { late_mismatch: Recovery::NativeContinue, build_failure: Recovery::RollbackBuilt };
    /// Mii-payload track (WORKAROUND-DECISIONS row 7, operator-approved root fix): a late mismatch
    /// continues natively as well — the staged descriptor installs with the native-current Mii rows
    /// and the transaction settles unowned at an idle loader; the fail-closed epilogue that stranded
    /// the entry at loader 6 (D-PTA1b) is gone for every track. Eden E2 validates the cross-class case.
    pub const MII_PAYLOAD: RecoveryPlan = RecoveryPlan { late_mismatch: Recovery::NativeContinue, build_failure: Recovery::RollbackBuilt };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadyReceipt {
    pub identity: Identity,
    pub kind: u32,
}

/// Every way a committed transaction ends; each carries the lease out exactly once.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome<L> {
    Applied { receipt: ReadyReceipt, lease: L },
    RecoveredKnownMember { receipt: ReadyReceipt, reason: &'static str, lease: L },
    NativeSettledUnowned { reason: &'static str, lease: L },
    Destroyed { reason: &'static str, lease: L },
}
impl<L> Outcome<L> {
    pub fn name(&self) -> &'static str {
        match self {
            Outcome::Applied { .. } => "Applied",
            Outcome::RecoveredKnownMember { .. } => "RecoveredKnownMember",
            Outcome::NativeSettledUnowned { .. } => "NativeSettledUnowned",
            Outcome::Destroyed { .. } => "Destroyed",
        }
    }
    pub fn reason(&self) -> &'static str {
        match self {
            Outcome::Applied { .. } => "ready proof passed",
            Outcome::RecoveredKnownMember { reason, .. } | Outcome::NativeSettledUnowned { reason, .. } | Outcome::Destroyed { reason, .. } => reason,
        }
    }
    /// The lease, to be released by the caller exactly once.
    pub fn into_lease(self) -> L {
        match self {
            Outcome::Applied { lease, .. } | Outcome::RecoveredKnownMember { lease, .. }
            | Outcome::NativeSettledUnowned { lease, .. } | Outcome::Destroyed { lease, .. } => lease,
        }
    }
}

/// A refusal before any native effect: nothing to undo, the lease comes back.
#[derive(Debug, PartialEq, Eq)]
pub struct Refusal<L> {
    pub reason: &'static str,
    pub lease: L,
}

/// What the gate hook does with the native prequeue (`0x6510f4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateDecision {
    /// Run the native state-6 tail (publication verified, or a native continuation).
    Continue,
    /// Branch to the epilogue `0x651464` (today's fail-closed path; only `AwaitDestruction`).
    Refuse,
}

/// Evidence a recovering transaction finishes on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Evidence {
    /// Loader idle at 14 with no pending change (`settled(entry)`).
    pub loader_settled: bool,
    /// The rollback arm ran and the restored window's member is ready.
    pub rollback_done: Option<ReadyReceipt>,
}

#[derive(Debug)]
pub struct Txn<S, L> {
    ticket: Ticket,
    lease: L,
    identity: Identity,
    plan: RecoveryPlan,
    gate: Gate,
    applied: Vec<&'static str>,
    recovery: Option<(Recovery, &'static str)>,
    _s: PhantomData<S>,
}

impl<S, L> Txn<S, L> {
    pub fn ticket(&self) -> Ticket { self.ticket }
    pub fn identity(&self) -> &Identity { &self.identity }
    pub fn plan(&self) -> RecoveryPlan { self.plan }
    pub fn applied(&self) -> &[&'static str] { &self.applied }
    pub fn published(&self) -> bool { self.gate.published() }
    fn into_outcome_destroyed(self, reason: &'static str) -> Outcome<L> { Outcome::Destroyed { reason, lease: self.lease } }
}

impl<L> Txn<Preflight, L> {
    /// Everything decidable from the snapshot is decided here. The first failing check refuses and
    /// hands the lease back with nothing applied.
    pub fn preflight(
        ticket: Ticket,
        lease: L,
        identity: Identity,
        plan: RecoveryPlan,
        checks: impl IntoIterator<Item = Result<(), &'static str>>,
    ) -> Result<Self, Refusal<L>> {
        for check in checks {
            if let Err(reason) = check { return Err(Refusal { reason, lease }); }
        }
        let Some(gate) = Gate::new(ticket) else { return Err(Refusal { reason: "invalid ticket", lease }); };
        Ok(Txn { ticket, lease, identity, plan, gate, applied: Vec::new(), recovery: None, _s: PhantomData })
    }
    /// A refusal between preflight and the first native effect (the bank refused before any native
    /// write; the re-snapshot differed): still nothing to undo.
    pub fn abandon(self, reason: &'static str) -> Refusal<L> { Refusal { reason, lease: self.lease } }
    /// The first native effect (`before_reload`, `submit`, `reload_at_pose`): from here on there is no
    /// refusal, only settlement or a planned recovery.
    pub fn commit(mut self, first_effect: &'static str) -> Txn<Committed, L> {
        self.applied.push(first_effect);
        Txn { ticket: self.ticket, lease: self.lease, identity: self.identity, plan: self.plan, gate: self.gate, applied: self.applied, recovery: None, _s: PhantomData }
    }
}

/// Result of a native tick entering a committed transaction.
#[derive(Debug)]
pub enum Enter<L> {
    /// Run the original service for this entry.
    Admit(Txn<Committed, L>),
    /// Do not run it (a nested same-entry callback).
    Suppress(Txn<Committed, L>),
    /// Native refused the Reload at state 6 (`loader >= 7`, nothing published): no alias exists, so
    /// the transaction recovers by destruction and the tick stays suppressed.
    NativeRefused(Txn<Recovering, L>),
}

impl<L> Txn<Committed, L> {
    pub fn record(&mut self, effect: &'static str) { self.applied.push(effect); }
    /// The native tick protocol (`Gate::enter`). A refused loader is a recovery, never a dead end.
    pub fn enter(mut self, loader: u8) -> Enter<L> {
        if self.gate.enter(self.ticket, loader) { return Enter::Admit(self); }
        if self.gate.phase() == Phase::Refused {
            self.recovery = Some((Recovery::AwaitDestruction, "native refused at state 6 (loader 7, resource owner busy); no alias to continue"));
            return Enter::NativeRefused(Txn { ticket: self.ticket, lease: self.lease, identity: self.identity, plan: self.plan, gate: self.gate, applied: self.applied, recovery: self.recovery, _s: PhantomData });
        }
        Enter::Suppress(self)
    }
    pub fn leave(&mut self, loader: u8) { self.gate.leave(self.ticket, loader); }
    pub fn may_publish(&self, loader: u8) -> bool { self.gate.may_publish(self.ticket, loader) }
    /// The prequeue may only assert "unchanged since preflight" (header captured, never derived).
    pub fn verify_unchanged(&self, now: &Identity) -> Result<(), Mismatch> { verify(&self.identity, now, ExpectedMutation::None) }
    /// The prequeue (`0x6510f4`): publish on a verified identity; otherwise the planned recovery decides
    /// whether the native tail still runs. Returns the decision and the transaction's next form.
    pub fn prequeue(mut self, now: &Identity, loader: u8) -> (GateDecision, Step<L>) {
        let late = self.plan.late_mismatch;
        if !self.may_publish(loader) {
            return self.recover_with(late, "duplicate or re-entrant gate");
        }
        match self.verify_unchanged(now) {
            Ok(()) => {
                self.gate.publish(self.ticket, loader);
                self.applied.push("publication");
                (GateDecision::Continue, Step::Committed(self))
            }
            Err(_) => self.recover_with(late, "prequeue identity differs from the preflight capture"),
        }
    }
    fn recover_with(self, how: Recovery, reason: &'static str) -> (GateDecision, Step<L>) {
        let decision = if how.admits_native_ticks() { GateDecision::Continue } else { GateDecision::Refuse };
        (decision, Step::Recovering(self.recover(how, reason)))
    }
    pub fn can_settle(&self) -> bool { self.gate.can_complete(self.ticket) }
    /// Ready: the full proof (`allowed` names the one mutation this phase admits, e.g. a native form
    /// selection) applies the receipt; a mismatch takes the planned late recovery.
    pub fn settle(self, now: Identity, kind: u32, allowed: ExpectedMutation) -> Settlement<L> {
        let late = self.plan.late_mismatch;
        match verify(&self.identity, &now, allowed) {
            Ok(()) => Settlement::Applied(Outcome::Applied { receipt: ReadyReceipt { identity: now, kind }, lease: self.lease }),
            Err(_) => Settlement::Recovering(self.recover(late, "ready proof differs from the preflight capture")),
        }
    }
    /// An explicit recovery (the Phase B failure arm, a scene departure during the build).
    pub fn recover(mut self, how: Recovery, reason: &'static str) -> Txn<Recovering, L> {
        // The gate leaves its publication path for good; a recovery that needs native ticks takes the
        // D-PTA1b shape (refused + `native_only`) so `enter` admits the loader all the way to settled.
        self.gate.refuse();
        if how.admits_native_ticks() { self.gate.continue_natively(); }
        Txn { ticket: self.ticket, lease: self.lease, identity: self.identity, plan: self.plan, gate: self.gate, applied: self.applied, recovery: Some((how, reason)), _s: PhantomData }
    }
    /// The entry was destroyed (scene exit, match start): the lease comes back with the reason.
    pub fn destroyed(self, reason: &'static str) -> Outcome<L> { self.into_outcome_destroyed(reason) }
}

impl<L> Txn<Recovering, L> {
    pub fn recovery(&self) -> Recovery { self.recovery.map(|(how, _)| how).unwrap_or(Recovery::AwaitDestruction) }
    pub fn reason(&self) -> &'static str { self.recovery.map(|(_, reason)| reason).unwrap_or("recovery") }
    pub fn progress(&self) -> Progress { self.recovery().progress() }
    /// Native ticks are admitted exactly when the recovery's progress path needs them.
    pub fn enter(&mut self, loader: u8) -> bool {
        if !self.recovery().admits_native_ticks() { return false; }
        if self.gate.phase() == Phase::Refused && !self.gate.native_only() { self.gate.continue_natively(); }
        self.gate.enter(self.ticket, loader)
    }
    pub fn leave(&mut self, loader: u8) { self.gate.leave(self.ticket, loader); }
    /// Finish on evidence, never on time: `NativeContinue` at a settled loader, `RollbackBuilt` on
    /// the restored member's receipt; `AwaitDestruction` only through [`Self::destroyed`].
    pub fn finish(self, evidence: Evidence) -> Finish<L> {
        match (self.recovery(), evidence.loader_settled, evidence.rollback_done) {
            (Recovery::NativeContinue, true, _) => Finish::Done(Outcome::NativeSettledUnowned { reason: self.reason(), lease: self.lease }),
            (Recovery::RollbackBuilt, _, Some(receipt)) => Finish::Done(Outcome::RecoveredKnownMember { receipt, reason: self.reason(), lease: self.lease }),
            _ => Finish::Pending(self),
        }
    }
    pub fn destroyed(self, reason: &'static str) -> Outcome<L> { self.into_outcome_destroyed(reason) }
}

/// `Txn::settle`'s answer. A plain enum, not a `Result`: the recovering transaction is the large
/// arm (it carries the preflight `Identity`), and it is a normal continuation, not an error.
#[derive(Debug)]
pub enum Settlement<L> {
    Applied(Outcome<L>),
    Recovering(Txn<Recovering, L>),
}
impl<L> Settlement<L> {
    pub fn applied(self) -> Option<Outcome<L>> {
        match self { Settlement::Applied(outcome) => Some(outcome), Settlement::Recovering(_) => None }
    }
    pub fn recovering(self) -> Option<Txn<Recovering, L>> {
        match self { Settlement::Recovering(txn) => Some(txn), Settlement::Applied(_) => None }
    }
}

/// `Txn<Recovering>::finish`'s answer: done with the outcome, or still pending (the same transaction).
#[derive(Debug)]
pub enum Finish<L> {
    Done(Outcome<L>),
    Pending(Txn<Recovering, L>),
}
impl<L> Finish<L> {
    pub fn done(self) -> Option<Outcome<L>> {
        match self { Finish::Done(outcome) => Some(outcome), Finish::Pending(_) => None }
    }
    pub fn pending(self) -> Option<Txn<Recovering, L>> {
        match self { Finish::Pending(txn) => Some(txn), Finish::Done(_) => None }
    }
}

/// The transaction as an adapter stores it between callbacks (the typestates above are the
/// compile-time contract; this holder sequences them at runtime).
#[derive(Debug)]
pub enum Step<L> {
    Committed(Txn<Committed, L>),
    Recovering(Txn<Recovering, L>),
}
impl<L> Step<L> {
    pub fn ticket(&self) -> Ticket {
        match self { Step::Committed(t) => t.ticket, Step::Recovering(t) => t.ticket }
    }
    pub fn progress(&self) -> Progress {
        match self { Step::Committed(_) => Progress::NativeTicks, Step::Recovering(t) => t.progress() }
    }
    pub fn published(&self) -> bool {
        match self { Step::Committed(t) => t.published(), Step::Recovering(t) => t.published() }
    }
    pub fn recovery(&self) -> Option<Recovery> {
        match self { Step::Committed(_) => None, Step::Recovering(t) => Some(t.recovery()) }
    }
    /// `enter_native`: admit the original service or not; a native refusal moves to recovery.
    pub fn enter(self, loader: u8) -> (bool, Step<L>) {
        match self {
            Step::Committed(t) => match t.enter(loader) {
                Enter::Admit(t) => (true, Step::Committed(t)),
                Enter::Suppress(t) => (false, Step::Committed(t)),
                Enter::NativeRefused(t) => (false, Step::Recovering(t)),
            },
            Step::Recovering(mut t) => { let admit = t.enter(loader); (admit, Step::Recovering(t)) }
        }
    }
    pub fn leave(&mut self, loader: u8) {
        match self { Step::Committed(t) => t.leave(loader), Step::Recovering(t) => t.leave(loader) }
    }
    /// The entry was destroyed: every form ends here with the lease.
    pub fn destroyed(self, reason: &'static str) -> Outcome<L> {
        match self { Step::Committed(t) => t.destroyed(reason), Step::Recovering(t) => t.destroyed(reason) }
    }
}

