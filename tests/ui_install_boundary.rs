//! R3 installation-spy contract, shared with the actual native anchor installer.
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

#[test]
fn every_renderer_only_guard_can_fail_without_disabling_native_pause() {
    for rejected in model::sites_ui::UI_SITES.iter().map(|s| s.name)
        .filter(|name| !model::readiness::NATIVE_PAUSE_ROWS.contains(name)) {
        let mut installed = 0;
        let ready = model::readiness::install_pause_observer(|name| name != rejected, || installed += 1);
        assert!(ready, "renderer-only guard {rejected}");
        assert_eq!(installed, 1);
    }
}

#[test]
fn rejecting_each_actual_observer_guard_performs_no_installation() {
    for rejected in model::readiness::NATIVE_PAUSE_ROWS {
        let mut installed = false;
        assert!(!model::readiness::install_pause_observer(|name| name != rejected, || installed = true));
        assert!(!installed, "never bypass {rejected}");
    }
}

#[test]
fn boot_binds_delay_to_observer_before_the_independent_renderer_closure() {
    let boot = include_str!("../src/runtime/boot.rs");
    let observer = boot.find("let native_pause_on = ui::anchors::install(&usable);").unwrap();
    let delay = boot.find("if !native_pause_on").unwrap();
    let renderer = boot.find("if capabilities::unit_usable(Feature::Ui, &usable)").unwrap();
    assert!(observer < delay && delay < renderer);
    assert!(boot[delay..renderer].contains("native_pause_observer_unavailable"));
    let installer = include_str!("../src/runtime/ui/anchors.rs");
    assert!(installer.contains("crate::model::readiness::install_pause_observer("));
    assert_eq!(installer.matches("skyline::install_hook!(ui_pass_calculate)").count(), 1);
    assert_eq!(installer.matches("skyline::install_hook!(ui_register_calculate)").count(), 1);
    assert!(boot.contains("ui::install(&*loaded, input_delay_on, ui_found)"));
    assert!(!include_str!("../src/runtime/ui/mod.rs").contains("site.outcome(&game::read_window(site.offset))"));
}

#[test]
fn actual_guard_receipt_survives_our_anchor_writes_without_accepting_foreign_bytes() {
    use model::sites_ui::UI_SITES;
    let mut windows: Vec<_> = UI_SITES.iter().map(|s| s.guard).collect();
    let preflight: Vec<_> = UI_SITES.iter().zip(&windows).map(|(s,w)| s.outcome(w)).collect();
    let before = |name: &str| name == "is_training_mode" || UI_SITES.iter().position(|s| s.name == name).is_some_and(|i| preflight[i].usable());
    assert!(model::readiness::install_pause_observer(before, || {
        for name in ["ui_pass_calculate", "ui_register_calculate"] {
            let i = UI_SITES.iter().position(|s| s.name == name).unwrap();
            windows[i][..4].copy_from_slice(&0x14000400u32.to_le_bytes());
        }
    }));
    // Negative control reproduces the rejected post-write re-read from f9860f2.
    let reread = |name: &str| UI_SITES.iter().position(|s| s.name == name).is_some_and(|i| UI_SITES[i].outcome(&windows[i]).usable());
    assert!(!model::readiness::renderer_anchors_ready(reread, true, true, true));
    assert!(model::readiness::renderer_anchors_ready(before, true, true, true));
    for rejected in ["ui_archive_alloc", "ui_set_text", "ui_pass_calculate", "ui_register_calculate"] {
        assert!(!model::readiness::renderer_anchors_ready(|name| before(name) && name != rejected, true, true, true));
    }
    assert!(!model::readiness::renderer_anchors_ready(before, true, false, true));
}
