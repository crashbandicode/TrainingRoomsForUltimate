//! The frame strip (SPEC-input-06, operator decision INPUT-VIZ option A, 2026-11-05): the last 60 game frames of
//! P1 and the primary CPU as coloured lanes, P1's presses as chips above them, and how the last hits linked. Pure:
//! the runtime samples both fighters once per game frame (`Tracker::step`) and the HUD tick turns the tracker into
//! pane looks (`frame_strip_plan`). Fixed arrays only, so nothing here allocates.

use super::input_delay::MappedInputs;

/// Frames on screen and links kept for the LAST row (the pane table's numbers).
pub const WINDOW: usize = super::panes::iv::WINDOW;
pub const HISTORY: usize = super::panes::iv::HISTORY;
/// Presses kept (chips and the input look-back).
pub const PRESSES: usize = 16;
/// Both fighters actionable and no new press for this many frames: the strip stops scrolling.
pub const FREEZE_AFTER: u16 = 30;
/// How far back a follow-up's press is looked for (SPEC-input-06 §2, "input").
pub const INPUT_LOOKBACK: u32 = 15;
/// The sampling constant `K` (SPEC-input-06 §4): the free frame a hitstun read predicts is `(H - 1) + ceil(r) + K`.
pub const K: i32 = 1;
/// A follow-up that comes more than this many frames after the CPU could act is a new exchange, not a drop.
pub const DROP_MAX: u32 = 20;
/// A hitstun read above this is not believed (the game's largest reactions stay far below it).
pub const REACTION_MAX: f32 = 255.0;

/// One fighter's state on one frame (SPEC-input-06 §3.2, the lane colours).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Actionable,
    Startup,
    Active,
    Recovery,
    Hitlag,
    Hitstun,
    Shieldstun,
}

impl Cell {
    /// The lane colour (sRGB).
    pub const fn color(self) -> [u8; 4] {
        match self {
            Cell::Actionable => [64, 72, 88, 255],
            Cell::Startup => [244, 180, 52, 255],
            Cell::Active => [238, 70, 84, 255],
            Cell::Recovery => [82, 136, 232, 255],
            Cell::Hitlag => [232, 232, 236, 255],
            Cell::Hitstun => [170, 98, 234, 255],
            Cell::Shieldstun => [48, 196, 188, 255],
        }
    }

    /// Runs this long of these states show their length.
    pub const fn counted(self) -> bool {
        matches!(self, Cell::Recovery | Cell::Hitstun | Cell::Shieldstun)
    }
}

/// The engine-common move a P1 attack status belongs to (for the LAST row's names).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Jab,
    DashAttack,
    SideTilt,
    UpTilt,
    DownTilt,
    SideSmash,
    UpSmash,
    DownSmash,
    Aerial,
    NeutralSpecial,
    SideSpecial,
    UpSpecial,
    DownSpecial,
    /// A fighter's own attack status, or one this list does not name.
    Other,
}

impl Move {
    pub const fn name(self) -> &'static str {
        match self {
            Move::Jab => "Jab",
            Move::DashAttack => "Dash attack",
            Move::SideTilt => "Side tilt",
            Move::UpTilt => "Up tilt",
            Move::DownTilt => "Down tilt",
            Move::SideSmash => "Side smash",
            Move::UpSmash => "Up smash",
            Move::DownSmash => "Down smash",
            Move::Aerial => "Aerial",
            Move::NeutralSpecial => "Neutral B",
            Move::SideSpecial => "Side B",
            Move::UpSpecial => "Up B",
            Move::DownSpecial => "Down B",
            Move::Other => "Move",
        }
    }
}

/// What the runtime read from one fighter on one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sample {
    /// `StopModule::is_stop`.
    pub stop: bool,
    pub hitstun: bool,
    pub shieldstun: bool,
    /// The frame-advantage test (SPEC-cpu-07 §3).
    pub actionable: bool,
    /// P1 only: any attack slot active.
    pub active: bool,
    /// P1 only: the status is an attack, and which.
    pub attack: Option<Move>,
    /// The CPU only, in hitstun: the hitstun frames left (`None` when it could not be read).
    pub reaction: Option<f32>,
    /// Holding or raising a shield (`GUARD_ON ..= GUARD_DAMAGE`).
    pub shielding: bool,
    /// P1 only: the status kind and the motion frame (a new move starts when either goes back).
    pub status: i32,
    pub motion_frame: f32,
}

