//! Native count reconciliation's additional managed-load dependency.
//! The caller owns the live FighterManager map for this one reconciliation.
//! This reader neither changes the requested count nor calls a game function.

use crate::engine::seams::native_fields as fields;

/// Current helper identity and the loader phase native will use. Addresses are
/// used only in this snapshot; they are never retained by this reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Helper {
    pub entry: usize,
    pub owner: u32,
    pub phase: u8,
}

/// Read both Training helper seats from the SAME manager map as the native scan.
/// A missing/foreign entry cannot prove a newly activating native load.
///
/// # Safety
/// `manager` is the live x9 singleton at main+0x155c16c, not a cached manager.
/// Its current primary entries remain live throughout this native callback.
pub(super) unsafe fn helpers(manager: usize) -> [Option<Helper>; 2] {
    let Some(internal) = fields::child(manager, 0) else { return [None; 2]; };
    std::array::from_fn(|i| {
        let owner = i as u32 + 2;
        let entry = fields::child(internal, 0x20 + owner as usize * 8)?;
        (fields::read::<u32>(entry, 0xe0)? == owner).then_some(Helper {
            entry, owner, phase: fields::read(entry, 0x5920)?,
        })
    })
}

/// Only a count transition needs this additional dependency. The native loop
/// already checks its old active entries. INSTALL serializes every managed
/// native mutation, including visible installs and resource-only prefetch.
pub(super) fn busy(old: u32, requested: u32, helpers: [Option<Helper>; 2],
    managed_pending: bool) -> bool {
    if old == requested || !(1..=3).contains(&old) || !(1..=3).contains(&requested) {
        return false;
    }
    managed_pending || helpers.into_iter().flatten().any(|h| {
        old < h.owner && h.owner <= requested && h.phase == 3
    })
}

#[cfg(target_os = "switch")]
#[skyline::hook(offset = 0x155c16c, inline)]
unsafe fn count_readiness(ctx: &mut skyline::hooks::InlineCtx) {
    let _scope = crate::model::hook_stats::HookScope::enter_hook(crate::model::hook_stats::HookId::RosterSources);
    // The displaced CMP still owns flags and its original branches. A native
    // busy phase already takes the right path and needs no managed inspection.
    if ctx.registers[12].w() == 3 { return; }
    let old = ctx.registers[23].w();
    let requested = ctx.registers[8].w();
    if old == requested || !(1..=3).contains(&old) || !(1..=3).contains(&requested) { return; }
    let current = helpers(ctx.registers[9].x() as usize);
    if busy(old, requested, current, super::INSTALL.active())
        || !super::sources::reserve_count_promotion(old, requested, current) {
        ctx.registers[12].set_x(3);
    }
}

/// Install only after the native roster's guarded base has been published.
#[cfg(target_os = "switch")]
pub(super) fn install() {
    let base = super::BASE.load(std::sync::atomic::Ordering::Acquire);
    let legacy = base != 0 && crate::engine::seams::native_seams::rows_training_count::ROWS.iter().all(|row| {
        // Quiescent plugin initialization, before this module patches anything.
        unsafe { std::slice::from_raw_parts((base + row.offset) as *const u8, row.guard.len()) == row.guard }
    });
    if cfg!(feature = "managed-sources") && crate::runtime::roster::cpu_quantity_guarded()
        && crate::runtime::seams::seam_install::gate("character_roster",
            &["roster.count_readiness", "roster.count_inputs", "roster.count_retry"], legacy) && legacy {
        skyline::install_hooks!(count_readiness);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_transition_waits_for_native_load_and_install_publication() {
        let ready = Some(Helper { entry: 0x2000, owner: 2, phase: 1 });
        let loading = Some(Helper { entry: 0x3000, owner: 3, phase: 3 });
        // Old native scan 0..=1 misses newly active CPU2/3.
        assert!(busy(1, 3, [ready, loading], false));
        assert!(!busy(1, 2, [ready, loading], false));
        // Any existing INSTALL owner still protects the shared mutation boundary,
        // even outside the new active set or after native phase 3 has ended.
        assert!(busy(1, 2, [ready, loading], true));
        assert!(busy(3, 1, [ready, None], true));
        assert!(!busy(1, 2, [ready, None], false));
        assert!(!busy(1, 1, [ready, loading], true));
        assert!(!busy(u32::MAX, 2, [ready, loading], true));
        assert!(!busy(1, 4, [ready, loading], true));
    }

    #[test]
    fn manager_snapshot_rejects_replaced_owner_and_missing_entry() {
        let mut seat2 = vec![0u64; 0x5940 / 8];
        let mut seat3 = vec![0u64; 0x5940 / 8];
        let mut internal = [0usize; 16];
        let manager = [&mut internal as *mut _ as usize];
        unsafe {
            fields::write_unchecked(seat2.as_mut_ptr() as usize, 0xe0, 2u32);
            fields::write_unchecked(seat2.as_mut_ptr() as usize, 0x5920, 1u8);
            fields::write_unchecked(seat3.as_mut_ptr() as usize, 0xe0, 3u32);
            fields::write_unchecked(seat3.as_mut_ptr() as usize, 0x5920, 3u8);
            internal[6] = seat2.as_ptr() as usize;
            internal[7] = seat3.as_ptr() as usize;
            let measured = crate::engine::swap::mii_cycle::compact_regressions::AllocationCount::start();
            let current = helpers(manager.as_ptr() as usize);
            assert_eq!(current[0], Some(Helper { entry: seat2.as_ptr() as usize, owner: 2, phase: 1 }));
            assert_eq!(current[1], Some(Helper { entry: seat3.as_ptr() as usize, owner: 3, phase: 3 }));
            for _ in 0..128 {
                assert!(busy(1, 3, helpers(manager.as_ptr() as usize), false));
            }
            assert_eq!(measured.finish(), 0);
            fields::write_unchecked(seat2.as_mut_ptr() as usize, 0xe0, 4u32);
            fields::write_unchecked(internal.as_mut_ptr() as usize, 7 * std::mem::size_of::<usize>(), 0usize);
            assert_eq!(helpers(manager.as_ptr() as usize), [None; 2]);
        }
    }
}
