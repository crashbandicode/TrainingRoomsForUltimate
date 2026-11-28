use crate::engine::seams::native_seams::{HookForm, Placement, SeamKind, Sharing};
use crate::engine::seams::native_seams::rows_cut_file::ROWS;

#[test]
fn picture_copy_hook_has_the_expected_guard_and_site() {
    assert_eq!(ROWS.len(), 1);
    let row = &ROWS[0];
    assert_eq!(row.label, crate::model::cut_file::ROW);
    assert_eq!(row.group, "cut_file");
    assert_eq!(row.offset, crate::model::cut_file::SITE);
    assert!(matches!(row.kind, SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid }));
    assert!(matches!(row.sharing, Sharing::None));
    let word = |i: usize| u32::from_le_bytes(row.guard[4 * i..4 * i + 4].try_into().unwrap());
    assert_eq!(word(0), 0xaa15_03e0);
    assert_eq!(word(1), 0xf900_0a74);
    assert_eq!(word(2) & 0xfc00_0000, 0x9400_0000);
    let source = include_str!("../runtime/cut_file.rs");
    assert!(source.contains("#[skyline::hook(offset = 0x37a2dd4, inline)]"));
    let gate = source.find("seam_install::gate(\"cut_file\", &[cut_file::ROW], true)").unwrap();
    let install = source.find("if installed { skyline::install_hooks!(picture_copied); }").unwrap();
    assert!(gate < install);
    let installer = include_str!("../runtime/seams/seam_install.rs");
    let authoritative = installer.split("const AUTHORITATIVE:").nth(1).unwrap().split("];").next().unwrap();
    assert!(authoritative.contains("\"cut_file\","));
    let outcome = |actual| crate::model::guard::outcome_for(&row.guard, actual, row.kind, row.sharing);
    assert!(outcome(&row.guard).usable());
    let mut occupied = row.guard;
    occupied[..4].copy_from_slice(&0x1400_0000u32.to_le_bytes());
    assert!(!outcome(&occupied).usable());
    let mut foreign = row.guard;
    foreign[0] ^= 1;
    assert!(!outcome(&foreign).usable());
}
