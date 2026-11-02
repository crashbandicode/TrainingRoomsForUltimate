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
