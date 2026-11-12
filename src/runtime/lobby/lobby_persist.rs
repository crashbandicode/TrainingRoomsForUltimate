//! The waiting room's Training partners and CPU action persist (rc.6, `TASK-OPUS-rc6-lobby-cpu-settings.md` requirement
//! 1; design `artifacts/phase23/rc6-lobby-persist/DESIGN.md`). Switch only; the rule is `model::lobby_persist`.
//!
//! Two inline hooks, both inside the waiting room's practice panel code and both only in a local-wireless or Quickplay
//! waiting family:
//! * **Apply** (`lobby_persist.entry_applied`, `main+0x13ce1b4`): every waiting-room entry builds a fresh panel with the
//!   game's defaults (`FUN_013cdd00`); after its last default the saved pair is written into the panel and the game's
//!   own row handler (`FUN_01bc83a0`) is called with the same value, so it takes its equal-value path and sets the label.
//! * **Save** (`lobby_persist.setter_entry`, `main+0x1bc83b4`, after the row handler's prologue): the player's change is
//!   the row handler's `(row, index)`; a value that differs from the saved one is published and the writer thread
//!   stores it in `settings.json` (`lobby_practice_panel`).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use skyline::hooks::InlineCtx;

use crate::engine::seams::native_fields::{child, read, write_unchecked as write};
use crate::engine::stages::lobby_stage_cycle::family;
use crate::model::hook_stats::{HookId, HookScope};
use crate::model::lobby_persist::{chosen, restore, Apply, Change, Saved, BEHAVIOR_ROW, MAX_CHOICES, PARTNERS_ROW};

/// The panel's fields (13.0.5; `lobby_persist.entry_defaults`, `lobby_persist.setter_entry`).
const PARTNERS: usize = 0x420;
const PARTNERS_INDEX: usize = 0x424;
const BEHAVIOR: usize = 0x428;
const CHOICES_BEGIN: usize = 0x438;
const CHOICES_END: usize = 0x440;
/// The scene holder's controller and the controller's panel (`[[holder+0x130]+0x80]`).
const HOLDER_CONTROLLER: usize = 0x130;
const CONTROLLER_PANEL: usize = 0x80;
/// The row handler (`FUN_01bc83a0`): `(ctx, int *row, u32 *index)`, panel = `[ctx+8]`.
const ROW_HANDLER: usize = 0x1bc83a0;

/// The rows compared before anything hooks.
const ROWS: &[&str] = &[
    "lobby_persist.entry_applied",
    "lobby_persist.entry_defaults",
    "lobby_persist.setter_entry",
    "lobby_persist.row_handler",
    "lobby_persist.behavior_label",
];

static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The saved pair (`Saved::pack`); boot publishes what the file holds.
static SAVED: AtomicU32 = AtomicU32::new(Saved::NONE.pack());
/// Our own row handler call is running: the save hook leaves it alone.
static APPLYING: AtomicBool = AtomicBool::new(false);
/// The writer thread stores the published pair.
static STORE_DUE: AtomicBool = AtomicBool::new(false);

/// Boot: the pair `settings.json` holds.
pub(crate) fn publish(saved: Saved) {
    SAVED.store(saved.pack(), Ordering::Release);
}

fn published() -> Saved {
    Saved::unpack(SAVED.load(Ordering::Acquire))
}

/// The writer thread: the pair to store, if a change is due.
pub(crate) fn store_due() -> Option<Saved> {
    STORE_DUE.load(Ordering::Acquire).then(published)
}

/// The writer thread stored `pair` (`None`: nothing was due): cleared, and due again when the hook saved a newer pair
/// meanwhile (`model::lobby_persist::clear_stored`).
pub(crate) fn clear_store(pair: Option<Saved>) {
    crate::model::lobby_persist::clear_stored(&STORE_DUE, &SAVED, pair, || {});
}

/// A local-wireless or Quickplay waiting room (`lobby_stage_cycle::family`, the outer mode the panel is built for).
fn waiting() -> bool {
    crate::runtime::lobby::game_mode::current().is_some_and(|mode| family(mode.mode).is_some())
}

/// The panel's offered partner values, when they are the panel's.
struct Choices {
    values: [u32; MAX_CHOICES],
    len: usize,
}

impl Choices {
    fn as_slice(&self) -> &[u32] {
        &self.values[..self.len]
    }
}

