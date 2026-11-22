//! The hitbox visualizer, the runtime half (SPEC-core-06; the rules are `model::hitbox`).
//!
//! With items of the `hitbox_vis` checklist ticked (rc.14, `model::legend`), the ticked kinds of a fighter's boxes in Training (every
//! active hitbox by strength, every hurtbox by status, every grab box; launch-angle marks on request) are drawn with the plugin's own translucent panes (decision HB-OWN-ORBS, option 1, operator 2026-11-05: "go with
//! discs for all"), a capsule as one pill (decision HB-PILLS, 2026-11-14), placed on its bone and projected with the game's
//! camera every layout tick. An article's hitboxes
//! are own orbs too (rc.13, `draw_articles`): the game's effect sphere on a projectile lived its whole flight but never
//! showed. The overlay leaves native animation-command effects to the game.
//!
//! The spheres are kept, not respawned (lead ruling 2026-11-02 on Q3): each fighter's pass works out the spheres it wants and
//! `model::hitbox::HandleSet` holds them through `OrbSink` (never refused, ended at a status change), keeps the unchanged ones
//! and drops the ones that changed or went away. The pass publishes the held spheres' world positions; `draw_orbs` draws them.
//!
//! The animation hooks keep track of live shapes and grabs. The CPU frame wrapper queues the fighter identities
//! and runs practice gameplay. The phase-8 `process_end` hook samples world geometry after the stock model updates.
//! All entry points are behind the install gates and the phase-8 sampling uses bounded lock acquisitions.
//!
//! The hurt spheres (decision HB1) come from the `HitModule` reader (`GameHurt`, over the three relied `hurtbox_*` guard
//! rows in `sites_hitbox`), planned into the same wants as the hit spheres. The game's shared effect pool refused part of a
//! large fighter's spheres every pass (two Ridleys: 22 of 55, Eden run tr-p23-hitbox-ab4); own orbs are never refused.
//! The visualizer's Off and stop paths read no HitModule. A live Perfect recovery plan may sample the current P1's
//! hurtbox height at its original phase through the same guarded reader.
//! The diag build additionally runs the reader once per
//! fighter per frame as the hurt census (`[Hitbox] hurt census`: the parts, the first pass and the read cost).
//! No allocation: the spheres and the handle sets are fixed storage, the Lua stack and attack data are read in place, and
//! every entry point counts under a `HookScope`. Fighter initialization can run on a stock worker.
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};

use hos_lock::Mutex;

use crate::model::cpu::hash::{hash40, lua_bind_hash};
use crate::model::cpu::role::LOBBY_CPU_ENTRY;
use crate::model::eggs::{self, Recovery, Run, TargetEvent, HEADER, NOTICE_ID};
use crate::model::egg_zone::{Side, Zone};
use crate::runtime::features::btt;
use crate::model::notify::DEFAULT_COLOR;
use crate::model::guard::GuardOutcome;
use crate::model::hitbox::{self, layout, AttackReader, Cell, Counts, FighterGenerations, FighterLife, FrameFacts, Indices, SetTable, Sphere, StatusIds};
use crate::model::panes::EggLook;
use crate::model::hitbox_orbs::{self as orbs, Board, Orb, OrbSink};
use crate::model::install_line::SiteReport;
use crate::model::legend;
use crate::model::practice_games::{self, Game};
use crate::model::scene::Scene;
use crate::model::settings::Snapshot;
use crate::model::sites_hitbox;
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::game::{self, natives};
use crate::runtime::{scene, state_natives};

/// Boot: the feature's closure (its rows, the Training predicate and the rows of the natives it shares) passed its guards and its
/// hooks are installed. The two frame entry points do nothing before.
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// `hitbox_vis` draws something (`legend::draws`).
static SETTING: AtomicBool = AtomicBool::new(false);
/// The ticked items of `hitbox_vis` (`legend::shown`): which kinds of orb show, the frame data colours and the angles.
static SHOWN: AtomicU8 = AtomicU8::new(0);

/// Item `bit` of the checklist is ticked.
fn shown(bit: u8) -> bool {
    SHOWN.load(Ordering::Acquire) & bit != 0
}
/// The game's status ids were resolved through its constant table.
static READY: AtomicBool = AtomicBool::new(false);
static IDS: [AtomicI32; hitbox::STATUS_NAMES.len()] = [const { AtomicI32::new(0) }; hitbox::STATUS_NAMES.len()];
/// The work id of a fighter's entry (`hitbox::ENTRY_ID_NAME`), for the lobby gate; resolved with the status ids.
static ENTRY_WORK: AtomicI32 = AtomicI32::new(0);
static ENTRY_WORK_READY: AtomicBool = AtomicBool::new(false);
/// Calls of `ensure_ready` since boot: the table is asked again every 30th call until it answers.
static TRIES: AtomicU32 = AtomicU32::new(0);

/// Boot: the feature installed (or not).
pub fn set_installed(on: bool) {
    INSTALLED.store(on, Ordering::Release);
}

/// Every settings publish.
pub fn publish(snapshot: &Snapshot) {
    let shown = hitbox::shown(snapshot);
    SHOWN.store(shown, Ordering::Release);
    let draws = legend::draws(shown);
    if SETTING.swap(draws, Ordering::AcqRel) && !draws {
        CLEAR_ARTICLES.store(true, Ordering::Release);
    }
    let mut row_bytes = [0; 8];
    row_bytes[..5].copy_from_slice(&eggs::choices(snapshot));
    row_bytes[4] = eggs::checklist(snapshot, eggs::ROW_AREA);
    row_bytes[5] = eggs::choice(snapshot, "eggsercise_hit_streak");
    row_bytes[6] = eggs::checklist(snapshot, eggs::ROW_PATHS);
    row_bytes[7] = eggs::choice(snapshot, eggs::ROW_EXPIRY);
    let rows = u64::from_le_bytes(row_bytes);
    EGG_ROWS.store(rows, Ordering::Release);
    let hide = eggs::choice(snapshot, eggs::ROW_PRACTICE_CPU) == 0;
    PRACTICE_HIDE.store(hide, Ordering::Release);
    btt::publish(snapshot);
    super::tech::publish(snapshot);
    super::edge_juggle::publish(snapshot);
    super::parry::publish(snapshot);
}

// ---- Eggs-ercise (rc.14, `model::eggs`) -------------------------------------------------------------------------------
//
// The rows reach the game thread as atomics (a publish runs on a plugin thread); the run is game-thread state, read by the
// layout tick for the egg's orb like `BOARD`.

/// The rows (`eggs::choices`, little-endian), as every settings publish leaves them.
static EGG_ROWS: AtomicU64 = AtomicU64::new(0);
/// The rows of the run: latched by every practice game's Start (`practice_started`; rc.15 practice menu: a running
/// game's settings apply at its next Start). The Eggs-ercise run and the Breaks on rule of both target games read these.
static EGG_LIVE: AtomicU64 = AtomicU64::new(0);
/// CPU during a game: Hide (the first option).
static PRACTICE_HIDE: AtomicBool = AtomicBool::new(true);
/// CPU during a game for the running game, latched by its Start.
static PRACTICE_HIDE_RUN: AtomicBool = AtomicBool::new(true);
/// Start (rc.15 practice menu; D9 (c) is reversed: a close no longer restarts it): the next pass starts a run.
static EGG_START: AtomicBool = AtomicBool::new(false);
// Save-state placement cannot complete an off-stage recovery.
static EGG_STATE_LOAD: AtomicBool = AtomicBool::new(false);

pub(crate) fn egg_state_loaded() {
    EGG_STATE_LOAD.store(true, Ordering::Release);
}
struct EggState {
    run: eggs::Run,
    streak: crate::model::hit_streak::Streak,
    cpu_streak_dirty: bool,
}
static RUN: Mutex<EggState> = Mutex::new(EggState {
    run: eggs::Run::IDLE,
    streak: crate::model::hit_streak::Streak::EMPTY,
    cpu_streak_dirty: false,
});
/// The menu reads the round state without taking `RUN`.
static EGG_ARMED: AtomicBool = AtomicBool::new(false);
static EGG_FINISHED: AtomicBool = AtomicBool::new(false);
static EGG_GENERATION: AtomicU32 = AtomicU32::new(0);

fn lockon_key(serial: u32) -> u64 {
    (u64::from(EGG_GENERATION.load(Ordering::Relaxed)) << 32) | u64::from(serial)
}

pub(crate) fn lockon_egg() -> Option<Option<crate::model::lockon::Target>> {
    let state = RUN.try_lock()?;
    let run = &state.run;
    let key = lockon_key(run.target_id());
    let egg = run.egg_at().filter(|_| Run::running(run) && !run.cpu_target());
    Some(egg.map(|at| crate::model::lockon::Target { key, at }))
}

fn seed() -> u32 {
    // SAFETY: `GetSystemTick` has no preconditions.
    unsafe { skyline::nn::os::GetSystemTick() as u32 }
}

/// Start (the menu worker, `practice::start`; an atomic only).
pub fn eggs_start() {
    EGG_START.store(true, Ordering::Release);
}

/// Any practice game's Start (the menu worker, after the close's publish; atomics only): CPU during a game and the
/// Eggs-ercise rows are the ones the menu showed at the Start. Break the Targets reads Breaks on from these rows too.
pub fn practice_started() {
    PRACTICE_HIDE_RUN.store(PRACTICE_HIDE.load(Ordering::Acquire), Ordering::Release);
    EGG_LIVE.store(EGG_ROWS.load(Ordering::Acquire), Ordering::Release);
}

/// An Eggs-ercise run is armed, or a Start is on its way to the next pass (the menu worker; atomics only).
pub fn eggs_running() -> bool {
    EGG_ARMED.load(Ordering::Acquire) || EGG_START.load(Ordering::Acquire)
}

pub fn eggs_finished() -> bool {
    EGG_FINISHED.load(Ordering::Acquire) && !EGG_START.load(Ordering::Acquire)
}

/// The reset hub ran (an L+R+A reset, or a new Training scene's first CPU init): an armed run starts over. Game thread.
/// A TR Change Stage initialises no fighter and does not reach the hub (trace boot latch-order-2, release 116a3829).
pub fn eggs_reset() {
    let mut state = RUN.lock();
    state.run.reset(seed());
    state.cpu_streak_dirty = false;
    if state.streak.enabled() {
        state.streak.restart(true);
    }
    EGG_FINISHED.store(false, Ordering::Release);
    EGG_GENERATION.fetch_add(1, Ordering::Relaxed);
}

