//! The stage grid's thumbnails and the roster grids' fighter stock icons (SPEC-core-07 §4.5): when a
//! stage page opens, each tile asks the game for its stage's own UI texture
//! (`ui/replace[_patch]/stage/stage_1/stage_1_<name>.bntx`, by path hash, through the game's loader,
//! which is the layer ARCropolis hooks: a stage mod's picture is what loads); when a roster page opens,
//! each tile asks for the fighter's own stock icon (the native key builder's `picture=2`
//! stock-icon set `chara_2`, the 64×64 flat head icon, at the fighter's default costume — the same
//! stock-icon art the fork's menu showed, operator decision 2026-11-03 amended 2026-11-04). Each texture binds
//! to the tile's picture pane once it is ready, and is unbound and
//! retired when the page changes or the menu closes — except a texture the new page wants at the same tile
//! (`facts::carry`): a role switch re-shows the same files, and re-requesting a hash whose previous texture
//! is still being released stalls the game's loader. A tile whose file is missing, refused or too slow
//! keeps its text tile; nothing is substituted.
//!
//! Runs on the UI job thread inside the layout tick (`tick::tick`), where `texture` requires its calls. Never allocates
//! (fixed slots, a stack path buffer). A layout destroyed or re-registered while textures are bound drops them without
//! touching the panes (`lose_layout`); textures still observe the same GPU retirement grace.

use std::sync::atomic::{AtomicBool, Ordering};

use hos_lock::Mutex;

use crate::model::frame::Frame;
use crate::model::panes as p;
use crate::model::stage_textures;
use crate::model::ui_texture as facts;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::ui::texture::{self, Readiness, Texture};
use crate::runtime::ui::view::Panes;

/// Frames a requested texture may stay "loading" before its tile gives up and keeps the text (10 s at 60 fps).
pub const GIVE_UP_FRAMES: u32 = 600;

#[derive(Clone, Copy)]
enum Slot {
    Idle,
    Loading { texture: Texture, hash: u64, frames: u32 },
    Bound { texture: Texture, hash: u64 },
    /// No file, no texture or too slow: the text tile stays.
    Failed,
}

/// The hash of the file a slot holds, `None` when it holds nothing.
fn held(slot: &Slot) -> Option<u64> {
    match slot {
        Slot::Loading { hash, .. } | Slot::Bound { hash, .. } => Some(*hash),
        Slot::Idle | Slot::Failed => None,
    }
}

struct State {
    /// The tile numbers of the page showing (0 = All, n = catalog stage n - 1).
    want: [u16; p::TILES],
    count: usize,
    /// A stage grid is on screen.
    active: bool,
    roster: bool,
    /// The page signature the slots were requested for (`u32::MAX` = none).
    requested: u32,
    sig: u32,
    slots: [Slot; p::TILES],
}

static STATE: Mutex<State> = Mutex::new(State {
    want: [u16::MAX; p::TILES],
    count: 0,
    active: false,
    roster: false,
    requested: u32::MAX,
    sig: 0,
    slots: [Slot::Idle; p::TILES],
});

/// One flag per pane: the roster has more than 64 tiles.
static BOUND: [AtomicBool; p::TILES] = [const { AtomicBool::new(false) }; p::TILES];

/// One log line (the page-level events are once per page, counted as per-load, not steady-state, allocations).
fn log(args: core::fmt::Arguments<'_>) {
    crate::model::hook_stats::per_load(|| diagnostic(args));
}

/// Whether tile `i` shows a picture now.
pub fn is_bound(i: usize) -> bool {
    BOUND.get(i).is_some_and(|flag| flag.load(Ordering::Acquire))
}

