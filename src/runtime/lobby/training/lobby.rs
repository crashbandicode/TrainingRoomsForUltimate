//! Waiting-room practice support. Never change the game's own mode predicate.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use {crate::runtime::compat::consts::FighterId, crate::engine::compat::menu::OnOff, crate::runtime::compat::menu::MENU};
use crate::runtime::compat::sync::read;

use crate::runtime::lobby::game_mode;

macro_rules! trace {
    ($($arg:tt)*) => { crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}

static NATIVE_UI_BUSY: AtomicBool = AtomicBool::new(false);
static UI_READY: AtomicBool = AtomicBool::new(false);
static WAS_ACTIVE: AtomicBool = AtomicBool::new(false);
// Setting-plus-tuple edge for the hazard receipt; UI readiness is not part of it.
static SEEN_ENTRIES: AtomicU32 = AtomicU32::new(0);
static LAST_MODE: AtomicU64 = AtomicU64::new(u64::MAX);

pub fn native_ui_busy() -> bool {
    NATIVE_UI_BUSY.load(Ordering::Acquire)
}

/// Native pause/dialog ownership takes priority over an open editor or delayed input.
pub fn set_native_ui_busy(busy: bool) {
    // R2: the waiting-room native UI busy fact is one producer: the owner bit that the input-delay stage reads
    crate::runtime::compat::common::menu::set_native_ui_owner(busy);
    if NATIVE_UI_BUSY.swap(busy, Ordering::AcqRel) != busy {
        if busy {
            crate::runtime::compat::common::menu::clear_session_state();
            crate::runtime::compat::common::button_config::clear_session_state();
            super::input_delay::clear_session_state();
        }
        trace!("[LobbyPractice] native_ui_busy={busy}");
    }
}

pub fn init() {
    trace!("[LobbyPractice] verified_13_0_5={}", game_mode::init());
}

pub fn is_waiting_scene() -> bool {
    // Shared PracticeScene setup (13.0.5, 0x1790020) explicitly selects melee 0x0d.
    // The normal 0x0701xxxx match mapper never returns that value. Keep the proven
    // local family exact; online activation also needs the actual waiting UI graft.
    // LDN is runtime-tested; the online family is supported by static call paths.
    matches!(game_mode::current(), Some(mode)
        if mode.initialized && mode.melee == 0x0d
            && (mode.mode == 0x02010000 || mode.mode & 0x7fff0000 == 0x07010000))
}

pub fn requested() -> bool {
    read(&MENU).lobby_practice == OnOff::ON && is_waiting_scene()
}

pub fn active() -> bool {
    requested() && UI_READY.load(Ordering::Acquire)
}

pub fn set_ui_ready(ready: bool) {
    UI_READY.store(ready, Ordering::Release);
}

pub fn entry_id(role: FighterId) -> i32 {
    if role == FighterId::CPU && is_waiting_scene() {
        4
    } else {
        role as i32
    }
}

/// Called before input processing and UI dispatch. Cleanup only touches plugin state;
/// do not roll Smash RNG, dereference departing fighters or request a game status here.
pub fn update() {
    if let Some(mode) = game_mode::current() {
        let key = u64::from(mode.mode)
            | (u64::from(mode.melee) << 32)
            | (u64::from(mode.initialized) << 40);
        if LAST_MODE.swap(key, Ordering::Relaxed) != key {
            // A native Training departure need not change lobby::active(). Drop
            // preview values and pending injection at the actual scene boundary.
            if WAS_ACTIVE.load(Ordering::Acquire) { super::named_macros::suspend_waiting(); }
            crate::runtime::roster::character_roster::scene_changed(mode);
            super::practice::scene_departed();
            #[cfg(feature = "practice-stage-request-alpha")]
            crate::runtime::lobby::practice_stage_request::depart();
            // A scene switch invalidates the previous layout even if the next
            // waiting room happens to use the same allocation address.
            UI_READY.store(false, Ordering::Release);
            set_native_ui_busy(false);
            super::ui::native_menu::reset();
            trace!(
                "[LobbyPractice] scene initialized={} melee=0x{:02x} mode=0x{:08x}",
                mode.initialized,
                mode.melee,
                mode.mode
            );
        }
    }
    let requested_now = requested();
    if !requested_now {
        UI_READY.store(false, Ordering::Release);
    }
    let now = active();
    let previous = WAS_ACTIVE.swap(now, Ordering::AcqRel);
    if now != previous {
        SEEN_ENTRIES.store(0, Ordering::Relaxed);
        crate::runtime::features::cpu::set_lobby(now);
        super::clear_session_state();
        // The CPU's cached rolls start from the live menu, as after a Training reset.
        super::cpu_rolls::request_all();
        trace!(
            "[LobbyPractice] active={} mode={:?}",
            now,
            game_mode::current()
        );
    }
    // The practice entry lease is refreshed once per game frame from the P1 body's
    // frame callback, not from every caller of this update.
}

pub unsafe fn observe_fighter(accessor: &mut smash::app::BattleObjectModuleAccessor) {
    if !active() || !crate::runtime::compat::common::is_fighter(accessor) {
        return;
    }
    let entry = smash::app::lua_bind::WorkModule::get_int(
        accessor,
        *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    );
    if !(0..8).contains(&entry) {
        return;
    }
    let bit = 1 << entry;
    if SEEN_ENTRIES.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
        #[cfg(feature = "lobby-stage")]
        crate::runtime::lobby::lobby_stage_probe::observe_loaded();
        trace!(
            "[LobbyPractice] fighter entry={} cpu={} ready_go={}",
            entry,
            crate::runtime::compat::common::is_operation_cpu(accessor),
            crate::runtime::compat::common::is_ready_go()
        );
    }
}
