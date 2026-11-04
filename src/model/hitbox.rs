//! The hitbox visualizer's rules (SPEC-core-06), pure and host-tested: the colour table, the sphere placement of an extended
//! (capsule) hitbox, what the animation commands' Lua stack means, the per-frame step order with its status exemptions, the
//! rebound rule and the bookkeeping that restores a fighter's effects once when the setting goes Off. The runtime half
//! (`runtime::features::hitbox`) reads the game and calls these.
//!
//! ## Layouts the plugin reads (13.0.5 `main`, derived from the owned ELF; `docs/test-provenance/core-06.md`)
//!
//! `AttackData` (one attack slot of `AttackModule`, returned by `AttackModule::attack_data(accessor, slot, false)`):
//! slots are 0x130 bytes apart from the pointer at module `+0x68` (`attack_data`, vtable slot 0x1e8: `madd x0, slot, 0x130,
//! [module+0x68]`). Fields: the offset vector at `+0x00` (x, y, z, 4 bytes each), the second offset vector at `+0x10`
//! (`set_offset2` / `get_offset2`, vtable slots 0x138 / 0x140; `set_attack` copies the vector at `+0x10` of its argument to
//! `+0x10`), the size at `+0x24` (`set_size`, vtable slot 0x118: `str s0, [slot, #0x24]`), the node (bone hash) at `+0x48`
//! (`set_node`, slot 0x148: `str x2, [slot, #0x48]`), and the active word at `+0x108` (`is_attack`, slot 0xc8). The ATTACK
//! animation command builds the same structure on its stack before handing it to `set_attack` (vtable slot 0x68): the
//! vectors at `+0x00` / `+0x10`, the size at `+0x24`, the node at `+0x48`.
//!
//! The Lua stack (`lua_State`): the top pointer at `+0x10`, a pointer at `+0x20` whose first word is the frame base; argument
//! `i` (1-based) is the 16-byte value at `base + 0x10 * i`, the nil value when that is not below the top. A value is 8 bytes
//! of payload and a tag word at `+8`: tag 3 is a float (4 bytes), tag 0x13 an integer (8 bytes); the command code reads a
//! number from either (an integer converts with `scvtf`), a hash as the integer, and treats any other tag as 0. A nil
//! argument has tag 0.

/// Eight attack slots per fighter (`AttackModule`).
pub const SLOTS: i32 = 8;
/// The game effect the spheres are made of (a translucent sphere; also the shield bubble).
pub const EFFECT: &str = "sys_shield";
/// The sphere's scale per unit of hitbox size (`size x 19/200`).
pub const SCALE: f32 = 0.095;
/// The effect's playing rate, so the sphere does not pulse.
pub const RATE: f32 = 8.0;
/// Spheres per extended hitbox: spacing is `size x 1.75`, clamped to 2..=16 spheres.
pub const SPACING: f32 = 1.75;
pub const SPHERE_MIN: usize = 2;
pub const SPHERE_MAX: usize = 16;

/// `AttackData` field offsets (see the module notes).
pub mod layout {
    /// The offset vector (x, y, z).
    pub const OFFSET: usize = 0x00;
    /// The second offset vector of an extended hitbox (zero otherwise).
    pub const OFFSET2: usize = 0x10;
    pub const SIZE: usize = 0x24;
    /// The node's `Hash40`, the bone the hitbox follows.
    pub const NODE: usize = 0x48;

    /// Historical pure HB1 facts, not read by the RC runtime: `HitModule`'s module pointer in the accessor, its group table (0x3c bytes per group: the first part's
    /// index at `+4`, the part count at `+6`, both `i16`) and its part table (0x50 bytes per part).
    pub const HIT_MODULE: usize = 0xb0;
    pub const HIT_GROUPS: usize = 0x60;
    pub const HIT_GROUP_STRIDE: usize = 0x3c;
    pub const HIT_GROUP_FIRST: usize = 0x4;
    pub const HIT_GROUP_COUNT: usize = 0x6;
    pub const HIT_PARTS: usize = 0x70;
    /// The number of parts in the part table (`i32`).
    pub const HIT_PART_TOTAL: usize = 0x78;
    pub const HIT_PART_STRIDE: usize = 0x50;
    /// One hurtbox part: its `HitStatus` word, the two offset vectors of its capsule, its size and its node (bone hash).
    pub const PART_STATUS: usize = 0x00;
    pub const PART_OFFSET: usize = 0x08;
    pub const PART_OFFSET2: usize = 0x14;
    pub const PART_SIZE: usize = 0x20;
    pub const PART_NODE: usize = 0x28;

    /// `lua_State` fields.
    pub const LUA_TOP: usize = 0x10;
    pub const LUA_FRAME: usize = 0x20;
    /// One stack value: payload at +0, tag at +8.
    pub const SLOT_BYTES: usize = 0x10;
    pub const TAG_FIELD: usize = 0x8;
}

/// The eight tints (RGB multipliers; at least one component is 1.0), by `id mod 8`.
pub const TINTS: [[f32; 3]; 8] = [
    [1.0, 0.0, 0.0],
    [1.0, 0.4, 0.0],
    [0.8, 1.0, 0.0],
    [0.2, 1.0, 0.2],
    [0.0, 0.8, 1.0],
    [0.4, 0.4, 1.0],
    [0.8, 0.0, 1.0],
    [1.0, 0.2, 0.8],
];

/// The tint of colour index `index` (taken modulo 8, negatives wrap).
pub fn tint(index: i32) -> [f32; 3] {
    TINTS[index.rem_euclid(8) as usize]
}

