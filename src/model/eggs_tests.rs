//! rc.14 Eggs-ercise (`artifacts/rc14-eggsercise/SPEC.md` §5, rows 1–15 and 17–18; row 16 is Eden only).

lib_unit_tests! {
use super::*;
use crate::model::hitbox::{self, sphere_key, Sphere, KIND_GRAB, KIND_HIT, KIND_HURT};
use crate::model::panes::{EggLook, EggPicture, EGG_WHOLE};
use crate::model::menu::table::{Control, Page, GROUPS, ROWS};
use crate::model::scene::Facts;

const TRAINING: Facts = Facts { native_training: true, lobby_active: false, waiting_room: false, busy: false };
const LOBBY: Facts = Facts { native_training: false, lobby_active: true, waiting_room: true, busy: false };
const MATCH: Facts = Facts { native_training: false, lobby_active: false, waiting_room: false, busy: false };
const SWAPPING: Facts = Facts { native_training: true, lobby_active: false, waiting_room: false, busy: true };
const P1: usize = 0x1000;
const NANA: usize = 0x2000;

fn hit(at: [f32; 2], power: f32) -> Hit {
    Hit([at[0], at[1], 0.0], [at[0], at[1], 0.0], 3.0, power)
}

/// Training's camera at reset-center, measured in Eden (wa-eggs-6e9a8fa5-1, `eggs-E5-before`: the game picture 1198 px
/// wide, 16.9 px a unit, world x 0 at px 895, the ground at px 502): on screen x −50.5 to +20.4, y −10.2 to +29.7.
/// The range of a missing or implausible stage read: the draft's box.
const BOX: [f32; 2] = [-HALF_WIDTH, HALF_WIDTH];
const RESET_CENTRE: View = View([27.09, 27.09], [408.7, -264.4], BOX);

fn started(mode: u8, rule: u8, seed: u32) -> Run {
    let mut run = Run::IDLE;
    assert_eq!(run.plan([mode, rule, NORMAL, DISC], Some(seed), 0, || TRAINING), Plan::Read);
    settle(&mut run, RESET_CENTRE);
    run
}

/// The draw's ticks with the camera at `view`, enough for it to count as still.
fn settle(run: &mut Run, view: View) {
    for _ in 0..=SETTLE {
        run.look(true, || Some(view));
    }
}

/// One pass of Player 1 (the clock keeper) standing at y = 0.
fn pass(run: &mut Run, hits: &[Hit], aerial: bool) -> Option<u16> {
    let tick = run.keeper(P1);
    run.step(tick, 0.0, hits, aerial, 7)
}

fn notice(run: &Run) -> String {
    let mut text = String::new();
    run.write_notice(&mut text).unwrap();
    text
}

/// A reset (L+R+A, Change Stage) and the draw's next tick, which publishes the camera the first target spawns by.
fn restart(run: &mut Run, seed: u32) {
    run.reset(seed);
    settle(run, RESET_CENTRE);
}

fn gap(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn eggs_rows_and_help() {
    let at = |id: &str| ROWS.iter().position(|r| r.id == id).unwrap_or_else(|| panic!("row {id}"));
    let mode = at(ROW);
    let rows = [&ROWS[mode], &ROWS[at(ROW_BREAK)], &ROWS[at(ROW_TARGET)], &ROWS[at(ROW_SPEED)], &ROWS[at(ROW_PRACTICE_CPU)]];
    // D7 (b), the operator 2026-11-14: their own group, appended to `GROUPS` so no group index moves.
    let g = usize::from(rows[0].group);
    assert_eq!(g, GROUPS.len() - 1, "the last group");
    assert_eq!((GROUPS[g].title, GROUPS[g].page, GROUPS[g].advanced), ("Practice games", Page::FightersStages, false));
    assert!(ROWS.iter().filter(|r| usize::from(r.group) == g).map(|r| r.id).take(5).eq([ROW, ROW_BREAK, ROW_TARGET, ROW_SPEED, ROW_PRACTICE_CPU]), "the five rows come first, in order");
    let values = |r: &crate::model::menu::table::Row| match r.control {
        Control::Dropdown(v) => v.to_vec(),
        _ => panic!("{} is not a dropdown", r.title),
    };
    // SPEC V2.4 (D4 a + c) and V2.3 (D3 b: Any attack first, the default); V2.7's rows and help.
    let modes = vec!["Off", "1 minute", "10 targets", "20 targets", "Free practice"];
    assert_eq!((rows[0].title, values(rows[0])), ("Eggs-ercise", modes));
    assert_eq!((rows[1].title, values(rows[1])), ("Breaks on", vec!["Any attack", "Aerials", "Hits of 12%+"]));
    assert_eq!(
        rows[0].help_lines(),
        ("Break the targets that appear. Starts when the menu closes.", "Timed: 60 s, or break 10 or 20 fastest. Free practice: no timer.")
    );
    assert_eq!(rows[1].help_lines(), ("Which of your hits break a target. Projectiles never do.", "12%+: only attacks that deal 12% or more."));
    // §2.3b's speed row, renamed by V2.2.
    assert_eq!((rows[2].title, values(rows[2])), ("Target", vec!["Disc", "CPU"]));
    assert_eq!(rows[2].help_lines(), ("Disc: a target appears. CPU: the CPU is the target.", "The CPU target stays still and is reset after each hit."));
    assert_eq!((rows[3].title, values(rows[3])), ("Target speed", vec!["Normal", "Slow", "Fast"]));
    assert_eq!(
        rows[3].help_lines(),
        ("How fast targets fly. Normal is Melee's Eggs-ercise speed.", "Slow: half speed. Fast: one and a half times as fast.")
    );
    // V2.6's second row: Hide CPU (B-2's) reads it with `eggs_running`.
    assert_eq!((rows[4].title, values(rows[4])), ("CPU during a game", vec!["Hide", "Show"]));
    assert_eq!(
        rows[4].help_lines(),
        ("Hide: the CPU is hidden while a practice game runs.", "Afterwards it's as Hide CPU says. Show: it stays.")
    );
    for r in rows {
        let (a, b) = r.help_lines();
        assert!(a.chars().count() <= 67 && b.chars().count() <= 67, "{}", r.title);
    }
    for (id, default) in [(ROW, &[1, 0, 0, 0, 0][..]), (ROW_BREAK, &[1, 0, 0]), (ROW_TARGET, &[1, 0]), (ROW_SPEED, &[1, 0, 0]), (ROW_PRACTICE_CPU, &[1, 0])] {
        let def = crate::model::factory::DEFS.iter().find(|d| d.id == id).unwrap_or_else(|| panic!("factory {id}"));
        assert_eq!(def.default, default, "{id}: the first option (Off, Any attack) by default");
    }
    let indices = [OFF, TIMED, TEN, TWENTY, FREE, ANY, AERIALS, STRONG, NORMAL, SLOW, FAST];
    assert_eq!(indices, [0, 1, 2, 3, 4, 0, 1, 2, 0, 1, 2], "the rows' option indices");
    assert_eq!(Run::IDLE.rows, [OFF, ANY, NORMAL, DISC]);
}

/// SPEC V2.4: `10 targets` and `20 targets` count the clock up in hundredths and stop at the Nth; the session's best is
/// the fastest, per fighter and per N.
fn eggs_count_mode() {
    let mut run = started(TEN, ANY, 7);
    assert_eq!(pass(&mut run, &[], false), Some(u16::MAX));
    assert_eq!(notice(&run), "Eggs 0/10   0:00.01");
    let changes = (0..59).filter(|_| pass(&mut run, &[], false).is_some()).count();
    assert_eq!((changes, notice(&run)), (59, "Eggs 0/10   0:01.00".to_owned()), "counts up in hundredths");
    // One target every `every` passes; the pass that breaks the Nth ends the run.
    let go = |run: &mut Run, every: u32, kind: i32| {
        for k in 1..=2000 {
            let hits = run.egg_at().filter(|_| k % every == 0).map(|egg| hit(egg, 0.0));
            let tick = run.keeper(P1);
            if run.step(tick, 0.0, hits.as_slice(), false, kind) == Some(END_FRAMES) {
                return k;
            }
        }
        panic!("no end")
    };
    assert_eq!(go(&mut run, 30, 7), 300);
    assert_eq!((run.score, notice(&run)), (10, "Done! 10 in 0:06.00 (best 0:06.00)".to_owned()), "60 + 300 frames");
    assert!(target(&mut run).is_none(), "the targets stop");
    assert!(run.look(true, || panic!("an ended run: no projection"))[0].is_none());
    for _ in 0..BURST_TICKS {
        pass(&mut run, &[], false);
    }
    assert_eq!(run.plan([TEN, ANY, NORMAL, DISC], None, 0, || TRAINING), Plan::Idle, "it waits for a restart");
    restart(&mut run, 5);
    assert_eq!(pass(&mut run, &[], false), Some(u16::MAX));
    assert_eq!(notice(&run), "Eggs 0/10   0:00.01", "L+R+A starts it over from 0:00");
    restart(&mut run, 5);
    go(&mut run, 40, 7);
    assert_eq!(notice(&run), "Done! 10 in 0:06.66 (best 0:06.00)", "a slower run keeps the best");
    restart(&mut run, 5);
    go(&mut run, 20, 7);
    assert_eq!(notice(&run), "Done! 10 in 0:03.33 (best 0:03.33)", "a faster one replaces it");
    restart(&mut run, 5);
    go(&mut run, 40, 8);
    assert_eq!(notice(&run), "Done! 10 in 0:06.66 (best 0:06.66)", "another fighter has its own best");
    let mut twenty = started(TWENTY, ANY, 7);
    twenty.best = run.best;
    assert_eq!((go(&mut twenty, 20, 7), twenty.score), (400, 20));
    assert_eq!(notice(&twenty), "Done! 20 in 0:06.66 (best 0:06.66)", "20 targets has its own best");
    let mut timed = started(TIMED, ANY, 7);
    timed.best = run.best;
    for _ in 0..3600 {
        pass(&mut timed, &[], false);
    }
    assert_eq!(notice(&timed), "Time! 0 eggs (best 0)", "the 1-minute best is kept apart");
}

fn eggs_run_start() {
    let mut blind = Run::IDLE;
    blind.plan([TIMED, AERIALS, NORMAL, DISC], Some(7), 0, || TRAINING);
    assert_eq!(blind.step(true, 0.0, &[], false, 7), None);
    assert!(target(&mut blind).is_none(), "no egg before the draw published the camera");
    assert!(blind.look(false, || panic!("another layout's tick reads nothing"))[0].is_none());
    assert!(blind.look(true, || Some(RESET_CENTRE))[0].is_none(), "the draw's tick takes the camera");
    settle(&mut blind, RESET_CENTRE);
    assert_eq!(blind.step(true, 0.0, &[], false, 7), Some(u16::MAX));
    assert!(target(&mut blind).is_some(), "the next pass spawns the egg");
    let mut run = started(TIMED, AERIALS, 7);
    assert!(target(&mut run).is_none(), "the first pass reads the ground before the egg spawns");
    assert_eq!(pass(&mut run, &[], false), Some(u16::MAX), "the standing notice is shown");
    assert_eq!(notice(&run), "Eggs 0   1:00");
    let egg = target(&mut run).expect("one egg");
    assert!(egg[0].abs() <= HALF_WIDTH && (RADIUS..=RADIUS + 2.0).contains(&egg[1]), "on the ground, launched: {egg:?}");
    assert_eq!(run.look(false, || None)[0], Some((egg, EGG_WHOLE)), "the whole target, full size");
    let changes = (0..60).filter(|_| pass(&mut run, &[], false).is_some()).count();
    assert_eq!((changes, notice(&run)), (1, "Eggs 0   0:59".to_owned()), "counts down, written once a second");
}

fn eggs_hit_overlap() {
    let mut run = started(TIMED, ANY, 7);
    pass(&mut run, &[], false);
    let egg = run.egg_at().unwrap();
    pass(&mut run, &[hit([egg[0] + 7.5, egg[1]], 9.0)], false);
    assert_eq!(run.score, 0, "3 + 4 apart is no overlap at 7.5");
    let egg = run.egg_at().unwrap();
    assert_eq!(pass(&mut run, &[hit([egg[0] + 6.9, egg[1]], 9.0)], false), Some(u16::MAX));
    assert_eq!((run.score, notice(&run).starts_with("Eggs 1 ")), (1, true));
    let next = run.egg_at().expect("the next egg spawns in the same pass");
    assert!(gap(next, egg) >= MIN_GAP - 1.0, "{next:?} {egg:?}");
    let egg = next;
    let pill = Hit([egg[0] - 20.0, egg[1] + 5.0, 0.0], [egg[0] + 20.0, egg[1] + 5.0, 0.0], 2.0, 9.0);
    pass(&mut run, &[pill], false);
    assert_eq!(run.score, 2, "a capsule's band between its ends overlaps too");
}

fn eggs_respawn_distance() {
    for seed in 0..200 {
        let mut run = started(FREE, ANY, seed);
        // Zoomed out, so the box has room 30 away from every spot (at reset-center, 43 wide, the farther end is the
        // fallback when none is: `eggs_on_screen`).
        settle(&mut run, View([14.0, 14.0], [0.0, -300.0], BOX));
        run.step(false, -10.0, &[], false, 0);
        for _ in 0..20 {
            let last = run.egg_at().unwrap();
            run.step(false, 99.0, &[hit(last, 0.0)], false, 0);
            let next = run.egg_at().unwrap();
            assert!(gap(next, last) >= MIN_GAP, "seed {seed}: {next:?} from {last:?}");
            assert!(next[0].abs() <= HALF_WIDTH && next[1] == -10.0 + RADIUS, "seed {seed}: on the run's ground {next:?}");
        }
        assert_eq!(run.score, 20);
    }
}

fn eggs_on_screen() {
    // Reset-center; the camera following Player 1 to the right (the box's left part off screen); zoomed out; zoomed in
    // (the apex clamped under the top).
    let views = [RESET_CENTRE, View([27.09, 27.09], [-900.0, -264.4], BOX), View([14.0, 14.0], [0.0, -300.0], BOX), View([40.0, 40.0], [0.0, -300.0], BOX)];
    // §2.3b row 22: every spawn's whole path, at every speed, stays on screen (the notice is in a top band: V2.4).
    for (view, speed) in views.into_iter().flat_map(|v| [NORMAL, SLOW, FAST].map(|s| (v, s))) {
        for seed in 0..60 {
            let mut run = started(FREE, ANY, seed);
            settle(&mut run, view);
            run.rows[2] = speed;
            run.step(false, 0.0, &[], false, 0);
            for _ in 0..6 {
                let mut egg = run.egg_at().unwrap();
                for _ in 0..200 {
                    egg = run.egg_at().unwrap();
                    let ([cx, cy], pad) = ([0, 1].map(|i| egg[i] * view.0[i] + view.1[i]), RADIUS * view.0[0] - 0.01);
                    assert!(cx.abs() <= 960.0 - pad && cy.abs() <= 540.0 - pad, "seed {seed}: {egg:?} off screen in {view:?}");
                    assert!(egg[0].abs() <= HALF_WIDTH && egg[1] >= RADIUS, "seed {seed}: {egg:?} outside the box");
                    run.step(true, 0.0, &[], false, 0);
                }
                run.step(false, 0.0, &[hit(egg, 0.0)], false, 0);
            }
        }
    }
}

/// The operator, 11-14 ("a, move the notice to the top"): a practice game's notice sits in a top band, so a target may
/// rest in the middle of the screen where the notice used to stand. Zoomed in to 40 the whole on-screen ground was
/// behind the old plates (canvas x within 496 of the centre) and the spawn fell back to an end of the range.
fn eggs_spawn_ignores_the_notice() {
    let view = View([40.0, 40.0], [0.0, -300.0], BOX);
    let mut middle = 0;
    for seed in 0..40 {
        let mut run = started(FREE, ANY, seed);
        settle(&mut run, view);
        run.step(false, 0.0, &[], false, 0);
        for _ in 0..200 {
            run.step(true, 0.0, &[], false, 0);
        }
        middle += usize::from(run.egg_at().is_some_and(|egg| (egg[0] * view.0[0]).abs() < 400.0));
    }
    assert!(middle > 10, "{middle} of 40 targets rested in the middle of the screen");
}

/// The notice slot's y by state (the operator, 11-14): the old place with no practice game; the top band while one runs; and,
/// with the frame strip drawn at the top, the band directly under the strip, so the two never overlap.
fn eggs_notice_band() {
    use crate::model::panes::{iv, notice_y, NOTICE_ORIGIN_Y, NOTICE_TOP_Y};
    // The header plate's top edge above the slot's y: the box top 445 + 25, and the plate's 16 padding.
    let plates_top = 486.0;
    assert_eq!([notice_y(false, false), notice_y(false, true)], [NOTICE_ORIGIN_Y; 2], "no game: the old place");
    assert_eq!(notice_y(true, false), NOTICE_TOP_Y, "a game and no strip: the top band");
    let strip_bottom = iv::STRIP_Y - iv::STRIP_H / 2.0;
    let under = notice_y(true, true) + plates_top;
    assert!(under <= strip_bottom && under >= strip_bottom - 20.0, "the band's top {under} is not just under the strip's {strip_bottom}");
    assert!(NOTICE_TOP_Y + plates_top <= 540.0, "the top band is on screen");
}

/// The height of the target above its spawn point after each of `n` ticks, and its x.
fn path(run: &mut Run, n: usize) -> Vec<[f32; 2]> {
    let spawn = run.egg_at().unwrap();
    (0..n)
        .map(|_| {
            pass(run, &[], false);
            let at = run.egg_at().unwrap();
            [at[1] - spawn[1], at[0]]
        })
        .collect()
}

fn apex(heights: &[[f32; 2]]) -> (usize, f32) {
    heights.iter().enumerate().fold((0, 0.0), |a, (i, h)| if h[0] > a.1 { (i + 1, h[0]) } else { a })
}

/// §2.3b row 20: Normal launches straight up, apex at frame 30 21.6 above, back at spawn height at 60, one bounce of at
/// most 8, at rest from 96.
fn eggs_motion_normal() {
    let mut run = started(FREE, ANY, 7);
    pass(&mut run, &[], false);
    let y = run.egg_at().unwrap()[1];
    assert!((y - (RADIUS + 1.44 - 0.024)).abs() < 1e-4, "spawned on the ground and launched: {y}");
    let mut run = started(FREE, ANY, 7);
    run.step(false, 0.0, &[], false, 7);
    let h = path(&mut run, 200);
    let (frame, top) = apex(&h);
    assert!((29..=31).contains(&frame) && (top - 21.6).abs() <= 0.2, "apex {top} at frame {frame}");
    assert!(h[59][0].abs() < 1e-3, "back at spawn height at frame 60: {:?}", h[59]);
    let bounce = h[60..96].iter().map(|h| h[0]).fold(0.0, f32::max);
    assert!((7.0..=8.0).contains(&bounce), "one bounce of {bounce}");
    assert!(h[95..].iter().all(|x| *x == h[95]) && h[95][0].abs() < 1e-3, "at rest from frame 96: {:?}", &h[90..100]);
    let drift = (h[95][1] - h[0][1]).abs();
    assert!(drift <= DRIFT, "sideways at most {DRIFT}: {drift}");
}

/// §2.3b row 21: Slow and Fast trace the same path in 2 and 2/3 the time.
fn eggs_speed_scales_time() {
    let flight = |speed: u8| {
        let mut run = started(FREE, ANY, 9);
        run.rows[2] = speed;
        run.step(false, 0.0, &[], false, 7);
        path(&mut run, 400)
    };
    let (normal, slow, fast) = (flight(NORMAL), flight(SLOW), flight(FAST));
    for (h, frame) in [(&normal, 30), (&slow, 60), (&fast, 20)] {
        let (at, top) = apex(h);
        assert!(at.abs_diff(frame) <= 1 && (top - 21.6).abs() <= 0.2, "apex {top} at {at}, want {frame}");
        assert!((h[399][1] - normal[399][1]).abs() < 1e-3, "the same landing x");
    }
    assert!(slow[150][0] > 1.0 && slow[192][0].abs() < 1e-3 && fast[64][0].abs() < 1e-3, "at rest by 192 / 64");
}

/// The pictures a draw tick shows: the target, then the break animations.
fn own(run: &mut Run) -> Vec<([f32; 2], EggLook)> {
    run.look(false, || panic!("not the practice's layout")).into_iter().flatten().collect()
}

/// Where the target picture is.
fn target(run: &mut Run) -> Option<[f32; 2]> {
    run.look(false, || None)[0].map(|(at, _)| at)
}

/// V2.6: "a practice game is running" from the menu close that starts a mode until Off or the scene is left; the end
/// notice and the wait for a restart count.
fn eggs_running() {
    let mut run = Run::IDLE;
    assert!(!run.running());
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC], None, 0, || TRAINING), Plan::Idle);
    assert!(!run.running(), "not before the menu closes");
    let mut run = started(TIMED, ANY, 7);
    assert!(run.running());
    for _ in 0..3700 {
        pass(&mut run, &[], false);
    }
    assert!(run.ended() && run.running(), "Time! and the wait for a restart");
    run.plan([OFF, ANY, NORMAL, DISC], None, 0, || TRAINING);
    assert!(!run.running(), "Off");
    let mut run = started(FREE, ANY, 7);
    run.plan([FREE, ANY, NORMAL, DISC], None, 0, || MATCH);
    assert!(!run.running(), "the scene left");
}

