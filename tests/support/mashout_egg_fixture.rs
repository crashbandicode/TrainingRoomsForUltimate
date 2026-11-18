#![allow(dead_code)]
use std::{
    cell::{Cell, RefCell},
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
};
mod model {
    pub mod sites_pkg3 { // TROLLEY_SITES
    }
    pub mod sites_hitbox { // EGG_HITBOX_SITES
    }
    pub mod sites_pkg2 { // EGG_PKG2_SITES
    }
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
        pub mod clatter { // CLATTER
        }
        pub mod entry { // ENTRY
        }
        /// The ice116 record the runtime collects (the G2 solve itself is model-tested in ice116_tests).
        pub mod ice116 {
            pub const TAG: u16 = 116;
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub struct Applied { pub boundary: u32, pub old: u32, pub operand: u32, pub new: u32, pub d_max: u32 }
        }
    }
}
// The M3 Set observer stand-in: tests queue published writes; arm() calls are recorded.
mod clatter_set {
    use std::cell::RefCell;
    pub const RING: usize = 8;
    thread_local! {
        pub static PUBLISHED: RefCell<Vec<crate::model::mashout::clatter::ClatterWrite>> = const { RefCell::new(Vec::new()) };
        pub static ARMS: RefCell<Vec<(u32, u16)>> = const { RefCell::new(Vec::new()) };
    }
    pub fn poll(generation: u32, seq: u32) -> crate::model::mashout::clatter::Poll {
        PUBLISHED.with_borrow(|v| v.iter().copied().find(|w| w.generation == generation && w.seq == seq))
            .map_or(crate::model::mashout::clatter::Poll::Empty, crate::model::mashout::clatter::Poll::Event)
    }
    pub fn arm(generation: u32, tag: u16, _: usize) -> Result<u32, ()> {
        ARMS.with_borrow_mut(|v| v.push((generation, tag)));
        Ok(generation)
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
        pub fn granted(offset: usize) -> bool {
            offset != super::super::model::sites_pkg2::WORK_GET_INT_OFFSET
                || super::super::WORK_GET_INT_GRANTED.load(super::super::Ordering::Relaxed)
        }
    }
    pub mod roster {
        pub mod character_roster {
            pub fn solo_popo_active(_: bool) -> bool {
                crate::SOLO_POPO.load(crate::Ordering::Relaxed)
            }
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
static TROLLEY_MISSING_GRANT: AtomicUsize = AtomicUsize::new(0);
static TROLLEY_NATIVE_CALLS: AtomicUsize = AtomicUsize::new(0);
static TROLLEY_INT_READS: AtomicUsize = AtomicUsize::new(0);
static CART_EXISTS: AtomicBool = AtomicBool::new(true);
static CART_ACTIVE: AtomicBool = AtomicBool::new(true);
static CART_ARTICLE: AtomicUsize = AtomicUsize::new(0x100);
static CART_BOID: AtomicU32 = AtomicU32::new(0x200);
static CART_BOMA: AtomicUsize = AtomicUsize::new(0x300);
static CART_PARENT: AtomicU32 = AtomicU32::new(0x200);
static CART_COUNTER: AtomicU32 = AtomicU32::new(0);
static CART_GET_COUNT: AtomicUsize = AtomicUsize::new(0);
static CART_REPLACE_SECOND_READ: AtomicBool = AtomicBool::new(false);
static READY: AtomicBool = AtomicBool::new(true);
static SLOTS_READY: AtomicBool = AtomicBool::new(true);
static P1: AtomicUsize = AtomicUsize::new(0);
static THRESHOLD: AtomicU32 = AtomicU32::new(0);
static ARMED: AtomicU64 = AtomicU64::new(0);
static GENERATION: AtomicU64 = AtomicU64::new(5);
static CPU_STATUS: AtomicU32 = AtomicU32::new(0);
static ROLLER_LR: AtomicU32 = AtomicU32::new(1.0f32.to_bits());
static INIT_POS_X: AtomicU32 = AtomicU32::new(0);
static P1_X: AtomicU32 = AtomicU32::new(0);
static INIT_POS_CALLS: AtomicUsize = AtomicUsize::new(0);
static FREEZE_NO_BANG: AtomicU32 = AtomicU32::new(1);
static JUDGE_FIXED: AtomicBool = AtomicBool::new(false);
static JUDGE_KIND: AtomicU32 = AtomicU32::new(7);
static JUDGE_GETTER_GRANTED: AtomicBool = AtomicBool::new(true);
static JUDGE_FLAG_GRANTED: AtomicBool = AtomicBool::new(true);
static JUDGE_INT_READS: AtomicUsize = AtomicUsize::new(0);
static JUDGE_FLAG_READS: AtomicUsize = AtomicUsize::new(0);
static WORK_GET_INT_GRANTED: AtomicBool = AtomicBool::new(true);
static WORK_IS_FLAG_GRANTED: AtomicBool = AtomicBool::new(true);
static HERO_WINDOW_STATE: AtomicU32 = AtomicU32::new(0);
static HERO_INDEX: AtomicU32 = AtomicU32::new(0);
static HERO_COMMANDS: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];
static HERO_DECIDE: AtomicU32 = AtomicU32::new(17);
static HERO_ACTIVE: AtomicU32 = AtomicU32::new(17);
static HERO_SUCCESS_SP: AtomicBool = AtomicBool::new(false);
static HERO_INT_READS: AtomicUsize = AtomicUsize::new(0);
static HERO_FLAG_READS: AtomicUsize = AtomicUsize::new(0);
static TRAIL_MAGIC_KIND_VALUE: AtomicU32 = AtomicU32::new(0);
static TRAIL_MAGIC_KIND_RESOLVED: AtomicBool = AtomicBool::new(true);
// Sora116 F2: CPU x (P1 is at 0), P1 damage bits, live preparatory articles (None = reader unavailable), set_lr calls.
static CPU_X: AtomicU32 = AtomicU32::new(0x40c00000);
static P1_DAMAGE: AtomicU32 = AtomicU32::new(0x41bc0000);
static PREP_ARTICLE: AtomicU32 = AtomicU32::new(0);
// S1 (116): 0 = Blizzard provider closed, 1 = still open (N2 or live Ice), else reader unavailable.
static EGG_SPITED: AtomicBool = AtomicBool::new(false);
static YOSHI_PROVIDER_FIXTURE: AtomicBool = AtomicBool::new(false);
static YOSHI_ARTICLES: AtomicU32 = AtomicU32::new(0);
static EGG_VICTIM_SHELL: AtomicU32 = AtomicU32::new(0x5000_0000);
static EGG_WEAPON_KIND: AtomicU32 = AtomicU32::new(0x39);
static EGG_WEAPON_OWNER: AtomicU32 = AtomicU32::new(0x400);
static EGG_WEAPON_ACTIVE: AtomicBool = AtomicBool::new(false);
static EGG_WEAPON_ATTACK: AtomicBool = AtomicBool::new(false);
static EGG_PROVIDER_READER: AtomicUsize = AtomicUsize::new(0);
static BLIZZARD_PROVIDER: AtomicU32 = AtomicU32::new(0);
unsafe fn trail_blizzard_closed(cpu: usize) -> Result<bool, ()> {
    assert_eq!(cpu, 2);
    match BLIZZARD_PROVIDER.load(Ordering::Relaxed) { 0 => Ok(true), 1 => Ok(false), _ => Err(()) }
}
thread_local! { static LR_SETS: RefCell<Vec<(usize, f32)>> = const { RefCell::new(Vec::new()) }; }
unsafe fn trail_prep_article_live(cpu: usize) -> Result<bool, ()> {
    assert_eq!(cpu, 2);
    match PREP_ARTICLE.load(Ordering::Relaxed) { 0 => Ok(false), 1 => Ok(true), _ => Err(()) }
}
static TRAIL_MAGIC_READS: AtomicUsize = AtomicUsize::new(0);
static COVERED: AtomicBool = AtomicBool::new(false);
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
    // Shim of the runtime's olimar_pikmin (Save-State respawn/line natives); None models an unavailable native or constant.
    static PIKMIN_LINE: RefCell<Option<[Option<i32>; 3]>> = const { RefCell::new(Some([Some(1), Some(2), Some(3)])) };
    static PIKMIN_READS: RefCell<Vec<[Option<i32>; 3]>> = const { RefCell::new(Vec::new()) };
    static RESPAWNS: RefCell<Vec<[Option<i32>; 3]>> = const { RefCell::new(Vec::new()) };
}
static LINE_READS: AtomicUsize = AtomicUsize::new(0);
unsafe fn olimar_pikmin(cpu: usize, line: Option<[Option<i32>; 3]>) -> Option<[Option<i32>; 3]> {
    assert_eq!(cpu, 2);
    PIKMIN_LINE.with_borrow(|l| *l)?;
    if let Some(line) = line {
        RESPAWNS.with_borrow_mut(|r| r.push(line));
        PIKMIN_LINE.set(Some(line));
    } else {
        LINE_READS.fetch_add(1, Ordering::Relaxed);
        if let Some(forced) = PIKMIN_READS.with_borrow_mut(|q| (!q.is_empty()).then(|| q.remove(0))) {
            return Some(forced);
        }
    }
    PIKMIN_LINE.with_borrow(|l| *l)
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
        if id == 0x21000010 {
            assert_eq!(cpu, 2);
            return super::EGG_SPITED.load(super::Ordering::Relaxed);
        }
        if Some(id) == super::state_id(super::C::HeroSuccessSp) {
            assert_eq!(cpu, 2);
            super::HERO_FLAG_READS.fetch_add(1, super::Ordering::Relaxed);
            return super::HERO_SUCCESS_SP.load(super::Ordering::Relaxed);
        }
        assert_eq!(cpu, 2);
        assert_eq!(Some(id), super::state_id(super::C::JudgeFixed));
        super::JUDGE_FLAG_READS.fetch_add(1, super::Ordering::Relaxed);
        super::JUDGE_FIXED.load(super::Ordering::Relaxed)
    }
    pub fn get_int(cpu: usize, id: i32) -> i32 {
        if super::ICE_SHOT_FIXTURE.load(super::Ordering::Relaxed) {
            if id == 0x1000_00be {
                assert_eq!(cpu, 2);
                return super::NANA_OBJECT.load(super::Ordering::Relaxed) as i32;
            }
            if cpu == super::NANA_BOMA && Some(id) == super::state_id(super::C::WorkIntEntryId) {
                return super::NANA_ENTRY.load(super::Ordering::Relaxed) as i32;
            }
        }
        if id == 0x1000_0056 {
            assert_eq!(cpu, super::P1.load(super::Ordering::Relaxed));
            return super::EGG_VICTIM_SHELL.load(super::Ordering::Relaxed) as i32;
        }
        if id == 0x1000_0000 && super::YOSHI_PROVIDER_FIXTURE.load(super::Ordering::Relaxed) {
            assert_eq!(cpu, 0x200);
            return super::EGG_WEAPON_OWNER.load(super::Ordering::Relaxed) as i32;
        }
        if id == 0x100000bf {
            assert_eq!(cpu, 2);
            super::TRAIL_MAGIC_READS.fetch_add(1, super::Ordering::Relaxed);
            return super::TRAIL_MAGIC_KIND_VALUE.load(super::Ordering::Relaxed) as i32;
        }
        if Some(id) == super::state_id(super::C::HeroWindowState) {
            assert_eq!(cpu, 2);
            super::HERO_INT_READS.fetch_add(1, super::Ordering::Relaxed);
            return super::HERO_WINDOW_STATE.load(super::Ordering::Relaxed) as i32;
        }
        if Some(id) == super::state_id(super::C::HeroSelectIndex) {
            assert_eq!(cpu, 2);
            super::HERO_INT_READS.fetch_add(1, super::Ordering::Relaxed);
            return super::HERO_INDEX.load(super::Ordering::Relaxed) as i32;
        }
        if id == 0x11000005 {
            assert_eq!(cpu, 2);
            super::HERO_INT_READS.fetch_add(1, super::Ordering::Relaxed);
            return super::HERO_ACTIVE.load(super::Ordering::Relaxed) as i32;
        }
        if Some(id) == super::state_id(super::C::HeroDecideCommand) {
            assert_eq!(cpu, 2);
            super::HERO_INT_READS.fetch_add(1, super::Ordering::Relaxed);
            return super::HERO_DECIDE.load(super::Ordering::Relaxed) as i32;
        }
        for (index, command) in [super::C::HeroCommand1, super::C::HeroCommand2,
            super::C::HeroCommand3, super::C::HeroCommand4].into_iter().enumerate()
        {
            if Some(id) == super::state_id(command) {
                assert_eq!(cpu, 2);
                super::HERO_INT_READS.fetch_add(1, super::Ordering::Relaxed);
                return super::HERO_COMMANDS[index].load(super::Ordering::Relaxed) as i32;
            }
        }
        if Some(id) == super::state_id(super::C::PickelTrolleyJumpDisable) {
            super::TROLLEY_NATIVE_CALLS.fetch_add(1, super::Ordering::Relaxed);
            super::TROLLEY_INT_READS.fetch_add(1, super::Ordering::Relaxed);
            assert_eq!(cpu, super::CART_BOMA.load(super::Ordering::Relaxed));
            return super::CART_COUNTER.load(super::Ordering::Relaxed) as i32;
        }
        if id == 0x100000c1 || id == 0x100000c2 {
            assert_eq!(cpu, 2);
            super::JUDGE_INT_READS.fetch_add(1, super::Ordering::Relaxed);
            // The real native shim defaults to0 when unavailable; this must
            // never be mistaken for a positively observed other Judge roll.
            return if super::JUDGE_GETTER_GRANTED.load(super::Ordering::Relaxed) {
                // Native init_panel commits this roll to PREV_KIND, then clears KIND.
                if id == 0x100000c1 { super::JUDGE_KIND.load(super::Ordering::Relaxed) as i32 }
                else { -1 }
            } else { 0 };
        }
        assert_eq!(id, 0x11000008); // Lucas work id, pinned01bf583/native table.
        super::FREEZE_NO_BANG.load(super::Ordering::Relaxed) as i32
    }
    pub fn module_accessor(_: i32) -> Option<usize> {
        Some(super::P1.load(super::Ordering::Relaxed))
    }
    pub fn fighter_kind(accessor: usize) -> Option<i32> {
        if accessor == super::NANA_BOMA && super::ICE_SHOT_FIXTURE.load(super::Ordering::Relaxed) {
            return Some(0x4c);
        }
        assert_eq!(accessor, 0x200);
        Some(super::EGG_WEAPON_KIND.load(super::Ordering::Relaxed) as i32)
    }
    pub fn attack_is_attack(accessor: usize, slot: i32) -> bool {
        assert_eq!(accessor, 0x200);
        assert!((0..7).contains(&slot));
        slot == 6 && super::EGG_WEAPON_ATTACK.load(super::Ordering::Relaxed)
    }
    pub fn status_kind(accessor: usize) -> i32 {
        if accessor == super::NANA_BOMA {
            return super::NANA_STATUS.load(super::Ordering::Relaxed) as i32;
        }
        if accessor == 0x200 {
            return 2;
        }
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
    pub fn pos_x(actor: usize) -> f32 {
        let x = if actor == 2 { &super::CPU_X } else { &super::P1_X };
        f32::from_bits(x.load(super::Ordering::Relaxed))
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
        f32::from_bits(super::P1_DAMAGE.load(super::Ordering::Relaxed))
    }
}
// The native floor check belongs to Trolley. Egg preparation must never reach it.
unsafe fn trolley_floor_allows(_: usize, _: f32, _: f32, _: f32, _: f32) -> bool {
    panic!("Egg fixture reached the Trolley-only floor check")
}

mod state_natives {
    fn read() { super::TROLLEY_NATIVE_CALLS.fetch_add(1, super::Ordering::Relaxed); }
    pub fn article_is_exist(cpu: usize, article: i32) -> bool {
        assert_ne!(cpu, 0);
        if super::ICE_SHOT_FIXTURE.load(super::Ordering::Relaxed) {
            assert_eq!(article, 0);
            assert!(matches!(cpu, 2 | super::NANA_BOMA));
            let mask = if cpu == 2 { 1 } else { 2 };
            return super::ICE_SHOT_ARTICLES.load(super::Ordering::Relaxed) & mask != 0;
        }
        if super::YOSHI_PROVIDER_FIXTURE.load(super::Ordering::Relaxed) {
            assert_eq!(cpu, 2);
            return super::YOSHI_ARTICLES.load(super::Ordering::Relaxed) & (1 << article) != 0;
        }
        read();
        article == 2 && super::CART_EXISTS.load(super::Ordering::Relaxed)
    }
    pub fn article_get(_: usize, _: i32) -> usize {
        if super::YOSHI_PROVIDER_FIXTURE.load(super::Ordering::Relaxed) {
            return 0x100;
        }
        read();
        let n=super::CART_GET_COUNT.fetch_add(1,super::Ordering::Relaxed);
        super::CART_ARTICLE.load(super::Ordering::Relaxed)
            + usize::from(super::CART_REPLACE_SECOND_READ.load(super::Ordering::Relaxed) && n%2==1)
    }
    pub fn article_object_id(_: usize) -> u32 {
        if super::YOSHI_PROVIDER_FIXTURE.load(super::Ordering::Relaxed) {
            return 0x1000_0402;
        }
        read(); super::CART_BOID.load(super::Ordering::Relaxed)
    }
    pub fn object_is_active(id: u32) -> bool {
        if super::ICE_SHOT_FIXTURE.load(super::Ordering::Relaxed) {
            return id == 0x402;
        }
        if super::YOSHI_PROVIDER_FIXTURE.load(super::Ordering::Relaxed) {
            assert_eq!(id, 0x1000_0402);
            return super::EGG_WEAPON_ACTIVE.load(super::Ordering::Relaxed);
        }
        read(); super::CART_ACTIVE.load(super::Ordering::Relaxed)
    }
    pub fn object_accessor(id: u32) -> usize {
        if super::ICE_SHOT_FIXTURE.load(super::Ordering::Relaxed) {
            assert_eq!(id, 0x402);
            return super::NANA_BOMA;
        }
        if super::YOSHI_PROVIDER_FIXTURE.load(super::Ordering::Relaxed) {
            return match id {
                0x400 => super::P1.load(super::Ordering::Relaxed),
                0x401 => 2,
                0x1000_0402 => 0x200,
                _ => panic!("unexpected Egg Lay object: {id:#x}"),
            };
        }
        read(); super::CART_BOMA.load(super::Ordering::Relaxed)
    }
    pub fn link_parent_object_id(_: usize, link:i32) -> u64 {
        read(); assert_eq!(link,28); super::CART_PARENT.load(super::Ordering::Relaxed) as u64
    }
    #[derive(Clone, Copy)]
    pub struct Vector3f {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }
    pub fn clear_speed_all(_: usize) {}
    pub fn init_pos(actor: usize, position: &Vector3f) {
        assert_eq!(actor, 2);
        super::INIT_POS_X.store(position.x.to_bits(), super::Ordering::Relaxed);
        super::INIT_POS_CALLS.fetch_add(1, super::Ordering::Relaxed);
    }
    pub fn set_lr(actor: usize, lr: f32) {
        super::LR_SETS.with_borrow_mut(|v| v.push((actor, lr)));
    }
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
        Writes(u64, u64, [crate::model::mashout::ice116::Applied; 8], u8),
        EggWrites(u64, u64, [crate::model::mashout::ice116::Applied; 8], u8),
    }
}
thread_local! { static WRITES_POSTED: RefCell<Vec<(usize, [model::mashout::ice116::Applied; 8])>> = const { RefCell::new(Vec::new()) }; }
thread_local! { static FINISHED: Cell<usize> = const { Cell::new(0) }; }
fn post(e: worker::Event, _: u64) {
    if matches!(e, worker::Event::Observe(_, _, _, true)) {
        FINISHED.set(FINISHED.get() + 1);
    }
    if let worker::Event::Writes(_, _, sets, n) | worker::Event::EggWrites(_, _, sets, n) = e {
        WRITES_POSTED.with_borrow_mut(|w| w.push((usize::from(n), sets)));
    }
    if let worker::Event::Begin(_, epoch, k, s, origin) = e {
        BEGINS.with_borrow_mut(|b| b.push((k, s, epoch, origin)));
    }
    if let worker::Event::NativeEnd(generation, epoch) = e {
        NATIVE_ENDS.with_borrow_mut(|v| v.push((generation, epoch)));
    }
}
thread_local! { static NATIVE_ENDS: RefCell<Vec<(u64, u64)>> = const { RefCell::new(Vec::new()) }; }
fn state_id(c: C) -> Option<i32> {
    if c == C::TrailMagicKind && !TRAIL_MAGIC_KIND_RESOLVED.load(Ordering::Relaxed) {
        return None;
    }
    match model::cpu::ids::NAMES[c as usize] {
        "FIGHTER_KIND_BRAVE" => return Some(0x53),
        "FIGHTER_STATUS_KIND_SPECIAL_LW" => return Some(479),
        "FIGHTER_BRAVE_STATUS_KIND_SPECIAL_LW_SELECT" => return Some(493),
        "FIGHTER_BRAVE_STATUS_KIND_SPECIAL_LW_CANCEL" => return Some(494),
        "FIGHTER_BRAVE_STATUS_KIND_SPECIAL_LW_FAILURE" => return Some(496),
        "FIGHTER_BRAVE_STATUS_KIND_SPECIAL_LW_START" => return Some(497),
        "FIGHTER_STATUS_KIND_SPECIAL_N" => return Some(476),
        "FIGHTER_POPO_GENERATE_ARTICLE_ICESHOT" => return Some(0),
        "FIGHTER_POPO_INSTANCE_WORK_ID_INT_PARTNER_OBJECT_ID" => return Some(0x1000_00be),
        "FIGHTER_KIND_NANA" => return Some(0x4c),
        "BATTLE_OBJECT_ID_INVALID" => return Some(0x5000_0000),
        "FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID" => return Some(0x1000_0000),
        "FIGHTER_STATUS_KIND_SPECIAL_S" => return Some(477),
        "FIGHTER_LUCAS_STATUS_KIND_SPECIAL_N_HOLD" => return Some(481),
        "FIGHTER_LUCAS_STATUS_SPECIAL_N_WORK_INT_NOBANG_TIME" => return Some(0x11000008),
        "FIGHTER_GAMEWATCH_INSTANCE_WORK_ID_INT_SPECIAL_S_KIND" => return Some(0x100000c2),
        "FIGHTER_GAMEWATCH_INSTANCE_WORK_ID_INT_SPECIAL_S_PREV_KIND" => return Some(0x100000c1),
        _ => (),
    }
    Some(match c {
        C::HeroIceSwordCommand => 17,
        C::HeroWindowOpen => 3,
        C::HeroSelectIndex => 0x100000c8,
        C::HeroDecideCommand => 0x100000c9,
        C::HeroActiveCommand => 0x11000005,
        C::HeroWindowState => 0x100000d1,
        C::HeroCommand1 => 0x100000d4,
        C::HeroCommand2 => 0x100000d5,
        C::HeroCommand3 => 0x100000d6,
        C::HeroCommand4 => 0x100000d7,
        C::HeroSuccessSp => 0x2100000c,
        C::TrailSpecialN1 => 483,
        C::TrailSpecialN1Shoot => 484,
        C::TrailSpecialN1End => 485,
        C::TrailSpecialN2 => 486,
        C::TrailSpecialN3 => 487,
        C::TrailMagicKind => 0x100000bf,
        C::TrailMagicFire => 0,
        C::TrailMagicBlizzard => 1,
        C::TrailMagicThunder => 2,
        C::PickelTrolleyFailed => 518,
        C::PickelTrolleyRide => 519,
        C::PickelTrolleyDrive => 520,
        C::PickelTrolleyJump => 521,
        C::PickelTrolleyArticle => 2,
        C::PickelTrolleyLink => 28,
        C::PickelTrolleyJumpDisable => 0x10000021,
        C::ClatterCatchedPickelTrolley => 446,
        // Pinned skyline_smash lua_const.rs:19102/19104.
        C::ClatterShoulderedDonkeyStart => 0xC5,
        C::ClatterShoulderedDonkey => 0xC6,
        // Pinned lua_const.rs:18800-18802 / 5449.
        C::ClatterCatchedAirGanon => 0xBF,
        C::ClatterCatchedAirFallGanon => 0xC0,
        C::CatchedAirEndGanon => 0xC1,
        C::GanonSpecialAirSEnd => 0x1E5,
        C::ClatterCaptureYoshi => 243,
        C::ClatterIce => 203,
        C::ClatterYoshiEgg => 244,
        C::YoshiSpecialN1 => 481,
        C::YoshiSpecialN2 => 482,
        C::YoshiSpited => 0x21000010,
        C::YoshiArticleStar => 0,
        C::YoshiArticleTamago => 1,
        C::YoshiArticleMob => 2,
        C::YoshiArticleBg => 3,
        C::YoshiTamagoSpit => 2,
        C::YoshiVictimEgg => 0x1000_0056,
        C::YoshiTamagoKind => 0x39,
        C::WeaponLinkOwner => 0x1000_0000,
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
    roll: Option<Scenario>,
    followed: bool,
    saw_catch: bool,
    clock: Option<Clock>,
    writes: Option<model::mashout::clatter::Writes>,
    applied: bool,
    sets: [crate::model::mashout::ice116::Applied; 8],
    set_count: usize,
    egg_spat: bool,
    egg_shell: Option<u32>,
    provider_close_frames: u8,
    config: Option<()>,
    restore_requested: bool,
    failed: Option<&'static str>,
    trolley: Option<entry::TrolleyEntry>,
    hero: Option<entry::HeroEntry>,
    trail_magic: Option<entry::TrailMagicEntry>,
    prep_damage: Option<u32>,
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
            roll: None,
            followed: false,
            saw_catch: false,
            clock: None,
            writes: None,
            applied: false,
            sets: [crate::model::mashout::ice116::Applied { boundary: 0, old: 0, operand: 0, new: 0, d_max: 0 }; 8],
            set_count: 0,
            egg_spat: false,
            egg_shell: None,
            provider_close_frames: 0,
            config: Some(()),
            restore_requested: false,
            failed: None,
            trolley: None,
            hero: None,
            trail_magic: None,
            prep_damage: None,
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
// TROLLEY_INPUT
// EGG_CLOSURE
// ICE_SHOT_CLOSURE
// TRAIL_MAGIC_KIND
// TRAIL_MAGIC_STATUS
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
    let preparing = false; // START
}
unsafe fn update(game: &mut Game, p1: usize, status: i32, cpu: usize, fighter: usize) {
    let ids = ids();
    let cpu_entry = 1;
    let covered = COVERED.load(Ordering::Relaxed);
    // LINKS
    // ENTRY_ADMISSION
    // SEEDS
    // ENDS
    // RELEASED
    // COVERED
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
        reset_cart();
        ARMED.store(0, Ordering::Relaxed);
        EPOCH.store(0, Ordering::Relaxed);
        FAULT.store(0, Ordering::Relaxed);
        QUEUE_FULL.store(false, Ordering::Relaxed);
        READY.store(true, Ordering::Relaxed);
        SLOTS_READY.store(true, Ordering::Relaxed);
        CPU_STATUS.store(481, Ordering::Relaxed);
        EGG_SPITED.store(false, Ordering::Relaxed);
        YOSHI_PROVIDER_FIXTURE.store(false, Ordering::Relaxed);
        YOSHI_ARTICLES.store(0, Ordering::Relaxed);
        EGG_VICTIM_SHELL.store(0x5000_0000, Ordering::Relaxed);
        EGG_WEAPON_KIND.store(0x39, Ordering::Relaxed);
        EGG_WEAPON_OWNER.store(0x400, Ordering::Relaxed);
        EGG_WEAPON_ACTIVE.store(false, Ordering::Relaxed);
        EGG_WEAPON_ATTACK.store(false, Ordering::Relaxed);
        EGG_PROVIDER_READER.store(0, Ordering::Relaxed);
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
        WORK_GET_INT_GRANTED.store(true, Ordering::Relaxed);
        WORK_IS_FLAG_GRANTED.store(true, Ordering::Relaxed);
        HERO_WINDOW_STATE.store(0, Ordering::Relaxed);
        HERO_INDEX.store(0, Ordering::Relaxed);
        HERO_COMMANDS[0].store(4, Ordering::Relaxed);
        HERO_COMMANDS[1].store(17, Ordering::Relaxed);
        HERO_COMMANDS[2].store(7, Ordering::Relaxed);
        HERO_COMMANDS[3].store(12, Ordering::Relaxed);
        HERO_DECIDE.store(17, Ordering::Relaxed);
        HERO_ACTIVE.store(17, Ordering::Relaxed);
        HERO_SUCCESS_SP.store(false, Ordering::Relaxed);
        HERO_INT_READS.store(0, Ordering::Relaxed);
        HERO_FLAG_READS.store(0, Ordering::Relaxed);
        TRAIL_MAGIC_KIND_VALUE.store(0, Ordering::Relaxed);
        TRAIL_MAGIC_KIND_RESOLVED.store(true, Ordering::Relaxed);
        CPU_X.store(6.0f32.to_bits(), Ordering::Relaxed);
        P1_DAMAGE.store(23.5f32.to_bits(), Ordering::Relaxed);
        PREP_ARTICLE.store(0, Ordering::Relaxed);
        BLIZZARD_PROVIDER.store(0, Ordering::Relaxed);
        LR_SETS.with_borrow_mut(Vec::clear);
        TRAIL_MAGIC_READS.store(0, Ordering::Relaxed);
        ICE_SHOT_FIXTURE.store(false, Ordering::Relaxed);
        COVERED.store(false, Ordering::Relaxed);
        AIR.store(false, Ordering::Relaxed);
        ROLLER_LR.store(1.0f32.to_bits(), Ordering::Relaxed);
        INIT_POS_X.store(0, Ordering::Relaxed);
        INIT_POS_CALLS.store(0, Ordering::Relaxed);
        P1_X.store(0.0_f32.to_bits(), Ordering::Relaxed);
        REQUESTS.with_borrow_mut(Vec::clear);
        QUEUE.with_borrow_mut(Vec::clear);
        END_QUEUE.with_borrow_mut(Vec::clear);
        BEGINS.with_borrow_mut(Vec::clear);
        NATIVE_ENDS.with_borrow_mut(Vec::clear);
        DRIVEN.with_borrow_mut(Vec::clear);
        PIKMIN_LINE.set(Some([Some(1), Some(2), Some(3)]));
        PIKMIN_READS.with_borrow_mut(Vec::clear);
        RESPAWNS.with_borrow_mut(Vec::clear);
        LINE_READS.store(0, Ordering::Relaxed);
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
        if super::TROLLEY_MISSING_GRANT.load(super::Ordering::Relaxed) == offset { return false; }
        if super::EGG_PROVIDER_READER.load(super::Ordering::Relaxed) == offset { return false; }
        match offset {
            crate::model::sites_pkg2::STATUS_KIND_OFFSET | crate::model::sites_pkg2::UTILITY_GET_KIND_OFFSET => true,
            crate::model::sites_pkg2::WORK_IS_FLAG_OFFSET =>
                super::WORK_IS_FLAG_GRANTED.load(super::Ordering::Relaxed)
                    && super::JUDGE_FLAG_GRANTED.load(super::Ordering::Relaxed),
            crate::model::sites_pkg2::WORK_GET_INT_OFFSET =>
                super::WORK_GET_INT_GRANTED.load(super::Ordering::Relaxed)
                    && super::JUDGE_GETTER_GRANTED.load(super::Ordering::Relaxed),
            crate::model::sites_pkg3::STATE_ARTICLE_IS_EXIST_OFFSET
            | crate::model::sites_pkg3::STATE_ARTICLE_GET_ARTICLE_OFFSET
            | crate::model::sites_pkg3::STATE_ARTICLE_OBJECT_ID_OFFSET
            | crate::model::sites_pkg3::STATE_OBJECT_IS_ACTIVE_OFFSET
            | crate::model::sites_pkg3::STATE_OBJECT_MODULE_ACCESSOR_OFFSET
            | crate::model::sites_pkg3::STATE_LINK_PARENT_OBJECT_ID_OFFSET
            | crate::model::sites_hitbox::HITBOX_IS_ATTACK_OFFSET => true,
            _ => panic!("unexpected reader guard: {offset:x}"),
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
fn requests() -> Vec<(usize, i32)> {
    REQUESTS.with_borrow(Clone::clone)
}
fn respawns() -> Vec<[Option<i32>; 3]> {
    RESPAWNS.with_borrow(Clone::clone)
}
// Two released reads, each exactly one Pikmin, then the existing Place and Settle.
unsafe fn finish_line(m: &NativeMemory, g: &mut Game) {
    m.update(g, 0);
    assert_eq!((g.step, g.followed, LINE_READS.load(Ordering::Relaxed)), (Step::Entering, false, 1));
    assert_eq!(ARMED.load(Ordering::Relaxed), 0);
    m.update(g, 0);
    assert_eq!((g.step, g.followed, g.saw_catch), (Step::Place, true, false));
    assert_eq!(LINE_READS.load(Ordering::Relaxed), 2);
    assert_eq!(ARMED.load(Ordering::Relaxed), 0);
    m.update(g, 0);
    assert_eq!(g.step, Step::Settle);
    m.update(g, 0);
}
#[test]
fn olimar_one_pikmin_line_then_configured_catch_once_and_actual_native_seed() {
    unsafe {
        assert_eq!(entry::ONE_PIKMIN, [Some(0), None, None]);
        for (grab, catch) in [(0, 50), (1, 54), (2, 55)] {
            let mut m = NativeMemory::new();
            let mut g = olimar(&mut m, grab);
            m.update(&mut g, 0);
            assert_eq!(respawns(), vec![entry::ONE_PIKMIN]);
            assert!(requests().is_empty(), "no Pluck or other status request");
            assert_eq!(g.step, Step::Entering);
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
            assert!(seed_token(m.p1()).is_none());
            assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);
            finish_line(&m, &mut g);
            assert_eq!(requests(), vec![(m.p1(), 0), (2, 0), (2, catch)]);
            assert_eq!(respawns().len(), 1, "one line per attempt");
            assert_eq!(g.step, Step::Entering);
            assert_eq!(ARMED.load(Ordering::Relaxed), g.generation);
            CPU_STATUS.store(52, Ordering::Relaxed);
            m.seed(60.0, 8.0, 1);
            m.start();
            m.update(&mut g, 66);
            assert_eq!(g.step, Step::Held);
            assert_eq!(BEGINS.with_borrow(Vec::len), 1);
            assert_eq!(LINE_READS.load(Ordering::Relaxed), 2);
        }
    }
}
#[test]
fn olimar_line_other_than_one_pikmin_fails_closed_unavailable() {
    unsafe {
        let one = Some(entry::ONE_PIKMIN);
        for (first, second) in [
            ([Some(0), Some(0), None], None),
            ([Some(0), Some(1), Some(2)], None),
            ([None; 3], None),
            ([None, Some(0), None], None),
            (entry::ONE_PIKMIN, Some([Some(0), Some(1), None])),
            (entry::ONE_PIKMIN, Some([None; 3])),
        ] {
            let mut m = NativeMemory::new();
            let mut g = olimar(&mut m, 0);
            m.update(&mut g, 0);
            assert_eq!(PIKMIN_LINE.with_borrow(|l| *l), one);
            PIKMIN_READS.with_borrow_mut(|q| q.extend([first].into_iter().chain(second)));
            m.update(&mut g, 0);
            if second.is_some() {
                assert_eq!(g.step, Step::Entering, "{first:?}");
                m.update(&mut g, 0);
            }
            assert_eq!((g.step, g.failed), (Step::Failed, Some(UNAVAILABLE_NOTICE)), "{first:?} {second:?}");
            assert!(!g.followed);
            assert!(requests().is_empty(), "never placed or caught");
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
            assert!(BEGINS.with_borrow(Vec::is_empty));
            m.update(&mut g, 0);
            assert_eq!((g.step, respawns().len()), (Step::Failed, 1), "no retry or second line");
        }
    }
}
#[test]
fn olimar_unavailable_line_natives_fail_closed_before_respawn() {
    unsafe {
        for later in [false, true] {
            let mut m = NativeMemory::new();
            let mut g = olimar(&mut m, 0);
            if later {
                m.update(&mut g, 0);
                assert_eq!(respawns().len(), 1);
            }
            PIKMIN_LINE.set(None);
            m.update(&mut g, 0);
            assert_eq!((g.step, g.failed), (Step::Failed, Some(UNAVAILABLE_NOTICE)));
            assert_eq!(respawns().len(), usize::from(later));
            assert!(requests().is_empty());
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
    }
}
#[test]
fn olimar_existing_ground_readiness_applies_before_and_after_line() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = olimar(&mut m, 0);
        AIR.store(true, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert_eq!(g.step, Step::Settle);
        assert!(respawns().is_empty() && requests().is_empty());
        AIR.store(false, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert_eq!((g.step, respawns().len()), (Step::Entering, 1));
        // Reads count only on released (grounded WAIT, not stopped) frames.
        AIR.store(true, Ordering::Relaxed);
        m.update(&mut g, 0);
        AIR.store(false, Ordering::Relaxed);
        for status in [14, 476] {
            CPU_STATUS.store(status, Ordering::Relaxed);
            m.update(&mut g, 0);
        }
        CPU_STATUS.store(0, Ordering::Relaxed);
        CPU_STOP.store(true, Ordering::Relaxed);
        m.update(&mut g, 0);
        CPU_STOP.store(false, Ordering::Relaxed);
        assert_eq!((g.step, g.followed, LINE_READS.load(Ordering::Relaxed)), (Step::Entering, false, 0));
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        finish_line(&m, &mut g);
        assert_eq!(requests(), vec![(m.p1(), 0), (2, 0), (2, 50)]);
        assert_eq!(respawns().len(), 1);
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
            assert_eq!(respawns(), vec![entry::ONE_PIKMIN]);
            assert!(requests().is_empty());
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
        assert_eq!(g.failed, Some("Move entry unconfirmed · no grade"));
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
        assert_eq!(requests(), vec![(2, 50)]);
        assert!(respawns().is_empty());
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
fn actual_place_uses_measured_flip_spacing_and_preserves_other_plans() {
    for (id, plan, kind, name, evidence, expected_spacing) in [
        (
            108,
            Plan::Flip,
            32,
            "Zero Suit Samus",
            "B11",
            41.123188019_f32,
        ),
        (
            101,
            Plan::DownSmash,
            28,
            "Mr. Game & Watch",
            "B04",
            14.2_f32,
        ),
        (99, Plan::UpSpecial, 42, "King Dedede", "B02", 6.0_f32),
    ] {
        unsafe {
            let mut m = NativeMemory::new();
            let mut g = Game::new(plan, State::Bury);
            let request = g.requested.as_mut().unwrap();
            request.id = id;
            request.attacker_kind = kind;
            request.attacker_name = name;
            request.evidence = evidence;
            m.body[3] = kind;
            g.step = Step::Place;
            CPU_STATUS.store(0, Ordering::Relaxed);

            m.update(&mut g, 0);

            assert_eq!(g.step, Step::Settle);
            assert_eq!(INIT_POS_CALLS.load(Ordering::Relaxed), 1);
            let placed_x = f32::from_bits(INIT_POS_X.load(Ordering::Relaxed));
            assert_eq!(placed_x, expected_spacing);
            if plan == Plan::Flip {
                let displacements = [35.235_488_892_f32, 36.510_887_146_f32];
                assert!(displacements.iter().all(|d| 6.0_f32 - d < 0.0));
                assert!(displacements
                    .iter()
                    .all(|d| (0.0_f32..=10.5_f32).contains(&(placed_x - d))));
            }
        }
    }
}

#[test]
fn actual_place_faces_flip_from_stage_interior_after_p1_turns() {
    // rc15-flip108-e051-ally-1 gen11: P1 at x -40.465 facing -1 put the 41-unit CPU at -81.59,
    // off the stage. Flip now places toward the camera centre (fixture centre 0); Drag stays outward.
    for (plan, kind, name, evidence, expected_x) in [
        (Plan::Flip, 32, "Zero Suit Samus", "B11", -40.465_f32 + 41.123188019_f32),
        (Plan::Drag, 66, "Ridley", "G95", -40.465_f32 - 6.0_f32),
    ] {
        unsafe {
            let mut m = NativeMemory::new();
            let mut g = Game::new(plan, State::Bury);
            let request = g.requested.as_mut().unwrap();
            request.attacker_kind = kind;
            request.attacker_name = name;
            request.evidence = evidence;
            m.body[3] = kind;
            g.step = Step::Place;
            CPU_STATUS.store(0, Ordering::Relaxed);
            P1_X.store((-40.465_f32).to_bits(), Ordering::Relaxed);
            ROLLER_LR.store((-1.0_f32).to_bits(), Ordering::Relaxed);

            m.update(&mut g, 0);

            assert_eq!(g.step, Step::Settle, "{plan:?}");
            assert_eq!(INIT_POS_CALLS.load(Ordering::Relaxed), 1);
            assert_eq!(f32::from_bits(INIT_POS_X.load(Ordering::Relaxed)), expected_x, "{plan:?}");
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
            if let Some(text) = crate::model::mashout::moves::practice_only(id) {
                assert_eq!(g.failed, Some(text));
                assert!(BEGINS.with_borrow(Vec::is_empty));
                continue;
            }
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
                Some(crate::model::mashout::moves::practice_only(id).unwrap_or("Move did not capture · reopen menu to retry"))
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

fn reset_cart() {
    TROLLEY_MISSING_GRANT.store(0,Ordering::Relaxed);
    TROLLEY_NATIVE_CALLS.store(0,Ordering::Relaxed);
    TROLLEY_INT_READS.store(0,Ordering::Relaxed);
    CART_EXISTS.store(true,Ordering::Relaxed);
    CART_ACTIVE.store(true,Ordering::Relaxed);
    CART_ARTICLE.store(0x100,Ordering::Relaxed);
    CART_BOID.store(0x200,Ordering::Relaxed);
    CART_BOMA.store(0x300,Ordering::Relaxed);
    CART_PARENT.store(0x200,Ordering::Relaxed);
    CART_COUNTER.store(0,Ordering::Relaxed);
    CART_GET_COUNT.store(0,Ordering::Relaxed);
    CART_REPLACE_SECOND_READ.store(false,Ordering::Relaxed);
}
unsafe fn trolley(m: &mut NativeMemory) -> Game {
    let mut g=Game::new(Plan::Trolley,State::Grab);
    let r=g.requested.as_mut().unwrap();
    r.id=97; r.attacker_kind=0x58; r.attacker_name="Steve";
    m.body[3]=0x58;
    g.step=Step::Settle;
    CPU_STATUS.store(0,Ordering::Relaxed);
    m.update(&mut g,0);
    assert_eq!(g.step,Step::Entering);
    assert_eq!(REQUESTS.with_borrow(Clone::clone),vec![(2,477)]);
    CPU_STATUS.store(477,Ordering::Relaxed);
    g
}
#[test]
fn trolley_actual_runtime_ride_drive_exact_zero_single_jump_then_native_release() {
    unsafe {
        let mut m=NativeMemory::new(); let mut g=trolley(&mut m);
        let mut inputs=Vec::new();
        for (status,counter) in [(477,0),(519,0),(520,3),(520,1),(520,0),(520,0),(520,0),(521,0)] {
            CPU_STATUS.store(status,Ordering::Relaxed);
            CART_COUNTER.store(counter,Ordering::Relaxed);
            m.update(&mut g,0);
            assert_eq!(g.step,Step::Entering);
            inputs.push(DRIVEN.with_borrow(|v|v.last().unwrap().buttons));
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
        assert_eq!(inputs,vec![0,0,0,0,4,4,4,0]);
        assert_eq!(inputs.windows(2).filter(|v|v[0]==0 && v[1]==4).count(),1);
        assert!(DRIVEN.with_borrow(|v|v.iter().all(|i|i.lstick_x==0 && i.lstick_y==0)));
    }
}
#[test]
fn trolley_each_missing_grant_refuses_before_any_native_read_or_default_zero() {
    unsafe {
        use model::sites_pkg3 as s;
        for grant in [s::STATE_ARTICLE_IS_EXIST_OFFSET,s::STATE_ARTICLE_GET_ARTICLE_OFFSET,
            s::STATE_ARTICLE_OBJECT_ID_OFFSET,s::STATE_OBJECT_IS_ACTIVE_OFFSET,
            s::STATE_OBJECT_MODULE_ACCESSOR_OFFSET,s::STATE_LINK_PARENT_OBJECT_ID_OFFSET,
            model::sites_pkg2::WORK_GET_INT_OFFSET] {
            let _m=NativeMemory::new(); let mut actor=None;
            assert!(trolley_input(2,519,&mut actor).is_ok());
            TROLLEY_NATIVE_CALLS.store(0,Ordering::Relaxed);
            TROLLEY_INT_READS.store(0,Ordering::Relaxed);
            TROLLEY_MISSING_GRANT.store(grant,Ordering::Relaxed);
            let before=actor;
            assert_eq!(trolley_input(2,520,&mut actor),Err(()));
            assert_eq!(actor,before);
            assert_eq!(TROLLEY_NATIVE_CALLS.load(Ordering::Relaxed),0);
            assert_eq!(TROLLEY_INT_READS.load(Ordering::Relaxed),0);
        }
    }
}
#[test]
fn trolley_runtime_replaced_owner_or_cart_identity_and_parent_refuse() {
    unsafe {
        for field in ["cpu","article","boid","boma","parent","midread","inactive","nullarticle","nullboma"] {
            let _m=NativeMemory::new(); let mut actor=None;
            trolley_input(2,519,&mut actor).unwrap();
            CART_COUNTER.store(2,Ordering::Relaxed);
            trolley_input(2,520,&mut actor).unwrap();
            let before=actor;
            match field {
                "article"=>CART_ARTICLE.store(0x101,Ordering::Relaxed),
                "boid"=>{CART_BOID.store(0x201,Ordering::Relaxed);CART_PARENT.store(0x201,Ordering::Relaxed);},
                "boma"=>CART_BOMA.store(0x301,Ordering::Relaxed),
                "parent"=>CART_PARENT.store(0x201,Ordering::Relaxed),
                "midread"=>CART_REPLACE_SECOND_READ.store(true,Ordering::Relaxed),
                "inactive"=>CART_ACTIVE.store(false,Ordering::Relaxed),
                "nullarticle"=>CART_ARTICLE.store(0,Ordering::Relaxed),
                "nullboma"=>CART_BOMA.store(0,Ordering::Relaxed),
                _=>(),
            }
            CART_COUNTER.store(0,Ordering::Relaxed);
            assert_eq!(trolley_input(if field=="cpu"{3}else{2},520,&mut actor),Err(()),"{field}");
            assert_eq!(actor,before,"{field}");
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
    }
}
#[test]
fn trolley_runtime_failed_missing_cart_and_unobserved_ride_cannot_begin() {
    unsafe {
        for case in ["failed","missing","no-ride"] {
            let mut m=NativeMemory::new(); let mut g=trolley(&mut m);
            if case!="no-ride" { CPU_STATUS.store(519,Ordering::Relaxed);m.update(&mut g,0); }
            if case=="missing" { CART_EXISTS.store(false,Ordering::Relaxed); }
            CPU_STATUS.store(if case=="failed"{518}else{520},Ordering::Relaxed);
            m.update(&mut g,0);
            assert_eq!(g.failed,Some(UNAVAILABLE_NOTICE),"{case}");
            assert_eq!(ARMED.load(Ordering::Relaxed),0);
            assert!(BEGINS.with_borrow(Vec::is_empty));
            assert_eq!(DRIVEN.with_borrow(|v|*v.last().unwrap()),MappedInputs::NEUTRAL);
        }
    }
}
#[test]
fn trolley_runtime_wait_still_fails_after_native_entry_without_capture() {
    unsafe {
        let mut m=NativeMemory::new();let mut g=trolley(&mut m);
        CPU_STATUS.store(519,Ordering::Relaxed);m.update(&mut g,0);
        CPU_STATUS.store(520,Ordering::Relaxed);m.update(&mut g,0);
        CPU_STATUS.store(521,Ordering::Relaxed);m.update(&mut g,0);
        CPU_STATUS.store(0,Ordering::Relaxed);m.update(&mut g,0);
        assert_eq!(g.failed,Some("Move did not capture · reopen menu to retry"));
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}
#[test]
fn trolley_actual_victim446_and_positive_native_seed_are_both_required() {
    unsafe {
        for (victim,remaining,accepted) in [(446,75.0,true),(446,0.0,false),(66,75.0,false),(0,75.0,false)] {
            let mut m=NativeMemory::new();let mut g=trolley(&mut m);
            CPU_STATUS.store(519,Ordering::Relaxed);m.update(&mut g,0);
            CPU_STATUS.store(520,Ordering::Relaxed);m.update(&mut g,0);
            CPU_STATUS.store(521,Ordering::Relaxed);m.update(&mut g,0);
            m.seed(remaining,9.0,1);
            let before=(m.control,m.record);
            let actual=reader::sample(m.p1(),0.37).map(|s|s.seed());
            m.start();m.update(&mut g,victim);
            assert_eq!(BEGINS.with_borrow(Vec::len),usize::from(accepted), "victim={victim} remaining={remaining}");
            assert_eq!((m.control,m.record),before);
            if accepted {
                assert_eq!(g.step,Step::Held);
                assert_eq!(BEGINS.with_borrow(|v|v[0].1),actual.unwrap());
                assert_eq!(BEGINS.with_borrow(|v|v[0].0.0),97);
            } else { assert!(g.clock.is_none()); }
            assert_eq!(DRIVEN.with_borrow(|v|*v.last().unwrap()),MappedInputs::NEUTRAL);
        }
    }
}

// Cargo89 (Plan::Cargo): CATCH 50, CATCH_WAIT 52 (fixture ids), Donkey SHOULDER_START 0x1EA /
// SHOULDER_WAIT 0x1EB (lua_const.rs:4055/4057); victim SHOULDERED_DONKEY_START 0xC5 / SHOULDERED_DONKEY 0xC6.
unsafe fn cargo(m: &mut NativeMemory, facing: f32) -> Game {
    let mut g = Game::new(Plan::Cargo, State::Grab);
    let request = g.requested.as_mut().unwrap();
    request.id = 89;
    request.attacker_kind = 1;
    m.body[3] = 1;
    assert_eq!(entry::start(g.requested.unwrap()), Some("FIGHTER_STATUS_KIND_CATCH"));
    startup(&mut g, m.p1());
    assert_eq!(ARMED.load(Ordering::Relaxed), 0);
    ROLLER_LR.store(facing.to_bits(), Ordering::Relaxed);
    g
}
fn cargo_forward(facing: f32) -> MappedInputs {
    MappedInputs { lstick_x: if facing > 0.0 { 120 } else { -120 }, ..MappedInputs::NEUTRAL }
}
fn shoulder_seed(m: &mut NativeMemory, automatic: f32) {
    m.seed(119.5, 10.0, 1);
    unsafe { (m.record.as_mut_ptr().cast::<u8>().add(4) as *mut f32).write(automatic) };
}
#[test]
fn cargo_real_catch_then_forward_only_in_catch_wait_and_no_shoulder_seed_begins() {
    unsafe {
        for facing in [-1.0f32, 1.0] {
            let mut m = NativeMemory::new();
            let mut g = cargo(&mut m, facing);
            CPU_STATUS.store(50, Ordering::Relaxed);
            m.update(&mut g, 0);
            assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);
            // The preparatory grab's own start is never armed or scored.
            m.seed(90.0, 8.0, 1);
            m.start();
            m.update(&mut g, 66);
            assert!(g.clock.is_none() && BEGINS.with_borrow(Vec::is_empty));
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
            CPU_STATUS.store(52, Ordering::Relaxed);
            for _ in 0..2 {
                m.update(&mut g, 66);
                assert_eq!(ARMED.load(Ordering::Relaxed), g.generation);
                assert!(g.followed);
                assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), cargo_forward(facing));
            }
            for attacker in [0x1EA, 0x1EB] {
                CPU_STATUS.store(attacker, Ordering::Relaxed);
                m.update(&mut g, 0xC5);
                assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);
                assert_eq!(g.step, Step::Entering);
            }
            // Even a non-zero automatic shoulder seed is refused before Begin until a Cargo
            // grading rule is proven and reviewed (OPUS-REVIEW-84e9b33b §1).
            shoulder_seed(&mut m, 1.0);
            let before = (m.control, m.record);
            m.start();
            m.update(&mut g, 0xC6);
            assert_eq!(g.failed, Some("Native tick observation unavailable for this hold"));
            assert!(g.clock.is_none() && BEGINS.with_borrow(Vec::is_empty));
            assert_eq!((m.control, m.record), before);
            assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);
        }
    }
}
#[test]
fn cargo_zero_automatic_shoulder_seed_keeps_the_existing_observation_refusal() {
    unsafe {
        for victim in [0xC5, 0xC6] {
            let mut m = NativeMemory::new();
            let mut g = cargo(&mut m, 1.0);
            CPU_STATUS.store(52, Ordering::Relaxed);
            m.update(&mut g, 66);
            CPU_STATUS.store(0x1EA, Ordering::Relaxed);
            m.update(&mut g, victim);
            shoulder_seed(&mut m, 0.0);
            m.start();
            m.update(&mut g, victim);
            assert_eq!(g.failed, Some("Native tick observation unavailable for this hold"));
            assert!(g.clock.is_none() && BEGINS.with_borrow(Vec::is_empty));
        }
    }
}
#[test]
fn cargo_capture_seed_miss_and_unusable_facing_fail_closed() {
    unsafe {
        // An armed seed while the victim is still in an ordinary capture status is not Cargo.
        for victim in [66, 67, 0] {
            let mut m = NativeMemory::new();
            let mut g = cargo(&mut m, 1.0);
            CPU_STATUS.store(52, Ordering::Relaxed);
            m.update(&mut g, 66);
            shoulder_seed(&mut m, 1.0);
            m.start();
            m.update(&mut g, victim);
            assert_eq!(g.failed, Some("Move entry unconfirmed · no grade"), "victim={victim}");
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
        let mut m = NativeMemory::new();
        let mut g = cargo(&mut m, 1.0);
        CPU_STATUS.store(50, Ordering::Relaxed);
        m.update(&mut g, 0);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut g, 0);
        assert_eq!(g.failed, Some("Move did not capture · reopen menu to retry"));
        for facing in [0.0f32, f32::NAN, f32::INFINITY] {
            let mut m = NativeMemory::new();
            let mut g = cargo(&mut m, facing);
            CPU_STATUS.store(52, Ordering::Relaxed);
            m.update(&mut g, 66);
            assert_eq!(g.failed, Some(UNAVAILABLE_NOTICE), "facing={facing}");
            assert!(DRIVEN.with_borrow(|v| v.iter().all(|i| i.lstick_x == 0)));
        }
    }
}
#[test]
fn cargo_never_drives_forward_before_catch_wait_or_for_other_plans() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = cargo(&mut m, 1.0);
        for attacker in [0, 50, 51, 53] {
            CPU_STATUS.store(attacker, Ordering::Relaxed);
            m.update(&mut g, 66);
            if g.step == Step::Failed { break; }
            assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL, "attacker={attacker}");
        }
        let mut m = NativeMemory::new();
        let mut g = held_downthrow(&mut m);
        assert_eq!(g.step, Step::Held);
        g.step = Step::Entering;
        CPU_STATUS.store(52, Ordering::Relaxed);
        m.update(&mut g, 66);
        assert!(DRIVEN.with_borrow(|v| v.iter().all(|i| i.lstick_x == 0)));
    }
}

// Move90 Ganondorf aerial side special: victim 0xBF/0xC0 hold, landing pair victim 0xC1 + attacker 0x1E5.
unsafe fn held_ganon(m: &mut NativeMemory, plan: Plan) -> Game {
    EXECUTE_HELD.store(true, Ordering::Relaxed);
    let mut g = Game::new(plan, State::Grab);
    let r = g.requested.as_mut().unwrap();
    r.id = 90;
    r.attacker_kind = 0x18;
    m.body[3] = 0x18;
    startup(&mut g, m.p1());
    assert_eq!(ARMED.load(Ordering::Relaxed), g.generation);
    // An ordinary grab seed is only admitted from the attacker's catch statuses (fixture 52).
    CPU_STATUS.store(if plan == Plan::Grab { 52 } else { 0x1E3 }, Ordering::Relaxed);
    m.seed(100.0, 4.0, 1);
    m.start();
    m.update(&mut g, 0xBF);
    assert_eq!(g.step, Step::Held);
    CPU_STATUS.store(0x1E4, Ordering::Relaxed);
    m.update(&mut g, 0xC0);
    assert_eq!(g.step, Step::Held, "the record carries from 191 into 192");
    g
}
#[test]
fn ganon_landing_pair_is_the_existing_native_end_outcome() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut g = held_ganon(&mut m, Plan::AirSideSpecial);
        CPU_STATUS.store(0x1E5, Ordering::Relaxed);
        m.update(&mut g, 0xC1);
        assert_eq!(g.step, Step::Result);
        assert_eq!(g.failed, None);
        assert!(g.clock.is_none());
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        assert_eq!(NATIVE_ENDS.with_borrow(Clone::clone), vec![(g.generation, g.epoch)]);
        assert_eq!(BEGINS.with_borrow(Vec::len), 1);
    }
}
#[test]
fn ganon_other_unlinked_statuses_and_plans_stay_interruptions() {
    unsafe {
        for (plan, victim, attacker) in [
            (Plan::AirSideSpecial, 0xC1, 0x1E4), // victim ended while the attacker still falls
            (Plan::AirSideSpecial, 0xC1, 0),
            (Plan::AirSideSpecial, 79, 0x1E5),   // DAMAGE_FALL is not the landing pair
            (Plan::AirSideSpecial, 0, 0x1E5),
            (Plan::Grab, 0xC1, 0x1E5),           // only move90's plan owns the pair
            (Plan::Drag, 0xC1, 0x1E5),
        ] {
            let mut m = NativeMemory::new();
            let mut g = held_ganon(&mut m, plan);
            CPU_STATUS.store(attacker, Ordering::Relaxed);
            m.update(&mut g, victim);
            assert_eq!(g.failed, Some("Capture interrupted · no grade"), "{plan:?} {victim:#x} {attacker:#x}");
            assert!(NATIVE_ENDS.with_borrow(Vec::is_empty));
        }
    }
}
#[test]
fn ganon_escape_status_keeps_precedence_over_the_landing_pair() {
    unsafe {
        // An escape status in the same frame as the landing pair's attacker status.
        for escape in [68, 69] {
            let mut m = NativeMemory::new();
            let mut g = held_ganon(&mut m, Plan::AirSideSpecial);
            CPU_STATUS.store(0x1E5, Ordering::Relaxed);
            m.update(&mut g, escape);
            assert_ne!(g.failed, Some("Capture interrupted · no grade"));
            assert!(NATIVE_ENDS.with_borrow(Vec::is_empty), "escape={escape}");
        }
    }
}

fn hero_read(
    cpu: usize,
    status: entry::HeroStatus,
    index: Option<i32>,
    commands: Option<[i32; 4]>,
    success_sp: Option<bool>,
    active_command: Option<i32>,
) -> entry::HeroRead {
    entry::HeroRead {
        readers_ready: true,
        cpu,
        status,
        ice_sword_command: 17,
        window_state: Some(3),
        index,
        commands,
        success_sp,
        active_command,
    }
}

#[test]
fn hero_actor_navigates_all_four_slots_with_down_ack_neutral_and_one_confirm() {
    for target in 0..4 {
        for start in 0usize..4 {
            let mut commands = [2, 5, 9, 14];
            commands[target] = 17;
            let mut actor = entry::HeroEntry::new(2).unwrap();
            let (first, _) = actor.observe(hero_read(
                2, entry::HeroStatus::Select, Some(start as i32), Some(commands), None, None,
            )).unwrap();
            let mut downs = 0;
            let mut index = start;
            if start == target {
                assert_eq!((first.buttons, first.lstick_y), (0x2, 0));
            } else {
                assert_eq!((first.buttons, first.lstick_y), (0, -120));
                downs += 1;
                loop {
                    let (neutral, _) = actor.observe(hero_read(
                        2, entry::HeroStatus::Select, Some(index as i32), Some(commands), None, None,
                    )).unwrap();
                    assert_eq!(neutral, MappedInputs::NEUTRAL, "old index waits for native ack");
                    index = (index + 1) % 4;
                    let (neutral, _) = actor.observe(hero_read(
                        2, entry::HeroStatus::Select, Some(index as i32), Some(commands), None, None,
                    )).unwrap();
                    assert_eq!(neutral, MappedInputs::NEUTRAL, "fresh down edge is released");
                    if index == target {
                        let (confirm, _) = actor.observe(hero_read(
                            2, entry::HeroStatus::Select, Some(index as i32), Some(commands), None, None,
                        )).unwrap();
                        assert_eq!((confirm.buttons, confirm.lstick_y), (0x2, 0));
                        break;
                    }
                    let (down, _) = actor.observe(hero_read(
                        2, entry::HeroStatus::Select, Some(index as i32), Some(commands), None, None,
                    )).unwrap();
                    assert_eq!((down.buttons, down.lstick_y), (0, -120));
                    downs += 1;
                }
            }
            assert_eq!(downs, (target + 4 - start) % 4);
            let (neutral, outcome) = actor.observe(hero_read(
                2, entry::HeroStatus::Start, None, None, Some(true), Some(17),
            )).unwrap();
            assert_eq!(neutral, MappedInputs::NEUTRAL);
            assert_eq!(outcome, entry::HeroOutcome::Confirmed);
            assert!(actor.confirmed());
            let (neutral, outcome) = actor.observe(hero_read(
                2, entry::HeroStatus::Other, None, None, None, None,
            )).unwrap();
            assert_eq!(neutral, MappedInputs::NEUTRAL);
            assert_eq!(outcome, entry::HeroOutcome::Confirmed);
        }
    }
}

#[test]
fn hero_actor_refuses_invalid_menu_and_requires_native_success_and_command17() {
    let mut actor = entry::HeroEntry::new(2).unwrap();
    let mut invalid_window = hero_read(
        2, entry::HeroStatus::Select, None, None, None, None,
    );
    invalid_window.window_state = Some(-99);
    assert_eq!(actor.observe(invalid_window), Err(entry::HeroError::Unavailable));

    let invalid = hero_read(
        2, entry::HeroStatus::Select, Some(0), Some([1, 2, 21, 4]), None, None,
    );
    assert_eq!(actor.observe(invalid), Err(entry::HeroError::Unavailable));

    for (success_sp, active) in [(false, 17), (true, 18)] {
        let mut actor = entry::HeroEntry::new(2).unwrap();
        let (special, _) = actor.observe(hero_read(
            2, entry::HeroStatus::Select, Some(1), Some([4, 17, 8, 12]), None, None,
        )).unwrap();
        assert_eq!(special.buttons, 0x2);
        let (neutral, outcome) = actor.observe(hero_read(
            2, entry::HeroStatus::Start, None, None, Some(success_sp), Some(active),
        )).unwrap();
        assert_eq!(neutral, MappedInputs::NEUTRAL);
        assert_eq!(outcome, entry::HeroOutcome::Rejected);
        assert!(!actor.confirmed());
    }
}

unsafe fn hero(m: &mut NativeMemory) -> Game {
    let mut game = Game::new(Plan::Hero, State::Frozen);
    let request = game.requested.as_mut().unwrap();
    request.id = 110;
    request.attacker_kind = 0x53;
    request.attacker_name = "Hero";
    request.name = "Command18";
    request.evidence = "F01";
    m.body[3] = 0x53;
    game.step = Step::Settle;
    CPU_STATUS.store(0, Ordering::Relaxed);
    m.update(&mut game, 0);
    assert_eq!(game.step, Step::Entering);
    assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 479)]);
    CPU_STATUS.store(479, Ordering::Relaxed);
    m.update(&mut game, 0);
    game
}

