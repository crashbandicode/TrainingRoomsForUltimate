//! The Replay capture row on the Online Deluxe Plugins page (menu question 7; decision L1: the operator's MIT plugin SDK,
//! `deluxe-plugin-sdk` 5.0.0 at rev 20dd1cb, vendored under `third_party/`). Pure: the row's spec, its handler over the
//! capture preference and the registration outcome the install line reports. The Switch side
//! (`runtime::deluxe_row`) registers it once at boot, after the INI seed and before the capture adapter installs.
//!
//! Q7: the TrainingRooms menu row, `replay_capture.ini` and this row always agree. One preference
//! (`engine::macros::CaptureToggle`) is read and changed by both rows, and one writer (the small-file writer thread,
//! lead ruling R5) writes the INI. The row therefore asks the host for no persistence (`config_path` is `None`): the
//! host would otherwise be a second writer of the same file. After an accepted change the handler only schedules a
//! write; the writer takes the value from the preference when it writes, so the file ends at the newest value whichever
//! row changed it last.
//!
//! ABI (evidence in `IMPLEMENTATION-PROGRESS-deluxe.md (removed after v0.2.0-rc.1; see that tag)`): the SDK resolves `ssbu_online_deluxe_plugin_api_v1` at run
//! time, accepts only table version 1 (32 bytes) and sends descriptor version 4 (72-byte `PluginDesc`) exactly once.
//! The card's Deluxe 0.10.1 host exports that table and its `register` accepts descriptor versions 2 to 5, so the
//! version 4 descriptor registers directly; no fallback exists or is needed. A host older than 0.5.3 answers
//! `REGISTER_ERR_ABI` and the row is simply absent (`host_older_than_0_5_3`); no host at all is `host_missing`.
//!
//! Threading: `current` and `request` run on the host's overlay draw thread. They touch only the toggle's packed atomic
//! word and the writer's request flag: no lock held across a call into the game, no file I/O, no allocation.

use deluxe_plugin_sdk::{Handle, OptionSpec, PluginHandler, PluginSpec, RegisterError, RequestStatus, Tone};

use crate::engine::macros::{CaptureToggle, ToggleRequest, CAPTURE_ON};

/// Stable plugin id on the host (B1: the plugin is `training_rooms`).
pub const PLUGIN_ID: &str = "training_rooms";
/// Option id of the capture toggle.
pub const OPTION_ID: &str = "replay_capture";

/// Off is default text, On is green.
static TONES: [Tone; 2] = [Tone::Neutral, Tone::Ok];

/// The one row: Off / On, the same labels as the TrainingRooms menu row. `no_persist`: the writer thread owns the INI.
pub static OPTIONS: [OptionSpec; 1] =
    [OptionSpec { no_persist: true, tones: Some(&TONES), ..OptionSpec::new(OPTION_ID, "Replay capture", &["Off", "On"]) }];

/// A group on the shared Plugins page, no host persistence.
pub static SPEC: PluginSpec = PluginSpec::new(PLUGIN_ID, "Training Rooms", &OPTIONS);

/// The row's handler. `toggle` is the capture preference both rows change; `persist` schedules the INI write of the
/// preference's current value (it must not block or allocate: it runs on the host's draw thread).
#[derive(Clone, Copy)]
pub struct CaptureRow {
    pub toggle: fn() -> &'static CaptureToggle,
    pub persist: fn(),
}

impl PluginHandler for CaptureRow {
    fn current(&self, _option: usize) -> Option<usize> {
        (self.toggle)().current_choice()
    }

    fn request(&self, _option: usize, choice: usize) -> RequestStatus {
        let toggle = (self.toggle)();
        let before = toggle.desired();
        let answer = toggle.request_choice(choice);
        // Deluxe 0.10.1's L+R+X asks for the choice `current` already shows: an accepted request that leaves the preference
        // as it was writes nothing.
        if matches!(answer, ToggleRequest::Applied | ToggleRequest::Pending) && before != (choice == CAPTURE_ON) {
            (self.persist)();
        }
        status(answer)
    }
}

/// The toggle's answer as the host's request code.
pub fn status(answer: ToggleRequest) -> RequestStatus {
    match answer {
        ToggleRequest::Applied => RequestStatus::Applied,
        ToggleRequest::Pending => RequestStatus::Pending,
        ToggleRequest::Rejected => RequestStatus::Rejected,
        ToggleRequest::Unavailable => RequestStatus::Unavailable,
    }
}

/// What became of the row at boot, kept in one atomic byte by the runtime and named in the install line
/// (`deluxe_row=installed` or `deluxe_row=not_installed reason=<why>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RowState {
    /// Registration never ran: the named-macro / capture layer was not installed (a row of its closure was rejected), so
    /// there is no preference to show.
    LayerOff = 0,
    Registered = 1,
    /// No Deluxe host (not installed): the plugin runs without the row.
    HostMissing = 2,
    /// A host table with another size or table version.
    HostIncompatible = 3,
    /// `REGISTER_ERR_ABI`: a host older than Deluxe 0.5.3 (descriptor version 4 unknown to it).
    HostTooOld = 4,
    /// The spec failed the SDK's local check or `REGISTER_ERR_INVALID`.
    Invalid = 5,
    Null = 6,
    /// The host already holds its maximum plugin count.
    Full = 7,
    /// Another plugin registered `training_rooms`.
    Duplicate = 8,
    Callbacks = 9,
    /// A negative code this SDK does not know.
    HostError = 10,
}

impl RowState {
    const ALL: [RowState; 11] = [
        RowState::LayerOff,
        RowState::Registered,
        RowState::HostMissing,
        RowState::HostIncompatible,
        RowState::HostTooOld,
        RowState::Invalid,
        RowState::Null,
        RowState::Full,
        RowState::Duplicate,
        RowState::Callbacks,
        RowState::HostError,
    ];

    pub fn from_result(result: &Result<Handle, RegisterError>) -> Self {
        match result {
            Ok(_) => RowState::Registered,
            Err(RegisterError::HostMissing) => RowState::HostMissing,
            Err(RegisterError::HostIncompatible) => RowState::HostIncompatible,
            Err(RegisterError::Abi) => RowState::HostTooOld,
            Err(RegisterError::Invalid) => RowState::Invalid,
            Err(RegisterError::Null) => RowState::Null,
            Err(RegisterError::Full) => RowState::Full,
            Err(RegisterError::Duplicate) => RowState::Duplicate,
            Err(RegisterError::Callbacks) => RowState::Callbacks,
            Err(RegisterError::Unknown(_)) => RowState::HostError,
        }
    }

    pub const fn code(self) -> u8 {
        self as u8
    }

    /// An unknown byte reads as `LayerOff` (the initial value).
    pub fn from_code(code: u8) -> Self {
        Self::ALL.into_iter().find(|state| state.code() == code).unwrap_or(RowState::LayerOff)
    }

    pub fn registered(self) -> bool {
        self == RowState::Registered
    }

    /// The install line's `reason=` when the row is not registered.
    pub fn reason(self) -> Option<&'static str> {
        Some(match self {
            RowState::Registered => return None,
            RowState::LayerOff => "capture_layer_off",
            RowState::HostMissing => "host_missing",
            RowState::HostIncompatible => "host_incompatible",
            RowState::HostTooOld => "host_older_than_0_5_3",
            RowState::Invalid => "invalid_spec",
            RowState::Null => "host_null",
            RowState::Full => "host_full",
            RowState::Duplicate => "duplicate_id",
            RowState::Callbacks => "callbacks",
            RowState::HostError => "host_error",
        })
    }
}

