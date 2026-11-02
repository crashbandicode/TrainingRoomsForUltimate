//! The input display (SPEC-input-02): a column of up to fifteen rows on the left of the Training HUD,
//! one row per distinct input with the frames it was held and, optionally, the fighter status. Pure:
//! the logger (the P1 poll) and the renderer (the HUD tick) are both here as data; the runtime reads the
//! game and writes the panes the render plan names.
//!
//! Built to the fork's optimised renderer behaviour from the start (SPEC-input-02 §5.2): the renderer
//! keeps what it last wrote per physical row and returns only what changed, writes a row other than
//! the newest only after a full refresh, retains no pane pointer, and invalidates its cache whenever
//! the layout is registered. The plan is a fixed buffer, so planning allocates nothing.
//!
//! Open items (spec question 1, 3, 5): status names are the numeric id until the lead decides how the
//! names are produced (`StatusText`); the "strong" stick tier is not produced; a missing row pane
//! disables the display for the session instead of panicking (`Renderer::disable`).

use super::input_delay::MappedInputs;

/// Rows in the column.
pub const NUM_LOGS: usize = 15;
/// A new row's time to live in in-game frames: opaque for 400, fading for 200.
pub const ROW_TTL: u32 = 600;
/// The frame count shown is capped here.
pub const FRAME_CAP: u32 = 99;
/// Vertical distance between rows in layout units.
pub const ROW_PITCH: f32 = 52.5;
/// Icon slots per row.
pub const ICON_SLOTS: usize = 5;
/// A row is fully opaque at or above this ttl.
pub const FADE_START: u32 = 200;
/// The stick thresholds (SPEC-input-02 §3.3).
pub const WALK_THRESHOLD: i16 = 20;
pub const DEAD_ZONE: i16 = 30;

/// The pane names the display writes, inside the plugin's own `info_training` part (SPEC-core-05).
pub mod panes {
    pub const COLUMN: &str = "TrModInputLog";
    /// `TrModInputLog0` .. `TrModInputLog14`.
    pub const ROW_PREFIX: &str = "TrModInputLog";
    pub const FRAME_TEXT: &str = "Frame";
    pub const STATUS_TEXT: &str = "Status";
    /// `Input0` .. `Input4`: one text box per icon slot (a glyph in the slot's colour).
    pub const ICON_SLOT_PREFIX: &str = "Input";
}

/// The display mode (`input_display`: None, Smash Inputs, Raw Inputs, Status Only).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    None,
    #[default]
    Smash,
    Raw,
    StatusOnly,
}

/// The two settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub mode: Mode,
    pub status: bool,
}

impl Settings {
    pub const fn factory() -> Self {
        Self { mode: Mode::Smash, status: false }
    }

    pub fn from_snapshot(snapshot: &super::settings::Snapshot) -> Self {
        let mode = match snapshot.get("input_display").and_then(|r| r.iter().position(|v| *v != 0)) {
            Some(0) => Mode::None,
            Some(2) => Mode::Raw,
            Some(3) => Mode::StatusOnly,
            _ => Mode::Smash,
        };
        let status = snapshot.get("input_display_status").and_then(|r| r.first()).is_some_and(|v| *v != 0);
        Self { mode, status }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::factory()
    }
}

/// An icon the layout holds (37 per slot, SPEC-input-02 §3.4); only the ones the display produces are
/// variants here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    A,
    B,
    X,
    Y,
    Lb,
    Rb,
    Zl,
    Zr,
    Plus,
    Minus,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Right,
    UpRight,
    Up,
    UpLeft,
    Left,
    DownLeft,
    Down,
    DownRight,
}

