use super::{quick_attack, skull_bash_start};
use crate::model::edge_juggle::perfect::{Point, Segment};

// Stock common ring from the Battlefield and Waiting Room LVD, in a near-ledge frame.
const RING: [Segment; 8] = [
    [[0.0, 0.0], [0.0, -8.319]],
    [[0.0, -8.319], [15.591, -25.111]],
    [[15.591, -25.111], [38.590, -40.099]],
    [[38.590, -40.099], [121.402, -40.118]],
    [[121.402, -40.118], [144.373, -25.103]],
    [[144.373, -25.103], [159.975, -8.319]],
    [[159.975, -8.319], [159.975, 0.0]],
    [[159.975, 0.0], [0.0, 0.0]],
];
const TARGET: Point = [161.675, -9.9];
const APEX: Point = [136.825, -76.025];
const BOTTOM: f32 = -140.0;

fn degrees([x, y]: Point) -> f32 {
    y.atan2(x).to_degrees()
}

#[test]
fn skull_bash_apex_aims_two_distinct_zips_around_the_far_lip() {
    for sign in [1.0, -1.0] {
        let mirror = |[x, y]: Point| [sign * x, y];
        let stage = RING.map(|line| line.map(mirror));
        let route = quick_attack(&stage, mirror(APEX), mirror(TARGET), BOTTOM, 10.0)
            .expect("the baked Skull Bash approach leaves both zips");
        let first = degrees(mirror(route.first));
        let second = degrees(mirror(route.second.expect("one zip falls short")));
        assert!((first - 50.0).abs() < 0.01);
        assert!((second - 114.0).abs() < 1.0);
        assert!(second - first > 39.0);

        let mut wall = stage.to_vec();
        // Any approach from the apex through this wall intersects the live collision lines.
        wall.push([mirror([158.0, -135.0]), mirror([158.0, 30.0])]);
        assert!(quick_attack(&wall, mirror(APEX), mirror(TARGET), BOTTOM, 10.0).is_none());
    }
}

#[test]
fn quick_attack_declines_an_invalid_or_unreachable_apex() {
    assert!(quick_attack(&RING, [f32::NAN, -50.0], TARGET, BOTTOM, 10.0).is_none());
    assert!(quick_attack(&RING, [12.0, -76.0], TARGET, BOTTOM, 10.0).is_none());
    assert!(quick_attack(&RING, [158.0, -137.0], TARGET, BOTTOM, 10.0).is_none());
}


#[test]
fn skull_bash_starts_only_inside_its_baked_region_and_live_stage_shape() {
    let ledges = [[0.0, 0.0], [159.975, 0.0]];
    let release = [-20.0, -28.0];
    for stage in [0, 3, 311] {
        let path = skull_bash_start((stage, 0), &RING, ledges, release, BOTTOM, 8.5).unwrap();
        assert_eq!(path.direction, 1.0);
        assert!((path.launch_y - (-49.018)).abs() < 0.002);
        assert_eq!(path.near, [-1.7, -9.9]);
        assert_eq!(path.far, TARGET);
        assert_eq!(path.height, 10.0);
    }
    let mirror = |[x, y]: Point| [159.975 - x, y];
    let right = skull_bash_start((0, 0), &RING, ledges, mirror(release), BOTTOM, 10.0).unwrap();
    assert_eq!(right.direction, -1.0);
    assert!((right.far[0] - mirror(TARGET)[0]).abs() < 0.001);

    for stage in [(89, 0), (33, 0), (0, 1), (3, 2)] {
        assert!(skull_bash_start(stage, &RING, ledges, release, BOTTOM, 10.0).is_none());
    }
    for start in [[-60.0, -28.0], [-20.0, -69.0], [-6.0, -28.0], [f32::NAN, -28.0]] {
        assert!(skull_bash_start((3, 0), &RING, ledges, start, BOTTOM, 10.0).is_none());
    }
    assert!(skull_bash_start((3, 0), &RING, ledges, release, -125.0, 10.0).is_none());
    assert!(skull_bash_start((3, 0), &RING, ledges, release, BOTTOM, 15.0).is_none());
    let shape_changed = RING.map(|line| line.map(|[x, y]| [x, y * 0.9]));
    assert!(skull_bash_start((3, 0), &shape_changed, ledges, release, BOTTOM, 10.0).is_none());
}
