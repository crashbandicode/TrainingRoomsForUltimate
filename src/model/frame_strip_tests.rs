//! SPEC-input-06 §2 arithmetic and §3.5 behaviour, frame by frame. Frame numbers are the tracker's own: the first
//! `step` is frame 1. Statuses are arbitrary numbers: the model only compares them.

use super::*;

const ATTACK: i32 = 0x27;
const ATTACK_HI3: i32 = 0x2b;
const ATTACK_LW3: i32 = 0x2c;
const A: MappedInputs = MappedInputs { buttons: 0x1, lstick_x: 0, lstick_y: 0, rstick_x: 0, rstick_y: 0 };
const UP_A: MappedInputs = MappedInputs { buttons: 0x1, lstick_x: 0, lstick_y: 100, rstick_x: 0, rstick_y: 0 };
const NONE: MappedInputs = MappedInputs::NEUTRAL;

fn idle() -> Sample {
    Sample { actionable: true, ..Sample::default() }
}

fn attack(mv: Move, status: i32, motion_frame: f32) -> Sample {
    Sample { attack: Some(mv), status, motion_frame, ..Sample::default() }
}

/// P1 in its own hitlag while its move connects.
fn p1_stop(mv: Move, status: i32, motion_frame: f32) -> Sample {
    Sample { stop: true, active: true, ..attack(mv, status, motion_frame) }
}

fn cpu_hit(stop: bool, r: f32) -> Sample {
    Sample { stop, hitstun: true, reaction: Some(r), ..Sample::default() }
}

/// A script: P1, the CPU and P1's inputs for each frame, from frame 1.
struct Script {
    frames: Vec<(Sample, Sample, MappedInputs)>,
}