/// A hitbox of id `id`.
pub fn hitbox_tint(id: i32) -> [f32; 3] {
    tint(id)
}

/// A grab box of id `id`: the same table shifted by three, so a grab box next to a hitbox of the same id reads differently.
pub fn grab_tint(id: i32) -> [f32; 3] {
    tint(id.wrapping_add(3))
}

/// The sphere scale for a hitbox size.
pub fn sphere_scale(size: f32) -> f32 {
    size * SCALE
}

/// How many spheres an extended hitbox of `distance` between its centres and `size` is drawn with:
/// `ceil(distance / (size x 1.75)) + 1`, clamped to 2..=16 (a non-positive size or a non-finite distance draws the minimum).
pub fn sphere_count(distance: f32, size: f32) -> usize {
    let step = size * SPACING;
    if step.is_nan() || step <= 0.0 || !distance.is_finite() || distance < 0.0 {
        return SPHERE_MIN;
    }
    let n = (distance / step).ceil() + 1.0;
    if n >= SPHERE_MAX as f32 {
        SPHERE_MAX
    } else {
        (n as usize).clamp(SPHERE_MIN, SPHERE_MAX)
    }
}

/// The centres to draw spheres at (no allocation: an iterator over at most 16 points).
#[derive(Clone, Copy, Debug)]
pub struct Spheres {
    a: [f32; 3],
    b: [f32; 3],
    n: usize,
    i: usize,
}

impl Iterator for Spheres {
    type Item = [f32; 3];

    fn next(&mut self) -> Option<[f32; 3]> {
        if self.i >= self.n {
            return None;
        }
        let at = if self.n == 1 {
            self.a
        } else {
            let t = self.i as f32 / (self.n - 1) as f32;
            [self.a[0] + (self.b[0] - self.a[0]) * t, self.a[1] + (self.b[1] - self.a[1]) * t, self.a[2] + (self.b[2] - self.a[2]) * t]
        };
        self.i += 1;
        Some(at)
    }
}

/// A point hitbox is one sphere at `a`; one with a second centre `b` is `sphere_count` spheres evenly spaced from `a` to `b`
/// (`t = i / (n - 1)`).
pub fn spheres(a: [f32; 3], b: Option<[f32; 3]>, size: f32) -> Spheres {
    match b {
        None => Spheres { a, b: a, n: 1, i: 0 },
        Some(b) => {
            let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            Spheres { a, b, n: sphere_count(distance, size), i: 0 }
        }
    }
}

/// The second centre of a hitbox: the second offset vector, or `None` when it is all zero (the game leaves it zero for a point
/// hitbox).
pub fn second_centre(b: [f32; 3]) -> Option<[f32; 3]> {
    (b != [0.0; 3]).then_some(b)
}

// ---- hurtboxes (decision HB1 (a), 2026-11-02; route B, operator decision 2026-11-04 ~06:5xZ) ------------------------
//
// The rules below are renderer-agnostic and host-tested: what a part's shape is, which parts a pass reads, and the spheres
// they become. The draw route is route B, the fork's shared-pool `sys_shield` spheres (operator decision, `docs/DECISIONS.md`
// HB1-route, `review/menu-proposal` commit `5ca5e3b`; it reverses the 2026-11-03 option A default, which
// `OPTION-A-DESIGN.md` could not route safely): `reconcile_frame` plans the enabled parts' spheres into the same wants and
// requests them through the same set and `Effects` interface as the hitboxes, and the runtime passes `Some(GameHurt { .. })`
// while the setting is On (`docs/HB1-RENDER-ROUTES.md`).

/// Hurtboxes are drawn yellow.
pub const HURTBOX_TINT: [f32; 3] = [1.0, 1.0, 0.0];
/// The `HitStatus` of a part that is switched off (the game's part update skips its shape for it).
pub const HIT_STATUS_OFF: u32 = 3;
/// The hurtbox group the pass draws: group 0, the one the fighters' scripts address (`set_status_all(status, 0)`).
pub const HURTBOX_GROUP: usize = 0;
/// No fighter has more parts in a group than this; a larger count is not a hurtbox table and nothing is drawn.
pub const MAX_HURTBOX_PARTS: i16 = 64;

/// The part indices of a group (`first`, `count` as the game stores them; `total` is the module's part count at `+0x78`), or `None`
/// for a table that does not look like one.
pub fn group_parts(first: i16, count: i16, total: i32) -> Option<std::ops::Range<usize>> {
    if first < 0 || !(0..=MAX_HURTBOX_PARTS).contains(&count) || i32::from(first) + i32::from(count) > total {
        return None;
    }
    Some(first as usize..first as usize + count as usize)
}

/// A hurtbox part as a shape to draw: `None` while the part is switched off. A capsule's two centres are both given; when they
/// are the same point the part is one sphere.
pub fn hurtbox_shape(record: &[u8; layout::HIT_PART_STRIDE]) -> Option<Shape> {
    let u32_at = |at: usize| u32::from_le_bytes([record[at], record[at + 1], record[at + 2], record[at + 3]]);
    let f32_at = |at: usize| f32::from_bits(u32_at(at));
    let vec3 = |at: usize| [f32_at(at), f32_at(at + 4), f32_at(at + 8)];
    if u32_at(layout::PART_STATUS) == HIT_STATUS_OFF {
        return None;
    }
    let mut node = [0u8; 8];
    node.copy_from_slice(&record[layout::PART_NODE..layout::PART_NODE + 8]);
    let a = vec3(layout::PART_OFFSET);
    let b = vec3(layout::PART_OFFSET2);
    Some(Shape { id: 0, joint: u64::from_le_bytes(node), size: f32_at(layout::PART_SIZE), a, b: (b != a).then_some(b) })
}

