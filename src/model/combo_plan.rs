//! The combo bar's panes (SPEC-input-07 §7): what each shows for the helper, and the renderer that writes only what
//! changed (the frame strip's rule, SPEC-input-06 §5.2). The layout spec places these readouts.

use core::fmt::Write as _;

use super::combo::{Dir, Flick, Grade, Helper, Kind, LinkRec, Mark, Outcome, Reason, Summary};
use super::drill::{Advance, Stepped};
use super::frame_strip::{Button, Verdict};
use super::frame_strip_plan::{Changed, Look, Text, BAD, GOOD, MUTED, RULE, SHORT_UNITS, TEXT, WARN};
use super::panes::cb::*;
use super::panes::strip_layout::{card::{Detail, Metrics, REASON_LINES}, LayoutSpec};
pub use super::panes::cb::{at, COUNT};

pub const fn button_name(button: Button) -> &'static str {
    match button {
        Button::Attack => "Attack",
        Button::Special => "Special",
        Button::Jump => "Jump",
        Button::Shield => "Shield",
        Button::Grab => "Grab",
    }
}

pub fn verdict_text(verdict: Verdict) -> Text {
    let mut t = Text::EMPTY;
    let _ = match verdict {
        Verdict::True(Some(link)) => write!(t, "TRUE {link}"),
        Verdict::True(None) => write!(t, "TRUE"),
        Verdict::Drop(link) => write!(t, "DROP +{link}"),
    };
    t
}

/// The bar's name for one event. Press names stay the button alone, so an old fault still reads "Jump missing".
pub fn event_label(kind: Kind) -> &'static str {
    match kind {
        Kind::Press(button) => button_name(button),
        Kind::Release(Button::Attack) => "Attack release",
        Kind::Release(Button::Special) => "Special release",
        Kind::Release(Button::Jump) => "Jump release",
        Kind::Release(Button::Shield) => "Shield release",
        Kind::Release(Button::Grab) => "Grab release",
        Kind::CStick => "C-stick",
        Kind::Flick(Flick::FastFall) => "fast-fall",
        Kind::Flick(Flick::Dash) => "dash",
        Kind::Flick(Flick::TapJump) => "tap-jump",
        Kind::Aim(1) => "1st direction",
        Kind::Aim(_) => "2nd direction",
    }
}

fn write_angle(text: &mut Text, ordinal: &str, tenths: i16) -> core::fmt::Result {
    let magnitude = tenths.unsigned_abs();
    write!(text, "{ordinal} angle {}", magnitude / 10)?;
    if !magnitude.is_multiple_of(10) {
        write!(text, ".{}", magnitude % 10)?;
    }
    text.write_str(" off")
}

/// A reference event's mark: "on time", "Jump 2f late", "Jump release missing", "C-stick wrong direction".
pub fn mark_text(mark: Mark, name: &str, tolerance: u8) -> (Text, [u8; 4]) {
    let mut t = Text::EMPTY;
    let color = match mark {
        Mark::At(d) if d.unsigned_abs() <= u32::from(tolerance) => {
            let _ = t.write_str("on time");
            GOOD
        }
        Mark::At(d) => {
            let _ = write!(
                t,
                "{name} {}f {}",
                d.abs(),
                if d > 0 { "late" } else { "early" }
            );
            WARN
        }
        Mark::Missing => {
            let _ = write!(t, "{name} missing");
            BAD
        }
        Mark::WrongDir => {
            let _ = if name.ends_with("direction") {
                write!(t, "{name} wrong")
            } else {
                write!(t, "{name} wrong direction")
            };
            BAD
        }
        Mark::Path => {
            let _ = write!(t, "{} path off", name.split_whitespace().next().unwrap_or(name));
            BAD
        }
        Mark::Angle(delta) => {
            let _ = write_angle(&mut t, name.split_whitespace().next().unwrap_or(name), delta);
            BAD
        }
        Mark::Deflection(delta) => {
            let _ = write!(t, "{} deflection {}% {}", name.split_whitespace().next().unwrap_or(name),
                delta.unsigned_abs(), if delta < 0 { "low" } else { "high" });
            BAD
        }
        Mark::Steer => {
            let _ = write!(t, "{} steer wrong", name.split_whitespace().next().unwrap_or(name));
            BAD
        }
    };
    (t, color)
}

/// Whose travel a fault names: an aim part's, or as part 0 the special's own.
const fn travel_name(part: u8) -> &'static str {
    match part {
        0 => "Special",
        1 => "1st",
        _ => "2nd",
    }
}

