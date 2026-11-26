lib_unit_tests! {
use super::*;
use crate::model::btt::Pad;
use crate::model::chords::logical::{A, B, DPAD_LEFT, DPAD_RIGHT, L, PLUS, R, Y};
use crate::model::stage_geometry::{self, Copied, Met, Probe, Unread};
use std::cell::Cell;

const TRAINING: [f32; 4] = [-590.0, 500.0, 360.0, -137.852];
const BATTLEFIELD: [f32; 4] = [-240.0, 240.0, 192.0, -140.0];

fn line(ends: [[f32; 2]; 2], stand: bool, through: bool) -> Met {
    Met { ends, stand, through }
}

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

fn fighter(place: Place, at: [f32; 2]) -> Fighter {
    Fighter { place, at, state: StartState::Standard, percent: 0 }
}

fn setup(at: [f32; 2]) -> Setup {
    [fighter(Place::OnStage, at), fighter(Place::OnStage, [0., 0.])]
}

fn button(key: u8) -> Pad {
    let bit = 1u16 << key;
    Pad { held: bit, pressed: bit, ..Pad::default() }
}

fn rule<'a>(lines: &'a [Met], blast: [f32; 4], asked: &'a Cell<u32>)
    -> impl Fn([f32; 2]) -> Option<[f32; 2]> + 'a
{
    move |at| {
        asked.set(asked.get() + 1);
        off_stage_spot(&mut Copied(lines), blast, at)
    }
}

fn floor(lines: &[Met], blast: [f32; 4], [x, y]: [f32; 2]) -> Option<[f32; 2]> {
    lines.iter().filter(|line| {
        line.stand && (line.ends[0][0].min(line.ends[1][0])..=line.ends[0][0].max(line.ends[1][0])).contains(&x)
    }).filter_map(|line| line.height(x))
        .filter(|&height| (blast[3]..=y + 1.0).contains(&height))
        .max_by(f32::total_cmp)
        .map(|height| [x, height])
}

fn near(actual: [f32; 2], expected: [f32; 2]) {
    assert!((actual[0] - expected[0]).abs() <= 0.01 && (actual[1] - expected[1]).abs() <= 0.01,
        "actual {actual:?}, expected {expected:?}");
}

fn same_as_rule(lines: &[Met], blast: [f32; 4], at: [f32; 2], spot: [f32; 2]) {
    let mut stage = Copied(lines);
    let ledge = stage_geometry::nearest_ledge(&mut stage, blast, at).unwrap().unwrap();
    assert_eq!(Some(spot), stage_geometry::default_spot(&mut stage, blast, ledge).unwrap());
}

struct Counted<'a> {
    stage: Copied<'a>,
    calls: u32,
}

impl Probe for Counted<'_> {
    fn first(&mut self, from: [f32; 2], to: f32, platforms: bool) -> Result<Option<Met>, Unread> {
        self.calls += 1;
        self.stage.first(from, to, platforms)
    }
}

