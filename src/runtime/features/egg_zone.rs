//! Runtime zone setup, P1 wizard and overlay. Native results are copied
//! on a primary fighter pass. Only stage coordinates survive the pass.

use std::fmt::Write;
use std::sync::atomic::{AtomicU32, Ordering};

use hos_lock::Mutex;

use crate::model::chords::{field, Sample};
use crate::model::edge_juggle::perfect::{self, Outline, Point, Segment};
use crate::model::egg_zone::{blast_rect, stroke_layers, Outcome, Side, Stage, Step, Wizard, Zone};
use crate::model::eggs::View;
use crate::model::input_delay::Owners;
use crate::model::notify::DEFAULT_COLOR;
use crate::model::storage::FixedPath;
use crate::runtime::{game::natives, published, scene, ui::worker};

use super::{edge_juggle, mashout::FloorRay, notices, states, tech_surfaces};

const NOTICE: u8 = 0x35;
const INSET_BACKING: [u8; 4] = [10, 15, 27, 235];

struct State {
    epoch: u32,
    stage: Option<Stage>,
    wizard: Option<Wizard>,
}

static STATE: Mutex<State> = Mutex::new(State {
    epoch: 0,
    stage: None,
    wizard: None,
});
static EPOCH: AtomicU32 = AtomicU32::new(1);
static REQUEST: AtomicU32 = AtomicU32::new(0);
static LIVE: AtomicU32 = AtomicU32::new(0);
static BACK: AtomicU32 = AtomicU32::new(0);

pub fn request() {
    REQUEST.store(epoch(), Ordering::Release);
}

fn epoch() -> u32 {
    EPOCH.load(Ordering::Acquire)
}

pub fn wizard_active() -> bool {
    LIVE.load(Ordering::Acquire) == epoch()
}

pub fn take_menu_request() -> bool {
    BACK.swap(0, Ordering::AcqRel) == epoch()
}

fn close() {
    if LIVE.swap(0, Ordering::AcqRel) != 0 {
        notices::clear(NOTICE);
    }
}

pub fn menu_opened() {
    REQUEST.store(0, Ordering::Release);
    close();
}

pub fn reset() {
    // Invalidate a read already in flight, without taking the geometry lock.
    EPOCH.fetch_add(1, Ordering::AcqRel);
    menu_opened();
    BACK.store(0, Ordering::Release);
}

pub fn with_stage<T>(read: impl FnOnce(&Stage) -> T) -> Option<T> {
    let state = STATE.try_lock()?;
    (state.epoch == epoch())
        .then_some(state.stage.as_ref())
        .flatten()
        .map(read)
}

pub fn preferences(stage: &Stage) -> Option<[u8; 4]> {
    worker::egg_zone(stage.id())
}

/// Guarded world AABB for both Eggsercise practice scenes.
pub unsafe fn blast() -> Option<[Point; 2]> {
    let range = if scene::is_native_training() {
        tech_surfaces::training_blast_bounds()
    } else if scene::lobby_term() {
        tech_surfaces::blast_bounds()
    } else {
        None
    }?;
    let [left, right, top, bottom] = range;
    (range.into_iter().all(f32::is_finite) && left < right && bottom < top)
        .then(|| blast_rect(range))
}

/// Call with the current primary P1 accessor. Native calls run outside the
/// state lock so fighter, input and HUD callbacks can read the last snapshot.
pub unsafe fn prepare(p1: usize) {
    let generation = epoch();
    let id = natives::stage_id();
    let Some(mut state) = STATE.try_lock() else {
        return;
    };
    if state.epoch == generation && state.stage.as_ref().is_some_and(|s| s.id() == id) {
        return;
    }
    // A same-match stage change retires both the geometry and the editor.
    if state.stage.is_some() {
        close();
    }
    state.epoch = generation;
    state.stage = None;
    state.wizard = None;
    drop(state);

    if id < 0 || !scene::is_training() || states::is_loading() {
        return;
    }
    let Some([[left, bottom], [right, top]]) = blast() else {
        return;
    };
    let Some(camera) = natives::camera_limit_range() else {
        return;
    };
    let range = [left, right, top, bottom];
    let Some((mid, limits)) = perfect::outline_probe(camera, range) else {
        return;
    };
    let Some(ray) = FloorRay::new(p1) else { return };
    let Some((ledges, _)) = edge_juggle::stage_floor(&ray, camera) else {
        return;
    };
    let Some((outline, _)) = tech_surfaces::training_outline(&ray, mid, limits) else {
        return;
    };
    let Some(stage) = Stage::new(id, range, ledges, outline) else {
        return;
    };

    if let Some(mut state) = STATE.try_lock() {
        if generation == epoch() && state.epoch == generation && state.stage.is_none() {
            state.stage = Some(stage);
        }
    }
}

