//! The hitbox visualizer, the runtime half (SPEC-core-06; the rules are `model::hitbox`).
//!
//! With the `hitbox_vis` setting On, every active hitbox (by id colour) and every grab box
//! of a fighter in Training is drawn as translucent spheres (the game's `sys_shield` effect) on its bone. The fighter's other
//! animation-command effects are put to sleep meanwhile, except in the grab, tread and knockdown statuses.
//!
//! The spheres are kept, not respawned (lead ruling 2026-11-02 on Q3, after Eden hb-01 ran the effect pool dry): each fighter's
//! pass works out the spheres it wants and `model::hitbox::HandleSet` requests the new ones, keeps the unchanged ones (they follow
//! their bones by themselves) and kills the ones that changed or went away by their own handles. Nothing is ever killed by kind,
//! so the shield bubble and the game's other `sys_shield` effects are never touched.
//!
//! Three hooks (`sites_hitbox`: the `ATTACK` and `CATCH` animation commands and `GrabModule::set_rebound`) and one per-frame
//! pass, `frame`, which the CPU core's on-frame wrapper calls for every fighter after the game's own frame (SPEC-core-01 §5.2:
//! after combo, before the save states). The pass is a readiness boundary of the capability model (`INSTALLED`): until boot
//! says the feature's whole closure passed its guards it makes no call and reads no native. Every native goes through
//! `game::natives`, whose grant check refuses a row boot rejected.
//!
//! HB1 route B (operator decision 2026-11-04 ~06:5xZ, `docs/DECISIONS.md` HB1-route, `review/menu-proposal`
//! commit `5ca5e3b`; it reverses the 2026-11-03 option A default): the hurt spheres are drawn the fork's way, as yellow
//! `sys_shield` spheres from the game's shared effect pool, through the same handle sets as the hit spheres
//! (`docs/HB1-RENDER-ROUTES.md (removed after v0.2.0-rc.1; see that tag)`). While the setting is On the pass constructs the `HitModule` reader (`GameHurt`, over the
//! three relied `hurtbox_*` guard rows back in `sites_hitbox`) and plans the enabled parts' spheres into the same wants;
//! the pool's refusals are counted (`Counts::failed`, the `[Hitbox] stats` line) and each refused sphere is asked again on
//! a later pass, once the pool has room — one request per want per pass, never a respawn loop (the lead ruling above).
//! Off — and the stop paths the dispatch takes first (not a practice fighter, shielding, grab) — constructs no reader, so
//! the shipped build reads the HitModule only while the setting is On. The diag build additionally runs the reader once per
//! fighter per frame as the hurt census (`[Hitbox] hurt census`: the parts, the first pass and the read cost).
//! No allocation: the spheres and the handle sets are fixed storage, the Lua stack and attack data are read in place, and
//! every entry point counts under a `HookScope`. The pass and the hooks run on the game thread.
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};

use hos_lock::Mutex;

use crate::model::cpu::hash::{hash40, lua_bind_hash};
use crate::model::guard::GuardOutcome;
use crate::model::hitbox::{self, layout, Cell, Counts, Effects, FrameFacts, Indices, SetTable, Shape, SleepBook, Sphere, StatusIds};
use crate::model::hitbox_orbs::{self as orbs, Board, Orb, OrbSink, Route, View};
use crate::model::install_line::{SiteReport, Written};
use crate::model::scene::Scene;
use crate::model::settings::Snapshot;
use crate::model::sites_hitbox;
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::game::{self, natives};
use crate::runtime::scene;

/// Boot: the feature's closure (its rows, the Training predicate and the rows of the natives it shares) passed its guards and its
/// three hooks are installed. The per-frame pass does nothing before.
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// `hitbox_vis` On.
static SETTING: AtomicBool = AtomicBool::new(false);
/// The game's status ids and effect attributes were resolved through its constant table.
static READY: AtomicBool = AtomicBool::new(false);
static IDS: [AtomicI32; 6] = [const { AtomicI32::new(0) }; 6];
static ATTRIBUTE: AtomicU32 = AtomicU32::new(0);
/// The work id of a fighter's entry (`hitbox::ENTRY_ID_NAME`), for the lobby gate; resolved with the status ids.
static ENTRY_WORK: AtomicI32 = AtomicI32::new(0);
static ENTRY_WORK_READY: AtomicBool = AtomicBool::new(false);
/// Calls of `ensure_ready` since boot: the table is asked again every 30th call until it answers.
static TRIES: AtomicU32 = AtomicU32::new(0);
/// The `Hash40` of `sys_shield`.
static EFFECT: AtomicU64 = AtomicU64::new(0);
/// Fighters whose effects the plugin put to sleep (by accessor address), and how many.
static BOOK: Mutex<SleepBook> = Mutex::new(SleepBook::new());
static SLEPT: AtomicUsize = AtomicUsize::new(0);

/// Boot: the feature installed (or not).
pub fn set_installed(on: bool) {
    EFFECT.store(hash40(hitbox::EFFECT), Ordering::Release);
    INSTALLED.store(on, Ordering::Release);
}

