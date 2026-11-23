//! SPEC-input-05 §9 host rows: the notification queue and the damage display helper.

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
use crate::model;

use std::cell::Cell;

use model::damage_display::*;
use model::notify::{Body, Queue, BODY_MAX, CAPACITY, DEFAULT_COLOR};

fn startup_notices_wait_for_training_and_settings_and_do_not_repeat_after_clear() {
    let mut boot = model::boot_notices::BootNotices::new();
    let mut queue = Queue::new();
    let chords = model::chords::ChordSet::factory();
    assert!(!boot.post_once(false, Some(chords), &mut queue));
    assert!(!boot.post_once(true, None, &mut queue));
    assert!(queue.is_empty());
    assert!(boot.post_once(true, Some(chords), &mut queue));
    assert_eq!(queue.len(), 6);
    queue.clear();
    assert!(!boot.post_once(true, Some(chords), &mut queue));
    assert!(queue.is_empty());
}

fn startup_displays_current_chords_in_order_for_exact_visible_durations() {
    use model::menu::chord::Chord;
    let mut chords = model::chords::ChordSet::factory();
    chords.shortcuts = model::menu_shortcuts::MenuShortcuts::new(Chord::NONE, Chord(1));
    chords.save_state = Chord(1 << 4 | 1 << 9);
    let mut boot = model::boot_notices::BootNotices::new();
    let mut queue = Queue::new();
    assert!(boot.post_once(true, Some(chords), &mut queue));
    for _ in 0..200 {
        assert!(!queue.tick(true, true).visible, "open menu must not consume notice time");
    }
    for (header, text, frames) in [
        ("TrainingRooms", "Welcome!", 60),
        ("Open Menu", "B + D-pad Up or A", 120),
        ("Save State", "L + D-pad Down", 120),
        ("Load State", "ZL + D-pad Up", 120),
        ("Input Record", "ZR + D-pad Down", 120),
        ("Input Playback", "ZR + D-pad Up", 120),
    ] {
        let out = queue.tick(true, false);
        assert!(out.visible);
        let draw = out.draw.unwrap();
        assert_eq!((draw.header, draw.body.as_str()), (header, text));
        for _ in 1..frames {
            let out = queue.tick(true, false);
            assert!(out.visible);
            assert!(out.draw.is_none());
        }
    }
    assert!(!queue.tick(true, false).visible);
}

fn longest_chords_survive_queue_utf16_wrap_and_preallocated_pane_boundaries() {
    use model::{boot_notices, menu::chord::Chord, notice_box, panes};
    // The review counterexample exceeds the former 48-unit queue and writer
    // limits; both maximal shortcuts also exceeded the former 256-byte builder.
    for chord in [Chord(0xc1c0), Chord(u16::MAX)] {
        let mut chords = model::chords::ChordSet::factory();
        chords.shortcuts = model::menu_shortcuts::MenuShortcuts::new(chord, chord);
        chords.save_state = chord;
        chords.load_state = chord;
        chords.input_record = chord;
        chords.input_playback = chord;
        let chord_text = chord.to_string();
        let menu_text = format!("B + D-pad Up or {chord} or {chord}");
        let mut queue = Queue::new();
        let mut boot = boot_notices::BootNotices::new();
        let before = ALLOCS.with(Cell::get);
        assert!(boot.post_once(true, Some(chords), &mut queue));
        assert_eq!(queue.len(), 6);
        for _ in 0..60 { queue.tick(true, false); }
        for expected in [&menu_text, &chord_text, &chord_text, &chord_text, &chord_text] {
            let draw = queue.tick(true, false).draw.unwrap();
            assert_eq!(draw.body.as_str(), expected);
            let mut units = [0; BODY_MAX];
            let len = boot_notices::notice_units(draw.body.as_str(), &mut units);
            assert_eq!(len, expected.len());
            let mut heading = [0u16; notice_box::HEADER_UNITS];
            let (placed, _, wrapped) = notice_box::layout(draw.header, &mut heading, &mut units, len);
            assert_eq!(wrapped, len, "the chord bodies wrap at spaces only");
            for (&unit, byte) in units[..len].iter().zip(expected.bytes()) {
                assert!(unit == u16::from(byte) || (byte == b' ' && unit == u16::from(b'\n')));
            }
            for line in units[..len].split(|unit| *unit == u16::from(b'\n')) {
                let (width, _) = notice_box::measure(line.iter().copied(), panes::NOTICE_BODY_FONT.0);
                assert!(width <= notice_box::MAX_TEXT_W, "every chord line fits the compact box");
            }
            assert!(placed.plate[0] + placed.plate[2] / 2.0 <= panes::NOTICE_LEFT + panes::NOTICE_MAX_W);
            assert!(placed.plate[1] - placed.plate[3] / 2.0 > 0.0, "the box stays in the top half");
            for _ in 1..120 { assert!(queue.tick(true, false).visible); }
        }
        assert!(!queue.tick(true, false).visible);
        assert_eq!(ALLOCS.with(Cell::get), before);
    }
    let pane = panes::spec(panes::IDX_NOTICE_LONG);
    let panes::Kind::Text { chars, font, .. } = pane.kind else { panic!("notice must be text") };
    assert!(usize::from(chars) > BODY_MAX, "preallocate the complete body and terminator");
    assert_eq!(font, panes::NOTICE_BODY_FONT, "the long body draws like the short one");
}

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn count_allocation() {
    let _ = ALLOCS.try_with(|count| count.set(count.get() + 1));
}

