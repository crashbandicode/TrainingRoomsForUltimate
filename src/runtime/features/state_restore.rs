//! The fighter-specific restores and the held-item restore of the save states (SPEC-state-05,
//! SPEC-state-04, SPEC-state-02 §5.1): Steve's materials and tools, Olimar's Pikmin line, Sora's spell,
//! the player's held item with the turnip face override, and the Pikmin autonomy gate. No allocation, no
//! lock held across a game call. NOT YET SWITCH-RUN.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::model::sites_pkg3 as s;
use crate::model::state::charge;
use crate::model::state::ids::{Table, C, UNRESOLVED};
use crate::model::state::items;
use crate::model::state::record::Steve;
use crate::model::state::steve as steve_layout;
use crate::runtime::alloc::HookScope;
use crate::runtime::game::{self, natives};
use crate::runtime::state_natives as n;

fn id(ids: &Table, c: C) -> Option<i32> {
    ids.resolved(c).then(|| ids.get(c))
}

// ---- Steve ------------------------------------------------------------------------------------------

/// Read Steve's materials and tool records. The tool buffer's address is re-read on every use.
///
/// # Safety
/// The restore sites passed their guards; `accessor` is Steve's.
pub unsafe fn steve_read(accessor: usize, ids: &Table) -> Option<Steve> {
    let buffer = n::work_get_int64(accessor, id(ids, C::SteveExtendBuffer)?) as usize;
    if buffer == 0 {
        return None;
    }
    let mut counts = [0i32; 7];
    for (slot, c) in counts.iter_mut().zip(steve_layout::COUNT_IDS) {
        *slot = n::work_get_int(accessor, id(ids, c)?);
    }
    let mut tools = [(0u8, 0.0f32); steve_layout::TOOLS];
    for (i, tool) in tools.iter_mut().enumerate() {
        let at = buffer + steve_layout::tool_offset(i);
        *tool = (
            ((at + steve_layout::TOOL_MATERIAL_OFFSET) as *const u8).read(),
            ((at + steve_layout::TOOL_DURABILITY_OFFSET) as *const f32).read(),
        );
    }
    Some(steve_layout::from_parts(&steve_layout::Parts { counts, tools }))
}

/// Write Steve's materials and tools, then refresh the HUD bar with seven zero-subtractions.
///
/// # Safety
/// As `steve_read`.
pub unsafe fn steve_write(accessor: usize, ids: &Table, steve: &Steve) {
    let parts = steve_layout::parts(steve);
    for (value, c) in parts.counts.iter().zip(steve_layout::COUNT_IDS) {
        if let Some(slot) = id(ids, c) {
            n::work_set_int(accessor, *value, slot);
        }
    }
    if let Some(slot) = id(ids, C::SteveExtendBuffer) {
        let buffer = n::work_get_int64(accessor, slot) as usize;
        if buffer != 0 {
            for (i, (material, durability)) in parts.tools.iter().enumerate() {
                let at = buffer + steve_layout::tool_offset(i);
                ((at + steve_layout::TOOL_MATERIAL_OFFSET) as *mut u8).write(*material);
                ((at + steve_layout::TOOL_DURABILITY_OFFSET) as *mut f32).write(*durability);
            }
        }
    }
    for material in 0..7 {
        n::sub_material_num(accessor, material);
    }
}

// ---- Olimar -----------------------------------------------------------------------------------------

/// The troops manager's work int64 id (SPEC-state-02 §5.1) and its layout at 13.0.5.
const TROOPS_MANAGER_ID: i32 = 0x100000C0;
const TROOPS_FOLLOW_COUNT: usize = 0x10;
const TROOPS_FOLLOWERS: usize = 0x20;
const TROOPS_HELD_COUNT: usize = 0x80;
/// The battle object's id field (the `sv_battle_object` lookups take it).
const BATTLE_OBJECT_ID_FIELD: usize = 0x8;

unsafe fn pikmin_colour(object_id: u32, ids: &Table) -> Option<i32> {
    if !n::object_is_active(object_id) {
        return None;
    }
    let accessor = n::object_accessor(object_id);
    if accessor == 0 {
        return None;
    }
    Some(n::work_get_int(accessor, id(ids, C::PikminVariation)?))
}

/// The colours of Olimar's Pikmin in line order, held first (SPEC-state-02 §5.1).
///
/// # Safety
/// As `steve_read`; `accessor` is Olimar's.
pub unsafe fn olimar_line(accessor: usize, ids: &Table) -> [Option<i32>; 3] {
    let manager = n::work_get_int64(accessor, TROOPS_MANAGER_ID) as usize;
    if manager == 0 {
        return [None; 3];
    }
    let mut held = [None; 3];
    let held_count = (((manager + TROOPS_HELD_COUNT) as *const u32).read() as usize).min(3);
    for (i, c) in [C::OlimarHold0, C::OlimarHold1, C::OlimarHold2].into_iter().enumerate().take(held_count) {
        if let Some(slot) = id(ids, c) {
            let object = n::work_get_int(accessor, slot);
            if object > 0 {
                held[i] = pikmin_colour(object as u32, ids);
            }
        }
    }
    let mut following = [None; 3];
    let follow_count = (((manager + TROOPS_FOLLOW_COUNT) as *const u32).read() as usize).min(3);
    for (i, slot) in following.iter_mut().enumerate().take(follow_count) {
        let object = ((manager + TROOPS_FOLLOWERS + 8 * i) as *const usize).read();
        if object != 0 {
            let object_id = ((object + BATTLE_OBJECT_ID_FIELD) as *const u32).read();
            *slot = pikmin_colour(object_id, ids);
        }
    }
    charge::line_order(&held, &following)
}

