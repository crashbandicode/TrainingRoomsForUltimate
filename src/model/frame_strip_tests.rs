//! SPEC-input-06 §2 arithmetic and §3.5 behaviour, frame by frame. Frame numbers are the tracker's own: the first
//! `step` is frame 1. Statuses are arbitrary numbers: the model only compares them.

lib_unit_tests! {
use super::*;
use crate::model::spacing::{self, Contact, Grade, Rings, Taken};
use std::collections::BTreeMap;

const ATTACK: i32 = 0x27;
const A: MappedInputs = MappedInputs { buttons: 0x1, lstick_x: 0, lstick_y: 0, rstick_x: 0, rstick_y: 0 };
const NONE: MappedInputs = MappedInputs::NEUTRAL;

fn idle() -> Sample {
    Sample { actionable: true, ..Sample::default() }
}

fn attack(mv: Move, status: i32, motion_frame: f32) -> Sample {
    Sample { attack: Some(mv), status, motion_frame, power_mul: 1.0, object_id: P1_ID, ..Sample::default() }
}

/// The fighters' battle object ids (what a damage record names as the attacker).
const P1_ID: u32 = 0x10;
const CPU_ID: u32 = 0x20;

struct ContactFixture {
    memory: BTreeMap<usize, u64>,
    module: usize,
    entry: usize,
    lanes: [usize; 2],
}

fn contact_fixture(receiver: usize, attacker: u32, entry_kind: u32) -> ContactFixture {
    use crate::model::hitbox::layout;
    use crate::model::sites_hitbox::{damage_record, hurt_shape};

    let (module, entry, lanes, hit_module, parts, shape) =
        (0x20000, 0x30000, [0x10000, 0x11000], 0x40000, 0x50000, 0x60000);
    let mut memory = BTreeMap::new();
    memory.insert(lanes[receiver] + damage_record::MODULE, module as u64);
    memory.insert(lanes[receiver] + layout::HIT_MODULE, hit_module as u64);
    memory.insert(hit_module + layout::HIT_PARTS, parts as u64);
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
    put_u32(&mut memory, hit_module + layout::HIT_PART_TOTAL, 2);
    memory.insert(parts + layout::HIT_PART_STRIDE + hurt_shape::SHAPE, shape as u64);
    put_u32(&mut memory, entry + 0x20, entry_kind);
    put_u32(&mut memory, entry + 0x24, attacker);
    put_u8(&mut memory, entry + 0x32, 2);
    put_u8(&mut memory, entry + 0x33, 1);
    for (i, value) in [1.0_f32, 2.0, -3.5].into_iter().enumerate() {
        put_u32(&mut memory, entry + 0x10 + i * 4, value.to_bits());
        put_u32(&mut memory, shape + hurt_shape::CONTACT + i * 4, value.to_bits());
    }
    put_u32(&mut memory, shape + hurt_shape::PEN, 2.0_f32.to_bits());
    put_u32(&mut memory, shape + hurt_shape::RADIUS, 4.0_f32.to_bits());
    ContactFixture { memory, module, entry, lanes }
}

fn capture_contact(fixture: &ContactFixture) -> Option<Contact> {
    spacing::capture(
        |at| fixture.memory.get(&at).copied(),
        fixture.module,
        fixture.entry,
        fixture.lanes,
    )
}

fn receive(contact: Contact, epoch: u32) -> (Rings, Taken) {
    let rings = Rings::new();
    rings.received[usize::from(contact.receiver)].push(epoch, contact);
    let taken = rings.take(epoch);
    (rings, taken)
}

fn samples(taken: Taken, player: bool) -> (Sample, Sample) {
    (
        Sample { contact: taken.received[0], ..Sample::default() },
        Sample { contact: taken.received[1], player, ..Sample::default() },
    )
}

/// A script: P1, the CPU and P1's inputs for each frame, from frame 1.
struct Script {
    frames: Vec<(Sample, Sample, MappedInputs)>,
}

impl Script {
    fn new() -> Self {
        Self { frames: Vec::new() }
    }
    fn push(&mut self, p1: Sample, cpu: Sample, inputs: MappedInputs) -> &mut Self {
        self.frames.push((p1, cpu, inputs));
        self
    }
    fn run(&self) -> Tracker {
        let mut t = Tracker::new();
        for (p1, cpu, inputs) in &self.frames {
            t.step(*p1, *cpu, *inputs);
        }
        t
    }
}

fn freeze_after_quiet_frames_and_move_again_on_a_press() {
    let mut s = Script::new();
    for _ in 0..(FREEZE_AFTER as usize + 10) {
        s.push(idle(), idle(), NONE);
    }
    let mut t = s.run();
    assert!(t.frozen());
    assert_eq!(t.len(), FREEZE_AFTER as usize - 1, "the frames before the freeze stay on screen");
    t.step(attack(Move::Jab, ATTACK, 0.0), idle(), A);
    assert!(!t.frozen());
    assert_eq!(t.newest(), Some(FREEZE_AFTER as u32 + 11));
    assert_eq!(t.presses().next().map(|p| p.button), Some(Button::Attack));
}

fn bin_stick_eight_directions_and_the_dead_zone() {
    assert_eq!(bin_stick(100, 0), Some(0));
    assert_eq!(bin_stick(100, 100), Some(1));
    assert_eq!(bin_stick(0, 100), Some(2));
    assert_eq!(bin_stick(-100, 100), Some(3));
    assert_eq!(bin_stick(-100, 0), Some(4));
    assert_eq!(bin_stick(-100, -100), Some(5));
    assert_eq!(bin_stick(0, -100), Some(6));
    assert_eq!(bin_stick(100, -100), Some(7));
    assert_eq!(bin_stick(20, 30), None, "on both thresholds");
    assert_eq!(bin_stick(21, 0), Some(0));
}

fn the_status_predicates_use_the_resolved_constants() {
    use super::ids::{Table, C};
    let t = Table::with(&[
        (C::Damage, 0x47),
        (C::DamageFall, 0x4f),
        (C::GuardOn, 0x1b),
        (C::GuardDamage, 0x1e),
        (C::GuardOff, 0x1d),
        (C::Attack, 0x27),
        (C::AttackHi3, 0x2b),
        (C::AttackS4Start, 0x2d),
        (C::AttackS4, 0x2f),
        (C::Final, 0x1e0),
    ]);
    assert!(t.hitstun(0x47) && t.hitstun(0x4e) && !t.hitstun(0x4f), "DAMAGE ..< DAMAGE_FALL");
    assert!(t.shieldstun(0x1e, 0) && t.shieldstun(0x1d, 0x1e) && !t.shieldstun(0x1d, 0x1b));
    assert!(t.shielding(0x1b) && t.shielding(0x1e) && !t.shielding(0x1f));
    assert_eq!(t.attack(0x27), Some(Move::Jab));
    assert_eq!(t.attack(0x2b), Some(Move::UpTilt));
    assert_eq!(t.attack(0x2e), Some(Move::SideSmash), "inside the smash's start..=end range");
    assert_eq!(t.attack(0x1f0), Some(Move::Other), "a fighter's own status");
    assert_eq!(t.attack(0x10), None);
    let empty = Table::unresolved();
    assert!(!empty.hitstun(0x47) && empty.attack(0x27).is_none(), "nothing matches an unresolved constant");
}

/// rc.8's input display bind flips the row the menu shows, read as `Settings::from_snapshot` reads it.
fn display_toggle_flips_the_strip_row() {
    assert_eq!(toggled(Some(&[0, 1])), ([1, 0], false), "Frame strip -> Off");
    assert_eq!(toggled(Some(&[1, 0])), ([0, 1], true), "Off -> Frame strip");
    assert_eq!(toggled(None), ([1, 0], false), "a missing row is the factory Frame strip");
    assert_eq!(toggled(Some(&[0, 0])), ([1, 0], false), "nothing set reads as the strip");
    let mut snapshot = crate::model::settings::Snapshot::factory();
    assert!(Settings::from_snapshot(&snapshot).on);
    for expected in [false, true, false] {
        let (next, on) = toggled(snapshot.get(ROW));
        snapshot.set(ROW, &next).unwrap();
        assert_eq!((Settings::from_snapshot(&snapshot).on, on), (expected, expected));
    }
}

/// The damages of hitboxes 0.. (the rest inactive), each of growth 100 and no base: damage ranks them.
fn p(values: &[f32]) -> Powers {
    let mut out = [spots::Hit::default(); spots::SLOTS];
    for (slot, damage) in values.iter().enumerate() {
        out[slot] = spots::Hit { damage: *damage, growth: 100, fixed: 0, base: 0 };
    }
    out
}

/// `victim` (at `before`, standard weight) hurt by slot `slot` of `powers`, whose owner's object id is `by`: the damage it gained, and what its damage
/// record holds on that frame (the knockback the game stored and the attacker's id and slot).
fn struck_by(victim: Sample, by: u32, powers: &Powers, slot: usize, gained: f32, before: f32) -> Sample {
    // The game's percent for its knockback: the percent before plus the hit's damage before the 1-on-1 ratio (spots-4).
    let launched = spots::knockback(&powers[slot], gained / spots::TARGET_MUL, before + gained / spots::TARGET_MUL, 100.0);
    Sample { stop: true, hitstun: true, damage: before + gained, launched, struck: Some((by, slot as u8)), ..victim }
}

/// spots-3: King K. Rool's Gut Check, the hit and the gust of equal damage (numbers drawn for the case). The CPU takes the hit of slot `slot`;
/// `recorded`: its damage record names P1 and the slot. The labelled hits.
fn gut_check_marks(slot: usize, recorded: bool) -> Vec<Mark> {
    let mut powers = p(&[]);
    powers[0] = spots::Hit { damage: 14.0, growth: 90, fixed: 0, base: 70 };
    powers[1] = spots::Hit { damage: 14.0, growth: 50, fixed: 0, base: 40 };
    let calm = Sample { actionable: true, damage: 20.0, power_mul: 1.0, ..Sample::default() };
    let swing = |stop| Sample { attack: Some(Move::DownSpecial), status: ATTACK, active: true, stop, powers, object_id: P1_ID, ..calm };
    let hurt = struck_by(calm, if recorded { P1_ID } else { CPU_ID }, &powers, slot, 14.0 * M, 20.0);
    let mut t = Tracker::new();
    t.step(calm, calm, NONE);
    t.step(swing(false), calm, NONE);
    t.step(swing(true), hurt, NONE);
    t.marks().collect()
}

/// spots-3: hitboxes of equal damage are told apart by the slot the game recorded; a record that names another object is no label.
fn a_hit_of_equal_damage_is_labelled_by_the_slot_the_game_recorded() {
    assert_eq!(gut_check_marks(0, true), vec![Mark { frame: 3, lane: 1, spot: Spot::Sweet }], "the hit");
    assert_eq!(gut_check_marks(1, true), vec![Mark { frame: 3, lane: 1, spot: Spot::Sour }], "the gust");
    assert!(gut_check_marks(1, false).is_empty(), "the record names another object (a projectile): no label");
}

/// spots-3: past the game's damage cap (999%). The CPU at 995%, then at 999%, takes P1's 13 (16.38 added): its percent rises 4, then not at
/// all, so the hit is seen and placed by the damage its record says it took (`+0x8`, before the 1-on-1 ratio).
fn a_hit_past_the_damage_cap_is_labelled_by_the_records_damage() {
    let powers = p(&[16.0, 13.0]);
    let p1 = Sample { actionable: true, power_mul: 1.0, object_id: P1_ID, ..Sample::default() };
    let swing = |stop| Sample { attack: Some(Move::Jab), status: ATTACK, active: true, stop, powers, ..p1 };
    for before in [995.0, spots::DAMAGE_MAX] {
        let calm = Sample { actionable: true, damage: before, ..Sample::default() };
        let hurt = Sample { damage: spots::DAMAGE_MAX, dealt: 13.0 * M / spots::TARGET_MUL, ..struck_by(calm, P1_ID, &powers, 1, 13.0 * M, before) };
        let mut t = Tracker::new();
        t.step(p1, calm, NONE);
        t.step(swing(false), calm, NONE);
        t.step(swing(true), hurt, NONE);
        assert_eq!(t.marks().collect::<Vec<_>>(), vec![Mark { frame: 3, lane: 1, spot: Spot::Sour }], "from {before}%");
    }
}

/// Three frames: both calm (the victim at 20%), the attacker swings, the attacker's slot `slot` hits (hitlag) and the victim, hurt, has
/// `gained` more damage. The labelled hits.
fn marks_of(attacker_is_p1: bool, powers: &[f32], slot: usize, gained: f32) -> Vec<Mark> {
    marks_charged(attacker_is_p1, powers, 1.0, slot, gained)
}

/// marks_of with the attacker's AttackModule::power_mul_status at mul (a smash charged to that multiplier).
fn marks_charged(attacker_is_p1: bool, powers: &[f32], mul: f32, slot: usize, gained: f32) -> Vec<Mark> {
    let id = if attacker_is_p1 { P1_ID } else { CPU_ID };
    let calm = Sample { actionable: true, damage: 20.0, power_mul: mul, object_id: id, ..Sample::default() };
    let swing = |stop| Sample { attack: Some(Move::Jab), status: ATTACK, active: true, stop, powers: p(powers), ..calm };
    let hurt = struck_by(Sample { object_id: id ^ 0x30, ..Sample::default() }, id, &p(powers), slot, gained, 20.0);
    let mut t = Tracker::new();
    for frame in 1..=3 {
        let (attacker, victim) = match frame {
            1 => (calm, calm),
            2 => (swing(false), calm),
            _ => (swing(true), hurt),
        };
        let (p1, cpu) = if attacker_is_p1 { (attacker, victim) } else { (victim, attacker) };
        t.step(p1, cpu, NONE);
    }
    t.marks().collect()
}

/// Decision SP3: a connecting hit is labelled with the spot it landed, on the lane of the fighter it hit, the CPU or P1.
/// A fresh hit adds 1.26 times its hitbox's damage (Eden, 2026-11-14).
const M: f32 = 1.26;

fn a_hit_is_labelled_with_its_spot_on_the_lane_of_the_fighter_it_hit() {
    let tip_and_blade = [13.0, 13.0, 13.0, 18.0];
    let mark = |lane, spot| vec![Mark { frame: 3, lane, spot }];
    assert_eq!(marks_of(true, &tip_and_blade, 3, 18.0 * M), mark(1, Spot::Sweet), "P1's hit is on the lane of the CPU it hit");
    assert_eq!(marks_of(true, &tip_and_blade, 0, 13.0 * M), mark(1, Spot::Sour));
    assert_eq!(marks_of(false, &tip_and_blade, 0, 13.0 * M), mark(0, Spot::Sour), "the CPU's hit is on P1's lane");
    assert_eq!(marks_of(true, &tip_and_blade, 3, 18.0 * 0.6), mark(1, Spot::Sweet), "a staled tip");
}

/// A smash charged by the game's own multiplier (`AttackModule::power_mul_status`): the label follows the hitbox that dealt the hit, not the charge.
fn a_charged_hit_is_labelled_by_the_hitbox_that_dealt_it() {
    let tip_and_blade = [13.0, 13.0, 13.0, 18.0];
    let mark = |lane, spot| vec![Mark { frame: 3, lane, spot }];
    for charge in [1.0, 1.2, 1.4] {
        assert_eq!(marks_charged(true, &tip_and_blade, charge, 0, 13.0 * charge * M), mark(1, Spot::Sour), "a blade hit at charge {charge}");
        assert_eq!(marks_charged(true, &tip_and_blade, charge, 3, 18.0 * charge * M), mark(1, Spot::Sweet), "a tip hit at charge {charge}");
    }
    assert_eq!(marks_charged(false, &tip_and_blade, 1.4, 0, 13.0 * 1.4 * M), mark(0, Spot::Sour), "the CPU's charged hit is on P1's lane");
    assert!(marks_charged(true, &tip_and_blade, 0.0, 0, 13.0 * 1.4 * M).is_empty(), "a multiplier that was not read: no label");
}
/// Never a guess: equal hitboxes, a hit that added no damage (a shield), a trade.
fn a_hit_that_cannot_be_told_is_not_labelled() {
    assert!(marks_of(true, &[12.0, 12.0, 12.0], 1, 12.0 * M).is_empty(), "equal hitboxes");
    assert!(marks_of(true, &[16.0, 13.0], 1, 0.0).is_empty(), "no damage: a shield");
    let calm = Sample { actionable: true, damage: 20.0, ..Sample::default() };
    let hit = Sample { active: true, powers: p(&[16.0, 13.0]), ..struck_by(calm, P1_ID, &p(&[16.0, 13.0]), 1, 16.0, 20.0) };
    let mut t = Tracker::new();
    t.step(calm, calm, NONE);
    t.step(hit, hit, NONE);
    assert!(t.marks().next().is_none(), "both fighters hurt on one frame: whose hit?");
}

fn grade_tag_on_attacker_lane() {
    let contact = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    let (_, taken) = receive(contact, 1);
    let (p1, cpu) = samples(taken, false);
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    tracker.step(p1, cpu, NONE);
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), vec![Tag { frame: 2, lane: 0, grade: Grade::Good }]);
}

