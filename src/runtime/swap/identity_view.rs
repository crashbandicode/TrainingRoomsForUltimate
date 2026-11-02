//! Per-body identity view, Switch side (WP-R; refactor/wp-r/CATALOG.md, DECISION in
//! refactor/INTEGRATION.md "WP-R — per-body identity view").
//!
//! Contract: every native reader that derives a per-body value from the entry's identity words gets
//! the body's own value. The pure rule is `crate::engine::swap::identity_view`; this file only reads
//! registers and fields and rewrites one register per site. It writes no `R`, FE or D word.
//!
//! - **Sites:** an inline hook at the exit of each of the 18 inlined costume selectors that serve a
//!   body or a request (`native_seams::rows_identity`). Each exit is reached only by its selector's
//!   branches (`wp-r/re/hook_point_preds.tsv`; the item site's extra packed path is excluded by its
//!   own flag test). When the rule's key differs from the primary the costume register is replaced;
//!   otherwise native's value stands untouched. No kind register is changed: init's `prepare_entry`
//!   keeps native's `R+0x58[sub]` (a Phase B group's stale header and roster are WP-K's open finding,
//!   CATALOG §3.1; the diag build counts the cells where it differs from the descriptor).
//! - **All or nothing (I1):** a registration and its release must use one costume per `(R, kind)`
//!   (`0x17eeaa0` ↔ `0x17ef0f0`, `0x17efb40` ↔ `0x17f0290`, acquire ↔ construct/paths). `install`
//!   hooks every site or none, checks that Skyline wrote one-word branches (a far jump would cover
//!   instructions that are branch targets, `hook_span_preds.tsv`), and otherwise puts every site's
//!   words back. Without the view the selectors stay keyed on the primary, which is today's behaviour.
//! - **Threads (I4):** the hooked copies run on the game thread, the resource worker and
//!   TaskWorker2. A hook reads registers, the `R` words its copy just read, the region code and one
//!   atomic set per owner; it takes no lock, allocates nothing and logs nothing. `publish` runs on the
//!   game thread under INSTALL, before the bank's first native call for a kind on `R` (`r_handle::acquire`
//!   for a group prefetch, since WP4),
//!   and a kind is never withdrawn while that `R` stays the owner's: one `(R, kind)` keeps one costume
//!   for the epoch (I2). A new `R` for the owner starts an empty set.

use core::fmt;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use skyline::hooks::InlineCtx;
use crate::engine::swap::identity_view::{self as view, Header, Published, Scope, NO_KIND, REGION_OFFSET};
use crate::engine::seams::native_seams::{self, GuardOutcome};

use crate::engine::seams::native_fields as fields;
use crate::runtime::seams::seam_install;

/// `G = *(main+0x5324680)`; `R_e = *(G + 0xe8 + 8e)` (`0x607a9c-0x607aa8`).
const OWNER_TABLE: usize = 0x5324680;
/// `E_e = *(*(*(main+0x52b94f8)) + 8e + 0x20)` (`0x607a28-0x607a38`); `E+0x14` count, `E+0x18..0x20`
/// descriptor kinds, `E+0xe0` owner, `E+0x5918` active cell. Diag evidence only.
#[cfg(feature = "diag")]
const ENTRY_TABLE: usize = 0x52b94f8;
const OWNERS: usize = 8;
/// The item site's flag register: bit 30 = packed costume (`0x33a0564`), bit 29 = packed owner (`0x33a0568`).
const ITEM_PACKED_COSTUME: u32 = 1 << 30;
const ITEM_PACKED_OWNER: u32 = 1 << 29;
/// Words a Skyline far jump would cover (`nop` + `ldr x17, #8; br x17; .quad`).
const SPAN_WORDS: usize = native_seams::HOOK_SPAN_WORDS;

