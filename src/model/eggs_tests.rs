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
const RESET_CENTRE: View = View([27.09, 27.09], [408.7, -264.4], BOX, None);

fn started(mode: u8, rule: u8, seed: u32) -> Run {
    let mut run = Run::IDLE;
    assert_eq!(run.plan([mode, rule, NORMAL, DISC, IN_VIEW], Some(seed), 0, || TRAINING), Plan::Read);
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

/// A reset (the reset hub) and the draw's next tick, which publishes the camera the first target spawns by.
fn restart(run: &mut Run, seed: u32) {
    run.reset(seed);
    settle(run, RESET_CENTRE);
}

/// rc.15: the first seed from 0 whose first target spawns on the ground (targets may also spawn perched in the air), for
/// the rows that measure the whole Melee flight from the ground.
fn ground_seed(target: u8, speed: u8, view: View) -> u32 {
    (0..).find(|&s| {
        let mut run = started_with(FREE, ANY, speed, target, s, view);
        run.step(false, 0.0, &[], false, 0);
        run.egg.is_some() && !run.perch
    }).unwrap()
}

fn gap(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn eggs_rows_and_help() {
    let at = |id: &str| ROWS.iter().position(|r| r.id == id).unwrap_or_else(|| panic!("row {id}"));
    let mode = at(ROW);
    let rows = [&ROWS[mode], &ROWS[at(ROW_BREAK)], &ROWS[at(ROW_TARGET)], &ROWS[at(ROW_SPEED)], &ROWS[at(ROW_AREA)], &ROWS[at("eggsercise_hit_streak")], &ROWS[at(ROW_PRACTICE_CPU)]];
    // D7 (b), the operator 2026-11-14: their own group, appended to `GROUPS` so no group index moves. rc.15 practice menu
    // (2026-11-18): the Eggs-ercise sub-screen's group (`sub`), after its Start row; Practice games lists the game.
    let g = usize::from(rows[0].group);
    assert_eq!((GROUPS[g].title, GROUPS[g].page, GROUPS[g].advanced, GROUPS[g].sub), ("Eggs-ercise", Page::FightersStages, false, true));
    assert_eq!(GROUPS[usize::from(crate::model::menu::table::GAMES)].title, "Practice games");
    assert!(ROWS.iter().filter(|r| usize::from(r.group) == g).map(|r| r.title).eq(["Start", "Eggs-ercise", "Breaks on", "Target", "Target speed", "Spawn area", "Off-stage zone…", "Moving paths", "Target expiry", "Hit streak", "CPU during a game"]));
    assert!(ROWS.iter().filter(|r| usize::from(r.group) == g).skip(1).map(|r| r.id).eq([ROW, ROW_BREAK, ROW_TARGET, ROW_SPEED, ROW_AREA, "eggsercise_zone", "eggsercise_paths", "eggsercise_expiry", "eggsercise_hit_streak", ROW_PRACTICE_CPU]), "the settings follow Start, in order");
    let values = |r: &crate::model::menu::table::Row| match r.control {
        Control::Dropdown(v) => v.to_vec(),
        _ => panic!("{} is not a dropdown", r.title),
    };
    // SPEC V2.4 (D4 a + c) and V2.3 (D3 b: Any attack first, the default); V2.7's rows and help. rc.15 (Q2 A): no Off; the
    // stored row keeps its Off at index 0.
    let modes = vec!["1 minute", "10 targets", "20 targets", "Free practice"];
    assert_eq!((rows[0].title, values(rows[0])), ("Eggs-ercise", modes));
    assert_eq!((rows[1].title, values(rows[1])), ("Breaks on", vec!["Any attack", "Aerials", "Hits of 12%+"]));
    assert_eq!(
        rows[0].help_lines(),
        ("1 minute: break as many as you can. 10 or 20 targets: fastest.", "Free practice: no timer.")
    );
    // rc.15 (docs/DECISIONS.md): projectiles count as Breaks on allows (SPEC V2.3 superseded).
    assert_eq!(rows[1].help_lines(), ("Which of your hits break a target, projectiles included.", "12%+: only attacks that deal 12% or more."));
    // §2.3b's speed row, renamed by V2.2.
    assert_eq!((rows[2].title, values(rows[2])), ("Target", vec!["Disc", "CPU"]));
    assert_eq!(rows[2].help_lines(), ("Disc: a target appears. CPU: the CPU is the target.", "The CPU flies like the disc and is reset after each hit."));
    assert_eq!((rows[3].title, values(rows[3])), ("Target speed", vec!["Normal", "Slow", "Fast"]));
    assert_eq!(
        rows[3].help_lines(),
        ("How fast targets fly. Normal is Melee's Eggs-ercise speed.", "Slow: half speed. Fast: one and a half times as fast.")
    );
    assert!(matches!(rows[4].control, Control::MultiSelect(["In view", "Whole stage", "Off stage"])));
    assert_eq!(
        rows[4].help_lines(),
        ("Each checked area has the same chance on each spawn.", "In view is used when none are checked. Applies at the next Start.")
    );
    assert_eq!((rows[5].title, values(rows[5])), ("Hit streak", vec!["Off", "On"]));
    // V2.6's second row: Hide CPU (B-2's) reads it with `eggs_running`.
    assert_eq!((rows[6].title, values(rows[6])), ("CPU during a game", vec!["Hide", "Show"]));
    assert_eq!(
        rows[6].help_lines(),
        ("Hide: the CPU is hidden while a practice game runs.", "Afterwards it's as Hide CPU says. Show: it stays.")
    );
    for r in rows {
        let (a, b) = r.help_lines();
        assert!(a.chars().count() <= 67 && b.chars().count() <= 67, "{}", r.title);
    }
    for (id, default) in [(ROW, &[1, 0, 0, 0, 0][..]), (ROW_BREAK, &[1, 0, 0]), (ROW_TARGET, &[1, 0]), (ROW_SPEED, &[1, 0, 0]), (ROW_AREA, &[1, 0, 0]), ("eggsercise_hit_streak", &[1, 0]), (ROW_PRACTICE_CPU, &[1, 0])] {
        let def = crate::model::factory::DEFS.iter().find(|d| d.id == id).unwrap_or_else(|| panic!("factory {id}"));
        assert_eq!(def.default, default, "{id}: the first option (Off, Any attack) by default");
    }
    let indices = [OFF, TIMED, TEN, TWENTY, FREE, ANY, AERIALS, STRONG, NORMAL, SLOW, FAST, IN_VIEW, WHOLE];
    assert_eq!(indices, [0, 1, 2, 3, 4, 0, 1, 2, 0, 1, 2, 0, 1], "the rows' option indices");
    assert_eq!(Run::IDLE.rows, [OFF, ANY, NORMAL, DISC, IN_VIEW]);
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
    assert_eq!(run.plan([TEN, ANY, NORMAL, DISC, IN_VIEW], None, 0, || TRAINING), Plan::Idle, "it waits for a restart");
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
    blind.plan([TIMED, AERIALS, NORMAL, DISC, IN_VIEW], Some(7), 0, || TRAINING);
    // The standing notice publishes before the camera, while the target still waits.
    assert_eq!(blind.step(true, 0.0, &[], false, 7), Some(u16::MAX));
    assert!(target(&mut blind).is_none(), "no egg before the draw published the camera");
    assert!(blind.look(false, || panic!("another layout's tick reads nothing"))[0].is_none());
    assert!(blind.look(true, || Some(RESET_CENTRE))[0].is_none(), "the draw's tick takes the camera");
    settle(&mut blind, RESET_CENTRE);
    assert_eq!(blind.step(true, 0.0, &[], false, 7), None, "the initial minute notice is already shown");
    assert!(target(&mut blind).is_some(), "the next pass spawns the egg");
    let mut run = started(TIMED, AERIALS, 7);
    assert!(target(&mut run).is_none(), "the first pass reads the ground before the egg spawns");
    assert_eq!(pass(&mut run, &[], false), Some(u16::MAX), "the standing notice is shown");
    assert_eq!(notice(&run), "Eggs 0   1:00");
    let egg = target(&mut run).expect("one egg");
    assert!(egg[0].abs() <= HALF_WIDTH && (RADIUS..=RADIUS + REACH + 2.0).contains(&egg[1]), "over the ground, launched: {egg:?}");
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
    // rc.15: at reset-center's narrow view the spacing is a third of the floor left beside Player 1, not `MIN_GAP` (which
    // forced every target to the other side: `eggs_spawn_not_alternating`).
    assert!(gap(next, egg) >= 3.0 && next != egg, "{next:?} {egg:?}");
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
        settle(&mut run, View([14.0, 14.0], [0.0, -300.0], BOX, None));
        run.step(false, -10.0, &[], false, 0);
        for _ in 0..20 {
            let last = run.egg_at().unwrap();
            run.step(false, 99.0, &[hit(last, 0.0)], false, 0);
            let next = run.egg_at().unwrap();
            // rc.15: a 100-unit floor beside Player 1 keeps the targets at least 20 apart (`MIN_GAP` where the floor is longer).
            assert!(gap(next, last) >= 20.0, "seed {seed}: {next:?} from {last:?}");
            // rc.15: on the run's ground (no floor read) or in the air above it, within `REACH`.
            assert!(next[0].abs() <= HALF_WIDTH && (-10.0 + RADIUS..=-10.0 + RADIUS + REACH).contains(&next[1]), "seed {seed}: over the run's ground {next:?}");
        }
        assert_eq!(run.score, 20);
    }
}

fn eggs_on_screen() {
    // Reset-center; the camera following Player 1 to the right (the box's left part off screen); zoomed out; zoomed in
    // (the apex clamped under the top).
    let views = [RESET_CENTRE, View([27.09, 27.09], [-900.0, -264.4], BOX, None), View([14.0, 14.0], [0.0, -300.0], BOX, None), View([40.0, 40.0], [0.0, -300.0], BOX, None)];
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

/// rc.15 games-fix (card report on reship-4 `8100b5e9`): at the camera Eden measured in Training (`RESET_CENTRE`), a run of
/// breaks must not just alternate between the band's two ends. RED on `c212bc50`: the ~39-unit band and `MIN_GAP` 30 left
/// no room near the last target, so every spawn after the first was the `far` fallback, left, right, left.
fn eggs_spawn_not_alternating() {
    for seed in 0..40 {
        let mut run = started(FREE, ANY, seed);
        run.step(false, 0.0, &[], false, 0);
        let mut xs = Vec::new();
        for _ in 0..40 {
            let egg = run.egg_at().unwrap();
            xs.push(egg[0]);
            run.step(false, 0.0, &[hit(egg, 0.0)], false, 0);
        }
        let bins: std::collections::BTreeSet<i32> = xs.iter().map(|x| (x / 5.0).floor() as i32).collect();
        assert!(bins.len() >= 4, "seed {seed}: only {} 5-unit columns: {xs:?}", bins.len());
        let mid = (xs.iter().copied().fold(f32::MAX, f32::min) + xs.iter().copied().fold(f32::MIN, f32::max)) / 2.0;
        let flips = xs.windows(2).filter(|w| (w[0] < mid) != (w[1] < mid)).count();
        assert!(flips < 34, "seed {seed}: {flips} of 39 spawns switched sides: {xs:?}");
        let ends = xs.iter().filter(|x| xs.iter().filter(|y| (*y - *x).abs() < 0.01).count() > 3).count();
        assert!(ends < 20, "seed {seed}: {ends} spawns repeat the same spot: {xs:?}");
    }
}

/// A Battlefield-like stage (main floor ±70 at 0, side platforms at 27, the top one at 54), seen by the Fixed camera: the
/// whole stage on screen, x ±160 and y −65 to 115.
const STAGE: [Floor; 4] = [[[-70.0, 0.0], [70.0, 0.0]], [[-50.0, 27.0], [-20.0, 27.0]], [[20.0, 27.0], [50.0, 27.0]], [[-15.0, 54.0], [15.0, 54.0]]];
const FIXED: View = View([6.0, 6.0], [0.0, -150.0], [-200.0, 200.0], None);

/// The spawns of `count` breaks in a row on `floors`, Player 1 at `p1`.
fn spawns(seed: u32, floors: &[Floor], p1: [f32; 2], count: usize) -> Vec<[f32; 2]> {
    let mut run = started(FREE, ANY, seed);
    run.set_floors(floors);
    run.sense(p1);
    settle(&mut run, FIXED);
    run.step(false, 0.0, &[], false, 0);
    (0..count)
        .map(|_| {
            let egg = run.egg_at().unwrap();
            run.step(false, 0.0, &[hit(egg, 0.0)], false, 0);
            egg
        })
        .collect()
}

/// The floor a spawn stands over: within its x span, at most `REACH` below the disc's bottom.
fn floor_under(at: [f32; 2], floors: &[Floor]) -> Option<f32> {
    floors.iter().find(|f| (f[0][0]..=f[1][0]).contains(&at[0]) && (0.0..=REACH + 1e-3).contains(&(at[1] - RADIUS - f[0][1]))).map(|f| f[0][1])
}

/// rc.15 (the primary's ruling 15:30Z on the operator's card report): targets appear at random places across the stage:
/// every 10-unit column of the main floor, the platforms' heights and the air above them, never inside Player 1, and no
/// forced side.
fn eggs_spawn_covers_the_stage() {
    let mut columns = std::collections::BTreeSet::new();
    let (mut heights, mut air, mut flips, mut total) = ([0usize; 3], 0, 0, 0);
    for seed in 0..20 {
        let xs = spawns(seed, &STAGE, [-30.0, 27.0], 60);
        for at in &xs {
            let floor = floor_under(*at, &STAGE).unwrap_or_else(|| panic!("seed {seed}: {at:?} over no floor"));
            heights[[0.0, 27.0, 54.0].iter().position(|y| *y == floor).unwrap()] += 1;
            air += usize::from(at[1] - RADIUS - floor > 1.0);
            columns.insert((at[0] / 10.0).floor() as i32);
            assert!((at[0] + 30.0).hypot(at[1] - 27.0 - BODY) >= P1_GAP, "seed {seed}: {at:?} inside Player 1");
            let [cx, cy] = [0, 1].map(|i| at[i] * FIXED.0[i] + FIXED.1[i]);
            assert!(cx.abs() <= 960.0 && cy.abs() <= 540.0, "seed {seed}: {at:?} off the Fixed view");
        }
        assert!(xs.windows(2).all(|w| gap(w[0], w[1]) >= MIN_GAP), "seed {seed}: the whole stage keeps `MIN_GAP`");
        flips += xs.windows(2).filter(|w| (w[0][0] < 0.0) != (w[1][0] < 0.0)).count();
        total += xs.len() - 1;
    }
    assert_eq!(columns, (-6..6).collect(), "every column of the main floor, the drift margin in from its ends");
    assert!(heights.iter().all(|h| *h >= 60), "the ground and both platform heights: {heights:?}");
    assert!((300..=900).contains(&air), "about half in the air above a floor: {air} of 1200");
    assert!(flips * 10 < total * 7, "no forced side: {flips} of {total} switched");
}

fn whole_stage_places_targets_outside_the_view() {
    const FLOORS: [Floor; 2] = [[[-200.0, 0.0], [200.0, 0.0]], [[-120.0, 150.0], [120.0, 150.0]]];
    const GROUND: [Floor; 1] = [[[-200.0, 0.0], [200.0, 0.0]]];
    let view = View([16.0; 2], [0.0, -300.0], [-200.0, 200.0], Some([-50.0, 180.0]));
    assert_eq!(vertical(Some([-200.0, 200.0, 180.0, -50.0])), Some([-50.0, 180.0]));
    assert_eq!(vertical(Some([-200.0, 200.0, f32::NAN, -50.0])), None);
    assert_eq!(vertical(Some([-200.0, 200.0, -50.0, 180.0])), None);
    assert_eq!(vertical(None), None);

    let mut run = started_with_area(FREE, ANY, NORMAL, DISC, WHOLE, 0, view);
    run.set_floors(&FLOORS);
    run.sense([0.0, 0.0]);
    run.step(false, 0.0, &[], false, 0);
    let mut outside_view = false;
    for _ in 0..80 {
        let egg = run.egg_at().expect("a target");
        assert!((-200.0..=200.0).contains(&egg[0]), "{egg:?} outside the stage range");
        assert!((-50.0..=180.0).contains(&egg[1]), "{egg:?} outside the stage vertical bounds");
        assert!(floor_under(egg, &FLOORS).is_some(), "{egg:?} over no floor");
        assert!(gap(egg, [0.0, BODY]) >= P1_GAP, "{egg:?} too close to P1");
        let apex = egg[1] + run.fly[0] * run.fly[0] / (2.0 * run.fly[2]);
        assert!(apex <= 180.0 - RADIUS + 1e-4, "{egg:?} apex {apex} exceeds the stage top");
        outside_view |= egg[0] < -60.0 || egg[0] > 60.0;
        run.step(false, 0.0, &[hit(egg, 0.0)], false, 0);
    }
    assert!(outside_view, "the whole-stage targets stay inside the view");

    let view = View([16.0; 2], [0.0, -300.0], [-200.0, 200.0], None);
    let mut fallback = started_with_area(FREE, ANY, NORMAL, DISC, WHOLE, 1, view);
    fallback.set_floors(&GROUND);
    fallback.sense([0.0, 0.0]);
    fallback.step(false, 0.0, &[], false, 0);
    let egg = fallback.egg_at().expect("a target without a vertical stage bound");
    let screen_top = (CANVAS_HALF[1] - view.1[1]) / view.0[1] - RADIUS;
    let apex = egg[1] + fallback.fly[0] * fallback.fly[0] / (2.0 * fallback.fly[2]);
    assert!(apex <= screen_top + 1e-4, "fallback apex {apex} exceeds screen top {screen_top}");
}

fn in_view_spawns_match_5b6ab8cc() {
    let golden: [([u32; 2], [u32; 3]); 12] = [
        ([3261123816, 1082130432], [1069044204, 1030232039, 1027906470]),
        ([1091397936, 1082130432], [1069044204, 3168429831, 1027906470]),
        ([3259566282, 1082130432], [1069044204, 1035709536, 1027906470]),
        ([1099693208, 1082130432], [1069044204, 3155061427, 1027906470]),
        ([3250075772, 1082130432], [1069044204, 1028994183, 1027906470]),
        ([1065181440, 1114112000], [1069044204, 3170901655, 1027906470]),
        ([1106645472, 1106771968], [1069044204, 1032547047, 1027906470]),
        ([3233179696, 1097750414], [1069044204, 1034079133, 1027906470]),
        ([3258605743, 1094187944], [1069044204, 3180505944, 1027906470]),
        ([3234742432, 1101404777], [1069044204, 1019098810, 1027906470]),
        ([1104480452, 1101432451], [1069044204, 3180517677, 1027906470]),
        ([1108919540, 1111993661], [1069044204, 3181452058, 1027906470]),
    ];
    let mut run = started(FREE, ANY, 0x15_5eed);
    run.set_floors(&STAGE);
    run.sense([-30.0, 27.0]);
    settle(&mut run, FIXED);
    run.step(false, 0.0, &[], false, 0);
    for (egg_bits, flight_bits) in golden {
        let egg = run.egg_at().expect("a target");
        assert_eq!(egg.map(f32::to_bits), egg_bits);
        assert_eq!(run.fly.map(f32::to_bits), flight_bits);
        run.step(false, 0.0, &[hit(egg, 0.0)], false, 0);
    }
}

/// The operator, 11-18 (in view, the player's camera): at the camera Eden measured in Training the targets cover the
/// visible band, rest on the ground or in the air within reach, and each one's whole flight stays on screen.
fn eggs_spawn_in_view_heights() {
    let (mut columns, mut air, mut ground) = (std::collections::BTreeSet::new(), 0, 0);
    for seed in 0..20 {
        let mut run = started(FREE, ANY, seed);
        // Player 1 just right of the band, so the band's whole width is open (ggs_spawn_not_alternating covers P1 in it).
        run.sense([20.0, 0.0]);
        run.step(false, 0.0, &[], false, 0);
        for _ in 0..30 {
            let egg = run.egg_at().unwrap();
            columns.insert((egg[0] / 5.0).floor() as i32);
            if egg[1] - RADIUS > 1.0 { air += 1 } else { ground += 1 }
            let [v, _, g] = run.fly;
            let top = egg[1] + v * v / (2.0 * g);
            let [cx, cy] = [0, 1].map(|i| [egg[0], top][i] * RESET_CENTRE.0[i] + RESET_CENTRE.1[i]);
            assert!(cx.abs() <= 960.0 && cy <= 540.0 - RADIUS * RESET_CENTRE.0[1] + 0.5, "seed {seed}: {egg:?} apex {top} off screen");
            assert!(egg[1] - RADIUS <= REACH + 1e-3, "seed {seed}: {egg:?} out of reach");
            run.step(false, 0.0, &[hit(egg, 0.0)], false, 0);
        }
    }
    assert!(columns.len() >= 7, "the visible band, not two spots: {columns:?}");
    assert!(air >= 100 && ground >= 100, "both on the ground and in the air: {air} air, {ground} ground");
}

/// A stage with a pit (two islands): no target is put over it, nor off the stage.
fn eggs_spawn_skips_pits() {
    let islands = [[[-80.0, 0.0], [-30.0, 0.0]], [[30.0, 0.0], [80.0, 0.0]]];
    for seed in 0..20 {
        for at in spawns(seed, &islands, [-60.0, 0.0], 40) {
            assert!(floor_under(at, &islands).is_some() && at[0].abs() >= 40.0 && at[0].abs() <= 70.0, "seed {seed}: {at:?}");
        }
    }
}

/// The first target waits for the floor read the runtime began (`want_floors`), then stands on a floor it found.
fn eggs_first_target_waits_for_floors() {
    let mut run = started(FREE, ANY, 3);
    assert!(run.floors_wanted());
    run.want_floors();
    settle(&mut run, FIXED);
    run.step(false, 0.0, &[], false, 0);
    assert!(run.egg_at().is_none() && run.pending(), "no target while the floors are read");
    run.set_floors(&STAGE[3..]);
    run.step(false, 0.0, &[], false, 0);
    let egg = run.egg_at().expect("then the target");
    assert_eq!((floor_under(egg, &STAGE[3..]), run.floors_wanted()), (Some(54.0), false), "on the only floor read: {egg:?}");
    run.reset(4);
    assert!(run.floors_wanted(), "a reset reads them again");
}

/// rc.15 (docs/DECISIONS.md, SPEC V2.3 superseded): one of Player 1's articles breaks the disc; only the fighter's own attack
/// freezes it (`melee`).
fn eggs_articles_break() {
    let mut run = started(FREE, ANY, 7);
    pass(&mut run, &[], false);
    let egg = run.egg_at().unwrap();
    run.step_with_depth(false, 0.0, &[], |at| (at == egg).then_some(Some(0.0)), false, 0);
    assert_eq!((run.score, run.melee()), (1, false), "an article's break");
    let egg = run.egg_at().unwrap();
    run.step_with_depth(false, 0.0, &[hit(egg, 0.0)], |_| panic!("the fighter's hit decides first"), false, 0);
    assert_eq!((run.score, run.melee()), (2, true), "the fighter's own break");
    let mut cpu = Run::IDLE;
    cpu.plan([FREE, ANY, NORMAL, CPU, IN_VIEW], Some(7), 0, || TRAINING);
    settle(&mut cpu, RESET_CENTRE);
    cpu.step_with_depth(false, 0.0, &[], |_| panic!("the CPU target counts its own damage"), false, 0);
}

/// The operator, 11-14 ("a, move the notice to the top"): a practice game's notice sits in a top band, so a target may
/// rest in the middle of the screen where the notice used to stand. Zoomed in to 40 the whole on-screen ground was
/// behind the old plates (canvas x within 496 of the centre) and the spawn fell back to an end of the range.
fn eggs_spawn_ignores_the_notice() {
    let view = View([40.0, 40.0], [0.0, -300.0], BOX, None);
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
    let seed = ground_seed(DISC, NORMAL, RESET_CENTRE);
    let mut run = started(FREE, ANY, seed);
    pass(&mut run, &[], false);
    let y = run.egg_at().unwrap()[1];
    assert!((y - (RADIUS + 1.44 - 0.024)).abs() < 1e-4, "spawned on the ground and launched: {y}");
    let mut run = started(FREE, ANY, seed);
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

/// V2.6: the CPU-hide request stays armed through the end notice and the wait for a restart.
fn eggs_running() {
    let mut run = Run::IDLE;
    assert!(!run.running());
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC, IN_VIEW], None, 0, || TRAINING), Plan::Idle);
    assert!(!run.running(), "not before the menu closes");
    let mut run = started(TIMED, ANY, 7);
    assert!(run.running());
    for _ in 0..3700 {
        pass(&mut run, &[], false);
    }
    assert!(run.ended() && run.armed() && !run.running(), "Time! and the wait for a restart");
    run.plan([OFF, ANY, NORMAL, DISC, IN_VIEW], None, 0, || TRAINING);
    assert!(!run.armed(), "Off");
    let mut run = started(FREE, ANY, 7);
    run.plan([FREE, ANY, NORMAL, DISC, IN_VIEW], None, 0, || MATCH);
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
            settle(&mut run, View([10.0, 10.0], [0.0, -300.0], range, None));
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
    assert_eq!(run.plan([mode, rule, NORMAL, CPU, IN_VIEW], Some(seed), 0, || TRAINING), Plan::Read);
    // Zoomed out, so the box has room 30 away from every spot.
    settle(&mut run, View([14.0, 14.0], [0.0, -300.0], BOX, None));
    run
}

