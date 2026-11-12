//! Persistent replay-capture storage and the live-capture preference.
//!
//! The INI is the single startup authority: it seeds the runtime preference once
//! and is never read while sampling. With the `deluxe-sdk` feature and a Deluxe
//! host present, the Plugins page changes the in-memory preference at runtime
//! (`runtime::deluxe_row`); the plugin's own writer thread rewrites the INI from
//! that preference (`features::writer`). Native save success supplies the replay identity;
//! this module never guesses it from a filename appearing on disk or a
//! controller button press.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
// rc.8 hang: game threads post jobs here; std's `try_send` could spin on the writer mid-read.
use crate::model::nowait_channel::{self as mpsc, SyncSender};

use {crate::engine::macros::recover_previous_ini, crate::engine::macros::CaptureArchive, crate::engine::macros::CaptureSettings, crate::engine::macros::CaptureToggle, crate::engine::macros::IniRecovery, crate::engine::macros::LiveSession, crate::engine::macros::SharedCaptureArchive, crate::engine::macros::DEFAULT_CAPTURE_INI, crate::engine::macros::MAX_RETAINED_CAPTURES};

/// The file Training reads at startup and the Deluxe host rewrites after menu
/// edits. Spelled out because the plugin descriptor needs a `'static` literal.
pub const REPLAY_CAPTURE_INI: &str = "sd:/ultimate/TrainingRooms/replay_capture.ini";
const _: () = assert!(
    path_under_root(REPLAY_CAPTURE_INI, crate::model::storage::ROOT, "/replay_capture.ini"),
    "REPLAY_CAPTURE_INI must be TRAINING_MODPACK_ROOT + /replay_capture.ini"
);

const fn path_under_root(path: &str, root: &str, suffix: &str) -> bool {
    let (path, root, suffix) = (path.as_bytes(), root.as_bytes(), suffix.as_bytes());
    if path.len() != root.len() + suffix.len() { return false; }
    let mut i = 0;
    while i < path.len() {
        let expected = if i < root.len() { root[i] } else { suffix[i - root.len()] };
        if path[i] != expected { return false; }
        i += 1;
    }
    true
}

/// INI value read at startup; `toggle()` holds the runtime preference.
static SEED_ENABLED: AtomicBool = AtomicBool::new(false);
static TOGGLE: CaptureToggle = CaptureToggle::new();
static WRITER: OnceLock<SyncSender<WriteJob>> = OnceLock::new();

struct CommittedCapture {
    replay_id: String,
    replay_name: String,
    session: Arc<LiveSession>,
    miis: Option<crate::engine::mii::replay::ReplayMiis>,
}

enum WriteJob {
    Capture(Box<CommittedCapture>),
    ReadCatalog(CatalogJob),
    // The box and reply endpoint are reserved at boot and returned after each job.
    Import(Box<crate::runtime::roster::character_roster::mii_replay::ImportPayload>,
        crate::runtime::roster::character_roster::mii_replay::ImportReply),
}

pub fn archive() -> CaptureArchive {
    CaptureArchive::new(format!(
        "{}/replay_captures",
        crate::model::storage::ROOT
    ))
}

/// Packed runtime preference and record latch, shared by the Deluxe row (draw
/// thread) and the native adapter (game thread).
pub fn toggle() -> &'static CaptureToggle {
    &TOGGLE
}

/// What the INI said at startup. Runtime edits live in [`toggle`].
pub fn seed_enabled() -> bool {
    SEED_ENABLED.load(Ordering::Acquire)
}

