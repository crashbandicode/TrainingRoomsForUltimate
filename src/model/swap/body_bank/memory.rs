//! Process-memory admission of extra groups (GS-PANIC root fix, C116.1;
//! `D175/refactor/gs-panic-pool14/PROGRESS.md`).
//!
//! FACTs (13.0.5 ELF `0789f0a4…`, Eden s19/s21 crash logs):
//! - The game's heap `0x392e290` (freed by `0x392eb40`) grows through the page provider `0x37c9600`, which commits
//!   64 KiB pages of a reserved region (`*(main+0x593d430)`, here `0x1180000000`) with
//!   `nn::os::AllocateMemoryPages` (`0x37ca664`) and calls `abort()` at `0x37ca688` when the kernel refuses
//!   (s19/s21: `AllocateMemoryPages(0x123ACB0000|0x123FF20000, 0x50000)`, `nnosBreak`, Userspace PANIC).
//! - The kernel refuses when the process charge (svcGetInfo id 7) would pass the process budget (id 6),
//!   3,444,572,160 bytes (3285 MiB) in every sample of both runs. `headroom` (id 6 − id 7) is that margin.
//! - A group spends it in two phases seconds apart: its prefetch (the resource owner's file loads) and its build at
//!   the request (`0x658af0`; each body's init creates its article models: `0x6081f8 → 0x3a7320 → … → 0x3a6408 →
//!   0x33a0974 0x17def20 → 0x35c2e50 → … → 0x392e290`). Measured costs range 3–147 MiB (prefetch) and 0–98 MiB
//!   (build) per kind ([`super::memory_table`]); Game & Watch builds 80–98 MiB alone.
//!
//! The rule before this module admitted a group when the current headroom covered 96 MiB per body of that group
//! only. A second group admitted between the first group's prefetch and its build was measured against headroom the
//! first group's build still needed, and 96 MiB is below several kinds' measured cost (Simon/Richter ≈ 100 MiB,
//! Game & Watch ≈ 151 MiB). Both crashes are that sequence (s21 L7770/L7804/L7889, s19 L9037/L9084/L9219).
//!
//! This module owns the accounting for **every fighter load the mod starts** (s30, `D175/refactor/gs-panic-pool14`
//! PROGRESS §10: a helper source slot's speculative Reload of Joker at 63 MiB headroom took the same `abort()`):
//! - a group's prefetch and build ([`group_cost`]), and a helper source load ([`helper_cost`]), cost what the logs
//!   measured for them;
//! - an admitted group holds its unspent part ([`outstanding`]) until it is built, failed or torn down; a helper load
//!   in flight holds its whole cost ([`helper_outstanding`]) until it settles;
//! - a discretionary load (a group prefetch, a speculative helper load or the helper owner created for one) starts
//!   only when the headroom minus what every other consumer still holds covers its whole cost, with a margin and a
//!   floor ([`admit`]). A refused group fails and is served canonically; a refused helper preload stays in the plan
//!   and is tried again later. A load a visible request waits for is not refused here; it holds its cost while in
//!   flight so the discretionary ones wait for it.
//! - The Kirby copy packages (`swap_native::copy_packages`) are acquired only for the sets of the window and of
//!   groups that hold keys or bodies, in the tick an admitted group's prefetch starts: they are part of that group's
//!   prefetch, inside its admission and its measured cost.

use super::memory_table::{
    DEFAULT_BUILD, DEFAULT_PREFETCH, HELPER_COSTS, KIND_COSTS, SET_COSTS,
};
use super::GroupState;

pub const MIB: u64 = 1 << 20;
/// Every estimate is raised by `1/MARGIN_DIVISOR` (12.5 %). Leave-one-out over the 2,653 usable observations of the
/// regenerated table (`gs-panic-pool14/re/margin_loo.py`): no prefetch (663 checked) or build (590) observation
/// exceeds the largest of its key's other observations by more than this margin plus [`FLOOR`].
pub const MARGIN_DIVISOR: u64 = 8;
/// Headroom left after an admitted group's prefetch and build, for the request's own allocations around the build
/// (HUD, effects, the swap's bookkeeping).
pub const FLOOR: u64 = 16 * MIB;

