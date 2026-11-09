//! Seam installer (WP-A core, WORKPLAN §2.7): reads each site's 64 bytes, compares them with the
//! 13.0.5 bytes in `crate::engine::seams::native_seams`, records the outcome, and writes the one
//! install line at the end of `main`. The version guard (lib.rs, WP-B) runs before any of this.
//!
//! Shadow first: a group not in `AUTHORITATIVE` keeps its module's own (legacy) enable decision,
//! and the 64-byte outcome is recorded and reported (`shadow_diff=` when they disagree). An A-WP
//! adds its group to `AUTHORITATIVE` (one line) once its module smoke row is green; from then on the
//! outcome decides, and a site that is not usable is skipped and logged instead of installed. Never
//! panics, never allocates: outcomes live in fixed tables of atomics sized from the row files at compile
//! time (`native_seams::COUNT`, `IMAGE_COUNT`), so every row has a slot.
//!
//! Non-`main` rows (`native_seams::images`) flip separately through `AUTHORITATIVE_IMAGES`: their
//! 64 bytes are read when their image loads, after `main`, so a group's main rows can be enforcing
//! (boot smoke green) while its image rows stay shadow until the late report makes them observable
//! (Eden run `m4a`, 2026-10-31: image rows read `unchecked` at boot; R-A0b reads the late report).
//!
//! Late report (R-A0b): a row checked first after the install line (a runtime-only row, or an image
//! row whose NRO loads later), or whose outcome or flags change after it, is marked pending in the
//! same tables; the `practice-heartbeat` thread writes one `[TrainingSeams] late group=…` or
//! `late image=…` line per settled group or image (`report_late`, rules in
//! `native_seams::report`). Hook paths only update the slot atomics.

use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};

use skyline::hooks::{getRegionAddress, Region};
use crate::engine::seams::native_seams::report::{
    self, code_name, listed_as_other, other_with_code, CODE_MASK, DATA, EXTERNAL, FOREIGN, OCCUPIED, OTHER,
    OUT_OF_REGION, SHADOW_DIFF, SHARED, SKIPPED, STOCK, UNCAPTURED, UNCHECKED,
};
use crate::engine::seams::native_seams::{self, GuardOutcome, ImageSite, Locator, MainRegion, Site, GUARD_BYTES};

use crate::engine::compat::logging;
use crate::model::own_patches::{self, OwnPatch, SPAN};
use crate::model::version as model;

/// Groups whose 64-byte outcome decides installation (the per-module flip). Empty = all shadow.
/// Flipped 2026-10-31 after the Yoga Eden runs `m4a` (M4 NRO) and `m4b1` (M4b NRO): `shadow_diff=[]`
/// for every group, R-A3 PASS incl. the hitbox bubbles (`refactor/eden-m4/PROGRESS.md`; operator: the
/// guards ship enforcing in C116.1). `cpu_pilot` flipped for M4d after CP0 + CP1 PASS on the Yoga run
/// `m4c1`, with `cpu_pilot.ai_think` reading our own set_cpu_controls far jump as stock (`OwnHook`).
/// The A2 Mii groups (every `mii_*` group in the row tables) flipped for M5h (operator, SEAMS-MII-AUTH):
/// every install line of the M5e/M5f Eden runs (Yoga sweep and LDN online) has `skipped=[]` and
/// `shadow_diff=[]`, no `mii_*` skip or late line; the one non-stock Mii row there,
/// `mii_editor_bridge.scene_queue` (ARCropolis's far hook, `other`), is a `Called` row, which another
/// plugin's hook never blocks, so it stays usable. There are no Mii image rows.
/// The remaining groups compiled into the release (FULL) or diag build flipped for M5i (operator,
/// SEAMS-ALL): every M5e-M5h install line has `skipped=[]` and `shadow_diff=[]`, no `result=skipped`
/// line, every late `game_mode` / `character_sources` line all stock, and each module's legacy verdict
/// logged true (`[NativeUnlink] … guard enabled`, `[NativeGcGuard] … guard enabled`, `verified=true`).
/// `live_replay` flipped with them once `Sharing::EntryOrBody` is applied at run time
/// (`native_seams::entry_or_body_outcome`): smash_replay_to_mp4's far jump over `replay.write_record`
/// (every card log: `save=shared`) reads `shared` and the adapter takes the body, as it did. Still
/// shadow: the groups no shipped build compiles (`automation_helper_*`, `kirby_resource_probe`,
/// `console_map_probe`, `native_unlink_probe`, `practice_trace`, `source_lifecycle_probe`).
const AUTHORITATIVE: &[&str] = &[
    "lobby_items",
    "lobby_persist",
    "cpu_level",
    "native_body_bank",
    "character_roster",
    "character_lifecycle",
    "native_source_capacity",
    "identity_view",
    "training",
    "hitbox_visualizer",
    "cpu_pilot",
    "mii_editor_bridge",
    "mii_fullconfig_probe",
    "mii_hud_portrait",
    "mii_identity_probe",
    "mii_ordinary_gate",
    "mii_preview_observer",
    "mii_quit_observer",
    "mii_saved_fingerprints",
    "mii_setup",
    "native_unlink_install",
    "native_gc_guard",
    "turnip_identity",
    "practice_stage_request",
    "practice_stage_observer",
    "lobby_stage_probe",
    "training_stage_rotation",
    "hazard_manager",
    "game_mode",
    "character_sources",
    "live_replay",
];

