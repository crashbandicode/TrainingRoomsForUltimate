//! The layout archive hook (SPEC-core-05 §4.1, §4.2, REVISION33).
//!
//! At `main+0x3774704` the UI holder constructor has just decompressed a layout archive: x21 holds the
//! bytes and x1, x23 and x24 their size; the game allocates that size next and copies from x21. For the
//! Training or waiting HUD archive (found by its directory only, never by parsing the payloads) the hook swaps in
//! the plugin's archive, rebuilt from the stock one by the recipe (`model::archive_recipe`), and sets the
//! size registers to match. The rebuilt archive is parked and handed back to the free buffers five seconds later by a small
//! thread: the game copies it during the hooked call sequence and never reads the pointer again.
//!
//! No heap in the hook (R2-7, playbook): the two working buffers are made once at boot (`prepare`, sized from the
//! recipe headers, using the larger layout) and reused for every load; the retire thread is made there too. The hook runs on the
//! game's task workers, so the buffers pass between it and the retire thread with no lock (`model::archive_slots`, rc.6 freeze
//! class). A load that finds no free buffer keeps the game's archive (the menu then cannot open, R2-2). The vanilla base of
//! decision B3 is read once into the second buffer (`build_on_vanilla`: no seek on an `arc:` file), so B3 needs both buffers
//! free: a load within the grace of the previous one keeps the game's archive.
//!
//! The diag build also writes the stock archive to `sd:/ultimate/TrainingRooms/stock/` so the host
//! builder can derive the recipe from the player's own copy (never shipped in the release NRO).

use std::time::Duration;

use skyline::hooks::InlineCtx;

use crate::model::archive_name;
use crate::model::archive_recipe;
use crate::model::archive_slots::{Claim, Slots};
use crate::model::hook_stats::{load_allocations, per_load, HookId, HookScope};
use crate::model::sites_ui::UI_ARCHIVE_ALLOC_OFFSET;
use crate::runtime::clock;
use crate::runtime::diag_log::diagnostic;

/// Bounds on what the hook will look at, including the larger waiting archive.
const MIN_STOCK_BYTES: usize = 256;
const MAX_STOCK_BYTES: usize = 16 * 1024 * 1024;
/// The grace before a parked archive goes back to the pool (SPEC-core-05 question 5).
const GRACE_MS: u64 = 5_000;
/// Working buffers made at boot: one for the archive being handed out, one for the previous load still inside its grace.
const BUFFERS: usize = 2;

/// Recipes that turn the stock HUD archives into the plugin's (built by
/// `tools/layout-builder`; copy ranges and the plugin's own bytes, never a game archive).
struct Layout {
    name: &'static str,
    member: &'static [u8],
    recipe: &'static [u8],
    vanilla_path: &'static [u8],
    #[cfg(feature = "diag")]
    dump_path: &'static str,
}

static LAYOUTS: [Layout; 2] = [
    Layout {
        name: "info_training",
        member: b"blyt/info_training.bflyt",
        recipe: include_bytes!("../../../assets/info_training.tra1"),
        vanilla_path: b"arc:/ui/layout/info/info_training/info_training/layout.arc\0",
        #[cfg(feature = "diag")]
        dump_path: "stock/info_training.arc",
    },
    Layout {
        name: "info_waiting",
        member: b"blyt/info_waiting.bflyt",
        recipe: include_bytes!("../../../assets/info_waiting.tra1"),
        vanilla_path: b"arc:/ui/layout/info/info_waiting/info_waiting/layout.arc\0",
        #[cfg(feature = "diag")]
        dump_path: "stock/info_waiting.arc",
    },
];

/// The working buffers (made by `prepare`, each with the larger recipe's size as capacity), handed between the hook and the retire
/// thread without a lock (`model::archive_slots`): the hook runs on the game's task workers, so it must never wait on the retire
/// thread (rc.6 freeze class).
static SLOTS: Slots<BUFFERS> = Slots::new();

pub fn install() {
    skyline::install_hook!(archive_alloc);
}

