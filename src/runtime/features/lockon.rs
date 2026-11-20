//! A native search target for each breakable target in ordinary Training.

use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use smash::app::{self, lua_bind::*, sv_battle_object, BattleObjectModuleAccessor as Accessor};
use smash::lib::lua_const::*;
use smash::phx::Vector3f;

use crate::model::{
    btt, eggs, hitbox,
    lockon::{free_slot_top, names_helper, take_damage, ItemHost, Target, Tracker},
    lra_reset,
    practice_games::Game,
    sites_lockon::{ITEM_FACTORY_OFFSET, OBJECT_POOLS_SLOT},
    sites_hitbox,
};
use crate::runtime::{
    compat::common, compat::consts::FighterId, game, scene, state_natives as native,
};

const COUNT: usize = 1 + btt::MAX;
use crate::model::lockon::{HELPER_CENTER_HEIGHT as CENTER_HEIGHT, HELPER_SCALE as SCALE, PICTURE_RADIUS};
const _: () = assert!(PICTURE_RADIUS == eggs::RADIUS);

static USABLE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
struct Handle {
    id: u32,
    accessor: *mut Accessor,
    damage: f32,
}

impl Handle {
    unsafe fn live(self) -> bool {
        sv_battle_object::is_active(self.id)
            && sv_battle_object::module_accessor(self.id) == self.accessor
            && app::utility::get_category(&mut *self.accessor) == *BATTLE_OBJECT_CATEGORY_ITEM
            && app::utility::get_kind(&mut *self.accessor) == *ITEM_KIND_SANDBAG
    }
}

struct GameItems(RefCell<Tracker<Handle, COUNT>>);
// SAFETY: only P1's primary game-thread callback accesses this cell. A native
// call reentering that callback cannot borrow the items a second time.
unsafe impl Sync for GameItems {}
static LIVE: GameItems = GameItems(RefCell::new(Tracker::new()));
/// The tracker's items' ids (0: an empty slot) and their scene, for the visualizer, which never draws them.
static HELPERS: [AtomicU32; COUNT] = [const { AtomicU32::new(0) }; COUNT];
static HELPER_SCENE: AtomicU32 = AtomicU32::new(0);
static ANY_HELPER: AtomicBool = AtomicBool::new(false);

pub fn is_helper(id: u32) -> bool {
    if !ANY_HELPER.load(Ordering::Acquire) {
        return false;
    }
    let ids = HELPERS.each_ref().map(|h| h.load(Ordering::Relaxed));
    names_helper(&ids, HELPER_SCENE.load(Ordering::Relaxed), lra_reset::GENERATION.load(Ordering::Acquire), id)
}

pub fn set_usable(usable: bool) {
    USABLE.store(usable, Ordering::Release);
}

/// The native hurtbox can connect before the rendered target enters the attack.
/// Use the owned item's damage once, with the same owner and break rule.
/// # Safety
/// Player 1's game-thread pass, with a live fighter accessor.
pub unsafe fn hit(
    owner: usize, slot: usize, key: u64, rule: u8, aerial: bool, projectiles: bool,
) -> Option<()> {
    if !USABLE.load(Ordering::Acquire) || super::states::is_loading() {
        return None;
    }
    let mut tracker = LIVE.0.try_borrow_mut().ok()?;
    let item = tracker.item_mut(lra_reset::GENERATION.load(Ordering::Acquire), slot, key)?;
    item.live().then_some(())?;
    let at = item.accessor as usize;
    take_damage(&mut item.damage, game::natives::damage(at, 0)).then_some(())?;
    use sites_hitbox::damage_record as record;
    let module = ((at + record::MODULE) as *const usize).read();
    btt::pointer_ok(module).then_some(())?;
    // DamageModule keeps the source and its resolved native owner after the attack ends.
    let source = ((module + 0x48) as *const u32).read();
    let root = ((module + 0x54) as *const u32).read();
    let p1 = (*(owner as *const Accessor)).battle_object_id;
    (root == p1 && (source == p1 || projectiles && matches!(source >> 28, 1 | 4))).then_some(())?;
    let power = if rule >= eggs::STRONG {
        let (attacker, attack, _) = super::input_display::struck(at)?;
        (attacker == source && i32::from(attack) < hitbox::SLOTS).then_some(())?;
        native::object_is_active(source).then_some(())?;
        let from = native::object_accessor(source);
        btt::pointer_ok(from).then_some(())?;
        let data = game::natives::attack_data(from, i32::from(attack));
        btt::pointer_ok(data).then_some(())?;
        ((data + hitbox::layout::POWER) as *const f32).read()
    } else {
        0.0
    };
    eggs::counts(rule, aerial, power).then_some(())
}

#[repr(C, align(16))]
struct Spawn([u32; 20]);

unsafe fn has_item_capacity(manager: usize) -> bool {
    let registry = ((game::text_base() + OBJECT_POOLS_SLOT) as *const usize).read();
    let slots = (manager as *const usize).read();
    if registry == 0 || slots == 0 || ((manager + 0xd4) as *const u8).read() != 0 {
        return false;
    }
    let pool = ((registry + 0x20) as *const usize).read();
    pool != 0 && ((pool + 0x1c) as *const u16).read() != u16::MAX
        && free_slot_top(((slots + 0x58) as *const [usize; 3]).read())
            .is_some_and(|top| (top as *const i32).read() >= 0)
}

struct NativeItems {
    manager: usize,
    team: Option<(i32, u32)>,
    spawn: bool,
}

fn position([x, y]: [f32; 2]) -> Vector3f {
    Vector3f {
        x,
        y: y - CENTER_HEIGHT,
        z: 0.0,
    }
}

