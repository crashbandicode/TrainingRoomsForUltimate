//! Copy-only bridge to the existing roster identity provider.
//! The worker never takes the native provider's state locks.
use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};
use crate::engine::mii::{lifetime::Configuration, replay::Target};
use super::{named_macros, replay_capture};
use crate::runtime::roster::character_roster as roster;

#[derive(Clone, Copy, PartialEq)]
struct Key { session: u64, named_revision: u64, clip_revision: u64, named: Option<Target>, clip: Option<Target> }
#[derive(Clone, Copy)]
struct Facts { key: Key, kind: Option<i32>, configuration: Option<Configuration>,
    named_ready: bool, clip_ready: bool, named_refusal: Option<&'static str>, clip_refusal: Option<&'static str>,
    /// #54, reading B: the named and clip targets as Switch installs them (`mii_install::installed_target`).
    named_installed: Option<Target>, clip_installed: Option<Target> }
struct Worker { named: Option<Target>, clip: Option<Target>, facts: Option<Facts> }
static WORKER: Mutex<Worker> = Mutex::new(Worker { named: None, clip: None, facts: None });
static REQUEST: hos_lock::Mailbox<Key, 1> = hos_lock::Mailbox::new();
static RESULT: hos_lock::Mailbox<Facts, 1> = hos_lock::Mailbox::new();
static IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static REACK: hos_lock::Mailbox<(Target, u64, u64), 32> = hos_lock::Mailbox::new();

fn key(worker: &Worker) -> Key {
    Key { session: named_macros::session_revision(), named_revision: named_macros::selection_revision(),
        clip_revision: replay_capture::selection_revision(), named: worker.named, clip: worker.clip }
}

/// Worker tick: consume the one reply before allowing another request.
pub fn worker_tick() {
    let dropped = REACK.take_dropped();
    if dropped != 0 { crate::runtime::diag_log::diagnostic(format_args!("[MacroHandoff] identity_reack_dropped={dropped}")); }
    RESULT.drain(|facts| {
        WORKER.lock().unwrap().facts = Some(facts);
        IN_FLIGHT.store(false, Ordering::Release);
    });
    if !IN_FLIGHT.swap(true, Ordering::AcqRel) {
        let request = key(&WORKER.lock().unwrap());
        let posted = REQUEST.post(request);
        debug_assert!(posted, "one identity query in flight");
    }
}

fn facts(worker: &Worker) -> Option<Facts> {
    worker.facts.filter(|facts| !roster::busy() && facts.key.session == named_macros::session_revision()
        && facts.kind == crate::runtime::ui::facts::fact_cpu_kind())
}

pub fn current_configuration() -> Option<Configuration> {
    facts(&WORKER.lock().unwrap()).and_then(|facts| facts.configuration)
}

/// #54, reading B (primary 2026-11-08 23:03Z): `target` as Switch installs it, when the game callback has worked it out
/// for the current named or clip selection. Reads the copy only; never asks for another target.
pub fn installed(target: Target) -> Option<Target> {
    let worker = WORKER.lock().unwrap();
    facts(&worker).and_then(|facts| if facts.key.named == Some(target) {facts.named_installed}
        else if facts.key.clip == Some(target) {facts.clip_installed} else {None})
}

pub fn named_ready(target: Target) -> bool {
    let mut worker = WORKER.lock().unwrap();
    worker.named = Some(target);
    facts(&worker).is_some_and(|facts| facts.key.named == Some(target)
        && facts.key.named_revision == named_macros::selection_revision() && facts.named_ready)
}
pub fn clip_ready(target: Target) -> bool {
    let mut worker = WORKER.lock().unwrap();
    worker.clip = Some(target);
    facts(&worker).is_some_and(|facts| facts.key.clip == Some(target)
        && facts.key.clip_revision == replay_capture::selection_revision() && facts.clip_ready)
}
pub fn named_refusal(target: Target) -> Option<&'static str> {
    let mut worker = WORKER.lock().unwrap();
    worker.named = Some(target);
    facts(&worker).filter(|facts| facts.key.named == Some(target)
        && facts.key.named_revision == named_macros::selection_revision()).and_then(|facts| facts.named_refusal)
}
pub fn clip_refusal(target: Target, revision: u64) -> Option<&'static str> {
    let mut worker = WORKER.lock().unwrap();
    worker.clip = Some(target);
    facts(&worker).filter(|facts| facts.key.clip == Some(target)
        && facts.key.clip_revision == revision).and_then(|facts| facts.clip_refusal)
}
pub fn reacknowledge_named(target: Target) {
    REACK.post((target, named_macros::session_revision(), named_macros::selection_revision()));
}

/// Existing game callback only; the roster remains the sole provider.
pub fn service() {
    REACK.drain(|(target, session, revision)| {
        if session == named_macros::session_revision() && revision == named_macros::selection_revision() {
            roster::mii_replay::reacknowledge_named(target);
        }
    });
    REQUEST.drain(|key| {
        let kind = crate::runtime::ui::facts::fact_cpu_kind();
        let current = key.session == named_macros::session_revision();
        let mut facts = Facts { key, kind, configuration: None, named_ready: false, clip_ready: false,
            named_refusal: None, clip_refusal: None, named_installed: None, clip_installed: None };
        if current {
            facts.configuration = roster::current_mii_configuration(1);
            {
                facts.named_installed = key.named.and_then(roster::mii_replay::installed_target);
                facts.clip_installed = key.clip.and_then(roster::mii_replay::installed_target);
                if key.named_revision == named_macros::selection_revision() {
                    if let Some(target) = key.named {
                        facts.named_ready = kind.is_some_and(|kind| roster::mii_replay::named_ready(target, kind as u32));
                        facts.named_refusal = roster::mii_replay::named_refusal(target);
                    }
                }
                if key.clip_revision == replay_capture::selection_revision() {
                    if let Some(target) = key.clip {
                        facts.clip_ready = kind.is_some_and(|kind| roster::mii_replay::ready(target, kind as u32));
                        facts.clip_refusal = roster::mii_replay::refusal(target, key.clip_revision);
                    }
                }
            }
        }
        let posted = RESULT.post(facts);
        debug_assert!(posted, "one identity reply in flight");
        crate::runtime::ui::worker::wake();
    });
}