impl Icon {
    /// The picture pane's name in the layout.
    pub const fn name(self) -> &'static str {
        match self {
            Icon::A => "a",
            Icon::B => "b",
            Icon::X => "x",
            Icon::Y => "y",
            Icon::Lb => "lb",
            Icon::Rb => "rb",
            Icon::Zl => "zl",
            Icon::Zr => "zr",
            Icon::Plus => "plus",
            Icon::Minus => "minus",
            Icon::DpadUp => "dpad_up",
            Icon::DpadDown => "dpad_down",
            Icon::DpadLeft => "dpad_left",
            Icon::DpadRight => "dpad_right",
            Icon::Right => "right",
            Icon::UpRight => "up_right",
            Icon::Up => "up",
            Icon::UpLeft => "up_left",
            Icon::Left => "left",
            Icon::DownLeft => "down_left",
            Icon::Down => "down",
            Icon::DownRight => "down_right",
        }
    }
    /// The glyph the slot's text box shows.
    pub const fn glyph(self) -> Glyph {
        match self {
            Icon::A => Glyph::Font(0xE0E0),
            Icon::B => Glyph::Letter(0x42),
            Icon::X => Glyph::Font(0xE0E2),
            Icon::Y => Glyph::Font(0xE0E3),
            Icon::Lb => Glyph::Font(0xE0E4),
            Icon::Rb => Glyph::Font(0xE0E5),
            Icon::Zl => Glyph::Font(0xE0E6),
            Icon::Zr => Glyph::Font(0xE0E7),
            Icon::Plus => Glyph::Font(0xE0EF),
            Icon::Minus => Glyph::Font(0xE0F0),
            Icon::DpadUp => Glyph::Font(0xE079),
            Icon::DpadDown => Glyph::Font(0xE07A),
            Icon::DpadLeft => Glyph::Font(0xE07B),
            Icon::DpadRight => Glyph::Font(0xE07C),
            Icon::Right => Glyph::Arrow(0),
            Icon::UpRight => Glyph::Arrow(45),
            Icon::Up => Glyph::Arrow(90),
            Icon::UpLeft => Glyph::Arrow(135),
            Icon::Left => Glyph::Arrow(180),
            Icon::DownLeft => Glyph::Arrow(225),
            Icon::Down => Glyph::Arrow(270),
            Icon::DownRight => Glyph::Arrow(315),
        }
    }
}

/// An icon's colour: the text colour of its slot (RGB, sRGB bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    pub const GREEN: Color = Color(22, 156, 0);
    pub const RED: Color = Color(153, 10, 10);
    pub const CYAN: Color = Color(0, 255, 255);
    pub const BLUE: Color = Color(0, 40, 108);
    pub const PURPLE: Color = Color(100, 66, 202);
    pub const YELLOW: Color = Color(230, 180, 14);
    pub const WHITE: Color = Color(255, 255, 255);
}

/// What an icon slot's text box shows (lead decision 2026-11-02: no picture panes, each slot is a text
/// box holding one glyph of the game's own font, or the plugin's arrow glyph for a stick direction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    /// A private-use code point of the game's font (SPEC-core-03 §4.3, Pro / Joy-Con set).
    Font(u16),
    /// A plain character where the font has no glyph (B: "no distinct glyph upstream"; the colour
    /// tells it from A).
    Letter(u16),
    /// The plugin's own arrow glyph (package 5): the stick direction in degrees, 0 right .. 315.
    Arrow(u16),
}

impl Glyph {
    /// The UTF-16 unit written into the text box, for the font and letter glyphs; the arrow is the
    /// layout owner's (`None`).
    pub const fn code_unit(self) -> Option<u16> {
        match self {
            Glyph::Font(c) | Glyph::Letter(c) => Some(c),
            Glyph::Arrow(_) => None,
        }
    }
}
/// One icon slot's content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub icon: Icon,
    pub color: Color,
}

/// The icons of a row, in slot order (at most five). Each is written as text: the slot's glyph in the slot's colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Icons {
    slots: [Option<Slot>; ICON_SLOTS],
}

impl Icons {
    pub const fn none() -> Self {
        Self { slots: [None; ICON_SLOTS] }
    }

    pub fn slots(&self) -> &[Option<Slot>; ICON_SLOTS] {
        &self.slots
    }

    pub fn count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }
}

/// A stick direction (SPEC-input-02 §3.3): the angle in degrees of the direction icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Direction(pub u16);

impl Direction {
    pub const fn icon(self) -> Option<Icon> {
        Some(match self.0 {
            0 => Icon::Right,
            45 => Icon::UpRight,
            90 => Icon::Up,
            135 => Icon::UpLeft,
            180 => Icon::Left,
            225 => Icon::DownLeft,
            270 => Icon::Down,
            315 => Icon::DownRight,
            _ => return None,
        })
    }
}

