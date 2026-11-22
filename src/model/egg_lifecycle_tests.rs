use super::*;

const VIEW: View = View([14.0; 2], [0.0, -250.0], [-100.0, 100.0], Some([-60.0, 90.0]));

fn target(seconds: u8) -> Run {
    let mut run = Run::IDLE;
    run.start([FREE, ANY, NORMAL, DISC, IN_VIEW], 7);
    run.configure(0, seconds);
    for _ in 0..=SETTLE {
        run.look(true, || Some(VIEW));
    }
    run.step(false, 0.0, &[], false, 0);
    assert!(run.egg_at().is_some());
    run
}

fn contact(run: &mut Run) {
    let [x, y] = run.egg_at().expect("live target");
    let point = [x, y, 0.0];
    run.step(true, 0.0, &[Hit(point, point, 2.0, 15.0)], true, 0);
}

#[test]
fn expiry_uses_active_ticks_and_rolls_a_new_target() {
    let mut run = target(1);
    let original = run.target_id();
    for _ in 0..59 {
        run.step(true, 0.0, &[], false, 0);
    }
    for _ in 0..120 {
        run.step(false, 0.0, &[], false, 0);
    }
    assert_eq!(run.target_id(), original);
    assert_eq!(run.event(), TargetEvent::None);
    run.step(true, 0.0, &[], false, 0);
    assert_eq!((run.event(), run.score), (TargetEvent::Miss, 0));
    assert_ne!(run.target_id(), original);
    assert!(run.egg_at().is_some());
}

#[test]
fn deadline_contact_wins_and_off_never_expires() {
    let mut run = target(1);
    for _ in 0..59 {
        run.step(true, 0.0, &[], false, 0);
    }
    contact(&mut run);
    assert_eq!((run.event(), run.score), (TargetEvent::Hit, 1));
    let mut untimed = target(0);
    let id = untimed.target_id();
    for _ in 0..601 {
        untimed.step(true, 0.0, &[], false, 0);
    }
    assert_eq!(untimed.target_id(), id);
}

#[test]
fn offstage_contact_scores_immediately_from_stage() {
    for (where_p1, observation) in [
        ("grounded at the lip", Recovery::Returned),
        ("grounded on a platform", Recovery::Unchanged),
        ("airborne inboard above the floor", Recovery::Unchanged),
        ("caught the ledge", Recovery::Returned),
    ] {
        let mut run = target(0);
        run.area = Some(OFF_STAGE);
        let original = run.target_id();
        let broken = run.egg_at().unwrap();
        run.observe_recovery(observation);
        contact(&mut run);
        assert_eq!((run.event(), run.score, run.recovering()), (TargetEvent::Hit, 1, false), "{where_p1}");
        assert_eq!(run.bursts[0], Some((broken, 0)));
        assert_ne!(run.target_id(), original, "{where_p1}");
        assert!(run.egg_at().is_some(), "{where_p1}");
        let mut hud = String::new();
        run.write_notice(&mut hud).unwrap();
        assert_eq!(hud, "Eggs 1", "{where_p1}");
    }
}

#[test]
fn offstage_contact_waits_for_an_excursion_then_a_return() {
    let mut run = target(1);
    run.area = Some(OFF_STAGE);
    run.observe_recovery(Recovery::Away);
    contact(&mut run);
    assert_eq!((run.event(), run.score), (TargetEvent::Pending, 0));
    assert!(run.egg_at().is_none());
    let mut hud = String::new();
    run.write_notice(&mut hud).unwrap();
    assert!(hud.contains("Recover!"));
    for _ in 0..61 {
        run.step(true, 0.0, &[], false, 0);
    }
    assert_eq!((run.event(), run.score), (TargetEvent::None, 0));
    run.observe_recovery(Recovery::Away);
    run.step(true, 0.0, &[], false, 0);
    run.observe_recovery(Recovery::Unchanged);
    run.step(true, 0.0, &[], false, 0);
    assert_eq!(run.score, 0, "a platform or ordinary airborne frame is not a return");
    run.observe_recovery(Recovery::Returned);
    run.step(true, 0.0, &[], false, 0);
    assert_eq!((run.event(), run.score), (TargetEvent::Recovered, 1));
}

