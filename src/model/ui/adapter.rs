//! The menu's input adapter (SPEC-core-03 §2.1, §3): Player 1's controller record, once per poll, turned
//! into the opening chords and the logical presses of `model::menu`. Pure: the runtime hands it the
//! record's button bits and style and takes what to do.
//!
//! Record bit order (13.0.5, SPEC-core-03 §3): 0 d-pad up, 1 right, 2 down, 3 left, 4 x, 5 a, 6 b, 7 y, 8 l,
//! 9 r, 10 zl, 11 zr, 12 left SL, 13 left SR, 14 right SL, 15 right SR, 16 stick L, 17 stick R, 18 plus,
//! 19 minus, 20-23 left stick up / right / down / left, 24-27 right stick up / right / down / left, 28 real
//! digital L, 29 real digital R.

use super::menu::chord::Chord;
use super::menu::input::{Button, HoldRepeat};

pub mod bit {
    pub const DPAD_UP: u32 = 0;
    pub const DPAD_RIGHT: u32 = 1;
    pub const DPAD_DOWN: u32 = 2;
    pub const DPAD_LEFT: u32 = 3;
    pub const X: u32 = 4;
    pub const A: u32 = 5;
    pub const B: u32 = 6;
    pub const Y: u32 = 7;
    pub const L: u32 = 8;
    pub const R: u32 = 9;
    pub const ZL: u32 = 10;
    pub const ZR: u32 = 11;
    pub const LEFT_SL: u32 = 12;
    pub const LEFT_SR: u32 = 13;
    pub const RIGHT_SL: u32 = 14;
    pub const RIGHT_SR: u32 = 15;
    pub const STICK_L: u32 = 16;
    pub const STICK_R: u32 = 17;
    pub const PLUS: u32 = 18;
    pub const MINUS: u32 = 19;
    pub const LSTICK_UP: u32 = 20;
    pub const LSTICK_RIGHT: u32 = 21;
    pub const LSTICK_DOWN: u32 = 22;
    pub const LSTICK_LEFT: u32 = 23;
    pub const RSTICK_UP: u32 = 24;
    pub const RSTICK_RIGHT: u32 = 25;
    pub const RSTICK_DOWN: u32 = 26;
    pub const RSTICK_LEFT: u32 = 27;
    pub const REAL_L: u32 = 28;
    pub const REAL_R: u32 = 29;
}

/// The controller record's style field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Handheld,
    DualJoyCon,
    LeftJoyCon,
    RightJoyCon,
    Pro,
    DebugPad,
    GameCube,
    Unknown,
}

impl Style {
    pub const fn from_raw(raw: u32) -> Style {
        match raw {
            1 => Style::Handheld,
            2 => Style::DualJoyCon,
            3 => Style::LeftJoyCon,
            4 => Style::RightJoyCon,
            5 => Style::Pro,
            6 => Style::DebugPad,
            7 => Style::GameCube,
            _ => Style::Unknown,
        }
    }

    pub const fn is_gamecube(self) -> bool {
        matches!(self, Style::GameCube)
    }
}

const fn has(buttons: u32, bit: u32) -> bool {
    buttons & (1 << bit) != 0
}

