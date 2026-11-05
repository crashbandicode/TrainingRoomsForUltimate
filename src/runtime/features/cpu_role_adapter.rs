//! The value-only entry/scene/consumer adapter used by the native CPU callbacks.
//! Host probes include this exact file; it holds no native pointer or second registry.
use crate::model::cpu::{
    role::{FighterKey, FrameReceipt, Scene},
    state::Cpu,
};

pub fn entry_departed(cpu: &mut Cpu, owner: u32) {
    cpu.retire_entry(owner);
}

pub fn scene_departed(cpu: &mut Cpu) {
    cpu.role_scene(None);
}

pub fn before_frame(cpu: &mut Cpu, scene: Option<Scene>, entry: i32) -> Option<FrameReceipt> {
    cpu.role_scene(scene);
    cpu.roles.receipt(entry)
}

pub fn current_frame(cpu: &mut Cpu, scene: Option<Scene>, receipt: FrameReceipt) -> bool {
    cpu.role_scene(scene);
    cpu.roles.current(receipt)
}

pub fn role(cpu: &mut Cpu, scene: Option<Scene>, accessor: usize) -> Option<FighterKey> {
    cpu.role_scene(scene);
    cpu.roles.find(scene, accessor)
}

/// `fresh_cpu` came from the current manager, NOT by reading a stored accessor.
/// Return a native-query plan; callers release the CPU lock before executing it.
/// The frame strip's CPU (SPEC-input-06 §5.1): `fresh` when it is the primary CPU role's live accessor in `scene`, and
/// not P1's. Unlike `measurement` it leaves the frame-advantage state alone.
pub fn primary_cpu(cpu: &Cpu, scene: Scene, player: usize, fresh: usize) -> Option<usize> {
    let key = FighterKey::new(if scene == Scene::Lobby { 4 } else { 1 }, false)?;
    (fresh != player && cpu.roles.accessor_of(Some(scene), key) == Some(fresh)).then_some(fresh)
}

pub fn measurement(
    cpu: &mut Cpu,
    scene: Option<Scene>,
    player: usize,
    fresh_cpu: Option<usize>,
) -> Option<(usize, usize)> {
    cpu.role_scene(scene);
    let Some(scene) = scene else {
        cpu.frame_advantage(false, false);
        return None;
    };
    let primary = if scene == Scene::Lobby { 4 } else { 1 };
    let key = FighterKey::new(primary, false)?;
    let Some(fresh) = fresh_cpu else {
        cpu.frame_advantage(false, false);
        return None;
    };
    if !cpu.settings.frame_advantage
        || !cpu.settings.mash_state.is_empty()
        || cpu.roles.accessor_of(Some(scene), key) != Some(fresh)
    {
        cpu.frame_advantage(false, false);
        return None;
    }
    Some((player, fresh))
}
