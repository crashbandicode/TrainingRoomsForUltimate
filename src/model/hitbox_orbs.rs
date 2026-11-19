//! Own orbs: the hitbox visualizer's fighter spheres drawn by the plugin's own layout panes instead of the game's
//! `sys_shield` effects (decision HB-OWN-ORBS, option 1, operator 2026-11-05: "go with discs for all"; the A/B is
//! `docs/decisions/HB-OWN-ORBS-ab.png`). The shared effect pool refused part of a large fighter's spheres every pass; an
//! orb is not asked of that pool. A capsule is one pill (decision HB-PILLS, operator 2026-11-14): its two end spheres are
//! one orb, drawn as two half-disc caps and the band between them. The layout holds `panes::ORBS` (512) orb panes,
//! `panes::CAPS` caps and `panes::BANDS` bands, 170 pills. When they run out the cut is deterministic and whole pills go,
//! hitboxes and grab boxes before hurtboxes (`Board::in_draw_order`), and the layout tick says so once (review HB-2). An article's hitboxes are own orbs
//! too (rc.13, `Articles`): the game's effect sphere on a projectile lived its whole flight but never showed.
//!
//! The same `HandleSet` keeps the spheres; `OrbSink` is its `Effects` for own orbs: a request always succeeds and a held
//! orb lives while the fighter's status is the one it was made in (what the game's `CONCLUDE_STATUS` did to an effect).
//! The pass turns each held sphere into an `Orb` in world space (the bone's transform, a game call) and publishes the
//! fighter's orbs on the `Board`; the layout tick projects them with the game's own `world_to_screen` and plans each
//! orb's pill (`plan_pill`). Pure and host-tested (`tests/core06_orbs.rs`).

use super::hitbox::{want_hits, AttackReader, Effects, HandleSet, Sphere, FIGHTER_SETS, KIND_ARTICLE_HURT, KIND_HIT, KIND_HURT, MAX_HELD, SCALE, SLOTS};
use super::legend::{self, KIND_MARK, KIND_WIND};

/// One fighter's orbs on the board: its spheres' and, with *Launch angles* On, up to two marks per hitbox, and with *Windboxes* its wind
/// areas (legend::area_orbs: a shape, the line's shadow and the line each).
pub const BOARD_ORBS: usize = MAX_HELD + 2 * SLOTS as usize + 3 * legend::area::MAX_AREAS;

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

    fn sphere_spawn(&mut self, _sphere: &Sphere, made_in: Option<i32>) -> u32 {
        orb_handle(made_in.unwrap_or(self.status))
    }
}

/// One orb in world space, with the pane colour it is drawn in (linear-light RGBA, as the layout takes vertex colours):
/// a pill from `centre` to `tail` (the same point for a point hitbox).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orb {
    pub centre: [f32; 3],
    pub tail: [f32; 3],
    pub radius: f32,
    pub color: [u8; 4],
    /// The first sphere's `hitbox::sphere_key`.
    pub key: u32,
}

impl Orb {
    pub const NONE: Orb = Orb { centre: [0.0; 3], tail: [0.0; 3], radius: 0.0, color: [0; 4], key: 0 };

    /// `hitbox::KIND_HIT`, `KIND_HURT` or `KIND_GRAB`: the cut order when the panes run out.
    pub fn kind(&self) -> u32 {
        self.key >> 24
    }
}

/// The opacity of an orb pane's colour; the disc texture carries the shading.
pub const ORB_ALPHA: u8 = 255;

/// Exact sRGB bytes shared by spheres, wind areas and the menu colour key.
pub fn tint_colour(tint: [f32; 3], alpha: u8) -> [u8; 4] {
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    [byte(tint[0]), byte(tint[1]), byte(tint[2]), alpha]
}

/// The orb a held sphere becomes: `centre` is the bone's transform of the sphere's offset (the runtime's game call); the
/// radius is the hitbox size (route B's effect scale is `size x SCALE`).
pub fn orb_of(sphere: &Sphere, centre: [f32; 3]) -> Orb {
    Orb { centre, tail: centre, radius: sphere.scale / SCALE, color: super::panes::linear(tint_colour(sphere.tint, ORB_ALPHA)), key: sphere.key }
}

