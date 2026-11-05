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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::macros::CAPTURE_OFF;
    use deluxe_plugin_sdk::{abi, register, register_with};
    use std::cell::{Cell, RefCell};
    use std::ffi::{c_char, c_void, CStr};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// What a fake host kept from one `register` call: the user pointer, the callbacks and copies of the strings.
    #[derive(Default)]
    struct Registered {
        calls: u32,
        versions: Vec<u32>,
        user: usize,
        current: Option<abi::CurrentFn>,
        request: Option<abi::RequestFn>,
        id: String,
        label: String,
        options: Vec<Opt>,
        config_path_null: bool,
        flags: u32,
    }

    /// One option as the host copied it.
    struct Opt {
        id: String,
        label: String,
        choices: Vec<String>,
        flags: u32,
        key_null: bool,
        tones: Vec<u8>,
    }

    thread_local! {
        static HOST: RefCell<Registered> = RefCell::new(Registered::default());
        static ACCEPTED: Cell<(u32, u32)> = const { Cell::new((2, 5)) };
    }

    unsafe fn text(ptr: *const c_char) -> String {
        unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_owned()
    }

    /// A fake host whose `register` applies the checks the card's Deluxe 0.10.1 `register` (`.text+0x1fdb0`) applies
    /// first, as read from the NRO: null descriptor -> -1; `struct_size != 0x48` or `abi_version - 2 > 3` -> -2
    /// (descriptor versions 2..=5 accepted); `flags > 1` -> -3; a null callback -> -6. `ACCEPTED` narrows the version
    /// range to model an older host. It copies what it needs, as the real host does.
    unsafe extern "C" fn fake_register(desc: *const abi::PluginDesc) -> i32 {
        if desc.is_null() {
            return abi::REGISTER_ERR_NULL;
        }
        let desc = unsafe { &*desc };
        let (low, high) = ACCEPTED.with(Cell::get);
        HOST.with(|host| {
            let mut host = host.borrow_mut();
            host.calls += 1;
            host.versions.push(desc.abi_version);
        });
        if desc.struct_size != 0x48 || desc.abi_version.wrapping_sub(2) > 3 || desc.abi_version < low || desc.abi_version > high {
            return abi::REGISTER_ERR_ABI;
        }
        if desc.flags > 1 {
            return abi::REGISTER_ERR_INVALID;
        }
        if desc.current.is_none() || desc.request.is_none() {
            return abi::REGISTER_ERR_CALLBACKS;
        }
        let options = (0..desc.option_count as usize)
            .map(|i| {
                let o = unsafe { &*desc.options.add(i) };
                let choices = (0..o.choice_count as usize).map(|c| unsafe { text(*o.choices.add(c)) }).collect();
                let tones = if o.choice_tones.is_null() {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(o.choice_tones, o.choice_count as usize) }.to_vec()
                };
                Opt { id: unsafe { text(o.id) }, label: unsafe { text(o.label) }, choices, flags: o.flags, key_null: o.config_key.is_null(), tones }
            })
            .collect();
        HOST.with(|host| {
            let mut host = host.borrow_mut();
            host.user = desc.user as usize;
            host.current = desc.current;
            host.request = desc.request;
            host.id = unsafe { text(desc.id) };
            host.label = unsafe { text(desc.label) };
            host.options = options;
            host.config_path_null = desc.config_path.is_null();
            host.flags = desc.flags;
        });
        0
    }

    unsafe extern "C" fn fake_status() -> u32 {
        abi::HOST_STATUS_MENU_ACTIVE
    }

    unsafe extern "C" fn fake_build() -> *const c_char {
        c"0.10.1-test".as_ptr()
    }

    /// The table the card's 0.10.1 host returns from `ssbu_online_deluxe_plugin_api_v1` (`.data+0x1155d8`):
    /// `struct_size = 0x20`, table `abi_version = 1`, then `register`, `host_status`, `host_build`.
    fn host_0101() -> abi::PluginApiV1 {
        abi::PluginApiV1 { struct_size: 0x20, abi_version: 1, register: fake_register, host_status: fake_status, host_build: fake_build }
    }

    fn reset(accepted: (u32, u32)) {
        HOST.with(|host| *host.borrow_mut() = Registered::default());
        ACCEPTED.with(|a| a.set(accepted));
    }

    // One toggle and one persist counter per test: each test names its own statics so tests run in parallel.
    macro_rules! row {
        ($toggle:ident, $writes:ident) => {{
            static $toggle: CaptureToggle = CaptureToggle::new();
            static $writes: AtomicUsize = AtomicUsize::new(0);
            fn toggle() -> &'static CaptureToggle {
                &$toggle
            }
            fn persist() {
                $writes.fetch_add(1, Ordering::SeqCst);
            }
            (CaptureRow { toggle, persist }, &$toggle, &$writes)
        }};
    }

    /// Calls `current` / `request` the way the host does: through the C callbacks it was handed, with its `user`.
    fn host_current(option: u32) -> u32 {
        let (current, user) = HOST.with(|h| (h.borrow().current.unwrap(), h.borrow().user));
        unsafe { current(user as *mut c_void, option) }
    }

    fn host_request(option: u32, choice: u32) -> u32 {
        let (request, user) = HOST.with(|h| (h.borrow().request.unwrap(), h.borrow().user));
        unsafe { request(user as *mut c_void, option, choice) }
    }

    #[test]
    fn version_4_registers_on_the_0101_host_in_one_call_with_no_persistence() {
        reset((2, 5));
        let (row, _, _) = row!(T, W);
        let result = unsafe { register_with(&host_0101(), &SPEC, row) };
        assert_eq!(result, Ok(Handle(0)));
        assert_eq!(RowState::from_result(&result), RowState::Registered);
        HOST.with(|host| {
            let host = host.borrow();
            assert_eq!(host.calls, 1, "the SDK calls register exactly once");
            assert_eq!(host.versions, vec![abi::ABI_VERSION]);
            assert_eq!(abi::ABI_VERSION, 4);
            assert_eq!((host.id.as_str(), host.label.as_str()), ("training_rooms", "Training Rooms"));
            assert_eq!(host.flags, 0, "a group on the shared Plugins page");
            assert!(host.config_path_null, "the host is not asked to write replay_capture.ini");
            assert_eq!(host.options.len(), 1);
            let option = &host.options[0];
            assert_eq!((option.id.as_str(), option.label.as_str()), ("replay_capture", "Replay capture"));
            assert_eq!(option.choices, ["Off", "On"]);
            assert_eq!(option.flags, abi::OPTION_FLAG_NO_PERSIST);
            assert!(option.key_null);
            assert_eq!(option.tones, [abi::TONE_NEUTRAL, abi::TONE_OK]);
        });
    }

    #[test]
    fn a_host_older_than_0_5_3_answers_abi_once_and_is_not_retried() {
        // Hosts before 0.5.3 took descriptor versions 2..=3 only.
        reset((2, 3));
        let (row, _, _) = row!(T, W);
        let result = unsafe { register_with(&host_0101(), &SPEC, row) };
        assert_eq!(result, Err(RegisterError::Abi));
        assert_eq!(RowState::from_result(&result), RowState::HostTooOld);
        assert_eq!(RowState::HostTooOld.reason(), Some("host_older_than_0_5_3"));
        HOST.with(|host| assert_eq!(host.borrow().versions, vec![4], "no fallback to an older descriptor version"));
    }

    #[test]
    fn a_table_of_another_shape_is_incompatible_and_never_called() {
        for (size, version) in [(0x20, 2), (0x18, 1), (0x28, 1)] {
            reset((2, 5));
            let (row, _, _) = row!(T, W);
            let api = abi::PluginApiV1 { struct_size: size, abi_version: version, ..host_0101() };
            let result = unsafe { register_with(&api, &SPEC, row) };
            assert_eq!(result, Err(RegisterError::HostIncompatible), "{size:#x}/{version}");
            assert_eq!(RowState::from_result(&result).reason(), Some("host_incompatible"));
            HOST.with(|host| assert_eq!(host.borrow().calls, 0));
        }
    }

    #[test]
    fn no_host_off_the_console_means_no_row_and_nothing_else() {
        let (row, toggle, writes) = row!(T, W);
        toggle.seed(true);
        toggle.set_ready();
        let result = register(&SPEC, row);
        assert_eq!(result, Err(RegisterError::HostMissing));
        assert_eq!(RowState::from_result(&result).reason(), Some("host_missing"));
        assert_eq!(writes.load(Ordering::SeqCst), 0);
        assert!(toggle.desired());
    }

    #[test]
    fn the_host_reads_and_changes_the_one_preference_and_each_change_schedules_one_write() {
        reset((2, 5));
        let (row, toggle, writes) = row!(T, W);
        toggle.seed(false);
        unsafe { register_with(&host_0101(), &SPEC, row) }.unwrap();
        // Before the capture adapter is ready the row shows Unavailable and changes nothing.
        assert_eq!(host_current(0), abi::CHOICE_UNAVAILABLE);
        assert_eq!(host_request(0, 1), abi::REQUEST_UNAVAILABLE);
        assert!(!toggle.desired());
        assert_eq!(writes.load(Ordering::SeqCst), 0);
        toggle.set_ready();
        assert_eq!(host_current(0), CAPTURE_OFF as u32);
        assert_eq!(host_request(0, CAPTURE_ON as u32), abi::REQUEST_APPLIED);
        assert!(toggle.desired());
        assert_eq!(host_current(0), CAPTURE_ON as u32);
        assert_eq!(writes.load(Ordering::SeqCst), 1);
        // L+R+X (0.10.1): a request for the choice already shown is applied and writes nothing.
        assert_eq!(host_request(0, CAPTURE_ON as u32), abi::REQUEST_APPLIED);
        assert_eq!(writes.load(Ordering::SeqCst), 1);
        // The menu row changes the same preference: the host sees it on its next frame.
        assert_eq!(toggle.request_choice(CAPTURE_OFF), ToggleRequest::Applied);
        assert_eq!(host_current(0), CAPTURE_OFF as u32);
        // Out-of-range indices never reach the handler (the SDK's trampolines).
        assert_eq!(host_request(0, 2), abi::REQUEST_REJECTED);
        assert_eq!(host_request(1, 0), abi::REQUEST_REJECTED);
        assert_eq!(host_current(1), abi::CHOICE_UNAVAILABLE);
        assert_eq!(writes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_change_during_a_record_is_pending_shows_the_latched_value_and_is_written_at_once() {
        reset((2, 5));
        let (row, toggle, writes) = row!(T, W);
        toggle.seed(true);
        toggle.set_ready();
        unsafe { register_with(&host_0101(), &SPEC, row) }.unwrap();
        assert!(toggle.begin_record());
        assert_eq!(host_request(0, CAPTURE_OFF as u32), abi::REQUEST_PENDING);
        // `current` keeps the record's arm state until the record ends; the INI holds the next-record preference.
        assert_eq!(host_current(0), CAPTURE_ON as u32);
        assert!(!toggle.desired());
        assert_eq!(writes.load(Ordering::SeqCst), 1);
        toggle.end_record().unwrap();
        assert_eq!(host_current(0), CAPTURE_OFF as u32);
        // Stepping back to the live value before the record ends is a second change, a second write.
        assert!(!toggle.begin_record());
        assert_eq!(host_request(0, CAPTURE_ON as u32), abi::REQUEST_PENDING);
        assert_eq!(host_request(0, CAPTURE_OFF as u32), abi::REQUEST_APPLIED);
        assert_eq!(writes.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn the_handler_maps_every_toggle_answer() {
        assert_eq!(status(ToggleRequest::Applied), RequestStatus::Applied);
        assert_eq!(status(ToggleRequest::Pending), RequestStatus::Pending);
        assert_eq!(status(ToggleRequest::Rejected), RequestStatus::Rejected);
        assert_eq!(status(ToggleRequest::Unavailable), RequestStatus::Unavailable);
    }

    #[test]
    fn every_register_error_has_a_state_a_reason_and_a_code_that_round_trips() {
        let errors = [
            (RegisterError::HostMissing, "host_missing"),
            (RegisterError::HostIncompatible, "host_incompatible"),
            (RegisterError::Abi, "host_older_than_0_5_3"),
            (RegisterError::Invalid, "invalid_spec"),
            (RegisterError::Null, "host_null"),
            (RegisterError::Full, "host_full"),
            (RegisterError::Duplicate, "duplicate_id"),
            (RegisterError::Callbacks, "callbacks"),
            (RegisterError::Unknown(-9), "host_error"),
        ];
        for (error, reason) in errors {
            let state = RowState::from_result(&Err(error));
            assert!(!state.registered());
            assert_eq!(state.reason(), Some(reason));
            assert_eq!(RowState::from_code(state.code()), state);
        }
        assert_eq!(RowState::from_code(RowState::Registered.code()), RowState::Registered);
        assert_eq!(RowState::Registered.reason(), None);
        assert_eq!(RowState::from_code(0), RowState::LayerOff);
        assert_eq!(RowState::LayerOff.reason(), Some("capture_layer_off"));
        assert_eq!(RowState::from_code(200), RowState::LayerOff);
    }

    #[test]
    fn other_host_answers_map_to_their_states() {
        for (code, state) in [
            (abi::REGISTER_ERR_FULL, RowState::Full),
            (abi::REGISTER_ERR_DUPLICATE, RowState::Duplicate),
            (abi::REGISTER_ERR_INVALID, RowState::Invalid),
            (-42, RowState::HostError),
        ] {
            thread_local!(static CODE: Cell<i32> = const { Cell::new(0) });
            unsafe extern "C" fn answer(_: *const abi::PluginDesc) -> i32 {
                CODE.with(Cell::get)
            }
            CODE.with(|c| c.set(code));
            let (row, _, _) = row!(T, W);
            let api = abi::PluginApiV1 { register: answer, ..host_0101() };
            assert_eq!(RowState::from_result(&unsafe { register_with(&api, &SPEC, row) }), state);
        }
    }
}
