//! Product launch adapter. Game-thread-only owner; no stored live native pointer.
//! The native hitlag resolver is supplied by the core (never a DIAG fixture value).
use super::tech_info_seed as info_seed;
use super::tech_native as native;
use crate::model::cpu::role::FrameReceipt;
use crate::model::tech_launch::{HitHeight, Launch, Launcher, Vec2};
use crate::runtime::game::{self, natives};
use crate::runtime::state_natives::{self as state, Vector3f};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

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
}
static OWNED: hos_lock::Mutex<Option<Owned>> = hos_lock::Mutex::new(None);
static NEXT: AtomicU64 = AtomicU64::new(1);
static RESTORE: AtomicU64 = AtomicU64::new(0);
static YIELD_TO_STATE: AtomicBool = AtomicBool::new(false);

/// Construct and use only on the native game thread. Rc marker prevents thread transfer.
/// place may replace a completed attempt; the first saved pose survives until restore.
pub struct Adapter {
    owner: u64,
    acquired: bool,
    hitlag: HitlagResolver,
    _game_thread: PhantomData<Rc<()>>,
}
impl Adapter {
    /// Caller must be on the game thread and supply the core's verified hitlag resolver.
    pub unsafe fn new(hitlag: HitlagResolver) -> Self {
        Self {
            owner: NEXT.fetch_add(1, Ordering::Relaxed),
            acquired: false,
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
    if !native::launch_ready(accessor) || (launch.di.is_some() && !super::cpu_common::di_ready()) {
        return Err(Error::Guards);
    }
    let id = |name| native::id(name).ok_or(Error::NativeValue);
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
    let (direction, _) = native::native_direction(angle).ok_or(Error::NativeValue)?;
    let magnitude: extern "C" fn(usize, usize, i32, i32, f32, f32, f32) -> f32 =
        std::mem::transmute(game::text_base() + 0x403a20);
    let magnitude = magnitude(0, 0, 0, 0, launch.knockback, 0.0, 0.0);
    let calc: extern "C" fn(f32) -> f32 = std::mem::transmute(game::text_base() + 0x404ec0);
    let reaction = calc(launch.knockback);
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
    })
}
unsafe fn position(accessor: usize, pos: Vec2, facing: f32) {
    state::clear_speed_all(accessor);
    state::init_pos(
        accessor,
        &Vector3f {
            x: pos.x,
            y: pos.y,
            z: 0.0,
        },
    );
    state::set_lr(accessor, facing);
}
unsafe fn hold(accessor: usize, owned: Owned) {
    state::set_sleep(accessor, true);
    state::clear_speed_all(accessor);
    state::set_pos(
        accessor,
        &Vector3f {
            x: owned.launch.start.x,
            y: owned.launch.start.y,
            z: 0.0,
        },
    );
    state::set_lr(accessor, owned.launch.facing);
}
impl Launcher for Adapter {
    type Error = Error;
    fn place(&mut self, launch: Launch) -> Result<(), Error> {
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
            };
            *slot = Some(owned);
            drop(slot);
            self.acquired = true;
            RESTORE
                .compare_exchange(self.owner, 0, Ordering::AcqRel, Ordering::Acquire)
                .ok();
            position(accessor, launch.start, launch.facing);
            hold(accessor, owned);
            Ok(())
        }
    }
    fn launch(&mut self) -> Result<(), Error> {
        unsafe {
            if super::states::is_loading() || YIELD_TO_STATE.load(Ordering::Acquire) {
                return Err(Error::Busy);
            }
            let mut slot = OWNED.try_lock().ok_or(Error::Busy)?;
            let mut owned = slot
                .filter(|x| x.owner == self.owner && x.phase == Phase::Held)
                .ok_or(Error::NotPlaced)?;
            let accessor = current(owned)?;
            if !native::launch_ready(accessor) {
                return Err(Error::Guards);
            }
            owned.phase = Phase::Flying;
            *slot = Some(owned);
            drop(slot); // DI hook can read ownership during native init/release.
            state::set_sleep(accessor, false);
            let plan = super::tech_launch_native::Plan {
                start: owned.launch.start,
                facing: owned.launch.facing,
                percent: owned.launch.percent,
                reaction: owned.prepared.reaction,
                seed: owned.prepared.seed,
                frames: owned.prepared.stop,
                reserve_lr: Some(owned.launch.reserve_damage_lr()),
                ground_to_air: owned.saved.grounded,
            };
            if let Err(error) = super::tech_launch_native::launch(accessor, plan, |_| Some(())) {
                crate::runtime::diag_log::diagnostic(format_args!(
                    "[TechPractice] launch_refused reason={error:?}"
                ));
                self.restore();
                return Err(Error::NativeValue);
            }
            Ok(())
        }
    }
    fn restore(&mut self) {
        if !self.acquired {
            return;
        }
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
        state::set_sleep(accessor, false);
        if owned.phase == Phase::Yielded || super::states::is_loading() {
            return;
        }
        position(accessor, owned.saved.position, owned.saved.facing);
        state::heal(accessor, -natives::damage(accessor, 0));
        state::add_damage(accessor, owned.saved.percent);
        state::set_situation_kind(
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
    let Some(owned) = *slot else {
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
    drop(slot);
    if owned.phase == Phase::Yielded {
        if let Some(mut slot) = OWNED.try_lock() {
            if slot.is_some_and(|x| x.owner == owned.owner) {
                state::set_sleep(natives::fighter_accessor(fighter), false);
                *slot = None;
            }
        }
    } else if owned.phase == Phase::Held {
        hold(natives::fighter_accessor(fighter), owned);
    }
}
/// Existing DI hook calls this before its CPU role gate. None preserves player DI.
pub(super) unsafe fn di_before(accessor: usize) -> bool {
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
