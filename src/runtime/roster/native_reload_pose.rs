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

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        entry: Vec<u64>,
        body: [u64; 8],
        info: Vec<u64>,
        module: [u64; 1],
    }

    impl Fixture {
        fn new() -> Self {
            Self { entry: vec![0; 0x5a00 / 8], body: [0; 8], info: vec![0xabababababababab; 0xa00 / 8], module: [0] }
        }
        unsafe fn prepare(&mut self, owner: u32, index: usize) -> usize {
            let e = self.entry.as_mut_ptr() as usize;
            let b = self.body.as_mut_ptr() as usize;
            fields::write_unchecked(e, 0xe0, owner);
            fields::write_unchecked(e, 0x14, 3u32);
            fields::write_unchecked(e, 0x5918, index as u8);
            fields::write_unchecked(e, 0x18 + index * 4, 0x4au32);
            fields::write_unchecked(e, 0x4160 + index * 8, b);
            fields::write_unchecked(e, 0xf8, self.info.as_mut_ptr() as usize);
            fields::write_unchecked(b, 0, 0x84f80210usize);
            fields::write_unchecked(b, 0xc, 0x4au32);
            fields::write_unchecked(b, 0x10, u64::from(owner));
            fields::write_unchecked(b, 0x20, self.module.as_mut_ptr() as usize);
            // Native active state, independent of the production constant.
            fields::write_unchecked(b, 0x3a, 4u8);
            e
        }
    }

    #[test]
    fn valid_save_restore_replaces_stale_rebirth_pose_only() {
        for (owner, index) in [(0, 0), (1, 1), (4, 2)] {
            let mut f = Fixture::new();
            let entry = unsafe { f.prepare(owner, index) };
            let info = f.info.as_mut_ptr() as usize;
            unsafe { fields::write_unchecked(info, 0x9f0, [36.0f32, 70.0, 0.0]); }
            let before = f.info.clone();
            let pose = unsafe { capture(0x80000000, entry, owner, |module| {
                assert_eq!(module, f.module.as_ptr() as usize);
                Some(SpawnPose::Current([36.0, 0.01, 0.0], -1.0))
            }) }.unwrap();
            unsafe { pose.stage(); }
            assert_eq!(unsafe { fields::read::<[f32; 3]>(info, 0x9f0) }, Some([36.0, 0.01, 0.0]));
            assert_eq!(unsafe { fields::read::<f32>(info, 0x9e0) }, Some(-1.0));
            let before = before.iter().flat_map(|v| v.to_ne_bytes()).collect::<Vec<_>>();
            let after = f.info.iter().flat_map(|v| v.to_ne_bytes()).collect::<Vec<_>>();
            for i in 0..before.len() {
                if !(0x9e0..0x9e4).contains(&i) && !(0x9f0..0x9fc).contains(&i) {
                    assert_eq!(before[i], after[i], "unrelated information byte {i:x}");
                }
            }
        }
    }

    #[test]
    fn malformed_or_departing_body_never_calls_posture_or_writes() {
        for case in 0..8 {
            let mut f = Fixture::new();
            let entry = unsafe { f.prepare(0, 0) };
            let b = f.body.as_mut_ptr() as usize;
            unsafe {
                match case {
                    0 => fields::write_unchecked(entry, 0xe0, 4u32),
                    1 => fields::write_unchecked(entry, 0x5918, 3u8),
                    2 => fields::write_unchecked(entry, 0xf8, 0usize),
                    3 => fields::write_unchecked(b, 0, 0usize),
                    4 => fields::write_unchecked(b, 0x10, 1u64),
                    5 => fields::write_unchecked(b, 0xc, 9u32),
                    6 => fields::write_unchecked(b, 0x3a, 3u8),
                    _ => fields::write_unchecked(b, 0x20, 0usize),
                }
            }
            let before = f.info.clone();
            assert!(unsafe { capture(0x80000000, entry, 0, |_| panic!("invalid body")) }.is_none());
            assert_eq!(f.info, before);
        }
    }

    #[test]
    fn finite_airborne_pose_is_preserved_and_invalid_values_refuse_without_writes() {
        for (position, facing, accepted) in [
            ([11.0, 26.5, 2.0], 1.0, true),
            ([f32::NAN, 0.0, 0.0], 1.0, false),
            ([0.0, f32::INFINITY, 0.0], 1.0, false),
            ([0.0, 0.0, 0.0], 0.0, false),
        ] {
            let mut f = Fixture::new();
            let entry = unsafe { f.prepare(0, 0) };
            let before = f.info.clone();
            let pose = unsafe { capture(0x80000000, entry, 0, |_| Some(SpawnPose::Current(position, facing))) };
            assert_eq!(pose.is_some(), accepted);
            assert_eq!(f.info, before, "capture is read-only even on success");
            if let Some(pose) = pose {
                unsafe { pose.stage(); }
                assert_eq!(unsafe { fields::read::<[f32; 3]>(f.info.as_ptr() as usize, 0x9f0) }, Some(position));
            }
        }
    }

    #[test]
    fn native_spawn_keeps_the_entire_anchor_unchanged() {
        let mut f = Fixture::new();
        let entry = unsafe { f.prepare(4, 0) };
        let before = f.info.clone();
        let pose = unsafe { capture(0x80000000, entry, 4, |_| Some(SpawnPose::Native)) }.unwrap();
        unsafe { pose.stage(); }
        assert_eq!(f.info, before);
    }
}
