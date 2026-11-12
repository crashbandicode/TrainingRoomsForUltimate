//! Own orbs (decision HB-OWN-ORBS, `model::hitbox_orbs`): the own-orb `Effects` sink inside the same
//! `HandleSet` the visualizer uses (never refused, concluded with the status like route B's effects), the board the layout
//! tick reads, and the projection's pane plan.

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
    assert!(set.push_grab(grab));
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
        assert_eq!(orbs::orb_of(&sphere(hitbox::sphere_key(kind, 3, 1), 0.0, 1.0), [0.0; 3]).kind, kind);
    }
    // The tints of the colour table go through the layout's linear-light encoding.
    let orange = orbs::orb_of(&Sphere { tint: hitbox::tint(1), ..s }, [0.0; 3]);
    assert_eq!(orange.color, model::panes::linear([255, 102, 0, orbs::ORB_ALPHA]));
}

fn orb(x: f32) -> Orb {
    Orb { centre: [x, 0.0, 0.0], radius: 1.0, color: [1, 2, 3, 4], kind: hitbox::KIND_HURT }
}

fn orb_of_kind(x: f32, kind: u32) -> Orb {
    Orb { kind, ..orb(x) }
}

/// Review HB-2: the panes cover Training's worst case, four fighter objects (Player 1 and the CPU, each possibly Ice
/// Climbers with Nana) of `MAX_HELD` spheres each, so a full board is drawn without a cut.
fn the_orb_panes_cover_four_full_fighters() {
    assert_eq!(model::panes::ORBS, orbs::ORB_FIGHTERS * MAX_HELD);
    let mut board = Board::new();
    for f in 0..orbs::ORB_FIGHTERS {
        let kind = |i: usize| if i.is_multiple_of(8) { hitbox::KIND_HIT } else { hitbox::KIND_HURT };
        assert_eq!(board.publish((f + 1) << 8, (0..MAX_HELD).map(|i| orb_of_kind(i as f32, kind(i)))), MAX_HELD);
    }
    assert_eq!(board.in_draw_order().count(), model::panes::ORBS, "every orb of a full board has a pane");
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
    // At most MAX_HELD per fighter.
    assert_eq!(board.publish(0xc0, (0..MAX_HELD + 5).map(|i| orb(i as f32))), MAX_HELD);
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
    let pane = orbs::plan_pane([960.0, 540.0], [[980.0, 540.0], [960.0, 530.0]], half, color).unwrap();
    assert_eq!((pane.x, pane.y, pane.size, pane.color), (0.0, 0.0, 40.0, color), "the longer projected radius, doubled");
    let small = orbs::plan_pane([640.0, 360.0], [[650.0, 360.0], [640.0, 365.0]], [640.0, 360.0], color).unwrap();
    assert_eq!(small.size, 30.0, "720p pixels are 1.5 canvas units");
    assert!(orbs::plan_pane([f32::NAN, 0.0], [[0.0; 2]; 2], half, color).is_none());
    assert!(orbs::plan_pane([960.0, 540.0], [[960.2, 540.0], [960.0, 540.2]], half, color).is_none(), "under a unit");
    assert!(orbs::plan_pane([5000.0, 540.0], [[5010.0, 540.0], [5000.0, 530.0]], half, color).is_none(), "off the canvas");
    assert!(orbs::plan_pane([1925.0, 540.0], [[1955.0, 540.0], [1925.0, 540.0]], half, color).is_some(), "partly on the canvas");
    assert_eq!(orbs::plan_pane([960.0, 540.0], [[9000.0, 540.0], [960.0, 540.0]], half, color).unwrap().size, orbs::MAX_PANE);
    assert!(orbs::plan_pane([960.0, 540.0], [[970.0, 540.0]; 2], [0.0, 540.0], color).is_none(), "no screen size");
    let o = Orb { centre: [1.0, 2.0, 3.0], radius: 0.5, color, kind: hitbox::KIND_HIT };
    assert_eq!(orbs::edge_points(&o), [[1.5, 2.0, 3.0], [1.0, 2.5, 3.0]]);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("own_orbs_are_never_refused_and_conclude_with_the_status", own_orbs_are_never_refused_and_conclude_with_the_status as fn()),
        ("an_orb_is_the_hitbox_size_in_the_spheres_tint", an_orb_is_the_hitbox_size_in_the_spheres_tint as fn()),
        ("the_orb_panes_cover_four_full_fighters", the_orb_panes_cover_four_full_fighters as fn()),
        ("an_inactive_tick_hides_the_orbs_it_showed", an_inactive_tick_hides_the_orbs_it_showed as fn()),
        ("the_board_keeps_each_fighters_last_pass_and_ages_out_the_gone", the_board_keeps_each_fighters_last_pass_and_ages_out_the_gone as fn()),
        ("screen_half_mirrors_the_games_size_words", screen_half_mirrors_the_games_size_words as fn()),
        ("projected_orbs_map_onto_the_layout_canvas", projected_orbs_map_onto_the_layout_canvas as fn()),
    ]);
}
