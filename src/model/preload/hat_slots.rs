//! P-HOT helper entries by purpose (`artifacts/phase23/LANE-HOT-SLOTS-REPORT.md` §3.2, §3.5.1, §5.1). Values only.
//!
//! - The helper plan keeps `merge`'s order. Hats are budgeted apart from the fighters ("Ready Hats", not charged
//!   against Ready Fighters). A hat whose kind a fighter lane wants too is one fighter target: the shared body.
//! - Training's helper entries by purpose: the created entries 7/6/5/4 take hat-only targets first; entry 2 holds
//!   fighter targets only (F9's seat for the CPU rotation's next fighter); entry 3 holds fighter targets, and a
//!   hat-only target only through the borrow: the fifth hat, while the native CPU count is 1, the pause is closed,
//!   entry 3 is replaceable and the created entries are all in use (operator decision "B").
//! - The borrow record is what F9 reads (operator decision "1"): while it exists entry 3 holds only the borrowed hat;
//!   F9 clears it when it retired or promoted the entry; a new scene clears it.

/// Entry 2: F9's seat for the CPU rotation's next fighter. Never a hat-only target.
pub const SEAT_ENTRY: usize = 2;
/// Entry 3: F9's seat for the fighter after next; a hat-only target only through the borrow.
pub const BORROW_ENTRY: usize = 3;
/// Ready Hats: 0..=5, default 5 (DECISIONS 2026-11-02: "a separate budget of five hot Kirby hats").
pub const READY_HATS_MAX: u8 = 5;
pub const READY_HATS_DEFAULT: u8 = 5;

use crate::engine::compat::choice::byteflags;
use serde::Deserialize;

// The "Ready Hats" row (Fighters & Stages > Loading): one choice per hat count, 0 first.
byteflags! {
    pub struct CyclePreloadReadyHats {
        pub ZERO = "0",
        pub ONE = "1",
        pub TWO = "2",
        pub THREE = "3",
        pub FOUR = "4",
        pub FIVE = "5",
    }
}

const _: () = assert!(CyclePreloadReadyHats::ALL_CONSTS.len() == READY_HATS_MAX as usize + 1);

/// Built-in menu selection: five hats.
pub const fn default_ready_hats() -> CyclePreloadReadyHats {
    CyclePreloadReadyHats::ALL_CONSTS[READY_HATS_DEFAULT as usize]
}

/// The hat budget a selection means; a selection that is not exactly one choice is the built-in five.
pub fn ready_hats(choice: CyclePreloadReadyHats) -> u8 {
    CyclePreloadReadyHats::ALL_CONSTS.iter().position(|c| *c == choice).map_or(READY_HATS_DEFAULT, |i| i as u8)
}

/// A menu file without the key, or with a list that is not exactly one choice, keeps the built-in five.
pub fn deserialize_ready_hats<'de, D>(deserializer: D) -> Result<CyclePreloadReadyHats, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<u8>::deserialize(deserializer).map_err(serde::de::Error::custom)?;
    let one = values.len() == CyclePreloadReadyHats::ALL_CONSTS.len()
        && values.iter().all(|v| *v <= 1)
        && values.iter().filter(|v| **v == 1).count() == 1;
    Ok(if one {
        CyclePreloadReadyHats::ALL_CONSTS[values.iter().position(|v| *v == 1).unwrap_or(0)]
    } else {
        default_ready_hats()
    })
}

/// Why a target is in the helper plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// A fighter lane wants it (a hat of the same kind shares it).
    Fighter,
    /// Only a hat lane wants it.
    Hat,
}

/// `order` is the merged plan (`cycle_preload_policy::merge` or `merge_player_hats_first`); `fighter` tells whether
/// a fighter lane wants a target. Keeps the order, at most `hats` hat-only targets and `fighters` fighter targets.
pub fn budgeted<T: Copy, const N: usize>(
    order: [Option<T>; N],
    fighter: impl Fn(T) -> bool,
    hats: u8,
    fighters: u8,
) -> [Option<(T, Purpose)>; N] {
    let mut result = [None; N];
    let (mut count, mut hat_count, mut fighter_count) = (0, 0u8, 0u8);
    for target in order.into_iter().flatten() {
        let purpose = if fighter(target) { Purpose::Fighter } else { Purpose::Hat };
        let (used, cap) = match purpose {
            Purpose::Fighter => (&mut fighter_count, fighters),
            Purpose::Hat => (&mut hat_count, hats),
        };
        if *used >= cap {
            continue;
        }
        *used += 1;
        result[count] = Some((target, purpose));
        count += 1;
    }
    result
}

