//! Fixed-capacity surface snapshot; native discovery is spread over frames.
use super::{add, dot, finite, scale, sub, Vec2};

pub const MAX_LINES: usize = 64;
pub const END_MARGIN: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Floor,
    Wall,
    Ceiling,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub id: usize,
    pub ends: [Vec2; 2],
    pub normal: Vec2,
    pub floor: bool,
    pub wall: bool,
    pub passable: bool,
}

impl Line {
    /// Geometry validity is independent of whether a line can be targeted.
    pub fn valid(self) -> bool {
        self.ends.into_iter().all(finite)
            && finite(self.normal)
            && self.length().is_finite()
            && self.length() > 0.0
    }
    pub fn kind(self) -> Option<Kind> {
        if !self.ends.into_iter().all(finite) || !finite(self.normal) {
            return None;
        }
        if self.passable {
            return (self.normal.y > 0.0 && !self.wall).then_some(Kind::Floor);
        }
        if self.floor || self.normal.y >= 0.7 {
            Some(Kind::Floor)
        } else if self.wall {
            Some(Kind::Wall)
        } else {
            (self.normal.y < 0.0).then_some(Kind::Ceiling)
        }
    }
    pub fn length(self) -> f32 {
        let d = sub(self.ends[1], self.ends[0]);
        dot(d, d).sqrt()
    }
    pub fn snap(self, cursor: Vec2) -> Option<Vec2> {
        let d = sub(self.ends[1], self.ends[0]);
        let n = dot(d, d);
        (finite(cursor) && self.valid() && n > 0.0 && n.is_finite()).then(|| {
            add(
                self.ends[0],
                scale(d, (dot(sub(cursor, self.ends[0]), d) / n).clamp(0.0, 1.0)),
            )
        })
    }
    /// Endpoints are compared exactly: even a small moved line is excluded.
    pub fn stationary(self, now: Self) -> bool {
        self == now
    }
    /// `unit` is uniform in [0,1]; Random excludes both five-unit corner margins.
    pub fn random_point(self, now: Self, unit: f32) -> Option<Vec2> {
        let length = self.length();
        if !self.stationary(now)
            || self.kind().is_none()
            || !length.is_finite()
            || length < 2.0 * END_MARGIN
            || !(0.0..=1.0).contains(&unit)
        {
            return None;
        }
        let t = (END_MARGIN + unit * (length - 2.0 * END_MARGIN)) / length;
        Some(add(self.ends[0], scale(sub(self.ends[1], self.ends[0]), t)))
    }
}

/// `next` follows the native neighbour at the far endpoint. The adapter resolves
/// which endpoint of the returned line is shared and returns its *far* end.
pub trait Source {
    fn read(&mut self, id: usize) -> Option<Line>;
    fn next(&mut self, line: Line, far_end: usize) -> Option<(usize, usize)>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Walk {
    More,
    Closed,
    Broken,
    Full,
}

pub struct Surfaces {
    lines: [Option<Line>; MAX_LINES],
    len: usize,
    cursor: Option<(usize, usize)>,
    seed: usize,
    pub state: Walk,
}
impl Surfaces {
    pub const EMPTY: Self = Self {
        lines: [None; MAX_LINES],
        len: 0,
        cursor: None,
        seed: 0,
        state: Walk::Broken,
    };
    pub fn begin(&mut self, seed: usize, far_end: usize) {
        *self = Self {
            cursor: Some((seed, far_end)),
            seed,
            state: Walk::More,
            ..Self::EMPTY
        };
    }
    pub fn lines(&self) -> impl Iterator<Item = Line> + '_ {
        self.lines[..self.len].iter().flatten().copied()
    }
    pub fn push(&mut self, line: Line) -> bool {
        if self.lines().any(|l| l.id == line.id) {
            return true;
        }
        if self.len == MAX_LINES || !line.valid() {
            return false;
        }
        self.lines[self.len] = Some(line);
        self.len += 1;
        true
    }
    /// One native line per invocation; a repeat other than the seed is broken.
    pub fn step(&mut self, source: &mut impl Source) -> Walk {
        if self.state != Walk::More {
            return self.state;
        }
        let result = (|| {
            let (id, end) = self.cursor?;
            if end > 1 {
                return None;
            }
            let line = source.read(id)?;
            if line.id != id || !line.valid() {
                return None;
            }
            if !self.push(line) {
                self.state = Walk::Full;
                return None;
            }
            let next = source.next(line, end)?;
            if next.0 == self.seed {
                self.state = Walk::Closed;
            } else if self.lines().any(|l| l.id == next.0) {
                return None;
            } else if self.len == MAX_LINES {
                self.state = Walk::Full;
            }
            self.cursor = Some(next);
            Some(())
        })();
        if result.is_none() && self.state == Walk::More {
            self.state = Walk::Broken;
        }
        self.state
    }
    pub fn nearest(&self, cursor: Vec2, kind: Option<Kind>) -> Option<(Line, Vec2)> {
        self.lines()
            .filter(|l| l.kind().is_some() && (kind.is_none() || l.kind() == kind))
            .filter_map(|l| l.snap(cursor).map(|p| (l, p)))
            .min_by(|a, b| {
                dot(sub(a.1, cursor), sub(a.1, cursor))
                    .total_cmp(&dot(sub(b.1, cursor), sub(b.1, cursor)))
            })
    }
}

/// Segment crossing. A passable platform blocks only a downward crossing from above.
pub fn crossing(a: Vec2, b: Vec2, line: Line) -> Option<f32> {
    let r = sub(b, a);
    if line.passable && (dot(sub(a, line.ends[0]), line.normal) < 0.0 || dot(r, line.normal) >= 0.0)
    {
        return None;
    }
    let s = sub(line.ends[1], line.ends[0]);
    let cross = |u: Vec2, v: Vec2| u.x * v.y - u.y * v.x;
    let denom = cross(r, s);
    if denom == 0.0 {
        return None;
    }
    let d = sub(line.ends[0], a);
    let (t, u) = (cross(d, s) / denom, cross(d, r) / denom);
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
}
