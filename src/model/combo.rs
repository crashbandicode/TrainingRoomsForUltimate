//! The combo helper (SPEC-input-07; operator decision COMBO-HELPER, option A, 2026-11-05): the exchanges the frame
//! strip links, kept as combos; a saved reference; and each attempt graded link by link against it. Pure and
//! allocation-free: it steps once per frame right after the strip's tracker (`model::frame_strip`), on fixed arrays.

use super::frame_strip::{Button, Link, Move, Press, Tracker, Verdict, PRESSES};

/// A name the menu shows (the target, the draft): `Copy`, 48 characters at most, cut at a character boundary.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Label {
    bytes: [u8; 192],
    len: u8,
}

impl Label {
    pub const EMPTY: Label = Label { bytes: [0; 192], len: 0 };

    pub fn new(text: &str) -> Label {
        let mut label = Label::EMPTY;
        for c in text.chars().take(48) {
            let end = usize::from(label.len) + c.len_utf8();
            c.encode_utf8(&mut label.bytes[usize::from(label.len)..end]);
            label.len = end as u8;
        }
        label
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("")
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl core::fmt::Debug for Label {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(f)
    }
}

/// Links of one combo, steps kept per link, results in the history.
pub const LINKS: usize = 16;
pub const STEPS: usize = 4;
pub const RESULTS: usize = 8;

/// A step's aim: the strip's stick bin (0 right, counter-clockwise in eighths) with left and right merged, so the
/// fighter's facing does not matter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aim {
    Neutral,
    Up,
    Down,
    Side,
}

impl Aim {
    pub fn of(stick: Option<u8>) -> Aim {
        match stick {
            None => Aim::Neutral,
            Some(1..=3) => Aim::Up,
            Some(5..=7) => Aim::Down,
            Some(_) => Aim::Side,
        }
    }
}

/// One press of a link's follow-up, relative to the link's A₁ (negative when buffered).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub button: Button,
    pub aim: Aim,
    pub offset: i32,
}

/// One link of a combo: the strip's `Link` reduced to what is compared, plus the presses of its follow-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkRec {
    pub from: Move,
    pub to: Move,
    pub verdict: Verdict,
    /// The combo window relative to A₁.
    pub window: Option<(i32, i32)>,
    pub steps: [Option<Step>; STEPS],
}

impl LinkRec {
    /// `presses` are the follow-up's presses, oldest first; the first `STEPS` are kept.
    pub fn of(link: &Link, presses: impl Iterator<Item = Press>) -> LinkRec {
        let a1 = link.actionable as i32;
        let mut steps = [None; STEPS];
        for (slot, press) in steps.iter_mut().zip(presses) {
            *slot = Some(Step { button: press.button, aim: Aim::of(press.stick), offset: press.frame as i32 - a1 });
        }
        LinkRec {
            from: link.from,
            to: link.to,
            verdict: link.verdict,
            window: link.window.map(|(start, end)| (start as i32 - a1, end as i32 - a1)),
            steps,
        }
    }

    pub fn steps(&self) -> impl Iterator<Item = Step> + '_ {
        self.steps.iter().flatten().copied()
    }

    pub fn is_true(&self) -> bool {
        matches!(self.verdict, Verdict::True(_))
    }
}

/// The links of one exchange, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Combo {
    links: [Option<LinkRec>; LINKS],
    len: usize,
}

impl Default for Combo {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Combo {
    pub const EMPTY: Combo = Combo { links: [None; LINKS], len: 0 };

    /// A combo of 1 to `LINKS` links.
    pub fn from_links(links: impl IntoIterator<Item = LinkRec>) -> Option<Combo> {
        let mut combo = Combo::EMPTY;
        for link in links {
            if !combo.push(link) {
                return None;
            }
        }
        (!combo.is_empty()).then_some(combo)
    }

    /// False when the combo is full.
    pub fn push(&mut self, link: LinkRec) -> bool {
        if self.len == LINKS {
            return false;
        }
        self.links[self.len] = Some(link);
        self.len += 1;
        true
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: usize) -> Option<&LinkRec> {
        self.links.get(index).and_then(Option::as_ref)
    }