/// Facts of one helper entry at one admission, read by the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotFacts<T> {
    /// Native Training (the waiting rooms keep one ordered view: entries 2/3 are not F9 seats there).
    pub training: bool,
    pub slot: usize,
    /// One of the entries the plugin creates (7/6/5/4 in Training).
    pub created: bool,
    /// Every created entry holds or loads a current plan target (the borrow is for the fifth hat only).
    pub created_busy: bool,
    /// The borrow's admission ([`borrow_admitted`]) holds now.
    pub borrow_open: bool,
    /// The borrow record's target, when entry 3 is borrowed.
    pub borrowed: Option<T>,
}

/// The targets this entry may load, in the order it should prefer them.
pub fn slot_view<T: Copy + PartialEq, const N: usize>(plan: [Option<(T, Purpose)>; N], facts: SlotFacts<T>) -> [Option<T>; N] {
    let mut result = [None; N];
    let mut count = 0;
    let mut push = |target: T| {
        if count < N {
            result[count] = Some(target);
            count += 1;
        }
    };
    let of = |purpose: Purpose| plan.into_iter().flatten().filter(move |(_, p)| *p == purpose).map(|(t, _)| t);
    if !facts.training {
        plan.into_iter().flatten().for_each(|(t, _)| push(t));
    } else if facts.slot == BORROW_ENTRY && facts.borrowed.is_some() {
        // While the record exists entry 3 holds the borrowed hat and nothing else.
        if let Some(borrowed) = facts.borrowed.filter(|b| plan.into_iter().flatten().any(|(t, _)| t == *b)) {
            push(borrowed);
        }
    } else if facts.created {
        of(Purpose::Hat).chain(of(Purpose::Fighter)).for_each(&mut push);
    } else if facts.slot == SEAT_ENTRY {
        of(Purpose::Fighter).for_each(&mut push);
    } else if facts.slot == BORROW_ENTRY {
        of(Purpose::Fighter).for_each(&mut push);
        if facts.borrow_open && facts.created_busy {
            of(Purpose::Hat).for_each(&mut push);
        }
    }
    result
}

/// §3.5.1 rule 1 (operator decisions "B" and "1"): entry 3 may be borrowed by a hat only while the native CPU count
/// is 1, the pause is closed and entry 3 is replaceable. No hold or hook is needed.
pub fn borrow_admitted(cpu_count: Option<u32>, pause_open: bool, replaceable: bool) -> bool {
    cpu_count == Some(1) && !pause_open && replaceable
}

/// The borrow record F9 reads (§3.5.1 rule 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Borrow<T, E> {
    pub kind: T,
    /// The scene epoch the borrow belongs to.
    pub epoch: E,
    /// When the hat's load was admitted on entry 3 (the runtime's clock).
    pub since: u64,
}

/// How a borrow ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// F9 loaded entry 3 with its seat member after the activation walk (a 1 -> 2 raise).
    Retired,
    /// F9 promoted the hat body as itself, then swapped it (a 1 -> 3 raise in one pause close).
    Promoted,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BorrowBook<T, E> {
    record: Option<Borrow<T, E>>,
}

impl<T: Copy + PartialEq, E: Copy + PartialEq> BorrowBook<T, E> {
    pub const fn new() -> Self {
        Self { record: None }
    }

    /// The record F9 reads: entry 3 holds (or loads) this hat and no CPU rotation fighter.
    pub fn current(&self) -> Option<Borrow<T, E>> {
        self.record
    }

    /// The hat's load was admitted on entry 3. False (nothing changes) when a borrow already exists.
    pub fn publish(&mut self, kind: T, epoch: E, since: u64) -> bool {
        if self.record.is_some() {
            return false;
        }
        self.record = Some(Borrow { kind, epoch, since });
        true
    }

    /// F9 retired or promoted entry 3. Returns the ended record.
    pub fn release(&mut self, _how: Release) -> Option<Borrow<T, E>> {
        self.record.take()
    }

