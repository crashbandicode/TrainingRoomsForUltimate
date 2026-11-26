//! The off-stage rule (`artifacts/phase23/stage-geometry/RULE-API.md`), expected red until `rule.rs` exists.
//! Fixtures are the collision lines of the stock stage files (Training, Battlefield), read by lane
//! training-offstage (`artifacts/phase23/training-stage-tracing/data/*.json`); expected values are its replay's
//! (`data/*-readers.txt`). The Training floors are also the game's own `[Eggs] floors n=8` line (Ally run
//! `tso-3fd708bc-ally-1`). Every case runs with a ray that meets a line from either side and with one that meets
//! only its front, because the native ray's facing rule is not established.

lib_unit_tests! {
use super::*;

/// ax, ay, bx, by, normal x, normal y, pass-through.
type Row = (f32, f32, f32, f32, f32, f32, bool);

const TRAINING_BLAST: [f32; 4] = [-589.999_8, 500.0, 360.00015, -137.85207];
const TRAINING: &[Row] = &[
    // The long floor (COL_curve1).
    (620.0, 0.0, -80.0, 0.0, 0.0, 1.0, false),
    (-80.0, 0.0, -80.0, -340.0, -1.0, 0.0, false),
    (-80.0, -340.0, 620.0, -340.0, 0.0, -1.0, false),
    (620.0, -340.0, 620.0, 0.0, 1.0, 0.0, false),
    // The left block (COL_curve36).
    (-635.0, 47.391, -490.0, 47.391, 0.0, 1.0, false),
    (-490.0, 47.391, -490.0, 0.0, 1.0, 0.0, false),
    (-490.0, 0.0, -420.0, 0.0, 0.0, 1.0, false),
    (-420.0, 0.0, -420.0, -165.0, 1.0, 0.0, false),
    (-420.0, -165.0, -635.0, -165.0, 0.0, -1.0, false),
    (-635.0, -165.0, -635.0, 47.391, -1.0, 0.0, false),
    // The three platforms.
    (-309.422, 24.119, -274.789, 24.119, 0.0, 1.0, true),
    (-267.316, 47.189, -232.684, 47.189, 0.0, 1.0, true),
    (-225.211, 24.119, -190.578, 24.119, 0.0, 1.0, true),
    // The island (COL_curve35).
    (-329.986, -8.208, -330.0, 0.014, -1.0, -0.002, false),
    (-330.0, 0.014, -249.921, 0.014, 0.0, 1.0, false),
    (-249.921, 0.014, -170.0, 0.014, 0.0, 1.0, false),
    (-170.0, 0.014, -170.011, -8.208, 1.0, -0.001, false),
    (-170.011, -8.208, -185.613, -24.992, 0.732, -0.681, false),
    (-185.613, -24.992, -208.584, -40.007, 0.547, -0.837, false),
    (-208.584, -40.007, -291.396, -39.988, 0.0, -1.0, false),
    (-291.396, -39.988, -314.395, -25.0, -0.546, -0.838, false),
    (-314.395, -25.0, -329.986, -8.208, -0.733, -0.68, false),
];

const BATTLEFIELD_BLAST: [f32; 4] = [-240.0, 240.0, 192.00009, -140.00003];
const BATTLEFIELD: &[Row] = &[
    (41.416, -40.118, -41.396, -40.099, 0.0, -1.0, false),
    (-41.396, -40.099, -64.395, -25.111, -0.546, -0.838, false),
    (-64.395, -25.111, -79.986, -8.319, -0.733, -0.68, false),
    (-79.986, -8.319, -79.986, 0.0, -1.0, 0.0, false),
    (-79.986, 0.0, -74.986, 0.0, 0.0, 1.0, false),
    (-74.986, 0.0, 74.989, 0.0, 0.0, 1.0, false),
    (74.989, 0.0, 79.989, 0.0, 0.0, 1.0, false),
    (79.989, 0.0, 79.989, -8.319, 1.0, 0.0, false),
    (79.989, -8.319, 64.387, -25.103, 0.732, -0.681, false),
    (64.387, -25.103, 41.416, -40.118, 0.547, -0.837, false),
    (-59.422, 24.119, -24.789, 24.119, 0.0, 1.0, true),
    (-17.316, 47.189, 17.316, 47.189, 0.0, 1.0, true),
    (24.789, 24.119, 59.422, 24.119, 0.0, 1.0, true),
];

/// A stage file's lines behind the point ray. `front_only`: a line is met only by a ray that comes at its front
/// (a floor from above, an underside from below); otherwise from either side. A wall is parallel to the ray.
struct Lines {
    rows: &'static [Row],
    front_only: bool,
}

impl Probe for Lines {
    fn first(&mut self, from: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread> {
        let down = to < from[1];
        let mut best: Option<(f32, Met)> = None;
        for &(ax, ay, bx, by, _, ny, through) in self.rows {
            if ax == bx || (through && !platforms) || !(ax.min(bx)..=ax.max(bx)).contains(&from[0]) {
                continue;
            }
            if self.front_only && (ny > 0.0) != down {
                continue;
            }
            let height = ay + (by - ay) * ((from[0] - ax) / (bx - ax));
            let reached = if down { to <= height && height <= from[1] } else { from[1] <= height && height <= to };
            let gap = (from[1] - height).abs();
            if reached && best.is_none_or(|(nearest, _)| gap < nearest) {
                // Placement's floor rule: a platform with its normal up; a solid line with the floor bit, taken
                // here from the stored normal (0.7 and up).
                let stand = if through { ny > 0.0 } else { ny >= 0.7 };
                best = Some((gap, Met { ends: [[ax, ay], [bx, by]], stand, through }));
            }
        }
        Ok(best.map(|(_, met)| met))
    }
}

/// Each stage with both rays.
fn both(rows: &'static [Row], mut case: impl FnMut(&mut Lines, &str)) {
    for front_only in [false, true] {
        case(&mut Lines { rows, front_only }, if front_only { "front only" } else { "either side" });
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.05
}

fn ledge(x: f32, y: f32, facing: f32) -> Option<Ledge> {
    Some(Ledge { at: [x, y], facing })
}

#[test]
fn training_points_are_classified_from_the_lines() {
    both(TRAINING, |stage, ray| {
        for (at, want) in [
            // The two gaps beside the island, at every height.
            ([-375.0, -10.0], Place::Open), ([-375.0, -68.4], Place::Open), ([-375.0, 200.0], Place::Open),
            ([-125.0, -10.0], Place::Open), ([-125.0, -68.4], Place::Open), ([-125.0, 100.0], Place::Open),
            // Below the island; the second is half a unit under its underside (rv-hit-streak, 17:18:41Z).
            ([-250.0, -60.0], Place::Under), ([-250.0, -40.49752], Place::Under), ([-200.0, -120.0], Place::Under),
            // Over a floor or a platform.
            ([0.0, 5.0], Place::OverStage), ([400.0, 150.0], Place::OverStage), ([-250.0, 10.0], Place::OverStage),
            ([-290.0, 30.0], Place::OverStage), ([-450.0, 20.0], Place::OverStage), ([-550.0, 60.0], Place::OverStage),
            // Up to SLACK under a floor's surface still stands on it.
            ([0.0, -0.5], Place::OverStage),
            // In each of the three solids.
            ([100.0, -50.0], Place::Solid), ([0.0, -2.0], Place::Solid), ([-250.0, -20.0], Place::Solid),
            ([-550.0, 20.0], Place::Solid), ([-450.0, -100.0], Place::Solid),
            // Not INSIDE every blast line.
            ([-589.5, 0.0], Place::Outside), ([499.5, 10.0], Place::Outside), ([0.0, 359.5], Place::Outside),
            ([-375.0, -137.0], Place::Outside),
        ] {
            assert_eq!(classify(stage, TRAINING_BLAST, at), Ok(want), "{at:?}, ray meets {ray}");
        }
    });
}

#[test]
fn battlefield_points_are_classified_as_guard_the_edge_does_today() {
    both(BATTLEFIELD, |stage, ray| {
        for (at, want) in [
            ([-160.0, -69.5], Place::Open), ([160.0, -69.5], Place::Open), ([-100.0, 50.0], Place::Open),
            ([81.0, 0.0], Place::Open), ([-81.0, 150.0], Place::Open),
            // Under the stage; the first is half a unit under its underside (rv-hit-streak, 17:18:41Z).
            ([0.0, -40.60852], Place::Under), ([0.0, -100.0], Place::Under), ([70.0, -60.0], Place::Under),
            ([0.0, 10.0], Place::OverStage), ([-40.0, 30.0], Place::OverStage), ([0.0, 100.0], Place::OverStage),
            ([0.0, -20.0], Place::Solid), ([-70.0, -10.0], Place::Solid),
            ([-239.5, 0.0], Place::Outside), ([0.0, -139.5], Place::Outside),
        ] {
            assert_eq!(classify(stage, BATTLEFIELD_BLAST, at), Ok(want), "{at:?}, ray meets {ray}");
        }
    });
}

#[test]
fn off_stage_takes_under_only_when_asked() {
    assert!(Place::Open.off_stage(false) && Place::Open.off_stage(true));
    assert!(!Place::Under.off_stage(false) && Place::Under.off_stage(true));
    for place in [Place::Outside, Place::Solid, Place::OverStage] {
        assert!(!place.off_stage(false) && !place.off_stage(true), "{place:?}");
    }
}

#[test]
fn training_floor_runs_have_the_stage_files_four_ledges() {
    both(TRAINING, |stage, ray| {
        for (at, ends, ledges) in [
            // The long floor: a ledge on the left; its right end is past the blast line.
            ([0.0, 5.0], [[-80.0, 0.0], [620.0, 0.0]], [ledge(-80.0, 0.0, -1.0), None]),
            // The island, also from above one of its platforms: both ends.
            ([-250.0, 5.0], [[-330.0, 0.014], [-170.0, 0.014]], [ledge(-330.0, 0.014, -1.0), ledge(-170.0, 0.014, 1.0)]),
            ([-290.0, 30.0], [[-330.0, 0.014], [-170.0, 0.014]], [ledge(-330.0, 0.014, -1.0), ledge(-170.0, 0.014, 1.0)]),
            // The block's low floor: its left end is against the wall.
            ([-450.0, 5.0], [[-490.0, 0.0], [-420.0, 0.0]], [None, ledge(-420.0, 0.0, 1.0)]),
            // The block's top: past the blast line on the left, a step down on the right.
            ([-550.0, 60.0], [[-635.0, 47.391], [-490.0, 47.391]], [None, None]),
        ] {
            assert_eq!(floor_run(stage, TRAINING_BLAST, at), Ok(Some(FloorRun { ends, ledges })), "{at:?}, ray meets {ray}");
        }
        for gap in [[-375.0, 5.0], [-125.0, 5.0], [-250.0, -60.0]] {
            assert_eq!(floor_run(stage, TRAINING_BLAST, gap), Ok(None), "{gap:?}, ray meets {ray}");
        }
    });
}

#[test]
fn battlefield_floor_run_joins_its_three_floors_and_has_both_ledges() {
    both(BATTLEFIELD, |stage, ray| {
        let run = FloorRun {
            ends: [[-79.986, 0.0], [79.989, 0.0]],
            ledges: [ledge(-79.986, 0.0, -1.0), ledge(79.989, 0.0, 1.0)],
        };
        for at in [[0.0, 5.0], [-77.0, 5.0], [78.0, 60.0]] {
            assert_eq!(floor_run(stage, BATTLEFIELD_BLAST, at), Ok(Some(run)), "{at:?}, ray meets {ray}");
        }
    });
}

#[test]
fn default_spots_are_the_middle_of_the_open_air_beside_a_ledge() {
    both(TRAINING, |stage, ray| {
        for (from, end, spot) in [
            (ledge(-80.0, 0.0, -1.0), -170.0, [-125.0, -68.426]),
            (ledge(-170.0, 0.014, 1.0), -80.0, [-125.0, -68.419]),
            (ledge(-330.0, 0.014, -1.0), -420.0, [-375.0, -68.419]),
            (ledge(-420.0, 0.0, 1.0), -330.0, [-375.0, -68.426]),
        ] {
            let from = from.unwrap();
            let air = open_air_end(stage, TRAINING_BLAST, from).unwrap();
            assert!(near(air, end), "open air from {from:?} ends at {air}, not {end}; ray meets {ray}");
            let got = default_spot(stage, TRAINING_BLAST, from).unwrap().expect("a spot in open air");
            assert!(near(got[0], spot[0]) && near(got[1], spot[1]), "{from:?}: {got:?}, not {spot:?}; ray meets {ray}");
            assert_eq!(classify(stage, TRAINING_BLAST, got), Ok(Place::Open), "{got:?}, ray meets {ray}");
        }
    });
    both(BATTLEFIELD, |stage, ray| {
        // Guard the Edge's default start today: halfway to the side blast line, halfway down.
        for (from, end, spot) in [
            (ledge(-79.986, 0.0, -1.0), -240.0, [-159.993, -69.5]),
            (ledge(79.989, 0.0, 1.0), 240.0, [159.9945, -69.5]),
        ] {
            let from = from.unwrap();
            assert!(near(open_air_end(stage, BATTLEFIELD_BLAST, from).unwrap(), end), "{from:?}, ray meets {ray}");
            let got = default_spot(stage, BATTLEFIELD_BLAST, from).unwrap().expect("a spot in open air");
            assert!(near(got[0], spot[0]) && near(got[1], spot[1]), "{from:?}: {got:?}, not {spot:?}; ray meets {ray}");
        }
    });
}

#[test]
fn nearest_ledge_skips_floor_ends_that_are_not_ledges() {
    both(TRAINING, |stage, ray| {
        for (at, want) in [
            // Player 1's start on the long floor: the ledge at its left end (placement's T1 A sends it to -125, -68.4).
            ([-30.0, 0.01], ledge(-80.0, 0.0, -1.0)),
            // Far along the long floor: its right end leaves through the blast line, so the left ledge.
            ([300.0, 0.01], ledge(-80.0, 0.0, -1.0)),
            // On the island, right of its middle.
            ([-240.0, 5.0], ledge(-170.0, 0.014, 1.0)),
            // On the block's top: past the step, the low floor's ledge.
            ([-550.0, 60.0], ledge(-420.0, 0.0, 1.0)),
        ] {
            assert_eq!(nearest_ledge(stage, TRAINING_BLAST, at), Ok(want), "{at:?}, ray meets {ray}");
        }
    });
    both(BATTLEFIELD, |stage, ray| {
        assert_eq!(nearest_ledge(stage, BATTLEFIELD_BLAST, [10.0, 0.0]), Ok(ledge(79.989, 0.0, 1.0)), "ray meets {ray}");
        assert_eq!(nearest_ledge(stage, BATTLEFIELD_BLAST, [-10.0, 30.0]), Ok(ledge(-79.986, 0.0, -1.0)), "ray meets {ray}");
    });
}

/// A stage that cannot be read.
struct Gone;
impl Probe for Gone {
    fn first(&mut self, _: Point, _: f32, _: bool) -> Result<Option<Met>, Unread> {
        Err(Unread)
    }
}

#[test]
fn an_unread_stage_is_an_error_not_open_air() {
    let at = [-125.0, -68.4];
    let from = Ledge { at: [-80.0, 0.0], facing: -1.0 };
    assert_eq!(classify(&mut Gone, TRAINING_BLAST, at), Err(Unread));
    assert_eq!(floor_run(&mut Gone, TRAINING_BLAST, at), Err(Unread));
    assert_eq!(open_air_end(&mut Gone, TRAINING_BLAST, from), Err(Unread));
    assert_eq!(default_spot(&mut Gone, TRAINING_BLAST, from), Err(Unread));
    assert_eq!(nearest_ledge(&mut Gone, TRAINING_BLAST, at), Err(Unread));
    // Outside the box is known without a ray.
    assert_eq!(classify(&mut Gone, TRAINING_BLAST, [600.0, 0.0]), Ok(Place::Outside));
}
}