/// Every settings publish.
pub fn publish(snapshot: &Snapshot) {
    SETTING.store(hitbox::enabled(snapshot), Ordering::Release);
}

/// The status ids and effect attributes, resolved once through the game's constant table (`None` until it answers; it may not be
/// built at boot). Asked again every 30th call.
fn ensure_ready() -> bool {
    if READY.load(Ordering::Acquire) {
        return true;
    }
    if TRIES.fetch_add(1, Ordering::Relaxed) % 30 != 0 {
        return false;
    }
    let mut ids = [0i32; 6];
    for (slot, name) in ids.iter_mut().zip(hitbox::STATUS_NAMES) {
        // SAFETY: the constant lookup is a guarded called site (`lua_const` checks its grant).
        let Some(value) = (unsafe { game::lua_const(lua_bind_hash(name)) }) else { return false };
        *slot = value;
    }
    let mut attribute = 0u32;
    for name in hitbox::ATTRIBUTE_NAMES {
        // SAFETY: as above.
        let Some(value) = (unsafe { game::lua_const(lua_bind_hash(name)) }) else { return false };
        attribute |= value as u32;
    }
    // SAFETY: as above.
    let Some(entry_work) = (unsafe { game::lua_const(lua_bind_hash(hitbox::ENTRY_ID_NAME)) }) else { return false };
    ENTRY_WORK.store(entry_work, Ordering::Release);
    ENTRY_WORK_READY.store(true, Ordering::Release);
    for (slot, value) in IDS.iter().zip(ids) {
        slot.store(value, Ordering::Release);
    }
    ATTRIBUTE.store(attribute, Ordering::Release);
    READY.store(true, Ordering::Release);
    true
}

fn status_ids() -> StatusIds {
    StatusIds::from_values(std::array::from_fn(|i| IDS[i].load(Ordering::Acquire)))
}

/// Whether the practice visualizes this object (`hitbox::visualizes_entry`): native Training, every object; the waiting room's
/// practice, only its practice fighters (Player 1 and package 2's lobby CPU entry, read from the fighter's work), never an article
/// there; every other scene (online, a hot swap) never.
unsafe fn visualizes(accessor: usize) -> bool {
    let scene = scene::current();
    let entry = match scene {
        Some(Scene::Lobby) if ENTRY_WORK_READY.load(Ordering::Acquire) && natives::is_fighter(accessor) => {
            Some(natives::get_int(accessor, ENTRY_WORK.load(Ordering::Acquire)))
        }
        _ => None,
    };
    hitbox::visualizes_entry(scene, entry)
}

fn book_set(accessor: usize, slept: bool) {
    let mut book = BOOK.lock();
    let before = book.slept(accessor);
    book.set(accessor, slept);
    match (before, slept) {
        (false, true) => {
            SLEPT.fetch_add(1, Ordering::Relaxed);
        }
        (true, false) => {
            SLEPT.fetch_sub(1, Ordering::Relaxed);
        }
        _ => {}
    }
}


// ---- the spheres the plugin keeps (lead ruling 2026-11-02 on Q3) ------------------------------------------------------

/// Every fighter's held spheres, the scratch list of the spheres a pass wants, the save-state epoch each set last saw, the
/// route each set was filled under, the A/B view epoch each set last applied, and the pass count each set was last seen at.
struct Store {
    table: SetTable,
    seen_epoch: [u32; hitbox::FIGHTER_SETS],
    wants: [Sphere; hitbox::WANT_SCRATCH],
    routes: [u8; hitbox::FIGHTER_SETS],
    view_applied: [u32; hitbox::FIGHTER_SETS],
    seen_pass: [u32; hitbox::FIGHTER_SETS],
    passes: u32,
}

static STORE: Mutex<Store> = Mutex::new(Store {
    table: SetTable::new(),
    seen_epoch: [0; hitbox::FIGHTER_SETS],
    wants: [Sphere::NONE; hitbox::WANT_SCRATCH],
    routes: [0; hitbox::FIGHTER_SETS],
    view_applied: [0; hitbox::FIGHTER_SETS],
    seen_pass: [0; hitbox::FIGHTER_SETS],
    passes: 0,
});

// ---- own orbs (HB1 route A4, diag prototype; `model::hitbox_orbs`) ----------------------------------------------------
//
// The route is route B unless the diag build's request file (`diag-hitbox-route.req`, `runtime::diag_reload`) says
// otherwise, so a release build behaves as before. With `Route::Orbs` no effect is requested: each held sphere becomes an
// orb placed on its bone by the game's `joint_global_position_with_offset`, published on `BOARD`, and drawn by the layout
// tick (`draw_orbs`) as one of the plugin's own disc panes. `Route::Both` keeps route B's effects and computes the orbs
// beside them; `View` picks the layer shown, which flips on the identical frame while the game is paused (`view_poll`).

