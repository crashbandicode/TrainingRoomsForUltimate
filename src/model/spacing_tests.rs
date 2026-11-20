lib_unit_tests! {
use super::*;
use crate::model::eggs::{self, Hit};
use std::collections::BTreeMap;

const ENTRY_KIND: usize = 0x20;
const ENTRY_ATTACKER: usize = 0x24;
const ENTRY_SLOT: usize = 0x32;
const ENTRY_PART: usize = 0x33;
const ENTRY_CONTACT: usize = 0x10;

#[derive(Clone)]
struct Fixture {
    memory: BTreeMap<usize, u64>,
    module: usize,
    entry: usize,
    lanes: [usize; 2],
    damage: usize,
    hit_module: usize,
    parts: usize,
    shape: usize,
}

fn put_u8(memory: &mut BTreeMap<usize, u64>, at: usize, value: u8) {
    let base = at & !7;
    let shift = (at & 7) * 8;
    let word = memory.get(&base).copied().unwrap_or(0) & !(u64::from(u8::MAX) << shift);
    memory.insert(base, word | (u64::from(value) << shift));
}

fn put_u32(memory: &mut BTreeMap<usize, u64>, at: usize, value: u32) {
    let base = at & !7;
    let shift = (at & 7) * 8;
    let word = memory.get(&base).copied().unwrap_or(0) & !(u64::from(u32::MAX) << shift);
    memory.insert(base, word | (u64::from(value) << shift));
}

fn fixture() -> Fixture {
    use crate::model::hitbox::layout;
    use crate::model::sites_hitbox::{damage_record as record, hurt_shape as shape_layout};

    let (module, entry, lanes, hit_module, parts, shape) =
        (0x20000, 0x30000, [0x10000, 0x11000], 0x40000, 0x50000, 0x60000);
    let damage = lanes[1] + record::MODULE;
    let part = parts + layout::HIT_PART_STRIDE;
    let mut memory = BTreeMap::new();
    memory.insert(damage, module as u64);
    memory.insert(lanes[1] + layout::HIT_MODULE, hit_module as u64);
    memory.insert(hit_module + layout::HIT_PARTS, parts as u64);
    put_u32(&mut memory, hit_module + layout::HIT_PART_TOTAL, 2);
    memory.insert(part + shape_layout::SHAPE, shape as u64);
    put_u32(&mut memory, entry + ENTRY_KIND, 1);
    put_u32(&mut memory, entry + ENTRY_ATTACKER, 0x1000_0000);
    put_u8(&mut memory, entry + ENTRY_SLOT, 2);
    put_u8(&mut memory, entry + ENTRY_PART, 1);
    for (i, value) in [1.0_f32, 2.0, -3.5].into_iter().enumerate() {
        put_u32(&mut memory, entry + ENTRY_CONTACT + i * 4, value.to_bits());
        put_u32(&mut memory, shape + shape_layout::CONTACT + i * 4, value.to_bits());
    }
    put_u32(&mut memory, shape + shape_layout::PEN, 2.0_f32.to_bits());
    put_u32(&mut memory, shape + shape_layout::RADIUS, 4.0_f32.to_bits());
    Fixture { memory, module, entry, lanes, damage, hit_module, parts, shape }
}

fn captured(fixture: &Fixture) -> Option<Contact> {
    capture(|at| fixture.memory.get(&at).copied(), fixture.module, fixture.entry, fixture.lanes)
}

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
    let contact = Contact { attacker: 7, receiver: 1, slot: 2, part: 3, pen: 2.0, radius: 4.0 };
    assert_eq!(contact.grade(), Some(Grade::Good));
    assert_eq!(Contact { pen: -0.1, ..contact }.grade(), None);
    assert_eq!(Contact { pen: f32::NAN, ..contact }.grade(), None);
    assert_eq!(Contact { radius: 0.0, ..contact }.grade(), None);
    assert_eq!(Contact { radius: f32::INFINITY, ..contact }.grade(), None);
}

fn capture_reads_contact_at_handler() {
    let fixture = fixture();
    assert_eq!(
        captured(&fixture),
        Some(Contact { attacker: 0x1000_0000, receiver: 1, slot: 2, part: 1, pen: 2.0, radius: 4.0 })
    );
}

