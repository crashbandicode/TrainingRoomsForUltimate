//! Eggs-ercise (rc.14, `artifacts/rc14-eggsercise/SPEC.md` §V2): break as many targets as you can in a minute, 10 or 20
//! as fast as you can, or without a timer in free practice. The target is one of the plugin's own pictures (D1 a), drawn
//! whether Hitbox Visualization is On or Off (`Run::look`) and broken when one of Player 1's live attack pills overlaps it;
//! nothing is written to the game. The runtime half is `runtime::features::hitbox::eggs_pass`.

use super::egg_paths::{Flight, Path};
use super::egg_zone::{Side, Zone};
use super::hitbox::{Sphere, KIND_HIT, SCALE};
use super::hitbox_orbs::CANVAS_HALF;
use super::panes::{EggLook, EggPicture, EGG_WHOLE};
use super::scene::{scene, Facts};
use super::spacing;

pub const ROW: &str = "eggsercise";
pub const ROW_BREAK: &str = "eggsercise_break";
pub const ROW_SPEED: &str = "eggsercise_speed";
pub const ROW_TARGET: &str = "eggsercise_target";
pub const ROW_AREA: &str = "eggsercise_area";
pub const IN_VIEW: u8 = 0;
pub const WHOLE: u8 = 1;
pub const OFF_STAGE: u8 = 2;
pub const ROW_PATHS: &str = "eggsercise_paths";
pub const ROW_EXPIRY: &str = "eggsercise_expiry";
// Checklist bits 0–3 use projectile flight. Index 4 uses the original Pop up flight.
const MOVING_PATHS: u8 = 0b1111;
const POP_UP_PATH: u8 = 1 << 4;

/// Encode checklist cells without losing additional checked options.
pub fn checklist(snapshot: &super::settings::Snapshot, id: &str) -> u8 {
    snapshot.get(id).unwrap_or_default().iter().take(8).enumerate()
        .fold(0, |mask, (bit, value)| mask | (u8::from(*value != 0) << bit))
}
/// Hide CPU's row (CPU page): with it On the CPU cannot be the target.
pub const ROW_HIDE_CPU: &str = "hide_cpu";
/// V2.6: `Hide` (the first option) hides the CPU while a practice game is armed; Hide CPU's rule reads it.
pub const ROW_PRACTICE_CPU: &str = "practice_cpu";
/// The standing notice (D5).
pub const NOTICE_ID: u8 = 0x2E;
pub const HEADER: &str = "Eggs-ercise";
/// The `eggsercise` row's options: the mode.
pub const OFF: u8 = 0;
pub const TIMED: u8 = 1;
/// `10 targets`, `20 targets` (SPEC V2.4): the clock counts up and stops at the Nth target (`goal`).
#[cfg(test)]
pub const TEN: u8 = 2;
#[cfg(test)]
pub const TWENTY: u8 = 3;
pub const FREE: u8 = 4;
/// The `eggsercise_break` row's options: what breaks an egg (D3 b: any attack by default; projectiles never).
pub const ANY: u8 = 0;
pub const AERIALS: u8 = 1;
pub const STRONG: u8 = 2;
/// `Hits of 12%+`.
pub const STRONG_DAMAGE: f32 = 12.0;
/// The `eggsercise_target` row's options (V2.1): the plugin's disc, or the CPU, flying the disc's flight (the operator,
/// 11-15; it stood still before).
pub const DISC: u8 = 0;
pub const CPU: u8 = 1;
/// In flight the CPU target is never put lower than this above its ground (the launch tick, the bounce's touch): the
/// game's own gravity moves it for one frame before the next placement, and must not land it mid-flight.
pub const LIFT: f32 = 1.0;

/// What the CPU target's pass asks of the game besides its position (`Run::cpu_pass`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Put {
    /// Only the position (its speed cleared): every tick it is held.
    Hold,
    /// Afresh: 0%, and its pose: in the air (`true`, the game's fall) or standing.
    Fresh(bool),
    /// The flight came to rest: standing.
    Land,
}

/// Whether TR keeps the CPU target's player tag on this tick, by writing `FLAG_CURSOR` = true (rc.14 cpu-tag-target). The
/// game turns the flag off for the CPU that TR places in the air (Eden's cursor probe: false through the flight, true again at
/// the landing). `moved` is the target's pass (`Run::cpu_pass`): a position means TR owns the placement this tick (the flight
/// and the hold at rest); none means the game owns the CPU (the hit's pass, its hitlag, a run that ended), and the tag stays
/// the game's business there, as in vanilla. A hidden body keeps the hide step's off-write (`cpu::hide::tag`).
pub fn keeps_cursor(moved: Option<([f32; 2], Put)>, body_hidden: bool) -> bool {
    moved.is_some() && !body_hidden
}

/// `Aerials`: the fighter's status is `FIGHTER_STATUS_KIND_ATTACK_AIR` (13.0.5; the pilot's move observer's value).
pub const ATTACK_AIR: i32 = 0x36;
/// `1 minute`, in clock ticks (D4: Player 1's game frames, so pause, menus and Game speed stretch it alike).
pub const RUN_FRAMES: u32 = 3600;
/// How long "Time!" shows.
pub const END_FRAMES: u16 = 180;
/// The `eggsercise_speed` row's options (§2.3b): time runs `SPEEDS[option]` times as fast; the path stays the same.
pub const NORMAL: u8 = 0;
#[cfg(test)]
pub const SLOW: u8 = 1;
pub const FAST: u8 = 2;
pub const SPEEDS: [f32; 3] = [1.0, 0.5, 1.5];
/// The spawn (§2.3b, V2.2): on the run's ground, x within the stage's camera range (`range`; `HALF_WIDTH` either side of the
/// centre without it) and on screen, `DRIFT` in from its edges, at least `MIN_GAP` from the target just broken.
pub const HALF_WIDTH: f32 = 60.0;
pub const MIN_GAP: f32 = 30.0;
pub const RADIUS: f32 = 4.0;
/// The flight at Normal (§2.3b): straight up at `LAUNCH` a tick under `GRAVITY` (21.6 up at tick 30, back down at 60), one
/// bounce at `BOUNCE` times that speed, then rest; sideways up to `DRIFT_SPEED` a tick either way, so at most `DRIFT` in
/// all.
pub const LAUNCH: f32 = 1.44;
pub const GRAVITY: f32 = 0.048;
/// The flight's apex above its spawn, the same at every speed (`LAUNCH² / 2 GRAVITY`: 21.6).
#[cfg(test)]
pub const APEX: f32 = LAUNCH * LAUNCH / (2.0 * GRAVITY);
pub const BOUNCE: f32 = 0.6;
pub const DRIFT_SPEED: f32 = 0.1;
pub const DRIFT: f32 = 10.0;
/// rc.15 (the operator's card report, 11-18; the primary's ruling 15:30Z): targets appear at random places across the stage,
/// like Melee's Eggs-ercise, not on Player 1's ground alone. A spawn takes a random point of one of the stage's floors (its
/// main-stage tops and platforms, read from the game's collision: `set_floors`), chosen by length, and rests on it or in the
/// air above it, up to `REACH` (within a jump).
pub type Floor = [[f32; 2]; 2];
pub const FLOORS: usize = 24;
pub const REACH: f32 = 25.0;
/// A new target keeps this far from Player 1's body centre (`BODY` above its `top` joint): never inside P1.
pub const P1_GAP: f32 = 16.0;
pub const BODY: f32 = 8.0;
/// A break freezes the fighter that broke it for `HITLAG` ticks (V2.1: no native gives an attack's own hitlag, so the
/// spec's fixed 6).
pub const HITLAG: i32 = 6;
/// Draw ticks the camera must hold still after a fighter reset: L+R+A sends it gliding to the new spot for a
/// moment, and the target must land in the view it keeps (Eden wa-eggsfly-a6266ac1-1: a target spawned at x 70 by the view
/// before the glide was off screen for 92 of its 93 ticks).
pub const SETTLE: u32 = 10;
/// The break animation (V2.1 ii, our own): the broken target grows by `BURST_GROW` of its size and fades out over
/// `BURST_TICKS`; at most two at once, a third replaces the older.
pub const BURST_TICKS: u32 = 12;
pub const BURST_GROW: f32 = 0.6;
/// The animations' spots and ticks, the newest first (Break the Targets keeps the same).
pub type Bursts = [Option<([f32; 2], u32)>; 2];

