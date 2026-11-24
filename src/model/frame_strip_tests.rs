//! SPEC-input-06 §2 arithmetic and §3.5 behaviour, frame by frame. Frame numbers are the tracker's own: the first
//! `step` is frame 1. Statuses are arbitrary numbers: the model only compares them.

lib_unit_tests! {
use super::*;
use crate::model::spacing::{self, Grade, Grader, Lane, Taken};
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
    lanes: [Option<Lane>; 2],
}

fn contact_fixture(receiver: usize, attacker: u32, entry_kind: u32) -> ContactFixture {
    use crate::model::hitbox::layout;
    use crate::model::sites_hitbox::{damage_record, hurt_shape};

    let (module, entry, lanes, hit_module, parts, shape) =
        (0x20000, 0x30000, [0x10000, 0x11000], 0x40000, 0x50000, 0x60000);
    let mut memory = BTreeMap::new();
    let published = core::array::from_fn(|lane| {
        let damage = if lane == receiver { module } else { 0x21000 + lane * 0x1000 };
        let hit = if lane == receiver { hit_module } else { 0x41000 + lane * 0x1000 };
        memory.insert(lanes[lane] + damage_record::MODULE, damage as u64);
        memory.insert(lanes[lane] + layout::HIT_MODULE, hit as u64);
        Some(Lane { damage, hit, id: if lane == 0 { P1_ID } else { CPU_ID }, attacks: lane == 0 })
    });
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
    ContactFixture { memory, module, entry, lanes: published }
}

fn capture_contact(fixture: &ContactFixture) -> Option<(u8, Grade)> {
    spacing::capture(
        |at| fixture.memory.get(&at).copied(),
        |id| match id { 0x1000_0000 => Some(P1_ID), 0x1000_0001 => Some(CPU_ID), _ => None },
        fixture.module,
        fixture.entry,
        fixture.lanes,
    )
}

fn take_hit(hit: Option<(u8, Grade)>) -> Taken {
    let grader = Grader::new();
    if let Some((lane, grade)) = hit {
        grader.hit(lane, grade);
    }
    grader.take()
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
    let taken = take_hit(capture_contact(&contact_fixture(1, P1_ID, 1)));
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, Taken::default());
    tracker.step_graded(idle(), idle(), NONE, taken);
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), vec![Tag { frame: 2, lane: 0, grade: Grade::Good }]);
}

fn projectile_contact_is_captured() {
    let taken = take_hit(capture_contact(&contact_fixture(1, 0x1000_0000, 0x3a6)));
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, Taken::default());
    tracker.step_graded(idle(), idle(), NONE, taken);
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), vec![Tag { frame: 2, lane: 0, grade: Grade::Good }]);
}

fn no_grade_on_shield() {
    // The hit handler is not called for a shield hit (Eden 2026-11-20).
    let taken = take_hit(capture_contact(&contact_fixture(1, P1_ID, 1)));
    let mut cpu = idle();
    cpu.shieldstun = true;
    cpu.shielding = true;
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, Taken::default());
    tracker.step_graded(idle(), cpu, NONE, taken);
    assert!(tracker.tags().next().is_none());
}

fn p2_lane_graded_separately() {
    let mut fixture = contact_fixture(0, CPU_ID, 1);
    fixture.lanes[1].as_mut().unwrap().attacks = true;
    let taken = take_hit(capture_contact(&fixture));
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, Taken::default());
    tracker.step_graded(idle(), idle(), NONE, taken);
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), vec![Tag { frame: 2, lane: 1, grade: Grade::Good }]);

    let taken = take_hit(capture_contact(&contact_fixture(0, CPU_ID, 1)));
    let mut cpu_only = Tracker::new();
    cpu_only.step_graded(idle(), idle(), NONE, Taken::default());
    cpu_only.step_graded(idle(), idle(), NONE, taken);
    assert!(cpu_only.tags().next().is_none(), "a CPU-operated lane is not P2");
}

fn simultaneous_hits_tag_both_lanes() {
    let mut cpu_attack = contact_fixture(0, CPU_ID, 1);
    cpu_attack.lanes[1].as_mut().unwrap().attacks = true;
    let p1_hit = capture_contact(&cpu_attack).unwrap();
    let cpu_hit = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    let grader = Grader::new();
    grader.hit(p1_hit.0, p1_hit.1);
    grader.hit(cpu_hit.0, cpu_hit.1);
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, Taken::default());
    tracker.step_graded(idle(), idle(), NONE, grader.take());
    let mut tags = tracker.tags().copied().collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.lane);
    assert_eq!(tags, vec![
        Tag { frame: 2, lane: 0, grade: Grade::Good },
        Tag { frame: 2, lane: 1, grade: Grade::Good },
    ]);
}

