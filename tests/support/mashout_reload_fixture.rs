use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
static OWN_CPU: AtomicUsize = AtomicUsize::new(0);
static CPU_ENTRY: AtomicUsize = AtomicUsize::new(0);
static ACCESSOR: AtomicUsize = AtomicUsize::new(0);
static BODY: AtomicUsize = AtomicUsize::new(0);
static READY: AtomicBool = AtomicBool::new(false);
static REVOKED: AtomicUsize = AtomicUsize::new(0);
static TEMP_KIND: AtomicU32 = AtomicU32::new(u32::MAX);
static ACCEPT: AtomicBool = AtomicBool::new(false);
static REQUESTED: AtomicU32 = AtomicU32::new(u32::MAX);
static CANCELLED: AtomicUsize = AtomicUsize::new(0);
static CPU_STATUS: AtomicU32 = AtomicU32::new(0);
static P1_AIR: AtomicBool = AtomicBool::new(false);
static CPU_AIR: AtomicBool = AtomicBool::new(false);
static P1_STOP: AtomicBool = AtomicBool::new(false);
static CPU_STOP: AtomicBool = AtomicBool::new(false);
static LINKED: AtomicBool = AtomicBool::new(false);
static REQUEST_COUNT: AtomicUsize = AtomicUsize::new(0);
static DRIVEN: AtomicUsize = AtomicUsize::new(0);
static GENERATION: AtomicU64 = AtomicU64::new(5);
static MENU_OPEN: AtomicBool = AtomicBool::new(false);
struct MappedInputs;
impl MappedInputs {
    const NEUTRAL: Self = MappedInputs;
}
fn drive(_: MappedInputs) {
    DRIVEN.fetch_add(1, Ordering::Relaxed);
}
fn own_result(_: u64) {}
mod worker {
    pub enum Event {
        Cancel(u64),
    }
}
fn post(event: worker::Event, generation: u64) {
    let worker::Event::Cancel(value) = event;
    assert_eq!(value, generation);
    CANCELLED.fetch_add(1, Ordering::Relaxed);
}
fn revoke() {
    REVOKED.fetch_add(1, Ordering::Relaxed);
}
mod natives {
    pub fn status_kind(_: usize) -> u32 {
        super::CPU_STATUS.load(super::Ordering::Relaxed)
    }
    pub fn situation_kind(accessor: usize) -> u32 {
        u32::from(
            if accessor == 11 {
                &super::P1_AIR
            } else {
                &super::CPU_AIR
            }
            .load(super::Ordering::Relaxed),
        )
    }
    pub fn is_stop(accessor: usize) -> bool {
        if accessor == 11 {
            &super::P1_STOP
        } else {
            &super::CPU_STOP
        }
        .load(super::Ordering::Relaxed)
    }
    pub fn module_accessor(_: usize) -> Option<usize> {
        let a = super::ACCESSOR.load(super::Ordering::Relaxed);
        (a != 0).then_some(a)
    }
    pub fn fighter_entry(entry: usize) -> usize {
        entry
    }
    pub fn entry_active_body(_: usize) -> usize {
        super::BODY.load(super::Ordering::Relaxed)
    }
}
mod runtime {
    pub mod ui {
        pub mod menu_app {
            pub fn active() -> bool {
                crate::MENU_OPEN.load(crate::Ordering::Relaxed)
            }
        }
    }
    pub mod roster {
        pub mod character_roster {
            pub fn cpu_reload_ready() -> bool {
                crate::READY.load(crate::Ordering::Relaxed)
            }
            pub fn request_practice_cpu_kind(kind: u32) -> bool {
                crate::REQUESTED.store(kind, crate::Ordering::Relaxed);
                crate::REQUEST_COUNT.fetch_add(1, crate::Ordering::Relaxed);
                crate::ACCEPT.load(crate::Ordering::Relaxed)
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Off,
    Choose,
    Reload,
    Release,
    Restore,
    Place,
    Entering,
    Held,
    Result,
}
#[derive(Clone, Copy)]
struct Request {
    attacker_kind: u32,
}
struct Game {
    generation: u64,
    step: Step,
    requested: Option<Request>,
    p1: usize,
    cpu: usize,
    failed: Option<&'static str>,
    continued: usize,
    original: Option<u32>,
    restore_requested: bool,
    config: Option<()>,
    roll: Option<()>,
    clock: Option<()>,
    hero: Option<()>,
    trail_magic: Option<()>,
    prep_damage: Option<()>,
}
impl Game {
    fn new() -> Self {
        Self::at(Step::Off)
    }
    fn at(step: Step) -> Self {
        Self {
            generation: 5,
            step,
            requested: Some(Request { attacker_kind: 6 }),
            p1: 11,
            cpu: 22,
            failed: None,
            continued: 0,
            original: Some(4),
            restore_requested: false,
            config: Some(()),
            roll: None,
            clock: None,
            hero: None,
            trail_magic: None,
            prep_damage: None,
        }
    }
    fn fail(&mut self, text: &'static str) {
        self.failed = Some(text);
    }
    fn step(&mut self, step: Step, _: &str) {
        self.step = step;
    }
    // END_PRACTICE
}
unsafe fn actual_discovery(game: &mut Game, p1: usize, cpu_entry: usize) {
    // MISSING
    // IDENTITY
    game.continued += 1;
}
unsafe fn actual_reload(game: &mut Game, p1: usize, cpu: usize, cpu_entry: usize) {
    match game.step {
        // RELOAD
        _ => (),
    }
}
fn actual_choose_request(game: &mut Game, kind: u32, request: Request) {
    // CHOOSE
}
fn actual_choose_gate(status: u32, admitted: &mut bool) {
    let (p1, cpu) = (11, 22);
    let mut ids = [0u32; 16];
    ids[15] = 0x7f;
    let linked = LINKED.load(Ordering::Relaxed);
    // RELEASE_CHECK
    // ADMISSION_GATE
    *admitted = true;
}

#[test]
fn choose_recovers_an_unlinked_grounded_cpu_teeter_only() {
    for (p1_status, cpu_status, blocker, expected) in [
        (0, 0, None, true),
        (0, 0x7f, None, true),
        (0x7f, 0x7f, None, false),
        (0, 0x7e, None, false),
        (0, 0x41, None, false),
        (0, 0x7f, Some(&LINKED), false),
        (0, 0x7f, Some(&P1_AIR), false),
        (0, 0x7f, Some(&CPU_AIR), false),
        (0, 0x7f, Some(&P1_STOP), false),
        (0, 0x7f, Some(&CPU_STOP), false),
    ] {
        reset();
        CPU_STATUS.store(cpu_status, Ordering::Relaxed);
        if let Some(blocker) = blocker {
            blocker.store(true, Ordering::Relaxed);
        }
        let mut admitted = false;
        actual_choose_gate(p1_status, &mut admitted);
        assert_eq!(admitted, expected, "P1={p1_status:#x}, CPU={cpu_status:#x}, blocked={}", blocker.is_some());
    }
}

unsafe fn actual_restore(game: &mut Game, released: bool, cpu_entry: usize) {
    let (p1, cpu) = (11, 22);
    let status = if released { 0 } else { 5 };
    let mut ids = [0u32; 16];
    ids[15] = 127; // Exact pinned native OTTOTTO_WAIT; runtime resolves its symbol.
    let linked = LINKED.load(Ordering::Relaxed);
    let released = released && CPU_STATUS.load(Ordering::Relaxed) == 0;
    match game.step {
        // RESTORE
        _ => (),
    }
}
unsafe fn actual_covered_restore(game: &mut Game, covered: bool, released: bool, cpu_entry: usize) {
    let (p1, cpu) = (11, 22);
    let status = if released { 0 } else { 5 };
    let mut ids = [0u32; 16];
    ids[15] = 127;
    let linked = LINKED.load(Ordering::Relaxed);
    let released = released && CPU_STATUS.load(Ordering::Relaxed) == 0;
    // COVERED
    match game.step {
        // RESTORE
        _ => (),
    }
}
unsafe fn actual_stop_frame(game: &mut Game, config: Option<()>, covered: bool, released: bool, cpu_entry: usize) {
    let (p1, cpu) = (11, 22);
    let status = if released { 0 } else { 5 };
    let mut ids = [0u32; 16];
    ids[15] = 127;
    let linked = LINKED.load(Ordering::Relaxed);
    let released = released && CPU_STATUS.load(Ordering::Relaxed) == 0;
    let restart = game.config != config;
    // RESTART
    // OWNERSHIP
    // COVERED
    match game.step {
        // RELEASE_ARMS
        // RESTORE
        _ => (),
    }
}
fn reset() {
    OWN_CPU.store(22, Ordering::Relaxed);
    CPU_ENTRY.store(99, Ordering::Relaxed);
    ACCESSOR.store(0, Ordering::Relaxed);
    BODY.store(0, Ordering::Relaxed);
    READY.store(false, Ordering::Relaxed);
    REVOKED.store(0, Ordering::Relaxed);
    TEMP_KIND.store(u32::MAX, Ordering::Relaxed);
    ACCEPT.store(false, Ordering::Relaxed);
    REQUESTED.store(u32::MAX, Ordering::Relaxed);
    CANCELLED.store(0, Ordering::Relaxed);
    CPU_STATUS.store(0, Ordering::Relaxed);
    for value in [&P1_AIR, &CPU_AIR, &P1_STOP, &CPU_STOP, &LINKED] {
        value.store(false, Ordering::Relaxed);
    }
    REQUEST_COUNT.store(0, Ordering::Relaxed);
    DRIVEN.store(0, Ordering::Relaxed);
    GENERATION.store(5, Ordering::Relaxed);
    MENU_OPEN.store(false, Ordering::Relaxed);
}
#[test]
fn stop_during_a_hold_under_the_menu_gives_up_the_cpu_and_releases_after_the_close() {
    reset();
    let mut body = [0u32, 0, 0, 66];
    BODY.store(body.as_ptr() as usize, Ordering::Relaxed);
    let mut g = Game::at(Step::Held);
    g.original = Some(34);
    MENU_OPEN.store(true, Ordering::Relaxed);
    LINKED.store(true, Ordering::Relaxed);
    CPU_STOP.store(true, Ordering::Relaxed);
    READY.store(false, Ordering::Relaxed);
    ACCEPT.store(true, Ordering::Relaxed);
    unsafe { actual_stop_frame(&mut g, None, true, false, 1); }
    assert_eq!(g.step, Step::Release);
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 0);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 1);
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 0);
    assert_eq!(g.config, None);
    for _ in 0..2 {
        unsafe { actual_stop_frame(&mut g, None, true, false, 1); }
    }
    assert_eq!(g.step, Step::Release);
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 0);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 1);
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 0);

    MENU_OPEN.store(false, Ordering::Relaxed);
    CPU_STOP.store(false, Ordering::Relaxed);
    unsafe { actual_stop_frame(&mut g, None, true, false, 1); }
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 22);
    assert_eq!(g.step, Step::Release);
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 0);
    unsafe { actual_stop_frame(&mut g, None, false, false, 1); }
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 22);
    assert_eq!(g.step, Step::Release);

    LINKED.store(false, Ordering::Relaxed);
    READY.store(true, Ordering::Relaxed);
    unsafe { actual_stop_frame(&mut g, None, false, true, 1); }
    assert_eq!(g.step, Step::Restore);
    unsafe { actual_stop_frame(&mut g, None, false, true, 1); }
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 0);
    assert_eq!(REQUESTED.load(Ordering::Relaxed), 34);
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 1);
    body[3] = 34;
    std::hint::black_box(&body);
    unsafe { actual_stop_frame(&mut g, None, false, true, 1); }
    assert_eq!(g.step, Step::Off);
    assert_eq!(TEMP_KIND.load(Ordering::Relaxed), u32::MAX);
}
#[test]
fn stop_during_a_hold_without_the_menu_keeps_the_cpu_neutral_until_release() {
    reset();
    let mut g = Game::at(Step::Held);
    LINKED.store(true, Ordering::Relaxed);
    MENU_OPEN.store(false, Ordering::Relaxed);
    unsafe { actual_stop_frame(&mut g, None, false, false, 1); }
    assert_eq!(g.step, Step::Release);
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 22);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 1);
}
#[test]
fn a_running_game_keeps_its_cpu_under_the_menu() {
    reset();
    let mut g = Game::at(Step::Held);
    MENU_OPEN.store(true, Ordering::Relaxed);
    unsafe { actual_stop_frame(&mut g, Some(()), true, false, 1); }
    assert_eq!(g.step, Step::Held);
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 22);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 0);
}
#[test]
fn stop_under_the_menu_gives_up_the_cpu_and_asks_for_the_borrowed_fighter_at_the_close() {
    reset();
    let mut body = [0u32, 0, 0, 66];
    BODY.store(body.as_ptr() as usize, Ordering::Relaxed);
    let mut game = Game::at(Step::Restore);
    game.original = Some(34);
    CPU_STOP.store(true, Ordering::Relaxed);
    READY.store(false, Ordering::Relaxed);
    ACCEPT.store(true, Ordering::Relaxed);
    for _ in 0..3 {
        unsafe { actual_covered_restore(&mut game, true, true, 1); }
    }
    assert_eq!(DRIVEN.load(Ordering::Relaxed), 0);
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 0);
    assert_eq!(REVOKED.load(Ordering::Relaxed), 3);
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 0);
    assert_eq!(game.step, Step::Restore);
    assert!(!game.restore_requested);
    assert_eq!(game.failed, None);

    CPU_STOP.store(false, Ordering::Relaxed);
    READY.store(true, Ordering::Relaxed);
    unsafe { actual_covered_restore(&mut game, false, true, 1); }
    assert_eq!(REQUESTED.load(Ordering::Relaxed), 34);
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 1);
    assert!(game.restore_requested);
    assert_eq!(TEMP_KIND.load(Ordering::Relaxed), 34);
    unsafe { actual_covered_restore(&mut game, false, true, 1); }
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 1);
    body[3] = 34;
    std::hint::black_box(&body);
    unsafe { actual_covered_restore(&mut game, false, true, 1); }
    assert_eq!(game.step, Step::Off);
    assert_eq!(TEMP_KIND.load(Ordering::Relaxed), u32::MAX);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 1);
}
#[test]
fn stop_under_the_menu_ends_a_drill_that_borrowed_no_fighter_at_once() {
    reset();
    let mut game = Game::at(Step::Restore);
    game.original = None;
    CPU_STOP.store(true, Ordering::Relaxed);
    READY.store(false, Ordering::Relaxed);
    unsafe { actual_covered_restore(&mut game, true, true, 1); }
    assert_eq!(game.step, Step::Off);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 1);
    assert_eq!(DRIVEN.load(Ordering::Relaxed), 0);
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 0);
}
#[test]
fn setup_steps_stay_neutral_under_the_menu() {
    for step in [Step::Choose, Step::Reload, Step::Place, Step::Entering] {
        reset();
        let mut game = Game::at(step);
        unsafe { actual_covered_restore(&mut game, true, true, 1); }
        assert_eq!(DRIVEN.load(Ordering::Relaxed), 1);
        assert_eq!(game.step, step);
        assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 0);
        assert_eq!(OWN_CPU.load(Ordering::Relaxed), 22);
    }
}
#[test]
fn reload_missing_cpu_waits_and_revokes() {
    reset();
    let mut g = Game::at(Step::Reload);
    unsafe {
        actual_discovery(&mut g, 11, 1);
    }
    assert_eq!(g.failed, None);
    assert_eq!(g.step, Step::Reload);
    assert_eq!(g.continued, 0);
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 0);
    assert_eq!(REVOKED.load(Ordering::Relaxed), 1);
}
#[test]
fn protected_phases_missing_cpu_fail_closed() {
    for step in [Step::Entering, Step::Held, Step::Result] {
        reset();
        let mut g = Game::at(step);
        unsafe {
            actual_discovery(&mut g, 11, 1);
        }
        assert_eq!(g.failed, Some("Practice CPU changed · no grade"));
        assert_eq!(g.continued, 0);
        assert_eq!(OWN_CPU.load(Ordering::Relaxed), 0);
    }
}
#[test]
fn protected_phases_replacement_fails_identity() {
    for step in [Step::Entering, Step::Held, Step::Result] {
        reset();
        ACCESSOR.store(33, Ordering::Relaxed);
        let mut g = Game::at(step);
        unsafe {
            actual_discovery(&mut g, 11, 1);
        }
        assert_eq!(g.failed, Some("Fighter changed · no grade"));
    }
}
#[test]
fn target_present_loader_busy_cannot_place_or_grant() {
    reset();
    OWN_CPU.store(0, Ordering::Relaxed);
    let body = [0u32, 0, 0, 6];
    BODY.store(body.as_ptr() as usize, Ordering::Relaxed);
    let mut g = Game::at(Step::Reload);
    unsafe {
        actual_reload(&mut g, 111, 222, 1);
    }
    assert_eq!(g.step, Step::Reload);
    assert_eq!((g.p1, g.cpu), (11, 22));
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 0);
    assert_eq!(CPU_ENTRY.load(Ordering::Relaxed), 99);
}
#[test]
fn absent_wrong_unspecified_target_cannot_place() {
    for kind in [None, Some(7)] {
        reset();
        READY.store(true, Ordering::Relaxed);
        let body = [0u32, 0, 0, kind.unwrap_or(0)];
        BODY.store(
            if kind.is_some() {
                body.as_ptr() as usize
            } else {
                0
            },
            Ordering::Relaxed,
        );
        let mut g = Game::at(Step::Reload);
        unsafe {
            actual_reload(&mut g, 111, 222, 1);
        }
        assert_eq!(g.step, Step::Reload);
    }
    reset();
    READY.store(true, Ordering::Relaxed);
    let body = [0u32, 0, 0, 6];
    BODY.store(body.as_ptr() as usize, Ordering::Relaxed);
    let mut g = Game::at(Step::Reload);
    g.requested = None;
    unsafe {
        actual_reload(&mut g, 111, 222, 1);
    }
    assert_eq!(g.step, Step::Reload);
}
#[test]
fn missing_wrong_busy_then_ready_continuation() {
    reset();
    let mut g = Game::at(Step::Reload);
    unsafe {
        actual_discovery(&mut g, 111, 1);
    }
    assert_eq!(g.failed, None);
    ACCESSOR.store(222, Ordering::Relaxed);
    let mut body = [0u32, 0, 0, 7];
    BODY.store(body.as_ptr() as usize, Ordering::Relaxed);
    READY.store(false, Ordering::Relaxed);
    unsafe {
        actual_reload(&mut g, 111, 222, 1);
    }
    assert_eq!(g.step, Step::Reload);
    assert_eq!(g.failed, None);
    body[3] = 6;
    std::hint::black_box(&body);
    READY.store(false, Ordering::Relaxed);
    unsafe {
        actual_reload(&mut g, 111, 222, 1);
    }
    assert_eq!(g.step, Step::Reload);
    READY.store(true, Ordering::Relaxed);
    unsafe {
        actual_reload(&mut g, 111, 222, 1);
    }
    assert_eq!(g.step, Step::Place);
    assert_eq!((g.p1, g.cpu), (111, 222));
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 222);
    assert_eq!(CPU_ENTRY.load(Ordering::Relaxed), 1);
}

