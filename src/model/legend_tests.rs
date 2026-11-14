use super::*;
use crate::model::hitbox::{self, layout, HandleSet, HurtboxReader, Sphere, KIND_GRAB, KIND_HIT, KIND_HURT};
use crate::model::hitbox_orbs::{self as orbs, Board};
use crate::model::spots::Spot;

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
}

#[test]
fn legend_part_status_is_the_games_max() {
    // (part word, group whole, group global) -> status, as `HitModule::get_status` (main+0x46a800) computes it: a spot
    // dodge's whole status, a respawn's global timer and a part set on its own each win over a normal part.
    let cases = [
        (0, 0, 0, 0),
        (0, -1, 0, 0),
        (0, INTANGIBLE, 0, INTANGIBLE),
        (0, 0, INVINCIBLE, INVINCIBLE),
        (INVINCIBLE, INTANGIBLE, 0, INTANGIBLE),
        (INTANGIBLE, 0, INVINCIBLE, INTANGIBLE),
        (3, 0, INTANGIBLE, 3),
        (0, -5, -1, 0),
    ];
    for (part, whole, global, want) in cases {
        assert_eq!(part_status(part, whole, global), want, "part {part} whole {whole} global {global}");
    }
}

#[test]
fn legend_hurtbox_status_colours() {
    assert_eq!(hurt_tint(0), [1.0, 1.0, 1.0], "normal: white");
    assert_eq!(hurt_tint(INVINCIBLE), [0.2, 1.0, 0.2], "invincible: green");
    assert_eq!(hurt_tint(INTANGIBLE), [0.45, 0.8, 1.0], "intangible: sky blue");
    assert_eq!(hurt_tint(OFF), [0.45, 0.8, 1.0], "switched off as a whole (the revival platform): nothing hits it, drawn as intangible");
    assert_eq!(hurt_tint(-1), [1.0, 1.0, 1.0]);
    assert!(![hurt_tint(INTANGIBLE), hurt_tint(INVINCIBLE), hurt_tint(0)].contains(&GRAB));
}

#[test]
fn legend_windboxes_keep_the_hit_colours() {
    // Eden hbl-trace-8fadea57-1 (the design report): three 0-damage hitboxes with knockback, by their collision attribute.
    // Ganondorf's up-tilt windbox is `collision_attr_normal`, F.L.U.D.D.'s water push `collision_attr_water`, Sing (no push,
    // sleep) `collision_attr_sleep`. The two windboxes differ where Sing differs too, so no rule over the attack data
    // separates them, and a 0-damage hitbox keeps the hit colours.
    let usual = [0.1, 0.2, 0.3];
    for spot in [None, Some(Spot::Sweet), Some(Spot::Sour), Some(Spot::Late)] {
        let on = hitbox::legend_colour(KIND_HIT, spot, 0, usual, true);
        assert!(on == hitbox::STRONG_TINT || on == hitbox::WEAK_TINT, "{spot:?}");
    }
}
fn hit(slot: usize) -> Orb {
    Orb { centre: [10.0, 5.0, 1.0], tail: [12.0, 5.0, 1.0], radius: 4.0, color: [255, 0, 0, 255], key: sphere_key(KIND_HIT, slot, 0) }
}

#[test]
fn legend_angle_line_points_along_the_angle_mirrored_by_facing() {
    let r = 4.0 * std::f32::consts::FRAC_1_SQRT_2;
    let cases = [
        (0, 1.0, [14.0, 5.0, 1.0]),
        (0, -1.0, [6.0, 5.0, 1.0]),
        (45, 1.0, [10.0 + r, 5.0 + r, 1.0]),
        (45, -1.0, [10.0 - r, 5.0 + r, 1.0]),
        (90, -1.0, [10.0, 9.0, 1.0]),
        (180, 1.0, [6.0, 5.0, 1.0]),
        (270, -1.0, [10.0, 1.0, 1.0]),
        (360, -1.0, [6.0, 5.0, 1.0]),
    ];
    for (angle, lr, end) in cases {
        let [Some(line), None] = marks(&hit(3), angle, lr) else { panic!("angle {angle}: one line") };
        assert!(close(line.centre, [10.0, 5.0, 1.0]) && close(line.tail, end), "angle {angle} lr {lr}: {line:?}");
        assert_eq!((line.key >> 24, (line.key >> 8) & 0xffff), (KIND_MARK, 3), "its own kind, the hitbox's slot");
        assert_ne!(line.key & 0xff, 1, "never paired as a capsule end");
        assert!(line.radius < hit(3).radius && line.color == [255; 4]);
    }
}

