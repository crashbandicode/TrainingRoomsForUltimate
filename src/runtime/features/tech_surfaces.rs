//! Read-only native stage surfaces. Called only on the owned P1 game thread.
//! Field/call evidence: DESIGN §4 and TECH-UI-SURFACE-GUARDS.json (owned 13.0.5).
use super::stage_geometry::{native_ray, RayPoint};
use crate::model::guard::{guard_outcome, GuardOutcome, SeamKind};
use crate::model::sites_mashout::FLOOR_RAY;
use crate::model::sites_tech_surfaces::windows as guards;
use crate::model::tech::discovery::{decode, far_end, Bootstrap, Rays};
use crate::model::tech::surface::{damaging_material, Kind, Line, Source, Surfaces};
use crate::model::tech::Vec2;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::game::{self, natives};
use std::sync::OnceLock;
static VERIFIED: OnceLock<bool> = OnceLock::new();
static MATERIAL_VERIFIED: OnceLock<bool> = OnceLock::new();
static PASSIVE_TABLES: OnceLock<[Option<usize>; 2]> = OnceLock::new();
const GROUND_TABLES: [usize; 2] = [0x4f87560, 0x4f6d8b0];
const GROUND_GEOMETRY: [(usize, usize); 2] = [(0x4d8, 0x463c40), (0x668, 0x4658b0)];

#[repr(C)]
#[derive(Default)]
struct MemoryInfo {
    address: u64,
    size: u64,
    kind: u32,
    attributes: u32,
    permission: u32,
    ipc: u32,
    device: u32,
    padding: u32,
}
/// Same pinned Horizon QueryMemory ABI as custom_stages::readable.
/// Mapping does not confer lifetime: caller remains on the native game thread.
unsafe fn readable(address: usize, size: usize) -> bool {
    let Some(end) = address.checked_add(size) else {
        return false;
    };
    if address < 0x10000 || address & 3 != 0 || address >> 47 != 0 {
        return false;
    }
    let mut info = MemoryInfo::default();
    let result: u64;
    core::arch::asm!("svc 0x6", inout("x0") &mut info as *mut MemoryInfo as u64 => result,
        lateout("x1") _, in("x2") address as u64, clobber_abi("C"), options(nostack));
    result as u32 == 0
        && info.permission & 1 == 1
        && info.address <= address as u64
        && info
            .address
            .checked_add(info.size)
            .is_some_and(|e| end as u64 <= e)
}
unsafe fn pointer(address: usize) -> Option<usize> {
    (address & 7 == 0 && readable(address, 8)).then(|| (address as *const usize).read())
}
/// Verify immutable native code during initialization, before installing hooks.
pub(super) unsafe fn init() -> bool {
    *VERIFIED.get_or_init(|| {
        if !game::display_version().starts_with(b"13.0.5\0") {
            return false;
        }
        let base = game::text_base();
        if base == 0 {
            return false;
        }
        let mut good = true;
        for &(name, offset, called, expected) in guards::SITES {
            let actual = ((base + offset) as *const [u8; 64]).read_unaligned();
            let kind = if called {
                SeamKind::Called
            } else {
                SeamKind::Relied
            };
            let outcome = guard_outcome(&expected, &actual, kind);
            diagnostic(format_args!(
                "[TechPractice] surface_guard site={name} outcome={outcome:?}"
            ));
            good &= outcome.usable();
        }
        if good
            && *MATERIAL_VERIFIED.get_or_init(|| {
                guards::MATERIAL.iter().all(|&(name, offset, expected)| {
                    let actual = ((base + offset) as *const [u8; 64]).read_unaligned();
                    let outcome = guard_outcome(&expected, &actual, SeamKind::Relied);
                    diagnostic(format_args!(
                        "[TechPractice] material_guard site={name} outcome={outcome:?}"
                    ));
                    outcome == GuardOutcome::Stock
                })
            })
        {
            PASSIVE_TABLES.get_or_init(|| {
                GROUND_TABLES.map(|offset| {
                    let vptr = base + offset;
                    (ground_slots(base, vptr, &GROUND_GEOMETRY)
                        && ground_slots(base, vptr, &[(0x4a0, 0x4635b0), (0x478, 0x4628c0)]))
                    .then_some(vptr)
                })
            });
        }
        good
    })
}
pub(super) unsafe fn guarded() -> bool {
    VERIFIED.get().copied().unwrap_or(false)
}

unsafe fn point(line: usize, end: usize) -> Option<Vec2> {
    let vertex = pointer(line + [0x88, 0x90][end])?;
    if vertex & 7 != 0 || !readable(vertex, 0x18) {
        return None;
    }
    let [x, y] = ((vertex + 0x10) as *const [f32; 2]).read_unaligned();
    Some(Vec2 { x, y })
}