/// The rows of `native_seams::rows_identity`, in hook order.
pub(crate) const LABELS: [&str; 18] = [
    "identity_view.acquire",
    "identity_view.acquire_joker",
    "identity_view.worker_0",
    "identity_view.worker_1",
    "identity_view.worker_2",
    "identity_view.release",
    "identity_view.path",
    "identity_view.construct",
    "identity_view.init_register",
    "identity_view.init_prepare_entry",
    "identity_view.setup_construct",
    "identity_view.setup_key_a",
    "identity_view.setup_key_b",
    "identity_view.unload",
    "identity_view.object_a",
    "identity_view.object_b",
    "identity_view.object_c",
    "identity_view.item_owner",
];

static BASE: AtomicUsize = AtomicUsize::new(0);

/// One owner's published set: valid only while `r` is that owner's `R`.
struct View {
    r: AtomicUsize,
    low: AtomicU64,
    high: AtomicU64,
}

static VIEWS: [View; OWNERS] = [const { View { r: AtomicUsize::new(0), low: AtomicU64::new(0), high: AtomicU64::new(0) } }; OWNERS];

#[cfg(feature = "diag")]
mod notes {
    //! Diag-only evidence for the Eden rows: per site, how often the view changed native's costume
    //! and the last change (kind, native costume, view costume). Atomics only.
    use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

    pub(super) static CHANGED: [AtomicU32; 18] = [const { AtomicU32::new(0) }; 18];
    pub(super) static LAST: [AtomicU64; 18] = [const { AtomicU64::new(0) }; 18];
    pub(super) static KIND_MISMATCH: AtomicU32 = AtomicU32::new(0);

    pub(super) fn note(site: usize, kind: u32, native: u32, keyed: u32) {
        CHANGED[site].fetch_add(1, Ordering::Relaxed);
        let packed = (u64::from(kind & 0xffff) << 32) | (u64::from(native & 0xffff) << 16) | u64::from(keyed & 0xffff);
        LAST[site].store(packed, Ordering::Relaxed);
    }
}

#[inline(always)]
fn object(address: usize) -> bool {
    fields::object(address)
}

#[inline(always)]
unsafe fn word(r: usize, offset: usize) -> u32 {
    fields::read_unchecked::<u32>(r, offset)
}

#[inline(always)]
unsafe fn header(r: usize) -> Header {
    Header { kinds: [word(r, 0x58), word(r, 0x5c), word(r, 0x60)], costume: word(r, 0x64) }
}

#[inline(always)]
fn published(r: usize, owner: u32) -> Published {
    let Some(v) = VIEWS.get(owner as usize) else { return Published::EMPTY; };
    if v.r.load(Ordering::Acquire) != r {
        return Published::EMPTY;
    }
    let set = Published::from_halves(v.low.load(Ordering::Acquire), v.high.load(Ordering::Acquire));
    // A concurrent epoch change (a new `R` for the owner) invalidates what was read.
    if v.r.load(Ordering::Acquire) == r { set } else { Published::EMPTY }
}

#[inline(always)]
unsafe fn region(base: usize) -> u32 {
    fields::read_unchecked::<u32>(base, REGION_OFFSET)
}

/// Replace the costume in `costume_reg` when the body's key is not the primary (the value native
/// computed from the primary otherwise stands).
#[inline(always)]
unsafe fn rekey(ctx: &mut InlineCtx, site: usize, r: usize, costume_reg: usize, scope: Scope) {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 || !object(r) {
        return;
    }
    let h = header(r);
    let key = view::key(scope, &h, published(r, word(r, 0x54)));
    if key == h.primary() {
        return;
    }
    let keyed = view::native_costume(key, h.costume, region(base));
    #[cfg(feature = "diag")]
    {
        let native = ctx.registers[costume_reg].w();
        if native != keyed { notes::note(site, key, native, keyed); }
    }
    #[cfg(not(feature = "diag"))]
    let _ = site;
    ctx.registers[costume_reg].set_x(u64::from(keyed));
}

#[inline(always)]
fn reg_w(ctx: &InlineCtx, i: usize) -> u32 {
    ctx.registers[i].w()
}

#[inline(always)]
fn reg_x(ctx: &InlineCtx, i: usize) -> usize {
    ctx.registers[i].x() as usize
}

