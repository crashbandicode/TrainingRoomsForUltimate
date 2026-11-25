//! World-space bounds for the two Eggs-ercise recovery zones. No borrowed
//! game collision identity survives into this model.

use super::chords::logical as key;
use super::edge_juggle::perfect::{Outline, Point, Segment};
use super::eggs::RADIUS;
use super::stage_geometry::{classify, floor_run, open_air_end, Copied, Met, Probe, Unread, INSIDE, SLACK};

const EDGE_MARGIN: f32 = 8.0;
const BOTTOM_MARGIN: f32 = 24.0;
const DIAMETER: f32 = 2.0 * RADIUS;
const DEFAULT: [u8; 4] = [128, 128, 255, 128];
/// Other lines a stage keeps at most; a spot's test rebuilds them as `Met` on the stack.
const LINES: usize = 16;
/// The read's columns are this far apart, as the rule's own.
const COLUMN: f32 = 10.0;
/// How far past a line just met the next cast starts.
const STEP: f32 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub const BOTH: [Self; 2] = [Self::Left, Self::Right];
}

/// Validated stage/form and its closed main solid contour.
pub struct Stage {
    id: i32,
    blast: [f32; 4],
    ledges: [Point; 2],
    /// The main solid's closed outline, then the stage's other lines in the zone's columns: solid lines (left end first when a fighter can stand on it, right end first when not), then platforms.
    outline: Outline,
    /// Where each end's open air stops (`stage_geometry::open_air_end`); the ledge's own x for a side with no zone.
    air: [f32; 2],
    /// Segments of the main outline.
    main: u8,
    /// End of the other solid lines; the platforms follow.
    solid: u8,
    available: [bool; 2],
}

/// Physical lines, before egg-radius clearance. Each interval is ascending.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    vertical: [f32; 2],
    horizontal: [Option<[f32; 2]>; 2],
}

impl Zone {
    pub fn horizontal(self, side: Side) -> Option<[f32; 2]> {
        self.horizontal[side as usize]
    }

    #[cfg(target_os = "switch")]
    pub fn horizontal_span(self) -> Option<[f32; 2]> {
        match self.horizontal {
            [Some([left, _]), Some([_, right])] => Some([left, right]),
            [Some(side), None] | [None, Some(side)] => Some(side),
            [None, None] => None,
        }
    }

    pub fn vertical(self) -> [f32; 2] {
        self.vertical
    }

    /// Bounding box of legal egg centers before main-stage clipping.
    pub fn rect(self, side: Side) -> Option<[Point; 2]> {
        let [left, right] = self.horizontal(side)?;
        let [bottom, top] = self.vertical;
        Some([
            [left + RADIUS, bottom + RADIUS],
            [right - RADIUS, top - RADIUS],
        ])
    }
}

/// Piecewise interpolation preserves exact factory and extreme coordinates.
fn value(bin: u8, [lo, default, hi]: [f32; 3]) -> f32 {
    let bin = f32::from(bin);
    if bin <= 128.0 {
        lo + (default - lo) * (bin / 128.0)
    } else {
        default + (hi - default) * ((bin - 128.0) / 127.0)
    }
}

fn bin_ceiling(range: [f32; 3], target: f32) -> u8 {
    let (mut lo, mut hi) = (0, 255);
    while lo < hi {
        let middle = lo + (hi - lo) / 2;
        if value(middle, range) < target {
            lo = middle + 1;
        } else {
            hi = middle;
        }
    }
    lo
}

pub fn blast_rect([left, right, top, bottom]: [f32; 4]) -> [Point; 2] {
    [[left, bottom], [right, top]]
}

/// The stage without the main solid's own lines: a line of the main outline is passed, and the cast goes on under it.
struct Others<'a, P> {
    stage: &'a mut P,
    main: &'a Outline,
}

