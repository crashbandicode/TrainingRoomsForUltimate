//! Diag builds only: the scrub's pose line (`model::macro_ui::scrub_pose::PoseLine`), written on the role's own
//! fighter frame when what it says changes. A game run's exact-frame rows read it: the clip frame, the animation
//! asked for and the one read back are those of one fighter frame. The editor publishes the frame a moment before
//! its request, so a line written between the two carries the new frame with the old request; the last line of a
//! settled frame is the one to read.
use std::sync::atomic::{AtomicU64, Ordering};

use crate::engine::macro_ui::scrub_pose::PoseLine;
use crate::engine::macros::PackedPose;

/// Each role's last line, as its key. All zero: no line yet.
static LAST: [[AtomicU64; 4]; 2] = [const { [const { AtomicU64::new(0) }; 4] }; 2];

/// `frame`: the clip frame the request is for. `shown`: the animation and frame read back from the game.
pub(super) fn note(role: usize, frame: Option<u32>, state: u32, asked: PackedPose, shown: (u64, f32)) {
    let line = PoseLine { role, frame, state, asked, shown };
    let Some(last) = LAST.get(role) else { return };
    let mut new = false;
    for (cell, word) in last.iter().zip(line.key()) {
        new |= cell.swap(word, Ordering::Relaxed) != word;
    }
    if new {
        crate::runtime::diag_log::enqueue_diagnostic(format_args!("{line}"));
    }
}
