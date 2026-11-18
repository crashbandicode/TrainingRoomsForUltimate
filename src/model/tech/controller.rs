//! Receipt-driven session owner. Native decision snapshots are supplied separately
//! from contact receipts; a missing or busy observation never becomes a grade.
use super::session::{
    self, Action, Decision, Direction, Grade, Length, NativeOutcome, Phase, Session,
};
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
    /// rc.15 practice menu: the menu opened while the session runs. The attempt in flight is cut (it never counts), the
    /// fighter is released and the session waits in Place with its count, streak, clock and length; the close resumes
    /// it with a new attempt, the same setup when the cut came before its grade. Once per visit; nothing before a start
    /// or after the summary.
    pub fn pause_for_menu<L: Launcher>(&mut self, launcher: &mut L) -> Event {
        let reroll = match self.session.phase {
            Phase::Delay(_) | Phase::Flight => false,
            Phase::Hold(_) => self.session.result.is_some_and(Grade::counted),
            Phase::Off | Phase::Place | Phase::Done => return Event::None,
        };
        launcher.restore();
        self.session.phase = Phase::Place;
        self.attempt = None;
        self.launch_pending = false;
        self.awaiting_outcome = false;
        self.fresh_outcome = false;
        Event::NeedSetup { reroll }
    }
    /// Decision snapshots are tagged by the adapter attempt id at the native
    /// status-decision seam. Stale or duplicate snapshots cannot count twice.
    pub fn outcome(&mut self, outcome: Outcome) -> Event {
        self.accept_outcome(
            outcome.attempt,
            outcome.decision.tech,
            session::grade(outcome.decision),
        )
    }
    /// Acceptance proof must describe this exact retained native request. Returning
    /// Event::None keeps the core delivery pending; Graded permits an exact-attempt ack.
    pub fn snapshot(
        &mut self,
        snapshot: super::super::tech_launch::DecisionSnapshot,
        accepted: Option<NativeOutcome>,
    ) -> Event {
        let Some(accepted) = accepted else {
            return Event::None;
        };
        let direction = match accepted {
            NativeOutcome::Tech(direction) => direction,
            NativeOutcome::Miss => None,
        };
        self.accept_outcome(
            snapshot.attempt,
            direction,
            session::map_snapshot(snapshot, accepted),
        )
    }
    /// As snapshot(), with a contact age frozen during pauses/query unavailability.
    /// Call before tick() in the same callback. Historical guard events that were
    /// not delivered cannot become a first late press or a no-press grade.
    pub fn snapshot_at(
        &mut self,
        snapshot: super::super::tech_launch::DecisionSnapshot,
        accepted: Option<NativeOutcome>,
        contact_age: u16,
    ) -> Event {
        let event = self.snapshot(snapshot, accepted);
        self.align_contact_age(event, contact_age);
        event
    }
    /// First decision-frame values win; only the core's one-time confirmation
    /// may fill entered_status. A duplicate cannot reset the contact clock.
    pub fn retain_snapshot(
        &self,
        pending: &mut Option<(super::super::tech_launch::DecisionSnapshot, u16)>,
        snapshot: super::super::tech_launch::DecisionSnapshot,
    ) -> bool {
        if !self.accepts_snapshot(snapshot.attempt) {
            return false;
        }
        if let Some((saved, _)) = pending
            .as_mut()
            .filter(|(s, _)| s.attempt == snapshot.attempt)
        {
            if saved.entered_status.is_none() {
                saved.entered_status = snapshot.entered_status;
            }
        } else {
            *pending = Some((snapshot, 0));
        }
        true
    }
    /// Shared by the retained query and confirmed recorder delivery. Unknown
    /// native entry expires as unavailable at the existing hold boundary.
    pub fn retained_snapshot(
        &mut self,
        snapshot: super::super::tech_launch::DecisionSnapshot,
        ids: [i32; 11],
        contact_age: u16,
    ) -> Event {
        if let Some(accepted) = session::accepted_status(snapshot, ids) {
            self.snapshot_at(snapshot, Some(accepted), contact_age)
        } else if contact_age >= session::HOLD {
            self.snapshot_unavailable(snapshot, contact_age)
        } else {
            Event::None
        }
    }
    /// Retire an exact retained contact when accepted status cannot be proven.
    /// This is explicitly ungraded, never a guessed miss or no-contact result.
    /// Like snapshot(), Graded permits acknowledgement; stale delivery does not.
    pub fn snapshot_unavailable(
        &mut self,
        snapshot: super::super::tech_launch::DecisionSnapshot,
        contact_age: u16,
    ) -> Event {
        let event = self.accept_outcome(snapshot.attempt, None, Some(Grade::Unavailable));
        self.align_contact_age(event, contact_age);
        event
    }
    fn align_contact_age(&mut self, event: Event, age: u16) {
        if event == Event::Graded && age > 0 {
            self.session.contact_age(age);
            self.fresh_outcome = false;
        }
    }
    /// Shared runtime/controller admission gate for retryable native delivery.
    pub fn accepts_snapshot(&self, attempt: u64) -> bool {
        self.session.phase == Phase::Flight
            && self.attempt == Some(attempt)
            && self.last_contact != Some(attempt)
    }
    fn accept_outcome(
        &mut self,
        attempt: u64,
        direction: Option<Direction>,
        grade: Option<Grade>,
    ) -> Event {
        if !self.accepts_snapshot(attempt) {
            return Event::None;
        }
        self.direction = direction;
        self.session.contact_grade(grade);
        self.last_contact = Some(attempt);
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
        new_guard_press: Option<bool>,
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
