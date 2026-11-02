//! Diag only (stream u4): the Eden proof of `texture`. About two seconds after the Training layout resolves it asks the game for the
//! UI textures of a few stage files (one of them absent from the resource table), polls them each frame, binds every ready one to a
//! menu tile pane, forces the menu container visible so the tiles can be photographed, holds them, then unbinds and retires them and
//! logs the release. Never in the shipped NRO; pkg5's stage grid calls the same `texture` functions when its page opens and closes.

use hos_lock::Mutex;

use crate::model::panes as p;
use crate::runtime::alloc::hook_allocations;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::ui::texture::{self, Readiness, Texture};
use crate::runtime::ui::view::Panes;
use crate::runtime::ui::pane;

/// Stock file, a file the fixture mod replaces, a patch-stage file, another stock file, and one the game does not have.
const PATHS: [&str; 5] = [
    "ui/replace/stage/stage_1/stage_1_battlefield.bntx",
    "ui/replace/stage/stage_1/stage_1_end.bntx",
    "ui/replace_patch/stage/stage_1/stage_1_battlefields.bntx",
    "ui/replace/stage/stage_1/stage_1_zelda_hyrule.bntx",
    "ui/replace/stage/stage_1/stage_1_not_a_stage.bntx",
];
const START_TICK: u32 = 120;
const GIVE_UP_TICKS: u32 = 900;
const HOLD_TICKS: u32 = 900;
/// Frames between one cycle (request, bind, hold, unbind, release) and the next: the proof repeats so the screenshots can be taken at
/// any time and a repeated load and release shows no growth.
const IDLE_TICKS: u32 = 240;

struct State {
    phase: u8,
    ticks: u32,
    phase_at: u32,
    cycle: u32,
    textures: [Option<Texture>; PATHS.len()],
    bound: [bool; PATHS.len()],
}

static STATE: Mutex<State> = Mutex::new(State { phase: 0, ticks: 0, phase_at: 0, cycle: 0, textures: [None; PATHS.len()], bound: [false; PATHS.len()] });

/// Tile `i`'s place in the row of the proof: 200 x 167 (the stage texture's 108 x 90 aspect), 225 apart, centred.
fn place(index: usize) -> (f32, f32) {
    (-450.0 + 225.0 * index as f32, 0.0)
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
                let _ = texture::install();
            }
            diagnostic(format_args!("[Texture] request cycle={} usable={} allocations={}", s.cycle, texture::usable(), hook_allocations()));
            for (i, path) in PATHS.iter().enumerate() {
                let hash = texture::path_hash(path);
                let loaded = texture::load(hash);
                diagnostic(format_args!("[Texture] request idx={i} path={path} hash={hash:#x} loaded={}", loaded.is_some()));
                s.textures[i] = loaded;
            }
            s.phase = 1;
            s.phase_at = now;
        }
        1 => {
            let mut pending = 0;
            for i in 0..PATHS.len() {
                let Some(t) = s.textures[i] else { continue };
                if s.bound[i] {
                    continue;
                }
                match t.poll() {
                    Readiness::Ready => {
                        let tile = panes.all[p::IDX_TILE + i];
                        let bound = texture::bind(tile, &t);
                        s.bound[i] = bound;
                        diagnostic(format_args!("[Texture] ready idx={i} after={} bound={bound} facts={:?}", now - s.phase_at, t.facts()));
                    }
                    Readiness::Loading => pending += 1,
                }
            }
            if pending == 0 || now - s.phase_at >= GIVE_UP_TICKS {
                for i in 0..PATHS.len() {
                    if let (Some(t), false) = (s.textures[i], s.bound[i]) {
                        diagnostic(format_args!("[Texture] never ready idx={i} facts={:?}", t.facts()));
                    }
                }
                diagnostic(format_args!("[Texture] shown cycle={} bound={:?} allocations={}", s.cycle, s.bound, hook_allocations()));
                s.phase = 2;
                s.phase_at = now;
            }
        }
        2 if now - s.phase_at >= HOLD_TICKS => {
            for i in 0..PATHS.len() {
                let tile = panes.all[p::IDX_TILE + i];
                if s.bound[i] {
                    texture::unbind(tile);
                    pane::set_visible(tile, false);
                    s.bound[i] = false;
                }
                if let Some(t) = s.textures[i].take() {
                    let queued = texture::retire(t);
                    diagnostic(format_args!("[Texture] retire idx={i} queued={queued}"));
                }
            }
            pane::set_visible(panes.container, false);
            s.phase = 3;
            s.phase_at = now;
        }
        3 if texture::retired_waiting() == 0 => {
            diagnostic(format_args!("[Texture] released cycle={} after={} allocations={}", s.cycle, now - s.phase_at, hook_allocations()));
            s.phase = 4;
            s.phase_at = now;
        }
        4 if now - s.phase_at >= IDLE_TICKS => {
            s.cycle += 1;
            s.phase = 0;
            s.phase_at = now;
        }
        _ => {}
    }
    // While the tiles are bound the container and the tiles are forced on (the menu is closed, so its own draw leaves them hidden).
    if s.phase == 1 || s.phase == 2 {
        pane::set_visible(panes.container, true);
        pane::set_alpha(panes.container, 255);
        for i in 0..PATHS.len() {
            if s.bound[i] {
                let tile = panes.all[p::IDX_TILE + i];
                let (x, y) = place(i);
                pane::set_visible(tile, true);
                pane::set_alpha(tile, 255);
                pane::set_pos(tile, x, y);
                pane::set_width(tile, 200.0);
                pane::set_height(tile, 167.0);
                pane::set_rect_color(tile, [255, 255, 255, 255]);
            }
        }
    }
}
