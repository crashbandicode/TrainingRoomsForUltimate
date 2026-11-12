//! The cargo feature table (rc.10 S2, docs/DECISIONS.md "the product is not behind feature switches", which supersedes
//! D11): the product is always compiled; `default` adds the optional Deluxe SDK; everything else is `diag` and the
//! diagnostic family, never in a shipped build, each requiring `diag`.

use std::collections::{BTreeMap, BTreeSet};

const MANIFEST: &str = include_str!("../Cargo.toml");

/// The features every shipped build enabled until rc.10 S2. They are not features any more: their code is always compiled.
const FORMER_PRODUCT_FEATURES: &[&str] = &[
    "enhancements-alpha", "native-unlink-guard", "mii-hud-portrait-alpha", "cycle-preload-extended", "lobby-preload-extended",
    "native-body-bank-alpha", "managed-visible-grouped-source-alpha", "mii-ordinary-cpu-alpha", "managed-third-source-alpha",
    "mii-editor-alpha", "cpu-pilot-probe", "p1-cpu-control", "mii-managed-helper-alpha", "waiting-roster-reload",
    "practice-stage-request-alpha", "training-stage-rotation-alpha", "lobby-stage", "practice-stage", "training-stage-rotation",
    "mii-saved-alpha", "mii-ordinary-player-alpha", "mii-managed-source-coexist-alpha", "managed-sources",
    "managed-grouped-source-alpha", "managed-ldn-third-source-alpha",
];

/// Product features kept under a diagnostic-looking name. None since rc.10 S2 (`cpu-pilot-probe` was the one).
const PRODUCT_DESPITE_NAME: &[&str] = &[];

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
fn the_product_is_not_behind_feature_switches() {
    let map = features();
    assert_eq!(map.get("default"), Some(&vec!["deluxe-sdk".to_string()]), "the default build adds only the optional Deluxe SDK");
    for f in FORMER_PRODUCT_FEATURES {
        assert!(!map.contains_key(*f), "{f} is not a switch any more");
        assert!(map.values().all(|deps| !deps.iter().any(|d| d == f)), "no feature requires {f}");
    }
    // Explicit: a new feature must change this test.
    let names: BTreeSet<&str> = map.keys().map(String::as_str).collect();
    assert_eq!(names, BTreeSet::from([
        "console-practice-trace", "default", "deluxe-sdk", "diag", "hang-trace", "named-input-trace", "plugin-heap-meter",
        "seam-drill", "simulator",
    ]));
}

#[test]
fn the_shipped_closure_is_deluxe_sdk_which_pulls_the_vendored_mit_sdk() {
    let map = features();
    let mut shipped = closure(&map, &["default"]);
    shipped.remove("default");
    assert_eq!(shipped, BTreeSet::from(["deluxe-sdk".to_string()]));
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
        "hang-trace",
        "named-input-trace",
        "plugin-heap-meter",
    ]);
    assert_eq!(checked, expected);
}

#[test]
fn the_retired_aliases_of_product_code_stay_retired() {
    let map = features();
    for old in ["lobby-stage-probe", "practice-stage-observer"] {
        assert!(!map.contains_key(old), "{old} was an empty diagnostic alias retired after its probe served");
    }
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

/// `tools/feature_matrix.py` is the CI-style build check (every feature must build); its list covers every feature of
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
    assert!(listing.contains("--features diag,console-practice-trace,plugin-heap-meter"));
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
    // Option 3 was not approved. Keep the historical tools.
    // rc.10 S4 (operator 2026-11-12): the probes nothing used are deleted.
    for feature in ["console-swap-trace", "native-unlink-probe", "mii-ldn-qualification-probe", "texture-probe",
        "automation-results-native"] {
        assert!(!map.contains_key(feature), "deleted probe feature {feature}");
    }
    for feature in ["console-practice-trace", "hang-trace", "named-input-trace", "plugin-heap-meter"] {
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
