//! Directional-special parts are inferred from movement and the stick.
//! Entry, velocity, heading and the special's own direction determine an onset.
//! See `docs/design/action-model.md`.

use super::action::{self, Travel};

/// Minimum movement in game units before a travel's heading is known.
pub const MOVED: f32 = 1.0;
/// Frames after an entry, not counting hitstop, in which the movement must show.
pub const W: u8 = 3;
/// The least change of velocity, in units a frame, that makes a movement a new one.
pub const DV: f32 = 1.0;
/// How far a heading may lie from the stick, from the onset before it and from the special's own direction, in degrees.
pub const FOLLOW: f32 = 33.75;
/// The stick is off centre from this radius (raw stick units; 24 is 20% deflection).
pub const DEAD: i16 = 24;
/// A velocity slower than this, in units a frame, is not one that can be carried on.
const CARRIED: f32 = 0.3;

/// The nearest eight-way screen direction of a movement, once it has moved far enough.
pub fn heading(dx: f32, dy: f32) -> Option<u8> {
    if dx * dx + dy * dy < MOVED * MOVED {
        return None;
    }
    let (x, y) = (dx.abs(), dy.abs());
    Some(if y <= x * 0.414_213_56 {
        if dx > 0.0 { 0 } else { 4 }
    } else if x <= y * 0.414_213_56 {
        if dy > 0.0 { 2 } else { 6 }
    } else {
        match (dx > 0.0, dy > 0.0) {
            (true, true) => 1,
            (false, true) => 3,
            (false, false) => 5,
            (true, false) => 7,
        }
    })
}

/// What the strip knows about Player 1's directional special on one frame (F13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    /// Player 1 is in a special: from its start status to the end of its own statuses.
    pub inside: bool,
    /// 1 or 2 on the one frame that part's direction became known, else 0.
    pub part: u8,
    /// That part's direction (a `bin_stick` bin, screen directions); `None` when it did not move.
    pub dir: Option<u8>,
    /// Player 1's facing when the special began.
    pub facing: i8,
    /// Frames since that part's travel began.
    pub age: u8,
}

impl Seen {
    pub const NONE: Seen = Seen { inside: false, part: 0, dir: None, facing: 0, age: 0 };
}

fn angle(x: f32, y: f32) -> f32 {
    y.atan2(x).to_degrees().rem_euclid(360.0)
}

fn diff(a: f32, b: f32) -> f32 {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs()
}

/// Which special Player 1 is inside; it gives the direction the special goes by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family { Neutral, Side, Up, Down }

impl Family {
    /// Whether the heading is the special's own direction.
    pub fn nominal(self, heading: f32) -> bool {
        match self {
            Self::Neutral => false,
            Self::Side => diff(heading, 0.0).min(diff(heading, 180.0)) <= FOLLOW,
            Self::Up => diff(heading, 90.0) <= FOLLOW,
            Self::Down => diff(heading, 270.0) <= FOLLOW,
        }
    }
}

/// Whether the stick points along the heading, allowing downward ground travel along the same side.
pub fn follows(stick: (i8, i8), heading: f32, ground: bool) -> bool {
    let (x, y) = (f32::from(stick.0), f32::from(stick.1));
    if x * x + y * y < f32::from(DEAD) * f32::from(DEAD) { return false; }
    if diff(angle(x, y), heading) <= FOLLOW { return true; }
    let level = diff(heading, 0.0).min(diff(heading, 180.0)) <= FOLLOW;
    ground && level && y < 0.0 && x != 0.0 && (x > 0.0) == (heading.to_radians().cos() > 0.0)
}

/// One frame of Player 1 for [`Rule::step`].
#[derive(Clone, Copy, Debug)]
pub struct Now {
    pub frame: u32,
    pub status: i32,
    pub start: Option<Family>,
    pub own: bool,
    pub pos: Option<(f32, f32)>,
    pub stop: bool,
    pub facing: i8,
    pub grounded: bool,
    pub press: u32,
    /// The left stick on this frame.
    pub stick: (i8, i8),
}

