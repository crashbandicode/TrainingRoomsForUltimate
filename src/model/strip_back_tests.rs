lib_unit_tests! {
use super::*;
use crate::model::frame_strip::{Button, Move, Sample};
use crate::model::frame_strip_plan::{at, Look, Renderer, StripDraw, COUNT};
use crate::model::input_delay::MappedInputs;
use crate::model::panes::strip_layout::C;
use crate::model::spacing::{Grade, Taken};
use crate::model::strip_scroll::{Cursor, Dir, ScrollEvent};

fn idle() -> Sample {
    Sample { actionable: true, ..Sample::default() }
}

fn attack(mv: Move, status: i32, frame: u32) -> Sample {
    Sample { attack: Some(mv), status, motion_frame: frame as f32, ..Sample::default() }
}

fn busy(tracker: &mut Tracker, back: &mut Back, frames: usize) {
    for _ in 0..frames {
        tracker.step(Sample::default(), Sample::default(), MappedInputs::NEUTRAL);
        back.record(tracker);
    }
}

fn looks(tracker: &Tracker) -> Vec<Look> {
    let mut looks = vec![Look::HIDDEN; COUNT];
    Renderer::new().draw_with_spec(tracker,
        StripDraw { shown: true, back: None, spots: true, parry: None, history_enabled: true, cells: false, spec: &C },
        |index, look, _| looks[index] = *look);
    looks
}

#[test]
fn the_history_keeps_ten_seconds_of_columns_and_no_more() {
    let mut tracker = Tracker::new();
    let mut back = Back::new();
    busy(&mut tracker, &mut back, 700);
    assert_eq!(back.span(), (700, 600));
    let mut oldest = tracker;
    let mut boundary = tracker;
    assert_eq!(back.rewind(&mut oldest, 100), Some(540));
    assert_eq!(oldest.columns().next().unwrap().frame, 101);
    assert_eq!(oldest.columns().last().unwrap().frame, 160);
    assert_eq!(back.rewind(&mut boundary, 160), Some(540));
    assert_eq!(oldest.columns().collect::<Vec<_>>(), boundary.columns().collect::<Vec<_>>());
    assert!(core::mem::size_of::<Back>() <= 12 * 1024);
}

#[test]
fn a_missed_frame_is_caught_up() {
    let mut tracker = Tracker::new();
    let mut back = Back::new();
    busy(&mut tracker, &mut back, 5);
    for buttons in [1, 0, 2] {
        tracker.step(Sample::default(), Sample::default(), MappedInputs { buttons, ..MappedInputs::NEUTRAL });
    }
    back.record(&tracker);
    assert_eq!(back.span(), (8, 8));
    let mut view = tracker;
    assert_eq!(back.rewind(&mut view, 8), Some(0));
    assert_eq!(view.columns().map(|column| column.frame).collect::<Vec<_>>(), (1..=8).collect::<Vec<_>>());
    assert_eq!(view.presses().map(|press| (press.frame, press.button)).collect::<Vec<_>>(),
               [(8, Button::Special), (6, Button::Attack)]);
}

#[test]
fn a_view_of_the_past_draws_what_was_live_then() {
    let mut tracker = Tracker::new();
    let mut back = Back::new();
    busy(&mut tracker, &mut back, 60);
    let a = MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL };
    let up_a = MappedInputs { buttons: 1, lstick_y: 100, ..MappedInputs::NEUTRAL };
    for frame in 1..=26u32 {
        let (p1, input) = match frame {
            1 | 2 | 20 => (idle(), MappedInputs::NEUTRAL),
            3..=6 => (attack(Move::Jab, 0x27, frame - 3), if frame == 3 { a } else { MappedInputs::NEUTRAL }),
            7..=10 => (Sample { stop: true, active: true, ..attack(Move::Jab, 0x27, 4) }, MappedInputs::NEUTRAL),
            11..=19 => (Sample { active: frame == 11, ..attack(Move::Jab, 0x27, frame - 6) }, MappedInputs::NEUTRAL),
            21..=25 => (attack(Move::UpTilt, 0x2b, frame - 21), if frame == 21 { up_a } else { MappedInputs::NEUTRAL }),
            _ => (Sample { stop: true, active: true, ..attack(Move::UpTilt, 0x2b, 5) }, MappedInputs::NEUTRAL),
        };
        let cpu = match frame {
            0..=6 => idle(),
            7..=10 => Sample { stop: true, hitstun: true, reaction: Some(18.0), ..Sample::default() },
            26 => Sample { stop: true, hitstun: true, reaction: Some(30.0), ..Sample::default() },
            f if f < 29 => Sample { hitstun: true, reaction: Some(18.0 - (f - 11) as f32), ..Sample::default() },
            _ => idle(),
        };
        let taken = Taken { hits: [if frame == 26 { Some(Grade::Good) } else { None }, None], earlier: None };
        tracker.step_graded(p1, cpu, input, taken);
        back.record(&tracker);
    }
    assert!(tracker.history().next().is_some());
    let then = tracker;
    let anchor = back.span().0;
    busy(&mut tracker, &mut back, 200);
    assert_eq!(back.rewind(&mut tracker, anchor), Some(200));
    let (expected, actual) = (looks(&then), looks(&tracker));
    for index in (at::RUN..at::WINDOW_BAR)
        .chain(at::READ_VALUE + 1..at::READ_VALUE + 3)
        .chain(at::READ_SUB + 1..at::READ_SUB + 3)
        .chain(at::PILL..COUNT)
    {
        assert_eq!(actual[index], expected[index], "pane {index}");
    }
}