/// Boot (not a hook): everything a hand-out would otherwise allocate: the working buffers and the thread that returns a handed-out
/// archive to the free buffers after its grace. The thread looks once a second; the hook never wakes it (waking a thread can wait).
pub fn prepare() {
    let capacity = LAYOUTS.iter().filter_map(|layout| archive_recipe::header(layout.recipe).ok())
        .map(|head| head.output_len.max(head.stock_len)).max().unwrap_or(0);
    SLOTS.prepare(capacity);
    diagnostic(format_args!("[LayoutArchive] boot buffers {BUFFERS} x {capacity} B"));
    let spawned = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::Archive, || loop {
        SLOTS.retire_expired(clock::uptime_ms(), GRACE_MS, |layout, len, handed_out_ms| {
            diagnostic(format_args!(
                "[LayoutArchive] released {} bytes={len} after={}ms",
                LAYOUTS[layout].name,
                clock::uptime_ms().saturating_sub(handed_out_ms)
            ));
        });
        std::thread::sleep(Duration::from_secs(1));
    });
    if let Err(error) = spawned {
        diagnostic(format_args!("[LayoutArchive] retire thread did not start: {error}"));
    }
}

/// Inline hook on the allocator call, x21 = decompressed archive, x1 = x23 = x24 = size.
#[skyline::hook(offset = UI_ARCHIVE_ALLOC_OFFSET, inline)]
unsafe fn archive_alloc(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::UiArchive);
    let size = ctx.registers[1].x() as usize;
    let src = ctx.registers[21].x() as *const u8;
    if src.is_null() || !(MIN_STOCK_BYTES..=MAX_STOCK_BYTES).contains(&size) {
        return;
    }
    // SAFETY: the game decompressed `size` bytes at `src` and reads them again right after this hook.
    let stock = std::slice::from_raw_parts(src, size);
    let Some(index) = LAYOUTS.iter().position(|layout| archive_name::contains(stock, layout.member) == Some(true)) else { return };
    let layout = &LAYOUTS[index];
    #[cfg(feature = "diag")]
    crate::model::hook_stats::uncounted(|| dump_stock(stock, layout));
    // `per_load` is the measuring section: the goal is that nothing inside it allocates (`load_allocations=0`).
    let Some((mut built, base, via)) = per_load(|| rebuilt(stock, size, layout)) else { return };
    let (pointer, len) = (built.bytes().as_ptr() as u64, built.bytes().len() as u64);
    // Parked until its grace is over: the game copies it during the hooked call sequence and never reads the pointer again.
    built.park(clock::uptime_ms(), index);
    ctx.registers[21].set_x(pointer);
    ctx.registers[1].set_x(len);
    ctx.registers[23].set_x(len);
    ctx.registers[24].set_x(len);
    diagnostic(format_args!("[LayoutArchive] handed out {} bytes={len} stock={base} base={via} load_allocations={}", layout.name, load_allocations()));
}

/// The plugin's archive for the one the game just decompressed: from the stock archive, or (B3: another mod replaced
/// it) from the vanilla archive read through `arc:/`. `None` leaves the game's archive in place (the menu then cannot open).
fn rebuilt(stock: &[u8], size: usize, layout: &'static Layout) -> Option<(Claim<'static, BUFFERS>, usize, &'static str)> {
    #[cfg(feature = "diag")]
    if crate::model::hook_stats::uncounted(refusal_forced) {
        diagnostic(format_args!("[LayoutArchive] refusal forced by refuse-archive.flag (diag): stock {} kept (layout_arc=skipped)", layout.name));
        return None;
    }
    let Some(mut out) = SLOTS.claim() else {
        diagnostic(format_args!("[LayoutArchive] no free archive buffer: stock {} kept (layout_arc=skipped)", layout.name));
        return None;
    };
    let mut source = stock;
    let reason = match archive_recipe::build_into(&mut source, layout.recipe, out.bytes()) {
        Ok(()) => return Some((out, size, "stock")),
        Err(reason) => reason,
    };
    // B3 (decided 2026-11-01): another mod replaced the layout archive. Inject anyway on the vanilla archive,
    // read through ARCropolis's arc:/ view (the vanilla view, SPEC-core-07 U2); the other mod's HUD does not show
    // while this plugin is installed.
    match build_on_vanilla(out.bytes(), layout) {
        Ok(len) => {
            diagnostic(format_args!("[LayoutArchive] stock {} differs ({reason}); injected on the vanilla archive read through arc:/ bytes={len} (layout=other-mod)", layout.name));
            Some((out, len, "vanilla"))
        }
        Err(why) => {
            diagnostic(format_args!("[LayoutArchive] stock {} kept: {reason}; {why} (layout_arc=skipped)", layout.name));
            None
        }
    }
}