/// V2.2 (D2 c): the box a target spawns in is the stage's own camera range, cut to what is on screen; the draft's
/// +/-`HALF_WIDTH` when the game's range is missing or is not a range.
fn eggs_stage_range() {
    assert_eq!(range(Some([-220.4, 220.4, 153.17, -80.0])), [-220.4, 220.4], "Training's own camera range (Eden)");
    for bad in [None, Some([f32::NAN, 10.0, 0.0, 0.0]), Some([5.0, -5.0, 0.0, 0.0]), Some([-10.0, 10.0, 0.0, 0.0])] {
        assert_eq!(range(bad), BOX, "{bad:?}");
    }
    // Zoomed out to a screen 192 wide: where the targets are, flight and landing.
    let spread = |range: [f32; 2]| {
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        // Seeds spread over the generator's range (consecutive ones start it almost alike).
        for seed in 0..200u32 {
            let mut run = started(FREE, ANY, seed.wrapping_mul(0x9e37_79b9));
            settle(&mut run, View([10.0, 10.0], [0.0, -300.0], range));
            run.step(false, 0.0, &[], false, 0);
            for _ in 0..200 {
                let x = run.egg_at().unwrap()[0];
                (lo, hi) = (lo.min(x), hi.max(x));
                pass(&mut run, &[], false);
            }
        }
        [lo, hi]
    };
    let [lo, hi] = spread([-40.0, 30.0]);
    assert!(lo >= -40.0 - 1e-3 && hi <= 30.0 + 1e-3 && hi - lo > 40.0, "a narrow stage bounds the targets: {lo} {hi}");
    let [lo, hi] = spread([-220.0, 220.0]);
    assert!(lo < -HALF_WIDTH && hi > HALF_WIDTH && lo >= -92.0 - 1e-3 && hi <= 92.0 + 1e-3, "a wide one leaves the screen: {lo} {hi}");
}
/// A run with the CPU as the target.
fn started_cpu(mode: u8, rule: u8, seed: u32) -> Run {
    let mut run = Run::IDLE;
    assert_eq!(run.plan([mode, rule, NORMAL, CPU], Some(seed), 0, || TRAINING), Plan::Read);
    // Zoomed out, so the box has room 30 away from every spot.
    settle(&mut run, View([14.0, 14.0], [0.0, -300.0], BOX));
    run
}

