//! Ordinary Grab selection. Fighter identity comes from Training Rooms' serialized roster table;
//! native initialized clatter values and move-entry success are observed by the runtime.

use crate::model::{settings::Snapshot, swap::trio::CHOICES};

pub const ROW: &str = "mashout";
pub const STATE_ROW: &str = "mashout_state";
pub const ATTACKER_ROW: &str = "mashout_attacker";
pub const GRAB_ROW: &str = "mashout_grab";

/// All 89 ordinary fighter families, including starting forms and the three Mii classes.
/// The Ice Climbers entry is the native entry descriptor; Nana is not an extra roster choice.
pub const ATTACKERS: &[(u32, &str)] = CHOICES;
pub const RANDOM_ATTACKER: usize = ATTACKERS.len();

const fn attacker_labels() -> [&'static str; RANDOM_ATTACKER + 1] {
    let mut labels = ["Random"; RANDOM_ATTACKER + 1];
    let mut index = 0;
    while index < ATTACKERS.len() {
        labels[index] = ATTACKERS[index].1;
        index += 1;
    }
    labels
}

/// Reuse the roster's labels without maintaining another 89-name menu list.
pub const ATTACKER_LABELS: [&str; RANDOM_ATTACKER + 1] = attacker_labels();
pub const STATE_LABELS: [&str; 1] = ["Grab"];
pub const GRAB_LABELS: [&str; 4] = ["Standing", "Dash", "Pivot", "Random"];
pub const ON_OFF_LABELS: [&str; 2] = ["Off", "On"];

/// Stable state index 0. Further accepted states append after Grab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Grab,
}

impl State {
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Grab),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grab {
    Standing,
    Dash,
    Pivot,
}

impl Grab {
    pub const ALL: [Self; 3] = [Self::Standing, Self::Dash, Self::Pivot];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attacker {
    Fighter(u32),
    Random,
}

impl Attacker {
    pub fn from_index(index: usize) -> Option<Self> {
        if index == RANDOM_ATTACKER {
            Some(Self::Random)
        } else {
            ATTACKERS.get(index).map(|&(kind, _)| Self::Fighter(kind))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrabChoice {
    Fixed(Grab),
    Random,
}

impl GrabChoice {
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0..=2 => Some(Self::Fixed(Grab::ALL[index])),
            3 => Some(Self::Random),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub state: State,
    pub attacker: Attacker,
    pub grab: GrabChoice,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            state: State::Grab,
            attacker: Attacker::Fighter(ATTACKERS[0].0),
            grab: GrabChoice::Fixed(Grab::Standing),
        }
    }
}

impl Config {
    /// Like other menu dropdowns, the first selected option wins edited conflicting choices;
    /// an absent/empty row uses its factory selection. These keys belong only to Mash out.
    pub fn from_snapshot(rows: &Snapshot) -> Option<Self> {
        let index = |id| {
            rows.get(id)
                .and_then(|row| row.iter().position(|value| *value != 0))
                .unwrap_or(0)
        };
        Some(Self {
            state: State::from_index(index(STATE_ROW))?,
            attacker: Attacker::from_index(index(ATTACKER_ROW))?,
            grab: GrabChoice::from_index(index(GRAB_ROW))?,
        })
    }

