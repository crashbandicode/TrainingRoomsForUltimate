//! Optional read-ahead intent. No native pointer, lease, or installer is owned here.
//! Actual loading uses CharacterSources' existing native callback and lifetime book.
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use hos_lock::Mutex;
use crate::engine::preload::cycle_admission_grace::{AdmissionGrace, Settlement};
pub(crate) use crate::engine::preload::cycle_preload_capacity::MAX_TARGETS;
use crate::engine::mii::lifetime::Configuration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub kind: u32,
    pub configuration: Option<Configuration>,
}

impl Target {
    pub const fn fighter(kind: u32) -> Self {
        Self {
            kind,
            configuration: None,
        }
    }

    fn fighter_cache(mut self) -> Self {
        if self.configuration.is_none() {
            self.kind = crate::engine::preload::cycle_preload_policy::fighter_group(self.kind);
        }
        self
    }
}

static POLL: AtomicU32 = AtomicU32::new(0);
static FRAME: AtomicU32 = AtomicU32::new(0);
static REQUESTS: Mutex<[Option<crate::engine::preload::preload_metrics::OperationStamp>; 4]> = Mutex::new([None; 4]);
static CONDITION: Mutex<Option<crate::engine::preload::cycle_preload_config::Settings>> = Mutex::new(None);
// This mutex owns values only. No source/roster/native lock is acquired here.
// Final admission while source STATE is held reads only the atomic snapshot.
struct GraceCoordinator {
    generation: u64,
    policy: AdmissionGrace,
    deferred_logged: bool,
}
static GRACE: Mutex<GraceCoordinator> = Mutex::new(GraceCoordinator {
    generation: 1,
    policy: AdmissionGrace::new(1),
    deferred_logged: false,
});
static GRACE_GENERATION: AtomicU64 = AtomicU64::new(1);
static GRACE_DEADLINE: AtomicU64 = AtomicU64::new(0);

fn grace_now() -> Option<u64> {
    // Raw monotonic clock only: no sampling, metrics flag or metrics session.
    crate::engine::preload::preload_metrics::micros(crate::engine::preload::preload_metrics::ticks(),
        crate::engine::preload::preload_metrics::tick_hz())
}

fn advance_grace(grace: &mut GraceCoordinator, now: u64) {
    if grace.policy.advance(now, grace.generation) {
        GRACE_DEADLINE.store(0, Ordering::Release);
        crate::engine::compat::logging::diagnostic(format_args!(
            "[PreloadGrace] expire generation={} us={}", grace.generation, now));
    }
}

/// Scene/settings retirement, independent of measurements and sparse planning.
pub(crate) fn reset_admission_grace() {
    let mut grace = GRACE.lock();
    grace.generation = grace.generation.checked_add(1).expect("preload grace generation exhausted");
    let generation = grace.generation;
    grace.policy.reset(generation);
    grace.deferred_logged = false;
    // Retire the old generation before clearing its deadline, so a concurrent final
    // recheck can never pair the old generation with the cleared deadline.
    GRACE_GENERATION.store(generation, Ordering::Release);
    GRACE_DEADLINE.store(0, Ordering::Release);
}

/// Only real success paths call this, after readiness/provenance proof and
/// before releasing their existing INSTALL ownership. No request stamp needed.
pub(crate) fn foreground_settled() {
    // Fail open: without a monotonic clock no burst is armed, so speculation is never held.
    let Some(now) = grace_now() else { return; };
    let mut grace = GRACE.lock();
    advance_grace(&mut grace, now);
    let generation = grace.generation;
    let event = grace.policy.settled(now, generation);
    GRACE_DEADLINE.store(grace.policy.deadline().unwrap_or(0), Ordering::Release);
    let label = match event {
        Settlement::Started => {
            grace.deferred_logged = false;
            "start"
        }
        Settlement::Extended => "extend",
        Settlement::Unchanged => return,
    };
    crate::engine::compat::logging::diagnostic(format_args!(
        "[PreloadGrace] {} generation={} us={} deadline_us={}",
        label, generation, now, grace.policy.deadline().unwrap_or(0)));
}

