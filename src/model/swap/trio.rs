//! Practice roster choices, including individual starting forms.
//! Order is serialized as one-hot menu choices: keep it stable.
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub(crate) const LEGACY_CHOICE_COUNT: usize = 80;
pub(crate) const PRE_IC_CHOICE_COUNT: usize = 85;
pub(crate) const PRE_MII_CHOICE_COUNT: usize = 86;
pub(crate) const MII_ALPHA_CHOICE_COUNT: usize = 87; // Existing Sword-only envelope.
pub(crate) const MII_CLASSES_CHOICE_COUNT: usize = 89;

pub(crate) use crate::model::fighter_choices::CHOICES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrioFighter(u32);

impl TrioFighter {
    pub const MARIO: Self = Self(0);
    pub const KIRBY: Self = Self(0x06);
    #[cfg(test)]
    pub const PIKACHU: Self = Self(0x08);
    #[cfg(test)]
    pub const BOWSER: Self = Self(0x0f);
    #[cfg(test)]
    pub const GAMEWATCH: Self = Self(0x1c);
    #[cfg(test)]
    pub const PALUTENA: Self = Self(0x36);
    #[cfg(test)]
    pub const BAYONETTA: Self = Self(0x40);

    pub const fn kind(self) -> u32 {
        self.0
    }
    pub fn name(self) -> &'static str {
        CHOICES
            .iter()
            .find(|(kind, _)| *kind == self.0)
            .map(|(_, name)| *name)
            .unwrap_or("Unavailable")
    }
    pub fn supported_kind(kind: u32) -> bool {
        CHOICES.iter().any(|(candidate, _)| *candidate == kind)
    }
    pub fn available() -> Vec<Self> {
        CHOICES.iter().map(|(kind, _)| Self(*kind)).collect()
    }
    pub fn from_kind(kind: u32) -> Option<Self> {
        Self::supported_kind(kind).then_some(Self(kind))
    }
}

impl Serialize for TrioFighter {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        CHOICES
            .iter()
            .map(|(kind, _)| u8::from(*kind == self.0))
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for TrioFighter {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let values = Vec::<u8>::deserialize(deserializer)?;
        if ![
            LEGACY_CHOICE_COUNT,
            PRE_IC_CHOICE_COUNT,
            PRE_MII_CHOICE_COUNT,
            MII_ALPHA_CHOICE_COUNT,
            MII_CLASSES_CHOICE_COUNT,
        ]
        .contains(&values.len())
            || values.iter().any(|v| *v > 1)
            || values.iter().filter(|v| **v == 1).count() != 1
        {
            return Err(serde::de::Error::custom(
                "select exactly one supported trio fighter",
            ));
        }
        let selected = values.iter().position(|v| *v == 1).unwrap();
        // Only the exact known alpha extension is accepted. On a base build,
        // its unsupported sole Sword choice downgrades to Mario, never All.
        Ok(CHOICES
            .get(selected)
            .map_or(Self::MARIO, |(kind, _)| Self(*kind)))
    }
}

