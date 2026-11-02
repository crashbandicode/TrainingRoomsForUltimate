//! The playbook's hook-sharing drill (HOOK-SHARING "Testing the chain in Eden", CHECKLIST hook sharing):
//! diagnostic build only. `foreign=inline|far[,after]` in `sd:/ultimate/TrainingRooms/diag.txt` makes
//! this plugin hook one site per sharing rule (`native_seams::drill::SITES`) the way a plugin that does
//! not share would: `inline` is Skyline's inline hook (a one-word branch) to a callback that counts and
//! returns, `far` a replace hook (a far jump) to a handler that counts and continues to Skyline's
//! trampoline (the replaced instructions, then back to the site). By default the stand-ins go in
//! before any hook of ours (`before`, at the start of `main`); with `,after` they go in after ours,
//! just before the install line (`after`), so both load orders are covered.
//!
//! The drill never stacks onto a hook Skyline cannot relocate: before ours it hooks only a site that
//! still holds its 13.0.5 bytes (another real plugin's hook there is left alone), and after ours only a
//! site holding the 13.0.5 bytes or a one-word branch. Over our far jump a stand-in is refused: a far
//! stand-in would relocate the jump's address word as code (HOOK-SHARING "The problem"), and an inline
//! one would relocate its `ldr x17` while Skyline's own jump back through x17 overwrites the loaded
//! target. A plugin that did either would break the site for both; the drill records the refusal
//! (logged, and listed in `standin_refused=`) instead.
//!
//! The install line gains ` foreign=<mode>` and, per row of each site, ` <row>=<entry>/<outcome>`: the
//! form the row's check met (or, after ours, the form the stand-in met) and the row's recorded outcome
//! (`:skipped` when an authoritative group skipped it). Run counts are logged by the heartbeat as
//! `[TrainingSeams] diag standin <site>_<form>=<n> …` when they change (at most once a second).

use core::fmt;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

use skyline::hooks::{getRegionAddress, A64HookFunction, A64InlineHook, InlineCtx, Region};
use skyline::libc::c_void;
use crate::engine::seams::native_seams::drill::{Drill, Standin, SITES};
use crate::engine::seams::native_seams::report::{code_name, SKIPPED};
use crate::engine::seams::native_seams::{self, entry_form, EntryForm, GUARD_BYTES};

const DIAG_FILE: &str = "sd:/ultimate/TrainingRooms/diag.txt";
const SITE_COUNT: usize = SITES.len();
/// Rows per site (a site and its pair).
const ROWS: usize = 2;

/// 0 none, 1 inline, 2 far; +4 after ours.
static MODE: AtomicU8 = AtomicU8::new(0);
/// Per site: 0 not attempted, 1 installed, 2 refused, 3 no row.
static STATE: [AtomicU8; SITE_COUNT] = [const { AtomicU8::new(0) }; SITE_COUNT];
/// Per row of each site: the entry form (`EntryForm` as u8 + 1; 0 = not read).
static ENTRY: [[AtomicU8; ROWS]; SITE_COUNT] = [const { [const { AtomicU8::new(0) }; ROWS] }; SITE_COUNT];
static INLINE_HITS: [AtomicU64; SITE_COUNT] = [const { AtomicU64::new(0) }; SITE_COUNT];
/// Counted by the far handlers below (plain loads and stores, as the playbook's stand-in does).
static FAR_HITS: [AtomicU64; SITE_COUNT] = [const { AtomicU64::new(0) }; SITE_COUNT];
/// Skyline's trampoline per far stand-in (the replaced instructions, then back to the site).
static ORIGINAL: [AtomicU64; SITE_COUNT] = [const { AtomicU64::new(0) }; SITE_COUNT];
static LOGGED: [AtomicU64; SITE_COUNT] = [const { AtomicU64::new(0) }; SITE_COUNT];
static LAST_LOG_MS: AtomicU64 = AtomicU64::new(0);

fn drill() -> Drill {
    let bits = MODE.load(Ordering::Acquire);
    let standin = match bits & 3 {
        1 => Standin::Inline,
        2 => Standin::Far,
        _ => Standin::None,
    };
    Drill { standin, after: bits & 4 != 0 }
}

