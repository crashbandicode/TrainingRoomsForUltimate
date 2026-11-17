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

/// What a committed menu change asks of one role's loader (`character_roster::settings_changed`): `None`, or a Direct
/// load of the role's selected fighter. `pressed`: a portrait press for the role; `changed`: its selected fighter
/// changed; `cycle_turned_on`: Player Swapping just went On. `Some(once)`: `once` marks one load the user chose, a
/// pick, which loads whatever Player Swapping says, like a combo's Switch (operator 2026-11-13, TR8).
pub fn menu_request(pressed: bool, changed: bool, cycle_turned_on: bool) -> Option<bool> {
    let pick = pressed || changed;
    (pick || cycle_turned_on).then_some(pick)
}

/// Whether a role's loader takes a request: the role's cycle is on (Player Swapping for Player 1; the CPU's always is),
/// the request is one load the user chose (`once`), or it only refreshes the partners.
pub fn admitted(cycle_on: bool, once: bool, refresh: bool) -> bool {
    cycle_on || once || refresh
}

// ---------------------------------------------------------------------------------------------------------------------
// LEAD-1: explicit Player / CPU Character picks. A pick of the fighter a role already stores (a pick that never loaded,
// or the stored pick a cycle left behind) changed no setting, so nothing dispatched it.
//
// The dispatch is one versioned transaction (review R1/R2):
// - Every settings publish folds its own causes into one cumulative `DispatchRecord`, under `published::PUBLISHING`
//   (`DispatchChannel::publish`). The causes, per role, are the selection the publish changed, Player Swapping turning On
//   and the menu close's explicit pick. The record keeps the committed TARGET of each role and the publication sequence
//   number of that role's latest cause.
// - The game frame reads the record as one coherent copy (`hos_lock::Published`) and loads a role once per cause sequence
//   (`DispatchChannel::take`). The target comes from the record, not from the live typed menu, so a load completion that
//   writes a game-side edit after the publish cannot erase the pick (R1). A record re-read or re-noted never loads twice,
//   and an overlapping older publication cannot add a second load (R2), because a role loads only when its cause sequence
//   is newer than the one it last loaded.
//
// Contract: the LAST pick per role per menu visit wins. Earlier presses of the same role in the same visit are
// superseded (the menu's outbox keeps one value per role, `model::menu::Outbox::picks`); there is no per-press stream.
// A later publication's cause for the same role supersedes an earlier one the frame has not taken (the newest target
// loads).
// ---------------------------------------------------------------------------------------------------------------------

/// What one role's selection looks like in a typed menu: its picked fighter and (Player 1) whether Player Swapping is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleView {
    pub selected: u32,
    pub cycle_on: bool,
}

/// `[Player, CPU]` of a typed menu. The CPU's cycle is always on.
pub fn role_views(menu: &crate::engine::compat::menu::TrainingModpackMenu) -> [RoleView; 2] {
    [
        RoleView { selected: menu.player_character.kind(), cycle_on: menu.trio_practice == crate::engine::compat::menu::OnOff::ON },
        RoleView { selected: menu.cpu_character.kind(), cycle_on: true },
    ]
}

/// One publication's dispatch cause for one role (review F2): the selection it changed, Player Swapping turning On
/// (Player 1) and the close's explicit pick, decided together. `previous`: the role in the view the publish replaced
/// (None at boot: nothing to compare with); `next`: the committed role. `Some(once)`: one load (`once`: a load the user
/// chose, which Player Swapping Off does not refuse).
pub fn role_dispatch(player: bool, previous: Option<RoleView>, next: RoleView, picked: bool) -> Option<bool> {
    let changed = previous.is_some_and(|previous| previous.selected != next.selected);
    let cycle_turned_on = player && previous.is_some_and(|previous| !previous.cycle_on) && next.cycle_on;
    menu_request(picked, changed, cycle_turned_on)
}

/// One role's latest dispatch cause in the record: the fighter to load, whether it is a load the user chose, and the
/// publication sequence number of the cause (0: none yet).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoleDispatch {
    pub target: u32,
    pub once: bool,
    pub cause: u64,
}