/// Which button a press was: the chip's colour and glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Attack,
    Special,
    Jump,
    Shield,
    Grab,
}

impl Button {
    pub const fn color(self) -> [u8; 4] {
        match self {
            Button::Attack => [72, 190, 110, 255],
            Button::Special => [226, 82, 76, 255],
            Button::Jump => [64, 170, 220, 255],
            Button::Shield => [90, 110, 160, 255],
            Button::Grab => [140, 100, 210, 255],
        }
    }

    /// The glyph: a private-use unit of the game's font where it has one, else a plain letter.
    pub const fn glyph(self) -> u16 {
        match self {
            Button::Attack => 0xE0E0,
            Button::Special => 0x42,
            Button::Jump => 0xE0E2,
            Button::Shield => 0xE0E4,
            Button::Grab => 0xE0E7,
        }
    }
}

/// The mapped ("Smash") button bits a press is read from (SPEC-input-01 §3.1), in chip priority order.
pub const PRESS_BITS: [(u32, Button); 5] = [
    (0x1 | 0x200, Button::Attack),
    (0x2 | 0x20000, Button::Special),
    (0x10, Button::Grab),
    (0x4 | 0x8000, Button::Jump),
    (0x8 | 0x10000, Button::Shield),
];

/// The stick thresholds of the old log (SPEC-input-02 §3.3, kept by SPEC-input-06 §3.3).
pub const WALK_THRESHOLD: i16 = 20;
pub const DEAD_ZONE: i16 = 30;

/// A stick binned to one of 8 directions: 0 right, counter-clockwise, 45 degrees apart (`0..8`).
pub fn bin_stick(x: i8, y: i8) -> Option<u8> {
    let (x, y) = (i16::from(x), i16::from(y));
    let right = x > WALK_THRESHOLD;
    let left = x < -WALK_THRESHOLD;
    let up = y > DEAD_ZONE;
    let down = y < -DEAD_ZONE;
    Some(match (right, left, up, down) {
        (true, _, true, _) => 1,
        (_, true, true, _) => 3,
        (_, true, _, true) => 5,
        (true, _, _, true) => 7,
        (true, ..) => 0,
        (_, _, true, _) => 2,
        (_, true, ..) => 4,
        (.., true) => 6,
        _ => return None,
    })
}

/// A new press on one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Press {
    pub frame: u32,
    pub button: Button,
    pub stick: Option<u8>,
}

/// The press P1's mapped inputs show on this frame against the last frame's buttons: the first button in
/// `PRESS_BITS` order that went down.
pub fn press_of(now: MappedInputs, before: u32, frame: u32) -> Option<Press> {
    let down = now.buttons & !before;
    PRESS_BITS
        .iter()
        .find(|(bits, _)| down & bits != 0)
        .map(|&(_, button)| Press { frame, button, stick: bin_stick(now.lstick_x, now.lstick_y) })
}

/// How a follow-up linked (SPEC-input-06 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// A true combo; `Some(link)` (`<= 0`) when the hitstun left could be read.
    True(Option<i32>),
    /// The CPU had `link` (`>= 1`) actionable frames before the hit.
    Drop(i32),
}

/// One follow-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    pub verdict: Verdict,
    /// The follow-up hit `H₂`.
    pub hit: u32,
    /// The CPU's free frame the first hit gave (`F₁`: observed for a drop, predicted for a true combo).
    pub free: Option<u32>,
    /// P1's first actionable frame after the first hit (`A₁`).
    pub actionable: u32,
    /// The press `I` and `input = I − A₁`.
    pub press: u32,
    pub input: i32,
    /// The combo window `A₁ ..= S₂ − link` (`None` when even a frame-perfect press drops, or the link is unknown).
    pub window: Option<(u32, u32)>,
    pub from: Move,
    pub to: Move,
}

