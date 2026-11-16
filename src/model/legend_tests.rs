use super::*;
use crate::model::hitbox::{self, layout, HandleSet, HurtboxReader, Sphere, KIND_GRAB, KIND_HIT, KIND_HURT};
use crate::model::hitbox_orbs::{self as orbs, Board};
use crate::model::spots::Spot;

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
}

#[test]
fn legend_part_status_is_the_games_max() {
    // (part word, group whole, group global) -> status, as `HitModule::get_status` (main+0x46a800) computes it: a spot
    // dodge's whole status, a respawn's global timer and a part set on its own each win over a normal part.
    let cases = [
        (0, 0, 0, 0),
        (0, -1, 0, 0),
        (0, INTANGIBLE, 0, INTANGIBLE),
        (0, 0, INVINCIBLE, INVINCIBLE),
        (INVINCIBLE, INTANGIBLE, 0, INTANGIBLE),
        (INTANGIBLE, 0, INVINCIBLE, INTANGIBLE),
        (3, 0, INTANGIBLE, 3),
        (0, -5, -1, 0),
    ];
    for (part, whole, global, want) in cases {
        assert_eq!(part_status(part, whole, global), want, "part {part} whole {whole} global {global}");
    }
}

#[test]
fn legend_hurtbox_status_colours() {
    assert_eq!(hurt_tint(0), [1.0, 1.0, 1.0], "normal: white");
    assert_eq!(hurt_tint(INVINCIBLE), [0.0, 0.5, 0.375], "invincible: teal");
    assert_eq!(hurt_tint(INTANGIBLE), [0.45, 0.8, 1.0], "intangible: sky blue");
    assert_eq!(hurt_tint(OFF), [0.45, 0.8, 1.0], "switched off as a whole (the revival platform): nothing hits it, drawn as intangible");
    assert_eq!(hurt_tint(-1), [1.0, 1.0, 1.0]);
    assert!(![hurt_tint(INTANGIBLE), hurt_tint(INVINCIBLE), hurt_tint(0)].contains(&GRAB));
}

#[test]
fn legend_hits_keep_the_strong_and_weak_colours() {
    let usual = [0.1, 0.2, 0.3];
    for spot in [None, Some(Spot::Sweet), Some(Spot::Sour), Some(Spot::Late)] {
        let on = hitbox::legend_colour(KIND_HIT, spot, 0, usual, true);
        assert!(on == hitbox::STRONG_TINT || on == hitbox::WEAK_TINT, "{spot:?}");
    }
}

#[test]
fn legend_windbox_is_knockback_without_hitstun() {
    // The game's no-reaction flag (`AttackData +0x70`, ATTACK argument 26) with knockback; damage plays no part.
    assert!(is_windbox(1, [100, 0, 0]) && is_windbox(1, [0, 30, 0]) && is_windbox(1, [0, 0, 20]));
    assert!(is_windbox(1, [0, 0, -20]), "a pull's negative knockback counts");
    assert!(!is_windbox(1, [0, 0, 0]), "no knockback: nothing is pushed");
    assert!(!is_windbox(0, [100, 70, 13]), "knockback with hitstun: a hit");
    assert_eq!(legend_colour_of_wind(), WIND);
    // Deep blue, apart from the intangible hurtboxes' sky blue and every other legend colour.
    for other in [hitbox::STRONG_TINT, hitbox::WEAK_TINT, HURT[0], HURT[1], HURT[2], GRAB] {
        assert!(WIND.iter().zip(other).map(|(a, b)| (a - b).abs()).sum::<f32>() > 0.5, "{other:?}");
    }
}

fn legend_colour_of_wind() -> [f32; 3] {
    let colours = [false, true].map(|on| hitbox::legend_colour(KIND_WIND, None, 0, [0.1, 0.2, 0.3], on));
    assert_eq!(colours[0], colours[1], "a windbox is deep blue whether or not Frame data colours is ticked");
    colours[1]
}

/// Attack slots for `want_hits`: (damage, knockback growth / fixed / base, the +0x70 flag, the +0x6e flag), one point sphere each.
struct Attacks {
    slots: Vec<(f32, [i32; 3], u8, u8)>,
    reads: Vec<usize>,
}

impl hitbox::AttackReader for Attacks {
    fn active(&mut self, slot: i32) -> bool {
        (slot as usize) < self.slots.len()
    }
    fn data(&mut self, slot: i32) -> usize {
        0x1000 * (slot as usize + 1)
    }
    fn read_u64(&mut self, at: usize) -> u64 {
        self.reads.push(at % 0x1000);
        let (_, [kbg, fkb, bkb], f70, f6e) = self.slots[at / 0x1000 - 1];
        let pair = |lo: i32, hi: i32| u64::from(lo as u32) | (u64::from(hi as u32) << 32);
        match at % 0x1000 {
            0x28 => pair(361, kbg),
            0x30 => pair(fkb, bkb),
            0x68 => u64::from(f6e) << 48,
            0x70 => u64::from(f70),
            _ => 0,
        }
    }
    fn read_f32(&mut self, at: usize) -> f32 {
        self.reads.push(at % 0x1000);
        match at % 0x1000 {
            layout::POWER => self.slots[at / 0x1000 - 1].0,
            layout::SIZE => 2.0,
            _ => 0.0,
        }
    }
}

/// F.L.U.D.D. (0 %, both flags), a damaging windbox (5 %, +0x70), Peach Bomber (12 %, +0x6e alone: hitstun), a jab (neither)
/// and Sing-like 0 % with the flag but no knockback.
fn cases() -> Attacks {
    Attacks {
        slots: vec![(0.0, [100, 70, 13], 1, 1), (5.0, [100, 0, 40], 1, 0), (12.0, [73, 0, 60], 0, 1), (2.2, [25, 0, 3], 0, 0), (0.0, [0, 0, 0], 1, 1)],
        reads: Vec::new(),
    }
}