#[test]
fn legend_special_angles_are_dashed_or_a_dot() {
    // 0 and 360 are lines (the line test), 361 two dashes, 362 and above (and a negative) one dot.
    let shape = |angle| marks(&hit(0), angle, 1.0).map(|m| m.map(|o| (o.centre == o.tail, o.radius)));
    for angle in [0, 360] {
        assert!(matches!(shape(angle), [Some((false, _)), None]), "angle {angle}");
    }
    assert!(matches!(shape(361), [Some((false, _)), Some((false, _))]));
    for angle in [362, 363, 365, 366, 367, 368, 400, -1] {
        assert_eq!(shape(angle), [Some((true, 4.0 * DOT)), None], "angle {angle}");
    }
    assert_eq!(SAKURAI_SHOWN, 38.0, "the Eden measurement (design report)");
    // 361: two dashes along the shown direction, apart, inside the hitbox; mirrored by facing.
    for lr in [1.0, -1.0] {
        let [Some(a), Some(b)] = marks(&hit(0), 361, lr) else { panic!("two dashes") };
        let [dx, dy] = direction(SAKURAI_SHOWN, lr);
        let along = |p: [f32; 3]| ((p[0] - 10.0) * dx + (p[1] - 5.0) * dy) / 4.0;
        assert!(0.0 < along(a.centre) && along(a.tail) < along(b.centre) && along(b.tail) < 1.0, "{a:?} {b:?}");
        assert_ne!(a.key, b.key);
    }
    // Autolink: one dot on the centre, bigger than a line, smaller than the hitbox.
    let [Some(dot), None] = marks(&hit(0), 367, 1.0) else { panic!("one dot") };
    assert!(close(dot.centre, [10.0, 5.0, 1.0]) && close(dot.tail, dot.centre));
    assert!(dot.radius > 4.0 * LINE && dot.radius < 4.0);
}

#[test]
fn legend_grab_is_purple_while_frame_data_colours_is_on() {
    let colour = |on| hitbox::legend_colour(KIND_GRAB, None, 0, hitbox::GRAB_TINT, on);
    assert_eq!((colour(false), colour(true)), (hitbox::GRAB_TINT, GRAB), "Off: today's magenta; On: purple");
    // Purple is none of the On colours of hits and hurtboxes.
    for other in [hitbox::STRONG_TINT, hitbox::WEAK_TINT, HURT[0], HURT[1], HURT[2]] {
        assert!(GRAB.iter().zip(other).map(|(a, b)| (a - b).abs()).sum::<f32>() > 0.5, "{other:?}");
    }
}