impl<P: Probe> Probe for Others<'_, P> {
    fn first(&mut self, [x, mut y]: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread> {
        let direction = if to < y { -1.0 } else { 1.0 };
        for _ in 0..64 {
            let Some(line) = self.stage.first([x, y], to, platforms)? else { return Ok(None) };
            let [a, b] = line.ends;
            if !self.main.segments().iter().any(|&s| s == [a, b] || s == [b, a]) {
                return Ok(Some(line));
            }
            y = line.height(x).ok_or(Unread)? + direction * STEP;
            if (to - y) * direction <= 0.0 { return Ok(None); }
        }
        Err(Unread)
    }
}

impl Stage {
    pub fn new(id: i32, blast: [f32; 4], ledges: [Point; 2], mut outline: Outline,
        air: [Option<f32>; 2], others: &[Met]) -> Option<Self> {
        let [left, right, top, bottom] = blast;
        let [[lx, ly], [rx, ry]] = ledges;
        let room = [
            left + EDGE_MARGIN + DIAMETER <= lx && lx <= right - EDGE_MARGIN,
            left + EDGE_MARGIN <= rx && rx + EDGE_MARGIN + DIAMETER <= right,
        ];
        let available = std::array::from_fn(|i| {
            room[i] && air[i].is_some_and(|end| end.is_finite()
                && (end - ledges[i][0]) * [-1.0, 1.0][i] >= DIAMETER)
        });
        let enough_room = available.contains(&true)
            && lx < rx
            && top - EDGE_MARGIN - DIAMETER >= bottom + BOTTOM_MARGIN;
        if !(enough_room
            && blast
                .into_iter()
                .chain(ledges.into_iter().flatten())
                .all(f32::is_finite)
            && [ly, ry].into_iter().all(|y| (bottom..=top).contains(&y))
            && !outline.segments().is_empty()
            && outline.segments().len() <= u8::MAX as usize
            && outline
                .segments()
                .iter()
                .flatten()
                .flatten()
                .all(|v| v.is_finite())) {
            return None;
        }
        let main = outline.segments().len();
        let mut solid = main;
        let mut kept = 0;
        for through in [false, true] {
            for &line in others {
                let [a, b] = line.ends;
                if !a.into_iter().chain(b).all(f32::is_finite) { return None; }
                if line.through != through || a[0] == b[0] || (line.through && !line.stand) {
                    continue;
                }
                if outline.segments().iter().any(|&s| s == [a, b] || s == [b, a]) {
                    continue;
                }
                if kept == LINES || outline.segments().len() == 64 { return None; }
                let left_first = line.through || line.stand;
                let ends = if (a[0] < b[0]) == left_first { [a, b] } else { [b, a] };
                outline.add(ends);
                kept += 1;
            }
            if !through { solid = outline.segments().len(); }
        }
        Some(Self {
            id,
            blast,
            ledges,
            outline,
            air: std::array::from_fn(|i| if available[i] { air[i].unwrap_or(ledges[i][0]) } else { ledges[i][0] }),
            main: main as u8,
            solid: solid as u8,
            available,
        })
    }

