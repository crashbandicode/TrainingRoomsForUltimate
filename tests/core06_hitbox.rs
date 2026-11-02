//! SPEC-core-06, the hitbox visualizer: every model rule of the spec's §9 table by its planned name, the site rows against the
//! owned 13.0.5 ELF, and the facts the plugin relies on for the `AttackData` layout and the Lua stack, each shown by the
//! instruction that does the access in the game's own code (`docs/test-provenance/core-06.md` names the derivation).

mod common;

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
    // Three hooks (each its own row, installed independently), eight called natives, the accessor getter, two relied windows.
    let hooks: Vec<&str> = SITES.iter().filter(|s| matches!(s.kind, SeamKind::Hook { .. })).map(|s| s.name).collect();
    assert_eq!(hooks, sites_hitbox::HOOK_ROWS);
    assert_eq!(SITES.iter().filter(|s| matches!(s.kind, SeamKind::Called)).count(), 8);
    assert_eq!(SITES.iter().filter(|s| matches!(s.kind, SeamKind::Relied)).count(), 2);
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
    for shared in ["status_kind", "lua_const_lookup", "effect_set_visible_kind"] {
        let usable = |row: &model::sites::Site| row.name != shared;
        assert_eq!(capabilities::blocked_by(Feature::Hitbox, &usable), Some(shared), "{shared}");
    }
    // And the native-call layer grants only what passed: an unreachable row's function is not granted.
    // (Rows with the same bytes and kind of guard on one function pass and fail together: the state package's
    // `state_effect_kill_kind` names the same function as our `effect_kill_kind`.)
    let poisoned: Vec<&str> = capabilities::equivalent_rows(capabilities::find("effect_kill_kind").unwrap()).map(|r| r.name).collect();
    assert!(poisoned.len() >= 2 && poisoned.contains(&"effect_kill_kind"), "{poisoned:?}");
    let usable = |row: &model::sites::Site| !poisoned.contains(&row.name);
    let granted = capabilities::Granted::from_usable(&usable);
    assert!(!granted.allows(sites_hitbox::HITBOX_KILL_KIND_OFFSET));
    assert!(granted.allows(sites_hitbox::HITBOX_REQ_FOLLOW_OFFSET));
    // The offsets constants name the rows.
    for (offset, name) in [
        (sites_hitbox::HITBOX_ATTACK_OFFSET, "hitbox_attack"),
        (sites_hitbox::HITBOX_CATCH_OFFSET, "hitbox_catch"),
        (sites_hitbox::HITBOX_REBOUND_OFFSET, "hitbox_set_rebound"),
        (sites_hitbox::HITBOX_IS_ATTACK_OFFSET, "attack_is_attack"),
        (sites_hitbox::HITBOX_ATTACK_DATA_OFFSET, "attack_data"),
        (sites_hitbox::HITBOX_REQ_FOLLOW_OFFSET, "effect_req_follow"),
        (sites_hitbox::HITBOX_KILL_KIND_OFFSET, "effect_kill_kind"),
        (sites_hitbox::HITBOX_RGB_LAST_OFFSET, "effect_set_rgb_partial_last"),
        (sites_hitbox::HITBOX_RATE_LAST_OFFSET, "effect_set_rate_last"),
        (sites_hitbox::HITBOX_SLEEP_EFFECT_OFFSET, "motion_animcmd_set_sleep_effect"),
        (sites_hitbox::HITBOX_ACCESSOR_OFFSET, "sv_battle_object_module_accessor"),
        (sites_hitbox::HITBOX_ATTACK_STACK_OFFSET, "hitbox_attack_stack"),
        (sites_hitbox::HITBOX_CATCH_STACK_OFFSET, "hitbox_catch_stack"),
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
        ("effect_kill_kind", "_ZN3app8lua_bind28EffectModule__kill_kind_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40Ebb"),
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
