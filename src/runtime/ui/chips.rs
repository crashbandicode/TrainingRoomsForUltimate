//! The DI and SDI direction chips from the game's own effect textures (stream u4b; SPEC-core-07 section 4.6, MENU-DRAFT *Direction
//! chips*). When the DI or SDI grid shows, the page asks `effect_texture` for the visual's pictures, builds one UI texture over the
//! texture view the game already holds (`texture::load_resident`) and binds it to the pointer pane of every chip but Neutral; the view
//! (`view.rs`) then places and turns each pointer with `model::effect_chips::place`. When the grid closes, changes or the menu hides,
//! the pointers are unbound and the texture retired (released a few frames later); when the layout goes away (`lose_layout`, the
//! layout's teardown signal) the texture is released at once without touching a pane. Any failure leaves the clean room's arrows.
//!
//! The decisions are `model::effect_chips::plan` (host-tested); this module carries them out on the UI job thread. Its state is a
//! handful of atomic words (the stream/sync rule: no lock on a game-thread path), written and read only by the layout tick and the
//! view it draws from.

use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};

use crate::model::effect_chips::{self as m, Candidate, Held, LayerWord, Step};
use crate::model::effect_texture::Visual;
use crate::model::frame::{ChipVisual, Frame, View};
use crate::model::panes as p;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::ui::effect_texture::{self, MAX_PICTURES};
use crate::runtime::ui::texture::{self, Texture};
use crate::runtime::ui::view::Panes;

/// The chip whose pointer stays the ring (Neutral, `options::DIRECTIONS` index 8).
pub const NEUTRAL: usize = 8;

/// The visual the last drawn frame wants (`effect_chips::visual_code`).
static WANTED: AtomicU8 = AtomicU8::new(0);
/// What the page holds (`effect_chips::held_code`).
static HELD: AtomicU8 = AtomicU8::new(0);
/// The bound texture's shared-pointer words (0, 0 when none).
static TEXTURE_HOLDER: AtomicUsize = AtomicUsize::new(0);
static TEXTURE_CONTROL: AtomicUsize = AtomicUsize::new(0);
/// What the view draws on the pointers (`effect_chips::pack_layer`; 0 when nothing is bound).
static LAYER: AtomicU64 = AtomicU64::new(0);

fn log(args: core::fmt::Arguments<'_>) {
    crate::model::hook_stats::per_load(|| diagnostic(args));
}

/// The layer the pointers carry, if any (one atomic load).
pub fn layer() -> Option<LayerWord> {
    m::unpack_layer(LAYER.load(Ordering::Acquire))
}

/// The frame the menu drew: the DI / SDI grid wants its visual, anything else none.
pub fn note_frame(frame: &Frame) {
    let wanted = if frame.view == View::Chips { frame.chip_visual } else { ChipVisual::None };
    WANTED.store(m::visual_code(wanted), Ordering::Release);
}

/// The menu is not showing: no visual is wanted.
pub fn hide() {
    WANTED.store(m::visual_code(ChipVisual::None), Ordering::Release);
}

/// One UI frame: carry out the page's step. Returns whether the chips changed (the caller redraws).
///
/// # Safety
/// On the UI job thread, inside the Training layout's tick; `panes` are the resolved panes of that layout.
pub unsafe fn run(panes: &Panes) -> bool {
    let wanted = m::visual_of(WANTED.load(Ordering::Acquire));
    match m::plan(m::held_of(HELD.load(Ordering::Acquire)), wanted) {
        Step::Keep => false,
        Step::Drop => {
            drop_layer(Some(panes));
            true
        }
        Step::Request(v) => {
            request(panes, v);
            true
        }
        Step::DropThenRequest(v) => {
            drop_layer(Some(panes));
            request(panes, v);
            true
        }
    }
}

unsafe fn request(panes: &Panes, visual: ChipVisual) {
    let effect = match visual {
        ChipVisual::Di => Visual::Di,
        ChipVisual::Sdi => Visual::Sdi,
        ChipVisual::None => return,
    };
    HELD.store(m::held_code(Held::Failed(visual)), Ordering::Release);
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
    let (holder, control) = built.to_words();
    TEXTURE_HOLDER.store(holder, Ordering::Release);
    TEXTURE_CONTROL.store(control, Ordering::Release);
    LAYER.store(
        m::pack_layer(LayerWord { visual, width: picture.texture.width, height: picture.texture.height, tint: picture.tint }),
        Ordering::Release,
    );
    HELD.store(m::held_code(Held::Bound(visual)), Ordering::Release);
    log(format_args!(
        "[Chips] visual={visual:?} bound={bound} texture={}x{} tint={:?} allocations={}",
        picture.texture.width,
        picture.texture.height,
        picture.tint,
        crate::runtime::alloc::hook_allocations()
    ));
}

/// Unbind the pointers (when the panes still exist) and retire the texture; with no panes (the layout is gone) release it at once.
unsafe fn drop_layer(panes: Option<&Panes>) {
    LAYER.store(0, Ordering::Release);
    HELD.store(m::held_code(Held::Nothing), Ordering::Release);
    let (holder, control) = (TEXTURE_HOLDER.swap(0, Ordering::AcqRel), TEXTURE_CONTROL.swap(0, Ordering::AcqRel));
    let Some(t) = Texture::from_words(holder, control) else { return };
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
    drop_layer(None);
}
