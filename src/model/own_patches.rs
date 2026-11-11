//! Our own writes in the game's code, as the seam checks must see them.
//!
//! A seam check compares the 64 bytes now at a site with the 13.0.5 bytes. When one of this plugin's own hooks or text
//! patches already sits inside that window, the check reads our own write and reports `occupied` or `foreign`, and an
//! enforcing group is skipped: in rc.2, `lobby_stage::init` hooked `0x25f7874` before `practice_stage`'s group checked
//! that row, and all nine practice stage sites were skipped. So the installer keeps, for every hook row (a site we
//! write), the bytes there before our boot wrote anything and the bytes our write left; a check puts our own write back
//! to what it replaced before it compares. Another plugin's patch is never put back: before our boot it is in the
//! "before" bytes, and over our write it no longer matches the "after" bytes.
//!
//! Pure: the installer (`runtime::seams::seam_install`) reads the game's memory and keeps the records.

use crate::model::guard::{GUARD_BYTES, HOOK_SPAN_WORDS};

/// Bytes a hook row's write may cover: Skyline's `nop` + far jump (five words), or a shorter patch.
pub const SPAN: usize = HOOK_SPAN_WORDS * 4;

/// One site this plugin writes, as recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnPatch {
    /// Offset of the hook row's site from the start of `main`'s `.text`.
    pub offset: usize,
    /// The bytes there before this plugin's boot wrote anything (another plugin's earlier patch included).
    pub before: [u8; SPAN],
    /// The bytes our write left, once recorded (`seal`, at the install line). `None` while the boot is still installing:
    /// then every change since `before` is ours, because only our boot runs then.
    pub after: Option<[u8; SPAN]>,
}

/// The overlap of `[a, a + len_a)` and `[b, b + len_b)` as `(start, end)`, if any.
fn overlap(a: usize, len_a: usize, b: usize, len_b: usize) -> Option<(usize, usize)> {
    let start = a.max(b);
    let end = (a + len_a).min(b + len_b);
    (start < end).then_some((start, end))
}

impl OwnPatch {
    /// Whether this write can show inside the 64 bytes at `offset`.
    pub fn touches(&self, offset: usize) -> bool {
        overlap(self.offset, SPAN, offset, GUARD_BYTES).is_some()
    }

}

