use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
mod model {
    pub mod storage { // STORAGE
    }
    pub mod eggs {
        pub const END_FRAMES: u16 = 180;
    }
    pub mod practice_games { // PRACTICE
    }
    pub mod practice_state { // PRACTICE_STATE
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
    pub mod tech { pub mod session { // TECH_SESSION
    } }
    pub mod tech_launch { // TECH_LAUNCH
    }
}
mod runtime {
    pub mod bridge_state {
        use crate::model::{practice_games::Game, practice_state::Fields};
        use std::sync::atomic::Ordering;

        #[derive(Clone, Copy, Default)]
        pub struct PracticeCapture { activation: u64 }
        #[allow(dead_code)]
        impl PracticeCapture {
            pub fn of(_: Game) -> Self { Self { activation: crate::ACTIVATION.load(Ordering::Acquire) } }
            pub fn only(self, activation: u64) -> Self {
                if self.activation == activation { self } else { Self::default() }
            }
            pub fn result(self, sample: impl FnOnce() -> Option<Fields>) {
                if self.activation == 0 || self.activation != crate::ACTIVATION.load(Ordering::Acquire) {
                    return;
                }
                if let Some(fields) = sample() {
                    let mut queue = crate::RESULTS.lock().unwrap();
                    if queue.len() < crate::model::practice_state::SAMPLE_SLOTS {
                        queue.push((self.activation, fields, true));
                    } else {
                        crate::REFUSED.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
    }
    pub mod diag_log {
        pub fn diagnostic(_: core::fmt::Arguments<'_>) {}
    }
}
// SEED_ORIGIN
// FAULTED
const NOTICE_ID: u8 = 48;
const DEFAULT_COLOR: u32 = 0;
const UNAVAILABLE_NOTICE: &str = "Mash out unavailable";
static READY: AtomicBool = AtomicBool::new(true);
static SLOTS_READY: AtomicBool = AtomicBool::new(true);
static WANTED: AtomicBool = AtomicBool::new(true);
/// The runtime reads the running game through `wanted()` (rc.15 practice menu); the fixture keeps its own flag.
fn wanted() -> bool {
    WANTED.load(Ordering::Acquire)
}
static EPOCH: AtomicU64 = AtomicU64::new(0);
static FAULT: AtomicU64 = AtomicU64::new(0);
static SWITCH_AFTER_DRAIN: AtomicU64 = AtomicU64::new(0);
static ACTIVATION: AtomicU64 = AtomicU64::new(0);
static OWNER_GENERATION: AtomicU64 = AtomicU64::new(0);
static OWNER_ACTIVATION: AtomicU64 = AtomicU64::new(0);
static REFUSED: AtomicU64 = AtomicU64::new(0);
static RESULTS: std::sync::Mutex<Vec<(u64, model::practice_state::Fields, bool)>> = std::sync::Mutex::new(Vec::new());
#[allow(dead_code)]
fn result_owner() -> Option<(u64, u64)> {
    let generation = OWNER_GENERATION.load(Ordering::Acquire);
    let activation = OWNER_ACTIVATION.load(Ordering::Acquire);
    (generation != 0 && generation == OWNER_GENERATION.load(Ordering::Acquire)).then_some((generation, activation))
}
fn own_result(generation: u64, activation: u64) {
    OWNER_GENERATION.store(0, Ordering::Release);
    OWNER_ACTIVATION.store(activation, Ordering::Release);
    OWNER_GENERATION.store(generation, Ordering::Release);
}
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
    use crate::model::practice_games::Game as PracticeGame;
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
                tech_grade: None,
                shown: crate::model::storage::FixedPath::new(),
            };
            EPOCH.store(1, Ordering::Relaxed);
            FAULT.store(0, Ordering::Relaxed);
            SWITCH_AFTER_DRAIN.store(0, Ordering::Relaxed);
            ACTIVATION.store(0, Ordering::Relaxed);
            own_result(0, 0);
            REFUSED.store(0, Ordering::Relaxed);
            RESULTS.lock().unwrap().clear();
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
        fn drain(overlay: &mut crate::model::practice_state::Fields) -> usize {
            let mut results = 0;
            for (activation, fields, is_result) in std::mem::take(&mut *RESULTS.lock().unwrap()) {
                if is_result && activation == ACTIVATION.load(Ordering::Acquire) {
                    *overlay = fields;
                    results += 1;
                }
            }
            results
        }
        #[test]
        fn a_grade_refused_by_a_full_box_is_posted_again() {
            use crate::model::practice_state::{Fields, SAMPLE_SLOTS};
            use crate::model::tech::session::Grade;
            reset();
            ACTIVATION.store(1, Ordering::Release);
            own_result(5, 1);
            queue(Event::Setup(5, "Mash out · Setting up"));
            service();

            let mut overlay = Fields::default();
            queue(Event::TechGrade(5, Grade::Perfect(Some(2)), None, 3));
            service();
            assert_eq!(drain(&mut overlay), 1);
            assert_eq!((overlay.last_outcome, overlay.best), ("perfect", Some(3)));

            queue(Event::TechGrade(5, Grade::Missed, None, 4));
            service();
            assert_eq!(drain(&mut overlay), 1, "a grade with room arrives on the first tick");
            assert_eq!((overlay.last_outcome, overlay.best), ("missed", Some(4)));

            queue(Event::TechGrade(5, Grade::Perfect(Some(2)), None, 3));
            service();
            assert_eq!(drain(&mut overlay), 1);
            assert_eq!(overlay.last_outcome, "perfect");
            {
                let mut mailbox = RESULTS.lock().unwrap();
                for _ in 0..SAMPLE_SLOTS {
                    mailbox.push((1, Fields { phase: "result", ..Fields::default() }, false));
                }
            }
            queue(Event::TechGrade(5, Grade::Missed, None, 4));
            service();
            assert_eq!(REFUSED.load(Ordering::Relaxed), 1);
            assert_eq!(drain(&mut overlay), 0);
            assert_eq!(overlay.last_outcome, "perfect");
            service();
            assert_eq!(drain(&mut overlay), 1, "the refused grade is posted on the next tick");
            assert_eq!((overlay.last_outcome, overlay.best), ("missed", Some(4)));

            own_result(6, 1);
            queue(Event::Setup(6, "Mash out · Setting up"));
            service();
            assert_eq!(drain(&mut overlay), 0, "the previous generation must not be reposted");
        }
        #[test]
        fn a_new_activation_does_not_inherit_the_previous_done_grade() {
            reset();
            ACTIVATION.store(1, Ordering::Release);
            let stale = crate::runtime::bridge_state::PracticeCapture::of(PracticeGame::Mashout);
            ACTIVATION.store(2, Ordering::Release);
            stale.result(|| panic!("a stale capture must not project"));
            crate::runtime::bridge_state::PracticeCapture::default().result(|| panic!("a disabled capture must not project"));
            ACTIVATION.store(1, Ordering::Release);
            own_result(5, 1);
            begin(1, SeedOrigin::Export);
            finish(1);
            let posted = std::mem::take(&mut *RESULTS.lock().unwrap());
            assert_eq!(posted.len(), 1);
            assert_eq!(posted[0].0, 1);
            assert!(!posted[0].1.last_outcome.is_empty());

            ACTIVATION.store(2, Ordering::Release);
            service();
            assert!(RESULTS.lock().unwrap().is_empty(), "old Done must not publish under the new activation");

            own_result(6, 2);
            queue(Event::Cancel(5));
            queue(Event::Setup(6, "Mash out · Loading attacker"));
            service();
            assert!(RESULTS.lock().unwrap().is_empty());
            EPOCH.store(2, Ordering::Release);
            queue(Event::Begin(6, 2, key(SeedOrigin::Export), seed(), SeedOrigin::Export));
            service();
            let bounds = solved();
            queue(Event::Observe(6, 2, bounds.fastest, true));
            service();
            let posted = std::mem::take(&mut *RESULTS.lock().unwrap());
            assert_eq!(posted.len(), 1);
            assert_eq!(posted[0].0, 2);
            assert!(!posted[0].1.last_outcome.is_empty());

            own_result(7, 2);
            service();
            assert!(RESULTS.lock().unwrap().is_empty(), "a different generation must not publish");
            own_result(6, 2);
            ACTIVATION.store(3, Ordering::Release);
            queue(Event::TechGrade(6, crate::model::tech::session::Grade::Perfect(Some(2)), None, 3));
            service();
            assert!(RESULTS.lock().unwrap().is_empty(), "old TechGrade must not publish under the new activation");
            own_result(6, 3);
            queue(Event::TechGrade(6, crate::model::tech::session::Grade::Perfect(Some(2)), None, 3));
            service();
            let posted = std::mem::take(&mut *RESULTS.lock().unwrap());
            assert_eq!(posted.len(), 1);
            assert_eq!((posted[0].0, posted[0].1.last_outcome, posted[0].1.best), (3, "perfect", Some(3)));
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
