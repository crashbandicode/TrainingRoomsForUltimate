//! Startup-only INI parsing and fixed-size, edge-triggered menu shortcuts (SPEC-input-04 §2.3, §7, §9).
//!
//! Carried over from the earlier fork's `training_mod_consts/src/menu_shortcuts.rs` (stage m5o,
//! ours-new, sha256 `b19b20ac365178ff7ddba0b4801698872048de0081e8955ed4df9e87ea2e6d0d`). Clean-room
//! changes (CARRY-OVER.md, `## pkg4`): the fork's `ButtonConfig` (a byteflags struct of the fork's
//! const crate) is the plugin's own `Chord` (a `u16` of the sixteen logical buttons, bit order of the
//! chord settings' arrays); the button names are tables here; the doc line of `DEFAULT_INI` no longer
//! says "restart" (draft 9, N7: the menu edits the file and a change applies at once); added
//! `to_ini_text`, `with_slot` and `ini_name` for the menu's write (N7), and the tests moved to
//! `Chord`.

use super::menu::chord::{Chord, BUTTONS as BUTTON_COUNT};

/// The permanent chord: B + D-pad Up, never editable, never disabled.
pub const DEFAULT_OPEN_MENU_CONFIG: Chord = Chord((1 << 1) | (1 << 8));

pub const DEFAULT_INI: &str =
    "# B + DPad Up is always available. The menu edits this file; a change applies at once.\n[menu]\nshortcut_1=Minus\nshortcut_2=disabled\n";
pub const DEFAULT_SHORTCUTS: MenuShortcuts = MenuShortcuts { additional: [Chord::DEFAULT_SHORTCUT_1, Chord::NONE] };

/// The file's name below the storage root and its size cap (SPEC-input-04 §7).
pub const FILE_NAME: &str = "button_config.ini";
pub const MAX_BYTES: usize = 8 * 1024;

/// The field name of each logical button (the identifiers the fork's byteflags struct declared), in
/// bit order of `Chord`.
pub const FIELDS: [&str; BUTTON_COUNT] = [
    "A", "B", "X", "Y", "L", "R", "ZL", "ZR", "DPAD_UP", "DPAD_DOWN", "DPAD_LEFT", "DPAD_RIGHT", "PLUS", "MINUS", "LSTICK", "RSTICK",
];

/// The menu label of each logical button: Pro and GameCube alternatives are separated by `;`
/// (SPEC-input-04 §2.1).
pub const LABELS: [&str; BUTTON_COUNT] = [
    "A",
    "B",
    "X",
    "Y",
    "Pro L",
    "Pro R; GCC Z",
    "Pro ZL; GCC L",
    "Pro ZR; GCC R",
    "DPad Up",
    "DPad Down",
    "DPad Left",
    "DPad Right",
    "Plus",
    "Minus",
    "Left Stick Press",
    "Right Stick Press",
];

