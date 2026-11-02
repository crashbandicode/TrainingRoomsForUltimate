//! The pure session helpers of the carried CPU-as-P1 adapter (`src/model/p1/{menu_session,stage_session}.rs`), run by path until
//! `cpu_pilot.rs`, which constructs every variant, is declared. The `allow` is the playbook's by-path include pattern.

#[allow(dead_code)]
#[path = "../src/model/p1/menu_session.rs"]
mod menu_session;

#[allow(dead_code)]
#[path = "../src/model/p1/stage_session.rs"]
mod stage_session;