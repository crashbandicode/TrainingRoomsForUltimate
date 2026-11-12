//! SPEC-state-02: which fields each fighter's charge lives in, what a Save reads and what a Load writes
//! and shows (host tests; the planned names are the spec's §9 rows).

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::collections::HashMap;

use model::state::charge::{self, Joint, Op, Step, Work};
use model::state::ids::{Table, C};
use model::state::record::{Charge, Robin};

/// Every constant is `1000 + index`, so a work id is its own key in the mock.
fn ids() -> Table {
    Table::sequential(1000)
}

#[derive(Default)]
struct Mock {
    ints: HashMap<i32, i32>,
    floats: HashMap<i32, f32>,
    flags: HashMap<i32, bool>,
}

impl Work for Mock {
    fn get_int(&self, id: i32) -> i32 {
        self.ints.get(&id).copied().unwrap_or(-12345)
    }
    fn get_float(&self, id: i32) -> f32 {
        self.floats.get(&id).copied().unwrap_or(-1.5)
    }
    fn get_flag(&self, id: i32) -> bool {
        self.flags.get(&id).copied().unwrap_or(false)
    }
}

fn kind(c: C) -> i32 {
    ids().get(c)
}

fn id(c: C) -> i32 {
    ids().get(c)
}

fn steps(kind_c: C, charge: &Charge) -> Vec<Step> {
    charge::plan_write(&ids(), kind(kind_c), charge).iter().collect()
}

fn state02_read_table_per_kind() {
    let ids = ids();
    let mut work = Mock::default();
    work.ints.insert(id(C::SamusShotCount), 112);
    work.ints.insert(id(C::MewtwoCharge), 40);
    work.ints.insert(id(C::MewtwoPrevCharge), 120);
    work.flags.insert(id(C::MewtwoHad), true);
    work.floats.insert(id(C::RobGyro), 90.0);
    work.floats.insert(id(C::RobBeam), 1.0);
    work.floats.insert(id(C::RobBurner), 2.0);
    let samus = charge::read(&ids, kind(C::KindSamus), &work);
    assert_eq!(samus, Charge { int_x: Some(112), ..Charge::default() });
    let mewtwo = charge::read(&ids, kind(C::KindMewtwo), &work);
    assert_eq!((mewtwo.int_x, mewtwo.int_y, mewtwo.has_charge), (Some(40), Some(120), Some(true)));
    let rob = charge::read(&ids, kind(C::KindRob), &work);
    assert_eq!((rob.float_x, rob.float_y, rob.float_z), (Some(1.0), Some(90.0), Some(2.0)));
    // Every other kind reads nothing.
    assert_eq!(charge::read(&ids, kind(C::KindMario) + 5000, &work), Charge::default());
    assert!(!charge::has_entry(&ids, 7));
    // Dark Samus shares Samus's field; Robin reads eight values.
    assert_eq!(charge::read(&ids, kind(C::KindSamusD), &work).int_x, Some(112));
    work.ints.insert(id(C::RobinThunderKind), 3);
    work.floats.insert(id(C::RobinRed), 0.5);
    work.flags.insert(id(C::RobinLevinOn), true);
    let robin = charge::read(&ids, kind(C::KindRobin), &work).robin.unwrap();
    assert_eq!((robin.thunder_kind, robin.red_book, robin.has_levin), (3, 0.5, true));
}

fn state02_sheik_needle_groups() {
    assert_eq!(charge::sheik_group(6), "group_main_default");
    assert_eq!(charge::sheik_group(5), "group_main_5");
    assert_eq!(charge::sheik_group(1), "group_main_1");
    assert_eq!(charge::sheik_group(0), "group_main_0");
    assert_eq!(charge::sheik_group(9), "group_main_0");
    let plan = steps(C::KindSheik, &Charge { int_x: Some(6), ..Charge::default() });
    assert!(plan.contains(&Step::SheikNeedles(6)));
    assert!(plan.contains(&Step::Visual(Op::ChargeMax)), "6 needles show the flash");
    assert!(!steps(C::KindSheik, &Charge { int_x: Some(5), ..Charge::default() }).contains(&Step::Visual(Op::ChargeMax)));
}

fn state02_robin_tomes() {
    let robin = Robin { thunder_kind: 3, yellow_book: 7, red_book: 2.5, green_book: 6, purple_book: 5, levin_charge: 4, levin_recharge: 9, has_levin: true };
    let plan = steps(C::KindRobin, &Charge { robin: Some(robin), ..Charge::default() });
    assert_eq!(plan.iter().filter(|s| matches!(s, Step::SetInt(..) | Step::SetFloat(..) | Step::SetFlag(..))).count(), 8);
    assert!(plan.contains(&Step::SetFloat(C::RobinRed, 2.5)));
    // Thoron: the flash and the unlimited follow effect on handl.
    assert!(plan.contains(&Step::Visual(Op::ChargeMax)));
    assert!(plan.iter().any(|s| matches!(s, Step::Visual(Op::Follow { effect: "reflet_thunder_max", joint: Joint::Name("handl"), lifetime: -1, .. }))));
    let other = steps(C::KindRobin, &Charge { robin: Some(Robin { thunder_kind: 2, ..robin }), ..Charge::default() });
    assert!(!other.contains(&Step::Visual(Op::ChargeMax)));
}

fn state02_incineroar_revenge() {
    let plan = steps(C::KindIncineroar, &Charge { has_charge: Some(true), float_x: Some(50.0), ..Charge::default() });
    assert!(plan.contains(&Step::SetFlag(C::IncineroarIsRevenge, true)));
    assert!(plan.contains(&Step::SetInt(C::IncineroarRevengeTimer, 3500)), "a fresh full timer");
    assert_eq!(plan.iter().filter(|s| matches!(s, Step::RevengeAura(_))).count(), 5);
    let off = steps(C::KindIncineroar, &Charge { has_charge: Some(false), ..Charge::default() });
    assert!(!off.iter().any(|s| matches!(s, Step::RevengeAura(_) | Step::SetInt(C::IncineroarRevengeTimer, _))));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("state02_read_table_per_kind", state02_read_table_per_kind as fn()),
        ("state02_sheik_needle_groups", state02_sheik_needle_groups as fn()),
        ("state02_robin_tomes", state02_robin_tomes as fn()),
        ("state02_incineroar_revenge", state02_incineroar_revenge as fn()),
    ]);
}
