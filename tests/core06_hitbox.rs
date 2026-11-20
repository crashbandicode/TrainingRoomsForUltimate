//! SPEC-core-06, the hitbox visualizer: every model rule of the spec's §9 table by its planned name, the site rows against the
//! owned 13.0.5 ELF, and the facts the plugin relies on for the `AttackData` layout and the Lua stack, each shown by the
//! instruction that does the access in the game's own code (`docs/test-provenance/core-06.md` names the derivation).

mod common;

// rc.9 test cut: the call-graph scanner's own tests run once, in `tests/hook_locks.rs`.
macro_rules! callgraph_self_tests {
    ($($body:tt)*) => {};
}
#[path = "common/callgraph.rs"]
mod callgraph;

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::capabilities;
use model::guard::SeamKind;
use model::hitbox::{self, Cell, Indices, StatusIds, FrameFacts, FramePlan, SleepBook, TAG_FLOAT, TAG_INT};
use model::sites::Feature;
use model::sites_hitbox::{self, SITES};

fn f(v: f32) -> Cell {
    Cell { raw: u64::from(v.to_bits()), tag: TAG_FLOAT }
}

fn int(v: i64) -> Cell {
    Cell { raw: v as u64, tag: TAG_INT }
}

// ---- colour table, sphere count and spacing --------------------------------------------------------------------------

#[test]
fn core06_colour_table() {
    // A hitbox without a weaker strength classification starts red for every ID.
    for id in [i32::MIN, -9, -1, 0, 1, 7, 8, 15, 255, i32::MAX] {
        let tint = hitbox::hitbox_tint(id);
        assert_eq!(tint, [1.0, 0.0, 0.0], "id {id}");
        assert!(tint.contains(&1.0), "fully saturated: {tint:?}");
    }
    assert_eq!(hitbox::HURTBOX_TINT, [0.6; 3]);
}

#[test]
fn every_grab_box_is_purple_distinct_from_hits_and_hurtboxes() {
    assert_eq!(hitbox::GRAB_TINT, [0.6, 0.2, 1.0]);
    for tint in [hitbox::STRONG_TINT, hitbox::WEAK_TINT, hitbox::HURTBOX_TINT] {
        assert_ne!(tint, hitbox::GRAB_TINT, "purple identifies grabs separately: {tint:?}");
    }
}