static ROUTE: AtomicU8 = AtomicU8::new(0);
static VIEW: AtomicU8 = AtomicU8::new(2);
/// The last argument of the joint position call (`joint=0|1`).
static JOINT_FLAG: AtomicBool = AtomicBool::new(false);
/// Bumped by every request that changes the view: the sets re-apply route B's visibility.
static VIEW_EPOCH: AtomicU32 = AtomicU32::new(1);
static POLL_EPOCH: AtomicU32 = AtomicU32::new(1);
static BOARD: Mutex<Board> = Mutex::new(Board::new());

fn route() -> Route {
    Route::from_code(ROUTE.load(Ordering::Acquire))
}

fn view() -> View {
    View::from_code(VIEW.load(Ordering::Acquire))
}

/// Diag only: a request from `diag-hitbox-route.req`.
#[cfg(feature = "diag")]
pub fn set_request(request: orbs::Request) {
    if let Some(route) = request.route {
        ROUTE.store(route.code(), Ordering::Release);
    }
    if let Some(view) = request.view {
        VIEW.store(view.code(), Ordering::Release);
    }
    if let Some(flag) = request.joint_flag {
        JOINT_FLAG.store(flag, Ordering::Release);
    }
    let epoch = VIEW_EPOCH.fetch_add(1, Ordering::AcqRel) + 1;
    let passes = STORE.lock().passes;
    crate::runtime::diag_log::diagnostic(format_args!(
        "[Hitbox] orbs request route={:?} view={:?} joint={} epoch={epoch} pass={passes}",
        route(),
        view(),
        JOINT_FLAG.load(Ordering::Acquire)
    ));
}

/// Clear set `i` with the effects interface of the route it was filled under; its orbs go too.
fn clear_set(store: &mut Store, i: usize, accessor: usize) -> Counts {
    BOARD.lock().clear(accessor);
    if Route::from_code(store.routes[i]).effects() {
        store.table.sets[i].clear(&mut effects(accessor))
    } else {
        store.table.sets[i].clear(&mut OrbSink { status: i32::MIN })
    }
}

/// Place every sphere `set` holds on its bone and publish them as `accessor`'s orbs.
unsafe fn publish_orbs(accessor: usize, set: &hitbox::HandleSet) -> usize {
    let flag = JOINT_FLAG.load(Ordering::Acquire);
    let mut placed = [Orb::NONE; hitbox::MAX_HELD];
    let mut len = 0;
    for sphere in set.spheres() {
        if let Some(centre) = natives::joint_position(accessor, sphere.joint, &sphere.pos, flag) {
            placed[len] = orbs::orb_of(sphere, centre);
            len += 1;
        }
    }
    BOARD.lock().publish(accessor, placed[..len].iter().copied())
}

/// Route B's spheres of set `i` shown or hidden for the A/B view (only under `Route::Both`).
unsafe fn apply_view(store: &mut Store, i: usize, epoch: u32, force: bool) {
    let key = store.table.key(i);
    if key == 0 || Route::from_code(store.routes[i]) != Route::Both || (store.view_applied[i] == epoch && !force) {
        return;
    }
    let (_, effects_shown) = orbs::layers(Route::Both, view());
    for handle in store.table.sets[i].handles() {
        natives::sphere_set_visible(key, handle, effects_shown);
    }
    store.view_applied[i] = epoch;
}

/// The orb panes of the layout last drawn into, resolved once per layout registration, and how many were shown.
struct OrbPanes {
    layout: usize,
    registrations: u32,
    container: usize,
    panes: [usize; crate::model::panes::ORBS],
    shown: usize,
}

static ORB_PANES: Mutex<OrbPanes> =
    Mutex::new(OrbPanes { layout: 0, registrations: 0, container: 0, panes: [0; crate::model::panes::ORBS], shown: 0 });

