//! Native calls and the ItemManager reference used by practice lock-on.

use super::guard::SeamKind;
use super::sites::{Feature, Site};

pub const ITEM_FACTORY_OFFSET: usize = 0x15db0b0;
pub const OBJECT_POOLS_SLOT: usize = 0x52b8548;

pub static CALLEES: [Site; 25] =
    include!(concat!(env!("OUT_DIR"), "/tables/sites_lockon.CALLEES.rs"));