impl Link {
    /// Frames to spare (true combo) or past the window (drop, negative). `None` without a number.
    pub fn spare(&self) -> Option<i32> {
        match self.verdict {
            Verdict::True(Some(link)) => Some(-link),
            Verdict::True(None) => None,
            Verdict::Drop(link) => Some(-link),
        }
    }
}

/// The frame advantage after an exchange (SPEC-input-06 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Advantage {
    pub value: i32,
    pub on_shield: bool,
    /// P1's and the CPU's first actionable frames.
    pub p1: u32,
    pub cpu: u32,
}

/// One frame on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Column {
    pub frame: u32,
    pub p1: Cell,
    pub cpu: Cell,
}

/// The last hit and what followed it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hit {
    frame: u32,
    on_shield: bool,
    action: u32,
    mv: Move,
    /// P1's and the CPU's first actionable frames after it.
    p1_free: Option<u32>,
    cpu_free: Option<u32>,
}

/// The sampling state and what the strip shows. `Copy`, so the HUD tick can take a snapshot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tracker {
    columns: [Column; WINDOW],
    len: usize,
    head: usize,
    presses: [Option<Press>; PRESSES],
    press_head: usize,
    history: [Option<Link>; HISTORY],
    advantage: Option<Advantage>,
    frame: u32,
    quiet: u16,
    buttons: u32,
    // P1's action.
    p1_was_actionable: bool,
    action: u32,
    action_start: u32,
    active_seen: bool,
    action_move: Move,
    p1_status: i32,
    p1_motion_frame: f32,
    // The CPU.
    cpu_was_stop: bool,
    reaction: Option<f32>,
    last_hit: Option<Hit>,
    /// Stun checks for the diag build: a hit not followed up, its predicted and observed free frames.
    pub stun_check: Option<(u32, u32)>,
}

impl Default for Tracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Tracker {
    pub const fn new() -> Self {
        Self {
            columns: [Column { frame: 0, p1: Cell::Actionable, cpu: Cell::Actionable }; WINDOW],
            len: 0,
            head: 0,
            presses: [None; PRESSES],
            press_head: 0,
            history: [None; HISTORY],
            advantage: None,
            frame: 0,
            quiet: 0,
            buttons: 0,
            p1_was_actionable: true,
            action: 0,
            action_start: 0,
            active_seen: false,
            action_move: Move::Other,
            p1_status: -1,
            p1_motion_frame: 0.0,
            cpu_was_stop: false,
            reaction: None,
            last_hit: None,
            stun_check: None,
        }
    }

    /// The frames on screen, oldest first.
    pub fn columns(&self) -> impl Iterator<Item = Column> + '_ {
        (0..self.len).map(move |i| self.columns[(self.head + WINDOW - self.len + i) % WINDOW])
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The newest frame on screen.
    pub fn newest(&self) -> Option<u32> {
        (self.len > 0).then(|| self.columns[(self.head + WINDOW - 1) % WINDOW].frame)
    }