/// The reviewed real GroundModule tables share these exact slot implementations.
unsafe fn ground_slots(base: usize, vptr: usize, slots: &[(usize, usize)]) -> bool {
    slots
        .iter()
        .all(|&(slot, target)| pointer(vptr + slot) == Some(base + target))
}

unsafe fn ground_contract(accessor: usize) -> Option<usize> {
    let base = game::text_base();
    let vptr = pointer(pointer(accessor + 0x58)?)?;
    (GROUND_TABLES.contains(&vptr.wrapping_sub(base)) && ground_slots(base, vptr, &GROUND_GEOMETRY))
        .then_some(vptr)
}

/// Stock floor-touch eligibility includes the contact line's material.
/// Caller has revalidated the live owned fighter on this game-thread pass.
pub(super) unsafe fn passive_floor_touch(accessor: usize) -> Option<bool> {
    let tables = PASSIVE_TABLES.get()?;
    let module = ((accessor + 0x58) as *const usize).read();
    if module < 0x10000 || module & 7 != 0 || module >> 47 != 0 {
        return None;
    }
    let vptr = (module as *const usize).read();
    if tables.contains(&Some(vptr)) {
        natives::tech_passive_ground_touch(accessor, 8)
    } else {
        None
    }
}
/// Ephemeral reader: construct per P1 pass and never retain a live accessor.
/// Construction works while airborne; only initial seed acquisition needs ground.
pub struct NativeSource {
    accessor: usize,
    ground_vptr: usize,
    material_usable: Option<bool>,
}
impl NativeSource {
    /// Caller owns this P1 accessor for the current native game-thread pass.
    pub unsafe fn new(accessor: usize) -> Option<Self> {
        if crate::model::scene::scene(crate::runtime::scene::facts()).is_none()
            || !guarded()
            || accessor & 7 != 0
            || !readable(accessor, 0x60)
        {
            return None;
        }
        Some(Self {
            accessor,
            ground_vptr: ground_contract(accessor)?,
            material_usable: None,
        })
    }
    /// Use only after this source has validated the same live line in this pass.
    pub(super) unsafe fn damaging_floor(&mut self, line: Line) -> bool {
        let kind = line.kind();
        if kind != Some(Kind::Floor) {
            return false;
        }
        let vptr = self.ground_vptr;
        if !*self.material_usable.get_or_insert_with(|| {
            MATERIAL_VERIFIED.get().copied().unwrap_or(false)
                && pointer(vptr + 0x478) == Some(game::text_base() + 0x4628c0)
        }) {
            return false;
        }
        damaging_material(
            kind,
            ((line.id + 0xc6) as *const u8).read(),
            ((line.id + 0xd0) as *const u32).read(),
        )
    }
    pub fn touched(&mut self) -> Option<Line> {
        unsafe {
            if !game::granted(0x2029fb0) || !natives::is_grounded(self.accessor) {
                return None;
            }
            let down = super::tech_native::id("GROUND_TOUCH_ID_DOWN")?;
            let touch: extern "C" fn(usize, i32) -> usize =
                std::mem::transmute(game::text_base() + 0x2029fb0);
            let id = touch(self.accessor, down);
            self.read(id)
        }
    }
    /// One stage collision query per invocation, independent of P1's footing.
    pub fn find_seed(&mut self, search: &mut Bootstrap) -> Result<Option<(usize, usize)>, ()> {
        if let Some(seed) = search.seed() {
            return Ok(Some(seed));
        }
        if let Some(ray) = search.current() {
            match unsafe { self.ray(ray.origin, ray.delta) }? {
                Some((line, point)) => search.hit(line, point).map_err(|_| ())?,
                None => search.miss(),
            }
        }
        if search.finished() && search.seed().is_none() {
            return Err(());
        }
        Ok(search.seed())
    }
    /// Reviewed 0x4658b0 → 0x5266c0 returns a borrowed raw line or zero.
    /// The second vector is a displacement; true leaves exclusion bit 0x2000 clear.
    /// Inputs and output reserve the complete 16-byte SIMD storage used by native.
    unsafe fn ray(&mut self, origin: Vec2, delta: Vec2) -> Result<Option<(Line, Vec2)>, ()> {
        if !game::granted(FLOOR_RAY) {
            return Err(());
        }
        let mut hit = RayPoint([f32::NAN; 4]);
        let id = super::stage_geometry::cast(native_ray(game::text_base()), self.accessor, [origin.x, origin.y], [delta.x, delta.y], true, &mut hit);
        if id == 0 {
            return Ok(None);
        }
        self.read(id)
            .map(|line| {
                Some((
                    line,
                    Vec2 {
                        x: hit.0[0],
                        y: hit.0[1],
                    },
                ))
            })
            .ok_or(())
    }
}
impl Source for NativeSource {
    fn read(&mut self, id: usize) -> Option<Line> {
        unsafe {
            if id & 7 != 0 || !readable(id, 0x120) {
                return None;
            }
            // Borrowed line: the native touch getter checks this same active/alive state.
            let control = pointer(id + 0x118)?;
            if control & 7 != 0
                || !readable(control, 0x10)
                || ((control + 8) as *const i64).read() == -1
                || ((id + 0x100) as *const u8).read() == 0
            {
                return None;
            }
            let ends = [point(id, 0)?, point(id, 1)?];
            let [x, y] = ((id + 0xa0) as *const [f32; 2]).read_unaligned();
            decode(id, ends, Vec2 { x, y }, ((id + 0xc5) as *const u8).read())
        }
    }
    fn next(&mut self, line: Line, end: usize) -> Option<(usize, usize)> {
        let shared = *line.ends.get(end)?;
        // Read again: a moved line must not be joined using stale endpoint coordinates.
        if self.read(line.id)? != line {
            return None;
        }
        unsafe {
            if !game::granted(0x15cbc60) {
                return None;
            }
            let call: extern "C" fn(usize, *const RayPoint) -> usize =
                std::mem::transmute(game::text_base() + 0x15cbc60);
            let id = call(line.id, &RayPoint([shared.x, shared.y, 0.0, 0.0]));
            let next = self.read(id)?;
            far_end(shared, next).map(|end| (id, end))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanState {
    More,
    Done,
    Broken,
    Full,
}
pub struct PlatformScan {
    rays: Rays,
}
impl PlatformScan {
    pub fn new(range: [f32; 4]) -> Option<Self> {
        Rays::new(range).map(|rays| Self { rays })
    }
    /// At most one native ray per invocation, including stacked-platform continuation.
    pub fn step(&mut self, source: &mut NativeSource, surfaces: &mut Surfaces) -> ScanState {
        use crate::model::stage_geometry::{scan_step, Scan};
        match scan_step(&mut self.rays, |ray| unsafe { source.ray(ray.origin, ray.delta) }) {
            Scan::Done => return ScanState::Done,
            Scan::Hit(line) => {
                if line.passable && line.kind().is_some() && !surfaces.push(line) {
                    return ScanState::Full;
                }
            }
            Scan::Miss => {}
            Scan::Broken => return ScanState::Broken,
        }
        if self.rays.current().is_none() {
            ScanState::Done
        } else {
            ScanState::More
        }
    }
}

/// One step of a floor read (rc.15 Eggs-ercise, `hitbox::eggs_floors`): done, the next ray (and the floor it met), or broken.
pub enum FloorStep {
    Done,
    Next(Option<[[f32; 2]; 2]>),
    Broken,
}
impl NativeSource {
    /// One native ray of `rays` (Tech's column walk, stacked platforms included), and the floor it met as world geometry
    /// only: no borrowed line id survives the call.
    pub fn floor_step(&mut self, rays: &mut Rays) -> FloorStep {
        use crate::model::stage_geometry::{scan_step, Scan};
        match scan_step(rays, |ray| unsafe { self.ray(ray.origin, ray.delta) }) {
            Scan::Done => FloorStep::Done,
            Scan::Hit(line) => {
                FloorStep::Next((line.kind() == Some(Kind::Floor)).then(|| line.ends.map(|e| [e.x, e.y])))
            }
            Scan::Miss => FloorStep::Next(None),
            Scan::Broken => FloorStep::Broken,
        }
    }
}

/// Native dead-area fields returned by app::camera::get_dead_area in SIMD q0.
/// Read the guarded fields directly rather than using an incompatible Rust HFA return.
pub unsafe fn blast_bounds() -> Option<[f32; 4]> {
    if !guarded() {
        return None;
    }
    let holder = pointer(game::text_base() + 0x52b8f00)?;
    let manager = pointer(holder)?;
    if !readable(manager + 0xc30, 16) {
        return None;
    }
    let range = ((manager + 0xc30) as *const [f32; 4]).read_unaligned();
    (range.into_iter().all(f32::is_finite) && range[0] < range[1] && range[3] < range[2])
        .then_some(range)
}

/// The existing dead-area guard, using the same owned camera lifetime as
/// camera_limit_range. Call only on a current native Training fighter frame.
pub(super) unsafe fn training_blast_bounds() -> Option<[f32; 4]> {
    use crate::model::sites_hitbox::CAMERA_LIMIT_RANGE_OFFSET;
    use crate::model::sites_pkg2::CAMERA_MANAGER_SLOT_OFFSET;
    if !guarded() || !game::granted(CAMERA_LIMIT_RANGE_OFFSET) {
        return None;
    }
    let holder = ((game::text_base() + CAMERA_MANAGER_SLOT_OFFSET) as *const *const usize).read_volatile();
    let manager = (!holder.is_null()).then(|| holder.read_volatile())?;
    (manager != 0).then(|| ((manager + 0xc30) as *const [f32; 4]).read_unaligned())
}
