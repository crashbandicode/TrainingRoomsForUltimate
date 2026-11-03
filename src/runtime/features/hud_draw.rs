//! Package 4's one HUD draw: the callback registered with package 5's layout tick (`ui::hud::register`). Once per
//! tick of the plugin's `info_training` layout it runs the notice queue and the input display, and writes their
//! answers into the panes of the plugin's own layout part (SPEC-core-05) by name: the notice slot `TrModDisp0`
//! (header, text, colour) and the input log (`TrModInputLog`, its 15 rows and their slots).
//!
//! Runs on the UI job thread, inside the tick. No pane pointer is kept between ticks: every write finds its pane
//! by name below the layout's root (the panes the plan names are few after the first full refresh). When the
//! layout is registered again everything the plugin wrote is gone, so the next tick draws it all again.
//!
//! The in-game frame counters (`input_display::tick_frame`) are not here: they tick with the simulation, in the CPU
//! body's frame, whatever the number of UI ticks.

use core::sync::atomic::{AtomicU32, Ordering};
use std::fmt::Write;

use hos_lock::Mutex;

use skyline::nn::ui2d::{Layout, Pane};

use crate::model::input_delay::Owners;
use crate::model::input_display::{Glyph, Icons, Op, Plan, Slot, StatusText, ICON_SLOTS};
use crate::model::notify::TickOut;
use crate::model::panes as p;
use crate::model::storage::FixedPath;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{input_display, notices};
use crate::runtime::ui::{pane, tick};
use crate::runtime::{published, scene};

/// The layout this draw serves.
const INFO_TRAINING: &[u8] = b"info_training";
const INFO_WAITING: &[u8] = b"info_waiting";

/// The `tick::REGISTRATIONS` value the last draw saw.
static SEEN_REGISTRATIONS: AtomicU32 = AtomicU32::new(0);

/// The two containers of the plugin's layout part, resolved once per registration of the layout (a walk from the
/// root visits the stock panes first and is capped, so the plugin's own panes are searched below their container).
struct Containers {
    layout: usize,
    registrations: u32,
    notice: usize,
    column: usize,
}

/// The coalescer of the `input_display_plan` diag line.
#[cfg(feature = "diag")]
static PLAN_LOG: Mutex<crate::model::input_display::PlanLogGate> = Mutex::new(crate::model::input_display::PlanLogGate::new());

static CONTAINERS: Mutex<Containers> = Mutex::new(Containers { layout: 0, registrations: 0, notice: 0, column: 0 });

/// The longest text written to one pane (the status text's 48 units, the held-frames' 4).
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
    }
    let (notice, column) = {
        let mut c = CONTAINERS.lock();
        if c.layout != layout as usize || c.registrations != registrations {
            *c = Containers { layout: layout as usize, registrations, notice: pane::find_child(root, p::name_of(p::IDX_NOTICE).as_bytes()) as usize, column: pane::find_child(root, p::name_of(p::IDX_LOG).as_bytes()) as usize };
        }
        (c.notice as *mut Pane, c.column as *mut Pane)
    };
    let owners = published::input_owners();
    let menu_open = owners.contains(Owners::MENU) || owners.contains(Owners::VANILLA_PAUSE);
    write_notice(notice, &notices::hud_tick(!waiting && !notice.is_null()));
    let plan = input_display::hud_tick(menu_open);
    if !plan.is_empty() {
        #[cfg(feature = "diag")]
        {
            let rows = plan.ops().iter().filter(|op| matches!(op, Op::Row { .. })).count();
            let icons = plan.ops().iter().filter(|op| matches!(op, Op::Icons { .. })).count();
            // Coalesced (`PlanLogGate`): the line is for a changed plan or a heartbeat, never one per tick; the check and the
            // line format on the stack and allocate nothing, so the hook counter stays at 0 with the HUD drawing.
            if PLAN_LOG.lock().should_log(plan.ops().len(), rows, icons) {
                diagnostic(format_args!("[TrainingRooms] input_display_plan ops={} rows={rows} icons={icons}", plan.ops().len()));
            }
        }
        write_plan(column, &plan);
    }
}

