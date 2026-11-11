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

