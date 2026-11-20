use super::quick_attack;
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
