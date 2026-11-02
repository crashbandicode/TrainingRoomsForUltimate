//! INTERFACE STUB (package 4 provides the real module: the replay store under `crate::runtime::macros`; the manifest key
//! `modules.root.replay_store` is repointed when it lands). The fork's `replay_store`, as far as the Mii catalog request and the Mii
//! replay import call it.
//!
//! Without package 4's store there is nothing to read the Mii catalog from or import into, so both requests answer
//! "unavailable", which the callers already handle (their own refusal notice).

use std::sync::mpsc::{Sender, SyncSender};

/// The number of live captures the store keeps (package 4: `model::macros::MAX_RETAINED_CAPTURES`).
pub const MAX_RETAINED_CAPTURES: usize = 4;

/// How the store took a catalog request.
pub enum CatalogQueue {
    Queued,
    Full,
    Unavailable,
}

/// Queue a Mii catalog read for the replay store's worker (the answer goes through `sender`).
pub fn request_mii_catalog<T>(_sender: SyncSender<T>) -> CatalogQueue {
    CatalogQueue::Unavailable
}

/// Queue an import of one Mii configuration into the replay store (the answer goes through `reply`).
pub fn import_mii<C, K, R>(_configuration: C, _catalog: K, _reply: Sender<R>) -> Result<(), String> {
    Err("Replay store unavailable".into())
}