/// Groups whose non-`main` (image) rows decide installation. Image rows are checked when their image
/// loads, after the install line, so they flip on their own evidence (R-A0b). Empty = all shadow.
/// Keyed by group, like `AUTHORITATIVE`: `training` holds the three `common` rows (the lua2cpp_common
/// replace hooks), `turnip_identity` the twelve `item` rows (ITEM_CODE). Flipped for M5h (operator,
/// SEAMS-MII-AUTH): every `late image=common` / `late image=item` line of the M5e/M5f/M5g Eden runs
/// reads all rows stock (`unusable=[] skipped=[] shadow_diff=[] unchecked=0`), with no image skip or
/// capture line. The `sdk` rows (`console_map_probe`, `console_load_probe`: probes the shipped builds
/// do not check) stay shadow.
const AUTHORITATIVE_IMAGES: &[&str] = &["training", "turnip_identity"];

const MAX_MISSING: usize = 8;

// One slot per row: outcome code (low nibble) and flags (`native_seams::report`).
static OUTCOMES: [AtomicU8; native_seams::COUNT] = [const { AtomicU8::new(UNCHECKED) }; native_seams::COUNT];
static IMAGE_OUTCOMES: [AtomicU8; native_seams::IMAGE_COUNT] =
    [const { AtomicU8::new(UNCHECKED) }; native_seams::IMAGE_COUNT];
/// Labels asked for that have no row: pointer and length of `&'static str`s.
static MISSING: [(AtomicUsize, AtomicUsize); MAX_MISSING] =
    [const { (AtomicUsize::new(0), AtomicUsize::new(0)) }; MAX_MISSING];
