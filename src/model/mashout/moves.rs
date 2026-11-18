//! Reviewed primary move catalog (MOVES.md, 2026-11-15). These are entry requests,
//! not runtime support claims. Actual initialized native records supply all mechanics.
use crate::model::{fighter_choices as choices, settings::Snapshot};

pub const ATTACKERS: &[(u32, &str)] = choices::CHOICES;
pub const ROW: &str = "mashout";
pub const STATE_ROW: &str = "mashout_state";
pub const ATTACKER_ROW: &str = "mashout_attacker";
pub const GRAB_ROW: &str = "mashout_grab";
pub const RANDOM_ATTACKER: usize = ATTACKERS.len();
pub const RANDOM_STATE: usize = 5;
pub const STATE_LABELS: [&str; 6] = ["Grab", "Bury", "Frozen", "Egg", "Inhale", "Random state"];
pub const GRAB_LABELS: [&str; 4] = ["Standing", "Dash", "Pivot", "Random"];
pub const ON_OFF_LABELS: [&str; 2] = ["Off", "On"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State { Grab, Bury, Frozen, Egg, Inhale }
impl State {
    pub const ALL: [Self; 5] = [Self::Grab, Self::Bury, Self::Frozen, Self::Egg, Self::Inhale];
}

/// Native entry sequence family; the runtime must prove the complete move, not
/// just issue its first command. In particular Final/Frozen entries retain the
/// unresolved transitions recorded in MOVES.md.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    Grab, Cargo, AirSideSpecial, NeutralSpecial, Cling, Drain, Pin, Drag, Whip,
    Trolley, DownThrow, UpSpecial, SideSpecial, DownSmash, Roller, DownTilt,
    Final, Flip, Jab3, Hero, Judge, Freeze, Blizzard, IceShot, IceMagic, Inhale, Egg,
}

/// The one per-move availability path for the menu, concrete picks and the Random drill.
/// Rows keep their CATALOG index and id either way, so saved settings never shift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveAvailability {
    Available,
    /// Listed greyed and never selectable or rolled.
    NotMashable,
    /// Not listed at all (operator 2026-11-17: Final Smash rows 106 and 115 dropped).
    Hidden,
}

/// 116 Sora Ice magic (reviewer 08:37Z, option b): the Blizzard cast outlasts the hold and its last shard frame is
/// not byte-provable, so S1 cannot close; it runs as practice and never grades (sora116-g2-pair-xps harvest-6).
pub const fn practice_only(id: u16) -> Option<&'static str> {
    match id {
        // restriction: mashout-blizzard-practice
        116 => Some("Sora's Blizzard re-freezes outlast the escape · practice only, no grade"),
        _ => None,
    }
}

pub const fn availability(id: u16) -> MoveAvailability {
    match id {
        106 | 115 => MoveAvailability::Hidden,
        // Primary ruling 14:00Z 2026-11-17 (capture lane evidence, capture-lane/runtime/OUTCOMES.md):
        // 89 Cargo and 96 Whip hold with automatic decay 0 and no product tick source; 90 lands
        // after ~26 ticks, shorter than any authored cadence; 120's victim is never mashable.
        89 | 90 | 96 | 120 => MoveAvailability::NotMashable,
        _ => MoveAvailability::Available,
    }
}


