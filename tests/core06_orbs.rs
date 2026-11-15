//! Own orbs (decision HB-OWN-ORBS, `model::hitbox_orbs`): the own-orb `Effects` sink inside the same
//! `HandleSet` the visualizer uses (never refused, concluded with the status like route B's effects), the board the layout
//! tick reads, and the projection's pill plan (decision HB-PILLS: a capsule is one pill, two half-disc caps and a band).

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::hitbox::{self, HandleSet, Sphere, FIGHTER_SETS, MAX_HELD};
use model::hitbox_orbs::{self as orbs, Board, Orb, OrbSink};

fn sphere(key: u32, x: f32, size: f32) -> Sphere {
    Sphere { key, joint: 0x1234, pos: [x, 0.0, 0.0], scale: hitbox::sphere_scale(size), tint: hitbox::HURTBOX_TINT }
}

fn own_orbs_are_never_refused_and_conclude_with_the_status() {
    let wants: Vec<Sphere> = (0..74).map(|i| sphere(hitbox::sphere_key(hitbox::KIND_HURT, i, 0), i as f32, 2.0)).collect();
    let mut set = HandleSet::new();
    let mut sink = OrbSink { status: 5 };
    let c = set.reconcile(&wants, true, &mut sink);
    assert_eq!((c.spawned, c.failed, c.dropped), (74, 0, 0), "Ridley + Steve's first pass: every sphere held");
    let c = set.reconcile(&wants, true, &mut sink);
    assert_eq!((c.kept, c.spawned, c.ended), (74, 0, 0), "unchanged wants are kept");
    // A status change ends every orb (route B's CONCLUDE_STATUS) and the same pass brings back what is still wanted.
    let mut sink = OrbSink { status: 6 };
    let c = set.reconcile(&wants, true, &mut sink);
    assert_eq!((c.ended, c.spawned, c.failed), (74, 74, 0));
    // A grab sphere is held once and ends with its status, never requested again.
    let grab = sphere(hitbox::sphere_key(hitbox::KIND_GRAB, 0, 0), 9.0, 3.0);
    assert!(set.push_grab(grab, None));
    let c = set.reconcile(&wants, true, &mut sink);
    assert_eq!(c.spawned, 1);
    assert_eq!(set.len(), 75);
    let mut sink = OrbSink { status: 7 };
    let c = set.reconcile(&wants, true, &mut sink);
    assert_eq!(set.len(), 74, "the grab orb ended with its status");
    assert_eq!(c.failed, 0);
    // Clearing asks nothing of the game and empties the set.
    let c = set.clear(&mut OrbSink { status: i32::MIN });
    assert_eq!(c.killed + c.ended, 74);
    assert!(set.is_empty());
    assert_ne!(orbs::orb_handle(0), 0, "an orb handle is never the refused handle");
}

fn a_grab_box_set_in_the_frame_it_connects_shows_once_not_through_the_pull() {
    let grab = sphere(hitbox::sphere_key(hitbox::KIND_GRAB, 0, 0), 9.0, 3.0);
    // Samus's tether (Eden gbx-samus-t1): the CATCH command runs in CATCH (55), the pass after it is already CATCH_PULL (56).
    let mut set = HandleSet::new();
    assert!(set.push_grab(grab, Some(55)));
    set.reconcile(&[], true, &mut OrbSink { status: 56 });
    assert_eq!(set.len(), 1, "drawn the pass after its command");
    let c = set.reconcile(&[], true, &mut OrbSink { status: 56 });
    assert_eq!((set.len(), c.ended), (0, 1), "not held through the pull");
    // A whiff: the pass is in the command's own status, and the box stays while the status does.
    assert!(set.push_grab(grab, Some(55)));
    set.reconcile(&[], true, &mut OrbSink { status: 55 });
    set.reconcile(&[], true, &mut OrbSink { status: 55 });
    assert_eq!(set.len(), 1);
}

