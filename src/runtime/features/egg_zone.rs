//! Runtime zone setup, P1 wizard and overlay. Native results are copied
//! on a primary fighter pass. Only stage coordinates survive the pass.

use std::fmt::Write;
use std::sync::atomic::{AtomicU32, Ordering};

use hos_lock::Mutex;

use crate::model::chords::{field, Sample};
use crate::model::edge_juggle::perfect::{self, Outline, Point, Segment};
use crate::model::egg_zone::{blast_rect, crop_window, CropRect, CropView, Outcome, Side, Stage, Step, Wizard, Zone, CROP_RECTS};
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

/// The crop window `draw` computed in this pass and the layout it was drawn for, taken by that layout's menu tick
/// (`ui::tick`) right after the draw. The tick reads no camera itself: the projection stays behind the hitbox unit's draw.
static CROP: Mutex<Option<(usize, [Option<CropRect>; CROP_RECTS])>> = Mutex::new(None);

pub fn request() {
    REQUEST.store(epoch(), Ordering::Release);
}

fn epoch() -> u32 {
    EPOCH.load(Ordering::Acquire)
}

pub fn wizard_active() -> bool {
    LIVE.load(Ordering::Acquire) == epoch()
}

/// This pass's crop window for `layout`, once; `None` when the editor's guide was not drawn for it in this pass.
pub fn crop(layout: usize) -> Option<[Option<CropRect>; CROP_RECTS]> {
    let mut slot = CROP.try_lock()?;
    let (owner, rects) = (*slot)?;
    (owner == layout).then(|| {
        *slot = None;
        rects
    })
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

#[derive(PartialEq, Eq)]
pub enum Geometry {
    Pending,
    Ready,
    Unavailable,
}

/// Call with the current primary P1 accessor. Native calls run outside the
/// state lock so fighter, input and HUD callbacks can read the last snapshot.
pub unsafe fn prepare(p1: usize) -> Geometry {
    let generation = epoch();
    let id = natives::stage_id();
    let Some(mut state) = STATE.try_lock() else {
        return Geometry::Pending;
    };
    if state.epoch == generation && state.stage.as_ref().is_some_and(|s| s.id() == id) {
        return Geometry::Ready;
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
        return Geometry::Pending;
    }
    let stage = read_stage(p1, id);
    let Some(mut state) = STATE.try_lock() else {
        return Geometry::Pending;
    };
    if generation != epoch() || state.epoch != generation {
        return Geometry::Pending;
    }
    if state.stage.is_none() {
        state.stage = stage;
    }
    match state.stage.as_ref() {
        Some(stage) if stage.id() == id => Geometry::Ready,
        Some(_) => Geometry::Pending,
        None => Geometry::Unavailable,
    }
}

unsafe fn read_stage(p1: usize, id: i32) -> Option<Stage> {
    let [[left, bottom], [right, top]] = blast()?;
    let camera = natives::camera_limit_range()?;
    let range = [left, right, top, bottom];
    let (mid, limits) = perfect::outline_probe(camera, range)?;
    let ray = FloorRay::new(p1)?;
    let (ledges, _) = edge_juggle::stage_floor(&ray, camera)?;
    let (outline, _) = tech_surfaces::training_outline(&ray, mid, limits)?;
    Stage::new(id, range, ledges, outline)
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
    if prepare(p1) == Geometry::Unavailable {
        if REQUEST
            .compare_exchange(generation, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let _ = notices::post_required(
                "Off-stage zone",
                "No usable Off stage zone on this stage.",
                120,
                DEFAULT_COLOR,
            );
        }
        return;
    }
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
            let _ = write!(preview,
                "5/5 Preview · Y {bottom:.0}…{top:.0}");
            for (side, label) in [(Side::Left, "L"), (Side::Right, "R")] {
                if let Some([lo, hi]) = zone.horizontal(side) {
                    let _ = write!(preview, " · {label} {lo:.0}…{hi:.0}");
                }
            }
            let _ = write!(preview, " · A/+ apply · B back · X defaults");
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
    span: [f32; 2],
    inward: [f32; 2],
    step: Step,
    samples: [Option<Point>; 10],
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
    let [left, right] = zone.horizontal_span()?;
    let [lo, hi] = [[left, bottom], [right, top]].map(map);
    let overview = (lo[0] < -940.0 || lo[1] < -500.0 || hi[0] > 940.0 || hi[1] > 500.0)
        .then(|| *stage.outline());
    Some(Display {
        zone,
        span: [left, right],
        inward: stage.inner_limit(),
        step: wizard.step(),
        samples: stage.samples(&zone),
        overview,
    })
}

/// Leave this pass's crop window for `layout` in the `CROP` slot and return the spots of the sample targets. On the
/// shared orb bands it paints only the overview inset's backing and contour, when a bar is outside the camera.
pub fn draw(layout: usize, view: View, mut line: impl FnMut(Segment, f32, [u8; 4])) -> [Option<Point>; 10] {
    let View(scale, origin, ..) = view;
    let map = |p: Point| [p[0] * scale[0] + origin[0], p[1] * scale[1] + origin[1]];
    let Some(display) = display(map) else { return [None; 10] };
    let crop_view = |project: &dyn Fn(Point) -> Point| CropView {
        vertical: display.zone.vertical().map(|y| project([0.0, y])[1]),
        sides: Side::BOTH.map(|side| display.zone.horizontal(side).map(|span| span.map(|x| project([x, 0.0])[0]))),
        limits: display.inward.map(|x| project([x, 0.0])[0]),
    };
    let inset = display.overview.as_ref().map(|outline| {
        let [bottom, top] = display.zone.vertical();
        let [left, right] = display.span;
        let origin = [left.midpoint(right), bottom.midpoint(top)];
        let span = outline.segments().iter().flatten().fold(
            (right - left).max(top - bottom),
            |span, p| span.max(2.0 * (p[0] - origin[0]).abs()).max(2.0 * (p[1] - origin[1]).abs()),
        );
        let scale = 218.0 / span;
        // One backing and up to 64 contour segments use at most 65 of the 172 bands.
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
        crop_view(&overview)
    });
    if let Some(mut crop) = CROP.try_lock() {
        *crop = Some((layout, crop_window(crop_view(&map), inset, display.step)));
    }
    display.samples
}
