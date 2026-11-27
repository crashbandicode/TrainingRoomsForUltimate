//! Own orbs (decision HB-OWN-ORBS, `model::hitbox_orbs`): the own-orb `Effects` sink inside the same
//! `HandleSet` the visualizer uses (never refused, concluded with the status like route B's effects), the board the layout
//! tick reads, and the projection's pill plan (decision HB-PILLS: a capsule is one pill, two half-disc caps and a band).

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
use crate::model;

use model::hitbox::{self, HandleSet, Sphere, FIGHTER_SETS, MAX_HELD};
use model::hitbox_orbs::{self as orbs, Board, Orb, OrbSink};

fn sphere(key: u32, x: f32, size: f32) -> Sphere {
    Sphere { key, joint: 0x1234, pos: [x, 0.0, 0.0], scale: hitbox::sphere_scale(size), tint: hitbox::HURTBOX_TINT }
}

#[test]
fn a_start_outside_the_picture_gets_a_pointer_at_the_border() {
    let [half_x, half_y] = orbs::CANVAS_HALF;
    assert_eq!(orbs::edge_pointer([0.0, 0.0]), None);
    assert_eq!(orbs::edge_pointer([half_x, half_y]), None);
    assert_eq!(orbs::edge_pointer([f32::NAN, 0.0]), None);
    assert_eq!(orbs::edge_pointer([0.0, f32::INFINITY]), None);
    let x = -(half_x - 40.0);
    assert_eq!(
        orbs::edge_pointer([-half_x - 100.0, 0.0]),
        Some([[x, 0.0], [x + 14.0, -10.0], [x + 14.0, 10.0], [x + 28.0, -20.0], [x + 28.0, 20.0]])
    );
    let below = orbs::edge_pointer([0.0, -half_y - 100.0]).unwrap();
    assert_eq!(below[0], [0.0, -(half_y - 90.0)]);
    assert!(below[1..].iter().all(|p| p[1] > below[0][1]));
    let corner = orbs::edge_pointer([half_x + 100.0, half_y + 100.0]).unwrap();
    assert_eq!(corner[0], [half_x - 40.0, half_y - 90.0]);
    assert!(corner.iter().all(|p| p[0].abs() <= half_x && p[1].abs() <= half_y));
    let extreme = orbs::edge_pointer([f32::MAX, f32::MAX]).unwrap();
    assert!(extreme.iter().flatten().all(|value| value.is_finite()));
    assert_ne!(extreme[0], extreme[1]);
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
    // Every sphere colour takes the same sRGB-to-linear path, including status and strength colours.
    let cases = [
        (hitbox::KIND_HIT, hitbox::STRONG_TINT, [255, 0, 0, orbs::ORB_ALPHA]),
        (hitbox::KIND_HIT, hitbox::WEAK_TINT, [255, 143, 199, orbs::ORB_ALPHA]),
        (hitbox::KIND_GRAB, hitbox::GRAB_TINT, [153, 51, 255, orbs::ORB_ALPHA]),
        (hitbox::KIND_HURT, hitbox::HURTBOX_TINT, [153, 153, 153, orbs::ORB_ALPHA]),
        (hitbox::KIND_ARTICLE_HURT, model::legend::hurt_tint(model::legend::INVINCIBLE), [51, 255, 51, orbs::ORB_ALPHA]),
        (hitbox::KIND_HURT, model::legend::hurt_tint(model::legend::INTANGIBLE), [255, 255, 0, orbs::ORB_ALPHA]),
        (model::legend::KIND_WIND, model::legend::WIND, [115, 204, 255, orbs::ORB_ALPHA]),
        (model::legend::KIND_AREA, model::legend::WIND_AREA, [75, 85, 0, orbs::ORB_ALPHA]),
        (model::legend::KIND_MARK, [1.0; 3], [255; 4]),
    ];
    for (kind, tint, srgb) in cases {
        let key = hitbox::sphere_key(kind, 3, 1);
        let orb = orbs::orb_of(&Sphere { key, tint, ..s }, [0.0; 3]);
        assert_eq!(orb.color, model::panes::linear(srgb), "kind {kind}, tint {tint:?}");
        assert_eq!(orb.kind(), kind, "the orb preserves its sphere kind for draw order");
    }
}

fn orb(x: f32) -> Orb {
    Orb { centre: [x, 0.0, 0.0], tail: [x, 0.0, 0.0], radius: 1.0, color: [1, 2, 3, 4], key: hitbox::sphere_key(hitbox::KIND_HURT, 0, 0) }
}

fn orb_of_kind(x: f32, kind: u32) -> Orb {
    Orb { key: hitbox::sphere_key(kind, 0, 0), ..orb(x) }
}

