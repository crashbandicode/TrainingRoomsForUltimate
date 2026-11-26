//! Shared pure stage geometry: placement classification, scans, outlines and ledges.
use super::edge_juggle::perfect::Outline;
pub use super::edge_juggle::perfect::{Point, Segment};
use super::tech::discovery::{Ray, Rays};
use super::tech::surface::{Kind, Line, Surfaces, MAX_LINES};
use super::tech::Vec2;

/// True when the line is a placement floor.
pub fn is_placement_floor(flags: u32, normal_y: f32, platforms: bool) -> bool {
    // Classify one-way platforms before using the solid floor bit.
    if flags & 0x2000 != 0 {
        platforms && flags & 0x0300 == 0 && normal_y > 0.0
    } else {
        flags & 0x0800 != 0
    }
}

/// One ray of the column scan Tech's platform scan and eggs' floor read both step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scan { Done, Hit(Line), Miss, Broken }

pub fn scan_step(rays: &mut Rays, ray: impl FnOnce(Ray) -> Result<Option<(Line, Vec2)>, ()>) -> Scan {
    let Some(current) = rays.current() else {
        return Scan::Done;
    };
    match ray(current) {
        Ok(Some((line, hit))) if rays.hit(line, hit).is_ok() => Scan::Hit(line),
        Ok(Some(_)) | Err(()) => Scan::Broken,
        Ok(None) => {
            rays.miss();
            Scan::Miss
        }
    }
}

/// Endpoints of the current connected solid floor, including its slope.
pub fn floor_ledges(floors: impl IntoIterator<Item = Segment>) -> Option<[Point; 2]> {
    let mut points = floors.into_iter().flatten();
    let first = points.next()?;
    let [mut left, mut right] = [first; 2];
    for point in std::iter::once(first).chain(points) {
        if !point.into_iter().all(f32::is_finite) {
            return None;
        }
        if point[0] < left[0] {
            left = point;
        }
        if point[0] > right[0] {
            right = point;
        }
    }
    (left[0] < right[0]).then_some([left, right])
}

/// A closed walk as world geometry: its segments in walk order, and the ledges of its solid floors.
pub fn outline_of(walk: &Surfaces) -> (Outline, Option<[Point; 2]>) {
    let mut outline = Outline::new();
    let ends = |line: Line| {
        line.ends.map(|point| [point.x, point.y])
    };
    for line in walk.lines() {
        outline.add(ends(line));
    }
    let floor = walk.lines().filter(|line| line.kind() == Some(Kind::Floor) && !line.passable);
    (outline, floor_ledges(floor.map(ends)))
}

