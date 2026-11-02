//! Draw callbacks of the other packages' panels (notifications, input display, damage readouts): each is
//! called for every layout tick with the layout and its name, and gates on the name itself
//! (SPEC-core-05 §5: "name-gated draws"). Registered at boot; the tick reads the slots lock-free.

use std::sync::atomic::{AtomicUsize, Ordering};

use skyline::nn::ui2d::Layout;

/// `layout` is the layout being ticked, `name` its name bytes (e.g. `info_training`).
pub type HudDraw = unsafe fn(layout: *mut Layout, name: &[u8]);

const SLOTS: usize = 8;
static DRAWS: [AtomicUsize; SLOTS] = [const { AtomicUsize::new(0) }; SLOTS];

/// Register a callback; returns false when all slots are taken.
pub fn register(draw: HudDraw) -> bool {
    for slot in &DRAWS {
        if slot.compare_exchange(0, draw as usize, Ordering::AcqRel, Ordering::Acquire).is_ok() {
            return true;
        }
    }
    false
}

pub(super) unsafe fn run(layout: *mut Layout, name: &[u8]) {
    for slot in &DRAWS {
        let value = slot.load(Ordering::Acquire);
        if value != 0 {
            // SAFETY: only `register` stores a non-zero value, and it stores a `HudDraw`.
            let draw: HudDraw = core::mem::transmute::<usize, HudDraw>(value);
            draw(layout, name);
        }
    }
}