#[test]
fn legend_off_keeps_todays_tints() {
    let usual = [0.1, 0.2, 0.3];
    for kind in [KIND_HIT, KIND_HURT, KIND_GRAB] {
        for spot in [None, Some(Spot::Sweet), Some(Spot::Sour), Some(Spot::Late)] {
            for status in [0, INVINCIBLE, INTANGIBLE, OFF] {
                assert_eq!(hitbox::legend_colour(kind, spot, status, usual, false), usual, "{kind} {spot:?} {status}");
            }
        }
    }
    // On: a hurtbox takes its status' colour.
    assert_eq!([0, INVINCIBLE, INTANGIBLE].map(|s| hitbox::legend_colour(KIND_HURT, None, s, usual, true)), HURT);
    // The hurt pass: Off every part is today's yellow and the group's words are not read; On each part takes the game's status.
    for (legend, whole, words, want) in [
        (false, INTANGIBLE, [0, 1], [hitbox::HURTBOX_TINT; 2]),
        (true, 0, [0, INVINCIBLE], [HURT[0], HURT[1]]),
        (true, INTANGIBLE, [0, INVINCIBLE], [HURT[2]; 2]),
        (true, -1, [INTANGIBLE, 0], [HURT[2], HURT[0]]),
    ] {
        let mut hurt = Hurt { whole, words, reads: Vec::new() };
        let (mut out, mut len) = ([Sphere::NONE; 4], 0);
        assert_eq!(hitbox::want_hurt(&mut hurt, &mut out, &mut len, legend), 0);
        assert_eq!([out[0].tint, out[1].tint], want, "legend {legend} whole {whole} words {words:?}");
        assert_eq!(hurt.reads.contains(&(GROUPS + GROUP_WHOLE)), legend, "the whole word is read only while On");
    }
}

const MODULE: usize = 0x100;
const GROUPS: usize = 0x200;
const PARTS: usize = 0x400;

/// Two hurt parts in group 0 with the given words; the group's whole status `whole`, global 0.
struct Hurt {
    whole: i32,
    words: [i32; 2],
    reads: Vec<usize>,
}

impl HurtboxReader for Hurt {
    fn module(&mut self) -> usize {
        MODULE
    }
    fn read_usize(&mut self, at: usize) -> usize {
        match at - MODULE {
            layout::HIT_GROUPS => GROUPS,
            layout::HIT_PARTS => PARTS,
            _ => 0,
        }
    }
    fn read_i32(&mut self, at: usize) -> i32 {
        self.reads.push(at);
        match at {
            _ if at == MODULE + layout::HIT_PART_TOTAL => 2,
            _ if at == GROUPS + GROUP_WHOLE => self.whole,
            _ => 0,
        }
    }
    fn read_i16(&mut self, at: usize) -> i16 {
        i16::from(at == GROUPS + layout::HIT_GROUP_COUNT) * 2
    }
    fn read_part(&mut self, at: usize, record: &mut [u8; layout::HIT_PART_STRIDE]) {
        let index = (at - PARTS) / layout::HIT_PART_STRIDE;
        *record = [0; layout::HIT_PART_STRIDE];
        record[..4].copy_from_slice(&self.words[index].to_le_bytes());
        record[layout::PART_SIZE..layout::PART_SIZE + 4].copy_from_slice(&2.0f32.to_le_bytes());
        record[layout::PART_OFFSET..layout::PART_OFFSET + 4].copy_from_slice(&(index as f32).to_le_bytes());
    }
}