    pub fn links(&self) -> impl Iterator<Item = &LinkRec> {
        self.links[..self.len].iter().flatten()
    }
}

/// A reference step's match in an attempt: the attempt's offset minus the reference's, or no press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    At(i32),
    Missing,
}

/// One attempted link against its reference link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grade {
    /// The attempt's own link, and the reference link it was graded against.
    pub link: LinkRec,
    pub reference: LinkRec,
    pub wrong_move: bool,
    /// Per reference step, in order.
    pub marks: [Option<Mark>; STEPS],
    /// The attempt's steps no reference step matched.
    pub extra: u8,
}

impl Grade {
    /// TRUE and the right move (SPEC-input-07 §3).
    pub fn is_true(&self) -> bool {
        self.link.is_true() && !self.wrong_move
    }

    pub fn missing(&self) -> Option<usize> {
        self.marks.iter().position(|m| *m == Some(Mark::Missing))
    }

    /// The step with the largest offset outside the tolerance, with that offset.
    pub fn worst(&self, tolerance: u8) -> Option<(usize, i32)> {
        let mut worst: Option<(usize, i32)> = None;
        for (i, mark) in self.marks.iter().enumerate() {
            if let Some(Mark::At(d)) = mark {
                if d.unsigned_abs() > u32::from(tolerance) && worst.is_none_or(|(_, w)| d.unsigned_abs() > w.unsigned_abs()) {
                    worst = Some((i, *d));
                }
            }
        }
        worst
    }

    /// True, every reference step pressed, and each within the tolerance.
    pub fn clean(&self, tolerance: u8) -> bool {
        self.is_true() && self.missing().is_none() && self.worst(tolerance).is_none()
    }
}

/// Grade `attempt` against `reference`: each reference step, in order, matches the attempt's next unused step with the
/// same button and aim.
pub fn grade(reference: &LinkRec, attempt: &LinkRec) -> Grade {
    let mut used = [false; STEPS];
    let mut marks = [None; STEPS];
    let mut from = 0;
    for (mark, want) in marks.iter_mut().zip(reference.steps.iter()) {
        let Some(want) = want else { continue };
        let found = (from..STEPS).find(|&i| {
            !used[i] && attempt.steps[i].is_some_and(|s| s.button == want.button && s.aim == want.aim)
        });
        *mark = Some(match found {
            Some(i) => {
                used[i] = true;
                from = i + 1;
                Mark::At(attempt.steps[i].map_or(0, |s| s.offset) - want.offset)
            }
            None => Mark::Missing,
        });
    }
    let extra = (0..STEPS).filter(|&i| attempt.steps[i].is_some() && !used[i]).count() as u8;
    Grade { link: *attempt, reference: *reference, wrong_move: attempt.to != reference.to, marks, extra }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Every reference link was graded.
    Completed,
    /// The exchange closed first.
    Stopped,
    /// The strip was cleared (a save-state load, a scene change, a hot swap).
    Cleared,
}

/// Why an attempt was not clean: the first link that was not true, else the first missing press, else the first
/// press outside the tolerance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Drop(i32),
    WrongMove(Move),
    Missing(Button),
    Off(Button, i32),
}

/// An attempt's result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    pub outcome: Outcome,
    /// Links graded, true links among them, and the reference's links.
    pub graded: u8,
    pub true_links: u8,
    pub total: u8,
    pub clean: bool,
    /// The link (0-based) and the reason.
    pub fault: Option<(u8, Reason)>,
}

/// The attempt in progress, or the last one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attempt {
    exchange: u32,
    grades: [Option<Grade>; LINKS],
    len: usize,
    pub summary: Option<Summary>,
}

impl Attempt {
    pub fn grades(&self) -> impl Iterator<Item = &Grade> {
        self.grades[..self.len].iter().flatten()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn open(&self) -> bool {
        self.summary.is_none()
    }
}

/// The rows: *Combo helper* (`[On, Off]`), *On-time tolerance* (0, 1, 2 frames), *After an attempt* (keep playing,
/// load the save state).
pub const ROW_ON: &str = "combo_helper";
pub const ROW_TOLERANCE: &str = "combo_tolerance";
pub const ROW_AFTER: &str = "combo_after";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub on: bool,
    pub tolerance: u8,
    pub load_after: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self::factory()
    }
}

