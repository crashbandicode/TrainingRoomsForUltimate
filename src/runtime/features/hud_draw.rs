//! Package 4's one HUD draw: the callback registered with package 5's layout tick (`ui::hud::register`). Once per
//! tick of the plugin's `info_training` layout it runs the notice queue and the frame strip (SPEC-input-06), and
//! writes their answers into the panes of the plugin's own layout part (SPEC-core-05): the notice slot `TrModDisp0`
//! (header, text, colour) and the strip (`TrIv` and its panes).
//!
//! Runs on the UI job thread, inside the tick. The containers and the strip's panes are found by name once per
//! registration of the layout; when it is registered again everything the plugin wrote is gone, so the next tick
//! finds and draws it all again.

use core::sync::atomic::{AtomicU32, Ordering};

use hos_lock::Mutex;

use skyline::nn::ui2d::{Layout, Pane};

use crate::model::frame_strip_plan::{Changed, Look};
use crate::model::input_delay::Owners;
use crate::model::notify::TickOut;
use crate::model::panes::{self as p, cb, iv};
#[cfg(feature = "diag")]
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{input_display, notices};
use crate::runtime::ui::{pane, tick};
use crate::runtime::{published, scene};

/// The layout this draw serves.
const INFO_TRAINING: &[u8] = b"info_training";
const INFO_WAITING: &[u8] = b"info_waiting";

/// The `tick::REGISTRATIONS` value the last draw saw.
static SEEN_REGISTRATIONS: AtomicU32 = AtomicU32::new(0);

/// The notice container and the frame strip's panes, resolved once per registration of the layout (a walk from the
/// root visits the stock panes first and is capped, so the plugin's own panes are searched below their container).
/// A strip pane the layout lacks stays 0 and is not written.
struct Containers {
    layout: usize,
    registrations: u32,
    notice: usize,
    strip: [usize; iv::COUNT],
    /// The combo bar's panes (SPEC-input-07 §7), below its container `TrCb` inside the strip's.
    bar: [usize; cb::COUNT],
}

static CONTAINERS: Mutex<Containers> =
    Mutex::new(Containers { layout: 0, registrations: 0, notice: 0, strip: [0; iv::COUNT], bar: [0; cb::COUNT] });

/// The longest text written to one pane (the notice header's 24 units).
const TEXT_UNITS: usize = 48;

/// `ui::hud::HudDraw`: `layout` is the layout being ticked and `name` its name.
///
/// # Safety
/// `layout` is the live layout the tick passed; the call is made on the UI job thread inside the tick.
pub unsafe fn draw(layout: *mut Layout, name: &[u8]) {
    let waiting = name == INFO_WAITING;
    let active = if waiting {
        crate::runtime::lobby::training::lobby::requested()
    } else {
        name == INFO_TRAINING && scene::is_training()
    };
    if !active || layout.is_null() {
        return;
    }
    let root = if waiting { pane::find_child((*layout).root_pane, b"TrModPort") } else { (*layout).root_pane };
    if root.is_null() {
        return;
    }
    let registrations = tick::REGISTRATIONS.load(Ordering::Acquire);
    if SEEN_REGISTRATIONS.swap(registrations, Ordering::AcqRel) != registrations {
        #[cfg(feature = "diag")]
        diagnostic(format_args!("[TrainingRooms] hud_draw layout registrations={registrations}"));
        notices::on_layout_registered();
        input_display::on_layout_registered();
        crate::runtime::features::combo::on_layout_registered();
    }
    let (notice, strip, bar) = {
        let mut c = CONTAINERS.lock();
        if c.layout != layout as usize || c.registrations != registrations {
            let container = pane::find_child(root, p::name_of(p::IDX_IV).as_bytes());
            let mut strip = [0usize; iv::COUNT];
            if !container.is_null() {
                strip[iv::at::CONTAINER] = container as usize;
                for (local, slot) in strip.iter_mut().enumerate().skip(1) {
                    *slot = pane::find(container, p::name_of(p::IDX_IV + local).as_bytes()) as usize;
                }
            }
            let mut bar = [0usize; cb::COUNT];
            let bar_root = if container.is_null() { container } else { pane::find(container, p::name_of(p::IDX_CB).as_bytes()) };
            if !bar_root.is_null() {
                bar[cb::at::CONTAINER] = bar_root as usize;
                for (local, slot) in bar.iter_mut().enumerate().skip(1) {
                    *slot = pane::find(bar_root, p::name_of(p::IDX_CB + local).as_bytes()) as usize;
                }
            }
            #[cfg(feature = "diag")]
            diagnostic(format_args!(
                "[FrameStrip] panes resolved found={} of {}",
                strip.iter().filter(|s| **s != 0).count(),
                iv::COUNT
            ));
            let notice = pane::find_child(root, p::name_of(p::IDX_NOTICE).as_bytes()) as usize;
            *c = Containers { layout: layout as usize, registrations, notice, strip, bar };
        }
        (c.notice as *mut Pane, c.strip, c.bar)
    };
    let owners = published::input_owners();
    let menu_open = owners.contains(Owners::MENU) || owners.contains(Owners::VANILLA_PAUSE);
    write_notice(notice, &notices::hud_tick(!waiting && !notice.is_null()));
    if strip[iv::at::CONTAINER] != 0 {
        input_display::hud_tick(menu_open, |local, look, changed| {
            write_look(strip[local] as *mut Pane, p::IDX_IV + local, look, changed)
        });
    }
    if bar[cb::at::CONTAINER] != 0 {
        let strip_shown = !menu_open && input_display::sampling();
        crate::runtime::features::combo::hud_tick(strip_shown, |local, look, changed| {
            write_look(bar[local] as *mut Pane, p::IDX_CB + local, look, changed)
        });
    }
}

