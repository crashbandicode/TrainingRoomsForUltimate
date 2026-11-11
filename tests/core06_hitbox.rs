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
    // The spec's table, by `id mod 8`.
    let spec: [[f32; 3]; 8] = [[1.0, 0.0, 0.0], [1.0, 0.4, 0.0], [0.8, 1.0, 0.0], [0.2, 1.0, 0.2], [0.0, 0.8, 1.0], [0.4, 0.4, 1.0], [0.8, 0.0, 1.0], [1.0, 0.2, 0.8]];
    for (id, tint) in spec.iter().enumerate() {
        assert_eq!(hitbox::hitbox_tint(id as i32), *tint, "id {id}");
        assert_eq!(hitbox::hitbox_tint(id as i32 + 8), *tint, "id {} wraps", id + 8);
        assert!(tint.contains(&1.0), "fully saturated: {tint:?}");
    }
    // A negative id wraps the same way.
    assert_eq!(hitbox::hitbox_tint(-1), spec[7]);
    // Grab boxes use `(id + 3) mod 8`.
    for id in 0..16 {
        assert_eq!(hitbox::grab_tint(id), spec[((id + 3) % 8) as usize], "grab id {id}");
    }
    assert_eq!(hitbox::grab_tint(5), spec[0]);
    assert_ne!(hitbox::grab_tint(0), hitbox::hitbox_tint(0));
}

