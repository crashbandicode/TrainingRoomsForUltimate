//! U20: what a swap fallback does to the group it could not serve (`native_body_bank::perform_swap`
//! reason chain).
//!
//! The defect (pre-existing, found by the `mii-phaseb` worker and its reviewer): every fallback
//! except the pair-rule wait set the requested group to `Failed`, including a `Prefetched` one. A
//! Prefetched group holds ledger tokens and their native keys on `R` (`0x17e64d0`). `Failed` means
//! "keys released; never retried in this scene" (`body_bank::GroupState`), but nothing released them.
//! The keys stayed on `R` until the scene ended. While they stay, `R`'s map never empties, so `R` is
//! never free for a Reload (`0x17e4700`) and never resets (`0x17e8848–0x17e8880`).
//!
//! Native semantics decide the split. Native never abandons a registration it is waiting on: loader
//! state 7 polls `R`'s readiness until the loads complete. It gives keys up only through the per-kind
//! release `0x17e8610`, the same call a construction tail makes. So a fallback whose condition clears
//! by itself keeps the group `Prefetched` with its keys, and the next request retries. A permanent one
//! fails the group and returns every key through the ledger's exact inverse (`r_handle::release`:
//! orphans, then `0x17e8610`), the path the teardown and a failed prefetch already use.

use crate::engine::swap::body_bank::GroupState;

/// Every reason the reason chain can fall back for, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
    OwnerReceiptChanged,
    GroupSwapDisabled,
    NotReachable,
    WindowShape,
    AliasChanged,
    ResourceNotReady,
    RegistryUnreadable,
    PoolFull,
    PairResetWait,
    CopyPackagesLoading,
    /// MII-PHASEB: the one-member Mii group's Phase B preflight refused (its clause text). It
    /// touched nothing and the group keeps its keys (that worker's rule, kept here).
    MiiSetupRefused(&'static str),
}

impl Fallback {
    pub const ALL: [Fallback; 11] = [
        Fallback::OwnerReceiptChanged, Fallback::GroupSwapDisabled, Fallback::NotReachable, Fallback::WindowShape,
        Fallback::AliasChanged, Fallback::ResourceNotReady, Fallback::RegistryUnreadable, Fallback::PoolFull,
        Fallback::PairResetWait, Fallback::CopyPackagesLoading, Fallback::MiiSetupRefused("Mii setup: native loader not idle"),
    ];

    /// The `[GroupSwap] fallback … reason=` text (unchanged from the chain's strings).
    pub const fn text(self) -> &'static str {
        match self {
            Fallback::OwnerReceiptChanged => "group owner receipt changed",
            Fallback::GroupSwapDisabled => "group swap disabled",
            Fallback::NotReachable => "group not reachable",
            Fallback::WindowShape => "window records or vector shape differ",
            Fallback::AliasChanged => "resource owner alias changed",
            Fallback::ResourceNotReady => "resource owner not ready (task running or handles unloaded)",
            Fallback::RegistryUnreadable => "copy registry or R+0x54 owner unreadable",
            Fallback::PoolFull => "fighter pool has no room for the group",
            Fallback::PairResetWait => "pair build waits for an exclusive reset batch",
            Fallback::CopyPackagesLoading => "Kirby copy packages for the registration still loading",
            Fallback::MiiSetupRefused(why) => why,
        }
    }

    /// The condition clears without the bank doing anything: a task finishes, a pool body is
    /// returned, another batch's keys drain, a package finishes loading, or a group not yet
    /// prefetched gets its keys. The same request succeeds later.
    pub const fn transient(self) -> bool {
        matches!(self, Fallback::ResourceNotReady | Fallback::PoolFull | Fallback::PairResetWait
            | Fallback::CopyPackagesLoading | Fallback::NotReachable | Fallback::MiiSetupRefused(_))
    }
}

/// What the fallback does to the requested group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// The group keeps its state (and its keys); the next request retries.
    Keep,
    /// The group fails; it holds no keys.
    Fail,
    /// The group fails and returns every key it holds through `r_handle::release` first.
    FailReturningKeys,
}

