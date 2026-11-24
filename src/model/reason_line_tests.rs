//! The card's reason line (`combo_plan`, pane `REASON`): every fault the grader can name is drawn whole, inside the pane
//! as the card places it. A child of `combo`, so a finished attempt can be put on the helper directly.
lib_unit_tests! {
use super::*;
use crate::model::combo_plan::{at, button_name, event_label, Renderer, SeqView, COUNT};
use crate::model::drill::{Advance, Stepped};
use crate::model::frame_strip_plan::{Look, Text};
use crate::model::notice_box;
use crate::model::panes::{cb::CHIPS, iv::FONT_SUB, strip_layout::C};

const BUTTONS: [Button; 5] = [Button::Attack, Button::Special, Button::Jump, Button::Shield, Button::Grab];
const FLICKS: [Flick; 3] = [Flick::FastFall, Flick::Dash, Flick::TapJump];
const MOVES: [Move; 14] = [
    Move::Jab, Move::DashAttack, Move::SideTilt, Move::UpTilt, Move::DownTilt, Move::SideSmash, Move::UpSmash,
    Move::DownSmash, Move::Aerial, Move::NeutralSpecial, Move::SideSpecial, Move::UpSpecial, Move::DownSpecial, Move::Other,
];
const SPOTS: [Spot; 3] = [Spot::Sweet, Spot::Sour, Spot::Late];
/// One frame, the tracker's whole ring, and the widest number an offset's type holds.
const DELTAS: [i32; 6] = [1, -1, 96, -96, i32::MAX, -i32::MAX];

/// Every fault a `Reason` can carry.
fn reasons() -> Vec<Reason> {
    let kinds = BUTTONS.iter().map(|&b| Kind::Press(b))
        .chain(BUTTONS.iter().map(|&b| Kind::Release(b)))
        .chain(FLICKS.iter().map(|&f| Kind::Flick(f)))
        .chain([Kind::CStick, Kind::Aim(1), Kind::Aim(2)]);
    let mut all = vec![Reason::Drop(1), Reason::Drop(96), Reason::MissingCStick];
    all.extend(MOVES.iter().map(|&m| Reason::WrongMove(m)));
    for saved in SPOTS {
        all.push(Reason::WrongSpot(None, saved));
        all.extend(SPOTS.iter().map(|&got| Reason::WrongSpot(Some(got), saved)));
    }
    for b in BUTTONS {
        all.extend([Reason::Missing(b), Reason::MissingRelease(b)]);
        all.extend(DELTAS.iter().flat_map(|&d| [Reason::Off(b, d), Reason::ReleaseOff(b, d)]));
    }
    for f in FLICKS {
        all.push(Reason::MissingFlick(f));
        all.extend(DELTAS.iter().map(|&d| Reason::FlickOff(f, d)));
    }
    all.extend(DELTAS.iter().map(|&d| Reason::CStickOff(d)));
    all.extend(kinds.map(Reason::WrongDir));
    for part in [1, 2] {
        all.extend([Reason::MissingAim(part), Reason::WrongAim(part), Reason::ExtraAim(part)]);
    }
    // Part 0 is the special's own travel, kept on its press.
    for part in [0, 1, 2] {
        all.push(Reason::AimPath(part));
        // Include the smallest rejected angle and the widest fractional label.
        all.extend([101, -101, 1799, -1799, -1800].map(|d| Reason::AimAngle(part, d)));
        all.extend([16, -16, 100, -100].map(|d| Reason::AimDeflection(part, d)));
    }
    all.push(Reason::Steer);
    all
}

/// A travel as its faults name it: an aim part's, or as part 0 the special's own.
fn ordinal(part: u8) -> &'static str {
    match part { 0 => "Special", 1 => "1st", _ => "2nd" }
}

