//! The menu as package 6's Mii files see it (`compat::menu_app`: the fork's `QUICK_MENU_APP`, `AppPage`,
//! `MENU_RECEIVED_INPUT`). The Mii saved-catalog and editor-bridge code publishes an owned catalog into the open menu
//! and asks for the Mii list to show it; this holder keeps what it published and whether the menu is open, and the menu
//! worker takes the catalog when it draws the Mii list screen (`Editor::MiiList`).
//!
//! Status (2026-11-02): the holder, the page tracking and the signatures are real; the Mii list screen that consumes
//! `take_catalog` is not built yet (the row opens the "arrives with the package that owns it" placeholder), so a mount is
//! accepted while the menu is open and kept until the screen exists or the menu closes.

use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;


/// Which page of the menu is showing, as the Mii code asks: only "the menu's own page list is up" matters to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppPage(u8);

impl AppPage {
    /// The menu is closed.
    pub const CLOSED: AppPage = AppPage(0);
    /// The menu is open on its pages (the fork's submenu page): a catalog can be mounted into it.
    pub const SUBMENU: AppPage = AppPage(1);
}

/// What the Mii code last published into the menu.
pub struct App {
    pub page: AppPage,
    /// The published catalog (package 6's ngine::mii::inventory::Snapshot, which is behind a feature this module does
    /// not depend on: held type-erased, the Mii list screen downcasts it).
    catalog: Option<(Box<dyn Any + Send>, u64)>,
    /// The owner context that came with the catalog (the Mii menu's `Context`, owned by package 6).
    context: Option<Box<dyn Any + Send>>,
    ordinary_cpu: bool,
}

impl App {
    const fn new() -> App {
        App { page: AppPage::CLOSED, catalog: None, context: None, ordinary_cpu: false }
    }

    /// Mount an owned Mii catalog for the open Mii list (the editor entry). `false` when the menu is closed or the
    /// epoch is stale (an older capture finished after a newer opening).
    pub fn mount_mii_editor_catalog<S: Any + Send>(&mut self, catalog: S, epoch: u64) -> bool {
        // The menu's open flag is the truth (it is also cleared when the renderer is lost, R2-2); page follows the worker.
        if !active() || self.catalog.as_ref().is_some_and(|(_, held)| *held > epoch) {
            return false;
        }
        self.catalog = Some((Box::new(catalog), epoch));
        true
    }

    /// Publish the catalog with its owner context into the open menu; `on_mounted` runs once it is held.
    pub fn publish_mii_catalog<S: Any + Send, C: Any + Send>(&mut self, catalog: S, context: C, epoch: u64, on_mounted: impl FnOnce()) -> bool {
        if !self.mount_mii_editor_catalog(catalog, epoch) {
            return false;
        }
        self.context = Some(Box::new(context));
        on_mounted();
        true
    }

    /// The ordinary-CPU cycle was qualified for the catalog now held.
    pub fn qualify_ordinary_cpu_cycle(&mut self, qualified: bool) {
        self.ordinary_cpu = qualified;
    }

    pub fn ordinary_cpu_qualified(&self) -> bool {
        self.ordinary_cpu
    }

    /// The Mii list screen takes what was published (and its context), once.
    pub fn take_catalog(&mut self) -> Option<(Box<dyn Any + Send>, u64, Option<Box<dyn Any + Send>>)> {
        let (catalog, epoch) = self.catalog.take()?;
        Some((catalog, epoch, self.context.take()))
    }

    /// The menu opened or closed: closing drops what was published and not yet shown.
    pub fn set_open(&mut self, open: bool) {
        self.page = if open { AppPage::SUBMENU } else { AppPage::CLOSED };
        if !open {
            self.catalog = None;
            self.context = None;
        }
    }
}

pub static QUICK_MENU_APP: Mutex<App> = Mutex::new(App::new());
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
