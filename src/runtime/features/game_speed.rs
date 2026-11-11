//! The game speed bind (rc.8, `docs/design/rc8-binds.md`), the Switch half. The rules are `model::game_speed`; every
//! address used here has a seam row (`native_seams::rows_game_speed`, group `game_speed`), compared once at boot. No hook
//! of its own:
//! - P1's poll (`chords::on_poll`, a game thread) runs the hold: it opens the pop-up at the current speed, steps it and
//!   closes it. A Training step is handed to the UI tick; a lobby step sets the lobby's wanted speed. Atomics only.
//! - Training: `hud_draw`'s `info_training` tick (the UI job thread) stores `PauseMenu+0xb54` and calls
//!   `0x1bbbd30(menu, value, menu+0x538)`, the menu setter's own path for an unchanged value. The game applies the menu's
//!   speed only on the pause/resume side, so P1's next poll (a game thread) then does the apply's own writes to the
//!   Training mode object, the slow table and the rate words (`training_apply`, QCF's Q4).
//! - The waiting-room practice: Player 1's frame step (`lobby::frame`) keeps one slot of the game's slow table (1/2,
//!   1/4, and 2/3's paced slot). The practice's teardown (the session clear on its P1's destroy, its exit, a scene
//!   change) frees it with a write, only in the table it was taken from and only while the slot is still ours; a match
//!   found shows no busy edge first.
//! - rc.8 R2 (`model::lobby_speed`): the waiting area's update (`lobby::lobby_speed`'s hook, after the update's slow step
//!   and scheduler pass) paces 2/3's slot and runs x1.5's extra world step. Both run only while the practice is live
//!   (Player 1's frame arms it, the same teardown disarms it before the native teardown), and neither writes the update
//!   rate words: those are Training's alone.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use crate::engine::seams::native_fields as field;
use crate::model::game_speed::{self as rules, mode, native, table, Bytes, HoldEvent, LobbyNow, LobbySlot, ModeApply, Place, Speed};
use crate::model::lobby_speed::{self as lobby_rules, Cadence, LobbyPlan, PassNow};
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
/// The Training value the UI tick wrote into the menu, 0 when none: P1's poll applies it (`training_apply`).
static MODE_PENDING: AtomicU32 = AtomicU32::new(0);
/// The speed the open pop-up shows, 0 while closed: the next step goes from it.
static SHOWN: AtomicU32 = AtomicU32::new(0);
/// The waiting-room practice's speed; x1 at each new practice session.
static LOBBY_WANTED: AtomicU32 = AtomicU32::new(Speed::X1 as u32);
/// The lobby's slot: Player 1's frame and the waiting area's update (game threads), `try_lock`.
static LOBBY: hos_lock::Mutex<LobbySlot> = hos_lock::Mutex::new(LobbySlot::new());
/// The practice is live: Player 1's frame in the practice arms it, `session_clear` (the practice's teardown) disarms it.
/// The waiting area's update paces and steps only while it is set.
static LOBBY_LIVE: AtomicBool = AtomicBool::new(false);
/// x1.5's every-other-update cadence: the waiting area's update only (one game thread), `try_lock`.
static CADENCE: hos_lock::Mutex<Cadence> = hos_lock::Mutex::new(Cadence::new());

/// The speed bind's rows all passed at boot (`lobby_speed` hooks only then).
pub(crate) fn installed() -> bool {
    base().is_some()
}

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
        let shown = SHOWN.swap(0, Ordering::AcqRel);
        if shown != 0 {
            notices::clear(rules::POPUP_ID);
        }
        #[cfg(feature = "diag")]
        if let Some(speed) = Speed::from_value(shown) {
            crate::runtime::diag_log::diagnostic(format_args!("[GameSpeed] popup close speed={}", speed.label()));
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
    MODE_PENDING.store(write.value, Ordering::Release);
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[GameSpeed] training write value={} was={current}", write.value));
}

