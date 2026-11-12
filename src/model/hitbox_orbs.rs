//! Own orbs: the hitbox visualizer's fighter spheres drawn by the plugin's own layout panes instead of the game's
//! `sys_shield` effects (decision HB-OWN-ORBS, option 1, operator 2026-11-05: "go with discs for all"; the A/B is
//! `docs/decisions/HB-OWN-ORBS-ab.png`). The shared effect pool refused part of a large fighter's spheres every pass; an
//! orb is not asked of that pool. The layout holds `panes::ORBS` (512) orb panes, Training's most fighter objects
//! (`ORB_FIGHTERS`) times the spheres one fighter holds (`MAX_HELD`). More orbs than panes happen only while a gone
//! fighter's last pass is still on the board (a few frames): then the cut is deterministic, hitboxes and grab boxes before
//! hurtboxes (`Board::in_draw_order`), and the layout tick says so once (review HB-2). Articles keep the effect spheres.
//!
//! The same `HandleSet` keeps the spheres; `OrbSink` is its `Effects` for own orbs: a request always succeeds and a held
//! orb lives while the fighter's status is the one it was made in (what the game's `CONCLUDE_STATUS` did to an effect).
//! The pass turns each held sphere into an `Orb` in world space (the bone's transform, a game call) and publishes the
//! fighter's orbs on the `Board`; the layout tick projects them with the game's own `world_to_screen` and plans one pane
//! per orb (`plan_pane`). Pure and host-tested (`tests/core06_orbs.rs`).

use super::hitbox::{Effects, Sphere, FIGHTER_SETS, KIND_HURT, MAX_HELD, SCALE};

/// The most fighter objects Training (and the waiting room's practice) draws at once: Player 1 and the CPU, each of them
/// possibly Ice Climbers, whose Nana is a fighter object of her own. No other fighter adds a fighter object (Pokémon
/// Trainer's trainer, Pyra and Mythra's partner, summons and items are articles or swap in place).
#[cfg(test)]
pub const ORB_FIGHTERS: usize = 4;

/// An own orb's handle: non-zero, and naming the status the orb was made in.
pub const fn orb_handle(status: i32) -> u32 {
    0x8000_0000 | (status as u32 & 0x7fff_ffff)
}

/// `Effects` for own orbs: nothing is asked of the game. `status` is the fighter's status this pass.
pub struct OrbSink {
    pub status: i32,
}

impl Effects for OrbSink {
    fn sphere_alive(&mut self, handle: u32) -> bool {
        handle == orb_handle(self.status)
    }

    fn sphere_kill_own(&mut self, _handle: u32) {}

    fn sphere_spawn(&mut self, _sphere: &Sphere) -> u32 {
        orb_handle(self.status)
    }
}

/// One orb in world space, with the pane colour it is drawn in (linear-light RGBA, as the layout takes vertex colours).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orb {
    pub centre: [f32; 3],
    pub radius: f32,
    pub color: [u8; 4],
    /// The sphere's kind (`hitbox::KIND_HIT`, `KIND_HURT`, `KIND_GRAB`): the cut order when the panes run out.
    pub kind: u32,
}

impl Orb {
    pub const NONE: Orb = Orb { centre: [0.0; 3], radius: 0.0, color: [0; 4], kind: 0 };
}

/// The opacity of an orb pane's colour; the disc texture carries the shading.
pub const ORB_ALPHA: u8 = 255;

/// The orb a held sphere becomes: `centre` is the bone's transform of the sphere's offset (the runtime's game call); the
/// radius is the hitbox size (route B's effect scale is `size x SCALE`).
pub fn orb_of(sphere: &Sphere, centre: [f32; 3]) -> Orb {
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    let srgb = [byte(sphere.tint[0]), byte(sphere.tint[1]), byte(sphere.tint[2]), ORB_ALPHA];
    Orb { centre, radius: sphere.scale / SCALE, color: super::panes::linear(srgb), kind: sphere.key >> 24 }
}

/// Publishes a fighter has missed before its orbs stop being drawn (other fighters' publishes count; a pause publishes
/// nothing, so the orbs of a paused scene stay).
pub const STALE_PUBLISHES: u32 = 2 * FIGHTER_SETS as u32;

/// Every fighter's orbs of its last pass: fixed storage, one slot per fighter (by accessor address).
pub struct Board {
    keys: [usize; FIGHTER_SETS],
    stamps: [u32; FIGHTER_SETS],
    lens: [usize; FIGHTER_SETS],
    orbs: [[Orb; MAX_HELD]; FIGHTER_SETS],
    clock: u32,
}

impl Default for Board {
    fn default() -> Self {
        Board::new()
    }
}

impl Board {
    pub const fn new() -> Board {
        Board { keys: [0; FIGHTER_SETS], stamps: [0; FIGHTER_SETS], lens: [0; FIGHTER_SETS], orbs: [[Orb::NONE; MAX_HELD]; FIGHTER_SETS], clock: 0 }
    }

    /// Replace `key`'s orbs (at most `MAX_HELD`; a new key takes a free or the stalest slot). Returns how many were kept.
    pub fn publish(&mut self, key: usize, orbs: impl Iterator<Item = Orb>) -> usize {
        if key == 0 {
            return 0;
        }
        self.clock = self.clock.wrapping_add(1);
        let i = match self.keys.iter().position(|k| *k == key) {
            Some(i) => i,
            None => (0..FIGHTER_SETS)
                .min_by_key(|&i| (self.keys[i] != 0 && self.lens[i] != 0, core::cmp::Reverse(self.clock.wrapping_sub(self.stamps[i]))))
                .unwrap_or(0),
        };
        self.keys[i] = key;
        self.stamps[i] = self.clock;
        let mut len = 0;
        for orb in orbs.take(MAX_HELD) {
            self.orbs[i][len] = orb;
            len += 1;
        }
        self.lens[i] = len;
        len
    }

