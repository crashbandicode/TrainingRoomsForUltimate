use super::*;
use crate::model::hitbox::KIND_HIT;

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
    assert_eq!(hurt_tint(3), [1.0, 1.0, 1.0]);
    assert_eq!(hurt_tint(-1), [1.0, 1.0, 1.0]);
    // The two blues stay apart, and purple is neither.
    assert_ne!(hurt_tint(INTANGIBLE), WIND);
    assert!(![WIND, hurt_tint(INTANGIBLE), hurt_tint(INVINCIBLE), hurt_tint(0)].contains(&GRAB));
}

#[test]
fn legend_windbox_rule_cases() {
    let (normal, sleep) = (hash40("collision_attr_normal"), hash40("collision_attr_sleep"));
    let cases = [
        (0.0, [100, 0, 30], normal, true, "no damage, pushes"),
        (0.0, [0, 50, 0], normal, true, "fixed knockback only"),
        (0.0, [0, 0, 0], normal, false, "no knockback"),
        (0.0, [100, 0, 30], sleep, false, "a sleep"),
        (3.0, [100, 0, 30], normal, false, "damage"),
    ];
    for (power, knockback, attribute, want, case) in cases {
        assert_eq!(windbox(power, knockback, attribute), want, "{case}");
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
    assert_eq!(mark(0), Mark::Line(0.0));
    assert_eq!(mark(360), Mark::Line(360.0));
    assert_eq!(mark(361), Mark::Dashes(SAKURAI_SHOWN));
    for angle in [362, 363, 365, 366, 367, 368, 400, -1] {
        assert_eq!(mark(angle), Mark::Dot, "angle {angle}");
    }
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
