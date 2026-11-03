//! The install line's worst-case length, as a constant computed from the tables it reports (lead ruling 2026-11-02, after the line
//! was cut twice by fixed buffers that the next package's rows outgrew: 2,048 B, then 8,192 B).
//!
//! The runtime renders the line once, on the boot thread and outside every hook scope, into a buffer of exactly
//! [`INSTALL_LINE_CAPACITY`] bytes taken from the boot heap and freed after the write (`model::install_line::render_into`: a fixed
//! sink that never grows the buffer; `heap_allocations` counts hook scopes only, so it stays 0). That satisfies the playbook's
//! "fixed buffers" and "no heap in hooks", and nothing of it stays resident after boot: a static buffer or a thread stack of this
//! size (about 110 KB, most of it the seam lists' worst case) would. A line that would not fit ends `result=line_overflow`; the
//! test below renders the worst case and fails before that can happen.
//!
//! Every bound is a sum over the rows as they are: a new site or seam row grows the constant by itself.

use crate::engine::seams::native_seams;
use crate::model::install_line::OVERFLOW_TAIL;
use crate::model::sites::{self, Site};
use crate::model::{sites_pkg3, sites_pkg4, sites_pkg6, sites_u4, sites_ui};

/// The feature reports the boot writes, at most (`runtime::boot`, its `features` array: a compile-time check there).
pub const MAX_FEATURE_REPORTS: usize = 32;
/// The longest feature name in a report.
pub const MAX_FEATURE_NAME: usize = 32;
/// The longest `reason=` value a feature report names (a provider's state, or `site:<row>`).
pub const MAX_REASON: usize = 128;
/// The longest label the seam installer lists as asked for but missing, and how many it lists (`seam_install::MAX_MISSING`).
pub const MAX_MISSING_LABEL: usize = 128;
pub const MISSING_SHOWN: usize = 8;
/// The longest outcome word of a site (`GuardOutcome::code`: `occupied`) and of a write (`Written::word`: `restored`).
const MAX_FOUND_WORD: usize = 8;
const MAX_WRITTEN_WORD: usize = 8;
/// The longest seam outcome word (`native_seams::report::code_name`: `out_of_region`).
const MAX_SEAM_CODE: usize = 13;
/// The longest stand-in form word of the hook-sharing drill (its only reader, `drill_fields`, is compiled with `seam-drill`).
#[cfg(feature = "seam-drill")]
const MAX_DRILL_FORM: usize = 16;

/// The fields every line has, at their longest: tick (20 digits), ` plugin=training_rooms/` and the version (32), ` game=` and the
/// display version (16 bytes, one character each), ` build=` and a build id of up to 64 bytes in hex, ` developer=off reason=` and
/// its reason (32), ` settings: input_delay=255 lra_reset=off stage_hazards=off`, ` migration=` and its 96-byte value, ` result=`
/// with an unsupported game's `reason=`, ` heap_allocations=` and 20 digits, the seams' leading space: 590, rounded up.
pub const FIXED_FIELDS: usize = 1024;

const fn site_fields(tables: &[&[Site]]) -> usize {
    let mut total = 0;
    let mut t = 0;
    while t < tables.len() {
        let rows = tables[t];
        let mut i = 0;
        while i < rows.len() {
            // ` <name>=<found>/<written>`
            total += 1 + rows[i].name.len() + 1 + MAX_FOUND_WORD + 1 + MAX_WRITTEN_WORD;
            i += 1;
        }
        t += 1;
    }
    total
}

const fn longest_name(tables: &[&[Site]]) -> usize {
    let mut longest = 0;
    let mut t = 0;
    while t < tables.len() {
        let rows = tables[t];
        let mut i = 0;
        while i < rows.len() {
            if rows[i].name.len() > longest {
                longest = rows[i].name.len();
            }
            i += 1;
        }
        t += 1;
    }
    longest
}

/// Every table whose rows the line reports, once each (base/packages, UI, Hitbox and Developer).
const TABLES: [&[Site]; 8] =
    [&sites::SITES, &sites_pkg3::STATE_SITES, &sites_pkg4::SITES, &sites_ui::UI_SITES, &sites_u4::SITES, &sites_pkg6::SITES, &crate::model::sites_hitbox::SITES, &crate::model::developer_debug::SITES];

