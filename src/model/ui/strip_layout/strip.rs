use super::super::{
    container, hidden, iv, rect, text, under, Align, Name, Spec, ROOT_PANE, RULE, TEXT,
};
use super::Place;

pub const HISTORY_H: f32 = 170.0;
pub const BASE_H: f32 = 138.0;
pub const STRIP_Y: f32 = 420.0;
/// The strip keeps its existing origin. Distances from the top use y-down layout units.
pub const TOP: f32 = 108.0;
pub const STRIP_W: f32 = 1296.0;
pub const STRIP_H: f32 = HISTORY_H;
pub const NO_HISTORY_H: f32 = BASE_H;
/// The left edge of the oldest frame's cell, the distance between frames and a cell's width.
pub const FIRST_X: f32 = -564.0;
pub const PITCH: f32 = 13.5;
pub const CELL_W: f32 = 12.0;
pub const NOW_X: f32 = FIRST_X + super::super::iv::WINDOW as f32 * PITCH + 3.0;
pub const LABEL_X: f32 = -606.0;
pub const IN_Y: f32 = TOP - 21.0;
pub const P1_Y: f32 = TOP - 56.0;
pub const CPU_Y: f32 = TOP - 92.0;
pub const LANE_H: f32 = 16.0;
pub const STICK_Y: f32 = TOP - 74.0;
pub const CHIP: f32 = 30.0;
pub const TICK_Y: f32 = TOP - 42.0;
pub const TICK_H: f32 = 12.0;
pub const WINDOW_Y: f32 = TOP - 42.0;
pub const WINDOW_H: f32 = 7.5;
pub const LEG_Y: f32 = TOP - 107.5;
pub const LEG_H: f32 = 9.0;
pub const BAR_Y: f32 = LEG_Y - 4.5;
pub const BRACKET_TEXT_Y: f32 = LEG_Y - 18.0;
pub const READ_X: f32 = 276.0;
pub const READ_VALUE_Y: [f32; 3] = [TOP - 26.0, TOP - 68.5, TOP - 111.0];
pub const READ_SUB_X: f32 = READ_X + 75.0;
/// The units the folded LINK line is fitted to: what its box holds beside the readout.
pub const LINK_UNITS: u16 = 32;
pub const DIVIDER_X: f32 = 262.5;
pub const DIVIDER_Y: f32 = TOP - 69.0;
pub const DIVIDER_H: f32 = 128.0;
pub const NOW_Y: f32 = TOP - 52.0;
pub const NOW_H: f32 = 96.0;
pub const NOW_TEXT_Y: f32 = TOP - 11.0;
pub const LAST_Y: f32 = TOP - 153.5;
pub const PILL_W: f32 = 96.0;
pub const PILL_STRIDE: f32 = 300.0;
/// The move names start a gap after their pill and end before the next pill.
pub const MOVE_GAP: f32 = 8.0;
pub const MOVE_W: f32 = PILL_STRIDE - PILL_W - MOVE_GAP - 4.0;
pub const PILL_X0: f32 = FIRST_X + PILL_W / 2.0;

#[derive(Clone, Copy)]
pub struct StripSpec {
    pub origin_x: f32,
    pub origin_y: f32,
    pub width: f32,
    pub top: f32,
    pub base_height: f32,
    pub history_height: f32,
    pub cells_height: f32,
    label_x: f32,
    lanes: [f32; 3],
    divider: Place,
    history_rule_inset: f32,
    first_frame: f32,
    pitch: f32,
    cell_width: f32,
    stick_y: f32,
    now: [Place; 2],
    chip: f32,
    tick_y: f32,
    window_y: f32,
    legs: [f32; 3],
    readout_left: [f32; 2],
    readout_rows: [f32; 3],
    history: [f32; 6],
    tag_widths: [f32; 3],
}