fn hero_menu(index: u32, commands: [u32; 4]) {
    HERO_WINDOW_STATE.store(3, Ordering::Relaxed);
    HERO_INDEX.store(index, Ordering::Relaxed);
    for (slot, value) in HERO_COMMANDS.iter().zip(commands) {
        slot.store(value, Ordering::Relaxed);
    }
}

#[test]
fn hero_runtime_reads_only_open3_mirrors_and_accepts_consumed_c9_active17_success_seed() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut game = hero(&mut m);
        assert_eq!(HERO_INT_READS.load(Ordering::Relaxed), 0);
        CPU_STATUS.store(493, Ordering::Relaxed);
        HERO_WINDOW_STATE.store(2, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(HERO_INT_READS.load(Ordering::Relaxed), 1, "only WINDOW_STATE before OPEN3");
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);

        hero_menu(3, [17, 5, 9, 14]);
        m.update(&mut game, 0);
        assert_eq!(HERO_INT_READS.load(Ordering::Relaxed), 7, "window, index, and all four native mirrors");
        let down = DRIVEN.with_borrow(|v| *v.last().unwrap());
        assert_eq!((down.buttons, down.lstick_y), (0, -120));
        HERO_INDEX.store(0, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);
        m.update(&mut game, 0);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs { buttons: 2, ..MappedInputs::NEUTRAL });
        assert_eq!(DRIVEN.with_borrow(|v| v.iter().filter(|i| i.buttons == 2).count()), 1);

        let reads_before_closed_window = HERO_INT_READS.load(Ordering::Relaxed);
        HERO_WINDOW_STATE.store(4, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(HERO_INT_READS.load(Ordering::Relaxed), reads_before_closed_window + 1,
            "closed window reads only WINDOW_STATE, not index or command mirrors");
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);
        assert!(!game.hero.unwrap().confirmed(), "closed menu is not native confirmation");

        HERO_SUCCESS_SP.store(true, Ordering::Relaxed);
        // Native START copied command17 to ACTIVE, then consumed pending C9.
        HERO_DECIDE.store(u32::MAX, Ordering::Relaxed);
        HERO_ACTIVE.store(17, Ordering::Relaxed);
        CPU_STATUS.store(497, Ordering::Relaxed);
        m.seed(73.25, 1.75, 2);
        m.start();
        m.update(&mut game, 203);
        assert_eq!(game.step, Step::Held);
        assert!(game.followed);
        assert!(game.hero.is_some_and(|actor| actor.confirmed()));
        assert_eq!(HERO_FLAG_READS.load(Ordering::Relaxed), 1);
        assert_eq!(BEGINS.with_borrow(|v| v.len()), 1);
        assert_eq!(BEGINS.with_borrow(|v| v[0].0.0), 110);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);
    }
}

