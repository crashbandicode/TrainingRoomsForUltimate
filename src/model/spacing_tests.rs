lib_unit_tests! {
use super::*;
use crate::model::eggs::{self, Hit};

fn zone_boundaries() {
    let two_thirds = 2.0_f32 / 3.0;
    let below_two_thirds = f32::from_bits(two_thirds.to_bits() - 1);
    for (depth, expected) in [
        (0.0, Grade::Excellent),
        (0.3333, Grade::Excellent),
        (1.0 / 3.0, Grade::Good),
        (0.5, Grade::Good),
        (below_two_thirds, Grade::Good),
        (two_thirds, Grade::Ok),
        (1.0, Grade::Ok),
    ] {
        assert_eq!(Grade::of(depth), expected, "depth={depth}");
    }
    assert_eq!(Grade::Excellent.name(), "Excellent");
    assert_eq!(Grade::Good.name(), "Good");
    assert_eq!(Grade::Ok.name(), "OK");
    assert_eq!(Grade::Excellent.color(), [72, 208, 118, 255]);
    assert_eq!(Grade::Good.color(), [244, 186, 64, 255]);
    assert_eq!(Grade::Ok.color(), [232, 232, 236, 255]);
    assert_eq!(ROW, "spacing_grader");
}

fn edge_hit_is_excellent() {
    let hit = Hit([4.9, 0.0, 0.0], [4.9, 0.0, 0.0], 1.0, 16.0);
    assert_eq!(eggs::depth(&hit, [0.0, 0.0]).map(Grade::of), Some(Grade::Excellent));
}

fn depth_caps_at_axis() {
    assert_eq!(depth(5.0, 4.0), Some(1.0));
    assert_eq!(grade(8.0, 4.0), Some(Grade::Ok));
    let pill = Hit([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.1, 16.0);
    assert_eq!(eggs::depth(&pill, [0.0, 0.0]).map(Grade::of), Some(Grade::Ok));
}

fn grade_from_native_record() {
    let record_p = [1.0, 0.0, -3.5];
    assert_eq!(native(record_p, record_p, 2.0, 4.0), Some(Grade::Good));

    let mut different_p = record_p;
    different_p[0] = f32::from_bits(different_p[0].to_bits() + 1);
    assert_eq!(native(record_p, different_p, 2.0, 4.0), None);
    assert_eq!(native(record_p, record_p, -0.1, 4.0), None);
    assert_eq!(native(record_p, record_p, 1.0, 0.0), None);
    assert_eq!(native(record_p, record_p, f32::NAN, 4.0), None);
    assert_eq!(native(record_p, record_p, 1.0, f32::NAN), None);
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("zone_boundaries", zone_boundaries as fn()),
        ("edge_hit_is_excellent", edge_hit_is_excellent as fn()),
        ("depth_caps_at_axis", depth_caps_at_axis as fn()),
        ("grade_from_native_record", grade_from_native_record as fn()),
    ]);
}
}