/// Player 1's pass (entry 0, Ice Climbers' Nana too) steps the run: the clock, the egg, its pills against the egg. It runs
/// ahead of the visualizer's Off return, so it does not depend on Hitbox Visualization. Off, no run, an ended run and every
/// other fighter make no game call.
unsafe fn eggs_pass(fighter: usize) -> Option<()> {
    let seed = EGG_START.swap(false, Ordering::AcqRel).then(seed);
    let bytes = EGG_LIVE.load(Ordering::Acquire).to_le_bytes();
    let mut rows = [bytes[0], bytes[1], bytes[2], bytes[3], eggs::IN_VIEW];
    // rc.15 practice menu: the mode while Eggs-ercise is the running game, Off otherwise.
    rows[0] = practice_games::effective(Game::Eggs, rows[0], super::practice::active());
    // The fighter object's own entry field, as the reset hub reads it: not a game call.
    let entry = ((fighter + crate::model::lra_reset::FIGHTER_ENTRY_ID_FIELD) as *const i32).read();
    let mut state = RUN.lock();
    let EggState { run, streak, cpu_streak_dirty } = &mut *state;
    let plan = run.plan(rows, seed, entry, scene::facts);
    if seed.is_some() {
        run.set_areas(bytes[4]);
        run.configure(bytes[6], bytes[7]);
        *cpu_streak_dirty = false;
        streak.restart(bytes[5] == 1 && Run::running(run));
        EGG_GENERATION.fetch_add(1, Ordering::Relaxed);
    }
    // V2.6: the CPU is hidden while a practice game runs and `CPU during a game` says Hide, unless it is the target
    // (`eggs::hides_cpu`). The CPU's body, its tag and its panel follow this one request (`cpu::hide::step`).
    let armed = run.armed();
    EGG_ARMED.store(armed, Ordering::Release);
    EGG_FINISHED.store(armed && !Run::running(run), Ordering::Release);
    super::hide_cpu::hide(super::tech::running() || eggs::hides_cpu(PRACTICE_HIDE_RUN.load(Ordering::Acquire), armed, run.cpu_target(), btt::running()));
    if plan == eggs::Plan::Clear {
        streak.restart(false);
        super::notices::clear(NOTICE_ID);
    }
    if entry != 0 {
        return eggs_cpu(run, streak, cpu_streak_dirty, fighter, entry);
    }
    // Any menu over the game holds the clock: the plugin's, the game's own pause menu (the fighters' frames run on under
    // it: Eden wa-eggs-07bae4c1-1, E11) and the waiting room's busy native UI.
    let owners = crate::runtime::published::input_owners();
    let held = owners.covers_strip() || super::egg_zone::wizard_active() || super::states::is_loading();
    (plan == eggs::Plan::Read).then_some(())?;
    let at = Some(natives::fighter_accessor(fighter)).filter(|at| *at != 0)?;
    let tick = run.keeper(at) && !held;
    #[cfg(feature = "diag")]
    {
        static SAID: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(2);
        if SAID.swap(u8::from(held), Ordering::Relaxed) != u8::from(held) {
            crate::runtime::diag_log::diagnostic(format_args!("[Eggs] held={held} owners={:#x}", owners.bits()));
        }
    }
    let (top, hits, n, aerial, kind) = p1_read(at);
    let hits = &hits[..if held { 0 } else { n }];
    let before = streak.counts();
    let owner = if !held && !run.ended() {
        super::btt_projectiles::sample(streak, at, hits.iter().any(|h| h.3 > 0.0))
    } else {
        None
    };
    let target = lockon_key(run.target_id());
    let mut source = None;
    #[cfg(feature = "diag")]
    if let Some([dx, dy]) = crate::runtime::diag_reload::take_egg_request() {
        run.place([top[0] + dx, top[1] + dy]);
    }
    #[cfg(feature = "diag")]
    if n != 0 {
        let score = run.score;
        let egg = run.egg_at();
        let breaks = |e| hits.iter().any(|h| eggs::breaks(eggs::ANY, true, h, e));
        let overlap = egg.is_some_and(breaks);
        crate::runtime::diag_log::diagnostic(format_args!(
            "[Eggs] pass at={at:#x} p1={top:?} egg={egg:?} overlap={overlap} air={aerial} score={score} hits={hits:?}"
        ));
    }
    let primary = if run.uses_paths() || run.needs_recovery() || run.wants_zone() {
        natives::module_accessor(0).unwrap_or(at)
    } else {
        at
    };
    if run.wants_zone() && primary == at && !held {
        use super::egg_zone::{self, Geometry};
        let id = natives::stage_id();
        let preferences = (id >= 0)
            .then(|| run.zone_preferences(id, || crate::runtime::ui::worker::egg_zone(id)))
            .flatten();
        match egg_zone::prepare(at) {
            Geometry::Ready => {
                if let Some(zone) = egg_zone::with_stage(|stage| {
                    (stage.id() == id).then(|| stage.resolve(preferences))
                })
                .flatten()
                {
                    run.set_zone(zone);
                }
            }
            Geometry::Unavailable => {
                if let Some(reason) = run.disable_offstage() {
                    let _ = super::notices::post_required(HEADER, reason, 120, DEFAULT_COLOR);
                }
            }
            Geometry::Pending => {}
        }
    }
    if run.floors_wanted() {
        eggs_floors(run, at);
    }
    if run.wants_blast() && !held {
        if let Some(blast) = super::egg_zone::blast() {
            run.set_blast(blast);
        }
    }
    let state_loaded = tick && EGG_STATE_LOAD.swap(false, Ordering::AcqRel);
    if !held && run.needs_recovery() {
        let recovery = if state_loaded && run.recovering() { Recovery::Lost } else { egg_recovery(primary) };
        run.observe_recovery(recovery);
    }
    let player = if primary == at { [top[0], top[1]] } else { [natives::pos_x(primary), natives::pos_y(primary)] };
    run.sense(player);
    // rc.15: Player 1's articles break the target too, as `Breaks on` allows (docs/DECISIONS.md).
    let rule = run.rule();
    let grader = super::input_display::grader_live();
    let (length, depth) = run.pass(
        eggs::Pass { tick, y: top[1], hits, air: aerial, kind },
        |egg| {
            if held {
                return None;
            }
            let helper = super::lockon::hit(at, 0, target, rule, aerial, true);
            source = helper;
            eggs::other_break(grader, helper.is_some(), |deepest| {
                let (projectile, depth) = super::btt_projectiles::hit(
                    at,
                    egg,
                    rule,
                    aerial,
                    true,
                    if deepest { crate::model::btt::Search::Deepest } else { crate::model::btt::Search::First },
                )?;
                source = helper.or(Some(projectile));
                Some(depth)
            })
        },
        &egg_clearance,
    );
    if let Some(depth) = depth {
        super::input_display::target_grade(depth);
    }
    let event = run.event();
    let contact = matches!(event, TargetEvent::Hit | TargetEvent::Pending);
    let credit = owner.and_then(|(owner, _)| {
        if run.melee() { Some((owner, owner)) } else { source.map(|source| (owner, source)) }
    });
    egg_streak(streak, event, credit);
    if let Some((_, body)) = owner {
        streak.end(body);
    }
    // The fighter whose attack broke the egg freezes on contact, including a pending break.
    let broke = contact && run.melee();
    if let Some(frames) = broke.then(scene::facts).and_then(eggs::hitlag) {
        let _ = natives::set_hit_stop_frame(at, frames, false);
    }
    let streak_changed = core::mem::take(cpu_streak_dirty) || streak.counts() != before;
    let length = length.or_else(|| streak_changed.then_some(u16::MAX))?;
    let mut text = crate::model::storage::FixedPath::<96>::new();
    let _ = run.write_notice(&mut text);
    let _ = streak.write(&mut text);
    super::notices::upsert(NOTICE_ID, HEADER, text.as_str(), length, DEFAULT_COLOR);
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!(
        "[Eggs] notice=\"{}\" at={at:#x} p1={top:?} egg={:?} rest={:?} tick={tick} view={:?}",
        text.as_str(),
        run.egg_at(),
        run.rest(),
        run.view
    ));
    Some(())
}

fn egg_clearance(zone: &Zone, side: Side, from: [f32; 2], to: [f32; 2]) -> bool {
    super::egg_zone::with_stage(|stage| stage.allows_move(zone, side, from, to)) == Some(true)
}

unsafe fn egg_recovery(primary: usize) -> Recovery {
    use crate::model::cpu::ids::{C, UNRESOLVED};
    use crate::model::sites_pkg2::{POS_X_OFFSET, POS_Y_OFFSET, SITUATION_KIND_OFFSET, STATUS_KIND_OFFSET};
    use crate::model::tech::surface::Kind;
    use super::cpu::konst;

    if !game::granted(STATUS_KIND_OFFSET) || !game::granted(SITUATION_KIND_OFFSET) {
        return Recovery::Unknown;
    }
    let matches_id = |id, kind| id != UNRESOLVED && id == konst(kind);
    let status = natives::status_kind(primary);
    let situation = natives::situation_kind(primary);
    if matches_id(status, C::StatusDead) || matches_id(status, C::StatusRebirth) {
        return Recovery::Lost;
    }

    if !game::granted(POS_X_OFFSET) || !game::granted(POS_Y_OFFSET) {
        return Recovery::Unknown;
    }
    let player = [natives::pos_x(primary), natives::pos_y(primary)];
    if !player.into_iter().all(f32::is_finite) {
        return Recovery::Unknown;
    }
    let terrain = super::egg_zone::with_stage(|stage| {
        let [left, right] = stage.main_span();
        (player[1] <= stage.bottom_blast(), player[0] < left || player[0] > right || stage.under_main(player))
    });
    let Some((below_blast, away)) = terrain else { return Recovery::Unknown };
    if below_blast {
        return Recovery::Lost;
    }
    if matches_id(status, C::StatusCliffCatch) || matches_id(status, C::StatusCliffWait) {
        return Recovery::Returned;
    }
    if matches_id(situation, C::SituationGround) {
        if let Some(line) = super::tech_surfaces::NativeSource::new(primary).and_then(|mut source| source.touched()) {
            if !line.passable && line.kind() == Some(Kind::Floor)
                && super::egg_zone::with_stage(|stage| stage.supports(line.ends.map(|p| [p.x, p.y]))) == Some(true)
            {
                return Recovery::Returned;
            }
        }
        // A platform is on stage at contact but does not complete a pending return.
        return Recovery::Unchanged;
    }
    if away { Recovery::Away } else { Recovery::Unchanged }
}

fn egg_streak(
    streak: &mut crate::model::hit_streak::Streak,
    event: TargetEvent,
    credit: Option<(u32, u32)>,
) {
    match (event, credit) {
        (TargetEvent::Hit, Some((owner, source))) => streak.hit(owner, source),
        (TargetEvent::Pending, Some((owner, source))) => streak.defer(owner, source),
        (TargetEvent::Recovered, _) => streak.recover(),
        (TargetEvent::Miss, _) => streak.miss(),
        _ => {}
    }
}

/// Native rays of the floor read a pass (`eggs_floors`): Training's stage, the widest, takes about a hundred.
const RAYS_PER_PASS: usize = 12;

/// The floor read of the running game (rc.15): Tech's column walk (`Rays`, stacked platforms included) over the stage's
/// camera range, at most `RAYS_PER_PASS` rays a pass, kept as world geometry. Only Player 1's pass on the game thread uses it.
struct FloorRead(core::cell::RefCell<(Option<crate::model::tech::discovery::Rays>, [eggs::Floor; eggs::FLOORS], usize)>);
// SAFETY: only `eggs_floors`, on the game thread's Player 1 pass, touches the cell; no borrow crosses a call that re-enters it.
unsafe impl Sync for FloorRead {}
static FLOOR_READ: FloorRead = FloorRead(core::cell::RefCell::new((None, [[[0.0; 2]; 2]; eggs::FLOORS], 0)));

/// One pass of the floor read: a run that wants its floors starts it; done, broken or unusable (Tech's surface guards did not
/// pass, no camera range) it hands the floors found so far to the run (none: the run's ground is used).
unsafe fn eggs_floors(run: &mut eggs::Run, at: usize) {
    use super::tech_surfaces::{FloorStep, NativeSource};
    let Ok(mut read) = FLOOR_READ.0.try_borrow_mut() else { return };
    let (rays, floors, n) = &mut *read;
    if !run.pending() {
        run.want_floors();
        (*rays, *n) = (natives::camera_limit_range().and_then(crate::model::tech::discovery::Rays::new), 0);
    }
    let (Some(walk), Some(mut source)) = (rays.as_mut(), NativeSource::new(at)) else {
        run.set_floors(&floors[..*n]);
        return;
    };
    for _ in 0..RAYS_PER_PASS {
        match source.floor_step(walk) {
            FloorStep::Next(Some(f)) if *n < eggs::FLOORS && !floors[..*n].contains(&f) => (floors[*n], *n) = (f, *n + 1),
            FloorStep::Next(_) => {}
            FloorStep::Done | FloorStep::Broken => {
                #[cfg(feature = "diag")]
                crate::runtime::diag_log::diagnostic(format_args!("[Eggs] floors n={} {:?}", *n, &floors[..*n]));
                run.set_floors(&floors[..*n]);
                return;
            }
        }
    }
}

