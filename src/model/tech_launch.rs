//! S0 direct-launch reference. Reaction math is source-derived f32; flight is a provisional
//! zero-stick approximation, NOT native DI/integration parity. Runtime must compare 20 frames.
pub const DAMAGE_SPEED_MUL: f32 = 0.03;
pub const DAMAGE_AIR_BRAKE: f32 = 0.051;
pub const DAMAGE_FRAME_MUL: f32 = 0.4;
pub const DAMAGE_FRAME_MIN: f32 = 5.0;
pub const MAX_FRAMES: u16 = 600;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
impl Vec2 {
    fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
    fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

/// main+0x404ec0: one rounded f32 multiply, then max with the source parameter.
/// No percent/weight argument: KB already describes the selected direct launch.
pub fn reaction_frames(kb: f32, damage_frame_mul: f32, damage_frame_min: f32) -> Option<f32> {
    if ![kb, damage_frame_mul, damage_frame_min]
        .iter()
        .all(|v| v.is_finite() && *v >= 0.0)
    {
        return None;
    }
    let scaled = kb * damage_frame_mul;
    scaled.is_finite().then_some(scaled.max(damage_frame_min))
}

/// Degrees counterclockwise from right. Cardinal axes are exact; other angles use host trig
/// and make no native direction/DI bit-parity claim.
pub fn direction_degrees(degrees: f32) -> Option<Vec2> {
    if !degrees.is_finite() {
        return None;
    }
    let angle = degrees.rem_euclid(360.0);
    Some(match angle {
        0.0 => Vec2 { x: 1.0, y: 0.0 },
        90.0 => Vec2 { x: 0.0, y: 1.0 },
        180.0 => Vec2 { x: -1.0, y: 0.0 },
        270.0 => Vec2 { x: 0.0, y: -1.0 },
        _ => {
            let (y, x) = angle.to_radians().sin_cos();
            Vec2 { x, y }
        }
    })
}

/// Caller supplies the native unit direction unchanged; no normalization or DI is invented.
/// Native order: scalar KB * .03, then each direction component * that rounded scalar.
pub fn launch_velocity(kb: f32, direction: Vec2) -> Option<Vec2> {
    let length = direction.length();
    if !kb.is_finite() || kb < 0.0 || !direction.finite() || !length.is_finite() || length == 0.0 {
        return None;
    }
    let speed = kb * DAMAGE_SPEED_MUL;
    let v = Vec2 {
        x: direction.x * speed,
        y: direction.y * speed,
    };
    v.finite().then_some(v)
}

#[derive(Clone, Copy, Debug)]
pub struct FlightParams {
    pub brake: f32,
    pub gravity: f32,
    pub fall_speed_cap: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flight {
    pub displacement: Vec2,
    /// Damage energy only (the native passive-speed check excludes gravity energy).
    pub damage_velocity: Vec2,
    /// Downward gravity speed, separate from damage energy.
    pub gravity_speed: f32,
}

/// Approximation convention: move with current damage and gravity speed, then decay damage
/// along its initial direction and accelerate gravity for the next step. `frames=0` is launch.
/// Fighter gravity/cap come from runtime parameters; this module supplies no fighter defaults.
pub fn flight(initial: Vec2, params: FlightParams, frames: u16) -> Option<Flight> {
    if !initial.finite()
        || frames > MAX_FRAMES
        || ![params.brake, params.gravity, params.fall_speed_cap]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0)
    {
        return None;
    }
    let mut speed = initial.length();
    if !speed.is_finite() {
        return None;
    }
    let direction = if speed == 0.0 {
        Vec2::default()
    } else {
        Vec2 {
            x: initial.x / speed,
            y: initial.y / speed,
        }
    };
    let mut state = Flight {
        damage_velocity: initial,
        ..Flight::default()
    };
    for _ in 0..frames {
        state.displacement.x += state.damage_velocity.x;
        state.displacement.y += state.damage_velocity.y - state.gravity_speed;
        speed = (speed - params.brake).max(0.0);
        state.damage_velocity = Vec2 {
            x: direction.x * speed,
            y: direction.y * speed,
        };
        state.gravity_speed = (state.gravity_speed + params.gravity).min(params.fall_speed_cap);
    }
    (state.displacement.finite()
        && state.damage_velocity.finite()
        && state.gravity_speed.is_finite())
    .then_some(state)
}

/// Translate the provisional path to the target. Normal points OUT of the solid; arrival must
/// point inward. Native geometry/blast-zone/earlier-contact checks belong to the runtime caller.
/// Contact must occur strictly inside reaction and strictly below the supplied passive limit.
pub fn solve_start(
    target: Vec2,
    initial: Vec2,
    params: FlightParams,
    frames: u16,
    reaction: f32,
    outward_normal: Vec2,
    passive_speed_limit: f32,
) -> Option<Vec2> {
    if !target.finite()
        || !outward_normal.finite()
        || outward_normal.length() == 0.0
        || frames == 0
        || !reaction.is_finite()
        || f32::from(frames) >= reaction
        || !passive_speed_limit.is_finite()
        || passive_speed_limit <= 0.0
    {
        return None;
    }
    let end = flight(initial, params, frames)?;
    let inward = end.damage_velocity.x * outward_normal.x
        + (end.damage_velocity.y - end.gravity_speed) * outward_normal.y;
    if inward >= 0.0 || !inward.is_finite() || end.damage_velocity.length() >= passive_speed_limit {
        return None;
    }
    let start = Vec2 {
        x: target.x - end.displacement.x,
        y: target.y - end.displacement.y,
    };
    start.finite().then_some(start)
}

/// The UI-to-core boundary. Percent is the desired displayed pre-flight percent;
/// hit_damage shapes hitlag and must not be added to percent by the adapter.
/// The core derives theta_rel, speed_vec, native hit height, hitlag and LR seed.
/// KB-to-speed conversion belongs exclusively to the core's native 0x403a20 call;
/// the approximate host solver does not supply a speed vector to this interface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub start: Vec2,
    pub target: Vec2,
    /// WORLD angle in degrees; the core derives the hit-side-relative angle.
    pub angle: f32,
    pub knockback: f32,
    pub percent: f32,
    pub hit_damage: f32,
    pub height: HitHeight,
    pub facing: f32,
    pub hit_from_behind: bool,
    pub di: Option<Vec2>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitHeight {
    Low,
    Middle,
    High,
}

/// Native calls and guards belong to the core, which owns sleep/placement/release.
/// A rejected request does not begin or count an attempt.
pub trait Launcher {
    type Error;
    fn place(&mut self, launch: Launch) -> Result<(), Self::Error>;
    fn launch(&mut self) -> Result<(), Self::Error>;
    fn restore(&mut self);
}

impl Launch {
    /// Validate the whole request before holding or mutating the owned player.
    pub fn valid(self) -> bool {
        self.start.finite()
            && self.target.finite()
            && self.angle.is_finite()
            && self.knockback.is_finite()
            && self.knockback > 0.0
            && self.percent.is_finite()
            && self.percent >= 0.0
            && (1.0..=30.0).contains(&self.hit_damage)
            && matches!(self.facing, -1.0 | 1.0)
            && self
                .di
                .is_none_or(|v| v.finite() && v.x.abs() <= 1.0 && v.y.abs() <= 1.0)
    }
    /// Hit side is independent of world trajectory (including reverse-angle launches).
    pub fn reserve_damage_lr(self) -> f32 {
        if self.hit_from_behind {
            -self.facing
        } else {
            self.facing
        }
    }
    pub fn away_from_hit_toward_right(self) -> bool {
        self.reserve_damage_lr() < 0.0
    }
}