/// P1's poll (`chords::on_poll`, a game thread: the one the game recomputes the slow table on), after the UI tick wrote
/// a value into the menu: the game's speed code for that change, done here because the game runs it only on the
/// pause/resume side. With the game's pause menu open the menu's own apply runs instead; the value waits.
pub(crate) fn training_apply() {
    let wanted = MODE_PENDING.load(Ordering::Acquire);
    if wanted == 0 {
        return;
    }
    let Some(base) = base() else { return };
    if !scene::is_native_training() {
        let _ = MODE_PENDING.compare_exchange(wanted, 0, Ordering::AcqRel, Ordering::Acquire);
        return;
    }
    if published::input_owners().contains(Owners::VANILLA_PAUSE) {
        return;
    }
    if MODE_PENDING.compare_exchange(wanted, 0, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return;
    }
    // SAFETY: native Training on P1's poll, a game thread; every address is formed from a row that passed at boot.
    let outcome = unsafe { apply_now(base, wanted) };
    // A skip that may clear by itself (the battle's objects not yet there, state 11) keeps the value for the next poll,
    // unless a newer one came meanwhile (worker B's R8 finding: a skipped apply waited for the next pause). The log says
    // each skip once.
    let repeat = match outcome {
        Ok(_) => {
            LAST_SKIP.store(0, Ordering::Relaxed);
            false
        }
        Err(why) => {
            if RETRIED.contains(&why) {
                let _ = MODE_PENDING.compare_exchange(0, wanted, Ordering::AcqRel, Ordering::Acquire);
            }
            let key = wanted << 8 | RETRIED.iter().position(|r| *r == why).map_or(0xff, |i| i as u32);
            LAST_SKIP.swap(key, Ordering::Relaxed) == key
        }
    };
    #[cfg(feature = "diag")]
    match outcome {
        Err(_) if repeat => {}
        Ok((applied, slot, done)) => crate::runtime::diag_log::diagnostic(format_args!(
            "[GameSpeed] training apply value={} was={applied} slot={slot:#x}->{:#x} freed={:?} code={} updates={} thread={:#x}",
            done.value,
            done.slot,
            done.freed,
            done.code,
            done.updates,
            // SAFETY: reads the calling thread's own object pointer; nothing is written.
            unsafe { skyline::nn::os::GetCurrentThread() } as usize
        )),
        Err(why) => crate::runtime::diag_log::diagnostic(format_args!(
            "[GameSpeed] training apply skipped value={wanted} why={why} retry={}",
            RETRIED.contains(&why)
        )),
    }
    #[cfg(not(feature = "diag"))]
    let _ = (outcome, repeat);
}

/// The skips `training_apply` retries on the next poll: the battle's objects or the slow table not there yet, the state
/// the game itself applies 0 in. The others end the value: it is no longer the menu's, or it is applied already.
const RETRIED: [&str; 4] = ["no menu", "state 11", "no mode object", "no slow table"];
/// The last skip logged, `value << 8 | reason`, 0 after an apply: each skip is logged once.
static LAST_SKIP: AtomicU32 = AtomicU32::new(0);

/// The Training mode object (`FUN_0155a000`'s x25), found the way the game finds it on every update
/// (`rules::registry_find` over `[[main+0x52c3910]]`, key `0x1022590708`), and only when its vtable is Training's.
///
/// # Safety
/// Native Training on a game thread: the registry is the battle's own.
unsafe fn mode_object(base: usize) -> Option<usize> {
    let map = field::child(base, native::REGISTRY).and_then(|holder| field::child(holder, 0))?;
    // SAFETY: a word of the battle's registry, bucket array or one of its nodes, as `FUN_015435c0` reads it.
    let read = |at: usize| if field::object(at) { unsafe { field::read::<u64>(at, 0) } } else { None };
    let object = rules::registry_find(map, mode::KEY, &read)? as usize;
    (field::object(object) && field::read::<usize>(object, 0) == Some(base + native::MODE_VTABLE)).then_some(object)
}

/// The state in which the game's speed code reads 0 instead of the menu (`[[main+0x532f8b0]+8]`: `+0x138` set and byte
/// `+0x2b8 == 11`).
///
/// # Safety
/// Native Training on a game thread.
unsafe fn state_off(base: usize) -> bool {
    let Some(inner) = field::child(base, native::STATE).and_then(|state| field::child(state, 8)) else { return false };
    field::read::<u32>(inner, mode::STATE_SET).is_some_and(|set| set != 0)
        && field::read::<u8>(inner, mode::STATE_KIND) == Some(mode::STATE_OFF)
}

