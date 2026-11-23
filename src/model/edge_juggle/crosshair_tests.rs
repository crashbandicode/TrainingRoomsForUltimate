lib_unit_tests! {
use super::*;
use crate::model::btt::{self, Pad};
use crate::model::chords::logical::{DPAD_RIGHT, DPAD_UP};
use crate::model::panes::{EggPicture, EGG_WHOLE};

fn stage() -> Stage {
    Stage::new(
        [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
        [[-80.0, 0.0], [80.0, 0.0]],
    ).unwrap()
}

fn wizard(drill: Drill, fighter: Option<i32>) -> Wizard {
    Wizard::new(drill, stage(), [0.0; 2], 100, (3, 0), fighter).unwrap()
}

fn pad(stick: [f32; 2], held: u16) -> Pad {
    Pad { stick, held, ..Pad::default() }
}

#[test]
fn edge_wizard_x_and_y_pick_side_defaults() {
    let stage = stage();
    let mut link = wizard(Drill::GuardEdge, Some(perfect::link::KIND));
    assert_eq!(link.setup.cpu, [-110.0, 18.0]);
    link.steer(stage, &pad([1.0, 0.0], 0));
    assert!(!link.unmoved());
    link.set_side_default(stage, false);
    assert_eq!(link.setup.cpu, [-110.0, 18.0]);
    assert!(link.unmoved());
    link.set_side_default(stage, true);
    assert_eq!(link.setup.cpu, [118.0, 38.0]);
    assert!(link.unmoved());
    link.steer(stage, &pad([1.0, 0.0], 0));
    link.set_side_default(stage, false);
    assert_eq!(link.setup.cpu, [118.0, 38.0]);
    link.set_side_default(stage, true);
    assert_eq!(link.setup.cpu, [-110.0, 18.0]);

    let mut lucas = wizard(Drill::GuardEdge, Some(perfect::lucas::KIND));
    assert_eq!(lucas.setup.cpu, [-98.0, 2.0]);
    lucas.set_side_default(stage, true);
    assert_eq!(lucas.setup.cpu, [155.0, -69.5]);
    lucas.set_side_default(stage, true);
    assert_eq!(lucas.setup.cpu, [-98.0, 2.0]);
}

#[test]
fn edge_wizard_clipped_start_uses_the_lower_middle() {
    let stage = Stage::new(
        [-100.0, 100.0, 185.0, -140.0], [-90.0, 90.0, 100.0, -110.0],
        [[-80.0, 0.0], [80.0, 0.0]],
    ).unwrap();
    let mut link = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, (3, 0), Some(perfect::link::KIND)).unwrap();
    assert_eq!(link.setup.cpu, [-90.0, -69.5]);
    link.set_side_default(stage, true);
    assert_eq!(link.setup.cpu, [90.0, -69.5]);
}

#[test]
fn edge_wizard_refuses_a_start_over_the_stage() {
    let stage = stage();
    for y in [-20.0, 0.0, 20.0] {
        for x in [-80.999, 0.0, 80.999] {
            assert_eq!(stage.off_stage([x, y]), None);
        }
        assert_eq!(stage.off_stage([-81.0, y]), Some([-81.0, y]));
        assert_eq!(stage.off_stage([81.0, y]), Some([81.0, y]));
    }
    assert_eq!(stage.off_stage([f32::NAN, 0.0]), None);
    assert_eq!(stage.edge_cursor([-1000.0, 1000.0]), Some([-229.0, 184.0]));
    assert_eq!(stage.edge_cursor([1000.0, -1000.0]), Some([229.0, -139.0]));
    let mut edge = wizard(Drill::GuardEdge, None);
    assert!(edge.accepts_cpu(stage));
    for _ in 0..40 {
        edge.steer(stage, &pad([1.0, 0.0], 0));
    }
    assert!(!edge.accepts_cpu(stage));
    let mut juggle = wizard(Drill::Juggler, None);
    juggle.steer(stage, &pad([1.0, 0.0], 0));
    assert!(juggle.accepts_cpu(stage));
}

#[test]
fn edge_bests_bucket_reads_the_point() {
    let stage = stage();
    assert_eq!(stage.bucket([-90.0, -1.0]), 0);
    assert_eq!(stage.bucket([90.0, -1.0]), 1);
    assert_eq!(stage.bucket([-90.0, 0.0]), 2);
    assert_eq!(stage.bucket([90.0, 0.0]), 3);
    let mut left_entries = 0;
    let mut shared_entries = 0;
    for key in [(0, 0), (3, 0), (311, 0), (4, 0), (3, 1), (89, 0)] {
        for fighter in 0..=0xff {
            for percent in [0, 60, 100, 130, 300] {
                let w = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], percent, key, Some(fighter)).unwrap();
                if let Some(offset) = w.left_start {
                    left_entries += 1;
                    assert_ne!(offset[1], 0.0);
                    let point = [stage.ledges[0][0] - offset[0], stage.ledges[0][1] + offset[1]];
                    if stage.off_stage(point) == Some(point) {
                        assert_eq!(stage.bucket(w.default_cpu(stage, 0).unwrap()), 2 * u8::from(offset[1] > 0.0));
                    }
                }
                if let Some(offset) = w.area_offset {
                    shared_entries += 1;
                    assert_ne!(offset[1], 0.0);
                    let point = [stage.ledges[1][0] + offset[0], stage.ledges[1][1] + offset[1]];
                    if stage.off_stage(point) == Some(point) {
                        assert_eq!(stage.bucket(w.default_cpu(stage, 1).unwrap()), 1 + 2 * u8::from(offset[1] > 0.0));
                    }
                }
            }
        }
    }
    assert!(left_entries > 0 && shared_entries > 0);
}