// Host-only perceptual regression: sRGB decoding, XYZ D65 and CIELab D65.
// D65 reference white is [95.047, 100, 108.883]; no chromatic adaptation is applied.
fn srgb_lab_d65(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb.map(|v| {
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    });
    let xyz = [
        (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047,
        0.2126729 * r + 0.7151522 * g + 0.0721750 * b,
        (0.0193339 * r + 0.1191920 * g + 0.9503041 * b) / 1.08883,
    ];
    let [x, y, z] = xyz.map(|v| {
        let delta: f64 = 6.0 / 29.0;
        if v > delta.powi(3) { v.cbrt() } else { v / (3.0 * delta.powi(2)) + 4.0 / 29.0 }
    });
    [116.0 * y - 16.0, 500.0 * (x - y), 200.0 * (y - z)]
}

// CIEDE2000, kL=kC=kH=1; Sharma, Wu and Dalal (2005), equations 2-22.
// Primary source and validation data:
// https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/
fn ciede2000(first: [f64; 3], second: [f64; 3]) -> f64 {
    let [l1, a1, b1] = first;
    let [l2, a2, b2] = second;
    let c_bar = (a1.hypot(b1) + a2.hypot(b2)) / 2.0;
    let chroma_ratio = |c: f64| {
        let c7 = c.powi(7);
        (c7 / (c7 + 25.0_f64.powi(7))).sqrt()
    };
    let g = 0.5 * (1.0 - chroma_ratio(c_bar));
    let (a1p, a2p) = ((1.0 + g) * a1, (1.0 + g) * a2);
    let (c1, c2) = (a1p.hypot(b1), a2p.hypot(b2));
    let hue = |a: f64, b: f64| {
        if a == 0.0 && b == 0.0 { 0.0 } else { b.atan2(a).to_degrees().rem_euclid(360.0) }
    };
    let (h1, h2) = (hue(a1p, b1), hue(a2p, b2));
    let delta_h = if c1 * c2 == 0.0 {
        0.0
    } else if (h2 - h1).abs() <= 180.0 {
        h2 - h1
    } else if h2 > h1 {
        h2 - h1 - 360.0
    } else {
        h2 - h1 + 360.0
    };
    let h_bar = if c1 * c2 == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() <= 180.0 {
        (h1 + h2) / 2.0
    } else if h1 + h2 < 360.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        (h1 + h2 - 360.0) / 2.0
    };
    let cos = |degrees: f64| degrees.to_radians().cos();
    let t = 1.0 - 0.17 * cos(h_bar - 30.0) + 0.24 * cos(2.0 * h_bar)
        + 0.32 * cos(3.0 * h_bar + 6.0) - 0.20 * cos(4.0 * h_bar - 63.0);
    let (l_bar, cp_bar) = ((l1 + l2) / 2.0, (c1 + c2) / 2.0);
    let sl = 1.0 + 0.015 * (l_bar - 50.0).powi(2) / (20.0 + (l_bar - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * cp_bar;
    let sh = 1.0 + 0.015 * cp_bar * t;
    let rotation = -2.0 * chroma_ratio(cp_bar)
        * (60.0 * (-((h_bar - 275.0) / 25.0).powi(2)).exp()).to_radians().sin();
    let dl = (l2 - l1) / sl;
    let dc = (c2 - c1) / sc;
    let dh = 2.0 * (c1 * c2).sqrt() * (delta_h / 2.0).to_radians().sin() / sh;
    (dl * dl + dc * dc + dh * dh + rotation * dc * dh).sqrt()
}

#[test]
fn core06_ciede2000_matches_sharma_reference_pairs() {
    // Published supplementary data, including achromatic, hue-wrap and 180-degree boundary cases.
    // Expected differences are published to four decimals; half a last-place unit is 0.00005.
    // https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/dataNprograms/ciede2000testdata.txt
    let pairs = [
        ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
        ([50.0, 3.1571, -77.2803], [50.0, 0.0, -82.7485], 2.8615),
        ([50.0, 2.8361, -74.0200], [50.0, 0.0, -82.7485], 3.4412),
        ([50.0, -1.3802, -84.2814], [50.0, 0.0, -82.7485], 1.0000),
        ([50.0, 0.0, 0.0], [50.0, -1.0, 2.0], 2.3669),
        ([50.0, -1.0, 2.0], [50.0, 0.0, 0.0], 2.3669),
        ([50.0, 2.4900, -0.0010], [50.0, -2.4900, 0.0009], 7.1792),
        ([50.0, 2.4900, -0.0010], [50.0, -2.4900, 0.0010], 7.1792),
        ([50.0, 2.4900, -0.0010], [50.0, -2.4900, 0.0011], 7.2195),
        ([50.0, -0.0010, 2.4900], [50.0, 0.0010, -2.4900], 4.8045),
        ([50.0, 2.5000, 0.0], [73.0, 25.0, -18.0], 27.1492),
        ([2.0776, 0.0795, -1.1350], [0.9033, -0.0636, -0.5514], 0.9082),
    ];
    for (first, second, expected) in pairs {
        for (a, b) in [(first, second), (second, first)] {
            let actual = ciede2000(a, b);
            assert!((actual - expected).abs() <= 0.00005, "{a:?} / {b:?}: {actual:.9} != {expected}");
        }
        assert_eq!(ciede2000(first, first), 0.0);
    }
    // Anchor the conversion as well as the difference formula.
    for (rgb, expected) in [
        ([0.0; 3], [0.0; 3]),
        ([1.0; 3], [100.0, 0.0, 0.0]),
        ([1.0, 0.0, 0.0], [53.2408, 80.0925, 67.2032]),
    ] {
        let actual = srgb_lab_d65(rgb);
        for (component, expected_component) in actual.into_iter().zip(expected) {
            assert!((component - expected_component).abs() <= 0.0001, "sRGB {rgb:?}, Lab {actual:?}");
        }
    }
}

#[test]
fn core06_palette_is_perceptually_distinct() {
    use model::{hitbox_orbs as orbs, legend};
    let palette = [
        ("strong", hitbox::STRONG_TINT),
        ("weak", hitbox::WEAK_TINT),
        ("grab", legend::GRAB),
        ("hurt", legend::HURT[0]),
        ("invincible", legend::HURT[1]),
        ("intangible", legend::HURT[2]),
        ("windbox", legend::WIND),
        ("wind area", legend::WIND_AREA),
        ("direction", std::array::from_fn(|i| f32::from(legend::DIRECTION_COLOUR[i]) / 255.0)),
    ];
    // Primary-approved replacement for the inherited RGB-distance >=1.0 criterion.
    // All 36 pairs must meet the fixed perceptual threshold; report every failing pair.
    let displayed_lab = |tint| {
        let rgba = orbs::tint_colour(tint, orbs::ORB_ALPHA);
        srgb_lab_d65(std::array::from_fn(|i| f64::from(rgba[i]) / 255.0))
    };
    let mut failures = Vec::new();
    let mut compared = 0;
    for (i, &(left_name, left)) in palette.iter().enumerate() {
        for &(right_name, right) in &palette[i + 1..] {
            let difference = ciede2000(displayed_lab(left), displayed_lab(right));
            compared += 1;
            if !difference.is_finite() || difference < 20.0 {
                failures.push(format!("{left_name} / {right_name}: DeltaE2000={difference:.9}, required >=20"));
            }
        }
    }
    assert_eq!(compared, 36);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn core06_capsule_is_two_ends() {
    // Decision HB-PILLS (2026-11-14): a shape with a second centre is drawn as one pill, so it is held as its two end
    // spheres, sphere 0 at its centre and sphere 1 at its second centre; a point hitbox is one sphere at its centre. This
    // retires the bead rule (`ceil(d / (size x 1.75)) + 1` spheres, 2..16).
    let tint = hitbox::hitbox_tint(0);
    let a = [1.0, 2.0, 3.0];
    for (b, want) in [(Some([11.0, 2.0, 3.0]), vec![a, [11.0, 2.0, 3.0]]), (None, vec![a])] {
        let shape = hitbox::Shape { id: 0, joint: 9, size: 2.0, a, b };
        let mut out = [hitbox::Sphere::NONE; 4];
        let mut len = 0;
        assert_eq!(hitbox::push_shape(&mut out, &mut len, hitbox::KIND_HIT, 0, &shape, tint), 0);
        assert_eq!(out[..len].iter().map(|s| s.pos).collect::<Vec<_>>(), want);
        for (i, s) in out[..len].iter().enumerate() {
            assert_eq!(s.key, hitbox::sphere_key(hitbox::KIND_HIT, 0, i));
        }
    }
    // However long, a capsule is two spheres (the beads stopped at 16).
    let long = hitbox::Shape { id: 0, joint: 9, size: 1.0, a: [0.0; 3], b: Some([500.0, 0.0, 0.0]) };
    let mut out = [hitbox::Sphere::NONE; 4];
    let mut len = 0;
    hitbox::push_shape(&mut out, &mut len, hitbox::KIND_HIT, 0, &long, tint);
    assert_eq!(len, hitbox::SHAPE_SPHERES);
    assert_eq!(hitbox::second_centre([0.0; 3]), None);
    assert_eq!(hitbox::second_centre([0.0, 0.0, 0.5]), Some([0.0, 0.0, 0.5]));
    // The sphere scale is size * 19/200.
    assert!((hitbox::sphere_scale(4.0) - 0.38).abs() < 1e-6);
}

// ---- the commands' parameters ----------------------------------------------------------------------------------------

/// A 36-parameter ATTACK stack (index 1 first) the way the animation script pushes it.
fn attack_stack(second: bool) -> Vec<Cell> {
    let mut s = vec![Cell::NIL; 37];
    s[1] = int(5); // id
    s[2] = int(0); // part
    s[3] = int(0x0000_0007_1c8b_f1a3); // joint hash
    for (i, v) in [(4, 12.0), (5, 361.0), (6, 100.0), (7, 0.0), (8, 40.0)] {
        s[i] = f(v); // damage, angle, kbg, fkb, bkb: unused
    }
    s[9] = f(3.25);
    s[10] = f(1.5);
    s[11] = f(-2.0);
    s[12] = f(0.75);
    if second {
        s[13] = f(4.0);
        s[14] = int(6); // an integer parameter converts like the game's scvtf
        s[15] = f(-1.0);
    }
    s[20] = f(0.0); // shield damage: the shield feature's, not read here
    s
}

#[test]
fn core06_command_param_indices() {
    // ATTACK: 1 id, 3 joint, 9 size, 10..12 centre, 13..15 second centre.
    assert_eq!(hitbox::ATTACK, Indices { id: 1, joint: 3, size: 9, centre: 10, second: 13 });
    let stack = attack_stack(false);
    let shape = hitbox::parse(hitbox::ATTACK, |i| stack.get(i).copied().unwrap_or(Cell::NIL)).unwrap();
    assert_eq!((shape.id, shape.joint, shape.size, shape.a, shape.b), (5, 0x0000_0007_1c8b_f1a3, 3.25, [1.5, -2.0, 0.75], None));
    // The damage group (4..8) and everything past 15 do not matter.
    let mut other = stack.clone();
    other[4] = f(99.0);
    other[20] = f(-999.0);
    assert_eq!(hitbox::parse(hitbox::ATTACK, |i| other.get(i).copied().unwrap_or(Cell::NIL)), Some(shape));
    // An extended hitbox: floats or integers at 13..15.
    let stack = attack_stack(true);
    let shape = hitbox::parse(hitbox::ATTACK, |i| stack.get(i).copied().unwrap_or(Cell::NIL)).unwrap();
    assert_eq!(shape.b, Some([4.0, 6.0, -1.0]));
    // Arguments past the top of the stack read as nil, i.e. no second centre.
    let short: Vec<Cell> = attack_stack(false).into_iter().take(13).collect();
    let shape = hitbox::parse(hitbox::ATTACK, |i| short.get(i).copied().unwrap_or(Cell::NIL)).unwrap();
    assert_eq!(shape.b, None);
    // A joint that is not an integer cannot be placed on a bone: not drawn.
    let mut bad = attack_stack(false);
    bad[3] = f(1.0);
    assert_eq!(hitbox::parse(hitbox::ATTACK, |i| bad.get(i).copied().unwrap_or(Cell::NIL)), None);
    bad[3] = Cell::NIL;
    assert_eq!(hitbox::parse(hitbox::ATTACK, |i| bad.get(i).copied().unwrap_or(Cell::NIL)), None);

    // CATCH: 1 id, 2 joint, 3 size, 4..6 centre, 7..9 second centre.
    assert_eq!(hitbox::CATCH, Indices { id: 1, joint: 2, size: 3, centre: 4, second: 7 });
    let mut grab = [Cell::NIL; 10];
    grab[1] = int(2);
    grab[2] = int(0x0000_0004_aaaa_bbbb);
    grab[3] = f(2.0);
    for (i, v) in [(4, 0.0), (5, 3.0), (6, 1.0), (7, 0.0), (8, 9.0), (9, 1.0)] {
        grab[i] = f(v);
    }
    let shape = hitbox::parse(hitbox::CATCH, |i| grab.get(i).copied().unwrap_or(Cell::NIL)).unwrap();
    assert_eq!((shape.id, shape.joint, shape.size, shape.a, shape.b), (2, 0x0000_0004_aaaa_bbbb, 2.0, [0.0, 3.0, 1.0], Some([0.0, 9.0, 1.0])));

    // The stack slot rule: argument i is at base + 0x10 * i, nil at or past the top.
    let load = |slot: usize| int(slot as i64);
    assert_eq!(hitbox::stack_cell(0x1000, 0x1100, 3, load), int(0x1030));
    assert_eq!(hitbox::stack_cell(0x1000, 0x1100, 15, load), int(0x10f0));
    assert_eq!(hitbox::stack_cell(0x1000, 0x1100, 16, load), Cell::NIL, "the slot at the top is not below it");
    assert_eq!(hitbox::stack_cell(0x1000, 0x1100, 17, load), Cell::NIL);
    // Number reads: float, integer (converted), anything else 0; integer reads floor a float.
    assert_eq!(f(1.5).float(), 1.5);
    assert_eq!(int(7).float(), 7.0);
    assert_eq!(Cell { raw: 1, tag: 1 }.float(), 0.0);
    assert_eq!(f(2.9).int(), 2);
    assert_eq!(Cell::NIL.int(), 0);
}

// ---- the per-frame pass -------------------------------------------------------------------------------------------

fn ids() -> StatusIds {
    // Ordered as the game orders them: GUARD_ON..=GUARD_DAMAGE, DOWN, then CATCH.. CATCH_TURN .. THROW/CAPTURE .. TREAD_FALL.
    StatusIds { guard_on: 30, guard_damage: 33, down: 40, catch: 62, catch_turn: 70, tread_fall: 80 }
}

fn facts(visualizes: bool, enabled: bool, slept: bool, status: i32) -> FrameFacts {
    FrameFacts { visualizes, enabled, slept, status, ids: ids() }
}

#[test]
fn core06_frame_rules_table() {
    let ids = ids();
    // (status, restore, sleep, redraw) with the setting On, in Training.
    let on = [
        (0, true, true, true),   // an ordinary status: restore, sleep, kill and draw
        (29, true, true, true),
        (30, true, false, false), // GUARD_ON: shielding stops the pass after the restore
        (31, true, false, false),
        (33, true, false, false), // GUARD_DAMAGE
        (34, true, true, true),
        (40, true, false, true), // DOWN: effects stay awake, spheres still drawn
        (62, true, false, false), // CATCH: grab range stops after the restore
        (66, true, false, false),
        (70, true, false, false), // CATCH_TURN
        (71, true, false, true), // past the grab range, up to TREAD_FALL: awake, drawn
        (79, true, false, true),
        (80, true, false, true), // TREAD_FALL
        (81, true, true, true),
    ];
    for (status, restore, sleep, redraw) in on {
        let p = hitbox::plan(facts(true, true, false, status));
        assert_eq!(p, FramePlan { restore, sleep, redraw }, "status {status}");
    }
    // The plan equals the spec's step order written out independently, for every status and every state.
    for status in -2..200 {
        for slept in [false, true] {
            let mut calls = Vec::new();
            // (1) set_sleep_effect(false) at the start of every pass (while On); (2) Off: stop.
            let p = hitbox::plan(facts(true, true, slept, status));
            calls.push("restore");
            let stop = (ids.catch..=ids.catch_turn).contains(&status) || (ids.guard_on..=ids.guard_damage).contains(&status);
            if !stop {
                if !((ids.catch..=ids.tread_fall).contains(&status) || status == ids.down) {
                    calls.push("sleep");
                }
                calls.push("redraw");
            }
            let mut got = vec!["restore"];
            if p.sleep {
                got.push("sleep");
            }
            if p.redraw {
                got.push("redraw");
            }
            assert!(p.restore);
            assert_eq!(got, calls, "status {status}");
        }
    }
    // Off: one restore when the plugin had put the fighter to sleep, otherwise no call at all (Q4).
    assert_eq!(hitbox::plan(facts(true, false, true, 0)), FramePlan { restore: true, sleep: false, redraw: false });
    assert_eq!(hitbox::plan(facts(true, false, false, 0)), FramePlan::NONE);
    assert!(!FramePlan::NONE.calls());
}

#[test]
fn core06_rebound_rule() {
    // Rebound cleared: spheres killed unless shielding; a set rebound, the Off setting and other scenes leave them.
    assert!(hitbox::kill_on_rebound(false, true, true, false));
    assert!(!hitbox::kill_on_rebound(false, true, true, true), "shielding keeps the shield bubble");
    assert!(!hitbox::kill_on_rebound(true, true, true, false), "rebound set, not cleared");
    assert!(!hitbox::kill_on_rebound(false, true, false, false), "setting Off");
    assert!(!hitbox::kill_on_rebound(false, false, true, false), "not practice");
    for rebound in [false, true] {
        for visualizes in [false, true] {
            for enabled in [false, true] {
                for shielding in [false, true] {
                    assert_eq!(hitbox::kill_on_rebound(rebound, visualizes, enabled, shielding), !rebound && visualizes && enabled && !shielding);
                }
            }
        }
    }
}

#[test]
fn core06_training_gate() {
    // A scene that is not practice (online, a hot swap, any other mode): no call, whatever the setting, the fighter or the
    // sleep record: the plugin never touches another scene's effect flag (`077a8e43`'s early return, before step 1).
    for enabled in [false, true] {
        for slept in [false, true] {
            for status in [0, 31, 40, 62, 75] {
                let facts = facts(false, enabled, slept, status);
                let plan = hitbox::plan(facts);
                assert_eq!(plan, FramePlan::NONE);
                assert_eq!(hitbox::slept_after(facts, plan), slept, "the record is left as it was");
            }
        }
    }
    // Which objects a scene visualizes: native Training every object; the waiting room's practice only Player 1 (entry 0) and the
    // practice CPU (package 2's lobby entry), never another member of the room nor an object whose entry is unknown (an article);
    // no practice scene, nothing.
    use model::scene::Scene;
    let lobby_cpu = model::cpu::role::LOBBY_CPU_ENTRY;
    for entry in [None, Some(0), Some(1), Some(2), Some(3), Some(lobby_cpu), Some(7)] {
        assert!(hitbox::visualizes_entry(Some(Scene::Training), entry));
        assert!(!hitbox::visualizes_entry(None, entry));
        assert_eq!(hitbox::visualizes_entry(Some(Scene::Lobby), entry), entry == Some(0) || entry == Some(lobby_cpu), "{entry:?}");
    }
    assert_eq!(hitbox::ENTRY_ID_NAME, "FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID");
    // The command hooks draw only with the setting On in a practice scene.
    for enabled in [false, true] {
        for visualizes in [false, true] {
            assert_eq!(hitbox::draw_on_command(enabled, visualizes), enabled && visualizes);
        }
    }
    // The setting (rc.14): a checklist, nothing ticked by default; the visualizer runs while a ticked item draws.
    let mut snapshot = model::settings::Snapshot::factory();
    assert!(!model::legend::draws(hitbox::shown(&snapshot)));
    snapshot.set("hitbox_vis", &[1, 1, 1, 0, 0, 0]).unwrap();
    assert!(model::legend::draws(hitbox::shown(&snapshot)));
    snapshot.set("hitbox_vis", &[0, 0, 0, 1, 0, 0]).unwrap();
    assert!(!model::legend::draws(hitbox::shown(&snapshot)));
}

/// The waiting room resolves its own entry gate (tr-p23-rc5-1, 2026-11-05: Hitbox Visualization On, a boot whose first
/// practice was the waiting room, 691 orb ticks with nothing drawn). The Lobby scene judges a fighter by its entry, read
/// with the work id `ensure_ready` gets from the constant table. Before the fix only three callers asked: Training's pass
/// (whose scene needs no entry), the ATTACK/CATCH command hook and the rebound hook; the pass asked
/// `!visualizes || !ensure_ready()`, so in the Lobby it never asked (visualizes needed the id first). The Lobby's own
/// path, `visualizes`, must ask itself, through the model's `practice_entry`.
#[test]
fn core06_lobby_resolves_its_entry_gate() {
    let g = callgraph::Graph::scan(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[]);
    let found = g.find("hitbox::visualizes");
    assert_eq!(found.len(), 1, "{found:?}");
    let body = &g.funcs[found[0]].body;
    let names = |ident: &str| body.iter().any(|t| matches!(t, callgraph::Tok::Ident(s) if s == ident));
    assert!(names("ensure_ready"), "visualizes must resolve the entry work id on the waiting room's own path");
    assert!(names("practice_entry"), "visualizes takes the entry through model::hitbox::practice_entry");

    // The model: native Training needs no entry and asks nothing; the waiting room asks for the work id when it does not
    // know it, then reads the fighter's entry; an object that is not a fighter, or no practice scene, asks nothing.
    use model::scene::Scene;
    use std::cell::Cell as Count;
    let lobby_cpu = model::cpu::role::LOBBY_CPU_ENTRY;
    for (scene, fighter, id, entry, want_asked, want) in [
        (Some(Scene::Training), true, Some(7), 0, false, None),
        (None, true, Some(7), 0, false, None),
        (Some(Scene::Lobby), false, Some(7), 0, false, None),
        (Some(Scene::Lobby), true, None, 0, true, None),
        (Some(Scene::Lobby), true, Some(7), 0, true, Some(0)),
        (Some(Scene::Lobby), true, Some(7), lobby_cpu, true, Some(lobby_cpu)),
        (Some(Scene::Lobby), true, Some(7), 3, true, Some(3)),
    ] {
        let (asked, read_with) = (Count::new(false), Count::new(None));
        let got = hitbox::practice_entry(scene, fighter, || { asked.set(true); id }, |work| { read_with.set(Some(work)); entry });
        assert_eq!((asked.get(), got), (want_asked, want), "{scene:?} fighter={fighter} id={id:?}");
        assert_eq!(read_with.get(), if want.is_some() { id } else { None });
        // The first Lobby frame of a boot draws Player 1 and the practice CPU: no earlier Training or attack needed.
        if scene == Some(Scene::Lobby) && fighter && id.is_some() {
            assert_eq!(hitbox::visualizes_entry(scene, got), entry == 0 || entry == lobby_cpu);
        }
    }
}

#[test]
fn core06_off_restores() {
    // Replay the pass over frames with the sleep record the runtime keeps (one entry per fighter accessor).
    let mut book = SleepBook::new();
    let (a, b) = (0x1000usize, 0x2000usize);
    let mut calls: Vec<(usize, &str)> = Vec::new();
    fn pass(book: &mut SleepBook, calls: &mut Vec<(usize, &'static str)>, enabled: bool, key: usize, status: i32) {
        let facts = facts(true, enabled, book.slept(key), status);
        let plan = hitbox::plan(facts);
        if plan.restore {
            calls.push((key, "restore"));
        }
        if plan.sleep {
            calls.push((key, "sleep"));
        }
        book.set(key, hitbox::slept_after(facts, plan));
    }
    // On: two fighters, one in a normal status, one shielding.
    for _ in 0..3 {
        pass(&mut book, &mut calls, true, a, 0);
        pass(&mut book, &mut calls, true, b, 31);
    }
    assert!(book.slept(a) && !book.slept(b));
    calls.clear();
    // The setting goes Off: the next pass restores the sleeping fighter, exactly once; the other makes no call.
    for _ in 0..5 {
        pass(&mut book, &mut calls, false, a, 0);
        pass(&mut book, &mut calls, false, b, 31);
    }
    assert_eq!(calls, vec![(a, "restore")]);
    assert!(!book.slept(a));
    // Back On: it sleeps again, then Off again restores again.
    calls.clear();
    pass(&mut book, &mut calls, true, a, 0);
    pass(&mut book, &mut calls, false, a, 0);
    assert_eq!(calls, vec![(a, "restore"), (a, "sleep"), (a, "restore")]);

    // The record: fixed storage, a restored fighter frees its entry, a full record replaces its oldest.
    let mut book = SleepBook::new();
    for key in 1..=hitbox::BOOK_SLOTS {
        book.set(key * 8, true);
    }
    assert!((1..=hitbox::BOOK_SLOTS).all(|k| book.slept(k * 8)));
    book.set(0x9990, true);
    assert!(book.slept(0x9990) && !book.slept(8), "the oldest entry made room");
    book.set(0x9990, false);
    assert!(!book.slept(0x9990));
    assert!(!book.slept(0), "address 0 is never recorded");
    book.set(0, true);
    assert!(!book.slept(0));
}

// ---- the site rows --------------------------------------------------------------------------------------------------

#[test]
fn core06_site_guards() {
    // The sixteen called natives include indexed article lookup and TeamModule ownership.
    // Fourteen main relied windows cover layout, camera, damage and hitbox reads; eight are optional spacing-grader rows.
    let hooks: Vec<&str> = SITES.iter().filter(|s| matches!(s.kind, SeamKind::Hook { .. })).map(|s| s.name).collect();
    assert_eq!(hooks, sites_hitbox::HOOK_ROWS);
    assert_eq!(SITES.iter().filter(|s| matches!(s.kind, SeamKind::Called)).count(), 16);
    assert_eq!(SITES.iter().filter(|s| matches!(s.kind, SeamKind::Relied)).count(), 22);
    for site in SITES.iter() {
        assert_eq!(site.guard.len(), 64, "{}: a 64-byte guard", site.name);
        assert_eq!(site.feature, Feature::Hitbox);
        assert!(site.guard.iter().any(|b| *b != 0), "{}", site.name);
    }
    // No hook window of ours overlaps another row's, and no other hook's window holds one of ours (the rebound thunk's window holds
    // only its neighbouring thunks), apart from rows that name the very same function.
    for site in SITES.iter() {
        for other in capabilities::all_sites() {
            // Only a write changes what another row's check reads, so two rows that are both only called or relied on may share bytes.
            let writes = |s: &model::sites::Site| matches!(s.kind, SeamKind::Hook { .. });
            if other.name == site.name || other.offset == site.offset || !(writes(site) || writes(other)) {
                continue;
            }
            assert!(!site.overlaps(other), "{} overlaps {}", site.name, other.name);
        }
    }
    // A hook row's window may not hold another hook of ours, so a write at one never changes what another's check reads.
    // Every one of our own rows, rejected alone, blocks the unit and names itself (an own row is a capability).
    for site in SITES.iter() {
        let usable = |row: &model::sites::Site| row.name != site.name;
        if sites_hitbox::GRADER_ROWS.iter().any(|(name, _)| *name == site.name) {
            assert!(capabilities::unit_usable(Feature::Hitbox, &usable), "optional row {} blocked Hitbox", site.name);
        } else {
            assert_eq!(capabilities::blocked_by(Feature::Hitbox, &usable), Some(site.name), "{}", site.name);
        }
    }
    // The natives it shares with other packages are in its closure too: rejecting any of them blocks the feature.
    for shared in capabilities::CROSS_NEEDS.iter().find(|(u, _)| *u == Feature::Hitbox).map(|(_, n)| *n).unwrap_or(&[]) {
        let usable = |row: &model::sites::Site| row.name != *shared;
        assert_eq!(capabilities::blocked_by(Feature::Hitbox, &usable), Some(*shared), "{shared}");
    }
    // And the native-call layer grants only what passed: an unreachable row's function is not granted.
    // A rejected native is not granted, and the others still are (rows with the same bytes on one function fail together).
    let poisoned: Vec<&str> = capabilities::equivalent_rows(capabilities::find("sphere_effect_kill").unwrap()).map(|r| r.name).collect();
    let usable = |row: &model::sites::Site| !poisoned.contains(&row.name);
    let granted = capabilities::Granted::from_usable(&usable);
    assert!(!granted.allows(sites_hitbox::HITBOX_KILL_OFFSET));
    assert!(granted.allows(sites_hitbox::HITBOX_REQ_FOLLOW_OFFSET));
    // The plugin never kills by kind: no row of ours names `EffectModule::kill_kind` (0x2017860), so the shield bubble and every other
    // `sys_shield` user are never touched.
    assert!(SITES.iter().all(|s| s.offset != 0x2017860));
    // The offsets constants name the rows.
    for (offset, name) in [
        (sites_hitbox::HITBOX_ATTACK_OFFSET, "hitbox_attack"),
        (sites_hitbox::HITBOX_CATCH_OFFSET, "hitbox_catch"),
        (sites_hitbox::HITBOX_REBOUND_OFFSET, "hitbox_set_rebound"),
        (sites_hitbox::HITBOX_IS_ATTACK_OFFSET, "attack_is_attack"),
        (sites_hitbox::HITBOX_ATTACK_DATA_OFFSET, "attack_data"),
        (sites_hitbox::HITBOX_POWER_MUL_OFFSET, "attack_power_mul_status"),
        (sites_hitbox::HITBOX_DAMAGE_REACTION_OFFSET, "damage_reaction"),
        (sites_hitbox::DAMAGE_RECORD_OFFSET, "damage_record_layout"),
        (sites_hitbox::DAMAGE_HIT_IDS_OFFSET, "damage_record_hit_ids"),
        (sites_hitbox::DAMAGE_DEALT_OFFSET, "damage_record_dealt"),
        (sites_hitbox::HITBOX_REQ_FOLLOW_OFFSET, "effect_req_follow"),
        (sites_hitbox::HITBOX_KILL_OFFSET, "sphere_effect_kill"),
        (sites_hitbox::HITBOX_IS_EXIST_OFFSET, "sphere_effect_is_exist"),
        (sites_hitbox::HITBOX_RGB_LAST_OFFSET, "effect_set_rgb_partial_last"),
        (sites_hitbox::HITBOX_RATE_LAST_OFFSET, "effect_set_rate_last"),
        (sites_hitbox::HITBOX_SLEEP_EFFECT_OFFSET, "motion_animcmd_set_sleep_effect"),
        (sites_hitbox::HITBOX_ACCESSOR_OFFSET, "sv_battle_object_module_accessor"),
        (sites_hitbox::HITBOX_ARTICLE_FROM_NO_OFFSET, "article_get_from_no"),
        (sites_hitbox::HITBOX_ATTACK_STACK_OFFSET, "hitbox_attack_stack"),
        (sites_hitbox::HITBOX_CATCH_STACK_OFFSET, "hitbox_catch_stack"),
        (sites_hitbox::HITBOX_HURT_MODULE_OFFSET, "hurtbox_module"),
        (sites_hitbox::HITBOX_HURT_GROUPS_OFFSET, "hurtbox_groups"),
        (sites_hitbox::HITBOX_HURT_PART_OFFSET, "hurtbox_part_shape"),
        (sites_hitbox::ORB_JOINT_POSITION_OFFSET, "orb_joint_position"),
        (sites_hitbox::ORB_WORLD_TO_SCREEN_OFFSET, "orb_world_to_screen"),
        (sites_hitbox::ORB_SCREEN_SIZE_OFFSET, "orb_screen_size"),
        (sites_hitbox::AREA_WIND_OFFSET, "hitbox_area_wind"),
        (sites_hitbox::AREA_WIND_ARG10_OFFSET, "hitbox_area_wind_arg10"),
        (sites_hitbox::AREA_WIND_RAD_OFFSET, "hitbox_area_wind_rad"),
        (sites_hitbox::AREA_WIND_RAD_ARG9_OFFSET, "hitbox_area_wind_rad_arg9"),
    ] {
        assert_eq!(model::sites_hitbox::SITES.iter().find(|s| s.name == name).map(|s| s.offset), Some(offset), "{name}");
    }
    for (name, offset) in sites_hitbox::GRADER_ROWS {
        let row = model::sites_hitbox::SITES.iter().find(|s| s.name == name).unwrap();
        assert_eq!(row.offset, offset, "{name}");
        assert!(matches!(row.kind, SeamKind::Relied), "{name}");
    }
}

#[test]
fn core06_rows_match_the_owned_elf() {
    let Some(main) = common::owned_main("core06_rows_match_the_owned_elf") else { return };
    for site in SITES.iter() {
        assert_eq!(main.window(site.offset), site.guard, "{}", site.name);
    }
    // The symbols name what the notes say.
    for (name, symbol) in [
        ("hitbox_attack", "_ZN3app10sv_animcmd6ATTACKEP9lua_State"),
        ("hitbox_catch", "_ZN3app10sv_animcmd5CATCHEP9lua_State"),
        ("hitbox_set_rebound", "_ZN3app8lua_bind28GrabModule__set_rebound_implEPNS_26BattleObjectModuleAccessorEb"),
        ("attack_is_attack", "_ZN3app8lua_bind28AttackModule__is_attack_implEPNS_26BattleObjectModuleAccessorEib"),
        ("attack_data", "_ZN3app8lua_bind30AttackModule__attack_data_implEPNS_26BattleObjectModuleAccessorEib"),
        ("attack_power_mul_status", "_ZN3app8lua_bind35AttackModule__power_mul_status_implEPNS_26BattleObjectModuleAccessorE"),
        ("damage_reaction", "_ZN3app8lua_bind27DamageModule__reaction_implEPNS_26BattleObjectModuleAccessorEi"),
        ("effect_req_follow", "_ZN3app8lua_bind29EffectModule__req_follow_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40ES4_RKNS3_8Vector3fES7_fbjiiiibb"),
        ("sphere_effect_kill", "_ZN3app8lua_bind23EffectModule__kill_implEPNS_26BattleObjectModuleAccessorEjbb"),
        ("sphere_effect_is_exist", "_ZN3app8lua_bind34EffectModule__is_exist_effect_implEPNS_26BattleObjectModuleAccessorEj"),
        ("effect_set_rgb_partial_last", "_ZN3app8lua_bind39EffectModule__set_rgb_partial_last_implEPNS_26BattleObjectModuleAccessorEfff"),
        ("effect_set_rate_last", "_ZN3app8lua_bind32EffectModule__set_rate_last_implEPNS_26BattleObjectModuleAccessorEf"),
        ("motion_animcmd_set_sleep_effect", "_ZN3app8lua_bind42MotionAnimcmdModule__set_sleep_effect_implEPNS_26BattleObjectModuleAccessorEb"),
        ("sv_battle_object_module_accessor", "_ZN3app9sv_system29battle_object_module_accessorEP9lua_State"),
        ("article_get_from_no", "_ZN3app8lua_bind39ArticleModule__get_article_from_no_implEPNS_26BattleObjectModuleAccessorEii"),
        ("stop_set_hit_stop_frame", "_ZN3app8lua_bind35StopModule__set_hit_stop_frame_implEPNS_26BattleObjectModuleAccessorEib"),
        ("camera_limit_range", "_ZN3app6camera22get_camera_limit_rangeEv"),
        ("hitbox_area_wind", "_ZN3app10sv_animcmd13AREA_WIND_2NDEP9lua_State"),
        ("hitbox_area_wind_arg10", "_ZN3app10sv_animcmd19AREA_WIND_2ND_arg10EP9lua_State"),
        ("hitbox_area_wind_rad", "_ZN3app10sv_animcmd17AREA_WIND_2ND_RADEP9lua_State"),
        ("hitbox_area_wind_rad_arg9", "_ZN3app10sv_animcmd22AREA_WIND_2ND_RAD_arg9EP9lua_State"),
    ] {
        let site = SITES.iter().find(|s| s.name == name).unwrap();
        assert_eq!(main.symbol(symbol), Some(site.offset), "{name}");
    }
}

// ---- the layouts, shown by the game's own instructions ---------------------------------------------------------------------

fn word_at(main: &common::OwnedMain, offset: usize) -> u32 {
    let w = main.window(offset);
    u32::from_le_bytes([w[0], w[1], w[2], w[3]])
}

// AArch64 encoders for the few instruction forms the checks name.
const fn ldr_x(rt: u32, rn: u32, off: u32) -> u32 {
    0xf940_0000 | ((off / 8) << 10) | (rn << 5) | rt
}
const fn ldr_w(rt: u32, rn: u32, off: u32) -> u32 {
    0xb940_0000 | ((off / 4) << 10) | (rn << 5) | rt
}
const fn str_x(rt: u32, rn: u32, off: u32) -> u32 {
    0xf900_0000 | ((off / 8) << 10) | (rn << 5) | rt
}
const fn str_w(rt: u32, rn: u32, off: u32) -> u32 {
    0xb900_0000 | ((off / 4) << 10) | (rn << 5) | rt
}
const fn str_s(rt: u32, rn: u32, off: u32) -> u32 {
    0xbd00_0000 | ((off / 4) << 10) | (rn << 5) | rt
}
const fn ldr_s(rt: u32, rn: u32, off: u32) -> u32 {
    0xbd40_0000 | ((off / 4) << 10) | (rn << 5) | rt
}
const fn ldp_s(rt: u32, rt2: u32, rn: u32, off: u32) -> u32 {
    0x2d40_0000 | ((off / 4) << 15) | (rt2 << 10) | (rn << 5) | rt
}
const fn str_q(rt: u32, rn: u32, off: u32) -> u32 {
    0x3d80_0000 | ((off / 16) << 10) | (rn << 5) | rt
}
const fn movz_w(rd: u32, imm: u32) -> u32 {
    0x5280_0000 | (imm << 5) | rd
}
const fn add_x(rd: u32, rn: u32, imm: u32) -> u32 {
    0x9100_0000 | (imm << 10) | (rn << 5) | rd
}

#[test]
fn core06_attack_data_layout_matches_the_owned_elf() {
    let Some(main) = common::owned_main("core06_attack_data_layout_matches_the_owned_elf") else { return };
    // AttackModule's vtable functions (the vtable is at main+0x4f6a5b8 by its relocations; the functions are the lua_bind thunks'
    // targets: `is_attack` slot 0xc8, `set_size` 0x118, `set_node` 0x148, `attack_data` 0x1e8, `set_attack` 0x68).
    let w = |offset: usize| word_at(&main, offset);
    // set_node: `ldr x8,[x0,#0x68]` (the attack array), `mov w9,#0x130` (the stride), ..., `str x2,[x8,#0x48]`: the node at +0x48.
    assert_eq!(w(0x3dd540), ldr_x(8, 0, 0x68));
    assert_eq!(w(0x3dd544), movz_w(9, 0x130));
    assert_eq!(w(0x3dd54c), str_x(2, 8, hitbox::layout::NODE as u32));
    // set_size (slot 0x118): `str s0,[x8,#0x24]`: the size at +0x24.
    assert_eq!(w(0x3dd3b0), ldr_x(8, 0, 0x68));
    assert_eq!(w(0x3dd3b4), movz_w(9, 0x130));
    assert_eq!(w(0x3dd3bc), str_s(0, 8, hitbox::layout::SIZE as u32));
    // attack_data (slot 0x1e8): with the flag clear, `[module+0x68] + slot * 0x130`; with it set, the second array
    // (`[module+0x78] + slot * 0x150 + 0x10`).
    assert_eq!(w(0x3dda88), ldr_x(9, 0, 0x78));
    assert_eq!(w(0x3dda8c), movz_w(10, 0x150));
    assert_eq!(w(0x3dda9c), ldr_x(9, 0, 0x68));
    assert_eq!(w(0x3ddaa0), movz_w(10, 0x130));
    // is_attack (slot 0xc8): the slot's word at +0x108 (active when non-zero), the second array's byte at +0x148.
    assert_eq!(w(0x3dd214), ldr_x(9, 0, 0x68));
    assert_eq!(w(0x3dd220), ldr_w(8, 8, 0x108));
    // get_offset2 (slot 0x140) calls attack_data (vtable slot 0x1e8) and loads the vector 0x10 past it.
    assert_eq!(w(0x3dd520), add_x(8, 0, hitbox::layout::OFFSET2 as u32));
    // set_attack (slot 0x68) copies the argument's offset vector to +0 and its second vector to +0x10 and the node to +0x48.
    assert_eq!(w(0x3dc21c), str_q(0, 19, hitbox::layout::OFFSET as u32));
    assert_eq!(w(0x3dc22c), str_q(0, 27, hitbox::layout::OFFSET2 as u32));
    assert_eq!(w(0x3dc240), str_x(9, 27, hitbox::layout::NODE as u32));
    // The ATTACK command builds that argument on its stack at sp+0x20: the vectors at +0 and +0x10, the size at +0x24, the node
    // at +0x48 (sp+0x68), then calls vtable slot 0x68 with it: `ldr x8,[x8,#0x68]; add x3,sp,#0x20; blr x8`.
    assert_eq!(w(0x2285094), str_q(1, 31, 0x20), "centre at +0");
    assert_eq!(w(0x2285320), str_q(2, 31, 0x30), "second centre at +0x10");
    assert_eq!(w(0x2284e80), str_w(8, 31, 0x20 + hitbox::layout::SIZE as u32), "size at +0x24");
    assert_eq!(w(0x2284ccc), str_x(0, 31, 0x20 + hitbox::layout::NODE as u32), "node at +0x48");
    assert_eq!(w(0x2285ba0), ldr_x(8, 8, 0x68));
    assert_eq!(w(0x2285ba4), add_x(3, 31, 0x20));
    assert_eq!(w(0x2285bb0), 0xd63f_0100, "blr x8");
    // ...and the stride the slots are walked with is the AttackData size the plugin never assumes elsewhere.
}

/// rc.14 spots-2: the smash charge is the `AttackModule`'s status-scoped damage multiplier (module `+0xe0`), read here through the game's own
/// getter. Proven from the owned game, nothing recalled: the getter and setter thunks and their vtable slots, `get_power` multiplying every slot's damage
/// by the field, and the common fighter script `sub_set_attack_s4_power_mul` (the owned `lua2cpp_common.nro`) ending with a call of the setter.
#[test]
fn smash_charge_multiplier_matches_the_owned_game() {
    const MUL: u32 = 0xe0;
    let Some(main) = common::owned_main("smash_charge_multiplier_matches_the_owned_game") else { return };
    let w = |offset: usize| word_at(&main, offset);
    // The lua_bind thunks: `ldr x0,[x0,#0xa0]; ldr x8,[x0]; ldr x1,[x8,#slot]; br x1`.
    for (thunk, slot) in [(0x1fd0540_usize, 0x1f0_u32), (sites_hitbox::HITBOX_POWER_MUL_OFFSET, 0x1f8)] {
        assert_eq!([w(thunk), w(thunk + 4), w(thunk + 8), w(thunk + 12)], [ldr_x(0, 0, 0xa0), ldr_x(8, 0, 0), ldr_x(1, 8, slot), 0xd61f_0020], "thunk {thunk:#x}");
    }
    assert_eq!(main.symbols.iter().find(|(n, _)| n.contains("AttackModule__set_power_mul_status_impl")).map(|(_, a)| *a), Some(0x1fd0540));
    // The setter (slot 0x1f0) stores s0 at `+0xe0`, the getter (slot 0x1f8) loads it, and `get_power` (slot 0x158) multiplies the slot's damage (`+0x20` of its
    // AttackData, loaded into s0) by the module's `+0xf0`, `+0xf4` and `+0xe0`.
    assert_eq!(w(0x3ddab0), str_s(0, 0, MUL), "set_power_mul_status");
    assert_eq!(w(0x3ddac0), ldr_s(0, 0, MUL), "power_mul_status");
    assert_eq!([w(0x3dd5b0), w(0x3dd5b4), w(0x3dd5b8), w(0x3dd5bc), w(0x3dd5c4), w(0x3dd5c8)], [
        ldp_s(1, 2, 19, 0xf0),
        0x1e22_0821,
        ldr_s(2, 19, MUL),
        ldr_s(0, 0, hitbox::layout::POWER as u32),
        0x1e22_0821,
        0x1e21_0800,
    ], "get_power: power x [+0xf0] x [+0xf4] x [+0xe0]");
    // Those are AttackModule's vtable slots (address point main+0x4f6a5b8, by its relocations).
    let reloc = |slot: usize, target: u64| {
        let mut entry = [0u8; 24];
        entry[..8].copy_from_slice(&((0x4f6a5b8 + slot) as u64).to_le_bytes());
        entry[8..16].copy_from_slice(&0x403u64.to_le_bytes());
        entry[16..].copy_from_slice(&target.to_le_bytes());
        let ro = &main.bytes[main.rodata.offset..main.rodata.offset + main.rodata.size];
        (0..ro.len() / 8).any(|i| ro.get(i * 8..i * 8 + 24) == Some(&entry[..]))
    };
    assert!(reloc(0x1f0, 0x3ddab0) && reloc(0x1f8, 0x3ddac0) && reloc(0x158, 0x3dd580), "AttackModule vtable slots");

    // The charge writes it: `L2CFighterCommon::sub_set_attack_s4_power_mul` ends with `bl <plt>` (main's `AttackModule::set_power_mul_status` import).
    let Some(common_nro) = common::owned_common_nro("smash_charge_multiplier_matches_the_owned_game") else { return };
    let setter = common_nro.symbols.iter().find(|(n, _)| n == "_ZN7lua2cpp16L2CFighterCommon27sub_set_attack_s4_power_mulEv").map(|(_, a)| *a);
    assert_eq!(setter, Some(0x8dd90));
    let b = &common_nro.bytes;
    let u32_at = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let u64_at = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap()) as usize;
    let mod0 = u32_at(4) as usize;
    let dynamic = (mod0 as i64 + i64::from(u32_at(mod0 + 4) as i32)) as usize;
    let (mut jmprel, mut pltrelsz, mut symtab, mut strtab) = (0, 0, 0, 0);
    let mut at = dynamic;
    loop {
        let (tag, value) = (u64_at(at), u64_at(at + 8));
        at += 16;
        match tag {
            0 => break,
            2 => pltrelsz = value,
            5 => strtab = value,
            6 => symtab = value,
            23 => jmprel = value,
            _ => {}
        }
    }
    let call = 0x8e1d4_usize;
    let bl = u32_at(call);
    assert_eq!(bl >> 26, 0x25, "bl");
    let stub = (call as i64 + i64::from(((bl & 0x03ff_ffff) << 6) as i32 >> 6) * 4) as usize;
    let (adrp, ldr) = (u32_at(stub), u32_at(stub + 4));
    assert_eq!((adrp & 0x9f00_001f, ldr & 0xffc0_03ff, u32_at(stub + 12)), (0x9000_0010, 0xf940_0211, 0xd61f_0220), "PLT stub");
    let imm21 = ((((adrp >> 5) & 0x7_ffff) << 2) | ((adrp >> 29) & 3)) as i32;
    let slot = (stub as i64 & !0xfff) + i64::from(imm21 << 11 >> 11) * 4096 + i64::from(((ldr >> 10) & 0xfff) * 8);
    let imported = (0..pltrelsz / 24)
        .find_map(|i| {
            let entry = jmprel + i * 24;
            (u64_at(entry) as i64 == slot).then(|| {
                let name_at = strtab + u32_at(symtab + (u64_at(entry + 8) >> 32) * 24) as usize;
                let len = b[name_at..].iter().position(|c| *c == 0).unwrap();
                String::from_utf8_lossy(&b[name_at..name_at + len]).into_owned()
            })
        })
        .expect("a relocation for the stub's GOT slot");
    assert!(imported.contains("AttackModule__set_power_mul_status_impl"), "the script's last call imports {imported}");
}

/// rc.14 spots-3: `DamageModule::reaction(accessor, index)` is the knockback the defender received, as the game's hit handler stores it. Proven
/// from the owned game: the thunk and its vtable slot (the fighter DamageModule's address point main+0x4f86a48, by its relocations), the getter
/// reading `+0x10` of the 0x1c0-byte damage record, and the hit handler (main+0x402cc0) storing the knockback its formula (main+0x401e50)
/// returned there when it is larger than the stored one, with the hit's id block (the attack slot at `+0x32`) beside it.
#[test]
fn damage_reaction_matches_the_owned_game() {
    let Some(main) = common::owned_main("damage_reaction_matches_the_owned_game") else { return };
    let w = |offset: usize| word_at(&main, offset);
    let thunk = sites_hitbox::HITBOX_DAMAGE_REACTION_OFFSET;
    assert_eq!([w(thunk), w(thunk + 4), w(thunk + 8), w(thunk + 12)], [ldr_x(0, 0, 0xa8), ldr_x(8, 0, 0), ldr_x(2, 8, 0xd0), 0xd61f_0040], "the thunk");
    let reloc = |slot: usize, target: u64| {
        let mut entry = [0u8; 24];
        entry[..8].copy_from_slice(&((0x4f86a48 + slot) as u64).to_le_bytes());
        entry[8..16].copy_from_slice(&0x403u64.to_le_bytes());
        entry[16..].copy_from_slice(&target.to_le_bytes());
        let ro = &main.bytes[main.rodata.offset..main.rodata.offset + main.rodata.size];
        (0..ro.len() / 8).any(|i| ro.get(i * 8..i * 8 + 24) == Some(&entry[..]))
    };
    assert!(reloc(0xc8, 0x3ff9f0) && reloc(0xd0, 0x3ffa10), "DamageModule vtable slots: damage, reaction");
    // reaction: `ldr x8,[x0,#0x18]; orr w9,wzr,#0x1c0; smaddl x8,w1,w9,x8; ldr s0,[x8,#0x10]; ret`.
    assert_eq!([w(0x3ffa10), w(0x3ffa14), w(0x3ffa18), w(0x3ffa1c), w(0x3ffa20)], [ldr_x(8, 0, 0x18), 0x321a_0be9, 0x9b29_2028, ldr_s(0, 8, 0x10), 0xd65f_03c0]);
    // The hit handler: `ldrb w26,[x20,#0x32]` (the attack slot id of the hit), the record `madd x24,x23,x8(=0x1c0),x9`, `bl 0x401e50` (the knockback),
    // `ldr s0,[x24,#0x10]!; fcmp s8,s0; b.le` (only a larger one is stored), `ldp x9,x10,[x20,#0x20]; ... stp x10,x11,[x8,#0x48]` (the id block
    // with it: the hit's +0x32 at the record's +0x52), `str s8,[x24]`.
    assert_eq!(w(0x402d00), 0x3940_ca9a, "ldrb w26, [x20, #0x32]");
    assert_eq!([w(0x402e28), w(0x402e2c)], [0x321a_0be8, 0x9b08_26f8], "the record");
    assert_eq!(w(0x402f00), 0x97ff_fbd4, "bl 0x401e50");
    assert_eq!([w(0x402f5c), w(0x402f60), w(0x402f64)], [0xbc41_0f00, 0x1e20_2100, 0x5400_180d], "compare");
    assert_eq!([w(0x402f9c), w(0x402fb0), w(0x402fb4)], [0xa942_2a89, 0xa904_ad0a, str_s(8, 24, 0)], "store");
    // The two Relied rows the plugin reads record 0 by (`sites_hitbox::damage_record`): `ldr x11,[x20,#0x30]; str x9,[x8,#0x40]` put the
    // hit's +0x24 (the attacker's object id) at +0x44 and its +0x32 (the slot) at +0x52; the module is the thunk's `[x0,#0xa8]`.
    use model::sites_hitbox::damage_record as r;
    assert_eq!([w(0x402fa0), w(0x402fac)], [ldr_x(11, 20, 0x30), str_x(9, 8, 0x40)]);
    assert_eq!((r::MODULE, r::RECORDS, r::ATTACKER, r::SLOT), (0xa8, 0x18, 0x40 + (0x24 - 0x20), 0x50 + (0x32 - 0x30)));
    assert_eq!((sites_hitbox::DAMAGE_RECORD_OFFSET, sites_hitbox::DAMAGE_HIT_IDS_OFFSET), (0x3ffa10, 0x402f9c));
    // The third Relied row: the hit's damage transaction (main+0x4026c0) adds the hit's damage into record +0x8 (`ldr s0,[x8,#0x8];
    // fadd s0,s9,s0; str s0,[x8,#0x8]`), and the hit handler takes +0x0 plus +0x8 as the knockback's percent (`ldr s14,[x24,#0x8]` ...
    // `ldr s0,[x24]; fadd s0,s14,s0`).
    let dealt = r::DEALT as u32;
    assert_eq!([w(0x402908), w(0x40290c), w(0x402910)], [ldr_s(0, 8, dealt), 0x1e20_2920, str_s(0, 8, dealt)], "the damage added");
    assert_eq!([w(0x402e44), w(0x402e94), w(0x402e98)], [ldr_s(14, 24, dealt), ldr_s(0, 24, 0), 0x1e20_29c0], "the knockback's percent");
    assert_eq!(sites_hitbox::DAMAGE_DEALT_OFFSET, 0x4028e0);
}

/// rc.14 hb-legend: the launch angle field of `AttackData`, and the `HitStatus` values with the
/// game's own effective status of a hurtbox part (`model::legend`).
#[test]
fn legend_facts_match_the_owned_elf() {
    use model::legend::{self, ANGLE, GROUP_GLOBAL, GROUP_WHOLE};
    let Some(main) = common::owned_main("legend_facts_match_the_owned_elf") else { return };
    let w = |offset: usize| word_at(&main, offset);
    // ATTACK (its AttackData at sp+0x20): arg 4, the damage, to +0x20; arg 5, the angle, to +0x28.
    assert_eq!(w(0x2284d90), str_w(8, 31, 0x20 + hitbox::layout::POWER as u32), "damage");
    assert_eq!(w(0x2284d98), str_w(0, 31, 0x20 + ANGLE as u32), "angle");
    // The game reads the angle back from `attack_data(slot)` and compares it with 361: `ldr w8,[x0,#0x28]; cmp w8,#0x169`.
    assert_eq!(w(0x10d6db0), ldr_w(8, 0, ANGLE as u32));
    assert_eq!(w(0x10d6dc4), 0x7105_a51f, "cmp w8, #361");
    // HitModule's vtable (address point main+0x4f6e1d0 by its relocations): slot 0x108 set_xlu_global main+0x46a030, slot
    // 0x148 get_status main+0x46a800 (the lua_bind thunks load those slots).
    let reloc = |slot: usize, target: u64| {
        let mut entry = [0u8; 24];
        entry[..8].copy_from_slice(&((0x4f6e1d0 + slot) as u64).to_le_bytes());
        entry[8..16].copy_from_slice(&0x403u64.to_le_bytes());
        entry[16..].copy_from_slice(&target.to_le_bytes());
        let ro = &main.bytes[main.rodata.offset..main.rodata.offset + main.rodata.size];
        (0..ro.len() / 8).any(|i| ro.get(i * 8..i * 8 + 24) == Some(&entry[..]))
    };
    assert!(reloc(0x108, 0x46a030) && reloc(0x148, 0x46a800), "HitModule vtable slots");
    assert_eq!(w(0x203db68), ldr_x(2, 8, 0x108), "HitModule::set_xlu_global thunk");
    assert_eq!(w(0x203dbb8), ldr_x(3, 8, 0x148), "HitModule::get_status thunk");
    // set_xlu_global writes 2 to the group's global status; cancel_xlu_global writes 2 (xlu frames left) or 1 (invincible).
    assert_eq!((w(0x46a05c), w(0x46a060)), (0x321f_03e9, str_w(9, 8, GROUP_GLOBAL as u32)), "orr w9,wzr,#2");
    assert_eq!((w(0x46a168), w(0x46a180), w(0x46a1a4)), (0x321f_03ea, 0x3200_03ea, str_w(10, 8, GROUP_GLOBAL as u32)));
    assert_eq!((legend::INTANGIBLE, legend::INVINCIBLE), (2, 1));
    // get_status: `ldp w10,w8,[x8,#0x1c]` (whole, global), `bic w10,w10,w10,asr #31` (whole at least 0), then two signed
    // maxima with the part's word (`csel ... gt`): `legend::part_status`.
    assert_eq!(GROUP_GLOBAL, GROUP_WHOLE + 4);
    assert_eq!(w(0x46a824), 0x2943_a10a, "ldp w10, w8, [x8, #0x1c]");
    assert_eq!(w(0x46a828), 0x0aaa_7d4a, "bic w10, w10, w10, asr #31");
    assert_eq!((w(0x46a830), w(0x46a838)), (0x1a8a_c108, 0x1a88_c120), "csel gt, csel gt");
    // Windboxes: ATTACK takes argument 26 (`add x10, x10, #0x1a0`: the 26th value past the stack's base) and stores it as a byte at
    // `+0x70` (`strb w11, [sp, #0x90]`, the AttackData at sp+0x20); argument 27 goes to `+0x6e` (`strb w11, [sp, #0x8e]`). The knockback
    // words: growth, fixed and base at `+0x2c`, `+0x30`, `+0x34` (`str w0, [sp, #0x4c/0x50/0x54]`), the halves `legend::KNOCKBACK_QUADS` reads.
    assert_eq!(legend::NO_REACTION, 0x70);
    assert_eq!(w(0x22857fc) & 0xffff_fc00, 0x9106_8000, "add xN, xN, #0x1a0");
    assert_eq!((w(0x2285880), w(0x2285910)), (0x3902_43eb, 0x3902_3beb), "strb w11, [sp, #0x90]; strb w11, [sp, #0x8e]");
    assert_eq!([0x2284dac, 0x2284dc0, 0x2284dd4].map(w), [0xb900_4fe0, 0xb900_53e0, 0xb900_57e0], "str w0, [sp, #0x4c / #0x50 / #0x54]");
    assert_eq!(legend::KNOCKBACK_QUADS, [0x28, 0x30]);
    // The defender's DamageModule hit handler (main+0x402cc0) reads it (`ldrb w8, [x22, #0x70]`, x22 the attacker's AttackData).
    assert_eq!([0x402e04, 0x402f70, 0x4032fc].map(w), [0x3941_c2c8; 3]);
    // Wind areas (`legend::area`). The lua_bind thunks take the AreaModule at accessor +0xc0 and call add_wind_area_2nd / _rad
    // (slots 0x158 / 0x160); FighterAreaModuleImpl's vtable (address point main+0x4f85690, by its relocations) holds 0x3d03e0 / 0x3d0680.
    use model::legend::area;
    assert_eq!(area::MODULE, 0xc0);
    assert_eq!([0x1fcc210, 0x1fcc218, 0x1fcc228].map(w), [0xf940_6000, 0xf940_ad05, 0xf940_b104], "ldr x0,[x0,#0xc0]; ldr x5,[x8,#0x158]; ldr x4,[x8,#0x160]");
    assert!(reloc(0x4f85690 - 0x4f6e1d0 + 0x158, 0x3d03e0) && reloc(0x4f85690 - 0x4f6e1d0 + 0x160, 0x3d0680), "AreaModule wind slots");
    // add_wind_area_2nd: the table `[this+0x18]`, its records `+0x18`, 0x70 each; the shape type byte `+0x60`; the shape object
    // `+0x28`; a box's corners stored at shape `+0x40` / `+0x50` (`stp q2, q1`), a circle's centre `+0x80` and radius `+0x90`.
    assert_eq!((area::TABLE, area::RECORDS, area::STRIDE, area::SHAPE_TYPE, area::SHAPE), (0x18, 0x18, 0x70, 0x60, 0x28));
    assert_eq!([0x3d0440, 0x3d0444, 0x3d0448, 0x3d045c, 0x3d04b0].map(w), [0xf940_0e88, 0xf940_0d08, 0x321c_0be9, 0x3941_8129, 0xf940_1509]);
    assert_eq!((area::BOX_MIN, area::BOX_MAX, area::CENTRE, area::RADIUS), (0x40, 0x50, 0x80, 0x90));
    assert_eq!([0x3d04c8, 0x3d0534, 0x3d0530].map(w), [0xad02_0522, 0x3d80_2123, 0xbd00_9121], "stp q2,q1,[x9,#0x40]; str q3,[x9,#0x80]; str s1,[x9,#0x90]");
    // is_enable_area: the count `+0x10` and the enabled byte `+0x61`; the wind manager's registration: the record's kind byte `+0x1`
    // must be 0x19, the wind-set byte `+0x63`, the shape's id `+0x70`, the manager behind main+0x52b85e8.
    assert_eq!((area::COUNT, area::ENABLED, area::KIND, area::WIND, area::WIND_SET, area::ID), (0x10, 0x61, 0x1, 0x19, 0x63, 0x70));
    assert_eq!([0x3cf798, 0x3cf7b4, 0x3cfee0, 0x3cfee4, 0x3cfefc, 0x3cffcc].map(w), [0xf940_090a, 0x3941_8508, 0x3940_0529, 0x7100_653f, 0x3846_3f68, 0xb940_7115]);
    let (adrp, ldr) = (w(0x3cfff4), w(0x3cfff8));
    let page = (0x3cfff4usize & !0xfff) + ((((adrp >> 5) & 0x7ffff) << 2 | (adrp >> 29) & 3) as usize) * 0x1000;
    assert_eq!(page + ((ldr >> 10) & 0xfff) as usize * 8, area::MANAGER_SLOT, "adrp x8 / ldr x8,[x8,#0x5e8]");
    assert_eq!([0x3cfffc, 0x3d0000, 0x3d000c].map(w), [0xf940_0116, 0xa940_22d7, 0xb940_02e9], "ldr x22,[x8]; ldp x23,x8,[x22]; ldr w9,[x23]");
    // The entry: 0x28 bytes, the id first, then the parameters from +4 (`strh`/`stur ... #0x6`); the angle the command stored at
    // parameter +8 (`str s0,[sp,#0x20]`, the parameters at sp+0x18) lands at entry +0xc.
    assert_eq!((area::ENTRY, area::ENTRY_ANGLE), (0x28, 0xc));
    assert_eq!([0x3d0558, 0x3d0638, 0x3d0060].map(w), [0x7900_33e8, 0xbd00_23e0, 0xf800_6128], "strh w8,[sp,#0x18]; str s0,[sp,#0x20]; stur x8,[x9,#0x6]");
}

/// rc.14 wind-articles: the four wind-area commands the plugin hooks. Each reads the running object's accessor from the Lua state
/// (`ldur x8,[x0,#-8]; ldr x8,[x8,#0x1a0]`), takes its AreaModule (`ldr x0,[x8,#0xc0]`, `legend::area::MODULE`) and calls a worker that
/// adds the record through the module's slot 0x158 (`add_wind_area_2nd`, boxes) or 0x160 (`_rad`, circles). Nothing in them depends on
/// the object being a fighter, so an article's command writes its own AreaModule.
#[test]
fn core06_wind_area_commands_match_the_owned_elf() {
    use model::legend::area;
    let Some(main) = common::owned_main("core06_wind_area_commands_match_the_owned_elf") else { return };
    let w = |offset: usize| word_at(&main, offset);
    let bl_target = |at: usize| {
        let word = w(at);
        assert_eq!(word & 0xfc00_0000, 0x9400_0000, "bl at {at:#x}");
        (at as i64 + ((((word & 0x03ff_ffff) << 6) as i32 >> 6) as i64) * 4) as usize
    };
    for (offset, load, call, worker, slot) in [
        (sites_hitbox::AREA_WIND_OFFSET, 0x44, 0x4c, 0x22a_8fd0, 0x158),
        (sites_hitbox::AREA_WIND_ARG10_OFFSET, 0x44, 0x4c, 0x22a_8fd0, 0x158),
        (sites_hitbox::AREA_WIND_RAD_OFFSET, 0x44, 0x4c, 0x22a_86a0, 0x160),
        (sites_hitbox::AREA_WIND_RAD_ARG9_OFFSET, 0x44, 0x4c, 0x22a_86a0, 0x160),
    ] {
        assert_eq!([w(offset + 0x10), w(offset + 0x14)], [0xf85f_8008, ldr_x(8, 8, 0x1a0)], "{offset:#x}: ldur x8,[x0,#-8]; ldr x8,[x8,#0x1a0]");
        assert_eq!(w(offset + load), ldr_x(0, 8, area::MODULE as u32), "{offset:#x}: ldr x0,[x8,#0xc0]");
        assert_eq!(bl_target(offset + call), worker, "{offset:#x}: the worker");
        // The worker reaches the module's wind slot (`ldr x8,[x8,#slot]` before its `blr x8`).
        let reaches = (worker..worker + 0x1000).step_by(4).any(|at| w(at) == ldr_x(8, 8, slot));
        assert!(reaches, "{worker:#x} loads slot {slot:#x}");
    }
}

/// rc.14 wind-articles: `legend::wind_areas` reads one AreaModule table for every object. The two functions that write it
/// (`add_wind_area_2nd` main+0x3d03e0 and `_rad` main+0x3d0680, whose stores `legend_facts_match_the_owned_elf` checks) sit at
/// slots 0x158 / 0x160 of four AreaModule vtables, by their relocations: the fighter's (address point main+0x4f85690) and three
/// more, so an article's or an item's module keeps the same records.
#[test]
fn legend_every_area_module_writes_the_table_the_reader_reads() {
    let Some(main) = common::owned_main("legend_every_area_module_writes_the_table_the_reader_reads") else { return };
    let reloc = |at: usize, target: u64| {
        let mut entry = [0u8; 24];
        entry[..8].copy_from_slice(&(at as u64).to_le_bytes());
        entry[8..16].copy_from_slice(&0x403u64.to_le_bytes());
        entry[16..].copy_from_slice(&target.to_le_bytes());
        let ro = &main.bytes[main.rodata.offset..main.rodata.offset + main.rodata.size];
        (0..ro.len() / 8).any(|i| ro.get(i * 8..i * 8 + 24) == Some(&entry[..]))
    };
    for vtable in [0x4f6_9ec0usize, 0x4f8_5690, 0x507_6470, 0x517_9668] {
        assert!(reloc(vtable + 0x158, 0x3d03e0) && reloc(vtable + 0x160, 0x3d0680), "vtable {vtable:#x}: add_wind_area_2nd / _rad");
    }
}

/// Eggs-ercise V2.2: the stage's camera range is the camera manager's four floats (left, right, top, bottom) at `+0xc20`, read
/// the way the game's own accessor reads them: the slot `main+0x52b8f00` (the one the camera setup hook reads), the pointer in
/// it, the manager that points at, then the floats.
#[test]
fn core06_camera_limit_range_is_the_manager_field() {
    let Some(main) = common::owned_main("core06_camera_limit_range_is_the_manager_field") else { return };
    let (at, field) = (sites_hitbox::CAMERA_LIMIT_RANGE_OFFSET, sites_hitbox::CAMERA_LIMIT_FIELD as u32);
    let w = |offset: usize| word_at(&main, offset);
    let (adrp, ldr) = (w(at), w(at + 4));
    assert_eq!(adrp & 0x9f00_001f, 0x9000_0008, "adrp x8");
    let page = (at & !0xfff) + ((((adrp >> 5) & 0x7ffff) << 2 | (adrp >> 29) & 3) as usize) * 0x1000;
    assert_eq!(ldr & 0xffc0_03ff, ldr_x(8, 8, 0) & 0xffc0_03ff, "ldr x8,[x8,#slot]");
    assert_eq!(page + ((ldr >> 10) & 0xfff) as usize * 8, model::sites_pkg2::CAMERA_MANAGER_SLOT_OFFSET);
    assert_eq!(w(at + 8), ldr_x(8, 8, 0), "the manager");
    assert_eq!(w(at + 12), 0xbd40_0000 | (field / 4) << 10 | 8 << 5, "ldr s0,[x8,#0xc20]");
    assert_eq!([w(at + 16), w(at + 20), w(at + 28)], [add_x(9, 8, field + 4), add_x(10, 8, field + 8), add_x(8, 8, field + 12)]);
}

#[test]
fn core06_lua_stack_layout_matches_the_owned_elf() {
    let Some(main) = common::owned_main("core06_lua_stack_layout_matches_the_owned_elf") else { return };
    let w = |offset: usize| word_at(&main, offset);
    let l = hitbox::layout::LUA_FRAME as u32;
    let top = hitbox::layout::LUA_TOP as u32;
    let tag = hitbox::layout::TAG_FIELD as u32;
    // ATTACK, argument 4 (`+0x40` = 0x10 * 4): frame pointer at L+0x20 then its first word, top at L+0x10, the slot tested
    // against the top (`cmp x8,x9; csel`), the nil value otherwise, then the tag at +8 compared with 3 (float) and 0x13 (integer).
    let base = model::sites_hitbox::HITBOX_ATTACK_STACK_OFFSET;
    assert_eq!(w(base), ldr_x(8, 22, l));
    assert_eq!(w(base + 4), ldr_x(8, 8, 0));
    assert_eq!(w(base + 8), ldr_x(9, 22, top));
    assert_eq!(w(base + 12), add_x(8, 8, 0x10 * 4));
    assert_eq!(w(base + 0x20), ldr_w(8, 27, tag));
    assert_eq!(w(base + 0x28), 0x7100_0d1f, "cmp w8,#3");
    // The stack read is the same for every argument: base + 0x10 * index (9 = 0x90 size, 10..12 = 0xa0..0xc0 centre, 13..15 =
    // 0xd0..0xf0 second centre), each preceded by the same two loads.
    assert_eq!(w(0x2284dd8), ldr_x(8, 22, l));
    assert_eq!(w(0x2284de4), add_x(8, 8, 0x10 * 9), "size is argument 9");
    assert_eq!(w(0x2284e90), add_x(8, 8, 0x10 * 10), "x is argument 10");
    assert_eq!(w(0x2284f34), add_x(8, 8, 0x10 * 11));
    assert_eq!(w(0x2284fe0), add_x(8, 8, 0x10 * 12));
    assert_eq!(w(0x2285110), add_x(8, 8, 0x10 * 13), "second centre x is argument 13");
    assert_eq!(w(0x22851c0), add_x(8, 8, 0x10 * 14));
    assert_eq!(w(0x2285270), add_x(8, 8, 0x10 * 15));
    // A float is read as a float (tag 3) and an integer converted with scvtf (tag 0x13): `scvtf s0,x8`.
    assert_eq!(w(0x2284e0c + 4), 0x9e22_0100, "scvtf s0, x8 after the tag 0x13 test");
    assert_eq!(w(0x2284e04), 0x7100_4d1f, "cmp w8,#0x13");
    // CATCH: argument 3 (size) at the same shape: base + 0x30.
    let catch = model::sites_hitbox::HITBOX_CATCH_STACK_OFFSET;
    assert_eq!(w(catch), ldr_x(8, 19, l));
    assert_eq!(w(catch + 4), ldr_x(8, 8, 0));
    assert_eq!(w(catch + 8), ldr_x(9, 19, top));
    assert_eq!(w(catch + 12), add_x(8, 8, 0x10 * 3));
    // The command reads the object's accessor through `sv_system::battle_object_module_accessor` (L-8 holds the agent, its
    // field 0x1a0 the accessor): `ldur x8,[x0,#-8]; ldr x0,[x8,#0x1a0]; ret`.
    let accessor = model::sites_hitbox::HITBOX_ACCESSOR_OFFSET;
    assert_eq!(w(accessor), 0xf85f_8008, "ldur x8,[x0,#-8]");
    assert_eq!(w(accessor + 4), ldr_x(0, 8, 0x1a0));
}

// ---- the hurtbox part records and the owned-ELF instructions they come from ---------------------------------------------

/// A 0x50-byte part record as the game keeps it.
fn part(status: u32, a: [f32; 3], b: [f32; 3], size: f32, node: u64) -> [u8; 0x50] {
    let mut r = [0u8; 0x50];
    r[0..4].copy_from_slice(&status.to_le_bytes());
    for (i, v) in a.iter().chain(b.iter()).enumerate() {
        r[8 + 4 * i..12 + 4 * i].copy_from_slice(&v.to_bits().to_le_bytes());
    }
    r[0x20..0x24].copy_from_slice(&size.to_bits().to_le_bytes());
    r[0x28..0x30].copy_from_slice(&node.to_le_bytes());
    r
}

#[test]
fn core06_hurtbox_shapes_per_fighter() {
    use model::hitbox::layout;
    assert_eq!(std::mem::size_of::<[u8; layout::HIT_PART_STRIDE]>(), 0x50);
    // A capsule part: both centres, its size and its bone; drawn grey as one pill, like a hitbox.
    let capsule = hitbox::hurtbox_shape(&part(0, [0.0, 1.0, 0.0], [0.0, 6.0, 0.0], 2.5, 0x0000_0004_1234_5678)).unwrap();
    assert_eq!((capsule.joint, capsule.size, capsule.a, capsule.b), (0x0000_0004_1234_5678, 2.5, [0.0, 1.0, 0.0], Some([0.0, 6.0, 0.0])));
    // A part whose two centres coincide is one sphere; a centre at the bone's origin is a real centre (no "zero means absent").
    let ball = hitbox::hurtbox_shape(&part(0, [1.0, 2.0, 3.0], [1.0, 2.0, 3.0], 3.0, 7)).unwrap();
    assert_eq!(ball.b, None);
    let from_origin = hitbox::hurtbox_shape(&part(0, [0.0; 3], [4.0, 0.0, 0.0], 3.0, 7)).unwrap();
    assert_eq!(from_origin.b, Some([4.0, 0.0, 0.0]));
    // Intangible (1) and invincible (2) parts are still drawn; a part switched off (3) is not.
    for status in [0, 1, 2] {
        assert!(hitbox::hurtbox_shape(&part(status, [0.0; 3], [1.0; 3], 1.0, 7)).is_some(), "status {status}");
    }
    assert_eq!(hitbox::hurtbox_shape(&part(hitbox::HIT_STATUS_OFF, [0.0; 3], [1.0; 3], 1.0, 7)), None);
    // A normal hurtbox has the same grey identity in every drawing path.
    assert_eq!(hitbox::HURTBOX_TINT, [0.6; 3]);
    // A group's parts: `first..first + count`, inside the module's part count; anything else is not a hurtbox table.
    assert_eq!(hitbox::group_parts(0, 12, 12), Some(0..12));
    assert_eq!(hitbox::group_parts(3, 4, 20), Some(3..7));
    assert_eq!(hitbox::group_parts(0, 0, 0), Some(0..0));
    assert_eq!(hitbox::group_parts(-1, 4, 20), None);
    assert_eq!(hitbox::group_parts(0, -4, 20), None);
    assert_eq!(hitbox::group_parts(10, 12, 20), None, "past the part count");
    assert_eq!(hitbox::group_parts(0, 65, 100), None, "more parts than any fighter has");
}

#[test]
fn core06_hurtbox_layout_matches_the_owned_elf() {
    let Some(main) = common::owned_main("core06_hurtbox_layout_matches_the_owned_elf") else { return };
    let w = |offset: usize| word_at(&main, offset);
    use model::hitbox::layout;
    // Every HitModule lua_bind thunk loads the module from the accessor's +0xb0; set_status_all is vtable slot 0xc8.
    let thunk = 0x203_db10;
    assert_eq!(w(thunk), ldr_x(0, 0, layout::HIT_MODULE as u32));
    assert_eq!(w(thunk + 8), ldr_x(3, 8, 0xc8));
    assert_eq!(main.symbol("_ZN3app8lua_bind30HitModule__set_status_all_implEPNS_26BattleObjectModuleAccessorENS_9HitStatusEi"), Some(thunk));
    // set_status_all(status, group) (the vtable at main+0x4f6e1d0 by its relocations, slot 0xc8): the group table at +0x60,
    // 0x3c bytes a group, the count at +6, the first index at +4, the part table at +0x70, 0x50 bytes a part, the status word first.
    let g = 0x46_9af0;
    assert_eq!(w(g), ldr_x(9, 0, layout::HIT_GROUPS as u32));
    assert_eq!(w(g + 4), 0x321e_0fea, "orr w10, wzr, #0x3c");
    assert_eq!(w(g + 0x10), 0x78c0_6d4b, "ldrsh w11, [x10, #0x6]!");
    assert_eq!(w(g + 0x2c), add_x(12, 12, layout::HIT_GROUP_FIRST as u32));
    assert_eq!(w(g + 0x38), ldr_x(15, 0, layout::HIT_PARTS as u32));
    assert_eq!(w(g + 0x28), movz_w(13, layout::HIT_PART_STRIDE as u32));
    assert_eq!(w(g + 0x44), 0xb82e_69e1, "str w1, [x15, x14]: the status word at +0 of a part");
    // The part update: status 3 switches the part off; otherwise the capsule from +0x08 and +0x14, size +0x20, node +0x28.
    let u = 0x46_8490;
    assert_eq!(w(u + 4), 0x7100_0d7f, "cmp w11, #3");
    assert_eq!(w(0x4684c0), 0xbd40_0b00, "ldr s0, [x24, #0x8]");
    assert_eq!(w(0x4684c4), add_x(8, 24, 0xc));
    assert_eq!(w(0x4684c8), add_x(10, 24, 0x10));
    assert_eq!(w(0x4684e0), ldr_x(3, 24, layout::PART_NODE as u32));
    assert_eq!(w(0x4684e8), 0xbd40_1700, "ldr s0, [x24, #0x14]");
    assert_eq!(w(0x4684f0), add_x(8, 24, 0x18));
    assert_eq!(w(0x4684f4), add_x(10, 24, 0x1c));
    assert_eq!(w(0x46851c), 0xbd40_2300, "ldr s0, [x24, #0x20]: the size");
    // The part count at +0x78 (the add-part function reads and bumps it).
    assert_eq!(w(0x469540), 0xb9807818, "ldrsw x24, [x0, #0x78]");
    // get_part_size(group) answers the group's count (slot 0x170): `ldrsh w0, [x8, #0x6]`.
    assert_eq!(w(0x46a8bc), 0x79c0_0d00);
}

// ---- the hurtbox reader and the dispatch (decision HB1, renderer-agnostic) ---------------------------------------------

use model::hitbox::layout as hb_layout;

/// No HitModule bytes are exposed: the module pointer once, the two table pointers and the part total, the group record's
/// first/count, and each enabled part's 0x50-byte record — every address is checked, any other read fails.
struct HurtSpy {
    module: usize,
    groups: usize,
    parts: usize,
    total: i32,
    first: i16,
    count: i16,
    records: Vec<[u8; 0x50]>,
    parts_read: Vec<usize>,
    /// How often the reader asked for the module pointer (the Off/stop paths must not ask at all).
    module_calls: usize,
}

impl hitbox::HurtboxReader for HurtSpy {
    fn module(&mut self) -> usize {
        self.module_calls += 1;
        self.module
    }

    fn read_usize(&mut self, at: usize) -> usize {
        if at == self.module + hb_layout::HIT_GROUPS {
            self.groups
        } else if at == self.module + hb_layout::HIT_PARTS {
            self.parts
        } else {
            panic!("non-table pointer read {at:#x}")
        }
    }

    fn read_i32(&mut self, at: usize) -> i32 {
        if [model::legend::GROUP_WHOLE, model::legend::GROUP_GLOBAL].contains(&(at.wrapping_sub(self.groups))) { return 0; }
        assert_eq!(at, self.module + hb_layout::HIT_PART_TOTAL, "the part count is read once, at +0x78");
        self.total
    }

    fn read_i16(&mut self, at: usize) -> i16 {
        let group = self.groups + hitbox::HURTBOX_GROUP * hb_layout::HIT_GROUP_STRIDE;
        if at == group + hb_layout::HIT_GROUP_FIRST {
            self.first
        } else if at == group + hb_layout::HIT_GROUP_COUNT {
            self.count
        } else {
            panic!("non-group-field read {at:#x}")
        }
    }

    fn read_part(&mut self, at: usize, record: &mut [u8; hb_layout::HIT_PART_STRIDE]) {
        assert_eq!((at - self.parts) % hb_layout::HIT_PART_STRIDE, 0, "a part record starts on the 0x50 stride");
        self.parts_read.push(at);
        *record = self.records[(at - self.parts) / hb_layout::HIT_PART_STRIDE];
    }
}

fn hurt_spy(records: Vec<[u8; 0x50]>, first: i16, count: i16) -> HurtSpy {
    HurtSpy { module: 0xb000, groups: 0xc000, parts: 0xd000, total: records.len() as i32, first, count, records, parts_read: Vec::new(), module_calls: 0 }
}

#[test]
fn core06_hurtbox_reader_enumerates_the_enabled_parts() {
    // Group 0 only, the records on the 0x50 stride, exactly the reads the runtime would make.
    let mut records = vec![part(0, [0.6, 0.0, 0.0], [2.4, 0.0, 0.0], 2.4, 0x51a3_6341); 3];
    records.push(part(hitbox::HIT_STATUS_OFF, [1.0; 3], [2.0; 3], 1.0, 9)); // switched off: read, not drawn
    let mut spy = hurt_spy(records, 0, 4);
    let mut out = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
    let mut len = 0;
    let left = hitbox::want_hurtboxes(&mut spy, &mut out, &mut len);
    assert_eq!((len, left), (6, 0), "three capsule parts x 2 spheres (lr02 geometry: 1.8 apart, size 2.4)");
    for (i, at) in spy.parts_read.iter().enumerate() {
        assert_eq!(*at, 0xd000 + i * hb_layout::HIT_PART_STRIDE);
    }
    for sphere in &out[..len] {
        assert_eq!(sphere.tint, hitbox::HURTBOX_TINT, "grey");
        assert_eq!(sphere.joint, 0x51a3_6341);
        assert!((sphere.scale - hitbox::sphere_scale(2.4)).abs() < 1e-6);
        assert_eq!(sphere.key >> 24, hitbox::KIND_HURT);
    }
    assert_eq!(out[0].key, hitbox::sphere_key(hitbox::KIND_HURT, 0, 0));
    assert_eq!(out[1].key, hitbox::sphere_key(hitbox::KIND_HURT, 0, 1));
    assert_eq!(out[2].key, hitbox::sphere_key(hitbox::KIND_HURT, 1, 0));
    // A group that starts past the part count, or a count the bound refuses, reads nothing else.
    let mut spy = hurt_spy(vec![part(0, [0.0; 3], [1.0; 3], 1.0, 7)], 8, 4);
    let mut len = 0;
    assert_eq!(hitbox::want_hurtboxes(&mut spy, &mut out, &mut len), 0);
    assert_eq!(len, 0, "first 8 + count 4 > total 1: not a hurtbox table");
    assert!(spy.parts_read.is_empty());
    // No module: nothing.
    let mut spy = hurt_spy(Vec::new(), 0, 0);
    spy.module = 0;
    let mut len = 0;
    assert_eq!(hitbox::want_hurtboxes(&mut spy, &mut out, &mut len), 0);
    // Parts per fighter: a second fighter's own table is enumerated the same way, by its own indices.
    let mut other = hurt_spy(vec![part(0, [1.0, 2.0, 3.0], [1.0, 2.0, 3.0], 3.0, 7)], 0, 1);
    other.module = 0xb100;
    other.groups = 0xc100;
    other.parts = 0xd100;
    let mut len = 0;
    assert_eq!(hitbox::want_hurtboxes(&mut other, &mut out, &mut len), 0);
    assert_eq!(len, 1, "a coinciding centre pair is one sphere");
    assert_eq!(out[0].pos, [1.0, 2.0, 3.0]);
    assert_eq!(out[0].key, hitbox::sphere_key(hitbox::KIND_HURT, 0, 0));
}

#[test]
fn article_hurtboxes_share_the_reader_and_admit_whole_shapes_within_both_caps() {
    let capsule = part(0, [0.0; 3], [0.0, 2.0, 0.0], 2.0, 7);
    let point = part(0, [0.0; 3], [0.0; 3], 2.0, 7);
    let mut out = [hitbox::Sphere::NONE; 128];
    let (mut len, mut budget) = (0, hitbox::ARTICLE_HURT_BATCH);
    assert_eq!(hitbox::want_article_hurt(&mut hurt_spy(vec![capsule; 5], 0, 5), &mut out, &mut len, false, &mut budget), 2);
    assert_eq!((len, budget), (8, 16));
    assert!(out[..len].iter().all(|s| s.key >> 24 == hitbox::KIND_ARTICLE_HURT && s.tint == hitbox::HURTBOX_TINT));
    let mut records = vec![point; 7];
    records.extend([capsule, point]);
    (len, budget) = (0, hitbox::ARTICLE_HURT_BATCH);
    assert_eq!(hitbox::want_article_hurt(&mut hurt_spy(records, 0, 9), &mut out, &mut len, false, &mut budget), 2);
    assert_eq!((len, out[7].key), (8, hitbox::sphere_key(hitbox::KIND_ARTICLE_HURT, 8, 0)), "skip both ends; a later point still fits");
    let mut dropped = 0;
    budget = hitbox::ARTICLE_HURT_BATCH;
    for want in [8, 8, 8, 0] {
        len = 0;
        dropped += hitbox::want_article_hurt(&mut hurt_spy(vec![capsule; 4], 0, 4), &mut out, &mut len, false, &mut budget);
        assert_eq!(len, want);
    }
    assert_eq!((budget, dropped), (0, 8));
    (len, budget) = (0, hitbox::ARTICLE_HURT_BATCH);
    assert_eq!(hitbox::want_article_hurt(&mut hurt_spy(vec![], 0, 0), &mut out, &mut len, false, &mut budget), 0);
    assert_eq!((len, budget), (0, hitbox::ARTICLE_HURT_BATCH), "empty groups fabricate nothing");
    let mut record = point;
    record[..4].copy_from_slice(&model::legend::INVINCIBLE.to_le_bytes());
    hitbox::want_article_hurt(&mut hurt_spy(vec![record], 0, 1), &mut out, &mut len, true, &mut budget);
    assert_eq!((len, out[0].tint), (1, model::legend::hurt_tint(model::legend::INVINCIBLE)));
}

#[test]
fn article_discovery_is_off_without_queries_and_visits_every_matching_ordinal() {
    assert_eq!(hitbox::article_instances(false, |_, _| panic!("Off called a native")).count(), 0);
    let queries = std::cell::RefCell::new(Vec::new());
    let seen: Vec<_> = hitbox::article_instances(true, |kind, n| { queries.borrow_mut().push((kind, n)); match (kind, n) { (0, 0..=2) => 10 + n as usize, (1, 0) => 20, _ => 0 } }).collect();
    let queries = queries.into_inner();
    assert_eq!(seen, [10, 11, 12, 20]);
    assert_eq!(queries[..5], [(0, 0), (0, 1), (0, 2), (0, 3), (1, 0)]);
}

#[test]
fn core06_hurtbox_dispatch_equivalence_with_the_hitbox_rules() {
    // The hurt spheres go through the same set, the same requests and the same retention rules as the hit spheres, in one
    // reconcile: kept while unchanged, killed by their own handle when their shape changes or their part goes, dropped when
    // the set is full, and cleared together on the stop paths.
    /// One active attack slot while `on`, none otherwise (the hit the test switches on and off).
    struct OneHit(bool);
    impl hitbox::AttackReader for OneHit {
        fn active(&mut self, slot: i32) -> bool {
            self.0 && slot == 1
        }
        fn data(&mut self, _slot: i32) -> usize {
            0x1000
        }
        fn read_u64(&mut self, at: usize) -> u64 {
            match at {
                0x1048 => 7,
                0x1028 => (90u64 << 32) | 45,
                0x1030 => 40u64 << 32,
                0x1070 => 0,
                _ => panic!("non-attack word read {at:#x}"),
            }
        }
        fn read_f32(&mut self, at: usize) -> f32 {
            match at {
                0x1020 => 10.0,
                0x1024 => 2.0,
                0x1000 => 1.0,
                0x1004 | 0x1008 | 0x1010 | 0x1014 | 0x1018 => 0.0,
                _ => panic!("non-attack scalar read {at:#x}"),
            }
        }
    }
    let mut hurt = hurt_spy(vec![part(0, [0.6, 0.0, 0.0], [2.4, 0.0, 0.0], 2.4, 0x51a3_6341); 2], 0, 2);
    let mut set = hitbox::HandleSet::new();
    let mut fx = RequestSpy::new();
    let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
    // First pass with one active hit slot and two hurt parts: 1 + 2x2 spheres.
    let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut OneHit(true), Some(&mut hurt), &mut fx);
    assert_eq!((counts.spawned, counts.dropped), (5, 0));
    assert_eq!(len, 5);
    let keys: Vec<_> = fx.requests.iter().map(|s| s.key).collect();
    assert_eq!(keys[0], hitbox::sphere_key(hitbox::KIND_HIT, 1, 0));
    assert_eq!(keys[1], hitbox::sphere_key(hitbox::KIND_HURT, 0, 0));
    assert_eq!(*keys.last().unwrap(), hitbox::sphere_key(hitbox::KIND_HURT, 1, 1));
    // Unchanged frame: everything kept, no request.
    let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut OneHit(true), Some(&mut hurt), &mut fx);
    assert_eq!((counts.kept, counts.spawned, counts.killed, len), (5, 0, 0, 5));
    // The hit goes away: only the hit sphere is killed, by its own handle.
    let hit_handle = *fx.fx.live.first().unwrap();
    let (counts, _) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut OneHit(false), Some(&mut hurt), &mut fx);
    assert_eq!((counts.killed, counts.kept, counts.spawned), (1, 4, 0));
    assert!(!fx.fx.live.contains(&hit_handle));
    // A hurt part moves: its two spheres are killed and requested again; the rest kept.
    let before = fx.requests.len();
    hurt.records[0] = part(0, [3.6, 0.0, 0.0], [5.4, 0.0, 0.0], 2.4, 0x51a3_6341);
    let (counts, _) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut OneHit(false), Some(&mut hurt), &mut fx);
    assert_eq!((counts.killed, counts.spawned, counts.kept), (2, 2, 2));
    assert_eq!(fx.requests.len(), before + 2);
    assert_eq!(fx.requests[before].key, hitbox::sphere_key(hitbox::KIND_HURT, 0, 0));
    // A part is switched off: its spheres go, by their own handles.
    hurt.records[0] = part(hitbox::HIT_STATUS_OFF, [0.0; 3], [1.0; 3], 1.0, 7);
    let (counts, _) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut OneHit(false), Some(&mut hurt), &mut fx);
    assert_eq!((counts.killed, counts.kept, counts.spawned), (2, 2, 0));
    // Shielding clears the whole set (hurt spheres included), like the hit spheres; the game's own shield bubble is never
    // owned. Off clears too (facts the plan already covers).
    for (enabled, status) in [(true, ids().guard_on), (false, 0)] {
        let mut set = hitbox::HandleSet::new();
        let mut fx = RequestSpy::new();
        let mut hurt = hurt_spy(vec![part(0, [2.0, 1.0, 0.5], [2.0, 1.0, 0.5], 1.0, 7)], 0, 1); // a point part: one sphere
        let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
        hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut OneHit(false), Some(&mut hurt), &mut fx);
        assert_eq!(set.len(), 1);
        fx.fx.live.push(900); // the game's own shield bubble: never owned
        let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, enabled, false, status), &mut wants, &mut OneHit(false), Some(&mut hurt), &mut fx);
        assert_eq!((counts.killed, len), (1, 0));
        assert!(fx.fx.live.contains(&900));
        assert!(set.is_empty());
    }
    // A grab status (`CATCH..=CATCH_TURN`) keeps the grabber's hurt spheres: a whiffed grab can be punished. Its hit spheres
    // still go, as upstream's grab rule has them.
    for status in [ids().catch, ids().catch_turn] {
        let mut set = hitbox::HandleSet::new();
        let mut fx = RequestSpy::new();
        let mut hurt = hurt_spy(vec![part(0, [2.0, 1.0, 0.5], [2.0, 1.0, 0.5], 1.0, 7)], 0, 1);
        let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
        hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut OneHit(true), Some(&mut hurt), &mut fx);
        assert_eq!(set.len(), 2);
        let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, true, false, status), &mut wants, &mut OneHit(true), Some(&mut hurt), &mut fx);
        assert_eq!((counts.kept, counts.killed, counts.spawned, len), (1, 1, 0, 1), "status {status}");
        assert!(set.spheres().all(|s| s.key >> 24 == hitbox::KIND_HURT), "status {status}");
    }
    // A dispatch without a reader (`None`) plans and requests the hit spheres only: what the runtime passes on its
    // Off and stop paths never asks for a hurt sphere.
    let mut set = hitbox::HandleSet::new();
    let mut fx = RequestSpy::new();
    let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
    let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut OneHit(true), None, &mut fx);
    assert_eq!((counts.spawned, len), (1, 1), "the hit sphere only");
    assert!(fx.requests.iter().all(|s| s.key >> 24 == hitbox::KIND_HIT));
}