#[test]
fn refused_request_is_an_explicit_failure_not_an_infinite_reload() {
    for target in [6, 31, 0xbeef] {
        reset();
        READY.store(true, Ordering::Relaxed);
        let mut g = Game::at(Step::Reload);
        actual_choose_request(
            &mut g,
            4,
            Request {
                attacker_kind: target,
            },
        );
        assert_eq!(REQUESTED.load(Ordering::Relaxed), target);
        assert_eq!(g.failed, Some("Attacker unavailable"));
    }
}

#[test]
fn accepted_request_waits_and_busy_loader_does_not_receive_request() {
    reset();
    READY.store(true, Ordering::Relaxed);
    ACCEPT.store(true, Ordering::Relaxed);
    let mut g = Game::at(Step::Reload);
    actual_choose_request(&mut g, 4, Request { attacker_kind: 31 });
    assert_eq!(REQUESTED.load(Ordering::Relaxed), 31);
    assert_eq!(g.failed, None);
    assert_eq!(g.step, Step::Reload);
    reset();
    actual_choose_request(&mut g, 4, Request { attacker_kind: 31 });
    assert_eq!(REQUESTED.load(Ordering::Relaxed), u32::MAX);
    assert_eq!(g.step, Step::Choose);
}

#[test]
fn completed_reload_with_wrong_real_body_fails_without_ownership_grant() {
    for actual in [4, 31, 0xbeef] {
        reset();
        OWN_CPU.store(0, Ordering::Relaxed);
        READY.store(true, Ordering::Relaxed);
        let body = [0u32, 0, 0, actual];
        BODY.store(body.as_ptr() as usize, Ordering::Relaxed);
        let mut g = Game::at(Step::Reload);
        unsafe {
            actual_reload(&mut g, 111, 222, 1);
        }
        assert_eq!(g.failed, Some("Attacker unavailable"));
        assert_ne!(g.step, Step::Place);
        assert_eq!(OWN_CPU.load(Ordering::Relaxed), 0);
        assert_eq!(CPU_ENTRY.load(Ordering::Relaxed), 99);
    }
}