#[test]
fn the_view_stands_still_while_frames_arrive() {
    let mut tracker = Tracker::new();
    let mut back = Back::new();
    busy(&mut tracker, &mut back, 150);
    let anchor = 100;
    let mut first = tracker;
    assert_eq!(back.rewind(&mut first, anchor), Some(50));
    let columns: Vec<_> = first.columns().collect();
    busy(&mut tracker, &mut back, 30);
    let mut second = tracker;
    assert_eq!(back.rewind(&mut second, anchor), Some(80));
    assert_eq!(second.columns().collect::<Vec<_>>(), columns);
}

#[test]
fn a_cleared_strip_has_no_past() {
    let mut tracker = Tracker::new();
    let mut back = Back::new();
    busy(&mut tracker, &mut back, 100);
    tracker.clear();
    let mut cleared = tracker;
    assert_eq!(back.rewind(&mut cleared, 60), None);
    busy(&mut tracker, &mut back, 1);
    assert_eq!(back.span(), (1, 1));
    let mut fresh = tracker;
    assert_eq!(back.rewind(&mut fresh, 60), None);
}

#[test]
fn scrollback_review_each_timeline_reset_invalidates_the_published_cursor() {
    let native = include_str!("../runtime/features/input_display.rs");
    for boundary in ["fn session_clear()", "fn fighter_initialised()", "fn sample_frame("] {
        let body = native.split_once(boundary).unwrap().1.split_once("\n}").unwrap().0;
        assert!(body.contains("clear_live_strip(&mut"), "{boundary} must invalidate the shared cursor");
    }
    let clear = native.split_once("fn clear_live_strip(").unwrap().1.split_once("\n}").unwrap().0;
    assert!(clear.contains("SCROLL.clear()") && clear.contains("tracker.clear_live_strip("));
    let cursor = Cursor::new();
    let mut tracker = Tracker::new();
    let mut back = Back::new();
    for _ in 0..100 {
        tracker.step(Sample::default(), Sample::default(), MappedInputs::NEUTRAL);
        cursor.record(&mut back, &tracker);
    }
    cursor.publish_selection(ScrollEvent::Opened(Dir::Back), true);
    assert_eq!(cursor.anchor(), 85);
    tracker.clear();
    cursor.clear();
    // Miss enough records to overtake the old timeline's last frame: frame numbers alone cannot detect the reset.
    for _ in 0..105 { tracker.step(Sample::default(), Sample::default(), MappedInputs::NEUTRAL); }
    cursor.record(&mut back, &tracker);
    assert_eq!(back.span(), (60, 60));
    for _ in 0..30 {
        tracker.step(Sample::default(), Sample::default(), MappedInputs::NEUTRAL);
        cursor.record(&mut back, &tracker);
    }
    cursor.publish_selection(ScrollEvent::Held, true);
    assert_eq!(cursor.anchor(), 0, "the former column 85 exists again, but the view stays live");
    cursor.publish_selection(ScrollEvent::Step(Dir::Back), true);
    assert_eq!(cursor.anchor(), 75);
}
}