#[test]
fn edge_remembered_start_follows_its_side() {
    let stage = stage();
    let mut first = wizard(Drill::GuardEdge, None);
    let diddy = wizard(Drill::GuardEdge, Some(perfect::diddy::KIND));
    assert_eq!(diddy.remembered(stage, first.setup, true).cpu, [-94.0, -30.0]);
    first.set_side_default(stage, true);
    assert_eq!(diddy.remembered(stage, first.setup, true).cpu, [94.0, -30.0]);
    first.steer(stage, &pad([0.5, 0.0], 0));
    assert_eq!(diddy.remembered(stage, first.setup, false), first.setup);
}

#[test]
fn edge_wizard_has_three_pages() {
    let mut edge = wizard(Drill::GuardEdge, None);
    assert_eq!(edge.step, WizardStep::Cpu);
    assert!(!edge.back());
    edge.advance();
    assert_eq!(edge.step, WizardStep::Player);
    edge.advance();
    assert_eq!(edge.step, WizardStep::Preview);
    assert!(edge.back());
    assert_eq!(edge.step, WizardStep::Player);
    assert!(edge.back());
    assert_eq!(edge.step, WizardStep::Cpu);
    assert!(!edge.back());

    let mut juggle = wizard(Drill::Juggler, None);
    assert_eq!(juggle.step, WizardStep::Cpu);
    assert!(!juggle.back());
    for step in [WizardStep::Percent, WizardStep::Player, WizardStep::Preview] {
        juggle.advance();
        assert_eq!(juggle.step, step);
    }
}

#[test]
fn juggle_wizard_keeps_its_percent_page() {
    let stage = stage();
    let mut juggle = wizard(Drill::Juggler, None);
    for _ in 0..200 {
        juggle.steer(stage, &pad([1.0, 1.0], 0));
    }
    assert_eq!(juggle.setup.cpu, [160.0, 100.0]);
    juggle.advance();
    assert_eq!(juggle.step, WizardStep::Percent);
    for (delta, expected) in [(1, 101), (10, 111), (-1, 110), (-10, 100), (300, 300), (-301, 0)] {
        juggle.change_percent(delta);
        assert_eq!(juggle.setup.percent, expected);
    }
}

#[test]
fn edge_and_juggle_wizards_use_the_shared_crosshair() {
    let stage = stage();
    for drill in [Drill::GuardEdge, Drill::Juggler] {
        for player in [false, true] {
            let mut w = wizard(drill, None);
            if player {
                if drill == Drill::Juggler { w.advance(); }
                w.advance();
                assert_eq!(w.step, WizardStep::Player);
            }
            let mut reference = w.cursor();
            let (mut was, mut age) = (0, 0);
            let samples = [pad([0.4, -0.6], 0), pad([0.0; 2], 1 << DPAD_RIGHT)]
                .into_iter().chain(std::iter::repeat_n(pad([0.0; 2], 1 << DPAD_UP), 40));
            for sample in samples {
                let m = btt::placing_motion(&sample, was, &mut age);
                was = sample.held;
                let wanted = [reference[0] + m[0], reference[1] + m[1]];
                reference = if player {
                    stage.player_cursor(drill, wanted).unwrap()
                } else if drill == Drill::GuardEdge {
                    stage.edge_cursor(wanted).unwrap()
                } else {
                    stage.camera_point(wanted).unwrap()
                };
                w.steer(stage, &sample);
                assert_eq!(w.cursor().map(f32::to_bits), reference.map(f32::to_bits));
            }
        }
    }
    let stage = Stage::new(
        [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
        [[-79.99, 0.0], [79.99, 0.0]],
    ).unwrap();
    let mut w = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, (3, 0), Some(0x10)).unwrap();
    let start = stage.ledges[0][0] - 18.0;
    assert_eq!(w.setup.cpu[0].to_bits(), start.to_bits());
    w.steer(stage, &pad([0.0; 2], 1 << DPAD_RIGHT));
    assert_eq!(w.setup.cpu[0].to_bits(), (start + 1.0).to_bits());
}