/// Capture outside source STATE, after owned-load service. Returning None only
/// postpones new speculative work; plans, attempts and warm sources stay intact.
pub(crate) fn speculative_admission() -> Option<u64> {
    let mut grace = GRACE.lock();
    let Some(now) = grace_now() else { return Some(grace.generation); };
    advance_grace(&mut grace, now);
    let generation = grace.generation;
    if grace.policy.allows(now, generation, true) { return Some(generation); }
    if !grace.deferred_logged {
        grace.deferred_logged = true;
        crate::engine::compat::logging::diagnostic(format_args!(
            "[PreloadGrace] speculative-admission-deferred generation={} us={} deadline_us={}",
            generation, now, grace.policy.deadline().unwrap_or(0)));
    }
    None
}

/// Lock-free value/generation recheck at the serialized final admission seam.
/// INSTALL prevents another visible settlement after that token is acquired.
pub(crate) fn speculative_admission_current(generation: u64) -> bool {
    if GRACE_GENERATION.load(Ordering::Acquire) != generation { return false; }
    let deadline = GRACE_DEADLINE.load(Ordering::Acquire);
    (deadline == 0 || grace_now().is_none_or(|now| now >= deadline))
        && GRACE_GENERATION.load(Ordering::Acquire) == generation
}

/// Call only after SourceBook::begin_install succeeds, never for reservations
/// or preflight refusals. Pure value lock; never traverses source/roster state.
pub(crate) fn speculative_admission_committed(generation: u64) {
    let mut grace = GRACE.lock();
    let now = grace_now();
    if let Some(now) = now { advance_grace(&mut grace, now); }
    if grace.policy.speculative_committed(generation) {
        GRACE_DEADLINE.store(0, Ordering::Release);
        grace.deferred_logged = false;
        crate::engine::compat::logging::diagnostic(format_args!(
            "[PreloadGrace] speculative-admission-consumed generation={} us={}", generation, now.unwrap_or(0)));
    }
}

pub(crate) fn frame() -> u32 {
    FRAME.load(Ordering::Relaxed)
}

pub(crate) fn settings_changed() {
    reset_admission_grace();
    crate::runtime::roster::character_roster::sources::pause_preload();
    close_requests(crate::engine::preload::preload_metrics::Outcome::Abandoned);
    POLL.store(0, Ordering::Relaxed);
    reset_status();
}

/// Fighter status on the existing notice queue: built and prefetched fighters, and "Done" once no
/// background work is left (`CacheStatus`). Asset progress is logged. The queue shows one entry at
/// a time, so this is one standing notice per scene, updated in place when the displayed state
/// changes and kept behind every other notice (`notifications::upsert`): it neither gives way nor
/// vanishes. A state counts as shown only after the notice was visible for its whole time (not
/// while the quick menu hides it); a notice another writer wiped is raised again. Runs on the
/// sparse planning boundary only.
struct Status {
    /// What the notice holds or last showed in full; raised again only when this changes.
    shown: Option<crate::engine::preload::cycle_preload_policy::CacheStatus>,
    /// `shown` is queued or on screen and has not completed its visible time yet.
    pending: bool,
    assets_logged: Option<crate::engine::preload::cycle_preload_policy::Readiness>,
    /// Diagnostic dedupe of the `[PreloadStatus]` line.
    fighters_logged: Option<(crate::engine::preload::cycle_preload_policy::Progress, crate::engine::preload::cycle_preload_policy::BuiltFighters)>,
    paused_polls: u8,
    /// Last unfiltered plan, so a failed target stays counted while blocked.
    desired: [Option<Target>; MAX_TARGETS],
}

static STATUS: Mutex<Status> = Mutex::new(Status {
    shown: None,
    pending: false,
    assets_logged: None,
    fighters_logged: None,
    paused_polls: 0,
    desired: [None; MAX_TARGETS],
});
const STATUS_HEADER: &str = "Fighters";

fn reset_status() {
    *STATUS.lock() = Status {
        shown: None,
        pending: false,
        assets_logged: None,
        fighters_logged: None,
        paused_polls: 0,
        desired: [None; MAX_TARGETS],
    };
    crate::runtime::lobby::training::ui::notifications::clear_notification(STATUS_HEADER);
}