fn projectile_contact_is_captured() {
    let contact = capture_contact(&contact_fixture(1, 0x1000_0000, 0x3a6)).unwrap();
    let (_, taken) = receive(contact, 1);
    let (p1, cpu) = samples(taken, false);
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    tracker.step(p1, cpu, NONE);
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), vec![Tag { frame: 2, lane: 0, grade: Grade::Good }]);
}

fn no_grade_on_shield() {
    // The hit handler is not called for a shield hit (Eden 2026-11-20).
    let contact = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    let (_, taken) = receive(contact, 1);
    let (p1, mut cpu) = samples(taken, false);
    cpu.shieldstun = true;
    cpu.shielding = true;
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    tracker.step(p1, cpu, NONE);
    assert!(tracker.tags().next().is_none());
}

fn p2_lane_graded_separately() {
    let contact = capture_contact(&contact_fixture(0, CPU_ID, 1)).unwrap();
    let (_, taken) = receive(contact, 1);
    let (p1, cpu) = samples(taken, true);
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    tracker.step(p1, cpu, NONE);
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), vec![Tag { frame: 2, lane: 1, grade: Grade::Good }]);

    let (p1, cpu) = samples(taken, false);
    let mut cpu_only = Tracker::new();
    cpu_only.step(idle(), idle(), NONE);
    cpu_only.step(p1, cpu, NONE);
    assert!(cpu_only.tags().next().is_none(), "a CPU-operated lane is not P2");
}

