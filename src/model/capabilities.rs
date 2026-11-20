//! Native capabilities (review R2-1): every called, hooked or relied-on site row of every table is a
//! capability, and an entry point (a hook, a wrapper) may only run when every capability it can reach is
//! granted. A feature's *closure* is what it needs: its own rows, the Training predicate, the CPU core's
//! rows for the CPU features, and the rows of other features that its entry points call (`CROSS_NEEDS`).
//! Boot installs a feature only when its whole closure passed the guards, and names the first rejected row
//! on the install line (`<feature>=not_installed reason=site:<name>`). The native-call layer (the runtime's
//! `game::natives`, `state_natives`) additionally refuses to call a native whose row was not granted
//! (`Granted`), so a call that is not in a closure (a deliberately optional one) is a no-op, never a call
//! into code boot rejected.
//!
//! `CROSS_NEEDS` is data, not an opinion: `tests/capabilities.rs` scans the runtime source, computes every
//! entry point's reachable rows through the call graph (stopping at the declared readiness boundaries) and
//! fails when the declared closures differ.

use super::guard::SeamKind;
use super::sites::{Feature, Site, SITES};
use super::sites_pkg3::STATE_SITES;
use super::sites_ui::UI_SITES;
use super::{sites_pkg4, sites_pkg6, sites_u4, sites_u4b};

/// Whether a row is a site this plugin writes in code: a compared hook row inside `main`'s `.text`.
pub const fn writes_code(site: &Site) -> bool {
    matches!(site.kind, SeamKind::Hook { .. }) && site.compared() && matches!(super::guard::region_of(site.offset), Some(super::guard::MainRegion::Text))
}

const fn writing(rows: &[Site]) -> usize {
    let (mut n, mut i) = (0, 0);
    while i < rows.len() {
        if writes_code(&rows[i]) {
            n += 1;
        }
        i += 1;
    }
    n
}

/// The rows of `all_sites` that write code (`writes_code`): the seam installer sizes its own-write records with it.
pub const WRITING_SITES: usize = writing(&SITES)
    + writing(&STATE_SITES)
    + writing(&sites_pkg4::SITES)
    + writing(&UI_SITES)
    + writing(&sites_pkg6::SITES)
    + writing(&sites_u4::SITES)
    + writing(&sites_u4b::SITES)
    + writing(&super::sites_hitbox::SITES)
    + writing(&super::developer_debug::SITES)
    + writing(&super::sites_mashout::SITES);

/// Every row of every table, in install-line order.
pub fn all_sites() -> impl Iterator<Item = &'static Site> {
    SITES
        .iter()
        .chain(STATE_SITES.iter())
        .chain(sites_pkg4::SITES.iter())
        .chain(UI_SITES.iter())
        .chain(sites_pkg6::SITES.iter())
        .chain(sites_u4::SITES.iter())
        .chain(sites_u4b::SITES.iter())
        .chain(super::sites_hitbox::SITES.iter())
        .chain(super::developer_debug::SITES.iter())
        .chain(super::sites_mashout::SITES.iter())
        .chain(super::sites_tech_launch::SITES.iter())
        .chain(super::sites_tech_hitlag::SITES.iter())
        .chain(super::sites_tech_surfaces::SITES.iter())
        .chain(super::sites_tech_surfaces::SPEED_NOTICE_SITES.iter())
        .chain(super::sites_lockon::CALLEES.iter())
}

/// The install unit that owns a row: its feature's unit, except that the run-time texture helper's rows (they say `Ui` in their table)
/// are the `UiTexture` unit, so a helper whose rows are rejected leaves the menu installed.
pub fn unit_of_row(site: &Site) -> Feature {
    if sites_u4::SITES.iter().chain(sites_u4b::SITES.iter()).any(|row| row.name == site.name) {
        Feature::UiTexture
    } else {
        unit_of(site.feature)
    }
}

/// The row called `name` in any table.
pub fn find(name: &str) -> Option<&'static Site> {
    all_sites().find(|site| site.name == name)
}

/// The install unit a row's feature belongs to: package 3's five features install together.
pub const fn unit_of(feature: Feature) -> Feature {
    match feature {
        Feature::StateCharge | Feature::StateBuff | Feature::StateItems | Feature::StateRestore => Feature::StateCore,
        other => other,
    }
}

fn optional_row(site: &Site) -> bool {
    super::sites_hitbox::GRADER_ROWS.iter().any(|(name, _)| *name == site.name)
}

const fn is_cpu_unit(unit: Feature) -> bool {
    matches!(unit, Feature::CpuDefence | Feature::CpuLedge | Feature::CpuTech | Feature::CpuShield | Feature::CpuMash | Feature::CpuCommon | Feature::CpuShieldDamage | Feature::CpuCamera)
}

