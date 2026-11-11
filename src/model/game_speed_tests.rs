use super::*;
use crate::model::chords::{logical, Sample, STYLE_GAMECUBE};
use crate::model::menu::chord::Chord;

const PRO: u32 = 5;

fn bits(buttons: &[u8]) -> u16 {
    buttons.iter().fold(0u16, |set, button| set | (1 << button))
}

fn sample(held: &[u8], pressed: &[u8]) -> Sample {
    Sample { held: bits(held), pressed: bits(pressed) }
}

const R_UP: Chord = Chord((1 << logical::R) | (1 << logical::DPAD_UP));
use logical::{DPAD_LEFT as LEFT, DPAD_RIGHT as RIGHT, DPAD_UP as UP, R};

#[test]
fn speeds_are_the_native_values_and_labels() {
    let table = [(1, "x1"), (2, "1 Frame"), (3, "2/3"), (4, "1/2"), (5, "1/4"), (6, "x1.5")];
    for (value, label) in table {
        let speed = Speed::from_value(value).unwrap();
        assert_eq!((speed.value(), speed.label()), (value, label));
    }
    assert_eq!(Speed::from_value(0), None, "0 is the apply's 'no Training object', not a speed");
    assert_eq!(Speed::from_value(7), None);
    assert_eq!(Speed::Half.slot_divisor(), Some(2));
    assert_eq!(Speed::Quarter.slot_divisor(), Some(4));
    for speed in [Speed::X1, Speed::OneFrame, Speed::TwoThirds, Speed::OneAndHalf] {
        assert_eq!(speed.slot_divisor(), None, "{speed:?} needs the update rate too");
    }
}

#[test]
fn training_steps_walk_the_speeds_and_stop_at_the_ends() {
    let list = Place::Training.speeds();
    assert_eq!(list, &[Speed::Quarter, Speed::Half, Speed::TwoThirds, Speed::X1, Speed::OneAndHalf]);
    assert!(!list.contains(&Speed::OneFrame), "1 Frame stays with the native menu");
    let mut speed = Speed::X1;
    for expected in [Speed::TwoThirds, Speed::Half, Speed::Quarter, Speed::Quarter] {
        speed = step(list, speed, Dir::Left);
        assert_eq!(speed, expected);
    }
    for expected in [Speed::Half, Speed::TwoThirds, Speed::X1, Speed::OneAndHalf, Speed::OneAndHalf] {
        speed = step(list, speed, Dir::Right);
        assert_eq!(speed, expected);
    }
    // Training on 1 Frame: the first step starts from x1's place.
    assert_eq!(step(list, Speed::OneFrame, Dir::Left), Speed::TwoThirds);
    assert_eq!(step(list, Speed::OneFrame, Dir::Right), Speed::OneAndHalf);
    assert_eq!(step(&[], Speed::Half, Dir::Right), Speed::Half);
}

#[test]
fn lobby_offers_only_slot_speeds() {
    let list = Place::Lobby.speeds();
    assert_eq!(list, &[Speed::Quarter, Speed::Half, Speed::X1]);
    for speed in list {
        assert!(*speed == Speed::X1 || speed.slot_divisor().is_some(), "{speed:?} is not the slot alone");
    }
    assert_eq!(step(list, Speed::X1, Dir::Right), Speed::X1, "no speed-up in the lobby until QCF's proof");
    assert_eq!(step(list, Speed::X1, Dir::Left), Speed::Half);
    assert_eq!(step(list, Speed::Half, Dir::Left), Speed::Quarter);
}