static MISSING_COUNT: AtomicUsize = AtomicUsize::new(0);
/// The install line has started: from here on a new or changed outcome is reported late.
static BOOTED: AtomicBool = AtomicBool::new(false);
/// A slot became pending since the late reporter's last pass.
static LATE_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Our own replace hooks that a `Sharing::OwnHook` row names, as installed: (hook row index + 1,
/// replacement address passed to `A64HookFunction`). One slot per such row (`OWN_HOOK_ROWS`).
static OWN_HOOKS: [(AtomicUsize, AtomicUsize); native_seams::OWN_HOOK_ROWS] =
    [const { (AtomicUsize::new(0), AtomicUsize::new(0)) }; native_seams::OWN_HOOK_ROWS];

/// Our own writes in code (`model::own_patches`): one slot per site a hook row writes, in the native table
/// (`native_seams::PATCH_SITES`) or in TrainingRooms' own site tables (`capabilities::all_sites`), one per offset. Each keeps the
/// 20 bytes there before the boot wrote anything (`prime`) and the bytes our writes left (`seal`), as words. Every `main` write of this
/// plugin happens during the boot (the only later hook goes into the `common` NRO when it loads); a hook added after `seal` would
/// need its own record.
const SPAN_WORDS: usize = SPAN / 4;
/// Every native hook row in code plus every such row of TrainingRooms' own tables (some share an offset).
const OWN_CAPACITY: usize = native_seams::PATCH_ROWS + crate::model::capabilities::WRITING_SITES;
static OWN_OFFSET: [AtomicUsize; OWN_CAPACITY] = [const { AtomicUsize::new(0) }; OWN_CAPACITY];
static OWN_BEFORE: [[AtomicU32; SPAN_WORDS]; OWN_CAPACITY] =
    [const { [const { AtomicU32::new(0) }; SPAN_WORDS] }; OWN_CAPACITY];
static OWN_AFTER: [[AtomicU32; SPAN_WORDS]; OWN_CAPACITY] =
    [const { [const { AtomicU32::new(0) }; SPAN_WORDS] }; OWN_CAPACITY];
/// Per slot: `OWN_BOOTING` (`before` kept; any change since is ours) or `OWN_RECORDED` (`after` kept too).
static OWN_STATE: [AtomicU8; OWN_CAPACITY] = [const { AtomicU8::new(OWN_NONE) }; OWN_CAPACITY];
/// Slots in use (set once by `prime`; 0 before it: checks then compare the bytes as they are, as before).
static OWN_SLOTS: AtomicUsize = AtomicUsize::new(0);
const OWN_NONE: u8 = 0;
const OWN_BOOTING: u8 = 1;
const OWN_RECORDED: u8 = 2;

/// The 20 bytes at a site, when they lie inside `main`'s `.text`.
unsafe fn read_span(offset: usize) -> Option<[u8; SPAN]> {
    let text = getRegionAddress(Region::Text) as usize;
    let rodata = getRegionAddress(Region::Rodata) as usize;
    let start = text.checked_add(offset)?;
    if !offset.is_multiple_of(4) || start.checked_add(SPAN)? > rodata {
        return None;
    }
    let mut bytes = [0u8; SPAN];
    bytes.copy_from_slice(core::slice::from_raw_parts(start as *const u8, SPAN));
    Some(bytes)
}

fn store_words(slot: &[AtomicU32; SPAN_WORDS], bytes: &[u8; SPAN]) {
    for (word, chunk) in slot.iter().zip(bytes.chunks_exact(4)) {
        word.store(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]), Ordering::Relaxed);
    }
}

fn load_words(slot: &[AtomicU32; SPAN_WORDS]) -> [u8; SPAN] {
    let mut bytes = [0u8; SPAN];
    for (word, chunk) in slot.iter().zip(bytes.chunks_exact_mut(4)) {
        chunk.copy_from_slice(&word.load(Ordering::Relaxed).to_le_bytes());
    }
    bytes
}

/// The offset of every site a hook row of any table writes in code.
fn own_sites() -> impl Iterator<Item = usize> {
    native_seams::PATCH_SITES.iter().map(|&index| native_seams::SITES[index].offset).chain(
        crate::model::capabilities::all_sites().filter(|site| crate::model::capabilities::writes_code(site)).map(|site| site.offset),
    )
}

/// Boot, before the first hook or patch of this plugin (after the hook-sharing drill's stand-ins, which play another
/// plugin): keep the bytes at every site a hook row writes. From here until `seal`, a change at one of them is ours (only
/// this plugin's boot runs), so a check puts the kept bytes back first.
pub(crate) fn prime() {
    let mut slots = 0;
    for offset in own_sites() {
        let known = (0..slots).any(|slot| OWN_OFFSET[slot].load(Ordering::Relaxed) == offset);
        if known || slots == OWN_CAPACITY {
            continue;
        }
        // SAFETY: bounds-checked against main's .text inside read_span.
        if let Some(bytes) = unsafe { read_span(offset) } {
            OWN_OFFSET[slots].store(offset, Ordering::Relaxed);
            store_words(&OWN_BEFORE[slots], &bytes);
            OWN_STATE[slots].store(OWN_BOOTING, Ordering::Relaxed);
            slots += 1;
        }
    }
    OWN_SLOTS.store(slots, Ordering::Release);
}