/// The rows a *call* of the function at `offset` relies on. A called or relied-on row says the bytes are the
/// function and tolerates another plugin's hook, which is all a call needs, so when there are such rows they
/// are the call's rows (a hook of ours on the same function is not needed to call it); a function we only ever
/// call through our own hook has just the hook row. Rows with the same guard bytes and the same kind of guard
/// (two packages' tables naming one function) pass and fail together, so one stands for the group.
pub fn call_rows(offset: usize) -> impl Iterator<Item = &'static Site> {
    let writes = |s: &Site| matches!(s.kind, SeamKind::Hook { .. });
    let any_plain = all_sites().any(|s| s.offset == offset && !writes(s));
    all_sites()
        .enumerate()
        .filter(move |(i, s)| {
            s.offset == offset && (!any_plain || !writes(s)) && !all_sites().take(*i).any(|earlier| earlier.offset == offset && writes(earlier) == writes(s) && earlier.guard == s.guard)
        })
        .map(|(_, s)| s)
}
/// Every row that stands for the same guard as `row` (same offset, same kind of guard, same bytes), itself
/// included.
#[cfg(test)]
pub fn equivalent_rows(row: &Site) -> impl Iterator<Item = &'static Site> + '_ {
    let writes = |s: &Site| matches!(s.kind, SeamKind::Hook { .. });
    all_sites().filter(move |s| s.offset == row.offset && writes(s) == writes(row) && s.guard == row.guard)
}

/// The name a call of the function at `offset` goes by in `CROSS_NEEDS` (the first of its rows).
#[cfg(test)]
pub fn call_name(offset: usize) -> Option<&'static str> {
    call_rows(offset).next().map(|s| s.name)
}

/// Rows of other features that a unit's entry points reach (generated by `tests/capabilities.rs`
/// `print_cross_needs`, checked by `cross_needs_match_the_call_graph`), as the first row of each called
/// function.
pub static CROSS_NEEDS: [(Feature, &[&str]); 15] = [
    (Feature::Lockon, &["attack_data", "damage_of", "damage_record_dealt", "damage_record_hit_ids", "damage_record_layout", "fighter_manager_get_entry"]),
    (Feature::BttProjectiles, &["article_get_from_no", "attack_data", "attack_is_attack", "orb_joint_position", "state_article_object_id", "state_object_is_active", "state_object_module_accessor"]),
    (Feature::CpuCommon, &["transition_term_hook", "work_set_float"]),
    (Feature::CpuCore, &["fighter_information", "is_operation_cpu"]),
    (Feature::CpuMash, &["fighter_manager_get_entry", "motion_kind", "transition_term_hook"]),
    (Feature::DeveloperDebug, &["lua_const_lookup", "utility_get_category", "utility_get_kind"]),
    (
        Feature::EdgeJuggle,
        &[
            "attack_data",
            "attack_is_attack",
            "camera_limit_range",
            "cancel_is_enable",
            "damage_of",
            "fighter_information",
            "fighter_manager_get_entry",
            "is_operation_cpu",
            "kinetic_sum_speed_y",
            "motion_kind",
            "orb_joint_position",
            "pos_x",
            "pos_y",
            "situation_kind",
            "stage_id_call",
            "stage_table",
            "state_add_damage",
            "state_clear_speed_all",
            "state_get_param_int",
            "state_heal",
            "state_init_pos",
            "state_set_lr",
            "state_set_pos",
            "state_set_situation_kind",
            "state_set_sleep",
            "state_status_request",
            "state_work_set_int",
            "status_kind",
            "tech_notice_damage_stop",
            "transition_term_hook",
            "utility_get_category",
            "utility_get_kind",
            "work_get_float",
            "work_get_int",
            "work_get_param_float",
            "work_is_flag",
        ],
    ),
    (Feature::Hitbox, &["damage_of", "fighter_manager_get_entry", "lua_const_lookup", "motion_frame", "motion_kind", "stage_id_call", "state_article_object_id", "state_clear_speed_all", "state_heal", "state_item_active_count", "state_item_active_item", "state_item_object_id", "state_lr", "state_object_is_active", "state_set_pos", "state_status_request", "status_kind", "stop_is_stop", "utility_get_category", "utility_get_kind", "work_get_int", "work_get_param_float"]),
    (Feature::InputDisplay, &["attack_data", "attack_is_attack", "attack_power_mul_status", "cancel_is_enable", "damage_of", "damage_reaction", "damage_record_dealt", "damage_record_hit_ids", "damage_record_layout", "fighter_manager_get_entry", "lua_const_lookup", "motion_frame", "prev_status_kind", "situation_kind", "state_lr", "status_kind", "stop_is_stop", "transition_term_hook", "work_get_int", "work_get_param_float", "work_is_flag"]),
    (Feature::NamedMacros, &["fighter_information", "fighter_manager_get_entry", "is_operation_cpu", "state_lr", "ui_is_ready_go", "work_get_int"]),
    (
        Feature::Mashout,
        &[
            "attack_is_attack",
            "camera_limit_range",
            "damage_of",
            "fighter_manager_get_entry",
            "lua_const_lookup",
            "pos_x",
            "pos_y",
            "pos_z",
            "prev_status_kind",
            "situation_kind",
            "state_article_active_num",
            "state_article_generate",
            "state_article_get_article",
            "state_article_is_exist",
            "state_article_object_id",
            "state_article_remove_exist",
            "state_clear_speed_all",
            "state_get_param_int",
            "state_init_pos",
            "state_link_parent_object_id",
            "state_lr",
            "state_object_is_active",
            "state_object_module_accessor",
            "state_set_lr",
            "state_status_request",
            "state_work_get_int64",
            "state_work_set_int",
            "status_kind",
            "stop_is_stop",
            "utility_get_kind",
            "work_get_float",
            "work_get_int",
            "work_get_param_float",
            "work_is_flag",
            "work_set_float",
        ],
    ),
    (Feature::P1Cpu, &["fighter_manager_get_entry", "situation_kind", "utility_get_category"]),
    (Feature::Recording, &["fighter_information", "is_operation_cpu", "pos_x", "pos_y", "situation_kind", "status_kind", "transition_term_hook", "work_get_int", "work_is_flag"]),
    (Feature::StateCore, &["change_status_force", "damage_of", "effect_req", "effect_req_follow", "effect_req_on_joint", "fighter_information", "is_operation_cpu", "lua_const_lookup", "pos_x", "prev_status_kind", "situation_kind", "status_kind", "work_get_param_float", "work_is_flag", "work_set_flag", "work_set_float"]),
    (Feature::TechPractice, &["camera_limit_range", "change_status_force", "damage_of", "fighter_manager_get_entry", "kinetic_get_energy", "lua_const_lookup", "pos_x", "pos_y", "situation_kind", "stage_id_call", "stage_table", "state_add_damage", "state_clear_speed_all", "state_get_param_int", "state_heal", "state_init_pos", "state_lr", "state_set_lr", "state_set_pos", "state_set_situation_kind", "state_set_sleep", "state_work_set_int", "status_kind", "stop_is_stop", "stop_set_hit_stop_frame", "utility_get_kind", "work_get_float", "work_get_int", "work_get_param_float", "work_set_float"]),
];

