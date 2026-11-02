//! The typed menu view the carried fork code reads: `OnOff` and a `TrainingModpackMenu` with the fields the carried code uses.
//!
//! The fork keeps one big struct of every setting behind a global (`MENU`). The clean room keeps settings as rows by id
//! (`model::settings::Snapshot`); this view is built from a snapshot (and the kept-as-they-are rows) for the carried engines
//! and written back through [`TrainingModpackMenu::write_rows`] (decision D1: package 5's commit path persists the rows).
//! Fields are added here when a carried file needs them; each is the fork's field name, i.e. the setting id. Written for this
//! repository (not carried). The choice types are the carried files' own `byteflags!` groups (`CyclePreloadSlots`, ...); `OnOff`
//! is the fork's on/off pair in the fork's order, `[On, Off]` (the factory table's order too, `model::factory`).
//!
//! The serde attributes are the ones the fork's struct carries on the same fields: a menu file written by the fork (or an older
//! one) is read key by key, a missing key takes the built-in value and a malformed choice list the carried deserializer's value.
//! The carried tests exercise exactly that through [`super::menu_json::parse_menu_file`].

use serde::{Deserialize, Serialize};

use super::choice::byteflags;

use crate::engine::preload::cycle_preload_config as preload;
use crate::model::settings::Snapshot;

byteflags! {
    /// A two-state setting as the carried code names it: `OnOff::ON` / `OnOff::OFF`, option order `[On, Off]`.
    pub struct OnOff {
        pub ON = "On",
        pub OFF = "Off",
    }
}

/// One setting row's elements as a typed choice: element `i` is the weight of option `i`. Missing elements count as 0 (a
/// shorter array leaves the tail off, `settings::parse_row`).
pub fn from_row<T>(row: &[u8], all: &[T]) -> T
where
    T: Copy + std::ops::Add<Output = T> + std::ops::Mul<u8, Output = T>,
{
    let mut weighted = all.iter().enumerate().map(|(index, option)| *option * row.get(index).copied().unwrap_or(0));
    let first = weighted.next().expect("a choice group has choices");
    weighted.fold(first, |sum, next| sum + next)
}

fn row_or<T>(snapshot: &Snapshot, id: &str, all: &[T], fallback: T) -> T
where
    T: Copy + std::ops::Add<Output = T> + std::ops::Mul<u8, Output = T>,
{
    snapshot.get(id).map_or(fallback, |row| from_row(row, all))
}

const fn solo_popo_player_cycle_default() -> OnOff {
    OnOff::OFF
}
const fn solo_popo_cpu_default() -> OnOff {
    OnOff::OFF
}
const fn solo_popo_original_default() -> OnOff {
    OnOff::ON
}

/// The settings the carried engines read (grows with each carried layer).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainingModpackMenu {
    /// Ice Climbers' Solo Popo preference for Player slot 0 (`solo_popo_original`; factory default On).
    #[serde(default = "solo_popo_original_default")]
    pub solo_popo_original: OnOff,
    /// Solo Popo for the Player's numbered cycle slots (`solo_popo_player_cycle`).
    #[serde(default = "solo_popo_player_cycle_default")]
    pub solo_popo_player_cycle: OnOff,
    /// Solo Popo for the CPU (`solo_popo_cpu`).
    #[serde(default = "solo_popo_cpu_default")]
    pub solo_popo_cpu: OnOff,
    /// Cycle Preload: Off, 1..=5 (`cycle_preload_slots`).
    #[serde(default = "preload::default_slots", deserialize_with = "preload::deserialize_slots")]
    pub cycle_preload_slots: preload::CyclePreloadSlots,
    /// Cycle Preload sources: Fighters, Hats, Both.
    #[serde(default = "preload::default_sources")]
    pub cycle_preload_sources: preload::CyclePreloadSources,
    /// Preload Samples (the Developer page row, N1).
    #[serde(default = "preload::default_measurements")]
    pub cycle_preload_measurements: OnOff,
    /// Ready Fighter Bodies: Off, 2, 3.
    #[serde(default = "preload::default_bank_members", deserialize_with = "preload::deserialize_bank_members")]
    pub cycle_preload_bank_members: preload::CyclePreloadBankMembers,
    /// Group Swap (the master switch of every detached group).
    #[serde(default = "preload::default_group_swap")]
    pub cycle_preload_group_swap: OnOff,
    /// Ready Fighters: 2..=12.
    #[serde(default = "preload::default_ready_fighters", deserialize_with = "preload::deserialize_ready_fighters")]
    pub cycle_preload_ready_fighters: preload::CyclePreloadReadyFighters,
    /// Ice Climbers Pair Bank.
    #[serde(default = "preload::default_ic_pair_bank")]
    pub cycle_preload_ic_pair_bank: OnOff,
    /// Original Fighter Bank.
    #[serde(default = "preload::default_original_bank")]
    pub cycle_preload_original_bank: OnOff,
    /// Solo Popo Groups.
    #[serde(default = "preload::default_solo_popo_groups")]
    pub cycle_preload_solo_popo_groups: OnOff,
    /// Ice Climbers Pair Groups.
    #[serde(default = "preload::default_ic_pair_groups")]
    pub cycle_preload_ic_pair_groups: OnOff,
    /// PT/Aegis Groups.
    #[serde(default = "preload::default_native_groups")]
    pub cycle_preload_native_groups: OnOff,
    /// Ice Climbers Pair Group.
    #[serde(default = "preload::default_pair_groups")]
    pub cycle_preload_pair_groups: OnOff,
    /// Mii Groups.
    #[serde(default = "preload::default_mii_groups")]
    pub cycle_preload_mii_groups: OnOff,
}