fn simultaneous_hits_tag_both_lanes() {
    let p1_hit = capture_contact(&contact_fixture(0, CPU_ID, 1)).unwrap();
    let cpu_hit = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    let rings = Rings::new();
    rings.received[0].push(1, p1_hit);
    rings.received[1].push(1, cpu_hit);
    let (p1, cpu) = samples(rings.take(1), true);
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    tracker.step(p1, cpu, NONE);
    let mut tags = tracker.tags().copied().collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.lane);
    assert_eq!(
        tags,
        vec![
            Tag { frame: 2, lane: 0, grade: Grade::Good },
            Tag { frame: 2, lane: 1, grade: Grade::Good },
        ]
    );
}

fn grader_off_marks_nothing() {
    let fixture = contact_fixture(1, P1_ID, 1);
    assert!(spacing::hook(false, |at| fixture.memory.get(&at).copied(), fixture.module, fixture.entry, fixture.lanes).is_none());
    let rings = Rings::new();
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    let (p1, cpu) = samples(rings.take(1), false);
    tracker.step(p1, cpu, NONE);
    assert!(tracker.tags().next().is_none());
}

fn each_hit_graded_once() {
    let contact = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    let (_, taken) = receive(contact, 1);
    let (p1, cpu) = samples(taken, false);
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    tracker.step(p1, cpu, NONE);
    let expected = vec![Tag { frame: 2, lane: 0, grade: Grade::Good }];
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), expected);

    let contact = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    let (rings, taken) = receive(contact, 2);
    let (p1, mut cpu) = samples(taken, false);
    cpu.stop = true;
    cpu.hitstun = true;
    cpu.damage = 1.0;
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    tracker.step(p1, cpu, NONE);
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), expected);
    for epoch in 3..=4 {
        let (p1, mut cpu) = samples(rings.take(epoch), false);
        cpu.stop = true;
        cpu.hitstun = true;
        cpu.damage = 1.0;
        tracker.step(p1, cpu, NONE);
    }
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), expected);
}

