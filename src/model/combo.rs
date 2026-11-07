//! The combo helper (SPEC-input-07; operator decision COMBO-HELPER, option A, 2026-11-05): the exchanges the frame
//! strip links, kept as combos; a saved reference; and each attempt graded link by link against it. Pure and
//! allocation-free: it steps once per frame right after the strip's tracker (`model::frame_strip`), on fixed arrays.

use super::frame_strip::{
    bin_stick, stick_axis, Button, Link, Move, Snap, Tracker, Verdict, CSTICK_ON, DASH_FLICK_X,
    DASH_STICK_X, DIVE_CONT, DIVE_FLICK, FLICK_JUMP, JUMP_FLICK_Y, JUMP_STICK_Y, PRESS_BITS,
};

/// A name the menu shows (the target, the draft): `Copy`, 48 characters at most, cut at a character boundary.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Label {
    bytes: [u8; 192],
    len: u8,
}

impl Label {
    pub const EMPTY: Label = Label {
        bytes: [0; 192],
        len: 0,
    };

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

/// Links of one combo, events kept per link, results in the history.
pub const LINKS: usize = 16;
/// Input events kept on one link. A 13th is dropped; the first 12 in time order stay.
pub const STEPS: usize = 12;
/// Presses an older file stores, and the press subset a new file still writes for that reader.
pub const LEGACY_STEPS: usize = 4;
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

/// A stick flick the game treats as its own input (revision 8.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flick {
    FastFall,
    Dash,
    TapJump,
}

/// What one recorded input is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Press(Button),
    Release(Button),
    CStick,
    Flick(Flick),
}

/// Facing-relative direction. `Side` is only a presses-only file, where left and right are one aim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Neutral,
    Forward,
    ForwardUp,
    Up,
    BackUp,
    Back,
    BackDown,
    Down,
    ForwardDown,
    Side,
}

impl Dir {
    pub const fn from_aim(aim: Aim) -> Dir {
        match aim {
            Aim::Neutral => Dir::Neutral,
            Aim::Up => Dir::Up,
            Aim::Down => Dir::Down,
            Aim::Side => Dir::Side,
        }
    }

    /// The aim an older file would have stored for this direction.
    pub const fn legacy_aim(self) -> Aim {
        match self {
            Dir::Neutral => Aim::Neutral,
            Dir::Up | Dir::ForwardUp | Dir::BackUp => Aim::Up,
            Dir::Down | Dir::ForwardDown | Dir::BackDown => Aim::Down,
            Dir::Side | Dir::Forward | Dir::Back => Aim::Side,
        }
    }
}

/// One input event of a link's follow-up, relative to the link's A₁ (negative when buffered).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub kind: Kind,
    pub dir: Dir,
    pub offset: i32,
}

impl Event {
    /// A press as an older file records it: the aim, with left and right already merged.
    pub const fn press(button: Button, aim: Aim, offset: i32) -> Self {
        Event {
            kind: Kind::Press(button),
            dir: Dir::from_aim(aim),
            offset,
        }
    }
}

/// The first `STEPS` events, in the order given. A later one is dropped.
pub fn retain(events: impl IntoIterator<Item = Event>) -> [Option<Event>; STEPS] {
    let mut out = [None; STEPS];
    for (slot, event) in out.iter_mut().zip(events) {
        *slot = Some(event);
    }
    out
}

/// One link of a combo: the strip's `Link` reduced to what is compared, plus the follow-up's input events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkRec {
    pub from: Move,
    pub to: Move,
    pub verdict: Verdict,
    /// The combo window relative to A₁.
    pub window: Option<(i32, i32)>,
    pub events: [Option<Event>; STEPS],
    /// Loaded from `steps` with no `events`. Graded on presses, left and right merged.
    pub presses_only: bool,
    /// Held direction between the events. Shown, never graded.
    pub drift: Option<Dir>,
}

