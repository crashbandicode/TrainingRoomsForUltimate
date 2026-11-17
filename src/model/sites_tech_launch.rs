//! Production grants for the shared tech launcher; same owned windows as S0.
use super::{
    guard::SeamKind,
    sites::{Feature, Site},
};
#[path = "../runtime/features/tech_reaction_guard.rs"]
pub mod core;
#[path = "../runtime/features/tech_decision_guards.rs"]
pub mod decision;
#[path = "../runtime/features/tech_direct_v1_guards.rs"]
pub mod v1;
#[path = "../runtime/features/tech_direct_v3_guards.rs"]
pub mod v3;

pub const MAGNITUDE_OFFSET: usize = 0x403a20;
pub const REACTION_OFFSET: usize = 0x404ec0;
pub const COS_OFFSET: usize = 0x39c07d0;
pub const SIN_OFFSET: usize = 0x39c07e0;
pub const OVERWRITE_REACTION_OFFSET: usize = 0x200b8b0;
pub const IS_DAMAGE_OFFSET: usize = 0x2089220;
pub const SPEED_X_OFFSET: usize = 0x20f6460;
pub const SPEED_Y_OFFSET: usize = 0x20f6480;
pub const GET_ENERGY_OFFSET: usize = 0x2043970; // Existing NamedMacros grant.
pub const GUARD_COUNT_OFFSET: usize = 0x3f8620;
pub const GUARD_PREVIOUS_OFFSET: usize = 0x3f8630;
pub const STICK_X_OFFSET: usize = 0x3f84a0;
const fn window(offset: usize) -> [u8; 64] {
    let groups = [
        v1::DIRECT_V1_GUARDS,
        v3::DIRECT_V3_GUARDS,
        core::CORE_GUARDS,
        decision::GUARDS,
    ];
    let mut g = 0;
    while g < groups.len() {
        let mut i = 0;
        while i < groups[g].len() {
            if groups[g][i].0 == offset {
                return groups[g][i].1;
            }
            i += 1;
        }
        g += 1;
    }
    panic!("missing owned tech window")
}
const fn site(name: &'static str, offset: usize) -> Site {
    Site {
        name,
        feature: Feature::TechPractice,
        offset,
        kind: SeamKind::Called,
        guard: window(offset),
        note: "Shared tech launch native; concrete type checks remain mandatory.",
    }
}
pub const SITES: [Site; 11] = [
    site("tech_magnitude", MAGNITUDE_OFFSET),
    site("tech_reaction", REACTION_OFFSET),
    site("tech_cos", COS_OFFSET),
    site("tech_sin", SIN_OFFSET),
    site("tech_overwrite_reaction", OVERWRITE_REACTION_OFFSET),
    site("tech_is_damage", IS_DAMAGE_OFFSET),
    site("tech_speed_x", SPEED_X_OFFSET),
    site("tech_speed_y", SPEED_Y_OFFSET),
    site("tech_guard_count", GUARD_COUNT_OFFSET),
    site("tech_guard_previous", GUARD_PREVIOUS_OFFSET),
    site("tech_stick_x", STICK_X_OFFSET),
];
