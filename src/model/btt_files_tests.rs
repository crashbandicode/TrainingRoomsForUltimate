//! rc.14 Break the Targets, the Saved layouts list on the menu's side (`artifacts/rc14-eggsercise/SPEC.md` §V2.5 "Saved
//! layouts"): its lines and footer.

lib_unit_tests! {
use super::*;

/// A slot of `n` targets.
fn slot(n: usize) -> Slot {
    Slot { n: n as u8, at: [[0.0; 2]; crate::model::btt::MAX] }
}

/// SPEC V2.5: the list shows each slot with its targets; the plugin has no date to show.
fn btt_slot_labels() {
    assert_eq!(slot_label(2, None), "Slot 3 · empty");
    assert_eq!(slot_label(0, Some(&slot(1))), "Slot 1 · 1 target");
    assert_eq!(slot_label(1, Some(&slot(6))), "Slot 2 · 6 targets");
    assert_eq!(slot_label(4, Some(&slot(10))), "Slot 5 · 10 targets");
}

/// The list's own first line is the focused row's label; the second says what the buttons do here.
fn btt_layouts_footer() {
    assert_eq!(layouts_footer("Slot 2 · 6 targets\nA Choose · X Rename · Y Delete"), "Slot 2 · 6 targets\nA Load · Y Delete · B Return");
    assert_eq!(layouts_footer(""), "\nA Load · Y Delete · B Return");
}

/// The cases of this module in one table.
#[test]
fn case_table() {
    crate::test_table::run(&[("btt_slot_labels", btt_slot_labels as fn()), ("btt_layouts_footer", btt_layouts_footer as fn())]);
}
}