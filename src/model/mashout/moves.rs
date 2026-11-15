//! Move selection uses the existing roster and menu indices; initialized native records supply the mechanics.
use crate::model::settings::Snapshot;

#[path = "../swap/choices.rs"]
mod choices;
pub const ATTACKERS: &[(u32, &str)] = choices::CHOICES;
pub const ROW: &str = "mashout";
pub const STATE_ROW: &str = "mashout_state";
pub const ATTACKER_ROW: &str = "mashout_attacker";
pub const GRAB_ROW: &str = "mashout_grab";
pub const RANDOM_ATTACKER: usize = ATTACKERS.len();
pub const STATE_LABELS: [&str; 1] = ["Grab"];
pub const GRAB_LABELS: [&str; 4] = ["Standing", "Dash", "Pivot", "Random"];
pub const ON_OFF_LABELS: [&str; 2] = ["Off", "On"];

const fn attacker_labels() -> [&'static str; RANDOM_ATTACKER + 1] {
    let mut labels = ["Random"; RANDOM_ATTACKER + 1];
    let mut index = 0;
    while index < ATTACKERS.len() {
        labels[index] = ATTACKERS[index].1;
        index += 1;
    }
    labels
}
pub const ATTACKER_LABELS: [&str; RANDOM_ATTACKER + 1] = attacker_labels();

/// Indices into the state, attacker and variant menu data. The current runtime exposes standing Grab only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub state: usize,
    pub attacker: usize,
    pub grab: usize,
}
impl Config {
    pub fn from_snapshot(rows: &Snapshot) -> Option<Self> {
        let index = |id| {
            rows.get(id)
                .and_then(|row| row.iter().position(|v| *v != 0))
                .unwrap_or(0)
        };
        let value = Self {
            state: index(STATE_ROW),
            attacker: index(ATTACKER_ROW),
            grab: index(GRAB_ROW),
        };
        value.resolve(0, 0).map(|_| value)
    }
    /// Resolve once using externally supplied rolls. All variants use the same observed native profile.
    pub fn resolve(self, attacker_roll: u64, grab_roll: u64) -> Option<OrdinaryGrab> {
        if self.state >= STATE_LABELS.len()
            || self.attacker > RANDOM_ATTACKER
            || self.grab >= GRAB_LABELS.len()
        {
            return None;
        }
        let attacker = if self.attacker == RANDOM_ATTACKER {
            (attacker_roll % ATTACKERS.len() as u64) as usize
        } else {
            self.attacker
        };
        let grab = if self.grab == GRAB_LABELS.len() - 1 {
            (grab_roll % 3) as usize
        } else {
            self.grab
        };
        let &(attacker_kind, attacker_name) = ATTACKERS.get(attacker)?;
        Some(OrdinaryGrab {
            attacker_kind,
            attacker_name,
            grab,
        })
    }
}
/// Requested native move, not proof of entry. Variant indices follow GRAB_LABELS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrdinaryGrab {
    pub attacker_kind: u32,
    pub attacker_name: &'static str,
    pub grab: usize,
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
            for (index, &(kind, name)) in choices::CHOICES.iter().enumerate() {
                assert!(!ATTACKERS[index + 1..].iter().any(|entry| entry.0 == kind));
                assert_eq!(ATTACKER_LABELS[index], name);
                for grab in 0..3 {
                    let config = Config { attacker: index, grab, ..Config::default() };
                    assert_eq!(config.resolve(u64::MAX, u64::MAX), Some(OrdinaryGrab { attacker_kind: kind, attacker_name: name, grab }));
                }
            }
            for config in [Config { state: 1, ..Config::default() }, Config { attacker: 90, ..Config::default() }, Config { grab: 4, ..Config::default() }] {
                assert_eq!(config.resolve(0, 0), None);
            }
        }

        #[test]
        fn selection_factory_and_state_are_stable() {
            let snapshot = Snapshot::factory();
            assert_eq!(Config::from_snapshot(&snapshot), Some(Config::default()));
            assert_eq!(STATE_LABELS, ["Grab"]);
            assert_eq!(Config::default().resolve(0, 0), Some(OrdinaryGrab { attacker_kind: 0, attacker_name: "Mario", grab: 0 }));
            assert_eq!(snapshot.get(ROW), Some(&[1, 0][..]));
            assert_eq!(snapshot.get(STATE_ROW), Some(&[1][..]));
            assert_eq!(snapshot.get(GRAB_ROW), None);
            assert!(ROWS.iter().all(|row| row.id != GRAB_ROW));
            for (id, labels) in [(ATTACKER_ROW, &ATTACKER_LABELS[..]), (STATE_ROW, &STATE_LABELS[..])] {
                let Control::Dropdown(actual) = ROWS.iter().find(|row| row.id == id).unwrap().control else { panic!("dropdown {id}") };
                assert_eq!(actual, labels);
            }
        }

        #[test]
        fn random_attacker_covers_full_catalog() {
            let config = Config { attacker: RANDOM_ATTACKER, grab: 3, ..Config::default() };
            assert_eq!(ATTACKER_LABELS[RANDOM_ATTACKER], "Random");
            for (index, &(kind, _)) in choices::CHOICES.iter().enumerate() {
                for roll in [index as u64, index as u64 + 89] {
                    for grab in 0..3 {
                        assert_eq!(config.resolve(roll, grab).unwrap().attacker_kind, kind);
                        assert_eq!(config.resolve(roll, grab).unwrap().grab, grab as usize);
                    }
                }
            }
            assert!(ATTACKERS.iter().any(|entry| entry.0 == config.resolve(u64::MAX, u64::MAX).unwrap().attacker_kind));
        }

        #[test]
        fn random_variant_covers_all_three() {
            let random = Config { grab: 3, ..Config::default() };
            for roll in 0..9 {
                assert_eq!(random.resolve(0, roll).unwrap().grab, (roll % 3) as usize);
            }
        }

        #[test]
        fn selection_roundtrip_preserves_cpu_settings() {
            let mut value = Settings::factory();
            let mut cpu = vec![0; ATTACKERS.len()];
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
            assert_eq!(Config::from_snapshot(&after.settings.menu).unwrap().attacker, RANDOM_ATTACKER);
        }
    }
}