/// The logical buttons of `buttons` in the chord order of SPEC-input-04 §3.1 (`chord::NAMES`): A, B, X, Y,
/// L, R, ZL, ZR, D-pad up / down / left / right, Plus, Minus, left and right stick press. L and R follow the
/// controller type: a Pro-style L is `l`, a GameCube pad has none; R is `r`, or the GameCube's `zr`; ZL / ZR
/// are the triggers or the Joy-Con side buttons, and on a GameCube pad its own L and R (SPEC-core-03 §3).
pub fn logical(buttons: u32, style: Style) -> u16 {
    let gc = style.is_gamecube();
    let l = !gc && has(buttons, bit::L);
    let r = if gc { has(buttons, bit::ZR) } else { has(buttons, bit::R) };
    let zl = if gc {
        has(buttons, bit::L) || has(buttons, bit::REAL_L)
    } else {
        has(buttons, bit::ZL) || has(buttons, bit::LEFT_SL) || has(buttons, bit::RIGHT_SL)
    };
    let zr = if gc {
        has(buttons, bit::R) || has(buttons, bit::REAL_R)
    } else {
        has(buttons, bit::ZR) || has(buttons, bit::LEFT_SR) || has(buttons, bit::RIGHT_SR)
    };
    let flags = [
        has(buttons, bit::A),
        has(buttons, bit::B),
        has(buttons, bit::X),
        has(buttons, bit::Y),
        l,
        r,
        zl,
        zr,
        has(buttons, bit::DPAD_UP),
        has(buttons, bit::DPAD_DOWN),
        has(buttons, bit::DPAD_LEFT),
        has(buttons, bit::DPAD_RIGHT),
        has(buttons, bit::PLUS),
        has(buttons, bit::MINUS),
        has(buttons, bit::STICK_L),
        has(buttons, bit::STICK_R),
    ];
    flags.iter().enumerate().fold(0u16, |mask, (i, on)| mask | (u16::from(*on) << i))
}

/// The four directions (up, right, down, left) held by the d-pad, the left stick and the right stick.
fn directions(buttons: u32) -> ([bool; 4], [bool; 4]) {
    let dpad = [bit::DPAD_UP, bit::DPAD_RIGHT, bit::DPAD_DOWN, bit::DPAD_LEFT].map(|b| has(buttons, b));
    let stick = [
        has(buttons, bit::LSTICK_UP) || has(buttons, bit::RSTICK_UP),
        has(buttons, bit::LSTICK_RIGHT) || has(buttons, bit::RSTICK_RIGHT),
        has(buttons, bit::LSTICK_DOWN) || has(buttons, bit::RSTICK_DOWN),
        has(buttons, bit::LSTICK_LEFT) || has(buttons, bit::RSTICK_LEFT),
    ];
    (dpad, stick)
}

const DIRECTION_BUTTONS: [Button; 4] = [Button::Up, Button::Right, Button::Down, Button::Left];

/// Chord-order bits of the logical buttons the menu reads.
const A: u16 = 1 << 0;
const B: u16 = 1 << 1;
const X: u16 = 1 << 2;
const Y: u16 = 1 << 3;
const L: u16 = 1 << 4;
const R: u16 = 1 << 5;
const ZL: u16 = 1 << 6;
const ZR: u16 = 1 << 7;
const DPAD_UP: u16 = 1 << 8;

/// The permanent opening chord: B and D-pad Up (always available in Training, cannot be configured away).
pub const PERMANENT: Chord = Chord(B | DPAD_UP);

/// What one poll asks the menu to do. A poll holds at most the presses of one frame: four directions and
/// the eight buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Step {
    /// A closed menu was asked to open (a chord passed).
    pub open: bool,
    /// Logical chord fields from this same raw poll, before pause suppression.
    pub held: u16,
    pub pressed: u16,
    /// Release-only polls must reach an active physical chord capture too.
    pub changed: bool,
    pub presses: [Option<Button>; 12],
}

impl Step {
    pub fn presses(&self) -> impl Iterator<Item = Button> + '_ {
        self.presses.iter().flatten().copied()
    }

    fn push(&mut self, button: Button) {
        if let Some(slot) = self.presses.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(button);
        }
    }
}

/// The adapter's memory between polls: the held buttons, and the stick hold counters.
#[derive(Clone, Copy, Debug, Default)]
pub struct Adapter {
    logical: u16,
    dpad: [bool; 4],
    stick: [bool; 4],
    repeat: [HoldRepeat; 4],
}

impl Adapter {
    pub const fn new() -> Adapter {
        Adapter { logical: 0, dpad: [false; 4], stick: [false; 4], repeat: [HoldRepeat::new(); 4] }
    }

