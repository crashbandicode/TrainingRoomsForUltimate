#![allow(dead_code)]
use std::{
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
};
mod model {
    pub mod hash40 { // HASH40
    }
    pub mod cpu {
        pub mod hash { // HASH
        }
        pub mod ids { // CONSTANTS
        }
    }
    pub mod input_delay { // MAPPED
    }
    pub mod mashout {
        pub mod moves { // MOVE_TYPES
        }
        pub mod grade {
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub struct Bounds {
                pub fastest: u32,
                pub no_input: Option<u32>,
            }
        }
        pub mod native { // NATIVE
        }
        pub mod entry { // ENTRY
        }
    }
}
use model::{
    cpu::ids::C,
    input_delay::MappedInputs,
    mashout::{
        entry,
        moves::{Plan, Scenario, State},
        native::{Clock, Seed},
    },
};
mod runtime {
    pub mod game {
        pub fn text_base() -> usize {
            super::super::threshold_getter as *const () as usize - 0x6ba1e0
        }
        pub fn granted(_: usize) -> bool {
            true
        }
    }
}
extern "C" fn threshold_getter(_: usize) -> f32 {
    0.37
}
// Allocation-backed reader fixture; real exact-vptr/slot policy has its own interposer tests.
mod interposer {
    pub unsafe fn reader_table(table: usize) -> bool {
        *((table + 0x78) as *const usize) == super::runtime::game::text_base() + 0x6b9150
    }
}
mod reader { // READER
}
static READY: AtomicBool = AtomicBool::new(true);
static SLOTS_READY: AtomicBool = AtomicBool::new(true);
static P1: AtomicUsize = AtomicUsize::new(0);
static THRESHOLD: AtomicU32 = AtomicU32::new(0);
static ARMED: AtomicU64 = AtomicU64::new(0);
static GENERATION: AtomicU64 = AtomicU64::new(5);
static CPU_STATUS: AtomicU32 = AtomicU32::new(0);
static BODY: AtomicUsize = AtomicUsize::new(0);
static FAULT: AtomicU64 = AtomicU64::new(0);
static EPOCH: AtomicU64 = AtomicU64::new(0);
static QUEUE_FULL: AtomicBool = AtomicBool::new(false);
// NATIVE_SEED_TYPES
thread_local! {
    static QUEUE: RefCell<Vec<NativeSeed>> = const { RefCell::new(Vec::new()) };
    static BEGINS: RefCell<Vec<(worker::Key, Seed, u64, SeedOrigin)>> = const { RefCell::new(Vec::new()) };
    static DRIVEN: RefCell<Vec<MappedInputs>> = const { RefCell::new(Vec::new()) };
}
struct Mailbox;
static SEEDS: Mailbox = Mailbox;
impl Mailbox {
    fn post(&self, sample: NativeSeed) -> bool {
        if QUEUE_FULL.load(Ordering::Relaxed) { return false; }
        QUEUE.with_borrow_mut(|q| q.push(sample));
        true
    }
    fn drain(&self, mut f: impl FnMut(NativeSeed)) {
        let q = QUEUE.with_borrow_mut(std::mem::take);
        for s in q {
            f(s);
        }
    }
}
struct Admission;
impl Admission {
    fn with<T>(&self, f: impl FnOnce() -> T) -> Option<T> {
        Some(f())
    }
}
fn admission() -> Admission {
    Admission
}
fn fault(generation: u64) {
    FAULT.store(generation, Ordering::Relaxed);
}
fn revoke() {
    ARMED.store(0, Ordering::Relaxed);
    drive(MappedInputs::NEUTRAL);
}
fn drive(v: MappedInputs) {
    DRIVEN.with_borrow_mut(|d| d.push(v));
}
mod scene {
    pub fn current() -> Option<()> {
        Some(())
    }
}
mod natives {
    pub fn module_accessor(_: i32) -> Option<usize> {
        Some(super::P1.load(super::Ordering::Relaxed))
    }
    pub fn status_kind(_: usize) -> i32 {
        super::CPU_STATUS.load(super::Ordering::Relaxed) as i32
    }
    pub fn situation_kind(_: usize) -> i32 {
        0
    }
    pub fn fighter_entry(_: i32) -> usize {
        1
    }
    pub fn entry_active_body(_: usize) -> usize {
        super::BODY.load(super::Ordering::Relaxed)
    }
    pub fn damage(_: usize, _: usize) -> f32 {
        23.5
    }
}
mod state_natives {
    pub fn status_request(_: usize, _: i32) {
        panic!("unexpected air-special transition")
    }
}
mod worker {
    use super::{Seed, SeedOrigin};
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Key(pub u16, pub u32, pub u32, pub f32, pub Seed, pub SeedOrigin);
    impl Key {
        pub fn new(id: u16, a: u32, v: u32, d: f32, s: Seed, origin: SeedOrigin) -> Self {
            Self(id, a, v, d, s, origin)
        }
    }
    pub enum Event {
        Begin(u64, u64, Key, Seed, SeedOrigin),
        Cancel(u64),
        NativeEnd(u64, u64),
    }
}
fn post(e: worker::Event, _: u64) {
    if let worker::Event::Begin(_, epoch, k, s, origin) = e {
        BEGINS.with_borrow_mut(|b| b.push((k, s, epoch, origin)));
    }
}
fn state_id(c: C) -> Option<i32> {
    Some(match c {
        C::ClatterCaptureYoshi => 243,
        C::ClatterYoshiEgg => 244,
        C::ClatterCaptureWait => 66,
        C::ClatterBury => 75,
        C::SituationAir => 1,
        _ => -1000 - c as i32,
    })
}
fn holding(request: Option<Scenario>, status: i32) -> bool {
    request.is_some_and(|r| entry::holding(r.state, |c| state_id(c) == Some(status)))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Entering,
    Held,
    Result,
    Failed,
    Release,
    Choose,
    Restore,
}
struct Game {
    generation: u64,
    epoch: u64,
    step: Step,
    requested: Option<Scenario>,
    followed: bool,
    saw_catch: bool,
    clock: Option<Clock>,
    config: Option<()>,
    restore_requested: bool,
    failed: Option<&'static str>,
}
impl Game {
    fn new(plan: Plan, state: State) -> Self {
        Self {
            generation: 5,
            epoch: 0,
            step: Step::Entering,
            requested: Some(Scenario {
                id: 117,
                state,
                attacker_kind: 5,
                attacker_name: "Yoshi",
                name: "Egg",
                plan,
                grab: 0,
                evidence: "E01",
            }),
            followed: false,
            saw_catch: false,
            clock: None,
            config: Some(()),
            restore_requested: false,
            failed: None,
        }
    }
    fn fail(&mut self, s: &'static str) {
        self.failed = Some(s);
        self.step = Step::Failed;
        self.clock = None;
        revoke();
    }
    fn step(&mut self, s: Step, _: &str) {
        self.step = s;
    }
}
const UNAVAILABLE_NOTICE: &str = "unavailable";
// ARM
#[derive(Clone, Copy)]
// TOKEN_TYPE
// TOKEN
// AFTER
// THROW_AFTER
// PUBLISH_SEED
fn ids() -> [i32; 15] {
    [0, 50, 65, 66, 67, 68, 69, 0, 51, 52, 53, 70, 71, 72, 73]
}
unsafe fn startup(game: &mut Game, p1: usize) {
    let request = game.requested.unwrap(); // START
}
unsafe fn update(game: &mut Game, p1: usize, status: i32, cpu: usize, fighter: usize) {
    let ids = ids();
    let cpu_entry = 1;
    // LINKS
    // SEEDS
    // ENDS
    match game.step {
        // ENTERING
        _ => (),
    }
}
fn cancel(game: &mut Game, status: i32) {
    let ids = ids();
    let restart = true;
    let config = None;
    // LINKS
    // RESTART
}
struct NativeMemory {
    accessor: [usize; 10],
    control: [u64; 40],
    table: [usize; 32],
    record: [u64; 4],
    body: [u32; 4],
}
impl NativeMemory {
    fn new() -> Box<Self> {
        let mut m = Box::new(Self {
            accessor: [0; 10],
            control: [0; 40],
            table: [0; 32],
            record: [0; 4],
            body: [0, 0, 0, 5],
        });
        m.accessor[9] = m.control.as_ptr() as usize;
        m.control[0] = m.table.as_ptr() as u64;
        m.table[0x78 / 8] = runtime::game::text_base() + 0x6b9150;
        m.control[0x10 / 8] = 1;
        m.control[0x18 / 8] = m.record.as_ptr() as u64;
        P1.store(m.p1(), Ordering::Relaxed);
        BODY.store(m.body.as_ptr() as usize, Ordering::Relaxed);
        ARMED.store(0, Ordering::Relaxed);
        EPOCH.store(0, Ordering::Relaxed);
        FAULT.store(0, Ordering::Relaxed);
        QUEUE_FULL.store(false, Ordering::Relaxed);
        READY.store(true, Ordering::Relaxed);
        SLOTS_READY.store(true, Ordering::Relaxed);
        CPU_STATUS.store(481, Ordering::Relaxed);
        QUEUE.with_borrow_mut(Vec::clear);
        END_QUEUE.with_borrow_mut(Vec::clear);
        BEGINS.with_borrow_mut(Vec::clear);
        DRIVEN.with_borrow_mut(Vec::clear);
        m
    }
    fn p1(&self) -> usize {
        self.accessor.as_ptr() as usize
    }
    fn write<T: Copy>(&mut self, offset: usize, v: T) {
        unsafe {
            (self.control.as_mut_ptr().cast::<u8>().add(offset) as *mut T).write(v);
        }
    }
    fn seed(&mut self, remaining: f32, manual: f32, history: u8) {
        let p = self.record.as_mut_ptr().cast::<u8>();
        unsafe {
            for (offset, v) in [(0, remaining), (4, 1.0), (8, manual), (12, 1.25)] {
                (p.add(offset) as *mut f32).write(v);
            }
            for (offset, v) in [
                (0x10, -1i8),
                (0x11, 1),
                (0x12, -1),
                (0x13, 2),
                (0x14, 1),
                (0x15, 2),
                (0x16, 3),
                (0x17, 4),
                (0x18, 3),
            ] {
                p.add(offset).cast::<i8>().write(v);
            }
        }
        self.write(0x30, -0.9f32);
        self.write(0x34, 0.2f32);
        self.write(0x9c, 0x20u32);
        self.write(0xa0, 0x8u32);
        self.write(0xa4, 0x200u32);
        self.write(0xa8, 0x2u32);
        self.write(0x102, history);
    }
    unsafe fn start(&self) {
        if let Some(t) = seed_token(self.p1()) {
            seed_after(self.p1(), t, -1, 0, false, true);
        }
    }
    unsafe fn throw_start(&self) {
        throw_seed_after(self.control.as_ptr() as usize);
    }
    unsafe fn update(&self, g: &mut Game, status: i32) {
        update(g, self.p1(), status, 2, self.body.as_ptr() as usize);
    }
}
#[test]
fn tongue_is_native_unscored_and_egg_uses_actual_history() {
    for history in [0, 1] {
        unsafe {
            let mut m = NativeMemory::new();
            let mut g = Game::new(Plan::Egg, State::Egg);
            startup(&mut g, m.p1());
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
            m.seed(90.0, 8.0, history);
            let before = (m.control, m.record);
            m.start();
            m.update(&mut g, 243);
            assert_eq!(g.step, Step::Entering);
            assert!(g.clock.is_none());
            assert!(BEGINS.with_borrow(|b| b.is_empty()));
            assert_eq!(
                (m.control, m.record),
                before,
                "arming must preserve native tongue input and timer"
            );
            assert_eq!(ARMED.load(Ordering::Relaxed), 5);
            m.seed(142.3, 7.0, history);
            let before = (m.control, m.record);
            let actual = reader::sample(m.p1(), 0.37).unwrap().seed();
            m.start();
            m.update(&mut g, 244);
            assert_eq!(g.step, Step::Held);
            assert_eq!(g.clock.unwrap().last(), actual.record);
            let begins = BEGINS.with_borrow(Clone::clone);
            assert_eq!(begins.len(), 1);
            assert_eq!(begins[0].1, actual);
            assert_eq!(actual.held, if history == 0 { 1 } else { 0x20 });
            assert_eq!(begins[0].0 .3, 23.5);
            assert_eq!(
                (m.control, m.record),
                before,
                "native initialized latches/history must survive"
            );
            assert_eq!(entry::controls(Plan::Egg, true), MappedInputs::NEUTRAL);
            assert_eq!(
                entry::start(g.requested.unwrap()),
                Some("FIGHTER_STATUS_KIND_SPECIAL_N")
            );
        }
    }
}
#[test]
fn missed_tongue_does_not_arm_or_score() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = Game::new(Plan::Egg, State::Egg);
        startup(&mut g, m.p1());
        m.seed(100.0, 7.0, 1);
        m.start();
        m.update(&mut g, 244);
        m.start();
        m.update(&mut g, 244);
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        assert_eq!(g.step, Step::Entering);
        assert!(g.clock.is_none());
        assert!(BEGINS.with_borrow(|b| b.is_empty()));
    }
}
#[test]
fn later_start_still_fails() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = Game::new(Plan::Egg, State::Egg);
        startup(&mut g, m.p1());
        m.update(&mut g, 243);
        m.seed(117.0, 7.0, 0);
        m.start();
        m.update(&mut g, 244);
        assert_eq!(g.step, Step::Held);
        m.start();
        m.update(&mut g, 244);
        assert_eq!(g.failed, Some("Capture restarted · no grade"));
        assert!(g.clock.is_none());
        assert_eq!(BEGINS.with_borrow(Vec::len), 1);
    }
}
#[test]
fn ordinary_grab_and_downthrow_preserve_entry_semantics() {
    unsafe {
        for status in [66, 73] {
            let mut m = NativeMemory::new();
            let mut g = Game::new(Plan::Grab, State::Grab);
            startup(&mut g, m.p1());
            assert_eq!(ARMED.load(Ordering::Relaxed), 5);
            CPU_STATUS.store(52, Ordering::Relaxed);
            m.seed(60.0, 8.0, 1);
            m.start();
            m.update(&mut g, status);
            assert_eq!(g.step, Step::Held);
        }
        let mut m = NativeMemory::new();
        let mut g = Game::new(Plan::DownThrow, State::Bury);
        startup(&mut g, m.p1());
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        m.seed(90.0, 8.0, 1);
        m.start();
        m.update(&mut g, 66);
        assert!(g.clock.is_none());
        CPU_STATUS.store(52, Ordering::Relaxed);
        m.update(&mut g, 66);
        assert_eq!(ARMED.load(Ordering::Relaxed), 5);
        assert!(g.followed);
        m.seed(80.0, 7.0, 0);
        m.start();
        m.update(&mut g, 75);
        assert_eq!(g.step, Step::Held);
        assert_eq!(entry::controls(Plan::DownThrow, true).lstick_y, -120);
    }
}
#[test]
fn cancel_during_tongue_waits_for_release() {
    let _m = NativeMemory::new();
    let mut g = Game::new(Plan::Egg, State::Egg);
    cancel(&mut g, 243);
    assert_eq!(g.step, Step::Release);
    assert!(g.clock.is_none());
    assert_eq!(ARMED.load(Ordering::Relaxed), 0);
}

