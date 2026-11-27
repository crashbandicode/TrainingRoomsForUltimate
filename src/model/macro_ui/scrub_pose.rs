//! The clip editor's scrub, the pose (SM1, `docs/DECISIONS.md`, 2026-11-26): what the game side does with a held
//! fighter's animation on one fighter frame, and the diag line that says what it drew. The game calls are
//! `runtime::features::scrub_hold`'s; nothing here reads the game.
//!
//! NOT YET SWITCH-RUN.
#[cfg(any(feature = "diag", test))]
use core::fmt;

use super::scrub_track::{POSE_NONE, POSE_OTHER, POSE_SHOWN};
use crate::engine::macros::{PackedPose, Pose};
use crate::model::hash40::hash40;

/// The plain animations a held body goes back to: standing, and falling.
pub const WAIT: u64 = hash40("wait");
pub const FALL: u64 = hash40("fall");

/// What the scrub does with the body's animation on this fighter frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Act {
    /// Nothing: the game already shows what is asked, or the body is not the scrub's to pose.
    Keep,
    /// Start the recorded animation at its frame, at rate 0, without its script lines.
    Set(Pose),
    /// Put the plain falling animation back on a body the scrub had posed.
    Plain,
}

/// The pose asked of a body: the recorded one when it unpacks and the body has an animation of that name (`has`,
/// which is asked of the game only then), else none. The game starts an animation it is given without a test
/// that the fighter has one of that name.
pub fn asked_of(recorded: PackedPose, has: impl FnOnce(u64) -> bool) -> PackedPose {
    match recorded.get() {
        Some(pose) if has(pose.motion) => recorded,
        _ => PackedPose::NONE,
    }
}

/// The start of a hold, the same way: the role whose hold `began` on this callback (`held` after its own step)
/// runs its partner's first hold step there too, when the partner has none yet. Held a frame apart, the body not
/// yet held could be pushed by the one already placed, and that pushed place would be its origin: where it is put
/// back at the release, and the group's home when it is the target.
pub fn takes_partner(began: bool, held: bool, partner_held: bool) -> bool {
    began && held && !partner_held
}

/// A fighter the scrub places from its partner's callback (put back at the release, or first held at the start) is
/// left asleep there, and is woken only on its own callback. The game writes a push into two bodies whose boxes
/// overlap, held or not, after both fighters' frames. A body's own frame function turns the push it has into speed
/// only while the body is awake, and the game clears an awake body's push before it tests the pair again. So a
/// body woken on its own callback, after its own frame function, has that push cleared before it can use it; one
/// woken from its partner's callback, before its own frame function, slides (place-t1a's trace D, 2026-11-27: the
/// CPU 0.4 on the first frame, braked, 1.98 in all).
///
/// `asleep`: the hold has the body asleep. `Some(asleep)` to write: an awake body is put to sleep, a sleeping one is
/// left, and none is ever woken from here.
pub fn partner_sleep(asleep: bool) -> Option<bool> {
    (!asleep).then_some(true)
}

/// One fighter frame of a held role. `asked_kind`, `asked`: the recorded fighter and its animation on the shown
/// frame (`Spot`). `body_kind`: the fighter the body is now (below 0: not known). `asleep`: the hold reports the
/// body held with its scripts asleep. `posed`: the scrub's own animation is on this body. `shown`: the animation
/// and frame the game shows now.
pub fn decide(asked_kind: i32, asked: PackedPose, body_kind: i32, asleep: bool, posed: bool, shown: (u64, f32)) -> Act {
    if !asleep {
        return Act::Keep;
    }
    match asked.get().filter(|_| body_kind >= 0 && body_kind == asked_kind) {
        Some(pose) if !posed || shown != (pose.motion, pose.frame) => Act::Set(pose),
        Some(_) => Act::Keep,
        None if posed => Act::Plain,
        None => Act::Keep,
    }
}

/// What the role's frame reports (`Block::set_pose`), from what the game shows after `decide`'s act. `POSE_SHOWN`
/// means the scrub issued the setter and the body has that motion. The actual frame is reported separately.
pub fn drawn(asked_kind: i32, asked: PackedPose, body_kind: i32, asleep: bool, owned: bool, shown: (u64, f32)) -> u32 {
    if !asleep {
        return if owned { POSE_SHOWN } else { POSE_NONE };
    }
    if body_kind < 0 || body_kind != asked_kind {
        return POSE_OTHER;
    }
    if owned && asked.get().is_some_and(|pose| pose.motion == shown.0) { POSE_SHOWN } else { POSE_NONE }
}

/// The diag build's line for one role, written on its fighter frame when what is shown changes:
/// `[Scrub] pose role=<p1|cpu> frame=<clip frame|none> state=<shown|none|other_fighter> asked=<0xNAME,FRAME|none> shown=<0xNAME,FRAME>`.
/// `shown` is read back from the game after the set; frames print in the float's shortest form.
#[cfg(any(feature = "diag", test))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseLine {
    pub role: usize,
    pub frame: Option<u32>,
    pub state: u32,
    pub asked: PackedPose,
    pub shown: (u64, f32),
}

#[cfg(any(feature = "diag", test))]
impl PoseLine {
    /// What makes a line new for its role: the clip frame, the state, the animation asked for and the one shown.
    /// The shown animation's frame counts only on a posed body: on any other it runs on with the game.
    pub fn key(&self) -> [u64; 4] {
        [
            u64::from(self.frame.map_or(0, |frame| frame.wrapping_add(1))) | u64::from(self.state) << 32,
            u64::from(self.asked.0[0]) | u64::from(self.asked.0[1]) << 32,
            self.shown.0,
            if self.state == POSE_SHOWN { u64::from(self.shown.1.to_bits()) } else { 0 },
        ]
    }
}