// ---- the animation commands' Lua stack -----------------------------------------------------------------------------

pub const TAG_FLOAT: u32 = 3;
pub const TAG_INT: u32 = 0x13;

/// One value of the Lua stack: payload and tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub raw: u64,
    pub tag: u32,
}

impl Cell {
    /// The value past the top of the stack.
    pub const NIL: Cell = Cell { raw: 0, tag: 0 };

    /// The value as a number, as the command code reads it: a float, an integer converted to float, anything else 0.
    pub fn float(self) -> f32 {
        match self.tag {
            TAG_FLOAT => f32::from_bits(self.raw as u32),
            TAG_INT => (self.raw as i64) as f32,
            _ => 0.0,
        }
    }

    /// The value as an integer (a hash is read this way): an integer, a float floored, anything else 0.
    pub fn int(self) -> i64 {
        match self.tag {
            TAG_INT => self.raw as i64,
            TAG_FLOAT => f32::from_bits(self.raw as u32).floor() as i64,
            _ => 0,
        }
    }
}

/// Where a command's parameters sit on the stack (1-based).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Indices {
    pub id: usize,
    pub joint: usize,
    pub size: usize,
    /// The first of the three centre components (x, y, z); the second centre follows at `second`.
    pub centre: usize,
    pub second: usize,
}

/// `ATTACK` (36 parameters; 4..8 are damage, angle, knockback growth, fixed and base knockback, unused here).
pub const ATTACK: Indices = Indices { id: 1, joint: 3, size: 9, centre: 10, second: 13 };
/// `CATCH`.
pub const CATCH: Indices = Indices { id: 1, joint: 2, size: 3, centre: 4, second: 7 };

/// A hitbox or grab box as its command gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub id: i32,
    /// The bone's `Hash40`.
    pub joint: u64,
    pub size: f32,
    pub a: [f32; 3],
    pub b: Option<[f32; 3]>,
}

/// Read a command's shape from the stack values `cell(i)` (1-based). `None` when the joint is not an integer (a command the
/// plugin cannot place on a bone is not drawn).
pub fn parse(ix: Indices, cell: impl Fn(usize) -> Cell) -> Option<Shape> {
    let joint = cell(ix.joint);
    if joint.tag != TAG_INT {
        return None;
    }
    let vec3 = |first: usize| [cell(first).float(), cell(first + 1).float(), cell(first + 2).float()];
    Some(Shape {
        id: cell(ix.id).int() as i32,
        joint: joint.raw,
        size: cell(ix.size).float(),
        a: vec3(ix.centre),
        b: second_centre(vec3(ix.second)),
    })
}

/// The `Cell` at argument `index` of a stack whose frame base is `base`, top `top` and whose value `i` is read by `load`:
/// the nil value when the slot is not below the top. `base`/`top` are addresses, `load(address)` reads one value.
pub fn stack_cell(base: usize, top: usize, index: usize, load: impl Fn(usize) -> Cell) -> Cell {
    let slot = base.wrapping_add(layout::SLOT_BYTES.wrapping_mul(index));
    if slot < top {
        load(slot)
    } else {
        Cell::NIL
    }
}

// ---- the per-frame pass --------------------------------------------------------------------------------------------

/// The game's status ids the rules compare against (resolved once through the constant table; `UNRESOLVED` until then).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusIds {
    pub catch: i32,
    pub catch_turn: i32,
    pub tread_fall: i32,
    pub down: i32,
    pub guard_on: i32,
    pub guard_damage: i32,
}

/// The identifiers of `StatusIds`, in field order.
pub const STATUS_NAMES: [&str; 6] = [
    "FIGHTER_STATUS_KIND_CATCH",
    "FIGHTER_STATUS_KIND_CATCH_TURN",
    "FIGHTER_STATUS_KIND_TREAD_FALL",
    "FIGHTER_STATUS_KIND_DOWN",
    "FIGHTER_STATUS_KIND_GUARD_ON",
    "FIGHTER_STATUS_KIND_GUARD_DAMAGE",
];

/// The effect attributes OR-ed for a sphere: no joint scale, follow, conclude with the status.
pub const ATTRIBUTE_NAMES: [&str; 3] = ["EFFECT_SUB_ATTRIBUTE_NO_JOINT_SCALE", "EFFECT_SUB_ATTRIBUTE_FOLLOW", "EFFECT_SUB_ATTRIBUTE_CONCLUDE_STATUS"];

impl StatusIds {
    pub fn from_values(v: [i32; 6]) -> StatusIds {
        StatusIds { catch: v[0], catch_turn: v[1], tread_fall: v[2], down: v[3], guard_on: v[4], guard_damage: v[5] }
    }

    /// In a grab status range (`CATCH..=CATCH_TURN`): no drawing and no sleeping, so the grab animation's effects show.
    pub fn in_grab(&self, status: i32) -> bool {
        (self.catch..=self.catch_turn).contains(&status)
    }

    /// Shielding (`GUARD_ON..=GUARD_DAMAGE`): the shield bubble is itself a `sys_shield` effect, so nothing is killed.
    pub fn shielding(&self, status: i32) -> bool {
        (self.guard_on..=self.guard_damage).contains(&status)
    }