/// # Safety
/// `panel` is the live panel of the current callback.
unsafe fn choices(panel: usize) -> Option<Choices> {
    let (begin, end) = (read::<usize>(panel, CHOICES_BEGIN)?, read::<usize>(panel, CHOICES_END)?);
    if begin == 0 || end < begin || (end - begin) % 4 != 0 || (end - begin) / 4 > MAX_CHOICES {
        return None;
    }
    let mut out = Choices { values: [0; MAX_CHOICES], len: (end - begin) / 4 };
    for i in 0..out.len {
        out.values[i] = read::<u32>(begin, i * 4)?;
    }
    Some(out)
}

/// # Safety
/// `holder` is the waiting scene's holder in its entry callback.
unsafe fn panel_of(holder: usize) -> Option<usize> {
    child(child(holder, HOLDER_CONTROLLER)?, CONTROLLER_PANEL)
}

/// Write one saved value into the panel, then let the game's own row handler set the label from it: the value is already
/// stored, so the handler takes its equal-value path (no slide, no sound).
///
/// # Safety
/// `panel` is the live panel of the entry callback; the row rows were compared at init.
unsafe fn apply(panel: usize, step: Apply) {
    APPLYING.store(true, Ordering::Release);
    match step.row {
        PARTNERS_ROW => {
            write::<u32>(panel, PARTNERS, step.value);
            write::<u32>(panel, PARTNERS_INDEX, step.index);
        }
        BEHAVIOR_ROW => write::<u32>(panel, BEHAVIOR, step.value),
        _ => {}
    }
    let ctx: [usize; 2] = [0, panel];
    let (row, index) = (step.row, step.index);
    let row_handler: unsafe extern "C" fn(usize, *const i32, *const u32) =
        core::mem::transmute(crate::runtime::game::text_base() + ROW_HANDLER);
    row_handler(ctx.as_ptr() as usize, &row, &index);
    APPLYING.store(false, Ordering::Release);
}

/// After the entry's last default (`0x13ce1b4`, `ldr w8,[x23,#0x150]`; `x23` = the scene holder).
#[skyline::hook(offset = 0x13ce1b4, inline)]
unsafe fn entry_applied(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::LobbyPersistEntry);
    if !waiting() {
        return;
    }
    let holder = ctx.registers[23].x() as usize;
    let Some(panel) = panel_of(holder) else { return };
    let Some(choices) = choices(panel) else { return };
    for step in restore(published(), choices.as_slice()).into_iter().flatten() {
        apply(panel, step);
    }
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!(
        "[LobbyPersist] entry saved={:?} offered={:?} partners={:?} behavior={:?}",
        published(), choices.as_slice(), read::<u32>(panel, PARTNERS), read::<u32>(panel, BEHAVIOR)));
}

/// The row handler after its prologue (`0x1bc83b4`, `ldr w8,[x1]`): `x0` = ctx, `x1` = the row, `x2` = the index.
#[skyline::hook(offset = 0x1bc83b4, inline)]
unsafe fn setter_entry(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::LobbyPersistSave);
    if APPLYING.load(Ordering::Acquire) || !waiting() {
        return;
    }
    let row = read::<i32>(ctx.registers[1].x() as usize, 0);
    let index = read::<u32>(ctx.registers[2].x() as usize, 0);
    let (Some(row), Some(index)) = (row, index) else { return };
    let Some(panel) = child(ctx.registers[0].x() as usize, 8) else { return };
    let Some(choices) = choices(panel) else { return };
    if let Some(change) = chosen(row, index, choices.as_slice()) {
        save(change);
    }
}

/// Publish the player's change and ask the writer to store it (only when it differs from the saved pair).
fn save(change: Change) {
    let next = published().with(change);
    if next == published() {
        return;
    }
    SAVED.store(next.pack(), Ordering::Release);
    // The writer thread polls this and stores the live settings with the pair (`settings_json_for_write`).
    STORE_DUE.store(true, Ordering::Release);
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[LobbyPersist] saved {next:?} change={change:?}"));
}

/// Once from `lobby::init`: compare every row, then hook.
pub(crate) fn init() {
    if crate::runtime::seams::seam_install::gate("lobby_persist", ROWS, true) {
        skyline::install_hooks!(entry_applied, setter_entry);
        INSTALLED.store(true, Ordering::Release);
    }
    crate::engine::compat::logging::diagnostic(format_args!(
        "[TrainingRooms] lobby_persist installed={} saved={:?}",
        INSTALLED.load(Ordering::Acquire),
        published()
    ));
}
