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

use skyline::hooks::InlineCtx;

use crate::model::archive_name;
use crate::model::sites_ui::UI_ARCHIVE_ALLOC_OFFSET;
use crate::runtime::diag_log::diagnostic;

/// Bounds on what the hook will look at: a layout archive is about 1.3 MB; the largest the plugin's own
/// rebuilt archive may be is the recipe bound.
const MIN_STOCK_BYTES: usize = 256;
const MAX_STOCK_BYTES: usize = 16 * 1024 * 1024;

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
    diagnostic(format_args!("[LayoutArchive] stock info_training bytes={size}"));
    #[cfg(feature = "diag")]
    dump_stock(stock);
}

/// Diag only: write the stock archive for the host builder.
#[cfg(feature = "diag")]
fn dump_stock(stock: &[u8]) {
    use crate::model::storage::{self, FixedPath};
    let Some(dir) = FixedPath::<128>::in_root("stock") else { return };
    let _ = std::fs::create_dir_all(dir.as_str());
    let Some(path) = FixedPath::<128>::in_root("stock/info_training.arc") else { return };
    match std::fs::write(path.as_str(), stock) {
        Ok(()) => diagnostic(format_args!(
            "[LayoutArchive] dumped stock info_training bytes={} fnv={:016x}",
            stock.len(),
            crate::model::archive_recipe::fingerprint(stock)
        )),
        Err(error) => diagnostic(format_args!("[LayoutArchive] dump failed: {error}")),
    }
    let _ = storage::ROOT;
}