//! The waiting room's practice settings persist (rc.6, task `TASK-OPUS-rc6-lobby-cpu-settings.md` requirement 1): the
//! game's own Training Partners and Training Partner Behavior, saved when the player changes them and applied on the next
//! entry into a waiting room (local wireless or Quickplay). Pure half; the hooks are `runtime::lobby::lobby_persist`.
//!
//! The values are the game's own encoding (13.0.5, the waiting room's practice panel): partners `0..=3`, or
//! [`AUTOMATIC`]; behavior [`ATTACK`] or [`STOP`]. Nothing saved leaves the game's defaults (Automatic, Attack).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

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

/// The writer thread stored `pair` (`None`: nothing was due). `due` is cleared first and set again when the save hook
/// published a newer pair into `saved` meanwhile: checking before clearing lost a change saved between the two (the
/// thread-safety audit of 2026-11-12, item 1). `between` runs after the clear, the one point a save can land inside this.
pub fn clear_stored(due: &AtomicBool, saved: &AtomicU32, pair: Option<Saved>, between: impl FnOnce()) {
    if pair.is_some() && due.swap(false, Ordering::AcqRel) {
        between();
        if pair != Some(Saved::unpack(saved.load(Ordering::Acquire))) {
            due.store(true, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn a_saved_pair_round_trips_through_the_settings_key() {
        let saved = Saved { partners: Some(2), behavior: Some(STOP) };
        let value = saved.to_value().expect("a saved pair is stored");
        assert_eq!(value, serde_json::json!({"partners": 2, "behavior": 1}));
        assert_eq!(Saved::from_value(Some(&value)), saved);
        let half = Saved { partners: None, behavior: Some(ATTACK) };
        assert_eq!(Saved::from_value(half.to_value().as_ref()), half);
        assert_eq!(Saved::NONE.to_value(), None, "nothing saved writes no key");
        for saved in [saved, half, Saved::NONE, Saved { partners: Some(AUTOMATIC), behavior: None }] {
            assert_eq!(Saved::unpack(saved.pack()), saved);
        }
    }

    fn chosen_maps_the_row_handler_to_the_panel_values() {
        let room = [AUTOMATIC, 1, 2, 3];
        assert_eq!(chosen(PARTNERS_ROW, 0, &room), Some(Change::Partners(AUTOMATIC)));
        assert_eq!(chosen(PARTNERS_ROW, 2, &room), Some(Change::Partners(2)));
        assert_eq!(chosen(PARTNERS_ROW, 4, &room), None, "past the offered values");
        assert_eq!(chosen(BEHAVIOR_ROW, 0, &room), Some(Change::Behavior(ATTACK)));
        assert_eq!(chosen(BEHAVIOR_ROW, 1, &room), Some(Change::Behavior(STOP)));
        assert_eq!(chosen(BEHAVIOR_ROW, 2, &room), None);
        assert_eq!(chosen(2, 0, &room), None, "the panel has two rows");
        assert_eq!(chosen(PARTNERS_ROW, 0, &[0, 1, 2, 3, AUTOMATIC, 5]), None, "not the panel's vector");
        assert_eq!(chosen(PARTNERS_ROW, 0, &[7]), None, "a value the game does not use");
    }

    fn restore_applies_only_what_the_room_offers() {
        let saved = Saved { partners: Some(3), behavior: Some(STOP) };
        // A room with two free slots offers Automatic and 1..=3 (FUN_013cdd00 `(1,1,1,1,0)`).
        assert_eq!(restore(saved, &[AUTOMATIC, 1, 2, 3]),
            [Some(Apply { row: PARTNERS_ROW, index: 3, value: 3 }), Some(Apply { row: BEHAVIOR_ROW, index: 1, value: STOP })]);
        // A fuller room does not offer 3: the game's choice stays, the behavior still applies.
        assert_eq!(restore(saved, &[AUTOMATIC, 1]), [None, Some(Apply { row: BEHAVIOR_ROW, index: 1, value: STOP })]);
        assert_eq!(restore(Saved { partners: Some(AUTOMATIC), behavior: Some(ATTACK) }, &[0, AUTOMATIC]),
            [Some(Apply { row: PARTNERS_ROW, index: 1, value: AUTOMATIC }), Some(Apply { row: BEHAVIOR_ROW, index: 0, value: ATTACK })]);
        assert_eq!(restore(saved, &[0, 1, 2, 3, AUTOMATIC, 5])[0], None, "not the panel's vector");
    }

    fn a_change_updates_only_its_own_row() {
        let saved = Saved::NONE.with(Change::Partners(1));
        assert_eq!(saved, Saved { partners: Some(1), behavior: None });
        let saved = saved.with(Change::Behavior(STOP));
        assert_eq!(saved, Saved { partners: Some(1), behavior: Some(STOP) });
        assert_eq!(saved.with(Change::Partners(AUTOMATIC)), Saved { partners: Some(AUTOMATIC), behavior: Some(STOP) });
        assert_eq!(saved.with(Change::Partners(1)), saved, "the same value is no change");
    }

    /// The save hook's two stores (`runtime::lobby::lobby_persist::save`): the pair, then the store is due.
    fn hook_save(due: &AtomicBool, saved: &AtomicU32, next: Saved) {
        saved.store(next.pack(), Ordering::Release);
        due.store(true, Ordering::Release);
    }

    /// The writer's read (`runtime::lobby::lobby_persist::store_due`).
    fn store_due(due: &AtomicBool, saved: &AtomicU32) -> Option<Saved> {
        due.load(Ordering::Acquire).then(|| Saved::unpack(saved.load(Ordering::Acquire)))
    }

    fn a_change_saved_while_the_writer_clears_stays_due() {
        let (due, saved) = (AtomicBool::new(false), AtomicU32::new(Saved::NONE.pack()));
        let first = Saved { partners: Some(1), behavior: None };
        let newer = first.with(Change::Behavior(STOP));
        hook_save(&due, &saved, first);
        let pair = store_due(&due, &saved);
        assert_eq!(pair, Some(first));
        // The hook saves inside the writer's clear (after it, before the re-check): the newer pair stays due.
        clear_stored(&due, &saved, pair, || hook_save(&due, &saved, newer));
        assert_eq!(store_due(&due, &saved), Some(newer), "the newer pair stays due");
        // The hook saves after the writer read the pair and before its clear (where the check-then-clear lost it).
        let pair = store_due(&due, &saved);
        let newest = newer.with(Change::Partners(3));
        hook_save(&due, &saved, newest);
        clear_stored(&due, &saved, pair, || {});
        assert_eq!(store_due(&due, &saved), Some(newest), "a pair saved before the clear stays due");
        // Nothing newer: the store is done.
        clear_stored(&due, &saved, Some(newest), || {});
        assert_eq!(store_due(&due, &saved), None);
        // Nothing was due: no clear, and `between` does not run.
        clear_stored(&due, &saved, None, || hook_save(&due, &saved, first));
        assert_eq!(store_due(&due, &saved), None);
    }

    fn the_writer_ends_on_the_last_saved_pair() {
        // Two real threads: the hook saves 20,000 changes while the writer stores and clears. Once the hook has stopped
        // the writer's last stored pair is the last one saved (a lost `due` would leave it on an older pair).
        let (due, saved, done) = (AtomicBool::new(false), AtomicU32::new(Saved::NONE.pack()), AtomicBool::new(false));
        let last = std::thread::scope(|scope| {
            scope.spawn(|| {
                for i in 0..20_000u32 {
                    let behavior = if i % 2 == 0 { ATTACK } else { STOP };
                    hook_save(&due, &saved, Saved { partners: Some(i % 4), behavior: Some(behavior) });
                }
                done.store(true, Ordering::Release);
            });
            let mut last = None;
            loop {
                let finished = done.load(Ordering::Acquire);
                let pair = store_due(&due, &saved);
                last = pair.or(last);
                clear_stored(&due, &saved, pair, || {});
                if finished && !due.load(Ordering::Acquire) {
                    break last;
                }
            }
        });
        assert_eq!(last, Some(Saved::unpack(saved.load(Ordering::Acquire))));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_saved_pair_round_trips_through_the_settings_key", a_saved_pair_round_trips_through_the_settings_key as fn()),
            ("chosen_maps_the_row_handler_to_the_panel_values", chosen_maps_the_row_handler_to_the_panel_values as fn()),
            ("restore_applies_only_what_the_room_offers", restore_applies_only_what_the_room_offers as fn()),
            ("a_change_updates_only_its_own_row", a_change_updates_only_its_own_row as fn()),
            ("a_change_saved_while_the_writer_clears_stays_due", a_change_saved_while_the_writer_clears_stays_due as fn()),
            ("the_writer_ends_on_the_last_saved_pair", the_writer_ends_on_the_last_saved_pair as fn()),
        ]);
    }
    }
}
