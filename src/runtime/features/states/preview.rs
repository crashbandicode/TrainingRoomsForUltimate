//! Clip checkpoints are game-owned values, observed and restored on a fighter callback.
use super::{Raw, Role, ACTIVE, CONSTS_READY, LOADING};
use crate::engine::rr_host::HomeWarp;
use crate::engine::mii::lifetime::Configuration;
use crate::model::state::{ids::C, preview::{Checkpoint, Error}};
use crate::runtime::features::recorder;
use crate::runtime::{game::natives, lobby::training::practice};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Identity {
    pub(super) entries: [usize; 2],
    pub(super) kinds: [i32; 2],
    pub(super) stage: i32,
    pub(super) miis: [Option<Configuration>; 2],
}

static CHECKPOINT: hos_lock::Mutex<Checkpoint<Identity>> = hos_lock::Mutex::new(Checkpoint::new());
static ASK: AtomicU8 = AtomicU8::new(0);
const NOT_READY: &str = "Wait for both fighters to finish loading before previewing";
const CHANGED: &str = "Preview fighters changed; close the editor, let them settle and reopen";

pub(super) fn clear() {
    ASK.store(0, Ordering::Release);
    CHECKPOINT.lock().clear();
}

pub(crate) fn request_home(home: HomeWarp) {
    let ask = match home {
        HomeWarp::Start { air: false, player: false } => 1,
        HomeWarp::Start { air: true, player: false } => 2,
        HomeWarp::Start { air: false, player: true } => 4,
        HomeWarp::Start { air: true, player: true } => 5,
        HomeWarp::Return => 3,
    };
    ASK.store(ask, Ordering::Release);
}

pub(super) unsafe fn take_home() -> bool {
    if LOADING.load(Ordering::Acquire) {
        return false;
    }
    let ask = ASK.swap(0, Ordering::AcqRel);
    if ask == 0 {
        return false;
    }
    let result = if ask == 3 {
        let result = request(false, false, false);
        CHECKPOINT.lock().clear();
        result
    } else {
        request(true, ask == 2 || ask == 5, ask >= 4)
    };
    match result {
        Ok(()) => true,
        Err(text) => {
            if ask == 3 {
                crate::engine::rr_host::warp_landed();
            } else {
                crate::engine::rr_host::preview_refused(text);
            }
            false
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
        // The running body, the one the Save chord records: after a live swap the entry's first body is not the fighter on
        // screen, and a setup of the fighters on screen was refused (CODE-FINDINGS #25).
        let fighter = natives::entry_active_body(entries[index]);
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

pub(super) unsafe fn request(start: bool, air: bool, player: bool) -> Result<(), &'static str> {
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
    let mut pair = CHECKPOINT.lock().restore(&identity).map_err(message)?;
    if air {
        let index = crate::model::state::preview::playing_index(player);
        super::with(|state| {
            if state.ids.is(C::SituationGround, pair[index].situation_kind) {
                pair[index].situation_kind = state.ids.get(C::SituationAir);
            }
        });
    }
    let accepted = super::begin_temporary(pair);
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