/// Record the bytes our writes left at one slot; whether they differ from the bytes before the boot.
fn record_after(slot: usize) -> bool {
    // SAFETY: bounds-checked against main's .text inside read_span.
    let Some(bytes) = (unsafe { read_span(OWN_OFFSET[slot].load(Ordering::Relaxed)) }) else { return false };
    store_words(&OWN_AFTER[slot], &bytes);
    OWN_STATE[slot].store(OWN_RECORDED, Ordering::Release);
    bytes != load_words(&OWN_BEFORE[slot])
}

/// Every boot hook and patch is in place (just before the install line, and before the drill's "after" stand-ins): record
/// what our writes left, so from now on only those exact bytes are put back. Another plugin hooking over one of them
/// later changes them, and it reads as it did before this change. Writes one `[TrainingSeams] own_writes=<sites written>
/// sites=<sites recorded>` line.
pub(crate) fn seal() {
    let slots = OWN_SLOTS.load(Ordering::Acquire);
    let written = (0..slots).filter(|&slot| record_after(slot)).count();
    logging::diagnostic(format_args!("[TrainingSeams] own_writes={} sites={}", written, slots));
}

/// Our recorded writes that can show inside the 64 bytes at `offset` (no heap: a lazy walk of the fixed slots).
fn own_writes(offset: usize) -> impl Iterator<Item = OwnPatch> {
    (0..OWN_SLOTS.load(Ordering::Acquire)).filter_map(move |slot| {
        let patch = OwnPatch {
            offset: OWN_OFFSET[slot].load(Ordering::Relaxed),
            before: load_words(&OWN_BEFORE[slot]),
            after: (OWN_STATE[slot].load(Ordering::Acquire) == OWN_RECORDED).then(|| load_words(&OWN_AFTER[slot])),
        };
        patch.touches(offset).then_some(patch)
    })
}

fn outcome_code(outcome: GuardOutcome) -> u8 {
    match outcome {
        GuardOutcome::Stock => STOCK,
        GuardOutcome::HookedByOther { usable: true, .. } => OTHER,
        GuardOutcome::HookedByOther { usable: false, .. } => OCCUPIED,
        GuardOutcome::Shared => SHARED,
        GuardOutcome::Foreign { .. } => FOREIGN,
    }
}

/// Apply `next` (`report::recorded` or `report::flagged`) to one slot and wake the late reporter when
/// the row became pending. Returns the previous bits. Atomics only: safe in any hook.
fn update(slot: &AtomicU8, next: impl Fn(u8, bool) -> u8) -> u8 {
    let booted = BOOTED.load(Ordering::Acquire);
    let (Ok(old) | Err(old)) = slot.fetch_update(Ordering::Release, Ordering::Relaxed, |old| Some(next(old, booted)));
    let new = next(old, booted);
    if new != old && new & report::PENDING != 0 {
        LATE_ACTIVE.store(true, Ordering::Release);
    }
    old
}

/// Store the latest outcome code of a site, keeping its flags.
fn record(index: usize, code: u8) {
    if let Some(slot) = OUTCOMES.get(index) {
        update(slot, |old, booted| report::recorded(old, code, booted));
    }
}

/// Set a flag (`SKIPPED`, `SHADOW_DIFF`) on a site.
fn flag(index: usize, bits: u8) {
    if let Some(slot) = OUTCOMES.get(index) {
        update(slot, |old, booted| report::flagged(old, bits, booted));
    }
}

/// `record` for a non-`main` row; returns the previous bits.
fn record_image(index: usize, code: u8) -> u8 {
    IMAGE_OUTCOMES.get(index).map_or(UNCHECKED, |slot| update(slot, |old, booted| report::recorded(old, code, booted)))
}

