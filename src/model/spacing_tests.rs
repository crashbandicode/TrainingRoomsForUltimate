lib_unit_tests! {
use super::*;
use crate::model::eggs::{self, Hit};
use std::collections::BTreeMap;

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

fn contact_reads_the_receiver_part() {
    use crate::model::hitbox::layout;
    use crate::model::sites_hitbox::{damage_record as record_layout, hurt_shape as shape_layout};

    let (accessor, damage, record, module, parts, shape) = (0x10000, 0x20000, 0x30000, 0x40000, 0x50000, 0x60000);
    let part = parts + layout::HIT_PART_STRIDE;
    let mut memory = BTreeMap::new();
    let put_u8 = |memory: &mut BTreeMap<usize, u64>, at: usize, value: u8| {
        let base = at & !7;
        let shift = (at & 7) * 8;
        let word = memory.get(&base).copied().unwrap_or(0) & !(u64::from(u8::MAX) << shift);
        memory.insert(base, word | (u64::from(value) << shift));
    };
    let put_u32 = |memory: &mut BTreeMap<usize, u64>, at: usize, value: u32| {
        let base = at & !7;
        let shift = (at & 7) * 8;
        let word = memory.get(&base).copied().unwrap_or(0) & !(u64::from(u32::MAX) << shift);
        memory.insert(base, word | (u64::from(value) << shift));
    };
    for (at, value) in [
        (accessor + record_layout::MODULE, damage as u64),
        (damage + record_layout::RECORDS, record as u64),
        (accessor + layout::HIT_MODULE, module as u64),
        (module + layout::HIT_PARTS, parts as u64),
        (module + layout::HIT_PART_TOTAL, 3),
        (part + shape_layout::SHAPE, shape as u64),
    ] {
        memory.insert(at, value);
    }
    put_u8(&mut memory, record + record_layout::PART, 1);
    for (i, value) in [1.0_f32, 2.0, -3.5].into_iter().enumerate() {
        put_u32(&mut memory, record + record_layout::CONTACT + i * 4, value.to_bits());
        put_u32(&mut memory, shape + shape_layout::CONTACT + i * 4, value.to_bits());
    }
    put_u32(&mut memory, shape + shape_layout::PEN, 2.0_f32.to_bits());
    put_u32(&mut memory, shape + shape_layout::RADIUS, 4.0_f32.to_bits());
    assert_eq!(read_contact(|at| memory.get(&at).copied(), accessor), Some(Grade::Good));

    put_u8(&mut memory, record + record_layout::PART, 3);
    assert_eq!(read_contact(|at| memory.get(&at).copied(), accessor), None, "part must be below total");

    put_u8(&mut memory, record + record_layout::PART, 1);
    memory.insert(part + shape_layout::SHAPE, 0);
    assert_eq!(read_contact(|at| memory.get(&at).copied(), accessor), None, "null shape pointer");
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("zone_boundaries", zone_boundaries as fn()),
        ("edge_hit_is_excellent", edge_hit_is_excellent as fn()),
        ("depth_caps_at_axis", depth_caps_at_axis as fn()),
        ("grade_from_native_record", grade_from_native_record as fn()),
        ("contact_reads_the_receiver_part", contact_reads_the_receiver_part as fn()),
    ]);
}
}