    /// The stage as the off-stage rule reads it: the floor run under the camera's middle, each ledge's open air, and the other lines in the zone's columns. `Ok(None)`: no usable zone. `Err`: the stage could not be read.
    pub fn read(id: i32, blast: [f32; 4], camera: [f32; 4], outline: Outline,
        stage: &mut impl Probe) -> Result<Option<Self>, Unread> {
        if !blast.into_iter().chain(camera).all(f32::is_finite) { return Ok(None); }
        let Some(run) = floor_run(stage, blast, [camera[0].midpoint(camera[1]), camera[2]])? else {
            return Ok(None);
        };
        let mut others = Others { stage, main: &outline };
        let mut air = [None; 2];
        for (i, ledge) in run.ledges.into_iter().enumerate() {
            if let Some(ledge) = ledge {
                air[i] = Some(open_air_end(&mut others, blast, ledge)?);
            }
        }
        let Some(bare) = Self::new(id, blast, run.ends, outline, air, &[]) else {
            return Ok(None);
        };
        let empty = Met { ends: [[0.0; 2]; 2], stand: false, through: false };
        let mut lines = [empty; LINES + 1];
        let mut count = 0;
        for i in 0..2 {
            if !bare.available[i] { continue; }
            let dir = [-1.0, 1.0][i];
            let ledge = run.ends[i][0];
            for outward in [true, false] {
                let step = if outward { dir * COLUMN } else { -dir * COLUMN };
                let mut x = ledge + step;
                let end = if outward { bare.air[i] } else { bare.inner_limit()[i] };
                while if outward { (end - x) * dir > 0.0 } else { (x - end) * dir >= 0.0 } {
                    let start = if outward {
                        Some(blast[2] - INSIDE)
                    } else {
                        bare.main_segments().iter().filter_map(|&s| column_at(x, s))
                            .map(|[low, _]| low).reduce(f32::min).map(|low| low - SLACK)
                    };
                    if let Some(mut y) = start {
                        while y > blast[3] {
                            let Some(line) = others.first([x, y], blast[3], true)? else { break };
                            let Some(height) = line.height(x) else { break };
                            let [a, b] = line.ends;
                            if !lines[..count].iter().any(|s| s.ends == [a, b] || s.ends == [b, a]) {
                                lines[count] = line;
                                count += 1;
                                if count > LINES { return Ok(None); }
                            }
                            y = height - STEP;
                        }
                    }
                    x += step;
                }
            }
        }
        Ok(Self::new(id, blast, run.ends, outline, air, &lines[..count]))
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn blast(&self) -> [Point; 2] {
        blast_rect(self.blast)
    }

    pub fn bottom_blast(&self) -> f32 {
        self.blast[3]
    }

    pub fn ledges(&self) -> [Point; 2] {
        self.ledges
    }

    pub fn main_span(&self) -> [f32; 2] {
        self.ledges.map(|p| p[0])
    }

    pub fn outline(&self) -> &Outline {
        &self.outline
    }

    fn main_segments(&self) -> &[Segment] {
        &self.outline.segments()[..usize::from(self.main)]
    }

    pub fn inner_limit(&self) -> [f32; 2] {
        let [left, right] = self.main_span();
        let center = left.midpoint(right);
        [
            left.midpoint(center).min(self.blast[1] - EDGE_MARGIN),
            right.midpoint(center).max(self.blast[0] + EDGE_MARGIN),
        ]
    }

    fn vertical_ranges(&self) -> [[f32; 3]; 2] {
        let [_, _, top, bottom] = self.blast;
        let ledge = self.ledges[0][1].min(self.ledges[1][1]);
        let [lo, hi] = [bottom + BOTTOM_MARGIN, top - EDGE_MARGIN];
        let top_default = ledge.clamp(lo + DIAMETER, hi);
        let bottom_default = (ledge + (bottom - ledge) * 0.45).clamp(lo, top_default - DIAMETER);
        [
            [lo + DIAMETER, top_default, hi],
            [lo, bottom_default, hi - DIAMETER],
        ]
    }

    pub fn resolve(&self, saved: Option<[u8; 4]>) -> Zone {
        let [top, bottom, inner, outer] = saved.unwrap_or(DEFAULT);
        let ranges = self.vertical_ranges();
        let top = value(top, ranges[0]);
        let bottom = value(bottom, ranges[1]).min(top - DIAMETER);
        let ledges = self.main_span();
        let limits = self.inner_limit();
        let horizontal = Side::BOTH.map(|side| {
            let i = side as usize;
            if !self.available[i] {
                return None;
            }
            let direction = [-1.0, 1.0][i];
            let ledge = ledges[i];
            let inside = limits[i] + (ledge - limits[i]) * (f32::from(inner) / 255.0);
            let reach = (self.air[i] - ledge) * direction;
            let outside = ledge + direction * value(outer, self.outer_range(i)).min(reach);
            Some(if side == Side::Left {
                [outside, inside]
            } else {
                [inside, outside]
            })
        });
        Zone {
            vertical: [bottom, top],
            horizontal,
        }
    }

    fn outer_range(&self, i: usize) -> [f32; 3] {
        let distance = (self.blast[i] - self.ledges[i][0]).abs() - EDGE_MARGIN;
        [DIAMETER, (distance * 0.35).max(DIAMETER), distance]
    }

    /// A probe with no segment below it is above the playable main stage even
    /// when its disc misses the actual surface. Inboard off-stage air is underneath.
    pub fn under_main(&self, [x, y]: Point) -> bool {
        let [left, right] = self.main_span();
        (left..=right).contains(&x)
            && self
                .main_segments()
                .iter()
                .all(|s| column_at(x, *s).is_none_or(|[lo, _]| y < lo))
    }

    // A contour can protrude past a ledge farther down, even when the
    // point is comfortably clear of every boundary segment.
    fn inside_main(&self, [x, y]: Point) -> bool {
        self.main_segments()
            .iter()
            .fold(false, |inside, &[[ax, ay], [bx, by]]| {
                inside ^ (((ay > y) != (by > y)) && x < ax + (bx - ax) * ((y - ay) / (by - ay)))
            })
    }

    /// Membership of the exact world segment in the main solid component.
    /// Caller checks the line is a native, non-passable floor first.
    pub fn supports(&self, segment @ [a, b]: Segment) -> bool {
        self.main_segments()
            .iter()
            .any(|&s| s == segment || s == [b, a])
    }

    pub fn contains(&self, zone: &Zone, side: Side, at: Point) -> bool {
        let Some([lo, hi]) = zone.rect(side) else {
            return false;
        };
        let [left, right] = self.main_span();
        let outside = at[0] < left || at[0] > right;
        (0..2).all(|i| (lo[i]..=hi[i]).contains(&at[i]))
            && (if outside {
                !self.inside_main(at)
            } else {
                self.under_main(at)
            })
            && self.off_stage(at)
            && self.outline.segments().iter().all(|&s| clear_of(s, at))
    }

    /// Whether a target centred at `at` is off stage by the rule among the stage's other lines: its centre and its two side points.
    fn off_stage(&self, at: Point) -> bool {
        let empty = Met { ends: [[0.0; 2]; 2], stand: false, through: false };
        let mut lines = [empty; LINES];
        let main = usize::from(self.main);
        let solid = usize::from(self.solid);
        let segments = self.outline.segments();
        let count = segments.len() - main;
        for (i, &ends) in segments[main..].iter().enumerate() {
            let through = main + i >= solid;
            lines[i] = Met { ends, stand: through || ends[0][0] < ends[1][0], through };
        }
        [at, [at[0] - RADIUS, at[1]], [at[0] + RADIUS, at[1]]]
            .into_iter().all(|p| matches!(classify(&mut Copied(&lines[..count]), self.blast, p),
                Ok(place) if place.off_stage(true)))
    }

    /// Endpoints alone miss a thin wall when a path moves quickly. Test the
    /// swept disc against the same stage outlines used by spawn validation.
    pub fn allows_move(&self, zone: &Zone, side: Side, from: Point, to: Point) -> bool {
        self.contains(zone, side, to)
            && self
                .outline
                .segments()
                .iter()
                .all(|&s| !crosses([from, to], s) && s.into_iter().all(|p| clear_of([from, to], p)))
    }

    /// Where a target can appear, for the editor's sample pictures: up to five spots a side (`side as usize * 5 + row`,
    /// rows bottom to top), each on the part of its row that is connected outward.
    pub fn samples(&self, zone: &Zone) -> [Option<Point>; 10] {
        const FRACTIONS: [f32; 5] = [0.35, 0.65, 0.3, 0.5, 0.25];
        std::array::from_fn(|i| {
            let side = Side::BOTH[i / 5];
            let [[left, low], [right, high]] = zone.rect(side)?;
            let [outer, inner] = if side == Side::Left { [left, right] } else { [right, left] };
            let row = i % 5;
            let y = low + (high - low) * ((row + 1) as f32 / 6.0);
            let start = [outer, y];
            // Only sample the air directly connected outward at this height.
            if !self.contains(zone, side, start) {
                return None;
            }
            let end = if self.allows_move(zone, side, start, [inner, y]) {
                inner
            } else {
                let (mut end, mut hidden) = (outer, inner);
                for _ in 0..6 {
                    let mid = end.midpoint(hidden);
                    if self.allows_move(zone, side, start, [mid, y]) {
                        end = mid;
                    } else {
                        hidden = mid;
                    }
                }
                end
            };
            let spot = [outer + FRACTIONS[row] * (end - outer), y];
            Some(if self.contains(zone, side, spot) { spot } else { start })
        })
    }
}

/// The two heights coincide except on a vertical segment.
fn column_at(x: f32, [[ax, ay], [bx, by]]: Segment) -> Option<[f32; 2]> {
    if !(ax.min(bx)..=ax.max(bx)).contains(&x) {
        return None;
    }
    Some(if ax == bx {
        [ay.min(by), ay.max(by)]
    } else {
        [ay + (by - ay) * ((x - ax) / (bx - ax)); 2]
    })
}

fn difference(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: Point, b: Point) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

pub fn clear_of([a, b]: Segment, p: Point) -> bool {
    let ab = difference(b, a);
    let ap = difference(p, a);
    let len = dot(ab, ab);
    let u = if len > 0.0 {
        (dot(ap, ab) / len).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let delta = [ap[0] - u * ab[0], ap[1] - u * ab[1]];
    dot(delta, delta) >= RADIUS * RADIUS
}

fn crosses([a, b]: Segment, [c, d]: Segment) -> bool {
    let area = |a, b, p| {
        let [x, y] = difference(b, a);
        let [px, py] = difference(p, a);
        x * py - y * px
    };
    let straddles = |a, b, c, d| {
        let [p, q] = [area(a, b, c), area(a, b, d)];
        (p > 0.0 && q < 0.0) || (p < 0.0 && q > 0.0)
    };
    straddles(a, b, c, d) && straddles(c, d, a, b)
}

/// One flat rectangle of the crop window, in canvas units (1920 x 1080, origin at the centre, y up): centre and size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropRect { pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub color: [u8; 4] }

/// One view of the zones, the world view or the inset, in canvas units: `vertical` is [bottom, top],
/// `sides[Side as usize]` that side's [left, right], `limits[Side as usize]` the x of that side's inner limit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropView {
    pub vertical: [f32; 2],
    pub sides: [Option<[f32; 2]>; 2],
    pub limits: [f32; 2],
}

/// Five dims, two limits, eight edges and four handle rectangles, then ten inset rectangles.
pub const CROP_RECTS: usize = 29;

const CROP_X: [f32; 2] = [-960.0, 960.0];
const CROP_Y: [f32; 2] = [-540.0, 540.0];
const CROP_DIM: [u8; 4] = [10, 15, 27, 140];
const CROP_BLUE: [u8; 4] = [66, 142, 255, 255];
const CROP_YELLOW: [u8; 4] = [255, 221, 77, 255];
const CROP_PURPLE: [u8; 4] = [206, 132, 255, 255];
const CROP_INK: [u8; 4] = [10, 15, 27, 255];

fn crop_limits(view: CropView, width: f32, push: &mut impl FnMut([f32; 2], [f32; 2], [u8; 4])) {
    for (i, side) in view.sides.into_iter().enumerate() {
        if side.is_some() {
            let x = view.limits[i];
            push([x - width / 2.0, x + width / 2.0], view.vertical, CROP_YELLOW);
        }
    }
}

fn crop_edges(view: CropView, step: Step, thin: f32, active: f32,
    push: &mut impl FnMut([f32; 2], [f32; 2], [u8; 4]))
{
    let [bottom, top] = view.vertical;
    let width = |edge| if step == edge || step == Step::Preview { active } else { thin };
    for (i, side) in view.sides.into_iter().enumerate() {
        let Some([left, right]) = side else { continue };
        let (t, b, inner, outer) = (width(Step::Top), width(Step::Bottom), width(Step::Inner), width(Step::Outer));
        push([left - thin, right + thin], [top, top + t], CROP_BLUE);
        push([left - thin, right + thin], [bottom - b, bottom], CROP_BLUE);
        let (in_x, out_x) = if i == Side::Left as usize {
            ([right, right + inner], [left - outer, left])
        } else {
            ([left - inner, left], [right, right + outer])
        };
        push(in_x, [bottom - thin, top + thin], CROP_YELLOW);
        push(out_x, [bottom - thin, top + thin], CROP_PURPLE);
    }
}

/// The crop window (design 3). The world view: everything outside the two zones dimmed; then, opaque, each side's
/// inner limit, each zone's four boundaries just outside it (the step's one thicker) and a handle on the boundary
/// the step moves. Then the inset's limits and boundaries. The rectangles come back to front, the unused entries last.
pub fn crop_window(world: CropView, inset: Option<CropView>, step: Step) -> [Option<CropRect>; CROP_RECTS] {
    let mut rects = [None; CROP_RECTS];
    let mut count = 0;
    let mut push = |[left, right]: [f32; 2], [bottom, top]: [f32; 2], color| {
        let (w, h) = (right - left, top - bottom);
        if w > 0.0 && h > 0.0 && count < CROP_RECTS {
            rects[count] = Some(CropRect { x: left.midpoint(right), y: bottom.midpoint(top), w, h, color });
            count += 1;
        }
    };
    let [bottom, top] = world.vertical;
    let [low, high] = world.vertical.map(|y| y.clamp(CROP_Y[0], CROP_Y[1]));
    push(CROP_X, [high, CROP_Y[1]], CROP_DIM);
    push(CROP_X, [CROP_Y[0], low], CROP_DIM);
    let mut cursor = CROP_X[0];
    for [left, right] in world.sides.into_iter().flatten() {
        let [left, right] = [left, right].map(|x| x.clamp(CROP_X[0], CROP_X[1]));
        push([cursor, left], [low, high], CROP_DIM);
        cursor = cursor.max(right);
    }
    push([cursor, CROP_X[1]], [low, high], CROP_DIM);
    let (thin, active) = (5.0, 11.0);
    crop_limits(world, 3.0, &mut push);
    crop_edges(world, step, thin, active, &mut push);
    for (i, side) in world.sides.into_iter().enumerate() {
        let Some([left, right]) = side else { continue };
        let (edge_x, edge_y, horizontal, color) = match step {
            Step::Top => ([left - thin, right + thin], [top, top + active], true, CROP_BLUE),
            Step::Bottom => ([left - thin, right + thin], [bottom - active, bottom], true, CROP_BLUE),
            Step::Inner => (if i == 0 { [right, right + active] } else { [left - active, left] },
                [bottom - thin, top + thin], false, CROP_YELLOW),
            Step::Outer => (if i == 0 { [left - active, left] } else { [right, right + active] },
                [bottom - thin, top + thin], false, CROP_PURPLE),
            Step::Preview => continue,
        };
        let [x0, x1] = edge_x.map(|x| x.clamp(CROP_X[0], CROP_X[1]));
        let [y0, y1] = edge_y.map(|y| y.clamp(CROP_Y[0], CROP_Y[1]));
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let (x, y, w, h, iw, ih) = if horizontal {
            (x0.midpoint(x1), edge_y[0].midpoint(edge_y[1]), 36.0, 30.0, 20.0, 14.0)
        } else {
            (edge_x[0].midpoint(edge_x[1]), y0.midpoint(y1), 30.0, 36.0, 14.0, 20.0)
        };
        push([x - w / 2.0, x + w / 2.0], [y - h / 2.0, y + h / 2.0], color);
        push([x - iw / 2.0, x + iw / 2.0], [y - ih / 2.0, y + ih / 2.0], CROP_INK);
    }
    if let Some(inset) = inset {
        crop_limits(inset, 2.0, &mut push);
        crop_edges(inset, step, 2.0, 4.0, &mut push);
    }
    rects
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Top,
    Bottom,
    Inner,
    Outer,
    Preview,
}

impl Step {
    const ALL: [Self; 5] = [
        Self::Top,
        Self::Bottom,
        Self::Inner,
        Self::Outer,
        Self::Preview,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Editing,
    Cancel,
    Apply([u8; 4]),
}

#[derive(Clone, Copy, PartialEq)]
pub struct Wizard {
    bins: [u8; 4],
    step: Step,
}

impl Wizard {
    pub fn new(saved: Option<[u8; 4]>) -> Self {
        Self {
            bins: saved.unwrap_or(DEFAULT),
            step: Step::Top,
        }
    }

