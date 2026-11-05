//! The runtime wiring of the rc.3 implementation review's fixes (2026-11-05, `artifacts/phase23/LANE-SURFACE-REPORT.md`
//! "rc.3 implementation review"). The runtime is Switch-only, so the call-graph scanner (`tests/common/callgraph.rs`)
//! reads it: each test names the hook or thread that must reach the host-tested model logic, so a refactor that drops
//! the call fails here. The logic itself is tested in `frame_strip_tests.rs`, `tests/core06_orbs.rs`,
//! `src/model/ui/adapter.rs` and `tests/menu_nav.rs`.

#[path = "common/callgraph.rs"]
mod callgraph;

use std::path::Path;

use callgraph::{Graph, Tok};

fn graph() -> Graph {
    Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[])
}

fn one(g: &Graph, key: &str) -> usize {
    let found = g.find(key);
    assert_eq!(found.len(), 1, "{key}: {found:?}");
    found[0]
}

fn calls(g: &Graph, from: &str, to: &str) -> bool {
    let target = one(g, to);
    g.callees(one(g, from)).contains(&target)
}

/// Whether `key`'s body names `ident`.
fn names(g: &Graph, key: &str, ident: &str) -> bool {
    g.funcs[one(g, key)].body.iter().any(|t| matches!(t, Tok::Ident(s) if s == ident))
}

/// IV-4: the fighter on-init wrapper (a Training reset, a CPU replaced) clears the strip's tracker.
#[test]
fn iv4_a_fighter_initialised_again_clears_the_strip() {
    let g = graph();
    assert!(calls(&g, "reset_hub::on_init", "input_display::fighter_initialised"));
    assert!(names(&g, "input_display::fighter_initialised", "TRACKER") && names(&g, "input_display::fighter_initialised", "clear"));
}

/// IV-2: the strip owns the frame advantage only with its panes: the HUD draw reports them, `sampling` asks the model
/// rule with them, and both the advantage notice and the strip's sampling go through `sampling`.
#[test]
fn iv2_the_strip_takes_the_advantage_only_with_its_panes() {
    let g = graph();
    assert!(calls(&g, "hud_draw::draw", "input_display::set_panes"));
    assert!(names(&g, "input_display::sampling", "strip_owns_advantage") && names(&g, "input_display::sampling", "PANES"));
    assert!(calls(&g, "cpu::frame_advantage", "input_display::sampling"));
    assert!(calls(&g, "input_display::sample_frame", "input_display::sampling"));
}

/// HB-1: the orb draw's inactive branch writes the hiding tick before it returns, and the active branch uses the same
/// plan (`orbs::pane_tick`).
#[test]
fn hb1_an_inactive_layout_tick_hides_the_orbs() {
    let g = graph();
    assert!(calls(&g, "hitbox::draw_orbs", "hitbox::write_tick"));
    let body = &g.funcs[one(&g, "hitbox::draw_orbs")].body;
    let ident = |k: usize, s: &str| matches!(body.get(k), Some(Tok::Ident(t)) if t == s);
    let inactive = (0..body.len())
        .find(|&k| body[k] == Tok::Punct('!') && ident(k + 1, "active") && body.get(k + 2) == Some(&Tok::Punct('{')))
        .expect("the `if !active {` branch");
    let ret = (inactive..body.len()).find(|&k| ident(k, "return")).expect("its return");
    assert!((inactive..ret).any(|k| ident(k, "write_tick")), "the inactive branch hides before it returns");
    assert!((inactive..ret).any(|k| ident(k, "pane_tick")));
    assert_eq!((0..body.len()).filter(|&k| ident(k, "pane_tick")).count(), 2, "both branches plan through pane_tick");
    assert!(names(&g, "hitbox::draw_orbs", "in_draw_order"), "the panes take orbs in the cut order (HB-2)");
}

/// MN-1: the worker publishes where jumps are after every event, and the input hook hands it to the adapter.
#[test]
fn mn1_the_input_hook_asks_the_worker_where_jumps_are() {
    let g = graph();
    assert!(names(&g, "input::on_p1_poll", "set_jumps") && names(&g, "input::on_p1_poll", "JUMPS"));
    assert!(names(&g, "worker::publish_jumps", "takes_jumps") && names(&g, "worker::publish_jumps", "JUMPS"));
    for from in ["worker::handle", "worker::sync_input_generation"] {
        assert!(calls(&g, from, "worker::publish_jumps"), "{from}");
    }
}

/// MN-4: the worker saves and restores the place only through the host-tested keeper.
#[test]
fn mn4_the_worker_saves_and_restores_through_the_keeper() {
    let g = graph();
    assert!(names(&g, "worker::sync_input_generation", "drop_model"));
    assert!(names(&g, "worker::handle", "keeper") && names(&g, "worker::handle", "open"));
    let worker: Vec<usize> = (0..g.funcs.len()).filter(|&i| g.funcs[i].file == "ui/worker.rs").collect();
    assert!(!worker.is_empty());
    for i in worker {
        let f = &g.funcs[i];
        assert!(!f.body.iter().any(|t| matches!(t, Tok::Ident(s) if s == "restore_place")), "{} restores on its own", f.key());
    }
}
