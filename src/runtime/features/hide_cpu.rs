//! Hide CPU (rc.14, SPEC-eggsercise V2.6), the Switch half. The rule and each body's step are `model::cpu::hide`; every
//! native used here has a seam row (`native_seams::rows_hide_cpu`, group `hide_cpu`), compared once at boot.
//!
//! No hook of its own: the CPU frame wrapper (`cpu::fighter_frame`) asks the model once per body frame and `apply` does
//! the step. A hidden body is invisible (model, effects, shadow), has no hurtbox and no push, has no hitbox out, and waits in
//! the game's own hitlag, renewed every frame, so its status, motion and fall wait with it. Shown, all of it comes back
//! and the body goes on from the same spot and status. The practice games ask through `hide` and `restore`.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::model::cpu::hide;
use crate::runtime::features::notices;
use crate::runtime::game;
use crate::runtime::seams::seam_install::gate;

/// Every native takes the accessor and two more words, so one call shape serves them all (a bool or an i32 argument is read
/// from the low bits). Each row: its seam label, its `main` offset, and the two words while hidden and while shown.
/// `VisibilityModule`, `EffectModule` (whole), `ShadowModule` and `JostleModule` take `shown`; `HitModule::set_whole` the
/// `HitStatus` (3 off, 0 normal) and the part; `AttackModule::clear_all` nothing; `StopModule::set_hit_stop_frame` the
/// frames (3, renewed every frame; 0 ends it) and `false`.
const NATIVES: [(&str, usize, [usize; 2], [usize; 2]); 7] = [
    ("hide_cpu.visibility", 0x208c0c0, [0, 0], [1, 0]),
    ("hide_cpu.effects", 0x2017a70, [0, 0], [1, 0]),
    ("hide_cpu.shadow", 0x2072af0, [0, 0], [1, 0]),
    ("hide_cpu.jostle", 0x2040d40, [0, 0], [1, 0]),
    ("hide_cpu.hurtbox", 0x203db20, [3, 0], [0, 0]),
    ("hide_cpu.attacks", 0x1fd00c0, [0, 0], [0, 0]),
    ("hide_cpu.stop", 0x2089260, [3, 0], [0, 0]),
];

/// `main`'s base once every row passed the authoritative gate at boot, else 0.
static BASE: AtomicUsize = AtomicUsize::new(0);
/// A practice game asked for the CPU to be hidden (`hide`).
pub(crate) static REQUEST: AtomicBool = AtomicBool::new(false);
/// A CPU body was hidden by its last step.
static HIDDEN: AtomicBool = AtomicBool::new(false);

/// Once from `lobby::init`: compare every row (the installer logs a group it skips); nothing is written unless all passed.
pub(crate) fn init() {
    let rows = gate("hide_cpu", &NATIVES.map(|(label, ..)| label), false);
    BASE.store(game::text_base() * usize::from(rows), Ordering::Release);
}

/// A practice game's request: with `on`, the CPU is hidden from its next frame whatever the Hide CPU row says, until
/// `restore` (or `hide(false)`). Call it every frame or on each change; it only stores a flag.
pub fn hide(on: bool) {
    REQUEST.store(on, Ordering::Release);
}

/// The practice game ended: the CPU is as the Hide CPU row says.
pub fn restore() {
    hide(false);
}

/// Play on CPU while the CPU is hidden is refused, with the notice (SPEC-eggsercise V2.6).
pub(crate) fn refuses_play() -> bool {
    let hidden = HIDDEN.load(Ordering::Acquire);
    if hidden {
        notices::refuse(hide::refusal(REQUEST.load(Ordering::Acquire)));
    }
    hidden
}

/// Do one body's step.
///
/// # Safety
/// The game thread, with `accessor` the module accessor of a live fighter.
pub(crate) unsafe fn apply(accessor: usize, step: Option<bool>) {
    let base = BASE.load(Ordering::Acquire);
    let Some(hidden) = step.filter(|_| base != 0) else { return };
    HIDDEN.store(hidden, Ordering::Release);
    for (_, offset, hid, vis) in NATIVES {
        let [a, b] = if hidden { hid } else { vis };
        let call: extern "C" fn(usize, usize, usize) = std::mem::transmute(base + offset);
        call(accessor, a, b);
    }
}
