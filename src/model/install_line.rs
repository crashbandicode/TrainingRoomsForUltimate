//! The install line: the one line the shipped plugin writes at boot to
//! `sd:/ultimate/TrainingRooms/events.log` and to the Skyline log (SPEC-core-01 §7, playbook
//! "Log what happened"). Formatted straight into a `fmt::Write` sink, so writing it allocates
//! nothing.
//!
//! ```text
//! <ms> plugin=training_rooms/0.1.0 game=13.0.5 build=21450c64… <site>=<found>/<written> …
//!   <feature>=installed|not_installed … settings: input_delay=5 lra_reset=on stage_hazards=off
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
}

/// The settings values in force at boot (what the player chose, after the file was read).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsReport {
    pub input_delay: u8,
    pub lra_reset: bool,
    pub stage_hazards: bool,
}

/// The bytes the runtime formats the install line into (a stack buffer). A line that is longer is cut, which loses
/// `result=` and `heap_allocations=`: `tests/cpu_sites.rs` renders the line of every site and feature of the table and
/// keeps it under this with room to spare, so adding sites cannot silently cut it again.
pub const INSTALL_LINE_CAPACITY: usize = 4096;

/// The console's system tick rate (`nn::os::GetSystemTickFrequency`, 19.2 MHz).
pub const SYSTEM_TICK_HZ: u64 = 19_200_000;

/// System ticks to milliseconds since boot.
pub const fn tick_to_ms(tick: u64) -> u64 {
    tick / (SYSTEM_TICK_HZ / 1000)
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
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_convert_to_milliseconds() {
        assert_eq!(tick_to_ms(0), 0);
        assert_eq!(tick_to_ms(SYSTEM_TICK_HZ), 1000);
        assert_eq!(tick_to_ms(SYSTEM_TICK_HZ * 3600 + 19_199), 3_600_000);
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