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

