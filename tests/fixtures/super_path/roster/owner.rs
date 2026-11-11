// rc.8 R8 fixture: `character_roster.rs` declares its Mii modules from a sibling folder (`#[path = "../mii/…"]`).
#[path = "../mii/far_child.rs"]
mod far_child;

fn owner_service() {}