/// A pane of the plugin's layout part by its index in the pane table.
unsafe fn find(root: *mut Pane, index: usize) -> *mut Pane {
    pane::find(root, p::name_of(index).as_bytes())
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

// ---- the input log ------------------------------------------------------------------------------------

/// `cos` and `sin` of the eight stick directions (0 = right, counter-clockwise, 45 degrees apart): a table, because
/// the float library's `cos` and `sin` link the whole math library into the NRO.
const DIRECTION: [(f32, f32); 8] = [
    (1.0, 0.0),
    (0.707_106_77, 0.707_106_77),
    (0.0, 1.0),
    (-0.707_106_77, 0.707_106_77),
    (-1.0, 0.0),
    (-0.707_106_77, -0.707_106_77),
    (0.0, -1.0),
    (0.707_106_77, -0.707_106_77),
];

unsafe fn write_plan(column: *mut Pane, plan: &Plan) {
    // `root` below is the log column: every pane of the log is its descendant.
    let root = column;
    for op in plan.ops() {
        match *op {
            Op::Column { visible } => {
                if column.is_null() {
                    missing("TrModInputLog");
                    return;
                }
                pane::set_visible(column, visible);
            }
            Op::Row { phys, y, alpha } => {
                let row = find(root, p::IDX_LOG_ROW + usize::from(phys));
                if row.is_null() {
                    missing("row");
                    return;
                }
                pane::set_pos(row, pane::pos_x(row), y);
                pane::set_alpha(row, alpha);
                pane::set_visible(row, true);
            }
            Op::RowHidden { phys } => {
                let row = find(root, p::IDX_LOG_ROW + usize::from(phys));
                if !row.is_null() {
                    pane::set_visible(row, false);
                }
            }
            Op::Icons { phys, icons } => write_icons(root, usize::from(phys), &icons),
            Op::Frames { phys, frames } => {
                let target = find(root, p::IDX_LOG_FRAME + usize::from(phys));
                let backing = pane::preceding_sibling(target, p::name_of(p::IDX_LOG_FRAME_BASE + usize::from(phys)).as_bytes());
                if backing.is_null() {
                    missing("frame backing");
                    return;
                }
                {
                    let mut text = FixedPath::<8>::new();
                    let _ = write!(text, "{frames}");
                    set_text(target, text.as_str());
                    pane::set_visible(backing, true);
                }
            }
            Op::Status { phys, text } => {
                let target = find(root, p::IDX_LOG_STATUS + usize::from(phys));
                let backing = pane::preceding_sibling(target, p::name_of(p::IDX_LOG_STATUS_BASE + usize::from(phys)).as_bytes());
                if backing.is_null() {
                    missing("status backing");
                    return;
                }
                {
                    pane::set_visible(backing, !matches!(text, StatusText::Empty));
                    match text {
                        StatusText::Empty => set_text(target, ""),
                        StatusText::Id { kind, status } => {
                            let mut line = FixedPath::<32>::new();
                            let _ = write!(line, "{kind}:{status}");
                            set_text(target, line.as_str());
                        }
                    }
                }
            }
        }
    }
}

/// A pane the plan needs is not in the layout (the stock archive is showing): log once, the display is off for the
/// session instead of panicking (SPEC-input-02 question 5).
fn missing(what: &str) {
    diagnostic(format_args!("[TrainingRooms] input_display pane missing: {what}; display off for this session"));
    input_display::disable();
}

unsafe fn write_icons(root: *mut Pane, phys: usize, icons: &Icons) {
    for slot in 0..ICON_SLOTS {
        let at = p::log_slot(phys, slot);
        let glyph = find(root, p::IDX_LOG_GLYPH + at);
        let shaft = find(root, p::IDX_LOG_SHAFT + at);
        let head = find(root, p::IDX_LOG_HEAD + at);
        if glyph.is_null() || shaft.is_null() || head.is_null() {
            missing("icon slot");
            return;
        }
        match icons.slots()[slot] {
            None => {
                pane::set_visible(glyph, false);
                pane::set_visible(shaft, false);
                pane::set_visible(head, false);
            }
            Some(Slot { icon, color }) => {
                let rgba = p::linear([color.0, color.1, color.2, 255]);
                match icon.glyph() {
                    Glyph::Font(unit) | Glyph::Letter(unit) => {
                        pane::set_text(glyph, &[unit]);
                        pane::set_text_color(glyph, rgba);
                        pane::set_visible(glyph, true);
                        pane::set_visible(shaft, false);
                        pane::set_visible(head, false);
                    }
                    Glyph::Arrow(degrees) => {
                        pane::set_visible(glyph, false);
                        // The shaft is vertical and the head sits above it in the layout: turn both to the
                        // direction and put the head that far along it from the shaft's centre.
                        let (c, s) = DIRECTION[usize::from(degrees / 45) % 8];
                        let turn = f32::from(degrees) - 90.0;
                        let x = p::LOG_SLOT_X0 + p::LOG_SLOT_PITCH * slot as f32;
                        pane::set_rot_z(shaft, turn);
                        pane::set_rot_z(head, turn);
                        pane::set_pos(head, x + p::LOG_HEAD_OFFSET * c, p::LOG_HEAD_OFFSET * s);
                        pane::set_rect_color(shaft, rgba);
                        pane::set_rect_color(head, rgba);
                        pane::set_visible(shaft, true);
                        pane::set_visible(head, true);
                    }
                }
            }
        }
    }
}
