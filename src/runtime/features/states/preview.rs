//! Clip checkpoints are game-owned values, observed and restored on a fighter callback.
use super::{Raw, Role, ACTIVE, CONSTS_READY, LOADING};
use crate::engine::rr_host::HomeWarp;
use crate::engine::mii::lifetime::Configuration;
use crate::model::state::{ids::C, preview::{settle, take_home_ask, Body, Checkpoint, Error, Settle}};
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
    // A start that follows the scrub hold's release stays asked, as it does while a load runs, until the released CPU
    // has settled or will not (tr-primary-2 2026-11-26 12:54:05Z). The wait is on the CPU's state, never on a clock.
    let ask = take_home_ask(&ASK, |waiting| waiting != 0 && waiting != 3 && settling() == Some(Settle::Wait));
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

/// The released CPU's state for a start the editor asked after its hold let go; none when no such start waits. A
/// verdict other than `Wait` ends the wait: the checkpoint's own gate then takes or refuses the start.
unsafe fn settling() -> Option<Settle> {
    use crate::engine::macro_ui::scrub_track::SLOT;
    if !SLOT.with(|block| block.settling()).unwrap_or(false) {
        return None;
    }
    let verdict = observe().map_or(Settle::Refuse, |(_, raws)| settle(cpu_body(&raws[1])));
    if verdict != Settle::Wait {
        SLOT.with(|block| block.settled());
    }
    Some(verdict)
}

/// The CPU for `settle`: the gate's own situation and status reads, the landing statuses, and the floor under an
/// airborne body by the shared stage rule.
unsafe fn cpu_body(cpu: &Raw) -> Body {
    use crate::model::cpu::ids::{C as Cpu, UNRESOLVED};
    use crate::model::stage_geometry::{classify, Place, Unread};
    use crate::runtime::features::{cpu::konst, stage_geometry::RayProbe, tech_surfaces};
    let (first, last) = (konst(Cpu::StatusLanding), konst(Cpu::StatusLandingDamageLight));
    let landing = first != UNRESOLVED && last != UNRESOLVED && (first..=last).contains(&cpu.status);
    let mut body = super::with(|state| Body {
        ground: state.ids.is(C::SituationGround, cpu.situation),
        air: state.ids.is(C::SituationAir, cpu.situation),
        idle: state.ids.is(C::StatusWait, cpu.status),
        falling: state.ids.is(C::StatusFall, cpu.status),
        landing,
        gone: [C::StatusDead, C::StatusStandby, C::StatusRebirth].into_iter().any(|gone| state.ids.is(gone, cpu.status)),
        floor: false,
    })
    .unwrap_or(Body { gone: true, ..Body::default() });
    if body.air {
        let at = [natives::pos_x(cpu.accessor), natives::pos_y(cpu.accessor)];
        body.floor = tech_surfaces::blast_bounds().is_some_and(|blast| {
            RayProbe::new(cpu.accessor).ok_or(Unread).and_then(|mut stage| classify(&mut stage, blast, at)) == Ok(Place::OverStage)
        });
    }
    body
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
