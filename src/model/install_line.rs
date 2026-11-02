//! The install line: the one line the shipped plugin writes at boot to
//! `sd:/ultimate/TrainingRooms/events.log` and to the Skyline log (SPEC-core-01 §7, playbook
//! "Log what happened"). Formatted straight into a `fmt::Write` sink, so writing it allocates
//! nothing.
//!
//! ```text
//! <tick> plugin=training_rooms/0.1.0 game=13.0.5 build=21450c64… <site>=<entry>/<outcome> …
//!   <feature>=on|off … migration=<…> result=installed heap_allocations=0
//! <tick> plugin=training_rooms/0.1.0 game=13.0.4 build=none result=unsupported_version
//!   reason=display_version heap_allocations=0
//! ```
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

/// One feature's result (`off` when a site it needs was refused, or the setting is not wired yet).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureReport {
    pub feature: &'static str,
    pub on: bool,
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
                write!(f, " {}={}", feature.feature, if feature.on { "on" } else { "off" })?;
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
    fn an_unsupported_game_line_has_no_sites() {
        let line = InstallLine {
            tick: 1,
            version: "0.1.0",
            display: b"12.0.0",
            build_id: None,
            verdict: Verdict::UnsupportedVersion,
            sites: &[],
            features: &[],
            migration: None,
            heap_allocations: Some(0),
        };
        assert_eq!(
            line.to_string(),
            "1 plugin=training_rooms/0.1.0 game=12.0.0 build=none result=unsupported_version reason=display_version heap_allocations=0"
        );
    }
}