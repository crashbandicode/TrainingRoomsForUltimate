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

use super::cpu::role::FighterKey;
use super::hitbox::{self, want_hits, AttackReader, Counts, Effects, FighterGenerations, FighterLife, HandleSet, SetTable, Shape, Sphere, FIGHTER_SETS, KIND_ARTICLE_HURT, KIND_HIT, KIND_HURT, MAX_HELD, SCALE, SLOTS};
use super::legend::{self, KIND_MARK, KIND_WIND};
use super::handoff::{Handoff, Record};

/// What telling a set its waiting command did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Applied {
    /// The grab spheres the set took; `None` when the fighter's lifetime had ended.
    Grab(Option<u32>),
    /// What clearing the fighter's set did and the spheres every fighter holds afterwards; `None` when its lifetime had ended.
    Rebound(Option<(Counts, usize)>),
}

/// Queue a CATCH's spheres in `set` with the status the command ran in: how many the set took.
pub fn push_grabs(set: &mut HandleSet, shape: &Shape, status: i32) -> u32 {
    let mut wants = [Sphere::NONE; hitbox::SHAPE_SPHERES];
    let mut len = 0;
    hitbox::push_shape(&mut wants, &mut len, hitbox::KIND_GRAB, shape.id as usize, shape, hitbox::legend_colour(hitbox::KIND_GRAB, None, 0));
    wants[..len].iter().filter(|w| set.push_grab(**w, Some(status))).count() as u32
}

/// Phase 8, a linked table: adopt what the command hooks changed, then tell each fighter's set its waiting commands, oldest
/// first: how many.
pub fn apply_fighters(table: &mut SetTable, generations: &FighterGenerations, each: &mut dyn FnMut(&Record, Applied)) -> usize {
    table.settle();
    let mut count = 0;
    for i in 0..FIGHTER_SETS {
        while let Some(record) = table.next_waiting(i) {
            let applied = match record {
                Record::Grab { life: Some(life), shape, status, .. } => {
                    Applied::Grab(generations.is_current(life).then(|| push_grabs(&mut table.sets[i], &shape, status)))
                }
                Record::Rebound { life, .. } => Applied::Rebound(generations.is_current(life).then(|| {
                    let counts = table.sets[i].clear(&mut OrbSink { status: i32::MIN });
                    (counts, table.held_spheres())
                })),
                Record::Grab { life: None, .. } => continue,
            };
            each(&record, applied);
            count += 1;
        }
    }
    count
}

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

pub const HIDDEN_WORDS: usize = 2 * FighterKey::COUNT;

struct HiddenFrame {
    generation: u32,
    words: [u64; HIDDEN_WORDS],
}

/// The hide state when a frame is built. A failed bounded read must not reuse a previous frame's ownership.
pub struct DrawHide(Result<Option<HiddenFrame>, ()>);

impl DrawHide {
    pub const VISIBLE: Self = Self(Ok(None));

    pub fn sampled(read: Option<(u32, [u64; HIDDEN_WORDS])>) -> Self {
        Self(read.map(|(generation, words)| Some(HiddenFrame { generation, words })).ok_or(()))
    }

    pub fn generation(&self) -> Option<u32> {
        self.0.as_ref().ok()?.as_ref().map(|frame| frame.generation)
    }

    /// The article reader runs only for a coherent hidden generation.
    pub fn owner(&self, read: impl FnOnce() -> Option<u32>) -> Option<u32> {
        self.generation().and_then(|_| read())
    }
}

/// The latest orbs of each object, keyed by its live accessor.
pub struct Board {
    keys: [usize; FIGHTER_SETS],
    fighter_lives: [Option<FighterLife>; FIGHTER_SETS],
    article_ids: [Option<u32>; FIGHTER_SETS],
    owners: [Option<u32>; FIGHTER_SETS],
    article_generation: Option<u32>,
    stamps: [u32; FIGHTER_SETS],
    lens: [usize; FIGHTER_SETS],
    orbs: [[Orb; BOARD_ORBS]; FIGHTER_SETS],
    clock: u32,
    frame: Option<u32>,
}

impl Default for Board {
    fn default() -> Self {
        Board::new()
    }
}