#[test]
fn hero_closed_confirm_window_does_not_accept_premature_seed() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut game = hero(&mut m);
        CPU_STATUS.store(493, Ordering::Relaxed);
        hero_menu(1, [4, 17, 8, 12]);
        m.update(&mut game, 0);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()).buttons, 2);

        let reads_before_closed_window = HERO_INT_READS.load(Ordering::Relaxed);
        HERO_WINDOW_STATE.store(4, Ordering::Relaxed);
        HERO_SUCCESS_SP.store(true, Ordering::Relaxed);
        HERO_ACTIVE.store(17, Ordering::Relaxed);
        HERO_DECIDE.store(u32::MAX, Ordering::Relaxed);
        m.seed(73.25, 1.75, 2);
        m.start();
        m.update(&mut game, 203);
        assert_eq!(HERO_INT_READS.load(Ordering::Relaxed), reads_before_closed_window + 1,
            "closed confirmation window reads no index or command mirrors");
        assert_eq!(game.failed, Some("Move entry unconfirmed · no grade"));
        assert!(game.hero.is_some_and(|actor| !actor.confirmed()));
        assert!(QUEUE.with_borrow(Vec::is_empty));
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}

#[test]
fn hero_covered_edge_neutralizes_without_advancing_and_retries_same_edge() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut game = hero(&mut m);
        CPU_STATUS.store(493, Ordering::Relaxed);
        hero_menu(3, [17, 5, 9, 14]);
        DRIVEN.with_borrow_mut(|v| v.push(MappedInputs { buttons: 2, ..MappedInputs::NEUTRAL }));

        COVERED.store(true, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL,
            "covered proposed Down edge must release the prior CPU input");
        assert!(game.hero.is_some(), "the previously committed actor remains available");
        assert_eq!(game.failed, None);

        COVERED.store(false, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs { lstick_y: -120, ..MappedInputs::NEUTRAL },
            "same native menu retries the first Down edge, proving blocked observation did not advance the actor");
        assert!(game.hero.is_some());
    }
}

