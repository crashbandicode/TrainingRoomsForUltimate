lib_unit_tests! {
use super::*;

type Line = (Segment, bool, bool);

// training.json (f32): camera range [left, right, top, bottom] [-509.99976, 420.0, 280.00003, -39.864185]; blast lines [-589.9998, 500.0, 360.00015, -137.85207]
const TRAINING_LINES: [(Segment, bool, bool); 22] = [
    ([[620.0, 0.0], [-80.0, 0.0]], true, false), // L1 COL_curve1 floor
    ([[-80.0, 0.0], [-80.0, -340.0]], false, false), // L2 COL_curve1 wall
    ([[-80.0, -340.0], [620.0, -340.0]], false, false), // L3 COL_curve1 ceiling
    ([[620.0, -340.0], [620.0, 0.0]], false, false), // L4 COL_curve1 wall
    ([[-635.0, 47.39081], [-490.0, 47.39081]], true, false), // L5 COL_curve36 floor
    ([[-490.0, 47.39081], [-490.0, 0.0]], false, false), // L6 COL_curve36 wall
    ([[-490.0, 0.0], [-420.0, 0.0]], true, false), // L7 COL_curve36 floor
    ([[-420.0, 0.0], [-420.0, -165.0]], false, false), // L8 COL_curve36 wall
    ([[-420.0, -165.0], [-635.0, -165.0]], false, false), // L9 COL_curve36 ceiling
    ([[-635.0, -165.0], [-635.0, 47.39081]], false, false), // L10 COL_curve36 wall
    ([[-309.422, 24.119], [-274.789, 24.119]], true, true), // L11 COL_00_Platform01_through floor
    ([[-267.3165, 47.189003], [-232.6835, 47.189003]], true, true), // L12 COL_00_Platform02_through floor
    ([[-225.211, 24.119], [-190.578, 24.119]], true, true), // L13 COL_00_Platform03_through floor
    ([[-329.9856, -8.207711], [-330.0, 0.01357623]], false, false), // L14 COL_curve35 wall
    ([[-330.0, 0.01357623], [-249.92056, 0.01357623]], true, false), // L15 COL_curve35 floor
    ([[-249.92056, 0.01357623], [-170.0, 0.01357623]], true, false), // L16 COL_curve35 floor
    ([[-170.0, 0.01357623], [-170.0107, -8.207711]], false, false), // L17 COL_curve35 wall
    ([[-170.0107, -8.207711], [-185.61272, -24.992105]], false, false), // L18 COL_curve35 wall
    ([[-185.61272, -24.992105], [-208.58385, -40.00707]], false, false), // L19 COL_curve35 ceiling
    ([[-208.58385, -40.00707], [-291.3962, -39.987976]], false, false), // L20 COL_curve35 ceiling
    ([[-291.3962, -39.987976], [-314.3947, -25.000002]], false, false), // L21 COL_curve35 ceiling
    ([[-314.3947, -25.000002], [-329.9856, -8.207711]], false, false), // L22 COL_curve35 wall
];

// battlefield.json (f32): camera range [left, right, top, bottom] [-170.0, 170.0, 130.0, -80.0]; blast lines [-240.0, 240.0, 192.00009, -140.00003]
const BATTLEFIELD_LINES: [(Segment, bool, bool); 13] = [
    ([[41.416157, -40.11807], [-41.3962, -40.098976]], false, false), // L1 COL_00_Floor01 ceiling
    ([[-41.3962, -40.098976], [-64.39471, -25.111]], false, false), // L2 COL_00_Floor01 ceiling
    ([[-64.39471, -25.111], [-79.9856, -8.318714]], false, false), // L3 COL_00_Floor01 wall
    ([[-79.9856, -8.318714], [-79.9856, 0.00025950794]], false, false), // L4 COL_00_Floor01 wall
    ([[-79.9856, 0.00025950794], [-74.9856, 0.00025950794]], true, false), // L5 COL_00_Floor01 floor
    ([[-74.9856, 0.00025950794], [74.9893, 0.00025950794]], true, false), // L6 COL_00_Floor01 floor
    ([[74.9893, 0.00025950794], [79.9893, 0.00025950794]], true, false), // L7 COL_00_Floor01 floor
    ([[79.9893, 0.00025950794], [79.9893, -8.318714]], false, false), // L8 COL_00_Floor01 wall
    ([[79.9893, -8.318714], [64.38729, -25.103106]], false, false), // L9 COL_00_Floor01 wall
    ([[64.38729, -25.103106], [41.416157, -40.11807]], false, false), // L10 COL_00_Floor01 ceiling
    ([[-59.422, 24.119], [-24.789, 24.119]], true, true), // L11 COL_00_Platform01_through floor
    ([[-17.316496, 47.189003], [17.316496, 47.189003]], true, true), // L12 COL_00_Platform02_through floor
    ([[24.789, 24.119], [59.42199, 24.119]], true, true), // L13 COL_00_Platform03_through floor
];

// Recorded on a game: ib-practice-publisher/combined-yoga-main-1/out/bridge-run/bridge.log (zone.stage=3).
const STAGE_THREE: [([u8; 4], &str, &str, &str); 26] = [
    ([128, 128, 255, 128], "-63.00,0.00", "-133.19,-79.99", "79.99,133.19"), // bridge.log:15470
    ([140, 128, 255, 128], "-63.00,16.25", "-133.19,-79.99", "79.99,133.19"), // bridge.log:24235
    ([248, 128, 255, 128], "-63.00,162.52", "-133.19,-79.99", "79.99,133.19"), // bridge.log:24277
    ([244, 128, 255, 128], "-63.00,157.10", "-133.19,-79.99", "79.99,133.19"), // bridge.log:24445
    ([188, 128, 255, 128], "-63.00,81.26", "-133.19,-79.99", "79.99,133.19"), // bridge.log:24487
    ([188, 148, 255, 128], "-27.25,81.26", "-133.19,-79.99", "79.99,133.19"), // bridge.log:24825
    ([188, 204, 255, 128], "72.84,81.26", "-133.19,-79.99", "79.99,133.19"), // bridge.log:24867
    ([188, 192, 255, 128], "51.39,81.26", "-133.19,-79.99", "79.99,133.19"), // bridge.log:25035
    ([188, 144, 255, 128], "-34.40,81.26", "-133.19,-79.99", "79.99,133.19"), // bridge.log:25077
    ([188, 144, 135, 128], "-34.40,81.26", "-133.19,-61.17", "61.17,133.19"), // bridge.log:25456
    ([188, 144, 151, 128], "-34.40,81.26", "-133.19,-63.67", "63.68,133.19"), // bridge.log:25624
    ([188, 144, 195, 128], "-34.40,81.26", "-133.19,-70.58", "70.58,133.19"), // bridge.log:25666
    ([188, 144, 195, 112], "-34.40,81.26", "-127.54,-70.58", "70.58,127.54"), // bridge.log:26003
    ([188, 144, 195, 8], "-34.40,81.26", "-90.81,-70.58", "70.58,90.81"), // bridge.log:26045
    ([188, 144, 195, 24], "-34.40,81.26", "-96.46,-70.58", "70.58,96.47"), // bridge.log:26213
    ([188, 144, 195, 68], "-34.40,81.26", "-112.00,-70.58", "70.58,112.00"), // bridge.log:26255
    ([144, 128, 255, 128], "-63.00,21.67", "-133.19,-79.99", "79.99,133.19"), // bridge.log:33317
    ([255, 128, 255, 128], "-63.00,172.00", "-133.19,-79.99", "79.99,133.19"), // bridge.log:33359
    ([255, 120, 255, 128], "-66.31,172.00", "-133.19,-79.99", "79.99,133.19"), // bridge.log:33700
    ([255, 0, 255, 128], "-116.00,172.00", "-133.19,-79.99", "79.99,133.19"), // bridge.log:33743
    ([255, 0, 0, 128], "-116.00,172.00", "-133.19,-39.99", "40.00,133.19"), // bridge.log:34126
    ([255, 0, 12, 128], "-116.00,172.00", "-133.19,-41.87", "41.88,133.19"), // bridge.log:34298
    ([255, 0, 255, 112], "-116.00,172.00", "-127.54,-79.99", "79.99,127.54"), // bridge.log:34677
    ([255, 0, 255, 0], "-116.00,172.00", "-87.99,-79.99", "79.99,87.99"), // bridge.log:34720
    ([255, 0, 255, 4], "-116.00,172.00", "-89.40,-79.99", "79.99,89.40"), // bridge.log:34892
    ([255, 0, 255, 255], "-116.00,172.00", "-232.00,-79.99", "79.99,232.00"), // bridge.log:34934
];

// The same log, zone.stage=308, with the left outer line stopped at the island's near end.
const TRAINING: [([u8; 4], &str, &str, &str); 30] = [
    ([128, 128, 255, 128], "-62.03,0.00", "-170.00,-80.00", ""), // bridge.log:56322; recorded today: -255.70,-80.00
    ([136, 128, 255, 128], "-62.03,22.17", "-170.00,-80.00", ""), // bridge.log:61181; recorded today: -255.70,-80.00
    ([248, 128, 255, 128], "-62.03,332.60", "-170.00,-80.00", ""), // bridge.log:61223; recorded today: -255.70,-80.00
    ([244, 128, 255, 128], "-62.03,321.51", "-170.00,-80.00", ""), // bridge.log:61391; recorded today: -255.70,-80.00
    ([188, 128, 255, 128], "-62.03,166.30", "-170.00,-80.00", ""), // bridge.log:61433; recorded today: -255.70,-80.00
    ([188, 132, 255, 128], "-49.24,166.30", "-170.00,-80.00", ""), // bridge.log:61770; recorded today: -255.70,-80.00
    ([188, 196, 255, 128], "155.37,166.30", "-170.00,-80.00", ""), // bridge.log:61812; recorded today: -255.70,-80.00
    ([188, 192, 255, 128], "142.58,166.30", "-170.00,-80.00", ""), // bridge.log:61981; recorded today: -255.70,-80.00
    ([188, 136, 255, 128], "-36.46,166.30", "-170.00,-80.00", ""), // bridge.log:62023; recorded today: -255.70,-80.00
    ([188, 136, 231, 128], "-36.46,166.30", "-170.00,-63.53", ""), // bridge.log:62360; recorded today: -255.70,-63.53
    ([188, 136, 135, 128], "-36.46,166.30", "-170.00,2.35", ""), // bridge.log:62402; recorded today: -255.70,2.35
    ([188, 136, 159, 128], "-36.46,166.30", "-170.00,-14.12", ""), // bridge.log:62570; recorded today: -255.70,-14.12
    ([188, 136, 203, 128], "-36.46,166.30", "-170.00,-44.31", ""), // bridge.log:62612; recorded today: -255.70,-44.31
    ([188, 136, 203, 120], "-36.46,166.30", "-170.00,-44.31", ""), // bridge.log:62949; recorded today: -245.22,-44.31
    ([188, 136, 203, 8], "-36.46,166.30", "-98.48,-44.31", ""), // bridge.log:62991
    ([188, 136, 203, 16], "-36.46,166.30", "-108.96,-44.31", ""), // bridge.log:63159
    ([188, 136, 203, 68], "-36.46,166.30", "-170.00,-44.31", ""), // bridge.log:63201; recorded today: -177.09,-44.31
    ([140, 128, 255, 128], "-62.03,33.26", "-170.00,-80.00", ""), // bridge.log:65937; recorded today: -255.70,-80.00
    ([240, 128, 255, 128], "-62.03,310.43", "-170.00,-80.00", ""), // bridge.log:66147; recorded today: -255.70,-80.00
    ([144, 128, 255, 128], "-62.03,44.35", "-170.00,-80.00", ""), // bridge.log:70263; recorded today: -255.70,-80.00
    ([255, 128, 255, 128], "-62.03,352.00", "-170.00,-80.00", ""), // bridge.log:70305; recorded today: -255.70,-80.00
    ([255, 112, 255, 128], "-68.51,352.00", "-170.00,-80.00", ""), // bridge.log:70646; recorded today: -255.70,-80.00
    ([255, 0, 255, 128], "-113.85,352.00", "-170.00,-80.00", ""), // bridge.log:70688; recorded today: -255.70,-80.00
    ([255, 0, 247, 128], "-113.85,352.00", "-170.00,-74.51", ""), // bridge.log:71030; recorded today: -255.70,-74.51
    ([255, 0, 0, 128], "-113.85,352.00", "-170.00,95.00", ""), // bridge.log:71072; recorded today: -255.70,95.00
    ([255, 0, 12, 128], "-113.85,352.00", "-170.00,86.76", ""), // bridge.log:71244; recorded today: -255.70,86.76
    ([255, 0, 255, 124], "-113.85,352.00", "-170.00,-80.00", ""), // bridge.log:71623; recorded today: -250.46,-80.00
    ([255, 0, 255, 0], "-113.85,352.00", "-88.00,-80.00", ""), // bridge.log:71665
    ([255, 0, 255, 20], "-113.85,352.00", "-114.20,-80.00", ""), // bridge.log:71838
    ([255, 0, 255, 255], "-113.85,352.00", "-170.00,-80.00", ""), // bridge.log:71880; recorded today: -582.00,-80.00
];

const TRAINING_BLAST: [f32; 4] = [-589.9998, 500.0, 360.00015, -137.85207];
const TRAINING_CAMERA: [f32; 4] = [-509.99976, 420.0, 280.00003, -39.864185];
const BATTLEFIELD_BLAST: [f32; 4] = [-240.0, 240.0, 192.00009, -140.00003];
const BATTLEFIELD_CAMERA: [f32; 4] = [-170.0, 170.0, 130.0, -80.0];

fn mets(lines: &[Line]) -> Vec<Met> {
    lines.iter().map(|&(ends, stand, through)| Met { ends, stand, through }).collect()
}

fn outline(lines: &[Line], n: usize) -> Outline {
    let mut result = Outline::new();
    for line in &lines[..n] {
        result.add(line.0);
    }
    result
}

fn read(id: i32, lines: &[Line], n: usize, blast: [f32; 4], camera: [f32; 4]) -> Stage {
    Stage::read(id, blast, camera, outline(lines, n), &mut Copied(&mets(lines))).unwrap().unwrap()
}

fn training() -> Stage {
    read(308, &TRAINING_LINES, 4, TRAINING_BLAST, TRAINING_CAMERA)
}

fn battlefield(lines: &[Line]) -> Stage {
    read(0, lines, 10, BATTLEFIELD_BLAST, BATTLEFIELD_CAMERA)
}

fn plain_battlefield() -> Stage {
    Stage::new(0, BATTLEFIELD_BLAST, [BATTLEFIELD_LINES[4].0[0], BATTLEFIELD_LINES[6].0[1]],
        outline(&BATTLEFIELD_LINES, 10), [Some(BATTLEFIELD_BLAST[0]), Some(BATTLEFIELD_BLAST[1])], &[]).unwrap()
}

fn stage_three() -> Stage {
    // The stage file's floor ends: with them rounded to -79.986 and 79.989 one recorded row differs in its last digit.
    let ledges = [BATTLEFIELD_LINES[4].0[0], BATTLEFIELD_LINES[6].0[1]];
    Stage::new(3, [-240.0, 240.0, 180.0, -140.0], ledges,
        outline(&BATTLEFIELD_LINES, 10), [Some(-240.0), Some(240.0)], &[]).unwrap()
}

fn text(span: Option<[f32; 2]>) -> String {
    span.map_or_else(String::new, |[a, b]| format!("{a:.2},{b:.2}"))
}

fn check_rows(stage: &Stage, rows: &[([u8; 4], &str, &str, &str)]) {
    for &(bins, vertical, left, right) in rows {
        let zone = stage.resolve(Some(bins));
        assert_eq!(text(Some(zone.vertical())), vertical, "{bins:?} vertical");
        assert_eq!(text(zone.horizontal(Side::Left)), left, "{bins:?} left");
        assert_eq!(text(zone.horizontal(Side::Right)), right, "{bins:?} right");
    }
}

fn points(zone: Zone, side: Side) -> Vec<Point> {
    let Some([[left, bottom], [right, top]]) = zone.rect(side) else { return Vec::new() };
    let mut result = Vec::new();
    let mut x = left;
    while x <= right {
        let mut y = bottom;
        while y <= top {
            result.push([x, y]);
            y += 1.0;
        }
        x += 1.0;
    }
    result
}

#[test]
fn stage_three_keeps_every_recorded_zone_row() {
    check_rows(&stage_three(), &STAGE_THREE);
}

#[test]
fn training_rows_keep_their_bins_and_stop_at_the_island() {
    check_rows(&training(), &TRAINING);
}

#[test]
fn training_read_finds_the_gap_beside_the_long_floor() {
    let stage = training();
    let ledges = [[-80.0, 0.0], [620.0, 0.0]];
    assert_eq!(stage.ledges(), ledges);
    let zone = stage.resolve(None);
    assert_eq!(zone.horizontal(Side::Left), Some([-170.0, -80.0]));
    assert_eq!(zone.horizontal(Side::Right), None);
    assert_eq!(text(Some(zone.vertical())), "-62.03,0.00");
    let plain = Stage::new(308, TRAINING_BLAST, ledges, outline(&TRAINING_LINES, 4),
        [Some(-170.0), None], &[]).unwrap();
    for bins in [None, Some([255, 0, 0, 255]), Some([188, 136, 203, 8])] {
        assert_eq!(stage.resolve(bins), plain.resolve(bins));
    }
}

#[test]
fn training_targets_stay_out_of_the_island() {
    let stage = training();
    let zone = stage.resolve(None);
    let mut count = 0;
    for i in 0..168 {
        for j in 0..55 {
            let at = [-251.70 + i as f32, -58.03 + j as f32];
            if stage.contains(&zone, Side::Left, at) {
                count += 1;
                assert!(at[0] >= -166.0, "{at:?}");
            }
        }
    }
    assert_eq!(count, 4510);
    for at in [[-200.0, -30.0], [-250.0, -50.0]] {
        assert!(!stage.contains(&zone, Side::Left, at), "{at:?}");
    }
    assert!(stage.contains(&zone, Side::Left, [-125.0, -30.0]));
    let tall = stage.resolve(Some([255, 128, 255, 128]));
    assert!(!stage.contains(&tall, Side::Left, [-250.0, 60.0]));
    assert!(stage.contains(&tall, Side::Left, [-125.0, 60.0]));
    assert!(stage.allows_move(&zone, Side::Left, [-125.0, -30.0], [-120.0, -30.0]));
    assert!(!stage.allows_move(&zone, Side::Left, [-160.0, -30.0], [-175.0, -30.0]));
}

#[test]
fn battlefield_read_keeps_both_sides_and_every_spot() {
    let stage = battlefield(&BATTLEFIELD_LINES);
    let plain = plain_battlefield();
    assert_eq!(stage.resolve(None), plain.resolve(None));
    let zone = stage.resolve(None);
    assert_eq!(text(Some(zone.vertical())), "-63.00,0.00");
    assert_eq!(text(zone.horizontal(Side::Left)), "-133.19,-79.99");
    assert_eq!(text(zone.horizontal(Side::Right)), "79.99,133.19");
    for bins in [None, Some([255, 0, 0, 255])] {
        let zone = stage.resolve(bins);
        let mut count = 0;
        for side in Side::BOTH {
            for at in points(zone, side) {
                let present = stage.contains(&zone, side, at);
                assert_eq!(present, plain.contains(&zone, side, at), "{side:?} {at:?}");
                count += usize::from(present);
            }
        }
        if bins.is_none() {
            assert_eq!(count, 5152);
        }
    }
}

#[test]
fn a_platform_in_the_open_air_drops_the_spots_over_it() {
    let mut lines = BATTLEFIELD_LINES.to_vec();
    lines.push(([[100.0, -30.0], [130.0, -30.0]], true, true));
    let stage = battlefield(&lines);
    let plain = battlefield(&BATTLEFIELD_LINES);
    let zone = stage.resolve(Some([255, 0, 255, 255]));
    for at in [[115.0, 0.0], [115.0, -20.0], [115.0, -32.0]] {
        assert!(!stage.contains(&zone, Side::Right, at), "{at:?}");
    }
    for at in [[115.0, -45.0], [90.0, 0.0], [140.0, 0.0]] {
        assert!(stage.contains(&zone, Side::Right, at), "{at:?}");
    }
    for at in [[115.0, 0.0], [115.0, -20.0], [115.0, -32.0],
        [115.0, -45.0], [90.0, 0.0], [140.0, 0.0]] {
        assert!(plain.contains(&zone, Side::Right, at), "{at:?}");
    }
    assert!(!stage.allows_move(&zone, Side::Right, [115.0, -45.0], [115.0, -20.0]));
}

#[test]
fn another_solid_stops_the_outer_line() {
    let mut lines = BATTLEFIELD_LINES.to_vec();
    lines.extend([
        ([[150.0, -10.0], [200.0, -10.0]], true, false),
        ([[200.0, -10.0], [200.0, -60.0]], false, false),
        ([[200.0, -60.0], [150.0, -60.0]], false, false),
        ([[150.0, -60.0], [150.0, -10.0]], false, false),
    ]);
    let stage = battlefield(&lines);
    let zone = stage.resolve(Some([128, 128, 255, 255]));
    assert_eq!(text(zone.horizontal(Side::Right)), "79.99,150.00");
    assert_eq!(text(zone.horizontal(Side::Left)), "-232.00,-79.99");
    assert_eq!(text(stage.resolve(None).horizontal(Side::Right)), "79.99,133.19");
    let contained: Vec<_> = points(zone, Side::Right).into_iter()
        .filter(|&at| stage.contains(&zone, Side::Right, at)).collect();
    assert!(!contained.is_empty());
    assert!(contained.iter().all(|p| p[0] <= 146.0));
}

#[test]
fn another_solid_under_the_stage_drops_the_spots_in_and_over_it() {
    let mut lines = BATTLEFIELD_LINES.to_vec();
    lines.extend([
        ([[45.0, -70.0], [75.0, -70.0]], true, false),
        ([[75.0, -70.0], [75.0, -90.0]], false, false),
        ([[75.0, -90.0], [45.0, -90.0]], false, false),
        ([[45.0, -90.0], [45.0, -70.0]], false, false),
    ]);
    let stage = battlefield(&lines);
    let plain = battlefield(&BATTLEFIELD_LINES);
    let zone = stage.resolve(Some([128, 0, 0, 255]));
    for at in [[60.0, -60.0], [60.0, -80.0], [77.0, -80.0], [78.5, -80.0]] {
        assert!(!stage.contains(&zone, Side::Right, at), "{at:?}");
        assert!(plain.contains(&zone, Side::Right, at), "{at:?}");
    }
    for at in [[60.0, -100.0], [79.5, -80.0], [81.0, -80.0]] {
        assert!(stage.contains(&zone, Side::Right, at), "{at:?}");
    }
}

#[test]
fn a_platform_under_a_sloping_underside_drops_the_spot_over_it() {
    let mut lines = BATTLEFIELD_LINES.to_vec();
    lines.push(([[-79.0, -36.0], [-61.0, -36.0]], true, true));
    let stage = battlefield(&lines);
    let plain = battlefield(&BATTLEFIELD_LINES);
    let zone = stage.resolve(Some([128, 0, 0, 255]));
    assert!(!stage.contains(&zone, Side::Left, [-70.0, -30.0]));
    assert!(stage.contains(&zone, Side::Left, [-70.0, -45.0]));
    assert!(plain.contains(&zone, Side::Left, [-70.0, -30.0]));
}

#[test]
fn the_main_solid_is_not_the_next_solid() {
    let lines = [
        ([[-40.0, 5.0], [40.0, 5.0]], true, false),
        ([[40.0, 5.0], [70.0, -60.0]], false, false),
        ([[70.0, -60.0], [-70.0, -60.0]], false, false),
        ([[-70.0, -60.0], [-40.0, 5.0]], false, false),
    ];
    let blast = [-140.0, 140.0, 140.0, -130.0];
    let stage = read(0, &lines, 4, blast, [-100.0, 100.0, 100.0, -100.0]);
    let plain = Stage::new(0, blast, [[-40.0, 5.0], [40.0, 5.0]], outline(&lines, 4),
        [Some(-140.0), Some(140.0)], &[]).unwrap();
    let zone = stage.resolve(Some([128, 0, 0, 255]));
    assert_eq!(zone, plain.resolve(Some([128, 0, 0, 255])));
    assert!(zone.horizontal(Side::Left).is_some() && zone.horizontal(Side::Right).is_some());
    assert!(stage.contains(&zone, Side::Right, [78.0, -38.0]));
    assert!(!stage.contains(&zone, Side::Right, [48.0, -38.0]));
}

#[test]
fn a_side_without_open_air_has_no_zone() {
    let ledges = [BATTLEFIELD_LINES[4].0[0], BATTLEFIELD_LINES[6].0[1]];
    let make = |air| Stage::new(0, BATTLEFIELD_BLAST, ledges, outline(&BATTLEFIELD_LINES, 10), air, &[]);
    let one = make([Some(-240.0), None]).unwrap().resolve(None);
    assert!(one.horizontal(Side::Left).is_some() && one.horizontal(Side::Right).is_none());
    let narrow = make([Some(-85.0), Some(240.0)]).unwrap().resolve(None);
    assert!(narrow.horizontal(Side::Left).is_none() && narrow.horizontal(Side::Right).is_some());
    assert!(make([None, None]).is_none());
    let lines = [
        ([[-80.0, 0.0], [80.0, 0.0]], true, false),
        ([[80.0, 0.0], [80.0, -30.0]], false, false),
        ([[80.0, -30.0], [-80.0, -30.0]], false, false),
        ([[-80.0, -30.0], [-80.0, 0.0]], false, false),
        ([[80.0, 40.0], [140.0, 40.0]], true, false),
        ([[140.0, 40.0], [140.0, -40.0]], false, false),
        ([[140.0, -40.0], [80.0, -40.0]], false, false),
        ([[80.0, -40.0], [80.0, 40.0]], false, false),
    ];
    let stage = read(0, &lines, 4, [-240.0, 240.0, 180.0, -140.0], BATTLEFIELD_CAMERA);
    let zone = stage.resolve(None);
    assert!(zone.horizontal(Side::Left).is_some());
    assert!(zone.horizontal(Side::Right).is_none());
}

#[test]
fn a_failed_or_overfull_read_gives_no_zone() {
    struct Failed;
    impl Probe for Failed {
        fn first(&mut self, _: Point, _: f32, _: bool) -> Result<Option<Met>, Unread> { Err(Unread) }
    }
    assert!(matches!(Stage::read(0, BATTLEFIELD_BLAST, BATTLEFIELD_CAMERA,
        outline(&BATTLEFIELD_LINES, 10), &mut Failed), Err(Unread)));
    let mut lines = BATTLEFIELD_LINES.to_vec();
    let others: Vec<_> = (0..17).map(|k| {
        let y = -10.0 - 5.0 * k as f32;
        ([[95.0, y], [125.0, y]], true, true)
    }).collect();
    lines.extend(others.iter().copied());
    assert!(matches!(Stage::read(0, BATTLEFIELD_BLAST, BATTLEFIELD_CAMERA,
        outline(&lines, 10), &mut Copied(&mets(&lines))), Ok(None)));
    let ledges = [BATTLEFIELD_LINES[4].0[0], BATTLEFIELD_LINES[6].0[1]];
    let make = |other: &[Line]| Stage::new(0, BATTLEFIELD_BLAST, ledges,
        outline(&BATTLEFIELD_LINES, 10), [Some(-240.0), Some(240.0)], &mets(other));
    assert!(make(&others).is_none());
    assert!(make(&others[..16]).is_some());
}

#[test]
fn the_outer_step_stops_at_the_solid_without_dead_travel() {
    let stage = training();
    let mut wizard = Wizard::new(None);
    for _ in 0..3 { wizard.input(&stage, [0.0; 2], 1 << key::A); }
    assert_eq!(wizard.step(), Step::Outer);
    let outer = |wizard: Wizard| wizard.zone(&stage).horizontal(Side::Left).unwrap()[0];
    assert_eq!(outer(wizard), -170.0);
    wizard.input(&stage, [-1.0, 0.0], 0);
    assert!(outer(wizard) > -170.0);
    for _ in 0..2 { wizard.input(&stage, [1.0, 0.0], 0); }
    assert_eq!(outer(wizard), -170.0);
    wizard.input(&stage, [-1.0, 0.0], 0);
    assert!(outer(wizard) > -170.0);
    let stage = stage_three();
    let mut wizard = Wizard::new(None);
    for _ in 0..3 { wizard.input(&stage, [0.0; 2], 1 << key::A); }
    for _ in 0..64 { wizard.input(&stage, [1.0, 0.0], 0); }
    assert_eq!(text(wizard.zone(&stage).horizontal(Side::Left)), "-232.00,-79.99");
}

#[test]
fn the_stage_grows_by_its_two_open_air_ends_only() {
    assert_eq!(std::mem::size_of::<Stage>(), 1080);
    assert_eq!(std::mem::size_of::<Option<Stage>>(), 1080);
}

#[test]
fn a_long_move_does_not_pass_through_another_solid() {
    let mut lines = BATTLEFIELD_LINES.to_vec();
    lines.extend([
        ([[55.0, -70.0], [75.0, -70.0]], true, false),
        ([[75.0, -70.0], [75.0, -90.0]], false, false),
        ([[75.0, -90.0], [55.0, -90.0]], false, false),
        ([[55.0, -90.0], [55.0, -70.0]], false, false),
    ]);
    let stage = battlefield(&lines);
    let plain = battlefield(&BATTLEFIELD_LINES);
    let zone = stage.resolve(Some([128, 0, 0, 255]));
    assert!(stage.contains(&zone, Side::Right, [82.0, -80.0]));
    assert!(stage.contains(&zone, Side::Right, [46.0, -80.0]));
    assert!(!stage.allows_move(&zone, Side::Right, [82.0, -80.0], [46.0, -80.0]));
    assert!(plain.allows_move(&zone, Side::Right, [82.0, -80.0], [46.0, -80.0]));
    assert!(stage.allows_move(&zone, Side::Right, [82.0, -80.0], [80.5, -80.0]));
    for spot in stage.samples(&zone)[5..].iter().flatten()
        .filter(|spot| (-88.0..=-72.0).contains(&spot[1])) {
        assert!(spot[0] > 75.0, "{spot:?}");
    }
}
}