/// The CPU target's pass (V2.1; the practice CPU's own frame, only while a run has it as the target): put it where the run
/// asks, on the target's flight (the operator, 11-15), its speed cleared; afresh, at 0% and in the game's own fall (in the
/// air) or standing (at rest), as a save state's load places a fighter; and standing once the flight comes to rest.
unsafe fn eggs_cpu(
    run: &mut eggs::Run,
    streak: &mut crate::model::hit_streak::Streak,
    streak_dirty: &mut bool,
    fighter: usize,
    entry: i32,
) -> Option<()> {
    run.cpu_target().then_some(())?;
    let primary = [1, LOBBY_CPU_ENTRY][usize::from(scene::current()? == Scene::Lobby)];
    (entry == primary).then_some(())?;
    let at = Some(natives::fighter_accessor(fighter)).filter(|at| *at != 0)?;
    let (damage, stopped) = (natives::damage(at, 0), natives::is_stop(at));
    #[cfg(feature = "diag")]
    let score = run.score;
    let moved = if crate::runtime::published::input_owners().covers_strip()
        || super::egg_zone::wizard_active() || super::states::is_loading()
    {
        return Some(());
    } else {
        run.cpu_pass_in(damage, stopped, &egg_clearance, || {
            natives::module_accessor(0).map_or(Recovery::Unknown, |p1| egg_recovery(p1))
        })
    };
    let event = run.event();
    let before = streak.counts();
    let source = if streak.enabled() && matches!(event, TargetEvent::Hit | TargetEvent::Pending)
        && super::lockon::damage_source_ready()
    {
        super::lockon::damage_source(at)
    } else {
        None
    };
    egg_streak(streak, event, source);
    *streak_dirty |= before != streak.counts();
    #[cfg(feature = "diag")]
    eggs_cpu_diag(run, score, at, damage, stopped, moved);
    // The game turns the placed CPU's tag flag off in the air: while TR owns the placement and the body is shown, the CPU frame
    // wrapper writes it on again right after this pass (`hide_cpu::write_kept_cursor`).
    super::hide_cpu::keep_cursor(fighter, eggs::keeps_cursor(moved, super::hide_cpu::hidden()));
    let ([x, y], put) = moved?;
    state_natives::set_pos(at, &state_natives::Vector3f { x, y, z: 0.0 });
    state_natives::clear_speed_all(at);
    let air = match put {
        eggs::Put::Hold => return Some(()),
        eggs::Put::Fresh(air) => {
            state_natives::heal(at, -natives::damage(at, 0));
            air
        }
        eggs::Put::Land => false,
    };
    let name = ["FIGHTER_STATUS_KIND_WAIT", "FIGHTER_STATUS_KIND_FALL"][usize::from(air)];
    if let Some(status) = game::lua_const(lua_bind_hash(name)) {
        state_natives::status_request(at, status);
    }
    Some(())
}

/// Diag: one line per event of the CPU target (a placement afresh, a landing, a counted hit) and every 6th held tick: where
/// the game's frame left it (`at`, its `top` joint before this pass moves it), where the last pass put it (`want`), the
/// flight's tick and launch, its status and the ids of FALL and WAIT. Eden reads the flight and the game's drift from the
/// path off these. Only natives the feature's closure already has (the joint read, the status, the constant table).
#[cfg(feature = "diag")]
unsafe fn eggs_cpu_diag(run: &eggs::Run, before: u32, at: usize, damage: f32, stop: bool, moved: Option<([f32; 2], eggs::Put)>) {
    // The last pass's position as two f32 bit patterns; all ones is none.
    static WANT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    static TICK: AtomicU32 = AtomicU32::new(0);
    let score = run.score;
    let event = score > before || moved.is_some_and(|m| m.1 != eggs::Put::Hold);
    let next = moved.map_or(u64::MAX, |([x, y], _)| u64::from(x.to_bits()) << 32 | u64::from(y.to_bits()));
    let last = WANT.swap(next, Ordering::Relaxed);
    let tick = TICK.fetch_add(1, Ordering::Relaxed);
    if !event && (last == u64::MAX || moved.is_none() || tick % 6 != 0) {
        return;
    }
    let want = (last != u64::MAX).then(|| [f32::from_bits((last >> 32) as u32), f32::from_bits(last as u32)]);
    let pos = natives::joint_position(at, hash40("top"), &[0.0; 3]).map_or([f32::NAN; 2], |p| [p[0], p[1]]);
    let (status, (egg, age, fly)) = (natives::status_kind(at), run.flight());
    let id = |name| game::lua_const(lua_bind_hash(name)).unwrap_or(-1);
    let (fall, wait) = (id("FIGHTER_STATUS_KIND_FALL"), id("FIGHTER_STATUS_KIND_WAIT"));
    crate::runtime::diag_log::diagnostic(format_args!(
        "[Eggs] cpu damage={damage} score={score} move={moved:?} want={want:?} at={pos:?} egg={egg:?} age={age} fly={fly:?} \
         status={status} fall={fall} wait={wait} stop={stop}"
    ));
}

/// What a practice game reads of Player 1: its top joint, its live attack pills and their number, whether it is in an aerial
/// and its kind.
pub(crate) type P1 = ([f32; 3], [eggs::Hit; hitbox::SLOTS as usize], usize, bool, i32);

/// # Safety
/// `at` is a live fighter's accessor.
pub(crate) unsafe fn p1_read(at: usize) -> P1 {
    let top = natives::joint_position(at, hash40("top"), &[0.0; 3]).unwrap_or_default();
    let (hits, n) = object_hits(at);
    let (aerial, kind) = (natives::status_kind(at) == eggs::ATTACK_AIR, natives::fighter_kind(at));
    (top, hits, n, aerial, kind.unwrap_or(-1))
}

/// Native attack pills of a live fighter or article, independent of visualizer settings.
pub(crate) unsafe fn object_hits(at: usize) -> ([eggs::Hit; hitbox::SLOTS as usize], usize) {
    let (mut wants, mut len) = ([Sphere::NONE; 2 * hitbox::SLOTS as usize], 0);
    hitbox::want_hitboxes(&mut GameAttacks(at), &mut wants, &mut len);
    let mut hits = [eggs::Hit::default(); hitbox::SLOTS as usize];
    let place = |s: &Sphere| natives::joint_position(at, s.joint, &s.pos);
    let power = |slot| GameAttacks(at).read_f32(GameAttacks(at).data(slot) + layout::POWER);
    let n = eggs::pills(&wants[..len], place, power, &mut hits);
    (hits, n)
}

/// One guarded P1 height sample for a live Perfect recovery plan when the visualizer does not read fighter hurtboxes.
unsafe fn perfect_player_height(attempt: &super::edge_juggle::LivePerfect) -> Option<f32> {
    let player = attempt.player();
    let mut reader = hitbox::GameHurt { accessor: player, load: mem_load };
    let mut top = f32::NEG_INFINITY;
    hitbox::hurt_parts(&mut reader, false, |_, shape, _| {
        for offset in std::iter::once(shape.a).chain(shape.b) {
            if let Some([_, y, _]) = natives::joint_position(player, shape.joint, &offset) {
                top = top.max(y + shape.size);
            }
        }
    });
    (top.is_finite()).then_some(top)
}

unsafe fn drawn_player_height(player: usize, wants: &[Sphere]) -> Option<f32> {
    wants.iter()
        .filter(|sphere| sphere.key >> 24 == hitbox::KIND_HURT)
        .filter_map(|sphere| {
            let [_, y, _] = natives::joint_position(player, sphere.joint, &sphere.pos)?;
            let top = y + sphere.scale / hitbox::SCALE;
            top.is_finite().then_some(top)
        })
        .reduce(f32::max)
}

/// The status ids, resolved once through the game's constant table (`None` until it answers; it may not be
/// built at boot). Asked again every 30th call.
fn ensure_ready() -> bool {
    if READY.load(Ordering::Acquire) {
        return true;
    }
    if TRIES.fetch_add(1, Ordering::Relaxed) % 30 != 0 {
        return false;
    }
    let mut ids = [0; hitbox::STATUS_NAMES.len()];
    for (slot, name) in ids.iter_mut().zip(hitbox::STATUS_NAMES) {
        // SAFETY: the constant lookup is a guarded called site (`lua_const` checks its grant).
        let Some(value) = (unsafe { game::lua_const(lua_bind_hash(name)) }) else { return false };
        *slot = value;
    }
    // SAFETY: as above.
    let Some(entry_work) = (unsafe { game::lua_const(lua_bind_hash(hitbox::ENTRY_ID_NAME)) }) else { return false };
    ENTRY_WORK.store(entry_work, Ordering::Release);
    ENTRY_WORK_READY.store(true, Ordering::Release);
    for (slot, value) in IDS.iter().zip(ids) {
        slot.store(value, Ordering::Release);
    }
    READY.store(true, Ordering::Release);
    true
}

fn status_ids() -> StatusIds {
    StatusIds::from_values(std::array::from_fn(|i| IDS[i].load(Ordering::Acquire)))
}

/// Whether the practice visualizes this object (`hitbox::visualizes_entry`): native Training, every object; the waiting room's
/// practice, only its practice fighters (Player 1 and package 2's lobby CPU entry, read from the fighter's work), never an article
/// there; every other scene (online, a hot swap) never.
///
/// The entry's work id comes from the constant table (`ensure_ready`), asked here on the waiting room's own path
/// (`hitbox::practice_entry`): the pass asks `visualizes` before `ensure_ready`, so a boot whose first practice was the waiting
/// room never resolved it and drew no orb until an attack command ran (Eden tr-p23-rc5-1, 2026-11-05).
unsafe fn visualizes(accessor: usize) -> bool {
    let scene = scene::current();
    let fighter = matches!(scene, Some(Scene::Lobby)) && natives::is_fighter(accessor);
    let entry = hitbox::practice_entry(
        scene,
        fighter,
        || (ENTRY_WORK_READY.load(Ordering::Acquire) || ensure_ready()).then(|| ENTRY_WORK.load(Ordering::Acquire)),
        // SAFETY: `accessor` is a live fighter's (the pass's or the hook's own), as `is_fighter` said.
        |work| unsafe { natives::get_int(accessor, work) },
    );
    hitbox::visualizes_entry(scene, entry)
}

// ---- the spheres the plugin keeps (lead ruling 2026-11-02 on Q3) ------------------------------------------------------

/// Every fighter's held spheres, the scratch list of the spheres a pass wants, and the save-state epoch each set last saw.
struct Store {
    table: SetTable,
    seen_epoch: [u32; hitbox::FIGHTER_SETS],
    wants: [Sphere; hitbox::WANT_SCRATCH],
    #[cfg(feature = "diag")]
    article_signatures: [(u32, u64); 8],
    #[cfg(feature = "diag")]
    census: stats::CensusBook,
}

static FIGHTER_GENERATIONS: FighterGenerations = FighterGenerations::new();

static STORE: Mutex<Store> = Mutex::new(Store {
    table: SetTable::new(),
    seen_epoch: [0; hitbox::FIGHTER_SETS],
    wants: [Sphere::NONE; hitbox::WANT_SCRATCH],
    #[cfg(feature = "diag")]
    article_signatures: [(0, 0); 8],
    #[cfg(feature = "diag")]
    census: stats::CensusBook::new(),
});

// ---- own orbs (decision HB-OWN-ORBS, option 1; `model::hitbox_orbs`) -------------------------------------------------
//
// A fighter's spheres are the plugin's own orbs, never game effects: the pass keeps them in the same `HandleSet` through
// `OrbSink` (never refused; an orb ends at a status change, as the game's `CONCLUDE_STATUS` ended an effect), places each on
// its bone with the game's `joint_global_position_with_offset` (a read: the joint flag is false) and publishes them on
// `BOARD`. The layout tick (`draw_orbs`) projects them with the game's own `world_to_screen` and writes each orb's pill,
// two half-disc caps and the band between them; the panes' container is RootPane's first child, so the HUD, the pause menu and the plugin's menu draw over them as
// they drew over the effect spheres. Articles' hitboxes join them (`draw_articles`).

type PracticePictures = ([eggs::Look; 3], [eggs::Look; btt::LOOKS]);
type WorldAnchor = (FighterLife, [f32; 3]);

struct OverlaySample {
    board: Board,
    scene: Option<Scene>,
    pictures: PracticePictures,
    anchors: [Option<WorldAnchor>; hitbox::FIGHTER_SETS],
}

// All zero bytes, which keeps the board in `.bss`. No reader accepts a sample before the board's first frame.
// SAFETY: zero is a valid value of every field; the compiler rejects this initializer otherwise.
static BOARD: Mutex<OverlaySample> = Mutex::new(unsafe { core::mem::zeroed() });
static SAMPLE: AtomicU32 = AtomicU32::new(0);
static FIGHTERS: Mutex<[Option<(u32, usize)>; hitbox::FIGHTER_SETS]> =
    Mutex::new([None; hitbox::FIGHTER_SETS]);

/// The renderer can hold its last board while projecting. A fighter frame never waits on it.
fn clear_board(accessor: usize) {
    if let Some(mut sample) = BOARD.try_lock() {
        sample.board.clear(accessor);
    }
}

/// Clear set `i`; its orbs go too. Nothing is asked of the game.
fn clear_set(store: &mut Store, board: &mut Board, i: usize, accessor: usize) -> Counts {
    board.clear(accessor);
    store.table.sets[i].clear(&mut OrbSink { status: i32::MIN })
}

unsafe fn sphere_world_position(accessor: usize, sphere: &Sphere) -> Option<[f32; 3]> {
    natives::joint_position(accessor, sphere.joint, &sphere.pos)
}