/// The sentence the card owes for a fault, written out here a second time.
fn sentence(unit: &str, link: u8, reason: Reason) -> String {
    let off = |d: i32| format!("{}f {}", d.abs(), if d > 0 { "late" } else { "early" });
    let fault = match reason {
        Reason::Drop(d) => format!("DROP +{d}"),
        Reason::WrongMove(m) => format!("wrong move: {}", m.name()),
        Reason::WrongSpot(Some(got), saved) => format!("{}, saved {}", got.name(), saved.name()),
        Reason::WrongSpot(None, saved) => format!("no spot, saved {}", saved.name()),
        Reason::Missing(b) => format!("{} missing", button_name(b)),
        Reason::Off(b, d) => format!("{} {}", button_name(b), off(d)),
        Reason::MissingRelease(b) => format!("{} release missing", button_name(b)),
        Reason::ReleaseOff(b, d) => format!("{} release {}", button_name(b), off(d)),
        Reason::MissingFlick(f) => format!("{} missing", event_label(Kind::Flick(f))),
        Reason::FlickOff(f, d) => format!("{} {}", event_label(Kind::Flick(f)), off(d)),
        Reason::MissingCStick => "C-stick missing".to_owned(),
        Reason::CStickOff(d) => format!("C-stick {}", off(d)),
        Reason::WrongDir(kind) => format!("{} wrong direction", event_label(kind)),
        Reason::MissingAim(part) => format!("{} missing", event_label(Kind::Aim(part))),
        Reason::WrongAim(part) => format!("{} wrong", event_label(Kind::Aim(part))),
        Reason::ExtraAim(part) => format!("extra {}", event_label(Kind::Aim(part))),
        Reason::AimPath(part) => format!("{} path off", ordinal(part)),
        Reason::AimAngle(part, d) => format!("{} angle {} off", ordinal(part), f32::from(d.unsigned_abs()) / 10.0),
        Reason::AimDeflection(part, d) => {
            format!("{} deflection {}% {}", ordinal(part), d.unsigned_abs(), if d < 0 { "low" } else { "high" })
        }
        Reason::Steer => "Special steer wrong".to_owned(),
    };
    format!("{unit}{} {fault}", u32::from(link) + 1)
}

fn faulted(link: u8, reason: Reason) -> Summary {
    Summary { outcome: Outcome::Completed, graded: 2, true_links: 0, total: 2, clean: false, fault: Some((link, reason)) }
}

/// A helper whose last attempt ended with `summary`: a drill (`input_only`) or a hit combo.
fn graded(input_only: bool, summary: Summary) -> Helper {
    let link = LinkRec::presses(
        Move::Jab,
        Move::UpTilt,
        Verdict::True(Some(-3)),
        Some((0, 4)),
        &[Event::press(Button::Attack, Aim::Up, 1)],
    );
    let mut h = Helper::new();
    h.set_target(Some(Combo { input_only, ..Combo::from_links([link, link]).unwrap() }));
    h.step(&Tracker::new(), Settings { on: true, ..Settings::factory() });
    h.attempt = Some(Attempt { summary: Some(summary), ..Attempt::new(0, false) });
    h
}

/// A sequence whose last combo dropped with `summary`.
fn dropped(summary: Summary) -> SeqView {
    SeqView {
        name: Text::new("Corner to kill"),
        at: 0,
        steps: 2,
        streak: 0,
        best: 0,
        first: 0,
        chips: [Text::EMPTY; CHIPS],
        last: Some(Stepped { step: 0, summary, advance: Advance::Dropped }),
        last_name: Text::new("Ryu BnB"),
    }
}

/// Every pane of the card as drawn, and the lines of its reason.
fn drawn(h: &Helper, seq: Option<&SeqView>) -> (Vec<Look>, usize) {
    let mut looks = vec![Look::HIDDEN; COUNT];
    let (_, lines) = Renderer::new().draw(h, &Text::new("Jab loop"), seq, true, (&C, false), |i, look, _| looks[i] = *look);
    (looks, lines)
}

/// What the card puts on its reason line, and the pane as placed.
fn shown(h: &Helper, seq: Option<&SeqView>) -> (String, Look) {
    let look = drawn(h, seq).0[at::REASON];
    (String::from_utf16(look.text.units()).unwrap(), look)
}

/// The whole sentence, on at most two lines, each no wider than the pane at the reason's font.
fn check(text: &str, look: Look, want: &str) {
    assert!(look.visible, "{want}: the reason line is shown");
    assert_eq!(text.replace('\n', " "), want, "the whole sentence");
    let lines: Vec<&str> = text.split('\n').collect();
    assert!(lines.len() <= 2, "{want}: {} lines", lines.len());
    for line in lines {
        let (width, _) = notice_box::measure(line.encode_utf16(), FONT_SUB.0);
        assert!(width <= look.w, "{want}: {line:?} is {width} wide in a pane of {}", look.w);
    }
}

#[test]
fn every_reason_the_grader_can_give_is_shown_in_full_at_the_cards_width() {
    let mut seen = 0;
    for reason in reasons() {
        for link in [0u8, 15] {
            let summary = faulted(link, reason);
            // A drill counts steps and a combo links.
            for (unit, input_only) in [("S", true), ("L", false)] {
                let (text, look) = shown(&graded(input_only, summary), None);
                check(&text, look, &sentence(unit, link, reason));
            }
            // A dropped sequence names the fault on the sequence card.
            let mut waiting = graded(false, summary);
            waiting.attempt = None;
            let (text, look) = shown(&waiting, Some(&dropped(summary)));
            check(&text, look, &sentence("L", link, reason));
            seen += 3;
        }
    }
    assert_eq!(seen, reasons().len() * 2 * 3, "every reason, at link 1 and link 16, on the three cards");
}