#[test]
fn offstage_contact_is_cancelled_by_ko_or_bottom_blast() {
    let mut run = target(1);
    run.area = Some(OFF_STAGE);
    run.observe_recovery(Recovery::Away);
    contact(&mut run);
    run.observe_recovery(Recovery::Lost);
    run.step(true, 0.0, &[], false, 0);
    assert_eq!((run.event(), run.score), (TargetEvent::Miss, 0));
    run.observe_recovery(Recovery::Returned);
    run.step(true, 0.0, &[], false, 0);
    assert_eq!(run.score, 0);
    assert!(run.egg_at().is_some());
}

#[test]
fn seeded_selection_reaches_every_enabled_area_evenly() {
    let mut run = target(0);
    for mask in 0b001u8..=0b111 {
        let mut counts = [0u32; 3];
        for _ in 0..12_000 {
            counts[usize::from(run.pick(mask))] += 1;
        }
        let expected = 12_000 / mask.count_ones();
        for (bit, count) in counts.into_iter().enumerate() {
            if mask & (1 << bit) == 0 {
                assert_eq!(count, 0);
            } else {
                assert!(count.abs_diff(expected) < 300, "{mask:03b}: {counts:?}");
            }
        }
    }
}

fn spawn_with_paths(paths: u8, seed: u32, blast: Option<Rect>) -> Run {
    let mut run = Run::IDLE;
    run.start([FREE, ANY, NORMAL, DISC, IN_VIEW], seed);
    run.configure(paths, 0);
    if let Some(blast) = blast {
        run.set_blast(blast);
    }
    for _ in 0..=SETTLE {
        run.look(true, || Some(VIEW));
    }
    run.step(false, 0.0, &[], false, 0);
    assert!(run.egg_at().is_some());
    run
}

#[test]
fn egg_paths_draw_includes_pop_up_alongside_axe() {
    let blast = Some([[-145.0, -100.0], [140.0, 125.0]]);
    let mask = 0b1_0001;
    let axe = spawn_with_paths(mask, 7, blast);
    let popup = spawn_with_paths(mask, 1200, blast);
    assert!(matches!(axe.path, Some(Path::Axe)));
    assert!(popup.path.is_none(), "the fifth checked cell should select Pop up");
}

#[test]
fn all_five_egg_paths_are_reached_evenly_on_spawn() {
    let mut counts = [0u32; 5];
    const SAMPLES: u32 = 5_000;
    let blast = Some([[-145.0, -100.0], [140.0, 125.0]]);
    for index in 0..SAMPLES {
        let seed = index.wrapping_mul(0x9e37_79b9);
        let run = spawn_with_paths(0b1_1111, seed, blast);
        let chosen = match run.path {
            Some(Path::Axe) => 0,
            Some(Path::Galaxian) => 1,
            Some(Path::Boomerang) => 2,
            Some(Path::Missile) => 3,
            None => 4,
        };
        counts[chosen] += 1;
    }
    assert!(counts.iter().all(|&count| count.abs_diff(SAMPLES / 5) <= 100), "{counts:?}");
}

#[test]
fn egg_paths_pop_up_only_matches_empty_flight_and_geometry_queries() {
    let [mut empty, mut popup] = [0, 0b1_0000].map(|paths| spawn_with_paths(paths, 7, None));
    for run in [&empty, &popup] {
        assert!(!run.uses_paths());
        assert!(!run.wants_blast());
        assert!(!run.wants_zone());
    }
    assert_eq!(empty.egg_at(), popup.egg_at());
    for _ in 0..150 {
        for run in [&mut empty, &mut popup] {
            run.step(true, 0.0, &[], false, 0);
        }
        assert_eq!(empty.egg_at(), popup.egg_at());
    }
}

fn cpu_at_deadline() -> Run {
    let mut run = Run::IDLE;
    run.start([FREE, ANY, NORMAL, CPU, IN_VIEW], 19);
    run.configure(0, 1);
    for _ in 0..=SETTLE {
        run.look(true, || Some(VIEW));
    }
    run.step(false, 0.0, &[], false, 0);
    run.cpu_pass(0.0, false);
    run.cpu_pass(0.0, false);
    for _ in 0..60 {
        run.step(true, 0.0, &[], false, 0);
    }
    run
}

