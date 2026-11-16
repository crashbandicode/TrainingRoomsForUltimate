//! The rest of Ultimate Frame Data's legend for Hitbox Visualization (rc.14; the spots lane owns strong and weak): grab boxes
//! and the hurtboxes' states while *Frame data colours* is On, each hitbox's launch direction while *Launch angles* is On, and
//! the windboxes (`is_windbox`: a hitbox with knockback whose hit causes no hitstun, by the game's own no-reaction flag) in deep
//! blue, with orange wind areas and silver direction lines. Pure and host-tested; the offsets and values are the owned 13.0.5 ELF's (`tests/core06_hitbox.rs`
//! `legend_facts_match_the_owned_elf`) and Eden's (the design report).

use super::hitbox::sphere_key;
use super::hitbox_orbs::Orb;

/// Hitbox Visualization is one checklist (`hitbox_vis`, the operator's order of 2026-11-14 ~19:50Z): each tick draws on its own,
/// none ticked draws nothing. The items in menu order, as bits (bit i: item i): hitboxes, grab boxes, hurtboxes, the frame data
/// colours (applied to whatever is drawn), the launch angles and the windboxes (the sixth, the operator ~21:28Z).
pub const ROW: &str = "hitbox_vis";
pub const ITEMS: usize = 7;
pub const HITBOXES: u8 = 1;
pub const GRABS: u8 = 2;
pub const HURTBOXES: u8 = 4;
pub const COLOURS: u8 = 8;
pub const ANGLES: u8 = 16;
pub const WINDBOXES: u8 = 32;
pub const ARTICLE_HURTBOXES: u8 = 64;

/// The ticked items of the row as bits.
pub fn shown(row: &[u8]) -> u8 {
    row.iter().take(ITEMS).enumerate().fold(0, |bits, (i, v)| bits | (u8::from(*v != 0) << i))
}

/// Whether the visualizer draws anything: a box, the windboxes or the angles (the colours alone recolour nothing).
pub fn draws(bits: u8) -> bool {
    bits & (HITBOXES | GRABS | HURTBOXES | ANGLES | WINDBOXES | ARTICLE_HURTBOXES) != 0
}

/// Whether an orb of `kind` (`hitbox::KIND_*`, `KIND_MARK`, `KIND_WIND`, `KIND_AREA`) shows. A windbox is a hitbox (the operator
/// ~21:27Z): Hitboxes shows it as Windboxes does, and it is one orb either way, so it is never drawn twice. A wind area is not a
/// hitbox: Windboxes alone shows it.
pub fn shows(bits: u8, kind: u32) -> bool {
    bits & [0, HITBOXES, HURTBOXES, GRABS, ANGLES, HITBOXES | WINDBOXES, WINDBOXES, ARTICLE_HURTBOXES].get(kind as usize).copied().unwrap_or(0) != 0
}

/// The windboxes' orb kind (after `KIND_MARK`): a hitbox `is_windbox` names while the windboxes are asked for (*Windboxes* or
/// *Frame data colours* ticked); otherwise it stays a `hitbox::KIND_HIT` in its per-id colour, as before.
pub const KIND_WIND: u32 = 5;
/// Windboxes: deep blue (Ultimate Frame Data's legend), apart from the intangible hurtboxes' sky blue.
pub const WIND: [f32; 3] = [0.0, 0.15, 1.0];
/// Burnt orange separates wind areas from blue windboxes in both colour modes.
pub const WIND_AREA: [f32; 3] = [0.68, 0.30, 0.0];
/// `AttackData`'s no-reaction flag (ATTACK argument 26, a byte; `sv_animcmd::ATTACK` stores it at `main+0x2285880`). The
/// defender's DamageModule hit handler (`main+0x402cc0`) reads it; a hit with it set leaves the target out of hitstun (Eden,
/// the design report).
pub const NO_REACTION: usize = 0x70;
/// `AttackData`'s knockback words (i32: growth `+0x2c`, fixed `+0x30`, base `+0x34`), read as the two aligned quads at `+0x28`
/// (the angle, then the growth) and `+0x30` (fixed, then base).
pub const KNOCKBACK_QUADS: [usize; 2] = [0x28, 0x30];