/// Owner `e`'s entry object, checked by its owner word (`E+0xe0`).
#[cfg(feature = "diag")]
unsafe fn entry_object(base: usize, owner: u32) -> Option<usize> {
    if owner as usize >= OWNERS {
        return None;
    }
    let table = fields::chain(base, &[ENTRY_TABLE, 0])?;
    let entry = fields::child(table, owner as usize * 8 + 0x20)?;
    (fields::read::<u32>(entry, 0xe0) == Some(owner)).then_some(entry)
}

#[cfg(feature = "diag")]
unsafe fn descriptor(entry: usize) -> ([u32; 3], u32) {
    ([word(entry, 0x18), word(entry, 0x1c), word(entry, 0x20)], word(entry, 0x14))
}

// ----- the 18 sites (registers per `wp-r/re/*.asm`; rows in `native_seams::rows_identity`) -----

/// Acquire `0x17e64d0`: R x20, requested kind w19, costume w27.
#[skyline::hook(offset = 0x17e6554, inline)]
unsafe fn acquire(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 19);
    let r = reg_x(ctx, 20);
    rekey(ctx, 0, r, 27, Scope::Request(kind));
}

/// Acquire's Joker path (`R+0x48 == 1`, kind `0x52`, `R+0x7d == 0`), after its `adrp`: costume w3.
#[skyline::hook(offset = 0x17e7c80, inline)]
unsafe fn acquire_joker(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 19);
    let r = reg_x(ctx, 20);
    rekey(ctx, 1, r, 3, Scope::Request(kind));
}

/// Load completion `0x17e8440`, member 0 (`R+0x58`): R x19, kind w2, costume w3, then `0x17efb40`.
#[skyline::hook(offset = 0x17e84e4, inline)]
unsafe fn worker_0(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 2);
    let r = reg_x(ctx, 19);
    rekey(ctx, 2, r, 3, Scope::Request(kind));
}

/// Load completion, member 1 (`R+0x5c`).
#[skyline::hook(offset = 0x17e8564, inline)]
unsafe fn worker_1(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 2);
    let r = reg_x(ctx, 19);
    rekey(ctx, 3, r, 3, Scope::Request(kind));
}

/// Load completion, member 2 (`R+0x60`).
#[skyline::hook(offset = 0x17e85e4, inline)]
unsafe fn worker_2(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 2);
    let r = reg_x(ctx, 19);
    rekey(ctx, 4, r, 3, Scope::Request(kind));
}

/// Per-kind release `0x17e8610` before the mirror release `0x17f0290`: R x19, kind w20, costume w3.
#[skyline::hook(offset = 0x17e87a4, inline)]
unsafe fn release(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 20);
    let r = reg_x(ctx, 19);
    rekey(ctx, 5, r, 3, Scope::Request(kind));
}

/// Path builder `0x17e8890(out, R, kind, purpose)`: R x1, kind w20, costume w4.
#[skyline::hook(offset = 0x17e8910, inline)]
unsafe fn path(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 20);
    let r = reg_x(ctx, 1);
    rekey(ctx, 6, r, 4, Scope::Request(kind));
}

/// Construct `0x17e99c0(out, R, kind, variant)`: R x1, kind w2, costume w3.
#[skyline::hook(offset = 0x17e9b00, inline)]
unsafe fn construct(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 2);
    let r = reg_x(ctx, 1);
    rekey(ctx, 7, r, 3, Scope::Request(kind));
}

/// Body init V+0x10 before `0x17eeaa0(G68, kind, costume, …)`: R x24, the body's kind w22, costume w2.
#[skyline::hook(offset = 0x607e1c, inline)]
unsafe fn init_register(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 22);
    let r = reg_x(ctx, 24);
    rekey(ctx, 8, r, 2, Scope::Body(kind));
}

