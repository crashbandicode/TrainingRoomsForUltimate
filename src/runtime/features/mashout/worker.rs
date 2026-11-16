//! Single existing diagnostic-worker consumer. All solver, grading and best work lives here.
use super::*;
use crate::model::mashout::{native::Seed, run::Run};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Key {
    move_id: u16,
    attacker: u32,
    victim: u32,
    damage: u32,
    seed: [u32; 9],
}
impl Key {
    pub fn new(move_id: u16, attacker: u32, victim: u32, damage: f32, seed: Seed) -> Self {
        let r = seed.record;
        Self {
            move_id,
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
    session: Run<Key>,
    shown: crate::model::storage::FixedPath<256>,
}
static WORKER: Mutex<Worker> = Mutex::new(Worker {
    generation: 0,
    session: Run::new(),
    shown: crate::model::storage::FixedPath::new(),
});

fn show(worker: &mut Worker, text: &str) {
    let text = text.strip_prefix("Mash out · ").unwrap_or(text);
    if worker.shown.as_str() == text {
        return;
    }
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!(
        "[Mashout] notice gen={} text={text}",
        worker.generation
    ));
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
    if !wanted {
        worker.session.cancel();
        EVENTS.drain(|_| ());
        show(&mut worker, "");
        notices::raise(NOTICE_RAISE, false);
        return;
    }
    notices::raise(NOTICE_RAISE, true);
    if !READY.load(Ordering::Acquire) || CONFIG.read().is_none() {
        worker.session.cancel();
        EVENTS.drain(|_| ());
        show(&mut worker, UNAVAILABLE_NOTICE);
        return;
    }
    EVENTS.drain(|event| match event {
        Event::Setup(generation, text) => {
            if generation < worker.generation {
                return;
            }
            worker.generation = generation;
            worker.session.cancel();
            show(&mut worker, text);
        }
        Event::Begin(generation, key, seed) => {
            if generation < worker.generation {
                return;
            }
            worker.generation = generation;
            if worker.session.capture(generation, key, seed).is_err() {
                worker.session.cancel();
                // restriction: MASH-M1-MISMATCH
                show(&mut worker, "Native model unavailable · no grade");
            }
        }
        Event::Observe(generation, elapsed, escaped) => {
            if generation == worker.generation
                && !faulted(generation, FAULT.load(Ordering::Acquire))
            {
                worker
                    .session
                    .observe_generation(generation, elapsed, escaped);
            }
        }
        Event::Pulse(generation, tick) => {
            if generation == worker.generation && worker.session.pass_generation(generation, tick) {
                if !REPLIES.post(generation) {
                    fault(generation);
                }
            }
        }
        Event::Failed(generation, reason) => {
            if generation != worker.generation {
                return;
            }
            worker.session.cancel();
            show(&mut worker, reason);
        }
        Event::Cancel(generation) => {
            if generation != worker.generation {
                return;
            }
            worker.session.cancel();
            show(&mut worker, "");
        }
    });
    worker.session.advance(256);
    // Completion linearizes at this load, after solver work and before any best/Done publication.
    if faulted(worker.generation, FAULT.load(Ordering::Acquire)) {
        worker.session.cancel();
        // restriction: MASH-M1-MISMATCH
        show(&mut worker, "Observation incomplete · no grade");
    } else if let Some(error) = worker.session.error() {
        // restriction: MASH-M1-MISMATCH
        show(&mut worker, "Native model unavailable · no grade");
        crate::runtime::diag_log::diagnostic(format_args!(
            "[Mashout] solver_error gen={} error={error:?}",
            worker.generation
        ));
        worker.session.cancel();
    } else {
        worker.session.finish();
        if worker.session.phase() != crate::model::mashout::run::Phase::Off {
            let text = worker.session.notice();
            show(&mut worker, text.as_str());
        }
    }
}
