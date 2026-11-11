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

/// How many created entries (7/6/5/4 in Training) a helper plan of `fighters` fighter targets and `hats` hat-only
/// targets may use, of the `created` the scene supports, beside `seats` native helpers. Training (§5.1): the native
/// helpers hold fighter targets only (entry 3 a hat only as the borrowed last one), so every other target needs a
/// created entry. The waiting rooms keep one view: the helpers take any target first.
pub fn created_allowance(fighters: usize, hats: usize, seats: usize, created: usize, training: bool) -> usize {
    let needed = if training { fighters.saturating_sub(seats) + hats } else { (fighters + hats).saturating_sub(seats) };
    needed.min(created)
}

/// §5.1, entry 3's borrow: "the created entries already hold or are loading plan targets". `in_use` lists the created
/// entries in creation order, each true when it exists and holds or loads a current plan target; the first
/// `allowance` ([`created_allowance`], less any a refused creation ended) must all be. With no created entry allowed
/// the borrow is the only place left.
pub fn created_busy(in_use: impl IntoIterator<Item = bool>, allowance: usize) -> bool {
    in_use.into_iter().take(allowance).filter(|used| *used).count() == allowance
}

/// Fill what exists before creating: an existing helper (or an earlier created entry) holds the next created entry
/// back while its `view` admits a plan target and it does not hold a Ready one (`filled`, [`creation_filled`]). An entry
/// whose view admits nothing can never be filled by this plan (entries 2 and 3 under a hats-only plan, unless parked
/// as F9 seats: then they hold their seat members).
pub fn holds_back_creation<T, const N: usize>(view: [Option<T>; N], filled: bool) -> bool {
    !filled && view.iter().any(Option::is_some)
}

/// [`holds_back_creation`]'s `filled` for an existing helper or an earlier created entry: it holds a `ready` source
/// that the plan lists (`in_plan`) or that may not be replaced. Primary 2026-11-09 09:47Z (Eden final-xps-hot hs06): a
/// parked F9 seat (entry 2 or 3) holding its own seat member (`own_seat_member`) is filled as well. F9 reading A keeps
/// that member whether or not the plan lists it, so under a hats-only plan the seat's view admits it while the plan
/// never does; without this the created entries are held back for good (P-HOT 5.1 undone).
pub fn creation_filled(ready: bool, in_plan: bool, replaceable: bool, own_seat_member: bool) -> bool {
    ready && (in_plan || !replaceable || own_seat_member)
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

/// F9-SEAT-NOTICE (primary 2026-11-08 23:03Z, reading A): the fighter status with Training's parked seats counted (entry
/// 2 and entry 3 at one CPU, `parked_entries::seat_state`). Each one is one more planned fighter; a ready seat is ready,
/// a held one waits for memory (the notice's "… waiting for memory"), and any other is pending. A live seat (2 or 3 CPUs)
/// or an entry without a member is `None` and adds nothing, and so do the seats while the fighter status itself is hidden
/// (nothing planned): they never raise a notice of their own. No new text.
pub fn with_seats(
    mut fighters: super::cycle_preload_policy::BuiltFighters,
    seats: [Option<crate::model::parked_entries::SeatState>; 2],
) -> super::cycle_preload_policy::BuiltFighters {
    use crate::engine::swap::body_bank::{merge_hold, Hold};
    use crate::model::parked_entries::SeatState;
    if fighters.planned == 0 {
        return fighters;
    }
    for seat in seats.into_iter().flatten() {
        fighters.planned = fighters.planned.saturating_add(1);
        match seat {
            SeatState::Ready => fighters.ready = fighters.ready.saturating_add(1),
            SeatState::Loading => fighters.pending = fighters.pending.saturating_add(1),
            SeatState::HeldMemory => {
                fighters.held = fighters.held.saturating_add(1);
                fighters.hold = merge_hold(fighters.hold, Some(Hold::Memory));
            }
        }
    }
    fighters
}