/// The rule. A `Built` group keeps its detached bodies. A `Prefetching` one is owned by the in-flight
/// prefetch, whose poll settles it. A transient reason keeps any group. A permanent one fails it,
/// returning the keys a `Prefetched` group holds.
pub const fn disposition(reason: Fallback, state: GroupState) -> Disposition {
    match state {
        GroupState::Built | GroupState::Prefetching => Disposition::Keep,
        _ if reason.transient() => Disposition::Keep,
        GroupState::Prefetched => Disposition::FailReturningKeys,
        GroupState::Planned | GroupState::Held(_) | GroupState::Failed | GroupState::Released => Disposition::Fail,
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::swap::swap_model::ledger::{GroupId, Holder, Ledger, NativeCall, NoConsumers, ReleaseRecipe, RFlags, Role};
    use crate::engine::swap::swap_model::native_model::{kind_table, RModel, G68};

    const GROUPS: [&[u32]; 5] = [&[0x07], &[0x1b, 0x16, 0x15], &[0x48], &[0x52, 0x03], &[0x06, 0x04, 0x05]];

    /// Prefetch `kinds` as the bank does (ledger token, then `0x17e64d0`), on an R that also holds
    /// another group's keys so the map does not empty by accident.
    fn prefetched(kinds: &[u32]) -> (Ledger, RModel, Vec<crate::engine::swap::swap_model::ledger::AcqToken>) {
        let table = kind_table();
        let mut ledger = Ledger::new(RFlags::TRAINING);
        let mut r = RModel::default();
        let other = ledger.new_batch();
        let (_other, _) = ledger.acquire(0x0b, Holder::Prefetch(GroupId(1)), ReleaseRecipe::ConstructionTail, other, &NoConsumers, Role::Player).unwrap();
        r.acquire(0x0b, &table);
        let batch = ledger.new_batch();
        let mut tokens = Vec::new();
        for kind in kinds {
            let (token, call) = ledger.acquire(*kind, Holder::Prefetch(GroupId(0)), ReleaseRecipe::ConstructionTail, batch, &NoConsumers, Role::Player).unwrap();
            assert_eq!(call, Some(NativeCall::Acquire { kind: *kind }));
            r.acquire(*kind, &table);
            tokens.push(token);
        }
        (ledger, r, tokens)
    }

    fn release_all(ledger: &mut Ledger, r: &mut RModel, tokens: Vec<crate::engine::swap::swap_model::ledger::AcqToken>) {
        let mut g68 = G68::default();
        for token in tokens {
            for call in ledger.release(token, &NoConsumers, Role::Player).unwrap() {
                match call {
                    NativeCall::ReleaseKey(key) => r.release_key(key.value()),
                    NativeCall::ReleaseKind { kind } => r.release_kind(kind, None, &mut g68),
                    other => panic!("unexpected {:?}", other),
                }
            }
        }
    }

    /// The reported defect: under today's rule a Prefetched group fails with its keys still on R,
    /// and no later state releases them (`Failed` is never retried).
    fn today_a_failed_prefetched_group_keeps_its_keys() {
        for kinds in GROUPS {
            let (ledger, r, tokens) = prefetched(kinds);
            assert!(!tokens.is_empty());
            let reconcile = ledger.reconcile(&r.snapshot());
            assert!(reconcile.balanced());
            assert!(ledger.predicted().len() > 1, "{:x?}: the group's keys stay predicted and on R", kinds);
        }
    }

    /// Every permanent fallback on a Prefetched group returns exactly what the prefetch took:
    /// `[Ledger] reconcile residual=[] missing=[]`, and only the other group's keys stay on R.
    /// A transient one keeps the keys for the retry, still reconciled.
    fn a_permanent_fallback_returns_every_key_and_a_transient_one_keeps_them() {
        let table = kind_table();
        for reason in Fallback::ALL {
            for kinds in GROUPS {
                let (mut ledger, mut r, tokens) = prefetched(kinds);
                match disposition(reason, GroupState::Prefetched) {
                    Disposition::FailReturningKeys => {
                        assert!(!reason.transient());
                        release_all(&mut ledger, &mut r, tokens);
                        let reconcile = ledger.reconcile(&r.snapshot());
                        assert!(reconcile.residual.is_empty() && reconcile.missing.is_empty(), "{:?} {:x?}: {:?}", reason, kinds, reconcile);
                        let mut other = RModel::default();
                        other.acquire(0x0b, &table);
                        assert_eq!(r.snapshot(), other.snapshot(), "{:?} {:x?}: only the other group's keys remain", reason, kinds);
                    }
                    Disposition::Keep => {
                        assert!(reason.transient());
                        assert!(ledger.reconcile(&r.snapshot()).balanced());
                        assert_eq!(ledger.tokens().len(), kinds.len() + 1, "the group keeps its tokens for the retry");
                    }
                    Disposition::Fail => panic!("{:?}: a Prefetched group never fails without returning its keys", reason),
                }
            }
        }
    }

    fn built_and_in_flight_groups_are_never_failed_by_a_fallback() {
        for reason in Fallback::ALL {
            assert_eq!(disposition(reason, GroupState::Built), Disposition::Keep);
            assert_eq!(disposition(reason, GroupState::Prefetching), Disposition::Keep);
            let planned = disposition(reason, GroupState::Planned);
            assert_eq!(planned, if reason.transient() { Disposition::Keep } else { Disposition::Fail });
        }
        // The chain's strings are unchanged (Eden rows grep them).
        assert_eq!(Fallback::PairResetWait.text(), "pair build waits for an exclusive reset batch");
        assert_eq!(Fallback::PoolFull.text(), "fighter pool has no room for the group");
    }

    /// A swap that reaches a group held for capacity (nothing native exists, like a planned one) is served
    /// canonically by the same rule as a planned group: a transient reason keeps it, a permanent one fails it.
    fn held_group_at_a_swap_is_disposed_like_planned() {
        use crate::engine::swap::body_bank::Hold;
        for reason in Fallback::ALL {
            for hold in [Hold::Memory, Hold::Pool] {
                assert_eq!(disposition(reason, GroupState::Held(hold)), disposition(reason, GroupState::Planned), "{reason:?}");
            }
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("today_a_failed_prefetched_group_keeps_its_keys", today_a_failed_prefetched_group_keeps_its_keys as fn()),
            ("a_permanent_fallback_returns_every_key_and_a_transient_one_keeps_them", a_permanent_fallback_returns_every_key_and_a_transient_one_keeps_them as fn()),
            ("built_and_in_flight_groups_are_never_failed_by_a_fallback", built_and_in_flight_groups_are_never_failed_by_a_fallback as fn()),
            ("held_group_at_a_swap_is_disposed_like_planned", held_group_at_a_swap_is_disposed_like_planned as fn()),
        ]);
    }
    }
}
