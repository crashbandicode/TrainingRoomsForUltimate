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
// or the stored pick a cycle left behind) changed no setting, so nothing dispatched it. Now the close carries each role's
// pick through the settings publish to the game frame, which makes ONE decision per role from the settings change, Player
// Swapping turning On and the pick together (`role_dispatch`).
//
// Contract: the LAST pick per role per menu visit wins. Earlier presses of the same role in the same visit are
// superseded (the menu's outbox keeps one value per role, `model::menu::Outbox::picks`); there is no per-press stream.
// ---------------------------------------------------------------------------------------------------------------------

/// One role's portrait pick on its way to the game frame: the fighter, and the live settings generation of the publish
/// that carried it. That publish is live before the pick is posted, so the generation is at least 1 and a posted word is
/// never 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pick {
    pub kind: u32,
    pub generation: u32,
}

impl Pick {
    /// The handoff word (`PickSlots`); 0 is empty.
    pub const fn encode(self) -> u64 {
        ((self.generation as u64) << 32) | self.kind as u64
    }
    pub const fn decode(word: u64) -> Option<Self> {
        if word == 0 { None } else { Some(Self { kind: word as u32, generation: (word >> 32) as u32 }) }
    }
}

/// The publish → game frame handoff of the two roles' picks: one word per role, written by the publishing path under
/// `published::PUBLISHING` (a later pick of the same role replaces one not taken yet: the last pick wins) and taken by
/// the game frame (`compat::menu::service_roster_change`) with one swap per role. Never waits, no heap.
pub struct PickSlots([core::sync::atomic::AtomicU64; 2]);

impl Default for PickSlots {
    fn default() -> Self {
        Self::new()
    }
}

impl PickSlots {
    pub const fn new() -> Self {
        Self([core::sync::atomic::AtomicU64::new(0), core::sync::atomic::AtomicU64::new(0)])
    }
    /// The publishing path, after the publish went live at `generation` (and after it noted any roster re-plan).
    pub fn post(&self, picks: [Option<u32>; 2], generation: u32) {
        for (slot, pick) in self.0.iter().zip(picks) {
            if let Some(kind) = pick {
                slot.store(Pick { kind, generation }.encode(), core::sync::atomic::Ordering::Release);
            }
        }
    }
    /// The game frame: the picks posted since the last take, `[Player, CPU]`.
    pub fn take(&self) -> [Option<Pick>; 2] {
        [0, 1].map(|role| Pick::decode(self.0[role].swap(0, core::sync::atomic::Ordering::AcqRel)))
    }
}

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

/// The one per-role dispatch decision of a game frame (review F2/F3). A selection the settings change changed, Player
/// Swapping just turning On (Player 1) and an explicit pick the frame took are decided together, so a role loads at most
/// once for them.
/// - `previous`: the role before the earliest publish the game frame had not applied (None: no settings change this frame).
/// - `current`: the role in the committed, live settings.
/// - `pick`: the role's pick this frame took. It counts only while it still names the committed selection (a later
///   selection supersedes it and loads through its own change) and while its publish is newer than `covered`.
/// - `covered`: the live generation the role's last load request compared against. The pick of a publish that request
///   already covered is not asked for twice, whichever frame the change and the pick reached.
///
/// `Some(once)`: queue one load of `current.selected` (`once`: a load the user chose, which Player Swapping Off does not
/// refuse).
pub fn role_dispatch(player: bool, previous: Option<RoleView>, current: RoleView, pick: Option<Pick>, covered: u32) -> Option<bool> {
    let changed = previous.is_some_and(|previous| previous.selected != current.selected);
    let cycle_turned_on = player && previous.is_some_and(|previous| !previous.cycle_on) && current.cycle_on;
    let pressed = pick.is_some_and(|pick| pick.generation > covered && pick.kind == current.selected);
    menu_request(pressed, changed, cycle_turned_on)
}

/// The game frame's per-role record of what its last load request covered (`role_dispatch`'s `covered`). Game thread only.
pub struct PickCoverage([core::sync::atomic::AtomicU32; 2]);

impl Default for PickCoverage {
    fn default() -> Self {
        Self::new()
    }
}

impl PickCoverage {
    pub const fn new() -> Self {
        Self([core::sync::atomic::AtomicU32::new(0), core::sync::atomic::AtomicU32::new(0)])
    }
    /// `role_dispatch` for `role` (0 Player, 1 CPU), recording `live_generation` when it asks for a load.
    pub fn decide(&self, role: usize, previous: Option<RoleView>, current: RoleView, pick: Option<Pick>, live_generation: u32) -> Option<bool> {
        let covered = self.0[role].load(core::sync::atomic::Ordering::Acquire);
        let once = role_dispatch(role == 0, previous, current, pick, covered);
        if once.is_some() {
            self.0[role].store(live_generation.max(covered), core::sync::atomic::Ordering::Release);
        }
        once
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

/// What became of a role's load request at admission (review F4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// Queued, or taken by the role's Mii owner (which may hold it without the roster's pending request).
    Accepted,
    /// Queued over an older automatic request (a lobby restore, a partner refresh), which it replaces.
    Superseded,
    /// Not now: a load or another practice action is in progress. The player is told.
    Busy(&'static str),
    /// Not here: unavailable on this game version, or no practice entry yet. The player is told.
    Unavailable(&'static str),
}

impl Admission {
    /// The notice the player sees for this outcome, if any.
    pub const fn notice(self) -> Option<&'static str> {
        match self {
            Self::Busy(text) | Self::Unavailable(text) => Some(text),
            Self::Accepted | Self::Superseded => None,
        }
    }
}

/// A load the user chose (`once`) meeting the role's own pending request: `Some(true)` a user's request still waiting
/// (busy: the player is told, never silently replaced), `Some(false)` an automatic one (a restore or a partner refresh:
/// replaced), `None` nothing pending.
pub const fn over_pending(pending_user: Option<bool>) -> Admission {
    match pending_user {
        Some(true) => Admission::Busy(FIGHTER_STILL_LOADING),
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

    fn a_pick_word_round_trips_and_the_slots_keep_the_last_pick() {
        let pick = Pick { kind: 0x07, generation: 3 };
        assert_eq!(Pick::decode(pick.encode()), Some(pick));
        assert_eq!(Pick::decode(0), None);
        let slots = PickSlots::new();
        slots.post([Some(0x07), None], 3);
        slots.post([Some(0x58), Some(0x00)], 4);
        assert_eq!(slots.take(), [Some(Pick { kind: 0x58, generation: 4 }), Some(Pick { kind: 0x00, generation: 4 })]);
        assert_eq!(slots.take(), [None, None], "taken once");
    }

    fn admission_outcomes_always_tell_the_player_when_not_accepted() {
        assert_eq!(over_pending(None), Admission::Accepted);
        assert_eq!(over_pending(Some(false)), Admission::Superseded);
        assert_eq!(over_pending(Some(true)).notice(), Some(FIGHTER_STILL_LOADING), "a waiting user load: a notice, never silence");
        assert_eq!(Admission::Unavailable("x").notice(), Some("x"));
        assert_eq!(Admission::Accepted.notice(), None);
    }

    /// This module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_player_character_pick_loads_with_player_swapping_off", a_player_character_pick_loads_with_player_swapping_off as fn()),
            ("a_pick_word_round_trips_and_the_slots_keep_the_last_pick", a_pick_word_round_trips_and_the_slots_keep_the_last_pick as fn()),
            ("admission_outcomes_always_tell_the_player_when_not_accepted", admission_outcomes_always_tell_the_player_when_not_accepted as fn()),
        ]);
    }
    }
}

