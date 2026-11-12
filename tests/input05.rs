//! SPEC-input-05 §9 host rows: the notification queue and the damage display helper.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::alloc::{GlobalAlloc, Layout, System};
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

fn save_notice_replaces_old_save_and_preserves_other_notices() {
    let mut queue = Queue::new();
    queue.post_static("Save State", "old", 120, DEFAULT_COLOR);
    queue.post_static("Other", "keep", 1, DEFAULT_COLOR);
    model::boot_notices::saved(&mut queue, 4);
    assert_eq!(queue.len(), 2);
    assert_eq!(queue.tick(true, false).draw.unwrap().header, "Other");
    let draw = queue.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Save State", "Saved Slot 5"));
    for _ in 1..120 {
        assert!(queue.tick(true, false).visible);
    }
    assert!(!queue.tick(true, false).visible);
    for (slot, text) in ["Saved Slot 1", "Saved Slot 2", "Saved Slot 3", "Saved Slot 4", "Saved Slot 5"].iter().enumerate() {
        model::boot_notices::saved(&mut queue, slot);
        assert_eq!(queue.tick(true, false).draw.unwrap().body.as_str(), *text);
        queue.clear();
    }
    model::boot_notices::saved(&mut queue, usize::MAX);
    assert!(queue.is_empty());
}

fn disabled_optional_shortcuts_do_not_hide_the_permanent_menu_chord() {
    use model::menu::chord::Chord;
    let mut chords = model::chords::ChordSet::factory();
    chords.shortcuts = model::menu_shortcuts::MenuShortcuts::new(Chord::NONE, Chord::NONE);
    let mut queue = Queue::new();
    model::boot_notices::BootNotices::new().post_once(true, Some(chords), &mut queue);
    for _ in 0..60 { queue.tick(true, false); }
    assert_eq!(queue.tick(true, false).draw.unwrap().body.as_str(), "B + D-pad Up");
}

fn startup_and_save_notices_allocate_nothing_on_first_and_repeated_use() {
    use model::menu::chord::Chord;
    let mut chords = model::chords::ChordSet::factory();
    chords.shortcuts = model::menu_shortcuts::MenuShortcuts::new(Chord(1), Chord(2));
    let mut queue = Queue::new();
    let mut boot = model::boot_notices::BootNotices::new();
    let before = ALLOCS.with(Cell::get);
    assert!(boot.post_once(true, Some(chords), &mut queue));
    assert!(!boot.post_once(true, Some(chords), &mut queue));
    for slot in [0, 4, 0] {
        model::boot_notices::saved(&mut queue, slot);
    }
    while !queue.is_empty() { queue.tick(true, false); }
    assert_eq!(ALLOCS.with(Cell::get), before);
}

fn longest_chords_survive_queue_utf16_wrap_and_preallocated_pane_boundaries() {
    use model::{boot_notices, menu::chord::Chord, panes};
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
            for (&unit, byte) in units[..len].iter().zip(expected.bytes()) {
                assert!(unit == u16::from(byte) || (byte == b' ' && unit == u16::from(b'\n')));
            }
            let mut lines = 0;
            for line in units[..len].split(|unit| *unit == u16::from(b'\n')) {
                assert!(line.len() <= boot_notices::LONG_COLUMNS);
                lines += 1;
            }
            assert!(lines <= 7, "all lines must fit the 180-unit pane at 24-unit font height");
            for _ in 1..120 { assert!(queue.tick(true, false).visible); }
        }
        assert!(!queue.tick(true, false).visible);
        assert_eq!(ALLOCS.with(Cell::get), before);
    }
    let pane = panes::spec(panes::IDX_NOTICE_LONG);
    let panes::Kind::Text { chars, font, .. } = pane.kind else { panic!("notice must be text") };
    assert!(usize::from(chars) > BODY_MAX, "preallocate the complete body and terminator");
    assert!(font.0 * boot_notices::LONG_COLUMNS as f32 <= pane.w);
    assert!(font.1 * 7.0 <= pane.h);
}

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static A: Counting = Counting;

fn queue_order_and_lengths() {
    let mut q = Queue::new();
    q.post_static("H", "first", 3, [1, 2, 3, 255]);
    q.post_static("H", "second", 2, DEFAULT_COLOR);
    // One visible at a time, in order; the first lasts exactly its length in ticks.
    let mut shown = Vec::new();
    for _ in 0..6 {
        let out = q.tick(true, false);
        shown.push((out.visible, out.draw.map(|d| d.body.as_str().to_owned())));
    }
    assert_eq!(
        shown,
        [
            (true, Some("first".to_owned())),
            (true, None),
            (true, None),
            (true, Some("second".to_owned())),
            (true, None),
            (false, None),
        ]
    );
    assert!(q.is_empty());
    // A post of length 0 is treated as 1 (question 1): it draws once and never underflows.
    q.post_static("H", "zero", 0, DEFAULT_COLOR);
    assert_eq!(q.tick(true, false).draw.map(|d| d.body.as_str().to_owned()), Some("zero".to_owned()));
    assert!(q.is_empty());
}