/// A request arrives from the menu after it has committed the selected rows.
pub unsafe fn frame(fighter: usize) {
    let generation = epoch();
    if REQUEST.load(Ordering::Acquire) != generation && !wizard_active() {
        return;
    }
    let owners = published::input_owners();
    if !scene::is_training() || states::is_loading() || owners.contains(Owners::REPLAY_EDITOR) {
        close();
        REQUEST.store(0, Ordering::Release);
        return;
    }
    if owners.covers_hud() {
        close();
        return;
    }
    if fighter == 0 {
        return;
    }
    let Some(p1) = natives::module_accessor(0).filter(|&a| a == natives::fighter_accessor(fighter))
    else {
        return;
    };
    prepare(p1);
    let Some(mut state) = STATE.try_lock() else {
        return;
    };
    if generation != epoch() || state.epoch != generation {
        return;
    }
    let Some(stage) = state.stage.as_ref() else {
        return;
    };
    let saved = preferences(stage);
    if REQUEST
        .compare_exchange(generation, 0, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        let wizard = Wizard::new(saved);
        let zone = wizard.zone(stage);
        state.wizard = Some(wizard);
        LIVE.store(generation, Ordering::Release);
        prompt(wizard.step(), zone);
    }
}

fn prompt(step: Step, zone: Zone) {
    let mut preview = FixedPath::<128>::new();
    let body = match step {
        Step::Top => "1/5 Top · Stick up/down · A next · B cancel · X defaults",
        Step::Bottom => "2/5 Bottom · Stick up/down · A next · B back · X defaults",
        Step::Inner => "3/5 Inner edges · Stick left/right · A next · B back · X defaults",
        Step::Outer => "4/5 Outer edges · Stick left/right · A next · B back · X defaults",
        Step::Preview => {
            let [bottom, top] = zone.vertical();
            let [outer_left, inner_left] = zone.horizontal(Side::Left);
            let [inner_right, outer_right] = zone.horizontal(Side::Right);
            let _ = write!(preview,
                "5/5 Preview · Y {bottom:.0}…{top:.0} · L {outer_left:.0}…{inner_left:.0} · R {inner_right:.0}…{outer_right:.0} · A/+ apply · B back · X defaults");
            preview.as_str()
        }
    };
    notices::show_now(NOTICE, "Off-stage zone", body, u16::MAX, DEFAULT_COLOR);
}

pub unsafe fn poll(controller: usize, sample: Sample) -> bool {
    if !wizard_active() {
        return false;
    }
    let generation = epoch();
    let Some(mut state) = STATE.try_lock() else {
        return true;
    };
    if state.epoch != generation || !wizard_active() {
        return false;
    }
    let State {
        stage: Some(stage),
        wizard: Some(wizard),
        ..
    } = &mut *state
    else {
        return false;
    };
    let was = wizard.step();
    let stick = [0, 1].map(|i| ((controller + field::STICKS[i]) as *const f32).read_volatile());
    let outcome = wizard.input(stage, stick, sample.pressed);
    match outcome {
        Outcome::Cancel => {
            state.wizard = None;
            close();
            BACK.store(generation, Ordering::Release);
        }
        Outcome::Apply(bins) if worker::request_egg_zone(stage.id(), bins) => {
            state.wizard = None;
            close();
            notices::post("Off-stage zone", "Applied", 90, DEFAULT_COLOR);
        }
        _ if wizard.step() != was => prompt(wizard.step(), wizard.zone(stage)),
        _ => {}
    }
    true
}

struct Display {
    zone: Zone,
    inward: [f32; 2],
    step: Step,
    hatching: [Option<Segment>; 10],
    overview: Option<Outline>,
}

