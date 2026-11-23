//! The optional test bridge's state provider (`model::bridge_state`, ssbu-hw-bridge docs/PLUGIN-STATE.md v2.1).
//! TrainingRooms has no link-time dependency on the bridge: the UI worker looks its table up by name at most once a second
//! for the first 30 s (`nn::ro::LookupSymbol`), validates it before any call, registers once and keeps that handle for the
//! boot. Without the bridge the lookups stop at 30 s and every entry point is one relaxed load. Publishing happens on the
//! worker only. Game-side notice and practice producers use bounded, nonblocking handoffs.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::model::bridge_state as m;
use crate::model::menu::table::{Control, Row};
use crate::model::practice_games::Game;
use crate::model::practice_state::{self as practice, Fields};

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
const ADMISSION_SYMBOL: &[u8] = b"ssbu_hw_bridge_state_admission_gen_v1\0";

/// Optional, boot-lifetime bridge export. Zero when no compatible practice provider was registered.
static ADMISSION: AtomicUsize = AtomicUsize::new(0);
static THINKS: [AtomicU64; practice::CPU_ENTRIES] = [const { AtomicU64::new(0) }; practice::CPU_ENTRIES];

#[derive(Clone, Copy)]
struct PracticeSample {
    admission: u64,
    activation: u64,
    fields: Fields,
    result: bool,
}

static PRACTICE: hos_lock::Mailbox<PracticeSample, 8> = hos_lock::Mailbox::new();

fn practice_admission() -> u64 {
    let address = ADMISSION.load(Ordering::Acquire);
    if address == 0 {
        return 0;
    }
    // SAFETY: lookup accepts the boot-lifetime C-ABI export with this signature only.
    let admitted: extern "C" fn() -> u64 = unsafe { core::mem::transmute(address) };
    admitted()
}

/// A sample's owner before the engine moves. Disabled captures never evaluate their projection.
#[derive(Clone, Copy, Default)]
pub struct PracticeCapture {
    admission: u64,
    activation: u64,
}

impl PracticeCapture {
    pub fn of(game: Game) -> Self {
        let admission = practice_admission();
        if admission != 0 {
            use crate::runtime::features::practice::{activation, STARTING};
            let (activation, active) = activation();
            if active == Some(game) && activation & STARTING == 0 {
                return Self { admission, activation };
            }
        }
        Self { admission: 0, activation: 0 }
    }

    pub fn submit(self, sample: impl FnOnce() -> Fields) {
        self.post(false, || Some(sample()));
    }

    /// Mash out's grading worker owns the asynchronous result, while the game thread owns phase and elapsed.
    pub fn result(self, sample: impl FnOnce() -> Option<Fields>) {
        self.post(true, sample);
    }

    fn post(self, result: bool, sample: impl FnOnce() -> Option<Fields>) {
        let Self { admission, activation } = self;
        if admission == 0 || crate::runtime::features::practice::activation().0 != activation {
            return;
        }
        if let Some(fields) = sample() {
            PRACTICE.post(PracticeSample { admission, activation, fields, result });
        }
    }
}

/// Existing AI epilogue. Use the primary match first and inspect other live entries only while admitted.
pub fn note_ai_think(primary_match: Option<i32>, matches: impl FnMut(i32) -> bool) {
    if let Some(entry) = practice::think_entry(practice_admission(), primary_match, matches) {
        THINKS[entry].fetch_add(1, Ordering::Relaxed);
    }
}

struct PracticeProvider {
    admission_export: usize,
    registration: m::Discovery,
    handle: Option<i32>,
    admission: u64,
    activation: u64,
    fields: Fields,
    result: Fields,
    publisher: m::Publisher,
    snapshot: String,
}

impl PracticeProvider {
    fn discover() -> Option<Self> {
        let mut address = 0usize;
        // SAFETY: the loader is given a NUL-terminated symbol and one output address.
        let result = unsafe { skyline::nn::ro::LookupSymbol(&mut address, ADMISSION_SYMBOL.as_ptr()) };
        if result != 0 || address == 0 {
            return None;
        }
        Some(Self {
            admission_export: address,
            registration: m::Discovery::default(),
            handle: None,
            admission: 0,
            activation: 0,
            fields: Fields::default(),
            result: Fields::default(),
            publisher: m::Publisher::default(),
            snapshot: String::new(),
        })
    }