/// What the file says for a button, written back: the plain menu name.
const INI_NAMES: [&str; BUTTON_COUNT] = [
    "A", "B", "X", "Y", "L", "R", "ZL", "ZR", "DPAD_UP", "DPAD_DOWN", "DPAD_LEFT", "DPAD_RIGHT", "Plus", "Minus", "Left Stick Press",
    "Right Stick Press",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuShortcuts {
    additional: [Chord; 2],
}

fn button_index(token: &str) -> Option<usize> {
    FIELDS.iter().zip(LABELS.iter()).position(|(field, label)| {
        token.eq_ignore_ascii_case(field) || label.split(';').any(|name| token.eq_ignore_ascii_case(name.trim()))
    })
}

fn parse_chord(value: &str) -> Option<Chord> {
    if value.eq_ignore_ascii_case("disabled") || value.eq_ignore_ascii_case("none") {
        return Some(Chord::NONE);
    }
    let mut chord = Chord::NONE;
    for token in value.split('+').map(str::trim) {
        let index = button_index(token)?;
        if chord.contains(index as u8) {
            return None;
        }
        chord = Chord(chord.0 | (1 << index));
    }
    Some(chord)
}

/// A chord as the file writes it: `Minus`, `ZL + DPAD_UP`; `disabled` for an empty chord.
pub fn ini_name(chord: Chord) -> String {
    if chord.is_empty() {
        return "disabled".to_owned();
    }
    (0..BUTTON_COUNT).filter(|i| chord.contains(*i as u8)).map(|i| INI_NAMES[i]).collect::<Vec<_>>().join(" + ")
}

impl MenuShortcuts {
    /// The two slots as chords.
    pub const fn new(first: Chord, second: Chord) -> Self {
        Self { additional: [first, second] }
    }

    pub const fn slots(self) -> [Chord; 2] {
        self.additional
    }

    /// Replace one slot (0 or 1); the other is kept. A slot index above 1 changes nothing.
    pub fn with_slot(mut self, slot: usize, chord: Chord) -> Self {
        if let Some(target) = self.additional.get_mut(slot) {
            *target = chord;
        }
        self
    }

    /// Unknown keys/sections are ignored. Missing keys use defaults; malformed
    /// or duplicate recognised keys fall back independently to that slot's default.
    pub fn parse(contents: &str) -> Self {
        let mut result = DEFAULT_SHORTCUTS;
        let mut in_menu = false;
        let mut seen = [false; 2];
        let mut invalid = [false; 2];
        for raw in contents.lines() {
            let line = raw.split([';', '#']).next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') {
                in_menu = line
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .is_some_and(|section| section.trim().eq_ignore_ascii_case("menu"));
                continue;
            }
            if !in_menu {
                continue;
            }
            let (key, value) = line.split_once('=').unwrap_or((line, ""));
            let index = if key.trim().eq_ignore_ascii_case("shortcut_1") {
                0
            } else if key.trim().eq_ignore_ascii_case("shortcut_2") {
                1
            } else {
                continue;
            };
            if seen[index] {
                invalid[index] = true;
            }
            seen[index] = true;
            match parse_chord(value.trim()) {
                Some(chord) if !invalid[index] => result.additional[index] = chord,
                _ => invalid[index] = true,
            }
            if invalid[index] {
                result.additional[index] = DEFAULT_SHORTCUTS.additional[index];
            }
        }
        result
    }

    /// The file's text after a confirmed change (draft 9, N7; SPEC-input-04 question 6 (a), the
    /// recommendation: keep comments and unknown lines, replace only the two `shortcut_` lines in the
    /// `[menu]` section). A missing `[menu]` section or key is added. `original` is the file as read,
    /// or the default text when there was none.
    pub fn to_ini_text(self, original: &str) -> String {
        let mut out = String::with_capacity(original.len() + 64);
        let mut in_menu = false;
        let mut have_menu = false;
        let mut wrote = [false; 2];
        let value = |slot: usize| ini_name(self.additional[slot]);
        let flush_missing = |out: &mut String, wrote: &mut [bool; 2]| {
            for (slot, done) in wrote.iter_mut().enumerate() {
                if !*done {
                    out.push_str(&format!("shortcut_{}={}\n", slot + 1, value(slot)));
                    *done = true;
                }
            }
        };
        for raw in original.lines() {
            let line = raw.split([';', '#']).next().unwrap_or("").trim();
            if line.starts_with('[') {
                if in_menu {
                    flush_missing(&mut out, &mut wrote);
                }
                in_menu = line
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .is_some_and(|section| section.trim().eq_ignore_ascii_case("menu"));
                have_menu |= in_menu;
                out.push_str(raw);
                out.push('\n');
                continue;
            }
            if in_menu {
                let key = line.split_once('=').map_or(line, |(k, _)| k).trim();
                let slot = if key.eq_ignore_ascii_case("shortcut_1") {
                    Some(0)
                } else if key.eq_ignore_ascii_case("shortcut_2") {
                    Some(1)
                } else {
                    None
                };
                if let Some(slot) = slot {
                    // A duplicate key's later lines are dropped: the file then has one line per slot.
                    if !wrote[slot] {
                        out.push_str(&format!("shortcut_{}={}\n", slot + 1, value(slot)));
                        wrote[slot] = true;
                    }
                    continue;
                }
            }
            out.push_str(raw);
            out.push('\n');
        }
        if in_menu {
            flush_missing(&mut out, &mut wrote);
        }
        if !have_menu {
            out.push_str("[menu]\n");
            flush_missing(&mut out, &mut wrote);
        }
        out
    }

    /// The text for the menu: `Minus or B + DPad Up`.
    pub fn description(self) -> String {
        [self.additional[0], self.additional[1], DEFAULT_OPEN_MENU_CONFIG]
            .iter()
            .copied()
            .filter(|chord| !chord.is_empty())
            .map(|chord| (0..BUTTON_COUNT).filter(|i| chord.contains(*i as u8)).map(|i| LABELS[i].split(';').next().unwrap_or("")).collect::<Vec<_>>().join(" + "))
            .collect::<Vec<_>>()
            .join(" or ")
    }

    /// Controller mapping stays with the input owner. This only combines logical
    /// button predicates and does not allocate or retain controller state.
    pub fn passes(self, mut held: impl FnMut(u8) -> bool, mut pressed: impl FnMut(u8) -> bool) -> bool {
        [DEFAULT_OPEN_MENU_CONFIG, self.additional[0], self.additional[1]].iter().copied().any(|chord| {
            let mut edge = false;
            let mut any = false;
            for button in (0..BUTTON_COUNT as u8).filter(|b| chord.contains(*b)) {
                any = true;
                if !held(button) {
                    return false;
                }
                edge |= pressed(button);
            }
            any && edge
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: Chord = Chord(1 << 1);
    const X: Chord = Chord(1 << 2);
    const Y: Chord = Chord(1 << 3);
    const R: Chord = Chord(1 << 5);
    const ZL: Chord = Chord(1 << 6);
    const DPAD_UP: Chord = Chord(1 << 8);
    const DPAD_RIGHT: Chord = Chord(1 << 11);
    const PLUS: Chord = Chord(1 << 12);
    const MINUS: Chord = Chord(1 << 13);
    const LSTICK: Chord = Chord(1 << 14);

    fn union(a: Chord, b: Chord) -> Chord {
        Chord(a.0 | b.0)
    }

    fn passes(config: MenuShortcuts, held: Chord, pressed: Chord) -> bool {
        config.passes(|button| held.contains(button), |button| pressed.contains(button))
    }

    #[test]
    fn defaults_and_missing_keys() {
        assert_eq!(MenuShortcuts::parse(""), DEFAULT_SHORTCUTS);
        assert_eq!(MenuShortcuts::parse(DEFAULT_INI), DEFAULT_SHORTCUTS);
        assert_eq!(MenuShortcuts::parse("[other]\nshortcut_1=Plus"), DEFAULT_SHORTCUTS);
        let config = MenuShortcuts::parse("[menu]\nshortcut_2=ZL + R");
        assert_eq!(config.additional, [MINUS, union(ZL, R)]);
    }

    #[test]
    fn supported_names_comments_and_disabled_slots() {
        let config =
            MenuShortcuts::parse("[ MeNu ]\nSHORTCUT_1= disabled ; off\nshortcut_2= Left Stick Press + DPAD_RIGHT # chord");
        assert_eq!(config.additional, [Chord::NONE, union(LSTICK, DPAD_RIGHT)]);
        assert_eq!(MenuShortcuts::parse("[menu]\nshortcut_1=none").additional[0], Chord::NONE);
    }

    #[test]
    fn invalid_or_duplicate_slot_falls_back_independently() {
        for value in ["", "garbage", "B +", "+ B", "B + B", "disabled + B"] {
            let config = MenuShortcuts::parse(&format!("[menu]\nshortcut_1={value}\nshortcut_2=X"));
            assert_eq!(config.additional, [MINUS, X]);
        }
        let config = MenuShortcuts::parse("[menu]\nshortcut_1=X\nshortcut_1=Y\nshortcut_2=Plus");
        assert_eq!(config.additional, [MINUS, PLUS]);
        assert_eq!(MenuShortcuts::parse("[menu]\nshortcut_2=X\nshortcut_2=Y").additional[1], Chord::NONE);
    }

    #[test]
    fn permanent_chord_survives_disabled_extras_and_uses_press_edges() {
        let config = MenuShortcuts::parse("[menu]\nshortcut_1=disabled\nshortcut_2=disabled");
        let held = union(B, DPAD_UP);
        assert!(passes(config, held, B));
        assert!(passes(config, held, DPAD_UP));
        assert!(!passes(config, held, Chord::NONE));
        assert!(!passes(config, B, B));
    }

    #[test]
    fn minus_is_edge_triggered_and_plus_is_not_an_implicit_opener() {
        assert!(passes(DEFAULT_SHORTCUTS, MINUS, MINUS));
        assert!(!passes(DEFAULT_SHORTCUTS, MINUS, Chord::NONE));
        assert!(!passes(DEFAULT_SHORTCUTS, PLUS, PLUS));
        assert!(!passes(DEFAULT_SHORTCUTS, PLUS, Chord::NONE));
        assert!(passes(MenuShortcuts::parse("[menu]\nshortcut_2=Plus"), PLUS, PLUS));
    }

    #[test]
    fn custom_chord_requires_every_held_button_and_one_new_edge() {
        let config = MenuShortcuts::parse("[menu]\nshortcut_1=disabled\nshortcut_2=R + ZL");
        let both = union(R, ZL);
        assert!(passes(config, both, R));
        assert!(passes(config, both, ZL));
        assert!(!passes(config, R, R));
        assert!(!passes(config, both, Chord::NONE));
    }

    #[test]
    fn to_ini_text_roundtrip() {
        for (a, b) in [(MINUS, Chord::NONE), (Chord::NONE, union(ZL, DPAD_UP)), (union(X, Y), union(R, LSTICK)), (Chord::NONE, Chord::NONE)] {
            let shortcuts = MenuShortcuts::new(a, b);
            let text = shortcuts.to_ini_text(DEFAULT_INI);
            assert_eq!(MenuShortcuts::parse(&text), shortcuts, "{text}");
        }
        assert!(MenuShortcuts::new(Chord::NONE, Chord::NONE).to_ini_text(DEFAULT_INI).contains("shortcut_1=disabled"));
    }

    #[test]
    fn to_ini_text_keeps_comments_and_unknown_lines() {
        let original = "# my notes\n[misc]\nfoo=bar\n[menu]\n; keep this\nshortcut_2=R\nother=1\nshortcut_1=Plus\n[tail]\nx=y\n";
        let text = MenuShortcuts::new(X, Chord::NONE).to_ini_text(original);
        assert!(text.contains("# my notes\n") && text.contains("foo=bar\n") && text.contains("; keep this\n"));
        assert!(text.contains("other=1\n") && text.contains("[tail]\nx=y\n"));
        assert_eq!(MenuShortcuts::parse(&text).additional, [X, Chord::NONE]);
        // Each slot is written exactly once.
        assert_eq!(text.matches("shortcut_1=").count(), 1);
        assert_eq!(text.matches("shortcut_2=").count(), 1);
        // A file with no [menu] section gains one; an empty file is the default text plus the change.
        let bare = MenuShortcuts::new(X, Y).to_ini_text("# only a comment\n");
        assert_eq!(MenuShortcuts::parse(&bare).additional, [X, Y]);
        assert!(bare.starts_with("# only a comment\n"));
        assert_eq!(MenuShortcuts::parse(&MenuShortcuts::new(X, Y).to_ini_text("")).additional, [X, Y]);
    }

    #[test]
    fn description_reads_aloud() {
        assert_eq!(DEFAULT_SHORTCUTS.description(), "Minus or B + DPad Up");
        assert_eq!(MenuShortcuts::new(Chord::NONE, Chord::NONE).description(), "B + DPad Up");
    }

    #[test]
    fn written_names_parse_back_for_every_button() {
        for bit in 0..BUTTON_COUNT {
            let chord = Chord(1 << bit);
            assert_eq!(parse_chord(&ini_name(chord)), Some(chord), "button {bit}");
        }
    }
}
