//! The settings every hook reads, as atomics: replaced as a whole at load and at every commit
//! (SPEC-core-04 §5, SPEC-core-01 question 10), read without a lock.
//!
//! NOT YET SWITCH-COMPILED.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use crate::model::settings::Snapshot;

static INPUT_DELAY: AtomicU8 = AtomicU8::new(0);
static LRA_RESET: AtomicBool = AtomicBool::new(true);
static STAGE_HAZARDS: AtomicBool = AtomicBool::new(false);
static DEVELOPER_PAGE: AtomicBool = AtomicBool::new(false);
static HUD: AtomicBool = AtomicBool::new(true);

/// Diag build only: which step of a publish ran (the menu-close hang, 2026-11-02: the commit never logged `menu closed`).
#[inline]
fn probe(step: u8) {
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[Ui] probe publish {step}"));
    let _ = step;
}

/// Make `snapshot` the live settings. `rerolls` is the menu outbox's re-roll bits (decisions Q18 A,
/// B11); the first publish at boot has none.
pub fn publish(snapshot: &Snapshot, rerolls: u8) {
    #[cfg(feature = "diag")]
    crate::runtime::diag_reload::remember(snapshot);
    probe(1);
    crate::runtime::features::cpu::publish(snapshot, rerolls);
    probe(2);
    crate::runtime::features::states::publish(crate::model::state::settings::StateSettings::from_snapshot(snapshot));
    probe(3);
    crate::runtime::features::recorder::publish(snapshot);
    probe(4);
    crate::runtime::compat::menu::apply_snapshot(snapshot);
    probe(5);
    crate::runtime::features::chords::publish(snapshot);
    probe(6);
    crate::runtime::p1::publish(snapshot);
    probe(7);
    crate::runtime::features::input_display::publish(snapshot);
    probe(8);
    INPUT_DELAY.store(snapshot.input_delay(), Ordering::Release);
    LRA_RESET.store(snapshot.lra_reset(), Ordering::Release);
    STAGE_HAZARDS.store(snapshot.stage_hazards(), Ordering::Release);
    HUD.store(snapshot.hud(), Ordering::Release);
}

pub fn input_delay() -> u8 {
    INPUT_DELAY.load(Ordering::Acquire)
}

pub fn lra_reset() -> bool {
    LRA_RESET.load(Ordering::Acquire)
}

/// The hud setting (SPEC-core-05 §2).
pub fn hud() -> bool {
    HUD.load(Ordering::Acquire)
}

pub fn stage_hazards() -> bool {
    STAGE_HAZARDS.load(Ordering::Acquire)
}

/// Decision N1: whether `developer.ini` said `developer_page=on` at boot (the menu reads it in a
/// later package).
pub fn developer_page() -> bool {
    DEVELOPER_PAGE.load(Ordering::Acquire)
}

pub fn set_developer_page(on: bool) {
    DEVELOPER_PAGE.store(on, Ordering::Release);
}

/// What other than gameplay owns P1's input right now (`Owners` bits). The packages that own those
/// states set and clear their bit with `model::input_adapter::{take, release}`: the plugin menu and
/// modal panels (package 5), the named stop chord and the replay editor (package 4), the native
/// waiting-room UI (package 6). The native Training menu is set and released by the UI tick from the
/// game's layout state (`ui::state::PAUSE_ACTIVE`, via `InputAdapter::pause_flag`). The input-delay adapter
/// reads the mask on every poll.
pub static INPUT_OWNERS: AtomicU8 = AtomicU8::new(0);

/// The owners now (pkg4's chords, recorder and notices and pkg5's display read it).
pub fn input_owners() -> crate::model::input_delay::Owners {
    crate::model::input_adapter::owners(&INPUT_OWNERS)
}

/// Set while the hot-swap engine replaces a fighter (package 6); the reset hub does not run then.
pub static HOT_SWAP_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