/// Bin a stick (signed units, SPEC-input-02 §3.3): thresholds X 20, Y 30, in the spec's rule order.
pub fn bin_stick(x: i8, y: i8) -> Option<Direction> {
    let (x, y) = (i16::from(x), i16::from(y));
    let angle = if y.abs() < DEAD_ZONE {
        if x > WALK_THRESHOLD {
            0
        } else if x < -WALK_THRESHOLD {
            180
        } else {
            return None;
        }
    } else if x.abs() < WALK_THRESHOLD {
        if y > DEAD_ZONE {
            90
        } else if y < -DEAD_ZONE {
            270
        } else {
            return None;
        }
    } else if y > DEAD_ZONE {
        if x > WALK_THRESHOLD {
            45
        } else if x < -WALK_THRESHOLD {
            135
        } else {
            90
        }
    } else if x > WALK_THRESHOLD {
        315
    } else if x < -WALK_THRESHOLD {
        225
    } else {
        270
    };
    Some(Direction(angle))
}

/// A raw stick axis (`f32`, ±1.0) as signed units: scaled by 120 and truncated.
pub fn raw_axis(v: f32) -> i8 {
    (v * 120.0).clamp(-128.0, 127.0) as i8
}

/// The raw controller's part the display reads: the current-buttons set and the binned-stick inputs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RawInput {
    pub buttons: u32,
    pub lstick: (i8, i8),
    pub rstick: (i8, i8),
}

/// Raw button bits (the `Controller` set, SPEC-input-04 §3.2).
mod raw_bit {
    pub const A: u32 = 1 << 5;
    pub const B: u32 = 1 << 6;
    pub const X: u32 = 1 << 4;
    pub const Y: u32 = 1 << 7;
    pub const L: u32 = 1 << 8;
    pub const R: u32 = 1 << 9;
    pub const ZL: u32 = 1 << 10;
    pub const ZR: u32 = 1 << 11;
    pub const PLUS: u32 = 1 << 18;
    pub const MINUS: u32 = 1 << 19;
    pub const DPAD_UP: u32 = 1 << 0;
    pub const DPAD_RIGHT: u32 = 1 << 1;
    pub const DPAD_DOWN: u32 = 1 << 2;
    pub const DPAD_LEFT: u32 = 1 << 3;
    pub const REAL_DIGITAL_L: u32 = 1 << 28;
    pub const REAL_DIGITAL_R: u32 = 1 << 29;
}

/// Mapped ("Smash") button bits (SPEC-input-01 §3.1).
mod smash_bit {
    pub const ATTACK: u32 = 0x1;
    pub const SPECIAL: u32 = 0x2;
    pub const JUMP: u32 = 0x4;
    pub const GUARD: u32 = 0x8;
    pub const CATCH: u32 = 0x10;
    pub const STOCK_SHARE: u32 = 0x100;
    pub const ATTACK_RAW: u32 = 0x200;
    pub const APPEAL_HI: u32 = 0x400;
    pub const APPEAL_LW: u32 = 0x1000;
    pub const APPEAL_SL: u32 = 0x2000;
    pub const APPEAL_SR: u32 = 0x4000;
    pub const GUARD_HOLD: u32 = 0x10000;
    pub const SPECIAL_RAW2: u32 = 0x20000;
}

/// Everything the poll gives the logger for one row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Candidate {
    pub raw: RawInput,
    pub smash: MappedInputs,
    pub status: i32,
    pub fighter_kind: i32,
}

/// One row of the ring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Row {
    pub ttl: u32,
    pub frames: u32,
    pub overall_frame: u32,
    pub input: Candidate,
}