#[test]
fn hold_opens_on_the_chord_steps_on_left_right_and_closes_on_release() {
    let mut hold = Hold::new();
    assert_eq!(hold.poll(R_UP, sample(&[R], &[R]), false), HoldEvent::Idle, "R alone is no chord");
    assert_eq!(hold.poll(R_UP, sample(&[R, UP], &[UP]), false), HoldEvent::Opened);
    assert!(hold.is_open());
    // The opening press is not a step; holding is not a step.
    assert_eq!(hold.poll(R_UP, sample(&[R, UP], &[]), false), HoldEvent::Held);
    // Up released, R still held: still open.
    assert_eq!(hold.poll(R_UP, sample(&[R], &[]), false), HoldEvent::Held);
    assert_eq!(hold.poll(R_UP, sample(&[R, RIGHT], &[RIGHT]), false), HoldEvent::Step(Dir::Right));
    assert_eq!(hold.poll(R_UP, sample(&[R, RIGHT], &[]), false), HoldEvent::Held, "a held Right steps once");
    assert_eq!(hold.poll(R_UP, sample(&[R, LEFT], &[LEFT]), false), HoldEvent::Step(Dir::Left));
    assert_eq!(hold.poll(R_UP, sample(&[R, LEFT, RIGHT], &[LEFT, RIGHT]), false), HoldEvent::Held, "both at once: no step");
    // Every chord button released: the pop-up goes.
    assert_eq!(hold.poll(R_UP, sample(&[LEFT], &[]), false), HoldEvent::Closed);
    assert!(!hold.is_open());
    assert_eq!(hold.poll(R_UP, sample(&[LEFT], &[LEFT]), false), HoldEvent::Idle, "Left alone after close is the game's");
    // An input owner closes it and keeps it from opening.
    assert_eq!(hold.poll(R_UP, sample(&[R, UP], &[UP]), true), HoldEvent::Idle);
    assert_eq!(hold.poll(R_UP, sample(&[R, UP], &[UP]), false), HoldEvent::Opened);
    assert_eq!(hold.poll(R_UP, sample(&[R, UP], &[]), true), HoldEvent::Closed);
    // A chord that holds D-pad Left itself: its opening press is not a step, a new press is.
    let r_left = Chord((1 << R) | (1 << LEFT));
    let mut hold = Hold::new();
    assert_eq!(hold.poll(r_left, sample(&[R, LEFT], &[LEFT]), false), HoldEvent::Opened);
    assert_eq!(hold.poll(r_left, sample(&[R], &[]), false), HoldEvent::Held);
    assert_eq!(hold.poll(r_left, sample(&[R, LEFT], &[LEFT]), false), HoldEvent::Step(Dir::Left));
    hold.clear();
    assert!(!hold.is_open());
}

#[test]
fn the_hold_hides_its_chord_and_the_step_buttons_while_open() {
    let mut hold = Hold::new();
    assert_eq!(hold.hidden(), 0);
    hold.poll(R_UP, sample(&[R, UP], &[R, UP]), false);
    assert_eq!(hold.hidden(), bits(&[R, UP, LEFT, RIGHT]));
    hold.poll(R_UP, sample(&[], &[]), false);
    assert_eq!(hold.hidden(), 0);
}

#[test]
fn popup_body_shows_the_speed_between_the_dpad_glyphs() {
    assert_eq!(popup_body(Speed::Half, PRO).as_str(), "\u{E07B} 1/2 \u{E07C}");
    assert_eq!(popup_body(Speed::OneAndHalf, PRO).as_str(), "\u{E07B} x1.5 \u{E07C}");
    assert_eq!(popup_body(Speed::OneFrame, STYLE_GAMECUBE).as_str(), "\u{E20B} 1 Frame \u{E20C}");
    assert_eq!(POPUP_HEADER, "Game speed");
}

#[test]
fn the_popup_takes_the_screen_at_once_and_the_notice_it_covered_returns() {
    use crate::model::notify::{Body, Queue, DEFAULT_COLOR};
    let mut q = Queue::new();
    q.post_static("Save state", "Saved", 3, DEFAULT_COLOR);
    assert_eq!(q.tick(true, false).draw.map(|d| d.header), Some("Save state"));
    let body = popup_body(Speed::Half, PRO);
    q.show_now(POPUP_ID, POPUP_HEADER, Body::copied(body.as_str()), POPUP_FRAMES, DEFAULT_COLOR);
    assert_eq!(q.tick(true, false).draw.map(|d| d.header), Some(POPUP_HEADER), "on screen the next tick");
    // A step updates it in place: drawn again, still first.
    q.show_now(POPUP_ID, POPUP_HEADER, Body::copied(popup_body(Speed::Quarter, PRO).as_str()), POPUP_FRAMES, DEFAULT_COLOR);
    assert_eq!(q.tick(true, false).draw.map(|d| d.header), Some(POPUP_HEADER));
    assert_eq!(q.tick(true, false).draw, None, "drawn once per change");
    assert_eq!(q.len(), 2);
    // Released: the covered notice is drawn again with the time it had left (2 of 3 ticks).
    q.clear_id(POPUP_ID);
    assert_eq!(q.tick(true, false).draw.map(|d| d.header), Some("Save state"));
    assert!(q.tick(true, false).visible);
    assert!(q.is_empty());
}

#[test]
fn training_write_is_the_native_store_then_text() {
    let write = training_write(Speed::Half.value(), Speed::X1.value(), false).unwrap();
    assert_eq!(write, TrainingWrite { field: 0xb54, value: 4, text_pane: 0x538 });
    assert_eq!(training_write(4, 4, false), None, "nothing to write when the field already holds it");
    assert_eq!(training_write(4, 1, true), None, "the game's own menu is open: it owns the pane");
    assert_eq!(training_write(0, 1, false), None);
    assert_eq!(training_write(9, 1, false), None);
    // The offsets are the ones the setter's own unchanged-value path uses: `game_speed_rows_decode_the_native_offsets`
    // decodes both from that row's guard words.
    assert_eq!((write.field, write.text_pane), (SPEED_FIELD, SPEED_TEXT_PANE));
}

