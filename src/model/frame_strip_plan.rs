//! The frame strip's panes (SPEC-input-06 §3.2–3.4, §5.2): where each of the strip's panes goes and what it shows for
//! a tracker, and the renderer that writes only what changed. Positions are the native canvas (1920 x 1080, y up),
//! relative to the strip's container. Pure: the runtime resolves the panes by name once per layout registration and
//! applies what `Renderer::draw` hands it.

use core::fmt::Write as _;

use super::frame_strip::{Cell, Link, Tracker, Verdict, WINDOW};

pub use super::panes::iv::{at, COUNT};
use super::panes::iv::*;

/// The colours of the strip's own parts (sRGB).
pub const TEXT: [u8; 4] = [236, 240, 246, 255];
pub const MUTED: [u8; 4] = [150, 160, 176, 255];
pub const GOOD: [u8; 4] = [72, 208, 118, 255];
pub const BAD: [u8; 4] = [240, 86, 78, 255];
pub const WARN: [u8; 4] = [244, 186, 64, 255];
pub const RULE: [u8; 4] = [60, 66, 78, 255];
pub const CHIP_BACK: [u8; 4] = [30, 36, 48, 255];
pub const P1_TAG: [u8; 4] = [228, 64, 64, 255];
pub const CPU_TAG: [u8; 4] = [150, 150, 158, 255];

/// A text box's content: UTF-16 units of the game's font (ASCII, or one glyph unit).
pub const TEXT_UNITS: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text {
    units: [u16; TEXT_UNITS],
    len: u8,
}

impl Text {
    pub const EMPTY: Text = Text { units: [0; TEXT_UNITS], len: 0 };

    pub fn new(s: &str) -> Text {
        let mut t = Text::EMPTY;
        let _ = t.write_str(s);
        t
    }

    pub fn glyph(unit: u16) -> Text {
        let mut t = Text::EMPTY;
        t.units[0] = unit;
        t.len = 1;
        t
    }

    pub fn units(&self) -> &[u16] {
        &self.units[..usize::from(self.len)]
    }
}

impl core::fmt::Write for Text {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for unit in s.encode_utf16() {
            if usize::from(self.len) == TEXT_UNITS {
                break;
            }
            self.units[usize::from(self.len)] = unit;
            self.len += 1;
        }
        Ok(())
    }
}

/// What one pane shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub visible: bool,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    /// Rotation about z, degrees.
    pub rot: f32,
    pub color: [u8; 4],
    pub text: Text,
}

impl Look {
    const HIDDEN: Look = Look { visible: false, x: 0.0, y: 0.0, w: 0.0, rot: 0.0, color: [0; 4], text: Text::EMPTY };

    const fn rect(x: f32, y: f32, w: f32, color: [u8; 4]) -> Look {
        Look { visible: true, x, y, w, rot: 0.0, color, text: Text::EMPTY }
    }

    fn text(x: f32, y: f32, text: Text, color: [u8; 4]) -> Look {
        Look { visible: true, x, y, w: 0.0, rot: 0.0, color, text }
    }
}

/// Which parts of a look a write changes (`Renderer::draw` passes them so the runtime writes only those).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Changed {
    pub visible: bool,
    pub pos: bool,
    pub width: bool,
    pub rot: bool,
    pub color: bool,
    pub text: bool,
}

impl Changed {
    fn between(old: Option<&Look>, new: &Look) -> Changed {
        let Some(old) = old else {
            return Changed { visible: true, pos: true, width: true, rot: true, color: true, text: true };
        };
        if !new.visible {
            return Changed { visible: old.visible, ..Changed::default() };
        }
        Changed {
            visible: !old.visible,
            pos: old.x != new.x || old.y != new.y,
            width: old.w != new.w,
            rot: old.rot != new.rot,
            color: old.color != new.color,
            text: old.text != new.text,
        }
    }