/// The frame the menu drew: a stage grid asks for its page's thumbnails, anything else for none.
pub fn note_frame(frame: &Frame) {
    let Some(mut s) = STATE.try_lock() else { return };
    s.active = frame.view == crate::model::frame::View::Tiles && (frame.tile_stage_grid || frame.tile_roster_grid);
    s.roster = frame.tile_roster_grid;
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

/// The hash of the file tile `i`'s page wants: a roster tile asks for the fighter's stock icon, a stage tile for the
/// stage's UI texture; `None` for a text tile. Called with the page now showing (`note_frame` ran first).
unsafe fn wanted(s: &State, i: usize) -> Option<u64> {
    if s.roster {
        (s.want[i] != u16::MAX)
            .then(|| crate::runtime::roster::character_roster::menu_stock_icon_hash(u32::from(s.want[i])))
            .flatten()
    } else {
        let mut buffer = [0u8; 96];
        stage_id(s.want[i])
            .and_then(|id| stage_textures::path_into(id, &mut buffer))
            .and_then(|len| core::str::from_utf8(&buffer[..len]).ok())
            .map(texture::path_hash)
    }
}

/// One UI frame: ask for what the page needs, poll what is loading, bind what is ready, release what is no longer wanted, then
/// let the helper retire released textures. Returns whether the set of pictures on screen changed (the caller redraws).
///
/// # Safety
/// On the UI job thread, inside the Training layout's tick; `panes` are the resolved panes of that layout.
pub unsafe fn run(panes: &Panes) -> bool {
    let _scope = crate::runtime::alloc::HookScope::enter_hook(crate::runtime::alloc::HookId::MenuThumbs);
    let mut changed = false;
    if let Some(mut s) = STATE.try_lock() {
        if !s.active || s.sig != s.requested {
            changed |= release_all(&mut s, Some(panes));
            if s.active && texture::usable() {
                let keyed = request(&mut s);
                let loading = s.slots.iter().take(s.count).filter(|slot| matches!(slot, Slot::Loading { .. })).count();
                log(format_args!("[Thumbs] page sig={:#x} tiles={} requested={loading} keyed={keyed}", s.sig, s.count));
            }
        }
        for i in 0..s.count {
            match s.slots[i] {
                Slot::Loading { texture: t, hash, frames } => {
                    let pane = panes.all[p::IDX_TILE + i];
                    match t.poll() {
                        Readiness::Ready if !pane.is_null() && texture::bind(pane, &t) => {
                            s.slots[i] = Slot::Bound { texture: t, hash };
                            BOUND[i].store(true, Ordering::Release);
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
                        Readiness::Loading => s.slots[i] = Slot::Loading { texture: t, hash, frames: frames + 1 },
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
    changed
}

/// Ask for the textures of the page's tiles. A slot that already holds the file its tile wants (kept across the page
/// change by `release_all`) is left alone and counted; the rest are requested.
unsafe fn request(s: &mut State) -> usize {
    s.requested = s.sig;
    let mut keyed = 0;
    for i in 0..s.count {
        if !matches!(s.slots[i], Slot::Idle | Slot::Failed) {
            keyed += 1;
            continue;
        }
        let slot = match wanted(s, i) {
            Some(hash) => {
                keyed += 1;
                texture::load(hash).map(|texture| Slot::Loading { texture, hash, frames: 0 })
            }
            None => None,
        };
        s.slots[i] = slot.unwrap_or(Slot::Failed);
    }
    keyed
}

/// Unbind what is bound (when the panes still exist) and retire every texture the slots hold — except one the page
/// now showing wants at the same tile (`facts::carry`): a role switch re-shows the same files, and a request for a
/// hash whose previous texture is still being released stalls the game's loader. True when a picture was on screen.
unsafe fn release_all(s: &mut State, panes: Option<&Panes>) -> bool {
    let mut shown = false;
    let keep = s.active && panes.is_some();
    for i in 0..p::TILES {
        let slot = core::mem::replace(&mut s.slots[i], Slot::Idle);
        let want = if keep { wanted(s, i) } else { None };
        match facts::carry(held(&slot), want) {
            facts::Carry::Keep => {
                s.slots[i] = slot;
                shown |= matches!(slot, Slot::Bound { .. });
            }
            facts::Carry::Request | facts::Carry::Idle => match slot {
                Slot::Bound { texture: t, hash: _ } => {
                    shown |= BOUND[i].swap(false, Ordering::AcqRel);
                    if let Some(panes) = panes {
                        let pane = panes.all[p::IDX_TILE + i];
                        if !pane.is_null() {
                            texture::unbind(pane);
                        }
                    }
                    if !texture::retire(t) {
                        texture::release(t);
                    }
                }
                Slot::Loading { texture: t, .. } => texture::release(t),
                Slot::Idle | Slot::Failed => {}
            },
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
