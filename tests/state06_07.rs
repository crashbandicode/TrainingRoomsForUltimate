//! SPEC-state-06 (L+R+A Reset, reset hub) and SPEC-state-07 (Stage Hazards). Test names are the
//! specs' §9 rows. `state07_scan_matches_pinned` re-runs the instruction scans on the owned 13.0.5
//! `.text`.

mod common;

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::lra_reset::{self, FrameCounter, FrameCounterType, HubStep};
use model::sites;
use model::stage_hazards::{self as hz, Missing, Plan};

fn words_of(guard: &[u8; 64]) -> Vec<u32> {
    guard.as_chunks::<4>().0.iter().map(|w| u32::from_le_bytes(*w)).collect()
}

#[test]
fn state06_lra_register_rule() {
    // Off in native Training: x8 is cleared so the game's check fails. On: untouched.
    assert_eq!(lra_reset::x8_write(false, true), Some(0));
    assert_eq!(lra_reset::x8_write(true, true), None);
    // Outside native Training the register is never touched (the fork's early return).
    assert_eq!(lra_reset::x8_write(false, false), None);
    assert_eq!(lra_reset::x8_write(true, false), None);
    // The setting decodes from the file: [1,0] On, [0,1] Off.
    let mut s = model::settings::Snapshot::factory();
    assert!(s.lra_reset());
    s.set("lra_reset", &[0, 1]).unwrap();
    assert!(!s.lra_reset());
    assert_eq!(lra_reset::x8_write(s.lra_reset(), true), Some(0));
    // The inverted upstream gate is not adopted: the hub never runs outside Training.
    assert!(!lra_reset::hub_should_run(true, false, false));
}

#[test]
fn state06_hub_trigger_rules() {
    assert!(lra_reset::hub_should_run(true, true, false));
    assert!(!lra_reset::hub_should_run(false, true, false), "only the CPU's init triggers it");
    assert!(!lra_reset::hub_should_run(true, false, false), "native Training only");
    assert!(!lra_reset::hub_should_run(true, true, true), "not during a hot swap");
    // The reset-check hook and the on-init wrapper sit on sites that this build guards.
    assert!(sites::find("reset_check").is_some());
    assert!(sites::find("on_init_slot").is_some());
    assert!(sites::find("on_init").is_some());
}

#[test]
fn state06_hub_order() {
    assert_eq!(
        lra_reset::HUB_ORDER[..7],
        [HubStep::Mash, HubStep::Sdi, HubStep::FrameCounters, HubStep::Combo, HubStep::Ledge, HubStep::Throw, HubStep::ShieldTilt]
    );
    // This plugin's own step (decision B8) comes after the spec's seven.
    assert_eq!(lra_reset::HUB_ORDER[7], HubStep::InputDelay);
    let mut seen = Vec::new();
    lra_reset::run_hub(|step| seen.push(step));
    assert_eq!(seen, lra_reset::HUB_ORDER);
}

#[test]
fn state06_counter_types() {
    let mut counters = [
        FrameCounter { kind: FrameCounterType::InGame, frames: 40, counting: true },
        FrameCounter { kind: FrameCounterType::InGameNoReset, frames: 41, counting: true },
        FrameCounter { kind: FrameCounterType::Real, frames: 42, counting: true },
    ];
    lra_reset::reset_in_game_counters(&mut counters);
    assert_eq!(counters[0], FrameCounter { kind: FrameCounterType::InGame, frames: 0, counting: false });
    assert_eq!(counters[1].frames, 41);
    assert!(counters[1].counting);
    assert_eq!(counters[2].frames, 42);
    assert!(counters[2].counting);
}

