//! Deferred replay resource identity. Native callbacks exchange boot-reserved values
//! with the existing file writer; catalog construction stays on that writer.
use super::*;
use std::sync::{mpsc, Arc, OnceLock};
use crate::model::{catalog_reply::{Job, Replies}, replay_jobs::{Identity, Queue, Work}, pooled_reply};
use crate::engine::mii::{
    draft::Library, inventory::Snapshot as Catalog, lifetime::Configuration,
    native_values::RowIds,
    replay::{RawMii, ReplayMiis, Selection, SelectionSource, Target},
};

const MAX_PENDING: usize = crate::engine::macros::MAX_RETAINED_CAPTURES * 8;
static STATE: Mutex<Option<State>> = Mutex::new(None);
type Outcome = Result<Configuration, Error>;
static REPLIES: OnceLock<pooled_reply::Pool<Outcome>> = OnceLock::new();
static REQUESTS: OnceLock<mpsc::SyncSender<Submission>> = OnceLock::new();
struct Submission {
    raw: RawMii,
    identity: Option<Identity>,
    reply: pooled_reply::Job<'static, Outcome>,
}

#[derive(Clone, Debug)]
pub(crate) enum Error { Native(&'static str), Worker(Arc<str>) }
impl Error {
    pub(crate) fn message(&self) -> &str {
        match self { Self::Native(s) => s, Self::Worker(s) => s }
    }
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { f.write_str(self.message()) }
}

pub(crate) struct Ticket {
    identity: Identity,
    reply: pooled_reply::Ticket<'static, Outcome>,
}
impl Ticket {
    pub(crate) fn try_recv(&self) -> Result<Outcome, mpsc::TryRecvError> {
        if self.identity != identity() { return Err(mpsc::TryRecvError::Disconnected); }
        match self.reply.try_recv() {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(())) => Err(mpsc::TryRecvError::Disconnected),
            Err(error) => Err(error),
        }
    }
}

pub(crate) struct CatalogMerge {
    expected: Arc<Catalog>,
    prepared: Option<Arc<Catalog>>,
}
/// One box allocated at startup and returned with every writer completion.
/// Contains owned values only; no native pointer crosses the worker boundary.
pub(crate) struct ImportPayload {
    pub(crate) configuration: Option<Configuration>,
    pub(crate) native: Option<Arc<Catalog>>,
    pub(crate) library: Option<Arc<Library>>,
    pub(crate) error: Option<Error>,
    updates: [Option<CatalogMerge>; 2],
}
impl ImportPayload {
    fn empty() -> Self {
        Self { configuration: None, native: None, library: None, error: None, updates: [None, None] }
    }
    /// Existing file worker only: cloning a catalog allocates.
    pub(crate) fn merge(&mut self, library: Arc<Library>) {
        for update in self.updates.iter_mut().flatten() {
            update.prepared = Some(Arc::new((*update.expected).clone().with_library(&library)));
        }
        self.library = Some(library);
    }
}
pub(crate) type ImportReply = Job<Box<ImportPayload>>;
pub(crate) enum ImportQueue {
    Queued,
    Full(Box<ImportPayload>, ImportReply),
    Unavailable(Box<ImportPayload>, ImportReply),
}
struct State {
    queue: Queue<Submission, ()>,
    requests: mpsc::Receiver<Submission>,
    active: Option<Work<Submission>>,
    payload: Option<Box<ImportPayload>>,
    replies: Replies<Box<ImportPayload>>,
    waiting: bool,
}
pub(super) fn init() {
    let mut guard = STATE.lock().unwrap();
    if guard.is_none() {
        let (sender, requests) = mpsc::sync_channel(MAX_PENDING + 1);
        let _ = REPLIES.set(pooled_reply::Pool::new(MAX_PENDING + 1));
        let _ = REQUESTS.set(sender);
        *guard = Some(State { queue: Queue::new(MAX_PENDING + 1), requests, active: None,
            payload: Some(Box::new(ImportPayload::empty())), replies: Replies::new(), waiting: false });
    }
}
fn generation() -> u64 { mii_saved_alpha::current_generation(1) }
fn identity() -> Identity {
    Identity { epoch: generation(), revision: crate::runtime::macros::replay_capture::selection_revision() }
}
impl State {
    fn finish(&mut self, result: Result<Configuration, Error>) {
        if let Some(work) = self.active.take() {
            work.request.reply.complete(result);
            self.queue.complete(work.ticket, ());
            let _ = self.queue.poll(work.ticket);
        }
        if let Some(payload) = self.payload.as_mut() { **payload = ImportPayload::empty(); }
    }
    fn submit_import(&mut self) {
        let Some(sender) = self.replies.begin() else { return; };
        let Some(payload) = self.payload.take() else {
            self.replies.rejected(sender);
            self.finish(Err(Error::Native("Mii preset owner unavailable")));
            return;
        };
        match crate::runtime::macros::replay_store::import_mii(payload, Job::new(sender)) {
            ImportQueue::Queued => self.waiting = true,
            ImportQueue::Full(payload, reply) => {
                self.payload = Some(payload); self.replies.rejected(reply.rejected());
            }
            ImportQueue::Unavailable(payload, reply) => {
                self.payload = Some(payload); self.replies.rejected(reply.rejected());
                self.finish(Err(Error::Native("Mii preset owner unavailable")));
            }
        }
    }
}

/// File worker only. Durable association remains authoritative if the bounded
/// optimization queue is full; a later selected capture resolves it again.
pub(crate) fn committed(miis: ReplayMiis) {
    let (Some(pool), Some(sender)) = (REPLIES.get(), REQUESTS.get()) else { return; };
    for raw in miis.entries() {
        let Some((ticket, reply)) = pool.claim() else { break; };
        drop(ticket); // durable companion has no selected reader
        if sender.try_send(Submission { raw: *raw, identity: None, reply }).is_err() { break; }
    }
}
pub(crate) fn resolve(raw: RawMii, revision: u64) -> Result<Ticket, Error> {
    if !mii_ordinary_cpu::enabled() { return Err(Error::Native("Replay Mii loading is unavailable in this build")); }
    let (Some(pool), Some(sender)) = (REPLIES.get(), REQUESTS.get()) else {
        return Err(Error::Native("Mii preset owner unavailable"));
    };
    let current = identity();
    if revision != current.revision { return Err(Error::Native("Replay selection retired")); }
    let Some((ticket, reply)) = pool.claim() else {
        return Err(Error::Native("Mii import queue is busy; reopen this capture"));
    };
    sender.try_send(Submission { raw, identity: Some(current), reply })
        .map_err(|_| Error::Native("Mii preset owner unavailable"))?;
    Ok(Ticket { identity: current, reply: ticket })
}
pub(crate) fn selected(target: Target, revision: u64) -> Selection {
    Selection {
        source: SelectionSource::Clip,
        generation: generation(),
        revision,
        target,
    }
}
/// Atomic-only query: safe while the existing ordinary CPU STATE lock is held.
pub(super) fn revision(source: SelectionSource) -> u64 {
    match source {
        SelectionSource::Clip => crate::runtime::macros::replay_capture::selection_revision(),
        SelectionSource::NamedMacro => crate::runtime::macros::named_macros::selection_revision(),
    }
}
fn named_selection(target: Target) -> Selection {
    Selection { source: SelectionSource::NamedMacro, generation: generation(),
        revision: revision(SelectionSource::NamedMacro), target }
}
pub(crate) fn reacknowledge_named(target: Target) {
    if target.configuration().is_some() {
        mii_ordinary_cpu::request_reacknowledge(named_selection(target));
    }
}
pub(crate) fn named_ready(target: Target, kind: u32) -> bool {
    mii_ordinary_cpu::replay_ready(named_selection(target), kind)
}
pub(crate) fn named_refusal(target: Target) -> Option<&'static str> {
    mii_ordinary_cpu::replay_refusal(named_selection(target))
}
pub(crate) fn request_named(target: Target) -> bool {
    if mii_ordinary_cpu::replay_catalog().is_some() {
        mii_ordinary_cpu::request_replay(named_selection(target))
    } else if target.configuration().is_none() { request_cpu_kind(target.kind()) }
    else { false }
}
pub(crate) fn refusal(target: Target, revision: u64) -> Option<&'static str> {
    mii_ordinary_cpu::replay_refusal(selected(target, revision))
}
pub(crate) fn ready(target: Target, kind: u32) -> bool {
    if target.configuration().is_some() {
        mii_ordinary_cpu::replay_ready(
            selected(target, crate::runtime::macros::replay_capture::selection_revision()),
            kind,
        )
    } else {
        target.ready(kind, mii_ordinary_cpu::replay_current())
    }
}
pub(crate) fn request(target: Target, revision: u64) -> bool {
    if mii_ordinary_cpu::replay_catalog().is_some() {
        mii_ordinary_cpu::request_replay(selected(target, revision))
    } else if target.configuration().is_none() {
        request_cpu_kind(target.kind())
    } else {
        false
    }
}


