//! F9: Training's parked CPU entries 2 and 3.
//!
//! The earlier setter-deferral machine is gone. The native CPU count is never
//! delayed and no setter is replayed. A raise's activation frame may publish a
//! ready body's own kind through the HUD and may release a borrow as Promoted.
//! A Reload of entry 2 or 3 is an `After` the runtime applies only on a later
//! frame, after that close's activation walk has returned.
//!
//! Entry 3's borrowed hat is not removed with `retire_at` (that destroys a
//! created extra). Retired means the runtime loads the seat member and then
//! `BorrowBook::release(Release::Retired)`.

/// Training helper entry that becomes CPU 2.
pub const ENTRY2: u32 = 2;
/// Training helper entry that becomes CPU 3, and the fifth-hat borrow.
pub const ENTRY3: u32 = 3;
/// Hat entries, first to last. Entry 3 is not in this list.
pub const HAT_ENTRIES: [u32; 4] = [7, 6, 5, 4];

/// What the loader callback can see on one parked entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Empty,
    /// A body is in flight. The activation frame does not start another install.
    Loading {
        id: Option<u32>,
    },
    Ready {
        id: u32,
        kind: u32,
    },
}

/// The CPU rotation's next fighter and the one after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seats {
    pub entry2: u32,
    pub entry3: u32,
}

/// How F9 ends the entry-3 borrow record (`hat_slots::Release` uses the same names).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// The seat member was loaded after the walk, and entry 3 did not activate.
    Retired,
    /// The borrowed body was published as CPU 3.
    Promoted,
}

/// HUD publication. The kind and body id are the body's own, not the seat's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Publish {
    pub owner: u32,
    pub kind: u32,
    pub body: u32,
}

/// Identity or body-id change. There is no charge field: nothing writes charge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cutover {
    pub clear_hat: bool,
    pub copy_kind: Option<u32>,
    pub item: Option<u32>,
}

/// A Reload the runtime may start only after the activation walk returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct After {
    pub slot: u32,
    pub kind: u32,
    /// Set when this load is what ends a 1→2 borrow.
    pub release: Option<Release>,
    pub cutover: Option<Cutover>,
}

/// One observation of a CPU-count change. `after` is not an activation-frame install.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Decision {
    pub publish: [Option<Publish>; 2],
    /// Release the borrow before the next admission. Promoted only; Retired waits on `after`.
    pub release_now: Option<Release>,
    /// While set, the runtime admits no new load on entries 2 and 3.
    pub hold_installs: bool,
    pub after_entry2: Option<After>,
    pub after_entry3: Option<After>,
}

/// Facts for one count change. The runtime fills these; the model does not read the game.
#[derive(Clone, Copy, Debug)]
pub struct CountFacts {
    pub previous: Option<u32>,
    pub quantity: Option<u32>,
    pub borrowed_kind: Option<u32>,
    pub entry2: Body,
    pub entry3: Body,
    pub seats: Option<Seats>,
    pub thrown_from_entry3: Option<u32>,
    pub kirby_has_copy: bool,
}

/// Training resource admission: one, two or three CPUs. None and anything else stay closed.
pub fn training_quantity_open(quantity: Option<u32>) -> bool {
    matches!(quantity, Some(1..=3))
}

/// A parked helper seat. A live entry (`slot <= quantity`) is not one. A borrowed entry 3 is a hat, not this seat.
pub fn helper_seat(slot: u32, quantity: Option<u32>, borrowed: bool) -> bool {
    let Some(quantity) = quantity else {
        return false;
    };
    if !training_quantity_open(Some(quantity)) || !matches!(slot, ENTRY2 | ENTRY3) {
        return false;
    }
    if slot <= quantity {
        return false;
    }
    !(slot == ENTRY3 && borrowed)
}

/// TASK-OPUS-rc6-open-fixes item 1, reading A (primary 2026-11-08 23:03Z): entry `slot` (2 or 3) is a live seat while
/// the game plays it as a CPU: F9 promoted it (`promoted`, per entry 2 and 3: a published seat the game activated, until
/// it is re-proved once parked) and the slot is within No. of CPUs. The Mii owners' snapshot then treats it as a CPU
/// owner, not a parked helper row.
pub fn live_seat(slot: u32, promoted: [bool; 2], quantity: Option<u32>) -> bool {
    let index = match slot {
        ENTRY2 => 0,
        ENTRY3 => 1,
        _ => return false,
    };
    promoted[index] && training_quantity_open(quantity) && quantity.is_some_and(|quantity| slot <= quantity)
}

/// Where a hat may be placed. Entries 7/6/5/4 at any open count. Entry 3 only as the fifth hat at one CPU.
/// Entry 2 never.
pub fn hat_may_use(slot: u32, quantity: Option<u32>, borrow_open: bool) -> bool {
    match slot {
        4..=7 => training_quantity_open(quantity),
        ENTRY3 => quantity == Some(1) && borrow_open,
        _ => false,
    }
}

