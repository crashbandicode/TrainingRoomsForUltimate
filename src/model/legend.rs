//! The rest of Ultimate Frame Data's legend for Hitbox Visualization (rc.14; the spots lane owns strong and weak): grab boxes
//! and the hurtboxes' states while *Frame data colours* is On, and each hitbox's launch direction while *Launch angles* is On.
//! Windboxes keep the hit colours: the game marks no windbox, and no rule over its attack data held on Eden (the design
//! report). Pure and host-tested; the offsets and values are the owned 13.0.5 ELF's (`tests/core06_hitbox.rs`
//! `legend_facts_match_the_owned_elf`).

use super::hitbox::sphere_key;
use super::hitbox_orbs::Orb;

/// The *Launch angles* row (Display & Controls, under *Frame data colours*; Off by default).
pub const ROW_LAUNCH: &str = "launch_angles";

/// `AttackData`'s launch angle in degrees (i32, beside `hitbox::layout`).
pub const ANGLE: usize = 0x28;
/// `HitModule`'s group record: the whole status (`set_whole`) and the global status the xlu and invincible timers set.
pub const GROUP_WHOLE: usize = 0x1c;
pub const GROUP_GLOBAL: usize = 0x20;

/// `HitStatus`: 0 normal, 1 invincible (hit, no damage), 2 intangible (`xlu`, not hit), 3 off.
pub const INVINCIBLE: i32 = 1;
pub const INTANGIBLE: i32 = 2;
pub const OFF: i32 = 3;

/// Frame data colours On: grab boxes purple.
pub const GRAB: [f32; 3] = [0.6, 0.2, 1.0];
/// Hurtboxes: normal white, invincible green, intangible sky blue (by status).
pub const HURT: [[f32; 3]; 3] = [[1.0; 3], [0.2, 1.0, 0.2], [0.45, 0.8, 1.0]];

/// A part's status as the game reads it (`HitModule::get_status`, `main+0x46a800`): the highest of its own word, the
/// group's whole status (at least 0) and the group's global status.
pub fn part_status(part: i32, whole: i32, global: i32) -> i32 {
    part.max(global.max(whole.max(0)))
}

/// A hurtbox's tint for its status: invincible green; intangible sky blue, and so is a part the whole group switched off
/// (the revival platform: nothing hits it); any other white.
pub fn hurt_tint(status: i32) -> [f32; 3] {
    HURT[match status {
        INVINCIBLE => 1,
        INTANGIBLE | OFF => 2,
        _ => 0,
    }]
}

/// What a launch angle draws: a line at a fixed angle, dashes where the game picks the direction at hit time (361), a
/// centre dot where it links toward the attacker (362 and above) or the angle is not one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    Line(f32),
    Dashes(f32),
    Dot,
}

/// The direction 361's dashes point in, counter-clockwise from forward: the launch the game gave a target in the air, measured
/// on Eden (`hbl-trace-8fadea57-1` `t-hbl-361b`: 38.1 degrees, the first frame's fall taken from a 70-degree hit on the same CPU).
pub const SAKURAI_SHOWN: f32 = 38.0;

pub fn mark(angle: i32) -> Mark {
    match angle {
        0..=360 => Mark::Line(angle as f32),
        361 => Mark::Dashes(SAKURAI_SHOWN),
        _ => Mark::Dot,
    }
}

/// The unit vector of `degrees`, counter-clockwise from the forward of an attacker facing `lr` (+1 right, -1 left).
pub fn direction(degrees: f32, lr: f32) -> [f32; 2] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [if lr < 0.0 { -cos } else { cos }, sin]
}

/// The marks' orb kind (after `hitbox::KIND_GRAB`); their sphere numbers start at 0x10, so `Board::publish` never pairs them.
pub const KIND_MARK: u32 = 4;
/// A line's or dash's radius and a dot's, as fractions of the hitbox's.
pub const LINE: f32 = 0.1;
pub const DOT: f32 = 0.25;

/// The orbs marking hitbox orb `hit`'s launch angle `angle` (its owner facing `lr`), white: the line from the centre to the
/// edge, the two dashes, or the dot.
pub fn marks(hit: &Orb, angle: i32, lr: f32) -> [Option<Orb>; 2] {
    let index = ((hit.key >> 8) & 0xffff) as usize;
    let [x, y, z] = hit.centre;
    let seg = |n: usize, [dx, dy]: [f32; 2], from: f32, to: f32| {
        let at = |t: f32| [x + dx * hit.radius * t, y + dy * hit.radius * t, z];
        Orb { centre: at(from), tail: at(to), radius: hit.radius * LINE, color: [255; 4], key: sphere_key(KIND_MARK, index, 0x10 + n) }
    };
    match mark(angle) {
        Mark::Line(d) => [Some(seg(0, direction(d, lr), 0.0, 1.0)), None],
        Mark::Dashes(d) => [Some(seg(0, direction(d, lr), 0.1, 0.4)), Some(seg(1, direction(d, lr), 0.6, 0.9))],
        Mark::Dot => [Some(Orb { radius: hit.radius * DOT, ..seg(0, [0.0; 2], 0.0, 0.0) }), None],
    }
}

#[cfg(test)]
#[path = "legend_tests.rs"]
mod tests;
