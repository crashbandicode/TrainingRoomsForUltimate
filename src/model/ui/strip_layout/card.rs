//! Option C's helper card and the link cells shared by single and sequence drills.

use super::super::cb::{self, at::*};

use super::{strip, Place};

/// Content measurements used only to place captions that can share a row.
#[derive(Clone, Copy)]
pub struct Metrics {
    pub pills: usize,
    pub result: usize,
    pub reason: usize,
    pub previous: usize,
    pub streak: usize,
    pub best: usize,
    pub links: usize,
    pub tolerance: usize,
    pub title: usize,
}

impl Metrics {
    const EMPTY: Self = Self {
        pills: 0,
        result: 0,
        reason: 0,
        previous: 0,
        streak: 0,
        best: 0,
        links: 0,
        tolerance: 0,
        title: 0,
    };
}

/// Logical dimensions of a rendered item. All conversion to canvas units happens here.
#[derive(Clone, Copy)]
pub enum Detail {
    Fixed,
    Chip { units: usize, check: bool },
    Marker(i32),
    Window(i32, i32),
}

#[derive(Clone, Copy)]
pub struct CardSpec {
    pub left: f32,
    pub top: f32,
    pub text_height: f32,
    width: f32,
    heights: [f32; 2],
    inset: f32,
    single_rows: [f32; 3],
    single_extra_rows: [f32; 4],
    sequence_rows: [f32; 5],
    sequence_extra_rows: [f32; 2],
    caption_gap: f32,
    font_widths: [f32; 2],
    pill: [f32; 3],
    sequence_pill: [f32; 2],
    chip: ChipSpec,
    cells: CellSpec,
}

#[derive(Clone, Copy)]
struct ChipSpec {
    pitch: f32,
    height: f32,
    pad: f32,
    stroke: f32,
    check_room: f32,
    check_from_text: f32,
    check_depth: f32,
    /// Length, angle and unit vector for the check's short and long strokes.
    check: [(f32, f32, f32, f32); 2],
    check_stroke: f32,
}

#[derive(Clone, Copy)]
struct CellSpec {
    edge_inset: f32,
    gap: f32,
    center_depth: f32,
    /// Width, height, and vertical offset for each pane in the cell.
    parts: [[f32; 3]; 8],
    grade_dx: [f32; 2],
    ruler_start: i32,
    ruler_frames: i32,
    ruler_step: f32,
    window_min: f32,
}

pub const C: CardSpec = CardSpec {
    left: 660.0,
    top: strip::TOP,
    text_height: 26.0,
    width: 288.0,
    heights: [84.0, 196.0],
    inset: 10.0,
    single_rows: [18.0, 44.0, 68.0],
    // Title and pills split if all the result squares would touch the streak.
    // Result and reason split if they cannot share one line.
    single_extra_rows: [12.0, 28.0, 58.0, 74.0],
    sequence_rows: [20.0, 47.0, 73.0, 160.0, 184.0],
    // When the previous result and the current status cannot share a row, each gets its own, 19 units apart.
    sequence_extra_rows: [146.0, 165.0],
    caption_gap: 6.0,
    font_widths: [
        super::super::iv::FONT_SUB.0 * 0.56,
        super::super::iv::FONT_TAG.0 * 0.56,
    ],
    pill: [14.0, 20.0, 16.0],
    sequence_pill: [156.0, 28.0],
    chip: ChipSpec {
        pitch: 26.0,
        height: 24.0,
        pad: 14.0,
        stroke: 2.0,
        check_room: 24.0,
        check_from_text: 8.0,
        check_depth: 5.0,
        check: [
            (9.0, 135.0, -0.707_106_77, 0.707_106_77),
            (16.0, 50.0, 0.642_787_64, 0.766_044_44),
        ],
        check_stroke: 3.0,
    },
    cells: CellSpec {
        edge_inset: 18.0,
        gap: 8.0,
        center_depth: 32.0,
        parts: [
            [0.0, 56.0, 0.0],
            [0.0, 0.0, 16.0],
            [96.0, 0.0, -2.0],
            [100.0, 0.0, -2.0],
            [180.0, 3.0, -19.0],
            [6.0, 9.0, -19.0],
            [6.0, 16.0, -19.0],
            [6.0, 12.0, -19.0],
        ],
        grade_dx: [-48.0, 50.0],
        ruler_start: -10,
        ruler_frames: 30,
        ruler_step: 6.0,
        window_min: 2.0,
    },
};