    /// The edge rule of a chord: every button held and at least one pressed this poll.
    fn chord_passes(chord: Chord, held: u16, pressed: u16) -> bool {
        !chord.is_empty() && chord.0 & held == chord.0 && chord.0 & pressed != 0
    }

    /// One poll. `open` is whether the menu owns input now; `chords_allowed` is false while the game's pause
    /// menu is open (SPEC-core-03 §2.4); `shortcuts` are the player's two configured chords (N7).
    pub fn poll(&mut self, buttons: u32, style: Style, open: bool, chords_allowed: bool, shortcuts: [Chord; 2]) -> Step {
        let logical = logical(buttons, style);
        let pressed = logical & !self.logical;
        let (dpad, stick) = directions(buttons);
        let mut step = Step { held: logical, pressed, changed: logical != self.logical, ..Step::default() };
        if open {
            for i in 0..4 {
                let repeat = self.repeat[i].frame(stick[i]);
                // A d-pad direction presses on its edge only; a stick direction on its edge and every 21 frames.
                if (dpad[i] && !self.dpad[i]) || (stick[i] && !self.stick[i]) || repeat {
                    step.push(DIRECTION_BUTTONS[i]);
                    if !repeat {
                        self.repeat[i].reset();
                    }
                }
            }
            for (bit, button) in [(A, Button::A), (B, Button::B), (X, Button::X), (Y, Button::Y), (L, Button::L), (R, Button::R), (ZL, Button::ZL), (ZR, Button::ZR)] {
                if pressed & bit != 0 {
                    step.push(button);
                }
            }
        } else {
            for repeat in &mut self.repeat {
                repeat.reset();
            }
            if chords_allowed
                && [PERMANENT, shortcuts[0], shortcuts[1]].into_iter().any(|chord| Self::chord_passes(chord, logical, pressed))
            {
                step.open = true;
            }
        }
        self.logical = logical;
        self.dpad = dpad;
        self.stick = stick;
        step
    }

