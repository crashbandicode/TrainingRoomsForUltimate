//! Compile-time compatibility shim until the BTT owner publishes placing_motion.
//! Replace this module with a re-export on the BTT commit; no Btt::placing edits.
use crate::model::btt::Pad;
use crate::model::chords::logical::{B, DPAD_DOWN, DPAD_LEFT, DPAD_RIGHT, DPAD_UP, ZL};
const SPEED: f32 = 2.0;
const DEAD: f32 = 0.3;
const DPAD: u16 = 1 << DPAD_UP | 1 << DPAD_DOWN | 1 << DPAD_LEFT | 1 << DPAD_RIGHT;
const fn bit(button: u8) -> u16 {
    1 << button
}
/// Shared placing-view cursor controls: stick speed/deadzone and D-pad repeat.
/// Both target and Tech point editors use the same logical buttons and cadence.
pub fn placing_motion(p: &Pad, was: u16, repeat_age: &mut u8) -> [f32; 2] {
    let down = |k: u8| p.held & bit(k) != 0;
    let pad = p.held & DPAD;
    *repeat_age = repeat_age.saturating_add(1) * u8::from(pad != 0 && pad == was & DPAD);
    let repeat = *repeat_age == 0 || *repeat_age >= 15 && *repeat_age % 2 == 1;
    let step = f32::from(pad != 0 && !down(ZL) && !down(B) && repeat);
    let unit = [[DPAD_RIGHT, DPAD_LEFT], [DPAD_UP, DPAD_DOWN]]
        .map(|[hi, lo]| step * (f32::from(down(hi)) - f32::from(down(lo))));
    [0, 1].map(|i| unit[i] + p.stick[i] * SPEED * f32::from(p.stick[i].abs() >= DEAD))
}