    pub fn step(self) -> Step {
        self.step
    }

    pub fn bins(self) -> [u8; 4] {
        self.bins
    }

    /// Whether the notice must be written again after an input: the step changed, or X loaded the factory zone at
    /// Preview, the one step whose notice shows the zone's numbers.
    pub fn prompts_after(self, before: Self) -> bool {
        self.step != before.step || (self.step == Step::Preview && self.bins != before.bins)
    }

    pub fn zone(self, stage: &Stage) -> Zone {
        stage.resolve(Some(self.bins))
    }

    /// Same navigation and stick deadzone as Guard the Edge.
    pub fn input(&mut self, stage: &Stage, stick: Point, pressed: u16) -> Outcome {
        let i = self.step as usize;
        let down = |k| pressed & (1u16 << k) != 0;
        if down(key::B) {
            if i == 0 {
                return Outcome::Cancel;
            }
            self.step = Step::ALL[i - 1];
        } else if down(key::X) {
            self.bins = DEFAULT;
            self.step = Step::Preview;
        } else if self.step == Step::Preview {
            if down(key::A) || down(key::PLUS) {
                return Outcome::Apply(self.bins);
            }
        } else {
            self.adjust(stage, stick);
            if down(key::A) {
                self.step = Step::ALL[i + 1];
            }
        }
        Outcome::Editing
    }

