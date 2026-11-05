//! HB1 route A4 prototype (own orbs, `model::hitbox_orbs`): the request parser, the own-orb `Effects` sink inside the same
//! `HandleSet` the visualizer uses (never refused, concluded with the status like route B's effects), the board the layout
//! tick reads, and the projection's pane plan.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::hitbox::{self, HandleSet, Sphere, FIGHTER_SETS, MAX_HELD};
use model::hitbox_orbs::{self as orbs, Board, Orb, OrbSink, Request, Route, View};

fn sphere(key: u32, x: f32, size: f32) -> Sphere {
    Sphere { key, joint: 0x1234, pos: [x, 0.0, 0.0], scale: hitbox::sphere_scale(size), tint: hitbox::HURTBOX_TINT }
}

#[test]
fn requests_parse_route_view_and_joint_flag() {
    assert_eq!(orbs::parse_request("route=orbs"), Request { route: Some(Route::Orbs), view: None, joint_flag: None });
    assert_eq!(
        orbs::parse_request(" view=effects\nroute=both joint=1 junk route=nope"),
        Request { route: Some(Route::Both), view: Some(View::Effects), joint_flag: Some(true) }
    );
    assert_eq!(orbs::parse_request("joint=0 view=orbs"), Request { route: None, view: Some(View::Orbs), joint_flag: Some(false) });
    assert_eq!(orbs::parse_request(""), Request::default());
    for route in [Route::Effects, Route::Orbs, Route::Both] {
        assert_eq!(Route::from_code(route.code()), route);
    }
    for view in [View::Effects, View::Orbs, View::Both] {
        assert_eq!(View::from_code(view.code()), view);
    }
}

#[test]
fn layers_follow_route_and_view() {
    assert_eq!(orbs::layers(Route::Effects, View::Orbs), (false, true), "route B ignores the view");
    assert_eq!(orbs::layers(Route::Orbs, View::Effects), (true, false), "orbs only never shows effects");
    assert_eq!(orbs::layers(Route::Both, View::Effects), (false, true));
    assert_eq!(orbs::layers(Route::Both, View::Orbs), (true, false));
    assert_eq!(orbs::layers(Route::Both, View::Both), (true, true));
    assert!(!Route::Effects.orbs() && Route::Orbs.orbs() && Route::Both.orbs());
    assert!(Route::Effects.effects() && !Route::Orbs.effects() && Route::Both.effects());
}

#[test]
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

#[test]
fn an_orb_is_the_hitbox_size_in_the_spheres_tint() {
    let s = Sphere { key: 1, joint: 7, pos: [0.0; 3], scale: hitbox::sphere_scale(4.5), tint: [1.0, 0.0, 0.0] };
    let orb = orbs::orb_of(&s, [1.0, 2.0, 3.0]);
    assert_eq!(orb.centre, [1.0, 2.0, 3.0]);
    assert!((orb.radius - 4.5).abs() < 1e-5);
    assert_eq!(orb.color, [255, 0, 0, orbs::ORB_ALPHA]);
    let yellow = orbs::orb_of(&sphere(2, 0.0, 1.0), [0.0; 3]);
    assert_eq!(&yellow.color[..3], &[255, 255, 0]);
    // The tints of the colour table go through the layout's linear-light encoding.
    let orange = orbs::orb_of(&Sphere { tint: hitbox::tint(1), ..s }, [0.0; 3]);
    assert_eq!(orange.color, model::panes::linear([255, 102, 0, orbs::ORB_ALPHA]));
}

fn orb(x: f32) -> Orb {
    Orb { centre: [x, 0.0, 0.0], radius: 1.0, color: [1, 2, 3, 4] }
}

#[test]
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
    // A fighter that stops publishing ages out while others go on; a paused board (no publishes) keeps everything.
    for _ in 0..orbs::STALE_PUBLISHES {
        board.publish(0xa0, (0..10).map(|i| orb(i as f32)));
    }
    assert_eq!(board.live().count(), 10, "0xb0 aged out");
    let before: Vec<Orb> = board.live().copied().collect();
    assert_eq!(board.live().copied().collect::<Vec<_>>(), before);
    // Every slot can be taken; a new fighter replaces the stalest.
    for k in 1..=FIGHTER_SETS + 2 {
        board.publish(k << 8, std::iter::once(orb(k as f32)));
    }
    assert!(board.live().count() <= FIGHTER_SETS);
    board.clear_all();
    assert_eq!(board.live().count(), 0);
    assert_eq!(board.publish(0, std::iter::once(orb(0.0))), 0, "no fighter, no orbs");
}

#[test]
fn screen_half_mirrors_the_games_size_words() {
    assert_eq!(orbs::screen_half(0.0, 960.0), 960.0);
    assert_eq!(orbs::screen_half(1920.0, 960.0), 960.0);
    assert_eq!(orbs::screen_half(1280.9, 960.0), 640.0, "truncated, then halved");
    assert_eq!(orbs::screen_half(-5.0, 960.0), 0.0, "fcvtzu saturates a negative word to 0");
}

#[test]
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
    let o = Orb { centre: [1.0, 2.0, 3.0], radius: 0.5, color };
    assert_eq!(orbs::edge_points(&o), [[1.5, 2.0, 3.0], [1.0, 2.5, 3.0]]);
}

#[test]
fn the_layout_carries_one_disc_pane_per_orb() {
    use model::panes as p;
    let specs = p::specs();
    assert_eq!(specs.len(), p::COUNT);
    let container = &specs[p::IDX_ORBS];
    assert_eq!(container.name.as_str(), "TrOrbs");
    assert!(!container.visible);
    for k in 0..p::ORBS {
        let s = &specs[p::IDX_ORB + k];
        assert_eq!(s.parent, container.name, "orb {k}");
        assert!(!s.visible);
        assert!(matches!(s.kind, p::Kind::Rect { color: [255, 255, 255, 255] }));
    }
    const { assert!(p::ORBS >= 2 * 74, "two large fighters' first passes fit") };
}