fn hidden_fighter_and_owned_objects_leave_and_rejoin_the_draw_order() {
    use model::hitbox_orbs::DrawHide;
    use std::cell::Cell;

    let mut board = Board::new();
    let fighter = [hitbox::KIND_HIT, hitbox::KIND_HURT, hitbox::KIND_GRAB, model::legend::KIND_MARK];
    let article = [hitbox::KIND_HIT, hitbox::KIND_ARTICLE_HURT, model::legend::KIND_WIND, model::legend::KIND_AREA];
    let orbs = |x, kinds: [u32; 4]| kinds.into_iter().map(move |kind| orb_of_kind(x, kind));
    let queries = Cell::new(0);
    let report = |hide: &DrawHide, id| hide.owner(|| {
        queries.set(queries.get() + 1);
        id
    });
    let publish = |board: &mut Board, hide: &DrawHide| {
        board.publish_article(30, 0x1000_0001, report(hide, Some(10)), orbs(3.0, article));
        board.publish_article(40, 0x1000_0002, report(hide, Some(0)), orbs(4.0, article));
        board.publish_article(50, 0x4000_0003, report(hide, None), orbs(5.0, article));
        board.finish_articles(hide.generation());
    };
    let scans = Cell::new(0);
    let draw = |b: &Board, hide: &DrawHide| {
        let mut counts = [0; 5];
        let slots = b.draw_slots_with(hide, |words, token| {
            scans.set(scans.get() + 1);
            words.contains(&token)
        });
        for orb in b.draw_slots_in_order(slots) {
            counts[orb.centre[0] as usize - 1] += 1;
        }
        counts
    };
    board.publish(10, orbs(1.0, fighter));
    board.publish(20, orbs(2.0, fighter));
    publish(&mut board, &DrawHide::VISIBLE);
    assert_eq!(draw(&board, &DrawHide::VISIBLE), [4; 5]);
    assert_eq!(queries.get(), 0, "all visible: zero native owner probes");
    assert_eq!(scans.get(), 0, "all visible: zero hidden ID scans across all four draw passes");
    let pending = DrawHide::sampled(None);
    assert_eq!(draw(&board, &pending), [0; 5], "a torn publication cannot draw stale CPU geometry");
    assert_eq!(report(&pending, Some(0)), None);
    assert_eq!((queries.get(), scans.get()), (0, 0));

    let mut words = [0; orbs::HIDDEN_WORDS];
    words[1] = 20;
    words[model::cpu::role::FighterKey::COUNT + 1] = 1;
    let hide = DrawHide::sampled(Some((2, words)));
    assert_eq!(draw(&board, &hide), [4, 0, 0, 0, 0], "an overlapping UI tick refuses unclassified article panes");
    publish(&mut board, &hide);
    assert_eq!(queries.get(), 3, "first hidden pass samples every previously live article");
    assert_eq!(draw(&board, &hide), [4, 0, 4, 0, 4], "first classified draw");
    assert_eq!(scans.get(), 6, "four scans for the whole classified draw, plus two for the earlier overlap");
    board.publish_article(40, 0x1000_0002, report(&hide, Some(10)), orbs(4.0, article));
    assert_eq!(draw(&board, &hide), [4, 0, 4, 4, 4], "the same object changes owner");
    board.publish_article(40, 0x1000_0002, report(&hide, Some(0)), orbs(4.0, article));
    assert_eq!(draw(&board, &hide), [4, 0, 4, 0, 4]);
    assert_eq!(draw(&board, &DrawHide::VISIBLE), [4; 5], "both live contributions return on Stop");
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
    fn read_u64(&mut self, at: usize) -> u64 {
        match at {
            0x1048 => 0x3_1ed9_1fca,
            0x1028 => (90u64 << 32) | 45,
            0x1030 => 40u64 << 32,
            0x1070 => 0,
            _ => panic!("unexpected article attack word {at:#x}"),
        }
    }
    fn read_f32(&mut self, at: usize) -> f32 {
        match at {
            0x1020 => 10.0,
            0x1024 => 1.9,
            0x1000 | 0x1004 | 0x1008 | 0x1010 | 0x1014 | 0x1018 => 0.0,
            _ => panic!("unexpected article attack scalar {at:#x}"),
        }
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
    articles.track(id, key, Some(&mut board), true).unwrap();
    articles.track(id, key, Some(&mut board), true).unwrap();
    assert_eq!(articles.tracked().iter().filter(|t| t.0 != 0).count(), 1, "one article, however many ATTACKs");
    let on_bone = |s: &Sphere| (s.joint == 0x3_1ed9_1fca).then_some([40.0 + s.pos[0], 12.0, 0.0]);
    assert!(orbs::article_pass(&mut board, &mut set, (id, key), 7, &mut Shot(true), on_bone, orbs::Extras::NONE));
    let hit: Vec<Orb> = board.in_draw_order().copied().collect();
    assert_eq!(hit.len(), 2, "the shot's orb (a hitbox, drawn first), then Player 1's hurtbox");
    assert_eq!(hit[0].centre, [40.0, 12.0, 0.0]);
    assert!((hit[0].radius - 1.9).abs() < 1e-5);
    assert_eq!((hit[0].kind(), &hit[0].color[..3]), (hitbox::KIND_HIT, &model::panes::linear([255, 0, 0, 255])[..3]));
    // Its attack ended: its orbs go and the pass says to forget it; Player 1's stay.
    assert!(!orbs::article_pass(&mut board, &mut set, (id, key), 7, &mut Shot(false), on_bone, orbs::Extras::NONE));
    assert_eq!(board.live().count(), 1);
    // A thrown item (a bomb, a turnip) sets its hitbox without `ATTACK`: the pass tracks an active item while a slot is live.
    assert_eq!((hitbox::live_slot(&mut Shot(true)), hitbox::live_slot(&mut Shot(false))), (Some(0), None));
    articles.track(id, key, Some(&mut board), true).unwrap();
    orbs::article_pass(&mut board, &mut set, (id, key), 7, &mut Shot(true), on_bone, orbs::Extras::NONE);
    articles.forget(id, &mut board);
    assert_eq!((board.live().count(), articles.tracked().iter().filter(|t| t.0 != 0).count()), (1, 0), "gone article");
    // Fighters and articles share the board's slots; a fifth article replaces the oldest, whose orbs go.
    assert_eq!(orbs::ORB_FIGHTERS + orbs::ARTICLES, FIGHTER_SETS);
    for k in 1..=orbs::ARTICLES {
        articles.track(k as u32, k << 4, Some(&mut board), true).unwrap();
        orbs::article_pass(&mut board, &mut HandleSet::new(), (k as u32, k << 4), 7, &mut Shot(true), on_bone, orbs::Extras::NONE);
    }
    assert_eq!(board.live().count(), 1 + orbs::ARTICLES);
    // The animation hook still observes the command while a projection holds the board.
    articles.track(99, 99 << 4, None, true).unwrap();
    articles.clear_evicted(&mut board);
    assert_eq!(board.live().count(), orbs::ARTICLES, "the oldest article's orb went");
    let mut ids = articles.tracked().map(|t| t.0);
    ids.sort();
    assert_eq!(ids, [2, 3, 4, 99], "article 1, the oldest, was replaced");
    // Hitbox Visualization Off: every article goes.
    articles.forget_all(Some(&mut board));
    assert_eq!((board.live().count(), articles.tracked()), (1, [(0, 0); orbs::ARTICLES]));
}

/// rc.13 (the primary 2026-11-14 06:10Z; Eden wa-grab-bfc1a3b9-6): Olimar's grab is a `CATCH` his Pikmin, an article, runs,
/// and no grab box was drawn. The command's spheres wait in the article's own set with the status it ran in and are drawn
/// as the fighters' are (deluxe-2's rule): purple on the article's bone while it stays in that status, once when the
/// status already moved on, and the article stays tracked while it has one.
fn an_articles_grab_box_is_an_own_orb_while_its_status_lasts() {
    let (id, key) = (0x102e_0001u32, 0xb000usize);
    let mut board = Board::new();
    let mut articles = orbs::Articles::new();
    let tint = hitbox::GRAB_TINT;
    let pikmin = Sphere { key: hitbox::sphere_key(hitbox::KIND_GRAB, 0, 0), joint: 0x3_1ed9_1fca, pos: [0.0; 3], scale: hitbox::sphere_scale(3.0), tint };
    let on_bone = |s: &Sphere| (s.joint == 0x3_1ed9_1fca).then_some([25.0, 4.0, 0.0]);
    let set = articles.track(id, key, Some(&mut board), true).unwrap().0;
    assert!(set.push_grab(pikmin, Some(40)));
    for pass in 0..3 {
        assert!(orbs::article_pass(&mut board, set, (id, key), 40, &mut Shot(false), on_bone, orbs::Extras::NONE), "pass {pass}: no hitbox, still tracked");
        let grab: Vec<Orb> = board.live().copied().collect();
        assert_eq!(grab.len(), 1, "pass {pass}: the Pikmin's grab box");
        assert_eq!((grab[0].kind(), grab[0].centre), (hitbox::KIND_GRAB, [25.0, 4.0, 0.0]));
        assert_eq!(&grab[0].color[..3], &model::panes::linear([153, 51, 255, 255])[..3], "purple, as the fighters'");
        assert!((grab[0].radius - 3.0).abs() < 1e-5);
    }
    assert!(!orbs::article_pass(&mut board, set, (id, key), 41, &mut Shot(false), on_bone, orbs::Extras::NONE), "its status ended: forgotten");
    assert_eq!(board.live().count(), 0);
    // Set the frame the grab connected (the pass already sees the next status): drawn that pass, gone at the next.
    assert!(set.push_grab(pikmin, Some(40)));
    assert!(orbs::article_pass(&mut board, set, (id, key), 41, &mut Shot(false), on_bone, orbs::Extras::NONE));
    assert_eq!(board.live().count(), 1);
    assert!(!orbs::article_pass(&mut board, set, (id, key), 41, &mut Shot(false), on_bone, orbs::Extras::NONE));
    // A grab box beside a hitbox: both drawn, the hitbox's colour unchanged.
    assert!(set.push_grab(pikmin, Some(40)));
    assert!(orbs::article_pass(&mut board, set, (id, key), 40, &mut Shot(true), on_bone, orbs::Extras::NONE));
    let mut kinds: Vec<u32> = board.live().map(|o| o.kind()).collect();
    kinds.sort();
    assert_eq!(kinds, [hitbox::KIND_HIT, hitbox::KIND_GRAB]);
    // A forgotten article drops what its set held.
    assert!(articles.track(id, key, Some(&mut board), true).unwrap().0.push_grab(pikmin, Some(40)));
    articles.forget(id, &mut board);
    assert!(articles.track(id, key, Some(&mut board), true).unwrap().0.is_empty(), "a forgotten article's waiting grab box went with it");
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
    let set = articles.track(id, key, Some(&mut board), true).unwrap().0;
    for pass in 0..3 {
        assert!(orbs::article_pass(&mut board, set, (id, key), 9, &mut Shot(false), on_bone, orbs::Extras { areas: &pull, ..orbs::Extras::NONE }), "pass {pass}: no hitbox, a wind area: kept");
        let drawn: Vec<Orb> = board.live().copied().collect();
        assert_eq!(drawn.len(), 3, "pass {pass}: the disc, the line's shadow and the line");
        assert!(drawn.iter().all(|o| o.kind() == legend::KIND_AREA));
        assert_eq!((drawn[0].centre, drawn[0].radius), ([10.0, 20.0, 0.0], 30.0));
    }
    // Its hitbox beside its area: the sphere first, then the area's three orbs.
    assert!(orbs::article_pass(&mut board, set, (id, key), 9, &mut Shot(true), on_bone, orbs::Extras { areas: &pull, ..orbs::Extras::NONE }));
    let kinds: Vec<u32> = board.live().map(|o| o.kind()).collect();
    assert_eq!(kinds, [hitbox::KIND_HIT, legend::KIND_AREA, legend::KIND_AREA, legend::KIND_AREA]);
    // The area ended and nothing is held: forgotten, its orbs gone.
    assert!(!orbs::article_pass(&mut board, set, (id, key), 9, &mut Shot(false), on_bone, orbs::Extras::NONE));
    assert_eq!(board.live().count(), 0);
}

fn article_hurtboxes_take_free_slots_and_draw_after_every_other_kind() {
    let (mut board, mut articles) = (Board::new(), orbs::Articles::new());
    for id in 1..=4 { articles.track(id, id as usize * 16, Some(&mut board), true).unwrap(); }
    let before = articles.tracked();
    assert!(!articles.track(99, 0x990, Some(&mut board), false).is_some());
    assert_eq!(articles.tracked(), before, "hurt-only discovery never evicts an attacking object");
    articles.forget(3, &mut board);
    assert!(articles.track(99, 0x990, Some(&mut board), false).is_some());
    assert!(!*articles.track(99, 0x990, Some(&mut board), false).unwrap().1);
    articles.track(100, 0x1000, Some(&mut board), true).unwrap();
    assert!(!articles.tracked().contains(&(99, 0x990)), "an attack evicts the passive object before an older attacker");
    assert!(articles.tracked().contains(&(1, 16)));
    articles.forget(100, &mut board);
    assert!(articles.track(99, 0x990, Some(&mut board), false).is_some());
    articles.track(99, 0x990, Some(&mut board), true).unwrap();
    assert!(*articles.track(99, 0x990, Some(&mut board), false).unwrap().1, "ATTACK promotes an existing passive object");
    // One article pill published first: its two endpoints still pair, and every other kind draws before it.
    let hurts = [0, 1].map(|end| sphere(hitbox::sphere_key(hitbox::KIND_ARTICLE_HURT, 0, end), end as f32 * 2.0, 2.0));
    let mut set = HandleSet::new();
    let extras = orbs::Extras { hurt: &hurts, ..orbs::Extras::NONE };
    let place = |s: &Sphere| Some(s.pos);
    assert!(orbs::article_pass(&mut board, &mut set, (99, 0x990), 0, &mut Shot(false), place, extras));
    board.publish(0xf000, [hitbox::KIND_HURT, model::legend::KIND_MARK, hitbox::KIND_HIT].into_iter().map(|k| orb_of_kind(0.0, k)));
    let kinds: Vec<_> = board.in_draw_order().map(|o| o.kind()).collect();
    assert_eq!(kinds, [hitbox::KIND_HIT, hitbox::KIND_HURT, model::legend::KIND_MARK, hitbox::KIND_ARTICLE_HURT]);
    let pill = board.in_draw_order().last().unwrap();
    assert_ne!(pill.centre, pill.tail);
    let mut panes = orbs::PaneUse { caps: model::panes::CAPS - 6, bands: 0 };
    let kept: Vec<_> = board.in_draw_order().filter(|o| panes.take(o.centre != o.tail).is_some()).map(|o| o.kind()).collect();
    assert_eq!(kept, kinds[..3], "the article pill is cut first and whole");
    assert!(!orbs::article_pass(&mut board, &mut set, (99, 0x990), 0, &mut Shot(false), place, orbs::Extras::NONE), "unticking removes its hurt-only lifetime");
    assert!(board.live().all(|o| o.kind() != hitbox::KIND_ARTICLE_HURT));
    articles.forget_all(Some(&mut board));
    assert_eq!(articles.tracked(), [(0, 0); orbs::ARTICLES]);
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

#[test]
fn wind_box_projection_uses_its_corners() {
    use model::legend::{area_orbs, AreaShape, WindArea};

    let area = WindArea { shape: AreaShape::Box { min: [-16.0, 8.0], max: [32.0, 24.0] }, angle: Some(170.0) };
    let orb = area_orbs(&area, 0, 0.0, [255; 4])[0].unwrap();
    let project = |[x, y, _]: [f32; 3]| Some([512.0 + 2.0 * x, 512.0 - y]);
    let (centre, size) = orbs::plan_wind_box(&orb, [512.0; 2], project).unwrap();
    assert_eq!(centre, [30.0, 16.875]);
    assert_eq!(size, [180.0, 16.875]);
}

/// rc.15 target-hurtbox-size: the game's hurt radius is the part's size at its bone's world scale (Eden
/// target-hurtbox-d6393cb0-yoga-4: the 0.6-posed lock-on Sandbag's size-5 parts touch at 3.45, its world scale 0.69;
/// Mario's size-1.2 parts at 1.14). A hurtbox orb is drawn at that radius; a hitbox orb keeps its size.
fn a_hurtbox_orb_is_its_size_at_the_bones_world_scale() {
    let place = |s: &Sphere| Some([10.0 + 0.69 * s.pos[0], 6.0 + 0.69 * s.pos[1], 0.69 * s.pos[2]]);
    for kind in [hitbox::KIND_HURT, hitbox::KIND_ARTICLE_HURT] {
        let mut set = HandleSet::new();
        let hurt = Sphere { key: hitbox::sphere_key(kind, 0, 0), joint: 0x1234, pos: [2.0, 0.0, 0.0], scale: hitbox::sphere_scale(5.0), tint: hitbox::HURTBOX_TINT };
        set.reconcile(&[hurt], true, &mut OrbSink { status: 1 });
        let mut out = [Orb::NONE; orbs::BOARD_ORBS];
        assert_eq!(orbs::place_orbs(&set, place, None, |_| None, &mut out), 1);
        assert!((out[0].radius - 3.45).abs() < 1e-4, "kind {kind}: radius {}", out[0].radius);
        assert!((out[0].centre[0] - 11.38).abs() < 1e-4 && (out[0].centre[1] - 6.0).abs() < 1e-4);
    }
    let mut set = HandleSet::new();
    let hit = Sphere { key: hitbox::sphere_key(hitbox::KIND_HIT, 0, 0), joint: 0x1234, pos: [2.0, 0.0, 0.0], scale: hitbox::sphere_scale(5.0), tint: hitbox::HURTBOX_TINT };
    set.reconcile(&[hit], true, &mut OrbSink { status: 1 });
    let mut out = [Orb::NONE; orbs::BOARD_ORBS];
    assert_eq!(orbs::place_orbs(&set, place, None, |_| None, &mut out), 1);
    assert!((out[0].radius - 5.0).abs() < 1e-4, "a hitbox orb keeps its size: {}", out[0].radius);
}

type Place = fn(&Sphere) -> Option<[f32; 3]>;

/// rc.15 target-hurtbox-size review: when the bone's scale can't be sampled (the unit offset doesn't place, or places
/// on the centre or at a non-finite point) the hurtbox orb is left out that frame rather than drawn at the unscaled size.
fn a_hurtbox_without_a_valid_bone_scale_is_not_drawn() {
    let samples: [Place; 3] = [
        |s| (s.pos[0] == 2.0).then_some([10.0, 6.0, 0.0]),
        |_| Some([10.0, 6.0, 0.0]),
        |s| Some(if s.pos[0] == 2.0 { [10.0, 6.0, 0.0] } else { [f32::NAN, 6.0, 0.0] }),
    ];
    for place in samples {
        let mut set = HandleSet::new();
        let hurt = Sphere { key: hitbox::sphere_key(hitbox::KIND_HURT, 0, 0), joint: 0x1234, pos: [2.0, 0.0, 0.0], scale: hitbox::sphere_scale(5.0), tint: hitbox::HURTBOX_TINT };
        set.reconcile(&[hurt], true, &mut OrbSink { status: 1 });
        let mut out = [Orb::NONE; orbs::BOARD_ORBS];
        assert_eq!(orbs::place_orbs(&set, place, None, |_| None, &mut out), 0, "no unscaled hurtbox orb");
    }
}

/// rc.15 target-hurtbox-display: a live target's hurtbox is its break volume, the R = 4 picture disc at its spot, drawn as a
/// hurtbox; break animations and the cursor get none.
fn a_live_targets_hurtbox_is_its_picture_disc() {
    use model::panes::{EggPicture, EGG_WHOLE};
    let mut burst = EGG_WHOLE;
    burst.scale = 1.6;
    let mut cursor = EGG_WHOLE;
    cursor.picture = EggPicture::Cursor;
    let looks = [Some(([1.0, 2.0], EGG_WHOLE)), Some(([5.0, 6.0], burst)), None, Some(([3.0, 4.0], cursor)), Some(([-8.0, 9.0], EGG_WHOLE))];
    let mut out = [Orb::NONE; 8];
    let n = orbs::target_hurt_orbs(&looks, &mut out);
    assert_eq!(n, 2);
    for (orb, at) in out[..n].iter().zip([[1.0, 2.0, 0.0], [-8.0, 9.0, 0.0]]) {
        assert_eq!((orb.centre, orb.tail, orb.radius, orb.kind()), (at, at, model::eggs::RADIUS, hitbox::KIND_HURT));
        assert_eq!(orb.color, model::panes::linear(orbs::tint_colour(hitbox::HURTBOX_TINT, orbs::ORB_ALPHA)));
    }
    assert_eq!(orbs::target_hurt_orbs(&looks, &mut out[..1]), 1, "as many as fit");
}

/// rc.15 target-hurtbox-display review: a fighter's capsule drawn in an earlier frame, then a busy board with a live target:
/// that tick draws no target disc, so it takes no pane and the fighter's retained panes are unchanged.
fn a_busy_board_tick_leaves_the_fighters_panes_to_the_target() {
    let mut earlier = orbs::PaneUse::default();
    assert!(earlier.take(true).is_some(), "the capsule took a cap pair and a band");
    let mut target = [Orb::NONE; 1];
    let n = orbs::target_hurt_orbs(&[Some(([3.0, 4.0], model::panes::EGG_WHOLE))], &mut target);
    assert_eq!((orbs::targets_drawn(true, true, n), orbs::targets_drawn(true, false, n)), (1, 0), "held: drawn when ticked");
    let mut busy = orbs::PaneUse::default();
    for _ in 0..orbs::targets_drawn(false, true, n) {
        busy.take(false);
    }
    assert_eq!((busy.caps, busy.bands), (0, 0), "a busy tick writes no cap over the retained capsule");
    assert_eq!((earlier.caps, earlier.bands), (2, 1), "the capsule's panes stay as the earlier frame wrote them");
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("own_orbs_are_never_refused_and_conclude_with_the_status", own_orbs_are_never_refused_and_conclude_with_the_status as fn()),
        ("a_grab_box_set_in_the_frame_it_connects_shows_once_not_through_the_pull", a_grab_box_set_in_the_frame_it_connects_shows_once_not_through_the_pull as fn()),
        ("an_orb_is_the_hitbox_size_in_the_spheres_tint", an_orb_is_the_hitbox_size_in_the_spheres_tint as fn()),
        ("hidden_fighter_and_owned_objects_leave_and_rejoin_the_draw_order", hidden_fighter_and_owned_objects_leave_and_rejoin_the_draw_order as fn()),
        ("the_pill_panes_are_cut_whole_hurtboxes_first", the_pill_panes_are_cut_whole_hurtboxes_first as fn()),
        ("a_capsules_two_ends_are_one_orb", a_capsules_two_ends_are_one_orb as fn()),
        ("an_inactive_tick_hides_the_orbs_it_showed", an_inactive_tick_hides_the_orbs_it_showed as fn()),
        ("the_board_keeps_each_fighters_last_pass_and_ages_out_the_gone", the_board_keeps_each_fighters_last_pass_and_ages_out_the_gone as fn()),
        ("an_articles_hitboxes_are_own_orbs_while_its_attack_lasts", an_articles_hitboxes_are_own_orbs_while_its_attack_lasts as fn()),
        ("an_articles_grab_box_is_an_own_orb_while_its_status_lasts", an_articles_grab_box_is_an_own_orb_while_its_status_lasts as fn()),
        ("an_articles_wind_area_is_drawn_and_keeps_it_tracked", an_articles_wind_area_is_drawn_and_keeps_it_tracked as fn()),
        ("article_hurtboxes_take_free_slots_and_draw_after_every_other_kind", article_hurtboxes_take_free_slots_and_draw_after_every_other_kind as fn()),
        ("screen_half_mirrors_the_games_size_words", screen_half_mirrors_the_games_size_words as fn()),
        ("projected_orbs_map_onto_the_layout_canvas", projected_orbs_map_onto_the_layout_canvas as fn()),
        ("a_hurtbox_orb_is_its_size_at_the_bones_world_scale", a_hurtbox_orb_is_its_size_at_the_bones_world_scale as fn()),
        ("a_hurtbox_without_a_valid_bone_scale_is_not_drawn", a_hurtbox_without_a_valid_bone_scale_is_not_drawn as fn()),
        ("a_live_targets_hurtbox_is_its_picture_disc", a_live_targets_hurtbox_is_its_picture_disc as fn()),
        ("a_busy_board_tick_leaves_the_fighters_panes_to_the_target", a_busy_board_tick_leaves_the_fighters_panes_to_the_target as fn()),
        ("handed_off_commands_leave_what_immediate_commands_left", handed_off_commands_leave_what_immediate_commands_left as fn()),
        ("commands_pushed_from_several_threads_match_the_synchronous_result", commands_pushed_from_several_threads_match_the_synchronous_result as fn()),
        ("an_exhausted_pool_applies_directly_in_issue_order_or_skips_when_the_store_is_busy", an_exhausted_pool_applies_directly_in_issue_order_or_skips_when_the_store_is_busy as fn()),
        ("a_rebound_clears_only_its_own_fighters_published_orbs_at_once", a_rebound_clears_only_its_own_fighters_published_orbs_at_once as fn()),
        ("a_rebound_during_the_sampling_pass_hides_what_the_pass_published", a_rebound_during_the_sampling_pass_hides_what_the_pass_published as fn()),
    ]);
}

