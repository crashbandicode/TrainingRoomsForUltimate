use super::chords::logical as key;
use super::edge_juggle::perfect::Outline;
use super::egg_zone::{stroke_layers, Outcome, Side, Stage, Step, Wizard};

fn stage(blast: [f32; 4], points: &[[f32; 2]]) -> Stage {
    let mut outline = Outline::new();
    for i in 0..points.len() {
        outline.add([points[i], points[(i + 1) % points.len()]]);
    }
    Stage::new(0, blast, [points[0], points[1]], outline).unwrap()
}

fn battlefield() -> Stage {
    stage(
        [-190.0, 190.0, 160.0, -120.0],
        &[[-70.0, 0.0], [70.0, 0.0], [52.0, -24.0], [-52.0, -24.0]],
    )
}

#[test]
fn zone_vertical_lines_respect_blast_margins_and_egg_diameter() {
    let stage = battlefield();
    let default = stage.resolve(None);
    assert_eq!(default.vertical(), [-54.0, 0.0]);
    let mut wizard = Wizard::new(None);
    assert_eq!(wizard.step(), Step::Top);
    wizard.input(&stage, [0.0, 1000.0], 0);
    assert_eq!(wizard.zone(&stage).vertical()[1], 152.0);
    wizard.input(&stage, [0.0, 0.0], 1 << key::A);
    wizard.input(&stage, [0.0, -1000.0], 0);
    assert_eq!(wizard.zone(&stage).vertical(), [-96.0, 152.0]);
    for _ in 0..64 {
        wizard.input(&stage, [0.0, 1.0], 0);
    }
    let [bottom, top] = wizard.zone(&stage).vertical();
    assert_eq!(top, 152.0);
    assert!((143.0..=144.0).contains(&bottom));
}

#[test]
fn zone_fractions_remap_and_clip_to_each_stages_real_outline() {
    let stage = battlefield();
    let zone = stage.resolve(Some([128, 0, 0, 255]));
    assert_eq!(zone.horizontal(Side::Left), Some([-182.0, -35.0]));
    assert_eq!(zone.horizontal(Side::Right), Some([35.0, 182.0]));
    assert!(stage.contains(&zone, Side::Right, [48.0, -34.0]));
    assert!(!stage.contains(&zone, Side::Right, [48.0, -12.0]));
    assert!(!stage.contains(&zone, Side::Right, [48.0, -25.0]));
    assert!(stage.under_main([48.0, -26.0]));
    assert!(!stage.under_main([48.0, -23.0]));
    let upper = stage.resolve(Some([255, 0, 0, 255]));
    assert!(!stage.contains(&upper, Side::Right, [48.0, 20.0]));
    assert!(stage.contains(&upper, Side::Right, [76.0, 20.0]));
    assert!(stage.allows_move(&upper, Side::Right, [77.0, 20.0], [77.0, -34.0]));
    assert!(!stage.allows_move(&upper, Side::Right, [77.0, 20.0], [48.0, -34.0]));
    let asymmetric = self::stage(
        [-150.0, 190.0, 140.0, -84.0],
        &[[-30.0, 10.0], [90.0, -6.0], [51.0, -25.0], [-15.0, -25.0]],
    );
    let zone = asymmetric.resolve(Some([255, 0, 0, 255]));
    assert_eq!(zone.horizontal(Side::Left), Some([-142.0, 0.0]));
    assert_eq!(zone.horizontal(Side::Right), Some([60.0, 182.0]));
    assert_eq!(zone.vertical(), [-60.0, 132.0]);
    assert!(asymmetric.supports([[-30.0, 10.0], [90.0, -6.0]]));
    assert!(!asymmetric.supports([[15.0, 27.0], [45.0, 27.0]]));
}

#[test]
fn a_solid_contour_can_protrude_outboard_of_the_ledge() {
    let stage = stage(
        [-140.0, 140.0, 140.0, -130.0],
        &[[-40.0, 5.0], [40.0, 5.0], [70.0, -60.0], [-70.0, -60.0]],
    );
    let zone = stage.resolve(Some([128, 0, 0, 255]));
    assert!(!stage.contains(&zone, Side::Right, [48.0, -38.0]));
    assert!(stage.contains(&zone, Side::Right, [78.0, -38.0]));
}

