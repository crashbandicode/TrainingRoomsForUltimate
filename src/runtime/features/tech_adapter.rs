//! Product launch adapter. Game-thread-only owner; no stored live native pointer.
//! The native hitlag resolver is supplied by the core (never a DIAG fixture value).
use super::tech_info_seed as info_seed;
use super::tech_native;
use crate::model::cpu::role::FrameReceipt;
use crate::model::tech_launch::{
    Attempt, AttemptState, ContactReceipt, DecisionDelivery, DecisionSnapshot, HitHeight, Launch,
    Launcher, Surface, Vec2,
};
use crate::runtime::game::natives;
use crate::runtime::state_natives::{self, Vector3f};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    Busy,
    NoPlayer,
    Guards,
    NativeValue,
    HitlagUnavailable,
    NotPlaced,
}
/// Core-owned runtime native-param calculation. It must validate its own native calls.
pub type HitlagResolver = unsafe fn(usize, f32) -> Result<i32, Error>;
/// Game-thread recorder: must not block, allocate or mutate native state.
/// Return true only after accepting this exact snapshot; false retains it for retry.
pub type DecisionObserver = fn(DecisionSnapshot) -> bool;
static DECISION_OBSERVER: OnceLock<DecisionObserver> = OnceLock::new();
/// Bind outside hooks, before starting a session. No observer/calibration default exists.
pub fn bind_decision_observer(observer: DecisionObserver) -> Result<(), DecisionObserver> {
    DECISION_OBSERVER.set(observer)
}
/// Separate from launch readiness: both common observation prerequisites must be installed.
pub fn decision_observer_ready() -> bool {
    tech_ready() && super::cpu_common::contact_ready() && DECISION_OBSERVER.get().is_some()
}
#[derive(Clone, Copy)]
struct Saved {
    position: Vec2,
    facing: f32,
    percent: f32,
    grounded: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Held,
    Flying,
    Yielded,
}
#[derive(Clone, Copy)]
struct Prepared {
    seed: info_seed::Seed,
    reaction: f32,
    stop: i32,
    air: i32,
    ground: i32,
    wait: i32,
    fall: i32,
    di_x: i32,
    di_y: i32,
    contact_statuses: [i32; 11],
}
#[derive(Clone, Copy)]
struct Owned {
    owner: u64,
    receipt: FrameReceipt,
    accessor_token: usize,
    saved: Saved,
    launch: Launch,
    prepared: Prepared,
    phase: Phase,
    attempt: Attempt,
    decision: DecisionDelivery,
}
static OWNED: hos_lock::Mutex<Option<Owned>> = hos_lock::Mutex::new(None);
static NEXT: AtomicU64 = AtomicU64::new(1);
static RESTORE: AtomicU64 = AtomicU64::new(0);
static YIELD_TO_STATE: AtomicBool = AtomicBool::new(false);
static READY: AtomicBool = AtomicBool::new(false);

/// Boot publishes this only after the TechPractice closure passed every guard.
pub(crate) fn set_ready(ready: bool) {
    READY.store(ready, Ordering::Release);
}
pub(crate) fn tech_ready() -> bool {
    READY.load(Ordering::Acquire)
}

