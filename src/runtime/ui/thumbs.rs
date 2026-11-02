//! The stage grid's thumbnails (SPEC-core-07 §4.5, decision "read at run time, never baked"): when a stage page opens, each
//! tile asks the game for its stage's own UI texture (`ui/replace[_patch]/stage/stage_1/stage_1_<name>.bntx`, by path hash,
//! through the game's loader, which is the layer ARCropolis hooks: a stage mod's picture is what loads), binds it to the tile's
//! picture pane once it is ready, and unbinds and retires it when the page changes or the menu closes. A tile whose file is
//! missing, refused or too slow keeps its text tile; nothing is substituted.
//!
//! Runs on the UI job thread inside the layout tick (`tick::tick`), where `texture` requires its calls. Never allocates
//! (fixed slots, a stack path buffer). A layout destroyed or re-registered while textures are bound drops them without
//! touching the panes (`lose_layout`): the panes are gone, the textures are released at once.

use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use crate::model::frame::Frame;
use crate::model::panes as p;
use crate::model::stage_textures;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::ui::texture::{self, Readiness, Texture};
use crate::runtime::ui::view::Panes;

/// Frames a requested texture may stay "loading" before its tile gives up and keeps the text (10 s at 60 fps).
pub const GIVE_UP_FRAMES: u32 = 600;

#[derive(Clone, Copy)]
enum Slot {
    Idle,
    Loading { texture: Texture, frames: u32 },
    Bound { texture: Texture },
    /// No file, no texture or too slow: the text tile stays.
    Failed,
}

struct State {
    /// The tile numbers of the page showing (0 = All, n = catalog stage n - 1).
    want: [u16; p::TILES],
    count: usize,
    /// A stage grid is on screen.
    active: bool,
    /// The page signature the slots were requested for (`u32::MAX` = none).
    requested: u32,
    sig: u32,
    slots: [Slot; p::TILES],
}

static STATE: Mutex<State> = Mutex::new(State {
    want: [u16::MAX; p::TILES],
    count: 0,
    active: false,
    requested: u32::MAX,
    sig: 0,
    slots: [Slot::Idle; p::TILES],
});

/// Bit `i` set: tile `i` shows its picture (the view reads it).
pub static BOUND: AtomicU64 = AtomicU64::new(0);

/// One log line (the page-level events are once per page, counted as per-load, not steady-state, allocations).
fn log(args: core::fmt::Arguments<'_>) {
    crate::model::hook_stats::per_load(|| diagnostic(args));
}

/// Whether tile `i` shows a picture now.
pub fn is_bound(i: usize) -> bool {
    BOUND.load(Ordering::Acquire) & (1 << i) != 0
}

/// The frame the menu drew: a stage grid asks for its page's thumbnails, anything else for none.
pub fn note_frame(frame: &Frame) {
    let Some(mut s) = STATE.try_lock() else { return };
    s.active = frame.view == crate::model::frame::View::Tiles && frame.tile_stage_grid;
    s.count = usize::from(frame.tile_count).min(p::TILES);
    s.sig = frame.tile_sig;
    s.want = frame.tile_n;
}

/// The menu is not showing: no thumbnails are wanted.
pub fn hide() {
    if let Some(mut s) = STATE.try_lock() {
        s.active = false;
    }
}

fn stage_id(n: u16) -> Option<u32> {
    let index = usize::from(n.checked_sub(1)?);
    crate::engine::stages::lobby_stages::STAGES.get(index).map(|(id, _)| *id)
}

/// One UI frame: ask for what the page needs, poll what is loading, bind what is ready, release what is no longer wanted, then
/// let the helper retire released textures. Returns whether the set of pictures on screen changed (the caller redraws).
///
/// # Safety
/// On the UI job thread, inside the Training layout's tick; `panes` are the resolved panes of that layout.
pub unsafe fn run(panes: &Panes) -> bool {
    let mut changed = false;
    if let Some(mut s) = STATE.try_lock() {
        if !s.active || s.sig != s.requested {
            changed |= release_all(&mut s, Some(panes));
            if s.active && texture::usable() {
                request(&mut s);
                let loading = s.slots.iter().take(s.count).filter(|slot| matches!(slot, Slot::Loading { .. })).count();
                log(format_args!("[Thumbs] page sig={:#x} tiles={} requested={loading}", s.sig, s.count));
            }
        }
        for i in 0..s.count {
            match s.slots[i] {
                Slot::Loading { texture: t, frames } => {
                    let pane = panes.all[p::IDX_TILE + i];
                    match t.poll() {
                        Readiness::Ready if !pane.is_null() && texture::bind(pane, &t) => {
                            s.slots[i] = Slot::Bound { texture: t };
                            BOUND.fetch_or(1 << i, Ordering::AcqRel);
                            changed = true;
                        }
                        Readiness::Ready => {
                            texture::release(t);
                            s.slots[i] = Slot::Failed;
                        }
                        Readiness::Loading if frames >= GIVE_UP_FRAMES => {
                            texture::release(t);
                            s.slots[i] = Slot::Failed;
                        }
                        Readiness::Loading => s.slots[i] = Slot::Loading { texture: t, frames: frames + 1 },
                    }
                }
                _ => {}
            }
        }
    }
    if changed {
        if let Some(s) = STATE.try_lock() {
            let bound = s.slots.iter().filter(|slot| matches!(slot, Slot::Bound { .. })).count();
            let loading = s.slots.iter().filter(|slot| matches!(slot, Slot::Loading { .. })).count();
            let failed = s.slots.iter().take(s.count).filter(|slot| matches!(slot, Slot::Failed)).count();
            log(format_args!("[Thumbs] bound={bound} loading={loading} failed={failed} retire_waiting={} allocations={}", texture::retired_waiting(), crate::runtime::alloc::hook_allocations()));
        }
    }
    texture::tick();
    changed
}

/// Ask for the textures of the page's tiles.
unsafe fn request(s: &mut State) {
    s.requested = s.sig;
    for i in 0..s.count {
        let mut buffer = [0u8; 96];
        let slot = stage_id(s.want[i])
            .and_then(|id| stage_textures::path_into(id, &mut buffer))
            .and_then(|len| core::str::from_utf8(&buffer[..len]).ok())
            .and_then(|path| texture::load(texture::path_hash(path)));
        s.slots[i] = match slot {
            Some(texture) => Slot::Loading { texture, frames: 0 },
            None => Slot::Failed,
        };
    }
}

/// Unbind what is bound (when the panes still exist) and retire every texture the slots hold. True when a picture was on screen.
unsafe fn release_all(s: &mut State, panes: Option<&Panes>) -> bool {
    let shown = BOUND.swap(0, Ordering::AcqRel) != 0;
    for i in 0..p::TILES {
        match core::mem::replace(&mut s.slots[i], Slot::Idle) {
            Slot::Bound { texture: t } => {
                if let Some(panes) = panes {
                    let pane = panes.all[p::IDX_TILE + i];
                    if !pane.is_null() {
                        texture::unbind(pane);
                    }
                    if !texture::retire(t) {
                        texture::release(t);
                    }
                } else {
                    texture::release(t);
                }
            }
            Slot::Loading { texture: t, .. } => texture::release(t),
            Slot::Idle | Slot::Failed => {}
        }
    }
    s.requested = u32::MAX;
    shown
}

/// The layout is gone or about to be registered again (panes destroyed or unresolved): drop every texture without touching a
/// pane, so nothing dangles.
///
/// # Safety
/// On the UI job thread.
pub unsafe fn lose_layout() {
    if let Some(mut s) = STATE.try_lock() {
        release_all(&mut s, None);
    }
}
