//! The carried CPU-as-P1 helpers whose consumer is the Switch-only `runtime::p1::cpu_pilot` (it loads them as its child modules by
//! `#[path]`, as the fork does): the pilot core (`src/model/p1/mod.rs` with its config, move observer and tests) and the menu and
//! stage session arbitration. They run here by path on the host, with the playbook's by-path include `allow`.

// rc.9: the pilot core is compiled on the host by this crate only, so its unit tests (wrapped in `lib_unit_tests!`) run here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => { $($body)* };
}
#[path = "../src/test_table.rs"]
mod test_table;
#[allow(dead_code)]
#[path = "../src/model/p1/mod.rs"]
mod pilot_core;

#[allow(dead_code)]
#[path = "../src/model/p1_session/menu_session.rs"]
mod menu_session;

#[allow(dead_code)]
#[path = "../src/model/p1_session/stage_session.rs"]
mod stage_session;