#[test]
fn a_fighter_switched_off_stage_on_training_goes_beside_the_nearest_ledge() {
    let lines = training();
    let asked = Cell::new(0);
    let off = rule(&lines, TRAINING, &asked);
    let saved = setup([-30., 0.]);
    let mut w = Wizard::for_save(saved, [false; 2], Who::P1);
    w.set_home(Some([-45., 0.]));
    w.step_on(&button(DPAD_RIGHT), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
    assert_eq!(w.setup[0].place, Place::OffStage);
    near(w.setup[0].at, [-125., -68.426]);
    same_as_rule(&lines, TRAINING, saved[0].at, w.setup[0].at);
    assert_eq!(w.cursor(), w.setup[0].at);
    assert_eq!(w.setup[1], saved[1]);
    assert_eq!(w.edits(), [Edits { spot: true, ..Edits::default() }, Edits::default()]);
    assert_eq!(w.commit(7, 308).edits[1], Edits::default());
    assert_eq!(asked.get(), 1);
}

#[test]
fn spot_y_then_a_neutral_frame_then_one_move_start_from_the_default() {
    let lines = training();
    for save in [false, true] {
        let asked = Cell::new(0);
        let off = rule(&lines, TRAINING, &asked);
        let initial = setup([-30., 0.]);
        let mut w = if save { Wizard::for_save(initial, [false; 2], Who::P1) } else { Wizard::new(initial, None) };
        w.step_on(&button(A), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
        w.step_on(&button(Y), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
        let spot = w.setup[0].at;
        assert_eq!(w.setup[0].place, Place::OffStage);
        near(spot, [-125., -68.426]);
        same_as_rule(&lines, TRAINING, initial[0].at, spot);
        assert_eq!(w.cursor(), spot);
        w.step_on(&Pad::default(), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
        assert_eq!((w.cursor(), w.setup[0].at), (spot, spot));
        w.step_on(&button(DPAD_RIGHT), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
        let moved = [spot[0] + 1., spot[1]];
        assert_eq!((w.cursor(), w.setup[0].at), (moved, moved));
        assert_eq!(asked.get(), 1);
    }
}

#[test]
fn battlefield_switches_go_beside_the_two_ledges() {
    let lines = battlefield();
    for (at, expected) in [([-30., 0.], [-159.993, -69.5]), ([30., 0.], [159.9945, -69.5])] {
        let asked = Cell::new(0);
        let off = rule(&lines, BATTLEFIELD, &asked);
        let mut w = Wizard::new(setup(at), None);
        w.step_on(&button(DPAD_RIGHT), Some, |point| floor(&lines, BATTLEFIELD, point), &off, &[]);
        assert_eq!(w.setup[0].place, Place::OffStage);
        near(w.setup[0].at, expected);
        same_as_rule(&lines, BATTLEFIELD, at, w.setup[0].at);
        assert_eq!(w.cursor(), w.setup[0].at);
        assert_eq!(asked.get(), 1);
    }
}

#[test]
fn a_crosshair_already_off_stage_is_not_moved_and_reads_nothing_more() {
    let lines = training();
    for (at, expected) in [
        ([-250., -117.85], stage_geometry::Place::Under),
        ([-125., 20.], stage_geometry::Place::Open),
    ] {
        let mut classified = Counted { stage: Copied(&lines), calls: 0 };
        assert_eq!(stage_geometry::classify(&mut classified, TRAINING, at), Ok(expected));
        let mut switched = Counted { stage: Copied(&lines), calls: 0 };
        assert_eq!(off_stage_spot(&mut switched, TRAINING, at), None);
        assert_eq!(switched.calls, classified.calls);
    }
    let asked = Cell::new(0);
    let off = rule(&lines, TRAINING, &asked);
    let mut w = Wizard::new(setup([-250., 0.014]), None);
    w.step_on(&button(A), Some, |_| None, &off, &[]);
    for _ in 0..59 {
        w.step_on(&Pad { stick: [0., -1.], ..Pad::default() }, Some, |_| None, &off, &[]);
    }
    let cursor = w.cursor();
    assert_eq!(stage_geometry::classify(&mut Copied(&lines), TRAINING, cursor), Ok(stage_geometry::Place::Under));
    assert_eq!(w.setup[0].at, [-250., 0.014]);
    w.step_on(&button(Y), Some, |_| None, &off, &[]);
    assert_eq!((w.setup[0].place, w.setup[0].at, w.cursor()), (Place::OffStage, cursor, cursor));
    assert_eq!(cursor[0], -250.);
    assert_eq!(asked.get(), 1);
}

#[test]
fn spot_y_classifies_the_crosshair_not_the_fighter() {
    let lines = training();
    let asked = Cell::new(0);
    let off = rule(&lines, TRAINING, &asked);
    // The fighter stays on the long floor (no floor is read under the free crosshair); the crosshair goes over the island.
    let mut w = Wizard::new(setup([-30., 0.]), None);
    w.step_on(&button(A), Some, |_| None, &off, &[]);
    for _ in 0..135 {
        w.step_on(&Pad { stick: [-1., 0.], ..Pad::default() }, Some, |_| None, &off, &[]);
    }
    for _ in 0..5 {
        w.step_on(&Pad { stick: [0., 1.], ..Pad::default() }, Some, |_| None, &off, &[]);
    }
    let cursor = w.cursor();
    assert_eq!(cursor, [-300., 10.]);
    assert_eq!((w.setup[0].place, w.setup[0].at), (Place::OnStage, [-30., 0.]));
    assert_eq!(stage_geometry::classify(&mut Copied(&lines), TRAINING, cursor), Ok(stage_geometry::Place::OverStage));
    w.step_on(&button(Y), Some, |_| None, &off, &[]);
    let spot = w.setup[0].at;
    assert_eq!(w.setup[0].place, Place::OffStage);
    // The island's left ledge is nearest the crosshair; the long floor's ledge is nearest the fighter.
    near(spot, [-375., -68.419]);
    same_as_rule(&lines, TRAINING, cursor, spot);
    let by_fighter = off_stage_spot(&mut Copied(&lines), TRAINING, [-30., 0.]).unwrap();
    near(by_fighter, [-125., -68.426]);
    assert_ne!(spot, by_fighter);
    assert_eq!(w.cursor(), spot);
    w.step_on(&Pad::default(), Some, |_| None, &off, &[]);
    assert_eq!((w.cursor(), w.setup[0].at), (spot, spot));
    assert_eq!(asked.get(), 1);
}

#[test]
fn the_crosshair_stays_free_after_the_switch() {
    let lines = training();
    let asked = Cell::new(0);
    let off = rule(&lines, TRAINING, &asked);
    let initial = setup([-30., 0.]);
    let mut w = Wizard::for_save(initial, [false; 2], Who::P1);
    w.step_on(&button(A), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
    w.step_on(&button(Y), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
    near(w.setup[0].at, [-125., -68.426]);
    same_as_rule(&lines, TRAINING, initial[0].at, w.setup[0].at);
    for _ in 0..60 {
        if w.cursor()[0] > -80. && w.cursor()[1] > 0. { break; }
        w.step_on(&Pad { stick: [1., 1.], ..Pad::default() }, Some, |at| floor(&lines, TRAINING, at), &off, &[]);
        assert_eq!((w.setup[0].place, w.setup[0].at), (Place::OffStage, w.cursor()));
    }
    assert!(w.cursor()[0] > -80. && w.cursor()[1] > 0.);
    for pad in [Pad::default(), button(L), button(R), button(B), button(A)] {
        w.step_on(&pad, Some, |at| floor(&lines, TRAINING, at), &off, &[]);
    }
    assert_eq!(asked.get(), 1);
    w.step_on(&button(B), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
    w.step_on(&button(DPAD_LEFT), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
    assert_eq!(w.setup[0].place, Place::OnStage);
    assert_eq!(asked.get(), 1);
    let at = w.setup[0].at;
    w.step_on(&button(DPAD_RIGHT), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
    assert_eq!(w.setup[0].place, Place::OffStage);
    same_as_rule(&lines, TRAINING, at, w.setup[0].at);
    assert_eq!(asked.get(), 2);
}

#[test]
fn opening_an_off_stage_save_is_not_a_switch() {
    let lines = training();
    let asked = Cell::new(0);
    let off = rule(&lines, TRAINING, &asked);
    let saved = [fighter(Place::OffStage, [-30., 40.]), fighter(Place::OnStage, [0., 0.])];
    assert_eq!(stage_geometry::classify(&mut Copied(&lines), TRAINING, saved[0].at),
        Ok(stage_geometry::Place::OverStage));
    let mut w = Wizard::for_save(saved, [false; 2], Who::P1);
    for pad in [Pad::default(), Pad::default(), button(A), Pad::default()] {
        w.step_on(&pad, Some, |at| floor(&lines, TRAINING, at), &off, &[]);
    }
    assert_eq!(w.step_on(&button(PLUS), Some, |at| floor(&lines, TRAINING, at), &off, &[]), Outcome::Start);
    assert_eq!(w.setup, saved);
    assert_eq!(w.edited(), [false; 2]);
    assert_eq!(w.cursor(), [-30., 40.]);
    assert_eq!(asked.get(), 0);
}

#[test]
fn a_stage_that_cannot_be_read_moves_nothing() {
    struct Broken;
    impl Probe for Broken {
        fn first(&mut self, _: [f32; 2], _: f32, _: bool) -> Result<Option<Met>, Unread> { Err(Unread) }
    }
    assert_eq!(off_stage_spot(&mut Broken, TRAINING, [-30., 0.]), None);
    let lines = [line([[-700., 0.], [700., 0.]], true, false)];
    assert_eq!(off_stage_spot(&mut Copied(&lines), TRAINING, [-30., 0.]), None);
    let asked = Cell::new(0);
    let off = rule(&lines, TRAINING, &asked);
    let mut w = Wizard::new(setup([-30., 0.]), None);
    w.step_on(&button(DPAD_RIGHT), Some, |at| floor(&lines, TRAINING, at), &off, &[]);
    assert_eq!((w.setup[0].place, w.setup[0].at, w.cursor()), (Place::OffStage, [-30., 0.], [-30., 0.]));
}

#[test]
fn the_runtime_reads_the_rule_only_at_the_switch() {
    let runtime = include_str!("../../runtime/features/record_place.rs");
    assert_eq!(runtime.matches("record_place::off_stage_spot(").count(), 1);
    assert_eq!(runtime.matches("RayProbe::new(").count(), 1);
    for call in ["classify(", "nearest_ledge(", "default_spot(", "floor_run(", "open_air_end("] {
        assert!(!runtime.contains(call), "{call}");
    }
    let model = include_str!("../record_place.rs").split("#[cfg(test)]").next().unwrap();
    assert_eq!(model.matches("off(at)").count(), 1);
}
}