/// Place every sphere `set` holds on its bone and publish them as `accessor`'s orbs, with the launch-angle marks while that row is On
/// and the fighter's wind `areas` (Windboxes ticked).
unsafe fn publish_orbs(board: &mut Board, accessor: usize, life: FighterLife, set: &hitbox::HandleSet, areas: &[legend::WindArea]) -> usize {
    let mut placed = [Orb::NONE; orbs::BOARD_ORBS];
    let lr = shown(legend::ANGLES).then(|| natives::lr(accessor));
    let place = |s: &Sphere| sphere_world_position(accessor, s);
    let mut len = orbs::place_orbs(set, place, lr, |slot| orbs::slot_angle(&mut GameAttacks(accessor), slot), &mut placed);
    let colour = legend::area_colour();
    for orb in areas.iter().enumerate().flat_map(|(k, area)| legend::area_orbs(area, k, 0.0, colour)).flatten() {
        if len < placed.len() {
            (placed[len], len) = (orb, len + 1);
        }
    }
    board.publish_fighter(accessor, life, placed[..len].iter().copied())
}

/// One raw game-memory read for `GameArea`, the wind-area reader: the pass (`frame`) runs it only behind `enabled` with Windboxes
/// ticked, through `object_wind_areas`.
fn area_load(at: usize, out: &mut [u8]) {
    // SAFETY: called only by `GameArea`, whose every address `legend::wind_areas` derived from a pointer it type-checked first
    // (non-zero, aligned) or from the live fighter's accessor; the read writes nothing but `out`.
    unsafe { std::ptr::copy_nonoverlapping(at as *const u8, out.as_mut_ptr(), out.len()) };
}

/// `legend::AreaReader` over game memory (`area_load`): `legend::wind_areas` type-checks every pointer before it reads through it.
struct GameArea;

impl legend::AreaReader for GameArea {
    fn area_usize(&mut self, at: usize) -> usize {
        let mut b = [0u8; 8];
        area_load(at, &mut b);
        u64::from_le_bytes(b) as usize
    }
    fn area_u8(&mut self, at: usize) -> u8 {
        let mut b = [0u8; 1];
        area_load(at, &mut b);
        b[0]
    }
    fn area_u32(&mut self, at: usize) -> u32 {
        let mut b = [0u8; 4];
        area_load(at, &mut b);
        u32::from_le_bytes(b)
    }
    fn area_f32(&mut self, at: usize) -> f32 {
        f32::from_bits(self.area_u32(at))
    }
}

/// An object's enabled gameplay wind areas (its AreaModule at accessor `+0xc0`, `legend::area`) with their angles from the game's wind
/// manager: a fighter's, an article's or an item's (every AreaModule writes the same table: `legend_every_area_module_writes_the_table_the_reader_reads`).
unsafe fn object_wind_areas(accessor: usize, out: &mut [legend::WindArea]) -> usize {
    use legend::AreaReader;
    let mut reader = GameArea;
    let module = reader.area_usize(accessor + legend::area::MODULE);
    let slot = reader.area_usize(crate::runtime::game::text_base() + legend::area::MANAGER_SLOT);
    let entries = if slot != 0 && slot.is_multiple_of(8) { reader.area_usize(slot) } else { 0 };
    legend::wind_areas(&mut reader, module, entries, out)
}

/// The orb panes of the layout last drawn into, resolved once per layout registration, and how many were shown.
struct OrbPanes {
    layout: usize,
    registrations: u32,
    container: usize,
    /// The caps (`0..CAPS`), then the bands.
    panes: [usize; crate::model::panes::ORBS],
    /// Caps and bands shown.
    shown: [usize; 2],
    /// The practice games' pictures (`panes::IDX_EGG`..) and how many of each pool (`panes::EGG_POOLS`) show.
    egg: ([usize; crate::model::panes::EGG_PANES], [usize; 3]),
}

static ORB_PANES: Mutex<OrbPanes> = Mutex::new(OrbPanes {
    layout: 0,
    registrations: 0,
    container: 0,
    panes: [0; crate::model::panes::ORBS],
    shown: [0; 2],
    egg: ([0; crate::model::panes::EGG_PANES], [0; 3]),
});

/// An unavailable waiting-room menu must not be interpreted as a scene departure.
fn overlay_layout_scene(name: &[u8]) -> Result<Option<Scene>, ()> {
    match name {
        b"info_training" => Ok(scene::current()),
        b"info_waiting" => {
            let requested = crate::runtime::lobby::training::lobby::try_requested().map_err(|_| ())?;
            Ok(requested.then_some(Scene::Lobby).filter(|_| scene::current() == Some(Scene::Lobby)))
        }
        _ => Ok(None),
    }
}

fn read_overlay_sample(scene: Scene) -> Option<hos_lock::MutexGuard<'static, OverlaySample>> {
    BOARD.try_lock().filter(|sample| sample.scene == Some(scene) && sample.board.is_frame(SAMPLE.load(Ordering::Acquire)))
}

fn hud_half() -> Option<[f32; 2]> {
    let words = unsafe { natives::screen_size_words()? };
    Some(core::array::from_fn(|i| orbs::screen_half(words[i], orbs::SCREEN_HALF_DEFAULT[i])))
}

unsafe fn project_to_canvas(at: [f32; 3], half: [f32; 2]) -> Option<[f32; 2]> {
    let canvas = orbs::to_canvas(natives::world_to_screen(&at)?, half);
    canvas.into_iter().all(f32::is_finite).then_some(canvas)
}

/// The current pose's fighter origin and its position on the HUD.
#[derive(Clone, Copy)]
pub struct FighterScreenAnchor {
    pub object_id: u32,
    pub world: [f32; 3],
    pub canvas: [f32; 2],
}

/// Read the same scene/pose as the orb renderer, even with its legend toggled off. A readiness boundary: no anchors
/// before boot installed the overlay.
///
/// # Safety
/// Call on the HUD layout thread while the named layout is current.
pub unsafe fn fighter_screen_anchors(name: &[u8]) -> Option<[Option<FighterScreenAnchor>; hitbox::FIGHTER_SETS]> {
    if !INSTALLED.load(Ordering::Acquire) {
        return None;
    }
    let scene = overlay_layout_scene(name).ok()??;
    let anchors = read_overlay_sample(scene)?.anchors;
    let half = hud_half()?;
    Some(anchors.map(|anchor| {
        let (life, world) = anchor?;
        let canvas = project_to_canvas(world, half)?;
        FIGHTER_GENERATIONS.is_current(life).then_some(FighterScreenAnchor { object_id: life.id, world, canvas })
    }))
}

/// The layout tick's draw (`ui::hud`): every live orb projected with the game's own `world_to_screen` (so the orbs follow
/// the camera, paused or not) and written into the plugin's panes as a pill; the rest hidden. A readiness boundary:
/// nothing before boot installed the feature. Writes only the plugin's own panes.
///
/// # Safety
/// `layout` is the live layout the tick passed; called on the UI job thread inside the tick.
pub unsafe fn draw_orbs(layout: *mut skyline::nn::ui2d::Layout, name: &[u8]) {
    use crate::model::panes as p;
    use crate::runtime::ui::{pane, tick};
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Ok(scene) = overlay_layout_scene(name) else { return };
    let active = scene.is_some();
    if layout.is_null() {
        return;
    }
    let _scope = HookScope::enter_hook(HookId::HitboxOrbs);
    RUN.lock().run.see(active, || view());
    let board = scene.and_then(read_overlay_sample);
    let (eggs_orbs, targets_in_play) = board.as_ref().map_or(([None; 3], [None; btt::LOOKS]), |s| s.pictures);
    // Resolve the editor handoff while this sample is held, so the first Running
    // sample and its updated target pictures become visible together.
    let editor = btt::editor_pictures(active, || view());
    let marks = editor.unwrap_or_else(|| if btt::running() { targets_in_play } else { [None; btt::LOOKS] });
    let mut own = [None; 3 + btt::LOOKS];
    own[..3].copy_from_slice(&eggs_orbs);
    let zone_wizard = super::egg_zone::wizard_active();
    let edge_wizard = super::edge_juggle::wizard_active();
    let tech = super::tech::running();
    let grader = super::input_display::grader_live();
    // The break volume supplies the geometry for both the hurt disc and the grade zones.
    let hurt_on = SETTING.load(Ordering::Acquire) && legend::shows(SHOWN.load(Ordering::Acquire), hitbox::KIND_HURT);
    let mut targets = [Orb::NONE; 1 + crate::model::btt::MAX];
    let mut n_targets = 0;
    if (hurt_on || grader) && !tech && !edge_wizard && !zone_wizard {
        n_targets = orbs::target_hurt_orbs(&own[..3], &mut targets);
        n_targets += orbs::target_hurt_orbs(&marks, &mut targets[n_targets..]);
    }
    let (marks, pointed) = if edge_wizard {
        super::edge_juggle::look(active)
    } else if tech {
        (super::tech::look(active), false)
    } else {
        (marks, false)
    };
    own[3..].copy_from_slice(&marks);
    if zone_wizard {
        own.fill(None);
    }
    let mut cache = ORB_PANES.lock();
    let registrations = tick::REGISTRATIONS.load(Ordering::Acquire);
    if !active {
        // Review HB-1: the waiting layout outlives its practice request (and Training's layout can tick outside
        // Training), and the orb container sits outside the plugin host `tick` hides. Hide what the last tick showed,
        // but only in the layout those panes belong to: another layout's cached panes may be gone.
        if cache.layout == layout as usize && cache.registrations == registrations && cache.container != 0 {
            let shown = cache.shown;
            write_tick(&mut cache, [orbs::pane_tick(0, shown[0]), orbs::pane_tick(0, shown[1])]);
            write_eggs(&mut cache, &[]);
        }
        return;
    }
    let on = SETTING.load(Ordering::Acquire) && !zone_wizard;
    let any = on || zone_wizard || own.iter().any(Option::is_some);
    if !any && cache.shown == [0; 2] && cache.egg.1 == [0; 3] && cache.layout == layout as usize {
        return;
    }
    if cache.layout != layout as usize || cache.registrations != registrations {
        let container = pane::find_child((*layout).root_pane, p::name_of(p::IDX_ORBS).as_bytes());
        cache.layout = layout as usize;
        cache.registrations = registrations;
        cache.container = container as usize;
        cache.shown = [p::CAPS, p::BANDS];
        let root = (*layout).root_pane;
        for k in 0..p::EGG_PANES {
            cache.egg.0[k] = pane::find_child(root, p::name_of(p::IDX_EGG + k).as_bytes()) as usize;
        }
        for k in 0..p::ORBS {
            cache.panes[k] = if container.is_null() { 0 } else { pane::find_child(container, p::name_of(p::IDX_ORB + k).as_bytes()) as usize };
        }
    }
    if cache.container == 0 {
        // The layout has no orb panes (the plugin's archive did not load): nothing is drawn; said once.
        static MISSING_SAID: AtomicBool = AtomicBool::new(false);
        if any && !MISSING_SAID.swap(true, Ordering::Relaxed) {
            crate::runtime::diag_log::diagnostic(format_args!("[Hitbox] orb panes missing in the layout: fighter spheres are not drawn"));
        }
        return;
    }
    let mut used = orbs::PaneUse::default();
    let (mut over, mut looks) = (0u32, [None; 3 + btt::LOOKS]);
    if any {
        let half = hud_half().unwrap_or_default();
        let plan = |orb: &Orb, color| {
            let end = |at| {
                let [x, y] = orbs::edge_points(at, orb.radius);
                Some((natives::world_to_screen(&at)?, [natives::world_to_screen(&x)?, natives::world_to_screen(&y)?]))
            };
            let ((a, ea), (b, eb)) = (end(orb.centre)?, end(orb.tail)?);
            orbs::plan_pill([a, b], [ea, eb], half, color)
        };
        if zone_wizard {
            if let Some(view) = view() {
                let samples = super::egg_zone::draw(layout as usize, view, |[a, b], width, color| {
                    let [dx, dy] = [b[0] - a[0], b[1] - a[1]];
                    let length = dx.hypot(dy);
                    if length > 0.0 && length.is_finite() {
                        if let Some(band) = used.take_band() {
                            let center = [a[0].midpoint(b[0]), a[1].midpoint(b[1])];
                            write_pane(cache.panes[p::CAPS + band], center, [length, width], dy.atan2(dx).to_degrees(), color);
                        }
                    }
                });
                for (slot, spot) in own.iter_mut().zip(samples) {
                    *slot = spot.map(|spot| (spot, p::EGG_WHOLE));
                }
            }
        }
        // The target and its break animations are pictures whatever Hitbox Visualization says (rc.14): each is planned at the
        // target's size in the texture's colours; its look scales and fades it (a break grows and fades).
        let mut dot = Orb::NONE;
        dot.radius = eggs::RADIUS;
        looks = own.map(|look| {
            let ([x, y], look) = look?;
            [dot.centre, dot.tail] = [[x, y, 0.0]; 2];
            Some((plan(&dot, [255; 4])?, look))
        });
        if let Some(pointer) = own[3]
            .filter(|_| pointed && !zone_wizard)
            .and_then(|([x, y], _)| project_to_canvas([x, y, 0.0], half))
            .and_then(|canvas| orbs::edge_pointer(canvas))
        {
            for (look, point) in looks[4..9].iter_mut().zip(pointer) {
                *look = Some((orbs::Pill {
                    caps: [point; 2], band: point, length: 0.0, size: 14.0, rot: 0.0, color: [255; 4],
                }, p::EGG_WHOLE));
            }
        }
        let hide = super::hide_cpu::draw_hide();
        let combat = board.as_ref().filter(|_| on).map(|sample| &sample.board);
        // Hitboxes and grabs take the panes first. Uncertain snapshots yield no fighter panes this tick.
        let bits = SHOWN.load(Ordering::Acquire);
        let drawn = orbs::targets_drawn(combat.is_some() || !on, hurt_on || grader, n_targets);
        let write_pill = |pill: orbs::Pill, used: &mut orbs::PaneUse, over: &mut u32, panes: &[usize; p::ORBS]| {
            let Some((cap, band)) = used.take(pill.length > 0.0) else {
                *over += 1;
                return;
            };
            for (k, at) in pill.caps.into_iter().enumerate() {
                unsafe { write_pane(panes[cap + k], at, [pill.size / 2.0, pill.size], pill.rot + 180.0 * k as f32, pill.color) };
            }
            if let Some(band) = band {
                unsafe { write_pane(panes[p::CAPS + band], pill.band, [pill.length, pill.size], pill.rot, pill.color) };
            }
        };
        for orb in combat
            .iter()
            .flat_map(|b| b.draw_slots_in_order(b.draw_slots(&hide, &FIGHTER_GENERATIONS)))
            .filter(|o| legend::shows(bits, o.kind()))
        {
            if orb.kind() == legend::KIND_AREA && orb.key & 0xff == legend::AREA_BOX {
                let Some((centre, size)) = orbs::plan_wind_box(orb, half, |corner| natives::world_to_screen(&corner)) else { continue };
                let Some(band) = used.take_band() else {
                    over += 1;
                    continue;
                };
                write_pane(cache.panes[p::CAPS + band], centre, size, 0.0, orb.color);
                continue;
            }
            let Some(pill) = plan(orb, orb.color) else { continue };
            write_pill(pill, &mut used, &mut over, &cache.panes);
        }
        for target in &targets[..drawn] {
            for layer in crate::model::spacing::target_layers(target, hurt_on, grader).into_iter().flatten() {
                let Some(pill) = plan(&layer, layer.color) else { continue };
                write_pill(pill, &mut used, &mut over, &cache.panes);
            }
        }
    }
    let n = used.caps / 2;
    let shown = cache.shown;
    write_tick(&mut cache, [orbs::pane_tick(used.caps, shown[0]), orbs::pane_tick(used.bands, shown[1])]);
    write_eggs(&mut cache, &looks);
    // Release builds say it too, once: more orbs than panes means a fighter's spheres were cut (review HB-2).
    static OVER_SAID: AtomicBool = AtomicBool::new(false);
    if over != 0 && !OVER_SAID.swap(true, Ordering::Relaxed) {
        crate::runtime::diag_log::diagnostic(format_args!(
            "[Hitbox] more orbs than panes: drawn={n} cut={over} (hurtboxes cut first; said once)"
        ));
    }
    #[cfg(feature = "diag")]
    stats::orbs_drawn(n as u32, over);
}