impl LinkRec {
    /// The follow-up's events from the tracker's snaps in `(after, hit]`. The first 12 are kept.
    pub fn of(link: &Link, tracker: &Tracker, after: u32) -> LinkRec {
        let a1 = link.actionable as i32;
        let (events, drift) = events_between(tracker, after, link.hit, a1);
        LinkRec {
            from: link.from,
            to: link.to,
            verdict: link.verdict,
            window: link
                .window
                .map(|(start, end)| (start as i32 - a1, end as i32 - a1)),
            events,
            presses_only: false,
            drift,
        }
    }

    /// A presses-only link, the shape an old file grades as.
    pub fn presses(
        from: Move,
        to: Move,
        verdict: Verdict,
        window: Option<(i32, i32)>,
        presses: &[Event],
    ) -> Self {
        LinkRec {
            from,
            to,
            verdict,
            window,
            events: retain(presses.iter().copied()),
            presses_only: true,
            drift: None,
        }
    }

    pub fn events(&self) -> impl Iterator<Item = Event> + '_ {
        self.events.iter().flatten().copied()
    }

    /// Presses an older reader can store, capped at [`LEGACY_STEPS`].
    pub fn legacy_presses(&self) -> impl Iterator<Item = Event> + '_ {
        self.events()
            .filter(|event| matches!(event.kind, Kind::Press(_)))
            .take(LEGACY_STEPS)
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
    pub const EMPTY: Combo = Combo {
        links: [None; LINKS],
        len: 0,
    };

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

/// A reference event's match in an attempt: the attempt's offset minus the reference's, or no such event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    At(i32),
    Missing,
    /// The same kind of event, in order, with a different direction.
    WrongDir,
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

    pub fn wrong_dir(&self) -> Option<usize> {
        self.marks.iter().position(|m| *m == Some(Mark::WrongDir))
    }

    /// The step with the largest offset outside the tolerance, with that offset.
    pub fn worst(&self, tolerance: u8) -> Option<(usize, i32)> {
        let mut worst: Option<(usize, i32)> = None;
        for (i, mark) in self.marks.iter().enumerate() {
            if let Some(Mark::At(d)) = mark {
                if d.unsigned_abs() > u32::from(tolerance)
                    && worst.is_none_or(|(_, w)| d.unsigned_abs() > w.unsigned_abs())
                {
                    worst = Some((i, *d));
                }
            }
        }
        worst
    }

    /// The recorded input events are there, with the same direction, in order and within the tolerance, and the
    /// move that came out is the recorded move. TRUE is not required (operator 2026-11-06, answer 1). Drift is
    /// not read.
    pub fn clean(&self, tolerance: u8) -> bool {
        !self.wrong_move
            && self.missing().is_none()
            && self.wrong_dir().is_none()
            && self.worst(tolerance).is_none()
    }
}

/// Grade `attempt` against `reference`. A presses-only reference matches the next unused press with the same button
/// and the old aim. Any other reference matches the next event of the same kind; a different direction is
/// [`Mark::WrongDir`] and is not skipped.
pub fn grade(reference: &LinkRec, attempt: &LinkRec) -> Grade {
    let mut used = [false; STEPS];
    let mut marks = [None; STEPS];
    let mut from = 0;
    for (mark, want) in marks.iter_mut().zip(reference.events.iter()) {
        let Some(want) = want else { continue };
        let found = (from..STEPS).find(|&i| {
            !used[i]
                && attempt.events[i]
                    .is_some_and(|got| event_matches(*want, got, reference.presses_only))
        });
        *mark = Some(match found {
            Some(i) => {
                used[i] = true;
                from = i + 1;
                let got = attempt.events[i].unwrap();
                if reference.presses_only || got.dir == want.dir {
                    Mark::At(got.offset - want.offset)
                } else {
                    Mark::WrongDir
                }
            }
            None => Mark::Missing,
        });
    }
    let extra = (0..STEPS)
        .filter(|&i| attempt.events[i].is_some() && !used[i])
        .count() as u8;
    Grade {
        link: *attempt,
        reference: *reference,
        wrong_move: attempt.to != reference.to,
        marks,
        extra,
    }
}