/// `flag` for a non-`main` row.
fn flag_image(index: usize, bits: u8) {
    if let Some(slot) = IMAGE_OUTCOMES.get(index) {
        update(slot, |old, booted| report::flagged(old, bits, booted));
    }
}

fn note_missing(label: &'static str) {
    let n = MISSING_COUNT.fetch_add(1, Ordering::Relaxed);
    if let Some((ptr, len)) = MISSING.get(n) {
        ptr.store(label.as_ptr() as usize, Ordering::Relaxed);
        len.store(label.len(), Ordering::Release);
    }
}

fn authoritative(seam: &Site) -> bool {
    AUTHORITATIVE.contains(&seam.group)
}


/// The registered replacement address of the hook an `OwnHook` row names (`None`: not installed).
fn own_target(seam: &Site) -> Option<u64> {
    let (index, _) = native_seams::find_site(seam.own_hook()?)?;
    OWN_HOOKS
        .iter()
        .find(|(slot, _)| slot.load(Ordering::Acquire) == index + 1)
        .map(|(_, target)| target.load(Ordering::Acquire))
        .filter(|target| *target != 0)
        .map(|target| target as u64)
}

/// The site's 64 bytes now, when they lie inside the read-only region of main its row names
/// (`.text` for code, `.rodata` for relied-on tables).
unsafe fn read_site(seam: &Site) -> Option<[u8; GUARD_BYTES]> {
    let text = getRegionAddress(Region::Text) as usize;
    let rodata = getRegionAddress(Region::Rodata) as usize;
    let (low, high) = match seam.region()? {
        MainRegion::Text => (text, rodata),
        MainRegion::Rodata => (rodata, getRegionAddress(Region::Data) as usize),
    };
    let start = text.checked_add(seam.offset)?;
    if !seam.offset.is_multiple_of(4) || start < low || start.checked_add(GUARD_BYTES)? > high {
        return None;
    }
    let mut bytes = [0u8; GUARD_BYTES];
    bytes.copy_from_slice(core::slice::from_raw_parts(start as *const u8, GUARD_BYTES));
    Some(bytes)
}

/// Compare one site and record the outcome. `None`: the label has no row (reported as `missing=`).
/// Data rows (vtable slots) are not compared and count as usable (`data`); a site whose 64 bytes
/// are not inside its region (`.text`, or `.rodata` for relied-on tables) is `Foreign` (`out_of_region`).
pub(crate) fn check(label: &'static str) -> Option<GuardOutcome> {
    let Some((index, seam)) = native_seams::find_site(label) else {
        note_missing(label);
        return None;
    };
    if !seam.compared() {
        record(index, DATA);
        return Some(GuardOutcome::Stock);
    }
    // SAFETY: bounds-checked against the row's read-only region of main inside read_site.
    let Some(actual) = (unsafe { read_site(seam) }) else {
        unexpected(index, seam, OUT_OF_REGION);
        return Some(GuardOutcome::Foreign { first_diff: 0 });
    };
    // Our own hooks and patches read as the bytes they replaced (`model::own_patches`): only another plugin's write, or a
    // change that is no hook at all, can make a row unusable.
    let (actual, _) = own_patches::unpatch(seam.offset, &actual, own_writes(seam.offset));
    let outcome = seam.outcome_with(&actual, own_target(seam));
    unexpected(index, seam, outcome_code(outcome));
    Some(outcome)
}

/// Record a row's outcome, and name the row in the shipped log the first time it reads `occupied`, `foreign` or`n/// `out_of_region` (`report::names_row`): after `check` put our own writes back, only another plugin's write or a change that is no
/// hook can do that.
fn unexpected(index: usize, seam: &Site, code: u8) {
    let old = OUTCOMES.get(index).map_or(UNCHECKED, |slot| slot.load(Ordering::Acquire));
    record(index, code);
    if report::names_row(old, code) {
        logging::diagnostic(format_args!("{}", report::RefusedLine { label: seam.label, group: seam.group, offset: seam.offset, code }));
    }
}

