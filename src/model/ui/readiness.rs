//! When the plugin menu may own Player 1's input (review R2-2, SPEC-core-05 question 1 "refuse to open"). The menu
//! cannot open, and never sets `Owners::MENU`, unless a renderer exists: the UI anchors are installed (`installed`: the
//! guards passed and the worker runs) and the Training HUD layout is registered with its panes resolved
//! (`layout_ready`: the injected archive was accepted). Any later loss of the renderer releases the menu's ownership
//! at once: the layout tick says so when the panes are gone or the scene is not Training, and the input hook says so
//! when the tick has stopped (the layout was destroyed) while the menu is open.
//!
//! Pure rules; the runtime keeps the facts in atomics (`runtime::ui::state`).

/// Polls of Player 1 without a layout tick after which an open menu has lost its renderer (about three seconds at
/// 60 polls a second; the tick runs every frame while the layout lives).
pub const STALL_POLLS: u32 = 180;

/// The native ownership observer needs only its two live-layout callbacks and
/// the Training predicate, not the archive, text writer, manager or worker.
pub const NATIVE_PAUSE_ROWS: [&str; 3] = ["ui_pass_calculate", "ui_register_calculate", "is_training_mode"];

/// Actual installation boundary, shared with host installation-spy regressions.
pub fn install_pause_observer(usable: impl Fn(&str) -> bool, install: impl FnOnce()) -> bool {
    if !NATIVE_PAUSE_ROWS.into_iter().all(usable) { return false; }
    install();
    true
}

/// Renderer admission consumes the same pre-write guard receipt as the observer.
/// Never reinterpret our now-written hooks as stock, or relax any guard for them.
pub fn renderer_anchors_ready(usable: impl Fn(&str) -> bool, input_ready: bool,
    observer_installed: bool, manager_slot: bool) -> bool {
    input_ready && observer_installed && manager_slot
        && ["ui_archive_alloc", "ui_pass_calculate", "ui_register_calculate", "ui_manager_global", "ui_set_text"]
            .into_iter().all(usable)
}

/// The facts the decision reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// The anchors and the worker are installed.
    pub installed: bool,
    /// The tick resolved the menu's panes in the current Training layout.
    pub layout_ready: bool,
    /// The game's pause menu is open.
    pub pause_active: bool,
    /// The 15-frame close window is running.
    pub closing: bool,
    /// The menu is open or closing, or holds the owner bit.
    pub owns_input: bool,
    /// P1 polls since the layout tick last ran.
    pub polls_since_tick: u32,
}

/// Whether an opening chord may be accepted now.
pub fn may_open(facts: Facts) -> bool {
    facts.installed && facts.layout_ready && !facts.pause_active && !facts.closing
}

/// Whether the menu's ownership must be released now: it owns input but its renderer is gone.
pub fn must_release(facts: Facts) -> bool {
    facts.owns_input && (!facts.installed || !facts.layout_ready || facts.polls_since_tick > STALL_POLLS)
}

