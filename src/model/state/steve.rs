//! Steve's materials and tool records (SPEC-state-05 §5.4): the layout of what a Save reads and a Load
//! writes. Pure; the runtime does the raw reads and writes through the tool buffer pointer, which it
//! re-reads on every use (a stored address from another Training session crashes).

use super::ids::C;
use super::record::Steve;

/// The seven material counts, in the record's order, and the work int each lives in.
pub const COUNT_IDS: [C; 7] = [
    C::SteveMatG1,
    C::SteveMatWood,
    C::SteveMatStone,
    C::SteveMatIron,
    C::SteveMatGold,
    C::SteveMatRedstone,
    C::SteveMatDiamond,
];

/// Tool records: sword, axe, pick, shovel, each `TOOL_STRIDE` bytes into the buffer, the material kind a
/// byte at `+0`, the durability an `f32` at `+4`.
pub const TOOLS: usize = 4;
pub const TOOL_STRIDE: usize = 0xC;
pub const TOOL_MATERIAL_OFFSET: usize = 0;
pub const TOOL_DURABILITY_OFFSET: usize = 4;

/// The values of a Steve record as the game stores them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parts {
    pub counts: [i32; 7],
    pub tools: [(u8, f32); TOOLS],
}

pub fn parts(steve: &Steve) -> Parts {
    Parts {
        counts: [steve.mat_g1, steve.mat_wood, steve.mat_stone, steve.mat_iron, steve.mat_gold, steve.mat_redstone, steve.mat_diamond],
        tools: [
            (steve.sword_mat as u8, steve.sword_durability),
            (steve.axe_mat as u8, steve.axe_durability),
            (steve.pick_mat as u8, steve.pick_durability),
            (steve.shovel_mat as u8, steve.shovel_durability),
        ],
    }
}

pub fn from_parts(parts: &Parts) -> Steve {
    let [g1, wood, stone, iron, gold, redstone, diamond] = parts.counts;
    let [sword, axe, pick, shovel] = parts.tools;
    Steve {
        mat_g1: g1,
        mat_wood: wood,
        mat_stone: stone,
        mat_iron: iron,
        mat_gold: gold,
        mat_redstone: redstone,
        mat_diamond: diamond,
        sword_mat: u32::from(sword.0),
        sword_durability: sword.1,
        axe_mat: u32::from(axe.0),
        axe_durability: axe.1,
        pick_mat: u32::from(pick.0),
        pick_durability: pick.1,
        shovel_mat: u32::from(shovel.0),
        shovel_durability: shovel.1,
    }
}

/// The byte offset of tool `i`'s record in the buffer.
pub const fn tool_offset(i: usize) -> usize {
    i * TOOL_STRIDE
}