impl ItemHost for NativeItems {
    type Handle = Handle;

    fn owns_live_item(&mut self, item: Handle) -> bool {
        unsafe { item.live() }
    }

    fn create_target_item(&mut self, at: [f32; 2]) -> Option<Handle> {
        let (_, owner) = self.team?;
        if !core::mem::take(&mut self.spawn) {
            return None; // Spread a full BTT layout's native creations across frames.
        }
        unsafe {
            // The ordinary item wrapper caps Sandbags at one. The factory's forced mode
            // admits duplicates, with both free pools checked before it can evict an item.
            if !game::granted(ITEM_FACTORY_OFFSET) || !has_item_capacity(self.manager) {
                return None;
            }
            let pos = position(at);
            let mut descriptor = Spawn([0; 20]);
            descriptor.0[..3].copy_from_slice(&[pos.x, pos.y, pos.z].map(f32::to_bits));
            descriptor.0[8..15].copy_from_slice(&[
                *ITEM_KIND_SANDBAG as u32, owner, 0, 1.0f32.to_bits(), owner, 20, u32::MAX,
            ]);
            descriptor.0[18] = 1.0f32.to_bits();
            let create: unsafe extern "C" fn(usize, &Spawn, u32, u32, u32) -> *mut app::Item =
                core::mem::transmute(game::text_base() + ITEM_FACTORY_OFFSET);
            let item = create(self.manager, &descriptor, 1, 0, 0);
            if item.is_null() {
                return None;
            }
            let id = native::item_object_id(item as usize);
            let accessor = Item::item_module_accessor(item) as *mut Accessor;
            if accessor.is_null() {
                native::item_remove(self.manager, id);
                return None;
            }
            WorkModule::on_flag(accessor, *ITEM_SANDBAG_INSTANCE_WORK_FLAG_NO_POCKET);
            WorkModule::on_flag(accessor, *ITEM_SANDBAG_INSTANCE_WORK_FLAG_NO_LANDING_SMOKE);
            WorkModule::on_flag(accessor, *ITEM_INSTANCE_WORK_FLAG_DISABLE_AUTO_GRAVITY_MOVE);
            PostureModule::set_scale(accessor, SCALE, false);
            Some(Handle { id, accessor, damage: game::natives::damage(accessor as usize, 0) })
        }
    }

    fn place_target_item(&mut self, item: Handle, at: [f32; 2]) {
        unsafe {
            let accessor = item.accessor;
            // Native Sandbag initialization can reset the team and enable jostling.
            if let Some((team, _)) = self.team.filter(|(team, _)| TeamModule::team_no(accessor) as i32 != *team) {
                TeamModule::set_team(accessor, team, true);
                TeamModule::set_hit_team(accessor, team);
            }
            JostleModule::set_status(accessor, false);
            PostureModule::set_pos(accessor, &position(at));
            KineticModule::clear_speed_all(accessor);
            VisibilityModule::set_whole(accessor, false);
            EffectModule::set_whole(accessor, false);
            ShadowModule::set_draw_status(accessor, false);
        }
    }

    fn retire_target_item(&mut self, item: Handle) {
        unsafe { native::item_remove(self.manager, item.id) };
    }
}

fn wanted(game: Option<Game>) -> Option<[Option<Target>; COUNT]> {
    let mut targets = [None; COUNT];
    match game {
        Some(Game::Eggs) => targets[0] = super::hitbox::lockon_egg()?,
        Some(Game::Btt) => targets[1..].copy_from_slice(&super::btt::lockon_targets()?),
        _ => {}
    }
    Some(targets)
}

/// The practice snapshots use try_lock and are released before any game call.
///
/// # Safety
/// P1's non-Nana game-thread pass, after the practice games and save-state step.
pub unsafe fn update_practice_items(fighter: usize) {
    if !USABLE.load(Ordering::Acquire) {
        return;
    }
    let active = super::practice::active().filter(|g| matches!(g, Game::Eggs | Game::Btt));
    let Ok(mut tracker) = LIVE.0.try_borrow_mut() else {
        return;
    };
    if active.is_none() && tracker.is_empty() {
        return;
    }
    let facts = scene::facts();
    if !facts.native_training || facts.busy || facts.waiting_room {
        return;
    }
    let targets = if super::states::is_loading() {
        [None; COUNT]
    } else {
        let Some(targets) = wanted(active) else {
            return;
        };
        targets
    };
    let manager = native::item_manager();
    if manager == 0 {
        return;
    }
    let team = if targets.iter().any(Option::is_some) {
        // An uncarryable Sandbag on the CPU's team stays outside the CPU's target selection.
        let own = game::natives::fighter_accessor(fighter) as *mut Accessor;
        common::try_get_module_accessor(FighterId::CPU).and_then(|cpu| {
            let team = TeamModule::team_no(cpu) as i32;
            (team >= 0 && !own.is_null() && team != TeamModule::team_no(own) as i32)
                .then(|| (team, (*cpu).battle_object_id))
        })
    } else {
        None
    };
    tracker.update(
        lra_reset::GENERATION.load(Ordering::Acquire),
        targets,
        &mut NativeItems {
            manager,
            team,
            spawn: true,
        },
    );
    // The visualizer leaves out exactly the slots' items, for this scene only (review: stale ids after a scene change).
    let ids = tracker.ids(|item| item.id);
    for (published, id) in HELPERS.iter().zip(ids) {
        published.store(id, Ordering::Relaxed);
    }
    HELPER_SCENE.store(tracker.scene(), Ordering::Relaxed);
    ANY_HELPER.store(ids.iter().any(|&id| id != 0), Ordering::Release);
}