#[test]
fn legend_windboxes_by_the_flag_whatever_the_damage() {
    let (mut out, mut len) = ([Sphere::NONE; 8], 0);
    let mut attacks = cases();
    assert_eq!(hitbox::want_hits(&mut attacks, &mut out, &mut len, true), 0);
    let kinds: Vec<u32> = out[..len].iter().map(|s| s.key >> 24).collect();
    assert_eq!(kinds, [KIND_WIND, KIND_WIND, KIND_HIT, KIND_HIT, KIND_HIT]);
    assert!(out[..2].iter().all(|s| s.tint == WIND) && out[2..5].iter().all(|s| s.tint != WIND));
    assert_eq!(out[..len].iter().map(|s| (s.key >> 8) & 0xffff).collect::<Vec<_>>(), [0, 1, 2, 3, 4], "one sphere a slot, its own slot");
    // Not asked for (Hitboxes alone, no Frame data colours): today's hits in their per-id colours, the flag never read.
    let (mut today, mut n) = ([Sphere::NONE; 8], 0);
    let mut plain = cases();
    hitbox::want_hits(&mut plain, &mut today, &mut n, false);
    assert!(today[..n].iter().enumerate().all(|(i, s)| s.key == sphere_key(KIND_HIT, i, 0) && s.tint == hitbox::hitbox_tint(i as i32)));
    assert!(!plain.reads.iter().any(|at| [0x28, 0x30, 0x70].contains(at)), "{:?}", plain.reads);
    let (mut old, mut m) = ([Sphere::NONE; 8], 0);
    hitbox::want_hitboxes(&mut cases(), &mut old, &mut m);
    assert_eq!(old[..m], today[..n], "want_hitboxes (Eggs-ercise's reader) is unchanged");
}

#[test]
fn legend_windboxes_and_hitboxes_combine() {
    // The operator (~21:27Z): a windbox is a hitbox. Hitboxes ticked draws every hitbox, windboxes included (per-id colours, or the
    // legend's with Frame data colours, windboxes deep blue); Windboxes ticked draws the windboxes deep blue; both ticked draw each
    // windbox once; Windboxes alone draws only the windboxes.
    let row = |hits: u8, wind: u8, colours: u8| shown(&[hits, 0, 0, colours, 0, wind]);
    let drawn = |bits: u8| {
        let mut set = HandleSet::new();
        set.frame_data = bits & COLOURS != 0;
        set.wind = bits & WINDBOXES != 0;
        let (mut wants, mut len) = ([Sphere::NONE; 8], 0);
        let mut attacks = cases();
        hitbox::want_hits(&mut attacks, &mut wants, &mut len, set.identifies_wind());
        set.tint(&mut attacks, &mut wants[..len], 1);
        set.reconcile(&wants[..len], true, &mut orbs::OrbSink { status: 1 });
        let mut out = [Orb::NONE; orbs::BOARD_ORBS];
        let n = orbs::place_orbs(&set, |s| Some(s.pos), None, |_| None, &mut out);
        let mut shownorbs: Vec<(u32, u32, [u8; 4])> =
            out[..n].iter().filter(|o| shows(bits, o.kind())).map(|o| ((o.key >> 8) & 0xffff, o.kind(), o.color)).collect();
        shownorbs.sort();
        shownorbs
    };
    let blue = crate::model::panes::linear([0, 38, 255, 255]);
    let slots = |v: &[(u32, u32, [u8; 4])]| v.iter().map(|o| o.0).collect::<Vec<_>>();
    // Hitboxes only: all five hitboxes, the windboxes in their per-id colours.
    let hits = drawn(row(1, 0, 0));
    assert_eq!(slots(&hits), [0, 1, 2, 3, 4]);
    assert!(hits.iter().all(|o| o.1 == KIND_HIT && o.2 != blue));
    // Hitboxes with Frame data colours: all five, the two windboxes deep blue, the rest the legend's red.
    let legend = drawn(row(1, 0, 1));
    assert_eq!(slots(&legend), [0, 1, 2, 3, 4]);
    assert!(legend[..2].iter().all(|o| o.1 == KIND_WIND && o.2 == blue) && legend[2..].iter().all(|o| o.2 != blue));
    // Windboxes only: just the two windboxes, deep blue.
    let wind = drawn(row(0, 1, 0));
    assert_eq!(slots(&wind), [0, 1]);
    assert!(wind.iter().all(|o| o.1 == KIND_WIND && o.2 == blue));
    // Both: every hitbox once, the windboxes deep blue among the others (per-id colours without Frame data colours).
    let both = drawn(row(1, 1, 0));
    assert_eq!(slots(&both), [0, 1, 2, 3, 4], "each windbox drawn once");
    assert!(both[..2].iter().all(|o| o.2 == blue) && both[2..].iter().all(|o| o.1 == KIND_HIT && o.2 != blue));
    // Nothing else ticked draws a windbox; its marks are the hitbox marks.
    assert!(draws(WINDBOXES) && shows(WINDBOXES, KIND_WIND) && !shows(WINDBOXES, KIND_HIT));
    assert!([GRABS, HURTBOXES, COLOURS, ANGLES].iter().all(|b| !shows(*b, KIND_WIND)));
}
fn hit(slot: usize) -> Orb {
    Orb { centre: [10.0, 5.0, 1.0], tail: [12.0, 5.0, 1.0], radius: 4.0, color: [255, 0, 0, 255], key: sphere_key(KIND_HIT, slot, 0) }
}

