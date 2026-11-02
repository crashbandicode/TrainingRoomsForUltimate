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

#[cfg(test)]
mod tests {
    use super::*;

    // A table shaped like the R-A0 finding: two runtime-only groups checked late and one image.
    const LABELS: [&str; 7] = ["g.a", "g.b", "g.c", "pop.idle", "pop.binding", "item.x", "item.y"];
    const KEYS: [&str; 7] = ["mii_gate", "mii_gate", "mii_gate", "game_mode", "game_mode", "item", "item"];

    fn label(i: usize) -> &'static str {
        LABELS[i]
    }

    fn key(i: usize) -> &'static str {
        KEYS[i]
    }

    struct Table {
        slots: [AtomicU8; 7],
        booted: bool,
    }

    impl Table {
        fn new() -> Self {
            Table { slots: [const { AtomicU8::new(UNCHECKED) }; 7], booted: false }
        }

        fn record(&self, i: usize, code: u8) {
            let old = self.slots[i].load(Ordering::Relaxed);
            self.slots[i].store(recorded(old, code, self.booted), Ordering::Relaxed);
        }

        fn flag(&self, i: usize, flags: u8) {
            let old = self.slots[i].load(Ordering::Relaxed);
            self.slots[i].store(flagged(old, flags, self.booted), Ordering::Relaxed);
        }

        /// One heartbeat tick: the lines written and whether another pass is due.
        fn tick(&self) -> (Vec<String>, bool) {
            let mut bits = [0u8; 7];
            let mut lines = Vec::new();
            let again = pass("group", &self.slots, &mut bits, label, key, &mut |line| lines.push(line.to_string()));
            (lines, again)
        }
    }

    #[test]
    fn checks_before_the_install_line_are_never_pending() {
        assert_eq!(recorded(UNCHECKED, STOCK, false), STOCK);
        assert_eq!(flagged(STOCK, SHADOW_DIFF, false), STOCK | SHADOW_DIFF);
        let t = Table::new();
        t.record(4, STOCK);
        assert_eq!(t.tick(), (vec![], false));
    }

    #[test]
    fn a_first_or_changed_check_after_boot_is_pending_and_a_repeat_is_not() {
        assert_eq!(recorded(UNCHECKED, STOCK, true), STOCK | PENDING);
        assert_eq!(recorded(STOCK, STOCK, true), STOCK, "same outcome again: nothing new");
        assert_eq!(recorded(STOCK | SHADOW_DIFF, FOREIGN, true), FOREIGN | SHADOW_DIFF | PENDING, "flags kept");
        assert_eq!(flagged(FOREIGN, SKIPPED, true), FOREIGN | SKIPPED | PENDING);
        assert_eq!(flagged(FOREIGN | SKIPPED, SKIPPED, true), FOREIGN | SKIPPED, "repeat flag: nothing new");
        assert_eq!(code_name(FOREIGN | SKIPPED | PENDING | SEEN), "foreign");
    }

    #[test]
    fn one_line_per_group_after_its_runtime_rows_resolve_in_a_burst() {
        let mut t = Table::new();
        t.record(4, STOCK); // boot: game_mode's binding row is in the install line
        t.booted = true;
        // The Mii gate checks its rows in two calls (gate, then headgear) within one interval.
        t.record(0, STOCK);
        t.record(1, OTHER);
        let (lines, again) = t.tick();
        assert!(lines.is_empty() && again, "a key waits one whole interval");
        t.record(2, FOREIGN);
        let (lines, again) = t.tick();
        assert!(lines.is_empty() && again, "a row that joined during the interval holds its key back");
        let (lines, again) = t.tick();
        assert_eq!(
            lines,
            ["[TrainingSeams] late group=mii_gate sites=3 stock=1 other=[g.b] skipped=[] shadow_diff=[] unusable=[g.c:foreign] unchecked=0"]
        );
        assert!(!again);
        assert_eq!(t.tick(), (vec![], false), "reported once");
        // The same outcomes again (a re-check per request): no new line.
        t.record(0, STOCK);
        t.record(2, FOREIGN);
        assert_eq!(t.tick(), (vec![], false));
    }

    #[test]
    fn groups_are_reported_separately_and_a_partly_boot_checked_group_counts_its_unchecked_rows() {
        let mut t = Table::new();
        t.booted = true;
        t.record(3, STOCK); // game_mode.popup_idle, first popup query
        t.record(0, STOCK);
        t.tick();
        let (lines, _) = t.tick();
        assert_eq!(
            lines,
            [
                "[TrainingSeams] late group=mii_gate sites=1 stock=1 other=[] skipped=[] shadow_diff=[] unusable=[] unchecked=2",
                "[TrainingSeams] late group=game_mode sites=1 stock=1 other=[] skipped=[] shadow_diff=[] unusable=[] unchecked=1",
            ]
        );
    }

    #[test]
    fn a_changed_outcome_or_new_flag_is_reported_again() {
        let mut t = Table::new();
        t.booted = true;
        t.record(5, STOCK);
        t.record(6, STOCK);
        t.tick();
        assert_eq!(t.tick().0.len(), 1);
        // Another plugin hooks item.y later; an authoritative group skips it.
        t.record(6, OCCUPIED);
        t.flag(6, SKIPPED);
        t.tick();
        assert_eq!(
            t.tick().0,
            ["[TrainingSeams] late group=item sites=1 stock=0 other=[] skipped=[item.y:occupied] shadow_diff=[] unusable=[item.y:occupied] unchecked=0"]
        );
    }

    #[test]
    fn a_row_changed_after_the_snapshot_stays_pending() {
        let mut t = Table::new();
        t.booted = true;
        t.record(5, STOCK);
        t.tick();
        let mut bits = [0u8; 7];
        let mut written = 0;
        let again = pass("image", &t.slots, &mut bits, label, key, &mut |line| {
            assert!(line.to_string().starts_with("[TrainingSeams] late image=item sites=1 stock=1 "));
            written += 1;
            // A concurrent re-check between the snapshot and the clear.
            t.slots[5].store(recorded(t.slots[5].load(Ordering::Relaxed), FOREIGN, true), Ordering::Relaxed);
        });
        assert!(written == 1 && again, "the changed row is not cleared");
        assert_ne!(t.slots[5].load(Ordering::Relaxed) & PENDING, 0);
        t.tick();
        assert_eq!(t.tick().0.len(), 1, "and is reported on a later pass");
    }

    /// One reporter pass over a whole table: the lines written and whether another pass is due.
    fn run(slots: &[AtomicU8], kind: &'static str, label: fn(usize) -> &'static str, key: fn(usize) -> &'static str) -> (Vec<String>, bool) {
        let mut bits = vec![0u8; slots.len()];
        let mut lines = Vec::new();
        let again = pass(kind, slots, &mut bits, label, key, &mut |line| lines.push(line.to_string()));
        (lines, again)
    }

    #[test]
    fn the_r_a0_runtime_rows_get_one_line_per_group_and_per_image() {
        use crate::engine::seams::native_seams::{Image, COUNT, IMAGE_COUNT, IMAGE_SITES, SITES};
        // R-A0 (Yoga, M4): 21 main rows are checked only at runtime, after the install line.
        let runtime = |i: usize| {
            SITES[i].group == "mii_ordinary_gate" || matches!(SITES[i].label, "sources.kirby_copy_clear" | "game_mode.popup_idle")
        };
        assert_eq!((0..COUNT).filter(|&i| runtime(i)).count(), 21, "19 Mii gate rows, the Kirby copy clear, the popup query");
        let slots: Vec<AtomicU8> =
            (0..COUNT).map(|i| AtomicU8::new(if runtime(i) { UNCHECKED } else { recorded(UNCHECKED, STOCK, false) })).collect();
        for i in (0..COUNT).filter(|&i| runtime(i)) {
            slots[i].store(recorded(slots[i].load(Ordering::Relaxed), STOCK, true), Ordering::Relaxed);
        }
        let site_label: fn(usize) -> &'static str = |i| SITES[i].label;
        let group: fn(usize) -> &'static str = |i| SITES[i].group;
        assert_eq!(run(&slots, "group", site_label, group), (vec![], true), "settling");
        let (lines, again) = run(&slots, "group", site_label, group);
        assert!(!again);
        assert_eq!(
            lines,
            [
                "[TrainingSeams] late group=character_sources sites=1 stock=1 other=[] skipped=[] shadow_diff=[] unusable=[] unchecked=0",
                "[TrainingSeams] late group=mii_ordinary_gate sites=19 stock=19 other=[] skipped=[] shadow_diff=[] unusable=[] unchecked=0",
                "[TrainingSeams] late group=game_mode sites=1 stock=1 other=[] skipped=[] shadow_diff=[] unusable=[] unchecked=0",
            ]
        );
        assert_eq!(run(&slots, "group", site_label, group), (vec![], false), "once");
        // 49 image rows read `unchecked` at boot: the item NRO loads later and checks its 12 rows.
        let images: Vec<AtomicU8> = (0..IMAGE_COUNT).map(|_| AtomicU8::new(UNCHECKED)).collect();
        for i in (0..IMAGE_COUNT).filter(|&i| IMAGE_SITES[i].image == Image::Nro("item")) {
            images[i].store(recorded(UNCHECKED, STOCK, true), Ordering::Relaxed);
        }
        let image_label: fn(usize) -> &'static str = |i| IMAGE_SITES[i].label;
        let image: fn(usize) -> &'static str = |i| IMAGE_SITES[i].image.name();
        run(&images, "image", image_label, image);
        let (image_lines, _) = run(&images, "image", image_label, image);
        assert_eq!(
            image_lines,
            ["[TrainingSeams] late image=item sites=12 stock=12 other=[] skipped=[] shadow_diff=[] unusable=[] unchecked=0"]
        );
        // Short enough to be formatted on the logger's 512-byte stack line (no ring re-format).
        assert!(lines.iter().chain(&image_lines).all(|l| l.len() < 512));
    }

    /// SEAMS-ALL: an `EntryOrBody` entry taken through its body is usable and listed with `other=` as
    /// `<label>:shared` (late line and, through the same helpers, the install line).
    #[test]
    fn a_shared_entry_is_usable_and_listed_with_its_form() {
        assert_eq!(code_name(SHARED | PENDING), "shared");
        assert!(!unusable(SHARED));
        assert!(listed_as_other(SHARED) && listed_as_other(OTHER) && !listed_as_other(STOCK));
        assert!(other_with_code(SHARED) && !other_with_code(OTHER));
        let mut t = Table::new();
        t.booted = true;
        t.record(5, SHARED);
        t.record(6, OTHER);
        t.tick();
        let (lines, _) = t.tick();
        assert_eq!(
            lines,
            ["[TrainingSeams] late group=item sites=2 stock=0 other=[item.x:shared,item.y] skipped=[] shadow_diff=[] unusable=[] unchecked=0"]
        );
    }

    #[test]
    fn the_snapshot_steps() {
        let bits = [STOCK | PENDING | SEEN, OTHER | PENDING, UNCHECKED, STOCK, STOCK, STOCK | PENDING | SEEN, UNCHECKED];
        let s = Snapshot { bits: &bits, label, key };
        assert_eq!(s.step(0), Step::Wait, "g.b joined after g.a was seen");
        assert_eq!(s.step(1), Step::Skip, "only the key's first pending row decides");
        assert_eq!(s.step(2), Step::Skip);
        assert_eq!(s.step(5), Step::Report);
        assert_eq!(s.pending(0).collect::<Vec<_>>(), [0, 1]);
    }
}