    /// Statuses whose effects stay awake: grab, capture and throw, the tread (footstool) statuses (`CATCH..=TREAD_FALL`) and the
    /// knockdown (`DOWN`), so missed-tech and footstool cues remain.
    pub fn effects_stay_awake(&self, status: i32) -> bool {
        (self.catch..=self.tread_fall).contains(&status) || status == self.down
    }
}

/// What the pass knows about one fighter this frame.
#[derive(Clone, Copy, Debug)]
pub struct FrameFacts {
    /// Training or lobby practice, and this fighter is one the practice visualizes (`false` in every other scene, online too).
    pub visualizes: bool,
    pub enabled: bool,
    /// The plugin put this fighter's effects to sleep on an earlier frame and has not restored them.
    pub slept: bool,
    pub status: i32,
    pub ids: StatusIds,
}

/// The pass's native calls for one fighter, in order: restore, put to sleep, hide and kill the old spheres then draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramePlan {
    /// `set_sleep_effect(false)`.
    pub restore: bool,
    /// `set_sleep_effect(true)` (after the restore).
    pub sleep: bool,
    /// `set_visible_kind(sys_shield, false)`, `kill_kind(sys_shield)`, then one sphere set per active slot.
    pub redraw: bool,
}

impl FramePlan {
    pub const NONE: FramePlan = FramePlan { restore: false, sleep: false, redraw: false };

    pub const fn calls(self) -> bool {
        self.restore || self.sleep || self.redraw
    }
}

/// SPEC-core-06 section 5, in order: (0) not practice: nothing, so another scene's effect flag is never touched; (1) restore;
/// (2) Off: stop; (3) grab range: stop; (4) shielding: stop; (5) sleep unless the status keeps effects awake; (6) kill the old
/// spheres; (7) draw. With the setting Off the restore is made once, when the plugin had put the fighter to sleep (Q4's
/// recommendation: no game call per frame while Off); while On it is made every frame, as the spec orders. The spec's step (5)
/// `set_sleep_effect(!awake)` is the single `sleep` call after the restore: when the fighter stays awake the restore already set the
/// flag to `false`, so a second `false` call is not made.
pub fn plan(f: FrameFacts) -> FramePlan {
    if !f.visualizes {
        return FramePlan::NONE;
    }
    if !f.enabled {
        return FramePlan { restore: f.slept, sleep: false, redraw: false };
    }
    if f.ids.in_grab(f.status) || f.ids.shielding(f.status) {
        return FramePlan { restore: true, sleep: false, redraw: false };
    }
    FramePlan { restore: true, sleep: !f.ids.effects_stay_awake(f.status), redraw: true }
}

/// The fighter's `slept` flag after the pass.
pub fn slept_after(f: FrameFacts, plan: FramePlan) -> bool {
    if !f.visualizes {
        f.slept
    } else {
        plan.sleep
    }
}

/// The rebound hook (a grab ended, `rebound` is the new value): the spheres are hidden and killed when the rebound is cleared,
/// the setting is On, the object is one the practice visualizes and the fighter is not shielding.
pub fn kill_on_rebound(rebound: bool, visualizes: bool, enabled: bool, shielding: bool) -> bool {
    !rebound && visualizes && enabled && !shielding
}

/// Which fighters the practice visualizes: in native Training every one; in the waiting room's lobby practice only the practice
/// fighters, Player 1 (entry 0) and the practice CPU (package 2's `LOBBY_CPU_ENTRY`), never the opponents in the room; in any other
/// scene (online, a hot swap) none. `entry` is `None` for an object whose entry is unknown (an article in the lobby: not drawn).
pub fn visualizes_entry(scene: Option<super::scene::Scene>, entry: Option<i32>) -> bool {
    use super::scene::Scene;
    match scene {
        Some(Scene::Training) => true,
        Some(Scene::Lobby) => matches!(entry, Some(e) if e == 0 || e == super::cpu::role::LOBBY_CPU_ENTRY),
        None => false,
    }
}

/// The work id the lobby gate reads a command's object's entry with (resolved through the constant table).
pub const ENTRY_ID_NAME: &str = "FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID";

/// Whether the animation commands draw the shape they create: the setting is On and the object is one the practice visualizes.
pub fn draw_on_command(enabled: bool, visualizes: bool) -> bool {
    enabled && visualizes
}

/// The setting: `hitbox_vis` is a two-option single choice (`On`, `Off`; Off by default), On when its first option is set.
pub fn enabled(snapshot: &super::settings::Snapshot) -> bool {
    snapshot.get("hitbox_vis").is_some_and(|row| row.first().copied().unwrap_or(0) != 0)
}

/// How many fighters the sleep bookkeeping tracks (eight entries and their partners).
pub const BOOK_SLOTS: usize = 16;

/// Which fighters (by their accessor address) the plugin has put to sleep: fixed storage, no allocation.
#[derive(Clone, Copy, Debug)]
pub struct SleepBook {
    keys: [usize; BOOK_SLOTS],
    next: usize,
}

impl SleepBook {
    pub const fn new() -> SleepBook {
        SleepBook { keys: [0; BOOK_SLOTS], next: 0 }
    }

    pub fn slept(&self, key: usize) -> bool {
        key != 0 && self.keys.contains(&key)
    }

    /// Record `slept` for `key`: a sleeping fighter takes a free entry (the oldest is replaced when all are taken), a restored
    /// one frees its entry.
    pub fn set(&mut self, key: usize, slept: bool) {
        if key == 0 {
            return;
        }
        let at = self.keys.iter().position(|k| *k == key);
        match (slept, at) {
            (true, Some(_)) => {}
            (true, None) => {
                let free = self.keys.iter().position(|k| *k == 0).unwrap_or_else(|| {
                    let i = self.next;
                    self.next = (self.next + 1) % BOOK_SLOTS;
                    i
                });
                self.keys[free] = key;
            }
            (false, Some(i)) => self.keys[i] = 0,
            (false, None) => {}
        }
    }
}