    pub fn any(&self) -> bool {
        self.visible || self.pos || self.width || self.rot || self.color || self.text
    }
}

/// One lane run: the first slot, the length in frames and the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run {
    slot: usize,
    len: usize,
    cell: Cell,
}

/// The tracker laid out once per tick: runs, the frames on screen and their slots.
struct Layout<'a> {
    tracker: &'a Tracker,
    frames: [u32; WINDOW],
    first_slot: usize,
    runs: [[Option<Run>; RUNS]; 2],
}

impl<'a> Layout<'a> {
    fn new(tracker: &'a Tracker) -> Self {
        let mut frames = [0u32; WINDOW];
        let first_slot = WINDOW - tracker.len();
        let mut runs = [[None; RUNS]; 2];
        for (lane, out) in runs.iter_mut().enumerate() {
            // Newest runs first: walk the columns from the newest backwards.
            let mut kept = 0;
            let mut current: Option<Run> = None;
            let columns: [Option<(u32, Cell)>; WINDOW] = {
                let mut c = [None; WINDOW];
                for (i, column) in tracker.columns().enumerate() {
                    c[first_slot + i] = Some((column.frame, if lane == 0 { column.p1 } else { column.cpu }));
                }
                c
            };
            for slot in (first_slot..WINDOW).rev() {
                let Some((frame, cell)) = columns[slot] else { continue };
                frames[slot] = frame;
                match current.as_mut() {
                    Some(run) if run.cell == cell => {
                        run.slot = slot;
                        run.len += 1;
                    }
                    _ => {
                        if let Some(run) = current.take() {
                            if kept < RUNS {
                                out[kept] = Some(run);
                                kept += 1;
                            }
                        }
                        current = Some(Run { slot, len: 1, cell });
                    }
                }
            }
            if let Some(run) = current {
                if kept < RUNS {
                    out[kept] = Some(run);
                }
            }
        }
        Self { tracker, frames, first_slot, runs }
    }

    /// The slot a frame is drawn in; a frame after the newest counts on past the right edge (a predicted free frame).
    fn slot(&self, frame: u32) -> Option<f32> {
        let newest = self.tracker.newest()?;
        if frame > newest {
            return Some((WINDOW - 1) as f32 + (frame - newest) as f32);
        }
        (self.first_slot..WINDOW).find(|s| self.frames[*s] == frame).map(|s| s as f32)
    }

    fn centre(slot: f32) -> f32 {
        FIRST_X + slot * PITCH + CELL_W / 2.0
    }

