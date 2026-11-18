//! Read-only Control reader. Entry is admitted by Mash's full boot closure, never by a stock default.
//! Layout: approved proposal3979f5d6; main6b9150/3f89b0/3f9140/6b9324.

use crate::model::mashout::native::{mapped, Input, Record, Seed};
use crate::runtime::game;

#[derive(Clone, Copy, Debug)]
pub(super) struct Sample {
    pub record: Record,
    pub input: Input,
    pub current: u32,
    #[cfg(feature = "diag")]
    pub previous: u32,
    pub alternate: u32,
    pub history: u8,
}

impl Sample {
    pub fn seed(self) -> Seed {
        // The next native copier takes this history source before forming its trigger.
        let held = if self.history == 0 {
            self.alternate
        } else {
            self.current
        };
        Seed {
            record: self.record,
            held: mapped(held),
            deadline: None,
        }
    }
}

unsafe fn read<T: Copy>(base: usize, offset: usize) -> Option<T> {
    let address = base.checked_add(offset)?;
    (base != 0 && address % core::mem::align_of::<T>() == 0).then(|| (address as *const T).read())
}

/// A fresh live BOMA, checked against P1's current active body by the caller.
pub(super) unsafe fn control(accessor: usize) -> Option<usize> {
    let control = read::<usize>(accessor, 0x48)?;
    let table = read::<usize>(control, 0)?;
    // Check the live concrete dispatch slot, rather than assuming every Control subclass has this layout.
    super::interposer::reader_table(table).then_some(control)
}

/// Actual getter, sampled before arming; no stock 0.5 fallback. Called only on the game thread.
pub(super) unsafe fn threshold(accessor: usize) -> Option<f32> {
    if !game::granted(0x6ba1e0) {
        return None;
    }
    let control = control(accessor)?;
    let getter: extern "C" fn(usize) -> f32 = core::mem::transmute(game::text_base() + 0x6ba1e0);
    let value = getter(control);
    (value.is_finite() && value >= 0.0).then_some(value)
}

/// No allocation, lock, writes or parameter getter. Ordinary Grab uses record0, source -1.
pub(super) unsafe fn sample(accessor: usize, threshold: f32) -> Option<Sample> {
    let control = control(accessor)?;
    if read::<i32>(control, 0x10)? <= 0 {
        return None;
    }
    let record = read::<usize>(control, 0x18)?;
    let at = |offset| read::<f32>(record, offset);
    let byte = |offset| read::<i8>(record, offset);
    let record = Record {
        remaining: at(0)?,
        automatic: at(4)?,
        manual: at(8)?,
        button_mul: at(12)?,
        threshold,
        sign: [byte(0x10)?, byte(0x11)?],
        source: byte(0x12)?,
        stick_block: byte(0x13)?,
        button_block: byte(0x14)?,
        accepted: read::<u8>(record, 0x15)?,
        other_block: byte(0x16)?,
        cross_reload: byte(0x17)?,
        other_reload: byte(0x18)?,
    };
    if !record.valid() {
        return None;
    }
    let input = Input {
        axes: [read::<f32>(control, 0x30)?, read::<f32>(control, 0x34)?],
        trigger: read::<u32>(control, 0xa8)?,
    };
    if !input
        .axes
        .iter()
        .all(|x| x.is_finite() && (-1.0..=1.0).contains(x))
    {
        return None;
    }
    Some(Sample {
        record,
        input,
        current: read::<u32>(control, 0x9c)?,
        #[cfg(feature = "diag")]
        previous: read::<u32>(control, 0xa0)?,
        alternate: read::<u32>(control, 0xa4)?,
        history: read::<u8>(control, 0x102)?,
    })
}
