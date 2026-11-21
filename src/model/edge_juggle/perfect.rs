//! Perfect-mode recovery and escape decisions. The runtime applies only pad inputs.

#[path = "perfect/diddy.rs"]
pub mod diddy;
#[path = "perfect/fly.rs"]
pub mod fly;
#[path = "perfect/game_watch.rs"]
pub mod game_watch;
#[path = "perfect/kart.rs"]
pub mod kart;
#[path = "perfect/peach.rs"]
pub mod peach;
#[path = "perfect/snake.rs"]
pub mod snake;
#[path = "perfect/link.rs"]
pub mod link;
#[path = "perfect/mario.rs"]
pub mod mario;
#[path = "perfect/niche.rs"]
pub mod niche;
#[path = "perfect/olimar.rs"]
pub mod olimar;
#[path = "perfect/pacman.rs"]
pub mod pacman;
#[path = "perfect/pichu.rs"]
pub mod pichu;
#[path = "perfect/pikachu.rs"]
pub mod pikachu;
#[path = "perfect/recovery.rs"]
pub mod recovery;
#[path = "perfect/region.rs"]
mod region;
#[path = "perfect/rob.rs"]
pub mod rob;
#[path = "perfect/rule_counts.rs"]
pub mod rule_counts;
#[path = "perfect/sheik.rs"]
pub mod sheik;
#[cfg(test)]
#[path = "perfect/sora.rs"]
mod sora;
#[path = "perfect/sora_flight.rs"]
pub mod sora_flight;
#[path = "perfect/wings.rs"]
pub mod wings;

pub type Point = [f32; 2];
pub type Segment = [Point; 2];

/// A complete, bounded walk around the main solid stage component.
#[derive(Clone, Copy)]
pub struct Outline {
    segments: [Segment; 64],
    len: usize,
}

impl Outline {
    pub const fn new() -> Self {
        Self {
            segments: [[[0.0; 2]; 2]; 64],
            len: 0,
        }
    }

    pub fn add(&mut self, segment: Segment) {
        if let Some(slot) = self.segments.get_mut(self.len) {
            *slot = segment;
            self.len += 1;
        }
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments[..self.len]
    }
}

impl Default for Outline {
    fn default() -> Self {
        Self::new()
    }
}

/// Endpoints of the current connected solid floor, including its slope.
pub fn floor_ledges(floors: impl IntoIterator<Item = Segment>) -> Option<[Point; 2]> {
    let mut points = floors.into_iter().flatten();
    let first = points.next()?;
    let [mut left, mut right] = [first; 2];
    for point in std::iter::once(first).chain(points) {
        if !point.into_iter().all(f32::is_finite) {
            return None;
        }
        if point[0] < left[0] {
            left = point;
        }
        if point[0] > right[0] {
            right = point;
        }
    }
    (left[0] < right[0]).then_some([left, right])
}

/// Seed the solid outline walk at the stage center through the full blast zone.
pub fn outline_probe(camera: [f32; 4], blast: [f32; 4]) -> Option<(f32, Point)> {
    let [cam_left, cam_right, _, _] = camera;
    let [left, right, top, bottom] = blast;
    let center = cam_left.midpoint(cam_right);
    (blast.into_iter().chain([center]).all(f32::is_finite)
        && left < center
        && center < right
        && top > bottom)
        .then_some((center, [top, bottom]))
}

/// The foot heights that leave enough room below a closed main-stage outline.
/// The caller passes live segments from one walk in the current stage.
fn corridor(
    outline: &[Segment],
    ledges: [Point; 2],
    blast_bottom: f32,
    body_clearance: f32,
    fall_budget: f32,
) -> Option<[f32; 2]> {
    let [[left, left_y], [right, right_y]] = ledges;
    if ![
        left,
        left_y,
        right,
        right_y,
        blast_bottom,
        body_clearance,
        fall_budget,
    ]
    .into_iter()
    .all(f32::is_finite)
        || left >= right
        || body_clearance <= 0.0
        || fall_budget < 0.0
        || outline.is_empty()
    {
        return None;
    }
    let mut underside = left_y.min(right_y);
    for &[a, b] in outline {
        if !a.into_iter().chain(b).all(f32::is_finite) {
            return None;
        }
        let ([x0, y0], [x1, y1]) = if a[0] <= b[0] { (a, b) } else { (b, a) };
        let start = x0.max(left);
        let end = x1.min(right);
        if start <= end {
            let low = if x0 == x1 {
                y0.min(y1)
            } else {
                let at = |x| y0 + (y1 - y0) * ((x - x0) / (x1 - x0));
                at(start).min(at(end))
            };
            underside = underside.min(low);
        }
    }
    let low = blast_bottom + fall_budget;
    let high = underside - body_clearance;
    (low.is_finite() && high.is_finite() && low < high).then_some([low, high])
}

/// Native fighter parameters in world units and frames.
#[derive(Clone, Copy, Debug)]
pub struct AirPhysics {
    pub horizontal_speed: f32,
    pub horizontal_acceleration: f32,
    pub gravity: f32,
    pub terminal_speed: f32,
    pub jump_speed: f32,
}

/// The `fighter_param` fields the air model is built from, as the game stores them.
#[derive(Clone, Copy)]
pub struct NativeAir {
    pub air_speed_x_stable: f32,
    pub air_accel_x_add: f32,
    pub air_accel_x_mul: f32,
    pub air_accel_y: f32,
    pub air_speed_y_stable: f32,
    pub jump_aerial_y: f32,
}

/// Rejected input identifiers shared with the release planner trace.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanInput {
    PlayerHeight = 1,
    Snapshot = 2,
    SpecialEnvelope = 3,
    StageOutline = 4,
    FighterGeometry = 5,
    AirPhysics = 6,
    Velocity = 7,
}

#[cfg(test)]
mod fly_jump_tests {
    lib_unit_tests! {
    use super::{AirPhysics, NativeAir};

    #[test]
    fn fly_aerial_jump_uses_the_next_native_launch_entry() {
        let puff = AirPhysics::from_params(NativeAir {
            air_speed_x_stable: 1.332,
            air_accel_x_add: 0.05,
            air_accel_x_mul: 0.08,
            air_accel_y: 0.053,
            air_speed_y_stable: 0.98,
            jump_aerial_y: 20.0,
        });
        for (fighter, gravity, table) in [
            (0xc, 0.053, &[1.65, 1.59, 1.47, 1.36, 1.25][..]),
            (0x1d, 0.110, &[2.2, 2.1, 2.0, 1.88, 1.75][..]),
            (0x1e, 0.081, &[2.0, 1.8, 1.6][..]),
            (0x1f, 0.081, &[2.0, 1.8, 1.6][..]),
        ] {
            let air = AirPhysics { gravity, ..puff };
            for (i, speed) in table.iter().enumerate() {
                let next = air.with_fly_jump(fighter, Some((table.len() - i) as u8));
                assert!((next.jump_speed - (speed - gravity)).abs() < 0.001);
            }
            for left in [None, Some(0), Some(9)] {
                assert_eq!(air.with_fly_jump(fighter, left).jump_speed, air.jump_speed);
            }
        }
        assert_eq!(puff.with_fly_jump(0, Some(1)).jump_speed, puff.jump_speed);
    }
    }
}