/// The apply's step clears (`.155b5f0`–`.155b62c`): bit 2 of `main+0x52b92ec` once its static guard is set, and for a
/// value other than 1 Frame the byte at `[[main+0x53398d8]+8]`.
///
/// # Safety
/// Native Training on a game thread.
unsafe fn step_clear(base: usize, value: u32) {
    if field::read::<u8>(base, native::STEP_GUARD).is_some_and(|guard| guard & 1 != 0) {
        let bits = field::read_unchecked::<u32>(base, native::STEP_BITS);
        field::write_unchecked::<u32>(base, native::STEP_BITS, bits & !mode::STEP_BIT);
    }
    if value != Speed::OneFrame.value() {
        // A byte pointer: any alignment.
        let flag = field::child(base, native::STEP_FLAG).and_then(|holder| field::read::<usize>(holder, 8));
        if let Some(flag) = flag.filter(|at| *at != 0 && *at < field::USER_ADDRESS_END) {
            field::write_unchecked::<u8>(flag, 0, 0);
        }
    }
}

/// The writes of the game's speed code for `wanted` (`rules::mode_apply`), in its order: the slot bytes, the applied
/// value and the slot index, the step clears, the rate code and update rate; the table's dirty byte last. Returns the
/// value applied before and the slot it held.
///
/// # Safety
/// Native Training on a game thread (P1's poll), the game's pause menu closed.
unsafe fn apply_now(base: usize, wanted: u32) -> Result<(u32, u8, ModeApply), &'static str> {
    let speed = Speed::from_value(wanted).ok_or("value")?;
    let menu = pause_menu(base).ok_or("no menu")?;
    if field::read::<u32>(menu, rules::SPEED_FIELD) != Some(wanted) {
        return Err("menu changed");
    }
    if state_off(base) {
        return Err("state 11");
    }
    let object = mode_object(base).ok_or("no mode object")?;
    let slow_at = slow_table(base);
    if !field::object(slow_at) {
        return Err("no slow table");
    }
    let applied = field::read_unchecked::<u32>(object, mode::APPLIED);
    let slot = field::read_unchecked::<u8>(object, mode::SLOT);
    let mut slow = NativeTable(slow_at);
    let done = rules::mode_apply(speed, applied, slot, &mut slow).ok_or("applied already")?;
    field::write_unchecked::<u32>(object, mode::APPLIED, done.value);
    field::write_unchecked::<u8>(object, mode::SLOT, done.slot);
    step_clear(base, done.value);
    field::write_unchecked::<u32>(base, native::RATE_CODE, done.code);
    field::write_unchecked::<u32>(base, native::UPDATE_RATE, done.updates);
    if done.dirty {
        slow.set(table::DIRTY_AT, 1);
    }
    Ok((applied, slot, done))
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

/// The slow table's global now, 0 without one.
fn slow_table(base: usize) -> usize {
    // SAFETY: a `.bss` word of main (`game_speed.rate_request` reads it there).
    unsafe { field::read::<usize>(base, native::SLOW_TABLE) }.unwrap_or(0)
}

/// A teardown release that found `LOBBY` taken (`release_lobby_slot`): the next `lobby_frame` makes it.
static RELEASE_PENDING: AtomicBool = AtomicBool::new(false);