/// V2.1 (D1 c), flying since the operator's 11-15 order: with the CPU as the target no disc is drawn; the CPU is placed
/// afresh at 0% on the target's spot and follows its flight to rest on the ground, where it is held; a hit the game
/// registers on it (its damage rises) that the rule lets count breaks the target (the game's own hitlag and knockback
/// play), and after the hitlag the CPU is put at the next spawn spot.
fn eggs_target_cpu() {
    let mut run = started_cpu(FREE, ANY, 7);
    assert_eq!(run.cpu_pass(0.0, false), None, "nothing to place before the first target");
    pass(&mut run, &[], false);
    assert!(target(&mut run).is_none() && run.look(false, || None).iter().all(Option::is_none), "the CPU is the target: no disc");
    let (first, put) = run.cpu_pass(0.0, false).expect("placed");
    assert!(put == Put::Fresh(true) && first[0].abs() <= HALF_WIDTH, "afresh in the air, in the box: {first:?} {put:?}");
    for _ in 0..120 {
        pass(&mut run, &[], false);
        run.cpu_pass(0.0, false);
    }
    let (spot, put) = run.cpu_pass(0.0, false).expect("at rest");
    // rc.15: at rest where it spawned, on the ground or perched above it (held in its fall, `LIFT` up).
    let rest = run.egg.unwrap()[1] + LIFT * f32::from(u8::from(run.perch));
    assert!(put == Put::Hold && (spot[1] - rest).abs() < 1e-3, "at rest where it spawned: {spot:?} {put:?}");
    for _ in 0..30 {
        pass(&mut run, &[], false);
        assert_eq!(run.cpu_pass(0.0, false), Some((spot, Put::Hold)), "held at rest, tick after tick");
    }
    // Still damaged from before the placement took: asked again until it reads 0.
    let mut stale = started_cpu(FREE, ANY, 7);
    pass(&mut stale, &[], false);
    assert_eq!(stale.cpu_pass(40.0, false).map(|p| p.1), Some(Put::Fresh(true)));
    assert_eq!(stale.cpu_pass(40.0, false).map(|p| p.1), Some(Put::Fresh(true)), "a stale 40% is not a hit");
    assert_eq!((stale.score, stale.cpu_pass(0.0, false).map(|p| p.1)), (0, Some(Put::Hold)));
    // A hit on the ground at rest: the count rises, the game plays its hitlag and knockback (nothing asked of the CPU),
    // then the next spot, launched again.
    assert_eq!(run.cpu_pass(9.0, false), None);
    assert_eq!(run.score, 1);
    assert_eq!(notice_after(&mut run), "Eggs 1");
    assert_eq!(run.cpu_pass(9.0, true), None, "in hitlag");
    let (next, put) = run.cpu_pass(9.0, false).expect("the next spawn");
    assert!(put == Put::Fresh(true) && gap(next, spot) >= MIN_GAP - LIFT, "reset at the next spot, flying: {next:?} {put:?}");
    // The rule filters: Aerials counts only in an aerial; Hits of 12%+ only a hit of 12 or more. A hit that does not count
    // resets the CPU where it was.
    let mut aerials = started_cpu(FREE, AERIALS, 7);
    pass(&mut aerials, &[], false);
    let (spot, _) = aerials.cpu_pass(0.0, false).unwrap();
    aerials.cpu_pass(0.0, false);
    assert_eq!(aerials.cpu_pass(5.0, false), None);
    let back = Some((spot, Put::Fresh(true)));
    assert_eq!((aerials.score, aerials.cpu_pass(5.0, false)), (0, back), "a jab does not count: reset in place");
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

/// A run of `mode` under `rule`, `speed`, `target` and `area`, the camera held still at `view`.
fn started_with_area(mode: u8, rule: u8, speed: u8, target: u8, area: u8, seed: u32, view: View) -> Run {
    let mut run = Run::IDLE;
    assert_eq!(run.plan([mode, rule, speed, target, area], Some(seed), 0, || TRAINING), Plan::Read);
    settle(&mut run, view);
    run
}

/// A run of `mode` under `rule` at `speed` with `target`, using the default spawn area.
fn started_with(mode: u8, rule: u8, speed: u8, target: u8, seed: u32, view: View) -> Run {
    started_with_area(mode, rule, speed, target, IN_VIEW, seed, view)
}

/// The operator, 11-15: the CPU target is in the air like the discs. The same seed and camera give the CPU the disc's
/// launch, drift, apex, bounce and rest at every Target speed; its feet trace the disc's bottom edge.
fn eggs_cpu_flies_like_the_disc() {
    let views = [RESET_CENTRE, View([14.0, 14.0], [0.0, -300.0], BOX, None), View([40.0, 40.0], [0.0, -300.0], BOX, None)];
    for (view, speed) in views.into_iter().flat_map(|v| [NORMAL, SLOW, FAST].map(|s| (v, s))) {
        for seed in 0..20 {
            let [mut disc, mut cpu] = [DISC, CPU].map(|t| started_with(FREE, ANY, speed, t, seed, view));
            let mut top = 0.0f32;
            for tick in 0..300 {
                pass(&mut disc, &[], false);
                pass(&mut cpu, &[], false);
                let (d, c) = (disc.egg_at().unwrap(), cpu.egg_at().unwrap());
                let same = (c[0] - d[0]).abs() < 1e-4 && (c[1] + RADIUS - d[1]).abs() < 1e-4;
                assert!(same, "seed {seed} speed {speed} tick {tick}: the CPU at {c:?}, the disc at {d:?}");
                top = top.max(c[1]);
            }
            assert!(top > 10.0, "seed {seed} speed {speed} {view:?}: the CPU's highest point is {top}");
        }
    }
}

/// The CPU target's pass puts it on the moving `egg_at()` every tick: in flight, a point within `LIFT` of the ground is
/// raised to `LIFT` (the game never lands it mid-flight); at rest, exactly the path's end. It is placed afresh in the air
/// once, held without a pose request through the flight, stood up once when the flight comes to rest, then held.
fn eggs_cpu_pass_follows_the_flight() {
    for speed in [NORMAL, SLOW, FAST] {
        let mut run = started_with(FREE, ANY, speed, CPU, ground_seed(CPU, speed, RESET_CENTRE), RESET_CENTRE);
        pass(&mut run, &[], false);
        let ground = run.egg.unwrap()[1];
        let (mut top, mut puts) = (0.0f32, vec![]);
        for tick in 0..400 {
            let want = run.egg_at().unwrap();
            let [v, _, g] = run.fly;
            let aloft = v > 0.0 && (run.age as f32) < 2.0 * v / g * (1.0 + BOUNCE);
            let (at, put) = run.cpu_pass(0.0, false).expect("held every tick");
            let expect = if aloft { [want[0], want[1].max(ground + LIFT)] } else { want };
            assert_eq!(at, expect, "speed {speed} tick {tick}");
            assert!(at[1] >= ground + LIFT || !aloft, "speed {speed} tick {tick}: {at:?} too low in flight");
            top = top.max(at[1] - ground);
            if puts.last().is_none_or(|(p, _)| *p != put) {
                puts.push((put, tick));
            }
            pass(&mut run, &[], false);
        }
        assert!((top - 21.6).abs() < 0.2, "speed {speed}: the flight's apex {top}, the disc's 21.6");
        let kinds: Vec<Put> = puts.iter().map(|p| p.0).collect();
        assert_eq!(kinds, [Put::Fresh(true), Put::Hold, Put::Land, Put::Hold], "speed {speed}: {puts:?}");
        let landed = puts[2].1 as f32 + 1.0;
        let want = 96.0 / SPEEDS[usize::from(speed)];
        assert!((landed - want).abs() <= 1.0, "speed {speed}: at rest after {landed} ticks, want {want}");
        let rest = run.egg_at();
        assert_eq!(run.cpu_pass(0.0, false).map(|p| p.0), rest, "at rest on the ground");
        assert!(rest.is_some_and(|r| (r[1] - ground).abs() < 1e-3), "{rest:?}");
    }
}

/// rc.14 cpu-tag-target: the game turns the CPU's cursor flag (its player tag) off for the CPU the target places in the air
/// (Eden's cursor probe: false through the flight, true again at the landing). While TR owns the placement (every tick the
/// pass returns a position: the flight and the hold at rest) and the body is not hidden, it keeps the flag on. Where the game
/// owns the CPU (the hit, its hitlag, a run that ended: no position) the tag stays the game's business, and a hidden body's
/// off-write wins.
fn eggs_cpu_target_keeps_its_cursor() {
    let mut run = started_with(FREE, ANY, NORMAL, CPU, ground_seed(CPU, NORMAL, RESET_CENTRE), RESET_CENTRE);
    assert!(!keeps_cursor(run.cpu_pass(0.0, false), false), "nothing placed before the first target");
    pass(&mut run, &[], false);
    let mut puts = vec![];
    for tick in 0..130 {
        let moved = run.cpu_pass(0.0, false);
        let put = moved.expect("held every tick").1;
        assert!(keeps_cursor(moved, false), "tick {tick} ({put:?}): the flight and the hold keep the tag on");
        assert!(!keeps_cursor(moved, true), "tick {tick}: a hidden body's off-write wins");
        if puts.last() != Some(&put) {
            puts.push(put);
        }
        pass(&mut run, &[], false);
    }
    assert_eq!(puts, [Put::Fresh(true), Put::Hold, Put::Land, Put::Hold], "the flight, its landing and the rest were all covered");
    // A hit: the game owns the CPU in the hit's pass and in its hitlag: no write. After the hitlag it is placed at the
    // next spot (TR owns it again).
    assert!(!keeps_cursor(run.cpu_pass(9.0, false), false), "the hit's pass");
    assert!(!keeps_cursor(run.cpu_pass(9.0, true), false), "in hitlag");
    assert!(keeps_cursor(run.cpu_pass(9.0, false), false), "placed at the next spot");
    // A run the last hit ended is the game's again.
    let mut ten = started_cpu(TEN, ANY, 7);
    pass(&mut ten, &[], false);
    for _ in 0..10 {
        ten.cpu_pass(0.0, false);
        ten.cpu_pass(0.0, false);
        ten.cpu_pass(5.0, false);
        ten.cpu_pass(5.0, true);
        pass(&mut ten, &[], false);
    }
    assert!(ten.ended());
    assert!(!keeps_cursor(ten.cpu_pass(0.0, false), false), "ended: released to the game");
}

/// Hits in the air: a counted one breaks the target (the game's hitlag plays, nothing is asked), then the CPU goes to 0%
/// at the next spot and flies again; an uncounted one puts it back on its flight where it was hit.
fn eggs_cpu_air_hits() {
    // Placed, then armed (0% read), 20 ticks into the flight.
    let fly = |rule: u8| {
        let mut run = started_with(FREE, rule, NORMAL, CPU, 7, View([14.0, 14.0], [0.0, -300.0], BOX, None));
        pass(&mut run, &[], false);
        let ground = run.egg.unwrap()[1];
        let mut at = [0.0; 2];
        for _ in 0..20 {
            at = run.cpu_pass(0.0, false).expect("held").0;
            pass(&mut run, &[], false);
        }
        assert!(at[1] > ground + 10.0, "mid-flight: {at:?}");
        (run, ground, at)
    };
    let (mut run, ground, at) = fly(ANY);
    assert_eq!(run.cpu_pass(9.0, false), None, "the hit: the game plays it");
    assert_eq!(run.score, 1, "a hit in the air counts");
    for _ in 0..6 {
        pass(&mut run, &[], false);
        assert_eq!(run.cpu_pass(9.0, true), None, "in hitlag");
    }
    let (next, put) = run.cpu_pass(9.0, false).expect("the next spot");
    // rc.15: each spot has its own height (a floor, or perched above one), so the flight is measured from the new spawn.
    let spawn = run.egg.unwrap()[1];
    assert!(next[1] <= spawn + LIFT && gap(next, at) >= 20.0 - RADIUS, "0% at the next spot: {next:?} from {at:?} (first {ground})");
    assert_eq!(put, Put::Fresh(true), "afresh (0%), in the air");
    let mut top = next[1];
    for _ in 0..30 {
        pass(&mut run, &[], false);
        top = top.max(run.cpu_pass(0.0, false).expect("held").0[1]);
    }
    assert!(top > spawn + 15.0, "the next spot flies again: {top} from {spawn}");
    // Aerials, and Player 1 not in an aerial: the hit does not count; after the hitlag the CPU is back where it was hit.
    let (mut run, _, at) = fly(AERIALS);
    let before = run.cpu_pass(0.0, false).expect("held").0;
    assert_eq!(run.cpu_pass(5.0, false), None);
    for _ in 0..6 {
        pass(&mut run, &[], false);
        assert_eq!(run.cpu_pass(5.0, true), None, "in hitlag");
    }
    let back = run.cpu_pass(5.0, false).expect("put back");
    assert_eq!((run.score, back), (0, (before, Put::Fresh(true))), "back on its flight where it was hit, not counted (from {at:?})");
    let mut on = before;
    for _ in 0..5 {
        pass(&mut run, &[], false);
        on = run.cpu_pass(0.0, false).expect("held").0;
    }
    assert!(on[1] > before[1] + 1.0, "the flight goes on from there, still rising: {on:?} from {before:?}");
    // In an aerial, the same hit in the air counts.
    let (mut run, _, _) = fly(AERIALS);
    pass(&mut run, &[], true);
    run.cpu_pass(0.0, false);
    assert_eq!((run.cpu_pass(5.0, false), run.score), (None, 1), "an aerial's hit in the air counts");
}

/// `10 targets` with the CPU as the target, each pass through `plan` as the runtime does it: the CPU's tenth counted hit
/// ends the run, and Player 1's next pass closes it ("Done! 10", the best kept) with the clock stopped at the hit's frame.
/// Eden cf-flight-8551434e-1: the notice stayed at "Eggs 9/10" (`plan` idled the ended run before its closing pass).
fn eggs_cpu_done() {
    let rows = [TEN, ANY, NORMAL, CPU, IN_VIEW];
    let mut run = started_with(TEN, ANY, NORMAL, CPU, 7, View([14.0, 14.0], [0.0, -300.0], BOX, None));
    // Player 1's frame: the plan, then the step when it says Read (the runtime's `eggs_pass`).
    let p1 = |run: &mut Run| match run.plan(rows, None, 0, || TRAINING) {
        Plan::Read => {
            let tick = run.keeper(P1);
            run.step(tick, 0.0, &[], false, 7)
        }
        _ => None,
    };
    // Ten targets of four frames each: placed, armed, hit, hitlag. Player 1's pass comes first in a frame, then the CPU's.
    let mut end = None;
    for k in 0..10 {
        for (damage, stopped) in [(0.0, false), (0.0, false), (5.0, false), (5.0, true)] {
            if p1(&mut run) == Some(END_FRAMES) {
                end = Some(k);
            }
            run.cpu_pass(damage, stopped);
        }
    }
    assert_eq!((run.score, end), (10, Some(9)), "the frame after the tenth hit closes the run");
    assert_eq!(run.frames, 9 * 4 + 3, "the clock stopped at the tenth hit's frame");
    assert!(notice(&run).starts_with("Done! 10 in 0:00.") && notice(&run).contains("(best"), "{}", notice(&run));
    assert_eq!((p1(&mut run), run.cpu_pass(0.0, false)), (None, None), "closed; the CPU is released");
    assert_eq!(run.plan(rows, None, 0, || panic!("closed: nothing is read")), Plan::Idle, "it waits for a restart");
}

/// The notice after one more pass of Player 1.
fn notice_after(run: &mut Run) -> String {
    pass(run, &[], false);
    notice(run)
}
/// A reset spawns its first target after the reset camera's glide. Eden `wa-eggsfly-9c16babe-1`
/// and `-a6266ac1-1` spawned a target (x 32.7, x 70.3) by the panned camera of the walk before the reset, then drew it off
/// screen for 95 of 96 and 92 of 93 ticks.
fn eggs_restart_waits_for_camera() {
    let panned = View([27.09, 27.09], [-900.0, -264.4], BOX, None);
    for seed in 0..40 {
        let mut run = started(FREE, ANY, seed);
        settle(&mut run, panned);
        run.reset(11);
        // The notice resets immediately; the target still waits for the new camera.
        assert_eq!(pass(&mut run, &[], false), Some(u16::MAX), "seed {seed}: no target before the draw published the new camera");
        assert!(target(&mut run).is_none());
        // The camera still shows the panned view for a draw, then glides to the default over a few more.
        for k in 0..7u8 {
            let at = View([27.09; 2], [-900.0 + 1308.7 * f32::from(k) / 7.0, -264.4], BOX, None);
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
    // A replacement uses the current moving view as soon as the target breaks.
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
    assert_eq!(choices(&snapshot), [OFF, ANY, NORMAL, DISC, IN_VIEW]);
    snapshot.set(ROW_TARGET, &[0, 1]).unwrap();
    assert_eq!(choices(&snapshot)[3], CPU);
    snapshot.set(ROW_HIDE_CPU, &[1, 0]).unwrap();
    assert_eq!(choices(&snapshot)[3], DISC, "Hide CPU wins");
    snapshot.set(ROW_HIDE_CPU, &[0, 1]).unwrap();
    assert_eq!(choices(&snapshot)[3], CPU, "Off: the CPU is the target again");
    assert_eq!(choice(&snapshot, ROW_PRACTICE_CPU), 0, "CPU during a game: Hide first");
}

/// V2.6: the practice game's request to hide the CPU (`hides_cpu`), one table: CPU during a game says Hide and a game runs,
/// except that the CPU is never hidden while it is the Eggs-ercise target (the target wins; its body, its tag and its panel
/// all stay); Break the Targets hides it whatever the Eggs rows say; Show never hides it.
fn eggs_practice_hide_request() {
    // (CPU during a game is Hide, an Eggs run is on, the CPU is its target, Break the Targets runs) -> the request
    let table = [
        ((true, true, false, false), true, "Eggs-ercise, Target Disc, Hide"),
        ((true, true, true, false), false, "Eggs-ercise, Target CPU, Hide: the target wins"),
        ((true, false, false, true), true, "Break the Targets, Hide"),
        ((true, false, false, false), false, "no game"),
        ((false, true, false, false), false, "Eggs-ercise, Target Disc, Show"),
        ((false, true, true, false), false, "Eggs-ercise, Target CPU, Show"),
        ((false, false, false, true), false, "Break the Targets, Show"),
        ((false, false, false, false), false, "no game, Show"),
    ];
    for ((practice_hide, running, cpu_target, btt), want, what) in table {
        assert_eq!(hides_cpu(practice_hide, running, cpu_target, btt), want, "{what}");
    }
}

/// V2.1: a break freezes the breaker for `HITLAG` ticks wherever the practice is live: native Training and an active
/// waiting-room practice (rc.14, 11-15: the LDN pass showed it local to the console). Not in a match, not while a fighter
/// swap owns the scene, and not on a lobby flag left over from an earlier room.
fn eggs_hitlag() {
    assert_eq!(HITLAG, 6);
    assert_eq!([hitlag(TRAINING), hitlag(LOBBY)], [Some(6); 2], "Training and the waiting room's practice");
    let stale = Facts { waiting_room: false, ..LOBBY };
    let busy = Facts { busy: true, ..LOBBY };
    assert_eq!([hitlag(MATCH), hitlag(stale), hitlag(busy)], [None; 3], "outside practice");
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
    assert_eq!(run.plan([TEN, ANY, NORMAL, DISC, IN_VIEW], None, 0, || TRAINING), Plan::Read, "the animation runs on");
    for _ in 0..BURST_TICKS {
        assert_eq!(pass(&mut run, &[], false), None, "the end notice stays");
    }
    assert!(own(&mut run).is_empty());
    assert_eq!(run.plan([TEN, ANY, NORMAL, DISC, IN_VIEW], None, 0, || panic!("ended, no animation: nothing is read")), Plan::Idle);
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
    assert_eq!(run.plan([TIMED, AERIALS, NORMAL, DISC, IN_VIEW], None, 0, || TRAINING), Plan::Idle, "it waits for a restart");
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
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC, IN_VIEW], None, 1, || panic!("a CPU's pass reads nothing")), Plan::Idle);
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
    assert_eq!(run.plan([FREE, ANY, NORMAL, DISC, IN_VIEW], None, 0, || TRAINING), Plan::Read);
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
    assert_eq!(idle.plan([TIMED, ANY, NORMAL, DISC, IN_VIEW], None, 0, || TRAINING), Plan::Idle, "a reset starts nothing before the menu closed");
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
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC, IN_VIEW], None, 0, || SWAPPING), Plan::Idle, "a hot swap idles the run");
    let swapped = 0x3000;
    let ticked = (0..61).filter(|_| run.keeper(swapped)).count();
    assert_eq!(ticked, 59, "the new fighter takes the clock after two passes");
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC, IN_VIEW], None, 0, || TRAINING), Plan::Read, "the fighter cycle continues the run");
    restart(&mut run, 3);
    assert_eq!(pass(&mut run, &[], false), Some(u16::MAX));
    assert_eq!(notice(&run), "Eggs 0   1:00", "the reset hub restarts it");
}

