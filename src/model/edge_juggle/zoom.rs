//! Simulation-frame rules for the Hero Zoom landing-read drill.

pub const COMMAND: u8 = 15;
pub const COMMAND_COUNT: usize = 4;

/// Keep the cursor in its native slot, preserving the other rolls when Zoom is already present.
pub fn force_zoom(mut commands: [u8; COMMAND_COUNT], selected: usize) -> Option<[u8; COMMAND_COUNT]> {
    let displaced = *commands.get(selected)?;
    if let Some(old) = commands.iter().position(|&command| command == COMMAND) {
        commands[old] = displaced;
    }
    commands[selected] = COMMAND;
    Some(commands)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadState {
    Busy,
    /// Native down special can start on this frame.
    CanOpen,
    /// Native Command Selection is active. The selected list slot has been read back.
    Menu {
        open: bool,
        zoom_selected: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadIntent {
    Neutral,
    Open,
    Confirm,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Flight {
    #[default]
    Other,
    Ascent,
    /// Hero has met the ceiling and has yet to fall.
    Bonk,
    Descent,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Observation {
    pub flight: Flight,
    pub grounded: bool,
    /// A newly observed hit or grab by P1 on Hero.
    pub contact: bool,
    /// An independent terminal event such as a KO or the drill clock expiring.
    pub ended: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Win,
    Miss,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Waiting,
    Rising,
    Descending,
    Touchdown(u32),
    Scored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attempt {
    phase: Phase,
}

impl Default for Attempt {
    fn default() -> Self {
        Self::new()
    }
}

impl Attempt {
    pub const fn new() -> Self {
        Self {
            phase: Phase::Waiting,
        }
    }

    /// Input requests leave the attempt unchanged until native flight confirms the cast.
    pub fn pad(&self, state: PadState) -> PadIntent {
        if self.phase != Phase::Waiting {
            return PadIntent::Neutral;
        }
        match state {
            PadState::CanOpen => PadIntent::Open,
            PadState::Menu {
                open: true,
                zoom_selected: true,
            } => PadIntent::Confirm,
            _ => PadIntent::Neutral,
        }
    }

    /// Score once, on descent contact or the inclusive window from the first touchdown.
    /// Frames come from the drill's simulation clock, which advances only while the attempt is live.
    pub fn step(&mut self, frame: u32, read: Observation, tolerance: u8) -> Option<Verdict> {
        if self.phase == Phase::Scored {
            return None;
        }
        self.phase = match (self.phase, read.flight) {
            (Phase::Waiting, Flight::Ascent | Flight::Bonk) => Phase::Rising,
            (Phase::Waiting | Phase::Rising, Flight::Descent) => Phase::Descending,
            (phase, _) => phase,
        };
        let verdict = match self.phase {
            Phase::Descending if read.contact => Some(Verdict::Win),
            Phase::Descending if read.grounded => {
                self.phase = Phase::Touchdown(frame);
                None
            }
            Phase::Touchdown(landed) if frame.wrapping_sub(landed) > u32::from(tolerance) => {
                Some(Verdict::Miss)
            }
            Phase::Touchdown(_) if read.contact => Some(Verdict::Win),
            Phase::Waiting | Phase::Rising if read.grounded => Some(Verdict::Miss),
            _ => None,
        }
        .or(read.ended.then_some(Verdict::Miss));
        if verdict.is_some() {
            self.phase = Phase::Scored;
        }
        verdict
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn observe(flight: Flight) -> Observation {
        Observation {
            flight,
            ..Observation::default()
        }
    }

    #[test]
    fn zoom_pad_waits_for_forced_menu() {
        let mut attempt = Attempt::new();
        assert_eq!(attempt.pad(PadState::Busy), PadIntent::Neutral);
        assert_eq!(attempt.pad(PadState::CanOpen), PadIntent::Open);

        for (open, zoom_selected, expected) in [
            (false, false, PadIntent::Neutral),
            (false, true, PadIntent::Neutral),
            (true, false, PadIntent::Neutral),
            (true, true, PadIntent::Confirm),
        ] {
            assert_eq!(
                attempt.pad(PadState::Menu {
                    open,
                    zoom_selected
                }),
                expected
            );
        }

        // An input is only a request. If the menu is interrupted, retry when eligible.
        assert_eq!(attempt.pad(PadState::Busy), PadIntent::Neutral);
        assert_eq!(attempt.pad(PadState::CanOpen), PadIntent::Open);
        assert_eq!(attempt.step(10, observe(Flight::Ascent), 10), None);
        // The confirmed cast keeps control for the rest of this attempt.
        assert_eq!(attempt.pad(PadState::CanOpen), PadIntent::Neutral);
        assert_eq!(
            attempt.pad(PadState::Menu {
                open: true,
                zoom_selected: true
            }),
            PadIntent::Neutral
        );
    }

    #[test]
    fn zoom_is_forced_into_the_native_cursor_slot() {
        assert_eq!(force_zoom([2, 8, 4, 11], 2), Some([2, 8, COMMAND, 11]));
        assert_eq!(force_zoom([2, 8, 4, COMMAND], 1), Some([2, COMMAND, 4, 8]));
        assert_eq!(force_zoom([2, 8, 4, COMMAND], 3), Some([2, 8, 4, COMMAND]));
        assert_eq!(force_zoom([2, 8, 4, 11], 4), None);
    }

    #[test]
    fn zoom_scores_descent_and_touchdown_window() {
        let hit = Observation {
            contact: true,
            ..Observation::default()
        };
        let mut attempt = Attempt::new();
        assert_eq!(
            attempt.step(
                1,
                Observation {
                    flight: Flight::Ascent,
                    ..hit
                },
                10
            ),
            None
        );
        assert_eq!(
            attempt.step(
                10,
                Observation {
                    flight: Flight::Descent,
                    ..hit
                },
                10
            ),
            Some(Verdict::Win)
        );
        assert_eq!(attempt.step(11, hit, 10), None);

        // A hit may replace the falling status before the native observation.
        let mut attempt = Attempt::new();
        assert_eq!(attempt.step(17, observe(Flight::Descent), 10), None);
        assert_eq!(attempt.step(18, hit, 10), Some(Verdict::Win));

        for (tolerance, offset, verdict) in [
            (10, 0, Verdict::Win),
            (10, 10, Verdict::Win),
            (10, 11, Verdict::Miss),
            (1, 1, Verdict::Win),
            (1, 2, Verdict::Miss),
            (30, 30, Verdict::Win),
            (30, 31, Verdict::Miss),
        ] {
            let mut attempt = Attempt::new();
            let touchdown = 20;
            assert_eq!(
                attempt.step(touchdown - 1, observe(Flight::Descent), tolerance),
                None
            );
            let ground = Observation {
                grounded: true,
                contact: offset == 0,
                ..Observation::default()
            };
            let at_ground = attempt.step(touchdown, ground, tolerance);
            if offset == 0 {
                assert_eq!(at_ground, Some(verdict));
            } else {
                assert_eq!(at_ground, None);
                // Reobserving the same simulation frame does not spend a frame.
                assert_eq!(attempt.step(touchdown, ground, tolerance), None);
                assert_eq!(
                    attempt.step(touchdown + offset, hit, tolerance),
                    Some(verdict)
                );
            }
            assert_eq!(attempt.step(touchdown + offset + 1, hit, tolerance), None);
        }
    }

    #[test]
    fn zoom_bonk_enters_descent_after_the_ceiling() {
        let mut attempt = Attempt::new();
        assert_eq!(attempt.step(2, observe(Flight::Ascent), 10), None);
        let bonk = Observation {
            flight: Flight::Bonk,
            contact: true,
            ..Observation::default()
        };
        assert_eq!(attempt.step(3, bonk, 10), None);
        assert_eq!(attempt.pad(PadState::CanOpen), PadIntent::Neutral);
        assert_eq!(attempt.step(4, bonk, 10), None);
        let fall = Observation {
            flight: Flight::Descent,
            ..bonk
        };
        assert_eq!(attempt.step(5, fall, 10), Some(Verdict::Win));
    }

    #[test]
    fn zoom_a_failed_attempt_scores_one_miss() {
        let ground = Observation {
            grounded: true,
            ..Observation::default()
        };
        let ended = Observation {
            ended: true,
            ..Observation::default()
        };
        let mut attempt = Attempt::new();
        assert_eq!(attempt.step(60, ended, 10), Some(Verdict::Miss));
        assert_eq!(attempt.step(61, ended, 10), None);
        assert_eq!(attempt.pad(PadState::CanOpen), PadIntent::Neutral);

        let mut attempt = Attempt::new();
        assert_eq!(attempt.step(60, ground, 10), Some(Verdict::Miss));

        let mut attempt = Attempt::new();
        assert_eq!(attempt.step(56, observe(Flight::Descent), 10), None);
        assert_eq!(attempt.step(57, ground, 10), None);
        assert_eq!(attempt.step(67, ground, 10), None);
        assert_eq!(attempt.step(68, ground, 10), Some(Verdict::Miss));
    }
    }
}