fn same_slot_rehit_after_idle_frames() {
    let first = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    let rings = Rings::new();
    rings.received[1].push(1, first);
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    let (p1, cpu) = samples(rings.take(1), false);
    tracker.step(p1, cpu, NONE);
    for epoch in 2..=4 {
        let (p1, cpu) = samples(rings.take(epoch), false);
        tracker.step(p1, cpu, NONE);
    }
    let second = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    rings.received[1].push(5, second);
    let (p1, cpu) = samples(rings.take(5), false);
    tracker.step(p1, cpu, NONE);
    assert_eq!(
        tracker.tags().map(|tag| tag.frame).collect::<Vec<_>>(),
        vec![6, 2]
    );
}

fn vanishing_projectile_keeps_grade() {
    let fixture = contact_fixture(1, 0x1000_0000, 1);
    let contact = capture_contact(&fixture).unwrap();
    drop(fixture);
    let (_, taken) = receive(contact, 1);
    let (p1, cpu) = samples(taken, false);
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    tracker.step(p1, cpu, NONE);
    assert_eq!(tracker.tags().next().map(|tag| tag.grade), Some(Grade::Good));
}

fn stale_contact_dropped() {
    let contact = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    let rings = Rings::new();
    rings.received[1].push(4, contact);
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    let (p1, cpu) = samples(rings.take(5), false);
    tracker.step(p1, cpu, NONE);
    assert!(tracker.tags().next().is_none());
}

