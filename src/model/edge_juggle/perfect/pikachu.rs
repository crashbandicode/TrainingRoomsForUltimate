//! Bounded Quick Attack geometry for Pikachu's Skull Bash crossing.

use super::{
    niche::{Motion, Move, Sample, Sequence},
    recovery::Zone,
    Correction, Outline, Point, Route, Segment,
};

#[derive(Clone, Copy, Debug)]
pub struct ZipPlan {
    pub first: Point,
    pub second: Option<Point>,
}

// The traced first zip and vl.prc second-zip length, minimum turn and motion limits.
// See phase23/edge-juggle/reference-crossings/MODEL.md §4 for the candidate margins.
const FIRST: f32 = 55.0;
const SECOND: f32 = 49.5;
const MIN_TURN: f32 = 40.0;
const SLACK: f32 = 4.0;
const GRAB_RADIUS: f32 = 5.0;
const HALF_BODY: f32 = 1.5;

fn dot(a: Point, b: Point) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn difference(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}

fn advance(from: Point, direction: Point, distance: f32) -> Point {
    [from[0] + distance * direction[0], from[1] + distance * direction[1]]
}

fn clear(outline: &[Segment], path: Segment, bottom: f32, height: f32) -> bool {
    super::aerial_clear(outline, path, bottom + SLACK, height + 1.0, HALF_BODY)
}

fn second_zip(
    outline: &[Segment],
    from: Point,
    target: Point,
    first: Point,
    bottom: f32,
    height: f32,
) -> Option<(f32, Point)> {
    let [dx, dy] = difference(target, from);
    let distance = dx.hypot(dy);
    if !(distance > 0.0 && distance <= SECOND - SLACK) {
        return None;
    }
    let direction = [dx / distance, dy / distance];
    let turn = dot(first, direction).clamp(-1.0, 1.0).acos().to_degrees();
    let margin = (SECOND - distance).min(turn - MIN_TURN);
    if margin < SLACK || !clear(outline, [from, target], bottom, height) {
        return None;
    }
    Some((margin, direction))
}

