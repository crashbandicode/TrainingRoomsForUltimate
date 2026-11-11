//! The game speed bind (rc.8, `docs/design/rc8-binds.md`), the Switch half. The rules are `model::game_speed`; every
//! address used here has a seam row (`native_seams::rows_game_speed`, group `game_speed`), compared once at boot. No hook
//! of its own:
//! - P1's poll (`chords::on_poll`, a game thread) runs the hold: it opens the pop-up at the current speed, steps it and
//!   closes it. A Training step is handed to the UI tick; a lobby step sets the lobby's wanted speed. Atomics only.
//! - Training: `hud_draw`'s `info_training` tick (the UI job thread) stores `PauseMenu+0xb54` and calls
//!   `0x1bbbd30(menu, value, menu+0x538)`, the menu setter's own path for an unchanged value. The game's per-update apply
//!   does the rest; the rate globals and the slow table are never written in Training.
//! - The waiting-room practice: Player 1's frame step (`lobby::frame`) keeps one slot of the game's slow table, and
//!   writes nothing once the scene has left the practice.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::engine::seams::native_fields as field;
use crate::model::game_speed::{self as rules, native, Bytes, HoldEvent, LobbyNow, LobbySlot, Place, Speed};
use crate::model::input_delay::Owners;
use crate::model::notify::DEFAULT_COLOR;
use crate::runtime::features::notices;
use crate::runtime::{game, published, scene};

/// `main`'s base once every row passed the authoritative gate at boot, else 0: the only base this module forms an
/// address with.
static BASE: AtomicUsize = AtomicUsize::new(0);

fn base() -> Option<usize> {
    Some(BASE.load(Ordering::Acquire)).filter(|base| *base != 0)
}

/// The Training value the bind asked for, 0 when none is pending (the UI tick takes it).
static TRAINING_WANTED: AtomicU32 = AtomicU32::new(0);
/// The speed the open pop-up shows, 0 while closed: the next step goes from it.
static SHOWN: AtomicU32 = AtomicU32::new(0);
/// The waiting-room practice's speed; x1 at each new practice session.
static LOBBY_WANTED: AtomicU32 = AtomicU32::new(Speed::X1 as u32);
/// The lobby's slot: Player 1's frame only (a game thread), `try_lock`.
static LOBBY: hos_lock::Mutex<LobbySlot> = hos_lock::Mutex::new(LobbySlot::new());

/// Once from `lobby::init`: compare every row; the bind does nothing unless all passed.
pub(crate) fn init() {
    let rows = crate::runtime::seams::seam_install::gate("game_speed", &rules::SEAM_ROWS, false);
    if rows {
        BASE.store(game::text_base(), Ordering::Release);
    }
    crate::engine::compat::logging::diagnostic(format_args!("[TrainingRooms] game_speed installed={rows}"));
}

/// Training's pause menu, or `None` while its chain is not readable.
///
/// # Safety
/// Native Training on a game thread: the chain is the one the game's own apply and HUD read there.
unsafe fn pause_menu(base: usize) -> Option<usize> {
    let root = field::read::<usize>(base, native::TRAINING_ROOT)?;
    field::chain(root, &native::PANEL_CHAIN)
}

fn place() -> Option<Place> {
    if scene::is_native_training() {
        Some(Place::Training)
    } else if scene::lobby_term() {
        Some(Place::Lobby)
    } else {
        None
    }
}

fn show(speed: Speed, style: u32) {
    notices::show_now(rules::POPUP_ID, rules::POPUP_HEADER, rules::popup_body(speed, style).as_str(), rules::POPUP_FRAMES, DEFAULT_COLOR);
}

/// What P1's poll did to the hold (`chords::on_poll`, after the detector let go of its lock). `style` is P1's controller
/// style (the pop-up's glyphs). Atomics and the notice mailbox only.
pub(crate) fn on_hold(event: HoldEvent, style: u32) {
    if event == HoldEvent::Idle || event == HoldEvent::Held {
        return;
    }
    if event == HoldEvent::Closed {
        if SHOWN.swap(0, Ordering::AcqRel) != 0 {
            notices::clear(rules::POPUP_ID);
        }
        return;
    }
    let (Some(base), Some(place)) = (base(), place()) else { return };
    match event {
        HoldEvent::Opened => {
            let current = match place {
                // SAFETY: native Training on the game's input thread (`place`).
                Place::Training => unsafe { pause_menu(base) }
                    .and_then(|menu| unsafe { field::read::<u32>(menu, rules::SPEED_FIELD) })
                    .and_then(Speed::from_value),
                Place::Lobby => Speed::from_value(LOBBY_WANTED.load(Ordering::Acquire)),
            }
            .unwrap_or(Speed::X1);
            SHOWN.store(current.value(), Ordering::Release);
            show(current, style);
            #[cfg(feature = "diag")]
            crate::runtime::diag_log::diagnostic(format_args!("[GameSpeed] popup open place={place:?} speed={}", current.label()));
        }
        HoldEvent::Step(dir) => {
            let Some(current) = Speed::from_value(SHOWN.load(Ordering::Acquire)) else { return };
            let next = rules::step(place.speeds(), current, dir);
            SHOWN.store(next.value(), Ordering::Release);
            match place {
                Place::Training => TRAINING_WANTED.store(next.value(), Ordering::Release),
                Place::Lobby => LOBBY_WANTED.store(next.value(), Ordering::Release),
            }
            show(next, style);
            #[cfg(feature = "diag")]
            crate::runtime::diag_log::diagnostic(format_args!("[GameSpeed] step place={place:?} dir={dir:?} speed={}", next.label()));
        }
        HoldEvent::Idle | HoldEvent::Held | HoldEvent::Closed => {}
    }
}