    fn look(&self, i: usize) -> Look {
        use at::*;
        match i {
            CONTAINER => Look { visible: true, ..Look::HIDDEN },
            BACKDROP => Look::rect(0.0, 0.0, STRIP_W, BACKDROP_COLOR),
            i if (LANE_LABEL..LANE_LABEL + 3).contains(&i) => {
                let (name, y, color) = [("IN", IN_Y, MUTED), ("P1", P1_Y, P1_TAG), ("CPU", CPU_Y, CPU_TAG)][i - LANE_LABEL];
                Look::text(LABEL_X, y, Text::new(name), color)
            }
            SEPARATOR => Look::rect(DIVIDER_X, 19.5, 1.5, RULE),
            i if i == SEPARATOR + 1 => Look::rect(0.0, RULE_Y, STRIP_W - 36.0, RULE),
            i if (RUN..RUN + 2 * RUNS).contains(&i) => {
                let (lane, n) = ((i - RUN) / RUNS, (i - RUN) % RUNS);
                match self.runs[lane][n] {
                    Some(run) => {
                        let x = FIRST_X + run.slot as f32 * PITCH + (run.len as f32 * PITCH - (PITCH - CELL_W)) / 2.0;
                        let w = run.len as f32 * PITCH - (PITCH - CELL_W);
                        Look::rect(x, if lane == 0 { P1_Y } else { CPU_Y }, w, run.cell.color())
                    }
                    None => Look::HIDDEN,
                }
            }
            i if (RUN_LABEL..RUN_LABEL + 2 * RUN_LABELS).contains(&i) => {
                let (lane, n) = ((i - RUN_LABEL) / RUN_LABELS, (i - RUN_LABEL) % RUN_LABELS);
                let run = self.runs[lane].iter().flatten().filter(|r| r.cell.counted() && r.len >= 7).nth(n);
                match run {
                    Some(run) => {
                        let x = FIRST_X + (run.slot as f32 + run.len as f32 / 2.0) * PITCH;
                        let mut text = Text::EMPTY;
                        let _ = write!(text, "{}", run.len);
                        Look::text(x, if lane == 0 { P1_Y } else { CPU_Y }, text, TEXT)
                    }
                    None => Look::HIDDEN,
                }
            }
            i if (CHIP_BOX..WINDOW_BAR).contains(&i) => self.chip((i - CHIP_BOX) / CHIPS, (i - CHIP_BOX) % CHIPS),
            WINDOW_BAR | WINDOW_TEXT => self.window(i == WINDOW_TEXT),
            i if (BRACKET..BRACKET + 6).contains(&i) => self.bracket((i - BRACKET) / 3, Some((i - BRACKET) % 3)),
            i if (BRACKET_TEXT..BRACKET_TEXT + 2).contains(&i) => self.bracket(i - BRACKET_TEXT, None),
            NOW => Look::rect(NOW_X, 32.25, 1.5, MUTED),
            NOW_TEXT => Look::text(NOW_X - 24.0, 84.0, Text::new("now"), MUTED),
            i if (READ_LABEL..READ_LABEL + 3).contains(&i) => {
                let n = i - READ_LABEL;
                Look::text(READ_X + 60.0, READ_LABEL_Y[n], Text::new(["ADVANTAGE", "LINK", "INPUT"][n]), MUTED)
            }
            i if (READ_VALUE..READ_SUB + 3).contains(&i) => self.readout((i - READ_VALUE) % 3, i >= READ_SUB),
            LAST_TEXT => Look::text(LABEL_X, LAST_Y, Text::new("LAST"), MUTED),
            i if (PILL..PILL + 3 * HISTORY).contains(&i) => self.last((i - PILL) / HISTORY, (i - PILL) % HISTORY),
            _ => Look::HIDDEN,
        }
    }

    /// Chip part `part` (box, glyph, shaft, head, tick) of the `n`th newest press on screen.
    fn chip(&self, part: usize, n: usize) -> Look {
        let Some((press, slot)) =
            self.tracker.presses().filter_map(|p| self.slot(p.frame).filter(|s| *s < WINDOW as f32).map(|s| (p, s))).nth(n)
        else {
            return Look::HIDDEN;
        };
        let x = Self::centre(slot);
        let arrow = press.stick.is_some();
        let glyph_x = if arrow { x + CHIP * 0.45 } else { x };
        let dir = press.stick.unwrap_or(0);
        let rot = f32::from(dir) * 45.0 - 90.0;
        let (c, s) = DIRECTION[usize::from(dir) % 8];
        let arrow_x = x - CHIP * 0.45;
        match part {
            0 => Look::rect(x, IN_Y, if arrow { CHIP * 1.9 } else { CHIP }, CHIP_BACK),
            1 => Look::text(glyph_x, IN_Y, Text::glyph(press.button.glyph()), press.button.color()),
            2 if arrow => Look { rot, ..Look::rect(arrow_x - 2.0 * c, IN_Y - 2.0 * s, 4.5, TEXT) },
            3 if arrow => Look { rot: rot + 45.0, ..Look::rect(arrow_x + 5.0 * c, IN_Y + 5.0 * s, 9.0, TEXT) },
            4 => Look::rect(x, TICK_Y, 1.5, [255, 255, 255, 170]),
            _ => Look::HIDDEN,
        }
    }

