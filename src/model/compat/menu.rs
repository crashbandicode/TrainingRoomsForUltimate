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
use crate::engine::preload::hat_slots as hats;
use crate::engine::stages::lobby_stages::LobbyStageOrder;
use crate::engine::swap::ordered_roster::{OrderedFighterRoster, DEFAULT_KIRBY_HATS};
use crate::engine::swap::roster_grid::Role as RosterRole;
use crate::engine::swap::trio::TrioFighter;
use crate::model::settings::Snapshot;

byteflags! {
    /// A two-state setting as the carried code names it: `OnOff::ON` / `OnOff::OFF`, option order `[On, Off]`.
    pub struct OnOff {
        pub ON = "On",
        pub OFF = "Off",
    }
}

byteflags! {
    /// Where the CPU's recordings come from (macro_source): the numbered slots or the named library. Option order as the row.
    pub struct MacroSource {
        pub NUMBERED = "Numbered Slots",
        pub NAMED = "Named Library",
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
    if let Some(row) = snapshot.get(id) {
        return from_row(row, all);
    }
    // A row the settings table does not define (the Solo Popo cycle rows, which the Character Cycles grid sets with Y): its kept key,
    // which the write-back stores in the row's own shape (`rows()`: the option bytes). Without this the typed view always read the
    // factory value, so Solo Popo on a cycle slot could never be on.
    let kept = snapshot.extra().get(id).and_then(|value| value.as_array()).map(|values| {
        values.iter().map(|v| v.as_u64().and_then(|n| u8::try_from(n).ok())).collect::<Option<Vec<u8>>>()
    });
    match kept {
        Some(Some(row)) if row.len() == all.len() => from_row(&row, all),
        _ => fallback,
    }
}

const fn solo_popo_player_cycle_default() -> OnOff {
    OnOff::OFF
}
const fn solo_popo_cpu_default() -> OnOff {
    OnOff::OFF
}
const fn solo_popo_original_default() -> OnOff {
    OnOff::OFF
}
const fn fighter_prompt_default() -> OnOff {
    OnOff::ON
}
const fn macro_source_default() -> MacroSource {
    MacroSource::NUMBERED
}
const fn lobby_practice_default() -> OnOff {
    OnOff::OFF
}
const fn training_stage_rotation_default() -> OnOff {
    OnOff::OFF
}
const fn trio_practice_default() -> OnOff {
    OnOff::OFF
}
const fn character_default() -> TrioFighter {
    TrioFighter::MARIO
}
/// A missing `player_roster` key is All. That is the portrait grid's factory (`RosterEditor::values` writes a leading
/// 1), so the typed view and the grid read one roster. `DEFAULT_PLAYER_ROSTER` is the named explicit list, not this
/// fallback (P1-CYCLE).
const fn player_roster_default() -> OrderedFighterRoster {
    OrderedFighterRoster::all()
}
/// A missing `cpu_roster` key is All, the same factory as `player_roster_default`.
const fn cpu_roster_default() -> OrderedFighterRoster {
    OrderedFighterRoster::all()
}
const fn kirby_hats_default() -> OrderedFighterRoster {
    DEFAULT_KIRBY_HATS
}

/// A top-level kept key of the settings file as a typed value; a missing or malformed one is the type's default (the fork's rule for
/// `stage_preferences` and `mii_preferences`: a bad stored preference never blocks the menu).
pub fn kept_or_default<T: serde::de::DeserializeOwned + Default>(extra: &serde_json::Map<String, serde_json::Value>, key: &str) -> T {
    extra.get(key).and_then(|value| serde_json::from_value(value.clone()).ok()).unwrap_or_default()
}

/// The stored `mii_preferences`, one selector at a time (the fork's `deserialize_mii_preferences`,
/// `training_mod_consts/src/menu_json.rs` 17-37): each valid selector is kept, unknown fields are ignored, and only an
/// invalid selector is left unset, so one bad entry never resets the other five.
pub fn mii_preferences_of(value: Option<&serde_json::Value>) -> crate::engine::mii::selection::Preferences {
    use crate::engine::mii::{inventory::Canonical, selection::{Preferences, Role, CLASSES}};
    let mut preferences = Preferences::default();
    let Some(value) = value else { return preferences };
    for (role, key) in [(Role::Player, "player"), (Role::Cpu, "cpu")] {
        if let Some(selectors) = value.get(key).and_then(serde_json::Value::as_array) {
            for (class, selector) in CLASSES.iter().zip(selectors) {
                if let Ok(identity) = serde_json::from_value::<Canonical>(selector.clone()) {
                    if let Ok(updated) = preferences.with(role, *class, Some(identity)) {
                        preferences = updated;
                    }
                }
            }
        }
    }
    preferences
}

/// A kept key of the snapshot (a row whose value is not a plain array) as a typed value; a missing or malformed one is `fallback`.
fn kept_or<T: serde::de::DeserializeOwned>(snapshot: &Snapshot, id: &str, fallback: T) -> T {
    snapshot.extra().get(id).and_then(|value| serde_json::from_value(value.clone()).ok()).unwrap_or(fallback)
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
    /// Ready Hats: 0..=5, the hat budget apart from Ready Fighters (P-HOT).
    #[serde(default = "hats::default_ready_hats", deserialize_with = "hats::deserialize_ready_hats")]
    pub cycle_preload_ready_hats: hats::CyclePreloadReadyHats,
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
    /// Ask before a replay or a named recording switches the CPU's fighter (`fighter_prompt`, package 4).
    #[serde(default = "fighter_prompt_default")]
    pub fighter_prompt: OnOff,
    /// Where the CPU's recordings come from (macro_source, package 4).
    #[serde(default = "macro_source_default")]
    pub macro_source: MacroSource,
    /// Lobby Practice (the waiting room runs the practice session).
    #[serde(default = "lobby_practice_default")]
    pub lobby_practice: OnOff,
    /// Training Stage Rotation (ordinary Training starts on the next stage of the Lobby Stages order).
    #[serde(default = "training_stage_rotation_default")]
    pub training_stage_rotation: OnOff,
    /// The ordered stage list (`lobby_stages`: a kept key of the settings file, not a setting row).
    #[serde(default)]
    pub lobby_stages: LobbyStageOrder,
    /// Player Swapping (`trio_practice`): the Player 1 cycle chord changes the fighter.
    #[serde(default = "trio_practice_default")]
    pub trio_practice: OnOff,
    /// The fighter picked for Player 1 / the CPU (`player_character`, `cpu_character`: kept keys, one-hot over the roster catalog).
    #[serde(default = "character_default")]
    pub player_character: TrioFighter,
    #[serde(default = "character_default")]
    pub cpu_character: TrioFighter,
    /// The cycle orders (`player_roster`, `cpu_roster`: kept keys, `[all, ranks...]`).
    #[serde(default = "player_roster_default")]
    pub player_roster: OrderedFighterRoster,
    #[serde(default = "cpu_roster_default")]
    pub cpu_roster: OrderedFighterRoster,
    /// The Kirby hat orders (`player_kirby_hats`, `cpu_kirby_hats`: kept keys).
    #[serde(default = "kirby_hats_default")]
    pub player_kirby_hats: OrderedFighterRoster,
    #[serde(default = "kirby_hats_default")]
    pub cpu_kirby_hats: OrderedFighterRoster,
}

/// The factory values (`BASE_MENU` in the fork).
pub const BASE_MENU: TrainingModpackMenu = TrainingModpackMenu {
    solo_popo_original: OnOff::OFF,
    solo_popo_player_cycle: OnOff::OFF,
    solo_popo_cpu: OnOff::OFF,
    cycle_preload_slots: preload::default_slots(),
    cycle_preload_sources: preload::CyclePreloadSources::BOTH,
    cycle_preload_measurements: OnOff::OFF,
    cycle_preload_bank_members: preload::default_bank_members(),
    cycle_preload_group_swap: OnOff::ON,
    cycle_preload_ready_fighters: preload::default_ready_fighters(),
    cycle_preload_ready_hats: hats::default_ready_hats(),
    cycle_preload_ic_pair_bank: OnOff::ON,
    cycle_preload_original_bank: OnOff::ON,
    cycle_preload_solo_popo_groups: OnOff::ON,
    cycle_preload_ic_pair_groups: OnOff::ON,
    cycle_preload_native_groups: OnOff::ON,
    cycle_preload_pair_groups: OnOff::ON,
    cycle_preload_mii_groups: OnOff::ON,
    fighter_prompt: OnOff::ON,
    macro_source: MacroSource::NUMBERED,
    lobby_practice: OnOff::OFF,
    training_stage_rotation: OnOff::OFF,
    lobby_stages: LobbyStageOrder::NATIVE,
    trio_practice: OnOff::OFF,
    player_character: TrioFighter::MARIO,
    cpu_character: TrioFighter::MARIO,
    player_roster: OrderedFighterRoster::all(),
    cpu_roster: OrderedFighterRoster::all(),
    player_kirby_hats: DEFAULT_KIRBY_HATS,
    cpu_kirby_hats: DEFAULT_KIRBY_HATS,
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
            cycle_preload_ready_hats: row_or(
                snapshot,
                "cycle_preload_ready_hats",
                &hats::CyclePreloadReadyHats::ALL_CONSTS,
                BASE_MENU.cycle_preload_ready_hats,
            ),
            cycle_preload_ic_pair_bank: on_off("cycle_preload_ic_pair_bank", BASE_MENU.cycle_preload_ic_pair_bank),
            cycle_preload_original_bank: on_off("cycle_preload_original_bank", BASE_MENU.cycle_preload_original_bank),
            cycle_preload_solo_popo_groups: on_off("cycle_preload_solo_popo_groups", BASE_MENU.cycle_preload_solo_popo_groups),
            cycle_preload_ic_pair_groups: on_off("cycle_preload_ic_pair_groups", BASE_MENU.cycle_preload_ic_pair_groups),
            cycle_preload_native_groups: on_off("cycle_preload_native_groups", BASE_MENU.cycle_preload_native_groups),
            cycle_preload_pair_groups: on_off("cycle_preload_pair_groups", BASE_MENU.cycle_preload_pair_groups),
            cycle_preload_mii_groups: on_off("cycle_preload_mii_groups", BASE_MENU.cycle_preload_mii_groups),
            fighter_prompt: on_off("fighter_prompt", BASE_MENU.fighter_prompt),
            macro_source: row_or(snapshot, "macro_source", &MacroSource::ALL_CONSTS, BASE_MENU.macro_source),
            lobby_practice: on_off("lobby_practice", BASE_MENU.lobby_practice),
            training_stage_rotation: on_off("training_stage_rotation", BASE_MENU.training_stage_rotation),
            // a kept key: a missing or malformed list is the native order
            lobby_stages: snapshot
                .extra()
                .get("lobby_stages")
                .and_then(|value| serde_json::from_value(value.clone()).ok())
                .unwrap_or(BASE_MENU.lobby_stages),
            trio_practice: on_off("trio_practice", BASE_MENU.trio_practice),
            player_character: kept_or(snapshot, "player_character", BASE_MENU.player_character),
            cpu_character: kept_or(snapshot, "cpu_character", BASE_MENU.cpu_character),
            player_roster: kept_or(snapshot, "player_roster", BASE_MENU.player_roster),
            cpu_roster: kept_or(snapshot, "cpu_roster", BASE_MENU.cpu_roster),
            player_kirby_hats: kept_or(snapshot, "player_kirby_hats", BASE_MENU.player_kirby_hats),
            cpu_kirby_hats: kept_or(snapshot, "cpu_kirby_hats", BASE_MENU.cpu_kirby_hats),
        }
    }

    /// The Kirby hat order of `role` as fighter kinds (Kirby removed, the Mii classes only with the helper build).
    pub fn ordered_kirby_hat_kinds(&self, role: RosterRole) -> Vec<u32> {
        self.ordered_kirby_hat_kinds_fixed(role).to_vec()
    }

    /// The same hat order without heap: the preload planner reads it on every planning tick inside the fighter-frame hook (R5, F10).
    pub fn ordered_kirby_hat_kinds_fixed(&self, role: RosterRole) -> crate::engine::swap::ordered_roster::KindList {
        match role {
            RosterRole::Player => self.player_kirby_hats.hat_kinds_fixed(),
            RosterRole::Cpu => self.cpu_kirby_hats.hat_kinds_fixed(),
        }
    }

    /// The kept keys of this view as JSON values (the settings file's non-array rows), in field order: the write-back of D1.
    pub fn kept_values(&self) -> Vec<(&'static str, serde_json::Value)> {
        fn value<T: Serialize>(typed: &T) -> serde_json::Value {
            serde_json::to_value(typed).unwrap_or(serde_json::Value::Null)
        }
        vec![
            ("lobby_stages", value(&self.lobby_stages)),
            ("player_character", value(&self.player_character)),
            ("cpu_character", value(&self.cpu_character)),
            ("player_roster", value(&self.player_roster)),
            ("cpu_roster", value(&self.cpu_roster)),
            ("player_kirby_hats", value(&self.player_kirby_hats)),
            ("cpu_kirby_hats", value(&self.cpu_kirby_hats)),
        ]
    }

    /// Every row this view holds as (setting id, elements), in the order of the fields: the write-back of D1 (package 5's commit
    /// path persists them; `compat::menu::write_back(row_id, value)` is one of these pairs).
    pub fn rows(&self) -> [(&'static str, Vec<u8>); 22] {
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
            ("cycle_preload_ready_hats", self.cycle_preload_ready_hats.to_byte_vec()),
            ("cycle_preload_ic_pair_bank", self.cycle_preload_ic_pair_bank.to_byte_vec()),
            ("cycle_preload_original_bank", self.cycle_preload_original_bank.to_byte_vec()),
            ("cycle_preload_solo_popo_groups", self.cycle_preload_solo_popo_groups.to_byte_vec()),
            ("cycle_preload_ic_pair_groups", self.cycle_preload_ic_pair_groups.to_byte_vec()),
            ("cycle_preload_native_groups", self.cycle_preload_native_groups.to_byte_vec()),
            ("cycle_preload_pair_groups", self.cycle_preload_pair_groups.to_byte_vec()),
            ("cycle_preload_mii_groups", self.cycle_preload_mii_groups.to_byte_vec()),
            ("fighter_prompt", self.fighter_prompt.to_byte_vec()),
            ("macro_source", self.macro_source.to_byte_vec()),
            ("lobby_practice", self.lobby_practice.to_byte_vec()),
            ("training_stage_rotation", self.training_stage_rotation.to_byte_vec()),
            ("trio_practice", self.trio_practice.to_byte_vec()),
        ]
    }
}

pub use super::grid::NX_SUBMENU_COLUMNS;

