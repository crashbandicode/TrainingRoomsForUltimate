//! Parry practice detector (`artifacts/rc15-parry/DESIGN.md` §3).

lib_unit_tests! {
use super::*;

const IDLE: Frame = Frame { stop: false, gained: false, guard_damage: false, guard_off: false, just_shield: None, just_frame: None, shield_held: false, armed: true };
const HELD: Frame = Frame { shield_held: true, ..IDLE };
const OFF: Frame = Frame { guard_off: true, just_frame: Some(5), ..IDLE };
const PARRY: Frame = Frame { stop: true, guard_damage: true, just_shield: Some(true), ..IDLE };
const SHIELD_HIT: Frame = Frame { stop: true, guard_damage: true, just_shield: Some(false), shield_held: true, ..IDLE };
const BODY_HIT: Frame = Frame { stop: true, gained: true, ..IDLE };

fn run(frames: &[Frame]) -> Vec<Attempt> {
    let mut d = Detector::new();
    frames.iter().zip(1..).flat_map(|(f, n)| d.step(f, n)).flatten().collect()
}

fn grades(frames: &[Frame]) -> Vec<Grade> {
    run(frames).iter().map(|a| a.grade).collect()
}

/// Shield held 4 frames, released into GUARD_OFF (frame 5, a 5-frame window), then `gap` more GUARD_OFF frames, then `last`.
fn release_then(gap: usize, last: &[Frame]) -> Vec<Frame> {
    [vec![HELD; 4], vec![OFF; gap + 1], last.to_vec()].concat()
}

fn perfect_inside_the_window() {
    let a = run(&release_then(2, &[PARRY]));
    assert_eq!(a.iter().map(|a| a.grade).collect::<Vec<_>>(), vec![Grade::Perfect(Some(3))], "released 3 frames before the hit");
    assert_eq!(a[0].window, Some((5, 5)));
}

fn early_after_the_window() {
    // GUARD_OFF at frame 5 with a 5-frame window, hit at frame 14: 4 frames past g + w (a parry's hitstop is first
    // sampled at g + w, Eden accept-1).
    assert_eq!(grades(&release_then(8, &[BODY_HIT])), vec![Grade::Early(4)]);
}

fn late_release_after_a_held_shield_hit() {
    let v = [vec![HELD; 3], vec![SHIELD_HIT, Frame { guard_damage: true, shield_held: true, ..IDLE }, IDLE]].concat();
    assert_eq!(grades(&v), vec![Grade::Late(2)], "the hit at frame 4, the release at frame 6");
}

fn parry_held_shield_and_unshielded_hit_are_both_missed() {
    let held = [vec![HELD; 2], vec![SHIELD_HIT], vec![HELD; BOUND as usize]].concat();
    assert_eq!(grades(&held), vec![Grade::Missed], "held through the hit");
    assert_eq!(grades(&[IDLE, IDLE, BODY_HIT]), vec![Grade::Missed], "hit without shielding");
}

fn parry_every_hit_is_one_attempt() {
    let v = release_then(1, &[PARRY, OFF, BODY_HIT, IDLE, PARRY]);
    assert_eq!(run(&v).len(), 3, "three hitstop onsets, three attempts");
    assert!(run(&[IDLE, Frame { stop: true, ..IDLE }, IDLE]).is_empty(), "P1's own hitlag is not an attempt");
    assert!(run(&[Frame { armed: false, ..BODY_HIT }]).is_empty(), "no pass, no attempt");
}

fn parry_three_parried_hits_count_three() {
    let between = Frame { guard_damage: true, just_shield: Some(true), ..IDLE };
    let v = release_then(1, &[PARRY, between, PARRY, between, PARRY]);
    assert_eq!(grades(&v).iter().filter(|g| matches!(g, Grade::Perfect(_))).count(), 3);
}

fn a_second_hit_ends_an_open_late_attempt_as_missed() {
    let mut d = Detector::new();
    let v = [HELD, HELD, SHIELD_HIT, Frame { guard_damage: true, shield_held: true, ..IDLE }, SHIELD_HIT];
    let seen: Vec<Grade> = v.iter().zip(1..).flat_map(|(f, n)| d.step(f, n)).flatten().map(|a| a.grade).collect();
    assert_eq!(seen, vec![Grade::Missed]);
    assert!(d.busy(), "the second hit is open");
}

fn unavailable_reads_are_not_counted() {
    let a = grades(&[IDLE, Frame { just_shield: None, ..PARRY }]);
    assert_eq!(a, vec![Grade::Unavailable]);
    assert!(!a[0].counted());
}

/// tr-review-astra-2 703e3f8d finding 2: a start-save load restarts the strip's frames; nothing from the old timeline
/// (a release, a GUARD_OFF window, an open attempt) may grade a hit after it.
fn detector_restarts_with_the_strip() {
    let mut d = Detector::new();
    let before = [HELD, HELD, SHIELD_HIT, HELD, HELD, OFF, OFF];
    for (f, n) in before.iter().zip(100..) {
        let _ = d.step(f, n);
    }
    let after: Vec<Attempt> = [IDLE, IDLE, BODY_HIT].iter().zip(1..).flat_map(|(f, n)| d.step(f, n)).flatten().collect();
    assert_eq!(after.iter().map(|a| (a.grade, a.window)).collect::<Vec<_>>(), vec![(Grade::Missed, None)], "only the new hit, with no old window");
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("perfect_inside_the_window", perfect_inside_the_window as fn()),
        ("early_after_the_window", early_after_the_window as fn()),
        ("late_release_after_a_held_shield_hit", late_release_after_a_held_shield_hit as fn()),
        ("parry_held_shield_and_unshielded_hit_are_both_missed", parry_held_shield_and_unshielded_hit_are_both_missed as fn()),
        ("parry_every_hit_is_one_attempt", parry_every_hit_is_one_attempt as fn()),
        ("parry_three_parried_hits_count_three", parry_three_parried_hits_count_three as fn()),
        ("a_second_hit_ends_an_open_late_attempt_as_missed", a_second_hit_ends_an_open_late_attempt_as_missed as fn()),
        ("unavailable_reads_are_not_counted", unavailable_reads_are_not_counted as fn()),
        ("detector_restarts_with_the_strip", detector_restarts_with_the_strip as fn()),
    ]);
}
}