/// Drift, shown and never graded. The font draws `>` and `<` for the two sideways holds.
pub fn drift_text(dir: Dir) -> Text {
    let mut t = Text::EMPTY;
    let word = match dir {
        Dir::Forward => ">",
        Dir::Back => "<",
        Dir::Up => "up",
        Dir::Down => "down",
        Dir::ForwardUp => "forward-up",
        Dir::BackUp => "back-up",
        Dir::ForwardDown => "forward-down",
        Dir::BackDown => "back-down",
        Dir::Neutral | Dir::Side => return t,
    };
    let _ = write!(t, "drift {word}");
    t
}

/// "3 of 4 true", "stopped: 2 of 4", "cleared".
pub fn result_text(s: &Summary) -> Text { result_mode(s, false) }
fn result_mode(s: &Summary, input: bool) -> Text {
    let mut t = Text::EMPTY;
    let _ = match s.outcome {
        Outcome::Completed if s.clean => write!(t, "clean, {} of {}", s.true_links, s.total),
        Outcome::Completed if input => write!(t, "{} of {} counted", s.true_links, s.total),
        Outcome::Completed => write!(t, "{} of {} true", s.true_links, s.total),
        Outcome::Stopped => write!(t, "stopped: {} of {}", s.true_links, s.total),
        Outcome::Cleared => write!(t, "cleared"),
    };
    t
}

/// The first fault: "L3 DROP +1", "L2 wrong move: Aerial", "L1 Jump missing", "L2 Jump 2f late".
pub fn reason_text(s: &Summary) -> Text { reason_mode(s, false) }
fn reason_mode(s: &Summary, input: bool) -> Text {
    let unit = if input { "S" } else { "L" };
    let mut t = Text::EMPTY;
    let Some((link, reason)) = s.fault else {
        return t;
    };
    let n = u32::from(link) + 1;
    let _ = match reason {
        Reason::Drop(d) => write!(t, "{unit}{n} DROP +{d}"),
        Reason::WrongMove(m) => write!(t, "{unit}{n} wrong move: {}", m.name()),
        Reason::Missing(b) => write!(t, "{unit}{n} {} missing", button_name(b)),
        Reason::Off(b, d) => write!(
            t,
            "{unit}{n} {} {}f {}",
            button_name(b),
            d.abs(),
            if d > 0 { "late" } else { "early" }
        ),
        Reason::MissingRelease(b) => write!(t, "{unit}{n} {} release missing", button_name(b)),
        Reason::ReleaseOff(b, d) => {
            write!(
                t,
                "{unit}{n} {} release {}f {}",
                button_name(b),
                d.abs(),
                if d > 0 { "late" } else { "early" }
            )
        }
        Reason::MissingFlick(flick) => {
            write!(t, "{unit}{n} {} missing", event_label(Kind::Flick(flick)))
        }
        Reason::FlickOff(flick, d) => {
            write!(
                t,
                "{unit}{n} {} {}f {}",
                event_label(Kind::Flick(flick)),
                d.abs(),
                if d > 0 { "late" } else { "early" }
            )
        }
        Reason::MissingCStick => write!(t, "{unit}{n} C-stick missing"),
        Reason::CStickOff(d) => write!(
            t,
            "{unit}{n} C-stick {}f {}",
            d.abs(),
            if d > 0 { "late" } else { "early" }
        ),
        Reason::WrongDir(kind) => write!(t, "{unit}{n} {} wrong direction", event_label(kind)),
        Reason::MissingAim(part) => write!(t, "{unit}{n} {} missing", event_label(Kind::Aim(part))),
        Reason::WrongAim(part) => write!(t, "{unit}{n} {} wrong", event_label(Kind::Aim(part))),
        Reason::ExtraAim(part) => write!(t, "{unit}{n} extra {}", event_label(Kind::Aim(part))),
        Reason::AimPath(part) => write!(t, "{unit}{n} {} path off", travel_name(part)),
        Reason::AimAngle(part, delta) => write!(t, "{unit}{n} ")
            .and_then(|()| write_angle(&mut t, travel_name(part), delta)),
        Reason::AimDeflection(part, delta) => write!(t, "{unit}{n} {} deflection {}% {}",
            travel_name(part), delta.unsigned_abs(), if delta < 0 { "low" } else { "high" }),
        Reason::Steer => write!(t, "{unit}{n} {} steer wrong", travel_name(0)),
        Reason::WrongSpot(Some(got), saved) => write!(t, "{unit}{n} {}, saved {}", got.name(), saved.name()),
        Reason::WrongSpot(None, saved) => write!(t, "{unit}{n} no spot, saved {}", saved.name()),
    };
    t
}