fn target_grade_coexists_with_fighter_hit() {
    let target = Contact { attacker: P1_ID, receiver: 0, slot: 0, part: 0, pen: 2.0, radius: 4.0 };
    let fighter = Contact { attacker: P1_ID, receiver: 1, slot: 2, part: 1, pen: 1.0, radius: 4.0 };
    let rings = Rings::new();
    rings.targets.push(1, target);
    rings.received[1].push(1, fighter);
    let taken = rings.take(1);
    let (p1, cpu) = samples(taken, false);
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    tracker.step(p1, cpu, NONE);
    if let Some(grade) = taken.target {
        tracker.tag_target(grade);
    }
    assert_eq!(
        tracker.tags().copied().collect::<Vec<_>>(),
        vec![
            Tag { frame: 2, lane: 0, grade: Grade::Good },
            Tag { frame: 2, lane: 0, grade: Grade::Excellent },
        ]
    );
}

/// SP1: a weaker hitbox than the strongest of the same action is Late; a new action starts over.
fn a_weaker_hit_later_in_the_same_action_is_late() {
    for (status, expected) in [(ATTACK, Some(Spot::Late)), (ATTACK + 1, None)] {
        let calm = Sample { actionable: true, damage: 20.0, power_mul: 1.0, object_id: P1_ID, ..Sample::default() };
        let swing = |status, stop, powers: &[f32]| Sample { attack: Some(Move::Jab), status, active: true, stop, powers: p(powers), ..calm };
        let hurt = struck_by(Sample::default(), P1_ID, &p(&[13.0]), 0, 13.0 * M, 20.0);
        let mut t = Tracker::new();
        t.step(calm, calm, NONE);
        t.step(swing(ATTACK, false, &[16.0]), calm, NONE);
        t.step(swing(status, false, &[13.0]), calm, NONE);
        t.step(swing(status, true, &[13.0]), hurt, NONE);
        assert_eq!(t.marks().next().map(|m| m.spot), expected, "status {status:#x}");
    }
}

