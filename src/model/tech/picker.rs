//! BTT placing controls over the complete camera rectangle, including undersides.
use super::surface::{Kind, Line, Source, Surfaces, END_MARGIN};
use super::{add, dot, finite, scale, sub, Vec2};
use crate::model::btt::placing_motion;
use crate::model::btt::Pad;
use crate::model::chords::logical::{A, X, ZL};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub line: Line,
    pub point: Vec2,
}
impl Spot {
    /// A moved line invalidates the old point rather than silently dragging it.
    pub fn current(self, source: &mut impl Source) -> bool {
        source
            .read(self.line.id)
            .is_some_and(|now| self.line.stationary(now))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    None,
    Saved,
    Start,
}
pub struct Picker {
    pub cursor: Vec2,
    pub hovered: Option<Spot>,
    pub selected: Option<Spot>,
    stage: i32,
    // Only world data crosses stage lifetimes; native line ids never do.
    kept: [Option<(i32, Vec2, Option<Kind>)>; 8],
    restore: Option<(Vec2, Option<Kind>)>,
    previous: u16,
    repeat: u8,
}
impl Picker {
    pub const EMPTY: Self = Self {
        cursor: Vec2 { x: 0.0, y: 0.0 },
        hovered: None,
        selected: None,
        stage: -1,
        kept: [None; 8],
        restore: None,
        previous: 0,
        repeat: 0,
    };
    /// Begin discovery for a stage lifetime, including reloads of the same id.
    /// Save only world data and discard every borrowed native-line snapshot.
    pub fn enter(&mut self, stage: i32, here: Vec2) {
        if let Some(spot) = self.selected {
            let remembered = (self.stage, spot.point, spot.line.kind());
            if let Some(entry) = self
                .kept
                .iter_mut()
                .flatten()
                .find(|entry| entry.0 == self.stage)
            {
                *entry = remembered;
            } else {
                self.kept.rotate_left(1);
                self.kept[7] = Some(remembered);
            }
        }
        self.restore = self
            .kept
            .iter()
            .flatten()
            .find(|entry| entry.0 == stage)
            .map(|entry| (entry.1, entry.2));
        self.cursor = self.restore.map(|(point, _)| point).unwrap_or(here);
        self.selected = None;
        self.hovered = None;
        self.stage = stage;
        self.previous = 0;
        self.repeat = 0;
    }
    /// One restoration attempt after current-stage surface discovery completes.
    /// Bind the remembered world point to a fresh targetable line of its kind.
    /// Further calls do not update a same-stage selection or drag a moved line.
    pub fn restore(&mut self, surfaces: &Surfaces) -> bool {
        let Some((point, kind)) = self.restore.take() else {
            return false;
        };
        self.selected = surfaces
            .nearest(point, kind)
            .map(|(line, point)| Spot { line, point });
        self.selected.is_some()
    }
    /// The full camera rectangle is [left,right,top,bottom]; unlike BTT targets,
    /// Tech points have no ground-minus-ten lower limit.
    pub fn feed(
        &mut self,
        pad: Pad,
        surfaces: &Surfaces,
        range: [f32; 4],
        kind: Option<Kind>,
    ) -> Pick {
        if !range.into_iter().all(f32::is_finite) || range[0] >= range[1] || range[3] >= range[2] {
            return Pick::None;
        }
        let motion = self.placing_motion(&pad);
        self.cursor.x = (self.cursor.x + motion[0]).clamp(range[0], range[1]);
        self.cursor.y = (self.cursor.y + motion[1]).clamp(range[3], range[2]);
        self.hovered = surfaces
            .nearest(self.cursor, kind)
            .filter(|(_, p)| {
                p.x >= range[0] && p.x <= range[1] && p.y >= range[3] && p.y <= range[2]
            })
            .map(|(line, point)| Spot { line, point });
        if pad.held & (1 << ZL) != 0 {
            return Pick::None;
        }
        if pad.pressed & (1 << A) != 0 {
            if let Some(spot) = self.hovered {
                self.selected = Some(spot);
                return Pick::Saved;
            }
        }
        if pad.pressed & (1 << X) != 0 && self.selected.is_some() {
            return Pick::Start;
        }
        Pick::None
    }
    /// The wizard's value steps use the same stick deadzone and D-pad repeat.
    pub fn placing_motion(&mut self, pad: &Pad) -> [f32; 2] {
        let motion = placing_motion(pad, self.previous, &mut self.repeat);
        self.previous = pad.held;
        motion
    }
    pub fn normal_arrow(&self) -> Option<[Vec2; 2]> {
        self.hovered
            .map(|s| [s.point, add(s.point, scale(s.line.normal, 5.0))])
    }
}

/// Shared four-marker arrow for the full picker and free-form wizard.
pub fn arrow_points(tip: Vec2, direction: Vec2, stem: f32, head: f32) -> [Vec2; 4] {
    let side = Vec2 {
        x: -direction.y,
        y: direction.x,
    };
    let back = add(tip, scale(direction, -head));
    [
        add(tip, scale(direction, -stem)),
        tip,
        add(back, side),
        add(back, scale(side, -1.)),
    ]
}

/// Draw a line uniformly among currently eligible stationary lines, then a point
/// within its five-unit end margins. Read every candidate fresh on each attempt.
pub fn random_spot<S: Source>(
    surfaces: &Surfaces,
    source: &mut S,
    kind: Option<Kind>,
    range: [f32; 4],
    draw: &mut impl FnMut(u32) -> u32,
    observe: &mut impl FnMut(&mut S, Line),
) -> Option<Spot> {
    if !range.into_iter().all(f32::is_finite) {
        return None;
    }
    let mut chosen = None;
    let mut count = 0;
    for line in surfaces
        .lines()
        .filter(|l| l.kind().is_some() && (kind.is_none() || l.kind() == kind))
    {
        let Some(now) = source.read(line.id) else {
            continue;
        };
        if !line.stationary(now) || line.length() < 2.0 * END_MARGIN {
            continue;
        }
        // Clip the permitted line interval to the camera before sampling; no
        // retry bias and no out-of-camera endpoint can be offered.
        let d = sub(line.ends[1], line.ends[0]);
        let mut low = END_MARGIN / line.length();
        let mut high = 1.0 - low;
        for (base, delta, min, max) in [
            (line.ends[0].x, d.x, range[0], range[1]),
            (line.ends[0].y, d.y, range[3], range[2]),
        ] {
            if delta == 0.0 {
                if base < min || base > max {
                    low = 1.0;
                    high = 0.0;
                }
            } else {
                let a = (min - base) / delta;
                let b = (max - base) / delta;
                low = low.max(a.min(b));
                high = high.min(a.max(b));
            }
        }
        if low > high {
            continue;
        }
        observe(source, now);
        count += 1;
        let choice = draw(count);
        if choice >= count {
            return None;
        }
        if choice == 0 {
            chosen = Some((line, low, high));
        }
    }
    let (line, low, high) = chosen?;
    let unit = draw(65536);
    if unit >= 65536 {
        return None;
    }
    let point = add(
        line.ends[0],
        scale(
            sub(line.ends[1], line.ends[0]),
            low + (high - low) * unit as f32 / 65535.0,
        ),
    );
    (finite(point) && dot(sub(point, line.ends[0]), sub(point, line.ends[0])).is_finite())
        .then_some(Spot { line, point })
}
