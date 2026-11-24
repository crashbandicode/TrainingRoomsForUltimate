//! The combo helper (SPEC-input-07; operator decision COMBO-HELPER, option A, 2026-11-05): the exchanges the frame
//! strip links, kept as combos; a saved reference; and each attempt graded link by link against it. Pure and
//! allocation-free: it steps once per frame right after the strip's tracker (`model::frame_strip`), on fixed arrays.

use super::action::{self, ActionDesc, Late, Travel};
use super::frame_strip::{
    bin_stick, stick_axis, Angle, Button, Link, Move, Snap, Tracker, Verdict, CSTICK_ON, DASH_FLICK_X,
    DASH_STICK_X, DIVE_CONT, DIVE_FLICK, FLICK_JUMP, INPUT_LOOKBACK, JUMP_FLICK_Y, JUMP_STICK_Y, PRESS_BITS,
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
pub const DETAILS: usize = 4;

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
    pub(crate) shown: [u8; 3],
}

const DESC_MOVES: [Move; 17] = [
    Move::Jab, Move::DashAttack, Move::SideTilt, Move::UpTilt, Move::DownTilt,
    Move::SideSmash, Move::UpSmash, Move::DownSmash, Move::Aerial, Move::NeutralSpecial,
    Move::SideSpecial, Move::UpSpecial, Move::DownSpecial, Move::Other,
    Move::Grab, Move::Throw, Move::FinalSmash,
];

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
        shown: [0; 3],
    };
    /// The follow-up's events from the tracker's snaps in `(after, hit]`. The first 12 are kept.
    pub fn of(link: &Link, tracker: &Tracker, after: u32, aims: bool) -> LinkRec {
        let a1 = link.actionable as i32;
        let (events, drift) = events_between(tracker, after, link.hit, a1, aims);
        let mut rec = LinkRec {
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
            shown: [0; 3],
        };
        rec.set_descriptions([
            ActionDesc::of(link.from, link.shown[0], link.variants[0]),
            ActionDesc::of(link.to, link.shown[1], link.variants[1]),
        ]);
        rec
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
            shown: [0; 3],
        }
    }

    fn description_bits(&self, side: usize) -> u32 {
        let bits = u32::from(self.shown[0]) | (u32::from(self.shown[1]) << 8) | (u32::from(self.shown[2]) << 16);
        (bits >> (side * 12)) & 0xfff
    }

    pub(crate) fn set_descriptions(&mut self, desc: [ActionDesc; 2]) {
        let mut bits = 0u32;
        for (side, desc) in desc.into_iter().enumerate() {
            let of = desc.of.and_then(|mv| DESC_MOVES.iter().position(|&candidate| candidate == mv))
                .map_or(1, |index| index as u32 + 2);
            let angle = match desc.angle { None => 0, Some(Angle::Up) => 1, Some(Angle::Down) => 2 };
            bits |= (of | (angle << 5) | (u32::from(desc.normal.min(31)) << 7)) << (side * 12);
        }
        self.shown = [bits as u8, (bits >> 8) as u8, (bits >> 16) as u8];
    }

    /// Whether both moves have descriptions.
    pub fn described(&self) -> bool {
        self.description_bits(0) & 31 != 0 && self.description_bits(1) & 31 != 0
    }

    /// A move's saved description, with the follow-up's aim events.
    pub fn desc(&self, side: usize) -> ActionDesc {
        let bits = self.description_bits(side);
        let code = bits & 31;
        let of = code.checked_sub(2).and_then(|index| DESC_MOVES.get(index as usize)).copied();
        let angle = match (bits >> 5) & 3 { 1 => Some(Angle::Up), 2 => Some(Angle::Down), _ => None };
        let mut parts = [None; 2];
        if side == 1 {
            for event in self.events() {
                if let Kind::Aim(part @ 1..=2) = event.kind {
                    parts[usize::from(part - 1)] = Some(event.dir);
                }
            }
        }
        ActionDesc { mv: if side == 0 { self.from } else { self.to }, of, angle, normal: ((bits >> 7) & 31) as u8, parts }
    }

    pub fn events(&self) -> impl Iterator<Item = Event> + '_ {
        self.events.iter().flatten().copied()
    }

    fn holds_from(&self, before: impl Iterator<Item = Event> + Clone) -> [Option<u32>; STEPS] {
        let mut holds = [None; STEPS];
        for (i, event) in self.events.iter().enumerate() {
            let Some(Event { kind: Kind::Release(button), offset, .. }) = event else { continue };
            let owner = self.events[..i].iter().rev().flatten().copied().chain(before.clone())
                .find(|event| matches!(event.kind, Kind::Press(b) | Kind::Release(b) if b == *button));
            if let Some(Event { kind: Kind::Press(_), offset: press, .. }) = owner {
                holds[i] = u32::try_from(i64::from(*offset) - i64::from(press)).ok();
            }
        }
        holds
    }

    /// Holds owned by presses in this link.
    pub fn holds(&self) -> [Option<u32>; STEPS] {
        self.holds_from(core::iter::empty())
    }

    fn owns_aim_from(&self, after: impl Iterator<Item = Event> + Clone) -> [bool; STEPS] {
        let mut owns = [false; STEPS];
        for (i, event) in self.events.iter().enumerate() {
            if !matches!(event, Some(Event { kind: Kind::Press(Button::Special), .. })) { continue; }
            let next = self.events[i + 1..].iter().flatten().copied().chain(after.clone())
                .find(|event| matches!(event.kind, Kind::Press(Button::Special) | Kind::Aim(_)));
            owns[i] = next.is_some_and(|event| matches!(event.kind, Kind::Aim(_)));
        }
        owns
    }

    /// Special presses that own an aim in this link.
    pub fn owns_aim(&self) -> [bool; STEPS] {
        self.owns_aim_from(core::iter::empty())
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

/// One travel detail as a combo's table keeps it: the event it belongs to and its travel, in 24 bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Detail {
    /// The link in the high four bits and the event in the low four; `FREE` while unused.
    at: u8,
    /// The deciding stick's deflection in percent; `NO_STICK` when none was kept.
    deflection: u8,
    angle: u16,
    heading: u16,
    distance: u16,
    path: [(i8, i8); 8],
}

