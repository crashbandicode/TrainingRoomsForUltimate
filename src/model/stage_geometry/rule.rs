//! The off-stage rule (artifacts/phase23/training-stage-tracing/FINDINGS.md §6): where a point is against the stage,
//! the ledges with open air, the default spot beside one. Pure: the stage is read through `Probe`, by the game's ray
//! on a fighter's frame or from copied lines. No stage is named.
use super::{Point, Segment};
use crate::model::tech::surface::{Line, MAX_LINES};

/// Open-air columns are this far apart, as the stage scan's are.
const COLUMN: f32 = 10.0;
/// Lines read in one column before the read gives up.
const LINES: usize = 16;
/// Columns or runs searched on one side.
const SEARCH: usize = 64;

/// One stage line as world geometry. No native id survives the read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Met {
    pub ends: Segment,
    /// A fighter can stand on it: placement's floor rule with platforms (`is_placement_floor(flags, normal_y, true)`).
    pub stand: bool,
    /// A pass-through platform. It is never part of a solid.
    pub through: bool,
}
impl Met {
    /// The line's height at `x` (its ends' line; `None` for a vertical line).
    pub fn height(self, x: f32) -> Option<f32> {
        let [[a, y], [b, z]] = self.ends;
        if ![a, y, b, z, x].into_iter().all(f32::is_finite) || a == b {
            return None;
        }
        let height = y + (z - y) * ((x - a) / (b - a));
        height.is_finite().then_some(height)
    }
    /// `stand && !through`.
    pub fn solid_floor(self) -> bool {
        self.stand && !self.through
    }
}

/// The stage could not be read (a guard, a line that is gone).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unread;

/// The stage on this frame, read by the point ray.
pub trait Probe {
    /// The first line of any kind the ray meets from `from` straight to the height `to`: down when `to < from[1]`,
    /// up when `to > from[1]`. `platforms = false` passes through pass-through lines, as the game's ray does.
    fn first(&mut self, from: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread>;
}

/// The box is this far inside every blast line (Guard the Edge's limit today).
pub const INSIDE: f32 = 1.0;
/// A point up to this far under a floor's surface stands on it (`placement::floor_under`'s slack).
pub const SLACK: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Not `INSIDE` every blast line.
    Outside,
    /// In a solid: the nearest solid line above the point is a floor, more than `SLACK` above it.
    Solid,
    /// A line a fighter can stand on (a solid floor or a platform) lies under the point, above the bottom blast line.
    OverStage,
    /// Off stage, no solid above the point.
    Open,
    /// Off stage, a solid above the point (below an island, below the main stage).
    Under,
}
impl Place {
    /// `Open`, or `Under` when `under` is allowed.
    pub fn off_stage(self, under: bool) -> bool {
        self == Self::Open || under && self == Self::Under
    }
}

/// Where the next cast starts to pass a line met at `height` on the way to `to`: the game's ray accepts a line 1e-5
/// of its length behind its origin (`tech::discovery::Rays`).
fn past(height: f32, to: f32, down: bool) -> f32 {
    let reach = 1e-5 * (to - height).abs();
    if down { (height - reach).next_down() } else { (height + reach).next_up() }
}

/// The lines of one column from `from` toward `to`, nearest first, until `each` says stop.
fn column(
    stage: &mut impl Probe, x: f32, from: f32, to: f32, platforms: bool, mut each: impl FnMut(Met, f32) -> bool,
) -> Result<(), Unread> {
    let down = to < from;
    let mut y = from;
    for _ in 0..LINES {
        if y == to || (to < y) != down {
            return Ok(());
        }
        let Some(line) = stage.first([x, y], to, platforms)? else { return Ok(()) };
        let height = line.height(x).ok_or(Unread)?;
        if !each(line, height) {
            return Ok(());
        }
        let next = past(height, to, down);
        if next == y || (next < y) != down {
            return Err(Unread);
        }
        y = next;
    }
    // One more look: a column that still holds a line is too deep to be read.
    if y == to || (to < y) != down {
        return Ok(());
    }
    match stage.first([x, y], to, platforms)? {
        Some(_) => Err(Unread),
        None => Ok(()),
    }
}