fn full_queue_drops_the_oldest_waiting_never_the_one_on_screen() {
    let mut q = Queue::new();
    for _ in 0..CAPACITY {
        q.post_static("H", "x", 5, DEFAULT_COLOR);
    }
    q.tick(true, false);
    q.post_static("N", "new", 5, DEFAULT_COLOR);
    assert_eq!(q.len(), CAPACITY);
    assert_eq!(q.front_header(), Some("H"));
    q.clear_header("H");
    assert_eq!(q.front_header(), Some("N"));
}

// Literal fixture lengths are independent of the production truncation loop.
const FULL_STATIC_BODY: &str = concat!(
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
);

fn maximum_inline_bodies_continue_through_redraw_update_and_delivery_without_heap() {
    let before = ALLOCS.with(Cell::get);
    let mut q = Queue::new();
    for _ in 0..32 {
        q.upsert(7, "Standing", Body::copied("old"), 1, DEFAULT_COLOR);
        q.post_static("Plain", FULL_STATIC_BODY, 2, DEFAULT_COLOR);
        q.upsert(7, "Standing", Body::copied(FULL_STATIC_BODY), 2, DEFAULT_COLOR);
        assert!(!q.tick(true, true).visible);
        assert_eq!(q.tick(true, false).draw.unwrap().body.as_str(), FULL_STATIC_BODY);
        q.on_layout_registered();
        assert_eq!(q.tick(true, false).draw.unwrap().body.as_str(), FULL_STATIC_BODY);
        let standing = q.tick(true, false).draw.unwrap();
        assert_eq!((standing.header, standing.body.as_str()), ("Standing", FULL_STATIC_BODY));
        assert_eq!(q.take_delivered(), None);
        assert!(q.tick(true, false).draw.is_none());
        assert_eq!(q.take_delivered(), Some(7));
        assert_eq!(q.take_delivered(), None);
        assert!(q.is_empty());
    }
    assert_eq!(ALLOCS.with(Cell::get), before);
}

fn copied_body_matches_single_part_joined() {
    let over_capacity = format!("{FULL_STATIC_BODY}x");
    let before = ALLOCS.with(Cell::get);
    for text in ["", "simple text", "é日本🦀", FULL_STATIC_BODY, &over_capacity] {
        assert_eq!(Body::copied(text), Body::joined(&[text]));
    }
    assert_eq!(ALLOCS.with(Cell::get), before);
}

fn copied_multibyte_body_truncates_at_a_complete_character_without_heap() {
    // 317 ASCII bytes plus the two-byte é fit; the following four-byte crab crosses byte 320.
    // The expected prefix and length are independent of the production truncation loop.
    let expected = format!("{}é", "a".repeat(317));
    let text = format!("{expected}🦀tail");
    assert!(text.len() > BODY_MAX);
    let before = ALLOCS.with(Cell::get);
    let copied = Body::copied(&text);
    let joined = Body::joined(&[&text]);
    assert_eq!(copied.as_str().as_bytes(), expected.as_bytes());
    assert_eq!(copied.as_str().len(), 319);
    assert_eq!(std::str::from_utf8(copied.as_str().as_bytes()), Ok(expected.as_str()));
    assert!(copied.as_str().len() <= BODY_MAX);
    assert_eq!(copied, joined);
    assert_eq!(ALLOCS.with(Cell::get), before);
}

// ---- damage display ------------------------------------------------------------------------------