fn grader_off_marks_nothing() {
    let fixture = contact_fixture(1, P1_ID, 1);
    assert!(spacing::hook(false, |_| panic!("Off reads nothing"), |_| panic!("Off does not resolve owners"), fixture.module, fixture.entry, fixture.lanes).is_none());
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, Taken::default());
    assert!(tracker.tags().next().is_none());
}

fn each_hit_graded_once() {
    let taken = take_hit(capture_contact(&contact_fixture(1, P1_ID, 1)));
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, Taken::default());
    tracker.step_graded(idle(), idle(), NONE, taken);
    let expected = vec![Tag { frame: 2, lane: 0, grade: Grade::Good }];
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), expected);

    let taken = take_hit(capture_contact(&contact_fixture(1, P1_ID, 1)));
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, Taken::default());
    let cpu = Sample { stop: true, hitstun: true, damage: 1.0, ..Sample::default() };
    tracker.step_graded(idle(), cpu, NONE, taken);
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), expected);
    for _ in 0..2 {
        tracker.step_graded(idle(), cpu, NONE, Taken::default());
    }
    assert_eq!(tracker.tags().copied().collect::<Vec<_>>(), expected);
}

fn same_slot_rehit_after_idle_frames() {
    let grader = Grader::new();
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, grader.take());
    grader.finish(true);
    let (lane, grade) = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    grader.hit(lane, grade);
    tracker.step_graded(idle(), idle(), NONE, grader.take());
    grader.finish(true);
    for _ in 0..3 {
        tracker.step_graded(idle(), idle(), NONE, grader.take());
        grader.finish(true);
    }
    let (lane, grade) = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    grader.hit(lane, grade);
    tracker.step_graded(idle(), idle(), NONE, grader.take());
    grader.finish(true);
    assert_eq!(tracker.tags().map(|tag| tag.frame).collect::<Vec<_>>(), vec![6, 2]);
}

fn vanishing_projectile_keeps_grade() {
    let fixture = contact_fixture(1, 0x1000_0000, 1);
    let hit = capture_contact(&fixture);
    drop(fixture);
    let taken = take_hit(hit);
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, Taken::default());
    tracker.step_graded(idle(), idle(), NONE, taken);
    assert_eq!(tracker.tags().next().map(|tag| tag.grade), Some(Grade::Good));
}

fn stale_contact_dropped() {
    let grader = Grader::new();
    let (lane, grade) = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    grader.hit(lane, grade);
    grader.finish(false);
    assert_eq!(grader.take(), Taken::default());
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, grader.take());
    assert!(tracker.tags().next().is_none());
}

fn foreign_or_self_attacker_awards_nothing() {
    for (receiver, attacker) in [
        (1, 0x1000_0001),
        (1, CPU_ID),
        (1, 0x30),
        (1, 0x1000_0002),
        (0, CPU_ID),
    ] {
        let taken = take_hit(capture_contact(&contact_fixture(receiver, attacker, 1)));
        let mut tracker = Tracker::new();
        tracker.step_graded(idle(), idle(), NONE, Taken::default());
        tracker.step_graded(idle(), idle(), NONE, taken);
        assert!(tracker.tags().next().is_none(), "receiver={receiver} attacker={attacker:#x}");
    }
}

fn target_grade_coexists_with_fighter_hit() {
    let grader = Grader::new();
    let fighter = capture_contact(&contact_fixture(1, P1_ID, 1)).unwrap();
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, grader.take());
    grader.finish(true);
    grader.hit(fighter.0, fighter.1);
    grader.target(Grade::Excellent);
    let taken = grader.take();
    tracker.step_graded(idle(), idle(), NONE, taken);
    grader.finish(true);
    assert_eq!(tracker.tags().map(|tag| tag.frame).collect::<Vec<_>>(), vec![2, 1]);
}

