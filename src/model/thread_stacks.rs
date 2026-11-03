//! Every thread the plugin spawns, with its stack size, from one table (stream/stack, 2026-11-02).
//!
//! Eden s4-07 and u4b-02 faulted at boot with `Unmapped Write64` just below a 64 KiB stack: `tr-trace-writer`'s body had a
//! 59,712 B frame (an 11 KB Mii editor trace moved by value three times) on a 65,536 B stack, so the first SDK call under it
//! ran off the bottom. A size per call site, chosen by eye, could not show that. Here each thread has a row with its size,
//! the deepest static path of its entry (`tools/stack/stack_depth.py`, direct calls only: a lower bound) and the high-water
//! mark measured in Eden by the diag build (`[ThreadStack]` lines: the stack is painted at spawn and the untouched depth
//! read back). The host test requires every row to cover both, plus a margin and the SDK reserve, so a frame that grows
//! fails the gate instead of a console.
//!
//! `runtime::threads::spawn` is the only way the runtime starts a thread; `tests/thread_stacks.rs` checks that no other
//! `stack_size(` call remains under `src/runtime`.

/// Room kept for what neither number sees: SDK code under a Rust frame (the s4-07 fault ran a 6,240 B SDK frame there),
/// indirect calls the static walk does not follow, and paths Eden did not drive.
pub const SDK_RESERVE: usize = 16 * 1024;

/// The word painted over a fresh stack; a word that no longer holds it was used.
pub const PAINT: u64 = 0x5452_5354_4143_4b21; // "TRSTACK!"

/// The threads, in the order the diag build reports them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum ThreadId {
    Heartbeat,
    SettingsLoad,
    StateLoad,
    StateWriter,
    StateDiag,
    IniWriter,
    TraceWriter,
    DiagReload,
    Import,
    ImportCaptures,
    Menu,
    Archive,
    Probe,
    MacroDiag,
    ReplaySave,
    MiiCatalog,
    PracticeTrace,
}

/// One row: what runs on the thread and the numbers its size is checked against.
#[derive(Clone, Copy, Debug)]
pub struct ThreadStack {
    pub id: ThreadId,
    /// The thread's name (`nn::os::SetThreadNamePointer`; a static C string, nothing allocated).
    pub c_name: &'static core::ffi::CStr,
    pub name: &'static str,
    pub bytes: usize,
    /// Deepest static path from the thread's entry in the diag NRO (`stack_depth.py <elf> table`): a lower bound.
    pub static_bound: usize,
    /// Qualified callees outside the owned ELF, in addition to its static path.
    pub external_bound: usize,
    /// Highest `used=` of the thread's `[ThreadStack]` lines over the Eden runs named in `measured_in` (0: not measured).
    pub measured: usize,
    pub measured_in: &'static str,
    /// A part of the entry symbol's name that picks this thread's `spawn_unchecked` shim in the ELF.
    pub symbol: &'static str,
}

const KIB: usize = 1024;

/// The table (one row per `ThreadId`, in its order).
pub const THREADS: [ThreadStack; 17] = [
    // the diagnostics drain (open, append, close every 50 ms), the late seam report, diag hook_stats and stack reports
    row(ThreadId::Heartbeat, c"tr-heartbeat", 28 * KIB, 5_056, 9_072, "Eden st-02..st-12", "diag_log::start_drain_thread::{closure#0}"),
    // settings.json read, recovery and the JSON parse, joined by the boot thread
    row(ThreadId::SettingsLoad, c"tr-settings-load", 28 * KIB, 4_592, 9_488, "Eden st-02..st-12", "boot::load_settings_boxed"),
    // save_states.json read and parse, joined by the boot thread
    row(ThreadId::StateLoad, c"tr-state-load", 40 * KIB, 14_192, 18_624, "Eden st-02..st-12", "states::load_file"),
    // save_states.json serialise and atomic replace
    row(ThreadId::StateWriter, c"tr-state-writer", 36 * KIB, 15_184, 15_408, "Eden st-02..st-12", "states::start_writer::{closure#0}"),
    // diag: request files, diag-settings.json parse, publish and apply
    row(ThreadId::StateDiag, c"tr-state-diag", 40 * KIB, 17_616, 8_592, "Eden st-02..st-12", "states::start_diag_triggers::{closure#0}"),
    // button_config.ini, replay_capture.ini and settings.json atomic writes
    row(ThreadId::IniWriter, c"tr-ini-writer", 28 * KIB, 3_184, 9_344, "Eden st-02..st-12", "features::writer::run"),
    // mii-editor-last.json (and the diag results observer trace): serialise from the boxed slot, atomic replace
    row(ThreadId::TraceWriter, c"tr-trace-writer", 20 * KIB, 2_720, 1_696, "Eden st-02..st-12", "compat::filesystem::init::{closure#0}"),
    // diag: diag-settings-next.json parse, publish and apply
    row(ThreadId::DiagReload, c"tr-diag-reload", 40 * KIB, 17_696, 9_840, "Eden st-02..st-12", "diag_reload::start::{closure#0}"),
    // first-boot import of the fork's files (blocking part), joined with a deadline
    row(ThreadId::Import, c"tr-import", 32 * KIB, 8_032, 10_080, "Eden st-02..st-12", "import::start::{closure#0}"),
    // first-boot import of the fork's captures in the background
    row(ThreadId::ImportCaptures, c"tr-import-captures", 24 * KIB, 3_968, 0, "", "import::start_background::{closure#0}"),
    // the menu worker: model steps, screens, settings JSON on close
    row(ThreadId::Menu, c"tr-menu", 56 * KIB, 32_432, 13_488, "Eden st-02..st-12", "ui::worker::run"),
    // returns parked layout archives to the pool after their grace
    row(ThreadId::Archive, c"tr-archive", 28 * KIB, 2_672, 7_712, "Eden st-02..st-12", "ui::archive::prepare::{closure#2}"),
    // ARCropolis read ABI includes a 131904-byte decoder frame. The qualified
    // external path is separate from the owned-ELF scan; see docs/UI16-PROBE-STACK-FAULT.md.
    row(ThreadId::Probe, c"tr-probe", 204 * KIB, 3_440, 151_920, "Eden ui-17 same ARC92171b2f full probe", "ui::probe::install::{closure#0}").with_external(143_376),
    // diag: diag-macro.req read and parse
    row(ThreadId::MacroDiag, c"tr-macro-diag", 28 * KIB, 3_248, 8_768, "Eden st-02..st-12", "macros::diag::start::{closure#0}"),
    // the replay capture archive and Mii preset writer
    row(ThreadId::ReplaySave, c"replay-capture-save", 56 * KIB, 32_416, 24_400, "Eden st-02..st-12", "replay_store::start_writer::{closure#0}"),
    // the saved Mii catalog read for the library
    row(ThreadId::MiiCatalog, c"mii-catalog", 24 * KIB, 5_872, 0, "", "replay_store::init_catalog_fallback::{closure#0}::{closure#0}"),
    // console-practice-trace builds only: the practice trace file writer
    row(ThreadId::PracticeTrace, c"tr-practice-trace", 128 * KIB, 0, 0, "", "practice_trace::writer"),
];