/// One animation's picture, `t` ticks in.
pub fn burst((at, t): ([f32; 2], u32)) -> ([f32; 2], EggLook) {
    let fade = t as f32 / BURST_TICKS as f32;
    let mut pic = EGG_WHOLE;
    pic.scale = 1.0 + BURST_GROW * fade;
    pic.alpha = (255.0 * (1.0 - fade)) as u8;
    pic.picture = EggPicture::Cracked;
    (at, pic)
}

/// Age the animations by `tick`; those past `BURST_TICKS` are gone.
pub fn age(bursts: &mut Bursts, tick: bool) {
    let dt = u32::from(tick);
    for b in bursts {
        *b = b.map(|(at, t)| (at, t + dt)).filter(|b| b.1 < BURST_TICKS);
    }
}

/// The hitlag of a break (Eggs-ercise and Break the Targets): wherever the practice is live, native Training or an active
/// waiting-room practice (`scene::is_training`; V2.1; rc.14, 11-15: the LDN pass showed it local to the console).
pub fn hitlag(facts: Facts) -> Option<i32> {
    super::scene::is_training(facts).then_some(HITLAG)
}


/// The camera's map from the fighters' plane to the layout canvas (`hitbox_orbs::to_canvas`): per axis, canvas = world *
/// `.0` + `.1`; `.2` is the x range the stage allows, and `.3` its optional vertical bounds. The orb draw publishes it
/// each tick of a live run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View(pub [f32; 2], pub [f32; 2], pub [f32; 2], pub Option<[f32; 2]>);

/// One picture of the draw: its spot in the fighters' plane and its look.
pub type Look = Option<([f32; 2], EggLook)>;

/// The x range a target may spawn in (V2.2): the stage's own camera range, the game's camera limit rect `[left, right, ..]`;
/// the draft's `HALF_WIDTH` box when it is missing or not a range at least as wide.
pub fn range(limit: Option<[f32; 4]>) -> [f32; 2] {
    let [l, r, ..] = limit.unwrap_or_default();
    let wide = (r - l).is_finite() && r - l >= 2.0 * HALF_WIDTH;
    [[-HALF_WIDTH, HALF_WIDTH], [l, r]][usize::from(wide)]
}

/// The stage's vertical range from the camera limit rect `[left, right, top, bottom]`.
pub fn vertical(limit: Option<[f32; 4]>) -> Option<[f32; 2]> {
    let [_, _, top, bottom] = limit?;
    (bottom.is_finite() && top.is_finite() && top > bottom).then_some([bottom, top])
}

/// The rows' option indices (the first option when a row is missing).
pub fn choices(snapshot: &super::settings::Snapshot) -> [u8; 5] {
    let mut rows = [ROW, ROW_BREAK, ROW_SPEED, ROW_TARGET, ROW_AREA].map(|id| choice(snapshot, id));
    // Hide CPU wins over the CPU as the target: the disc is used (V2.1; `DISC` is 0).
    let hide = snapshot.get(ROW_HIDE_CPU).unwrap_or_default();
    rows[3] *= u8::from(!hide.first().is_some_and(|on| *on != 0));
    rows
}

/// One row's option index (the first when the row is missing).
pub fn choice(snapshot: &super::settings::Snapshot, id: &str) -> u8 {
    let row = snapshot.get(id).unwrap_or_default();
    row.iter().position(|v| *v != 0).unwrap_or(0) as u8
}

/// V2.6: whether a practice game asks for the CPU to be hidden this frame (`hide_cpu::hide`, the `request` of
/// `cpu::hide::step`). `practice_hide`: CPU during a game says Hide; `running`: an Eggs-ercise run is armed;
/// `cpu_target`: the CPU is its target (`Run::cpu_target`); `btt`: Break the Targets runs. An Eggs-ercise run hides the CPU
/// unless the CPU is its target: the target wins, and then the CPU's body, its tag and its panel all stay, because all three
/// follow the one step this request feeds (`cpu::hide::step`, `tag` and `panels`). Break the Targets hides it whatever the
/// Eggs rows say.
pub const fn hides_cpu(practice_hide: bool, running: bool, cpu_target: bool, btt: bool) -> bool {
    practice_hide && (running && !cpu_target || btt)
}

/// One live attack of the fighter in world space: a pill from `.0` to `.1` (the same point for a point hitbox), its radius
/// and its damage.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hit(pub [f32; 3], pub [f32; 3], pub f32, pub f32);

/// Whether a hit of `power` counts under `rule`, `aerial` being whether the fighter is in an aerial.
pub(crate) fn counts(rule: u8, aerial: bool, power: f32) -> bool {
    [true, aerial, power >= STRONG_DAMAGE][usize::from(rule.min(STRONG))]
}

/// Whether `hit` breaks the egg at `egg` (the fighters' plane): `rule` lets it count and the pill overlaps the egg.
pub fn breaks(rule: u8, aerial: bool, hit: &Hit, egg: [f32; 2]) -> bool {
    counts(rule, aerial, hit.3) && depth(hit, egg).is_some()
}

/// The hit's depth into the target disc, using the same pill-to-centre distance as `breaks`.
pub fn depth(hit: &Hit, egg: [f32; 2]) -> Option<f32> {
    let Hit(a, b, radius, _) = *hit;
    let (dx, dy, px, py) = (b[0] - a[0], b[1] - a[1], egg[0] - a[0], egg[1] - a[1]);
    // A point hitbox: 0 / epsilon is 0.
    let t = ((px * dx + py * dy) / (dx * dx + dy * dy).max(1e-9)).clamp(0.0, 1.0);
    let d = (px - t * dx).hypot(py - t * dy);
    spacing::depth(radius + RADIUS - d, RADIUS)
}

/// The deepest hit that counts under `rule` and overlaps the target.
pub fn deepest(rule: u8, aerial: bool, hits: &[Hit], egg: [f32; 2]) -> Option<f32> {
    hits.iter()
        .filter(|hit| counts(rule, aerial, hit.3))
        .filter_map(|hit| depth(hit, egg))
        .reduce(f32::max)
}

/// Article depth is graded regardless of a helper break; a helper-only break has no grade.
pub fn other_break(graded: bool, helper: bool, article: impl FnOnce(bool) -> Option<f32>) -> Option<Option<f32>> {
    let depth = if helper && !graded { None } else { article(graded) };
    (helper || depth.is_some()).then_some(depth)
}

/// The pills of a fighter's live attack spheres (`hitbox::want_hitboxes`), each end placed by `place`, a capsule's two ends
/// one pill; `power(slot)` is the slot's damage. BTT also reads articles here; Eggs-ercise calls it only for fighters.
pub fn pills<P, D>(spheres: &[Sphere], mut place: P, mut power: D, out: &mut [Hit]) -> usize
where
    P: FnMut(&Sphere) -> Option<[f32; 3]>,
    D: FnMut(i32) -> f32,
{
    let first = |s: &&Sphere| s.key >> 24 == KIND_HIT && s.key & 0xff == 0;
    let mut n = 0;
    for s in spheres.iter().filter(first).take(out.len()) {
        let Some(a) = place(s) else { continue };
        let b = spheres.iter().find(|e| e.key == s.key + 1);
        let b = b.and_then(&mut place).unwrap_or(a);
        out[n] = Hit(a, b, s.scale / SCALE, power((s.key >> 8 & 0xffff) as i32));
        n += 1;
    }
    n
}

