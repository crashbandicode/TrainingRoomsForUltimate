//! Receipt-driven session owner. Native decision snapshots are supplied separately
//! from contact receipts; a missing or busy observation never becomes a grade.
use super::session::{Action, Decision, Direction, Length, Phase, Session};
use super::{AttemptState, Launch, Launcher};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Setup {
    pub launch: Launch,
    pub delay: u16,
}
#[derive(Clone, Copy, Debug)]
/// Timing counters belong to the native decision frame. `decision.tech` is
/// present only after the observer confirms the native tech status was accepted.
pub struct Outcome {
    pub attempt: u64,
    pub decision: Decision,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    None,
    Placed,
    Launched,
    Graded,
    Summary,
    NeedSetup { reroll: bool },
}
pub struct Controller {
    pub session: Session,
    pub direction: Option<Direction>,
    setup: Option<Setup>,
    attempt: Option<u64>,
    last_contact: Option<u64>,
    awaiting_outcome: bool,
    launch_pending: bool,
    fresh_outcome: bool,
}
impl Controller {
    pub const IDLE: Self = Self {
        session: Session::IDLE,
        direction: None,
        setup: None,
        attempt: None,
        last_contact: None,
        awaiting_outcome: false,
        launch_pending: false,
        fresh_outcome: false,
    };
    pub fn start(&mut self, length: Length) -> Event {
        *self = Self::IDLE;
        self.session.start(length);
        Event::NeedSetup { reroll: true }
    }
    /// Called only once per requested setup. Errors preserve Place for a retry;
    /// failed placements never count and do not reroll the accepted sample.
    pub fn place<L: Launcher>(
        &mut self,
        launcher: &mut L,
        setup: Setup,
    ) -> Result<Event, L::Error> {
        if self.session.phase != Phase::Place {
            return Ok(Event::None);
        }
        self.setup = Some(setup);
        launcher.place(setup.launch)?;
        self.awaiting_outcome = false;
        self.fresh_outcome = false;
        self.launch_pending = false;
        self.last_contact = None;
        self.direction = None;
        // A busy query after accepted placement must not cause a second place.
        self.attempt = None;
        self.session.placed(setup.delay);
        if let Ok(attempt) = launcher.attempt() {
            self.attempt = Some(attempt.id);
        }
        Ok(Event::Placed)
    }
    pub fn sampled(&self) -> Option<Setup> {
        self.setup
    }
    pub fn stop<L: Launcher>(&mut self, launcher: &mut L) -> Event {
        self.session.stop();
        launcher.restore();
        self.attempt = None;
        self.launch_pending = false;
        self.awaiting_outcome = false;
        self.fresh_outcome = false;
        Event::Summary
    }
    /// Decision snapshots are tagged by the adapter attempt id at the native
    /// status-decision seam. Stale or duplicate snapshots cannot count twice.
    pub fn outcome(&mut self, outcome: Outcome) -> Event {
        if self.session.phase != Phase::Flight
            || self.attempt != Some(outcome.attempt)
            || self.last_contact == Some(outcome.attempt)
        {
            return Event::None;
        }
        self.direction = outcome.decision.tech;
        self.session.contact(outcome.decision);
        self.last_contact = Some(outcome.attempt);
        self.fresh_outcome = true;
        self.awaiting_outcome = false;
        Event::Graded
    }
    /// One P1 simulation frame. Pausing freezes both observations and clocks.
    /// Adapter query errors (including Busy) are returned without manufacturing
    /// contact, abandonment, a miss or a duplicate launch.
    pub fn tick<L: Launcher>(
        &mut self,
        launcher: &mut L,
        paused: bool,
        new_guard_press: bool,
    ) -> Result<Event, L::Error> {
        if paused || matches!(self.session.phase, Phase::Off | Phase::Done) {
            return Ok(Event::None);
        }
        if self.fresh_outcome {
            self.fresh_outcome = false;
            self.session.elapsed = self.session.elapsed.saturating_add(1);
            return Ok(Event::None);
        }

        if matches!(self.session.phase, Phase::Delay(_) | Phase::Flight) {
            let seen = launcher.attempt()?;
            if self.attempt.is_none() && seen.state != AttemptState::Idle {
                self.attempt = Some(seen.id);
            }
            if self.attempt != Some(seen.id) {
                return Ok(Event::None);
            }
            if self.attempt == Some(seen.id) {
                if seen.state == AttemptState::Abandoned {
                    let action = self.session.abandon();
                    let event = self.action(action);
                    if event == Event::Summary {
                        launcher.restore();
                    }
                    return Ok(event);
                }
                if self.launch_pending
                    && matches!(
                        seen.state,
                        AttemptState::Stopped | AttemptState::Flying | AttemptState::Contact
                    )
                {
                    self.launch_pending = false;
                    return Ok(Event::Launched);
                }
                if seen.state == AttemptState::Contact && self.session.phase == Phase::Flight {
                    self.awaiting_outcome = true;
                }
            }
        }
        if self.launch_pending {
            launcher.launch()?;
            self.launch_pending = false;
            return Ok(Event::Launched);
        }
        let action = self.session.tick(false, new_guard_press);
        if action == Action::Launch {
            self.launch_pending = true;
            launcher.launch()?;
            self.launch_pending = false;
            return Ok(Event::Launched);
        }
        let event = self.action(action);
        if event == Event::Summary {
            launcher.restore();
        }
        Ok(event)
    }
    pub fn awaiting_outcome(&self) -> bool {
        self.awaiting_outcome
    }
    pub fn abandon(&mut self) -> Event {
        let action = self.session.abandon();
        self.action(action)
    }
    fn action(&mut self, action: Action) -> Event {
        match action {
            Action::Place { reroll } => Event::NeedSetup { reroll },
            Action::Summary => Event::Summary,
            _ => Event::None,
        }
    }
}
