//! The hook-sharing drill's setting and sites (playbook HOOK-SHARING "Testing the chain in Eden"). Pure
//! data and parsing: the stand-in hooks themselves are in the root crate's diagnostic build
//! (`src/swap_native/seam_drill.rs`), which reads the setting from the plugin's diagnostic file.

/// How the drill hooks its sites, as a plugin that does not share would.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standin {
    None,
    /// An inline hook: a one-word branch to a counting callback that returns to the site.
    Inline,
    /// A replace hook: a far jump to a handler that counts and continues to Skyline's trampoline.
    Far,
}

/// `foreign=<inline|far>[,after]`: the stand-in form, and whether it goes in after this plugin's own
/// hooks (the other load order) instead of before them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drill {
    pub standin: Standin,
    pub after: bool,
}

impl Drill {
    pub const OFF: Drill = Drill { standin: Standin::None, after: false };

    /// The last `foreign=` line of the file decides; anything unknown is off.
    pub fn parse(text: &[u8]) -> Drill {
        text.split(|b| *b == b'\n').fold(Drill::OFF, |found, line| {
            let Some(eq) = line.iter().position(|b| *b == b'=') else {
                return found;
            };
            if line[..eq].trim_ascii() != b"foreign" {
                return found;
            }
            let mut parts = line[eq + 1..].split(|b| *b == b',').map(|part| part.trim_ascii());
            let standin = match parts.next() {
                Some(b"inline") => Standin::Inline,
                Some(b"far") => Standin::Far,
                _ => return Drill::OFF,
            };
            match (parts.next(), parts.next()) {
                (None, _) => Drill { standin, after: false },
                (Some(b"after"), None) => Drill { standin, after: true },
                _ => Drill::OFF,
            }
        })
    }

    pub const fn name(self) -> &'static str {
        match (self.standin, self.after) {
            (Standin::None, _) => "none",
            (Standin::Inline, false) => "inline",
            (Standin::Far, false) => "far",
            (Standin::Inline, true) => "inline,after",
            (Standin::Far, true) => "far,after",
        }
    }

    /// The form's name in the run-count line (`<site>_<form>=<n>`).
    pub const fn form(self) -> &'static str {
        match self.standin {
            Standin::None => "none",
            Standin::Inline => "inline",
            Standin::Far => "far",
        }
    }
}

/// One site per sharing rule: (name in the count line, the rows at that address and its pair). The
/// stand-in goes on the first row's address.
pub const SITES: [(&str, &[&str]); 4] = [
    // `Sharing::Chain` replace row (`effect_hook_chain`).
    ("effect", &["training.handle_effect"]),
    // A function this plugin only calls.
    ("popup_idle", &["game_mode.popup_idle"]),
    // `Sharing::EntryOrBody`: the entry row and its body row.
    ("write_record", &["replay.write_record", "replay.write_record_shared"]),
    // `Sharing::OwnHook`: our replace hook's row and the Called row on the same address.
    ("set_cpu_controls", &["needle.set_cpu_controls", "cpu_pilot.ai_think"]),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::seams::native_seams::{all, find_site, Sharing};

    #[test]
    fn the_drill_setting_parses_and_names_both_orders() {
        assert_eq!(Drill::parse(b""), Drill::OFF);
        assert_eq!(Drill::parse(b"foreign=inline\n"), Drill { standin: Standin::Inline, after: false });
        assert_eq!(Drill::parse(b"x=1\nforeign = far \n"), Drill { standin: Standin::Far, after: false });
        assert_eq!(Drill::parse(b"foreign=inline,after"), Drill { standin: Standin::Inline, after: true });
        assert_eq!(Drill::parse(b"foreign=far, after\r"), Drill { standin: Standin::Far, after: true });
        assert_eq!(Drill::parse(b"foreign=far\nforeign=off"), Drill::OFF, "the last line decides");
        assert_eq!(Drill::parse(b"foreign=far,before"), Drill::OFF);
        assert_eq!(Drill::parse(b"foreign=far,after").name(), "far,after");
        assert_eq!(Drill::parse(b"foreign=inline").form(), "inline");
        // One site per rule, each row present; a pair's rows share the address or name the body.
        for (name, rows) in SITES {
            let (_, first) = find_site(rows[0]).unwrap_or_else(|| panic!("{}: no row {}", name, rows[0]));
            for row in &rows[1..] {
                let (_, other) = find_site(row).unwrap_or_else(|| panic!("{}: no row {}", name, row));
                let pair = all().find(|s| s.label == rows[0]).map(|s| s.sharing);
                assert!(
                    other.offset == first.offset
                        || matches!(pair, Some(Sharing::EntryOrBody { body, .. }) if body == other.offset),
                    "{}: {} is not the pair of {}",
                    name,
                    row,
                    rows[0]
                );
            }
        }
        let rules: Vec<_> = SITES.iter().map(|(_, rows)| all().find(|s| s.label == rows[0]).unwrap().sharing).collect();
        assert!(matches!(rules[0], Sharing::Chain));
        assert!(matches!(rules[2], Sharing::EntryOrBody { .. }));
        assert!(all().any(|s| s.label == SITES[3].1[1] && matches!(s.sharing, Sharing::OwnHook { .. })));
    }
}