#[test]
fn the_cpu_point_follows_the_crosshair_in_the_cpu_step() {
    for drill in [Drill::GuardEdge, Drill::Juggler] {
        let stage = stage();
        let mut w = wizard(drill, None);
        assert_eq!(w.setup.cpu, w.cursor(), "{drill:?}");
        for sample in [pad([-1.0, 0.0], 0), pad([0.0, -1.0], 0), pad([0.0; 2], 1 << DPAD_RIGHT), pad([0.0; 2], 1 << DPAD_UP)] {
            w.steer(stage, &sample);
            // The point a picker holds the CPU at is the crosshair's own.
            assert_eq!(w.setup.cpu, w.cursor(), "{drill:?}");
            assert_eq!(w.marks().pointer, Some(w.cursor()));
        }
    }
}

#[test]
fn edge_wizard_look_points_at_the_crosshair() {
    for drill in [Drill::GuardEdge, Drill::Juggler] {
        let mut w = wizard(drill, None);
        let cpu = w.setup.cpu;
        let marks = w.marks();
        assert_eq!(marks.pointer, Some(cpu));
        assert_eq!(marks.looks[0], None);
        assert_eq!(marks.looks[btt::LOOKS - 1].unwrap().1.picture, EggPicture::Cursor);
        if drill == Drill::Juggler {
            w.advance();
            let marks = w.marks();
            assert_eq!(marks.looks[0], Some((cpu, EGG_WHOLE)));
            assert_eq!(marks.pointer, None);
        }
        w.advance();
        let marks = w.marks();
        assert_eq!(marks.looks[0], Some((cpu, EGG_WHOLE)));
        assert_eq!(marks.looks[btt::LOOKS - 1].unwrap().0, w.setup.p1);
        assert_eq!(marks.pointer, None);
        w.advance();
        assert_eq!(w.marks().pointer, Some(cpu));
    }
}

#[test]
fn wizard_notice_fits_its_buffer() {
    use std::fmt::Write;
    use crate::model::notify::Body;
    use crate::model::storage::FixedPath;

    let source = include_str!("../../runtime/features/edge_juggle.rs");
    assert!(source.contains("notice: FixedPath<128>"));
    assert!(source.contains("let mut text = FixedPath::<128>::new();"));
    for format in [
        "CPU ({x:.0}, {y:.0}) · Stick or D-pad move · A next · X default start · Y other side",
        "CPU x {x:.0} · Height {:+.0} · Stick or D-pad move · A next",
        "{percent}% · D-pad change · A next",
        "P1 ({px:.0}, {py:.0}) · Stick or D-pad move · A next",
        "CPU ({x:.0}, {y:.0}) · {percent}% · P1 ({px:.0}, {py:.0}) · A/+ start",
    ] {
        assert!(source.contains(format), "{format}");
    }
    // A hundred times any stage's blast box; the wizard clamps points to the stage's boxes.
    let x = -99999.0_f32;
    let y = x;
    let px = x;
    let py = x;
    let mut edge = FixedPath::<128>::new();
    assert!(write!(edge, "CPU ({x:.0}, {y:.0}) · Stick or D-pad move · A next · X default start · Y other side").is_ok());
    assert_eq!(Body::copied(edge.as_str()).as_str(), edge.as_str());

    let percent = 300;
    let mut juggle = FixedPath::<128>::new();
    assert!(write!(juggle, "CPU x {x:.0} · Height {:+.0} · Stick or D-pad move · A next", -199998.0_f32).is_ok());
    assert_eq!(Body::copied(juggle.as_str()).as_str(), juggle.as_str());

    let mut percent_page = FixedPath::<128>::new();
    assert!(write!(percent_page, "{percent}% · D-pad change · A next").is_ok());
    assert_eq!(Body::copied(percent_page.as_str()).as_str(), percent_page.as_str());

    let mut player = FixedPath::<128>::new();
    assert!(write!(player, "P1 ({px:.0}, {py:.0}) · Stick or D-pad move · A next").is_ok());
    assert_eq!(Body::copied(player.as_str()).as_str(), player.as_str());

    let mut preview = FixedPath::<128>::new();
    assert!(write!(preview, "CPU ({x:.0}, {y:.0}) · {percent}% · P1 ({px:.0}, {py:.0}) · A/+ start").is_ok());
    assert_eq!(Body::copied(preview.as_str()).as_str(), preview.as_str());
}
}
