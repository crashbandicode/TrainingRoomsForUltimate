//! Read-only reconciliation of a resource owner's key map against the ledger's prediction
//! (WORKPLAN §2.3; D-SG1-RCA §5 "read-only confirmation"). Diagnostics only: no native call,
//! no write, no cleanup. During WP2 the ledger is a mirror of what the current bank code holds
//! (`Ledger::mirror`), so a `residual` names a key the current code acquired and never released
//! (the Aegis `0x1338` of D-SG1 would have been named in the first lap) and a `missing` names a
//! predicted key whose file is absent (`0x17e2df0` inserts no node for the `0xffffff` handle).
//!
//! The map is the libc++ `__tree` at `R+0x10` (FACT `0x17e2fe0` listing,
//! `resource-audit/resource_17e2c00_17e9200.asm:259–309`; SOLO-STALL-RCA §5): `R+0x8` begin
//! node, `R+0x10` root (`__end_node.__left_`), `R+0x18` size; node `{left @0, right @8, parent
//! @0x10, is_black @0x18, key u64 @0x20, refs u32 @0x28, handle u32 @0x2c}`; the native lock is
//! `R+0x28` (`0x39c19c0`/`0x39c19d0`). This observer never takes that lock: it walks only when the
//! owner's task is idle (`task+0x10 == 0`, the same `task_pending` the bank logs) and every read
//! is a validated `native_fields::read`/`child`; a walk whose node count disagrees with `R+0x18`,
//! exceeds the cap or meets an invalid pointer is reported as `walk=invalid`, never trusted.
//! Taking the native lock (two `Called` seams for WP-A's table) is the follow-up if idle walks
//! prove insufficient.
//!
//! Seams (three integrator-placed call lines in `src/native_body_bank.rs`, WORKPLAN §2.0): after a
//! Phase B build (`perform_swap`, after the post-build object release), before every mod Reload
//! (`before_reload`, after `retire_groups`) and at the end of `retire_groups`. Line (interface,
//! diag-only once WP-D lands):
//! `[Ledger] reconcile owner={} phase={} map={} tree={} task_pending={} r48={} r7d={} r7f={}
//!  predicted=[(key, refs)…] actual=[…] residual=[…] missing=[…]`.

use crate::engine::seams::native_fields as fields;
use std::convert::TryFrom; // root crate is edition 2018: not in the prelude
use crate::engine::swap::swap_model::ledger::RFlags;
use crate::engine::compat::fixed::FixedList;

/// Nodes a walk may visit. Every recorded lap stayed at `map<=15`; two prefetched specials plus a
/// window cannot exceed 24 keys (8 + 8 + 8).
pub(crate) const MAP_WALK_LIMIT: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MapNode {
    pub key: u32,
    pub refs: u32,
}

/// In-order walk of the key map rooted at `*(R+0x10)`. `None` when a pointer fails validation,
/// the node count disagrees with `R+0x18`, or the cap is exceeded.
///
/// # Safety
/// `r` must be the role's resource owner inside the current entry callback with INSTALL held
/// (the bank's `resource(base, owner)` proof) and its task idle; every dereference goes through
/// `native_fields` validation and nothing is retained past the call.
pub(crate) unsafe fn walk_map(r: usize) -> Option<FixedList<MapNode, MAP_WALK_LIMIT>> {
    let count = usize::try_from(fields::read::<u64>(r, 0x18)?).ok()?;
    if count > MAP_WALK_LIMIT { return None; }
    let mut out = FixedList::new(MapNode { key: 0, refs: 0 });
    let Some(root) = fields::child(r, 0x10) else { return (count == 0).then_some(out); };
    let mut stack = FixedList::<usize, MAP_WALK_LIMIT>::new(0);
    let mut node = Some(root);
    while node.is_some() || !stack.is_empty() {
        while let Some(current) = node {
            if stack.len() >= MAP_WALK_LIMIT { return None; }
            stack.push(current);
            node = fields::child(current, 0);
        }
        let current = stack.pop()?;
        let key = fields::read::<u64>(current, 0x20)?;
        let refs = fields::read::<u32>(current, 0x28)?;
        if !out.push(MapNode { key: u32::try_from(key).ok()?, refs }) { return None; }
        if out.len() > count { return None; }
        node = fields::child(current, 8);
    }
    (out.len() == count).then_some(out)
}

/// `R+0x48`, `R+0x7d`, `R+0x7f` as read (the key rules' inputs; their QP/LDN values are an open fact).
///
/// # Safety
/// As for [`walk_map`].
pub(crate) unsafe fn owner_flags(r: usize) -> RFlags {
    RFlags {
        r48: fields::read::<u8>(r, 0x48).unwrap_or(0),
        r7d: fields::read::<u8>(r, 0x7d).unwrap_or(0),
        r7f: fields::read::<u8>(r, 0x7f).unwrap_or(0),
    }
}

// WP2's mirror-based `reconcile(r, owner, phase, prefetched)` is superseded at M3 by
// `r_handle::reconcile(role, phase)`, which reconciles the role's real ledger (WP4) at the same build and
// teardown seams; the before-reload seam is `r_handle::expect_free`. `walk_map` and `owner_flags` stay.

#[cfg(test)]
mod tests {
    use super::*;

    /// A host-built libc++-shaped tree: nodes `{left, right, parent, is_black, key u64, refs u32, handle u32}`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Node { left: usize, right: usize, parent: usize, black: u64, key: u64, refs: u32, handle: u32 }

    #[repr(C)]
    struct Owner { pad: [u64; 1], begin: usize, root: usize, size: u64 }

    #[test]
    fn the_walk_returns_the_keys_in_order_with_their_refs_and_rejects_a_torn_count() {
        // The tree and the owner are only ever read through the raw address `walk_map` validates, so
        // they are written through raw pointers too (the by-name writes would be "never read").
        let mut nodes = [Node { left: 0, right: 0, parent: 0, black: 0, key: 0, refs: 0, handle: 0 }; 3];
        let base = nodes.as_mut_ptr() as usize;
        let at = |i: usize| base + i * std::mem::size_of::<Node>();
        let mut owner = Owner { pad: [0], begin: at(0), root: at(1), size: 3 };
        let r = &mut owner as *mut Owner as usize;
        unsafe {
            let p = nodes.as_mut_ptr();
            p.add(1).write(Node { left: at(0), right: at(2), parent: 0, black: 1, key: 0x1333, refs: 2, handle: 0 });
            p.add(0).write(Node { left: 0, right: 0, parent: at(1), black: 0, key: 0x1332, refs: 1, handle: 0 });
            p.add(2).write(Node { left: 0, right: 0, parent: at(1), black: 0, key: 0x1338, refs: 1, handle: 0 });
        }
        let walked = unsafe { walk_map(r) }.expect("valid tree");
        assert_eq!(&*walked, &[MapNode { key: 0x1332, refs: 1 }, MapNode { key: 0x1333, refs: 2 }, MapNode { key: 0x1338, refs: 1 }]);
        unsafe { (*(r as *mut Owner)).size = 2; }
        assert!(unsafe { walk_map(r) }.is_none(), "a count that disagrees with the nodes is not trusted");
        unsafe { (*(r as *mut Owner)).size = 0; (*(r as *mut Owner)).root = 0; }
        assert!(unsafe { walk_map(r) }.is_some_and(|nodes| nodes.is_empty()), "an empty map walks to nothing");
    }
}