/// Body init before `prepare_entry(G68, R+0xb0, kind, costume, entry, sub)`: native passes
/// `R+0x58[sub]` in w2 and the copy tuples carry that kind; the costume (w3) is keyed on it. The
/// kind stays native's (CATALOG §3.1: a group's stale header and roster are WP-K's open finding).
/// R x24, kind w2, costume w3.
#[skyline::hook(offset = 0x607f24, inline)]
unsafe fn init_prepare_entry(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 2);
    let r = reg_x(ctx, 24);
    #[cfg(feature = "diag")]
    {
        // Evidence for the open finding: the body's descriptor kind differs from the kind native passes.
        let base = BASE.load(Ordering::Relaxed);
        if base != 0 && object(r) {
            if let Some(entry) = entry_object(base, word(r, 0x54)) {
                let (kinds, count) = descriptor(entry);
                let sub = ctx.registers[25].x() as i64;
                if (0..i64::from(count.min(3))).contains(&sub) && kinds[sub as usize] != kind {
                    notes::KIND_MISMATCH.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
    rekey(ctx, 9, r, 3, Scope::Body(kind));
}

/// Body setup V+0x18, construct-path costume: R x20, the body's kind w8 (`body+0xc`), costume w4.
#[skyline::hook(offset = 0x60c144, inline)]
unsafe fn setup_construct(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 8);
    let r = reg_x(ctx, 20);
    rekey(ctx, 10, r, 4, Scope::Body(kind));
}

/// Body setup, key builder `0x17df420(…, kind, 0x24, costume, R+0x7d)`: R x20, kind w1, costume w3.
#[skyline::hook(offset = 0x60d6a8, inline)]
unsafe fn setup_key_a(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 1);
    let r = reg_x(ctx, 20);
    rekey(ctx, 11, r, 3, Scope::Body(kind));
}

/// Body setup, key builder `0x17df420(…, kind, 0x10, costume, R+0x7d)`.
#[skyline::hook(offset = 0x60d9bc, inline)]
unsafe fn setup_key_b(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 1);
    let r = reg_x(ctx, 20);
    rekey(ctx, 12, r, 3, Scope::Body(kind));
}

/// Body unload V+0x50 before `0x17ef0f0(G68, kind, costume, …)`: R x27, kind w20 (`body+0xc`), costume w2.
#[skyline::hook(offset = 0x611b94, inline)]
unsafe fn unload(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 20);
    let r = reg_x(ctx, 27);
    rekey(ctx, 13, r, 2, Scope::Body(kind));
}

/// Object `0x6e6450` (fighter kind `+0x10`, entry `+0x14`), costume stored to `+0x7ec`: R x21, costume w8.
#[skyline::hook(offset = 0x6e65b8, inline)]
unsafe fn object_a(ctx: &mut InlineCtx) {
    let obj = reg_x(ctx, 19);
    if !object(obj) {
        return;
    }
    let kind = word(obj, 0x10);
    let r = reg_x(ctx, 21);
    rekey(ctx, 14, r, 8, Scope::Request(kind));
}

/// Object `0x6e6b70` (same layout): R x20, costume w8.
#[skyline::hook(offset = 0x6e6d28, inline)]
unsafe fn object_b(ctx: &mut InlineCtx) {
    let obj = reg_x(ctx, 19);
    if !object(obj) {
        return;
    }
    let kind = word(obj, 0x10);
    let r = reg_x(ctx, 20);
    rekey(ctx, 15, r, 8, Scope::Request(kind));
}

/// Object `0x33c3370(obj, kind)`: R x21, kind w20, costume w8 (stored to `+0x7ec`).
#[skyline::hook(offset = 0x33c34c4, inline)]
unsafe fn object_c(ctx: &mut InlineCtx) {
    let kind = reg_w(ctx, 20);
    let r = reg_x(ctx, 21);
    rekey(ctx, 16, r, 8, Scope::Request(kind));
}