/// Start once for the mod library (combined alpha) or after live-capture guards.
/// A single bounded writer serializes publication; fighter callbacks never perform I/O.
pub fn start_writer() -> Result<(), String> {
    if WRITER.get().is_some() { return Ok(()); }
    let (sender, receiver) = mpsc::sync_channel::<WriteJob>(MAX_RETAINED_CAPTURES);
    crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::ReplaySave, move || {
            let archive = SharedCaptureArchive::new(format!("{}/replay_captures",
                crate::model::storage::ROOT));
            // The same serialized owner is the only writer of mod presets. Native saves are read-only.
            let mut presets = crate::engine::mii::draft_store::Store::open(
                crate::model::storage::ROOT).map_err(|error| error.to_string());
            while let Ok(message) = receiver.recv() {
                match message {
                    WriteJob::Capture(saved) => {
                        match archive.save_with_miis(&saved.replay_id, &saved.replay_name, "13.0.5",
                            &saved.session, saved.miis.as_ref()) {
                            Ok(outcome) => {
                                println!("[ReplayCapture] {} id={} name={:?}",
                                    if outcome.created { "persisted" } else { "already_present" },
                                    outcome.id, saved.replay_name);
                                if let Some(miis) = saved.miis {
                                    crate::runtime::roster::character_roster::mii_replay::committed(miis);
                                }
                            }
                            Err(error) => println!("[ReplayCapture] persistence_failed id={:?}: {}",
                                saved.replay_id, error),
                        }
                    }
                    WriteJob::ReadCatalog(reply) => {
                        // Stable native header reads stay in saved::read_catalog. Preset merge stays
                        // on this writer, after any import already queued ahead of this job.
                        let mut native = read_saved_mii_catalog();
                        match &presets {
                            Ok(store) => native = native.with_library(store.library()),
                            Err(error) => native.findings.push(crate::engine::mii::inventory::Finding::PresetSource {
                                error: format!("Mii preset library: {error}") }),
                        }
                        reply.complete(Arc::new(native));
                    }
                    WriteJob::Import(mut payload, reply) => {
                        use crate::runtime::roster::character_roster::mii_replay::Error;
                        let result = if let Some(library) = payload.library.as_ref() {
                            Ok(Arc::clone(library))
                        } else {
                            presets.as_mut().map_err(|error| error.clone()).and_then(|store| {
                                let configuration = payload.configuration.expect("native import configuration");
                                let name = format!("Replay {:?}", configuration.canonical().class);
                                store.import(configuration, &name, payload.native.as_ref().unwrap())
                                    .map_err(|error| error.to_string())?;
                                Ok(Arc::new(store.library().clone()))
                            })
                        };
                        match result {
                            Ok(library) => payload.merge(library),
                            Err(error) => payload.error = Some(Error::Worker(Arc::from(error))),
                        }
                        reply.complete(payload);
                    }
                }
            }
        }).map_err(|error| error.to_string())?;
    // Startup owns initialization. Dropping a redundant sender also closes its
    // receiver, so even a repeated init cannot leave an idle extra worker behind.
    let _ = WRITER.set(sender);
    Ok(())
}

/// Only the verified committed-save boundary may submit a value-owned snapshot.
/// No native actor, string, or fighter pointer is sent to the background thread.
pub fn persist_committed(replay_id: String, replay_name: String, session: Arc<LiveSession>,
    miis: Option<crate::engine::mii::replay::ReplayMiis>) {
    let Some(writer) = WRITER.get() else {
        println!("[ReplayCapture] persistence_writer_unavailable");
        return;
    };
    if let Err(error) = writer.try_send(WriteJob::Capture(Box::new(CommittedCapture { replay_id, replay_name, session, miis }))) {
        println!("[ReplayCapture] persistence_queue_unavailable: {}", error);
    }
}

/// Queue one owned catalog read. Never blocks the caller on the writer or the filesystem.
pub(crate) enum CatalogQueue {
    Queued,
    Full(CatalogJob),
    Unavailable(CatalogJob),
}

pub(crate) type CatalogJob = crate::model::catalog_reply::Job<Arc<crate::engine::mii::inventory::Snapshot>>;
static CATALOG_FALLBACK: OnceLock<SyncSender<CatalogJob>> = OnceLock::new();

fn read_saved_mii_catalog() -> crate::engine::mii::inventory::Snapshot {
    crate::engine::mii::saved::read_catalog(Path::new("save:/save_data/mii"))
}

pub(crate) fn request_mii_catalog(
    reply: CatalogJob,
) -> CatalogQueue {
    if let Some(writer) = WRITER.get() {
        return match writer.try_send(WriteJob::ReadCatalog(reply)) {
            Ok(()) => CatalogQueue::Queued,
            Err(mpsc::TrySendError::Full(WriteJob::ReadCatalog(reply))) => CatalogQueue::Full(reply),
            Err(mpsc::TrySendError::Disconnected(WriteJob::ReadCatalog(reply))) => {
                fallback_catalog(reply)
            }
            Err(_) => unreachable!("catalog submission returned another job variant"),
        };
    }
    fallback_catalog(reply)
}

