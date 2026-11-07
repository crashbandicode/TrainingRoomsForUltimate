//! The combo bar's panes (SPEC-input-07 §7): what each shows for the helper, and the renderer that writes only what
//! changed (the frame strip's rule, SPEC-input-06 §5.2). Positions are relative to the bar's container `TrCb`.

use core::fmt::Write as _;

use super::combo::{Dir, Flick, Grade, Helper, Kind, LinkRec, Mark, Outcome, Reason, Summary};
use super::frame_strip::{Button, Verdict};
use super::frame_strip_plan::{Changed, Look, Text, BAD, GOOD, MUTED, RULE, TEXT, WARN};
use super::panes::cb::*;
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
    let cx = CELL_X0 + cell as f32 * CELL_PITCH;
    cx - RULER_W / 2.0 + (frames - RULER_FROM).clamp(0, RULER_FRAMES) as f32 * PER_FRAME
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
}

impl Plan<'_> {
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
        match i {
            CONTAINER => Look {
                visible: true,
                x: 0.0,
                y: BAR_Y,
                ..Look::HIDDEN
            },
            BACKDROP => Look::rect(0.0, 0.0, BAR_W, BACKDROP_COLOR),
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
        let cx = CELL_X0 + c as f32 * CELL_PITCH;
        let first_step = reference.events[0];
        match (i - CELL) / CELLS {
            0 => Look::rect(cx, CELL_Y, CELL_W, CELL_COLOR),
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
    /// Shown needs the strip shown, the helper On and a target. `name` is the target's name. Returns the writes.
    pub fn draw(
        &mut self,
        helper: &Helper,
        name: &Text,
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
        let plan = Plan { helper, name };
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
