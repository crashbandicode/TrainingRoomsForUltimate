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

use crate::engine::swap::roster_grid::{
    catalog_index, css_order, cycle_exclusion, group_kind, group_rank, rank, select_all, CellContent, Outcome, Role, RosterGrid, CELLS,
};
use crate::engine::swap::trio::CHOICES;
use crate::model::frame::{Frame, Text};
use crate::model::menu::grid::RosterUse;
use crate::model::menu::input::Button;
use crate::model::menu::Menu;

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
        flag_on(menu, self.solo_key())
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

    /// Write the tiles' labels (the fighter name only; the pick's `> ` chosen marker stays) and the role
    /// on the second help line. The fork's corner badges and Solo Popo markers are separate frame fields
    /// (`decorate_badges`), drawn by the renderer at the cells' corners.
    pub fn decorate(&self, frame: &mut Frame, values: &[u8]) {
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
                    let _ = label.write_str("All");
                }
                CellContent::Unsupported => {
                    let _ = label.write_str(info.label);
                }
                CellContent::Fighter(kind) if self.pick() => {
                    let chosen = CHOICES.iter().position(|(k, _)| *k == kind).is_some_and(|i| values.get(i) == Some(&1));
                    let _ = write!(label, "{}{}", if chosen { "> " } else { "" }, info.label);
                }
                CellContent::Fighter(_) => {
                    let _ = label.write_str(info.label);
                }
            }
            frame.tiles[cell] = label;
        }
        if !self.pick() {
            frame.help[1].clear();
            let _ = write!(frame.help[1], "Editing the {} list.", if self.grid.role() == Role::Player { "Player" } else { "CPU" });
        }
    }

    /// Fill the fork's per-role corner badges (`roster_grid.rs:227-251`: the order number; "All" on the All cell),
    /// All-stripes (`:213-225`: the cell is in that role's All order) and Solo Popo "S" flags (`:253-271`, cycles
    /// only) from the menu's kept keys, for both roles at once. Picks keep none.
    pub fn decorate_badges(&self, frame: &mut Frame, menu: &Menu) {
        let (kinds, hats) = (kinds(), self.use_ == RosterUse::Hats);
        let original_solo = flag_on(menu, "solo_popo_original");
        if self.pick() {
            return;
        }
        for (r, role) in [Role::Player, Role::Cpu].into_iter().enumerate() {
            let pick = Self::new([RosterUse::PlayerPick, RosterUse::CpuPick][r]);
            let mut editor = *self;
            editor.grid = RosterGrid::new(role);
            let values = editor.values(menu.kept(editor.key()));
            let picked = pick.values(menu.kept(pick.key()));
            let original = picked.iter().position(|v| *v == 1).map(|i| kinds[i]);
            let solo = flag_on(menu, ["solo_popo_player_cycle", "solo_popo_cpu"][r]);
            // The fork's `role_cycle_exclusion` (app.rs:690-703): the CPU has one Solo setting for both.
            let excluded = cycle_exclusion(original, if r == 0 { original_solo } else { solo }, solo);
            let badges = roster_badges_from_values(self.use_, &values, excluded);
            let all = values.first() == Some(&1);
            for (cell, info) in (0..CELLS).filter_map(|c| Some((c, self.grid.info(c)?))) {
                let badge = info.kind().and_then(|k| badges[catalog_index(&kinds, k)?]);
                let text = &mut frame.tile_badges[r][cell];
                text.clear();
                if all && info.content == CellContent::All {
                    let _ = text.write_str("All");
                } else if let Some(n) = badge {
                    let _ = write!(text, "{n}");
                }
                frame.tile_stripes[r][cell] = all && badge.is_some_and(|n| n > 0);
            }
            frame.tile_solo[r] = !hats && solo;
        }
    }
}

/// A kept `[on, off]` toggle reads On.
fn flag_on(menu: &Menu, key: &str) -> bool {
    menu.kept(key) == Some(&serde_json::json!([1, 0]))
}

/// Cells that never take an order badge: the Miis (this build's cycle engine drops them from every roster,
/// `src/runtime/roster/character_roster.rs:513`; no Mii helper hats) and, for the hats, Kirby (app.rs:564-573).
fn unbadged(use_: RosterUse, kind: u32) -> bool {
    (0x48..=0x4a).contains(&kind) || (use_ == RosterUse::Hats && group_kind(kind) == 0x06)
}