/// V2.1 (D1 c): with the CPU as the target no disc is drawn; the CPU is placed on the spawn spot with 0% and held there, a
/// hit the game registers on it (its damage rises) that the rule lets count breaks the target (the game's own hitlag and
/// knockback play), and after the hitlag the CPU is put back at the next spawn spot.
fn eggs_target_cpu() {
    let mut run = started_cpu(FREE, ANY, 7);
    assert_eq!(run.cpu_pass(0.0, false), None, "nothing to place before the first target");
    pass(&mut run, &[], false);
    assert!(target(&mut run).is_none() && run.look(false, || None).iter().all(Option::is_none), "the CPU is the target: no disc");
    let (spot, fresh) = run.cpu_pass(0.0, false).expect("placed");
    assert!(fresh && spot[1] == 0.0 && spot[0].abs() <= HALF_WIDTH, "on the ground, in the box: {spot:?}");
    for _ in 0..30 {
        pass(&mut run, &[], false);
        assert_eq!(run.cpu_pass(0.0, false), Some((spot, false)), "held, tick after tick");
    }
    // Still damaged from before the placement took: asked again until it reads 0.
    let mut stale = started_cpu(FREE, ANY, 7);
    pass(&mut stale, &[], false);
    assert_eq!(stale.cpu_pass(40.0, false).map(|p| p.1), Some(true));
    assert_eq!(stale.cpu_pass(40.0, false).map(|p| p.1), Some(true), "a stale 40% is not a hit");
    assert_eq!((stale.score, stale.cpu_pass(0.0, false).map(|p| p.1)), (0, Some(false)));
    // A hit: the count rises, the game plays its hitlag and knockback (nothing asked of the CPU), then the next spot.
    assert_eq!(run.cpu_pass(9.0, false), None);
    assert_eq!(run.score, 1);
    assert_eq!(notice_after(&mut run), "Eggs 1");
    assert_eq!(run.cpu_pass(9.0, true), None, "in hitlag");
    let (next, fresh) = run.cpu_pass(9.0, false).expect("the next spawn");
    assert!(fresh && gap(next, spot) >= MIN_GAP, "reset at the next spot: {next:?}");
    // The rule filters: Aerials counts only in an aerial; Hits of 12%+ only a hit of 12 or more. A hit that does not count
    // resets the CPU where it was.
    let mut aerials = started_cpu(FREE, AERIALS, 7);
    pass(&mut aerials, &[], false);
    let (spot, _) = aerials.cpu_pass(0.0, false).unwrap();
    aerials.cpu_pass(0.0, false);
    assert_eq!(aerials.cpu_pass(5.0, false), None);
    assert_eq!((aerials.score, aerials.cpu_pass(5.0, false)), (0, Some((spot, true))), "a jab does not count: reset in place");
    pass(&mut aerials, &[], true);
    aerials.cpu_pass(0.0, false);
    aerials.cpu_pass(5.0, false);
    assert_eq!(aerials.score, 1, "in an aerial it counts");
    let mut strong = started_cpu(FREE, STRONG, 7);
    pass(&mut strong, &[], false);
    strong.cpu_pass(0.0, false);
    strong.cpu_pass(0.0, false);
    strong.cpu_pass(11.9, false);
    assert_eq!(strong.score, 0);
    strong.cpu_pass(0.0, false);
    strong.cpu_pass(0.0, false);
    strong.cpu_pass(12.0, false);
    assert_eq!(strong.score, 1);
    // The last target of 10 ends the run: the CPU is released where it is.
    let mut ten = started_cpu(TEN, ANY, 7);
    pass(&mut ten, &[], false);
    for _ in 0..10 {
        ten.cpu_pass(0.0, false);
        ten.cpu_pass(0.0, false);
        ten.cpu_pass(5.0, false);
        ten.cpu_pass(5.0, true);
        pass(&mut ten, &[], false);
    }
    assert!(ten.ended() && ten.cpu_pass(0.0, false).is_none(), "released at the end");
    // Off: no run, nothing asked.
    let mut off = Run::IDLE;
    assert_eq!(off.cpu_pass(0.0, false), None);
}

