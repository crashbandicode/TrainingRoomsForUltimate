//! Once-per-results operation policy. Native code supplies observed states and
//! a results-lifetime generation; pointers or sent inputs are not completion.
use super::control::{Decision, Request};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Waiting,
    IntroReady,
    Cards,
    SaveConfirm,
    Saving,
    SaveSuccess,
    SavedAndClosed,
    SaveFailed,
    Rematch,
    Accepted(Decision),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    TimedOut,
    SceneChanged,
    SaveFailed,
    SaveSkipped,
    WrongDecision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    AdvanceIntro,
    RequestSave,
    ConfirmSave,
    CloseSave,
    AdvanceResults,
    Choose(Decision),
    Complete,
    Failed(Failure),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Intro,
    Cards,
    Confirm,
    Saving,
    Close,
    Rematch,
    Acknowledge,
    End,
}

pub struct Flow {
    request: Request,
    epoch: Option<u64>,
    deadline_ms: u64,
    stage: Stage,
}

impl Flow {
    pub fn new(request: Request, now_ms: u64) -> Self {
        Self {
            request,
            epoch: None,
            deadline_ms: now_ms.saturating_add(600_000),
            stage: Stage::Intro,
        }
    }

    fn fail(&mut self, why: Failure) -> Effect {
        self.stage = Stage::End;
        Effect::Failed(why)
    }

    fn saved(&mut self) -> Effect {
        if self.request.rematch == Decision::Manual {
            self.stage = Stage::End;
            Effect::Complete
        } else {
            self.stage = Stage::Rematch;
            Effect::AdvanceResults
        }
    }

    /// `epoch` spans the results and its rematch prompt; the adapter increments
    /// it on a new results sequence, even if its native address was reused.
    /// The adapter must recheck Controls::owns immediately before each effect.
    pub fn observe(&mut self, now_ms: u64, epoch: Option<u64>, screen: Screen) -> Effect {
        if self.stage == Stage::End {
            return Effect::None;
        }
        if now_ms >= self.deadline_ms {
            return self.fail(Failure::TimedOut);
        }
        if let Some(bound) = self.epoch {
            if epoch != Some(bound) {
                return self.fail(Failure::SceneChanged);
            }
        } else if let Some(epoch) = epoch {
            self.epoch = Some(epoch);
            // This is an active native flow now, not the wait for a match.
            self.deadline_ms = now_ms.saturating_add(120_000);
        } else {
            return Effect::None;
        }
        if screen == Screen::SaveFailed {
            return self.fail(Failure::SaveFailed);
        }
        match (self.stage, screen) {
            (Stage::Intro, Screen::IntroReady) => {
                if !self.request.save_replay && self.request.rematch == Decision::Manual {
                    self.stage = Stage::End;
                    Effect::Complete
                } else {
                    self.stage = Stage::Cards;
                    Effect::AdvanceIntro
                }
            }
            (Stage::Intro | Stage::Cards, Screen::Cards) if self.request.save_replay => {
                self.stage = Stage::Confirm;
                Effect::RequestSave
            }
            (Stage::Intro | Stage::Cards, Screen::Cards) => self.saved(),
            (Stage::Confirm, Screen::SaveConfirm) => {
                self.stage = Stage::Saving;
                Effect::ConfirmSave
            }
            (Stage::Saving, Screen::SaveSuccess) => {
                self.stage = Stage::Close;
                Effect::CloseSave
            }
            (Stage::Close, Screen::SavedAndClosed) => self.saved(),
            (Stage::Intro | Stage::Cards | Stage::Rematch, Screen::Rematch)
                if !self.request.save_replay || self.stage == Stage::Rematch =>
            {
                // Native results can advance on their own. A no-save request
                // need not observe the cards, but a requested save must finish.
                if self.request.rematch == Decision::Manual {
                    self.stage = Stage::End;
                    Effect::Complete
                } else {
                    self.stage = Stage::Acknowledge;
                    Effect::Choose(self.request.rematch)
                }
            }
            (Stage::Acknowledge, Screen::Accepted(choice)) => {
                if choice != self.request.rematch {
                    return self.fail(Failure::WrongDecision);
                }
                self.stage = Stage::End;
                Effect::Complete
            }
            (
                Stage::Intro | Stage::Cards | Stage::Confirm | Stage::Saving | Stage::Close,
                Screen::Rematch,
            ) if self.request.save_replay => self.fail(Failure::SaveSkipped),
            _ => Effect::None,
        }
    }
}