#[test]
fn cpu_native_contact_wins_the_deadline_and_can_be_deferred() {
    let mut run = cpu_at_deadline();
    run.area = Some(OFF_STAGE);
    run.observe_recovery(Recovery::Away);
    assert_eq!(run.cpu_pass(10.0, true), None);
    assert_eq!((run.event(), run.score, run.egg_at()), (TargetEvent::Pending, 0, None));
    run.observe_recovery(Recovery::Returned);
    run.step(true, 0.0, &[], false, 0);
    assert_eq!((run.event(), run.score), (TargetEvent::Recovered, 1));
    assert!(matches!(run.cpu_pass(10.0, false), Some((_, Put::Fresh(_)))));
}

#[test]
fn cpu_offstage_contact_also_scores_immediately() {
    let mut run = cpu_at_deadline();
    let original = run.target_id();
    run.area = Some(OFF_STAGE);
    run.observe_recovery(Recovery::Unchanged);
    assert_eq!(run.cpu_pass(10.0, true), None);
    assert_eq!((run.event(), run.score, run.recovering()), (TargetEvent::Hit, 1, false));
    assert_eq!(run.cpu_pass(10.0, true), None, "the struck CPU can finish hitlag");
    assert_eq!(run.target_id(), original);
    assert!(matches!(run.cpu_pass(10.0, false), Some((_, Put::Fresh(_)))));
    assert_ne!(run.target_id(), original);
}

#[test]
fn cpu_target_expires_unhit_and_starts_fresh() {
    let mut run = cpu_at_deadline();
    let prior = run.target_id();
    assert!(matches!(run.cpu_pass(0.0, false), Some((_, Put::Fresh(_)))));
    assert_eq!((run.event(), run.score), (TargetEvent::Miss, 0));
    assert_ne!(run.target_id(), prior);
}

#[test]
fn time_cancels_unresolved_hit_and_never_accepts_post_round_recovery() {
    let mut run = target(0);
    run.rows[0] = TIMED;
    run.frames = 2;
    run.area = Some(OFF_STAGE);
    run.observe_recovery(Recovery::Away);
    contact(&mut run);
    assert_eq!(run.event(), TargetEvent::Pending);
    run.step(true, 0.0, &[], false, 0);
    assert_eq!((run.event(), run.score, run.ended()), (TargetEvent::Miss, 0, true));
    run.observe_recovery(Recovery::Returned);
    run.step(true, 0.0, &[], false, 0);
    assert_eq!(run.score, 0);
}

#[test]
fn zone_assignment_latches_preference_for_the_run_and_reads_new_stage_on_reset() {
    let mut run = target(0);
    assert_eq!(run.zone_preferences(1, || Some([1; 4])), Some([1; 4]));
    assert_eq!(run.zone_preferences(1, || panic!("already latched")), Some([1; 4]));
    run.reset(8);
    assert_eq!(run.zone_preferences(2, || Some([2; 4])), Some([2; 4]));
}

#[test]
fn unavailable_offstage_falls_back_and_preserves_the_next_runs_choices() {
    for (areas, fallback, blast_available, notice) in [
        (1 << OFF_STAGE, IN_VIEW, true, "No usable Off stage zone. Using In view."),
        (1 << OFF_STAGE, IN_VIEW, false, "No usable Off stage zone. Using In view."),
        ((1 << OFF_STAGE) | (1 << WHOLE), WHOLE, true, "No usable Off stage zone. Using other checked areas."),
    ] {
        let mut run = Run::IDLE;
        run.start([FREE, ANY, NORMAL, DISC, OFF_STAGE], 7);
        run.configure(0b1111, 0);
        run.set_areas(areas);
        if blast_available {
            run.set_blast([[-145.0, -100.0], [140.0, 125.0]]);
        }
        for _ in 0..=SETTLE {
            run.look(true, || Some(VIEW));
        }
        // Simulate an Off stage selection waiting for native geometry.
        run.area = Some(OFF_STAGE);
        run.path = Some(Path::Axe);
        run.step(true, 0.0, &[], false, 0);
        assert!(run.egg_at().is_none());
        assert_eq!(run.disable_offstage(), Some(notice));
        assert_eq!(run.disable_offstage(), None);
        assert!(!run.wants_zone());
        run.step(true, 0.0, &[], false, 0);
        assert_eq!(run.area, Some(fallback));
        assert_eq!(run.motion.is_some(), blast_available);
        assert!(run.look(false, || None)[0].is_some());
        run.reset(8);
        assert!(run.wants_zone());
        assert_eq!(run.areas, areas);
    }
}

