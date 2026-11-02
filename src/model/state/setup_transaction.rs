//! Setup selection is worker-owned; validation repeats on the game thread before
//! either numbered record changes. No live position is an export source.
use super::{
    file::SLOTS,
    machine::{Situation, SituationIds},
    record::SavedState,
    setup::{FighterSetup, SetupError, SetupRecord, MII_KINDS},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub session: u64,
    pub slot: usize,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket(u64);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Pending,
    Game,
    Writing,
    Writer,
    Reply,
}
struct Transfer<T> {
    phase: Phase,
    ticket: Ticket,
    job: Option<Box<T>>,
}

/// One accepted transaction retains its allocation from worker submission to
/// worker retirement. Game and writer take ownership without allocating or
/// destroying it. Busy submission is rejected before acceptance.
pub struct Exchange<T> {
    state: hos_lock::Mutex<Transfer<T>>,
}
impl<T> Default for Exchange<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> Exchange<T> {
    pub const fn new() -> Self {
        Self {
            state: hos_lock::Mutex::new(Transfer {
                phase: Phase::Idle,
                ticket: Ticket(0),
                job: None,
            }),
        }
    }
    pub fn submit(&self, job: Box<T>) -> Result<Ticket, Box<T>> {
        let mut state = self.state.lock();
        if state.phase != Phase::Idle {
            return Err(job);
        }
        state.ticket.0 = state.ticket.0.wrapping_add(1).max(1);
        state.job = Some(job);
        state.phase = Phase::Pending;
        Ok(state.ticket)
    }
    fn take(&self, from: Phase, to: Phase) -> Option<(Ticket, Box<T>)> {
        let mut state = self.state.lock();
        if state.phase != from {
            return None;
        }
        let job = state.job.take()?;
        state.phase = to;
        Some((state.ticket, job))
    }
    fn finish(&self, ticket: Ticket, job: Box<T>, from: Phase, to: Phase) {
        let mut state = self.state.lock();
        assert!(state.ticket == ticket && state.phase == from && state.job.is_none());
        state.job = Some(job);
        state.phase = to;
    }
    pub fn take_game(&self) -> Option<(Ticket, Box<T>)> {
        self.take(Phase::Pending, Phase::Game)
    }
    pub fn finish_game(&self, ticket: Ticket, job: Box<T>, writer: bool) {
        self.finish(
            ticket,
            job,
            Phase::Game,
            if writer { Phase::Writing } else { Phase::Reply },
        );
    }
    pub fn take_writer(&self) -> Option<(Ticket, Box<T>)> {
        self.take(Phase::Writing, Phase::Writer)
    }
    pub fn finish_writer(&self, ticket: Ticket, job: Box<T>) {
        self.finish(ticket, job, Phase::Writer, Phase::Reply);
    }
    pub fn poll(&self, ticket: Ticket) -> Option<Box<T>> {
        let mut state = self.state.lock();
        if state.ticket != ticket || state.phase != Phase::Reply {
            return None;
        }
        state.phase = Phase::Idle;
        state.job.take()
    }
    /// Worker retirement before the game accepted the pending job. An in-flight
    /// commit or writer merge must finish and retain its reply instead.
    pub fn cancel_pending_if(
        &self,
        ticket: Ticket,
        stale: impl FnOnce(&T) -> bool,
    ) -> Option<Box<T>> {
        let mut state = self.state.lock();
        if state.ticket != ticket || state.phase != Phase::Pending {
            return None;
        }
        if !stale(state.job.as_deref()?) {
            return None;
        }
        state.phase = Phase::Idle;
        state.job.take()
    }
}

#[derive(Clone, Copy)]
pub struct Facts<I> {
    pub session: u64,
    pub owned: bool,
    pub busy: bool,
    pub enabled: bool,
    pub stage: i32,
    pub kinds: [i32; 2],
    pub miis: [Option<I>; 2],
    pub situations: SituationIds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Session,
    Ownership,
    Busy,
    Slot,
    Situation,
    Disabled,
    Stage,
    Fighter,
    Mii,
}