/// The notice after one more pass of Player 1.
fn notice_after(run: &mut Run) -> String {
    pass(run, &[], false);
    notice(run)
}
/// A restart (a menu close, L+R+A, Change Stage) spawns its first target by the camera it keeps, not by the one from
/// before or the one right after the reset: L+R+A sends the camera gliding back for a moment. Eden `wa-eggsfly-9c16babe-1`
/// and `-a6266ac1-1` spawned a target (x 32.7, x 70.3) by the panned camera of the walk before the reset, then drew it off
/// screen for 95 of 96 and 92 of 93 ticks.
fn eggs_restart_waits_for_camera() {
    let panned = View([27.09, 27.09], [-900.0, -264.4], BOX);
    let restarts: [fn(&mut Run); 2] = [|run| run.reset(11), |run| _ = run.plan([FREE, ANY, NORMAL, DISC], Some(11), 0, || TRAINING)];
    for restart in restarts {
        for seed in 0..40 {
            let mut run = started(FREE, ANY, seed);
            settle(&mut run, panned);
            restart(&mut run);
            assert_eq!(pass(&mut run, &[], false), None, "seed {seed}: no target before the draw published the new camera");
            assert!(target(&mut run).is_none());
            // The camera still shows the panned view for a draw, then glides to the default over a few more.
            for k in 0..7u8 {
                let at = View([27.09; 2], [-900.0 + 1308.7 * f32::from(k) / 7.0, -264.4], BOX);
                run.look(true, || Some(at));
                assert_eq!(pass(&mut run, &[], false), None, "seed {seed}: the camera is gliding (draw {k})");
            }
            for _ in 0..SETTLE {
                run.look(true, || Some(RESET_CENTRE));
                assert_eq!(pass(&mut run, &[], false), None, "seed {seed}: the camera is not yet still");
            }
            run.look(true, || Some(RESET_CENTRE));
            pass(&mut run, &[], false);
            for _ in 0..200 {
                let egg = run.egg_at().expect("spawned");
                let ([cx, cy], pad) = ([0, 1].map(|i| egg[i] * RESET_CENTRE.0[i] + RESET_CENTRE.1[i]), RADIUS * RESET_CENTRE.0[0] - 0.01);
                assert!(cx.abs() <= 960.0 - pad && cy.abs() <= 540.0 - pad, "seed {seed}: {egg:?} off screen");
                pass(&mut run, &[], false);
            }
        }
    }
    // Only a run's first target waits: the next ones spawn at once, by the camera as it is (the player may be walking).
    let mut run = started(FREE, ANY, 3);
    pass(&mut run, &[], false);
    run.look(true, || Some(panned));
    let egg = run.egg_at().unwrap();
    pass(&mut run, &[hit(egg, 0.0)], false);
    assert_eq!(run.score, 1);
    assert!(run.egg_at().is_some_and(|next| next != egg), "the next target spawned on the break's pass");
}
/// V2.1: with Hide CPU On the CPU cannot be the target: Hide CPU wins and the disc is used (SPEC V2.6 row 38).
fn eggs_target_cpu_hidden() {
    use crate::model::settings::Snapshot;
    let mut snapshot = Snapshot::factory();
    assert_eq!(choices(&snapshot), [OFF, ANY, NORMAL, DISC]);
    snapshot.set(ROW_TARGET, &[0, 1]).unwrap();
    assert_eq!(choices(&snapshot)[3], CPU);
    snapshot.set(ROW_HIDE_CPU, &[1, 0]).unwrap();
    assert_eq!(choices(&snapshot)[3], DISC, "Hide CPU wins");
    snapshot.set(ROW_HIDE_CPU, &[0, 1]).unwrap();
    assert_eq!(choices(&snapshot)[3], CPU, "Off: the CPU is the target again");
    assert_eq!(choice(&snapshot, ROW_PRACTICE_CPU), 0, "CPU during a game: Hide first");
}

