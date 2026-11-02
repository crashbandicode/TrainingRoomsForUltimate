//! The portrait roster grid (Player Character, CPU Character, Character Cycles, Kirby Hats): package 6's roster rules
//! (`engine::swap::roster_grid`: ranks, All, the shared Aegis / Pokemon Trainer slots and their previewed form) applied to the menu's
//! kept values, and the tile labels the menu draws. The approved menu's one-page 13 x 7 grid (MENU-PROPOSAL); the grid's cell `i` is
//! the roster grid's cell `i` (six rows of 13, a centred last row of 9).
//!
//! Values (the menu snapshot's kept keys, the forms the typed menu reads):
//! - a pick (`player_character`, `cpu_character`): one-hot over the roster catalog (`engine::swap::trio::CHOICES`);
//! - an order (`player_roster`, `cpu_roster`, `player_kirby_hats`, `cpu_kirby_hats`): `[all, ranks...]` over the same catalog.

use core::fmt::Write;

use serde_json::Value;

use crate::engine::swap::roster_grid::{group_rank, select_all, CellContent, Outcome, Role, RosterGrid, CELLS};
use crate::engine::swap::trio::CHOICES;
use crate::model::frame::{Frame, Text};
use crate::model::menu::grid::RosterUse;
use crate::model::menu::input::Button;

/// What a press changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Nothing,
    /// The values of the active key changed: write them back.
    Changed,
    /// Toggle the active role's independent Ice Climbers preference.
    ToggleSolo,
    /// Only what is shown changed (a previewed form, the role).
    Shown,
}

/// The editor of one open roster grid: the roster rules' cursor-free state (role, previewed forms) for one use.
#[derive(Clone, Copy, Debug)]
pub struct RosterEditor {
    grid: RosterGrid,
    use_: RosterUse,
}

fn kinds() -> Vec<u32> {
    CHOICES.iter().map(|(kind, _)| *kind).collect()
}

impl RosterEditor {
    pub fn new(use_: RosterUse) -> Self {
        let role = if use_ == RosterUse::CpuPick { Role::Cpu } else { Role::Player };
        Self { grid: RosterGrid::new(role), use_ }
    }

    pub fn use_(&self) -> RosterUse {
        self.use_
    }

    fn pick(&self) -> bool {
        matches!(self.use_, RosterUse::PlayerPick | RosterUse::CpuPick)
    }

