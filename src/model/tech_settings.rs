//! Stage tech practice settings: storage indices are converted into public units here.
use super::settings::Snapshot;
use core::fmt::{self, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Set,
    Random,
    Auto,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Numeric {
    pub mode: Mode,
    pub lo: f32,
    pub hi: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// Off, 20 attempts, 10 attempts, 1 minute, 3 minutes, Free practice.
    pub session: usize,
    /// Any, Floor, Wall, Ceiling, Chosen point.
    pub surface: usize,
    pub damage: Numeric,
    /// Knockback, supplied unchanged to the core's native conversion.
    pub speed: Numeric,
    pub hit_damage: Numeric,
    pub angle: Numeric,
    pub start: Numeric,
    pub time: Numeric,
    pub delay: Numeric,
    /// Off, Player, Set, Random.
    pub di: usize,
    pub di_direction: usize,
    /// Toward the point, Left, Right, Random.
    pub facing: usize,
    /// Follow, Whole stage.
    pub camera: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct Scale {
    pub mode: &'static str,
    pub first: i16,
    pub step: u16,
    pub max: u8,
    pub unit: &'static str,
}
pub fn scale(id: &str) -> Option<Scale> {
    let (mode, first, step, max, unit) = match id {
        "tech_damage_range" => ("tech_damage", 0, 5, 30, "%"),
        "tech_speed_range" => ("tech_speed", 80, 10, 17, " KB"),
        "tech_hit_damage_range" => ("tech_hit_damage", 1, 1, 29, "%"),
        "tech_angle_range" => ("tech_angle", 0, 5, 71, "°"),
        "tech_start_range" => ("tech_start", -120, 1, 240, " units"),
        "tech_time_range" => ("tech_time", 10, 1, 50, "f"),
        "tech_delay_range" => ("tech_delay", 10, 1, 80, "f"),
        _ => return None,
    };
    Some(Scale {
        mode,
        first,
        step,
        max,
        unit,
    })
}
impl Scale {
    pub fn value(self, index: u8) -> f32 {
        f32::from(self.first) + f32::from(index.min(self.max)) * f32::from(self.step)
    }
    pub fn write(self, index: u8, out: &mut impl Write) -> fmt::Result {
        let n = self.value(index);
        write!(out, "{n:.0}{}", self.unit)
    }
}
pub fn selected(s: &Snapshot, id: &str) -> usize {
    s.get(id)
        .and_then(|v| v.iter().position(|x| *x != 0))
        .unwrap_or(0)
}
pub fn mode(s: &Snapshot, id: &str) -> Mode {
    let n = selected(s, id);
    if matches!(id, "tech_angle" | "tech_start") {
        match n {
            0 => Mode::Auto,
            2 => Mode::Random,
            _ => Mode::Set,
        }
    } else if n == 1 {
        Mode::Random
    } else {
        Mode::Set
    }
}
pub fn numeric(s: &Snapshot, id: &str) -> Numeric {
    let spec = scale(id).expect("declared tech range");
    let row = s.get(id).unwrap_or(&[0, 0]);
    Numeric {
        mode: mode(s, spec.mode),
        lo: spec.value(row.first().copied().unwrap_or(0)),
        hi: spec.value(row.get(1).copied().unwrap_or(0)),
    }
}
pub fn read(s: &Snapshot) -> Config {
    Config {
        session: selected(s, "tech_practice"),
        surface: selected(s, "tech_surface"),
        damage: numeric(s, "tech_damage_range"),
        speed: numeric(s, "tech_speed_range"),
        hit_damage: numeric(s, "tech_hit_damage_range"),
        angle: numeric(s, "tech_angle_range"),
        start: numeric(s, "tech_start_range"),
        time: numeric(s, "tech_time_range"),
        delay: numeric(s, "tech_delay_range"),
        di: selected(s, "tech_di"),
        di_direction: selected(s, "tech_di_direction"),
        facing: selected(s, "tech_facing"),
        camera: selected(s, "tech_camera"),
    }
}
/// Menu changes are last-choice-wins; file normalization keeps the older practice game first.
pub fn practice(s: &Snapshot) -> [bool; 3] {
    [
        selected(s, "eggsercise") > 0,
        selected(s, "btt") > 0,
        selected(s, "tech_practice") > 0,
    ]
}

/// Canonical DirectionGrid vector before target-relative mirroring. Left/Right
/// remain absolute; callers mirror x for indices 0..=7 when the point is right.
pub fn di_vector(index: usize) -> super::tech_launch::Vec2 {
    let (x, y) = super::cpu::defence::stick_vector(u8::try_from(index).unwrap_or(8), -1.0, 0.0);
    super::tech_launch::Vec2 { x, y }
}