/// The camera this tick (`eggs::View`): two points of the fighters' plane projected on the canvas.
unsafe fn view() -> Option<eggs::View> {
    let half = hud_half()?;
    let canvas = |p| project_to_canvas(p, half);
    let (a, b) = (canvas([0.0; 3])?, canvas([10.0, 10.0, 0.0])?);
    let scale = [(b[0] - a[0]) / 10.0, (b[1] - a[1]) / 10.0];
    (scale[0] > 0.0 && scale[1] > 0.0).then(|| {
        let limit = natives::camera_limit_range();
        eggs::View(scale, a, eggs::range(limit), eggs::vertical(limit))
    })
}

/// Show one cap or band picture (`target`, 0 when the layout lacks it) at `at`, `size` (width, height), turned by `rot`.
unsafe fn write_pane(target: usize, at: [f32; 2], size: [f32; 2], rot: f32, color: [u8; 4]) {
    use crate::runtime::ui::pane;
    let target = target as *mut skyline::nn::ui2d::Pane;
    if !target.is_null() {
        pane::set_pos(target, at[0], at[1]);
        pane::set_width(target, size[0]);
        pane::set_height(target, size[1]);
        pane::set_rot_z(target, rot);
        pane::set_rect_color(target, color);
        pane::set_visible(target, true);
    }
}

/// Show this frame's practice-game pictures, each at its planned point pill (`orbs::plan_pill` with both ends on the
/// spot): centred on it, `size × look.scale` across, unturned, in the texture's own colours at pane alpha `look.alpha`,
/// on the next free pane of `look.picture`'s pool (a pool's extra pictures are not drawn). The panes that showed last
/// frame and not this one hide.
unsafe fn write_eggs(cache: &mut OrbPanes, eggs: &[Option<(orbs::Pill, EggLook)>]) {
    use crate::model::panes::EGG_POOLS;
    use crate::runtime::ui::pane;
    let mut used = [0; 3];
    for (pill, look) in eggs.iter().flatten().copied() {
        let pool = look.picture as usize;
        let (first, count) = EGG_POOLS[pool];
        let slot = first + used[pool];
        if used[pool] < count && cache.egg.0[slot] != 0 {
            let (target, size) = (cache.egg.0[slot], [pill.size * look.scale; 2]);
            write_pane(target, pill.band, size, 0.0, [255; 4]);
            pane::set_alpha(target as *mut _, look.alpha);
            used[pool] += 1;
        }
    }
    for (pool, (first, _)) in EGG_POOLS.into_iter().enumerate() {
        for k in first + used[pool]..first + cache.egg.1[pool] {
            pane::set_visible(cache.egg.0[k] as *mut _, false);
        }
    }
    cache.egg.1 = used;
}

/// Apply one tick's pane writes (`orbs::pane_tick`, the caps' and the bands'): hide the panes that stop showing, show or
/// hide the container, and keep how many show.
unsafe fn write_tick(cache: &mut OrbPanes, ticks: [Option<orbs::PaneTick>; 2]) {
    use crate::model::panes::{BANDS, CAPS};
    use crate::runtime::ui::pane;
    let mut container = None;
    for (pool, (base, size)) in [(0, CAPS), (CAPS, BANDS)].into_iter().enumerate() {
        let Some(tick) = ticks[pool] else { continue };
        for k in tick.hide.0..tick.hide.1.min(size) {
            let target = cache.panes[base + k] as *mut skyline::nn::ui2d::Pane;
            if !target.is_null() {
                pane::set_visible(target, false);
            }
        }
        container = Some(container == Some(true) || tick.container);
        cache.shown[pool] = tick.shown;
    }
    if let (Some(show), true) = (container, cache.container != 0) {
        pane::set_visible(cache.container as *mut skyline::nn::ui2d::Pane, show);
    }
}

/// Spheres held by every fighter together (the per-frame pass skips everything when this is 0 and the setting is Off).
static HELD: AtomicUsize = AtomicUsize::new(0);
/// Bumped when a save state was loaded: every fighter's set kills its spheres on its next pass.
static RESTORE_EPOCH: AtomicU32 = AtomicU32::new(0);

/// A save state finished loading (package 3's restore completion calls this; no game call here).
pub fn on_state_restored() {
    RESTORE_EPOCH.fetch_add(1, Ordering::AcqRel);
}

/// The game initialised this fighter object again (the reset hub's on-init wrapper calls this): its records are dropped without a
/// game call, since its effect module may hand out the same numbers again; the game ends the old spheres with their status.
pub fn forget_fighter(fighter: usize) {
    if fighter == 0 {
        return;
    }
    // SAFETY: a field read of the fighter object the game just initialised.
    let accessor = unsafe { natives::fighter_accessor(fighter) };
    if accessor == 0 {
        return;
    }
    // SAFETY: this accessor belongs to the fighter the game just initialized.
    let id = unsafe { (*(accessor as *const smash::app::BattleObjectModuleAccessor)).battle_object_id };
    FIGHTER_GENERATIONS.retire(id);
}

/// One raw game-memory read for `hitbox::GameHurt` (the per-frame pass's hurt reader, route B; the diag census uses the
/// same read). The reader type-checks every pointer it dereferences before it calls this: a null or misaligned address or
/// pointer is refused there (Eden answers 0 for an unmapped read, so a missing check would not show there).
fn mem_load(at: usize, out: &mut [u8]) {
    // SAFETY: the reader runs only inside the installed feature's pass for a live fighter, on the game thread, and only
    // while the setting is On (`frame` constructs it behind `enabled`); every address the reader passes was type-checked
    // first (`model::hitbox::GameHurt`), and the read writes nothing but `out`.
    unsafe { std::ptr::copy_nonoverlapping(at as *const u8, out.as_mut_ptr(), out.len()) };
}

pub(crate) struct GameAttacks(pub(crate) usize);

impl hitbox::AttackReader for GameAttacks {
    fn active(&mut self, slot: i32) -> bool {
        // SAFETY: the current fighter's pass, after the feature's closure passed.
        unsafe { natives::attack_is_attack(self.0, slot) }
    }

    fn data(&mut self, slot: i32) -> usize {
        // SAFETY: as above; only an active AttackModule slot is queried.
        unsafe { natives::attack_data(self.0, slot) }
    }

    fn motion(&mut self) -> f32 {
        // SAFETY: as above.
        unsafe { natives::motion_frame(self.0) }
    }

    fn read_u64(&mut self, at: usize) -> u64 {
        // SAFETY: the planner reads only the guarded AttackData node field.
        unsafe { (at as *const u64).read() }
    }

    fn read_f32(&mut self, at: usize) -> f32 {
        // SAFETY: the planner reads only the guarded AttackData shape fields.
        unsafe { (at as *const f32).read() }
    }
}

/// Articles tracked by animation commands and active items. The post-update overlay pass samples their world shapes.
static ARTICLES: Mutex<orbs::Articles> = Mutex::new(orbs::Articles::new());
/// An Off transition clears tracked articles even when no fighter orbs are held.
static CLEAR_ARTICLES: AtomicBool = AtomicBool::new(false);