/// The stick's eight-way bin on one frame; 8 inside the dead zone.
fn bin(stick: (i8, i8)) -> usize {
    let (x, y) = (f32::from(stick.0), f32::from(stick.1));
    if x * x + y * y < f32::from(DEAD) * f32::from(DEAD) { 8 } else { ((angle(x, y) + 22.5) / 45.0) as usize % 8 }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Candidate {
    status: i32,
    base: (f32, f32),
    old: (f32, f32),
    entry: u32,
    k: u8,
    ground: bool,
}

/// A travel being measured: an aim part, or as number 0 the special's own travel.
#[derive(Clone, Copy, Debug, PartialEq)]
struct OpenPart {
    number: u8,
    status: i32,
    base: (f32, f32),
    facing: i8,
    stick: Option<(u16, u8)>,
    last: (f32, f32),
    trail: [(f32, f32); 16],
    len: u8,
    stride: u32,
    tick: u32,
}

impl OpenPart {
    fn begun(number: u8, status: i32, base: (f32, f32), facing: i8, stick: Option<(u16, u8)>) -> Self {
        Self { number, status, base, facing, stick, last: (0.0, 0.0), trail: [(0.0, 0.0); 16], len: 0, stride: 1, tick: 0 }
    }

    /// Whether some kept position is far enough from the base to be a travel.
    fn has_moved(&self) -> bool {
        self.trail[..usize::from(self.len)].iter().chain(core::iter::once(&self.last))
            .any(|point| point.0 * point.0 + point.1 * point.1 >= MOVED * MOVED)
    }

    fn sample_part(&mut self, pos: (f32, f32)) {
        self.last = (pos.0 - self.base.0, pos.1 - self.base.1);
        self.tick += 1;
        if self.tick < self.stride { return; }
        self.tick = 0;
        if usize::from(self.len) == self.trail.len() {
            for i in 0..8 { self.trail[i] = self.trail[i * 2 + 1]; }
            self.len = 8;
            self.stride = self.stride.saturating_mul(2);
        }
        self.trail[usize::from(self.len)] = self.last;
        self.len += 1;
    }

    fn measured(&self) -> Travel {
        let mirror = if self.facing < 0 { -1.0 } else { 1.0 };
        let last = (if self.last.0 == 0.0 { 0.0 } else { self.last.0 * mirror }, self.last.1);
        let heading = ((last.1.atan2(last.0).to_degrees().rem_euclid(360.0) * 10.0).round() as u16) % 3600;
        let distance = (last.0.hypot(last.1) * 10.0).round().clamp(0.0, f32::from(u16::MAX)) as u16;
        let mut length = 0.0;
        let mut previous = (0.0_f32, 0.0_f32);
        for point in self.trail[..usize::from(self.len)].iter().copied().chain(core::iter::once(self.last)) {
            length += (point.0 - previous.0).hypot(point.1 - previous.1);
            previous = point;
        }
        let path = core::array::from_fn(|i| {
            let wanted = length * (i + 1) as f32 / 8.0;
            let mut travelled = 0.0;
            let mut previous = (0.0_f32, 0.0_f32);
            let mut point = self.last;
            for next in self.trail[..usize::from(self.len)].iter().copied().chain(core::iter::once(self.last)) {
                let segment = (next.0 - previous.0).hypot(next.1 - previous.1);
                if segment > 0.0 && travelled + segment >= wanted {
                    let t = (wanted - travelled) / segment;
                    point = (previous.0 + (next.0 - previous.0) * t, previous.1 + (next.1 - previous.1) * t);
                    break;
                }
                travelled += segment;
                previous = next;
            }
            ((point.0 * mirror).round().clamp(-127.0, 127.0) as i8,
                point.1.round().clamp(-127.0, 127.0) as i8)
        });
        Travel { stick: self.stick, heading, distance, path }
    }
}

/// The state of the outcome rule for the current special.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rule {
    family: Option<Family>,
    facing: i8,
    onsets: u8,
    statuses: [Option<i32>; 2],
    last_heading: Option<f32>,
    window: u32,
    last_status: Option<i32>,
    previous: Option<(f32, f32)>,
    previous2: Option<(f32, f32)>,
    previous_ground: bool,
    candidate: Option<Candidate>,
    part: Option<OpenPart>,
    ended: Option<(u8, Travel)>,
    /// The press that began the current special, and the one whose own travel ended last.
    press: u32,
    origin: u32,
    /// Frames the stick spent in each bin during the special's own travel, and the newest sample of each bin.
    held: [u8; 9],
    sticks: [(i8, i8); 9],
    #[cfg(test)]
    pub(crate) keep_nominal: bool,
}