impl Board {
    pub const fn new() -> Board {
        Board { keys: [0; FIGHTER_SETS], fighter_lives: [None; FIGHTER_SETS], article_ids: [None; FIGHTER_SETS], owners: [None; FIGHTER_SETS], article_generation: None, stamps: [0; FIGHTER_SETS], lens: [0; FIGHTER_SETS], orbs: [[Orb::NONE; BOARD_ORBS]; FIGHTER_SETS], clock: 0, frame: None }
    }

    /// Start a complete sample. Each producer must republish its visible geometry.
    pub fn begin_frame(&mut self, frame: u32) {
        self.lens.fill(0);
        self.article_generation = None;
        self.frame = Some(frame);
    }

    /// A failed sample may leave the previous board in memory.
    pub fn is_frame(&self, frame: u32) -> bool {
        self.frame == Some(frame)
    }

    /// Replace `key`'s orbs (at most `BOARD_ORBS`; a new key takes a free or the stalest slot). A capsule's second end
    /// (sphere 1 of its shape) becomes the tail of its first (sphere 0). Returns how many were kept.
    #[cfg(test)]
    pub fn publish(&mut self, key: usize, orbs: impl Iterator<Item = Orb>) -> usize {
        self.publish_object(key, None, None, None, orbs)
    }

    pub fn publish_fighter(&mut self, key: usize, life: FighterLife, orbs: impl Iterator<Item = Orb>) -> usize {
        self.publish_object(key, Some(life), None, None, orbs)
    }

    fn publish_object(&mut self, key: usize, fighter: Option<FighterLife>, article: Option<u32>, owner: Option<u32>, orbs: impl Iterator<Item = Orb>) -> usize {
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
        self.fighter_lives[i] = fighter;
        self.article_ids[i] = article;
        self.owners[i] = owner;
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

    pub(crate) fn publish_article(&mut self, key: usize, id: u32, owner: Option<u32>, orbs: impl Iterator<Item = Orb>) -> usize {
        self.publish_object(key, None, Some(id), owner, orbs)
    }

    /// Call after the complete article pass, under the same guard as its publications.
    pub fn finish_articles(&mut self, generation: Option<u32>) {
        self.article_generation = generation;
    }

    /// Animation hooks can replace tracking entries even when the renderer holds the board.
    fn clear_evicted(&mut self, tracked: &[(u32, usize); ARTICLES]) {
        for i in 0..FIGHTER_SETS {
            if self.article_ids[i].is_some_and(|id| !tracked.contains(&(id, self.keys[i]))) {
                self.keys[i] = 0;
                self.lens[i] = 0;
                self.article_ids[i] = None;
            }
        }
    }

    /// `key` has no orbs now.
    pub fn clear(&mut self, key: usize) {
        if let Some(i) = self.keys.iter().position(|k| *k == key && key != 0) {
            self.keys[i] = 0;
            self.lens[i] = 0;
            self.article_ids[i] = None;
        }
    }

    /// Clear only this fighter lifetime's publication of the named sample.
    pub fn clear_publication(&mut self, key: usize, life: FighterLife, frame: u32) {
        if !self.is_frame(frame) { return; }
        if let Some(i) = self.keys.iter().enumerate().position(|(i, k)| *k == key && key != 0 && self.fighter_lives[i] == Some(life)) {
            self.keys[i] = 0;
            self.lens[i] = 0;
            self.article_ids[i] = None;
        }
    }

    #[cfg(test)]
    pub fn clear_all(&mut self) {
        self.keys = [0; FIGHTER_SETS];
        self.lens = [0; FIGHTER_SETS];
        self.article_ids = [None; FIGHTER_SETS];
    }

    /// Resolve visibility once per frame. In the visible path none of the four passes scans IDs.
    pub fn draw_slots(&self, hide: &DrawHide, generations: &FighterGenerations) -> u8 {
        let visible = self.draw_slots_with(hide, |words, token| words.contains(&token));
        self.fighter_lives.iter().enumerate().fold(visible, |mask, (i, life)| {
            if self.lens[i] != 0 && life.is_some_and(|life| !generations.is_current(life)) {
                mask & !(1 << i)
            } else {
                mask
            }
        })
    }

    pub(crate) fn draw_slots_with(&self, hide: &DrawHide, mut contains: impl FnMut(&[u64], u64) -> bool) -> u8 {
        let frame = match &hide.0 {
            Ok(None) => return u8::MAX,
            Err(()) => return 0,
            Ok(Some(frame)) => frame,
        };
        let (accessors, ids) = frame.words.split_at(FighterKey::COUNT);
        (0..FIGHTER_SETS).fold(0, |mask, i| {
            if self.keys[i] == 0 {
                return mask;
            }
            let visible = if self.article_ids[i].is_some() {
                // A draw racing the first hidden fighter pass cannot use an earlier ownership sample.
                self.article_generation == Some(frame.generation)
                    && self.owners[i].is_none_or(|owner| !contains(ids, u64::from(owner) + 1))
            } else {
                !contains(accessors, self.keys[i] as u64)
            };
            mask | (u8::from(visible) << i)
        })
    }

    /// Every selected slot published within the last `STALE_PUBLISHES` publishes.
    fn live_slots(&self, slots: u8) -> impl Iterator<Item = &Orb> + '_ {
        (0..FIGHTER_SETS)
            .filter(move |&i| slots & (1 << i) != 0 && self.keys[i] != 0 && self.clock.wrapping_sub(self.stamps[i]) < STALE_PUBLISHES)
            .flat_map(move |i| self.orbs[i][..self.lens[i]].iter())
    }

