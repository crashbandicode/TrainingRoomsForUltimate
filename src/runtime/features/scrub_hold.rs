//! The clip editor's scrub, the game side (`model::macro_ui::scrub_track`): while In or Out is selected, each role's
//! body is held where the recording had it on that frame, the way a placement picker holds one (`placement`), and
//! put back when nothing is asked of it any more. A held body of the recorded fighter is also given that frame's
//! animation (`model::macro_ui::scrub_pose`), so the scrub shows the move.
//!
//! It runs on a role's own fighter frame, in Training and in the waiting room's practice. It reads the editor's
//! block through the published slot and takes no lock a plugin thread can hold: the holds' lock is only ever tried,
//! and only here.
//!
//! NOT YET SWITCH-RUN.
use super::placement;
use super::stage_geometry::RayProbe;
use crate::engine::macro_ui::scrub_pose::{asked_of, decide, drawn, takes_partner, Act, FALL, WAIT};
use crate::engine::macro_ui::scrub_track::{can_stand, in_bounds, Ask, Block, Spot, POSE_NONE, POSE_SHOWN, SEEN_HELD, SEEN_IDLE, SEEN_OFF, SLOT};
use crate::engine::macros::{PackedPose, Pose};
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
            // `forget` clears both roles' pose reports; the other role's body is still the one the scrub posed, and
            // its own frame lets it go by that report.
            let partner = block.pose(slot ^ 1);
            block.forget();
            block.set_pose(slot ^ 1, partner.0, partner.1);
        }
        // Nobody is asked any more: the other role's body is let go here too, on this callback, so the game never
        // tests the two fighters against each other with one home and one still held. Its own
        // frame then finds no hold and only reports.
        let ask = block.frame_ask(generation);
        if holds[slot ^ 1].is_some() && matches!(ask, Ask::None) {
            let partner = other_body(slot).filter(|&body| holds[slot ^ 1].is_some_and(|hold| hold.body() == natives::battle_object_id(body)));
            if let Some(partner) = partner {
                let seen = release(block, slot ^ 1, &mut holds[slot ^ 1], partner);
                block.set_seen(slot ^ 1, seen);
            }
        }
        let began = holds[slot].is_none();
        let seen = match point(block, &ask, &holds, slot, accessor) {
            Some(spot) => hold(block, generation, slot, &mut holds[slot], accessor, spot),
            None => release(block, slot, &mut holds[slot], accessor),
        };
        // This role's hold began here: the other role's first hold step runs here too, so neither body is free
        // beside a held one for a frame, to be pushed before its hold reads where it stands (`takes_partner`).
        if takes_partner(began, holds[slot].is_some(), holds[slot ^ 1].is_some()) {
            if let Some(partner) = other_body(slot) {
                if let Some(spot) = point(block, &ask, &holds, slot ^ 1, partner) {
                    let taken = hold(block, generation, slot ^ 1, &mut holds[slot ^ 1], partner, spot);
                    block.set_seen(slot ^ 1, taken);
                }
            }
        }
        block.set_seen(slot, seen);
        block.ack(slot, generation);
    });
}

/// The other role's body, found as the placement picker finds the two: the CPU's for Player 1, Player 1's for the
/// CPU.
unsafe fn other_body(slot: usize) -> Option<usize> {
    if slot == 0 {
        super::recorder::cpu_body().map(|body| body.accessor)
    } else {
        natives::module_accessor(0)
    }
}

/// Where this role's body is asked to be. The group's home is where the hold found the target role's body: its
/// hold's origin, which the block keeps when the target is let go while its partner is held (`Block::point`), or
/// where it stands now when it was not held.
unsafe fn point(block: &Block, ask: &Ask, holds: &[Option<CpuPreview>; 2], slot: usize, accessor: usize) -> Option<Spot> {
    block.point_for(ask, slot, |target| holds[target].map(|hold| hold.origin().at), |target| {
        if target == slot {
            return Some([natives::pos_x(accessor), natives::pos_y(accessor)]);
        }
        let pose = crate::engine::rr_host::live_pose(target);
        (pose.fighter_kind >= 0).then_some([pose.x, pose.y])
    })
}