/// Each tracked article's live attack slots as its own orbs, placed on its bones, and with *Windboxes* ticked its wind areas; an
/// article that is gone (its id inactive or now another object's), or whose attack, grab and wind ended, is forgotten.
/// `sv_battle_object` as `cpu_pilot` uses it.
unsafe fn draw_articles(
    board: &mut Board,
    articles: &mut orbs::Articles,
    owners: &[usize],
    #[cfg(feature = "diag")] article_signatures: &mut [(u32, u64); 8],
) -> u32 {
    articles.clear_evicted(board);
    let hide = super::hide_cpu::draw_hide();
    let wind = shown(legend::WINDBOXES);
    let hurt = shown(legend::ARTICLE_HURTBOXES) && matches!(scene::current(), Some(Scene::Training));
    let (mut dropped, mut budget) = (0, hitbox::ARTICLE_HURT_BATCH);
    let mut hurts = [Sphere::NONE; hitbox::ARTICLE_HURT_LIMIT];
    let mut areas = [legend::WindArea::NONE; legend::area::MAX_AREAS];
    // A thrown item (a bomb, a turnip) runs no `ATTACK`: an active item with a live attack slot, or with Windboxes ticked an
    // enabled wind area, is tracked as an article (`legend::tracks_item`). The list holds this frame's active items; a tracked one
    // is checked again on every later pass (below).
    let manager = state_natives::item_manager();
    let count = match manager {
        0 => 0,
        m => state_natives::item_active_count(m).min(16),
    };
    let items = (0..count).map(|i| state_natives::item_active(manager, i)).filter(|&item| item != 0).map(|item| state_natives::item_object_id(item));
    let instances = owners.iter().flat_map(|&owner| {
        hitbox::article_instances(hurt, move |kind, ordinal| state_natives::article_get_from_no(owner, kind, ordinal))
    }).map(|article| state_natives::article_object_id(article));
    // The lock-on helpers are never drawn: a target's hurtbox is its picture disc (`orbs::target_hurt_orbs`).
    for id in items.chain(instances).filter(|&id| state_natives::object_is_active(id) && !super::lockon::is_helper(id)) {
        let key = smash::app::sv_battle_object::module_accessor(id) as usize;
        if key == 0 {
            continue;
        }
        let live_slot = hitbox::live_slot(&mut GameAttacks(key)).is_some();
        let found = if wind && !live_slot { object_wind_areas(key, &mut areas) } else { 0 };
        if legend::tracks_item(live_slot, found) {
            #[cfg(feature = "diag")]
            stats::item(articles, id, key);
            let _ = articles.track(id, key, Some(board), true);
        }
        if hurt && !articles.tracked().contains(&(id, key)) {
            let (mut len, mut cap) = (0, hurts.len());
            let mut reader = hitbox::GameHurt { accessor: key, load: mem_load };
            let cut = hitbox::want_article_hurt(&mut reader, &mut hurts, &mut len, false, &mut cap);
            #[cfg(feature = "diag")]
            stats::article_hurt(id, key, len, cut);
            if len != 0 && articles.track(id, key, Some(board), false).is_none() { dropped += cut + len as u32; }
        }
    }
    for (id, key) in articles.tracked().into_iter().filter(|t| t.0 != 0) {
        let live = smash::app::sv_battle_object::is_active(id) && smash::app::sv_battle_object::module_accessor(id) as usize == key;
        // `key` is the live article's accessor (checked above); the natives check their own grants. Tracked: its set.
        let Some((set, attacking)) = articles.track(id, key, Some(board), false) else { continue };
        let place = |s: &Sphere| sphere_world_position(key, s);
        // Only a live article is read: its areas, its facing and its status (`legend::keeps_article` inside the pass).
        let kept = live && {
            let found = if wind { object_wind_areas(key, &mut areas) } else { 0 };
            #[cfg(feature = "diag")]
            stats::object(article_signatures, id, key, &areas[..found]);
            let mut len = 0;
            if hurt {
                let mut reader = hitbox::GameHurt { accessor: key, load: mem_load };
                let cut = hitbox::want_article_hurt(&mut reader, &mut hurts, &mut len, true, &mut budget);
                dropped += cut;
                #[cfg(feature = "diag")]
                stats::article_hurt(id, key, len, cut);
            }
            let extras = orbs::Extras {
                lr: shown(legend::ANGLES).then(|| natives::lr(key)),
                areas: &areas[..found],
                hurt: &hurts[..len],
                owner: hide.owner(|| article_owner(id, key)),
            };
            let kept = orbs::article_pass(board, set, (id, key), natives::status_kind(key), &mut GameAttacks(key), place, extras);
            *attacking = found != 0 || set.spheres().any(|s| s.key >> 24 != hitbox::KIND_ARTICLE_HURT);
            kept
        };
        if !kept {
            articles.forget(id, board);
        }
    }
    board.finish_articles(hide.generation());
    dropped
}

/// The combat owner of a validated live item or weapon, when it is still a live fighter.
pub(crate) unsafe fn article_owner(id: u32, key: usize) -> Option<u32> {
    if !matches!(id >> 28, 1 | 4) {
        return None;
    }
    let owner = natives::team_owner_id(key)?;
    (owner >> 28 == 0 && state_natives::object_is_active(owner)).then_some(owner)
}

/// *Frame data colours* (spots-3): what this fighter's hitboxes launch, by which they rank: its own damage multiplier (the smash charge) and
/// the other practice fighter as it is now (Player 1's hitboxes against the primary CPU, any other fighter's against Player 1); a standard
/// target (weight 100, 0%) when there is none.
unsafe fn fighter_launch(accessor: usize) -> crate::model::spots::Launch {
    let p1 = natives::module_accessor(0);
    let target = if p1 == Some(accessor) { natives::module_accessor(i32::from(crate::runtime::features::cpu::primary_entry_now())) } else { p1 };
    let mul = natives::attack_power_mul(accessor);
    let standard = crate::model::spots::Launch::STANDARD;
    match target.filter(|&t| t != accessor) {
        Some(t) => crate::model::spots::Launch { mul, percent: natives::damage(t, 0), weight: natives::weight(t), ..standard },
        None => crate::model::spots::Launch { mul, ..standard },
    }
}

// ---- the per-frame pass ---------------------------------------------------------------------------------------------

/// The early fighter pass retains the gameplay work. Only the live object identity crosses into the overlay sample.
///
/// # Safety
/// `fighter` is the fighter object the game just ran a frame of.
pub unsafe fn frame(fighter: usize) {
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let _scope = HookScope::enter_hook(HookId::HitboxFrame);
    super::egg_zone::frame(fighter);
    eggs_pass(fighter);
    let accessor = natives::fighter_accessor(fighter);
    if accessor == 0 {
        return;
    }
    let id = (*(accessor as *const smash::app::BattleObjectModuleAccessor)).battle_object_id;
    if let Some(mut fighters) = FIGHTERS.try_lock() {
        let object = Some((id, accessor));
        if !fighters.contains(&object) {
            if let Some(slot) = fighters.iter_mut().find(|slot| slot.is_none()) {
                *slot = object;
            }
        }
    }
    if !super::edge_juggle::may_request_player_height() {
        return;
    }
    if let Some(attempt) = super::edge_juggle::player_height_request(accessor) {
        let height = if SETTING.load(Ordering::Acquire) && shown(legend::HURTBOXES) {
            perfect_visualizer_height(attempt.player())
        } else {
            perfect_player_height(&attempt)
        };
        attempt.record_height(height);
    }
}

/// Keep the Perfect recovery measurement on its original fighter phase, including the visualizer's shape policy.
unsafe fn perfect_visualizer_height(player: usize) -> Option<f32> {
    if !visualizes(player) || !ensure_ready() {
        return None;
    }
    let status = natives::status_kind(player);
    let facts = FrameFacts { visualizes: true, enabled: true, status, ids: status_ids() };
    if facts.ids.shielding(status) {
        return None;
    }
    let mut wants = [Sphere::NONE; hitbox::WANT_SCRATCH];
    let mut len = 0;
    if facts.draws_attacks() {
        hitbox::want_hits(&mut GameAttacks(player), &mut wants, &mut len, true);
    }
    hitbox::want_hurt(&mut hitbox::GameHurt { accessor: player, load: mem_load }, &mut wants, &mut len, true, hitbox::KIND_HURT);
    drawn_player_height(player, &wants[..len])
}

/// The one game-thread sampling point for combat geometry and the practice discs.
/// The HUD projects the completed sample with the current camera.
unsafe fn sample_overlay() {
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(mut fighters) = FIGHTERS.try_lock() else { return };
    if fighters[0].is_none() {
        // A paused simulation has no new fighter pose to publish.
        return;
    }
    let queued = std::mem::take(&mut *fighters);
    drop(fighters);
    let sample_id = SAMPLE.fetch_add(1, Ordering::AcqRel).wrapping_add(1);
    let Some(mut sample) = BOARD.try_lock() else { return };
    let Some(mut store) = STORE.try_lock() else { return };
    let Some(mut articles) = ARTICLES.try_lock() else { return };
    store.table.forget_retired(&FIGHTER_GENERATIONS);
    let OverlaySample { board, scene: sampled_scene, pictures, anchors } = &mut *sample;
    board.begin_frame(sample_id);
    *sampled_scene = scene::current();
    let active = sampled_scene.is_some();
    anchors.fill(None);
    let enabled = active && SETTING.load(Ordering::Acquire);
    if CLEAR_ARTICLES.swap(false, Ordering::AcqRel) {
        articles.forget_all(Some(board));
    }
    let mut owners = [0; hitbox::FIGHTER_SETS];
    let mut owner_count = 0;
    for (slot, object) in queued.into_iter().enumerate() {
        let Some((id, accessor)) = object else { continue };
        let Some(life) = FIGHTER_GENERATIONS.stamp(id) else { continue };
        if !state_natives::object_is_active(id)
            || smash::app::sv_battle_object::module_accessor(id) as usize != accessor
        {
            continue;
        }
        let drawable = active && visualizes(accessor);
        if drawable {
            anchors[slot] = natives::joint_position(accessor, hash40("top"), &[0.0; 3])
                .filter(|point| point.iter().all(|x| x.is_finite()))
                .map(|world| (life, world));
        }
        let held = store.table.find_life(accessor, life).filter(|&i| !store.table.sets[i].is_empty());
        if !enabled && held.is_none() {
            continue;
        }
        if !drawable || !ensure_ready() {
            if let Some(i) = held {
                clear_set(&mut store, board, i, accessor);
            }
            continue;
        }
        if enabled {
            owners[owner_count] = accessor;
            owner_count += 1;
        }
        sample_fighter(&mut store, board, accessor, life, enabled);
    }
    if owner_count != 0 {
        let _ = draw_articles(
            board,
            &mut articles,
            &owners[..owner_count],
            #[cfg(feature = "diag")]
            &mut store.article_signatures,
        );
    }
    HELD.store(store.table.held_spheres(), Ordering::Relaxed);
    *pictures = if active {
        let eggs = RUN.try_lock().map_or([None; 3], |state| state.run.pictures());
        (eggs, btt::overlay_pictures())
    } else {
        ([None; 3], [None; btt::LOOKS])
    };
}

unsafe fn sample_fighter(store: &mut Store, board: &mut Board, accessor: usize, life: FighterLife, enabled: bool) {
    let ids = status_ids();
    let status = if enabled { natives::status_kind(accessor) } else { 0 };
    #[cfg(feature = "diag")]
    stats::frame_begin();
    let facts = FrameFacts { visualizes: true, enabled, status, ids };
    let i = store.table.index_for(accessor, life);
    let epoch = RESTORE_EPOCH.load(Ordering::Acquire);
    let mut counts = Counts::default();
    if store.seen_epoch[i] != epoch {
        // A save state was loaded since this fighter's last pass: its spheres go and come back from the restored state.
        store.seen_epoch[i] = epoch;
        counts = clear_set(store, board, i, accessor);
    }
    let Store { table, wants, .. } = store;
    let set = &mut table.sets[i];
    if enabled {
        set.launch = fighter_launch(accessor);
    }
    // The current pose is available to every shape through the same joint transform.
    let mut hurt = (enabled && shown(legend::HURTBOXES)).then(|| hitbox::GameHurt { accessor, load: mem_load });
    let hurt: Option<&mut dyn hitbox::HurtboxReader> = hurt.as_mut().map(|h| h as _);
    let (c, len) = hitbox::reconcile_frame(set, facts, wants, &mut GameAttacks(accessor), hurt, &mut OrbSink { status });
    // Wind areas (Windboxes ticked): pushes that are not hitboxes, read from the fighter's AreaModule.
    let mut areas = [legend::WindArea::NONE; legend::area::MAX_AREAS];
    let found = if enabled && shown(legend::WINDBOXES) { object_wind_areas(accessor, &mut areas) } else { 0 };
    let placed = publish_orbs(board, accessor, life, set, &areas[..found]);
    #[cfg(feature = "diag")]
    let (hits, hurts) = {
        let hits = wants[..len].iter().filter(|s| s.key >> 24 != hitbox::KIND_HURT).count() as u32;
        (hits, len as u32 - hits)
    };
    #[cfg(feature = "diag")]
    let grabs = set.spheres().filter(|s| s.key >> 24 == hitbox::KIND_GRAB).count() as u32;
    #[cfg(feature = "diag")]
    stats::wanted(hits, hurts);
    let held_now = set.len();
    counts.kept += c.kept;
    counts.spawned += c.spawned;
    counts.killed += c.killed;
    counts.ended += c.ended;
    counts.failed += c.failed;
    counts.dropped += c.dropped;
    #[cfg(feature = "diag")]
    stats::orbs_pass(i, placed as u32, [hits, hurts, grabs], status);
    #[cfg(not(feature = "diag"))]
    let _ = placed;
    // The diag hurt census (decision HB1): a second HitModule read per fighter per frame over the pass's own read,
    // for the census line's parts/first-pass/cost fields; no sphere is requested by the census itself, and the
    // shipped build has no census at all (the module notes).
    #[cfg(feature = "diag")]
    if enabled {
        stats::hurt_census(&mut store.census, accessor);
    }
    #[cfg(feature = "diag")]
    stats::frame_end(facts.draws_attacks(), counts, held_now as u32, HELD.load(Ordering::Relaxed) as u32);
    #[cfg(not(feature = "diag"))]
    let _ = (counts, held_now, len);
}