/// Weapon init `0x33a0490`: the weapon's costume byte (`+0x9a`), which keys its file node
/// (`0x17eeee0`, lookup `0x17def20` at `0x33a0974`). The selector ran on the owner entry's `R`
/// (`G+0xe8+8e`, `0x33a0598-0x33a05a4`); the key is the weapon's own fighter, native's table
/// `WEAPON_OWNER[+0xc]`, as a request on that `R`. Not the active body: a body builds its article
/// pools inside its own init on the loader fiber, and the canonical Reload builds every window
/// body while another is active (C116.1 R.O.B. Gyro crash). The packed path (w21 bit 30, the
/// creator's costume) is left alone.
#[skyline::hook(offset = 0x33a0604, inline)]
unsafe fn item_owner(ctx: &mut InlineCtx) {
    let base = BASE.load(Ordering::Relaxed);
    let flags = reg_w(ctx, 21);
    let obj = reg_x(ctx, 19);
    if base == 0 || flags & ITEM_PACKED_COSTUME != 0 || !object(obj) {
        return;
    }
    let owner = if flags & ITEM_PACKED_OWNER != 0 { (flags >> 11) & 0x7ff } else { word(obj, 0x10) };
    if owner as usize >= OWNERS {
        return;
    }
    let Some(r) = fields::chain(base, &[OWNER_TABLE, 0xe8 + owner as usize * 8]) else { return; };
    rekey(ctx, 17, r, 8, Scope::Request(view::weapon_owner(word(obj, 0xc))));
}

// ----- publication (game thread, the bank) -----

/// The bank is about to make `kind` a body of `r` (prefetch of a detached group, a Mii group's
/// key request): from now on every request for `kind` on `r` keys on `kind`. Call it before the
/// first native call for that kind on `r`.
pub(crate) unsafe fn publish(r: usize, kind: u32) {
    if !object(r) || kind == NO_KIND {
        return;
    }
    let owner = word(r, 0x54);
    let Some(v) = VIEWS.get(owner as usize) else { return; };
    let same_epoch = v.r.load(Ordering::Acquire) == r;
    let current = if same_epoch {
        Published::from_halves(v.low.load(Ordering::Acquire), v.high.load(Ordering::Acquire))
    } else {
        Published::EMPTY
    };
    let next = current.with(kind);
    if same_epoch && next == current {
        return;
    }
    let (low, high) = next.halves();
    if !same_epoch {
        v.r.store(0, Ordering::Release);
    }
    v.low.store(low, Ordering::Release);
    v.high.store(high, Ordering::Release);
    v.r.store(r, Ordering::Release);
    #[cfg(feature = "diag")]
    crate::engine::compat::logging::diagnostic(format_args!(
        "[BodyBank] identity_view publish owner={} R={:#x} kind={:#x} epoch={} published={:#x}",
        owner, r, kind, if same_epoch { "same" } else { "new" }, next.0
    ));
}

// ----- install -----

struct Sites<'a>(&'a [Option<GuardOutcome>; 18]);

impl fmt::Display for Sites<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        for (i, (label, outcome)) in LABELS.iter().zip(self.0.iter()).enumerate() {
            if i > 0 {
                f.write_str(",")?;
            }
            let short = label.trim_start_matches("identity_view.");
            write!(f, "{}:{}", short, outcome.map_or("missing", GuardOutcome::code))?;
        }
        f.write_str("]")
    }
}

/// The site's words now, read from main `.text`.
unsafe fn site_words(base: usize, offset: usize) -> [u32; SPAN_WORDS] {
    core::array::from_fn(|i| fields::read_unchecked::<u32>(base, offset + 4 * i))
}

fn guard_words(guard: &[u8; native_seams::GUARD_BYTES]) -> [u32; SPAN_WORDS] {
    core::array::from_fn(|i| u32::from_le_bytes([guard[4 * i], guard[4 * i + 1], guard[4 * i + 2], guard[4 * i + 3]]))
}

/// Skyline wrote a one-word `b` at the site and left the next words as they were.
unsafe fn one_word_hook(base: usize, offset: usize, guard: &[u8; native_seams::GUARD_BYTES]) -> bool {
    let now = site_words(base, offset);
    let stock = guard_words(guard);
    now[0] & 0xfc00_0000 == 0x1400_0000 && now[1..] == stock[1..]
}

/// The running game's weapon→fighter table (`main+0x455e7e4`, read-only data) is the 13.0.5 table
/// the weapon site keys on.
unsafe fn weapon_owner_table_matches(base: usize) -> bool {
    view::WEAPON_OWNER.iter().enumerate().all(|(w, &owner)| {
        fields::read_unchecked::<u32>(base, view::WEAPON_OWNER_OFFSET + 4 * w) == u32::from(owner)
    })
}