/// The layout tick's draw (`ui::hud`): every live orb projected with the game's own `world_to_screen` (so the orbs follow
/// the camera, paused or not) and written into one of the plugin's disc panes; the rest hidden. A readiness boundary:
/// nothing before boot installed the feature.
///
/// # Safety
/// `layout` is the live layout the tick passed; called on the UI job thread inside the tick.
pub unsafe fn draw_orbs(layout: *mut skyline::nn::ui2d::Layout, name: &[u8]) {
    use crate::model::panes as p;
    use crate::runtime::ui::{pane, tick};
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let waiting = name == b"info_waiting";
    let active = if waiting { crate::runtime::lobby::training::lobby::requested() } else { name == b"info_training" && scene::is_training() };
    if !active || layout.is_null() {
        return;
    }
    let _scope = HookScope::enter_hook(HookId::HitboxOrbs);
    let (shown, _) = orbs::layers(route(), view());
    let mut cache = ORB_PANES.lock();
    if !shown && cache.shown == 0 {
        return;
    }
    let registrations = tick::REGISTRATIONS.load(Ordering::Acquire);
    if cache.layout != layout as usize || cache.registrations != registrations {
        let root = (*layout).root_pane;
        let parent = if waiting { pane::find_child(root, b"TrModPort") } else { root };
        let container = pane::find_child(parent, p::name_of(p::IDX_ORBS).as_bytes());
        cache.layout = layout as usize;
        cache.registrations = registrations;
        cache.container = container as usize;
        cache.shown = 0;
        for k in 0..p::ORBS {
            cache.panes[k] = pane::find_child(container, p::name_of(p::IDX_ORB + k).as_bytes()) as usize;
        }
    }
    if cache.container == 0 {
        return;
    }
    let menu = crate::runtime::published::input_owners().contains(crate::model::input_delay::Owners::MENU);
    let mut n = 0;
    let mut over = 0;
    if shown && SETTING.load(Ordering::Acquire) && !menu {
        let half = match natives::screen_size_words() {
            Some([w, h]) => [orbs::screen_half(w, orbs::SCREEN_HALF_DEFAULT[0]), orbs::screen_half(h, orbs::SCREEN_HALF_DEFAULT[1])],
            None => [0.0; 2],
        };
        let board = BOARD.lock();
        for orb in board.live() {
            let [a, b] = orbs::edge_points(orb);
            let (Some(c), Some(ea), Some(eb)) = (natives::world_to_screen(&orb.centre), natives::world_to_screen(&a), natives::world_to_screen(&b)) else { continue };
            let Some(plan) = orbs::plan_pane(c, [ea, eb], half, orb.color) else { continue };
            if n == p::ORBS {
                over += 1;
                continue;
            }
            let target = cache.panes[n] as *mut skyline::nn::ui2d::Pane;
            if !target.is_null() {
                pane::set_pos(target, plan.x, plan.y);
                pane::set_width(target, plan.size);
                pane::set_height(target, plan.size);
                pane::set_rect_color(target, plan.color);
                pane::set_visible(target, true);
            }
            n += 1;
        }
    }
    for k in n..cache.shown {
        let target = cache.panes[k] as *mut skyline::nn::ui2d::Pane;
        if !target.is_null() {
            pane::set_visible(target, false);
        }
    }
    pane::set_visible(cache.container as *mut skyline::nn::ui2d::Pane, n != 0);
    cache.shown = n;
    #[cfg(feature = "diag")]
    stats::orbs_drawn(n as u32, over);
    #[cfg(not(feature = "diag"))]
    let _ = over;
}

/// The input hook's call while the game may be paused (no fighter pass runs then): applies a new A/B view to route B's
/// spheres of the fighters seen in the last passes, so both layers are shown on one frozen frame.
///
/// # Safety
/// Called on the game thread from the P1 input hook.
pub unsafe fn view_poll() {
    if !INSTALLED.load(Ordering::Acquire) || route() != Route::Both {
        return;
    }
    let epoch = VIEW_EPOCH.load(Ordering::Acquire);
    if POLL_EPOCH.swap(epoch, Ordering::AcqRel) == epoch {
        return;
    }
    let _scope = HookScope::enter_hook(HookId::HitboxOrbs);
    let mut store = STORE.lock();
    for i in 0..hitbox::FIGHTER_SETS {
        if store.passes.wrapping_sub(store.seen_pass[i]) < orbs::STALE_PUBLISHES {
            apply_view(&mut store, i, epoch, false);
        }
    }
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[Hitbox] orbs view_poll view={:?} epoch={epoch} pass={}", view(), store.passes));
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
    let mut store = STORE.lock();
    if let Some(i) = store.table.find(accessor) {
        store.table.sets[i].forget();
    }
    HELD.store(store.table.held_spheres(), Ordering::Relaxed);
    BOARD.lock().clear(accessor);
}

/// The game's effect module of one fighter, as `model::hitbox::Effects`.
struct GameEffects {
    accessor: usize,
    effect: u64,
    attribute: u32,
}

impl Effects for GameEffects {
    fn sphere_alive(&mut self, handle: u32) -> bool {
        // SAFETY: `accessor` is the live fighter whose pass or hook runs now; the native checks its own grant.
        unsafe { natives::sphere_is_exist(self.accessor, handle) }
    }

    fn sphere_kill_own(&mut self, handle: u32) {
        // SAFETY: as above; `handle` is one of this fighter's own spheres that still exists.
        unsafe { natives::effect_kill(self.accessor, handle) }
    }

    fn sphere_spawn(&mut self, sphere: &Sphere) -> u32 {
        // SAFETY: as above.
        unsafe {
            let tail = (true, self.attribute, 0, 0, 0, 0, true, true);
            #[cfg(feature = "diag")]
            let pool_before = stats::pool_snapshot();
            #[cfg(feature = "diag")]
            let limits_before = stats::emitter_limits();
            let handle = natives::effect_req_follow(self.accessor, self.effect, sphere.joint, &sphere.pos, &[0.0; 3], sphere.scale, tail);
            if handle != 0 {
                natives::sphere_set_rgb(self.accessor, sphere.tint);
                natives::sphere_set_rate(self.accessor, hitbox::RATE);
            }
            #[cfg(feature = "diag")]
            if handle == 0 {
                stats::refused(self.accessor, sphere, pool_before, limits_before);
            }
            handle
        }
    }
}

