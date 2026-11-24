//! The frame strip's panes (SPEC-input-06 §3.2–3.4, §5.2): where each of the strip's panes goes and what it shows for
//! a tracker, and the renderer that writes only what changed. Positions are the native canvas (1920 x 1080, y up),
//! relative to the strip's container. Pure: the runtime resolves the panes by name once per layout registration and
//! applies what `Renderer::draw` hands it.

use core::fmt::Write as _;

use super::frame_strip::{Aim, Cell, Link, Move, Tracker, Verdict, WINDOW};
use super::panes::strip_layout::{LayoutSpec, Place, strip::StripSpec};
#[cfg(test)]
use super::panes::strip_layout::C;
use super::spacing::Grade;

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
/// Dark text for a label that sits on a light block (the LAST pills' text, a hit block's spot label).
pub const INK: [u8; 4] = [10, 14, 20, 255];

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

    fn from_units(units: &[u16]) -> Text {
        let mut text = Text::EMPTY;
        for &unit in units.iter().take(TEXT_UNITS) {
            text.units[usize::from(text.len)] = unit;
            text.len += 1;
        }
        text
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

/// The LINK sub-line has a dedicated native text box. Keep its extra capacity out of the pane cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkLine {
    units: [u16; LINK_UNITS as usize],
    len: usize,
}

impl LinkLine {
    fn from_units(units: impl IntoIterator<Item = u16>) -> Self {
        let mut line = Self { units: [0; LINK_UNITS as usize], len: 0 };
        for (out, unit) in line.units.iter_mut().zip(units) {
            *out = unit;
            line.len += 1;
        }
        line
    }

    pub fn units(&self) -> &[u16] {
        &self.units[..self.len]
    }
}

/// The game's font has cardinal arrows, but not diagonal arrows.
pub const fn arrow(dir: u8) -> &'static str {
    match dir {
        0 => "→",
        1 => "↑→",
        2 => "↑",
        3 => "↑←",
        4 => "←",
        5 => "↓←",
        6 => "↓",
        7 => "↓→",
        _ => "",
    }
}

/// The newest two direction marks visible on the strip.
pub fn aims_shown(tracker: &Tracker) -> [Option<Aim>; 2] {
    let mut aims = tracker.aims().filter(|aim| tracker.columns().any(|column| column.frame == aim.frame));
    [aims.next(), aims.next()]
}

/// The marks the lane can draw, oldest first, for the bridge. `boxes[n]`: Player 1's `n`th label box exists (the newest mark takes the first).
pub fn aims_drawn(tracker: &Tracker, boxes: [bool; 2]) -> [Option<u8>; 2] {
    let [newest, older] = aims_shown(tracker).map(|aim| aim.map(|aim| aim.dir));
    let (newest, older) = (newest.filter(|_| boxes[0]), older.filter(|_| boxes[1]));
    [older.or(newest), older.and(newest)]
}

/// The two move names and their direction marks, fitted to the given capacity.
pub fn link_moves(link: &Link, capacity: usize) -> LinkLine {
    let moves = [link.from, link.to];
    let names = {
        let namer = super::move_names::Namer::live();
        core::array::from_fn::<_, 2, _>(|i| namer.name(link.shown[i].of.filter(|_| moves[i] == Move::Other).unwrap_or(moves[i]), super::move_names::Normal::Unknown))
    };
    let names = names.each_ref().map(|name| name.as_str());
    let arrows = link.shown.map(|shown| shown.dirs.map(|dir| dir.map_or("", arrow)));
    let space = |s: &str| if s.is_empty() { "" } else { " " };
    let loose = [names[0], space(arrows[0][0]), arrows[0][0], space(arrows[0][1]), arrows[0][1], " > ",
                 names[1], space(arrows[1][0]), arrows[1][0], space(arrows[1][1]), arrows[1][1]];
    let tight = [names[0], "", arrows[0][0], "", arrows[0][1], ">", names[1], "", arrows[1][0], "", arrows[1][1]];
    let parts = if loose.iter().map(|s| s.encode_utf16().count()).sum::<usize>() <= capacity { loose } else { tight };
    LinkLine::from_units(parts.into_iter().flat_map(str::encode_utf16))
}