/// Hold the body at `spot`, facing as recorded, and show the frame's animation on it (`pose`). A point outside the
/// camera's limits, one the stage rule does not let a body stand at (inside a solid, past a blast line), or one the
/// game pushed this body sideways off under this request (`Block::pushed`: a wall beside it, a floor's end) is not
/// taken: the body goes back, or stays, where the hold found it. `generation`: the request, as the frame read it
/// before the ask.
unsafe fn hold(block: &Block, generation: u32, index: usize, slot: &mut Option<CpuPreview>, accessor: usize, spot: Spot) -> u32 {
    let Some(limits) = natives::camera_limit_range() else { return release(block, index, slot, accessor) };
    let body = natives::battle_object_id(accessor);
    if !in_bounds(spot.at, limits) || block.refused(index, generation, body, spot.at) || !block.standable(index, generation, spot.at, || stands(accessor, spot.at)) {
        release(block, index, slot, accessor);
        return SEEN_OFF;
    }
    let Some(asleep) = placement::hold_at(slot, accessor, body, spot.at, false) else {
        block.set_pose(index, POSE_NONE, PackedPose::NONE);
        return SEEN_IDLE;
    };
    // The hold's report is about the write of the frame before; the body is where the game left it until this write.
    if block.pushed(index, generation, body, spot.at, slot.and_then(|hold| hold.displaced).map(|moved| moved.at)) {
        release(block, index, slot, accessor);
        return SEEN_OFF;
    }
    state::set_lr(accessor, spot.facing);
    pose(block, index, accessor, spot, asleep);
    SEEN_HELD
}

/// The shared stage rule for a point, read by the placement ray through this role's own body on its own frame. The
/// block asks it again for a new shown frame or a changed world point. A stage that cannot be read lets nobody stand.
unsafe fn stands(accessor: usize, at: [f32; 2]) -> bool {
    let Some(blast) = super::tech_surfaces::blast_bounds() else { return false };
    can_stand(RayProbe::new(accessor).ok_or(Unread).and_then(|mut stage| classify(&mut stage, blast, at)))
}

/// The frame's animation on a held body (SM1). The scrub sets it once to make the body its own, and again only
/// when the game shows another animation or frame; at rate 0 and without its script lines: the scripts sleep
/// under the hold, and the game would otherwise run them up to the frame the animation starts at, hitbox lines
/// included. A body is asked only for an animation it has: the game starts one without that test. The role's
/// frame then reports what the game shows. `asleep`: the hold's own report for this frame. The body's kind is
/// the role's, as its fighter frame published it.
unsafe fn pose(block: &Block, index: usize, accessor: usize, spot: Spot, asleep: bool) {
    let mut owned = block.pose(index).0 == POSE_SHOWN;
    let kind = crate::engine::rr_host::live_pose(index).fighter_kind;
    let read = || (natives::motion_kind(accessor), natives::motion_frame(accessor));
    let asked = asked_of(spot.pose, |motion| natives::is_anim_resource(accessor, motion));
    match decide(spot.kind, asked, kind, asleep, owned, read()) {
        Act::Set(pose) => {
            owned |= natives::change_motion(accessor, pose.motion, pose.frame, 0.0, false);
            // A set the game did not take leaves nothing of the scrub's on the body: the report below says none.
            if natives::motion_kind(accessor) != pose.motion {
                natives::change_motion(accessor, FALL, 0.0, 1.0, true);
            }
        }
        Act::Plain => { natives::change_motion(accessor, FALL, 0.0, 1.0, true); }
        Act::Keep => {}
    }
    let shown = read();
    let state = drawn(spot.kind, asked, kind, asleep, owned, shown);
    block.set_pose(index, state, PackedPose::pack(Pose { motion: shown.0, frame: shown.1 }));
    #[cfg(feature = "diag")]
    super::scrub_pose_probe::note(index, block.shown(), state, spot.pose, shown);
}

/// Let the body go: awake again and back where the hold found it. A body the scrub posed gets its plain animation
/// first, with no blend from the pose, and its ground shape made again before its place is written
/// (`placement::restore_posed`); it is woken last, so it never wakes with a move on it. Any other body is let go
/// as before. A hold of another body (a swapped fighter, a new scene) is only forgotten.
unsafe fn release(block: &Block, index: usize, slot: &mut Option<CpuPreview>, accessor: usize) -> u32 {
    if let Some(hold) = slot.take() {
        if hold.body() == natives::battle_object_id(accessor) {
            if block.pose(index).0 == POSE_SHOWN {
                placement::restore_posed(accessor, hold.origin(), if hold.origin().grounded { WAIT } else { FALL });
                if hold.asleep {
                    state::set_sleep(accessor, false);
                }
            } else {
                if hold.asleep {
                    state::set_sleep(accessor, false);
                }
                placement::restore(accessor, hold.origin());
            }
            #[cfg(feature = "diag")]
            super::scrub_pose_probe::note(index, block.shown(), POSE_NONE, PackedPose::NONE, (natives::motion_kind(accessor), natives::motion_frame(accessor)));
        }
    }
    block.set_pose(index, POSE_NONE, PackedPose::NONE);
    SEEN_IDLE
}
