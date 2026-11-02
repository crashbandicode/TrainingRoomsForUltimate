//! One shared catalog read for menu open and lifetime auto-init.
//! The game thread only queues and later applies an owned snapshot.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use crate::model::catalog_reply::{Job, Replies};
use crate::runtime::ui::menu_app::{CatalogPublication, MENU_RECEIVED_INPUT};

use crate::engine::mii::{
    catalog_request::{Effect, Kind, Live, Requests, Step, Verdict},
    inventory::Snapshot,
};

static PENDING: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<Broker> = Mutex::new(Broker::empty());

struct Broker {
    policy: Requests,
    reply: Option<Replies<Arc<Snapshot>>>,
    in_flight: bool,
    flight_serial: u64,
    held_editor: Option<(Arc<Snapshot>, u64)>,
    held_menu: Option<CatalogPublication>,
}

impl Broker {
    const fn empty() -> Self {
        Self {
            policy: Requests::new(),
            reply: None,
            in_flight: false,
            flight_serial: 0,
            held_editor: None,
            held_menu: None,
        }
    }

    fn busy(&self) -> bool {
        self.held_menu.is_some() || self.held_editor.is_some() || self.policy.pending()
    }
}

/// Startup only, before native callbacks are installed.
pub(super) fn init() {
    super::menu_handoff::init();
    crate::runtime::ui::menu_app::init_publications();
    let mut broker = STATE.lock().unwrap();
    if broker.reply.is_none() { broker.reply = Some(Replies::new()); }
}

fn live() -> Live {
    Live {
        menu_active: crate::runtime::compat::common::menu::quick_menu_active(),
        opening: super::mii_saved_alpha::menu_epoch(),
        owners: [
            super::mii_saved_alpha::current_generation(0),
            super::mii_saved_alpha::current_generation(1),
        ],
        auto: reading_token(),
    }
}

fn reading_token() -> Option<crate::engine::mii::auto_init::Token> {
    #[cfg(feature = "mii-ordinary-cpu-alpha")]
    {
        super::mii_auto_init::reading_token()
    }
    #[cfg(not(feature = "mii-ordinary-cpu-alpha"))]
    {
        None
    }
}

pub(crate) fn request(kind: Kind) {
    let mut broker = STATE.lock().unwrap();
    let effect = broker.policy.request(kind);
    broker.drive(effect);
    PENDING.store(broker.busy(), Ordering::Release);
}

pub(crate) fn close_opening(opening: u64) {
    let mut broker = STATE.lock().unwrap();
    let restart_auto = broker.policy.close_menu(opening);
    if broker.held_menu.as_ref().is_some_and(|p| p.epoch == opening) { broker.held_menu = None; }
    if broker
        .held_editor
        .as_ref()
        .is_some_and(|(_, held)| *held == opening)
    {
        broker.held_editor = None;
    }
    PENDING.store(broker.busy(), Ordering::Release);
    drop(broker);
    // A quick close can cancel the menu read that superseded automatic capture.
    // Restart through the normal eligible native callback, never the stale reply.
    #[cfg(feature = "mii-ordinary-cpu-alpha")]
    if restart_auto {
        super::mii_auto_init::reset();
    }
    #[cfg(not(feature = "mii-ordinary-cpu-alpha"))]
    let _ = restart_auto;
}

pub(super) fn retire() {
    let mut broker = STATE.lock().unwrap();
    broker.policy.retire();
    broker.held_menu = None;
    broker.held_editor = None;
    PENDING.store(broker.busy(), Ordering::Release);
}

pub(super) fn service() {
    crate::runtime::mii::menu_handoff::service();
    if !PENDING.load(Ordering::Acquire) {
        return;
    }
    let (held, menu, ready) = {
        let mut broker = STATE.lock().unwrap();
        let held = broker.held_editor.take();
        let menu = broker.held_menu.take();
        let ready = broker.poll();
        PENDING.store(broker.busy(), Ordering::Release);
        (held, menu, ready)
    };
    if let Some(publication) = menu { publish_menu(publication); }
    if let Some((snapshot, opening)) = held {
        retain_editor(snapshot, opening);
    }
    if let Some((verdict, snapshot)) = ready {
        apply(verdict, snapshot);
    }
}

impl Broker {
    fn poll(&mut self) -> Option<(Verdict, Arc<Snapshot>)> {
        if self.in_flight {
            let reply = self.reply.as_mut().expect("catalog initialized before requests");
            match reply.poll() {
                Ok(Ok(snapshot)) => {
                    self.in_flight = false;
                    let effect = self.policy.complete(self.flight_serial, live());
                    let verdict = effect.verdict;
                    self.drive(effect);
                    return Some((verdict, snapshot));
                }
                Err(mpsc::TryRecvError::Empty) => return None,
                Ok(Err(())) | Err(mpsc::TryRecvError::Disconnected) => {
                    self.in_flight = false;
                    let effect = self.policy.failed(live());
                    self.drive(effect);
                    return None;
                }
            }
        }
        if self.policy.needs_retry() {
            let effect = self.policy.retry(live());
            self.drive(effect);
        }
        None
    }