#[test]
fn restore_waits_through_replacement_then_real_cleanup_clears_temporary_kind() {
    reset();
    let mut g = Game::at(Step::Restore);
    unsafe {
        actual_discovery(&mut g, 11, 1);
    }
    assert_eq!(g.failed, None);
    assert_eq!(g.continued, 0);
    unsafe {
        actual_restore(&mut g, true, 1);
    }
    assert_eq!(g.step, Step::Restore);
    let mut body = [0u32, 0, 0, 6];
    BODY.store(body.as_ptr() as usize, Ordering::Relaxed);
    READY.store(true, Ordering::Relaxed);
    ACCEPT.store(true, Ordering::Relaxed);
    unsafe {
        actual_restore(&mut g, true, 1);
    }
    assert!(g.restore_requested);
    assert_eq!(REQUESTED.load(Ordering::Relaxed), 4);
    assert_eq!(TEMP_KIND.load(Ordering::Relaxed), 4);
    unsafe {
        actual_discovery(&mut g, 11, 1);
    }
    assert_eq!(g.failed, None);
    assert_eq!(TEMP_KIND.load(Ordering::Relaxed), 4);
    body[3] = 4;
    std::hint::black_box(&body);
    unsafe {
        actual_restore(&mut g, true, 1);
    }
    assert_eq!(g.step, Step::Off);
    assert_eq!(TEMP_KIND.load(Ordering::Relaxed), u32::MAX);
    assert_eq!(OWN_CPU.load(Ordering::Relaxed), 0);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 1);
}

