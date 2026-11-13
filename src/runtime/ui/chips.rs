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

use crate::model::effect_chips::{self as m, Candidate, Held, LayerWord, Step, LAYERS};
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
/// Each layer's bound texture, as shared-pointer words (0, 0 when none).
static TEXTURE_HOLDER: [AtomicUsize; LAYERS] = [const { AtomicUsize::new(0) }; LAYERS];
static TEXTURE_CONTROL: [AtomicUsize; LAYERS] = [const { AtomicUsize::new(0) }; LAYERS];
/// What the view draws on each layer's pointers (`effect_chips::pack_layer`; 0 when nothing is bound).
static LAYER: [AtomicU64; LAYERS] = [const { AtomicU64::new(0) }; LAYERS];

fn log(args: core::fmt::Arguments<'_>) {
    crate::model::hook_stats::per_load(|| diagnostic(args));
}

/// What layer `k`'s pointers carry, if anything (one atomic load).
pub fn layer(k: usize) -> Option<LayerWord> {
    m::unpack_layer(LAYER[k].load(Ordering::Acquire))
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
    let mut layers = 0;
    for (k, pick) in m::pick_layers(visual, &candidates[..pictures.count]).into_iter().enumerate() {
        let Some(picture) = pick.and_then(|i| pictures.pictures[i]) else { continue };
        let Some(built) = effect_texture::texture_of(&picture) else {
            log(format_args!("[Chips] visual={visual:?} layer={k} texture not built"));
            continue;
        };
        let mut bound = 0;
        for i in (0..p::CHIPS).filter(|i| *i != NEUTRAL) {
            let pane = panes.all[p::IDX_CHIP_NEEDLE + k * p::CHIPS + i];
            if !pane.is_null() && texture::bind(pane, &built) {
                bound += 1;
            }
        }
        if bound == 0 {
            texture::release(built);
            log(format_args!("[Chips] visual={visual:?} layer={k} nothing bound"));
            continue;
        }
        let (holder, control) = built.to_words();
        TEXTURE_HOLDER[k].store(holder, Ordering::Release);
        TEXTURE_CONTROL[k].store(control, Ordering::Release);
        let (width, height, tint) = (picture.texture.width, picture.texture.height, picture.tint);
        LAYER[k].store(m::pack_layer(LayerWord { visual, width, height, tint }), Ordering::Release);
        layers += 1;
        log(format_args!(
            "[Chips] visual={visual:?} layer={k} bound={bound} texture={width}x{height} tint={tint:?} allocations={}",
            crate::runtime::alloc::hook_allocations()
        ));
    }
    if layers == 0 {
        log(format_args!("[Chips] visual={visual:?} no layer among {} pictures (arrows kept)", pictures.count));
        return;
    }
    HELD.store(m::held_code(Held::Bound(visual)), Ordering::Release);
}

/// Unbind the pointers when the panes still exist; layout loss retains the same GPU retirement grace.
unsafe fn drop_layer(panes: Option<&Panes>) {
    HELD.store(m::held_code(Held::Nothing), Ordering::Release);
    let mut dropped = false;
    for k in 0..LAYERS {
        LAYER[k].store(0, Ordering::Release);
        let (holder, control) = (TEXTURE_HOLDER[k].swap(0, Ordering::AcqRel), TEXTURE_CONTROL[k].swap(0, Ordering::AcqRel));
        let Some(t) = Texture::from_words(holder, control) else { continue };
        for i in (0..p::CHIPS).filter(|i| panes.is_some() && *i != NEUTRAL) {
            let pane = panes.map_or(core::ptr::null_mut(), |panes| panes.all[p::IDX_CHIP_NEEDLE + k * p::CHIPS + i]);
            if !pane.is_null() {
                texture::unbind(pane);
            }
        }
        if !texture::retire(t) {
            texture::release(t);
        }
        dropped = true;
    }
    if dropped && panes.is_some() {
        log(format_args!("[Chips] dropped (retired) waiting={}", texture::retired_waiting()));
    } else if dropped {
        log(format_args!("[Chips] dropped with the layout (retired)"));
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