/// The factory values (`BASE_MENU` in the fork).
pub const BASE_MENU: TrainingModpackMenu = TrainingModpackMenu {
    solo_popo_original: OnOff::ON,
    solo_popo_player_cycle: OnOff::OFF,
    solo_popo_cpu: OnOff::OFF,
    cycle_preload_slots: preload::default_slots(),
    cycle_preload_sources: preload::CyclePreloadSources::BOTH,
    cycle_preload_measurements: OnOff::ON,
    cycle_preload_bank_members: preload::default_bank_members(),
    cycle_preload_group_swap: OnOff::ON,
    cycle_preload_ready_fighters: preload::default_ready_fighters(),
    cycle_preload_ic_pair_bank: OnOff::ON,
    cycle_preload_original_bank: OnOff::ON,
    cycle_preload_solo_popo_groups: OnOff::ON,
    cycle_preload_ic_pair_groups: OnOff::ON,
    cycle_preload_native_groups: OnOff::ON,
    cycle_preload_pair_groups: OnOff::ON,
    cycle_preload_mii_groups: OnOff::ON,
};

impl TrainingModpackMenu {
    /// The view of one snapshot (`menu` or `defaults_menu` of the settings file). A row the snapshot does not know (the Solo
    /// Popo cycle rows are kept as they are until the swap package gives them a row) takes the factory value.
    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        let on_off = |id: &str, fallback: OnOff| row_or(snapshot, id, &OnOff::ALL_CONSTS, fallback);
        Self {
            solo_popo_original: on_off("solo_popo_original", BASE_MENU.solo_popo_original),
            solo_popo_player_cycle: on_off("solo_popo_player_cycle", BASE_MENU.solo_popo_player_cycle),
            solo_popo_cpu: on_off("solo_popo_cpu", BASE_MENU.solo_popo_cpu),
            cycle_preload_slots: row_or(
                snapshot,
                "cycle_preload_slots",
                &preload::CyclePreloadSlots::ALL_CONSTS,
                BASE_MENU.cycle_preload_slots,
            ),
            cycle_preload_sources: row_or(
                snapshot,
                "cycle_preload_sources",
                &preload::CyclePreloadSources::ALL_CONSTS,
                BASE_MENU.cycle_preload_sources,
            ),
            cycle_preload_measurements: on_off("cycle_preload_measurements", BASE_MENU.cycle_preload_measurements),
            cycle_preload_bank_members: row_or(
                snapshot,
                "cycle_preload_bank_members",
                &preload::CyclePreloadBankMembers::ALL_CONSTS,
                BASE_MENU.cycle_preload_bank_members,
            ),
            cycle_preload_group_swap: on_off("cycle_preload_group_swap", BASE_MENU.cycle_preload_group_swap),
            cycle_preload_ready_fighters: row_or(
                snapshot,
                "cycle_preload_ready_fighters",
                &preload::CyclePreloadReadyFighters::ALL_CONSTS,
                BASE_MENU.cycle_preload_ready_fighters,
            ),
            cycle_preload_ic_pair_bank: on_off("cycle_preload_ic_pair_bank", BASE_MENU.cycle_preload_ic_pair_bank),
            cycle_preload_original_bank: on_off("cycle_preload_original_bank", BASE_MENU.cycle_preload_original_bank),
            cycle_preload_solo_popo_groups: on_off("cycle_preload_solo_popo_groups", BASE_MENU.cycle_preload_solo_popo_groups),
            cycle_preload_ic_pair_groups: on_off("cycle_preload_ic_pair_groups", BASE_MENU.cycle_preload_ic_pair_groups),
            cycle_preload_native_groups: on_off("cycle_preload_native_groups", BASE_MENU.cycle_preload_native_groups),
            cycle_preload_pair_groups: on_off("cycle_preload_pair_groups", BASE_MENU.cycle_preload_pair_groups),
            cycle_preload_mii_groups: on_off("cycle_preload_mii_groups", BASE_MENU.cycle_preload_mii_groups),
        }
    }

    /// Every row this view holds as (setting id, elements), in the order of the fields: the write-back of D1 (package 5's commit
    /// path persists them; `compat::menu::write_back(row_id, value)` is one of these pairs).
    pub fn rows(&self) -> [(&'static str, Vec<u8>); 16] {
        [
            ("solo_popo_original", self.solo_popo_original.to_byte_vec()),
            ("solo_popo_player_cycle", self.solo_popo_player_cycle.to_byte_vec()),
            ("solo_popo_cpu", self.solo_popo_cpu.to_byte_vec()),
            ("cycle_preload_slots", self.cycle_preload_slots.to_byte_vec()),
            ("cycle_preload_sources", self.cycle_preload_sources.to_byte_vec()),
            ("cycle_preload_measurements", self.cycle_preload_measurements.to_byte_vec()),
            ("cycle_preload_bank_members", self.cycle_preload_bank_members.to_byte_vec()),
            ("cycle_preload_group_swap", self.cycle_preload_group_swap.to_byte_vec()),
            ("cycle_preload_ready_fighters", self.cycle_preload_ready_fighters.to_byte_vec()),
            ("cycle_preload_ic_pair_bank", self.cycle_preload_ic_pair_bank.to_byte_vec()),
            ("cycle_preload_original_bank", self.cycle_preload_original_bank.to_byte_vec()),
            ("cycle_preload_solo_popo_groups", self.cycle_preload_solo_popo_groups.to_byte_vec()),
            ("cycle_preload_ic_pair_groups", self.cycle_preload_ic_pair_groups.to_byte_vec()),
            ("cycle_preload_native_groups", self.cycle_preload_native_groups.to_byte_vec()),
            ("cycle_preload_pair_groups", self.cycle_preload_pair_groups.to_byte_vec()),
            ("cycle_preload_mii_groups", self.cycle_preload_mii_groups.to_byte_vec()),
        ]
    }
}

