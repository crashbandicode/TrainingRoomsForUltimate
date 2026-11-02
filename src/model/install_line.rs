//! The install line: the one line the shipped plugin writes at boot to
//! `sd:/ultimate/TrainingRooms/events.log` and to the Skyline log (SPEC-core-01 §7, playbook
//! "Log what happened"). Formatted straight into a `fmt::Write` sink, so writing it allocates
//! nothing.
//!
//! ```text
//! <ms> plugin=training_rooms/0.1.0 game=13.0.5 build=21450c64… <site>=<found>/<written> …
//!   <feature>=installed|not_installed … developer=off reason=missing settings: input_delay=5 lra_reset=on stage_hazards=off
//!   migration=<…> result=installed heap_allocations=0
//! <ms> plugin=training_rooms/0.1.0 game=13.0.4 build=none result=unsupported_version
//!   reason=display_version heap_allocations=0
//! ```
//!
//! The first token is milliseconds since the console booted (`tick_to_ms` of the system tick). A
//! feature is `installed` when every site it needs passed its guard and its hook was written; the
//! `settings:` group is what the player had chosen at boot, which is a different thing.
//!
//! The playbook's form puts `result=` last before `heap_allocations=`; SPEC-core-01 §7 lists
//! `result=` first. This follows the playbook (the lead's brief: the playbook wins).

use core::fmt::{self, Display, Formatter};

use super::guard::GuardOutcome;
use super::version::{Hex, Printable, Verdict};

/// What the installer did at a site (the word after `/` in `<site>=<entry>/<outcome>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Written {
    /// A one-word branch (an inline hook).
    Near,
    /// A far jump (a replace hook or a vtable-slot wrapper).
    Far,
    /// The site was left alone because the guard refused it.
    Skipped,
    /// A write was undone after a failed check.
    Restored,
    /// A called or relied-on site: compared, nothing written.
    Checked,
}

impl Written {
    pub const fn word(self) -> &'static str {
        match self {
            Written::Near => "near",
            Written::Far => "far",
            Written::Skipped => "skipped",
            Written::Restored => "restored",
            Written::Checked => "checked",
        }
    }
}

/// One site's result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SiteReport {
    pub site: &'static str,
    /// What the guard found at the site (`stock`, `other`, `occupied`, `shared`, `foreign`).
    pub found: GuardOutcome,
    pub written: Written,
}

/// One feature's install result: `installed` when every site it needs passed its guard and the hook
/// was written, `not_installed` otherwise. Not the player's setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureReport {
    pub feature: &'static str,
    pub installed: bool,
    /// Why a feature is not installed, when it has a reason to name (`no_nro_hook`, a provider that is
    /// absent: the provider pattern of HOOK-SHARING, not a workaround).
    pub reason: Option<&'static str>,
    /// The first row of the feature's closure that its guard rejected (R2-1): `reason=site:<name>`. A
    /// feature is not installed when any row it can reach is rejected, whichever feature owns the row.
    pub site: Option<&'static str>,
}

/// The settings values in force at boot (what the player chose, after the file was read).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsReport {
    pub input_delay: u8,
    pub lra_reset: bool,
    pub stage_hazards: bool,
}

/// The Developer-page gate's outcome at boot (decision N1): `developer=on|off reason=<why>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeveloperReport {
    pub on: bool,
    /// `developer_ini::Reason::text()`.
    pub reason: &'static str,
}

/// The console's system tick rate (`nn::os::GetSystemTickFrequency`, 19.2 MHz).
pub const SYSTEM_TICK_HZ: u64 = 19_200_000;

/// System ticks to milliseconds since boot.
pub const fn tick_to_ms(tick: u64) -> u64 {
    tick / (SYSTEM_TICK_HZ / 1000)
}

/// Fields another package formats itself (the seam installer's): written after a space, as given.
#[derive(Clone, Copy)]
pub struct Extra<'a>(pub &'a dyn Display);

impl fmt::Debug for Extra<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Extra(..)")
    }
}

/// Everything the install line says.
#[derive(Clone, Copy, Debug)]
pub struct InstallLine<'a> {
    /// A monotonic tick or the Unix time in milliseconds; the first token of the line.
    pub tick: u64,
    pub version: &'a str,
    /// `nn::oe::DisplayVersion.name` up to its NUL.
    pub display: &'a [u8],
    pub build_id: Option<&'a [u8]>,
    pub verdict: Verdict,
    /// Empty on an unsupported game (nothing is installed).
    pub sites: &'a [SiteReport],
    pub features: &'a [FeatureReport],
    /// The seam installer's fields (package 6: seams=<n> seams_stock=<n> ...), written after the features; None leaves them out.
    pub seams: Option<Extra<'a>>,
    /// The Developer-page gate (decision N1); `None` leaves the `developer=` pair out (an unsupported
    /// game never reads the file).
    pub developer: Option<DeveloperReport>,
    /// `None` leaves the `settings:` group out.
    pub settings: Option<SettingsReport>,
    /// `none`, `done`, `partial` or `skipped` (SPEC-core-08 §2); `None` leaves the token out.
    pub migration: Option<&'a str>,
    /// The counting allocator's count since boot; `None` prints `unmetered`.
    pub heap_allocations: Option<u64>,
}

