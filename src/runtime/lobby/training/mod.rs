//! The fork's `training` module as the carried lobby files name it (`super::input_delay`, `super::named_macros`, `training::ui::...`):
//! the two carried children (`lobby`, `practice`) and what they reach through their parent. Our code; each item forwards to the clean
//! room's provider.

pub(crate) use crate::engine::lobby::practice_policy;

pub(crate) mod lobby;
pub(crate) mod practice;

// Not declared yet (it joins with worker A's cycle): the providers `named_macros` (package 4, S4-4), `tech` and `cpu_rolls` (package 2)
// do not exist in the tree. They are `pub(crate) mod` items of this facade that forward to those packages' functions; see
// IMPLEMENTATION-PROGRESS-pkg6b.md for the names and signatures the carried files call.

/// The fork's `input_delay` module as the lobby names it.
pub(crate) mod input_delay {
    /// The input-delay queue empties (session clear, SPEC-input-03 section 5.1).
    pub fn clear_session_state() {
        crate::runtime::features::input_delay::clear(crate::model::input_delay::Clear::Session);
    }
}

/// Every per-session state the plugin keeps ends with the departure from the practice scene (recorder, chords, notices, input
/// display and the menu session: package 4's `session_clear` runs all of them).
pub(crate) fn clear_session_state() {
    crate::runtime::features::recorder::session_clear();
}

/// A recording or a playback is running (package 4's recorder).
pub(crate) fn recording_or_playback() -> bool {
    crate::runtime::features::recorder::recording_or_playback()
}

/// The fork's `save_states` module, as far as the lobby reads it.
pub(crate) mod save_states {
    /// The stage id the game holds now (package 3's guarded native).
    pub fn stage_id() -> i32 {
        // SAFETY: a guarded called function with no arguments.
        unsafe { crate::runtime::state_natives::stage_id() }
    }
}

/// The fork's `ui` module, as far as the lobby uses it.
pub(crate) mod ui {
    /// The native pause menu's Combo Display copy (package 4's observers own it).
    pub(crate) mod native_menu {
        /// A scene boundary: the copied value is forgotten (Combo Display reads as enabled again).
        pub fn reset() {
            crate::runtime::features::notices::reset_combo_display();
        }
    }

    /// The fork's name-keyed notifications over package 4's standing notices (a fixed id per header).
    pub(crate) mod notifications {
        use crate::engine::lobby::notice_ids::{id_of, HEADERS};
        use crate::model::notify::DEFAULT_COLOR;
        use crate::runtime::features::notices;

        use std::borrow::Cow;

        /// Show `body` under `header` for `frames` (replacing the notice of the same header, shown behind the plain notices). The
        /// arguments are `Cow`s so a callback that passes static text (`"..".into()`) allocates nothing; the text is copied into
        /// the notice queue's fixed buffer.
        pub fn notification(header: Cow<'static, str>, body: Cow<'static, str>, frames: u16) {
            if let Some(slot) = id_of(&header) {
                notices::upsert(HEADERS[slot].0, HEADERS[slot].1, &body, frames, DEFAULT_COLOR);
            }
        }

        /// Remove the notice of `header`.
        pub fn clear_notification(header: &str) {
            if let Some(slot) = id_of(header) {
                notices::clear(HEADERS[slot].0);
            }
        }
    }
}