/// A group's charge to the process budget, per phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupCost {
    /// Committed by the prefetch (the resource owner's loads).
    pub prefetch: u64,
    /// Committed by the build at the request (the bodies and their article models).
    pub build: u64,
}

impl GroupCost {
    pub fn total(self) -> u64 {
        self.prefetch.saturating_add(self.build)
    }
}

/// `bytes` raised by the margin.
pub fn with_margin(bytes: u64) -> u64 {
    bytes.saturating_add(bytes.div_ceil(MARGIN_DIVISOR))
}

/// One kind's measured cost; a phase never measured for the kind takes the largest estimate of that phase.
pub fn kind_cost(kind: u32) -> GroupCost {
    let (prefetch, build) = KIND_COSTS.iter().find(|(k, ..)| *k == kind).map_or((0, 0), |(_, p, b)| (*p, *b));
    GroupCost {
        prefetch: if prefetch == 0 { DEFAULT_PREFETCH } else { prefetch },
        build: if build == 0 { DEFAULT_BUILD } else { build },
    }
}

/// A group's cost: per phase, the measured cost of exactly this member set when the logs have it at least twice,
/// otherwise the larger of a single set observation and the sum of its members' costs (each an upper bound,
/// [`super::memory_table`]). One observation of a set is not a spread: gsp6 #295 built `[12,13,14]` for 33.1 MB
/// where the only earlier observation had been 17.9 MB.
pub fn group_cost(kinds: &[u32]) -> GroupCost {
    let summed = kinds.iter().fold(GroupCost::default(), |sum, kind| {
        let cost = kind_cost(*kind);
        GroupCost { prefetch: sum.prefetch.saturating_add(cost.prefetch), build: sum.build.saturating_add(cost.build) }
    });
    if !(2..=3).contains(&kinds.len()) {
        return summed;
    }
    let mut key = [u32::MAX; 3];
    key[..kinds.len()].copy_from_slice(kinds);
    key[..kinds.len()].sort_unstable();
    let phase = |measured: u64, observations: u16, sum: u64| match observations {
        0 => sum,
        1 => measured.max(sum),
        _ => measured,
    };
    match SET_COSTS.iter().find(|(set, ..)| *set == key) {
        Some((_, prefetch, build, prefetch_n, build_n)) => GroupCost {
            prefetch: phase(*prefetch, *prefetch_n, summed.prefetch),
            build: phase(*build, *build_n, summed.build),
        },
        None => summed,
    }
}

/// The bodies a helper source load of `kind` brings in: the native groups load whole (Pokémon Trainer
/// `[24,25,26]`, Pyra/Mythra `[5a,5b]`, Ice Climbers `[4b,4c]`; s30 L6171 `incoming=(3, [24, 25, 26] …)`).
pub fn helper_members(kind: u32) -> ([u32; 3], usize) {
    match kind {
        0x24..=0x26 => ([0x24, 0x25, 0x26], 3),
        0x5a | 0x5b => ([0x5a, 0x5b, u32::MAX], 2),
        0x4b => ([0x4b, 0x4c, u32::MAX], 2),
        _ => ([kind, u32::MAX, u32::MAX], 1),
    }
}

/// A helper source load of `kind` (a native Reload of a helper entry: its files and its bodies): the group cost of
/// the members it brings in, or the largest net charge a helper load of that kind was measured at, whichever is
/// larger (the measured value is net of what the outgoing helper kind released, so it is only a floor).
pub fn helper_cost(kind: u32) -> GroupCost {
    let (members, count) = helper_members(kind);
    let group = group_cost(&members[..count]);
    let measured = HELPER_COSTS.iter().find(|(k, _)| *k == kind).map_or(0, |(_, bytes)| *bytes);
    GroupCost { prefetch: group.prefetch, build: group.build.saturating_add(measured.saturating_sub(group.total())) }
}