/// Hat clear, copy setup and thrown-item removal when the identity or the body id changes.
/// Same id and kind: nothing. No charge is represented.
pub fn plan_cutover(
    old_id: u32,
    old_kind: u32,
    new_id: u32,
    new_kind: u32,
    item: Option<u32>,
    kirby_has_copy: bool,
) -> Option<Cutover> {
    if old_id == new_id && old_kind == new_kind {
        return None;
    }
    Some(Cutover {
        clear_hat: true,
        copy_kind: kirby_has_copy.then_some(new_kind),
        item,
    })
}

fn cutover_from(body: Body, new_kind: u32, facts: &CountFacts) -> Option<Cutover> {
    // The new body id is not known until the load finishes. A different kind is an identity change.
    // The same kind is not, and this plan does not invent a body-id change.
    let (old_id, old_kind) = match body {
        Body::Ready { id, kind } if kind != new_kind => (id, kind),
        Body::Loading { .. } => (0, facts.borrowed_kind.filter(|kind| *kind != new_kind)?),
        Body::Ready { .. } | Body::Empty => return None,
    };
    plan_cutover(
        old_id,
        old_kind,
        old_id,
        new_kind,
        facts.thrown_from_entry3,
        facts.kirby_has_copy,
    )
}

fn publish_ready(body: Body, owner: u32) -> Option<Publish> {
    match body {
        Body::Ready { id, kind } => Some(Publish {
            owner,
            kind,
            body: id,
        }),
        _ => None,
    }
}

/// The activation-frame decision. Reloads are returned in `after_*` and are not performed here.
pub fn on_count(facts: CountFacts) -> Decision {
    let mut decision = Decision::default();
    let (Some(previous), Some(quantity)) = (facts.previous, facts.quantity) else {
        return decision;
    };
    if quantity == previous
        || !training_quantity_open(Some(quantity))
        || !training_quantity_open(Some(previous))
    {
        return decision;
    }
    if quantity < previous {
        // The callback that sees a drop installs nothing.
        return decision;
    }
    decision.hold_installs = true;
    let seats = facts.seats;
    if previous < ENTRY2 && quantity >= ENTRY2 {
        decision.publish[0] = publish_ready(facts.entry2, ENTRY2);
        if let Some(seats) = seats {
            decision.after_entry2 =
                reload_if_needed(ENTRY2, facts.entry2, seats.entry2, None, &facts);
        }
    }
    // 1→2 does not activate entry 3. The borrowed hat is replaced after the walk.
    if previous == 1 && quantity == 2 && facts.borrowed_kind.is_some() {
        if let Some(seats) = seats {
            decision.after_entry3 = Some(After {
                slot: ENTRY3,
                kind: seats.entry3,
                release: Some(Release::Retired),
                cutover: cutover_from(facts.entry3, seats.entry3, &facts),
            });
        }
    } else if previous < ENTRY3 && quantity >= ENTRY3 {
        if facts.borrowed_kind.is_some() && previous == 1 {
            // Option 1: the hat body activates. Publish it as itself, then swap after the walk.
            decision.publish[1] = publish_ready(facts.entry3, ENTRY3);
            decision.release_now = Some(Release::Promoted);
            if let Some(seats) = seats {
                decision.after_entry3 = Some(After {
                    slot: ENTRY3,
                    kind: seats.entry3,
                    release: None,
                    cutover: cutover_from(facts.entry3, seats.entry3, &facts),
                });
            }
        } else {
            decision.publish[1] = publish_ready(facts.entry3, ENTRY3);
            if let Some(seats) = seats {
                decision.after_entry3 =
                    reload_if_needed(ENTRY3, facts.entry3, seats.entry3, None, &facts);
            }
        }
    }
    decision
}

/// Two count observations before one close. The pause menu raises No. of CPUs one step per press, so a raise from
/// one to three on one close is seen as 1→2 and then 2→3 (Eden covrest-07: `cpu_quantity=2`, then `=3`, both while
/// paused). The later decision keeps what the earlier one decided for an entry it does not decide itself: entry 2's
/// publication and seat Reload from the 1→2 step survive the 2→3 step.
pub fn merge_paused(earlier: Decision, later: Decision) -> Decision {
    Decision {
        publish: [later.publish[0].or(earlier.publish[0]), later.publish[1].or(earlier.publish[1])],
        release_now: later.release_now.or(earlier.release_now),
        hold_installs: later.hold_installs || earlier.hold_installs,
        after_entry2: later.after_entry2.or(earlier.after_entry2),
        after_entry3: later.after_entry3.or(earlier.after_entry3),
    }
}

/// What a seat Reload loads: the seat's kind with the seat member's exact-Mii configuration (F9 report §4.2: a member
/// is the kind, the costume, and the exact-Mii configuration when the CPU target has one). `member` is the CPU
/// rotation's member for that entry as (kind, configuration); its configuration counts only for the same kind.
/// A Mii class (`0x48..=0x4a`) without one is not loaded: installed as a plain fighter it is a body the game reads
/// null from once the entry activates (Eden covrest-08). The entry then keeps what it has.
pub fn seat_load<C: Copy>(kind: u32, member: Option<(u32, Option<C>)>) -> Option<(u32, Option<C>)> {
    if !(0x48..=0x4a).contains(&kind) {
        return Some((kind, None));
    }
    let configuration = member.filter(|(member, _)| *member == kind).and_then(|(_, configuration)| configuration)?;
    Some((kind, Some(configuration)))
}

