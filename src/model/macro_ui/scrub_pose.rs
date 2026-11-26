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
        let plain = all("natives::change_motion(accessor, if hold.origin().grounded { WAIT } else { FALL }, 0.0, 1.0, true);");
        let (restore, wake) = (all("placement::restore(accessor, hold.origin());"), all("state::set_sleep(accessor, false);"));
        assert_eq!((posed.len(), plain.len(), restore.len(), wake.len()), (1, 1, 2, 2));
        // The scrub's own body: the order run pose-probe-yoga-03 flew. Any other body: today's two lines, in today's order.
        assert!(posed[0] < plain[0] && plain[0] < restore[0] && restore[0] < wake[0] && wake[0] < wake[1] && wake[1] < restore[1]);
        assert!(release.contains("block.set_pose(index, POSE_NONE, PackedPose::NONE);"), "a body let go shows nothing of the scrub's");
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
        assert_eq!(swapped.lines().filter(|line| !line.trim_start().starts_with("//")).map(str::trim).collect::<Vec<_>>(),
            ["let partner = block.pose(slot ^ 1);", "block.forget();", "block.set_pose(slot ^ 1, partner.0, partner.1);"]);
        // The plain animation before the wake: the posed arm of the release reads that report.
        let release = hold.split_once("unsafe fn release(").map_or("", |(_, rest)| rest);
        let posed = release.find("if block.pose(index).0 == POSE_SHOWN {").expect("the release reads the role's report");
        let plain = release.find("natives::change_motion(accessor, if hold.origin().grounded { WAIT } else { FALL }, 0.0, 1.0, true);").expect("the plain animation");
        assert!(posed < plain && plain < release.find("state::set_sleep(accessor, false);").expect("the wake"));
    }
    }
}
