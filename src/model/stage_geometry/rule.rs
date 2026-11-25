//! The off-stage rule (artifacts/phase23/training-stage-tracing/FINDINGS.md §6): where a point is against the stage,
//! the ledges with open air, the default spot beside one. Pure: the stage is read through `Probe`, by the game's ray
//! on a fighter's frame or from copied lines. No stage is named.
use super::{Point, Segment};
use crate::model::tech::surface::Line;

/// One stage line as world geometry. No native id survives the read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Met {
    pub ends: Segment,
    /// A fighter can stand on it: placement's floor rule with platforms.
    pub stand: bool,
    /// A pass-through platform. It is never part of a solid.
    pub through: bool,
}
impl Met {
    /// The line's height at `x` (its ends' line; `None` for a vertical line).
    pub fn height(self, _x: f32) -> Option<f32> { None }
    /// `stand && !through`.
    pub fn solid_floor(self) -> bool { false }
}

/// The stage could not be read (a guard, a line that is gone).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unread;

/// The stage on this frame, read by the point ray.
pub trait Probe {
    /// The first line of any kind from `from` straight to the height `to`.
    fn first(&mut self, from: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread>;
}

/// The box is this far inside every blast line.
pub const INSIDE: f32 = 1.0;
/// A point up to this far under a floor's surface stands on it.
pub const SLACK: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Not `INSIDE` every blast line.
    Outside,
    /// In a solid.
    Solid,
    /// A line a fighter can stand on lies under the point.
    OverStage,
    /// Off stage, no solid above the point.
    Open,
    /// Off stage, a solid above the point.
    Under,
}
impl Place {
    /// `Open`, or `Under` when `under` is allowed.
    pub fn off_stage(self, _under: bool) -> bool { false }
}

/// Classify a point against the stage and blast lines.
pub fn classify(_stage: &mut impl Probe, _blast: [f32; 4], _at: Point) -> Result<Place, Unread> {
    Ok(Place::Outside)
}

/// An end of a solid floor run with open air beside it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ledge {
    pub at: Point,
    /// Where the open air is: `-1.0` left, `1.0` right.
    pub facing: f32,
}

/// A run of solid floors joined end to end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorRun {
    /// Its left end and its right end.
    pub ends: Segment,
    /// The ledge at each end, when it has open air beside it.
    pub ledges: [Option<Ledge>; 2],
}

/// The solid floor run under `at`.
pub fn floor_run(_stage: &mut impl Probe, _blast: [f32; 4], _at: Point) -> Result<Option<FloorRun>, Unread> { Ok(None) }
/// Where the open air beside a ledge ends.
pub fn open_air_end(_stage: &mut impl Probe, _blast: [f32; 4], _ledge: Ledge) -> Result<f32, Unread> { Ok(f32::NAN) }
/// The default spot beside a ledge.
pub fn default_spot(_stage: &mut impl Probe, _blast: [f32; 4], _ledge: Ledge) -> Result<Option<Point>, Unread> { Ok(None) }
/// The nearest ledge with open air to a point.
pub fn nearest_ledge(_stage: &mut impl Probe, _blast: [f32; 4], _at: Point) -> Result<Option<Ledge>, Unread> { Ok(None) }

/// The same queries over copied lines; pure, any thread.
pub struct Copied<'a>(pub &'a [Met]);
impl Probe for Copied<'_> {
    fn first(&mut self, _from: Point, _to: f32, _platforms: bool) -> Result<Option<Met>, Unread> { Ok(None) }
}

/// A traced line as the rule sees it.
pub fn met(_line: Line) -> Met { Met { ends: [[0.0; 2]; 2], stand: false, through: false } }

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
