//! One wrapper per native (lead ruling 2026-11-02, after the package 2 merge): every game function the plugin calls through the
//! grant check is wrapped by exactly one function of the native-call layers (`runtime::game::natives`, `runtime::state_natives`).
//! Two wrappers of one address drift apart (defaults, argument order, fixed arguments) and the capability model cannot tell them
//! apart; the one kept is the `game::natives` one.

#[path = "common/callgraph.rs"]
mod callgraph;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// The modules that call game functions through an `at::<F>(offset)` grant check.
const LAYERS: [&str; 2] = ["src/runtime/game.rs", "src/runtime/state_natives.rs"];

/// `(function, offset constant)` for every line of a layer that calls or grants a native by an `*_OFFSET` constant.
fn wrapped(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if let Some(at) = line.find("fn ") {
            let name: String = line[at + 3..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                current = name;
            }
        }
        let calls = line.contains("at::<") || line.contains("granted(") || line.contains("transmute(text_base() + ");
        if !calls || current.is_empty() {
            continue;
        }
        for word in line.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
            if word.ends_with("_OFFSET") && word.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                out.push((current.clone(), word.to_owned()));
            }
        }
    }
    out
}

#[test]
fn no_native_address_is_wrapped_twice() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let offsets = callgraph::model_offsets(&root.join("src/model"));
    let mut by_address: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    let mut seen = 0;
    for layer in LAYERS {
        let text = fs::read_to_string(root.join(layer)).unwrap();
        for (function, constant) in wrapped(&text) {
            let address = *offsets.get(&constant).unwrap_or_else(|| panic!("{layer}::{function} names {constant}, which is not a model offset"));
            let who = format!("{layer}::{function}");
            let list = by_address.entry(address).or_default();
            if !list.contains(&who) {
                list.push(who);
                seen += 1;
            }
        }
    }
    assert!(seen > 50, "the scan found {seen} wrappers: the layers' shape changed, update the scan");
    let twice: Vec<String> = by_address.iter().filter(|(_, who)| who.len() > 1).map(|(a, who)| format!("{a:#x}: {}", who.join(", "))).collect();
    assert!(twice.is_empty(), "natives wrapped more than once (keep the runtime::game::natives one):\n{}", twice.join("\n"));
}

#[test]
fn the_scan_sees_a_duplicate() {
    let text = "pub unsafe fn a(x: usize) -> f32 {\n    at::<extern \"C\" fn(usize) -> f32>(s::POS_Y_OFFSET).map_or(0.0, |f| f(x))\n}\n\
                pub unsafe fn b(x: usize) -> f32 {\n    at::<extern \"C\" fn(usize) -> f32>(sites_pkg2::POS_Y_OFFSET).map_or(0.0, |f| f(x))\n}\n";
    assert_eq!(wrapped(text), vec![("a".to_owned(), "POS_Y_OFFSET".to_owned()), ("b".to_owned(), "POS_Y_OFFSET".to_owned())]);
}
