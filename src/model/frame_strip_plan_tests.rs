//! SPEC-input-06 §3.2–3.4 and §5.2: what the strip's panes show, and that the renderer writes only what changed.

lib_unit_tests! {
use super::*;
use crate::model::frame_strip::{Move, Sample, Tracker};
use crate::model::input_delay::MappedInputs;

const NONE: MappedInputs = MappedInputs::NEUTRAL;
const A: MappedInputs = MappedInputs { buttons: 0x1, lstick_x: 0, lstick_y: 0, rstick_x: 0, rstick_y: 0 };
const UP_A: MappedInputs = MappedInputs { buttons: 0x1, lstick_x: 0, lstick_y: 100, rstick_x: 0, rstick_y: 0 };

fn idle() -> Sample {
    Sample { actionable: true, ..Sample::default() }
}

fn attack(mv: Move, status: i32, frame: u32) -> Sample {
    Sample { attack: Some(mv), status, motion_frame: frame as f32, ..Sample::default() }
}

/// The CPU hit on 7 with 4 frames of hitlag and 18 of hitstun (free on 29), as in the model's tests.
fn cpu(frame: u32) -> Sample {
    match frame {
        0..=6 => idle(),
        7..=10 => Sample { stop: true, hitstun: true, reaction: Some(18.0), ..Sample::default() },
        f if f < 29 => Sample { hitstun: true, reaction: Some(18.0 - (f - 11) as f32), ..Sample::default() },
        _ => idle(),
    }
}

/// Jab → up tilt, a true combo by 3 (the up tilt pressed one frame late), then the CPU in the second hit's hitlag.
fn true_combo() -> Tracker {
    let mut t = Tracker::new();
    for f in 1..=26u32 {
        let (p1, inputs) = match f {
            1 | 2 | 20 => (idle(), NONE),
            3..=6 => (attack(Move::Jab, 0x27, f - 3), if f == 3 { A } else { NONE }),
            7..=10 => (Sample { stop: true, active: true, ..attack(Move::Jab, 0x27, 4) }, NONE),
            11..=19 => (Sample { active: f == 11, ..attack(Move::Jab, 0x27, f - 6) }, NONE),
            21..=25 => (attack(Move::UpTilt, 0x2b, f - 21), if f == 21 { UP_A } else { NONE }),
            _ => (Sample { stop: true, active: true, ..attack(Move::UpTilt, 0x2b, 5) }, NONE),
        };
        let cpu = if f == 26 { Sample { stop: true, hitstun: true, reaction: Some(30.0), ..Sample::default() } } else { cpu(f) };
        t.step(p1, cpu, inputs);
    }
    t
}

fn directional_combo(from: Move, to: Move, from_angle: Option<crate::model::frame_strip::Angle>,
                     to_angle: Option<crate::model::frame_strip::Angle>) -> Tracker {
    let mut tracker = Tracker::new();
    for frame in 1..=26u32 {
        let p1 = match frame {
            1 | 2 | 20 => idle(),
            3..=19 => Sample {
                angle: from_angle,
                active: frame == 7,
                ..attack(from, 1, frame - 3)
            },
            _ => Sample {
                angle: to_angle,
                active: frame == 26,
                ..attack(to, if from == to { 1 } else { 2 }, frame - 21)
            },
        };
        let cpu = match frame {
            7 | 26 => Sample { stop: true, hitstun: true, reaction: Some(30.0), ..Sample::default() },
            8..=25 => Sample { hitstun: true, reaction: Some(30.0), ..Sample::default() },
            _ => idle(),
        };
        tracker.step(p1, cpu, NONE);
    }
    tracker
}

#[test]
fn the_lane_draws_each_mark_on_its_frame_in_a_label_box() {
    use crate::model::frame_strip::{Aim, Angle};
    for (active, color) in [(false, INK), (true, TEXT)] {
        let mut tracker = Tracker::new();
        tracker.step(idle(), idle(), NONE);
        tracker.step(Sample { angle: Some(Angle::Up), active, ..attack(Move::SideTilt, 1, 0) }, idle(), NONE);
        assert_eq!(aims_shown(&tracker), [Some(Aim { frame: 2, dir: 2 }), None]);
        let (_, seen) = draw_all(&mut Renderer::new(), &tracker, true);
        let mark = look_at(&seen, at::RUN_LABEL);
        assert!(mark.visible);
        assert_eq!(mark.text, Text::new("↑"));
        assert_eq!((mark.x, mark.y, mark.w, mark.rot),
                   (C.strip.lane_run(0, WINDOW - 1, 1, true).x,
                    C.strip.lane_run(0, WINDOW - 1, 1, true).y,
                    C.strip.lane_run(0, WINDOW - 1, 1, true).w,
                    C.strip.lane_run(0, WINDOW - 1, 1, true).rot));
        assert_eq!(mark.color, color);
        assert!(!look_at(&seen, at::RUN_LABEL + RUN_LABELS).visible);
        for _ in 0..WINDOW {
            tracker.step(Sample::default(), Sample::default(), NONE);
        }
        assert_eq!(aims_shown(&tracker), [None; 2]);
    }
}

#[test]
fn marks_take_two_label_boxes_and_lengths_keep_the_rest() {
    use crate::model::frame_strip::Angle;
    let mut tracker = Tracker::new();
    for (index, mv, angle, recovery) in [
        (0, Move::SideTilt, Angle::Up, 0),
        (1, Move::SideSmash, Angle::Down, 8),
        (2, Move::SideTilt, Angle::Up, 8),
    ] {
        tracker.step(Sample { angle: Some(angle), active: true, ..attack(mv, index, 0) }, idle(), NONE);
        for frame in 1..=recovery {
            tracker.step(Sample { angle: Some(angle), ..attack(mv, index, frame) }, idle(), NONE);
        }
    }
    let (_, seen) = draw_all(&mut Renderer::new(), &tracker, true);
    let texts: Vec<_> = (0..4).map(|n| String::from_utf16(look_at(&seen, at::RUN_LABEL + n).text.units()).unwrap()).collect();
    assert_eq!(texts, ["↑", "↓", "8", "8"]);
}

#[test]
fn the_bridge_reports_only_the_marks_whose_label_box_exists() {
    use crate::model::frame_strip::Angle;
    let mut tracker = Tracker::new();
    tracker.step(Sample { angle: Some(Angle::Down), active: true, ..attack(Move::SideSmash, 1, 0) }, idle(), NONE);
    tracker.step(Sample { angle: Some(Angle::Up), active: true, ..attack(Move::SideTilt, 2, 0) }, idle(), NONE);
    let (_, seen) = draw_all(&mut Renderer::new(), &tracker, true);
    assert_eq!(look_at(&seen, at::RUN_LABEL).text, Text::new("↑"), "the newest mark takes the first box");
    assert_eq!(look_at(&seen, at::RUN_LABEL + 1).text, Text::new("↓"));
    assert_eq!(aims_drawn(&tracker, [true, true]), [Some(6), Some(2)]);
    assert_eq!(aims_drawn(&tracker, [false, true]), [Some(6), None], "the newest mark's box is missing");
    assert_eq!(aims_drawn(&tracker, [true, false]), [Some(2), None], "the older mark's box is missing");
    assert_eq!(aims_drawn(&tracker, [false, false]), [None; 2]);
}

#[test]
fn a_diagonal_is_written_as_its_two_cardinal_arrows() {
    assert_eq!((0..8).map(arrow).collect::<Vec<_>>(), ["→", "↑→", "↑", "↑←", "←", "↓←", "↓", "↓→"]);
    assert!((0..8).flat_map(|dir| arrow(dir).chars()).all(|c| ('\u{2190}'..='\u{2193}').contains(&c)));
}

#[test]
fn the_last_row_and_the_link_line_name_the_direction_beside_the_move() {
    use crate::model::frame_strip::{Angle, Shown};
    let tracker = directional_combo(Move::Jab, Move::SideTilt, None, Some(Angle::Up));
    let (_, seen) = draw_all(&mut Renderer::new(), &tracker, true);
    assert_eq!(String::from_utf16(look_at(&seen, at::MOVE_TEXT).text.units()).unwrap(), "Jab > Side tilt ↑");
    let link = tracker.latest_link().unwrap();
    assert_eq!(String::from_utf16(folded_link(&link).units()).unwrap(), "TRUE · Jab > Side tilt ↑");
    let (_, line, _) = Renderer::new().draw_with_spec(
        &tracker, StripDraw { shown: true, spots: false, parry: None, history_enabled: false, cells: false, spec: &C },
        |_, _, _| {},
    );
    assert_eq!(String::from_utf16(line.unwrap().units()).unwrap(), "TRUE · Jab > Side tilt ↑");
    let special = Link { to: Move::Other, shown: [Shown::default(), Shown { of: Some(Move::UpSpecial), dirs: [Some(4), Some(0)] }], ..link };
    assert_eq!(String::from_utf16(link_moves(&special, SHORT_UNITS).units()).unwrap(), "Jab > Up B ← →");
    let named = Link { to: Move::SideTilt, shown: [Shown::default(), Shown { of: Some(Move::UpSpecial), dirs: [Some(2), None] }], ..link };
    assert_eq!(String::from_utf16(link_moves(&named, SHORT_UNITS).units()).unwrap(), "Jab > Side tilt ↑");
    let plain = Link { shown: [Shown::default(); 2], ..link };
    assert_eq!(String::from_utf16(link_moves(&plain, SHORT_UNITS).units()).unwrap(), "Jab > Side tilt");
}

#[test]
fn a_long_pair_of_moves_drops_its_spaces_to_fit() {
    use crate::model::frame_strip::{Angle, Shown};
    let tracker = directional_combo(Move::SideSmash, Move::SideSmash, Some(Angle::Down), Some(Angle::Down));
    let (_, seen) = draw_all(&mut Renderer::new(), &tracker, true);
    assert_eq!(String::from_utf16(look_at(&seen, at::MOVE_TEXT).text.units()).unwrap(), "Side smash↓>Side smash↓");
    let link = tracker.latest_link().unwrap();
    assert_eq!(String::from_utf16(folded_link(&link).units()).unwrap(), "TRUE · Side smash↓>Side smash↓");
    let short = Link { from: Move::Jab, shown: [Shown::default(), Shown { of: None, dirs: [Some(6), None] }], ..link };
    assert_eq!(String::from_utf16(folded_link(&short).units()).unwrap(), "TRUE · Jab > Side smash ↓");
}

#[test]
fn runtime_and_archive_share_the_strip_layout() {
    use crate::model::panes::{IDX_IV, strip_pane_spec};
    let tracker = true_combo();
    let mut alternate = C;
    alternate.strip.origin_y -= 300.0;
    alternate.strip.width -= 40.0;
    alternate.strip.base_height += 20.0;
    let collect = |spec: &LayoutSpec| {
        let mut looks = [Look::HIDDEN; COUNT];
        let (_, line, history) = Renderer::new().draw_with_spec(
            &tracker, StripDraw { shown: true, spots: true, parry: None, history_enabled: true, cells: false, spec },
            |i, look, _| looks[i] = *look,
        );
        let root = strip_pane_spec(spec, IDX_IV + at::CONTAINER);
        assert_eq!((root.x, root.y), (looks[at::CONTAINER].x, looks[at::CONTAINER].y));
        for i in [at::BACKDROP, at::SEPARATOR + 1, at::READ_LABEL + 1, at::READ_SUB + 1] {
            let pane = strip_pane_spec(spec, IDX_IV + i);
            let look = looks[i];
            assert_eq!((pane.x, pane.y, pane.w), (look.x, look.y, look.w));
        }
        let rule = looks[at::SEPARATOR + 1];
        assert!(rule.visible && rule.x.abs() + rule.w / 2.0 <= spec.strip.width / 2.0);
        (looks, line, history)
    };
    let (compact, line, history) = collect(&C);
    let (other, other_line, other_history) = collect(&alternate);
    assert_ne!((compact[0].y, compact[1].w), (other[0].y, other[1].w));
    assert_ne!(compact[at::READ_SUB + 1].x, other[at::READ_SUB + 1].x);
    assert_eq!(compact.map(|l| (l.visible, l.color, l.text)), other.map(|l| (l.visible, l.color, l.text)));
    assert_eq!((line, history), (other_line, other_history));
}

#[test]
fn link_history_hides_the_last_row_and_folds_the_newest_moves() {
    let tracker = true_combo();
    let mut renderer = Renderer::new();
    let mut draw = |on| {
        let (_, line, history) = renderer.draw_with_spec(
            &tracker, StripDraw { shown: true, spots: false, parry: None, history_enabled: on, cells: false, spec: &C }, |_, _, _| {},
        );
        let last = renderer.written(at::LAST_TEXT).unwrap().visible;
        let pill = renderer.written(at::PILL).unwrap().visible;
        assert_eq!((history, last, pill), (on, on, on));
        String::from_utf16(line.unwrap().units()).unwrap()
    };
    assert_eq!(draw(true), "TRUE combo");
    assert_eq!(draw(false), "TRUE · Jab > Up tilt");
    assert_eq!(draw(true), "TRUE combo");
}

#[test]
fn last_row_moves_keep_a_gap_from_their_pill() {
    use crate::model::panes::{iv, IDX_IV, strip_pane_spec};
    for n in 0..iv::HISTORY {
        let pill = C.strip.last(0, n);
        let moves = C.strip.last(2, n);
        let (left, right) = (moves.x - moves.w / 2.0, moves.x + moves.w / 2.0);
        assert_eq!((left - (pill.x + pill.w / 2.0), moves.w), (8.0, 192.0));
        let (bound, clear) = if n + 1 < iv::HISTORY {
            let next = C.strip.last(0, n + 1);
            (next.x - next.w / 2.0, 4.0)
        } else {
            (C.strip.width / 2.0, 16.0)
        };
        assert_eq!(bound - right, clear);
        let pane = strip_pane_spec(&C, IDX_IV + at::MOVE_TEXT + n);
        assert_eq!((pane.x, pane.w), (moves.x, moves.w));
    }
}

#[test]
fn compact_bounds_and_fade_follow_the_scene_and_visible_rows() {
    assert_eq!(C.strip.bounds(true, false), [0.0, 23.0, 1296.0, 170.0]);
    assert_eq!(C.strip.bounds(false, false), [0.0, 39.0, 1296.0, 138.0]);
    assert_eq!(C.strip.bounds(false, true), [0.0, 7.0, 1296.0, 202.0]);
    let seen = C.opacity(0.0, false, Some((true, 1)), [[0.0, 535.0]]);
    assert_eq!(seen, u8::MAX);
    assert_eq!(
        C.opacity(0.0, false, Some((true, 1)), [[0.0, 500.0]]),
        C.covered_alpha
    );
    assert_eq!(
        C.opacity(0.0, false, Some((true, 1)), [[804.0, 400.0]]),
        C.covered_alpha
    );
    assert_eq!(C.opacity(0.0, false, None, [[804.0, 400.0]]), u8::MAX);
    assert_eq!(
        C.opacity(C.scene_drop(true), false, Some((true, 1)), [[804.0, 500.0]]),
        u8::MAX
    );
    assert_eq!(
        C.opacity(C.scene_drop(true), false, Some((true, 1)), [[804.0, 400.0]]),
        C.covered_alpha
    );
}

fn draw_all(r: &mut Renderer, t: &Tracker, shown: bool) -> (usize, Vec<(usize, Look, Changed)>) {
    let mut seen = Vec::new();
    let n = r.draw(t, shown, |i, look, changed| seen.push((i, *look, changed)));
    (n, seen)
}

fn hidden_writes_only_the_container_once_and_showing_again_writes_only_it() {
    let t = true_combo();
    let mut r = Renderer::new();
    draw_all(&mut r, &t, true);
    let (n, seen) = draw_all(&mut r, &t, false);
    assert_eq!(n, 1);
    assert_eq!(seen[0].0, at::CONTAINER);
    assert!(!seen[0].1.visible);
    assert_eq!(draw_all(&mut r, &t, false).0, 0, "hidden stays quiet");
    let (n, seen) = draw_all(&mut r, &t, true);
    assert_eq!((n, seen[0].0), (1, at::CONTAINER), "nothing else changed while hidden");
}

#[test]
fn clip_editor_hides_and_restores_both_hud_renderers() {
    use crate::model::{combo, combo_plan, frame_strip::Button, input_delay::Owners, flow::{self, Phase, Seen, Step}};

    for bits in 0..64 {
        let owners = Owners::from_bits(bits);
        assert_eq!(owners.covers_hud(), owners.covers_strip() || owners.contains(Owners::REPLAY_EDITOR));
    }
    assert!(!Owners::REPLAY_EDITOR.covers_strip(), "keep gameplay clocks and camera unchanged");
    for input_only in [false, true] {
        for entry in [Phase::Menu, Phase::Panel] {
            let tracker = true_combo();
            let link = combo::LinkRec::presses(Move::Jab, Move::UpTilt, Verdict::True(None), None,
                &[combo::Event::press(Button::Attack, combo::Aim::Neutral, 0)]);
            let mut target = combo::Combo::from_links([link]).unwrap();
            target.input_only = input_only;
            let mut helper = combo::Helper::new();
            helper.set_target(Some(target));
            helper.step(&tracker, combo::Settings { on: true, ..combo::Settings::factory() });
            let mut strip = Renderer::new();
            let mut bar = combo_plan::Renderer::new();
            let mut visible = [false; 2];
            let mut draw = |owners: Owners| {
                let shown = !owners.covers_hud();
                strip.draw(&tracker, shown, |i, look, _| {
                    if i == at::CONTAINER { visible[0] = look.visible; }
                });
                bar.draw(&helper, &Text::new("Target"), None, shown, (&C, false), |i, look, _| {
                    if i == combo_plan::at::CONTAINER { visible[1] = look.visible; }
                });
                visible
            };
            assert_eq!(draw(Owners::NONE), [true; 2]);
            let clip = Seen { menu_open: true, panel_visible: true, clip_visible: true,
                clip_foreground: true, prompt_visible: false };
            let (phase, step) = flow::next(entry, clip);
            assert_eq!((phase, step), (Phase::Clip, Step::HandOverToClip));
            // sync_owner retains REPLAY_EDITOR while the editor exists, even after the menu drops.
            assert_eq!(draw(Owners::REPLAY_EDITOR), [false; 2]);
            // Saving opens the keyboard without closing the clip; neither HUD may redraw over it on return.
            let naming = Seen { menu_open: false, clip_foreground: false, ..clip };
            assert_eq!(flow::next(phase, naming), (Phase::Clip, Step::Nothing));
            assert_eq!(draw(Owners::REPLAY_EDITOR), [false; 2]);
            assert_eq!(flow::next(phase, Seen { clip_foreground: true, ..naming }), (Phase::Clip, Step::Nothing));
            assert_eq!(draw(Owners::REPLAY_EDITOR), [false; 2]);
            assert_eq!(flow::next(phase, Seen::default()), (Phase::Menu, Step::HideClip));
            assert_eq!(draw(Owners::NONE), [true; 2], "B/close restores both cached containers");
        }
    }
}

fn input_lines() {
    let base = true_combo().latest_link().unwrap();
    let mut l = base;
    l.input = -3;
    assert_eq!(String::from_utf16(input_line(&l).units()).unwrap(), "buffered 3f, 3 spare");
    l.input = 0;
    l.verdict = crate::model::frame_strip::Verdict::Drop(2);
    assert_eq!(String::from_utf16(input_line(&l).units()).unwrap(), "on time, 2f past");
    l.verdict = crate::model::frame_strip::Verdict::True(None);
    assert_eq!(String::from_utf16(input_line(&l).units()).unwrap(), "on time");
}

fn look_at(seen: &[(usize, Look, Changed)], i: usize) -> Look {
    seen.iter().find(|(at, ..)| *at == i).map(|(_, look, _)| *look).expect("pane written")
}

fn held(x: i8, y: i8) -> MappedInputs {
    MappedInputs { lstick_x: x, lstick_y: y, ..NONE }
}

fn grade_tag_text_and_colour() {
    use crate::model::spacing::{Grade, Grader};

    let grader = Grader::new();
    let mut tracker = Tracker::new();
    tracker.step_graded(idle(), idle(), NONE, grader.take());
    grader.finish(true);
    grader.target(Grade::Good);
    tracker.step_graded(idle(), idle(), NONE, grader.take());
    grader.finish(true);

    let tag_box = at::STICK + WINDOW;
    let tag_text = tag_box + crate::model::frame_strip::TAGS;
    let mut renderer = Renderer::new();
    let (_, seen) = draw_all(&mut renderer, &tracker, true);
    let box_look = look_at(&seen, tag_box);
    let text_look = look_at(&seen, tag_text);
    let x = FIRST_X + (WINDOW - 2) as f32 * PITCH + CELL_W / 2.0;
    assert_eq!((box_look.x, box_look.y, box_look.w, box_look.color), (x, P1_Y, 54.0, Grade::Good.color()));
    assert_eq!((text_look.x, text_look.y, text_look.color), (x, P1_Y, INK));
    assert_eq!(String::from_utf16(text_look.text.units()).unwrap(), "Good");

    for frame in 0..WINDOW {
        tracker.step(idle(), idle(), if frame % 2 == 0 { held(100, 0) } else { held(0, 100) });
    }
    let (_, offscreen) = draw_all(&mut renderer, &tracker, true);
    assert!(!look_at(&offscreen, tag_box).visible);
    assert!(!look_at(&offscreen, tag_text).visible);
}

fn the_stick_row_scrolls_and_freezes_with_the_strip() {
    use crate::model::frame_strip::FREEZE_AFTER;
    let mut t = Tracker::new();
    for frame in 1..=80u32 {
        let (x, y) = if frame % 2 == 0 { (100, 0) } else { (0, 100) };
        t.step(idle(), idle(), held(x, y));
    }
    assert!(!t.frozen(), "the freeze waits while the stick moves");
    assert_eq!(t.len(), WINDOW);
    let moving: Vec<_> = t.columns().map(|c| (c.frame, c.stick)).collect();
    assert_eq!(moving.first().copied(), Some((21, Some(2))));
    assert_eq!(moving.last().copied(), Some((80, Some(0))));
    for (frame, stick) in &moving {
        assert_eq!(*stick, if frame % 2 == 0 { Some(0) } else { Some(2) });
    }
    let mut r = Renderer::new();
    let (_, moving_draw) = draw_all(&mut r, &t, true);
    for (i, (_, stick)) in moving.iter().enumerate() {
        let look = look_at(&moving_draw, at::STICK + i);
        assert!(look.visible);
        assert_eq!(look.rot, if *stick == Some(0) { 0.0 } else { 90.0 });
    }
    for _ in 0..u32::from(FREEZE_AFTER) {
        t.step(idle(), idle(), held(100, 0));
    }
    assert!(t.frozen());
    let newest = t.newest();
    assert_eq!(newest, Some(80 + u32::from(FREEZE_AFTER) - 1));
    let held_row: Vec<_> = t.columns().map(|c| (c.frame, c.stick)).collect();
    let _ = draw_all(&mut r, &t, true);
    for _ in 0..5 {
        t.step(idle(), idle(), held(100, 0));
    }
    assert!(t.frozen());
    assert_eq!(t.newest(), newest, "a held direction does not scroll the row");
    assert_eq!(t.columns().map(|c| (c.frame, c.stick)).collect::<Vec<_>>(), held_row);
    let (n, _) = draw_all(&mut r, &t, true);
    assert_eq!(n, 0, "the frozen row is not rewritten");
    t.step(idle(), idle(), held(0, 100));
    assert!(!t.frozen());
    assert_eq!(t.columns().last().map(|c| c.stick), Some(Some(2)));
    let (_, moved) = draw_all(&mut r, &t, true);
    let look = look_at(&moved, at::STICK + WINDOW - 1);
    assert!(look.visible);
    assert_eq!(look.rot, 90.0);
}

/// Decision SP3: with the labels On, a hit block carries its spot, on the lane of the fighter that was hit; Off, nothing is written for it.
fn a_hit_block_carries_its_spot_label_when_the_row_is_on() {
    let calm = |damage| Sample { actionable: true, damage, ..Sample::default() };
    use crate::model::spots::{self, Hit};
    let mut powers = [Hit::default(); spots::SLOTS];
    (powers[0], powers[1]) = (Hit { damage: 13.0, growth: 100, fixed: 0, base: 0 }, Hit { damage: 18.0, growth: 100, fixed: 0, base: 0 });
    let swing = |stop| Sample { attack: Some(Move::Jab), status: 0x27, active: true, stop, powers, power_mul: 1.0, object_id: 7, ..Sample::default() };
    for (attacker_is_p1, slot, gained, label) in [(true, 1, 18.0 * 1.26, "Sweet"), (true, 0, 13.0 * 1.26, "Sour"), (false, 0, 13.0 * 1.26, "Sour")] {
        let mut t = Tracker::new();
        // The victim's damage record on the hit's first frame: the knockback the game stored and the attacker's slot (spots-3).
        // (the game's percent: the 20% before plus the hit's damage before the 1-on-1 ratio)
        let launched = spots::knockback(&powers[slot], gained / spots::TARGET_MUL, 20.0 + gained / spots::TARGET_MUL, 100.0);
        for f in 1..=8u32 {
            let victim = match f {
                1..=3 => calm(20.0),
                4 => Sample { stop: true, hitstun: true, damage: 20.0 + gained, launched, struck: Some((7, slot as u8)), ..Sample::default() },
                5..=7 => Sample { stop: true, hitstun: true, damage: 20.0 + gained, ..Sample::default() },
                _ => Sample { hitstun: true, damage: 20.0 + gained, ..Sample::default() },
            };
            let attacker = if f == 1 || f == 2 { calm(0.0) } else { swing((4..=7).contains(&f)) };
            let (p1, cpu) = if attacker_is_p1 { (attacker, victim) } else { (victim, attacker) };
            t.step(p1, cpu, NONE);
        }
        let lane = usize::from(attacker_is_p1);
        let mut on = Renderer::new();
        let mut seen = Vec::new();
        on.draw_spots(&t, true, true, |i, look, changed| seen.push((i, *look, changed)));
        let shown = look_at(&seen, at::RUN_LABEL + lane * RUN_LABELS);
        assert!(shown.visible, "lane {lane}");
        assert_eq!(String::from_utf16(shown.text.units()).unwrap(), label);
        assert_eq!(shown.x, FIRST_X + (f32::from(WINDOW as u8) - 8.0 + 3.0 + 2.0) * PITCH, "centred on the 4-frame hit block");
        assert!(!look_at(&seen, at::RUN_LABEL + (1 - lane) * RUN_LABELS).visible, "the other lane has no label");
        let mut off = Renderer::new();
        let mut seen = Vec::new();
        off.draw_spots(&t, true, false, |i, look, changed| seen.push((i, *look, changed)));
        assert!(!look_at(&seen, at::RUN_LABEL + lane * RUN_LABELS).visible, "Off draws today's strip");
    }
}

fn deferred_cached_looks_match_direct_visible_writes() {
    let mut direct = Panes::<1>::new();
    let mut deferred = Panes::<1>::new();
    let shown = Look::text(3.0, 4.0, Text::new("Step 40"), [1, 2, 3, 255]);
    for invalidate in [false, true] {
        if invalidate { direct.invalidate(); deferred.invalidate(); }
        for look in [shown, shown, Look::HIDDEN, Look::HIDDEN, shown] {
            let mut expected = Vec::new();
            let count = direct.show(|_| look, |i, look, change| expected.push((i, *look, change)));
            let mut changes = [None];
            assert_eq!(deferred.show(|_| look, |i, _, change| changes[i] = Some(change)), count);
            let actual: Vec<_> = changes.iter().enumerate().filter_map(|(i, c)| c.map(|c| (i, *deferred.written(i).unwrap(), c))).collect();
            assert_eq!(expected.len(), actual.len());
            for ((i, want, changed), (j, got, flags)) in expected.into_iter().zip(actual) {
                assert_eq!((i, changed, want.visible), (j, flags, got.visible));
                // Native apply returns after setting visibility when hidden; retained fields must not be applied then.
                if want.visible { assert_eq!(want, got); }
            }
        }
    }
}

/// rc.15 Parry practice: the PARRY and SET readouts replace LINK and INPUT, with Tech's words for the grade.
fn parry_strip_readouts_show_grade_and_counter() {
    use crate::model::parry::{Attempt, View};
    use crate::model::tech::session::Grade;
    let mut view = View::EMPTY;
    view.last[0] = Some(Attempt { grade: Grade::Early(3), hit: 1, window: None });
    view.set = Text::new("7/10");
    let mut looks = vec![Look::HIDDEN; COUNT];
    Renderer::new().draw_parry(&Tracker::new(), true, false, Some(&view), |i, look, _| looks[i] = *look);
    assert_eq!(looks[at::READ_LABEL + 1].text, Text::new("PARRY"));
    assert_eq!(looks[at::READ_LABEL + 2].text, Text::new("SET"));
    assert_eq!(looks[at::READ_VALUE + 1].text, Text::new("-3"));
    assert_eq!(looks[at::READ_SUB + 1].text, Text::new("Early by 3f"));
    assert_eq!(looks[at::READ_VALUE + 2].text, Text::new("7/10"));
    assert!(looks[at::PILL].visible, "the newest attempt's pill");
}

/// tr-review-astra-2 finding 2 (70bb16e9): with the strip already running, a parry window must be drawn on the strip's
/// own frames (the detector shares the strip's frame numbers).
fn parry_window_uses_the_strip_frames() {
    use crate::model::parry::{Detector, Frame, View};
    let mut tracker = Tracker::new();
    // Busy fighters (not actionable): the strip keeps scrolling instead of freezing on a quiet stretch.
    for _ in 0..100 {
        tracker.step(Sample::default(), Sample::default(), NONE);
    }
    assert!(tracker.newest().is_some_and(|n| n >= 100), "the strip ran ahead of Parry practice");
    let idle_p1 = Frame { armed: true, ..Frame::default() };
    let held = Frame { shield_held: true, ..idle_p1 };
    let off = Frame { guard_off: true, just_frame: Some(4), ..idle_p1 };
    let parry = Frame { stop: true, guard_damage: true, just_shield: Some(true), ..idle_p1 };
    let mut detector = Detector::new();
    let mut view = View::EMPTY;
    for f in [held, held, off, off, parry] {
        tracker.step(Sample::default(), Sample::default(), NONE);
        for a in detector.step(&f, tracker.newest().unwrap()).into_iter().flatten() {
            view.last[0] = Some(a);
        }
    }
    assert!(view.last[0].is_some(), "the parry is graded");
    let mut looks = vec![Look::HIDDEN; COUNT];
    Renderer::new().draw_parry(&tracker, true, false, Some(&view), |i, look, _| looks[i] = *look);
    assert!(looks[at::WINDOW_BAR].visible, "the window bar is on the strip's frames");
}

/// Parry practice's readout line carries Tech's words for the grade, whole.
#[test]
fn parrys_grade_words_are_shown_in_full() {
    use crate::model::parry::{Attempt, View};
    use crate::model::tech::session::Grade;
    let mut view = View::EMPTY;
    view.last[0] = Some(Attempt { grade: Grade::Unavailable, hit: 1, window: None });
    let mut looks = vec![Look::HIDDEN; COUNT];
    let (_, line, _) = Renderer::new().draw_with_spec(
        &Tracker::new(),
        StripDraw { shown: true, spots: false, parry: Some(&view), history_enabled: true, cells: false, spec: &C },
        |i, look, _| looks[i] = *look,
    );
    let words = "Grading unavailable (not counted)";
    assert_eq!(String::from_utf16(looks[at::READ_SUB + 1].text.units()).unwrap(), words);
    assert_eq!(String::from_utf16(line.unwrap().units()).unwrap(), words);
}

/// One length for a HUD line: every text box of the strip and of the combo card holds what a plan's `Text` holds.
/// The two-unit boxes are single glyphs.
#[test]
fn the_hud_text_boxes_are_as_long_as_the_plans_text() {
    use crate::model::panes::{cb, iv, spec, Kind, IDX_CB, IDX_IV};
    assert_eq!((usize::from(iv::TEXT_UNITS), usize::from(cb::TEXT_UNITS)), (TEXT_UNITS, TEXT_UNITS));
    let lengths: Vec<u16> = (IDX_IV..IDX_IV + iv::COUNT)
        .chain(IDX_CB..IDX_CB + cb::COUNT)
        .filter_map(|index| match spec(index).kind {
            Kind::Text { chars, .. } => Some(chars),
            _ => None,
        })
        .collect();
    assert!(lengths.iter().all(|&chars| usize::from(chars) == TEXT_UNITS || chars == 2), "{lengths:?}");
    assert_eq!(lengths.iter().filter(|&&chars| chars != 2).count(), 68, "41 lines of the strip and 27 of the card");
}

/// Lines whose place cannot grow keep 24 units: a name on the card, Parry's pills and the LAST row's moves.
#[test]
fn lines_that_cannot_grow_keep_their_24_units() {
    use crate::model::drill::{Advance, Stepped};
    use crate::model::{combo, combo_plan, frame_strip::Button};
    let long = "A thirty character combo name!";
    assert_eq!((long.len(), SHORT_UNITS), (30, 24));
    assert_eq!(Text::short(long), Text::new(&long[..SHORT_UNITS]));

    // A sequence's card: the name beside "landed", and a chip.
    let link = combo::LinkRec::presses(Move::Jab, Move::UpTilt, Verdict::True(None), None,
        &[combo::Event::press(Button::Attack, combo::Aim::Neutral, 0)]);
    let target = combo::Combo::from_links([link]).unwrap();
    let mut helper = combo::Helper::new();
    helper.set_target(Some(target));
    helper.step(&Tracker::new(), combo::Settings { on: true, ..combo::Settings::factory() });
    helper.continue_with(target);
    let clean = combo::Summary { outcome: combo::Outcome::Completed, graded: 1, true_links: 1, total: 1, clean: true, fault: None };
    let mut chips = [Text::EMPTY; crate::model::panes::cb::CHIPS];
    chips[0] = Text::new(long);
    let seq = combo_plan::SeqView {
        name: Text::short(long),
        at: 1,
        steps: 2,
        streak: 0,
        best: 0,
        first: 0,
        chips,
        last: Some(Stepped { step: 0, summary: clean, advance: Advance::Next(1) }),
        last_name: Text::new(long),
    };
    let mut card = vec![Look::HIDDEN; combo_plan::COUNT];
    combo_plan::Renderer::new().draw(&helper, &Text::EMPTY, Some(&seq), true, (&C, false), |i, look, _| card[i] = *look);
    let on_card = |i: usize| String::from_utf16(card[i].text.units()).unwrap();
    assert_eq!(on_card(combo_plan::at::SEQ_LANDED), long[..SHORT_UNITS]);
    assert_eq!(on_card(combo_plan::at::SEQ_CHIP_TEXT), format!("1  {}", &long[..SHORT_UNITS - 3]));

    // Parry's pill is 96 units wide: Tech's words keep their first 24 units there.
    use crate::model::parry::{Attempt, View};
    let mut view = View::EMPTY;
    view.last[0] = Some(Attempt { grade: crate::model::tech::session::Grade::Unavailable, hit: 1, window: None });
    let mut strip = vec![Look::HIDDEN; COUNT];
    Renderer::new().draw_parry(&Tracker::new(), true, false, Some(&view), |i, look, _| strip[i] = *look);
    assert_eq!(String::from_utf16(strip[at::PILL_TEXT].text.units()).unwrap(), "Grading unavailable (not");

    // The LAST row's moves are fitted to the gap before the next pill.
    let tracker = directional_combo(Move::DashAttack, Move::DashAttack, None, None);
    let (_, seen) = draw_all(&mut Renderer::new(), &tracker, true);
    assert_eq!(String::from_utf16(look_at(&seen, at::MOVE_TEXT).text.units()).unwrap(), "Dash attack>Dash attack");
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("hidden_writes_only_the_container_once_and_showing_again_writes_only_it", hidden_writes_only_the_container_once_and_showing_again_writes_only_it as fn()),
        ("input_lines", input_lines as fn()),
        ("the_stick_row_scrolls_and_freezes_with_the_strip", the_stick_row_scrolls_and_freezes_with_the_strip as fn()),
        ("a_hit_block_carries_its_spot_label_when_the_row_is_on", a_hit_block_carries_its_spot_label_when_the_row_is_on as fn()),
        ("deferred_cached_looks_match_direct_visible_writes", deferred_cached_looks_match_direct_visible_writes as fn()),
        ("parry_strip_readouts_show_grade_and_counter", parry_strip_readouts_show_grade_and_counter as fn()),
        ("parry_window_uses_the_strip_frames", parry_window_uses_the_strip_frames as fn()),
        ("grade_tag_text_and_colour", grade_tag_text_and_colour as fn()),
    ]);
}
}