impl Default for SleepBook {
    fn default() -> Self {
        SleepBook::new()
    }
}

// ---- persistent spheres (lead ruling 2026-11-02 on Q3, after Eden hb-01) ------------------------------------------
//
// Respawning every sphere every frame (kill by kind, request again) ran the game's effect pool dry once hurtboxes were drawn
// (Eden hb-01: the CPU's 55 hurtbox spheres and the grab boxes never appeared, later none at all). Each sphere is now an effect
// the plugin keeps: requested once on its bone (the effect follows the bone by itself), killed by its own handle when its shape
// changes or goes away, and requested again only when the game itself ended it (a status change concludes it). The plugin never
// kills by kind, so the shield bubble and every other `sys_shield` user are left alone.

/// Spheres one fighter can hold at once (every attack slot's and every hurtbox part's spheres together).
pub const MAX_HELD: usize = 128;
/// The want scratch of one pass: the attack slots' spheres and the hurt parts' spheres together (decision HB1), so a
/// full slot set never starves the hurt enumeration; what still does not fit is counted in `Counts::dropped`.
pub const WANT_SCRATCH: usize = SLOTS as usize * SPHERE_MAX + MAX_HURTBOX_PARTS as usize * SPHERE_MAX;
/// Grab spheres waiting for the fighter's pass (the `CATCH` command records them; the pass requests them).
pub const MAX_PENDING: usize = 24;
/// Fighters tracked at once (eight entries; a fighter that is gone is forgotten without a game call).
pub const FIGHTER_SETS: usize = 8;

pub const KIND_HIT: u32 = 1;
pub const KIND_HURT: u32 = 2;
pub const KIND_GRAB: u32 = 3;

/// The identity of one sphere: what it belongs to (`kind`), which slot, part or grab id (`index`), which sphere of its capsule.
pub const fn sphere_key(kind: u32, index: usize, sphere: usize) -> u32 {
    (kind << 24) | (((index & 0xffff) as u32) << 8) | (sphere & 0xff) as u32
}

/// One sphere as the game is asked for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    pub key: u32,
    pub joint: u64,
    pub pos: [f32; 3],
    pub scale: f32,
    pub tint: [f32; 3],
}

impl Sphere {
    pub const NONE: Sphere = Sphere { key: 0, joint: 0, pos: [0.0; 3], scale: 0.0, tint: [0.0; 3] };
}

/// Append a shape's spheres to `out[..*len]` (as many as fit). Returns how many were left out for lack of room.
pub fn push_shape(out: &mut [Sphere], len: &mut usize, kind: u32, index: usize, shape: &Shape, tint: [f32; 3]) -> u32 {
    let mut left_out = 0;
    for (i, pos) in spheres(shape.a, shape.b, shape.size).enumerate() {
        if *len >= out.len() {
            left_out += 1;
            continue;
        }
        out[*len] = Sphere { key: sphere_key(kind, index, i), joint: shape.joint, pos, scale: sphere_scale(shape.size), tint };
        *len += 1;
    }
    left_out
}

/// The retained attack-slot reader. The runtime supplies guarded calls and reads;
/// host probes run this same planning/dispatch code with address-counting spies.
pub trait AttackReader {
    fn active(&mut self, slot: i32) -> bool;
    fn data(&mut self, slot: i32) -> usize;
    fn read_u64(&mut self, at: usize) -> u64;
    fn read_f32(&mut self, at: usize) -> f32;
}

fn attack_vec3(reader: &mut impl AttackReader, at: usize) -> [f32; 3] {
    [reader.read_f32(at), reader.read_f32(at + 4), reader.read_f32(at + 8)]
}

/// Ordinary hitboxes only: no HitModule lookup or HB1 part enumeration.
pub fn want_hitboxes(reader: &mut impl AttackReader, out: &mut [Sphere], len: &mut usize) -> u32 {
    let mut left_out = 0;
    for slot in 0..SLOTS {
        if !reader.active(slot) {
            continue;
        }
        let data = reader.data(slot);
        if data == 0 {
            continue;
        }
        let shape = Shape {
            id: slot,
            joint: reader.read_u64(data + layout::NODE),
            size: reader.read_f32(data + layout::SIZE),
            a: attack_vec3(reader, data + layout::OFFSET),
            b: second_centre(attack_vec3(reader, data + layout::OFFSET2)),
        };
        left_out += push_shape(out, len, KIND_HIT, slot as usize, &shape, hitbox_tint(slot));
    }
    left_out
}

/// The retained hurtbox reader. The runtime supplies the HitModule pointer and the in-place table reads; host probes run this
/// same enumeration with address-counting spies (as `AttackReader` does for the attack slots). Every address is derived from
/// `module()`; the reader only answers reads.
pub trait HurtboxReader {
    /// The fighter's `HitModule` (accessor `+0xb0`), or 0 when the fighter has none.
    fn module(&mut self) -> usize;
    fn read_usize(&mut self, at: usize) -> usize;
    fn read_i32(&mut self, at: usize) -> i32;
    fn read_i16(&mut self, at: usize) -> i16;
    /// The 0x50-byte part record at `at` (the part table's record `at` names), read in place.
    fn read_part(&mut self, at: usize, record: &mut [u8; layout::HIT_PART_STRIDE]);
}