/// A windbox (SmashWiki's terms, the operator ~21:25Z: it pushes or pulls without hitstun; most deal no damage): the game's
/// no-reaction flag on a hitbox with knockback (growth, fixed or base, either sign: a pull counts). Damage plays no part.
pub fn is_windbox(no_reaction: u8, knockback: [i32; 3]) -> bool {
    no_reaction != 0 && knockback.iter().any(|k| *k != 0)
}

/// An older two-option row (`[On, Off]`): On becomes the three boxes, Off nothing ticked. `None` for a row already a checklist.
pub fn migrate(row: &[u8]) -> Option<[u8; ITEMS]> {
    (row.len() == 2).then(|| if row[0] != 0 { [1, 1, 1, 0, 0, 0, 0] } else { [0; ITEMS] })
}
/// `AttackData`'s launch angle in degrees (i32, beside `hitbox::layout`).
pub const ANGLE: usize = 0x28;
/// `HitModule`'s group record: the whole status (`set_whole`) and the global status the xlu and invincible timers set.
pub const GROUP_WHOLE: usize = 0x1c;
pub const GROUP_GLOBAL: usize = 0x20;

/// `HitStatus`: 0 normal, 1 invincible (hit, no damage), 2 intangible (`xlu`, not hit), 3 off.
pub const INVINCIBLE: i32 = 1;
pub const INTANGIBLE: i32 = 2;
pub const OFF: i32 = 3;

/// Frame data colours On: grab boxes purple.
pub const GRAB: [f32; 3] = [0.6, 0.2, 1.0];
/// Hurtboxes: white normal, dark teal invincible, light sky blue intangible. Teal avoids a red/green-only distinction.
pub const HURT: [[f32; 3]; 3] = [[1.0; 3], [0.0, 0.5, 0.375], [0.45, 0.8, 1.0]];

/// A part's status as the game reads it (`HitModule::get_status`, `main+0x46a800`): the highest of its own word, the
/// group's whole status (at least 0) and the group's global status.
pub fn part_status(part: i32, whole: i32, global: i32) -> i32 {
    part.max(global.max(whole.max(0)))
}

/// A hurtbox's tint for its status: invincible teal; intangible sky blue, and so is a part the whole group switched off (OFF,
/// the revival platform: nothing hits it); any other white.
pub fn hurt_tint(status: i32) -> [f32; 3] {
    HURT[usize::try_from(status).ok().and_then(|s| [0, 1, 2, 2].get(s).copied()).unwrap_or(0)]
}

/// The direction 361's dashes point in, counter-clockwise from forward: the launch the game gave a target in the air, measured
/// on Eden (`hbl-trace-8fadea57-1` `t-hbl-361b`: 38.1 degrees, the first frame's fall taken from a 70-degree hit on the same CPU).
pub const SAKURAI_SHOWN: f32 = 38.0;

/// The unit vector of `degrees`, counter-clockwise from the forward of an attacker facing `lr` (+1 right, -1 left).
pub fn direction(degrees: f32, lr: f32) -> [f32; 2] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [if lr < 0.0 { -cos } else { cos }, sin]
}

/// The marks' orb kind (after `hitbox::KIND_GRAB`); their sphere numbers start at 0x10, so `Board::publish` never pairs them.
pub const KIND_MARK: u32 = 4;
/// Silver launch/wind lines, distinct from white normal hurtboxes in frame-data mode.
pub const DIRECTION_COLOUR: [u8; 4] = [176, 176, 176, 255];
/// A line's or dash's radius and a dot's, as fractions of the hitbox's.
pub const LINE: f32 = 0.1;
pub const DOT: f32 = 0.25;