fn command_shape(key: usize, id: u32, life: Option<hitbox::FighterLife>, grab: bool, status: i32) -> orbs::Command {
    orbs::Command::Shape {
        key, id, life, grab, status,
        shape: hitbox::Shape { id: id as i32, joint: 7, size: 3.0, a: [1.0, 2.0, 3.0], b: Some([4.0, 5.0, 6.0]) },
    }
}

fn immediate(table: &mut hitbox::SetTable, articles: &mut orbs::Articles, generations: &hitbox::FighterGenerations, command: &orbs::Command) {
    match *command {
        orbs::Command::Shape { key, id, life, grab, shape, status } => {
            let mut wants = [Sphere::NONE; hitbox::SHAPE_SPHERES];
            let mut len = 0;
            let tint = hitbox::legend_colour(hitbox::KIND_GRAB, None, 0);
            hitbox::push_shape(&mut wants, &mut len, hitbox::KIND_GRAB, shape.id as usize, &shape, tint);
            let set = if let Some(life) = life {
                if !generations.is_current(life) { return; }
                let i = table.index_for(key, life);
                &mut table.sets[i]
            } else {
                articles.track(id, key, None, true).unwrap().0
            };
            let _ = wants[..len].iter().filter(|w| grab && set.push_grab(**w, Some(status))).count() as u32;
        }
        orbs::Command::Wind { key, id } => { articles.track(id, key, None, true).unwrap(); }
        orbs::Command::Rebound { key, life } => {
            if let Some(i) = table.find_life(key, life).filter(|_| generations.is_current(life)) {
                let _ = table.sets[i].clear(&mut OrbSink { status: i32::MIN });
                let _ = table.held_spheres();
            }
        }
    }
}