fn an_orb_is_the_hitbox_size_in_the_spheres_tint() {
    let s = Sphere { key: 1, joint: 7, pos: [0.0; 3], scale: hitbox::sphere_scale(4.5), tint: [1.0, 0.0, 0.0] };
    let orb = orbs::orb_of(&s, [1.0, 2.0, 3.0]);
    assert_eq!(orb.centre, [1.0, 2.0, 3.0]);
    assert!((orb.radius - 4.5).abs() < 1e-5);
    assert_eq!(orb.color, [255, 0, 0, orbs::ORB_ALPHA]);
    let yellow = orbs::orb_of(&sphere(2, 0.0, 1.0), [0.0; 3]);
    assert_eq!(&yellow.color[..3], &[255, 255, 0]);
    // The orb keeps its sphere's kind (the cut order, review HB-2).
    for kind in [hitbox::KIND_HIT, hitbox::KIND_HURT, hitbox::KIND_GRAB] {
        assert_eq!(orbs::orb_of(&sphere(hitbox::sphere_key(kind, 3, 1), 0.0, 1.0), [0.0; 3]).kind(), kind);
    }
    // The tints of the colour table go through the layout's linear-light encoding.
    let orange = orbs::orb_of(&Sphere { tint: hitbox::tint(1), ..s }, [0.0; 3]);
    assert_eq!(orange.color, model::panes::linear([255, 102, 0, orbs::ORB_ALPHA]));
}

fn orb(x: f32) -> Orb {
    Orb { centre: [x, 0.0, 0.0], tail: [x, 0.0, 0.0], radius: 1.0, color: [1, 2, 3, 4], key: hitbox::sphere_key(hitbox::KIND_HURT, 0, 0) }
}

fn orb_of_kind(x: f32, kind: u32) -> Orb {
    Orb { key: hitbox::sphere_key(kind, 0, 0), ..orb(x) }
}

/// Decision HB-PILLS: the 512 orb panes are 340 half-disc caps and 172 bands, so 170 pills. A pill takes two caps and,
/// when it has a length, a band. A pill that does not get every pane it needs is cut whole, never a cap without its band,
/// and the panes are taken in the board's draw order, so hurtboxes are cut first (review HB-2).
fn the_pill_panes_are_cut_whole_hurtboxes_first() {
    use model::panes::{BANDS, CAPS, ORBS};
    assert_eq!((CAPS, BANDS, CAPS + BANDS), (340, 172, ORBS));
    let mut panes = orbs::PaneUse::default();
    assert_eq!(panes.take(true), Some((0, Some(0))));
    assert_eq!(panes.take(false), Some((2, None)), "a point takes two caps and no band");
    assert_eq!(panes.take(true), Some((4, Some(1))));
    let mut full = orbs::PaneUse::default();
    let drawn = (0..ORBS).filter(|_| full.take(true).is_some()).count();
    assert_eq!(drawn, CAPS / 2, "170 pills");
    assert_eq!((full.caps, full.bands), (CAPS, CAPS / 2), "the last pill's caps were the last caps; no cap went alone");
    // Bands run out before caps when every pill has one only if BANDS < CAPS / 2; with 172 they never do.
    let mut bands = orbs::PaneUse { caps: 0, bands: BANDS };
    assert_eq!(bands.take(true), None, "no band left: the pill is cut whole");
    assert_eq!(bands.take(false), Some((0, None)), "a point still fits");
    // The draw order puts every hitbox and grab box before any hurtbox.
    let mut board = Board::new();
    let kind = |i: usize| if i.is_multiple_of(8) { hitbox::KIND_HIT } else { hitbox::KIND_HURT };
    board.publish(0x100, (0..MAX_HELD).map(|i| orb_of_kind(i as f32, kind(i))));
    let order: Vec<u32> = board.in_draw_order().map(|o| o.kind()).collect();
    assert!(order[..16].iter().all(|k| *k == hitbox::KIND_HIT) && order[16..].iter().all(|k| *k == hitbox::KIND_HURT));
}