/// Hook every site or none (I1). `base` is main `.text`. Returns whether the view is active.
pub(crate) unsafe fn install(base: usize) -> bool {
    BASE.store(base, Ordering::Release);
    let mut outcomes = [None; 18];
    for (outcome, label) in outcomes.iter_mut().zip(LABELS) {
        *outcome = seam_install::check(label);
    }
    // Today's (legacy) decision for a new module: every site byte-identical to 13.0.5.
    let all_stock = outcomes.iter().all(|o| *o == Some(GuardOutcome::Stock));
    if !seam_install::gate("identity_view", &LABELS, all_stock) {
        BASE.store(0, Ordering::Release);
        crate::engine::compat::logging::diagnostic(format_args!(
            "[BodyBank] identity_view result=not_installed reason=site guard sites={}", Sites(&outcomes)));
        return false;
    }
    if !weapon_owner_table_matches(base) {
        BASE.store(0, Ordering::Release);
        crate::engine::compat::logging::diagnostic(format_args!(
            "[BodyBank] identity_view result=not_installed reason=weapon owner table sites={}", Sites(&outcomes)));
        return false;
    }
    skyline::install_hooks!(
        acquire, acquire_joker, worker_0, worker_1, worker_2, release, path, construct, init_register,
        init_prepare_entry, setup_construct, setup_key_a, setup_key_b, unload, object_a, object_b, object_c,
        item_owner
    );
    let rows: [Option<&'static native_seams::Site>; 18] = core::array::from_fn(|i| native_seams::find_site(LABELS[i]).map(|(_, s)| s));
    let shapes_ok = rows.iter().all(|row| row.is_some_and(|s| one_word_hook(base, s.offset, &s.guard)));
    if !shapes_ok {
        // A far jump would cover branch targets after some sites: put every site back (I1).
        BASE.store(0, Ordering::Release);
        let mut restored = 0;
        for seam in rows.iter().flatten() {
            if skyline::patching::Patch::in_text(seam.offset).bytes(&seam.guard[..4 * SPAN_WORDS]).is_ok() {
                restored += 1;
            }
        }
        crate::engine::compat::logging::diagnostic(format_args!(
            "[BodyBank] identity_view result=restored reason=hook shape restored={} sites={}", restored, Sites(&outcomes)));
        return false;
    }
    crate::engine::compat::logging::diagnostic(format_args!(
        "[BodyBank] identity_view result=installed sites={} region={} weapon_owner_table=match", Sites(&outcomes), region(base)));
    true
}

// ----- diag evidence -----

#[cfg(feature = "diag")]
struct Changes;

#[cfg(feature = "diag")]
impl fmt::Display for Changes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        let mut first = true;
        for (i, label) in LABELS.iter().enumerate() {
            let n = notes::CHANGED[i].load(Ordering::Relaxed);
            if n == 0 {
                continue;
            }
            if !first {
                f.write_str(",")?;
            }
            first = false;
            let last = notes::LAST[i].load(Ordering::Relaxed);
            write!(f, "{}:{}:{:#x}/{}->{}", label.trim_start_matches("identity_view."), n,
                (last >> 32) & 0xffff, (last >> 16) & 0xffff, last & 0xffff)?;
        }
        f.write_str("]")
    }
}

/// Diag: the owner's view at a bank milestone (`[BodyBank] ready`), for the Eden rows.
#[cfg(feature = "diag")]
pub(crate) unsafe fn report(owner: u32, r: usize, reason: &str) {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 || !object(r) {
        return;
    }
    let h = header(r);
    crate::engine::compat::logging::diagnostic(format_args!(
        "[BodyBank] identity_view owner={} reason={} R={:#x} header={:x?} costume={} region={} published={:#x} prepare_entry_kind_mismatch={} changed={}",
        owner, reason, r, h.kinds, h.costume, region(base), published(r, owner).0,
        notes::KIND_MISMATCH.load(Ordering::Relaxed), Changes));
}
