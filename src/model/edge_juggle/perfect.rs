//! Perfect-mode recovery and escape decisions. The runtime applies only pad inputs.

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
        Self { segments: [[[0.0; 2]; 2]; 64], len: 0 }
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

/// The foot heights that leave enough room below a closed main-stage outline.
/// The caller passes live segments from one walk in the current stage.
pub fn corridor(
    outline: &[Segment],
    ledges: [Point; 2],
    blast_bottom: f32,
    body_clearance: f32,
    fall_budget: f32,
) -> Option<[f32; 2]> {
    let [[left, left_y], [right, right_y]] = ledges;
    if ![left, left_y, right, right_y, blast_bottom, body_clearance, fall_budget]
        .into_iter().all(f32::is_finite)
        || left >= right || body_clearance <= 0.0 || fall_budget < 0.0 || outline.is_empty()
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

impl AirPhysics {
    pub fn valid(self) -> bool {
        [self.horizontal_speed, self.horizontal_acceleration, self.gravity,
            self.terminal_speed, self.jump_speed]
            .into_iter().all(|value| value.is_finite() && value > 0.0)
    }

    /// Drift towards the near ledge, preserving the native drift velocity,
    /// then spend the remaining jump. Stop at the first crossing of ledge x.
    pub fn reaches_with_jump(
        self, from: Point, speed: Point, ledge: Point, delay: u8, blast_bottom: f32,
    ) -> bool {
        if !self.valid() || !from.into_iter().chain(speed).chain(ledge).chain([blast_bottom]).all(f32::is_finite) {
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

pub struct RouteFacts<'a> {
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

impl RouteFacts<'_> {
    fn candidate(&self, route: Route, waypoints: [Point; 3], count: usize) -> Option<Plan> {
        let [left, right, top, bottom] = self.blast;
        let mut from = self.position;
        let mut horizontal = 0.0;
        let mut ascent = 0.0;
        let mut threat = 0;
        for &to in &waypoints[..count] {
            if !(left < to[0] && to[0] < right && bottom < to[1]
                && to[1] + self.body_clearance < top)
                || self.outline.iter().any(|&line| blocked([from, to], line, self.body_clearance))
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
        Some(Plan { route, waypoints, count, weight })
    }
}

pub fn possible_routes(facts: &RouteFacts<'_>) -> [Option<Plan>; 4] {
    let [left, right] = facts.ledges;
    let clearance = facts.body_clearance;
    let offset = facts.ledge_distance;
    if !facts.position.into_iter().chain(facts.player).chain(facts.ledges.into_iter().flatten())
        .chain(facts.blast).chain(facts.reach).chain([clearance, offset, facts.player_reach])
        .chain(facts.outline.iter().flatten().flatten().copied())
        .all(f32::is_finite)
        || clearance <= 0.0 || offset <= 0.0 || left[0] >= right[0]
        || facts.reach.into_iter().any(|value| value <= 0.0)
        || facts.outline.is_empty()
    {
        return [None; 4];
    }
    let (near, other, edge_dir) = if facts.position[0] <= left[0].midpoint(right[0]) {
        (left, right, -1.0)
    } else {
        (right, left, 1.0)
    };
    let outside = near[0] + offset * edge_dir;
    let exit = other[0] - offset * edge_dir;
    let low = near[1] - clearance;
    let high = near[1].max(other[1]) + clearance;
    let finish = [outside, near[1]];
    let near_plan = facts.candidate(Route::Near, [finish; 3], 1);
    let high_plan = facts.candidate(
        Route::High, [[outside, high], [near[0].midpoint(other[0]), high], [0.0; 2]], 2,
    );
    let late_plan = facts.candidate(Route::LateLow, [[outside, low], finish, [0.0; 2]], 2);
    let near_covered = (facts.player[0] - near[0]).hypot(facts.player[1] - near[1]) <= facts.player_reach;
    let other_covered = (facts.player[0] - other[0]).hypot(facts.player[1] - other[1]) <= facts.player_reach;
    let opposite = if !other_covered && (near_plan.is_none() || near_covered) {
        corridor(facts.outline, facts.ledges, facts.blast[3], clearance, facts.fall_budget)
            .and_then(|[bottom, top]| {
                let crossing = facts.position[1].clamp(bottom, top.next_down().max(bottom));
                facts.candidate(
                    Route::Opposite,
                    [[outside, crossing], [exit, crossing], [exit, other[1]]],
                    3,
                )
            })
    } else {
        None
    };
    [near_plan, high_plan, late_plan, opposite]
}

/// Weighted selection among feasible options, reducing the weight of the last
/// route. The draw comes from the plugin's own RNG.
pub fn choose_route(options: [Option<Plan>; 4], last: Option<Route>, draw: u32) -> Option<Plan> {
    let weight = |plan: Plan| if Some(plan.route) == last { plan.weight.div_ceil(2) } else { plan.weight };
    let total: u32 = options.iter().flatten().map(|&plan| u32::from(weight(plan))).sum();
    if total == 0 {
        return None;
    }
    let mut draw = ((u64::from(draw) * u64::from(total)) >> 32) as u32;
    for plan in options.into_iter().flatten() {
        let weight = u32::from(weight(plan));
        if draw < weight {
            return Some(plan);
        }
        draw -= weight;
    }
    None
}

/// Edits to the CPU controller after the stock AI has produced its pad.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Correction {
    pub stick: Option<Point>,
    pub press: u32,
    pub release: u32,
}

pub const JUMP: u32 = 0x4;

/// A stock jump held in reserve until its remaining delay fits the recovery budget.
#[derive(Default)]
pub struct JumpTiming {
    pending: Option<u8>,
}

impl JumpTiming {
    pub fn waiting(&self) -> bool {
        self.pending.is_some()
    }

    pub fn step(
        &mut self, buttons: u32, can_jump: bool, delay: u8, within_reach: impl FnOnce(u8) -> bool,
    ) -> Correction {
        if !can_jump || buttons & !(JUMP | FLICK_JUMP) != 0 {
            self.pending = None;
            return Correction::default();
        }
        let mut out = Correction::default();
        if let Some(remaining) = self.pending.take() {
            if remaining == 0 || !within_reach(remaining) {
                out.press = JUMP;
                return out;
            }
            self.pending = Some(remaining - 1);
        } else if buttons & JUMP != 0 && delay != 0 && within_reach(delay) {
            self.pending = Some(delay - 1);
        }
        if self.pending.is_some() {
            out.release = buttons & JUMP;
        }
        out
    }
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

pub fn juggle_drift(facts: LandingFacts, draw: u32) -> Correction {
    let LandingFacts { cpu, player, main_floor: [left, right], stock_stick_y, stock_buttons } = facts;
    if stock_buttons & !FLICK_JUMP != 0 || cpu[1] <= player[1]
        || cpu[0] <= left || cpu[0] >= right
        || !cpu.into_iter().chain(player).chain([left, right, stock_stick_y]).all(f32::is_finite)
    {
        return Correction::default();
    }
    let x = match draw {
        0..=3 if cpu[0] != player[0] => (cpu[0] - player[0]).signum(),
        0 | 2 => -1.0,
        1 | 3 => 1.0,
        4 => 0.0,
        _ => return Correction::default(),
    };
    Correction { stick: Some([x, stock_stick_y]), ..Correction::default() }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn juggle_drift_uses_legal_fall_time_and_leaves_stock_defensive_inputs() {
        let facts = LandingFacts {
            cpu: [5., 45.], player: [0., 5.], main_floor: [-70., 70.],
            stock_stick_y: -0.15, stock_buttons: 0x8000,
        };
        assert_eq!(juggle_drift(facts, 0).stick, Some([1., -0.15]));
        assert_eq!(juggle_drift(facts, 4).stick, Some([0., -0.15]));
        assert_eq!(juggle_drift(LandingFacts { stock_buttons: 0x8008, ..facts }, 0), Correction::default());
        assert_eq!(juggle_drift(LandingFacts { cpu: [70., 45.], ..facts }, 0), Correction::default());
        assert_eq!(juggle_drift(LandingFacts { player: [0., 50.], ..facts }, 0), Correction::default());
    }

    #[test]
    fn recovery_delays_a_stock_jump_then_spends_it_with_a_real_button() {
        let mut timing = JumpTiming::default();
        assert_eq!(timing.step(0x8004, true, 2, |_| true).release, 0x4);
        assert_eq!(timing.step(0x8000, true, 2, |_| true), Correction::default());
        assert_eq!(timing.step(0x8000, true, 2, |_| true).press, 0x4);
        assert_eq!(timing.step(0x8000, true, 2, |_| true), Correction::default());

        assert_eq!(timing.step(0x8004, true, 2, |_| false), Correction::default());
        assert_eq!(timing.step(0x8004, true, 2, |_| true).release, 0x4);
        assert_eq!(timing.step(0x8000, true, 2, |_| false).press, 0x4);
        assert_eq!(timing.step(0x8004, true, 2, |_| true).release, 0x4);
        assert_eq!(timing.step(0x8000, false, 2, |_| true), Correction::default());
        assert_eq!(timing.step(0x8000, true, 2, |_| true), Correction::default());
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
    fn a_reachable_opposite_ledge_is_considered_only_when_it_avoids_the_player() {
        let ledges = [[-40., 0.], [40., 0.]];
        let lines = [[ledges[0], ledges[1]], [ledges[0], [0., -15.]], [[0., -15.], ledges[1]]];
        let mut facts = RouteFacts {
            position: [-53., -22.], player: [-41., 0.], player_reach: 18.,
            ledges, blast: [-130., 130., 100., -90.], outline: &lines,
            body_clearance: 9., ledge_distance: 5., fall_budget: 12.,
            reach: [120., 42.],
        };
        assert!(possible_routes(&facts).iter().flatten().any(|plan| plan.route == Route::Opposite));
        facts.player = [0., 25.];
        assert!(!possible_routes(&facts).iter().flatten().any(|plan| plan.route == Route::Opposite));
        facts.player = [-41., 0.];
        facts.reach[0] = 45.;
        assert!(!possible_routes(&facts).iter().flatten().any(|plan| plan.route == Route::Opposite));
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
