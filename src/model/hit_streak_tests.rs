use super::{Motion, Streak};

const P1: u32 = 0;
const WEAPON: u32 = 0x1000_0001;
const WEAPON2: u32 = 0x1000_0002;

fn live() -> Streak {
    let mut streak = Streak::EMPTY;
    streak.restart(true);
    streak
}

fn frame(streak: &mut Streak, status: i32, kind: u64, frame: f32, attack: bool, weapons: &[(u32, bool)]) {
    let body = streak.begin(P1, Motion { status, kind, frame }, attack);
    for &(id, has_hitbox) in weapons {
        streak.projectile(body, id, || has_hitbox);
    }
    streak.end(body);
}

fn jab(streak: &mut Streak) {
    frame(streak, 3, 3, 3.0, true, &[]);
    streak.hit(P1, P1);
    frame(streak, 0, 0, 0.0, false, &[]);
}

#[test]
fn hit_increments_and_multihit_counts_once() {
    let mut streak = live();
    frame(&mut streak, 3, 3, 3.0, true, &[]);
    streak.hit(P1, P1);
    assert_eq!(streak.counts(), (1, 1));
    frame(&mut streak, 3, 3, 4.0, false, &[]);
    frame(&mut streak, 3, 3, 8.0, true, &[]);
    streak.hit(P1, P1);
    streak.hit(P1, P1);
    assert_eq!(streak.counts(), (1, 1));

    // A different attack motion in the same status opens another move.
    frame(&mut streak, 3, 4, 0.0, false, &[]);
    frame(&mut streak, 3, 4, 4.0, true, &[]);
    streak.hit(P1, P1);
    assert_eq!(streak.counts(), (2, 2));
}

#[test]
fn whiff_resets_current_and_keeps_best() {
    let mut streak = live();
    jab(&mut streak);
    jab(&mut streak);
    frame(&mut streak, 3, 3, 3.0, true, &[]);
    frame(&mut streak, 3, 3, 5.0, false, &[]);
    assert_eq!(streak.counts(), (2, 2), "recovery is still part of the attack");
    frame(&mut streak, 0, 0, 0.0, false, &[]);
    assert_eq!(streak.counts(), (0, 2));
    jab(&mut streak);
    assert_eq!(streak.counts(), (1, 2));
}

#[test]
fn expiring_projectile_is_a_delayed_whiff() {
    let mut streak = live();
    jab(&mut streak);
    frame(&mut streak, 4, 4, 6.0, false, &[(WEAPON, true), (WEAPON2, true)]);
    frame(&mut streak, 0, 0, 0.0, false, &[(WEAPON, false), (WEAPON2, true)]);
    frame(&mut streak, 0, 0, 1.0, false, &[(WEAPON2, false)]);
    assert_eq!(streak.counts(), (1, 1), "an inactive hitbox is not expiry");
    frame(&mut streak, 0, 0, 2.0, false, &[]);
    assert_eq!(streak.counts(), (0, 1));
}

#[test]
fn projectile_hit_counts_once() {
    let mut streak = live();
    frame(&mut streak, 4, 4, 6.0, false, &[(WEAPON, true), (WEAPON2, true)]);
    frame(&mut streak, 0, 0, 0.0, false, &[(WEAPON, true), (WEAPON2, true)]);
    streak.hit(P1, WEAPON);
    streak.hit(P1, WEAPON2);
    assert_eq!(streak.counts(), (1, 1));
    frame(&mut streak, 0, 0, 1.0, false, &[]);
    assert_eq!(streak.counts(), (1, 1));
}

#[test]
fn body_and_projectile_are_one_move() {
    let mut streak = live();
    frame(&mut streak, 4, 4, 4.0, true, &[]);
    streak.hit(P1, P1);
    frame(&mut streak, 4, 4, 6.0, false, &[(WEAPON, true)]);
    frame(&mut streak, 0, 0, 0.0, false, &[(WEAPON, true)]);
    streak.hit(P1, WEAPON);
    assert_eq!(streak.counts(), (1, 1));
}

#[test]
fn no_attack_ignores_grabs_dodges_and_movement() {
    let mut streak = live();
    jab(&mut streak);
    for status in 30..34 {
        frame(&mut streak, status, status as u64, 0.0, false, &[(WEAPON, false)]);
    }
    frame(&mut streak, 0, 0, 0.0, false, &[]);
    assert_eq!(streak.counts(), (1, 1));
}

#[test]
fn round_restart_keeps_session_best() {
    let mut streak = live();
    jab(&mut streak);
    jab(&mut streak);
    frame(&mut streak, 4, 4, 6.0, false, &[(WEAPON, true)]);
    streak.restart(false);
    assert_eq!(streak.counts(), (0, 2));
    jab(&mut streak);
    assert_eq!(streak.counts(), (0, 2));
    streak.restart(true);
    jab(&mut streak);
    assert_eq!(streak.counts(), (1, 2));
}

#[test]
fn status_preserving_restart_ends_prior_move() {
    let mut streak = live();
    jab(&mut streak);
    frame(&mut streak, 3, 3, 14.0, true, &[]);
    frame(&mut streak, 3, 3, 0.0, false, &[]);
    assert_eq!(streak.counts(), (0, 1));
    frame(&mut streak, 3, 3, 4.0, true, &[]);
    streak.hit(P1, P1);
    assert_eq!(streak.counts(), (1, 1));
}

#[test]
fn a_delayed_article_keeps_its_spawning_move() {
    let mut streak = live();
    frame(&mut streak, 4, 4, 4.0, true, &[(WEAPON, false)]);
    streak.hit(P1, P1);
    frame(&mut streak, 0, 0, 0.0, false, &[(WEAPON, false)]);
    frame(&mut streak, 3, 3, 2.0, true, &[(WEAPON, true)]);
    streak.hit(P1, WEAPON);
    assert_eq!(streak.counts(), (1, 1));
    streak.hit(P1, P1);
    assert_eq!(streak.counts(), (2, 2));
}

#[test]
fn final_sample_can_credit_native_hit_before_projectile_expiry() {
    let mut streak = live();
    frame(&mut streak, 4, 4, 4.0, false, &[(WEAPON, true)]);
    let body = streak.begin(P1, Motion { status: 0, kind: 0, frame: 0.0 }, false);
    streak.hit(P1, WEAPON);
    streak.end(body);
    assert_eq!(streak.counts(), (1, 1));
}