    /// `key` has no orbs now.
    pub fn clear(&mut self, key: usize) {
        if let Some(i) = self.keys.iter().position(|k| *k == key && key != 0) {
            self.keys[i] = 0;
            self.lens[i] = 0;
        }
    }

    #[cfg(test)]
    pub fn clear_all(&mut self) {
        self.keys = [0; FIGHTER_SETS];
        self.lens = [0; FIGHTER_SETS];
    }

    /// The orbs to draw: every slot published within the last `STALE_PUBLISHES` publishes.
    pub fn live(&self) -> impl Iterator<Item = &Orb> + '_ {
        (0..FIGHTER_SETS)
            .filter(move |&i| self.keys[i] != 0 && self.clock.wrapping_sub(self.stamps[i]) < STALE_PUBLISHES)
            .flat_map(move |i| self.orbs[i][..self.lens[i]].iter())
    }

    /// The live orbs in the order the panes take them: every hitbox and grab box first, then every hurtbox, each in slot
    /// order. When the panes run out the cut is deterministic and takes hurtboxes first (review HB-2).
    pub fn in_draw_order(&self) -> impl Iterator<Item = &Orb> + '_ {
        let first = self.live().filter(|o| o.kind != KIND_HURT);
        first.chain(self.live().filter(|o| o.kind == KIND_HURT))
    }
}

/// The orb panes' writes of one layout tick (review HB-1): `drawn` orbs were written into the panes `0..drawn`, and
/// `shown` panes showed an orb after the last tick. The panes `hide.0..hide.1` that showed one and do not now are hidden,
/// and the container shows only while one orb does. An inactive tick (the waiting room's practice ended while its
/// layout stays, or Training's layout outside Training) draws nothing, so everything shown is hidden; `None` when there
/// is nothing to write (nothing shown before or now).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneTick {
    pub hide: (usize, usize),
    pub container: bool,
    pub shown: usize,
}

// Kept out of line: the follow-up Eden rows arm its entry (x0 = drawn, x1 = shown) on the release NRO.
#[inline(never)]
pub fn pane_tick(drawn: usize, shown: usize) -> Option<PaneTick> {
    if drawn == 0 && shown == 0 {
        return None;
    }
    Some(PaneTick { hide: (drawn, shown.max(drawn)), container: drawn != 0, shown: drawn })
}

// ---- projection to the layout ---------------------------------------------------------------------------------------

/// The layout canvas (`panes`): 1920 x 1080, origin at the centre, y up.
pub const CANVAS_HALF: [f32; 2] = [960.0, 540.0];
/// The half screen size the game's `world_to_screen` falls back to when its size words are 0 (`main+0x44728b8`,
/// `+0x44724c4`).
pub const SCREEN_HALF_DEFAULT: [f32; 2] = [960.0, 540.0];
/// The largest orb pane (a sphere at the camera would otherwise cover the screen).
pub const MAX_PANE: f32 = 1200.0;

/// The half screen extent `world_to_screen` maps with: the size word truncated to an unsigned integer and halved, or the
/// default when the word is 0 (`main+0x13415f0..0x1341638`: `fcvtzu`, `ucvtf`, `* 0.5`, `fcsel` on `!= 0`).
pub fn screen_half(raw: f32, default: f32) -> f32 {
    if raw == 0.0 {
        default
    } else {
        (if raw.is_nan() || raw <= 0.0 { 0.0 } else { raw.min(u32::MAX as f32) as u32 as f32 }) * 0.5
    }
}

/// A screen point (pixels, y down, from `world_to_screen`) on the layout canvas (y up, origin at the centre).
pub fn to_canvas(point: [f32; 2], half: [f32; 2]) -> [f32; 2] {
    [(point[0] - half[0]) / half[0] * CANVAS_HALF[0], (half[1] - point[1]) / half[1] * CANVAS_HALF[1]]
}

/// One orb's pane: centre and diameter in canvas units, and its colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbPane {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: [u8; 4],
}

/// Plan an orb's pane from the screen projections of its centre and of two points one radius from it (along world x and
/// y): the diameter is twice the longer projected radius. `None` when a projection is not finite, the orb is under a
/// canvas unit across, or it lies wholly off the canvas.
pub fn plan_pane(centre: [f32; 2], edges: [[f32; 2]; 2], half: [f32; 2], color: [u8; 4]) -> Option<OrbPane> {
    if half[0] <= 0.0 || half[1] <= 0.0 {
        return None;
    }
    let c = to_canvas(centre, half);
    let mut radius = 0.0f32;
    for edge in edges {
        let e = to_canvas(edge, half);
        let (dx, dy) = (e[0] - c[0], e[1] - c[1]);
        radius = radius.max((dx * dx + dy * dy).sqrt());
    }
    let size = 2.0 * radius;
    if !(c[0].is_finite() && c[1].is_finite() && size.is_finite()) || size < 1.0 {
        return None;
    }
    let size = size.min(MAX_PANE);
    if c[0].abs() - size / 2.0 > CANVAS_HALF[0] || c[1].abs() - size / 2.0 > CANVAS_HALF[1] {
        return None;
    }
    Some(OrbPane { x: c[0], y: c[1], size, color })
}

/// The two world points one radius from `orb`'s centre that `plan_pane` takes the projected radius from.
pub fn edge_points(orb: &Orb) -> [[f32; 3]; 2] {
    let [x, y, z] = orb.centre;
    [[x + orb.radius, y, z], [x, y + orb.radius, z]]
}