#[test]
fn hero_covered_edge_still_drains_and_refuses_unconfirmed_seed() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut game = hero(&mut m);
        CPU_STATUS.store(493, Ordering::Relaxed);
        hero_menu(3, [17, 5, 9, 14]);
        COVERED.store(true, Ordering::Relaxed);
        m.seed(73.25, 1.75, 2);
        m.start();
        assert_eq!(QUEUE.with_borrow(Vec::len), 1);

        m.update(&mut game, 203);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);
        assert!(game.hero.is_some_and(|actor| !actor.confirmed()), "unissued edge cannot confirm the actor");
        assert_eq!(game.failed, Some("Move entry unconfirmed · no grade"),
            "covered admission must continue into the existing seed refusal");
        assert!(QUEUE.with_borrow(Vec::is_empty), "queued seed was drained");
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}

#[test]
fn hero_absent_command18_is_not_icesword_guard_cancel_ack_retries_selected110() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut game = hero(&mut m);
        CPU_STATUS.store(493, Ordering::Relaxed);
        hero_menu(3, [18, 5, 9, 14]);
        m.update(&mut game, 0);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs { buttons: 0x8, ..MappedInputs::NEUTRAL });
        assert_eq!(game.step, Step::Entering);
        let reads_before_closed_window = HERO_INT_READS.load(Ordering::Relaxed);
        HERO_WINDOW_STATE.store(1, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(HERO_INT_READS.load(Ordering::Relaxed), reads_before_closed_window + 1,
            "CancelPending CLOSING reads no menu mirrors");
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);
        assert_eq!(game.step, Step::Entering, "closed window is not cancel acknowledgment");
        assert_eq!(game.failed, None);

        CPU_STATUS.store(494, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Retry);
        assert_eq!(game.failed, None);
        assert_eq!(game.requested.unwrap().id, 110);
        assert_eq!(BEGINS.with_borrow(Vec::len), 0);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Reload);
        assert_eq!(game.requested.unwrap().id, 110);
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 479)]);
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}