    /// Resolve once per requested attempt using independent, externally supplied rolls. This
    /// module draws no RNG and contains no assumptions about catch frames, durations or inputs.
    pub fn resolve(self, attacker_roll: u64, grab_roll: u64) -> Option<OrdinaryGrab> {
        let kind = match self.attacker {
            Attacker::Fighter(kind) => kind,
            Attacker::Random => ATTACKERS[(attacker_roll % ATTACKERS.len() as u64) as usize].0,
        };
        let grab = match self.grab {
            GrabChoice::Fixed(grab) => grab,
            GrabChoice::Random => Grab::ALL[(grab_roll % Grab::ALL.len() as u64) as usize],
        };
        ordinary_grab(kind, grab)
    }
}

/// A requested ordinary native move, not proof of entry. Runtime setup must load the attacker,
/// execute this variant and verify native capture before collecting the initialized timer seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrdinaryGrab {
    pub attacker_kind: u32,
    pub attacker_name: &'static str,
    pub grab: Grab,
}

pub fn ordinary_grab(kind: u32, grab: Grab) -> Option<OrdinaryGrab> {
    ATTACKERS
        .iter()
        .find(|entry| entry.0 == kind)
        .map(|&(attacker_kind, attacker_name)| OrdinaryGrab {
            attacker_kind,
            attacker_name,
            grab,
        })
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::{menu::table::{Control, ROWS}, settings::{self, Settings}};

    #[test]
    fn ordinary_grab_catalog_matches_tr_choices() {
        assert_eq!(ATTACKERS.len(), 89);
        assert_eq!(ATTACKER_LABELS.len(), 90);
        for (index, &(kind, name)) in CHOICES.iter().enumerate() {
            assert!(!ATTACKERS[index + 1..].iter().any(|entry| entry.0 == kind));
            assert_eq!(ATTACKER_LABELS[index], name);
            assert_eq!(Attacker::from_index(index), Some(Attacker::Fighter(kind)));
            for grab in Grab::ALL {
                assert_eq!(ordinary_grab(kind, grab), Some(OrdinaryGrab { attacker_kind: kind, attacker_name: name, grab }));
            }
        }
        for (kind, name) in [(0x28, "Lucas"), (0x2b, "Olimar"), (0x4b, "Ice Climbers"), (0x49, "Mii Swordfighter"), (0x48, "Mii Brawler"), (0x4a, "Mii Gunner")] {
            assert_eq!(ordinary_grab(kind, Grab::Standing).unwrap().attacker_name, name);
        }
        assert_eq!(ordinary_grab(0x4c, Grab::Standing), None, "Nana is a secondary actor");
        assert_eq!(ordinary_grab(u32::MAX, Grab::Standing), None);
        assert_eq!(Attacker::from_index(90), None);
    }

    #[test]
    fn selection_factory_and_state_are_stable() {
        let snapshot = Snapshot::factory();
        assert_eq!(Config::from_snapshot(&snapshot), Some(Config::default()));
        assert_eq!(State::from_index(0), Some(State::Grab));
        assert_eq!(State::from_index(1), None);
        assert_eq!(STATE_LABELS, ["Grab"]);
        assert_eq!(Config::default().resolve(0, 0), Some(OrdinaryGrab {
            attacker_kind: 0, attacker_name: "Mario", grab: Grab::Standing,
        }));
        for (id, expected) in [(ROW, &[1, 0][..]), (STATE_ROW, &[1][..])] {
            assert_eq!(snapshot.get(id), Some(expected));
        }
        assert_eq!(snapshot.get(GRAB_ROW), None, "stage 1 uses the standing default");
        assert!(ROWS.iter().all(|row| row.id != GRAB_ROW));
        for (id, labels) in [(ATTACKER_ROW, &ATTACKER_LABELS[..]), (STATE_ROW, &STATE_LABELS[..])] {
            let row = ROWS.iter().find(|row| row.id == id).unwrap();
            let Control::Dropdown(actual) = row.control else { panic!("dropdown {id}") };
            assert_eq!(actual, labels);
        }
    }

    #[test]
    fn random_attacker_covers_full_catalog() {
        let config = Config { attacker: Attacker::Random, ..Config::default() };
        assert_eq!(Attacker::from_index(RANDOM_ATTACKER), Some(Attacker::Random));
        assert_eq!(ATTACKER_LABELS[RANDOM_ATTACKER], "Random");
        for (roll, &(kind, _)) in CHOICES.iter().enumerate() {
            assert_eq!(config.resolve(roll as u64, 0).unwrap().attacker_kind, kind);
            assert_eq!(config.resolve((roll + 89) as u64, 0).unwrap().attacker_kind, kind);
        }
        let last = config.resolve(u64::MAX, u64::MAX).unwrap();
        assert!(CHOICES.iter().any(|entry| entry.0 == last.attacker_kind));
        let both = Config { attacker: Attacker::Random, grab: GrabChoice::Random, ..Config::default() };
        for (fighter_roll, &(kind, _)) in CHOICES.iter().enumerate() {
            for (grab_roll, expected) in Grab::ALL.into_iter().enumerate() {
                let selected = both.resolve(fighter_roll as u64, grab_roll as u64).unwrap();
                assert_eq!((selected.attacker_kind, selected.grab), (kind, expected));
            }
        }
    }

    #[test]
    fn random_variant_covers_all_three() {
        let config = Config { grab: GrabChoice::Random, ..Config::default() };
        for (roll, expected) in [(0, Grab::Standing), (1, Grab::Dash), (2, Grab::Pivot), (3, Grab::Standing), (4, Grab::Dash), (5, Grab::Pivot)] {
            assert_eq!(config.resolve(0, roll).unwrap().grab, expected);
        }
        for (index, expected) in Grab::ALL.into_iter().enumerate() {
            let fixed = Config { grab: GrabChoice::from_index(index).unwrap(), ..Config::default() };
            assert_eq!(fixed.resolve(u64::MAX, u64::MAX).unwrap().grab, expected);
        }
        assert_eq!(GrabChoice::from_index(4), None);
    }

    #[test]
    fn selection_roundtrip_preserves_cpu_settings() {
        let mut value = Settings::factory();
        let mut cpu = vec![0; CHOICES.len()];
        cpu[1] = 1;
        value.menu.set_kept("cpu_character", serde_json::json!(cpu)).unwrap();
        value.menu.set("cpu_level", &[0, 0, 0, 1]).unwrap();
        let cpu_before = value.menu.extra().get("cpu_character").cloned();
        let cpu_level_before = value.menu.get("cpu_level").unwrap().to_vec();
        let mut random = vec![0; ATTACKER_LABELS.len()];
        random[RANDOM_ATTACKER] = 1;
        value.menu.set(ATTACKER_ROW, &random).unwrap();
        let after = settings::parse(&value.to_json());
        assert!(after.issues.is_empty());
        assert_eq!(after.settings.menu.extra().get("cpu_character").cloned(), cpu_before);
        assert_eq!(after.settings.menu.get("cpu_level"), Some(cpu_level_before.as_slice()));
        assert_eq!(Config::from_snapshot(&after.settings.menu).unwrap().attacker, Attacker::Random);
    }
    }
}
