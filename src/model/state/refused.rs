//! rc.12 (the console's Save and Load of 2026-11-13; the primary 13:29Z: "no silent no-ops"): a Save or Load the player
//! asked for and did not get says why: one release-build line (`key`), and a notice (`notice`) unless the press visibly
//! acted somewhere else (an open menu or pop-up took it).

/// What was asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Save,
    Load,
}

impl Action {
    pub const fn key(self) -> &'static str {
        match self {
            Action::Save => "save",
            Action::Load => "load",
        }
    }
}

/// Why it was not done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// An open menu, pop-up or editor owns the input: the press acted there.
    Owner,
    /// The game speed pop-up is open: the press steered it.
    SpeedHold,
    /// Online Deluxe's open page acts on this chord's D-pad (a chord rebound onto a button Deluxe uses).
    DeluxeRows,
    /// Both GameCube triggers are held: that is Online Deluxe's L + R.
    GameCubeTriggers,
    /// Save States is Off.
    Off,
    /// The save-state step has not started in this match (its game values were not found yet).
    NotReady,
    /// A CPU drives Player 1, so no body takes the Load.
    PlayerIsCpu,
    /// A load is already being placed.
    Loading,
    /// A setup, a preview or a Play warp is being placed.
    Temporary,
    /// Nothing is saved in the slot the Load reads (no bound save).
    Empty,
}

impl Refused {
    /// The reason in the release line `[TrainingRooms] save_state refused action=… reason=…`.
    pub const fn key(self) -> &'static str {
        match self {
            Refused::Owner => "owner",
            Refused::SpeedHold => "speed_hold",
            Refused::DeluxeRows => "deluxe_rows",
            Refused::GameCubeTriggers => "gamecube_triggers",
            Refused::Off => "off",
            Refused::NotReady => "not_ready",
            Refused::PlayerIsCpu => "player_is_cpu",
            Refused::Loading => "loading",
            Refused::Temporary => "temporary",
            Refused::Empty => "empty",
        }
    }

    /// The notice body under the "Save State" header, or `None` where the press visibly acted elsewhere.
    pub const fn notice(self, action: Action) -> Option<&'static str> {
        let save = matches!(action, Action::Save);
        match self {
            Refused::Owner | Refused::SpeedHold => None,
            // restriction: save-load-deluxe-page
            Refused::DeluxeRows => Some(if save { "Online Deluxe's open page uses this button: close it to save" } else { "Online Deluxe's open page uses this button: close it to load" }),
            // restriction: save-load-gamecube-triggers
            Refused::GameCubeTriggers => Some("Both triggers are Online Deluxe's L + R: hold one of them"),
            // restriction: save-load-off
            Refused::Off => Some("Save States is Off (Saves page)"),
            // restriction: save-load-not-ready
            Refused::NotReady => Some("Save states have not started in this match yet"),
            // restriction: save-load-player-cpu
            Refused::PlayerIsCpu => Some("A CPU drives Player 1: take Player 1 back to load"),
            // restriction: save-load-busy
            Refused::Loading => Some("A load is already being placed"),
            // restriction: save-load-busy
            Refused::Temporary => Some(if save { "A setup is being placed: save again after it" } else { "A setup is being placed: load again after it" }),
            // restriction: save-load-empty
            Refused::Empty => Some("Nothing is saved here yet: Save first, or bind a save"),
        }
    }
}