    fn window(&self, text: bool) -> Look {
        let Some((from, to)) = self.tracker.latest_link().and_then(|l| l.window) else { return Look::HIDDEN };
        let (Some(a), Some(b)) = (self.slot(from).or(Some(0.0)), self.slot(to)) else { return Look::HIDDEN };
        let (a, b) = (a.min(WINDOW as f32), b.min(WINDOW as f32 - 1.0));
        if b < a {
            return Look::HIDDEN;
        }
        let left = FIRST_X + a * PITCH;
        let right = FIRST_X + b * PITCH + CELL_W;
        if text {
            let mut t = Text::EMPTY;
            let _ = write!(t, "window {}f", to - from + 1);
            return Look::text(right + 60.0, WINDOW_Y, t, GOOD);
        }
        Look::rect((left + right) / 2.0, WINDOW_Y, right - left, GOOD)
    }

    /// Bracket `which` (0 link, 1 advantage): its part (left leg, bottom, right leg) or its text (`None`).
    fn bracket(&self, which: usize, part: Option<usize>) -> Look {
        let ends = if which == 0 {
            self.tracker.latest_link().and_then(|l| {
                let free = l.free?;
                let color = if matches!(l.verdict, Verdict::True(_)) { GOOD } else { BAD };
                Some((l.hit.min(free), l.hit.max(free), color, signed("link ", l.hit as i32 - free as i32)))
            })
        } else {
            self.tracker.advantage().map(|a| {
                let color = match a.value {
                    v if v > 0 => GOOD,
                    v if v < 0 => BAD,
                    _ => MUTED,
                };
                (a.p1.min(a.cpu), a.p1.max(a.cpu), color, signed("adv ", a.value))
            })
        };
        let Some((from, to, color, text)) = ends else { return Look::HIDDEN };
        let (Some(a), Some(b)) = (self.slot(from), self.slot(to)) else { return Look::HIDDEN };
        let b = b.min(WINDOW as f32);
        let (left, right) = (FIRST_X + a * PITCH, FIRST_X + b * PITCH);
        match part {
            Some(0) => Look::rect(left, LEG_Y, 3.0, color),
            Some(1) => Look::rect((left + right) / 2.0, BAR_Y, (right - left).max(3.0), color),
            Some(_) => Look::rect(right, LEG_Y, 3.0, color),
            None => Look::text((left + right) / 2.0, BRACKET_TEXT_Y, text, color),
        }
    }

    /// Readout `n` (ADVANTAGE, LINK, INPUT): its value or its sub-line.
    fn readout(&self, n: usize, sub: bool) -> Look {
        let (value, line, color) = match n {
            0 => match self.tracker.advantage() {
                Some(a) => (
                    signed("", a.value),
                    Text::new(if a.on_shield { "on shield" } else { "on hit" }),
                    if a.value > 0 { GOOD } else if a.value < 0 { BAD } else { TEXT },
                ),
                None => (Text::new("-"), Text::EMPTY, MUTED),
            },
            1 => match self.tracker.latest_link() {
                Some(Link { verdict: Verdict::True(Some(link)), .. }) => (signed("", link), Text::new("TRUE combo"), GOOD),
                Some(Link { verdict: Verdict::True(None), .. }) => (Text::new("TRUE"), Text::new("TRUE combo"), GOOD),
                Some(Link { verdict: Verdict::Drop(link), .. }) => (signed("", link), Text::new("DROP"), BAD),
                None => (Text::new("-"), Text::EMPTY, MUTED),
            },
            _ => match self.tracker.latest_link() {
                Some(link) => (signed("", link.input), input_line(&link), WARN),
                None => (Text::new("-"), Text::EMPTY, MUTED),
            },
        };
        if sub {
            Look::text(READ_SUB_X + 90.0, READ_VALUE_Y[n] - 1.5, line, TEXT)
        } else {
            Look::text(READ_X + 30.0, READ_VALUE_Y[n], value, color)
        }
    }

