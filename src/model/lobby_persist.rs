//! The waiting room's practice settings persist (rc.6, task `TASK-OPUS-rc6-lobby-cpu-settings.md` requirement 1): the
//! game's own Training Partners and Training Partner Behavior, saved when the player changes them and applied on the next
//! entry into a waiting room (local wireless or Quickplay). Pure half; the hooks are `runtime::lobby::lobby_persist`.
//!
//! The values are the game's own encoding (13.0.5, the waiting room's practice panel): partners `0..=3`, or
//! [`AUTOMATIC`]; behavior [`ATTACK`] or [`STOP`]. Nothing saved leaves the game's defaults (Automatic, Attack).

use serde_json::{Map, Value};

/// Training Partners: Automatic (the panel's `+0x420`; `0..=3` is that many partners).
pub const AUTOMATIC: u32 = 4;
/// Training Partner Behavior: Attack, the game's default (the panel's `+0x428`).
pub const ATTACK: u32 = 7;
/// Training Partner Behavior: Stop.
pub const STOP: u32 = 1;
/// The top-level `settings.json` key the pair is stored under, beside the mod's other settings.
pub const KEY: &str = "lobby_practice_panel";
/// The row handler's row numbers (`FUN_01bc83a0`'s `*row`).
pub const PARTNERS_ROW: i32 = 0;
pub const BEHAVIOR_ROW: i32 = 1;
/// The most partner values a room offers (`0..=3` and Automatic); a longer vector is not the panel's.
pub const MAX_CHOICES: usize = 5;

/// The saved pair; `None` is "not saved".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Saved {
    pub partners: Option<u32>,
    pub behavior: Option<u32>,
}

/// One row's value as the player chose it (`chosen`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Partners(u32),
    Behavior(u32),
}

/// What an entry applies to one row: the row, the index the row handler takes, the value it writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Apply {
    pub row: i32,
    pub index: u32,
    pub value: u32,
}

/// A partner value the game uses.
pub const fn valid_partners(value: u32) -> bool {
    value <= AUTOMATIC
}

/// A behavior value the game uses.
pub const fn valid_behavior(value: u32) -> bool {
    value == ATTACK || value == STOP
}

/// One stored field: a small non-negative integer the game uses for that row.
fn field(object: &Map<String, Value>, name: &str, valid: fn(u32) -> bool) -> Option<u32> {
    let value = u32::try_from(object.get(name)?.as_u64()?).ok()?;
    valid(value).then_some(value)
}

impl Saved {
    pub const NONE: Saved = Saved { partners: None, behavior: None };

    /// The stored key's value (`{"partners": n, "behavior": m}`): a field the game does not use reads as not saved.
    pub fn from_value(value: Option<&Value>) -> Saved {
        let Some(object) = value.and_then(Value::as_object) else { return Saved::NONE };
        Saved { partners: field(object, "partners", valid_partners), behavior: field(object, "behavior", valid_behavior) }
    }

    /// The key's value to store; `None` when nothing is saved.
    pub fn to_value(self) -> Option<Value> {
        let mut object = Map::new();
        if let Some(partners) = self.partners {
            object.insert("partners".to_owned(), Value::from(partners));
        }
        if let Some(behavior) = self.behavior {
            object.insert("behavior".to_owned(), Value::from(behavior));
        }
        (!object.is_empty()).then_some(Value::Object(object))
    }

    /// The pair after the player's change.
    pub fn with(self, change: Change) -> Saved {
        match change {
            Change::Partners(value) => Saved { partners: Some(value), ..self },
            Change::Behavior(value) => Saved { behavior: Some(value), ..self },
        }
    }

    /// One word for the game thread (`0xff` per field: not saved).
    pub const fn pack(self) -> u32 {
        let partners = match self.partners {
            Some(value) if valid_partners(value) => value,
            _ => 0xff,
        };
        let behavior = match self.behavior {
            Some(value) if valid_behavior(value) => value,
            _ => 0xff,
        };
        partners | behavior << 8
    }

    pub const fn unpack(word: u32) -> Saved {
        let (partners, behavior) = (word & 0xff, word >> 8 & 0xff);
        Saved {
            partners: if valid_partners(partners) { Some(partners) } else { None },
            behavior: if valid_behavior(behavior) { Some(behavior) } else { None },
        }
    }
}

/// The panel's offered partner values, when they are the panel's (at most [`MAX_CHOICES`], each one the game uses).
fn offered(choices: &[u32]) -> bool {
    choices.len() <= MAX_CHOICES && choices.iter().all(|value| valid_partners(*value))
}

/// The value a row handler call gives the panel: row 0 takes the offered value at `index`, row 1 maps index 0 to
/// Attack and 1 to Stop. `None` for any other row or index.
pub fn chosen(row: i32, index: u32, choices: &[u32]) -> Option<Change> {
    match (row, index) {
        (PARTNERS_ROW, _) if offered(choices) => choices.get(index as usize).copied().map(Change::Partners),
        (BEHAVIOR_ROW, 0) => Some(Change::Behavior(ATTACK)),
        (BEHAVIOR_ROW, 1) => Some(Change::Behavior(STOP)),
        _ => None,
    }
}

/// What an entry applies: the saved partner count only when this room offers it (its index in `choices`), the saved
/// behavior always. Nothing for what is not saved.
pub fn restore(saved: Saved, choices: &[u32]) -> [Option<Apply>; 2] {
    let partners = saved.partners.filter(|_| offered(choices)).and_then(|value| {
        choices.iter().position(|offer| *offer == value).map(|index| Apply { row: PARTNERS_ROW, index: index as u32, value })
    });
    let behavior = saved.behavior.and_then(|value| match value {
        ATTACK => Some(Apply { row: BEHAVIOR_ROW, index: 0, value }),
        STOP => Some(Apply { row: BEHAVIOR_ROW, index: 1, value }),
        _ => None,
    });
    [partners, behavior]
}