/// Player 1's frame, in every scene (`lobby::frame`): the waiting-room practice's slot. Outside the practice it writes
/// nothing: a slot record left there is dropped (design §3.8; worker B's R8 finding: a match's table can reuse the
/// practice table's block and hold the game's own `{1, N}` at the same slot).
pub(crate) fn lobby_frame() {
    let Some(base) = base() else { return };
    let Some(mut keeper) = LOBBY.try_lock() else { return };
    let in_practice = scene::lobby_term();
    let pending = RELEASE_PENDING.swap(false, Ordering::AcqRel);
    if in_practice {
        LOBBY_LIVE.store(true, Ordering::Release);
    }
    if !in_practice && keeper.held().is_none() {
        return;
    }
    let table = if in_practice { slow_table(base) } else { 0 };
    let mut native = NativeTable(table);
    let bytes: Option<&mut dyn Bytes> = if in_practice && field::object(table) { Some(&mut native) } else { None };
    let wanted = Speed::from_value(LOBBY_WANTED.load(Ordering::Acquire)).unwrap_or(Speed::X1);
    let step = if pending {
        keeper.release(table, bytes)
    } else {
        let now = LobbyNow { in_practice, ui_busy: crate::runtime::lobby::training::lobby::native_ui_busy(), table };
        keeper.step(wanted, now, bytes)
    };
    drop(keeper);
    #[cfg(feature = "diag")]
    if step != rules::LobbyStep::Nothing {
        crate::runtime::diag_log::diagnostic(format_args!(
            "[GameSpeed] lobby {step:?} wanted={} in_practice={in_practice} pending={pending}",
            wanted.label()
        ));
    }
    #[cfg(not(feature = "diag"))]
    let _ = step;
}

/// The slow state's byte at `at` (`model::lobby_speed::native`).
///
/// # Safety
/// `base` is main's base with `lobby_speed.slow_step` guarded: `.bss` bytes of main the slow step reads every update.
unsafe fn slow_byte(base: usize, at: usize) -> u8 {
    field::read_unchecked::<u8>(base + lobby_rules::native::SLOW_STATE, at)
}

/// The waiting area's update (`lobby::lobby_speed`'s hook: the main thread, after this update's slow step and
/// scheduler pass): 2/3's pace and x1.5's extra world step, only while the practice is live.
///
/// # Safety
/// The hook's thread and moment; every `lobby_speed` row passed (the hook is installed only then).
pub(crate) unsafe fn lobby_update() {
    let Some(base) = base() else { return };
    // Only in the practice, as every lobby write (`lobby_frame`; worker B's R8 finding).
    if !LOBBY_LIVE.load(Ordering::Acquire) || !scene::lobby_term() {
        return;
    }
    let wanted = Speed::from_value(LOBBY_WANTED.load(Ordering::Acquire)).unwrap_or(Speed::X1);
    match LobbyPlan::of(wanted) {
        LobbyPlan::TwoThirds => lobby_pace(base),
        LobbyPlan::ExtraPass => lobby_extra_step(base),
        LobbyPlan::X1 | LobbyPlan::Slot(_) => {}
    }
}

/// 2/3: at a slow period's end, the paced slot's next divisor (`LobbySlot::pace`).
///
/// # Safety
/// As `lobby_update`.
unsafe fn lobby_pace(base: usize) {
    let Some(mut keeper) = LOBBY.try_lock() else { return };
    let table = slow_table(base);
    let (current_n, sub) = (slow_byte(base, lobby_rules::native::CURRENT_N_AT), slow_byte(base, lobby_rules::native::SUB_AT));
    let mut native = NativeTable(table);
    let bytes: Option<&mut dyn Bytes> = if field::object(table) { Some(&mut native) } else { None };
    let step = keeper.pace(current_n, sub, table, bytes);
    drop(keeper);
    #[cfg(feature = "diag")]
    {
        static PACED: AtomicU32 = AtomicU32::new(0);
        if let rules::LobbyStep::Paced(held) = step {
            let n = PACED.fetch_add(1, Ordering::Relaxed);
            if n < 4 || n.is_multiple_of(600) {
                crate::runtime::diag_log::diagnostic(format_args!(
                    "[GameSpeed] lobby paced n={n} divisor={} slot={} current_n={current_n}",
                    held.divisor, held.slot
                ));
            }
        }
    }
    #[cfg(not(feature = "diag"))]
    let _ = step;
}