/// `info_training`'s tick (`hud_draw::draw`, the UI job thread): the wanted Training speed is written the native way.
///
/// # Safety
/// The UI job thread inside the `info_training` tick.
pub(crate) unsafe fn training_tick() {
    let wanted = TRAINING_WANTED.load(Ordering::Acquire);
    if wanted == 0 {
        return;
    }
    let Some(base) = base() else { return };
    if !scene::is_native_training() {
        let _ = TRAINING_WANTED.compare_exchange(wanted, 0, Ordering::AcqRel, Ordering::Acquire);
        return;
    }
    let Some(menu) = pause_menu(base) else { return };
    let current = field::read_unchecked::<u32>(menu, rules::SPEED_FIELD);
    let pause_open = published::input_owners().contains(Owners::VANILLA_PAUSE);
    let Some(write) = rules::training_write(wanted, current, pause_open) else {
        // The game's menu is open (it owns the pane: the next tick after it closes), or there is nothing to write.
        if !pause_open {
            let _ = TRAINING_WANTED.compare_exchange(wanted, 0, Ordering::AcqRel, Ordering::Acquire);
        }
        return;
    };
    // A newer step since the load: the next tick writes that one.
    if TRAINING_WANTED.compare_exchange(wanted, 0, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return;
    }
    field::write_unchecked::<u32>(menu, write.field, write.value);
    // SAFETY: `game_speed.speed_text` passed its guard: `0x1bbbd30(menu, value, pane)`, as the setter calls it.
    let text: unsafe extern "C" fn(usize, u32, usize) = std::mem::transmute(base + native::SPEED_TEXT_FN);
    text(menu, write.value, menu + write.text_pane);
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[GameSpeed] training write value={} was={current}", write.value));
}

/// The slow table in game memory (`*(main+0x52c3610)`), byte by byte.
struct NativeTable(usize);

impl Bytes for NativeTable {
    fn get(&self, at: usize) -> u8 {
        // SAFETY: `at` < `table::SIZE` inside the live table the global holds (read on this frame).
        unsafe { ((self.0 + at) as *const u8).read_volatile() }
    }
    fn set(&mut self, at: usize, value: u8) {
        // SAFETY: as `get`; the game's own apply writes these bytes on the same table.
        unsafe { ((self.0 + at) as *mut u8).write_volatile(value) }
    }
}

/// Player 1's frame, in every scene (`lobby::frame`): the waiting-room practice's slot. Outside the practice it writes
/// nothing; a slot record left from it is dropped.
pub(crate) fn lobby_frame() {
    let Some(base) = base() else { return };
    let Some(mut keeper) = LOBBY.try_lock() else { return };
    let in_practice = scene::lobby_term();
    if !in_practice && keeper.held().is_none() {
        return;
    }
    // SAFETY: a `.bss` word of main (`game_speed.rate_request` reads it there).
    let table = if in_practice { unsafe { field::read::<usize>(base, native::SLOW_TABLE) }.unwrap_or(0) } else { 0 };
    let now = LobbyNow { in_practice, ui_busy: crate::runtime::lobby::training::lobby::native_ui_busy(), table };
    let wanted = Speed::from_value(LOBBY_WANTED.load(Ordering::Acquire)).unwrap_or(Speed::X1);
    let mut native = NativeTable(table);
    let bytes: Option<&mut dyn Bytes> = if in_practice && field::object(table) { Some(&mut native) } else { None };
    let step = keeper.step(wanted, now, bytes);
    drop(keeper);
    #[cfg(feature = "diag")]
    if step != rules::LobbyStep::Nothing {
        crate::runtime::diag_log::diagnostic(format_args!("[GameSpeed] lobby {step:?} wanted={}", wanted.label()));
    }
    #[cfg(not(feature = "diag"))]
    let _ = step;
}

/// The lobby practice started or ended (`lobby::update`): a new session starts at x1, and the pop-up goes.
pub(crate) fn practice_changed() {
    LOBBY_WANTED.store(Speed::X1 as u32, Ordering::Release);
    session_clear();
}

/// Session clear (`chords::session_clear`): the pop-up goes; a Training write not yet made is dropped.
pub(crate) fn session_clear() {
    if SHOWN.swap(0, Ordering::AcqRel) != 0 {
        notices::clear(rules::POPUP_ID);
    }
    TRAINING_WANTED.store(0, Ordering::Release);
}
