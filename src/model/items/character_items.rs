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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    fn all_choices_round_trip_including_second_page() {
        for i in 0..ITEM_NAMES.len() {
            let choice = CharacterItemChoice(Some(i as u8));
            assert_eq!(serde_json::from_str::<CharacterItemChoice>(&serde_json::to_string(&choice).unwrap()).unwrap(), choice);
        }
        let none = CharacterItemChoice(None);
        assert_eq!(serde_json::from_str::<CharacterItemChoice>(&serde_json::to_string(&none).unwrap()).unwrap(), none);
    }
    fn malformed_or_multiple_choices_are_rejected() {
        assert!(serde_json::from_str::<CharacterItemChoice>("[]").is_err());
        let mut values = vec![0; ITEM_NAMES.len()];
        values[0] = 1; values[37] = 1;
        assert!(serde_json::from_str::<CharacterItemChoice>(&serde_json::to_string(&values).unwrap()).is_err());
    }

    fn saved_variant_is_validated_independently_of_the_current_menu() {
        let stitch = HeldItemSnapshot::Item { choice: 25, variation: 7,
            lifetime: 0, lifetime_max: 0, fuse: -1, fuse_max: 0 };
        assert!(stitch.valid());
        assert_eq!(serde_json::from_str::<HeldItemSnapshot>(&serde_json::to_string(&stitch).unwrap()).unwrap(), stitch);
        assert!(!valid_variation(25, 0));
        assert!(!valid_variation(255, 0));
        assert!(valid_variation(29, 7));
        assert!(!valid_variation(29, 8));
        let invalid = HeldItemSnapshot::Item { choice: 11, variation: 0,
            lifetime: -1, lifetime_max: -1, fuse: 1801, fuse_max: 1800 };
        assert!(!invalid.valid());
        assert_ne!(HeldItemSnapshot::Empty, HeldItemSnapshot::Unsupported);
    }

    fn all_native_turnip_faces_keep_exact_saved_identity() {
        for base in [2u8, 22u8] {
            for face in 0..=7 {
                let choice = base + match face { 5 => 1, 6 => 2, 7 => 3, _ => 0 };
                let item = HeldItemSnapshot::Item { choice, variation: face,
                    lifetime: 0, lifetime_max: 0, fuse: -1, fuse_max: 0 };
                assert!(item.valid());
                let json = serde_json::to_string(&item).unwrap();
                assert_eq!(serde_json::from_str::<HeldItemSnapshot>(&json).unwrap(), item);
            }
            assert!(!valid_variation(usize::from(base), -1));
            assert!(!valid_variation(usize::from(base), 5));
            assert!(!valid_variation(usize::from(base + 1), 1));
            assert!(!valid_variation(usize::from(base + 3), 8));
        }
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Slot {
        marker: u32,
        #[serde(default, deserialize_with = "deserialize_saved_item")]
        held_item: Option<HeldItemSnapshot>,
    }
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Slots { player: Vec<Slot> }

    fn persisted_slots_keep_legacy_empty_unsupported_and_exact_item_distinct() {
        let saved = Slots { player: vec![
            Slot { marker: 11, held_item: None },
            Slot { marker: 22, held_item: Some(HeldItemSnapshot::Empty) },
            Slot { marker: 33, held_item: Some(HeldItemSnapshot::Unsupported) },
            Slot { marker: 44, held_item: Some(HeldItemSnapshot::Item {
                choice: 25, variation: 7, lifetime: 500, lifetime_max: 1400,
                fuse: -1, fuse_max: 0 }) },
        ] };
        let json = serde_json::to_string_pretty(&saved).unwrap();
        assert_eq!(serde_json::from_str::<Slots>(&json).unwrap(), saved);
    }

    fn damaged_json_item_fields_do_not_discard_adjacent_slots() {
        // `save_states.json` uses this field deserializer.
        for damaged in [
            r#""damaged""#,
            r#"{"state": "unknown"}"#,
            r#"{"state": "item", "choice": 25, "variation": 0, "lifetime": 0, "lifetime_max": 0, "fuse": -1, "fuse_max": 0}"#,
            r#"{"state": "item", "choice": 11, "variation": 0, "lifetime": -1, "lifetime_max": -1, "fuse": 1801, "fuse_max": 1800}"#,
            r#"{"state": "item", "choice": 999}"#,
        ] {
            let file = format!(r#"{{"player": [{{"marker": 11}}, {{"marker": 22, "held_item": {damaged}}}, {{"marker": 33, "held_item": {{"state": "empty"}}}}]}}"#);
            let loaded: Slots = serde_json::from_str(&file).unwrap();
            assert_eq!(loaded.player, vec![
                Slot { marker: 11, held_item: None },
                Slot { marker: 22, held_item: Some(HeldItemSnapshot::Unsupported) },
                Slot { marker: 33, held_item: Some(HeldItemSnapshot::Empty) },
            ]);
        }
        // serde_json writes a `None` snapshot as `null`; that is an absent snapshot, never Unsupported.
        let loaded: Slots = serde_json::from_str(r#"{"player": [{"marker": 1, "held_item": null}]}"#).unwrap();
        assert_eq!(loaded.player, vec![Slot { marker: 1, held_item: None }]);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("all_choices_round_trip_including_second_page", all_choices_round_trip_including_second_page as fn()),
            ("malformed_or_multiple_choices_are_rejected", malformed_or_multiple_choices_are_rejected as fn()),
            ("saved_variant_is_validated_independently_of_the_current_menu", saved_variant_is_validated_independently_of_the_current_menu as fn()),
            ("all_native_turnip_faces_keep_exact_saved_identity", all_native_turnip_faces_keep_exact_saved_identity as fn()),
            ("persisted_slots_keep_legacy_empty_unsupported_and_exact_item_distinct", persisted_slots_keep_legacy_empty_unsupported_and_exact_item_distinct as fn()),
            ("damaged_json_item_fields_do_not_discard_adjacent_slots", damaged_json_item_fields_do_not_discard_adjacent_slots as fn()),
        ]);
    }
    }
}
