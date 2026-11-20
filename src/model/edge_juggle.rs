//! Shared attempt clock and scoring for Guard the Edge and Juggler (DESIGN v2).
use super::practice_games::{record_max, record_time, result_tick, Game};

const EDGE_TIMEOUT: u32 = 15 * 60;
const RESULT_FRAMES: u16 = 90;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drill {
    GuardEdge,
    Juggler,
}

impl Drill {
    pub const fn from_game(game: Game) -> Option<Self> {
        match game {
            Game::GuardEdge => Some(Self::GuardEdge),
            Game::Juggler => Some(Self::Juggler),
            _ => None,
        }
    }

    pub const fn game(self) -> Game {
        match self {
            Self::GuardEdge => Game::GuardEdge,
            Self::Juggler => Game::Juggler,
        }
    }

    pub const fn ready_frames(self) -> u16 {
        match self {
            Self::GuardEdge => 60,
            Self::Juggler => 45,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Secured,
    RecoveredAtLedge,
    RecoveredOnStage,
    Unresolved,
    Landed,
    GrabbedLedge,
    KnockedOut,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Observation {
    pub dead_or_rebirthing: bool,
    pub grounded: bool,
    pub cliff_catch: bool,
    pub hit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Place,
    Ready(u16),
    Live,
    Result(Outcome, u16),
    Done,
}

/// Runtime admits placement, then ticks only on eligible simulation frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Session {
    drill: Drill,
    limit: Option<u16>,
    pub phase: Phase,
    pub attempts: u16,
    pub secured: u16,
    pub streak: u16,
    pub best_streak: u16,
    pub frames: u32,
    pub hits: u16,
}

impl Session {
    pub const fn new(drill: Drill, limit: Option<u16>) -> Self {
        Self {
            drill,
            limit,
            phase: Phase::Place,
            attempts: 0,
            secured: 0,
            streak: 0,
            best_streak: 0,
            frames: 0,
            hits: 0,
        }
    }

    pub fn restart_drill(&mut self) {
        *self = Self::new(self.drill, self.limit);
    }

    pub fn discard_attempt(&mut self) {
        if matches!(self.phase, Phase::Ready(_) | Phase::Live) {
            self.phase = Phase::Place;
        }
    }

    pub fn placed(&mut self) {
        if self.phase == Phase::Place {
            self.frames = 0;
            self.hits = 0;
            self.phase = Phase::Ready(self.drill.ready_frames());
        }
    }

    /// The transition to Live is the release frame (time zero).
    pub fn advance_attempt(&mut self, observed: Observation) {
        match self.phase {
            Phase::Ready(mut left) => {
                self.phase = if result_tick(&mut left, true) {
                    Phase::Live
                } else {
                    Phase::Ready(left)
                };
            }
            Phase::Live => {
                self.frames = self.frames.saturating_add(1);
                self.hits = self.hits.saturating_add(u16::from(observed.hit));
                let outcome = match (self.drill, observed) {
                    (
                        Drill::GuardEdge,
                        Observation {
                            dead_or_rebirthing: true,
                            ..
                        },
                    ) => Outcome::Secured,
                    (
                        Drill::GuardEdge,
                        Observation {
                            cliff_catch: true, ..
                        },
                    ) => Outcome::RecoveredAtLedge,
                    (Drill::GuardEdge, Observation { grounded: true, .. }) => {
                        Outcome::RecoveredOnStage
                    }
                    (Drill::GuardEdge, _) if self.frames == EDGE_TIMEOUT => Outcome::Unresolved,
                    (
                        Drill::Juggler,
                        Observation {
                            dead_or_rebirthing: true,
                            ..
                        },
                    ) => Outcome::KnockedOut,
                    (
                        Drill::Juggler,
                        Observation {
                            cliff_catch: true, ..
                        },
                    ) => Outcome::GrabbedLedge,
                    (Drill::Juggler, Observation { grounded: true, .. }) => Outcome::Landed,
                    _ => return,
                };
                if outcome != Outcome::Unresolved {
                    self.attempts = self.attempts.saturating_add(1);
                    if outcome == Outcome::Secured {
                        self.secured = self.secured.saturating_add(1);
                        self.streak = self.streak.saturating_add(1);
                        self.best_streak = self.best_streak.max(self.streak);
                    } else {
                        self.streak = 0;
                    }
                }
                self.phase = Phase::Result(outcome, RESULT_FRAMES);
            }
            Phase::Result(outcome, mut left) => {
                self.phase = if !result_tick(&mut left, true) || observed.dead_or_rebirthing {
                    Phase::Result(outcome, left)
                } else if self.limit.is_some_and(|limit| self.attempts >= limit) {
                    Phase::Done
                } else {
                    Phase::Place
                };
            }
            Phase::Done | Phase::Place => {}
        }
    }

    pub fn record_result<K: Copy + Eq, const N: usize>(
        &self,
        bests: &mut [Option<(K, u32)>; N],
        key: K,
    ) -> Option<u32> {
        match self.phase {
            Phase::Result(Outcome::Secured, RESULT_FRAMES) => {
                Some(record_time(bests, key, self.frames))
            }
            Phase::Result(
                Outcome::Landed | Outcome::GrabbedLedge | Outcome::KnockedOut,
                RESULT_FRAMES,
            ) => Some(record_max(bests, key, self.frames)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn release(drill: Drill, limit: Option<u16>) -> Session {
        let mut session = Session::new(drill, limit);
        session.placed();
        for _ in 0..drill.ready_frames() {
            session.advance_attempt(Observation::default());
        }
        assert_eq!((session.phase, session.frames), (Phase::Live, 0));
        session
    }

    #[test]
    fn edge_scores_ko_and_waits_for_respawn() {
        let mut session = release(Drill::GuardEdge, None);
        let dead = Observation { dead_or_rebirthing: true, ..Observation::default() };
        session.advance_attempt(dead);
        assert_eq!((session.secured, session.attempts, session.streak), (1, 1, 1));
        let bests = &mut [None; 4];
        assert_eq!(session.record_result(bests, 42), Some(1));
        for _ in 0..=RESULT_FRAMES {
            session.advance_attempt(dead);
        }
        assert_eq!(session.phase, Phase::Result(Outcome::Secured, 0));
        session.advance_attempt(Observation::default());
        assert_eq!(session.phase, Phase::Place);
        session.restart_drill();
        assert_eq!(session, Session::new(Drill::GuardEdge, None));
        assert_eq!(super::super::practice_games::best_time(bests, 42), Some(1));
    }

    #[test]
    fn edge_timeout_does_not_spend_an_attempt() {
        let mut session = release(Drill::GuardEdge, Some(10));
        for _ in 0..EDGE_TIMEOUT {
            session.advance_attempt(Observation::default());
        }
        assert_eq!(session.phase, Phase::Result(Outcome::Unresolved, RESULT_FRAMES));
        assert_eq!((session.secured, session.attempts), (0, 0));
    }

    #[test]
    fn juggle_clock_starts_at_release() {
        let mut session = release(Drill::Juggler, Some(1));
        for _ in 0..25 {
            session.advance_attempt(Observation::default());
        }
        session.advance_attempt(Observation { hit: true, ..Observation::default() });
        session.advance_attempt(Observation { grounded: true, ..Observation::default() });
        assert_eq!(session.phase, Phase::Result(Outcome::Landed, RESULT_FRAMES));
        assert_eq!((session.frames, session.hits), (27, 1));
        let bests = &mut [Some((0, 15)), None];
        assert_eq!(session.record_result(bests, 0), Some(27));
        for _ in 0..RESULT_FRAMES {
            session.advance_attempt(Observation::default());
        }
        assert_eq!(session.phase, Phase::Done);
    }

    #[test]
    fn menu_interruption_discards_the_current_attempt_only() {
        let mut session = release(Drill::GuardEdge, None);
        session.advance_attempt(Observation { cliff_catch: true, ..Observation::default() });
        for _ in 0..RESULT_FRAMES {
            session.advance_attempt(Observation::default());
        }
        session.placed();
        session.discard_attempt();
        assert_eq!((session.phase, session.attempts), (Phase::Place, 1));
    }
    }
}
