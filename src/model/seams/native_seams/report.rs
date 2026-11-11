//! The installer's outcome slots and its late report (R-A0b). The Switch installer
//! (`src/swap_native/seam_install.rs`) keeps one `u8` per row in two fixed tables of atomics
//! (`SITES`, `IMAGE_SITES`): the low nibble is the row's latest outcome code, the high bits are flags.
//!
//! The install line at the end of `main` reports the boot checks. A row checked for the first time
//! after that line (a row only a runtime path checks, or an image row whose NRO loads later), or one
//! whose code or flags change after it, is marked `PENDING`. The `practice-heartbeat` thread (never
//! a hook) runs `pass` every drain interval and writes one line per group and one per image:
//!
//! `[TrainingSeams] late group=<group> sites=<n> stock=<n> other=[…] skipped=[…] shadow_diff=[…] unusable=[…] unchecked=<n>`
//!
//! (`image=<name>` for image rows). A key's line waits until every pending row of the key has been
//! pending for a whole interval (`SEEN`), so the rows one caller checks in a burst share one line;
//! the snapshot is cleared with a compare-exchange, so a row that changes meanwhile is reported
//! again. Hooks only flip bits here: no allocation, no lock. The line carries labels and codes only
//! (U17: no row text reaches the NRO).

use core::fmt;
use core::sync::atomic::{AtomicU8, Ordering};

// Outcome codes (low nibble).
pub const UNCHECKED: u8 = 0;
pub const STOCK: u8 = 1;
pub const OTHER: u8 = 2;
pub const OCCUPIED: u8 = 3;
pub const FOREIGN: u8 = 4;
pub const OUT_OF_REGION: u8 = 5;
pub const DATA: u8 = 6;
/// Listed, not converted. No row has it since the CPU-as-P1 merge guarded `cpu_pilot` (the last
/// external row); the install line keeps its `external=` field (now `[]`) for the Eden parsers.
pub const EXTERNAL: u8 = 7;
/// A non-`main` row whose 64 bytes are not captured yet (`ImageSeam::guard` is `None`).
pub const UNCAPTURED: u8 = 8;
/// A `Sharing::EntryOrBody` entry row taken through its body (`GuardOutcome::Shared`): usable. Listed
/// with the `other=` rows as `<label>:shared`.
pub const SHARED: u8 = 9;
pub const CODE_MASK: u8 = 0x0f;
// Flags (high bits).
/// Pending for at least one whole reporter interval.
pub const SEEN: u8 = 0x10;
/// Checked or changed after the install line, not reported yet.
pub const PENDING: u8 = 0x20;
pub const SKIPPED: u8 = 0x40;
pub const SHADOW_DIFF: u8 = 0x80;

/// The install line's word for a slot's code.
pub const fn code_name(bits: u8) -> &'static str {
    match bits & CODE_MASK {
        STOCK => "stock",
        OTHER => "other",
        OCCUPIED => "occupied",
        FOREIGN => "foreign",
        OUT_OF_REGION => "out_of_region",
        DATA => "data",
        EXTERNAL => "external",
        UNCAPTURED => "uncaptured",
        SHARED => "shared",
        _ => "unchecked",
    }
}

/// The `other=` list's rows: another plugin's hook we accept (`other`), or an entry taken through its
/// body (`shared`, listed with its code).
pub const fn listed_as_other(bits: u8) -> bool {
    matches!(bits & CODE_MASK, OTHER | SHARED)
}

/// Whether the `other=` list writes this row's code (`<label>:shared`).
pub const fn other_with_code(bits: u8) -> bool {
    bits & CODE_MASK == SHARED
}

/// Codes a guard refuses (an authoritative group skips the site).
const fn unusable(bits: u8) -> bool {
    matches!(bits & CODE_MASK, OCCUPIED | FOREIGN | OUT_OF_REGION | UNCAPTURED)
}

/// Whether a check that records `code` over the row's `old` bits names the row in the shipped log: a refused code
/// (`occupied`, `foreign`, `out_of_region`) the row did not have yet. The installer puts this plugin's own writes back
/// before it compares (`model::own_patches`), so such a code means another plugin's write or a change that is no hook.
pub const fn names_row(old: u8, code: u8) -> bool {
    old & CODE_MASK != code & CODE_MASK && matches!(code & CODE_MASK, OCCUPIED | FOREIGN | OUT_OF_REGION)
}

/// The shipped line that names a refused row (`names_row`).
pub struct RefusedLine<'a> {
    pub label: &'a str,
    pub group: &'a str,
    pub offset: usize,
    pub code: u8,
}

impl fmt::Display for RefusedLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[TrainingSeams] seam={} group={} offset={:#x} outcome={}", self.label, self.group, self.offset, code_name(self.code))
    }
}

/// Bits after a check records `code`, flags kept. From the install line on (`booted`), a changed code
/// (a first check included) makes the row pending and restarts its interval (`SEEN` cleared).
pub const fn recorded(old: u8, code: u8, booted: bool) -> u8 {
    let new = (old & !CODE_MASK) | (code & CODE_MASK);
    if booted && old & CODE_MASK != code & CODE_MASK { (new & !SEEN) | PENDING } else { new }
}

/// Bits after `flags` (`SKIPPED`, `SHADOW_DIFF`) are set. From the install line on, a newly set flag
/// makes the row pending and restarts its interval.
pub const fn flagged(old: u8, flags: u8, booted: bool) -> u8 {
    let new = old | flags;
    if booted && old & flags != flags { (new & !SEEN) | PENDING } else { new }
}