/// `desired` is the freshly merged plan, or `None` to reuse the last one. `blocked` is the
/// scheduler's gate (asset readiness, diagnostic); `notice_blocked` is what may pause the
/// Fighters notice: a visible operation, never the preloader's own helper or hat loads.
fn poll_status(blocked: bool, notice_blocked: bool, desired: Option<[Option<Target>; MAX_TARGETS]>) {
    let desired = {
        let mut status = STATUS.lock();
        if let Some(desired) = desired {
            status.desired = desired;
        }
        status.desired
    };
    let readiness = crate::runtime::roster::character_roster::sources::preload_readiness(blocked, &desired);
    if crate::runtime::roster::cycle_preload_config::snapshot().measurements {
        let changed = {
            let mut status = STATUS.lock();
            let changed = status.assets_logged != Some(readiness);
            status.assets_logged = Some(readiness);
            changed
        };
        if changed {
            crate::engine::compat::logging::diagnostic(format_args!(
                "[Preload] assets ready={} planned={} configured={} limit={} limited={} failed={} state={:?}",
                readiness.ready, readiness.planned, readiness.configured, readiness.limit,
                readiness.limited(), readiness.failed, readiness.progress));
        }
    }
    #[cfg(feature = "native-body-bank-alpha")]
    let fighters = crate::runtime::roster::character_roster::body_bank::progress();
    #[cfg(not(feature = "native-body-bank-alpha"))]
    let fighters = crate::engine::preload::cycle_preload_policy::BuiltFighters::default();
    show_status(crate::engine::preload::cycle_preload_policy::CacheStatus::new(readiness, fighters, notice_blocked), blocked);
}

fn show_status(now: crate::engine::preload::cycle_preload_policy::CacheStatus, scheduler_blocked: bool) {
    use crate::engine::preload::cycle_preload_policy::Progress;
    use crate::runtime::lobby::training::ui::notifications::{clear_notification, is_queued, take_delivered, upsert};
    let mut status = STATUS.lock();
    status.paused_polls = if now.progress() == Progress::Paused {
        status.paused_polls.saturating_add(1)
    } else {
        0
    };
    let key = (now.progress(), now.fighters);
    if status.fighters_logged != Some(key) {
        status.fighters_logged = Some(key);
        crate::engine::compat::logging::diagnostic(format_args!(
            "[PreloadStatus] state={:?} planned={} ready={} prefetched={} failed={} pending={} unstarted={} swap={} blocked={} scheduler_blocked={} text={:?}",
            key.0, key.1.planned, key.1.ready, key.1.prefetched, key.1.failed, key.1.pending,
            key.1.unstarted(), key.1.swap, now.blocked, scheduler_blocked, now.text()));
    }
    if now.progress() == Progress::Hidden {
        status.pending = false;
        if status.shown.take().is_some() {
            clear_notification(STATUS_HEADER);
        }
        return;
    }
    if status.pending {
        if take_delivered(STATUS_HEADER) {
            // Visible for its whole time: this state is shown and stays quiet until it changes.
            status.pending = false;
            crate::engine::compat::logging::diagnostic(format_args!("[PreloadStatus] delivered text={:?}",
                status.shown.map(|shown| shown.text())));
        } else if !is_queued(STATUS_HEADER) {
            // Wiped by another writer (frame advantage, session reset) before it was seen.
            status.pending = false;
            crate::engine::compat::logging::diagnostic(format_args!("[PreloadStatus] wiped text={:?}; raised again",
                status.shown.map(|shown| shown.text())));
            status.shown = None;
        }
    }
    if now.announce(status.shown, status.paused_polls) {
        status.shown = Some(now);
        status.pending = true;
        crate::engine::compat::logging::diagnostic(format_args!("[PreloadStatus] raise text={:?} frames={}",
            now.text(), now.frames()));
        use std::fmt::Write;
        let mut text = crate::model::storage::FixedPath::<96>::new();
        write!(text, "{}", now.text()).expect("bounded status text fits notice buffer");
        upsert(STATUS_HEADER, text.as_str(), now.frames());
    }
}