fn target_tag_lands_on_the_break_column() {
    let grader = Grader::new();
    let mut tracker = Tracker::new();
    let taken = grader.take();
    tracker.step_graded(idle(), idle(), NONE, taken);
    grader.finish(true);
    let break_frame = tracker.frame();
    grader.target(Grade::Good);
    let taken = grader.take();
    tracker.step_graded(idle(), idle(), NONE, taken);
    grader.finish(true);
    assert_eq!(tracker.tags().next().map(|tag| tag.frame), Some(break_frame));
}

fn skipped_step_drops_target_grade() {
    let skipped = Grader::new();
    let mut skipped_tracker = Tracker::new();
    skipped_tracker.step_graded(idle(), idle(), NONE, skipped.take());
    skipped.finish(true);
    skipped.skip();
    skipped.target(Grade::Good);
    assert_eq!(skipped.take(), Taken::default());

    let busy = Grader::new();
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, busy.take());
    busy.finish(true);
    busy.target(Grade::Good);
    let _taken_while_busy = busy.take();
    busy.finish(false);
    tracker.step_graded(idle(), idle(), NONE, busy.take());
    assert!(tracker.tags().next().is_none());
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
        assert_eq!(crate::model::combo::LinkRec::of(&link, &t, 0, false).spot, expected, "the saved link keeps it");
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

fn angle_of_reads_the_angled_side_animations() {
    use crate::model::hash40::hash40;
    assert_eq!(hash40("attack_s3_hi"), 0x0cba28ee97);
    for (motion, expected) in [
        (0x0cba28ee97, Some(Angle::Up)), (0x0c27ffd62e, Some(Angle::Up)),
        (0x0c244b16f0, Some(Angle::Down)), (0x0cb99c2e49, Some(Angle::Down)),
        (0x0b7dd72b57, None), (0x0b78983dd2, None), (0, None),
    ] {
        assert_eq!(angle_of(motion), expected, "{motion:#x}");
    }
}

fn an_angled_side_tilt_marks_the_frame_its_swing_starts() {
    let mut tilt = Tracker::new();
    tilt.step(idle(), idle(), NONE);
    for frame in 0..8 {
        tilt.step(Sample { angle: Some(Angle::Up), ..attack(Move::SideTilt, ATTACK, frame as f32) }, idle(), NONE);
    }
    assert_eq!(tilt.aims().collect::<Vec<_>>(), vec![Aim { frame: 2, dir: 2 }]);
    let mut smash = Tracker::new();
    smash.step(idle(), idle(), NONE);
    for frame in 0..12 {
        let angle = (frame >= 6).then_some(Angle::Down);
        smash.step(Sample { angle, ..attack(Move::SideSmash, ATTACK, frame as f32) }, idle(), NONE);
    }
    assert_eq!(smash.aims().collect::<Vec<_>>(), vec![Aim { frame: 8, dir: 6 }]);
}

fn a_level_side_tilt_marks_nothing() {
    let mut tracker = Tracker::new();
    for frame in 1..=26u32 {
        let p1 = match frame {
            1 | 2 | 20 => idle(),
            3..=19 => Sample { active: frame == 7, ..attack(Move::Jab, 1, (frame - 3) as f32) },
            _ => Sample { active: frame == 26, angle: None, ..attack(Move::SideTilt, 2, (frame - 21) as f32) },
        };
        let cpu = match frame {
            7 | 26 => Sample { stop: true, hitstun: true, reaction: Some(30.0), ..Sample::default() },
            8..=25 => Sample { hitstun: true, reaction: Some(30.0), ..Sample::default() },
            _ => idle(),
        };
        tracker.step(p1, cpu, NONE);
    }
    assert!(tracker.aims().next().is_none());
    assert_eq!(tracker.latest_link().map(|link| link.shown[1]), Some(Shown::default()));
    assert_eq!(angle_of(0x0cba28ee97), Some(Angle::Up));
}

fn a_buffered_second_swing_of_the_same_angle_marks_again() {
    let mut tracker = Tracker::new();
    for motion in [0.0, 1.0, 2.0, 0.0, 1.0] {
        tracker.step(Sample { angle: Some(Angle::Up), ..attack(Move::SideTilt, ATTACK, motion) }, idle(), NONE);
    }
    assert_eq!(tracker.aims().collect::<Vec<_>>(),
               vec![Aim { frame: 4, dir: 2 }, Aim { frame: 1, dir: 2 }]);
}