impl Detail {
    const FREE: u8 = 0xFF;
    const NO_STICK: u8 = 0xFF;
    const EMPTY: Detail =
        Detail { at: Self::FREE, deflection: Self::NO_STICK, angle: 0, heading: 0, distance: 0, path: [(0, 0); 8] };

    /// The key of an event; `link` is below `LINKS` and `event` below `STEPS`.
    const fn at(link: usize, event: usize) -> u8 {
        ((link << 4) | event) as u8
    }

    fn of(at: u8, travel: Travel) -> Detail {
        let (angle, deflection) = travel.stick.unwrap_or((0, Self::NO_STICK));
        Detail { at, deflection, angle, heading: travel.heading, distance: travel.distance, path: travel.path }
    }

    fn travel(&self) -> Travel {
        Travel {
            stick: (self.deflection != Self::NO_STICK).then_some((self.angle, self.deflection)),
            heading: self.heading,
            distance: self.distance,
            path: self.path,
        }
    }
}

/// The links of one exchange, in order, and the travel details of a drill's first `DETAILS` aims.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Combo {
    pub input_only: bool,
    pub overflow: bool,
    links: [Option<LinkRec>; LINKS],
    len: usize,
    /// Sorted by event, the free entries last.
    details: [Detail; DETAILS],
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
        details: [Detail::EMPTY; DETAILS],
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

    pub fn travel(&self, link: usize, event: usize) -> Option<Travel> {
        if link >= LINKS || event >= STEPS {
            return None;
        }
        let at = Detail::at(link, event);
        self.details.iter().find(|detail| detail.at == at).map(Detail::travel)
    }

    pub fn travels(&self, link: usize) -> [Option<Travel>; STEPS] {
        core::array::from_fn(|event| self.travel(link, event))
    }

    pub(crate) fn attach_travel(&mut self, link: usize, event: usize, travel: Travel) -> bool {
        if link >= self.len || event >= STEPS || self.links[link].is_none_or(|l| l.events[event].is_none()) {
            return false;
        }
        let detail = Detail::of(Detail::at(link, event), travel);
        if let Some(slot) = self.details.iter().position(|entry| entry.at == detail.at) {
            self.details[slot] = detail;
            return true;
        }
        let Some(mut slot) = self.details.iter().position(|entry| entry.at == Detail::FREE) else { return false };
        while slot > 0 && self.details[slot - 1].at > detail.at {
            self.details[slot] = self.details[slot - 1];
            slot -= 1;
        }
        self.details[slot] = detail;
        true
    }

    /// Whether the newest event for this button is an unreleased press.
    pub fn unreleased(&self, button: Button) -> bool {
        self.links[..self.len].iter().rev().flatten()
            .flat_map(|link| link.events.iter().rev().flatten())
            .find(|event| matches!(event.kind, Kind::Press(b) | Kind::Release(b) if b == button))
            .is_some_and(|event| matches!(event.kind, Kind::Press(_)))
    }

    /// Holds in step `k`, including presses owned by an earlier step.
    pub fn holds(&self, k: usize) -> [Option<u32>; STEPS] {
        let Some(link) = self.get(k) else { return [None; STEPS] };
        let before = self.links[..k].iter().rev().flatten()
            .flat_map(|link| link.events.iter().rev().flatten().copied());
        link.holds_from(before)
    }

    /// Special presses in step `k` that own an aim in this combo.
    pub fn owns_aim(&self, k: usize) -> [bool; STEPS] {
        let Some(link) = self.get(k) else { return [false; STEPS] };
        let after = self.links[k + 1..self.len].iter().flatten()
            .flat_map(|link| link.events.iter().flatten().copied());
        link.owns_aim_from(after)
    }

    pub(crate) fn join_split_steps(&mut self) {
        let mut k = 1;
        while k < self.len {
            let previous = self.links[k - 1].unwrap();
            let current = self.links[k].unwrap();
            let count = previous.events().count();
            if matches!(current.events().next().map(|event| event.kind), Some(Kind::Release(_) | Kind::Aim(1)))
                && count + current.events().count() <= STEPS
            {
                let previous = self.links[k - 1].as_mut().unwrap();
                for (slot, event) in previous.events[count..].iter_mut().zip(current.events()) {
                    *slot = Some(event);
                }
                for detail in self.details.iter_mut().filter(|detail| detail.at != Detail::FREE) {
                    let (link, event) = (usize::from(detail.at >> 4), usize::from(detail.at & 0xF));
                    if link == k {
                        detail.at = Detail::at(k - 1, event + count);
                    } else if link > k {
                        detail.at -= 0x10;
                    }
                }
                self.links.copy_within(k + 1..self.len, k);
                self.len -= 1;
                self.links[self.len] = None;
            } else {
                k += 1;
            }
        }
    }
}