pub(crate) fn request(role: usize, hat: bool, target: Target) {
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    if !settings.measurements {
        return;
    }
    let stamp = crate::engine::preload::preload_metrics::OperationStamp::begin(
        true,
        crate::engine::preload::preload_metrics::Operation::Request,
        if hat {
            crate::engine::preload::preload_metrics::Target::Hat
        } else {
            crate::engine::preload::preload_metrics::Target::Fighter
        },
        target.kind,
        crate::runtime::roster::character_roster::sources::resident(target, hat),
        FRAME.load(Ordering::Relaxed),
    );
    let old = std::mem::replace(
        &mut REQUESTS.lock()[role + usize::from(hat) * 2],
        stamp,
    );
    if let Some(old) = old {
        old.finish(
            frame(),
            crate::engine::preload::preload_metrics::Outcome::Abandoned,
            crate::runtime::roster::character_roster::sources::metrics(),
        );
    }
}

fn close_requests(outcome: crate::engine::preload::preload_metrics::Outcome) {
    let pending = std::mem::take(&mut *REQUESTS.lock());
    for stamp in pending.iter().flatten() {
        stamp.finish(
            frame(),
            outcome,
            crate::runtime::roster::character_roster::sources::metrics(),
        );
    }
}

pub(crate) fn completed(role: usize, hat: bool, kind: u32) {
    if role == 1 && !hat { POLL.store(0, Ordering::Relaxed); }
    let stamp = {
        let mut requests = REQUESTS.lock();
        let slot = &mut requests[role + usize::from(hat) * 2];
        if slot.is_some_and(|s| s.kind == kind) {
            slot.take()
        } else {
            None
        }
    };
    if let Some(stamp) = stamp {
        stamp.finish(
            FRAME.load(Ordering::Relaxed),
            crate::engine::preload::preload_metrics::Outcome::Ready,
            crate::runtime::roster::character_roster::sources::metrics(),
        );
    }
}

pub(crate) fn scene_end() {
    reset_admission_grace();
    let enabled = CONDITION.lock().take().is_some_and(|c| c.measurements);
    close_requests(crate::engine::preload::preload_metrics::Outcome::Departed);
    crate::engine::preload::preload_metrics::snapshot(
        enabled,
        crate::engine::preload::preload_metrics::Label::SceneEnd,
        FRAME.load(Ordering::Relaxed),
        crate::runtime::roster::character_roster::sources::metrics(),
    );
    POLL.store(0, Ordering::Relaxed);
    reset_status();
}

/// Same ordered mixed-roster resolver as actual cycling, with value-only
/// qualification. Helper admission later checks exact rows and headgear again.
#[cfg(any(
    feature = "mii-ordinary-player-alpha",
    feature = "mii-ordinary-cpu-alpha"
))]
pub(crate) fn mixed_targets(
    menu: &crate::engine::compat::menu::TrainingModpackMenu,
    role: usize,
    original: crate::engine::swap::mii_cycle::Identity,
    mut live: crate::engine::swap::mii_cycle::Live,
    catalog: &crate::engine::mii::inventory::Snapshot,
    origin: Option<Configuration>,
) -> [Option<Target>; MAX_TARGETS] {
    use crate::engine::swap::mii_cycle::{self, Identity, Live};
    // P-CYCLE / P1-CYCLE: the one cycle list. Each role's Mii admission is its owner's published one.
    let roster = if role == 0 {
        crate::engine::swap::cycle_members::cycle_members(menu.player_roster, crate::runtime::roster::character_roster::player_cycle_admission())
    } else {
        crate::engine::swap::cycle_members::cycle_members(menu.cpu_roster, crate::runtime::roster::character_roster::cpu_cycle_admission())
    };
    let preferences = crate::runtime::compat::menu::mii_preferences_view();
    let qualify = |step: mii_cycle::Step| match step.target {
        mii_cycle::Target::Fighter(kind) => Some(Target::fighter(kind)),
        mii_cycle::Target::Original => Some(match original {
            Identity::Fighter(kind) => Target::fighter(kind),
            Identity::Saved(c) => Target {
                kind: c.class as u32,
                configuration: Some(origin?),
            },
        }),
        mii_cycle::Target::Saved(choice) => Some(Target {
            kind: choice.canonical.class as u32,
            configuration: Some(catalog.configuration(choice)?),
        }),
    };
    let mut targets = [None; MAX_TARGETS];
    for target in &mut targets {
        let next = if role == 0 {
            mii_cycle::next_qualified_player_with_ice_climbers(
                &roster,
                catalog,
                &preferences,
                original,
                live,
                menu.solo_popo_original != menu.solo_popo_player_cycle,
                qualify,
            )
        } else {
            mii_cycle::next_qualified_cpu(&roster, catalog, &preferences, original, live, qualify)
        };
        let Some((step, value)) = next else {
            break;
        };
        *target = Some(value);
        live = if step.target == mii_cycle::Target::Original {
            Live::Original
        } else {
            Live::Numbered {
                identity: value
                    .configuration
                    .map_or(Identity::Fighter(value.kind), |c| {
                        Identity::Saved(c.canonical())
                    }),
                cursor: step.cursor,
            }
        };
    }
    targets
}

