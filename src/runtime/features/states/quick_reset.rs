//! Waiting-room reset adapter. Native facts are read outside all state locks.
use super::{Raw, Role, ACTIVE, KILLING, LOADING, TEMPORARY};
use crate::engine::lobby::practice_policy::Context;
use crate::model::input_delay::MappedInputs;
use crate::model::state::quick_reset::{Pose, QuickReset};
use crate::runtime::features::{input_delay, notices, recorder};
use crate::runtime::game::natives;
use crate::runtime::lobby::training::practice;
use crate::runtime::macros::{named_macros, replay_capture, replay_preview};
use crate::runtime::published;
use hos_lock::Mutex;
use std::sync::atomic::Ordering;

static QUICK: Mutex<QuickReset> = Mutex::new(QuickReset::new());
const NOT_READY: &str = "Wait for both practice fighters to finish loading";

pub(super) fn clear() {
    QUICK.lock().clear();
}

unsafe fn entries() -> Option<[usize; 2]> {
    if practice::context() != Some(Context::Waiting) || !practice::enabled() {
        return None;
    }
    Some([practice::entry(0)?, practice::entry(1)?])
}

pub(super) unsafe fn observe(raw: &Raw, nana: bool) {
    if nana || LOADING.load(Ordering::Acquire) || !QUICK.lock().needs_pose(raw.role.index()) {
        return;
    }
    let Some(entries) = entries() else { return };
    let waiting = super::with(|s| {
        s.ids
            .is(crate::model::state::ids::C::StatusWait, raw.status)
    })
    .unwrap_or(false);
    if !waiting {
        return;
    }
    let pose = Pose {
        x: natives::pos_x(raw.accessor),
        y: natives::pos_y(raw.accessor),
        lr: natives::lr(raw.accessor),
        percent: natives::damage(raw.accessor, 0),
        kind: raw.kind,
        situation: raw.situation,
        stage: natives::stage_id(),
    };
    QUICK.lock().observe(entries, raw.role.index(), true, pose);
}

/// On the game thread only. A temporary pair never reaches the slot writer.
pub(crate) unsafe fn request() -> Result<(), &'static str> {
    if !ACTIVE.load(Ordering::Acquire) || !practice::owned() {
        return Err(NOT_READY);
    }
    let entries = entries().ok_or(NOT_READY)?;
    let pair = QUICK.lock().pair(entries).ok_or(NOT_READY)?;
    let accepted = super::with(|s| {
        let accepted = s.machine.begin_temporary(pair.map(Pose::record));
        if accepted {
            LOADING.store(true, Ordering::Release);
            KILLING.store(true, Ordering::Release);
            TEMPORARY.store(true, Ordering::Release);
        }
        accepted
    })
    .unwrap_or(false);
    if !accepted {
        return Err("A restore is already in progress");
    }
    recorder::on_load_request();
    super::hud_load(Role::Player.index(), true);
    super::hud_load(Role::Cpu.index(), true);
    Ok(())
}

/// Returns true when this chord owns the mapped poll, including while held.
/// # Safety
/// `controller` and `out` are the live mapping-call controller and output.
pub(crate) unsafe fn on_input(controller: usize, out: *mut MappedInputs) -> bool {
    if controller == 0 {
        return false;
    }
    let raw = crate::runtime::macros::port::input::Controller::read(controller)
        .current_buttons
        .0;
    let eligible = practice::context() == Some(Context::Waiting)
        && practice::enabled()
        && practice::owned()
        && published::lra_reset()
        && !published::input_owners().any()
        && !named_macros::panel_visible()
        && !replay_capture::panel_visible()
        && !replay_capture::browser_visible()
        && !replay_preview::occupied();
    let (consume, request_now) = QUICK.lock().input(raw, eligible);
    if !consume {
        return false;
    }
    input_delay::clear(crate::model::input_delay::Clear::TrainingReset);
    *out = MappedInputs::NEUTRAL;
    if request_now && super::request_quick_reset() == Err(NOT_READY) {
        notices::upsert(
            0x28,
            "Quick Reset",
            NOT_READY,
            120,
            crate::model::notify::DEFAULT_COLOR,
        );
    }
    true
}
