//! Stage geometry reads shared by runtime features.
use crate::model::edge_juggle::perfect;
use crate::model::mashout::entry;
use crate::model::sites_mashout::{self, FLOOR_RAY, FLOOR_RAY_IMPL};
use crate::model::tech::discovery::{decode, far_end};
use crate::model::tech::surface::{Kind, Line, Source, Surfaces};
use crate::model::tech::Vec2;
use crate::runtime::game;

// This 13.0.5 ray reads and writes all 16 bytes of its Vector2f references.
#[repr(C, align(16))]
pub(super) struct RayPoint(pub(super) [f32; 4]);

type RayCast = unsafe extern "C" fn(usize, *const RayPoint, *const RayPoint, *mut RayPoint, bool) -> usize;

/// The game's floor ray. The one place its pointer is made; each caller passes its own admission first.
pub(super) unsafe fn native_ray(base: usize) -> RayCast { core::mem::transmute(base + FLOOR_RAY) }

/// The one invocation of the game's floor ray. The caller has passed its own admission and owns the hit buffer.
pub(super) unsafe fn cast(
    ray: RayCast, accessor: usize, origin: [f32; 2], delta: [f32; 2], platforms: bool, hit: &mut RayPoint,
) -> usize {
    ray(accessor, &RayPoint([origin[0], origin[1], 0., 0.]), &RayPoint([delta[0], delta[1], 0., 0.]), hit, platforms)
}

pub(super) struct FloorRay {
    accessor: usize,
    cast: RayCast,
}

impl FloorRay {
    /// Validate the exact borrowed-line and endpoint reader contract before any ray.
    pub(super) unsafe fn new(p1: usize) -> Option<Self> {
        if !sites_mashout::SITES.iter().filter(|site| sites_mashout::placement_floor(site))
            .all(|site| game::granted(site.offset) && game::read_window(site.offset) == site.guard)
        {
            return None;
        }
        let base = game::text_base();
        let ground = ((p1 + 0x58) as *const usize).read();
        if ground == 0 {
            return None;
        }
        let vtable = (ground as *const usize).read();
        if (vtable != base + 0x4f87560 && vtable != base + 0x4f6d8b0)
            || ((vtable + 0x668) as *const usize).read() != base + FLOOR_RAY_IMPL
        {
            return None;
        }
        let cast = native_ray(base);
        Some(Self { accessor: p1, cast })
    }

    /// The caller must consume the borrowed line on this game-thread pass.
    pub(super) unsafe fn borrowed_line(
        &self, x: f32, [top, bottom]: [f32; 2], platforms: bool,
    ) -> Option<usize> {
        if ![x, top, bottom].into_iter().all(f32::is_finite) || top <= bottom {
            return None;
        }
        let mut hit = RayPoint([0.; 4]);
        let line = cast(self.cast, self.accessor, [x, top], [0., bottom - top], platforms, &mut hit);
        (line != 0).then_some(line)
    }

    /// The native result is borrowed only until the next query. Copy its current geometry.
    pub(super) unsafe fn floor(&self, x: f32, limits: [f32; 2], platforms: bool) -> Option<entry::PlacementFloor> {
        let line = self.borrowed_line(x, limits, platforms)?;
        let flags = ((line + 0xc4) as *const u32).read();
        let normal_y = ((line + 0xa4) as *const f32).read();
        if !normal_y.is_finite() {
            return None;
        }
        if !crate::model::stage_geometry::is_placement_floor(flags, normal_y, platforms) {
            return None;
        }
        let vertices = ((line + 0x88) as *const [usize; 2]).read();
        if vertices.contains(&0) {
            return None;
        }
        let ends = vertices.map(|ptr| ((ptr + 0x10) as *const [f32; 2]).read());
        let stationary = ends == vertices.map(|ptr| ((ptr + 0x20) as *const [f32; 2]).read());
        Some(entry::PlacementFloor { ends, stationary, flags })
    }
}

/// The main floor under the camera's middle and its two ledges, by the placement ray.
pub(super) unsafe fn stage_floor(ray: &FloorRay, camera: [f32; 4]) -> Option<(perfect::Segment, perfect::Point)> {
    crate::model::stage_geometry::main_floor(|x, limits, platforms| ray.floor(x, limits, platforms), camera)
}

use crate::model::stage_geometry::{is_placement_floor, Met, Point, Probe, Unread};