/// The site fields at their longest.
pub const SITE_FIELDS: usize = site_fields(&TABLES);

const fn max(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

/// The feature reports at their longest: ` <feature>=not_installed reason=<reason or site:<row>>`.
pub const FEATURE_FIELDS: usize =
    MAX_FEATURE_REPORTS * (1 + MAX_FEATURE_NAME + "=not_installed".len() + " reason=".len() + max(MAX_REASON, "site:".len() + longest_name(&TABLES)));

/// The seam installer's fields at their longest (`seam_install::InstallFields`): every seam row in each of the three lists that
/// print a code (`seams_other`, `seams_skipped`, `seams_shadow_diff`, as `<label>:<code>,`) and in `seams_external` (`<label>,`),
/// the missing labels, and the fixed words with five-digit counts.
const fn seam_fields() -> usize {
    let rows = &native_seams::SITES;
    let mut labels = 0;
    let mut i = 0;
    while i < rows.len() {
        labels += rows[i].label.len();
        i += 1;
    }
    let with_code = labels + rows.len() * (1 + MAX_SEAM_CODE + 1);
    let without_code = labels + rows.len();
    let missing = MISSING_SHOWN * (MAX_MISSING_LABEL + 1) + ",...".len();
    let words = "seams= seams_stock= seams_other=[] seams_skipped=[] seams_shadow_diff=[] seams_missing=[] seams_external=[] seams_unchecked= seams_images_unchecked=".len()
        + 4 * 5;
    3 * with_code + without_code + missing + words
}

pub const SEAM_FIELDS: usize = seam_fields();

/// The hook-sharing drill's fields (diag feature `seam-drill`): ` foreign=<mode>`, ` <row>=<form>/<code>:skipped` per drill row
/// and ` standin_refused=[<site>,…]`.
#[cfg(feature = "seam-drill")]
const fn drill_fields() -> usize {
    let sites = &native_seams::drill::SITES;
    let mut total = " foreign=far,after".len() + " standin_refused=[]".len();
    let mut i = 0;
    while i < sites.len() {
        total += sites[i].0.len() + 1;
        let rows = sites[i].1;
        let mut j = 0;
        while j < rows.len() {
            total += 1 + rows[j].len() + 1 + MAX_DRILL_FORM + 1 + MAX_SEAM_CODE + ":skipped".len();
            j += 1;
        }
        i += 1;
    }
    total
}

#[cfg(feature = "seam-drill")]
pub const DRILL_FIELDS: usize = drill_fields();
#[cfg(not(feature = "seam-drill"))]
pub const DRILL_FIELDS: usize = 0;

/// The install line's buffer size: every part at its longest, plus the overflow ending's reserve.
pub const INSTALL_LINE_CAPACITY: usize = FIXED_FIELDS + SITE_FIELDS + FEATURE_FIELDS + SEAM_FIELDS + DRILL_FIELDS + OVERFLOW_TAIL;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::guard::GuardOutcome;
    use crate::model::install_line::{render_into, DeveloperReport, Extra, FeatureReport, InstallLine, SettingsReport, SiteReport, Written};
    use crate::model::version::Verdict;
    use core::fmt;

    /// The seam fields as `seam_install::InstallFields` writes them, with every row in every list and the longest code.
    struct WorstSeams;

    impl fmt::Display for WorstSeams {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let list = |f: &mut fmt::Formatter<'_>, code: bool| -> fmt::Result {
                f.write_str("[")?;
                for (i, row) in native_seams::SITES.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    f.write_str(row.label)?;
                    if code {
                        f.write_str(":out_of_region")?;
                    }
                }
                f.write_str("]")
            };
            write!(f, "seams=99999 seams_stock=99999 seams_other=")?;
            list(f, true)?;
            f.write_str(" seams_skipped=")?;
            list(f, true)?;
            f.write_str(" seams_shadow_diff=")?;
            list(f, true)?;
            f.write_str(" seams_missing=[")?;
            for i in 0..MISSING_SHOWN {
                if i > 0 {
                    f.write_str(",")?;
                }
                f.write_str(&"m".repeat(MAX_MISSING_LABEL))?;
            }
            f.write_str(",...] seams_external=")?;
            list(f, false)?;
            write!(f, " seams_unchecked=99999 seams_images_unchecked=99999")
        }
    }

    /// The longest line the boot can write: every row of every table with the longest outcome words, the most feature reports
    /// with the longest names and reasons, the seam fields at their longest, every fixed field at its longest. It fits the buffer
    /// with the reserve to spare, ends with its result and allocation count, and the buffer was not reallocated.
    #[test]
    fn the_worst_case_line_fits_the_capacity_and_the_buffer_never_grows() {
        let reports: Vec<SiteReport> = TABLES
            .iter()
            .flat_map(|table| table.iter())
            .map(|site| SiteReport { site: site.name, found: GuardOutcome::HookedByOther { at: 0, usable: false }, written: Written::Restored })
            .collect();
        let name = "f".repeat(MAX_FEATURE_NAME);
        let reason = "r".repeat(MAX_REASON);
        let (name, reason): (&'static str, &'static str) = (Box::leak(name.into_boxed_str()), Box::leak(reason.into_boxed_str()));
        let features: Vec<FeatureReport> =
            (0..MAX_FEATURE_REPORTS).map(|_| FeatureReport { feature: name, installed: false, reason: Some(reason), site: None }).collect();
        let migration = "m".repeat(96);
        let seams = WorstSeams;
        let line = InstallLine {
            tick: u64::MAX,
            version: "0.1.0-a-long-pre-release-tag-xx",
            display: &[b'9'; 16],
            build_id: Some(&[0xff; 64]),
            verdict: Verdict::Supported,
            sites: &reports,
            features: &features,
            seams: Some(Extra(&seams)),
            developer: Some(DeveloperReport { on: false, reason: "duplicate_key" }),
            settings: Some(SettingsReport { input_delay: 255, lra_reset: false, stage_hazards: false }),
            migration: Some(&migration),
            heap_allocations: Some(u64::MAX),
        };
        let mut buffer = vec![0u8; INSTALL_LINE_CAPACITY];
        let (pointer, capacity) = (buffer.as_ptr(), buffer.capacity());
        let (text, overflow) = render_into(&mut buffer, &line);
        let length = text.len();
        assert!(!overflow, "the worst case ({length} B) overflowed {INSTALL_LINE_CAPACITY} B");
        assert!(length + OVERFLOW_TAIL <= INSTALL_LINE_CAPACITY);
        assert!(text.ends_with(" result=installed heap_allocations=18446744073709551615"));
        assert_eq!(text, line.to_string());
        assert_eq!((buffer.as_ptr(), buffer.capacity(), buffer.len()), (pointer, capacity, INSTALL_LINE_CAPACITY), "the sink reallocated");
    }

    /// The constants the bound assumes hold for every row and every word the line writes.
    #[test]
    fn the_bounds_cover_every_word() {
        for outcome in [
            GuardOutcome::Stock,
            GuardOutcome::HookedByOther { at: 0, usable: true },
            GuardOutcome::HookedByOther { at: 0, usable: false },
            GuardOutcome::Shared,
            GuardOutcome::Foreign { first_diff: 0 },
        ] {
            assert!(outcome.code().len() <= MAX_FOUND_WORD);
        }
        for written in [Written::Near, Written::Far, Written::Skipped, Written::Restored, Written::Checked] {
            assert!(written.word().len() <= MAX_WRITTEN_WORD);
        }
        for bits in 0..=u8::MAX {
            assert!(native_seams::report::code_name(bits).len() <= MAX_SEAM_CODE);
        }
        for feature in [
            sites::Feature::Training,
            sites::Feature::CpuShieldDamage,
            sites::Feature::NamedMacros,
            sites::Feature::InputDisplay,
            sites::Feature::UiTexture,
        ] {
            assert!(feature.name().len() <= MAX_FEATURE_NAME);
        }
    }
}