fn first_draw_writes() {
    let mut q = Queue::new();
    q.post_static("Input Recording", "Standby", 5, [200, 8, 200, 255]);
    let first = q.tick(true, false).draw.unwrap();
    assert_eq!((first.header, first.body.as_str(), first.color), ("Input Recording", "Standby", [200, 8, 200, 255]));
    for _ in 0..3 {
        assert_eq!(q.tick(true, false).draw, None, "text and colour are written on the first draw only");
    }
}

fn clear_by_header_and_all() {
    let mut q = Queue::new();
    q.post_static("A", "1", 10, DEFAULT_COLOR);
    q.post_static("B", "2", 10, DEFAULT_COLOR);
    q.post_static("A", "3", 10, DEFAULT_COLOR);
    q.post_static("C", "4", 10, DEFAULT_COLOR);
    q.clear_header("A");
    assert_eq!(q.len(), 2);
    assert_eq!(q.front_header(), Some("B"));
    q.clear();
    assert!(q.is_empty());
    q.post_static("A", "1", 10, DEFAULT_COLOR);
    q.clear_header("Z");
    assert_eq!(q.len(), 1);
}

fn hidden_and_combo_display_off() {
    let mut q = Queue::new();
    q.post_static("H", "x", 5, DEFAULT_COLOR);
    // The mod menu hides the slot and (B8, question 3) stops the countdown.
    for _ in 0..50 {
        let out = q.tick(true, true);
        assert!(!out.visible && out.draw.is_none());
    }
    assert_eq!(q.len(), 1);
    // Closed: it shows for its full length.
    let ticks = (0..10).filter(|_| q.tick(true, false).visible).count();
    assert_eq!(ticks, 5);
    // Combo Display off empties the queue at once.
    q.post_static("H", "y", 5, DEFAULT_COLOR);
    let out = q.tick(false, false);
    assert!(!out.visible);
    assert!(q.is_empty());
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

fn copied_bodies_are_bounded_and_char_safe() {
    let long = "é".repeat(BODY_MAX);
    let body = Body::copied(&long);
    assert!(body.as_str().len() <= BODY_MAX);
    assert!(body.as_str().chars().all(|c| c == 'é'));
    assert_eq!(Body::copied("hello").as_str(), "hello");
}

// Literal fixture lengths are independent of the production truncation loop.
const FULL_STATIC_BODY: &str = concat!(
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
);
const SPLIT_STATIC_BODY: &str = concat!(
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "é!",
);

fn inline_body_preserves_copy_capacity_and_complete_static_notice() {
    assert_eq!(BODY_MAX, 320, "complete configured chords retain their capacity");
    assert_eq!(FULL_STATIC_BODY.len(), 320);
    assert!(std::mem::size_of::<Body>() <= 328, "must not grow beyond the former enum");
    let before = ALLOCS.with(Cell::get);
    let mut q = Queue::new();
    q.post_static("Full", FULL_STATIC_BODY, 1, DEFAULT_COLOR);
    let draw = q.tick(true, false).draw.unwrap();
    let copied_twice = (draw.body, draw.body);
    assert_eq!(copied_twice.0, copied_twice.1);
    assert_eq!(draw.body.as_str(), FULL_STATIC_BODY);
    assert!(q.is_empty());
    assert_eq!(ALLOCS.with(Cell::get), before);
}

fn static_utf8_cut_preserves_next_notice_and_visible_time() {
    assert_eq!(SPLIT_STATIC_BODY.len(), 322);
    // 319 ASCII bytes then a two-byte character straddling the 320-byte edge.
    assert_eq!(&SPLIT_STATIC_BODY[319..], "é!");
    let before = ALLOCS.with(Cell::get);
    let mut q = Queue::new();
    q.post_static("Cut", SPLIT_STATIC_BODY, 2, DEFAULT_COLOR);
    q.post_static("Next", "é complete", 1, DEFAULT_COLOR);
    assert!(!q.tick(true, true).visible);
    let draw = q.tick(true, false).draw.unwrap();
    assert_eq!(draw.body.as_str(), &SPLIT_STATIC_BODY[..319]);
    assert!(q.tick(true, false).draw.is_none());
    let next = q.tick(true, false).draw.unwrap();
    assert_eq!((next.header, next.body.as_str()), ("Next", "é complete"));
    assert!(q.is_empty());
    assert_eq!(ALLOCS.with(Cell::get), before);
}

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

fn tick_allocates_nothing() {
    let mut q = Queue::new();
    let before = ALLOCS.with(Cell::get);
    for n in 0..2000u32 {
        q.clear_header("Input Recording");
        q.post_static("Input Recording", "Recording", 60, [200, 8, 8, 255]);
        if n % 7 == 0 {
            q.post(
                "Copy",
                Body::copied("Recording is 601 frames; a Record Slot holds 600."),
                3,
                DEFAULT_COLOR,
            );
        }
        q.tick(true, n % 11 == 0);
    }
    assert_eq!(ALLOCS.with(Cell::get), before);
}

// ---- damage display ------------------------------------------------------------------------------

fn damage_digits() {
    assert_eq!(digits(123.4), Digits { hundreds: 1, tens: 2, ones: 3, tenths: 4 });
    assert_eq!(digits(7.0), Digits { hundreds: 0, tens: 0, ones: 7, tenths: 0 });
    assert_eq!(digits(0.0), Digits::ZERO);
    assert_eq!(digits(99.9), Digits { hundreds: 0, tens: 9, ones: 9, tenths: 9 });
    assert_eq!(digits(999.9), Digits { hundreds: 9, tens: 9, ones: 9, tenths: 9 });
    assert_eq!(digits(5000.0), Digits { hundreds: 9, tens: 9, ones: 9, tenths: 9 });
    assert_eq!(digits(-3.0), Digits::ZERO);
    assert_eq!(digits(f32::NAN), Digits::ZERO);
    assert_eq!(digits(10.1), Digits { hundreds: 0, tens: 1, ones: 0, tenths: 1 });
}

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

fn drawn_bodies(q: &mut Queue, ticks: usize) -> Vec<String> {
    (0..ticks).filter_map(|_| q.tick(true, false).draw.map(|d| d.body.as_str().to_owned())).collect()
}

fn a_standing_notice_waits_behind_plain_ones_and_is_updated_in_place() {
    let mut q = Queue::new();
    q.upsert(7, "Ready", Body::copied("one"), 3, DEFAULT_COLOR);
    q.post_static("H", "plain", 2, DEFAULT_COLOR);
    // The plain notice goes ahead of the standing one that has not been drawn yet.
    assert_eq!(drawn_bodies(&mut q, 2), ["plain"]);
    assert!(q.is_queued(7));
    // An update replaces text and length, keeps the one entry and redraws.
    q.upsert(7, "Ready", Body::copied("two"), 2, DEFAULT_COLOR);
    assert_eq!(q.len(), 1);
    assert_eq!(drawn_bodies(&mut q, 1), ["two"]);
    q.upsert(7, "Ready", Body::copied("three"), 2, DEFAULT_COLOR);
    assert_eq!(drawn_bodies(&mut q, 1), ["three"], "an update of the entry on screen redraws it");
    assert_eq!(q.len(), 1);
}

fn a_plain_notice_posted_behind_a_drawn_standing_one_waits_for_it() {
    let mut q = Queue::new();
    q.upsert(1, "S", Body::copied("standing"), 3, DEFAULT_COLOR);
    assert_eq!(drawn_bodies(&mut q, 1), ["standing"]);
    q.post_static("H", "plain", 2, DEFAULT_COLOR);
    assert_eq!(drawn_bodies(&mut q, 4), ["plain"]);
    assert!(!q.is_queued(1));
    assert_eq!(q.take_delivered(), Some(1));
}

fn delivery_needs_the_whole_visible_time() {
    let mut q = Queue::new();
    q.upsert(3, "S", Body::copied("x"), 3, DEFAULT_COLOR);
    // While the menu covers the pane the countdown stops (decision B8 question 3).
    for _ in 0..5 {
        assert!(!q.tick(true, true).visible);
    }
    assert!(q.is_queued(3));
    assert_eq!(q.take_delivered(), None);
    for _ in 0..3 {
        q.tick(true, false);
    }
    assert!(!q.is_queued(3));
    assert_eq!(q.take_delivered(), Some(3));
    assert_eq!(q.take_delivered(), None, "a delivery is taken once");
}

fn clear_and_session_clear_deliver_nothing_and_an_update_drops_an_old_record() {
    let mut q = Queue::new();
    q.upsert(4, "S", Body::copied("x"), 2, DEFAULT_COLOR);
    q.clear_id(4);
    assert!(!q.is_queued(4));
    assert_eq!(q.take_delivered(), None, "a cleared notice was not seen");
    // Session clear wipes the queue but keeps a record that already exists.
    q.upsert(5, "S", Body::copied("y"), 1, DEFAULT_COLOR);
    q.tick(true, false);
    q.upsert(6, "S", Body::copied("z"), 5, DEFAULT_COLOR);
    q.clear();
    assert!(!q.is_queued(6));
    assert_eq!(q.take_delivered(), Some(5));
    // New content of an id forgets its earlier delivery.
    q.upsert(5, "S", Body::copied("y"), 1, DEFAULT_COLOR);
    q.tick(true, false);
    q.upsert(5, "S", Body::copied("again"), 4, DEFAULT_COLOR);
    assert_eq!(q.take_delivered(), None);
    // Clearing an id clears its record too.
    q.upsert(8, "S", Body::copied("w"), 1, DEFAULT_COLOR);
    q.tick(true, false);
    q.clear_id(8);
    assert_eq!(q.take_delivered(), None);
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
fn a_registered_layout_draws_the_showing_notice_again() {
    let mut q = Queue::new();
    q.post_static("H", "text", 5, DEFAULT_COLOR);
    assert!(q.tick(true, false).draw.is_some());
    assert_eq!(q.tick(true, false).draw, None);
    q.on_layout_registered();
    assert_eq!(q.tick(true, false).draw.map(|d| d.body.as_str().to_owned()), Some("text".to_owned()));
    assert_eq!(q.tick(true, false).draw, None);
    // Nothing queued: nothing to draw.
    let mut empty = Queue::new();
    empty.on_layout_registered();
    assert!(!empty.tick(true, false).visible);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("startup_notices_wait_for_training_and_settings_and_do_not_repeat_after_clear", startup_notices_wait_for_training_and_settings_and_do_not_repeat_after_clear as fn()),
        ("startup_displays_current_chords_in_order_for_exact_visible_durations", startup_displays_current_chords_in_order_for_exact_visible_durations as fn()),
        ("save_notice_replaces_old_save_and_preserves_other_notices", save_notice_replaces_old_save_and_preserves_other_notices as fn()),
        ("disabled_optional_shortcuts_do_not_hide_the_permanent_menu_chord", disabled_optional_shortcuts_do_not_hide_the_permanent_menu_chord as fn()),
        ("startup_and_save_notices_allocate_nothing_on_first_and_repeated_use", startup_and_save_notices_allocate_nothing_on_first_and_repeated_use as fn()),
        ("longest_chords_survive_queue_utf16_wrap_and_preallocated_pane_boundaries", longest_chords_survive_queue_utf16_wrap_and_preallocated_pane_boundaries as fn()),
        ("queue_order_and_lengths", queue_order_and_lengths as fn()),
        ("first_draw_writes", first_draw_writes as fn()),
        ("clear_by_header_and_all", clear_by_header_and_all as fn()),
        ("hidden_and_combo_display_off", hidden_and_combo_display_off as fn()),
        ("full_queue_drops_the_oldest_waiting_never_the_one_on_screen", full_queue_drops_the_oldest_waiting_never_the_one_on_screen as fn()),
        ("copied_bodies_are_bounded_and_char_safe", copied_bodies_are_bounded_and_char_safe as fn()),
        ("inline_body_preserves_copy_capacity_and_complete_static_notice", inline_body_preserves_copy_capacity_and_complete_static_notice as fn()),
        ("static_utf8_cut_preserves_next_notice_and_visible_time", static_utf8_cut_preserves_next_notice_and_visible_time as fn()),
        ("maximum_inline_bodies_continue_through_redraw_update_and_delivery_without_heap", maximum_inline_bodies_continue_through_redraw_update_and_delivery_without_heap as fn()),
        ("tick_allocates_nothing", tick_allocates_nothing as fn()),
        ("damage_digits", damage_digits as fn()),
        ("pane_rules_fixture", pane_rules_fixture as fn()),
        ("missing_accessor_zero", missing_accessor_zero as fn()),
        ("a_standing_notice_waits_behind_plain_ones_and_is_updated_in_place", a_standing_notice_waits_behind_plain_ones_and_is_updated_in_place as fn()),
        ("a_plain_notice_posted_behind_a_drawn_standing_one_waits_for_it", a_plain_notice_posted_behind_a_drawn_standing_one_waits_for_it as fn()),
        ("delivery_needs_the_whole_visible_time", delivery_needs_the_whole_visible_time as fn()),
        ("clear_and_session_clear_deliver_nothing_and_an_update_drops_an_old_record", clear_and_session_clear_deliver_nothing_and_an_update_drops_an_old_record as fn()),
        ("the_standing_api_is_bounded_and_allocation_free", the_standing_api_is_bounded_and_allocation_free as fn()),
        ("a_registered_layout_draws_the_showing_notice_again", a_registered_layout_draws_the_showing_notice_again as fn()),
    ]);
}