fn heading_bins_a_movement_to_the_nearest_of_eight_directions() {
    use crate::model::aim::heading;
    for (dx, dy, expected) in [
        (-10.0, 0.0, Some(4)), (7.0, 7.0, Some(1)), (-7.07, 7.07, Some(3)),
        (0.0, -5.0, Some(6)), (-1.91, 9.82, Some(2)), (8.66, 5.0, Some(1)),
        (0.5, 0.5, None), (0.0, 0.0, None), (1.0, 0.0, Some(0)),
        (7.0, -7.0, Some(7)), (-7.0, -7.0, Some(5)),
    ] {
        assert_eq!(heading(dx, dy), expected, "({dx}, {dy})");
    }
}

fn directional_special(still: bool) -> Tracker {
    let mut tracker = Tracker::new();
    for frame in 1..=33u32 {
        let (mv, status, x) = match frame {
            1..=14 => (Move::UpSpecial, 1, 0.0),
            15..=19 => (Move::Other, 2, if still { 0.0 } else { -11.0 * (frame - 15) as f32 }),
            20..=28 => (Move::Other, 3, if still { 0.0 } else { -44.0 }),
            _ => (Move::Other, 2, if still { 0.0 } else { -44.0 + 11.0 * (frame - 29) as f32 }),
        };
        tracker.step(Sample { pos: Some((x, 0.0)), active: status == 2, ..attack(mv, status, frame as f32) },
                     idle(), MappedInputs { lstick_x: if frame < 29 { -100 } else { 100 }, ..NONE });
    }
    tracker
}

fn each_entry_into_the_travel_status_marks_its_frame_with_the_way_it_moved() {
    let tracker = directional_special(false);
    assert_eq!(tracker.aims().collect::<Vec<_>>(),
               vec![Aim { frame: 29, dir: 0 }, Aim { frame: 15, dir: 4 }]);
    assert!(directional_special(true).aims().next().is_none());
    let mut restarted = Tracker::new();
    let left = MappedInputs { lstick_x: -100, ..NONE };
    restarted.step(Sample { pos: Some((0.0, 0.0)), ..attack(Move::UpSpecial, 1, 0.0) }, idle(), left);
    for (motion, x) in [(0.0, 0.0), (1.0, -11.0), (2.0, -22.0),
                        (0.0, -33.0), (1.0, -44.0), (2.0, -55.0)] {
        restarted.step(Sample { pos: Some((x, 0.0)), ..attack(Move::Other, 2, motion) }, idle(), left);
    }
    assert_eq!(restarted.aims().collect::<Vec<_>>(), vec![Aim { frame: 2, dir: 4 }]);
    let mut pending = Tracker::new();
    pending.step(Sample { pos: Some((0.0, 0.0)), ..attack(Move::UpSpecial, 1, 0.0) }, idle(), left);
    for (motion, x, stop) in [(0.0, 0.0, false), (1.0, 0.0, true), (0.0, 0.0, false), (1.0, -11.0, false)] {
        pending.step(Sample { pos: Some((x, 0.0)), stop, ..attack(Move::Other, 2, motion) }, idle(), left);
    }
    assert_eq!(pending.aims().collect::<Vec<_>>(), vec![Aim { frame: 2, dir: 4 }]);
}

fn linked_special(travel: bool) -> Tracker {
    let mut tracker = Tracker::new();
    for frame in 1..=35u32 {
        let p1 = match frame {
            1 | 2 | 20 => idle(),
            3..=19 => Sample { active: frame == 7, ..attack(Move::Jab, 1, (frame - 3) as f32) },
            21..=34 => attack(Move::UpSpecial, 2, (frame - 21) as f32),
            _ => Sample { active: true, stop: true, ..attack(Move::Other, if travel { 3 } else { 4 }, 0.0) },
        };
        let cpu = match frame {
            7 | 35 => Sample { stop: true, hitstun: true, reaction: Some(50.0), ..Sample::default() },
            8..=34 => Sample { hitstun: true, reaction: Some(50.0), ..Sample::default() },
            _ => idle(),
        };
        tracker.step(Sample { pos: Some((0.0, 0.0)), ..p1 }, cpu, MappedInputs { lstick_x: -100, ..NONE });
    }
    tracker
}