/// Construct and use only on the native game thread. Rc marker prevents thread transfer.
/// place may replace a completed attempt; the first saved pose survives until restore.
pub struct Adapter {
    owner: u64,
    acquired: bool,
    last_attempt: Attempt,
    hitlag: HitlagResolver,
    _game_thread: PhantomData<Rc<()>>,
}
impl Adapter {
    /// Caller must be on the game thread and supply the core's verified hitlag resolver.
    pub unsafe fn new(hitlag: HitlagResolver) -> Self {
        Self {
            owner: NEXT.fetch_add(1, Ordering::Relaxed),
            acquired: false,
            last_attempt: Attempt::IDLE,
            hitlag,
            _game_thread: PhantomData,
        }
    }
}
unsafe fn player() -> Result<(usize, FrameReceipt), Error> {
    let receipt = super::cpu::tech_player_receipt()
        .map_err(|_| Error::Busy)?
        .ok_or(Error::NoPlayer)?;
    let accessor = natives::module_accessor(0).ok_or(Error::NoPlayer)?;
    if accessor < 0x1000 || accessor & 7 != 0 || accessor >> 47 != 0 {
        return Err(Error::NoPlayer);
    }
    Ok((accessor, receipt))
}
unsafe fn current(owned: Owned) -> Result<usize, Error> {
    let (accessor, receipt) = player()?;
    (receipt == owned.receipt && accessor == owned.accessor_token)
        .then_some(accessor)
        .ok_or(Error::NoPlayer)
}
unsafe fn prepare(
    accessor: usize,
    launch: Launch,
    hitlag: HitlagResolver,
) -> Result<Prepared, Error> {
    if !launch.valid() {
        return Err(Error::InvalidInput);
    }
    if !tech_native::launch_ready(accessor)
        || (launch.di.is_some() && !super::cpu_common::di_ready())
    {
        return Err(Error::Guards);
    }
    let id = |name| tech_native::id(name).ok_or(Error::NativeValue);
    let heights = [
        id("HIT_HEIGHT_LOW")?,
        id("HIT_HEIGHT_CENTER")?,
        id("HIT_HEIGHT_HIGH")?,
    ];
    let height = heights[match launch.height {
        HitHeight::Low => 0,
        HitHeight::Middle => 1,
        HitHeight::High => 2,
    }];
    let angle = launch.angle.rem_euclid(360.0);
    let (direction, _) = tech_native::native_direction(angle).ok_or(Error::NativeValue)?;
    let magnitude = natives::tech_magnitude(launch.knockback).ok_or(Error::Guards)?;
    let reaction = natives::tech_reaction(launch.knockback).ok_or(Error::Guards)?;
    let stop = hitlag(accessor, launch.hit_damage)?;
    let seed = info_seed::Seed {
        height,
        speed: [magnitude * direction.x, magnitude * direction.y],
        angle: info_seed::relative_angle(angle, launch.away_from_hit_toward_right()),
    };
    if !magnitude.is_finite()
        || magnitude < 0.0
        || !reaction.is_finite()
        || reaction <= 0.0
        || stop <= 0
        || !seed.valid(heights)
    {
        return Err(Error::NativeValue);
    }
    Ok(Prepared {
        seed,
        reaction,
        stop,
        air: id("SITUATION_KIND_AIR")?,
        ground: id("SITUATION_KIND_GROUND")?,
        wait: id("FIGHTER_STATUS_KIND_WAIT")?,
        fall: id("FIGHTER_STATUS_KIND_FALL")?,
        di_x: id("FIGHTER_STATUS_DAMAGE_WORK_FLOAT_VECOR_CORRECT_STICK_X")?,
        di_y: id("FIGHTER_STATUS_DAMAGE_WORK_FLOAT_VECOR_CORRECT_STICK_Y")?,
        contact_statuses: [
            id("FIGHTER_STATUS_KIND_PASSIVE")?,
            id("FIGHTER_STATUS_KIND_PASSIVE_FB")?,
            id("FIGHTER_STATUS_KIND_DOWN")?,
            id("FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_D")?,
            id("FIGHTER_STATUS_KIND_PASSIVE_CEIL")?,
            id("FIGHTER_STATUS_KIND_STOP_CEIL")?,
            id("FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_U")?,
            id("FIGHTER_STATUS_KIND_PASSIVE_WALL")?,
            id("FIGHTER_STATUS_KIND_PASSIVE_WALL_JUMP")?,
            id("FIGHTER_STATUS_KIND_STOP_WALL")?,
            id("FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_LR")?,
        ],
    })
}
unsafe fn position(accessor: usize, pos: Vec2, facing: f32) {
    state_natives::clear_speed_all(accessor);
    state_natives::init_pos(
        accessor,
        &Vector3f {
            x: pos.x,
            y: pos.y,
            z: 0.0,
        },
    );
    state_natives::set_lr(accessor, facing);
}
unsafe fn hold(accessor: usize, owned: Owned) {
    state_natives::set_sleep(accessor, true);
    state_natives::clear_speed_all(accessor);
    state_natives::set_pos(
        accessor,
        &Vector3f {
            x: owned.launch.start.x,
            y: owned.launch.start.y,
            z: 0.0,
        },
    );
    state_natives::set_lr(accessor, owned.launch.facing);
}
impl Launcher for Adapter {
    type Error = Error;
    fn place(&mut self, launch: Launch) -> Result<(), Error> {
        if !tech_ready() {
            return Err(Error::Guards);
        }
        unsafe {
            if super::states::is_loading() || YIELD_TO_STATE.load(Ordering::Acquire) {
                return Err(Error::Busy);
            }
            let (accessor, receipt) = player()?;
            let prepared = prepare(accessor, launch, self.hitlag)?;
            let mut slot = OWNED.try_lock().ok_or(Error::Busy)?;
            let saved = match *slot {
                Some(old) if old.owner != self.owner || old.phase == Phase::Yielded => {
                    return Err(Error::Busy)
                }
                Some(old) if old.receipt == receipt && old.accessor_token == accessor => old.saved,
                _ => Saved {
                    position: Vec2 {
                        x: natives::pos_x(accessor),
                        y: natives::pos_y(accessor),
                    },
                    facing: natives::lr(accessor),
                    percent: natives::damage(accessor, 0),
                    grounded: natives::is_grounded(accessor),
                },
            };
            if !saved.position.x.is_finite()
                || !saved.position.y.is_finite()
                || !saved.percent.is_finite()
                || !saved.facing.is_finite()
            {
                return Err(Error::NativeValue);
            }
            let owned = Owned {
                owner: self.owner,
                receipt,
                accessor_token: accessor,
                saved,
                launch,
                prepared,
                phase: Phase::Held,
                attempt: Attempt::held(NEXT.fetch_add(1, Ordering::Relaxed)),
                decision: DecisionDelivery::EMPTY,
            };
            *slot = Some(owned);
            drop(slot);
            self.acquired = true;
            self.last_attempt = owned.attempt;
            RESTORE
                .compare_exchange(self.owner, 0, Ordering::AcqRel, Ordering::Acquire)
                .ok();
            position(accessor, launch.start, launch.facing);
            hold(accessor, owned);
            Ok(())
        }
    }
    fn launch(&mut self) -> Result<(), Error> {
        if !tech_ready() {
            return Err(Error::Guards);
        }
        unsafe {
            if super::states::is_loading() || YIELD_TO_STATE.load(Ordering::Acquire) {
                return Err(Error::Busy);
            }
            let mut slot = OWNED.try_lock().ok_or(Error::Busy)?;
            let mut owned = slot
                .filter(|x| x.owner == self.owner && x.phase == Phase::Held)
                .ok_or(Error::NotPlaced)?;
            let accessor = current(owned)?;
            if !tech_native::launch_ready(accessor) {
                return Err(Error::Guards);
            }
            if owned.launch.grounded_hit
                && (natives::situation_kind(accessor) != owned.prepared.ground
                    || !natives::is_grounded(accessor))
            {
                return Err(Error::NativeValue);
            }
            owned.phase = Phase::Flying;
            *slot = Some(owned);
            drop(slot); // DI hook can read ownership during native init/release.
            state_natives::set_sleep(accessor, false);
            let plan = super::tech_launch_native::Plan {
                start: owned.launch.start,
                facing: owned.launch.facing,
                percent: owned.launch.percent,
                reaction: owned.prepared.reaction,
                seed: owned.prepared.seed,
                frames: owned.prepared.stop,
                reserve_lr: Some(owned.launch.reserve_damage_lr()),
                ground_to_air: owned.launch.ground_to_air(),
            };
            if let Err(error) = super::tech_launch_native::launch(accessor, plan, |moment| {
                if let super::tech_launch_native::Moment::Stop {
                    stopped: true,
                    damage: true,
                } = moment
                {
                    let mut slot = OWNED.try_lock()?;
                    let active = slot.as_mut().filter(|x| x.owner == self.owner)?;
                    active.attempt.launched();
                }
                Some(())
            }) {
                crate::runtime::diag_log::diagnostic(format_args!(
                    "[TechPractice] launch_refused reason={error:?}"
                ));
                self.restore();
                return Err(Error::NativeValue);
            }
            self.last_attempt.launched();
            Ok(())
        }
    }
    fn restore(&mut self) {
        if !tech_ready() {
            return;
        }
        if !self.acquired {
            return;
        }
        if let Some(slot) = OWNED.try_lock() {
            if let Some(active) = slot.filter(|x| x.owner == self.owner) {
                self.last_attempt = active.attempt;
            }
        }
        self.last_attempt.mark_abandoned();
        // Only an adapter that acquired the lease may enqueue its cleanup.
        if RESTORE
            .compare_exchange(0, self.owner, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            || RESTORE.load(Ordering::Acquire) == self.owner
        {
            self.acquired = false;
            unsafe {
                restore_pending();
            }
        }
    }
    fn attempt(&self) -> Result<Attempt, Error> {
        if !tech_ready() {
            return Err(Error::Guards);
        }
        if !super::cpu_common::contact_ready() {
            return Err(Error::Guards);
        }
        let slot = OWNED.try_lock().ok_or(Error::Busy)?;
        if let Some(active) = slot.filter(|x| x.owner == self.owner) {
            match unsafe { current(active) } {
                Ok(_) => {
                    let mut attempt = active.attempt;
                    if active.phase == Phase::Yielded
                        || RESTORE.load(Ordering::Acquire) == self.owner
                    {
                        attempt.mark_abandoned();
                    }
                    return Ok(attempt);
                }
                Err(Error::Busy) => return Err(Error::Busy),
                Err(_) => {}
            }
        }
        let mut last = self.last_attempt;
        last.mark_abandoned();
        Ok(last)
    }
}
impl Drop for Adapter {
    fn drop(&mut self) {
        self.restore();
    }
}
unsafe fn restore_pending() {
    let owner = RESTORE.load(Ordering::Acquire);
    if owner == 0 {
        return;
    }
    let Some(mut slot) = OWNED.try_lock() else {
        return;
    };
    let Some(owned) = slot.filter(|x| x.owner == owner) else {
        RESTORE
            .compare_exchange(owner, 0, Ordering::AcqRel, Ordering::Acquire)
            .ok();
        return;
    };
    let accessor = current(owned);
    if accessor == Err(Error::Busy) {
        return;
    }
    // Zero-frame set_hit_stop_frame is a native no-op. Keep the owned cleanup queued
    // until native release; a save-state takeover still wakes without restoring pose.
    if let Ok(accessor) = accessor {
        if owned.phase != Phase::Yielded
            && !super::states::is_loading()
            && natives::is_stop(accessor)
        {
            return;
        }
    }
    *slot = None;
    drop(slot);
    RESTORE
        .compare_exchange(owner, 0, Ordering::AcqRel, Ordering::Acquire)
        .ok();
    if let Ok(accessor) = accessor {
        state_natives::set_sleep(accessor, false);
        if owned.phase == Phase::Yielded || super::states::is_loading() {
            return;
        }
        position(accessor, owned.saved.position, owned.saved.facing);
        state_natives::heal(accessor, -natives::damage(accessor, 0));
        state_natives::add_damage(accessor, owned.saved.percent);
        state_natives::set_situation_kind(
            accessor,
            if owned.saved.grounded {
                owned.prepared.ground
            } else {
                owned.prepared.air
            },
        );
        natives::change_status_force(
            accessor,
            if owned.saved.grounded {
                owned.prepared.wait
            } else {
                owned.prepared.fall
            },
            false,
        );
    }
}
/// Existing post-native P1 callback; sleep only pauses animcmd, so hold position every frame.
pub(super) unsafe fn frame(fighter: usize, receipt: Option<FrameReceipt>) {
    if !tech_ready() {
        return;
    }
    if fighter == 0 || ((fighter + 0x10) as *const i32).read() != 0 {
        return;
    }
    // Latch relinquishment: even if loading ends during contention, never reapply the old hold.
    if super::states::is_loading() {
        YIELD_TO_STATE.store(true, Ordering::Release);
    }
    if YIELD_TO_STATE.load(Ordering::Acquire) {
        if let Some(mut slot) = OWNED.try_lock() {
            if let Some(owned) = slot.as_mut() {
                owned.phase = Phase::Yielded;
            }
            YIELD_TO_STATE.store(false, Ordering::Release);
        } else {
            return;
        }
    }
    restore_pending();
    let Some(mut slot) = OWNED.try_lock() else {
        return;
    };
    let Some(mut owned) = *slot else {
        return;
    };
    let fresh = current(owned);
    if fresh == Err(Error::Busy) {
        return;
    }
    if receipt != Some(owned.receipt) || fresh.is_err() {
        *slot = None;
        return;
    }
    if owned.phase == Phase::Flying {
        owned
            .attempt
            .observe_stop(natives::is_stop(natives::fighter_accessor(fighter)));
        if let Ok(accessor) = fresh {
            observe_entry(&mut owned, accessor);
        }
        *slot = Some(owned);
    }
    drop(slot);
    if owned.phase == Phase::Yielded {
        if let Some(mut slot) = OWNED.try_lock() {
            if slot.is_some_and(|x| x.owner == owned.owner) {
                state_natives::set_sleep(natives::fighter_accessor(fighter), false);
                *slot = None;
            }
        }
    } else if owned.phase == Phase::Held {
        hold(natives::fighter_accessor(fighter), owned);
    }
    deliver_decision();
}

/// Retry the retained bytes, never the native reads. The recorder runs without OWNED.
unsafe fn deliver_decision() {
    let Some(observer) = DECISION_OBSERVER.get() else {
        return;
    };
    let Some(slot) = OWNED.try_lock() else { return };
    let Some(owned) = *slot else { return };
    if owned.phase != Phase::Flying
        || RESTORE.load(Ordering::Acquire) == owned.owner
        || super::states::is_loading()
        || current(owned).is_err()
    {
        return;
    }
    let Some(snapshot) = owned.decision.confirmed_decision() else {
        return;
    };
    drop(slot);
    if observer(snapshot) {
        let Some(mut slot) = OWNED.try_lock() else {
            return;
        };
        if let Some(active) = slot
            .as_mut()
            .filter(|x| x.owner == owned.owner && x.attempt.id == snapshot.attempt)
        {
            active.decision.acknowledge_decision(snapshot.attempt);
        }
    }
}
/// Existing DI hook calls this before its CPU role gate. None preserves player DI.
pub(super) unsafe fn di_before(accessor: usize) -> bool {
    if !tech_ready() {
        return false;
    }
    let Some(slot) = OWNED.try_lock() else {
        return false;
    };
    let Some(owned) = *slot else {
        return false;
    };
    drop(slot);
    if owned.phase != Phase::Flying || current(owned) != Ok(accessor) {
        return false;
    }
    let Some(di) = owned.launch.di else {
        return false;
    };
    natives::set_float(accessor, di.x, owned.prepared.di_x);
    natives::set_float(accessor, di.y, owned.prepared.di_y);
    true
}

/// Existing common status-decision seam, before the original native status change.
/// Records only known contact requests for this exact owned player and attempt.
pub(super) unsafe fn decision(
    accessor: usize,
    status: i32,
    passive: impl FnOnce() -> bool,
) -> Option<u64> {
    if !tech_ready() {
        return None;
    }
    let Some(mut slot) = OWNED.try_lock() else {
        return None;
    };
    let Some(owned) = slot.as_mut() else { return None };
    if owned.phase != Phase::Flying
        || !matches!(
            owned.attempt.state,
            AttemptState::Stopped | AttemptState::Flying
        )
        || current(*owned) != Ok(accessor)
    {
        return None;
    }
    let Some(index) = owned
        .prepared
        .contact_statuses
        .iter()
        .position(|&s| s == status)
    else {
        return None;
    };
    let surface = match index {
        0..=3 => Surface::Floor,
        4..=6 => Surface::Ceiling,
        _ => Surface::Wall,
    };
    let contact = ContactReceipt {
        surface,
        requested_status: status,
        passive_enabled: passive(),
        position: Vec2 {
            x: natives::pos_x(accessor),
            y: natives::pos_y(accessor),
        },
    };
    owned.attempt.record_contact(contact);
    if owned.attempt.contact != Some(contact) {
        return None;
    }
    owned.decision.capture_first(super::tech_decision::snapshot(
        accessor,
        owned.attempt.id,
        contact,
    ));
    let attempt = owned.attempt.id;
    drop(slot);
    deliver_decision();
    Some(attempt)
}

/// Read only after fresh ownership validation, and only until the first confirmation.
unsafe fn observe_entry(owned: &mut Owned, accessor: usize) {
    if owned
        .decision
        .pending_decision()
        .is_some_and(|s| s.entered_status.is_none())
    {
        owned.decision.observe_entered_status(
            owned.attempt.id,
            super::tech_decision::entered_status(accessor),
            &owned.prepared.contact_statuses,
        );
    }
}

/// Existing status hook calls this after its original returns. The accessor is an
/// equality token only: resolve the live owned body again before reading native state.
pub(super) unsafe fn confirm_decision(accessor: usize, attempt: u64) {
    if !tech_ready() {
        return;
    }
    let Some(mut slot) = OWNED.try_lock() else { return };
    let Some(owned) = slot.as_mut() else { return };
    if owned.phase != Phase::Flying
        || owned.attempt.id != attempt
        || RESTORE.load(Ordering::Acquire) == owned.owner
        || super::states::is_loading()
    {
        return;
    }
    let Ok(fresh) = current(*owned) else { return };
    if fresh != accessor {
        return;
    }
    observe_entry(owned, fresh);
    drop(slot);
    deliver_decision();
}