impl Rule {
    pub const fn new() -> Self {
        Self { family: None, facing: 0, onsets: 0, statuses: [None; 2], last_heading: None,
            window: 0, last_status: None, previous: None, previous2: None, previous_ground: true,
            candidate: None, part: None, ended: None, press: 0, origin: 0, held: [0; 9], sticks: [(0, 0); 9],
            #[cfg(test)] keep_nominal: false }
    }

    /// Player 1 is inside a special as of the last step.
    pub fn inside(&self) -> bool { self.family.is_some() }

    /// The entry of a status whose movement may still show.
    pub fn pending(&self) -> Option<u32> { self.candidate.map(|candidate| candidate.entry) }

    /// The travel that ended on the last step and its part. Part 0 is the special's own travel: where the fighter
    /// went from the special's start to its end or to its first aim part, with the stick most held on the way.
    pub fn ended(&self) -> Option<(u8, Travel)> { self.ended }

    /// The press that began the special whose own travel ended last.
    pub fn own_press(&self) -> u32 { self.origin }

    /// End the open travel. An aim part is handed over as measured; the special's own travel only if it moved.
    fn finish_travel(&mut self) {
        let Some(part) = self.part.take() else { return };
        if part.number != 0 {
            self.ended = Some((part.number, part.measured()));
        } else if part.has_moved() {
            // The bin held longest; a tie goes to a direction before neutral. Neutral is kept as no deflection.
            let most = (0..9).rev().max_by_key(|&bin| self.held[bin]).unwrap_or(8);
            let stick = if most == 8 { (0, 0) } else { action::stick_of(self.sticks[most], part.facing) };
            self.ended = Some((0, Travel { stick: Some(stick), ..part.measured() }));
            self.origin = self.press;
        }
    }

    fn tally_stick(&mut self, stick: (i8, i8)) {
        let bin = bin(stick);
        self.held[bin] = self.held[bin].saturating_add(1);
        self.sticks[bin] = stick;
    }

    /// Advance the rule, looking back for the newest matching stick sample.
    pub fn step(&mut self, now: Now, mut stick: impl FnMut(u32, f32, bool) -> Option<(i8, i8)>) -> Seen {
        self.ended = None;
        let restart = now.start.is_some() && self.last_status != Some(now.status);
        let leaves = self.family.is_some() && now.start.is_none() && !now.own;
        let entering = self.family.is_some() && now.own && self.last_status != Some(now.status);
        let lasts = self.part.is_some_and(|part| if part.number == 0 {
            !restart && !leaves
        } else {
            now.status == part.status && (now.start.is_some() || now.own)
        });
        if !lasts {
            self.finish_travel();
        } else if self.part.is_some_and(|part| part.number == 0) {
            // The own travel stops sampling while an entry may turn out to be an aim part.
            self.tally_stick(now.stick);
            let measuring = !entering && self.candidate.is_none() && !now.stop;
            if let (Some(part), Some(pos)) = (self.part.as_mut(), now.pos.filter(|_| measuring)) {
                part.sample_part(pos);
            }
        } else if let (Some(part), Some(pos)) = (self.part.as_mut(), now.pos.filter(|_| !now.stop)) {
            part.sample_part(pos);
        }
        if restart {
            self.family = now.start;
            self.facing = now.facing;
            self.onsets = 0;
            self.statuses = [None; 2];
            self.last_heading = None;
            self.window = now.press;
            self.candidate = None;
            self.press = now.press;
            self.held = [0; 9];
            self.sticks = [(0, 0); 9];
            self.part = self.previous.or(now.pos).map(|base| OpenPart::begun(0, now.status, base, now.facing, None));
            self.tally_stick(now.stick);
            if let (Some(part), Some(pos)) = (self.part.as_mut(), now.pos.filter(|_| !now.stop)) {
                part.sample_part(pos);
            }
        } else if leaves {
            self.family = None;
            self.candidate = None;
        }
        if self.family.is_some() && now.own && self.last_status != Some(now.status) {
            let old = match (self.previous, self.previous2) {
                (Some(a), Some(b)) => (a.0 - b.0, a.1 - b.1),
                _ => (0.0, 0.0),
            };
            self.candidate = self.previous.or(now.pos).map(|base| Candidate {
                status: now.status, base, old, entry: now.frame, k: 0, ground: self.previous_ground,
            });
        }

        let mut seen = if self.family.is_some() {
            Seen { inside: true, facing: self.facing, ..Seen::NONE }
        } else { Seen::NONE };
        if let Some(mut candidate) = self.candidate {
            if now.status != candidate.status {
                self.candidate = None;
            } else if let Some(pos) = now.pos.filter(|_| !now.stop) {
                let dx = pos.0 - candidate.base.0;
                let dy = pos.1 - candidate.base.1;
                if dx * dx + dy * dy < MOVED * MOVED {
                    candidate.k += 1;
                    self.candidate = (candidate.k < W).then_some(candidate);
                } else {
                    let n = f32::from(candidate.k + 1);
                    let new = (dx / n, dy / n);
                    let old = candidate.old;
                    let old_speed = old.0.hypot(old.1);
                    let new_speed = new.0.hypot(new.1);
                    let heading = angle(dx, dy);
                    let carried = old_speed >= CARRIED && diff(angle(old.0, old.1), heading) <= FOLLOW
                        && new_speed <= old_speed + DV;
                    let turned = self.last_heading.is_none_or(|last| diff(last, heading) > FOLLOW);
                    let impulse = self.statuses.contains(&Some(now.status))
                        || ((new.0 - old.0).hypot(new.1 - old.1) >= DV && !carried && turned);
                    if impulse {
                        self.onsets = self.onsets.saturating_add(1);
                        let deciding = stick(self.window, heading, candidate.ground);
                        #[cfg(test)]
                        let nominal = self.family.is_some_and(|family| family.nominal(heading)) && !self.keep_nominal;
                        #[cfg(not(test))]
                        let nominal = self.family.is_some_and(|family| family.nominal(heading));
                        if deciding.is_some() && !nominal && self.onsets <= 2 {
                            seen.part = self.onsets;
                            seen.dir = self::heading(dx, dy);
                            seen.age = now.frame.saturating_sub(candidate.entry).min(u32::from(u8::MAX)) as u8;
                            let mut part = OpenPart::begun(seen.part, now.status, candidate.base, self.facing,
                                deciding.map(|sample| action::stick_of(sample, self.facing)));
                            part.sample_part(pos);
                            // The special's own travel ends where its first aim part begins.
                            self.finish_travel();
                            self.part = Some(part);
                        }
                        self.window = now.frame.saturating_add(1);
                        self.last_heading = Some(heading);
                        if !self.statuses.contains(&Some(now.status)) {
                            if self.statuses[0].is_none() { self.statuses[0] = Some(now.status); }
                            else if self.statuses[1].is_none() { self.statuses[1] = Some(now.status); }
                        }
                    }
                    self.candidate = None;
                }
            }
        }
        self.last_status = Some(now.status);
        if !now.stop {
            self.previous2 = self.previous;
            self.previous = now.pos;
            self.previous_ground = now.grounded;
        }
        seen
    }
}

impl Default for Rule {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
#[path = "aim_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "aim_f13_fixture.rs"]
mod f13;
