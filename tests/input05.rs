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
