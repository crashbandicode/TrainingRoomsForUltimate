//! The runtime wiring of shareable combo files (`p23/combo-share`, `docs/COMBOS.md`). The runtime is Switch-only, so
//! the call-graph scanner (`tests/common/callgraph.rs`) reads it: each test names the path that must reach the
//! host-tested model logic (`src/model/combo_record_share_tests.rs`, the cross-fighter test in `combo_tests.rs`).

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

/// "Recorded on X": the strip's per-frame sample reads P1's fighter with the existing guarded native (behind the CPU
/// core unit's gate, `tests/capabilities.rs`) and hands it to the helper, which keeps it; *Save last combo* writes its name into the new file. The helper's grading takes no
/// fighter (`Helper::step` gets the tracker and the settings only).
#[test]
fn the_strip_hands_p1s_fighter_to_the_save_and_not_to_the_grading() {
    let g = graph();
    assert!(calls(&g, "input_display::sample_frame", "combo::after_strip"));
    assert!(calls(&g, "input_display::sample_frame", "input_display::p1_fighter"));
    assert!(names(&g, "input_display::p1_fighter", "fighter_kind") && names(&g, "input_display::p1_fighter", "KIND_ON"));
    assert!(names(&g, "combo::after_strip", "P1_KIND"));
    assert!(names(&g, "combos::save", "p1_kind") && names(&g, "combos::save", "fighter_label"));
    assert!(names(&g, "combos::save", "save_new"), "a save never replaces a file");
    let step = g.funcs[one(&g, "combo::after_strip")].body.iter().skip_while(|t| !matches!(t, Tok::Ident(s) if s == "step"))
        .take_while(|t| !matches!(t, Tok::Punct(';'))).cloned().collect::<Vec<_>>();
    assert!(!step.iter().any(|t| matches!(t, Tok::Ident(s) if s.contains("kind"))), "the grading gets no fighter: {step:?}");
}

/// The list: every file in the folder is read at each opening (dropped-in files show), a bad file is named in one
/// notice, and the counts go to the exported trace function.
#[test]
fn the_list_names_bad_files_and_reports_its_counts() {
    let g = graph();
    assert!(names(&g, "combos::show_list", "list"));
    assert!(calls(&g, "combos::show_list", "combos::note_skipped"));
    assert!(names(&g, "combos::note_skipped", "post_skipped") && names(&g, "combos::note_skipped", "upsert"));
    assert!(calls(&g, "combos::load_target", "combos::note_skipped"), "an unreadable target is named too");
    assert!(names(&g, "combos::show_list", "listed") && names(&g, "combos::show_list", "shared"));
    assert!(calls(&g, "combo::listed", "combo::tr_combo_listed_v1"));
    assert!(names(&g, "combos::show_list", "label"), "rows show 'recorded on X'");
}

/// The trace functions stay exported, distinct and out of line (an Eden trace point names them).
#[test]
fn the_trace_functions_are_exported_with_tags_of_their_own() {
    let text = include_str!("../src/runtime/features/combo.rs");
    let mut tags = Vec::new();
    for name in ["tr_combo_link_v1", "tr_combo_attempt_v1", "tr_combo_saved_v1", "tr_combo_listed_v1", "tr_combo_fighter_v1"] {
        let at = text.find(&format!("pub extern \"C\" fn {name}(")).unwrap_or_else(|| panic!("{name} missing"));
        let head = &text[text[..at].rfind("\n\n").unwrap_or(0)..at];
        assert!(head.contains("#[no_mangle]") && head.contains("#[inline(never)]"), "{name}: {head}");
        let body = &text[at..at + text[at..].find("\n}").unwrap()];
        let tag = body.split("black_box((").nth(1).and_then(|rest| rest.split(',').next()).unwrap_or_else(|| panic!("{name} tag"));
        tags.push(tag.trim().to_owned());
    }
    let mut unique = tags.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), tags.len(), "{tags:?}");
}
