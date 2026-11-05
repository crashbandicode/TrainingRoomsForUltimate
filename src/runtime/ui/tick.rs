//! The per-layout tick (SPEC-core-05 §4.1, §5): called for every live layout every frame on the UI job
//! thread, and once for a newly registered layout on the main thread. For the Training HUD it resolves the
//! menu's panes once per layout registration, detects the game's pause menu, closes the 15-frame window
//! after the menu closes, and draws the worker's frame when it changed.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use hos_lock::Mutex;
use skyline::nn::ui2d::{Layout, Pane};

use crate::model::input_delay::Owners;
use crate::model::frame::{fade_alpha, CLOSE_FRAMES};
use crate::model::panes as p;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::published::INPUT_OWNERS;
use crate::runtime::ui::state::{self, CLOSE_COUNT, CLOSING, EXT_SHOWN, FRAME, FRAME_SEQ, LAYOUT_READY, MENU_SHOWN, OPEN, PAUSE_ACTIVE, POLLS_AT_TICK};
use crate::runtime::ui::requests;
use crate::runtime::ui::{chips, thumbs};
use crate::runtime::ui::view::Panes;
use crate::runtime::ui::{hud, pane, registry};
use crate::runtime::game;

const INFO_TRAINING: &[u8] = b"info_training";
const INFO_WAITING: &[u8] = b"info_waiting";

struct Resolved {
    layout: usize,
    panes: Option<Panes>,
    /// The game's own HUD status panel, hidden while the menu shows (SPEC-core-03 §2.2).
    status_r: *mut Pane,
    drawn_seq: u32,
    shown: bool,
}

// SAFETY: the pointers are only used on the UI job thread inside the tick.
unsafe impl Send for Resolved {}

static RESOLVED: Mutex<Resolved> = Mutex::new(Resolved {
    layout: 0,
    panes: None,
    status_r: core::ptr::null_mut(),
    drawn_seq: 0,
    shown: false,
});

/// Set by the registration tick: the layout object is new, so its panes must be resolved again.
static INVALIDATE: AtomicBool = AtomicBool::new(false);
pub static TICKS: AtomicU32 = AtomicU32::new(0);
/// How many times `info_training` was registered (a new layout object): the other packages' HUD draws compare it
/// with the value they last saw to know that nothing they wrote to the layout is there any more.
pub static REGISTRATIONS: AtomicU32 = AtomicU32::new(0);
pub static DRAWS: AtomicU32 = AtomicU32::new(0);