#[test]
fn hero_native_failure_wrong_body_and_missing_work_grants_never_begin() {
    unsafe {
        for (case, active, success, status) in [
            ("native-failure", 17, true, 496u32),
            ("wrong-active", 18, true, 497),
            ("missing-active", u32::MAX, true, 497),
            ("no-success-sp", 17, false, 497),
        ] {
            let mut m = NativeMemory::new();
            let mut game = hero(&mut m);
            CPU_STATUS.store(493, Ordering::Relaxed);
            hero_menu(1, [4, 17, 8, 12]);
            m.update(&mut game, 0);
            // A stale pending C9=17 must not substitute for the active START command.
            HERO_DECIDE.store(17, Ordering::Relaxed);
            HERO_ACTIVE.store(active, Ordering::Relaxed);
            HERO_SUCCESS_SP.store(success, Ordering::Relaxed);
            CPU_STATUS.store(status, Ordering::Relaxed);
            m.seed(73.25, 1.75, 2);
            m.start();
            m.update(&mut game, 203);
            assert_eq!(game.failed, Some("Move entry unconfirmed · no grade"), "{case}");
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }

        let mut m = NativeMemory::new();
        let mut game = hero(&mut m);
        CPU_STATUS.store(493, Ordering::Relaxed);
        hero_menu(1, [4, 17, 8, 12]);
        WORK_GET_INT_GRANTED.store(false, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.failed, Some(UNAVAILABLE_NOTICE));
        assert_eq!(HERO_INT_READS.load(Ordering::Relaxed), 0);
        assert!(BEGINS.with_borrow(Vec::is_empty));

        let mut m = NativeMemory::new();
        let mut game = hero(&mut m);
        CPU_STATUS.store(493, Ordering::Relaxed);
        hero_menu(1, [4, 17, 8, 12]);
        m.update(&mut game, 0);
        WORK_IS_FLAG_GRANTED.store(false, Ordering::Relaxed);
        HERO_SUCCESS_SP.store(true, Ordering::Relaxed);
        CPU_STATUS.store(497, Ordering::Relaxed);
        m.seed(73.25, 1.75, 2);
        m.start();
        m.update(&mut game, 203);
        assert_eq!(game.failed, Some(UNAVAILABLE_NOTICE));
        assert_eq!(HERO_FLAG_READS.load(Ordering::Relaxed), 0);
        assert!(BEGINS.with_borrow(Vec::is_empty));

        let mut m = NativeMemory::new();
        let mut game = hero(&mut m);
        m.body[3] = 0x1c;
        CPU_STATUS.store(493, Ordering::Relaxed);
        hero_menu(1, [4, 17, 8, 12]);
        m.update(&mut game, 0);
        assert_eq!(game.failed, Some("Attacker changed · no grade"));
        assert_eq!(HERO_INT_READS.load(Ordering::Relaxed), 0);
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}


#[test]
fn hero_start_missing_getter_or_changed_body_cannot_admit_native_ice() {
    unsafe {
        for changed_body in [false, true] {
            let mut m = NativeMemory::new();
            let mut game = hero(&mut m);
            CPU_STATUS.store(493, Ordering::Relaxed);
            hero_menu(1, [4, 17, 8, 12]);
            m.update(&mut game, 0);
            HERO_SUCCESS_SP.store(true, Ordering::Relaxed);
            HERO_ACTIVE.store(17, Ordering::Relaxed);
            HERO_DECIDE.store(u32::MAX, Ordering::Relaxed);
            CPU_STATUS.store(497, Ordering::Relaxed);
            if changed_body {
                m.body[3] = 0x1c;
            } else {
                WORK_GET_INT_GRANTED.store(false, Ordering::Relaxed);
            }
            let reads_before = HERO_INT_READS.load(Ordering::Relaxed);
            m.seed(73.25, 1.75, 2);
            m.start();
            m.update(&mut game, 203);
            assert_eq!(game.failed, Some(if changed_body {
                "Attacker changed · no grade"
            } else { UNAVAILABLE_NOTICE }));
            assert_eq!(HERO_INT_READS.load(Ordering::Relaxed), reads_before);
            assert_eq!(HERO_FLAG_READS.load(Ordering::Relaxed), 0);
            assert!(game.hero.is_some_and(|actor| !actor.confirmed()));
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
    }
}

#[test]
fn sora_ice_magic_uses_the_generic_native_special_n_dispatch() {
    let scenario = Scenario {
        id: 116,
        state: State::Frozen,
        attacker_kind: 0x5d,
        attacker_name: "Sora",
        name: "Ice magic",
        plan: Plan::IceMagic,
        grab: 0,
        evidence: "F07",
    };
    assert_eq!(
        entry::start(scenario),
        Some("FIGHTER_STATUS_KIND_SPECIAL_N")
    );
}

fn sora_game(kind: u32) -> Game {
    TRAIL_MAGIC_KIND_VALUE.store(kind, Ordering::Relaxed);
    let mut game = Game::new(Plan::IceMagic, State::Frozen);
    game.requested = Some(Scenario {
        id: 116,
        state: State::Frozen,
        attacker_kind: 0x5d,
        attacker_name: "Sora",
        name: "Ice magic",
        plan: Plan::IceMagic,
        grab: 0,
        evidence: "F07",
    });
    game.step = Step::Settle;
    CPU_STATUS.store(0, Ordering::Relaxed);
    game
}

#[test]
fn sora_blizzard_starts_real_special_n_and_only_actual_ice_seed_begins() {
    unsafe {
        let mut m = NativeMemory::new();
        m.body[3] = 0x5d;
        let mut game = sora_game(1);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Entering);
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476)]);
        assert_eq!(ARMED.load(Ordering::Relaxed), 5);
        assert_eq!(game.trail_magic, None, "Blizzard starts directly without preparatory casts");
        assert_eq!(DRIVEN.with_borrow(|v| v.last().unwrap().buttons), 2);

        CPU_STATUS.store(476, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL,
            "the generic Special N edge is not repeated while entering");
        assert_eq!(REQUESTS.with_borrow(Vec::len), 1);

        CPU_STATUS.store(486, Ordering::Relaxed);
        m.seed(142.3, 7.0, 1);
        m.start();
        m.update(&mut game, 203);
        // restriction: mashout-blizzard-practice: 116 ends at the seed with the practice notice, no Begin.
        if let Some(text) = crate::model::mashout::moves::practice_only(116) {
            assert_eq!(game.failed, Some(text));
            assert!(BEGINS.with_borrow(Vec::is_empty));
            return;
        }
        assert_eq!(game.step, Step::Held);
        assert_eq!(BEGINS.with_borrow(Vec::len), 1);
        assert!(game.clock.is_some());
    }
}

