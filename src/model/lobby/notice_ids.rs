//! The standing-notice ids of the carried lobby files (package 4's queue keys a standing notice by a `u8`; the fork keyed them by their
//! header text). One table, one id per header, so a header the carried files post can be found and a duplicate cannot exist.
//! Package 6's other notices (Ready, hot swap) use ids from `0x10` up in their own table; the lobby's start at `0x20`; the named-macro layer's (package 4, S4-4) start at `0x30`.

/// `(id, header)`: the headers the carried lobby files post through `notifications::notification`.
pub const HEADERS: [(u8, &str); 10] = [
    (0x20, "Practice Stage"),
    (0x21, "Mii Editor"),
    (0x22, "Fighters"),
    (0x23, "Player Character"),
    (0x24, "CPU Character"),
    (0x25, "Kirby Hats"),
    (0x26, "CPU Kirby Hats"),
    (0x27, "Character Item"),
    (0x30, "Named Macro"),
    (0x31, "Clip Editor"),
];

/// The slot of `header` in [`HEADERS`].
pub fn id_of(header: &str) -> Option<usize> {
    HEADERS.iter().position(|(_, name)| *name == header)
}