pub const C: StripSpec = StripSpec {
    origin_x: 0.0,
    origin_y: STRIP_Y,
    width: STRIP_W,
    top: TOP,
    base_height: BASE_H,
    history_height: HISTORY_H,
    cells_height: 64.0,
    label_x: LABEL_X,
    lanes: [IN_Y, P1_Y, CPU_Y],
    divider: Place::new(DIVIDER_X, DIVIDER_Y, 1.5),
    history_rule_inset: 18.0,
    first_frame: FIRST_X,
    pitch: PITCH,
    cell_width: CELL_W,
    stick_y: STICK_Y,
    now: [
        Place::new(NOW_X, NOW_Y, 1.5),
        Place::new(NOW_X - 24.0, NOW_TEXT_Y, 0.0),
    ],
    chip: CHIP,
    tick_y: TICK_Y,
    window_y: WINDOW_Y,
    legs: [LEG_Y, BAR_Y, BRACKET_TEXT_Y],
    readout_left: [READ_X, READ_SUB_X],
    readout_rows: READ_VALUE_Y,
    history: [LAST_Y, PILL_X0, PILL_STRIDE, PILL_W, MOVE_GAP, MOVE_W],
    tag_widths: [92.0, 54.0, 40.0],
};

const DIRECTIONS: [(f32, f32); 8] = [
    (1.0, 0.0),
    (0.707_106_77, 0.707_106_77),
    (0.0, 1.0),
    (-0.707_106_77, 0.707_106_77),
    (-1.0, 0.0),
    (-0.707_106_77, -0.707_106_77),
    (0.0, -1.0),
    (0.707_106_77, -0.707_106_77),
];

impl StripSpec {
    pub const fn band_top(&self, history: bool) -> f32 {
        self.top
            - if history {
                self.history_height
            } else {
                self.base_height
            }
    }

    pub const fn bounds(&self, history: bool, cells: bool) -> [f32; 4] {
        let height = self.top - self.band_top(history);
        let h = height + if cells { self.cells_height } else { 0.0 };
        [0.0, self.top - h / 2.0, self.width, h]
    }

    pub const fn container(&self) -> Place {
        Place::new(self.origin_x, self.origin_y, 0.0)
    }

    pub const fn lane_label(&self, lane: usize) -> Place {
        Place::new(self.label_x, self.lanes[lane], 0.0)
    }

    pub const fn rule(&self, rule: usize) -> Place {
        if rule == 0 {
            self.divider
        } else {
            Place::new(0.0, self.band_top(false), self.width - 2.0 * self.history_rule_inset)
        }
    }

    pub const fn now(&self, part: usize) -> Place {
        self.now[part]
    }

    fn edge(&self, slot: f32) -> f32 {
        self.first_frame + slot * self.pitch
    }

    fn centre(&self, slot: f32) -> f32 {
        self.edge(slot) + self.cell_width / 2.0
    }

    pub fn lane_run(&self, lane: usize, slot: usize, len: usize, label: bool) -> Place {
        let left = self.edge(slot as f32);
        let w = len as f32 * self.pitch - (self.pitch - self.cell_width);
        let x = left + if label { len as f32 * self.pitch } else { w } / 2.0;
        Place::new(x, self.lanes[lane + 1], if label { 0.0 } else { w })
    }

    pub fn stick(&self, slot: usize, dir: u8) -> Place {
        Place {
            rot: f32::from(dir) * 45.0,
            ..Place::new(self.centre(slot as f32), self.stick_y, 0.0)
        }
    }

    pub fn tag(&self, slot: f32, lane: usize, grade: usize) -> Place {
        Place::new(
            self.centre(slot),
            self.lanes[lane + 1],
            self.tag_widths[grade],
        )
    }

    pub fn chip(&self, slot: f32, stick: Option<u8>, part: usize) -> Place {
        let x = self.centre(slot);
        let y = self.lanes[0];
        let dir = stick.unwrap_or(0);
        let rot = f32::from(dir) * 45.0 - 90.0;
        let (c, s) = DIRECTIONS[usize::from(dir) % DIRECTIONS.len()];
        let arrow_x = x - self.chip * 0.45;
        match part {
            0 => Place::new(x, y, self.chip * if stick.is_some() { 1.9 } else { 1.0 }),
            1 => Place::new(
                if stick.is_some() {
                    x + self.chip * 0.45
                } else {
                    x
                },
                y,
                0.0,
            ),
            2 => Place {
                rot,
                ..Place::new(arrow_x - 2.0 * c, y - 2.0 * s, 4.5)
            },
            3 => Place {
                rot: rot + 45.0,
                ..Place::new(arrow_x + 5.0 * c, y + 5.0 * s, 9.0)
            },
            _ => Place::new(x, self.tick_y, 1.5),
        }
    }