unsafe extern "C" fn inline_0(_: &InlineCtx) {
    INLINE_HITS[0].fetch_add(1, Ordering::Relaxed);
}
unsafe extern "C" fn inline_1(_: &InlineCtx) {
    INLINE_HITS[1].fetch_add(1, Ordering::Relaxed);
}
unsafe extern "C" fn inline_2(_: &InlineCtx) {
    INLINE_HITS[2].fetch_add(1, Ordering::Relaxed);
}
unsafe extern "C" fn inline_3(_: &InlineCtx) {
    INLINE_HITS[3].fetch_add(1, Ordering::Relaxed);
}

const INLINE: [unsafe extern "C" fn(&InlineCtx); SITE_COUNT] = [inline_0, inline_1, inline_2, inline_3];

extern "C" {
    /// Count a run in `FAR_HITS[i]`, then jump to `ORIGINAL[i]`, keeping every register but x17 (as
    /// Skyline's far jump does).
    fn tm_seam_drill_far_0();
    fn tm_seam_drill_far_1();
    fn tm_seam_drill_far_2();
    fn tm_seam_drill_far_3();
}

macro_rules! far_handler {
    ($name:literal, $slot:literal) => {
        concat!(
            ".balign 16\n",
            ".hidden ", $name, "\n",
            ".global ", $name, "\n",
            $name, ":\n",
            "stp x16, x17, [sp, #-16]!\n",
            "adrp x16, {hits}\n",
            "add x16, x16, #:lo12:{hits}\n",
            "ldr x17, [x16, #", $slot, "]\n",
            "add x17, x17, #1\n",
            "str x17, [x16, #", $slot, "]\n",
            "ldp x16, x17, [sp], #16\n",
            "adrp x17, {original}\n",
            "add x17, x17, #:lo12:{original}\n",
            "ldr x17, [x17, #", $slot, "]\n",
            "br x17\n",
        )
    };
}

core::arch::global_asm!(
    ".pushsection .text.tm_seam_drill_far, \"ax\", %progbits",
    far_handler!("tm_seam_drill_far_0", "0"),
    far_handler!("tm_seam_drill_far_1", "8"),
    far_handler!("tm_seam_drill_far_2", "16"),
    far_handler!("tm_seam_drill_far_3", "24"),
    ".popsection",
    hits = sym FAR_HITS,
    original = sym ORIGINAL,
);

fn far_handler(index: usize) -> *const c_void {
    let handler: unsafe extern "C" fn() = match index {
        0 => tm_seam_drill_far_0,
        1 => tm_seam_drill_far_1,
        2 => tm_seam_drill_far_2,
        _ => tm_seam_drill_far_3,
    };
    handler as *const c_void
}

/// The 64 bytes at a row's address now.
unsafe fn bytes_at(address: usize) -> [u8; GUARD_BYTES] {
    let mut out = [0u8; GUARD_BYTES];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = ((address + i) as *const u8).read_volatile();
    }
    out
}

fn form_code(form: EntryForm) -> u8 {
    match form {
        EntryForm::Stock => 1,
        EntryForm::Branch => 2,
        EntryForm::FarJump => 3,
        EntryForm::Changed => 4,
    }
}

fn form_name(code: u8) -> &'static str {
    match code {
        1 => EntryForm::Stock.name(),
        2 => EntryForm::Branch.name(),
        3 => EntryForm::FarJump.name(),
        4 => EntryForm::Changed.name(),
        _ => "unread",
    }
}

/// Record the entry form of each row of site `index` as it is now.
unsafe fn record_entries(base: usize, index: usize) {
    for (slot, label) in ENTRY[index].iter().zip(SITES[index].1) {
        if let Some((_, row)) = native_seams::find_site(label) {
            slot.store(form_code(entry_form(&row.guard, &bytes_at(base + row.offset))), Ordering::Release);
        }
    }
}

