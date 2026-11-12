//! The combo bar's panes (SPEC-input-07 §7): what each shows for the helper, and the renderer that writes only what
//! changed (the frame strip's rule, SPEC-input-06 §5.2). Positions are relative to the bar's container `TrCb`.

use core::fmt::Write as _;

use super::combo::{Dir, Flick, Grade, Helper, Kind, LinkRec, Mark, Outcome, Reason, Summary};
use super::drill::{Advance, Stepped};
use super::frame_strip::{Button, Verdict};
use super::frame_strip_plan::{Changed, Look, Text, BAD, GOOD, MUTED, RULE, TEXT, WARN};
use super::panes::cb::*;
use super::panes::iv::{FONT_SUB, FONT_TAG};
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
    }
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
            let _ = write!(t, "{name} wrong direction");
            BAD
        }
    };
    (t, color)
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
pub fn result_text(s: &Summary) -> Text {
    let mut t = Text::EMPTY;
    let _ = match s.outcome {
        Outcome::Completed if s.clean => write!(t, "clean, {} of {}", s.true_links, s.total),
        Outcome::Completed => write!(t, "{} of {} true", s.true_links, s.total),
        Outcome::Stopped => write!(t, "stopped: {} of {}", s.true_links, s.total),
        Outcome::Cleared => write!(t, "cleared"),
    };
    t
}