fn cross_needs(unit: Feature) -> &'static [&'static str] {
    CROSS_NEEDS.iter().find(|(u, _)| *u == unit).map_or(&[], |(_, names)| names)
}

/// The first row of a unit's closure that `usable` rejects, or `None` when every one passed: its own rows, the
/// Training predicate, the CPU core's rows for the CPU features, and every row of each function in
/// `CROSS_NEEDS`.
pub fn blocked_by(unit: Feature, usable: &dyn Fn(&Site) -> bool) -> Option<&'static str> {
    let own = all_sites().find(|s| {
        unit_of_row(s) == unit
            && !optional_row(s)
            && !super::sites_mashout::move_specific(s)
            && !super::sites_tech_surfaces::SPEED_NOTICE_SITES.iter().any(|row| row.name == s.name)
            && !usable(s)
    });
    if let Some(site) = own {
        return Some(site.name);
    }
    cross_blocked_by(unit, usable)
}

/// The part of a closure that is not the unit's own rows: the Training predicate, the CPU core's rows (for the
/// CPU features) and the cross-feature rows. A unit whose own rows install one by one (the notices'
/// observers) asks only this.
pub fn cross_blocked_by(unit: Feature, usable: &dyn Fn(&Site) -> bool) -> Option<&'static str> {
    let shared = all_sites().find(|s| (s.feature == Feature::Training || (is_cpu_unit(unit) && s.feature == Feature::CpuCore)) && !usable(s));
    if let Some(site) = shared {
        return Some(site.name);
    }
    if unit == Feature::Mashout {
        // Execution/layout dependencies, rather than calls made by Mash's own functions.
        for name in ["clatter_start_hook", "entry_active_cell"] {
            if !find(name).is_some_and(usable) {
                return Some(name);
            }
        }
    }
    cross_needs(unit).iter().find_map(|name| {
        let site = find(name)?;
        call_rows(site.offset).find(|row| !usable(row)).map(|row| row.name)
    })
}
/// Whether a unit's closure passed.
pub fn unit_usable(unit: Feature, usable: &dyn Fn(&Site) -> bool) -> bool {
    blocked_by(unit, usable).is_none()
}

/// Mash's existing callbacks and completed roster service must be installed before admitting work.
pub const fn mashout_ready(guards: bool, cpu_core: bool, recording: bool, active_cell: bool, roster_ready: bool) -> bool {
    guards && cpu_core && recording && active_cell && roster_ready
}

/// The offsets of the natives whose rows passed their guards: the native-call layer calls nothing else.
#[derive(Clone, Debug, Default)]
pub struct Granted {
    offsets: Vec<usize>,
}

impl Granted {
    /// Every offset some row names whose call rows are all usable.
    pub fn from_usable(usable: &dyn Fn(&Site) -> bool) -> Granted {
        let mut offsets: Vec<usize> = all_sites().map(|s| s.offset).collect();
        offsets.sort_unstable();
        offsets.dedup();
        offsets.retain(|&offset| call_rows(offset).all(usable));
        Granted { offsets }
    }

    /// Whether the function at `offset` may be called.
    pub fn allows(&self, offset: usize) -> bool {
        self.offsets.binary_search(&offset).is_ok()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }
}