/// The cumulative dispatch record: the last publication's sequence number and each role's latest cause.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DispatchRecord {
    pub seq: u64,
    pub roles: [RoleDispatch; 2],
}

impl DispatchRecord {
    /// The record after one more publication. An explicit pick names its own target (it is the committed pick: the close
    /// marks the pick key as the player's edit); otherwise the target is the committed selection.
    pub fn fold(self, previous: Option<[RoleView; 2]>, next: [RoleView; 2], picks: [Option<u32>; 2]) -> Self {
        let seq = self.seq + 1;
        let mut roles = self.roles;
        for (role, slot) in roles.iter_mut().enumerate() {
            if let Some(once) = role_dispatch(role == 0, previous.map(|views| views[role]), next[role], picks[role].is_some()) {
                *slot = RoleDispatch { target: picks[role].unwrap_or(next[role].selected), once, cause: seq };
            }
        }
        Self { seq, roles }
    }
}

/// The publish → game frame dispatch transaction (one per runtime, `compat::menu::DISPATCH`). The publisher folds and
/// publishes under `published::PUBLISHING` (one writer); the game frame reads a coherent copy and records, per role, the
/// cause sequence it last loaded. Neither side takes a lock the other holds: `Published` readers never wait, and the frame's
/// receipts are atomics only it writes.
pub struct DispatchChannel {
    record: hos_lock::Published<DispatchRecord>,
    seen: core::sync::atomic::AtomicU32,
    loaded: [core::sync::atomic::AtomicU64; 2],
}

impl Default for DispatchChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl DispatchChannel {
    pub const fn new() -> Self {
        Self {
            record: hos_lock::Published::new(DispatchRecord { seq: 0, roles: [RoleDispatch { target: 0, once: false, cause: 0 }; 2] }),
            seen: core::sync::atomic::AtomicU32::new(0),
            loaded: [core::sync::atomic::AtomicU64::new(0), core::sync::atomic::AtomicU64::new(0)],
        }
    }

    /// The publisher (`compat::menu::after_publish`, under `PUBLISHING`): fold this publication into the record and publish
    /// it. Returns the record it published.
    pub fn publish(&self, previous: Option<[RoleView; 2]>, next: [RoleView; 2], picks: [Option<u32>; 2]) -> DispatchRecord {
        let record = self.record.read().fold(previous, next, picks);
        self.record.publish(record);
        record
    }

    /// The game frame (`compat::menu::service_roster_change`): each role's load this frame owes, from one coherent record,
    /// at most once per cause. One atomic load when nothing was published.
    pub fn take(&self) -> [Option<RoleDispatch>; 2] {
        let Some(record) = self.record.read_if_newer(&self.seen) else { return [None; 2] };
        let mut due = [None; 2];
        for (role, slot) in due.iter_mut().enumerate() {
            let cause = record.roles[role];
            if cause.cause > self.loaded[role].load(core::sync::atomic::Ordering::Acquire) {
                self.loaded[role].store(cause.cause, core::sync::atomic::Ordering::Release);
                *slot = Some(cause);
            }
        }
        due
    }
}

/// `[Player, CPU]`: which roles a menu close picked. The Mii list close drops that role's saved-Mii reload: the pick
/// resolves the role's freshly saved choice itself (`editors::mii::MiiList::close_for`). A pick counts whether or not it
/// changed the stored value. `changed`: the role's kept pick key differs from the live settings.
pub fn close_picked(changed: [bool; 2], picks: [Option<u32>; 2]) -> [bool; 2] {
    [changed[0] || picks[0].is_some(), changed[1] || picks[1].is_some()]
}

/// The notice a user's pick shows when the role's own load request is still waiting (review F4: never silence).
pub const FIGHTER_STILL_LOADING: &str = "Fighter still loading - pick it again when it is ready";

