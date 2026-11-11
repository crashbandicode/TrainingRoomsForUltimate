use serde::{Deserialize, Deserializer, Serialize, Serializer};

use {crate::engine::swap::trio::{
        CHOICES, LEGACY_CHOICE_COUNT, MII_ALPHA_CHOICE_COUNT, MII_CLASSES_CHOICE_COUNT, PRE_IC_CHOICE_COUNT,
        PRE_MII_CHOICE_COUNT,
    }, crate::engine::swap::trio::TrioFighter};

/// Allocation-free roster and hat lists (R5, F10): the catalog bounds their length.
pub type FighterList = crate::engine::compat::fixed::FixedList<TrioFighter, { CHOICES.len() }>;
pub type KindList = crate::engine::compat::fixed::FixedList<u32, { CHOICES.len() }>;

/// Catalog positions stay fixed; nonzero ranks represent the user's cycle order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderedFighterRoster {
    all: bool,
    ranks: [u8; CHOICES.len()],
}

impl OrderedFighterRoster {
    pub const fn all() -> Self {
        Self {
            all: true,
            ranks: [0; CHOICES.len()],
        }
    }

    /// Retains the first occurrence of each supported fighter. Empty input is invalid.
    pub const fn from_fighters(fighters: &[TrioFighter]) -> Option<Self> {
        let mut roster = Self {
            all: false,
            ranks: [0; CHOICES.len()],
        };
        let mut rank = 0;
        let mut input = 0;
        while input < fighters.len() {
            let mut catalog = 0;
            while catalog < CHOICES.len() {
                let mut seen_group = false;
                let mut earlier = 0;
                while earlier < CHOICES.len() {
                    if roster.ranks[earlier] != 0
                        && crate::engine::swap::roster_grid::group_kind(CHOICES[earlier].0)
                            == crate::engine::swap::roster_grid::group_kind(fighters[input].kind())
                    {
                        seen_group = true;
                    }
                    earlier += 1;
                }
                if fighters[input].kind() == CHOICES[catalog].0 && !seen_group {
                    rank += 1;
                    roster.ranks[catalog] = rank;
                }
                catalog += 1;
            }
            input += 1;
        }
        if rank == 0 {
            None
        } else {
            Some(roster)
        }
    }

    pub fn fighters(self) -> Vec<TrioFighter> {
        self.fighters_excluding(None)
    }

    /// `[all, ranks...]`, the same array the portrait editor and JSON store.
    pub fn values(self) -> Vec<u8> {
        std::iter::once(u8::from(self.all))
            .chain(self.ranks)
            .collect()
    }

    /// Known roster catalog with Kirby removed. Does not apply original-P1 exclusion.
    pub fn hat_kinds(self) -> Vec<u32> {
        self.hat_kinds_fixed().to_vec()
    }

    /// `hat_kinds` without heap (the preload planner's per-tick form).
    pub fn hat_kinds_fixed(self) -> KindList {
        let mut out = KindList::new(0);
        for fighter in self.fighters_excluding_fixed(None).iter() {
            if fighter.kind() != TrioFighter::KIRBY.kind()
                && (cfg!(feature="mii-managed-helper-alpha") || !(0x48..=0x4a).contains(&fighter.kind()))
            {
                out.push(fighter.kind());
            }
        }
        out
    }

    pub fn fighters_excluding(self, original: Option<u32>) -> Vec<TrioFighter> {
        self.fighters_excluding_fixed(original).to_vec()
    }

    /// `fighters_excluding` without heap (the preload planner's per-tick form): All in CSS order, otherwise the ranked
    /// fighters in rank order, the original's native group left out.
    pub fn fighters_excluding_fixed(self, original: Option<u32>) -> FighterList {
        let mut out = FighterList::new(TrioFighter::MARIO);
        if self.all {
            let mut kinds = [0u32; CHOICES.len()];
            for (slot, (kind, _)) in kinds.iter_mut().zip(CHOICES) {
                *slot = *kind;
            }
            let mut values = [0u8; CHOICES.len() + 1];
            values[0] = 1;
            values[1..].copy_from_slice(&self.ranks);
            crate::engine::swap::roster_grid::css_order_each(&kinds, &values, original, |kind| {
                if let Some(fighter) = TrioFighter::from_kind(kind) {
                    out.push(fighter);
                }
            });
            return out;
        }
        let excluded = original.map(crate::engine::swap::roster_grid::group_kind);
        for rank in 1..=CHOICES.len() {
            let Some(index) = self.ranks.iter().position(|r| usize::from(*r) == rank) else { break };
            let Some(fighter) = TrioFighter::from_kind(CHOICES[index].0) else { continue };
            if excluded != Some(crate::engine::swap::roster_grid::group_kind(fighter.kind())) {
                out.push(fighter);
            }
        }
        out
    }


}