#[test]
fn slot_take_and_free_match_the_native_steps() {
    let mut t = fresh_table();
    t[table::DIRTY_AT] = 0;
    // The game holds slot 0 (a hit slow, divisor 3).
    t[0x20] = 1;
    t[0x21] = 3;
    assert_eq!(take(&mut t, 2), Some(1));
    assert_eq!((t[0x22], t[0x23], t[table::DIRTY_AT]), (1, 2, 1));
    t[table::DIRTY_AT] = 0;
    // A free of a slot that no longer holds our {1, 2} writes nothing.
    assert!(!free(&mut t, 1, 4));
    assert!(!free(&mut t, 2, 2));
    assert!(!free(&mut t, 16, 2));
    assert_eq!(t[table::DIRTY_AT], 0);
    assert!(free(&mut t, 1, 2));
    assert_eq!((t[0x22], t[0x23], t[table::DIRTY_AT]), (0, 1, 1));
    assert_eq!((t[0x20], t[0x21]), (1, 3), "the game's slot is untouched");
    // A full table: no slot, no write.
    let mut full = fresh_table();
    for slot in 0..table::SLOTS {
        full[0x20 + 2 * slot] = 1;
    }
    let before = full;
    assert_eq!(take(&mut full, 2), None);
    assert_eq!(full, before);
}

#[test]
fn lobby_slot_is_released_outside_the_practice_and_never_written_elsewhere() {
    const TABLE: usize = 0x1_0000_1000;
    let live = LobbyNow { in_practice: true, ui_busy: false, table: TABLE };
    let mut t = fresh_table();
    let mut keeper = LobbySlot::new();
    // x1 takes nothing.
    assert_eq!(keeper.step(Speed::X1, live, Some(&mut t)), LobbyStep::Nothing);
    let taken = Held { table: TABLE, slot: 0, divisor: 2 };
    assert_eq!(keeper.step(Speed::Half, live, Some(&mut t)), LobbyStep::Taken(taken));
    assert_eq!((t[0x20], t[0x21]), (1, 2));
    assert_eq!(keeper.step(Speed::Half, live, Some(&mut t)), LobbyStep::Nothing, "edge-triggered");
    // A change frees first; the next frame takes the new one.
    assert_eq!(keeper.step(Speed::Quarter, live, Some(&mut t)), LobbyStep::Freed(taken));
    assert_eq!((t[0x20], t[0x21]), (0, 1));
    let quarter = Held { table: TABLE, slot: 0, divisor: 4 };
    assert_eq!(keeper.step(Speed::Quarter, live, Some(&mut t)), LobbyStep::Taken(quarter));
    // The match is found (the waiting-room UI busy): freed while the practice still runs, and not taken again.
    let busy = LobbyNow { ui_busy: true, ..live };
    assert_eq!(keeper.step(Speed::Quarter, busy, Some(&mut t)), LobbyStep::Freed(quarter));
    assert_eq!(keeper.step(Speed::Quarter, busy, Some(&mut t)), LobbyStep::Nothing);
    assert_eq!(t, fresh_table(), "back to the game's own table");
    // Taken, then the scene leaves the practice before a frame frees it, the table still the same: the first frame
    // outside frees it there (the slot is still ours), and nothing is taken outside the practice.
    assert_eq!(keeper.step(Speed::Half, live, Some(&mut t)), LobbyStep::Taken(taken));
    let gone = LobbyNow { in_practice: false, ui_busy: false, table: TABLE };
    assert_eq!(keeper.step(Speed::Half, gone, Some(&mut t)), LobbyStep::Freed(taken));
    assert_eq!(t, fresh_table());
    assert_eq!(keeper.held(), None);
    assert_eq!(keeper.step(Speed::Half, gone, Some(&mut t)), LobbyStep::Nothing, "never taken outside the practice");
    // Outside the practice with the slot no longer ours (the game freed and reused it): the record goes, no write.
    assert_eq!(keeper.step(Speed::Half, live, Some(&mut t)), LobbyStep::Taken(taken));
    t[0x21] = 3;
    let snapshot = t;
    assert_eq!(keeper.step(Speed::Half, gone, Some(&mut t)), LobbyStep::Dropped(taken));
    assert_eq!(t, snapshot, "a slot that is not ours is not written");
    // A different table (the practice's was destroyed and a new one made): the record goes, the new table is not touched
    // by a free.
    let mut keeper = LobbySlot::new();
    let mut old = fresh_table();
    keeper.step(Speed::Half, live, Some(&mut old));
    let mut new = fresh_table();
    new[0x20] = 1;
    new[0x21] = 2;
    let other = LobbyNow { table: TABLE + 0x100, ..live };
    let before = new;
    assert_eq!(keeper.step(Speed::X1, other, Some(&mut new)), LobbyStep::Dropped(taken));
    assert_eq!(new, before);
    // No table: nothing.
    let none = LobbyNow { table: 0, ..live };
    assert_eq!(LobbySlot::new().step(Speed::Half, none, None), LobbyStep::Nothing);
    // Every slot busy: no slot.
    let mut full = fresh_table();
    for slot in 0..table::SLOTS {
        full[0x20 + 2 * slot] = 1;
    }
    assert_eq!(LobbySlot::new().step(Speed::Half, live, Some(&mut full)), LobbyStep::NoFreeSlot);
    let mut keeper = LobbySlot::new();
    keeper.step(Speed::Half, live, Some(&mut fresh_table()));
    assert_eq!(keeper.forget(), Some(taken));
    assert_eq!(keeper.held(), None);
}

