//! SPEC-input-06 §2 arithmetic and §3.5 behaviour, frame by frame. Frame numbers are the tracker's own: the first
//! `step` is frame 1. Statuses are arbitrary numbers: the model only compares them.

lib_unit_tests! {
use super::*;

const ATTACK: i32 = 0x27;
const A: MappedInputs = MappedInputs { buttons: 0x1, lstick_x: 0, lstick_y: 0, rstick_x: 0, rstick_y: 0 };
const NONE: MappedInputs = MappedInputs::NEUTRAL;

fn idle() -> Sample {
    Sample { actionable: true, ..Sample::default() }
}

fn attack(mv: Move, status: i32, motion_frame: f32) -> Sample {
    Sample { attack: Some(mv), status, motion_frame, ..Sample::default() }
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

/// The damages of hitboxes 0.. (the rest inactive).
fn p(values: &[f32]) -> Powers {
    let mut out = [0.0; spots::SLOTS];
    out[..values.len()].copy_from_slice(values);
    out
}

/// Three frames: both calm (the victim at 20%), the attacker swings, the attacker hits (hitlag) and the victim, hurt, has
/// `gained` more damage. The labelled hits.
fn marks_of(attacker_is_p1: bool, powers: &[f32], gained: f32) -> Vec<Mark> {
    let calm = Sample { actionable: true, damage: 20.0, ..Sample::default() };
    let swing = |stop| Sample { attack: Some(Move::Jab), status: ATTACK, active: true, stop, powers: p(powers), ..calm };
    let hurt = Sample { stop: true, hitstun: true, damage: 20.0 + gained, ..Sample::default() };
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
    assert_eq!(marks_of(true, &tip_and_blade, 18.0 * M), mark(1, Spot::Sweet), "P1's hit is on the lane of the CPU it hit");
    assert_eq!(marks_of(true, &tip_and_blade, 13.0 * M), mark(1, Spot::Sour));
    assert_eq!(marks_of(false, &tip_and_blade, 13.0 * M), mark(0, Spot::Sour), "the CPU's hit is on P1's lane");
    assert_eq!(marks_of(true, &tip_and_blade, 13.0 * 1.15), mark(1, Spot::Sour), "a staled hit");
}

/// Never a guess: equal hitboxes, a hit two hitboxes could have made, a hit that added no damage (a shield), a trade.
fn a_hit_that_cannot_be_told_is_not_labelled() {
    assert!(marks_of(true, &[12.0, 12.0, 12.0], 12.0 * M).is_empty(), "equal hitboxes");
    assert!(marks_of(true, &[16.0, 15.0], 15.5 * M).is_empty(), "two candidates of different classes");
    assert!(marks_of(true, &[16.0, 13.0], 0.0).is_empty(), "no damage: a shield");
    let calm = Sample { actionable: true, damage: 20.0, ..Sample::default() };
    let hit = Sample { stop: true, hitstun: true, active: true, powers: p(&[16.0, 13.0]), damage: 36.0, ..Sample::default() };
    let mut t = Tracker::new();
    t.step(calm, calm, NONE);
    t.step(hit, hit, NONE);
    assert!(t.marks().next().is_none(), "both fighters hurt on one frame: whose hit?");
}

/// SP1: a weaker hitbox than the strongest of the same action is Late; a new action starts over.
fn a_weaker_hit_later_in_the_same_action_is_late() {
    for (status, expected) in [(ATTACK, Some(Spot::Late)), (ATTACK + 1, None)] {
        let calm = Sample { actionable: true, damage: 20.0, ..Sample::default() };
        let swing = |status, stop, powers: &[f32]| Sample { attack: Some(Move::Jab), status, active: true, stop, powers: p(powers), ..calm };
        let hurt = Sample { stop: true, hitstun: true, damage: 20.0 + 13.0 * M, ..Sample::default() };
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
    for (second, gained, expected) in [(&[13.0, 18.0][..], 18.0 * M, Some(Spot::Sweet)), (&[13.0, 18.0], 13.0 * M, Some(Spot::Sour)), (&[13.0], 13.0 * M, None)] {
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
                _ => Sample { stop: true, hitstun: true, damage: 3.0 + gained, reaction: Some(30.0), ..Sample::default() },
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
        ("a_weaker_hit_later_in_the_same_action_is_late", a_weaker_hit_later_in_the_same_action_is_late as fn()),
        ("a_link_remembers_the_spot_of_its_follow_up_hit", a_link_remembers_the_spot_of_its_follow_up_hit as fn()),
        ("the_spot_labels_row_reads_its_setting", the_spot_labels_row_reads_its_setting as fn()),
    ]);
}
}
