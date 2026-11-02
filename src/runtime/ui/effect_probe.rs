//! Diag only (stream u4b): the Eden proof of `effect_texture`. A while after the Training layout resolves it logs a survey of the effect
//! manager's resident files, describes the DI (`SYS_VECTOR`) and SDI (`SYS_HITSTOP_SPARK`) effects as the helper reads them, asks the game
//! for a UI texture over each emitter texture, binds every one to a menu tile pane with the emitter's colour, forces the menu container
//! visible so the tiles can be photographed (DI in the upper row, SDI in the lower), holds them, then unbinds and retires them and logs the
//! release. The cycle repeats so screenshots can be taken at any time and a repeated build and release shows no growth. Never in the
//! shipped NRO; the DI / SDI direction chips call the same `effect_texture` functions when their page opens and closes.

use parking_lot::Mutex;

use crate::model::effect_texture::Visual;
use crate::model::panes as p;
use crate::runtime::alloc::hook_allocations;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::ui::effect_texture::{self, Picture, MAX_PICTURES};
use crate::runtime::ui::pane;
use crate::runtime::ui::texture::{self, Texture};
use crate::runtime::ui::view::Panes;

const START_TICK: u32 = 300;
const HOLD_TICKS: u32 = 900;
const IDLE_TICKS: u32 = 240;
const VISUALS: [Visual; 2] = [Visual::Di, Visual::Sdi];
const SLOTS: usize = VISUALS.len() * MAX_PICTURES;

struct State {
    phase: u8,
    ticks: u32,
    phase_at: u32,
    cycle: u32,
    textures: [Option<Texture>; SLOTS],
    pictures: [Option<Picture>; SLOTS],
    bound: [bool; SLOTS],
}

static STATE: Mutex<State> = Mutex::new(State { phase: 0, ticks: 0, phase_at: 0, cycle: 0, textures: [None; SLOTS], pictures: [None; SLOTS], bound: [false; SLOTS] });

/// Slot `i`'s place: DI in the upper row, SDI in the lower, left to right.
fn place(index: usize) -> (f32, f32) {
    let (row, column) = (index / MAX_PICTURES, index % MAX_PICTURES);
    (-420.0 + 120.0 * column as f32, 110.0 - 220.0 * row as f32)
}

/// The tile pane a slot uses.
unsafe fn tile(panes: &Panes, index: usize) -> *mut skyline::nn::ui2d::Pane {
    panes.all[p::IDX_TILE + index]
}

/// Called once per frame at the end of the Training layout's tick, on the UI job thread.
pub unsafe fn tick(panes: &Panes) {
    let Some(mut s) = STATE.try_lock() else { return };
    s.ticks += 1;
    texture::tick();
    let now = s.ticks;
    match s.phase {
        0 if now - s.phase_at >= START_TICK => {
            if s.cycle == 0 {
                let found = effect_texture::install();
                diagnostic(format_args!("[EffTex] probe guards={found:?}"));
            }
            diagnostic(format_args!("[EffTex] request cycle={} usable={} allocations={}", s.cycle, effect_texture::usable(), hook_allocations()));
            if s.cycle == 0 {
                effect_texture::survey(|a| diagnostic(a));
                for visual in VISUALS {
                    effect_texture::describe(visual, |a| diagnostic(a));
                }
            }
            for (v, visual) in VISUALS.iter().enumerate() {
                match effect_texture::pictures(*visual) {
                    Ok(found) => {
                        for (i, picture) in found.pictures.iter().enumerate() {
                            let Some(picture) = picture else { continue };
                            let slot = v * MAX_PICTURES + i;
                            let built = effect_texture::texture_of(picture);
                            diagnostic(format_args!(
                                "[EffTex] texture visual={visual:?} picture={i} {}x{} slot={:#x} built={}",
                                picture.texture.width,
                                picture.texture.height,
                                picture.texture.slot,
                                built.is_some()
                            ));
                            s.pictures[slot] = Some(*picture);
                            s.textures[slot] = built;
                        }
                    }
                    Err(why) => diagnostic(format_args!("[EffTex] pictures visual={visual:?} absent={why:?}")),
                }
            }
            for slot in 0..SLOTS {
                let (Some(t), Some(picture)) = (s.textures[slot], s.pictures[slot]) else { continue };
                let target = tile(panes, slot);
                if target.is_null() {
                    continue;
                }
                let ready = t.poll();
                let bound = texture::bind(target, &t);
                s.bound[slot] = bound;
                diagnostic(format_args!("[EffTex] bind slot={slot} ready={ready:?} bound={bound} tint={:?} facts={:?}", picture.tint, t.facts()));
            }
            diagnostic(format_args!("[EffTex] shown cycle={} bound={:?} allocations={}", s.cycle, s.bound, hook_allocations()));
            s.phase = 1;
            s.phase_at = now;
        }
        1 if now - s.phase_at >= HOLD_TICKS => {
            for slot in 0..SLOTS {
                if s.bound[slot] {
                    let target = tile(panes, slot);
                    texture::unbind(target);
                    pane::set_visible(target, false);
                    s.bound[slot] = false;
                }
                if let Some(t) = s.textures[slot].take() {
                    let queued = texture::retire(t);
                    diagnostic(format_args!("[EffTex] retire slot={slot} queued={queued}"));
                }
                s.pictures[slot] = None;
            }
            pane::set_visible(panes.container, false);
            s.phase = 2;
            s.phase_at = now;
        }
        2 if texture::retired_waiting() == 0 => {
            diagnostic(format_args!("[EffTex] released cycle={} after={} allocations={}", s.cycle, now - s.phase_at, hook_allocations()));
            s.phase = 3;
            s.phase_at = now;
        }
        3 if now - s.phase_at >= IDLE_TICKS => {
            s.cycle += 1;
            s.phase = 0;
            s.phase_at = now;
        }
        _ => {}
    }
    // While the tiles are bound the container and the tiles are forced on (the menu is closed, so its own draw leaves them hidden).
    if s.phase == 1 {
        pane::set_visible(panes.container, true);
        pane::set_alpha(panes.container, 255);
        for slot in 0..SLOTS {
            if let (true, Some(picture)) = (s.bound[slot], s.pictures[slot]) {
                let target = tile(panes, slot);
                let (x, y) = place(slot);
                let width = 110.0;
                let height = (width * picture.texture.height as f32 / picture.texture.width as f32).clamp(24.0, 200.0);
                pane::set_visible(target, true);
                pane::set_alpha(target, 255);
                pane::set_pos(target, x, y);
                pane::set_width(target, width);
                pane::set_height(target, height);
                pane::set_rect_color(target, picture.tint);
            }
        }
    }
}
