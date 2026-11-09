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

/// Deviations from the shipped closure. None since decision L1 was answered (2026-11-02): `deluxe-sdk` is back in `default` with the
/// MIT SDK (rev 20dd1cb, vendored under third_party/). A new deviation is added here with its decision.
const PENDING_DEVIATIONS: &[&str] = &[];

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
fn the_shipped_closure_is_the_m5o_closure_and_deluxe_sdk_pulls_the_vendored_mit_sdk() {
    // Explicit on purpose: a new deviation must change this test.
    assert!(PENDING_DEVIATIONS.is_empty());
    let map = features();
    let mut shipped = closure(&map, &["default"]);
    shipped.remove("default");
    assert_eq!(shipped, closure(&map, RELEASE), "the default closure is the M5o release closure");
    assert!(shipped.contains("deluxe-sdk"));
    assert_eq!(map.get("deluxe-sdk"), Some(&vec!["dep:deluxe-plugin-sdk".to_string()]));
    let dependency = MANIFEST
        .lines()
        .find(|l| l.trim_start().starts_with("deluxe-plugin-sdk"))
        .expect("the SDK is a dependency");
    assert!(
        dependency.contains("path = \"third_party/deluxe-plugin-sdk/sdk\"") && dependency.contains("optional = true"),
        "the vendored path crate, optional behind deluxe-sdk: {dependency}"
    );
    for crate_manifest in [
        include_str!("../third_party/deluxe-plugin-sdk/sdk/Cargo.toml"),
        include_str!("../third_party/deluxe-plugin-sdk/abi/Cargo.toml"),
    ] {
        assert!(crate_manifest.contains("version = \"5.0.0\"") && crate_manifest.contains("license = \"MIT\""), "{crate_manifest}");
    }
    assert!(include_str!("../third_party/deluxe-plugin-sdk/LICENSE").starts_with("MIT License\n\nCopyright (c) 2026 intpa\n"));
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
    let mut checked = BTreeSet::new();
    for name in map.keys().filter(|n| is_diagnostic_name(n)) {
        let reaches_diag = closure(&map, &[name.as_str()]).contains("diag");
        if PRODUCT_DESPITE_NAME.contains(&name.as_str()) {
            assert!(!reaches_diag, "{name} is a product feature");
        } else {
            assert!(reaches_diag, "{name} must require diag");
            checked.insert(name.as_str());
        }
    }
    // Explicit retained inventory: approved probe retirements must not erase unresolved diagnostics.
    let expected = BTreeSet::from([
        "console-practice-trace",
        "console-swap-trace",
        "hang-trace",
        "mii-ldn-qualification-probe",
        "named-input-trace",
        "native-unlink-probe",
        "plugin-heap-meter",
        "texture-probe",
    ]);
    assert_eq!(checked, expected);
}

#[test]
fn the_renamed_product_features_and_their_old_names() {
    let map = features();
    for (product, old) in [("lobby-stage", "lobby-stage-probe"), ("practice-stage", "practice-stage-observer")] {
        assert!(!closure(&map, &[product]).contains("diag"), "{product} is product code");
        assert!(!map.contains_key(old), "{old} was an empty diagnostic alias retired after its probe served");
    }
    assert!(closure(&map, &["practice-stage-request-alpha"]).contains("practice-stage"));
    assert!(closure(&map, &["training-stage-rotation-alpha"]).contains("lobby-stage"));
}

/// Lead ruling 2026-11-02: one global allocator. The fork's `console-practice-trace`, `native-unlink-probe` and `plugin-heap-meter` each
/// declared their own, which cannot coexist with the plugin's counting allocator; their features read the byte meter of
/// `model::hook_stats` instead. The only `#[global_allocator]` items under `src/` are the two cfg-exclusive statics of
/// `runtime/alloc.rs` (the counting allocator, or the recorder's forwarding `TraceAllocator` when `console-practice-trace` is on).
#[test]
fn the_plugin_has_one_global_allocator() {
    fn walk(dir: &std::path::Path, hits: &mut Vec<(String, usize)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, hits);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                let n = text.lines().filter(|l| l.trim_start().starts_with("#[global_allocator]")).count();
                if n > 0 {
                    hits.push((path.to_string_lossy().replace('\\', "/"), n));
                }
            }
        }
    }
    let mut hits = Vec::new();
    walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut hits);
    assert_eq!(hits.len(), 1, "global allocators: {hits:?}");
    assert!(hits[0].0.ends_with("src/runtime/alloc.rs") && hits[0].1 == 2, "{hits:?}");
    let alloc = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime/alloc.rs")).unwrap();
    assert!(alloc.contains("#[cfg(not(feature = \"console-practice-trace\"))]\n#[global_allocator]"), "exclusive with the trace allocator");
    assert!(alloc.contains("#[cfg(feature = \"console-practice-trace\")]\n#[global_allocator]"));
}

