//! One role's cycle, as the menu numbers it and every walker walks it (P-CYCLE; operator 2026-11-05: "mii gunner is in
//! the cycle somehow for cpu even though he's not listed in the menu").
//!
//! - [`cycle_members`]: the roster's own order (All in CSS order, otherwise the ranks) with a Mii class kept only when
//!   the role admits it. The CPU Mii owner's walker, Cycle Preload, the body bank's plan and the plain loader's
//!   `roster_fixed` walk this list, and the portrait grid numbers it.
//! - [`numbered`]: what the grid numbers 1, 2, … and the order the walkers step through: slot 0's fighter group out, one
//!   entry per fighter group and per Mii class (the rules of `mii_cycle::next_cycle`).
//! - [`cpu_admission`]: which Mii classes the CPU cycle admits. The CPU Mii owner computes it from its own state and
//!   publishes it; the menu reads the publication.

use crate::engine::mii::inventory::{Canonical, MiiClass, Snapshot};
use crate::engine::mii::selection::{self, PlayerOrigin, Role};
use crate::engine::swap::character_forms::Forms;
use crate::engine::swap::ordered_roster::{FighterList, KindList, OrderedFighterRoster};
use crate::engine::swap::trio::TrioFighter;

/// Per Mii class (`[Brawler, Swordfighter, Gunner]`): whether a role's cycle admits it.
pub type MiiAdmission = [bool; 3];

/// No Mii class: the plain loader's cycle (it cannot load a Mii), and every cycle while no Mii owner is qualified.
pub const NO_MII: MiiAdmission = [false; 3];

const CLASSES: [MiiClass; 3] = [MiiClass::Brawler, MiiClass::Swordfighter, MiiClass::Gunner];

/// The class index of a Mii class kind (`0x48..=0x4a`).
pub fn mii_index(kind: u32) -> Option<usize> {
    (0x48..=0x4a).contains(&kind).then(|| (kind - 0x48) as usize)
}

/// The typed roster of a portrait grid's `[all, ranks...]` values, read as the menu's typed view reads them
/// (`OrderedFighterRoster`'s own deserializer); values it refuses are All, the grid's own fallback (`RosterEditor::values`).
pub fn roster_of(values: &[u8]) -> OrderedFighterRoster {
    serde_json::from_value(serde_json::Value::from(values.to_vec())).unwrap_or(OrderedFighterRoster::all())
}

/// The ordered members of one role's cycle, before slot 0 is taken out: All in CSS order, otherwise the ranks in rank
/// order, with each Mii class kept only when `admitted`. Allocation-free (the planner's per-tick form).
pub fn cycle_members(roster: OrderedFighterRoster, admitted: MiiAdmission) -> FighterList {
    cycle_members_excluding(roster, admitted, None)
}

/// [`cycle_members`] with `original`'s native group left out, as `OrderedFighterRoster::fighters_excluding_fixed`
/// does (the plain loader's `roster_fixed`).
pub fn cycle_members_excluding(roster: OrderedFighterRoster, admitted: MiiAdmission, original: Option<u32>) -> FighterList {
    let mut out = FighterList::new(TrioFighter::MARIO);
    for fighter in roster.fighters_excluding_fixed(original).iter() {
        if mii_index(fighter.kind()).is_none_or(|class| admitted[class]) {
            out.push(*fighter);
        }
    }
    out
}

/// The numbered order of a cycle (what the grid numbers 1, 2, …): `members` with the fighter group of `excluded` taken
/// out (slot 0: the role's original after `roster_grid::cycle_exclusion`; a Mii class original takes no fighter group
/// out), and one entry per fighter group and per Mii class, as `mii_cycle::next_cycle` builds its choices.
pub fn numbered(members: &[TrioFighter], excluded: Option<u32>) -> KindList {
    let mut out = KindList::new(0);
    let slot_zero = excluded.filter(|kind| mii_index(*kind).is_none());
    for fighter in members {
        let kind = fighter.kind();
        let taken = if mii_index(kind).is_some() {
            out.contains(&kind)
        } else {
            let forms = Forms::starting(kind);
            slot_zero.is_some_and(|original| forms.same_group(original))
                || out.iter().any(|k| mii_index(*k).is_none() && forms.same_group(*k))
        };
        if !taken {
            out.push(kind);
        }
    }
    out
}

/// The 1-based number of `kind`'s entry in a numbered order (its fighter group, or its Mii class).
pub fn number_of(order: &[u32], kind: u32) -> Option<usize> {
    let same = |k: u32| match (mii_index(k), mii_index(kind)) {
        (Some(a), Some(b)) => a == b,
        (None, None) => Forms::starting(k).same_group(kind),
        _ => false,
    };
    order.iter().position(|k| same(*k)).map(|i| i + 1)
}

/// The CPU cycle's Mii admission. A class is in when the CPU Mii owner is `qualified` (the condition under which it
/// serves the cycle), the CPU loads the class (`ordinary_cpu::supported_kind`), and the catalog has a configuration of
/// it other than the CPU's captured saved original (slot 0): a saved Mii or a `mii-presets.json` preset. That is when
/// `mii_cycle::numbered_saved` resolves the class for the CPU (for `Role::Cpu` eligibility ignores the origin, and a
/// preference only picks which configuration resolves, not whether one does). Allocation-free.
pub fn cpu_admission(qualified: bool, catalog: &Snapshot, original: Option<Canonical>) -> MiiAdmission {
    let mut admitted = NO_MII;
    if !qualified {
        return admitted;
    }
    for (slot, class) in admitted.iter_mut().zip(CLASSES) {
        *slot = crate::engine::mii::ordinary_cpu::supported_kind(class.kind())
            && selection::eligible(catalog, Role::Cpu, class, PlayerOrigin::NotMii).any(|cfg| Some(cfg.canonical) != original);
    }
    admitted
}

/// The Player cycle's Mii admission. A class is in when the Player walker would load it: `selection::eligible` for
/// `Role::Player` yields a configuration (`class_selectable`). That is when `mii_cycle::numbered_saved` resolves the
/// class. For the Player, eligibility already excludes the origin configuration, so the origin's own class is in only
/// when another configuration of it exists. A preference only picks which configuration resolves. The live fighter
/// being a Mii, and a hat match, are not part of this: those are `alpha_player_cycle`, the Player-pick rule. The
/// Player Mii owner computes this from the catalog and origin it walks and publishes it; the menu and every Player
/// walker read the publication (P1-CYCLE). Allocation-free.
pub fn player_admission(catalog: &Snapshot, origin: PlayerOrigin) -> MiiAdmission {
    let mut admitted = NO_MII;
    for (slot, class) in admitted.iter_mut().zip(CLASSES) {
        *slot = selection::class_selectable(catalog, Role::Player, class, origin);
    }
    admitted
}