fn display(map: impl Fn(Point) -> Point) -> Option<Display> {
    if !wizard_active() {
        return None;
    }
    let state = STATE.try_lock()?;
    if state.epoch != epoch() || !wizard_active() {
        return None;
    }
    let stage = state.stage.as_ref()?;
    let wizard = state.wizard?;
    let zone = wizard.zone(stage);
    let [bottom, top] = zone.vertical();
    let [left, _] = zone.horizontal(Side::Left);
    let [_, right] = zone.horizontal(Side::Right);
    let [lo, hi] = [[left, bottom], [right, top]].map(map);
    let overview = (lo[0] < -940.0 || lo[1] < -500.0 || hi[0] > 940.0 || hi[1] > 500.0)
        .then(|| *stage.outline());
    let hatching = std::array::from_fn(|i| {
        let side = Side::BOTH[i / 5];
        let [[left, low], [right, high]] = zone.rect(side);
        let [outer, inner] = if side == Side::Left {
            [left, right]
        } else {
            [right, left]
        };
        let y = low + (high - low) * ((i % 5 + 1) as f32 / 6.0);
        let start = [outer, y];
        // Only shade the air directly connected outward at this height.
        if !stage.contains(&zone, side, start) {
            return None;
        }
        if stage.allows_move(&zone, side, start, [inner, y]) {
            return Some([start, [inner, y]]);
        }
        let (mut end, mut hidden) = (outer, inner);
        for _ in 0..6 {
            let mid = end.midpoint(hidden);
            if stage.allows_move(&zone, side, start, [mid, y]) {
                end = mid;
            } else {
                hidden = mid;
            }
        }
        Some([start, [end, y]])
    });
    Some(Display {
        zone,
        inward: stage.inner_limit(),
        step: wizard.step(),
        hatching,
        overview,
    })
}

/// Paint on the shared orb bands, using the live world projection plus a
/// compact overview when a bar is outside the camera.
pub fn draw(view: View, mut line: impl FnMut(Segment, f32, [u8; 4])) {
    let View(scale, origin, ..) = view;
    let map = |p: Point| [p[0] * scale[0] + origin[0], p[1] * scale[1] + origin[1]];
    let Some(display) = display(map) else { return };
    display.guides(map, &mut |segment, width, color| {
        for (width, color) in stroke_layers(width, color) {
            line(segment, width, color);
        }
    });
    if let Some(outline) = &display.overview {
        let [bottom, top] = display.zone.vertical();
        let [left, _] = display.zone.horizontal(Side::Left);
        let [_, right] = display.zone.horizontal(Side::Right);
        let origin = [left.midpoint(right), bottom.midpoint(top)];
        let span = outline.segments().iter().flatten().fold(
            (right - left).max(top - bottom),
            |span, p| span.max(2.0 * (p[0] - origin[0]).abs()).max(2.0 * (p[1] - origin[1]).abs()),
        );
        let scale = 218.0 / span;
        // One dark pane backs every inset stroke. With 28 guides per view and
        // up to 64 contour segments, both views use at most 149 of 172 bands.
        line([[681.0, 347.0], [919.0, 347.0]], 238.0, INSET_BACKING);
        let overview = |p: Point| {
            [
                800.0 + (p[0] - origin[0]) * scale,
                347.0 + (p[1] - origin[1]) * scale,
            ]
        };
        // Show enough of the traced contour to locate the valid strips.
        for &segment in outline.segments() {
            line(segment.map(overview), 3.0, [240, 240, 255, 230]);
        }
        display.guides(overview, &mut line);
    }
}

impl Display {
    fn guides(&self, map: impl Fn(Point) -> Point, line: &mut impl FnMut(Segment, f32, [u8; 4])) {
        for segment in self.hatching.iter().flatten() {
            line(segment.map(&map), 5.0, [112, 204, 255, 222]);
        }
        let [bottom, top] = self.zone.vertical();
        let [left, inner_left] = self.zone.horizontal(Side::Left);
        let [inner_right, right] = self.zone.horizontal(Side::Right);
        let mut guide = |a, b, k, color: [u8; 3]| {
            let active = self.step == k || self.step == Step::Preview;
            let [r, g, b0] = color;
            line(
                [map(a), map(b)],
                if active { 5.0 } else { 3.0 },
                [r, g, b0, if active { 250 } else { 190 }],
            );
        };
        for (y, step) in [(top, Step::Top), (bottom, Step::Bottom)] {
            guide([left, y], [right, y], step, [66, 142, 255]);
        }
        for x in [inner_left, inner_right] {
            guide([x, bottom], [x, top], Step::Inner, [255, 221, 77]);
        }
        for x in [left, right] {
            guide([x, bottom], [x, top], Step::Outer, [206, 132, 255]);
        }
        for x in self.inward {
            let slice = (top - bottom) / 11.0;
            for i in (0..11).step_by(2) {
                let y = bottom + i as f32 * slice;
                line([map([x, y]), map([x, y + slice])], 2.0, [255, 221, 77, 180]);
            }
        }
    }
}