/// The orbs marking hitbox orb `hit`'s launch angle `angle` (its owner facing `lr`), white: a line from the centre to the edge at a
/// fixed angle (0 to 360); two dashes where the game picks the direction when the hit lands (361); a centre dot where the hit
/// carries the target along (362 and above), or the angle is not one.
pub fn marks(hit: &Orb, angle: i32, lr: f32) -> [Option<Orb>; 2] {
    let index = ((hit.key >> 8) & 0xffff) as usize;
    let [x, y, z] = hit.centre;
    let seg = |n: usize, [dx, dy]: [f32; 2], from: f32, to: f32| {
        let at = |t: f32| [x + dx * hit.radius * t, y + dy * hit.radius * t, z];
        Orb { centre: at(from), tail: at(to), radius: hit.radius * LINE, color: super::panes::linear(DIRECTION_COLOUR), key: sphere_key(KIND_MARK, index, 0x10 + n) }
    };
    let dashes = direction(SAKURAI_SHOWN, lr);
    match angle {
        0..=360 => [Some(seg(0, direction(angle as f32, lr), 0.0, 1.0)), None],
        361 => [Some(seg(0, dashes, 0.1, 0.4)), Some(seg(1, dashes, 0.6, 0.9))],
        _ => [Some(Orb { radius: hit.radius * DOT, ..seg(0, [0.0; 2], 0.0, 0.0) }), None],
    }
}

// ---- wind areas (the primary ~22:00Z: pushes that are not hitboxes) -------------------------------------------------------

/// A fighter's wind areas: the game's `AREA_WIND_2ND*` animation commands add them to the fighter's AreaModule
/// (`add_wind_area_2nd` / `_rad`, vtable slots 0x158 / 0x160 of `FighterAreaModuleImpl`, `main+0x3d03e0` / `+0x3d0680`), and the
/// wind manager pushes every fighter inside one. Drawn orange under *Windboxes*, with a silver line along the wind.
pub mod area {
    /// The accessor's AreaModule.
    pub const MODULE: usize = 0xc0;
    /// Module `+0x18`: the area table; its record count (`+0x10`) and records (`+0x18`), `STRIDE` bytes each.
    pub const TABLE: usize = 0x18;
    pub const COUNT: usize = 0x10;
    pub const RECORDS: usize = 0x18;
    pub const STRIDE: usize = 0x70;
    /// Record bytes: its kind (`WIND`, the only kind the wind manager takes: `main+0x3cfee4`), shape (0 box, 1 circle: `+0x3d045c`),
    /// enabled (`is_enable_area`, `+0x3cf7b4`) and the wind set on it (`erase_wind`, `+0x3cfcc8`); `SHAPE`: the shape object.
    pub const KIND: usize = 0x1;
    pub const WIND: u8 = 0x19;
    pub const SHAPE_TYPE: usize = 0x60;
    pub const ENABLED: usize = 0x61;
    pub const WIND_SET: usize = 0x63;
    pub const SHAPE: usize = 0x28;
    /// Shape object: a box's corners (min, max: x, y), a circle's centre and radius, the wind id (`add_wind_area_2nd`).
    pub const BOX_MIN: usize = 0x40;
    pub const BOX_MAX: usize = 0x50;
    pub const CENTRE: usize = 0x80;
    pub const RADIUS: usize = 0x90;
    pub const ID: usize = 0x70;
    /// The wind manager (`main+0x52b85e8` holds a pointer to a pointer to its entry vector, begin and end): `ENTRY` bytes each,
    /// the wind id first and the wind's angle in degrees at `ENTRY_ANGLE` (the animation command's angle, mirrored by the
    /// fighter's facing when it was set).
    pub const MANAGER_SLOT: usize = 0x52b_85e8;
    pub const ENTRY: usize = 0x28;
    pub const ENTRY_ANGLE: usize = 0xc;
    /// Bounds: records read per fighter, entries searched, areas drawn per fighter.
    pub const MAX_RECORDS: usize = 32;
    pub const MAX_ENTRIES: usize = 128;
    pub const MAX_AREAS: usize = 4;
}

/// Reads for `wind_areas` (the runtime reads game memory, type-checked; the tests a fake).
pub trait AreaReader {
    fn area_usize(&mut self, at: usize) -> usize;
    fn area_u8(&mut self, at: usize) -> u8;
    fn area_u32(&mut self, at: usize) -> u32;
    fn area_f32(&mut self, at: usize) -> f32;
}