fn pane_rules_fixture() {
    let d = Digits { hundreds: 0, tens: 0, ones: 7, tenths: 0 };
    let skipped_names: Vec<_> = PERCENT_PANES.iter().filter(|r| skipped(r, d)).map(|r| r.name).collect();
    assert_eq!(
        skipped_names,
        ["set_dmg_num_3", "dig_3", "dig_3_anim", "set_dmg_num_2", "dig_2", "dig_2_anim"]
    );
    // Tens shown once the hundreds are zero but the tens are not.
    let d = Digits { hundreds: 0, tens: 4, ones: 2, tenths: 1 };
    assert_eq!(PERCENT_PANES.iter().filter(|r| skipped(r, d)).count(), 3);
    let d = Digits { hundreds: 1, tens: 0, ones: 0, tenths: 0 };
    assert_eq!(PERCENT_PANES.iter().filter(|r| skipped(r, d)).count(), 0);
    let pos = |name: &str| PERCENT_PANES.iter().find(|r| r.name == name).unwrap().pos_y;
    assert_eq!((pos("set_dmg_p"), pos("set_dmg_num_p"), pos("dig_dec"), pos("dig_1")), (Some(0.0), Some(-4.0), Some(-16.0), Some(0.0)));
    assert_eq!(DEATH_EFFECT_PANES.len(), 4);
    // The animation frames follow the digits, and only the four material animations.
    let d = Digits { hundreds: 1, tens: 2, ones: 3, tenths: 4 };
    assert_eq!(
        ["set_dmg_num_3", "set_dmg_num_2", "set_dmg_num_1", "set_dmg_num_dec", "other"].map(|n| animation_frame(n, d)),
        [Some(1.0), Some(2.0), Some(3.0), Some(4.0), None]
    );
    assert!(needs_animation_rewrite(0, 255) && needs_animation_rewrite(255, 0) && !needs_animation_rewrite(255, 255));
    // Runs only during a load or a quick reset.
    assert!(runs(true, true, true));
    assert!(!runs(true, true, false) && !runs(false, true, true) && !runs(true, false, true));
}

fn missing_accessor_zero() {
    assert_eq!(digits_or_none(None), None, "nothing is written for a missing fighter");
    assert_eq!(digits_or_none(Some(12.3)), Some(Digits { hundreds: 0, tens: 1, ones: 2, tenths: 3 }));
}

fn the_standing_api_is_bounded_and_allocation_free() {
    let mut q = Queue::new();
    let before = ALLOCS.with(Cell::get);
    // More standing notices than the queue holds: the extra ids are ignored, nothing grows.
    for id in 0..(CAPACITY as u8 + 4) {
        q.upsert(id, "S", Body::copied("a body that is copied into the entry"), 5, DEFAULT_COLOR);
    }
    assert_eq!(q.len(), CAPACITY);
    assert!(q.is_queued(0) && !q.is_queued(CAPACITY as u8));
    // An existing id is still updated when the queue is full; a plain post finds nothing to drop.
    q.upsert(0, "S", Body::copied("new"), 5, DEFAULT_COLOR);
    q.post_static("H", "plain", 2, DEFAULT_COLOR);
    assert_eq!(q.len(), CAPACITY);
    // Delivery records are a small ring: the oldest goes.
    let mut q = Queue::new();
    for id in 0..6u8 {
        q.upsert(id, "S", Body::copied("x"), 1, DEFAULT_COLOR);
        q.tick(true, false);
    }
    let taken = [q.take_delivered(), q.take_delivered(), q.take_delivered(), q.take_delivered(), q.take_delivered()];
    assert_eq!(ALLOCS.with(Cell::get), before, "no allocation in the standing API");
    assert_eq!(taken, [Some(2), Some(3), Some(4), Some(5), None]);
}

