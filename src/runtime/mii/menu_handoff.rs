//! Typed UI intent handoff. Only service() may inspect native facts or call
//! the existing saved-Mii/editor adapters; it runs in the roster callback.
use std::sync::atomic::{AtomicU32, Ordering};
use hos_lock::Mutex;
use crate::engine::mii_root::mii_menu::PendingSelection;
#[cfg(feature = "mii-editor-alpha")]
use crate::engine::mii_root::mii_menu::EditorRequest;
use crate::runtime::{ui::menu_app, roster::character_roster::mii_saved_alpha};

/// UI-owned values, produced after the CatalogMenu closes. Post before clearing
/// OPEN and calling note_open(). An ordinary close without selections needs no post.
pub struct CloseIntent {
    pub epoch: u64,
    pub selection: Option<PendingSelection>,
    #[cfg(feature = "mii-editor-alpha")]
    pub editor: Option<EditorRequest>,
}
static CLOSE: Mutex<Option<CloseIntent>> = Mutex::new(None);
static SEEN: AtomicU32 = AtomicU32::new(0);
static PREPARED: AtomicU32 = AtomicU32::new(0);

/// UI worker only. The native consumer uses try_lock and never waits for this.
pub fn post_close(intent: CloseIntent) {
    *CLOSE.lock() = Some(intent);
}

/// Native game thread only; one bounded transition, no channel creation or IO.
pub(crate) fn service() {
    let sequence = menu_app::sequence();
    let seen = SEEN.load(Ordering::Acquire);
    if sequence == seen { return; }
    let Some(mut close) = CLOSE.try_lock() else { return; };
    if SEEN.compare_exchange(seen, sequence, Ordering::AcqRel, Ordering::Acquire).is_err() { return; }
    let intent = close.take();
    drop(close);
    if menu_app::active() {
        // Coalesced close/reopen cancels the old opening before capturing new facts.
        mii_saved_alpha::menu_close(None);
        let opening = mii_saved_alpha::menu_open();
        let captured = mii_saved_alpha::menu_ready(opening);
        #[cfg(feature = "mii-editor-alpha")]
        if !captured { super::mii_editor_bridge::menu_ready(opening); }
        #[cfg(not(feature = "mii-editor-alpha"))]
        let _ = captured;
    } else {
        let intent = intent.filter(|i| i.epoch == mii_saved_alpha::menu_epoch());
        if let Some(intent) = intent {
            mii_saved_alpha::menu_close(intent.selection);
            #[cfg(feature = "mii-editor-alpha")]
            if let Some(editor) = intent.editor { super::mii_editor_bridge::request(editor); }
        } else {
            mii_saved_alpha::menu_close(None);
        }
    }
    PREPARED.store(sequence, Ordering::Release);
}

/// Publications belong only to an opening already observed on the native side.
pub(crate) fn opening_current(sequence: u32) -> bool {
    PREPARED.load(Ordering::Acquire) == sequence
}