impl Serialize for OrderedFighterRoster {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        std::iter::once(u8::from(self.all))
            .chain(self.ranks.iter().copied())
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OrderedFighterRoster {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let values = Vec::<u8>::deserialize(deserializer)?;
        if ![
            LEGACY_CHOICE_COUNT + 1,
            PRE_IC_CHOICE_COUNT + 1,
            PRE_MII_CHOICE_COUNT + 1,
            MII_ALPHA_CHOICE_COUNT + 1,
            MII_CLASSES_CHOICE_COUNT + 1,
        ]
        .contains(&values.len())
            || values[0] > 1
        {
            return Err(serde::de::Error::custom("invalid ordered character roster"));
        }
        let mut ranks = [0; CHOICES.len()];
        let supported = (values.len() - 1).min(CHOICES.len());
        ranks[..supported].copy_from_slice(&values[1..=supported]);
        let all = values[0] == 1;
        // Validate the complete persisted order before dropping the known
        // unsupported alpha choices. A bad tail must not hide a gap or duplicate.
        let mut sorted: Vec<_> = values[1..]
            .iter()
            .copied()
            .filter(|rank| *rank != 0)
            .collect();
        sorted.sort_unstable();
        let valid = if all {
            // New-format All may store one non-default start per group. Legacy
            // mixed All/ranks remains invalid, rather than silently discarding it.
            sorted.is_empty()
                || ([PRE_MII_CHOICE_COUNT + 1, MII_ALPHA_CHOICE_COUNT + 1, MII_CLASSES_CHOICE_COUNT + 1].contains(&values.len())
                    && values[1..].iter().enumerate().all(|(i, rank)| {
                        *rank == 0
                            || (*rank == 1
                                && CHOICES
                                    .get(i)
                                    .is_some_and(|(kind, _)| [0x5b, 0x25, 0x26].contains(kind)))
                    })
                    && ranks
                        .iter()
                        .zip(CHOICES)
                        .filter(|(rank, (kind, _))| **rank != 0 && [0x25, 0x26].contains(kind))
                        .count()
                        <= 1)
        } else {
            !sorted.is_empty() && sorted.iter().copied().eq(1..=sorted.len() as u8)
        };
        if !valid {
            return Err(serde::de::Error::custom(
                "choose All or a nonempty contiguous character order",
            ));
        }
        if all {
            return Ok(Self { all, ranks });
        }
        // Validate the legacy rank sequence before migrating duplicate groups.
        let mut entries: Vec<_> = CHOICES
            .iter()
            .zip(ranks)
            .filter(|(_, rank)| *rank != 0)
            .collect();
        entries.sort_unstable_by_key(|(_, rank)| *rank);
        let fighters: Vec<_> = entries
            .into_iter()
            .filter_map(|((kind, _), _)| TrioFighter::from_kind(*kind))
            .collect();
        // A valid Mii-only alpha roster has no supported choices on base.
        // Preserve a deterministic single choice rather than enabling All.
        if fighters.is_empty() && [MII_ALPHA_CHOICE_COUNT + 1, MII_CLASSES_CHOICE_COUNT + 1].contains(&values.len()) {
            return Ok(Self::from_fighters(&[TrioFighter::MARIO]).unwrap());
        }
        Self::from_fighters(&fighters)
            .ok_or_else(|| serde::de::Error::custom("empty character order"))
    }
}

pub const DEFAULT_PLAYER_ROSTER: OrderedFighterRoster =
    match OrderedFighterRoster::from_fighters(&[
        TrioFighter::GAMEWATCH,
        TrioFighter::PALUTENA,
        TrioFighter::PIKACHU,
    ]) {
        Some(roster) => roster,
        None => unreachable!(),
    };

pub const DEFAULT_CPU_ROSTER: OrderedFighterRoster = match OrderedFighterRoster::from_fighters(&[
    TrioFighter::PIKACHU,
    TrioFighter::KIRBY,
    TrioFighter::BOWSER,
    TrioFighter::BAYONETTA,
]) {
    Some(roster) => roster,
    None => unreachable!(),
};

/// Kirby hats start on Mario, separate from either fighter-cycle default.
pub const DEFAULT_KIRBY_HATS: OrderedFighterRoster =
    match OrderedFighterRoster::from_fighters(&[TrioFighter::MARIO]) {
        Some(roster) => roster,
        None => unreachable!(),
    };

