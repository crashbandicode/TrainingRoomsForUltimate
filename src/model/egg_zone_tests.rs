use super::chords::logical as key;
use super::edge_juggle::perfect::Outline;
use super::egg_zone::{crop_window, CropRect, CropView, Outcome, Side, Stage, Step, Wizard};

fn stage(blast: [f32; 4], points: &[[f32; 2]]) -> Stage {
    let mut outline = Outline::new();
    for i in 0..points.len() {
        outline.add([points[i], points[(i + 1) % points.len()]]);
    }
    Stage::new(0, blast, [points[0], points[1]], outline, [Some(blast[0]), Some(blast[1])], &[]).unwrap()
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
    let stage = Stage::new(308, blast, ledges, outline, [Some(-170.0), None], &[])
        .expect("the left ledge has recovery room");
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
    assert!(Stage::new(308, [-70.0, blast[1], blast[2], blast[3]], ledges, outline,
        [Some(-170.0), None], &[]).is_none());
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
fn zone_wizard_writes_the_notice_again_when_x_resets_the_preview() {
    let stage = battlefield();
    let mut wizard = Wizard::new(None);
    let factory = wizard.zone(&stage);
    let before = wizard;
    assert_eq!(wizard.input(&stage, [0.0, 1.0], 0), Outcome::Editing);
    assert_ne!(wizard.zone(&stage), factory);
    assert!(!wizard.prompts_after(before));
    let before = wizard;
    assert_eq!(wizard.input(&stage, [0.0; 2], 1 << key::A), Outcome::Editing);
    assert!(wizard.prompts_after(before));
    for _ in 0..3 {
        wizard.input(&stage, [0.0; 2], 1 << key::A);
    }
    assert_eq!(wizard.step(), Step::Preview);
    let before = wizard;
    assert_eq!(wizard.input(&stage, [0.0; 2], 1 << key::X), Outcome::Editing);
    assert!(wizard.prompts_after(before));
    assert_eq!(wizard.zone(&stage), Wizard::new(None).zone(&stage));
    let before = wizard;
    assert_eq!(wizard.input(&stage, [0.0; 2], 1 << key::X), Outcome::Editing);
    assert!(!wizard.prompts_after(before));
    let mut wizard = Wizard::new(None);
    wizard.input(&stage, [0.0, 1.0], 0);
    let before = wizard;
    assert_eq!(wizard.input(&stage, [0.0; 2], 1 << key::X), Outcome::Editing);
    assert_eq!(wizard.step(), Step::Preview);
    assert!(wizard.prompts_after(before));
}

fn crop_rect(x: f32, y: f32, w: f32, h: f32, color: [u8; 4]) -> Option<CropRect> {
    Some(CropRect { x, y, w, h, color })
}

fn crop_overlaps(r: CropRect, [left, right]: [f32; 2], [bottom, top]: [f32; 2]) -> bool {
    r.x - r.w / 2.0 < right && r.x + r.w / 2.0 > left
        && r.y - r.h / 2.0 < top && r.y + r.h / 2.0 > bottom
}

fn crop_world() -> CropView {
    CropView {
        vertical: [-100.0, 200.0],
        sides: [Some([-700.0, -400.0]), Some([400.0, 700.0])],
        limits: [-300.0, 300.0],
    }
}

fn crop_inset() -> CropView {
    CropView {
        vertical: [300.0, 380.0],
        sides: [Some([720.0, 760.0]), Some([840.0, 880.0])],
        limits: [780.0, 820.0],
    }
}

#[test]
fn crop_window_dims_everything_outside_both_zones() {
    let world = crop_world();
    let rects = crop_window(world, None, Step::Top);
    let with_inset = crop_window(crop_world(), Some(crop_inset()), Step::Top);
    let dim = [10, 15, 27, 140];
    assert_eq!(rects[..5], [
        crop_rect(0.0, 370.0, 1920.0, 340.0, dim),
        crop_rect(0.0, -320.0, 1920.0, 440.0, dim),
        crop_rect(-830.0, 50.0, 260.0, 300.0, dim),
        crop_rect(0.0, 50.0, 800.0, 300.0, dim),
        crop_rect(830.0, 50.0, 260.0, 300.0, dim),
    ]);
    let area: f32 = rects[..5].iter().flatten().map(|r| r.w * r.h).sum();
    assert_eq!(area + 2.0 * 300.0 * 300.0, 1920.0 * 1080.0);
    assert_eq!(rects[..5], with_inset[..5]);
    for (i, rect) in rects[..5].iter().flatten().enumerate() {
        for side in crop_world().sides.into_iter().flatten() {
            assert!(!crop_overlaps(*rect, side, crop_world().vertical));
        }
        for other in rects[..5].iter().flatten().skip(i + 1) {
            assert!(!crop_overlaps(*rect, [other.x - other.w / 2.0, other.x + other.w / 2.0],
                [other.y - other.h / 2.0, other.y + other.h / 2.0]));
        }
    }
}

#[test]
fn crop_window_edges_lie_outside_the_zone_and_the_active_one_is_thicker() {
    let rects = crop_window(crop_world(), None, Step::Top);
    let blue = [66, 142, 255, 255];
    let yellow = [255, 221, 77, 255];
    let purple = [206, 132, 255, 255];
    assert_eq!(rects[7..15], [
        crop_rect(-550.0, 205.5, 310.0, 11.0, blue),
        crop_rect(-550.0, -102.5, 310.0, 5.0, blue),
        crop_rect(-397.5, 50.0, 5.0, 310.0, yellow),
        crop_rect(-702.5, 50.0, 5.0, 310.0, purple),
        crop_rect(550.0, 205.5, 310.0, 11.0, blue),
        crop_rect(550.0, -102.5, 310.0, 5.0, blue),
        crop_rect(397.5, 50.0, 5.0, 310.0, yellow),
        crop_rect(702.5, 50.0, 5.0, 310.0, purple),
    ]);
    for step in [Step::Top, Step::Bottom, Step::Inner, Step::Outer, Step::Preview] {
        let rects = crop_window(crop_world(), None, step);
        for (i, rect) in rects[7..15].iter().flatten().enumerate() {
            for side in crop_world().sides.into_iter().flatten() {
                assert!(!crop_overlaps(*rect, side, crop_world().vertical));
            }
            let edge = [Step::Top, Step::Bottom, Step::Inner, Step::Outer][i % 4];
            let thickness = if i % 4 < 2 { rect.h } else { rect.w };
            assert_eq!(thickness, if step == edge || step == Step::Preview { 11.0 } else { 5.0 });
        }
    }
}

#[test]
fn crop_window_with_one_side_or_a_zone_off_the_screen() {
    let dim = [10, 15, 27, 140];
    let one = crop_window(CropView { vertical: [-100.0, 200.0], sides: [Some([-1200.0, -800.0]), None],
        limits: [-300.0, 300.0] }, None, Step::Top);
    assert_eq!(one[..3], [
        crop_rect(0.0, 370.0, 1920.0, 340.0, dim),
        crop_rect(0.0, -320.0, 1920.0, 440.0, dim),
        crop_rect(80.0, 50.0, 1760.0, 300.0, dim),
    ]);
    assert_eq!(one.iter().flatten().count(), 10);
    let below = crop_window(CropView { vertical: [-700.0, -600.0], ..crop_world() }, None, Step::Top);
    assert_eq!(below[0], crop_rect(0.0, 0.0, 1920.0, 1080.0, dim));
    assert_eq!(below.iter().flatten().count(), 11);
    assert!(below.iter().flatten().all(|r| r.color != [10, 15, 27, 255]));
}

#[test]
fn crop_window_puts_a_handle_on_the_boundary_the_step_moves() {
    let blue = [66, 142, 255, 255];
    let yellow = [255, 221, 77, 255];
    let purple = [206, 132, 255, 255];
    let ink = [10, 15, 27, 255];
    for (step, points, size, color) in [
        (Step::Top, [[-550.0, 205.5], [550.0, 205.5]], [36.0, 30.0], blue),
        (Step::Bottom, [[-550.0, -105.5], [550.0, -105.5]], [36.0, 30.0], blue),
        (Step::Inner, [[-394.5, 50.0], [394.5, 50.0]], [30.0, 36.0], yellow),
        (Step::Outer, [[-705.5, 50.0], [705.5, 50.0]], [30.0, 36.0], purple),
    ] {
        let rects = crop_window(crop_world(), None, step);
        for (i, [x, y]) in points.into_iter().enumerate() {
            assert_eq!(rects[15 + 2 * i], crop_rect(x, y, size[0], size[1], color));
            let inner = if size[0] > size[1] { [20.0, 14.0] } else { [14.0, 20.0] };
            assert_eq!(rects[16 + 2 * i], crop_rect(x, y, inner[0], inner[1], ink));
        }
    }
    assert_eq!(crop_window(crop_world(), None, Step::Preview).iter().flatten().count(), 15);
    let one = crop_window(CropView { sides: [Some([-1200.0, -800.0]), None], ..crop_world() }, None, Step::Top);
    assert!(one.iter().flatten().any(|r| *r == crop_rect(-877.5, 205.5, 36.0, 30.0, blue).unwrap()));
    assert_eq!(one.iter().flatten().filter(|r| r.color == ink).count(), 1);
    let below = crop_window(CropView { vertical: [-700.0, 0.0], ..crop_world() }, None, Step::Inner);
    assert!(below.iter().flatten().any(|r| *r == crop_rect(-394.5, -267.5, 30.0, 36.0, yellow).unwrap()));
    let off = crop_window(CropView { sides: [Some([-1400.0, -1100.0]), Some([400.0, 700.0])], ..crop_world() }, None, Step::Top);
    let handles: Vec<_> = off.iter().flatten().filter(|r| r.color == ink).collect();
    assert_eq!(handles.len(), 1);
    assert_eq!(handles[0].x, 550.0);
    for step in [Step::Inner, Step::Outer] {
        let off = crop_window(CropView { vertical: [600.0, 700.0], ..crop_world() }, None, step);
        assert!(off.iter().flatten().all(|r| r.color != ink));
    }
}

#[test]
fn crop_window_handles_require_the_active_edge_on_both_axes() {
    let ink = [10, 15, 27, 255];
    let right = CropView { vertical: [300.0, 541.0], sides: [None, Some([400.0, 700.0])],
        limits: [-300.0, 300.0] };
    for (view, step) in [
        (right, Step::Top),
        (CropView { vertical: [-541.0, -300.0], ..right }, Step::Bottom),
        (CropView { vertical: [-100.0, 200.0], sides: [None, Some([600.0, 961.0])], ..right }, Step::Outer),
        (CropView { vertical: [-100.0, 200.0], sides: [Some([-961.0, -600.0]), None], ..right }, Step::Outer),
        (CropView { vertical: [-100.0, 200.0], sides: [None, Some([-961.0, 300.0])], ..right }, Step::Inner),
        (CropView { vertical: [300.0, 540.0], ..right }, Step::Top),
    ] {
        assert!(crop_window(view, None, step).iter().flatten().all(|r| r.color != ink));
    }
    let partial = crop_window(CropView { vertical: [300.0, 535.0], ..right }, None, Step::Top);
    let handles: Vec<_> = partial.iter().flatten().filter(|r| r.color == ink).collect();
    assert_eq!(handles, [crop_rect(550.0, 540.5, 20.0, 14.0, ink).as_ref().unwrap()]);
}

#[test]
fn crop_window_marks_the_inner_limit_once_a_side() {
    let yellow = [255, 221, 77, 255];
    let rects = crop_window(crop_world(), None, Step::Top);
    assert_eq!(rects[5..7], [crop_rect(-300.0, 50.0, 3.0, 300.0, yellow),
        crop_rect(300.0, 50.0, 3.0, 300.0, yellow)]);
    let one = crop_window(CropView { sides: [None, Some([400.0, 700.0])], ..crop_world() }, None, Step::Preview);
    assert_eq!(one.iter().flatten().filter(|r| r.color == yellow && r.w == 3.0).count(), 1);
}

#[test]
fn crop_window_draws_the_inset_after_the_world_view() {
    let blue = [66, 142, 255, 255];
    let yellow = [255, 221, 77, 255];
    let purple = [206, 132, 255, 255];
    let rects = crop_window(crop_world(), Some(crop_inset()), Step::Top);
    assert!(rects.iter().all(Option::is_some));
    assert_eq!(rects[..19], crop_window(crop_world(), None, Step::Top)[..19]);
    assert_eq!(rects[19..], [
        crop_rect(780.0, 340.0, 2.0, 80.0, yellow), crop_rect(820.0, 340.0, 2.0, 80.0, yellow),
        crop_rect(740.0, 382.0, 44.0, 4.0, blue), crop_rect(740.0, 299.0, 44.0, 2.0, blue),
        crop_rect(761.0, 340.0, 2.0, 84.0, yellow), crop_rect(719.0, 340.0, 2.0, 84.0, purple),
        crop_rect(860.0, 382.0, 44.0, 4.0, blue), crop_rect(860.0, 299.0, 44.0, 2.0, blue),
        crop_rect(839.0, 340.0, 2.0, 84.0, yellow), crop_rect(881.0, 340.0, 2.0, 84.0, purple),
    ]);
}

#[test]
fn zone_samples_are_spots_a_target_can_appear_at() {
    let stage = battlefield();
    let zone = stage.resolve(Some([128, 0, 255, 255]));
    let spots = stage.samples(&zone);
    for (i, spot) in spots.iter().enumerate() {
        if let Some(spot) = spot {
            assert!(stage.contains(&zone, Side::BOTH[i / 5], *spot));
        }
    }
    let [[lo, low], [hi, high]] = zone.rect(Side::Right).unwrap();
    let y = low + (high - low) / 6.0;
    assert!(stage.allows_move(&zone, Side::Right, [hi, y], [lo, y]));
    assert_eq!(spots[5], Some([hi + 0.35 * (lo - hi), y]));

    let protruding = self::stage([-140.0, 140.0, 140.0, -130.0],
        &[[-40.0, 5.0], [40.0, 5.0], [70.0, -60.0], [-70.0, -60.0]]);
    let blocked_zone = protruding.resolve(Some([128, 0, 0, 255]));
    let [[inner, low], [outer, high]] = blocked_zone.rect(Side::Right).unwrap();
    let spots = protruding.samples(&blocked_zone);
    let blocked = (0..5).find(|row| {
        let y = low + (high - low) * ((*row + 1) as f32 / 6.0);
        protruding.contains(&blocked_zone, Side::Right, [outer, y])
            && !protruding.allows_move(&blocked_zone, Side::Right, [outer, y], [inner, y])
    }).expect("fixture has an obstructed row");
    let spot = spots[5 + blocked].unwrap();
    assert!(protruding.contains(&blocked_zone, Side::Right, spot));
    assert!(protruding.allows_move(&blocked_zone, Side::Right, [outer, spot[1]], spot));
    assert!(spot[0] > inner);

    let one_side = self::stage([-589.9998, 500.0, 360.00015, -137.85207],
        &[[-80.0, 0.0], [620.0, 0.0], [620.0, -30.0], [-80.0, -30.0]]);
    assert!(one_side.samples(&one_side.resolve(None))[5..].iter().all(Option::is_none));
}
