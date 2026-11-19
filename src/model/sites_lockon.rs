//! Native calls and the ItemManager reference used by practice lock-on.

use super::guard::SeamKind;
use super::sites::{Feature, Site};

pub static CALLEES: [Site; 19] =
    include!(concat!(env!("OUT_DIR"), "/tables/sites_lockon.CALLEES.rs"));