    #[cfg(test)]
    pub fn live(&self) -> impl Iterator<Item = &Orb> + '_ {
        self.live_slots(u8::MAX)
    }

    /// Hitboxes and grabs, fighter hurtboxes, launch marks, then article hurtboxes.
    pub fn draw_slots_in_order(&self, slots: u8) -> impl Iterator<Item = &Orb> + '_ {
        let of = move |kind: fn(u32) -> bool| self.live_slots(slots).filter(move |o| kind(o.kind()));
        of(|k| k != KIND_HURT && k != KIND_MARK && k != KIND_ARTICLE_HURT).chain(of(|k| k == KIND_HURT)).chain(of(|k| k == KIND_MARK)).chain(of(|k| k == KIND_ARTICLE_HURT))
    }

    #[cfg(test)]
    pub fn in_draw_order(&self) -> impl Iterator<Item = &Orb> + '_ {
        self.draw_slots_in_order(u8::MAX)
    }
}

/// One sphere as an orb on its bone. A hurtbox's radius is its part size at the bone's world scale, as the game's hurt
/// test uses it (rc.15 target-hurtbox-size, Eden target-hurtbox-d6393cb0-yoga-4: the lock-on Sandbag posed at 0.6 touches
/// at 3.45 with size 5, Mario at 1.14 with size 1.2): the scale is the distance `place` puts a unit offset from the centre.
/// Without a valid sample the hurtbox is left out that frame, never drawn at the unscaled size.
fn place_one(s: &Sphere, place: &mut impl FnMut(&Sphere) -> Option<[f32; 3]>) -> Option<Orb> {
    let centre = place(s)?;
    let mut orb = orb_of(s, centre);
    if matches!(s.key >> 24, KIND_HURT | KIND_ARTICLE_HURT) {
        let unit = Sphere { pos: [s.pos[0] + 1.0, s.pos[1], s.pos[2]], ..*s };
        let e = place(&unit)?;
        let scale = ((e[0] - centre[0]).powi(2) + (e[1] - centre[1]).powi(2) + (e[2] - centre[2]).powi(2)).sqrt();
        orb.radius *= (scale.is_finite() && scale > 0.0).then_some(scale)?;
    }
    Some(orb)
}

/// rc.15 target-hurtbox-display: each live practice target's (BTT target, Eggs-ercise egg) break volume, the picture disc
/// (`eggs::RADIUS`) that decides the break, as a hurtbox orb in the fighters' plane. Break animations and the cursor are not
/// targets. How many were written.
pub fn target_hurt_orbs(looks: &[Option<([f32; 2], super::panes::EggLook)>], out: &mut [Orb]) -> usize {
    let color = super::panes::linear(tint_colour(super::hitbox::HURTBOX_TINT, ORB_ALPHA));
    let live = looks.iter().flatten().filter(|(_, look)| *look == super::panes::EGG_WHOLE);
    let mut n = 0;
    for (slot, (k, ([x, y], _))) in out.iter_mut().zip(live.enumerate()) {
        let centre = [*x, *y, 0.0];
        let key = super::hitbox::sphere_key(KIND_HURT, k, 0);
        (*slot, n) = (Orb { centre, tail: centre, radius: super::eggs::RADIUS, color, key }, n + 1);
    }
    n
}

