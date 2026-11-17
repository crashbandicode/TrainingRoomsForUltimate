//! L+R+A Reset and the Training reset hub (SPEC-state-06).
//!
//! Two pieces: the rule that lets the player turn the game's own L+R+A reset off (a register is
//! cleared inside the game's reset check), and the hub that clears every per-session plugin state
//! when the game resets Training. The upstream HEAD change that runs the hub only outside Training
//! (`077a8e43`, SPEC-state-06 question 1) is not adopted: the hub runs in native Training only.

/// What the inline hook inside the game's reset check does to register `x8`: when the setting is
/// Off, and only in native Training, the register is cleared so the game's check fails and no reset
/// happens (SPEC-state-06 §4). `None` leaves the register alone.
///
/// `fighter_loading` (CRASH-0900, reship-3 hardware 2026-11-17): a practice fighter is mid-load,
/// either the roster's install suspends practice or a practice entry's native loader is away from
/// its resting state ([`loader_resting`]). Vanilla Training never resets while a fighter loads; a
/// reset there left P1's loader parked in state 13 (Eden `crash0900-a-reship34`/`b-card4`: the
/// install never finished, every later pick refused) and on the console the state-13 commit
/// activation later read a null `cell[+0x5918]` (Data Abort at 0x10, PC main+0x628b40, LR
/// main+0x656f40 from `0x656644`). The reset is refused until the load settles; the player
/// presses L+R+A again.
pub const fn x8_write(lra_reset_on: bool, native_training: bool, fighter_loading: bool) -> Option<u64> {
    if native_training && (!lra_reset_on || fighter_loading) {
        Some(0)
    } else {
        None
    }
}

/// A fighter entry's native loader state byte (`entry+0x591c`; `0x655f60` dispatches on it).
pub const LOADER_STATE_FIELD: usize = 0x591c;
/// The loader's resting state for a live practice fighter (14): Eden `trace.*` on `main+0x656ee0`
/// (`crash0900-*`) reads 14 on both entries between loads and 13 (or lower) during a Reload or a
/// group-swap commit.
pub const LOADER_RESTING: u8 = 14;

/// Whether an entry's loader is at rest. An unreadable byte is not at rest (fail closed: the
/// reset waits); an absent entry is decided by the caller (no entry, nothing loading).
pub const fn loader_resting(state: Option<u8>) -> bool {
    matches!(state, Some(LOADER_RESTING))
}

/// The value `x8` holds at the hook when the game is about to reset: the site is
/// `cmp w8, #0x2; b.ne <no reset>` (`main+0x1378e50`, `w8 = [x8+0x240]`).
pub const RESET_REQUESTED: u64 = 2;

/// The notice a refused reset shows (never silent; primary 2026-11-17 09:55Z): the existing swap-busy text
/// (`mii::ordinary_cpu::SWAP_BUSY_NOTICE`, the roster's refusal of a pick during the same load), under the
/// setting's own menu label.
pub const REFUSED_HEADER: &str = "L+R+A Reset";
pub const REFUSED_BODY: &str = "Wait for the character swap to finish";

/// One reset check: the register write ([`x8_write`]), whether this check is a reset refused because a
/// fighter loads, and whether to post [`REFUSED_BODY`] now. The notice is posted once per refusal (the
/// first check of a run of refused requests: a held L+R+A, or a request the game keeps), never when the
/// setting itself is Off (the player turned the reset off) and never for a check that requests nothing.
pub const fn gate(x8: u64, lra_reset_on: bool, native_training: bool, fighter_loading: bool, was_refused: bool)
    -> (Option<u64>, bool, bool) {
    let write = x8_write(lra_reset_on, native_training, fighter_loading);
    let refused = x8 == RESET_REQUESTED && native_training && lra_reset_on && fighter_loading;
    (write, refused, refused && !was_refused)
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
#[cfg(test)]
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
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameCounter {
    pub kind: FrameCounterType,
    pub frames: u32,
    pub counting: bool,
}

/// `reset_all`: every in-game counter is set to 0 and stops counting; the other types are untouched.
#[cfg(test)]
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
        assert_eq!(x8_write(false, true, false), Some(0));
        assert_eq!(x8_write(false, false, false), None);
        assert_eq!(x8_write(true, true, false), None);
        // CRASH-0900: a practice fighter mid-load refuses the reset even with the setting On.
        assert_eq!(x8_write(true, true, true), Some(0));
        assert_eq!(x8_write(true, false, true), None);
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