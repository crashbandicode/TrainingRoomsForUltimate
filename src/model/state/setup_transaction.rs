//! Setup selection is worker-owned; validation repeats on the game thread before
//! either numbered record changes. A snapshot reads the bound save, or (Save setup here, rc.14 SL4: this reverses
//! "no live position is an export source") both fighters where they stand; neither writes a numbered record.
use super::{
    file::SLOTS,
    machine::{Situation, SituationIds},
    record::SavedState,
    refused::SAVE_STATES_OFF,
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
    Off,
}

impl<I> Facts<I> {
    /// Every setup transaction: the same practice session, both fighters loaded (an exact Mii too), nothing being
    /// saved, loaded or placed, and a slot that exists.
    fn admit(&self, target: Target) -> Result<(), Refusal> {
        if target.session != self.session {
            return Err(Refusal::Session);
        }
        if !self.owned {
            return Err(Refusal::Ownership);
        }
        if self.busy {
            return Err(Refusal::Busy);
        }
        if target.slot >= SLOTS {
            return Err(Refusal::Slot);
        }
        Ok(())
    }

    /// Save setup here reads both fighters where they stand, so it is refused where a Save is not taken (Save States
    /// Off, as the menu shows it; something being placed) and while the fighters or an exact Mii still load.
    pub fn here(&self, target: Target) -> Result<(), Refusal> {
        self.admit(target)?;
        if !target.enabled {
            return Err(Refusal::Off);
        }
        Ok(())
    }
}

