//! The held-item snapshot a save-state record carries (SPEC-state-04 §7) and the table of the fork's 38
//! item identities. Values only: never a native item pointer.
//!
//! Carried from the fork's `training_mod_consts/src/character_items.rs` (stage m5o, ours-new, see
//! CARRY-OVER.md): `ITEM_NAMES`, `valid_variation`, `HeldItemSnapshot` (renamed `HeldItem`) and its
//! `valid`, and the rule of `deserialize_saved_item` (a damaged item field becomes `Unsupported`; a JSON
//! `null` is absent). Clean-room changes: no serde derive (this crate reads and writes JSON by hand, so a
//! damaged field is handled where it is read), no menu code (`CharacterItemChoice`, `to_submenu`).

use serde_json::Value;

/// The fork's item identities, in the order of its 38-item list (`choice` indexes this).
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

/// What the player's fighter held when the state was saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldItem {
    Empty,
    Unsupported,
    Item {
        /// Index into [`ITEM_NAMES`].
        choice: u8,
        /// Logical native variant: turnips store the actual face index 0..7; other items store
        /// `ITEM_INSTANCE_WORK_INT_VARIATION`.
        variation: i32,
        lifetime: i32,
        lifetime_max: i32,
        fuse: i32,
        fuse_max: i32,
    },
}

/// Whether `variation` is a real variant of item `choice`.
pub fn valid_variation(choice: usize, variation: i32) -> bool {
    match choice {
        // The Smile choice also identifies the four unlisted common native faces in saved data.
        2 | 22 => (0..=4).contains(&variation),
        3 | 23 => variation == 5,
        4 | 24 => variation == 6,
        5 | 25 => variation == 7,
        29 => (0..8).contains(&variation),
        _ => choice < ITEM_NAMES.len() && variation == 0,
    }
}

impl HeldItem {
    pub fn valid(self) -> bool {
        match self {
            Self::Item { choice, variation, lifetime, lifetime_max, fuse, fuse_max } => {
                let counter = |value: i32, maximum: i32| {
                    (value == -1 && maximum == -1) || (maximum >= 0 && value >= -1 && value <= maximum)
                };
                valid_variation(usize::from(choice), variation)
                    && counter(lifetime, lifetime_max)
                    && counter(fuse, fuse_max)
            }
            Self::Empty | Self::Unsupported => true,
        }
    }

    /// Read a present `held_item` field: `null` is absent; anything damaged or unsupported becomes
    /// `Unsupported` (one bad item never discards a slot).
    pub fn from_json(value: &Value) -> Option<Self> {
        if value.is_null() {
            return None;
        }
        Some(Self::parse(value).filter(|item| item.valid()).unwrap_or(Self::Unsupported))
    }

    fn parse(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        match object.get("state")?.as_str()? {
            "empty" => Some(Self::Empty),
            "unsupported" => Some(Self::Unsupported),
            "item" => {
                let int = |key: &str| object.get(key).and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok());
                Some(Self::Item {
                    choice: u8::try_from(object.get("choice")?.as_u64()?).ok()?,
                    variation: int("variation")?,
                    lifetime: int("lifetime")?,
                    lifetime_max: int("lifetime_max")?,
                    fuse: int("fuse")?,
                    fuse_max: int("fuse_max")?,
                })
            }
            _ => None,
        }
    }

    /// The snapshot's JSON in the fork's field order (`state` first, then the item's fields).
    pub fn write_json(self, out: &mut super::json::Writer) {
        match self {
            Self::Empty | Self::Unsupported => {
                out.begin_object();
                out.field_str("state", if self == Self::Empty { "empty" } else { "unsupported" });
                out.end_object();
            }
            Self::Item { choice, variation, lifetime, lifetime_max, fuse, fuse_max } => {
                out.begin_object();
                out.field_str("state", "item");
                out.field_int("choice", i64::from(choice));
                out.field_int("variation", i64::from(variation));
                out.field_int("lifetime", i64::from(lifetime));
                out.field_int("lifetime_max", i64::from(lifetime_max));
                out.field_int("fuse", i64::from(fuse));
                out.field_int("fuse_max", i64::from(fuse_max));
                out.end_object();
            }
        }
    }
}