fn parity(table_a: &mut hitbox::SetTable, articles_a: &mut orbs::Articles, table_b: &mut hitbox::SetTable,
    articles_b: &mut orbs::Articles, fighters: &[(usize, hitbox::FighterLife)], article_keys: &[(u32, usize)], status: i32) {
    assert_eq!(format!("{table_a:?}"), format!("{table_b:?}"));
    assert_eq!(articles_a.tracked(), articles_b.tracked());
    let mut board_a = Board::new();
    let mut board_b = Board::new();
    board_a.begin_frame(7);
    board_b.begin_frame(7);
    for &(id, key) in article_keys {
        board_a.publish_article(key, id, None, [orb(key as f32)].into_iter());
        board_b.publish_article(key, id, None, [orb(key as f32)].into_iter());
    }
    for i in 0..FIGHTER_SETS {
        let a = &mut table_a.sets[i];
        let b = &mut table_b.sets[i];
        a.reconcile(&[], true, &mut OrbSink { status });
        b.reconcile(&[], true, &mut OrbSink { status });
        assert_eq!(a.handles().collect::<Vec<_>>(), b.handles().collect::<Vec<_>>());
        assert_eq!(a.spheres().copied().collect::<Vec<_>>(), b.spheres().copied().collect::<Vec<_>>());
    }
    for &(key, life) in fighters {
        if let Some(i) = table_a.find_life(key, life) {
            let mut a = [Orb::NONE; orbs::BOARD_ORBS];
            let mut b = [Orb::NONE; orbs::BOARD_ORBS];
            let n = orbs::place_orbs(&table_a.sets[i], |s| Some(s.pos), None, |_| None, &mut a);
            let m = orbs::place_orbs(&table_b.sets[i], |s| Some(s.pos), None, |_| None, &mut b);
            board_a.publish_fighter(key, life, a[..n].iter().copied());
            board_b.publish_fighter(key, life, b[..m].iter().copied());
        }
    }
    for (id, key) in articles_a.tracked().into_iter().filter(|(id, _)| *id != 0) {
        let (a, flag_a) = articles_a.track(id, key, None, false).unwrap();
        let (b, flag_b) = articles_b.track(id, key, None, false).unwrap();
        assert_eq!(*flag_a, *flag_b);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        orbs::article_pass(&mut board_a, a, (id, key), status, &mut Shot(false), |s| Some(s.pos), orbs::Extras::NONE);
        orbs::article_pass(&mut board_b, b, (id, key), status, &mut Shot(false), |s| Some(s.pos), orbs::Extras::NONE);
        assert_eq!(a.handles().collect::<Vec<_>>(), b.handles().collect::<Vec<_>>());
        assert_eq!(a.spheres().copied().collect::<Vec<_>>(), b.spheres().copied().collect::<Vec<_>>());
    }
    articles_a.clear_evicted(&mut board_a);
    articles_b.clear_evicted(&mut board_b);
    assert_eq!(board_a.draw_slots_in_order(u8::MAX).copied().collect::<Vec<_>>(),
        board_b.draw_slots_in_order(u8::MAX).copied().collect::<Vec<_>>());
}