/// The seat member a parked entry 2 or 3 holds, whether or not the merged plan lists it: the first raise after a scene
/// start shows the seats (reading A, primary 2026-11-08 16:04Z; "F9's fixed seats keep 2 and 3 for the next CPUs").
/// A body bank that covers the member does not free the seat. None on a live entry, on entry 3 while it holds the
/// borrowed hat, without a member, and for a Mii member without its configuration (`seat_load`).
pub fn parked_seat<C: Copy>(
    slot: u32,
    quantity: Option<u32>,
    borrowed: bool,
    member: Option<(u32, Option<C>)>,
) -> Option<(u32, Option<C>)> {
    if !helper_seat(slot, quantity, borrowed) {
        return None;
    }
    let (kind, _) = member?;
    seat_load(kind, member)
}

/// What F9 does with a promoted seat's recorded Mii proof (reading B, primary 2026-11-08 16:44Z). The game's activation
/// of entry 2 or 3 as a CPU, and its parking again after a drop, change bytes of the seat's Mii record (Eden
/// covrest-11: the helper `Proof` matched, the Mii record did not), and the Mii owners then refuse every snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reproof {
    /// The seat is not parked ready yet.
    Wait,
    /// The record and descriptor are the recorded ones.
    Unchanged,
    /// The identity is unchanged (source, configuration, binding, owner objects, scalars, charinfo, resource pair):
    /// take the record and descriptor again from the parked row.
    Rerecord,
    /// Another Mii, row or owner: never re-recorded; the refusal stands.
    Refuse,
}

pub fn reproof(parked: bool, identity_same: bool, state_same: bool) -> Reproof {
    match (parked, identity_same, state_same) {
        (false, _, _) => Reproof::Wait,
        (true, false, _) => Reproof::Refuse,
        (true, true, true) => Reproof::Unchanged,
        (true, true, false) => Reproof::Rerecord,
    }
}

/// The Mii record part of reading B's identity: the record still carries the configuration's charinfo (`0x00..0x58`)
/// and a resource pair (`0x58..0x68`, both halves present). The re-proof of a parked seat
/// (`mii_managed_helper::reproof_parked`) and a live seat's snapshot (`MiiProof::holds`, TASK-OPUS-rc6-open-fixes item
/// 1, primary 2026-11-09 00:23Z) check this one predicate in place of byte equality.
pub fn mii_record_identity(record: &[u8], charinfo: &[u8]) -> bool {
    record.len() >= 104
        && charinfo.len() == 88
        && record[..88] == *charinfo
        && record[88..96] != [0u8; 8]
        && record[96..104] != [0u8; 8]
}

/// What a parked seat (entry 2 or 3 at one CPU) holds, for the readiness notice (primary 2026-11-08 23:03Z,
/// F9-SEAT-NOTICE, reading A: the two seats count in the existing fighter status, with no new text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeatState {
    /// The seat member's body is ready on the entry.
    Ready,
    /// The member is still to load, or loading.
    Loading,
    /// The member's load waits for process memory: held and retried, never skipped (cov-rest 16:25Z).
    HeldMemory,
}

/// A seat's state from the seat member, the current Ready source on its entry (`ready`, kind and configuration) and
/// the kind whose last speculative load on that entry process memory refused (`held`; cleared when one is admitted).
pub fn seat_state<C: Copy + PartialEq>(member: (u32, Option<C>), ready: Option<(u32, Option<C>)>, held: Option<u32>) -> SeatState {
    if ready == Some(member) {
        SeatState::Ready
    } else if held == Some(member.0) {
        SeatState::HeldMemory
    } else {
        SeatState::Loading
    }
}

/// Ordering (primary 2026-11-08 17:28Z, reading B; Eden covrest-12): an F9 seat load waits while a HUD portrait job
/// wants a helper or holds one for its render, so the render is not pushed behind the seat's INSTALL into the role's
/// next press. The seat loads once the render is done.
pub fn seat_load_waits(portrait_job: bool) -> bool {
    portrait_job
}

/// A ready body that already is the seat member stays. An in-flight body is not given a second install.
/// An empty entry, or a ready body of another kind, is reloaded after the walk.
/// One read of a published cell: body id (`+8`), kind (`+0xc`), and `parked_ready`.
/// `None` means the cell word is not a native object, so the body is not read.
pub type PublishedBody = Option<(u32, u32, bool)>;