#[test]
fn legend_angle_line_points_along_the_angle_mirrored_by_facing() {
    let r = 4.0 * std::f32::consts::FRAC_1_SQRT_2;
    let cases = [
        (0, 1.0, [14.0, 5.0, 1.0]),
        (0, -1.0, [6.0, 5.0, 1.0]),
        (45, 1.0, [10.0 + r, 5.0 + r, 1.0]),
        (45, -1.0, [10.0 - r, 5.0 + r, 1.0]),
        (90, -1.0, [10.0, 9.0, 1.0]),
        (180, 1.0, [6.0, 5.0, 1.0]),
        (270, -1.0, [10.0, 1.0, 1.0]),
        (360, -1.0, [6.0, 5.0, 1.0]),
    ];
    for (angle, lr, end) in cases {
        let [Some(line), None] = marks(&hit(3), angle, lr) else { panic!("angle {angle}: one line") };
        assert!(close(line.centre, [10.0, 5.0, 1.0]) && close(line.tail, end), "angle {angle} lr {lr}: {line:?}");
        assert_eq!((line.key >> 24, (line.key >> 8) & 0xffff), (KIND_MARK, 3), "its own kind, the hitbox's slot");
        assert_ne!(line.key & 0xff, 1, "never paired as a capsule end");
        assert!(line.radius < hit(3).radius && line.color == crate::model::panes::linear(DIRECTION_COLOUR));
    }
}

#[test]
fn legend_special_angles_are_dashed_or_a_dot() {
    // 0 and 360 are lines (the line test), 361 two dashes, 362 and above (and a negative) one dot.
    let shape = |angle| marks(&hit(0), angle, 1.0).map(|m| m.map(|o| (o.centre == o.tail, o.radius)));
    for angle in [0, 360] {
        assert!(matches!(shape(angle), [Some((false, _)), None]), "angle {angle}");
    }
    assert!(matches!(shape(361), [Some((false, _)), Some((false, _))]));
    for angle in [362, 363, 365, 366, 367, 368, 400, -1] {
        assert_eq!(shape(angle), [Some((true, 4.0 * DOT)), None], "angle {angle}");
    }
    assert_eq!(SAKURAI_SHOWN, 38.0, "the Eden measurement (design report)");
    // 361: two dashes along the shown direction, apart, inside the hitbox; mirrored by facing.
    for lr in [1.0, -1.0] {
        let [Some(a), Some(b)] = marks(&hit(0), 361, lr) else { panic!("two dashes") };
        let [dx, dy] = direction(SAKURAI_SHOWN, lr);
        let along = |p: [f32; 3]| ((p[0] - 10.0) * dx + (p[1] - 5.0) * dy) / 4.0;
        assert!(0.0 < along(a.centre) && along(a.tail) < along(b.centre) && along(b.tail) < 1.0, "{a:?} {b:?}");
        assert_ne!(a.key, b.key);
    }
    // Autolink: one dot on the centre, bigger than a line, smaller than the hitbox.
    let [Some(dot), None] = marks(&hit(0), 367, 1.0) else { panic!("one dot") };
    assert!(close(dot.centre, [10.0, 5.0, 1.0]) && close(dot.tail, dot.centre));
    assert!(dot.radius > 4.0 * LINE && dot.radius < 4.0);
}

#[test]
fn legend_grab_is_purple_while_frame_data_colours_is_on() {
    let colour = |on| hitbox::legend_colour(KIND_GRAB, None, 0, hitbox::GRAB_TINT, on);
    assert_eq!((colour(false), colour(true)), (hitbox::GRAB_TINT, GRAB), "Off: today's magenta; On: purple");
    // Purple is none of the On colours of hits and hurtboxes.
    for other in [hitbox::STRONG_TINT, hitbox::WEAK_TINT, HURT[0], HURT[1], HURT[2]] {
        assert!(GRAB.iter().zip(other).map(|(a, b)| (a - b).abs()).sum::<f32>() > 0.5, "{other:?}");
    }
}

#[test]
fn legend_off_keeps_todays_tints() {
    let usual = [0.1, 0.2, 0.3];
    for kind in [KIND_HIT, KIND_HURT, KIND_GRAB] {
        for spot in [None, Some(Spot::Sweet), Some(Spot::Sour), Some(Spot::Late)] {
            for status in [0, INVINCIBLE, INTANGIBLE, OFF] {
                assert_eq!(hitbox::legend_colour(kind, spot, status, usual, false), usual, "{kind} {spot:?} {status}");
            }
        }
    }
    // On: a hurtbox takes its status' colour.
    assert_eq!([0, INVINCIBLE, INTANGIBLE].map(|s| hitbox::legend_colour(KIND_HURT, None, s, usual, true)), HURT);
    // The hurt pass: Off every part is today's yellow and the group's words are not read; On each part takes the game's status.
    for (legend, whole, words, want) in [
        (false, INTANGIBLE, [0, 1], [hitbox::HURTBOX_TINT; 2]),
        (true, 0, [0, INVINCIBLE], [HURT[0], HURT[1]]),
        (true, INTANGIBLE, [0, INVINCIBLE], [HURT[2]; 2]),
        (true, -1, [INTANGIBLE, 0], [HURT[2], HURT[0]]),
    ] {
        let mut hurt = Hurt { whole, words, reads: Vec::new() };
        let (mut out, mut len) = ([Sphere::NONE; 4], 0);
        assert_eq!(hitbox::want_hurt(&mut hurt, &mut out, &mut len, legend, hitbox::KIND_HURT), 0);
        assert_eq!([out[0].tint, out[1].tint], want, "legend {legend} whole {whole} words {words:?}");
        assert_eq!(hurt.reads.contains(&(GROUPS + GROUP_WHOLE)), legend, "the whole word is read only while On");
    }
}

const MODULE: usize = 0x100;
const GROUPS: usize = 0x200;
const PARTS: usize = 0x400;

/// Two hurt parts in group 0 with the given words; the group's whole status `whole`, global 0.
struct Hurt {
    whole: i32,
    words: [i32; 2],
    reads: Vec<usize>,
}