impl Refusal {
    pub fn message(self) -> &'static str {
        match self {
            Self::Session => "This setup selection belongs to an earlier session",
            Self::Ownership => "Wait for the practice fighters to finish loading",
            Self::Busy => "A save or restore is already in progress",
            Self::Slot => "The selected save slot is unavailable",
            Self::Situation => "This setup contains an unsupported saved situation",
            Self::Disabled => "Enable Save States before restoring a setup",
            Self::Stage => "Restore this setup on its saved stage; Y can bind it without moving",
            Self::Fighter => {
                "Restore this setup with its saved fighters; Y can bind it without moving"
            }
            Self::Mii => {
                "Restore this setup with its exact saved Miis; Y can bind it without moving"
            }
        }
    }
}

/// Constructed on the worker after decoding and class-validating exact Mii
/// identities. JSON stays in the worker's library record, never in this pair.
pub struct Prepared<I> {
    pair: [SavedState; 2],
    miis: [Option<I>; 2],
    stage: Option<i32>,
}

impl<I: Copy + Eq> Prepared<I> {
    pub fn new(record: &SetupRecord, miis: [Option<I>; 2]) -> Result<Self, SetupError> {
        record.validate()?;
        let mut pair = [
            record.player.to_saved(record.stage_id),
            record.cpu.to_saved(record.stage_id),
        ];
        for (index, saved) in pair.iter_mut().enumerate() {
            if MII_KINDS.contains(&saved.fighter_kind) && miis[index].is_none() {
                return Err(SetupError::Invalid(
                    "exact Mii identity could not be decoded",
                ));
            }
            saved.mii = None;
        }
        Ok(Self {
            pair,
            miis,
            stage: record.stage_id,
        })
    }

    /// Same check at selection and immediately before atomic two-role commit.
    /// Y is bind-only, so live stage/fighters and the enable flag do not restrict it.
    pub fn validate(&self, target: Target, facts: &Facts<I>, restore: bool) -> Result<(), Refusal> {
        if target.session != facts.session {
            return Err(Refusal::Session);
        }
        if !facts.owned {
            return Err(Refusal::Ownership);
        }
        if facts.busy {
            return Err(Refusal::Busy);
        }
        if target.slot >= SLOTS {
            return Err(Refusal::Slot);
        }
        if self
            .pair
            .iter()
            .any(|r| facts.situations.classify(r.situation_kind) == Situation::Other)
        {
            return Err(Refusal::Situation);
        }
        if restore {
            if !target.enabled || !facts.enabled {
                return Err(Refusal::Disabled);
            }
            if self.stage != Some(facts.stage) {
                return Err(Refusal::Stage);
            }
            if self.pair.each_ref().map(|r| r.fighter_kind) != facts.kinds {
                return Err(Refusal::Fighter);
            }
            for (index, record) in self.pair.iter().enumerate() {
                if MII_KINDS.contains(&record.fighter_kind) && self.miis[index] != facts.miis[index]
                {
                    return Err(Refusal::Mii);
                }
            }
        }
        Ok(())
    }

    pub fn pair(&self) -> [SavedState; 2] {
        self.pair.clone()
    }
    pub fn miis(&self) -> [Option<I>; 2] {
        self.miis
    }
}

/// Worker-only: the captured saved records, including their saved stage/Mii
/// values, are the complete source. Naming starts with "Setup" at the caller.
pub fn export(name: &str, pair: &[SavedState; 2]) -> Result<SetupRecord, SetupError> {
    if pair[0].saved_stage != pair[1].saved_stage {
        return Err(SetupError::Invalid(
            "the two saved fighters belong to different stages",
        ));
    }
    SetupRecord::new(
        name,
        "13.0.5",
        pair[0].saved_stage,
        FighterSetup::of_saved(&pair[0]),
        FighterSetup::of_saved(&pair[1]),
    )
}