/// What one fighter's pass does: nothing, clear the notice (the run stopped), or read the fighter and step the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    Idle,
    Clear,
    Read,
}

/// Live P1 observation. A pending hit resolves only on a main-floor landing or ledge catch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Recovery {
    /// On the stage without completing a pending recovery, including platforms.
    Unchanged,
    Away,
    Returned,
    Lost,
    #[default]
    Unknown,
}

/// The score and streak consume the outcome at the point of contact, expiry, or recovery.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TargetEvent {
    #[default]
    None,
    Hit,
    Pending,
    Recovered,
    Miss,
}

type Point = [f32; 2];
type Rect = [Point; 2];

/// The runtime supplies a scoped clearance check against the current stage's actual outline.
pub type Clearance<'a> = dyn Fn(&Zone, Side, Point, Point) -> bool + 'a;
const NO_HULL: &Clearance<'_> = &|_, _, _, _| false;

#[derive(Clone, Copy, Debug)]
struct Motion {
    flight: Option<Flight>,
    at: Point,
    scale: f32,
    bounds: Rect,
    side: Option<Side>,
}

impl Motion {
    fn new(path: Option<Path>, anchor: Point, bounds: Rect, side: Option<Side>, right: bool) -> Self {
        let envelope = path.map_or([[0.0; 2]; 2], |path| {
            let [mut low, mut high] = path.envelope();
            if !right {
                (low[0], high[0]) = (-high[0], -low[0]);
            }
            [low, high]
        });
        let mut scale = 1.0f32;
        for axis in 0..2 {
            let span = envelope[1][axis] - envelope[0][axis];
            if span > 0.0 {
                scale = scale.min((bounds[1][axis] - bounds[0][axis]) / span);
            }
        }
        let at = core::array::from_fn(|axis| {
            let lo = bounds[0][axis] - scale * envelope[0][axis];
            let hi = bounds[1][axis] - scale * envelope[1][axis];
            anchor[axis].clamp(lo, hi.max(lo))
        });
        Self { flight: path.map(|path| Flight::new(path, right)), at, scale, bounds, side }
    }
}

/// Native P1 inputs for one pass. The keeper ticks time and every live P1 body can hit.
pub struct Pass<'a> {
    pub tick: bool,
    pub y: f32,
    pub hits: &'a [Hit],
    pub air: bool,
    pub kind: i32,
}

/// The run: armed by a menu close with the row on, until Off or the end of practice.
#[derive(Clone, Copy, Debug)]
pub struct Run {
    /// The mode, rule, speed, target and spawn area (`choices`) of the Start that began it.
    rows: [u8; 5],
    areas: u8,
    paths: u8,
    expiry: u16,
    clock_started: bool,
    area: Option<u8>,
    path: Option<Path>,
    serial: u32,
    recovery: Recovery,
    pending_hit: Option<(Point, bool)>,
    zone: Option<Zone>,
    offstage_unavailable: bool,
    zone_prefs: Option<(i32, Option<[u8; 4]>)>,
    blast: Option<Rect>,
    motion: Option<Motion>,
    event: TargetEvent,
    /// Eggs broken this run.
    pub score: u32,
    frames: u32,
    ground: f32,
    /// The target's spawn point, its ticks since, and its flight: launch speed, sideways speed, gravity.
    egg: Option<[f32; 2]>,
    age: u32,
    fly: [f32; 3],
    /// The break animations.
    bursts: Bursts,
    rng: u32,
    /// The clock keeper (`keeper`) and its rivals' passes since its last.
    owner: (usize, u8),
    shown: Option<(u32, u32, bool)>,
    pub view: Option<View>,
    /// Draw ticks the camera has held the same `view`.
    still: u32,
    /// Whether Player 1 was in an aerial on its last pass, the CPU target's state (`cpu_pass`), and whether it was last
    /// put in the air.
    air: bool,
    cpu: u8,
    aloft: bool,
    /// The session's bests per fighter kind (`& 127`), and the last run's at 128, per mode (`TIMED` to `TWENTY`, from 0):
    /// the 1-minute score, and the fastest 10 and 20 as `!frames`, so the larger is the better in all three.
    best: [[u32; 3]; 129],
    /// The stage's floors (`set_floors`), whether this run has asked for them, Player 1's spot (`sense`), and whether the last
    /// break was the fighter's own attack (not an article's: only then does it freeze, `melee`).
    floors: [Option<Floor>; FLOORS],
    scanned: bool,
    pending: bool,
    p1: [f32; 2],
    melee: bool,
    /// The target rests in the air above its floor (the CPU target stays in its fall there, never stood up mid-air).
    perch: bool,
}

impl Run {
    pub const IDLE: Run = Run {
        rows: [OFF, ANY, NORMAL, DISC, IN_VIEW],
        areas: 1 << IN_VIEW,
        paths: 0,
        expiry: 0,
        clock_started: false,
        area: None,
        path: None,
        serial: 0,
        recovery: Recovery::Unknown,
        pending_hit: None,
        zone: None,
        offstage_unavailable: false,
        zone_prefs: None,
        blast: None,
        motion: None,
        event: TargetEvent::None,
        score: 0,
        frames: 0,
        ground: 0.0,
        egg: None,
        age: 0,
        fly: [0.0; 3],
        bursts: [None; 2],
        rng: 1,
        owner: (0, 2),
        shown: None,
        view: None,
        still: SETTLE,
        air: false,
        cpu: 0,
        aloft: false,
        best: [[0; 3]; 129],
        floors: [None; FLOORS],
        scanned: false,
        pending: false,
        p1: [0.0; 2],
        melee: false,
        perch: false,
    };

    /// The runtime is to read the stage's floors for this run (once per start: a reset may have changed the stage).
    pub fn floors_wanted(&self) -> bool {
        self.running() && self.available_areas() & 0b011 != 0 && !self.scanned
    }

    /// The runtime began reading the floors: the first target waits for them (`set_floors`), so it is never put by the
    /// fallback ground while the read is on its way.
    pub fn want_floors(&mut self) {
        self.pending = true;
    }

    /// The read `want_floors` began is still on its way.
    pub fn pending(&self) -> bool {
        self.pending
    }

    /// The stage's floors, as the game's collision has them (none when it could not be read: the run's ground is used).
    pub fn set_floors(&mut self, floors: &[Floor]) {
        self.floors = core::array::from_fn(|i| floors.get(i).copied());
        (self.scanned, self.pending) = (true, false);
    }

    /// Player 1's `top` joint this pass (each pass, before `step`).
    pub fn sense(&mut self, p1: [f32; 2]) {
        self.p1 = p1;
    }

    pub fn configure(&mut self, paths: u8, seconds: u8) {
        self.paths = paths & (MOVING_PATHS | POP_UP_PATH);
        self.expiry = u16::from(seconds.min(10)) * 60;
    }

    pub fn set_areas(&mut self, areas: u8) {
        self.areas = (areas & 0b111).max(1);
    }

    pub fn target_id(&self) -> u32 {
        self.serial
    }

    pub fn event(&self) -> TargetEvent {
        self.event
    }

    pub fn recovering(&self) -> bool {
        self.pending_hit.is_some()
    }

    pub fn observe_recovery(&mut self, status: Recovery) {
        self.recovery = status;
    }