unsafe fn held_downthrow(m: &mut NativeMemory) -> Game {
    let mut g = Game::new(Plan::DownThrow, State::Bury);
    startup(&mut g, m.p1());
    CPU_STATUS.store(52, Ordering::Relaxed);
    m.update(&mut g, 66);
    assert_eq!(ARMED.load(Ordering::Relaxed), g.generation);
    m.seed(80.0, 7.0, 0);
    m.start();
    m.update(&mut g, 75);
    assert_eq!(g.step, Step::Held);
    assert_eq!(g.epoch, 1);
    g
}

#[test]
fn tagged_held_bury_downthrow_reseeds_from_actual_native_record_and_history() {
    unsafe {
        for history in [0, 1] {
            for remaining in [34.0, 59.0, 29.0] {
                let mut m = NativeMemory::new();
                let mut g = held_downthrow(&mut m);
                m.seed(remaining, 9.0, history);
                let actual = reader::sample(m.p1(), 0.37).unwrap().seed();
                let before = (m.control, m.record);
                m.throw_start();
                m.update(&mut g, 75);
                assert_eq!(g.step, Step::Held);
                assert_eq!(g.failed, None);
                assert_eq!(g.epoch, 2);
                assert_eq!(g.clock.unwrap().last(), actual.record);
                let begins = BEGINS.with_borrow(Clone::clone);
                assert_eq!(begins.len(), 2);
                assert_eq!(begins[1].1, actual);
                assert_eq!(begins[1].2, g.epoch);
                assert_eq!(begins[1].3, SeedOrigin::ThrowClatterStart);
                assert_eq!(begins[1].0 .5, SeedOrigin::ThrowClatterStart);
                assert_eq!((m.control, m.record), before, "observer must preserve native latches and input");
                assert_eq!(actual.held, if history == 0 { 1 } else { 0x20 });
            }
        }
    }
}