impl CardSpec {
    /// Card centre and dimensions relative to TrIv.
    pub const fn bounds(&self, sequence: bool) -> [f32; 4] {
        let h = self.heights[sequence as usize];
        [
            self.left + self.width / 2.0,
            self.top - h / 2.0,
            self.width,
            h,
        ]
    }

    const fn content_left(&self) -> f32 {
        self.left + self.inset
    }
    const fn content_right(&self) -> f32 {
        self.left + self.width - self.inset
    }
    const fn content_width(&self) -> f32 {
        self.width - 2.0 * self.inset
    }

    /// Defaults and native pane dimensions for the archive. Relative placement at runtime uses this same spec.
    pub const fn at_rest(&self, index: usize, strip: &strip::StripSpec) -> (Place, f32) {
        self.base(index, false, strip.band_top(false), strip.width, Metrics::EMPTY)
    }

    fn fits(&self, a: f32, b: f32) -> bool {
        a + self.caption_gap + b <= self.content_width()
    }

    const fn estimating(&self, units: usize, tag: bool) -> f32 {
        units as f32 * self.font_widths[tag as usize]
    }

    const fn text(&self, left: f32, row: f32, width: f32) -> (Place, f32) {
        (
            Place::new(left + width / 2.0, self.top - row, width),
            self.text_height,
        )
    }

    const fn box_at(x: f32, y: f32, w: f32, h: f32) -> (Place, f32) {
        (Place::new(x, y, w), h)
    }

    const fn plate(&self, sequence: bool) -> (Place, f32) {
        let [x, y, width, height] = self.bounds(sequence);
        Self::box_at(x, y, width, height)
    }

    const fn base(
        &self,
        i: usize,
        sequence: bool,
        band_top: f32,
        strip_width: f32,
        metrics: Metrics,
    ) -> (Place, f32) {
        let left = self.content_left();
        let span = self.content_width();
        let [title, progress, result] = self.single_rows;
        let [seq_header, seq_name, seq_chips, seq_result, seq_stats] = self.sequence_rows;
        match i {
            CONTAINER => Self::box_at(0.0, 0.0, self.width, self.heights[1]),
            BACKDROP => self.plate(false),
            HEADER => self.text(left, title, span),
            STREAK => self.text(left, progress, span),
            RESULT | REASON => self.text(
                left,
                if sequence { seq_result } else { result },
                span,
            ),
            i if i < CELL => Self::box_at(
                left + self.pill[0] / 2.0 + (i - PILL) as f32 * self.pill[1],
                self.top - progress,
                self.pill[0],
                self.pill[0],
            ),
            i if i < SEQ_BACKDROP => {
                let (place, height) = self.cells.default_place(i, band_top, strip_width);
                let height = if i >= TITLE && i < RULER { self.text_height } else { height };
                (place, height)
            }
            SEQ_BACKDROP => self.plate(true),
            SEQ_PILL | SEQ_PILL_TEXT => Self::box_at(
                left + self.sequence_pill[0] / 2.0,
                self.top - seq_header,
                self.sequence_pill[0],
                if i == SEQ_PILL_TEXT {
                    super::super::iv::FONT_SUB.1
                } else {
                    self.sequence_pill[1]
                },
            ),
            SEQ_NAME => self.text(left, seq_name, span),
            SEQ_STEP => self.text(
                left + self.sequence_pill[0],
                seq_header,
                span - self.sequence_pill[0],
            ),
            SEQ_LINKS | SEQ_TOLERANCE => self.text(left, seq_stats, span),
            SEQ_STREAK => self.text(left, seq_stats, span),
            SEQ_BEST => {
                let used = self.estimating(metrics.streak, false) + self.caption_gap;
                self.text(left + used, seq_stats, span - used)
            }
            SEQ_LANDED => self.text(left, seq_result, span),
            i => {
                let part = (i - SEQ_CHIP_EDGE) / cb::CHIPS;
                let chip = (i - SEQ_CHIP_EDGE) % cb::CHIPS;
                let row = seq_chips + chip as f32 * self.chip.pitch;
                match part {
                    0 => Self::box_at(left + span / 2.0, self.top - row, span, self.chip.height),
                    1 => Self::box_at(
                        left + span / 2.0,
                        self.top - row,
                        span - self.chip.stroke * 2.0,
                        self.chip.height - self.chip.stroke * 2.0,
                    ),
                    2 => self.text(left + self.chip.pad, row, span - self.chip.pad * 2.0),
                    _ => Self::box_at(
                        left,
                        self.top - row,
                        self.chip.check[part - 3].0,
                        self.chip.check_stroke,
                    ),
                }
            }
        }
    }