#[cfg(test)]
mod tests {
    use super::super::{
        machine::{Machine, Role},
        record::Marker,
        settings::StateSettings,
    };
    use super::*;
    use crate::model::cpu::rng::Rng;
    fn pair() -> [SavedState; 2] {
        std::array::from_fn(|i| SavedState {
            fighter_kind: i as i32,
            x: 31.0 + i as f32,
            lr: 1.0,
            saved_stage: Some(17),
            situation_kind: 0,
            ..SavedState::never_saved()
        })
    }
    fn facts() -> Facts<u64> {
        Facts {
            session: 4,
            owned: true,
            busy: false,
            enabled: true,
            stage: 17,
            kinds: [0, 1],
            miis: [None; 2],
            situations: SituationIds {
                ground: 0,
                air: 2,
                cliff: 5,
            },
        }
    }
    fn target() -> Target {
        Target {
            session: 4,
            slot: 3,
            enabled: true,
        }
    }

    #[test]
    fn export_uses_saved_pair_and_rejects_mismatched_stage_halves() {
        let mut saved = pair();
        let record = export("Setup", &saved).unwrap();
        assert_eq!(
            (record.player.x, record.cpu.x, record.stage_id),
            (31.0, 32.0, Some(17))
        );
        saved[1].saved_stage = None;
        assert!(export("Setup", &saved).is_err());
        saved[0].saved_stage = None;
        assert_eq!(export("Setup", &saved).unwrap().stage_id, None);
    }

    #[test]
    fn selection_and_commit_revalidate_without_changing_frozen_target() {
        let prepared = Prepared::new(&export("Setup", &pair()).unwrap(), [None::<u64>; 2]).unwrap();
        let frozen = target();
        let mut current = facts();
        assert_eq!(prepared.validate(frozen, &current, true), Ok(()));
        current.enabled = false;
        assert_eq!(prepared.validate(frozen, &current, true), Err(Refusal::Disabled));
        assert_eq!(prepared.validate(frozen, &current, false), Ok(()));
        current.enabled = true;
        current.session += 1;
        assert_eq!(
            prepared.validate(frozen, &current, true),
            Err(Refusal::Session)
        );
        current = facts();
        current.kinds[1] = 9;
        assert_eq!(
            prepared.validate(frozen, &current, true),
            Err(Refusal::Fighter)
        );
        assert_eq!(prepared.validate(frozen, &current, false), Ok(()));
        current.busy = true;
        assert_eq!(
            prepared.validate(frozen, &current, false),
            Err(Refusal::Busy)
        );
        assert_eq!(frozen.slot, 3);
    }

    #[test]
    fn bind_only_allows_unknown_stage_and_disabled_saves_but_not_invalid_situation() {
        let mut record = export("Setup", &pair()).unwrap();
        record.stage_id = None;
        let mut frozen = target();
        frozen.enabled = false;
        let prepared = Prepared::new(&record, [None::<u64>; 2]).unwrap();
        assert_eq!(prepared.validate(frozen, &facts(), false), Ok(()));
        assert_eq!(
            prepared.validate(frozen, &facts(), true),
            Err(Refusal::Disabled)
        );
        frozen.enabled = true;
        assert_eq!(
            prepared.validate(frozen, &facts(), true),
            Err(Refusal::Stage)
        );
        record.cpu.situation_kind = 77;
        let invalid = Prepared::new(&record, [None::<u64>; 2]).unwrap();
        assert_eq!(
            invalid.validate(frozen, &facts(), false),
            Err(Refusal::Situation)
        );
    }

    #[test]
    fn exact_mii_rechecked_and_owned_json_never_crosses_game_pair() {
        let mut record = export("Setup", &pair()).unwrap();
        record.cpu.fighter_kind = 0x48;
        record.cpu.mii = Some(serde_json::json!({"identity": [1, 2, 3]}));
        assert!(Prepared::<u64>::new(&record, [None; 2]).is_err());
        let prepared = Prepared::new(&record, [None, Some(42)]).unwrap();
        let mut live = facts();
        live.kinds[1] = 0x48;
        live.miis[1] = Some(42);
        assert_eq!(prepared.validate(target(), &live, true), Ok(()));
        live.miis[1] = Some(43);
        assert_eq!(prepared.validate(target(), &live, true), Err(Refusal::Mii));
        assert_eq!(prepared.validate(target(), &live, false), Ok(()));
        assert!(prepared.pair().iter().all(|r| r.mii.is_none()));
        assert!(record.cpu.mii.is_some());
    }