#[test]
fn training_keeps_the_real_recovery_side_when_the_other_end_is_past_blast() {
    // Card trace measured the two floor endpoints and blast bounds on stage 308.
    let ledges = [[-80.0, 0.0], [620.0, 0.0]];
    let blast = [-589.9998, 500.0, 360.00015, -137.85207];
    let mut outline = Outline::new();
    let points = [ledges[0], ledges[1], [620.0, -30.0], [-80.0, -30.0]];
    for i in 0..points.len() {
        outline.add([points[i], points[(i + 1) % points.len()]]);
    }
    let stage = Stage::new(308, blast, ledges, outline).expect("the left ledge has recovery room");
    assert_eq!(stage.ledges(), ledges);
    let mut wizard = Wizard::new(None);
    assert_eq!(wizard.input(&stage, [0.0; 2], 1 << key::X), Outcome::Editing);
    let Outcome::Apply(bins) = wizard.input(&stage, [0.0; 2], 1 << key::PLUS) else {
        panic!("the single-sided zone must be applicable");
    };
    let zone = stage.resolve(Some(bins));
    assert!(zone.horizontal(Side::Left).is_some());
    assert!(zone.horizontal(Side::Right).is_none());
    assert!(stage.contains(&zone, Side::Left, [-100.0, -10.0]));
    assert!(!stage.contains(&zone, Side::Right, [490.0, -40.0]));
    assert!(stage.supports(ledges));
    assert!(Stage::new(308, [-70.0, blast[1], blast[2], blast[3]], ledges, outline).is_none());
}

#[test]
fn zone_wizard_polls_guard_edge_controls() {
    let stage = battlefield();
    let mut wizard = Wizard::new(None);
    let press = |wizard: &mut Wizard, button| wizard.input(&stage, [0.0; 2], 1u16 << button);
    assert_eq!(press(&mut wizard, key::B), Outcome::Cancel);
    assert_eq!(press(&mut wizard, key::PLUS), Outcome::Editing);
    assert_eq!(wizard.step(), Step::Top);
    for expected in [Step::Bottom, Step::Inner, Step::Outer, Step::Preview] {
        assert_eq!(press(&mut wizard, key::A), Outcome::Editing);
        assert_eq!(wizard.step(), expected);
    }
    press(&mut wizard, key::B);
    assert_eq!(wizard.step(), Step::Outer);
    let before = wizard.zone(&stage).horizontal(Side::Right).unwrap()[1];
    wizard.input(&stage, [1.0, 0.0], 0);
    assert!(wizard.zone(&stage).horizontal(Side::Right).unwrap()[1] > before);
    assert_eq!(press(&mut wizard, key::X), Outcome::Editing);
    assert_eq!(wizard.step(), Step::Preview);
    let Outcome::Apply(value) = press(&mut wizard, key::PLUS) else {
        panic!("Preview must apply with +");
    };
    let resolved = stage.resolve(Some(value));
    assert_eq!(resolved, wizard.zone(&stage));
    for side in [Side::Left, Side::Right] {
        let [lo, hi] = resolved.rect(side).unwrap();
        assert!(lo.into_iter().zip(hi).all(|(a, b)| a <= b));
    }
}

#[test]
fn zone_overlay_paints_a_dark_outline_behind_each_stroke() {
    for width in [2.0, 3.0, 5.0] {
        for color in [[66, 142, 255, 250], [255, 221, 77, 180], [206, 132, 255, 190], [112, 204, 255, 222]] {
            let [(outline, dark), foreground] = stroke_layers(width, color);
            assert!(outline - width >= 4.0);
            assert!(dark[..3].iter().all(|&v| v < 32) && dark[3] >= 224);
            assert_eq!(foreground, (width, color));
        }
    }
}