/// Available AND startable now: `entry::start` is a pure plan-table match (no native call,
/// no allocation), so a row becomes selectable by itself once its plan gains a start.
pub fn selectable(s: Scenario) -> bool {
    availability(s.id) == MoveAvailability::Available && super::entry::start(s).is_some()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scenario {
    /// Stable move identity, independent of the menu's Random hole at index89.
    pub id: u16,
    pub state: State,
    pub attacker_kind: u32,
    pub attacker_name: &'static str,
    pub name: &'static str,
    pub plan: Plan,
    /// Ordinary standing/dash/pivot index. Other move families use0.
    pub grab: usize,
    /// Row/profile identifier in artifacts/rc15-mashout/MOVES.md.
    pub evidence: &'static str,
}

/// TR choices Olimar (0x2b). Its ordinary grab row is graded with one Pikmin (`entry::ONE_PIKMIN`).
pub const OLIMAR: u32 = 0x2b;

const fn entry(id: u16, state: State, kind: u32, name: &'static str, plan: Plan, evidence: &'static str) -> Scenario {
    let mut i = 0;
    while i < ATTACKERS.len() {
        if ATTACKERS[i].0 == kind {
            return Scenario { id, state, attacker_kind: kind, attacker_name: ATTACKERS[i].1, name, plan, grab: 0, evidence };
        }
        i += 1;
    }
    panic!("reviewed move kind missing from TR fighter choices");
}

// MOVES.md positive common-clatter table, then B01–B12/F01–F07/E01/I01–I03.
// No additional14 capture candidates, Kirby copies or item entries are promoted.
// Ground/air/phases remain variants of a move ID, not duplicated catalog rows.
const EXTRA: [Scenario; 32] = [
    entry(89, State::Grab, 0x01, "Cargo shoulder", Plan::Cargo, "Grab DK / common25d720"),
    entry(90, State::Grab, 0x18, "Aerial side special", Plan::AirSideSpecial, "Grab Ganon / commond57d0"),
    entry(91, State::Grab, 0x21, "Chomp", Plan::NeutralSpecial, "Grab Wario / common9c740"),
    entry(92, State::Grab, 0x27, "Monkey Flip cling", Plan::Cling, "Grab Diddy / commonef0a0"),
    entry(93, State::Grab, 0x38, "Nosferatu", Plan::Drain, "Grab Robin / commond95e0"),
    entry(94, State::Grab, 0x3f, "Dragon Lunge pin", Plan::Pin, "Grab Corrin / common2333c0"),
    entry(95, State::Grab, 0x42, "Space Pirate Rush", Plan::Drag, "Grab Ridley / commondcd00"),
    entry(96, State::Grab, 0x47, "Alolan Whip", Plan::Whip, "Grab Incineroar / common28a510"),
    entry(97, State::Grab, 0x58, "Empty minecart", Plan::Trolley, "Grab Steve / common34da0"),
    entry(98, State::Bury, 0x54, "Down throw", Plan::DownThrow, "B01"),
    entry(99, State::Bury, 0x2a, "Up special descent", Plan::UpSpecial, "B02"),
    entry(100, State::Bury, 0x01, "Side special", Plan::SideSpecial, "B03"),
    entry(101, State::Bury, 0x1c, "Down smash", Plan::DownSmash, "B04"),
    entry(102, State::Bury, 0x41, "Roller", Plan::Roller, "B05"),
    entry(103, State::Bury, 0x45, "Down throw", Plan::DownThrow, "B06"),
    entry(104, State::Bury, 0x45, "Down tilt", Plan::DownTilt, "B07"),
    entry(105, State::Bury, 0x30, "Down smash", Plan::DownSmash, "B08"),
    entry(106, State::Bury, 0x2b, "Final Smash start", Plan::Final, "B09"),
    entry(107, State::Bury, 0x2d, "Down throw", Plan::DownThrow, "B10"),
    entry(108, State::Bury, 0x20, "Flip stomp", Plan::Flip, "B11"),
    entry(109, State::Bury, 0x32, "Jab3", Plan::Jab3, "B12"),
    entry(110, State::Frozen, 0x53, "Command18", Plan::Hero, "F01"),
    entry(111, State::Frozen, 0x1c, "Judge8", Plan::Judge, "F02"),
    entry(112, State::Frozen, 0x28, "PK Freeze", Plan::Freeze, "F03"),
    entry(113, State::Frozen, 0x4b, "Blizzard", Plan::Blizzard, "F04"),
    entry(114, State::Frozen, 0x4b, "Ice Shot", Plan::IceShot, "F05"),
    entry(115, State::Frozen, 0x4b, "Final Smash wind", Plan::Final, "F06"),
    entry(116, State::Frozen, 0x5d, "Ice magic", Plan::IceMagic, "F07"),
    entry(117, State::Egg, 0x05, "Egg Lay", Plan::Egg, "E01"),
    entry(118, State::Inhale, 0x06, "Inhale", Plan::Inhale, "I01"),
    entry(119, State::Inhale, 0x2a, "Inhale", Plan::Inhale, "I02"),
    entry(120, State::Inhale, 0x45, "Blunderbuss suction", Plan::Inhale, "I03"),
];

const fn catalog() -> [Scenario; 121] {
    let first = entry(0, State::Grab, ATTACKERS[0].0, "Grab", Plan::Grab, "ordinary G");
    let mut rows = [first; 121];
    let mut i = 0;
    while i < ATTACKERS.len() {
        let name = if ATTACKERS[i].0 == OLIMAR { "Grab (1 Pikmin)" } else { "Grab" };
        rows[i] = entry(i as u16, State::Grab, ATTACKERS[i].0, name, Plan::Grab, "ordinary G");
        i += 1;
    }
    let mut j = 0;
    while j < EXTRA.len() {
        rows[i + j] = EXTRA[j];
        j += 1;
    }
    rows
}
pub const CATALOG: [Scenario; 121] = catalog();

const fn attacker_labels() -> [&'static str; CATALOG.len() + 1] {
    let mut labels = ["Random"; CATALOG.len() + 1];
    let mut i = 0;
    while i < ATTACKERS.len() {
        labels[i] = ATTACKERS[i].1;
        i += 1;
    }
    let mut j = 0;
    while j < EXTRA.len() {
        labels[i + 1 + j] = EXTRA[j].name;
        j += 1;
    }
    labels
}
/// Maximum persisted row length. The menu resolves state-filtered labels and
/// indices; the existing ordinary0..88 and Random89 never move.
pub const ATTACKER_LABELS: [&str; CATALOG.len() + 1] = attacker_labels();

pub fn scenario(attacker: usize) -> Option<Scenario> {
    if attacker == RANDOM_ATTACKER { return None; }
    CATALOG.get(if attacker > RANDOM_ATTACKER { attacker - 1 } else { attacker }).copied()
}

fn matches_state(state: usize, value: Scenario) -> bool {
    state == RANDOM_STATE || State::ALL.get(state) == Some(&value.state)
}

pub fn attacker_valid(state: usize, attacker: usize) -> bool {
    state < STATE_LABELS.len() && (attacker == RANDOM_ATTACKER
        || state != RANDOM_STATE && scenario(attacker).is_some_and(|s| matches_state(state, s) && selectable(s)))
}

/// Listed rows: every valid pick plus NotMashable rows, which show greyed but stay invalid.
fn listed(state: usize, attacker: usize) -> bool {
    attacker_valid(state, attacker)
        || state < STATE_LABELS.len() && state != RANDOM_STATE && scenario(attacker).is_some_and(|s| {
            matches_state(state, s) && availability(s.id) == MoveAvailability::NotMashable
        })
}

fn options(state: usize) -> impl Iterator<Item = usize> {
    let random_last = state != 0 && state != RANDOM_STATE && state < STATE_LABELS.len();
    (0..ATTACKER_LABELS.len())
        .filter(move |&i| listed(state, i) && (!random_last || i != RANDOM_ATTACKER))
        .chain(random_last.then_some(RANDOM_ATTACKER))
}

/// Display options map to stable persisted indices. Grab retains Random at89;
/// other filtered states list their moves first and Random last.
pub fn attacker_index(state: usize, option: usize) -> Option<usize> {
    options(state).nth(option)
}

pub fn attacker_option(state: usize, attacker: usize) -> Option<usize> {
    options(state).position(|i| i == attacker)
}

/// Whether a menu option of the filtered attacker list may be confirmed: listed
/// NotMashable rows are shown but never offered.
pub fn attacker_offered(state: usize, option: usize) -> bool {
    attacker_index(state, option).is_some_and(|index| attacker_valid(state, index))
}

pub fn attacker_count(state: usize) -> usize {
    options(state).count()
}

/// The shared renderer supplies the separator only when the move part is nonempty.
pub fn attacker_label(state: usize, option: usize) -> Option<(&'static str, &'static str)> {
    let index = attacker_index(state, option)?;
    Some(match scenario(index) {
        Some(s) => (s.attacker_name, s.name),
        None => ("Random", ""),
    })
}

pub fn uses_grab(config: Config) -> bool {
    attacker_valid(config.state, config.attacker) && if config.attacker == RANDOM_ATTACKER {
        config.state == 0 || config.state == RANDOM_STATE
    } else {
        scenario(config.attacker).is_some_and(|s| s.plan == Plan::Grab)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub state: usize,
    pub attacker: usize,
    pub grab: usize,
}
impl Config {
    pub fn from_snapshot(rows: &Snapshot) -> Option<Self> {
        let index = |id| rows.get(id).and_then(|row| row.iter().position(|v| *v != 0)).unwrap_or(0);
        let value = Self { state: index(STATE_ROW), attacker: index(ATTACKER_ROW), grab: index(GRAB_ROW) };
        value.resolve(0, 0).map(|_| value)
    }

    /// Resolve once from external rolls only. Random state with Random attacker
    /// covers every selectable reviewed row. A concrete conflicting pair is rejected.
    pub fn resolve(self, attacker_roll: u64, grab_roll: u64) -> Option<Scenario> {
        if !attacker_valid(self.state, self.attacker) || self.grab >= GRAB_LABELS.len() { return None; }
        let mut value = if self.attacker == RANDOM_ATTACKER {
            let pool = |s: &&Scenario| matches_state(self.state, **s) && selectable(**s);
            let count = CATALOG.iter().filter(pool).count();
            if count == 0 { return None; }
            *CATALOG.iter().filter(pool).nth((attacker_roll % count as u64) as usize)?
        } else {
            scenario(self.attacker)?
        };
        if value.plan == Plan::Grab {
            value.grab = if self.grab == 3 { (grab_roll % 3) as usize } else { self.grab };
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
        use super::*;
        use crate::model::{menu::table::{Control, ROWS}, settings::{self, Settings}};

        #[test]
        fn ice_shot_is_graded_and_sora_blizzard_remains_practice_only() {
            assert_eq!(practice_only(114), None);
            assert_eq!(CATALOG.iter().filter(|s| practice_only(s.id).is_some()).map(|s| s.id).collect::<Vec<_>>(), [116]);
            assert_eq!(availability(114), MoveAvailability::Available);
            assert!(practice_only(113).is_none() && practice_only(112).is_none());
            // restriction: mashout-blizzard-practice (operator 11-18): 116 stays selectable and in Random.
            assert_eq!(practice_only(116), Some("Sora's Blizzard re-freezes outlast the escape · practice only, no grade"));
            assert_eq!(availability(116), MoveAvailability::Available);
            assert!(selectable(CATALOG[116]));
        }

        #[test]
        fn reviewed_catalog_has_exact_state_counts_and_unique_ids() {
            assert_eq!(CATALOG.len(), 121);
            assert_eq!(State::ALL.map(|state| CATALOG.iter().filter(|s| s.state == state).count()), [98, 12, 7, 1, 3]);
            for (i, s) in CATALOG.iter().enumerate() {
                assert_eq!(s.id as usize, i);
                assert!(choices::CHOICES.contains(&(s.attacker_kind, s.attacker_name)));
                assert!(!s.evidence.is_empty());
                assert!(!CATALOG[i + 1..].iter().any(|other| other.id == s.id));
            }
            // Independent reviewed row/kind order from MOVES B01–B12/F01–F07/E01/I01–I03.
            assert_eq!(CATALOG[98..].iter().map(|s| s.attacker_kind).collect::<Vec<_>>(),
                [0x54,0x2a,0x01,0x1c,0x41,0x45,0x45,0x30,0x2b,0x2d,0x20,0x32,0x53,0x1c,0x28,0x4b,0x4b,0x4b,0x5d,0x05,0x06,0x2a,0x45]);
            assert_eq!(CATALOG.iter().filter(|s| s.plan == Plan::Grab).count(), 89);
            assert_eq!(CATALOG.iter().filter(|s| s.plan == Plan::Final).count(), 2);
            assert_eq!(CATALOG[104].plan, Plan::DownTilt);
            assert_eq!(CATALOG[115].evidence, "F06");
        }

        #[test]
        fn ordinary_indices_and_legacy_snapshot_are_preserved() {
            for (index, &(kind, name)) in choices::CHOICES.iter().enumerate() {
                let s = Config { attacker: index, ..Config::default() }.resolve(0, 0).unwrap();
                assert_eq!((s.id as usize, s.attacker_kind, s.attacker_name, s.plan), (index, kind, name, Plan::Grab));
                assert_eq!(ATTACKER_LABELS[index], name);
            }
            assert_eq!(RANDOM_ATTACKER, 89);
            assert_eq!(ATTACKER_LABELS[89], "Random");
            // Previous ninety-element rows are padded by the real loader, without moving Random.
            let mut old = vec![0; 90]; old[89] = 1;
            let parsed = settings::parse(&serde_json::json!({"menu":{"mashout_attacker":old}}).to_string());
            assert_eq!(Config::from_snapshot(&parsed.settings.menu).unwrap().attacker, 89);
            assert_eq!(parsed.settings.menu.get(ATTACKER_ROW).unwrap().len(), 122);
            assert_eq!(Config::from_snapshot(&Snapshot::factory()), Some(Config::default()));
            assert_eq!(Config::default().resolve(0, 0).unwrap().attacker_name, "Mario");
            assert_eq!(Snapshot::factory().get(GRAB_ROW), Some(&[1,0,0,0][..]));
            for (id, labels) in [(ATTACKER_ROW, &ATTACKER_LABELS[..]), (STATE_ROW, &STATE_LABELS[..]), (GRAB_ROW, &GRAB_LABELS[..])] {
                let Control::Dropdown(actual) = ROWS.iter().find(|row| row.id == id).unwrap().control else { panic!("dropdown {id}") };
                assert_eq!(actual, labels);
            }
        }

        #[test]
        fn olimar_row_label_names_one_pikmin() {
            for s in CATALOG.iter().filter(|s| s.plan == Plan::Grab) {
                assert_eq!(s.name, if s.id == 40 { "Grab (1 Pikmin)" } else { "Grab" }, "id {}", s.id);
            }
            assert_eq!((CATALOG[40].attacker_kind, CATALOG[40].attacker_name), (OLIMAR, "Olimar"));
            let option = attacker_option(0, 40).unwrap();
            assert_eq!(attacker_label(0, option), Some(("Olimar", "Grab (1 Pikmin)")));
        }

        #[test]
        fn random_selection_covers_each_state_and_all_rows() {
            for state in 0..STATE_LABELS.len() {
                let config = Config { state, attacker: 89, grab: 3 };
                let expected: Vec<_> = CATALOG.iter().filter(|s| (state == 5 || s.state == State::ALL[state]) && selectable(**s)).map(|s| s.id).collect();
                let seen: Vec<_> = (0..expected.len()).map(|roll| config.resolve(roll as u64, 0).unwrap().id).collect();
                assert_eq!(seen, expected);
                assert_eq!(config.resolve(expected.len() as u64, 0).unwrap().id, expected[0]);
                assert!(expected.contains(&config.resolve(u64::MAX, u64::MAX).unwrap().id));
            }
            assert_eq!(attacker_count(RANDOM_STATE), 1);
            assert_eq!(attacker_index(RANDOM_STATE, 0), Some(89));
            for index in 0..122 {
                if index != 89 {
                    assert_eq!(Config { state: 5, attacker: index, grab: 0 }.resolve(0, 0), None);
                }
            }
        }

        #[test]
        fn hidden_final_rows_are_never_offered_picked_or_rolled() {
            let hidden: Vec<_> = CATALOG.iter().filter(|s| availability(s.id) == MoveAvailability::Hidden).map(|s| s.id).collect();
            assert_eq!(hidden, [106, 115]);
            assert!(CATALOG.iter().filter(|s| s.plan == Plan::Final).all(|s| !selectable(*s)));
            for id in [106usize, 115] {
                let index = id + 1; // extras sit after Random at89
                assert_eq!(scenario(index).unwrap().id as usize, id, "index and id stay stable");
                for state in 0..STATE_LABELS.len() {
                    assert!(!attacker_valid(state, index));
                    assert_eq!(attacker_option(state, index), None);
                    assert_eq!(Config { state, attacker: index, grab: 0 }.resolve(0, 0), None);
                }
                // A saved index pointing at a hidden row falls back through the existing invalid path.
                let mut row = vec![0; ATTACKER_LABELS.len()];
                row[index] = 1;
                let parsed = settings::parse(&serde_json::json!({"menu":{"mashout_attacker":row}}).to_string());
                assert_eq!(Config::from_snapshot(&parsed.settings.menu), None);
            }
            // Random pools equal exactly the selectable (Available and startable) rows; never hidden or unstartable.
            for state in [1usize, 2, 5] {
                let pool: std::collections::BTreeSet<_> = CATALOG.iter()
                    .filter(|s| (state == 5 || s.state == State::ALL[state]) && selectable(**s)).map(|s| s.id).collect();
                let config = Config { state, attacker: RANDOM_ATTACKER, grab: 0 };
                let rolled: std::collections::BTreeSet<_> = (0..pool.len() as u64 * 4).map(|r| config.resolve(r, 0).unwrap().id).collect();
                assert_eq!(rolled, pool, "state {state}");
                assert!(!rolled.contains(&106) && !rolled.contains(&115));
                assert!(rolled.iter().all(|&id| super::super::entry::start(CATALOG[id as usize]).is_some()));
            }
        }

        #[test]
        fn not_mashable_rows_are_listed_but_never_valid_offered_rolled_or_restored() {
            let not_mashable: Vec<_> = CATALOG.iter()
                .filter(|s| availability(s.id) == MoveAvailability::NotMashable).map(|s| s.id).collect();
            assert_eq!(not_mashable, [89, 90, 96, 120]);
            for id in [89u16, 90, 96, 120] {
                let s = CATALOG[id as usize];
                // Gaining or having a start never enables them (89 has one on this branch).
                assert!(super::super::entry::start(s).is_some(), "id {id} has a real start");
                assert!(!selectable(s));
                let index = id as usize + 1; // extras sit after Random at89
                let own = State::ALL.iter().position(|&st| st == s.state).unwrap();
                let option = attacker_option(own, index).expect("listed in its own state");
                assert!(!attacker_offered(own, option));
                assert_eq!(attacker_label(own, option), Some((s.attacker_name, s.name)));
                for state in 0..STATE_LABELS.len() {
                    assert!(!attacker_valid(state, index));
                    assert_eq!(Config { state, attacker: index, grab: 0 }.resolve(0, 0), None);
                    if state != own { assert_eq!(attacker_option(state, index), None); }
                }
                for roll in 0..512 {
                    for state in [own, RANDOM_STATE] {
                        assert_ne!(Config { state, attacker: RANDOM_ATTACKER, grab: 0 }.resolve(roll, 0).map(|r| r.id), Some(id));
                    }
                }
                let mut row = vec![0; ATTACKER_LABELS.len()];
                row[index] = 1;
                let parsed = settings::parse(&serde_json::json!({"menu":{"mashout_attacker":row}}).to_string());
                assert_eq!(Config::from_snapshot(&parsed.settings.menu), None);
            }
            // Every other listed option is offered, and Random stays offered.
            for state in 0..STATE_LABELS.len() {
                for option in 0..attacker_count(state) {
                    let index = attacker_index(state, option).unwrap();
                    let blocked = scenario(index).is_some_and(|s| availability(s.id) == MoveAvailability::NotMashable)
                        || scenario(index).is_some_and(|s| !selectable(s));
                    assert_eq!(attacker_offered(state, option), !blocked, "state {state} option {option}");
                }
            }
        }

        #[test]
        fn unstartable_rows_are_never_picked_or_rolled() {
            let unstartable: Vec<_> = CATALOG.iter()
                .filter(|s| availability(s.id) == MoveAvailability::Available && super::super::entry::start(**s).is_none())
                .map(|s| s.id).collect();
            assert!(!unstartable.contains(&116), "Sora Ice magic starts through real magic casts (a11b194a)");
            assert!(!unstartable.contains(&89), "Cargo has a start here but is NotMashable, not merely unstartable");
            for id in unstartable {
                let index = if (id as usize) < RANDOM_ATTACKER { id as usize } else { id as usize + 1 };
                for state in 0..STATE_LABELS.len() {
                    assert!(!attacker_valid(state, index), "id {id}");
                    assert_eq!(Config { state, attacker: index, grab: 0 }.resolve(0, 0), None);
                }
                for roll in 0..512 {
                    assert_ne!(Config { state: RANDOM_STATE, attacker: RANDOM_ATTACKER, grab: 0 }.resolve(roll, 0).unwrap().id, id);
                }
            }
            // Egg has one row; it is startable, so its pool is non-empty. An empty pool resolves None (not panics).
            assert!(Config { state: 3, attacker: RANDOM_ATTACKER, grab: 0 }.resolve(0, 0).is_some());
        }

        #[test]
        fn ordinary_variants_and_invalid_selection_boundaries() {
            let config = Config { grab: 3, ..Config::default() };
            for roll in 0..9 { assert_eq!(config.resolve(0, roll).unwrap().grab, (roll % 3) as usize); }
            for grab in 0..3 { assert_eq!(Config { grab, ..Config::default() }.resolve(0, 0).unwrap().grab, grab); }
            for config in [Config { state: 6, ..Config::default() }, Config { attacker: 122, ..Config::default() }, Config { grab: 4, ..Config::default() }, Config { state: 1, ..Config::default() }] {
                assert_eq!(config.resolve(0, 0), None);
            }
            // The first selectable non-grab Grab-state extra (index = id + 1 after Random at89).
            let extra = CATALOG.iter().find(|s| s.state == State::Grab && s.plan != Plan::Grab && selectable(**s)).unwrap().id as usize + 1;
            assert!(!uses_grab(Config { attacker: extra, ..Config::default() }));
            assert_eq!(Config { attacker: extra, grab: 3, ..Config::default() }.resolve(0, 2).unwrap().grab, 0);
        }

        #[test]
        fn selection_roundtrip_preserves_cpu_settings() {
            let mut value = Settings::factory();
            let mut cpu = vec![0; ATTACKERS.len()]; cpu[1] = 1;
            value.menu.set_kept("cpu_character", serde_json::json!(cpu)).unwrap();
            value.menu.set("cpu_level", &[0,0,0,1]).unwrap();
            let cpu_before = value.menu.extra().get("cpu_character").cloned();
            let level_before = value.menu.get("cpu_level").unwrap().to_vec();
            let mut attacker = vec![0; 122]; attacker[120] = 1; // Dedede Inhale; 120 is NotMashable
            value.menu.set(ATTACKER_ROW, &attacker).unwrap();
            value.menu.set(STATE_ROW, &[0,0,0,0,1,0]).unwrap();
            let after = settings::parse(&value.to_json());
            assert!(after.issues.is_empty());
            assert_eq!(after.settings.menu.extra().get("cpu_character").cloned(), cpu_before);
            assert_eq!(after.settings.menu.get("cpu_level"), Some(level_before.as_slice()));
            assert_eq!(Config::from_snapshot(&after.settings.menu).unwrap().resolve(0,0).unwrap().id,119);
        }
    }
}