/// F9's view of one fighter entry.
///
/// The body read runs only when `installed` publishes cell 0
/// (`entry_cells::active_cell`, the game's `b.ls` at `main+0x642c88`).
/// State 0xb is what publishes that count (`650010.c:886-890`). A count of 0
/// leaves heap residue in the cell (F6-30); alignment of that residue is not
/// a body. The loader byte of such an entry is residue as well (Eden covrest-04/05:
/// 6, then 88, on entries never installed), so an unpublished cell is in flight
/// only when the plugin's own install on that entry is (`own_load_in_flight`,
/// asked only for an unpublished cell); otherwise it is empty and its seat reloads.
pub fn classify_parked_body(
    entry_aligned: bool,
    installed: u64,
    own_load_in_flight: impl FnOnce() -> bool,
    deref_cell: impl FnOnce() -> PublishedBody,
) -> Body {
    if !entry_aligned {
        return Body::Empty;
    }
    // Cell 0 is a body only after state 0xb publishes it (count at +0x4150).
    if crate::model::entry_cells::active_cell(0, installed).is_none() {
        return unpublished_body(own_load_in_flight());
    }
    match deref_cell() {
        Some((id, kind, true)) => Body::Ready { id, kind },
        Some((id, _, false)) => Body::Loading { id: Some(id) },
        None => unpublished_body(own_load_in_flight()),
    }
}

fn unpublished_body(own_load_in_flight: bool) -> Body {
    if own_load_in_flight {
        Body::Loading { id: None }
    } else {
        Body::Empty
    }
}