#[test]
fn state06_hub_idempotent() {
    // Running the hub twice in a frame (CPU-as-P1, two CPUs) leaves the same state as once.
    #[derive(Default, PartialEq, Debug, Clone)]
    struct State {
        mash_queue: Vec<u8>,
        counters: Vec<FrameCounter>,
        delay_queue_len: usize,
    }
    let start = State {
        mash_queue: vec![1, 2],
        counters: vec![FrameCounter { kind: FrameCounterType::InGame, frames: 9, counting: true }],
        delay_queue_len: 3,
    };
    let hub = |s: &mut State| {
        lra_reset::run_hub(|step| match step {
            HubStep::Mash => s.mash_queue.clear(),
            HubStep::FrameCounters => lra_reset::reset_in_game_counters(&mut s.counters),
            HubStep::InputDelay => s.delay_queue_len = 0,
            _ => {}
        })
    };
    let mut once = start.clone();
    hub(&mut once);
    let mut twice = start;
    hub(&mut twice);
    hub(&mut twice);
    assert_eq!(once, twice);
}

#[test]
fn state07_flag_address_arithmetic() {
    // The two words at the scan site in 13.0.5 give the pinned byte.
    let site = sites::find("hazard_flag").unwrap();
    assert_eq!(site.offset, hz::FLAG_SCAN_OFFSET);
    let w = words_of(&site.guard);
    assert_eq!(hz::flag_offset(site.offset, w[0], w[1]), Some(hz::FLAG_OFFSET));
    assert_eq!(hz::FLAG_OFFSET, 0x530a985);
    // The pieces: adrp's page offset and add's immediate.
    assert!(hz::is_adrp(w[0]) && hz::is_add_imm(w[1]));
    assert_eq!(hz::adrp_offset(w[0]) + 0x1740000, 0x530a985 - 9 - hz::add_imm_value(w[1]) as i64);
    // A shifted add, a negative adrp, and non-instructions.
    assert_eq!(hz::add_imm_value(0x9140_0400), 0x1000);
    assert_eq!(hz::add_imm_value(0x9100_0400), 1);
    assert_eq!(hz::adrp_offset(0xf000_0000 | (0x7_ffff << 5)), -4096);
    assert_eq!(hz::flag_offset(0x1000, 0x1400_0000, w[1]), None);
    assert_eq!(hz::flag_offset(0x1000, w[0], 0x1400_0000), None);
}

#[test]
fn state07_scan_matches_pinned() {
    // The pinned windows hold exactly the instruction runs the scans look for.
    let flag = sites::find("hazard_flag").unwrap();
    assert_eq!(hz::find_flag_run(&words_of(&flag.guard)), Some(0));
    let window = sites::find("hazard_window").unwrap();
    let hook = sites::find("hazard_hook").unwrap();
    assert_eq!(hz::find_hook_run(&words_of(&window.guard)), Some(0));
    assert_eq!(window.offset + 8, hook.offset);
    assert_eq!(hook.offset, hz::HOOK_SITE_OFFSET);
    // On the owned `.text` the first match of each scan is the pinned one.
    let Some(main) = common::owned_main("state07_scan_matches_pinned") else {
        return;
    };
    let words = main.text_words();
    let flag_run = hz::find_flag_run(&words).expect("flag run");
    assert_eq!(flag_run * 4, hz::FLAG_SCAN_OFFSET);
    assert_eq!(hz::flag_offset(flag_run * 4, words[flag_run], words[flag_run + 1]), Some(hz::FLAG_OFFSET));
    let hook_run = hz::find_hook_run(&words).expect("hook run");
    assert_eq!((hook_run + 2) * 4, hz::HOOK_SITE_OFFSET);
}

#[test]
fn state07_write_rule() {
    assert_eq!(hz::rule_byte(true, true), Some(1));
    assert_eq!(hz::rule_byte(false, true), Some(0));
    assert_eq!(hz::rule_byte(true, false), None);
    assert_eq!(hz::rule_byte(false, false), None);
    let mut s = model::settings::Snapshot::factory();
    assert_eq!(hz::rule_byte(s.stage_hazards(), true), Some(0), "default Off writes 0 in Training");
    s.set("stage_hazards", &[1, 0]).unwrap();
    assert_eq!(hz::rule_byte(s.stage_hazards(), true), Some(1));
}