    /// Presses, newest first.
    pub fn presses(&self) -> impl Iterator<Item = Press> + '_ {
        (0..PRESSES).filter_map(move |i| self.presses[(self.press_head + PRESSES - 1 - i) % PRESSES])
    }

    /// Links, newest first.
    pub fn history(&self) -> impl Iterator<Item = Link> + '_ {
        self.history.iter().flatten().copied()
    }

    pub fn latest_link(&self) -> Option<Link> {
        self.history[0]
    }

    pub fn advantage(&self) -> Option<Advantage> {
        self.advantage
    }

    pub fn frozen(&self) -> bool {
        self.quiet >= FREEZE_AFTER
    }

    /// Leaving the scene, a hot swap, a save-state load: nothing on screen happened in this timeline any more.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// One game frame: P1's and the CPU's samples and P1's mapped inputs.
    pub fn step(&mut self, p1: Sample, cpu: Sample, inputs: MappedInputs) {
        self.frame = self.frame.wrapping_add(1);
        let frame = self.frame;
        let press = press_of(inputs, self.buttons, frame);
        self.buttons = inputs.buttons;
        if let Some(press) = press {
            self.presses[self.press_head] = Some(press);
            self.press_head = (self.press_head + 1) % PRESSES;
        }
        // P1's action (SPEC-input-06 §2): it starts on the first frame P1 cannot act, or when a buffered press starts a
        // new attack without an actionable frame between (another move, or the same status from its motion's start).
        let p1_busy = !p1.actionable && !p1.hitstun && !p1.shieldstun;
        let restarted = p1.attack.is_some_and(|m| {
            (m != self.action_move && self.action_move != Move::Other)
                || (p1.status == self.p1_status && p1.motion_frame < self.p1_motion_frame && !p1.stop)
        });
        if p1_busy && (self.p1_was_actionable || restarted) {
            self.action = self.action.wrapping_add(1);
            self.action_start = frame;
            self.active_seen = false;
            self.action_move = Move::Other;
        }
        if p1_busy && self.action_move == Move::Other {
            self.action_move = p1.attack.unwrap_or(Move::Other);
        }
        self.p1_was_actionable = p1.actionable || p1.hitstun || p1.shieldstun;
        self.p1_status = p1.status;
        self.p1_motion_frame = p1.motion_frame;
        let p1_cell = classify(&p1, true, self.active_seen);
        if p1.active {
            self.active_seen = true;
        }
        let cpu_cell = classify(&cpu, false, false);
        // A hit on the CPU: its hitstop starts while P1 is not the one being hit (SPEC-input-06 §2). On shield when the
        // CPU is shielding or in shieldstun.
        let p1_hit = p1.hitstun || p1.shieldstun || p1.shielding;
        if cpu.stop && !self.cpu_was_stop && !p1_hit {
            self.on_hit(frame, (cpu.shieldstun || cpu.shielding) && !cpu.hitstun);
        }
        self.cpu_was_stop = cpu.stop;
        // Both fighters' first actionable frames after the last hit.
        if let Some(hit) = self.last_hit.as_mut() {
            if p1.actionable && hit.p1_free.is_none() {
                hit.p1_free = Some(frame);
            }
            if cpu.actionable && hit.cpu_free.is_none() {
                hit.cpu_free = Some(frame);
                if let (Some(r), false) = (self.reaction, hit.on_shield) {
                    // Diag only: the free frame the read predicted against the one observed.
                    self.stun_check = Some((predicted_free(frame - 1, r).unwrap_or(0), frame));
                }
            }
            if let (Some(p), Some(c)) = (hit.p1_free, hit.cpu_free) {
                let value = c as i32 - p as i32;
                let advantage = Advantage { value, on_shield: hit.on_shield, p1: p, cpu: c };
                if self.advantage != Some(advantage) && hit.p1_free.max(hit.cpu_free) == Some(frame) {
                    self.advantage = Some(advantage);
                }
            }
        }
        self.reaction = if cpu.hitstun && !cpu.actionable { cpu.reaction } else { None };
        // Freeze while nothing happens.
        let quiet = p1.actionable && cpu.actionable && press.is_none() && !p1.stop && !cpu.stop;
        self.quiet = if quiet { self.quiet.saturating_add(1) } else { 0 };
        if !self.frozen() {
            self.columns[self.head] = Column { frame, p1: p1_cell, cpu: cpu_cell };
            self.head = (self.head + 1) % WINDOW;
            self.len = (self.len + 1).min(WINDOW);
        }
    }

    fn on_hit(&mut self, frame: u32, on_shield: bool) {
        // A hit long after the CPU could act again opens a new exchange instead of dropping a link.
        let follow_up = self.last_hit.filter(|prev| {
            !prev.on_shield
                && !on_shield
                && prev.action != self.action
                && prev.cpu_free.is_none_or(|free| frame.saturating_sub(free) <= DROP_MAX)
        });
        if let Some(prev) = follow_up {
            // A buffered follow-up starts on the first frame P1 could act, without an actionable sample between.
            let actionable = prev.p1_free.unwrap_or(self.action_start).min(self.action_start);
            let (verdict, free) = match prev.cpu_free {
                Some(free) if free < frame => (Verdict::Drop((frame - free) as i32), Some(free)),
                _ => match self.reaction.and_then(|r| predicted_free(frame - 1, r)) {
                    Some(free) if free >= frame => (Verdict::True(Some(frame as i32 - free as i32)), Some(free)),
                    _ => (Verdict::True(None), None),
                },
            };
            let press = self
                .presses()
                .find(|p| p.frame <= self.action_start && self.action_start - p.frame <= INPUT_LOOKBACK)
                .map_or(self.action_start, |p| p.frame);
            let link = match verdict {
                Verdict::True(link) => link,
                Verdict::Drop(link) => Some(link),
            };
            let window = link.and_then(|link| {
                let end = self.action_start as i64 - i64::from(link);
                (end >= i64::from(actionable)).then_some((actionable, end as u32))
            });
            let item = Link {
                verdict,
                hit: frame,
                free,
                actionable,
                press,
                input: press as i32 - actionable as i32,
                window,
                from: prev.mv,
                to: self.action_move,
            };
            self.history.copy_within(0..HISTORY - 1, 1);
            self.history[0] = Some(item);
        }
        // A hit of the same action as the last one (a multi-hit move) keeps its first actionable frames unknown.
        self.last_hit = Some(Hit {
            frame,
            on_shield,
            action: self.action,
            mv: self.action_move,
            p1_free: None,
            cpu_free: None,
        });
    }
}