fn reload_if_needed(
    slot: u32,
    body: Body,
    member: u32,
    release: Option<Release>,
    facts: &CountFacts,
) -> Option<After> {
    match body {
        Body::Ready { kind, .. } if kind == member => None,
        Body::Loading { .. } => None,
        Body::Ready { .. } | Body::Empty => Some(After {
            slot,
            kind: member,
            release,
            cutover: cutover_from(body, member, facts),
        }),
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    /// DK then Link: the CPU cycle Mario, choices DK/Link/Samus, original Mario.
    /// `f9_seat_tests` checks this pair against `character_forms`.
    fn seats() -> Seats {
        Seats { entry2: 0x01, entry3: 0x02 }
    }

    fn facts(
        previous: u32,
        quantity: u32,
        entry2: Body,
        entry3: Body,
        borrowed: Option<u32>,
    ) -> CountFacts {
        CountFacts {
            previous: Some(previous),
            quantity: Some(quantity),
            borrowed_kind: borrowed,
            entry2,
            entry3,
            seats: Some(seats()),
            thrown_from_entry3: None,
            kirby_has_copy: false,
        }
    }

    fn promote_ready_body_becomes_that_cpu() {
        let body = Body::Ready { id: 42, kind: 0x01 };
        let decision = on_count(facts(1, 2, body, Body::Empty, None));
        assert_eq!(
            decision.publish[0],
            Some(Publish {
                owner: 2,
                kind: 0x01,
                body: 42
            })
        );
        assert!(decision.after_entry2.is_none());
        assert!(decision.release_now.is_none());
        // A different ready body is labelled as itself, and the seat member waits until after the walk.
        let other = on_count(facts(
            1,
            2,
            Body::Ready { id: 7, kind: 0x05 },
            Body::Empty,
            None,
        ));
        assert_eq!(
            other.publish[0],
            Some(Publish {
                owner: 2,
                kind: 0x05,
                body: 7
            })
        );
        assert_eq!(other.after_entry2.unwrap().kind, seats().entry2);
        assert!(other.hold_installs);
        // In flight: no publication and no second install.
        let loading = on_count(facts(
            1,
            2,
            Body::Loading { id: Some(7) },
            Body::Empty,
            None,
        ));
        assert!(loading.publish[0].is_none());
        assert!(loading.after_entry2.is_none());
    }

    fn live_entries_are_not_helper_seats() {
        assert!(!helper_seat(2, Some(2), false));
        assert!(helper_seat(3, Some(2), false));
        assert_eq!(seats().entry3, 0x02);
        assert!(!helper_seat(2, Some(3), false));
        assert!(!helper_seat(3, Some(3), false));
        assert!(helper_seat(2, Some(1), false));
        assert!(helper_seat(3, Some(1), false));
        assert!(!helper_seat(3, Some(1), true));
        assert!(!hat_may_use(2, Some(1), true));
        for slot in HAT_ENTRIES {
            assert!(hat_may_use(slot, Some(2), false), "slot {slot}");
            assert!(hat_may_use(slot, Some(3), false), "slot {slot}");
        }
        assert!(!hat_may_use(3, Some(2), true));
        assert!(!hat_may_use(3, Some(3), true));
        assert!(hat_may_use(3, Some(1), true));
        let sources = include_str!("../runtime/roster/character_sources.rs");
        assert!(
            !sources.contains("Character resources currently need one visible CPU"),
            "counts 2 and 3 are still refused"
        );
        assert!(sources.contains("training_quantity_open"));
        assert!(sources.contains("f9_gate::observe_quantity"));
        let roster = include_str!("../runtime/roster/character_roster.rs");
        assert!(roster.contains("owner <= 3"));
        let hud = include_str!("swap/native_hud_refresh.rs");
        assert!(hud.contains("owner > 3"));
    }

    fn cutover_rebuilds_hat_and_removes_thrown_item() {
        let same = plan_cutover(4, 0x06, 4, 0x06, Some(99), true);
        assert!(same.is_none());
        let changed = plan_cutover(4, 0x18, 4, 0x02, Some(99), true).unwrap();
        assert!(changed.clear_hat);
        assert_eq!(changed.copy_kind, Some(0x02));
        assert_eq!(changed.item, Some(99));
        let body = plan_cutover(4, 0x02, 8, 0x02, None, false).unwrap();
        assert!(body.clear_hat);
        assert!(body.copy_kind.is_none());
        assert!(body.item.is_none());
        // The planned effect has no charge write.
        let _ = (changed.clear_hat, changed.copy_kind, changed.item);
    }

    fn raising_the_cpu_count_retires_a_borrowed_hat_before_promotion_uses_entry_3() {
        let decision = on_count(facts(
            1,
            2,
            Body::Ready { id: 3, kind: 0x01 },
            Body::Ready { id: 9, kind: 0x18 },
            Some(0x18),
        ));
        assert!(decision.release_now.is_none());
        assert!(decision.publish[1].is_none());
        let after = decision.after_entry3.unwrap();
        assert_eq!(after.slot, 3);
        assert_eq!(after.kind, seats().entry3);
        assert_eq!(after.release, Some(Release::Retired));
        assert!(after.cutover.is_some());
        assert!(decision.hold_installs);
        // The hat is not moved onto entry 2.
        assert_ne!(decision.publish[0].unwrap().kind, 0x18);
        assert!(decision.after_entry2.is_none());
    }

    fn a_same_close_raise_to_three_promotes_the_borrowed_hat_then_swaps_to_the_seat_member() {
        let decision = on_count(facts(
            1,
            3,
            Body::Ready { id: 3, kind: 0x01 },
            Body::Ready { id: 9, kind: 0x18 },
            Some(0x18),
        ));
        assert_eq!(
            decision.publish[1],
            Some(Publish {
                owner: 3,
                kind: 0x18,
                body: 9
            })
        );
        assert_eq!(decision.release_now, Some(Release::Promoted));
        let after = decision.after_entry3.unwrap();
        assert_eq!(after.kind, seats().entry3);
        assert!(after.release.is_none());
        assert!(after.cutover.is_some());
        assert!(decision.hold_installs);
        // Nothing in this decision is an install the activation frame may run.
        assert!(decision.after_entry2.is_none() || decision.hold_installs);
    }

    fn a_drop_installs_nothing() {
        let decision = on_count(facts(
            3,
            1,
            Body::Ready { id: 1, kind: 0x01 },
            Body::Ready { id: 2, kind: 0x02 },
            None,
        ));
        assert!(!decision.hold_installs);
        assert!(decision.publish[0].is_none() && decision.publish[1].is_none());
        assert!(decision.after_entry2.is_none() && decision.after_entry3.is_none());
        assert!(decision.release_now.is_none());
    }

    fn setter_is_not_replayed() {
        let src = include_str!("parked_entries.rs");
        let replay = ["fn ", "replay("].concat();
        let defer = ["Setter", "::Defer"].concat();
        assert!(!src.contains(&replay));
        assert!(!src.contains(&defer));
        let decision = on_count(facts(
            1,
            2,
            Body::Ready { id: 1, kind: 0x01 },
            Body::Empty,
            None,
        ));
        assert!(decision.hold_installs);
    }

    fn training_counts_one_through_three_are_open() {
        assert!(training_quantity_open(Some(1)));
        assert!(training_quantity_open(Some(2)));
        assert!(training_quantity_open(Some(3)));
        assert!(!training_quantity_open(None));
        assert!(!training_quantity_open(Some(0)));
        assert!(!training_quantity_open(Some(4)));
    }

    fn a_stale_entry_does_not_dereference_the_body() {
        let body = classify_parked_body(true, 0, || false, || panic!("dereferenced an unpublished cell"));
        assert_eq!(body, Body::Empty);
        let body = classify_parked_body(true, 0, || true, || panic!("dereferenced an unpublished cell"));
        assert_eq!(body, Body::Loading { id: None });
    }

    /// Eden covrest-04/05/06 (2026-11-08): at one CPU entries 2 and 3 were never installed (count 0 every read) and
    /// their loader byte held residue (6, then 88). Read as "in flight", it armed no seat Reload, and after a raise to
    /// three CPUs 2 and 3 stayed copies of CPU 1 instead of the rotation's next and the one after. (This test failed
    /// first in its loader-byte form, `classify_parked_body(true, 0, 6, ..)` returning `Loading { id: None }`.)
    fn an_unpublished_entry_with_a_residue_loader_byte_is_empty() {
        let body = classify_parked_body(true, 0, || false, || panic!("dereferenced an unpublished cell"));
        assert_eq!(body, Body::Empty);
        // The raise to three then arms both seat Reloads after the walk: the rotation's next and the one after.
        let decision = on_count(facts(1, 3, body, body, None));
        assert_eq!(decision.after_entry2.map(|after| (after.slot, after.kind)), Some((ENTRY2, seats().entry2)));
        assert_eq!(decision.after_entry3.map(|after| (after.slot, after.kind)), Some((ENTRY3, seats().entry3)));
        assert!(decision.hold_installs);
        // The plugin's own install on that entry is what is in flight; no second install then.
        let own = classify_parked_body(true, 0, || true, || panic!("dereferenced an unpublished cell"));
        let decision = on_count(facts(1, 3, own, own, None));
        assert!(decision.after_entry2.is_none() && decision.after_entry3.is_none());
    }

    /// Eden covrest-07 (2026-11-08): the pause menu's two Right presses were seen as 1→2 and 2→3 before the close;
    /// the 2→3 decision replaced the 1→2 one, so entry 2's seat Reload was lost and only entry 3 reloaded later.
    fn a_raise_seen_in_two_steps_before_the_close_keeps_both_seats() {
        let first = on_count(facts(1, 2, Body::Empty, Body::Empty, None));
        let second = on_count(facts(2, 3, Body::Ready { id: 5, kind: 0x41 }, Body::Empty, None));
        assert!(second.after_entry2.is_none(), "2→3 alone decides entry 3 only");
        let merged = merge_paused(first, second);
        assert_eq!(merged.after_entry2.map(|after| (after.slot, after.kind)), Some((ENTRY2, seats().entry2)));
        assert_eq!(merged.after_entry3.map(|after| (after.slot, after.kind)), Some((ENTRY3, seats().entry3)));
        assert!(merged.hold_installs);
        // The later step wins for the entry it decides itself.
        let ready3 = on_count(facts(2, 3, Body::Empty, Body::Ready { id: 9, kind: seats().entry3 }, None));
        let merged = merge_paused(first, ready3);
        assert!(merged.after_entry3.is_none());
        assert_eq!(merged.publish[1].map(|p| (p.owner, p.kind)), Some((ENTRY3, seats().entry3)));
        assert_eq!(merged.after_entry2.map(|after| after.kind), Some(seats().entry2));
        // The gate merges while the earlier step still waits for the close.
        let gate = include_str!("../runtime/roster/f9_gate.rs");
        assert!(gate.contains("merge_paused(gate.decision, decision)"));
    }

    /// Eden covrest-08 (2026-11-08, diag 888badce): the seat Reload installed the CPU rotation's Mii Gunner on entry 2
    /// as a plain fighter (`install slot=2 kind=0x4a`, no Mii helper). When No. of CPUs rose to 2 the game read null
    /// every frame (`Unmapped Read64 @ 0x0`, main+0x357bf34). A Mii seat loads with the member's own configuration.
    fn a_mii_seat_loads_with_its_member_configuration_or_not_at_all() {
        assert_eq!(seat_load(0x4a, Some((0x4a, Some(7u8)))), Some((0x4a, Some(7))));
        assert_eq!(seat_load::<u8>(0x4a, Some((0x4a, None))), None, "a Mii class is never a plain fighter");
        assert_eq!(seat_load::<u8>(0x48, None), None);
        assert_eq!(seat_load(0x49, Some((0x01, Some(7u8)))), None, "another member's configuration is not this seat's");
        assert_eq!(seat_load::<u8>(0x01, Some((0x01, None))), Some((0x01, None)));
        assert_eq!(seat_load::<u8>(0x01, None), Some((0x01, None)), "an ordinary fighter needs no configuration");
        assert_eq!(seat_load(0x01, Some((0x4a, Some(7u8)))), Some((0x01, None)));
        // The gate arms only a loadable seat, and the planner's seat branch loads what was armed.
        let gate = include_str!("../runtime/roster/f9_gate.rs");
        assert!(gate.contains("parked_entries::seat_load("));
        let sources = include_str!("../runtime/roster/character_sources.rs");
        assert!(sources.contains("f9_gate::override_seat(slot as u32)"));
        assert!(!sources.contains("f9_gate::override_kind("));
        let preload = include_str!("../runtime/roster/cycle_preload.rs");
        assert!(preload.contains("cpu_rotation.iter().flatten().next().copied()"), "the members keep their configuration");
    }

    /// Eden covrest-07/08 (`cr-f9-raise`): the first raise after a scene start showed copies of CPU 1. At one CPU the
    /// seat branch loaded a member only when the merged plan listed it, and the body bank covered Donkey Kong, so
    /// entries 2 and 3 stayed empty until a drop armed their Reloads. Reading A: they park the members at one CPU.
    fn the_seats_park_at_one_cpu_whatever_the_plan_lists() {
        let dk = Some((0x01u32, None::<u8>));
        assert_eq!(parked_seat(ENTRY2, Some(1), false, dk), Some((0x01, None)));
        assert_eq!(parked_seat(ENTRY3, Some(1), false, Some((0x4a, Some(7u8)))), Some((0x4a, Some(7))));
        assert_eq!(parked_seat(ENTRY3, Some(2), false, dk), Some((0x01, None)), "entry 3 is parked at two CPUs");
        assert_eq!(parked_seat(ENTRY2, Some(1), true, dk), Some((0x01, None)), "a borrow is entry 3's only");
        assert_eq!(parked_seat(ENTRY2, Some(2), false, dk), None, "a live entry is not a seat");
        assert_eq!(parked_seat(ENTRY3, Some(1), true, dk), None, "a borrowed entry 3 holds the hat");
        assert_eq!(parked_seat::<u8>(ENTRY2, Some(1), false, None), None);
        assert_eq!(parked_seat::<u8>(ENTRY2, Some(1), false, Some((0x4a, None))), None);
        assert_eq!(parked_seat(ENTRY2, None, false, dk), None);
        assert_eq!(parked_seat(4, Some(1), false, dk), None);
        let sources = include_str!("../runtime/roster/character_sources.rs");
        assert!(sources.contains("parked_entries::parked_seat("));
        assert!(!sources.contains("want.kind == kind && want.configuration.is_none()"), "the seat waited for the plan");
    }

    /// Eden covrest-11 (2026-11-08, `[MiiHelperProof]`): after a raise promoted the Mii Gunner seat on entry 2 and a
    /// drop parked it again, its helper `Proof` still matched but its Mii record did not, so both Mii owners refused
    /// every snapshot ("helper row kind/provenance changed") and the Mii classes left both cycles. Reading B (primary
    /// 16:44Z): once parked, re-record the seat's Mii proof, only when its identity is unchanged.
    fn a_promoted_seat_is_re_proved_once_parked_only_with_its_identity() {
        assert_eq!(reproof(false, true, false), Reproof::Wait, "not parked yet");
        assert_eq!(reproof(false, false, false), Reproof::Wait);
        assert_eq!(reproof(true, true, true), Reproof::Unchanged);
        assert_eq!(reproof(true, true, false), Reproof::Rerecord, "only the activation's state bytes changed");
        assert_eq!(reproof(true, false, false), Reproof::Refuse, "another Mii, row or owner: never re-recorded");
        assert_eq!(reproof(true, false, true), Reproof::Refuse);
        let helper = include_str!("../runtime/mii/mii_managed_helper.rs");
        assert!(helper.contains("crate::model::parked_entries::reproof("));
        let sources = include_str!("../runtime/roster/character_sources.rs");
        assert!(sources.contains("f9_gate::mark_promoted("), "a published seat is marked");
        assert!(sources.contains("mii_managed_helper::reproof_parked("), "and re-proved once parked");
    }

    /// TASK-OPUS-rc6-open-fixes item 1 (cov-rest 17:14Z): at 2 or 3 CPUs a promoted seat plays as a CPU, so it is
    /// never parked, and both Mii owners refused every snapshot: the Mii classes were greyed in both cycles. Eden
    /// covrest-12 `[MiiHelperProof] ... id=2 why=proof parked=false`: the recorded proof equal to the row read now, only
    /// `parked` false. Reading A (primary 2026-11-08 23:03Z): while live (slot <= No. of CPUs) entries 2/3 are CPU owners
    /// for the Mii owners' snapshot, not parked helper rows.
    fn a_promoted_seat_within_the_cpu_count_is_a_live_cpu_owner() {
        assert!(live_seat(ENTRY2, [true, false], Some(2)));
        assert!(live_seat(ENTRY2, [true, true], Some(3)));
        assert!(live_seat(ENTRY3, [true, true], Some(3)));
        assert!(!live_seat(ENTRY3, [true, true], Some(2)), "entry 3 is parked at two CPUs");
        assert!(!live_seat(ENTRY2, [true, false], Some(1)), "dropped: parked again, and re-proved by reading B");
        assert!(!live_seat(ENTRY2, [false, false], Some(3)), "never promoted: its own row policy, unchanged");
        assert!(!live_seat(ENTRY2, [true, true], None), "the count unknown");
        assert!(!live_seat(1, [true, true], Some(3)) && !live_seat(4, [true, true], Some(3)), "only entries 2 and 3");
        let gate = include_str!("../runtime/roster/f9_gate.rs");
        assert!(gate.contains("crate::model::parked_entries::live_seat("), "F9 publishes its live seats");
        let native = include_str!("../runtime/mii/mii_ordinary_native.rs");
        assert!(native.contains("crate::runtime::roster::f9_gate::seat_live(id as u32)"), "the snapshot asks F9");
        assert!(native.contains("(parked || owner_live)"), "a live seat need not be parked; its proof stays exact");
    }

    /// Eden covrest-12: after the CPU loaded the Mii Gunner, its HUD portrait leased the seat on entry 2 but its render
    /// waited ~6 s behind seat 3's INSTALL and met the CPU's next press. Ordering (primary 17:28Z, reading B): an F9
    /// seat load waits while a portrait job wants a helper or holds one for its render.
    fn a_seat_load_holds_back_while_a_portrait_render_waits() {
        assert!(seat_load_waits(true));
        assert!(!seat_load_waits(false));
        let sources = include_str!("../runtime/roster/character_sources.rs");
        assert!(sources.contains("parked_entries::seat_load_waits(portrait_waits(state))"));
        assert!(sources.contains("matches!(live.consumer, Consumer::Portrait)"));
    }

    fn the_in_flight_question_is_asked_only_for_an_unpublished_cell() {
        let ready = classify_parked_body(true, 1, || panic!("asked for a published cell"), || Some((7, 0x01, true)));
        assert_eq!(ready, Body::Ready { id: 7, kind: 0x01 });
        assert_eq!(classify_parked_body(false, 0, || panic!("asked for a null entry"), || None), Body::Empty);
    }

    fn a_published_body_is_ready_only_after_parked_ready() {
        let mut reads = 0;
        let ready = classify_parked_body(true, 1, || false, || {
            reads += 1;
            Some((7, 0x01, true))
        });
        assert_eq!(reads, 1);
        assert_eq!(ready, Body::Ready { id: 7, kind: 0x01 });
        let loading = classify_parked_body(true, 1, || false, || Some((7, 0x01, false)));
        assert_eq!(loading, Body::Loading { id: Some(7) });
    }

    fn an_unaligned_published_pointer_is_not_a_body() {
        assert_eq!(
            classify_parked_body(true, 1, || true, || None),
            Body::Loading { id: None }
        );
        assert_eq!(classify_parked_body(true, 1, || false, || None), Body::Empty);
        assert_eq!(
            classify_parked_body(false, 9, || true, || panic!("read a null entry")),
            Body::Empty
        );
    }

    fn the_f9_tracepoint_spec_names_the_stale_skip() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/eden/phase23/trace-rc5-f9.md");
        let text = std::fs::read_to_string(path).expect(path);
        for needle in ["f9_body_of", "0x4150", "0x4160", "on_return", "slot", "stale"] {
            assert!(text.contains(needle), "missing {needle}");
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("promote_ready_body_becomes_that_cpu", promote_ready_body_becomes_that_cpu as fn()),
            ("live_entries_are_not_helper_seats", live_entries_are_not_helper_seats as fn()),
            ("cutover_rebuilds_hat_and_removes_thrown_item", cutover_rebuilds_hat_and_removes_thrown_item as fn()),
            ("raising_the_cpu_count_retires_a_borrowed_hat_before_promotion_uses_entry_3", raising_the_cpu_count_retires_a_borrowed_hat_before_promotion_uses_entry_3 as fn()),
            ("a_same_close_raise_to_three_promotes_the_borrowed_hat_then_swaps_to_the_seat_member", a_same_close_raise_to_three_promotes_the_borrowed_hat_then_swaps_to_the_seat_member as fn()),
            ("a_drop_installs_nothing", a_drop_installs_nothing as fn()),
            ("setter_is_not_replayed", setter_is_not_replayed as fn()),
            ("training_counts_one_through_three_are_open", training_counts_one_through_three_are_open as fn()),
            ("a_stale_entry_does_not_dereference_the_body", a_stale_entry_does_not_dereference_the_body as fn()),
            ("an_unpublished_entry_with_a_residue_loader_byte_is_empty", an_unpublished_entry_with_a_residue_loader_byte_is_empty as fn()),
            ("a_raise_seen_in_two_steps_before_the_close_keeps_both_seats", a_raise_seen_in_two_steps_before_the_close_keeps_both_seats as fn()),
            ("a_mii_seat_loads_with_its_member_configuration_or_not_at_all", a_mii_seat_loads_with_its_member_configuration_or_not_at_all as fn()),
            ("the_seats_park_at_one_cpu_whatever_the_plan_lists", the_seats_park_at_one_cpu_whatever_the_plan_lists as fn()),
            ("a_promoted_seat_is_re_proved_once_parked_only_with_its_identity", a_promoted_seat_is_re_proved_once_parked_only_with_its_identity as fn()),
            ("a_promoted_seat_within_the_cpu_count_is_a_live_cpu_owner", a_promoted_seat_within_the_cpu_count_is_a_live_cpu_owner as fn()),
            ("a_seat_load_holds_back_while_a_portrait_render_waits", a_seat_load_holds_back_while_a_portrait_render_waits as fn()),
            ("the_in_flight_question_is_asked_only_for_an_unpublished_cell", the_in_flight_question_is_asked_only_for_an_unpublished_cell as fn()),
            ("a_published_body_is_ready_only_after_parked_ready", a_published_body_is_ready_only_after_parked_ready as fn()),
            ("an_unaligned_published_pointer_is_not_a_body", an_unaligned_published_pointer_is_not_a_body as fn()),
            ("the_f9_tracepoint_spec_names_the_stale_skip", the_f9_tracepoint_spec_names_the_stale_skip as fn()),
        ]);
    }
    }
}