#[test]
fn sora_fire_observes_n1_cycle_then_thunder_and_blizzard_before_arming() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut game = sora_game(0);
        COVERED.store(true, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Settle);
        assert!(REQUESTS.with_borrow(Vec::is_empty));
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);

        COVERED.store(false, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Entering);
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476)]);
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        assert_eq!(DRIVEN.with_borrow(|v| v.last().unwrap().buttons), 2);

        SEEDS.post(NativeSeed {
            generation: 5,
            epoch: 0,
            origin: SeedOrigin::Export,
            sample: None,
        });
        CPU_STATUS.store(476, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.failed, None, "preparatory seed is discarded before seed validation");
        assert!(game.clock.is_none());
        assert!(BEGINS.with_borrow(Vec::is_empty));
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        assert_eq!(DRIVEN.with_borrow(|v| *v.last().unwrap()), MappedInputs::NEUTRAL);

        for status in [483, 483, 484, 484] {
            CPU_STATUS.store(status, Ordering::Relaxed);
            m.update(&mut game, 0);
            assert_eq!(game.failed, None);
            assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        }
        TRAIL_MAGIC_KIND_VALUE.store(2, Ordering::Relaxed);
        CPU_STATUS.store(485, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(trail_magic_kind(2), Ok(entry::TrailMagicKind::Thunder));
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Settle, "native N1 end and fresh Thunder readback are required: {:?}", game.failed);
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476)]);
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        assert!(BEGINS.with_borrow(Vec::is_empty));

        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Entering);
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476), (2, 476)]);
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        CPU_STATUS.store(476, Ordering::Relaxed);
        m.update(&mut game, 0);
        CPU_STATUS.store(487, Ordering::Relaxed);
        m.update(&mut game, 0);
        CPU_STATUS.store(487, Ordering::Relaxed);
        m.update(&mut game, 0);
        TRAIL_MAGIC_KIND_VALUE.store(1, Ordering::Relaxed);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Place, "real N3 completion and Blizzard kind are required");
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        assert!(BEGINS.with_borrow(Vec::is_empty));

        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Settle);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Entering);
        assert_eq!(game.trail_magic, None);
        // Place requests WAIT on P1 and on the prepared CPU before the actual Blizzard cast.
        assert_eq!(
            REQUESTS.with_borrow(Clone::clone),
            vec![(2, 476), (2, 476), (m.p1(), 0), (2, 0), (2, 476)]
        );
        assert_eq!(ARMED.load(Ordering::Relaxed), 5, "arming begins only for the actual Blizzard cast");
        assert!(BEGINS.with_borrow(Vec::is_empty));
    }
}

