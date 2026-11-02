//! The per-layout tick (SPEC-core-05 §4.1, §5): called for every live layout every frame on the UI job
//! thread, and once for a newly registered layout on the main thread. For the Training HUD it resolves the
//! menu's panes once per layout registration, detects the game's pause menu, closes the 15-frame window
//! after the menu closes, and draws the worker's frame when it changed.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use parking_lot::Mutex;
use skyline::nn::ui2d::{Layout, Pane};

use crate::model::input_delay::Owners;
use crate::model::frame::{fade_alpha, CLOSE_FRAMES};
use crate::model::panes as p;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::published::INPUT_OWNERS;
use crate::runtime::ui::state::{CLOSE_COUNT, CLOSING, FRAME, FRAME_SEQ, MENU_SHOWN, OPEN, PAUSE_ACTIVE};
use crate::runtime::ui::view::Panes;
use crate::runtime::ui::{hud, pane, registry};
use crate::runtime::game;

const INFO_TRAINING: &[u8] = b"info_training";

struct Resolved {
    layout: usize,
    panes: Option<Panes>,
    /// The game's own HUD status panel, hidden while the menu shows (SPEC-core-03 §2.2).
    status_r: *mut Pane,
    /// Its help footer: rests at y = -80 and slides in when the game's pause menu opens.
    help: *mut Pane,
    drawn_seq: u32,
    shown: bool,
}

// SAFETY: the pointers are only used on the UI job thread inside the tick.
unsafe impl Send for Resolved {}

static RESOLVED: Mutex<Resolved> = Mutex::new(Resolved {
    layout: 0,
    panes: None,
    status_r: core::ptr::null_mut(),
    help: core::ptr::null_mut(),
    drawn_seq: 0,
    shown: false,
});

/// Set by the registration tick: the layout object is new, so its panes must be resolved again.
static INVALIDATE: AtomicBool = AtomicBool::new(false);
pub static TICKS: AtomicU32 = AtomicU32::new(0);
pub static DRAWS: AtomicU32 = AtomicU32::new(0);

/// `L_staying_help`'s resting `pos_y` (SPEC-core-03 §2.4).
const HELP_REST_Y: f32 = -80.0;

pub unsafe fn tick(layout: *mut Layout, registration: bool) {
    let Some(name) = registry::layout_name(layout) else { return };
    hud::run(layout, name);
    hud::hud_rule(layout, name);
    if name != INFO_TRAINING {
        return;
    }
    if registration {
        INVALIDATE.store(true, Ordering::Release);
    }
    if !game::is_training_mode() {
        return;
    }
    TICKS.fetch_add(1, Ordering::Relaxed);
    let Some(mut r) = RESOLVED.try_lock() else { return };
    if r.layout != layout as usize || INVALIDATE.swap(false, Ordering::AcqRel) {
        resolve(&mut r, layout);
    }
    let r = &mut *r;
    let Resolved { panes, status_r, help, drawn_seq, shown, .. } = r;
    let Some(panes) = panes.as_ref() else { return };

    if !help.is_null() {
        PAUSE_ACTIVE.store((pane::pos_y(*help) - HELP_REST_Y).abs() > 0.5, Ordering::Release);
    }
    let open = OPEN.load(Ordering::Acquire);
    let mut alpha = 255;
    let mut showing = open;
    if CLOSING.load(Ordering::Acquire) {
        let count = CLOSE_COUNT.fetch_add(1, Ordering::AcqRel) + 1;
        if count >= CLOSE_FRAMES {
            CLOSING.store(false, Ordering::Release);
            INPUT_OWNERS.fetch_and(!Owners::MENU.bits(), Ordering::AcqRel);
        } else {
            showing = true;
            alpha = fade_alpha(count);
        }
    }
    let visible = showing && !PAUSE_ACTIVE.load(Ordering::Acquire);
    MENU_SHOWN.store(visible, Ordering::Release);
    pane::set_visible(panes.container, visible);
    if visible {
        pane::set_alpha(panes.container, alpha);
    }
    if !status_r.is_null() && visible != *shown {
        pane::set_visible(*status_r, !visible);
    }
    let was_shown = *shown;
    *shown = visible;
    let seq = FRAME_SEQ.load(Ordering::Acquire);
    if visible && (seq != *drawn_seq || !was_shown) {
        if let Some(frame) = FRAME.try_lock() {
            panes.draw(&frame);
            *drawn_seq = seq;
            if DRAWS.fetch_add(1, Ordering::Relaxed) == 0 {
                diagnostic(format_args!("[Ui] first draw seq={seq}"));
            }
        }
    }
}

unsafe fn resolve(r: &mut Resolved, layout: *mut Layout) {
    r.layout = layout as usize;
    r.panes = None;
    r.status_r = core::ptr::null_mut();
    r.help = core::ptr::null_mut();
    r.drawn_seq = 0;
    r.shown = false;
    let root = (*layout).root_pane;
    let total = pane::count(root);
    r.status_r = pane::find(root, b"status_R");
    r.help = pane::find(root, b"L_staying_help");
    let mut all = [core::ptr::null_mut(); p::COUNT];
    let mut missing = 0;
    for (index, slot) in all.iter_mut().enumerate() {
        *slot = pane::find(root, p::name_of(index).as_bytes());
        missing += usize::from(slot.is_null());
    }
    if missing == p::COUNT {
        diagnostic(format_args!("[Ui] layout=info_training panes_absent total={total} (stock layout)"));
        return;
    }
    let capacity = if all[p::row_label_index(0)].is_null() { 0 } else { pane::text_capacity(all[p::row_label_index(0)]) };
    diagnostic(format_args!(
        "[Ui] layout=info_training resolved total={total} holders={} menu_panes={} missing={missing} text_capacity={capacity} help={} status_r={}",
        registry::holders().len(),
        p::COUNT,
        !r.help.is_null(),
        !r.status_r.is_null()
    ));
    if missing == 0 {
        #[cfg(feature = "diag")]
        dump_text_box(all[p::row_label_index(0)]);
        let container = all[p::IDX_MENU];
        r.panes = Some(Panes { container, all });
    }
}
/// Diag only: the fields around a text box's buffer length, to check the binding's layout against the game.
#[cfg(feature = "diag")]
unsafe fn dump_text_box(pane: *mut Pane) {
    use skyline::nn::ui2d::TextBox;
    let tb = pane as *mut TextBox;
    let words = core::ptr::addr_of!((*tb).text_buf_len) as *const u16;
    let mut line = [0u16; 24];
    for (i, slot) in line.iter_mut().enumerate() {
        *slot = words.sub(4).add(i).read_unaligned();
    }
    diagnostic(format_args!(
        "[Ui] text_box buf_len={} len={} bits={:#x} pos={} size_of_pane={} buffer={:#x} around={:04x?}",
        (*tb).text_buf_len,
        (*tb).text_len,
        (*tb).bits,
        (*tb).text_position,
        core::mem::size_of::<Pane>(),
        (*tb).text_buf as usize,
        line
    ));
}