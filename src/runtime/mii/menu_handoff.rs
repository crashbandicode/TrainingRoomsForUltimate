//! Typed UI intent handoff. Only service() may inspect native facts or call
//! the existing saved-Mii/editor adapters; it runs in the roster callback.
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use hos_lock::Mutex;
use crate::model::close_handoff::{self, Consumer, Intent, Producer};
use crate::engine::mii_root::mii_menu::PendingSelection;
#[cfg(feature = "mii-editor-alpha")]
use crate::engine::mii_root::mii_menu::EditorRequest;
use crate::runtime::{ui::menu_app, roster::character_roster::mii_saved_alpha};

/// UI-owned values, produced after the CatalogMenu closes. Post before clearing
/// OPEN and calling note_open(). An ordinary close without selections needs no post.
pub struct CloseIntent {
    pub opening: u32,
    pub epoch: u64,
    pub selection: Option<PendingSelection>,
    #[cfg(feature = "mii-editor-alpha")]
    pub editor: Option<EditorRequest>,
}
static CLOSE: OnceLock<Producer<CloseIntent>> = OnceLock::new();
// Native callback only. The UI producer never acquires or accesses this guard.
static READER: OnceLock<Mutex<Consumer<CloseIntent>>> = OnceLock::new();
static SEEN: AtomicU32 = AtomicU32::new(0);
static PREPARED: AtomicU32 = AtomicU32::new(0);

/// Boot only, before callbacks and the UI worker can submit intents.
pub(crate) fn init() {
    if CLOSE.get().is_some() { return; }
    let (producer, consumer) = close_handoff::channel(2);
    let _ = READER.set(Mutex::new(consumer));
    let _ = CLOSE.set(producer);
}

/// UI worker only. Retain a rejected intent and retry before clearing OPEN.
pub fn post_close(intent: CloseIntent) -> Result<(), CloseIntent> {
    let Some(producer) = CLOSE.get() else { return Err(intent); };
    producer.post(Intent { opening: intent.opening, epoch: intent.epoch, value: intent })
        .map_err(|returned| returned.value)
}

/// Native game thread only; one bounded transition, no channel creation or IO.
pub(crate) fn service() {
    let sequence = menu_app::sequence();
    let open = menu_app::active();
    let seen = SEEN.load(Ordering::Acquire);
    let Some(reader) = READER.get() else { return; };
    let Some(mut reader) = reader.try_lock() else { return; };
    // Drain stale values even without a transition, freeing bounded capacity for
    // a UI retry. Current values stay owned by the reader until their close.
    let intent = reader.observe(sequence, mii_saved_alpha::menu_epoch(), open);
    if sequence != menu_app::sequence() || open != menu_app::active() { return; }
    if sequence == seen { return; }
    if SEEN.compare_exchange(seen, sequence, Ordering::AcqRel, Ordering::Acquire).is_err() { return; }
    drop(reader);
    if open {
        // Coalesced close/reopen cancels the old opening before capturing new facts.
        mii_saved_alpha::menu_close(None);
        let opening = mii_saved_alpha::menu_open();
        let captured = mii_saved_alpha::menu_ready(opening);
        #[cfg(feature = "mii-editor-alpha")]
        let requested = captured || super::mii_editor_bridge::menu_ready(opening);
        #[cfg(not(feature = "mii-editor-alpha"))]
        let requested = captured;
        if !requested { menu_app::note_no_catalog(sequence); }
    } else {
        let intent = intent.filter(|i| i.epoch == mii_saved_alpha::menu_epoch());
        if let Some(intent) = intent {
            // The fork's order (`common/menu.rs` 486-491): the editor request first, then the catalog close.
            #[cfg(feature = "mii-editor-alpha")]
            if let Some(editor) = intent.editor { super::mii_editor_bridge::request(editor); }
            mii_saved_alpha::menu_close(intent.selection);
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
