use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
mod model {
    pub mod storage { // STORAGE
    }
    pub mod eggs {
        pub const END_FRAMES: u16 = 180;
    }
    pub mod practice_games { // PRACTICE
    }
    pub mod mashout {
        pub mod grade { // GRADE
        }
        pub mod native { // NATIVE
        }
        pub mod run { // RUN
        }
        pub mod clatter { // CLATTER
        }
        pub mod ice116 { // ICE116
        }
        pub mod egg117 { // EGG117
        }
    }
}
mod runtime {
    pub mod diag_log {
        pub fn diagnostic(_: core::fmt::Arguments<'_>) {}
    }
}
// SEED_ORIGIN
// FAULTED
const NOTICE_ID: u8 = 48;
const NOTICE_RAISE: u8 = 4;
const DEFAULT_COLOR: u32 = 0;
const UNAVAILABLE_NOTICE: &str = "Mash out unavailable";
static READY: AtomicBool = AtomicBool::new(true);
static SLOTS_READY: AtomicBool = AtomicBool::new(true);
static WANTED: AtomicBool = AtomicBool::new(true);
static EPOCH: AtomicU64 = AtomicU64::new(0);
static FAULT: AtomicU64 = AtomicU64::new(0);
static SWITCH_AFTER_DRAIN: AtomicU64 = AtomicU64::new(0);
struct Config;
static CONFIG: Config = Config;
impl Config {
    fn read(&self) -> Option<()> {
        Some(())
    }
}
struct Mutex<T>(std::sync::Mutex<T>);
impl<T> Mutex<T> {
    const fn new(value: T) -> Self {
        Self(std::sync::Mutex::new(value))
    }
    fn lock(&self) -> std::sync::MutexGuard<'_, T> {
        self.0.lock().unwrap()
    }
}
struct Events(std::sync::Mutex<Vec<worker::Event>>);
static EVENTS: Events = Events(std::sync::Mutex::new(Vec::new()));
impl Events {
    fn drain(&self, mut f: impl FnMut(worker::Event)) {
        let queued = std::mem::take(&mut *self.0.lock().unwrap());
        for event in queued {
            f(event);
        }
        // Controlled native-hook interleaving after event admission and before the worker's final publication check.
        let epoch = SWITCH_AFTER_DRAIN.swap(0, Ordering::Relaxed);
        if epoch != 0 {
            EPOCH.store(epoch, Ordering::Release);
        }
    }
}
struct Replies(std::sync::Mutex<Vec<u64>>);
static REPLIES: Replies = Replies(std::sync::Mutex::new(Vec::new()));
impl Replies {
    fn post(&self, g: u64) -> bool {
        self.0.lock().unwrap().push(g);
        true
    }
}
fn fault(generation: u64) {
    FAULT.fetch_max(generation, Ordering::AcqRel);
}
mod notices {
    static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
    pub fn clear(_: u8) {
        LAST.lock().unwrap().clear();
    }
    pub fn raise(_: u8, _: bool) {}
    pub fn show_now(_: u8, _: &str, text: &str, _: u16, _: u32) {
        *LAST.lock().unwrap() = text.to_owned();
    }
    pub fn upsert(id: u8, title: &str, text: &str, frames: u16, color: u32) {
        show_now(id, title, text, frames, color);
    }
    pub fn last() -> String {
        LAST.lock().unwrap().clone()
    }
}
mod worker {
    // WORKER
    #[cfg(test)]
    mod epoch_tests {
        use super::*;
        use crate::model::mashout::{native::Record, run::Phase};
        fn reset() {
            *WORKER.lock() = Worker {
                generation: 0,
                epoch: 0,
                origin: SeedOrigin::Export,
                seed: None,
                session: Run::new(),
                shown: crate::model::storage::FixedPath::new(),
            };
            EPOCH.store(1, Ordering::Relaxed);
            FAULT.store(0, Ordering::Relaxed);
            SWITCH_AFTER_DRAIN.store(0, Ordering::Relaxed);
            EVENTS.0.lock().unwrap().clear();
            REPLIES.0.lock().unwrap().clear();
            notices::clear(NOTICE_ID);
        }
        fn seed() -> Seed {
            Seed {
                record: Record {
                    remaining: 8.0,
                    automatic: 1.0,
                    manual: 2.0,
                    button_mul: 1.0,
                    threshold: 0.37,
                    sign: [0, 0],
                    source: -1,
                    stick_block: 0,
                    button_block: 0,
                    accepted: 0,
                    other_block: 0,
                    cross_reload: 4,
                    other_reload: 3,
                },
                held: 0,
                deadline: None,
            }
        }
        fn key(origin: SeedOrigin) -> Key {
            Key::new(117, 5, 0, 23.5, seed(), origin)
        }
        fn queue(event: Event) {
            EVENTS.0.lock().unwrap().push(event);
        }
        fn begin(epoch: u64, origin: SeedOrigin) {
            EPOCH.store(epoch, Ordering::Release);
            queue(Event::Begin(5, epoch, key(origin), seed(), origin));
            service();
            assert_eq!(WORKER.lock().session.phase(), Phase::Mashing);
        }
        fn solved() -> crate::model::mashout::grade::Bounds {
            for _ in 0..100 {
                if let Some(bounds) = WORKER.lock().session.bounds() {
                    return bounds;
                }
                service();
            }
            panic!("tiny native seed did not solve within the bound");
        }
        fn finish(epoch: u64) {
            let bounds = solved();
            queue(Event::Observe(5, epoch, bounds.fastest, true));
            service();
            assert_eq!(WORKER.lock().session.phase(), Phase::Done);
        }
        #[test]
        fn newer_native_epoch_cancels_before_best_publication() {
            reset();
            begin(1, SeedOrigin::Export);
            let bounds = solved();
            queue(Event::Observe(5, 1, bounds.fastest, true));
            SWITCH_AFTER_DRAIN.store(2, Ordering::Relaxed);
            service();
            let worker = WORKER.lock();
            assert_eq!(worker.session.phase(), Phase::Off);
            assert_eq!(worker.session.result(), None);
            assert_eq!(worker.session.best(key(SeedOrigin::Export)), None);
            assert!(!notices::last().contains("best"));
        }
        #[test]
        fn dropped_native_seed_fault_cancels_before_best_publication() {
            reset();
            begin(1, SeedOrigin::Export);
            let bounds = solved();
            queue(Event::Observe(5, 1, bounds.fastest, true));
            // publish_seed uses this watermark when its mailbox rejects the newer seed.
            fault(5);
            service();
            let worker = WORKER.lock();
            assert_eq!(worker.session.phase(), Phase::Off);
            assert_eq!(worker.session.result(), None);
            assert_eq!(worker.session.best(key(SeedOrigin::Export)), None);
            assert_eq!(notices::last(), "Observation incomplete · no grade");
        }
        #[test]
        fn same_outer_generation_new_epoch_replaces_run_and_rejects_stale_observation() {
            reset();
            begin(1, SeedOrigin::Export);
            queue(Event::Observe(5, 1, 2, false));
            service();
            assert_eq!(WORKER.lock().session.elapsed(), 2);
            begin(2, SeedOrigin::ThrowClatterStart);
            queue(Event::Observe(5, 1, 999, true));
            service();
            let worker = WORKER.lock();
            assert_eq!((worker.generation, worker.epoch), (5, 2));
            assert_eq!(worker.session.elapsed(), 0);
            assert!(!worker.session.escaped());
            assert_eq!(
                worker.session.key(),
                Some(key(SeedOrigin::ThrowClatterStart))
            );
            assert_eq!(worker.session.result(), None);
        }
        #[test]
        fn stale_pulse_cannot_advance_current_result_delay() {
            reset();
            begin(1, SeedOrigin::Export);
            begin(2, SeedOrigin::ThrowClatterStart);
            finish(2);
            let before = WORKER.lock().session.remaining();
            queue(Event::Pulse(5, 1, true));
            service();
            assert_eq!(WORKER.lock().session.remaining(), before);
            queue(Event::Pulse(5, 2, true));
            service();
            assert_eq!(WORKER.lock().session.remaining(), before - 1);
        }
        #[test]
        fn origin_separates_bests_and_conditional_notice_is_always_labelled() {
            reset();
            let export = key(SeedOrigin::Export);
            let native = key(SeedOrigin::ThrowClatterStart);
            assert_ne!(export, native);
            assert_eq!(export.seed, native.seed);
            begin(1, SeedOrigin::Export);
            finish(1);
            assert!(WORKER.lock().session.best(export).is_some());
            assert_eq!(WORKER.lock().session.best(native), None);
            assert!(!notices::last().contains("from the throw"));
            begin(2, SeedOrigin::ThrowClatterStart);
            assert!(notices::last().starts_with("from the throw's clatter start · "));
            finish(2);
            assert!(WORKER.lock().session.best(native).is_some());
            let text = notices::last();
            assert!(
                text.starts_with("from the throw's clatter start · "),
                "{text}"
            );
            assert!(text.contains("fastest") && text.contains("best"), "{text}");
        }