fn pill_color(s: &Summary) -> [u8; 4] {
    match s.outcome {
        _ if s.clean => GOOD,
        Outcome::Completed if s.true_links == s.total => WARN,
        Outcome::Cleared => MUTED,
        _ => BAD,
    }
}

/// What the bar shows for a drilled sequence (docs/RECORDING.md § Sequences and the one drill; mockup
/// `r8-sequence-bar`). The runtime copies it each UI tick from the staged plan and the frame hook's run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeqView {
    pub name: Text,
    /// The current combo (0-based) and the number of combos drilled.
    pub at: usize,
    pub steps: usize,
    pub streak: u32,
    pub best: u32,
    /// The first combo the chips show, and the names of the combos from there.
    pub first: usize,
    pub chips: [Text; CHIPS],
    /// The last graded combo and its name.
    pub last: Option<Stepped>,
    pub last_name: Text,
}

impl SeqView {
    /// The first chip: the current combo and the one before it stay in view.
    pub fn first_chip(at: usize, steps: usize) -> usize {
        at.saturating_sub(1).min(steps.saturating_sub(CHIPS))
    }

}

/// A chip's state in the row of combos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChipState {
    Done,
    Current,
    Dropped,
    Later,
}

/// The sequence's waiting state (no attempt open): the run just finished, just dropped, or has not started.
fn waiting_after(helper: &Helper, seq: &SeqView) -> Option<Advance> {
    if helper.attempt().is_some() {
        return None;
    }
    seq.last.map(|s| s.advance).filter(|a| matches!(a, Advance::Finished | Advance::Dropped))
}

fn chip_label(seq: &SeqView, c: usize) -> Text {
    let mut t = Text::EMPTY;
    let _ = write!(t, "{}  ", seq.first + c + 1);
    push_text(&mut t, &seq.chips[c]);
    // A chip is no wider than the card: its label keeps the short length.
    t.cut(SHORT_UNITS)
}

/// `tail` after `t`, cut where `t` is full.
fn push_text(t: &mut Text, tail: &Text) {
    for ch in char::decode_utf16(tail.units().iter().copied()).map(|r| r.unwrap_or('?')) {
        let _ = t.write_char(ch);
    }
}

/// The first link drawn: the newest graded links and the next ones of the reference, `CELLS` at a time.
pub fn first_cell(graded: usize, total: usize) -> usize {
    graded
        .saturating_sub(CELLS - 1)
        .min(total.saturating_sub(CELLS))
}

fn shown_drift(helper: &Helper) -> Option<Dir> {
    helper
        .attempt()?
        .grades()
        .filter_map(|grade| grade.link.drift)
        .last()
}

#[derive(Clone, Copy)]
struct Ink {
    text: Text,
    color: [u8; 4],
    detail: Detail,
}

impl Ink {
    const fn text(text: Text, color: [u8; 4]) -> Self {
        Self { text, color, detail: Detail::Fixed }
    }

    const fn shape(color: [u8; 4]) -> Self {
        Self::text(Text::EMPTY, color)
    }

    const fn detail(self, detail: Detail) -> Self {
        Self { detail, ..self }
    }

    fn optional(self) -> Option<Self> {
        (!self.text.units().is_empty()).then_some(self)
    }
}

struct Captions {
    name: Ink,
    result: Ink,
    reason: Ink,
    streak: Ink,
    best: Ink,
    step: Ink,
    links: Ink,
    tolerance: Ink,
    previous: Ink,
}

impl Captions {
    const EMPTY: Self = {
        let empty = Ink::shape(MUTED);
        Self { name: empty, result: empty, reason: empty, streak: empty, best: empty,
            step: empty, links: empty, tolerance: empty, previous: empty }
    };

