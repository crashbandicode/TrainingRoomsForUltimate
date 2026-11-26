//! The clip editor's scrub, the game side (`model::macro_ui::scrub_track`): while In or Out is selected, each role's
//! body is held where the recording had it on that frame, the way a placement picker holds one (`placement`), and
//! put back when nothing is asked of it any more.
//!
//! It runs on a role's own fighter frame, in Training and in the waiting room's practice. It reads the editor's
//! block through the published slot and takes no lock a plugin thread can hold: the holds' lock is only ever tried,
//! and only here.
//!
//! NOT YET SWITCH-RUN.
use super::placement;
use super::stage_geometry::RayProbe;
use crate::engine::macro_ui::scrub_track::{can_stand, in_bounds, Block, Spot, SEEN_HELD, SEEN_IDLE, SEEN_OFF, SLOT};
use crate::model::crosshair::CpuPreview;
use crate::model::stage_geometry::{classify, Unread};
use crate::runtime::{game::natives, lobby::training::practice, state_natives as state};

/// # Safety
/// The fighter frame of the role in `slot` (0 Player 1, 1 the CPU) on the game thread; `accessor` is that
/// fighter's own module accessor.
pub(super) unsafe fn frame(slot: usize, accessor: usize) {
    if !practice::enabled() || !practice::owned() {
        return;
    }
    SLOT.with(|block| {
        // Read before the ask: a frame that read an older ask reports the older generation (`Block::released`).
        let generation = block.generation();
        let Some(mut holds) = block.holds.try_lock() else { return };
        // A hold made on another body (a swapped fighter, a new scene): the hold is dropped, for both roles, until the
        // editor asks for another frame.
        if holds[slot].is_some_and(|hold| hold.body() != natives::battle_object_id(accessor)) {
            block.forget();
        }
        let seen = match point(block, &holds, slot, accessor) {
            Some(spot) => hold(block, slot, &mut holds[slot], accessor, spot),
            None => release(&mut holds[slot], accessor),
        };
        block.set_seen(slot, seen);
        block.ack(slot, generation);
    });
}

/// Where this role's body is asked to be. The group's home is where the hold found the target role's body: its
/// hold's origin, or where it stands now when it is not held yet.
unsafe fn point(block: &Block, holds: &[Option<CpuPreview>; 2], slot: usize, accessor: usize) -> Option<Spot> {
    let target = block.target_slot()?;
    let home = match holds[target] {
        Some(hold) => hold.origin().at,
        None if target == slot => [natives::pos_x(accessor), natives::pos_y(accessor)],
        None => {
            let pose = crate::engine::rr_host::live_pose(target);
            if pose.fighter_kind < 0 {
                return None;
            }
            [pose.x, pose.y]
        }
    };
    block.held_at(slot, home)
}

/// Hold the body at `spot`, facing as recorded. A point outside the camera's limits, or one the stage rule does not
/// let a body stand at (inside a solid, past a blast line), is not taken: the body goes back, or stays, where the
/// hold found it.
unsafe fn hold(block: &Block, index: usize, slot: &mut Option<CpuPreview>, accessor: usize, spot: Spot) -> u32 {
    let Some(limits) = natives::camera_limit_range() else { return release(slot, accessor) };
    if !in_bounds(spot.at, limits) || !block.standable(index, spot.at, || stands(accessor, spot.at)) {
        release(slot, accessor);
        return SEEN_OFF;
    }
    if placement::hold_at(slot, accessor, natives::battle_object_id(accessor), spot.at, false).is_none() {
        return SEEN_IDLE;
    }
    state::set_lr(accessor, spot.facing);
    SEEN_HELD
}

/// The shared stage rule for a point, read by the placement ray through this role's own body on its own frame. The
/// block asks it again for a new shown frame or a changed world point. A stage that cannot be read lets nobody stand.
unsafe fn stands(accessor: usize, at: [f32; 2]) -> bool {
    let Some(blast) = super::tech_surfaces::blast_bounds() else { return false };
    can_stand(RayProbe::new(accessor).ok_or(Unread).and_then(|mut stage| classify(&mut stage, blast, at)))
}

/// Let the body go: awake again and back where the hold found it. A hold of another body (a swapped fighter, a new
/// scene) is only forgotten.
unsafe fn release(slot: &mut Option<CpuPreview>, accessor: usize) -> u32 {
    if let Some(hold) = slot.take() {
        if hold.body() == natives::battle_object_id(accessor) {
            if hold.asleep {
                state::set_sleep(accessor, false);
            }
            placement::restore(accessor, hold.origin());
        }
    }
    SEEN_IDLE
}
