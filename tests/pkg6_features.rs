//! The cargo feature structure of the carried fork code (decision D11): `default` is the shipped closure, no
//! diagnostic feature is in it, every probe/observer/trace/meter requires `diag`. Ported from the fork's
//! `training_mod_consts/tests/diag_features.rs` (ours-new, stage m5o), which reads its crate's manifest the same way.

use std::collections::{BTreeMap, BTreeSet};

const MANIFEST: &str = include_str!("../Cargo.toml");

/// The M5o release feature list (stages/m5o RECEIPTS-m5o1.json, `m5o1-switch-root-full-release`).
const RELEASE: &[&str] = &[
    "enhancements-alpha",
    "native-unlink-guard",
    "deluxe-sdk",
    "mii-hud-portrait-alpha",
    "cycle-preload-extended",
    "lobby-preload-extended",
    "native-body-bank-alpha",
];

/// PENDING DEVIATION from the shipped closure (operator decision L1, parked by the lead 2026-11-02): `deluxe-sdk` is left out of
/// `default` until the Deluxe plugin SDK repository states a license (D3: a crate without a license is not added). The feature stays
/// declared (the table is the fork's); it has no `dep:` entry and nothing to compile yet. Remove it from this list and put it back in
/// `default` when L1 is answered.
const PENDING_DEVIATIONS: &[&str] = &["deluxe-sdk"];

/// Product features kept under a diagnostic-looking name (external boundary).
const PRODUCT_DESPITE_NAME: &[&str] = &["cpu-pilot-probe"];

fn features() -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::new();
    let mut inside = false;
    for line in MANIFEST.lines() {
        if line.starts_with('[') {
            inside = line.trim() == "[features]";
            continue;
        }
        let line = line.trim();
        if !inside || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line.split_once('=').expect("one-line feature");
        let value = value.trim();
        assert!(value.starts_with('[') && value.ends_with(']'), "one-line feature: {line}");
        let deps = value[1..value.len() - 1]
            .split(',')
            .map(|d| d.trim().trim_matches('"').to_string())
            .filter(|d| !d.is_empty())
            .collect();
        map.insert(name.trim().to_string(), deps);
    }
    map
}

fn closure(map: &BTreeMap<String, Vec<String>>, roots: &[&str]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack: Vec<String> = roots.iter().map(|r| r.to_string()).collect();
    while let Some(f) = stack.pop() {
        if !seen.insert(f.clone()) {
            continue;
        }
        if let Some(deps) = map.get(&f) {
            stack.extend(deps.iter().filter(|d| !d.contains('/') && !d.contains(':')).cloned());
        }
    }
    seen
}

fn is_diagnostic_name(name: &str) -> bool {
    name.ends_with("-probe") || name.ends_with("-observer") || name.ends_with("-trace") || name == "plugin-heap-meter"
}

#[test]
fn default_features_are_the_shipped_release_list_minus_the_pending_deviations() {
    let map = features();
    let default = map.get("default").expect("a default feature list");
    let expected: Vec<String> =
        RELEASE.iter().filter(|f| !PENDING_DEVIATIONS.contains(f)).map(|s| s.to_string()).collect();
    assert_eq!(default, &expected, "`cargo skyline build --release` builds the shipped product except the pending deviations");
    for f in RELEASE {
        assert!(map.contains_key(*f), "release feature {f} is declared");
    }
}

#[test]
fn the_only_deviation_from_the_shipped_closure_is_deluxe_sdk_pending_l1() {
    // Explicit on purpose: a second deviation, or this one outliving L1, must change this test.
    assert_eq!(PENDING_DEVIATIONS, &["deluxe-sdk"]);
    let map = features();
    let shipped = closure(&map, &["default"]);
    assert!(!shipped.contains("deluxe-sdk"), "deluxe-sdk is not in the default closure while L1 is open: {shipped:?}");
    let full: BTreeSet<String> = closure(&map, RELEASE);
    let missing: Vec<&String> = full.difference(&shipped).collect();
    // `automation-results-native` is not in either closure; what differs is `deluxe-sdk` alone.
    assert_eq!(missing, vec![&"deluxe-sdk".to_string()], "the closure of the M5o list minus the default closure");
    assert!(map.get("deluxe-sdk").is_some_and(|deps| deps.is_empty()), "no dep: entry until the license is stated");
    assert!(
        !MANIFEST.lines().filter(|l| !l.trim_start().starts_with('#')).any(|l| l.contains("deluxe-plugin-sdk")),
        "the SDK crate is not a dependency"
    );
}

#[test]
fn diag_is_an_empty_umbrella() {
    assert_eq!(features().get("diag"), Some(&Vec::new()));
}

#[test]
fn the_shipped_closure_never_enables_diag() {
    let map = features();
    let shipped = closure(&map, &["default"]);
    assert!(!shipped.contains("diag"), "shipped closure: {shipped:?}");
    for f in &shipped {
        assert!(
            !is_diagnostic_name(f) || PRODUCT_DESPITE_NAME.contains(&f.as_str()),
            "diagnostic feature {f} in the shipped closure"
        );
    }
}

#[test]
fn every_probe_trace_observer_and_meter_requires_diag() {
    let map = features();
    let mut checked = 0;
    for name in map.keys().filter(|n| is_diagnostic_name(n)) {
        let reaches_diag = closure(&map, &[name.as_str()]).contains("diag");
        if PRODUCT_DESPITE_NAME.contains(&name.as_str()) {
            assert!(!reaches_diag, "{name} is a product feature");
        } else {
            assert!(reaches_diag, "{name} must require diag");
            checked += 1;
        }
    }
    assert!(checked >= 30, "only {checked} diagnostic features found");
}

#[test]
fn the_renamed_product_features_and_their_old_names() {
    let map = features();
    for (product, old) in [("lobby-stage", "lobby-stage-probe"), ("practice-stage", "practice-stage-observer")] {
        assert!(!closure(&map, &[product]).contains("diag"), "{product} is product code");
        let old_closure = closure(&map, &[old]);
        assert!(old_closure.contains(product) && old_closure.contains("diag"), "{old} = {product} + diag");
    }
    assert!(closure(&map, &["practice-stage-request-alpha"]).contains("practice-stage"));
    assert!(closure(&map, &["training-stage-rotation-alpha"]).contains("lobby-stage"));
}

#[test]
fn every_feature_dependency_is_declared() {
    let map = features();
    for (name, deps) in &map {
        for d in deps {
            assert!(map.contains_key(d), "{name} requires undeclared feature {d}");
        }
    }
}