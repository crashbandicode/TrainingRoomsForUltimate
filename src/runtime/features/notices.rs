//! HUD notifications, the runtime half (SPEC-input-05): the queue every feature posts to, the Combo
//! Display copy, and the stale-moves toggle. The queue itself is `model::notify`.
//!
//! Where the panes are written: the notice slot `TrModDisp0` lives in the plugin's own layout
//! (package 5, SPEC-core-05). Until that anchor exists, `hud_tick` is called once per CPU frame by the
//! recorder's hook (`interim tick`) and the draw goes to the diagnostics only; package 5 moves the call
//! to the layout's tick and fills `draw`. Nothing else changes.
//!
//! * `Combo Display` is copied at the game's own two load sites (`combo_display_a`, `combo_display_b`):
//!   the value only, no pointer is kept (SPEC-input-05 §6.1). It resets to "enabled" on session clear.
//! * The stale-moves toggle is set to 1 at the game's menu construction in Training.
//!
//! NOT YET SWITCH-RUN.

use std::sync::atomic::{AtomicU8, Ordering};

use parking_lot::Mutex;
use skyline::hooks::InlineCtx;

use crate::model::notify::{Draw, Queue, TickOut, DEFAULT_COLOR};
use crate::model::recording::{Phase, NOTICE_HEADER};
use crate::model::sites_pkg4;
use crate::runtime::alloc::HookScope;
use crate::runtime::{game, published};

static QUEUE: Mutex<Queue> = Mutex::new(Queue::new());

/// 0 / 1 as copied from the game, 2 = not seen (enabled).
static COMBO_DISPLAY: AtomicU8 = AtomicU8::new(2);

/// The game's Combo Display option; enabled unless the game said 0.
pub fn combo_display() -> bool {
    COMBO_DISPLAY.load(Ordering::Acquire) != 0
}

fn observe_combo(value: u64) {
    // Values other than 0 and 1 are ignored (SPEC-input-05 §4).
    if value <= 1 {
        COMBO_DISPLAY.store(value as u8, Ordering::Release);
    }
}

/// `combo_display_a`: w20 holds the byte the game just loaded.
#[skyline::hook(offset = sites_pkg4::COMBO_DISPLAY_A_OFFSET, inline)]
pub unsafe fn combo_display_a(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter();
    observe_combo(u64::from(ctx.registers[20].w() & 0xff));
}

/// `combo_display_b`: w8 holds the byte the game is about to store.
#[skyline::hook(offset = sites_pkg4::COMBO_DISPLAY_B_OFFSET, inline)]
pub unsafe fn combo_display_b(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter();
    observe_combo(u64::from(ctx.registers[8].w() & 0xff));
}

/// The stale-moves toggle: `PauseMenu+0xb60` ← 1 right after the game clears it (x22 is the menu).
#[skyline::hook(offset = sites_pkg4::STALE_MOVES_TOGGLE_OFFSET, inline)]
pub unsafe fn stale_moves_toggle(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter();
    let menu = ctx.registers[22].x() as usize;
    if menu != 0 && game::is_training_mode() {
        ((menu + 0xb60) as *mut u8).write(1);
    }
}

/// Post a notice with a static body.
pub fn post(header: &'static str, body: &'static str, length: u16, color: [u8; 4]) {
    QUEUE.lock().post_static(header, body, length, color);
}

/// A refusal text, shown for two seconds in the default colour.
pub fn refuse(text: &'static str) {
    post("Input Recording", text, 120, DEFAULT_COLOR);
}

/// The recorder changed phase: replace its message (once per phase change, SPEC-input-05 §6.2).
/// Playback's notice lasts `playback_frames`; the other phases one second each, re-posted by the
/// recorder only when the phase changes (the queue shows the last one drawn until the next).
pub fn post_phase(phase: Phase, playback_frames: u16) {
    let mut queue = QUEUE.lock();
    queue.clear_header(NOTICE_HEADER);
    if let Some((body, color)) = phase.notice() {
        let length = if phase == Phase::Playback { playback_frames } else { 60 };
        queue.post_static(NOTICE_HEADER, body, length, color);
    }
}

/// Session clear: leaving Training, a hot swap, a lobby change. Empties the queue and forgets the
/// Combo Display copy.
pub fn session_clear() {
    QUEUE.lock().clear();
    COMBO_DISPLAY.store(2, Ordering::Release);
}

/// One HUD tick. `menu_open` is the plugin menu's flag (package 5; the owner bit meanwhile).
pub fn hud_tick() -> TickOut {
    let menu_open = published::input_owners().contains(crate::model::input_delay::Owners::MENU);
    let out = QUEUE.lock().tick(combo_display(), menu_open);
    if let Some(draw) = out.draw {
        write_panes(&draw);
    }
    out
}

/// Write the notice's header, body and colour into `TrModDisp0Header` / `TrModDisp0Txt`. The panes are
/// package 5's layout (SPEC-core-05); until then the draw is logged by the diag build.
fn write_panes(draw: &Draw) {
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!(
        "[TrainingRooms] notice_draw header={:?} body={:?} color={:?}",
        draw.header,
        draw.body.as_str(),
        draw.color
    ));
    #[cfg(not(feature = "diag"))]
    let _ = draw;
}