fn eggs_nana() {
    let mut run = started(TIMED, AERIALS, 7);
    for _ in 0..60 {
        for fighter in [P1, NANA] {
            assert_eq!(run.plan([TIMED, AERIALS, NORMAL, DISC, IN_VIEW], None, 0, || TRAINING), Plan::Read, "both are entry 0");
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
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC, IN_VIEW], None, 0, || LOBBY), Plan::Read, "the waiting room's practice");
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC, IN_VIEW], None, 0, || MATCH), Plan::Clear, "a match: the run stops, its notice goes");
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC, IN_VIEW], None, 0, || panic!("stopped: nothing is read")), Plan::Idle);
    assert!(target(&mut run).is_none());
}

fn eggs_off_no_calls() {
    let mut run = Run::IDLE;
    assert_eq!(run.plan([OFF, ANY, NORMAL, DISC, IN_VIEW], Some(1), 0, || panic!("Off reads nothing")), Plan::Idle);
    assert!(run.look(true, || panic!("Off: no projection"))[0].is_none());
    assert_eq!(run.plan([TIMED, ANY, NORMAL, DISC, IN_VIEW], None, 0, || panic!("no run before the menu closes")), Plan::Idle);
    let mut run = started(TIMED, ANY, 7);
    pass(&mut run, &[], false);
    assert_eq!(run.plan([OFF, ANY, NORMAL, DISC, IN_VIEW], None, 0, || panic!("Off reads nothing")), Plan::Clear, "the notice goes the next pass");
    assert!(target(&mut run).is_none(), "and the egg");
    assert_eq!(run.plan([OFF, ANY, NORMAL, DISC, IN_VIEW], None, 0, || panic!("Off reads nothing")), Plan::Idle);
}