/// V2.1: a break freezes the breaker for `HITLAG` ticks in native Training only (the waiting room waits for the LDN
/// check).
fn eggs_hitlag() {
    assert_eq!(HITLAG, 6);
    assert_eq!(hitlag(TRAINING), Some(6));
    assert_eq!([hitlag(LOBBY), hitlag(MATCH)], [None; 2]);
}

/// V2.1 row 24: a break starts one animation at the spot: the broken target grows and fades out, gone after
/// `BURST_TICKS` (at most 20); the next target spawns on the break's pass all the same.
fn eggs_break_anim_timing() {
    const { assert!(BURST_TICKS <= 20) };
    let mut run = started(FREE, ANY, 7);
    pass(&mut run, &[], false);
    let spot = run.egg_at().unwrap();
    pass(&mut run, &[hit(spot, 0.0)], false);
    let looks = own(&mut run);
    assert_eq!(looks.len(), 2, "the next target and one animation");
    let cracked = EggLook { scale: 1.0, alpha: 255, picture: EggPicture::Cracked };
    assert_eq!(looks[1], (spot, cracked), "starts at the spot, the cracked picture at full size");
    let mut last = looks[1].1;
    for tick in 1..BURST_TICKS {
        pass(&mut run, &[], false);
        let looks = own(&mut run);
        assert_eq!(looks.len(), 2, "tick {tick}");
        assert!(looks[1].1.scale > last.scale && looks[1].1.alpha < last.alpha && looks[1].1.alpha > 0, "grows and fades");
        last = looks[1].1;
    }
    pass(&mut run, &[], false);
    assert_eq!(own(&mut run).len(), 1, "gone after {BURST_TICKS} ticks, nothing left behind");
    // The last target of a run plays its animation too, then the run waits.
    let mut run = started(TEN, ANY, 7);
    pass(&mut run, &[], false);
    while run.score < 10 {
        let egg = run.egg_at().unwrap();
        pass(&mut run, &[hit(egg, 0.0)], false);
        for _ in 0..BURST_TICKS * u32::from(run.score < 10) {
            pass(&mut run, &[], false);
        }
    }
    assert_eq!(own(&mut run).len(), 1, "no target, the last animation");
    assert_eq!(run.plan([TEN, ANY, NORMAL, DISC], None, 0, || TRAINING), Plan::Read, "the animation runs on");
    for _ in 0..BURST_TICKS {
        assert_eq!(pass(&mut run, &[], false), None, "the end notice stays");
    }
    assert!(own(&mut run).is_empty());
    assert_eq!(run.plan([TEN, ANY, NORMAL, DISC], None, 0, || panic!("ended, no animation: nothing is read")), Plan::Idle);
}

