use super::blade_path;
use crate::model::edge_juggle::perfect::{Point, Segment};

// Stock Battlefield/FD solid ring in near-ledge coordinates.
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
const HANG: Point = [161.975, -13.4];
const BOTTOM: f32 = -140.0;

#[test]
fn sora_can_use_an_outward_middle_dash_under_the_far_slope() {
    let origin = [116.1, -55.5];
    let route = blade_path(&RING, origin, HANG, 1.0, BOTTOM).expect("the angled exit is clear");
    assert_eq!(route.count, 3);
    assert_eq!(route.angles[0], 0.0);
    assert!((40.0..90.0).contains(&route.angles[1]));
    assert!((90.0..=140.0).contains(&route.angles[2]));
}

#[test]
fn sora_keeps_stock_when_no_blade_path_reaches_or_live_geometry_closes_it() {
    for origin in [[88.0, -50.0], [161.0, -60.0], [155.2, -139.0], [f32::NAN, -42.1]] {
        assert!(blade_path(&RING, origin, HANG, 1.0, BOTTOM).is_none());
    }
    let mut changed = RING.to_vec();
    changed.push([[157.0, -49.0], [230.0, -49.0]]);
    assert!(blade_path(&changed, [155.2, -53.0], HANG, 1.0, BOTTOM).is_none());
    assert!(blade_path(&RING, [155.2, -42.1], HANG, 0.0, BOTTOM).is_none());
}

#[test]
fn sora_uses_status_to_status_dash_distances_for_the_measured_exit() {
    // Model §5.2, fd-1 and fd-3. This includes each dash's final step into
    // the next native status before applying the first dash's 4.9 carry.
    let origin = [145.7, -43.5];
    let route = blade_path(&RING, origin, HANG, 1.0, BOTTOM).unwrap();
    assert_eq!(route.count, 3);
    assert_eq!(route.angles[0], 0.0);
    assert_eq!(route.angles[1], 130.0);
    assert!((127.0..129.0).contains(&route.angles[2]), "{route:?}");
    assert!((18.5..19.0).contains(&route.spare), "{route:?}");

    let mirror = |[x, y]: Point| [-x, y];
    let outline = RING.map(|segment| segment.map(mirror));
    let mirrored = blade_path(&outline, mirror(origin), mirror(HANG), -1.0, BOTTOM).unwrap();
    assert_eq!(route, mirrored);
}
