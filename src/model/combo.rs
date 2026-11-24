//! The combo helper (SPEC-input-07; operator decision COMBO-HELPER, option A, 2026-11-05): the exchanges the frame
//! strip links, kept as combos; a saved reference; and each attempt graded link by link against it. Pure and
//! allocation-free: it steps once per frame right after the strip's tracker (`model::frame_strip`), on fixed arrays.

use super::frame_strip::{
    bin_stick, stick_axis, Button, Link, Move, Snap, Tracker, Verdict, CSTICK_ON, DASH_FLICK_X,
    DASH_STICK_X, DIVE_CONT, DIVE_FLICK, FLICK_JUMP, JUMP_FLICK_Y, JUMP_STICK_Y, PRESS_BITS,
};
use super::spots::Spot;

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
    #[cfg(test)]
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
    /// A part of a directional special took its direction; the part number.
    Aim(u8),
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
#[cfg(test)]
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
    /// The spot the follow-up hit landed (rc.14, SP4); None for an older file or a hit whose spot could not be told.
    pub spot: Option<Spot>,
}

impl LinkRec {
    pub const INPUT: Self = Self {
        from: Move::Other,
        to: Move::Other,
        verdict: Verdict::Drop(0),
        window: None,
        events: [None; STEPS],
        presses_only: false,
        drift: None,
        spot: None,
    };
    /// The follow-up's events from the tracker's snaps in `(after, hit]`. The first 12 are kept.
    pub fn of(link: &Link, tracker: &Tracker, after: u32, aims: bool) -> LinkRec {
        let a1 = link.actionable as i32;
        let (events, drift) = events_between(tracker, after, link.hit, a1, aims);
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
            spot: link.spot,
        }
    }

    /// A presses-only link, the shape an old file grades as.
    #[cfg(test)]
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
            spot: None,
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
    pub input_only: bool,
    pub overflow: bool,
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
        input_only: false,
        overflow: false,
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
            self.overflow = true;
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

    pub fn has_aim(&self) -> bool {
        self.links().any(|link| link.events().any(|event| matches!(event.kind, Kind::Aim(_))))
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
    pub input_only: bool,
    /// The attempt's own link, and the reference link it was graded against.
    pub link: LinkRec,
    pub reference: LinkRec,
    pub wrong_move: bool,
    /// Per reference step, in order.
    pub marks: [Option<Mark>; STEPS],
    /// The attempt's steps no reference step matched.
    pub extra: u8,
    pub extra_aim: Option<u8>,
    /// The reference link saved a spot and the attempt did not land that one (decision SP4; read only with Match sweet/sour spot On).
    pub wrong_spot: bool,
}

impl Grade {
    /// TRUE and the right move (SPEC-input-07 §3).
    pub fn is_true(&self) -> bool {
        !self.input_only && self.link.is_true() && !self.wrong_move
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
            && self.extra_aim.is_none()
            && self.worst(tolerance).is_none()
    }
}

/// Grade `attempt` against `reference`. A presses-only reference matches the next unused press with the same button
/// and the old aim. Any other reference matches the next event of the same kind; a different direction is
/// [`Mark::WrongDir`] and is not skipped. A reference release the attempt has no match for is not graded (rc.12).
pub fn grade(reference: &LinkRec, attempt: &LinkRec) -> Grade {
    grade_mode(reference, attempt, false, reference.events().any(|event| matches!(event.kind, Kind::Aim(_))))
}

