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

use super::{reserve_extras, NATIVE_MEMBERS, POPO};
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
pub fn groups_off_shares(wanted: [usize; 2], live: [usize; 2], slots: usize) -> ([usize; 2], usize) {
    let reserved = reserve_extras(wanted, live, slots);
    (reserved, slots.saturating_sub(reserved.iter().sum::<usize>()))
}

