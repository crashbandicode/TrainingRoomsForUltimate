//! Reviewed stage-surface grants. Core integration registers this table with all_sites.
use super::{
    guard::SeamKind,
    sites::{Feature, Site},
};
#[path = "../runtime/features/tech_surfaces_guards.rs"]
pub mod windows;
const fn site(name: &'static str, index: usize) -> Site {
    let (_, offset, called, guard) = windows::SITES[index];
    Site {
        name, feature: Feature::TechPractice, offset,
        kind: if called { SeamKind::Called } else { SeamKind::Relied }, guard,
        note: "Stage surface ABI; ephemeral borrowed line and concrete virtual target checks remain mandatory.",
    }
}
pub const SITES: [Site; 17] = [
    site("tech_surface_touch_line", 0),
    site("tech_surface_near_next_line", 1),
    site("tech_surface_left_pos_layout", 2),
    site("tech_surface_right_pos_layout", 3),
    site("tech_surface_normal_layout", 4),
    site("tech_surface_floor_bits_layout", 5),
    site("tech_surface_passable_bits_layout", 6),
    site("tech_surface_wall_bits_layout", 7),
    site("tech_surface_ray_line_hit", 8),
    site("tech_surface_dead_area_layout", 9),
    site("tech_surface_touch_line_concrete", 10),
    site("tech_surface_touch_line_lifetime", 11),
    site("tech_surface_ray_line_concrete", 12),
    site("tech_surface_ray_collision_scan", 13),
    site("tech_surface_ray_line_layout", 14),
    site("tech_surface_ray_hit_store", 15),
    site("tech_surface_fighter_ground_initializer", 16),
];