/// Decision HB-PILLS: a capsule's two end spheres (sphere 0 and sphere 1 of one shape) are one orb on the board, its
/// centre and its tail; a point's tail is its centre.
fn a_capsules_two_ends_are_one_orb() {
    let end = |i: usize| sphere(hitbox::sphere_key(hitbox::KIND_HIT, 4, i), 0.0, 2.0);
    let point = sphere(hitbox::sphere_key(hitbox::KIND_HURT, 0, 0), 0.0, 3.0);
    let mut board = Board::new();
    let placed = [orbs::orb_of(&end(0), [1.0, 0.0, 0.0]), orbs::orb_of(&end(1), [5.0, 0.0, 0.0]), orbs::orb_of(&point, [9.0, 0.0, 0.0])];
    assert_eq!(board.publish(0xa0, placed.into_iter()), 2, "two shapes");
    let live: Vec<Orb> = board.live().copied().collect();
    assert_eq!((live[0].centre, live[0].tail, live[0].kind()), ([1.0, 0.0, 0.0], [5.0, 0.0, 0.0], hitbox::KIND_HIT));
    assert_eq!((live[1].centre, live[1].tail), ([9.0, 0.0, 0.0], [9.0, 0.0, 0.0]), "a point's tail is its centre");
    // The held set keeps no order: the second end may come first, with other shapes between.
    let placed = [orbs::orb_of(&end(1), [5.0, 0.0, 0.0]), orbs::orb_of(&point, [9.0, 0.0, 0.0]), orbs::orb_of(&end(0), [1.0, 0.0, 0.0])];
    assert_eq!(board.publish(0xa0, placed.into_iter()), 2);
    let mut pairs: Vec<([f32; 3], [f32; 3])> = board.live().map(|o| (o.centre, o.tail)).collect();
    pairs.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]));
    assert_eq!(pairs, [([1.0, 0.0, 0.0], [5.0, 0.0, 0.0]), ([9.0, 0.0, 0.0], [9.0, 0.0, 0.0])]);
    // An end whose start could not be placed is drawn as a point; another shape's sphere 1 never joins a capsule.
    let other = sphere(hitbox::sphere_key(hitbox::KIND_HIT, 5, 1), 0.0, 2.0);
    let placed = [orbs::orb_of(&end(1), [5.0, 0.0, 0.0]), orbs::orb_of(&other, [7.0, 0.0, 0.0])];
    assert_eq!(board.publish(0xa0, placed.into_iter()), 2);
    let tails: Vec<[f32; 3]> = board.live().map(|o| o.tail).collect();
    assert_eq!(tails, [[5.0, 0.0, 0.0], [7.0, 0.0, 0.0]]);
}

/// Review HB-1: what the layout tick writes into the orb panes. An inactive tick (the waiting room's practice ended while
/// its layout stays) hides every pane the last tick showed and the container; once hidden, nothing more is written.
fn an_inactive_tick_hides_the_orbs_it_showed() {
    use orbs::{pane_tick, PaneTick};
    // Active: 37 orbs drawn after 0 shown: nothing to hide, the container shows.
    assert_eq!(pane_tick(37, 0), Some(PaneTick { hide: (37, 37), container: true, shown: 37 }));
    // Fewer orbs: the panes past them are hidden.
    assert_eq!(pane_tick(20, 37), Some(PaneTick { hide: (20, 37), container: true, shown: 20 }));
    // The practice ends: nothing drawn, every pane shown is hidden and the container with them.
    assert_eq!(pane_tick(0, 20), Some(PaneTick { hide: (0, 20), container: false, shown: 0 }));
    // The next inactive ticks write nothing.
    assert_eq!(pane_tick(0, 0), None);
    // After a new registration the runtime assumes every pane may show (`shown = ORBS`), so all of them are hidden.
    assert_eq!(pane_tick(0, model::panes::ORBS), Some(PaneTick { hide: (0, model::panes::ORBS), container: false, shown: 0 }));
}