// ---- the hooks --------------------------------------------------------------------------------------------------------

/// The Lua stack value of argument `index` (1-based) of the command running on `lua`, read as the command code reads it.
unsafe fn lua_cell(lua: usize, index: usize) -> Cell {
    let frame = ((lua + layout::LUA_FRAME) as *const usize).read() as *const usize;
    if frame.is_null() {
        return Cell::NIL;
    }
    let base = frame.read();
    let top = ((lua + layout::LUA_TOP) as *const usize).read();
    hitbox::stack_cell(base, top, index, |slot| Cell { raw: (slot as *const u64).read(), tag: ((slot + layout::TAG_FIELD) as *const u32).read() })
}

/// Fighter attacks are read from active slots in the post-update pass. A grab is queued with its command status,
/// and an article is tracked by its battle-object id.
unsafe fn animation_command(lua: usize, indices: Indices, grab: bool) {
    let _scope = HookScope::enter_hook(if grab { HookId::HitboxCatch } else { HookId::HitboxAttack });
    if lua == 0 || !SETTING.load(Ordering::Acquire) || !ensure_ready() {
        return;
    }
    let accessor = natives::lua_accessor(lua);
    if accessor == 0 || !hitbox::draw_on_command(true, visualizes(accessor)) {
        return;
    }
    // A fighter's hitbox is not taken from the command: the fighter's pass reads its attack slots. A grab box waits in its
    // set with the status its command ran in, an article's (Olimar's Pikmin) in the article's (`draw_articles`).
    let fighter = natives::is_fighter(accessor);
    if fighter && !grab {
        return;
    }
    let id = (*(accessor as *const smash::app::BattleObjectModuleAccessor)).battle_object_id;
    let life = fighter.then(|| FIGHTER_GENERATIONS.stamp(id)).flatten();
    let Some(shape) = hitbox::parse(indices, |i| lua_cell(lua, i)) else { return };
    let mut wants = [Sphere::NONE; hitbox::SHAPE_SPHERES];
    let mut len = 0;
    let tint = hitbox::legend_colour(hitbox::KIND_GRAB, None, 0);
    hitbox::push_shape(&mut wants, &mut len, hitbox::KIND_GRAB, shape.id as usize, &shape, tint);
    let status = Some(natives::status_kind(accessor));
    let (mut store, mut articles);
    let set = if fighter {
        let Some(life) = life else { return };
        store = STORE.lock();
        if !FIGHTER_GENERATIONS.is_current(life) {
            return;
        }
        let i = store.table.index_for(accessor, life);
        &mut store.table.sets[i]
    } else {
        articles = ARTICLES.lock();
        articles.track(id, accessor, None, true).unwrap().0
    };
    let made = wants[..len].iter().filter(|w| grab && set.push_grab(**w, status)).count() as u32;
    #[cfg(feature = "diag")]
    stats::command(grab, fighter, shape.id, shape.joint, shape.size, shape.b.is_some(), made);
    #[cfg(not(feature = "diag"))]
    let _ = made;
}

/// `sv_animcmd::ATTACK`: the plugin part first, then the game's own command.
#[skyline::hook(offset = sites_hitbox::HITBOX_ATTACK_OFFSET)]
pub unsafe fn hitbox_attack_hook(lua_state: usize) {
    animation_command(lua_state, hitbox::ATTACK, false);
    call_original!(lua_state);
}

/// `sv_animcmd::CATCH`: the plugin part first, then the game's own command.
#[skyline::hook(offset = sites_hitbox::HITBOX_CATCH_OFFSET)]
pub unsafe fn hitbox_catch_hook(lua_state: usize) {
    animation_command(lua_state, hitbox::CATCH, true);
    call_original!(lua_state)
}

/// The plugin part of a wind-area command (rc.14 wind-articles): an article the practice visualizes that adds a wind area (Zelda's
/// Triforce of Wisdom, Hero's Bang and Kaboom, a Final Smash's black hole) is tracked by its battle-object id, as an ATTACK's is, so
/// `draw_articles` reads its own AreaModule. A fighter's areas are read by its own pass (`legend::wind_tracks`).
unsafe fn wind_command(lua: usize) {
    let _scope = HookScope::enter_hook(HookId::HitboxWind);
    let (enabled, windboxes) = (SETTING.load(Ordering::Acquire), shown(legend::WINDBOXES));
    if lua == 0 || !enabled || !windboxes || !ensure_ready() {
        return;
    }
    let accessor = natives::lua_accessor(lua);
    if accessor == 0 {
        return;
    }
    let fighter = natives::is_fighter(accessor);
    if !legend::wind_tracks(enabled, windboxes, fighter, !fighter && visualizes(accessor)) {
        return;
    }
    let id = (*(accessor as *const smash::app::BattleObjectModuleAccessor)).battle_object_id;
    let mut articles = ARTICLES.lock();
    articles.track(id, accessor, None, true).unwrap();
}

/// `sv_animcmd::AREA_WIND_2ND` (a box): the plugin part first, then the game's own command.
#[skyline::hook(offset = sites_hitbox::AREA_WIND_OFFSET)]
pub unsafe fn hitbox_area_wind_hook(lua_state: usize) {
    wind_command(lua_state);
    call_original!(lua_state)
}

/// `sv_animcmd::AREA_WIND_2ND_arg10` (a box): the plugin part first, then the game's own command.
#[skyline::hook(offset = sites_hitbox::AREA_WIND_ARG10_OFFSET)]
pub unsafe fn hitbox_area_wind_arg10_hook(lua_state: usize) {
    wind_command(lua_state);
    call_original!(lua_state)
}

/// `sv_animcmd::AREA_WIND_2ND_RAD` (a circle): the plugin part first, then the game's own command.
#[skyline::hook(offset = sites_hitbox::AREA_WIND_RAD_OFFSET)]
pub unsafe fn hitbox_area_wind_rad_hook(lua_state: usize) {
    wind_command(lua_state);
    call_original!(lua_state)
}

/// `sv_animcmd::AREA_WIND_2ND_RAD_arg9` (a circle): the plugin part first, then the game's own command.
#[skyline::hook(offset = sites_hitbox::AREA_WIND_RAD_ARG9_OFFSET)]
pub unsafe fn hitbox_area_wind_rad_arg9_hook(lua_state: usize) {
    wind_command(lua_state);
    call_original!(lua_state)
}

/// `GrabModule::set_rebound`: a grab ended when the rebound is cleared; the fighter's spheres go (by their own handles) unless it
/// shields, and its next pass brings back what still stands.
#[skyline::hook(offset = sites_hitbox::HITBOX_REBOUND_OFFSET)]
pub unsafe fn hitbox_rebound_hook(accessor: usize, rebound: bool) {
    call_original!(accessor, rebound);
    let _scope = HookScope::enter_hook(HookId::HitboxRebound);
    if rebound || accessor == 0 || !SETTING.load(Ordering::Acquire) || !ensure_ready() || !visualizes(accessor) {
        return;
    }
    let id = (*(accessor as *const smash::app::BattleObjectModuleAccessor)).battle_object_id;
    let Some(life) = FIGHTER_GENERATIONS.stamp(id) else { return };
    let shielding = status_ids().shielding(natives::status_kind(accessor));
    if hitbox::kill_on_rebound(rebound, true, true, shielding) {
        let mut store = STORE.lock();
        if let Some(i) = store.table.find_life(accessor, life).filter(|_| FIGHTER_GENERATIONS.is_current(life)) {
            clear_board(accessor);
            let c = store.table.sets[i].clear(&mut OrbSink { status: i32::MIN });
            HELD.store(store.table.held_spheres(), Ordering::Relaxed);
            #[cfg(feature = "diag")]
            stats::rebound_cleared(c.killed);
            #[cfg(not(feature = "diag"))]
            let _ = c;
        }
    }
}

/// BattleObjectManager's last simulation phase updates the live object models before sampling.
#[skyline::hook(offset = sites_hitbox::OVERLAY_PROCESS_END_OFFSET)]
unsafe fn overlay_process_end_hook(context: usize) {
    call_original!(context);
    let _scope = HookScope::enter_hook(HookId::HitboxFrame);
    sample_overlay();
}

/// Number of rows of `sites_hitbox`.
pub const N: usize = sites_hitbox::SITES.len();

/// Compare every row of `sites_hitbox` (before anything is written: `set_rebound`'s window holds its neighbours' thunks, so no
/// write may precede the comparison).
pub fn check(poisoned: &dyn Fn(&str) -> bool) -> [GuardOutcome; N] {
    std::array::from_fn(|i| {
        let site = &sites_hitbox::SITES[i];
        if poisoned(site.name) {
            GuardOutcome::Foreign { first_diff: 0 }
        } else {
            site.outcome(&game::read_window(site.offset))
        }
    })
}

/// Install the overlay hooks after its full closure and the CPU frame wrapper pass their guards.
pub fn install(closure: bool) -> bool {
    if closure {
        skyline::install_hook!(hitbox_attack_hook);
        skyline::install_hook!(hitbox_catch_hook);
        skyline::install_hook!(hitbox_rebound_hook);
        skyline::install_hook!(hitbox_area_wind_hook);
        skyline::install_hook!(hitbox_area_wind_arg10_hook);
        skyline::install_hook!(hitbox_area_wind_rad_hook);
        skyline::install_hook!(hitbox_area_wind_rad_arg9_hook);
        skyline::install_hook!(overlay_process_end_hook);
    }
    set_installed(closure);
    closure
}

/// Record the visualizer and the separately installed spacing hook in the same install line.
pub fn reports(found: &[GuardOutcome; N], visualizer: bool, spacing: bool) -> [SiteReport; N] {
    std::array::from_fn(|i| {
        let site = &sites_hitbox::SITES[i];
        SiteReport { site: site.name, found: found[i], written: sites_hitbox::reported_write(site, found[i], visualizer, spacing) }
    })
}

// ---- diag: the cost of the pass and the plugin's share of the effect pool -------------------------------------------------

/// Counters and one line per 600 passes (`[Hitbox] stats ...`), so the Eden rows read the cost per frame, the spheres the plugin
/// holds (its share of the game's effect pool), the requests the game refused, and what changed; the shipped build compiles all of
/// it away.
#[cfg(feature = "diag")]
mod stats {
    use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

    use crate::model::hitbox::{self, Counts};
    use crate::runtime::diag_log::diagnostic;

    static PASSES: AtomicU32 = AtomicU32::new(0);
    static WORK: AtomicU32 = AtomicU32::new(0);
    static WANT_HIT: AtomicU32 = AtomicU32::new(0);
    static WANT_HURT: AtomicU32 = AtomicU32::new(0);
    static KEPT: AtomicU32 = AtomicU32::new(0);
    static SPAWNED: AtomicU32 = AtomicU32::new(0);
    static KILLED: AtomicU32 = AtomicU32::new(0);
    static ENDED: AtomicU32 = AtomicU32::new(0);
    static FAILED: AtomicU32 = AtomicU32::new(0);
    static DROPPED: AtomicU32 = AtomicU32::new(0);
    static COMMANDS: AtomicU32 = AtomicU32::new(0);
    static MAX_HELD: AtomicU32 = AtomicU32::new(0);
    static TICKS: AtomicU64 = AtomicU64::new(0);
    static MAX_TICKS: AtomicU64 = AtomicU64::new(0);
    static START: AtomicU64 = AtomicU64::new(0);
    static LAST: AtomicU64 = AtomicU64::new(u64::MAX);
    static LAST_ARTICLE_HURT: [AtomicU64; 16] = [const { AtomicU64::new(u64::MAX) }; 16];
    const REPORT_EVERY: u32 = 600;

    fn now() -> u64 {
        // SAFETY: `GetSystemTick` has no preconditions.
        unsafe { skyline::nn::os::GetSystemTick() as u64 }
    }

    pub fn frame_begin() {
        START.store(now(), Ordering::Relaxed);
    }

    /// Native group evidence, including live objects with no enabled part; no additional read in release.
    pub fn article_hurt(id: u32, key: usize, drawn: usize, cut: u32) {
        use hitbox::HurtboxReader;
        let sig = (u64::from(id) << 32) | ((drawn as u64) << 16) | u64::from(cut.min(0xffff));
        if LAST_ARTICLE_HURT[(id >> 16) as usize % 16].swap(sig, Ordering::Relaxed) == sig { return; }
        let mut reader = hitbox::GameHurt { accessor: key, load: super::mem_load };
        let module = reader.module();
        let groups = if module == 0 { 0 } else { reader.read_usize(module + hitbox::layout::HIT_GROUPS) };
        let count = if groups == 0 { 0 } else { reader.read_i16(groups + hitbox::layout::HIT_GROUP_COUNT) };
        let enabled = hitbox::hurt_parts(&mut reader, false, |_, _, _| {});
        let kind = unsafe { super::natives::fighter_kind(key) };
        diagnostic(format_args!("[Hitbox] article hurt id={id:#x} kind={kind:?} accessor={key:#x} group=0 parts={count} enabled={enabled} endpoints={drawn} cut={cut}"));
    }