/// Rebuild into `out` from the vanilla file; its length when it worked. The file is read once, front to back, into a second
/// working buffer (one `read` loop, no `seek` and no `fstat`: Eden s5-21 and s5-22 saw the game's file layer read null pointers,
/// 128 and 430 times, after those calls on an `arc:` file). The vanilla length is the recipe's own stock length.
fn build_on_vanilla(out: &mut Vec<u8>, layout: &Layout) -> Result<usize, &'static str> {
    let len = archive_recipe::header(layout.recipe)?.stock_len;
    let Some(mut scratch) = SLOTS.claim() else { return Err("no free second archive buffer") };
    read_vanilla(scratch.bytes(), len, layout.vanilla_path)?;
    let mut source: &[u8] = scratch.bytes();
    archive_recipe::build_into(&mut source, layout.recipe, out).map(|()| len)
}

/// Fill `buffer` with the first `len` bytes of the vanilla archive (it never grows: `len` is within its boot capacity).
fn read_vanilla(buffer: &mut Vec<u8>, len: usize, path: &'static [u8]) -> Result<(), &'static str> {
    if len > buffer.capacity() {
        return Err("vanilla base larger than the boot buffer");
    }
    // Opened through the SDK with a static NUL-terminated path: `File::open` on the Switch std builds its own path copy on the heap
    // (Eden s5-23: `load_allocations=1`), this call allocates nothing. Sequential reads at increasing offsets, no size query.
    let mut handle = skyline::nn::fs::FileHandle { handle: 0 };
    // SAFETY: every layout path is NUL-terminated and static; mode 1 is read.
    let opened = unsafe { skyline::nn::fs::OpenFile(&mut handle, path.as_ptr(), 1) };
    if opened != 0 || handle.handle == 0 {
        return Err("vanilla base not readable");
    }
    buffer.resize(len, 0);
    let mut filled = 0usize;
    let mut result = Ok(());
    while filled < len {
        let mut got = 0u64;
        // SAFETY: the destination is the unfilled tail of `buffer` (`len - filled` bytes).
        let read = unsafe {
            skyline::nn::fs::ReadFile1(&mut got, handle, filled as i64, buffer[filled..].as_mut_ptr(), (len - filled) as u64)
        };
        if read != 0 || got == 0 {
            result = Err("vanilla base not readable");
            break;
        }
        filled += usize::try_from(got).unwrap_or(len - filled).min(len - filled);
    }
    // SAFETY: `handle` was opened above and is closed once.
    unsafe { skyline::nn::fs::CloseFile(handle) };
    result
}

/// Diag only: write the stock archive for the host builder.
#[cfg(feature = "diag")]
fn dump_stock(stock: &[u8], layout: &Layout) {
    use crate::model::storage::FixedPath;
    let Some(dir) = FixedPath::<128>::in_root("stock") else { return };
    let _ = std::fs::create_dir_all(dir.as_str());
    let Some(path) = FixedPath::<128>::in_root(layout.dump_path) else { return };
    match std::fs::write(path.as_str(), stock) {
        Ok(()) => diagnostic(format_args!(
            "[LayoutArchive] dumped stock {} bytes={} fnv={:016x}",
            layout.name,
            stock.len(),
            archive_recipe::fingerprint(stock)
        )),
        Err(error) => diagnostic(format_args!("[LayoutArchive] dump failed: {error}")),
    }
}

/// Diag only: the Eden row of review R2-2 (a refused archive) creates `refuse-archive.flag` in the plugin's folder.
#[cfg(feature = "diag")]
fn refusal_forced() -> bool {
    use crate::model::storage::FixedPath;
    FixedPath::<128>::in_root("refuse-archive.flag").is_some_and(|path| std::fs::metadata(path.as_str()).is_ok())
}