pub(super) unsafe fn service(entry: *const u8, owner: u32) {
    if mii_backend::cpu() != Some(owner) || !mii_ordinary_cpu::enabled() { return; }
    let Ok(mut guard) = STATE.try_lock() else { return; };
    let Some(state) = guard.as_mut() else { return; };
    let current = identity();
    state.queue.retire(current);
    for _ in 0..MAX_PENDING + 1 {
        let Ok(request) = state.requests.try_recv() else { break; };
        if request.identity.is_some_and(|id| id != current) { continue; }
        let id = request.identity;
        // Every accepted request owns one pool permit until finish/drop; pool
        // and queue have the same capacity, so adoption cannot exhaust slots.
        if let Err(request) = state.queue.submit(request, id) {
            request.reply.complete(Err(Error::Native("Mii import queue unavailable")));
        }
    }
    if state.waiting {
        match state.replies.poll() {
            Ok(Ok(payload)) => { state.payload = Some(payload); state.waiting = false; }
            Ok(Err(())) | Err(mpsc::TryRecvError::Disconnected) => {
                state.waiting = false;
                state.finish(Err(Error::Native("Mii import writer disconnected")));
            }
            Err(mpsc::TryRecvError::Empty) => return,
        }
    }
    if state.active.as_ref().is_some_and(|work| work.identity.is_some_and(|id| id != current)) {
        state.finish(Err(Error::Native("Replay selection retired")));
    }
    if state.active.is_some() {
        let Some(payload) = state.payload.as_mut() else {
            state.finish(Err(Error::Native("Mii preset owner unavailable"))); return;
        };
        if let Some(error) = payload.error.take() { state.finish(Err(error)); return; }
        if payload.library.is_some() {
            for role in 0..2 {
                let Some(update) = payload.updates[role].as_mut() else { continue; };
                let Some(prepared) = update.prepared.as_ref() else { continue; };
                let published = if role == 0 {
                    mii_ordinary_player::publish_library(&update.expected, prepared)
                } else { mii_ordinary_cpu::publish_library(&update.expected, prepared) };
                match published {
                    Err(()) => return, // retain the immutable completion on contention
                    Ok(true) => payload.updates[role] = None,
                    Ok(false) => {
                        let fresh = if role == 0 { mii_ordinary_player::library_catalog() }
                            else { mii_ordinary_cpu::library_catalog() };
                        let Ok(fresh) = fresh else { return; };
                        payload.updates[role] = fresh.map(|expected| CatalogMerge { expected, prepared: None });
                    }
                }
            }
            if payload.updates.iter().all(Option::is_none) {
                let configuration = payload.configuration.unwrap();
                state.finish(Ok(configuration));
            } else {
                // A newer catalog won the publication race. Re-merge on the
                // existing writer; the durable preset import is not repeated.
                state.submit_import();
                return;
            }
        } else { state.submit_import(); return; }
    }
    if !state.queue.has_work() { return; }
    // No native row scans until the original gameplay and transaction gates permit them.
    if !crate::runtime::compat::common::is_ready_go() || INSTALL.active() || menu::captures_input()
        || lobby::native_ui_busy() || button_config::has_pending_request()
        || training::save_states::is_loading() || training::recording_or_playback()
        || crate::runtime::compat::replay_preview::occupied() || !mii_backend::ready()
        || !(ready_single_mii(entry, owner).is_some() || ready_forms(entry, owner).is_some()) { return; }
    let Some(catalog) = mii_ordinary_cpu::replay_catalog() else { return; };
    let Ok(player) = mii_ordinary_player::library_catalog() else { return; };
    let Ok(cpu) = mii_ordinary_cpu::library_catalog() else { return; };
    let Some(work) = state.queue.take_next() else { return; };
    use crate::runtime::mii::mii_native_access::{lookup_row_id, HAT_TABLE, OUTFIT_TABLE};
    let tokens = work.request.raw.tokens();
    let resolved = (|| {
        let base = BASE.load(Ordering::Acquire);
        let rows = RowIds { outfit: lookup_row_id(base, OUTFIT_TABLE, tokens.outfit)?,
            hat: lookup_row_id(base, HAT_TABLE, tokens.hat)? };
        work.request.raw.resolve(rows).map_err(|_| "Replay Mii resource identity did not resolve")
    })();
    state.active = Some(work);
    match resolved {
        Err(error) => state.finish(Err(Error::Native(error))),
        Ok(configuration) => {
            let Some(payload) = state.payload.as_mut() else {
                state.finish(Err(Error::Native("Mii preset owner unavailable"))); return;
            };
            payload.configuration = Some(configuration);
            payload.native = Some(catalog);
            payload.updates = [player, cpu].map(|catalog| catalog.map(|expected| CatalogMerge { expected, prepared: None }));
            state.submit_import();
        }
    }
}