/// `blast` is `[left, right, top, bottom]`, as `tech_surfaces::blast_bounds` gives it.
pub fn classify(stage: &mut impl Probe, blast: [f32; 4], at: Point) -> Result<Place, Unread> {
    let [left, right, top, bottom] = blast;
    let [x, y] = at;
    if !(left + INSIDE <= x && x <= right - INSIDE && bottom + INSIDE <= y && y <= top - INSIDE) {
        return Ok(Place::Outside);
    }
    // The nearest solid line above the point, however near: down the column from the top, and up from the point.
    let mut above: Option<(Met, f32)> = None;
    column(stage, x, top, y, false, |line, height| {
        if height > y {
            above = Some((line, height));
        }
        true
    })?;
    column(stage, x, y, top, false, |line, height| {
        if height <= y {
            return true;
        }
        if above.is_none_or(|(_, nearest)| height < nearest) {
            above = Some((line, height));
        }
        false
    })?;
    let mut start = y + SLACK;
    if let Some((line, height)) = above {
        if line.solid_floor() {
            // The floor-only slack: a point a little under a floor's surface stands on it.
            return Ok(if height <= y + SLACK { Place::OverStage } else { Place::Solid });
        }
        if height <= start {
            start = past(height, bottom, true);
        }
    }
    let mut over = false;
    column(stage, x, start, bottom, true, |line, height| {
        over = line.stand && height > bottom;
        !over
    })?;
    Ok(if over { Place::OverStage } else if above.is_some() { Place::Under } else { Place::Open })
}

/// An end of a solid floor run with open air beside it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ledge {
    pub at: Point,
    /// Where the open air is: `-1.0` left of the ledge, `1.0` right of it.
    pub facing: f32,
}

/// A run of solid floors joined end to end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorRun {
    /// Its left end and its right end.
    pub ends: Segment,
    /// For the left end and the right end: the ledge, when that end has open air beside it.
    pub ledges: [Option<Ledge>; 2],
}

/// The first solid line from `at` down to the bottom blast line, when it is a solid floor.
fn solid_floor_at(stage: &mut impl Probe, at: Point, bottom: f32) -> Result<Option<Met>, Unread> {
    if !(at[1] > bottom) {
        return Ok(None);
    }
    Ok(stage.first(at, bottom, false)?.filter(|line| line.solid_floor()))
}

/// A run's end on one side: the walk through solid floors that share an exact end point.
fn run_end(stage: &mut impl Probe, bottom: f32, mut floor: Met, right: bool) -> Result<Point, Unread> {
    let dir = if right { 1.0 } else { -1.0 };
    for _ in 0..MAX_LINES {
        let [a, b] = floor.ends;
        let end = if (a[0] < b[0]) == right { b } else { a };
        match solid_floor_at(stage, [end[0] + dir, end[1] + SLACK], bottom)? {
            Some(next) if next.ends.contains(&end) && next.ends != floor.ends => floor = next,
            _ => return Ok(end),
        }
    }
    Err(Unread)
}

fn ledge(stage: &mut impl Probe, blast: [f32; 4], end: Point, facing: f32) -> Result<Option<Ledge>, Unread> {
    let beside = [end[0] + facing, end[1] - 1.0];
    Ok((classify(stage, blast, beside)? == Place::Open).then_some(Ledge { at: end, facing }))
}

/// The solid floor run under `at` (the ledge walk, seeded at any floor). `None`: no solid floor under the point.
pub fn floor_run(stage: &mut impl Probe, blast: [f32; 4], at: Point) -> Result<Option<FloorRun>, Unread> {
    let bottom = blast[3];
    let Some(floor) = solid_floor_at(stage, [at[0], at[1] + SLACK], bottom)? else { return Ok(None) };
    let ends = [run_end(stage, bottom, floor, false)?, run_end(stage, bottom, floor, true)?];
    let ledges = [ledge(stage, blast, ends[0], -1.0)?, ledge(stage, blast, ends[1], 1.0)?];
    Ok(Some(FloorRun { ends, ledges }))
}

