//! Mash out's portrait-grid picker: the options a chip offers, taken from the reviewed move table (`moves`), and the way
//! back from a saved setting to its chip and line. The picker writes the same `mashout_state`, `mashout_attacker` and
//! `mashout_grab` values the State / Attacker move / Grab type rows wrote.
use core::fmt::{self, Write};

use super::moves::{self, Config, Plan, State, CATALOG, GRAB_LABELS, RANDOM_ATTACKER, RANDOM_STATE, STATE_LABELS};

/// A chip of the grid: a fighter (the kind it shows), or the Random chip on the grid's All cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    Fighter(u32),
    Random,
}

/// One line of a chip's popup: the persisted state and attacker indices, and whether it takes the grab type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub state: usize,
    pub attacker: usize,
    pub grab: bool,
}

/// The most lines a popup has (the Random chip's six).
pub const MAX: usize = 6;

/// The Random chip's lines: Random state, then Random within each state, as the dropdowns offered them.
const RANDOM: [usize; MAX] = [RANDOM_STATE, 0, 1, 2, 3, 4];

/// The options of `chip`, in catalog order: a fighter's ordinary Grab, then its reviewed moves. Only valid options
/// (`moves::attacker_valid`: Available and startable) are listed.
pub fn choices(chip: Chip) -> impl Iterator<Item = Choice> {
    let kind = match chip {
        Chip::Fighter(kind) => Some(kind),
        Chip::Random => None,
    };
    let fighter = CATALOG.iter().enumerate().filter(move |(_, s)| Some(s.attacker_kind) == kind).map(|(row, s)| Choice {
        state: State::ALL.iter().position(|x| *x == s.state).unwrap_or(0),
        attacker: if row < RANDOM_ATTACKER { row } else { row + 1 },
        grab: s.plan == Plan::Grab,
    });
    let random = RANDOM.into_iter().filter(move |_| kind.is_none()).map(|state| Choice {
        state,
        attacker: RANDOM_ATTACKER,
        grab: state == 0 || state == RANDOM_STATE,
    });
    fighter.chain(random).filter(|c| moves::attacker_valid(c.state, c.attacker))
}

/// A chip with no valid option is greyed and cannot be picked.
pub fn pickable(chip: Chip) -> bool {
    choices(chip).next().is_some()
}

/// The chip a saved setting belongs to (`None` for an invalid pair).
pub fn chip_of(config: Config) -> Option<Chip> {
    if !moves::attacker_valid(config.state, config.attacker) {
        return None;
    }
    Some(moves::scenario(config.attacker).map_or(Chip::Random, |s| Chip::Fighter(s.attacker_kind)))
}

/// The popup line of a saved setting on its chip.
pub fn line_of(config: Config) -> Option<usize> {
    choices(chip_of(config)?).position(|c| c.state == config.state && c.attacker == config.attacker)
}

/// The option a saved setting is.
pub fn choice_of(config: Config) -> Option<Choice> {
    choices(chip_of(config)?).nth(line_of(config)?)
}

/// The option's name: "Empty minecart" (with `fighter`, "Steve · Empty minecart"), "Random state", "Random Bury".
pub fn write_name(c: Choice, fighter: bool, out: &mut impl Write) -> fmt::Result {
    match moves::scenario(c.attacker) {
        Some(s) if fighter => write!(out, "{} · {}", s.attacker_name, s.name),
        Some(s) => out.write_str(s.name),
        None if c.state == RANDOM_STATE => out.write_str("Random state"),
        None => write!(out, "Random {}", STATE_LABELS.get(c.state).copied().unwrap_or("-")),
    }
}

/// The grab type's label (`GRAB_LABELS`).
pub fn grab_label(grab: usize) -> &'static str {
    GRAB_LABELS.get(grab).copied().unwrap_or("-")
}
