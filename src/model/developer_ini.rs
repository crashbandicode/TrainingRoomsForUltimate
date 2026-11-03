//! `sd:/ultimate/TrainingRooms/developer.ini` (decision N1, `docs/DECISIONS.md`; SPEC-core-01 §2 and
//! its N1 acceptance row): the Developer page is hidden, and its rows do nothing, unless the file says
//! `developer_page=on`. Read once at boot. The gate fails closed: anything that is not one clean,
//! complete, unambiguous `developer_page=on` hides the page, and the reason is reported on the
//! install line (`developer=off reason=…`).
//!
//! Format: one `key=value` per line; spaces around the key, the `=` and the value are ignored; a
//! line whose first non-blank character is `;` or `#`, a `[section]` line and a line without `=`
//! are ignored. The key and the value are compared ASCII case-insensitively (SPEC-core-01 step 2a). A
//! leading UTF-8 byte-order mark is skipped. The whole file is validated before it is trusted:
//!
//! - a missing file, a read error (including one after a partial read) or any byte that is not
//!   valid UTF-8 anywhere in the file hides the page;
//! - a file longer than `MAX_BYTES` hides the page (the loader reads `MAX_BYTES + 1` bytes, which is
//!   enough to tell, and never trusts the prefix);
//! - the key appearing more than once, in any order and with any values, hides the page.

use std::io::Read;

/// The largest file that is accepted (the fork's INI files cap at the same size).
pub const MAX_BYTES: usize = 8 * 1024;

/// The key the Developer page reads.
pub const KEY: &str = "developer_page";

/// Runtime publication gate for Developer rows. The persisted choice is an input,
/// never rewritten when a boot hides the page (SPEC-core-01, decision N1).
pub const fn effective(page_enabled: bool, stored: bool) -> bool {
    page_enabled && stored
}

/// The two stored Developer toggles, both ordered `[On, Off]` by the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    DebugHooks,
    ReloadConfig,
}

impl Control {
    pub const fn id(self) -> &'static str {
        match self {
            Self::DebugHooks => "dev_debug_hooks",
            Self::ReloadConfig => "dev_reload_config",
        }
    }
}

/// Publish the displayed On choice, gated by this boot's Developer-page setting.
/// Reading a hidden page never changes its persisted choices.
pub fn control_enabled(snapshot: &super::settings::Snapshot, page_enabled: bool, control: Control) -> bool {
    let stored = snapshot.get(control.id()).is_some_and(|row| row.first().is_some_and(|v| *v != 0));
    effective(page_enabled, stored)
}

/// Why the page is shown or hidden; the install line prints `text()`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reason {
    /// No `developer.ini`.
    #[default]
    Missing,
    /// The file exists but opening or reading it failed.
    Unreadable,
    /// The file is longer than `MAX_BYTES`.
    TooLarge,
    /// The file is not valid UTF-8.
    NotUtf8,
    /// `developer_page` appears more than once.
    DuplicateKey,
    /// The file has no `developer_page` line.
    KeyAbsent,
    /// `developer_page` has a value other than exactly `on`.
    NotOn,
    /// One `developer_page=on` line in a clean file.
    On,
}

impl Reason {
    pub const fn text(self) -> &'static str {
        match self {
            Reason::Missing => "missing",
            Reason::Unreadable => "unreadable",
            Reason::TooLarge => "too_large",
            Reason::NotUtf8 => "not_utf8",
            Reason::DuplicateKey => "duplicate_key",
            Reason::KeyAbsent => "key_absent",
            Reason::NotOn => "not_on",
            Reason::On => "on",
        }
    }
}

/// What the file says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeveloperIni {
    /// The page is shown: the file held exactly one `developer_page=on` line and nothing disqualified it.
    pub developer_page: bool,
    pub reason: Reason,
}

impl DeveloperIni {
    const fn hidden(reason: Reason) -> Self {
        Self { developer_page: false, reason }
    }
}

/// Parse the whole contents of the file. `contents` is everything that was read; a value longer than
/// `MAX_BYTES` means the file is over the limit (the loader reads `MAX_BYTES + 1` bytes at most).
pub fn parse(contents: &[u8]) -> DeveloperIni {
    if contents.len() > MAX_BYTES {
        return DeveloperIni::hidden(Reason::TooLarge);
    }
    let Ok(text) = std::str::from_utf8(contents) else {
        return DeveloperIni::hidden(Reason::NotUtf8);
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut seen: Option<bool> = None;
    for line in text.split('\n') {
        let line = line.trim_ascii();
        if line.is_empty() || line.starts_with([';', '#', '[']) {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if !key.trim_ascii().eq_ignore_ascii_case(KEY) {
            continue;
        }
        if seen.is_some() {
            return DeveloperIni::hidden(Reason::DuplicateKey);
        }
        seen = Some(value.trim_ascii().eq_ignore_ascii_case("on"));
    }
    match seen {
        None => DeveloperIni::hidden(Reason::KeyAbsent),
        Some(false) => DeveloperIni::hidden(Reason::NotOn),
        Some(true) => DeveloperIni { developer_page: true, reason: Reason::On },
    }
}

/// Read the file from `reader`: at most `MAX_BYTES + 1` bytes (one more than the limit tells an
/// oversized file from one of exactly the limit), a read error at any point rejects the file.
pub fn read(reader: impl Read) -> DeveloperIni {
    let mut bytes = Vec::new();
    match reader.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes) {
        Ok(_) => parse(&bytes),
        Err(_) => DeveloperIni::hidden(Reason::Unreadable),
    }
}

/// Load the file at `path` (the boot loader and the host tests use this one path).
pub fn load(path: &str) -> DeveloperIni {
    match std::fs::File::open(path) {
        Ok(file) => read(file),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DeveloperIni::hidden(Reason::Missing),
        Err(_) => DeveloperIni::hidden(Reason::Unreadable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_exact_on_shows_the_page() {
        assert!(parse(b"developer_page=on").developer_page);
        assert!(!parse(b"developer_page=yes").developer_page);
        assert!(!parse(b"").developer_page);
    }
}
