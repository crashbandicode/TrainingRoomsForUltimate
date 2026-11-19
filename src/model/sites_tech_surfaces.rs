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

pub const PASSIVE_GROUND_OFFSET: usize = 0x696910;
pub const DAMAGE_STOP_OFFSET: usize = 0x20ad720;
pub const SPEED_NOTICE_SITES: [Site; 2] = [
    Site {
        name: "tech_notice_passive_ground",
        feature: Feature::TechPractice,
        offset: PASSIVE_GROUND_OFFSET,
        kind: SeamKind::Called,
        guard: windows::MATERIAL[0].2,
        note: "Optional read-only floor material observation for the Tech speed notice.",
    },
    Site {
        name: "tech_notice_damage_stop",
        feature: Feature::TechPractice,
        offset: DAMAGE_STOP_OFFSET,
        kind: SeamKind::Called,
        guard: [
            0xf3,0x0f,0x1e,0xf8,0xfd,0x7b,0x01,0xa9,0xfd,0x43,0x00,0x91,0x13,0x48,0x40,0xf9,
            0x68,0x06,0x40,0xf9,0x00,0x21,0x40,0xf9,0x08,0x00,0x40,0xf9,0x08,0x89,0x40,0xf9,
            0x00,0x01,0x3f,0xd6,0x1f,0x1c,0x03,0x71,0x4c,0x02,0x00,0x54,0x08,0x1c,0x01,0x51,
            0x1f,0x15,0x00,0x71,0x22,0x01,0x00,0x54,0x68,0x06,0x40,0xf9,0x00,0x29,0x40,0xf9,
        ],
        note: "Optional read-only damage-stop observation for the Tech speed notice.",
    },
];