#[test]
fn core06_dispatch_reads_the_hit_module_only_while_the_setting_is_on() {
    // Route B (decision `5ca5e3b`): with the setting On the dispatch constructs the reader and reads the HitModule;
    // with it Off — and on the stop path it takes first (shielding) — the reader is never touched:
    // no module read, no part read, no hurt sphere. The runtime constructs the reader behind `enabled` itself; this
    // pins the dispatch side of the same gate.
    let mut hurt = hurt_spy(vec![part(0, [0.0, 1.0, 0.0], [0.0, 5.0, 0.0], 2.0, 0x51a3_6341)], 0, 1);
    let mut set = hitbox::HandleSet::new();
    let mut fx = RequestSpy::new();
    let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
    // Off: the set is cleared and the HitModule is not read at all.
    let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, false, false, 0), &mut wants, &mut NoHit, Some(&mut hurt), &mut fx);
    assert_eq!((counts.killed, counts.spawned, len), (0, 0, 0));
    assert_eq!((hurt.module_calls, hurt.parts_read.len(), fx.requests.len()), (0, 0, 0));
    // Shielding (the stop path the dispatch takes before the reader): no HitModule read either.
    {
        let mut set = hitbox::HandleSet::new();
        let mut fx = RequestSpy::new();
        let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
        let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, true, false, ids().guard_on), &mut wants, &mut NoHit, Some(&mut hurt), &mut fx);
        assert_eq!((counts.spawned, len), (0, 0));
        assert_eq!(hurt.module_calls, 0, "shielding: no HitModule read on a stop path");
    }
    // A grab status is not a stop path for the hurtboxes: the module is read and the grabber's capsule drawn.
    {
        let mut set = hitbox::HandleSet::new();
        let mut fx = RequestSpy::new();
        let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
        let mut hurt = hurt_spy(vec![part(0, [0.0, 1.0, 0.0], [0.0, 5.0, 0.0], 2.0, 0x51a3_6341)], 0, 1);
        let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, true, false, ids().catch), &mut wants, &mut NoHit, Some(&mut hurt), &mut fx);
        assert_eq!((counts.spawned, len), (2, 2));
        assert!(hurt.module_calls >= 1, "a grab status: the HitModule is read");
    }
    // On, ordinary status: the module is read and the part's capsule drawn (one part: its two ends, one pill).
    let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut NoHit, Some(&mut hurt), &mut fx);
    assert_eq!((counts.spawned, len), (2, 2));
    assert!(hurt.module_calls >= 1, "On: the HitModule is read");
    assert!(fx.requests.iter().all(|s| s.key >> 24 == hitbox::KIND_HURT && s.tint == hitbox::HURTBOX_TINT));
}