#[test]
fn legend_marks_follow_their_hitbox_and_draw_last() {
    // A set holding a point hitbox (slot 2), a capsule hitbox (slot 5, two spheres) and a hurt sphere.
    let mut set = HandleSet::new();
    let sphere = |kind, index, end, x: f32| Sphere { key: sphere_key(kind, index, end), joint: 1, pos: [x, 0.0, 0.0], scale: 0.5, tint: [1.0; 3] };
    let wants = [sphere(KIND_HIT, 2, 0, 0.0), sphere(KIND_HIT, 5, 0, 10.0), sphere(KIND_HIT, 5, 1, 14.0), sphere(KIND_HURT, 0, 0, 20.0)];
    set.reconcile(&wants, true, &mut orbs::OrbSink { status: 1 });
    let angle = |slot| Some([0, 0, 45, 0, 0, 361][slot as usize]);
    let mut out = [orbs::Orb::NONE; orbs::BOARD_ORBS];
    // Off: the orbs only.
    assert_eq!(orbs::place_orbs(&set, |s| Some(s.pos), None, angle, &mut out), 4);
    // On: one line for slot 2, two dashes for slot 5 (from its first sphere), nothing for the second end or the hurt sphere.
    let n = orbs::place_orbs(&set, |s| Some(s.pos), Some(-1.0), angle, &mut out);
    assert_eq!(out[..n].iter().filter(|o| o.kind() == KIND_MARK).count(), 3);
    assert_eq!(out[..n].iter().filter(|o| o.kind() == KIND_MARK && (o.key >> 8) & 0xffff == 5).count(), 2);
    // Facing left, the 45-degree line goes up and to the left of its hitbox.
    let line = out[..n].iter().find(|o| o.kind() == KIND_MARK && (o.key >> 8) & 0xffff == 2).unwrap();
    assert!(line.tail[0] < line.centre[0] && line.tail[1] > line.centre[1]);
    // The board keeps them and draws them after every hitbox and hurtbox.
    let mut board = Board::new();
    board.publish(0x80, out[..n].iter().copied());
    let kinds: Vec<u32> = board.in_draw_order().map(|o| o.kind()).collect();
    assert_eq!(kinds, [KIND_HIT, KIND_HIT, KIND_HURT, KIND_MARK, KIND_MARK, KIND_MARK]);
}
#[test]
fn legend_checklist_items_draw_independently() {
    let bits = |row: &[u8]| shown(row);
    assert_eq!(bits(&[0; ITEMS]), 0);
    assert!(!draws(bits(&[0; ITEMS])), "nothing ticked draws nothing");
    assert!(!draws(bits(&[0, 0, 0, 1, 0])), "the colours alone recolour nothing");
    // Hurtboxes only.
    let hurt = bits(&[0, 0, 1, 0, 0]);
    assert!(draws(hurt) && shows(hurt, KIND_HURT));
    assert!([KIND_HIT, KIND_GRAB, KIND_MARK].iter().all(|k| !shows(hurt, *k)));
    // Hitboxes and the angles, no grab or hurt boxes.
    let lines = bits(&[1, 0, 0, 0, 1]);
    assert!(draws(lines) && shows(lines, KIND_HIT) && shows(lines, KIND_MARK));
    assert!(!shows(lines, KIND_GRAB) && !shows(lines, KIND_HURT));
    // The angles alone draw their lines without the discs; every tick has its own bit.
    assert!(draws(ANGLES) && shows(ANGLES, KIND_MARK) && !shows(ANGLES, KIND_HIT));
    assert_eq!(bits(&[1; ITEMS]), HITBOXES | GRABS | HURTBOXES | COLOURS | ANGLES);
}

#[test]
fn legend_old_hitbox_vis_migrates() {
    assert_eq!(migrate(&[1, 0]), Some([1, 1, 1, 0, 0]), "On: the three boxes, as before");
    assert_eq!(migrate(&[0, 1]), Some([0; ITEMS]), "Off: nothing ticked");
    assert_eq!(migrate(&[1, 0, 0, 0, 1]), None, "a checklist stays as it is");
    // Through the settings file: an older file's On comes back as the three boxes, its Off as none; the default is none.
    for (old, want) in [("[1, 0]", [1, 1, 1, 0, 0]), ("[0, 1]", [0; ITEMS]), ("[0, 1, 1, 0, 1]", [0, 1, 1, 0, 1])] {
        let file = format!("{{\"menu\": {{\"{ROW}\": {old}}}, \"defaults_menu\": {{\"{ROW}\": {old}}}}}");
        let loaded = crate::model::settings::parse_bytes(file.as_bytes());
        assert!(!loaded.corrupt(), "{old}: {:?}", loaded.issues);
        assert_eq!((loaded.settings.menu.get(ROW), loaded.settings.defaults.get(ROW)), (Some(&want[..]), Some(&want[..])), "{old}");
    }
    assert_eq!(crate::model::settings::Snapshot::factory().get(ROW), Some(&[0u8; ITEMS][..]));
}