    pub fn window(&self, a: f32, b: f32, text: bool) -> Place {
        let left = self.edge(a);
        let right = self.edge(b) + self.cell_width;
        if text {
            Place::new(right + 60.0, self.window_y, 0.0)
        } else {
            Place::new((left + right) / 2.0, self.window_y, right - left)
        }
    }

    pub fn bracket(&self, a: f32, b: f32, part: Option<usize>) -> Place {
        let (left, right) = (self.edge(a), self.edge(b));
        let [leg, bar, text] = self.legs;
        match part {
            Some(0) => Place::new(left, leg, 3.0),
            Some(1) => Place::new((left + right) / 2.0, bar, (right - left).max(3.0)),
            Some(_) => Place::new(right, leg, 3.0),
            None => Place::new((left + right) / 2.0, text, 0.0),
        }
    }

    const fn link_box_width(&self) -> f32 {
        self.width / 2.0 - self.readout_left[1] - 12.0
    }

    pub const fn readout(&self, row: usize, part: usize) -> Place {
        let y = self.readout_rows[row];
        let [value, label] = self.readout_left;
        let w = match part {
            0 => 120.0,
            1 => 60.0,
            _ if row == 1 => self.link_box_width(),
            _ => 180.0,
        };
        let (left, y) = match part {
            0 => (label, y + 11.0),
            1 => (value, y),
            _ => (label, y - 10.0),
        };
        Place::new(left + w / 2.0, y, w)
    }

    pub const fn last_label(&self) -> Place {
        Place::new(self.label_x, self.history[0], 0.0)
    }

    pub const fn last(&self, part: usize, n: usize) -> Place {
        let [y, x0, stride, w, gap, moves] = self.history;
        let x = x0 + n as f32 * stride;
        match part {
            0 => Place::new(x, y, w),
            1 => Place::new(x, y, 0.0),
            _ => Place::new(x + w / 2.0 + gap + moves / 2.0, y, moves),
        }
    }
}

/// A hidden text box of the frame strip.
const fn iv_text(name: Name, [x, y, w]: [f32; 3], align: Align, font: (f32, f32)) -> Spec {
    hidden(under(
        text(name, [x, y, w, 30.0], align, font, iv::SHORT_UNITS, TEXT),
        Name::new("TrIv"),
    ))
}