    fn practice_handle(&mut self, now_ms: u64, register: extern "C" fn(*const u8, u32, u32, u32) -> i32) -> Option<i32> {
        if self.registration.due(now_ms) {
            let result = register(practice::NAME.as_ptr(), practice::NAME.len() as u32, practice::SCHEMA_ID, practice::SCHEMA_VERSION);
            self.handle = self.registration.registration(result);
            if self.handle.is_some() {
                ADMISSION.store(self.admission_export, Ordering::Release);
            }
        }
        self.handle
    }

    fn poll_practice(
        &mut self,
        now_ms: u64,
        register: extern "C" fn(*const u8, u32, u32, u32) -> i32,
        publish: extern "C" fn(i32, *const u8, u32) -> i32,
    ) {
        let Some(handle) = self.practice_handle(now_ms, register) else { return };
        let admission = practice_admission();
        if admission == 0 {
            return;
        }
        let (activation, game) = practice::published_activation(scene(now_ms), crate::runtime::features::practice::activation);
        if admission != self.admission || activation != self.activation {
            self.admission = admission;
            self.activation = activation;
            self.fields = Fields::default();
            self.result = Fields::default();
            self.publisher = m::Publisher::default();
        }
        PRACTICE.drain(|sample| {
            if sample.admission == admission && sample.activation == activation {
                if sample.result {
                    self.result = sample.fields;
                } else {
                    practice::adopt_sample(&mut self.fields, sample.fields);
                }
            }
        });
        let counts = core::array::from_fn(|entry| THINKS[entry].load(Ordering::Relaxed));
        let mut fields = self.fields;
        if game == Some(Game::Mashout) && !self.result.last_outcome.is_empty() {
            fields.last_outcome = self.result.last_outcome;
            fields.last_frames = self.result.last_frames;
            fields.baseline_frames = self.result.baseline_frames;
            fields.fastest_frames = self.result.fastest_frames;
            fields.best = self.result.best;
        }
        practice::snapshot(game, fields, counts, &mut self.snapshot);
        if self.publisher.wants(self.snapshot.as_bytes()) && practice_admission() == admission {
            let result = publish(handle, self.snapshot.as_ptr(), self.snapshot.len() as u32);
            self.publisher.published(self.snapshot.as_bytes(), result);
        }
    }
}

/// Off once the lookup window ended without a bridge (or its table was unusable): every entry point returns at once.
static ENABLED: AtomicBool = AtomicBool::new(true);
static FOUND: AtomicBool = AtomicBool::new(false);
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
    practice: Option<PracticeProvider>,
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
        let Some(publish) = api.publish else { return };
        if let (Some(practice), Some(register)) = (&mut self.practice, api.register_provider) {
            practice.poll_practice(now_ms, register, publish);
        }
        read_notice(&mut self.notice.0, &mut self.notice.1);
        let shown = m::shown_notice(NOTICE_VISIBLE.load(Ordering::Relaxed), (self.notice.0.as_str(), self.notice.1.as_str()));
        let notice = shown.map_or((String::new(), String::new()), |(h, b)| (h.to_string(), b.to_string()));
        view(&notice, ACK.load(Ordering::Acquire), &mut self.snapshot);
        if !self.publisher.wants(self.snapshot.as_bytes()) {
            return;
        }
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
        let result = register(m::NAME.as_ptr(), m::NAME.len() as u32, m::SCHEMA_ID, m::SCHEMA_VERSION);
        let Some(handle) = self.discovery.registration(result) else { return };
        self.api = Some(api);
        self.practice = PracticeProvider::discover();
        self.handle = handle;
        FOUND.store(true, Ordering::Relaxed);
        crate::runtime::diag_log::diagnostic(format_args!("[TrainingRooms] bridge_state registered handle={handle}"));
    }
}
