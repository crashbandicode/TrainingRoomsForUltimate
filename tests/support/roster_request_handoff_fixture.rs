use runtime::lobby::training::{self, lobby};
type Request = GenericRequest;
use engine::swap::cycle_members::{admitted, over_pending, Admission};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {{ let _ = format_args!($($arg)*); }};
}

struct Lock<T>(Mutex<T>);
impl<T> Lock<T> {
    const fn new(value: T) -> Self {
        Self(Mutex::new(value))
    }
    fn lock(&self) -> MutexGuard<'_, T> {
        self.0.lock().unwrap_or_else(|poison| poison.into_inner())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
enum Role {
    Player = 0,
    Cpu = 1,
}
impl Role {
    fn enabled(self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fighter(u32);
impl Fighter {
    fn kind(self) -> u32 {
        self.0
    }
}

struct TrioFighter;
impl TrioFighter {
    fn from_kind(kind: u32) -> Option<Fighter> {
        matches!(kind, 0 | 0x48 | 0x54).then_some(Fighter(kind))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Selection {
    Cycle,
    Direct(Fighter),
    RefreshPartners,
}

#[derive(Clone, Copy)]
struct RequestScope {
    owner: usize,
    waiting: bool,
}
impl RequestScope {
    fn waiting(&self) -> bool {
        self.waiting
    }
}

#[derive(Clone, Copy)]
struct GenericRequest {
    scope: RequestScope,
    epoch: u64,
    entry: usize,
    selection: Selection,
    age: u8,
    restoring: bool,
    once: bool,
}
struct RosterState {
    pending: Option<GenericRequest>,
    active: Option<GenericRequest>,
    observed: Option<(usize, u32)>,
    epoch: u64,
    cursor: Option<usize>,
}
impl RosterState {
    const fn new(observed: bool) -> Self {
        Self {
            pending: None,
            active: None,
            observed: if observed { Some((17, 0)) } else { None },
            epoch: 1,
            cursor: None,
        }
    }
}
static STATES: [Lock<RosterState>; 2] = [
    Lock::new(RosterState::new(true)),
    Lock::new(RosterState::new(true)),
];
fn state_for(role: Role) -> &'static Lock<RosterState> {
    &STATES[role as usize]
}
fn reset_roster(role: Role, observed: bool) {
    *state_for(role).lock() = RosterState::new(observed);
}
fn generic_pending(role: Role) -> bool {
    state_for(role).lock().pending.is_some()
}

struct WaitingValue {
    owned: bool,
    fighter: Fighter,
}
struct WaitingMemory;
impl WaitingMemory {
    fn restore_target(&self, _: usize) -> Option<WaitingValue> {
        None
    }
    fn manual_request(&mut self, _: usize) {}
}
static WAITING_MEMORY: Lock<WaitingMemory> = Lock::new(WaitingMemory);

struct Install;
impl Install {
    fn suspends_practice(&self) -> bool {
        GLOBAL_BUSY.load(Ordering::Relaxed)
    }
    fn active(&self) -> bool {
        GLOBAL_BUSY.load(Ordering::Relaxed)
    }
}
static INSTALL: Install = Install;
static BASE: AtomicUsize = AtomicUsize::new(1);
static GLOBAL_BUSY: AtomicBool = AtomicBool::new(false);
static RECORDING: AtomicBool = AtomicBool::new(false);
static LOADING_SAVE: AtomicBool = AtomicBool::new(false);
static NATIVE_MENU_BUSY: AtomicBool = AtomicBool::new(false);
static SHORTCUT_PENDING: AtomicBool = AtomicBool::new(false);
static MENU_CAPTURED: AtomicBool = AtomicBool::new(false);
static SCOPE_ALLOWED: AtomicBool = AtomicBool::new(true);

fn original_kind(_: Role) -> Option<u32> {
    Some(0)
}
fn busy() -> bool {
    GLOBAL_BUSY.load(Ordering::Relaxed)
}
fn scope(_: Role) -> Option<RequestScope> {
    SCOPE_ALLOWED
        .load(Ordering::Relaxed)
        .then_some(RequestScope {
            owner: 1,
            waiting: false,
        })
}
static NOTICES: Lock<Vec<String>> = Lock::new(Vec::new());
static SAVED_RESULT: Lock<Option<bool>> = Lock::new(None);
fn notify(_: Role, message: String) { NOTICES.lock().push(message); }
fn preload_targets(
    _: &TrainingModpackMenu,
    _: usize,
) -> [Option<runtime::roster::cycle_preload::Target>; 1] {
    [None]
}
fn queue(role: Role, selection: Selection) {
    queue_admitted(role, selection, false, false);
}

#[derive(Clone, Copy)]
struct TrainingModpackMenu {
    cpu_roster: CpuRoster,
}
#[derive(Clone, Copy)]
struct CpuRoster;
impl CpuRoster {
    fn fighters_excluding_fixed(&self, _: Option<usize>) -> [Fighter; 1] {
        [Fighter(0x48)]
    }
}
static MENU: TrainingModpackMenu = TrainingModpackMenu {
    cpu_roster: CpuRoster,
};
fn setting(menu: &TrainingModpackMenu) -> TrainingModpackMenu {
    *menu
}

mod button_config {
    pub fn has_pending_request() -> bool {
        super::SHORTCUT_PENDING.load(super::Ordering::Relaxed)
    }
}
mod menu {
    pub fn captures_input() -> bool {
        super::MENU_CAPTURED.load(super::Ordering::Relaxed)
    }
}
mod sources {
    pub fn pause_preload() {}
}

mod runtime {
    pub mod roster {
        pub mod character_lifecycle {
            pub fn native_hooks_ready() -> bool {
                true
            }
        }
        pub mod cycle_preload_config {
            pub struct Snapshot {
                pub measurements: bool,
            }
            pub fn snapshot() -> Snapshot {
                Snapshot {
                    measurements: false,
                }
            }
        }
        pub mod cycle_preload {
            #[derive(Clone, Copy)]
            pub struct Target;
            impl Target {
                pub fn fighter(_: u32) -> Self {
                    Self
                }
            }
            pub fn request(_: usize, _: bool, _: Target) {}
        }
    }
    pub mod lobby {
        pub mod training {
            pub fn recording_or_playback() -> bool {
                super::super::super::RECORDING.load(super::super::super::Ordering::Relaxed)
            }
            pub mod save_states {
                pub fn is_loading() -> bool {
                    super::super::super::super::LOADING_SAVE
                        .load(super::super::super::super::Ordering::Relaxed)
                }
            }
            pub mod lobby {
                pub fn native_ui_busy() -> bool {
                    super::super::super::super::NATIVE_MENU_BUSY
                        .load(super::super::super::super::Ordering::Relaxed)
                }
            }
        }
    }
}

mod mii_auto_init {
    use super::Role;
    pub fn initializing() -> bool {
        false
    }
    pub fn origin_capture_pending(_: Role, _: bool, _: bool) -> bool {
        false
    }
}
mod mii_backend {
    pub unsafe fn ready() -> bool {
        true
    }
    pub fn current() -> Option<()> {
        None
    }
}
mod engine {
    pub mod swap {
        pub mod cycle_members {
            // ADMISSION_MODEL
        }
    }
    pub mod mii {
        pub mod auto_init {
            pub fn requires_owner(_: Option<u32>, _: bool, _: Option<u32>) -> bool {
                false
            }
        }
    }
}

mod mii_ordinary_player {
    use super::{Role, Selection};
    // PLAYER_INTERCEPT_STUB
    pub fn preload_pending() -> bool {
        false
    }
}
mod mii_saved_alpha {
    use super::{Role, Selection};
    pub fn menu_epoch() -> u64 {
        1
    }
    // SAVED_ALPHA_INTERCEPT_STUB
}
mod mii_ordinary_cpu {
    use super::*;

    #[derive(Clone, Copy)]
    struct Binding {
        entry: usize,
        epoch: u64,
    }
    #[derive(Clone, Copy)]
    struct Origin {
        binding: Binding,
    }
    #[derive(Clone, Copy)]
    struct LiveTarget;
    impl LiveTarget {
        fn saved(&self) -> Option<()> {
            None
        }
    }
    #[derive(Clone, Copy)]
    struct Live {
        target: LiveTarget,
    }
    #[derive(Clone, Copy)]
    struct Owner {
        origin: Origin,
        origin_saved: Option<()>,
        live: Live,
    }
    #[derive(Clone, Copy)]
    struct Reservation;
    impl Reservation {
        fn routes(&self, _: bool, _: bool) -> bool {
            true
        }
    }

    #[derive(Clone, Copy)]
    struct Request {
        selection: Selection,
    }
    impl Request {
        const CYCLE: Self = Self {
            selection: Selection::Cycle,
        };
        const REFRESH_PARTNERS: Self = Self {
            selection: Selection::RefreshPartners,
        };
        fn direct(_: u64, kind: u32) -> Option<Self> {
            Some(Self {
                selection: Selection::Direct(Fighter(kind)),
            })
        }
    }
    struct Waiting;
    impl Waiting {
        fn new() -> Self {
            Self
        }
    }

    struct State {
        owner: Option<Owner>,
        capture: bool,
        reservation: Reservation,
        departed: bool,
        request: Option<Request>,
        restore: Option<()>,
        waiting: Waiting,
        occupied: bool,
        menu_valid: bool,
        catalog: Option<()>,
    }
    impl State {
        const fn empty() -> Self {
            Self {
                owner: None,
                capture: false,
                reservation: Reservation,
                departed: false,
                request: None,
                restore: None,
                waiting: Waiting,
                occupied: false,
                menu_valid: true,
                catalog: Some(()),
            }
        }
        fn busy(&self) -> bool {
            self.occupied
        }
    }
    static STATE: Lock<State> = Lock::new(State::empty());
    static ENABLED: AtomicBool = AtomicBool::new(true);

    fn epoch() -> u64 {
        1
    }
    fn note(reason: impl std::fmt::Display) {
        trace!("[MiiOrdinaryCpu] {}", reason);
    }

    pub fn reset(with_owner: bool) {
        let mut state = STATE.lock();
        *state = State::empty();
        if with_owner {
            state.owner = Some(Owner {
                origin: Origin {
                    binding: Binding { entry: 4, epoch: 1 },
                },
                origin_saved: None,
                live: Live { target: LiveTarget },
            });
        }
    }
    pub fn set_busy(value: bool) {
        STATE.lock().occupied = value;
    }
    pub fn set_departed(value: bool) {
        STATE.lock().departed = value;
    }
    pub fn set_request(value: bool) {
        STATE.lock().request = value.then_some(Request::CYCLE);
    }
    pub fn request_pending() -> bool {
        STATE.lock().request.is_some()
    }
    pub fn clear_request() {
        STATE.lock().request = None;
    }

    struct MiiClass;
    impl MiiClass {
        fn from_kind(kind: u32) -> Option<()> {
            (0x48..=0x4a).contains(&kind).then_some(())
        }
    }

    // CPU_PRELOAD_PENDING
    // CPU_INTERCEPT
}

// NOTICE_BLOCKED
// QUEUE_ADMITTED
// CPU_RELOAD_READY
// REQUEST_CPU_KIND

fn reset_fixture() {
    NOTICES.lock().clear();
    *SAVED_RESULT.lock() = None;
    BASE.store(1, Ordering::Relaxed);
    GLOBAL_BUSY.store(false, Ordering::Relaxed);
    RECORDING.store(false, Ordering::Relaxed);
    LOADING_SAVE.store(false, Ordering::Relaxed);
    NATIVE_MENU_BUSY.store(false, Ordering::Relaxed);
    SHORTCUT_PENDING.store(false, Ordering::Relaxed);
    MENU_CAPTURED.store(false, Ordering::Relaxed);
    SCOPE_ALLOWED.store(true, Ordering::Relaxed);
    reset_roster(Role::Player, true);
    reset_roster(Role::Cpu, true);
    mii_ordinary_cpu::reset(false);
    *WAITING_MEMORY.lock() = WaitingMemory;
}

// Kinds are from TR's fighter table: ordinary Banjo 0x54 and Mii Brawler 0x48.
#[test]
fn accepted_alternate_owner_is_reported_with_no_generic_pending() {
    for kind in [0x54, 0x48] {
        reset_fixture();
        mii_ordinary_cpu::reset(true);
        assert!(!INSTALL.active());
        assert!(
            request_cpu_kind(kind),
            "accepted owner request must report true: {kind:#x}"
        );
        assert!(mii_ordinary_cpu::request_pending());
        assert!(!generic_pending(Role::Cpu));
    }
}

#[test]
fn cold_owner_request_blocks_reload_until_its_native_work_settles() {
    reset_fixture();
    mii_ordinary_cpu::reset(true);
    // Seed through the actual intercept so old facade failure cannot preempt this independent case.
    assert_eq!(
        mii_ordinary_cpu::intercept(Role::Cpu, Selection::Direct(Fighter(0x54)), true),
        Some(Admission::Accepted)
    );
    assert!(mii_ordinary_cpu::request_pending());
    assert!(!generic_pending(Role::Cpu));
    assert!(
        !INSTALL.active(),
        "cold-demand wait precedes the visible INSTALL ticket"
    );
    assert!(notice_blocked());
    assert!(
        !cpu_reload_ready(),
        "pending owner request must not expose the outgoing body as completed"
    );

    // The existing owner then holds native work while the queued request has been consumed.
    mii_ordinary_cpu::set_busy(true);
    GLOBAL_BUSY.store(true, Ordering::Relaxed);
    mii_ordinary_cpu::clear_request();
    assert!(!cpu_reload_ready());
    mii_ordinary_cpu::set_busy(false);
    GLOBAL_BUSY.store(false, Ordering::Relaxed);
    assert!(!notice_blocked());
    assert!(cpu_reload_ready());
}

#[derive(Clone, Copy, Debug)]
enum Refusal {
    Busy,
    Departed,
    AlreadyPending,
}
#[test]
fn consumed_cpu_owner_refusals_do_not_fall_through_to_generic_loader() {
    for refusal in [Refusal::Busy, Refusal::Departed, Refusal::AlreadyPending] {
        reset_fixture();
        mii_ordinary_cpu::reset(true);
        match refusal {
            Refusal::Busy => mii_ordinary_cpu::set_busy(true),
            Refusal::Departed => mii_ordinary_cpu::set_departed(true),
            Refusal::AlreadyPending => mii_ordinary_cpu::set_request(true),
        }
        assert!(
            !request_cpu_kind(0x48),
            "owner refusal should be reported: {refusal:?}"
        );
        assert!(
            !generic_pending(Role::Cpu),
            "owner refusal must not create a generic fallback: {refusal:?}"
        );
    }

    reset_fixture();
    mii_ordinary_cpu::reset(true);
    GLOBAL_BUSY.store(true, Ordering::Relaxed);
    assert!(
        !request_cpu_kind(0x48),
        "protected generic loader state refuses the request"
    );
    assert!(!generic_pending(Role::Cpu));

    reset_fixture();
    mii_ordinary_cpu::reset(true);
    assert!(
        !request_cpu_kind(u32::MAX),
        "unsupported fighter is refused before routing"
    );
    assert!(!generic_pending(Role::Cpu));
}

#[test]
fn generic_loader_acceptance_and_refusal_are_reported() {
    reset_fixture();
    mii_ordinary_cpu::reset(false);
    assert!(
        request_cpu_kind(0),
        "generic pending-store is an accepted request"
    );
    assert!(generic_pending(Role::Cpu));
    assert!(!mii_ordinary_cpu::request_pending());

    reset_fixture();
    mii_ordinary_cpu::reset(false);
    reset_roster(Role::Cpu, false);
    assert!(
        !request_cpu_kind(0),
        "generic refusal with no observed entry is reported"
    );
    assert!(!generic_pending(Role::Cpu));
}

#[test]
fn cpu_facade_reports_only_admitted_outcomes() {
    for (outcome, accepted) in [
        (Admission::Accepted, true),
        (Admission::Superseded, true),
        (Admission::Busy, false),
        (Admission::Unavailable, false),
        (Admission::Owned, false),
    ] {
        reset_fixture();
        let mut calls = 0;
        assert_eq!(request_cpu_kind_with(0, |fighter| {
            calls += 1;
            assert_eq!(fighter, Fighter(0));
            outcome
        }), accepted, "{outcome:?}");
        assert_eq!(calls, 1);
        assert!(!generic_pending(Role::Cpu));
    }
}

#[test]
fn saved_mii_consumption_does_not_imply_acceptance_or_generic_fallback() {
    for (saved, outcome, accepted) in [
        (true, Admission::Accepted, true),
        (false, Admission::Owned, false),
    ] {
        reset_fixture();
        *SAVED_RESULT.lock() = Some(saved);
        assert_eq!(queue_admitted(Role::Cpu, Selection::Direct(Fighter(0)), false, false), outcome);
        assert!(!generic_pending(Role::Cpu));
        assert_eq!(request_cpu_kind(0), accepted);
        assert!(!generic_pending(Role::Cpu));
    }
}

#[test]
fn explicit_pick_preserves_pending_user_but_replaces_automatic_request() {
    for (restoring, selection, expected) in [
        (false, Selection::Direct(Fighter(0)), Admission::Busy),
        (true, Selection::Direct(Fighter(0)), Admission::Superseded),
        (false, Selection::RefreshPartners, Admission::Superseded),
    ] {
        reset_fixture();
        state_for(Role::Cpu).lock().pending = Some(GenericRequest {
            scope: scope(Role::Cpu).unwrap(), epoch: 1, entry: 17,
            selection, age: 7, restoring, once: false,
        });
        let picked = Selection::Direct(Fighter(0x54));
        assert_eq!(queue_admitted(Role::Cpu, picked, false, true), expected);
        let pending = state_for(Role::Cpu).lock().pending.unwrap();
        if expected == Admission::Busy {
            assert_eq!(pending.selection, selection);
            assert_eq!(pending.age, 7, "refusal preserves the original request");
            assert_eq!(NOTICES.lock().last().map(String::as_str),
                Some(engine::swap::cycle_members::FIGHTER_STILL_LOADING));
        } else {
            assert_eq!(pending.selection, picked);
            assert_eq!(pending.age, 0);
            assert!(!pending.restoring);
            assert!(pending.once);
        }
    }
}