fn a_refusal_shows_at_once_over_the_guard_wizard_page_and_gives_it_back() {
    const HEADER: &str = "Edge · 1/3";
    const PAGE: &str = "CPU (-12, 2) · Stick or D-pad move · A next · X default start · Y other side";
    const MOVED: &str = "CPU (-131, 2) · Stick or D-pad move · A next · X default start · Y other side";
    let mut q = Queue::new();
    q.upsert(0x34, HEADER, Body::copied(PAGE), u16::MAX, DEFAULT_COLOR);
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), (HEADER, PAGE));
    assert!(q.tick(true, false).draw.is_none());
    let mut plain = q;
    plain.post_static("Input Recording", "Pick a point off the stage", 120, DEFAULT_COLOR);
    for _ in 0..121 {
        let out = plain.tick(true, false);
        assert!(out.visible && out.draw.is_none());
    }
    q.post_refusal("Input Recording", Body::copied("Pick a point off the stage"), 120, DEFAULT_COLOR);
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Input Recording", "Pick a point off the stage"));
    q.upsert(0x34, HEADER, Body::copied(MOVED), u16::MAX, DEFAULT_COLOR);
    for _ in 2..=120 {
        let out = q.tick(true, false);
        assert!(out.visible && out.draw.is_none());
    }
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), (HEADER, MOVED));
    assert!(q.tick(true, false).draw.is_none());
    assert!(q.is_queued(0x34));
    assert_eq!(q.len(), 1);
    assert_eq!(q.take_delivered(), None);
}

fn a_refused_juggler_player_point_shows_over_its_page_once_per_press() {
    const PAGE: &str = "P1 (-34, 0) · Stick or D-pad move · A next";
    let mut q = Queue::new();
    q.upsert(0x34, "Juggle · 3/4", Body::copied(PAGE), u16::MAX, DEFAULT_COLOR);
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Juggle · 3/4", PAGE));
    for _ in 0..2 {
        q.post_refusal("Input Recording", Body::copied("Pick a point above a floor"), 120, DEFAULT_COLOR);
    }
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Input Recording", "Pick a point above a floor"));
    for _ in 2..=120 { assert!(q.tick(true, false).draw.is_none()); }
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Input Recording", "Pick a point above a floor"));
    for _ in 122..=240 { assert!(q.tick(true, false).draw.is_none()); }
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Juggle · 3/4", PAGE));
    assert_eq!(q.len(), 1);
}

fn a_refusal_during_a_running_drill_shows_over_the_drill_status() {
    let mut q = Queue::new();
    q.upsert(0x34, "Guard the Edge", Body::copied("  0/0 · Streak 0 · Best 0"), u16::MAX, DEFAULT_COLOR);
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Guard the Edge", "  0/0 · Streak 0 · Best 0"));
    q.post_refusal("Input Recording", Body::copied("Stop the drill to start recording or play inputs"), 120, DEFAULT_COLOR);
    for n in 1..=120 {
        if n % 30 == 0 {
            q.upsert(0x34, "Guard the Edge", Body::copied(&format!("status {n}")), u16::MAX, DEFAULT_COLOR);
        }
        let out = q.tick(true, false);
        assert!(out.visible);
        if n == 1 {
            let draw = out.draw.unwrap();
            assert_eq!((draw.header, draw.body.as_str()), ("Input Recording", "Stop the drill to start recording or play inputs"));
        } else {
            assert!(out.draw.is_none(), "tick {n}");
        }
    }
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Guard the Edge", "status 120"));
    assert!(q.is_queued(0x34));
}

fn a_refused_save_or_load_shows_over_a_standing_notice_and_gives_it_back() {
    use model::state::refused::{Action, Refused};
    let mut q = Queue::new();
    let status = "0/0 · Streak 0 · Best 0";
    q.upsert(0x34, "Guard the Edge", Body::copied(status), 600, DEFAULT_COLOR);
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Guard the Edge", status));
    for _ in 1..10 { assert!(q.tick(true, false).visible); }

    let text = Refused::Empty.notice(Action::Load).unwrap();
    q.post_refusal("Save State", Body::copied(text), 150, DEFAULT_COLOR);
    for tick in 0..150 {
        let out = q.tick(true, false);
        assert!(out.visible);
        if tick == 0 {
            let draw = out.draw.unwrap();
            assert_eq!((draw.header, draw.body.as_str()), ("Save State", text));
        } else {
            assert!(out.draw.is_none());
        }
    }
    let out = q.tick(true, false);
    assert!(out.visible);
    let draw = out.draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Guard the Edge", status));
    for _ in 1..590 {
        let out = q.tick(true, false);
        assert!(out.visible && out.draw.is_none());
    }
    assert!(q.is_empty());
    assert_eq!(q.take_delivered(), Some(0x34));
    assert!(!q.tick(true, false).visible);
}