    pub fn solo_key(&self) -> &'static str {
        if self.grid.role() == Role::Cpu { "solo_popo_cpu" } else { "solo_popo_player_cycle" }
    }

    pub fn solo_enabled(&self, menu: &crate::model::menu::Menu) -> bool {
        menu.kept(self.solo_key()).and_then(Value::as_array).is_some_and(|v| {
            v.len() == 2 && v[0].as_u64() == Some(1) && v[1].as_u64() == Some(0)
        })
    }

    /// Apply one editor input to the menu snapshot, preserving the other role's settings.
    pub fn apply(&mut self, menu: &mut crate::model::menu::Menu, cell: usize, button: Button) {
        let mut values = self.values(menu.kept(self.key()));
        match self.press(cell, button, &mut values) {
            Effect::Changed => menu.set_kept(self.key(), Self::value_of(&values)),
            Effect::ToggleSolo => {
                let on = !self.solo_enabled(menu);
                menu.set_kept(self.solo_key(), serde_json::json!([u8::from(on), u8::from(!on)]));
            }
            Effect::Nothing | Effect::Shown => {}
        }
    }

    /// The kept key this editor writes now (the order keys follow the role ZR chose).
    pub fn key(&self) -> &'static str {
        match (self.use_, self.grid.role()) {
            (RosterUse::PlayerPick, _) => "player_character",
            (RosterUse::CpuPick, _) => "cpu_character",
            (RosterUse::Cycles, Role::Player) => "player_roster",
            (RosterUse::Cycles, Role::Cpu) => "cpu_roster",
            (RosterUse::Hats, Role::Player) => "player_kirby_hats",
            (RosterUse::Hats, Role::Cpu) => "cpu_kirby_hats",
        }
    }

    /// The values of the active key from the kept JSON; a missing or malformed value is the factory form (All, or Mario).
    pub fn values(&self, kept: Option<&Value>) -> Vec<u8> {
        let want = if self.pick() { CHOICES.len() } else { CHOICES.len() + 1 };
        let parsed: Option<Vec<u8>> = kept.and_then(Value::as_array).and_then(|a| {
            let v: Option<Vec<u8>> = a.iter().map(|x| x.as_u64().and_then(|n| u8::try_from(n).ok())).collect();
            v.filter(|v| v.len() == want)
        });
        parsed.unwrap_or_else(|| {
            let mut v = vec![0u8; want];
            v[0] = 1;
            v
        })
    }

    /// The JSON the kept key stores.
    pub fn value_of(values: &[u8]) -> Value {
        Value::Array(values.iter().map(|v| Value::from(*v)).collect())
    }

    /// A pick/add, R remove, Y Solo Popo on Ice Climbers, X form, ZR role.
    pub fn press(&mut self, cell: usize, button: Button, values: &mut [u8]) -> Effect {
        let Some(info) = self.grid.info(cell) else { return Effect::Nothing };
        match (button, info.content) {
            (Button::ZR, _) if !self.pick() => {
                self.grid.toggle_role();
                Effect::Shown
            }
            (Button::X, CellContent::Fighter(kind)) => {
                self.grid.focus_kind(kind);
                if self.grid.cycle_form() {
                    Effect::Shown
                } else {
                    Effect::Nothing
                }
            }
            (Button::A, CellContent::Fighter(kind)) if self.pick() => {
                let Some(index) = CHOICES.iter().position(|(k, _)| *k == kind) else { return Effect::Nothing };
                if values.get(index) == Some(&1) {
                    return Effect::Nothing;
                }
                values.fill(0);
                values[index] = 1;
                Effect::Changed
            }
            (Button::A, CellContent::All) if !self.pick() => {
                select_all(&kinds(), values);
                Effect::Changed
            }
            (Button::A, CellContent::Fighter(kind)) => {
                self.grid.focus_kind(kind);
                if self.grid.confirm(&kinds(), values) == Outcome::RosterChanged {
                    Effect::Changed
                } else {
                    Effect::Nothing
                }
            }
            (Button::Y, CellContent::Fighter(0x4b)) if self.use_ == RosterUse::Cycles => Effect::ToggleSolo,
            (Button::R, CellContent::Fighter(kind)) if !self.pick() => {
                self.grid.focus_kind(kind);
                if self.grid.remove(&kinds(), values) {
                    Effect::Changed
                } else {
                    Effect::Nothing
                }
            }
            _ => Effect::Nothing,
        }
    }

    /// Write the tiles' labels (name with the rank, the pick or the All mark) and the role on the second help line.
    pub fn decorate(&self, frame: &mut Frame, values: &[u8], solo: bool) {
        let kinds = kinds();
        frame.tile_roster_grid = true;
        frame.tile_cpu = self.grid.role() == Role::Cpu;
        frame.tile_sig = 0x8000_0000 | u32::from(frame.tile_cpu);
        for cell in 0..CELLS.min(frame.tiles.len()) {
            let Some(info) = self.grid.info(cell) else { continue };
            frame.tile_n[cell] = info.kind().and_then(|kind| u16::try_from(kind).ok()).unwrap_or(u16::MAX);
            frame.tile_sig = frame.tile_sig.wrapping_mul(16777619) ^ u32::from(frame.tile_n[cell]);
            let mut label = Text::<28>::new();
            match info.content {
                CellContent::All => {
                    let _ = write!(label, "{}All", if values.first() == Some(&1) { "* " } else { "" });
                }
                CellContent::Unsupported => {
                    let _ = write!(label, "({})", info.label);
                }
                CellContent::Fighter(kind) if self.pick() => {
                    let chosen = CHOICES.iter().position(|(k, _)| *k == kind).is_some_and(|i| values.get(i) == Some(&1));
                    let _ = write!(label, "{}{}", if chosen { "> " } else { "" }, info.label);
                }
                CellContent::Fighter(kind) => match group_rank(&kinds, values, kind) {
                    Some(rank) => {
                        let _ = write!(label, "{rank}. {}", info.label);
                    }
                    None => {
                        let _ = label.write_str(info.label);
                    }
                },
            }
            if solo && self.use_ == RosterUse::Cycles && info.kind() == Some(0x4b) {
                let _ = label.write_str(" S");
            }
            frame.tiles[cell] = label;
        }
        if !self.pick() {
            frame.help[1].clear();
            let _ = write!(frame.help[1], "Editing the {} list.", if self.grid.role() == Role::Player { "Player" } else { "CPU" });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell_of(kind: u32) -> usize {
        let grid = RosterGrid::new(Role::Player);
        (0..CELLS).find(|c| grid.info(*c).and_then(|i| i.kind()) == Some(kind)).expect("kind on the grid")
    }

    #[test]
    fn a_pick_is_one_hot_and_a_second_pick_moves_it() {
        let mut editor = RosterEditor::new(RosterUse::PlayerPick);
        let mut values = editor.values(None);
        assert_eq!(values.len(), CHOICES.len());
        assert_eq!(editor.press(cell_of(0x58), Button::A, &mut values), Effect::Changed); // Steve
        assert_eq!(values.iter().filter(|v| **v == 1).count(), 1);
        assert_eq!(editor.press(cell_of(0x58), Button::A, &mut values), Effect::Nothing);
        assert_eq!(editor.press(cell_of(0x00), Button::A, &mut values), Effect::Changed); // Mario
        assert_eq!(values[CHOICES.iter().position(|(k, _)| *k == 0x00).unwrap()], 1);
        assert_eq!(editor.key(), "player_character");
    }

    #[test]
    fn an_order_ranks_adds_removes_and_switches_role() {
        let mut editor = RosterEditor::new(RosterUse::Cycles);
        let mut values = editor.values(None);
        assert_eq!(values[0], 1, "factory: All");
        assert_eq!(editor.press(cell_of(0x58), Button::A, &mut values), Effect::Changed);
        assert_eq!(editor.press(cell_of(0x07), Button::A, &mut values), Effect::Changed); // Fox
        assert_eq!(values[0], 0);
        let kinds = kinds();
        assert_eq!(group_rank(&kinds, &values, 0x58), Some(1));
        assert_eq!(group_rank(&kinds, &values, 0x07), Some(2));
        assert_eq!(editor.press(cell_of(0x58), Button::R, &mut values), Effect::Changed);
        assert_eq!(group_rank(&kinds, &values, 0x07), Some(1));
        assert_eq!(editor.press(0, Button::ZR, &mut values), Effect::Shown);
        assert_eq!(editor.key(), "cpu_roster");
        let mut frame = Frame::new();
        editor.decorate(&mut frame, &values, false);
        assert!(frame.help[1].string_lossy().contains("CPU"));
    }

    #[test]
    fn solo_popo_changes_only_the_cycle_role_preference_and_marks_ice_climbers() {
        let mut editor = RosterEditor::new(RosterUse::Cycles);
        let mut values = editor.values(None);
        let before = values.clone();
        assert_eq!(editor.press(cell_of(0x4b), Button::Y, &mut values), Effect::ToggleSolo);
        assert_eq!(values, before, "Solo Popo does not remove a roster member");
        assert_eq!(editor.solo_key(), "solo_popo_player_cycle");
        let mut frame = Frame::new();
        editor.decorate(&mut frame, &values, true);
        assert!(frame.tiles[cell_of(0x4b)].string_lossy().ends_with(" S"));
        assert_eq!(editor.press(cell_of(0), Button::Y, &mut values), Effect::Nothing);
        editor.press(0, Button::ZR, &mut values);
        assert_eq!(editor.solo_key(), "solo_popo_cpu");
        for use_ in [RosterUse::PlayerPick, RosterUse::CpuPick, RosterUse::Hats] {
            let mut other = RosterEditor::new(use_);
            let mut values = other.values(None);
            assert_eq!(other.press(cell_of(0x4b), Button::Y, &mut values), Effect::Nothing);
        }
    }

    #[test]
    fn solo_popo_persists_independently_for_each_role() {
        use crate::model::{menu::{Env, Menu}, settings::Settings};
        let mut menu = Menu::new(Settings::factory(), Env::default());
        let mut editor = RosterEditor::new(RosterUse::Cycles);
        let cell = cell_of(0x4b);
        editor.apply(&mut menu, cell, Button::Y);
        assert!(editor.solo_enabled(&menu));
        editor.apply(&mut menu, cell, Button::ZR);
        assert!(!editor.solo_enabled(&menu));
        editor.apply(&mut menu, cell, Button::Y);
        assert!(editor.solo_enabled(&menu));
        editor.apply(&mut menu, cell, Button::ZR);
        editor.apply(&mut menu, cell, Button::Y);
        assert!(!editor.solo_enabled(&menu));
        assert_eq!(menu.kept("solo_popo_cpu"), Some(&serde_json::json!([1, 0])));
        assert_eq!(menu.settings().menu.get("solo_popo_original"), Some(&[0, 1][..]));
    }

    #[test]
    fn values_round_trip_through_the_kept_json_and_bad_json_is_the_factory() {
        let editor = RosterEditor::new(RosterUse::Hats);
        let mut values = editor.values(None);
        values[0] = 0;
        values[3] = 1;
        let json = RosterEditor::value_of(&values);
        assert_eq!(editor.values(Some(&json)), values);
        assert_eq!(editor.values(Some(&Value::from("x")))[0], 1);
    }
}
