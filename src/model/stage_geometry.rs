//! Shared pure stage geometry: placement classification, scans, outlines and ledges.
use super::edge_juggle::perfect::{Outline, Point, Segment};
use super::mashout::entry::PlacementFloor;
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

fn ledge(
    ray: &mut impl FnMut(f32, [f32; 2], bool) -> Option<PlacementFloor>,
    mut floor: PlacementFloor,
    right: bool,
    limits: [f32; 2],
) -> Option<Point> {
    let dir = if right { 1.0 } else { -1.0 };
    for _ in 0..MAX_LINES {
        let [a, b] = floor.ends;
        let end = if (a[0] < b[0]) == right { b } else { a };
        let Some(next) = ray(end[0] + dir, limits, false) else {
            return Some(end);
        };
        if !next.ends.contains(&end) || next.ends == floor.ends {
            return Some(end);
        }
        floor = next;
    }
    None
}

/// The main floor under the camera's middle and its two ledges.
pub fn main_floor(
    mut ray: impl FnMut(f32, [f32; 2], bool) -> Option<PlacementFloor>,
    [left, right, top, bottom]: [f32; 4],
) -> Option<(Segment, Point)> {
    let x = left.midpoint(right);
    let limits = [top, bottom];
    let floor = ray(x, limits, false)?;
    let ledges = [
        ledge(&mut ray, floor, false, limits)?,
        ledge(&mut ray, floor, true, limits)?,
    ];
    Some((ledges, [x, floor.height(x)?]))
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::tech::discovery::{decode, far_end};
    use crate::model::tech::surface::{Kind, Source, Walk, MAX_LINES};

    fn point([x, y]: Point) -> Vec2 { Vec2 { x, y } }

    fn line(id: usize, ends: Segment, normal: Point, flags: u32) -> Line {
        decode(id, ends.map(point), point(normal), (flags >> 8) as u8).unwrap()
    }

    fn floor(ends: Segment) -> PlacementFloor {
        PlacementFloor { ends, stationary: true, flags: 0x0800 }
    }

    fn floor_at(floors: &[PlacementFloor], x: f32) -> Option<PlacementFloor> {
        floors.iter().copied().find(|floor| {
            let [a, b] = floor.ends.map(|end| end[0]);
            (a.min(b)..=a.max(b)).contains(&x)
        })
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
        let main = floor([[-10., 0.], [10., 0.]]);
        let lower = floor([[10., -5.], [30., -5.]]);
        assert!(floor_at(&[lower], 11.).is_some(), "a lower floor exists past the wall");
        let result = main_floor(|x, _, _| if x > 10. { None } else { floor_at(&[main], x) }, [-10., 10., 50., -50.]);
        assert_eq!(result, Some(([[-10., 0.], [10., 0.]], [0., 0.])));
    }

    #[test]
    fn ledges_follow_joined_floors_to_the_last_end() {
        let joined = [
            floor([[-10., 0.], [10., 0.]]),
            floor([[-30., 0.], [-10., 0.]]),
            floor([[10., 0.], [30., 10.]]),
        ];
        let camera = [-10., 10., 50., -50.];
        assert_eq!(main_floor(|x, limits, platforms| {
            assert_eq!(limits, [50., -50.]);
            assert!(!platforms);
            floor_at(&joined, x)
        }, camera), Some(([[-30., 0.], [30., 10.]], [0., 0.])));
        let gap = [joined[0], floor([[10_f32.next_up(), 0.], [30., 10.]])];
        assert_eq!(main_floor(|x, _, _| floor_at(&gap, x), camera),
                   Some(([[-10., 0.], [10., 0.]], [0., 0.])));
        assert_eq!(main_floor(|_, _, _| Some(joined[0]), camera),
                   Some(([[-10., 0.], [10., 0.]], [0., 0.])));
        let chain: Vec<_> = (0..=MAX_LINES).map(|i| {
            let x = i as f32 * 2.;
            floor([[x, 0.], [x + 2., 0.]])
        }).collect();
        assert_eq!(main_floor(|x, _, _| floor_at(&chain, x), [0., 2., 50., -50.]), None);
        assert_eq!(main_floor(|_, _, _| None, camera), None);
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