/// A snapshot of one outcome table, with its rows' labels and report keys (group or image name).
#[derive(Clone, Copy)]
pub struct Snapshot<'a> {
    pub bits: &'a [u8],
    pub label: fn(usize) -> &'static str,
    pub key: fn(usize) -> &'static str,
}

/// What the reporter does at one row this pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Not pending, or not the first pending row of its key.
    Skip,
    /// The key has a row that became pending during the last interval: mark its rows `SEEN`.
    Wait,
    /// Every pending row of the key was already pending a whole interval ago: write the line.
    Report,
}

impl<'a> Snapshot<'a> {
    fn same_key(&self, a: usize, b: usize) -> bool {
        (self.key)(a) == (self.key)(b)
    }

    /// Every row with `first`'s key.
    fn of_key(self, first: usize) -> impl Iterator<Item = usize> + 'a {
        (0..self.bits.len()).filter(move |&i| self.same_key(first, i))
    }

    /// The pending rows of `first`'s key.
    pub fn pending(self, first: usize) -> impl Iterator<Item = usize> + 'a {
        self.of_key(first).filter(move |&i| self.bits[i] & PENDING != 0)
    }

    /// The decision at row `i`: only the first pending row of a key decides for the key.
    pub fn step(&self, i: usize) -> Step {
        if self.bits[i] & PENDING == 0
            || (0..i).any(|j| self.bits[j] & PENDING != 0 && self.same_key(i, j))
        {
            return Step::Skip;
        }
        if self.pending(i).all(|j| self.bits[j] & SEEN != 0) { Step::Report } else { Step::Wait }
    }
}

/// One late line: the pending rows of the key of row `first` (`kind` is `group` or `image`).
pub struct LateLine<'a> {
    pub kind: &'static str,
    pub first: usize,
    pub snapshot: Snapshot<'a>,
}

impl LateLine<'_> {
    /// `[label,…]` of this line's rows whose bits satisfy `keep`, `label:code` where `with_code` holds.
    fn list(&self, f: &mut fmt::Formatter<'_>, keep: fn(u8) -> bool, with_code: fn(u8) -> bool) -> fmt::Result {
        f.write_str("[")?;
        let s = self.snapshot;
        for (n, i) in s.pending(self.first).filter(|&i| keep(s.bits[i])).enumerate() {
            if n > 0 {
                f.write_str(",")?;
            }
            f.write_str((s.label)(i))?;
            if with_code(s.bits[i]) {
                write!(f, ":{}", code_name(s.bits[i]))?;
            }
        }
        f.write_str("]")
    }
}

const fn always(_: u8) -> bool {
    true
}

impl fmt::Display for LateLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.snapshot;
        let sites = s.pending(self.first).count();
        let stock = s.pending(self.first).filter(|&i| s.bits[i] & CODE_MASK == STOCK).count();
        let unchecked = s.of_key(self.first).filter(|&i| s.bits[i] & CODE_MASK == UNCHECKED).count();
        write!(f, "[TrainingSeams] late {}={} sites={} stock={} other=", self.kind, (s.key)(self.first), sites, stock)?;
        self.list(f, listed_as_other, other_with_code)?;
        f.write_str(" skipped=")?;
        self.list(f, |b| b & SKIPPED != 0, always)?;
        f.write_str(" shadow_diff=")?;
        self.list(f, |b| b & SHADOW_DIFF != 0, always)?;
        f.write_str(" unusable=")?;
        self.list(f, unusable, always)?;
        write!(f, " unchecked={}", unchecked)
    }
}

/// One reporter pass over one table (`kind` `group` or `image`): snapshots `slots` into `bits`, marks
/// the rows of keys that are still settling `SEEN`, hands each settled key's line to `emit`, then
/// clears `PENDING`/`SEEN` on the reported rows unless they changed meanwhile. Returns whether
/// another pass is needed (a key waits, or a reported row changed).
pub fn pass(
    kind: &'static str,
    slots: &[AtomicU8],
    bits: &mut [u8],
    label: fn(usize) -> &'static str,
    key: fn(usize) -> &'static str,
    emit: &mut dyn FnMut(&LateLine<'_>),
) -> bool {
    let rows = bits.len().min(slots.len());
    for (bit, slot) in bits.iter_mut().zip(slots) {
        *bit = slot.load(Ordering::Acquire);
    }
    let bits: &[u8] = &bits[..rows];
    let snapshot = Snapshot { bits, label, key };
    let mut again = false;
    for first in 0..rows {
        match snapshot.step(first) {
            Step::Skip => {}
            Step::Wait => {
                // `SEEN` only on a row unchanged since the snapshot: a row that changes meanwhile waits
                // its own whole interval.
                for i in snapshot.pending(first) {
                    let _ = slots[i].compare_exchange(bits[i], bits[i] | SEEN, Ordering::AcqRel, Ordering::Relaxed);
                }
                again = true;
            }
            Step::Report => {
                emit(&LateLine { kind, first, snapshot });
                for i in snapshot.pending(first) {
                    let reported = bits[i];
                    again |= slots[i]
                        .compare_exchange(reported, reported & !(PENDING | SEEN), Ordering::AcqRel, Ordering::Relaxed)
                        .is_err();
                }
            }
        }
    }
    again
}

