//! Hide CPU (rc.14, SPEC-eggsercise V2.6), the Switch half. The rule and each body's step are `model::cpu::hide`; every
//! native used here has a seam row (`native_seams::rows_hide_cpu`, group `hide_cpu`), compared once at boot.
//!
//! No hook of its own: the CPU frame wrapper (`cpu::fighter_frame`) asks the model once per body frame and `apply` does
//! the step. A hidden body is invisible (model, effects, shadow), has no hurtbox and no push, has no hitbox out, and waits in
//! the game's own hitlag, renewed every frame, so its status, motion and fall wait with it. Shown, all of it comes back
//! and the body goes on from the same spot and status. The practice games ask through `hide` and `restore`.
//!
//! The player tag goes with the body: the game's two per-fighter cursor flags (`hide::tag`), written through package 2's
//! guarded `WorkModule::set_flag`. The HUD panel too: the wrapper publishes the hidden entries (`publish_panels`) and the
//! `info_melee` tick hides those parts (`runtime::ui::hud::cpu_panels`); in a waiting room the `info_waiting` tick hides
//! the CPU's partner card instead (`room_card`).

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

use crate::model::cpu::hide;
use crate::model::cpu::ids::UNRESOLVED;
use crate::runtime::features::notices;
use crate::runtime::game::{self, natives};
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
/// The entries whose HUD panel is hidden, one bit each (`Cpu::hidden_panels`), for the `info_melee` tick.
static PANELS: AtomicU8 = AtomicU8::new(0);

/// From the CPU frame wrapper after each body frame and on a session clear: the entries whose panel is hidden.
pub(crate) fn publish_panels(bits: u8) {
    let bits = bits * u8::from(BASE.load(Ordering::Acquire) != 0);
    PANELS.store(bits, Ordering::Release);
}

/// The entries whose HUD panel is hidden.
pub(crate) fn panels() -> u8 {
    PANELS.load(Ordering::Acquire)
}

/// The fighter object whose player tag the Eggs-ercise CPU target's pass wants kept on this tick (`keep_cursor`), else 0.
static KEPT: AtomicUsize = AtomicUsize::new(0);

/// Whether a CPU body was hidden by its last step (the hide step's off-write owns its tag then).
pub(crate) fn hidden() -> bool {
    HIDDEN.load(Ordering::Acquire)
}

/// From the CPU target's pass (`hitbox::eggs_cpu`): `keep` (`eggs::keeps_cursor`) asks for `fighter`'s tag flag to be written
/// on right after this pass. It only stores a value; the write is `write_kept_cursor`'s.
pub(crate) fn keep_cursor(fighter: usize, keep: bool) {
    KEPT.store(fighter * usize::from(keep), Ordering::Release);
}

/// From the CPU frame wrapper right after the hitbox pass of `fighter`: when that pass asked for it, write `FLAG_CURSOR` = true
/// (`flag`, from the game's constant table; `UNRESOLVED` skips it) through package 2's guarded `WorkModule::set_flag`, the call the
/// hide step's tag write uses. Taken once: a tick whose pass asked for nothing writes nothing.
///
/// # Safety
/// The game thread, with `fighter` the fighter object the game just ran a frame of.
pub(crate) unsafe fn write_kept_cursor(fighter: usize, flag: i32) {
    if fighter == 0 || flag == UNRESOLVED || KEPT.compare_exchange(fighter, 0, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return;
    }
    let accessor = natives::fighter_accessor(fighter);
    if accessor != 0 {
        natives::set_flag(accessor, true, flag);
    }
}

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

/// Do one body's step. `tag` is the two cursor flags' ids from the game's constant table (`UNRESOLVED` skips one).
///
/// # Safety
/// The game thread, with `accessor` the module accessor of a live fighter.
pub(crate) unsafe fn apply(accessor: usize, step: Option<bool>, tag: [i32; 2]) {
    let base = BASE.load(Ordering::Acquire);
    let Some(hidden) = step.filter(|_| base != 0) else { return };
    HIDDEN.store(hidden, Ordering::Release);
    for (_, offset, hid, vis) in NATIVES {
        let [a, b] = if hidden { hid } else { vis };
        let call: extern "C" fn(usize, usize, usize) = std::mem::transmute(base + offset);
        call(accessor, a, b);
    }
    if let Some(shown) = hide::tag(step) {
        for flag in tag.into_iter().filter(|&flag| flag != UNRESOLVED) {
            natives::set_flag(accessor, shown, flag);
        }
    }
}
