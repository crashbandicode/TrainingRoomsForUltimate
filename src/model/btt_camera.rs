//! Break the Targets' camera (rc.14, `artifacts/rc14-eggsercise/SPEC.md` §V2.5 `camera`): in Placing, Ready and Running the
//! stage is shown by the game's own Training Fixed camera (mode 4), and the game's own camera comes back when the mode ends.
//! This is the pure half: what to ask of the game each pass (`Camera::step`) and the check on the manager's pointer
//! (`pointer_ok`). The runtime half is `runtime::features::btt_camera`.

/// The camera mode that is Training's Fixed camera (`cpu::tech::CAMERA_MODE_FIXED`).
pub const FIXED: i32 = super::cpu::tech::CAMERA_MODE_FIXED;
/// Frames between two asks while the game keeps taking the camera back (its own Training Camera option, a reset).
pub const RETRY: u32 = 10;

/// A game object's address as the plugin may dereference it: aligned, and not in the first pages.
pub const fn pointer_ok(p: usize) -> bool {
    p >= 0x1_0000 && p & 7 == 0
}

/// The camera's state: the mode the game was in when this mode first took the camera (`saved`, while it holds it) and the
/// frames left before it may ask again.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    saved: Option<i32>,
    cool: u32,
}

impl Camera {
    pub const IDLE: Camera = Camera { saved: None, cool: 0 };

    /// The mode took the camera from the game and has not given it back: the Hide Tech camera hooks stand down.
    pub fn holding(&self) -> bool {
        self.saved.is_some()
    }

    /// One pass of Player 1: `want` the Fixed camera, `mode` the game's now (`None`: no camera manager, the scene is over). It
    /// asks (`Some(mode)`) for the Fixed camera at most once in `RETRY` frames while the game is in another mode, and gives the camera back
    /// at once, to the mode first seen, when it is not wanted. A mode the game was already in (the Fixed camera) is not held.
    pub fn step(&mut self, want: bool, mode: Option<i32>) -> Option<i32> {
        self.cool = self.cool.saturating_sub(1);
        let Some(now) = mode else {
            self.saved = None;
            return None;
        };
        match (want, self.saved) {
            (true, _) if now != FIXED && self.cool == 0 => {
                self.saved.get_or_insert(now);
                self.cool = RETRY;
                Some(FIXED)
            }
            (false, Some(back)) => {
                self.saved = None;
                (now == FIXED).then_some(back)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "btt_camera_tests.rs"]
mod tests;
