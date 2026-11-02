//! The fork's `training` module as the carried lobby files name it (`super::input_delay`, `super::named_macros`, `training::ui::...`):
//! the two carried children (`lobby`, `practice`) and what they reach through their parent. Our code; each item forwards to the clean
//! room's provider.

pub(crate) use crate::engine::lobby::practice_policy;

pub(crate) mod character_specific;
pub(crate) mod lobby;
pub(crate) mod practice;

/// The fork's `training::named_macros`: package 4's named-macro runtime (the waiting-room side the carried lobby and roster files call
/// is `suspend_waiting`, `resume_waiting`, `suspended`, `forget_stopped_entry`, `forget_stopped_scene`, `waiting_scene_changed`).
pub(crate) use crate::runtime::macros::named_macros;

/// The fork's `tech` module as the lobby names it: package 2's CPU state drops the hidden-tech record of a departed body.
pub(crate) mod tech {
    pub fn forget_hidden_entry(entry: usize) {
        crate::runtime::features::cpu::forget_hidden_entry(entry);
    }
    pub fn forget_hidden_scene() {
        crate::runtime::features::cpu::forget_hidden_scene();
    }
}

/// The fork's `cpu_rolls` module as the lobby names it: every cached CPU choice is taken again from the live menu.
pub(crate) mod cpu_rolls {
    use crate::model::menu::reroll;

    pub fn request_all() {
        crate::runtime::features::cpu::request_rerolls(
            reroll::SDI_DIRECTION | reroll::SHIELD_TILT | reroll::LEDGE_OPTIONS | reroll::THROW_OPTIONS,
        );
    }
}

/// The fork's `training::charge`, as far as the Kirby hats call it: package 3's guarded Kirby copy setup.
pub(crate) mod charge {
    /// `FighterSpecializer_Kirby::copy_setup`: Kirby (`body`) gets the copy ability of `kind` from the fighter of entry `opponent`.
    ///
    /// # Safety
    /// `body` is Kirby's live module accessor.
    pub unsafe fn setup_copy_ability(body: *mut smash::app::BattleObjectModuleAccessor, opponent: impl TryInto<i32>, kind: i32) {
        if let Ok(opponent) = opponent.try_into() {
            crate::runtime::state_natives::kirby_copy_setup(body as usize, opponent, kind);
        }
    }
}

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
    crate::runtime::features::states::clear_session();
    crate::runtime::features::recorder::session_clear();
}

/// A recording or a playback is running (package 4's recorder).
pub(crate) fn recording_or_playback() -> bool {
    crate::runtime::features::recorder::recording_or_playback()
}

/// The fork's `save_states` module, as far as the lobby reads it.
pub(crate) mod save_states {
    /// The stage id the game holds now (the guarded native, `game::natives::stage_id`; -1 when there is none).
    pub fn stage_id() -> i32 {
        // SAFETY: a guarded called function with no arguments.
        unsafe { crate::runtime::game::natives::stage_id() }
    }

    /// A save state is loading (package 3's fact, `features::states::is_loading`).
    pub fn is_loading() -> bool {
        crate::runtime::features::states::is_loading()
    }
}

/// The fork's `ui` module, as far as the lobby and the carried files use it.
pub(crate) mod ui {
    /// The fork's `ui::menu`, as far as the carried files read it: whether the game's own pause menu is open.
    pub(crate) mod menu {
        use crate::model::input_delay::Owners;
        use crate::runtime::compat::sync::Readable;
        use crate::runtime::published;

        /// The native (vanilla) pause menu is open (`Owners::VANILLA_PAUSE`).
        pub struct VanillaPause;

        impl Readable for VanillaPause {
            type Value = bool;
            fn read_value(&self) -> bool {
                published::input_owners().contains(Owners::VANILLA_PAUSE)
            }
        }

        pub static VANILLA_MENU_ACTIVE: VanillaPause = VanillaPause;
    }

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

        /// `notification` with the text formatted on the stack (a fixed buffer, copied into the notice queue): no heap, for the
        /// callbacks whose text has numbers or names in it.
        pub fn notification_args(header: Cow<'static, str>, args: std::fmt::Arguments<'_>, frames: u16) {
            use std::fmt::Write;
            let mut text = crate::model::storage::FixedPath::<96>::new();
            // A text that does not fit is cut at the buffer (the notice shows its first characters).
            let _ = text.write_fmt(args);
            if let Some(slot) = id_of(&header) {
                notices::upsert(HEADERS[slot].0, HEADERS[slot].1, text.as_str(), frames, DEFAULT_COLOR);
            }
        }

        /// Show or update the standing notice of `header` with `body` for `frames`, kept behind every other notice.
        pub fn upsert(header: &'static str, body: impl AsRef<str>, frames: u32) {
            if let Some(slot) = id_of(header) {
                notices::upsert(HEADERS[slot].0, HEADERS[slot].1, body.as_ref(), u16::try_from(frames).unwrap_or(u16::MAX), DEFAULT_COLOR);
            }
        }

        /// The standing notice of `header` was visible for its whole time (once per delivery).
        pub fn take_delivered(header: &str) -> bool {
            id_of(header).is_some_and(|slot| notices::take_delivered() == Some(HEADERS[slot].0))
        }

        /// The standing notice of `header` is queued (not yet delivered, not cleared).
        pub fn is_queued(header: &str) -> bool {
            id_of(header).is_some_and(|slot| notices::is_queued(HEADERS[slot].0))
        }

        /// Remove the notice of `header`.
        pub fn clear_notification(header: &str) {
            if let Some(slot) = id_of(header) {
                notices::clear(HEADERS[slot].0);
            }
        }
    }
}
