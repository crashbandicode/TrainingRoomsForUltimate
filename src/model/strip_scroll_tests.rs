lib_unit_tests! {
use super::*;
use crate::model::chords::{logical, ChordSet, Detector, Sample};
use logical::{DPAD_LEFT as LEFT, DPAD_RIGHT as RIGHT, STICK_R as R3};

fn bits(buttons: &[u8]) -> u16 {
    buttons.iter().fold(0u16, |set, button| set | (1 << button))
}

fn sample(held: &[u8], pressed: &[u8]) -> Sample {
    Sample { held: bits(held), pressed: bits(pressed), ..Sample::default() }
}

#[test]
fn r3_alone_hides_nothing_and_opens_nothing() {
    let mut scroll = Scroll::new();
    assert_eq!(scroll.poll(sample(&[R3], &[R3]), false, true), ScrollEvent::Idle);
    assert!(!scroll.is_open());
    assert_eq!(scroll.hidden(), 0);
    assert_eq!(scroll.poll(sample(&[LEFT], &[LEFT]), false, true), ScrollEvent::Idle);
    let mut detector = Detector::new();
    detector.poll(&ChordSet::factory(), sample(&[R3], &[R3]).with_strip(true), false);
    assert_eq!(detector.hidden(), 0);
    assert_eq!(detector.scroll_event(), ScrollEvent::Idle);
    assert_eq!(scroll.poll(sample(&[R3, LEFT], &[LEFT]), false, true), ScrollEvent::Opened(Dir::Back));
}

#[test]
fn a_held_direction_repeats() {
    let mut scroll = Scroll::new();
    assert_eq!(scroll.poll(sample(&[R3, LEFT], &[LEFT]), false, true), ScrollEvent::Opened(Dir::Back));
    for poll in 1..=40 {
        let expected = if [18, 24, 30, 36].contains(&poll) { ScrollEvent::Step(Dir::Back) } else { ScrollEvent::Held };
        assert_eq!(scroll.poll(sample(&[R3, LEFT], &[]), false, true), expected, "poll {poll}");
    }
    assert_eq!(scroll.poll(sample(&[R3, RIGHT], &[RIGHT]), false, true), ScrollEvent::Step(Dir::Forward));
    for poll in 1..=18 {
        let expected = if poll == 18 { ScrollEvent::Step(Dir::Forward) } else { ScrollEvent::Held };
        assert_eq!(scroll.poll(sample(&[R3, RIGHT], &[]), false, true), expected);
    }
}

#[test]
fn letting_go_of_r3_returns_to_live_at_once() {
    let mut scroll = Scroll::new();
    assert_eq!(scroll.poll(sample(&[R3, LEFT], &[LEFT]), false, true), ScrollEvent::Opened(Dir::Back));
    assert_eq!(scroll.poll(sample(&[R3, RIGHT], &[RIGHT]), false, true), ScrollEvent::Step(Dir::Forward));
    assert_eq!(scroll.poll(sample(&[LEFT], &[]), false, true), ScrollEvent::Closed);
    assert!(!scroll.is_open());
    let mut detector = Detector::new();
    let chords = ChordSet::factory();
    detector.poll(&chords, sample(&[R3, LEFT], &[LEFT]).with_strip(true), false);
    detector.poll(&chords, sample(&[LEFT], &[]).with_strip(true), false);
    assert_eq!(detector.scroll_event(), ScrollEvent::Closed);
    assert!(!detector.scroll_open());
}

#[test]
fn a_step_back_stops_at_the_oldest_column() {
    assert_eq!(stepped(None, Dir::Back, 700, 600), Some(685));
    let mut anchor = None;
    for _ in 0..100 { anchor = stepped(anchor, Dir::Back, 700, 600); }
    assert_eq!(anchor, Some(160));
    assert_eq!(stepped(anchor, Dir::Back, 700, 600), Some(160));
    assert_eq!(stepped(Some(100), Dir::Back, 700, 600), Some(160));
    assert_eq!(stepped(Some(690), Dir::Forward, 700, 600), None);
    for kept in [0, 59, 60] {
        assert_eq!(stepped(None, Dir::Back, 700, kept), None);
        assert_eq!(stepped(None, Dir::Forward, 700, kept), None);
    }
    assert_eq!(stepped(None, Dir::Back, 0, 0), None);
}

#[test]
fn the_scroll_runs_on_the_one_practice_poll() {
    let delay = include_str!("../runtime/features/input_delay.rs");
    let poll = delay.split("fn note(").nth(1).unwrap().split("\n}\n").next().unwrap();
    assert!(poll.contains("scene::is_training()"));
    assert!(poll.contains("chords::on_poll(controller)"));
    let chords = include_str!("../runtime/features/chords.rs");
    let poll = chords.split("pub unsafe fn on_poll(").nth(1).unwrap().split("\n}\n").next().unwrap();
    assert!(poll.contains("detector.scroll_event()"));
    assert!(poll.contains("input_display::on_scroll("));
    let display = include_str!("../runtime/features/input_display.rs");
    let scroll = display.split("pub fn on_scroll(").nth(1).unwrap().split("\n}\n").next().unwrap();
    assert!(!scroll.contains("scene::"));
}
}