fn capture_guards_suppress_only_grade() {
    use crate::model::hitbox::layout;
    use crate::model::sites_hitbox::hurt_shape;

    let invalid = [0, 0x1001, 0x0000_8000_0000_0000];
    for bad in invalid {
        let mut f = fixture();
        f.module = bad;
        assert!(captured(&f).is_none(), "module {bad:#x}");
        let mut f = fixture();
        f.entry = bad;
        assert!(captured(&f).is_none(), "entry {bad:#x}");
        let mut f = fixture();
        f.lanes[1] = bad;
        assert!(captured(&f).is_none(), "lane {bad:#x}");
        let mut f = fixture();
        f.memory.insert(f.lanes[1] + layout::HIT_MODULE, bad as u64);
        assert!(captured(&f).is_none(), "HitModule {bad:#x}");
        let mut f = fixture();
        f.memory.insert(f.hit_module + layout::HIT_PARTS, bad as u64);
        assert!(captured(&f).is_none(), "parts {bad:#x}");
        let mut f = fixture();
        f.memory.insert(f.parts + layout::HIT_PART_STRIDE + hurt_shape::SHAPE, bad as u64);
        assert!(captured(&f).is_none(), "shape {bad:#x}");
    }
    let mut f = fixture();
    f.memory.insert(f.damage, 0x21000);
    assert!(captured(&f).is_none(), "no matching lane");
    let mut f = fixture();
    put_u32(&mut f.memory, f.entry + ENTRY_KIND, 0x3a6);
    assert!(captured(&f).is_some(), "projectile entry kind");
    let mut f = fixture();
    put_u32(&mut f.memory, f.hit_module + layout::HIT_PART_TOTAL, 1);
    assert!(captured(&f).is_none(), "part equals total");
    let mut f = fixture();
    put_u32(&mut f.memory, f.shape + hurt_shape::CONTACT, f32::from_bits(1.0_f32.to_bits() + 1).to_bits());
    assert!(captured(&f).is_none(), "contact differs by one ulp");
    for pen in [f32::NAN, -0.1] {
        let mut f = fixture();
        put_u32(&mut f.memory, f.shape + hurt_shape::PEN, pen.to_bits());
        assert!(captured(&f).is_none(), "pen={pen}");
    }
    for radius in [0.0, f32::INFINITY] {
        let mut f = fixture();
        put_u32(&mut f.memory, f.shape + hurt_shape::RADIUS, radius.to_bits());
        assert!(captured(&f).is_none(), "radius={radius}");
    }
}

fn grader_off_hook_reads_nothing() {
    use core::cell::Cell;
    let f = fixture();
    let reads = Cell::new(0);
    assert!(hook(false, |_| { reads.set(reads.get() + 1); None }, f.module, f.entry, f.lanes).is_none());
    assert_eq!(reads.get(), 0);
}

fn ring_consumes_each_contact_once() {
    let ring = Ring::<8>::new();
    let contact = Contact { attacker: 4, receiver: 1, slot: 2, part: 3, pen: 1.0, radius: 4.0 };
    ring.push(10, contact);
    ring.push(11, contact);
    let mut got = Vec::new();
    ring.drain(|epoch, item| got.push((epoch, item)));
    assert_eq!(got.len(), 2);
    ring.drain(|epoch, item| got.push((epoch, item)));
    assert_eq!(got.len(), 2, "each pushed entry is yielded once");

    let overflow = Ring::<8>::new();
    for epoch in 0..9 {
        overflow.push(epoch, contact);
    }
    let mut epochs = Vec::new();
    overflow.drain(|epoch, _| epochs.push(epoch));
    assert_eq!(epochs, (1..9).collect::<Vec<_>>(), "overflow drops the oldest entry");

    let torn = Ring::<8>::new();
    torn.tear_for_test(0);
    let mut count = 0;
    torn.drain(|_, _| count += 1);
    assert_eq!(count, 0, "a torn slot is skipped");
}

fn rings_take_first_accepted_per_ring() {
    let rings = Rings::new();
    let excellent = Contact { attacker: 1, receiver: 1, slot: 0, part: 0, pen: 1.0, radius: 4.0 };
    let ok = Contact { pen: 4.0, ..excellent };
    rings.received[1].push(7, excellent);
    rings.received[1].push(7, ok);
    rings.targets.push(7, ok);
    let taken = rings.take(7);
    assert_eq!(taken.received, [None, Some(Grade::Excellent)]);
    assert_eq!(taken.target, Some(Grade::Ok));
    assert_eq!(rings.take(8), Taken { received: [None; 2], target: None });
}

fn target_zone_rings_radii() {
    let centre = [3.0, -2.0, 0.0];
    let discs = zone_discs(centre);
    assert_eq!(discs.map(|d| d.0), [centre; 3]);
    for (actual, expected) in discs.map(|d| d.1).into_iter().zip([4.0, 8.0 / 3.0, 4.0 / 3.0]) {
        assert!((actual - expected).abs() < 1e-6);
    }
    assert_eq!(discs.map(|d| d.2), [
        [72, 208, 118, 96],
        [244, 186, 64, 96],
        [232, 232, 236, 96],
    ]);
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("zone_boundaries", zone_boundaries as fn()),
        ("edge_hit_is_excellent", edge_hit_is_excellent as fn()),
        ("depth_caps_at_axis", depth_caps_at_axis as fn()),
        ("grade_from_native_record", grade_from_native_record as fn()),
        ("capture_reads_contact_at_handler", capture_reads_contact_at_handler as fn()),
        ("capture_guards_suppress_only_grade", capture_guards_suppress_only_grade as fn()),
        ("grader_off_hook_reads_nothing", grader_off_hook_reads_nothing as fn()),
        ("ring_consumes_each_contact_once", ring_consumes_each_contact_once as fn()),
        ("rings_take_first_accepted_per_ring", rings_take_first_accepted_per_ring as fn()),
        ("target_zone_rings_radii", target_zone_rings_radii as fn()),
    ]);
}
}