unsafe fn install(drill: Drill) {
    let base = getRegionAddress(Region::Text) as usize;
    for (index, (name, rows)) in SITES.iter().enumerate() {
        let Some((_, row)) = native_seams::find_site(rows[0]) else {
            STATE[index].store(3, Ordering::Release);
            continue;
        };
        let address = base + row.offset;
        let form = entry_form(&row.guard, &bytes_at(address));
        let allowed = if drill.after {
            // After ours: the 13.0.5 bytes or our one-word branch (Skyline relocates a branch); never
            // our far jump (see the module doc).
            matches!(form, EntryForm::Stock | EntryForm::Branch)
        } else {
            // Before ours: only a site no other plugin has hooked.
            form == EntryForm::Stock
        };
        if drill.after {
            record_entries(base, index);
        }
        if !allowed {
            STATE[index].store(2, Ordering::Release);
            if !drill.after {
                record_entries(base, index);
            }
            crate::engine::compat::logging::diagnostic(format_args!(
                "[TrainingSeams] diag standin {} refused mode={} entry={}",
                name,
                drill.name(),
                form.name()
            ));
            continue;
        }
        match drill.standin {
            Standin::Inline => A64InlineHook(address as *const c_void, INLINE[index] as *const c_void),
            Standin::Far => A64HookFunction(
                address as *const c_void,
                far_handler(index),
                ORIGINAL[index].as_ptr() as *mut *mut c_void,
            ),
            Standin::None => continue,
        }
        STATE[index].store(1, Ordering::Release);
        if !drill.after {
            // What this plugin's own check of each row will meet.
            record_entries(base, index);
        }
    }
}

/// Start of `main`: read the drill setting and, for the default order, hook the sites before ours.
pub(crate) fn before() {
    let drill = std::fs::read(DIAG_FILE).map(|text| Drill::parse(&text)).unwrap_or(Drill::OFF);
    let bits = match drill.standin {
        Standin::None => 0,
        Standin::Inline => 1,
        Standin::Far => 2,
    } | if drill.after { 4 } else { 0 };
    MODE.store(bits, Ordering::Release);
    if drill.standin != Standin::None && !drill.after {
        unsafe { install(drill) };
    }
}

/// Just before the install line: the `,after` order hooks the sites after ours.
pub(crate) fn after() {
    let drill = drill();
    if drill.standin != Standin::None && drill.after {
        unsafe { install(drill) };
    }
}

/// ` foreign=<mode>` and, with a drill, ` <row>=<entry>/<outcome>` per row and the refused sites.
pub(crate) struct InstallFields;

impl fmt::Display for InstallFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let drill = drill();
        write!(f, " foreign={}", drill.name())?;
        if drill.standin == Standin::None {
            return Ok(());
        }
        for (index, (_, rows)) in SITES.iter().enumerate() {
            for (slot, label) in ENTRY[index].iter().zip(rows.iter()) {
                let bits = super::seam_install::bits_of(label).unwrap_or(0);
                write!(f, " {}={}/{}", label, form_name(slot.load(Ordering::Acquire)), code_name(bits))?;
                if bits & SKIPPED != 0 {
                    f.write_str(":skipped")?;
                }
            }
        }
        f.write_str(" standin_refused=[")?;
        let mut first = true;
        for (index, (name, _)) in SITES.iter().enumerate() {
            if STATE[index].load(Ordering::Acquire) >= 2 {
                if !first {
                    f.write_str(",")?;
                }
                first = false;
                f.write_str(name)?;
            }
        }
        f.write_str("]")
    }
}

/// Heartbeat: log the stand-ins' run counts when they changed, at most once a second.
pub(crate) fn report_counts(now_ms: u64) {
    let drill = drill();
    if drill.standin == Standin::None || now_ms.saturating_sub(LAST_LOG_MS.load(Ordering::Relaxed)) < 1000 {
        return;
    }
    let hits = |index: usize| match drill.standin {
        Standin::Far => FAR_HITS[index].load(Ordering::Relaxed),
        _ => INLINE_HITS[index].load(Ordering::Relaxed),
    };
    let changed = (0..SITE_COUNT).any(|index| hits(index) != LOGGED[index].load(Ordering::Relaxed));
    if !changed {
        return;
    }
    LAST_LOG_MS.store(now_ms, Ordering::Relaxed);
    struct Counts<F: Fn(usize) -> u64>(Drill, F);
    impl<F: Fn(usize) -> u64> fmt::Display for Counts<F> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for (index, (name, _)) in SITES.iter().enumerate() {
                if STATE[index].load(Ordering::Acquire) == 1 {
                    write!(f, " {}_{}={}", name, self.0.form(), (self.1)(index))?;
                }
            }
            Ok(())
        }
    }
    for (index, logged) in LOGGED.iter().enumerate() {
        logged.store(hits(index), Ordering::Relaxed);
    }
    crate::engine::compat::logging::diagnostic(format_args!("[TrainingSeams] diag standin{}", Counts(drill, hits)));
}