    /// A new scene: a record of another epoch ends with its scene.
    pub fn departed(&mut self, epoch: E) {
        if self.record.is_some_and(|record| record.epoch != epoch) {
            self.record = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HAT: Purpose = Purpose::Hat;
    const FIGHTER: Purpose = Purpose::Fighter;

    fn facts(slot: usize, created: bool, created_busy: bool, borrow_open: bool) -> SlotFacts<u32> {
        SlotFacts { training: true, slot, created, created_busy, borrow_open, borrowed: None }
    }

    /// Five hats (kinds 0x18, 0x1a, 0x13, 0x14, 0x16) and one fighter look-ahead (2) in `merge` order.
    fn plan() -> [Option<(u32, Purpose)>; 6] {
        [Some((0x18, HAT)), Some((0x2, FIGHTER)), Some((0x1a, HAT)), Some((0x13, HAT)), Some((0x14, HAT)), Some((0x16, HAT))]
    }

    #[test]
    fn hats_are_budgeted_apart_from_fighters_and_a_shared_kind_is_one_fighter_target() {
        // Lanes: hats 0x18, 0x2 (also the CPU's next fighter), 0x1a; fighters 0x2, 0x3.
        let order = [Some(0x18u32), Some(0x2), Some(0x1a), Some(0x3), None];
        let fighter = |kind: u32| kind == 0x2 || kind == 0x3;
        assert_eq!(budgeted(order, fighter, 5, 5),
            [Some((0x18, HAT)), Some((0x2, FIGHTER)), Some((0x1a, HAT)), Some((0x3, FIGHTER)), None]);
        // The fighter budget spent elsewhere (Ready Fighters by windows and groups) leaves every hat.
        assert_eq!(budgeted(order, fighter, 5, 0), [Some((0x18, HAT)), Some((0x1a, HAT)), None, None, None]);
        // Ready Hats 1: the first hat only; the fighters keep their own budget.
        assert_eq!(budgeted(order, fighter, 1, 5), [Some((0x18, HAT)), Some((0x2, FIGHTER)), Some((0x3, FIGHTER)), None, None]);
        assert_eq!(READY_HATS_DEFAULT, 5);
    }

    /// Operator decision "B": the created entries take the hats first; entry 2 never takes a hat-only target; entry 3
    /// takes one only through the borrow, which needs one native CPU, a closed pause, a replaceable entry 3 and every
    /// created entry in use.
    #[test]
    fn a_fifth_hat_borrows_entry_3_only_with_one_cpu() {
        let plan = plan();
        assert_eq!(slot_view(plan, facts(7, true, false, false))[..6],
            [Some(0x18), Some(0x1a), Some(0x13), Some(0x14), Some(0x16), Some(0x2)], "created: hats first");
        assert_eq!(slot_view(plan, facts(SEAT_ENTRY, false, true, true))[..2], [Some(0x2), None], "entry 2: fighters only");
        assert_eq!(slot_view(plan, facts(BORROW_ENTRY, false, false, true))[..2], [Some(0x2), None],
            "entry 3 before the created entries are all in use: fighters only");
        assert_eq!(slot_view(plan, facts(BORROW_ENTRY, false, true, true))[..3], [Some(0x2), Some(0x18), Some(0x1a)],
            "entry 3 with the borrow open: fighters, then the hats");
        assert_eq!(slot_view(plan, facts(BORROW_ENTRY, false, true, false))[..2], [Some(0x2), None], "borrow closed");
        assert!(borrow_admitted(Some(1), false, true));
        for (count, pause, replaceable) in [(Some(2), false, true), (Some(3), false, true), (None, false, true),
            (Some(1), true, true), (Some(1), false, false)] {
            assert!(!borrow_admitted(count, pause, replaceable), "{count:?} pause={pause} replaceable={replaceable}");
        }
        // The waiting rooms keep one view: entries 2/3 there are not F9 seats.
        let waiting = SlotFacts { training: false, ..facts(BORROW_ENTRY, false, false, false) };
        assert_eq!(slot_view(plan, waiting)[0], Some(0x18));
    }

    /// Operator decision "1": the record is published at the hat's admission on entry 3, read by F9, kept until F9
    /// retires or promotes the entry (or the scene ends), and while it exists entry 3 holds only the borrowed hat.
    #[test]
    fn a_borrowed_entry_3_carries_a_record_f9_reads() {
        let mut book = BorrowBook::<u32, u64>::new();
        assert_eq!(book.current(), None);
        assert!(book.publish(0x16, 7, 1000));
        assert!(!book.publish(0x18, 7, 2000), "one borrow at a time");
        assert_eq!(book.current(), Some(Borrow { kind: 0x16, epoch: 7, since: 1000 }));
        let borrowed = SlotFacts { borrowed: book.current().map(|b| b.kind), ..facts(BORROW_ENTRY, false, true, true) };
        assert_eq!(slot_view(plan(), borrowed)[..2], [Some(0x16), None], "entry 3 holds the borrowed hat only");
        let gone = SlotFacts { borrowed: Some(0x30), ..borrowed };
        assert_eq!(slot_view(plan(), gone)[0], None, "a borrowed hat out of the plan is not replaced by P-HOT");
        book.departed(7);
        assert!(book.current().is_some(), "the same scene keeps it");
        assert_eq!(book.release(Release::Retired).map(|b| b.kind), Some(0x16));
        assert_eq!(book.current(), None);
        assert!(book.publish(0x18, 7, 3000), "after the drop to 1 the fifth hat may borrow again");
        assert_eq!(book.release(Release::Promoted).map(|b| b.kind), Some(0x18));
        assert!(book.publish(0x18, 7, 4000));
        book.departed(8);
        assert_eq!(book.current(), None, "a new scene ends the borrow");
    }
}