impl HurtboxReader for Hurt {
    fn module(&mut self) -> usize {
        MODULE
    }
    fn read_usize(&mut self, at: usize) -> usize {
        match at - MODULE {
            layout::HIT_GROUPS => GROUPS,
            layout::HIT_PARTS => PARTS,
            _ => 0,
        }
    }
    fn read_i32(&mut self, at: usize) -> i32 {
        self.reads.push(at);
        match at {
            _ if at == MODULE + layout::HIT_PART_TOTAL => 2,
            _ if at == GROUPS + GROUP_WHOLE => self.whole,
            _ => 0,
        }
    }
    fn read_i16(&mut self, at: usize) -> i16 {
        i16::from(at == GROUPS + layout::HIT_GROUP_COUNT) * 2
    }
    fn read_part(&mut self, at: usize, record: &mut [u8; layout::HIT_PART_STRIDE]) {
        let index = (at - PARTS) / layout::HIT_PART_STRIDE;
        *record = [0; layout::HIT_PART_STRIDE];
        record[..4].copy_from_slice(&self.words[index].to_le_bytes());
        record[layout::PART_SIZE..layout::PART_SIZE + 4].copy_from_slice(&2.0f32.to_le_bytes());
        record[layout::PART_OFFSET..layout::PART_OFFSET + 4].copy_from_slice(&(index as f32).to_le_bytes());
    }
}

#[test]
fn legend_marks_follow_their_hitbox_and_draw_last() {
    // A set holding a point hitbox (slot 2), a capsule hitbox (slot 5, two spheres) and a hurt sphere.
    let mut set = HandleSet::new();
    let sphere = |kind, index, end, x: f32| Sphere { key: sphere_key(kind, index, end), joint: 1, pos: [x, 0.0, 0.0], scale: 0.5, tint: [1.0; 3] };
    let wants = [sphere(KIND_HIT, 2, 0, 0.0), sphere(KIND_HIT, 5, 0, 10.0), sphere(KIND_HIT, 5, 1, 14.0), sphere(KIND_HURT, 0, 0, 20.0)];
    set.reconcile(&wants, true, &mut orbs::OrbSink { status: 1 });
    let angle = |slot| Some([0, 0, 45, 0, 0, 361][slot as usize]);
    let mut out = [orbs::Orb::NONE; orbs::BOARD_ORBS];
    // Off: the orbs only.
    assert_eq!(orbs::place_orbs(&set, |s| Some(s.pos), None, angle, &mut out), 4);
    // On: one line for slot 2, two dashes for slot 5 (from its first sphere), nothing for the second end or the hurt sphere.
    let n = orbs::place_orbs(&set, |s| Some(s.pos), Some(-1.0), angle, &mut out);
    assert_eq!(out[..n].iter().filter(|o| o.kind() == KIND_MARK).count(), 3);
    assert_eq!(out[..n].iter().filter(|o| o.kind() == KIND_MARK && (o.key >> 8) & 0xffff == 5).count(), 2);
    // Facing left, the 45-degree line goes up and to the left of its hitbox.
    let line = out[..n].iter().find(|o| o.kind() == KIND_MARK && (o.key >> 8) & 0xffff == 2).unwrap();
    assert!(line.tail[0] < line.centre[0] && line.tail[1] > line.centre[1]);
    // The board keeps them and draws them after every hitbox and hurtbox.
    let mut board = Board::new();
    board.publish(0x80, out[..n].iter().copied());
    let kinds: Vec<u32> = board.in_draw_order().map(|o| o.kind()).collect();
    assert_eq!(kinds, [KIND_HIT, KIND_HIT, KIND_HURT, KIND_MARK, KIND_MARK, KIND_MARK]);
}
#[test]
fn legend_checklist_items_draw_independently() {
    let bits = |row: &[u8]| shown(row);
    assert_eq!(bits(&[0; ITEMS]), 0);
    assert!(!draws(bits(&[0; ITEMS])), "nothing ticked draws nothing");
    assert!(!draws(bits(&[0, 0, 0, 1, 0])), "the colours alone recolour nothing");
    // Hurtboxes only.
    let hurt = bits(&[0, 0, 1, 0, 0]);
    assert!(draws(hurt) && shows(hurt, KIND_HURT));
    assert!([KIND_HIT, KIND_GRAB, KIND_MARK].iter().all(|k| !shows(hurt, *k)));
    // Hitboxes and the angles, no grab or hurt boxes.
    let lines = bits(&[1, 0, 0, 0, 1]);
    assert!(draws(lines) && shows(lines, KIND_HIT) && shows(lines, KIND_MARK));
    assert!(!shows(lines, KIND_GRAB) && !shows(lines, KIND_HURT));
    // The angles alone draw their lines without the discs; every tick has its own bit.
    assert!(draws(ANGLES) && shows(ANGLES, KIND_MARK) && !shows(ANGLES, KIND_HIT));
    let article = bits(&[0, 0, 0, 0, 0, 0, 1]);
    assert!(draws(article) && shows(article, hitbox::KIND_ARTICLE_HURT));
    assert!([KIND_HIT, KIND_HURT, KIND_GRAB, KIND_MARK, KIND_WIND, KIND_AREA].iter().all(|k| !shows(article, *k)));
    assert!(!shows(HURTBOXES, hitbox::KIND_ARTICLE_HURT));
    assert_eq!(bits(&[1; ITEMS]), HITBOXES | GRABS | HURTBOXES | COLOURS | ANGLES | WINDBOXES | ARTICLE_HURTBOXES);
    assert_eq!(ITEMS, 7, "Article hurtboxes appended as bit64");
}

