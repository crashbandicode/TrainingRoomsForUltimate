//! L+R+A Reset and the Training reset hub (SPEC-state-06).
//!
//! Two pieces: the rule that lets the player turn the game's own L+R+A reset off (a register is
//! cleared inside the game's reset check), and the hub that clears every per-session plugin state
//! when the game resets Training. The upstream HEAD change that runs the hub only outside Training
//! (`077a8e43`, SPEC-state-06 question 1) is not adopted: the hub runs in native Training only.

/// What the inline hook inside the game's reset check does to register `x8`: when the setting is
/// Off, and only in native Training, the register is cleared so the game's check fails and no reset
/// happens (SPEC-state-06 §4). `None` leaves the register alone.
pub const fn x8_write(lra_reset_on: bool, native_training: bool) -> Option<u64> {
    if native_training && !lra_reset_on {
        Some(0)
    } else {
        None
    }
}

/// Whether the fighter on-init wrapper runs the hub after the game's own on-init (SPEC-state-06
/// §5.1): only for the CPU-operated fighter, so the hub runs once per reset and not once per
/// fighter; only in native Training; not while a hot swap is replacing a fighter.
pub const fn hub_should_run(cpu_operated: bool, native_training: bool, hot_swap_in_progress: bool) -> bool {
    cpu_operated && native_training && !hot_swap_in_progress
}

/// The steps of `on_reset()` in order (SPEC-state-06 §5.2), then the plugin's own additions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubStep {
    /// 1. Discard the queued and current mash actions.
    Mash,
    /// 2. Draw a new random SDI direction.
    Sdi,
    /// 3. Set every in-game frame counter to 0 and stop it.
    FrameCounters,
    /// 4. Reset the combo counter and the frame-advantage bookkeeping.
    Combo,
    /// 5. Reset the ledge delay.
    Ledge,
    /// 6. Reset the throw delay.
    Throw,
    /// 7. Draw a new random shield-tilt direction.
    ShieldTilt,
    /// Added by this plugin (decision B8, SPEC-input-03 question 2): empty the input-delay queue.
    InputDelay,
}

/// The hub's steps in the order it runs them. Features that are not built yet are skipped by the
/// runtime's step handler; the order does not change when they arrive.
pub const HUB_ORDER: [HubStep; 8] = [
    HubStep::Mash,
    HubStep::Sdi,
    HubStep::FrameCounters,
    HubStep::Combo,
    HubStep::Ledge,
    HubStep::Throw,
    HubStep::ShieldTilt,
    HubStep::InputDelay,
];

/// Run the hub: call `step` once per `HUB_ORDER` entry.
pub fn run_hub(mut step: impl FnMut(HubStep)) {
    for hub_step in HUB_ORDER {
        step(hub_step);
    }
}

/// How the frame counters reset (SPEC-state-06 §5.2 step 3). The counters' own behaviour is
/// SPEC-cpu-07's; only the type rule is needed by the hub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameCounterType {
    /// Counts game frames and is reset to 0 (and stopped) by a reset or a save-state load.
    InGame,
    /// Counts game frames and survives a reset.
    InGameNoReset,
    /// Counts real frames, paused or not; survives a reset.
    Real,
}

/// A counter's value and whether it is running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameCounter {
    pub kind: FrameCounterType,
    pub frames: u32,
    pub counting: bool,
}

/// `reset_all`: every in-game counter is set to 0 and stops counting; the other types are untouched.
pub fn reset_in_game_counters(counters: &mut [FrameCounter]) {
    for counter in counters {
        if counter.kind == FrameCounterType::InGame {
            counter.frames = 0;
            counter.counting = false;
        }
    }
}

/// Where the hub reads the fighter on init (facts read from the 13.0.5 `Fighter` on-init function,
/// `main+0x60dfa0`, whose first instructions are guarded by the `on_init` site: `ldp w9, w8, [x0,
/// #0xc]` loads the fighter kind and the entry id from the fighter object).
pub const FIGHTER_ENTRY_ID_FIELD: usize = 0x10;
/// Fighter entry ids are `0..8` (the game compares against 8 before indexing its table).
pub const ENTRY_COUNT: i32 = 8;
/// The `FighterManager` singleton slot (`_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E`, in
/// `.bss`): holds the manager pointer. Derived from the guarded `adrp`/`ldr` pair of the on-init
/// function; `manager_slot_offset` recomputes it from those words.
pub const FIGHTER_MANAGER_SLOT_OFFSET: usize = 0x52b_94f8;

/// A valid entry id.
pub fn entry_id_valid(entry: i32) -> bool {
    (0..ENTRY_COUNT).contains(&entry)
}

/// The offset of the pointer slot an `adrp Xn, page; ldr Xt, [Xn, #imm]` pair at `adrp_site` reads
/// (the 64-bit unsigned-offset form), or `None` when the words are not that pair.
#[cfg(test)]
pub fn manager_slot_offset(adrp_site: usize, adrp: u32, ldr: u32) -> Option<usize> {
    use super::stage_hazards::{adrp_offset, is_adrp};
    if !is_adrp(adrp) || ldr & 0xffc0_0000 != 0xf940_0000 {
        return None;
    }
    let page = i64::try_from(adrp_site & !0xfff).ok()?;
    let slot = page.checked_add(adrp_offset(adrp))?.checked_add(i64::from((ldr >> 10) & 0xfff) << 3)?;
    usize::try_from(slot).ok()
}
#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn entry_ids_are_zero_to_seven() {
        assert!(entry_id_valid(0) && entry_id_valid(7));
        assert!(!entry_id_valid(-1) && !entry_id_valid(8));
    }

    fn off_clears_x8_in_training_only() {
        assert_eq!(x8_write(false, true), Some(0));
        assert_eq!(x8_write(false, false), None);
        assert_eq!(x8_write(true, true), None);
    }

    fn the_hub_runs_every_step_in_order() {
        let mut seen = Vec::new();
        run_hub(|step| seen.push(step));
        assert_eq!(seen, HUB_ORDER);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("entry_ids_are_zero_to_seven", entry_ids_are_zero_to_seven as fn()),
            ("off_clears_x8_in_training_only", off_clears_x8_in_training_only as fn()),
            ("the_hub_runs_every_step_in_order", the_hub_runs_every_step_in_order as fn()),
        ]);
    }
    }
}