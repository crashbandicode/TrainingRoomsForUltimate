//! Current combined alpha: one shared catalog read, then current-callback capture.
use super::*;
use hos_lock::Mutex;
use std::sync::Arc;
use crate::engine::mii::{
    auto_init::{self, Initialization, Token},
    catalog_request::Kind,
    inventory::Snapshot,
};

// Completed initialization costs one atomic load per visible entry callback.
static PENDING: AtomicBool = AtomicBool::new(true);
/// Safe to query under a role STATE lock; never takes the coordinator mutex.
pub(super) fn pending() -> bool {
    PENDING.load(Ordering::Acquire)
}
static STATE: Mutex<State> = Mutex::new(State {
    init: Initialization::empty(),
    held: None,
});
struct State {
    init: Initialization,
    held: Option<(Token, Arc<Snapshot>)>,
}
fn current_token() -> Token {
    Token {
        generations: [
            mii_saved_alpha::current_generation(0),
            mii_saved_alpha::current_generation(1),
        ],
        opening: mii_saved_alpha::menu_epoch(),
    }
}
/// Initial qualification takes priority over speculative source loads. Both role
/// adapters must be enabled; unsupported native layouts never start this read.
pub(super) fn initializing() -> bool {
    pending() && mii_ordinary_player::enabled() && mii_ordinary_cpu::enabled()
}
/// Hold a replacement until its role adapter can preserve the immutable CSS origin.
/// Atomic loads and fresh native reads only, never the coordinator mutex, so a role
/// STATE lock may be held. Disabled adapters never start the automatic read. The
/// origin test is capture's own guard: FE+0x18 of the visible entry is the original.
pub(super) unsafe fn origin_capture_pending(role: Role, qualified: bool, capture: bool) -> bool {
    let automatic = initializing();
    if qualified || !(automatic || capture) { return false; }
    let visible = mii_backend::current().map(|layout| layout.visible()[role as usize]);
    let origin = visible
        .and_then(|id| super::native_source::entry(id))
        .zip(original_kind(role))
        .is_some_and(|(entry, original)| read::<u32>(entry, 0x18) == original);
    let ready_go = crate::runtime::compat::common::is_ready_go();
    let ready = ready_go && mii_backend::ready();
    auto_init::holds_origin_replacement(qualified, origin, automatic, capture, ready_go, ready)
}
pub(super) fn reading_token() -> Option<Token> {
    STATE.lock().init.reading()
}
pub(super) fn cancel() {
    let mut s = STATE.lock();
    s.init.cancel();
    s.held = None;
    PENDING.store(false, Ordering::Release);
}
pub(super) fn reset() {
    let mut s = STATE.lock();
    s.init.reset();
    s.held = None;
    PENDING.store(true, Ordering::Release);
}
unsafe fn idle() -> bool {
    mii_backend::ready()
        && crate::runtime::compat::common::is_ready_go()
        && !menu::captures_input()
        && !lobby::native_ui_busy()
        && !button_config::has_pending_request()
        && !INSTALL.active()
        && !training::save_states::is_loading()
        && !training::recording_or_playback()
        && !crate::runtime::compat::replay_preview::occupied()
        && STATES.iter().all(|s| {
            let s = s.lock();
            s.active.is_none() && s.pending.is_none()
        })
}
pub(super) unsafe fn service(entry: *const u8, owner: u32) {
    if !pending()
        || !mii_ordinary_player::enabled()
        || !mii_ordinary_cpu::enabled()
    {
        return;
    }
    let Some(logical)=mii_backend::current().and_then(|l|l.logical(owner)) else{return;};
    if logical > 1 {return;}
    let eligible = original_kind(Role::Player).is_some()
        && original_kind(Role::Cpu).is_some()
        && scope(Role::Player).is_some_and(|s| s.owner == 0)
        && idle()
        && (ready_single_mii(entry, owner).is_some() || ready_forms(entry, owner).is_some());
    let mut s = STATE.lock();
    let current = current_token();
    if s.held.is_some() {
        // Keep the owned catalog until this callback may arm capture. No extra file read.
        if !eligible {
            return;
        }
        let (started, catalog) = s.held.take().unwrap();
        let arm = s.init.finish(started, current, catalog.is_complete());
        PENDING.store(s.init.pending(), Ordering::Release);
        drop(s);
        if arm {
            let player = mii_ordinary_player::catalog_ready(
                Arc::clone(&catalog),
                started.generations[0],
                started.opening,
            );
            let cpu = mii_ordinary_cpu::catalog_ready(
                catalog,
                started.generations[1],
                started.opening,
            );
            trace!("[MiiAutoInit] catalog ready; capture_armed={:?} generations={:?} opening={}", [player, cpu], started.generations, started.opening);
        } else {
            trace!("[MiiAutoInit] catalog refused: incomplete or retired; no automatic retry");
        }
        return;
    }
    let Some(started) = s.init.observe(current, logical as u32, eligible) else {
        return;
    };
    drop(s);
    // The shared writer owns the read. No native pointer crosses that boundary.
    super::mii_catalog_request::request(Kind::Auto(started));
    if pending() {
        trace!(
            "[MiiAutoInit] catalog read queued generations={:?} opening={}",
            started.generations, started.opening
        );
    }
}

pub(super) fn deliver(token: Token, catalog: Arc<Snapshot>) {
    let mut s = STATE.lock();
    if s.init.reading() == Some(token) {
        s.held = Some((token, catalog));
    }
}

pub(super) fn abandon(token: Token) {
    let mut s = STATE.lock();
    if s.held.as_ref().is_some_and(|(started, _)| *started == token) {
        s.held = None;
    }
    let _ = s.init.finish(token, current_token(), false);
    PENDING.store(s.init.pending(), Ordering::Release);
}