#[test]
fn legend_old_hitbox_vis_migrates() {
    assert_eq!(migrate(&[1, 0]), Some([1, 1, 1, 0, 0, 0, 0]), "On: the three boxes, as before (windboxes among the hitboxes)");
    assert_eq!(migrate(&[0, 1]), Some([0; ITEMS]), "Off: nothing ticked");
    assert_eq!(migrate(&[1, 0, 0, 0, 1]), None, "a checklist stays as it is");
    // Through the settings file: an older file's On comes back as the three boxes, its Off as none, a five-item checklist (an
    // rc.14 build before Windboxes) with Windboxes unticked; the default is none.
    for (old, want) in [("[1, 0]", [1, 1, 1, 0, 0, 0, 0]), ("[0, 1]", [0; ITEMS]), ("[0, 1, 1, 0, 1]", [0, 1, 1, 0, 1, 0, 0]), ("[0, 0, 0, 0, 0, 1]", [0, 0, 0, 0, 0, 1, 0]), ("[0, 0, 0, 0, 0, 0, 1]", [0, 0, 0, 0, 0, 0, 1])] {
        let file = format!("{{\"menu\": {{\"{ROW}\": {old}}}, \"defaults_menu\": {{\"{ROW}\": {old}}}}}");
        let loaded = crate::model::settings::parse_bytes(file.as_bytes());
        assert!(!loaded.corrupt(), "{old}: {:?}", loaded.issues);
        assert_eq!((loaded.settings.menu.get(ROW), loaded.settings.defaults.get(ROW)), (Some(&want[..]), Some(&want[..])), "{old}");
        assert_eq!(crate::model::settings::parse_bytes(loaded.settings.to_json().as_bytes()).settings.menu.get(ROW), Some(&want[..]));
    }
    assert_eq!(crate::model::settings::Snapshot::factory().get(ROW), Some(&[0u8; ITEMS][..]));
}
/// Game memory for `wind_areas`: little-endian bytes from `BASE`; a read outside it is 0.
struct Mem(Vec<u8>);

const BASE: usize = 0x10_0000;

impl Mem {
    fn put(&mut self, at: usize, v: &[u8]) {
        self.0[at - BASE..at - BASE + v.len()].copy_from_slice(v);
    }
    fn get<const N: usize>(&self, at: usize) -> [u8; N] {
        at.checked_sub(BASE).and_then(|o| self.0.get(o..o + N)).map_or([0; N], |b| b.try_into().unwrap())
    }
}

impl AreaReader for Mem {
    fn area_usize(&mut self, at: usize) -> usize {
        u64::from_le_bytes(self.get(at)) as usize
    }
    fn area_u8(&mut self, at: usize) -> u8 {
        self.get::<1>(at)[0]
    }
    fn area_u32(&mut self, at: usize) -> u32 {
        u32::from_le_bytes(self.get(at))
    }
    fn area_f32(&mut self, at: usize) -> f32 {
        f32::from_le_bytes(self.get(at))
    }
}

/// An AreaModule at `BASE` with six records: not wind; a wind box (id 7, 30 degrees); a wind circle (id 9, 200 degrees); a wind box
/// switched off; a wind box with no manager entry (id 13); a wind record whose wind is not set. The manager's vector at `BASE+0x1000`.
fn areas_memory() -> Mem {
    use area::*;
    let mut m = Mem(vec![0; 0x2000]);
    let (table, records, entries) = (BASE + 0x100, BASE + 0x200, BASE + 0x1000);
    m.put(BASE + TABLE, &(table as u64).to_le_bytes());
    m.put(table + COUNT, &6u64.to_le_bytes());
    m.put(table + RECORDS, &(records as u64).to_le_bytes());
    let boxed = |m: &mut Mem, shape: usize, min: [f32; 2], max: [f32; 2]| {
        for (k, v) in [min[0], min[1]].iter().enumerate() {
            m.put(shape + BOX_MIN + 4 * k, &v.to_le_bytes());
        }
        for (k, v) in [max[0], max[1]].iter().enumerate() {
            m.put(shape + BOX_MAX + 4 * k, &v.to_le_bytes());
        }
    };
    // (kind, shape type, enabled, wind set, id)
    for (i, (kind, ty, on, set, id)) in [(5u8, 0u8, 1u8, 1u8, 5u32), (WIND, 0, 1, 1, 7), (WIND, 1, 1, 1, 9), (WIND, 0, 0, 1, 11), (WIND, 0, 1, 1, 13), (WIND, 0, 1, 0, 15)].into_iter().enumerate() {
        let (rec, shape) = (records + i * STRIDE, BASE + 0x800 + i * 0x100);
        m.put(rec + KIND, &[kind]);
        m.put(rec + SHAPE_TYPE, &[ty, on, 0, set]);
        m.put(rec + SHAPE, &(shape as u64).to_le_bytes());
        m.put(shape + ID, &id.to_le_bytes());
        boxed(&mut m, shape, [-10.0, 2.0], [10.0, 12.0]);
    }
    let circle = BASE + 0x800 + 2 * 0x100;
    m.put(circle + CENTRE, &5.0f32.to_le_bytes());
    m.put(circle + CENTRE + 4, &8.0f32.to_le_bytes());
    m.put(circle + RADIUS, &6.0f32.to_le_bytes());
    let begin = BASE + 0x1100;
    m.put(entries, &(begin as u64).to_le_bytes());
    m.put(entries + 8, &((begin + 3 * ENTRY) as u64).to_le_bytes());
    for (k, (id, angle)) in [(9u32, 200.0f32), (7, 30.0), (11, 90.0)].into_iter().enumerate() {
        m.put(begin + k * ENTRY, &id.to_le_bytes());
        m.put(begin + k * ENTRY + ENTRY_ANGLE, &angle.to_le_bytes());
    }
    m
}