    /// The P1 pass only needs recovery reads for an off-stage target or an unsettled hit.
    pub fn needs_recovery(&self) -> bool {
        self.area == Some(OFF_STAGE) || self.recovering()
            || self.area.is_none() && self.available_areas() & (1 << OFF_STAGE) != 0
    }

    pub fn wants_zone(&self) -> bool {
        self.areas & (1 << OFF_STAGE) != 0 && self.zone.is_none() && !self.offstage_unavailable
    }

    pub fn wants_blast(&self) -> bool {
        self.uses_paths() && self.blast.is_none()
    }

    pub fn set_zone(&mut self, zone: Zone) {
        self.zone = Some(zone);
    }

    /// Keep the configured areas for the next reset, but retire an unavailable
    /// Off stage choice for this run. A pending draw can now pick a usable area.
    #[cfg(any(test, target_os = "switch"))]
    pub fn disable_offstage(&mut self) -> Option<&'static str> {
        if !self.wants_zone() {
            return None;
        }
        self.offstage_unavailable = true;
        if self.area == Some(OFF_STAGE) {
            self.area = None;
            self.path = None;
        }
        Some(if self.areas == 1 << OFF_STAGE {
            "No usable Off stage zone. Using In view."
        } else {
            "No usable Off stage zone. Using other checked areas."
        })
    }

    fn available_areas(&self) -> u8 {
        if self.offstage_unavailable {
            (self.areas & !(1 << OFF_STAGE)).max(1 << IN_VIEW)
        } else {
            self.areas
        }
    }

    pub fn zone_preferences(&mut self, stage: i32, published: impl FnOnce() -> Option<[u8; 4]>) -> Option<[u8; 4]> {
        if let Some((id, preference)) = self.zone_prefs {
            if id == stage {
                return preference;
            }
        }
        let preference = published();
        self.zone_prefs = Some((stage, preference));
        preference
    }

    pub fn uses_paths(&self) -> bool {
        self.paths & MOVING_PATHS != 0
    }

    pub fn set_blast(&mut self, blast: Rect) {
        self.blast = Some(blast);
    }

    fn next_random(&mut self) -> u32 {
        self.rng = 1_013_904_223u32.wrapping_add(self.rng.wrapping_mul(1_664_525));
        self.rng
    }

    /// Uniform over checked cells. A singleton preserves the ordinary spawn RNG sequence.
    fn pick(&mut self, mut mask: u8) -> u8 {
        let count = mask.count_ones();
        let mut index = 0;
        if count > 1 {
            let threshold = count.wrapping_neg() % count;
            loop {
                let scaled = u64::from(self.next_random()) * u64::from(count);
                if scaled as u32 >= threshold {
                    index = (scaled >> 32) as u32;
                    break;
                }
            }
        }
        for _ in 0..index {
            mask &= mask - 1;
        }
        mask.trailing_zeros() as u8
    }

    fn empty_target(&mut self) {
        self.egg = None;
        self.motion = None;
        self.area = None;
        self.path = None;
        self.cpu = 0;
        self.age = 0;
    }

    fn contact(&mut self, egg: [f32; 2], melee: bool, recovery: Recovery) {
        self.melee = melee;
        if self.area == Some(OFF_STAGE) && !matches!(recovery, Recovery::Unchanged | Recovery::Returned) {
            self.event = if recovery == Recovery::Lost {
                TargetEvent::Miss
            } else {
                self.pending_hit = Some((egg, recovery == Recovery::Away));
                TargetEvent::Pending
            };
            self.empty_target();
        } else {
            self.event = TargetEvent::Hit;
            self.score += 1;
        }
    }

    fn resolve_hit(&mut self) -> Option<[f32; 2]> {
        let (egg, away) = self.pending_hit.as_mut()?;
        if self.recovery == Recovery::Away {
            *away = true;
        }
        self.event = match self.recovery {
            Recovery::Lost => TargetEvent::Miss,
            Recovery::Returned if *away => TargetEvent::Recovered,
            _ => return None,
        };
        self.score += u32::from(self.event == TargetEvent::Recovered);
        let egg = *egg;
        self.pending_hit = None;
        Some(egg)
    }

    fn expired(&self) -> bool {
        self.expiry != 0 && self.age >= u32::from(self.expiry)
    }

    /// Whether the last break was Player 1's own attack (the fighter freezes), not one of its articles'.
    pub fn melee(&self) -> bool {
        self.melee
    }

    /// The `Breaks on` option of this run.
    pub fn rule(&self) -> u8 {
        self.rows[1]
    }

    /// The N of `10 targets` and `20 targets`, 0 in the other modes.
    fn goal(&self) -> u32 {
        [0, 0, 10, 20, 0][usize::from(self.rows[0].min(FREE))]
    }

    /// A started mode remains armed through the result so L+R+A can restart it and Stop can disarm it.
    pub fn armed(&self) -> bool {
        self.rows[0] != OFF
    }

    pub fn running(&self) -> bool {
        self.armed() && !self.ended()
    }

    /// A run has the CPU as its target.
    pub fn cpu_target(&self) -> bool {
        self.rows[3] == CPU && self.armed()
    }

    pub fn ended(&self) -> bool {
        match self.rows[0] {
            TIMED => self.frames == 0,
            _ => self.goal() > 0 && self.score >= self.goal(),
        }
    }

    /// The draw's tick on the practice's layout (`active`): a run that wants eggs takes the camera `look` reads (nothing
    /// is read otherwise); the pictures to draw, each at its spot of the fighters' plane: the target, then the break
    /// animations.
    pub fn see(&mut self, active: bool, look: impl FnOnce() -> Option<View>) {
        if active && self.running() {
            let seen = look();
            if self.still < SETTLE {
                self.still = if seen.is_some() && seen == self.view { self.still + 1 } else { 0 };
            }
            self.view = seen;
        }
    }

    #[cfg(test)]
    pub fn look(&mut self, active: bool, look: impl FnOnce() -> Option<View>) -> [Look; 3] {
        self.see(active, look);
        self.pictures()
    }

    pub fn pictures(&self) -> [Look; 3] {
        let [a, b] = self.bursts.map(|b| b.map(burst));
        let whole = self.egg_at().filter(|_| self.rows[3] == DISC);
        [whole.map(|at| (at, EGG_WHOLE)), a, b]
    }

    /// A fresh run (the menu closed with the row on, or a reset): score 0, a full clock and no view from the previous run.
    pub fn start(&mut self, rows: [u8; 5], seed: u32) {
        let (rng, best) = ((self.rng ^ seed) | 1, self.best);
        *self = Run::IDLE;
        let frames = RUN_FRAMES * u32::from(rows[0] == TIMED);
        (self.rows, self.frames, self.rng, self.best) = (rows, frames, rng, best);
        self.set_areas(1u8.checked_shl(rows[4].into()).unwrap_or(1));
    }

    /// The reset hub ran (an L+R+A reset, or a new Training scene): an armed run starts over. A TR Change Stage
    /// does not call this.
    pub fn reset(&mut self, seed: u32) {
        if self.rows[0] != OFF {
            let settings = (self.areas, self.paths, self.expiry);
            self.start(self.rows, seed);
            self.still = 0;
            (self.areas, self.paths, self.expiry) = settings;
        }
    }

    fn stop(&mut self) -> Plan {
        let (armed, rng, best) = (self.rows[0] != OFF, self.rng, self.best);
        (*self, self.rng, self.best) = (Run::IDLE, rng, best);
        [Plan::Idle, Plan::Clear][usize::from(armed)]
    }

    /// One fighter's pass: `rows` are the two rows, `go` a menu close's seed, `entry` the fighter's entry; `f` (the scene
    /// facts, a game read) only for an entry-0 pass of a live run (or an ended one with a break animation to finish). Off,
    /// an unarmed or ended run and every other entry read nothing; outside practice the run stops, and a hot swap (busy)
    /// only idles it.
    pub fn plan(&mut self, rows: [u8; 5], go: Option<u32>, entry: i32, f: fn() -> Facts) -> Plan {
        match go {
            _ if rows[0] == OFF => return self.stop(),
            Some(seed) => self.start(rows, seed),
            None => {}
        }
        // An ended run is read again only to finish its break animations, or to close it when the CPU target's own pass
        // counted the last target (its target is still held: `step` writes the end notice and the best).
        if self.rows[0] == OFF || self.ended() && self.egg.is_none() && self.bursts == [None; 2] || entry != 0 {
            return Plan::Idle;
        }
        let facts = f();
        match scene(facts) {
            _ if facts.busy => Plan::Idle,
            None => self.stop(),
            Some(_) => Plan::Read,
        }
    }

    /// Whether `fighter`'s pass ticks the clock: the first to pass keeps it, and another takes it after the keeper missed
    /// two passes in a row (a swap replaced it); Ice Climbers' Nana passes too and does not tick it twice.
    pub fn keeper(&mut self, fighter: usize) -> bool {
        self.owner.1 += 1;
        if self.owner.0 == fighter || self.owner.1 > 2 {
            self.owner = (fighter, 0);
        }
        self.owner.1 == 0
    }

    /// A new target, launched (§2.3b) from a random spot of the stage's floors (`set_floors`; the run's ground when none was
    /// read), chosen by length and the selected spawn area. In view uses the screen bounds; Whole stage uses the full stage
    /// range and its vertical bounds when valid. It keeps `spacing` from `last` and `P1_GAP` from Player 1; when sixteen tries
    /// all fall short, it uses the try that falls least short (a random spot, never a fixed side).
    /// Screen bounds are usable on the first draw after Start. A fighter reset waits for its camera glide before
    /// using screen bounds. Complete Whole stage bounds are in world coordinates.
    fn spawn(&mut self, last: Option<Point>, clear: &Clearance<'_>) -> Option<()> {
        let area = if let Some(area) = self.area {
            area
        } else {
            let area = self.pick(self.available_areas());
            self.area = Some(area);
            self.path = (self.paths != 0).then(|| self.pick(self.paths)).and_then(Path::from_index);
            area
        };
        if area == OFF_STAGE {
            return self.spawn_offstage(last, clear);
        }
        let View(s, o, range, vertical) = self.view.filter(|_| !self.pending)?;
        let whole = area == WHOLE;
        (last.is_some() || self.still >= SETTLE || (whole && vertical.is_some())).then_some(())?;
        let edge = |i: usize, c: f32| (c * CANVAS_HALF[i] - o[i]) / s[i] - c * RADIUS;
        let (mut x0, mut x1, mut top, mut bottom) = if whole {
            let (top, bottom) = match vertical {
                Some([lo, hi]) => (hi - RADIUS, lo + RADIUS),
                None => (edge(1, 1.0), edge(1, -1.0)),
            };
            (range[0] + RADIUS + DRIFT, range[1] - RADIUS - DRIFT, top, bottom)
        } else {
            let inside = |x: f32| x.max(edge(0, -1.0) + DRIFT).min(edge(0, 1.0) - DRIFT);
            let [lo, hi] = [inside(range[0] + DRIFT), inside(range[1] - DRIFT)];
            (lo, hi, edge(1, 1.0), edge(1, -1.0))
        };
        let path_bounds = self.path.and_then(|_| {
            let [[left, low], [right, high]] = self.blast?;
            let [west, east] = if whole {
                [range[0] + RADIUS, range[1] - RADIUS]
            } else {
                [edge(0, -1.0), edge(0, 1.0)]
            };
            let west = west.max(left + RADIUS);
            let east = east.min(right - RADIUS);
            let low = bottom.max(low + RADIUS);
            let high = top.min(high - RADIUS);
            (west <= east && low <= high).then_some([[west, low], [east, high]])
        });
        if self.path.is_some() && path_bounds.is_none() {
            if !self.offstage_unavailable {
                return None;
            }
            // The Off stage fallback must also work if native blast bounds
            // cannot constrain a moving path. Use the ordinary visible flight.
            self.path = None;
        }
        if let Some([[west, low], [east, high]]) = path_bounds {
            x0 = x0.clamp(west, east);
            x1 = x1.clamp(west, east);
            bottom = low;
            top = high;
        }
        // The disc's centre is `RADIUS` above its floor; the CPU target's feet are on it, and it flies the disc's flight
        // (the operator, 11-15): its feet trace the disc's bottom edge.
        let lift = [0.0, -RADIUS][usize::from(self.rows[3] == CPU)];
        let mut spans = [([0.0f32; 2], [[0.0f32; 2]; 2]); FLOORS];
        let mut total = 0.0;
        let mut n = 0;
        for f in self.floors.iter().flatten() {
            let [a, b] = if f[0][0] <= f[1][0] { *f } else { [f[1], f[0]] };
            let (lo, hi) = ((a[0] + DRIFT).max(x0), (b[0] - DRIFT).min(x1));
            let height = a[1].max(b[1]) + RADIUS;
            if hi > lo && height >= bottom && height <= top {
                (spans[n], total, n) = (([lo, hi], [a, b]), total + hi - lo, n + 1);
            }
        }
        if n == 0 {
            // With no usable floor, keep the run's fallback ground in the spawn bounds.
            let ground = self.ground.max(bottom - RADIUS).min(top - RADIUS);
            let flat = [[x0, ground], [x1, ground]];
            (spans[0], total, n) = (([x0, x1.max(x0)], flat), (x1 - x0).max(0.0), 1);
        }
        let spans = &spans[..n];
        let speed = SPEEDS[usize::from(self.rows[2].min(FAST))];
        let g = GRAVITY * speed * speed;
        // A gap that cannot push every target to the other side of a narrow view: at most a third of its floors' length (a
        // spot a third in from either end still has room on its own side), `MIN_GAP` where there is room.
        let spacing = MIN_GAP.min((total - 2.0 * P1_GAP).max(0.0) / 3.0);
        let body = [self.p1[0], self.p1[1] + BODY];
        let flown = self.path.zip(path_bounds);
        let fit = |[a, b]: [Point; 2]| {
            let (path, [[west, low], [east, high]]) = flown?;
            let bounds = [[west.max(a[0] + RADIUS), low.max(a[1].max(b[1]) + RADIUS)], [east.min(b[0] - RADIUS), high]];
            let [from, to] = path.envelope();
            (0..2).all(|i| bounds[1][i] - bounds[0][i] >= [2.0 * RADIUS, 0.0][usize::from(from[i] == to[i])]).then_some(bounds)
        };
        let mut random = || {
            self.rng = 1_013_904_223u32.wrapping_add(self.rng.wrapping_mul(1_664_525));
            (self.rng >> 8) as f32 / (1u32 << 24) as f32
        };
        let mut pick = || {
            let mut u = random() * total;
            let ([lo, hi], [a, b]) = *spans.iter().find(|s| {
                let fits = u <= s.0[1] - s.0[0];
                u -= (s.0[1] - s.0[0]) * f32::from(!fits);
                fits
            }).unwrap_or(&spans[n - 1]);
            let x = (lo + u).min(hi);
            let on = a[1] + (b[1] - a[1]) * ((x - a[0]) / (b[0] - a[0]).max(1e-6)).clamp(0.0, 1.0);
            // In the air half the time, the disc kept a radius under the screen's top (its launch is lowered to fit below).
            let room = (top - on - 2.0 * RADIUS).max(0.0);
            let air = if room < 2.0 * RADIUS || random() < 0.5 { 0.0 } else { random() * REACH.min(room) };
            ([x, on + RADIUS + air], air > 0.0, (2.0 * random() - 1.0) * DRIFT_SPEED * speed, [a, b])
        };
        let dist = |p: [f32; 2], q: [f32; 2]| (p[0] - q[0]).hypot(p[1] - q[1]);
        let short = |disc: [f32; 2]| {
            let from_last = last.map_or(0.0, |l| spacing - dist([disc[0], disc[1] + lift], l));
            from_last.max(P1_GAP - dist(disc, body)).max(0.0)
        };
        let (disc, perch, vx, line) = (0..32)
            .map(|_| pick())
            .min_by(|p, q| fit(q.3).is_some().cmp(&fit(p.3).is_some()).then_with(|| short(p.0).total_cmp(&short(q.0))))?;
        let launch = (LAUNCH * speed).min((2.0 * g * (top - disc[1]).max(0.0)).sqrt());
        (self.egg, self.age, self.fly, self.perch) = (Some([disc[0], disc[1] + lift]), 0, [launch, vx, g], perch);
        if let Some((path, _)) = flown {
            let right = if matches!(path, Path::Missile) {
                self.p1[0] >= disc[0]
            } else {
                self.next_random() >> 31 == 0
            };
            if let Some(bounds) = fit(line) {
                let mut motion = Motion::new(self.path, disc, bounds, None, right);
                motion.at = [0, 1].map(|axis| motion.at[axis].min(bounds[1][axis]));
                self.motion = Some(motion);
                self.perch = true;
            } else {
                self.path = None;
            }
        }
        self.serial = self.serial.wrapping_add(1);
        self.clock_started = true;
        Some(())
    }

    fn spawn_offstage(&mut self, last: Option<Point>, clear: &Clearance<'_>) -> Option<()> {
        let zone = self.zone?;
        let first_right = self.next_random() >> 31 == 0;
        let body = [self.p1[0], self.p1[1] + BODY];
        let speed = SPEEDS[usize::from(self.rows[2].min(FAST))];
        let mut best = None;
        let mut penalty = f32::INFINITY;
        for side in if first_right { [Side::Right, Side::Left] } else { [Side::Left, Side::Right] } {
            let Some(bounds) = zone.rect(side) else { continue };
            if (0..2).any(|axis| bounds[1][axis] < bounds[0][axis]) {
                continue;
            }
            for _ in 0..32 {
                let anchor = core::array::from_fn(|axis| {
                    let t = (self.next_random() >> 8) as f32 / (1u32 << 24) as f32;
                    bounds[0][axis] + (bounds[1][axis] - bounds[0][axis]) * t
                });
                if !clear(&zone, side, anchor, anchor) {
                    continue;
                }
                let right = if matches!(self.path, Some(Path::Missile)) {
                    self.p1[0] >= anchor[0]
                } else {
                    side == Side::Right
                };
                let mut motion = Motion::new(self.path, anchor, bounds, Some(side), right);
                if !clear(&zone, side, anchor, motion.at) {
                    // Fitting the full envelope would cross the outline. Begin in free air
                    // and let the same swept clearance constrain each step of the flight.
                    motion.at = anchor;
                }
                let at = motion.at;
                let distance = |p: Point| (at[0] - p[0]).hypot(at[1] - p[1]);
                let short = (P1_GAP - distance(body))
                    .max(last.map_or(0.0, |p| MIN_GAP - distance(p)))
                    .max(0.0);
                if short < penalty {
                    (best, penalty) = (Some(motion), short);
                }
                if short == 0.0 {
                    break;
                }
            }
            if best.is_some() {
                break;
            }
        }
        let motion = best?;
        let g = GRAVITY * speed * speed;
        let room = motion.bounds[1][1] - motion.at[1];
        let launch = (LAUNCH * speed).min((2.0 * g * room.max(0.0)).sqrt());
        let vx = [DRIFT_SPEED, -DRIFT_SPEED][usize::from(first_right)] * speed;
        (self.egg, self.age, self.fly, self.perch) = (Some(motion.at), 0, [launch, vx, g], true);
        self.motion = Some(motion);
        self.serial = self.serial.wrapping_add(1);
        self.clock_started = true;
        Some(())
    }

    fn advance_motion(&mut self, clear: &Clearance<'_>) {
        let Some(mut motion) = self.motion else { return };
        let at = motion.at;
        let wanted = match motion.flight.as_mut() {
            Some(flight) => {
                let target = [self.p1[0] - at[0], self.p1[1] + BODY - at[1]];
                let delta = flight.advance(self.rows[2], target);
                core::array::from_fn(|axis| at[axis] + motion.scale * delta[axis])
            }
            None => self.bounce_at().unwrap_or(at),
        };
        let next = core::array::from_fn(|axis| wanted[axis].clamp(motion.bounds[0][axis], motion.bounds[1][axis]));
        if self.zone.zip(motion.side).is_none_or(|(zone, side)| clear(&zone, side, at, next)) {
            motion.at = next;
        }
        self.motion = Some(motion);
    }

    /// Diag only (Eden's `diag-egg.req`): the target at `at`: the disc at rest there; the CPU target placed afresh there and
    /// launched straight up at the run's speed.
    #[cfg(feature = "diag")]
    pub fn place(&mut self, at: [f32; 2]) {
        let speed = SPEEDS[usize::from(self.rows[2].min(FAST))];
        let v = LAUNCH * speed * f32::from(u8::from(self.rows[3] == CPU));
        self.motion = None;
        self.serial = self.serial.wrapping_add(1);
        (self.egg, self.age, self.fly, self.cpu, self.perch) = (Some(at), 0, [v, 0.0, GRAVITY * speed * speed], 0, false);
    }

    /// Diag only: the target's spawn point, its ticks since and its flight (launch speed, sideways speed, gravity).
    #[cfg(feature = "diag")]
    pub fn flight(&self) -> (Option<[f32; 2]>, u32, [f32; 3]) {
        (self.egg, self.age, self.fly)
    }

    /// Where the target is now (the fighters' plane): up and back down in `2 v / g` ticks, one bounce at `BOUNCE` times the
    /// speed, then at rest on its spawn height.
    pub fn egg_at(&self) -> Option<Point> {
        self.motion.map(|motion| motion.at).or_else(|| self.bounce_at())
    }

    fn bounce_at(&self) -> Option<Point> {
        let ([x, y], [v, vx, g], t) = (self.egg?, self.fly, self.age as f32);
        let up = 2.0 * v / g;
        let (v, t) = [(v * BOUNCE, (t - up).min(up * BOUNCE)), (v, t)][usize::from(t <= up)];
        let flown = (self.age as f32).min(up * (1.0 + BOUNCE));
        Some([x + vx * flown, y + (v * t - g * t * t / 2.0).max(0.0)])
    }

    /// Diag only: where the target comes to rest (Eden's check that it is clear of the notice there).
    #[cfg(feature = "diag")]
    pub fn rest(&self) -> Option<[f32; 2]> {
        let ([x, y], [v, vx, g]) = (self.egg?, self.fly);
        Some([x + vx * 2.0 * (1.0 + BOUNCE) * v / g, y])
    }


    /// One pass of an entry-0 fighter (`plan` said `Read`): `tick` (the clock keeper's pass, menu closed), `y` (its height:
    /// the ground of a run's first egg), its pills, whether it is in an aerial (`air`) and its kind (the session's best when the
    /// minute ends). The notice's length when its text changed (`write_notice`).
    #[cfg(test)]
    pub fn step(&mut self, tick: bool, y: f32, hits: &[Hit], air: bool, kind: i32) -> Option<u16> {
        self.step_with_depth(tick, y, hits, |_| None, air, kind).0
    }

    /// `step_with`, returning the deepest melee or article hit when this pass breaks a target.
    pub fn step_with_depth(
        &mut self,
        tick: bool,
        y: f32,
        hits: &[Hit],
        breaks: impl FnMut(Point) -> Option<Option<f32>>,
        air: bool,
        kind: i32,
    ) -> (Option<u16>, Option<f32>) {
        self.pass(Pass { tick, y, hits, air, kind }, breaks, NO_HULL)
    }

    pub fn pass(
        &mut self,
        Pass { tick, y, hits, air, kind }: Pass<'_>,
        mut breaks: impl FnMut(Point) -> Option<Option<f32>>,
        clear: &Clearance<'_>,
    ) -> (Option<u16>, Option<f32>) {
        // The break animations run on the clock's ticks (pause freezes them); an ended run only finishes them.
        let dt = u32::from(tick);
        age(&mut self.bursts, tick);
        self.air = air;
        self.event = TargetEvent::None;
        // A run the CPU target's last hit ended still holds its target until this pass closes it.
        if self.ended() && self.egg.is_none() {
            return (None, None);
        }
        let mut depth = None;
        let recovering = self.recovering();
        if recovering {
            if tick {
                if let Some(last) = self.resolve_hit() {
                    if !self.ended() {
                        self.spawn(Some(last), clear);
                    }
                }
            }
        } else if self.egg.is_none() {
            self.ground = y;
            self.spawn(None, clear);
        }

        // The pass closing a CPU target's last hit still writes the end notice.
        let dt = dt * u32::from(!self.ended() || self.event == TargetEvent::Recovered);
        let clock_live = self.clock_started || recovering || self.egg.is_some();
        if !recovering {
            if let Some(egg) = self.egg_at() {
                let disc = self.rows[3] == DISC;
                let melee = if disc { deepest(self.rows[1], air, hits, egg) } else { None };
                let additional = if melee.is_none() && disc { breaks(egg) } else { None };
                if melee.is_some() || additional.is_some() {
                    self.bursts = [Some((egg, 0)), self.bursts[0]];
                    self.contact(egg, melee.is_some(), self.recovery);
                    depth = melee.or_else(|| additional.flatten());
                    if self.event == TargetEvent::Hit {
                        self.empty_target();
                        self.spawn(Some(egg), clear);
                    }
                }
                if self.egg.is_some() && self.cpu < 3 {
                    self.age += dt;
                    if dt != 0 {
                        self.advance_motion(clear);
                    }
                }
                if disc && self.event == TargetEvent::None && self.expired() {
                    self.event = TargetEvent::Miss;
                    self.empty_target();
                    self.spawn(Some(egg), clear);
                }
            }
        }
        if clock_live {
            match self.rows[0] {
                TIMED => self.frames -= dt,
                _ => self.frames += dt * u32::from(self.goal() > 0),
            }
            if self.ended() {
                if self.pending_hit.take().is_some() {
                    self.event = TargetEvent::Miss;
                }
                let (k, m) = (kind as usize & 127, usize::from(self.rows[0] - TIMED));
                self.best[k][m] = self.best[k][m].max(if m == 0 { self.score } else { !self.frames });
                self.best[128][m] = self.best[k][m];
                self.empty_target();
                return (Some(END_FRAMES), depth);
            }
        }
        // The 1-minute notice changes once a second, the N-target one every tick (hundredths).
        let time = [self.frames, self.frames.div_ceil(60)][usize::from(self.rows[0] == TIMED)];
        let shown = (self.score, time, self.recovering());
        ((self.shown.replace(shown) != Some(shown)).then_some(u16::MAX), depth)
    }

    /// Whether the target is still in flight (launched, not yet at rest).
    fn flying(&self) -> bool {
        let [v, _, g] = self.fly;
        v > 0.0 && (self.age as f32) < 2.0 * v / g * (1.0 + BOUNCE)
    }

    /// The CPU target's pass (V2.1; the CPU's own fighter pass): where to hold the CPU this tick, the target's moving spot
    /// (`egg_at`, at least `LIFT` above its ground while it flies), and what else to ask (`Put`); `None` leaves it to the
    /// game (no CPU target, a run that ended, the hitlag and knockback after a hit). `damage` is its damage, `stopped`
    /// whether it is in hitlag. The first damage after a placement has read 0% is a hit, in the air or on the ground; it
    /// breaks the target when the rule lets it count, and then the next spot comes once the hitlag is over (a hit that does
    /// not count puts the CPU back on its flight where it was hit). `cpu`: 0 place, 1 wait for 0%, 2 armed, 3 counted hit in
    /// hitlag, 4 uncounted.
    pub fn cpu_pass(&mut self, damage: f32, stopped: bool) -> Option<(Point, Put)> {
        let recovery = self.recovery;
        self.cpu_pass_in(damage, stopped, NO_HULL, || recovery)
    }

    pub fn cpu_pass_in(
        &mut self,
        damage: f32,
        stopped: bool,
        clear: &Clearance<'_>,
        recovery_at_contact: impl FnOnce() -> Recovery,
    ) -> Option<(Point, Put)> {
        self.event = TargetEvent::None;
        self.egg.filter(|_| self.cpu_target() && !self.ended())?;
        let fresh = match (self.cpu, damage > 0.0) {
            (2, true) => {
                let counts = counts(self.rows[1], self.air, damage);
                self.cpu = 3 + u8::from(!counts);
                if counts {
                    let recovery = if self.area == Some(OFF_STAGE) { recovery_at_contact() } else { Recovery::Unknown };
                    self.contact(self.egg_at()?, false, recovery);
                }
                return None;
            }
            (3 | 4, _) if stopped => return None,
            (3, _) => {
                // The next target, `MIN_GAP` from where this one was hit.
                let last = self.egg_at();
                self.empty_target();
                self.spawn(last, clear);
                true
            }
            _ if self.expired() => {
                let last = self.egg_at();
                self.event = TargetEvent::Miss;
                self.empty_target();
                self.spawn(last, clear);
                true
            }
            (0 | 4, _) | (1, true) => true,
            _ => false,
        };
        self.cpu = 1 + u8::from(!fresh);
        let ([x, y], [_, ground], aloft) = (self.egg_at()?, self.egg?, self.flying() || self.perch);
        let put = match (fresh, self.aloft && !aloft) {
            (true, _) => Put::Fresh(aloft),
            (false, true) => Put::Land,
            _ => Put::Hold,
        };
        self.aloft = aloft;
        let foot = match self.motion {
            Some(_) => y - RADIUS,
            None if aloft => y.max(ground + LIFT),
            None => y,
        };
        Some(([x, foot], put))
    }

    /// The notice (SPEC V2.4): `Eggs N   m:ss` (1 minute), `Eggs N/10   m:ss.cc` (10 or 20 targets), `Eggs N` (free
    /// practice); at the end `Time! N eggs (best M)` or `Done! 10 in m:ss.cc (best m:ss.cc)`.
    pub fn write_notice(&self, out: &mut impl core::fmt::Write) -> core::fmt::Result {
        let (score, goal, best) = (self.score, self.goal(), self.best[128]);
        let left = self.frames.div_ceil(60);
        // m:ss.cc of a frame count.
        let clock = |f: u32| (f * 5 / 3 / 6000, f * 5 / 3 / 100 % 60, f * 5 / 3 % 100);
        let (m, s, c) = clock(self.frames);
        if self.recovering() {
            write!(out, "Recover!  ")?;
        }
        match self.rows[0] {
            TIMED if self.ended() => write!(out, "Time! {score} eggs (best {})", best[0]),
            _ if self.ended() => {
                let (bm, bs, bc) = clock(!best[usize::from(self.rows[0] - TIMED)]);
                write!(out, "Done! {goal} in {m}:{s:02}.{c:02} ")?;
                write!(out, "(best {bm}:{bs:02}.{bc:02})")
            }
            TIMED => write!(out, "Eggs {score}   {}:{:02}", left / 60, left % 60),
            _ if goal > 0 => write!(out, "Eggs {score}/{goal}   {m}:{s:02}.{c:02}"),
            _ => write!(out, "Eggs {score}"),
        }
    }
}

