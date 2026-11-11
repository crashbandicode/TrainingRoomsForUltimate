//! Spawn pose for a visible practice fighter replacement (Smash 13.0.5).
//!
//! The asynchronous constructor reads FighterEntry+0xf8 -> information+0x9f0
//! (xyz) and +0x9e0 (facing). Moving PostureModule after a save-state rebirth does
//! not update that record: a later reload otherwise uses the old high rebirth
//! point. Capture the outgoing live body before retirement and stage its pose
//! immediately before INSTALL. Parked preload sources never use this helper.
//!
//! Addresses below are borrowed for ONE loader callback under the caller's
//! existing readiness/scene/INSTALL protocol. No object is owned, retained,
//! allocated, freed, or moved here. Structural checks do not prove liveness.

use crate::engine::seams::native_fields as fields;
use crate::engine::swap::character_forms::ACTIVE_BODY_STATE;

pub(super) enum SpawnPose {
    Current([f32; 3], f32),
    /// During entry/death/rebirth, preserve the native spawn rather than an
    /// animation position or a body already beyond the blast zone.
    Native,
}

/// A single-callback preflight; deliberately neither Clone nor Copy.
pub(super) struct ReloadPose {
    information: usize,
    pose: SpawnPose,
}

/// Capture the current active form, not always the first body in a group.
///
/// # Safety
/// `entry` is the current ready visible entry owned by `owner`; all non-null
/// children are live under its native loader callback. The caller has checked
/// the scene/role and full entry readiness. `base` is fingerprint-verified main
/// text. `posture` may only read the supplied live module, without reentrancy.
/// Use the result only in this callback, before any native retirement.
pub(super) unsafe fn capture(
    base: usize,
    entry: usize,
    owner: u32,
    posture: impl FnOnce(usize) -> Option<SpawnPose>,
) -> Option<ReloadPose> {
    if !fields::object(entry) || fields::read::<u32>(entry, 0xe0)? != owner {
        return None;
    }
    let count = fields::read::<u32>(entry, 0x14)?;
    let index = fields::read::<u8>(entry, 0x5918)? as usize;
    if !(1..=3).contains(&count) || index >= count as usize {
        return None;
    }
    let body = fields::child(entry, 0x4160 + index * 8)?;
    if fields::read::<usize>(body, 0)? != base.checked_add(0x4f80210)?
        || fields::read::<u64>(body, 0x10)? != u64::from(owner)
        || fields::read::<u32>(body, 0xc)? != fields::read::<u32>(entry, 0x18 + index * 4)?
        || fields::read::<u8>(body, 0x3a)? != ACTIVE_BODY_STATE
    {
        return None;
    }
    let module = fields::child(body, 0x20)?;
    let information = fields::child(entry, 0xf8)?;
    information.checked_add(0xa00)?;
    let pose = posture(module)?;
    if let SpawnPose::Current(position, facing) = &pose {
        if !position.iter().all(|v| v.is_finite()) || !matches!(*facing, -1.0 | 1.0) {
            return None;
        }
    }
    Some(ReloadPose { information, pose })
}

impl ReloadPose {
    /// Publish only the constructor's xyz/facing; leave padding and all other
    /// information fields intact. There is no recoverable failure halfway
    /// through a committed INSTALL, so required fields use qualified writes.
    ///
    /// # Safety
    /// Still the SAME callback/entry as capture, now owning INSTALL. No native
    /// operation has retired/replaced the entry or its information record.
    /// Call native INSTALL immediately afterward; never store this preflight.
    pub(super) unsafe fn stage(self) {
        if let SpawnPose::Current(position, facing) = self.pose {
            fields::write_unchecked(self.information, 0x9f0, position);
            fields::write_unchecked(self.information, 0x9e0, facing);
        }
    }
}

