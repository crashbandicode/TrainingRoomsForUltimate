#![allow(dead_code)]
use std::{
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
};
mod model {
    pub mod sites_pkg2 { pub const WORK_IS_FLAG_OFFSET: usize = 0x208dc80; pub const WORK_GET_INT_OFFSET: usize = 0x208dbc0; }
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
static ROLLER_LR: AtomicU32 = AtomicU32::new(1.0f32.to_bits());
static FREEZE_NO_BANG: AtomicU32 = AtomicU32::new(1);
static JUDGE_FIXED: AtomicBool = AtomicBool::new(false);
static JUDGE_KIND: AtomicU32 = AtomicU32::new(7);
static JUDGE_GETTER_GRANTED: AtomicBool = AtomicBool::new(true);
static JUDGE_FLAG_GRANTED: AtomicBool = AtomicBool::new(true);
static JUDGE_INT_READS: AtomicUsize = AtomicUsize::new(0);
static JUDGE_FLAG_READS: AtomicUsize = AtomicUsize::new(0);
static P1_AIR: AtomicBool = AtomicBool::new(false);
static CPU_AIR: AtomicBool = AtomicBool::new(false);
static P1_STOP: AtomicBool = AtomicBool::new(false);
// Existing tests stop at seed/end publication; new continuation probes opt into the actual Held arm.
static EXECUTE_HELD: AtomicBool = AtomicBool::new(false);
static CPU_STOP: AtomicBool = AtomicBool::new(false);
static AIR: AtomicBool = AtomicBool::new(false);
static BODY: AtomicUsize = AtomicUsize::new(0);
static FAULT: AtomicU64 = AtomicU64::new(0);
static EPOCH: AtomicU64 = AtomicU64::new(0);
static QUEUE_FULL: AtomicBool = AtomicBool::new(false);
// NATIVE_SEED_TYPES
thread_local! {
    static QUEUE: RefCell<Vec<NativeSeed>> = const { RefCell::new(Vec::new()) };
    static BEGINS: RefCell<Vec<(worker::Key, Seed, u64, SeedOrigin)>> = const { RefCell::new(Vec::new()) };
    static REQUESTS: RefCell<Vec<(usize,i32)>> = const { RefCell::new(Vec::new()) };
    static DRIVEN: RefCell<Vec<MappedInputs>> = const { RefCell::new(Vec::new()) };
}
struct Mailbox;
static SEEDS: Mailbox = Mailbox;
impl Mailbox {
    fn post(&self, sample: NativeSeed) -> bool {
        if QUEUE_FULL.load(Ordering::Relaxed) {
            return false;
        }
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
    pub fn is_flag(cpu: usize, id: i32) -> bool {
        assert_eq!(cpu, 2);
        assert_eq!(Some(id), super::state_id(super::C::JudgeFixed));
        super::JUDGE_FLAG_READS.fetch_add(1, super::Ordering::Relaxed);
        super::JUDGE_FIXED.load(super::Ordering::Relaxed)
    }
    pub fn get_int(cpu: usize, id: i32) -> i32 {
        if Some(id) == super::state_id(super::C::JudgeKind) {
            assert_eq!(cpu, 2);
            super::JUDGE_INT_READS.fetch_add(1, super::Ordering::Relaxed);
            // The real native shim defaults to0 when unavailable; this must
            // never be mistaken for a positively observed other Judge roll.
            return if super::JUDGE_GETTER_GRANTED.load(super::Ordering::Relaxed) {
                super::JUDGE_KIND.load(super::Ordering::Relaxed) as i32
            } else { 0 };
        }
        assert_eq!(id, 0x11000008); // Lucas work id, pinned01bf583/native table.
        super::FREEZE_NO_BANG.load(super::Ordering::Relaxed) as i32
    }
    pub fn module_accessor(_: i32) -> Option<usize> {
        Some(super::P1.load(super::Ordering::Relaxed))
    }
    pub fn status_kind(_: usize) -> i32 {
        super::CPU_STATUS.load(super::Ordering::Relaxed) as i32
    }
    pub fn situation_kind(actor: usize) -> i32 {
        (super::AIR.load(super::Ordering::Relaxed) ||
            if actor == super::P1.load(super::Ordering::Relaxed) {
                super::P1_AIR.load(super::Ordering::Relaxed)
            } else { super::CPU_AIR.load(super::Ordering::Relaxed) }) as i32
    }
    pub fn is_stop(actor: usize) -> bool {
        if actor == super::P1.load(super::Ordering::Relaxed) {
            super::P1_STOP.load(super::Ordering::Relaxed)
        } else { super::CPU_STOP.load(super::Ordering::Relaxed) }
    }
    pub fn pos_x(_: usize) -> f32 {
        0.0
    }
    pub fn pos_y(_: usize) -> f32 {
        0.0
    }
    pub fn pos_z(_: usize) -> f32 {
        0.0
    }
    pub fn lr(_: usize) -> f32 {
        f32::from_bits(super::ROLLER_LR.load(super::Ordering::Relaxed))
    }
    pub fn camera_limit_range() -> Option<[f32; 4]> {
        Some([-50.0, 50.0, 0.0, 100.0])
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
    #[derive(Clone, Copy)]
    pub struct Vector3f {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }
    pub fn clear_speed_all(_: usize) {}
    pub fn init_pos(_: usize, _: &Vector3f) {}
    pub fn set_lr(_: usize, _: f32) {}
    pub fn status_request(accessor: usize, status: i32) {
        super::REQUESTS.with_borrow_mut(|v| v.push((accessor, status)));
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
        Observe(u64, u64, u32, bool),
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
    match model::cpu::ids::NAMES[c as usize] {
        "FIGHTER_STATUS_KIND_SPECIAL_N" => return Some(476),
        "FIGHTER_STATUS_KIND_SPECIAL_S" => return Some(477),
        "FIGHTER_LUCAS_STATUS_KIND_SPECIAL_N_HOLD" => return Some(481),
        "FIGHTER_LUCAS_STATUS_SPECIAL_N_WORK_INT_NOBANG_TIME" => return Some(0x11000008),
        _ => (),
    }
    Some(match c {
        C::ClatterCaptureYoshi => 243,
        C::ClatterIce => 203,
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
    Settle,
    Place,
    Entering,
    Held,
    Result,
    Failed,
    Release,
    Choose,
    Restore,
    Retry,
    Reload,
}
struct Game {
    p1: usize,
    cpu: usize,
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
            p1: P1.load(Ordering::Relaxed),
            cpu: 2,
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
// JUDGE_CHOICE
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
    let request = game.requested.unwrap();
    let pluck = false; // START
}
unsafe fn update(game: &mut Game, p1: usize, status: i32, cpu: usize, fighter: usize) {
    let ids = ids();
    let cpu_entry = 1;
    // LINKS
    // JUDGE_ADMISSION
    // SEEDS
    // ENDS
    // RELEASED
    match game.step {
        // RETRY
        // PLACE
        // SETTLE
        // ENTERING
        // HELD
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
        EXECUTE_HELD.store(false, Ordering::Relaxed);
        CPU_STOP.store(false, Ordering::Relaxed);
        P1_STOP.store(false, Ordering::Relaxed);
        P1_AIR.store(false, Ordering::Relaxed);
        CPU_AIR.store(false, Ordering::Relaxed);
        JUDGE_FIXED.store(false, Ordering::Relaxed);
        JUDGE_KIND.store(7, Ordering::Relaxed);
        JUDGE_GETTER_GRANTED.store(true, Ordering::Relaxed);
        JUDGE_FLAG_GRANTED.store(true, Ordering::Relaxed);
        JUDGE_INT_READS.store(0, Ordering::Relaxed);
        JUDGE_FLAG_READS.store(0, Ordering::Relaxed);
        AIR.store(false, Ordering::Relaxed);
        ROLLER_LR.store(1.0f32.to_bits(), Ordering::Relaxed);
        REQUESTS.with_borrow_mut(Vec::clear);
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
                assert_eq!(
                    (m.control, m.record),
                    before,
                    "observer must preserve native latches and input"
                );
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
fn faulted(generation: u64, watermark: u64) -> bool {
    generation != 0 && watermark >= generation
}
impl EndMailbox {
    fn drain(&self, mut f: impl FnMut(NativeEnd)) {
        for ended in END_QUEUE.with_borrow_mut(std::mem::take) {
            f(ended);
        }
    }
    fn post(&self, ended: NativeEnd) -> bool {
        if QUEUE_FULL.load(Ordering::Relaxed) {
            return false;
        }
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
        assert!(
            end_token(control, 0).is_none(),
            "normal exhaustion is not an interruption"
        );
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
            if status == 75 {
                g.clock = Some(clock);
            }
            END_QUEUE.with_borrow_mut(|q| q.push(token));
            m.update(&mut g, status);
            assert_eq!(g.step, Step::Held, "End must not replace a native escape");
            assert!(g.clock.is_some());
            assert_eq!(g.failed, None);
            if status == 75 {
                assert!(g.clock.unwrap().exhausted());
            }
        }
    }
}

#[test]
fn lucas_real_charge_releases_only_at_native_eligible_hold_and_uses_ice_seed() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = Game::new(Plan::Freeze, State::Frozen);
        let request = g.requested.as_mut().unwrap();
        request.id = 112;
        request.attacker_kind = 40; // TR choices Lucas0x28.
        m.body[3] = 40;
        assert_eq!(
            entry::start(g.requested.unwrap()),
            Some("FIGHTER_STATUS_KIND_SPECIAL_N")
        );
        startup(&mut g, m.p1());
        assert_eq!(ARMED.load(Ordering::Relaxed), 5);
        FREEZE_NO_BANG.store(1, Ordering::Relaxed);
        CPU_STATUS.store(476, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert!(!g.followed);
        assert_eq!(DRIVEN.with_borrow(|v| v.last().unwrap().buttons), 2);
        CPU_STATUS.store(481, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert!(!g.followed);
        FREEZE_NO_BANG.store(0, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert!(g.followed);
        assert_eq!(
            DRIVEN.with_borrow(|v| *v.last().unwrap()),
            MappedInputs::NEUTRAL
        );
        FREEZE_NO_BANG.store(20, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert_eq!(
            DRIVEN.with_borrow(|v| *v.last().unwrap()),
            MappedInputs::NEUTRAL
        );
        m.seed(124.0, 4.0, 1);
        let actual = reader::sample(m.p1(), 0.37).unwrap().seed();
        m.start();
        m.update(&mut g, 203);
        assert_eq!(g.step, Step::Held);
        assert_eq!(g.clock.unwrap().last(), actual.record);
        assert_eq!(BEGINS.with_borrow(Vec::len), 1);
    }
}
#[test]
fn lucas_wait_without_ice_still_fails_and_wrong_victim_seed_never_grades() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = Game::new(Plan::Freeze, State::Frozen);
        startup(&mut g, m.p1());
        CPU_STATUS.store(481, Ordering::Relaxed);
        FREEZE_NO_BANG.store(0, Ordering::Relaxed);
        m.update(&mut g, 0);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert_eq!(
            g.failed,
            Some("Move did not capture · reopen menu to retry")
        );
        assert!(BEGINS.with_borrow(Vec::is_empty));
        let mut g = Game::new(Plan::Freeze, State::Frozen);
        startup(&mut g, m.p1());
        m.seed(100.0, 4.0, 1);
        m.start();
        m.update(&mut g, 244);
        assert_eq!(g.failed, Some("Move entry unconfirmed · no grade"));
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}

// Native status/position APIs are shims. Place, Settle, Entering, release
// predicate, arming, native seed admission and cancellation are actual source.
use model::cpu::hash::lua_bind_hash;
mod game {
    pub fn granted(offset: usize) -> bool {
        match offset {
            crate::model::sites_pkg2::WORK_IS_FLAG_OFFSET =>
                super::JUDGE_FLAG_GRANTED.load(super::Ordering::Relaxed),
            crate::model::sites_pkg2::WORK_GET_INT_OFFSET =>
                super::JUDGE_GETTER_GRANTED.load(super::Ordering::Relaxed),
            _ => panic!("unexpected Judge reader guard: {offset:x}"),
        }
    }
    pub fn lua_const(hash: u64) -> Option<i32> {
        for (name, id) in [
            ("FIGHTER_STATUS_KIND_SPECIAL_N", 476),
            ("FIGHTER_STATUS_KIND_SPECIAL_S", 477),
            ("FIGHTER_STATUS_KIND_SPECIAL_HI", 478),
            ("FIGHTER_STATUS_KIND_SPECIAL_LW", 479),
            ("FIGHTER_STATUS_KIND_CATCH", 50),
            ("FIGHTER_STATUS_KIND_CATCH_DASH", 54),
            ("FIGHTER_STATUS_KIND_CATCH_TURN", 55),
        ] {
            if super::lua_bind_hash(name) == hash {
                return Some(id);
            }
        }
        None
    }
}
fn actual_body_guard(game: &mut Game, p1: usize, cpu: usize) {
    // BODY_GUARD
}
fn olimar(m: &mut NativeMemory, grab: usize) -> Game {
    let mut g = Game::new(Plan::Grab, State::Grab);
    let q = g.requested.as_mut().unwrap();
    q.id = 40;
    q.attacker_kind = 43;
    q.attacker_name = "Olimar";
    q.grab = grab;
    m.body[3] = 43;
    g.step = Step::Settle;
    CPU_STATUS.store(0, Ordering::Relaxed);
    g
}
unsafe fn finish_pluck(m: &NativeMemory, g: &mut Game) {
    CPU_STATUS.store(476, Ordering::Relaxed);
    m.update(g, 0);
    assert_eq!(g.step, Step::Entering);
    CPU_STATUS.store(0, Ordering::Relaxed);
    m.update(g, 0);
    assert_eq!(g.step, Step::Place);
    assert!(g.followed);
    assert!(!g.saw_catch);
    assert_eq!(ARMED.load(Ordering::Relaxed), 0);
    m.update(g, 0);
    assert_eq!(g.step, Step::Settle);
    m.update(g, 0);
}
#[test]
fn olimar_real_pluck_then_configured_catch_once_and_actual_native_seed() {
    unsafe {
        for (grab, catch) in [(0, 50), (1, 54), (2, 55)] {
            let mut m = NativeMemory::new();
            let mut g = olimar(&mut m, grab);
            m.update(&mut g, 0);
            assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476)]);
            assert_eq!(g.step, Step::Entering);
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
            assert!(seed_token(m.p1()).is_none());
            finish_pluck(&m, &mut g);
            assert_eq!(
                REQUESTS.with_borrow(Clone::clone),
                vec![(2, 476), (m.p1(), 0), (2, 0), (2, catch)]
            );
            assert_eq!(g.step, Step::Entering);
            assert_eq!(ARMED.load(Ordering::Relaxed), g.generation);
            CPU_STATUS.store(52, Ordering::Relaxed);
            m.seed(60.0, 8.0, 1);
            m.start();
            m.update(&mut g, 66);
            assert_eq!(g.step, Step::Held);
            assert_eq!(BEGINS.with_borrow(Vec::len), 1);
        }
    }
}
#[test]
fn olimar_already_wait_or_unrelated_status_cannot_complete_or_arm_pluck() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = olimar(&mut m, 0);
        m.update(&mut g, 0);
        for status in [0, 0, 14, 0, 0] {
            CPU_STATUS.store(status, Ordering::Relaxed);
            m.update(&mut g, 0);
            assert_eq!(g.step, Step::Entering);
            assert!(!g.followed);
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        }
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476)]);
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}
#[test]
fn olimar_existing_ground_readiness_applies_before_and_after_pluck() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = olimar(&mut m, 0);
        AIR.store(true, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert_eq!(g.step, Step::Settle);
        assert!(REQUESTS.with_borrow(Vec::is_empty));
        AIR.store(false, Ordering::Relaxed);
        m.update(&mut g, 0);
        CPU_STATUS.store(476, Ordering::Relaxed);
        m.update(&mut g, 0);
        AIR.store(true, Ordering::Relaxed);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert_eq!(g.step, Step::Entering);
        assert!(!g.followed);
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476)]);
    }
}
#[test]
fn olimar_existing_cancel_and_body_guards_revoke_without_catch() {
    unsafe {
        for wrong_body in [false, true] {
            let mut m = NativeMemory::new();
            let mut g = olimar(&mut m, 0);
            m.update(&mut g, 0);
            if wrong_body {
                actual_body_guard(&mut g, m.p1(), 3);
                assert_eq!(g.step, Step::Failed);
            } else {
                cancel(&mut g, 0);
                assert_eq!(g.step, Step::Restore);
            }
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
            assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476)]);
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
    }
}
#[test]
fn olimar_stale_seed_ignored_and_unconfirmed_seed_keeps_existing_refusal() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = olimar(&mut m, 0);
        m.update(&mut g, 0);
        m.seed(60.0, 8.0, 1);
        QUEUE.with_borrow_mut(|q| {
            q.push(NativeSeed {
                generation: 4,
                epoch: 1,
                origin: SeedOrigin::Export,
                sample: reader::sample(m.p1(), 0.37),
            })
        });
        m.update(&mut g, 0);
        assert_eq!(g.step, Step::Entering);
        assert!(BEGINS.with_borrow(Vec::is_empty));
        QUEUE.with_borrow_mut(|q| {
            q.push(NativeSeed {
                generation: 5,
                epoch: 1,
                origin: SeedOrigin::Export,
                sample: reader::sample(m.p1(), 0.37),
            })
        });
        m.update(&mut g, 66);
        assert_eq!(g.step, Step::Failed);
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}
#[test]
fn olimar_ordinary_attacker_still_requests_configured_catch_immediately() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = olimar(&mut m, 0);
        g.requested.as_mut().unwrap().attacker_kind = 42;
        m.body[3] = 42;
        m.update(&mut g, 0);
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 50)]);
        assert_eq!(ARMED.load(Ordering::Relaxed), g.generation);
    }
}