/// x1.5: on every other allowed update, one extra world step (`lobby::lobby_speed::extra_pass`). No lock is held while
/// the world's tasks run (Player 1's frame inside them takes `LOBBY`).
///
/// # Safety
/// As `lobby_update`.
unsafe fn lobby_extra_step(base: usize) {
    use crate::runtime::lobby::lobby_speed as pass;
    let s = pass::scheduler(base);
    let now = PassNow {
        live: LOBBY_LIVE.load(Ordering::Acquire),
        in_practice: scene::lobby_term(),
        ui_busy: crate::runtime::lobby::training::lobby::native_ui_busy(),
        paused: field::read_unchecked::<u32>(base, lobby_rules::native::PAUSED),
        current_n: slow_byte(base, lobby_rules::native::CURRENT_N_AT),
        target_n: slow_byte(base, lobby_rules::native::TARGET_N_AT),
        phase_word: pass::phase_word(s),
    };
    let Some(mut cadence) = CADENCE.try_lock() else { return };
    if s == 0 || !lobby_rules::pass_allowed(now) {
        cadence.reset();
        return;
    }
    let due = cadence.due();
    drop(cadence);
    if !due {
        return;
    }
    let plan = pass::extra_pass(base, s);
    #[cfg(feature = "diag")]
    {
        let n = pass::passes();
        static REFUSED: AtomicU32 = AtomicU32::new(0);
        if !plan.runnable() {
            if REFUSED.fetch_add(1, Ordering::Relaxed) < 4 {
                crate::runtime::diag_log::diagnostic(format_args!("[GameSpeed] lobby pass refused {plan:?}"));
            }
        } else if n <= 2 || n.is_multiple_of(600) {
            crate::runtime::diag_log::diagnostic(format_args!(
                "[GameSpeed] lobby pass n={n} tasks={} skipped={} masked={}",
                plan.tasks, plan.skipped, plan.masked
            ));
        }
    }
    #[cfg(not(feature = "diag"))]
    let _ = plan;
}

/// The practice's teardown (`session_clear`: the practice P1's destroy, the scene's exit, a scene change, the
/// waiting-room UI turning busy): the lobby slot is freed with a write while the table it was taken from is still the
/// game's, before the native teardown can hand that table to the next battle. Wait-free: with `LOBBY` taken, the next
/// `lobby_frame` releases it (worker B's R8 finding on the blocking lock).
fn release_lobby_slot() {
    let Some(base) = base() else { return };
    let Some(mut keeper) = LOBBY.try_lock() else {
        RELEASE_PENDING.store(true, Ordering::Release);
        return;
    };
    if keeper.held().is_none() {
        return;
    }
    // The practice P1's destroy comes while the mode words are still the waiting room (Eden rc8b-qp2: the release
    // 2.3 s before the match scene); a clear after the scene left the practice writes nothing, as `lobby_frame`.
    let in_practice = scene::lobby_term();
    let table = if in_practice { slow_table(base) } else { 0 };
    let mut native = NativeTable(table);
    let bytes: Option<&mut dyn Bytes> = if in_practice && field::object(table) { Some(&mut native) } else { None };
    let step = keeper.release(table, bytes);
    drop(keeper);
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!(
        "[GameSpeed] lobby release {step:?} table={table:#x} in_practice={in_practice} thread={:#x}",
        // SAFETY: reads the calling thread's own object pointer; nothing is written.
        unsafe { skyline::nn::os::GetCurrentThread() } as usize
    ));
    #[cfg(not(feature = "diag"))]
    let _ = step;
}

/// The lobby practice started or ended (`lobby::update`): a new session starts at x1, and the pop-up goes.
pub(crate) fn practice_changed() {
    LOBBY_WANTED.store(Speed::X1 as u32, Ordering::Release);
    session_clear();
}

/// Session clear (`chords::session_clear`): the pop-up goes; a Training write not yet made is dropped; the practice is
/// no longer live (the waiting area's update stops pacing and stepping before the native teardown), and the lobby slot
/// is released (`release_lobby_slot`).
pub(crate) fn session_clear() {
    if SHOWN.swap(0, Ordering::AcqRel) != 0 {
        notices::clear(rules::POPUP_ID);
    }
    TRAINING_WANTED.store(0, Ordering::Release);
    MODE_PENDING.store(0, Ordering::Release);
    LOBBY_LIVE.store(false, Ordering::Release);
    if let Some(mut cadence) = CADENCE.try_lock() {
        cadence.reset();
    }
    release_lobby_slot();
}