/// A wind area in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AreaShape {
    Box { min: [f32; 2], max: [f32; 2] },
    Circle { centre: [f32; 2], radius: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindArea {
    pub shape: AreaShape,
    /// The wind's direction in degrees, counter-clockwise from screen right; `None` when the manager has no entry for it.
    pub angle: Option<f32>,
}

impl WindArea {
    pub const NONE: WindArea = WindArea { shape: AreaShape::Circle { centre: [0.0; 2], radius: 0.0 }, angle: None };
}

fn pointer(at: usize) -> bool {
    at != 0 && at.is_multiple_of(8)
}

/// The enabled wind areas of the AreaModule at `module` into `out`, with each one's angle from the wind manager's entry vector
/// (`entries`: the vector's address, `[begin, end]`; 0 when there is none). How many were written. A table or a pointer that
/// does not look like one gives nothing; a shape that is not finite, or empty, is left out.
pub fn wind_areas(reader: &mut impl AreaReader, module: usize, entries: usize, out: &mut [WindArea]) -> usize {
    use area::*;
    if !pointer(module) {
        return 0;
    }
    let table = reader.area_usize(module + TABLE);
    if !pointer(table) {
        return 0;
    }
    let (count, records) = (reader.area_usize(table + COUNT).min(MAX_RECORDS), reader.area_usize(table + RECORDS));
    if !pointer(records) {
        return 0;
    }
    let mut n = 0;
    for i in 0..count {
        if n == out.len() {
            break;
        }
        let rec = records + i * STRIDE;
        if reader.area_u8(rec + KIND) != WIND || reader.area_u8(rec + ENABLED) == 0 || reader.area_u8(rec + WIND_SET) == 0 {
            continue;
        }
        let shape = reader.area_usize(rec + SHAPE);
        if !pointer(shape) {
            continue;
        }
        let ty = reader.area_u8(rec + SHAPE_TYPE);
        let area = {
            let mut f = |at: usize| reader.area_f32(shape + at);
            match ty {
                0 => AreaShape::Box { min: [f(BOX_MIN), f(BOX_MIN + 4)], max: [f(BOX_MAX), f(BOX_MAX + 4)] },
                1 => AreaShape::Circle { centre: [f(CENTRE), f(CENTRE + 4)], radius: f(RADIUS) },
                _ => continue,
            }
        };
        let usable = match area {
            AreaShape::Box { min, max } => min.iter().chain(&max).all(|v| v.is_finite()) && max[0] > min[0] && max[1] > min[1],
            AreaShape::Circle { centre, radius } => centre.iter().all(|v| v.is_finite()) && radius.is_finite() && radius > 0.0,
        };
        if !usable {
            continue;
        }
        let id = reader.area_u32(shape + ID);
        out[n] = WindArea { shape: area, angle: entry_angle(reader, entries, id) };
        n += 1;
    }
    n
}

fn entry_angle(reader: &mut impl AreaReader, entries: usize, id: u32) -> Option<f32> {
    use area::*;
    if !pointer(entries) {
        return None;
    }
    let (begin, end) = (reader.area_usize(entries), reader.area_usize(entries + 8));
    if !pointer(begin) || end < begin || (end - begin) / ENTRY > MAX_ENTRIES {
        return None;
    }
    (0..(end - begin) / ENTRY)
        .map(|k| begin + k * ENTRY)
        .find(|e| reader.area_u32(*e) == id)
        .map(|e| reader.area_f32(e + ENTRY_ANGLE))
        .filter(|a| a.is_finite())
}

/// The wind areas' orb kind (after `KIND_WIND`): shown by *Windboxes* only (an area is not a hitbox). Sphere numbers: a box's band
/// `AREA_BOX` (the layout draws its band alone, no round ends), a circle's disc `AREA_CIRCLE`, the wind line `AREA_LINE`.
pub const KIND_AREA: u32 = 6;
pub const AREA_BOX: u32 = 0x20;
pub const AREA_CIRCLE: u32 = 0x21;
pub const AREA_LINE: u32 = 0x22;
pub const AREA_SHADOW: u32 = 0x23;
/// Orange wind areas, at full pane alpha like every other orb. The band texture is only 0.30 opaque in
/// its middle (`tools/layout-builder` `orb_texture`), so the fighters inside stay visible; at 110 Eden showed (226, 226, 242) over
/// the Training grid's 240 white, too pale to read (hb-legend-finish, 2026-11-15).
pub const AREA_ALPHA: u8 = 255;
/// The wind line's shadow: the line's pill `SHADOW_GROW` times as wide, near-black navy and opaque, drawn under the silver line.
pub const SHADOW: [u8; 3] = [0, 0, 40];
pub const SHADOW_GROW: f32 = 1.6;

/// The areas' pane colour: `WIND_AREA` as the layout takes it (linear light), at `AREA_ALPHA`.
pub fn area_colour() -> [u8; 4] {
    super::panes::linear(super::hitbox_orbs::tint_colour(WIND_AREA, AREA_ALPHA))
}

/// Area `index`'s orbs at depth `z`, in the order they draw (a later pane draws over an earlier one): its shape (a box as a band from
/// its left to its right edge, as tall as the box; a circle as a disc) in translucent `colour`, the dark shadow of its line (a wider
/// pill under it, so the silver line keeps an outline over a pale or a dark background) and a silver line from its centre to its edge
/// along the wind (none, and no shadow, without an angle).
pub fn area_orbs(area: &WindArea, index: usize, z: f32, colour: [u8; 4]) -> [Option<Orb>; 3] {
    let (centre, shape, reach) = match area.shape {
        AreaShape::Box { min, max } => {
            let (cy, half) = ((min[1] + max[1]) / 2.0, [(max[0] - min[0]) / 2.0, (max[1] - min[1]) / 2.0]);
            let orb = Orb { centre: [min[0], cy, z], tail: [max[0], cy, z], radius: half[1], color: colour, key: sphere_key(KIND_AREA, index, AREA_BOX as usize) };
            ([min[0] + half[0], cy], orb, half)
        }
        AreaShape::Circle { centre, radius } => {
            let at = [centre[0], centre[1], z];
            (centre, Orb { centre: at, tail: at, radius, color: colour, key: sphere_key(KIND_AREA, index, AREA_CIRCLE as usize) }, [radius; 2])
        }
    };
    let line = area.angle.map(|degrees| {
        if degrees > 360.0 {
            // 361: the wind has no one direction (Eden: Meta Knight's Mach Tornado, Mewtwo's Shadow Ball, Wario Waft's circles): a
            // centre dot, as the launch angles mark a hit with no fixed direction.
            let at = [centre[0], centre[1], z];
            return Orb { centre: at, tail: at, radius: reach[0].min(reach[1]) * DOT, color: super::panes::linear(DIRECTION_COLOUR), key: sphere_key(KIND_AREA, index, AREA_LINE as usize) };
        }
        let [dx, dy] = direction(degrees, 1.0);
        // To the edge: a circle's radius; a box's nearest side along the direction.
        let t = match area.shape {
            AreaShape::Circle { radius, .. } => radius,
            AreaShape::Box { .. } => [(reach[0], dx), (reach[1], dy)].iter().filter(|(_, d)| d.abs() > 1e-6).map(|(h, d)| h / d.abs()).fold(f32::MAX, f32::min),
        };
        let thick = reach[0].min(reach[1]) * LINE;
        Orb { centre: [centre[0], centre[1], z], tail: [centre[0] + dx * t, centre[1] + dy * t, z], radius: thick, color: super::panes::linear(DIRECTION_COLOUR), key: sphere_key(KIND_AREA, index, AREA_LINE as usize) }
    });
    let shadow = line.map(|l| Orb {
        radius: l.radius * SHADOW_GROW,
        color: super::panes::linear([SHADOW[0], SHADOW[1], SHADOW[2], 255]),
        key: sphere_key(KIND_AREA, index, AREA_SHADOW as usize),
        ..l
    });
    [Some(shape), shadow, line]
}

// ---- articles' and items' wind areas (rc.14 wind-articles) ----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Swatch {
    Colour([u8; 4]),
    HitboxShades,
}