impl Display for InstallLine<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} plugin=training_rooms/{} game={} build={}",
            self.tick,
            self.version,
            Printable(self.display),
            Hex(self.build_id.unwrap_or(&[])),
        )?;
        if self.verdict == Verdict::Supported {
            for site in self.sites {
                write!(f, " {}={}/{}", site.site, site.found.code(), site.written.word())?;
            }
            for feature in self.features {
                write!(f, " {}={}", feature.feature, if feature.installed { "installed" } else { "not_installed" })?;
                if let Some(reason) = feature.reason {
                    write!(f, " reason={reason}")?;
                } else if let (false, Some(site)) = (feature.installed, feature.site) {
                    write!(f, " reason=site:{site}")?;
                }
            }
            if let Some(extra) = self.seams {
                write!(f, " {}", extra.0)?;
            }
            if let Some(d) = self.developer {
                write!(f, " developer={} reason={}", if d.on { "on" } else { "off" }, d.reason)?;
            }
            if let Some(s) = self.settings {
                write!(
                    f,
                    " settings: input_delay={} lra_reset={} stage_hazards={}",
                    s.input_delay,
                    if s.lra_reset { "on" } else { "off" },
                    if s.stage_hazards { "on" } else { "off" },
                )?;
            }
            if let Some(migration) = self.migration {
                write!(f, " migration={migration}")?;
            }
        }
        write!(f, " result={}", self.verdict.result())?;
        if self.verdict != Verdict::Supported {
            write!(f, " reason={}", self.verdict.reason())?;
        }
        match self.heap_allocations {
            Some(n) => write!(f, " heap_allocations={n}"),
            None => f.write_str(" heap_allocations=unmetered"),
        }
    }
}

/// The whole line as text. Called once, on the boot thread (the counting allocator does not count it): the line has no fixed size
/// because every package adds its sites (8,150 characters at the 2026-11-02 integration head, which a fixed 8 KiB buffer cut before
/// `result=` and `heap_allocations=`).
pub fn render(line: &InstallLine<'_>) -> String {
    use core::fmt::Write;
    let mut text = String::with_capacity(16 * 1024);
    let _ = write!(text, "{line}");
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every site of every table of the merged tree and every feature the boot reports: the line is whole (longer than the old 8 KiB
    /// buffer) and still ends with its result and the allocation count.
    #[test]
    fn the_merged_trees_line_is_whole_and_ends_with_its_result() {
        use super::super::{sites, sites_pkg3, sites_pkg4, sites_pkg6, sites_u4, sites_ui};
        let tables: [&[sites::Site]; 6] =
            [&sites::SITES, &sites_pkg3::STATE_SITES, &sites_pkg4::SITES, &sites_ui::UI_SITES, &sites_u4::SITES, &sites_pkg6::SITES];
        let reports: Vec<SiteReport> = tables
            .iter()
            .flat_map(|table| table.iter())
            .map(|site| SiteReport { site: site.name, found: GuardOutcome::Stock, written: Written::Checked })
            .collect();
        let names = [
            "input_delay", "lra_reset", "stage_hazards", "reset_hub", "cpu_core", "cpu_defence", "cpu_ledge", "cpu_tech", "cpu_shield",
            "cpu_mash", "cpu_common", "state_core", "chords", "input_display", "recording", "notices", "ui", "p1_cpu", "ui_texture",
            "named_macros", "live_replay", "macros_port", "roster",
        ];
        let features: Vec<FeatureReport> = names
            .iter()
            .map(|feature| FeatureReport { feature, installed: false, reason: Some("roster=unavailable_until_cycle_lands menu_app=screens_installed"), site: None })
            .collect();
        let seams = "seams=176 seams_stock=176 seams_other=0 seams_foreign=0 seam_groups=cpu_pilot,roster,mii,lobby";
        let line = InstallLine {
            tick: 330571890,
            version: "0.1.0",
            display: b"13.0.5",
            build_id: Some(&[0x21, 0x45]),
            verdict: Verdict::Supported,
            sites: &reports,
            features: &features,
            seams: Some(Extra(&seams)),
            developer: Some(DeveloperReport { on: true, reason: "on" }),
            settings: Some(SettingsReport { input_delay: 0, lra_reset: true, stage_hazards: false }),
            migration: Some("done"),
            heap_allocations: Some(0),
        };
        let text = render(&line);
        assert!(text.len() > 8192, "the merged tree's line ({} characters) is longer than the old fixed buffer", text.len());
        assert!(text.ends_with(" migration=done result=installed heap_allocations=0"), "{}", &text[text.len() - 120..]);
        assert!(text.contains(" roster=not_installed reason=roster=unavailable_until_cycle_lands"));
        assert_eq!(text, line.to_string());
    }

    #[test]
    fn ticks_convert_to_milliseconds() {
        assert_eq!(tick_to_ms(0), 0);
        assert_eq!(tick_to_ms(SYSTEM_TICK_HZ), 1000);
        assert_eq!(tick_to_ms(SYSTEM_TICK_HZ * 3600 + 19_199), 3_600_000);
    }

    #[test]
    fn the_developer_gate_outcome_is_on_the_line() {
        let line = |on, reason| InstallLine {
            tick: 7,
            version: "0.1.0",
            display: b"13.0.5",
            build_id: None,
            verdict: Verdict::Supported,
            sites: &[],
            features: &[],
            seams: None,
            developer: Some(DeveloperReport { on, reason }),
            settings: None,
            migration: None,
            heap_allocations: Some(0),
        };
        assert!(line(false, "duplicate_key").to_string().contains(" developer=off reason=duplicate_key result="));
        assert!(line(true, "on").to_string().contains(" developer=on reason=on result="));
    }

    #[test]
    fn an_unsupported_game_line_has_no_sites() {
        let line = InstallLine {
            tick: 1,
            version: "0.1.0",
            display: b"12.0.0",
            build_id: None,
            verdict: Verdict::UnsupportedVersion,
            sites: &[],
            features: &[],
            seams: None,
            developer: None,
            settings: None,
            migration: None,
            heap_allocations: Some(0),
        };
        assert_eq!(
            line.to_string(),
            "1 plugin=training_rooms/0.1.0 game=12.0.0 build=none result=unsupported_version reason=display_version heap_allocations=0"
        );
    }
}