/// Which part of a `Sharing::EntryOrBody` site to hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Take {
    /// The entry is ours to hook (stock, or other plugins' hooks only past the words we replace).
    Entry,
    /// Another plugin's far jump owns the entry and the body after it is stock: hook the body.
    Body,
}

/// `site` for a `Sharing::EntryOrBody` entry row: which part to hook, or `None`. Authoritative group:
/// the row's outcome decides (usable → `Entry`, `Shared` → `Body`, anything else → `None`, skipped and
/// logged). Shadow group: returns `legacy`, flagging `shadow_diff` when the outcome's answer differs.
pub(crate) fn entry_or_body(label: &'static str, legacy: Option<Take>) -> Option<Take> {
    let outcome = check(label);
    let Some((index, seam)) = native_seams::find_site(label) else {
        return legacy;
    };
    let take = match outcome {
        Some(GuardOutcome::Shared) => Some(Take::Body),
        Some(outcome) if outcome.usable() => Some(Take::Entry),
        _ => None,
    };
    if authoritative(seam) {
        if take.is_none() {
            flag(index, SKIPPED);
            logging::diagnostic(format_args!(
                "[TrainingModpack] seam={} offset={:#x} result=skipped outcome={}",
                label,
                seam.offset,
                outcome.map_or("missing", GuardOutcome::code)
            ));
        }
        return take;
    }
    if take != legacy {
        flag(index, SHADOW_DIFF);
    }
    legacy
}

/// Per-site skip-and-log. Shadow group: returns `legacy` (today's decision); authoritative group:
/// returns whether the site is usable, logging a skipped site. A label without a row returns
/// `legacy`. Never panics.
pub(crate) fn site(label: &'static str, legacy: bool) -> bool {
    let outcome = check(label);
    let Some((index, seam)) = native_seams::find_site(label) else {
        return legacy;
    };
    let usable = outcome.is_some_and(GuardOutcome::usable);
    if authoritative(seam) {
        if !usable {
            flag(index, SKIPPED);
            logging::diagnostic(format_args!(
                "[TrainingModpack] seam={} offset={:#x} result=skipped outcome={}",
                label,
                seam.offset,
                outcome.map_or("missing", GuardOutcome::code)
            ));
        }
        return usable;
    }
    if usable != legacy {
        flag(index, SHADOW_DIFF);
    }
    legacy
}