fn handed_off_commands_leave_what_immediate_commands_left() {
    use model::pool_list::PoolList;
    let generations = hitbox::FighterGenerations::new();
    let life = generations.stamp(0x10000).unwrap();
    let retired = generations.stamp(0x20000).unwrap();
    generations.retire(0x20000);
    let article = |id| command_shape(id as usize * 16, id, None, true, 10);
    let catch = command_shape(0x1000, life.id, Some(life), true, 10);
    let rebound = orbs::Command::Rebound { key: 0x1000, life };
    let sequences = [
        vec![article(1), article(2), article(3), article(4), article(5)],
        vec![article(1), article(2), article(3), article(4), article(1), article(5)],
        vec![article(1), article(2), article(3), article(5)],
        vec![command_shape(16, 1, None, false, 10), article(1), orbs::Command::Wind { key: 16, id: 1 }],
        vec![catch],
        vec![command_shape(0x2000, retired.id, Some(retired), true, 10)],
        vec![catch, rebound],
        vec![rebound, catch],
        vec![orbs::Command::Rebound { key: 0x2000, life: retired }],
    ];
    let keys: Vec<_> = (1..=5).map(|id| (id, id as usize * 16)).collect();
    for (case, commands) in sequences.iter().enumerate() {
        let mut a = Box::new(hitbox::SetTable::new());
        let mut b = Box::new(hitbox::SetTable::new());
        let mut aa = orbs::Articles::new();
        let mut ab = orbs::Articles::new();
        if case == 2 {
            aa.track(4, 64, Some(&mut Board::new()), false).unwrap();
            ab.track(4, 64, Some(&mut Board::new()), false).unwrap();
        }
        let list = PoolList::new(commands.len()).unwrap();
        for command in commands {
            immediate(&mut a, &mut aa, &generations, command);
            assert!(list.push(*command));
        }
        assert_eq!(orbs::apply_all(&list, None, &mut b, &mut ab, &generations, &mut |_, _| {}), commands.len());
        if case <= 1 { assert!(!aa.tracked().contains(&(1, 16))); }
        if case == 2 { assert!(!aa.tracked().contains(&(4, 64)) && aa.tracked().contains(&(1, 16))); }
        parity(&mut a, &mut aa, &mut b, &mut ab, &[(0x1000, life), (0x2000, retired)], &keys, 11);
        if case == 4 {
            let i = b.find_life(0x1000, life).unwrap();
            assert_eq!(b.sets[i].handles().collect::<Vec<_>>(), [orbs::orb_handle(10); 2]);
        }
        if case == 5 { assert_eq!(b.find_life(0x2000, retired), None); }
    }
}