impl Script {
    fn new() -> Self {
        Self { frames: Vec::new() }
    }
    fn at(&self) -> u32 {
        self.frames.len() as u32 + 1
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

/// The CPU's hitstun after a hit on frame `hit` with `stop` hitlag frames: `r` counts down from `stun` on the
/// first frame after the hitlag, so the free frame is `hit + stop + stun + K`.
fn cpu_after(hit: u32, stop: u32, stun: f32, frame: u32) -> Sample {
    let first = hit + stop;
    if frame < first {
        cpu_hit(true, stun)
    } else {
        let r = stun - (frame - first) as f32;
        if r + K as f32 > 0.0 {
            cpu_hit(false, r.max(0.0))
        } else {
            idle()
        }
    }
}

/// Jab on frame 3 (startup 3..=6), hit on 7 with 4 frames of hitlag (7..=10), active 11, recovery to 19, actionable
/// from 20. The CPU gets `stun` frames of hitstun after the hitlag.
fn jab(script: &mut Script, stun: f32, until: u32) {
    script.push(idle(), idle(), NONE).push(idle(), idle(), NONE);
    for f in 3..=6u32 {
        script.push(attack(Move::Jab, ATTACK, (f - 3) as f32), idle(), if f == 3 { A } else { NONE });
    }
    for f in 7..=10u32 {
        script.push(p1_stop(Move::Jab, ATTACK, 4.0), cpu_after(7, 4, stun, f), NONE);
    }
    script.push(Sample { active: true, ..attack(Move::Jab, ATTACK, 5.0) }, cpu_after(7, 4, stun, 11), NONE);
    for f in 12..=19u32 {
        script.push(attack(Move::Jab, ATTACK, (f - 6) as f32), cpu_after(7, 4, stun, f), NONE);
    }
    for f in 20..until {
        script.push(idle(), cpu_after(7, 4, stun, f), NONE);
    }
}

/// An up tilt pressed on `press` (P1 busy from then), 5 frames of startup, hitting on `press + 5`.
fn up_tilt(script: &mut Script, press: u32, cpu: impl Fn(u32) -> Sample) {
    for f in press..press + 5 {
        script.push(attack(Move::UpTilt, ATTACK_HI3, (f - press) as f32), cpu(f), if f == press { UP_A } else { NONE });
    }
}

#[test]
fn classify_follows_the_spec_order() {
    let stop_in_stun = Sample { stop: true, hitstun: true, actionable: true, ..Sample::default() };
    assert_eq!(classify(&stop_in_stun, true, false), Cell::Hitlag);
    assert_eq!(classify(&Sample { hitstun: true, actionable: true, ..Sample::default() }, false, false), Cell::Hitstun);
    assert_eq!(classify(&Sample { shieldstun: true, ..Sample::default() }, false, false), Cell::Shieldstun);
    assert_eq!(classify(&idle(), true, false), Cell::Actionable);
    assert_eq!(classify(&Sample { active: true, ..attack(Move::Jab, ATTACK, 4.0) }, true, false), Cell::Active);
    assert_eq!(classify(&attack(Move::Jab, ATTACK, 1.0), true, false), Cell::Startup);
    assert_eq!(classify(&attack(Move::Jab, ATTACK, 9.0), true, true), Cell::Recovery, "after the active frames");
    assert_eq!(classify(&Sample::default(), true, false), Cell::Recovery, "a busy frame that is no attack (a dodge)");
    assert_eq!(classify(&Sample { active: true, ..Sample::default() }, false, false), Cell::Recovery, "the CPU has no active lane");
}

#[test]
fn link_true_combo_with_its_margin_window_and_late_input() {
    let mut s = Script::new();
    jab(&mut s, 18.0, 21);
    assert_eq!(s.at(), 21);
    up_tilt(&mut s, 21, |f| cpu_after(7, 4, 18.0, f));
    s.push(p1_stop(Move::UpTilt, ATTACK_HI3, 5.0), cpu_hit(true, 30.0), NONE);
    let t = s.run();
    let link = t.latest_link().expect("a follow-up");
    // r on frame 25 is 18 - (25 - 11) = 4: free frame 25 + 4 + K = 29; the hit on 26 beat it by 3.
    assert_eq!(link.verdict, Verdict::True(Some(-3)));
    assert_eq!((link.hit, link.free, link.actionable, link.press), (26, Some(29), 20, 21));
    assert_eq!(link.input, 1, "pressed one frame after P1 could act");
    assert_eq!(link.window, Some((20, 24)), "presses 20..=24 hit by frame 29");
    assert_eq!(link.spare(), Some(3));
    assert_eq!((link.from, link.to), (Move::Jab, Move::UpTilt));
}

#[test]
fn link_drop_counts_the_free_frames_and_has_no_window_when_even_frame_one_drops() {
    let mut s = Script::new();
    jab(&mut s, 10.0, 21);
    up_tilt(&mut s, 21, |f| cpu_after(7, 4, 10.0, f));
    s.push(p1_stop(Move::UpTilt, ATTACK_HI3, 5.0), cpu_hit(true, 30.0), NONE);
    let t = s.run();
    let link = t.latest_link().unwrap();
    // r reaches 0 on frame 21, where the CPU is already actionable, so the hit on 26 came 5 frames late.
    assert_eq!(link.verdict, Verdict::Drop(5));
    assert_eq!(link.free, Some(21));
    assert_eq!(link.window, None, "the window would end on 21 - 5 = 16, before P1 could act on 20");
    assert_eq!(link.spare(), Some(-5));
}

#[test]
fn link_drop_with_a_window_the_press_missed() {
    let mut s = Script::new();
    jab(&mut s, 14.0, 24);
    // Free frame 7 + 4 + 14 + K = 25. A press on 24 hits on 29: four frames late; the window was 20..=20.
    up_tilt(&mut s, 24, |f| cpu_after(7, 4, 14.0, f));
    s.push(p1_stop(Move::UpTilt, ATTACK_HI3, 5.0), cpu_hit(true, 30.0), NONE);
    let link = s.run().latest_link().unwrap();
    assert_eq!(link.verdict, Verdict::Drop(4));
    assert_eq!(link.input, 4);
    assert_eq!(link.window, Some((20, 20)));
}

#[test]
fn link_buffered_press_starts_on_the_first_actionable_frame() {
    let mut s = Script::new();
    s.push(idle(), idle(), NONE).push(idle(), idle(), NONE);
    for f in 3..=6u32 {
        s.push(attack(Move::Jab, ATTACK, (f - 3) as f32), idle(), if f == 3 { A } else { NONE });
    }
    for f in 7..=10u32 {
        s.push(p1_stop(Move::Jab, ATTACK, 4.0), cpu_after(7, 4, 18.0, f), NONE);
    }
    s.push(Sample { active: true, ..attack(Move::Jab, ATTACK, 5.0) }, cpu_after(7, 4, 18.0, 11), NONE);
    // Recovery 12..=19 with the up tilt pressed on 17 (A released on 16 so the press is new); the tilt starts on 20
    // with no actionable frame between.
    for f in 12..=19u32 {
        let inputs = if f == 17 { UP_A } else { NONE };
        s.push(attack(Move::Jab, ATTACK, (f - 6) as f32), cpu_after(7, 4, 18.0, f), inputs);
    }
    for f in 20..25u32 {
        s.push(attack(Move::UpTilt, ATTACK_HI3, (f - 20) as f32), cpu_after(7, 4, 18.0, f), NONE);
    }
    s.push(p1_stop(Move::UpTilt, ATTACK_HI3, 5.0), cpu_hit(true, 30.0), NONE);
    let link = s.run().latest_link().unwrap();
    assert_eq!((link.actionable, link.press, link.input), (20, 17, -3), "buffered three frames");
    // r on 24 is 5: free 24 + 5 + K = 29; the hit on 25 beat it by 4.
    assert_eq!(link.verdict, Verdict::True(Some(-4)));
    assert_eq!(link.window, Some((20, 24)));
}

#[test]
fn link_through_hitlag_reads_the_frame_before_the_hit() {
    // A long hitlag (8 frames) on the jab: r does not move during it, and the link is counted from the frame before
    // the follow-up hit.
    let mut s = Script::new();
    s.push(idle(), idle(), NONE).push(idle(), idle(), NONE);
    for f in 3..=6u32 {
        s.push(attack(Move::Jab, ATTACK, (f - 3) as f32), idle(), if f == 3 { A } else { NONE });
    }
    for f in 7..=14u32 {
        s.push(p1_stop(Move::Jab, ATTACK, 4.0), cpu_after(7, 8, 18.0, f), NONE);
    }
    for f in 15..=23u32 {
        s.push(attack(Move::Jab, ATTACK, (f - 10) as f32), cpu_after(7, 8, 18.0, f), NONE);
    }
    s.push(idle(), cpu_after(7, 8, 18.0, 24), NONE);
    up_tilt(&mut s, 25, |f| cpu_after(7, 8, 18.0, f));
    s.push(p1_stop(Move::UpTilt, ATTACK_HI3, 5.0), cpu_hit(true, 30.0), NONE);
    let link = s.run().latest_link().unwrap();
    // Free frame 7 + 8 + 18 + K = 33 (r on 29 is 18 - 14 = 4: 29 + 4 + K); the hit on 30 beat it by 3.
    assert_eq!(link.verdict, Verdict::True(Some(-3)));
    assert_eq!(link.free, Some(33));
}

#[test]
fn link_without_a_hitstun_read_is_true_without_a_number() {
    let mut s = Script::new();
    jab(&mut s, 18.0, 21);
    up_tilt(&mut s, 21, |f| Sample { reaction: None, ..cpu_after(7, 4, 18.0, f) });
    s.push(p1_stop(Move::UpTilt, ATTACK_HI3, 5.0), cpu_hit(true, 30.0), NONE);
    let link = s.run().latest_link().unwrap();
    assert_eq!(link.verdict, Verdict::True(None));
    assert_eq!((link.window, link.spare()), (None, None));
    assert_eq!(link.input, 1, "the input is still measured");
}

#[test]
fn hits_of_one_move_are_not_links() {
    let mut s = Script::new();
    jab(&mut s, 18.0, 13);
    // A second hit of the same jab action (no new move, no actionable frame).
    s.push(p1_stop(Move::Jab, ATTACK, 9.0), cpu_hit(true, 20.0), NONE);
    assert!(s.run().latest_link().is_none());
}

#[test]
fn shield_is_not_a_link_and_gives_advantage_on_shield() {
    let shield = Sample { actionable: true, shielding: true, ..Sample::default() };
    let stun = Sample { shieldstun: true, shielding: true, ..Sample::default() };
    let mut s = Script::new();
    s.push(idle(), shield, NONE).push(idle(), shield, NONE);
    for f in 3..=6u32 {
        s.push(attack(Move::Jab, ATTACK, (f - 3) as f32), shield, if f == 3 { A } else { NONE });
    }
    for _ in 7..=10 {
        s.push(p1_stop(Move::Jab, ATTACK, 4.0), Sample { stop: true, ..stun }, NONE);
    }
    for f in 11..=19u32 {
        s.push(attack(Move::Jab, ATTACK, f as f32), if f < 14 { stun } else { shield }, NONE);
    }
    s.push(idle(), shield, NONE);
    // A second jab on shield: still no link.
    for f in 21..=24u32 {
        s.push(attack(Move::Jab, ATTACK, (f - 21) as f32), shield, if f == 21 { A } else { NONE });
    }
    s.push(p1_stop(Move::Jab, ATTACK, 4.0), Sample { stop: true, ..stun }, NONE);
    let t = s.run();
    assert!(t.latest_link().is_none());
    // After the first jab: the CPU could act (shield) on 14, P1 on 20.
    assert_eq!(t.advantage(), Some(Advantage { value: -6, on_shield: true, p1: 20, cpu: 14 }));
}

#[test]
fn advantage_on_hit_after_the_exchange_ends() {
    let mut s = Script::new();
    jab(&mut s, 12.0, 30);
    let t = s.run();
    // Free frames: P1 20, the CPU 7 + 4 + 12 + K = 23.
    assert_eq!(t.advantage(), Some(Advantage { value: 3, on_shield: false, p1: 20, cpu: 23 }));
    assert_eq!(t.stun_check, Some((23, 23)), "the read predicted the free frame it saw");
}

#[test]
fn a_projectile_hit_counts_and_a_hit_on_p1_does_not() {
    let mut s = Script::new();
    s.push(idle(), idle(), NONE);
    // The CPU's hitstop starts while P1 can act (a projectile): a hit.
    s.push(idle(), cpu_hit(true, 10.0), NONE);
    let t = s.run();
    assert!(t.last_hit.is_some());
    let mut s = Script::new();
    s.push(idle(), idle(), NONE);
    // Both stop with P1 in hitstun: the CPU hit P1.
    s.push(Sample { stop: true, hitstun: true, ..Sample::default() }, Sample { stop: true, ..Sample::default() }, NONE);
    assert!(s.run().last_hit.is_none());
}

#[test]
fn history_keeps_the_last_four_links_newest_first() {
    let mut t = Tracker::new();
    let mut verdicts = Vec::new();
    for n in 0..5u32 {
        // Each round: a jab that hits, then a down tilt follow-up that drops by n + 1.
        let stun = 10.0 - n as f32;
        let mut s = Script::new();
        jab(&mut s, stun, 21);
        for f in 21..26u32 {
            s.push(attack(Move::DownTilt, ATTACK_LW3, (f - 21) as f32), cpu_after(7, 4, stun, f), if f == 21 { A } else { NONE });
        }
        s.push(p1_stop(Move::DownTilt, ATTACK_LW3, 5.0), cpu_hit(true, 30.0), NONE);
        for _ in 0..40 {
            s.push(idle(), idle(), NONE);
        }
        for (p1, cpu, inputs) in &s.frames {
            t.step(*p1, *cpu, *inputs);
        }
        verdicts.push(t.latest_link().unwrap().verdict);
    }
    let kept: Vec<_> = t.history().map(|l| l.verdict).collect();
    assert_eq!(kept.len(), HISTORY);
    assert_eq!(kept, verdicts.iter().rev().take(HISTORY).copied().collect::<Vec<_>>());
    assert_eq!(t.history().next().unwrap().to, Move::DownTilt);
}

#[test]
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

#[test]
fn the_window_holds_sixty_frames() {
    let mut s = Script::new();
    for f in 0..200u32 {
        s.push(attack(Move::Jab, ATTACK, f as f32), idle(), NONE);
    }
    let t = s.run();
    assert_eq!(t.len(), WINDOW);
    let frames: Vec<u32> = t.columns().map(|c| c.frame).collect();
    assert_eq!(frames.first(), Some(&141));
    assert_eq!(frames.last(), Some(&200));
}

#[test]
fn clear_forgets_everything() {
    let mut s = Script::new();
    jab(&mut s, 18.0, 30);
    let mut t = s.run();
    t.clear();
    assert_eq!(t, Tracker::new());
}

#[test]
fn predicted_free_rounds_up_and_refuses_nonsense() {
    assert_eq!(predicted_free(10, 3.0), Some(13));
    assert_eq!(predicted_free(10, 2.4), Some(13), "2.4 needs three more frames to reach 0");
    assert_eq!(predicted_free(10, 0.0), Some(10));
    assert_eq!(predicted_free(10, -1.0), None);
    assert_eq!(predicted_free(10, f32::NAN), None);
    assert_eq!(predicted_free(10, 1000.0), None);
}

#[test]
fn presses_are_new_buttons_first_in_chip_order_with_the_stick() {
    assert_eq!(press_of(UP_A, 0, 5), Some(Press { frame: 5, button: Button::Attack, stick: Some(2) }));
    assert_eq!(press_of(UP_A, 0x1, 5), None, "held, not pressed");
    let both = MappedInputs { buttons: 0x1 | 0x4, ..NONE };
    assert_eq!(press_of(both, 0, 1).map(|p| p.button), Some(Button::Attack));
    assert_eq!(press_of(MappedInputs { buttons: 0x10, ..NONE }, 0, 1).map(|p| p.button), Some(Button::Grab));
    assert_eq!(press_of(MappedInputs { buttons: 0x400, ..NONE }, 0, 1), None, "a taunt is no chip");
}

#[test]
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

#[test]
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

#[test]
fn migrate_the_old_rows() {
    assert_eq!(migrate(&[1, 0, 0, 0]), Some([1, 0]), "Off stays Off");
    for old in [[0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]] {
        assert_eq!(migrate(&old), Some([0, 1]), "Smash, Raw and Status Only become the frame strip");
    }
    assert_eq!(migrate(&[0, 0, 0, 0]), None);
    assert_eq!(migrate(&[1, 1, 0, 0]), None);
    assert_eq!(migrate(&[0, 1]), None);
}