#[test]
fn unavailable_offstage_uses_the_normal_in_view_floor_scan() {
    let mut run = Run::IDLE;
    run.start([FREE, ANY, NORMAL, DISC, OFF_STAGE], 7);
    for _ in 0..=SETTLE {
        run.look(true, || Some(VIEW));
    }
    assert!(!run.floors_wanted());
    assert!(run.disable_offstage().is_some());
    assert!(run.floors_wanted());
    run.want_floors();
    run.step(true, 0.0, &[], false, 0);
    assert!(run.egg_at().is_none());
    run.set_floors(&[[[-35.0, 12.0], [35.0, 16.0]]]);
    run.step(true, 0.0, &[], false, 0);
    let [x, y] = run.egg_at().expect("target above the scanned floor");
    assert!((-35.0..=35.0).contains(&x));
    assert!(y >= 12.0 + RADIUS);
    assert!(!run.floors_wanted());
}

#[test]
fn training_offstage_draws_and_counts_each_path_on_the_real_recovery_side() {
    use crate::model::edge_juggle::perfect::Outline;
    use crate::model::egg_zone::Stage;

    let corners = [[-80.0, 0.0], [620.0, 0.0], [620.0, -30.0], [-80.0, -30.0]];
    let mut outline = Outline::new();
    for index in 0..corners.len() {
        outline.add([corners[index], corners[(index + 1) % corners.len()]]);
    }
    let stage = Stage::new(308, [-589.9998, 500.0, 360.00015, -137.85207],
                           [corners[0], corners[1]], outline).unwrap();
    let zone = stage.resolve(None);
    let clear = |zone: &Zone, side, from, to| stage.allows_move(zone, side, from, to);
    for path in [1, 2, 4, 8, 16, 31] {
        let mut run = Run::IDLE;
        run.start([FREE, ANY, NORMAL, DISC, OFF_STAGE], 7);
        run.configure(path, 0);
        run.set_zone(zone);
        let step = |run: &mut Run| run.pass(Pass { tick: true, y: 0.0, hits: &[], air: false, kind: 0 },
                                           |_| None, &clear);
        step(&mut run);
        let target = run.egg_at().expect("the usable left side spawns a target");
        assert_eq!(run.motion.unwrap().side, Some(Side::Left));
        assert!(stage.contains(&zone, Side::Left, target));
        assert_eq!(run.look(false, || None)[0].map(|(at, _)| at), Some(target));
        run.observe_recovery(Recovery::Away);
        contact(&mut run);
        assert_eq!((run.event(), run.score), (TargetEvent::Pending, 0));
        run.observe_recovery(Recovery::Returned);
        step(&mut run);
        assert_eq!((run.event(), run.score), (TargetEvent::Recovered, 1));
    }
}