/// Process the layout supplied by the current native Calculate callback.
///
/// # Safety
/// `layout` must be non-null and point to a live initialized layout and pane
/// tree for this callback. Native UI access/destruction must be serialized with
/// these reads/writes; regular passes run on the UI job thread. Pass
/// `registration = true` for every new layout lifetime, including address reuse,
/// before regular passes can use cached panes from that lifetime.
pub unsafe fn tick(layout: *mut Layout, registration: bool) {
    // These anchors are installed for native ownership even when the renderer's
    // archive/text/manager closure was refused. Never gate this on injected panes.
    super::native_pause::observe(layout, registration);
    if !state::INSTALLED.load(Ordering::Acquire) { return; }
    let Some(name) = registry::layout_name(layout) else { return };
    let waiting = name == INFO_WAITING;
    let practice_layout = name == INFO_TRAINING || waiting;
    if practice_layout && registration {
        // HUD consumers compare this generation before using cached containers.
        // Publish it before dispatch, including when a layout address is reused.
        INVALIDATE.store(true, Ordering::Release);
        REGISTRATIONS.fetch_add(1, Ordering::AcqRel);
    }
    hud::run(layout, name);
    hud::hud_rule(layout, name);
    if !practice_layout {
        return;
    }
    // Attribution (lead, 2026-11-02): the menu's own allocations inside the UI tick are charged here (thumbnails apart).
    let _scope = crate::runtime::alloc::HookScope::enter_hook(crate::runtime::alloc::HookId::MenuDraw);
    let lobby = crate::runtime::lobby::training::lobby::requested();
    if waiting {
        // The waiting layout can outlive a practice request. Hide its whole
        // plugin host when that request ends, even though the layout remains.
        let host = pane::find_child((*layout).root_pane, b"TrModPort");
        if !host.is_null() {
            pane::set_visible(host, lobby);
        }
    }
    if (waiting && !lobby) || (!waiting && !game::is_training_mode()) {
        renderer_lost("practice layout inactive");
        return;
    }
    TICKS.fetch_add(1, Ordering::Relaxed);
    // Registration is an initial calculate, not another rendered frame. Only
    // the regular pass advances the shared texture retirement grace.
    if !registration {
        crate::runtime::ui::texture::tick();
    }
    let Some(mut r) = RESOLVED.try_lock() else { return };
    if r.layout != layout as usize || INVALIDATE.swap(false, Ordering::AcqRel) {
        resolve(&mut r, layout, waiting);
    }
    let r = &mut *r;
    let Resolved { panes, status_r, drawn_seq, shown, .. } = r;
    let Some(panes) = panes.as_ref() else {
        // The injected archive was refused or the panes are absent: no renderer, so the menu cannot own input (R2-2).
        renderer_lost("panes unresolved");
        return;
    };
    LAYOUT_READY.store(true, Ordering::Release);
    crate::runtime::lobby::training::lobby::set_ui_ready(waiting);
    POLLS_AT_TICK.store(crate::runtime::ui::input::POLLS.load(Ordering::Relaxed), Ordering::Release);

    let open = OPEN.load(Ordering::Acquire);
    // A panel the menu does not own (the clip editor) is drawn with the menu closed.
    let ext = EXT_SHOWN.load(Ordering::Acquire);
    let mut alpha = 255;
    let mut showing = open || ext;
    if CLOSING.load(Ordering::Acquire) {
        let count = CLOSE_COUNT.fetch_add(1, Ordering::AcqRel) + 1;
        if count >= CLOSE_FRAMES {
            CLOSING.store(false, Ordering::Release);
            INPUT_OWNERS.fetch_and(!Owners::MENU.bits(), Ordering::AcqRel);
        } else {
            showing = true;
            alpha = if ext { 255 } else { fade_alpha(count) };
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
        // Wait-free (stream/sync): the worker hands frames over through a triple buffer; the tick reads its own slot.
        FRAME.with_latest(|frame, _| {
            // SAFETY: as the draw this tick made before (the panes were resolved from the live layout this tick).
            unsafe { panes.draw(frame) };
            thumbs::note_frame(frame);
            chips::note_frame(frame);
        });
        *drawn_seq = seq;
        if DRAWS.fetch_add(1, Ordering::Relaxed) == 0 {
            log(format_args!("[Ui] first draw seq={seq}"));
        }
    }
    // What the menu worker asked of the game (the Give item grid's item): made here, where the game is.
    requests::run_menu_requests();
    // The clip editor's held scrub repeats once per drawn frame of the Training layout (the layer's fter_draw, a no-op unless the
    // editor is in the foreground).
    crate::runtime::macros::after_layout_draw();
    // The stage grid's thumbnails (SPEC-core-07 section 4.5): requested when a stage page shows, released when it changes or
    // the menu closes; `thumbs::run` also lets the helper retire released textures, once per UI frame.
    if !visible {
        thumbs::hide();
        chips::hide();
    }
    // The texture helper's Eden proof (feature `texture-probe`): binds a few stage textures to menu tiles and photographs them.
    #[cfg(feature = "texture-probe")]
    crate::runtime::ui::texture_probe::tick(panes);
    if thumbs::run(panes) {
        FRAME_SEQ.fetch_add(1, Ordering::Release);
    }
    // The DI / SDI chips (SPEC-core-07 section 4.6): the game's own effect texture on the chip pointers while that grid shows.
    if chips::run(panes) {
        FRAME_SEQ.fetch_add(1, Ordering::Release);
    }
}

/// A log line from the tick. The tick runs inside the UI hooks; its lines are the once-per-registration kind (layout
/// resolved, panes absent, renderer lost, first draw): counted as per-load allocations, not as steady-state ones.
fn log(args: core::fmt::Arguments<'_>) {
    crate::model::hook_stats::per_load(|| diagnostic(args));
}

/// The layout cannot draw the menu now: drop the readiness and, if the menu owns Player 1's input, give it back.
fn renderer_lost(reason: &str) {
    crate::runtime::lobby::training::lobby::set_ui_ready(false);
    // SAFETY: on the UI job thread; the textures are released without touching a pane.
    unsafe {
        thumbs::lose_layout();
        chips::lose_layout();
    }
    let was_ready = LAYOUT_READY.swap(false, Ordering::AcqRel);
    if super::menu_app::captures_input() {
        state::release_menu();
        log(format_args!("[Ui] menu released: renderer lost ({reason})"));
    } else if was_ready {
        log(format_args!("[Ui] renderer lost ({reason})"));
    }
}

unsafe fn resolve(r: &mut Resolved, layout: *mut Layout, waiting: bool) {
    // The panes the thumbnails were bound to are about to be replaced: drop the textures first.
    thumbs::lose_layout();
    chips::lose_layout();
    r.layout = layout as usize;
    r.panes = None;
    r.status_r = core::ptr::null_mut();
    r.drawn_seq = 0;
    r.shown = false;
    let layout_name = if waiting { "info_waiting" } else { "info_training" };
    let root = (*layout).root_pane;
    let total = pane::count(root);
    r.status_r = pane::find(root, b"status_R");
    let help = pane::find(root, if waiting { b"option_menu_setting" } else { b"L_staying_help" });
    let menu_root = if waiting { pane::find_child(root, b"TrModPort") } else { root };
    // Waiting readiness covers every hosted panel, including the notification
    // and frame-strip consumers (SPEC-input-06), not just the menu subtree.
    let host_complete = !waiting || [p::IDX_NOTICE, p::IDX_IV].iter()
        .all(|&index| !pane::find_child(menu_root, p::name_of(index).as_bytes()).is_null());
    let mut all = [core::ptr::null_mut(); p::MENU_COUNT];
    let mut missing = 0;
    let container = pane::find_child(menu_root, p::ROOT.as_bytes());
    for (index, slot) in all.iter_mut().enumerate() {
        *slot = if index == p::IDX_MENU { container } else { pane::find(container, p::name_of(index).as_bytes()) };
        missing += usize::from(slot.is_null());
    }
    if container.is_null() {
        log(format_args!("[Ui] layout={layout_name} panes_absent total={total} (stock layout)"));
        return;
    }
    let capacity = if all[p::row_label_index(0)].is_null() { 0 } else { pane::text_capacity(all[p::row_label_index(0)]) };
    log(format_args!(
        "[Ui] layout={layout_name} resolved total={total} holders={} menu_panes={} missing={missing} host_complete={host_complete} text_capacity={capacity} help={} status_r={}",
        registry::holders().len(),
        p::MENU_COUNT,
        !help.is_null(),
        !r.status_r.is_null()
    ));
    if missing == 0 && host_complete {
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
    log(format_args!(
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
