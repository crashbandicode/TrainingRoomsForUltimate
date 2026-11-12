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
    #[cfg(test)]
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

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("built_and_in_flight_groups_are_never_failed_by_a_fallback", built_and_in_flight_groups_are_never_failed_by_a_fallback as fn()),
        ]);
    }
    }
}
