use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
const TABLE: usize = 0x4f85de0;
const START_SLOT: usize = 0x4f85e58;
const END_SLOT: usize = 0x4f85ea8;
static ORIGINAL_START: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_END: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static LOST: AtomicBool = AtomicBool::new(false);
static WANTED: AtomicBool = AtomicBool::new(true);
/// The runtime reads the running game through `wanted()` (rc.15 practice menu); the fixture keeps its own flag.
fn wanted() -> bool {
    WANTED.load(Ordering::Acquire)
}
static READY: AtomicBool = AtomicBool::new(true);
static SLOTS_READY: AtomicBool = AtomicBool::new(false);
static ARMED: AtomicU64 = AtomicU64::new(5);
static START_DATA: AtomicUsize = AtomicUsize::new(0);
static END_DATA: AtomicUsize = AtomicUsize::new(0);
static RACE_ON_END: AtomicBool = AtomicBool::new(false);
static PRACTICE: AtomicBool = AtomicBool::new(true);
static OBSERVED: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_CALLS: AtomicUsize = AtomicUsize::new(0);
static END_OBSERVED: AtomicUsize = AtomicUsize::new(0);
unsafe fn slot(offset: usize) -> &'static AtomicUsize {
    if offset == END_SLOT && RACE_ON_END.swap(false, Ordering::Relaxed) {
        START_DATA.store(0xbeef, Ordering::Relaxed);
    }
    match offset {
        START_SLOT => &START_DATA,
        END_SLOT => &END_DATA,
        _ => panic!("unexpected slot"),
    }
}
mod game {
    pub fn text_base() -> usize {
        0x10000000
    }
}
mod scene {
    pub fn current() -> Option<()> {
        super::PRACTICE.load(super::Ordering::Acquire).then_some(())
    }
}
mod runtime {
    pub mod diag_log {
        pub fn diagnostic(_: std::fmt::Arguments<'_>) {}
    }
}
unsafe extern "C" fn start() {}
unsafe fn throw_seed_after(control: usize) {
    OBSERVED.store(control, Ordering::Relaxed);
}
unsafe fn end_token(control: usize, index: i32) -> Option<usize> {
    (control == 7 && index == 0).then_some(control)
}
unsafe fn end_after(control: usize, token: usize) {
    assert_eq!(control, token);
    END_OBSERVED.fetch_add(1, Ordering::Relaxed);
}
// TARGETS
// SERVICE
// READER_TABLE
// STARTED
// END
fn reset() {
    RACE_ON_END.store(false, Ordering::Relaxed);
    START_DATA.store(game::text_base() + 0x6b9150, Ordering::Relaxed);
    END_DATA.store(game::text_base() + 0x3f8230, Ordering::Relaxed);
    INSTALLED.store(false, Ordering::Relaxed);
    LOST.store(false, Ordering::Relaxed);
    WANTED.store(true, Ordering::Relaxed);
    READY.store(true, Ordering::Relaxed);
    SLOTS_READY.store(false, Ordering::Relaxed);
    PRACTICE.store(true, Ordering::Relaxed);
    ARMED.store(5, Ordering::Relaxed);
    OBSERVED.store(0, Ordering::Relaxed);
}
#[test]
fn enable_repeat_disable_restores_both_slots() {
    reset();
    service();
    assert!(SLOTS_READY.load(Ordering::Relaxed));
    unsafe {
        assert!(reader_table(game::text_base() + TABLE));
        assert!(!reader_table(game::text_base() + TABLE + 8));
    }
    service();
    assert_eq!(START_DATA.load(Ordering::Relaxed), targets()[0]);
    WANTED.store(false, Ordering::Relaxed);
    service();
    assert!(!SLOTS_READY.load(Ordering::Relaxed));
    assert_eq!(ARMED.load(Ordering::Relaxed), 0);
    assert_eq!(
        START_DATA.load(Ordering::Relaxed),
        game::text_base() + 0x6b9150
    );
    assert_eq!(
        END_DATA.load(Ordering::Relaxed),
        game::text_base() + 0x3f8230
    );
    WANTED.store(true, Ordering::Relaxed);
    service();
    assert!(SLOTS_READY.load(Ordering::Relaxed));
    PRACTICE.store(false, Ordering::Relaxed);
    service();
    assert!(!SLOTS_READY.load(Ordering::Relaxed));
}
#[test]
fn foreign_start_or_end_is_preserved_and_partial_install_rolls_back() {
    for offset in [START_SLOT, END_SLOT] {
        reset();
        unsafe { slot(offset) }.store(0xbeef, Ordering::Relaxed);
        service();
        assert!(!SLOTS_READY.load(Ordering::Relaxed));
        assert!(LOST.load(Ordering::Relaxed));
        assert_eq!(unsafe { slot(offset) }.load(Ordering::Relaxed), 0xbeef);
        if offset == END_SLOT {
            assert_eq!(
                START_DATA.load(Ordering::Relaxed),
                game::text_base() + 0x6b9150
            );
        }
    }
}
#[test]
fn foreign_takeover_revokes_and_preserves_foreign_pointer() {
    reset();
    service();
    END_DATA.store(0xbeef, Ordering::Relaxed);
    service();
    assert!(!SLOTS_READY.load(Ordering::Relaxed));
    assert_eq!(ARMED.load(Ordering::Relaxed), 0);
    service();
    assert_eq!(END_DATA.load(Ordering::Relaxed), 0xbeef);
    assert_eq!(
        START_DATA.load(Ordering::Relaxed),
        game::text_base() + 0x6b9150
    );
}
#[test]
fn guard_miss_never_installs() {
    reset();
    READY.store(false, Ordering::Relaxed);
    service();
    assert!(!SLOTS_READY.load(Ordering::Relaxed));
    assert_eq!(
        START_DATA.load(Ordering::Relaxed),
        game::text_base() + 0x6b9150
    );
}
#[test]
fn saved_caller_filter_and_native_end_return_are_exact() {
    reset();
    unsafe {
        started(7, game::text_base() + 0x639d40);
        assert_eq!(OBSERVED.load(Ordering::Relaxed), 0);
        started(7, game::text_base() + 0x639d44);
        assert_eq!(OBSERVED.load(Ordering::Relaxed), 7);
        extern "C" fn original(_: usize, _: i32) -> u64 {
            ORIGINAL_CALLS.fetch_add(1, Ordering::Relaxed);
            0xfedcba9876543210
        }
        ORIGINAL_END.store(original as *const () as usize, Ordering::Relaxed);
        ORIGINAL_CALLS.store(0, Ordering::Relaxed);
        END_OBSERVED.store(0, Ordering::Relaxed);
        assert_eq!(end(7, 0), 0xfedcba9876543210);
        assert_eq!(end(8, 0), 0xfedcba9876543210);
        assert_eq!(end(7, 1), 0xfedcba9876543210);
        assert_eq!(ORIGINAL_CALLS.load(Ordering::Relaxed), 3);
        assert_eq!(END_OBSERVED.load(Ordering::Relaxed), 1);
    }
}

#[test]
fn foreign_race_between_slot_installs_blocks_joint_admission() {
    reset();
    RACE_ON_END.store(true, Ordering::Relaxed);
    service();
    assert!(!SLOTS_READY.load(Ordering::Relaxed));
    assert!(!INSTALLED.load(Ordering::Relaxed));
    assert!(LOST.load(Ordering::Relaxed));
    assert_eq!(START_DATA.load(Ordering::Relaxed), 0xbeef);
    assert_eq!(END_DATA.load(Ordering::Relaxed), game::text_base() + 0x3f8230);
    assert_eq!(ORIGINAL_START.load(Ordering::Relaxed), game::text_base() + 0x6b9150);
    assert_eq!(ORIGINAL_END.load(Ordering::Relaxed), game::text_base() + 0x3f8230);
}
