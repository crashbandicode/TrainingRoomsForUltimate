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

#[test]
fn moving_paths_stay_over_the_stage() {
    const MAIN: Floor = [[-50.0, 0.0], [50.0, 0.0]];
    const PLATFORM: Floor = [[-40.0, 30.0], [-10.0, 30.0]];
    for area in [IN_VIEW, WHOLE] {
        for target in [DISC, CPU] {
            for bit in 0..4u8 {
                for seed in 0..24u32 {
                    let mut run = Run::IDLE;
                    run.start([FREE, ANY, NORMAL, target, area], seed.wrapping_mul(0x9e37_79b9));
                    run.configure(1 << bit, 0);
                    run.set_blast([[-145.0, -100.0], [140.0, 125.0]]);
                    run.set_floors(&[MAIN, PLATFORM]);
                    run.sense([58.0, -30.0]);
                    for _ in 0..=SETTLE {
                        run.look(true, || Some(VIEW));
                    }
                    run.step(false, 0.0, &[], false, 0);
                    let floor = if run.egg.expect("a target")[1] >= 30.0 { 30.0 } else { 0.0 };
                    let first = run.egg_at().unwrap();
                    let mut moved = false;
                    for tick in 0..200 {
                        let [x, y] = run.egg_at().unwrap();
                        assert!(
                            (-46.01..=46.01).contains(&x) && y >= floor + RADIUS - 0.01,
                            "left the stage: area {area} target {target} path {bit} seed {seed} tick {tick}: \
                             ({x}, {y}), floor from -50 to 50 at {floor}"
                        );
                        moved |= [x, y] != first;
                        run.step(true, 0.0, &[], false, 0);
                    }
                    assert!(moved, "area {area} target {target} path {bit} seed {seed} should fly");
                    let [low, high] = run.motion.expect("a moving path").bounds;
                    assert_eq!((low, high[0]), ([-46.0, floor + RADIUS], 46.0));
                    assert!((high[1] - [52.4286, 86.0][usize::from(area)]).abs() < 0.001, "{high:?}");
                }
            }
        }
    }
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