fn commands_pushed_from_several_threads_match_the_synchronous_result() {
    use model::pool_list::PoolList;
    let generations = hitbox::FighterGenerations::new();
    let fighters: Vec<_> = (1..=4).map(|n| (0x1000 * n, generations.stamp((n as u32) << 16).unwrap())).collect();
    let list = PoolList::new(12).unwrap();
    std::thread::scope(|scope| {
        for &(key, life) in &fighters {
            let list = &list;
            scope.spawn(move || {
                let id = life.id >> 16;
                assert!(list.push(orbs::Command::Wind { key: id as usize * 16, id }));
                assert!(list.push(command_shape(key, life.id, Some(life), true, 10)));
                assert!(list.push(orbs::Command::Wind { key: (id as usize + 4) * 16, id: id + 4 }));
            });
        }
    });
    let mut a = Box::new(hitbox::SetTable::new());
    let mut b = Box::new(hitbox::SetTable::new());
    let mut aa = orbs::Articles::new();
    let mut ab = orbs::Articles::new();
    let mut observed = Vec::new();
    assert_eq!(orbs::apply_all(&list, None, &mut b, &mut ab, &generations, &mut |c, _| observed.push(*c)), 12);
    for &(key, life) in &fighters {
        let id = life.id >> 16;
        let own: Vec<_> = observed.iter().copied().filter(|c| match *c {
            orbs::Command::Wind { id: found, .. } => found == id || found == id + 4,
            orbs::Command::Shape { key: found, .. } => found == key,
            _ => false,
        }).collect();
        assert_eq!(own, [orbs::Command::Wind { key: id as usize * 16, id }, command_shape(key, life.id, Some(life), true, 10),
            orbs::Command::Wind { key: (id as usize + 4) * 16, id: id + 4 }]);
    }
    for command in &observed { immediate(&mut a, &mut aa, &generations, command); }
    let keys: Vec<_> = (1..=8).map(|id| (id, id as usize * 16)).collect();
    parity(&mut a, &mut aa, &mut b, &mut ab, &fighters, &keys, 11);
}

fn an_exhausted_pool_applies_directly_in_issue_order_or_skips_when_the_store_is_busy() {
    use model::pool_list::PoolList;
    let generations = hitbox::FighterGenerations::new();
    let life = generations.stamp(0x10000).unwrap();
    let catch = command_shape(0x1000, life.id, Some(life), true, 10);
    let rebound = orbs::Command::Rebound { key: 0x1000, life };
    let list = PoolList::new(1).unwrap();
    let table = hos_lock::Mutex::new(Box::new(hitbox::SetTable::new()));
    let articles = hos_lock::Mutex::new(orbs::Articles::new());
    let owner = |run: &mut dyn FnMut(&mut hitbox::SetTable, &mut orbs::Articles)| {
        let (Some(mut table), Some(mut articles)) = (table.try_lock(), articles.try_lock()) else { return false };
        run(&mut table, &mut articles);
        true
    };
    assert_eq!(orbs::hand_off(&list, &generations, catch, owner, &mut |_, _| {}), orbs::HandOff::Queued);
    assert_eq!(orbs::hand_off(&list, &generations, rebound, owner, &mut |_, _| {}), orbs::HandOff::Direct);
    let mut reference = Box::new(hitbox::SetTable::new());
    let mut reference_articles = orbs::Articles::new();
    immediate(&mut reference, &mut reference_articles, &generations, &catch);
    immediate(&mut reference, &mut reference_articles, &generations, &rebound);
    parity(&mut reference, &mut reference_articles, &mut table.lock(), &mut articles.lock(), &[(0x1000, life)], &[], 11);
    {
        let table = table.lock();
        assert!(table.sets[table.find_life(0x1000, life).unwrap()].is_empty());
    }
    assert_eq!(orbs::hand_off(&list, &generations, catch, owner, &mut |_, _| {}), orbs::HandOff::Queued);
    let busy = table.lock();
    assert_eq!(orbs::hand_off(&list, &generations, rebound, owner, &mut |_, _| {}), orbs::HandOff::Skipped);
    drop(busy);
    assert_eq!(orbs::apply_all(&list, None, &mut table.lock(), &mut articles.lock(), &generations, &mut |_, _| {}), 1);
    assert!(list.push(catch));
    let third = orbs::Command::Wind { key: 16, id: 1 };
    let mut first = true;
    assert_eq!(orbs::hand_off(&list, &generations, rebound, owner, &mut |_, _| {
        if first { first = false; assert!(list.push(third)); }
    }), orbs::HandOff::Direct);
    assert!(!articles.lock().tracked().contains(&(1, 16)));
    assert_eq!(orbs::apply_all(&list, None, &mut table.lock(), &mut articles.lock(), &generations, &mut |c, _| assert_eq!(*c, third)), 1);
    assert!(articles.lock().tracked().contains(&(1, 16)));
    assert!(list.push(catch));
    assert!(!list.push(rebound));
    assert_eq!(list.drain(|_| {}), 1);
}

