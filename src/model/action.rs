//! An action starts at an originating input and owns its later events.
//! Its model is described in `docs/design/action-model.md`.

use super::combo::{self, Dir, Kind, Mark};
use super::frame_strip::{Angle, Move, Shown};
use super::move_names::{Namer, Normal};

/// What the strip, the combo bar, the menu and the grader all read about one move. It holds kinds, never names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionDesc {
    pub mv: Move,
    pub of: Option<Move>,
    pub angle: Option<Angle>,
    pub normal: u8,
    pub parts: [Option<Dir>; 2],
}

impl ActionDesc {
    /// From the tracker's move, its `Shown` and its variant.
    pub fn of(mv: Move, shown: Shown, normal: u8) -> ActionDesc {
        let angled = matches!(mv, Move::SideTilt | Move::SideSmash);
        ActionDesc {
            mv,
            of: if mv == Move::Other { shown.of } else { None },
            angle: if angled { match shown.dirs[0] {
                Some(2) => Some(Angle::Up),
                Some(6) => Some(Angle::Down),
                _ => None,
            } } else { None },
            normal,
            parts: if angled { [None; 2] } else { shown.dirs.map(|dir| dir.map(|dir| combo::facing_dir(Some(dir), 1))) },
        }
    }

    /// Write the name as `namer` gives it, then the angle and parts, omitting spaces before arrows when tight.
    pub fn write(&self, out: &mut impl core::fmt::Write, tight: bool, namer: Namer<'_>) -> core::fmt::Result {
        let mv = self.of.unwrap_or(self.mv);
        let name = if mv == Move::FinalSmash { namer.final_smash() } else { namer.name(mv, Normal::from_u8(self.normal)) };
        out.write_str(name.as_str())?;
        let angle = self.angle.map(|angle| match angle { Angle::Up => Dir::Up, Angle::Down => Dir::Down });
        for dir in [angle, self.parts[0], self.parts[1]].into_iter().flatten() {
            let arrow = arrow(dir);
            if !arrow.is_empty() {
                if !tight { out.write_str(" ")?; }
                out.write_str(arrow)?;
            }
        }
        Ok(())
    }

    /// Compare move, continuation and angle, without variant or travel.
    pub fn same_move(&self, other: &ActionDesc) -> bool {
        (self.mv, self.of, self.angle) == (other.mv, other.of, other.angle)
    }
}

/// What the game decided on one frame for an earlier input. The drill capture attaches it to that input's event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Late {
    /// An action opened or a side attack's angle changed: the originating press (`None` if unseen) and the angle
    /// (`Some(None)` for a level side attack).
    pub open: Option<(Option<u32>, Option<Option<Angle>>)>,
    /// The originating press and its known move, base and variant.
    pub named: Option<(Option<u32>, Move, Option<Move>, u8)>,
    /// On a later aim part, the move that began the cast.
    pub continuation: Option<Move>,
    /// The part that ended on this frame and its measured travel. Part 0 is the special's own travel.
    pub travel: Option<(u8, Travel)>,
    /// With part 0 travel or a continuation: the press that began that special.
    pub press: u32,
}

impl Late {
    pub const NONE: Late = Late { open: None, travel: None, named: None, continuation: None, press: 0 };
}

/// Facing-relative entry velocity, in 1/256 game units per unstopped frame. Unknown is distinct from standing still.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry(i16, i16, u16);

pub const ENTRY_BASE: u16 = 16; // 1/16 unit/frame in 1/256 units.
pub const ENTRY_CHANGE_PER_FRAME: u16 = 1;

impl Entry {
    pub const UNKNOWN: Self = Self(i16::MIN, i16::MIN, 0);

    pub fn of(x: f64, y: f64) -> Option<Self> {
        let axis = |v: f64| {
            let n = (v * 256.0).round();
            ((-32767.0..=32767.0).contains(&n)).then_some(n as i16)
        };
        Some(Self(axis(x)?, axis(y)?, 0))
    }

    pub fn units(self) -> Option<(f64, f64)> {
        (self != Self::UNKNOWN).then(|| (f64::from(self.0) / 256.0, f64::from(self.1) / 256.0))
    }

    pub fn change(self) -> f64 { f64::from(self.2) / 256.0 }

    pub fn with_change(self, change: f64) -> Option<Self> {
        let n = (change * 256.0).round();
        ((0.0..=65535.0).contains(&n) && change >= 0.0).then_some(Self(self.0, self.1, n as u16))
    }

    fn comparable(self, other: Self, frames: u8) -> bool {
        let (x, y) = (i64::from(self.0) - i64::from(other.0), i64::from(self.1) - i64::from(other.1));
        // Either entry can be the later sample of an accelerating fall.
        let radius = i64::from(ENTRY_BASE) + i64::from(self.2.max(other.2)) * i64::from(frames) * i64::from(ENTRY_CHANGE_PER_FRAME);
        self != Self::UNKNOWN && other != Self::UNKNOWN && x * x + y * y <= radius * radius
    }
}

/// A travel part: angles are facing-relative degrees in tenths, path offsets are facing-relative game units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Travel {
    pub entry: Entry,
    /// The deciding stick's angle and deflection in percent.
    pub stick: Option<(u16, u8)>,
    /// The whole part's heading and straight-line distance in tenths of a unit.
    pub heading: u16,
    pub distance: u16,
    /// Eight points at equal intervals along the path.
    pub path: [(i8, i8); 8],
}

pub const PATH_ABS: f32 = 6.0;
pub const PATH_REL: f32 = 0.30;
pub const ANGLE_TOL: i16 = 10;
pub const DEFLECTION_TOL: i16 = 15;
/// Two own travels of a special count as steered differently only when some path point is farther apart than this.
pub const STEER_FLOOR: f32 = 2.0;