fn effects(accessor: usize) -> GameEffects {
    GameEffects { accessor, effect: EFFECT.load(Ordering::Acquire), attribute: ATTRIBUTE.load(Ordering::Acquire) }
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

struct GameAttacks(usize);

impl hitbox::AttackReader for GameAttacks {
    fn active(&mut self, slot: i32) -> bool {
        // SAFETY: the current fighter's pass, after the feature's closure passed.
        unsafe { natives::attack_is_attack(self.0, slot) }
    }

    fn data(&mut self, slot: i32) -> usize {
        // SAFETY: as above; only an active AttackModule slot is queried.
        unsafe { natives::attack_data(self.0, slot) }
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

/// One shape requested once and not tracked: an article's command (projectiles are not in the per-frame pass, spec Q1); it ends
/// with the article's status.
unsafe fn draw_untracked(accessor: usize, shape: &Shape, tint: [f32; 3]) -> u32 {
    let mut fx = effects(accessor);
    let mut wants = [Sphere::NONE; hitbox::SPHERE_MAX];
    let mut len = 0;
    hitbox::push_shape(&mut wants, &mut len, hitbox::KIND_HIT, 0, shape, tint);
    let mut made = 0;
    for sphere in &wants[..len] {
        if fx.sphere_spawn(sphere) != 0 {
            made += 1;
        }
    }
    made
}

// ---- the per-frame pass ---------------------------------------------------------------------------------------------

/// One fighter's pass (SPEC-core-06 §5), after the game's frame for it. A readiness boundary: nothing happens before boot
/// installed the feature. With the setting Off, nothing held and nothing to restore it makes no game call at all (Q4).
///
/// # Safety
/// `fighter` is the fighter object the game just ran a frame of.
pub unsafe fn frame(fighter: usize) {
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let _scope = HookScope::enter_hook(HookId::HitboxFrame);
    #[cfg(feature = "diag")]
    stats::frame_begin();
    let enabled = SETTING.load(Ordering::Acquire);
    if !enabled && SLEPT.load(Ordering::Relaxed) == 0 && HELD.load(Ordering::Relaxed) == 0 {
        return;
    }
    let accessor = natives::fighter_accessor(fighter);
    if accessor == 0 {
        return;
    }
    let slept = BOOK.lock().slept(accessor);
    let mut store = STORE.lock();
    let held = store.table.find(accessor).is_some_and(|i| !store.table.sets[i].is_empty());
    // Off with nothing of this fighter's to restore or kill: no game call at all.
    if !enabled && !slept && !held {
        return;
    }
    let visualizes = visualizes(accessor);
    if !visualizes || !ensure_ready() {
        // Not (or no longer) a practice fighter: its own spheres go, its effect flag is never touched.
        if let Some(i) = store.table.find(accessor) {
            clear_set(&mut store, i, accessor);
            HELD.store(store.table.held_spheres(), Ordering::Relaxed);
        }
        return;
    }
    let ids = status_ids();
    let status = if enabled { natives::status_kind(accessor) } else { 0 };
    let facts = FrameFacts { visualizes: true, enabled, slept, status, ids };
    let plan = hitbox::plan(facts);
    if plan.restore {
        natives::set_sleep_effect(accessor, false);
    }
    if plan.sleep {
        natives::set_sleep_effect(accessor, true);
    }
    let i = store.table.index_for(accessor);
    let epoch = RESTORE_EPOCH.load(Ordering::Acquire);
    let route = route();
    let mut counts = Counts::default();
    if store.seen_epoch[i] != epoch || store.routes[i] != route.code() {
        // A save state was loaded since this fighter's last pass, or the route changed: its spheres go and come back from the
        // restored state, under the route now in force.
        store.seen_epoch[i] = epoch;
        counts = clear_set(&mut store, i, accessor);
        store.routes[i] = route.code();
    }
    store.passes = store.passes.wrapping_add(1);
    store.seen_pass[i] = store.passes;
    let Store { table, wants, .. } = &mut *store;
    let set = &mut table.sets[i];
    // Route B (operator decision 2026-11-04 ~06:5xZ, `docs/DECISIONS.md` HB1-route, `review/menu-proposal` commit
    // `5ca5e3b`): while the setting is On the pass reads the HitModule (the relied `hurtbox_*` rows) and plans the
    // enabled parts' hurt spheres into the same wants, drawn from the same shared effect pool as the hit spheres
    // (`docs/HB1-RENDER-ROUTES.md (removed after v0.2.0-rc.1; see that tag)`). The reader is constructed only here, behind `enabled`: Off — and the stop paths
    // the dispatch takes first — reads no HitModule, so the shipped build's HitModule reads are On-only.
    let mut hurt = enabled.then(|| hitbox::GameHurt { accessor, load: mem_load });
    let hurt: Option<&mut dyn hitbox::HurtboxReader> = hurt.as_mut().map(|h| h as _);
    let (c, len) = if route.effects() {
        hitbox::reconcile_frame(set, facts, wants, &mut GameAttacks(accessor), hurt, &mut effects(accessor))
    } else {
        hitbox::reconcile_frame(set, facts, wants, &mut GameAttacks(accessor), hurt, &mut OrbSink { status })
    };
    let placed = if route.orbs() { publish_orbs(accessor, set) } else { 0 };
    #[cfg(feature = "diag")]
    let (hits, hurts) = {
        let hits = wants[..len].iter().filter(|s| s.key >> 24 == hitbox::KIND_HIT).count() as u32;
        (hits, len as u32 - hits)
    };
    #[cfg(feature = "diag")]
    stats::wanted(hits, hurts);
    #[cfg(not(feature = "diag"))]
    let _ = len;
    let held_now = set.len();
    HELD.store(table.held_spheres(), Ordering::Relaxed);
    if route == Route::Both {
        apply_view(&mut store, i, VIEW_EPOCH.load(Ordering::Acquire), c.spawned != 0);
    }
    drop(store);
    counts.kept += c.kept;
    counts.spawned += c.spawned;
    counts.killed += c.killed;
    counts.ended += c.ended;
    counts.failed += c.failed;
    counts.dropped += c.dropped;
    #[cfg(feature = "diag")]
    stats::orbs_pass(i, route, placed as u32, held_now as u32, c.failed);
    #[cfg(not(feature = "diag"))]
    let _ = placed;
    // The diag hurt census (decision HB1): a second HitModule read per fighter per frame over the pass's own read,
    // for the census line's parts/first-pass/cost fields; no sphere is requested by the census itself, and the
    // shipped build has no census at all (the module notes).
    #[cfg(feature = "diag")]
    if enabled && visualizes {
        stats::hurt_census(accessor);
    }
    let slept_now = hitbox::slept_after(facts, plan);
    if slept_now != facts.slept {
        book_set(accessor, slept_now);
    }
    #[cfg(feature = "diag")]
    stats::frame_end(plan, counts, held_now as u32, HELD.load(Ordering::Relaxed) as u32);
    #[cfg(not(feature = "diag"))]
    let _ = (counts, held_now);
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

/// The plugin part of an animation command. A fighter's hitbox is drawn by its pass in the same frame (the pass runs after the
/// game's frame, which ran this command), so nothing is requested here; a fighter's grab box is queued for that pass (the
/// command runs inside the animation script, where the fighter's effects may be asleep); an article's shape is requested now.
unsafe fn animation_command(lua: usize, indices: Indices, grab: bool) {
    let _scope = HookScope::enter_hook(if grab { HookId::HitboxCatch } else { HookId::HitboxAttack });
    if lua == 0 || !SETTING.load(Ordering::Acquire) || !ensure_ready() {
        return;
    }
    let accessor = natives::lua_accessor(lua);
    if accessor == 0 || !hitbox::draw_on_command(true, visualizes(accessor)) {
        return;
    }
    let fighter = natives::is_fighter(accessor);
    if fighter && !grab {
        return;
    }
    let Some(shape) = hitbox::parse(indices, |i| lua_cell(lua, i)) else { return };
    let tint = if grab { hitbox::grab_tint(shape.id) } else { hitbox::hitbox_tint(shape.id) };
    let made = if fighter {
        let mut wants = [Sphere::NONE; hitbox::SPHERE_MAX];
        let mut len = 0;
        hitbox::push_shape(&mut wants, &mut len, hitbox::KIND_GRAB, shape.id as usize, &shape, tint);
        let mut store = STORE.lock();
        let i = store.table.index_for(accessor);
        wants[..len].iter().filter(|w| store.table.sets[i].push_grab(**w)).count() as u32
    } else {
        draw_untracked(accessor, &shape, tint)
    };
    #[cfg(feature = "diag")]
    stats::command(grab, fighter, shape.id, shape.joint, shape.size, shape.b.is_some(), made);
    #[cfg(not(feature = "diag"))]
    let _ = made;
}

/// `sv_animcmd::ATTACK`: the plugin part first, then the game's own command.
#[skyline::hook(offset = sites_hitbox::HITBOX_ATTACK_OFFSET)]
pub unsafe fn hitbox_attack_hook(lua_state: usize) {
    animation_command(lua_state, hitbox::ATTACK, false);
    call_original!(lua_state)
}

/// `sv_animcmd::CATCH`: the plugin part first, then the game's own command.
#[skyline::hook(offset = sites_hitbox::HITBOX_CATCH_OFFSET)]
pub unsafe fn hitbox_catch_hook(lua_state: usize) {
    animation_command(lua_state, hitbox::CATCH, true);
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
    let shielding = status_ids().shielding(natives::status_kind(accessor));
    if hitbox::kill_on_rebound(rebound, true, true, shielding) {
        let mut store = STORE.lock();
        if let Some(i) = store.table.find(accessor) {
            let c = clear_set(&mut store, i, accessor);
            HELD.store(store.table.held_spheres(), Ordering::Relaxed);
            #[cfg(feature = "diag")]
            stats::rebound_cleared(c.killed);
            #[cfg(not(feature = "diag"))]
            let _ = c;
        }
    }
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

/// Boot: install the three hooks and switch the per-frame pass on when the feature's whole closure passed its guards (`closure`:
/// its own rows, the Training predicate and the rows of the natives it shares, `model::capabilities`; and the CPU core whose
/// frame wrapper carries the pass). Returns whether it is installed and the per-row state for the install line.
pub fn install(found: &[GuardOutcome; N], closure: bool) -> (bool, [Written; N]) {
    let mut written = [Written::Skipped; N];
    for (slot, outcome) in written.iter_mut().zip(found) {
        if outcome.usable() {
            *slot = Written::Checked;
        }
    }
    if closure {
        skyline::install_hook!(hitbox_attack_hook);
        skyline::install_hook!(hitbox_catch_hook);
        skyline::install_hook!(hitbox_rebound_hook);
        for (slot, site) in written.iter_mut().zip(&sites_hitbox::SITES) {
            if sites_hitbox::HOOK_ROWS.contains(&site.name) {
                *slot = Written::Far;
            }
        }
    }
    set_installed(closure);
    (closure, written)
}

/// The rows of `sites_hitbox` for the install line.
pub fn reports(found: &[GuardOutcome; N], written: &[Written; N]) -> [SiteReport; N] {
    std::array::from_fn(|i| SiteReport { site: sites_hitbox::SITES[i].name, found: found[i], written: written[i] })
}

// ---- diag: the cost of the pass and the plugin's share of the effect pool -------------------------------------------------

/// Counters and one line per 600 passes (`[Hitbox] stats ...`), so the Eden rows read the cost per frame, the spheres the plugin
/// holds (its share of the game's effect pool), the requests the game refused, and what changed; the shipped build compiles all of
/// it away.
#[cfg(feature = "diag")]
mod stats {
    use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

    use crate::model::hitbox::{self, Counts, FramePlan};
    use crate::runtime::diag_log::diagnostic;
    use hos_lock::Mutex;

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
    static SLEEPS: AtomicU32 = AtomicU32::new(0);
    static RESTORES: AtomicU32 = AtomicU32::new(0);
    static COMMANDS: AtomicU32 = AtomicU32::new(0);
    static MAX_HELD: AtomicU32 = AtomicU32::new(0);
    static TICKS: AtomicU64 = AtomicU64::new(0);
    static MAX_TICKS: AtomicU64 = AtomicU64::new(0);
    static START: AtomicU64 = AtomicU64::new(0);
    static LAST: AtomicU64 = AtomicU64::new(u64::MAX);
    const REPORT_EVERY: u32 = 600;
    static REFUSALS_REPORTED: AtomicU32 = AtomicU32::new(0);

    /// The two hitbox_pool_* relied rows guard these owned-native facts. Called
    /// only inside the installed hitbox unit, on the game thread; no mutation.
    pub unsafe fn pool_snapshot() -> Option<(u32, bool)> {
        let singleton = ((crate::runtime::game::text_base() + 0x5334920) as *const usize).read();
        if singleton == 0 || singleton % 8 != 0 { return None; }
        let manager = (singleton as *const usize).read();
        if manager == 0 || manager % 8 != 0 { return None; }
        Some((((manager + 0x19468) as *const u32).read(),
            ((manager + 0x19460) as *const usize).read() != 0))
    }

    /// Raw native limit/count pairs from the two guarded constructor branches.
    /// No writes and no inferred frame budget: the failed call is compared with its own entry snapshot.
    pub unsafe fn emitter_limits() -> [i32; 4] {
        let base = crate::runtime::game::text_base();
        [0x5334a00, 0x5334a0c, 0x5334a04, 0x5334a14]
            .map(|offset| ((base + offset) as *const i32).read())
    }

    pub unsafe fn refused(accessor: usize, sphere: &crate::model::hitbox::Sphere, before: Option<(u32, bool)>, limits_before: [i32; 4]) {
        if REFUSALS_REPORTED.fetch_add(1, Ordering::Relaxed) >= 8 { return; }
        let after = pool_snapshot();
        let limits_after = emitter_limits();
        diagnostic(format_args!(
            "[Hitbox] refused accessor={accessor:#x} key={} joint={:#x} scale={} free_head_before={before:?} free_head_after={after:?} limits_before={limits_before:?} limits_after={limits_after:?}",
            sphere.key, sphere.joint, sphere.scale
        ));
    }

    fn now() -> u64 {
        // SAFETY: `GetSystemTick` has no preconditions.
        unsafe { skyline::nn::os::GetSystemTick() as u64 }
    }

    pub fn frame_begin() {
        START.store(now(), Ordering::Relaxed);
    }

    pub fn wanted(hits: u32, hurts: u32) {
        WANT_HIT.fetch_add(hits, Ordering::Relaxed);
        WANT_HURT.fetch_add(hurts, Ordering::Relaxed);
    }

    pub fn frame_end(plan: FramePlan, c: Counts, held: u32, held_total: u32) {
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
        if plan.sleep {
            SLEEPS.fetch_add(1, Ordering::Relaxed);
        }
        if plan.restore {
            RESTORES.fetch_add(1, Ordering::Relaxed);
        }
        // One line whenever this fighter's set changed (a sphere was requested, killed, ended or refused).
        let change = (u64::from(held) << 32) | u64::from(c.spawned + c.killed + c.ended + c.failed);
        if (c.spawned | c.killed | c.ended | c.failed) != 0 && LAST.swap(change, Ordering::Relaxed) != change {
            diagnostic(format_args!(
                "[Hitbox] pass held={held} held_total={held_total} kept={} spawned={} killed={} ended={} failed={} dropped={} sleep={} redraw={}",
                c.kept, c.spawned, c.killed, c.ended, c.failed, c.dropped, plan.sleep, plan.redraw
            ));
        }
        let passes = PASSES.fetch_add(1, Ordering::Relaxed) + 1;
        if passes >= REPORT_EVERY {
            let take = |c: &AtomicU32| c.swap(0, Ordering::Relaxed);
            let passes = take(&PASSES).max(1);
            diagnostic(format_args!(
                "[Hitbox] stats passes={passes} with_work={} want_hit={} want_hurt={} kept={} spawned={} killed={} ended={} failed={} dropped={} held_max={} held_now={held_total} sleeps={} restores={} commands={} pass_ticks_avg={} pass_ticks_max={} heap_allocations={}",
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
                take(&SLEEPS),
                take(&RESTORES),
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

    pub fn rebound_cleared(killed: u32) {
        diagnostic(format_args!("[Hitbox] rebound cleared: killed={killed}"));
    }

    // ---- own orbs (HB1 route A4 prototype) ----------------------------------------------------------------------------

    static PASS_NO: AtomicU32 = AtomicU32::new(0);
    static LAST_ORB_PASS: [AtomicU64; hitbox::FIGHTER_SETS] = [const { AtomicU64::new(u64::MAX) }; hitbox::FIGHTER_SETS];
    static LAST_DRAWN: AtomicU64 = AtomicU64::new(u64::MAX);

    /// One fighter's (set `set`) pass: a line whenever its route, placed orbs, held spheres or refused requests change
    /// (route B's `failed` of the same pass beside the orbs' placed count).
    pub fn orbs_pass(set: usize, route: super::Route, placed: u32, held: u32, failed: u32) {
        let pass = PASS_NO.fetch_add(1, Ordering::Relaxed) + 1;
        let key = (u64::from(route.code()) << 48) | (u64::from(placed) << 32) | (u64::from(held) << 16) | u64::from(failed);
        if LAST_ORB_PASS[set % hitbox::FIGHTER_SETS].swap(key, Ordering::Relaxed) != key && (route.orbs() || failed != 0) {
            diagnostic(format_args!("[Hitbox] orbs pass set={set} route={route:?} placed={placed} held={held} failed={failed} pass={pass}"));
        }
    }

    /// One layout tick's draw: a line whenever the number of orb panes shown changes, with the pass number it was drawn at
    /// (a paused game draws at one pass number).
    pub fn orbs_drawn(n: u32, over: u32) {
        let pass = PASS_NO.load(Ordering::Relaxed);
        let key = (u64::from(n) << 32) | u64::from(over);
        if LAST_DRAWN.swap(key, Ordering::Relaxed) != key {
            diagnostic(format_args!("[Hitbox] orbs drawn n={n} over={over} view={:?} pass={pass}", super::view()));
        }
    }

    // ---- the hurt census (decision HB1) -------------------------------------------------------------------------------

    /// Census kinds tracked at once (eight distinct fighter kinds in a Training boot; a ninth takes the least
    /// recently used slot).
    const CENSUS_KINDS: usize = 8;

    /// One slot per fighter kind: the kind's open census window and its LRU clock. Game-thread state (the
    /// per-frame pass), one uncontended lock like `BOOK`.
    struct CensusBook {
        kinds: [i32; CENSUS_KINDS],
        used: [u64; CENSUS_KINDS],
        clock: u64,
        states: [hitbox::HurtCensus; CENSUS_KINDS],
    }

    impl CensusBook {
        const fn new() -> CensusBook {
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

    static CENSUS: Mutex<CensusBook> = Mutex::new(CensusBook::new());

    /// The census read of one fighter's pass: the enabled parts and the spheres they want, the read's cost in
    /// ticks, one bounded line per fighter kind per `REPORT_EVERY` frames. No sphere is requested, nothing drawn.
    pub fn hurt_census(accessor: usize) {
        // SAFETY: the current fighter's pass, after the feature's closure passed; the native checks its own grant.
        let kind = unsafe { crate::runtime::game::natives::fighter_kind(accessor) }.unwrap_or(-1);
        let window = {
            let mut book = CENSUS.lock();
            let slot = book.slot_for(kind);
            let mut reader = hitbox::GameHurt { accessor, load: super::mem_load };
            let start = now();
            let mut spheres = 0usize;
            let parts = hitbox::hurt_parts(&mut reader, |_, shape| spheres += hitbox::spheres(shape.a, shape.b, shape.size).count());
            slot.observe(parts, spheres, now().wrapping_sub(start));
            slot.window(REPORT_EVERY)
        };
        if let Some(window) = window {
            diagnostic(format_args!("[Hitbox] hurt census kind={kind} {window}"));
        }
    }

}
