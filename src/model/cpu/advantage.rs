//! Frame advantage (SPEC-cpu-07 §5): after an exchange, how many frames earlier or later than the CPU the
//! player can act again. Pure: the runtime reads whether each fighter is actionable (the five transition
//! terms or a cancel) once per frame on the player entry's callback and calls `step`; the elapsed clock is
//! one in-game counter that ticks once per simulation frame with the others (`Cpu::tick_clock`).

use super::counter::Counter;

/// The notification's colour class for a value (alpha 255 in every case): red negative, black zero, green
/// positive (SPEC-cpu-07 §2). The widget is the UI package's; the numbers are fixed here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tint {
    Red,
    Black,
    Green,
}

impl Tint {
    pub const fn of(advantage: i32) -> Tint {
        if advantage < 0 {
            Tint::Red
        } else if advantage == 0 {
            Tint::Black
        } else {
            Tint::Green
        }
    }

    /// `(r, g, b, a)`.
    pub const fn rgba(self) -> (u8, u8, u8, u8) {
        match self {
            Tint::Red => (200, 8, 8, 255),
            Tint::Black => (0, 0, 0, 255),
            Tint::Green => (31, 198, 0, 255),
        }
    }
}

/// How many frames the notification stays.
pub const NOTIFICATION_FRAMES: u32 = 60;

/// The text of the notification: the signed number (`+N`, `-N`, `+0`), into a fixed buffer.
pub fn format_advantage(advantage: i32, out: &mut [u8; 12]) -> usize {
    let mut digits = [0u8; 10];
    let mut n = advantage.unsigned_abs();
    let mut len = 0;
    loop {
        digits[len] = b'0' + (n % 10) as u8;
        len += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out[0] = if advantage < 0 { b'-' } else { b'+' };
    for i in 0..len {
        out[1 + i] = digits[len - 1 - i];
    }
    1 + len
}

/// The measurement state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameAdvantage {
    pub counting: bool,
    pub elapsed: Counter,
    pub locked_player: Option<u32>,
    pub locked_cpu: Option<u32>,
    last_player: bool,
    last_cpu: bool,
}

impl FrameAdvantage {
    pub const fn new() -> Self {
        Self { counting: false, elapsed: Counter::new(), locked_player: None, locked_cpu: None, last_player: false, last_cpu: false }
    }

    /// Clear everything (the toggle turned Off, a mash set, a reset or a save-state load).
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// One frame on the player entry's callback. `enabled` is the Frame Advantage row, `mash_empty`
    /// that Mash Toggles is empty (a mashing CPU spoils the measurement). Returns the advantage to show.
    pub fn step(&mut self, enabled: bool, mash_empty: bool, player_actionable: bool, cpu_actionable: bool) -> Option<i32> {
        if !enabled || !mash_empty {
            if self.counting {
                self.reset();
            }
            return None;
        }
        let player_just = player_actionable && !self.last_player;
        let cpu_just = cpu_actionable && !self.last_cpu;
        self.last_player = player_actionable;
        self.last_cpu = cpu_actionable;
        if !self.counting {
            if !(player_actionable && cpu_actionable) {
                self.elapsed.full_reset();
                self.elapsed.counting = true;
                self.locked_player = player_actionable.then_some(0);
                self.locked_cpu = cpu_actionable.then_some(0);
                self.counting = true;
            }
            return None;
        }
        let elapsed = self.elapsed.frames;
        if player_just {
            self.locked_player = Some(elapsed);
        }
        if cpu_just {
            self.locked_cpu = Some(elapsed);
        }
        if player_actionable && cpu_actionable {
            let shown = self.locked_player.zip(self.locked_cpu).map(|(p, c)| c as i32 - p as i32);
            self.elapsed.full_reset();
            self.locked_player = None;
            self.locked_cpu = None;
            self.counting = false;
            return shown;
        }
        None
    }
}