    /// LAST row part (pill, pill text, moves) of the `n`th newest link.
    fn last(&self, part: usize, n: usize) -> Look {
        let Some(link) = self.tracker.history().nth(n) else { return Look::HIDDEN };
        let x = PILL_X0 + n as f32 * PILL_STRIDE;
        let (ok, label) = match link.verdict {
            Verdict::True(Some(v)) => (true, signed("TRUE ", v)),
            Verdict::True(None) => (true, Text::new("TRUE")),
            Verdict::Drop(v) => (false, signed("DROP ", v)),
        };
        match part {
            0 => Look::rect(x, LAST_Y, PILL_W, if ok { GOOD } else { BAD }),
            1 => Look::text(x, LAST_Y, label, [10, 14, 20, 255]),
            _ => {
                let mut t = Text::EMPTY;
                let _ = write!(t, "{} > {}", link.from.name(), link.to.name());
                Look::text(x + PILL_W / 2.0 + 100.0, LAST_Y, t, TEXT)
            }
        }
    }
}

/// `cos` and `sin` of the eight stick directions (a table: the float library's would link the math library).
const DIRECTION: [(f32, f32); 8] = [
    (1.0, 0.0),
    (0.707_106_77, 0.707_106_77),
    (0.0, 1.0),
    (-0.707_106_77, 0.707_106_77),
    (-1.0, 0.0),
    (-0.707_106_77, -0.707_106_77),
    (0.0, -1.0),
    (0.707_106_77, -0.707_106_77),
];

/// `prefix` and a signed number: `+3`, `-3`, `0`.
pub fn signed(prefix: &str, v: i32) -> Text {
    let mut t = Text::new(prefix);
    let _ = if v > 0 { write!(t, "+{v}") } else { write!(t, "{v}") };
    t
}

/// The INPUT readout's line (SPEC-input-06 §3.4): when the press came, then the frames to spare or past the window.
pub fn input_line(link: &Link) -> Text {
    let mut t = Text::EMPTY;
    let _ = match link.input {
        0 => write!(t, "on time"),
        n if n > 0 => write!(t, "{n}f late"),
        n => write!(t, "buffered {}f", -n),
    };
    let _ = match link.spare() {
        Some(s) if s >= 0 => write!(t, ", {s} spare"),
        Some(s) => write!(t, ", {}f past", -s),
        None => Ok(()),
    };
    t
}

/// What was last written to each of the strip's panes.
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
        Self { written: [None; COUNT] }
    }

    /// The layout was registered again: nothing the plugin wrote is there any more.
    pub fn invalidate(&mut self) {
        self.written = [None; COUNT];
    }

    /// One UI tick. Hidden: only the container is written (and only when it was shown). Shown: every pane whose look
    /// changed, with what changed. Returns how many panes were written.
    pub fn draw(&mut self, tracker: &Tracker, shown: bool, mut apply: impl FnMut(usize, &Look, Changed)) -> usize {
        let mut writes = 0;
        if !shown {
            let hidden = Look { visible: false, ..Look::HIDDEN };
            let changed = Changed::between(self.written[at::CONTAINER].as_ref(), &hidden);
            if changed.any() {
                apply(at::CONTAINER, &hidden, Changed { visible: true, ..Changed::default() });
                self.written[at::CONTAINER] = Some(hidden);
                writes += 1;
            }
            return writes;
        }
        let layout = Layout::new(tracker);
        for i in 0..COUNT {
            let look = layout.look(i);
            let changed = Changed::between(self.written[i].as_ref(), &look);
            if changed.any() {
                apply(i, &look, changed);
                writes += 1;
            }
            // A hidden pane keeps what it showed last, so showing it again writes only what differs.
            self.written[i] = Some(if look.visible { look } else { Look { visible: false, ..self.written[i].unwrap_or(look) } });
        }
        writes
    }
}

#[cfg(test)]
#[path = "frame_strip_plan_tests.rs"]
mod tests;