#[test]
fn dropped_reason_fits_beside_the_actual_previous_caption() {
    let summary = faulted(0, Reason::Drop(1));
    let mut waiting = graded(false, summary);
    waiting.attempt = None;
    let mut seq = dropped(summary);
    for (name, shares) in [("MMMMMMMMMMM", false), ("iiiiiiiiiii", true)] {
        seq.last_name = Text::new(name);
        let (card, _) = drawn(&waiting, Some(&seq));
        let (prev, why) = (card[at::SEQ_LANDED], card[at::REASON]);
        check(&String::from_utf16(why.text.units()).unwrap(), why, "L1 DROP +1");
        assert_eq!(prev.y == why.y, shares, "{name}: fit depends on ink, not unit count");
        if shares {
            assert!(prev.x - prev.w / 2.0 + notice_box::measure(prev.text.units().iter().copied(), FONT_SUB.0).0
                < why.x - why.w / 2.0);
        }
    }
}

/// The mark under a step or link (`mark_text`, pane `OFFSET`): the first recorded event's name and what was wrong.
#[test]
fn every_mark_under_a_step_is_shown_in_full() {
    let kinds = BUTTONS.iter().map(|&b| Kind::Press(b))
        .chain(BUTTONS.iter().map(|&b| Kind::Release(b)))
        .chain(FLICKS.iter().map(|&f| Kind::Flick(f)))
        .chain([Kind::CStick, Kind::Aim(1), Kind::Aim(2)]);
    for kind in kinds {
        let name = event_label(kind);
        let wrong = if name.ends_with("direction") { format!("{name} wrong") } else { format!("{name} wrong direction") };
        let mut marks = vec![(Mark::Missing, format!("{name} missing")), (Mark::WrongDir, wrong)];
        marks.extend(DELTAS.iter().map(|&d| {
            (Mark::At(d), format!("{name} {}f {}", d.abs(), if d > 0 { "late" } else { "early" }))
        }));
        // A travel's own marks: its path, and with the stick toggle its angle and deflection. An aim part keeps one;
        // so does the press that began a special, which can also be steered wrong.
        let travel = match kind {
            Kind::Aim(part) => Some(ordinal(part)),
            Kind::Press(Button::Special) => Some("Special"),
            Kind::CStick => Some("C-stick"),
            _ => None,
        };
        if let Some(part) = travel {
            if !matches!(kind, Kind::Aim(_)) {
                marks.push((Mark::Steer, format!("{part} steer wrong")));
            }
            marks.push((Mark::Path, format!("{part} path off")));
            marks.extend([(101, "10.1"), (-1799, "179.9"), (-1800, "180")]
                .map(|(d, deg)| (Mark::Angle(d), format!("{part} angle {deg} off"))));
            marks.extend([16, -100].map(|d: i8| {
                (Mark::Deflection(d), format!("{part} deflection {}% {}", d.unsigned_abs(), if d < 0 { "low" } else { "high" }))
            }));
        }
        for (mark, want) in marks {
            let (text, _) = crate::model::combo_plan::mark_text(mark, name, 0);
            assert_eq!(String::from_utf16(text.units()).unwrap(), want);
        }
    }
}

#[test]
fn the_widest_fractional_angle_reaches_both_card_panes() {
    let reference = LinkRec {
        events: retain([Event { kind: Kind::Aim(2), dir: Dir::Back, offset: 0 }]),
        ..LinkRec::INPUT
    };
    let mut h = Helper::new();
    h.set_target(Some(Combo { input_only: true, ..Combo::from_links([reference; LINKS]).unwrap() }));
    let mut g = grade(&reference, &reference);
    g.marks[0] = Some(Mark::Angle(-1799));
    h.attempt = Some(Attempt { grades: [Some(g); LINKS], len: LINKS,
        summary: Some(faulted(15, Reason::AimAngle(2, -1799))), ..Attempt::new(0, false) });
    let (card, _) = drawn(&h, None);
    let why = card[at::REASON];
    check(&String::from_utf16(why.text.units()).unwrap(), why, "S16 2nd angle 179.9 off");
    let mark = card[at::RULER - 1];
    assert!(mark.visible);
    assert_eq!(String::from_utf16(mark.text.units()).unwrap(), "2nd angle 179.9 off");
}

