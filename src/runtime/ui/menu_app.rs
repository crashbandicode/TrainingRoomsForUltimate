//! The menu as package 6's Mii files see it (`compat::menu_app`: the fork's `QUICK_MENU_APP`, `AppPage`,
//! `MENU_RECEIVED_INPUT`). The Mii saved-catalog and editor-bridge code publishes an owned catalog into the open menu
//! and asks for the Mii list to show it; this holder keeps what it published and whether the menu is open, and the menu
//! worker takes the catalog when it draws the Mii list screen (`Editor::MiiList`).
//!
//! Catalog construction and deep copies belong to the UI worker. Native service
//! publishes only this typed, value-owned handoff and uses try_lock.

use std::sync::Arc;
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
    catalog: Option<CatalogPublication>,
    seen_open: u32,
}

impl App {
    const fn new() -> App {
        App { page: AppPage::CLOSED, catalog: None, seen_open: 0 }
    }

    /// Native producer calls this only after try_lock succeeds. No boxing or
    /// snapshot clone; a newer opening always supersedes an older publication.
    pub fn publish_catalog(&mut self, publication: CatalogPublication) -> bool {
        if !active() || self.catalog.as_ref().is_some_and(|held| held.epoch > publication.epoch) {
            return false;
        }
        self.catalog = Some(publication);
        true
    }

    /// UI worker: take once, release the guard, then construct CatalogMenu.
    pub fn take_catalog(&mut self) -> Option<CatalogPublication> {
        self.catalog.take()
    }

    /// Catch up with the menu's opens and closes (`note_open`): the page follows the open flag, and a close since the last look drops
    /// what was published and not yet shown. Run by every holder of `QUICK_MENU_APP` when it takes the lock (`compat::sync`), and by the UI worker. Native producers use try_lock, so they never wait on the worker.
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

    /// The menu opened or closed: closing drops what was published and not yet shown.
    pub fn set_open(&mut self, open: bool) {
        self.page = if open { AppPage::SUBMENU } else { AppPage::CLOSED };
        if !open {
            self.catalog = None;
        }
    }
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
