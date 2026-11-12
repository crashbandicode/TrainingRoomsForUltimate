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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    /// Primary 2026-11-08 23:03Z, F9-SEAT-NOTICE, reading A (cov-rest's question of 16:25Z): a seat held for memory
    /// showed only in a diag line, because the fighter status counted the plan's targets and the bank groups. The two
    /// seats count in the existing status, with no new text: a held seat is "waiting for memory".
    fn a_seat_held_for_memory_counts_in_the_fighter_status() {
        use crate::engine::preload::cycle_preload_policy::{BuiltFighters, CacheStatus, Progress, Readiness};
        use crate::engine::swap::body_bank::Hold;
        use crate::model::parked_entries::{seat_state, SeatState};
        let assets = Readiness { progress: Progress::Ready, ready: 0, planned: 0, configured: 10, limit: 10, failed: 0 };
        let ten = BuiltFighters { planned: 10, ready: 10, ..BuiltFighters::default() };
        let text = |seats| CacheStatus::new(assets, with_seats(ten, seats), false).text().to_string();
        let gunner = (0x4a, Some(7u8));
        let dk = (0x01, None::<u8>);
        assert_eq!(seat_state(gunner, Some(gunner), None), SeatState::Ready);
        assert_eq!(seat_state(dk, None, Some(0x01)), SeatState::HeldMemory, "refused for memory, and not loaded since");
        assert_eq!(seat_state(dk, Some((0x02, None)), None), SeatState::Loading, "another body is still on the entry");
        assert_eq!(seat_state(gunner, Some((0x4a, Some(8u8))), None), SeatState::Loading, "another Mii is not the seat");
        assert_eq!(text([Some(SeatState::Ready), Some(SeatState::Ready)]), "Ready 12/12");
        assert_eq!(text([Some(SeatState::Ready), Some(SeatState::HeldMemory)]), "Loaded 11/12 (1 waiting for memory)");
        assert_eq!(text([Some(SeatState::Ready), Some(SeatState::Loading)]), "Loading 11/12");
        assert_eq!(text([None, None]), "Ready 10/10", "no parked seat (2 or 3 CPUs, or no member): nothing added");
        let hidden = with_seats(BuiltFighters::default(), [Some(SeatState::HeldMemory), Some(SeatState::Ready)]);
        assert_eq!(hidden, BuiltFighters::default(), "the seats raise no notice of their own");
        let held = with_seats(BuiltFighters { held: 1, hold: Some(Hold::Pool), ..ten }, [Some(SeatState::HeldMemory), None]);
        assert_eq!((held.held, held.hold), (2, Some(Hold::Memory)), "memory is named first");
    }

    const HAT: Purpose = Purpose::Hat;
    const FIGHTER: Purpose = Purpose::Fighter;

    fn facts(slot: usize, created: bool, created_busy: bool, borrow_open: bool) -> SlotFacts<u32> {
        SlotFacts { training: true, slot, created, created_busy, borrow_open, borrowed: None }
    }

    /// Five hats (kinds 0x18, 0x1a, 0x13, 0x14, 0x16) and one fighter look-ahead (2) in `merge` order.
    fn plan() -> [Option<(u32, Purpose)>; 6] {
        [Some((0x18, HAT)), Some((0x2, FIGHTER)), Some((0x1a, HAT)), Some((0x13, HAT)), Some((0x14, HAT)), Some((0x16, HAT))]
    }

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

    /// rc6-hot-xps1 hs06 (2026-11-08, XPS): P1 Kirby with Ready Fighters 10 and groups on, so the windows and groups
    /// hold every fighter and the plan is five hats. No hat loaded in 40 s. Four created entries are allowed for five
    /// hats (the fifth borrows entry 3); entries 2 and 3, which may load none of the hats, do not hold them back; the
    /// borrow opens only once the allowed created entries are in use.
    fn a_hats_only_plan_creates_four_entries_before_the_borrow() {
        assert_eq!(created_allowance(0, 5, 2, 4, true), 4, "five hats: 7/6/5/4, the fifth borrows entry 3");
        assert_eq!(created_allowance(0, 3, 2, 4, true), 3);
        assert_eq!(created_allowance(1, 4, 2, 4, true), 4, "entry 2 holds the fighter");
        assert_eq!(created_allowance(2, 3, 2, 4, true), 3);
        assert_eq!(created_allowance(5, 0, 2, 4, true), 3, "fighters only: five targets less the two helpers, as before");
        assert_eq!(created_allowance(0, 5, 2, 1, true), 1, "one created entry supported");
        assert_eq!(created_allowance(3, 2, 2, 3, false), 3, "waiting rooms: one view, the helpers first");
        assert_eq!(created_allowance(1, 1, 2, 3, false), 0);

        let hats_only = [Some((0x14, HAT)), Some((0x10, HAT)), Some((0x11, HAT)), Some((0x18, HAT)), Some((0x1a, HAT)), None];
        let none_in_use = created_busy([false; 4], 4);
        assert!(!none_in_use, "no created entry in use yet: the borrow stays closed");
        for slot in [SEAT_ENTRY, BORROW_ENTRY] {
            let view = slot_view(hats_only, facts(slot, false, none_in_use, true));
            assert_eq!(view, [None; 6], "entry {slot}");
            assert!(!holds_back_creation(view, false), "entry {slot} does not hold the created entries back");
        }
        let created = slot_view(hats_only, facts(7, true, none_in_use, true));
        assert!(holds_back_creation(created, false), "a created entry with a target to load is filled first");
        assert!(!holds_back_creation(created, true));
        assert!(!created_busy([true, true, true, false], 4));
        assert!(created_busy([true, true, true, true], 4));
        assert_eq!(slot_view(hats_only, facts(BORROW_ENTRY, false, true, true))[0], Some(0x14),
            "every allowed created entry in use: entry 3 may borrow");
        assert!(created_busy([true, false, false, false], 1), "only the allowed ones count");
        assert!(created_busy([false; 4], 0), "no created entry allowed: the borrow is the only place left");
    }

    /// Primary 2026-11-09 09:47Z (Eden final-xps-hot hs06; it passed in rc6-hot-xps1/3): P1 Kirby, one native CPU, a
    /// hats-only plan. F9 reading A parks entries 2 and 3 on the CPU rotation's seat members (`parked_seat`) whether or
    /// not the plan lists them, so their views (`character_sources::purpose_view`) admit those members. A parked seat
    /// holding its own member Ready is filled: the created entries 7/6/5/4 are created in turn (the walk of
    /// `character_sources::creation_intent`) and each loads a hat. A seat still loading its member holds them back.
    fn parked_seats_on_their_members_let_a_hats_only_plan_create_its_entries() {
        use crate::model::parked_entries::parked_seat;
        let hats_only = [Some((0x14, HAT)), Some((0x10, HAT)), Some((0x11, HAT)), Some((0x18, HAT)), Some((0x1a, HAT)), None];
        let seats = [(SEAT_ENTRY, 0x0du32), (BORROW_ENTRY, 0x0f)];
        for (slot, member) in seats {
            assert_eq!(parked_seat::<u8>(slot as u32, Some(1), false, Some((member, None))), Some((member, None)),
                "entry {slot} is a parked seat on {member:#x}");
        }
        let seat_view = |member: u32| {
            let mut view = [None; 6];
            view[0] = Some(member);
            view
        };
        // Each step: earlier loads have completed; the next created entry is made unless an entry before it holds it
        // back, and it loads the first hat no entry holds.
        let walk = |seat_ready: bool| {
            let allowance = created_allowance(0, 5, 2, 4, true);
            let mut created: Vec<(usize, u32)> = Vec::new();
            for _ in 0..8 {
                if created.len() == allowance {
                    break;
                }
                let seats_back = seats.iter().any(|(_, member)| holds_back_creation(seat_view(*member),
                    creation_filled(seat_ready, hats_only.iter().flatten().any(|(k, _)| k == member), true, true)));
                let created_back = created.iter().any(|(slot, _)| holds_back_creation(
                    slot_view(hats_only, facts(*slot, true, false, false)), creation_filled(true, true, true, false)));
                if seats_back || created_back {
                    break;
                }
                let slot = [7, 6, 5, 4][created.len()];
                let hat = slot_view(hats_only, facts(slot, true, false, false)).into_iter().flatten()
                    .find(|kind| !created.iter().any(|(_, held)| held == kind)).expect("a hat to load");
                created.push((slot, hat));
            }
            created
        };
        assert_eq!(walk(true), [(7, 0x14), (6, 0x10), (5, 0x11), (4, 0x18)], "four created entries, four hats");
        assert_eq!(walk(false), [], "a seat still loading its member is filled first");
        // The rule's other clauses stand: a plan target, a source that may not be replaced, nothing Ready.
        assert!(creation_filled(true, true, true, false));
        assert!(creation_filled(true, false, false, false));
        assert!(!creation_filled(true, false, true, false), "another kind on a free helper: not filled");
        assert!(!creation_filled(false, true, true, true));
    }

    /// Operator decision "1": the record is published at the hat's admission on entry 3, read by F9, kept until F9
    /// retires or promotes the entry (or the scene ends), and while it exists entry 3 holds only the borrowed hat.
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

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_seat_held_for_memory_counts_in_the_fighter_status", a_seat_held_for_memory_counts_in_the_fighter_status as fn()),
            ("hats_are_budgeted_apart_from_fighters_and_a_shared_kind_is_one_fighter_target", hats_are_budgeted_apart_from_fighters_and_a_shared_kind_is_one_fighter_target as fn()),
            ("a_fifth_hat_borrows_entry_3_only_with_one_cpu", a_fifth_hat_borrows_entry_3_only_with_one_cpu as fn()),
            ("a_hats_only_plan_creates_four_entries_before_the_borrow", a_hats_only_plan_creates_four_entries_before_the_borrow as fn()),
            ("parked_seats_on_their_members_let_a_hats_only_plan_create_its_entries", parked_seats_on_their_members_let_a_hats_only_plan_create_its_entries as fn()),
            ("a_borrowed_entry_3_carries_a_record_f9_reads", a_borrowed_entry_3_carries_a_record_f9_reads as fn()),
        ]);
    }
    }
}
