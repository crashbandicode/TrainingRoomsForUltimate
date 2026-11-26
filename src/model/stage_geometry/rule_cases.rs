    lib_unit_tests! {
    use super::*;

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

    fn run_ends(stage: &mut impl Probe, at: Point) -> Result<Option<Segment>, Unread> {
        floor_run(stage, BATTLEFIELD, at).map(|run| run.map(|run| run.ends))
    }

    /// The walk under `floor_run`: joined solid floors are followed to the last end from any of them, a gap of one
    /// float step ends the run, and a run too long to walk is unread, not cut short.
    #[test]
    fn a_floor_run_follows_joined_floors_to_the_last_end() {
        struct Same(Met, f32);
        impl Probe for Same {
            fn first(&mut self, _: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread> {
                assert!(!platforms && to == self.1, "the walk reads solid lines down to the bottom blast line");
                Ok(Some(self.0))
            }
        }
        let joined = [
            (line([[-10., 0.], [10., 0.]], true, false), true),
            (line([[-30., 0.], [-10., 0.]], true, false), true),
            (line([[10., 0.], [30., 10.]], true, false), true),
        ];
        each(&joined, |stage| for x in [-20., 0., 20.] {
            assert_eq!(run_ends(stage, [x, 12.]), Ok(Some([[-30., 0.], [30., 10.]])), "from {x}");
        });
        let gap = [joined[0], (line([[10_f32.next_up(), 0.], [30., 10.]], true, false), true)];
        each(&gap, |stage| assert_eq!(run_ends(stage, [0., 12.]), Ok(Some([[-10., 0.], [10., 0.]]))));
        each(&[], |stage| assert_eq!(run_ends(stage, [0., 12.]), Ok(None)));
        // The same floor met again is the end of the run, not a step.
        let (floor, bottom) = (joined[0].0, BATTLEFIELD[3]);
        assert_eq!(run_end(&mut Same(floor, bottom), bottom, floor, true), Ok([10., 0.]));
        assert_eq!(run_end(&mut Same(floor, bottom), bottom, floor, false), Ok([-10., 0.]));
        let chain = |count: usize| -> Vec<(Met, bool)> {
            (0..count).map(|i| (line([[i as f32 * 2., 0.], [i as f32 * 2. + 2., 0.]], true, false), true)).collect()
        };
        each(&chain(MAX_LINES), |stage| {
            assert_eq!(run_ends(stage, [1., 0.]), Ok(Some([[0., 0.], [MAX_LINES as f32 * 2., 0.]])));
        });
        each(&chain(MAX_LINES + 1), |stage| assert_eq!(run_ends(stage, [1., 0.]), Err(Unread)));
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
    fn nearest_ledge_counts_the_near_end_when_entering_a_run() {
        let mut lines = training();
        lines.push((line([[-115., 25.], [-95., 25.]], true, true), true));
        each(&lines, |stage| {
            let at = [-100., 40.];
            assert_eq!(classify(stage, TRAINING, at), Ok(Place::OverStage));
            assert_eq!(nearest_ledge(stage, TRAINING, at), Ok(ledge([-80., 0.], -1.)));
            assert_eq!(nearest_ledge(stage, TRAINING, [-160., 40.]), Ok(ledge([-170., 0.014], 1.)));
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

    /// The Training stage's collision lines as the stage file has them (training-offstage's fixture rows, copied):
    /// the two ends, the stored normal's y, pass-through. `stand` is taken from the normal as that fixture takes it.
    const STAGE_FILE_BLAST: [f32; 4] = [-589.999_8, 500.0, 360.00015, -137.85207];
    const STAGE_FILE_ROWS: [(Segment, f32, bool); 22] = [
        ([[620.0, 0.0], [-80.0, 0.0]], 1.0, false),
        ([[-80.0, 0.0], [-80.0, -340.0]], 0.0, false),
        ([[-80.0, -340.0], [620.0, -340.0]], -1.0, false),
        ([[620.0, -340.0], [620.0, 0.0]], 0.0, false),
        ([[-635.0, 47.391], [-490.0, 47.391]], 1.0, false),
        ([[-490.0, 47.391], [-490.0, 0.0]], 0.0, false),
        ([[-490.0, 0.0], [-420.0, 0.0]], 1.0, false),
        ([[-420.0, 0.0], [-420.0, -165.0]], 0.0, false),
        ([[-420.0, -165.0], [-635.0, -165.0]], -1.0, false),
        ([[-635.0, -165.0], [-635.0, 47.391]], 0.0, false),
        ([[-309.422, 24.119], [-274.789, 24.119]], 1.0, true),
        ([[-267.316, 47.189], [-232.684, 47.189]], 1.0, true),
        ([[-225.211, 24.119], [-190.578, 24.119]], 1.0, true),
        ([[-329.986, -8.208], [-330.0, 0.014]], -0.002, false),
        ([[-330.0, 0.014], [-249.921, 0.014]], 1.0, false),
        ([[-249.921, 0.014], [-170.0, 0.014]], 1.0, false),
        ([[-170.0, 0.014], [-170.011, -8.208]], -0.001, false),
        ([[-170.011, -8.208], [-185.613, -24.992]], -0.681, false),
        ([[-185.613, -24.992], [-208.584, -40.007]], -0.837, false),
        ([[-208.584, -40.007], [-291.396, -39.988]], -1.0, false),
        ([[-291.396, -39.988], [-314.395, -25.0]], -0.838, false),
        ([[-314.395, -25.0], [-329.986, -8.208]], -0.68, false),
    ];
    fn stage_file_training() -> Vec<(Met, bool)> {
        STAGE_FILE_ROWS
            .iter()
            .map(|&(ends, normal_y, through)| {
                let stand = if through { normal_y > 0.0 } else { normal_y >= 0.7 };
                (line(ends, stand, through), normal_y > 0.0)
            })
            .collect()
    }

    /// The two spots Player 1 stood on in the probe boot of 2026-11-25 (dup-audit, inbox 19:08:07Z), where the probe
    /// printed nothing: on the island's floor, and on the platform over it. And the floor's own height.
    #[test]
    fn a_fighter_grounded_on_the_island_has_its_run_and_both_ledges() {
        each(&stage_file_training(), |stage| {
            for at in [[-215.88, 0.024], [-215.9, 24.1], [-215.88, 0.014]] {
                assert_eq!(
                    floor_run(stage, STAGE_FILE_BLAST, at),
                    Ok(Some(FloorRun {
                        ends: [[-330.0, 0.014], [-170.0, 0.014]],
                        ledges: [ledge([-330.0, 0.014], -1.0), ledge([-170.0, 0.014], 1.0)],
                    })),
                    "{at:?}"
                );
                assert_eq!(nearest_ledge(stage, STAGE_FILE_BLAST, at), Ok(ledge([-170.0, 0.014], 1.0)), "{at:?}");
            }
            for at in [[-215.88, 0.024], [-215.9, 24.1]] {
                assert_eq!(classify(stage, STAGE_FILE_BLAST, at), Ok(Place::OverStage), "{at:?}");
            }
        });
    }
    }