/// V2.1 row 25: two breaks in a row give two animations, a third replaces the oldest; pause freezes them.
fn eggs_break_anim_overlap() {
    let mut run = started(FREE, ANY, 7);
    pass(&mut run, &[], false);
    let mut spots = vec![];
    for _ in 0..3 {
        let egg = run.egg_at().unwrap();
        spots.push(egg);
        pass(&mut run, &[hit(egg, 0.0)], false);
    }
    let looks = own(&mut run);
    assert_eq!(looks.iter().skip(1).map(|l| l.0).collect::<Vec<_>>(), vec![spots[2], spots[1]], "the newest two");
    for _ in 0..100 {
        run.step(false, 0.0, &[], false, 7);
    }
    assert_eq!(own(&mut run), looks, "paused: frozen");
}

/// §2.3b row 23: a resting target stays until hit; pause holds it mid-flight.
fn eggs_rest_and_pause() {
    let mut run = started(FREE, ANY, 7);
    pass(&mut run, &[], false);
    for _ in 0..20 {
        pass(&mut run, &[], false);
    }
    let flying = run.egg_at();
    for _ in 0..300 {
        run.step(false, 0.0, &[], false, 7);
    }
    assert_eq!(run.egg_at(), flying, "paused mid-flight");
    for _ in 0..200 {
        pass(&mut run, &[], false);
    }
    let rest = run.egg_at();
    for _ in 0..3000 {
        pass(&mut run, &[], false);
    }
    assert_eq!((run.egg_at(), run.score), (rest, 0), "rests until hit, no timeout");
}

fn eggs_timer_end() {
    let mut run = started(TIMED, AERIALS, 3);
    let mut end = None;
    for n in 1..=4000 {
        let hits = run.egg_at().filter(|_| n <= 3).map(|egg| hit(egg, 0.0));
        if pass(&mut run, hits.as_slice(), true) == Some(END_FRAMES) {
            end = Some(n);
            break;
        }
    }
    assert_eq!(end, Some(3600), "the run lasts 3,600 ticks");
    assert_eq!((run.score, notice(&run)), (2, "Time! 2 eggs (best 2)".to_owned()), "the first pass only spawns");
    assert!(target(&mut run).is_none(), "the eggs stop");
    assert!(run.look(true, || panic!("an ended run: no projection"))[0].is_none());
    assert_eq!(run.plan([TIMED, AERIALS, NORMAL, DISC], None, 0, || TRAINING), Plan::Idle, "it waits for a restart");
    restart(&mut run, 5);
    for _ in 0..3600 {
        pass(&mut run, &[], true);
    }
    assert_eq!(notice(&run), "Time! 0 eggs (best 2)", "best per fighter for the session");
    restart(&mut run, 5);
    for _ in 0..3600 {
        let tick = run.keeper(P1);
        run.step(tick, 0.0, &[], true, 8);
    }
    assert_eq!(notice(&run), "Time! 0 eggs (best 0)", "another fighter has its own best");
}