const fn row(
    id: ThreadId,
    c_name: &'static core::ffi::CStr,
    bytes: usize,
    static_bound: usize,
    measured: usize,
    measured_in: &'static str,
    symbol: &'static str,
) -> ThreadStack {
    let name = match c_name.to_str() {
        Ok(name) => name,
        Err(_) => panic!("thread names are ASCII"),
    };
    ThreadStack { id, c_name, name, bytes, static_bound, external_bound: 0, measured, measured_in, symbol }
}

impl ThreadStack {
    const fn with_external(mut self, bytes: usize) -> Self {
        self.external_bound = bytes;
        self
    }

    pub const fn required(&self) -> usize {
        required(self.static_bound + self.external_bound, self.measured)
    }
}

/// The row of a thread.
pub const fn of(id: ThreadId) -> &'static ThreadStack {
    &THREADS[id as usize]
}

/// The boot loaders, joined one after the other by the boot thread before the next starts: they share one static stack,
/// sized for the largest of them (`runtime::threads` refuses a second start while the stack is in use).
pub const BOOT_LOADERS: [ThreadId; 3] = [ThreadId::Import, ThreadId::SettingsLoad, ThreadId::StateLoad];

/// The last user of the boot loaders' stack: a detached thread started at the end of boot (after the loaders and the
/// install line), which keeps the stack until it ends; nothing starts on that stack after it.
pub const BOOT_STACK_LAST: ThreadId = ThreadId::ImportCaptures;

/// Every thread on the boot stack: the joined loaders, then `BOOT_STACK_LAST`.
pub const BOOT_STACK: [ThreadId; 4] = [ThreadId::Import, ThreadId::SettingsLoad, ThreadId::StateLoad, ThreadId::ImportCaptures];

/// The threads that are joined (the boot thread waits for them); every other thread runs for the session or ends on its own.
pub const fn joined(id: ThreadId) -> bool {
    matches!(id, ThreadId::Import | ThreadId::SettingsLoad | ThreadId::StateLoad)
}

/// The size of the static stack shared by `ids`: the largest of their rows.
pub const fn slot_bytes(ids: &[ThreadId]) -> usize {
    let mut i = 0;
    let mut most = 0;
    while i < ids.len() {
        let bytes = of(ids[i]).bytes;
        if bytes > most {
            most = bytes;
        }
        i += 1;
    }
    most
}

/// The size of `nn::os::ThreadType` the static thread objects reserve (the SDK's 0x1c0, as the playbook's plugins use).
pub const THREAD_OBJECT_BYTES: usize = 0x1c0;

/// The smallest stack a row may have: the larger of its static bound and its measured high-water mark, plus a quarter, plus
/// the SDK reserve, rounded up to a page.
pub const fn required(static_bound: usize, measured: usize) -> usize {
    let need = if static_bound > measured { static_bound } else { measured };
    let total = need + need / 4 + SDK_RESERVE;
    total.div_ceil(4096) * 4096
}

/// Bytes used of a stack whose lowest `untouched` bytes still hold the paint.
pub const fn used(size: usize, untouched: usize) -> usize {
    size.saturating_sub(untouched)
}

/// Whether a measured high-water mark leaves less than the SDK reserve free (the diag build logs it as `low=true`).
pub const fn low(size: usize, used: usize) -> bool {
    size.saturating_sub(used) < SDK_RESERVE
}

/// The untouched depth of a painted stack, from its lowest word up (`words[0]` is the bottom).
pub fn untouched(words: &[u64]) -> usize {
    words.iter().take_while(|w| **w == PAINT).count() * 8
}
