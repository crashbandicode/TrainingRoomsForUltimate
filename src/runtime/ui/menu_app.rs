//! Native menu-page compatibility and an explicit publication channel to the UI.
//! Catalogs never live inside the hook-owned App; only the UI worker receives
//! publications and constructs the catalog menu.

use std::sync::Arc;
#[cfg(feature = "mii-saved-alpha")]
use std::sync::OnceLock;
#[cfg(feature = "mii-saved-alpha")]
use crate::model::catalog_mailbox::{self, Producer, Consumer, Publication};
use crate::engine::{mii::inventory::Snapshot, mii_root::mii_menu::Context};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use hos_lock::Mutex;


/// Which page of the menu is showing, as the Mii code asks: only "the menu's own page list is up" matters to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppPage(u8);

impl AppPage {
    /// The menu is closed.
    pub const CLOSED: AppPage = AppPage(0);
    /// The menu is open on its pages (the fork's submenu page): a catalog can be mounted into it.
    pub const SUBMENU: AppPage = AppPage(1);
}

/// Worker-built catalog plus facts captured on the native game thread.
pub struct CatalogPublication {
    pub catalog: Arc<Snapshot>,
    pub context: Option<Context>,
    pub epoch: u64,
    pub ordinary_cpu: bool,
}

/// The menu's pending native publication. It owns no game pointers.
pub struct App {
    pub page: AppPage,
    seen_open: u32,
}

impl App {
    const fn new() -> App {
        App { page: AppPage::CLOSED, seen_open: 0 }
    }

    /// Native compatibility holders follow the worker's atomic open sequence.
    pub fn follow_open(&mut self) {
        let seq = OPEN_SEQ.load(Ordering::Acquire);
        if seq == self.seen_open {
            return;
        }
        let open = active();
        // An odd distance or a closed menu means a close happened since the last look.
        if !open || seq.wrapping_sub(self.seen_open) > 1 {
            self.set_open(false);
        }
        if open {
            self.set_open(true);
        }
        self.seen_open = seq;
    }

    /// Native page compatibility follows menu state; publications are separate.
    pub fn set_open(&mut self, open: bool) {
        self.page = if open { AppPage::SUBMENU } else { AppPage::CLOSED };
    }
}


#[cfg(feature = "mii-saved-alpha")]
const PUBLICATION_CAPACITY: usize = 2;
#[cfg(feature = "mii-saved-alpha")]
static PUBLICATIONS: OnceLock<Producer<CatalogPublication>> = OnceLock::new();
// Only the UI worker takes this guard. Native producers hold no reference to it.
#[cfg(feature = "mii-saved-alpha")]
static CATALOG_READER: OnceLock<Mutex<Consumer<CatalogPublication>>> = OnceLock::new();

/// Boot only, before native callbacks can publish.
#[cfg(feature = "mii-saved-alpha")]
pub(crate) fn init_publications() {
    if PUBLICATIONS.get().is_some() { return; }
    let (producer, consumer) = catalog_mailbox::channel(PUBLICATION_CAPACITY);
    let _ = CATALOG_READER.set(Mutex::new(consumer));
    let _ = PUBLICATIONS.set(producer);
}

/// Native producer. Failed submission returns identical ownership to its policy.
#[cfg(feature = "mii-saved-alpha")]
pub(crate) fn publish_catalog(publication: CatalogPublication) -> Result<(), CatalogPublication> {
    let sequence = sequence();
    if !active() || publication.epoch != crate::runtime::roster::character_roster::mii_saved_alpha::menu_epoch()
        || !crate::runtime::mii::menu_handoff::opening_current(sequence) { return Err(publication); }
    let Some(producer) = PUBLICATIONS.get() else { return Err(publication); };
    producer.post(Publication { sequence, epoch: publication.epoch, value: publication })
        .map_err(|held| held.value)
}

/// UI worker only. Never takes the native App guard. Reject a close/reopen or
/// epoch retirement both before and after this bounded drain.
#[cfg(feature = "mii-saved-alpha")]
pub(crate) fn take_catalog() -> Option<CatalogPublication> {
    let sequence = sequence();
    let epoch = crate::runtime::roster::character_roster::mii_saved_alpha::menu_epoch();
    let open = active();
    let publication = CATALOG_READER.get()?.lock().take(sequence, epoch, open);
    if !open || !active() || sequence != self::sequence()
        || epoch != crate::runtime::roster::character_roster::mii_saved_alpha::menu_epoch() { return None; }
    publication
}

pub static QUICK_MENU_APP: Mutex<App> = Mutex::new(App::new());
/// Bumped by the menu worker at every open and every close (after `ui::state::OPEN` changed); `App::follow_open` catches up.
static OPEN_SEQ: AtomicU32 = AtomicU32::new(0);

/// The menu worker's side of an open or a close: one atomic, no lock (`App::follow_open`).
pub fn note_open() {
    OPEN_SEQ.fetch_add(1, Ordering::AcqRel);
}
/// The Mii code's "the menu got a catalog or an input since the last check" flag.
pub static MENU_RECEIVED_INPUT: AtomicBool = AtomicBool::new(false);

/// Whether the menu is open now (the fork's `quick_menu_active`).
pub fn active() -> bool {
    crate::runtime::ui::state::OPEN.load(Ordering::Acquire)
}

/// Whether the menu owns Player 1's input now, open or closing (the fork's `captures_input`).
pub fn captures_input() -> bool {
    crate::runtime::ui::state::readiness_facts(0).owns_input
}

/// Native handoff observes the same opening/closing sequence as App.
pub(crate) fn sequence() -> u32 { OPEN_SEQ.load(Ordering::Acquire) }