/// What a helper load in flight holds against the budget: all of it, until it settles (ready, failed or departed).
pub fn helper_outstanding(cost: GroupCost) -> u64 {
    with_margin(cost.total())
}

/// What an admitted group still holds against the budget: a prefetch in flight its whole cost, a prefetched group
/// its build. Planned, built and failed groups hold nothing (a built group's charge is in the headroom).
pub fn outstanding(state: GroupState, cost: GroupCost) -> u64 {
    match state {
        GroupState::Prefetching => with_margin(cost.total()),
        GroupState::Prefetched => with_margin(cost.build),
        GroupState::Planned | GroupState::Held(_) | GroupState::Built | GroupState::Failed | GroupState::Released => 0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// The headroom less every admitted group's unspent part covers this group: prefetch it.
    Admit,
    /// The headroom covers this group only with what admitted groups still hold: keep it planned and decide again
    /// once they have built (their builds spend at most what they hold).
    Defer,
    /// The headroom does not cover this load (or could not be read). A group fails and is served canonically; a
    /// speculative helper load stays in the plan and is tried again on a later callback.
    Refuse,
}

/// One admission decision with the values it was taken on (logged by the adapter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub admission: Admission,
    pub headroom: Option<u64>,
    pub outstanding: u64,
    /// Headroom minus `outstanding`.
    pub available: u64,
    /// The group's cost with the margin, plus the floor.
    pub need: u64,
}

pub fn admit(headroom: Option<u64>, outstanding: u64, cost: GroupCost) -> Decision {
    let need = with_margin(cost.total()).saturating_add(FLOOR);
    let available = headroom.map_or(0, |headroom| headroom.saturating_sub(outstanding));
    let admission = match headroom {
        Some(headroom) if headroom >= need => {
            if available >= need { Admission::Admit } else { Admission::Defer }
        }
        _ => Admission::Refuse,
    };
    Decision { admission, headroom, outstanding, available, need }
}

/// rc.6 window admission (primary 2026-11-09 13:28Z; Eden `final-e39-xps-hot` hs08b on `e39479df`: the window
/// `[42, 44, 1c]` was planned at 27.6 MB of headroom, both supplements were acquired and the page commit aborted in the
/// build). How many of a switch's new window's supplemental members, in the planner's order, the process can hold, and
/// the decision that stopped the count, if one did. The prefix `supplements[..k]` is admitted while [`admit`] admits
/// its [`group_cost`] against the headroom less what every other consumer still holds: a group prefetch's rule and
/// margin. A `Defer` stops it too: the window is built at this request and cannot wait. The target is not charged: it is
/// the visible request, never refused, and its Reload destroys the outgoing window first.
pub fn window_extras(headroom: Option<u64>, outstanding: u64, supplements: &[u32]) -> (usize, Option<Decision>) {
    for k in 1..=supplements.len() {
        let decision = admit(headroom, outstanding, group_cost(&supplements[..k]));
        if decision.admission != Admission::Admit {
            return (k - 1, Some(decision));
        }
    }
    (supplements.len(), None)
}

/// What a window that [`window_extras`] cut waits for: the refused supplements' cost with the margin, 0 when every one
/// was admitted. Created entries leave it ([`created_admission`]).
pub fn window_waiting(supplements: &[u32], admitted: usize) -> u64 {
    supplements.get(admitted..).filter(|refused| !refused.is_empty())
        .map_or(0, |refused| with_margin(group_cost(refused).total()))
}

/// A created entry's creation or speculative load (primary 2026-11-09 13:28Z: created entries give way to a window or a
/// group that needs the room; operator 13:20Z: what the player's cycle needs gets memory first). It is admitted only
/// when the headroom also leaves `waiting`, what the cycle's refused window supplements and memory-held groups need
/// ([`window_waiting`]); otherwise it is deferred and stays planned. With nothing waiting it is [`admit`].
pub fn created_admission(headroom: Option<u64>, outstanding: u64, waiting: u64, cost: GroupCost) -> Decision {
    admit(headroom, outstanding.saturating_add(waiting), cost)
}