fn a_link_keeps_both_moves_directions_and_the_special_a_travel_continues() {
    let mut tracker = linked_special(true);
    assert_eq!(tracker.latest_link().map(|l| (l.from, l.to, l.shown)),
               Some((Move::Jab, Move::Other, [Shown::default(), Shown { of: Some(Move::UpSpecial), dirs: [None; 2] }])));
    tracker.step(Sample { active: true, pos: Some((-11.0, 0.0)), ..attack(Move::Other, 3, 1.0) },
                 Sample { hitstun: true, reaction: Some(50.0), ..Sample::default() }, MappedInputs { lstick_x: -100, ..NONE });
    assert_eq!(tracker.latest_link().map(|l| (l.from, l.to, l.shown)),
               Some((Move::Jab, Move::Other, [Shown::default(), Shown { of: Some(Move::UpSpecial), dirs: [Some(4), None] }])));
    assert_eq!(tracker.aims().next(), Some(Aim { frame: 35, dir: 4 }));
    tracker.step(Sample { active: true, pos: Some((-22.0, 0.0)), ..attack(Move::Other, 3, 0.0) },
                 Sample { hitstun: true, reaction: Some(50.0), ..Sample::default() }, NONE);
    tracker.step(Sample { active: true, pos: Some((-33.0, 0.0)), ..attack(Move::Other, 3, 1.0) },
                 Sample { stop: true, hitstun: true, reaction: Some(50.0), ..Sample::default() }, NONE);
    assert_eq!(tracker.aims().collect::<Vec<_>>(), vec![Aim { frame: 35, dir: 4 }]);
    assert_eq!(tracker.latest_link().map(|l| l.shown[1]),
               Some(Shown { of: Some(Move::UpSpecial), dirs: [Some(4), None] }));
    tracker.step(Sample { pos: Some((-33.0, 0.0)), ..idle() }, Sample { hitstun: true, reaction: Some(50.0), ..Sample::default() }, NONE);
    tracker.step(Sample { active: true, pos: Some((-33.0, 0.0)), ..attack(Move::DownTilt, 4, 0.0) },
                 Sample { stop: true, hitstun: true, reaction: Some(50.0), ..Sample::default() }, NONE);
    assert_eq!(tracker.latest_link().map(|l| l.shown[0]),
               Some(Shown { of: Some(Move::UpSpecial), dirs: [Some(4), None] }));
    // A fighter-own status that continues a special is named by it, with or without a travel.
    assert_eq!(linked_special(false).latest_link().map(|l| l.shown[1]),
               Some(Shown { of: Some(Move::UpSpecial), dirs: [None; 2] }));
}

fn a_second_part_reaches_the_link_its_special_already_made() {
    use crate::model::frame_strip_plan::{folded_link, link_moves, SHORT_UNITS};
    let stun = || Sample { hitstun: true, reaction: Some(50.0), ..Sample::default() };
    let text = |line: crate::model::frame_strip_plan::Text| String::from_utf16(line.units()).unwrap();
    // Frame 35: the travel is entered and hits, so Jab links into it before any direction is known.
    let mut tracker = linked_special(true);
    // 36..=38 travel 1 goes left; 39..=41 the status between the parts; 42..=43 travel 2 goes right.
    for (motion, x) in [(1.0, -11.0), (2.0, -22.0), (3.0, -33.0)] {
        tracker.step(Sample { active: true, pos: Some((x, 0.0)), ..attack(Move::Other, 3, motion) }, stun(), MappedInputs { lstick_x: -100, ..NONE });
    }
    for motion in 0..3 {
        tracker.step(Sample { pos: Some((-33.0, 0.0)), ..attack(Move::Other, 4, motion as f32) }, stun(), MappedInputs { lstick_x: 100, ..NONE });
    }
    for (motion, x) in [(0.0, -33.0), (1.0, -22.0)] {
        tracker.step(Sample { active: true, pos: Some((x, 0.0)), ..attack(Move::Other, 3, motion) }, stun(), MappedInputs { lstick_x: 100, ..NONE });
    }
    let both = Shown { of: Some(Move::UpSpecial), dirs: [Some(4), Some(0)] };
    assert_eq!(tracker.aims().collect::<Vec<_>>(), vec![Aim { frame: 42, dir: 0 }, Aim { frame: 35, dir: 4 }]);
    let link = tracker.latest_link().expect("the jab linked into the special");
    assert_eq!((link.from, link.to, link.shown), (Move::Jab, Move::Other, [Shown::default(), both]));
    assert_eq!(text(link_moves(&link, SHORT_UNITS)), "Jab > Up B ← →");
    assert_eq!(text(folded_link(&link)), "TRUE · Jab > Up B ← →");
    // An unrelated move with a direction of its own alters neither name; when it hits, the special keeps both arrows as the link's first move.
    tracker.step(Sample { pos: Some((-22.0, 0.0)), ..idle() }, stun(), NONE);
    for motion in 0..3 {
        tracker.step(Sample { angle: Some(Angle::Up), pos: Some((-22.0, 0.0)), ..attack(Move::SideTilt, 5, motion as f32) }, stun(), NONE);
    }
    assert_eq!(tracker.aims().next(), Some(Aim { frame: 45, dir: 2 }));
    assert_eq!(tracker.latest_link().map(|l| l.shown), Some([Shown::default(), both]));
    tracker.step(Sample { active: true, angle: Some(Angle::Up), pos: Some((-22.0, 0.0)), ..attack(Move::SideTilt, 5, 3.0) },
                 Sample { stop: true, hitstun: true, reaction: Some(50.0), ..Sample::default() }, NONE);
    let links: Vec<_> = tracker.history().map(|l| (l.from, l.to, l.shown)).collect();
    assert_eq!(links[0], (Move::Other, Move::SideTilt, [both, Shown { of: None, dirs: [Some(2), None] }]));
    assert_eq!(links[1], (Move::Jab, Move::Other, [Shown::default(), both]));
    assert_eq!(text(link_moves(&tracker.latest_link().unwrap(), SHORT_UNITS)), "Up B ← → > Side tilt ↑");
}