#[test]
fn core06_ridley_first_pass_completes_in_the_model() {
    // Ridley's first pass as the Eden evidence recorded it (stream `hitbox`, hb-04, 2026-11-02: the CPU Ridley's hurtboxes
    // drew, 55 spheres wanted, 53 requested in the first pass and 2 refused by the game's effect pool, `failed=0` after):
    // 27 capsule parts at the lr02 part geometry (centres 1.8 apart, size 2.4 -> 2 spheres a part) and one point part.
    let mut records = vec![part(0, [0.6, 0.0, 0.0], [2.4, 0.0, 0.0], 2.4, 0x51a3_6341); 27];
    records.push(part(0, [1.0, 2.0, 3.0], [1.0, 2.0, 3.0], 3.0, 0x51a3_6341));
    let mut hurt = hurt_spy(records, 0, 28);
    let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
    let mut len = 0;
    assert_eq!(hitbox::want_hurtboxes(&mut hurt, &mut wants, &mut len), 0, "the whole first pass fits the want buffer");
    assert_eq!(len, 55);
    // The set holds the first pass: every sphere requested, none dropped, and the next pass keeps them all.
    let mut set = hitbox::HandleSet::new();
    let mut big = FakeFx::new(10_000);
    let (counts, _) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut NoHit, Some(&mut hurt), &mut big);
    assert_eq!((counts.spawned, counts.failed, counts.dropped), (55, 0, 0));
    let (counts, _) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut NoHit, Some(&mut hurt), &mut big);
    assert_eq!((counts.kept, counts.spawned, counts.killed), (55, 0, 0));
    // And with the pool two short, as hb-04 saw it: 53 asked, 2 refused, and once the game's own effects end and the pool
    // gets room, the refused two are asked again and spawn (`failed=0` afterwards, as hb-04 recorded).
    let mut set = hitbox::HandleSet::new();
    let mut short = FakeFx::new(53);
    let (counts, _) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut NoHit, Some(&mut hurt), &mut short);
    assert_eq!((counts.spawned, counts.failed, counts.dropped), (53, 2, 0));
    // No retry in a loop: a refused want is attempted exactly once per pass (two refused requests in the log,
    // no third), and the refusals are counted, not silently dropped.
    let refused: Vec<_> = short.log.iter().filter(|l| l.starts_with("refused ")).collect();
    assert_eq!(refused.len(), 2, "one attempt per refused want in the pass: {refused:?}");
    assert_eq!(counts.failed, 2);
    short.pool = 55; // the game's own effects end; the pool gets room
    let before = short.log.len();
    let (counts, _) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut NoHit, Some(&mut hurt), &mut short);
    assert_eq!((counts.kept, counts.spawned, counts.failed), (53, 2, 0), "the two refused spheres are asked again");
    assert_eq!(short.log[before..].iter().filter(|l| l.starts_with("refused ")).count(), 0, "recovered: no refusal stays");
}