#[test]
fn roller_starts_real_special_and_drives_forward_until_actual_bury_seed() {
    unsafe {
        for facing in [-1.0f32, 1.0] {
            let mut m = NativeMemory::new();
            let mut g = Game::new(Plan::Roller, State::Bury);
            g.requested.as_mut().unwrap().id = 102;
            g.requested.as_mut().unwrap().attacker_kind = 65;
            m.body[3] = 65;
            assert_eq!(entry::start(g.requested.unwrap()), Some("FIGHTER_STATUS_KIND_SPECIAL_S"));
            startup(&mut g, m.p1());
            assert_eq!(ARMED.load(Ordering::Relaxed), 5);
            ROLLER_LR.store(facing.to_bits(), Ordering::Relaxed);
            CPU_STATUS.store(477, Ordering::Relaxed);
            m.update(&mut g, 0);
            let input = DRIVEN.with_borrow(|v| *v.last().unwrap());
            assert_eq!((input.buttons,input.lstick_x,input.lstick_y), (0,if facing>0.0 {120}else{-120},0));
            assert!(g.clock.is_none());
            CPU_STATUS.store(486, Ordering::Relaxed);
            m.update(&mut g, 0);
            m.seed(136.25, 9.0, 1);
            let actual = reader::sample(m.p1(),0.37).unwrap().seed();
            m.start();
            m.update(&mut g,75);
            assert_eq!(g.step,Step::Held);
            assert_eq!(g.clock.unwrap().last(),actual.record);
            assert_eq!(BEGINS.with_borrow(Vec::len),1);
            assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()),MappedInputs::NEUTRAL);
        }
    }
}
#[test]
fn roller_miss_and_nonbury_seed_still_fail() {
    unsafe {
        let mut m=NativeMemory::new();
        let mut g=Game::new(Plan::Roller,State::Bury);
        startup(&mut g,m.p1());
        CPU_STATUS.store(477,Ordering::Relaxed);m.update(&mut g,0);
        CPU_STATUS.store(0,Ordering::Relaxed);m.update(&mut g,0);
        assert_eq!(g.failed,Some("Move did not capture · reopen menu to retry"));
        assert!(BEGINS.with_borrow(Vec::is_empty));
        let mut g=Game::new(Plan::Roller,State::Bury);startup(&mut g,m.p1());
        m.seed(100.0,9.0,1);m.start();m.update(&mut g,203);
        assert_eq!(g.failed,Some("Move entry unconfirmed · no grade"));
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}

#[test]
fn bury_specials_request_native_start_and_use_actual_bury_seed() {
    for (id, plan, kind, name, start, statuses) in [
        (99, Plan::UpSpecial, 42, "King Dedede", 478, &[478, 502, 503][..]),
        (108, Plan::Flip, 32, "Zero Suit Samus", 479, &[479, 484, 486, 487][..]),
    ] {
        unsafe {
            let mut m = NativeMemory::new();
            let mut g = Game::new(plan, State::Bury);
            let request = g.requested.as_mut().unwrap();
            request.id = id;
            request.attacker_kind = kind;
            request.attacker_name = name;
            m.body[3] = kind;
            g.step = Step::Settle;
            CPU_STATUS.store(0, Ordering::Relaxed);
            m.update(&mut g, 0);
            assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, start)]);
            assert_eq!(g.step, Step::Entering);
            assert_eq!(ARMED.load(Ordering::Relaxed), g.generation);
            for &cpu_status in statuses {
                CPU_STATUS.store(cpu_status, Ordering::Relaxed);
                m.update(&mut g, 0);
                assert_eq!(g.step, Step::Entering);
                assert_eq!(
                    DRIVEN.with_borrow(|v| *v.last().unwrap()),
                    MappedInputs::NEUTRAL
                );
                assert!(g.clock.is_none());
            }
            m.seed(136.25, 9.0, 1);
            let actual = reader::sample(m.p1(), 0.37).unwrap().seed();
            let before = (m.control, m.record);
            m.start();
            m.update(&mut g, 75);
            assert_eq!(g.step, Step::Held);
            assert_eq!(g.clock.unwrap().last(), actual.record);
            let begins = BEGINS.with_borrow(Clone::clone);
            assert_eq!(begins.len(), 1);
            assert_eq!(begins[0].1, actual);
            assert_eq!((m.control, m.record), before);
            assert_eq!(
                DRIVEN.with_borrow(|v| *v.last().unwrap()),
                MappedInputs::NEUTRAL
            );
            assert_eq!(
                entry::controls(plan, false),
                MappedInputs::NEUTRAL
            );
            assert_eq!(
                entry::controls(plan, true).buttons,
                if plan == Plan::UpSpecial { 2 } else { 0 }
            );
            assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, start)]);
        }
    }
}
#[test]
fn bury_specials_miss_and_nonbury_seed_keep_existing_refusals() {
    for (id, plan, kind, name, start, statuses) in [
        (99, Plan::UpSpecial, 42, "King Dedede", 478, &[478, 502, 503][..]),
        (108, Plan::Flip, 32, "Zero Suit Samus", 479, &[479, 484, 486, 487][..]),
    ] {
        unsafe {
            let mut m = NativeMemory::new();
            let mut g = Game::new(plan, State::Bury);
            let q = g.requested.as_mut().unwrap();
            q.id = id;
            q.attacker_kind = kind;
            q.attacker_name = name;
            m.body[3] = kind;
            startup(&mut g, m.p1());
            CPU_STATUS.store(start as u32, Ordering::Relaxed);
            m.update(&mut g, 0);
            CPU_STATUS.store(0, Ordering::Relaxed);
            m.update(&mut g, 0);
            assert_eq!(
                g.failed,
                Some("Move did not capture · reopen menu to retry")
            );
            assert!(BEGINS.with_borrow(Vec::is_empty));
            let mut g = Game::new(plan, State::Bury);
            let q = g.requested.as_mut().unwrap();
            q.id = id;
            q.attacker_kind = kind;
            q.attacker_name = name;
            m.body[3] = kind;
            startup(&mut g, m.p1());
            CPU_STATUS.store(*statuses.last().unwrap(), Ordering::Relaxed);
            m.seed(100.0, 9.0, 1);
            m.start();
            m.update(&mut g, 203);
            assert_eq!(g.failed, Some("Move entry unconfirmed · no grade"));
            assert!(g.clock.is_none());
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
    }
}

#[test]
fn ice_climbers_real_special_entries_require_actual_ice_seed() {
    for (id, plan, start) in [(113, Plan::Blizzard, 479), (114, Plan::IceShot, 476)] {
        unsafe {
            let mut m = NativeMemory::new();
            let mut g = Game::new(plan, State::Frozen);
            let q = g.requested.as_mut().unwrap();
            q.id = id;
            q.attacker_kind = 75;
            q.attacker_name = "Ice Climbers";
            m.body[3] = 75;
            g.step = Step::Settle;
            CPU_STATUS.store(0, Ordering::Relaxed);
            m.update(&mut g, 0);
            assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, start)]);
            assert_eq!(g.step, Step::Entering);
            assert_eq!(ARMED.load(Ordering::Relaxed), g.generation);
            CPU_STATUS.store(start as u32, Ordering::Relaxed);
            for _ in 0..4 {
                m.update(&mut g, 0);
                assert_eq!(g.step, Step::Entering);
                assert!(g.clock.is_none());
                assert_eq!(
                    DRIVEN.with_borrow(|v| *v.last().unwrap()),
                    MappedInputs::NEUTRAL
                );
            }
            m.seed(73.25, 1.75, 2);
            let actual = reader::sample(m.p1(), 0.37).unwrap().seed();
            let before = (m.control, m.record);
            m.start();
            m.update(&mut g, 203);
            assert_eq!(g.step, Step::Held);
            assert_eq!(g.clock.unwrap().last(), actual.record);
            assert_eq!((m.control, m.record), before);
            let begins = BEGINS.with_borrow(Clone::clone);
            assert_eq!(begins.len(), 1);
            assert_eq!(begins[0].1, actual);
            assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, start)]);
            assert_eq!(entry::controls(plan, false), MappedInputs::NEUTRAL);
            assert_eq!(entry::controls(plan, true), MappedInputs::NEUTRAL);
        }
    }
}
#[test]
fn ice_climbers_miss_and_nonice_seed_keep_existing_refusals() {
    for (id, plan, start) in [(113, Plan::Blizzard, 479), (114, Plan::IceShot, 476)] {
        unsafe {
            let mut m = NativeMemory::new();
            let mut g = Game::new(plan, State::Frozen);
            let q = g.requested.as_mut().unwrap();
            q.id = id;
            q.attacker_kind = 75;
            q.attacker_name = "Ice Climbers";
            m.body[3] = 75;
            startup(&mut g, m.p1());
            CPU_STATUS.store(start as u32, Ordering::Relaxed);
            m.update(&mut g, 0);
            CPU_STATUS.store(0, Ordering::Relaxed);
            m.update(&mut g, 0);
            assert_eq!(
                g.failed,
                Some("Move did not capture · reopen menu to retry")
            );
            assert!(BEGINS.with_borrow(Vec::is_empty));
            let mut g = Game::new(plan, State::Frozen);
            let q = g.requested.as_mut().unwrap();
            q.id = id;
            q.attacker_kind = 75;
            q.attacker_name = "Ice Climbers";
            startup(&mut g, m.p1());
            CPU_STATUS.store(start as u32, Ordering::Relaxed);
            m.seed(73.25, 1.75, 2);
            m.start();
            m.update(&mut g, 75);
            assert_eq!(g.failed, Some("Move entry unconfirmed · no grade"));
            assert!(g.clock.is_none());
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
    }
}

#[test]
fn dedede_bury_cancel_waits_for_confirmed_hold_and_end_of_stop() {
    unsafe {
        let mut m = NativeMemory::new();
        EXECUTE_HELD.store(true, Ordering::Relaxed);
        let mut g = Game::new(Plan::UpSpecial, State::Bury);
        m.body[3] = 42;
        let r = g.requested.as_mut().unwrap();
        r.id = 99;
        r.attacker_kind = 42;
        g.step = Step::Settle;
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert_eq!(g.step, Step::Entering);
        CPU_STATUS.store(502, Ordering::Relaxed);
        CPU_STOP.store(true, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert!(DRIVEN.with_borrow(|v| v.iter().all(|i| i.buttons == 0)));
        m.seed(204.38725, 8.0, 1);
        m.start();
        m.update(&mut g, 75);
        assert_eq!(g.step, Step::Held);
        assert!(!g.followed);
        assert_eq!(DRIVEN.with_borrow(|v| v.last().unwrap().buttons), 0);
        let n = DRIVEN.with_borrow(Vec::len);
        m.update(&mut g, 75);
        assert_eq!(DRIVEN.with_borrow(Vec::len), n);
        CPU_STOP.store(false, Ordering::Relaxed);
        m.update(&mut g, 75);
        assert_eq!(DRIVEN.with_borrow(|v| v.last().unwrap().buttons), 2);
        assert!(g.followed);
        let n = DRIVEN.with_borrow(Vec::len);
        m.update(&mut g, 75);
        assert_eq!(DRIVEN.with_borrow(Vec::len), n, "one native input edge");
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 478)]);
        assert_eq!(BEGINS.with_borrow(Vec::len), 1);
    }
}
#[test]
fn dedede_bury_cancel_never_runs_for_other_plans_or_exhausted_hold() {
    unsafe {
        for (plan, remaining) in [(Plan::UpSpecial, 0.0), (Plan::Flip, 90.0)] {
            let mut m = NativeMemory::new();
            EXECUTE_HELD.store(true, Ordering::Relaxed);
            let mut g = Game::new(plan, State::Bury);
            m.seed(90.0, 8.0, 1);
            startup(&mut g, m.p1());
            CPU_STOP.store(true, Ordering::Relaxed);
            m.start();
            m.update(&mut g, 75);
            assert_eq!(g.step, Step::Held);
            CPU_STOP.store(false, Ordering::Relaxed);
            if remaining == 0.0 {
                // A native escape status exhausts the real clock; no cancel is emitted.
                m.update(&mut g, 68);
                assert_eq!(g.step, Step::Result);
            } else {
                m.update(&mut g, 75);
            }
            assert!(DRIVEN.with_borrow(|v| v.iter().all(|i| i.buttons == 0)));
            assert!(!g.followed);
        }
    }
}