    fn single(helper: &Helper, mut name: Text) -> Self {
        let attempt = helper.attempt();
        let summary = attempt.and_then(|a| a.summary);
        let total = helper.target().map_or(0, |t| t.len());
        let input = helper.target().is_some_and(|t| t.input_only);
        let units = if input { "steps" } else { "links" };
        if name.units().is_empty() {
            name = Text::new(if input { "Drill" } else { "Combo" });
        }
        let mut streak = Text::EMPTY;
        let _ = write!(streak, "streak {}  best {}/{total}", helper.streak(), helper.best());

        let mut result = Text::EMPTY;
        let color = match (attempt, summary) {
            (_, Some(s)) => {
                result = result_mode(&s, input);
                if s.clean { GOOD } else { TEXT }
            }
            (Some(a), None) => {
                let _ = write!(result, "{} {} of {total}", if input { "step" } else { "link" }, a.len());
                TEXT
            }
            (None, None) => {
                let _ = write!(result, "ready: {total} {units}");
                MUTED
            }
        };
        let reason = match summary {
            Some(s) if s.fault.is_some() => Ink::text(reason_mode(&s, input), if s.clean { GOOD } else { WARN }),
            Some(s) if s.clean => Ink::text(shown_drift(helper).map_or(Text::EMPTY, drift_text), MUTED),
            _ => Self::EMPTY.reason,
        };
        Self { name: Ink::text(name, TEXT), result: Ink::text(result, color), reason,
            streak: Ink::text(streak, MUTED), ..Self::EMPTY }
    }

    fn sequence(helper: &Helper, seq: &SeqView) -> Self {
        let total = helper.target().map_or(0, |t| t.len());
        let mut step = Text::EMPTY;
        let mut streak = Text::EMPTY;
        let mut best = Text::EMPTY;
        let mut links = Text::EMPTY;
        let mut tolerance = Text::EMPTY;
        let waiting = waiting_after(helper, seq);
        if waiting == Some(Advance::Dropped) {
            let _ = write!(step, "restart 1/{}", seq.steps);
        } else {
            let _ = write!(step, "{} of {}", seq.at + 1, seq.steps);
        }
        let _ = write!(streak, "streak {}", seq.streak);
        let _ = write!(best, "· best {}", seq.best);
        let _ = write!(links, "{total} {}", if total == 1 { "link" } else { "links" });
        let _ = write!(tolerance, "±{}f", helper.settings().tolerance);
        let mut captions = Self {
            name: Ink::text(seq.name, TEXT),
            step: Ink::text(step, MUTED),
            streak: Ink::text(streak, GOOD),
            best: Ink::text(best, GOOD),
            links: Ink::text(links, MUTED),
            tolerance: Ink::text(tolerance, MUTED),
            ..Self::EMPTY
        };
        let mut previous = Text::EMPTY;
        let attempt = helper.attempt();
        match waiting {
            Some(Advance::Dropped) => {
                push_text(&mut previous, &seq.last_name);
                let _ = previous.write_str(" dropped");
                let reason = match seq.last.map(|s| s.summary) {
                    Some(s) if s.fault.is_some() => reason_text(&s),
                    Some(s) if s.outcome == Outcome::Cleared => Text::new("cleared"),
                    _ => Text::new("the exchange ended"),
                };
                // A name beside its verb shares the card's row with the reason: it keeps the short length.
                captions.previous = Ink::text(previous.cut(SHORT_UNITS), BAD);
                captions.reason = Ink::text(reason, WARN);
            }
            waiting => {
                if waiting == Some(Advance::Finished) {
                    captions.previous = Ink::text(Text::new("sequence done"), GOOD);
                } else if attempt.is_some() && seq.last.is_some_and(|s| matches!(s.advance, Advance::Next(_))) {
                    push_text(&mut previous, &seq.last_name);
                    let _ = previous.write_str(" landed");
                    captions.previous = Ink::text(previous.cut(SHORT_UNITS), TEXT);
                }
                captions.result = match (attempt, attempt.and_then(|a| a.summary)) {
                    (_, Some(s)) => Ink::text(result_text(&s), if s.clean { GOOD } else { TEXT }),
                    (Some(a), None) if !a.is_empty() => {
                        let mut result = Text::EMPTY;
                        let newest_true = a.grades().last().is_some_and(Grade::is_true);
                        let _ = write!(result, "link {} of {total}{}", a.len(), if newest_true { " true" } else { "" });
                        Ink::text(result, TEXT)
                    }
                    _ => {
                        let mut result = Text::EMPTY;
                        let _ = write!(result, "ready: {total} {}", if total == 1 { "link" } else { "links" });
                        Ink::text(result, MUTED)
                    }
                };
            }
        }
        captions
    }

    fn metrics(&self, pills: usize) -> Metrics {
        let len = |ink: Ink| ink.text.units().len();
        let width = |ink: &Ink| super::notice_box::measure(ink.text.units().iter().copied(), super::panes::iv::FONT_SUB.0).0;
        Metrics {
            pills,
            result: width(&self.result),
            reason: width(&self.reason),
            reason_lines: self.reason.text.lines(),
            previous: width(&self.previous),
            streak: len(self.streak),
            best: len(self.best),
            links: len(self.links),
            tolerance: len(self.tolerance),
            title: len(self.name),
        }
    }
}

