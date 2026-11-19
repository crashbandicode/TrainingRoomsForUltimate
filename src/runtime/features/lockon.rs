//! A native search target for each breakable target in ordinary Training.

use core::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering};
use smash::app::{self, lua_bind::*, sv_battle_object, BattleObjectModuleAccessor as Accessor};
use smash::lib::lua_const::*;
use smash::phx::Vector3f;

use crate::model::{
    btt,
    lockon::{ItemHost, Target, Tracker},
    practice_games::Game,
    sites_pkg3,
};
use crate::runtime::{
    compat::common, compat::consts::FighterId, game, scene, state_natives as native,
};

const COUNT: usize = 1 + btt::MAX;
const SCALE: f32 = 0.6;
const CENTER_HEIGHT: f32 = 4.0;

static SCENE: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy)]
struct Handle {
    id: u32,
    accessor: *mut Accessor,
}

struct GameItems(RefCell<Tracker<Handle, COUNT>>);
// SAFETY: only P1's primary game-thread callback accesses this cell. A native
// call reentering that callback cannot borrow the items a second time.
unsafe impl Sync for GameItems {}
static LIVE: GameItems = GameItems(RefCell::new(Tracker::new()));

/// The scene clock runs from more than one caller. Only the game thread owns the items.
pub fn scene_changed() {
    SCENE.fetch_add(1, Ordering::AcqRel);
}

extern "C" {
    #[link_name = "\u{1}_ZN3app7sv_item32create_item_init_normal_safe_posENS_8ItemKindEiiRKN3phx8Vector3fES5_fj"]
    fn create_sandbag(
        kind: app::ItemKind,
        variation: i32,
        owner: i32,
        at: &Vector3f,
        speed: &Vector3f,
        lr: f32,
        source: u32,
    ) -> *mut app::Item;
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

    fn valid(&mut self, item: Handle) -> bool {
        unsafe {
            sv_battle_object::is_active(item.id)
                && sv_battle_object::module_accessor(item.id) == item.accessor
                && app::utility::get_category(&mut *item.accessor) == *BATTLE_OBJECT_CATEGORY_ITEM
                && app::utility::get_kind(&mut *item.accessor) == *ITEM_KIND_SANDBAG
        }
    }

    fn create(&mut self, at: [f32; 2]) -> Option<Handle> {
        let (team, owner) = self.team?;
        if !core::mem::take(&mut self.spawn) {
            return None; // Spread a full BTT layout's native creations across frames.
        }
        unsafe {
            let speed = Vector3f {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            };
            let item = create_sandbag(
                app::ItemKind(*ITEM_KIND_SANDBAG),
                0,
                owner as i32,
                &position(at),
                &speed,
                1.0,
                owner,
            );
            if item.is_null() {
                return None;
            }
            let id = native::item_object_id(item as usize);
            let accessor = Item::item_module_accessor(item) as *mut Accessor;
            if accessor.is_null() {
                native::item_remove(self.manager, id);
                return None;
            }
            TeamModule::set_team(accessor, team, true);
            TeamModule::set_hit_team(accessor, team);
            WorkModule::on_flag(accessor, *ITEM_SANDBAG_INSTANCE_WORK_FLAG_NO_POCKET);
            WorkModule::on_flag(accessor, *ITEM_SANDBAG_INSTANCE_WORK_FLAG_NO_LANDING_SMOKE);
            WorkModule::on_flag(accessor, *ITEM_INSTANCE_WORK_FLAG_DISABLE_AUTO_GRAVITY_MOVE);
            JostleModule::set_status(accessor, false);
            PostureModule::set_scale(accessor, SCALE, false);
            Some(Handle { id, accessor })
        }
    }

    fn place(&mut self, item: Handle, at: [f32; 2]) {
        unsafe {
            let accessor = item.accessor;
            PostureModule::set_pos(accessor, &position(at));
            KineticModule::clear_speed_all(accessor);
            VisibilityModule::set_whole(accessor, false);
            EffectModule::set_whole(accessor, false);
            ShadowModule::set_draw_status(accessor, false);
        }
    }

    fn remove(&mut self, item: Handle) {
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
pub unsafe fn frame(fighter: usize) {
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
    if !game::granted(sites_pkg3::STATE_ITEM_OBJECT_ID_OFFSET)
        || !game::granted(sites_pkg3::STATE_ITEM_REMOVE_OFFSET)
    {
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
        SCENE.load(Ordering::Acquire),
        targets,
        &mut NativeItems {
            manager,
            team,
            spawn: true,
        },
    );
}