/// Remove Olimar's Pikmin, respawn the saved line in order and order the followers to follow at once.
///
/// # Safety
/// As `olimar_line`.
pub unsafe fn olimar_respawn(accessor: usize, ids: &Table, line: &[Option<i32>; 3]) {
    let (Some(article), Some(target), Some(pre), Some(before)) = (
        id(ids, C::ArticleOlimarPikmin),
        id(ids, C::ArticleAllTarget),
        id(ids, C::OlimarPreVariation),
        id(ids, C::OlimarBeforePreVariation),
    ) else {
        return;
    };
    n::article_remove_exist(accessor, article, target);
    // Gated on the active count being exactly 0, 1 and 2 in turn, so a failed spawn does not shift the
    // order of the rest.
    for _ in 0..3 {
        let active = n::article_active_num(accessor, article);
        let Some(colour) = charge::spawn_colour(active, line) else { break };
        n::work_set_int(accessor, charge::prev_colour(colour), pre);
        n::work_set_int(accessor, charge::prev_colour(charge::prev_colour(colour)), before);
        n::article_generate(accessor, article, -1);
    }
    let manager = n::work_get_int64(accessor, TROOPS_MANAGER_ID) as usize;
    if manager == 0 {
        return;
    }
    let Some(follow) = id(ids, C::PikminStatusAirFollow) else { return };
    let follow_count = (((manager + TROOPS_FOLLOW_COUNT) as *const u32).read() as usize).min(3);
    for i in 0..follow_count {
        let object = ((manager + TROOPS_FOLLOWERS + 8 * i) as *const usize).read();
        if object != 0 {
            let pikmin = n::object_accessor(((object + BATTLE_OBJECT_ID_FIELD) as *const u32).read());
            if pikmin != 0 {
                n::status_request(pikmin, follow);
            }
        }
    }
}

// ---- Sora -------------------------------------------------------------------------------------------

/// Make `saved` the current spell: write `(saved + 1) mod 3`, raise the change flag, and let the game's
/// routine advance one spell and redraw the HUD.
///
/// # Safety
/// As `steve_read`; `fighter` is Sora's fighter object.
pub unsafe fn sora_spell(accessor: usize, fighter: usize, ids: &Table, saved: i32) {
    if let (Some(kind), Some(flag)) = (id(ids, C::SoraMagicKind), id(ids, C::SoraChangeMagicFlag)) {
        n::work_set_int(accessor, charge::sora_written_kind(saved), kind);
        natives::set_flag(accessor, true, flag);
        n::sora_change_magic(fighter);
    }
}

// ---- the held item ----------------------------------------------------------------------------------

/// The turnip face override (`0` none, else the face 1..=8), read by the sixteen probability hooks.
static TURNIP_CHOSEN: AtomicU32 = AtomicU32::new(0);

/// The turnip face override now (package 6's character items read it around their own grant).
pub fn chosen_face() -> Option<u32> {
    face()
}

/// Set (or clear) the turnip face override (package 6's character items, around their own grant).
pub fn set_chosen_face(face: Option<u32>) {
    TURNIP_CHOSEN.store(face.unwrap_or(0), Ordering::Release);
}

fn face() -> Option<u32> {
    match TURNIP_CHOSEN.load(Ordering::Acquire) {
        0 => None,
        face => Some(face),
    }
}

macro_rules! prob_hooks {
    ($($name:ident = $offset:ident, $n:literal;)*) => {
        $(
            #[skyline::hook(offset = s::$offset)]
            pub unsafe fn $name() -> f32 {
                let _scope = HookScope::enter();
                match items::face_probability($n, face()) {
                    Some(probability) => probability,
                    None => call_original!(),
                }
            }
        )*

        /// Install the sixteen probability hooks.
        pub fn install_probability_hooks() {
            $( skyline::install_hook!($name); )*
        }
    };
}