impl Swatch {
    pub fn colours(self) -> impl Iterator<Item = [u8; 4]> {
        (0..if self == Self::HitboxShades { 8 } else { 1 }).map(move |i| match self {
            Self::Colour(colour) => colour,
            Self::HitboxShades => super::hitbox_orbs::tint_colour(super::hitbox::TINTS[i], super::hitbox_orbs::ORB_ALPHA),
        })
    }
}

/// Fixed colour-name and meaning columns after the shared 128-unit swatch column.
pub fn swatch_columns(left: f32, right: f32) -> [(f32, f32); 2] {
    [(left + 140.0, 132.0), (left + 284.0, right - left - 284.0)]
}

/// The current checklist's colour key, in sRGB (the UI converts once); only enabled drawing families appear.
pub fn swatches(bits: u8) -> impl Iterator<Item = (&'static str, Swatch)> {
    use super::hitbox as h;
    use super::hitbox_orbs::{tint_colour as rgba, ORB_ALPHA};
    let on = bits & COLOURS != 0;
    let hurt = bits & (HURTBOXES | ARTICLE_HURTBOXES) != 0;
    let wind = bits & WINDBOXES != 0;
    let (names, colours): (&[_], &[_]) = if on {
        (&["Red — Strong hitbox", "Pink — Weak hitbox"], &[h::STRONG_TINT, h::WEAK_TINT])
    } else {
        (&["Multi-colour — Hitbox (attack): one shade per hitbox"], &h::TINTS)
    };
    let hits = (0..names.len()).filter(move |_| bits & HITBOXES != 0).map(move |i| (names[i], if on { Swatch::Colour(rgba(colours[i], ORB_ALPHA)) } else { Swatch::HitboxShades }));
    let c = |status| h::legend_colour(h::KIND_HURT, None, status, h::HURTBOX_TINT, on);
    let grab = h::legend_colour(h::KIND_GRAB, None, 0, h::GRAB_TINT, on);
    let other = [
        (bits & GRABS != 0, if on { "Purple — Grab box (grabs you)" } else { "Magenta — Grab box (grabs you)" }, grab),
        (hurt, if on { "White — Hurtbox (can be hit)" } else { "Yellow — Hurtbox (can be hit)" }, c(0)),
        (hurt && on, "Sky blue — Hurtbox (cannot be hit)", c(INTANGIBLE)),
        (hurt && on, "Teal — Hurtbox (takes no damage)", c(INVINCIBLE)),
        (wind || on && bits & HITBOXES != 0, "Blue — Windbox (pushes without flinching)", WIND),
        (wind, "Burnt orange — Wind area (pushes in a box or circle)", WIND_AREA),
    ].into_iter().filter(|(show, ..)| *show).map(|(_, n, t)| (n, Swatch::Colour(rgba(t, ORB_ALPHA))));
    hits.chain(other)
        .chain((bits & (ANGLES | WINDBOXES) != 0).then_some(("Silver — Launch / wind direction", Swatch::Colour(DIRECTION_COLOUR))))
}