/// `tools/feature_matrix.py` is the CI-style build check (decision D11: every feature must build); its list covers every feature of
/// the table in the `each` and `alone` sets and the allocator features together.
#[test]
fn the_feature_matrix_lists_every_feature() {
    let root = env!("CARGO_MANIFEST_DIR");
    let out = match std::process::Command::new("python3").current_dir(root).args(["-I", "tools/feature_matrix.py", "--list", "--set", "all"]).output() {
        Ok(out) => out,
        Err(_) => return, // no python on this host
    };
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let listing = String::from_utf8_lossy(&out.stdout);
    for name in features().keys() {
        assert!(listing.contains(&format!("default+{name} --features {name}\n")), "{name} missing from the `each` set");
        assert!(listing.contains(&format!("alone:{name} --no-default-features --features {name}\n")), "{name} missing from the `alone` set");
    }
    assert!(listing.contains("--features diag,console-practice-trace,native-unlink-probe,plugin-heap-meter"));
}

/// Both mutually exclusive observer experiments have been retired. There must
/// be no stale matrix skip and no source exclusion left without a matrix row.
#[test]
fn the_matrix_skips_exactly_the_forks_compile_error_exclusions() {
    fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                for line in std::fs::read_to_string(&path).unwrap().lines() {
                    if let Some(rest) = line.trim_start().strip_prefix("compile_error!(\"") {
                        out.push(rest.split('"').next().unwrap().to_string());
                    }
                }
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut messages = Vec::new();
    walk(&root.join("src"), &mut messages);
    let tool = std::fs::read_to_string(root.join("tools/feature_matrix.py")).unwrap();
    assert!(messages.is_empty(), "new source exclusion needs a reviewed matrix rule: {messages:?}");
    assert!(tool.contains("EXCLUSIONS = []"), "retired experiments must not leave matrix skips");
}

#[test]
fn rc_retired_observation_features_and_modules_cannot_be_reenabled() {
    let map = features();
    for feature in ["source-owner-probe", "mii-origin-observer", "mii-pool-observer",
        "native-body-bank-probe", "automation-results-observer"] {
        assert!(!map.contains_key(feature), "retired RC feature {feature}");
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in ["src/runtime/roster/source_owner_probe.rs", "src/runtime/roster/mii_origin_observer.rs",
        "src/runtime/roster/mii_pool_observer.rs", "src/runtime/roster/body_bank_probe.rs",
        "src/model/mii_root/mii_pool_observer_policy.rs", "src/runtime/lobby/automation_helper/observer.rs",
        "src/runtime/hitbox_observer.rs", "src/model/hitbox_observation.rs",
        "src/model/seams/native_seams/rows_hitbox_observer.rs"] {
        assert!(!root.join(path).exists(), "retired module {path}");
    }
    // Option 3 was not approved. Keep historical tools and the product pilot alias.
    for feature in ["console-practice-trace", "console-swap-trace", "hang-trace",
        "native-unlink-probe", "native-unlink-guard", "plugin-heap-meter", "cpu-pilot-probe"] {
        assert!(map.contains_key(feature), "retained feature {feature}");
    }
}

/// `<crate>/<feature>` on one of this repository's path crates (`crates/<crate>`): the feature is declared in that crate's
/// `[features]` (`hang-trace` turns on `hos-lock/owners`, the diag stall watchdog's lock table).
fn path_crate_feature_declared(dep: &str) -> bool {
    let Some((krate, feature)) = dep.split_once('/') else { return false };
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates").join(krate).join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(manifest) else { return false };
    let mut inside = false;
    text.lines().any(|line| {
        if line.starts_with('[') {
            inside = line.trim() == "[features]";
            return false;
        }
        inside && line.split_once('=').is_some_and(|(name, _)| name.trim() == feature)
    })
}

#[test]
fn every_feature_dependency_is_declared() {
    let map = features();
    for (name, deps) in &map {
        // `dep:<crate>` names an optional dependency, not a feature (the manifest test above checks `deluxe-sdk`'s).
        for d in deps.iter().filter(|d| !d.starts_with("dep:")) {
            assert!(map.contains_key(d) || path_crate_feature_declared(d), "{name} requires undeclared feature {d}");
        }
    }
    assert!(path_crate_feature_declared("hos-lock/owners") && !path_crate_feature_declared("hos-lock/no-such"));
}
