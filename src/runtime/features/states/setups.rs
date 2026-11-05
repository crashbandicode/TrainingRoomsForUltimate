//! Worker-to-game setup transactions. The only game-owned lock is taken by
//! service; UI reads a small facts copy and moves already-allocated jobs.
use crate::engine::mii::lifetime::Configuration;
pub(crate) use crate::model::state::setup_transaction::{Target, Ticket};
use crate::model::state::{
    file::SaveFile,
    machine::Role,
    record::SavedState,
    setup::{SetupRecord, MII_KINDS},
    setup_transaction::{self, Exchange, Facts, Prepared, Refusal},
};
use std::sync::atomic::{AtomicU64, Ordering};

static SESSION: AtomicU64 = AtomicU64::new(1);
// This lock only copies a value; no native call, allocation or STATE access is
// made while it is held. It supports callbacks on more than one game thread.
static FACTS: hos_lock::Mutex<Option<Facts<Configuration>>> = hos_lock::Mutex::new(None);
static JOBS: Exchange<Job> = Exchange::new();
const BUSY: &str = "A setup transaction is already in progress";

enum Action {
    Snapshot {
        pair: [SavedState; 2],
        miis: [Option<Configuration>; 2],
    },
    Bind {
        prepared: Prepared<Configuration>,
        restore: bool,
    },
}
struct Job {
    target: Target,
    action: Action,
    result: Result<(), &'static str>,
    sequence: u64,
}
pub(crate) enum Outcome {
    Snapshot(SetupRecord),
    Bound,
}

fn decode(record: &SavedState) -> Result<Option<Configuration>, &'static str> {
    if !MII_KINDS.contains(&record.fighter_kind) {
        return Ok(None);
    }
    let value = record
        .mii
        .clone()
        .ok_or("Exact Mii identity missing; save this slot again")?;
    let config: Configuration =
        serde_json::from_value(value).map_err(|_| "Saved Mii identity is invalid")?;
    if config.canonical().class as i32 != record.fighter_kind {
        return Err("Saved Mii identity has the wrong class");
    }
    Ok(Some(config))
}

pub(super) fn saved_identities(file: &SaveFile) -> [[Option<Configuration>; 5]; 2] {
    [&file.player, &file.cpu]
        .map(|records| std::array::from_fn(|i| decode(&records[i]).ok().flatten()))
}

pub(super) fn retire_session() {
    SESSION.fetch_add(1, Ordering::AcqRel);
    clear_facts();
}

pub(super) fn clear_facts() {
    *FACTS.lock() = None;
}

fn facts() -> Result<Facts<Configuration>, &'static str> {
    let facts = (*FACTS.lock()).ok_or(Refusal::Ownership.message())?;
    if facts.session != SESSION.load(Ordering::Acquire) {
        return Err(Refusal::Session.message());
    }
    Ok(facts)
}

pub(crate) fn open(slot: usize, enabled: bool) -> Result<Target, &'static str> {
    if slot >= 5 {
        return Err(Refusal::Slot.message());
    }
    let facts = facts()?;
    Ok(Target {
        session: facts.session,
        slot,
        enabled,
    })
}

fn prepare(record: &SetupRecord) -> Result<Prepared<Configuration>, &'static str> {
    let pair = [
        record.player.to_saved(record.stage_id),
        record.cpu.to_saved(record.stage_id),
    ];
    let miis = [decode(&pair[0])?, decode(&pair[1])?];
    Prepared::new(record, miis).map_err(|_| "Setup contains invalid saved fighter values")
}

pub(crate) fn validate_selection(
    target: Target,
    record: &SetupRecord,
    restore: bool,
) -> Result<(), &'static str> {
    prepare(record)?
        .validate(target, &facts()?, restore)
        .map_err(Refusal::message)
}

fn enqueue(target: Target, action: Action) -> Result<Ticket, &'static str> {
    let job = Box::new(Job {
        target,
        action,
        result: Ok(()),
        sequence: 0,
    });
    JOBS.submit(job).map_err(|_| BUSY)
}

pub(crate) fn request_snapshot(target: Target) -> Result<Ticket, &'static str> {
    if target.slot >= 5 {
        return Err(Refusal::Slot.message());
    }
    if target.session != facts()?.session {
        return Err(Refusal::Session.message());
    }
    enqueue(
        target,
        Action::Snapshot {
            pair: std::array::from_fn(|_| SavedState::never_saved()),
            miis: [None; 2],
        },
    )
}

pub(crate) fn request_bind(
    target: Target,
    record: &SetupRecord,
    restore: bool,
) -> Result<Ticket, &'static str> {
    let prepared = prepare(record)?;
    prepared
        .validate(target, &facts()?, restore)
        .map_err(Refusal::message)?;
    enqueue(target, Action::Bind { prepared, restore })
}