// The off-stage rule (artifacts/phase23/training-stage-tracing/FINDINGS.md §6): where a point is against the stage,
// the ledges with open air, the default spot beside one. Pure: the stage is read through `Probe`, by the game's ray
// on a fighter's frame or from copied lines. No stage is named.

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
    if at[1] <= bottom {
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

fn open_ledge(stage: &mut impl Probe, blast: [f32; 4], end: Point, facing: f32) -> Result<Option<Ledge>, Unread> {
    let beside = [end[0] + facing, end[1] - 1.0];
    Ok((classify(stage, blast, beside)? == Place::Open).then_some(Ledge { at: end, facing }))
}

/// The solid floor run under `at` (the ledge walk, seeded at any floor). `None`: no solid floor under the point.
pub fn floor_run(stage: &mut impl Probe, blast: [f32; 4], at: Point) -> Result<Option<FloorRun>, Unread> {
    let bottom = blast[3];
    let Some(floor) = solid_floor_at(stage, [at[0], at[1] + SLACK], bottom)? else { return Ok(None) };
    let ends = [run_end(stage, bottom, floor, false)?, run_end(stage, bottom, floor, true)?];
    let ledges = [open_ledge(stage, blast, ends[0], -1.0)?, open_ledge(stage, blast, ends[1], 1.0)?];
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
                if best.is_some_and(|(_, nearest)| (x - at[0]) * dir >= nearest) {
                    break;
                }
                x += dir * COLUMN;
                continue;
            };
            let end = run_end(stage, bottom, floor, right_side)?;
            // The starting run is searched in both directions. In a later run
            // the ledge facing back into the gap may be the closest one.
            let near = if x == start {
                None
            } else {
                Some((run_end(stage, bottom, floor, !right_side)?, -dir))
            };
            for (point, facing) in near.into_iter().chain([(end, dir)]) {
                let distance = (point[0] - at[0]).hypot(point[1] - at[1]);
                if best.is_some_and(|(_, nearest)| distance >= nearest) {
                    continue;
                }
                if let Some(found) = open_ledge(stage, blast, point, facing)? {
                    best = Some((found, distance));
                }
            }
            // No point farther along this scan can beat the best straight-line distance.
            if best.is_some_and(|(_, nearest)| (end[0] - at[0]) * dir >= nearest) {
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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::tech::discovery::{decode, far_end};
    use crate::model::tech::surface::{Kind, Source, Walk};

    fn point([x, y]: Point) -> Vec2 { Vec2 { x, y } }

    fn line(id: usize, ends: Segment, normal: Point, flags: u32) -> Line {
        decode(id, ends.map(point), point(normal), (flags >> 8) as u8).unwrap()
    }

    struct Stage { lines: Vec<Line> }

    impl Stage {
        fn fixture() -> Self {
            Self { lines: vec![
                line(100, [[-80., 0.], [80., 0.]], [0., 1.], 0x0800),
                line(101, [[80., 0.], [80., -30.]], [1., 0.], 0x0100),
                line(102, [[80., -30.], [-80., -30.]], [0., -1.], 0),
                line(103, [[-80., -30.], [-80., 0.]], [-1., 0.], 0x0100),
            ] }
        }

        // Transcribes BootstrapStage::ray's native vertical intersection and reported-point bias.
        fn ray(&self, ray: Ray) -> Option<(Line, Vec2)> {
            assert_eq!(ray.delta.x, 0.);
            assert!(ray.delta.y < 0.);
            let mut nearest = f32::INFINITY;
            let mut chosen = None;
            for line in &self.lines {
                let dx = line.ends[1].x - line.ends[0].x;
                if dx == 0. { continue; }
                let u = (ray.origin.x - line.ends[0].x) / dx;
                let y = line.ends[0].y + u * (line.ends[1].y - line.ends[0].y);
                let t = (y - ray.origin.y) / ray.delta.y;
                if !(-1e-5..=1.00001).contains(&u) || !(-1e-5..=1.00001).contains(&t) { continue; }
                if t < nearest {
                    nearest = t;
                    let fraction = if t < 0. { 0. } else { t.min(1.) * 0.99 };
                    chosen = Some((*line, Vec2 { x: ray.origin.x, y: ray.origin.y + ray.delta.y * fraction }));
                }
            }
            chosen
        }
    }

    impl Source for Stage {
        fn read(&mut self, id: usize) -> Option<Line> {
            self.lines.iter().copied().find(|line| line.id == id)
        }

        fn next(&mut self, line: Line, end: usize) -> Option<(usize, usize)> {
            let shared = *line.ends.get(end)?;
            self.lines.iter().copied().filter(|next| next.id != line.id && !next.passable)
                .find_map(|next| far_end(shared, next).map(|far| (next.id, far)))
        }
    }

    fn manual_scan(stage: &Stage, range: [f32; 4]) -> Vec<Scan> {
        let mut rays = Rays::new(range).unwrap();
        let mut steps = Vec::new();
        while let Some(ray) = rays.current() {
            match stage.ray(ray) {
                Some((line, hit)) => {
                    rays.hit(line, hit).unwrap();
                    steps.push(Scan::Hit(line));
                }
                None => {
                    rays.miss();
                    steps.push(Scan::Miss);
                }
            }
        }
        steps
    }

    fn scanned(stage: &Stage, range: [f32; 4]) -> Vec<Scan> {
        let mut rays = Rays::new(range).unwrap();
        let mut steps = Vec::new();
        loop {
            match scan_step(&mut rays, |ray| Ok(stage.ray(ray))) {
                Scan::Done => break,
                Scan::Broken => { steps.push(Scan::Broken); break; }
                step => steps.push(step),
            }
        }
        steps
    }

    fn eggs(steps: &[Scan]) -> Vec<Segment> {
        let mut floors = Vec::new();
        for line in steps.iter().filter_map(|step| match step {
            Scan::Hit(line) if line.kind() == Some(Kind::Floor) => Some(line),
            _ => None,
        }) {
            let ends = line.ends.map(|p| [p.x, p.y]);
            if floors.len() < 24 && !floors.contains(&ends) { floors.push(ends); }
        }
        floors
    }

    #[test]
    fn placement_rule_reads_the_bits_it_always_did() {
        for (flags, normal_y, platforms, expected) in [
            (0x0800, 1., false, true), (0x0800, 1., true, true), (0x0800, -1., false, true),
            (0x0800, -1., true, true), (0, 1., false, false), (0, 1., true, false),
            (0x2000, 1., false, false), (0x2000, 1., true, true),
            (0x2000, 0., true, false), (0x2000, -1., true, false),
            (0x2100, 1., true, false), (0x2200, 1., true, false),
            (0x2800, -1., true, false), (0x0900, -1., false, true),
            (0x0100, 1., false, false),
        ] {
            assert_eq!(is_placement_floor(flags, normal_y, platforms), expected, "flags={flags:#x} y={normal_y} platforms={platforms}");
        }
    }

    #[test]
    fn the_two_rules_differ_where_they_always_did() {
        for (name, flags, normal, platforms, tech, placement) in [
            ("normal up", 0, [0., 1.], false, Some(Kind::Floor), false),
            ("normal 0.8", 0, [0., 0.8], false, Some(Kind::Floor), false),
            ("platforms declined", 0x2000, [0., 1.], false, Some(Kind::Floor), false),
            ("solid downward", 0x0800, [0., -1.], false, Some(Kind::Floor), true),
            ("passable wall", 0x2100, [0., 1.], true, None, false),
            ("wall", 0x0100, [1., 0.], false, Some(Kind::Wall), false),
            ("ceiling", 0, [0., -1.], false, Some(Kind::Ceiling), false),
            ("platform allowed", 0x2000, [0., 1.], true, Some(Kind::Floor), true),
        ] {
            let actual = decode(1, [point([-1., 0.]), point([1., 0.])], point(normal), (flags >> 8) as u8)
                .and_then(|line| line.kind());
            assert_eq!(actual, tech, "{name}: Tech");
            assert_eq!(is_placement_floor(flags, normal[1], platforms), placement, "{name}: placement");
        }
    }

    #[test]
    fn the_first_obstacle_is_kept_not_skipped() {
        for platforms in [false, true] {
            assert!(!is_placement_floor(0x0100, 0., platforms));
            assert!(!is_placement_floor(0, -1., platforms));
        }
    }

    #[test]
    fn ledges_span_the_connected_solid_floor() {
        let tilted = [
            [[-32.0, 1.0], [-80.0, -18.0]],
            [[-32.0, 1.0], [32.0, 5.0]],
            [[80.0, -2.0], [32.0, 5.0]],
        ];
        assert_eq!(floor_ledges(tilted), Some([[-80., -18.], [80., -2.]]));
        assert_eq!(floor_ledges([]), None);
        assert_eq!(floor_ledges([[[0., 0.], [0., 2.]]]), None);
        assert_eq!(floor_ledges([[[f32::NAN, 0.], [2., 2.]]]), None);
    }

    #[test]
    fn the_outline_fold_keeps_walk_order_and_solid_floor_ledges() {
        let mut stage = Stage::fixture();
        let mut walk = Surfaces::EMPTY;
        walk.begin(100, 0);
        for _ in 0..4 { walk.step(&mut stage); }
        assert_eq!(walk.state, Walk::Closed);
        let (outline, ledges) = outline_of(&walk);
        let expected: Vec<_> = walk.lines().map(|line| line.ends.map(|p| [p.x, p.y])).collect();
        assert_eq!(outline.segments(), expected.as_slice());
        assert_eq!(ledges, Some([[-80., 0.], [80., 0.]]));
        let platform = line(104, [[-100., 24.], [100., 24.]], [0., 1.], 0x2000);
        assert!(walk.push(platform));
        let (outline, ledges) = outline_of(&walk);
        let expected: Vec<_> = walk.lines().map(|line| line.ends.map(|p| [p.x, p.y])).collect();
        assert_eq!(outline.segments(), expected.as_slice());
        assert_eq!(ledges, Some([[-80., 0.], [80., 0.]]));
    }

    #[test]
    fn one_scan_step_serves_tech_and_eggs() {
        let stage = Stage { lines: vec![
            line(1, [[-50., 0.], [50., 0.]], [0., 1.], 0x0800),
            line(2, [[-50., 20.], [50., 20.]], [0., 1.], 0x2000),
            line(3, [[-50., 35.], [50., 35.]], [0., 1.], 0x2000),
            line(4, [[-50., 10.], [50., 10.]], [1., 0.], 0x0100),
        ] };
        let range = [0., 10., 50., -10.];
        let steps = scanned(&stage, range);
        assert_eq!(steps, manual_scan(&stage, range));
        let ids: Vec<_> = steps.iter().map(|step| match step {
            Scan::Hit(line) => Some(line.id), Scan::Miss => None, _ => panic!("unexpected scan state"),
        }).collect();
        assert_eq!(ids, [Some(3), Some(2), Some(4), Some(1), None, Some(3), Some(2), Some(4), Some(1), None]);
        let mut tech = Surfaces::EMPTY;
        for line in steps.iter().filter_map(|step| match step {
            Scan::Hit(line) if line.passable && line.kind().is_some() => Some(*line),
            _ => None,
        }) {
            assert!(tech.push(line));
        }
        assert_eq!(tech.lines().map(|line| line.id).collect::<Vec<_>>(), [3, 2]);
        assert_eq!(eggs(&steps), [
            [[-50., 35.], [50., 35.]], [[-50., 20.], [50., 20.]], [[-50., 0.], [50., 0.]],
        ]);
        let mut bad = Rays::new(range).unwrap();
        assert_eq!(scan_step(&mut bad, |ray| Ok(Some((stage.lines[2], Vec2 { x: ray.origin.x + 1., y: ray.origin.y })))),
                   Scan::Broken);
        assert!(bad.current().is_none());
        let mut failed = Rays::new(range).unwrap();
        assert_eq!(scan_step(&mut failed, |_| Err(())), Scan::Broken);
    }

    #[test]
    fn stacked_platforms_all_reach_the_floor_read() {
        let stage = Stage { lines: vec![
            line(1, [[-50., 0.], [50., 0.]], [0., 1.], 0x0800),
            line(2, [[-50., 30.], [50., 30.]], [0., 1.], 0x2000),
            line(3, [[-50., 55.], [50., 55.]], [0., 1.], 0x2000),
        ] };
        assert_eq!(eggs(&scanned(&stage, [0., 0., 70., -10.])), [
            [[-50., 55.], [50., 55.]], [[-50., 30.], [50., 30.]], [[-50., 0.], [50., 0.]],
        ]);
    }

    #[test]
    fn a_narrow_line_between_columns_is_seen_by_the_ray_not_the_scan() {
        let stage = Stage { lines: vec![
            line(1, [[-50., 0.], [50., 0.]], [0., 1.], 0x0800),
            line(2, [[4., 8.], [6., 8.]], [0., 1.], 0x2000),
        ] };
        let ray = |y| Ray { origin: point([5., y]), delta: point([0., -50. - y]) };
        assert_eq!(stage.ray(ray(12.)).unwrap().0.id, 2);
        assert_eq!(stage.ray(ray(7.)).unwrap().0.id, 1);
        assert_eq!(eggs(&scanned(&stage, [0., 10., 12., -50.])), [[[-50., 0.], [50., 0.]]]);
    }

    #[test]
    fn the_ray_is_cast_in_one_place() {
        let geometry = include_str!("../runtime/features/stage_geometry.rs");
        assert_eq!(geometry.lines().filter(|line| line.contains("transmute") && line.contains("FLOOR_RAY")).count(), 1);
        assert_eq!(geometry.matches("struct RayPoint").count(), 1);
        assert_eq!(geometry.matches("fn cast(").count(), 1);
        assert!(!geometry.contains("(self.cast)("));
        assert_eq!(geometry.matches("native_ray(").count(), 2);

        let tech_surfaces = include_str!("../runtime/features/tech_surfaces.rs");
        assert_eq!(tech_surfaces.matches("native_ray(").count(), 1);
        assert_eq!(tech_surfaces.matches("native_ray(").count(), tech_surfaces.matches("cast(native_ray(").count());
        let users = [
            include_str!("../runtime/features/mashout.rs"),
            include_str!("../runtime/features/tech.rs"),
            include_str!("../runtime/features/edge_juggle.rs"),
            include_str!("../runtime/features/egg_zone.rs"),
            include_str!("../runtime/features/placement.rs"),
            include_str!("../runtime/features/hitbox.rs"),
            include_str!("../runtime/features/record_place.rs"),
        ];
        for source in users {
            assert!(!source.contains("native_ray"));
            assert!(!source.contains("RayCast"));
        }
        // tech_spike.rs is diag-only research; tech_surfaces_guards.rs holds guard data.
        for source in users.into_iter().chain([tech_surfaces]) {
            assert!(!source.contains("0x202a240"));
            assert!(!source.lines().any(|line| line.contains("transmute") && line.contains("FLOOR_RAY")));
            assert!(!source.contains("struct RayPoint"));
            assert!(!source.contains("struct NativePoint"));
            assert!(!source.contains("type RayCast"));
        }
    }

    #[test]
    fn every_user_reads_the_stage_through_the_module() {
        let placement = include_str!("../runtime/features/placement.rs");
        let edge = include_str!("../runtime/features/edge_juggle.rs");
        let eggs = include_str!("../runtime/features/egg_zone.rs");
        let mashout = include_str!("../runtime/features/mashout.rs");
        let tech = include_str!("../runtime/features/tech_surfaces.rs");
        for source in [placement, edge, eggs, mashout] {
            assert!(source.contains("stage_geometry::"));
        }
        assert!(tech.contains("stage_geometry"));
        assert!(tech.contains("native_ray"));
        assert!(tech.contains("scan_step"));
        for source in [placement, edge, eggs, mashout, tech] {
            assert!(!source.contains("struct FloorRay"));
        }
        assert!(!edge.contains("unsafe fn ledge("));
        assert!(!edge.contains("fn stage_floor("));
        assert!(!tech.contains("fn training_outline("));
        assert!(!tech.contains("FrameOutlineReader"));
    }

    #[test]
    fn no_stage_is_named() {
        for source in [
            include_str!("stage_geometry.rs"),
            include_str!("../runtime/features/stage_geometry.rs"),
        ] {
            let code = source.split("#[cfg(test)]").next().unwrap();
            for name in ["stage_id", "StageID", "stage_key", "lobby_stages", "fighter_kind"] {
                assert!(!code.contains(name), "{name}");
            }
        }
    }
    }
}

#[cfg(test)]
#[path = "stage_geometry/rule_cases.rs"]
mod rule_cases;
#[cfg(test)]
#[path = "stage_geometry/rule_tests.rs"]
mod rule_tests;