fn the_runtime_reads_the_angle_and_the_travel_behind_their_gates() {
    let input = include_str!("../runtime/features/input_display.rs");
    assert!(input.contains("static ANGLE_ON: AtomicBool"));
    let motion = input.split("unsafe fn p1_motion(").nth(1).unwrap_or("");
    let motion = motion.split("\n}").next().unwrap_or("");
    assert!(motion.contains("ANGLE_ON.load(") && motion.contains("natives::motion_kind(accessor)"));
    assert!(input.contains("static AIM_ON: AtomicBool"));
    let position = input.split("unsafe fn p1_position(").nth(1).unwrap_or("");
    let position = position.split("\n}").next().unwrap_or("");
    assert!(position.contains("AIM_ON.load(") && !position.contains("p1_fighter")
            && position.contains("natives::pos_x(accessor)") && position.contains("natives::pos_y(accessor)"));
    let sample = input.split("unsafe fn sample(").nth(1).unwrap_or("");
    let sample = sample.split("\n}").next().unwrap_or("");
    assert!(sample.contains("named,") && sample.contains("motion,"));
    assert!(sample.contains("angle: if matches!(attack, Some(Move::SideTilt | Move::SideSmash)) { angle_of(motion) } else { None },"));
    assert!(sample.contains("pos: if p1 { p1_position(accessor) } else { None },"));
    assert!(!sample.contains("get_int"));
    let boot = include_str!("../runtime/boot.rs");
    assert!(boot.contains("set_angle_read(display_on && capabilities::unit_usable(Feature::CpuMash, &usable))"));
    assert!(boot.contains("set_aim_read(display_on && capabilities::unit_usable(Feature::Recording, &usable))"));
    let hud = include_str!("../runtime/features/hud_draw.rs");
    assert!(hud.contains("bridge_state::note_strip_aims(aims)") && hud.contains("bridge_state::note_strip_aims([None; 2])"));
    assert!(hud.contains("let labels = [strip[iv::at::RUN_LABEL] != 0, strip[iv::at::RUN_LABEL + 1] != 0];"));
    assert!(input.contains("aims_drawn(&tracker, labels)") && !input.contains("aims_shown("));
}

fn the_action_takes_its_origin_when_it_opens() {
    let mut tracker = Tracker::new();
    let stun = Sample { hitstun: true, reaction: Some(50.0), ..Sample::default() };
    let hit = Sample { stop: true, ..stun };
    tracker.step(attack(Move::Jab, 1, 0.0), hit, NONE);
    for frame in 2..=6 {
        tracker.step(idle(), stun, if frame == 5 { A } else { NONE });
    }
    tracker.step(attack(Move::SideTilt, 2, 0.0), hit, NONE);
    assert_eq!(tracker.origin(), Some(5));
    assert_eq!(tracker.latest_link().unwrap().press, 5);

    let mut old = Tracker::new();
    old.step(idle(), idle(), A);
    for _ in 2..=16 {
        old.step(idle(), idle(), NONE);
    }
    old.step(attack(Move::Jab, 1, 0.0), idle(), NONE);
    assert_eq!(old.origin(), None);
}