/// Native 13.0.5 `vl.prc fly_data[0].speed_y_table`, verified against the aerial jump
/// movement in edge-niche-cand-fd-4. Fighter kinds come from the choice catalog.
fn fly_launch(fighter: i32, remaining: u8) -> Option<f32> {
    let table: &[f32] = match fighter {
        0xc => &[1.65, 1.59, 1.47, 1.36, 1.25],
        0x1d => &[2.2, 2.1, 2.0, 1.88, 1.75],
        0x1e | 0x1f => &[2.0, 1.8, 1.6],
        _ => return None,
    };
    table.get(table.len().checked_sub(usize::from(remaining))?).copied()
}

impl AirPhysics {
    pub fn with_fly_jump(mut self, fighter: i32, remaining: Option<u8>) -> Self {
        if let Some(speed) = remaining.and_then(|left| fly_launch(fighter, left)) {
            self.jump_speed = speed - self.gravity;
        }
        self
    }

    pub fn plan_velocity(self, velocity: Option<(f32, f32)>) -> Result<Point, PlanInput> {
        if !self.valid() {
            return Err(PlanInput::AirPhysics);
        }
        velocity
            .map(|(x, y)| [x, y])
            .filter(|v| v.iter().all(|component| component.is_finite()))
            .ok_or(PlanInput::Velocity)
    }

    /// `jump_aerial_y` is the double jump's height. Its first frame moves
    /// `sqrt(2 g h) - g / 2`, then loses `g` a frame (exact trace
    /// edge-niche-cand-fd-3, six fighters). Full-stick drift gains `add + mul`.
    pub fn from_params(native: NativeAir) -> Self {
        let gravity = native.air_accel_y;
        Self {
            horizontal_speed: native.air_speed_x_stable,
            horizontal_acceleration: native.air_accel_x_add + native.air_accel_x_mul,
            gravity,
            terminal_speed: native.air_speed_y_stable,
            jump_speed: (2.0 * gravity * native.jump_aerial_y).sqrt() - gravity * 0.5,
        }
    }

    pub fn valid(self) -> bool {
        [
            self.horizontal_speed,
            self.horizontal_acceleration,
            self.gravity,
            self.terminal_speed,
            self.jump_speed,
        ]
        .into_iter()
        .all(|value| value.is_finite() && value > 0.0)
    }

    /// Take the high path only if the fighter will clear the lip while airborne.
    pub fn high_route_input(
        self,
        position: Point,
        speed: Point,
        lip: Point,
        inward: f32,
        blast_bottom: f32,
        can_jump: bool,
    ) -> Option<Correction> {
        let outside = (lip[0] - position[0]) * inward > 0.0;
        let press = if outside {
            if self.reaches_with_jump(position, speed, lip, u8::MAX, blast_bottom) {
                0
            } else if can_jump && self.reaches_with_jump(position, speed, lip, 0, blast_bottom) {
                JUMP
            } else {
                return None;
            }
        } else if position[1] >= lip[1] {
            0
        } else {
            return None;
        };
        Some(Correction {
            stick: Some([inward, 0.0]),
            press,
            ..Correction::default()
        })
    }

