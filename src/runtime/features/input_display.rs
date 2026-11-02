//! The input display, the runtime half (SPEC-input-02): the logger on the P1 poll and the frame
//! counters on the primary CPU frame are live; the render plan is computed every HUD tick.
//!
//! WAITS FOR PACKAGE 5: the panes (`TrModInputLog`, `TrModInputLog0..14` and their children, SPEC-core-05)
//! belong to the plugin's layout archive and the Revision 29 UI anchors, which package 5 builds.
//! `write_plan` is the one place that applies a plan; until the layout exists it only reports the plan
//! to the diagnostics (diag build), and package 5 fills it with the pane writes by name (`model::
//! input_display::panes`). No pane pointer is retained between ticks: the plan names panes, the writer
//! finds them.
//!
//! Poll order (SPEC-input-02 §5.1): after the input delay, before the recorder, so the display shows
//! what P1 pressed even while the recorder neutralises P1's output.
//!
//! NOT YET SWITCH-RUN.

use std::sync::atomic::{AtomicBool, Ordering};

use hos_lock::Mutex;

use crate::model::chords::field;
use crate::model::input_delay::{MappedInputs, Owners};
use crate::model::input_display::{raw_axis, Candidate, Log, Mode, Plan, RawInput, Renderer, Settings};
use crate::runtime::features::recorder;
use crate::runtime::published;

/// The display's natives (the P1 body's status, found through the fighter manager) passed their guards (`model::capabilities`,
/// review R2-1): until boot says so, the poll hook's call into the display does nothing and reads no native, so Input Delay installs on
/// its own rows and the display degrades by itself.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Boot: the input display's closure passed its guards.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Release);
}

static LOG: Mutex<Log> = Mutex::new(Log::new());
static RENDERER: Mutex<Renderer> = Mutex::new(Renderer::new());

/// `Controller` stick offsets (SPEC-input-04 §3.1).
const LEFT_STICK_X: usize = 0x10;
const LEFT_STICK_Y: usize = 0x14;
const RIGHT_STICK_X: usize = 0x20;
const RIGHT_STICK_Y: usize = 0x24;

/// The two settings, read wait-free from `published::Live` (stream/sync); the factory values before the first publish.
fn settings() -> Settings {
    crate::runtime::published::live().map_or(Settings::factory(), |live| live.input_display)
}

/// One P1 poll after the delay: log the candidate (skipped while the plugin menu owns the input, when
/// the mode is None, or when P1's body is missing).
///
/// # Safety
/// `controller` is the live `Controller` of player 0 (or 0); `out` is the delayed mapped record.
pub unsafe fn on_p1_poll(controller: usize, out: *const MappedInputs) {
    if !ENABLED.load(Ordering::Acquire) {
        return;
    }
    let settings = settings();
    if settings.mode == Mode::None || controller == 0 || published::input_owners().contains(Owners::MENU) {
        return;
    }
    let Some(body) = recorder::body_of(0) else { return };
    let status = crate::runtime::game::natives::status_kind(body.accessor);
    let fighter_kind = ((body.fighter + recorder::FIGHTER_KIND_FIELD) as *const i32).read();
    let f32_at = |offset: usize| ((controller + offset) as *const f32).read_volatile();
    let raw = RawInput {
        buttons: ((controller + field::CURRENT_BUTTONS) as *const u32).read_volatile(),
        lstick: (raw_axis(f32_at(LEFT_STICK_X)), raw_axis(f32_at(LEFT_STICK_Y))),
        rstick: (raw_axis(f32_at(RIGHT_STICK_X)), raw_axis(f32_at(RIGHT_STICK_Y))),
    };
    let candidate = Candidate { raw, smash: *out, status, fighter_kind };
    LOG.lock().record(&settings, candidate);
}

/// One in-game frame of the primary CPU body (once per frame, whatever the number of bodies).
pub fn tick_frame() {
    LOG.lock().tick_frame();
}

/// Session clear: leaving Training, a hot swap, a lobby change.
pub fn session_clear() {
    LOG.lock().session_clear();
    RENDERER.lock().session_clear();
}

/// The plugin''s layout was registered (or the draw was reset): every retained row is drawn again.
pub fn on_layout_registered() {
    RENDERER.lock().on_layout_registered();
}

/// A pane of the log is missing from the layout (SPEC-input-02 question 5): the display is off for the session.
pub fn disable() {
    RENDERER.lock().disable();
}

/// One HUD tick: the pane writes the display needs now (the layout draw in `hud_draw` applies them).
/// `menu_open` is the plugin menu''s flag or the vanilla pause menu''s.
pub fn hud_tick(menu_open: bool) -> Plan {
    let settings = settings();
    let log = *LOG.lock();
    RENDERER.lock().plan(&log, &settings, !menu_open)
}