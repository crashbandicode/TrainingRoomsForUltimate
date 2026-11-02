//! `sd:/ultimate/TrainingRooms/developer.ini` (decision N1, `docs/DECISIONS.md`): the Developer
//! page is hidden, and its rows do nothing, unless the file says `developer_page=on`. Read once at
//! boot. A missing file, an unreadable one, or any other value means hidden.
//!
//! Format: one `key=value` per line; spaces around the key, the `=` and the value are ignored; a
//! line whose first non-blank character is `;` or `#`, a `[section]` line and a line without `=`
//! are ignored. The key and the value are compared exactly (`developer_page`, `on`). When the key
//! appears more than once, the last line wins, as in the fork's other INI files. A leading UTF-8
//! byte-order mark is skipped. At most `MAX_BYTES` are read.

/// The most bytes of the file that are read (the fork's INI files cap at the same size).
pub const MAX_BYTES: usize = 8 * 1024;

/// The key the Developer page reads.
pub const KEY: &str = "developer_page";

/// What the file says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeveloperIni {
    /// `developer_page=on` was the last `developer_page` line.
    pub developer_page: bool,
}

/// Parse the contents of the file (bytes, so a file that is not UTF-8 simply reads as hidden).
pub fn parse(contents: &[u8]) -> DeveloperIni {
    let contents = &contents[..contents.len().min(MAX_BYTES)];
    let contents = contents.strip_prefix(b"\xef\xbb\xbf").unwrap_or(contents);
    let mut result = DeveloperIni::default();
    for line in contents.split(|b| *b == b'\n') {
        let line = trim(line);
        if line.is_empty() || line[0] == b';' || line[0] == b'#' || line[0] == b'[' {
            continue;
        }
        let Some(eq) = line.iter().position(|b| *b == b'=') else {
            continue;
        };
        let (key, value) = (trim(&line[..eq]), trim(&line[eq + 1..]));
        if key == KEY.as_bytes() {
            result.developer_page = value == b"on";
        }
    }
    result
}

fn trim(mut bytes: &[u8]) -> &[u8] {
    while let [first, rest @ ..] = bytes {
        if first.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
    while let [rest @ .., last] = bytes {
        if last.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
    bytes
}
