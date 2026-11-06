//! F9 seat checks that need the roster cycle and the hat placement. They live on the lib, not in
//! `model.rs`: the menu integration tests compile `model.rs` by path and have no `engine`.

use crate::engine::preload::hat_slots::{borrow_admitted, slot_view, Purpose, SlotFacts};
use crate::engine::swap::character_forms::{cycle_position, next_cycle, next_from_original};
use crate::model::parked_entries::{on_count, Body, CountFacts, Release, Seats, ENTRY3};

fn cpu_fixed_members(
    current: u32,
    cursor: Option<usize>,
    choices: &[u32],
    original: Option<u32>,
) -> Option<Seats> {
    let at_original =
        cursor.is_none() && original.is_some_and(|kind| cycle_position(&[kind], current).is_some());
    let (first, next) = if at_original {
        next_from_original(choices, original)?
    } else {
        next_cycle(current, cursor, choices, original)?
    };
    let (second, _) = next_cycle(first, next, choices, original)?;
    Some(Seats { entry2: first, entry3: second })
}

#[test]
fn fixed_seats_are_cpu_cycle_next_and_after() {
    let choices = [0x01u32, 0x02, 0x03];
    let original = Some(0x00);
    let members = cpu_fixed_members(0x00, None, &choices, original).unwrap();
    let at_original = original.is_some_and(|kind| cycle_position(&[kind], 0x00).is_some());
    let (first, cursor) = if at_original {
        next_from_original(&choices, original).unwrap()
    } else {
        next_cycle(0x00, None, &choices, original).unwrap()
    };
    let (second, _) = next_cycle(first, cursor, &choices, original).unwrap();
    assert_eq!((members.entry2, members.entry3), (first, second));
    assert_eq!((members.entry2, members.entry3), (0x01, 0x02));
    // Player 1's Kirby, and a hat source that is not on the CPU list, are not members.
    assert_ne!(members.entry2, 0x06);
    assert_ne!(members.entry3, 0x18);
    let decision = on_count(CountFacts {
        previous: Some(1),
        quantity: Some(2),
        borrowed_kind: Some(0x18),
        entry2: Body::Ready { id: 1, kind: 0x01 },
        entry3: Body::Ready { id: 9, kind: 0x18 },
        seats: Some(members),
        thrown_from_entry3: None,
        kirby_has_copy: false,
    });
    assert_eq!(decision.after_entry3.unwrap().kind, members.entry3);
    assert_eq!(decision.after_entry3.unwrap().release, Some(Release::Retired));
    // While the borrow record exists, entry 3's view is the hat. The member stays the one-after.
    let mut plan = [None; 8];
    plan[0] = Some((0x18u32, Purpose::Hat));
    plan[1] = Some((members.entry3, Purpose::Fighter));
    let view = slot_view(
        plan,
        SlotFacts {
            training: true,
            slot: ENTRY3 as usize,
            created: false,
            created_busy: true,
            borrow_open: true,
            borrowed: Some(0x18),
        },
    );
    assert_eq!(view[0], Some(0x18));
    assert!(!view.contains(&Some(members.entry3)));
    assert!(!borrow_admitted(Some(2), false, true));
    assert!(!borrow_admitted(Some(3), false, true));
    assert!(borrow_admitted(Some(1), false, true));
}
