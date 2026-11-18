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
    origin: SeedOrigin,
}
impl Key {
    pub fn new(
        move_id: u16,
        attacker: u32,
        victim: u32,
        damage: f32,
        seed: Seed,
        origin: SeedOrigin,
    ) -> Self {
        let r = seed.record;
        Self {
            move_id,
            origin,
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
    Begin(u64, u64, Key, Seed, SeedOrigin),
    Observe(u64, u64, u32, bool),
    /// 116: the attempt's applied Sets (G1 boundaries), replayed in the bounds by ice116::bounds.
    Writes(u64, u64, [crate::model::mashout::ice116::Applied; 8], u8),
    Pulse(u64, u64, bool),
    NativeEnd(u64, u64),
    Failed(u64, &'static str),
    Cancel(u64),
}
struct Worker {
    generation: u64,
    epoch: u64,
    origin: SeedOrigin,
    seed: Option<Seed>,
    session: Run<Key>,
    shown: crate::model::storage::FixedPath<256>,
}
static WORKER: Mutex<Worker> = Mutex::new(Worker {
    generation: 0,
    epoch: 0,
    origin: SeedOrigin::Export,
    seed: None,
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
    } else if worker.session.phase() == crate::model::mashout::run::Phase::Done {
        // Reuse the foreground standing notice: queued roster toasts wait behind the result.
        notices::show_now(NOTICE_ID, "Mash out", text, u16::MAX, DEFAULT_COLOR);
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
    if !READY.load(Ordering::Acquire)
        || !SLOTS_READY.load(Ordering::Acquire)
        || CONFIG.read().is_none()
    {
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
        Event::Begin(generation, epoch, key, seed, origin) => {
            if generation < worker.generation {
                return;
            }
            worker.generation = generation;
            worker.epoch = epoch;
            worker.origin = origin;
            worker.seed = Some(seed);
            if worker.session.capture(epoch, key, seed).is_err() {
                worker.session.cancel();
                // restriction: MASH-M1-MISMATCH
                show(&mut worker, "Native model unavailable · no grade");
            }
        }
        Event::Observe(generation, epoch, elapsed, escaped) => {
            if generation == worker.generation
                && epoch == worker.epoch
                && epoch == EPOCH.load(Ordering::Acquire)
                && !faulted(generation, FAULT.load(Ordering::Acquire))
            {
                worker.session.observe_generation(epoch, elapsed, escaped);
            }
        }
        Event::Writes(generation, epoch, sets, count) => {
            if generation == worker.generation
                && epoch == worker.epoch
                && epoch == EPOCH.load(Ordering::Acquire)
                && !faulted(generation, FAULT.load(Ordering::Acquire))
            {
                let writes = &sets[..usize::from(count).min(sets.len())];
                match worker.seed.ok_or(()).and_then(|seed| crate::model::mashout::ice116::bounds(seed, writes).map_err(|_| ())) {
                    Ok(bounds) => worker.session.rebound(epoch, bounds),
                    Err(()) => {
                        worker.session.cancel();
                        // restriction: MASH-M1-MISMATCH
                        show(&mut worker, "Native tick mismatch · no grade");
                    }
                }
            }
        }
        Event::Pulse(generation, epoch, tick) => {
            if generation == worker.generation
                && epoch == worker.epoch
                && epoch == EPOCH.load(Ordering::Acquire)
                && worker.session.pass_generation(epoch, tick)
            {
                if !REPLIES.post(generation) {
                    fault(generation);
                }
            }
        }
        Event::NativeEnd(generation, epoch) => {
            if generation == worker.generation
                && epoch == worker.epoch
                && epoch == EPOCH.load(Ordering::Acquire)
                && !faulted(generation, FAULT.load(Ordering::Acquire))
            {
                worker.session.native_end(epoch);
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
    } else if worker.epoch != EPOCH.load(Ordering::Acquire) {
        // A published native epoch supersedes this solver before any best/result publication.
        worker.session.cancel();
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
            let mut text = crate::model::storage::FixedPath::<256>::new();
            if worker.origin == SeedOrigin::ThrowClatterStart {
                let _ = core::fmt::Write::write_str(&mut text, "from the throw's clatter start · ");
            }
            let _ = worker.session.write_notice(&mut text);
            show(&mut worker, text.as_str());
        }
    }
}
