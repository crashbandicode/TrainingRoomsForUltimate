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
/// `manager` is freshly read from the guarded native singleton during count
/// reconciliation, not a cached manager.
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
#[skyline::hook(offset = 0x155c188, inline)]
unsafe fn count_readiness(ctx: &mut skyline::hooks::InlineCtx) {
    let _scope = crate::model::hook_stats::HookScope::enter_hook(crate::model::hook_stats::HookId::RosterSources);
    // Native's old-seat busy scan has already completed. At this boundary x9
    // is the cooldown, not the manager. The original BIC/CMP/STR/BEQ execute
    // after this callback; no branch targets the interior of their patch.
    let old = ctx.registers[23].w();
    let requested = ctx.registers[8].w();
    if old == requested || !(1..=3).contains(&old) || !(1..=3).contains(&requested) { return; }
    let base = super::BASE.load(std::sync::atomic::Ordering::Acquire);
    let symbol = super::setting(&crate::runtime::compat::common::FIGHTER_MANAGER_ADDR);
    let manager = (base != 0 && symbol == base + 0x52b94f8)
        .then(|| fields::child(symbol, 0)).flatten();
    let blocked = if let Some(manager) = manager {
        let current = helpers(manager);
        busy(old, requested, current, super::INSTALL.active())
            || !super::sources::reserve_count_promotion(old, requested, current)
    } else { true };
    if blocked {
        // SUB at 184 already ran. Feed the existing saturating decrement/store
        // and subsequent signed cooldown check: store 5 and return before the
        // committed-count write. Neither requested count nor old count changes.
        ctx.registers[9].set_x(6);
        ctx.registers[10].set_x(5);
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
            &["roster.count_readiness", "roster.count_inputs", "roster.count_retry", "roster.count_scan"], legacy) && legacy {
        skyline::install_hooks!(count_readiness);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_hook_window_leaves_native_empty_entry_branch_intact() {
        use crate::engine::seams::native_seams::rows_training_count::ROWS;
        let hook = ROWS.iter().find(|r| r.label == "roster.count_readiness").unwrap();
        let inputs = ROWS.iter().find(|r| r.label == "roster.count_inputs").unwrap();
        // The null-entry CBZ must still land on native CMP x10,x23. The old
        // far hook at 16c overwrote that target, even with a matching guard.
        let branch_offset = 0x155c164 - inputs.offset;
        let branch = u32::from_le_bytes(inputs.guard[branch_offset..branch_offset + 4].try_into().unwrap());
        assert_eq!(branch, 0xb400008c);
        let target = 0x155c164 + (((branch >> 5) & 0x7ffff) as usize * 4);
        assert_eq!(target, 0x155c174);
        assert_eq!(hook.offset & 7, 0);
        assert!(!(hook.offset..hook.offset + 16).contains(&target));
        let words: Vec<u32> = hook.guard[..16].chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();
        assert_eq!(words, [0x0aaa7d4a, 0x6b17011f, 0xb901c68a, 0x5400c060]);
        // BIC and STR do not change NZCV. CMP generates the flags consumed by
        // the final BEQ, whose existing target is outside the displaced span.
        assert_eq!(0x155c194 + (((words[3] >> 5) & 0x7ffff) as usize * 4), 0x155d9a0);
    }

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