/// A reference event's match in an attempt: the attempt's offset minus the reference's, or no such event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    At(i32),
    Missing,
    /// The same kind of event, in order, with a different direction.
    WrongDir,
    Path,
    Angle(i16),
    Deflection(i8),
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
    pub band: [u8; STEPS],
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

    pub fn detail(&self) -> Option<usize> {
        self.marks.iter().position(|m| matches!(m, Some(Mark::Path | Mark::Angle(_) | Mark::Deflection(_))))
    }

    /// The step with the largest offset outside the tolerance, with that offset.
    pub fn worst(&self, tolerance: u8) -> Option<(usize, i32)> {
        let mut worst: Option<(usize, i32)> = None;
        for (i, mark) in self.marks.iter().enumerate() {
            if let Some(Mark::At(d)) = mark {
                if d.unsigned_abs() > u32::from(tolerance) + u32::from(self.band[i])
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
            && self.detail().is_none()
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
    grade_with(reference, attempt, input_only, aims, GradeContext {
        ref_holds: reference.holds(),
        att_holds: attempt.holds(),
        owns_aim: reference.owns_aim(),
        ref_travels: [None; STEPS],
        att_travels: [None; STEPS],
        match_stick: false,
    })
}

struct GradeContext {
    ref_holds: [Option<u32>; STEPS],
    att_holds: [Option<u32>; STEPS],
    owns_aim: [bool; STEPS],
    ref_travels: [Option<Travel>; STEPS],
    att_travels: [Option<Travel>; STEPS],
    match_stick: bool,
}

fn grade_with(reference: &LinkRec, attempt: &LinkRec, input_only: bool, aims: bool, context: GradeContext) -> Grade {
    let mut used = [false; STEPS];
    let mut marks = [None; STEPS];
    let mut band = [0; STEPS];
    let mut from = 0;
    for (index, (mark, want)) in marks.iter_mut().zip(reference.events.iter()).enumerate() {
        let Some(want) = want else { continue };
        if input_only && matches!(want.kind, Kind::Release(_)) && context.ref_holds[index].is_none() {
            continue;
        }
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
                if reference.presses_only || got.dir == want.dir || matches!(want.kind, Kind::Release(_))
                    || (matches!(want.kind, Kind::Press(Button::Special)) && context.owns_aim[index])
                {
                    let delta = if let (Kind::Release(_), Some(saved), Some(actual)) =
                        (want.kind, context.ref_holds[index], context.att_holds[i])
                    {
                        band[index] = action::hold_band(saved);
                        (i64::from(actual) - i64::from(saved)).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
                    } else if aims && matches!(want.kind, Kind::Aim(_)) {
                        0
                    } else {
                        got.offset - want.offset
                    };
                    if matches!(want.kind, Kind::Aim(_)) && got.dir == want.dir {
                        if let (Some(reference), Some(attempt)) = (context.ref_travels[index], context.att_travels[i]) {
                            if action::path_off(&reference, &attempt) {
                                Mark::Path
                            } else if context.match_stick {
                                action::stick_off(&reference, &attempt).unwrap_or(Mark::At(delta))
                            } else { Mark::At(delta) }
                        } else { Mark::At(delta) }
                    } else { Mark::At(delta) }
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
        wrong_move: !input_only && if reference.described() && attempt.described() {
            !reference.desc(1).same_move(&attempt.desc(1))
        } else {
            attempt.to.legacy() != reference.to.legacy()
        },
        marks,
        band,
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

fn input_grade(target: &Combo, actual: &Combo, index: usize, rec: &LinkRec, extra: Option<u8>, match_stick: bool) -> Option<Grade> {
    let reference = target.get(index)?;
    let context = GradeContext {
        ref_holds: target.holds(index),
        att_holds: actual.holds(index),
        owns_aim: target.owns_aim(index),
        ref_travels: target.travels(index),
        att_travels: actual.travels(index),
        match_stick,
    };
    let mut grade = grade_with(reference, rec, true, target.has_aim(), context);
    grade.extra_aim = grade.extra_aim.or(extra);
    Some(grade)
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
    AimPath(u8),
    AimAngle(u8, i16),
    AimDeflection(u8, i8),
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
pub const ROW_STICK: &str = "combo_stick";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub on: bool,
    pub tolerance: u8,
    pub load_after: bool,
    /// *Match sweet/sour spot* (decision SP4, `spots::ROW_COMBO`): a link counts only when it lands the saved link's spot.
    pub match_spot: bool,
    pub match_stick: bool,
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
            match_stick: false,
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
            match_stick: super::spots::row_on(snapshot, ROW_STICK),
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
                self.step_inputs_late(snap, Some(tracker.late()), &mut out);
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

    #[cfg(test)]
    fn step_inputs(&mut self, snap: Snap, out: &mut Out) {
        self.step_inputs_late(snap, None, out);
    }

    fn step_inputs_late(&mut self, snap: Snap, late: Option<Late>, out: &mut Out) {
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
        let aims = self.target.as_ref().is_some_and(Combo::has_aim);
        let amended = self.input.step_with(snap, late.unwrap_or(Late::NONE), false, aims);
        let Some(start) = self.input.first else {
            return;
        };
        if self.attempt.is_none() {
            self.attempt = Some(Attempt::new(start, false));
        }
        if let Some(j) = amended[0] {
            self.regrade(j, out);
        }
        if let Some(j) = amended[1].filter(|j| Some(*j) != amended[0]) {
            self.regrade(j, out);
        }
        while let Some(attempt) = self.attempt.as_ref().filter(|a| a.open()) {
            let target = self.target.as_ref().unwrap();
            let aims = target.has_aim();
            let last = attempt.len + 1 == target.len();
            let reference = target.get(attempt.len).copied().unwrap();
            let angular = reference.events().any(|event| {
                matches!(event.kind, Kind::Press(Button::Attack) | Kind::CStick)
                    && matches!(event.dir, Dir::Forward | Dir::Back | Dir::ForwardUp | Dir::ForwardDown | Dir::BackUp | Dir::BackDown)
            });
            if late.is_some() && angular && self.input.undecided(attempt.len, snap.frame) {
                break;
            }
            let holds = target.holds(attempt.len);
            let deadline = reference.events.iter().enumerate().filter_map(|(i, event)| {
                event.map(|event| {
                    let band = holds[i].map_or(0, action::hold_band);
                    event.offset.saturating_add(i32::from(band))
                })
            }).max().unwrap_or(0).saturating_add(i32::from(self.settings.tolerance));
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

    fn regrade(&mut self, j: usize, out: &mut Out) {
        let grade = self.target.as_ref().filter(|target| target.input_only).and_then(|target| {
            let attempt = self.attempt.as_ref().filter(|attempt| attempt.open() && j < attempt.len)?;
            let previous = attempt.grades[j]?;
            let actual = self.input.combo.get(j).copied()?;
            let mut grade = input_grade(target, &self.input.combo, j, &actual, previous.extra_aim, self.settings.match_stick)?;
            grade.extra_aim = previous.extra_aim;
            Some(grade)
        });
        let Some(grade) = grade else { return };
        self.attempt.as_mut().unwrap().grades[j] = Some(grade);
        out.link = Some((j, grade));
        if !grade.clean(self.settings.tolerance) {
            self.resolve_open(Outcome::Completed, out);
        }
    }

    fn on_clear(&mut self, out: &mut Out) {
        let input_only = self.target.as_ref().is_some_and(|target| target.input_only);
        if !input_only || self.attempt.as_ref().is_none_or(|attempt| !attempt.open() || !attempt.is_empty()) {
            self.resolve_open(Outcome::Cleared, out);
        }
        if input_only {
            self.attempt = None;
        }
        self.pending_load = false;
        self.input.restart();
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
            if !first || target.get(0).is_none_or(|r| {
                if r.described() { r.from != rec.from } else { r.from.legacy() != rec.from.legacy() }
            }) {
                return;
            }
            self.attempt = Some(Attempt::new(exchange, false));
        }
        let Some(attempt) = self.attempt.as_mut() else {
            return;
        };
        if attempt.junction {
            attempt.junction = false;
            let matches = target.get(0).is_some_and(|r| {
                if r.described() { r.from == rec.to } else { r.from.legacy() == rec.to.legacy() }
            });
            if !matches {
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
        let g = if target.input_only {
            let Some(g) = input_grade(target, &self.input.combo, attempt.len, &rec, extra, self.settings.match_stick) else { return };
            g
        } else {
            let context = GradeContext { ref_holds: reference.holds(), att_holds: rec.holds(), owns_aim: reference.owns_aim(),
                ref_travels: [None; STEPS], att_travels: [None; STEPS], match_stick: false };
            let mut g = grade_with(reference, &rec, false, target.has_aim(), context);
            g.extra_aim = g.extra_aim.or(extra);
            g
        };
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
    let detail = || grades().find_map(|(i, g)| {
        let index = g.detail()?;
        let Kind::Aim(part) = event(g, index)?.kind else { return None };
        let reason = match g.marks[index]? {
            Mark::Path => Reason::AimPath(part),
            Mark::Angle(delta) => Reason::AimAngle(part, delta),
            Mark::Deflection(delta) => Reason::AimDeflection(part, delta),
            _ => return None,
        };
        Some((i as u8, reason))
    });
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
            .or_else(detail)
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
    decided: Option<u32>,
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
            decided: None,
        }
    }
    fn restart(&mut self) {
        *self = Self {
            prev: self.prev,
            ..Self::new()
        };
    }
    pub fn step(&mut self, snap: Snap) {
        self.step_late(snap, Late::NONE);
    }
    pub fn step_late(&mut self, snap: Snap, late: Late) {
        self.step_with(snap, late, true, true);
    }
    #[cfg(test)]
    fn step_mode(&mut self, snap: Snap, group_idle: bool, aims: bool) {
        self.step_with(snap, Late::NONE, group_idle, aims);
    }
    fn step_with(&mut self, mut snap: Snap, late: Late, group_idle: bool, aims: bool) -> [Option<usize>; 2] {
        if stick_axis(snap.lstick_y) < JUMP_STICK_Y {
            snap.buttons &= !FLICK_JUMP;
        }
        if self.combo.overflow { return [None; 2]; }
        let mut edges = [None; STEPS];
        let mut count = 0;
        push_edges(&mut edges, &mut count, self.prev, snap, aims);
        self.prev = Some(snap);
        for event in edges.into_iter().flatten() {
            if matches!(event.kind, Kind::Release(button) if !self.combo.unreleased(button)) {
                continue;
            }
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
                    && action::originates(event.kind)
                    && self
                        .last
                        .is_some_and(|last| event.frame - last > super::frame_strip::DROP_MAX)))
                && !self.combo.push(LinkRec::INPUT)
            {
                return [None; 2];
            }
            let link = self.combo.links[self.combo.len - 1].as_mut().unwrap();
            *link.events.iter_mut().find(|e| e.is_none()).unwrap() = Some(Event {
                kind: event.kind,
                dir: event.dir,
                offset: (event.frame - start) as i32,
            });
            self.last = Some(event.frame);
        }
        let mut amended = [None; 2];
        if let Some((Some(press), side)) = late.open {
            self.decided = Some(press);
            if let (Some(angle), Some(first)) = (side, self.first.filter(|&first| press >= first)) {
                let offset = (press - first) as i32;
                let chosen = [Kind::CStick, Kind::Press(Button::Attack)].into_iter().find_map(|kind| {
                    self.combo.links[..self.combo.len].iter().enumerate().find_map(|(step, link)| {
                        link.as_ref()?.events.iter().position(|event| {
                            event.is_some_and(|event| event.offset == offset && event.kind == kind)
                        }).map(|index| (step, index))
                    })
                });
                if let Some((step, index)) = chosen {
                    let event = self.combo.links[step].as_mut().unwrap().events[index].as_mut().unwrap();
                    let direction = action::angled(event.dir, angle);
                    if direction != event.dir {
                        event.dir = direction;
                        amended[0] = Some(step);
                    }
                }
            }
        }
        if let Some((part, travel)) = late.travel {
            let chosen = self.combo.links[..self.combo.len].iter().enumerate().rev().find_map(|(step, link)| {
                link.as_ref()?.events.iter().enumerate().rev().find_map(|(index, event)| {
                    event.is_some_and(|event| event.kind == Kind::Aim(part)).then_some((step, index))
                })
            });
            if let Some((step, index)) = chosen {
                if self.combo.attach_travel(step, index, travel) {
                    amended[1] = Some(step);
                }
            }
        }
        amended
    }
    fn undecided(&self, k: usize, now: u32) -> bool {
        let Some(first) = self.first else { return false };
        self.combo.get(k).is_some_and(|step| step.events().any(|event| {
            matches!(event.kind, Kind::Press(Button::Attack) | Kind::CStick)
                && u32::try_from(i64::from(first) + i64::from(event.offset)).ok().is_some_and(|frame| {
                    self.decided.is_none_or(|decided| decided < frame)
                        && now.saturating_sub(frame) < INPUT_LOOKBACK
                })
        }))
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

#[cfg(test)]
#[path = "reason_line_tests.rs"]
mod reason_line_tests;