fn a_throw_stays_in_the_action_of_its_grab() {
    use super::ids::{Table, C};
    let t = Table::with(&[(C::Catch, 0x30), (C::CatchAttack, 0x36), (C::Throw, 0x37), (C::Final, 0x1e0)]);
    assert_eq!(t.named_move(0x30), Some(Move::Grab));
    assert_eq!(t.named_move(0x36), Some(Move::Grab));
    assert_eq!(t.named_move(0x37), Some(Move::Throw));
    assert_eq!(t.named_move(0x1e0), Some(Move::FinalSmash));
    assert_eq!(t.named_move(0x1e1), None);
    let mut tracker = Tracker::new();
    let stun = Sample { hitstun: true, reaction: Some(50.0), ..Sample::default() };
    let hit = Sample { stop: true, ..stun };
    let grab = Sample { named: Some(Move::Grab), status: 0x30, ..Sample::default() };
    let throw = Sample { named: Some(Move::Throw), status: 0x37, ..Sample::default() };
    tracker.step(grab, hit, NONE);
    tracker.step(grab, stun, NONE);
    tracker.step(throw, hit, NONE);
    assert!(tracker.latest_link().is_none());
    tracker.step(idle(), stun, NONE);
    tracker.step(attack(Move::Jab, 1, 0.0), hit, NONE);
    let link = tracker.latest_link().unwrap();
    assert_eq!((link.from, link.to), (Move::Throw, Move::Jab));
}

fn any_fighters_special_can_aim() {
    let mut tracker = Tracker::new();
    let stick = MappedInputs { lstick_x: -100, lstick_y: 100, ..NONE };
    for motion in 0..12 {
        tracker.step(Sample { pos: Some((0.0, 0.0)), ..attack(Move::SideSpecial, 1, motion as f32) }, idle(), stick);
    }
    let entry = tracker.frame() + 1;
    for motion in 0..4 {
        tracker.step(Sample { pos: Some((-4.0 * (motion + 1) as f32, 4.0 * (motion + 1) as f32)),
            ..attack(Move::Other, 2, motion as f32) }, idle(), stick);
    }
    assert_eq!(tracker.aims().collect::<Vec<_>>(), vec![Aim { frame: entry, dir: 3 }]);
    assert!(tracker.snaps().any(|snap| snap.frame >= entry && snap.aim.part == 1));
    assert!(tracker.snaps().all(|snap| snap.aim.inside));
}

fn no_fighter_table_is_left() {
    let aim = include_str!("aim.rs");
    for word in ["TRAVEL", "travel_status", "kind", "0x1e"] { assert!(!aim.contains(word), "{word}"); }
    let input = include_str!("../runtime/features/input_display.rs");
    let position = input.split("unsafe fn p1_position(").nth(1).unwrap_or("").split("\n}").next().unwrap_or("");
    assert!(position.contains("AIM_ON.load(") && position.contains("natives::pos_x(accessor)") && !position.contains("p1_fighter"));
    assert!(!input.contains("aimer") && !input.contains("travel_status"));
}

fn the_tracker_reports_an_action_when_it_opens() {
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), NONE);
    tracker.step(idle(), idle(), NONE);
    tracker.step(idle(), idle(), A);
    tracker.step(attack(Move::Jab, 1, 0.0), idle(), NONE);
    assert_eq!(tracker.late().open, Some((Some(3), None)));
    tracker.step(attack(Move::Jab, 1, 1.0), idle(), NONE);
    assert_eq!(tracker.late().open, None);

    let mut smash = Tracker::new();
    smash.step(idle(), idle(), A);
    smash.step(attack(Move::SideSmash, 2, 0.0), idle(), NONE);
    assert_eq!(smash.late().open, Some((Some(1), Some(None))));
    smash.step(attack(Move::SideSmash, 2, 1.0), idle(), NONE);
    assert_eq!(smash.late().open, None);
    smash.step(Sample { angle: Some(Angle::Down), ..attack(Move::SideSmash, 2, 2.0) }, idle(), NONE);
    assert_eq!(smash.late().open, Some((Some(1), Some(Some(Angle::Down)))));
}