fn the_board_keeps_each_fighters_last_pass_and_ages_out_the_gone() {
    let mut board = Board::new();
    assert_eq!(board.publish(0xa0, (0..55).map(|i| orb(i as f32))), 55);
    assert_eq!(board.publish(0xb0, (0..19).map(|i| orb(i as f32))), 19);
    assert_eq!(board.live().count(), 74);
    // Republishing replaces, never appends.
    board.publish(0xa0, (0..10).map(|i| orb(i as f32)));
    assert_eq!(board.live().count(), 29);
    // At most BOARD_ORBS per fighter (its MAX_HELD spheres and the launch-angle marks, rc.14).
    assert_eq!(board.publish(0xc0, (0..orbs::BOARD_ORBS + 5).map(|i| orb(i as f32))), orbs::BOARD_ORBS);
    const { assert!(orbs::BOARD_ORBS > MAX_HELD) };
    board.clear(0xc0);
    assert_eq!(board.live().count(), 29);
    // A fighter that stops publishing ages out while others go on, counted in publishes, not in time: a paused game
    // publishes nothing, so a pause of any length keeps everything (review HB-4: the old check compared two reads).
    // 0xb0 last published one publish after 0xa0's 55 (then 0xa0's 10, then 0xc0's): it has seen 2 publishes since.
    for _ in 0..orbs::STALE_PUBLISHES - 3 {
        board.publish(0xa0, (0..10).map(|i| orb(i as f32)));
    }
    assert_eq!(board.live().count(), 29, "0xb0 is one publish from ageing out");
    let paused: Vec<Orb> = board.live().copied().collect();
    board.clear(0xdead); // an unknown fighter changes nothing
    assert_eq!(board.live().copied().collect::<Vec<_>>(), paused);
    board.publish(0xa0, (0..10).map(|i| orb(i as f32)));
    assert_eq!(board.live().count(), 10, "0xb0 aged out on the publish that made it stale");
    // Every slot can be taken; a new fighter replaces the stalest, the first one published.
    board.clear_all();
    for k in 1..=FIGHTER_SETS {
        board.publish(k << 8, std::iter::once(orb(k as f32)));
    }
    let xs = |b: &Board| { let mut v: Vec<i32> = b.live().map(|o| o.centre[0] as i32).collect(); v.sort(); v };
    assert_eq!(xs(&board), (1..=FIGHTER_SETS as i32).collect::<Vec<_>>());
    board.publish((FIGHTER_SETS + 1) << 8, std::iter::once(orb((FIGHTER_SETS + 1) as f32)));
    assert_eq!(xs(&board), (2..=FIGHTER_SETS as i32 + 1).collect::<Vec<_>>(), "fighter 1 (the stalest) was replaced");
    // A fighter that published nothing (an empty slot) is taken before any fighter with orbs.
    board.publish(2 << 8, std::iter::empty());
    board.publish((FIGHTER_SETS + 2) << 8, std::iter::once(orb((FIGHTER_SETS + 2) as f32)));
    assert_eq!(xs(&board), (3..=FIGHTER_SETS as i32 + 2).collect::<Vec<_>>(), "the empty slot went first");
    board.clear_all();
    assert_eq!(board.live().count(), 0);
    assert_eq!(board.publish(0, std::iter::once(orb(0.0))), 0, "no fighter, no orbs");
}

/// Samus's charge shot: one 1.9 hitbox in slot 0 on `top` while `on`.
struct Shot(bool);
impl hitbox::AttackReader for Shot {
    fn active(&mut self, slot: i32) -> bool {
        self.0 && slot == 0
    }
    fn data(&mut self, _slot: i32) -> usize {
        0x1000
    }
    fn read_u64(&mut self, _at: usize) -> u64 {
        0x3_1ed9_1fca
    }
    fn read_f32(&mut self, at: usize) -> f32 {
        if at == 0x1024 { 1.9 } else { 0.0 }
    }
}

