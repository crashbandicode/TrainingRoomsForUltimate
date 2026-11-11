//! One saved-Mii catalog read at a time. Values only: no files, threads, or native access.
//!
//! A menu open queues at most one read. A newer open, close, or retired generation does not
//! apply the older result and does not enqueue a second read beside it. The caller retries a
//! full writer queue across later polls, at most [`MAX_SENDS`] attempts, and never blocks.

use crate::engine::mii::auto_init::Token;

/// Attempts for one demand, including the first. Matches a full replay-writer queue draining
/// across later polls rather than a same-call spin.
pub const MAX_SENDS: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Live {
    pub menu_active: bool,
    pub opening: u64,
    pub owners: [u64; 2],
    pub auto: Option<Token>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Training or a qualified waiting backend (local wireless, Quickplay). The caller
    /// may arm fighter capture after this result.
    Menu { opening: u64, owners: [u64; 2] },
    /// A waiting room without a qualified fighter backend. The caller mounts the
    /// editor-only catalog and does not arm a fighter role. One opening mounts one
    /// catalog: a `Menu` demand for the same opening supersedes this kind, because the
    /// fighter menu already offers Add New.
    Editor { opening: u64 },
    /// Lifetime read before any menu open.
    Auto(Token),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Submit(Kind),
    Wait,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Menu { opening: u64, owners: [u64; 2] },
    Editor { opening: u64 },
    Auto(Token),
    Discard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effect {
    pub verdict: Verdict,
    pub step: Step,
    pub finished: Option<Kind>,
    /// A demand that will not receive a catalog, besides `finished` when that was discarded.
    pub dropped: Option<Kind>,
}

impl Effect {
    pub const fn quiet(step: Step) -> Self {
        Self {
            verdict: Verdict::Discard,
            step,
            finished: None,
            dropped: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Flight {
    kind: Kind,
    serial: u64,
    sends: u8,
    usable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Waiting(Flight),
    Flying(Flight),
}

pub struct Requests {
    phase: Phase,
    next: Option<Kind>,
    serial: u64,
    /// Set when a menu opening closes. This is not an epoch bump.
    closed_opening: Option<u64>,
    /// Remains pending through delivery, until capture is actually armed.
    menu_unarmed: Option<u64>,
    /// The opening whose fighter menu owns the catalog mount; editor-only demands for it
    /// are dropped. Cleared by that opening's close or by retirement.
    menu_opening: Option<u64>,
}

impl Default for Requests {
    fn default() -> Self {
        Self::new()
    }
}

impl Requests {
    pub const fn new() -> Self {
        Self {
            phase: Phase::Idle,
            next: None,
            serial: 0,
            closed_opening: None,
            menu_unarmed: None,
            menu_opening: None,
        }
    }

    pub fn pending(&self) -> bool {
        self.next.is_some() || !matches!(self.phase, Phase::Idle)
    }

    pub fn serial(&self) -> u64 {
        self.serial
    }

    pub fn needs_retry(&self) -> bool {
        matches!(self.phase, Phase::Waiting(_))
    }

    pub fn allows_editor(&self, opening: u64, live: &Live) -> bool {
        self.wanted(Kind::Editor { opening }, live)
    }

    pub fn request(&mut self, kind: Kind) -> Effect {
        // A closed opening stays closed. Menu epochs are not reused, so this does not block the next open.
        if opening_of(kind).is_some_and(|opening| self.closed_opening == Some(opening)) {
            return Effect {
                dropped: Some(kind),
                ..Effect::quiet(Step::Wait)
            };
        }
        if let Kind::Menu { opening, .. } = kind {
            self.menu_unarmed = Some(opening);
            self.menu_opening = Some(opening);
            // An editor-only read already in flight for this opening must not mount.
            if let Phase::Flying(flight) = &mut self.phase {
                if flight.kind == (Kind::Editor { opening }) {
                    flight.usable = false;
                }
            }
        }
        if let Kind::Editor { opening } = kind {
            if self.menu_opening == Some(opening) {
                return Effect {
                    dropped: Some(kind),
                    ..Effect::quiet(Step::Wait)
                };
            }
        }
        let mut dropped = None;
        let step = match self.phase {
            Phase::Idle => self.begin(kind),
            Phase::Waiting(flight) => {
                if flight.kind != kind {
                    dropped = Some(flight.kind);
                }
                self.phase = Phase::Waiting(Flight { kind, ..flight });
                Step::Wait
            }
            Phase::Flying(flight) if flight.usable && flight.kind == kind => Step::Wait,
            Phase::Flying(_) => {
                let previous = self.next.replace(kind);
                if previous.is_some_and(|previous| previous != kind) {
                    dropped = previous;
                }
                Step::Wait
            }
        };
        Effect {
            dropped,
            ..Effect::quiet(step)
        }
    }

    pub fn rejected_full(&mut self) -> Effect {
        let Phase::Flying(flight) = self.phase else {
            return Effect::quiet(Step::Wait);
        };
        if flight.sends >= MAX_SENDS {
            self.phase = Phase::Idle;
            let dropped = self.next.take();
            return Effect {
                verdict: Verdict::Discard,
                step: Step::Unavailable,
                finished: Some(flight.kind),
                dropped,
            };
        }
        self.phase = Phase::Waiting(Flight {
            sends: flight.sends + 1,
            ..flight
        });
        Effect::quiet(Step::Wait)
    }

    pub fn retry(&mut self, live: Live) -> Effect {
        let Phase::Waiting(flight) = self.phase else {
            return Effect::quiet(Step::Wait);
        };
        if !flight.usable || !self.wanted(flight.kind, &live) {
            self.phase = Phase::Idle;
            return Effect {
                verdict: Verdict::Discard,
                step: Step::Unavailable,
                finished: Some(flight.kind),
                dropped: None,
            };
        }
        let sends = flight.sends;
        let step = self.begin(flight.kind);
        if let Phase::Flying(current) = &mut self.phase {
            current.sends = sends;
        }
        Effect::quiet(step)
    }

    pub fn complete(&mut self, serial: u64, live: Live) -> Effect {
        let Phase::Flying(flight) = self.phase else {
            return Effect::quiet(Step::Wait);
        };
        if flight.serial != serial {
            return Effect::quiet(Step::Wait);
        }
        self.phase = Phase::Idle;
        let verdict = if flight.usable && self.wanted(flight.kind, &live) {
            match flight.kind {
                Kind::Menu { opening, owners } => Verdict::Menu { opening, owners },
                Kind::Editor { opening } => Verdict::Editor { opening },
                Kind::Auto(token) => Verdict::Auto(token),
            }
        } else {
            Verdict::Discard
        };
        let (step, dropped) = self.take_next(&live);
        Effect {
            verdict,
            step,
            finished: Some(flight.kind),
            dropped,
        }
    }

    pub fn failed(&mut self, live: Live) -> Effect {
        let Phase::Flying(flight) = self.phase else {
            return Effect::quiet(Step::Wait);
        };
        self.phase = Phase::Idle;
        let (step, dropped) = self.take_next(&live);
        Effect {
            verdict: Verdict::Discard,
            step: if matches!(step, Step::Wait) {
                Step::Unavailable
            } else {
                step
            },
            finished: Some(flight.kind),
            dropped,
        }
    }

    /// A delivered snapshot is not yet applied. Keep auto-init recoverable until
    /// capture acknowledges it, including a close between completion and apply.
    pub fn menu_armed(&mut self, opening: u64) {
        if self.menu_unarmed == Some(opening) {
            self.menu_unarmed = None;
        }
    }

    /// Record a close without changing the selection epoch. Return whether the
    /// cancelled menu read still owes this lifetime an automatic capture.
    pub fn close_menu(&mut self, opening: u64) -> bool {
        let restart_auto = self.menu_unarmed == Some(opening);
        if restart_auto {
            self.menu_unarmed = None;
        }
        if self.menu_opening == Some(opening) {
            self.menu_opening = None;
        }
        self.closed_opening = Some(opening);
        if self.next.and_then(opening_of) == Some(opening) {
            self.next = None;
        }
        if let Phase::Waiting(flight) = self.phase {
            if opening_of(flight.kind) == Some(opening) {
                self.phase = Phase::Idle;
            }
        }
        if let Phase::Flying(flight) = &mut self.phase {
            if opening_of(flight.kind) == Some(opening) {
                flight.usable = false;
            }
        }
        restart_auto
    }

    /// Scene change and visible-entry destruction. The in-flight reply is still consumed once.
    pub fn retire(&mut self) {
        self.next = None;
        self.menu_unarmed = None;
        self.menu_opening = None;
        if let Phase::Waiting(_) = self.phase {
            self.phase = Phase::Idle;
        }
        if let Phase::Flying(flight) = &mut self.phase {
            flight.usable = false;
        }
    }

    fn begin(&mut self, kind: Kind) -> Step {
        self.serial = self.serial.wrapping_add(1);
        self.phase = Phase::Flying(Flight {
            kind,
            serial: self.serial,
            sends: 1,
            usable: true,
        });
        Step::Submit(kind)
    }

    fn take_next(&mut self, live: &Live) -> (Step, Option<Kind>) {
        let Some(kind) = self.next.take() else {
            return (Step::Wait, None);
        };
        if self.wanted(kind, live) {
            (self.begin(kind), None)
        } else {
            (Step::Wait, Some(kind))
        }
    }

    fn wanted(&self, kind: Kind, live: &Live) -> bool {
        match kind {
            Kind::Menu { opening, owners } => {
                live.menu_active
                    && self.closed_opening != Some(opening)
                    && live.opening == opening
                    && live.owners == owners
            }
            Kind::Editor { opening } => {
                live.menu_active && self.closed_opening != Some(opening) && live.opening == opening
            }
            Kind::Auto(token) => live.auto == Some(token),
        }
    }
}

fn opening_of(kind: Kind) -> Option<u64> {
    match kind {
        Kind::Menu { opening, .. } | Kind::Editor { opening } => Some(opening),
        Kind::Auto(_) => None,
    }
}