/// The first fault: "L3 DROP +1", "L2 wrong move: Aerial", "L1 Jump missing", "L2 Jump 2f late".
pub fn reason_text(s: &Summary) -> Text {
    let mut t = Text::EMPTY;
    let Some((link, reason)) = s.fault else {
        return t;
    };
    let n = u32::from(link) + 1;
    let _ = match reason {
        Reason::Drop(d) => write!(t, "L{n} DROP +{d}"),
        Reason::WrongMove(m) => write!(t, "L{n} wrong move: {}", m.name()),
        Reason::Missing(b) => write!(t, "L{n} {} missing", button_name(b)),
        Reason::Off(b, d) => write!(
            t,
            "L{n} {} {}f {}",
            button_name(b),
            d.abs(),
            if d > 0 { "late" } else { "early" }
        ),
        Reason::MissingRelease(b) => write!(t, "L{n} {} release missing", button_name(b)),
        Reason::ReleaseOff(b, d) => {
            write!(
                t,
                "L{n} {} release {}f {}",
                button_name(b),
                d.abs(),
                if d > 0 { "late" } else { "early" }
            )
        }
        Reason::MissingFlick(flick) => {
            write!(t, "L{n} {} missing", event_label(Kind::Flick(flick)))
        }
        Reason::FlickOff(flick, d) => {
            write!(
                t,
                "L{n} {} {}f {}",
                event_label(Kind::Flick(flick)),
                d.abs(),
                if d > 0 { "late" } else { "early" }
            )
        }
        Reason::MissingCStick => write!(t, "L{n} C-stick missing"),
        Reason::CStickOff(d) => write!(
            t,
            "L{n} C-stick {}f {}",
            d.abs(),
            if d > 0 { "late" } else { "early" }
        ),
        Reason::WrongDir(kind) => write!(t, "L{n} {} wrong direction", event_label(kind)),
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

/// The x of `frames` from A₁ on cell `cell`'s ruler, clamped to its ends.
pub fn ruler_x(cell: usize, frames: i32) -> f32 {
    ruler_at(CELL_X0 + cell as f32 * CELL_PITCH, frames)
}

fn ruler_at(cx: f32, frames: i32) -> f32 {
    cx - RULER_W / 2.0 + (frames - RULER_FROM).clamp(0, RULER_FRAMES) as f32 * PER_FRAME
}

/// An estimate of a text's width in layout units in `font` (the game's font is proportional; the bar places its
/// texts one after another from it, and sizes a combo's chip around its name).
pub fn est_width(text: &Text, font: (f32, f32)) -> f32 {
    text.units().len() as f32 * font.0 * 0.56
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

    fn current(&self) -> Text {
        self.at.checked_sub(self.first).and_then(|c| self.chips.get(c)).copied().unwrap_or(Text::EMPTY)
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

/// The chips' left edges and widths (layout units), left to right.
fn chip_boxes(seq: &SeqView, done: impl Fn(usize) -> bool) -> [(f32, f32); CHIPS] {
    let mut boxes = [(0.0, 0.0); CHIPS];
    let mut x = SEQ_LEFT;
    for (c, slot) in boxes.iter_mut().enumerate() {
        let label = chip_label(seq, c);
        let check = if done(seq.first + c) { CHECK_ROOM } else { 0.0 };
        let w = CHIP_PAD * 2.0 + est_width(&label, FONT_SUB) + check;
        *slot = (x, w);
        x += w + ARROW_ROOM;
    }
    boxes
}

const CHIP_PAD: f32 = 14.0;
const CHECK_ROOM: f32 = 24.0;
const ARROW_ROOM: f32 = 44.0;
const GAP: f32 = 10.0;

fn chip_label(seq: &SeqView, c: usize) -> Text {
    let mut t = Text::EMPTY;
    let _ = write!(t, "{}  ", seq.first + c + 1);
    push_text(&mut t, &seq.chips[c]);
    t
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

struct Plan<'a> {
    helper: &'a Helper,
    name: &'a Text,
    seq: Option<&'a SeqView>,
}

impl Plan<'_> {
    /// Cell `c`'s centre and the backdrop's width: the strip-wide spread for a sequence, the combo bar's otherwise.
    fn cell_geometry(&self, c: usize) -> (f32, f32) {
        match self.seq {
            Some(_) => (SEQ_CELL_X0 + c as f32 * SEQ_CELL_PITCH, SEQ_CELL_W),
            None => (CELL_X0 + c as f32 * CELL_PITCH, CELL_W),
        }
    }

    fn chip_state(&self, seq: &SeqView, k: usize) -> ChipState {
        match waiting_after(self.helper, seq) {
            Some(Advance::Finished) => ChipState::Done,
            Some(Advance::Dropped) if seq.last.is_some_and(|s| s.step == k) => ChipState::Dropped,
            _ if k < seq.at => ChipState::Done,
            _ if k == seq.at => ChipState::Current,
            _ => ChipState::Later,
        }
    }

    /// The sequence's panes, header, chips and footer (mockup `r8-sequence-bar`).
    fn seq_look(&self, i: usize, seq: &SeqView) -> Look {
        use at::*;
        let total = self.helper.target().map_or(0, |t| t.len());
        let tolerance = self.helper.settings().tolerance;
        let left_text = |left: f32, y: f32, box_w: f32, text: Text, color: [u8; 4]| Look::text(left + box_w / 2.0, y, text, color);
        match i {
            SEQ_BACKDROP => Look::rect(0.0, SEQ_BAND_Y, SEQ_W, BACKDROP_COLOR),
            SEQ_PILL => Look::rect(SEQ_LEFT + SEQ_PILL_W / 2.0, SEQ_HEADER_Y, SEQ_PILL_W, SEQ_PILL_COLOR),
            SEQ_PILL_TEXT => Look::text(SEQ_LEFT + SEQ_PILL_W / 2.0, SEQ_HEADER_Y, Text::new("SEQUENCE"), SEQ_PILL_TEXT_COLOR),
            SEQ_STREAK | SEQ_BEST => {
                let mut streak = Text::EMPTY;
                let _ = write!(streak, "streak {}", seq.streak);
                let mut best = Text::EMPTY;
                let _ = write!(best, "· best {}", seq.best);
                if i == SEQ_BEST {
                    Look::text(SEQ_RIGHT - SEQ_TAG_BOX / 2.0, SEQ_HEADER_Y, best, GOOD)
                } else {
                    let right = SEQ_RIGHT - est_width(&best, FONT_SUB) - 6.0;
                    Look::text(right - SEQ_TAG_BOX / 2.0, SEQ_HEADER_Y, streak, GOOD)
                }
            }
            SEQ_NAME..=SEQ_TOLERANCE => {
                let mut x = SEQ_LEFT + SEQ_PILL_W + 14.0;
                let mut texts = [(Text::EMPTY, TEXT, FONT_SUB, SEQ_NAME_BOX); 4];
                texts[0].0 = seq.name;
                let _ = write!(texts[1].0, "combo {} of {} ·", seq.at + 1, seq.steps);
                texts[2].0 = seq.current();
                let _ = write!(texts[3].0, "· {total} {}", if total == 1 { "link" } else { "links" });
                for t in &mut texts[1..] {
                    t.1 = MUTED;
                    t.2 = FONT_TAG;
                    t.3 = SEQ_TAG_BOX;
                }
                let mut tol = Text::EMPTY;
                let _ = write!(tol, "· tolerance ±{tolerance}f");
                let all = [texts[0], texts[1], texts[2], texts[3], (tol, MUTED, FONT_TAG, SEQ_TAG_BOX)];
                for (k, (text, color, font, box_w)) in all.into_iter().enumerate() {
                    if k == i - SEQ_NAME {
                        return left_text(x, SEQ_HEADER_Y, box_w, text, color);
                    }
                    x += est_width(&text, font) + if k == 0 { 16.0 } else { 6.0 };
                }
                Look::HIDDEN
            }
            SEQ_LANDED | SEQ_CURRENT | RESULT | REASON => self.seq_footer(i, seq, total),
            i if (SEQ_CHIP_EDGE..SEQ_ARROW).contains(&i) => {
                let c = (i - SEQ_CHIP_EDGE) % CHIPS;
                let k = seq.first + c;
                if k >= seq.steps {
                    return Look::HIDDEN;
                }
                let state = self.chip_state(seq, k);
                let boxes = chip_boxes(seq, |k| k < seq.steps && self.chip_state(seq, k) == ChipState::Done);
                let (left, w) = boxes[c];
                let (edge, ink) = match state {
                    ChipState::Done => (GOOD, GOOD),
                    ChipState::Current => (TEXT, TEXT),
                    ChipState::Dropped => (BAD, BAD),
                    ChipState::Later => (RULE, MUTED),
                };
                let label = chip_label(seq, c);
                // The check's lowest point, after the name.
                let bx = left + CHIP_PAD + est_width(&label, FONT_SUB) + 8.0;
                let by = SEQ_CHIP_Y - 5.0;
                let stroke = |len: f32, deg: f32| {
                    let (s, co) = deg.to_radians().sin_cos();
                    Look { rot: deg, ..Look::rect(bx + co * len / 2.0, by + s * len / 2.0, len, GOOD) }
                };
                match (i - SEQ_CHIP_EDGE) / CHIPS {
                    0 => Look::rect(left + w / 2.0, SEQ_CHIP_Y, w, edge),
                    1 => Look::rect(left + w / 2.0, SEQ_CHIP_Y, w - 4.0, SEQ_CHIP_COLOR),
                    2 => left_text(left + CHIP_PAD, SEQ_CHIP_Y, SEQ_CHIP_TEXT_BOX, label, ink),
                    3 if state == ChipState::Done => stroke(9.0, 135.0),
                    4 if state == ChipState::Done => stroke(16.0, 50.0),
                    _ => Look::HIDDEN,
                }
            }
            i if (SEQ_ARROW..COUNT).contains(&i) => {
                let a = i - SEQ_ARROW;
                if seq.first + a + 1 >= seq.steps {
                    return Look::HIDDEN;
                }
                let boxes = chip_boxes(seq, |k| k < seq.steps && self.chip_state(seq, k) == ChipState::Done);
                let (left, w) = boxes[a];
                Look::text(left + w + ARROW_ROOM / 2.0, SEQ_CHIP_Y, Text::new("→"), MUTED)
            }
            _ => Look::HIDDEN,
        }
    }

    /// The footer: "Ryu BnB landed · Nair loop combo: link 1 of 4 true"; after a drop "Nair loop combo dropped ·
    /// L2 Attack 3f late · restart at combo 1"; after a finished run "sequence done · Ryu BnB: ready: 1 link".
    fn seq_footer(&self, i: usize, seq: &SeqView, total: usize) -> Look {
        use at::*;
        let mut parts = [(Text::EMPTY, TEXT, SEQ_SUB_BOX); 4];
        let [landed, reason, current, result] = &mut parts;
        let attempt = self.helper.attempt();
        match waiting_after(self.helper, seq) {
            Some(Advance::Dropped) => {
                let last = seq.last.map(|s| s.summary);
                push_text(&mut landed.0, &seq.last_name);
                let _ = landed.0.write_str(" dropped ·");
                landed.1 = BAD;
                reason.0 = match last {
                    Some(s) if s.fault.is_some() => reason_text(&s),
                    Some(s) if s.outcome == Outcome::Cleared => Text::new("cleared"),
                    _ => Text::new("the exchange ended"),
                };
                reason.1 = WARN;
                current.0 = Text::new("· restart at combo 1");
                current.1 = MUTED;
            }
            waiting => {
                if waiting == Some(Advance::Finished) {
                    landed.0 = Text::new("sequence done ·");
                    landed.1 = GOOD;
                } else if attempt.is_some() && seq.last.is_some_and(|s| matches!(s.advance, Advance::Next(_))) {
                    push_text(&mut landed.0, &seq.last_name);
                    let _ = landed.0.write_str(" landed ·");
                }
                push_text(&mut current.0, &seq.current());
                let _ = current.0.write_char(':');
                match (attempt, attempt.and_then(|a| a.summary)) {
                    (_, Some(s)) => {
                        result.0 = result_text(&s);
                        result.1 = if s.clean { GOOD } else { TEXT };
                    }
                    (Some(a), None) if !a.is_empty() => {
                        let newest_true = a.grades().last().is_some_and(Grade::is_true);
                        let _ = write!(result.0, "link {} of {total}{}", a.len(), if newest_true { " true" } else { "" });
                    }
                    _ => {
                        let _ = write!(result.0, "ready: {total} {}", if total == 1 { "link" } else { "links" });
                        result.1 = MUTED;
                    }
                }
            }
        }
        reason.2 = 400.0;
        result.2 = 240.0;
        let order = [SEQ_LANDED, REASON, SEQ_CURRENT, RESULT];
        let mut x = SEQ_LEFT;
        for (pane, (text, color, box_w)) in order.into_iter().zip(parts) {
            if text.units().is_empty() {
                if pane == i {
                    return Look::HIDDEN;
                }
                continue;
            }
            if pane == i {
                return Look::text(x + box_w / 2.0, FOOTER_Y, text, color);
            }
            x += est_width(&text, FONT_SUB) + GAP;
        }
        Look::HIDDEN
    }

    /// Cell `c`'s link: its reference and, once graded, its grade.
    fn cell(&self, c: usize) -> Option<(usize, LinkRec, Option<Grade>)> {
        let target = self.helper.target()?;
        let attempt = self.helper.attempt();
        let k = first_cell(attempt.map_or(0, |a| a.len()), target.len()) + c;
        let reference = *target.get(k)?;
        Some((
            k,
            reference,
            attempt.and_then(|a| a.grades().nth(k).copied()),
        ))
    }

    fn look(&self, i: usize) -> Look {
        use at::*;
        let last = self.helper.attempt().and_then(|a| a.summary);
        let total = self.helper.target().map_or(0, |t| t.len());
        if let Some(seq) = self.seq {
            match i {
                CONTAINER => {
                    return Look {
                        visible: true,
                        x: 0.0,
                        y: SEQ_BAR_Y,
                        ..Look::HIDDEN
                    }
                }
                BACKDROP => return Look::rect(0.0, 0.0, SEQ_W, BACKDROP_COLOR),
                HEADER | STREAK => return Look::HIDDEN,
                i if (PILL..CELL).contains(&i) => return Look::HIDDEN,
                RESULT | REASON => return self.seq_look(i, seq),
                i if i >= SEQ_BACKDROP => return self.seq_look(i, seq),
                _ => return self.cell_look(i),
            }
        }
        match i {
            CONTAINER => Look {
                visible: true,
                x: 0.0,
                y: BAR_Y,
                ..Look::HIDDEN
            },
            BACKDROP => Look::rect(0.0, 0.0, BAR_W, BACKDROP_COLOR),
            i if i >= SEQ_BACKDROP => Look::HIDDEN,
            HEADER => {
                let name = if self.name.units().is_empty() {
                    Text::new("Combo")
                } else {
                    *self.name
                };
                Look::text(LEFT + 180.0, HEADER_Y, name, TEXT)
            }
            STREAK => {
                let mut t = Text::EMPTY;
                let _ = write!(
                    t,
                    "streak {}  best {}/{}",
                    self.helper.streak(),
                    self.helper.best(),
                    total
                );
                Look::text(STREAK_X, HEADER_Y, t, MUTED)
            }
            RESULT => match (self.helper.attempt(), last) {
                (_, Some(s)) => Look::text(
                    LEFT + 120.0,
                    FOOTER_Y,
                    result_text(&s),
                    if s.clean { GOOD } else { TEXT },
                ),
                (Some(a), None) => {
                    let mut t = Text::EMPTY;
                    let _ = write!(t, "link {} of {}", a.len(), total);
                    Look::text(LEFT + 120.0, FOOTER_Y, t, TEXT)
                }
                (None, None) => {
                    let mut t = Text::EMPTY;
                    let _ = write!(t, "ready: {total} links");
                    Look::text(LEFT + 120.0, FOOTER_Y, t, MUTED)
                }
            },
            REASON => match last {
                Some(s) if s.fault.is_some() => Look::text(
                    LEFT + 460.0,
                    FOOTER_Y,
                    reason_text(&s),
                    if s.clean { GOOD } else { WARN },
                ),
                Some(s) if s.clean => match shown_drift(self.helper) {
                    Some(dir) => Look::text(LEFT + 460.0, FOOTER_Y, drift_text(dir), MUTED),
                    None => Look::HIDDEN,
                },
                _ => Look::HIDDEN,
            },
            i if i < CELL => match self.helper.results().nth(i - PILL) {
                Some(s) => Look::rect(
                    PILL_X0 + (i - PILL) as f32 * PILL_PITCH,
                    HEADER_Y,
                    PILL_SIZE,
                    pill_color(s),
                ),
                None => Look::HIDDEN,
            },
            _ => self.cell_look(i),
        }
    }

    fn cell_look(&self, i: usize) -> Look {
        use at::*;
        let tolerance = self.helper.settings().tolerance;
        let c = (i - CELL) % CELLS;
        let Some((k, reference, grade)) = self.cell(c) else {
            return Look::HIDDEN;
        };
        let (cx, cell_w) = self.cell_geometry(c);
        let ruler_x = |_: usize, frames: i32| ruler_at(cx, frames);
        let first_step = reference.events[0];
        match (i - CELL) / CELLS {
            0 => Look::rect(cx, CELL_Y, cell_w, CELL_COLOR),
            1 => {
                let mut t = Text::EMPTY;
                let _ = write!(
                    t,
                    "{} {}>{}",
                    k + 1,
                    reference.from.name(),
                    reference.to.name()
                );
                Look::text(cx, CELL_Y + TITLE_DY, t, MUTED)
            }
            2 => match grade {
                Some(g) if g.wrong_move => {
                    Look::text(cx - 48.0, CELL_Y + VERDICT_DY, Text::new("WRONG MOVE"), BAD)
                }
                Some(g) => Look::text(
                    cx - 48.0,
                    CELL_Y + VERDICT_DY,
                    verdict_text(g.link.verdict),
                    if g.is_true() { GOOD } else { BAD },
                ),
                None => Look::text(
                    cx - 48.0,
                    CELL_Y + VERDICT_DY,
                    verdict_text(reference.verdict),
                    RULE,
                ),
            },
            3 => match (grade.map(|g| g.marks[0]), first_step) {
                (Some(Some(mark)), Some(step)) => {
                    let (t, color) = mark_text(mark, event_label(step.kind), tolerance);
                    Look::text(cx + 50.0, CELL_Y + VERDICT_DY, t, color)
                }
                _ => Look::HIDDEN,
            },
            4 => Look::rect(cx, CELL_Y + RULER_DY, RULER_W, RULE),
            5 => match grade.map_or(reference.window, |g| g.link.window) {
                Some((a, b)) => {
                    let (x0, x1) = (ruler_x(c, a), ruler_x(c, b + 1));
                    Look::rect((x0 + x1) / 2.0, CELL_Y + RULER_DY, (x1 - x0).max(2.0), GOOD)
                }
                None => Look::HIDDEN,
            },
            6 => match first_step {
                Some(step) => Look::rect(ruler_x(c, step.offset), CELL_Y + RULER_DY, 6.0, MUTED),
                None => Look::HIDDEN,
            },
            _ => match (grade, first_step) {
                (Some(g), Some(step)) => match g.marks[0] {
                    Some(Mark::At(d)) => {
                        let color = if d.unsigned_abs() <= u32::from(tolerance) {
                            GOOD
                        } else {
                            WARN
                        };
                        Look::rect(ruler_x(c, step.offset + d), CELL_Y + RULER_DY, 6.0, color)
                    }
                    _ => Look::HIDDEN,
                },
                _ => Look::HIDDEN,
            },
        }
    }
}

/// What was last written to each of the bar's panes.
pub struct Renderer {
    written: [Option<Look>; COUNT],
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub const fn new() -> Self {
        Self {
            written: [None; COUNT],
        }
    }

    pub fn invalidate(&mut self) {
        self.written = [None; COUNT];
    }

    /// One UI tick (the strip's rule): hidden writes only the container, once; shown writes every pane that changed.
    /// Shown needs the strip shown, the helper On and a target. `name` is the target's name. A drilled sequence
    /// (`seq`) draws the sequence bar. Returns the writes.
    pub fn draw(
        &mut self,
        helper: &Helper,
        name: &Text,
        seq: Option<&SeqView>,
        strip_shown: bool,
        mut apply: impl FnMut(usize, &Look, Changed),
    ) -> usize {
        let shown = strip_shown && helper.settings().on && helper.target().is_some();
        let mut writes = 0;
        if !shown {
            let hidden = Look::HIDDEN;
            if Changed::between(self.written[at::CONTAINER].as_ref(), &hidden).any() {
                apply(
                    at::CONTAINER,
                    &hidden,
                    Changed {
                        visible: true,
                        ..Changed::default()
                    },
                );
                self.written[at::CONTAINER] = Some(hidden);
                writes += 1;
            }
            return writes;
        }
        let plan = Plan { helper, name, seq };
        for i in 0..COUNT {
            let look = plan.look(i);
            let changed = Changed::between(self.written[i].as_ref(), &look);
            if changed.any() {
                apply(i, &look, changed);
                writes += 1;
            }
            self.written[i] = Some(if look.visible {
                look
            } else {
                Look {
                    visible: false,
                    ..self.written[i].unwrap_or(look)
                }
            });
        }
        writes
    }
}

#[cfg(test)]
#[path = "combo_plan_tests.rs"]
mod tests;