impl FloorRay {
    /// The first line of any kind from `from` straight down or up to the height `to`, copied as world geometry.
    unsafe fn first(&self, [x, y]: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread> {
        if ![x, y, to].into_iter().all(f32::is_finite) || to == y {
            return Err(Unread);
        }
        let mut hit = RayPoint([0.; 4]);
        let line = cast(self.cast, self.accessor, [x, y], [0., to - y], platforms, &mut hit);
        if line == 0 {
            return Ok(None);
        }
        let flags = ((line + 0xc4) as *const u32).read();
        let normal_y = ((line + 0xa4) as *const f32).read();
        let vertices = ((line + 0x88) as *const [usize; 2]).read();
        if !normal_y.is_finite() || vertices.contains(&0) {
            return Err(Unread);
        }
        let ends = vertices.map(|ptr| ((ptr + 0x10) as *const [f32; 2]).read());
        Ok(Some(Met { ends, stand: is_placement_floor(flags, normal_y, true), through: flags & 0x2000 != 0 }))
    }
}

/// The stage for this pass of Player 1's frame, read by the placement ray. Never kept across frames.
pub(super) struct RayProbe(FloorRay);
impl RayProbe {
    pub(super) unsafe fn new(p1: usize) -> Option<Self> { FloorRay::new(p1).map(Self) }
}
impl Probe for RayProbe {
    fn first(&mut self, from: Point, to: f32, platforms: bool) -> Result<Option<Met>, Unread> {
        // SAFETY: built on this pass from the live Player 1 (`new`'s contract).
        unsafe { self.0.first(from, to, platforms) }
    }
}

struct FrameOutlineReader;
impl FrameOutlineReader {
    fn frame_read_line(&mut self, id: usize) -> Option<Line> {
        // The id is a borrowed line from the guarded no-cull ray or its
        // checked next-line call. This reader is confined to one native pass.
        let aligned = |address: usize| address >= 0x10000 && address & 7 == 0 && address >> 47 == 0;
        if !aligned(id) {
            return None;
        }
        unsafe {
            let owner = ((id + 0x118) as *const usize).read();
            if !aligned(owner) || ((owner + 8) as *const i64).read() == -1
                || ((id + 0x100) as *const u8).read() == 0
            {
                return None;
            }
            let vertices = ((id + 0x88) as *const [usize; 2]).read();
            if !vertices.into_iter().all(aligned) {
                return None;
            }
            let ends = vertices.map(|ptr| {
                let [x, y] = ((ptr + 0x10) as *const [f32; 2]).read();
                Vec2 { x, y }
            });
            let [x, y] = ((id + 0xa0) as *const [f32; 2]).read();
            decode(id, ends, Vec2 { x, y }, ((id + 0xc5) as *const u8).read())
        }
    }

    fn frame_next_line(&mut self, line: Line, end: usize) -> Option<(usize, usize)> {
        let shared = *line.ends.get(end)?;
        if self.frame_read_line(line.id)? != line {
            return None;
        }
        unsafe {
            const NEXT: usize = 0x15cbc60;
            if !game::granted(NEXT) {
                return None;
            }
            let call: extern "C" fn(usize, *const RayPoint) -> usize =
                std::mem::transmute(game::text_base() + NEXT);
            let id = call(line.id, &RayPoint([shared.x, shared.y, 0.0, 0.0]));
            let next = self.frame_read_line(id)?;
            far_end(shared, next).map(|end| (id, end))
        }
    }
}

impl Source for FrameOutlineReader {
    fn read(&mut self, id: usize) -> Option<Line> {
        self.frame_read_line(id)
    }