#[cfg(test)]
#[path = "eggs_tests.rs"]
mod tests;

#[cfg(test)]
mod standing_notice_tests {
    use super::*;

    const TRAINING: Facts = Facts { native_training: true, lobby_active: false, waiting_room: false, busy: false };
    const VIEW: View = View([14.0; 2], [0.0, -300.0], [-HALF_WIDTH, HALF_WIDTH], None);

    #[test]
    fn expired_time_is_finished_and_reset_restarts_the_round() {
        let mut run = Run::IDLE;
        let rows = [TIMED, ANY, FAST, DISC, IN_VIEW];
        run.start(rows, 7);
        run.look(true, || Some(VIEW));
        assert!(run.running());
        run.frames = 1;
        assert_eq!(run.step(true, 0.0, &[], false, 0), Some(END_FRAMES));
        assert!(run.egg_at().is_none());
        assert!(!run.running(), "a completed round must leave the Running state");

        run.reset(11);
        assert_eq!((run.frames, run.running()), (RUN_FRAMES, true));
        assert_eq!(run.plan(rows, None, 0, || TRAINING), Plan::Read);
    }

    #[test]
    fn in_view_uses_the_current_moving_projection_after_floor_read() {
        let mut run = Run::IDLE;
        run.start([TIMED, ANY, FAST, DISC, IN_VIEW], 7);
        run.want_floors();
        run.look(true, || Some(VIEW));
        run.step(true, 0.0, &[], false, 0);
        assert!(run.egg_at().is_none());
        assert_eq!(run.frames, RUN_FRAMES);

        // A projection across empty space can happen while following an off-stage fighter.
        run.set_floors(&[[[-80.0, 0.0], [80.0, 0.0]]]);
        let view = View([18.0; 2], [-2700.0, -2400.0], VIEW.2, None);
        run.look(true, || Some(view));
        run.step(true, -80.0, &[], false, 0);
        let at = run.egg_at().expect("a current projection suffices for the first target");
        for i in 0..2 {
            let canvas = at[i] * view.0[i] + view.1[i];
            assert!(canvas.abs() + RADIUS * view.0[i] <= CANVAS_HALF[i] + 0.01);
        }
        assert_eq!(run.frames, RUN_FRAMES - 1);
    }