impl Refusal {
    pub fn message(self) -> &'static str {
        match self {
            // restriction: save-load-off (Save setup here gives Save's own notice)
            Self::Off => SAVE_STATES_OFF,
            Self::Session => "The practice session changed: open the Setup library again",
            Self::Ownership => "Wait for the practice fighters to finish loading",
            Self::Busy => "A save or restore is already in progress",
            Self::Slot => "The selected save slot is unavailable",
            Self::Situation => "This setup holds a fighter pose Training does not know",
            Self::Disabled => "Enable Save States before restoring a setup",
            Self::Stage => "This setup is for another stage. Make Load use it works anywhere",
            Self::Fighter => "This setup is for other fighters. Make Load use it works anywhere",
            Self::Mii => "This setup is for other Miis. Make Load use it works anywhere",
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
        facts.admit(target)?;
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
/// The error is the player's text: nothing saved, or anything else that stops
/// the pair from being one setup (the stages differ first).
pub fn export(name: &str, pair: &[SavedState; 2]) -> Result<SetupRecord, &'static str> {
    const SAME_STAGE: &str = "Both fighters must be saved on the same stage";
    if !pair[0].is_saved() {
        return Err("Nothing is saved here yet: Save first, or bind a save");
    }
    if pair[0].saved_stage != pair[1].saved_stage {
        return Err(SAME_STAGE);
    }
    let [player, cpu] = pair.each_ref().map(FighterSetup::of_saved);
    SetupRecord::new(name, "13.0.5", pair[0].saved_stage, player, cpu).map_err(|_| SAME_STAGE)
}

/// A snapshot's two records with their exact Miis attached, as the setup they make: one mapping for the bound save and
/// for where the fighters stand. Worker only (the Mii JSON is built here).
pub fn export_pair<M: serde::Serialize>(name: &str, mut pair: [SavedState; 2], miis: [Option<M>; 2]) -> Result<SetupRecord, &'static str> {
    for (saved, mii) in pair.iter_mut().zip(miis) {
        saved.attach_mii(mii).map_err(|_| "Saved Mii could not be serialized")?;
    }
    export(name, &pair)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::super::{
        held_item::HeldItem,
        machine::{FrameIn, Machine, Role, StatusNow},
        record::{Charge, Marker, Steve},
        refused::{Action, Refused},
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

    fn export_says_nothing_is_saved_or_that_the_stages_differ() {
        let empty = [SavedState::never_saved(), SavedState::never_saved()];
        assert_eq!(export("Setup", &empty).unwrap_err(), "Nothing is saved here yet: Save first, or bind a save");
        let mut mixed = pair();
        mixed[1].saved_stage = Some(99);
        assert_eq!(export("Setup", &mixed).unwrap_err(), "Both fighters must be saved on the same stage");
        assert!(export("Setup", &pair()).is_ok());
    }

    /// Save setup here takes both fighters where they stand, so it is refused where a Save is not taken: one table runs
    /// the Save chord through the machine and Save setup here through its facts.
    fn save_setup_here_refuses_where_save_does() {
        let save = FrameIn {
            entry: 0,
            nana: false,
            popo: false,
            kind: 0,
            dead: false,
            status: StatusNow::Other,
            prev_rebirth: false,
            standby: false,
            stage_id: 17,
            situation: facts().situations,
            save_chord: true,
            load_chord: false,
            kirby: false,
            pokemon: false,
            buffable: false,
        };
        for (case, enable, placing, here) in [
            ("nothing running", true, false, Ok(())),
            ("Save States Off", false, false, Err(Refusal::Off)),
            ("a setup, preview or Play warp being placed", true, true, Err(Refusal::Busy)),
        ] {
            let mut machine = Machine::default();
            let settings = StateSettings { enable, ..StateSettings::default() };
            if placing {
                assert!(machine.begin_temporary(pair()));
            }
            let live = Facts { busy: machine.setup_busy(), enabled: enable, ..facts() };
            let saved = machine.frame(&save, &settings, &mut Rng::new(3)).saved_notice.is_some();
            assert_eq!(live.here(Target { enabled: enable, ..target() }), here, "{case}");
            assert_eq!(saved, here.is_ok(), "{case}: Save and Save setup here agree");
        }
        assert_eq!(Some(Refusal::Off.message()), Refused::Off.notice(Action::Save), "Save's own words");
        // A load being placed: Save would cut it short; Save setup here waits for it, as every setup transaction does.
        let mut loading = Machine::default();
        assert!(loading.bind_setup(0, pair(), true, &StateSettings::default(), &mut Rng::new(3)));
        assert_eq!(Facts { busy: loading.setup_busy(), ..facts() }.here(target()), Err(Refusal::Busy));
        let loading_fighters = Facts { owned: false, ..facts() };
        assert_eq!(loading_fighters.here(target()), Err(Refusal::Ownership), "fighters or an exact Mii still loading");
        assert_eq!(facts().here(Target { session: 5, ..target() }), Err(Refusal::Session));
    }

    /// Save setup here's pair, read as Save reads it, keeps every field a setup holds, its exact Mii and the stage.
    fn a_live_pair_maps_to_every_setup_field_with_its_exact_mii() {
        let mut live = pair();
        live[0] = SavedState {
            fighter_kind: 0x48,
            y: 12.5,
            percent: 37.5,
            lr: -1.0,
            situation_kind: 2,
            charge: Charge { int_x: Some(3), float_y: Some(0.5), has_charge: Some(true), ..Charge::default() },
            steve: Some(Steve { mat_iron: 4, ..Steve::START }),
            held_item: Some(HeldItem::Item { choice: 29, variation: 6, lifetime: 120, lifetime_max: 600, fuse: -1, fuse_max: -1 }),
            ..live[0].clone()
        };
        live[1].situation_kind = 5;
        let mii = serde_json::json!({"identity": [7, 8, 9]});
        let record = export_pair("Here", live.clone(), [Some(mii.clone()), None]).unwrap();
        live[0].mii = Some(mii);
        assert_eq!((&record.player, &record.cpu), (&FighterSetup::of_saved(&live[0]), &FighterSetup::of_saved(&live[1])));
        assert_eq!((record.name.as_str(), record.stage_id), ("Here", Some(17)));
        assert!(export_pair("Here", live, [None::<u64>, None]).is_err(), "a Mii is never kept without its exact Mii");
    }

    fn refusals_say_which_part_differs_and_what_y_does() {
        for (refusal, differs) in [(Refusal::Stage, "stage"), (Refusal::Fighter, "fighters"), (Refusal::Mii, "Miis")] {
            let text = refusal.message();
            assert!(text.contains(differs) && text.contains("Make Load use it"), "{text}");
            assert!(!text.contains("bind"), "{text}");
        }
        assert!(Refusal::Session.message().contains("open the Setup library again"));
        assert!(!Refusal::Situation.message().contains("situation"));
    }

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

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("export_uses_saved_pair_and_rejects_mismatched_stage_halves", export_uses_saved_pair_and_rejects_mismatched_stage_halves as fn()),
            ("export_says_nothing_is_saved_or_that_the_stages_differ", export_says_nothing_is_saved_or_that_the_stages_differ as fn()),
            ("save_setup_here_refuses_where_save_does", save_setup_here_refuses_where_save_does as fn()),
            ("a_live_pair_maps_to_every_setup_field_with_its_exact_mii", a_live_pair_maps_to_every_setup_field_with_its_exact_mii as fn()),
            ("refusals_say_which_part_differs_and_what_y_does", refusals_say_which_part_differs_and_what_y_does as fn()),
            ("selection_and_commit_revalidate_without_changing_frozen_target", selection_and_commit_revalidate_without_changing_frozen_target as fn()),
            ("bind_only_allows_unknown_stage_and_disabled_saves_but_not_invalid_situation", bind_only_allows_unknown_stage_and_disabled_saves_but_not_invalid_situation as fn()),
            ("exact_mii_rechecked_and_owned_json_never_crosses_game_pair", exact_mii_rechecked_and_owned_json_never_crosses_game_pair as fn()),
            ("machine_commits_both_roles_only_when_idle_and_restores_the_frozen_slot", machine_commits_both_roles_only_when_idle_and_restores_the_frozen_slot as fn()),
            ("accepted_exchange_survives_game_and_writer_until_matching_worker_poll", accepted_exchange_survives_game_and_writer_until_matching_worker_poll as fn()),
            ("departure_can_retire_unstarted_job_but_cannot_discard_committed_binding", departure_can_retire_unstarted_job_but_cannot_discard_committed_binding as fn()),
        ]);
    }
    }
}
