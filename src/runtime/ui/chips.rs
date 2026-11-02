//! The DI and SDI direction chips from the game's own effect textures (stream u4b; SPEC-core-07 section 4.6, MENU-DRAFT *Direction
//! chips*). When the DI or SDI grid shows, the page asks `effect_texture` for the visual's pictures, builds one UI texture over the
//! texture view the game already holds (`texture::load_resident`) and binds it to the pointer pane of every chip but Neutral; the view
//! (`view.rs`) then places and turns each pointer with `model::effect_chips::place`. When the grid closes, changes or the menu hides,
//! the pointers are unbound and the texture retired (released a few frames later); when the layout goes away (`lose_layout`, the
//! layout's teardown signal) the texture is released at once without touching a pane. Any failure leaves the clean room's arrows.
//!
//! The decisions are `model::effect_chips::plan` (host-tested); this module carries them out on the UI job thread.

use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

use crate::model::effect_chips::{self as m, Candidate, Held, Step};
use crate::model::effect_texture::Visual;
use crate::model::frame::{ChipVisual, Frame, View};
use crate::model::panes as p;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::ui::effect_texture::{self, MAX_PICTURES};
use crate::runtime::ui::texture::{self, Texture};
use crate::runtime::ui::view::Panes;

/// The chip whose pointer stays the ring (Neutral, `options::DIRECTIONS` index 8).
pub const NEUTRAL: usize = 8;

/// What the view needs to draw a bound chip: the visual, its tint and the texture's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layer {
    pub visual: ChipVisual,
    pub tint: [u8; 4],
    pub width: u32,
    pub height: u32,
}

struct State {
    wanted: ChipVisual,
    held: Held,
    texture: Option<Texture>,
    layer: Option<Layer>,
}

static STATE: Mutex<State> = Mutex::new(State { wanted: ChipVisual::None, held: Held::Nothing, texture: None, layer: None });
/// Whether the pointers carry the effect now (read by the view without the lock order mattering).
static BOUND: AtomicBool = AtomicBool::new(false);

fn log(args: core::fmt::Arguments<'_>) {
    crate::model::hook_stats::per_load(|| diagnostic(args));
}

/// The layer the pointers carry, if any.
pub fn layer() -> Option<Layer> {
    if !BOUND.load(Ordering::Acquire) {
        return None;
    }
    STATE.try_lock().and_then(|s| s.layer)
}

/// The frame the menu drew: the DI / SDI grid wants its visual, anything else none.
pub fn note_frame(frame: &Frame) {
    if let Some(mut s) = STATE.try_lock() {
        s.wanted = if frame.view == View::Chips { frame.chip_visual } else { ChipVisual::None };
    }
}

/// The menu is not showing: no visual is wanted.
pub fn hide() {
    if let Some(mut s) = STATE.try_lock() {
        s.wanted = ChipVisual::None;
    }
}

/// One UI frame: carry out the page's step. Returns whether the chips changed (the caller redraws).
///
/// # Safety
/// On the UI job thread, inside the Training layout's tick; `panes` are the resolved panes of that layout.
pub unsafe fn run(panes: &Panes) -> bool {
    let Some(mut s) = STATE.try_lock() else { return false };
    let wanted = s.wanted;
    match m::plan(s.held, wanted) {
        Step::Keep => false,
        Step::Drop => {
            drop_layer(&mut s, Some(panes));
            true
        }
        Step::Request(v) => {
            request(&mut s, panes, v);
            true
        }
        Step::DropThenRequest(v) => {
            drop_layer(&mut s, Some(panes));
            request(&mut s, panes, v);
            true
        }
    }
}

unsafe fn request(s: &mut State, panes: &Panes, visual: ChipVisual) {
    let effect = match visual {
        ChipVisual::Di => Visual::Di,
        ChipVisual::Sdi => Visual::Sdi,
        ChipVisual::None => return,
    };
    s.held = Held::Failed(visual);
    // Diag: what the walk finds, once per grid open (the resident files and the visual's emitters and textures).
    #[cfg(feature = "diag")]
    {
        effect_texture::survey(|a| log(a));
        effect_texture::describe(effect, |a| log(a));
    }
    let pictures = match effect_texture::pictures(effect) {
        Ok(found) => found,
        Err(why) => {
            log(format_args!("[Chips] visual={visual:?} absent={why:?} (arrows kept)"));
            return;
        }
    };
    let mut candidates = [Candidate { sampler: usize::MAX, width: 0, height: 0 }; MAX_PICTURES];
    for (slot, picture) in candidates.iter_mut().zip(pictures.pictures.iter()) {
        if let Some(pic) = picture {
            *slot = Candidate { sampler: pic.sampler, width: pic.texture.width, height: pic.texture.height };
        }
    }
    let Some(picture) = m::pick_layer(visual, &candidates[..pictures.count]).and_then(|i| pictures.pictures[i]) else {
        log(format_args!("[Chips] visual={visual:?} no layer among {} pictures (arrows kept)", pictures.count));
        return;
    };
    let Some(built) = effect_texture::texture_of(&picture) else {
        log(format_args!("[Chips] visual={visual:?} texture not built (arrows kept)"));
        return;
    };
    let mut bound = 0;
    for i in (0..p::CHIPS).filter(|i| *i != NEUTRAL) {
        let pane = panes.all[p::IDX_CHIP_NEEDLE + i];
        if !pane.is_null() && texture::bind(pane, &built) {
            bound += 1;
        }
    }
    if bound == 0 {
        texture::release(built);
        log(format_args!("[Chips] visual={visual:?} nothing bound (arrows kept)"));
        return;
    }
    s.texture = Some(built);
    s.layer = Some(Layer { visual, tint: picture.tint, width: picture.texture.width, height: picture.texture.height });
    s.held = Held::Bound(visual);
    BOUND.store(true, Ordering::Release);
    log(format_args!(
        "[Chips] visual={visual:?} bound={bound} texture={}x{} tint={:?} allocations={}",
        picture.texture.width,
        picture.texture.height,
        picture.tint,
        crate::runtime::alloc::hook_allocations()
    ));
}

/// Unbind the pointers (when the panes still exist) and retire the texture; with no panes (the layout is gone) release it at once.
unsafe fn drop_layer(s: &mut State, panes: Option<&Panes>) {
    BOUND.store(false, Ordering::Release);
    s.layer = None;
    s.held = Held::Nothing;
    let Some(t) = s.texture.take() else { return };
    match panes {
        Some(panes) => {
            for i in (0..p::CHIPS).filter(|i| *i != NEUTRAL) {
                let pane = panes.all[p::IDX_CHIP_NEEDLE + i];
                if !pane.is_null() {
                    texture::unbind(pane);
                }
            }
            if !texture::retire(t) {
                texture::release(t);
            }
            log(format_args!("[Chips] dropped (retired) waiting={}", texture::retired_waiting()));
        }
        None => {
            texture::release(t);
            log(format_args!("[Chips] dropped with the layout (released)"));
        }
    }
}

/// The layout is gone or about to be registered again (panes destroyed or unresolved): drop the texture without touching a pane, so
/// nothing dangles; the next grid open requests it again.
///
/// # Safety
/// On the UI job thread.
pub unsafe fn lose_layout() {
    if let Some(mut s) = STATE.try_lock() {
        drop_layer(&mut s, None);
    }
}
