#[test]
fn release_dead_code_stays_removed() {
    let deleted = [
        ("practice_stage_request_policy::plan", include_str!("../src/model/stages/practice_stage_request_policy.rs"), "fn plan("),
        ("tech_adapter::decision_observer_ready", include_str!("../src/runtime/features/tech_adapter.rs"), "fn decision_observer_ready("),
        ("tech::observe", include_str!("../src/runtime/features/tech.rs"), "fn observe("),
        ("tech::guard_press", include_str!("../src/runtime/features/tech.rs"), "fn guard_press("),
        ("practice::panel_entry", include_str!("../src/runtime/lobby/training/practice.rs"), "fn panel_entry("),
        ("mii_replay::request_named", include_str!("../src/runtime/mii/mii_replay.rs"), "fn request_named("),
        ("mii_replay::request", include_str!("../src/runtime/mii/mii_replay.rs"), "fn request("),
        ("facts::publish_scene_facts", include_str!("../src/runtime/ui/facts.rs"), "fn publish_scene_facts("),
    ];
    let mut failures = Vec::new();
    for (name, source, declaration) in deleted {
        if source.contains(declaration) {
            failures.push(format!("{name} is still defined"));
        }
    }
    let cpu = include_str!("../src/runtime/features/cpu.rs");
    if !cpu.contains("#[cfg(feature = \"diag\")]\n    pub const TAG: usize = 22;") {
        failures.push("ev::TAG is not diag-only".to_owned());
    }
    if !cpu.contains("pub const COUNT: usize = 23;") {
        failures.push("ev::COUNT changed".to_owned());
    }
    let launch = include_str!("../src/runtime/features/tech_launch_native.rs");
    if !launch.contains("// The diag launcher (tech_spike) reads these payloads; the product launcher reads only Stop.\n#[cfg_attr(not(feature = \"diag\"), allow(dead_code))]\npub(super) enum Moment") {
        failures.push("Moment is missing its reason and narrow allow".to_owned());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