fn breaks_iff_depth() {
    fn old_breaks(rule: u8, aerial: bool, hit: &Hit, egg: [f32; 2]) -> bool {
        let Hit(a, b, radius, power) = *hit;
        let counts = counts(rule, aerial, power);
        let (dx, dy, px, py) = (b[0] - a[0], b[1] - a[1], egg[0] - a[0], egg[1] - a[1]);
        let t = ((px * dx + py * dy) / (dx * dx + dy * dy).max(1e-9)).clamp(0.0, 1.0);
        counts && (px - t * dx).hypot(py - t * dy) <= radius + RADIUS
    }

    let egg = [0.0, 0.0];
    let cases = [
        ("point at centre", Hit([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.0, 16.0)),
        ("point halfway", Hit([2.0, 0.0, 0.0], [2.0, 0.0, 0.0], 0.0, 16.0)),
        ("point tangent", Hit([4.0, 0.0, 0.0], [4.0, 0.0, 0.0], 0.0, 16.0)),
        ("point outside", Hit([4.01, 0.0, 0.0], [4.01, 0.0, 0.0], 0.0, 16.0)),
        ("exact tangent", Hit([5.0, 0.0, 0.0], [5.0, 0.0, 0.0], 1.0, 16.0)),
        ("just outside", Hit([5.01, 0.0, 0.0], [5.01, 0.0, 0.0], 1.0, 16.0)),
        ("pill tangent", Hit([5.0, -2.0, 0.0], [5.0, 2.0, 0.0], 1.0, 16.0)),
        ("zero-length pill", Hit([4.5, 0.0, 0.0], [4.5, 0.0, 0.0], 0.5, 16.0)),
        ("pill covers centre", Hit([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.1, 16.0)),
        ("interior nearest point", Hit([-6.0, 3.0, 0.0], [6.0, 3.0, 0.0], 1.0, 16.0)),
        ("endpoint nearest point", Hit([4.0, 0.0, 0.0], [6.0, 0.0, 0.0], 0.5, 16.0)),
        ("pill outside", Hit([5.1, 0.0, 0.0], [7.0, 0.0, 0.0], 1.0, 16.0)),
        ("under strong threshold", Hit([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.0, 11.9)),
    ];

    for rule in [ANY, AERIALS, STRONG] {
        for aerial in [false, true] {
            for (label, hit) in &cases {
                let actual = breaks(rule, aerial, hit, egg);
                let expected = counts(rule, aerial, hit.3) && depth(hit, egg).is_some();
                assert_eq!(actual, expected, "{label}, rule={rule}, aerial={aerial}");
                assert_eq!(actual, old_breaks(rule, aerial, hit, egg), "{label}, rule={rule}, aerial={aerial}");
            }
        }
    }

    let hits = [
        Hit([3.0, 0.0, 0.0], [3.0, 0.0, 0.0], 0.0, 16.0),
        Hit([2.0, 0.0, 0.0], [2.0, 0.0, 0.0], 0.0, 16.0),
        Hit([6.0, 0.0, 0.0], [6.0, 0.0, 0.0], 0.0, 16.0),
        Hit([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.0, 11.9),
    ];
    assert_eq!(deepest(STRONG, false, &hits, egg), Some(0.5));
    assert_eq!(deepest(AERIALS, false, &hits, egg), None);
    assert_eq!(deepest(AERIALS, true, &hits, egg), Some(1.0));
}

fn tangency_breaks_excellent() {
    let egg = [0.0, 0.0];
    let hit = Hit([5.0, 0.0, 0.0], [5.0, 0.0, 0.0], 1.0, 16.0);
    assert!(breaks(ANY, false, &hit, egg));
    assert_eq!(depth(&hit, egg), Some(0.0));
    assert_eq!(depth(&hit, egg).map(crate::model::spacing::Grade::of), Some(crate::model::spacing::Grade::Excellent));
}

fn outside_disc_miss_has_no_grade() {
    let egg = [0.0, 0.0];
    let hit = Hit([5.001, 0.0, 0.0], [5.001, 0.0, 0.0], 1.0, 16.0);
    assert!(!breaks(ANY, false, &hit, egg));
    assert_eq!(depth(&hit, egg), None);
}

fn helper_only_break_has_no_grade() {
    let mut run = started(TIMED, ANY, 7);
    let (event, depth) = run.step_with_depth(true, 0.0, &[], |_| other_break(true, true, |_| None), false, 0);
    assert!(event.is_some());
    assert_eq!(depth, None);
}

fn eggs_projectile_with_helper_break_is_graded() {
    let mut run = started(TIMED, ANY, 7);
    let (event, depth) = run.step_with_depth(
        true,
        0.0,
        &[],
        |_| other_break(true, true, |deepest| { assert!(deepest); Some(0.75) }),
        false,
        0,
    );
    assert!(event.is_some());
    assert_eq!(depth, Some(0.75));
}

fn eggs_projectile_miss_has_no_break() {
    let mut run = started(TIMED, ANY, 7);
    let (_, depth) = run.step_with_depth(true, 0.0, &[], |_| other_break(true, false, |_| None), false, 0);
    assert_eq!(run.score, 0);
    assert_eq!(depth, None);
}

fn grader_off_helper_break_skips_article_search() {
    let mut run = started(TIMED, ANY, 7);
    let (event, depth) = run.step_with_depth(
        true,
        0.0,
        &[],
        |_| other_break(false, true, |_| panic!("grader Off skips article search for a helper break")),
        false,
        0,
    );
    assert!(event.is_some());
    assert_eq!((run.score, depth), (1, None));
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
        ("eggs_spawn_not_alternating", eggs_spawn_not_alternating as fn()),
        ("eggs_spawn_covers_the_stage", eggs_spawn_covers_the_stage as fn()),
        ("whole_stage_places_targets_outside_the_view", whole_stage_places_targets_outside_the_view as fn()),
        ("in_view_spawns_match_5b6ab8cc", in_view_spawns_match_5b6ab8cc as fn()),
        ("eggs_spawn_skips_pits", eggs_spawn_skips_pits as fn()),
        ("eggs_spawn_in_view_heights", eggs_spawn_in_view_heights as fn()),
        ("eggs_first_target_waits_for_floors", eggs_first_target_waits_for_floors as fn()),
        ("eggs_articles_break", eggs_articles_break as fn()),
        ("eggs_spawn_ignores_the_notice", eggs_spawn_ignores_the_notice as fn()),
        ("eggs_motion_normal", eggs_motion_normal as fn()),
        ("eggs_speed_scales_time", eggs_speed_scales_time as fn()),
        ("eggs_rest_and_pause", eggs_rest_and_pause as fn()),
        ("eggs_hitlag", eggs_hitlag as fn()),
        ("eggs_running", eggs_running as fn()),
        ("eggs_restart_waits_for_camera", eggs_restart_waits_for_camera as fn()),
        ("eggs_stage_range", eggs_stage_range as fn()),
        ("eggs_target_cpu", eggs_target_cpu as fn()),
        ("eggs_target_cpu_hidden", eggs_target_cpu_hidden as fn()),
        ("eggs_practice_hide_request", eggs_practice_hide_request as fn()),
        ("eggs_cpu_target_keeps_its_cursor", eggs_cpu_target_keeps_its_cursor as fn()),
        ("eggs_cpu_flies_like_the_disc", eggs_cpu_flies_like_the_disc as fn()),
        ("eggs_cpu_pass_follows_the_flight", eggs_cpu_pass_follows_the_flight as fn()),
        ("eggs_cpu_air_hits", eggs_cpu_air_hits as fn()),
        ("eggs_cpu_done", eggs_cpu_done as fn()),
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
        ("breaks_iff_depth", breaks_iff_depth as fn()),
        ("tangency_breaks_excellent", tangency_breaks_excellent as fn()),
        ("outside_disc_miss_has_no_grade", outside_disc_miss_has_no_grade as fn()),
        ("helper_only_break_has_no_grade", helper_only_break_has_no_grade as fn()),
        ("eggs_projectile_with_helper_break_is_graded", eggs_projectile_with_helper_break_is_graded as fn()),
        ("eggs_projectile_miss_has_no_break", eggs_projectile_miss_has_no_break as fn()),
        ("grader_off_helper_break_skips_article_search", grader_off_helper_break_skips_article_search as fn()),
    ]);
}
}