    #[test]
    fn whole_stage_starts_with_moving_camera_after_floor_read() {
        let mut run = Run::IDLE;
        run.start([TIMED, ANY, FAST, DISC, WHOLE], 7);
        run.want_floors();
        for frame in 0..SETTLE {
            run.look(true, || Some(View(VIEW.0, [frame as f32, VIEW.1[1]], VIEW.2, Some([-65.0, 115.0]))));
            run.step(true, 0.0, &[], false, 0);
            assert!(run.egg_at().is_none(), "floor discovery is still pending");
        }
        run.set_floors(&[[[-80.0, 0.0], [80.0, 0.0]]]);
        run.look(true, || Some(View(VIEW.0, [SETTLE as f32, VIEW.1[1]], VIEW.2, Some([-65.0, 115.0]))));
        run.step(true, 0.0, &[], false, 0);
        assert!(run.egg_at().is_some(), "stage bounds suffice while the visible projection is changing");
        assert_eq!(run.frames, RUN_FRAMES - 1);
    }

    fn scored_cpu(score: u32) -> Run {
        let mut run = Run::IDLE;
        assert_eq!(run.plan([FREE, ANY, FAST, CPU, IN_VIEW], Some(7), 0, || TRAINING), Plan::Read);
        for _ in 0..=SETTLE { run.look(true, || Some(VIEW)); }
        run.step(true, 0.0, &[], false, 0);
        for _ in 0..score {
            run.cpu_pass(0.0, false);
            run.cpu_pass(0.0, false);
            run.cpu_pass(5.0, false);
            run.cpu_pass(5.0, false);
            run.step(true, 0.0, &[], false, 0);
        }
        assert_eq!(run.score, score);
        let mut text = String::new();
        run.write_notice(&mut text).unwrap();
        assert_eq!(text, format!("Eggs {score}"));
        run
    }