/// rc.13 (operator 2026-11-14: "projectiles now for rc13"): an article's hitboxes are own orbs too. The game's effect
/// sphere on a projectile lived its whole flight but never showed (Eden wa-hbprobe-ae602057-1). Its `ATTACK` tracks the
/// article by battle-object id; each pass reads its live attack slots, places them on its bones and publishes them under
/// its accessor; it goes when its attack ends or it does.
fn an_articles_hitboxes_are_own_orbs_while_its_attack_lasts() {
    let (id, key) = (0x10a8_0001u32, 0xa000usize);
    let mut board = Board::new();
    board.publish(0xf000, std::iter::once(orb(1.0))); // Player 1's own pass
    let mut articles = orbs::Articles::new();
    let mut set = HandleSet::new();
    articles.track(id, key, &mut board);
    articles.track(id, key, &mut board);
    assert_eq!(articles.tracked().iter().filter(|t| t.0 != 0).count(), 1, "one article, however many ATTACKs");
    let on_bone = |s: &Sphere| (s.joint == 0x3_1ed9_1fca).then_some([40.0 + s.pos[0], 12.0, 0.0]);
    assert!(orbs::article_pass(&mut board, &mut set, key, 7, &mut Shot(true), on_bone, orbs::Extras::NONE));
    let hit: Vec<Orb> = board.in_draw_order().copied().collect();
    assert_eq!(hit.len(), 2, "the shot's orb (a hitbox, drawn first), then Player 1's hurtbox");
    assert_eq!(hit[0].centre, [40.0, 12.0, 0.0]);
    assert!((hit[0].radius - 1.9).abs() < 1e-5);
    assert_eq!((hit[0].kind(), &hit[0].color[..3]), (hitbox::KIND_HIT, &model::panes::linear([255, 0, 0, 255])[..3]));
    // Its attack ended: its orbs go and the pass says to forget it; Player 1's stay.
    assert!(!orbs::article_pass(&mut board, &mut set, key, 7, &mut Shot(false), on_bone, orbs::Extras::NONE));
    assert_eq!(board.live().count(), 1);
    // A thrown item (a bomb, a turnip) sets its hitbox without `ATTACK`: the pass tracks an active item while a slot is live.
    assert_eq!((hitbox::live_slot(&mut Shot(true)), hitbox::live_slot(&mut Shot(false))), (Some(0), None));
    articles.track(id, key, &mut board);
    orbs::article_pass(&mut board, &mut set, key, 7, &mut Shot(true), on_bone, orbs::Extras::NONE);
    articles.forget(id, &mut board);
    assert_eq!((board.live().count(), articles.tracked().iter().filter(|t| t.0 != 0).count()), (1, 0), "gone article");
    // Fighters and articles share the board's slots; a fifth article replaces the oldest, whose orbs go.
    assert_eq!(orbs::ORB_FIGHTERS + orbs::ARTICLES, FIGHTER_SETS);
    for k in 1..=orbs::ARTICLES {
        articles.track(k as u32, k << 4, &mut board);
        orbs::article_pass(&mut board, &mut HandleSet::new(), k << 4, 7, &mut Shot(true), on_bone, orbs::Extras::NONE);
    }
    assert_eq!(board.live().count(), 1 + orbs::ARTICLES);
    articles.track(99, 99 << 4, &mut board);
    assert_eq!(board.live().count(), orbs::ARTICLES, "the oldest article's orb went");
    let mut ids = articles.tracked().map(|t| t.0);
    ids.sort();
    assert_eq!(ids, [2, 3, 4, 99], "article 1, the oldest, was replaced");
    // Hitbox Visualization Off: every article goes.
    articles.forget_all(&mut board);
    assert_eq!((board.live().count(), articles.tracked()), (1, [(0, 0); orbs::ARTICLES]));
}