/// The icons of a candidate in `mode`, in slot order (§3.4): the two sticks first (left white, right
/// yellow), then the buttons; five at most; an icon the layout lacks is skipped.
pub fn icons_of(mode: Mode, c: &Candidate) -> Icons {
    // The button icons in final order. Smash: ascending bit order, duplicates removed. Raw: each
    // inserted at the front, so the final order is the reverse of the list.
    let mut list: [Option<Slot>; 24] = [None; 24];
    let mut len = 0usize;
    let push = |list: &mut [Option<Slot>; 24], len: &mut usize, icon: Icon, color: Color| {
        if !list[..*len].iter().flatten().any(|s| s.icon == icon) {
            list[*len] = Some(Slot { icon, color });
            *len += 1;
        }
    };
    match mode {
        Mode::Smash => {
            let b = c.smash.buttons;
            let has = |mask: u32| b & mask != 0;
            if has(smash_bit::ATTACK | smash_bit::ATTACK_RAW) {
                push(&mut list, &mut len, Icon::A, Color::GREEN);
            }
            if has(smash_bit::SPECIAL | smash_bit::SPECIAL_RAW2) {
                push(&mut list, &mut len, Icon::B, Color::RED);
            }
            if has(smash_bit::JUMP) {
                push(&mut list, &mut len, Icon::X, Color::CYAN);
            }
            if has(smash_bit::GUARD | smash_bit::GUARD_HOLD) {
                push(&mut list, &mut len, Icon::Lb, Color::BLUE);
            }
            if has(smash_bit::CATCH) {
                push(&mut list, &mut len, Icon::Zr, Color::PURPLE);
            }
            if has(smash_bit::STOCK_SHARE) {
                push(&mut list, &mut len, Icon::Plus, Color::WHITE);
            }
            for (mask, icon) in [
                (smash_bit::APPEAL_HI, Icon::DpadUp),
                (smash_bit::APPEAL_LW, Icon::DpadDown),
                (smash_bit::APPEAL_SL, Icon::DpadRight),
                (smash_bit::APPEAL_SR, Icon::DpadLeft),
            ] {
                if has(mask) {
                    push(&mut list, &mut len, icon, Color::WHITE);
                }
            }
        }
        Mode::Raw => {
            let b = c.raw.buttons;
            let has = |mask: u32| b & mask != 0;
            let mut forward: [Option<Slot>; 24] = [None; 24];
            let mut n = 0usize;
            for (on, icon, color) in [
                (has(raw_bit::A), Icon::A, Color::GREEN),
                (has(raw_bit::B), Icon::B, Color::RED),
                (has(raw_bit::X), Icon::X, Color::CYAN),
                (has(raw_bit::Y), Icon::Y, Color::CYAN),
                (has(raw_bit::L | raw_bit::REAL_DIGITAL_L), Icon::Lb, Color::BLUE),
                (has(raw_bit::R | raw_bit::REAL_DIGITAL_R), Icon::Rb, Color::BLUE),
                (has(raw_bit::ZL), Icon::Zl, Color::PURPLE),
                (has(raw_bit::ZR), Icon::Zr, Color::PURPLE),
                (has(raw_bit::PLUS), Icon::Plus, Color::WHITE),
                (has(raw_bit::MINUS), Icon::Minus, Color::WHITE),
                (has(raw_bit::DPAD_UP), Icon::DpadUp, Color::WHITE),
                (has(raw_bit::DPAD_DOWN), Icon::DpadDown, Color::WHITE),
                (has(raw_bit::DPAD_LEFT), Icon::DpadLeft, Color::WHITE),
                (has(raw_bit::DPAD_RIGHT), Icon::DpadRight, Color::WHITE),
            ] {
                if on {
                    push(&mut forward, &mut n, icon, color);
                }
            }
            // Each inserted at the front: the reverse of the list.
            for i in (0..n).rev() {
                list[len] = forward[i];
                len += 1;
            }
        }
        Mode::StatusOnly | Mode::None => {}
    }
    // The right stick is inserted at the front, then the left: the first two slots are the sticks.
    let (l, r) = match mode {
        Mode::Smash => ((c.smash.lstick_x, c.smash.lstick_y), (c.smash.rstick_x, c.smash.rstick_y)),
        Mode::Raw => (c.raw.lstick, c.raw.rstick),
        _ => ((0, 0), (0, 0)),
    };
    let mut out = Icons::none();
    let mut n = 0usize;
    let mut put = |slot: Slot| {
        if n < ICON_SLOTS {
            out.slots[n] = Some(slot);
            n += 1;
        }
    };
    if matches!(mode, Mode::Smash | Mode::Raw) {
        if let Some(icon) = bin_stick(l.0, l.1).and_then(Direction::icon) {
            put(Slot { icon, color: Color::WHITE });
        }
        if let Some(icon) = bin_stick(r.0, r.1).and_then(Direction::icon) {
            put(Slot { icon, color: Color::YELLOW });
        }
    }
    for slot in list[..len].iter().flatten() {
        put(*slot);
    }
    out
}