/// How many of the `n` target discs a layout tick draws: none while Hurtboxes is off, and none on a tick that keeps the
/// last frame's panes (the board was busy), whose caps would otherwise overwrite a fighter's retained ones.
pub fn targets_drawn(board_held: bool, hurtboxes: bool, n: usize) -> usize {
    if board_held && hurtboxes { n } else { 0 }
}

/// `set`'s spheres as orbs on their bones (`place`) into `out`, then, while *Launch angles* is On (`lr`: the owner's facing), each
/// hitbox's (and windbox's) marks (`legend::marks`) at its slot's angle (`angle(slot)`, `None` when the slot has no data). How many were written.
pub fn place_orbs(set: &HandleSet, mut place: impl FnMut(&Sphere) -> Option<[f32; 3]>, lr: Option<f32>, mut angle: impl FnMut(i32) -> Option<i32>, out: &mut [Orb]) -> usize {
    let mut n = 0;
    for (slot, orb) in out.iter_mut().zip(set.spheres().filter_map(|s| place_one(s, &mut place))) {
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

/// Which article holds which slot: (battle-object id, accessor), the stamp it came in with (the oldest goes first) and whether it
/// attacks. A plain value: the runtime's is the handoff's, which its command hooks change too (`Handoff::change_articles`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Members {
    pub ids: [(u32, usize); ARTICLES],
    pub ages: [u64; ARTICLES],
    pub attacking: [bool; ARTICLES],
    pub touched: [u64; ARTICLES],
}

impl Members {
    /// Track an object and its priority at `stamp`: its slot and, when it is new there, what the slot held. Hurt-only discovery
    /// takes free slots; attacks evict passive objects before the oldest attack.
    pub fn track(&mut self, id: u32, key: usize, attacking: bool, stamp: u64) -> Option<(usize, Option<(u32, usize)>)> {
        let i = (0..ARTICLES).min_by_key(|&i| (self.ids[i] != (id, key), self.ids[i].0 != 0, self.attacking[i], self.ages[i])).unwrap_or(0);
        if !attacking && self.ids[i] != (id, key) && self.ids[i].0 != 0 { return None; }
        let replaced = (self.ids[i] != (id, key)).then(|| std::mem::replace(&mut self.ids[i], (id, key)));
        if replaced.is_some() {
            (self.ages[i], self.attacking[i]) = (stamp, false);
            self.touched[i] = 0;
        }
        self.attacking[i] |= attacking;
        Some((i, replaced))
    }

    /// A command hook's track at `stamp`: the object attacks, and its slot remembers the command.
    pub fn command(&mut self, id: u32, key: usize, stamp: u64) -> Option<usize> {
        let (i, _) = self.track(id, key, true, stamp)?;
        self.touched[i] = stamp;
        Some(i)
    }
}

/// The articles whose `ATTACK` or `CATCH` ran (rc.13): (battle-object id, accessor), the accessor the board key of its
/// orbs, and its spheres (a `CATCH`'s wait there with their status, as a fighter's do: Olimar's Pikmin); the oldest is
/// replaced.
#[derive(Default)]
pub struct Articles {
    members: Members,
    clock: u64,
    sets: [HandleSet; ARTICLES],
    evicted: bool,
    /// The runtime's: the slots are the handoff's. `seen` is the version `members` was read at and `flags` its attacking flags
    /// then; what the pass set since goes out with the next change.
    link: Option<&'static Handoff>,
    seen: u64,
    applied: u64,
    flags: [bool; ARTICLES],
}

impl Articles {
    pub const fn new() -> Articles {
        Articles {
            members: Members { ids: [(0, 0); ARTICLES], ages: [0; ARTICLES], attacking: [false; ARTICLES], touched: [0; ARTICLES] },
            clock: 0,
            sets: [HandleSet::new(); ARTICLES],
            evicted: false,
            link: None,
            seen: 0,
            applied: 0,
            flags: [false; ARTICLES],
        }
    }

    /// From now on the slots are `link`'s.
    pub fn link(&mut self, link: &'static Handoff) {
        self.link = Some(link);
    }

    /// Change the slots: its own, or the link's. Linked, the change carries the attacking flags the pass set (assigned when nothing
    /// changed the slots since they were read, else only raised), and what the command hooks changed is adopted: a slot with a new
    /// article forgets its set, and the board is to be cleared of what left. Without a buffer (which `handoff::BUFFERS` rules out)
    /// the change stays on its own view.
    fn change<R>(&mut self, mut op: impl FnMut(&mut Members, u64) -> R) -> R {
        let (view, seen, flags) = (self.members, self.seen, self.flags);
        let changed = self.link.and_then(|link| link.change_articles(|members, version| {
            for (i, &flag) in flags.iter().enumerate() {
                if view.attacking[i] != flag && (members.ids[i], members.ages[i]) == (view.ids[i], view.ages[i]) {
                    members.attacking[i] = view.attacking[i] || (version != seen + 1 && members.attacking[i]);
                }
            }
            op(members, version)
        }));
        let Some((version, members, result)) = changed else {
            self.clock += 1;
            return op(&mut self.members, self.clock);
        };
        for i in 0..ARTICLES {
            if (members.ids[i], members.ages[i]) != (view.ids[i], view.ages[i]) {
                self.evicted |= view.ids[i].1 != 0;
                if members.ids[i].0 != 0 {
                    self.sets[i].forget();
                }
            }
        }
        (self.members, self.seen, self.flags) = (members, version, members.attacking);
        result
    }

    /// Track an object and its priority. Hurt-only discovery takes free slots; attacks evict passive objects before the oldest attack.
    pub fn track(&mut self, id: u32, key: usize, board: Option<&mut Board>, attacking: bool) -> Option<(&mut HandleSet, &mut bool)> {
        let (i, replaced) = self.change(|members, stamp| members.track(id, key, attacking, stamp))?;
        if let Some(old) = replaced {
            if let Some(board) = board {
                board.clear(old.1);
            } else {
                self.evicted |= old.1 != 0;
            }
            self.sets[i].forget();
        }
        Some((&mut self.sets[i], &mut self.members.attacking[i]))
    }

    pub fn tracked(&self) -> [(u32, usize); ARTICLES] {
        self.members.ids
    }

    pub fn forget(&mut self, id: u32, board: &mut Board) {
        let applied = self.applied;
        let (keys, retained) = self.change(|members, _| {
            let mut keys = [0; ARTICLES];
            let mut retained = [false; ARTICLES];
            for (i, key) in keys.iter_mut().enumerate() {
                if members.ids[i].0 == id {
                    *key = members.ids[i].1;
                    if members.touched[i] > applied {
                        (members.ages[i], members.attacking[i]) = (members.ages[i].max(applied + 1), true);
                        retained[i] = true;
                    } else {
                        members.ids[i] = (0, 0);
                    }
                }
            }
            (keys, retained)
        });
        for (i, key) in keys.into_iter().enumerate() {
            board.clear(key);
            if retained[i] { self.sets[i].forget(); }
        }
    }

    pub fn forget_all(&mut self, board: Option<&mut Board>) {
        self.change(|members, _| members.ids = [(0, 0); ARTICLES]);
        self.evicted = true;
        if let Some(board) = board {
            self.clear_evicted(board);
        }
    }

    pub fn clear_evicted(&mut self, board: &mut Board) {
        if std::mem::take(&mut self.evicted) {
            board.clear_evicted(&self.members.ids);
        }
    }

    /// Linked: send the attacking flags the pass set and adopt what the command hooks changed.
    pub fn settle(&mut self) {
        if self.link.is_some() {
            self.change(|_, _| ());
        }
    }

    /// Phase 8, linked: adopt what the command hooks changed, then queue each slot's waiting grab commands in its set, oldest
    /// first: how many.
    pub fn apply_grabs(&mut self, each: &mut dyn FnMut(&Record, Applied)) -> usize {
        let Some(link) = self.link else { return 0 };
        self.settle();
        self.applied = self.seen;
        let mut count = 0;
        for i in 0..ARTICLES {
            while let Some(record) = (self.members.ids[i].0 != 0).then(|| link.next_waiting(FIGHTER_SETS + i, self.members.ages[i], self.seen)).flatten() {
                if let Record::Grab { shape, status, .. } = record {
                    each(&record, Applied::Grab(Some(push_grabs(&mut self.sets[i], &shape, status))));
                    count += 1;
                }
            }
        }
        count
    }
}

/// What an article's pass draws besides its spheres: the launch-angle marks while *Launch angles* is On (`lr`: the article's
/// facing) and its wind areas while *Windboxes* is ticked.
#[derive(Clone, Copy, Default)]
pub struct Extras<'a> {
    pub lr: Option<f32>,
    pub areas: &'a [legend::WindArea],
    pub hurt: &'a [Sphere],
    pub owner: Option<u32>,
}