/// The recipes seed visible anchors from the same coordinates as the live strip. Hidden frame slots
/// start empty and are positioned by their slot on the first live update.
pub(in super::super) const fn iv_spec(spec: &StripSpec, local: usize, name: Name) -> Spec {
    use iv::at::*;
    use iv::{BACKDROP_COLOR, FONT_SUB, FONT_TAG, FONT_VALUE, SHORT_UNITS, TEXT_UNITS};
    let parent = Name::new("TrIv");
    match local {
        CONTAINER => container(
            name,
            ROOT_PANE,
            [
                spec.origin_x,
                spec.origin_y,
                spec.width,
                spec.history_height,
            ],
        ),
        BACKDROP => under(
            rect(name, spec.bounds(true, false), BACKDROP_COLOR, true),
            parent,
        ),
        i if i < SEPARATOR => iv_text(
            name,
            with_width(spec.lane_label(i - LANE_LABEL), 60.0),
            Align::Center,
            FONT_TAG,
        ),
        SEPARATOR => under(
            rect(name, with_height(spec.rule(0), DIVIDER_H), RULE, false),
            parent,
        ),
        i if i < RUN => under(
            rect(name, with_height(spec.rule(1), 1.5), RULE, false),
            parent,
        ),
        i if i < RUN_LABEL => under(
            rect(name, [0.0, 0.0, spec.cell_width, LANE_H], TEXT, false),
            parent,
        ),
        i if i < CHIP_BOX => iv_text(name, [0.0, 0.0, 60.0], Align::Center, FONT_TAG),
        i if i < CHIP_GLYPH => under(
            rect(
                name,
                [0.0, spec.lanes[0], spec.chip, spec.chip],
                TEXT,
                false,
            ),
            parent,
        ),
        i if i < CHIP_SHAFT => hidden(under(
            text(
                name,
                [0.0, spec.lanes[0], spec.chip, spec.chip],
                Align::Center,
                FONT_SUB,
                2,
                TEXT,
            ),
            parent,
        )),
        i if i < CHIP_HEAD => under(
            rect(name, [0.0, spec.lanes[0], 4.5, 14.0], TEXT, false),
            parent,
        ),
        i if i < CHIP_TICK => under(
            rect(name, [0.0, spec.lanes[0], 9.0, 9.0], TEXT, false),
            parent,
        ),
        i if i < WINDOW_BAR => under(
            rect(name, [0.0, spec.tick_y, 1.5, TICK_H], TEXT, false),
            parent,
        ),
        WINDOW_BAR => under(
            rect(name, [0.0, spec.window_y, 1.5, WINDOW_H], TEXT, false),
            parent,
        ),
        WINDOW_TEXT => iv_text(name, [0.0, spec.window_y, 120.0], Align::Center, FONT_TAG),
        i if i < BRACKET_TEXT && (i - BRACKET) % 3 == 1 => under(
            rect(name, [0.0, spec.legs[1], 3.0, 3.0], TEXT, false),
            parent,
        ),
        i if i < BRACKET_TEXT => under(
            rect(name, [0.0, spec.legs[0], 3.0, LEG_H], TEXT, false),
            parent,
        ),
        i if i < NOW => iv_text(name, [0.0, spec.legs[2], 120.0], Align::Center, FONT_TAG),
        NOW => under(
            rect(name, with_height(spec.now(0), NOW_H), TEXT, false),
            parent,
        ),
        NOW_TEXT => iv_text(name, with_width(spec.now(1), 48.0), Align::Center, FONT_TAG),
        i if i < READ_VALUE => {
            let place = spec.readout(i - READ_LABEL, 0);
            iv_text(name, [place.x, place.y, place.w], Align::Left, FONT_TAG)
        }
        i if i < READ_SUB => hidden(under(
            text(
                name,
                with_height(spec.readout(i - READ_VALUE, 1), 40.0),
                Align::Left,
                FONT_VALUE,
                SHORT_UNITS,
                TEXT,
            ),
            parent,
        )),
        i if i < LAST_TEXT => hidden(under(
            text(
                name,
                with_height(spec.readout(i - READ_SUB, 2), 30.0),
                Align::Left,
                FONT_SUB,
                TEXT_UNITS,
                TEXT,
            ),
            parent,
        )),
        LAST_TEXT => iv_text(
            name,
            with_width(spec.last_label(), 60.0),
            Align::Center,
            FONT_TAG,
        ),
        i if i < PILL_TEXT => under(
            rect(name, with_height(spec.last(0, i - PILL), 24.0), TEXT, false),
            parent,
        ),
        i if i < MOVE_TEXT => iv_text(
            name,
            with_width(spec.last(1, i - PILL_TEXT), spec.history[3]),
            Align::Center,
            FONT_TAG,
        ),
        i if i < STICK => {
            let place = spec.last(2, i - MOVE_TEXT);
            iv_text(name, [place.x, place.y, place.w], Align::Left, FONT_TAG)
        }
        i if i < TAG_BOX => hidden(under(
            text(
                name,
                [0.0, spec.stick_y, 16.0, 18.0],
                Align::Center,
                FONT_TAG,
                2,
                TEXT,
            ),
            parent,
        )),
        i if i < TAG_TEXT => hidden(under(
            rect(
                name,
                [0.0, 0.0, spec.tag_widths[0], LANE_H + 4.0],
                TEXT,
                false,
            ),
            parent,
        )),
        i if i < COUNT => iv_text(
            name,
            [0.0, 0.0, spec.tag_widths[0]],
            Align::Center,
            FONT_TAG,
        ),
        _ => panic!("no such frame strip pane"),
    }
}

const fn with_height(place: Place, height: f32) -> [f32; 4] {
    [place.x, place.y, place.w, height]
}

const fn with_width(place: Place, width: f32) -> [f32; 3] {
    [place.x, place.y, width]
}