    pub fn wanted(hits: u32, hurts: u32) {
        WANT_HIT.fetch_add(hits, Ordering::Relaxed);
        WANT_HURT.fetch_add(hurts, Ordering::Relaxed);
    }

    pub fn frame_end(redraw: bool, c: Counts, held: u32, held_total: u32) {
        let ticks = now().wrapping_sub(START.load(Ordering::Relaxed));
        TICKS.fetch_add(ticks, Ordering::Relaxed);
        MAX_TICKS.fetch_max(ticks, Ordering::Relaxed);
        WORK.fetch_add(1, Ordering::Relaxed);
        KEPT.fetch_add(c.kept, Ordering::Relaxed);
        SPAWNED.fetch_add(c.spawned, Ordering::Relaxed);
        KILLED.fetch_add(c.killed, Ordering::Relaxed);
        ENDED.fetch_add(c.ended, Ordering::Relaxed);
        FAILED.fetch_add(c.failed, Ordering::Relaxed);
        DROPPED.fetch_add(c.dropped, Ordering::Relaxed);
        MAX_HELD.fetch_max(held_total, Ordering::Relaxed);
        // One line whenever this fighter's set changed (a sphere was requested, killed, ended or refused).
        let change = (u64::from(held) << 32) | u64::from(c.spawned + c.killed + c.ended + c.failed);
        if (c.spawned | c.killed | c.ended | c.failed) != 0 && LAST.swap(change, Ordering::Relaxed) != change {
            diagnostic(format_args!(
                "[Hitbox] pass held={held} held_total={held_total} kept={} spawned={} killed={} ended={} failed={} dropped={} sleep=false redraw={redraw}",
                c.kept, c.spawned, c.killed, c.ended, c.failed, c.dropped
            ));
        }
        let passes = PASSES.fetch_add(1, Ordering::Relaxed) + 1;
        if passes >= REPORT_EVERY {
            let take = |c: &AtomicU32| c.swap(0, Ordering::Relaxed);
            let passes = take(&PASSES).max(1);
            diagnostic(format_args!(
                "[Hitbox] stats passes={passes} with_work={} want_hit={} want_hurt={} kept={} spawned={} killed={} ended={} failed={} dropped={} held_max={} held_now={held_total} sleeps=0 restores=0 commands={} pass_ticks_avg={} pass_ticks_max={} heap_allocations={}",
                take(&WORK),
                take(&WANT_HIT),
                take(&WANT_HURT),
                take(&KEPT),
                take(&SPAWNED),
                take(&KILLED),
                take(&ENDED),
                take(&FAILED),
                take(&DROPPED),
                take(&MAX_HELD),
                take(&COMMANDS),
                TICKS.swap(0, Ordering::Relaxed) / u64::from(passes),
                MAX_TICKS.swap(0, Ordering::Relaxed),
                crate::runtime::alloc::hook_allocations()
            ));
        }
    }

    pub fn command(grab: bool, fighter: bool, id: i32, joint: u64, size: f32, extended: bool, made: u32) {
        COMMANDS.fetch_add(1, Ordering::Relaxed);
        diagnostic(format_args!(
            "[Hitbox] command kind={} fighter={fighter} id={id} joint={joint:#x} size={size:.2} extended={extended} spheres={made}",
            if grab { "catch" } else { "attack" }
        ));
    }

    /// An item about to be tracked: its first live attack slot and that hitbox's size (once per item).
    pub fn item(articles: &super::orbs::Articles, id: u32, key: usize) {
        if articles.tracked().contains(&(id, key)) {
            return;
        }
        let mut g = super::GameAttacks(key);
        let slot = hitbox::live_slot(&mut g).unwrap_or(0);
        let at = hitbox::AttackReader::data(&mut g, slot) + hitbox::layout::SIZE;
        let size = hitbox::AttackReader::read_f32(&mut g, at);
        diagnostic(format_args!("[Hitbox] item tracked id={id:#x} slot={slot} size={size:.2}"));
    }

    /// rc.14 wind-articles probe (diag only): a tracked article's or item's live attack slots (the no-reaction byte, the knockback
    /// words, the damage) and its wind areas (shape, angle), one group of `[Legend] object` lines whenever they change. The Eden
    /// settle rows read them (Sheik's Burst Grenade, R.O.B.'s Gyro, items).
    pub(super) fn object(last: &mut [(u32, u64); 8], id: u32, key: usize, areas: &[crate::model::legend::WindArea]) {
        use crate::model::legend::{self, AreaShape};
        use hitbox::AttackReader;
        let fnv = |h: u64, v: u64| (h ^ v).wrapping_mul(0x0000_0100_0000_01b3);
        let mut g = super::GameAttacks(key);
        let mut slots = [(0i32, 0u8, [0i32; 3], 0f32); 8];
        let mut live = 0;
        let mut sig = fnv(0xcbf2_9ce4_8422_2325, u64::from(id));
        for slot in 0..hitbox::SLOTS {
            let data = if g.active(slot) { g.data(slot) } else { 0 };
            if data == 0 || live == slots.len() {
                continue;
            }
            let nr = g.read_u64(data + legend::NO_REACTION) as u8;
            let [a, b] = legend::KNOCKBACK_QUADS.map(|at| g.read_u64(data + at));
            let kb = [(a >> 32) as i32, b as i32, (b >> 32) as i32];
            let dmg = g.read_f32(data + hitbox::layout::POWER);
            slots[live] = (slot, nr, kb, dmg);
            live += 1;
            sig = [u64::from(slot as u32), u64::from(nr), a, b].into_iter().fold(sig, fnv);
        }
        for area in areas {
            let words = match area.shape {
                AreaShape::Box { min, max } => [min[0], min[1], max[0], max[1]],
                AreaShape::Circle { centre, radius } => [centre[0], centre[1], radius, 0.0],
            };
            sig = words.iter().chain(area.angle.iter()).fold(sig, |h, v| fnv(h, (*v as i32) as u32 as u64));
        }
        let i = last.iter().position(|e| e.0 == id).or_else(|| last.iter().position(|e| e.0 == 0)).unwrap_or(id as usize % 8);
        if last[i] == (id, sig) {
            return;
        }
        last[i] = (id, sig);
        // SAFETY: `key` is the live tracked object's accessor (draw_articles checked it this pass).
        let status = unsafe { super::natives::status_kind(key) };
        diagnostic(format_args!("[Legend] object id={id:#x} status={status} slots={live} areas={}", areas.len()));
        for (slot, nr, [g, f, b], dmg) in &slots[..live] {
            diagnostic(format_args!("[Legend] object id={id:#x} slot={slot} nr={nr} kb={g}/{f}/{b} dmg={dmg:.2}"));
        }
        for (k, area) in areas.iter().enumerate() {
            match area.shape {
                AreaShape::Box { min, max } => diagnostic(format_args!(
                    "[Legend] object id={id:#x} area={k} box=({:.1},{:.1})-({:.1},{:.1}) angle={:?}",
                    min[0], min[1], max[0], max[1], area.angle
                )),
                AreaShape::Circle { centre, radius } => diagnostic(format_args!(
                    "[Legend] object id={id:#x} area={k} circle=({:.1},{:.1}) r={radius:.1} angle={:?}",
                    centre[0], centre[1], area.angle
                )),
            }
        }
    }

    pub fn rebound_cleared(killed: u32) {
        diagnostic(format_args!("[Hitbox] rebound cleared: killed={killed}"));
    }

    // ---- own orbs (decision HB-OWN-ORBS) ------------------------------------------------------------------------------

    static PASS_NO: AtomicU32 = AtomicU32::new(0);
    static LAST_ORB_PASS: [AtomicU64; hitbox::FIGHTER_SETS] = [const { AtomicU64::new(u64::MAX) }; hitbox::FIGHTER_SETS];
    static LAST_DRAWN: AtomicU64 = AtomicU64::new(u64::MAX);

    /// One fighter's (set `set`) pass: a line whenever its placed orbs, its wanted hit / hurt spheres, its held grab spheres
    /// or its status change.
    pub fn orbs_pass(set: usize, placed: u32, kinds: [u32; 3], status: i32) {
        let pass = PASS_NO.fetch_add(1, Ordering::Relaxed) + 1;
        let [hits, hurts, grabs] = kinds;
        let key = [u64::from(placed), u64::from(hits), u64::from(hurts), u64::from(grabs), u64::from(status as u32)]
            .iter()
            .fold(0xcbf2_9ce4_8422_2325u64, |h, v| (h ^ v).wrapping_mul(0x0000_0100_0000_01b3));
        if LAST_ORB_PASS[set % hitbox::FIGHTER_SETS].swap(key, Ordering::Relaxed) != key {
            diagnostic(format_args!(
                "[Hitbox] orbs pass set={set} placed={placed} hits={hits} hurts={hurts} grabs={grabs} status={status} pass={pass}"
            ));
        }
    }

    /// One layout tick's draw: a line whenever the number of orb panes shown changes, with the pass count it was drawn at.
    pub fn orbs_drawn(n: u32, over: u32) {
        let pass = PASS_NO.load(Ordering::Relaxed);
        let key = (u64::from(n) << 32) | u64::from(over);
        if LAST_DRAWN.swap(key, Ordering::Relaxed) != key {
            diagnostic(format_args!("[Hitbox] orbs drawn n={n} over={over} pass={pass}"));
        }
    }

    // ---- the hurt census (decision HB1) -------------------------------------------------------------------------------

    /// Census kinds tracked at once (eight distinct fighter kinds in a Training boot; a ninth takes the least
    /// recently used slot).
    const CENSUS_KINDS: usize = 8;

    /// One slot per fighter kind: the kind's open census window and its LRU clock.
    /// The sampler owns this state for the duration of the phase-8 pass.
    pub(super) struct CensusBook {
        kinds: [i32; CENSUS_KINDS],
        used: [u64; CENSUS_KINDS],
        clock: u64,
        states: [hitbox::HurtCensus; CENSUS_KINDS],
    }

    impl CensusBook {
        pub(super) const fn new() -> CensusBook {
            CensusBook { kinds: [i32::MIN; CENSUS_KINDS], used: [0; CENSUS_KINDS], clock: 0, states: [hitbox::HurtCensus::EMPTY; CENSUS_KINDS] }
        }

        fn slot_for(&mut self, kind: i32) -> &mut hitbox::HurtCensus {
            self.clock += 1;
            let i = match self.kinds.iter().position(|k| *k == kind) {
                Some(i) => i,
                None => {
                    let i = (0..CENSUS_KINDS).min_by_key(|&i| (self.kinds[i] != i32::MIN, self.used[i])).unwrap_or(0);
                    self.kinds[i] = kind;
                    self.states[i] = hitbox::HurtCensus::EMPTY;
                    i
                }
            };
            self.used[i] = self.clock;
            &mut self.states[i]
        }
    }

    /// The census read of one fighter's pass: the enabled parts and the spheres they want, the read's cost in
    /// ticks, one bounded line per fighter kind per `REPORT_EVERY` frames. No sphere is requested, nothing drawn.
    pub(super) fn hurt_census(book: &mut CensusBook, accessor: usize) {
        // SAFETY: the current fighter's pass, after the feature's closure passed; the native checks its own grant.
        let kind = unsafe { crate::runtime::game::natives::fighter_kind(accessor) }.unwrap_or(-1);
        let window = {
            let slot = book.slot_for(kind);
            let mut reader = hitbox::GameHurt { accessor, load: super::mem_load };
            let start = now();
            let mut spheres = 0usize;
            let parts = hitbox::hurt_parts(&mut reader, false, |_, shape, _| spheres += 1 + usize::from(shape.b.is_some()));
            slot.observe(parts, spheres, now().wrapping_sub(start));
            slot.window(REPORT_EVERY)
        };
        if let Some(window) = window {
            diagnostic(format_args!("[Hitbox] hurt census kind={kind} {window}"));
        }
    }

}

pub(crate) fn egg_rule() -> u8 { EGG_LIVE.load(Ordering::Acquire).to_le_bytes()[1] }