/// What became of a role's load request at admission (review F4/R3). Every non-admission outcome a user's request reaches
/// shows its notice at the site that decides it (the roster's `notify`, the Mii owners' existing notices).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// Queued by the roster, or admitted by the role's ordinary Mii owner (which holds it without the roster's pending).
    Accepted,
    /// Queued over an older automatic request (a lobby restore, a partner refresh), or an automatic request coalesced.
    Superseded,
    /// Not now: a load or another practice action is in progress, or an owner refused it as busy.
    Busy,
    /// Not here: the game version, not a practice scene, the cycle off, or no practice entry or captured owner yet.
    Unavailable,
    /// Taken by the saved-Mii owner, which reports its own outcome.
    Owned,
}

/// A load the user chose (`once`) meeting the role's own pending request: `Some(true)` a user's request still waiting
/// (busy: the player is told, never silently replaced), `Some(false)` an automatic one (a restore or a partner refresh:
/// replaced), `None` nothing pending.
pub const fn over_pending(pending_user: Option<bool>) -> Admission {
    match pending_user {
        Some(true) => Admission::Busy,
        Some(false) => Admission::Superseded,
        None => Admission::Accepted,
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn a_player_character_pick_loads_with_player_swapping_off() {
        // Operator 2026-11-13 (TR8): a Player Character pick is one load the user chose, like a combo's Switch. It loads
        // whatever Player Swapping says; that setting governs the cycle chord only.
        for (pressed, changed) in [(true, true), (true, false), (false, true)] {
            let once = menu_request(pressed, changed, false).expect("a pick asks for a load");
            assert!(admitted(false, once, false), "pressed={pressed} changed={changed}: loads with Player Swapping Off");
        }
        assert_eq!(menu_request(false, false, false), None, "an unrelated edit asks for nothing");
        let on = menu_request(false, false, true).expect("turning Player Swapping On loads the pick");
        assert!(admitted(true, on, false));
        assert!(!admitted(false, false, false), "a cycle step stays refused with Player Swapping Off");
        assert!(admitted(false, false, true), "a partner refresh is always taken");
    }

    fn a_record_folds_each_cause_once_and_keeps_the_newest_target() {
        let views = |player: u32, on: bool| [RoleView { selected: player, cycle_on: on }, RoleView { selected: 0x00, cycle_on: true }];
        let record = DispatchRecord::default();
        let boot = record.fold(None, views(0x07, true), [None; 2]);
        assert_eq!(boot.roles, [RoleDispatch::default(); 2], "boot has nothing to compare with");
        let repick = boot.fold(Some(views(0x07, true)), views(0x07, true), [Some(0x07), None]);
        assert_eq!(repick.roles[0], RoleDispatch { target: 0x07, once: true, cause: 2 });
        let unrelated = repick.fold(Some(views(0x07, true)), views(0x07, true), [None; 2]);
        assert_eq!(unrelated.roles[0].cause, 2, "a publication without a cause keeps the role's last cause");
        let on = unrelated.fold(Some(views(0x07, false)), views(0x07, true), [None; 2]);
        assert_eq!(on.roles[0], RoleDispatch { target: 0x07, once: false, cause: 4 }, "Player Swapping turning On");
        assert_eq!(on.roles[1].cause, 0, "the CPU had no cause");
    }

    fn admission_outcomes_are_distinct() {
        assert_eq!(over_pending(None), Admission::Accepted);
        assert_eq!(over_pending(Some(false)), Admission::Superseded);
        assert_eq!(over_pending(Some(true)), Admission::Busy, "a waiting user load is busy: its site notifies");
        assert!(!FIGHTER_STILL_LOADING.is_empty());
    }

    /// This module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_player_character_pick_loads_with_player_swapping_off", a_player_character_pick_loads_with_player_swapping_off as fn()),
            ("a_record_folds_each_cause_once_and_keeps_the_newest_target", a_record_folds_each_cause_once_and_keeps_the_newest_target as fn()),
            ("admission_outcomes_are_distinct", admission_outcomes_are_distinct as fn()),
        ]);
    }
    }
}

