//! The two sites of the native pause owner (SPEC-input-03 §4, review R2 of package 1): the begin and the
//! end of the 13.0.5 melee scene's `StatePause`, the state Training enters when the player pauses the
//! match. Rows are checked against the owned 13.0.5 `main` by `tests/pause_sites.rs`; the install line
//! reports them after `sites::SITES`.
//!
//! Why these are the authoritative native transition (all read from the owned ELF; offsets from `.text`):
//!
//! - The scene state machine registered by `main+0x14d1ec0` (the one holding `StateResetTraining`, so
//!   the Training scene) maps the name `StatePause` to the factory `main+0x14d3180`, which builds an
//!   object whose vtable is at `main+0x506bd18`.
//! - Slot 2 of that vtable (`main+0x14d47b0`) is the state's enter: it loads the pause manager and
//!   calls the game's pause-begin routine `main+0x150f0c0`, which raises the pause flags (the byte at
//!   `+0xa9` of the object behind `0x5336e10`, `+0x568e` and `+0x2d` of the others it reaches).
//! - Slot 3 (`main+0x14d47d0`) is the state's exit and clears exactly those flags; all its paths
//!   leave through one epilogue at `main+0x14d4868`.
//!
//! So "pause begins" is the instruction `main+0x14d47c0` (the enter's `ldr x0, [x8]`, just before the
//! call) and "pause ends" is `main+0x14d4868` (the exit's `ldp x29, x30` epilogue). Both are inline hooks
//! on instructions that are not PC-relative. The other scenes that have a `StatePause` (the home-run
//! contest and the staff roll) call the same begin routine from their own classes and are not hooked: the
//! owner mask only matters in native Training.
//!
//! **Eden result (2026-11-02, labels r1-03 to r1-06, diag probes at the begin routine, both enter slots, the
//! exit and the routine's other callers, with a hook on `is_training_mode` as the control that fired):
//! the native Training pause menu ("Quit Training", opened with Plus) does NOT run this state.** None
//! of these sites fired while the menu was opened and closed, and the per-frame fighter hooks kept running
//! at the same rate throughout (the Training menu does not stop the frame loop). The chain above is
//! right for the scene's `StatePause` (the Versus-style pause; not exercised in Eden), so the two hooks
//! are installed and guarded but have no effect in Training. The authoritative Training pause signal is
//! not found yet: the menu is the layout-driven object family at `main+0x13af5f0` / `0x13e4ca0..0x13e6fe0`
//! (per-page callbacks, page index at `+0x1fc`), not a scene state.

use super::guard::{HookForm, Placement, SeamKind};
use super::sites::{Feature, Site};

/// `StatePause::enter`, before the call to the pause-begin routine.
pub const PAUSE_ENTER_OFFSET: usize = 0x14d_47c0;
/// `StatePause::exit`'s epilogue (every path of the function ends here).
pub const PAUSE_EXIT_OFFSET: usize = 0x14d_4868;

pub static SITES: [Site; 2] = [
    Site {
        name: "pause_enter",
        feature: Feature::PauseOwner,
        offset: PAUSE_ENTER_OFFSET,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0x00, 0x01, 0x40, 0xf9, 0x3f, 0xea, 0x00, 0x94, 0xfd, 0x7b, 0xc1, 0xa8, 0xc0, 0x03, 0x5f, 0xd6,
            0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91, 0x28, 0xef, 0x01, 0xb0, 0x08, 0x81, 0x0b, 0x91,
            0x08, 0xfd, 0xdf, 0x08, 0x68, 0x04, 0x00, 0x36, 0x28, 0xef, 0x01, 0xb0, 0x08, 0xc1, 0x4b, 0x39,
            0xe8, 0x00, 0x00, 0x35, 0x88, 0xef, 0x01, 0x90, 0x08, 0x39, 0x40, 0xf9, 0x08, 0x55, 0x40, 0xf9,
        ],
        note: "The Training scene's StatePause enter (vtable 0x506bd18 slot 2): `ldr x0, [x8]` loading the pause manager just before `bl 0x150f0c0`, the game's pause-begin routine. The hook runs as the native pause begins: the plugin takes over P1's input and empties the delay queue.",
    },
    Site {
        name: "pause_exit",
        feature: Feature::PauseOwner,
        offset: PAUSE_EXIT_OFFSET,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0xfd, 0x7b, 0xc1, 0xa8, 0xc0, 0x03, 0x5f, 0xd6, 0x20, 0xef, 0x01, 0xb0, 0x00, 0x80, 0x0b, 0x91,
            0x8e, 0xaf, 0x93, 0x94, 0x60, 0xfb, 0xff, 0x34, 0x20, 0xef, 0x01, 0xb0, 0x00, 0x80, 0x0b, 0x91,
            0x8e, 0xaf, 0x93, 0x94, 0xd7, 0xff, 0xff, 0x17, 0xc0, 0x03, 0x5f, 0xd6, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x03, 0x5f, 0xd6, 0x00, 0x00, 0x00, 0x00,
        ],
        note: "The same StatePause's exit (vtable 0x506bd18 slot 3), at the epilogue `ldp x29, x30, [sp], #0x10` every path of the function reaches after it cleared the pause flags the enter raised. The hook runs as the native pause ends: the plugin releases P1's input.",
    },
];

/// The row called `name`.
pub fn find(name: &str) -> Option<&'static Site> {
    SITES.iter().find(|s| s.name == name)
}

/// Every row of `feature`.
pub fn sites_of(feature: Feature) -> impl Iterator<Item = &'static Site> {
    SITES.iter().filter(move |s| s.feature == feature)
}