#[test]
fn export_held_and_tagged_result_restarts_still_fail() {
    unsafe {
        for tagged_result in [false, true] {
            let mut m = NativeMemory::new();
            let mut g = held_downthrow(&mut m);
            m.seed(34.0, 9.0, 1);
            if tagged_result {
                g.step = Step::Result;
                m.throw_start();
            } else {
                m.start();
            }
            m.update(&mut g, 75);
            assert_eq!(g.failed, Some("Capture restarted · no grade"));
            assert!(g.clock.is_none());
            assert_eq!(BEGINS.with_borrow(Vec::len), 1);
        }
    }
}

#[test]
fn identical_native_starts_get_distinct_epochs_and_wrong_control_is_ignored() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = held_downthrow(&mut m);
        m.seed(34.0, 9.0, 1);
        throw_seed_after(m.control.as_ptr() as usize + 8);
        assert!(QUEUE.with_borrow(Vec::is_empty));
        assert_eq!(EPOCH.load(Ordering::Relaxed), 1);
        for epoch in [2, 3] {
            m.throw_start();
            m.update(&mut g, 75);
            assert_eq!(g.step, Step::Held);
            assert_eq!(g.epoch, epoch);
        }
        let begins = BEGINS.with_borrow(Clone::clone);
        assert_eq!(begins.len(), 3);
        assert_eq!(begins[1].1, begins[2].1);
        assert_ne!(begins[1].2, begins[2].2);
    }
}