fn eggs_rule_aerials() {
    let mut run = started(TIMED, AERIALS, 7);
    pass(&mut run, &[], false);
    let egg = run.egg_at().unwrap();
    pass(&mut run, &[hit(egg, 20.0)], false);
    assert_eq!(run.score, 0, "a jab or a tilt does not break it");
    let egg = run.egg_at().unwrap();
    pass(&mut run, &[hit(egg, 1.0)], true);
    assert_eq!(run.score, 1, "an aerial does, whatever its damage");
    assert_eq!(ATTACK_AIR, 0x36);
}

fn eggs_rule_any() {
    let mut run = started(TIMED, ANY, 7);
    pass(&mut run, &[], false);
    let egg = run.egg_at().unwrap();
    pass(&mut run, &[hit(egg, 1.0)], false);
    assert_eq!(run.score, 1, "the same jab breaks it");
}

fn eggs_rule_damage() {
    let egg = [0.0, 10.0];
    assert!(!breaks(STRONG, true, &hit(egg, 3.0), egg), "a jab, under 12");
    assert!(breaks(STRONG, false, &hit(egg, 12.0), egg), "12 is enough");
    assert!(breaks(STRONG, false, &hit(egg, 16.0), egg), "an up smash");
    assert!(!breaks(STRONG, false, &hit([30.0, 10.0], 16.0), egg), "no overlap");
    let mut run = started(TIMED, STRONG, 7);
    pass(&mut run, &[], false);
    let egg = run.egg_at().unwrap();
    pass(&mut run, &[hit(egg, 3.0)], true);
    pass(&mut run, &[hit(egg, 16.0)], false);
    assert_eq!(run.score, 1, "the rule of the run's menu close");
}

fn eggs_no_projectiles() {
    let sphere = |kind: u32, slot: usize, end: usize, x: f32| Sphere {
        key: sphere_key(kind, slot, end),
        joint: 1,
        pos: [x, 0.0, 0.0],
        scale: hitbox::sphere_scale(3.0),
        tint: [1.0; 3],
    };
    let spheres = [sphere(KIND_HIT, 2, 1, 9.0), sphere(KIND_GRAB, 0, 0, 1.0), sphere(KIND_HURT, 4, 0, 2.0), sphere(KIND_HIT, 2, 0, 5.0)];
    let mut out = [Hit::default(); 8];
    let n = pills(&spheres, |s| Some(s.pos), |slot| slot as f32 * 10.0, &mut out);
    assert_eq!(n, 1, "only the fighter's own attack slots; grab and hurt spheres are not hits");
    assert_eq!((out[0].0, out[0].1, out[0].3), ([5.0, 0.0, 0.0], [9.0, 0.0, 0.0], 20.0), "a capsule is one pill");
    assert!((out[0].2 - 3.0).abs() < 1e-4);
    let mut run = started(TIMED, ANY, 7);
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC], None, 1, || panic!("a CPU's pass reads nothing")), Plan::Idle);
}

fn eggs_free() {
    let mut run = started(FREE, ANY, 7);
    assert_eq!(pass(&mut run, &[], false), Some(u16::MAX));
    assert_eq!(notice(&run), "Eggs 0");
    for n in 0..10_000 {
        let hits = run.egg_at().filter(|_| n % 50 == 0).map(|egg| hit(egg, 0.0));
        assert_ne!(pass(&mut run, hits.as_slice(), false), Some(END_FRAMES), "no timer");
    }
    assert_eq!((run.score, notice(&run)), (200, "Eggs 200".to_owned()));
    assert_eq!(pass(&mut run, &[], false), None, "rewritten only when it changes");
    assert_eq!(run.plan([FREE, ANY, NORMAL, DISC], None, 0, || TRAINING), Plan::Read);
}

fn eggs_lra_restart() {
    let mut run = started(TIMED, ANY, 7);
    for _ in 0..600 {
        let hits = run.egg_at().map(|egg| hit(egg, 0.0));
        pass(&mut run, hits.as_slice(), true);
    }
    assert!(run.score > 0);
    restart(&mut run, 11);
    assert!(target(&mut run).is_none(), "a new egg comes with the next pass");
    assert_eq!(pass(&mut run, &[], false), Some(u16::MAX));
    assert_eq!((run.score, notice(&run)), (0, "Eggs 0   1:00".to_owned()));
    let mut idle = Run::IDLE;
    idle.reset(11);
    assert_eq!(idle.plan([TIMED, ANY, NORMAL, DISC], None, 0, || TRAINING), Plan::Idle, "a reset starts nothing before the menu closed");
}

fn eggs_pause_frames() {
    let mut run = started(TIMED, ANY, 7);
    pass(&mut run, &[], false);
    let egg = run.egg_at();
    for _ in 0..500 {
        assert_eq!(run.step(false, 0.0, &[], false, 7), None);
    }
    assert_eq!((notice(&run), run.egg_at()), ("Eggs 0   1:00".to_owned(), egg), "paused: no tick, the egg holds still");
    pass(&mut run, &[], false);
    assert_ne!(run.egg_at(), egg, "it moves again once the clock runs");
}