/// SP4: the follow-up hit's spot is on its link, and a saved link keeps it.
fn a_link_remembers_the_spot_of_its_follow_up_hit() {
    for (second, slot, gained, expected) in [(&[13.0, 18.0][..], 1, 18.0 * M, Some(Spot::Sweet)), (&[13.0, 18.0], 0, 13.0 * M, Some(Spot::Sour)), (&[13.0], 0, 13.0 * M, None)] {
        let mut t = Tracker::new();
        for f in 1..=26u32 {
            let (p1, inputs) = match f {
                1 | 2 | 20 => (idle(), NONE),
                3..=6 => (attack(Move::Jab, 0x27, (f - 3) as f32), if f == 3 { A } else { NONE }),
                7..=10 => (Sample { stop: true, active: true, powers: p(&[3.0]), ..attack(Move::Jab, 0x27, 4.0) }, NONE),
                11..=19 => (Sample { active: f == 11, ..attack(Move::Jab, 0x27, (f - 6) as f32) }, NONE),
                21..=25 => (attack(Move::UpTilt, 0x2b, (f - 21) as f32), if f == 21 { A } else { NONE }),
                _ => (Sample { stop: true, active: true, powers: p(second), ..attack(Move::UpTilt, 0x2b, 5.0) }, NONE),
            };
            let cpu = match f {
                1..=6 => idle(),
                7..=10 => Sample { stop: true, hitstun: true, damage: 3.0, reaction: Some(18.0), ..Sample::default() },
                11..=25 => Sample { hitstun: true, damage: 3.0, reaction: Some(18.0 - (f - 11) as f32), ..Sample::default() },
                _ => Sample { damage: 3.0 + gained, reaction: Some(30.0), ..struck_by(Sample::default(), P1_ID, &p(second), slot, gained, 3.0) },
            };
            t.step(p1, cpu, inputs);
        }
        let link = t.latest_link().expect("the up tilt linked");
        assert_eq!(link.spot, expected, "gained {gained}");
        assert_eq!(crate::model::combo::LinkRec::of(&link, &t, 0).spot, expected, "the saved link keeps it");
    }
}