/// Where the open air beside a ledge ends: the x of the next solid's near end, or of the side blast line. No inset.
pub fn open_air_end(stage: &mut impl Probe, blast: [f32; 4], ledge: Ledge) -> Result<f32, Unread> {
    let [left, right, top, bottom] = blast;
    let dir = if ledge.facing < 0.0 { -1.0 } else { 1.0 };
    let limit = if dir < 0.0 { left } else { right };
    // The end of a line that is nearer the ledge.
    let near = |line: Met| {
        let [a, b] = line.ends;
        if (a[0] < b[0]) == (dir > 0.0) { a[0] } else { b[0] }
    };
    let mut open = ledge.at[0];
    let mut x = open + dir;
    for _ in 0..4 * SEARCH {
        if (limit - x) * dir <= 0.0 {
            return Ok(limit);
        }
        if let Some(line) = stage.first([x, top], bottom, false)? {
            let mut end = near(line);
            // A joined face nearer the ledge: step back toward the last open column.
            for _ in 0..8 {
                let back = end - dir;
                if (back - open) * dir <= 0.0 {
                    break;
                }
                match stage.first([back, top], bottom, false)? {
                    Some(line) if (near(line) - end) * dir < 0.0 => end = near(line),
                    _ => break,
                }
            }
            return Ok(if (end - open) * dir < 0.0 { open } else { end });
        }
        open = x;
        x += dir * COLUMN;
    }
    Err(Unread)
}

/// The default spot beside a ledge: halfway across its open air and halfway down to the bottom of the box.
/// `None`: that spot is not `Place::Open`.
pub fn default_spot(stage: &mut impl Probe, blast: [f32; 4], ledge: Ledge) -> Result<Option<Point>, Unread> {
    let dir = if ledge.facing < 0.0 { -1.0 } else { 1.0 };
    let end = open_air_end(stage, blast, ledge)?;
    let spot = [(ledge.at[0] + dir).midpoint(end - dir), (blast[3] + INSIDE).midpoint(ledge.at[1])];
    Ok((classify(stage, blast, spot)? == Place::Open).then_some(spot))
}

/// The nearest ledge with open air to a point. `None`: the stage has none within reach of the search.
pub fn nearest_ledge(stage: &mut impl Probe, blast: [f32; 4], at: Point) -> Result<Option<Ledge>, Unread> {
    let [left, right, top, bottom] = blast;
    if !at.into_iter().chain(blast).all(f32::is_finite) || left + INSIDE > right - INSIDE {
        return Ok(None);
    }
    let start = at[0].clamp(left + INSIDE, right - INSIDE);
    let mut best: Option<(Ledge, f32)> = None;
    for right_side in [false, true] {
        let dir = if right_side { 1.0 } else { -1.0 };
        let mut x = start;
        // The first column starts at the point; the columns after it are read from the top.
        let mut from = (at[1] + SLACK).min(top);
        for _ in 0..SEARCH {
            if !(left + INSIDE <= x && x <= right - INSIDE) {
                break;
            }
            let mut floor = solid_floor_at(stage, [x, from], bottom)?;
            if floor.is_none() && from != top {
                floor = solid_floor_at(stage, [x, top], bottom)?;
            }
            from = top;
            let Some(floor) = floor else {
                x += dir * COLUMN;
                continue;
            };
            let end = run_end(stage, bottom, floor, right_side)?;
            if let Some(found) = ledge(stage, blast, end, dir)? {
                let distance = (found.at[0] - at[0]).hypot(found.at[1] - at[1]);
                if best.is_none_or(|(_, nearest)| distance < nearest) {
                    best = Some((found, distance));
                }
                break;
            }
            let next = end[0] + dir;
            x = if (next - x) * dir > 0.0 { next } else { x + dir * COLUMN };
        }
    }
    Ok(best.map(|(found, _)| found))
}

/// The same queries over copied lines; pure, any thread.
pub struct Copied<'a>(pub &'a [Met]);
impl Probe for Copied<'_> {
    fn first(&mut self, [x, y]: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread> {
        let (low, high) = if to < y { (to, y) } else { (y, to) };
        Ok(self.0.iter().copied()
            .filter(|line| platforms || !line.through)
            .filter(|line| {
                let [a, b] = line.ends;
                a[0].min(b[0]) <= x && x <= a[0].max(b[0])
            })
            .filter_map(|line| line.height(x).map(|height| (line, height)))
            .filter(|&(_, height)| low <= height && height <= high)
            .min_by(|a, b| (a.1 - y).abs().total_cmp(&(b.1 - y).abs()))
            .map(|(line, _)| line))
    }
}

