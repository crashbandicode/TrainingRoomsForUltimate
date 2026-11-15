//! Single existing diagnostic-worker consumer. All solver, grading, best and cache work lives here.
use super::*;
use crate::model::mashout::{native::Seed, session::Session};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Key {
    attacker: u32,
    victim: u32,
    damage: u32,
    seed: [u32; 9],
}
impl Key {
    pub fn new(attacker: u32, victim: u32, damage: f32, seed: Seed) -> Self {
        let r = seed.record;
        Self {
            attacker,
            victim,
            damage: damage.to_bits(),
            seed: [
                r.remaining.to_bits(),
                r.automatic.to_bits(),
                r.manual.to_bits(),
                r.button_mul.to_bits(),
                r.threshold.to_bits(),
                u32::from_le_bytes([
                    r.sign[0] as u8,
                    r.sign[1] as u8,
                    r.source as u8,
                    r.stick_block as u8,
                ]),
                u32::from_le_bytes([
                    r.button_block as u8,
                    r.accepted,
                    r.other_block as u8,
                    r.cross_reload as u8,
                ]),
                u32::from_le_bytes([r.other_reload as u8, seed.held, 0, 0]),
                seed.deadline.unwrap_or(u32::MAX),
            ],
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Event {
    Setup(u64, &'static str),
    Begin(u64, Key, Seed),
    Observe(u64, u32, bool),
    Pulse(u64, bool),
    Failed(u64, &'static str),
    Cancel(u64),
}
struct Worker {
    generation: u64,
    session: Session<Key>,
    faulted: bool,
    shown: crate::model::storage::FixedPath<256>,
    wanted: bool,
}
static WORKER: Mutex<Worker> = Mutex::new(Worker {
    generation: 0,
    session: Session::new(),
    faulted: false,
    shown: crate::model::storage::FixedPath::new(),
    wanted: false,
});

fn show(worker: &mut Worker, text: &str) {
    let text = text.strip_prefix("Mash out · ").unwrap_or(text);
    if worker.shown.as_str() == text {
        return;
    }
    worker.shown = crate::model::storage::FixedPath::new();
    let _ = core::fmt::Write::write_str(&mut worker.shown, text);
    if text.is_empty() {
        notices::clear(NOTICE_ID);
    } else {
        notices::upsert(NOTICE_ID, "Mash out", text, u16::MAX, DEFAULT_COLOR);
    }
}

pub(super) fn service() {
    // Worker-only lock; no hook ever takes WORKER or a Session reference.
    let mut worker = WORKER.lock();
    let wanted = WANTED.load(Ordering::Acquire);
    if wanted && (!READY.load(Ordering::Acquire) || CONFIG.read().is_none()) {
        worker.session.cancel();
        EVENTS.drain(|_| ());
        show(&mut worker, UNAVAILABLE_NOTICE);
        notices::raise(NOTICE_RAISE, true);
        return;
    }
    if worker.wanted && !wanted {
        worker.session.cancel();
        show(&mut worker, "");
    }
    worker.wanted = wanted;
    notices::raise(NOTICE_RAISE, wanted);
    EVENTS.drain(|event| match event {
        Event::Setup(generation, text) => {
            if generation < worker.generation {
                return;
            }
            worker.generation = generation;
            worker.session.cancel();
            worker.faulted = false;
            if wanted {
                show(&mut worker, text);
            }
        }
        Event::Begin(generation, key, seed) => {
            if generation < worker.generation || !wanted {
                return;
            }
            worker.generation = generation;
            worker.faulted = false;
            if worker.session.begin(generation, key, seed).is_err() {
                worker.faulted = true;
                show(&mut worker, "Native model unavailable · no grade");
            }
        }
        Event::Observe(generation, elapsed, escaped) => {
            if generation == worker.generation
                && !worker.faulted
                && FAULT.load(Ordering::Acquire) != generation
            {
                worker.session.observe(generation, elapsed, escaped);
            }
        }
        Event::Pulse(generation, tick) => {
            if generation == worker.generation && worker.session.pass(generation, tick) {
                if !REPLIES.post(generation) {
                    FAULT.store(generation, Ordering::Release);
                }
            }
        }
        Event::Failed(generation, reason) => {
            if generation != worker.generation {
                return;
            }
            worker.session.cancel();
            worker.faulted = true;
            if wanted {
                show(&mut worker, reason);
            }
        }
        Event::Cancel(generation) => {
            if generation != worker.generation {
                return;
            }
            worker.session.cancel();
            show(&mut worker, "");
        }
    });
    if FAULT.load(Ordering::Acquire) == worker.generation && worker.generation != 0 {
        worker.session.cancel();
        worker.faulted = true;
        if wanted {
            show(&mut worker, "Observation queue full · no grade");
        }
    }
    if !worker.faulted {
        worker.session.advance(256);
        if let Some(error) = worker.session.error() {
            worker.faulted = true;
            show(&mut worker, "Native model unavailable · no grade");
            crate::runtime::diag_log::diagnostic(format_args!(
                "[Mashout] solver_error gen={} error={error:?}",
                worker.generation
            ));
        } else if worker.session.run().phase() != crate::model::mashout::run::Phase::Off {
            if let Some(text) = worker.session.run_mut().notice() {
                if wanted {
                    show(&mut worker, text.as_str());
                }
            }
        }
    }
}
