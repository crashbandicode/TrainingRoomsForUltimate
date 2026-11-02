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

/// Every feature's settings as a game-thread hook reads them: computed from a snapshot by the publishing thread (boot, the
/// menu worker, the diag reload) and read wait-free by the hooks (stream/sync: a hook never waits on a lock another thread
/// holds across real work; the menu-close hang of 2026-11-02 was the worker and the P1 poll meeting on `chords::STATE`).
/// A feature that keeps per-frame state under its own lock adopts a new value at its next hook ([`live_if_newer`]) under that
/// lock, which only game threads take.
#[derive(Clone, Copy)]
pub(crate) struct Live {
    pub cpu: crate::model::cpu::settings::CpuSettings,
    pub states: crate::model::state::settings::StateSettings,
    pub recorder: crate::model::recording::RecorderSettings,
    /// The six chord rows; the two menu shortcuts are not taken from here (they live in the INI, `chords::shortcuts`).
    pub chords: crate::model::chords::ChordSet,
    pub input_display: crate::model::input_display::Settings,
    /// The typed menu view of the rows (`compat::menu`, decision D1).
    pub menu: crate::engine::compat::menu::TrainingModpackMenu,
}

static LIVE: hos_lock::Published<Option<Live>> = hos_lock::Published::new(None);
/// The menu outbox's re-roll bits the CPU has not taken yet (decisions Q18 A, B11): or-ed by each publish, taken when the CPU
/// adopts a new value.
static REROLLS: AtomicU8 = AtomicU8::new(0);

/// The live settings (None before the first publish). Never waits.
#[inline]
pub(crate) fn live() -> Option<Live> {
    LIVE.read()
}

/// The live settings if they changed since `seen` (one atomic load when they did not). Never waits.
#[inline]
pub(crate) fn live_if_newer(seen: &std::sync::atomic::AtomicU32) -> Option<Live> {
    LIVE.read_if_newer(seen).flatten()
}

/// How many values were published (the typed menu's game-side edits are valid until the next one, `compat::menu`).
#[inline]
pub(crate) fn live_generation() -> u32 {
    LIVE.generation()
}

/// The re-roll bits published since the CPU last took them.
pub(crate) fn take_rerolls() -> u8 {
    REROLLS.swap(0, Ordering::AcqRel)
}

/// Make `snapshot` the live settings. `rerolls` is the menu outbox's re-roll bits (decisions Q18 A,
/// B11); the first publish at boot has none. Runs where a thread may wait (boot, the menu worker, the diag reload), never in a
/// hook, and takes no lock a hook takes: hooks get the values through [`Live`], the flags below are atomics.
/// One publish at a time (boot, the menu worker, the diag reload: never a game thread): the publishing path is the one reader of the
/// typed menu's edit handoff (`compat::menu`).
pub(crate) static PUBLISHING: hos_lock::Mutex<()> = hos_lock::Mutex::new(());

pub fn publish(snapshot: &Snapshot, rerolls: u8) {
    let _turn = PUBLISHING.lock();
    #[cfg(feature = "diag")]
    crate::runtime::diag_reload::remember(snapshot);
    let published_menu = crate::engine::compat::menu::TrainingModpackMenu::from_snapshot(snapshot);
    let previous_menu = crate::runtime::compat::menu::view_before_publish();
    let menu = crate::runtime::compat::menu::before_publish(published_menu, previous_menu);
    let live = Live {
        cpu: crate::model::cpu::settings::CpuSettings::from_snapshot(snapshot),
        states: crate::model::state::settings::StateSettings::from_snapshot(snapshot),
        recorder: crate::model::recording::RecorderSettings::from_snapshot(snapshot),
        chords: crate::model::chords::ChordSet::factory().with_rows(snapshot),
        input_display: crate::model::input_display::Settings::from_snapshot(snapshot),
        menu,
    };
    crate::runtime::features::states::set_flags(&live.states);
    // Macro Source = Named Library: the layer's frame buffers are reserved on this thread, never in a hook.
    crate::runtime::features::recorder::prepare_named_buffers(snapshot);
    REROLLS.fetch_or(rerolls, Ordering::AcqRel);
    LIVE.publish(Some(live));
    crate::runtime::compat::menu::after_publish(published_menu, menu, previous_menu);
    // P1 CPU control: the pilot takes it through an atomic (cpu_pilot::menu_settings_changed), no lock.
    crate::runtime::p1::publish(snapshot);
    crate::runtime::features::hitbox::publish(snapshot);
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
