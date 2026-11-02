//! A chord: a set of the 16 logical buttons of SPEC-input-04 §3.1 (bit `i` is button `i` in the
//! order of the chord settings' arrays), and how it reads aloud. The menu edits chords (the six chord
//! rows and the two menu shortcuts, N7); matching them against the controller is package 4.

use core::fmt;

/// Number of logical buttons.
pub const BUTTONS: usize = 16;

/// The logical buttons in bit order (the order of the 16-element chord arrays).
pub const NAMES: [&str; BUTTONS] = [
    "A", "B", "X", "Y", "L", "R", "ZL", "ZR", "D-pad Up", "D-pad Down", "D-pad Left", "D-pad Right", "Plus", "Minus",
    "Left Stick Press", "Right Stick Press",
];

/// Bit of `Minus` (the default of menu shortcut 1).
pub const MINUS: u8 = 13;

/// A set of logical buttons. The empty chord never passes (SPEC-input-04 §2.2); for a menu
/// shortcut it means `disabled`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Chord(pub u16);

impl Chord {
    pub const NONE: Chord = Chord(0);
    /// The default of menu shortcut 1.
    pub const DEFAULT_SHORTCUT_1: Chord = Chord(1 << MINUS);

    /// The chord of a 16-element settings array (any non-zero element is a member).
    pub fn from_row(row: &[u8]) -> Chord {
        let mut mask = 0u16;
        for (bit, value) in row.iter().take(BUTTONS).enumerate() {
            if *value != 0 {
                mask |= 1 << bit;
            }
        }
        Chord(mask)
    }

    /// The 16-element settings array (`1` for a member).
    pub fn to_row(self) -> [u8; BUTTONS] {
        let mut row = [0u8; BUTTONS];
        for (bit, slot) in row.iter_mut().enumerate() {
            *slot = u8::from(self.0 & (1 << bit) != 0);
        }
        row
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, bit: u8) -> bool {
        self.0 & (1 << bit) != 0
    }

    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }
}

/// `ZL + D-pad Up` style text; an empty chord reads `disabled`.
impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("disabled");
        }
        let mut first = true;
        for (bit, name) in NAMES.iter().enumerate() {
            if self.0 & (1 << bit) != 0 {
                if !first {
                    f.write_str(" + ")?;
                }
                f.write_str(name)?;
                first = false;
            }
        }
        Ok(())
    }
}
