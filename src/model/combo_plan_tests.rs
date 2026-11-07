//! SPEC-input-07 §7: what the bar shows, and the strip's write-on-change rule.

use super::*;
use crate::model::combo::{Aim, Combo, Event, Settings};
use crate::model::frame_strip::Move;

fn reference() -> Combo {
    let link = LinkRec::presses(
        Move::Jab,
        Move::UpTilt,
        Verdict::True(Some(-3)),
        Some((0, 4)),
        &[Event::press(Button::Attack, Aim::Up, 1)],
    );
    Combo::from_links([link, LinkRec { from: Move::UpTilt, ..link }]).unwrap()
}

fn helper(on: bool, target: bool) -> Helper {
    let mut h = Helper::new();
    if target {
        h.set_target(Some(reference()));
    }
    // The settings the bar reads come with each step; a fresh tracker step leaves everything else as it is.
    h.step(&crate::model::frame_strip::Tracker::new(), Settings { on, ..Settings::factory() });
    h
}

fn draw(r: &mut Renderer, h: &Helper, shown: bool) -> Vec<(usize, Look)> {
    let mut writes = Vec::new();
    r.draw(h, &Text::new("Jab loop"), shown, |i, look, _| writes.push((i, *look)));
    writes
}

#[test]
fn the_bar_is_hidden_without_the_strip_the_helper_or_a_target() {
    for (on, target, strip) in [(false, true, true), (true, false, true), (true, true, false)] {
        let mut r = Renderer::new();
        let writes = draw(&mut r, &helper(on, target), strip);
        assert_eq!(writes.len(), 1, "only the container");
        assert!(!writes[0].1.visible);
        assert!(draw(&mut r, &helper(on, target), strip).is_empty(), "and only once");
    }
}

#[test]
fn shown_it_writes_every_pane_once_and_then_only_changes() {
    let mut r = Renderer::new();
    let h = helper(true, true);
    let writes = draw(&mut r, &h, true);
    assert_eq!(writes.len(), COUNT);
    let at = |i: usize| writes.iter().find(|(j, _)| *j == i).unwrap().1;
    assert!(at(at::CONTAINER).visible);
    assert_eq!(at(at::HEADER).text, Text::new("Jab loop"));
    assert_eq!(at(at::RESULT).text, Text::new("ready: 2 links"));
    assert_eq!(at(at::TITLE).text, Text::new("1 Jab>Up tilt"));
    assert_eq!(at(at::VERDICT).text, Text::new("TRUE -3"), "the recorded verdict, before an attempt");
    assert!(!at(at::TITLE + 2).visible, "a two-link combo uses two cells");
    assert!(!at(at::PILL).visible && !at(at::YOURS).visible);
    // The recorded press is one frame after A1 on the ruler, the window 0..=4.
    assert_eq!(at(at::RECORDED).x, ruler_x(0, 1));
    assert_eq!(at(at::WINDOW).w, 5.0 * PER_FRAME);
    assert!(draw(&mut r, &h, true).is_empty(), "nothing changed");
    r.invalidate();
    assert_eq!(draw(&mut r, &h, true).len(), COUNT);
}

#[test]
fn texts_name_verdicts_marks_results_and_reasons() {
    assert_eq!(verdict_text(Verdict::True(Some(-2))), Text::new("TRUE -2"));
    assert_eq!(verdict_text(Verdict::True(None)), Text::new("TRUE"));
    assert_eq!(verdict_text(Verdict::Drop(1)), Text::new("DROP +1"));
    assert_eq!(mark_text(Mark::At(1), "Jump", 1).0, Text::new("on time"));
    assert_eq!(mark_text(Mark::At(2), "Jump", 1), (Text::new("Jump 2f late"), WARN));
    assert_eq!(mark_text(Mark::At(-3), "Attack", 0).0, Text::new("Attack 3f early"));
    assert_eq!(mark_text(Mark::Missing, "Jump", 2), (Text::new("Jump missing"), BAD));
    let s = |outcome, true_links, clean, fault| Summary { outcome, graded: 4, true_links, total: 4, clean, fault };
    assert_eq!(result_text(&s(Outcome::Completed, 4, true, None)), Text::new("clean, 4 of 4"));
    assert_eq!(result_text(&s(Outcome::Completed, 3, false, None)), Text::new("3 of 4 true"));
    assert_eq!(result_text(&s(Outcome::Stopped, 2, false, None)), Text::new("stopped: 2 of 4"));
    assert_eq!(reason_text(&s(Outcome::Completed, 3, false, Some((2, Reason::Drop(1))))), Text::new("L3 DROP +1"));
    assert_eq!(reason_text(&s(Outcome::Completed, 3, false, Some((0, Reason::Off(Button::Jump, -2))))), Text::new("L1 Jump 2f early"));
    assert_eq!(reason_text(&s(Outcome::Completed, 3, false, Some((1, Reason::WrongMove(Move::Aerial))))), Text::new("L2 wrong move: Aerial"));
}

#[test]
fn the_cells_follow_the_newest_links() {
    assert_eq!(first_cell(0, 2), 0);
    assert_eq!(first_cell(0, 9), 0);
    assert_eq!(first_cell(4, 9), 1);
    assert_eq!(first_cell(8, 9), 5);
    assert_eq!(first_cell(9, 9), 5);
    assert_eq!(ruler_x(1, -50), ruler_x(1, RULER_FROM), "clamped to the ruler");
}