fn a_refusal_keeps_plain_order_and_passes_every_standing_notice() {
    let before = ALLOCS.with(Cell::get);
    let mut q = Queue::new();
    q.post_static("Save State", "Saved", 3, DEFAULT_COLOR);
    assert_eq!(q.tick(true, false).draw.unwrap().header, "Save State");
    q.upsert(7, "S", Body::copied("standing"), 5, DEFAULT_COLOR);
    q.post_refusal("Input Recording", Body::copied("no"), 2, DEFAULT_COLOR);
    for _ in 0..2 { assert!(q.tick(true, false).draw.is_none()); }
    assert_eq!(q.tick(true, false).draw.unwrap().header, "Input Recording");
    assert!(q.tick(true, false).draw.is_none());
    assert_eq!(q.tick(true, false).draw.unwrap().header, "S");

    let mut q = Queue::new();
    q.upsert(7, "S", Body::copied("standing"), 50, DEFAULT_COLOR);
    assert_eq!(q.tick(true, false).draw.unwrap().header, "S");
    q.show_now(9, "Game speed", Body::copied("1/2"), 30, DEFAULT_COLOR);
    assert_eq!(q.tick(true, false).draw.unwrap().header, "Game speed");
    q.post_refusal("Input Recording", Body::copied("no"), 2, DEFAULT_COLOR);
    assert_eq!(q.tick(true, false).draw.unwrap().header, "Input Recording");
    assert!(q.tick(true, false).draw.is_none());
    assert_eq!(q.tick(true, false).draw.unwrap().header, "Game speed");
    for _ in 4..=31 { assert!(q.tick(true, false).draw.is_none()); }
    assert_eq!(q.tick(true, false).draw.unwrap().header, "S");
    assert_eq!(q.take_delivered(), Some(9));
    for _ in 0..48 {
        let out = q.tick(true, false);
        assert!(out.visible && out.draw.is_none());
    }
    assert!(q.is_empty());
    assert_eq!(q.take_delivered(), Some(7));

    let mut q = Queue::new();
    for id in 0..CAPACITY as u8 {
        q.upsert(id, "S", Body::copied("standing"), 5, DEFAULT_COLOR);
    }
    let header = q.front_header();
    q.post_refusal("Input Recording", Body::copied("no"), 2, DEFAULT_COLOR);
    assert_eq!(q.len(), CAPACITY);
    assert_eq!(q.front_header(), header);
    assert_eq!(ALLOCS.with(Cell::get), before);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("startup_notices_wait_for_training_and_settings_and_do_not_repeat_after_clear", startup_notices_wait_for_training_and_settings_and_do_not_repeat_after_clear as fn()),
        ("startup_displays_current_chords_in_order_for_exact_visible_durations", startup_displays_current_chords_in_order_for_exact_visible_durations as fn()),
        ("longest_chords_survive_queue_utf16_wrap_and_preallocated_pane_boundaries", longest_chords_survive_queue_utf16_wrap_and_preallocated_pane_boundaries as fn()),
        ("full_queue_drops_the_oldest_waiting_never_the_one_on_screen", full_queue_drops_the_oldest_waiting_never_the_one_on_screen as fn()),
        ("maximum_inline_bodies_continue_through_redraw_update_and_delivery_without_heap", maximum_inline_bodies_continue_through_redraw_update_and_delivery_without_heap as fn()),
        ("copied_body_matches_single_part_joined", copied_body_matches_single_part_joined as fn()),
        ("copied_multibyte_body_truncates_at_a_complete_character_without_heap", copied_multibyte_body_truncates_at_a_complete_character_without_heap as fn()),
        ("pane_rules_fixture", pane_rules_fixture as fn()),
        ("missing_accessor_zero", missing_accessor_zero as fn()),
        ("the_standing_api_is_bounded_and_allocation_free", the_standing_api_is_bounded_and_allocation_free as fn()),
        ("a_refusal_shows_at_once_over_the_guard_wizard_page_and_gives_it_back", a_refusal_shows_at_once_over_the_guard_wizard_page_and_gives_it_back as fn()),
        ("a_refused_juggler_player_point_shows_over_its_page_once_per_press", a_refused_juggler_player_point_shows_over_its_page_once_per_press as fn()),
        ("a_refusal_during_a_running_drill_shows_over_the_drill_status", a_refusal_during_a_running_drill_shows_over_the_drill_status as fn()),
        ("a_refused_save_or_load_shows_over_a_standing_notice_and_gives_it_back", a_refused_save_or_load_shows_over_a_standing_notice_and_gives_it_back as fn()),
        ("a_refusal_keeps_plain_order_and_passes_every_standing_notice", a_refusal_keeps_plain_order_and_passes_every_standing_notice as fn()),
    ]);
}