/// Module gate for all-or-nothing modules: every label is checked; shadow returns `legacy`,
/// authoritative returns whether all of them are usable.
pub(crate) fn gate(group: &str, labels: &[&'static str], legacy: bool) -> bool {
    let mut usable = true;
    for &label in labels {
        usable &= check(label).is_some_and(GuardOutcome::usable);
    }
    if AUTHORITATIVE.contains(&group) {
        if !usable {
            for label in labels {
                if let Some((index, _)) = native_seams::find_site(label) {
                    flag(index, SKIPPED);
                }
            }
            logging::diagnostic(format_args!(
                "[TrainingModpack] group={} result=skipped sites={}",
                group,
                labels.len()
            ));
        }
        return usable;
    }
    if usable != legacy {
        for label in labels {
            if let Some((index, _)) = native_seams::find_site(label) {
                flag(index, SHADOW_DIFF);
            }
        }
    }
    legacy
}

/// The address of a non-`main` row: `text_base` + offset (the caller passes the image's `.text`
/// start, e.g. from the NRO load callback), or the resolved symbol.
fn image_address(seam: &ImageSite, text_base: Option<usize>) -> Option<usize> {
    match seam.locator {
        Locator::Offset(offset) => text_base?.checked_add(offset),
        Locator::Symbol(name) => {
            let mut c_name = [0u8; 256];
            if name.len() >= c_name.len() {
                return None;
            }
            c_name[..name.len()].copy_from_slice(name.as_bytes());
            let mut address = 0usize;
            // SAFETY: NUL-terminated name on the stack; `LookupSymbol` writes one usize.
            unsafe { skyline::nn::ro::LookupSymbol(&mut address, c_name.as_ptr()) };
            (address != 0).then_some(address)
        }
    }
}

/// Compare one non-`main` row (WORKAROUND-DECISIONS rows 27/36) and record the outcome. `None`: no
/// such row, or its address could not be resolved. An uncaptured row (`guard: None`) logs its
/// 64 bytes once (`[TrainingModpack] seam=… image=… capture=…`) and is never `stock`.
pub(crate) fn check_image(label: &'static str, text_base: Option<usize>) -> Option<GuardOutcome> {
    let Some((index, seam)) = native_seams::find_image(label) else {
        note_missing(label);
        return None;
    };
    let address = image_address(seam, text_base)?;
    if !address.is_multiple_of(4) {
        return None;
    }
    let mut actual = [0u8; GUARD_BYTES];
    // SAFETY: the address is the start of a resolved function or a caller-given loaded image's text.
    actual.copy_from_slice(unsafe { core::slice::from_raw_parts(address as *const u8, GUARD_BYTES) });
    let Some(expected) = seam.guard else {
        if record_image(index, UNCAPTURED) & CODE_MASK != UNCAPTURED {
            logging::diagnostic(format_args!(
                "[TrainingModpack] seam={} image={} capture={}",
                label,
                seam.image.name(),
                model::Hex(&actual)
            ));
        }
        return Some(GuardOutcome::Foreign { first_diff: 0 });
    };
    let outcome = native_seams::outcome_for(&expected, &actual, seam.kind, seam.sharing());
    record_image(index, outcome_code(outcome));
    Some(outcome)
}

/// Per-site skip-and-log for a non-`main` row: the same shadow/authoritative rule (and flags) as `site`.
pub(crate) fn site_image(label: &'static str, text_base: Option<usize>, legacy: bool) -> bool {
    let outcome = check_image(label, text_base);
    let Some((index, seam)) = native_seams::find_image(label) else {
        return legacy;
    };
    let usable = outcome.is_some_and(GuardOutcome::usable);
    if AUTHORITATIVE_IMAGES.contains(&seam.group) {
        if !usable {
            flag_image(index, SKIPPED);
            logging::diagnostic(format_args!(
                "[TrainingModpack] seam={} image={} result=skipped outcome={}",
                label,
                seam.image.name(),
                outcome.map_or("unresolved", GuardOutcome::code)
            ));
        }
        return usable;
    }
    if usable != legacy {
        flag_image(index, SHADOW_DIFF);
    }
    legacy
}

/// Rows whose recorded bits satisfy `keep`, as `[label,…]`, each `label:code` where `with_code` holds.
struct Labels {
    keep: fn(u8) -> bool,
    with_code: fn(u8) -> bool,
}

impl fmt::Display for Labels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        let mut first = true;
        for (index, seam) in native_seams::SITES.iter().enumerate() {
            let bits = OUTCOMES[index].load(Ordering::Relaxed);
            if !(self.keep)(bits) {
                continue;
            }
            if !first {
                f.write_str(",")?;
            }
            first = false;
            f.write_str(seam.label)?;
            if (self.with_code)(bits) {
                write!(f, ":{}", code_name(bits))?;
            }
        }
        f.write_str("]")
    }
}

struct Missing;

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        let n = MISSING_COUNT.load(Ordering::Relaxed).min(MAX_MISSING);
        let mut first = true;
        for (ptr, len) in MISSING.iter().take(n) {
            let (ptr, len) = (ptr.load(Ordering::Relaxed), len.load(Ordering::Acquire));
            if ptr == 0 {
                continue;
            }
            // SAFETY: stored from a `&'static str` in `note_missing`.
            let label = unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr as *const u8, len)) };
            if !first {
                f.write_str(",")?;
            }
            first = false;
            f.write_str(label)?;
        }
        if MISSING_COUNT.load(Ordering::Relaxed) > MAX_MISSING {
            f.write_str(",...")?;
        }
        f.write_str("]")
    }
}

