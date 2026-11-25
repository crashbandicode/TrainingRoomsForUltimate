//! Guard the Edge and Juggler on the off-stage rule (`stage_geometry::rule`): the stage is the floor run Player 1
//! stands on, a start that cannot happen has a reason, and a CPU start is taken by the place the rule gives it.
//! The stage lines are the Training stage's and Battlefield's, as the rule's own tests hold them.
lib_unit_tests! {
use super::*;
use crate::model::stage_geometry::{classify, Copied, Met, Place, Point, Probe, Unread};

const TRAINING_BLAST: [f32; 4] = [-590.0, 500.0, 360.0, -137.852];
const TRAINING_CAMERA: [f32; 4] = [-510.0, 420.0, 280.0, -39.864];
const BATTLEFIELD_BLAST: [f32; 4] = [-240.0, 240.0, 192.0, -140.0];
const BATTLEFIELD_CAMERA: [f32; 4] = [-170.0, 170.0, 130.0, -80.0];
const ON: StartOptions = StartOptions { under: true, jumps_used: false };
const OFF: StartOptions = StartOptions { under: false, jumps_used: false };

fn line(ends: [Point; 2], stand: bool, through: bool) -> Met {
    Met { ends, stand, through }
}

/// The left block (its top and its low floor), the island with its platforms, and the long floor.
fn training() -> Vec<Met> {
    vec![
        line([[-635., 47.391], [-490., 47.391]], true, false),
        line([[-490., 0.], [-420., 0.]], true, false),
        line([[-635., -165.], [-420., -165.]], false, false),
        line([[-330., 0.014], [-170., 0.014]], true, false),
        line([[-330., 0.014], [-250., -40.]], false, false),
        line([[-250., -40.], [-170., 0.014]], false, false),
        line([[-309.4, 24.119], [-274.8, 24.119]], true, true),
        line([[-267.3, 47.189], [-232.7, 47.189]], true, true),
        line([[-225.2, 24.119], [-190.6, 24.119]], true, true),
        line([[-80., 0.], [620., 0.]], true, false),
        line([[-80., -340.], [620., -340.]], false, false),
    ]
}

fn battlefield() -> Vec<Met> {
    vec![
        line([[-79.986, 0.], [79.989, 0.]], true, false),
        line([[-79.986, 0.], [0., -40.1]], false, false),
        line([[0., -40.1], [79.989, 0.]], false, false),
        line([[-57.6, 27.2], [-20., 27.2]], true, true),
        line([[20., 27.2], [57.6, 27.2]], true, true),
        line([[-18.8, 54.4], [18.8, 54.4]], true, true),
    ]
}

/// A stage that cannot be read.
struct Gone;
impl Probe for Gone {
    fn first(&mut self, _: Point, _: f32, _: bool) -> Result<Option<Met>, Unread> {
        Err(Unread)
    }
}

fn near(a: Option<Point>, b: Point) -> bool {
    a.is_some_and(|a| (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01)
}

fn on_training(drill: Drill, p1: Point) -> Result<(Stage, Point), StartRefusal> {
    start_stage(&mut Copied(&training()), drill, TRAINING_BLAST, TRAINING_CAMERA, p1)
}

#[test]
fn the_stage_is_the_floor_run_the_player_stands_on() {
    // Player 1 at the Training stage's start: the long floor. Its left end is a ledge; its right end is past the
    // blast line.
    let (long, start) = on_training(Drill::GuardEdge, [-30.0, 0.01]).unwrap();
    assert_eq!(long.ledges, [[-80.0, 0.0], [620.0, 0.0]]);
    assert!(near(long.spot(0), [-125.0, -68.426]), "{:?}", long.spot(0));
    assert_eq!(long.spot(1), None);
    assert_eq!(start, [-45.0, 0.0], "the camera's middle is on this run");
    // Player 1 on the island, also on one of its platforms: both ends are ledges with open air.
    for p1 in [[-250.0, 0.024], [-250.0, 47.2]] {
        let (island, start) = on_training(Drill::GuardEdge, p1).unwrap();
        assert_eq!(island.ledges, [[-330.0, 0.014], [-170.0, 0.014]]);
        assert!(near(island.spot(0), [-375.0, -68.419]), "{:?}", island.spot(0));
        assert!(near(island.spot(1), [-125.0, -68.419]), "{:?}", island.spot(1));
        assert_eq!(start, [-250.0, 0.014], "the run's own middle");
    }
    // A stage of one solid is the stage as it was read before.
    let (found, start) = start_stage(&mut Copied(&battlefield()), Drill::GuardEdge, BATTLEFIELD_BLAST, BATTLEFIELD_CAMERA, [40.0, 27.21]).unwrap();
    let before = Stage::new(BATTLEFIELD_BLAST, BATTLEFIELD_CAMERA, [[-79.986, 0.0], [79.989, 0.0]]).unwrap();
    assert_eq!(found.ledges, before.ledges);
    for side in 0..2 {
        assert!(near(found.spot(side), before.spot(side).unwrap()), "{side}");
    }
    assert_eq!(start, [0.0, 0.0]);
    assert!(long.on_run(-80.0) && long.on_run(499.0) && !long.on_run(-80.5));
}

#[test]
fn on_the_training_stage_guard_the_edge_starts_from_the_long_floor_and_the_island() {
    let key = (308, 0);
    // The long floor: the left side only.
    let (long, p1) = on_training(Drill::GuardEdge, [-30.0, 0.01]).unwrap();
    let mut wizard = Wizard::new(Drill::GuardEdge, long, p1, 100, key, None).unwrap();
    assert!(near(Some(wizard.setup.cpu), [-125.0, -68.426]), "{:?}", wizard.setup.cpu);
    let left = wizard.setup.cpu;
    wizard.set_side_default(long, true);
    assert_eq!(wizard.setup.cpu, left, "there is no other side");
    wizard.setup.cpu = [400.0, 50.0];
    wizard.set_side_default(long, false);
    assert_eq!(wizard.setup.cpu, left, "the default of the one side there is");
    // The island: both sides.
    let (island, p1) = on_training(Drill::GuardEdge, [-250.0, 0.024]).unwrap();
    let mut wizard = Wizard::new(Drill::GuardEdge, island, p1, 100, key, None).unwrap();
    assert!(near(Some(wizard.setup.cpu), [-375.0, -68.419]));
    wizard.set_side_default(island, true);
    assert!(near(Some(wizard.setup.cpu), [-125.0, -68.419]));
    wizard.set_side_default(island, true);
    assert!(near(Some(wizard.setup.cpu), [-375.0, -68.419]));
    // The left block's low floor: its right end only (its left end is against the wall).
    let (block, p1) = on_training(Drill::GuardEdge, [-450.0, 0.01]).unwrap();
    assert_eq!((block.ledges, block.spot(0)), ([[-490.0, 0.0], [-420.0, 0.0]], None));
    let wizard = Wizard::new(Drill::GuardEdge, block, p1, 100, key, None).unwrap();
    assert!(near(Some(wizard.setup.cpu), [-375.0, -68.426]), "{:?}", wizard.setup.cpu);
    // Juggler starts on each of them, with the CPU over Player 1.
    for at in [[-30.0, 0.01], [-250.0, 0.024], [-450.0, 0.01]] {
        let (stage, p1) = on_training(Drill::Juggler, at).unwrap();
        let wizard = Wizard::new(Drill::Juggler, stage, p1, 60, key, None).unwrap();
        assert_eq!(classify(&mut Copied(&training()), TRAINING_BLAST, wizard.setup.cpu), Ok(Place::OverStage), "{at:?}");
    }
}

#[test]
fn a_start_that_cannot_happen_names_its_reason() {
    for drill in [Drill::GuardEdge, Drill::Juggler] {
        // Over a gap there is no floor to start from.
        assert_eq!(on_training(drill, [-375.0, 20.0]), Err(StartRefusal::NoFloor), "{drill:?}");
        assert_eq!(start_stage(&mut Gone, drill, TRAINING_BLAST, TRAINING_CAMERA, [-30.0, 0.01]), Err(StartRefusal::Unread), "{drill:?}");
    }
    // The left block's top: one end is past the blast line, the other steps down onto a floor. No ledge to guard;
    // Juggler needs none.
    assert_eq!(on_training(Drill::GuardEdge, [-520.0, 47.4]), Err(StartRefusal::NoLedge));
    let (top, _) = on_training(Drill::Juggler, [-520.0, 47.4]).unwrap();
    assert_eq!((top.spot(0), top.spot(1)), (None, None));
    // Each reason has its own approved text, shown where the start is refused.
    let runtime = include_str!("../../runtime/features/edge_juggle.rs");
    let frame = runtime.split_once("pub unsafe fn frame(").unwrap().1;
    for refusal in [
        "StartRefusal::Unread => notices::refuse(\"The stage could not be read\"), // restriction: edge-juggle-stage-unread",
        "StartRefusal::NoFloor => notices::refuse(\"Stand on the stage to start this drill\"), // restriction: edge-juggle-start-no-floor",
        "StartRefusal::NoLedge => notices::refuse(\"This floor has no ledge to guard\"), // restriction: edge-juggle-no-open-ledge",
    ] {
        assert!(frame.contains(refusal), "{refusal}");
    }
    let restrictions = include_str!("../../../docs/RESTRICTIONS.md");
    for id in ["edge-juggle-stage-unread", "edge-juggle-start-no-floor", "edge-juggle-no-open-ledge", "edge-juggle-juggler-start"] {
        assert!(restrictions.contains(&format!("| `{id}` |")), "{id}");
    }
}

#[test]
fn guard_the_edge_takes_open_air_and_under_only_with_the_option() {
    let lines = training();
    let (stage, p1) = on_training(Drill::GuardEdge, [-250.0, 0.024]).unwrap();
    let wizard = Wizard::new(Drill::GuardEdge, stage, p1, 100, (308, 0), None).unwrap();
    for (point, place, off, on) in [
        ([-375.0, -50.0], Place::Open, true, true),
        ([-125.0, 30.0], Place::Open, true, true),
        ([-250.0, -80.0], Place::Under, false, true),
        ([-250.0, -10.0], Place::Solid, false, false),
        ([100.0, 30.0], Place::OverStage, false, false),
        ([-250.0, 60.0], Place::OverStage, false, false),
        ([0.0, 400.0], Place::Outside, false, false),
    ] {
        assert_eq!(classify(&mut Copied(&lines), TRAINING_BLAST, point), Ok(place), "{point:?}");
        assert_eq!((wizard.takes(place, OFF), wizard.takes(place, ON)), (off, on), "{point:?}");
    }
    // A taken start goes on to the next page; a refused one, or an unread stage, stays on this one.
    let at = wizard.setup.cpu;
    for (place, options, taken) in [
        (Ok(Place::Open), OFF, true), (Ok(Place::Under), ON, true), (Ok(Place::Under), OFF, false),
        (Ok(Place::Solid), ON, false), (Err(Unread), ON, false),
    ] {
        let mut wizard = wizard;
        assert_eq!(wizard.confirm_start(at, place, options), Some(taken), "{place:?}");
        assert_eq!(wizard.step, if taken { WizardStep::Player } else { WizardStep::Cpu }, "{place:?}");
    }
}

#[test]
fn juggler_takes_a_spot_over_the_stage_or_in_open_air() {
    let (stage, p1) = on_training(Drill::Juggler, [-30.0, 0.01]).unwrap();
    let wizard = Wizard::new(Drill::Juggler, stage, p1, 60, (308, 0), None).unwrap();
    for options in [OFF, ON] {
        for (place, taken) in [
            (Place::OverStage, true), (Place::Open, true), (Place::Under, false), (Place::Solid, false), (Place::Outside, false),
        ] {
            assert_eq!(wizard.takes(place, options), taken, "{place:?}");
        }
    }
    let mut refused = wizard;
    assert_eq!(refused.confirm_start(wizard.setup.cpu, Ok(Place::Under), ON), Some(false));
    assert_eq!(refused.step, WizardStep::Cpu);
    let mut taken = wizard;
    assert_eq!(taken.confirm_start(wizard.setup.cpu, Ok(Place::OverStage), OFF), Some(true));
    assert_eq!(taken.step, WizardStep::Percent);
    let runtime = include_str!("../../runtime/features/edge_juggle.rs");
    assert!(runtime.contains("Drill::Juggler => notices::refuse(\"Pick a point over the stage or beside it\"), // restriction: edge-juggle-juggler-start"));
    assert!(runtime.contains("Drill::GuardEdge => notices::refuse(\"Pick a point off the stage\"), // restriction: edge-juggle-start-over-stage"));
}

#[test]
fn a_moved_cursor_does_not_use_an_earlier_press() {
    use crate::model::btt::Pad;
    let (stage, p1) = on_training(Drill::GuardEdge, [-250.0, 0.024]).unwrap();
    let mut wizard = Wizard::new(Drill::GuardEdge, stage, p1, 100, (308, 0), None).unwrap();
    let pressed = wizard.setup.cpu;
    wizard.steer(stage, &Pad { stick: [1.0, 0.0], ..Pad::default() });
    assert_ne!(wizard.setup.cpu, pressed);
    assert_eq!(wizard.confirm_start(pressed, Ok(Place::Open), OFF), None);
    assert_eq!(wizard.step, WizardStep::Cpu);
    // The press on the point the cursor is on now is the one that counts.
    let now = wizard.setup.cpu;
    assert_eq!(wizard.confirm_start(now, Ok(Place::Open), OFF), Some(true));
    assert_eq!(wizard.step, WizardStep::Player);
    // On another page the press is not a CPU start at all.
    assert_eq!(wizard.confirm_start(now, Ok(Place::Open), OFF), None);
    assert_eq!(wizard.step, WizardStep::Player);
    // The poll only records the press; the frame classes that point with the rule.
    let runtime = include_str!("../../runtime/features/edge_juggle.rs");
    let poll = runtime.split_once("pub unsafe fn poll(").unwrap().1.split_once("\n}").unwrap().0;
    assert!(poll.contains("run.confirm_cpu = Some(wizard.setup.cpu);"));
    assert!(!poll.contains("notices::refuse("));
    assert_eq!(runtime.matches("wizard.confirm_start(at, place, run.options)").count(), 1);
    assert_eq!(runtime.matches("geometry::classify(&mut stage, run.stage.blast(), at)").count(), 1);
}

#[test]
fn a_kept_start_is_reused_only_on_its_own_floor_run() {
    let key = (308, 0);
    for drill in [Drill::Juggler, Drill::GuardEdge] {
        let (long, long_p1) = on_training(drill, [-30.0, 0.01]).unwrap();
        let (island, island_p1) = on_training(drill, [-250.0, 0.024]).unwrap();
        let on_long = Wizard::new(drill, long, long_p1, 100, key, None).unwrap();
        let on_island = Wizard::new(drill, island, island_p1, 100, key, None).unwrap();
        // A start set up on each run, with the CPU moved off its default: beside the run's left ledge for Guard
        // the Edge, over Player 1 for Juggler.
        let (long_cpu, island_cpu) = match drill {
            Drill::GuardEdge => ([-125.0, -60.0], [-375.0, -60.0]),
            Drill::Juggler => ([0.0, 80.0], [-250.0, 80.0]),
        };
        let made_on_long = Setup { cpu: long_cpu, p1: [0.0, 0.0], percent: 100 };
        let made_on_island = Setup { cpu: island_cpu, p1: [-250.0, 0.014], percent: 100 };
        let kept = |run: Stage, setup| Kept { run: run.ledges, setup, unmoved: false, under: false };
        // The same run reuses it.
        assert_eq!(on_long.kept_setup(long, kept(long, made_on_long), OFF), Some(made_on_long), "{drill:?}");
        assert_eq!(on_island.kept_setup(island, kept(island, made_on_island), OFF), Some(made_on_island), "{drill:?}");
        // The other run does not: Start then opens the wizard, whose setup is that run's own default.
        assert_eq!(on_long.kept_setup(long, kept(island, made_on_island), OFF), None, "{drill:?}: kept on the island");
        assert_eq!(on_island.kept_setup(island, kept(long, made_on_long), OFF), None, "{drill:?}: kept on the long floor");
        assert!(long.on_run(on_long.setup.p1[0]) && island.on_run(on_island.setup.p1[0]), "{drill:?}");
        match drill {
            Drill::GuardEdge => {
                assert!(near(Some(on_long.setup.cpu), [-125.0, -68.426]));
                assert!(near(Some(on_island.setup.cpu), [-375.0, -68.419]));
            }
            Drill::Juggler => {
                assert_eq!(on_long.setup.cpu, long.default_juggle(long_p1));
                assert_eq!(on_island.setup.cpu, island.default_juggle(island_p1));
            }
        }
    }
    // The run's ends are kept with the setup and compared at Start; nothing is read again for it.
    let runtime = include_str!("../../runtime/features/edge_juggle.rs");
    assert!(runtime.contains("kept[run.drill as usize] = Some((run.stage_key, wizard.kept(run.stage)));"));
    assert_eq!(runtime.matches("wizard.kept_setup(stage, kept, config.options)").count(), 1);
}

/// tr-primary-2 2026-11-25 (rv-preview-vis 18:44:40Z): a kept start remembers that A confirmed it under the stage.
/// On the long floor the island's underside is beyond the run's left end, so the point alone reads as beside the run.
#[test]
fn a_kept_start_under_the_stage_is_reused_only_while_the_option_is_on() {
    let lines = training();
    let (long, p1) = on_training(Drill::GuardEdge, [-30.0, 0.01]).unwrap();
    let fresh = Wizard::new(Drill::GuardEdge, long, p1, 100, (308, 0), None).unwrap();
    let default = fresh.setup;
    // Under the island, from the long floor, with Under the stage On.
    let at = [-250.0, -80.0];
    let place = classify(&mut Copied(&lines), TRAINING_BLAST, at);
    assert_eq!(place, Ok(Place::Under));
    assert!(long.off_stage(at).is_some(), "beside the run by its ends alone");
    // The cursor was moved there: an unmoved start is the default, which is never kept as a point.
    let moved = |to: [f32; 2]| {
        let mut wizard = fresh;
        wizard.steer(long, &crate::model::btt::Pad { stick: [1.0, 0.0], ..Default::default() });
        wizard.setup.cpu = to;
        wizard
    };
    let mut wizard = moved(at);
    assert_eq!(wizard.confirm_start(at, place, ON), Some(true));
    let kept = wizard.kept(long);
    assert!(kept.under);
    assert_eq!((kept.run, kept.setup.cpu), (long.ledges, at));
    // Kept, then Start with the option still On: reused. With it Off: not reused, so Start uses the run's default.
    assert_eq!(fresh.kept_setup(long, kept, ON).map(|setup| setup.cpu), Some(at));
    assert_eq!(fresh.kept_setup(long, kept, OFF), None);
    assert!(near(Some(default.cpu), [-125.0, -68.426]));
    // A start confirmed in open air does not need the option, and a later confirm replaces the class.
    let open = [-125.0, -60.0];
    let mut wizard = moved(open);
    assert_eq!(wizard.confirm_start(open, Ok(Place::Open), OFF), Some(true));
    assert!(!wizard.kept(long).under);
    assert_eq!(fresh.kept_setup(long, wizard.kept(long), OFF).map(|setup| setup.cpu), Some(open));
    // The defaults are never under the stage.
    let mut reset = moved(at);
    assert_eq!(reset.confirm_start(at, place, ON), Some(true));
    reset.back_to_defaults(default);
    assert!(!reset.kept(long).under);
}

/// tr-primary-2 2026-11-25 (rv-preview-vis 19:21:39Z): the place A confirmed decides the reuse, not the older
/// one-unit inset from the run's ends. Half a unit outside the long floor's left ledge is open air by the rule.
#[test]
fn a_kept_start_confirmed_in_open_air_is_reused_whatever_its_inset() {
    let lines = training();
    let (long, p1) = on_training(Drill::GuardEdge, [-30.0, 0.01]).unwrap();
    let fresh = Wizard::new(Drill::GuardEdge, long, p1, 100, (308, 0), None).unwrap();
    let at = [-80.5, -60.0];
    let place = classify(&mut Copied(&lines), TRAINING_BLAST, at);
    assert_eq!(place, Ok(Place::Open));
    assert_eq!(long.off_stage(at), None, "inside the older inset");
    let mut wizard = fresh;
    wizard.steer(long, &crate::model::btt::Pad { stick: [1.0, 0.0], ..Default::default() });
    wizard.setup.cpu = at;
    assert_eq!(wizard.confirm_start(at, place, OFF), Some(true));
    let kept = wizard.kept(long);
    assert!(!kept.under && !kept.unmoved);
    for options in [OFF, ON] {
        assert_eq!(fresh.kept_setup(long, kept, options).map(|setup| setup.cpu), Some(at), "{options:?}");
    }
    // Still only on its own run.
    let (island, island_p1) = on_training(Drill::GuardEdge, [-250.0, 0.024]).unwrap();
    let on_island = Wizard::new(Drill::GuardEdge, island, island_p1, 100, (308, 0), None).unwrap();
    assert_eq!(on_island.kept_setup(island, kept, OFF), None);
}

/// tr-primary-2 2026-11-25 19:25Z: a fighter's own default start is used only where the rule calls its point open
/// air; the older inset from the run's ends does not decide it.
#[test]
fn a_fighters_own_start_is_used_only_where_the_rule_calls_it_open_air() {
    let lines = training();
    let (island, p1) = on_training(Drill::GuardEdge, [-250.0, 0.024]).unwrap();
    let wizard = |open: &mut dyn FnMut(Point) -> bool| {
        Wizard::on(Drill::GuardEdge, island, p1, 100, (3, 0), Some(perfect::diddy::KIND), open).unwrap()
    };
    // Diddy's own start, 14 out from the ledge and 30 down, on each side of the island.
    let own = [[-344.0, -29.986], [-156.0, -29.986]];
    let spots = [island.spot(0).unwrap(), island.spot(1).unwrap()];
    // The rule on the Training stage's lines calls both open air: the fighter's own start is the default.
    let mut asked = Vec::new();
    let by_rule = wizard(&mut |point| {
        asked.push(point);
        classify(&mut Copied(&lines), TRAINING_BLAST, point) == Ok(Place::Open)
    });
    for (side, &own) in own.iter().enumerate() {
        assert!(near(by_rule.default_cpu(island, side), own), "{side}: {:?}", by_rule.default_cpu(island, side));
        assert!(asked.iter().any(|&point| near(Some(point), own)), "{side}: the rule was asked about the point");
    }
    assert!(near(Some(by_rule.setup.cpu), own[0]));
    // Where the rule does not call the point open air, the default is the rule's spot, whatever the inset says.
    assert!(island.off_stage(own[0]).is_some() && island.off_stage(own[1]).is_some(), "the older test takes both");
    let refused = wizard(&mut |_| false);
    for (side, &spot) in spots.iter().enumerate() {
        assert_eq!(refused.default_cpu(island, side), Some(spot), "{side}");
    }
    assert_eq!(refused.setup.cpu, spots[0]);
    // The drill asks the rule on its frame.
    let runtime = include_str!("../../runtime/features/edge_juggle.rs");
    assert!(runtime.contains("geometry::classify(&mut probe, stage.blast(), point) == Ok(geometry::Place::Open)"));
    assert!(!runtime.contains("Wizard::new("));
}
}