/// The CPU's free frame a hitstun read `r` on frame `at` predicts: `at + ceil(r) + K` (SPEC-input-06 §2 and §4).
/// `None` for a value that is not a believable frame count.
pub fn predicted_free(at: u32, r: f32) -> Option<u32> {
    if !r.is_finite() || !(0.0..=REACTION_MAX).contains(&r) {
        return None;
    }
    let left = r as u32 + u32::from(r.fract() > 0.0);
    Some(at + left + K as u32)
}

/// One fighter's lane colour on one frame, by the first rule that matches (SPEC-input-06 §3.2). `active_seen`: P1's
/// current action already had an active frame.
pub fn classify(s: &Sample, p1: bool, active_seen: bool) -> Cell {
    if s.stop {
        Cell::Hitlag
    } else if s.hitstun {
        Cell::Hitstun
    } else if s.shieldstun {
        Cell::Shieldstun
    } else if s.actionable {
        Cell::Actionable
    } else if p1 && s.active {
        Cell::Active
    } else if p1 && !active_seen && s.attack.is_some() {
        Cell::Startup
    } else {
        Cell::Recovery
    }
}

/// The game constants the strip reads, by name (resolved through the game's own table at run time; SPEC-input-06 §4).
pub mod ids {
    crate::model::cpu::ids::game_constants! {
        ReactionFrame = "FIGHTER_INSTANCE_WORK_ID_FLOAT_DAMAGE_REACTION_FRAME",
        Damage = "FIGHTER_STATUS_KIND_DAMAGE",
        DamageFall = "FIGHTER_STATUS_KIND_DAMAGE_FALL",
        GuardOn = "FIGHTER_STATUS_KIND_GUARD_ON",
        GuardDamage = "FIGHTER_STATUS_KIND_GUARD_DAMAGE",
        GuardOff = "FIGHTER_STATUS_KIND_GUARD_OFF",
        Attack = "FIGHTER_STATUS_KIND_ATTACK",
        Attack100 = "FIGHTER_STATUS_KIND_ATTACK_100",
        AttackDash = "FIGHTER_STATUS_KIND_ATTACK_DASH",
        AttackS3 = "FIGHTER_STATUS_KIND_ATTACK_S3",
        AttackHi3 = "FIGHTER_STATUS_KIND_ATTACK_HI3",
        AttackLw3 = "FIGHTER_STATUS_KIND_ATTACK_LW3",
        AttackS4Start = "FIGHTER_STATUS_KIND_ATTACK_S4_START",
        AttackS4 = "FIGHTER_STATUS_KIND_ATTACK_S4",
        AttackLw4Start = "FIGHTER_STATUS_KIND_ATTACK_LW4_START",
        AttackLw4 = "FIGHTER_STATUS_KIND_ATTACK_LW4",
        AttackHi4Start = "FIGHTER_STATUS_KIND_ATTACK_HI4_START",
        AttackHi4 = "FIGHTER_STATUS_KIND_ATTACK_HI4",
        AttackAir = "FIGHTER_STATUS_KIND_ATTACK_AIR",
        SpecialN = "FIGHTER_STATUS_KIND_SPECIAL_N",
        SpecialS = "FIGHTER_STATUS_KIND_SPECIAL_S",
        SpecialHi = "FIGHTER_STATUS_KIND_SPECIAL_HI",
        SpecialLw = "FIGHTER_STATUS_KIND_SPECIAL_LW",
        Final = "FIGHTER_STATUS_KIND_FINAL",
    }

