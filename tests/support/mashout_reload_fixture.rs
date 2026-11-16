use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
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
    pub mod roster {
        pub mod character_roster {
            pub fn cpu_reload_ready() -> bool {
                crate::READY.load(crate::Ordering::Relaxed)
            }
            pub fn request_cpu_kind(kind: u32) -> bool {
                crate::REQUESTED.store(kind, crate::Ordering::Relaxed);
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
unsafe fn actual_restore(game: &mut Game, released: bool, cpu_entry: usize) {
    match game.step {
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