// The receiver functions and pre-seed/retry decisions above are extracted
// directly from runtime mashout.rs. Only native reads/requests are shims.
unsafe fn judge(m: &mut NativeMemory) -> Game {
    let mut g = Game::new(Plan::Judge, State::Frozen);
    let r = g.requested.as_mut().unwrap();
    r.id = 111;
    r.attacker_kind = 28;
    r.attacker_name = "Mr. Game & Watch";
    m.body[3] = 28;
    g.step = Step::Settle;
    CPU_STATUS.store(0, Ordering::Relaxed);
    m.update(&mut g, 0);
    assert_eq!(g.step, Step::Entering);
    assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 477)]);
    CPU_STATUS.store(477, Ordering::Relaxed);
    g
}
unsafe fn observe_unfixed_judge(m: &NativeMemory, g: &mut Game) {
    JUDGE_FIXED.store(false, Ordering::Relaxed);
    m.update(g, 0);
    assert_eq!(g.step, Step::Entering);
    assert!(g.followed, "actual native false flag observed for this attempt");
}
#[test]
fn judge_fresh_false_to_true_kind7_accepts_only_actual_ice_seed() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = judge(&mut m);
        observe_unfixed_judge(&m, &mut g);
        JUDGE_KIND.store(7, Ordering::Relaxed);
        JUDGE_FIXED.store(true, Ordering::Relaxed);
        m.seed(73.25, 1.75, 2);
        let actual = reader::sample(m.p1(), 0.37).unwrap().seed();
        let before = (m.control, m.record);
        m.start();
        m.update(&mut g, 203);
        assert_eq!(g.step, Step::Held);
        assert_eq!(g.clock.unwrap().last(), actual.record);
        assert_eq!((m.control, m.record), before);
        let begins = BEGINS.with_borrow(Clone::clone);
        assert_eq!(begins.len(), 1);
        assert_eq!(begins[0].0.0, 111);
        assert_eq!(begins[0].1, actual);
        assert_eq!(begins[0].2, g.epoch);
        assert!(DRIVEN.with_borrow(|v| v.iter().all(|i| *i == MappedInputs::NEUTRAL)));
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 477)]);
    }
}
#[test]
fn judge_stale_true_never_confirms_a_positive_ice_seed() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = judge(&mut m);
        JUDGE_FIXED.store(true, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert_eq!(g.step, Step::Entering);
        assert!(!g.followed);
        m.seed(73.25, 1.75, 2);
        m.start();
        m.update(&mut g, 203);
        assert_eq!(g.failed, Some("Move entry unconfirmed · no grade"));
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}
#[test]
fn judge_missing_native_read_guards_refuse_without_default_zero_other_roll() {
    unsafe {
        for missing_getter in [true, false] {
            let mut m = NativeMemory::new();
            let mut g = judge(&mut m);
            observe_unfixed_judge(&m, &mut g);
            JUDGE_FIXED.store(true, Ordering::Relaxed);
            JUDGE_KIND.store(7, Ordering::Relaxed);
            JUDGE_INT_READS.store(0, Ordering::Relaxed);
            JUDGE_FLAG_READS.store(0, Ordering::Relaxed);
            if missing_getter { JUDGE_GETTER_GRANTED.store(false, Ordering::Relaxed); }
            else { JUDGE_FLAG_GRANTED.store(false, Ordering::Relaxed); }
            m.update(&mut g, 0);
            assert_eq!(g.failed, Some(UNAVAILABLE_NOTICE));
            assert_ne!(g.step, Step::Retry);
            assert_eq!(JUDGE_INT_READS.load(Ordering::Relaxed), 0);
            assert_eq!(JUDGE_FLAG_READS.load(Ordering::Relaxed), 0);
            assert!(BEGINS.with_borrow(Vec::is_empty));
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        }
    }
}
#[test]
fn judge_every_positively_observed_other_choice_is_uncounted_retry() {
    unsafe {
        for kind in (0..=8).filter(|k| *k != 7) {
            let mut m = NativeMemory::new();
            let mut g = judge(&mut m);
            observe_unfixed_judge(&m, &mut g);
            let requested = g.requested;
            JUDGE_KIND.store(kind, Ordering::Relaxed);
            JUDGE_FIXED.store(true, Ordering::Relaxed);
            // Even a queued old/foreign-looking positive record cannot turn
            // a proved other roll into a scored Judge8 attempt.
            m.seed(73.25, 1.75, 2);
            m.start();
            m.update(&mut g, 203);
            assert_eq!(g.step, Step::Retry, "kind={kind}");
            assert_eq!(g.failed, None);
            assert!(g.clock.is_none());
            assert_eq!(g.requested, requested);
            assert!(BEGINS.with_borrow(Vec::is_empty));
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
            m.update(&mut g, 0); // queued seed drains while Retry; never Begin
            assert_eq!(g.step, Step::Retry);
            assert!(BEGINS.with_borrow(Vec::is_empty));
            assert!(DRIVEN.with_borrow(|v| v.iter().all(|i| *i == MappedInputs::NEUTRAL)));
        }
    }
}
#[test]
fn judge_invalid_native_choice_refuses_instead_of_retry_or_begin() {
    unsafe {
        for kind in [-1i32, 9, i32::MIN, i32::MAX] {
            let mut m = NativeMemory::new();
            let mut g = judge(&mut m);
            observe_unfixed_judge(&m, &mut g);
            JUDGE_KIND.store(kind as u32, Ordering::Relaxed);
            JUDGE_FIXED.store(true, Ordering::Relaxed);
            m.update(&mut g, 0);
            assert_eq!(g.failed, Some(UNAVAILABLE_NOTICE));
            assert_ne!(g.step, Step::Retry);
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
    }
}
#[test]
fn judge8_natural_choice_miss_and_wrong_victim_seed_still_fail() {
    unsafe {
        for wrong_victim in [false, true] {
            let mut m = NativeMemory::new();
            let mut g = judge(&mut m);
            observe_unfixed_judge(&m, &mut g);
            JUDGE_FIXED.store(true, Ordering::Relaxed);
            m.update(&mut g, 0); // native KIND7 fixed, no victim capture yet
            assert_eq!(g.step, Step::Entering);
            if wrong_victim {
                m.seed(73.25, 1.75, 2);
                m.start();
                m.update(&mut g, 75);
                assert_eq!(g.failed, Some("Move entry unconfirmed · no grade"));
            } else {
                CPU_STATUS.store(0, Ordering::Relaxed);
                m.update(&mut g, 0);
                assert_eq!(g.failed, Some("Move did not capture · reopen menu to retry"));
            }
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
    }
}
#[test]
fn judge_retry_waits_for_both_native_resting_bodies_then_reloads_same_selected_move() {
    unsafe {
        for blocked in ["p1-status", "cpu-status", "p1-air", "cpu-air", "p1-stop", "cpu-stop"] {
            let mut m = NativeMemory::new();
            let mut g = judge(&mut m);
            observe_unfixed_judge(&m, &mut g);
            JUDGE_KIND.store(0, Ordering::Relaxed);
            JUDGE_FIXED.store(true, Ordering::Relaxed);
            m.update(&mut g, 0);
            assert_eq!(g.step, Step::Retry);
            let request = g.requested;
            let generation = g.generation;
            GENERATION.store(generation, Ordering::Relaxed);
            CPU_STATUS.store(if blocked == "cpu-status" { 477 } else { 0 }, Ordering::Relaxed);
            P1_AIR.store(blocked == "p1-air", Ordering::Relaxed);
            CPU_AIR.store(blocked == "cpu-air", Ordering::Relaxed);
            P1_STOP.store(blocked == "p1-stop", Ordering::Relaxed);
            CPU_STOP.store(blocked == "cpu-stop", Ordering::Relaxed);
            m.update(&mut g, if blocked == "p1-status" { 75 } else { 0 });
            assert_eq!(g.step, Step::Retry, "{blocked}");
            assert_eq!(g.generation, generation);
            assert_eq!(g.requested, request);
            CPU_STATUS.store(0, Ordering::Relaxed);
            P1_AIR.store(false, Ordering::Relaxed);
            CPU_AIR.store(false, Ordering::Relaxed);
            P1_STOP.store(false, Ordering::Relaxed);
            CPU_STOP.store(false, Ordering::Relaxed);
            m.update(&mut g, 0);
            assert_eq!(g.step, Step::Reload);
            assert_eq!(g.requested, request, "selected111 must survive, never Choose/Random");
            assert_eq!(g.requested.unwrap().id, 111);
            assert_eq!(g.generation, generation + 1);
            assert!(!g.saw_catch);
            assert!(!g.followed);
            assert!(g.clock.is_none());
            assert!(BEGINS.with_borrow(Vec::is_empty));
            assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 477)]);
            assert!(DRIVEN.with_borrow(|v| v.iter().all(|i| *i == MappedInputs::NEUTRAL)));
        }
    }
}