/// The spheres of the fighter's enabled hurtbox parts (decision HB1 (a)): `HitModule`'s group `HURTBOX_GROUP` — the one the
/// fighters' scripts address — its parts that are not switched off, each its capsule on its bone, yellow, with the same sphere
/// rule as a hitbox. The tables are read in place (layout `layout::HIT_*`); a table that does not look like one gives nothing.
/// Appends to `out[..*len]`; returns how many were left out for lack of room.
pub fn want_hurtboxes(reader: &mut (impl HurtboxReader + ?Sized), out: &mut [Sphere], len: &mut usize) -> u32 {
    let mut left_out = 0;
    hurt_parts(reader, |index, shape| left_out += push_shape(out, len, KIND_HURT, index, shape, HURTBOX_TINT) as usize);
    left_out as u32
}

/// Walks the enabled parts of group `HURTBOX_GROUP` once, in part order: `each(index, shape)` for every part that is not
/// switched off — all of them, whatever `each` keeps (the draw appends spheres, the diag census only counts). Returns the
/// enabled parts seen; a module or table pointer that is missing or does not look like one is 0 parts.
pub fn hurt_parts(reader: &mut (impl HurtboxReader + ?Sized), mut each: impl FnMut(usize, &Shape)) -> usize {
    let module = reader.module();
    if module == 0 {
        return 0;
    }
    let groups = reader.read_usize(module + layout::HIT_GROUPS);
    let parts = reader.read_usize(module + layout::HIT_PARTS);
    if groups == 0 || parts == 0 {
        return 0;
    }
    let total = reader.read_i32(module + layout::HIT_PART_TOTAL);
    let group = groups + HURTBOX_GROUP * layout::HIT_GROUP_STRIDE;
    let first = reader.read_i16(group + layout::HIT_GROUP_FIRST);
    let count = reader.read_i16(group + layout::HIT_GROUP_COUNT);
    let Some(range) = group_parts(first, count, total) else { return 0 };
    let mut seen = 0;
    for index in range {
        let mut record = [0u8; layout::HIT_PART_STRIDE];
        reader.read_part(parts + index * layout::HIT_PART_STRIDE, &mut record);
        if let Some(shape) = hurtbox_shape(&record) {
            seen += 1;
            each(index, &shape);
        }
    }
    seen
}

/// The live `HitModule` of one fighter as a `HurtboxReader` (decision HB1), its reads bound to `load`: the runtime binds one
/// raw game-memory read, a host probe binds a fake layout (exact addresses). Every pointer read is type-checked first — a
/// null or misaligned address or pointer is refused with 0 — because Eden answers 0 for an unmapped read, so a missing
/// check would not show there (playbook EDEN, "Read the results").
pub struct GameHurt<F> {
    /// The fighter object's accessor address (the module pointer is read at `+layout::HIT_MODULE`).
    pub accessor: usize,
    /// `load(at, out)`: the `out.len()` bytes at `at`.
    pub load: F,
}

impl<F: FnMut(usize, &mut [u8])> GameHurt<F> {
    /// The pointer-sized value at `at` as a pointer: a null or misaligned address or pointer is refused (0).
    fn pointer(&mut self, at: usize) -> usize {
        let mut bytes = [0u8; 8];
        if at == 0 || !at.is_multiple_of(8) {
            return 0;
        }
        (self.load)(at, &mut bytes);
        let pointer = usize::from_le_bytes(bytes);
        if pointer == 0 || !pointer.is_multiple_of(8) {
            0
        } else {
            pointer
        }
    }

    /// The `width`-byte little-endian scalar at `at`: a null or misaligned address is refused (0).
    fn scalar(&mut self, at: usize, width: usize) -> u64 {
        let mut bytes = [0u8; 8];
        if at == 0 || !at.is_multiple_of(width) {
            return 0;
        }
        (self.load)(at, &mut bytes[..width]);
        u64::from_le_bytes(bytes)
    }
}

impl<F: FnMut(usize, &mut [u8])> HurtboxReader for GameHurt<F> {
    fn module(&mut self) -> usize {
        if self.accessor == 0 {
            return 0;
        }
        self.pointer(self.accessor.wrapping_add(layout::HIT_MODULE))
    }

    fn read_usize(&mut self, at: usize) -> usize {
        self.pointer(at)
    }

    fn read_i32(&mut self, at: usize) -> i32 {
        self.scalar(at, 4) as i32
    }

    fn read_i16(&mut self, at: usize) -> i16 {
        self.scalar(at, 2) as i16
    }

    fn read_part(&mut self, at: usize, record: &mut [u8; layout::HIT_PART_STRIDE]) {
        // The part table's stride is 0x50, so an 8-aligned table keeps every record 8-aligned.
        if at == 0 || !at.is_multiple_of(8) {
            *record = [0; layout::HIT_PART_STRIDE];
            return;
        }
        (self.load)(at, record);
    }
}

/// One fighter kind's hurt census (the diag build only; the shipped build reads no HitModule): the enabled parts and the
/// spheres they want of every observed pass, and the read's cost in ticks. The state of one window; `window` closes a full
/// one into the `[Hitbox] hurt census` line's fields and opens the next (the first pass's total is kept, never reset).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HurtCensus {
    /// Frames observed in the open window.
    pub frames: u32,
    /// The enabled parts of the last observed pass.
    pub parts: u32,
    /// The most enabled parts seen (kept across windows).
    pub parts_max: u32,
    /// The spheres the kind's first observed pass wanted.
    pub first: Option<u32>,
    /// The read costs of the open window, summed (the average divides by the frames).
    pub ticks: u64,
    pub ticks_min: u64,
    pub ticks_max: u64,
}

