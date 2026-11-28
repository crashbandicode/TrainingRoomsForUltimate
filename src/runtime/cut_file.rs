//! Observes picture copies at `main+0x37a2dd4` before the game's relocation cast.
//! A detection writes one fixed card line without a heap allocation or plugin lock.
//! See `docs/DECISIONS.md#rc16-cut-file-diagnostic`.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use skyline::hooks::InlineCtx;
use skyline::nn::fs::{self, FileHandle, WriteOption};

use crate::model::cut_file::{self, BootLine, CopyMeta, Records, Sink};
use crate::model::hook_stats::{HookId, HookScope};
use crate::runtime::{clock, diag_log, seams::seam_install};

static RECORDS: Records = Records::new();
static INIT_MS: AtomicU64 = AtomicU64::new(u64::MAX);
static BOOT_REPORTED: AtomicBool = AtomicBool::new(false);
static REPORTED_OVERFLOW: AtomicU32 = AtomicU32::new(0);
static API_ADDRESS: AtomicUsize = AtomicUsize::new(0);
static API_LAST: AtomicU64 = AtomicU64::new(u64::MAX);
static GUARANTEE_ADDRESS: AtomicUsize = AtomicUsize::new(0);
static GUARANTEE_LAST: AtomicU64 = AtomicU64::new(u64::MAX);

fn lookup(address: &AtomicUsize, last: &AtomicU64, name: &[u8]) -> Option<usize> {
    let cached = address.load(Ordering::Acquire);
    if cached != 0 { return Some(cached); }
    let now = clock::uptime_ms();
    let before = last.load(Ordering::Acquire);
    if before != u64::MAX && now.saturating_sub(before) < 1000 { return None; }
    if last.compare_exchange(before, now, Ordering::AcqRel, Ordering::Acquire).is_err() { return None; }
    let mut found = 0usize;
    // SAFETY: both symbol names are NUL-terminated and `found` is writable for one usize.
    unsafe { skyline::nn::ro::LookupSymbol(&mut found, name.as_ptr()) };
    if found == 0 { return None; }
    address.store(found, Ordering::Release);
    Some(found)
}

fn guarantee() -> Option<u32> {
    let address = lookup(&GUARANTEE_ADDRESS, &GUARANTEE_LAST, cut_file::GUARANTEE_SYMBOL)?;
    // SAFETY: the fork declares this export with this C ABI and signature.
    let function = unsafe { core::mem::transmute::<usize, unsafe extern "C" fn() -> u32>(address) };
    // SAFETY: the address is the resolved export with the fork's declared ABI.
    let value = unsafe { function() };
    (value != 0).then_some(value)
}

fn api_version() -> Option<(u32, u32)> {
    let address = lookup(&API_ADDRESS, &API_LAST, cut_file::API_SYMBOL)?;
    // SAFETY: ARCropolis declares this API export with this C ABI and signature.
    let function = unsafe { core::mem::transmute::<usize, unsafe extern "C" fn() -> *const [u32; 2]>(address) };
    // SAFETY: the resolved export returns a null or valid pointer to its two version words.
    let version = unsafe { function().as_ref()? };
    Some((version[0], version[1]))
}

struct NativeSink {
    handle: FileHandle,
}

impl NativeSink {
    fn new() -> Self { Self { handle: FileHandle { handle: 0 } } }
}

fn result(code: u32) -> Result<(), u32> {
    if code == 0 { Ok(()) } else { Err(code) }
}

impl Sink for NativeSink {
    fn create(&mut self) {
        // SAFETY: the path is NUL-terminated and remains valid for the call.
        unsafe { fs::CreateFile(cut_file::PATH.as_ptr(), 0) };
    }

    fn open(&mut self) -> Result<(), u32> {
        // SAFETY: the path is NUL-terminated and the handle output is writable.
        result(unsafe { fs::OpenFile(&mut self.handle, cut_file::PATH.as_ptr(), 3) })?;
        if self.handle.handle == 0 { return Err(0xffff_fffd); }
        Ok(())
    }

    fn size(&mut self) -> Result<u64, u32> {
        let mut size = 0i64;
        // SAFETY: this sink opened the handle and `size` is writable for the call.
        result(unsafe { fs::GetFileSize(&mut size, self.handle) })?;
        u64::try_from(size).map_err(|_| 0xffff_fffd)
    }

    fn grow(&mut self, size: u64) -> Result<(), u32> {
        // SAFETY: this sink opened the handle; the bounded size fits i64.
        result(unsafe { fs::SetFileSize(self.handle, size as i64) })
    }

    fn write_flush(&mut self, offset: u64, bytes: &[u8]) -> Result<(), u32> {
        // SAFETY: this sink opened the handle, and the buffer and write option live through the call.
        result(unsafe {
            fs::WriteFile(self.handle, offset as i64, bytes.as_ptr(), bytes.len() as u64, &WriteOption { flags: 1 })
        })
    }

    fn close(&mut self) {
        // SAFETY: this sink opened the handle and closes it once after the append.
        unsafe { fs::CloseFile(self.handle) }
    }
}

#[skyline::hook(offset = 0x37a2dd4, inline)]
unsafe fn picture_copied(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::CutFile);
    let copy = ctx.registers[21].x();
    let len = ctx.registers[20].x() as usize;
    if copy == 0 || len < cut_file::MIN_BYTES { return; }
    // SAFETY: x21 is the game's own block of x20 bytes that memcpy has just filled.
    let bytes = unsafe { core::slice::from_raw_parts(copy as *const u8, len) };
    let texture = ctx.registers[19].x();
    let index = if texture == 0 { 0 } else {
        // SAFETY: x19 is the game's texture record, read by this function at its entry.
        unsafe { (texture as *const u32).read_unaligned() }
    };
    cut_file::on_copy(&RECORDS, &mut NativeSink::new(), bytes,
        CopyMeta { index, address: copy, at_ms: clock::uptime_ms() }, guarantee);
}

/// Install the observer only after its seam guard succeeds.
pub(crate) fn init() {
    INIT_MS.store(clock::uptime_ms(), Ordering::Release);
    let installed = seam_install::gate("cut_file", &[cut_file::ROW], true);
    if installed { skyline::install_hooks!(picture_copied); }
    diag_log::diagnostic(format_args!("[TrainingRooms] cut_file installed={installed}"));
}

/// Report the ARCropolis boot line, published findings, and any overflow on the drain thread.
pub(crate) fn service() {
    let start = INIT_MS.load(Ordering::Acquire);
    if start == u64::MAX { return; }
    if !BOOT_REPORTED.load(Ordering::Relaxed) {
        let api = api_version();
        let marker = guarantee();
        if api.is_some() || clock::uptime_ms().saturating_sub(start) >= 30_000 {
            diag_log::diagnostic(format_args!("[TrainingRooms] {}", BootLine { api, guarantee: marker }));
            BOOT_REPORTED.store(true, Ordering::Relaxed);
        }
    }
    while let Some(entry) = RECORDS.take() {
        diag_log::diagnostic(format_args!("[TrainingRooms] {entry} persisted={:#x}", entry.persisted));
    }
    let overflow = RECORDS.overflow();
    if overflow > REPORTED_OVERFLOW.load(Ordering::Relaxed) {
        REPORTED_OVERFLOW.store(overflow, Ordering::Relaxed);
        diag_log::diagnostic(format_args!("[TrainingRooms] cut_file more={overflow}"));
    }
}
