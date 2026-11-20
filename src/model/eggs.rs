//! Eggs-ercise (rc.14, `artifacts/rc14-eggsercise/SPEC.md` §V2): break as many targets as you can in a minute, 10 or 20
//! as fast as you can, or without a timer in free practice. The target is one of the plugin's own pictures (D1 a), drawn
//! whether Hitbox Visualization is On or Off (`Run::look`) and broken when one of Player 1's live attack pills overlaps it;
//! nothing is written to the game. The runtime half is `runtime::features::hitbox::eggs_pass`.

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
/// Hide CPU's row (CPU page): with it On the CPU cannot be the target.
pub const ROW_HIDE_CPU: &str = "hide_cpu";
/// V2.6: `Hide` (the first option) hides the CPU while a practice game runs (`Run::running`); Hide CPU's rule reads it.
pub const ROW_PRACTICE_CPU: &str = "practice_cpu";
/// The standing notice (D5).
pub const NOTICE_ID: u8 = 0x2E;
pub const HEADER: &str = "Eggs-ercise";
/// The `eggsercise` row's options: the mode.
pub const OFF: u8 = 0;
pub const TIMED: u8 = 1;
/// `10 targets`, `20 targets` (SPEC V2.4): the clock counts up and stops at the Nth target (`goal`).
pub const TEN: u8 = 2;
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
/// Draw ticks the camera must hold still before a run's first target spawns: L+R+A sends it gliding to the new spot for a
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
/// `cpu::hide::step`). `practice_hide`: CPU during a game says Hide; `running`: an Eggs-ercise run is on (`Run::running`);
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