impl HurtCensus {
    pub const EMPTY: HurtCensus = HurtCensus { frames: 0, parts: 0, parts_max: 0, first: None, ticks: 0, ticks_min: 0, ticks_max: 0 };

    /// Observe one pass: `parts` enabled parts, `spheres` the pass wanted, `ticks` the read cost.
    pub fn observe(&mut self, parts: usize, spheres: usize, ticks: u64) {
        self.frames = self.frames.saturating_add(1);
        self.parts = parts as u32;
        self.parts_max = self.parts_max.max(self.parts);
        self.first = Some(self.first.unwrap_or(spheres as u32));
        self.ticks = self.ticks.saturating_add(ticks);
        self.ticks_min = if self.ticks_min == 0 { ticks } else { self.ticks_min.min(ticks) };
        self.ticks_max = self.ticks_max.max(ticks);
    }

    /// The window closed after `window` observed frames (`None` before that, and for a 0 window), reset for the next one.
    pub fn window(&mut self, window: u32) -> Option<HurtWindow> {
        if window == 0 || self.frames < window {
            return None;
        }
        let closed = HurtWindow {
            frames: self.frames,
            parts: self.parts,
            parts_max: self.parts_max,
            first: self.first.unwrap_or(0),
            ticks_avg: self.ticks / u64::from(self.frames.max(1)),
            ticks_min: self.ticks_min,
            ticks_max: self.ticks_max,
        };
        self.frames = 0;
        self.ticks = 0;
        self.ticks_min = 0;
        self.ticks_max = 0;
        Some(closed)
    }
}

/// One closed census window: the fields of one `[Hitbox] hurt census` line (the runtime writes the prefix and the kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HurtWindow {
    pub frames: u32,
    pub parts: u32,
    pub parts_max: u32,
    pub first: u32,
    pub ticks_avg: u64,
    pub ticks_min: u64,
    pub ticks_max: u64,
}

impl core::fmt::Display for HurtWindow {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "parts={} parts_max={} first_pass={} frames={} ticks_min={} ticks_avg={} ticks_max={}",
            self.parts, self.parts_max, self.first, self.frames, self.ticks_min, self.ticks_avg, self.ticks_max
        )
    }
}

/// The production pass's planning and owned-handle dispatch. Returns counts, the total spheres wanted this pass (pending
/// `CATCH` spheres stay in the set) and how many of them are hit spheres. `hurt` is the hurtbox reader of the same per-frame
/// entry (decision HB1 (a)); route B (operator decision 2026-11-04, `docs/DECISIONS.md` HB1-route, `review/menu-proposal`
/// commit `5ca5e3b`) draws the enabled parts' spheres as yellow `sys_shield` spheres from the shared pool through the same
/// set and `Effects` interface as the hitboxes: `Some` plans them, `None` (the runtime's Off dispatch, and every stop path
/// the dispatch takes first) reads no HitModule and requests no hurt sphere (`docs/HB1-RENDER-ROUTES.md`).
pub fn reconcile_frame(set: &mut HandleSet, facts: FrameFacts, wants: &mut [Sphere], reader: &mut impl AttackReader, hurt: Option<&mut dyn HurtboxReader>, fx: &mut impl Effects) -> (Counts, usize) {
    if !facts.visualizes || !facts.enabled || facts.ids.shielding(facts.status) {
        return (set.clear(fx), 0);
    }
    if facts.ids.in_grab(facts.status) {
        return (set.reconcile(&[], true, fx), 0);
    }
    let mut len = 0;
    let mut left_out = want_hitboxes(reader, wants, &mut len);
    if let Some(hurt) = hurt {
        left_out += want_hurtboxes(hurt, wants, &mut len);
    }
    let mut counts = set.reconcile(&wants[..len], true, fx);
    counts.dropped += left_out;
    (counts, len)
}

/// What the plugin asks of the game's effect module for one fighter (the runtime calls the natives; the tests count).
pub trait Effects {
    /// Whether the effect behind `handle` still exists (the game ends a sphere at a status change).
    fn sphere_alive(&mut self, handle: u32) -> bool;
    /// End one of the plugin's own effects.
    fn sphere_kill_own(&mut self, handle: u32);
    /// Request a sphere; its handle, 0 when the game made none.
    fn sphere_spawn(&mut self, sphere: &Sphere) -> u32;
}

/// What one reconciliation did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Spheres kept as they were (one existence check each).
    pub kept: u32,
    pub spawned: u32,
    /// Own spheres killed (their shape changed or went away).
    pub killed: u32,
    /// Own spheres the game had already ended (dropped from the set, no call).
    pub ended: u32,
    /// Requests the game answered with no effect (its pool is full).
    pub failed: u32,
    /// Spheres left out because the set was full.
    pub dropped: u32,
}

#[derive(Clone, Copy, Debug)]
struct Held {
    sphere: Sphere,
    handle: u32,
    /// A grab sphere: requested once, kept until the game ends it or the rebound clears it, never requested again.
    once: bool,
}

/// One fighter's spheres: the handles the plugin holds and the grab spheres waiting for the pass. Fixed storage.
#[derive(Clone, Copy, Debug)]
pub struct HandleSet {
    held: [Held; MAX_HELD],
    len: usize,
    pending: [Sphere; MAX_PENDING],
    pending_len: usize,
}

impl Default for HandleSet {
    fn default() -> Self {
        HandleSet::new()
    }
}

