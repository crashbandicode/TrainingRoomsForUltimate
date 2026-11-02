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

#[cfg(test)]
mod tests {
    use super::*;

    const READY: Facts = Facts { installed: true, layout_ready: true, pause_active: false, closing: false, owns_input: false, polls_since_tick: 0 };

    #[test]
    fn it_opens_only_with_a_renderer_and_no_pause_or_close_window() {
        assert!(may_open(READY));
        assert!(!may_open(Facts { installed: false, ..READY }));
        assert!(!may_open(Facts { layout_ready: false, ..READY }));
        assert!(!may_open(Facts { pause_active: true, ..READY }));
        assert!(!may_open(Facts { closing: true, ..READY }));
    }

    #[test]
    fn an_owning_menu_is_released_when_the_renderer_goes_away() {
        let owning = Facts { owns_input: true, ..READY };
        assert!(!must_release(owning));
        assert!(must_release(Facts { layout_ready: false, ..owning }));
        assert!(must_release(Facts { installed: false, ..owning }));
        assert!(!must_release(Facts { polls_since_tick: STALL_POLLS, ..owning }));
        assert!(must_release(Facts { polls_since_tick: STALL_POLLS + 1, ..owning }), "the tick stopped: the layout is gone");
        assert!(!must_release(Facts { layout_ready: false, ..READY }), "nothing owned, nothing to release");
    }
}