/// Whether `candidate` is different from `newest` in `mode` (§3.2).
pub fn is_different(mode: Mode, status_on: bool, newest: &Candidate, candidate: &Candidate) -> bool {
    let status_differs = status_on && newest.status != candidate.status;
    match mode {
        Mode::Smash => {
            newest.smash.buttons != candidate.smash.buttons
                || bin_stick(newest.smash.lstick_x, newest.smash.lstick_y) != bin_stick(candidate.smash.lstick_x, candidate.smash.lstick_y)
                || bin_stick(newest.smash.rstick_x, newest.smash.rstick_y) != bin_stick(candidate.smash.rstick_x, candidate.smash.rstick_y)
                || status_differs
        }
        Mode::Raw => {
            newest.raw.buttons != candidate.raw.buttons
                || bin_stick(newest.raw.lstick.0, newest.raw.lstick.1) != bin_stick(candidate.raw.lstick.0, candidate.raw.lstick.1)
                || bin_stick(newest.raw.rstick.0, newest.raw.rstick.1) != bin_stick(candidate.raw.rstick.0, candidate.raw.rstick.1)
                || status_differs
        }
        Mode::StatusOnly => status_differs,
        Mode::None => false,
    }
}

/// The fade below 200 frames: `255 / log10(201) * log10(ttl + 1)` truncated, for ttl 0..200, as a table so the
/// shipped build needs no libm (`log10f` pulled 77 KB of the compiler's math library into the NRO; the
/// host test `fade_table_is_the_formula` re-derives every entry).
pub const FADE_TABLE: [u8; 200] = [
    0, 33, 52, 66, 77, 86, 93, 99, 105, 110, 115, 119, 123, 126, 130, 133, 136, 138, 141, 144, 146, 148, 150, 152, 154, 156, 158, 160, 161, 163, 165, 166, 168, 169, 170, 172, 173, 174, 176, 177, 178, 179, 180, 181, 183, 184, 185, 186, 187, 188, 189, 189, 190, 191, 192, 193, 194, 195, 196, 196, 197, 198, 199, 199, 200, 201, 202, 202, 203, 204, 204, 205, 206, 206, 207, 208, 208, 209, 210, 210, 211, 211, 212, 213, 213, 214, 214, 215, 215, 216, 216, 217, 217, 218, 218, 219, 219, 220, 220, 221, 221, 222, 222, 223, 223, 224, 224, 225, 225, 226, 226, 226, 227, 227, 228, 228, 228, 229, 229, 230, 230, 230, 231, 231, 232, 232, 232, 233, 233, 234, 234, 234, 235, 235, 235, 236, 236, 236, 237, 237, 237, 238, 238, 238, 239, 239, 239, 240, 240, 240, 241, 241, 241, 242, 242, 242, 243, 243, 243, 244, 244, 244, 244, 245, 245, 245, 246, 246, 246, 246, 247, 247, 247, 248, 248, 248, 248, 249, 249, 249, 249, 250, 250, 250, 251, 251, 251, 251, 252, 252, 252, 252, 253, 253, 253, 253, 254, 254, 254, 254,
];

/// The alpha of a row with time to live `ttl` (§2.2): opaque from 200, logarithmic below, 0 at 0.
pub const fn fade_alpha(ttl: u32) -> u8 {
    if ttl >= FADE_START {
        255
    } else {
        FADE_TABLE[ttl as usize]
    }
}

/// The logger: the ring of rows and the two in-game frame counters.
#[derive(Clone, Copy, Debug)]
pub struct Log {
    rows: [Row; NUM_LOGS],
    /// The physical row that holds the newest row (rows keep their panes while they age).
    base: usize,
    /// The "overall" stamp and the "per-row" counter, both in-game frames that survive a reset.
    overall: u32,
    per_row: u32,
    counting: bool,
}

impl Default for Log {
    fn default() -> Self {
        Self::new()
    }
}