#[test]
fn restore_grounded_teeter_releases_control_and_restores_original_once() {
    reset();
    let mut body = [0u32, 0, 0, 66]; // Observed Ridley, original34.
    BODY.store(body.as_ptr() as usize, Ordering::Relaxed);
    CPU_STATUS.store(127, Ordering::Relaxed);
    READY.store(true, Ordering::Relaxed);
    ACCEPT.store(true, Ordering::Relaxed);
    let mut g = Game::at(Step::Restore);
    g.original = Some(34);
    unsafe {
        actual_restore(&mut g, true, 1);
    }
    assert_eq!(
        OWN_CPU.load(Ordering::Relaxed),
        0,
        "Off must relinquish neutral practice control"
    );
    assert_eq!(
        REQUESTED.load(Ordering::Relaxed),
        34,
        "grounded teeter is a native resting state"
    );
    assert!(g.restore_requested);
    unsafe {
        actual_restore(&mut g, true, 1);
    }
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 1);
    body[3] = 34;
    std::hint::black_box(&body);
    CPU_STATUS.store(0, Ordering::Relaxed);
    unsafe {
        actual_restore(&mut g, true, 1);
    }
    assert_eq!(g.step, Step::Off);
    assert_eq!(TEMP_KIND.load(Ordering::Relaxed), u32::MAX);
    assert_eq!(CANCELLED.load(Ordering::Relaxed), 1);
}

