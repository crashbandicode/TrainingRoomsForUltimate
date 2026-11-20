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
    /// True only for a launch whose start is a grounded floor-hit fixture.
    /// Air starts always use false, regardless of the player's saved pose.
    pub grounded_hit: bool,
    pub di: Option<Vec2>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitHeight {
    Low,
    Middle,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptState {
    Idle,
    Held,
    Stopped,
    Flying,
    Contact,
    Abandoned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Floor,
    Wall,
    Ceiling,
}

/// Observed at the native status-decision seam, before the original status change.
/// This is a contact request and passive-eligibility receipt, not proof of a successful tech.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactReceipt {
    pub surface: Surface,
    pub requested_status: i32,
    pub passive_enabled: bool,
    pub position: Vec2,
}

/// Exact pre-status native observations for the first contact of an owned attempt.
/// None is unavailable, never zero/default. Window inputs stay in native types;
/// the collector must not invent the game's conversion or press-frame calibration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecisionSnapshot {
    pub attempt: u64,
    pub contact: ContactReceipt,
    pub source_status: Option<i32>,
    /// First guarded post-request contact status observed different from known source.
    /// None is unconfirmed/unavailable, never a miss or requested-status success.
    pub entered_status: Option<i32>,
    pub guard_count: Option<u8>,
    pub previous_guard_gap: Option<u8>,
    /// None means the native event stream is unbound/unproven, never no press.
    pub guard_event: Option<bool>,
    pub window_base: Option<i32>,
    pub window_multiplier: Option<f32>,
    pub lockout: Option<i32>,
    pub reaction: Option<f32>,
    pub damage_fall: Option<bool>,
    pub stick_x: Option<f32>,
    /// Guarded native contact speed, with the surface and transition otherwise eligible.
    pub passive_speed: Option<f32>,
    pub press_frame_offset: Option<u16>,
}

/// Immutable pre-decision fields plus one actual entry, retained until observer acknowledgement.
/// Construct a fresh delivery with each placement; no previous attempt is reused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecisionDelivery(Option<DecisionSnapshot>);
impl DecisionDelivery {
    pub const EMPTY: Self = Self(None);
    pub fn capture_first(&mut self, mut snapshot: DecisionSnapshot) -> bool {
        if self.0.is_some() {
            return false;
        }
        snapshot.entered_status = None;
        self.0 = Some(snapshot);
        true
    }
    pub fn pending_decision(self) -> Option<DecisionSnapshot> {
        self.0
    }
    /// Confirm only this attempt from an actual native transition, never its request.
    pub fn observe_entered_status(
        &mut self,
        attempt: u64,
        actual: Option<i32>,
        contact_statuses: &[i32],
    ) -> bool {
        let Some(snapshot) = self.0.as_mut() else { return false };
        let Some((source, actual)) = snapshot.source_status.zip(actual) else { return false };
        if snapshot.attempt != attempt
            || snapshot.entered_status.is_some()
            || actual == source
            || !contact_statuses.contains(&actual)
        {
            return false;
        }
        snapshot.entered_status = Some(actual);
        true
    }
    pub fn confirmed_decision(self) -> Option<DecisionSnapshot> {
        self.0.filter(|snapshot| snapshot.entered_status.is_some())
    }
    pub fn acknowledge_decision(&mut self, attempt: u64) -> bool {
        if self.0.is_some_and(|snapshot| snapshot.attempt == attempt) {
            self.0 = None;
            true
        } else {
            false
        }
    }
}

/// Fresh per accepted placement. Missing contact stays missing until the native seam observes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attempt {
    pub id: u64,
    pub state: AttemptState,
    pub contact: Option<ContactReceipt>,
}
impl Attempt {
    pub const IDLE: Self = Self {
        id: 0,
        state: AttemptState::Idle,
        contact: None,
    };
    pub fn held(id: u64) -> Self {
        Self {
            id,
            state: AttemptState::Held,
            contact: None,
        }
    }
    pub fn launched(&mut self) {
        if self.state == AttemptState::Held {
            self.state = AttemptState::Stopped;
        }
    }
    pub fn observe_stop(&mut self, stopped: bool) {
        if self.state == AttemptState::Stopped && !stopped {
            self.state = AttemptState::Flying;
        }
    }
    pub fn record_contact(&mut self, receipt: ContactReceipt) {
        if matches!(self.state, AttemptState::Stopped | AttemptState::Flying)
            && receipt.position.finite()
        {
            self.contact = Some(receipt);
            self.state = AttemptState::Contact;
        }
    }
    pub fn mark_abandoned(&mut self) {
        if self.state != AttemptState::Idle {
            self.state = AttemptState::Abandoned;
        }
    }
}

/// Native calls and guards belong to the core, which owns sleep/placement/release.
/// A rejected request does not begin or count an attempt.
pub trait Launcher {
    type Error;
    fn place(&mut self, launch: Launch) -> Result<(), Self::Error>;
    fn launch(&mut self) -> Result<(), Self::Error>;
    /// Busy means no fresh observation was obtained; callers must not infer contact.
    fn attempt(&self) -> Result<Attempt, Self::Error>;
    fn restore(&mut self);
}

impl Launch {
    pub fn ground_to_air(self) -> bool {
        self.grounded_hit
    }
    /// Validate the whole request before holding or mutating the owned player.
    pub fn valid(self) -> bool {
        self.start.finite()
            && self.target.finite()
            && self.angle.is_finite()
            && self.knockback.is_finite()
            && self.knockback > 0.0
            && self.percent.is_finite()
            && self.percent >= 0.0
            && self.hit_damage.is_finite() && self.hit_damage > 0.0
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