/// `actual` (the 64 bytes now at `offset`) with every own write inside the window put back to the bytes it replaced.
/// A write is put back only where all of its bytes inside the window still hold our write: while booting, anything other
/// than `before`; once recorded, exactly `after`. Returns the bytes to compare and how many writes were put back.
pub fn unpatch(offset: usize, actual: &[u8; GUARD_BYTES], patches: impl IntoIterator<Item = OwnPatch>) -> ([u8; GUARD_BYTES], usize) {
    let mut out = *actual;
    let mut restored = 0;
    for patch in patches {
        let Some((start, end)) = overlap(patch.offset, SPAN, offset, GUARD_BYTES) else { continue };
        let (in_window, in_patch) = (start - offset..end - offset, start - patch.offset..end - patch.offset);
        let now = &actual[in_window.clone()];
        let ours = match patch.after {
            None => now != &patch.before[in_patch.clone()],
            Some(after) => after != patch.before && now == &after[in_patch.clone()],
        };
        if ours {
            out[in_window].copy_from_slice(&patch.before[in_patch]);
            restored += 1;
        }
    }
    (out, restored)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::guard::{guard_outcome, HookForm, Placement, SeamKind, BR_X17, LDR_X17_LITERAL_8, NOP};

    const HOOK: SeamKind = SeamKind::Hook { form: HookForm::Inline, placement: Placement::AfterPrologue };

    /// A stand-in for `main`'s `.text`: distinct, non-hook words.
    fn text(len: usize) -> Vec<u8> {
        (0..len / 4).flat_map(|i| (0x1100_0000u32 + i as u32).to_le_bytes()).collect()
    }

    fn window(text: &[u8], offset: usize) -> [u8; GUARD_BYTES] {
        text[offset..offset + GUARD_BYTES].try_into().unwrap()
    }

    fn span(text: &[u8], offset: usize) -> [u8; SPAN] {
        text[offset..offset + SPAN].try_into().unwrap()
    }

    /// Skyline's far jump (`nop` when 4 past an 8-byte boundary; this site is aligned) to `target`.
    fn far_jump(text: &mut [u8], offset: usize, target: u64) {
        let mut words = vec![LDR_X17_LITERAL_8, BR_X17];
        words.push(target as u32);
        words.push((target >> 32) as u32);
        for (i, word) in words.iter().enumerate() {
            text[offset + 4 * i..offset + 4 * i + 4].copy_from_slice(&word.to_le_bytes());
        }
    }

    #[test]
    fn our_own_hook_reads_stock_while_booting_and_after_the_record() {
        let original = text(0x200);
        let mut live = original.clone();
        let site = 0x40;
        let mut patch = OwnPatch { offset: site, before: span(&live, site), after: None };
        far_jump(&mut live, site, 0x8_0000_1000);
        // Without the record the window reads our own hook: occupied for a hook row there.
        let expected = window(&original, site);
        assert!(!guard_outcome(&expected, &window(&live, site), HOOK).usable());
        // While booting, the change since `before` is ours.
        let (seen, n) = unpatch(site, &window(&live, site), [patch]);
        assert_eq!((guard_outcome(&expected, &seen, HOOK), n), (crate::model::guard::GuardOutcome::Stock, 1));
        // Recorded: still ours, also for a window that starts before the site and covers part of it.
        patch.after = Some(span(&live, site));
        for at in [site, site - 0x30, site + 8] {
            let (seen, n) = unpatch(at, &window(&live, at), [patch]);
            assert_eq!((seen, n), (window(&original, at), 1), "window at {at:#x}");
        }
    }

    #[test]
    fn another_plugins_patch_is_never_put_back() {
        let original = text(0x200);
        let site = 0x40;
        // Another plugin's far jump over our recorded write: not ours any more.
        let mut live = original.clone();
        let mut patch = OwnPatch { offset: site, before: span(&live, site), after: None };
        far_jump(&mut live, site, 0x8_0000_1000);
        patch.after = Some(span(&live, site));
        far_jump(&mut live, site, 0x9_0000_2000);
        let (seen, n) = unpatch(site, &window(&live, site), [patch]);
        assert_eq!((seen, n), (window(&live, site), 0));
        // Another plugin's hook from before our boot is part of `before`: it stays, in both states.
        let mut foreign = original.clone();
        far_jump(&mut foreign, site, 0x9_0000_2000);
        let mut patch = OwnPatch { offset: site, before: span(&foreign, site), after: None };
        let (seen, n) = unpatch(site, &window(&foreign, site), [patch]);
        assert_eq!((seen, n), (window(&foreign, site), 0));
        patch.after = Some(patch.before);
        assert_eq!(unpatch(site, &window(&foreign, site), [patch]).1, 0, "no write of ours: nothing to put back");
        // A write elsewhere is not touched by a window that does not reach it.
        let far = OwnPatch { offset: 0x100, before: span(&original, 0x100), after: None };
        assert!(!far.touches(site));
        assert_eq!(unpatch(site, &window(&live, site), [far]).1, 0);
    }

    #[test]
    fn a_nop_aligned_far_jump_and_a_one_word_patch_are_put_back_exactly() {
        let original = text(0x200);
        let mut live = original.clone();
        let (a, b) = (0x44, 0x80);
        let mut patches = [
            OwnPatch { offset: a, before: span(&live, a), after: None },
            OwnPatch { offset: b, before: span(&live, b), after: None },
        ];
        live[a..a + 4].copy_from_slice(&NOP.to_le_bytes());
        far_jump(&mut live, a + 4, 0x8_0000_1000);
        live[b..b + 4].copy_from_slice(&NOP.to_le_bytes());
        for patch in &mut patches {
            patch.after = Some(span(&live, patch.offset));
        }
        // A window that covers the far jump's tail and the patched word's site.
        let (seen, n) = unpatch(0x50, &window(&live, 0x50), patches);
        assert_eq!((seen, n), (window(&original, 0x50), 2));
    }
    }
}