#[test]
fn restore_teeter_keeps_release_and_loader_guards() {
    for blocked in 0..8 {
        reset();
        let body = [0u32, 0, 0, 66];
        BODY.store(body.as_ptr() as usize, Ordering::Relaxed);
        CPU_STATUS.store(127, Ordering::Relaxed);
        READY.store(true, Ordering::Relaxed);
        ACCEPT.store(true, Ordering::Relaxed);
        match blocked {
            0 => P1_AIR.store(true, Ordering::Relaxed),
            1 => CPU_AIR.store(true, Ordering::Relaxed),
            2 => P1_STOP.store(true, Ordering::Relaxed),
            3 => CPU_STOP.store(true, Ordering::Relaxed),
            4 => LINKED.store(true, Ordering::Relaxed),
            5 => READY.store(false, Ordering::Relaxed),
            6 => CPU_STATUS.store(119, Ordering::Relaxed), // Actual CLIFF_WAIT is not permitted.
            _ => (),
        }
        let mut g = Game::at(Step::Restore);
        unsafe {
            actual_restore(&mut g, blocked != 7, 1);
        }
        assert_eq!(
            OWN_CPU.load(Ordering::Relaxed),
            0,
            "ownership must release even while waiting"
        );
        assert_eq!(
            REQUEST_COUNT.load(Ordering::Relaxed),
            0,
            "blocked case {blocked}"
        );
        assert_eq!(g.step, Step::Restore);
        assert_eq!(g.failed, None);
    }
}