/// Publishes a fighter has missed before its orbs stop being drawn (other fighters' publishes count; a pause publishes
/// nothing, so the orbs of a paused scene stay).
pub const STALE_PUBLISHES: u32 = 2 * FIGHTER_SETS as u32;

/// Every fighter's orbs of its last pass: fixed storage, one slot per fighter (by accessor address).
pub struct Board {
    keys: [usize; FIGHTER_SETS],
    stamps: [u32; FIGHTER_SETS],
    lens: [usize; FIGHTER_SETS],
    orbs: [[Orb; BOARD_ORBS]; FIGHTER_SETS],
    clock: u32,
}

impl Default for Board {
    fn default() -> Self {
        Board::new()
    }
}

impl Board {
    pub const fn new() -> Board {
        Board { keys: [0; FIGHTER_SETS], stamps: [0; FIGHTER_SETS], lens: [0; FIGHTER_SETS], orbs: [[Orb::NONE; BOARD_ORBS]; FIGHTER_SETS], clock: 0 }
    }

    /// Replace `key`'s orbs (at most `BOARD_ORBS`; a new key takes a free or the stalest slot). A capsule's second end
    /// (sphere 1 of its shape) becomes the tail of its first (sphere 0). Returns how many were kept.
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
        for orb in orbs.take(BOARD_ORBS) {
            self.orbs[i][len] = orb;
            len += 1;
        }
        // The set keeps no order (it removes by swapping), so each second end looks for its first anywhere.
        let slot = &mut self.orbs[i];
        let mut k = 0;
        while k < len {
            let end = slot[k];
            match slot[..len].iter().position(|o| end.key & 0xff == 1 && o.key == end.key - 1) {
                Some(j) => {
                    slot[j].tail = end.centre;
                    len -= 1;
                    slot[k] = slot[len];
                }
                None => k += 1,
            }
        }
        self.lens[i] = len;
        len
    }

    pub fn publish_owned(&mut self, key: usize, _owner: usize, orbs: impl Iterator<Item = Orb>) -> usize {
        self.publish(key, orbs)
    }

    pub fn set_hidden(&mut self, _accessors: &[usize]) {}

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

    /// The live orbs in the order the panes take them: every hitbox and grab box first, then every hurtbox, then the launch-angle
    /// marks, each in slot order. When the panes run out the cut is deterministic and takes the marks, then hurtboxes first (review HB-2).
    pub fn in_draw_order(&self) -> impl Iterator<Item = &Orb> + '_ {
        let of = move |kind: fn(u32) -> bool| self.live().filter(move |o| kind(o.kind()));
        of(|k| k != KIND_HURT && k != KIND_MARK && k != KIND_ARTICLE_HURT).chain(of(|k| k == KIND_HURT)).chain(of(|k| k == KIND_MARK)).chain(of(|k| k == KIND_ARTICLE_HURT))
    }
}

/// `set`'s spheres as orbs on their bones (`place`) into `out`, then, while *Launch angles* is On (`lr`: the owner's facing), each
/// hitbox's (and windbox's) marks (`legend::marks`) at its slot's angle (`angle(slot)`, `None` when the slot has no data). How many were written.
pub fn place_orbs(set: &HandleSet, mut place: impl FnMut(&Sphere) -> Option<[f32; 3]>, lr: Option<f32>, mut angle: impl FnMut(i32) -> Option<i32>, out: &mut [Orb]) -> usize {
    let mut n = 0;
    for (slot, orb) in out.iter_mut().zip(set.spheres().filter_map(|s| place(s).map(|c| orb_of(s, c)))) {
        (*slot, n) = (orb, n + 1);
    }
    for i in 0..lr.map_or(0, |_| n) {
        let hit = out[i];
        let Some(angle) = (matches!(hit.kind(), KIND_HIT | KIND_WIND) && hit.key & 0xff == 0).then(|| angle(((hit.key >> 8) & 0xffff) as i32)).flatten() else { continue };
        for mark in legend::marks(&hit, angle, lr.unwrap_or(1.0)).into_iter().flatten().take(out.len() - n) {
            (out[n], n) = (mark, n + 1);
        }
    }
    n
}

/// A slot's launch angle (`legend::ANGLE`, an i32 read through the reader's 4-byte read), or `None` when it has no data.
pub fn slot_angle(attacks: &mut impl AttackReader, slot: i32) -> Option<i32> {
    let data = attacks.data(slot);
    (data != 0).then(|| attacks.read_f32(data + legend::ANGLE).to_bits() as i32)
}