pub fn folded_link(link: &Link) -> LinkLine {
    let verdict = if matches!(link.verdict, Verdict::True(_)) { "TRUE" } else { "DROP" };
    let prefix = [verdict, " · "];
    let capacity = LINK_UNITS as usize - prefix.iter().map(|s| s.encode_utf16().count()).sum::<usize>();
    let moves = link_moves(link, capacity);
    LinkLine::from_units(prefix.into_iter().flat_map(str::encode_utf16).chain(moves.units().iter().copied()))
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
    pub(crate) const HIDDEN: Look = Look { visible: false, x: 0.0, y: 0.0, w: 0.0, rot: 0.0, color: [0; 4], text: Text::EMPTY };

    pub(crate) const fn at(self, place: Place) -> Self {
        Self { x: place.x, y: place.y, w: place.w, rot: place.rot, ..self }
    }

    fn paint(place: Place, color: [u8; 4]) -> Self {
        Self::rect(0.0, 0.0, 0.0, color).at(place)
    }

    fn label(place: Place, text: Text, color: [u8; 4]) -> Self {
        Self::text(0.0, 0.0, text, color).at(place)
    }

    pub(crate) const fn rect(x: f32, y: f32, w: f32, color: [u8; 4]) -> Look {
        Look { visible: true, x, y, w, rot: 0.0, color, text: Text::EMPTY }
    }

    pub(crate) fn text(x: f32, y: f32, text: Text, color: [u8; 4]) -> Look {
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
    pub(crate) fn between(old: Option<&Look>, new: &Look) -> Changed {
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
struct Content<'a> {
    spec: &'a StripSpec,
    history: bool,
    cells: bool,
    tracker: &'a Tracker,
    aims: [Option<Aim>; 2],
    frames: [u32; WINDOW],
    /// Left-stick bin in each slot, `None` for a neutral frame or a slot with no column.
    sticks: [Option<u8>; WINDOW],
    first_slot: usize,
    runs: [[Option<Run>; RUNS]; 2],
    /// Spot labels on the hit blocks (rc.14, SP3).
    spots: bool,
    /// Parry practice (rc.15): the PARRY and SET readouts, its window and its LAST pills replace the link's.
    parry: Option<&'a super::parry::View>,
}

impl<'a> Content<'a> {
    fn new(
        tracker: &'a Tracker,
        spots: bool,
        parry: Option<&'a super::parry::View>,
        spec: &'a StripSpec,
        history: bool,
        cells: bool,
    ) -> Self {
        let mut frames = [0u32; WINDOW];
        let mut sticks = [None; WINDOW];
        let first_slot = WINDOW - tracker.len();
        for (i, column) in tracker.columns().enumerate() {
            frames[first_slot + i] = column.frame;
            sticks[first_slot + i] = column.stick;
        }
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
        Self { spec, history, cells, tracker, aims: aims_shown(tracker), frames, sticks, first_slot, runs, spots, parry }
    }

    /// The slot a frame is drawn in; a frame after the newest counts on past the right edge (a predicted free frame).
    fn slot(&self, frame: u32) -> Option<f32> {
        let newest = self.tracker.newest()?;
        if frame > newest {
            return Some((WINDOW - 1) as f32 + (frame - newest) as f32);
        }
        (self.first_slot..WINDOW).find(|s| self.frames[*s] == frame).map(|s| s as f32)
    }

    fn look(&self, i: usize) -> Look {
        use at::*;
        match i {
            CONTAINER => Look { visible: true, ..Look::HIDDEN }.at(self.spec.container()),
            BACKDROP => {
                let [x, y, w, _] = self.spec.bounds(self.history, self.cells);
                Look::paint(Place::new(x, y, w), BACKDROP_COLOR)
            }
            i if (LANE_LABEL..LANE_LABEL + 3).contains(&i) => {
                let lane = i - LANE_LABEL;
                let (name, color) = [("IN", MUTED), ("P1", P1_TAG), ("CPU", CPU_TAG)][lane];
                Look::label(self.spec.lane_label(lane), Text::new(name), color)
            }
            SEPARATOR => Look::paint(self.spec.rule(0), RULE),
            i if i == SEPARATOR + 1 && self.history => Look::paint(self.spec.rule(1), RULE),
            i if (RUN..RUN + 2 * RUNS).contains(&i) => {
                let (lane, n) = ((i - RUN) / RUNS, (i - RUN) % RUNS);
                match self.runs[lane][n] {
                    Some(run) => Look::paint(self.spec.lane_run(lane, run.slot, run.len, false), run.cell.color()),
                    None => Look::HIDDEN,
                }
            }
            i if (RUN_LABEL..RUN_LABEL + 2 * RUN_LABELS).contains(&i) => {
                let (lane, n) = ((i - RUN_LABEL) / RUN_LABELS, (i - RUN_LABEL) % RUN_LABELS);
                let aims = if lane == 0 { self.aims } else { [None; 2] };
                let count = aims.iter().flatten().count();
                if n < count {
                    let Some(aim) = aims[n] else { return Look::HIDDEN };
                    let Some(slot) = self.slot(aim.frame).filter(|slot| *slot < WINDOW as f32) else { return Look::HIDDEN };
                    let slot = slot as usize;
                    let cell = self.runs[0].iter().flatten().find(|run| run.slot <= slot && slot < run.slot + run.len).map(|run| run.cell);
                    let color = if matches!(cell, Some(Cell::Startup | Cell::Hitlag)) { INK } else { TEXT };
                    return Look::label(self.spec.lane_run(0, slot, 1, true), Text::new(arrow(aim.dir)), color);
                }
                match self.runs[lane].iter().flatten().filter_map(|run| self.run_label(lane, run).map(|label| (run, label))).nth(n - count) {
                    Some((run, (text, color))) => Look::label(self.spec.lane_run(lane, run.slot, run.len, true), text, color),
                    None => Look::HIDDEN,
                }
            }
            i if (CHIP_BOX..WINDOW_BAR).contains(&i) => self.chip((i - CHIP_BOX) / CHIPS, (i - CHIP_BOX) % CHIPS),
            WINDOW_BAR | WINDOW_TEXT => self.window(i == WINDOW_TEXT),
            i if (BRACKET..BRACKET + 6).contains(&i) => self.bracket((i - BRACKET) / 3, Some((i - BRACKET) % 3)),
            i if (BRACKET_TEXT..BRACKET_TEXT + 2).contains(&i) => self.bracket(i - BRACKET_TEXT, None),
            NOW => Look::paint(self.spec.now(0), MUTED),
            NOW_TEXT => Look::label(self.spec.now(1), Text::new("now"), MUTED),
            i if (READ_LABEL..READ_LABEL + 3).contains(&i) => {
                let n = i - READ_LABEL;
                let names = if self.parry.is_some() { ["ADVANTAGE", "PARRY", "SET"] } else { ["ADVANTAGE", "LINK", "INPUT"] };
                Look::label(self.spec.readout(n, 0), Text::new(names[n]), MUTED)
            }
            i if (READ_VALUE..READ_SUB + 3).contains(&i) => self.readout((i - READ_VALUE) % 3, i >= READ_SUB),
            LAST_TEXT if self.history => Look::label(self.spec.last_label(), Text::new("LAST"), MUTED),
            i if self.history && (PILL..PILL + 3 * HISTORY).contains(&i) => self.last((i - PILL) / HISTORY, (i - PILL) % HISTORY),
            i if (STICK..STICK + WINDOW).contains(&i) => self.stick_mark(i - STICK),
            i if (TAG_BOX..TAG_TEXT).contains(&i) => self.tag(i - TAG_BOX, false),
            i if (TAG_TEXT..COUNT).contains(&i) => self.tag(i - TAG_TEXT, true),
            _ => Look::HIDDEN,
        }
    }

    /// A run's label, centred on it: its length (a long hitstun, recovery or shieldstun) or, with the spot labels on, the spot of the hit that
    /// began a hitlag block of this lane (rc.14, SP3): the lane of the fighter that was hit.
    fn run_label(&self, lane: usize, run: &Run) -> Option<(Text, [u8; 4])> {
        if run.cell.counted() && run.len >= 7 {
            let mut text = Text::EMPTY;
            let _ = write!(text, "{}", run.len);
            return Some((text, TEXT));
        }
        let frames = self.frames[run.slot]..=self.frames[run.slot + run.len - 1];
        let mark = self.tracker.marks().find(|m| usize::from(m.lane) == lane && frames.contains(&m.frame));
        mark.filter(|_| self.spots && run.cell == Cell::Hitlag).map(|m| (Text::new(m.spot.name()), INK))
    }

    /// One `>` for this slot, rotated so rot 0 points right and each bin is 45 degrees counter-clockwise.
    /// A neutral frame is hidden. One pane, not a shaft and a head: `>` is already in the font and is not symmetric.
    fn stick_mark(&self, slot: usize) -> Look {
        let Some(dir) = self.sticks[slot] else { return Look::HIDDEN };
        Look::label(self.spec.stick(slot, dir), Text::new(">"), TEXT)
    }

    fn tag(&self, n: usize, text: bool) -> Look {
        let Some(tag) = self.tracker.tags().nth(n) else { return Look::HIDDEN };
        let Some(slot) = self.slot(tag.frame).filter(|slot| *slot < WINDOW as f32) else { return Look::HIDDEN };
        let grade = match tag.grade { Grade::Excellent => 0, Grade::Good => 1, Grade::Ok => 2 };
        let place = self.spec.tag(slot, usize::from(tag.lane != 0), grade);
        if text {
            Look::label(place, Text::new(tag.grade.name()), INK)
        } else {
            Look::paint(place, tag.grade.color())
        }
    }

    /// Chip part `part` (box, glyph, shaft, head, tick) of the `n`th newest press on screen.
    fn chip(&self, part: usize, n: usize) -> Look {
        let Some((press, slot)) =
            self.tracker.presses().filter_map(|p| self.slot(p.frame).filter(|s| *s < WINDOW as f32).map(|s| (p, s))).nth(n)
        else {
            return Look::HIDDEN;
        };
        let place = self.spec.chip(slot, press.stick, part);
        match part {
            0 => Look::paint(place, CHIP_BACK),
            1 => Look::label(place, Text::glyph(press.button.glyph()), press.button.color()),
            2 | 3 if press.stick.is_some() => Look::paint(place, TEXT),
            4 => Look::paint(place, [255, 255, 255, 170]),
            _ => Look::HIDDEN,
        }
    }

    fn window(&self, text: bool) -> Look {
        let window = match self.parry {
            Some(p) => p.last[0].and_then(|a| a.window).map(|(g, w)| (g, g + u32::from(w).max(1) - 1)),
            None => self.tracker.latest_link().and_then(|l| l.window),
        };
        let Some((from, to)) = window else { return Look::HIDDEN };
        let (Some(a), Some(b)) = (self.slot(from).or(Some(0.0)), self.slot(to)) else { return Look::HIDDEN };
        let (a, b) = (a.min(WINDOW as f32), b.min(WINDOW as f32 - 1.0));
        if b < a {
            return Look::HIDDEN;
        }
        let place = self.spec.window(a, b, text);
        if text {
            let mut t = Text::EMPTY;
            let _ = write!(t, "window {}f", to - from + 1);
            return Look::label(place, t, GOOD);
        }
        Look::paint(place, GOOD)
    }

    /// Bracket `which` (0 link, 1 advantage): its part (left leg, bottom, right leg) or its text (`None`).
    fn bracket(&self, which: usize, part: Option<usize>) -> Look {
        if self.parry.is_some() && which == 0 {
            return Look::HIDDEN;
        }
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
        let place = self.spec.bracket(a, b, part);
        if part.is_some() { Look::paint(place, color) } else { Look::label(place, text, color) }
    }

    /// Readout `n` (ADVANTAGE, LINK, INPUT): its value or its sub-line.
    fn readout(&self, n: usize, sub: bool) -> Look {
        let parry = self.parry.filter(|_| n > 0).map(|p| match (n, p.last[0]) {
            (1, Some(a)) => graded(a.grade),
            (1, None) => (Text::new("-"), Text::new("Parry the replay"), MUTED),
            _ => (p.set, p.set_line, TEXT),
        });
        let (value, line, color) = if let Some(parry) = parry { parry } else { match n {
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
        } };
        if sub {
            Look::label(self.spec.readout(n, 2), line, TEXT)
        } else {
            Look::label(self.spec.readout(n, 1), value, color)
        }
    }

    /// LAST row part (pill, pill text, moves) of the `n`th newest link.
    fn last(&self, part: usize, n: usize) -> Look {
        let place = self.spec.last(part, n);
        let (color, label, link) = if let Some(p) = self.parry {
            let Some(a) = p.last[n] else { return Look::HIDDEN };
            let (_, label, color) = graded(a.grade);
            (color, label, None)
        } else {
            let Some(link) = self.tracker.history().nth(n) else { return Look::HIDDEN };
            match link.verdict {
                Verdict::True(Some(v)) => (GOOD, signed("TRUE ", v), Some(link)),
                Verdict::True(None) => (GOOD, Text::new("TRUE"), Some(link)),
                Verdict::Drop(v) => (BAD, signed("DROP ", v), Some(link)),
            }
        };
        match (part, link) {
            (0, _) => Look::paint(place, color),
            (1, _) => Look::label(place, label, INK),
            (_, None) => Look::HIDDEN,
            (_, Some(link)) => {
                let t = Text::from_units(link_moves(&link, TEXT_UNITS).units());
                Look::label(place, t, TEXT)
            }
        }
    }
}

/// A parry attempt's readout (rc.15): the frames off (0 inside the window, - before it, + after the hit), Tech's own words for the
/// grade (`Grade::write`), and its colour.
fn graded(grade: super::tech::session::Grade) -> (Text, Text, [u8; 4]) {
    use super::tech::session::Grade;
    let mut label = Text::EMPTY;
    let _ = grade.write(&mut label);
    match grade {
        Grade::Perfect(_) => (Text::new("0"), label, GOOD),
        Grade::Early(n) => (signed("", -i32::from(n)), label, WARN),
        Grade::Late(n) => (signed("", i32::from(n)), label, WARN),
        Grade::Missed => (Text::new("-"), label, BAD),
        _ => (Text::new("-"), label, MUTED),
    }
}

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

/// What was last written to each of a bar's `N` panes (the strip here, the combo bar in `combo_plan`).
pub struct Panes<const N: usize> {
    written: [Option<Look>; N],
}

impl<const N: usize> Default for Panes<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Panes<N> {
    pub const fn new() -> Self {
        Self { written: [None; N] }
    }

    /// The cached look for native writes deferred until the model guard is released.
    pub fn written(&self, index: usize) -> Option<&Look> { self.written[index].as_ref() }

    /// The layout was registered again: nothing the plugin wrote is there any more.
    pub fn invalidate(&mut self) {
        self.written = [None; N];
    }

    /// Hidden: only the container is written as `hidden` (and only when it was shown). Returns the writes.
    pub fn hide(&mut self, container: usize, hidden: Look, mut apply: impl FnMut(usize, &Look, Changed)) -> usize {
        if !Changed::between(self.written[container].as_ref(), &hidden).any() {
            return 0;
        }
        apply(container, &hidden, Changed { visible: true, ..Changed::default() });
        self.written[container] = Some(hidden);
        1
    }

    /// Shown: every pane whose look changed, with what changed. Returns the writes.
    pub fn show(&mut self, look: impl Fn(usize) -> Look, mut apply: impl FnMut(usize, &Look, Changed)) -> usize {
        let mut writes = 0;
        for i in 0..N {
            let look = look(i);
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

/// What one strip draw shows around the tracker, and the layout that places it.
#[derive(Clone, Copy)]
pub struct StripDraw<'a> {
    pub shown: bool,
    pub spots: bool,
    pub parry: Option<&'a super::parry::View>,
    pub history_enabled: bool,
    pub cells: bool,
    pub spec: &'a LayoutSpec,
}

/// What was last written to each of the strip's panes.
pub type Renderer = Panes<COUNT>;

impl Renderer {
    /// One UI tick. Hidden: only the container is written (and only when it was shown). Shown: every pane whose look
    /// changed, with what changed. Returns how many panes were written.
    #[cfg(test)]
    pub fn draw(&mut self, tracker: &Tracker, shown: bool, apply: impl FnMut(usize, &Look, Changed)) -> usize {
        self.draw_spots(tracker, shown, false, apply)
    }

    /// `draw`, with the spot labels on the hit blocks when `spots` (rc.14, SP3).
    #[cfg(test)]
    pub fn draw_spots(&mut self, tracker: &Tracker, shown: bool, spots: bool, apply: impl FnMut(usize, &Look, Changed)) -> usize {
        self.draw_parry(tracker, shown, spots, None, apply)
    }

    /// `draw_spots`, with Parry practice's readouts, window and pills when `parry` (rc.15).
    #[cfg(test)]
    pub fn draw_parry(&mut self, tracker: &Tracker, shown: bool, spots: bool, parry: Option<&super::parry::View>, apply: impl FnMut(usize, &Look, Changed)) -> usize {
        self.draw_with_spec(tracker, StripDraw { shown, spots, parry, history_enabled: true, cells: false, spec: &C }, apply).0
    }

    /// The wide LINK line travels separately from the cached panes.
    pub fn draw_with_spec(
        &mut self,
        tracker: &Tracker,
        draw: StripDraw<'_>,
        apply: impl FnMut(usize, &Look, Changed),
    ) -> (usize, Option<LinkLine>, bool) {
        let StripDraw { shown, spots, parry, history_enabled, cells, spec } = draw;
        let history = spec.history_row(history_enabled, parry.is_some(), cells);
        if !shown {
            return (self.hide(at::CONTAINER, Look::HIDDEN, apply), None, history);
        }
        let content = Content::new(tracker, spots, parry, &spec.strip, history, cells);
        let line = if !history_enabled && parry.is_none() {
            tracker.latest_link().as_ref().map(folded_link)
        } else {
            None
        }.unwrap_or_else(|| {
            let look = content.readout(1, true);
            LinkLine::from_units(look.text.units().iter().copied())
        });
        (self.show(|i| content.look(i), apply), Some(line), history)
    }
}

#[cfg(test)]
#[path = "frame_strip_plan_tests.rs"]
mod tests;
