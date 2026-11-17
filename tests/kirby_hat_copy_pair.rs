//! SOL-R23 (kh2-repro-xps-1): a Kirby hat request whose native hat pair was released must wait, not copy.

macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}

#[path = "../src/model/kirby_copy_pair.rs"]
#[allow(dead_code)]
mod kirby_copy_pair;

use kirby_copy_pair::{copy_ready, Pair};

fn hats_source() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/roster/kirby_hats.rs")).unwrap()
}

#[test]
fn the_crash_state_waits() {
    // frame 43268: Falco from source slot 7, record found, pair {0, 0}.
    assert!(!copy_ready(Pair::Object(0)));
    assert!(copy_ready(Pair::Object(0x11fc_c994_00)));
}

#[test]
fn the_hat_request_checks_the_pair_before_it_leases_or_copies() {
    let code = hats_source();
    let gate = code.find("kirby_copy_pair::copy_ready(copy_pair(body, pending.kind))")
        .expect("on_frame does not check the native copy pair");
    let prepare = code.find("match sources::prepare(role, pending.kind,").expect("prepare call");
    let copy = code.find("training::charge::setup_copy_ability(body, slot, pending.kind as i32)").expect("copy call");
    assert!(gate < prepare && prepare < copy, "the pair check must precede the lease and the copy");
    let waiting = &code[gate..prepare];
    assert!(waiting.contains("state.pending = Some(pending);") && waiting.contains("return;"),
        "a missing pair keeps the request pending (the existing timeout ends it)");
    assert!(!waiting.contains("clear_pending"), "a missing pair is not a refusal");
}
