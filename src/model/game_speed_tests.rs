lib_unit_tests! {
use super::*;
use crate::model::chords::{logical, Sample, STYLE_GAMECUBE};
use crate::model::menu::chord::Chord;

const PRO: u32 = 5;

fn bits(buttons: &[u8]) -> u16 {
    buttons.iter().fold(0u16, |set, button| set | (1 << button))
}

fn sample(held: &[u8], pressed: &[u8]) -> Sample {
    Sample { held: bits(held), pressed: bits(pressed), ..Sample::default() }
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

/// rc.8 R2: the lobby offers Training's five speeds, in the same order (2/3 and x1.5 through `model::lobby_speed`, never
/// through the update rate).
#[test]
fn lobby_offers_the_training_speeds() {
    let list = Place::Lobby.speeds();
    assert_eq!(list, Place::Training.speeds());
    assert_eq!(list, &[Speed::Quarter, Speed::Half, Speed::TwoThirds, Speed::X1, Speed::OneAndHalf]);
    assert_eq!(step(list, Speed::X1, Dir::Right), Speed::OneAndHalf);
    assert_eq!(step(list, Speed::X1, Dir::Left), Speed::TwoThirds);
    assert_eq!(step(list, Speed::TwoThirds, Dir::Left), Speed::Half);
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
    // With L held the chord doesn't open: Online Deluxe's L + R + D-pad Up (its 30 s capture) holds R + D-pad Up.
    let l = logical::L;
    assert_eq!(hold.poll(R_UP, sample(&[l, R, UP], &[UP]), false), HoldEvent::Idle, "L + R + Up is Online Deluxe's");
    assert_eq!(hold.poll(R_UP, sample(&[l, R, UP], &[l, R, UP]), false), HoldEvent::Idle, "nor all three on one poll");
    assert_eq!(hold.poll(R_UP, sample(&[R, UP], &[UP]), false), HoldEvent::Opened, "L released: it opens");
    // While open, a Left/Right with L held is no step; without L it is.
    assert_eq!(hold.poll(R_UP, sample(&[l, R, LEFT], &[LEFT]), false), HoldEvent::Held);
    assert_eq!(hold.poll(R_UP, sample(&[l, R, RIGHT], &[RIGHT]), false), HoldEvent::Held);
    assert_eq!(hold.poll(R_UP, sample(&[R, RIGHT], &[RIGHT]), false), HoldEvent::Step(Dir::Right));
    // A hold chord that holds L itself opens and steps with L held.
    let l_up = Chord((1 << l) | (1 << UP));
    let mut hold = Hold::new();
    assert_eq!(hold.poll(l_up, sample(&[l, UP], &[UP]), false), HoldEvent::Opened);
    assert_eq!(hold.poll(l_up, sample(&[l, LEFT], &[LEFT]), false), HoldEvent::Step(Dir::Left));
}

/// rc.9 (`PLUGIN-SHORTCUT-CONFLICTS.md`): the hold neither opens nor steps while Online Deluxe has the buttons: both
/// triggers are held on a GameCube pad, Deluxe's L + R there (C3; its L + R + Left/Right peer cycle is C10), or its page
/// with rows uses them (C4, narrowed in rc.12: R holds a shoulder, so R + Up opens and R + Left/Right steps beside the
/// page; a Left/Right under Deluxe's edit hold, ZR without ZL, does not step).
#[test]
fn hold_stands_down_while_deluxe_has_the_buttons() {
    use logical::{ZL, ZR};
    let gc = |held: &[u8], pressed: &[u8]| Sample { gamecube: true, ..sample(held, pressed) };
    let mut hold = Hold::new();
    assert_eq!(hold.poll(R_UP, sample(&[R, UP], &[UP]).with_overlay(true), false), HoldEvent::Opened, "a rows page");
    assert_eq!(hold.poll(R_UP, sample(&[R, RIGHT], &[RIGHT]).with_overlay(true), false), HoldEvent::Step(Dir::Right));
    assert_eq!(hold.poll(R_UP, sample(&[R, ZR, RIGHT], &[RIGHT]).with_overlay(true), false), HoldEvent::Held, "ZR: Deluxe edits");
    assert_eq!(hold.poll(R_UP, sample(&[], &[]), false), HoldEvent::Closed);
    assert_eq!(hold.poll(R_UP, gc(&[ZL, ZR, R, UP], &[UP]), false), HoldEvent::Idle, "GC: both triggers + Z + Up");
    assert_eq!(hold.poll(R_UP, gc(&[R, UP], &[UP]), false), HoldEvent::Opened, "GC: Z + Up opens");
    assert_eq!(hold.poll(R_UP, gc(&[ZL, ZR, R, LEFT], &[LEFT]), false), HoldEvent::Held, "GC: both triggers: no step");
    assert_eq!(hold.poll(R_UP, gc(&[ZL, R, LEFT], &[LEFT]), false), HoldEvent::Step(Dir::Left), "one trigger steps");
    assert_eq!(hold.poll(R_UP, sample(&[R, RIGHT], &[RIGHT]), false), HoldEvent::Step(Dir::Right));
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

/// The indicator after one bind step, from a flag: the animation played (if any) and the flag after it.
fn icon_step(to: Speed, shown: bool) -> (Option<usize>, bool) {
    let icon = hud_icon(to.value(), shown);
    (icon.map(|i| i.anim), icon.map_or(shown, |i| i.shown))
}

/// rc.8 R1 defect (the docs lane's GIFs 2026-11-11, `docs-rd10-eca14e4b`): after a step back to x1 the HUD kept showing
/// the last speed ("x1.5"), and a step away from x1 showed none. The game refreshes the indicator only in its menu state;
/// every closed-menu step now does that refresh.
#[test]
fn hud_icon_shows_away_from_x1_and_hides_back_at_x1() {
    let (show, hide) = (Some(hud::SHOW_ANIM), Some(hud::HIDE_ANIM));
    // Every step back to x1 from a shown indicator hides it: from x1.5 (the GIFs) and from 2/3 (1/2 and 1/4 reach x1
    // through 2/3).
    assert_eq!(icon_step(Speed::X1, true), (hide, false));
    // Every step away from x1 shows it.
    for to in [Speed::TwoThirds, Speed::OneAndHalf] {
        assert_eq!(icon_step(to, false), (show, true), "x1 -> {to:?}");
    }
    // Between the slow speeds the shown indicator stays; only its text changes (`0x1bbbd30` writes it for 2..6).
    for to in [Speed::Quarter, Speed::Half, Speed::TwoThirds, Speed::OneAndHalf] {
        assert_eq!(icon_step(to, true), (None, true), "{to:?} while shown");
    }
    assert_eq!(icon_step(Speed::X1, false), (None, false), "x1 while hidden");
    // The animations are the refresh's own (`game_speed_rows_decode_the_hud_icon`).
    assert_eq!((hud::SHOW_ANIM, hud::HIDE_ANIM, hud::SHOWN), (0x168, 0x178, 0x231));
}

/// Every Left/Right from every Training speed, from both flags: after the step the indicator is shown exactly when the
/// speed is not x1, and the game's own refresh on the next menu visit (the same rule) has nothing left to do.
#[test]
fn hud_icon_matches_the_speed_after_every_step_from_either_flag() {
    let list = Place::Training.speeds();
    for &from in list {
        for dir in [Dir::Left, Dir::Right] {
            let to = step(list, from, dir);
            for shown in [false, true] {
                let (anim, after) = icon_step(to, shown);
                assert_eq!(after, to != Speed::X1, "{from:?} {dir:?} -> {to:?} from shown={shown}");
                assert_eq!(anim.is_some(), after != shown, "{from:?} -> {to:?}: an animation only on a change");
                assert_eq!(hud_icon(to.value(), after), None, "{to:?}: the menu's refresh finds it done");
            }
        }
    }
    // The GIF's walk (`fx-speed-x1`): from 1/4 with the indicator shown, Right four times to x1.5, Left to x1.
    let (mut speed, mut shown) = (Speed::Quarter, true);
    for dir in [Dir::Right, Dir::Right, Dir::Right, Dir::Right, Dir::Left] {
        speed = step(list, speed, dir);
        shown = icon_step(speed, shown).1;
        assert_eq!(shown, speed != Speed::X1, "{speed:?}");
    }
    assert_eq!((speed, shown), (Speed::X1, false), "the stopwatch is gone at x1");
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
    // Taken, then the scene leaves the practice before a frame frees it: outside the practice nothing is written, even at
    // the same address (worker B's R8: a match's table can reuse the block, the game's own hit slow holding the same
    // `{1, N}` at the same slot); the record goes, and nothing is taken outside the practice.
    assert_eq!(keeper.step(Speed::Half, live, Some(&mut t)), LobbyStep::Taken(taken));
    let gone = LobbyNow { in_practice: false, ui_busy: false, table: TABLE };
    let snapshot = t;
    assert_eq!(keeper.step(Speed::Half, gone, Some(&mut t)), LobbyStep::Dropped(taken));
    assert_eq!(t, snapshot, "no write outside the practice");
    assert_eq!(keeper.held(), None);
    assert_eq!(keeper.step(Speed::Half, gone, Some(&mut t)), LobbyStep::Nothing, "never taken outside the practice");
    // The match's table in the same block, the game's own {1, 2} at the slot we held: untouched.
    let mut keeper = LobbySlot::new();
    let mut practice = fresh_table();
    assert_eq!(keeper.step(Speed::Half, live, Some(&mut practice)), LobbyStep::Taken(taken));
    let mut matched = fresh_table();
    matched[0x20] = 1;
    matched[0x21] = 2;
    let before = matched;
    assert_eq!(keeper.step(Speed::Half, gone, Some(&mut matched)), LobbyStep::Dropped(taken));
    assert_eq!(matched, before, "the game's own slot in a reused block is not freed");
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

#[test]
fn rates_are_the_speed_codes_table() {
    // QCF 09:12Z Q4 / jump table 0x44f0204: (rate code, updates per second, divisor).
    let got: Vec<_> = Speed::ALL.iter().map(|s| (s.label(), s.rate().code, s.rate().updates, s.rate().divisor)).collect();
    assert_eq!(
        got,
        [
            ("x1", 0, 60, None),
            ("1 Frame", 0, 60, None),
            ("2/3", 4, 120, Some(3)),
            ("1/2", 0, 60, Some(2)),
            ("1/4", 0, 60, Some(4)),
            ("x1.5", 6, 180, Some(2)),
        ]
    );
    // rc.8 R2: the lobby never applies `rate()`, so it never writes the rate words. Its two speeds whose Training rate
    // raises the update rate are exactly the paced slot and the extra pass (`model::lobby_speed::LobbyPlan`).
    use crate::model::lobby_speed::LobbyPlan;
    for speed in Place::Lobby.speeds() {
        let raises = speed.rate().code != 0 || speed.rate().updates != 60;
        let plan = LobbyPlan::of(*speed);
        assert_eq!(raises, matches!(plan, LobbyPlan::TwoThirds | LobbyPlan::ExtraPass), "{speed:?}");
    }
}

#[test]
fn mode_apply_does_the_speed_codes_writes() {
    // x1 -> 1/2 on a table where the game holds slot 0: the first free slot, no free, dirty to set.
    let mut t = fresh_table();
    t[table::DIRTY_AT] = 0;
    t[0x20] = 1;
    t[0x21] = 3;
    let half = mode_apply(Speed::Half, 1, mode::NO_SLOT, &mut t).unwrap();
    assert_eq!(half, ModeApply { value: 4, slot: 1, code: 0, updates: 60, freed: None, dirty: true });
    assert_eq!((t[0x22], t[0x23]), (1, 2));
    assert_eq!(t[table::DIRTY_AT], 0, "the caller sets dirty last");
    // 1/2 -> 1/4: its own slot freed {0, 1}, the first free one taken again.
    let quarter = mode_apply(Speed::Quarter, 4, 1, &mut t).unwrap();
    assert_eq!(quarter, ModeApply { value: 5, slot: 1, code: 0, updates: 60, freed: Some(1), dirty: true });
    assert_eq!((t[0x22], t[0x23]), (1, 4));
    assert_eq!((t[0x20], t[0x21]), (1, 3), "the game's other slot is untouched");
    // 1/4 -> 2/3 and x1.5: the update rate words, a slot each.
    let two_thirds = mode_apply(Speed::TwoThirds, 5, 1, &mut t).unwrap();
    assert_eq!((two_thirds.code, two_thirds.updates, two_thirds.slot, two_thirds.freed), (4, 120, 1, Some(1)));
    assert_eq!((t[0x22], t[0x23]), (1, 3));
    let fast = mode_apply(Speed::OneAndHalf, 3, 1, &mut t).unwrap();
    assert_eq!((fast.code, fast.updates, fast.slot), (6, 180, 1));
    assert_eq!((t[0x22], t[0x23]), (1, 2));
    // -> x1: freed, no slot, the rates back to 0/60.
    let x1 = mode_apply(Speed::X1, 6, 1, &mut t).unwrap();
    assert_eq!(x1, ModeApply { value: 1, slot: mode::NO_SLOT, code: 0, updates: 60, freed: Some(1), dirty: true });
    assert_eq!((t[0x22], t[0x23]), (0, 1));
    // Already applied: nothing, nothing written.
    let before = t;
    assert_eq!(mode_apply(Speed::X1, 1, mode::NO_SLOT, &mut t), None);
    assert_eq!(t, before);
    // A recorded slot the game has freed since is not written; a slot index past the table is ignored.
    let mut t = fresh_table();
    let done = mode_apply(Speed::Half, 1, 5, &mut t).unwrap();
    assert_eq!((done.freed, done.slot), (None, 0));
    let done = mode_apply(Speed::X1, 4, 0x20, &mut fresh_table()).unwrap();
    assert_eq!((done.freed, done.dirty), (None, false));
    // Every slot in use: no slot, as native (`+0x1b4` stays 0xff), the rates still written.
    let mut full = fresh_table();
    for slot in 0..table::SLOTS {
        full[0x20 + 2 * slot] = 1;
    }
    let done = mode_apply(Speed::Quarter, 1, mode::NO_SLOT, &mut full).unwrap();
    assert_eq!((done.slot, done.code, done.updates, done.dirty), (mode::NO_SLOT, 0, 60, false));
}

/// A libc++ unordered_map in a fake address space: `words` maps an address to its pointer-sized word.
fn registry(n: u64, nodes: &[(u64, u64, u64, u64)]) -> (usize, std::collections::HashMap<usize, u64>) {
    // nodes: (address, hash, key, object); chained in order through +0.
    let mut words = std::collections::HashMap::new();
    let (map, buckets) = (0x1000usize, 0x2000usize);
    words.insert(map, buckets as u64);
    words.insert(map + 8, n);
    for (i, node) in nodes.iter().enumerate() {
        let next = nodes.get(i + 1).map_or(0, |n| n.0);
        words.insert(node.0 as usize, next);
        words.insert(node.0 as usize + 8, node.1);
        words.insert(node.0 as usize + 0x10, node.2);
        words.insert(node.0 as usize + 0x18, node.3);
    }
    (map, words)
}

#[test]
fn registry_find_walks_the_games_map() {
    let key = mode::KEY;
    let hash = key & mode::KEY_MASK;
    // A power of two: bucket hash & (n-1); its head (a before-begin word) points at the first node.
    let n = 16u64;
    let bucket = hash & (n - 1);
    let other = hash + n; // same bucket, another hash
    let (map, mut words) = registry(n, &[(0x3000, other, other, 0xdead), (0x3100, hash, key | 0xff_0000_0000_0000, 0xbeef0)]);
    words.insert(0x2000 + 8 * bucket as usize, 0x4000);
    words.insert(0x4000, 0x3000);
    let read = |at: usize| words.get(&at).copied();
    assert_eq!(registry_find(map, key, &read), Some(0xbeef0), "the key above the 40-bit mask doesn't count");
    // A node of another bucket ends the walk.
    let (map, mut words) = registry(n, &[(0x3000, hash + 1, hash + 1, 0xdead), (0x3100, hash, key, 0xbeef0)]);
    words.insert(0x2000 + 8 * bucket as usize, 0x4000);
    words.insert(0x4000, 0x3000);
    let read = |at: usize| words.get(&at).copied();
    assert_eq!(registry_find(map, key, &read), None);
    // Not a power of two: hash % n.
    let n = 13u64;
    let (map, mut words) = registry(n, &[(0x3000, hash, key, 0xbeef0)]);
    words.insert(0x2000 + 8 * (hash % n) as usize, 0x4000);
    words.insert(0x4000, 0x3000);
    let read = |at: usize| words.get(&at).copied();
    assert_eq!(registry_find(map, key, &read), Some(0xbeef0));
    // An empty map, an empty bucket, an unreadable word: none.
    let (map, words) = registry(0, &[]);
    let read = |at: usize| words.get(&at).copied();
    assert_eq!(registry_find(map, key, &read), None);
    let (map, words) = registry(16, &[]);
    let read = |at: usize| words.get(&at).copied();
    assert_eq!(registry_find(map, key, &read), None);
    // A cycle is bounded.
    let (map, mut words) = registry(16, &[(0x3000, hash, key ^ 1, 0)]);
    words.insert(0x3000, 0x3000);
    words.insert(0x2000 + 8 * bucket as usize, 0x4000);
    words.insert(0x4000, 0x3000);
    let read = |at: usize| words.get(&at).copied();
    assert_eq!(registry_find(map, key, &read), None);
}
}