/// A wind-area command (`AREA_WIND_2ND`, `_arg10`, `_RAD`, `_RAD_arg9`: `sites_hitbox`) tracks the object running it as an article,
/// as ATTACK and CATCH do, while the visualizer is on with *Windboxes* ticked and the practice visualizes the object. A fighter's
/// areas stay with its own pass. 30 such commands on 17 fighters run in article scripts (Zelda's Triforce of Wisdom, Hero's Bang
/// and Kaboom, the Final Smashes' black holes; `artifacts/rc14-wind-audit/out/windareas.txt`).
pub fn wind_tracks(enabled: bool, windboxes: bool, fighter: bool, visualizes: bool) -> bool {
    enabled && windboxes && !fighter && visualizes
}

/// An active item is tracked as an article while it has a live attack slot or an enabled wind area (`wind_areas`, read only while
/// *Windboxes* is ticked).
pub fn tracks_item(live_slot: bool, wind_areas: usize) -> bool {
    live_slot || wind_areas > 0
}

/// A tracked article stays while it holds a sphere (a live hitbox or a waiting grab box) or an enabled wind area.
pub fn keeps_article(holds: bool, wind_areas: usize) -> bool {
    holds || wind_areas > 0
}

#[cfg(test)]
#[path = "legend_tests.rs"]
mod tests;