/// Scores, names and marks emitted without reference to canvas dimensions.
struct Plan<'a> {
    helper: &'a Helper,
    seq: Option<&'a SeqView>,
    captions: Captions,
}

impl Plan<'_> {
    fn chip_state(&self, seq: &SeqView, k: usize) -> ChipState {
        match waiting_after(self.helper, seq) {
            Some(Advance::Finished) => ChipState::Done,
            Some(Advance::Dropped) if seq.last.is_some_and(|s| s.step == k) => ChipState::Dropped,
            _ if k < seq.at => ChipState::Done,
            _ if k == seq.at => ChipState::Current,
            _ => ChipState::Later,
        }
    }

    fn sequence(&self, i: usize, seq: &SeqView) -> Option<Ink> {
        use at::*;
        let captions = &self.captions;
        match i {
            SEQ_BACKDROP => Some(Ink::shape(BACKDROP_COLOR)),
            SEQ_PILL => Some(Ink::shape(SEQ_PILL_COLOR)),
            SEQ_PILL_TEXT => Some(Ink::text(Text::new("SEQUENCE"), SEQ_PILL_TEXT_COLOR)),
            SEQ_NAME => captions.name.optional(),
            SEQ_STEP => Some(captions.step),
            SEQ_LINKS => Some(captions.links),
            SEQ_TOLERANCE => Some(captions.tolerance),
            SEQ_STREAK => Some(captions.streak),
            SEQ_BEST => Some(captions.best),
            SEQ_LANDED => captions.previous.optional(),
            _ => {
                let c = (i - SEQ_CHIP_EDGE) % CHIPS;
                let k = seq.first + c;
                if k >= seq.steps {
                    return None;
                }
                let state = self.chip_state(seq, k);
                let (edge, ink) = match state {
                    ChipState::Done => (GOOD, GOOD),
                    ChipState::Current => (TEXT, TEXT),
                    ChipState::Dropped => (BAD, BAD),
                    ChipState::Later => (RULE, MUTED),
                };
                let label = chip_label(seq, c);
                let check = state == ChipState::Done;
                let detail = Detail::Chip { units: label.units().len(), check };
                let part = match (i - SEQ_CHIP_EDGE) / CHIPS {
                    0 => Ink::shape(edge),
                    1 => Ink::shape(SEQ_CHIP_COLOR),
                    2 => Ink::text(label, ink),
                    _ if check => Ink::shape(GOOD),
                    _ => return None,
                };
                Some(part.detail(detail))
            }
        }
    }

    /// Cell c's link: its reference and, once graded, its grade.
    fn cell(&self, c: usize) -> Option<(usize, LinkRec, Option<Grade>)> {
        let target = self.helper.target()?;
        let attempt = self.helper.attempt();
        let k = first_cell(attempt.map_or(0, |a| a.len()), target.len()) + c;
        Some((
            k,
            *target.get(k)?,
            attempt.and_then(|a| a.grades().nth(k).copied()),
        ))
    }

    fn ink(&self, i: usize) -> Option<Ink> {
        use at::*;
        let captions = &self.captions;
        match i {
            CONTAINER => Some(Ink::shape([0; 4])),
            RESULT => captions.result.optional(),
            REASON => captions.reason.optional(),
            i if (CELL..SEQ_BACKDROP).contains(&i) => self.cell_ink(i),
            i if i >= SEQ_BACKDROP => self.seq.and_then(|seq| self.sequence(i, seq)),
            _ if self.seq.is_some() => None,
            BACKDROP => Some(Ink::shape(BACKDROP_COLOR)),
            HEADER => captions.name.optional(),
            STREAK => Some(captions.streak),
            i if (PILL..CELL).contains(&i) => self.helper.results().nth(i - PILL).map(|s| Ink::shape(pill_color(s))),
            _ => None,
        }
    }

    fn cell_ink(&self, i: usize) -> Option<Ink> {
        use at::*;
        let tolerance = self.helper.settings().tolerance;
        let c = (i - CELL) % CELLS;
        let (k, reference, grade) = self.cell(c)?;
        let input = self.helper.target().is_some_and(|t| t.input_only);
        let origin = if input { reference.events().next().map_or(0, |e| e.offset) } else { 0 };
        let first_step = reference.events[0];
        let ink = match (i - CELL) / CELLS {
            0 => Ink::shape(CELL_COLOR),
            1 => {
                let mut text = Text::EMPTY;
                if input {
                    let _ = write!(text, "Step {}", k + 1);
                } else {
                    let namer = super::move_names::Namer::target();
                    let _ = write!(text, "{} ", k + 1);
                    let [first, follow] = self.helper.target().and_then(|target| target.descs(k))
                        .unwrap_or([reference.desc(0), reference.desc(1)]);
                    let _ = first.write(&mut text, false, namer);
                    let _ = text.write_str(">");
                    let _ = follow.write(&mut text, false, namer);
                }
                Ink::text(text, MUTED)
            }
            2 if input => return None,
            2 => match grade {
                Some(g) if g.wrong_move => Ink::text(Text::new("WRONG MOVE"), BAD),
                Some(g) if g.wrong_spot && self.helper.settings().match_spot => Ink::text(Text::new("WRONG SPOT"), BAD),
                Some(g) => Ink::text(verdict_text(g.link.verdict), if g.is_true() { GOOD } else { BAD }),
                None => Ink::text(verdict_text(reference.verdict), RULE),
            },
            3 => {
                let g = grade?;
                let mark = g.marks[0]?;
                let step = first_step?;
                let (text, color) = mark_text(mark, event_label(step.kind), tolerance.saturating_add(g.band[0]));
                Ink::text(text, color)
            }
            4 => Ink::shape(RULE),
            5 if input => return None,
            5 => {
                let (a, b) = grade.map_or(reference.window, |g| g.link.window)?;
                Ink::shape(GOOD).detail(Detail::Window(a - origin, b + 1 - origin))
            }
            6 => Ink::shape(MUTED).detail(Detail::Marker(first_step?.offset - origin)),
            _ => {
                let g = grade?;
                match g.marks[0]? {
                    Mark::At(d) => {
                        let color = if d.unsigned_abs() <= u32::from(tolerance) + u32::from(g.band[0]) { GOOD } else { WARN };
                        Ink::shape(color).detail(Detail::Marker(first_step?.offset + d - origin))
                    }
                    _ => return None,
                }
            }
        };
        Some(ink)
    }
}

