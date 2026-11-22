//! The optional test bridge's state provider (`model::bridge_state`, ssbu-hw-bridge docs/PLUGIN-STATE.md v2.1).
//! TrainingRooms has no link-time dependency on the bridge: the UI worker looks its table up by name at most once a second
//! for the first 30 s (`nn::ro::LookupSymbol`), validates it before any call, registers once and keeps that handle for the
//! boot. Without the bridge the lookups stop at 30 s and every entry point is one relaxed load. Publishing happens on the
//! worker only; the game thread's calls (`note_notice`, `note_zone`) never wait.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::model::bridge_state as m;
use crate::model::menu::table::{Control, Row};

/// The table's fixed header, read and checked before the whole table is.
#[repr(C)]
struct Header {
    magic: u32,
    abi: u32,
    struct_size: u32,
    readers: u32,
    max_payload: u32,
    max_providers: u32,
}

/// The bridge's table (ssbu-hw-bridge `state::StateApiV1`, C ABI). Pointers are `Option`, so a null one is seen.
#[repr(C)]
struct Api {
    /// Read through `Header` before the table is formed.
    _header: Header,
    register_provider: Option<extern "C" fn(*const u8, u32, u32, u32) -> i32>,
    publish: Option<extern "C" fn(i32, *const u8, u32) -> i32>,
    event: Option<extern "C" fn(i32, *const u8, u32) -> i32>,
}

const SYMBOL: &[u8] = b"ssbu_hw_bridge_state_api_v1\0";

/// Off once the lookup window ended without a bridge (or its table was unusable): every entry point returns at once.
static ENABLED: AtomicBool = AtomicBool::new(true);
static FOUND: AtomicBool = AtomicBool::new(false);
/// The zone editor's state: written by the game thread under the editor's own lock, read by the worker.
static ZONE: m::ZoneCell = m::ZoneCell::new();

/// The editor's state after an input (game thread). One relaxed load without a bridge; `state` is not called then.
pub fn note_zone(state: impl FnOnce() -> m::ZoneEditor) {
    if FOUND.load(Ordering::Relaxed) { ZONE.write(state().pack()); }
}

/// The last complete state; `None` while one is being written.
pub fn zone() -> Option<[u64; 4]> { ZONE.read() }

/// The practice layout's last regular pass (the snapshot's `scene`): `uptime_ms << 2 | kind` (1 Training, 2 lobby
/// practice), one atomic store from the UI tick (never a lock), read by the worker; older than `SCENE_FRESH_MS` is gone.
static SCENE_AT: AtomicU64 = AtomicU64::new(0);
const SCENE_FRESH_MS: u64 = 500;

/// The UI tick passed its practice-layout check (`lobby` for the waiting room's practice).
pub fn note_scene(lobby: bool) {
    if FOUND.load(Ordering::Relaxed) {
        SCENE_AT.store(crate::runtime::clock::uptime_ms() << 2 | if lobby { 2 } else { 1 }, Ordering::Relaxed);
    }
}

pub fn scene(now_ms: u64) -> &'static str {
    m::scene_name(SCENE_AT.load(Ordering::Relaxed), now_ms, SCENE_FRESH_MS)
}

/// Inputs the menu dispatched this boot (the snapshot's `ack`), and whether the last one acted (`ack.result`).
static ACK: AtomicU64 = AtomicU64::new(0);
static ACK_HANDLED: AtomicBool = AtomicBool::new(true);
/// Whether the HUD shows the notice slot now (hidden with the menu open, gone once the notice expires).
static NOTICE_VISIBLE: AtomicBool = AtomicBool::new(false);

/// The current notice, written by the game thread, read by the worker; whoever finds it busy skips (never waits).
struct Notice {
    header: [u8; 64],
    header_len: usize,
    body: [u8; 192],
    body_len: usize,
}
struct NoticeCell(UnsafeCell<Notice>);
// SAFETY: the cell is touched only by whoever holds `NOTICE_BUSY`.
unsafe impl Sync for NoticeCell {}
static NOTICE_BUSY: AtomicBool = AtomicBool::new(false);
static NOTICE: NoticeCell = NoticeCell(UnsafeCell::new(Notice { header: [0; 64], header_len: 0, body: [0; 192], body_len: 0 }));

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// The worker should look again soon: the lookup is still running (once a second) or the bridge is there (each poll).
pub fn poll_interval() -> Option<std::time::Duration> {
    if !enabled() {
        return None;
    }
    Some(std::time::Duration::from_millis(if FOUND.load(Ordering::Relaxed) { 33 } else { 1_000 }))
}

/// One input the menu dispatched, after its handler ran: `ack` advances and `ack.result` is the handler's own outcome.
pub fn note_ack(handled: bool) {
    if FOUND.load(Ordering::Relaxed) {
        ACK_HANDLED.store(handled, Ordering::Relaxed);
        ACK.fetch_add(1, Ordering::Release);
    }
}

pub fn ack_result() -> &'static str {
    m::ack_result(ACK_HANDLED.load(Ordering::Relaxed))
}

/// The HUD's notice slot visibility each tick (game thread): one atomic store, after one relaxed load without a bridge.
pub fn note_notice_visible(visible: bool) {
    if FOUND.load(Ordering::Relaxed) {
        NOTICE_VISIBLE.store(visible, Ordering::Relaxed);
    }
}

