//! Pure validation of a managed descriptor against current native evidence.
//! The caller authorizes reset references only for an owned Training replacement.
//! This module never reads or writes native memory.
pub(crate) fn matches(
    current: [u32; 28],
    mut expected: [u32; 28],
    originals: Option<[[u64; 2]; 4]>,
    selection_base: u64,
) -> bool {
    if current == expected {
        return true;
    }
    let Some(originals) = originals else {
        return false;
    };
    let [rules, selections] = originals[0];
    if rules == 0
        || rules & 7 != 0
        || selection_base == 0
        || selections != selection_base
        || originals.iter().any(|pair| *pair != [rules, selections])
    {
        return false;
    }
    // main+1510114/+1510134 write exactly these two borrowed references.
    // Preserve exact comparison of the role/configuration fields and +68.
    expected[22] = rules as u32;
    expected[23] = (rules >> 32) as u32;
    expected[24] = selections as u32;
    expected[25] = (selections >> 32) as u32;
    current == expected
}