/// Aim one or two zips from the actual apex. The second zip must have spare travel
/// and a different angle, and neither segment may carry the body through the stage.
pub fn quick_attack(
    outline: &[Segment],
    from: Point,
    target: Point,
    bottom: f32,
    height: f32,
) -> Option<ZipPlan> {
    if outline.is_empty()
        || !from.into_iter().chain(target).chain([bottom, height]).all(f32::is_finite)
        || !outline.iter().flatten().flatten().all(|v| v.is_finite())
        || height <= 0.0
    {
        return None;
    }
    let mut best = None;
    let mut room = SLACK;
    let toward = difference(target, from);
    for step in 0..72 {
        let angle = (-90.0 + 5.0 * step as f32).to_radians();
        let (sine, cosine) = angle.sin_cos();
        let first = [cosine, sine];
        let projection = dot(toward, first);
        if projection > 0.0 && projection <= FIRST - room {
            let catch = advance(from, first, projection);
            let [dx, dy] = difference(catch, target);
            let margin = (GRAB_RADIUS - dx.hypot(dy)).min(FIRST - projection);
            if margin >= room && clear(outline, [from, catch], bottom, height) {
                best = Some(ZipPlan { first, second: None });
                room = margin;
            }
        }
        let end = advance(from, first, FIRST);
        if let Some((margin, second)) = second_zip(outline, end, target, first, bottom, height) {
            if margin >= room && clear(outline, [from, end], bottom, height) {
                best = Some(ZipPlan { first, second: Some(second) });
                room = margin;
            }
        }
    }
    best
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkullBashPath {
    pub direction: f32,
    pub launch_y: f32,
    pub near: Point,
    pub far: Point,
    pub height: f32,
}

// The 195 candidate cells from reference-crossings/gen_regions.py, P-SB on the common ring.
// Mirrored at runtime. The live outline and blast line must still match.
// The frame after a press can fall another 2.48 units (edge-ref-pika-sA2d-1).
const SKULL_BASH_RING: [u32; 44] = [
    0x0000, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc,
    0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc,
    0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc,
    0x01fc, 0x003c, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000,
];

/// Start outside the lip in a cell baked for the live normal-form floor and blast line.
pub fn skull_bash_start(
    stage: (i32, u32),
    outline: &[Segment],
    [left, right]: [Point; 2],
    [x, y]: Point,
    bottom: f32,
    height: f32,
) -> Option<SkullBashPath> {
    const MODEL_HEIGHT: f32 = 10.0;
    if !super::is_crossing_stage(stage)
        || !(height > 0.0 && height <= MODEL_HEIGHT)
        || !left.into_iter().chain(right).chain([x, y, bottom]).all(f32::is_finite)
        || !outline.iter().flatten().flatten().all(|v| v.is_finite())
    {
        return None;
    }
    let [near, far] = if x < left[0] { [left, right] } else { [right, left] };
    let direction = (far[0] - near[0]).signum();
    let outside = (near[0] - x) * direction;
    let y = y - near[1];
    let underside = outline.iter().flatten().map(|point| point[1]).reduce(f32::min)?;
    if ((right[0] - left[0]) * 0.5 - 79.99).abs() > 0.5
        || (right[1] - left[1]).abs() > 0.5
        || (underside - near[1] + 40.1).abs() > 1.0
        || (bottom - near[1] + 140.0).abs() > 1.0
    {
        return None;
    }
    if !super::region::contains(&SKULL_BASH_RING, outside, y) {
        return None;
    }
    Some(SkullBashPath {
        direction,
        // edge-ref-pika-1: drop 3.64 before flight, then rise 1.82 above
        // its start during flight. Leave two units between the body and the roof.
        launch_y: underside - MODEL_HEIGHT - 2.0 + 3.64 - 1.82,
        near: [near[0] - direction * 1.7, near[1] - 9.9],
        far: [far[0] + direction * 1.7, far[1] - 9.9],
        height: MODEL_HEIGHT,
    })
}

pub struct SkullBashFlight {
    path: SkullBashPath,
    outline: Outline,
    bottom: f32,
    bash_committed: bool,
    thunder_approach: bool,
    pub thunder: Option<thunder::Thunder>,
    jump_accepted: bool,
    zips: Option<ZipPlan>,
}

impl SkullBashFlight {
    pub fn new(path: SkullBashPath, outline: Outline, bottom: f32) -> Self {
        Self {
            path,
            outline,
            bottom,
            bash_committed: false,
            thunder_approach: false,
            thunder: None,
            jump_accepted: false,
            zips: None,
        }
    }

    pub fn with_measured_thunder(mut self, ledges: [Point; 2], position: Point, speed: Point, facing: f32) -> Self {
        self.thunder_approach = self.path.direction == 1.0
            && facing == 1.0
            && thunder::measured_approach(position, speed, ledges[0])
            && super::region::has_common_underside(ledges, self.bottom, self.outline.segments());
        self
    }

    pub fn observe_bash_pad(&mut self, press: thunder::Bash) {
        if self.thunder_approach && self.thunder.is_none() {
            let near = [self.path.near[0] + 1.7, self.path.near[1] + 9.9];
            self.thunder = thunder::Thunder::from_bash(press, near);
        }
    }

    pub fn into_fake_out(self) -> FakeOutFlight {
        FakeOutFlight::new(self.path, self.outline, self.bottom)
    }

    pub fn step(
        &mut self, sequence: &mut Sequence, mut sample: Sample, position: Point, vy: f32, jumps: u8,
    ) -> Option<Correction> {
        use super::niche::{self, Motion, Move};
        let movement = sequence.action()?;
        let outline = self.outline.segments();
        let path = self.path;
        if !sample.valid
            || sample.motion == Motion::Interrupted
            || !position.into_iter().chain([vy]).all(f32::is_finite)
            || !clear(outline, [position; 2], self.bottom, path.height)
        {
            return None;
        }
        if movement == Move::Side && !self.bash_committed {
            let ledge_x = path.near[0] + path.direction * 1.7;
            let ledge_y = path.near[1] + 9.9;
            let outside = (ledge_x - position[0]) * path.direction;
            if sample.motion != Motion::Free
                || jumps == 0
                || !super::region::contains(&SKULL_BASH_RING, outside, position[1] - ledge_y)
            {
                return None;
            }
            if self.thunder_approach && thunder::above_bash_line(position[1], ledge_y) {
                return Some(niche::pad([0.0; 2], 0));
            }
            if position[1] > path.launch_y {
                // Release before the side-special frame. Before this point, the full fall is reversible.
                return Some(niche::pad([0.0, -1.0], 0));
            }
            if !sample.can_start {
                return None;
            }
            self.bash_committed = true;
        }
        if movement == Move::Jump {
            if sample.motion == Motion::Travel {
                self.jump_accepted = true;
                if vy <= 0.0 {
                    sample.motion = Motion::Free;
                }
            } else if !self.jump_accepted && (jumps == 0 || !sample.can_start) {
                return None;
            }
        }
        if movement == Move::QuickAttack && self.zips.is_none() {
            if sample.motion != Motion::Free || !sample.can_start {
                return None;
            }
            let solve = |target| quick_attack(outline, position, target, self.bottom, path.height);
            self.zips = Some(solve(path.far).or_else(|| solve(path.near))?);
        }
        sample.direction = path.direction;
        sample.at_exit = true;
        sample.can_enter = true;
        let mut input = sequence.step(sample)?;
        if movement == Move::QuickAttack {
            let zips = self.zips?;
            input.stick = match sample.motion {
                Motion::Starting | Motion::Travel => Some(zips.first),
                Motion::AimAgain | Motion::Finish => Some(zips.second.unwrap_or([0.0; 2])),
                _ => input.stick,
            };
        }
        Some(input)
    }
}

/// Keep the crossing roll intact. Some near-side returns feint against a P1
/// on stage, with a higher share after P1 has seen a crossing.
pub fn fake_out_selected(normal: Option<Route>, zone: Zone, previous: Option<Route>, draw: u32) -> bool {
    if !matches!(normal, Some(Route::Near | Route::LateLow)) || !matches!(zone, Zone::Ledge | Zone::Mid) {
        return false;
    }
    let share = if previous == Some(Route::Opposite) { 20 } else { 10 };
    u64::from(draw.rotate_left(13)) < (1u64 << 32) * share / 100
}

#[derive(Default)]
struct FakeOutDrift {
    braking: bool,
    stopped: bool,
}

impl FakeOutDrift {
    fn stick(&mut self, distance: f32, speed: f32) -> f32 {
        // The jump begins at 0.90 on a full stick; reversal takes 0.10 off
        // the live speed per frame (Surface F1–F3, MODEL.md §4.5).
        const ACCELERATION: f32 = 0.10;
        const AT_REST: f32 = ACCELERATION * 0.5;
        self.braking |= speed.abs() > AT_REST
            && (distance * speed <= 0.0
                || distance.abs() <= speed * speed / (2.0 * ACCELERATION));
        self.stopped |= speed.abs() <= AT_REST && (self.braking || distance.abs() <= 0.5);
        if self.stopped {
            0.0
        } else if self.braking {
            -speed.signum()
        } else {
            distance.signum()
        }
    }
}

fn fake_out_zip(
    outline: &[Segment], from: Point, ledge: Point, inward: f32, bottom: f32, height: f32,
) -> Option<Point> {
    // Give the ordinary fall catch its window. Once that closes, aim an actual
    // zip step into the measured near catch band (MODEL.md §4.5).
    if !(from[1] <= ledge[1] - 24.0
        && inward.abs() == 1.0
        && from.into_iter().chain(ledge).all(f32::is_finite))
    {
        return None;
    }
    const STEPS: u8 = 5;
    const STEP: f32 = FIRST / STEPS as f32;
    const MIN_CATCH_MARGIN: f32 = 1.5;
    for angle in (20u8..=90).step_by(5).rev() {
        let aim = if angle == 90 {
            [0.0, 1.0]
        } else {
            let (sin, cos) = f32::from(angle).to_radians().sin_cos();
            [inward * cos, sin]
        };
        for step in 1..=STEPS {
            let end = advance(from, aim, STEP * f32::from(step));
            if !clear(outline, [from, end], bottom, height) {
                break;
            }
            let outside = (ledge[0] - end[0]) * inward;
            let below = ledge[1] - end[1];
            let margin = (outside - 2.7)
                .min(13.3 - outside)
                .min(below - 9.1)
                .min(18.45 - below);
            if margin >= MIN_CATCH_MARGIN {
                return Some(aim);
            }
        }
    }
    None
}

#[derive(Clone, Copy)]
pub struct FakeOutSample {
    pub frame: u32,
    pub motion: Motion,
    pub position: Point,
    pub velocity: Point,
    pub jumps: u8,
    pub can_start: bool,
}

#[derive(Clone, Copy)]
enum FakeOutPhase {
    Approach,
    JumpPressed(u32),
    Drift,
    Zip { aim: Point, pressed_at: u32, accepted: bool },
}

pub struct FakeOutFlight {
    path: SkullBashPath,
    outline: Outline,
    bottom: f32,
    phase: FakeOutPhase,
    drift: FakeOutDrift,
    last: Option<(u32, Correction)>,
}

// Leave the native double jump room to apex within the 19.7-unit catch depth
// after one more full fast-fall frame (MODEL.md §4.5).
const FAKE_OUT_JUMP_DEPTH: f32 = 46.0;

impl FakeOutFlight {
    fn new(path: SkullBashPath, outline: Outline, bottom: f32) -> Self {
        Self {
            path,
            outline,
            bottom,
            phase: FakeOutPhase::Approach,
            drift: FakeOutDrift::default(),
            last: None,
        }
    }

    pub fn movement(&self) -> Move {
        if matches!(self.phase, FakeOutPhase::Zip { .. }) {
            Move::QuickAttack
        } else {
            Move::Jump
        }
    }

    pub fn start_check(&self, position: Point, vertical_speed: f32) -> Option<Move> {
        match self.phase {
            FakeOutPhase::Approach if position[1] <= self.near_ledge()[1] - FAKE_OUT_JUMP_DEPTH => {
                Some(Move::Jump)
            }
            FakeOutPhase::Drift if position[1] <= self.near_ledge()[1] - 24.0 && vertical_speed <= 0.0 => {
                Some(Move::QuickAttack)
            }
            _ => None,
        }
    }

    pub fn near_ledge(&self) -> Point {
        [self.path.near[0] + 1.7 * self.path.direction, self.path.near[1] + 9.9]
    }

    pub fn step(&mut self, sample: FakeOutSample) -> Option<Correction> {
        let outline = self.outline.segments();
        if sample.motion == Motion::Interrupted
            || !sample.position.into_iter().chain(sample.velocity).all(f32::is_finite)
            || !clear(outline, [sample.position; 2], self.bottom, self.path.height)
        {
            return None;
        }
        if let Some((frame, correction)) = self.last {
            if frame == sample.frame {
                return Some(correction);
            }
        }
        let correction = self.on_frame(sample)?;
        self.last = Some((sample.frame, correction));
        Some(correction)
    }

    fn on_frame(&mut self, sample: FakeOutSample) -> Option<Correction> {
        use super::niche::pad;
        let inward = self.path.direction;
        let ledge = self.near_ledge();
        let outside = (ledge[0] - sample.position[0]) * inward;
        match self.phase {
            FakeOutPhase::Approach => {
                if sample.motion != Motion::Free
                    || sample.jumps == 0
                    || !super::region::contains(&SKULL_BASH_RING, outside, sample.position[1] - ledge[1])
                {
                    return None;
                }
                if sample.position[1] > ledge[1] - FAKE_OUT_JUMP_DEPTH {
                    return Some(pad([0.0, -1.0], 0));
                }
                if !sample.can_start {
                    return None;
                }
                self.phase = FakeOutPhase::JumpPressed(sample.frame);
                let jump_stick = if outside > 12.0 { inward } else { 0.0 };
                return Some(pad([jump_stick, 0.0], super::JUMP));
            }
            FakeOutPhase::JumpPressed(frame) => match sample.motion {
                Motion::Travel => self.phase = FakeOutPhase::Drift,
                Motion::Free if sample.frame.saturating_sub(frame) <= 1 => {
                    return Some(pad([0.0; 2], 0));
                }
                _ => return None,
            },
            FakeOutPhase::Drift => {}
            FakeOutPhase::Zip { aim, pressed_at, accepted } => {
                return match sample.motion {
                    Motion::Free if !accepted && sample.frame.saturating_sub(pressed_at) <= 1 => {
                        Some(pad([0.0; 2], 0))
                    }
                    Motion::Starting | Motion::Travel => {
                        self.phase = FakeOutPhase::Zip { aim, pressed_at, accepted: true };
                        Some(pad(aim, 0))
                    }
                    Motion::AimAgain | Motion::Finish => Some(pad([0.0; 2], 0)),
                    _ => None,
                };
            }
        }
        if !matches!(sample.motion, Motion::Free | Motion::Travel) {
            return None;
        }
        if sample.can_start && sample.velocity[1] <= 0.0 {
            if let Some(aim) = fake_out_zip(
                self.outline.segments(), sample.position, ledge, inward, self.bottom, self.path.height,
            ) {
                self.phase = FakeOutPhase::Zip { aim, pressed_at: sample.frame, accepted: false };
                return Some(pad([0.0, 1.0], super::SPECIAL));
            }
        }
        let stick = self.drift.stick(outside - 7.0, sample.velocity[0] * inward);
        Some(pad([stick * inward, 0.0], 0))
    }
}

#[path = "pikachu_thunder.rs"]
pub mod thunder;

#[cfg(test)]
lib_unit_tests! {
    #[path = "../../../../tests/common/edge_juggle_pikachu.rs"]
    mod tests;
}