/// Articles drawn at once: the board's slots past Training's fighter objects.
pub const ARTICLES: usize = 4;

/// The articles whose `ATTACK` or `CATCH` ran (rc.13): (battle-object id, accessor), the accessor the board key of its
/// orbs, and its spheres (a `CATCH`'s wait there with their status, as a fighter's do: Olimar's Pikmin); the oldest is
/// replaced.
#[derive(Default)]
pub struct Articles {
    ids: [(u32, usize); ARTICLES],
    ages: [u32; ARTICLES],
    clock: u32,
    sets: [HandleSet; ARTICLES],
    attacking: [bool; ARTICLES],
}

impl Articles {
    pub const fn new() -> Articles {
        Articles { ids: [(0, 0); ARTICLES], ages: [0; ARTICLES], clock: 0, sets: [HandleSet::new(); ARTICLES], attacking: [false; ARTICLES] }
    }

    /// Track an object and its priority. Hurt-only discovery takes free slots; attacks evict passive objects before the oldest attack.
    pub fn track(&mut self, id: u32, key: usize, board: &mut Board, attacking: bool) -> Option<(&mut HandleSet, &mut bool)> {
        let i = (0..ARTICLES).min_by_key(|&i| (self.ids[i] != (id, key), self.ids[i].0 != 0, self.attacking[i], self.ages[i])).unwrap_or(0);
        if !attacking && self.ids[i] != (id, key) && self.ids[i].0 != 0 { return None; }
        if self.ids[i] != (id, key) {
            self.clock = self.clock.wrapping_add(1);
            board.clear(self.ids[i].1);
            (self.ids[i], self.ages[i]) = ((id, key), self.clock);
            self.sets[i].forget();
            self.attacking[i] = false;
        }
        self.attacking[i] |= attacking;
        Some((&mut self.sets[i], &mut self.attacking[i]))
    }

    pub fn tracked(&self) -> [(u32, usize); ARTICLES] {
        self.ids
    }

    pub fn forget(&mut self, id: u32, board: &mut Board) {
        for slot in self.ids.iter_mut().filter(|t| t.0 == id) {
            board.clear(slot.1);
            *slot = (0, 0);
        }
    }

    pub fn forget_all(&mut self, board: &mut Board) {
        for (id, _) in self.ids {
            self.forget(id, board);
        }
    }
}

/// What an article's pass draws besides its spheres: the launch-angle marks while *Launch angles* is On (`lr`: the article's
/// facing) and its wind areas while *Windboxes* is ticked.
#[derive(Clone, Copy, Default)]
pub struct Extras<'a> {
    pub lr: Option<f32>,
    pub areas: &'a [legend::WindArea],
    pub hurt: &'a [Sphere],
}

impl Extras<'_> {
    pub const NONE: Extras<'static> = Extras { lr: None, areas: &[], hurt: &[] };
}

/// One article's pass in `status`: its live attack slots' spheres and its grab spheres (held as a fighter's), placed on its
/// bones by `place`, become its orbs on `board`, followed by its wind `areas` (rc.14 wind-articles: read from its own
/// AreaModule while *Windboxes* is ticked). False when it holds none and has no area: its attack, grab and wind ended, its orbs
/// went, and it is forgotten (`legend::keeps_article`).
pub fn article_pass(board: &mut Board, set: &mut HandleSet, key: usize, status: i32, attacks: &mut impl AttackReader, place: impl FnMut(&Sphere) -> Option<[f32; 3]>, extras: Extras<'_>) -> bool {
    let Extras { lr, areas, hurt } = extras;
    let mut wants = [Sphere::NONE; MAX_HELD];
    let mut len = 0;
    want_hits(attacks, &mut wants, &mut len, true);
    set.tint(attacks, &mut wants[..len], status);
    let n = hurt.len().min(wants.len() - len);
    wants[len..len + n].copy_from_slice(&hurt[..n]);
    len += n;
    set.reconcile(&wants[..len], true, &mut OrbSink { status });
    let mut orbs = [Orb::NONE; BOARD_ORBS];
    let mut n = place_orbs(set, place, lr, |slot| slot_angle(attacks, slot), &mut orbs);
    // Its wind areas after its spheres, as a fighter's pass publishes its own (the runtime's `publish_orbs`).
    let colour = legend::area_colour();
    for orb in areas.iter().enumerate().flat_map(|(k, area)| legend::area_orbs(area, k, 0.0, colour)).flatten() {
        if n < orbs.len() {
            (orbs[n], n) = (orb, n + 1);
        }
    }
    board.publish(key, orbs[..n].iter().copied());
    legend::keeps_article(!set.is_empty(), areas.len())
}

