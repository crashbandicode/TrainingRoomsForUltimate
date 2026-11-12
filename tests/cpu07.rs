//! SPEC-cpu-07 (frame advantage, the frame-counter service and the reset bundle). The counters and the
//! hub are tested in `tests/cpu_core.rs` (`cpu07_should_delay`, `cpu07_tick_types_and_reset`,
//! `cpu07_single_tick_per_frame`, `state06_hub_order_clears_cpu_state`); this file has the measurement.

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::cpu::advantage::{format_advantage, Tint, NOTIFICATION_FRAMES};
use model::cpu::state::Cpu;
use model::lra_reset::HubStep;
use model::settings::Snapshot;

fn cpu_on() -> Cpu {
    let mut snap = Snapshot::factory();
    snap.set("frame_advantage", &[1, 0]).unwrap();
    Cpu::new(model::cpu::settings::CpuSettings::from_snapshot(&snap), 3)
}

/// Run an exchange: the player is actionable from frame `p`, the CPU from frame `c`; returns what was
/// shown and on which frame.
fn exchange(cpu: &mut Cpu, p: u32, c: u32) -> Vec<(u32, i32)> {
    let mut shown = Vec::new();
    for frame in 0..(p.max(c) + 5) {
        cpu.tick_clock();
        if let Some(a) = cpu.frame_advantage(frame >= p, frame >= c) {
            shown.push((frame, a));
        }
    }
    shown
}

fn cpu07_frame_advantage_math() {
    // The player recovers first: positive; the CPU first: negative; together: +0.
    let mut cpu = cpu_on();
    // Both start unactionable (frame 0 is not actionable for either when p, c > 0).
    assert_eq!(exchange(&mut cpu, 10, 15), vec![(15, 5)]);
    let mut cpu = cpu_on();
    assert_eq!(exchange(&mut cpu, 20, 12), vec![(20, -8)]);
    let mut cpu = cpu_on();
    assert_eq!(exchange(&mut cpu, 8, 8), vec![(8, 0)]);
    // One measurement per exchange: nothing is shown again while both stay actionable.
    let mut cpu = cpu_on();
    let first = exchange(&mut cpu, 4, 6);
    assert_eq!(first.len(), 1);
    for _ in 0..30 {
        cpu.tick_clock();
        assert_eq!(cpu.frame_advantage(true, true), None);
    }
    // Colours and text.
    assert_eq!((Tint::of(-1).rgba(), Tint::of(0).rgba(), Tint::of(3).rgba()), ((200, 8, 8, 255), (0, 0, 0, 255), (31, 198, 0, 255)));
    let mut buf = [0u8; 12];
    for (value, text) in [(5, "+5"), (-12, "-12"), (0, "+0"), (123, "+123")] {
        let n = format_advantage(value, &mut buf);
        assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), text);
    }
    assert_eq!(NOTIFICATION_FRAMES, 60);
}

fn cpu07_frame_advantage_gates() {
    // Off: no work, nothing shown; a mash set aborts a measurement in progress; no state kept.
    let mut off = Cpu::factory(1);
    for frame in 0..40 {
        off.tick_clock();
        assert_eq!(off.frame_advantage(frame > 10, frame > 20), None);
    }
    assert!(!off.advantage.counting);
    let mut cpu = cpu_on();
    cpu.tick_clock();
    cpu.frame_advantage(false, false);
    assert!(cpu.advantage.counting);
    let mut snap = Snapshot::factory();
    snap.set("frame_advantage", &[1, 0]).unwrap();
    let mut mash = vec![0u8; 30];
    mash[18] = 1;
    snap.set("mash_state", &mash).unwrap();
    cpu.publish(model::cpu::settings::CpuSettings::from_snapshot(&snap), 0);
    assert_eq!(cpu.frame_advantage(true, true), None);
    assert!(!cpu.advantage.counting, "a non-empty Mash Toggles row aborts the measurement");
}

fn cpu07_reset_bundle_clears_the_measurement() {
    let mut cpu = cpu_on();
    cpu.tick_clock();
    cpu.frame_advantage(false, false);
    assert!(cpu.advantage.counting);
    model::lra_reset::run_hub(|step| cpu.hub_step(step));
    assert!(!cpu.advantage.counting);
    // And a session clear.
    cpu.tick_clock();
    cpu.frame_advantage(false, false);
    cpu.clear_session();
    assert!(!cpu.advantage.counting);
    let _ = HubStep::Combo;
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("cpu07_frame_advantage_math", cpu07_frame_advantage_math as fn()),
        ("cpu07_frame_advantage_gates", cpu07_frame_advantage_gates as fn()),
        ("cpu07_reset_bundle_clears_the_measurement", cpu07_reset_bundle_clears_the_measurement as fn()),
    ]);
}