impl HandleSet {
    pub const fn new() -> HandleSet {
        HandleSet {
            held: [Held { sphere: Sphere::NONE, handle: 0, once: false }; MAX_HELD],
            len: 0,
            pending: [Sphere::NONE; MAX_PENDING],
            pending_len: 0,
        }
    }

    /// Spheres held now.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0 && self.pending_len == 0
    }

    /// The handles held now (tests, diagnostics).
    pub fn handles(&self) -> impl Iterator<Item = u32> + '_ {
        self.held[..self.len].iter().map(|h| h.handle)
    }

    /// The `CATCH` command made a grab sphere: requested by the fighter's next pass. `false` when the queue is full.
    pub fn push_grab(&mut self, sphere: Sphere) -> bool {
        if self.pending_len == MAX_PENDING {
            return false;
        }
        self.pending[self.pending_len] = sphere;
        self.pending_len += 1;
        true
    }

    fn remove(&mut self, i: usize) {
        self.held[i] = self.held[self.len - 1];
        self.len -= 1;
    }

    fn add(&mut self, sphere: &Sphere, once: bool, fx: &mut impl Effects, c: &mut Counts) {
        if self.len == MAX_HELD {
            c.dropped += 1;
            return;
        }
        let handle = fx.sphere_spawn(sphere);
        if handle == 0 {
            c.failed += 1;
            return;
        }
        self.held[self.len] = Held { sphere: *sphere, handle, once };
        self.len += 1;
        c.spawned += 1;
    }

    /// Make the held spheres the `wants` (plus the grab spheres, kept while the game keeps them when `keep_grabs`): a held sphere
    /// whose want is unchanged and that still exists is kept; one whose want changed or went away is killed (when it still
    /// exists); one the game ended is dropped; every want not held is requested. Kills come before requests, so the pool gets
    /// back what the fighter gives up before it asks again. Waiting grab spheres are requested when `keep_grabs`, else dropped.
    pub fn reconcile(&mut self, wants: &[Sphere], keep_grabs: bool, fx: &mut impl Effects) -> Counts {
        let mut c = Counts::default();
        let mut i = 0;
        while i < self.len {
            let h = self.held[i];
            let wanted = if h.once { keep_grabs } else { wants.contains(&h.sphere) };
            let alive = fx.sphere_alive(h.handle);
            if wanted && alive {
                c.kept += 1;
                i += 1;
                continue;
            }
            if alive {
                fx.sphere_kill_own(h.handle);
                c.killed += 1;
            } else {
                c.ended += 1;
            }
            self.remove(i);
        }
        for w in wants {
            if !self.held[..self.len].iter().any(|h| !h.once && h.sphere.key == w.key) {
                self.add(w, false, fx, &mut c);
            }
        }
        if keep_grabs {
            for k in 0..self.pending_len {
                let p = self.pending[k];
                // The command sets a grab box again under the same id (a tether moving out): the old sphere of that key is
                // replaced, or kept when it is the same sphere and still exists.
                if let Some(j) = self.held[..self.len].iter().position(|h| h.once && h.sphere.key == p.key) {
                    let h = self.held[j];
                    let alive = fx.sphere_alive(h.handle);
                    if alive && h.sphere == p {
                        c.kept += 1;
                        continue;
                    }
                    if alive {
                        fx.sphere_kill_own(h.handle);
                        c.killed += 1;
                    }
                    self.remove(j);
                }
                self.add(&p, true, fx, &mut c);
            }
        }
        self.pending_len = 0;
        c
    }

    /// Kill every sphere still held and drop the waiting ones (the setting went Off, the fighter shields, the scene is no practice,
    /// a save state was loaded, a grab ended).
    pub fn clear(&mut self, fx: &mut impl Effects) -> Counts {
        self.reconcile(&[], false, fx)
    }

    /// Drop every record without a game call: the fighter object was initialised again (its effect module may reuse the numbers,
    /// so the old handles must not be used; the game ends the old spheres with the status they concluded with).
    pub fn forget(&mut self) {
        self.len = 0;
        self.pending_len = 0;
    }
}

/// The fighters' sets, by accessor address; a new fighter takes a free or the least recently used set (forgotten, no call).
#[derive(Clone, Copy, Debug)]
pub struct SetTable {
    keys: [usize; FIGHTER_SETS],
    used: [u64; FIGHTER_SETS],
    clock: u64,
    pub sets: [HandleSet; FIGHTER_SETS],
}

impl Default for SetTable {
    fn default() -> Self {
        SetTable::new()
    }
}

impl SetTable {
    pub const fn new() -> SetTable {
        SetTable { keys: [0; FIGHTER_SETS], used: [0; FIGHTER_SETS], clock: 0, sets: [HandleSet::new(); FIGHTER_SETS] }
    }

    /// The set of `key` (an accessor address), taken over when new.
    pub fn index_for(&mut self, key: usize) -> usize {
        self.clock += 1;
        let i = match self.keys.iter().position(|k| *k == key) {
            Some(i) => i,
            None => {
                let i = (0..FIGHTER_SETS).min_by_key(|&i| (self.keys[i] != 0, self.used[i])).unwrap_or(0);
                self.keys[i] = key;
                self.sets[i].forget();
                i
            }
        };
        self.used[i] = self.clock;
        i
    }

    /// The set of `key` if it has one.
    pub fn find(&self, key: usize) -> Option<usize> {
        self.keys.iter().position(|k| *k == key && key != 0)
    }

    /// Spheres held by every fighter (diagnostics: the plugin's share of the effect pool).
    pub fn held_spheres(&self) -> usize {
        self.sets.iter().map(|s| s.len()).sum()
    }
}