fn the_spot_labels_row_reads_its_setting() {
    let mut snapshot = crate::model::settings::Snapshot::factory();
    assert!(!Settings::from_snapshot(&snapshot).spots, "Off by default");
    snapshot.set(spots::ROW_STRIP, &[1, 0]).unwrap();
    assert!(Settings::from_snapshot(&snapshot).spots);
    assert!(Settings::from_snapshot(&snapshot).on, "the strip itself is unchanged");
}

fn spacing_grader_default_off() {
    let mut snapshot = crate::model::settings::Snapshot::factory();
    assert_eq!(snapshot.get(crate::model::spacing::ROW), Some(&[0, 1][..]));
    assert!(!Settings::from_snapshot(&snapshot).grader);
    snapshot.set(crate::model::spacing::ROW, &[1, 0]).unwrap();
    assert!(Settings::from_snapshot(&snapshot).grader);
}

fn strip_samples_while_grader_on() {
    let settings = Settings { on: false, grader: true, ..Settings::factory() };
    assert!(settings.samples(false));
    let off = Settings { on: false, grader: false, ..Settings::factory() };
    assert!(off.samples(true), "Parry keeps sampling while the strip is Off");
    assert!(!off.samples(false));
    assert!(strip_owns_advantage(true, settings.samples(false), true));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("freeze_after_quiet_frames_and_move_again_on_a_press", freeze_after_quiet_frames_and_move_again_on_a_press as fn()),
        ("bin_stick_eight_directions_and_the_dead_zone", bin_stick_eight_directions_and_the_dead_zone as fn()),
        ("the_status_predicates_use_the_resolved_constants", the_status_predicates_use_the_resolved_constants as fn()),
        ("display_toggle_flips_the_strip_row", display_toggle_flips_the_strip_row as fn()),
        ("a_hit_is_labelled_with_its_spot_on_the_lane_of_the_fighter_it_hit", a_hit_is_labelled_with_its_spot_on_the_lane_of_the_fighter_it_hit as fn()),
        ("a_hit_that_cannot_be_told_is_not_labelled", a_hit_that_cannot_be_told_is_not_labelled as fn()),
        ("a_hit_of_equal_damage_is_labelled_by_the_slot_the_game_recorded", a_hit_of_equal_damage_is_labelled_by_the_slot_the_game_recorded as fn()),
        ("a_hit_past_the_damage_cap_is_labelled_by_the_records_damage", a_hit_past_the_damage_cap_is_labelled_by_the_records_damage as fn()),
        ("a_charged_hit_is_labelled_by_the_hitbox_that_dealt_it", a_charged_hit_is_labelled_by_the_hitbox_that_dealt_it as fn()),
        ("grade_tag_on_attacker_lane", grade_tag_on_attacker_lane as fn()),
        ("projectile_contact_is_captured", projectile_contact_is_captured as fn()),
        ("no_grade_on_shield", no_grade_on_shield as fn()),
        ("p2_lane_graded_separately", p2_lane_graded_separately as fn()),
        ("simultaneous_hits_tag_both_lanes", simultaneous_hits_tag_both_lanes as fn()),
        ("grader_off_marks_nothing", grader_off_marks_nothing as fn()),
        ("each_hit_graded_once", each_hit_graded_once as fn()),
        ("same_slot_rehit_after_idle_frames", same_slot_rehit_after_idle_frames as fn()),
        ("vanishing_projectile_keeps_grade", vanishing_projectile_keeps_grade as fn()),
        ("stale_contact_dropped", stale_contact_dropped as fn()),
        ("target_grade_coexists_with_fighter_hit", target_grade_coexists_with_fighter_hit as fn()),
        ("a_weaker_hit_later_in_the_same_action_is_late", a_weaker_hit_later_in_the_same_action_is_late as fn()),
        ("a_link_remembers_the_spot_of_its_follow_up_hit", a_link_remembers_the_spot_of_its_follow_up_hit as fn()),
        ("the_spot_labels_row_reads_its_setting", the_spot_labels_row_reads_its_setting as fn()),
        ("spacing_grader_default_off", spacing_grader_default_off as fn()),
        ("strip_samples_while_grader_on", strip_samples_while_grader_on as fn()),
    ]);
}
}