/// rc.12 (operator 2026-11-13, "yes fix it in rc12"): Change Stage in Training takes the Stage Hazards setting. The
/// request copies the active stage's descriptor, whose hazards byte was fixed at Training entry, and the hook's rule
/// byte is not read on a Change Stage; so Off at entry and On before Change Stage gave no hazards.
#[test]
fn state07_change_stage_takes_the_setting() {
    const TRAINING: (u32, u8) = (0x04040000, 0x20);
    for on in [false, true] {
        assert_eq!(hz::change_stage_byte(true, TRAINING.0, TRAINING.1, None, on), Some(u8::from(on)), "Training, {on}");
        assert_eq!(hz::change_stage_byte(false, TRAINING.0, TRAINING.1, None, on), None, "not initialized");
        // Waiting rooms keep the carried rule's byte (or none).
        assert_eq!(hz::change_stage_byte(true, 0x02010000, 0x0d, Some(u8::from(on)), !on), Some(u8::from(on)));
        assert_eq!(hz::change_stage_byte(true, 0x02010000, 0x0d, None, on), None);
        // A match keeps its own.
        assert_eq!(hz::change_stage_byte(true, 0x02010000, 0x01, None, on), None);
    }
}

#[test]
fn state07_missing_address_disables() {
    assert_eq!(hz::plan(Some(1), true), Plan::Enabled);
    assert_eq!(hz::plan(None, true), Plan::Disabled(Missing::Flag));
    assert_eq!(hz::plan(Some(1), false), Plan::Disabled(Missing::Hook));
    assert_eq!(hz::plan(None, false), Plan::Disabled(Missing::Both));
    // Each condition has its own text (upstream's were swapped).
    assert_ne!(Missing::Flag.text(), Missing::Hook.text());
    assert!(Missing::Flag.text().contains("rule byte"));
    assert!(Missing::Hook.text().contains("hook site"));
}

#[test]
fn state06_hub_trigger_facts_match_the_guarded_words() {
    // The on_init function's first words (guarded by the `on_init` row) hold the two facts the
    // trigger relies on: the entry id is loaded from fighter+0x10, and the FighterManager slot is
    // read through an adrp/ldr pair that lands on the pinned slot.
    let on_init = sites::find("on_init").unwrap();
    let w = words_of(&on_init.guard);
    // ldp w9, w8, [x0, #0xc]
    assert_eq!(w[8], 0x2941_a009);
    assert_eq!(lra_reset::FIGHTER_ENTRY_ID_FIELD, 0x10);
    assert_eq!(0xc + 4, lra_reset::FIGHTER_ENTRY_ID_FIELD);
    // adrp x24 / ldr x27, [x24, #0x4f8]
    assert_eq!(lra_reset::manager_slot_offset(on_init.offset + 9 * 4, w[9], w[10]), Some(lra_reset::FIGHTER_MANAGER_SLOT_OFFSET));
    assert_eq!(lra_reset::manager_slot_offset(0, 0x1400_0000, w[10]), None);
    assert_eq!(lra_reset::manager_slot_offset(0, w[9], 0), None);
    // The two called thunks are the ones the symbols name: `ldr x8, [x0, #8]; ldrb w0, [x8, #0x92]`.
    let cpu = sites::find("is_operation_cpu").unwrap();
    let c = words_of(&cpu.guard);
    assert_eq!((c[0], c[1], c[2]), (0xf940_0408, 0x3942_4900, 0xd65f_03c0));
    // get_fighter_information(manager, entry): `cmp w1, #8` first.
    let info = sites::find("fighter_information").unwrap();
    assert_eq!(words_of(&info.guard)[0], 0x7100_203f);
    assert_eq!(lra_reset::ENTRY_COUNT, 8);
    assert!(sites::feature_usable(sites::Feature::ResetHub, |_| true));
    assert!(!sites::feature_usable(sites::Feature::ResetHub, |s| s.name != "is_operation_cpu"));
}

