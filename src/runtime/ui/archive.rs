//! The layout archive hook (SPEC-core-05 §4.1, §4.2, REVISION33).
//!
//! At `main+0x3774704` the UI holder constructor has just decompressed a layout archive: x21 holds the
//! bytes and x1, x23 and x24 their size; the game allocates that size next and copies from x21. For the
//! Training HUD's archive (found by its directory only, never by parsing the payloads) the hook swaps in
//! the plugin's archive, rebuilt from the stock one by the recipe (`model::archive_recipe`), and sets the
//! size registers to match. The rebuilt archive is parked and released five seconds later by a small
//! thread: the game copies it during the hooked call sequence and never reads the pointer again.
//!
//! The diag build also writes the stock archive to `sd:/ultimate/TrainingRooms/stock/` so the host
//! builder can derive the recipe from the player's own copy (never shipped in the release NRO).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use skyline::hooks::InlineCtx;

use crate::model::archive_name;
use crate::model::archive_recipe;
use crate::model::archive_retire::retire_expired;
use crate::model::sites_ui::UI_ARCHIVE_ALLOC_OFFSET;
use crate::runtime::clock;
use crate::runtime::diag_log::diagnostic;

/// Bounds on what the hook will look at: a layout archive is about 450 KB.
const MIN_STOCK_BYTES: usize = 256;
const MAX_STOCK_BYTES: usize = 16 * 1024 * 1024;
/// The grace before a parked archive is freed (SPEC-core-05 question 5).
const GRACE_MS: u64 = 5_000;

/// The recipe that turns the stock `info_training` archive into the plugin's (built by
/// `tools/layout-builder`; copy ranges and the plugin's own bytes, never a game archive).
static RECIPE: &[u8] = include_bytes!("../../../assets/info_training.tra1");

struct Parked {
    handed_out_ms: u64,
    bytes: Vec<u8>,
}

static PARKED: Mutex<Vec<Parked>> = Mutex::new(Vec::new());
static RETIRER: AtomicBool = AtomicBool::new(false);

pub fn install() {
    skyline::install_hook!(archive_alloc);
}

/// Inline hook on the allocator call, x21 = decompressed archive, x1 = x23 = x24 = size.
#[skyline::hook(offset = UI_ARCHIVE_ALLOC_OFFSET, inline)]
unsafe fn archive_alloc(ctx: &mut InlineCtx) {
    let size = ctx.registers[1].x() as usize;
    let src = ctx.registers[21].x() as *const u8;
    if src.is_null() || !(MIN_STOCK_BYTES..=MAX_STOCK_BYTES).contains(&size) {
        return;
    }
    // SAFETY: the game decompressed `size` bytes at `src` and reads them again right after this hook.
    let stock = std::slice::from_raw_parts(src, size);
    if archive_name::contains(stock, b"blyt/info_training.bflyt") != Some(true) {
        return;
    }
    #[cfg(feature = "diag")]
    dump_stock(stock);
    let (built, base, via) = match archive_recipe::build(stock, RECIPE) {
        Ok(built) => (built, size, "stock"),
        Err(reason) => {
            // B3 (decided 2026-11-01): another mod replaced the Training layout archive. Inject anyway on the vanilla
            // archive, read through ARCropolis's `arc:/` view (the vanilla view, SPEC-core-07 U2); the other mod's Training
            // HUD does not show while this plugin is installed.
            match vanilla_base().map(|vanilla| (archive_recipe::build(&vanilla, RECIPE), vanilla.len())) {
                Some((Ok(built), len)) => {
                    diagnostic(format_args!("[LayoutArchive] stock info_training differs ({reason}); injected on the vanilla archive read through arc:/ bytes={len} (layout=other-mod)"));
                    (built, len, "vanilla")
                }
                other => {
                    let why = match other {
                        Some((Err(error), _)) => format!("vanilla base refused: {error}"),
                        _ => "vanilla base not readable".to_string(),
                    };
                    diagnostic(format_args!("[LayoutArchive] stock info_training kept: {reason}; {why} (layout_arc=skipped)"));
                    return;
                }
            }
        }
    };
    let (pointer, len) = (built.as_ptr() as u64, built.len() as u64);
    park(built);
    ctx.registers[21].set_x(pointer);
    ctx.registers[1].set_x(len);
    ctx.registers[23].set_x(len);
    ctx.registers[24].set_x(len);
    diagnostic(format_args!("[LayoutArchive] handed out info_training bytes={len} stock={base} base={via}"));
}

/// The archive path of the Training HUD layout (found 2026-11-02 from `data.arc`: the one `layout.arc` entry of the stock
/// size 451,604 B, path hash40 `0x3575fbb045`, which this string reproduces).
const VANILLA_PATH: &str = "arc:/ui/layout/info/info_training/info_training/layout.arc";

/// The vanilla Training layout archive (bounded: a layout archive is about 450 KB).
fn vanilla_base() -> Option<Vec<u8>> {
    let bytes = std::fs::read(VANILLA_PATH).ok()?;
    (MIN_STOCK_BYTES..=MAX_STOCK_BYTES).contains(&bytes.len()).then_some(bytes)
}
fn park(bytes: Vec<u8>) {
    if let Ok(mut parked) = PARKED.lock() {
        parked.push(Parked { handed_out_ms: clock::uptime_ms(), bytes });
    }
    if !RETIRER.swap(true, Ordering::AcqRel) {
        let spawned = std::thread::Builder::new().name("tr-archive".into()).stack_size(64 * 1024).spawn(|| loop {
            std::thread::sleep(Duration::from_secs(1));
            retire_expired(&PARKED, clock::uptime_ms(), GRACE_MS, |p| p.handed_out_ms, |p| {
                diagnostic(format_args!(
                    "[LayoutArchive] released info_training bytes={} after={}ms",
                    p.bytes.len(),
                    clock::uptime_ms().saturating_sub(p.handed_out_ms)
                ));
            });
        });
        if spawned.is_err() {
            RETIRER.store(false, Ordering::Release);
        }
    }
}

/// Diag only: write the stock archive for the host builder.
#[cfg(feature = "diag")]
fn dump_stock(stock: &[u8]) {
    use crate::model::storage::FixedPath;
    let Some(dir) = FixedPath::<128>::in_root("stock") else { return };
    let _ = std::fs::create_dir_all(dir.as_str());
    let Some(path) = FixedPath::<128>::in_root("stock/info_training.arc") else { return };
    match std::fs::write(path.as_str(), stock) {
        Ok(()) => diagnostic(format_args!(
            "[LayoutArchive] dumped stock info_training bytes={} fnv={:016x}",
            stock.len(),
            archive_recipe::fingerprint(stock)
        )),
        Err(error) => diagnostic(format_args!("[LayoutArchive] dump failed: {error}")),
    }
}