/// The panes one layout tick has taken: a pill takes two caps and, when it has a length, a band; one that does not get
/// every pane it needs is cut whole.
#[derive(Clone, Copy, Debug, Default)]
pub struct PaneUse {
    pub caps: usize,
    pub bands: usize,
}

impl PaneUse {
    /// A band alone (a wind area's box: no round ends), or `None` (cut).
    pub fn take_band(&mut self) -> Option<usize> {
        (self.bands < super::panes::BANDS).then(|| {
            self.bands += 1;
            self.bands - 1
        })
    }

    /// The first of the pill's two caps and its band, or `None` (cut).
    pub fn take(&mut self, band: bool) -> Option<(usize, Option<usize>)> {
        if self.caps + 2 > super::panes::CAPS || (band && self.bands >= super::panes::BANDS) {
            return None;
        }
        let at = (self.caps, band.then_some(self.bands));
        self.caps += 2;
        self.bands += usize::from(band);
        Some(at)
    }
}

/// One pool's writes of one layout tick (review HB-1; the caps and the bands each): `drawn` panes were written, `0..drawn`, and
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

/// One orb's pill in canvas units: the two caps' centres (each a radius wide, its flat side on its end; cap 0 faces away
/// from end 1), the band's centre and length between the ends, the pill's width (`size`, a diameter), its rotation (degrees
/// counter-clockwise, the axis from end 0 to end 1; cap 0 turns by it, cap 1 by it plus 180) and its colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pill {
    pub caps: [[f32; 2]; 2],
    pub band: [f32; 2],
    pub length: f32,
    pub size: f32,
    pub rot: f32,
    pub color: [u8; 4],
}

/// Plan an orb's pill from the screen projections of its two ends and, for each end, of two points one radius from it
/// (`edge_points`): each end's radius is its longer projected one and the pill's is their mean. `None` when a projection
/// is not finite, the pill is under a canvas unit across, or it lies wholly off the canvas.
pub fn plan_pill(ends: [[f32; 2]; 2], edges: [[[f32; 2]; 2]; 2], half: [f32; 2], color: [u8; 4]) -> Option<Pill> {
    if half[0] <= 0.0 || half[1] <= 0.0 {
        return None;
    }
    let [a, b] = ends.map(|e| to_canvas(e, half));
    let radius = |c: [f32; 2], edges: [[f32; 2]; 2]| edges.map(|e| to_canvas(e, half)).iter().fold(0.0f32, |r, e| r.max((e[0] - c[0]).hypot(e[1] - c[1])));
    let size = radius(a, edges[0]) + radius(b, edges[1]);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length = dx.hypot(dy);
    if !(length.is_finite() && size.is_finite() && a[0].is_finite() && a[1].is_finite()) || size < 1.0 {
        return None;
    }
    let size = size.min(MAX_PANE);
    let off = |i: usize| a[i].min(b[i]) - size / 2.0 > CANVAS_HALF[i] || a[i].max(b[i]) + size / 2.0 < -CANVAS_HALF[i];
    if off(0) || off(1) {
        return None;
    }
    let (ux, uy) = if length > 0.0 { (dx / length, dy / length) } else { (1.0, 0.0) };
    let q = size / 4.0;
    Some(Pill {
        caps: [[a[0] - ux * q, a[1] - uy * q], [b[0] + ux * q, b[1] + uy * q]],
        band: [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0],
        length,
        size,
        rot: uy.atan2(ux).to_degrees(),
        color,
    })
}

/// The two world points one radius from `centre` that `plan_pill` takes an end's projected radius from.
pub fn edge_points(centre: [f32; 3], radius: f32) -> [[f32; 3]; 2] {
    let [x, y, z] = centre;
    [[x + radius, y, z], [x, y + radius, z]]
}
