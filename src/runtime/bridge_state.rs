//! The optional test bridge's state provider (`model::bridge_state`, ssbu-hw-bridge docs/PLUGIN-STATE.md v2.1).
//! TrainingRooms has no link-time dependency on the bridge: the UI worker looks its table up by name at most once a second
//! for the first 30 s (`nn::ro::LookupSymbol`), validates it before any call, registers once and keeps that handle for the
//! boot. Without the bridge the lookups stop at 30 s and every entry point is one relaxed load. Snapshot publishing happens on the
//! worker only. Game-side notice, zone and practice producers use bounded, nonblocking handoffs.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};

use crate::engine::{library::Role, rr_host::fed_run};
use crate::model::bridge_state as m;
use crate::model::bridge_events::{Clip, ClipSeen, Event, Fed, FedSeen, Outbox};
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

static PRACTICE: hos_lock::Mailbox<PracticeSample, { practice::SAMPLE_SLOTS }> = hos_lock::Mailbox::new();

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

    pub fn only(self, activation: u64) -> Self {
        if self.activation == activation { self } else { Self::default() }
    }

    pub fn submit(self, sample: impl FnOnce() -> Fields) {
        if self.admission != 0 {
            self.post(false, &sample());
        }
    }

    /// Mash out's grading worker owns the asynchronous result, while the game thread owns phase and elapsed.
    pub fn result(self, sample: impl FnOnce() -> Option<Fields>) {
        if self.admission != 0 {
            if let Some(fields) = sample() {
                self.post(true, &fields);
            }
        }
    }

    /// One copy of the hand-off: a sample of an activation that has ended is dropped.
    #[inline(never)]
    fn post(self, result: bool, fields: &Fields) {
        if crate::runtime::features::practice::activation().0 == self.activation {
            PRACTICE.post(PracticeSample { admission: self.admission, activation: self.activation, fields: *fields, result });
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
static EVENTS: hos_lock::Mailbox<Event, 8> = hos_lock::Mailbox::new();
static GAME_EVENT: AtomicUsize = AtomicUsize::new(0);
static GAME_HANDLE: AtomicI32 = AtomicI32::new(0);
static FED: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];

pub fn note_fed() {
    if !FOUND.load(Ordering::Relaxed) { return; }
    let address = GAME_EVENT.load(Ordering::Acquire);
    if address == 0 { return; }
    let handle = GAME_HANDLE.load(Ordering::Relaxed);
    // SAFETY: the bridge keeps this validated C-ABI export for the boot.
    let event: extern "C" fn(i32, *const u8, u32) -> i32 = unsafe { core::mem::transmute(address) };
    let before = FedSeen::from_words([FED[0].load(Ordering::Relaxed), FED[1].load(Ordering::Relaxed), FED[2].load(Ordering::Relaxed)]);
    let mut seen = before;
    match (fed_run(Role::Player), fed_run(Role::Cpu)) {
        (Some((run_p1, p1)), Some((run_cpu, cpu))) => seen.sample(Fed {
            p1, cpu, run: [run_p1, run_cpu],
            preview: crate::engine::rr_host::preview_running(),
            hides: crate::runtime::ui::state::PREVIEW_HIDES.load(Ordering::Acquire),
        }, |text| event(handle, text.as_ptr(), text.len() as u32)),
        _ => seen.skip(),
    }
    if seen != before {
        let words = seen.words();
        FED[0].store(words[0], Ordering::Relaxed);
        FED[1].store(words[1], Ordering::Relaxed);
        FED[2].store(words[2], Ordering::Relaxed);
    }
}

pub fn note_event(event: Event) {
    if FOUND.load(Ordering::Relaxed) {
        EVENTS.post(event);
    }
}
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
/// The HUD row's last pass (the snapshot's `recording.indicator`): `m::indicator_stamp`, one atomic store from the UI
/// tick (never a lock), read by the worker; older than `SCENE_FRESH_MS` is gone.
static RECORDING_INDICATOR: AtomicU64 = AtomicU64::new(0);
/// The direction marks drawn by the strip's last pass (`m::aims_stamp`).
static STRIP_AIMS: AtomicU64 = AtomicU64::new(0);
/// The serialized HUD tick publishes; only the menu worker reads. Neither waits.
static STRIP_SCROLL: hos_lock::TripleBuffer<m::ScrollStamp> = hos_lock::TripleBuffer::new(m::scroll_stamp(0, 0, 0));

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

/// The take the HUD's row drew on this pass (`None`: it drew none): one atomic store, after one relaxed load without a
/// bridge.
pub fn note_recording_indicator(drawn: Option<crate::model::hud_row::Take>) {
    if FOUND.load(Ordering::Relaxed) {
        RECORDING_INDICATOR.store(m::indicator_stamp(crate::runtime::clock::uptime_ms(), drawn), Ordering::Relaxed);
    }
}

/// The role of the take the HUD's row drew on its last pass, empty once that pass is stale.
pub fn recording_indicator(now_ms: u64) -> &'static str {
    m::recording_indicator(RECORDING_INDICATOR.load(Ordering::Relaxed), now_ms, SCENE_FRESH_MS)
}