/// rc.13 (the primary 2026-11-14 06:10Z; Eden wa-grab-bfc1a3b9-6): Olimar's grab is a `CATCH` his Pikmin, an article, runs,
/// and no grab box was drawn. The command's spheres wait in the article's own set with the status it ran in and are drawn
/// as the fighters' are (deluxe-2's rule): magenta on the article's bone while it stays in that status, once when the
/// status already moved on, and the article stays tracked while it has one.
fn an_articles_grab_box_is_an_own_orb_while_its_status_lasts() {
    let (id, key) = (0x102e_0001u32, 0xb000usize);
    let mut board = Board::new();
    let mut articles = orbs::Articles::new();
    let tint = hitbox::GRAB_TINT;
    let pikmin = Sphere { key: hitbox::sphere_key(hitbox::KIND_GRAB, 0, 0), joint: 0x3_1ed9_1fca, pos: [0.0; 3], scale: hitbox::sphere_scale(3.0), tint };
    let on_bone = |s: &Sphere| (s.joint == 0x3_1ed9_1fca).then_some([25.0, 4.0, 0.0]);
    let set = articles.track(id, key, &mut board);
    assert!(set.push_grab(pikmin, Some(40)));
    for pass in 0..3 {
        assert!(orbs::article_pass(&mut board, set, key, 40, &mut Shot(false), on_bone, orbs::Extras::NONE), "pass {pass}: no hitbox, still tracked");
        let grab: Vec<Orb> = board.live().copied().collect();
        assert_eq!(grab.len(), 1, "pass {pass}: the Pikmin's grab box");
        assert_eq!((grab[0].kind(), grab[0].centre), (hitbox::KIND_GRAB, [25.0, 4.0, 0.0]));
        assert_eq!(&grab[0].color[..3], &model::panes::linear([255, 0, 255, 255])[..3], "magenta, as the fighters'");
        assert!((grab[0].radius - 3.0).abs() < 1e-5);
    }
    assert!(!orbs::article_pass(&mut board, set, key, 41, &mut Shot(false), on_bone, orbs::Extras::NONE), "its status ended: forgotten");
    assert_eq!(board.live().count(), 0);
    // Set the frame the grab connected (the pass already sees the next status): drawn that pass, gone at the next.
    assert!(set.push_grab(pikmin, Some(40)));
    assert!(orbs::article_pass(&mut board, set, key, 41, &mut Shot(false), on_bone, orbs::Extras::NONE));
    assert_eq!(board.live().count(), 1);
    assert!(!orbs::article_pass(&mut board, set, key, 41, &mut Shot(false), on_bone, orbs::Extras::NONE));
    // A grab box beside a hitbox: both drawn, the hitbox's colour unchanged.
    assert!(set.push_grab(pikmin, Some(40)));
    assert!(orbs::article_pass(&mut board, set, key, 40, &mut Shot(true), on_bone, orbs::Extras::NONE));
    let mut kinds: Vec<u32> = board.live().map(|o| o.kind()).collect();
    kinds.sort();
    assert_eq!(kinds, [hitbox::KIND_HIT, hitbox::KIND_GRAB]);
    // A forgotten article drops what its set held.
    assert!(articles.track(id, key, &mut board).push_grab(pikmin, Some(40)));
    articles.forget(id, &mut board);
    assert!(articles.track(id, key, &mut board).is_empty(), "a forgotten article's waiting grab box went with it");
}