impl Settings {
    pub const fn factory() -> Self {
        Self { on: false, tolerance: 1, load_after: false }
    }

    pub fn from_snapshot(snapshot: &super::settings::Snapshot) -> Self {
        let choice = |id: &str| snapshot.get(id).and_then(|r| r.iter().position(|v| *v != 0));
        let factory = Self::factory();
        Self {
            on: choice(ROW_ON).map_or(factory.on, |i| i == 0),
            tolerance: choice(ROW_TOLERANCE).map_or(factory.tolerance, |i| i.min(2) as u8),
            load_after: choice(ROW_AFTER).map_or(factory.load_after, |i| i == 1),
        }
    }
}

/// What one step asks the runtime to do or report.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Out {
    /// A link of the attempt was graded: its index and grade.
    pub link: Option<(usize, Grade)>,
    pub resolved: Option<Summary>,
    /// *After an attempt: Load save state*: request the load now.
    pub load: bool,
}

/// The exchanges seen, as combos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Capture {
    /// The tracker's exchange count, and the frame after which the next link's presses count.
    exchange: u32,
    after: u32,
    current: Combo,
    previous: Combo,
    last_link: Option<u32>,
    frame: u32,
}

/// The helper: the capture, the target and the attempts. `Copy`, so the HUD can take a snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Helper {
    capture: Capture,
    target: Option<Combo>,
    attempt: Option<Attempt>,
    results: [Option<Summary>; RESULTS],
    streak: u16,
    best: u8,
    pending_load: bool,
    settings: Settings,
}

impl Default for Helper {
    fn default() -> Self {
        Self::new()
    }
}

impl Helper {
    pub const fn new() -> Self {
        Self {
            capture: Capture {
                exchange: 0,
                after: 0,
                current: Combo::EMPTY,
                previous: Combo::EMPTY,
                last_link: None,
                frame: 0,
            },
            target: None,
            attempt: None,
            results: [None; RESULTS],
            streak: 0,
            best: 0,
            pending_load: false,
            settings: Settings::factory(),
        }
    }

    /// A new target (or none): the attempt and the history start over.
    pub fn set_target(&mut self, target: Option<Combo>) {
        self.target = target;
        self.attempt = None;
        self.results = [None; RESULTS];
        self.streak = 0;
        self.best = 0;
        self.pending_load = false;
    }

    pub fn target(&self) -> Option<&Combo> {
        self.target.as_ref()
    }

    /// *Save last combo*: the open exchange when it has a link, else the one before.
    pub fn last_combo(&self) -> Option<Combo> {
        [self.capture.current, self.capture.previous].into_iter().find(|c| !c.is_empty())
    }

    pub fn attempt(&self) -> Option<&Attempt> {
        self.attempt.as_ref()
    }

    /// Results, newest first.
    pub fn results(&self) -> impl Iterator<Item = &Summary> {
        self.results.iter().flatten()
    }

    pub fn streak(&self) -> u16 {
        self.streak
    }

    pub fn best(&self) -> u8 {
        self.best
    }

    pub fn settings(&self) -> Settings {
        self.settings
    }

    /// The strip's session clear (a scene change, a hot swap): an open attempt is cleared, and the capture starts over
    /// (the last combo is kept for *Save last combo*).
    pub fn clear(&mut self) -> Out {
        let mut out = Out::default();
        self.on_clear(&mut out);
        out
    }

    /// One frame, after the strip's tracker stepped.
    pub fn step(&mut self, tracker: &Tracker, settings: Settings) -> Out {
        let mut out = Out::default();
        self.settings = settings;
        if tracker.frame() < self.capture.frame {
            self.on_clear(&mut out);
        }
        self.capture.frame = tracker.frame();
        let (exchange, first_hit) = tracker.exchange();
        if exchange != self.capture.exchange {
            self.resolve_open(Outcome::Stopped, &mut out);
            if !self.capture.current.is_empty() {
                self.capture.previous = self.capture.current;
            }
            self.capture.current = Combo::EMPTY;
            self.capture.exchange = exchange;
            self.capture.after = first_hit;
        }
        if let Some(link) = tracker.latest_link().filter(|l| Some(l.hit) != self.capture.last_link) {
            self.capture.last_link = Some(link.hit);
            let after = self.capture.after;
            // `presses()` is newest first: the follow-up's are after the last hit, up to this one; kept oldest first.
            let mut presses = [None; PRESSES];
            for (slot, press) in presses.iter_mut().zip(tracker.presses().filter(|p| p.frame > after && p.frame <= link.hit)) {
                *slot = Some(press);
            }
            let rec = LinkRec::of(&link, presses.iter().rev().flatten().copied());
            self.capture.after = link.hit;
            let first = self.capture.current.is_empty();
            self.capture.current.push(rec);
            self.on_link(rec, first, exchange, &mut out);
        }
        if !tracker.exchange_open() {
            self.resolve_open(Outcome::Stopped, &mut out);
            if self.pending_load {
                self.pending_load = false;
                out.load = true;
            }
        }
        out
    }