/// The run: armed by a menu close with the row on, until Off or the end of practice.
#[derive(Clone, Copy, Debug)]
pub struct Run {
    /// The mode, rule, speed, target and spawn area (`choices`) of the Start that began it.
    rows: [u8; 5],
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
    shown: Option<(u32, u32)>,
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
        still: 0,
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
        self.running() && !self.scanned
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
        self.floors = [None; FLOORS];
        for (slot, f) in self.floors.iter_mut().zip(floors) {
            *slot = Some(*f);
        }
        (self.scanned, self.pending) = (true, false);
    }

    /// Player 1's `top` joint this pass (each pass, before `step`).
    pub fn sense(&mut self, p1: [f32; 2]) {
        self.p1 = p1;
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

    /// V2.6: a practice game is running, from the menu close that starts a mode until Off or the scene is left (Time!, the
    /// end notice and the wait for a restart count).
    pub fn running(&self) -> bool {
        self.rows[0] != OFF
    }

    /// A run has the CPU as its target.
    pub fn cpu_target(&self) -> bool {
        self.rows[3] == CPU && self.running()
    }

    fn ended(&self) -> bool {
        match self.rows[0] {
            TIMED => self.frames == 0,
            _ => self.goal() > 0 && self.score >= self.goal(),
        }
    }

    /// The draw's tick on the practice's layout (`active`): a run that wants eggs takes the camera `look` reads (nothing
    /// is read otherwise); the pictures to draw, each at its spot of the fighters' plane: the target, then the break
    /// animations.
    pub fn look(&mut self, active: bool, look: impl FnOnce() -> Option<View>) -> [Look; 3] {
        if active && self.rows[0] != OFF && !self.ended() {
            let seen = look();
            self.still = if seen == self.view { self.still + 1 } else { 0 };
            self.view = seen;
        }
        let [a, b] = self.bursts.map(|b| b.map(burst));
        let whole = self.egg_at().filter(|_| self.rows[3] == DISC);
        [whole.map(|at| (at, EGG_WHOLE)), a, b]
    }

    /// A fresh run (the menu closed with the row on, or a reset): score 0, a full clock, the first egg once the camera has held
    /// still for `SETTLE` draws (a reset moves it, so the view from before is dropped, and the one right after is still on its
    /// way).
    pub fn start(&mut self, rows: [u8; 5], seed: u32) {
        let (rng, best) = ((self.rng ^ seed) | 1, self.best);
        *self = Run::IDLE;
        let frames = RUN_FRAMES * u32::from(rows[0] == TIMED);
        (self.rows, self.frames, self.rng, self.best) = (rows, frames, rng, best);
    }

    /// The game initialised the fighters again (L+R+A, Change Stage; the reset hub): an armed run starts over.
    pub fn reset(&mut self, seed: u32) {
        if self.rows[0] != OFF {
            self.start(self.rows, seed);
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
    /// None before the draw published the camera (a run's first one, with no `last`, waits for it to hold still).
    fn spawn(&mut self, last: Option<[f32; 2]>) -> Option<()> {
        let ready = (last.is_some() || self.still >= SETTLE) && !self.pending;
        let View(s, o, range, vertical) = self.view.filter(|_| ready)?;
        let edge = |i: usize, c: f32| (c * CANVAS_HALF[i] - o[i]) / s[i] - c * RADIUS;
        let (sx0, sx1, top, bottom) = if self.rows[4] == WHOLE {
            let [sx0, sx1] = [range[0] + RADIUS, range[1] - RADIUS];
            let (top, bottom) = match vertical {
                Some([lo, hi]) => (hi - RADIUS, lo + RADIUS),
                None => (edge(1, 1.0), edge(1, -1.0)),
            };
            (sx0, sx1, top, bottom)
        } else {
            let [sx0, sx1] = range.map(|v| v.max(edge(0, -1.0)).min(edge(0, 1.0)));
            (sx0, sx1, edge(1, 1.0), edge(1, -1.0))
        };
        let (x0, x1) = (sx0 + DRIFT, sx1 - DRIFT);
        // The disc's centre is `RADIUS` above its floor; the CPU target's feet are on it, and it flies the disc's flight
        // (the operator, 11-15): its feet trace the disc's bottom edge.
        let lift = [0.0, -RADIUS][usize::from(self.rows[3] == CPU)];
        let flat = [Some([[sx0, self.ground], [sx1, self.ground]])];
        let known = self.floors.iter().any(Option::is_some);
        let mut spans = [([0.0f32; 2], [[0.0f32; 2]; 2]); FLOORS];
        let mut total = 0.0;
        let floors = if known { &self.floors[..] } else { &flat[..] };
        let mut n = 0;
        for f in floors.iter().flatten() {
            let [a, b] = if f[0][0] <= f[1][0] { *f } else { [f[1], f[0]] };
            let (lo, hi) = ((a[0] + DRIFT).max(x0), (b[0] - DRIFT).min(x1));
            let height = a[1].max(b[1]) + RADIUS;
            if hi > lo && height >= bottom && height <= top {
                (spans[n], total, n) = (([lo, hi], [a, b]), total + hi - lo, n + 1);
            }
        }
        if n == 0 {
            // Nothing of the stage on screen, or no floor read: the run's ground across the screen.
            (spans[0], total, n) = (([x0, x1.max(x0)], flat[0]?), (x1 - x0).max(0.0), 1);
        }
        let spans = &spans[..n];
        let speed = SPEEDS[usize::from(self.rows[2].min(FAST))];
        let g = GRAVITY * speed * speed;
        // A gap that cannot push every target to the other side of a narrow view: at most a third of its floors' length (a
        // spot a third in from either end still has room on its own side), `MIN_GAP` where there is room.
        let spacing = MIN_GAP.min((total - 2.0 * P1_GAP).max(0.0) / 3.0);
        let body = [self.p1[0], self.p1[1] + BODY];
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
            ([x, on + RADIUS + air], air > 0.0, (2.0 * random() - 1.0) * DRIFT_SPEED * speed)
        };
        let tries = [(); 32].map(|_| pick());
        let dist = |p: [f32; 2], q: [f32; 2]| (p[0] - q[0]).hypot(p[1] - q[1]);
        let short = |disc: [f32; 2]| {
            let from_last = last.map_or(0.0, |l| spacing - dist([disc[0], disc[1] + lift], l));
            from_last.max(P1_GAP - dist(disc, body)).max(0.0)
        };
        let (disc, perch, vx) = tries.into_iter().min_by(|a, b| short(a.0).total_cmp(&short(b.0)))?;
        let launch = (LAUNCH * speed).min((2.0 * g * (top - disc[1]).max(0.0)).sqrt());
        (self.egg, self.age, self.fly, self.perch) = (Some([disc[0], disc[1] + lift]), 0, [launch, vx, g], perch);
        Some(())
    }

    /// Diag only (Eden's `diag-egg.req`): the target at `at`: the disc at rest there; the CPU target placed afresh there and
    /// launched straight up at the run's speed.
    #[cfg(feature = "diag")]
    pub fn place(&mut self, at: [f32; 2]) {
        let speed = SPEEDS[usize::from(self.rows[2].min(FAST))];
        let v = LAUNCH * speed * f32::from(u8::from(self.rows[3] == CPU));
        (self.egg, self.age, self.fly, self.cpu, self.perch) = (Some(at), 0, [v, 0.0, GRAVITY * speed * speed], 0, false);
    }

    /// Diag only: the target's spawn point, its ticks since and its flight (launch speed, sideways speed, gravity).
    #[cfg(feature = "diag")]
    pub fn flight(&self) -> (Option<[f32; 2]>, u32, [f32; 3]) {
        (self.egg, self.age, self.fly)
    }

    /// Where the target is now (the fighters' plane): up and back down in `2 v / g` ticks, one bounce at `BOUNCE` times the
    /// speed, then at rest on its spawn height.
    pub fn egg_at(&self) -> Option<[f32; 2]> {
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
    pub fn step(&mut self, tick: bool, y: f32, hits: &[Hit], air: bool, kind: i32) -> Option<u16> {
        self.step_with_depth(tick, y, hits, |_| None, air, kind).0
    }

    /// `step_with`, returning the deepest melee or article hit when this pass breaks a target.
    pub fn step_with_depth(
        &mut self,
        tick: bool,
        y: f32,
        hits: &[Hit],
        mut breaks: impl FnMut([f32; 2]) -> Option<Option<f32>>,
        air: bool,
        kind: i32,
    ) -> (Option<u16>, Option<f32>) {
        // The break animations run on the clock's ticks (pause freezes them); an ended run only finishes them.
        let dt = u32::from(tick);
        age(&mut self.bursts, tick);
        self.air = air;
        // A run the CPU target's last hit ended still holds its target until this pass closes it.
        if self.ended() && self.egg.is_none() {
            return (None, None);
        }
        if self.egg.is_none() {
            self.ground = y;
            self.spawn(None);
        }
        // The pass that closes a run the CPU target's last hit ended: the clock stopped on that hit's frame.
        let dt = dt * u32::from(!self.ended());
        let mut depth = None;
        if let Some(egg) = self.egg_at() {
            let melee = if self.rows[3] == DISC { deepest(self.rows[1], air, hits, egg) } else { None };
            let additional = if melee.is_none() && self.rows[3] == DISC { breaks(egg) } else { None };
            if melee.is_some() || additional.is_some() {
                (self.score, self.bursts, self.melee) = (self.score + 1, [Some((egg, 0)), self.bursts[0]], melee.is_some());
                depth = melee.or_else(|| additional.flatten());
                self.spawn(Some(egg));
            }
            // The flight holds while the CPU target is in a hit (its hitlag, then the placement): an uncounted hit puts it back
            // where it was hit, and a counted one's next target flies from its start.
            self.age += dt * u32::from(self.cpu < 3);
            match self.rows[0] {
                TIMED => self.frames -= dt,
                _ => self.frames += dt * u32::from(self.goal() > 0),
            }
            if self.ended() {
                let (k, m) = (kind as usize & 127, usize::from(self.rows[0] - TIMED));
                self.best[k][m] = self.best[k][m].max(if m == 0 { self.score } else { !self.frames });
                (self.best[128][m], self.egg) = (self.best[k][m], None);
                return (Some(END_FRAMES), depth);
            }
        }
        // The 1-minute notice changes once a second, the N-target one every tick (hundredths).
        let time = [self.frames, self.frames.div_ceil(60)][usize::from(self.rows[0] == TIMED)];
        let shown = (self.score, time);
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
    pub fn cpu_pass(&mut self, damage: f32, stopped: bool) -> Option<([f32; 2], Put)> {
        self.egg.filter(|_| self.cpu_target() && !self.ended())?;
        let fresh = match (self.cpu, damage > 0.0) {
            (2, true) => {
                let counts = counts(self.rows[1], self.air, damage);
                (self.cpu, self.score) = (3 + u8::from(!counts), self.score + u32::from(counts));
                return None;
            }
            (3 | 4, _) if stopped => return None,
            (3, _) => {
                // The next target, `MIN_GAP` from where this one was hit.
                self.spawn(self.egg_at());
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
        Some(([x, if aloft { y.max(ground + LIFT) } else { y }], put))
    }

    /// The notice (SPEC V2.4): `Eggs N   m:ss` (1 minute), `Eggs N/10   m:ss.cc` (10 or 20 targets), `Eggs N` (free
    /// practice); at the end `Time! N eggs (best M)` or `Done! 10 in m:ss.cc (best m:ss.cc)`.
    pub fn write_notice(&self, out: &mut impl core::fmt::Write) -> core::fmt::Result {
        let (score, goal, best) = (self.score, self.goal(), self.best[128]);
        let left = self.frames.div_ceil(60);
        // m:ss.cc of a frame count.
        let clock = |f: u32| (f * 5 / 3 / 6000, f * 5 / 3 / 100 % 60, f * 5 / 3 % 100);
        let (m, s, c) = clock(self.frames);
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