fn event_matches(want: Event, got: Event, presses_only: bool) -> bool {
    if presses_only {
        return matches!((want.kind, got.kind), (Kind::Press(a), Kind::Press(b)) if a == b)
            && want.dir.legacy_aim() == got.dir.legacy_aim();
    }
    got.kind == want.kind
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

/// Why an attempt was not clean: a different move, else the first missing event, else a wrong direction, else the
/// first event outside the tolerance. A link that is not TRUE is not a fault by itself (operator 2026-11-06,
/// answer 1). `Drop` stays so the bar can still name a verdict the summary no longer faults. The press variants
/// keep the sentences the bar already shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Drop(i32),
    WrongMove(Move),
    Missing(Button),
    Off(Button, i32),
    MissingRelease(Button),
    ReleaseOff(Button, i32),
    MissingFlick(Flick),
    FlickOff(Flick, i32),
    MissingCStick,
    CStickOff(i32),
    WrongDir(Kind),
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
        Self {
            on: false,
            tolerance: 1,
            load_after: false,
        }
    }

    pub fn from_snapshot(snapshot: &super::settings::Snapshot) -> Self {
        let choice = |id: &str| {
            snapshot
                .get(id)
                .and_then(|r| r.iter().position(|v| *v != 0))
        };
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
        [self.capture.current, self.capture.previous]
            .into_iter()
            .find(|c| !c.is_empty())
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
        if let Some(link) = tracker
            .latest_link()
            .filter(|l| Some(l.hit) != self.capture.last_link)
        {
            self.capture.last_link = Some(link.hit);
            let after = self.capture.after;
            let rec = LinkRec::of(&link, tracker, after);
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
        self.capture = Capture {
            previous: self.capture.previous,
            ..Helper::new().capture
        };
    }

    fn on_link(&mut self, rec: LinkRec, first: bool, exchange: u32, out: &mut Out) {
        let Some(target) = self.target.filter(|_| self.settings.on) else {
            return;
        };
        let open = self
            .attempt
            .as_ref()
            .is_some_and(|a| a.open() && a.exchange == exchange);
        if !open {
            if !first || target.get(0).is_none_or(|r| r.from != rec.from) {
                return;
            }
            self.attempt = Some(Attempt {
                exchange,
                grades: [None; LINKS],
                len: 0,
                summary: None,
            });
        }
        let Some(attempt) = self.attempt.as_mut() else {
            return;
        };
        let Some(reference) = target.get(attempt.len) else {
            return;
        };
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
        let Some(total) = self.target.map(|t| t.len()) else {
            return;
        };
        let Some(attempt) = self.attempt.as_mut().filter(|a| a.open()) else {
            return;
        };
        let summary = summarize(attempt, total, outcome, tolerance);
        attempt.summary = Some(summary);
        self.results.copy_within(0..RESULTS - 1, 1);
        self.results[0] = Some(summary);
        // Streak follows `clean`, so a non-true link still counts. Best stays the most true links in one
        // attempt (operator 2026-11-06, question 2: a single combo's best stays as today).
        self.streak = if summary.clean {
            self.streak.saturating_add(1)
        } else {
            0
        };
        self.best = self.best.max(summary.true_links);
        if outcome != Outcome::Cleared && self.settings.load_after {
            self.pending_load = true;
        }
        out.resolved = Some(summary);
    }
}

fn missing_reason(event: Event) -> Reason {
    match event.kind {
        Kind::Press(button) => Reason::Missing(button),
        Kind::Release(button) => Reason::MissingRelease(button),
        Kind::CStick => Reason::MissingCStick,
        Kind::Flick(flick) => Reason::MissingFlick(flick),
    }
}

fn off_reason(event: Event, delta: i32) -> Reason {
    match event.kind {
        Kind::Press(button) => Reason::Off(button, delta),
        Kind::Release(button) => Reason::ReleaseOff(button, delta),
        Kind::CStick => Reason::CStickOff(delta),
        Kind::Flick(flick) => Reason::FlickOff(flick, delta),
    }
}