impl Extras<'_> {
    pub const NONE: Extras<'static> = Extras { lr: None, areas: &[], hurt: &[], owner: None };
}

/// One article's pass in `status`: its live attack slots' spheres and its grab spheres (held as a fighter's), placed on its
/// bones by `place`, become its orbs on `board`, followed by its wind `areas` (rc.14 wind-articles: read from its own
/// AreaModule while *Windboxes* is ticked). False when it holds none and has no area: its attack, grab and wind ended, its orbs
/// went, and it is forgotten (`legend::keeps_article`).
pub fn article_pass(board: &mut Board, set: &mut HandleSet, (id, key): (u32, usize), status: i32, attacks: &mut impl AttackReader, place: impl FnMut(&Sphere) -> Option<[f32; 3]>, extras: Extras<'_>) -> bool {
    let Extras { lr, areas, hurt, owner } = extras;
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
    board.publish_article(key, id, owner, orbs[..n].iter().copied());
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

/// Five dots pointing toward an off-canvas start, kept clear of the canvas edges.
pub fn edge_pointer(point: [f32; 2]) -> Option<[[f32; 2]; 5]> {
    if !point.into_iter().all(f32::is_finite)
        || (point[0].abs() <= CANVAS_HALF[0] && point[1].abs() <= CANVAS_HALF[1])
    {
        return None;
    }
    let bounds = [CANVAS_HALF[0] - 40.0, CANVAS_HALF[1] - 90.0];
    let tip = [point[0].clamp(-bounds[0], bounds[0]), point[1].clamp(-bounds[1], bounds[1])];
    let delta = [point[0] - tip[0], point[1] - tip[1]];
    let scale = delta[0].abs().max(delta[1].abs());
    let unit = [delta[0] / scale, delta[1] / scale];
    let length = unit[0].hypot(unit[1]);
    let dir = [unit[0] / length, unit[1] / length];
    let perp = [-dir[1], dir[0]];
    let arm = |along: f32, across: f32| [
        tip[0] - along * dir[0] + across * perp[0],
        tip[1] - along * dir[1] + across * perp[1],
    ];
    Some([tip, arm(14.0, 10.0), arm(14.0, -10.0), arm(28.0, 20.0), arm(28.0, -20.0)])
}

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

/// The canvas bounds of a wind area's rectangle. Its endpoints are the midpoints of the vertical edges.
pub fn plan_wind_box(
    orb: &Orb,
    half: [f32; 2],
    project: impl Fn([f32; 3]) -> Option<[f32; 2]>,
) -> Option<([f32; 2], [f32; 2])> {
    if half[0] <= 0.0 || half[1] <= 0.0 {
        return None;
    }
    let mut min = [f32::INFINITY; 2];
    let mut max = [f32::NEG_INFINITY; 2];
    for [x, y, z] in [orb.centre, orb.tail] {
        for y in [y - orb.radius, y + orb.radius] {
            let point = to_canvas(project([x, y, z])?, half);
            if !point.iter().all(|value| value.is_finite()) {
                return None;
            }
            for axis in 0..2 {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }
    }
    let (width, height) = (max[0] - min[0], max[1] - min[1]);
    if !width.is_finite() || !height.is_finite() || height < 1.0 {
        return None;
    }
    if (0..2).any(|axis| min[axis] > CANVAS_HALF[axis] || max[axis] < -CANVAS_HALF[axis]) {
        return None;
    }
    let centre = [min[0] + width / 2.0, min[1] + height / 2.0];
    Some((centre, [width, height.min(MAX_PANE)]))
}

/// The two world points one radius from `centre` that `plan_pill` takes an end's projected radius from.
pub fn edge_points(centre: [f32; 3], radius: f32) -> [[f32; 3]; 2] {
    let [x, y, z] = centre;
    [[x + radius, y, z], [x, y + radius, z]]
}