/// A pane of the plugin's layout part by its index in the pane table.
unsafe fn find(root: *mut Pane, index: usize) -> *mut Pane {
    pane::find(root, p::name_of(index).as_bytes())
}

/// One frame-strip pane: only the parts `changed` names (SPEC-input-06 §5.2). Colours are sRGB in the look and
/// linear light in the pane.
unsafe fn write_look(target: *mut Pane, index: usize, look: &Look, changed: Changed) {
    if target.is_null() {
        return;
    }
    if changed.visible {
        pane::set_visible(target, look.visible);
    }
    if !look.visible {
        return;
    }
    if changed.pos {
        pane::set_pos(target, look.x, look.y);
    }
    if changed.width && look.w > 0.0 {
        pane::set_width(target, look.w);
    }
    if changed.rot {
        pane::set_rot_z(target, look.rot);
    }
    match p::spec(index).kind {
        p::Kind::Rect { .. } if changed.color => pane::set_rect_color(target, p::linear(look.color)),
        p::Kind::Text { .. } => {
            if changed.text {
                pane::set_text(target, look.text.units());
            }
            if changed.color {
                pane::set_text_color(target, p::linear(look.color));
            }
        }
        _ => {}
    }
}

/// UTF-16 units of `text` (cut to `TEXT_UNITS`) in `out`; returns how many.
fn units(text: &str, out: &mut [u16; TEXT_UNITS]) -> usize {
    let mut len = 0;
    for unit in text.encode_utf16() {
        if len == out.len() {
            break;
        }
        out[len] = unit;
        len += 1;
    }
    len
}

unsafe fn set_text(target: *mut Pane, text: &str) {
    let mut buffer = [0u16; TEXT_UNITS];
    let len = units(text, &mut buffer);
    pane::set_text(target, &buffer[..len]);
}

// ---- the notice slot ----------------------------------------------------------------------------------

unsafe fn write_notice(container: *mut Pane, out: &TickOut) {
    if container.is_null() {
        return;
    }
    pane::set_visible(container, out.visible);
    let Some(draw) = out.draw else { return };
    let header = find(container, p::IDX_NOTICE_HEADER);
    let text = find(container, p::IDX_NOTICE_TEXT);
    let long_text = find(container, p::IDX_NOTICE_LONG);
    let base = find(container, p::IDX_NOTICE_BASE);
    if !header.is_null() {
        set_text(header, draw.header);
    }
    let mut buffer = [0u16; crate::model::notify::BODY_MAX];
    let len = crate::model::boot_notices::notice_units(draw.body.as_str(), &mut buffer);
    let long = len > crate::model::boot_notices::SHORT_UNITS;
    if !text.is_null() { pane::set_visible(text, !long); }
    if !long_text.is_null() { pane::set_visible(long_text, long); }
    if !base.is_null() {
        pane::set_pos(base, 0.0, if long { 355.0 } else { 405.0 });
        pane::set_height(base, if long { 250.0 } else { 150.0 });
    }
    let target = if long { long_text } else { text };
    if !target.is_null() {
        pane::set_text(target, &buffer[..len]);
        pane::set_text_color(target, p::linear(draw.color));
    }
    #[cfg(feature = "diag")]
    diagnostic(format_args!(
        "[TrainingRooms] notice_draw header={:?} body={:?} color={:?}",
        draw.header,
        draw.body.as_str(),
        draw.color
    ));
}

