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

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("startup_notices_wait_for_training_and_settings_and_do_not_repeat_after_clear", startup_notices_wait_for_training_and_settings_and_do_not_repeat_after_clear as fn()),
        ("startup_displays_current_chords_in_order_for_exact_visible_durations", startup_displays_current_chords_in_order_for_exact_visible_durations as fn()),
        ("longest_chords_survive_queue_utf16_wrap_and_preallocated_pane_boundaries", longest_chords_survive_queue_utf16_wrap_and_preallocated_pane_boundaries as fn()),
        ("full_queue_drops_the_oldest_waiting_never_the_one_on_screen", full_queue_drops_the_oldest_waiting_never_the_one_on_screen as fn()),
        ("maximum_inline_bodies_continue_through_redraw_update_and_delivery_without_heap", maximum_inline_bodies_continue_through_redraw_update_and_delivery_without_heap as fn()),
        ("pane_rules_fixture", pane_rules_fixture as fn()),
        ("missing_accessor_zero", missing_accessor_zero as fn()),
        ("the_standing_api_is_bounded_and_allocation_free", the_standing_api_is_bounded_and_allocation_free as fn()),
    ]);
}