/// One permanent 128 KiB reader used only when the shared replay writer cannot take the job.
/// Saved-only builds have no preset writer; a dead writer still returns the native catalog.
fn init_catalog_fallback() {
    CATALOG_FALLBACK.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<CatalogJob>(1);
        let _ = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::MiiCatalog, move || {
                while let Ok(reply) = receiver.recv() {
                    let native = read_saved_mii_catalog();
                    let native = {
                        let mut native = native;
                        native.findings.push(crate::engine::mii::inventory::Finding::PresetSource {
                            error: "Mii preset owner unavailable".into(),
                        });
                        native
                    };
                    reply.complete(Arc::new(native));
                }
            });
        // A failed spawn drops the receiver. Later sends disconnect once and do not retry forever.
        sender
    });
}

fn fallback_catalog(reply: CatalogJob) -> CatalogQueue {
    let Some(sender) = CATALOG_FALLBACK.get() else {
        return CatalogQueue::Unavailable(reply);
    };
    match sender.try_send(reply) {
        Ok(()) => CatalogQueue::Queued,
        Err(mpsc::TrySendError::Full(reply)) => CatalogQueue::Full(reply),
        Err(mpsc::TrySendError::Disconnected(reply)) => CatalogQueue::Unavailable(reply),
    }
}

pub(crate) fn import_mii(
    payload: Box<crate::runtime::roster::character_roster::mii_replay::ImportPayload>,
    reply: crate::runtime::roster::character_roster::mii_replay::ImportReply,
) -> crate::runtime::roster::character_roster::mii_replay::ImportQueue {
    use crate::runtime::roster::character_roster::mii_replay::ImportQueue;
    let Some(writer) = WRITER.get() else { return ImportQueue::Unavailable(payload, reply); };
    match writer.try_send(WriteJob::Import(payload, reply)) {
        Ok(()) => ImportQueue::Queued,
        Err(mpsc::TrySendError::Full(WriteJob::Import(payload, reply))) => ImportQueue::Full(payload, reply),
        Err(mpsc::TrySendError::Disconnected(WriteJob::Import(payload, reply))) => ImportQueue::Unavailable(payload, reply),
        Err(_) => unreachable!("import submission returned another job variant"),
    }
}

/// Called after the Training data directory and legacy migration are ready, and
/// before the Deluxe row registers. Configuration is read once; capture
/// sampling performs no filesystem reads.
pub fn init() {
    init_catalog_fallback();
    if let Err(error) = start_writer() { println!("[ReplayCapture] Mii library unavailable: {error}"); }
    let path = REPLAY_CAPTURE_INI;
    let result = (|| -> Result<CaptureSettings, String> {
        // The host persists by rename; a crash between its renames leaves only
        // `replay_capture.previous.ini`. Restore it before creating a default.
        match recover_previous_ini(Path::new(path)) {
            Ok(IniRecovery::Recovered) => println!("[ReplayCapture] replay_capture.ini restored from replay_capture.previous.ini"),
            Ok(IniRecovery::Present | IniRecovery::Absent) => {}
            Err(error) => println!("[ReplayCapture] replay_capture.previous.ini recovery skipped: {error}"),
        }
        match OpenOptions::new().write(true).create(true).create_new(true).truncate(false).open(path) {
            Ok(mut file) => file
                .write_all(DEFAULT_CAPTURE_INI.as_bytes())
                .and_then(|()| file.sync_all())
                .map_err(|error| error.to_string())?,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.to_string()),
        }
        if fs::metadata(path).map_err(|error| error.to_string())?.len() > 16 * 1024 {
            return Err("replay_capture.ini exceeds 16 KiB".into());
        }
        let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
        CaptureSettings::parse(&text)
    })();
    match result {
        Ok(settings) => {
            SEED_ENABLED.store(settings.enabled, Ordering::Release);
            TOGGLE.seed(settings.enabled);
            crate::runtime::diag_log::diagnostic(format_args!("[ReplayCapture] live_enabled={}", settings.enabled));
        }
        Err(error) => {
            SEED_ENABLED.store(false, Ordering::Release);
            TOGGLE.seed(false);
            crate::runtime::diag_log::diagnostic(format_args!("[ReplayCapture] live capture disabled: {error}"));
        }
    }
}