#[test]
fn native_seed_mailbox_overflow_marks_attempt_incomplete() {
    unsafe {
        let mut m = NativeMemory::new();
        let g = held_downthrow(&mut m);
        m.seed(34.0, 9.0, 1);
        QUEUE_FULL.store(true, Ordering::Relaxed);
        m.throw_start();
        assert_eq!(EPOCH.load(Ordering::Relaxed), 2);
        assert_eq!(FAULT.load(Ordering::Relaxed), g.generation);
        assert!(QUEUE.with_borrow(Vec::is_empty));
        assert_eq!(BEGINS.with_borrow(Vec::len), 1);
    }
}


// END_TYPE
// END_TOKEN
// END_AFTER
struct EndMailbox;
static ENDS: EndMailbox = EndMailbox;
thread_local! { static END_QUEUE: RefCell<Vec<NativeEnd>> = const { RefCell::new(Vec::new()) }; }
fn faulted(generation: u64, watermark: u64) -> bool { generation != 0 && watermark >= generation }
impl EndMailbox {
    fn drain(&self, mut f: impl FnMut(NativeEnd)) {
        for ended in END_QUEUE.with_borrow_mut(std::mem::take) { f(ended); }
    }
    fn post(&self, ended: NativeEnd) -> bool {
        if QUEUE_FULL.load(Ordering::Relaxed) { return false; }
        END_QUEUE.with_borrow_mut(|q| q.push(ended));
        true
    }
}
#[test]
fn native_end_requires_positive_pre_call_and_actual_post_zero() {
    unsafe {
        let mut m = NativeMemory::new();
        let g = held_downthrow(&mut m);
        END_QUEUE.with_borrow_mut(Vec::clear);
        let control = m.control.as_ptr() as usize;
        let token = end_token(control, 0).unwrap();
        end_after(control, token);
        assert!(END_QUEUE.with_borrow(Vec::is_empty));
        m.seed(0.0, 9.0, 1);
        assert!(end_token(control, 0).is_none(), "normal exhaustion is not an interruption");
        end_after(control, token);
        let ends = END_QUEUE.with_borrow(Clone::clone);
        assert_eq!(ends.len(), 1);
        assert_eq!((ends[0].generation, ends[0].epoch), (g.generation, g.epoch));
        assert_eq!(EPOCH.load(Ordering::Relaxed), g.epoch);
    }
}
#[test]
fn native_end_wrong_identity_epoch_index_and_overflow_fail_closed() {
    unsafe {
        let mut m = NativeMemory::new();
        let g = held_downthrow(&mut m);
        END_QUEUE.with_borrow_mut(Vec::clear);
        let control = m.control.as_ptr() as usize;
        assert!(end_token(control + 8, 0).is_none());
        assert!(end_token(control, 1).is_none());
        let token = end_token(control, 0).unwrap();
        m.seed(0.0, 9.0, 1);
        EPOCH.store(g.epoch + 1, Ordering::Relaxed);
        end_after(control, token);
        EPOCH.store(g.epoch, Ordering::Relaxed);
        ARMED.store(g.generation + 1, Ordering::Relaxed);
        end_after(control, token);
        assert!(END_QUEUE.with_borrow(Vec::is_empty));
        ARMED.store(g.generation, Ordering::Relaxed);
        QUEUE_FULL.store(true, Ordering::Relaxed);
        end_after(control, token);
        assert!(END_QUEUE.with_borrow(Vec::is_empty));
        assert_eq!(FAULT.load(Ordering::Relaxed), g.generation);
    }
}