/// The fork's corner-badge model (`training_mod_tui/src/containers/app.rs:745-799`) on TrainingRooms' data: one
/// role's order number per catalog kind; `Some(0)` the slot-0 badge; `None` no badge. `excluded` is the role's
/// cycle exclusion (`cycle_exclusion`, the fork's `role_cycle_exclusion`); its group takes the slot-0 badge in
/// the cycles (`roster_slot_zero`, app.rs:621-643, without the Mii-catalog features this build lacks).
pub fn roster_badges_from_values(use_: RosterUse, values: &[u8], excluded: Option<u32>) -> Vec<Option<u8>> {
    let kinds = kinds();
    let slot_zero = |kind: u32| use_ == RosterUse::Cycles && excluded == Some(group_kind(kind));
    let all = values.first() == Some(&1);
    let skip = (use_ == RosterUse::Hats).then_some(0x06).or(excluded);
    let mut order: Vec<u32> = if all {
        css_order(&kinds, values, skip)
    } else if use_ == RosterUse::Cycles {
        // Stored ranks retain choices across scenes; number the order after this role's excluded cells
        // drop out, as the cycle does (app.rs:759-768).
        let mut ranked: Vec<_> = kinds.iter().enumerate().filter_map(|(i, kind)| rank(values, i).map(|r| (r, *kind))).collect();
        ranked.sort_unstable_by_key(|(r, _)| *r);
        ranked.into_iter().map(|(_, kind)| kind).collect()
    } else {
        Vec::new()
    };
    order.retain(|kind| !unbadged(use_, *kind) && !slot_zero(*kind));
    let groups: Vec<u32> = order.into_iter().map(group_kind).collect();
    let number = |kind| groups.iter().position(|g| *g == group_kind(kind));
    let badge = |kind| match () {
        _ if slot_zero(kind) => Some(0),
        _ if unbadged(use_, kind) => None,
        _ if all || use_ == RosterUse::Cycles => number(kind).map(|i| i as u8 + 1),
        _ => group_rank(&kinds, values, kind),
    };
    kinds.iter().map(|&kind| badge(kind)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell_of(kind: u32) -> usize {
        let grid = RosterGrid::new(Role::Player);
        (0..CELLS).find(|c| grid.info(*c).and_then(|i| i.kind()) == Some(kind)).expect("kind on the grid")
    }

    fn grid_info(editor: &RosterEditor, cell: usize) -> Option<CellContent> {
        editor.grid.info(cell).map(|i| i.content)
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
        editor.decorate(&mut frame, &values);
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
        editor.decorate(&mut frame, &values);
        assert!(!frame.tiles[cell_of(0x4b)].string_lossy().contains('S'), "the S is a corner marker, not a name suffix");
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

    fn cycles_values(picks: &[u32]) -> Vec<u8> {
        // `[all, ranks...]` over the catalog: rank 1.. for the picked kinds in order.
        let mut values = vec![0u8; CHOICES.len() + 1];
        values[0] = 0;
        for (rank, kind) in picks.iter().enumerate() {
            let index = CHOICES.iter().position(|(k, _)| *k == *kind).expect("picked kind in catalog");
            values[index + 1] = rank as u8 + 1;
        }
        values
    }

    fn badge_of(badges: &[Option<u8>], kind: u32) -> Option<u8> {
        badges[CHOICES.iter().position(|(k, _)| *k == kind).expect("kind in catalog")]
    }

    #[test]
    fn cycles_badges_number_each_role_independently() {
        let player = cycles_values(&[0x58, 0x07]); // Steve 1, Fox 2
        let cpu = cycles_values(&[0x42]);          // Ridley 1
        let player_badges = roster_badges_from_values(RosterUse::Cycles, &player, Some(0x00));
        let cpu_badges = roster_badges_from_values(RosterUse::Cycles, &cpu, Some(0x0b));
        // Both roles' numbers exist at once, on their own cells.
        assert_eq!(badge_of(&player_badges, 0x58), Some(1));
        assert_eq!(badge_of(&player_badges, 0x07), Some(2));
        assert_eq!(badge_of(&player_badges, 0x42), None, "the player's order does not number the CPU's pick");
        assert_eq!(badge_of(&cpu_badges, 0x42), Some(1));
        assert_eq!(badge_of(&cpu_badges, 0x58), None);
        // The Miis take no badge in this build's cycles (the engine's roster filter drops them).
        assert_eq!(badge_of(&player_badges, 0x49), None);
    }

    #[test]
    fn all_orders_number_every_member_and_the_all_cell_badges_all() {
        let mut values = vec![0u8; CHOICES.len() + 1];
        values[0] = 1; // All
        let badges = roster_badges_from_values(RosterUse::Cycles, &values, Some(0x00));
        assert_eq!(badge_of(&badges, 0x00), Some(0), "the excluded original carries the slot-0 badge");
        assert_eq!(badge_of(&badges, 0x01), Some(1), "the css order starts at the first non-excluded member");
        assert_eq!(badge_of(&badges, 0x07), Some(7), "the css order is the grid's: DK 1, Link 2, Samus 3, Dark Samus 4, Yoshi 5, Kirby 6, Fox 7");
        // The All cell's badge carries the text "All" while the order is All (decorate_badges).
        let mut frame = Frame::new();
        let editor = RosterEditor::new(RosterUse::Cycles);
        editor.decorate(&mut frame, &values);
        editor.decorate_badges(&mut frame, &menu_with_kept(&[("player_roster", &values)]));
        let all_cell = (0..CELLS).find(|c| matches!(grid_info(&editor, *c), Some(CellContent::All))).expect("the All cell");
        assert_eq!(frame.tile_badges[0][all_cell].string_lossy(), "All");
        assert_eq!(frame.tile_badges[1][all_cell].string_lossy(), "All", "the CPU's missing order is the factory All");
        // The All-stripes follow the role's All order: every numbered member, not the slot-0 original.
        assert!(frame.tile_stripes[0][cell_of(0x07)]);
        assert!(!frame.tile_stripes[0][all_cell]);
    }

    #[test]
    fn the_slot_zero_badge_greys_the_roles_original_group() {
        // The player's original is Steve; with Steve's Solo Popo off, the cycle exclusion excludes
        // Steve's group and the badge reads 0.
        let player = cycles_values(&[0x07]);
        let badges = roster_badges_from_values(RosterUse::Cycles, &player, cycle_exclusion(Some(0x58), false, false));
        assert_eq!(badge_of(&badges, 0x58), Some(0));
        assert_eq!(badge_of(&badges, 0x07), Some(1), "the exclusion does not renumber the others");
        // The solo flags only spare the Ice Climbers: an IC original with the original Solo Popo on
        // and the cycle solo off stays a cycle member (no 0 badge), the reverse excludes it.
        let ic = cycles_values(&[0x4b]);
        let badges = roster_badges_from_values(RosterUse::Cycles, &ic, cycle_exclusion(Some(0x4b), true, false));
        assert_eq!(badge_of(&badges, 0x4b), Some(1));
        let badges = roster_badges_from_values(RosterUse::Cycles, &ic, cycle_exclusion(Some(0x4b), false, false));
        assert_eq!(badge_of(&badges, 0x4b), Some(0));
    }

    #[test]
    fn hats_badge_the_hat_order_and_hide_the_solo_markers() {
        let hats = cycles_values(&[0x0b, 0x07]); // Captain Falcon 1, Fox 2
        let badges = roster_badges_from_values(RosterUse::Hats, &hats, Some(0x00));
        assert_eq!(badge_of(&badges, 0x0b), Some(1));
        assert_eq!(badge_of(&badges, 0x07), Some(2));
        assert_eq!(badge_of(&badges, 0x06), None, "Kirby cannot be a hat");
        assert_eq!(badge_of(&badges, 0x49), None, "no Mii helper hats in this build");
        // The Solo Popo markers are cycles-only: the hats view hides them (decorate_badges).
        let mut frame = Frame::new();
        let editor = RosterEditor::new(RosterUse::Hats);
        editor.decorate(&mut frame, &hats);
        let cycles = cycles_values(&[0x58]); // the cycle order must not leak into the hats view
        let kept = [("player_kirby_hats", &hats[..]), ("player_roster", &cycles[..]), ("solo_popo_player_cycle", &[1u8, 0][..])];
        editor.decorate_badges(&mut frame, &menu_with_kept(&kept));
        assert!(!frame.tile_solo[0]);
        assert_eq!(frame.tile_badges[0][cell_of(0x0b)].string_lossy(), "1", "the hats view badges the hat order");
        assert_eq!(frame.tile_badges[0][cell_of(0x07)].string_lossy(), "2");
        assert!(frame.tile_badges[0][cell_of(0x58)].is_empty(), "not the cycle order's Steve");
    }

    #[test]
    fn picks_keep_no_badges_and_the_name_labels_carry_the_name_only() {
        let editor = RosterEditor::new(RosterUse::PlayerPick);
        let mut values = editor.values(None);
        values[CHOICES.iter().position(|(k, _)| *k == 0x58).unwrap()] = 1;
        let mut frame = Frame::new();
        editor.decorate(&mut frame, &values);
        assert!(frame.tiles[cell_of(0x58)].string_lossy().contains("Steve"), "the pick's chosen marker stays");
        assert!(!frame.tiles[cell_of(0x58)].string_lossy().contains("1."), "no rank number in the pick's name label");
        editor.decorate_badges(&mut frame, &menu_with_kept(&[]));
        assert!(frame.tile_badges.iter().flatten().all(|b| b.is_empty()), "a pick grid keeps no badges");
    }

    #[test]
    fn cycles_name_labels_show_the_fighter_name_only() {
        let editor = RosterEditor::new(RosterUse::Cycles);
        let values = cycles_values(&[0x58]);
        let mut frame = Frame::new();
        editor.decorate(&mut frame, &values);
        let label = frame.tiles[cell_of(0x58)].string_lossy();
        assert_eq!(label, "Steve", "the order number moved to the corner badge");
        editor.decorate_badges(&mut frame, &menu_with_kept(&[("player_roster", &values), ("solo_popo_player_cycle", &[1u8, 0])]));
        assert!(!frame.tiles[cell_of(0x4b)].string_lossy().contains('S'), "the S is a corner marker now");
        assert!(frame.tile_solo[0] && !frame.tile_solo[1], "each role's own Solo Popo preference");
    }

    /// A minimal menu stub for decorate_badges: the kept keys as JSON arrays.
    fn menu_with_kept(kept: &[(&'static str, &[u8])]) -> crate::model::menu::Menu {
        use crate::model::{menu::{Env, Menu}, settings::Settings};
        let mut menu = Menu::new(Settings::factory(), Env::default());
        for (key, values) in kept {
            menu.set_kept(key, serde_json::json!(values.to_vec()));
        }
        menu
    }
}
