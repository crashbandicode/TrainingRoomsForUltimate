//! SPEC-input-07 §7: what the bar shows, and the strip's write-on-change rule.

lib_unit_tests! {
use super::*;
use crate::model::combo::{Aim, Combo, Event, Settings};
use crate::model::drill::{Advance, Stepped};
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
    r.draw(h, &Text::new("Jab loop"), None, shown, |i, look, _| writes.push((i, *look)));
    writes
}

fn the_bar_is_hidden_without_the_strip_the_helper_or_a_target() {
    for (on, target, strip) in [(false, true, true), (true, false, true), (true, true, false)] {
        let mut r = Renderer::new();
        let writes = draw(&mut r, &helper(on, target), strip);
        assert_eq!(writes.len(), 1, "only the container");
        assert!(!writes[0].1.visible);
        assert!(draw(&mut r, &helper(on, target), strip).is_empty(), "and only once");
    }
}

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

// ---- the sequence bar (mockup `r8-sequence-bar`, Q8 A) ---------------------------------------------------------------

fn clean() -> Summary {
    Summary { outcome: Outcome::Completed, graded: 2, true_links: 2, total: 2, clean: true, fault: None }
}

fn view(at: usize, last: Option<Stepped>, last_name: &str) -> SeqView {
    let mut chips = [Text::EMPTY; CHIPS];
    chips[0] = Text::new("Ryu BnB");
    chips[1] = Text::new("Nair loop combo");
    SeqView {
        name: Text::new("Corner to kill"),
        at,
        steps: 2,
        streak: 4,
        best: 9,
        first: 0,
        chips,
        last,
        last_name: Text::new(last_name),
    }
}

fn draw_seq(h: &Helper, seq: &SeqView) -> Vec<Look> {
    let mut looks = vec![Look::HIDDEN; COUNT];
    Renderer::new().draw(h, &Text::new("Nair loop combo"), Some(seq), true, |i, look, _| looks[i] = *look);
    looks
}

fn a_drilled_sequence_draws_the_header_the_combo_row_and_the_helper_cells() {
    let mut h = helper(true, true);
    h.continue_with(reference());
    let next = Stepped { step: 0, summary: clean(), advance: Advance::Next(1) };
    let looks = draw_seq(&h, &view(1, Some(next), "Ryu BnB"));
    let text = |i: usize| {
        assert!(looks[i].visible, "pane {i} is shown");
        looks[i].text
    };
    assert_eq!(looks[at::CONTAINER].y, SEQ_BAR_Y, "the bar moves down by the sequence band");
    assert_eq!(looks[at::BACKDROP].w, SEQ_W, "as wide as the strip");
    assert!(looks[at::SEQ_BACKDROP].visible && looks[at::SEQ_PILL].visible);
    assert_eq!(looks[at::SEQ_PILL].color, SEQ_PILL_COLOR);
    assert_eq!(text(at::SEQ_PILL_TEXT), Text::new("SEQUENCE"));
    assert_eq!(text(at::SEQ_NAME), Text::new("Corner to kill"));
    assert_eq!(text(at::SEQ_STEP), Text::new("combo 2 of 2 ·"));
    assert_eq!(text(at::SEQ_COMBO), Text::new("Nair loop combo"));
    assert_eq!(text(at::SEQ_LINKS), Text::new("· 2 links"));
    assert_eq!(text(at::SEQ_TOLERANCE), Text::new("· tolerance ±1f"));
    assert_eq!((text(at::SEQ_STREAK), looks[at::SEQ_STREAK].color), (Text::new("streak 4"), GOOD));
    assert_eq!((text(at::SEQ_BEST), looks[at::SEQ_BEST].color), (Text::new("· best 9"), GOOD));
    let header = [at::SEQ_NAME, at::SEQ_STEP, at::SEQ_COMBO, at::SEQ_LINKS, at::SEQ_TOLERANCE];
    assert!(header.windows(2).all(|w| looks[w[0]].x < looks[w[1]].x), "left to right");
    assert!(looks[at::SEQ_TOLERANCE].x < looks[at::SEQ_STREAK].x);
    // Combo 1 is done (green, with the drawn check), combo 2 is current (white), and an arrow joins them.
    assert_eq!((text(at::SEQ_CHIP_TEXT), looks[at::SEQ_CHIP_EDGE].color), (Text::new("1  Ryu BnB"), GOOD));
    assert!(looks[at::SEQ_CHECK_SHORT].visible && looks[at::SEQ_CHECK_LONG].visible);
    assert_eq!((text(at::SEQ_CHIP_TEXT + 1), looks[at::SEQ_CHIP_EDGE + 1].color), (Text::new("2  Nair loop combo"), TEXT));
    assert!(!looks[at::SEQ_CHECK_SHORT + 1].visible && !looks[at::SEQ_CHIP_EDGE + 2].visible);
    assert_eq!(text(at::SEQ_ARROW), Text::new("→"));
    assert!(!looks[at::SEQ_ARROW + 1].visible);
    assert!(looks[at::SEQ_CHIP_EDGE].x < looks[at::SEQ_ARROW].x && looks[at::SEQ_ARROW].x < looks[at::SEQ_CHIP_EDGE + 1].x);
    // The footer: "Ryu BnB landed · Nair loop combo: ready: 2 links".
    assert_eq!(text(at::SEQ_LANDED), Text::new("Ryu BnB landed ·"));
    assert_eq!(text(at::SEQ_CURRENT), Text::new("Nair loop combo:"));
    assert_eq!(text(at::RESULT), Text::new("ready: 2 links"));
    assert!(looks[at::SEQ_LANDED].x < looks[at::SEQ_CURRENT].x && looks[at::SEQ_CURRENT].x < looks[at::RESULT].x);
    // The helper's own cells, spread over the wider bar; the combo bar's header and pills are not shown.
    assert_eq!(text(at::TITLE), Text::new("1 Jab>Up tilt"));
    assert_eq!(looks[at::CELL].w, SEQ_CELL_W);
    assert!(!looks[at::HEADER].visible && !looks[at::STREAK].visible && !looks[at::PILL].visible);
}

