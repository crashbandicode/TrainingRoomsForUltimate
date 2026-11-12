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

#[cfg(test)]
mod tests {
    //! Replays of the logged sequences. Headroom values are the `[GroupSwap] prefetch … headroom=` admission samples
    //! and `[PreloadMetrics]` records of the cited lines; costs are the measured table.
    lib_unit_tests! {
    use super::*;

    /// The rule this module replaces: 96 MiB per body of the admitted group, against the current headroom only.
    fn old_rule(headroom: u64, bodies: u64) -> bool {
        headroom >= (96 << 20) * bodies
    }

    fn cost(kinds: &[u32]) -> GroupCost {
        group_cost(kinds)
    }

    /// s21 (M5c release `6b63a740`, arm B lap 1): group 0 `[40,41,42]` admitted at L7770, group 1 `[43]` at L7804
    /// while group 0 was prefetched; group 0 built at L7836–L7846, `[43]` built at L7889, PANIC L7891.
    fn s21_old_rule_admits_simon_on_headroom_group_zero_still_needs() {
        let (group0, simon) = (cost(&[0x40, 0x41, 0x42]), cost(&[0x43]));
        assert!(old_rule(309_547_008, 3));
        assert!(old_rule(122_245_120, 1), "L7804: 122,245,120 >= 96 MiB");
        // What was left for Simon's build: the headroom after its own prefetch (L7821: 50,089,984) less group 0's
        // build (L7846: committed 3394482176 -> 3423842304), as L7846 logged.
        let left: u64 = 50_089_984 - (3_423_842_304 - 3_394_482_176);
        assert_eq!(left, 20_729_856);
        assert!(simon.build > 21_319_680, "L7876 headroom before the build is below Simon's measured build");
        assert!(group0.build >= 3_423_842_304 - 3_394_482_176);
    }

    fn s21_simon_is_refused_and_group_zero_is_still_admitted() {
        let group0 = cost(&[0x40, 0x41, 0x42]);
        assert_eq!(admit(Some(309_547_008), 0, group0).admission, Admission::Admit, "L7770 stays warm");
        let held = outstanding(GroupState::Prefetched, group0);
        assert_eq!(held, with_margin(group0.build));
        let decision = admit(Some(122_245_120), held, cost(&[0x43]));
        assert_eq!(decision.admission, Admission::Refuse, "{:?}", decision);
        assert!(decision.need > 122_245_120);
    }

    /// s19 (B123-pre `70d1be45`, arm A lap 1): group 0 `[41,42,43]` admitted at L9037, group 1 `[44]` at L9084,
    /// group 0 built at L9128–L9144 (committed +50,331,648), `[44]` built at L9219, PANIC L9221.
    fn s19_richter_waits_for_group_zero_and_then_falls_back() {
        let group0 = cost(&[0x41, 0x42, 0x43]);
        let richter = cost(&[0x44]);
        assert!(old_rule(143_216_640, 1));
        assert_eq!(admit(Some(320_229_376), 0, group0).admission, Admission::Admit);
        let held = outstanding(GroupState::Prefetched, group0);
        assert_eq!(admit(Some(143_216_640), held, richter).admission, Admission::Defer);
        // Group 0 built: its charge is in the headroom and it holds nothing; Richter's prefetch never ran.
        let after_build = 143_216_640 - (3_419_975_680 - 3_369_644_032);
        assert_eq!(outstanding(GroupState::Built, group0), 0);
        assert_eq!(admit(Some(after_build), 0, richter).admission, Admission::Refuse);
    }

    /// s19 L7255: Game & Watch `[1c]` admitted by the old rule at 172,183,552; its build left 675,840 bytes (L7425).
    fn s19_game_and_watch_near_miss_is_refused() {
        assert!(old_rule(172_183_552, 1));
        let gnw = cost(&[0x1c]);
        assert!(gnw.total() > 96 << 20, "measured {} MiB", gnw.total() / MIB);
        let held = outstanding(GroupState::Prefetched, cost(&[0x19, 0x1a, 0x1b]));
        assert_eq!(admit(Some(172_183_552), held, gnw).admission, Admission::Refuse);
    }

    /// s21 L7579/L7612: `[39,3a,3b]` then `[3c]`, both built (L7644, L7698) with 155 MiB left: still admitted.
    fn s21_a_lap_with_room_keeps_both_groups_warm() {
        let group0 = cost(&[0x39, 0x3a, 0x3b]);
        assert_eq!(admit(Some(429_936_640), 0, group0).admission, Admission::Admit);
        let held = outstanding(GroupState::Prefetched, group0);
        assert_eq!(admit(Some(258_035_712), held, cost(&[0x3c])).admission, Admission::Admit);
    }

    fn a_set_measured_whole_uses_its_measurement_and_others_sum_their_members() {
        let measured = SET_COSTS.iter().find(|(set, ..)| *set == [0x40, 0x41, 0x42]).unwrap();
        assert_eq!(cost(&[0x42, 0x40, 0x41]), GroupCost { prefetch: measured.1, build: measured.2 });
        let unmeasured = cost(&[0x07, 0x08, 0x0d]);
        assert_eq!(unmeasured.build, kind_cost(0x07).build + kind_cost(0x08).build + kind_cost(0x0d).build);
        assert_eq!(kind_cost(0x4e), GroupCost { prefetch: DEFAULT_PREFETCH, build: DEFAULT_BUILD });
    }

    fn only_admitted_unbuilt_groups_hold_memory_and_an_unreadable_headroom_refuses() {
        let c = GroupCost { prefetch: 60 * MIB, build: 10 * MIB };
        assert_eq!(outstanding(GroupState::Prefetching, c), with_margin(70 * MIB));
        assert_eq!(outstanding(GroupState::Prefetched, c), with_margin(10 * MIB));
        for state in [GroupState::Planned, GroupState::Built, GroupState::Failed] {
            assert_eq!(outstanding(state, c), 0);
        }
        assert_eq!(admit(None, 0, c).admission, Admission::Refuse);
        let need = with_margin(70 * MIB) + FLOOR;
        assert_eq!(admit(Some(need), 0, c).admission, Admission::Admit);
        assert_eq!(admit(Some(need - 1), 0, c).admission, Admission::Refuse);
        assert_eq!(admit(Some(need), 1, c).admission, Admission::Defer);
    }

    fn every_measured_kind_set_and_helper_load_is_charged_at_least_its_measurement() {
        for (kind, prefetch, build) in KIND_COSTS {
            let c = kind_cost(*kind);
            assert!(c.prefetch >= *prefetch && c.build >= *build, "kind {:#x}", kind);
        }
        for (set, prefetch, build, _, _) in SET_COSTS {
            let kinds: Vec<u32> = set.iter().copied().filter(|k| *k != u32::MAX).collect();
            let c = group_cost(&kinds);
            assert!(c.prefetch >= *prefetch && c.build >= *build, "set {:x?}", set);
        }
        for (kind, bytes) in HELPER_COSTS {
            assert!(helper_cost(*kind).total() >= *bytes, "helper {:#x}", kind);
        }
    }

    /// One observation of a set is not a spread: the larger of it and the members' sum.
    fn a_set_seen_once_takes_the_larger_of_its_observation_and_the_members_sum() {
        for (set, prefetch, build, prefetch_n, build_n) in SET_COSTS {
            let kinds: Vec<u32> = set.iter().copied().filter(|k| *k != u32::MAX).collect();
            let summed = kinds.iter().fold(0u64, |sum, k| sum + kind_cost(*k).build);
            let c = group_cost(&kinds);
            match build_n {
                0 => assert_eq!(c.build, summed),
                1 => assert_eq!(c.build, (*build).max(summed)),
                _ => assert_eq!(c.build, *build),
            }
            if *prefetch_n >= 2 { assert_eq!(c.prefetch, *prefetch); }
        }
    }

    /// s30 (M5e release `5c6ea087`, R-GSP4 lap 1): group 0 `[46,47,51]` admitted at L8177 (299,257,856), group 1
    /// `[52]` refused at L8213, group 0 built at L8241 (66,015,232 left), then at L8292 the helper slot owner 2
    /// speculatively reloaded Joker `0x52` (L8282: 66,080,768) and the page commit aborted at L8298.
    fn s30_the_joker_helper_preload_is_refused_and_the_groups_decide_as_before() {
        assert_eq!(admit(Some(299_257_856), 0, cost(&[0x46, 0x47, 0x51])).admission, Admission::Admit);
        let held = outstanding(GroupState::Prefetched, cost(&[0x46, 0x47, 0x51]));
        assert_eq!(admit(Some(107_237_376), held, cost(&[0x52])).admission, Admission::Refuse);
        let joker = helper_cost(0x52);
        assert!(joker.total() >= 124_715_008, "at least the largest measured Joker helper load");
        let decision = admit(Some(66_080_768), 0, joker);
        assert_eq!(decision.admission, Admission::Refuse, "{:?}", decision);
    }

    /// s30 L8095/L8102/L8112: the Simon helper preload at 105,074,688 settled at 73,224,192 (net +31,850,496 after
    /// Palutena's release). Its gross cost is not known before the load, so it is charged the group cost and waits.
    fn s30_a_helper_preload_is_charged_its_gross_cost() {
        let simon = helper_cost(0x43);
        assert!(simon.total() >= cost(&[0x43]).total());
        assert_eq!(admit(Some(105_074_688), 0, simon).admission, Admission::Refuse);
    }

    /// The two consumers see each other: a group waits while a helper load holds its cost, and a helper preload
    /// waits while a prefetched group still has to build.
    fn groups_and_helper_loads_hold_memory_against_each_other() {
        let group0 = cost(&[0x46, 0x47, 0x51]);
        let joker_in_flight = helper_outstanding(helper_cost(0x52));
        assert_eq!(admit(Some(299_257_856), joker_in_flight, group0).admission, Admission::Defer);
        let small = helper_cost(0x3c);
        let room = with_margin(small.total()) + FLOOR;
        let held = outstanding(GroupState::Prefetched, group0);
        assert_eq!(admit(Some(room), 0, small).admission, Admission::Admit);
        assert_eq!(admit(Some(room), held, small).admission, Admission::Defer);
    }

    fn native_groups_load_whole_on_a_helper() {
        assert_eq!(helper_members(0x25), ([0x24, 0x25, 0x26], 3));
        assert_eq!(helper_members(0x5b), ([0x5a, 0x5b, u32::MAX], 2));
        assert_eq!(helper_members(0x4b), ([0x4b, 0x4c, u32::MAX], 2));
        assert_eq!(helper_members(0x52), ([0x52, u32::MAX, u32::MAX], 1));
        assert!(helper_cost(0x24).total() >= cost(&[0x24, 0x25, 0x26]).total());
    }

    /// tr-p23-hot-measure-1 (release `031f1eae`), the CPU cycle under All after the Mii stretch: the window
    /// `[34,0,1]` was built at frame 80722 and its next group `[2,3,4]` met the plugin's own svcGetInfo reads of
    /// 2,943,725,568 B used (frame 80686) and 2,950,082,560 B (80891) of 3,444,572,160 B. The group's need
    /// (prefetch + build summed per kind, margin, floor) is above both headrooms, so the admission refuses it,
    /// the cause of g03 frame 85's "Loaded 2/5 (3 failed)". The same group is admitted once the headroom
    /// covers its need; the start is therefore held and retried, never failed (`GroupState::Held`).
    fn frame85_group_refused_by_headroom_then_admitted() {
        const BUDGET: u64 = 3_444_572_160;
        let group = cost(&[0x2, 0x3, 0x4]);
        let need = with_margin(group.total()) + FLOOR;
        assert!((530 * MIB..531 * MIB).contains(&need), "need {} MiB", need / MIB);
        for used in [2_943_725_568u64, 2_950_082_560] {
            let decision = admit(Some(BUDGET - used), 0, group);
            assert_eq!(decision.admission, Admission::Refuse, "{decision:?}");
        }
        assert_eq!(admit(Some(need), 0, group).admission, Admission::Admit);
        assert_eq!(outstanding(GroupState::Held(super::super::Hold::Memory), group), 0, "a held group holds no memory");
    }

    /// Eden `final-e39-xps-hot` hs08b (`e39479df`, 2026-11-09 13:03:52Z): the CPU's switch to `0x42` planned the window
    /// `[42, 44, 1c]` (`#3404`) with 27,611,136 bytes of headroom (`#3391`, the last sample before it), acquired both
    /// supplements unconditionally (`#3421 supplemental=[44, 1c]`) and the game's page commit aborted in the build.
    /// Simon (65.2 + 35.1 MiB) and Game & Watch (53.1 + 98.4 MiB) each need more than that: the window keeps neither,
    /// and the switch is the canonical single-body Reload of the target.
    fn hs08b_window_keeps_no_supplement_at_its_headroom() {
        let (admitted, stopped) = window_extras(Some(27_611_136), 0, &[0x44, 0x1c]);
        assert_eq!(admitted, 0);
        let stopped = stopped.expect("the decision that refused Simon");
        assert_eq!(stopped.admission, Admission::Refuse, "{stopped:?}");
        assert_eq!(stopped.need, with_margin(cost(&[0x44]).total()) + FLOOR);
    }

    /// The same rule at the headrooms the window planner met in the two `xps-hot` runs: the supplements are the
    /// planner's prefix that fits (`[5c, 53, 45]`, `#3257` on `e39479df` at 73,420,800 and `#3138` on `646a5f0d` at
    /// 79,646,720, keep the target alone), and more of them fit as the headroom grows.
    fn window_supplements_shrink_as_the_headroom_falls() {
        let both = with_margin(cost(&[0x53, 0x45]).total()) + FLOOR;
        let first = with_margin(cost(&[0x53]).total()) + FLOOR;
        assert!(first < both);
        assert_eq!(window_extras(Some(both), 0, &[0x53, 0x45]).0, 2);
        assert_eq!(window_extras(Some(both - 1), 0, &[0x53, 0x45]).0, 1);
        assert_eq!(window_extras(Some(first), 0, &[0x53, 0x45]).0, 1);
        assert_eq!(window_extras(Some(first - 1), 0, &[0x53, 0x45]).0, 0);
        for headroom in [73_420_800u64, 79_646_720] {
            assert_eq!(window_extras(Some(headroom), 0, &[0x53, 0x45]).0, 0, "{headroom}");
        }
        assert_eq!(window_extras(None, 0, &[0x53]).0, 0, "an unread headroom admits nothing");
        assert_eq!(window_extras(Some(0), 0, &[]), (0, None), "a window with no supplement asks nothing");
    }

    /// `e39479df` `#3015`: the CPU's window `[0, 58, 59]` at 310,005,760 bytes (`#2937`) keeps both supplements: with room
    /// the windows are what they were.
    fn a_window_at_ample_headroom_keeps_both_supplements() {
        assert_eq!(window_extras(Some(310_005_760), 0, &[0x58, 0x59]), (2, None));
    }

    /// What admitted groups have not spent yet (a prefetched group's build) is not the window's: it takes the
    /// supplements the rest admits, as a group prefetch or a helper preload does.
    fn admitted_groups_still_hold_their_part_against_a_window() {
        let both = with_margin(cost(&[0x53, 0x45]).total()) + FLOOR;
        let held = outstanding(GroupState::Prefetched, cost(&[0x42, 0x45]));
        assert!(held > 0);
        assert_eq!(window_extras(Some(both), 0, &[0x53, 0x45]).0, 2);
        let (admitted, stopped) = window_extras(Some(both), held, &[0x53, 0x45]);
        assert_eq!(admitted, 1);
        assert_eq!(stopped.map(|d| d.admission), Some(Admission::Defer), "the headroom covers it once the group builds");
    }

    /// Primary 2026-11-09 13:28Z ("created entries … must yield to a window or group that needs the room"; operator
    /// 13:20Z: what the player's cycle needs gets memory first). A created entry's hat load (Falco, the hs06 plan's first
    /// hat) that today's admission takes is not admitted while the supplements a window was refused wait for the room:
    /// it would take the memory the cycle's next window needs. It is deferred, not refused: it stays planned.
    fn a_created_entry_gives_way_to_what_the_cycle_waits_for() {
        let hat = helper_cost(0x14);
        let room = with_margin(hat.total()) + FLOOR;
        assert_eq!(admit(Some(room), 0, hat).admission, Admission::Admit);
        let (admitted, _) = window_extras(Some(room), 0, &[0x44, 0x1c]);
        let waiting = window_waiting(&[0x44, 0x1c], admitted);
        assert_eq!(waiting, with_margin(cost(&[0x44, 0x1c]).total()), "both refused supplements wait");
        let decision = created_admission(Some(room), 0, waiting, hat);
        assert_eq!(decision.admission, Admission::Defer, "{decision:?}");
        assert_eq!(created_admission(Some(room + waiting), 0, waiting, hat).admission, Admission::Admit,
            "with room for both, the hat loads beside the window's wait");
        assert_eq!(window_waiting(&[0x44, 0x1c], 2), 0, "an admitted window leaves nothing waiting");
    }

    /// "Hats still preload when there's room": with nothing waiting, a created entry is admitted exactly as before.
    fn a_created_entry_loads_as_before_when_nothing_waits() {
        for kind in [0x14u32, 0x10, 0x11, 0x18, 0x1a] {
            let hat = helper_cost(kind);
            for headroom in [None, Some(0), Some(with_margin(hat.total()) + FLOOR - 1), Some(with_margin(hat.total()) + FLOOR), Some(800 * MIB)] {
                for held in [0, 40 * MIB] {
                    assert_eq!(created_admission(headroom, held, 0, hat), admit(headroom, held, hat), "{kind:#x} {headroom:?} {held}");
                }
            }
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("s21_old_rule_admits_simon_on_headroom_group_zero_still_needs", s21_old_rule_admits_simon_on_headroom_group_zero_still_needs as fn()),
            ("s21_simon_is_refused_and_group_zero_is_still_admitted", s21_simon_is_refused_and_group_zero_is_still_admitted as fn()),
            ("s19_richter_waits_for_group_zero_and_then_falls_back", s19_richter_waits_for_group_zero_and_then_falls_back as fn()),
            ("s19_game_and_watch_near_miss_is_refused", s19_game_and_watch_near_miss_is_refused as fn()),
            ("s21_a_lap_with_room_keeps_both_groups_warm", s21_a_lap_with_room_keeps_both_groups_warm as fn()),
            ("a_set_measured_whole_uses_its_measurement_and_others_sum_their_members", a_set_measured_whole_uses_its_measurement_and_others_sum_their_members as fn()),
            ("only_admitted_unbuilt_groups_hold_memory_and_an_unreadable_headroom_refuses", only_admitted_unbuilt_groups_hold_memory_and_an_unreadable_headroom_refuses as fn()),
            ("every_measured_kind_set_and_helper_load_is_charged_at_least_its_measurement", every_measured_kind_set_and_helper_load_is_charged_at_least_its_measurement as fn()),
            ("a_set_seen_once_takes_the_larger_of_its_observation_and_the_members_sum", a_set_seen_once_takes_the_larger_of_its_observation_and_the_members_sum as fn()),
            ("s30_the_joker_helper_preload_is_refused_and_the_groups_decide_as_before", s30_the_joker_helper_preload_is_refused_and_the_groups_decide_as_before as fn()),
            ("s30_a_helper_preload_is_charged_its_gross_cost", s30_a_helper_preload_is_charged_its_gross_cost as fn()),
            ("groups_and_helper_loads_hold_memory_against_each_other", groups_and_helper_loads_hold_memory_against_each_other as fn()),
            ("native_groups_load_whole_on_a_helper", native_groups_load_whole_on_a_helper as fn()),
            ("frame85_group_refused_by_headroom_then_admitted", frame85_group_refused_by_headroom_then_admitted as fn()),
            ("hs08b_window_keeps_no_supplement_at_its_headroom", hs08b_window_keeps_no_supplement_at_its_headroom as fn()),
            ("window_supplements_shrink_as_the_headroom_falls", window_supplements_shrink_as_the_headroom_falls as fn()),
            ("a_window_at_ample_headroom_keeps_both_supplements", a_window_at_ample_headroom_keeps_both_supplements as fn()),
            ("admitted_groups_still_hold_their_part_against_a_window", admitted_groups_still_hold_their_part_against_a_window as fn()),
            ("a_created_entry_gives_way_to_what_the_cycle_waits_for", a_created_entry_gives_way_to_what_the_cycle_waits_for as fn()),
            ("a_created_entry_loads_as_before_when_nothing_waits", a_created_entry_loads_as_before_when_nothing_waits as fn()),
        ]);
    }
    }
}