/// rc.14 wind-articles: an article's wind area (Zelda's Triforce of Wisdom pulls, Hero's Kaboom pushes; their `AREA_WIND_2ND*` runs
/// in the article's own script) is drawn as a fighter's is, after its spheres, and keeps the article tracked while it lasts, even
/// with no hitbox; with its area gone and nothing held it is forgotten.
fn an_articles_wind_area_is_drawn_and_keeps_it_tracked() {
    use model::legend::{self, AreaShape, WindArea};
    let (id, key) = (0x1047_0001u32, 0xc000usize);
    let mut board = Board::new();
    let mut articles = orbs::Articles::new();
    let on_bone = |s: &Sphere| (s.joint == 0x3_1ed9_1fca).then_some([40.0 + s.pos[0], 12.0, 0.0]);
    let pull = [WindArea { shape: AreaShape::Circle { centre: [10.0, 20.0], radius: 30.0 }, angle: Some(-90.0) }];
    let set = articles.track(id, key, &mut board);
    for pass in 0..3 {
        assert!(orbs::article_pass(&mut board, set, key, 9, &mut Shot(false), on_bone, orbs::Extras { lr: None, areas: &pull }), "pass {pass}: no hitbox, a wind area: kept");
        let drawn: Vec<Orb> = board.live().copied().collect();
        assert_eq!(drawn.len(), 3, "pass {pass}: the disc, the line's shadow and the line");
        assert!(drawn.iter().all(|o| o.kind() == legend::KIND_AREA));
        assert_eq!((drawn[0].centre, drawn[0].radius), ([10.0, 20.0, 0.0], 30.0));
    }
    // Its hitbox beside its area: the sphere first, then the area's three orbs.
    assert!(orbs::article_pass(&mut board, set, key, 9, &mut Shot(true), on_bone, orbs::Extras { lr: None, areas: &pull }));
    let kinds: Vec<u32> = board.live().map(|o| o.kind()).collect();
    assert_eq!(kinds, [hitbox::KIND_HIT, legend::KIND_AREA, legend::KIND_AREA, legend::KIND_AREA]);
    // The area ended and nothing is held: forgotten, its orbs gone.
    assert!(!orbs::article_pass(&mut board, set, key, 9, &mut Shot(false), on_bone, orbs::Extras::NONE));
    assert_eq!(board.live().count(), 0);
}

fn screen_half_mirrors_the_games_size_words() {
    assert_eq!(orbs::screen_half(0.0, 960.0), 960.0);
    assert_eq!(orbs::screen_half(1920.0, 960.0), 960.0);
    assert_eq!(orbs::screen_half(1280.9, 960.0), 640.0, "truncated, then halved");
    assert_eq!(orbs::screen_half(-5.0, 960.0), 0.0, "fcvtzu saturates a negative word to 0");
}