fn grade_mode(reference: &LinkRec, attempt: &LinkRec, input_only: bool, aims: bool) -> Grade {
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
        // rc.12: the follow-up's events end at its hit, so a button still held when the hit lands has no release here.
        // Holding it longer does not change the move that came out: a release with no counterpart is not graded.
        if !input_only && found.is_none() && matches!(want.kind, Kind::Release(_)) {
            continue;
        }
        *mark = Some(match found {
            Some(i) => {
                used[i] = true;
                from = i + 1;
                let got = attempt.events[i].unwrap();
                if reference.presses_only || got.dir == want.dir {
                    Mark::At(if aims && matches!(want.kind, Kind::Aim(_)) { 0 } else { got.offset - want.offset })
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
        input_only,
        link: *attempt,
        reference: *reference,
        wrong_move: !input_only && attempt.to != reference.to,
        marks,
        extra,
        extra_aim: aims.then(|| attempt.events.iter().enumerate().find_map(|(i, event)| {
            match event {
                Some(Event { kind: Kind::Aim(part), .. }) if !used[i] => Some(*part),
                _ => None,
            }
        })).flatten(),
        wrong_spot: (!input_only)
            .then_some(reference.spot)
            .flatten()
            .is_some_and(|saved| attempt.spot != Some(saved)),
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
    /// The spot the attempt landed (None: none read) and the one the saved link landed.
    WrongSpot(Option<Spot>, Spot),
    Missing(Button),
    Off(Button, i32),
    MissingRelease(Button),
    ReleaseOff(Button, i32),
    MissingFlick(Flick),
    FlickOff(Flick, i32),
    MissingCStick,
    CStickOff(i32),
    WrongDir(Kind),
    MissingAim(u8),
    WrongAim(u8),
    ExtraAim(u8),
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
    /// A later combo of a sequence: the link from the combo before it is still to come. That link is shown on the
    /// strip and not graded, and the move it lands must be this combo's first move.
    junction: bool,
}

impl Attempt {
    fn new(exchange: u32, junction: bool) -> Self {
        Self { exchange, grades: [None; LINKS], len: 0, summary: None, junction }
    }

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
    /// *Match sweet/sour spot* (decision SP4, `spots::ROW_COMBO`): a link counts only when it lands the saved link's spot.
    pub match_spot: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self::factory()
    }
}

impl Settings {
    pub const fn factory() -> Self {
        Self {
            // Operator 2026-11-07: a fresh profile grades a drill without opening Options.
            on: true,
            tolerance: 1,
            load_after: false,
            match_spot: false,
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
            match_spot: super::spots::row_on(snapshot, super::spots::ROW_COMBO),
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
    held: Option<Link>,
    graded: Option<u32>,
    frame: u32,
}

/// The helper: game-owned capture, target and attempts. The HUD borrows its static storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Helper {
    capture: Capture,
    /// Links captured so far (wrapping; a clear keeps it): Make combo saves only an exchange captured after it began.
    seen: u32,
    target: Option<Combo>,
    attempt: Option<Attempt>,
    results: [Option<Summary>; RESULTS],
    streak: u16,
    best: u8,
    pending_load: bool,
    settings: Settings,
    input: InputCapture,
}

impl Helper {
    pub(crate) const fn new() -> Self {
        Self {
            capture: Capture {
                exchange: 0,
                after: 0,
                current: Combo::EMPTY,
                previous: Combo::EMPTY,
                last_link: None,
                held: None,
                graded: None,
                frame: 0,
            },
            seen: 0,
            target: None,
            attempt: None,
            results: [None; RESULTS],
            streak: 0,
            best: 0,
            pending_load: false,
            settings: Settings::factory(),
            input: InputCapture::new(),
        }
    }

    /// A new target (or none): the attempt and the history start over.
    pub fn set_target(&mut self, target: Option<Combo>) {
        self.target = target;
        self.input = InputCapture::new();
        self.attempt = None;
        self.results = [None; RESULTS];
        self.streak = 0;
        self.best = 0;
        self.pending_load = false;
    }

    /// The next combo of a sequence, inside the exchange the last one finished in. Its attempt is open now: the next
    /// link joins the two combos and is not graded, and the move it lands must be `target`'s first move. A different
    /// move, or the exchange closing first, ends the attempt not clean. A load asked for after an attempt still waits
    /// for the exchange to close.
    pub fn continue_with(&mut self, target: Combo) {
        self.target = Some(target);
        self.attempt = Some(Attempt::new(self.capture.exchange, true));
    }

    /// Back to the first combo of a sequence. Its attempt starts on the first link of a later exchange, as a single
    /// combo's does. A load asked for after an attempt still waits for the exchange to close.
    pub fn restart_with(&mut self, target: Option<Combo>) {
        self.target = target;
        self.attempt = None;
    }

    pub fn target(&self) -> Option<&Combo> {
        self.target.as_ref()
    }

    /// *Save last combo*: the open exchange when it has a link, else the one before.
    pub fn last_combo(&self) -> Option<Combo> {
        self.last_combo_ref().copied()
    }

    pub fn last_combo_ref(&self) -> Option<&Combo> {
        [&self.capture.current, &self.capture.previous].into_iter().find(|c| !c.is_empty())
    }

    /// Links captured so far (wrapping). A change means `last_combo` holds a link captured since the count was read.
    pub fn seen(&self) -> u32 {
        self.seen
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
        let input_only = self.target.as_ref().is_some_and(|t| t.input_only);
        let (exchange, first_hit) = tracker.exchange();
        if exchange != self.capture.exchange {
            if let Some(held) = self.capture.held.take() {
                self.take_link(held, tracker, self.capture.exchange, &mut out);
            }
            if !input_only {
                self.resolve_open(Outcome::Stopped, &mut out);
            }
            if !self.capture.current.is_empty() {
                self.capture.previous = self.capture.current;
            }
            self.capture.current = Combo::EMPTY;
            self.capture.exchange = exchange;
            self.capture.after = first_hit;
            self.capture.graded = None;
        }
        if let Some(link) = tracker.latest_link().filter(|l| Some(l.hit) != self.capture.last_link) {
            if let Some(held) = self.capture.held.filter(|held| held.hit != link.hit) {
                self.capture.held = None;
                self.take_link(held, tracker, exchange, &mut out);
            }
            let aims = self.target.as_ref().is_some_and(Combo::has_aim);
            if !input_only && !aims && self.capture.graded != Some(link.hit) {
                let rec = LinkRec::of(&link, tracker, self.capture.after, false);
                self.capture.graded = Some(link.hit);
                self.on_link(rec, self.capture.current.is_empty(), exchange, &mut out, None);
            }
            if tracker.aim_pending().is_some_and(|entry| entry <= link.hit) {
                self.capture.held = Some(link);
            } else {
                self.capture.held = None;
                self.take_link(link, tracker, exchange, &mut out);
            }
        }
        if input_only {
            if let Some(snap) = tracker.snaps().last() {
                self.step_inputs(snap, &mut out);
            }
            if self.pending_load {
                self.pending_load = false;
                out.load = true;
            }
            return out;
        }
        if !tracker.exchange_open() && self.capture.held.is_none() {
            self.resolve_open(Outcome::Stopped, &mut out);
            if self.pending_load {
                self.pending_load = false;
                out.load = true;
            }
        }
        out
    }

    fn take_link(&mut self, link: Link, tracker: &Tracker, exchange: u32, out: &mut Out) {
        self.capture.last_link = Some(link.hit);
        let rec = LinkRec::of(&link, tracker, self.capture.after, true);
        self.capture.after = link.hit;
        let first = self.capture.current.is_empty();
        self.capture.current.push(rec);
        self.seen = self.seen.wrapping_add(1);
        if self.target.as_ref().is_some_and(|target| !target.input_only && target.has_aim()) {
            self.on_link(rec, first, exchange, out, None);
        }
    }

    fn step_inputs(&mut self, snap: Snap, out: &mut Out) {
        if !self.settings.on {
            return;
        }
        if self.attempt.as_ref().is_some_and(|a| !a.open())
            && self
                .input
                .last
                .is_some_and(|last| snap.frame.saturating_sub(last) > super::frame_strip::DROP_MAX)
        {
            self.input.restart();
            self.attempt = None;
        }
        self.input.step_mode(snap, false, self.target.as_ref().is_some_and(Combo::has_aim));
        let Some(start) = self.input.first else {
            return;
        };
        if self.attempt.is_none() {
            self.attempt = Some(Attempt::new(start, false));
        }
        while let Some(attempt) = self.attempt.as_ref().filter(|a| a.open()) {
            let target = self.target.as_ref().unwrap();
            let aims = target.has_aim();
            let last = attempt.len + 1 == target.len();
            let reference = target.get(attempt.len).copied().unwrap();
            let deadline =
                reference.events().last().unwrap().offset + i32::from(self.settings.tolerance);
            if (snap.frame - start) < deadline as u32 || (aims && last && snap.aim.inside) {
                break;
            }
            let actual = self
                .input
                .combo
                .get(attempt.len)
                .copied()
                .unwrap_or(LinkRec::INPUT);
            let later = if aims && last {
                self.input.combo.links().skip(attempt.len + 1).flat_map(LinkRec::events)
                    .find_map(|event| match event.kind { Kind::Aim(part) => Some(part), _ => None })
            } else {
                None
            };
            self.on_link(actual, false, start, out, later);
            if self
                .attempt
                .is_some_and(|a| a.open() && a.len == self.input.combo.len())
            {
                self.input.combo.push(LinkRec::INPUT);
            }
        }
    }

    fn on_clear(&mut self, out: &mut Out) {
        self.resolve_open(Outcome::Cleared, out);
        self.pending_load = false;
        self.input = InputCapture::new();
        if !self.capture.current.is_empty() {
            self.capture.previous = self.capture.current;
        }
        self.capture = Capture {
            previous: self.capture.previous,
            ..Helper::new().capture
        };
    }

    fn on_link(&mut self, rec: LinkRec, first: bool, exchange: u32, out: &mut Out, extra: Option<u8>) {
        let Some(target) = self.target.as_ref().filter(|_| self.settings.on) else {
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
            self.attempt = Some(Attempt::new(exchange, false));
        }
        let Some(attempt) = self.attempt.as_mut() else {
            return;
        };
        if attempt.junction {
            attempt.junction = false;
            let first = target.get(0).map(|r| r.from);
            if first != Some(rec.to) {
                let summary = Summary {
                    outcome: Outcome::Stopped,
                    graded: 0,
                    true_links: 0,
                    total: target.len() as u8,
                    clean: false,
                    fault: Some((0, Reason::WrongMove(rec.to))),
                };
                self.record(summary, out);
            }
            return;
        }
        let Some(reference) = target.get(attempt.len) else {
            return;
        };
        let mut g = grade_mode(reference, &rec, target.input_only, target.has_aim());
        g.extra_aim = g.extra_aim.or(extra);
        attempt.grades[attempt.len] = Some(g);
        out.link = Some((attempt.len, g));
        attempt.len += 1;
        if attempt.len == target.len() || (target.input_only && !g.clean(self.settings.tolerance)) {
            self.resolve_open(Outcome::Completed, out);
        }
    }

    /// Ends the open attempt with `outcome`, records it and asks for the load after it when set.
    fn resolve_open(&mut self, outcome: Outcome, out: &mut Out) {
        let tolerance = self.settings.tolerance;
        let Some(total) = self.target.as_ref().map(|t| t.len()) else {
            return;
        };
        let Some(attempt) = self.attempt.as_ref().filter(|a| a.open()) else {
            return;
        };
        let summary = summarize(attempt, total, outcome, tolerance, self.settings.match_spot);
        self.record(summary, out);
    }

    /// Closes the open attempt with `summary`: the history, the streak, best and the load after it.
    fn record(&mut self, summary: Summary, out: &mut Out) {
        let outcome = summary.outcome;
        if let Some(attempt) = self.attempt.as_mut() {
            attempt.summary = Some(summary);
        }
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
        Kind::Aim(part) => Reason::MissingAim(part),
    }
}

fn off_reason(event: Event, delta: i32) -> Reason {
    match event.kind {
        Kind::Press(button) => Reason::Off(button, delta),
        Kind::Release(button) => Reason::ReleaseOff(button, delta),
        Kind::CStick => Reason::CStickOff(delta),
        Kind::Flick(flick) => Reason::FlickOff(flick, delta),
        Kind::Aim(part) => Reason::WrongAim(part),
    }
}

fn summarize(
    attempt: &Attempt,
    total: usize,
    outcome: Outcome,
    tolerance: u8,
    match_spot: bool,
) -> Summary {
    let grades = || attempt.grades().enumerate();
    let event = |g: &Grade, step: usize| g.reference.events[step];
    let wrong_move = grades()
        .find(|(_, g)| g.wrong_move)
        .map(|(i, g)| (i as u8, Reason::WrongMove(g.link.to)));
    let missing =
        || grades().find_map(|(i, g)| Some((i as u8, missing_reason(event(g, g.missing()?)?))));
    let wrong_dir = || {
        grades()
            .find_map(|(i, g)| {
                let kind = event(g, g.wrong_dir()?)?.kind;
                Some((i as u8, match kind {
                    Kind::Aim(part) => Reason::WrongAim(part),
                    _ => Reason::WrongDir(kind),
                }))
            })
    };
    let extra_aim = || grades().find_map(|(i, g)| g.extra_aim.map(|part| (i as u8, Reason::ExtraAim(part))));
    let off = || {
        grades().find_map(|(i, g)| {
            let (step, delta) = g.worst(tolerance)?;
            Some((i as u8, off_reason(event(g, step)?, delta)))
        })
    };
    let wrong_spot = || {
        grades().find_map(|(i, g)| {
            let saved = g.reference.spot.filter(|_| match_spot && g.wrong_spot)?;
            Some((i as u8, Reason::WrongSpot(g.link.spot, saved)))
        })
    };
    let true_links = grades()
        .filter(|(_, g)| {
            if g.input_only {
                g.clean(tolerance)
            } else {
                g.is_true()
            }
        })
        .count() as u8;
    let clean = outcome == Outcome::Completed
        && grades().all(|(_, g)| g.clean(tolerance) && !(match_spot && g.wrong_spot));
    Summary {
        outcome,
        graded: attempt.len() as u8,
        true_links,
        total: total as u8,
        clean,
        fault: wrong_move
            .or_else(wrong_spot)
            .or_else(missing)
            .or_else(wrong_dir)
            .or_else(extra_aim)
            .or_else(off),
    }
}

/// Events in `(after, until]`, offsets relative to `a1`, and the drift on the frames that are not an event.
fn events_between(
    tracker: &Tracker,
    after: u32,
    until: u32,
    a1: i32,
    aims: bool,
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
        if snap.frame > until && !aims {
            break;
        }
        let in_range = snap.frame > after && snap.frame <= until;
        if in_range {
            let mut physical = snap;
            physical.aim.part = 0;
            if push_edges(&mut timed, &mut count, prev, physical, aims) && event_frame_count < STEPS {
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
        if aims && snap.aim.part != 0 {
            let entry = snap.frame.saturating_sub(u32::from(snap.aim.age));
            if entry > after && entry <= until {
                insert_timed(&mut timed, &mut count, Timed {
                    kind: Kind::Aim(snap.aim.part),
                    dir: facing_dir(snap.aim.dir, snap.aim.facing),
                    frame: entry,
                });
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
    aims: bool,
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
    if !(aims && now.aim.inside)
        && prev_x.abs() < DASH_STICK_X
        && now_x.abs() >= DASH_STICK_X
        && super::frame_strip::flick_fresh(0, DASH_FLICK_X)
    {
        saw = true;
        push_timed(out, count, Kind::Flick(Flick::Dash), left, now.frame);
    }
    if !(aims && now.aim.inside)
        && !held(prev_buttons, FLICK_JUMP)
        && held(now.buttons, FLICK_JUMP)
        && now_y >= JUMP_STICK_Y
        && super::frame_strip::flick_fresh(0, JUMP_FLICK_Y)
    {
        saw = true;
        push_timed(out, count, Kind::Flick(Flick::TapJump), left, now.frame);
    }
    if !(aims && now.aim.inside)
        && prev_y > DIVE_CONT
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
    if aims && now.aim.part != 0 {
        push_timed(out, count, Kind::Aim(now.aim.part), facing_dir(now.aim.dir, now.aim.facing), now.frame);
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

fn insert_timed(out: &mut [Option<Timed>; STEPS], count: &mut usize, event: Timed) {
    let at = (0..*count).find(|&i| out[i].is_some_and(|other| other.frame > event.frame)).unwrap_or(*count);
    if at < STEPS {
        let end = (*count).min(STEPS - 1);
        for i in (at..end).rev() {
            out[i + 1] = out[i];
        }
        out[at] = Some(event);
        *count = (*count + 1).min(STEPS);
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
    DIRECTIONS[usize::from(bin.min(7))]
}

const DIRECTIONS: [Dir; 8] = [
    Dir::Forward, Dir::ForwardUp, Dir::Up, Dir::BackUp,
    Dir::Back, Dir::BackDown, Dir::Down, Dir::ForwardDown,
];

fn drift_index(dir: Dir) -> Option<usize> {
    DIRECTIONS.iter().position(|&candidate| candidate == dir)
}

fn note_drift(counts: &mut [u8; 8], order: &mut [u8; 8], seen: &mut usize, index: usize) {
    if counts[index] == 0 && *seen < order.len() {
        order[*seen] = index as u8;
        *seen += 1;
    }
    counts[index] = counts[index].saturating_add(1);
}

fn modal_drift(counts: &[u8; 8], order: &[u8; 8], seen: usize) -> Option<Dir> {
    let mut best: Option<Dir> = None;
    let mut best_count = 0u8;
    for index in order.iter().take(seen).map(|index| usize::from(*index)) {
        if counts[index] > best_count {
            best_count = counts[index];
            best = Some(DIRECTIONS[index]);
        }
    }
    best
}

#[cfg(test)]
#[path = "combo_tests.rs"]
mod tests;

/// Conversion accumulator shared by Record/Make and the input mode of the same attempt engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputCapture {
    pub combo: Combo,
    prev: Option<Snap>,
    first: Option<u32>,
    last: Option<u32>,
}
impl InputCapture {
    pub const fn new() -> Self {
        Self {
            combo: Combo {
                input_only: true,
                ..Combo::EMPTY
            },
            prev: None,
            first: None,
            last: None,
        }
    }
    fn restart(&mut self) {
        *self = Self {
            prev: self.prev,
            ..Self::new()
        };
    }
    pub fn step(&mut self, snap: Snap) {
        self.step_mode(snap, true, true);
    }
    fn step_mode(&mut self, mut snap: Snap, group_idle: bool, aims: bool) {
        if stick_axis(snap.lstick_y) < JUMP_STICK_Y {
            snap.buttons &= !FLICK_JUMP;
        }
        if self.combo.overflow { return; }
        let mut edges = [None; STEPS];
        let mut count = 0;
        push_edges(&mut edges, &mut count, self.prev, snap, aims);
        self.prev = Some(snap);
        for event in edges.into_iter().flatten() {
            let start = *self.first.get_or_insert(event.frame);
            let full = self
                .combo
                .links()
                .last()
                .is_none_or(|l| l.events().count() == STEPS);
            let next_part = matches!(event.kind, Kind::Aim(2..))
                && self.combo.links().last().is_some_and(|link| link.events().next().is_some());
            if (full
                || next_part
                || (group_idle
                    && self
                        .last
                        .is_some_and(|last| event.frame - last > super::frame_strip::DROP_MAX)))
                && !self.combo.push(LinkRec::INPUT)
            {
                return;
            }
            let link = self.combo.links[self.combo.len - 1].as_mut().unwrap();
            *link.events.iter_mut().find(|e| e.is_none()).unwrap() = Some(Event {
                kind: event.kind,
                dir: event.dir,
                offset: (event.frame - start) as i32,
            });
            self.last = Some(event.frame);
        }
    }
}
impl Default for InputCapture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "input_drill_tests.rs"]
mod input_drill_tests;

#[cfg(test)]
#[path = "aim_steps_tests.rs"]
mod aim_steps_tests;

#[cfg(test)]
#[path = "action_model_tests.rs"]
mod action_model_tests;