prob_hooks! {
    daisy_prob_1 = STATE_DAISY_PROB_1_OFFSET, 1;
    daisy_prob_2 = STATE_DAISY_PROB_2_OFFSET, 2;
    daisy_prob_3 = STATE_DAISY_PROB_3_OFFSET, 3;
    daisy_prob_4 = STATE_DAISY_PROB_4_OFFSET, 4;
    daisy_prob_5 = STATE_DAISY_PROB_5_OFFSET, 5;
    daisy_prob_6 = STATE_DAISY_PROB_6_OFFSET, 6;
    daisy_prob_7 = STATE_DAISY_PROB_7_OFFSET, 7;
    daisy_prob_8 = STATE_DAISY_PROB_8_OFFSET, 8;
    peach_prob_1 = STATE_PEACH_PROB_1_OFFSET, 1;
    peach_prob_2 = STATE_PEACH_PROB_2_OFFSET, 2;
    peach_prob_3 = STATE_PEACH_PROB_3_OFFSET, 3;
    peach_prob_4 = STATE_PEACH_PROB_4_OFFSET, 4;
    peach_prob_5 = STATE_PEACH_PROB_5_OFFSET, 5;
    peach_prob_6 = STATE_PEACH_PROB_6_OFFSET, 6;
    peach_prob_7 = STATE_PEACH_PROB_7_OFFSET, 7;
    peach_prob_8 = STATE_PEACH_PROB_8_OFFSET, 8;
}

// ---- the Pikmin autonomy gate (SPEC-state-05 §5.3) --------------------------------------------------

/// The statuses between which a Pikmin is thrown or attacking; set at boot once the constants resolve.
static PIKMIN_FIRST: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(UNRESOLVED);
static PIKMIN_LAST: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(UNRESOLVED);

pub fn set_pikmin_statuses(first: i32, last: i32) {
    PIKMIN_FIRST.store(first, Ordering::Release);
    PIKMIN_LAST.store(last, Ordering::Release);
}

/// The Pikmin 'activate autonomy' routine `(weapon, weapon work module)`: in Training, only a thrown or
/// attacking Pikmin runs it, so idle Pikmin never go autonomous and lose their order.
#[skyline::hook(offset = s::STATE_PIKMIN_AUTONOMY_OFFSET)]
pub unsafe fn state_pikmin_autonomy_hook(weapon: usize, work: usize) -> u64 {
    let _scope = HookScope::enter();
    let (first, last) = (PIKMIN_FIRST.load(Ordering::Acquire), PIKMIN_LAST.load(Ordering::Acquire));
    if first != UNRESOLVED && last != UNRESOLVED && work != 0 && game::is_training_mode() {
        let accessor = ((work + 8) as *const usize).read();
        if accessor != 0 {
            let status = natives::status_kind(accessor);
            if status < first || status > last {
                return 0;
            }
        }
    }
    call_original!(weapon, work)
}

// ---- Pokemon Trainer (SPEC-state-05 §5.1) ----------------------------------------------------------

/// The resolved constants of the trainer rules (any may be `UNRESOLVED`: the rule is then skipped).
#[derive(Clone, Copy, Debug)]
pub struct PtIds {
    pub link: i32,
    pub enable_flag: i32,
    pub mball: i32,
    pub all_target: i32,
    pub run: i32,
    pub ground: i32,
    pub air: i32,
}

/// The accessor of a Pokemon's trainer (`LinkModule::get_parent_object_id` then the battle object's module
/// accessor); 0 when the link does not resolve or the object is not active.
///
/// # Safety
/// The restore sites passed their guards; `accessor` is a live fighter's.
pub unsafe fn trainer_of(accessor: usize, link: i32) -> usize {
    if link == UNRESOLVED {
        return 0;
    }
    let id = n::link_parent_object_id(accessor, link);
    if id < 0 {
        return 0;
    }
    let id = id as u32;
    if !n::object_is_active(id) {
        return 0;
    }
    n::object_accessor(id)
}

/// The pre-death rule (KillPlayer and every WaitForAlive frame): the trainer may not switch, its animation
/// runs at 1000x, and its Poke Ball (when it exists) runs at 1000x and is hidden.
///
/// # Safety
/// As `trainer_of`.
pub unsafe fn pt_predeath(accessor: usize, ids: &PtIds) {
    let trainer = trainer_of(accessor, ids.link);
    if trainer == 0 {
        return;
    }
    if ids.enable_flag != UNRESOLVED {
        n::work_off_flag(trainer, ids.enable_flag);
    }
    n::motion_set_rate(trainer, 1000.0);
    if ids.mball != UNRESOLVED && n::article_is_exist(trainer, ids.mball) {
        n::article_set_rate(trainer, ids.mball, 1000.0);
        if ids.all_target != UNRESOLVED {
            n::article_hide_whole(trainer, ids.mball, ids.all_target);
        }
    }
}

/// After placement: the trainer may switch again.
///
/// # Safety
/// As `trainer_of`.
pub unsafe fn pt_raise_switch(accessor: usize, ids: &PtIds) {
    let trainer = trainer_of(accessor, ids.link);
    if trainer != 0 && ids.enable_flag != UNRESOLVED {
        n::work_on_flag(trainer, ids.enable_flag);
    }
}

/// What one frame of the trainer wait asked for, applied to the trainer's accessor.
///
/// # Safety
/// `trainer` is a live trainer weapon's accessor.
pub unsafe fn pt_apply(trainer: usize, ids: &PtIds, out: &crate::model::state::pokemon::PtOut) {
    if out.rate_one {
        n::motion_set_rate(trainer, 1.0);
    }
    if out.ground_and_run && ids.ground != UNRESOLVED && ids.run != UNRESOLVED {
        n::set_situation_kind(trainer, ids.ground);
        n::status_force(trainer, ids.run, false);
    }
}