    impl Table {
        fn within(&self, from: C, to: C, status: i32) -> bool {
            self.resolved(from) && self.resolved(to) && (self.get(from)..=self.get(to)).contains(&status)
        }

        /// Hitstun: `DAMAGE ..< DAMAGE_FALL` (the range of the CPU specs' predicate, tumble included).
        pub fn hitstun(&self, status: i32) -> bool {
            self.resolved(C::Damage) && self.resolved(C::DamageFall) && (self.get(C::Damage)..self.get(C::DamageFall)).contains(&status)
        }

        /// Shieldstun: `GUARD_DAMAGE`, or `GUARD_OFF` right after it.
        pub fn shieldstun(&self, status: i32, prev: i32) -> bool {
            self.is(C::GuardDamage, status) || (self.is(C::GuardOff, status) && self.is(C::GuardDamage, prev))
        }

        /// Holding or raising a shield: `GUARD_ON ..= GUARD_DAMAGE`.
        pub fn shielding(&self, status: i32) -> bool {
            self.within(C::GuardOn, C::GuardDamage, status)
        }

        /// The move an attack status belongs to; a fighter's own status (above `FINAL`) is `Other`.
        pub fn attack(&self, status: i32) -> Option<super::Move> {
            use super::Move;
            let one = [
                (C::Attack, Move::Jab),
                (C::Attack100, Move::Jab),
                (C::AttackDash, Move::DashAttack),
                (C::AttackS3, Move::SideTilt),
                (C::AttackHi3, Move::UpTilt),
                (C::AttackLw3, Move::DownTilt),
                (C::AttackAir, Move::Aerial),
                (C::SpecialN, Move::NeutralSpecial),
                (C::SpecialS, Move::SideSpecial),
                (C::SpecialHi, Move::UpSpecial),
                (C::SpecialLw, Move::DownSpecial),
            ];
            if let Some((_, m)) = one.iter().find(|(c, _)| self.is(*c, status)) {
                return Some(*m);
            }
            let ranges = [
                (C::AttackS4Start, C::AttackS4, Move::SideSmash),
                (C::AttackLw4Start, C::AttackLw4, Move::DownSmash),
                (C::AttackHi4Start, C::AttackHi4, Move::UpSmash),
            ];
            if let Some((.., m)) = ranges.iter().find(|(a, b, _)| self.within(*a, *b, status)) {
                return Some(*m);
            }
            (self.resolved(C::Final) && status > self.get(C::Final)).then_some(Move::Other)
        }
    }
}

/// The row (`input_display_strip`: Off, Frame strip).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub on: bool,
}

/// The new row's id, and the old rows it replaces (SPEC-input-06 §3.1).
pub const ROW: &str = "input_display_strip";
pub const OLD_ROW: &str = "input_display";
pub const OLD_STATUS_ROW: &str = "input_display_status";

impl Settings {
    pub const fn factory() -> Self {
        Self { on: true }
    }

    pub fn from_snapshot(snapshot: &super::settings::Snapshot) -> Self {
        let on = snapshot.get(ROW).and_then(|r| r.iter().position(|v| *v != 0)).map_or(true, |i| i == 1);
        Self { on }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::factory()
    }
}

/// The migration of SPEC-input-06 §3.1: the new row's value from the old `input_display` array (Off stays Off, any
/// other choice is the frame strip). `None` when the old value is not a one-of-four choice.
pub fn migrate(old: &[u8]) -> Option<[u8; 2]> {
    if old.len() != 4 || old.iter().filter(|v| **v != 0).count() != 1 {
        return None;
    }
    Some(if old[0] != 0 { [1, 0] } else { [0, 1] })
}

#[cfg(test)]
#[path = "frame_strip_tests.rs"]
mod tests;