#[test]
fn sora_thunder_prepares_once_and_unseen_or_unresolved_magic_fails_closed() {
    unsafe {
        let mut m = NativeMemory::new();
        let mut game = sora_game(2);
        m.update(&mut game, 0);
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476)]);
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        CPU_STATUS.store(476, Ordering::Relaxed);
        m.update(&mut game, 0);
        CPU_STATUS.store(487, Ordering::Relaxed);
        m.update(&mut game, 0);
        CPU_STATUS.store(0, Ordering::Relaxed);
        assert_eq!(ids()[0], 0);
        assert_eq!(natives::status_kind(2), 0);
        assert_eq!(natives::situation_kind(m.p1()), ids()[7]);
        assert_eq!(natives::situation_kind(2), ids()[7]);
        assert!(!natives::is_stop(m.p1()) && !natives::is_stop(2));
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Entering, "Thunder cannot finish without fresh Blizzard kind: {:?}", game.failed);
        assert_eq!(REQUESTS.with_borrow(Vec::len), 1, "no repeated Special N edge while waiting");
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
        TRAIL_MAGIC_KIND_VALUE.store(1, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Place);

        for case in ["invalid-kind", "missing-grant", "unresolved-work-id"] {
            let mut m = NativeMemory::new();
            let kind = if case == "invalid-kind" { 9 } else { 0 };
            let mut game = sora_game(kind);
            if case == "missing-grant" {
                WORK_GET_INT_GRANTED.store(false, Ordering::Relaxed);
            } else if case == "unresolved-work-id" {
                TRAIL_MAGIC_KIND_RESOLVED.store(false, Ordering::Relaxed);
            }
            m.update(&mut game, 0);
            assert_eq!(game.failed, Some(UNAVAILABLE_NOTICE), "{case}");
            assert_eq!(game.step, Step::Failed, "{case}");
            assert_eq!(ARMED.load(Ordering::Relaxed), 0, "{case}");
            assert!(REQUESTS.with_borrow(Vec::is_empty), "{case}");
            if case != "invalid-kind" {
                assert_eq!(TRAIL_MAGIC_READS.load(Ordering::Relaxed), 0, "{case}");
            }
        }

        let mut m = NativeMemory::new();
        let mut game = sora_game(0);
        m.update(&mut game, 0);
        TRAIL_MAGIC_KIND_VALUE.store(2, Ordering::Relaxed);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.failed, Some(UNAVAILABLE_NOTICE), "magic change without N1 phases is refused");
        assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476)]);
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);

        let mut m = NativeMemory::new();
        let mut game = sora_game(0);
        m.update(&mut game, 0);
        actual_body_guard(&mut game, m.p1(), 3);
        assert_eq!(game.failed, Some("Fighter changed · no grade"));
        assert_eq!(ARMED.load(Ordering::Relaxed), 0);
    }
}

// Sora116 F2 (b)+(a): drive a Thunder preparation to its Place, then the Blizzard Settle.
unsafe fn sora_thunder_to_blizzard_settle(m: &NativeMemory, game: &mut Game) {
    m.update(game, 0);
    for status in [476, 487] {
        CPU_STATUS.store(status, Ordering::Relaxed);
        m.update(game, 0);
    }
    CPU_STATUS.store(0, Ordering::Relaxed);
    TRAIL_MAGIC_KIND_VALUE.store(1, Ordering::Relaxed);
    m.update(game, 0);
    assert_eq!(game.step, Step::Place, "{:?}", game.failed);
    m.update(game, 0);
    assert_eq!(game.step, Step::Settle);
}
#[test]
fn sora_prep_casts_face_away_then_blizzard_arms_only_with_unchanged_damage_and_no_article() {
    unsafe {
        for (cpu_x, away) in [(6.0f32, 1.0f32), (-6.0, -1.0)] {
            let mut m = NativeMemory::new();
            CPU_X.store(cpu_x.to_bits(), Ordering::Relaxed);
            let mut game = sora_game(2);
            m.update(&mut game, 0);
            assert_eq!(REQUESTS.with_borrow(Clone::clone), vec![(2, 476)]);
            assert_eq!(LR_SETS.with_borrow(Clone::clone), vec![(2, away)], "the preparatory cast faces away from P1");
            assert_eq!(game.prep_damage, Some(23.5f32.to_bits()));
        }
        let m = NativeMemory::new();
        let mut game = sora_game(2);
        sora_thunder_to_blizzard_settle(&m, &mut game);
        m.update(&mut game, 0);
        assert_eq!((game.step, game.failed), (Step::Entering, None));
        assert_eq!(ARMED.load(Ordering::Relaxed), 5, "Blizzard arms after an unchanged preparation");
        assert_eq!((game.trail_magic, game.prep_damage), (None, None));
        // Place faced the CPU back toward P1 (P1 lr 1 -> CPU -1) after the away-facing preparation.
        assert_eq!(LR_SETS.with_borrow(|v| v.iter().map(|s| s.1).collect::<Vec<_>>()), vec![1.0, -1.0]);
    }
}
#[test]
fn sora_prep_that_changed_damage_or_left_an_article_never_arms() {
    unsafe {
        for case in ["damage", "article", "reader-unavailable"] {
            let m = NativeMemory::new();
            let mut game = sora_game(2);
            sora_thunder_to_blizzard_settle(&m, &mut game);
            match case {
                "damage" => P1_DAMAGE.store(30.556f32.to_bits(), Ordering::Relaxed),
                "article" => PREP_ARTICLE.store(1, Ordering::Relaxed),
                _ => PREP_ARTICLE.store(2, Ordering::Relaxed),
            }
            m.update(&mut game, 0);
            let want = if case == "reader-unavailable" { UNAVAILABLE_NOTICE } else { "Setup hit you · no grade" };
            assert_eq!((game.step, game.failed), (Step::Failed, Some(want)), "{case}");
            assert_eq!(ARMED.load(Ordering::Relaxed), 0, "{case}");
            assert_eq!(REQUESTS.with_borrow(|v| v.iter().filter(|r| r.1 == 476).count()), 1, "{case}: no Blizzard cast");
            assert!(BEGINS.with_borrow(Vec::is_empty));
        }
        let m = NativeMemory::new();
        CPU_X.store(0.0f32.to_bits(), Ordering::Relaxed);
        let mut game = sora_game(0);
        m.update(&mut game, 0);
        assert_eq!(game.failed, Some("Position unavailable"), "no facing reference: fail before any cast");
        assert!(REQUESTS.with_borrow(Vec::is_empty));
    }
}

#[test]
fn sora_blizzard_escape_grades_only_when_the_provider_is_closed() {
    unsafe {
        for (provider, want) in [(0u32, None), (1, Some("Native tick mismatch · no grade")), (2, Some(UNAVAILABLE_NOTICE))] {
            let mut m = NativeMemory::new();
            m.body[3] = 0x5d;
            let mut game = sora_game(1);
            m.update(&mut game, 0);
            CPU_STATUS.store(486, Ordering::Relaxed);
            m.seed(142.3, 7.0, 1);
            m.start();
            m.update(&mut game, 203);
            // restriction: mashout-blizzard-practice: 116 ends at the seed with the practice notice, no Begin.
            if let Some(text) = crate::model::mashout::moves::practice_only(116) {
                assert_eq!(game.failed, Some(text));
                assert!(BEGINS.with_borrow(Vec::is_empty));
                continue;
            }
            assert_eq!(game.step, Step::Held);
            EXECUTE_HELD.store(true, Ordering::Relaxed);
            BLIZZARD_PROVIDER.store(provider, Ordering::Relaxed);
            m.update(&mut game, 68);
            assert_eq!(game.failed, want, "provider {provider}");
            assert_eq!(game.step, if want.is_none() { Step::Result } else { Step::Failed }, "provider {provider}");
        }
    }
}

#[test]
fn sora_blizzard_arms_the_set_observer_only_for_the_real_cast_and_posts_its_writes() {
    unsafe {
        clatter_set::ARMS.with_borrow_mut(Vec::clear);
        clatter_set::PUBLISHED.with_borrow_mut(Vec::clear);
        WRITES_POSTED.with_borrow_mut(Vec::clear);
        // Thunder preparation: no install during the preparatory cast.
        let m = NativeMemory::new();
        let mut game = sora_game(2);
        sora_thunder_to_blizzard_settle(&m, &mut game);
        assert!(clatter_set::ARMS.with_borrow(Vec::is_empty), "a preparatory cast never installs the observer");
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Entering);
        assert_eq!(clatter_set::ARMS.with_borrow(Clone::clone), vec![(game.generation as u32, 116)]);
        // Seed, then one published re-freeze Set on the held clock and an escape.
        let mut m = NativeMemory::new();
        m.body[3] = 0x5d;
        let mut game = sora_game(1);
        m.update(&mut game, 0);
        CPU_STATUS.store(486, Ordering::Relaxed);
        m.seed(142.3, 7.0, 1);
        m.start();
        m.update(&mut game, 203);
        // restriction: mashout-blizzard-practice: 116 ends at the seed with the practice notice, no Begin.
        if let Some(text) = crate::model::mashout::moves::practice_only(116) {
            assert_eq!(game.failed, Some(text));
            assert!(BEGINS.with_borrow(Vec::is_empty));
            return;
        }
        assert_eq!(game.step, Step::Held);
        EXECUTE_HELD.store(true, Ordering::Relaxed);
        let old = game.clock.unwrap().last().remaining;
        let new = old + 8.442;
        clatter_set::PUBLISHED.with_borrow_mut(|v| v.push(model::mashout::clatter::ClatterWrite {
            generation: game.generation as u32, seq: 1, slot: model::mashout::clatter::Slot::Set, tag: 116,
            old: old.to_bits(), operand: new.to_bits(), new: new.to_bits(), frame: 1, d_max: 1000.0f32.to_bits() }));
        m.seed(new, 7.0, 1);
        m.update(&mut game, 68);
        assert_eq!(game.failed, None, "a bit-exact observed Set keeps the 116 attempt alive");
        let posted = WRITES_POSTED.with_borrow(Clone::clone);
        assert_eq!(posted.len(), 1, "G2 writes posted at the escape");
        assert_eq!((posted[0].0, posted[0].1[0].boundary, posted[0].1[0].old), (1, 0, old.to_bits()));
    }
}



static ICE_SHOT_FIXTURE: AtomicBool = AtomicBool::new(false);
static ICE_SHOT_ARTICLES: AtomicU32 = AtomicU32::new(0);
const NANA_BOMA: usize = 0x4010;
static NANA_STATUS: AtomicU32 = AtomicU32::new(0);
static NANA_OBJECT: AtomicU32 = AtomicU32::new(0x402);
static NANA_ENTRY: AtomicU32 = AtomicU32::new(1);
static SOLO_POPO: AtomicBool = AtomicBool::new(false);

unsafe fn live_ice_shot() -> (Box<NativeMemory>, Game) {
    let mut m = NativeMemory::new();
    m.body[3] = 75;
    let mut game = Game::new(Plan::IceShot, State::Frozen);
    let request = game.requested.as_mut().unwrap();
    request.id = 114;
    request.attacker_kind = 75;
    request.attacker_name = "Ice Climbers";
    game.step = Step::Settle;
    FINISHED.set(0);
    WRITES_POSTED.with_borrow_mut(Vec::clear);
    clatter_set::ARMS.with_borrow_mut(Vec::clear);
    clatter_set::PUBLISHED.with_borrow_mut(Vec::clear);
    ICE_SHOT_FIXTURE.store(true, Ordering::Relaxed);
    ICE_SHOT_ARTICLES.store(0, Ordering::Relaxed);
    NANA_STATUS.store(0, Ordering::Relaxed);
    NANA_OBJECT.store(0x402, Ordering::Relaxed);
    NANA_ENTRY.store(1, Ordering::Relaxed);
    SOLO_POPO.store(false, Ordering::Relaxed);
    CPU_STATUS.store(0, Ordering::Relaxed);
    m.update(&mut game, 0);
    assert_eq!(clatter_set::ARMS.with_borrow(Clone::clone), vec![(game.generation as u32, 114)]);
    CPU_STATUS.store(476, Ordering::Relaxed);
    m.seed(73.25, 7.0, 1);
    m.start();
    m.update(&mut game, 203);
    assert_eq!(game.step, Step::Held, "{:?}", game.failed);
    EXECUTE_HELD.store(true, Ordering::Relaxed);
    (m, game)
}

fn publish_ice_shot_set(game: &Game, sequence: u32) -> f32 {
    let old = game.clock.unwrap().last().remaining;
    let new = old + 4.0;
    clatter_set::PUBLISHED.with_borrow_mut(|writes| writes.push(model::mashout::clatter::ClatterWrite {
        generation: game.generation as u32, seq: sequence, slot: model::mashout::clatter::Slot::Set, tag: 114,
        old: old.to_bits(), operand: new.to_bits(), new: new.to_bits(), frame: 1, d_max: 250.0f32.to_bits(),
    }));
    new
}

#[test]
fn ice_shot_waits_for_both_providers_before_posting_a_grade() {
    unsafe {
        let (mut m, mut game) = live_ice_shot();
        let new = publish_ice_shot_set(&game, 1);
        m.seed(new, 7.0, 1);
        m.update(&mut game, 203);
        assert_eq!(game.step, Step::Held, "{:?}", game.failed);
        CPU_STATUS.store(0, Ordering::Relaxed);
        NANA_STATUS.store(476, Ordering::Relaxed);
        m.update(&mut game, 68);
        assert_eq!(game.step, Step::Held, "Nana can still shoot");
        assert_eq!(FINISHED.get(), 0);
        let frozen = game.clock.unwrap().elapsed();

        NANA_STATUS.store(0, Ordering::Relaxed);
        for live in [1, 2] {
            ICE_SHOT_ARTICLES.store(live, Ordering::Relaxed);
            m.update(&mut game, 0);
            assert_eq!(game.step, Step::Held, "live article mask {live}");
            assert_eq!(game.clock.unwrap().elapsed(), frozen);
            assert_eq!(FINISHED.get(), 0);
            assert!(WRITES_POSTED.with_borrow(Vec::is_empty));
        }
        ICE_SHOT_ARTICLES.store(0, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Result);
        assert_eq!(game.clock.unwrap().elapsed(), frozen);
        assert_eq!(FINISHED.get(), 1);
        let posted = WRITES_POSTED.with_borrow(Clone::clone);
        assert_eq!((posted.len(), posted[0].0, posted[0].1[0].boundary), (1, 1, 0));
    }
}