pub use super::grid::NX_SUBMENU_COLUMNS;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::settings::DEFS;

    #[test]
    fn on_off_is_on_then_off_like_the_factory_table() {
        assert_eq!(OnOff::ALL_NAMES, ["On", "Off"]);
        assert_eq!(OnOff::ON.to_byte_vec(), [1, 0]);
        assert_eq!(OnOff::OFF.to_byte_vec(), [0, 1]);
        assert_eq!(OnOff::ON + OnOff::OFF, OnOff::all());
    }

    /// The two rows whose fork default differs from the clean-room factory table: the fork starts Solo Popo (original) On and
    /// Preload Samples On (`Settings::DEFAULT.measurements`); MENU-DRAFT and `model::factory` say Off for both. The runtime view
    /// is `from_snapshot`, so the settings file wins; `BASE_MENU` stays the fork's value because carried tests pin it.
    const KNOWN_DIVERGENCES: [&str; 2] = ["solo_popo_original", "cycle_preload_measurements"];

    #[test]
    fn the_fork_menu_is_the_factory_snapshot_row_for_row_except_the_known_divergences() {
        let factory = TrainingModpackMenu::from_snapshot(&Snapshot::factory());
        let mut known = 0;
        for (id, elements) in BASE_MENU.rows() {
            let Some((_, def)) = DEFS.iter().enumerate().find(|(_, def)| def.id == id) else { continue };
            assert_eq!(elements.len(), def.len, "{id}: option count");
            if KNOWN_DIVERGENCES.contains(&id) {
                assert_ne!(elements, def.default, "{id}: no longer diverges, drop it from KNOWN_DIVERGENCES");
                known += 1;
            } else {
                assert_eq!(elements, def.default, "{id}: factory value");
            }
        }
        assert_eq!(known, KNOWN_DIVERGENCES.len());
        assert_eq!(factory.solo_popo_original, OnOff::OFF);
        assert_eq!(factory.cycle_preload_measurements, OnOff::OFF);
    }
    #[test]
    fn a_snapshot_round_trips_through_the_view() {
        let mut snapshot = Snapshot::factory();
        snapshot.set("cycle_preload_slots", &[0, 1, 0, 0, 0, 0]).unwrap();
        snapshot.set("cycle_preload_group_swap", &[0, 1]).unwrap();
        snapshot.set("cycle_preload_ready_fighters", &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]).unwrap();
        let view = TrainingModpackMenu::from_snapshot(&snapshot);
        assert_eq!(view.cycle_preload_slots, preload::CyclePreloadSlots::ONE);
        assert_eq!(view.cycle_preload_group_swap, OnOff::OFF);
        assert_eq!(view.cycle_preload_ready_fighters, preload::CyclePreloadReadyFighters::TWELVE);
        let mut back = Snapshot::factory();
        for (id, elements) in view.rows() {
            if back.get(id).is_some() {
                back.set(id, &elements).unwrap();
            }
        }
        assert_eq!(back, snapshot);
    }

    #[test]
    fn a_short_row_leaves_the_tail_off() {
        assert_eq!(from_row(&[1], &OnOff::ALL_CONSTS), OnOff::ON);
        assert_eq!(from_row(&[], &OnOff::ALL_CONSTS), OnOff::empty());
    }
}
