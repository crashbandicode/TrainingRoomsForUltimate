//! CPU-WINDOW-REFORM: the window share of the saved slot count with group swap off (C116.1;
//! `D175/refactor/gs-panic-pool14/PROGRESS.md` §13).
//!
//! FACT (Yoga `swoff2`, M4d release, CPU role, `group_swap=off`, `slots=5`; `eden-sweep/yoga/analysis/swoff2/
//! window-budget.csv` `793c2677…`): steps 1–78 built a CPU window every third step with 3 resident helper sources;
//! at step 79 (Sora, whose next lane entries are Pyra, PT, IC and the Miis) no window could form, the bank was
//! retired canonically, the helpers grew to 5, and no CPU window formed again (lap 2, steps 87–97, all helper hits).
//!
//! Root cause (the planner before this module, `native_body_bank::plan_budget`/`proposed`):
//! - a role's window demand was read from the next `MAX_TARGETS` lane entries only, so a lookahead of kinds that
//!   cannot be window members gave it no demand and no reservation;
//! - the share was `slots − warm helpers`, and the helper cap `slots − reserved windows`: each consumer's budget was
//!   what the other had left, so whichever claimed the slots kept them;
//! - helper sources cannot be released within a scene (`SourceBook` has no release of a ready source; only native
//!   destruction or scene departure retires one), so the share the helpers took at step 79 never came back.
//!
//! The rule here: with group swap off, a role that cycles reserves the extras its **whole cycle** can use
//! ([`cycle_window_demand`]), out of the **saved slot count**, before the helpers; the helpers get what is left
//! ([`groups_off_shares`]). Whether a window can be built at a request is still decided on the physical state (slots
//! less the helper sources present and the other role's extras), so a window forms again as soon as the lane
//! reaches window kinds.

use super::{NATIVE_MEMBERS, POPO};
#[cfg(test)]
use super::reserve_extras;
use crate::engine::swap::swap_model::capabilities::bankable_kind;

/// The window extras a cycling role can use in this scene with group swap off: one fewer than the distinct kinds of
/// its cycle that can be window members, at most `bank_members − 1`. Not the next `MAX_TARGETS` entries: a run of
/// kinds that cannot be members (PT, Pyra/Mythra, Ice Climbers, the Miis) at the end of a lap would otherwise read as
/// no demand. A cycle `0x4b` counts only as Solo Popo (`solo_popo_member`, the rule of `body_bank::member_lane`).
pub fn cycle_window_demand(cycle: &[u32], bank_members: u8, solo_popo_member: bool) -> usize {
    let mut kinds = 0u128;
    for kind in cycle {
        if *kind < 128 && bankable_kind(*kind) && (*kind != POPO || solo_popo_member) {
            kinds |= 1u128 << kind;
        }
    }
    (kinds.count_ones() as usize)
        .saturating_sub(1)
        .min(usize::from(bank_members.saturating_sub(1)))
        .min(NATIVE_MEMBERS - 1)
}

/// Group swap off: the windows reserve their extras out of the saved slot count first (`reserve_extras`, existing
/// windows keep theirs), and the helper sources may hold the rest. Returns (per-role window reservation, helper cap).
#[cfg(test)]
pub fn groups_off_shares(wanted: [usize; 2], live: [usize; 2], slots: usize) -> ([usize; 2], usize) {
    let reserved = reserve_extras(wanted, live, slots);
    (reserved, slots.saturating_sub(reserved.iter().sum::<usize>()))
}