#[test]
fn ice_shot_refuses_a_set_after_the_native_escape() {
    unsafe {
        let (m, mut game) = live_ice_shot();
        m.update(&mut game, 68);
        assert_eq!(game.step, Step::Held, "Popo can still shoot");
        publish_ice_shot_set(&game, 1);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Failed);
        assert_eq!(FINISHED.get(), 0);
        assert!(WRITES_POSTED.with_borrow(Vec::is_empty));
    }
}

#[test]
fn ice_shot_refuses_unknown_provider_and_bounds_the_wait() {
    unsafe {
        for case in 0..4 {
            let (m, mut game) = live_ice_shot();
            CPU_STATUS.store(0, Ordering::Relaxed);
            match case {
                0 => EGG_PROVIDER_READER.store(model::sites_pkg2::WORK_GET_INT_OFFSET, Ordering::Relaxed),
                1 => NANA_OBJECT.store(0x403, Ordering::Relaxed),
                2 => NANA_OBJECT.store(0x5000_0000, Ordering::Relaxed),
                _ => NANA_ENTRY.store(2, Ordering::Relaxed),
            }
            m.update(&mut game, 68);
            assert_eq!((game.step, game.failed), (Step::Failed, Some(UNAVAILABLE_NOTICE)), "case {case}");
            assert_eq!(FINISHED.get(), 0);
        }
        let (m, mut game) = live_ice_shot();
        NANA_OBJECT.store(0x5000_0000, Ordering::Relaxed);
        SOLO_POPO.store(true, Ordering::Relaxed);
        m.update(&mut game, 68);
        for _ in 1..PROVIDER_CLOSE_FRAMES {
            m.update(&mut game, 0);
            assert_eq!(game.step, Step::Held);
        }
        m.update(&mut game, 0);
        assert_eq!(game.step, Step::Failed, "the cast never left SPECIAL_N");
        assert_eq!(FINISHED.get(), 0);

        let (m, mut game) = live_ice_shot();
        CPU_STATUS.store(0, Ordering::Relaxed);
        NANA_OBJECT.store(0x5000_0000, Ordering::Relaxed);
        SOLO_POPO.store(true, Ordering::Relaxed);
        m.update(&mut game, 68);
        assert_eq!(game.step, Step::Result, "the roster positively removed Nana");
        assert_eq!(FINISHED.get(), 1);
    }
}

unsafe fn live_egg(observed_spit: bool) -> (Box<NativeMemory>, Game) {
    let mut m = NativeMemory::new();
    let mut game = Game::new(Plan::Egg, State::Egg);
    YOSHI_PROVIDER_FIXTURE.store(true, Ordering::Relaxed);
    clatter_set::PUBLISHED.with_borrow_mut(Vec::clear);
    WRITES_POSTED.with_borrow_mut(Vec::clear);
    startup(&mut game, m.p1());
    m.update(&mut game, 243);
    game.writes = model::mashout::clatter::Writes::new(game.generation as u32, 117);
    m.seed(100.0, 7.0, 1);
    CPU_STATUS.store(482, Ordering::Relaxed);
    EGG_SPITED.store(observed_spit, Ordering::Relaxed);
    EXECUTE_HELD.store(true, Ordering::Relaxed);
    m.start();
    m.update(&mut game, 244);
    assert_eq!(game.step, Step::Held);
    (m, game)
}

unsafe fn observe_egg_shell(m: &NativeMemory, game: &mut Game) {
    EGG_VICTIM_SHELL.store(0x1000_0402, Ordering::Relaxed);
    EGG_WEAPON_ACTIVE.store(true, Ordering::Relaxed);
    m.update(game, 244);
    assert_eq!(game.step, Step::Held);
    EGG_VICTIM_SHELL.store(0x5000_0000, Ordering::Relaxed);
    EGG_WEAPON_ACTIVE.store(false, Ordering::Relaxed);
}

fn publish_egg_add(game: &Game, old: f32, operand: f32) -> f32 {
    let new = (old + operand).max(0.0);
    clatter_set::PUBLISHED.with_borrow_mut(|events| {
        events.push(model::mashout::clatter::ClatterWrite {
            generation: game.generation as u32,
            seq: 1,
            slot: model::mashout::clatter::Slot::Add,
            tag: 117,
            old: old.to_bits(),
            operand: operand.to_bits(),
            new: new.to_bits(),
            frame: 1,
            d_max: 0,
        });
    });
    new
}

#[test]
fn egg_completion_requires_same_move_spit_and_exit() {
    unsafe {
        for (spat, end_status) in [(false, 0), (true, 476), (true, 481), (true, 482)] {
            let (m, mut g) = live_egg(spat);
            if spat { observe_egg_shell(&m, &mut g); }
            CPU_STATUS.store(end_status, Ordering::Relaxed);
            m.update(&mut g, 68);
            assert_eq!(
                g.step,
                if spat { Step::Held } else { Step::Failed },
                "spat={spat}, status={end_status}"
            );
            assert!(WRITES_POSTED.with_borrow(Vec::is_empty));
            if spat {
                let frozen = g.clock.unwrap().elapsed();
                CPU_STATUS.store(0, Ordering::Relaxed);
                m.update(&mut g, 68);
                assert_eq!(g.step, Step::Result);
                assert_eq!(g.clock.unwrap().elapsed(), frozen);
            }
        }
        let (m, mut g) = live_egg(true);
        observe_egg_shell(&m, &mut g);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut g, 68);
        assert_eq!(g.step, Step::Result);
        assert!(DRIVEN.with_borrow(|samples| samples.iter().all(|v| *v == MappedInputs::NEUTRAL)));
    }
}

#[test]
fn egg_checks_all_provider_articles_and_fails_closed_on_unavailable_readers() {
    unsafe {
        for article in 0..4 {
            let (m, mut g) = live_egg(true);
            CPU_STATUS.store(0, Ordering::Relaxed);
            YOSHI_ARTICLES.store(1 << article, Ordering::Relaxed);
            m.update(&mut g, 68);
            assert_eq!(g.step, Step::Failed, "provider did not clear: {article}");
            assert!(WRITES_POSTED.with_borrow(Vec::is_empty));
        }
        for reader in [
            0x208_7720,
            model::sites_pkg2::WORK_IS_FLAG_OFFSET,
            model::sites_pkg2::WORK_GET_INT_OFFSET,
            model::sites_pkg2::UTILITY_GET_KIND_OFFSET,
            model::sites_pkg3::STATE_ARTICLE_IS_EXIST_OFFSET,
            model::sites_pkg3::STATE_OBJECT_IS_ACTIVE_OFFSET,
            model::sites_hitbox::HITBOX_IS_ATTACK_OFFSET,
            model::sites_pkg3::STATE_OBJECT_MODULE_ACCESSOR_OFFSET,
        ] {
            let (m, mut g) = live_egg(true);
            CPU_STATUS.store(0, Ordering::Relaxed);
            EGG_PROVIDER_READER.store(reader, Ordering::Relaxed);
            m.update(&mut g, 68);
            assert_eq!((g.step, g.failed), (Step::Failed, Some(UNAVAILABLE_NOTICE)), "reader {reader:#x}");
        }
    }
}

#[test]
fn egg_boundary_zero_replays() {
    unsafe {
        let (mut m, mut g) = live_egg(true);
        observe_egg_shell(&m, &mut g);
        let new = publish_egg_add(&g, 100.0, -17.64);
        m.seed(new, 7.0, 1);
        observe_egg_shell(&m, &mut g);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut g, 68);
        assert_eq!(g.step, Step::Result, "boundary zero: {:?}", g.failed);
        let posted = WRITES_POSTED.with_borrow(Clone::clone);
        assert_eq!((posted.len(), posted[0].0, posted[0].1[0].boundary), (1, 1, 0));
    }
}

#[test]
fn egg_late_add_refuses_only_its_attempt() {
    unsafe {
        let (mut m, mut g) = live_egg(true);
        let sample = reader::sample(m.p1(), 0.37).unwrap();
        let mut active = sample.record;
        active.step(sample.input);
        let mut clock = g.clock.unwrap();
        clock.observe(active, sample.input, false).unwrap();
        assert_eq!(clock.elapsed(), 1);
        g.clock = Some(clock);
        let new = publish_egg_add(&g, active.remaining, -17.64);
        m.seed(new, 7.0, 1);
        // Transfer the previous tick's finished record latches so this is an exact Add-only frame.
        let data = m.record.as_mut_ptr().cast::<i8>();
        for (offset, byte) in [
            (0x10, active.sign[0]), (0x11, active.sign[1]), (0x12, active.source),
            (0x13, active.stick_block as i8), (0x14, active.button_block as i8),
            (0x15, active.accepted as i8), (0x16, active.other_block as i8),
            (0x17, active.cross_reload as i8), (0x18, active.other_reload as i8),
        ] {
            data.add(offset).write(byte);
        }
        let observed = reader::sample(m.p1(), 0.37).unwrap();
        let mut verified = clock;
        let write = clatter_set::PUBLISHED.with_borrow(|events| events[0]);
        verified.observe_writes(observed.record, observed.input, false, &[write]).unwrap();
        assert_eq!(verified.elapsed(), 1);
        m.update(&mut g, 244);
        assert_eq!(g.step, Step::Failed, "Add after the first active tick");
        assert!(WRITES_POSTED.with_borrow(Vec::is_empty));

        let (m, mut g) = live_egg(true);
        observe_egg_shell(&m, &mut g);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut g, 68);
        assert_eq!(g.step, Step::Result, "no-Add attempt after a refusal");
    }
}

#[test]
fn egg_refuses_an_article_that_appears_and_clears_during_the_wait() {
    unsafe {
        for article in 0..4 {
            let (m, mut g) = live_egg(true);
            m.update(&mut g, 68);
            assert_eq!(g.step, Step::Held, "Yoshi is still in N2");
            YOSHI_ARTICLES.store(1 << article, Ordering::Relaxed);
            m.update(&mut g, 68);
            assert_eq!(g.step, Step::Failed, "live article {article} in the post-escape wait");
            YOSHI_ARTICLES.store(0, Ordering::Relaxed);
            CPU_STATUS.store(0, Ordering::Relaxed);
            m.update(&mut g, 68);
            assert_eq!(g.step, Step::Failed, "live article {article} cleared");
            assert!(WRITES_POSTED.with_borrow(Vec::is_empty));
        }
    }
}

#[test]
fn egg_tracks_detached_tamago_and_refuses_its_attack() {
    unsafe {
        let (m, mut g) = live_egg(true);
        EGG_VICTIM_SHELL.store(0x1000_0402, Ordering::Relaxed);
        EGG_WEAPON_ACTIVE.store(true, Ordering::Relaxed);
        m.update(&mut g, 244);
        assert_eq!(g.step, Step::Held, "the unlisted non-attacking shell is eligible");

        EGG_VICTIM_SHELL.store(0x5000_0000, Ordering::Relaxed);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut g, 68);
        assert_eq!(g.step, Step::Held, "the shell is still active after slot clear");
        assert!(WRITES_POSTED.with_borrow(Vec::is_empty));

        EGG_WEAPON_ATTACK.store(true, Ordering::Relaxed);
        m.update(&mut g, 68);
        assert_eq!(g.step, Step::Failed, "the exact detached shell gained a hitbox");
    }
}

#[test]
fn egg_grades_after_the_stock_unlisted_shell_deactivates() {
    unsafe {
        let (m, mut g) = live_egg(true);
        EGG_VICTIM_SHELL.store(0x1000_0402, Ordering::Relaxed);
        EGG_WEAPON_ACTIVE.store(true, Ordering::Relaxed);
        m.update(&mut g, 244);
        CPU_STATUS.store(0, Ordering::Relaxed);
        m.update(&mut g, 68);
        assert_eq!(g.step, Step::Held, "the unlisted shell is still live");
        assert!(WRITES_POSTED.with_borrow(Vec::is_empty));

        EGG_VICTIM_SHELL.store(0x5000_0000, Ordering::Relaxed);
        m.update(&mut g, 68);
        assert_eq!(g.step, Step::Held, "clearing the slot does not deactivate the weapon");
        EGG_WEAPON_ACTIVE.store(false, Ordering::Relaxed);
        m.update(&mut g, 68);
        assert_eq!(g.step, Step::Result, "the exact shell is now inactive");
    }
}


#[test]
fn egg_refuses_an_active_shell_with_wrong_kind_or_link_owner() {
    unsafe {
        for (kind, owner) in [(0x38, 0x400), (0x39, 0x401)] {
            let (m, mut g) = live_egg(true);
            EGG_VICTIM_SHELL.store(0x1000_0402, Ordering::Relaxed);
            EGG_WEAPON_ACTIVE.store(true, Ordering::Relaxed);
            EGG_WEAPON_KIND.store(kind, Ordering::Relaxed);
            EGG_WEAPON_OWNER.store(owner, Ordering::Relaxed);
            m.update(&mut g, 244);
            assert_eq!(g.step, Step::Failed, "kind={kind:#x}, owner={owner:#x}");
            assert!(WRITES_POSTED.with_borrow(Vec::is_empty));
        }
    }
}