#[test]
fn offstage_spawns_without_view_and_sweeps_motion_in_its_original_side() {
    use crate::model::edge_juggle::perfect::Outline;
    use crate::model::egg_zone::Stage;

    let hull = [[-50.0, 0.0], [50.0, 0.0], [34.0, -25.0], [-34.0, -25.0]];
    let mut outline = Outline::new();
    for index in 0..hull.len() {
        outline.add([hull[index], hull[(index + 1) % hull.len()]]);
    }
    let stage = Stage::new(0, [-145.0, 140.0, 125.0, -100.0], [hull[0], hull[1]], outline).unwrap();
    let zone = stage.resolve(None);
    let clearance = |zone: &Zone, side, from, to| stage.allows_move(zone, side, from, to);

    for path in [0, 1, 2, 4, 8] {
        let mut run = Run::IDLE;
        run.start([FREE, ANY, NORMAL, DISC, OFF_STAGE], u32::from(path) + 7);
        run.configure(path, 0);
        run.set_zone(zone);
        let step = |run: &mut Run| run.pass(Pass { tick: true, y: 0.0, hits: &[], air: false, kind: 0 },
                                           |_| None, &clearance);
        step(&mut run);
        let side = run.motion.unwrap().side.unwrap();
        let first = run.egg_at().expect("world area does not require a view");
        let mut last = first;
        let mut moved = false;
        for t in 0..150 {
            run.sense([78.0, -50.0 + 4.0 * t as f32]);
            step(&mut run);
            let next = run.egg_at().unwrap();
            assert!(clearance(&zone, side, last, next), "{path} at tick {t}");
            assert_eq!(run.look(false, || None)[0].map(|(at, _)| at), Some(next));
            moved |= first != next;
            last = next;
        }
        assert!(moved, "selected flight {path} should move");
    }
}

fn keeps_over_its_floor(floors: &[Floor; 2], own: fn([f32; 2]) -> usize, [area, target, bit]: [u8; 3], seed: u32) -> usize {
    let mut run = Run::IDLE;
    run.start([FREE, ANY, NORMAL, target, area], seed.wrapping_mul(0x9e37_79b9));
    run.configure(1 << bit, 0);
    run.set_blast([[-145.0, -100.0], [140.0, 125.0]]);
    run.set_floors(floors);
    run.sense([58.0, -30.0]);
    for _ in 0..=SETTLE {
        run.look(true, || Some(VIEW));
    }
    run.step(false, 0.0, &[], false, 0);
    let sampled = run.egg.expect("a target");
    let index = own(sampled);
    let [[west, floor], [east, _]] = floors[index];
    let case = format!(
        "area {area} target {target} path {bit} seed {seed}, sampled at {sampled:?}, its floor from {west} to {east} at {floor}"
    );
    assert!((west + DRIFT..=east - DRIFT).contains(&sampled[0]), "sampled off its floor's spawn span: {case}");
    let first = run.egg_at().unwrap();
    let mut moved = false;
    for tick in 0..200 {
        let [x, y] = run.egg_at().unwrap();
        assert!((west..=east).contains(&x) && y >= floor + RADIUS, "left its floor: tick {tick}: ({x}, {y}); {case}");
        moved |= [x, y] != first;
        run.step(true, 0.0, &[], false, 0);
    }
    assert!(moved, "should fly: {case}");
    let [low, high] = run.motion.expect("a moving path").bounds;
    assert_eq!((low, high[0]), ([west + RADIUS, floor + RADIUS], east - RADIUS), "{case}");
    assert!((high[1] - [52.4286, 86.0][usize::from(area)]).abs() < 0.001, "{high:?}");
    index
}

fn every_path_keeps_over_its_floor(floors: &[Floor; 2], own: fn([f32; 2]) -> usize) {
    let mut seen = [0usize; 2];
    for area in [IN_VIEW, WHOLE] {
        for target in [DISC, CPU] {
            for bit in 0..4u8 {
                for seed in 0..32u32 {
                    seen[keeps_over_its_floor(floors, own, [area, target, bit], seed)] += 1;
                }
            }
        }
    }
    assert!(seen.iter().all(|n| *n > 0), "targets appeared over both floors: {seen:?}");
}

#[test]
fn moving_paths_stay_over_the_stage() {
    every_path_keeps_over_its_floor(&[[[-50.0, 0.0], [50.0, 0.0]], [[-40.0, 30.0], [-10.0, 30.0]]], |at| {
        usize::from(at[1] >= 30.0)
    });
}

#[test]
fn moving_paths_keep_to_their_own_floor_across_a_gap() {
    every_path_keeps_over_its_floor(&[[[-50.0, 0.0], [-28.0, 0.0]], [[25.0, 0.0], [50.0, 0.0]]], |at| {
        usize::from(at[0] > 0.0)
    });
}