fn projected_orbs_map_onto_the_layout_canvas() {
    let half = [960.0, 540.0];
    assert_eq!(orbs::to_canvas([960.0, 540.0], half), [0.0, 0.0]);
    assert_eq!(orbs::to_canvas([0.0, 0.0], half), [-960.0, 540.0], "top left, y up");
    // A 720p screen maps to the same canvas.
    assert_eq!(orbs::to_canvas([1280.0, 720.0], [640.0, 360.0]), [960.0, -540.0]);
    let color = [9, 8, 7, 6];
    // A point: its two edges' longer projected radius, doubled; no length, the two caps back to back make the disc.
    let r20 = |c: [f32; 2]| [[c[0] + 20.0, c[1]], [c[0], c[1] - 10.0]];
    let dot = orbs::plan_pill([[960.0, 540.0]; 2], [r20([960.0, 540.0]); 2], half, color).unwrap();
    assert_eq!((dot.size, dot.length, dot.band, dot.color), (40.0, 0.0, [0.0, 0.0], color));
    assert_eq!(dot.caps, [[-10.0, 0.0], [10.0, 0.0]], "each cap a radius wide, its flat side on its end");
    let small = orbs::plan_pill([[640.0, 360.0]; 2], [[[650.0, 360.0], [640.0, 365.0]]; 2], [640.0, 360.0], color).unwrap();
    assert_eq!(small.size, 30.0, "720p pixels are 1.5 canvas units");
    // A capsule along +x: the band spans the two ends, the caps sit outside them, no rotation.
    let x = orbs::plan_pill([[960.0, 540.0], [1060.0, 540.0]], [r20([960.0, 540.0]), r20([1060.0, 540.0])], half, color).unwrap();
    assert_eq!((x.size, x.length, x.rot, x.band), (40.0, 100.0, 0.0, [50.0, 0.0]));
    assert_eq!(x.caps, [[-10.0, 0.0], [110.0, 0.0]]);
    // Up the screen is up the canvas (y up), a quarter turn counter-clockwise.
    let up = orbs::plan_pill([[960.0, 540.0], [960.0, 440.0]], [r20([960.0, 540.0]), r20([960.0, 440.0])], half, color).unwrap();
    assert!((up.rot - 90.0).abs() < 1e-4, "{}", up.rot);
    assert!((up.caps[0][0]).abs() < 1e-4 && (up.caps[0][1] + 10.0).abs() < 1e-4 && (up.caps[1][1] - 110.0).abs() < 1e-4, "{:?}", up.caps);
    // One size for the whole pill: the mean of the two ends' projected radii (20 and 30), doubled.
    let r30 = [[1090.0, 540.0], [1060.0, 540.0]];
    assert_eq!(orbs::plan_pill([[960.0, 540.0], [1060.0, 540.0]], [r20([960.0, 540.0]), r30], half, color).unwrap().size, 50.0);
    assert!(orbs::plan_pill([[f32::NAN, 0.0]; 2], [[[0.0; 2]; 2]; 2], half, color).is_none());
    assert!(orbs::plan_pill([[960.0, 540.0]; 2], [[[960.2, 540.0], [960.0, 540.2]]; 2], half, color).is_none(), "under a unit");
    assert!(orbs::plan_pill([[5000.0, 540.0]; 2], [r20([5000.0, 540.0]); 2], half, color).is_none(), "off the canvas");
    assert!(orbs::plan_pill([[1925.0, 540.0]; 2], [[[1955.0, 540.0], [1925.0, 540.0]]; 2], half, color).is_some(), "partly on the canvas");
    let across = orbs::plan_pill([[-500.0, 540.0], [2500.0, 540.0]], [r20([-500.0, 540.0]), r20([2500.0, 540.0])], half, color);
    assert!(across.is_some(), "both ends off the canvas, the band across it");
    assert_eq!(orbs::plan_pill([[960.0, 540.0]; 2], [[[9000.0, 540.0], [960.0, 540.0]]; 2], half, color).unwrap().size, orbs::MAX_PANE);
    assert!(orbs::plan_pill([[960.0, 540.0]; 2], [[[970.0, 540.0]; 2]; 2], [0.0, 540.0], color).is_none(), "no screen size");
    assert_eq!(orbs::edge_points([1.0, 2.0, 3.0], 0.5), [[1.5, 2.0, 3.0], [1.0, 2.5, 3.0]]);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("own_orbs_are_never_refused_and_conclude_with_the_status", own_orbs_are_never_refused_and_conclude_with_the_status as fn()),
        ("a_grab_box_set_in_the_frame_it_connects_shows_once_not_through_the_pull", a_grab_box_set_in_the_frame_it_connects_shows_once_not_through_the_pull as fn()),
        ("an_orb_is_the_hitbox_size_in_the_spheres_tint", an_orb_is_the_hitbox_size_in_the_spheres_tint as fn()),
        ("the_pill_panes_are_cut_whole_hurtboxes_first", the_pill_panes_are_cut_whole_hurtboxes_first as fn()),
        ("a_capsules_two_ends_are_one_orb", a_capsules_two_ends_are_one_orb as fn()),
        ("an_inactive_tick_hides_the_orbs_it_showed", an_inactive_tick_hides_the_orbs_it_showed as fn()),
        ("the_board_keeps_each_fighters_last_pass_and_ages_out_the_gone", the_board_keeps_each_fighters_last_pass_and_ages_out_the_gone as fn()),
        ("an_articles_hitboxes_are_own_orbs_while_its_attack_lasts", an_articles_hitboxes_are_own_orbs_while_its_attack_lasts as fn()),
        ("an_articles_grab_box_is_an_own_orb_while_its_status_lasts", an_articles_grab_box_is_an_own_orb_while_its_status_lasts as fn()),
        ("an_articles_wind_area_is_drawn_and_keeps_it_tracked", an_articles_wind_area_is_drawn_and_keeps_it_tracked as fn()),
        ("screen_half_mirrors_the_games_size_words", screen_half_mirrors_the_games_size_words as fn()),
        ("projected_orbs_map_onto_the_layout_canvas", projected_orbs_map_onto_the_layout_canvas as fn()),
    ]);
}