/// The target and its break animations are pictures of their own, drawn whether Hitbox Visualization is On or Off
/// (the runtime draws `Run::look`'s items whatever `on` says): the target is the whole picture and nothing else shows
/// before a break, a break the cracked one.
fn eggs_look_pictures() {
    let mut run = started(TIMED, ANY, 7);
    pass(&mut run, &[], false);
    let looks = run.look(false, || None);
    assert_eq!(looks.map(|l| l.map(|l| l.1)), [Some(EGG_WHOLE), None, None], "the whole target alone");
    let egg = target(&mut run).unwrap();
    pass(&mut run, &[hit(egg, 0.0)], false);
    assert_eq!(run.look(false, || None).map(|l| l.map(|l| l.1.picture)), [Some(EggPicture::Whole), Some(EggPicture::Cracked), None]);
}
fn eggs_state_load() {
    let mut run = started(TIMED, ANY, 7);
    pass(&mut run, &[], false);
    let egg = run.egg_at().unwrap();
    pass(&mut run, &[hit(egg, 0.0)], false);
    let (egg, text) = (run.egg_at(), notice(&run));
    // A load moves the fighters (here 80 up); the run has no restore path: the egg, the score and the clock go on.
    assert_eq!(run.step(false, 80.0, &[], false, 7), None);
    assert_eq!((run.egg_at(), notice(&run), run.score), (egg, text, 1));
}

fn eggs_swap_and_stage() {
    let mut run = started(TIMED, ANY, 7);
    for _ in 0..60 {
        pass(&mut run, &[], false);
    }
    assert_eq!(notice(&run), "Eggs 0   0:59");
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC], None, 0, || SWAPPING), Plan::Idle, "a hot swap idles the run");
    let swapped = 0x3000;
    let ticked = (0..61).filter(|_| run.keeper(swapped)).count();
    assert_eq!(ticked, 59, "the new fighter takes the clock after two passes");
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC], None, 0, || TRAINING), Plan::Read, "the fighter cycle continues the run");
    restart(&mut run, 3);
    assert_eq!(pass(&mut run, &[], false), Some(u16::MAX));
    assert_eq!(notice(&run), "Eggs 0   1:00", "Change Stage restarts it (the reset hub)");
}

fn eggs_nana() {
    let mut run = started(TIMED, AERIALS, 7);
    for _ in 0..60 {
        for fighter in [P1, NANA] {
            assert_eq!(run.plan([TIMED, AERIALS, NORMAL, DISC], None, 0, || TRAINING), Plan::Read, "both are entry 0");
            let tick = run.keeper(fighter);
            run.step(tick, 0.0, &[], false, 7);
        }
    }
    assert_eq!(notice(&run), "Eggs 0   0:59", "Nana does not tick the clock twice");
    let egg = run.egg_at().unwrap();
    let tick = run.keeper(NANA);
    run.step(tick, 0.0, &[hit(egg, 4.0)], true, 7);
    assert_eq!(run.score, 1, "Nana's aerial breaks it");
}

fn eggs_training_gate() {
    let mut run = started(TIMED, ANY, 7);
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC], None, 0, || LOBBY), Plan::Read, "the waiting room's practice");
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC], None, 0, || MATCH), Plan::Clear, "a match: the run stops, its notice goes");
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC], None, 0, || panic!("stopped: nothing is read")), Plan::Idle);
    assert!(target(&mut run).is_none());
}

fn eggs_off_no_calls() {
    let mut run = Run::IDLE;
    assert_eq!(run.plan([OFF, ANY, NORMAL, DISC], Some(1), 0, || panic!("Off reads nothing")), Plan::Idle);
    assert!(run.look(true, || panic!("Off: no projection"))[0].is_none());
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC], None, 0, || panic!("no run before the menu closes")), Plan::Idle);
    let mut run = started(TIMED, ANY, 7);
    pass(&mut run, &[], false);
    assert_eq!(run.plan([OFF, ANY, NORMAL, DISC], None, 0, || panic!("Off reads nothing")), Plan::Clear, "the notice goes the next pass");
    assert!(target(&mut run).is_none(), "and the egg");
    assert_eq!(run.plan([OFF, ANY, NORMAL, DISC], None, 0, || panic!("Off reads nothing")), Plan::Idle);
}

/// This module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("eggs_rows_and_help", eggs_rows_and_help as fn()),
        ("eggs_run_start", eggs_run_start as fn()),
        ("eggs_hit_overlap", eggs_hit_overlap as fn()),
        ("eggs_respawn_distance", eggs_respawn_distance as fn()),
        ("eggs_on_screen", eggs_on_screen as fn()),
        ("eggs_spawn_ignores_the_notice", eggs_spawn_ignores_the_notice as fn()),
        ("eggs_notice_band", eggs_notice_band as fn()),
        ("eggs_motion_normal", eggs_motion_normal as fn()),
        ("eggs_speed_scales_time", eggs_speed_scales_time as fn()),
        ("eggs_rest_and_pause", eggs_rest_and_pause as fn()),
        ("eggs_hitlag", eggs_hitlag as fn()),
        ("eggs_running", eggs_running as fn()),
        ("eggs_restart_waits_for_camera", eggs_restart_waits_for_camera as fn()),
        ("eggs_stage_range", eggs_stage_range as fn()),
        ("eggs_target_cpu", eggs_target_cpu as fn()),
        ("eggs_target_cpu_hidden", eggs_target_cpu_hidden as fn()),
        ("eggs_break_anim_timing", eggs_break_anim_timing as fn()),
        ("eggs_break_anim_overlap", eggs_break_anim_overlap as fn()),
        ("eggs_timer_end", eggs_timer_end as fn()),
        ("eggs_count_mode", eggs_count_mode as fn()),
        ("eggs_rule_aerials", eggs_rule_aerials as fn()),
        ("eggs_rule_any", eggs_rule_any as fn()),
        ("eggs_rule_damage", eggs_rule_damage as fn()),
        ("eggs_no_projectiles", eggs_no_projectiles as fn()),
        ("eggs_free", eggs_free as fn()),
        ("eggs_lra_restart", eggs_lra_restart as fn()),
        ("eggs_pause_frames", eggs_pause_frames as fn()),
        ("eggs_look_pictures", eggs_look_pictures as fn()),
        ("eggs_state_load", eggs_state_load as fn()),
        ("eggs_swap_and_stage", eggs_swap_and_stage as fn()),
        ("eggs_nana", eggs_nana as fn()),
        ("eggs_training_gate", eggs_training_gate as fn()),
        ("eggs_off_no_calls", eggs_off_no_calls as fn()),
    ]);
}
}