/// Worker B, 2026-11-11 08:20Z: a match found from the waiting room shows no busy edge (9 Quickplay matches); the practice
/// P1 is destroyed about 2 s before the match scene. Its session clear frees the slot with a write in the practice's
/// table, so the match finds the game's own table even if that table is never destroyed.
#[test]
fn match_found_with_no_busy_edge_frees_the_slot_before_the_drop() {
    const TABLE: usize = 0x1_0000_1000;
    for (speed, divisor) in [(Speed::Half, 2u8), (Speed::Quarter, 4)] {
        let live = LobbyNow { in_practice: true, ui_busy: false, table: TABLE };
        let mut t = fresh_table();
        // The game holds its own slot 0 (a hit slow) beside ours.
        t[0x20] = 1;
        t[0x21] = 3;
        let mut keeper = LobbySlot::new();
        let held = Held { table: TABLE, slot: 1, divisor };
        assert_eq!(keeper.step(speed, live, Some(&mut t)), LobbyStep::Taken(held));
        for _ in 0..120 {
            assert_eq!(keeper.step(speed, live, Some(&mut t)), LobbyStep::Nothing, "no busy edge: held all along");
        }
        t[table::DIRTY_AT] = 0;
        // The practice P1's destroy: the session clear, the table still the practice's.
        assert_eq!(keeper.release(TABLE, Some(&mut t)), LobbyStep::Freed(held));
        assert_eq!((t[0x22], t[0x23], t[table::DIRTY_AT]), (0, 1, 1), "freed as the game frees, dirty set");
        assert_eq!((t[0x20], t[0x21]), (1, 3), "the game's slot is untouched");
        // The scene change and the match's first P1 frame (the same table, never destroyed): nothing left to drop, no take.
        assert_eq!(keeper.release(TABLE, Some(&mut t)), LobbyStep::Nothing);
        let matched = LobbyNow { in_practice: false, ui_busy: false, table: TABLE };
        let before = t;
        assert_eq!(keeper.step(speed, matched, Some(&mut t)), LobbyStep::Nothing);
        assert_eq!(t, before);
        let in_use = (0..table::SLOTS).filter(|slot| t[0x20 + 2 * slot] == 1).map(|slot| t[0x21 + 2 * slot]).collect::<Vec<_>>();
        assert_eq!(in_use, [3], "only the game's own slot is in use in the match");
    }
    // The release in a table that is not the practice's (destroyed and made again, or none): the record goes, no write.
    let live = LobbyNow { in_practice: true, ui_busy: false, table: TABLE };
    let mut keeper = LobbySlot::new();
    keeper.step(Speed::Half, live, Some(&mut fresh_table()));
    let mut new = fresh_table();
    new[0x20] = 1;
    new[0x21] = 2;
    let before = new;
    assert_eq!(keeper.release(TABLE + 0x100, Some(&mut new)), LobbyStep::Dropped(Held { table: TABLE, slot: 0, divisor: 2 }));
    assert_eq!(new, before, "the next battle's table is not touched");
    keeper.step(Speed::Half, live, Some(&mut fresh_table()));
    assert_eq!(keeper.release(0, None), LobbyStep::Dropped(Held { table: TABLE, slot: 0, divisor: 2 }));
    assert_eq!(LobbySlot::new().release(TABLE, Some(&mut fresh_table())), LobbyStep::Nothing);
}