fn the_tracker_hands_over_a_travel_when_its_part_ends() {
    let mut tracker = Tracker::new();
    let stick = MappedInputs { lstick_x: -100, lstick_y: 100, ..NONE };
    for motion in 0..12 {
        tracker.step(Sample { pos: Some((0.0, 0.0)), facing: 1, ..attack(Move::SideSpecial, 1, motion as f32) }, idle(), stick);
    }
    for motion in 0..4 {
        tracker.step(Sample { pos: Some((-4.0 * (motion + 1) as f32, 4.0 * (motion + 1) as f32)),
            facing: 1, ..attack(Move::Other, 2, motion as f32) }, idle(), stick);
        assert_eq!(tracker.late().travel, None);
    }
    tracker.step(Sample { pos: Some((-16.0, 16.0)), facing: 1, ..attack(Move::Other, 3, 0.0) }, idle(), stick);
    let (part, travel) = tracker.late().travel.unwrap();
    assert_eq!((part, travel.heading), (1, 1350));
    tracker.step(Sample { pos: Some((-16.0, 16.0)), facing: 1, ..attack(Move::Other, 3, 1.0) }, idle(), stick);
    assert_eq!(tracker.late().travel, None);
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
        ("foreign_or_self_attacker_awards_nothing", foreign_or_self_attacker_awards_nothing as fn()),
        ("target_grade_coexists_with_fighter_hit", target_grade_coexists_with_fighter_hit as fn()),
        ("target_tag_lands_on_the_break_column", target_tag_lands_on_the_break_column as fn()),
        ("skipped_step_drops_target_grade", skipped_step_drops_target_grade as fn()),
        ("a_weaker_hit_later_in_the_same_action_is_late", a_weaker_hit_later_in_the_same_action_is_late as fn()),
        ("a_link_remembers_the_spot_of_its_follow_up_hit", a_link_remembers_the_spot_of_its_follow_up_hit as fn()),
        ("the_spot_labels_row_reads_its_setting", the_spot_labels_row_reads_its_setting as fn()),
        ("spacing_grader_default_off", spacing_grader_default_off as fn()),
        ("strip_samples_while_grader_on", strip_samples_while_grader_on as fn()),
        ("angle_of_reads_the_angled_side_animations", angle_of_reads_the_angled_side_animations as fn()),
        ("an_angled_side_tilt_marks_the_frame_its_swing_starts", an_angled_side_tilt_marks_the_frame_its_swing_starts as fn()),
        ("a_level_side_tilt_marks_nothing", a_level_side_tilt_marks_nothing as fn()),
        ("a_buffered_second_swing_of_the_same_angle_marks_again", a_buffered_second_swing_of_the_same_angle_marks_again as fn()),
        ("heading_bins_a_movement_to_the_nearest_of_eight_directions", heading_bins_a_movement_to_the_nearest_of_eight_directions as fn()),
        ("each_entry_into_the_travel_status_marks_its_frame_with_the_way_it_moved", each_entry_into_the_travel_status_marks_its_frame_with_the_way_it_moved as fn()),
        ("a_link_keeps_both_moves_directions_and_the_special_a_travel_continues", a_link_keeps_both_moves_directions_and_the_special_a_travel_continues as fn()),
        ("a_second_part_reaches_the_link_its_special_already_made", a_second_part_reaches_the_link_its_special_already_made as fn()),
        ("the_action_takes_its_origin_when_it_opens", the_action_takes_its_origin_when_it_opens as fn()),
        ("a_throw_stays_in_the_action_of_its_grab", a_throw_stays_in_the_action_of_its_grab as fn()),
        ("any_fighters_special_can_aim", any_fighters_special_can_aim as fn()),
        ("no_fighter_table_is_left", no_fighter_table_is_left as fn()),
        ("the_tracker_reports_an_action_when_it_opens", the_tracker_reports_an_action_when_it_opens as fn()),
        ("the_tracker_hands_over_a_travel_when_its_part_ends", the_tracker_hands_over_a_travel_when_its_part_ends as fn()),
        ("the_runtime_reads_the_angle_and_the_travel_behind_their_gates", the_runtime_reads_the_angle_and_the_travel_behind_their_gates as fn()),
    ]);
}
}