/// The strip's direction marks on this pass, oldest first.
pub fn note_strip_aims(drawn: [Option<u8>; 2]) {
    if FOUND.load(Ordering::Relaxed) {
        STRIP_AIMS.store(m::aims_stamp(crate::runtime::clock::uptime_ms(), drawn), Ordering::Relaxed);
    }
}

/// The strip's direction marks on its last fresh pass, oldest first.
pub fn strip_aims(now_ms: u64) -> [Option<u8>; 2] {
    m::strip_aims(STRIP_AIMS.load(Ordering::Relaxed), now_ms, SCENE_FRESH_MS)
}

/// The scroll distance drawn by the strip on this pass.
pub fn note_strip_scroll(drawn: (u32, u32)) {
    if FOUND.load(Ordering::Relaxed) {
        STRIP_SCROLL.write(m::scroll_stamp(crate::runtime::clock::uptime_ms(), drawn.0, drawn.1));
    }
}

/// The strip's scroll distance on its last fresh pass.
pub fn strip_scroll(now_ms: u64) -> (u32, u64) {
    m::strip_scroll(STRIP_SCROLL.read().0, now_ms, SCENE_FRESH_MS)
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
    events: Outbox,
    clip: ClipSeen,
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
        self.events(api, self.handle, now_ms);
        read_notice(&mut self.notice.0, &mut self.notice.1);
        let shown = m::shown_notice(NOTICE_VISIBLE.load(Ordering::Relaxed), (self.notice.0.as_str(), self.notice.1.as_str()));
        let notice = shown.map_or((String::new(), String::new()), |(h, b)| (h.to_string(), b.to_string()));
        view(&notice, ACK.load(Ordering::Acquire), &mut self.snapshot);
        if self.snapshot.is_empty() { return; }
        if !self.publisher.wants(self.snapshot.as_bytes()) {
            return;
        }
        let result = publish(self.handle, self.snapshot.as_ptr(), self.snapshot.len() as u32);
        self.publisher.published(self.snapshot.as_bytes(), result);
    }

    fn events(&mut self, api: &Api, handle: i32, now_ms: u64) {
        let Some(event) = api.event else { return };
        GAME_HANDLE.store(handle, Ordering::Relaxed);
        GAME_EVENT.store(event as usize, Ordering::Release);
        EVENTS.drain(|value| self.events.accept(value, now_ms));
        self.events.lost(EVENTS.take_dropped());
        let send = |text: &str| event(handle, text.as_ptr(), text.len() as u32);
        self.events.flush(now_ms, send);
        let clip = crate::runtime::macros::replay_capture::clip_view().map(|view| {
            use crate::engine::macro_ui::clip_panel::{ClipContext, Handle};
            Clip {
                ctx: match view.context {
                    ClipContext::Replay { .. } => "replay",
                    ClipContext::Macro { player: true } => "macro.p1",
                    ClipContext::Macro { player: false } => "macro.cpu",
                    ClipContext::Combo { .. } => "combo",
                },
                total: view.total, start: view.start, end: view.end, cursor: view.cursor,
                playing: view.playing,
                handle: match view.handle { Handle::Playhead => "playhead", Handle::In => "in", Handle::Out => "out" },
                running: crate::engine::rr_host::preview_running(),
            }
        });
        self.clip.send(clip, send);
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