    fn next(&mut self, line: Line, end: usize) -> Option<(usize, usize)> {
        self.frame_next_line(line, end)
    }
}

/// One closed outline walked in the current native frame, into the caller's `walk`.
unsafe fn closed_walk(ray: &FloorRay, x: f32, limits: [f32; 2], walk: &mut Surfaces) -> Option<()> {
    use crate::model::tech::surface::{Walk, MAX_LINES};
    if !super::tech_surfaces::guarded() || !game::granted(0x15cbc60) {
        return None;
    }
    let seed = ray.borrowed_line(x, limits, false)?;
    let mut reader = FrameOutlineReader;
    let line = reader.read(seed)?;
    if line.kind() != Some(Kind::Floor) || line.passable {
        return None;
    }
    walk.begin(seed, 0);
    for _ in 0..MAX_LINES {
        match walk.step(&mut reader) {
            Walk::More => {}
            Walk::Closed => return Some(()),
            _ => return None,
        }
    }
    None
}

/// Copy one closed outline in the current native Training frame. The returned
/// geometry has no raw line identities or borrowed native pointers.
pub(super) unsafe fn training_outline(
    ray: &FloorRay, x: f32, limits: [f32; 2],
) -> Option<(crate::model::edge_juggle::perfect::Outline, Option<[[f32; 2]; 2]>)> {
    let mut walk = Surfaces::EMPTY;
    closed_walk(ray, x, limits, &mut walk)?;
    Some(crate::model::stage_geometry::outline_of(&walk))
}

/// The closed outline of the solid under a column, line by line as the rule sees it. It stores nothing.
pub(super) unsafe fn solid(ray: &FloorRay, x: f32, limits: [f32; 2], each: impl FnMut(Met)) -> Option<()> {
    let mut walk = Surfaces::EMPTY;
    closed_walk(ray, x, limits, &mut walk)?;
    walk.lines().map(crate::model::stage_geometry::met).for_each(each);
    Some(())
}

#[cfg(feature = "diag")]
static PROBE_LAST: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
#[cfg(feature = "diag")]
static PROBE_TICK: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// Diag only: after a Training reset the probe logs again where Player 1 then stands.
#[cfg(feature = "diag")]
pub(super) fn diag_probe_rearm() {
    PROBE_LAST.store(0, core::sync::atomic::Ordering::Release);
}

/// Diag only (tr-primary-2, 2026-11-25 17:15:18Z): shows on a game how the ray reads "a solid above". In native
/// Training, whenever Player 1 stands on another floor run than the one last logged (looked at every 30 frames),
/// and again after a Training reset: the run, one upward cast and `classify` at three points taken from it, and
/// one cast started inside the solid.
#[cfg(feature = "diag")]
pub(super) unsafe fn diag_probe(fighter: usize) {
    use crate::model::stage_geometry::{classify, floor_run};
    use crate::model::sites_pkg2 as sites;
    use crate::runtime::diag_log::diagnostic;
    use crate::runtime::game::natives;
    use core::sync::atomic::Ordering;
    if fighter == 0 || !crate::runtime::scene::is_native_training()
        || PROBE_TICK.fetch_add(1, Ordering::Relaxed) % 30 != 0
    {
        return;
    }
    if ![sites::POS_X_OFFSET, sites::POS_Y_OFFSET, sites::SITUATION_KIND_OFFSET, sites::STAGE_ID_CALL_OFFSET,
        crate::model::sites_pkg4::FIGHTER_MANAGER_GET_ENTRY_OFFSET].into_iter().all(game::granted)
    {
        return;
    }
    let p1 = natives::fighter_accessor(fighter);
    if p1 == 0 || natives::module_accessor(0) != Some(p1) || !natives::is_grounded(p1) {
        return;
    }
    let (Some(blast), Some(mut stage)) = (super::tech_surfaces::training_blast_bounds(), RayProbe::new(p1)) else {
        return;
    };
    let at = [natives::pos_x(p1), natives::pos_y(p1)];
    let Ok(Some(run)) = floor_run(&mut stage, blast, at) else { return };
    let [left, right] = run.ends;
    let key = [left[0], left[1], right[0], right[1]]
        .into_iter()
        .fold(0x9e37_79b9_u64, |key, value| key.rotate_left(17) ^ u64::from(value.to_bits())) | 1;
    if PROBE_LAST.swap(key, Ordering::AcqRel) == key {
        return;
    }
    let [_, _, top, bottom] = blast;
    let middle = [left[0].midpoint(right[0]), left[1].midpoint(right[1])];
    diagnostic(format_args!(
        "[StageGeometry] probe stage={} p1=({:.3},{:.3}) run=({:.3},{:.3})..({:.3},{:.3}) ledges={:?}",
        natives::stage_id(), at[0], at[1], left[0], left[1], right[0], right[1], run.ledges
    ));
    for (name, sample) in [
        ("under", [middle[0], middle[1].midpoint(bottom + 1.0)]),
        ("beside", [left[0] - 1.0, left[1] - 1.0]),
        ("over", [middle[0], middle[1] + 20.0]),
    ] {
        let up = stage.first(sample, top, false);
        let class = classify(&mut stage, blast, sample);
        diagnostic(format_args!(
            "[StageGeometry] probe sample={name} at=({:.3},{:.3}) up={up:?} class={class:?}",
            sample[0], sample[1]
        ));
    }
    let back = stage.first([middle[0], middle[1] - 0.5], bottom, false);
    diagnostic(format_args!("[StageGeometry] probe back={back:?}"));
}