#[test]
fn legend_wind_areas_read_the_area_table() {
    let mut m = areas_memory();
    let mut out = [WindArea::NONE; area::MAX_AREAS];
    let n = wind_areas(&mut m, BASE, BASE + 0x1000, &mut out);
    assert_eq!(&out[..n], &[
        WindArea { shape: AreaShape::Box { min: [-10.0, 2.0], max: [10.0, 12.0] }, angle: Some(30.0) },
        WindArea { shape: AreaShape::Circle { centre: [5.0, 8.0], radius: 6.0 }, angle: Some(200.0) },
        WindArea { shape: AreaShape::Box { min: [-10.0, 2.0], max: [10.0, 12.0] }, angle: None },
    ], "not wind, switched off and not set are left out; no manager entry: no angle");
    // No manager: the shapes without angles; a module, table or record pointer that is not one: nothing.
    let bare = wind_areas(&mut m, BASE, 0, &mut out);
    assert!(bare == 3 && out[..bare].iter().all(|a| a.angle.is_none()));
    assert_eq!(wind_areas(&mut m, 0, BASE + 0x1000, &mut out), 0);
    assert_eq!(wind_areas(&mut m, BASE + 4, BASE + 0x1000, &mut out), 0);
    let mut one = [WindArea::NONE; 1];
    assert_eq!(wind_areas(&mut areas_memory(), BASE, BASE + 0x1000, &mut one), 1, "at most what fits");
}

#[test]
fn legend_wind_area_orbs_are_a_band_or_a_disc_with_a_line() {
    let colour = [0, 10, 255, AREA_ALPHA];
    let boxed = WindArea { shape: AreaShape::Box { min: [-10.0, 2.0], max: [10.0, 12.0] }, angle: Some(0.0) };
    let [Some(band), Some(_), Some(line)] = area_orbs(&boxed, 1, 0.5, colour) else { panic!("a band, a shadow and a line") };
    assert_eq!((band.centre, band.tail, band.radius, band.color), ([-10.0, 7.0, 0.5], [10.0, 7.0, 0.5], 5.0, colour), "left edge to right edge, as tall as the box");
    assert_eq!((band.kind(), band.key & 0xff, (band.key >> 8) & 0xffff), (KIND_AREA, AREA_BOX, 1));
    assert!(close(line.centre, [0.0, 7.0, 0.5]) && close(line.tail, [10.0, 7.0, 0.5]) && line.color == crate::model::panes::linear(DIRECTION_COLOUR), "{line:?}");
    assert_eq!(line.key & 0xff, AREA_LINE);
    let up = area_orbs(&WindArea { angle: Some(90.0), ..boxed }, 1, 0.0, colour)[2].unwrap();
    assert!(close(up.tail, [0.0, 12.0, 0.0]), "to the top edge: {up:?}");
    let circle = WindArea { shape: AreaShape::Circle { centre: [5.0, 8.0], radius: 6.0 }, angle: Some(180.0) };
    let [Some(disc), Some(_), Some(line)] = area_orbs(&circle, 0, 0.0, colour) else { panic!("a disc, a shadow and a line") };
    assert_eq!((disc.centre, disc.tail, disc.radius, disc.key & 0xff), ([5.0, 8.0, 0.0], [5.0, 8.0, 0.0], 6.0, AREA_CIRCLE));
    assert!(close(line.tail, [-1.0, 8.0, 0.0]), "{line:?}");
    assert!(area_orbs(&WindArea { angle: None, ..circle }, 0, 0.0, colour)[2].is_none(), "no angle: no line");
    // 361 (Eden: the circles of Mach Tornado, Shadow Ball, Wario Waft): no one direction, a centre dot.
    let dot = area_orbs(&WindArea { angle: Some(361.0), ..circle }, 0, 0.0, colour)[2].unwrap();
    assert_eq!((dot.centre, dot.tail, dot.key & 0xff), ([5.0, 8.0, 0.0], [5.0, 8.0, 0.0], AREA_LINE));
    assert!(dot.radius > 6.0 * LINE && dot.radius < 6.0, "{dot:?}");
    // Negative angles are directions too (Eden: -150, -180): a line.
    let back = area_orbs(&WindArea { angle: Some(-180.0), ..circle }, 0, 0.0, colour)[2].unwrap();
    assert!(close(back.tail, [-1.0, 8.0, 0.0]), "{back:?}");
    // Shown by Windboxes alone: an area is not a hitbox; never paired as a capsule end.
    assert!(shows(WINDBOXES, KIND_AREA) && [HITBOXES, GRABS, HURTBOXES, COLOURS, ANGLES].iter().all(|b| !shows(*b, KIND_AREA)));
    assert!([AREA_BOX, AREA_CIRCLE, AREA_LINE, AREA_SHADOW].iter().all(|k| *k != 1));
}

/// A pane colour's red, green and blue as linear-light fractions (the layout takes vertex colours as linear light).
fn rgb(colour: [u8; 4]) -> [f32; 3] {
    [colour[0], colour[1], colour[2]].map(|c| f32::from(c) / 255.0)
}

fn luminance(rgb: [f32; 3]) -> f32 {
    0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
}