/// A traced line as the rule sees it: `stand` is placement's floor rule with platforms.
pub fn met(line: Line) -> Met {
    let stand = if line.passable { !line.wall && line.normal.y > 0.0 } else { line.floor };
    Met { ends: line.ends.map(|point| [point.x, point.y]), stand, through: line.passable }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::tech::{discovery::decode, Vec2};

    fn line(ends: Segment, stand: bool, through: bool) -> Met { Met { ends, stand, through } }
    fn ledge(at: Point, facing: f32) -> Option<Ledge> { Some(Ledge { at, facing }) }

    /// A line and whether its front faces up; front-only rays meet only the facing side.
    struct Faced { lines: Vec<(Met, bool)>, front_only: bool }
    impl Probe for Faced {
        fn first(&mut self, [x, y]: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread> {
            let mut best: Option<(Met, f32)> = None;
            for &(line, up) in &self.lines {
                if (!platforms && line.through) || (self.front_only && up != (to < y)) {
                    continue;
                }
                let [[a, ay], [b, by]] = line.ends;
                if a == b || !(a.min(b)..=a.max(b)).contains(&x) {
                    continue;
                }
                let height = ay + (by - ay) * ((x - a) / (b - a));
                if !(y.min(to)..=y.max(to)).contains(&height) {
                    continue;
                }
                let distance = (height - y).abs();
                if best.is_none_or(|(_, nearest)| distance < nearest) {
                    best = Some((line, distance));
                }
            }
            Ok(best.map(|(line, _)| line))
        }
    }

    enum Fixture<'a> { Faced(Faced), Copied(Copied<'a>) }
    impl Probe for Fixture<'_> {
        fn first(&mut self, from: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread> {
            match self {
                Self::Faced(stage) => stage.first(from, to, platforms),
                Self::Copied(stage) => stage.first(from, to, platforms),
            }
        }
    }
    fn each(lines: &[(Met, bool)], mut check: impl FnMut(&mut Fixture<'_>)) {
        let copied: Vec<_> = lines.iter().map(|(line, _)| *line).collect();
        for mut stage in [
            Fixture::Faced(Faced { lines: lines.to_vec(), front_only: false }),
            Fixture::Faced(Faced { lines: lines.to_vec(), front_only: true }),
            Fixture::Copied(Copied(&copied)),
        ] {
            check(&mut stage);
        }
    }

    const TRAINING: [f32; 4] = [-590.0, 500.0, 360.0, -137.852];
    const BATTLEFIELD: [f32; 4] = [-240.0, 240.0, 192.0, -140.0];

    fn training() -> Vec<(Met, bool)> {
        vec![
            (line([[-635., 47.391], [-490., 47.391]], true, false), true),
            (line([[-490., 0.], [-420., 0.]], true, false), true),
            (line([[-635., -165.], [-420., -165.]], false, false), false),
            (line([[-330., 0.014], [-170., 0.014]], true, false), true),
            (line([[-330., 0.014], [-250., -40.]], false, false), false),
            (line([[-250., -40.], [-170., 0.014]], false, false), false),
            (line([[-309.4, 24.119], [-274.8, 24.119]], true, true), true),
            (line([[-267.3, 47.189], [-232.7, 47.189]], true, true), true),
            (line([[-225.2, 24.119], [-190.6, 24.119]], true, true), true),
            (line([[-80., 0.], [620., 0.]], true, false), true),
            (line([[-80., -340.], [620., -340.]], false, false), false),
        ]
    }

    fn battlefield() -> Vec<(Met, bool)> {
        vec![
            (line([[-79.986, 0.], [79.989, 0.]], true, false), true),
            (line([[-79.986, 0.], [0., -40.1]], false, false), false),
            (line([[0., -40.1], [79.989, 0.]], false, false), false),
            (line([[-57.6, 27.2], [-20., 27.2]], true, true), true),
            (line([[20., 27.2], [57.6, 27.2]], true, true), true),
            (line([[-18.8, 54.4], [18.8, 54.4]], true, true), true),
        ]
    }

    #[test]
    fn a_copied_stage_answers_the_first_line_either_way() {
        let floor = line([[-10., 0.], [10., 0.]], true, false);
        let underside = line([[-10., -5.], [10., -5.]], false, false);
        let platform = line([[-10., 10.], [10., 10.]], true, true);
        let vertical = line([[0., -8.], [0., 18.]], false, false);
        let elsewhere = line([[30., 15.], [40., 15.]], true, false);
        let lines = [floor, underside, platform, vertical, elsewhere];
        let mut stage = Copied(&lines);
        assert_eq!(stage.first([0., 20.], -10., true), Ok(Some(platform)));
        assert_eq!(stage.first([0., 20.], -10., false), Ok(Some(floor)));
        assert_eq!(stage.first([0., -10.], 20., false), Ok(Some(underside)));
        assert_eq!(stage.first([0., 1.], 20., true), Ok(Some(platform)));
        assert_eq!(stage.first([0., 1.], 20., false), Ok(None));
        assert_eq!(stage.first([0., -10.], -6., true), Ok(None));
        assert_eq!(stage.first([20., 20.], -20., true), Ok(None));
        assert_eq!(floor.height(0.), Some(0.));
        assert!(floor.solid_floor());
        assert!(!platform.solid_floor());
    }

    #[test]
    fn a_traced_line_stands_by_placements_rule() {
        let ends = [Vec2 { x: -10., y: 0. }, Vec2 { x: 10., y: 2. }];
        for byte in [0x00, 0x01, 0x02, 0x08, 0x09, 0x20, 0x21, 0x22, 0x28, 0x29] {
            for normal_y in [1.0, 0.8, 0.0, -1.0] {
                let actual = met(decode(1, ends, Vec2 { x: 1.0, y: normal_y }, byte).unwrap());
                assert_eq!(actual.stand, super::super::is_placement_floor((byte as u32) << 8, normal_y, true));
                assert_eq!(actual.through, byte & 0x20 != 0);
                assert_eq!(actual.ends, [[-10., 0.], [10., 2.]]);
            }
        }
    }

    #[test]
    fn a_line_at_the_end_of_a_column_is_passed_once() {
        let blast = [-100., 100., 100., -100.];
        let floor = [(line([[-10., 5.], [10., 5.]], true, false), true)];
        each(&floor, |stage| {
            assert_eq!(classify(stage, blast, [0., 5.]), Ok(Place::OverStage));
            assert_eq!(classify(stage, blast, [0., 4.999]), Ok(Place::OverStage));
        });
        let underside = [(line([[-10., 5.], [10., 5.]], false, false), false)];
        each(&underside, |stage| {
            assert!(classify(stage, blast, [0., 5.]).is_ok());
            assert_eq!(classify(stage, blast, [0., 4.]), Ok(Place::Under));
        });
    }

    #[test]
    fn training_points_are_classed_by_the_rule() {
        let cases = [
            ([-375., -68.4], Place::Open), ([-125., -68.4], Place::Open),
            ([-375., 300.], Place::Open), ([-81., -1.], Place::Open),
            ([-250., -68.4], Place::Under), ([-250., -20.], Place::Solid),
            ([270., -68.4], Place::Solid), ([270., -136.], Place::Solid),
            ([-550., 20.], Place::Solid), ([-250., 0.014], Place::OverStage),
            ([-250., 100.], Place::OverStage), ([-250., 30.], Place::OverStage),
            ([270., 20.], Place::OverStage), ([-450., 20.], Place::OverStage),
            ([-489., 46.391], Place::OverStage), ([-600., 100.], Place::Outside),
            ([0., 359.5], Place::Outside), ([0., -137.], Place::Outside),
        ];
        each(&training(), |stage| for (point, expected) in cases {
            assert_eq!(classify(stage, TRAINING, point), Ok(expected), "{point:?}");
        });
    }

    #[test]
    fn an_underside_just_over_the_point_is_under_not_solid() {
        each(&training(), |stage| for (point, expected) in [
            ([-250., -40.5], Place::Under), ([-250., -0.5], Place::OverStage),
            ([-250., -1.5], Place::Solid),
        ] {
            assert_eq!(classify(stage, TRAINING, point), Ok(expected), "{point:?}");
        });
        each(&battlefield(), |stage| {
            assert_eq!(classify(stage, BATTLEFIELD, [0., -40.6]), Ok(Place::Under));
        });
    }

    #[test]
    fn battlefield_points_are_classed_by_the_rule() {
        let cases = [
            ([0., -69.5], Place::Under), ([0., -20.], Place::Solid),
            ([160., -69.5], Place::Open), ([-160., -69.5], Place::Open),
            ([0., 50.], Place::OverStage), ([0., 0.], Place::OverStage),
            ([239.5, 0.], Place::Outside),
        ];
        each(&battlefield(), |stage| for (point, expected) in cases {
            assert_eq!(classify(stage, BATTLEFIELD, point), Ok(expected), "{point:?}");
        });
    }

    #[test]
    fn a_floor_run_names_its_ledges_with_open_air() {
        let cases = [
            ([-30., 0.], Some(FloorRun { ends: [[-80., 0.], [620., 0.]],
                ledges: [ledge([-80., 0.], -1.), None] })),
            ([-250., 47.2], Some(FloorRun { ends: [[-330., 0.014], [-170., 0.014]],
                ledges: [ledge([-330., 0.014], -1.), ledge([-170., 0.014], 1.)] })),
            ([-450., 0.], Some(FloorRun { ends: [[-490., 0.], [-420., 0.]],
                ledges: [None, ledge([-420., 0.], 1.)] })),
            ([-550., 47.4], Some(FloorRun { ends: [[-635., 47.391], [-490., 47.391]],
                ledges: [None, None] })),
            ([-375., 0.], None),
        ];
        each(&training(), |stage| for (point, expected) in cases {
            assert_eq!(floor_run(stage, TRAINING, point), Ok(expected), "{point:?}");
        });
        each(&battlefield(), |stage| {
            assert_eq!(floor_run(stage, BATTLEFIELD, [0., 0.]), Ok(Some(FloorRun {
                ends: [[-79.986, 0.], [79.989, 0.]],
                ledges: [ledge([-79.986, 0.], -1.), ledge([79.989, 0.], 1.)],
            })));
        });
        let joined = [
            (line([[-30., 0.], [-10., 0.]], true, false), true),
            (line([[-10., 0.], [10., 5.]], true, false), true),
            (line([[10., 5.], [30., 5.]], true, false), true),
        ];
        each(&joined, |stage| {
            assert_eq!(floor_run(stage, BATTLEFIELD, [-20., 0.]), Ok(Some(FloorRun {
                ends: [[-30., 0.], [30., 5.]],
                ledges: [ledge([-30., 0.], -1.), ledge([30., 5.], 1.)],
            })));
        });
        let split = [
            joined[0],
            (line([[(-10_f32).next_up(), 0.], [10., 5.]], true, false), true),
        ];
        each(&split, |stage| {
            assert_eq!(floor_run(stage, BATTLEFIELD, [-20., 0.]), Ok(Some(FloorRun {
                ends: [[-30., 0.], [-10., 0.]],
                ledges: [ledge([-30., 0.], -1.), None],
            })));
        });
    }

    #[test]
    fn the_open_air_ends_at_the_next_solid_or_the_blast_line() {
        each(&training(), |stage| for (end, facing, expected) in [
            ([-420., 0.], 1., -330.), ([-330., 0.014], -1., -420.),
            ([-170., 0.014], 1., -80.), ([-80., 0.], -1., -170.),
        ] {
            assert_eq!(open_air_end(stage, TRAINING, Ledge { at: end, facing }), Ok(expected), "{end:?}");
        });
        each(&battlefield(), |stage| for (end, facing, expected) in [
            ([79.989, 0.], 1., 240.), ([-79.986, 0.], -1., -240.),
        ] {
            assert_eq!(open_air_end(stage, BATTLEFIELD, Ledge { at: end, facing }), Ok(expected), "{end:?}");
        });
    }

    #[test]
    fn the_default_spot_is_halfway_across_and_halfway_down() {
        each(&training(), |stage| for (end, facing, expected) in [
            ([-420., 0.], 1., [-375., -68.426]), ([-80., 0.], -1., [-125., -68.426]),
        ] {
            let actual = default_spot(stage, TRAINING, Ledge { at: end, facing });
            assert!(matches!(actual, Ok(Some(p)) if (p[0] - expected[0]).abs() < 1e-3 && (p[1] - expected[1]).abs() < 1e-3),
                    "{end:?}: {actual:?}");
        });
        each(&battlefield(), |stage| for (end, facing, expected) in [
            ([79.989, 0.], 1., [159.9945, -69.5]), ([-79.986, 0.], -1., [-159.993, -69.5]),
        ] {
            let actual = default_spot(stage, BATTLEFIELD, Ledge { at: end, facing });
            assert!(matches!(actual, Ok(Some(p)) if (p[0] - expected[0]).abs() < 1e-3 && (p[1] - expected[1]).abs() < 1e-3),
                    "{end:?}: {actual:?}");
        });
        let platform = [(line([[45., -70.], [55., -70.]], true, true), true)];
        each(&platform, |stage| {
            assert_eq!(default_spot(stage, [-100., 100., 100., -100.], Ledge { at: [0., 0.], facing: 1. }), Ok(None));
        });
    }

    #[test]
    fn the_nearest_ledge_is_found_run_by_run() {
        let cases = [
            ([-30., 0.], ledge([-80., 0.], -1.)),
            ([400., 0.], ledge([-80., 0.], -1.)),
            ([-240., 47.2], ledge([-170., 0.014], 1.)),
            ([-550., 47.4], ledge([-420., 0.], 1.)),
            ([-250., -80.], ledge([-330., 0.014], -1.)),
        ];
        each(&training(), |stage| {
            for (point, expected) in cases {
                assert_eq!(nearest_ledge(stage, TRAINING, point), Ok(expected), "{point:?}");
            }
            let found = nearest_ledge(stage, TRAINING, [-30., 0.]);
            assert_eq!(found, Ok(ledge([-80., 0.], -1.)));
            let spot = default_spot(stage, TRAINING, found.unwrap().unwrap());
            assert!(matches!(spot, Ok(Some(p)) if (p[0] + 125.).abs() < 1e-3 && (p[1] + 68.426).abs() < 1e-3));
        });
        each(&[], |stage| {
            assert_eq!(nearest_ledge(stage, BATTLEFIELD, [0., 0.]), Ok(None));
        });
    }

    struct Fault<'a> { stage: Copied<'a>, calls: usize, fail: usize }
    impl Probe for Fault<'_> {
        fn first(&mut self, from: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread> {
            self.calls += 1;
            if self.calls == self.fail { Err(Unread) } else { self.stage.first(from, to, platforms) }
        }
    }
    fn query(stage: &mut impl Probe, which: usize) -> Result<(), Unread> {
        let ledge = Ledge { at: [-79.986, 0.], facing: -1. };
        match which {
            0 => classify(stage, BATTLEFIELD, [0., -69.5]).map(|_| ()),
            1 => floor_run(stage, BATTLEFIELD, [0., 0.]).map(|_| ()),
            2 => open_air_end(stage, BATTLEFIELD, ledge).map(|_| ()),
            3 => default_spot(stage, BATTLEFIELD, ledge).map(|_| ()),
            4 => nearest_ledge(stage, BATTLEFIELD, [0., 0.]).map(|_| ()),
            _ => unreachable!(),
        }
    }

    #[test]
    fn an_unread_stage_is_never_an_answer() {
        let lines: Vec<_> = battlefield().into_iter().map(|(line, _)| line).collect();
        for which in 0..5 {
            let mut normal = Fault { stage: Copied(&lines), calls: 0, fail: usize::MAX };
            assert_eq!(query(&mut normal, which), Ok(()));
            assert!(normal.calls > 0, "query {which} must read the stage");
            for fail in 1..=normal.calls {
                let mut fault = Fault { stage: Copied(&lines), calls: 0, fail };
                assert_eq!(query(&mut fault, which), Err(Unread), "query {which}, call {fail}");
            }
        }
    }

    #[test]
    fn a_column_with_too_many_lines_is_unread() {
        let lines: Vec<_> = (1..=17).map(|height| line([[-10., height as f32], [10., height as f32]], false, false)).collect();
        assert_eq!(classify(&mut Copied(&lines), [-20., 20., 40., -5.], [0., 0.]), Err(Unread));
        assert_eq!(classify(&mut Copied(&lines[..16]), [-20., 20., 40., -5.], [0., 0.]), Ok(Place::Under));
    }

    #[test]
    fn place_off_stage_takes_under_only_when_allowed() {
        assert!(Place::Open.off_stage(false));
        assert!(Place::Open.off_stage(true));
        assert!(!Place::Under.off_stage(false));
        assert!(Place::Under.off_stage(true));
        for place in [Place::Outside, Place::Solid, Place::OverStage] {
            assert!(!place.off_stage(false));
            assert!(!place.off_stage(true));
        }
    }
    }
}

#[cfg(test)]
#[path = "rule_tests.rs"]
mod rule_tests;