fn summarize(attempt: &Attempt, total: usize, outcome: Outcome, tolerance: u8) -> Summary {
    let grades = || attempt.grades().enumerate();
    let event = |g: &Grade, step: usize| g.reference.events[step];
    let wrong_move = grades()
        .find(|(_, g)| g.wrong_move)
        .map(|(i, g)| (i as u8, Reason::WrongMove(g.link.to)));
    let missing =
        || grades().find_map(|(i, g)| Some((i as u8, missing_reason(event(g, g.missing()?)?))));
    let wrong_dir = || {
        grades()
            .find_map(|(i, g)| Some((i as u8, Reason::WrongDir(event(g, g.wrong_dir()?)?.kind))))
    };
    let off = || {
        grades().find_map(|(i, g)| {
            let (step, delta) = g.worst(tolerance)?;
            Some((i as u8, off_reason(event(g, step)?, delta)))
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
        fault: wrong_move.or_else(missing).or_else(wrong_dir).or_else(off),
    }
}

/// Events in `(after, until]`, offsets relative to `a1`, and the drift on the frames that are not an event.
fn events_between(
    tracker: &Tracker,
    after: u32,
    until: u32,
    a1: i32,
) -> ([Option<Event>; STEPS], Option<Dir>) {
    let mut prev: Option<Snap> = None;
    let mut timed = [None; STEPS];
    let mut count = 0;
    let mut event_frames = [0u32; STEPS];
    let mut event_frame_count = 0usize;
    let mut drift_counts = [0u8; 8];
    let mut drift_order = [0u8; 8];
    let mut drift_seen = 0usize;
    for snap in tracker.snaps() {
        if snap.frame > until {
            break;
        }
        let in_range = snap.frame > after && snap.frame <= until;
        if in_range {
            if push_edges(&mut timed, &mut count, prev, snap) && event_frame_count < STEPS {
                event_frames[event_frame_count] = snap.frame;
                event_frame_count += 1;
            }
            let is_event = event_frames[..event_frame_count].contains(&snap.frame);
            if !is_event {
                if let Some(index) = drift_index(facing_dir(
                    bin_stick(snap.lstick_x, snap.lstick_y),
                    snap.facing,
                )) {
                    note_drift(&mut drift_counts, &mut drift_order, &mut drift_seen, index);
                }
            }
        }
        prev = Some(snap);
    }
    let mut events = [None; STEPS];
    for (slot, timed) in events.iter_mut().zip(timed.iter().flatten()) {
        *slot = Some(Event {
            kind: timed.kind,
            dir: timed.dir,
            offset: timed.frame as i32 - a1,
        });
    }
    (events, modal_drift(&drift_counts, &drift_order, drift_seen))
}

#[derive(Clone, Copy)]
struct Timed {
    kind: Kind,
    dir: Dir,
    frame: u32,
}

fn push_edges(
    out: &mut [Option<Timed>; STEPS],
    count: &mut usize,
    prev: Option<Snap>,
    now: Snap,
) -> bool {
    let mut saw = false;
    let prev_buttons = prev.map(|snap| snap.buttons).unwrap_or(0);
    let prev_x = prev.map(|snap| stick_axis(snap.lstick_x)).unwrap_or(0.0);
    let prev_y = prev.map(|snap| stick_axis(snap.lstick_y)).unwrap_or(0.0);
    let now_x = stick_axis(now.lstick_x);
    let now_y = stick_axis(now.lstick_y);
    let left = facing_dir(bin_stick(now.lstick_x, now.lstick_y), now.facing);
    for (mask, button) in PRESS_BITS {
        if held(prev_buttons, mask) && !held(now.buttons, mask) {
            saw = true;
            push_timed(out, count, Kind::Release(button), left, now.frame);
        }
    }
    if prev_x.abs() < DASH_STICK_X
        && now_x.abs() >= DASH_STICK_X
        && super::frame_strip::flick_fresh(0, DASH_FLICK_X)
    {
        saw = true;
        push_timed(out, count, Kind::Flick(Flick::Dash), left, now.frame);
    }
    if !held(prev_buttons, FLICK_JUMP)
        && held(now.buttons, FLICK_JUMP)
        && now_y >= JUMP_STICK_Y
        && super::frame_strip::flick_fresh(0, JUMP_FLICK_Y)
    {
        saw = true;
        push_timed(out, count, Kind::Flick(Flick::TapJump), left, now.frame);
    }
    if prev_y > DIVE_CONT
        && now_y <= DIVE_CONT
        && now.airborne
        && super::frame_strip::flick_fresh(0, DIVE_FLICK)
    {
        saw = true;
        push_timed(out, count, Kind::Flick(Flick::FastFall), left, now.frame);
    }
    if !held(prev_buttons, CSTICK_ON) && held(now.buttons, CSTICK_ON) {
        saw = true;
        let dir = facing_dir(bin_stick(now.rstick_x, now.rstick_y), now.facing);
        push_timed(out, count, Kind::CStick, dir, now.frame);
    }
    for (mask, button) in PRESS_BITS {
        if !held(prev_buttons, mask) && held(now.buttons, mask) {
            saw = true;
            push_timed(out, count, Kind::Press(button), left, now.frame);
        }
    }
    saw
}

fn held(buttons: u32, mask: u32) -> bool {
    buttons & mask != 0
}

fn push_timed(
    out: &mut [Option<Timed>; STEPS],
    count: &mut usize,
    kind: Kind,
    dir: Dir,
    frame: u32,
) {
    if *count < STEPS {
        out[*count] = Some(Timed { kind, dir, frame });
        *count += 1;
    }
}

/// Stick bin 0 is right, counter-clockwise. Facing left mirrors it. Neutral when the stick does not bin.
pub fn facing_dir(bin: Option<u8>, facing: i8) -> Dir {
    let Some(bin) = bin else { return Dir::Neutral };
    let bin = if facing < 0 {
        match bin {
            0 => 4,
            1 => 3,
            3 => 1,
            4 => 0,
            5 => 7,
            7 => 5,
            other => other,
        }
    } else {
        bin
    };
    match bin {
        0 => Dir::Forward,
        1 => Dir::ForwardUp,
        2 => Dir::Up,
        3 => Dir::BackUp,
        4 => Dir::Back,
        5 => Dir::BackDown,
        6 => Dir::Down,
        _ => Dir::ForwardDown,
    }
}

fn drift_index(dir: Dir) -> Option<usize> {
    Some(match dir {
        Dir::Forward => 0,
        Dir::ForwardUp => 1,
        Dir::Up => 2,
        Dir::BackUp => 3,
        Dir::Back => 4,
        Dir::BackDown => 5,
        Dir::Down => 6,
        Dir::ForwardDown => 7,
        Dir::Neutral | Dir::Side => return None,
    })
}

fn note_drift(counts: &mut [u8; 8], order: &mut [u8; 8], seen: &mut usize, index: usize) {
    if counts[index] == 0 && *seen < order.len() {
        order[*seen] = index as u8;
        *seen += 1;
    }
    counts[index] = counts[index].saturating_add(1);
}

fn modal_drift(counts: &[u8; 8], order: &[u8; 8], seen: usize) -> Option<Dir> {
    let dirs = [
        Dir::Forward,
        Dir::ForwardUp,
        Dir::Up,
        Dir::BackUp,
        Dir::Back,
        Dir::BackDown,
        Dir::Down,
        Dir::ForwardDown,
    ];
    let mut best: Option<Dir> = None;
    let mut best_count = 0u8;
    for index in order.iter().take(seen).map(|index| usize::from(*index)) {
        if counts[index] > best_count {
            best_count = counts[index];
            best = Some(dirs[index]);
        }
    }
    best
}

#[cfg(test)]
#[path = "combo_tests.rs"]
mod tests;