/// The diagnostic build's hook-sharing drill fields (` foreign=<mode> <site>=<entry>/<outcome> …`,
/// `super::seam_drill`); nothing in other builds.
#[cfg(feature = "seam-drill")]
use super::seam_drill::InstallFields as DrillFields;
#[cfg(not(feature = "seam-drill"))]
struct DrillFields;

#[cfg(not(feature = "seam-drill"))]
impl fmt::Display for DrillFields {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

/// The current bits of a `main` row (the diagnostic drill reads its sites' outcomes).
#[cfg(feature = "seam-drill")]
pub(crate) fn bits_of(label: &str) -> Option<u8> {
    native_seams::find_site(label).and_then(|(index, _)| OUTCOMES.get(index)).map(|slot| slot.load(Ordering::Relaxed))
}

fn count(code: u8) -> usize {
    native_seams::SITES
        .iter()
        .enumerate()
        .filter(|(index, _)| OUTCOMES[*index].load(Ordering::Relaxed) & CODE_MASK == code)
        .count()
}

/// The seam fields of the plugin's one install line (recorded clean-room change: the fork wrote a second line of its own here,
/// `[TrainingModpack] result=installed ...`; package 1's `InstallLine` takes these as its `seams` group). From the moment the
/// fields are formatted a new or changed outcome is reported late, so a row another thread checks while the line is written
/// appears in both rather than in neither. The line carries counts and the rows that fail; the authoritative group lists and the
/// unchecked image labels (static configuration, and image rows that load later) stay out of it: the late report names them (the
/// first Eden boot showed the full lists cut the one line before `result=`).
pub(crate) struct InstallFields;

impl fmt::Display for InstallFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        BOOTED.store(true, Ordering::Release);
        let rows = native_seams::COUNT;
        let checked = rows - count(UNCHECKED);
        write!(
            f,
            "seams={} seams_stock={} seams_other={} seams_skipped={} seams_shadow_diff={} seams_missing={} seams_external={} seams_unchecked={} seams_images_unchecked={}{}",
            checked,
            count(STOCK),
            Labels { keep: listed_as_other, with_code: other_with_code },
            Labels { keep: |b| b & SKIPPED != 0, with_code: |_| true },
            Labels { keep: |b| b & SHADOW_DIFF != 0, with_code: |_| true },
            Missing,
            Labels { keep: |b| b & CODE_MASK == EXTERNAL, with_code: |_| false },
            count(UNCHECKED),
            IMAGE_OUTCOMES.iter().filter(|slot| slot.load(Ordering::Relaxed) & CODE_MASK == UNCHECKED).count(),
            DrillFields,
        )
    }
}

/// The late report (R-A0b), on the `practice-heartbeat` thread each drain interval (never in a hook):
/// one `[TrainingSeams] late group=…` line per group and one `late image=…` line per image whose rows
/// were checked or changed after the install line, once they settle (`native_seams::report::pass`).
/// Returns at once while nothing is pending. Snapshots live on this thread's stack.
pub(crate) fn report_late() {
    // The drill's stand-in run counts (rate-limited to once a second by the drill itself).
    #[cfg(feature = "seam-drill")]
    super::seam_drill::report_counts(crate::runtime::clock::uptime_ms());
    if !BOOTED.load(Ordering::Acquire) || !LATE_ACTIVE.swap(false, Ordering::AcqRel) {
        return;
    }
    let mut emit = |line: &report::LateLine<'_>| logging::diagnostic(format_args!("{}", line));
    let mut bits = [0u8; native_seams::COUNT];
    let mut again =
        report::pass("group", &OUTCOMES, &mut bits, |i| native_seams::SITES[i].label, |i| native_seams::SITES[i].group, &mut emit);
    let mut image_bits = [0u8; native_seams::IMAGE_COUNT];
    again |= report::pass(
        "image",
        &IMAGE_OUTCOMES,
        &mut image_bits,
        |i| native_seams::IMAGE_SITES[i].label,
        |i| native_seams::IMAGE_SITES[i].image.name(),
        &mut emit,
    );
    if again {
        LATE_ACTIVE.store(true, Ordering::Release);
    }
}