/// Whether some path point of the attempt is farther from the reference's than the tolerance.
fn apart(reference: &Travel, attempt: &Travel, tolerance: f32) -> bool {
    reference.path.iter().zip(attempt.path).any(|(&(x, y), (a, b))| {
        (f32::from(a) - f32::from(x)).hypot(f32::from(b) - f32::from(y)) > tolerance
    })
}

pub fn path_off(reference: &Travel, attempt: &Travel) -> bool {
    let mut length = 0.0;
    let mut previous = (0.0, 0.0);
    for &(x, y) in &reference.path {
        let point = (f32::from(x), f32::from(y));
        length += (point.0 - previous.0).hypot(point.1 - previous.1);
        previous = point;
    }
    apart(reference, attempt, PATH_ABS.max(PATH_REL * length))
}

/// The eight-way direction a kept stick is held in; `None` inside the dead zone (`aim::DEAD`) or when none was kept.
fn sector(stick: Option<(u16, u8)>) -> Option<u16> {
    let dead = (super::aim::DEAD * 100 / 120) as u8;
    stick.filter(|&(_, deflection)| deflection >= dead).map(|(angle, _)| (angle + 225) / 450 % 8)
}

/// The fault of a special's own travel against the saved one, once neither path is off: the stick most held is
/// compared eight-way, and with the toggle On its angle and deflection too. Only when the two paths differ by more
/// than [`STEER_FLOOR`]: a special that ignores the stick has no fault here.
pub fn steer_off(reference: &Travel, attempt: &Travel, match_stick: bool, frames: u8) -> Option<Mark> {
    if !reference.entry.comparable(attempt.entry, frames) || !apart(reference, attempt, STEER_FLOOR) {
        return None;
    }
    let saved = sector(reference.stick);
    if saved != sector(attempt.stick) {
        return Some(Mark::Steer);
    }
    if match_stick && saved.is_some() { stick_off(reference, attempt) } else { None }
}

pub fn stick_off(reference: &Travel, attempt: &Travel) -> Option<Mark> {
    let ((angle, deflection), (actual, magnitude)) = (reference.stick?, attempt.stick?);
    let delta = (i32::from(actual) - i32::from(angle) + 1800).rem_euclid(3600) - 1800;
    if delta.abs() > i32::from(ANGLE_TOL) * 10 {
        return Some(Mark::Angle(delta as i16));
    }
    let delta = i16::from(magnitude) - i16::from(deflection);
    (delta.abs() > DEFLECTION_TOL).then_some(Mark::Deflection(delta as i8))
}

pub fn stick_of(stick: (i8, i8), facing: i8) -> (u16, u8) {
    let x = f32::from(i16::from(stick.0) * if facing < 0 { -1 } else { 1 });
    let y = f32::from(stick.1);
    let angle = ((y.atan2(x).to_degrees().rem_euclid(360.0) * 10.0).round() as u16) % 3600;
    let deflection = (x.hypot(y) / 120.0 * 100.0).round().clamp(0.0, 100.0) as u8;
    (angle, deflection)
}

/// A press's direction with the vertical part the animation chose.
pub const fn angled(dir: Dir, angle: Option<Angle>) -> Dir {
    let back = matches!(dir, Dir::Back | Dir::BackUp | Dir::BackDown);
    match (back, angle) {
        (false, None) => Dir::Forward,
        (true, None) => Dir::Back,
        (false, Some(Angle::Up)) => Dir::ForwardUp,
        (true, Some(Angle::Up)) => Dir::BackUp,
        (false, Some(Angle::Down)) => Dir::ForwardDown,
        (true, Some(Angle::Down)) => Dir::BackDown,
    }
}

/// The current action and the input that opened it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    pub id: u32,
    pub start: u32,
    pub mv: Move,
    pub base: Option<Move>,
    pub angle: Option<Angle>,
    pub shown: Shown,
    pub shown_id: u32,
    pub variant: u8,
    pub origin: Option<u32>,
    pub(crate) variant_known: bool,
    pub(crate) told: Option<Option<Angle>>,
    pub(crate) told_named: Option<(Move, Option<Move>, u8)>,
}

impl Action {
    pub const fn new() -> Self {
        Self { id: 0, start: 0, mv: Move::Other, base: None, angle: None,
            shown: Shown { of: None, dirs: [None; 2] }, shown_id: 0, variant: 0, origin: None, variant_known: false,
            told: None, told_named: None }
    }
}

impl Default for Action {
    fn default() -> Self { Self::new() }
}

/// The move variant identified by an animation: the move names' number for it (`Normal`), 0 when it is none of the
/// engine-common ones.
pub fn variant(motion: u64) -> u8 { Normal::of(motion) as u8 }

/// A font-supported arrow for a direction.
pub const fn arrow(dir: Dir) -> &'static str {
    match dir {
        Dir::Forward => "→", Dir::Back => "←", Dir::Up => "↑", Dir::Down => "↓",
        Dir::ForwardUp => "↑→", Dir::BackUp => "↑←",
        Dir::ForwardDown => "↓→", Dir::BackDown => "↓←",
        Dir::Neutral | Dir::Side => "",
    }
}

/// Whether this event can start a new action.
pub const fn originates(kind: Kind) -> bool {
    matches!(kind, Kind::Press(_) | Kind::CStick | Kind::Flick(_))
}

/// Extra tolerance for a hold, capped before the next idle-separated step.
pub const fn hold_band(hold: u32) -> u8 {
    if hold / 8 > 15 { 15 } else { (hold / 8) as u8 }
}