    fn adjust(&mut self, stage: &Stage, stick: Point) {
        let i = self.step as usize;
        let axis = stick[usize::from(i < 2)];
        if axis.abs() <= 0.3 || !axis.is_finite() {
            return;
        }
        // Move a few bins per poll, so even on wide stages a stick tap moves
        // visibly. Outer and inner pairs use one bin across unequal sides.
        let delta = (axis * 4.0).round() as i8;
        let mut bin = self.bins[i].saturating_add_signed(delta);
        if i == 3 {
            let cap = (0..2).filter(|&side| stage.available[side])
                .map(|side| {
                    let reach = (stage.air[side] - stage.ledges[side][0]) * [-1.0, 1.0][side];
                    bin_ceiling(stage.outer_range(side), reach)
                }).max().unwrap_or(255);
            bin = self.bins[i].min(cap).saturating_add_signed(delta).min(cap);
        }
        if i < 2 {
            let [bottom, top] = self.zone(stage).vertical();
            let [top_range, bottom_range] = stage.vertical_ranges();
            if i == 0 {
                let needed = bottom + DIAMETER;
                let first = bin_ceiling(top_range, needed);
                bin = bin.max(first);
            } else {
                let needed = top - DIAMETER;
                let n = bin_ceiling(bottom_range, needed);
                let last = n.saturating_sub(u8::from(value(n, bottom_range) > needed));
                bin = bin.min(last);
            }
        }
        self.bins[i] = bin;
    }
}