/// A link cell's title (pane `TITLE`): its number and both moves.
#[test]
fn a_links_title_names_both_moves_in_full() {
    let link = LinkRec::presses(
        Move::DashAttack,
        Move::DashAttack,
        Verdict::True(Some(-3)),
        Some((0, 4)),
        &[Event::press(Button::Attack, Aim::Neutral, 1)],
    );
    let mut h = Helper::new();
    h.set_target(Combo::from_links([link, link]));
    h.step(&Tracker::new(), Settings { on: true, ..Settings::factory() });
    let mut looks = vec![Look::HIDDEN; COUNT];
    Renderer::new().draw(&h, &Text::new("Jab loop"), None, true, (&C, false), |i, look, _| looks[i] = *look);
    assert!(looks[at::TITLE].visible);
    assert_eq!(String::from_utf16(looks[at::TITLE].text.units()).unwrap(), "1 Dash attack>Dash attack");
}

/// The one reason a player can reach that is wider than the card: it takes a second line, its first line stays on its
/// row, and the card, the statistics under it and the fade behind a fighter follow.
#[test]
fn a_reason_wider_than_the_card_wraps_and_the_card_grows_one_line() {
    let wide = Reason::WrongDir(Kind::Release(Button::Special));
    let text = |looks: &[Look]| String::from_utf16(looks[at::REASON].text.units()).unwrap();
    let pitch = C.card.text_height;
    assert_eq!(pitch, 26.0);
    assert_eq!(C.card.grown(false, 1), C.card.bounds(false));
    assert_eq!(C.card.bounds(false), [804.0, 66.0, 288.0, 84.0]);
    assert_eq!(C.card.grown(false, 2), [804.0, 53.0, 288.0, 110.0]);
    assert_eq!(C.card.grown(true, 2), [804.0, -3.0, 288.0, 222.0]);
    assert_eq!((C.card.reason_height(0), C.card.reason_height(1), C.card.reason_height(2)), (26.0, 26.0, 52.0));

    // A drill: step 9's sentence fits the card, step 16's does not.
    let (one, one_lines) = drawn(&graded(true, faulted(8, wide)), None);
    let (two, two_lines) = drawn(&graded(true, faulted(15, wide)), None);
    assert_eq!((text(&one).as_str(), one_lines), ("S9 Special release wrong direction", 1));
    assert_eq!((text(&two).as_str(), two_lines), ("S16 Special release wrong\ndirection", 2));
    assert_eq!((C.card.top - one[at::REASON].y, one[at::REASON].w), (74.0, 268.0));
    assert_eq!((one[at::REASON].y - two[at::REASON].y, two[at::REASON].w), (pitch / 2.0, 268.0), "the first line stays on its row");
    assert_eq!(one[at::RESULT].y, two[at::RESULT].y);
    assert_eq!(one[at::BACKDROP].y, C.card.bounds(false)[1]);
    assert_eq!(two[at::BACKDROP].y, C.card.grown(false, 2)[1]);

    // A dropped sequence: the statistics under the reason move down with the plate.
    let seq = |link| {
        let mut waiting = graded(false, faulted(link, wide));
        waiting.attempt = None;
        drawn(&waiting, Some(&dropped(faulted(link, wide))))
    };
    let ((one, one_lines), (two, two_lines)) = (seq(8), seq(15));
    assert_eq!((one_lines, two_lines), (1, 2));
    assert_eq!(text(&two), "L16 Special release wrong\ndirection");
    assert_eq!(one[at::REASON].y - two[at::REASON].y, pitch / 2.0);
    assert_eq!(one[at::SEQ_LANDED].y, two[at::SEQ_LANDED].y);
    for i in [at::SEQ_STREAK, at::SEQ_BEST, at::SEQ_LINKS, at::SEQ_TOLERANCE] {
        assert_eq!(one[i].y - two[i].y, pitch, "pane {i} is one line lower");
    }
    assert_eq!(one[at::SEQ_BACKDROP].y - two[at::SEQ_BACKDROP].y, pitch / 2.0);
    assert_eq!(two[at::SEQ_BACKDROP].y, C.card.grown(true, 2)[1]);

    // A fighter behind the grown part of the plate fades the panel only while the reason has its second line.
    let [x, y, _, h] = C.card.grown(false, 2);
    let behind = [[x + C.strip.origin_x, y - h / 2.0 + 5.0 + C.strip.origin_y]];
    assert_eq!(C.opacity(0.0, false, Some((false, 2)), behind), C.covered_alpha);
    assert_eq!(C.opacity(0.0, false, Some((false, 1)), behind), u8::MAX);
}
}