/// Called once on the visible Player's frame. Planning is bounded to once per
/// second, with an immediate refresh after a completed CPU rotation.
pub unsafe fn on_frame(body: &mut smash::app::BattleObjectModuleAccessor) {
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    if crate::runtime::lobby::training::practice::role(body) != Some(0)
        || !crate::runtime::compat::common::try_get_module_accessor(crate::runtime::compat::common::consts::FighterId::Player)
            .is_some_and(|current| std::ptr::eq(current, body))
    {
        return;
    }
    let frame = FRAME.fetch_add(1, Ordering::Relaxed);
    if settings.measurements && REQUESTS.lock().iter().any(Option::is_some) {
        let busy = crate::runtime::roster::character_roster::sources::metrics().busy;
        for stamp in REQUESTS.lock().iter_mut().flatten() {
            stamp.frame(busy);
        }
    }
    if !POLL.fetch_add(1, Ordering::Relaxed).is_multiple_of(60) {
        return;
    }
    if !crate::runtime::compat::common::is_ready_go() || crate::runtime::compat::common::menu::captures_input() {
        return;
    }
    if settings.slots == 0 && !settings.measurements { return; }
    // The UI gates defer every load and are visible operations. `preload_blocked` also holds
    // speculation behind the preloader's own helper and hat loads, which never pause the notice.
    let ui_blocked = crate::runtime::lobby::training::lobby::native_ui_busy()
        || crate::runtime::lobby::training::save_states::is_loading()
        || crate::runtime::lobby::training::recording_or_playback()
        || crate::runtime::compat::replay_preview::occupied();
    if ui_blocked || crate::runtime::roster::character_roster::preload_blocked() {
        poll_status(true, ui_blocked || crate::runtime::roster::character_roster::notice_blocked(), None);
        return;
    }
    // Rejected/no-op requests have no completion callback. Never carry their
    // timers into a later operation or count them as successful swaps.
    close_requests(crate::engine::preload::preload_metrics::Outcome::Abandoned);
    crate::runtime::roster::character_roster::sources::refresh_preload_capacity();
    let key = settings;
    let label = {
        let mut condition = CONDITION.lock();
        let label = if condition.is_none() {
            Some(crate::engine::preload::preload_metrics::Label::Baseline)
        } else if *condition != Some(key) {
            Some(crate::engine::preload::preload_metrics::Label::Settings)
        } else {
            None
        };
        *condition = Some(key);
        label
    };
    if let Some(label) = label {
        crate::engine::preload::preload_metrics::snapshot(
            settings.measurements,
            label,
            frame,
            crate::runtime::roster::character_roster::sources::metrics(),
        );
    }
    if settings.slots == 0 && label == Some(crate::engine::preload::preload_metrics::Label::Settings) {
        crate::engine::preload::preload_metrics::snapshot(
            settings.measurements,
            crate::engine::preload::preload_metrics::Label::Retired,
            frame,
            crate::runtime::roster::character_roster::sources::metrics(),
        );
    }
    if settings.slots == 0 {
        poll_status(false, false, Some([None; MAX_TARGETS]));
        return;
    }
    let epoch = crate::runtime::roster::character_roster::sources::preload_epoch();
    let menu = crate::runtime::compat::sync::read(&crate::runtime::compat::menu::MENU);
    // Keep physical seat identity before fighter_cache, bank filtering and the
    // hat-priority merge. Exact Mii payload and PT/Pyra member order matter.
    let cpu_rotation = crate::runtime::roster::character_roster::preload_targets(&menu, 1);
    crate::runtime::roster::f9_gate::set_fixed_members(
        cpu_rotation.iter().flatten().next().map(|target| target.kind),
        cpu_rotation.iter().flatten().nth(1).map(|target| target.kind),
    );
    let mut lanes = [[None; MAX_TARGETS]; 4];
    if settings.hats {
        lanes[0] = crate::runtime::roster::kirby_hats::preload_targets(&menu, 0);
        lanes[1] = crate::runtime::roster::kirby_hats::preload_targets(&menu, 1);
    }
    if settings.fighters {
        lanes[2] = crate::runtime::roster::character_roster::preload_targets(&menu, 0)
            .map(|t| t.map(Target::fighter_cache));
        lanes[3] = cpu_rotation.map(|t| t.map(Target::fighter_cache));
    }
    #[cfg(feature = "native-body-bank-alpha")]
    {
        let banks = crate::runtime::roster::character_roster::body_bank::plan_budget(&menu, &lanes);
        // Only ordinary fighter look-ahead is covered by a body bank or one of
        // its extra groups. A hat consumer still needs its independent source
        // lease, even for that kind.
        for (role, (lane, bank)) in lanes[2..].iter_mut().zip(banks).enumerate() {
            for target in lane {
                if target.is_some_and(|t| t.configuration.is_none()
                    && (bank.is_some_and(|b| b.index_of(t.kind).is_some())
                        || crate::runtime::roster::character_roster::body_bank::group_member(role, t.kind))) { *target = None; }
            }
        }
        crate::runtime::roster::character_roster::sources::refresh_preload_capacity();
    }
    #[cfg(feature = "native-body-bank-alpha")]
    let helper_slots = crate::runtime::roster::character_roster::body_bank::helper_budget(settings.slots);
    #[cfg(not(feature = "native-body-bank-alpha"))]
    let helper_slots = settings.slots;
    // `kirby_hats::preload_targets` yields Player hats only while the Player's current fighter is
    // Kirby with hats in its cycle: then those hats outrank the CPU's fighter look-ahead.
    let order = if lanes[crate::engine::preload::cycle_preload_policy::PLAYER_HATS].iter().any(Option::is_some) {
        crate::engine::preload::cycle_preload_policy::merge_player_hats_first(lanes, settings.slots)
    } else {
        crate::engine::preload::cycle_preload_policy::merge(lanes, settings.slots)
    };
    // P-HOT (LANE-HOT-SLOTS-REPORT.md §5.1): one merged order under the Cycle Preload cap; the fighter look-ahead keeps
    // what the Ready Fighters budget leaves (`helper_budget`), the hats have their own budget (Ready Hats). A hat of a
    // kind a fighter lane wants is that fighter target: the shared body.
    let hat_budget = crate::engine::preload::hat_slots::ready_hats(menu.cycle_preload_ready_hats).min(settings.slots);
    let fighter = |target: Target| lanes[2].contains(&Some(target)) || lanes[3].contains(&Some(target));
    let plan = crate::engine::preload::hat_slots::budgeted(order, fighter, hat_budget, helper_slots);
    let desired = plan.map(|entry| entry.map(|(target, _)| target));
    crate::runtime::roster::character_roster::sources::set_preload_plan(epoch, desired, plan.map(|entry| entry.map(|(_, purpose)| purpose)));
    poll_status(false, false, Some(desired));
}