#[test]
fn a_boomerang_rests_over_the_floor_it_appeared_over() {
    let mut run = Run::IDLE;
    run.start([FREE, ANY, NORMAL, DISC, IN_VIEW], 0);
    run.configure(1 << 2, 0);
    run.set_blast([[-145.0, -100.0], [140.0, 125.0]]);
    run.set_floors(&[[[-50.0, 0.0], [-28.0, 0.0]], [[25.0, 0.0], [50.0, 0.0]]]);
    run.sense([58.0, -30.0]);
    for _ in 0..=SETTLE {
        run.look(true, || Some(VIEW));
    }
    run.step(false, 0.0, &[], false, 0);
    let sampled = run.egg.expect("a target");
    assert!((-40.0..=-38.0).contains(&sampled[0]), "seed 0 samples a spot on the left floor: {sampled:?}");
    let start = run.egg_at().unwrap();
    let mut reach = [start[0]; 2];
    for _ in 0..200 {
        run.step(true, 0.0, &[], false, 0);
        let x = run.egg_at().unwrap()[0];
        reach = [reach[0].min(x), reach[1].max(x)];
    }
    let rest = run.egg_at().unwrap();
    assert!(
        (-50.0..=-28.0).contains(&rest[0]),
        "the Boomerang started at {start:?} and rests at {rest:?}, over no floor (floors from -50 to -28 and from 25 to 50)"
    );
    assert!(reach[1] - reach[0] > 10.0, "it flies out and back: from {} to {}", reach[0], reach[1]);
}

#[test]
fn a_moving_path_with_no_room_over_its_floor_pops_up() {
    let mut run = Run::IDLE;
    run.start([FREE, ANY, NORMAL, DISC, IN_VIEW], 7);
    run.configure(0b1111, 0);
    run.set_blast([[-145.0, -100.0], [140.0, 125.0]]);
    let narrow = View(VIEW.0, VIEW.1, [-12.0, 12.0], VIEW.3);
    for _ in 0..=SETTLE {
        run.look(true, || Some(narrow));
    }
    run.step(false, 0.0, &[], false, 0);
    let [x, y] = run.egg_at().expect("a target in the four-unit range");
    assert!((-2.0..=2.0).contains(&x));
    assert!(run.motion.is_none() && run.path.is_none(), "no room for a flight: Pop up");
    for _ in 0..30 {
        run.step(true, 0.0, &[], false, 0);
    }
    assert!(run.egg_at().unwrap()[1] > y + 20.0, "the Pop up flight is at its apex after 30 ticks");
}

#[test]
fn a_moving_path_with_exactly_no_room_pops_up() {
    let mut run = Run::IDLE;
    run.start([FREE, ANY, NORMAL, DISC, IN_VIEW], 7);
    run.configure(1, 0);
    run.set_blast([[-145.0, -100.0], [140.0, 125.0]]);
    let narrow = View(VIEW.0, VIEW.1, [-14.0, 14.0], VIEW.3);
    for _ in 0..=SETTLE {
        run.look(true, || Some(narrow));
    }
    run.step(false, 0.0, &[], false, 0);
    let [x, y] = run.egg_at().expect("a target in the eight-unit range");
    assert!((-4.0..=4.0).contains(&x));
    assert!(run.motion.is_none() && run.path.is_none(), "a box of no width: Pop up, not an Axe held at one point");
    for _ in 0..30 {
        run.step(true, 0.0, &[], false, 0);
    }
    assert!(run.egg_at().unwrap()[1] > y + 20.0, "the Pop up flight is at its apex after 30 ticks");
}

#[test]
fn a_moving_path_needs_room_only_on_the_axes_it_uses() {
    for (bit, flies) in [(2u8, true), (0, false)] {
        let mut run = Run::IDLE;
        run.start([FREE, ANY, NORMAL, DISC, WHOLE], 7);
        run.configure(1 << bit, 0);
        run.set_blast([[-145.0, -100.0], [140.0, 125.0]]);
        run.set_floors(&[[[-50.0, 82.0], [50.0, 82.0]]]);
        run.sense([58.0, -30.0]);
        for _ in 0..=SETTLE {
            run.look(true, || Some(VIEW));
        }
        run.step(false, 0.0, &[], false, 0);
        let first = run.egg_at().expect("a target on the floor at the top of the box");
        assert_eq!(first[1], 86.0, "no room above the target");
        assert_eq!((run.motion.is_some(), run.path.is_some()), (flies, flies), "path {bit} with room sideways and none above");
        let mut reach = [first[0]; 2];
        for _ in 0..200 {
            run.step(true, 0.0, &[], false, 0);
            let [x, y] = run.egg_at().unwrap();
            assert_eq!(y, 86.0);
            reach = [reach[0].min(x), reach[1].max(x)];
        }
        assert_eq!(reach[1] - reach[0] > 60.0, flies, "path {bit} went from {} to {}", reach[0], reach[1]);
    }
}

