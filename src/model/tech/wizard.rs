//! Transient, allocation-free wizard draft. The runtime owns input edges and
//! discards this draft (including borrowed line ids) when its stage epoch ends.
use super::picker::Spot;
use super::{add, dot, finite, scale, Vec2};
use crate::model::tech_settings::{Config, Mode, Numeric};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Spot,
    Angle,
    Distance,
    Hit,
    Preview,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Next,
    Back,
    Defaults,
    Start,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    None,
    Cancelled,
    Start,
}

pub struct Wizard {
    seed: Config,
    pub step: Step,
    pub selected: Option<Spot>,
    /// Surface-relative degrees; 90 is perpendicular inward. No 5-degree grid.
    pub degrees: f32,
    /// Outward-normal offset, matching the existing planner's Set start.
    pub distance: f32,
    /// Native knockback units, passed unchanged to the existing launch planner.
    pub speed: f32,
    pub hit: f32,
    /// False focuses launch speed; true focuses hit damage.
    pub hit_focus: bool,
    pub skip_preview: bool,
}

fn valid_spot(spot: Spot) -> bool {
    // The surface reader supplies native normals. Use their direction for the
    // angle and preserve their exact values for the existing start model.
    spot.line.valid()
        && spot.line.kind().is_some()
        && finite(spot.point)
        && dot(spot.line.normal, spot.line.normal).is_finite()
        && dot(spot.line.normal, spot.line.normal) > 0.
}
fn set(value: f32) -> Numeric {
    Numeric {
        mode: Mode::Set,
        lo: value,
        hi: value,
    }
}
impl Wizard {
    pub fn begin(seed: Config, skip_preview: bool) -> Self {
        Self {
            seed,
            step: Step::Spot,
            selected: None,
            degrees: 90.,
            distance: 30.,
            speed: 100.,
            hit: 12.,
            hit_focus: false,
            skip_preview,
        }
    }
    /// A rejected candidate leaves the previous valid selection untouched.
    pub fn set_spot(&mut self, spot: Spot) -> bool {
        if !valid_spot(spot) {
            return false;
        }
        self.selected = Some(spot);
        true
    }
    /// Convert the relative approach using t=(ny,-nx), d=t*cos(a)-n*sin(a).
    pub fn world_angle(&self) -> Option<f32> {
        let spot = self.selected.filter(|s| valid_spot(*s))?;
        if !self.degrees.is_finite() || !(0. ..=360.).contains(&self.degrees) {
            return None;
        }
        let a = self.degrees.rem_euclid(360.);
        let (sin, cos) = match a {
            0. => (0., 1.),
            90. => (1., 0.),
            180. => (0., -1.),
            270. => (-1., 0.),
            _ => a.to_radians().sin_cos(),
        };
        let n = spot.line.normal;
        let d = Vec2 {
            x: n.y * cos - n.x * sin,
            y: -n.x * cos - n.y * sin,
        };
        Some(d.y.atan2(d.x).to_degrees().rem_euclid(360.))
    }
    pub fn start_marker(&self) -> Option<Vec2> {
        let spot = self.selected.filter(|s| valid_spot(*s))?;
        if !self.distance.is_finite() || !(0. ..=120.).contains(&self.distance) {
            return None;
        }
        let point = add(spot.point, scale(spot.line.normal, self.distance));
        finite(point).then_some(point)
    }
    /// Exact transient overrides; advanced settings and their sampling remain
    /// in the seed. This does not write quantized full-menu storage indices.
    pub fn config(&self) -> Option<Config> {
        let angle = self.world_angle()?;
        self.start_marker()?;
        if !self.speed.is_finite()
            || !(80. ..=250.).contains(&self.speed)
            || (self.speed - 80.) % 10. != 0.
            || !self.hit.is_finite()
            || !(1. ..=30.).contains(&self.hit)
            || self.hit.fract() != 0.
        {
            return None;
        }
        Some(Config {
            surface: 4,
            angle: set(angle),
            start: set(self.distance),
            speed: set(self.speed),
            hit_damage: set(self.hit),
            ..self.seed
        })
    }
    /// One bounded edit per placing-motion repeat. Zero is neutral; the caller
    /// supplies the existing editor's deadzone/repeat-filtered motion.
    pub fn adjust(&mut self, horizontal: f32, vertical: f32) {
        if self.step == Step::Hit && vertical.is_finite() {
            if vertical > 0. {
                self.hit_focus = false;
            } else if vertical < 0. {
                self.hit_focus = true;
            }
        }
        if !horizontal.is_finite() || horizontal == 0. {
            return;
        }
        let change = if horizontal > 0. { 1. } else { -1. };
        match self.step {
            Step::Angle => self.degrees = (self.degrees + change).rem_euclid(360.),
            Step::Distance => self.distance = (self.distance + change).clamp(0., 120.),
            Step::Hit if self.hit_focus => self.hit = (self.hit + change).clamp(1., 30.),
            Step::Hit => self.speed = (self.speed + 10. * change).clamp(80., 250.),
            Step::Spot | Step::Preview => {}
        }
    }
    pub fn action(&mut self, action: Action) -> Outcome {
        match action {
            Action::Back => {
                self.step = match self.step {
                    Step::Spot => return Outcome::Cancelled,
                    Step::Angle => Step::Spot,
                    Step::Distance => Step::Angle,
                    Step::Hit => Step::Distance,
                    Step::Preview => Step::Hit,
                }
            }
            Action::Defaults => {
                match self.step {
                    Step::Spot | Step::Angle => {
                        self.degrees = 90.;
                        self.distance = 30.;
                        self.speed = 100.;
                        self.hit = 12.;
                    }
                    Step::Distance => {
                        self.distance = 30.;
                        self.speed = 100.;
                        self.hit = 12.;
                    }
                    Step::Hit => {
                        self.speed = 100.;
                        self.hit = 12.;
                    }
                    Step::Preview => {}
                }
                if self.config().is_some() {
                    self.step = Step::Preview;
                }
            }
            Action::Start => {
                if self.config().is_some() {
                    return Outcome::Start;
                }
            }
            Action::Next => {
                if self.config().is_none() {
                    return Outcome::None;
                }
                self.step = match self.step {
                    Step::Spot => Step::Angle,
                    Step::Angle => Step::Distance,
                    Step::Distance => Step::Hit,
                    Step::Hit if self.skip_preview => return Outcome::Start,
                    Step::Hit => Step::Preview,
                    Step::Preview => return Outcome::Start,
                };
            }
        }
        Outcome::None
    }
}