#[test]
fn native_end_consumed_before_arithmetic_only_for_held_current_epoch() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = held_downthrow(&mut m);
        END_QUEUE.with_borrow_mut(Vec::clear);
        let token = end_token(m.control.as_ptr() as usize, 0).unwrap();
        m.seed(0.0, 9.0, 1);
        end_after(m.control.as_ptr() as usize, token);
        m.update(&mut g, 75);
        assert_eq!(g.step, Step::Result);
        assert!(g.clock.is_none());
        assert_eq!(g.failed, None);
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
    }
}

#[test]
fn escape_status_and_already_exhausted_clock_take_precedence_over_end() {
    unsafe {
        for status in [68, 69, 75] {
            let mut m = NativeMemory::new();
            let mut g = held_downthrow(&mut m);
            let token = end_token(m.control.as_ptr() as usize, 0).unwrap();
            let mut record = g.clock.unwrap().last();
            let input = reader::sample(m.p1(), 0.37).unwrap().input;
            let mut clock = g.clock.unwrap();
            while record.remaining > 0.0 {
                record.step(input);
                clock.observe(record, input, false).unwrap();
            }
            if status == 75 { g.clock = Some(clock); }
            END_QUEUE.with_borrow_mut(|q| q.push(token));
            m.update(&mut g, status);
            assert_eq!(g.step, Step::Held, "End must not replace a native escape");
            assert!(g.clock.is_some());
            assert_eq!(g.failed, None);
            if status == 75 { assert!(g.clock.unwrap().exhausted()); }
        }
    }
}