// ---- the live reader over a fake memory layout, the census, the shipped build's silence (C2) -------------------------

/// The fake layout's fields (offsets into one 8-aligned buffer).
const FAKE_ACCESSOR: usize = 0x100;
const FAKE_MODULE: usize = 0x200;
const FAKE_GROUPS: usize = 0x300;
const FAKE_PARTS: usize = 0x400;

#[repr(align(8))]
struct Align8([u8; 0x800]);

/// Fake game memory for `hitbox::GameHurt`: `load` answers only the addresses the layout laid out
/// (anything else fails the test, so the reader's address chain is pinned exactly), and addresses in
/// `unmapped` answer 0 the way Eden answers an unmapped read.
struct FakeMem {
    buf: Box<Align8>,
    reads: Vec<usize>,
    unmapped: Vec<usize>,
}

impl FakeMem {
    fn new() -> FakeMem {
        FakeMem { buf: Box::new(Align8([0; 0x800])), reads: Vec::new(), unmapped: Vec::new() }
    }

    fn base(&self) -> usize {
        self.buf.0.as_ptr() as usize
    }

    fn set_u64(&mut self, at: usize, value: u64) {
        let from = at - self.base();
        self.buf.0[from..from + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn set_u32(&mut self, at: usize, value: u32) {
        let from = at - self.base();
        self.buf.0[from..from + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn set_i16(&mut self, at: usize, value: i16) {
        let from = at - self.base();
        self.buf.0[from..from + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_part(&mut self, index: usize, record: [u8; 0x50]) {
        let from = FAKE_PARTS + index * 0x50;
        self.buf.0[from..from + 0x50].copy_from_slice(&record);
    }

    fn load(&mut self, at: usize, out: &mut [u8]) {
        let base = self.base();
        assert!(at >= base && at + out.len() <= base + 0x800, "read outside the fake layout: {at:#x}");
        self.reads.push(at);
        if self.unmapped.contains(&at) {
            out.fill(0);
            return;
        }
        let from = at - base;
        out.copy_from_slice(&self.buf.0[from..from + out.len()]);
    }
}

/// A fake layout: the memory and the absolute addresses of its accessor, HitModule, group and part tables.
struct FakeLayout {
    mem: FakeMem,
    accessor: usize,
    module: usize,
    groups: usize,
    parts: usize,
}

/// A fake layout with a three-part group 0: one capsule, one part switched off, one point part.
fn fake_hurt_module() -> FakeLayout {
    let mut mem = FakeMem::new();
    let base = mem.base();
    let (accessor, module, groups, parts) = (base + FAKE_ACCESSOR, base + FAKE_MODULE, base + FAKE_GROUPS, base + FAKE_PARTS);
    mem.set_u64(accessor + hb_layout::HIT_MODULE, module as u64);
    mem.set_u64(module + hb_layout::HIT_GROUPS, groups as u64);
    mem.set_u64(module + hb_layout::HIT_PARTS, parts as u64);
    mem.set_u32(module + hb_layout::HIT_PART_TOTAL, 3);
    mem.set_i16(groups + hb_layout::HIT_GROUP_FIRST, 0);
    mem.set_i16(groups + hb_layout::HIT_GROUP_COUNT, 3);
    mem.put_part(0, part(0, [0.0, 1.0, 0.0], [0.0, 5.0, 0.0], 2.0, 0x51a3_6341));
    mem.put_part(1, part(hitbox::HIT_STATUS_OFF, [0.0; 3], [1.0; 3], 1.0, 9));
    mem.put_part(2, part(0, [1.0, 2.0, 3.0], [1.0, 2.0, 3.0], 3.0, 7));
    FakeLayout { mem, accessor, module, groups, parts }
}

/// One `want_hurtboxes` over the fake layout: the spheres it left out, the wants it made and the exact addresses read.
fn want_via(layout: &mut FakeLayout, out: &mut [hitbox::Sphere]) -> (u32, usize, Vec<usize>) {
    let mut len = 0;
    let accessor = layout.accessor;
    let mut hurt = hitbox::GameHurt { accessor, load: |at: usize, dst: &mut [u8]| layout.mem.load(at, dst) };
    let left = hitbox::want_hurtboxes(&mut hurt, out, &mut len);
    let _ = hurt;
    (left, len, layout.mem.reads.clone())
}

#[test]
fn core06_game_hurt_reads_the_live_layout_at_the_exact_addresses() {
    let mut layout = fake_hurt_module();
    let (module, groups, parts) = (layout.module, layout.groups, layout.parts);
    let mut out = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
    let (left, len, reads) = want_via(&mut layout, &mut out);
    assert_eq!((len, left), (3, 0), "the capsule's two ends and the point part's one; the off part nothing");
    assert_eq!(
        reads,
        vec![
            layout.accessor + hb_layout::HIT_MODULE,
            module + hb_layout::HIT_GROUPS,
            module + hb_layout::HIT_PARTS,
            module + hb_layout::HIT_PART_TOTAL,
            groups + hb_layout::HIT_GROUP_FIRST,
            groups + hb_layout::HIT_GROUP_COUNT,
            parts,
            parts + hb_layout::HIT_PART_STRIDE,
            parts + 2 * hb_layout::HIT_PART_STRIDE,
        ],
        "the module pointer once, the two table pointers, the part total, the group's first/count, every part in place"
    );
    assert_eq!((out[0].pos, out[1].pos, out[2].pos), ([0.0, 1.0, 0.0], [0.0, 5.0, 0.0], [1.0, 2.0, 3.0]));
    assert_eq!(out[2].joint, 7);
    for sphere in &out[..len] {
        assert_eq!(sphere.tint, hitbox::HURTBOX_TINT, "grey");
        assert_eq!(sphere.key >> 24, hitbox::KIND_HURT);
    }
}

#[test]
fn core06_game_hurt_refuses_a_null_or_mistyped_pointer() {
    let mut out = [hitbox::Sphere::NONE; hitbox::MAX_HELD];

    // A null module pointer: only the accessor's module field is read, nothing else.
    let mut layout = fake_hurt_module();
    let field = layout.accessor + hb_layout::HIT_MODULE;
    layout.mem.set_u64(field, 0);
    assert_eq!(want_via(&mut layout, &mut out), (0, 0, vec![field]), "nothing past the null module pointer");

    // A mis-typed (misaligned) module pointer: refused the same way.
    let mut layout = fake_hurt_module();
    let field = layout.accessor + hb_layout::HIT_MODULE;
    layout.mem.set_u64(field, (layout.module + 2) as u64);
    assert_eq!(want_via(&mut layout, &mut out), (0, 0, vec![field]), "nothing past the misaligned module pointer");

    // A module pointer into unmapped memory: the table reads answer 0 (Eden's unmapped read), the reader refuses them.
    let mut layout = fake_hurt_module();
    let (field, groups_at, parts_at) =
        (layout.accessor + hb_layout::HIT_MODULE, layout.module + hb_layout::HIT_GROUPS, layout.module + hb_layout::HIT_PARTS);
    layout.mem.unmapped.push(groups_at);
    assert_eq!(want_via(&mut layout, &mut out), (0, 0, vec![field, groups_at, parts_at]), "both tables are read, both answer 0, nothing else");

    // A null group or part table pointer: refused before any group field or part record is read.
    for (name, offset) in [("group", hb_layout::HIT_GROUPS), ("part", hb_layout::HIT_PARTS)] {
        let mut layout = fake_hurt_module();
        let (field, groups_at, parts_at) =
            (layout.accessor + hb_layout::HIT_MODULE, layout.module + hb_layout::HIT_GROUPS, layout.module + hb_layout::HIT_PARTS);
        layout.mem.set_u64(layout.module + offset, 0);
        assert_eq!(want_via(&mut layout, &mut out), (0, 0, vec![field, groups_at, parts_at]), "no group field or part read with the {name} table pointer null");
    }

    // Part limits hold: a count past MAX_HURTBOX_PARTS, and a group past the part total, read no part record.
    for (first, count) in [(0i16, 65i16), (8i16, 4i16)] {
        let mut layout = fake_hurt_module();
        let groups = layout.groups;
        layout.mem.set_i16(groups + hb_layout::HIT_GROUP_FIRST, first);
        layout.mem.set_i16(groups + hb_layout::HIT_GROUP_COUNT, count);
        let (left, len, reads) = want_via(&mut layout, &mut out);
        assert_eq!((left, len), (0, 0), "no sphere for first {first} count {count}");
        assert!(!reads.iter().any(|at| *at >= layout.parts), "no part read for first {first} count {count}");
    }

    // The reader itself refuses a null or misaligned accessor without any read, and a misaligned part
    // record address in place (zeroed, no read).
    let stray = std::cell::Cell::new(0usize);
    let mut hurt = hitbox::GameHurt { accessor: 0, load: |_at: usize, _: &mut [u8]| stray.set(stray.get() + 1) };
    assert_eq!(hitbox::HurtboxReader::module(&mut hurt), 0);
    let mut layout = fake_hurt_module();
    let mut hurt = hitbox::GameHurt { accessor: layout.accessor + 2, load: |_at: usize, _: &mut [u8]| stray.set(stray.get() + 1) };
    assert_eq!(hitbox::HurtboxReader::module(&mut hurt), 0, "a misaligned accessor is refused without a read");
    let _ = hurt;
    assert_eq!(stray.get(), 0, "no read happened for either refused accessor");
    let parts = layout.parts;
    let mut hurt = hitbox::GameHurt { accessor: layout.accessor, load: |at: usize, dst: &mut [u8]| layout.mem.load(at, dst) };
    let mut record = [7u8; hb_layout::HIT_PART_STRIDE];
    hitbox::HurtboxReader::read_part(&mut hurt, parts + 2, &mut record);
    let _ = hurt;
    assert_eq!(record, [0; hb_layout::HIT_PART_STRIDE], "a misaligned record address is refused in place");
    assert!(!layout.mem.reads.contains(&(parts + 2)));
}

#[test]
fn core06_hurt_census_window_and_line_format() {
    let mut census = hitbox::HurtCensus::EMPTY;
    assert_eq!(census.window(600), None, "an empty window does not close");
    assert_eq!(census.window(0), None, "a zero window never closes");
    census.observe(19, 45, 100);
    census.observe(23, 55, 140);
    for _ in 0..598 {
        census.observe(20, 50, 120);
    }
    let window = census.window(600).unwrap();
    assert_eq!((window.frames, window.parts, window.parts_max, window.first), (600, 20, 23, 45), "the last parts, the most seen, the first pass's total");
    assert_eq!((window.ticks_min, window.ticks_avg, window.ticks_max), (100, 120, 140), "min, integer average, max over the window");
    // The runtime writes the prefix and the fighter kind around the model's fields.
    assert_eq!(
        format!("[Hitbox] hurt census kind=91 {window}"),
        "[Hitbox] hurt census kind=91 parts=20 parts_max=23 first_pass=45 frames=600 ticks_min=100 ticks_avg=120 ticks_max=140"
    );
    // The window reset: the aggregates start over; the first pass's total and the part maximum are kept.
    assert_eq!(census.frames, 0);
    census.observe(3, 4, 600);
    for _ in 0..599 {
        census.observe(3, 4, 700);
    }
    let window = census.window(600).unwrap();
    assert_eq!((window.parts, window.parts_max, window.first), (3, 23, 45));
    assert_eq!((window.ticks_min, window.ticks_avg, window.ticks_max), (600, 699, 700));
    // The observed frames saturate instead of wrapping (a window cannot lie by overflow).
    let mut census = hitbox::HurtCensus::EMPTY;
    census.frames = u32::MAX;
    census.observe(1, 1, 1);
    assert_eq!(census.frames, u32::MAX);
    // The want scratch is sized for the hit slots' and the hurt parts' spheres together (two per shape, its ends).
    assert_eq!(hitbox::WANT_SCRATCH, (hitbox::SLOTS as usize + hitbox::MAX_HURTBOX_PARTS as usize) * hitbox::SHAPE_SPHERES);
    const { assert!(hitbox::WANT_SCRATCH >= hitbox::MAX_HELD) };
}

#[test]
fn core06_release_reads_the_hit_module_only_for_the_visualizer_or_live_perfect() {
    // Keep the original visualizer admission and diagnostic census boundaries. The only second release caller is
    // a live Perfect attempt sampling the current primary P1 for a recovery plan.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/features/hitbox.rs")).unwrap();
    let toks = callgraph::lex(&src);
    let refs = |name: &str| -> Vec<usize> {
        toks.iter().enumerate().filter(|(_, t)| matches!(t, callgraph::Tok::Ident(s) if s == name)).map(|(i, _)| i).collect()
    };
    // The two spans the reader may live in: the diag-gated `stats` module and the per-frame pass.
    let mut stats_span = None;
    for i in 0..toks.len() - 2 {
        if matches!(&toks[i], callgraph::Tok::Ident(s) if s == "mod")
            && matches!(&toks[i + 1], callgraph::Tok::Ident(s) if s == "stats")
            && matches!(&toks[i + 2], callgraph::Tok::Punct('{'))
        {
            let mut depth = 0i32;
            let close = (i + 2..toks.len())
                .find(|&k| match &toks[k] {
                    callgraph::Tok::Punct('{') => {
                        depth += 1;
                        false
                    }
                    callgraph::Tok::Punct('}') => {
                        depth -= 1;
                        depth == 0
                    }
                    _ => false,
                })
                .expect("the stats module closes");
            stats_span = Some((i, close));
            break;
        }
    }
    let (stats_open, stats_close) = stats_span.expect("one diag `mod stats` block");
    assert!(
        toks[..stats_open].iter().rev().take(10).any(|t| matches!(t, callgraph::Tok::Ident(s) if s == "cfg")),
        "the stats module is diag-gated"
    );
    let inside_stats = |i: usize| i > stats_open && i < stats_close;
    let mut frame_span = None;
    for i in 0..toks.len() - 2 {
        if matches!(&toks[i], callgraph::Tok::Ident(s) if s == "fn")
            && matches!(&toks[i + 1], callgraph::Tok::Ident(s) if s == "frame")
            && matches!(&toks[i + 2], callgraph::Tok::Punct('('))
        {
            let mut depth = 0i32;
            let body_open = (i + 2..toks.len())
                .find(|&k| match &toks[k] {
                    callgraph::Tok::Punct('(') => {
                        depth += 1;
                        false
                    }
                    callgraph::Tok::Punct(')') => {
                        depth -= 1;
                        depth == 0
                    }
                    _ => false,
                })
                .and_then(|close| (close..toks.len()).find(|&k| matches!(&toks[k], callgraph::Tok::Punct('{'))))
                .expect("the frame body opens");
            let mut depth = 0i32;
            let close = (body_open..toks.len())
                .find(|&k| match &toks[k] {
                    callgraph::Tok::Punct('{') => {
                        depth += 1;
                        false
                    }
                    callgraph::Tok::Punct('}') => {
                        depth -= 1;
                        depth == 0
                    }
                    _ => false,
                })
                .expect("the frame body closes");
            frame_span = Some((body_open, close));
            break;
        }
    }
    let (frame_open, frame_close) = frame_span.expect("one `fn frame`");
    let inside_frame = |i: usize| i > frame_open && i < frame_close;
    let perfect_start = toks.windows(3).position(|w| {
        matches!(&w[0], callgraph::Tok::Ident(s) if s == "fn")
            && matches!(&w[1], callgraph::Tok::Ident(s) if s == "perfect_player_height")
            && matches!(&w[2], callgraph::Tok::Punct('('))
    }).expect("the second release reader belongs to the live Perfect player snapshot");
    let perfect_open = (perfect_start + 3..toks.len())
        .find(|&i| matches!(toks[i], callgraph::Tok::Punct('{')))
        .expect("the Perfect reader body opens");
    let mut depth = 0;
    let perfect_close = (perfect_open..toks.len()).find(|&i| match toks[i] {
        callgraph::Tok::Punct('{') => { depth += 1; false }
        callgraph::Tok::Punct('}') => { depth -= 1; depth == 0 }
        _ => false,
    }).expect("the Perfect reader body closes");
    let inside_perfect = |i: usize| i > perfect_open && i < perfect_close;
    assert_eq!(refs("GameHurt").into_iter().filter(|&i| inside_perfect(i)).count(), 1);
    let adapter = include_str!("../src/runtime/features/edge_juggle.rs");
    assert_eq!(src.matches("perfect_player_height(&attempt)").count(), 1);
    assert!(adapter.contains("wanted() == Some(run.drill)"));
    assert!(adapter.contains("run.difficulty == Difficulty::Perfect"));
    assert!(adapter.contains("run.rounds.phase == Phase::Live"));
    assert!(adapter.contains("run.wizard.is_none()"));
    assert!(adapter.contains("natives::module_accessor(0)"));
    assert!(src.contains("let player = attempt.player()"));
    assert!(src.contains("let show_hurt = enabled && shown(legend::HURTBOXES);"));
    assert!(src.contains("let perfect = if show_hurt {
        perfect
    } else {"));
    assert!(adapter.contains("&& run.plan_pending"));
    assert!(adapter.contains("airborne_recovery(cpu, natives::status_kind(cpu))"));
    assert!(adapter.contains("run.player_height.request(frame, actionable)"));
    let plan = include_str!("../src/model/edge_juggle/perfect.rs");
    assert!(plan.contains("!actionable || self.frame == Some(frame) || self.value.is_some()"));
    // Reader construction and raw loads have the same reachability boundary.
    for name in ["GameHurt", "mem_load"] {
        let found = refs(name);
        assert!(!found.is_empty(), "{name} must appear in the runtime file");
        for &i in &found {
            let definition = toks[..i].iter().rev().take(3).any(|t| matches!(t, callgraph::Tok::Ident(s) if s == "fn"));
            assert!(definition || inside_frame(i) || inside_perfect(i) || inside_stats(i), "{name} at token {i} is outside the two release callers and diag census");
        }
    }
    // The construction is behind the setting: `enabled` names itself within the few tokens before `GameHurt`.
    for &i in &refs("GameHurt") {
        if inside_frame(i) {
            assert!(
                toks[i.saturating_sub(10)..i].iter().any(|t| matches!(t, callgraph::Tok::Ident(s) if s == "enabled" || s == "show_hurt")),
                "the pass's reader at token {i} is not gated on `enabled`"
            );
        }
    }
    // The census stays diag-only: its walk and its window type are named only inside the gated module.
    for name in ["hurt_parts", "HurtCensus"] {
        let found = refs(name);
        assert!(!found.is_empty(), "{name} must appear in the runtime file");
        assert!(found.iter().all(|&i| inside_stats(i) || name == "hurt_parts" && inside_perfect(i)), "{name} is outside its allowed readers");
    }
    // The per-frame census call: cfg-gated wherever it sits.
    for i in refs("hurt_census") {
        assert!(
            inside_stats(i) || toks[..i].iter().rev().take(24).any(|t| matches!(t, callgraph::Tok::Ident(s) if s == "cfg")),
            "the census call at token {i} is cfg-gated"
        );
    }
}

#[test]
fn core06_effect_pool_refusal_facts_match_the_owned_elf() {
    let Some(main) = common::owned_main("core06_effect_pool_refusal_facts_match_the_owned_elf") else { return };
    let w = |offset: usize| word_at(&main, offset);
    assert_eq!(w(0x44fbe4), 0xb002_773a, "adrp x26, main+0x5334000");
    assert_eq!(w(0x44fbf4), ldr_x(9, 26, 0x920), "singleton at main+0x5334920");
    assert_eq!(w(0x44fc14), ldr_x(0, 9, 0), "manager is singleton's first pointer");
    assert_eq!(w(0x35636b8), movz_w(8, 0x9460));
    assert_eq!(w(0x35636bc), 0x72a0_0028, "free head offset high16 = 1");
    assert_eq!(w(0x35636c4), ldr_x(0, 10, 0));
    assert_eq!(w(0x35636c8), 0xb4ff_fca0, "null free head branches to zero result");
    assert_eq!(w(0x35636d4), movz_w(10, 0x9468));
    assert_eq!(w(0x35636d8), 0x72a0_002a, "free count offset high16 = 1");
    assert_eq!(w(0x35636e0), 0x5100_056b, "taking a node decrements the free count");
    assert_eq!(w(0x35637d4), 0x2a1f_03fc, "lower constructor failure also returns zero");
}

// ---- persistent spheres: the handle-set state machine (lead ruling on Q3) ---------------------------------------------

/// A fake effect module: handles 1, 2, 3, ... while the pool has room; `end` ends a handle as a status change would.
struct FakeFx {
    live: Vec<u32>,
    next: u32,
    pool: usize,
    log: Vec<String>,
}

impl FakeFx {
    fn new(pool: usize) -> FakeFx {
        FakeFx { live: Vec::new(), next: 1, pool, log: Vec::new() }
    }
    fn end(&mut self, h: u32) {
        self.live.retain(|x| *x != h);
    }
}

impl hitbox::Effects for FakeFx {
    fn sphere_alive(&mut self, h: u32) -> bool {
        self.live.contains(&h)
    }
    fn sphere_kill_own(&mut self, h: u32) {
        assert!(self.live.contains(&h), "killed a handle that is not alive: {h}");
        self.live.retain(|x| *x != h);
        self.log.push(format!("kill {h}"));
    }
    fn sphere_spawn(&mut self, s: &hitbox::Sphere, _made_in: Option<i32>) -> u32 {
        if self.live.len() >= self.pool {
            self.log.push(format!("refused {:#x}", s.key));
            return 0;
        }
        let h = self.next;
        self.next += 1;
        self.live.push(h);
        self.log.push(format!("spawn {:#x} -> {h}", s.key));
        h
    }
}

fn sphere(key: u32, x: f32) -> hitbox::Sphere {
    hitbox::Sphere { key, joint: 7, pos: [x, 0.0, 0.0], scale: 0.2, tint: hitbox::HURTBOX_TINT }
}

/// No accessor/HitModule bytes are exposed: any read outside these two exact
/// AttackData shapes fails. Slot 6 also exercises an active slot with null data.
#[derive(Default)]
struct AttackSpy {
    active: Vec<i32>,
    data: Vec<i32>,
    reads: Vec<usize>,
}

impl hitbox::AttackReader for AttackSpy {
    fn active(&mut self, slot: i32) -> bool {
        self.active.push(slot);
        matches!(slot, 1 | 5 | 6)
    }

    fn data(&mut self, slot: i32) -> usize {
        self.data.push(slot);
        match slot { 1 => 0x1000, 5 => 0x2000, 6 => 0, _ => panic!("inactive slot read {slot}") }
    }

    fn read_u64(&mut self, at: usize) -> u64 {
        self.reads.push(at);
        match at {
            0x1048 => 7,
            0x2048 => 9,
            0x1028 | 0x2028 => (90u64 << 32) | 45,
            0x1030 | 0x2030 => 40u64 << 32,
            0x1070 | 0x2070 => 0,
            _ => panic!("non-attack word read {at:#x}"),
        }
    }

    fn read_f32(&mut self, at: usize) -> f32 {
        self.reads.push(at);
        match at {
            0x1020 | 0x2020 => 10.0,
            0x1024 | 0x2024 => 2.0,
            0x1000 => 1.0,
            0x1004 | 0x1008 | 0x1010 | 0x1014 | 0x1018 => 0.0,
            0x2000 | 0x2004 | 0x2008 | 0x2014 | 0x2018 => 0.0,
            0x2010 => 10.0,
            _ => panic!("non-attack scalar read {at:#x}"),
        }
    }
}

/// No active attack slot at all (the Ridley first pass: hurtboxes only).
struct NoHit;

impl hitbox::AttackReader for NoHit {
    fn active(&mut self, _slot: i32) -> bool {
        false
    }
    fn data(&mut self, slot: i32) -> usize {
        panic!("inactive slot read {slot}")
    }
    fn read_u64(&mut self, at: usize) -> u64 {
        panic!("non-attack node read {at:#x}")
    }
    fn read_f32(&mut self, at: usize) -> f32 {
        panic!("non-attack scalar read {at:#x}")
    }
}

struct RequestSpy {
    fx: FakeFx,
    requests: Vec<hitbox::Sphere>,
}

impl RequestSpy {
    fn new() -> Self { Self { fx: FakeFx::new(100), requests: Vec::new() } }
}

impl hitbox::Effects for RequestSpy {
    fn sphere_alive(&mut self, handle: u32) -> bool { hitbox::Effects::sphere_alive(&mut self.fx, handle) }
    fn sphere_kill_own(&mut self, handle: u32) { hitbox::Effects::sphere_kill_own(&mut self.fx, handle) }
    fn sphere_spawn(&mut self, value: &hitbox::Sphere, made_in: Option<i32>) -> u32 {
        self.requests.push(*value);
        hitbox::Effects::sphere_spawn(&mut self.fx, value, made_in)
    }
}

#[test]
fn core06_rc_dispatch_reads_only_attacks_and_requests_hits_and_grabs() {
    let mut snapshot = model::settings::Snapshot::factory();
    snapshot.set("hitbox_vis", &[1, 1, 1, 0, 0, 0]).unwrap();
    let mut set = hitbox::HandleSet::new();
    let grab = sphere(hitbox::sphere_key(hitbox::KIND_GRAB, 2, 0), 4.0);
    assert!(set.push_grab(grab, None));
    // Poison all unused scratch entries with valid-looking HB1 requests: the
    // production dispatcher must neither retain nor submit those stale values.
    let poison = sphere(hitbox::sphere_key(hitbox::KIND_HURT, 0, 0), 9.0);
    let mut wants = [poison; hitbox::MAX_HELD];
    let mut reads = AttackSpy::default();
    let mut fx = RequestSpy::new();
    let facts = facts(true, model::legend::draws(hitbox::shown(&snapshot)), false, 0);
    let (counts, hits) = hitbox::reconcile_frame(&mut set, facts, &mut wants, &mut reads, None, &mut fx);
    assert_eq!((counts.spawned, counts.failed, counts.dropped, hits), (4, 0, 0, 3));
    assert_eq!(reads.active, (0..8).chain(0..8).collect::<Vec<_>>());
    assert_eq!(reads.data, [1, 5, 6, 1, 5, 6]);
    assert_eq!(reads.reads, [0x1048, 0x1024, 0x1000, 0x1004, 0x1008, 0x1010, 0x1014, 0x1018, 0x1070,
                            0x2048, 0x2024, 0x2000, 0x2004, 0x2008, 0x2010, 0x2014, 0x2018, 0x2070,
                            0x1028, 0x1030, 0x1020, 0x2028, 0x2030, 0x2020]);
    let expected_keys: Vec<_> = std::iter::once(hitbox::sphere_key(hitbox::KIND_HIT, 1, 0))
        .chain((0..2).map(|i| hitbox::sphere_key(hitbox::KIND_HIT, 5, i))).chain([grab.key]).collect();
    assert_eq!(fx.requests.iter().map(|s| s.key).collect::<Vec<_>>(), expected_keys);
    assert_eq!(fx.requests[0], hitbox::Sphere { key: expected_keys[0], joint: 7, pos: [1.0, 0.0, 0.0], scale: 0.19, tint: hitbox::hitbox_tint(1) });
    assert_eq!(fx.requests[1].joint, 9);
    assert_eq!(fx.requests[1].pos, [0.0; 3]);
    assert_eq!(fx.requests[2].pos, [10.0, 0.0, 0.0], "slot 5's capsule: its two ends");
    assert_eq!(fx.requests[2].tint, hitbox::hitbox_tint(5));
    assert_eq!(fx.requests[3], grab);
    assert!(fx.requests.iter().all(|s| s.key >> 24 != hitbox::KIND_HURT));
    let (counts, hits) = hitbox::reconcile_frame(&mut set, facts, &mut wants, &mut reads, None, &mut fx);
    assert_eq!((counts.spawned, counts.kept, hits), (0, 4, 3), "unchanged owned handles are retained");
    assert_eq!(fx.requests.len(), 4);
}

#[test]
fn core06_rc_dispatch_preserves_grab_shield_off_and_retirement_cleanup() {
    for (visualizes, enabled, status, keep_grab) in [(true, true, 62, true), (true, true, 30, false), (true, false, 0, false), (false, true, 0, false)] {
        let mut set = hitbox::HandleSet::new();
        let mut fx = RequestSpy::new();
        // A normal game shield (or another plugin's effect) is never owned.
        fx.fx.live.push(900);
        let grab = sphere(hitbox::sphere_key(hitbox::KIND_GRAB, 2, 0), 4.0);
        set.push_grab(grab, None);
        let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
        hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut AttackSpy::default(), None, &mut fx);
        let mut reads = AttackSpy::default();
        let requests = fx.requests.len();
        let (counts, hits) = hitbox::reconcile_frame(&mut set, facts(visualizes, enabled, false, status), &mut wants, &mut reads, None, &mut fx);
        assert_eq!((reads.active.len(), reads.data.len(), reads.reads.len(), hits), (0, 0, 0, 0));
        assert_eq!(fx.requests.len(), requests, "no attack/HB1/ordinary shield request on stop path");
        assert_eq!((counts.killed, counts.kept), if keep_grab { (3, 1) } else { (4, 0) });
        assert!(fx.fx.live.contains(&900), "ordinary shield survives cleanup");
        assert_eq!(set.len(), usize::from(keep_grab));
    }
}

#[test]
fn core06_rc_closure_holds_hb1_on_route_b() {
    // Identical SITES/closure is used with and without `diag`; run this probe
    // in both host feature closures, and verify each compiled RC NRO separately.
    // The draw route is wired (route B, operator decision 2026-11-04, `docs/DECISIONS.md` HB1-route,
    // `review/menu-proposal` `5ca5e3b`): the three relied HitModule windows stay relied on — the pass reads them
    // while the setting is On — but no hurtbox site is hooked: the rows are relied on, never written, and no
    // `hurtbox_*` row is a hook site.
    for (offset, name) in [(sites_hitbox::HITBOX_HURT_MODULE_OFFSET, "hurtbox_module"), (sites_hitbox::HITBOX_HURT_GROUPS_OFFSET, "hurtbox_groups"), (sites_hitbox::HITBOX_HURT_PART_OFFSET, "hurtbox_part_shape")] {
        let site = SITES.iter().find(|s| s.offset == offset).unwrap_or_else(|| panic!("{name} back"));
        assert_eq!(site.name, name);
        assert!(matches!(site.kind, SeamKind::Relied), "{name}: relied on, never written");
        assert!(!capabilities::all_sites().any(|s| s.offset == offset && matches!(s.kind, SeamKind::Hook { .. })), "{name}: never hooked");
    }
    assert!(!SITES.iter().any(|s| matches!(s.kind, SeamKind::Hook { .. }) && s.name.starts_with("hurtbox_")));
    for required in ["hitbox_attack", "hitbox_catch", "hitbox_set_rebound", "attack_is_attack", "attack_data", "effect_req_follow", "sphere_effect_kill"] {
        assert_eq!(capabilities::blocked_by(Feature::Hitbox, &|s| s.name != required), Some(required));
    }
}

#[test]
fn core06_persistent_spheres() {
    use hitbox::{sphere_key, HandleSet, KIND_GRAB, KIND_HIT, KIND_HURT};
    let mut fx = FakeFx::new(100);
    let mut set = HandleSet::new();
    let hurt: Vec<_> = (0..3).map(|i| sphere(sphere_key(KIND_HURT, i, 0), i as f32)).collect();
    // First pass: every sphere is requested once.
    let c = set.reconcile(&hurt, true, &mut fx);
    assert_eq!((c.spawned, c.kept, c.killed), (3, 0, 0));
    // Unchanged frames: nothing is requested or killed, only checked (the effects follow their bones by themselves).
    for _ in 0..10 {
        let c = set.reconcile(&hurt, true, &mut fx);
        assert_eq!((c.spawned, c.kept, c.killed, c.ended), (0, 3, 0, 0));
    }
    assert_eq!(fx.live.len(), 3);
    // A hitbox appears: only it is requested; it goes: only it is killed, by its own handle.
    let mut with_hit = hurt.clone();
    with_hit.push(sphere(sphere_key(KIND_HIT, 0, 0), 9.0));
    let c = set.reconcile(&with_hit, true, &mut fx);
    assert_eq!((c.spawned, c.kept), (1, 3));
    let hit_handle = *fx.live.last().unwrap();
    let c = set.reconcile(&hurt, true, &mut fx);
    assert_eq!((c.killed, c.kept, c.spawned), (1, 3, 0));
    assert!(fx.log.contains(&format!("kill {hit_handle}")));
    // A shape that changed (a part moved to another offset) is killed and requested again.
    let mut moved = hurt.clone();
    moved[1].pos = [5.0, 0.0, 0.0];
    let c = set.reconcile(&moved, true, &mut fx);
    assert_eq!((c.killed, c.spawned, c.kept), (1, 1, 2));
    // The game ended a sphere (a status change concluded it): it is requested again, never killed.
    let ended = set.handles().next().unwrap();
    fx.end(ended);
    let c = set.reconcile(&moved, true, &mut fx);
    assert_eq!((c.ended, c.spawned, c.killed), (1, 1, 0));
    // Grab spheres: queued by the command, requested by the pass, kept while the game keeps them, never requested again.
    assert!(set.push_grab(sphere(sphere_key(KIND_GRAB, 0, 0), 1.0), None));
    assert!(set.push_grab(sphere(sphere_key(KIND_GRAB, 0, 1), 2.0), None));
    let c = set.reconcile(&[], true, &mut fx);
    assert_eq!(c.spawned, 2, "the grab spheres");
    assert_eq!(c.killed, 3, "a grab status wants no hitbox or hurtbox spheres");
    assert_eq!(set.len(), 2);
    let grab = set.handles().next().unwrap();
    fx.end(grab);
    let c = set.reconcile(&[], true, &mut fx);
    assert_eq!((c.ended, c.spawned, c.kept), (1, 0, 1), "a grab sphere the game ended is not requested again");
    // The command sets the same grab box again (Steve's tether in Eden hb-03 re-issued id 0 every few frames): one sphere per
    // key, the old one replaced, never piling up.
    for x in [4.0, 5.0, 5.0] {
        assert!(set.push_grab(sphere(sphere_key(KIND_GRAB, 0, 0), x), None));
        set.reconcile(&[], true, &mut fx);
    }
    assert_eq!(set.len(), 2, "the surviving grab sphere and the one of key (grab 0, sphere 0)");
    // Off / shield / rebound / not practice: every held sphere is killed by its handle, the queue is dropped.
    assert!(set.push_grab(sphere(sphere_key(KIND_GRAB, 1, 0), 3.0), None));
    let c = set.clear(&mut fx);
    assert_eq!((c.killed, c.spawned), (2, 0));
    assert!(set.is_empty() && fx.live.is_empty());
    // A full pool: the refused requests are counted and asked again next pass; what fits stays.
    let mut small = FakeFx::new(2);
    let mut set = HandleSet::new();
    let c = set.reconcile(&hurt, true, &mut small);
    assert_eq!((c.spawned, c.failed), (2, 1));
    let c = set.reconcile(&hurt, true, &mut small);
    assert_eq!((c.kept, c.failed, c.spawned), (2, 1, 0));
    // Never more than MAX_HELD per fighter: the rest is counted as dropped.
    let mut big = FakeFx::new(10_000);
    let mut set = HandleSet::new();
    let many: Vec<_> = (0..hitbox::MAX_HELD + 5).map(|i| sphere(sphere_key(KIND_HURT, i, 0), i as f32)).collect();
    let c = set.reconcile(&many, true, &mut big);
    assert_eq!((c.spawned as usize, c.dropped), (hitbox::MAX_HELD, 5));
    // Forget (the fighter object was initialised again): records go without a single call.
    let calls = big.log.len();
    set.forget();
    assert!(set.is_empty());
    assert_eq!(big.log.len(), calls);
}

#[test]
fn core06_fighter_sets_and_keys() {
    use hitbox::{sphere_key, SetTable, FIGHTER_SETS, KIND_GRAB, KIND_HIT, KIND_HURT};
    // Keys tell kind, index and sphere apart.
    let keys = [sphere_key(KIND_HIT, 0, 0), sphere_key(KIND_HIT, 0, 1), sphere_key(KIND_HIT, 1, 0), sphere_key(KIND_HURT, 0, 0), sphere_key(KIND_GRAB, 0, 0)];
    for (i, a) in keys.iter().enumerate() {
        for b in &keys[i + 1..] {
            assert_ne!(a, b);
        }
    }
    // push_shape: a capsule's two ends in order, keyed per sphere, scaled and tinted; what does not fit is counted.
    let shape = hitbox::Shape { id: 2, joint: 9, size: 2.0, a: [0.0; 3], b: Some([10.0, 0.0, 0.0]) };
    let mut out = [hitbox::Sphere::NONE; 1];
    let mut len = 0;
    let left = hitbox::push_shape(&mut out, &mut len, KIND_HIT, 2, &shape, hitbox::hitbox_tint(2));
    assert_eq!((len, left), (1, 1), "two ends, one left out");
    assert_eq!(out[0].key, sphere_key(KIND_HIT, 2, 0));
    assert!((out[0].scale - 0.19).abs() < 1e-6);
    // The table: one set per fighter; a new fighter takes a free set, then the least recently used one, forgotten.
    let mut table = SetTable::new();
    let first = table.index_for(0x1000);
    assert_eq!(table.index_for(0x1000), first);
    for k in 1..FIGHTER_SETS {
        table.index_for(0x1000 + k * 0x100);
    }
    table.sets[first].push_grab(hitbox::Sphere::NONE, None);
    assert!(!table.sets[first].is_empty());
    table.index_for(0x1000 + 0x100); // used again: 0x1000 is now the oldest
    let taken = table.index_for(0x9000);
    assert_eq!(taken, first, "the least recently used fighter's set is taken over");
    assert!(table.sets[taken].is_empty(), "and forgotten, without a game call");
    assert_eq!(table.find(0x1000), None);
    assert_eq!(table.find(0), None);
}

#[test]
fn core06_native_emitter_limits_are_distinct_from_pool_capacity() {
    let Some(main) = common::owned_main("core06_native_emitter_limits_are_distinct_from_pool_capacity") else { return };
    let w = |offset: usize| word_at(&main, offset);
    assert_eq!(w(0x356cfe4), 0x9000ee4a); // global page 0x5334000
    assert_eq!(w(0x356cfe8), 0x9128014a); // +0xa00 limit base
    assert_eq!(w(0x356cfec), 0xb9400d49); // priority count +0xc
    assert_eq!(w(0x356cff0), 0xb940014a); // priority limit +0
    assert_eq!(w(0x356cff4), 0x6b0a013f); // signed count versus limit
    assert_eq!(w(0x356d03c), 0x91281129); // normal limit base +0xa04
    assert_eq!(w(0x356d040), 0xb940112a); // normal count base+0x10 = 0xa14
    assert_eq!(w(0x356d044), 0xb9400129); // normal limit +0
    assert_eq!(w(0x356d04c), 0x54fffdaa); // count >= limit refuses
    assert_eq!(w(0x356d01c), 0x2a1f03e0); // zero result
    assert_eq!(w(0x3563764), 0x36000360); // caller observes failed constructor
}