        #[test]
        fn egg_opening_add_replaces_the_solver_and_partitions_the_best() {
            reset();
            begin(1, SeedOrigin::Export);
            assert_eq!(solved().no_input, Some(8));
            let mut writes = [crate::model::mashout::ice116::Applied {
                boundary: 0, old: 0, operand: 0, new: 0, d_max: 0,
            }; 8];
            writes[0].old = 8.0f32.to_bits();
            writes[0].operand = (-2.0f32).to_bits();
            writes[0].new = 6.0f32.to_bits();
            queue(Event::EggWrites(5, 1, writes, 1));
            service();
            assert_eq!(solved().no_input, Some(6));
            finish(1);
            {
                let worker = WORKER.lock();
                let original = key(SeedOrigin::Export);
                let altered = worker.session.key().unwrap();
                assert_eq!(worker.session.best(original), None);
                assert_eq!(worker.session.best(altered), Some(worker.session.result().unwrap().bounds.fastest));
            }

            reset();
            begin(1, SeedOrigin::Export);
            queue(Event::EggWrites(5, 0, writes, 1));
            service();
            assert_eq!(solved().no_input, Some(8));
        }

        #[test]
        fn native_end_is_unscored_and_reuses_result_delay() {
            reset();
            begin(1, SeedOrigin::Export);
            queue(Event::Observe(5, 1, 28, false));
            queue(Event::NativeEnd(5, 1));
            service();
            assert_eq!(notices::last(), "Move ended before escape · no grade");
            {
                let worker = WORKER.lock();
                assert_eq!(worker.session.phase(), Phase::Done);
                assert_eq!(worker.session.result(), None);
                assert_eq!(worker.session.best(key(SeedOrigin::Export)), None);
                assert_eq!(worker.session.remaining(), 180);
            }
            for _ in 0..180 {
                queue(Event::Pulse(5, 1, true));
                service();
            }
            assert_eq!(WORKER.lock().session.phase(), Phase::Setup);
        }
        #[test]
        fn stale_end_and_native_escape_cleanup_cannot_replace_grade() {
            reset();
            begin(1, SeedOrigin::Export);
            queue(Event::NativeEnd(4, 1));
            queue(Event::NativeEnd(5, 0));
            service();
            assert_eq!(WORKER.lock().session.phase(), Phase::Mashing);
            finish(1);
            let result = WORKER.lock().session.result();
            queue(Event::NativeEnd(5, 1));
            service();
            assert_eq!(WORKER.lock().session.result(), result);
            assert!(notices::last().contains("best"));
        }
    }
}