impl Log {
    pub const fn new() -> Self {
        Self {
            rows: [Row { ttl: 0, frames: 0, overall_frame: 0, input: Candidate {
                raw: RawInput { buttons: 0, lstick: (0, 0), rstick: (0, 0) },
                smash: MappedInputs::NEUTRAL,
                status: 0,
                fighter_kind: 0,
            } }; NUM_LOGS],
            base: 0,
            overall: 0,
            per_row: 0,
            counting: false,
        }
    }

    pub fn rows(&self) -> &[Row; NUM_LOGS] {
        &self.rows
    }

    pub const fn base(&self) -> usize {
        self.base
    }

    /// The physical pane index of logical row `i` (0 = newest): `(i + 15 - base) mod 15`.
    pub const fn physical(&self, i: usize) -> usize {
        (i + NUM_LOGS - self.base) % NUM_LOGS
    }

    /// One in-game frame of the primary CPU body passed (Ice Climbers count once; SPEC-input-02 §6.3).
    pub fn tick_frame(&mut self) {
        if self.counting {
            self.overall = self.overall.wrapping_add(1);
            self.per_row = self.per_row.saturating_add(1);
        }
    }

    pub const fn overall(&self) -> u32 {
        self.overall
    }

    /// Session clear (§5.4): all rows reset to default, base 0, both counters fully reset.
    pub fn session_clear(&mut self) {
        *self = Self::new();
    }

    /// One P1 poll after the input delay (§5.1). `None` mode logs nothing.
    pub fn record(&mut self, s: &Settings, c: Candidate) {
        if s.mode == Mode::None {
            return;
        }
        // Make sure both counters are counting.
        self.counting = true;
        let new_frame = self.rows[0].overall_frame != self.overall;
        let mut candidate = Row { ttl: ROW_TTL, frames: 1, overall_frame: self.overall, input: c };
        if new_frame && is_different(s.mode, s.status, &self.rows[0].input, &c) {
            // A new row: the per-row counter restarts at 1; the ring turns.
            self.per_row = 1;
            candidate.frames = 1;
            self.rows.copy_within(0..NUM_LOGS - 1, 1);
            self.rows[0] = candidate;
            self.base = (self.base + 1) % NUM_LOGS;
        } else if new_frame {
            // The same input held: overwrite the newest row, keeping its ttl.
            let ttl = self.rows[0].ttl;
            self.rows[0] = Row { ttl, frames: self.per_row.min(FRAME_CAP), ..candidate };
        }
        if new_frame {
            for row in &mut self.rows {
                if row.ttl > 0 {
                    row.ttl -= 1;
                }
            }
        }
    }
}

/// What the status text shows for a row. The names are the game's constants' identifiers (spec
/// question 1); until that is decided the numeric id is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusText {
    Empty,
    Id { kind: i32, status: i32 },
}

/// One write the renderer asks of the panes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    /// `TrModInputLog` visibility.
    Column { visible: bool },
    /// Row `phys`: position, visibility, alpha (and global alpha).
    Row { phys: u8, y: f32, alpha: u8 },
    /// Row `phys` hidden (ttl 0).
    RowHidden { phys: u8 },
    /// Row `phys`: the five slot text boxes: each shown slot's glyph and colour, the rest emptied.
    Icons { phys: u8, icons: Icons },
    /// Row `phys`: the `Frame` text.
    Frames { phys: u8, frames: u8 },
    /// Row `phys`: the `Status` text.
    Status { phys: u8, text: StatusText },
}

/// The most ops one plan holds: the column, and per row position+alpha, icons, frames, status.
pub const PLAN_MAX: usize = 1 + NUM_LOGS * 5;

/// A fixed-capacity list of ops.
#[derive(Clone, Copy, Debug)]
pub struct Plan {
    ops: [Op; PLAN_MAX],
    len: usize,
}

impl Plan {
    pub const fn new() -> Self {
        Self { ops: [Op::Column { visible: false }; PLAN_MAX], len: 0 }
    }

    fn push(&mut self, op: Op) {
        if self.len < PLAN_MAX {
            self.ops[self.len] = op;
            self.len += 1;
        }
    }