#[cfg(test)]
mod tests {
    //! Replay of Yoga `swoff2` (CPU role, slots 5, bank_members 3, P1 not cycling): the lane of
    //! `window-budget.csv`, one request per step, the planner once between steps, helper sources that grow to their
    //! cap and are never released.
    lib_unit_tests! {
    use super::*;
    use crate::engine::swap::body_bank::{member_lane, Members};
    use crate::engine::preload::cycle_preload_capacity::MAX_TARGETS;

    /// `window-budget.csv` column `kind`, seq 1–97 (one lap of 86, then lap 2 from Mario).
    const SWOFF2: [u32; 97] = [
        0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x10, 0x11, 0x12, 0x13, 0x14,
        0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x27, 0x28, 0x29,
        0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b,
        0x3c, 0x3d, 0x3e, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57,
        0x58, 0x59, 0x5c, 0x5d, 0x5a, 0x24, 0x4b, 0x49, 0x48, 0x4a, 0x3f, 0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8,
        0x9, 0xa,
    ];
    const LAP: usize = 86;
    const SLOTS: usize = 5;
    const BANK_MEMBERS: u8 = 3;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Route { WindowBuild, InWindow, Canonical }

    /// The `MAX_TARGETS` lane entries after position `at` of the cyclic lap.
    fn lookahead(lap: &[u32], at: usize) -> [u32; MAX_TARGETS] {
        std::array::from_fn(|i| lap[(at + 1 + i) % lap.len()])
    }

    /// A window for `target` with at most `budget` extras: the runtime's `window_lane` (`member_lane`, the CPU's Ice
    /// Climbers are the pair, so a lane `0x4b` is no member) and `members_for` for a single-body target.
    fn window(target: u32, lane: &[u32], budget: usize) -> Option<Members> {
        if !bankable_kind(target) { return None; }
        let (lane, count) = member_lane(lane, false);
        Members::from_cycle(target, &lane[..count], budget)
    }

    /// The planner's lookahead demand: the first lane entry that can form a window.
    fn lookahead_demand(lane: &[u32; MAX_TARGETS]) -> usize {
        (0..MAX_TARGETS).find_map(|i| window(lane[i], &lane[i + 1..], usize::from(BANK_MEMBERS - 1)))
            .map_or(0, |m| m.count() - 1)
    }

    /// One step-by-step replay. `fixed`: the rule of this module; otherwise the rule it replaces.
    fn replay(lap: &[u32], steps: &[u32], fixed: bool) -> Vec<(Route, usize)> {
        let mut helpers = 0usize;
        let mut live = 0usize;
        let mut members: Option<Members> = None;
        let mut position = 0usize;
        let plan = |helpers: &mut usize, live: usize, position: usize| {
            let lane = lookahead(lap, position);
            let wanted = lookahead_demand(&lane);
            let cap = if fixed {
                groups_off_shares([0, wanted.max(cycle_window_demand(lap, BANK_MEMBERS, false))], [0, live], SLOTS).1
            } else {
                // Window share = slots less the helpers present; helper cap = slots less the windows reserved.
                let reserved = reserve_extras([0, wanted], [0, live], SLOTS.saturating_sub(*helpers));
                SLOTS.saturating_sub(reserved.iter().sum::<usize>())
            };
            // Helper sources fill up to their cap and are never released within the scene.
            *helpers = (*helpers).max(cap);
        };
        plan(&mut helpers, live, lap.len() - 1);
        let mut out = Vec::new();
        for &kind in steps {
            position = lap.iter().position(|k| *k == kind).unwrap_or(position);
            let route = if members.is_some_and(|m| m.index_of(kind).is_some()) {
                Route::InWindow
            } else {
                // The request: the physical pool (slots less the helper sources present).
                let budget = SLOTS.saturating_sub(helpers).min(usize::from(BANK_MEMBERS - 1));
                match window(kind, &lookahead(lap, position), budget) {
                    Some(m) => { members = Some(m); live = m.count() - 1; Route::WindowBuild }
                    None => { members = None; live = 0; Route::Canonical }
                }
            };
            out.push((route, helpers));
            plan(&mut helpers, live, position);
        }
        out
    }

    fn swoff2_a_window_forms_again_once_the_lane_returns_to_window_kinds() {
        let lap = &SWOFF2[..LAP];
        let routes = replay(lap, &SWOFF2, true);
        for (i, (route, helpers)) in routes[..78].iter().enumerate() {
            let expected = if i % 3 == 0 { Route::WindowBuild } else { Route::InWindow };
            assert_eq!((*route, *helpers), (expected, 3), "step {}", i + 1);
        }
        // The special tail (steps 79-85: Sora, Pyra, PT, IC, the Miis) stays canonical and the helpers keep 3 of
        // the 5 slots.
        for (i, (route, helpers)) in routes[78..85].iter().enumerate() {
            assert_eq!((*route, *helpers), (Route::Canonical, 3), "step {}", 79 + i);
        }
        // From the first window kind after it (step 86, the CPU original 0x3f, then lap 2) a window forms again and
        // every third step is a build, the others in-window, through step 97.
        for (i, (route, helpers)) in routes[85..].iter().enumerate() {
            let expected = if i % 3 == 0 { Route::WindowBuild } else { Route::InWindow };
            assert_eq!((*route, *helpers), (expected, 3), "step {}", 86 + i);
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("swoff2_a_window_forms_again_once_the_lane_returns_to_window_kinds", swoff2_a_window_forms_again_once_the_lane_returns_to_window_kinds as fn()),
        ]);
    }
    }
}