    /// Apply the spec to one content item, including ruler spans and caption widths.
    pub fn place(
        &self,
        i: usize,
        sequence: bool,
        strip: &strip::StripSpec,
        history: bool,
        metrics: Metrics,
        detail: Detail,
    ) -> Place {
        let mut place = self.base(i, sequence, strip.band_top(history), strip.width, metrics).0;
        if (CELL..SEQ_BACKDROP).contains(&i) {
            return self.cells.place(place, detail);
        }
        if let Detail::Chip { units, check } = detail {
            return self.place_chip(i, place, units, check);
        }
        if sequence {
            self.place_sequence_text(i, place, metrics)
        } else {
            self.place_single(i, &mut place, metrics);
            place
        }
    }

    fn place_chip(&self, i: usize, mut place: Place, units: usize, check: bool) -> Place {
        let part = (i - SEQ_CHIP_EDGE) / cb::CHIPS;
        let left = self.content_left();
        let text = self.estimating(units, false);
        let check_room = if check { self.chip.check_room } else { 0.0 };
        let width = (self.chip.pad * 2.0 + text + check_room).min(self.content_width());
        match part {
            0 | 1 => {
                place.x = left + width / 2.0;
                place.w = width - part as f32 * self.chip.stroke * 2.0;
            }
            2 => {}
            _ => {
                let (len, angle, cos, sin) = self.chip.check[part - 3];
                let x = left
                    + (self.chip.pad + text + self.chip.check_from_text)
                        .min(width - self.chip.check_room / 2.0);
                place.x = x + cos * len / 2.0;
                place.y += sin * len / 2.0 - self.chip.check_depth;
                place.rot = angle;
            }
        }
        place
    }

    fn place_single(&self, i: usize, place: &mut Place, m: Metrics) {
        let [_, row, footer] = self.single_rows;
        let [title_high, pills_high, result_high, reason_low] = self.single_extra_rows;
        if i == HEADER || (PILL..CELL).contains(&i) {
            let streak_width = self.estimating(m.streak, true);
            let [size, pitch, tight_pitch] = self.pill;
            let occupied = |pitch| size + m.pills.saturating_sub(1) as f32 * pitch;
            let full = self.fits(occupied(pitch), streak_width);
            let split = m.pills > 0 && !self.fits(occupied(tight_pitch), streak_width);
            if i == HEADER && split {
                place.y = self.top - title_high;
            }
            if i != HEADER {
                let pitch = if full || split { pitch } else { tight_pitch };
                place.x = self.content_left() + size / 2.0 + (i - PILL) as f32 * pitch;
                place.y = self.top - if split { pills_high } else { row };
            }
        } else if (i == RESULT || i == REASON) && m.reason != 0 {
            let result_width = self.estimating(m.result, false);
            let reason_width = self.estimating(m.reason, false);
            if self.fits(result_width, reason_width) {
                if i == REASON {
                    *place = self
                        .text(
                            self.content_left() + result_width + self.caption_gap,
                            footer,
                            self.content_width() - result_width - self.caption_gap,
                        )
                        .0;
                }
            } else {
                place.y = self.top - if i == RESULT { result_high } else { reason_low };
            }
        }
    }

