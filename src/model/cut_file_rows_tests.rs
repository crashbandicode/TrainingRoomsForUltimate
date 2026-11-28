use crate::engine::seams::native_seams::{HookForm, Placement, SeamKind};
use crate::engine::seams::native_seams::rows_cut_file::ROWS;

#[test]
fn picture_copy_hook_has_the_expected_guard_and_site() {
    assert_eq!(ROWS.len(), 1);
    let row = &ROWS[0];
    assert_eq!(row.label, "cut_file.picture_copied");
    assert_eq!(row.offset, crate::model::cut_file::SITE);
    assert!(matches!(row.kind, SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid }));
    let word = |i| u32::from_le_bytes(row.guard[4 * i..4 * i + 4].try_into().unwrap());
    assert_eq!(word(0), 0xaa15_03e0);
    assert_eq!(word(1), 0xf900_0a74);
    assert_eq!(word(2) & 0xfc00_0000, 0x9400_0000);
    assert!(include_str!("../runtime/cut_file.rs").contains("#[skyline::hook(offset = 0x37a2dd4, inline)]"));
}