    /// Drift towards the near ledge, preserving the native drift velocity,
    /// then spend the remaining jump. Stop at the first crossing of ledge x.
    pub fn reaches_with_jump(
        self,
        from: Point,
        speed: Point,
        ledge: Point,
        delay: u8,
        blast_bottom: f32,
    ) -> bool {
        if !self.valid()
            || !from
                .into_iter()
                .chain(speed)
                .chain(ledge)
                .chain([blast_bottom])
                .all(f32::is_finite)
        {
            return false;
        }
        let direction = (ledge[0] - from[0]).signum();
        let mut dx = (ledge[0] - from[0]).abs();
        let mut y = from[1];
        let [mut vx, mut vy] = [speed[0] * direction, speed[1]];
        const SIMULATION_FRAMES: u8 = 90;
        for frame in 0..SIMULATION_FRAMES {
            if y <= blast_bottom {
                return false;
            }
            vy = if frame == delay {
                self.jump_speed
            } else {
                (vy - self.gravity).max(-self.terminal_speed)
            };
            vx = (vx + self.horizontal_acceleration).min(self.horizontal_speed);
            dx -= vx;
            y += vy;
            if dx <= 0.0 {
                return y >= ledge[1];
            }
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Near,
    High,
    LateLow,
    Opposite,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    pub route: Route,
    pub waypoints: [Point; 3],
    pub count: usize,
    weight: u16,
}

/// One P1 hurtbox sample for an actionable recovery plan.
#[derive(Default)]
pub struct PlanHeight {
    frame: Option<u32>,
    value: Option<Option<f32>>,
    reported_unavailable: bool,
}

impl PlanHeight {
    pub fn request(&mut self, frame: u32, actionable: bool) -> bool {
        if !actionable || self.frame == Some(frame) || self.value.is_some() {
            return false;
        }
        self.frame = Some(frame);
        self.reported_unavailable = false;
        // The request counts even when the native sample is unavailable.
        self.value = Some(None);
        true
    }

    pub fn record(&mut self, frame: u32, height: Option<f32>) {
        if self.frame == Some(frame) && self.value.is_some() {
            self.value = Some(height);
        }
    }

    // Keep the sample for a retry when kinetic data becomes readable.
    pub fn report_unavailable(&mut self) -> bool {
        self.value.is_some() && !std::mem::replace(&mut self.reported_unavailable, true)
    }

    pub fn take(&mut self) -> Option<Option<f32>> {
        self.value.take()
    }
}

/// Observe changes while following the most recent plan.
#[derive(Default)]
pub struct RouteWatch {
    resources: Option<(u8, bool)>,
    blocked: bool,
    guarder_zone: Option<recovery::Zone>,
    next_zone: Option<recovery::Zone>,
    zone_frame: Option<u32>,
}

impl RouteWatch {
    pub fn set_guarder_zone(&mut self, zone: recovery::Zone) {
        self.guarder_zone = Some(zone);
        self.next_zone = None;
    }

    /// Require two distinct game frames in the new zone before requesting another plan.
    pub fn observe_guarder(&mut self, zone: recovery::Zone, frame: u32) -> bool {
        if self.zone_frame == Some(frame) {
            return false;
        }
        self.zone_frame = Some(frame);
        if self.guarder_zone.is_none() || self.guarder_zone == Some(zone) {
            self.set_guarder_zone(zone);
            return false;
        }
        if self.next_zone == Some(zone) {
            self.set_guarder_zone(zone);
            return true;
        }
        self.next_zone = Some(zone);
        false
    }

    pub fn observe(&mut self, resources: Option<(u8, bool)>, blocked: bool) -> bool {
        let spent = self.resources.zip(resources).is_some_and(
            |((previous_jumps, previous_special), (jumps, special))| {
                jumps < previous_jumps || previous_special && !special
            },
        );
        if let Some(current) = resources {
            self.resources = Some(current);
        }
        let entered = blocked && !self.blocked;
        self.blocked = blocked;
        spent || entered
    }
}

pub fn player_on_path(player: Point, path: Segment, reach: f32) -> bool {
    let [start, end] = path;
    let [dx, dy] = [end[0] - start[0], end[1] - start[1]];
    let [px, py] = [player[0] - start[0], player[1] - start[1]];
    let length = dx * dx + dy * dy;
    let where_on_path = if length > 0.0 {
        ((px * dx + py * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (px - dx * where_on_path).hypot(py - dy * where_on_path) <= reach
}

pub struct RouteFacts<'a> {
    pub fighter: i32,
    pub stage: (i32, u32),
    pub position: Point,
    pub player: Point,
    pub player_reach: f32,
    pub ledges: [Point; 2],
    pub blast: [f32; 4],
    pub outline: &'a [Segment],
    pub body_clearance: f32,
    pub ledge_distance: f32,
    pub fall_budget: f32,
    /// Remaining horizontal travel and ascent offered by the fighter's resources.
    pub reach: Point,
}

fn line_height([a, b]: Segment, x: f32) -> [f32; 2] {
    if a[0] == b[0] {
        [a[1].min(b[1]), a[1].max(b[1])]
    } else {
        let y = a[1] + (b[1] - a[1]) * ((x - a[0]) / (b[0] - a[0]));
        [y, y]
    }
}

fn ordered_x([a, b]: Segment) -> [f32; 2] {
    [a[0].min(b[0]), a[0].max(b[0])]
}

/// A solid line intersects the swept vertical body of a segment. For
/// nonvertical spans the extrema are at the ends of the overlapping x range.
fn blocked(path: Segment, line: Segment, height: f32) -> bool {
    let [from, to] = ordered_x(path);
    let [start, end] = ordered_x(line);
    let left = from.max(start);
    let right = to.min(end);
    if left > right {
        return false;
    }
    let delta = [left, right].map(|x| {
        let [foot, top] = line_height(path, x);
        let [lower, upper] = line_height(line, x);
        [lower - top, upper - foot]
    });
    delta[0][0].min(delta[1][0]) <= height && delta[0][1].max(delta[1][1]) >= 0.0
}

/// Check the foot-to-head interval across the center and both sides of an aerial path.
fn aerial_clear(outline: &[Segment], path: Segment, min_y: f32, height: f32, half: f32) -> bool {
    path.into_iter().all(|p| p[1] > min_y)
        && [-half, 0.0, half].into_iter().all(|offset| {
            let path = path.map(|p| [p[0] + offset, p[1]]);
            !outline.iter().any(|&line| blocked(path, line, height))
        })
}

pub(super) const fn is_crossing_stage(stage: (i32, u32)) -> bool {
    // Normal-form IDs from the 13.0.5 catalog: Battlefield, Final Destination and Waiting Room.
    matches!(stage, (0 | 3 | 311, 0))
}

impl RouteFacts<'_> {
    pub fn crossing_band(&self) -> Option<Point> {
        if !is_crossing_stage(self.stage) {
            return None;
        }
        corridor(
            self.outline,
            self.ledges,
            self.blast[3],
            self.body_clearance,
            self.fall_budget,
        )
    }
    fn candidate(&self, route: Route, waypoints: [Point; 3], count: usize) -> Option<Plan> {
        let [left, right, top, bottom] = self.blast;
        let mut from = self.position;
        let mut horizontal = 0.0;
        let mut ascent = 0.0;
        let mut threat = 0;
        for &to in &waypoints[..count] {
            if !(left < to[0]
                && to[0] < right
                && bottom < to[1]
                && to[1] + self.body_clearance < top)
                || self
                    .outline
                    .iter()
                    .any(|&line| blocked([from, to], line, self.body_clearance))
            {
                return None;
            }
            horizontal += (to[0] - from[0]).abs();
            ascent += (to[1] - from[1]).max(0.0);
            let player_distance = (to[0] - self.player[0]).hypot(to[1] - self.player[1]);
            threat += u16::from(player_distance <= self.player_reach);
            from = to;
        }
        if horizontal > self.reach[0] || ascent > self.reach[1] {
            return None;
        }
        let travel = horizontal / self.reach[0] + ascent / self.reach[1];
        let weight = (16.0 / (1.0 + travel + f32::from(threat) * 2.0)).max(1.0) as u16;
        Some(Plan {
            route,
            waypoints,
            count,
            weight,
        })
    }
}

/// Normal ledge and stage routes. An Opposite candidate comes from a measured
/// fighter guide after live stage and resource checks.
pub fn possible_routes(facts: &RouteFacts<'_>) -> [Option<Plan>; 3] {
    let [left, right] = facts.ledges;
    let clearance = facts.body_clearance;
    let offset = facts.ledge_distance;
    if !facts
        .position
        .into_iter()
        .chain(facts.player)
        .chain(facts.ledges.into_iter().flatten())
        .chain(facts.blast)
        .chain(facts.reach)
        .chain([clearance, offset, facts.player_reach])
        .chain(facts.outline.iter().flatten().flatten().copied())
        .all(f32::is_finite)
        || clearance <= 0.0
        || offset <= 0.0
        || left[0] >= right[0]
        || facts.reach.into_iter().any(|value| value <= 0.0)
        || facts.outline.is_empty()
    {
        return [None; 3];
    }
    let (near, other, edge_dir) = if facts.position[0] <= left[0].midpoint(right[0]) {
        (left, right, -1.0)
    } else {
        (right, left, 1.0)
    };
    let outside = near[0] + offset * edge_dir;
    let low = near[1] - clearance;
    let high = near[1].max(other[1]) + clearance;
    let finish = [outside, near[1]];
    let near_plan = facts.candidate(Route::Near, [finish; 3], 1);
    let high_plan = facts.candidate(
        Route::High,
        [
            [outside, high],
            [near[0].midpoint(other[0]), high],
            [0.0; 2],
        ],
        2,
    );
    let late_plan = facts.candidate(Route::LateLow, [[outside, low], finish, [0.0; 2]], 2);
    [near_plan, high_plan, late_plan]
}

/// Edits to the CPU controller after the stock AI has produced its pad.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Correction {
    pub stick: Option<Point>,
    pub press: u32,
    pub release: u32,
}

pub const ATTACK: u32 = 0x1;
pub const SPECIAL: u32 = 0x2;

#[derive(Clone, Copy)]
pub struct SpecialReach {
    pub horizontal: f32,
    pub rise: f32,
}

impl SpecialReach {
    pub fn valid(self) -> bool {
        self.horizontal.is_finite()
            && self.horizontal >= 0.0
            && self.rise.is_finite()
            && self.rise > 0.0
    }

    pub fn reaches(self, position: Point, target: Point) -> bool {
        self.valid()
            && (target[0] - position[0]).abs() <= self.horizontal
            && target[1] - position[1] <= self.rise
    }

    /// Spend the special on the last actionable frame before the target moves
    /// out of the fighter's native two-axis envelope.
    pub fn timed_input(self, position: Point, next: Point, target: Point) -> Correction {
        if self.reaches(position, target)
            && next.into_iter().all(f32::is_finite)
            && !self.reaches(next, target)
        {
            return Correction {
                stick: Some([0.0, 1.0]),
                press: SPECIAL,
                ..Correction::default()
            };
        }
        Correction::default()
    }
}

pub fn avoid_detour(
    buttons: u32,
    can_recover: bool,
    player_in_lag: bool,
    direction: f32,
) -> Correction {
    if !can_recover || player_in_lag || buttons & ATTACK == 0 || !direction.is_finite() {
        return Correction::default();
    }
    Correction {
        stick: Some([direction, 0.0]),
        release: ATTACK,
        ..Correction::default()
    }
}

pub const JUMP: u32 = 0x4;

pub const GUARD: u32 = 0x8;

#[derive(Clone, Copy)]
pub struct DodgeReach {
    pub distance: f32,
    pub cancel_frames: f32,
}

pub fn distance_dodge(
    position: Point,
    ledge: Point,
    reach: DodgeReach,
    last_resource: bool,
    can_coast: bool,
) -> Correction {
    let x = ledge[0] - position[0];
    let y = ledge[1] - position[1];
    let distance = x.hypot(y);
    if !last_resource
        || can_coast
        || !reach.distance.is_finite()
        || !reach.cancel_frames.is_finite()
        || reach.cancel_frames <= 0.0
        || !(0.0 < distance && distance <= reach.distance)
    {
        return Correction::default();
    }
    Correction {
        stick: Some([x / distance, y / distance]),
        press: GUARD,
        ..Correction::default()
    }
}

const ATTACK_INPUT: u64 = u64::MAX;

pub fn attack_key(
    status: i32,
    input: u32,
    native_motion: impl FnOnce() -> u64,
    konst: impl Fn(crate::model::cpu::ids::C) -> i32,
) -> Option<u64> {
    use crate::model::{cpu::ids::{C, UNRESOLVED}, hash40::hash40};

    // A smash can transition from startup to charge to release with different motions.
    const SMASHES: [([C; 3], u64); 3] = [
        ([C::StatusAttackHi4Start, C::StatusAttackHi4Hold, C::StatusAttackHi4], hash40("attack_hi4")),
        ([C::StatusAttackLw4Start, C::StatusAttackLw4Hold, C::StatusAttackLw4], hash40("attack_lw4")),
        ([C::StatusAttackS4Start, C::StatusAttackS4Hold, C::StatusAttackS4], hash40("attack_s4_s")),
    ];
    if status == UNRESOLVED {
        return (input & 3 != 0).then_some(ATTACK_INPUT);
    }
    for (phases, key) in SMASHES {
        if phases.map(&konst).contains(&status) {
            return Some(key);
        }
    }

    let attacks = [
        C::StatusAttack,
        C::StatusAttackDash,
        C::StatusAttackS3,
        C::StatusAttackHi3,
        C::StatusAttackLw3,
        C::StatusAttackAir,
        C::StatusSpecialN,
        C::StatusSpecialS,
        C::StatusSpecialHi,
        C::StatusSpecialLw,
    ];
    let motion = attacks.map(konst).contains(&status)
        .then(native_motion)
        .filter(|&motion| motion != 0);
    motion.or_else(|| (input & 3 != 0).then_some(ATTACK_INPUT))
}

/// Earliest observable start of the current P1 attack on the attempt clock.
#[derive(Default)]
pub struct ThreatClock {
    attack: Option<u64>,
    since: u32,
    dodged: bool,
}

impl ThreatClock {
    pub fn trigger_frame(&self) -> u32 {
        self.since
    }

    pub fn observe(
        &mut self,
        attack: Option<u64>,
        frame: u32,
        reaction: u8,
        contact: bool,
        can_dodge: bool,
        buttons: u32,
    ) -> Correction {
        if self.attack != attack {
            // Input can precede the first attack-motion sample by one or more frames.
            if self.attack != Some(ATTACK_INPUT) || attack.is_none() {
                self.since = frame;
            }
            self.attack = attack;
            self.dodged = false;
        }
        let ready = attack.is_some() && frame.saturating_sub(self.since) >= u32::from(reaction);
        if ready && contact && can_dodge {
            if self.dodged {
                return Correction::default();
            }
            self.dodged = true;
            return Correction {
                stick: Some([0.0; 2]),
                press: GUARD,
                ..Correction::default()
            };
        }
        Correction {
            release: buttons & GUARD,
            ..Correction::default()
        }
    }
}

#[cfg(any(test, target_os = "switch"))]
#[path = "perfect/startup_dodge.rs"]
pub(crate) mod startup_dodge;

#[derive(Clone, Copy)]
pub struct Capsule {
    pub ends: [[f32; 3]; 2],
    pub radius: f32,
}

impl Capsule {
    pub fn touches_segment(self, ends: [[f32; 3]; 2]) -> bool {
        self.touches(Self { ends, radius: 0.0 })
    }

    fn touches(self, other: Self) -> bool {
        let dot = |a: [f32; 3], b: [f32; 3]| (0..3).map(|i| a[i] * b[i]).sum::<f32>();
        let minus = |a: [f32; 3], b: [f32; 3]| [0, 1, 2].map(|i| a[i] - b[i]);
        let [a0, a1] = self.ends;
        let [b0, b1] = other.ends;
        let u = minus(a1, a0);
        let v = minus(b1, b0);
        let w = minus(a0, b0);
        let (uu, vv, uv) = (dot(u, u), dot(v, v), dot(u, v));
        let (uw, vw) = (dot(u, w), dot(v, w));
        let det = uu * vv - uv * uv;
        let mut t = if det > 0.0 {
            ((uv * vw - vv * uw) / det).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let s = if vv > 0.0 {
            ((uv * t + vw) / vv).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if uu > 0.0 {
            t = ((uv * s - uw) / uu).clamp(0.0, 1.0);
        }
        let delta = [0, 1, 2].map(|i| w[i] + t * u[i] - s * v[i]);
        let radius = self.radius + other.radius;
        radius.is_finite()
            && self.radius > 0.0
            && other.radius >= 0.0
            && dot(delta, delta) <= radius * radius
    }
}

pub enum MeteorWindow {
    Waiting,
    Actionable,
    Interrupted,
}

#[derive(Default)]
pub struct MeteorBounce {
    awaiting_dodge: bool,
}

impl MeteorBounce {
    pub fn awaiting_dodge(&self) -> bool {
        self.awaiting_dodge
    }

    pub fn step(
        &mut self,
        bounce_started: bool,
        window: MeteorWindow,
        idle_player: bool,
        buttons: u32,
    ) -> Correction {
        self.awaiting_dodge |= bounce_started;
        let mut edit = Correction::default();
        match window {
            MeteorWindow::Waiting => {}
            MeteorWindow::Interrupted => self.awaiting_dodge = false,
            MeteorWindow::Actionable => {
                if std::mem::take(&mut self.awaiting_dodge) && idle_player {
                    edit.release = buttons & GUARD;
                }
            }
        }
        edit
    }
}

/// Prefer the first projected blast boundary to any follow-up positioning.
pub fn hit_recovery(
    cpu: Point,
    player: Point,
    speed: Point,
    hitstun: f32,
    blast: [f32; 4],
) -> Option<Point> {
    if !cpu
        .into_iter()
        .chain(player)
        .chain(speed)
        .chain(blast)
        .chain([hitstun])
        .all(f32::is_finite)
        || hitstun < 0.0
    {
        return None;
    }
    let [left, right, top, bottom] = blast;
    let [vx, vy] = speed;
    let crossing = |position: f32, speed: f32, low: f32, high: f32| {
        if speed == 0.0 {
            f32::INFINITY
        } else {
            (if speed > 0.0 {
                high - position
            } else {
                low - position
            }) / speed
        }
    };
    let x = crossing(cpu[0], vx, left, right);
    let y = crossing(cpu[1], vy, bottom, top);
    let away = if cpu[0] == player[0] {
        0.0
    } else {
        (cpu[0] - player[0]).signum()
    };
    Some(if x <= y && x <= hitstun {
        [-vx.signum(), 0.0]
    } else if y <= hitstun {
        [0.0, -vy.signum()]
    } else {
        [away, 0.0]
    })
}

#[derive(Clone, Copy)]
pub struct LandingFacts {
    pub cpu: Point,
    pub player: Point,
    pub main_floor: [f32; 2],
    pub stock_stick_y: f32,
    pub stock_buttons: u32,
}

// Stock enables flick jump separately from the button presses for this frame.
pub const FLICK_JUMP: u32 = 0x8000;

#[derive(Clone, Copy, Default)]
pub struct LandingOptions {
    pub player_in_reach: bool,
    pub fast_fall: bool,
    pub jump: bool,
    pub platform_x: Option<f32>,
}

pub fn landing_variant(facts: LandingFacts, options: LandingOptions, draw: u32) -> Correction {
    let LandingFacts {
        cpu,
        player,
        main_floor: [left, right],
        stock_stick_y,
        stock_buttons,
    } = facts;
    if stock_buttons & !FLICK_JUMP != 0
        || cpu[1] <= player[1]
        || cpu[0] <= left
        || cpu[0] >= right
        || !cpu
            .into_iter()
            .chain(player)
            .chain([left, right, stock_stick_y])
            .all(f32::is_finite)
    {
        return Correction::default();
    }
    let away = if cpu[0] != player[0] {
        (cpu[0] - player[0]).signum()
    } else if draw.is_multiple_of(2) {
        -1.0
    } else {
        1.0
    };
    let mut out = Correction::default();
    let planned = match draw {
        0..=3 => Some([away, stock_stick_y]),
        4 => Some([0.0, stock_stick_y]),
        6 if options.fast_fall => Some([0.0, -1.0]),
        7 if options.jump => {
            out.press = JUMP;
            Some([if cpu[0] == player[0] { 0.0 } else { away }, stock_stick_y])
        }
        8 => options
            .platform_x
            .filter(|x| left < *x && *x < right)
            .map(|x| [if x == cpu[0] { 0.0 } else { (x - cpu[0]).signum() }, stock_stick_y]),
        _ => None,
    };
    // An unavailable option under P1's reach can still leave the threat column by drifting.
    let retreat = options.player_in_reach && (6..=8).contains(&draw);
    out.stick = planned.or(retreat.then_some([away, stock_stick_y]));
    out
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn pending_recovery_samples_p1_once_after_actionable_status() {
        let mut height = PlanHeight::default();
        for frame in 10..20 {
            assert!(!height.request(frame, false), "damage or helpless frames need no sample");
        }
        assert!(!height.report_unavailable(), "a plan without a sample has no rejection");
        assert!(height.request(20, true));
        height.record(20, Some(14.6));
        assert!(height.report_unavailable());
        assert!(!height.report_unavailable(), "one rejection is sufficient while a plan waits");
        assert!(!height.request(21, true), "keep the first sample until it is consumed");
        assert_eq!(height.take(), Some(Some(14.6)));
        assert!(!height.request(20, true), "no second read on one frame");
        assert!(height.request(21, true));
        assert!(height.report_unavailable(), "a new plan has its own witness");
        assert_eq!(height.take(), Some(None), "failed or unavailable sample still counts");
        height.record(21, Some(14.6));
        assert_eq!(height.take(), None, "an in-flight sample cannot revive a consumed plan");
    }

    #[test]
    fn juggle_drift_uses_legal_fall_time_and_leaves_stock_defensive_inputs() {
        let facts = LandingFacts {
            cpu: [5., 45.], player: [0., 5.], main_floor: [-70., 70.],
            stock_stick_y: -0.15, stock_buttons: 0x8000,
        };
        let juggle_drift = |facts, draw| landing_variant(facts, LandingOptions::default(), draw);
        assert_eq!(juggle_drift(facts, 0).stick, Some([1., -0.15]));
        assert_eq!(juggle_drift(facts, 4).stick, Some([0., -0.15]));
        assert_eq!(juggle_drift(LandingFacts { stock_buttons: 0x8008, ..facts }, 0), Correction::default());
        assert_eq!(juggle_drift(LandingFacts { cpu: [70., 45.], ..facts }, 0), Correction::default());
        assert_eq!(juggle_drift(LandingFacts { player: [0., 50.], ..facts }, 0), Correction::default());
    }

    #[test]
    fn recovery_preserves_a_safe_punish_and_cancels_only_unsafe_attack_buttons() {
        let buttons = 0x8005;
        assert_eq!(avoid_detour(buttons, false, false, 1.), Correction::default());
        assert_eq!(avoid_detour(buttons, true, true, -1.), Correction::default());
        assert_eq!(avoid_detour(0x8004, true, false, 1.), Correction::default());
        let correction = avoid_detour(buttons, true, false, -1.);
        assert_eq!(correction.release, 0x1);
        assert_eq!(correction.stick, Some([-1., 0.]));
        assert_eq!(correction.press, 0);
    }

    #[test]
    fn landing_variants_only_use_a_safe_and_legal_input() {
        let facts = LandingFacts { cpu: [5., 45.], player: [0., 5.], main_floor: [-70., 70.],
            stock_stick_y: -0.15, stock_buttons: 0x8000 };
        let legal = LandingOptions { fast_fall: true, jump: true, platform_x: Some(-15.),
            ..LandingOptions::default() };
        assert_eq!(landing_variant(facts, legal, 6).stick, Some([0., -1.]));
        assert_eq!(landing_variant(facts, legal, 7).press, JUMP);
        assert_eq!(landing_variant(facts, legal, 8).stick, Some([-1., -0.15]));
        assert_eq!(landing_variant(facts, LandingOptions::default(), 7), Correction::default());
        assert_eq!(landing_variant(LandingFacts { stock_buttons: 0x8008, ..facts }, legal, 8),
            Correction::default());
    }

    #[test]
    fn recovery_planning_distinguishes_invalid_air_data_from_unavailable_velocity() {
        let air = AirPhysics { horizontal_speed: 0.924, horizontal_acceleration: 0.05,
            gravity: 0.125, terminal_speed: 1.75, jump_speed: 3.147 };
        assert_eq!(air.plan_velocity(Some((0., -1.75))), Ok([0., -1.75]));
        assert_eq!(air.plan_velocity(None), Err(PlanInput::Velocity));
        assert_eq!(air.plan_velocity(Some((f32::NAN, -1.75))), Err(PlanInput::Velocity));
        assert_eq!(air.plan_velocity(Some((0., f32::INFINITY))), Err(PlanInput::Velocity));
        let invalid = AirPhysics { horizontal_acceleration: 0., ..air };
        assert_eq!(invalid.plan_velocity(Some((0., -1.75))), Err(PlanInput::AirPhysics));
    }

    #[test]
    fn landing_replans_an_unavailable_option_away_from_p1() {
        let facts = LandingFacts { cpu: [5., 45.], player: [0., 5.], main_floor: [-70., 70.],
            stock_stick_y: -0.15, stock_buttons: FLICK_JUMP };
        let pressured = LandingOptions { player_in_reach: true, ..LandingOptions::default() };
        for draw in 6..=8 {
            for (x, expected) in [(-5., -1.), (5., 1.)] {
                let facts = LandingFacts { cpu: [x, 45.], ..facts };
                assert_eq!(landing_variant(facts, pressured, draw), Correction {
                    stick: Some([expected, -0.15]), ..Correction::default()
                }, "unavailable option {draw} at x={x}");
            }
        }
        assert_eq!(landing_variant(facts, pressured, 5), Correction::default());
        let defending = LandingFacts { stock_buttons: FLICK_JUMP | GUARD, ..facts };
        assert_eq!(landing_variant(defending, pressured, 8), Correction::default());
    }

    #[test]
    fn up_special_recovery_depends_on_horizontal_reach_and_height() {
        let reach = SpecialReach { horizontal: 20., rise: 45. };
        let ledge = [-80., 0.];
        let from = [-95., -44.];
        assert!(reach.reaches(from, ledge));
        assert!(!reach.reaches([-101., -25.], ledge));
        assert!(!reach.reaches([-90., -46.], ledge));
        assert!(!SpecialReach { horizontal: f32::NAN, ..reach }.reaches(from, ledge));
        assert_eq!(
            reach.timed_input(from, [-94., -46.], ledge).press,
            SPECIAL
        );
        assert_eq!(reach.timed_input(from, [-94., -43.], ledge), Correction::default());
        assert_eq!(
            reach.timed_input([-101., -30.], [-100., -32.], ledge),
            Correction::default()
        );
    }

    #[test]
    fn distance_airdodge_only_spends_the_last_resource_when_it_reaches() {
        let ledge = [-80., 0.];
        let from = [-104., -7.];
        let base = DodgeReach { distance: 25., cancel_frames: 60. };
        let pad = distance_dodge(from, ledge, base, true, false);
        assert_eq!(pad.press, GUARD);
        let [x, y] = pad.stick.unwrap();
        assert!(x > 0.9 && y > 0.0);
        assert_eq!(distance_dodge(from, ledge, base, false, false), Correction::default());
        assert_eq!(distance_dodge(from, ledge, base, true, true), Correction::default());
        assert_eq!(
            distance_dodge([-104., -8.], ledge, base, true, false),
            Correction::default()
        );
        assert_eq!(
            distance_dodge(from, ledge, DodgeReach { distance: 0., ..base }, true, false),
            Correction::default()
        );
        assert_eq!(
            distance_dodge(from, ledge, DodgeReach { cancel_frames: f32::NAN, ..base }, true, false),
            Correction::default()
        );
    }

    #[test]
    fn high_route_spends_the_jump_before_reaching_the_lip() {
        let air = AirPhysics {
            horizontal_speed: 2.0,
            horizontal_acceleration: 0.15,
            gravity: 0.12,
            terminal_speed: 2.1,
            jump_speed: 3.5,
        };
        let lip = [-70., 8.];
        assert_eq!(
            air.high_route_input([-95., -14.], [0., -1.4], lip, 1., -95., true),
            Some(Correction { stick: Some([1., 0.]), press: JUMP, release: 0 })
        );
        assert_eq!(air.high_route_input([-95., -14.], [0., -1.4], lip, 1., -95., false), None);
        assert_eq!(air.high_route_input([-73., -14.], [2., -1.4], lip, 1., -95., true), None);
        assert_eq!(
            air.high_route_input([-68., 10.], [1., -0.5], lip, 1., -95., false),
            Some(Correction { stick: Some([1., 0.]), ..Correction::default() })
        );
        assert_eq!(
            air.high_route_input([95., -14.], [0., -1.4], [70., 8.], -1., -95., true)
                .map(|pad| pad.stick),
            Some(Some([-1., 0.]))
        );
    }

    #[test]
    fn charged_smash_preserves_the_juggler_reaction_clock() {
        use crate::model::{cpu::ids::{C, Table}, hash40::hash40};

        // Native Mario P1 entered 0x33 -> 0x34 -> 0x35 at seq 26729/26742/26745
        // in the exact 3a29e6af j-h2 command 010. A longer hold exposes both floors.
        let ids = Table::with(&[
            (C::StatusAttackHi4Start, 0x33),
            (C::StatusAttackHi4Hold, 0x34),
            (C::StatusAttackHi4, 0x35),
        ]);
        let key = |status, input, motion| attack_key(status, input, || hash40(motion), |c| ids.get(c));
        let mut clocks = [ThreatClock::default(), ThreatClock::default()];
        for clock in &mut clocks {
            clock.observe(key(0, 1, "wait"), 100, 12, false, true, 0);
            clock.observe(key(0x33, 1, "attack_hi4"), 101, 12, false, true, 0);
            clock.observe(key(0x34, 1, "attack_hi4_hold"), 106, 12, false, true, 0);
            assert_eq!(clock.trigger_frame(), 100, "charge belongs to the same attack");
        }
        let released = key(0x35, 0, "attack_hi4");
        assert_eq!(clocks[0].observe(released, 111, 0, true, true, 0).press, GUARD);
        assert_eq!(clocks[1].observe(released, 111, 12, true, true, 0).press, 0);
        assert_eq!(clocks[1].observe(released, 112, 12, true, true, 0).press, GUARD);
    }

    #[test]
    fn timed_dodge_requires_contact_and_honors_the_reaction_clock() {
        let mut threat = ThreatClock::default();
        assert_eq!(threat.observe(None, 100, 0, true, true, 0x8008).release, 0x8);
        assert_eq!(threat.observe(Some(1), 101, 12, true, true, 0x8008).release, 0x8);
        assert_eq!(threat.observe(Some(1), 112, 12, true, true, 0x8008).release, 0x8);
        assert_eq!(threat.observe(Some(1), 113, 12, false, true, 0x8000), Correction::default());
        assert_eq!(threat.observe(Some(1), 114, 12, true, false, 0x8000), Correction::default());
        assert_eq!(threat.observe(Some(1), 115, 12, true, true, 0x8000).press, 0x8);
        assert_eq!(threat.observe(Some(1), 116, 12, true, true, 0x8000), Correction::default());
        assert_eq!(threat.observe(Some(2), 117, 0, true, true, 0x8000).press, 0x8);
    }

    #[test]
    fn hit_recovery_favors_survival_when_launch_would_cross_a_blast_zone() {
        let area = [-120., 120., 80., -80.];
        let player = [10., 0.];
        assert_eq!(hit_recovery([30., 32.], player, [2., 2.], 8., area), Some([1., 0.]));
        assert_eq!(hit_recovery([30., 70.], player, [2., 2.], 8., area), Some([0., -1.]));
        assert_eq!(hit_recovery([100., 0.], player, [3., 2.], 8., area), Some([-1., 0.]));
        assert_eq!(hit_recovery([0., -69.], player, [0., -3.], 8., area), Some([0., 1.]));
        assert_eq!(hit_recovery([f32::NAN, 0.], player, [1., 1.], 8., area), None);
    }

    #[test]
    fn contact_uses_the_full_capsules_and_rejects_out_of_plane_hits() {
        let hit = Capsule { ends: [[-8., 1., 0.], [8., 1., 0.]], radius: 1. };
        let hurt = Capsule { ends: [[0., -5., 0.], [0., 7., 0.]], radius: 1. };
        assert!(hit.touches(hurt));
        assert!(hurt.touches(hit));
        assert!(hit.touches_segment(hurt.ends));
        assert!(!hit.touches_segment([[0., -5., 1.25], [0., 7., 1.25]]));
        assert!(!hit.touches(Capsule { ends: [[0., -5., 3.], [0., 7., 3.]], ..hurt }));
        assert!(!hit.touches(Capsule { ends: [[11., 1., 0.]; 2], ..hurt }));
        assert!(hit.touches(Capsule { ends: [[10., 1., 0.]; 2], ..hurt }));
    }

    #[test]
    fn meteor_bounce_keeps_the_dodge_unless_the_player_waits_on_entry() {
        use MeteorWindow::Actionable;

        let mut bounce = MeteorBounce::default();
        assert_eq!(bounce.step(false, Actionable, true, 0x8008), Correction::default());
        assert_eq!(bounce.step(true, Actionable, true, 0x8008).release, GUARD);
        assert_eq!(bounce.step(false, Actionable, true, 0x8008), Correction::default());
        assert_eq!(bounce.step(true, Actionable, false, 0x8008), Correction::default());
        assert_eq!(bounce.step(false, Actionable, true, 0x8008), Correction::default());
    }

    #[test]
    fn meteor_bounce_waits_for_the_first_actionable_dodge() {
        use MeteorWindow::{Actionable, Waiting};

        let mut bounce = MeteorBounce::default();
        assert_eq!(bounce.step(true, Waiting, true, 0x8000), Correction::default());
        assert_eq!(bounce.step(false, Waiting, true, 0x8008), Correction::default());
        assert_eq!(bounce.step(false, Actionable, true, 0x8008).release, GUARD);
        assert_eq!(bounce.step(false, Actionable, true, 0x8008), Correction::default());
    }

    #[test]
    fn meteor_bounce_drops_its_pending_answer_on_interruption() {
        use MeteorWindow::{Actionable, Interrupted, Waiting};

        let mut bounce = MeteorBounce::default();
        bounce.step(true, Waiting, true, 0);
        assert!(bounce.awaiting_dodge());
        bounce.step(false, Interrupted, true, 0);
        assert!(!bounce.awaiting_dodge());
        assert_eq!(bounce.step(false, Actionable, true, GUARD), Correction::default());
    }

    #[test]
    fn recovery_jump_can_wait_only_with_enough_horizontal_and_vertical_reach() {
        let air = AirPhysics {
            horizontal_speed: 2.0,
            horizontal_acceleration: 0.15,
            gravity: 0.12,
            terminal_speed: 2.1,
            jump_speed: 3.5,
        };
        let position = [-95., -14.];
        let velocity = [0., -1.4];
        let ledge = [-70., 0.];
        assert!(air.reaches_with_jump(position, velocity, ledge, 4, -95.));
        assert!(!air.reaches_with_jump(position, velocity, ledge, 30, -95.));
        assert!(!air.reaches_with_jump(position, velocity, [40., 0.], 0, -95.));
        assert!(!air.reaches_with_jump([-95., -94.], velocity, ledge, 1, -95.));
    }

    #[test]
    fn a_crossable_outline_without_a_measured_route_never_offers_opposite() {
        let ledges = [[-40., 0.], [40., 0.]];
        let outline = [[ledges[0], ledges[1]], [ledges[0], [0., -15.]], [[0., -15.], ledges[1]]];
        let mut facts = RouteFacts {
            fighter: 0x1d,
            stage: (0, 0),
            position: [-53., -22.],
            player: [-41., 0.],
            player_reach: 18.,
            ledges,
            blast: [-130., 130., 100., -90.],
            outline: &outline,
            body_clearance: 9.,
            ledge_distance: 5.,
            fall_budget: 12.,
            reach: [120., 42.],
        };
        for stage in [0, 3, 311] {
            facts.stage = (stage, 0);
            assert!(facts.crossing_band().is_some(), "physical corridor is open");
            let plans = possible_routes(&facts);
            assert!(plans.iter().flatten().any(|plan| plan.route == Route::Near));
            assert!(plans.iter().flatten().all(|plan| plan.route != Route::Opposite),
                "stage {stage} requires a measured fighter route as well as geometry");
        }
    }

    #[test]
    fn moving_stage_ledges_come_from_the_current_solid_floor() {
        let tilted = [
            [[-32.0, 1.0], [-80.0, -18.0]],
            [[-32.0, 1.0], [32.0, 5.0]],
            [[80.0, -2.0], [32.0, 5.0]],
        ];
        assert_eq!(floor_ledges(tilted), Some([[-80.0, -18.0], [80.0, -2.0]]));
        assert_eq!(floor_ledges([]), None);
        assert_eq!(floor_ledges([[[0.0, 0.0], [0.0, 2.0]]]), None);
        assert_eq!(floor_ledges([[[f32::NAN, 0.0], [2.0, 2.0]]]), None);
    }

    #[test]
    fn only_the_three_approved_normal_stages_have_a_crossing_band() {
        use crate::engine::stages::catalog::STAGES;
        let ledges = [[-40., 0.], [40., 0.]];
        let outline = [[ledges[0], ledges[1]], [ledges[0], [0., -15.]], [[0., -15.], ledges[1]]];
        let mut facts = RouteFacts {
            fighter: 0x1d,
            stage: (0, 0),
            position: [-53., -22.],
            player: [-41., 0.],
            player_reach: 18.,
            ledges,
            blast: [-130., 130., 100., -90.],
            outline: &outline,
            body_clearance: 9.,
            ledge_distance: 5.,
            fall_budget: 12.,
            reach: [120., 42.],
        };
        let approved = ["Waiting Room", "Battlefield", "Final Destination"];
        for &(id, label) in STAGES {
            facts.stage = (id as i32, 0);
            let [near, high, low] = possible_routes(&facts);
            assert!([near, high, low].into_iter().any(|plan| plan.is_some()));
            assert_eq!(facts.crossing_band().is_some(), approved.contains(&label), "{label}");
        }
        for stage in [(-1, 0), (0, 1), (3, 2), (89, 0), (311, 1)] {
            facts.stage = stage;
            assert!(facts.crossing_band().is_none(), "{stage:?}");
        }

        // A fresh outline can close a corridor on an otherwise approved stage.
        let low_floor = [[ledges[0], ledges[1]], [ledges[0], [0.0, -83.0]], [[0.0, -83.0], ledges[1]]];
        facts.stage = (3, 0);
        facts.outline = &low_floor;
        assert!(facts.crossing_band().is_none());
    }

    #[test]
    fn a_recovery_replans_once_when_the_guarder_moves_to_a_new_zone() {
        use recovery::Zone::{Far, Hang, Ledge, OffHigh};
        let mut watch = RouteWatch::default();
        assert!(!watch.observe_guarder(Far, 11));
        assert!(!watch.observe_guarder(Ledge, 12));
        assert!(!watch.observe_guarder(Ledge, 12), "duplicate AI think");
        assert!(watch.observe_guarder(Ledge, 13), "P1 is guarding the ledge now");
        assert!(!watch.observe_guarder(Ledge, 14));
        assert!(!watch.observe_guarder(OffHigh, 15));
        assert!(!watch.observe_guarder(Ledge, 16), "one-frame change");
        assert!(!watch.observe_guarder(Hang, 17));
        assert!(watch.observe_guarder(Hang, 18), "P1 took the ledge");
        assert!(!watch.observe_guarder(Hang, 19));
    }

    #[test]
    fn a_recovery_plan_rebuilds_only_on_new_threat_or_spent_resource() {
        let mut watch = RouteWatch::default();
        assert!(!watch.observe(Some((2, true)), false));
        assert!(!watch.observe(Some((2, true)), false));
        assert!(!watch.observe(None, false));
        assert!(watch.observe(Some((1, true)), false));
        assert!(!watch.observe(Some((1, true)), false));
        assert!(watch.observe(Some((1, false)), false));
        assert!(watch.observe(Some((1, false)), true));
        assert!(!watch.observe(Some((1, false)), true));
        assert!(!watch.observe(Some((1, false)), false));
        assert!(watch.observe(Some((1, false)), true));
    }

    #[test]
    fn crossability_samples_below_camera_and_rejects_the_deep_underside() {
        let camera = [-105., 105., 60., -60.];
        let blast = [-180., 180., 140., -115.];
        assert_eq!(outline_probe(camera, blast), Some((0., [140., -115.])));
        assert_eq!(outline_probe(camera, [0., 0., 0., f32::NAN]), None);
        let ledges = [[-60., 0.], [60., 0.]];
        let main_stage = [
            [ledges[0], ledges[1]],
            [ledges[1], [0., -107.]],
            [[0., -107.], ledges[0]],
        ];
        assert_eq!(corridor(&main_stage, ledges, blast[3], 9., 2.), None);
    }

    #[test]
    fn opposite_ledge_requires_a_full_corridor_above_the_blast_zone() {
        let ledges = [[-70., 0.], [70., 0.]];
        let fd = [
            [ledges[0], ledges[1]],
            [ledges[0], [0., -32.]],
            [[0., -32.], ledges[1]],
        ];
        assert_eq!(corridor(&fd, ledges, -105., 12., 15.), Some([-90., -44.]));
        let wedge = [fd[0], [ledges[0], [0., -110.]], [[0., -110.], ledges[1]]];
        assert_eq!(corridor(&wedge, ledges, -105., 12., 15.), None);
        assert_eq!(corridor(&fd, ledges, -60., 18., 15.), None);
        assert_eq!(corridor(&[], ledges, -105., 12., 15.), None);
    }
    }
}