fn after_a_drop_the_bar_names_it_and_restarts_at_combo_one() {
    let mut h = helper(true, true);
    h.restart_with(Some(reference()));
    let fault = Summary { clean: false, fault: Some((1, Reason::Off(Button::Attack, 3))), ..clean() };
    let dropped = Stepped { step: 1, summary: fault, advance: Advance::Dropped };
    let mut seq = view(0, Some(dropped), "Nair loop combo");
    seq.streak = 0;
    let looks = draw_seq(&h, &seq);
    assert_eq!((looks[at::SEQ_LANDED].text, looks[at::SEQ_LANDED].color), (Text::new("Nair loop combo dropped ·"), BAD));
    assert_eq!((looks[at::REASON].text, looks[at::REASON].color), (Text::new("L2 Attack 3f late"), WARN));
    assert_eq!(looks[at::SEQ_CURRENT].text, Text::new("· restart at combo 1"));
    assert!(!looks[at::RESULT].visible);
    assert_eq!(looks[at::SEQ_CHIP_EDGE].color, TEXT, "combo 1 is current again");
    assert_eq!(looks[at::SEQ_CHIP_EDGE + 1].color, BAD, "the dropped combo");
    assert_eq!((looks[at::SEQ_STREAK].text, looks[at::SEQ_BEST].text), (Text::new("streak 0"), Text::new("· best 9")));
    let gap = Summary { outcome: Outcome::Stopped, graded: 0, true_links: 0, clean: false, fault: None, ..clean() };
    seq.last = Some(Stepped { step: 1, summary: gap, advance: Advance::Dropped });
    assert_eq!(draw_seq(&h, &seq)[at::REASON].text, Text::new("the exchange ended"));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_bar_is_hidden_without_the_strip_the_helper_or_a_target", the_bar_is_hidden_without_the_strip_the_helper_or_a_target as fn()),
        ("shown_it_writes_every_pane_once_and_then_only_changes", shown_it_writes_every_pane_once_and_then_only_changes as fn()),
        ("texts_name_verdicts_marks_results_and_reasons", texts_name_verdicts_marks_results_and_reasons as fn()),
        ("a_drilled_sequence_draws_the_header_the_combo_row_and_the_helper_cells", a_drilled_sequence_draws_the_header_the_combo_row_and_the_helper_cells as fn()),
        ("after_a_drop_the_bar_names_it_and_restarts_at_combo_one", after_a_drop_the_bar_names_it_and_restarts_at_combo_one as fn()),
    ]);
}
}