    #[test]
    fn machine_commits_both_roles_only_when_idle_and_restores_the_frozen_slot() {
        let mut machine = Machine::default();
        let settings = StateSettings::default();
        let mut rng = Rng::new(17);
        let original = machine.file.clone();
        machine.set_marker(Role::Cpu, 1, Marker::Save);
        let pending = machine.file.clone();
        assert!(!machine.bind_setup(3, pair(), false, &settings, &mut rng));
        assert_eq!(
            machine.file, pending,
            "neither role may change during another save"
        );
        machine.cancel_all();
        assert!(!machine.bind_setup(SLOTS, pair(), false, &settings, &mut rng));
        assert_eq!(machine.file, original);
        assert!(machine.bind_setup(3, pair(), false, &settings, &mut rng));
        assert!(!machine.is_loading(), "Y binds without movement");
        assert_eq!(machine.record(Role::Player, 3).x, 31.0);
        assert_eq!(machine.record(Role::Cpu, 3).x, 32.0);
        assert_eq!(machine.record(Role::Player, 0), &original.player[0]);
        assert!(machine.bind_setup(3, pair(), true, &settings, &mut rng));
        assert_eq!(machine.current_slot(&settings), 3);
        assert_eq!(settings.slot, 0, "binding does not rewrite menu settings");
        assert_eq!(machine.marker(Role::Player, 3), Marker::KillPlayer);
        assert_eq!(machine.marker(Role::Cpu, 3), Marker::KillPlayer);
        let loading = machine.file.clone();
        assert!(!machine.bind_setup(2, pair(), false, &settings, &mut rng));
        assert_eq!(machine.file, loading);
    }

    #[test]
    fn accepted_exchange_survives_game_and_writer_until_matching_worker_poll() {
        let exchange = Exchange::new();
        let ticket = exchange.submit(Box::new(vec![17, 23])).unwrap();
        assert!(exchange.submit(Box::new(vec![99])).is_err());
        assert!(exchange.poll(ticket).is_none());
        let (id, mut job) = exchange.take_game().unwrap();
        let address = (&*job) as *const Vec<i32>;
        assert_eq!(id, ticket);
        job[0] = 42;
        assert!(exchange.take_game().is_none());
        exchange.finish_game(id, job, true);
        assert!(exchange.poll(ticket).is_none());
        let (id, job) = exchange.take_writer().unwrap();
        assert_eq!((&*job) as *const Vec<i32>, address);
        assert!(exchange.submit(Box::new(vec![100])).is_err());
        exchange.finish_writer(id, job);
        assert!(exchange.poll(Ticket(ticket.0 + 1)).is_none());
        assert_eq!(*exchange.poll(ticket).unwrap(), vec![42, 23]);
        assert!(exchange.poll(ticket).is_none());
        let next = exchange.submit(Box::new(vec![55])).unwrap();
        assert_ne!(next, ticket);
        let (id, job) = exchange.take_game().unwrap();
        exchange.finish_game(id, job, false);
        assert_eq!(*exchange.poll(next).unwrap(), vec![55]);
    }

    #[test]
    fn departure_can_retire_unstarted_job_but_cannot_discard_committed_binding() {
        let exchange = Exchange::new();
        let ticket = exchange.submit(Box::new(4u64)).unwrap();
        assert!(exchange
            .cancel_pending_if(ticket, |session| *session == 5)
            .is_none());
        assert_eq!(
            *exchange
                .cancel_pending_if(ticket, |session| *session == 4)
                .unwrap(),
            4
        );
        let ticket = exchange.submit(Box::new(6u64)).unwrap();
        let (id, job) = exchange.take_game().unwrap();
        assert!(exchange.cancel_pending_if(ticket, |_| true).is_none());
        exchange.finish_game(id, job, true);
        assert!(exchange.cancel_pending_if(ticket, |_| true).is_none());
        let (id, job) = exchange.take_writer().unwrap();
        exchange.finish_writer(id, job);
        assert_eq!(*exchange.poll(ticket).unwrap(), 6);
    }
}
