//! The fork's `common::input::{Buttons, MappedInputs}` as the carried files name them: the game's final mapped input record (8 bytes,
//! SPEC-input-01 section 3.1) with its button set as a small flag type. Same layout as `model::input_delay::MappedInputs`; the
//! input stage converts at the call (`From` both ways), so the carried cycle code edits a copy of the record and the stage writes
//! it back.

use std::ops::BitOr;

use crate::model::input_delay::MappedInputs as Record;

/// The mapped ("Smash") button bits (SPEC-input-01 section 3.1): attack 0x1, special 0x2, jump 0x4, guard 0x8, ...
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Buttons(u32);

impl Buttons {
    pub const SPECIAL: Buttons = Buttons(0x2);
    pub const SPECIAL_RAW: Buttons = Buttons(0x800);
    pub const SPECIAL_RAW2: Buttons = Buttons(0x2_0000);

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn from_bits_truncate(bits: u32) -> Buttons {
        Buttons(bits)
    }

    pub const fn intersects(self, other: Buttons) -> bool {
        self.0 & other.0 != 0
    }

    pub fn remove(&mut self, other: Buttons) {
        self.0 &= !other.0;
    }
}

impl BitOr for Buttons {
    type Output = Buttons;
    fn bitor(self, other: Buttons) -> Buttons {
        Buttons(self.0 | other.0)
    }
}

/// The game's mapped input record: the buttons and the two sticks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct MappedInputs {
    pub buttons: Buttons,
    pub lstick_x: i8,
    pub lstick_y: i8,
    pub rstick_x: i8,
    pub rstick_y: i8,
}

const _: () = assert!(core::mem::size_of::<MappedInputs>() == core::mem::size_of::<Record>());

impl MappedInputs {
    /// No buttons, both sticks centred.
    pub const fn empty() -> MappedInputs {
        MappedInputs { buttons: Buttons(0), lstick_x: 0, lstick_y: 0, rstick_x: 0, rstick_y: 0 }
    }
}

impl From<Record> for MappedInputs {
    fn from(record: Record) -> MappedInputs {
        MappedInputs {
            buttons: Buttons(record.buttons),
            lstick_x: record.lstick_x,
            lstick_y: record.lstick_y,
            rstick_x: record.rstick_x,
            rstick_y: record.rstick_y,
        }
    }
}

impl From<MappedInputs> for Record {
    fn from(inputs: MappedInputs) -> Record {
        Record {
            buttons: inputs.buttons.0,
            lstick_x: inputs.lstick_x,
            lstick_y: inputs.lstick_y,
            rstick_x: inputs.rstick_x,
            rstick_y: inputs.rstick_y,
        }
    }
}