fn a_rebound_clears_only_its_own_fighters_published_orbs_at_once() {
    let generations = hitbox::FighterGenerations::new();
    let life = generations.stamp(0x10000).unwrap();
    let other = generations.stamp(0x20000).unwrap();
    let mut board = Board::new();
    board.begin_frame(7);
    board.publish_fighter(10, life, [orb(1.0)].into_iter());
    board.publish_fighter(20, other, [orb(2.0)].into_iter());
    board.publish_article(30, 1, None, [orb(3.0)].into_iter());
    let draw = |b: &Board| b.draw_slots_in_order(u8::MAX).map(|o| o.centre[0]).collect::<Vec<_>>();
    board.clear_publication(10, other, 7);
    board.clear_publication(99, life, 7);
    board.clear_publication(10, life, 8);
    assert_eq!(draw(&board), [1.0, 2.0, 3.0]);
    board.clear_publication(10, life, 7);
    assert_eq!(draw(&board), [2.0, 3.0]);
    board.begin_frame(8);
    board.publish_fighter(10, life, [orb(4.0)].into_iter());
    board.clear_publication(10, life, 7);
    assert_eq!(draw(&board), [4.0]);
}

fn a_rebound_during_the_sampling_pass_hides_what_the_pass_published() {
    let generations = hitbox::FighterGenerations::new();
    let life = generations.stamp(0x10000).unwrap();
    let other = generations.stamp(0x20000).unwrap();
    let catch = command_shape(10, life.id, Some(life), true, 10);
    let rebound = orbs::Command::Rebound { key: 10, life };
    for after_pass in [false, true] {
        let mut a = Box::new(hitbox::SetTable::new());
        let mut b = Box::new(hitbox::SetTable::new());
        let mut aa = orbs::Articles::new();
        let mut ab = orbs::Articles::new();
        immediate(&mut a, &mut aa, &generations, &catch);
        immediate(&mut b, &mut ab, &generations, &catch);
        for table in [&mut a, &mut b] {
            let i = table.find_life(10, life).unwrap();
            table.sets[i].reconcile(&[], true, &mut OrbSink { status: 10 });
            assert_eq!(table.sets[i].len(), 2);
        }
        let mut board_a = Board::new();
        let mut board_b = Board::new();
        for board in [&mut board_a, &mut board_b] {
            board.begin_frame(1);
            board.publish_fighter(10, life, [orb(1.0)].into_iter());
        }
        if !after_pass {
            board_a.clear(10);
            immediate(&mut a, &mut aa, &generations, &rebound);
            board_b.clear_publication(10, life, 1);
            orbs::apply(&mut b, &mut ab, &generations, &rebound);
        }
        for (board, table) in [(&mut board_a, &mut a), (&mut board_b, &mut b)] {
            board.begin_frame(2);
            let i = table.find_life(10, life).unwrap();
            let want = sphere(hitbox::sphere_key(hitbox::KIND_HIT, 0, 0), 4.0, 1.0);
            table.sets[i].reconcile(&[want], true, &mut OrbSink { status: 10 });
            let mut placed = [Orb::NONE; orbs::BOARD_ORBS];
            let count = orbs::place_orbs(&table.sets[i], |s| Some(s.pos), None, |_| None, &mut placed);
            board.publish_fighter(10, life, placed[..count].iter().copied());
            board.publish_fighter(20, other, [orb(2.0)].into_iter());
            board.publish_article(30, 1, None, [orb(3.0)].into_iter());
        }
        if after_pass {
            board_a.clear(10);
            immediate(&mut a, &mut aa, &generations, &rebound);
            board_b.clear_publication(10, life, 1);
            assert!(matches!(orbs::apply(&mut b, &mut ab, &generations, &rebound), orbs::Applied::Rebound(Some(_))));
            board_b.clear_publication(10, life, 2);
        } else {
            board_b.clear_publication(10, life, 1);
        }
        let draw = |board: &Board| board.draw_slots_in_order(u8::MAX).map(|o| o.centre[0]).collect::<Vec<_>>();
        assert_eq!(draw(&board_a), draw(&board_b));
        assert_eq!(draw(&board_b), if after_pass { vec![2.0, 3.0] } else { vec![4.0, 2.0, 3.0] });
        parity(&mut a, &mut aa, &mut b, &mut ab, &[(10, life)], &[], 10);
    }
}

#[test]
fn visible_frame_native_reads_are_lazy() {
    let runtime = include_str!("../src/runtime/features/hitbox.rs");
    let cpu = include_str!("../src/runtime/features/hide_cpu.rs");
    assert!(!runtime.contains("Some(natives::battle_object_id(accessor))"), "every visible fighter reads its object ID");
    assert!(cpu.contains("body.object_id = (body.away && BASE.load"), "the CPU's object ID read requires an admitted hide");
    assert!(runtime.contains("owner: hide.owner(|| article_owner(id, key))"), "the live caller must use the tested conditional owner read");
}

#[test]
fn overlay_board_exposes_only_the_completed_simulation_frame() {
    let fighter = orbs::orb_of(&sphere(hitbox::sphere_key(hitbox::KIND_HURT, 0, 0), 0.0, 2.0), [1.0, 0.0, 0.0]);
    let article = orbs::orb_of(&sphere(hitbox::sphere_key(hitbox::KIND_HIT, 0, 0), 0.0, 2.0), [3.0, 0.0, 0.0]);
    let mut board = Board::new();
    assert!(!board.is_frame(0));

    board.begin_frame(1);
    board.publish(10, [fighter].into_iter());
    board.publish(20, [article].into_iter());
    assert!(board.is_frame(1));
    assert!(!board.is_frame(2), "a skipped publication must not reuse the old pose");

    board.begin_frame(2);
    let current = Orb { centre: [2.0, 0.0, 0.0], tail: [2.0, 0.0, 0.0], ..fighter };
    board.publish(10, [current].into_iter());
    assert!(board.is_frame(2));
    assert_eq!(board.in_draw_order().copied().collect::<Vec<_>>(), [current], "the removed object has no pane in frame 2");
}

#[test]
fn paused_editor_polls_update_pictures_without_advancing_the_world_sample() {
    use model::btt::{Btt, Pad, Phase};
    use model::chords::logical::{A, X};
    use model::panes::EggPicture;

    let mut board = Board::new();
    board.begin_frame(7);
    let orb = orbs::orb_of(
        &sphere(hitbox::sphere_key(hitbox::KIND_HURT, 0, 0), 0.0, 2.0),
        [4.0, 6.0, 0.0],
    );
    board.publish(10, [orb].into_iter());

    let mut mode = Btt::IDLE;
    mode.close_menu(true, true);
    let before = mode.editor_pictures().unwrap();
    let cursor = |pictures: [model::eggs::Look; model::btt::LOOKS]| {
        pictures.into_iter().flatten().find_map(|(point, look)| {
            (look.picture == EggPicture::Cursor).then_some(point)
        })
    };
    assert!(mode.feed(Pad { stick: [1.0, 0.0], ..Pad::default() }));
    let moved = mode.editor_pictures().unwrap();
    assert!(cursor(moved).unwrap()[0] > cursor(before).unwrap()[0]);

    let place = 1u16 << A;
    assert!(mode.feed(Pad { held: place, pressed: place, ..Pad::default() }));
    let placed = mode.editor_pictures().unwrap();
    assert!(placed.into_iter().flatten().any(|(point, look)| {
        look.picture == EggPicture::Whole && Some(point) == cursor(moved)
    }));
    assert!(mode.sample_simulation_pictures().iter().all(Option::is_none));

    let confirm = 1u16 << X;
    assert!(mode.feed(Pad { held: confirm, pressed: confirm, ..Pad::default() }));
    assert_eq!(mode.phase(), Phase::Ready);
    assert!(!mode.feed(Pad { held: place, pressed: place, ..Pad::default() }));
    assert_eq!(mode.phase(), Phase::Running);
    for _ in 0..3 {
        let still_visible = mode.editor_pictures()
            .expect("a held run keeps its committed layout visible before the first sample");
        assert!(still_visible.into_iter().flatten().any(|(point, look)| {
            look.picture == EggPicture::Whole && Some(point) == cursor(moved)
        }));
    }
    assert!(board.is_frame(7));
    assert_eq!(board.in_draw_order().copied().collect::<Vec<_>>(), [orb]);

    assert_eq!(mode.pass_indexed(true, |_, _| true), 1);
    assert_eq!(mode.phase(), Phase::Done);
    let sample = mode.sample_simulation_pictures();
    assert!(sample.into_iter().flatten().any(|(point, _)| Some(point) == cursor(moved)));
    assert!(sample.into_iter().flatten().all(|(_, look)| look.picture != EggPicture::Whole));
    assert!(mode.editor_pictures().is_none(), "the HUD uses the current simulation sample");
}