    fn published(run: &mut Run) -> Option<(u16, String)> {
        assert_eq!(run.plan(run.rows, None, 0, || TRAINING), Plan::Read);
        let tick = run.keeper(0x1000);
        let length = run.step(tick, 0.0, &[], false, 0)?;
        let mut text = String::new();
        run.write_notice(&mut text).unwrap();
        Some((length, text))
    }

    #[test]
    fn fs57_menu_restart_publishes_zero_before_camera() {
        let mut run = scored_cpu(4);
        let rows = run.rows;
        assert_eq!(run.plan(rows, Some(11), 0, || TRAINING), Plan::Read);
        assert_eq!(published(&mut run), Some((u16::MAX, "Eggs 0".into())));
        assert_eq!((run.egg_at(), run.view, run.frames, run.age), (None, None, 0, 0));
        assert_eq!(published(&mut run), None, "no unchanged per-frame publication");
        assert_eq!(run.cpu_pass(0.0, false), None, "notice does not place a CPU target");
    }

    #[test]
    fn fs58_lra_reset_publishes_zero_before_camera() {
        let mut run = scored_cpu(2);
        run.reset(11);
        assert_eq!(published(&mut run), Some((u16::MAX, "Eggs 0".into())));
        for _ in 0..SETTLE {
            run.look(true, || Some(VIEW));
            assert_eq!(published(&mut run), None);
            assert_eq!((run.egg_at(), run.frames, run.age), (None, 0, 0));
        }
        run.look(true, || Some(VIEW));
        assert_eq!(published(&mut run), None, "same Free text when the first target appears");
        assert!(run.egg_at().is_some());
        assert_eq!(run.age, 1, "normal flight starts only after the camera settled");
        assert!(matches!(run.cpu_pass(0.0, false), Some((_, Put::Fresh(true)))));
    }