    fn on_clear(&mut self, out: &mut Out) {
        self.resolve_open(Outcome::Cleared, out);
        self.pending_load = false;
        if !self.capture.current.is_empty() {
            self.capture.previous = self.capture.current;
        }
        self.capture = Capture { previous: self.capture.previous, ..Helper::new().capture };
    }

    fn on_link(&mut self, rec: LinkRec, first: bool, exchange: u32, out: &mut Out) {
        let Some(target) = self.target.filter(|_| self.settings.on) else { return };
        let open = self.attempt.as_ref().is_some_and(|a| a.open() && a.exchange == exchange);
        if !open {
            if !first || target.get(0).is_none_or(|r| r.from != rec.from) {
                return;
            }
            self.attempt = Some(Attempt { exchange, grades: [None; LINKS], len: 0, summary: None });
        }
        let Some(attempt) = self.attempt.as_mut() else { return };
        let Some(reference) = target.get(attempt.len) else { return };
        let g = grade(reference, &rec);
        attempt.grades[attempt.len] = Some(g);
        out.link = Some((attempt.len, g));
        attempt.len += 1;
        if attempt.len == target.len() {
            self.resolve_open(Outcome::Completed, out);
        }
    }

    /// Ends the open attempt with `outcome`, records it and asks for the load after it when set.
    fn resolve_open(&mut self, outcome: Outcome, out: &mut Out) {
        let tolerance = self.settings.tolerance;
        let Some(total) = self.target.map(|t| t.len()) else { return };
        let Some(attempt) = self.attempt.as_mut().filter(|a| a.open()) else { return };
        let summary = summarize(attempt, total, outcome, tolerance);
        attempt.summary = Some(summary);
        self.results.copy_within(0..RESULTS - 1, 1);
        self.results[0] = Some(summary);
        self.streak = if summary.clean { self.streak.saturating_add(1) } else { 0 };
        self.best = self.best.max(summary.true_links);
        if outcome != Outcome::Cleared && self.settings.load_after {
            self.pending_load = true;
        }
        out.resolved = Some(summary);
    }
}

fn summarize(attempt: &Attempt, total: usize, outcome: Outcome, tolerance: u8) -> Summary {
    let grades = || attempt.grades().enumerate();
    let not_true = grades().find(|(_, g)| !g.is_true()).map(|(i, g)| {
        (i as u8, match g.link.verdict {
            Verdict::Drop(n) if !g.wrong_move => Reason::Drop(n),
            _ => Reason::WrongMove(g.link.to),
        })
    });
    let button = |g: &Grade, step: usize| g.reference.steps[step].map(|s| s.button);
    let missing = || {
        grades().find_map(|(i, g)| {
            let step = g.missing()?;
            Some((i as u8, Reason::Missing(button(g, step)?)))
        })
    };
    let off = || {
        grades().find_map(|(i, g)| {
            let (step, d) = g.worst(tolerance)?;
            Some((i as u8, Reason::Off(button(g, step)?, d)))
        })
    };
    let true_links = grades().filter(|(_, g)| g.is_true()).count() as u8;
    let clean = outcome == Outcome::Completed && grades().all(|(_, g)| g.clean(tolerance));
    Summary {
        outcome,
        graded: attempt.len() as u8,
        true_links,
        total: total as u8,
        clean,
        fault: not_true.or_else(missing).or_else(off),
    }
}

#[cfg(test)]
#[path = "combo_tests.rs"]
mod tests;