/// Keep polling accepted tickets on the worker even if a panel closes. The job
/// allocation and any serialization are retired here, never on a game hook.
pub(crate) fn poll(ticket: Ticket) -> Option<Result<Outcome, &'static str>> {
    if JOBS
        .cancel_pending_if(ticket, |job| {
            job.target.session != SESSION.load(Ordering::Acquire)
        })
        .is_some()
    {
        return Some(Err(Refusal::Session.message()));
    }
    let job = JOBS.poll(ticket)?;
    Some((|| {
        job.result?;
        if job.target.session != SESSION.load(Ordering::Acquire) {
            return Err(Refusal::Session.message());
        }
        match job.action {
            Action::Snapshot { mut pair, miis } => {
                for (saved, mii) in pair.iter_mut().zip(miis) {
                    saved
                        .attach_mii(mii)
                        .map_err(|_| "Saved Mii could not be serialized")?;
                }
                setup_transaction::export("Setup", &pair)
                    .map(Outcome::Snapshot)
                    .map_err(|_| {
                        "Save both fighters in this slot on the same stage before exporting"
                    })
            }
            Action::Bind { .. } => Ok(Outcome::Bound),
        }
    })())
}

/// P1 game callback, before the ordinary enable gate (Y also works when disabled).
pub(super) unsafe fn service() {
    let session = SESSION.load(Ordering::Acquire);
    let observed = super::preview::observe().ok().map(|v| v.0);
    let current = observed.and_then(|identity| {
        super::with(|state| Facts {
            session,
            owned: true,
            busy: state.machine.setup_busy(),
                enabled: state.settings.enable,
            stage: identity.stage,
            kinds: identity.kinds,
            miis: identity.miis,
            situations: state.ids.situations(),
        })
    });
    *FACTS.lock() = current;
    let Some((ticket, mut job)) = JOBS.take_game() else {
        return;
    };
    let mut writing = false;
    let mut loading = false;
    job.result = (|| {
        let mut current = current.ok_or(Refusal::Ownership.message())?;
        if job.target.session != session || session != SESSION.load(Ordering::Acquire) {
            return Err(Refusal::Session.message());
        }
        if job.target.slot >= 5 {
            return Err(Refusal::Slot.message());
        }
        // Native observations are complete; both records and typed saved-Mii
        // values are now read/committed under one game-owner lock.
        super::with(|state| {
            if job.target.session != SESSION.load(Ordering::Acquire) {
                return Err(Refusal::Session.message());
            }
            current.busy = state.machine.setup_busy();
            current.enabled = state.settings.enable;
            if current.busy {
                return Err(Refusal::Busy.message());
            }
            match &mut job.action {
                Action::Snapshot { pair, miis } => {
                    *pair = [
                        state.machine.record(Role::Player, job.target.slot).clone(),
                        state.machine.record(Role::Cpu, job.target.slot).clone(),
                    ];
                    *miis = [
                        state.saved_miis[0][job.target.slot],
                        state.saved_miis[1][job.target.slot],
                    ];
                }
                Action::Bind { prepared, restore } => {
                    prepared
                        .validate(job.target, &current, *restore)
                        .map_err(Refusal::message)?;
                    if !state.machine.bind_setup(
                        job.target.slot,
                        prepared.pair(),
                        *restore,
                        &state.settings,
                        &mut state.rng,
                    ) {
                        return Err(Refusal::Busy.message());
                    }
                    let miis = prepared.miis();
                    state.saved_miis[0][job.target.slot] = miis[0];
                    state.saved_miis[1][job.target.slot] = miis[1];
                    job.sequence = super::HANDOFF_SEQ.fetch_add(1, Ordering::AcqRel);
                    writing = true;
                    loading = *restore;
                    if loading {
                        super::LOADING.store(true, Ordering::Release);
                        super::KILLING.store(true, Ordering::Release);
                    }
                }
            }
            Ok(())
        })
        .unwrap_or(Err(Refusal::Ownership.message()))
    })();
    JOBS.finish_game(ticket, job, writing);
    if writing {
        super::DIRTY.store(true, Ordering::Release);
    }
    if loading {
        crate::runtime::features::recorder::on_load_request();
        super::hud_load(0, true);
        super::hud_load(1, true);
    }
    crate::runtime::ui::worker::wake();
}

pub(super) struct WriterBind {
    ticket: Ticket,
    job: Box<Job>,
}
impl WriterBind {
    pub(super) fn sequence(&self) -> u64 {
        self.job.sequence
    }
}
pub(super) fn take_writer_bind() -> Option<WriterBind> {
    JOBS.take_writer()
        .map(|(ticket, job)| WriterBind { ticket, job })
}

/// Merge in the same sequence order as ordinary captures. The full pair enters
/// the writer mirror together; existing persist retry keeps it dirty on failure.
pub(super) fn apply_writer_bind(file: &mut SaveFile, binding: WriterBind) {
    let Action::Bind { prepared, .. } = &binding.job.action else {
        unreachable!()
    };
    let mut pair = prepared.pair();
    for (saved, mii) in pair.iter_mut().zip(prepared.miis()) {
        saved
            .attach_mii(mii)
            .expect("validated Mii configuration serialization");
    }
    let [player, cpu] = pair;
    file.player[binding.job.target.slot] = player;
    file.cpu[binding.job.target.slot] = cpu;
    JOBS.finish_writer(binding.ticket, binding.job);
    crate::runtime::ui::worker::wake();
}
