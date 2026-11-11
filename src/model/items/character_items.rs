//! Stable item menu identities. Runtime item kinds belong to the native adapter.
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const ITEM_NAMES: [&str; 38] = [
    "Banjo & Kazooie: Grenade Egg", "Bowser Jr.: Mechakoopa",
    "Daisy: Smile Turnip", "Daisy: Winky Turnip", "Daisy: Dot-Eyes Turnip",
    "Daisy: Stitch-Face Turnip", "Daisy: Mr. Saturn", "Daisy: Bob-omb",
    "Diddy Kong: Banana Peel", "Diddy Kong: Peanut", "King K. Rool: Crown",
    "Link: Remote Bomb", "Link: Arrow", "Mega Man: Metal Blade",
    "Pac-Man: Cherry", "Pac-Man: Strawberry", "Pac-Man: Orange", "Pac-Man: Apple",
    "Pac-Man: Melon", "Pac-Man: Galaxian", "Pac-Man: Bell", "Pac-Man: Key",
    "Peach: Smile Turnip", "Peach: Winky Turnip", "Peach: Dot-Eyes Turnip",
    "Peach: Stitch-Face Turnip", "Peach: Mr. Saturn", "Peach: Bob-omb",
    "Richter: Holy Water", "R.O.B.: Gyro", "Robin: Tome", "Robin: Levin Sword",
    "Sheik: Burst Grenade", "Simon: Holy Water", "Snake: Hand Grenade",
    "Toon Link: Bomb", "Villager: Wood Chip", "Young Link: Bomb",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CharacterItemChoice(pub Option<u8>);

/// Values, never native item pointers. Missing snapshots in older saves retain
/// the legacy on-load item setting; Empty and Unsupported are distinct states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum HeldItemSnapshot {
    Empty,
    Unsupported,
    Item {
        choice: u8,
        // Logical native variant: turnips store the actual face index 0..7;
        // other items store ITEM_INSTANCE_WORK_INT_VARIATION. No engine work
        // field is rewritten when a natural turnip is saved.
        variation: i32,
        lifetime: i32,
        lifetime_max: i32,
        fuse: i32,
        fuse_max: i32,
    },
}

pub fn valid_variation(choice: usize, variation: i32) -> bool {
    match choice {
        // The source's Smile choice also identifies the four unlisted common
        // native faces in saved data. Menu selection still grants face zero.
        2 | 22 => (0..=4).contains(&variation),
        3 | 23 => variation == 5,
        4 | 24 => variation == 6,
        5 | 25 => variation == 7,
        29 => (0..8).contains(&variation),
        _ => choice < ITEM_NAMES.len() && variation == 0,
    }
}

impl HeldItemSnapshot {
    pub fn valid(self) -> bool {
        match self {
            Self::Item { choice, variation, lifetime, lifetime_max, fuse, fuse_max } => {
                let counter = |value, maximum| (value == -1 && maximum == -1)
                    || (maximum >= 0 && value >= -1 && value <= maximum);
                valid_variation(usize::from(choice), variation)
                    && counter(lifetime, lifetime_max) && counter(fuse, fuse_max)
            }
            Self::Empty | Self::Unsupported => true,
        }
    }
}

/// Consume a present item field independently of its slot's other data. Older
/// saves omit it (the field's serde default is None). A damaged or unsupported
/// item becomes Unsupported instead of discarding every practice save slot.
pub fn deserialize_saved_item<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<HeldItemSnapshot>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    if value.is_null() {
        // A persisted `None` (JSON `null`) is an absent snapshot, not a damaged item.
        return Ok(None);
    }
    Ok(Some(serde_json::from_value::<HeldItemSnapshot>(value)
        .ok().filter(|item| item.valid()).unwrap_or(HeldItemSnapshot::Unsupported)))
}

impl Serialize for CharacterItemChoice {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (0..ITEM_NAMES.len()).map(|i| u8::from(self.0 == Some(i as u8))).collect::<Vec<_>>().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CharacterItemChoice {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let values = Vec::<u8>::deserialize(deserializer)?;
        if values.len() != ITEM_NAMES.len() || values.iter().any(|v| *v > 1)
            || values.iter().filter(|v| **v == 1).count() > 1 {
            return Err(serde::de::Error::custom("invalid character item choice"));
        }
        Ok(Self(values.iter().position(|v| *v == 1).map(|i| i as u8)))
    }
}

