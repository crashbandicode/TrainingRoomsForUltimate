//! Clip checkpoints are game-owned values. UI close only posts retirement;
//! native observations and state-machine changes run on a fighter callback.
use super::{Raw, Role, ACTIVE, CONSTS_READY, KILLING, LOADING, TEMPORARY};
use crate::engine::mii::lifetime::Configuration;
use crate::model::state::{
    ids::C,
    preview::{Checkpoint, Error},
};
use crate::runtime::{features::recorder, game::natives, lobby::training::practice};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Identity {
    pub(super) entries: [usize; 2],
    pub(super) kinds: [i32; 2],
    pub(super) stage: i32,
    pub(super) miis: [Option<Configuration>; 2],
}

static CHECKPOINT: hos_lock::Mutex<Checkpoint<Identity>> = hos_lock::Mutex::new(Checkpoint::new());
static RETIRE: AtomicBool = AtomicBool::new(false);
const NOT_READY: &str = "Wait for both fighters to finish loading before previewing";
const CHANGED: &str = "Preview fighters changed; close the editor, let them settle and reopen";

pub(super) fn clear() {
    CHECKPOINT.lock().clear();
    RETIRE.store(false, Ordering::Release);
}

/// May be called by a UI worker: no game lock or native operation.
pub(crate) fn end() {
    RETIRE.store(true, Ordering::Release);
}

/// The machine itself releases its temporary pair after both roles finish.
/// An open editor retains its checkpoint for crop edits/restart; close retires it.
pub(crate) fn finish(editor_open: bool) {
    if !editor_open {
        end();
    }
}

pub(super) fn service_retirement() {
    if RETIRE.swap(false, Ordering::AcqRel) {
        CHECKPOINT.lock().clear();
        if TEMPORARY.load(Ordering::Acquire) {
            super::cancel_pending();
        }
    }
}

pub(super) unsafe fn observe() -> Result<(Identity, [Raw; 2]), &'static str> {
    if !practice::enabled() || !practice::owned() {
        return Err(NOT_READY);
    }
    let context = practice::context().ok_or(NOT_READY)?;
    let entries = [
        practice::entry(0).ok_or(NOT_READY)?,
        practice::entry(1).ok_or(NOT_READY)?,
    ];
    let mut raws = [None, None];
    let mut miis = [None; 2];
    for (index, role) in [Role::Player, Role::Cpu].into_iter().enumerate() {
        let fighter = natives::entry_body(entries[index]);
        let accessor = natives::fighter_accessor(fighter);
        if fighter == 0 || accessor == 0 {
            return Err(NOT_READY);
        }
        let kind = ((fighter + super::FIGHTER_KIND_FIELD) as *const i32).read();
        if crate::model::state::setup::MII_KINDS.contains(&kind) {
            miis[index] =
                crate::runtime::roster::character_roster::current_mii_configuration(index)
                    .filter(|mii| mii.canonical().class as i32 == kind);
            if miis[index].is_none() {
                return Err("Wait for the exact Mii before previewing");
            }
        }
        raws[index] = Some(Raw {
            entry: if index == 0 { 0 } else { context.cpu_entry() },
            role,
            kind,
            accessor,
            fighter,
            status: natives::status_kind(accessor),
            prev_status: natives::prev_status_kind(accessor, 0),
            situation: natives::situation_kind(accessor),
        });
    }
    let [Some(player), Some(cpu)] = raws else {
        return Err(NOT_READY);
    };
    let identity = Identity {
        entries,
        kinds: [player.kind, cpu.kind],
        stage: natives::stage_id(),
        miis,
    };
    Ok((identity, [player, cpu]))
}

/// Called only by preview's CPU callback, after its practice ownership check.
pub(crate) unsafe fn request(start: bool) -> Result<(), &'static str> {
    service_retirement();
    if !ACTIVE.load(Ordering::Acquire) || !CONSTS_READY.load(Ordering::Acquire) {
        return Err(NOT_READY);
    }
    let (identity, raws) = observe()?;
    let first = !CHECKPOINT.lock().captured();
    if first {
        if !start {
            return Err("No temporary preview checkpoint is available");
        }
        let ready = super::with(|state| {
            raws.iter().all(|raw| {
                state.ids.is(C::SituationGround, raw.situation)
                    || state.ids.is(C::SituationAir, raw.situation)
            }) && state.ids.is(C::SituationGround, raws[1].situation)
                && state.ids.is(C::StatusWait, raws[1].status)
        })
        .unwrap_or(false);
        if !ready {
            return Err("Let the CPU settle to idle and leave the ledge before previewing");
        }
        let pair = [super::read_record(&raws[0]), super::read_record(&raws[1])];
        if observe()?.0 != identity {
            return Err(CHANGED);
        }
        CHECKPOINT.lock().capture(identity, pair).map_err(message)?;
    }
    let pair = CHECKPOINT.lock().restore(&identity).map_err(message)?;
    let accepted = super::with(|state| {
        let accepted = state.machine.begin_temporary(pair);
        if accepted {
            LOADING.store(true, Ordering::Release);
            KILLING.store(true, Ordering::Release);
            TEMPORARY.store(true, Ordering::Release);
        }
        accepted
    })
    .unwrap_or(false);
    if !accepted {
        if first {
            CHECKPOINT.lock().clear();
        }
        return Err("A save or restore is already in progress");
    }
    recorder::on_load_request();
    super::hud_load(0, true);
    super::hud_load(1, true);
    Ok(())
}

fn message(error: Error) -> &'static str {
    match error {
        Error::Missing => "No temporary preview checkpoint is available",
        Error::FightersChanged => CHANGED,
        Error::OwnedJson => "Preview checkpoint contains worker-owned data",
    }
}
