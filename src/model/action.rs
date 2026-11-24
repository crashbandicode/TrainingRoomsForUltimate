//! An action starts at an originating input and owns its later events.
//! Its model is described in `docs/design/action-model.md`.

use super::combo::{self, Dir, Kind};
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
        out.write_str(namer.name(self.of.unwrap_or(self.mv), Normal::Unknown).as_str())?;
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
}

impl Late {
    pub const NONE: Late = Late { open: None };
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
}

impl Action {
    pub const fn new() -> Self {
        Self { id: 0, start: 0, mv: Move::Other, base: None, angle: None,
            shown: Shown { of: None, dirs: [None; 2] }, shown_id: 0, variant: 0, origin: None, variant_known: false,
            told: None }
    }
}

impl Default for Action {
    fn default() -> Self { Self::new() }
}

/// The move variant identified by an animation.
#[cfg(not(test))]
pub fn variant(_motion: u64) -> u8 { 0 }

/// Test stand-in for the move-name join (`Normal::of`): `catch` is Grab (18), `throw_f` is Forward throw (22).
#[cfg(test)]
pub fn variant(motion: u64) -> u8 {
    match motion { 0x05_c56b_7b64 => 18, 0x07_49f7_09a8 => 22, _ => 0 }
}

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