    #[test]
    fn fs54_mode_change_publishes_new_format_before_camera() {
        let mut run = Run::IDLE;
        for (mode, expected, initial_clock) in [
            (TIMED, "Eggs 0   1:00", RUN_FRAMES),
            (TEN, "Eggs 0/10   0:00.00", 0),
            (TWENTY, "Eggs 0/20   0:00.00", 0),
        ] {
            assert_eq!(run.plan([mode, ANY, FAST, CPU, IN_VIEW], Some(11), 0, || TRAINING), Plan::Read);
            assert_eq!(published(&mut run), Some((u16::MAX, expected.into())));
            assert_eq!((run.egg_at(), run.view, run.frames, run.age), (None, None, initial_clock, 0));
            assert_eq!(published(&mut run), None);
            assert_eq!(run.frames, initial_clock, "no clock before the first target");
            for _ in 0..=SETTLE { run.look(true, || Some(VIEW)); }
            let first = published(&mut run);
            assert!(run.egg_at().is_some());
            assert_eq!(run.frames, if mode == TIMED { RUN_FRAMES - 1 } else { 1 });
            if mode == TIMED { assert_eq!(first, None); }
            else { assert!(first.unwrap().1.starts_with(if mode == TEN { "Eggs 0/10" } else { "Eggs 0/20" })); }
        }
    }
}

#[cfg(test)]
#[path = "egg_lifecycle_tests.rs"]
mod lifecycle_tests;