    fn drive(&mut self, mut effect: Effect) {
        for _ in 0..2 {
            self.abandon(&effect);
            if !matches!(effect.step, Step::Submit(_)) {
                Self::note(&effect);
                return;
            }
            effect = self.enqueue();
        }
        self.abandon(&effect);
        Self::note(&effect);
    }

    fn enqueue(&mut self) -> Effect {
        let Some(replies) = self.reply.as_mut() else { return self.policy.failed(live()); };
        let Some(sender) = replies.begin() else { return self.policy.failed(live()); };
        match crate::runtime::macros::replay_store::request_mii_catalog(Job::new(sender)) {
            crate::runtime::macros::replay_store::CatalogQueue::Queued => {
                self.flight_serial = self.policy.serial();
                self.in_flight = true;
                Effect::quiet(Step::Wait)
            }
            crate::runtime::macros::replay_store::CatalogQueue::Full(job) => {
                replies.rejected(job.rejected());
                self.policy.rejected_full()
            }
            crate::runtime::macros::replay_store::CatalogQueue::Unavailable(job) => {
                replies.rejected(job.rejected());
                self.policy.failed(live())
            }
        }
    }

    fn abandon(&self, effect: &Effect) {
        abandon_auto(effect.dropped);
        if let Some(kind) = effect.finished {
            let applied = matches!(
                (kind, effect.verdict),
                (Kind::Auto(finished), Verdict::Auto(applied)) if finished == applied
            );
            if !applied {
                abandon_auto(Some(kind));
            }
        }
    }

    fn note(effect: &Effect) {
        if matches!(effect.step, Step::Unavailable) {
            trace!("[MiiCatalog] saved-Mii catalog unavailable");
        }
    }
}

fn abandon_auto(kind: Option<Kind>) {
    let Some(Kind::Auto(token)) = kind else {
        return;
    };
    #[cfg(feature = "mii-ordinary-cpu-alpha")]
    super::mii_auto_init::abandon(token);
    #[cfg(not(feature = "mii-ordinary-cpu-alpha"))]
    {
        let _ = token;
    }
}

fn apply(verdict: Verdict, snapshot: Arc<Snapshot>) {
    match verdict {
        Verdict::Discard => {}
        Verdict::Menu { opening, owners } => {
            if super::mii_saved_alpha::accept_catalog(snapshot, opening, owners) {
                STATE.lock().unwrap().policy.menu_armed(opening);
            }
        }
        Verdict::Editor { opening } => retain_editor(snapshot, opening),
        Verdict::Auto(token) => {
            #[cfg(feature = "mii-ordinary-cpu-alpha")]
            super::mii_auto_init::deliver(token, snapshot);
            #[cfg(not(feature = "mii-ordinary-cpu-alpha"))]
            {
                let _ = (token, snapshot);
            }
        }
    }
}

/// Capture has completed. Full/unavailable publication retains the identical
/// value for the next native callback; the existing close/retire policy cancels it.
pub(super) fn publish_menu(publication: CatalogPublication) {
    if !crate::runtime::ui::menu_app::active()
        || publication.epoch != super::mii_saved_alpha::menu_epoch() { return; }
    match crate::runtime::ui::menu_app::publish_catalog(publication) {
        Ok(()) => { MENU_RECEIVED_INPUT.store(true, Ordering::Release); }
        Err(publication) => {
            let mut broker = STATE.lock().unwrap();
            broker.held_menu = Some(publication);
            PENDING.store(true, Ordering::Release);
        }
    }
}

fn retain_editor(snapshot: Arc<Snapshot>, opening: u64) {
    #[cfg(feature = "mii-editor-alpha")]
    {
        match crate::runtime::mii::mii_editor_bridge::mount_opened_catalog(snapshot, opening) {
            crate::runtime::mii::mii_editor_bridge::OpenedCatalog::Shown
            | crate::runtime::mii::mii_editor_bridge::OpenedCatalog::Dropped => {}
            crate::runtime::mii::mii_editor_bridge::OpenedCatalog::Later(snapshot) => {
                let mut broker = STATE.lock().unwrap();
                if broker.policy.allows_editor(opening, &live()) {
                    broker.held_editor = Some((snapshot, opening));
                }
                PENDING.store(broker.busy(), Ordering::Release);
            }
        }
    }
    #[cfg(not(feature = "mii-editor-alpha"))]
    {
        let _ = (snapshot, opening);
    }
}
