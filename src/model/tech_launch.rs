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