#[test]
fn core06_capsule_sampling() {
    // n = ceil(d / (size * 1.75)) + 1, clamped to 2..16.
    assert_eq!(hitbox::sphere_count(10.0, 2.0), 4); // ceil(10 / 3.5) = 3, +1
    assert_eq!(hitbox::sphere_count(7.0, 2.0), 3); // exactly two steps, +1
    assert_eq!(hitbox::sphere_count(0.0, 2.0), 2); // ceil(0) + 1 = 1, clamped up
    assert_eq!(hitbox::sphere_count(0.1, 2.0), 2);
    assert_eq!(hitbox::sphere_count(1000.0, 2.0), 16);
    assert_eq!(hitbox::sphere_count(3.5 * 14.0, 2.0), 15);
    assert_eq!(hitbox::sphere_count(3.5 * 15.0, 2.0), 16);
    assert_eq!(hitbox::sphere_count(3.5 * 40.0, 2.0), 16);
    // A degenerate size or distance draws the minimum.
    for (d, s) in [(5.0, 0.0), (5.0, -1.0), (f32::NAN, 2.0), (f32::INFINITY, 2.0), (-3.0, 2.0), (5.0, f32::NAN)] {
        assert_eq!(hitbox::sphere_count(d, s), 2, "{d} {s}");
    }
    // Evenly spaced, `t = i / (n - 1)`, both ends included.
    let a = [1.0, 2.0, 3.0];
    let b = [11.0, 2.0, 3.0];
    let points: Vec<[f32; 3]> = hitbox::spheres(a, Some(b), 2.0).collect();
    assert_eq!(points.len(), 4);
    assert_eq!(points[0], a);
    assert_eq!(points[3], b);
    for (i, p) in points.iter().enumerate() {
        let t = i as f32 / 3.0;
        assert!((p[0] - (1.0 + 10.0 * t)).abs() < 1e-5, "{i}: {p:?}");
    }
    // A point hitbox (no second centre) is one sphere at its centre.
    let one: Vec<[f32; 3]> = hitbox::spheres(a, None, 2.0).collect();
    assert_eq!(one, vec![a]);
    assert_eq!(hitbox::second_centre([0.0; 3]), None);
    assert_eq!(hitbox::second_centre([0.0, 0.0, 0.5]), Some([0.0, 0.0, 0.5]));
    // The sphere scale is size * 19/200.
    assert!((hitbox::sphere_scale(4.0) - 0.38).abs() < 1e-6);
    // At most 16 spheres are ever produced.
    assert_eq!(hitbox::spheres([0.0; 3], Some([500.0, 0.0, 0.0]), 1.0).count(), 16);
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
    // The setting: a two-option single choice, On when its first option is set, Off by default.
    let mut snapshot = model::settings::Snapshot::factory();
    assert!(!hitbox::enabled(&snapshot));
    snapshot.set("hitbox_vis", &[1, 0]).unwrap();
    assert!(hitbox::enabled(&snapshot));
    snapshot.set("hitbox_vis", &[0, 1]).unwrap();
    assert!(!hitbox::enabled(&snapshot));
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
    // Three hooks, eleven called natives and ten relied windows (the Lua stack reads, HB1's three HitModule windows and, for
    // the own orbs (decision HB-OWN-ORBS), the bone transform, the projection and its screen size words).
    let hooks: Vec<&str> = SITES.iter().filter(|s| matches!(s.kind, SeamKind::Hook { .. })).map(|s| s.name).collect();
    assert_eq!(hooks, sites_hitbox::HOOK_ROWS);
    assert_eq!(SITES.iter().filter(|s| matches!(s.kind, SeamKind::Called)).count(), 11);
    assert_eq!(SITES.iter().filter(|s| matches!(s.kind, SeamKind::Relied)).count(), 10);
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
        assert_eq!(capabilities::blocked_by(Feature::Hitbox, &usable), Some(site.name), "{}", site.name);
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
        (sites_hitbox::HITBOX_REQ_FOLLOW_OFFSET, "effect_req_follow"),
        (sites_hitbox::HITBOX_KILL_OFFSET, "sphere_effect_kill"),
        (sites_hitbox::HITBOX_IS_EXIST_OFFSET, "sphere_effect_is_exist"),
        (sites_hitbox::HITBOX_RGB_LAST_OFFSET, "effect_set_rgb_partial_last"),
        (sites_hitbox::HITBOX_RATE_LAST_OFFSET, "effect_set_rate_last"),
        (sites_hitbox::HITBOX_SLEEP_EFFECT_OFFSET, "motion_animcmd_set_sleep_effect"),
        (sites_hitbox::HITBOX_ACCESSOR_OFFSET, "sv_battle_object_module_accessor"),
        (sites_hitbox::HITBOX_ATTACK_STACK_OFFSET, "hitbox_attack_stack"),
        (sites_hitbox::HITBOX_CATCH_STACK_OFFSET, "hitbox_catch_stack"),
        (sites_hitbox::HITBOX_HURT_MODULE_OFFSET, "hurtbox_module"),
        (sites_hitbox::HITBOX_HURT_GROUPS_OFFSET, "hurtbox_groups"),
        (sites_hitbox::HITBOX_HURT_PART_OFFSET, "hurtbox_part_shape"),
        (sites_hitbox::ORB_JOINT_POSITION_OFFSET, "orb_joint_position"),
        (sites_hitbox::ORB_WORLD_TO_SCREEN_OFFSET, "orb_world_to_screen"),
        (sites_hitbox::ORB_SCREEN_SIZE_OFFSET, "orb_screen_size"),
    ] {
        assert_eq!(model::sites_hitbox::SITES.iter().find(|s| s.name == name).map(|s| s.offset), Some(offset), "{name}");
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
        ("effect_req_follow", "_ZN3app8lua_bind29EffectModule__req_follow_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40ES4_RKNS3_8Vector3fES7_fbjiiiibb"),
        ("sphere_effect_kill", "_ZN3app8lua_bind23EffectModule__kill_implEPNS_26BattleObjectModuleAccessorEjbb"),
        ("sphere_effect_is_exist", "_ZN3app8lua_bind34EffectModule__is_exist_effect_implEPNS_26BattleObjectModuleAccessorEj"),
        ("effect_set_rgb_partial_last", "_ZN3app8lua_bind39EffectModule__set_rgb_partial_last_implEPNS_26BattleObjectModuleAccessorEfff"),
        ("effect_set_rate_last", "_ZN3app8lua_bind32EffectModule__set_rate_last_implEPNS_26BattleObjectModuleAccessorEf"),
        ("motion_animcmd_set_sleep_effect", "_ZN3app8lua_bind42MotionAnimcmdModule__set_sleep_effect_implEPNS_26BattleObjectModuleAccessorEb"),
        ("sv_battle_object_module_accessor", "_ZN3app9sv_system29battle_object_module_accessorEP9lua_State"),
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
    // A capsule part: both centres, its size and its bone; drawn yellow with the same sphere rule as a hitbox.
    let capsule = hitbox::hurtbox_shape(&part(0, [0.0, 1.0, 0.0], [0.0, 6.0, 0.0], 2.5, 0x0000_0004_1234_5678)).unwrap();
    assert_eq!((capsule.joint, capsule.size, capsule.a, capsule.b), (0x0000_0004_1234_5678, 2.5, [0.0, 1.0, 0.0], Some([0.0, 6.0, 0.0])));
    assert_eq!(hitbox::spheres(capsule.a, capsule.b, capsule.size).count(), hitbox::sphere_count(5.0, 2.5));
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
    // The tint: yellow, fully saturated.
    assert_eq!(hitbox::HURTBOX_TINT, [1.0, 1.0, 0.0]);
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
        assert_eq!(sphere.tint, hitbox::HURTBOX_TINT, "yellow");
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
        fn read_u64(&mut self, _at: usize) -> u64 {
            7
        }
        fn read_f32(&mut self, at: usize) -> f32 {
            match at {
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
    // Shielding and a grab status clear the whole set (hurt spheres included), like the hit spheres; the game's own shield
    // bubble is never owned. Off clears too (facts the plan already covers).
    for (enabled, status) in [(true, ids().guard_on), (true, ids().catch), (false, 0)] {
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
    // with it Off — and on the stop paths it takes first (shielding, a grab status) — the reader is never touched:
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
    // Shielding and grabbing (the stop paths the dispatch takes before the reader): no HitModule read either.
    for status in [ids().guard_on, ids().catch] {
        let mut set = hitbox::HandleSet::new();
        let mut fx = RequestSpy::new();
        let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
        let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, true, false, status), &mut wants, &mut NoHit, Some(&mut hurt), &mut fx);
        assert_eq!((counts.spawned, len), (0, 0));
        assert_eq!(hurt.module_calls, 0, "status {status}: no HitModule read on a stop path");
    }
    // On, ordinary status: the module is read and the part's capsule drawn (one part, centres 4 apart at size 2:
    // ceil(4 / 3.5) + 1 = 3 spheres).
    let (counts, len) = hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut NoHit, Some(&mut hurt), &mut fx);
    assert_eq!((counts.spawned, len), (3, 3));
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
    assert_eq!((len, left), (4, 0), "the capsule's three spheres (4 apart at size 2) and the point part's one; the off part nothing");
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
    assert_eq!((out[0].pos, out[1].pos, out[2].pos, out[3].pos), ([0.0, 1.0, 0.0], [0.0, 3.0, 0.0], [0.0, 5.0, 0.0], [1.0, 2.0, 3.0]));
    assert_eq!(out[3].joint, 7);
    for sphere in &out[..len] {
        assert_eq!(sphere.tint, hitbox::HURTBOX_TINT, "yellow");
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
    // The want scratch is sized for the hit slots' and the hurt parts' spheres together.
    assert_eq!(hitbox::WANT_SCRATCH, hitbox::SLOTS as usize * hitbox::SPHERE_MAX + hitbox::MAX_HURTBOX_PARTS as usize * hitbox::SPHERE_MAX);
    const { assert!(hitbox::WANT_SCRATCH >= hitbox::MAX_HELD) };
}

#[test]
fn core06_release_reads_the_hit_module_only_while_the_setting_is_on() {
    // Route B (operator decision 2026-11-04, `docs/DECISIONS.md` HB1-route, `review/menu-proposal` `5ca5e3b`): the
    // shipped build reads the HitModule only while the setting is On. The runtime's only reader construction sits in
    // `frame`, behind `enabled` (`enabled.then(|| GameHurt …)`), and the dispatch's stop paths (not a practice fighter,
    // shielding, grab, Off) return before any part is read; the diag census keeps its own reader inside the diag-gated
    // `stats` module. The built release NRO is re-checked at the gates (no census string in it; the diag NRO carries the
    // line), and the model tests above pin that a disabled or stop-path dispatch touches no HitModule.
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
    // The reader and its raw read exist in the shipped build — but only where the pass runs them.
    for name in ["GameHurt", "mem_load"] {
        let found = refs(name);
        assert!(!found.is_empty(), "{name} must appear in the runtime file");
        for &i in &found {
            let definition = toks[..i].iter().rev().take(3).any(|t| matches!(t, callgraph::Tok::Ident(s) if s == "fn"));
            assert!(definition || inside_frame(i) || inside_stats(i), "{name} at token {i} is outside `frame` and the diag census");
        }
    }
    // The construction is behind the setting: `enabled` names itself within the few tokens before `GameHurt`.
    for &i in &refs("GameHurt") {
        if inside_frame(i) {
            assert!(
                toks[i.saturating_sub(10)..i].iter().any(|t| matches!(t, callgraph::Tok::Ident(s) if s == "enabled")),
                "the pass's reader at token {i} is not gated on `enabled`"
            );
        }
    }
    // The census stays diag-only: its walk and its window type are named only inside the gated module.
    for name in ["hurt_parts", "HurtCensus"] {
        let found = refs(name);
        assert!(!found.is_empty(), "{name} must appear in the runtime file");
        assert!(found.iter().all(|&i| inside_stats(i)), "{name} is named outside the diag-gated stats module");
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
    fn sphere_spawn(&mut self, s: &hitbox::Sphere) -> u32 {
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
        match at { 0x1048 => 7, 0x2048 => 9, _ => panic!("non-attack node read {at:#x}") }
    }

    fn read_f32(&mut self, at: usize) -> f32 {
        self.reads.push(at);
        match at {
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
    fn sphere_spawn(&mut self, value: &hitbox::Sphere) -> u32 {
        self.requests.push(*value);
        hitbox::Effects::sphere_spawn(&mut self.fx, value)
    }
}

#[test]
fn core06_rc_dispatch_reads_only_attacks_and_requests_hits_and_grabs() {
    let mut snapshot = model::settings::Snapshot::factory();
    snapshot.set("hitbox_vis", &[1, 0]).unwrap();
    let mut set = hitbox::HandleSet::new();
    let grab = sphere(hitbox::sphere_key(hitbox::KIND_GRAB, 2, 0), 4.0);
    assert!(set.push_grab(grab));
    // Poison all unused scratch entries with valid-looking HB1 requests: the
    // production dispatcher must neither retain nor submit those stale values.
    let poison = sphere(hitbox::sphere_key(hitbox::KIND_HURT, 0, 0), 9.0);
    let mut wants = [poison; hitbox::MAX_HELD];
    let mut reads = AttackSpy::default();
    let mut fx = RequestSpy::new();
    let facts = facts(true, hitbox::enabled(&snapshot), false, 0);
    let (counts, hits) = hitbox::reconcile_frame(&mut set, facts, &mut wants, &mut reads, None, &mut fx);
    assert_eq!((counts.spawned, counts.failed, counts.dropped, hits), (6, 0, 0, 5));
    assert_eq!(reads.active, (0..8).collect::<Vec<_>>());
    assert_eq!(reads.data, [1, 5, 6]);
    assert_eq!(reads.reads, [0x1048, 0x1024, 0x1000, 0x1004, 0x1008, 0x1010, 0x1014, 0x1018,
                            0x2048, 0x2024, 0x2000, 0x2004, 0x2008, 0x2010, 0x2014, 0x2018]);
    let expected_keys: Vec<_> = std::iter::once(hitbox::sphere_key(hitbox::KIND_HIT, 1, 0))
        .chain((0..4).map(|i| hitbox::sphere_key(hitbox::KIND_HIT, 5, i))).chain([grab.key]).collect();
    assert_eq!(fx.requests.iter().map(|s| s.key).collect::<Vec<_>>(), expected_keys);
    assert_eq!(fx.requests[0], hitbox::Sphere { key: expected_keys[0], joint: 7, pos: [1.0, 0.0, 0.0], scale: 0.19, tint: hitbox::hitbox_tint(1) });
    assert_eq!(fx.requests[1].joint, 9);
    assert_eq!(fx.requests[1].pos, [0.0; 3]);
    assert_eq!(fx.requests[4].pos, [10.0, 0.0, 0.0]);
    assert_eq!(fx.requests[4].tint, hitbox::hitbox_tint(5));
    assert_eq!(fx.requests[5], grab);
    assert!(fx.requests.iter().all(|s| s.key >> 24 != hitbox::KIND_HURT));
    let (counts, hits) = hitbox::reconcile_frame(&mut set, facts, &mut wants, &mut reads, None, &mut fx);
    assert_eq!((counts.spawned, counts.kept, hits), (0, 6, 5), "unchanged owned handles are retained");
    assert_eq!(fx.requests.len(), 6);
}

#[test]
fn core06_rc_dispatch_preserves_grab_shield_off_and_retirement_cleanup() {
    for (visualizes, enabled, status, keep_grab) in [(true, true, 62, true), (true, true, 30, false), (true, false, 0, false), (false, true, 0, false)] {
        let mut set = hitbox::HandleSet::new();
        let mut fx = RequestSpy::new();
        // A normal game shield (or another plugin's effect) is never owned.
        fx.fx.live.push(900);
        let grab = sphere(hitbox::sphere_key(hitbox::KIND_GRAB, 2, 0), 4.0);
        set.push_grab(grab);
        let mut wants = [hitbox::Sphere::NONE; hitbox::MAX_HELD];
        hitbox::reconcile_frame(&mut set, facts(true, true, false, 0), &mut wants, &mut AttackSpy::default(), None, &mut fx);
        let mut reads = AttackSpy::default();
        let requests = fx.requests.len();
        let (counts, hits) = hitbox::reconcile_frame(&mut set, facts(visualizes, enabled, false, status), &mut wants, &mut reads, None, &mut fx);
        assert_eq!((reads.active.len(), reads.data.len(), reads.reads.len(), hits), (0, 0, 0, 0));
        assert_eq!(fx.requests.len(), requests, "no attack/HB1/ordinary shield request on stop path");
        assert_eq!((counts.killed, counts.kept), if keep_grab { (5, 1) } else { (6, 0) });
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
    assert!(set.push_grab(sphere(sphere_key(KIND_GRAB, 0, 0), 1.0)));
    assert!(set.push_grab(sphere(sphere_key(KIND_GRAB, 0, 1), 2.0)));
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
        assert!(set.push_grab(sphere(sphere_key(KIND_GRAB, 0, 0), x)));
        set.reconcile(&[], true, &mut fx);
    }
    assert_eq!(set.len(), 2, "the surviving grab sphere and the one of key (grab 0, sphere 0)");
    // Off / shield / rebound / not practice: every held sphere is killed by its handle, the queue is dropped.
    assert!(set.push_grab(sphere(sphere_key(KIND_GRAB, 1, 0), 3.0)));
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
    // push_shape: a capsule's spheres in order, keyed per sphere, scaled and tinted; what does not fit is counted.
    let shape = hitbox::Shape { id: 2, joint: 9, size: 2.0, a: [0.0; 3], b: Some([10.0, 0.0, 0.0]) };
    let mut out = [hitbox::Sphere::NONE; 3];
    let mut len = 0;
    let left = hitbox::push_shape(&mut out, &mut len, KIND_HIT, 2, &shape, hitbox::hitbox_tint(2));
    assert_eq!((len, left), (3, 1), "4 spheres for 10 units at size 2, one left out");
    assert_eq!(out[1].key, sphere_key(KIND_HIT, 2, 1));
    assert!((out[0].scale - 0.19).abs() < 1e-6);
    // The table: one set per fighter; a new fighter takes a free set, then the least recently used one, forgotten.
    let mut table = SetTable::new();
    let first = table.index_for(0x1000);
    assert_eq!(table.index_for(0x1000), first);
    for k in 1..FIGHTER_SETS {
        table.index_for(0x1000 + k * 0x100);
    }
    table.sets[first].push_grab(hitbox::Sphere::NONE);
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