    /// Forget what was held: the menu just opened or closed, so the press that did it is not read again.
    pub fn resync(&mut self, buttons: u32, style: Style) {
        self.logical = logical(buttons, style);
        let (dpad, stick) = directions(buttons);
        self.dpad = dpad;
        self.stick = stick;
        for repeat in &mut self.repeat {
            repeat.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_SHORTCUTS: [Chord; 2] = [Chord::NONE, Chord::NONE];

    fn mask(bits: &[u32]) -> u32 {
        bits.iter().fold(0, |m, b| m | (1 << b))
    }

    #[test]
    fn hold_b_press_up_or_hold_up_press_b_opens_and_holding_both_does_not_reopen() {
        let mut a = Adapter::new();
        assert!(!a.poll(mask(&[bit::B]), Style::Pro, false, true, NO_SHORTCUTS).open);
        assert!(a.poll(mask(&[bit::B, bit::DPAD_UP]), Style::Pro, false, true, NO_SHORTCUTS).open);
        // Held on: no edge, no second open.
        assert!(!a.poll(mask(&[bit::B, bit::DPAD_UP]), Style::Pro, false, true, NO_SHORTCUTS).open);
        let mut a = Adapter::new();
        assert!(!a.poll(mask(&[bit::DPAD_UP]), Style::Pro, false, true, NO_SHORTCUTS).open);
        assert!(a.poll(mask(&[bit::DPAD_UP, bit::B]), Style::Pro, false, true, NO_SHORTCUTS).open);
    }

    #[test]
    fn the_chord_is_refused_while_the_pause_menu_owns_input() {
        let mut a = Adapter::new();
        a.poll(mask(&[bit::B]), Style::Pro, false, false, NO_SHORTCUTS);
        assert!(!a.poll(mask(&[bit::B, bit::DPAD_UP]), Style::Pro, false, false, NO_SHORTCUTS).open);
    }

    #[test]
    fn a_configured_shortcut_opens_and_the_default_minus_is_an_edge() {
        let mut a = Adapter::new();
        let minus = [Chord::DEFAULT_SHORTCUT_1, Chord::NONE];
        assert!(a.poll(mask(&[bit::MINUS]), Style::Pro, false, true, minus).open);
        assert!(!a.poll(mask(&[bit::MINUS]), Style::Pro, false, true, minus).open);
        assert!(!a.poll(0, Style::Pro, false, true, minus).open);
        assert!(a.poll(mask(&[bit::MINUS]), Style::Pro, false, true, minus).open);
        // A disabled (empty) shortcut never passes.
        let mut a = Adapter::new();
        assert!(!a.poll(mask(&[bit::MINUS]), Style::Pro, false, true, NO_SHORTCUTS).open);
    }

    #[test]
    fn plus_alone_never_opens_it_stays_the_games_pause() {
        let mut a = Adapter::new();
        assert!(!a.poll(mask(&[bit::PLUS]), Style::Pro, false, true, [Chord::DEFAULT_SHORTCUT_1, Chord::NONE]).open);
    }

    #[test]
    fn an_open_menu_reads_edges_of_the_dpad_and_buttons_and_repeats_only_the_stick() {
        let mut a = Adapter::new();
        let s = a.poll(mask(&[bit::DPAD_DOWN, bit::A]), Style::Pro, true, true, NO_SHORTCUTS);
        assert_eq!(s.presses().collect::<Vec<_>>(), [Button::Down, Button::A]);
        // The d-pad held does not repeat, however long.
        for _ in 0..60 {
            assert_eq!(a.poll(mask(&[bit::DPAD_DOWN, bit::A]), Style::Pro, true, true, NO_SHORTCUTS).presses().count(), 0);
        }
        // A stick direction presses on its edge, then every 21 frames.
        let mut a = Adapter::new();
        let held = mask(&[bit::LSTICK_UP]);
        assert_eq!(a.poll(held, Style::Pro, true, true, NO_SHORTCUTS).presses().collect::<Vec<_>>(), [Button::Up]);
        let mut fired = Vec::new();
        for frame in 1..=50 {
            if a.poll(held, Style::Pro, true, true, NO_SHORTCUTS).presses().count() > 0 {
                fired.push(frame);
            }
        }
        assert_eq!(fired, [21, 42]);
    }

    #[test]
    fn shoulder_buttons_follow_the_controller_type() {
        assert_eq!(logical(mask(&[bit::L]), Style::Pro), L);
        assert_eq!(logical(mask(&[bit::L]), Style::GameCube), ZL);
        assert_eq!(logical(mask(&[bit::ZR]), Style::GameCube), R);
        assert_eq!(logical(mask(&[bit::R]), Style::GameCube), ZR);
        assert_eq!(logical(mask(&[bit::LEFT_SL]), Style::DualJoyCon), ZL);
        assert_eq!(logical(mask(&[bit::RIGHT_SR]), Style::Pro), ZR);
        assert_eq!(logical(mask(&[bit::REAL_L]), Style::GameCube), ZL);
    }

    #[test]
    fn the_open_press_is_not_read_as_a_menu_press_after_resync() {
        let mut a = Adapter::new();
        let held = mask(&[bit::B, bit::DPAD_UP]);
        a.poll(mask(&[bit::B]), Style::Pro, false, true, NO_SHORTCUTS);
        assert!(a.poll(held, Style::Pro, false, true, NO_SHORTCUTS).open);
        a.resync(held, Style::Pro);
        // Still holding both next poll: nothing is pressed, B is not a press.
        assert_eq!(a.poll(held, Style::Pro, true, true, NO_SHORTCUTS).presses().count(), 0);
        // Releasing then pressing B closes.
        a.poll(0, Style::Pro, true, true, NO_SHORTCUTS);
        assert_eq!(a.poll(mask(&[bit::B]), Style::Pro, true, true, NO_SHORTCUTS).presses().collect::<Vec<_>>(), [Button::B]);
    }
}