    fn place_sequence_text(&self, i: usize, place: Place, m: Metrics) -> Place {
        let left = self.content_left();
        let [_, title, _, footer, stats] = self.sequence_rows;
        let [previous_high, result_low] = self.sequence_extra_rows;
        if (i == RESULT || i == REASON || i == SEQ_LANDED) && m.previous != 0 {
            let prev_width = self.estimating(m.previous, false);
            let item_width = self.estimating(m.result.max(m.reason), false);
            if self.fits(prev_width, item_width) && i != SEQ_LANDED {
                return self
                    .text(
                        left + prev_width + self.caption_gap,
                        footer,
                        self.content_width() - prev_width - self.caption_gap,
                    )
                    .0;
            }
            if !self.fits(prev_width, item_width) {
                return self
                    .text(
                        left,
                        if i == SEQ_LANDED {
                            previous_high
                        } else {
                            result_low
                        },
                        self.content_width(),
                    )
                    .0;
            }
        }
        if i == SEQ_LINKS || i == SEQ_TOLERANCE {
            let links = self.estimating(m.links, true);
            let tol = self.estimating(m.tolerance, true);
            let meta_width = links + self.caption_gap + tol;
            let stats_width = self.estimating(m.streak + m.best, false) + self.caption_gap;
            let row = if self.fits(stats_width, meta_width) {
                stats
            } else if self.fits(self.estimating(m.title, false), meta_width) {
                title
            } else {
                footer
            };
            return if i == SEQ_LINKS {
                self.text(self.content_right() - meta_width, row, meta_width)
                    .0
            } else {
                self.text(left, row, self.content_width()).0
            };
        }
        place
    }
}

impl CellSpec {
    const fn default_place(&self, i: usize, band_top: f32, strip_width: f32) -> (Place, f32) {
        let k = (i - CELL) % cb::CELLS;
        let part = (i - CELL) / cb::CELLS;
        let pitch = (strip_width - 2.0 * self.edge_inset + self.gap) / cb::CELLS as f32;
        let width = pitch - self.gap;
        let cx = self.edge_inset - strip_width / 2.0 + width / 2.0 + k as f32 * pitch;
        let [w, h, dy] = self.parts[part];
        let w = match part {
            0 => width,
            1 => width - self.gap,
            _ => w,
        };
        let dx = match part {
            2 => self.grade_dx[0],
            3 => self.grade_dx[1],
            _ => 0.0,
        };
        CardSpec::box_at(cx + dx, band_top - self.center_depth + dy, w, h)
    }

    fn ruler(&self, cx: f32, frames: i32) -> f32 {
        cx - self.parts[4][0] / 2.0
            + (frames - self.ruler_start).clamp(0, self.ruler_frames) as f32 * self.ruler_step
    }

    fn place(&self, mut place: Place, detail: Detail) -> Place {
        match detail {
            Detail::Marker(frames) => place.x = self.ruler(place.x, frames),
            Detail::Window(a, b) => {
                let (a, b) = (self.ruler(place.x, a), self.ruler(place.x, b));
                place.x = (a + b) / 2.0;
                place.w = (b - a).max(self.window_min);
            }
            _ => {}
        }
        place
    }
}
