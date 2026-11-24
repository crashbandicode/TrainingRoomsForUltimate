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

use crate::model::boot_notices;
use crate::model::frame_strip_plan::{Changed, Look};
use crate::model::hud_row;
use crate::model::notice_box;
use crate::model::notify::TickOut;
use crate::model::panes::{self as p, cb, iv, strip_layout::C};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{combo, input_display, notices, parry};
use crate::runtime::lobby::training;
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

/// `ui::hud::HudDraw`: `layout` is the layout being ticked and `name` its name.
///
/// # Safety
/// `layout` is the live layout the tick passed; the call is made on the UI job thread inside the tick.
pub unsafe fn draw(layout: *mut Layout, name: &[u8]) {
    let waiting = name == INFO_WAITING;
    let active = if waiting {
        let Ok(active) = crate::runtime::lobby::training::lobby::try_requested() else { return; };
        active
    } else {
        name == INFO_TRAINING && scene::is_training()
    };
    if !active || layout.is_null() {
        return;
    }
    if !waiting {
        // rc.8: the game speed bind's Training write, on this tick (the UI job thread), the native way.
        crate::runtime::features::game_speed::training_tick();
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
            // Without the strip's container (another mod's layout, the stock archive) the strip neither samples nor
            // takes the frame advantage from the notice (review IV-2).
            input_display::set_panes(strip[iv::at::CONTAINER] != 0);
            *c = Containers { layout: layout as usize, registrations, notice, strip, bar };
        }
        (c.notice as *mut Pane, c.strip, c.bar)
    };
    let owners = published::input_owners();
    let menu_open = owners.covers_hud();
    let strip_shown = !published::WIZARDS.hides_strip(owners) && input_display::sampling();
    // Lobby practice reserves the top band for the native panels.
    let drop = C.scene_drop(scene::current() == Some(crate::model::scene::Scene::Lobby));
    let draw_settings = input_display::settings();
    let parry = strip_shown.then(parry::view).flatten();
    let card = if bar[cb::at::CONTAINER] != 0 {
        let mut drawn = None;
        combo::hud_tick(strip_shown, draw_settings.link_history, parry.is_some(), &mut drawn, |local, look, changed| {
            write_look(bar[local] as *mut Pane, p::IDX_CB + local, look, changed)
        });
        // A reason that wrapped is taller than its row, and its card grows by the wrapped line.
        if let Some(Some((sequence, lines))) = drawn {
            let reason = bar[cb::at::REASON] as *mut Pane;
            if !reason.is_null() {
                pane::set_height(reason, C.card.reason_height(lines));
            }
            let plate = bar[if sequence { cb::at::SEQ_BACKDROP } else { cb::at::BACKDROP }] as *mut Pane;
            if !plate.is_null() {
                pane::set_height(plate, C.card.grown(sequence, lines)[3]);
            }
        }
        // This draw's own card; only a contended tick falls back to the last one drawn.
        drawn.unwrap_or_else(|| combo::card(strip_shown))
    } else {
        None
    };
    let btt = crate::runtime::features::btt::hud();
    let row = (!menu_open).then(|| hud_row::hud_row(crate::engine::rr_host::capture_take(), btt.as_str())).flatten();
    let chip = row.as_ref().map(|row| hud_row::hud_row_box(row, strip_shown, card.is_some()));
    write_notice(
        notice,
        &notices::hud_tick(boot_notices::hud_ready(!notice.is_null(), waiting, training::session_open())),
        chip,
        drop,
    );
    let drawn = write_btt(root, row.as_ref(), chip.map(|[x, y, w, h]| [x, y - drop, w, h]));
    crate::runtime::bridge_state::note_recording_indicator(drawn);

    let container = strip[iv::at::CONTAINER] as *mut Pane;
    if !container.is_null() {
        let labels = [strip[iv::at::RUN_LABEL] != 0, strip[iv::at::RUN_LABEL + 1] != 0];
        let (history, aims) = input_display::hud_tick(
            strip_shown, draw_settings, parry.as_ref(), card.is_some(), labels, |local, look, changed| {
                write_look(strip[local] as *mut Pane, p::IDX_IV + local, look, changed)
            },
        );
        crate::runtime::bridge_state::note_strip_aims(aims);
        let backdrop = strip[iv::at::BACKDROP] as *mut Pane;
        if !backdrop.is_null() {
            pane::set_height(backdrop, C.strip.bounds(history, card.is_some())[3]);
        }
        pane::set_pos(container, C.strip.origin_x, C.strip.origin_y - drop);
        if strip_shown {
            let anchors = super::hitbox::fighter_screen_anchors(name);
            let alpha = C.opacity(drop, history, card, anchors.into_iter().flatten().flatten().map(|anchor| anchor.canvas));
            pane::set_alpha(container, alpha);
        }
    } else {
        crate::runtime::bridge_state::note_strip_aims([None; 2]);
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

// ---- the notice slot ----------------------------------------------------------------------------------

/// The height of the notice box drawn last (`f32` bits), which `notice_box::stack` keeps clear of the BTT chip each tick.
static NOTICE_H: AtomicU32 = AtomicU32::new(0);

/// The compact notice box (`model::notice_box`): sized to the notice when it is drawn, at the top left, under the BTT
/// chip (`chip`, `panes::btt_hud_box`) while that shows.
unsafe fn write_notice(container: *mut Pane, out: &TickOut, chip: Option<[f32; 4]>, drop: f32) {
    if container.is_null() {
        return;
    }
    pane::set_visible(container, out.visible);
    crate::runtime::bridge_state::note_notice_visible(out.visible);
    if let Some(draw) = out.draw {
        let mut buffer = [0u16; crate::model::notify::BODY_MAX];
        let len = crate::model::boot_notices::notice_units(draw.body.as_str(), &mut buffer);
        let mut heading = [0u16; notice_box::HEADER_UNITS];
        let (placed, header_len, len) = notice_box::layout(draw.header, &mut heading, &mut buffer, len);
        NOTICE_H.store(placed.plate[3].to_bits(), Ordering::Release);
        let long = len > crate::model::boot_notices::SHORT_UNITS;
        // The one plate behind both texts shows with the header; a long body uses the long text and hides the short one.
        let [header, plate, text, long_text] =
            [p::IDX_NOTICE_HEADER, p::IDX_NOTICE_HEADER_PLATE, p::IDX_NOTICE_TEXT, p::IDX_NOTICE_LONG].map(|i| find(container, i));
        place(plate, placed.plate, out.visible);
        place(header, placed.header, out.visible);
        place(text, placed.body, out.visible && !long);
        place(long_text, placed.body, out.visible && long);
        if !header.is_null() {
            pane::set_text(header, &heading[..header_len]);
        }
        let target = if long { long_text } else { text };
        if !target.is_null() {
            pane::set_text(target, &buffer[..len]);
            pane::set_text_color(target, p::linear(draw.color));
        }
        crate::runtime::bridge_state::note_notice(draw.header, draw.body.as_str());
        if crate::runtime::export::wanted() {
            diagnostic(format_args!(
                "[TrainingRooms] notice_draw header={:?} body={:?} color={:?} plate={:?}",
                draw.header,
                draw.body.as_str(),
                draw.color,
                placed.plate
            ));
        }
    }
    pane::set_pos(container, 0.0, notice_box::stack(f32::from_bits(NOTICE_H.load(Ordering::Acquire)), chip) - drop);
}

/// A notice pane shown or hidden, and when shown placed at `[centre x, centre y, w, h]`.
unsafe fn place(target: *mut Pane, [x, y, w, h]: [f32; 4], shown: bool) {
    if target.is_null() {
        return;
    }
    pane::set_visible(target, shown);
    if shown {
        pane::set_pos(target, x, y);
        pane::set_width(target, w);
        pane::set_height(target, h);
    }
}

/// The compact HUD row (`model::hud_row`): Break the Targets' line or the recording indicator. Existing builders and the
/// target's material, independent of the transient notice queue. Returns the take it drew: none while the row is hidden
/// or the layout has no such row (the stock layout), so the bridge never names a chip that is not on screen.
unsafe fn write_btt(root: *mut Pane, row: Option<&hud_row::HudRow>, chip: Option<[f32; 4]>) -> Option<hud_row::Take> {
    let container = pane::find_child(root, p::name_of(p::IDX_BTT).as_bytes());
    if container.is_null() { return None; }
    pane::set_visible(container, chip.is_some());
    let (Some(row), Some([x, y, w, _])) = (row, chip) else { return None };
    pane::set_pos(container, x, y);
    let plate = find(container, p::IDX_BTT + 1);
    if !plate.is_null() {
        pane::set_pos(plate, w / 2.0, 0.0);
        pane::set_width(plate, w);
    }
    let picture = find(container, p::IDX_BTT + 2);
    if !picture.is_null() {
        pane::set_rect_color(picture, p::linear(row.chip));
    }
    let text = find(container, p::IDX_BTT + 3);
    if !text.is_null() {
        pane::set_pos(text, 52.0 + (w - 64.0) / 2.0, 0.0);
        pane::set_width(text, w - 64.0);
        let mut buffer = [0; crate::model::notify::BODY_MAX];
        let n = crate::model::boot_notices::notice_units(row.text.as_str(), &mut buffer);
        pane::set_text(text, &buffer[..n]);
        pane::set_text_color(text, p::linear(row.color));
    }
    row.take.filter(|_| !picture.is_null() && !text.is_null())
}