#[test]
fn a_pop_up_fallback_takes_the_same_random_draws_as_a_flight() {
    let narrow = |half: f32| {
        let mut run = Run::IDLE;
        run.start([FREE, ANY, NORMAL, DISC, IN_VIEW], 7);
        run.configure(1, 0);
        run.set_blast([[-145.0, -100.0], [140.0, 125.0]]);
        let view = View(VIEW.0, VIEW.1, [-half, half], VIEW.3);
        for _ in 0..=SETTLE {
            run.look(true, || Some(view));
        }
        run.step(false, 0.0, &[], false, 0);
        run
    };
    let (mut fallback, mut flight) = (narrow(12.0), narrow(20.0));
    for target in 0..2 {
        assert!(fallback.egg_at().is_some() && flight.egg_at().is_some(), "target {target}");
        assert_eq!((fallback.motion.is_some(), flight.motion.is_some()), (false, true), "target {target}");
        assert_eq!(fallback.rng, flight.rng, "target {target}: the Pop up fallback and the flight took different random draws");
        contact(&mut fallback);
        contact(&mut flight);
    }
}

fn first_target(area: u8, bit: u8, seed: u32, view: View, floors: &[Floor]) -> Run {
    let mut run = Run::IDLE;
    run.start([FREE, ANY, NORMAL, DISC, area], seed);
    run.configure(1 << bit, 0);
    run.set_blast([[-145.0, -100.0], [140.0, 125.0]]);
    if !floors.is_empty() {
        run.set_floors(floors);
        run.sense([58.0, -30.0]);
    }
    for _ in 0..=SETTLE {
        run.look(true, || Some(view));
    }
    run.step(false, 0.0, &[], false, 0);
    run
}

#[test]
fn an_axe_with_a_hair_of_height_pops_up() {
    let floor = f32::from_bits(0x42a3_ffff);
    let run = first_target(WHOLE, 0, 7, VIEW, &[[[32.0, floor], [64.0, floor]]]);
    assert!(
        run.motion.is_none() && run.path.is_none(),
        "an Axe in a box {} high would be scaled to nothing and stay at one point: Pop up",
        86.0 - (floor + RADIUS)
    );
    let [x, y] = run.egg_at().expect("a target on the floor a hair under the top of the box");
    assert!((42.0..=54.0).contains(&x) && y == floor + RADIUS, "({x}, {y})");
}

#[test]
fn a_path_that_needs_height_appears_over_a_floor_that_has_it() {
    for (area, high) in [(IN_VIEW, 42.0), (WHOLE, 82.0), (WHOLE, f32::from_bits(0x42a3_ffff))] {
        let floors = [[[-50.0, high], [50.0, high]], [[-50.0, 0.0], [50.0, 0.0]]];
        let mut seen = [0usize; 2];
        for seed in 0..32u32 {
            let level = first_target(area, 2, seed.wrapping_mul(0x9e37_79b9), VIEW, &floors);
            let main_spot = level.egg.expect("a Boomerang target");
            assert!(level.motion.is_some(), "a Boomerang needs no height: area {area} floor at {high} seed {seed}");
            seen[usize::from(main_spot[1] > 35.0)] += 1;
            for bit in [0u8, 1, 3] {
                let mut run = first_target(area, bit, seed.wrapping_mul(0x9e37_79b9), VIEW, &floors);
                let spot = run.egg.expect("a target");
                let case = format!(
                    "area {area} floor at {high} path {bit} seed {seed}: sampled at {spot:?}, the Boomerang at {main_spot:?}"
                );
                assert!(run.motion.is_some() && spot[1] < 35.0, "should fly over the floor with room: {case}");
                if main_spot[1] < 35.0 {
                    assert_eq!(spot, main_spot, "a spot with room keeps its place: {case}");
                }
                if bit != 3 {
                    assert_eq!(run.rng, level.rng, "choosing among the sampled spots takes no random draw: {case}");
                }
                let first = run.egg_at().unwrap();
                let mut moved = false;
                for tick in 0..200 {
                    let [x, y] = run.egg_at().unwrap();
                    assert!((-46.0..=46.0).contains(&x) && y >= RADIUS, "left its floor: tick {tick}: ({x}, {y}); {case}");
                    moved |= [x, y] != first;
                    run.step(true, 0.0, &[], false, 0);
                }
                assert!(moved, "should move: {case}");
            }
        }
        assert!(seen.iter().all(|n| *n > 0), "the Boomerang's targets appeared over both floors: {seen:?}");
    }
}

