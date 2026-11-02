//! The standing-notice ids of the carried lobby files (package 4's queue keys a standing notice by a `u8`; the fork keyed them by their
//! header text). One table, one id per header, so a header the carried files post can be found and a duplicate cannot exist.
//! Package 6's other notices (Ready, hot swap) use ids from `0x10` up in their own table; the lobby's start at `0x20`.

/// `(id, header)`: the headers the carried lobby files post through `notifications::notification`.
pub const HEADERS: [(u8, &str); 8] = [
    (0x20, "Practice Stage"),
    (0x21, "Mii Editor"),
    (0x22, "Fighters"),
    (0x23, "Player Character"),
    (0x24, "CPU Character"),
    (0x25, "Kirby Hats"),
    (0x26, "CPU Kirby Hats"),
    (0x27, "Character Item"),
];

/// The slot of `header` in [`HEADERS`].
pub fn id_of(header: &str) -> Option<usize> {
    HEADERS.iter().position(|(_, name)| *name == header)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_headers_are_unique() {
        for (i, (id, name)) in HEADERS.iter().enumerate() {
            assert!(HEADERS[i + 1..].iter().all(|(other, other_name)| other != id && other_name != name));
        }
    }

    /// Every header the carried lobby files post is in the table (a header outside it would be dropped silently).
    #[test]
    fn every_header_the_carried_files_post_has_an_id() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
        let dir = root.join("lobby");
        let mut files = vec![
            dir.join("practice_stage_request.rs"),
            dir.join("practice_stage_observer.rs"),
            root.join("mii/mii_editor_bridge/mod.rs"),
        ];
        files.extend(["lobby.rs", "practice.rs"].map(|name| dir.join("training").join(name)));
        let mut seen = 0;
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            for call in text.match_indices("notifications::notification(\"").chain(text.match_indices("notifications::clear_notification(\"")) {
                let rest = &text[call.0 + call.1.len()..];
                let header = &rest[..rest.find('"').unwrap()];
                assert!(id_of(header).is_some(), "{}: header {header:?} has no standing-notice id", file.display());
                seen += 1;
            }
        }
        assert!(seen >= 4, "the scan found only {seen} calls");
    }
}