    pub fn ops(&self) -> &[Op] {
        &self.ops[..self.len]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for Plan {
    fn default() -> Self {
        Self::new()
    }
}

/// What was last written to one physical row.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Written {
    valid: bool,
    y: f32,
    /// The fade, as the minimum of ttl and 200 (only changes matter below 200).
    ttl_min: u32,
    mode: Mode,
    status_on: bool,
    icons: Icons,
    frames: u8,
    status: StatusText,
}

impl Written {
    const NONE: Written =
        Written { valid: false, y: 0.0, ttl_min: 0, mode: Mode::None, status_on: false, icons: Icons::none(), frames: 0, status: StatusText::Empty };
}

/// The renderer's cache and its plan (§5.2).
#[derive(Clone, Copy, Debug)]
pub struct Renderer {
    written: [Written; NUM_LOGS],
    column_visible: Option<bool>,
    /// The next visible tick redraws every row (the column was hidden, or the layout was registered).
    full_refresh: bool,
    disabled: bool,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub const fn new() -> Self {
        Self { written: [Written::NONE; NUM_LOGS], column_visible: None, full_refresh: true, disabled: false }
    }

    /// The layout was registered (even at an address it had before): nothing about the panes is known,
    /// and every retained row is drawn again on the next visible tick.
    pub fn on_layout_registered(&mut self) {
        self.written = [Written::NONE; NUM_LOGS];
        self.column_visible = None;
        self.full_refresh = true;
    }

    /// A row pane was missing (spec question 5): log once and turn the display off for the session
    /// instead of panicking.
    pub fn disable(&mut self) {
        self.disabled = true;
    }

    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn session_clear(&mut self) {
        *self = Self::new();
    }

    /// One HUD tick. `visible` is: mod menu closed, vanilla menu closed, mode not None.
    pub fn plan(&mut self, log: &Log, s: &Settings, visible: bool) -> Plan {
        let mut plan = Plan::new();
        if self.disabled {
            return plan;
        }
        let show = visible && s.mode != Mode::None;
        if self.column_visible != Some(show) {
            plan.push(Op::Column { visible: show });
            self.column_visible = Some(show);
        }
        if !show {
            // Hidden: no content work; a full refresh is due when visible again.
            self.full_refresh = true;
            return plan;
        }
        let refresh = self.full_refresh;
        for i in 0..NUM_LOGS {
            let phys = log.physical(i);
            let row = &log.rows()[i];
            let y = -ROW_PITCH * i as f32;
            let ttl_min = row.ttl.min(FADE_START);
            let w = &mut self.written[phys];
            let moved = !w.valid || w.y != y;
            let faded = !w.valid || w.ttl_min != ttl_min;
            if row.ttl == 0 {
                if !w.valid || w.ttl_min != 0 || moved {
                    plan.push(Op::RowHidden { phys: phys as u8 });
                    w.valid = true;
                    w.ttl_min = 0;
                    w.y = y;
                }
                continue;
            }
            if moved || faded || refresh {
                plan.push(Op::Row { phys: phys as u8, y, alpha: fade_alpha(row.ttl) });
                w.y = y;
                w.ttl_min = ttl_min;
            }
            // Content: the newest row always; every other row only after a full refresh.
            if i == 0 || refresh || !w.valid {
                let icons = icons_of(s.mode, &row.input);
                let frames = row.frames.min(FRAME_CAP) as u8;
                let status = if s.status || s.mode == Mode::StatusOnly {
                    StatusText::Id { kind: row.input.fighter_kind, status: row.input.status }
                } else {
                    StatusText::Empty
                };
                if !w.valid || w.mode != s.mode || w.icons != icons || refresh {
                    plan.push(Op::Icons { phys: phys as u8, icons });
                    w.icons = icons;
                }
                if !w.valid || w.frames != frames || refresh {
                    plan.push(Op::Frames { phys: phys as u8, frames });
                    w.frames = frames;
                }
                if !w.valid || w.status != status || w.status_on != s.status || refresh {
                    plan.push(Op::Status { phys: phys as u8, text: status });
                    w.status = status;
                }
                w.mode = s.mode;
                w.status_on = s.status;
            }
            w.valid = true;
        }
        self.full_refresh = false;
        plan
    }
}