const RESET_ACCESSOR: usize = 0x1000;
const RESET_ID: u32 = 0x0003_002a;

#[test]
fn delayed_worker_init_cannot_erase_a_fresh_overlay_sample() {
    let generations = hitbox::FighterGenerations::new();
    let old = generations.stamp(RESET_ID).unwrap();
    // The worker's sole write is the reset. Phase 8 runs before the worker returns.
    let resume = std::sync::Barrier::new(2);
    let marked = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            generations.retire(RESET_ID);
            marked.wait();
            resume.wait();
        });
        marked.wait();
        let life = generations.stamp(RESET_ID).unwrap();
        let anchor = (life, [2.0, 0.0, 0.0]);
        let mut board = Board::new();
        board.publish_fighter(RESET_ACCESSOR, life, [orb(2.0)].into_iter());
        resume.wait();

        assert!(!generations.is_current(old));
        let slots = board.draw_slots(&orbs::DrawHide::VISIBLE, &generations);
        assert_eq!(board.draw_slots_in_order(slots).map(|o| o.centre[0]).collect::<Vec<_>>(), [2.0]);
        assert_eq!(generations.is_current(anchor.0).then_some(anchor.1), Some([2.0, 0.0, 0.0]));
    });
}

#[test]
fn retirement_during_a_busy_board_invalidates_only_the_old_fighter() {
    let generations = hitbox::FighterGenerations::new();
    let old = generations.stamp(RESET_ID).unwrap();
    let other = generations.stamp(0x0004_002a).unwrap();
    let board = hos_lock::Mutex::new(Board::new());
    let mut in_flight = board.lock();

    // The sample already captured its life. The worker completes while the sample owns BOARD.
    std::thread::scope(|scope| {
        scope.spawn(|| generations.retire(RESET_ID));
    });
    in_flight.publish_fighter(RESET_ACCESSOR, old, [orb(1.0)].into_iter());
    in_flight.publish_fighter(0x2000, other, [orb(3.0)].into_iter());
    in_flight.publish_article(0x3000, 0x1000_0011, None, [orb(4.0)].into_iter());
    drop(in_flight);

    let board = board.lock();
    let slots = board.draw_slots(&orbs::DrawHide::VISIBLE, &generations);
    assert_eq!(board.draw_slots_in_order(slots).map(|o| o.centre[0]).collect::<Vec<_>>(), [3.0, 4.0]);
    assert!(!generations.is_current(old), "the layout must also reject the old anchor");
    assert!(generations.is_current(other));
}

#[test]
fn retirement_overlapping_first_bind_cannot_carry_an_old_grab_into_the_new_lifetime() {
    let generations = hitbox::FighterGenerations::new();
    let mut cache = hitbox::SetTable::new();

    // The command gets through its current-life check. Init runs before it first binds the slot.
    let old = generations.stamp(RESET_ID).unwrap();
    assert!(generations.is_current(old));
    generations.retire(RESET_ID);
    let slot = cache.index_for(RESET_ACCESSOR, old);
    let grab = sphere(hitbox::KIND_GRAB << 24, 1.0, 2.0);
    assert!(cache.sets[slot].push_grab(grab, None));

    let current = generations.stamp(RESET_ID).unwrap();
    assert_eq!(cache.find_life(RESET_ACCESSOR, current), None);
    let rebound = cache.index_for(RESET_ACCESSOR, current);
    assert_eq!(rebound, slot);
    assert!(cache.sets[slot].is_empty());
    assert!(cache.sets[slot].push_grab(grab, None));
    cache.forget_retired(&generations);
    assert!(!cache.sets[slot].is_empty(), "the new lifetime's grab survives the retirement");
}

#[test]
fn overlay_worker_retirement_reaches_both_hud_readers_without_a_board_write() {
    let src = include_str!("../src/runtime/features/hitbox.rs");
    let body = |name: &str| src.split_once(name).unwrap().1.split_once("\n}").unwrap().0;
    let init = body("fn forget_fighter(");
    assert!(init.contains("FIGHTER_GENERATIONS.retire(id)"));
    assert!(!init.contains("BOARD") && !init.contains("STORE") && !init.contains("clear_board"));
    assert!(body("fn sample_overlay(").contains(".map(|world| (life, world))"));
    assert!(body("fn publish_orbs(").contains("board.publish_fighter(accessor, life,"));
    assert!(body("fn fighter_screen_anchors(").contains("FIGHTER_GENERATIONS.is_current(life)"));
    assert!(body("fn draw_orbs(").contains("b.draw_slots(&hide, &FIGHTER_GENERATIONS)"));
}

/// The overlay board is all zero bytes, so it stays in `.bss`, and that pattern is never drawn: the one read path
/// requires the board's first completed frame.
#[test]
fn the_overlay_board_starts_as_zero_bytes_that_no_reader_accepts() {
    let src = include_str!("../src/runtime/features/hitbox.rs");
    assert!(src.contains("static BOARD: Mutex<OverlaySample> = Mutex::new(unsafe { core::mem::zeroed() });"));
    let body = |name: &str| src.split_once(name).unwrap().1.split_once("\n}").unwrap().0;
    assert!(body("fn read_overlay_sample(").contains("sample.board.is_frame(SAMPLE.load(Ordering::Acquire))"));
    assert_eq!(src.matches("BOARD.try_lock()").count(), 3, "clear_board, read_overlay_sample and sample_overlay");
    assert_eq!(src.matches("BOARD.lock()").count(), 0);
    // SAFETY: zero is a valid value of every field of the board.
    let zero: Board = unsafe { std::mem::zeroed() };
    let generations = hitbox::FighterGenerations::new();
    assert!(!zero.is_frame(0) && !Board::new().is_frame(0));
    let slots = zero.draw_slots(&orbs::DrawHide::VISIBLE, &generations);
    assert_eq!(zero.draw_slots_in_order(slots).count(), 0);
}