fn flies_or_pops_up(mut run: Run, flies: bool, room: Option<usize>, case: &str) {
    let first = run.egg_at().expect("a target");
    assert_eq!((run.motion.is_some(), run.path.is_some()), (flies, flies), "{case}");
    if let (Some(motion), Some(axis)) = (run.motion, room) {
        assert_eq!(motion.bounds[1][axis] - motion.bounds[0][axis], 2.0 * RADIUS, "{case}");
    }
    let mut moved = false;
    for _ in 0..200 {
        run.step(true, 0.0, &[], false, 0);
        moved |= run.egg_at().unwrap() != first;
    }
    assert!(moved, "should move: {case}");
}

#[test]
fn a_moving_path_needs_a_target_width_of_room() {
    for bit in 0..4u8 {
        for (half, flies) in [(18.0f32, true), (17.5, false)] {
            let run = first_target(IN_VIEW, bit, 7, View(VIEW.0, VIEW.1, [-half, half], VIEW.3), &[]);
            let case = format!("path {bit} with {} units of room sideways", 2.0 * (half - DRIFT - RADIUS));
            flies_or_pops_up(run, flies, Some(0), &case);
        }
        for (floor, flies, room) in [(74.0f32, true, Some(1)), (74.5, bit == 2, None)] {
            let run = first_target(WHOLE, bit, 7, VIEW, &[[[-50.0, floor], [50.0, floor]]]);
            let case = format!("path {bit} with {} units of room above", 86.0 - floor - RADIUS);
            flies_or_pops_up(run, flies, room, &case);
        }
    }
}

#[test]
fn a_spot_with_room_among_the_thirty_two_wins_and_pop_up_is_for_none() {
    const WITH_ROOM: [u32; 18] = [1, 3, 5, 6, 8, 9, 10, 12, 15, 16, 17, 18, 20, 24, 27, 29, 30, 31];
    let floors = [[[-50.0, 82.0], [50.0, 82.0]], [[60.0, 0.0], [82.0, 0.0]]];
    for seed in 0..32u32 {
        let run = first_target(WHOLE, 0, seed.wrapping_mul(0x9e37_79b9), VIEW, &floors);
        let spot = run.egg.expect("a target");
        let flies = WITH_ROOM.contains(&seed);
        assert_eq!(
            (run.motion.is_some(), run.path.is_some(), spot[0] > 60.0),
            (flies, flies, flies),
            "seed {seed} sampled at {spot:?}: an Axe over the short floor when one of its 32 spots is there, else Pop up"
        );
    }
}

#[test]
fn a_flight_starts_inside_its_box() {
    let floors = [[[-50.0, 42.0], [50.0, 42.0]], [[-50.0, 0.0], [50.0, 0.0]]];
    for area in [IN_VIEW, WHOLE] {
        for bit in 0..4u8 {
            for seed in 0..32u32 {
                let run = first_target(area, bit, seed.wrapping_mul(0x9e37_79b9), VIEW, &floors);
                let (at, [low, high]) = (run.egg_at().expect("a target"), run.motion.expect("a moving path").bounds);
                assert!(
                    (0..2).all(|i| (low[i]..=high[i]).contains(&at[i])),
                    "area {area} path {bit} seed {seed}: starts at {at:?}, outside its box from {low:?} to {high:?}"
                );
            }
        }
    }
}