#[test]
fn legend_wind_area_fill_reads_over_light_and_dark_backgrounds() {
    // The layout blends in linear light, and the band's texture is opaque only `BAND_MIDDLE` in its middle (tools/layout-builder
    // `orb_texture`: 0.30), scaled by the pane's alpha; Eden at alpha 110 showed (226, 226, 242) over the grid's 240 white, too pale.
    const BAND_MIDDLE: f32 = 0.30;
    let colour = area_colour();
    let a = BAND_MIDDLE * f32::from(colour[3]) / 255.0;
    let linear = |v: u8| {
        let s = f32::from(v) / 255.0;
        if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    // What a background of sRGB grey `bg` becomes under the fill, as a ratio of its luminance.
    let ratio = |bg: u8| {
        let b = linear(bg);
        luminance(rgb(colour).map(|c| b * (1.0 - a) + c * a)) / luminance([b; 3])
    };
    assert!(ratio(240) <= 0.75, "over the Training grid's white the fill darkens it by a quarter or more: {}", ratio(240));
    assert!(ratio(30) >= 2.5, "over a dark stage the fill lifts its luminance 2.5 times or more: {}", ratio(30));
    // Burnt orange differs from the blue windbox discs without weakening either background-contrast bound.
    assert_eq!(colour[..3], crate::model::panes::linear([173, 77, 0, 255])[..3]);
}

#[test]
fn legend_wind_line_has_a_dark_shadow_under_it() {
    let colour = area_colour();
    let boxed = WindArea { shape: AreaShape::Box { min: [-10.0, 2.0], max: [10.0, 12.0] }, angle: Some(0.0) };
    let [Some(band), Some(shadow), Some(line)] = area_orbs(&boxed, 1, 0.5, colour) else { panic!("a band, a shadow and a line") };
    // The shadow is the line's own pill, wider, dark and opaque; it comes between the band and the line (a later pane draws over
    // an earlier one), so the silver line keeps a dark outline over a pale or a dark background.
    assert_eq!((shadow.centre, shadow.tail, shadow.kind(), shadow.key & 0xff, (shadow.key >> 8) & 0xffff), (line.centre, line.tail, KIND_AREA, AREA_SHADOW, 1));
    assert!(SHADOW_GROW > 1.0 && (shadow.radius - line.radius * SHADOW_GROW).abs() < 1e-5, "{shadow:?} {line:?}");
    assert!(luminance(rgb(shadow.color)) < 0.01 && shadow.color[3] == 255 && line.color[3] == 255, "{shadow:?}");
    assert_ne!(shadow.key, line.key);
    assert_eq!((band.color, line.color), (colour, crate::model::panes::linear(DIRECTION_COLOUR)), "area fill and silver line share their menu colours");
    // A centre dot (361) is outlined too; a wind with no angle has no line and no shadow.
    let circle = WindArea { shape: AreaShape::Circle { centre: [5.0, 8.0], radius: 6.0 }, angle: Some(361.0) };
    let [_, Some(ring), Some(dot)] = area_orbs(&circle, 0, 0.0, colour) else { panic!("a disc, a ring and a dot") };
    assert_eq!((ring.centre, ring.tail, ring.key & 0xff), (dot.centre, dot.tail, AREA_SHADOW));
    assert!(ring.radius > dot.radius && ring.radius < 6.0, "{ring:?} {dot:?}");
    let [_, none_s, none_l] = area_orbs(&WindArea { angle: None, ..circle }, 0, 0.0, colour);
    assert!(none_s.is_none() && none_l.is_none());
}

#[test]
fn legend_board_holds_every_orb_of_every_area() {
    // A band, a shadow and a line for each of `MAX_AREAS` areas, beside the spheres and the launch-angle marks.
    assert!(orbs::BOARD_ORBS >= hitbox::MAX_HELD + 2 * hitbox::SLOTS as usize + 3 * area::MAX_AREAS, "{}", orbs::BOARD_ORBS);
}
// ---- articles' and items' wind areas (rc.14 wind-articles) ----------------------------------------------------------------

/// A wind-area command tracks the object running it as an article while the visualizer is on with Windboxes ticked and the
/// practice visualizes it; a fighter keeps its areas in its own pass, and nothing is tracked otherwise.
#[test]
fn legend_a_wind_command_tracks_a_visualized_article_only() {
    assert!(wind_tracks(true, true, false, true), "Zelda's Triforce, Hero's Kaboom: an article running AREA_WIND_2ND*");
    for (enabled, windboxes, fighter, visualizes) in [(false, true, false, true), (true, false, false, true), (true, true, true, true), (true, true, false, false)] {
        assert!(!wind_tracks(enabled, windboxes, fighter, visualizes), "enabled {enabled} windboxes {windboxes} fighter {fighter} visualizes {visualizes}");
    }
}

/// An active item is tracked with a live attack slot (a thrown bomb) or with an enabled wind area.
#[test]
fn legend_an_item_with_a_wind_area_is_tracked() {
    assert!(tracks_item(true, 0) && tracks_item(false, 1) && tracks_item(true, area::MAX_AREAS));
    assert!(!tracks_item(false, 0));
}

/// A tracked article stays while it holds a sphere or has a wind area: Zelda's Triforce pulls before any of its hitboxes.
#[test]
fn legend_an_article_is_kept_while_it_has_a_wind_area() {
    assert!(keeps_article(true, 0) && keeps_article(false, 1) && keeps_article(true, 2));
    assert!(!keeps_article(false, 0));
}

/// The menu docs name the windbox moves from the game data that the public lists miss, say that articles' and items' wind
/// areas are drawn, and that stage hazards are not.
#[test]
fn legend_docs_name_the_game_data_windboxes() {
    let docs = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/menu/display-and-controls.md"));
    for want in ["Crownerang", "Counter", "down taunt", "Blunderbuss", "Triforce of Wisdom", "Kaboom", "Stage hazards", "fire hydrant"] {
        assert!(docs.contains(want), "docs/menu/display-and-controls.md names {want}");
    }
    assert!(!docs.contains("Items' wind, such as the Gust Bellows, is not drawn"), "items' wind areas are drawn now");
}

#[test]
fn menu_swatches_match_the_renderer_in_both_schemes() {
    let rendered = |tint| orbs::orb_of(&Sphere { tint, ..Sphere::NONE }, [0.0; 3]).color;
    for on in [false, true] {
        let rows: Vec<_> = swatches(127 ^ if on { 0 } else { COLOURS }).collect();
        let colour = |name| rows.iter().find(|(label, ..)| *label == name).unwrap().1.colours().next().unwrap();
        if on {
            assert_eq!(crate::model::panes::linear(colour("Red — Strong hitbox")), rendered(hitbox::legend_colour(KIND_HIT, None, 0, hitbox::TINTS[0], true)));
            assert_eq!(crate::model::panes::linear(colour("Pink — Weak hitbox")), rendered(hitbox::legend_colour(KIND_HIT, Some(Spot::Late), 0, hitbox::TINTS[0], true)));
        } else {
            assert_eq!(rows[0].1.colours().map(crate::model::panes::linear).collect::<Vec<_>>(), hitbox::TINTS.map(rendered));
        }
        for (name, kind, status, usual) in [
            (if on { "Purple — Grab box (grabs you)" } else { "Magenta — Grab box (grabs you)" }, KIND_GRAB, 0, hitbox::GRAB_TINT),
            (if on { "White — Hurtbox (can be hit)" } else { "Yellow — Hurtbox (can be hit)" }, KIND_HURT, 0, hitbox::HURTBOX_TINT),
            ("Blue — Windbox (pushes without flinching)", KIND_WIND, 0, WIND),
        ] {
            assert_eq!(crate::model::panes::linear(colour(name)), rendered(hitbox::legend_colour(kind, None, status, usual, on)));
        }
        for (name, status) in [("Sky blue — Hurtbox (cannot be hit)", INTANGIBLE), ("Teal — Hurtbox (takes no damage)", INVINCIBLE)] {
            assert_eq!(rows.iter().any(|r| r.0 == name), on);
            if on {
                for kind in [KIND_HURT, hitbox::KIND_ARTICLE_HURT] {
                    assert_eq!(crate::model::panes::linear(colour(name)), rendered(hitbox::legend_colour(kind, None, status, hitbox::HURTBOX_TINT, on)));
                }
            }
        }
        assert_eq!(crate::model::panes::linear(colour("Burnt orange — Wind area (pushes in a box or circle)")), area_colour());
        let hit = orbs::Orb { radius: 1.0, ..orbs::Orb::NONE };
        assert_eq!(crate::model::panes::linear(colour("Silver — Launch / wind direction")), marks(&hit, 45, 1.0)[0].unwrap().color);
    }
    for bits in 0..=127 {
        let rows: Vec<_> = swatches(bits).collect();
        assert!(rows.len() <= 9);
        assert!(rows.iter().map(|r| r.1.colours().count()).sum::<usize>() <= crate::model::panes::CONN_BOXES * 4);
        assert!(rows.iter().all(|(label, _)| label.encode_utf16().count() <= 72));
        assert_eq!(rows.iter().filter(|r| r.1 == Swatch::HitboxShades).count(), usize::from(bits & (HITBOXES | COLOURS) == HITBOXES));
        assert_eq!(rows.is_empty(), !draws(bits));
        assert_eq!(rows.iter().any(|r| r.0 == "Blue — Windbox (pushes without flinching)"), bits & WINDBOXES != 0 || bits & (HITBOXES | COLOURS) == HITBOXES | COLOURS);
        assert_eq!(rows.iter().any(|r| r.0 == "Burnt orange — Wind area (pushes in a box or circle)"), bits & WINDBOXES != 0);
        assert_eq!(rows.iter().any(|r| r.0.ends_with("— Hurtbox (can be hit)")), bits & (HURTBOXES | ARTICLE_HURTBOXES) != 0);
    }
    assert_eq!(swatches(ANGLES).collect::<Vec<_>>(), [("Silver — Launch / wind direction", Swatch::Colour(DIRECTION_COLOUR))]);
    assert_eq!(swatches(ARTICLE_HURTBOXES).next().unwrap().1.colours().next().unwrap(), orbs::tint_colour(hitbox::HURTBOX_TINT, 255));
}

/// Every single-colour family has its own key; the attack-ID group intentionally shows all eight shades.
#[test]
fn legend_families_have_distinct_colours_in_every_checklist() {
    for bits in 0..=127 {
        let colours: Vec<_> = swatches(bits).filter_map(|(_, swatch)| match swatch {
            Swatch::Colour(c) => Some(c),
            Swatch::HitboxShades => None,
        }).collect();
        for (i, c) in colours.iter().enumerate() {
            assert!(!colours[i + 1..].contains(c), "duplicate colour {c:?} for checklist {bits}");
        }
    }
    let area = WindArea { shape: AreaShape::Circle { centre: [0.0; 2], radius: 2.0 }, angle: Some(0.0) };
    let drawn = area_orbs(&area, 0, 0.0, area_colour());
    assert_eq!(drawn[0].unwrap().color, crate::model::panes::linear(orbs::tint_colour(WIND_AREA, AREA_ALPHA)));
    assert_eq!(drawn[2].unwrap().color, crate::model::panes::linear(DIRECTION_COLOUR));
    assert_ne!(area_colour(), crate::model::panes::linear(orbs::tint_colour(WIND, AREA_ALPHA)));
    assert_ne!(DIRECTION_COLOUR, crate::model::panes::linear(orbs::tint_colour(HURT[0], 255)));
}

#[test]
fn legend_columns_fit_and_preserve_the_checklist() {
    use crate::model::panes as p;
    let [(cx, cw), (mx, mw)] = swatch_columns(p::CONN_LEFT, p::CONN_RIGHT);
    assert!(cx >= p::CONN_LEFT + 128.0 && cx + cw < mx && mw > 0.0);
    assert_eq!(mx + mw, p::CONN_RIGHT);
    for bits in 0..=127 {
        let rows: Vec<_> = swatches(bits).collect();
        assert!(rows.len() <= p::ROWS, "the checklist's unused subline pool fits every colour name");
        for (name, _) in rows {
            let (colour, meaning) = name.split_once(" — ").expect("a colour name and meaning in every row");
            assert!(colour.len() <= 12 && !meaning.is_empty());
        }
    }
}
