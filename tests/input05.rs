//! SPEC-input-05 §9 host rows: the notification queue and the damage display helper.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use model::damage_display::*;
use model::notify::{Body, Queue, BODY_MAX, CAPACITY, DEFAULT_COLOR};

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

#[test]
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

#[test]
fn first_draw_writes() {
    let mut q = Queue::new();
    q.post_static("Input Recording", "Standby", 5, [200, 8, 200, 255]);
    let first = q.tick(true, false).draw.unwrap();
    assert_eq!((first.header, first.body.as_str(), first.color), ("Input Recording", "Standby", [200, 8, 200, 255]));
    for _ in 0..3 {
        assert_eq!(q.tick(true, false).draw, None, "text and colour are written on the first draw only");
    }
}

#[test]
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

#[test]
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

#[test]
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

#[test]
fn copied_bodies_are_bounded_and_char_safe() {
    let long = "é".repeat(BODY_MAX);
    let body = Body::copied(&long);
    assert!(body.as_str().len() <= BODY_MAX);
    assert!(body.as_str().chars().all(|c| c == 'é'));
    assert_eq!(Body::copied("hello").as_str(), "hello");
}

#[test]
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

#[test]
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

#[test]
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

#[test]
fn missing_accessor_zero() {
    assert_eq!(digits_or_none(None), None, "nothing is written for a missing fighter");
    assert_eq!(digits_or_none(Some(12.3)), Some(Digits { hundreds: 0, tens: 1, ones: 2, tenths: 3 }));
}

fn drawn_bodies(q: &mut Queue, ticks: usize) -> Vec<String> {
    (0..ticks).filter_map(|_| q.tick(true, false).draw.map(|d| d.body.as_str().to_owned())).collect()
}

#[test]
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

#[test]
fn a_plain_notice_posted_behind_a_drawn_standing_one_waits_for_it() {
    let mut q = Queue::new();
    q.upsert(1, "S", Body::copied("standing"), 3, DEFAULT_COLOR);
    assert_eq!(drawn_bodies(&mut q, 1), ["standing"]);
    q.post_static("H", "plain", 2, DEFAULT_COLOR);
    assert_eq!(drawn_bodies(&mut q, 4), ["plain"]);
    assert!(!q.is_queued(1));
    assert_eq!(q.take_delivered(), Some(1));
}

#[test]
fn delivery_needs_the_whole_visible_time() {
    let mut q = Queue::new();
    q.upsert(3, "S", Body::copied("x"), 3, DEFAULT_COLOR);
    // While the menu covers the pane the countdown stops (decision B8 question 3).
    for _ in 0..5 {
        assert_eq!(q.tick(true, true).visible, false);
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

#[test]
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

#[test]
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