/// What was last written to each of the bar's panes (`frame_strip_plan::Panes`).
pub type Renderer = super::frame_strip_plan::Panes<COUNT, { super::panes::cb::LONG_LINES }>;

/// Whether the bar is shown: the strip shown, the helper On and a target (`Renderer::draw`; the notice slot sits under it).
pub fn shown(helper: &Helper, strip_shown: bool) -> bool {
    strip_shown && helper.settings().on && helper.target().is_some()
}

impl Renderer {
    /// One UI tick (the strip's rule): hidden writes only the container, once; shown writes every pane that changed.
    /// Shown needs the strip shown, the helper On and a target. `name` is the target's name. A drilled sequence
    /// (`seq`) draws the sequence bar. Returns the writes and the lines of the reason (0: none; each line after the
    /// first grows the card). The flag indicates a visible history row above the cells.
    pub fn draw(
        &mut self,
        helper: &Helper,
        name: &Text,
        seq: Option<&SeqView>,
        strip_shown: bool,
        (spec, history): (&LayoutSpec, bool),
        apply: impl FnMut(usize, &Look, Changed),
    ) -> (usize, usize) {
        if !shown(helper, strip_shown) {
            return (self.hide(at::CONTAINER, Look::HIDDEN, apply), 0);
        }
        let mut captions = match seq {
            Some(seq) => Captions::sequence(helper, seq),
            None => Captions::single(helper, *name),
        };
        // A reason wider than the card wraps at the card's width; the layout grows the card by the wrapped line.
        captions.reason.text = captions.reason.text.fitted(spec.card.reason_width(), super::panes::iv::FONT_SUB.0, REASON_LINES);
        let metrics = captions.metrics(helper.results().count());
        let plan = Plan { helper, seq, captions };
        let writes = self.show(super::panes::cb::units, |i| match plan.ink(i) {
            Some(ink) => Look { visible: true, text: ink.text, color: ink.color, ..Look::HIDDEN }
                .at(spec.card.place(i, seq.is_some(), &spec.strip, history, metrics, ink.detail)),
            None => Look::HIDDEN,
        }, apply);
        (writes, metrics.reason_lines)
    }
}

#[cfg(test)]
#[path = "combo_plan_tests.rs"]
mod tests;