#[cfg(any(feature = "diag", test))]
impl fmt::Display for PoseLine {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "[Scrub] pose role={} frame=", if self.role == 0 { "p1" } else { "cpu" })?;
        match self.frame {
            Some(frame) => write!(out, "{frame}")?,
            None => out.write_str("none")?,
        }
        out.write_str(match self.state {
            POSE_SHOWN => " state=shown asked=",
            POSE_OTHER => " state=other_fighter asked=",
            _ => " state=none asked=",
        })?;
        match self.asked.get() {
            Some(pose) => write!(out, "{:#x},{}", pose.motion, pose.frame)?,
            None => out.write_str("none")?,
        }
        write!(out, " shown={:#x},{}", self.shown.0, self.shown.1)
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use super::super::scrub_track::{Ask, Block, Spot};
    use crate::model::crosshair::{hold_step, BodyRead, CpuPreview, Origin, PreviewAct};

    const KIRBY: i32 = 6;
    const SMASH: u64 = 0x12_3456_789a;

    fn pose(frame: f32) -> PackedPose {
        PackedPose::pack(Pose { motion: SMASH, frame })
    }

    #[test]
    fn matching_pose_is_claimed_once_by_the_scrubs_own_setter() {
        let asked = pose(12.5);
        let natural = (SMASH, 12.5);
        assert_eq!(decide(KIRBY, asked, KIRBY, true, false, natural), Act::Set(asked.get().unwrap()),
            "a natural match is not a claim: the first eligible callback must issue the stopped setter");
        assert_eq!(decide(KIRBY, asked, KIRBY, true, true, natural), Act::Keep);
    }

    #[test]
    fn an_owned_pose_corrects_the_actual_frame_inside_one_packed_step() {
        let asked = pose(12.5);
        for frame in [12.5 - 1.0 / 1024.0, 12.5 + 1.0 / 1024.0] {
            assert_eq!(decide(KIRBY, asked, KIRBY, true, true, (SMASH, frame)), Act::Set(asked.get().unwrap()),
                "correct the game read itself, even when it would round to the requested frame");
        }
        assert_eq!(decide(KIRBY, asked, KIRBY, true, true, (SMASH, 12.5)), Act::Keep);
    }

    #[test]
    fn a_pose_is_set_on_a_sleeping_body_of_the_recorded_fighter_when_the_game_shows_another() {
        let asked = pose(12.5);
        // The held body falls; the recorded frame is mid-smash.
        assert_eq!(decide(KIRBY, asked, KIRBY, true, false, (FALL, 3.0)), Act::Set(Pose { motion: SMASH, frame: 12.5 }));
        // The same animation on another frame: the playhead moved.
        assert_eq!(decide(KIRBY, asked, KIRBY, true, true, (SMASH, 9.0)), Act::Set(Pose { motion: SMASH, frame: 12.5 }));
        // The game already shows it: nothing is set again.
        assert_eq!(decide(KIRBY, asked, KIRBY, true, true, (SMASH, 12.5)), Act::Keep);
        // A frame the file keeps in 256ths: the read-back is compared as the file keeps it.
        assert_eq!(decide(KIRBY, pose(9.013), KIRBY, true, true, (SMASH, pose(9.013).get().unwrap().frame)), Act::Keep);
        // Awake (the hold's placing frames): never.
        assert_eq!(decide(KIRBY, asked, KIRBY, false, false, (FALL, 3.0)), Act::Keep);
    }

    #[test]
    fn another_fighter_or_a_frame_without_a_pose_is_never_posed_and_a_posed_body_goes_plain_once() {
        let asked = pose(12.5);
        for (kind, pose) in [(0, asked), (-1, asked), (KIRBY, PackedPose::NONE), (KIRBY, PackedPose([0, 0x500]))] {
            // Never posed: the body is left exactly as it is.
            assert_eq!(decide(KIRBY, pose, kind, true, false, (FALL, 3.0)), Act::Keep, "{kind} {pose:?}");
            assert_eq!(decide(KIRBY, pose, kind, true, false, (SMASH, 12.5)), Act::Keep, "{kind} {pose:?}");
            // The scrub's own move is on it: the plain animation goes back, and then nothing more.
            assert_eq!(decide(KIRBY, pose, kind, true, true, (SMASH, 12.5)), Act::Plain, "{kind} {pose:?}");
            assert_eq!(decide(KIRBY, pose, kind, true, false, (FALL, 0.0)), Act::Keep, "{kind} {pose:?}");
        }
    }

    #[test]
    fn the_report_is_what_the_game_shows() {
        let asked = pose(12.5);
        assert_eq!(drawn(KIRBY, asked, KIRBY, true, false, (SMASH, 12.5)), POSE_NONE, "a matching motion without a setter call is not ours");
        assert_eq!(drawn(KIRBY, asked, KIRBY, true, true, (SMASH, 12.5)), POSE_SHOWN);
        // The game kept the animation and clamped the frame: the move is the scrub's, and the line carries the frame.
        assert_eq!(drawn(KIRBY, asked, KIRBY, true, true, (SMASH, 11.0)), POSE_SHOWN);
        // The fighter has no such animation: nothing of the scrub's is on it.
        assert_eq!(drawn(KIRBY, asked, KIRBY, true, false, (FALL, 0.0)), POSE_NONE);
        assert_eq!(drawn(KIRBY, PackedPose::NONE, KIRBY, true, true, (FALL, 0.0)), POSE_NONE);
        assert_eq!(drawn(KIRBY, asked, 0, true, false, (FALL, 0.0)), POSE_OTHER);
        assert_eq!(drawn(KIRBY, asked, -1, true, false, (FALL, 0.0)), POSE_OTHER);
        // An awake frame of a hold changes nothing about whose animation is on the body.
        assert_eq!(drawn(KIRBY, asked, KIRBY, false, true, (SMASH, 12.5)), POSE_SHOWN);
        assert_eq!(drawn(KIRBY, asked, KIRBY, false, false, (SMASH, 12.5)), POSE_NONE);
    }

    #[test]
    fn the_diag_line_reads_as_the_exact_frame_rows_expect() {
        let line = |role, frame, state, asked, shown| PoseLine { role, frame, state, asked, shown }.to_string();
        assert_eq!(line(0, Some(137), POSE_SHOWN, pose(12.5), (SMASH, 12.5)),
            "[Scrub] pose role=p1 frame=137 state=shown asked=0x123456789a,12.5 shown=0x123456789a,12.5");
        assert_eq!(line(1, Some(7), POSE_OTHER, pose(22.0), (0xab, 3.0)),
            "[Scrub] pose role=cpu frame=7 state=other_fighter asked=0x123456789a,22 shown=0xab,3");
        assert_eq!(line(1, None, POSE_NONE, PackedPose::NONE, (0xab, 0.0)),
            "[Scrub] pose role=cpu frame=none state=none asked=none shown=0xab,0");
    }

    #[test]
    fn a_line_is_new_only_when_what_it_says_changes() {
        let first = PoseLine { role: 0, frame: Some(7), state: POSE_SHOWN, asked: pose(12.5), shown: (SMASH, 12.5) };
        assert_eq!(first.key(), PoseLine { role: 1, ..first }.key(), "the role has its own memory");
        for other in [
            PoseLine { frame: Some(8), ..first }, PoseLine { frame: None, ..first }, PoseLine { state: POSE_NONE, ..first },
            PoseLine { asked: pose(13.0), ..first }, PoseLine { shown: (SMASH, 12.0), ..first }, PoseLine { shown: (FALL, 12.5), ..first },
        ] {
            assert_ne!(first.key(), other.key(), "{other:?}");
        }
        // A body that is not posed runs its own animation on: its frame alone is not a new line.
        let free = PoseLine { state: POSE_NONE, asked: PackedPose::NONE, shown: (FALL, 3.0), ..first };
        assert_eq!(free.key(), PoseLine { shown: (FALL, 4.0), ..free }.key());
        assert_ne!(free.key(), PoseLine { shown: (WAIT, 3.0), ..free }.key());
    }

    #[test]
    fn only_the_scrubs_pose_starts_an_animation_without_its_script_lines() {
        let game = include_str!("../../runtime/game.rs");
        assert!(game.contains("pub unsafe fn change_motion(accessor: usize, motion: u64, frame: f32, rate: f32, run_lines: bool) -> bool {"));
        assert!(game.contains("f(accessor, motion, frame, rate, false, 0.0, false, !run_lines);"), "the native's last argument skips the lines");
        let picker = include_str!("../../runtime/features/record_place.rs");
        assert_eq!(picker.matches("natives::change_motion(").count(), 1);
        assert!(picker.contains("natives::change_motion(self.0, motion, 0.0, 1.0, true);"), "the picker's call is as it was");
        let hold = include_str!("../../runtime/features/scrub_hold.rs");
        assert!(hold.contains("natives::change_motion(accessor, pose.motion, pose.frame, 0.0, false);"), "rate 0, no lines");
        assert_eq!(hold.matches(", false);").count() - hold.matches("state::set_sleep(accessor, false);").count(), 1, "one call without lines");
    }

    #[test]
    fn a_posed_body_is_let_go_plain_then_restored_then_woken_and_any_other_as_before() {
        let hold = include_str!("../../runtime/features/scrub_hold.rs");
        let release = hold.split_once("unsafe fn release(block: &Block, index: usize, slot: &mut Option<CpuPreview>, accessor: usize) -> u32 {").map_or("", |(_, rest)| rest);
        let all = |text: &str| release.match_indices(text).map(|(at, _)| at).collect::<Vec<_>>();
        let posed = all("if block.pose(index).0 == POSE_SHOWN {");
        let plain = all("placement::restore_posed(accessor, hold.origin(), if hold.origin().grounded { WAIT } else { FALL });");
        let (restore, wake) = (all("placement::restore(accessor, hold.origin());"), all("state::set_sleep(accessor, false);"));
        assert_eq!((posed.len(), plain.len(), restore.len(), wake.len()), (1, 1, 1, 2));
        // The scrub's own body: its plain animation and its place in one call, then the wake. Any other body: today's two
        // lines, in today's order, and nothing else.
        assert!(posed[0] < plain[0] && plain[0] < wake[0] && wake[0] < wake[1] && wake[1] < restore[0]);
        assert!(!release.contains("natives::change_motion"), "the posed release's change is the cut, made in placement.rs");
        assert!(release.contains("block.set_pose(index, POSE_NONE, PackedPose::NONE);"), "a body let go shows nothing of the scrub's");
    }

    /// The slide after a release (place-t1a's traces, 2026-11-27 00:35Z and 01:06Z; rv-preview-vis's read B, 01:15Z):
    /// the plain animation blended in from the pose, and the box the game pushes fighters by was made from the posed
    /// joints when the body's place was written, so the two fighters were pushed apart. tr-primary-2, 01:16:14Z:
    /// release with no blend, make the shape again from the released joints, then write the place, in that order,
    /// with facing and situation first; on the posed body only.
    #[test]
    fn a_posed_bodys_release_cuts_to_the_plain_animation_and_makes_its_shape_again_before_its_place() {
        let placement = include_str!("../../runtime/features/placement.rs");
        let item = |head: &str| placement.split(head).nth(1).unwrap_or("").split("\n}").next().unwrap_or("");
        let lines = |text: &'static str| text.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with("//")).collect::<Vec<_>>();
        // The game's one-shot is armed only when the change is certain to follow (its native granted, and the fighter
        // has the animation), and the change is the very next call: left armed, an unrelated change would use it.
        assert_eq!(lines(item("pub(super) unsafe fn restore_posed(body: usize, origin: Origin, motion: u64) {")), [
            "use crate::model::sites_pkg2::MOTION_CHANGE_HOOK_OFFSET;",
            "state::set_lr(body, origin.facing);",
            "state::set_situation_kind(body, konst(if origin.grounded { C::SituationGround } else { C::SituationAir }));",
            "if game::granted(MOTION_CHANGE_HOOK_OFFSET) && natives::is_anim_resource(body, motion) {",
            "natives::set_next_no_comp(body);",
            "}",
            "natives::change_motion(body, motion, 0.0, 1.0, true);",
            "natives::ground_update_shape(body, true);",
            "restore(body, origin);",
        ]);
        // The place is written as it always was, for every body: the speed cleared, then `init_pos`.
        assert_eq!(lines(item("pub(super) unsafe fn restore(body: usize, origin: Origin) {")), [
            "if origin.grounded {",
            "stand(body, origin.at, origin.facing);",
            "} else {",
            "lift(body, origin.at, origin.facing);",
            "fall(body);",
            "}",
        ]);
        for writer in ["pub(super) unsafe fn ground(", "pub(super) unsafe fn lift("] {
            let body = lines(item(writer));
            assert_eq!(body[1..3], ["state::clear_speed_all(body);", "state::init_pos(body, &state::Vector3f { x, y, z: 0.0 });"], "{writer}");
        }
        // Only the scrub's posed release makes the two calls: no other release, no picker and no drill.
        assert_eq!(placement.matches("natives::set_next_no_comp(").count(), 1);
        assert_eq!(placement.matches("natives::ground_update_shape(").count(), 1);
        assert_eq!(placement.matches("restore_posed(").count(), 1);
        for other in [include_str!("../../runtime/features/record_place.rs"), include_str!("../../runtime/features/edge_juggle.rs")] {
            assert!(!other.contains("restore_posed(") && !other.contains("set_next_no_comp(") && !other.contains("ground_update_shape("));
        }
        assert!(!placement.contains("jostle") && !placement.contains("as *mut"), "no push switch, and no write to a field");
        // Each of the two is one plain call of its native. The change itself is the shared one, as it was for every caller.
        let game = include_str!("../../runtime/game.rs");
        let native = |head: &str| lines(game.split(head).nth(1).unwrap_or("").split("\n    }").next().unwrap_or(""));
        assert_eq!(native("pub unsafe fn set_next_no_comp(accessor: usize) {"),
            ["at::<extern \"C\" fn(usize)>(sites_pkg2::MOTION_SET_NEXT_NO_COMP_OFFSET).map_or_else(Default::default, |f| f(accessor))"]);
        assert_eq!(native("pub unsafe fn ground_update_shape(accessor: usize, update: bool) {"),
            ["at::<extern \"C\" fn(usize, bool)>(sites_pkg2::GROUND_UPDATE_SHAPE_OFFSET).map_or_else(Default::default, |f| f(accessor, update))"]);
        assert_eq!(native("pub unsafe fn change_motion(accessor: usize, motion: u64, frame: f32, rate: f32, run_lines: bool) -> bool {"), [
            "if let Some(f) = at::<extern \"C\" fn(usize, u64, f32, f32, bool, f32, bool, bool) -> u64>(sites_pkg2::MOTION_CHANGE_HOOK_OFFSET) {",
            "f(accessor, motion, frame, rate, false, 0.0, false, !run_lines);",
            "true",
            "} else { false }",
        ]);
        assert_eq!(game.matches("fn change_motion").count(), 1, "one wrapper of the change");
        assert_eq!(game.matches("MOTION_SET_NEXT_NO_COMP_OFFSET").count(), 1, "armed in one place");
    }

    #[test]
    fn the_pose_is_reported_on_the_roles_own_frame_and_survives_the_partners_swap() {
        let hold = include_str!("../../runtime/features/scrub_hold.rs");
        assert!(hold.contains("state::set_lr(accessor, spot.facing);\n    pose(block, index, accessor, spot, asleep);\n    SEEN_HELD"));
        let order = |texts: &[&str]| texts.iter().map(|text| hold.find(text)).collect::<Vec<_>>().windows(2).all(|pair| pair[0].is_some() && pair[0] < pair[1]);
        // `forget` clears both roles' cells; the other role's body is still the one the scrub posed.
        assert!(order(&["let partner = block.pose(slot ^ 1);", "block.forget();", "block.set_pose(slot ^ 1, partner.0, partner.1);"]));
        // A set the fighter did not take (it has no animation of that name) leaves nothing of the scrub's on it.
        assert!(order(&["natives::change_motion(accessor, pose.motion, pose.frame, 0.0, false);",
            "if natives::motion_kind(accessor) != pose.motion {", "natives::change_motion(accessor, FALL, 0.0, 1.0, true);", "let shown = read();"]));
        // Every way out of the role's frame reports: a body let go, a hold that could not be made, a held body.
        assert!(order(&["None => release(block, slot, &mut holds[slot], accessor),", "block.set_seen(slot, seen);\n        block.ack(slot, generation);"]),
            "a body is let go before its frame reports the release");
        assert!(hold.contains("else {\n        block.set_pose(index, POSE_NONE, PackedPose::NONE);\n        return SEEN_IDLE;\n    };"));
        assert_eq!(hold.matches("release(block, index, slot, accessor)").count(), 3, "no camera limits, a refused point, a pushed body");
        // The animation only: the status is never forced.
        assert!(!hold.contains("status_request") && !hold.contains("change_status"));
        assert!(hold.contains("crate::engine::rr_host::live_pose(index).fighter_kind"), "the body's kind, as the recorder publishes it");
        assert!(!hold.contains("fighter_kind(accessor)"), "no native outside the placement unit's set");
    }

    #[test]
    fn a_pose_is_asked_only_of_a_body_that_has_the_animation() {
        let recorded = pose(12.5);
        assert_eq!(asked_of(recorded, |motion| motion == SMASH), recorded);
        assert_eq!(asked_of(recorded, |_| false), PackedPose::NONE, "the fighter has no animation of that name");
        // A row without a pose asks the game nothing.
        assert_eq!(asked_of(PackedPose::NONE, |_| panic!("no name to ask about")), PackedPose::NONE);
        assert_eq!(asked_of(PackedPose([0, 0x500]), |_| panic!("no name to ask about")), PackedPose::NONE);
        // So such a body is never given the setter, and one the scrub owns goes plain and reports none.
        let asked = asked_of(recorded, |_| false);
        assert_eq!(decide(KIRBY, asked, KIRBY, true, false, (FALL, 3.0)), Act::Keep);
        assert_eq!(drawn(KIRBY, asked, KIRBY, true, false, (FALL, 3.0)), POSE_NONE);
        assert_eq!(decide(KIRBY, asked, KIRBY, true, true, (SMASH, 9.0)), Act::Plain);
        assert_eq!(drawn(KIRBY, asked, KIRBY, true, true, (FALL, 0.0)), POSE_NONE);
    }

    #[test]
    fn the_scrub_asks_the_game_for_the_animation_before_it_claims_the_body() {
        let game = include_str!("../../runtime/game.rs");
        assert!(game.contains("pub unsafe fn is_anim_resource(accessor: usize, motion: u64) -> bool {"));
        assert!(game.contains("at::<extern \"C\" fn(usize, u64) -> bool>(sites_pkg2::MOTION_IS_ANIM_RESOURCE_OFFSET).is_some_and(|f| f(accessor, motion))"),
            "a refused row answers no: no pose");
        let hold = include_str!("../../runtime/features/scrub_hold.rs");
        let order = |texts: &[&str]| texts.iter().map(|text| hold.find(text)).collect::<Vec<_>>().windows(2).all(|pair| pair[0].is_some() && pair[0] < pair[1]);
        assert!(order(&["let asked = asked_of(spot.pose, |motion| natives::is_anim_resource(accessor, motion));",
            "match decide(spot.kind, asked, kind, asleep, owned, read()) {", "let state = drawn(spot.kind, asked, kind, asleep, owned, shown);"]));
        assert_eq!(hold.matches("spot.pose").count(), 2, "the pose as recorded goes to the question and to the diag line only");
        // The row is the placement unit's own: a refused one turns that unit off and no other.
        let row = crate::model::capabilities::find("motion_is_anim_resource").expect("the guarded row");
        assert_eq!((row.offset, row.feature), (0x205_ce10, crate::model::sites::Feature::RecordPlace));
        assert!(matches!(row.kind, crate::model::guard::SeamKind::Called));
    }

    #[test]
    fn a_posed_role_whose_partners_fighter_was_swapped_is_still_let_go_as_posed() {
        let block = super::super::scrub_track::Block::new();
        let shown = pose(12.5);
        // Both roles are posed. Player 1's fighter is swapped: its frame finds a hold made on another body.
        block.set_pose(0, POSE_SHOWN, shown);
        block.set_pose(1, POSE_SHOWN, shown);
        let partner = block.pose(1);
        block.forget();
        assert_eq!([block.pose(0), block.pose(1)], [(POSE_NONE, PackedPose::NONE); 2], "forget clears both roles' reports");
        block.set_pose(1, partner.0, partner.1);
        // The CPU's body is still the one the scrub posed: its own frame lets it go by this report.
        assert_eq!(block.pose(1), (POSE_SHOWN, shown));
        assert_eq!(block.pose(0), (POSE_NONE, PackedPose::NONE), "the swapped role's new body was never posed");
        let hold = include_str!("../../runtime/features/scrub_hold.rs");
        let swapped = hold.split_once("if holds[slot].is_some_and(|hold| hold.body() != natives::battle_object_id(accessor)) {").map_or("", |(_, rest)| rest);
        let swapped = swapped.split_once("\n        }\n").map_or("", |(inside, _)| inside);
        assert_eq!(swapped.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with("//")).collect::<Vec<_>>(),
            ["let partner = block.pose(slot ^ 1);", "block.forget();", "block.set_pose(slot ^ 1, partner.0, partner.1);"]);
        // The plain animation before the wake: the posed arm of the release reads that report.
        let release = hold.split_once("unsafe fn release(").map_or("", |(_, rest)| rest);
        let posed = release.find("if block.pose(index).0 == POSE_SHOWN {").expect("the release reads the role's report");
        let plain = release.find("placement::restore_posed(accessor, hold.origin(), if hold.origin().grounded { WAIT } else { FALL });").expect("the plain animation");
        assert!(posed < plain && plain < release.find("state::set_sleep(accessor, false);").expect("the wake"));
    }

    /// The two fighters stand free 9.5 apart. The recorded frame has them 6.25 apart: with a move's reach on both
    /// (`WIDE`) their boxes overlap there; with a standing fighter's (`PLAIN`) they do not.
    const HOME: [f32; 2] = [-8.0, 1.5];
    const WIDE: f32 = 4.0;
    const PLAIN: f32 = 2.5;
    /// What the game's pair test writes into each of two overlapping bodies, and how its brake runs a speed down.
    const PUSH: f32 = 0.5;
    const BRAKE: f32 = 0.125;

    /// The game around the scrub, a tick at a time, as place-t1a's traces show a tick: each fighter's frame function
    /// and then our callback on it; then the game's test of the pair, which writes a push into both bodies when their
    /// boxes overlap, asleep or not. The callback is `scrub_hold::frame`'s steps, on the real block and the real hold.
    struct Stage {
        block: Box<Block>,
        holds: [Option<CpuPreview>; 2],
        ids: [u32; 2],
        x: [f32; 2],
        speed: [f32; 2],
        /// The push in the body's module since the last pair test.
        push: [f32; 2],
        asleep: [bool; 2],
        /// The body's box has the reach of the scrub's animation: its own frame function makes it from the animation.
        wide: [bool; 2],
        /// The body's own game frames (the hold's clock).
        clock: [u32; 2],
        /// Its partner's callback put the body back; its own callback has not let it go yet.
        back: [bool; 2],
        /// A hold let the body go because the game had pushed it off its point.
        refused: [bool; 2],
        /// The recorded frame has an animation for the fighters.
        poses: bool,
    }

    impl Stage {
        fn new(poses: bool) -> Self {
            Self { block: Block::new(), holds: [None; 2], ids: [1, 2], x: HOME, speed: [0.0; 2], push: [0.0; 2], asleep: [false; 2],
                wide: [false; 2], clock: [0; 2], back: [false; 2], refused: [false; 2], poses }
        }

        /// The recorded frame, placed from where Player 1 stands. `cpu`: the CPU is asked for too.
        fn group(cpu: bool) -> Ask {
            let spot = |x: f32| Spot { at: [x, 0.0], facing: 1.0, kind: KIRBY, pose: pose(12.5) };
            Ask::Group { target: 0, deltas: [Some(spot(5.75)), cpu.then(|| spot(12.0))] }
        }

        /// One tick: the fighters in `order`, then the pair test. `publish`: the worker publishes a request before the
        /// first fighter's frame (0), between the two fighters (1) or after both callbacks (2).
        fn tick(&mut self, order: [usize; 2], publish: Option<(usize, Ask)>) {
            for step in 0..3 {
                if let Some((_, ask)) = publish.filter(|(at, _)| *at == step) {
                    self.block.set_ask(ask);
                }
                if let Some(&role) = order.get(step) {
                    self.update(role);
                    self.callback(role);
                }
            }
            self.pair_test();
        }

        /// The fighter's own frame function. Awake, it turns the push in its module into speed and moves.
        fn update(&mut self, i: usize) {
            self.clock[i] = self.clock[i].wrapping_add(1);
            self.wide[i] = self.block.pose(i).0 == POSE_SHOWN;
            if !self.asleep[i] {
                self.speed[i] += self.push[i];
                assert!(!self.back[i] || self.speed[i] == 0.0,
                    "role {i}, put back from its partner's callback, kept the push into its own update: speed {}", self.speed[i]);
                self.x[i] += self.speed[i];
                self.speed[i] -= self.speed[i].signum() * BRAKE.min(self.speed[i].abs());
            }
        }

        /// The game's test of the pair, after both fighters: an awake body's push is cleared, and two overlapping
        /// boxes get one each.
        fn pair_test(&mut self) {
            for (push, asleep) in self.push.iter_mut().zip(self.asleep) {
                if !asleep {
                    *push = 0.0;
                }
            }
            let reach = |wide: bool| if wide { WIDE } else { PLAIN };
            if (self.x[1] - self.x[0]).abs() < reach(self.wide[0]) + reach(self.wide[1]) {
                let left = usize::from(self.x[1] < self.x[0]);
                (self.push[left], self.push[left ^ 1]) = (-PUSH, PUSH);
            }
        }

        /// `scrub_hold::frame` for the role in `slot`.
        fn callback(&mut self, slot: usize) {
            let (generation, other) = (self.block.generation(), slot ^ 1);
            if self.holds[slot].is_some_and(|hold| hold.body() != self.ids[slot]) {
                let partner = self.block.pose(other);
                self.block.forget();
                self.block.set_pose(other, partner.0, partner.1);
            }
            let ask = self.block.frame_ask(generation);
            if matches!(ask, Ask::None) && self.holds[other].is_some_and(|hold| hold.body() == self.ids[other]) {
                self.put_back(other);
            }
            let began = self.holds[slot].is_none();
            match self.point(&ask, slot) {
                Some(at) => self.hold(generation, slot, at),
                None => self.release(slot),
            }
            if takes_partner(began, self.holds[slot].is_some(), self.holds[other].is_some()) {
                if let Some(at) = self.point(&ask, other) {
                    self.hold(generation, other, at);
                    self.leave_asleep(other);
                }
            }
        }

        fn point(&self, ask: &Ask, slot: usize) -> Option<f32> {
            self.block.point_for(ask, slot, |target| self.holds[target].map(|hold| hold.origin().at), |target| Some([self.x[target], 0.0]))
                .map(|spot| spot.at[0])
        }

        /// `scrub_hold::hold` with `placement::hold_at`: the body is read as the game left it, then written at its
        /// point, awake while it is being placed and asleep once held. A body the game moved off the point is let go.
        fn hold(&mut self, generation: u32, i: usize, at: f32) {
            let (point, id) = ([at, 0.0], self.ids[i]);
            if self.block.refused(i, generation, id, point) {
                self.release(i);
                return;
            }
            let read = BodyRead { at: [self.x[i], 0.0], landed: false, speed: Some([self.speed[i], 0.0]), frame: self.clock[i] };
            let act = hold_step(&mut self.holds[i], id, false, || Origin { at: read.at, grounded: true, facing: 1.0 });
            let Some(hold) = self.holds[i].as_mut() else { return };
            hold.observe(point, read, false);
            hold.asleep = act == PreviewAct::Hold;
            (self.x[i], self.speed[i], self.asleep[i]) = (at, 0.0, hold.asleep);
            if self.block.pushed(i, generation, id, point, hold.displaced.map(|moved| moved.at)) {
                self.refused[i] = true;
                self.release(i);
            } else if self.asleep[i] && self.poses {
                self.block.set_pose(i, POSE_SHOWN, pose(12.5));
            }
        }

        /// `scrub_hold::release`: the role's own callback lets its body go, awake, where the hold found it.
        fn release(&mut self, i: usize) {
            let id = self.ids[i];
            if let Some(hold) = self.holds[i].take().filter(|hold| hold.body() == id) {
                (self.x[i], self.speed[i], self.asleep[i], self.wide[i], self.back[i]) = (hold.origin().at[0], 0.0, false, false, false);
            }
            self.block.set_pose(i, POSE_NONE, PackedPose::NONE);
        }

        /// `scrub_hold::put_back`: from its partner's callback the body goes back where its hold found it, with a
        /// standing fighter's box. Its hold stays for its own callback.
        fn put_back(&mut self, i: usize) {
            let Some(hold) = self.holds[i] else { return };
            (self.x[i], self.speed[i], self.wide[i], self.back[i]) = (hold.origin().at[0], 0.0, false, true);
            self.block.set_pose(i, POSE_NONE, PackedPose::NONE);
            self.leave_asleep(i);
        }

        /// `scrub_hold::leave_asleep`.
        fn leave_asleep(&mut self, i: usize) {
            let Some(hold) = self.holds[i].as_mut() else { return };
            if let Some(asleep) = partner_sleep(hold.asleep) {
                self.asleep[i] = asleep;
                hold.rest();
            }
        }
    }

    /// place-t1a's trace D (2026-11-27 04:56Z, six posed rows of six): Player 1's callback read the release first,
    /// put the CPU back and woke it before the CPU's own frame function. The push the game had written into the held,
    /// posed CPU on the tick before was still in its module, and the CPU slid 1.98. tr-primary-2, 04:56:54Z: the push
    /// the game writes into a held, posed body must not survive into that body's own update after our release.
    #[test]
    fn a_fighter_put_back_from_its_partners_callback_keeps_no_push_into_its_own_update() {
        let held = |order: [usize; 2], poses: bool| {
            let mut stage = Stage::new(poses);
            stage.tick(order, Some((0, Stage::group(true))));
            for _ in 0..8 {
                stage.tick(order, None);
            }
            stage
        };
        for (order, published, poses) in [[0, 1], [1, 0]].into_iter()
            .flat_map(|order| (0..3).flat_map(move |published| [true, false].map(move |poses| (order, published, poses)))) {
            let case = format!("order {order:?}, published at {published}, posed {poses}");
            let mut stage = held(order, poses);
            // Held: both asleep at the recorded frame. Posed, their boxes overlap and the game writes a push every tick.
            assert_eq!((stage.x, stage.asleep, stage.refused), ([-2.25, 4.0], [true; 2], [false; 2]), "{case}");
            assert_eq!(stage.push, if poses { [-PUSH, PUSH] } else { [0.0; 2] }, "{case}");
            stage.tick(order, Some((published, Ask::None)));
            // From the first callback that reads the release on, the game never tests the pair with one fighter home and
            // the other at its held point (the staggered release, trace C), and neither fighter moves again.
            let mut read = published < 2;
            for tick in 0..6 {
                if read {
                    assert_eq!((stage.x, stage.speed), (HOME, [0.0; 2]), "{case}, tick {tick} after the release");
                }
                read = true;
                stage.tick(order, None);
            }
            assert_eq!((stage.holds.map(|hold| hold.is_some()), stage.asleep, stage.push), ([false; 2], [false; 2], [0.0; 2]), "{case}: both free");
            assert_eq!((stage.x, stage.speed), (HOME, [0.0; 2]), "{case}");
        }
        // A request that still asks Player 1 is no release: the CPU is let go alone, by its own callback, as before.
        let mut stage = held([0, 1], true);
        stage.tick([0, 1], Some((0, Stage::group(false))));
        assert_eq!((stage.holds.map(|hold| hold.is_some()), stage.x, stage.asleep, stage.back), ([true, false], [-2.25, 1.5], [true, false], [false; 2]));
        // A hold found on another body is forgotten and nobody is asked: the CPU's callback puts Player 1 back, asleep,
        // and Player 1's own callback lets it go.
        let mut stage = held([0, 1], true);
        stage.ids[1] = 9;
        stage.tick([0, 1], None);
        assert_eq!((stage.holds.map(|hold| hold.is_some()), stage.x[0], stage.asleep[0], stage.back[0]), ([true, false], HOME[0], true, true));
        stage.tick([0, 1], None);
        assert_eq!((stage.holds.map(|hold| hold.is_some()), stage.x[0], stage.asleep[0], stage.speed[0]), ([false; 2], HOME[0], false, 0.0));
    }

    /// The start of a hold, the same rule. A fighter with a push in its module (the two were touching) that its
    /// partner's callback places and leaves awake is moved off its point by its own frame function, and the scrub
    /// lets it go as pushed aside.
    #[test]
    fn a_fighter_taken_from_its_partners_callback_is_not_pushed_off_its_point() {
        for (order, published) in [[0, 1], [1, 0]].into_iter().flat_map(|order| (0..3).map(move |published| (order, published))) {
            let case = format!("order {order:?}, published at {published}");
            let mut stage = Stage::new(true);
            // The two were touching on the tick before: the game's push is in both modules.
            stage.push = [-PUSH, PUSH];
            stage.tick(order, Some((published, Stage::group(true))));
            let mut read = published < 2;
            for tick in 0..8 {
                if read {
                    // Both are held from the first callback that reads the request, and neither is let go again.
                    assert_eq!((stage.holds.map(|hold| hold.is_some()), stage.refused), ([true; 2], [false; 2]), "{case}, tick {tick}");
                }
                read = true;
                stage.tick(order, None);
            }
            assert_eq!((stage.asleep, stage.speed), ([true; 2], [0.0; 2]), "{case}: both held asleep");
            assert_eq!(stage.x[1] - stage.x[0], 6.25, "{case}: at the recorded frame's spacing");
        }
    }

    #[test]
    fn the_partner_is_only_placed_asleep_from_the_other_roles_callback() {
        let hold = include_str!("../../runtime/features/scrub_hold.rs");
        let item = |head: &str| hold.split_once(head).map_or("", |(_, rest)| rest).split("\n}").next().unwrap_or("");
        let lines = |text: &'static str| text.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with("//")).collect::<Vec<_>>();
        let frame = lines(item("pub(super) unsafe fn frame(slot: usize, accessor: usize) {"));
        let at = |text: &str| frame.iter().position(|line| *line == text);
        // The release: one read of the request for the callback. The partner's hold must be on the body found; the
        // partner is put back, and its hold and its report stay for its own callback.
        let back = [
            "let ask = block.frame_ask(generation);",
            "if let (Ask::None, Some(hold)) = (ask, holds[slot ^ 1].as_mut()) {",
            "if let Some(partner) = other_body(slot).filter(|&body| hold.body() == natives::battle_object_id(body)) {",
            "put_back(block, slot ^ 1, hold, partner);",
            "}",
            "}",
            "let began = holds[slot].is_none();",
            "let seen = match point(block, &ask, &holds, slot, accessor) {",
        ];
        let first = at(back[0]).expect("one read of the request");
        assert_eq!(frame[first..first + back.len()], back);
        assert!(at("block.set_pose(slot ^ 1, partner.0, partner.1);").expect("the forgotten hold") < first);
        assert_eq!(hold.matches("block.frame_ask(").count(), 1);
        // The start: the partner's first hold step, right after the role's own, and it is left asleep.
        let start = [
            "};",
            "if takes_partner(began, holds[slot].is_some(), holds[slot ^ 1].is_some()) {",
            "if let Some(partner) = other_body(slot) {",
            "if let Some(spot) = point(block, &ask, &holds, slot ^ 1, partner) {",
            "let taken = hold(block, generation, slot ^ 1, &mut holds[slot ^ 1], partner, spot);",
            "block.set_seen(slot ^ 1, taken);",
            "if let Some(hold) = holds[slot ^ 1].as_mut() {",
            "leave_asleep(hold, partner);",
            "}",
            "}",
            "}",
            "}",
            "block.set_seen(slot, seen);",
            "block.ack(slot, generation);",
        ];
        let taken = at(start[1]).expect("the partner is taken in the role's callback");
        assert_eq!(frame[taken - 1..taken - 1 + start.len()], start);
        assert_eq!(lines(item("unsafe fn put_back(block: &Block, index: usize, hold: &mut CpuPreview, accessor: usize) {")), [
            "if block.pose(index).0 == POSE_SHOWN {",
            "placement::restore_posed(accessor, hold.origin(), if hold.origin().grounded { WAIT } else { FALL });",
            "block.set_pose(index, POSE_NONE, PackedPose::NONE);",
            "} else {",
            "placement::restore(accessor, hold.origin());",
            "}",
            "leave_asleep(hold, accessor);",
        ]);
        assert_eq!(lines(item("unsafe fn leave_asleep(hold: &mut CpuPreview, accessor: usize) {")),
            ["if let Some(asleep) = partner_sleep(hold.asleep) {", "state::set_sleep(accessor, asleep);", "hold.rest();", "}"]);
        // Only a role's own callback drops its hold and wakes its body: the frame's one release is its own, and the
        // file's only wakes are that release's two.
        assert_eq!(frame.iter().filter(|line| line.contains("release(")).count(), 1);
        assert_eq!((hold.matches("state::set_sleep(").count(), hold.matches("state::set_sleep(accessor, false);").count()), (3, 2));
        // The other role's body is found the way the placement picker finds the two; the picker's call is as it was.
        assert_eq!(lines(item("unsafe fn other_body(slot: usize) -> Option<usize> {")),
            ["if slot == 0 {", "super::recorder::cpu_body().map(|body| body.accessor)", "} else {", "natives::module_accessor(0)", "}"]);
        assert_eq!(include_str!("../../runtime/features/record_place.rs").matches("super::scrub_hold::frame(slot, accessor);").count(), 1);
        // Nothing is kept for a later frame, and no push switch is called.
        assert!(!hold.contains("jostle") && !hold.contains("\nstatic "));
    }
    }
}