/// The notice the HUD draws (game thread). Skipped when the worker is reading it or no bridge was found.
pub fn note_notice(header: &str, body: &str) {
    if !FOUND.load(Ordering::Relaxed) || NOTICE_BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        return;
    }
    // SAFETY: we hold `NOTICE_BUSY`.
    let n = unsafe { &mut *NOTICE.0.get() };
    n.header_len = copy_cut(&mut n.header, header);
    n.body_len = copy_cut(&mut n.body, body);
    NOTICE_BUSY.store(false, Ordering::Release);
}

/// `text` into `out`, cut at a char boundary.
fn copy_cut(out: &mut [u8], text: &str) -> usize {
    let mut len = text.len().min(out.len());
    while !text.is_char_boundary(len) {
        len -= 1;
    }
    out[..len].copy_from_slice(&text.as_bytes()[..len]);
    len
}

fn read_notice(header: &mut String, body: &mut String) -> bool {
    if NOTICE_BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        return false;
    }
    // SAFETY: we hold `NOTICE_BUSY`.
    let n = unsafe { &*NOTICE.0.get() };
    header.clear();
    body.clear();
    header.push_str(core::str::from_utf8(&n.header[..n.header_len]).unwrap_or(""));
    body.push_str(core::str::from_utf8(&n.body[..n.body_len]).unwrap_or(""));
    NOTICE_BUSY.store(false, Ordering::Release);
    true
}

/// The row's stable action id: `start.<game>` / `game.<game>` for the keyless practice-game rows, else its setting id.
pub fn action(row: Option<&Row>) -> String {
    match row.map(|r| (r.control, r.id)) {
        Some((Control::Start(game), _)) => format!("start.{}", game.code()),
        Some((Control::Game(game), _)) => format!("game.{}", game.code()),
        Some((_, id)) if !id.is_empty() => id.to_string(),
        _ => "row".to_string(),
    }
}

/// The worker's provider: the lookup schedule, the table and handle (kept for the boot), the publish rule.
#[derive(Default)]
pub struct Provider {
    discovery: m::Discovery,
    api: Option<&'static Api>,
    handle: i32,
    publisher: m::Publisher,
    snapshot: String,
    notice: (String, String),
}

impl Provider {
    /// One worker pass. `view` builds the snapshot; it is called only once the bridge was found.
    pub fn tick(&mut self, now_ms: u64, view: impl FnOnce(&(String, String), u64, &mut String)) {
        if !enabled() {
            return;
        }
        let Some(api) = self.api else {
            if self.discovery.due(now_ms) {
                self.lookup();
            }
            if !self.discovery.enabled() {
                ENABLED.store(false, Ordering::Relaxed);
            }
            return;
        };
        read_notice(&mut self.notice.0, &mut self.notice.1);
        let shown = m::shown_notice(NOTICE_VISIBLE.load(Ordering::Relaxed), (self.notice.0.as_str(), self.notice.1.as_str()));
        let notice = shown.map_or((String::new(), String::new()), |(h, b)| (h.to_string(), b.to_string()));
        view(&notice, ACK.load(Ordering::Acquire), &mut self.snapshot);
        if self.snapshot.is_empty() { return; }
        if !self.publisher.wants(self.snapshot.as_bytes()) {
            return;
        }
        let Some(publish) = api.publish else { return };
        let result = publish(self.handle, self.snapshot.as_ptr(), self.snapshot.len() as u32);
        self.publisher.published(self.snapshot.as_bytes(), result);
    }

    fn lookup(&mut self) {
        let mut address = 0usize;
        // SAFETY: a NUL-terminated literal; the loader writes one address (0 when no image exports it).
        let result = unsafe { skyline::nn::ro::LookupSymbol(&mut address, SYMBOL.as_ptr()) };
        if result != 0 || address == 0 {
            return; // not loaded (yet): the schedule tries again
        }
        // SAFETY: the bridge's exported getter, `extern "C" fn() -> *const Api`.
        let getter: extern "C" fn() -> *const Api = unsafe { core::mem::transmute(address) };
        let table = getter();
        // SAFETY: checked non-null; only the fixed header is read until it says the whole table is this ABI and size.
        let Some(header) = (unsafe { table.cast::<Header>().as_ref() }) else { return self.discovery.disable() };
        if !m::header_usable(header.magic, header.abi, header.struct_size, header.readers, header.max_payload, header.max_providers,
            core::mem::size_of::<Api>())
        {
            return self.discovery.disable();
        }
        // SAFETY: the header declared exactly this table; the bridge keeps it for the boot.
        let api = unsafe { &*table };
        let usable = api.register_provider.is_some() && api.publish.is_some() && api.event.is_some();
        let Some(register) = api.register_provider.filter(|_| usable) else { return self.discovery.disable() };
        let handle = register(m::NAME.as_ptr(), m::NAME.len() as u32, m::SCHEMA_ID, m::SCHEMA_VERSION);
        if handle < 0 {
            return self.discovery.disable();
        }
        self.api = Some(api);
        self.handle = handle;
        self.discovery.found();
        FOUND.store(true, Ordering::Relaxed);
        crate::runtime::diag_log::diagnostic(format_args!("[TrainingRooms] bridge_state registered handle={handle}"));
    }
}
