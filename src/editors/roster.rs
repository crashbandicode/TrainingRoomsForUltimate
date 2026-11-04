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

use crate::engine::swap::roster_grid::{css_order, cycle_exclusion, group_kind, group_rank, rank, select_all, CellContent, Outcome, Role, RosterGrid, CELLS};
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

    /// Write the tiles' labels (the fighter name only; the pick's `> ` chosen marker stays) and the role
    /// on the second help line. The fork's corner badges and Solo Popo markers are separate frame fields
    /// (`decorate_badges`), drawn by the renderer at the cells' corners.
    pub fn decorate(&self, frame: &mut Frame, values: &[u8], solo: bool) {
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
        let _ = solo; // the Solo Popo markers are corner panes now (decorate_badges), not a name suffix
    }

    /// Fill the fork's corner badges (the order number per cell per role, both roles at once) and the
    /// Solo Popo "S" marker flags from the menu's kept keys. `values` is the active role's.
    pub fn decorate_badges(&self, frame: &mut Frame, menu: &crate::model::menu::Menu, values: &[u8]) {
        let original_of = |key: &str| -> Option<u32> {
            menu.kept(key).and_then(|v| v.as_array()).and_then(|a| a.iter().position(|v| v.as_u64() == Some(1)))
                .and_then(|i| CHOICES.get(i).map(|(kind, _)| *kind))
        };
        let solo_of = |key: &str| -> bool {
            menu.kept(key).and_then(Value::as_array).is_some_and(|v| v.len() == 2 && v[0].as_u64() == Some(1) && v[1].as_u64() == Some(0))
        };
        let (player_values, cpu_values) = match self.key() {
            "player_roster" => (values.to_vec(), self.values(menu.kept("cpu_roster"))),
            "cpu_roster" => (self.values(menu.kept("player_roster")), values.to_vec()),
            _ => (self.values(menu.kept("player_roster")), self.values(menu.kept("cpu_roster"))),
        };
        let (player_original, cpu_original) = (original_of("player_character"), original_of("cpu_character"));
        let original_solo = solo_of("solo_popo_original");
        let (player_solo, cpu_solo) = (solo_of("solo_popo_player_cycle"), solo_of("solo_popo_cpu"));
        let player_badges = roster_badges_from_values(self.use_, Role::Player, &player_values, player_original, original_solo, player_solo);
        let cpu_badges = roster_badges_from_values(self.use_, Role::Cpu, &cpu_values, cpu_original, original_solo, cpu_solo);
        for cell in 0..CELLS.min(frame.tiles.len()) {
            let Some(info) = self.grid.info(cell) else { continue };
            let CellContent::Fighter(kind) = info.content else {
                // The All cell: the fork's badge carries the text "All" while that role's order is All
                // (`roster_grid.rs:232-237`); the unsupported cells never badge.
                let all_p = info.content == CellContent::All && player_values.first() == Some(&1);
                let all_c = info.content == CellContent::All && cpu_values.first() == Some(&1);
                frame.tile_badges_p[cell] = text_of(if all_p { "All" } else { "" });
                frame.tile_badges_c[cell] = text_of(if all_c { "All" } else { "" });
                continue;
            };
            let Some(index) = CHOICES.iter().position(|(k, _)| *k == kind) else { continue };
            frame.tile_badges_p[cell] = number_text(player_badges[index]);
            frame.tile_badges_c[cell] = number_text(cpu_badges[index]);
        }
        // The Solo Popo "S" markers: the cycles view only, per role's own preference (the fork's
        // `!hats && !manual && roster_solo_enabled(role)`, `roster_grid.rs:253-271`).
        let cycles = self.use_ == RosterUse::Cycles;
        frame.tile_solo_p = cycles && player_solo;
        frame.tile_solo_c = cycles && cpu_solo;
    }
}

/// The fork's corner-badge model, ported (`training_mod_tui` `App::roster_badges_from_values`,
/// `training_mod_tui/src/containers/app.rs:745-799`) to TrainingRooms' data: the order number per
/// catalog kind per role; `Some(0)` = the slot-0 badge; `None` = no badge. `original` is the role's
/// captured character-select fighter (the role's pick), `original_solo`/`cycle_solo` the Solo Popo
/// preferences the cycle exclusion reads.
pub fn roster_badges_from_values(use_: RosterUse, role: Role, values: &[u8],
    original: Option<u32>, original_solo: bool, cycle_solo: bool) -> Vec<Option<u8>> {
    let kinds = kinds();
    if matches!(use_, RosterUse::PlayerPick | RosterUse::CpuPick) {
        return vec![None; kinds.len()];
    }
    let hat_cell_disabled = |kind: u32| {
        // Kirby cannot be a hat; this build has no Mii helper hats (the fork's app.rs:564-573).
        use_ == RosterUse::Hats && (group_kind(kind) == 0x06 || (0x48..=0x4a).contains(&kind))
    };
    // This build's cycle engine excludes the Miis from every cycle roster
    // (`src/runtime/roster/character_roster.rs:513`), so they take no badge and no order slot.
    let not_a_cycle_member = |kind: u32| (0x48..=0x4a).contains(&kind);
    // The fork's `roster_slot_zero` (app.rs:621-643) without its Mii-catalog branch: a Mii original
    // has no numbered alternative here.
    let slot_zero = |kind: u32| {
        if use_ != RosterUse::Cycles {
            return false;
        }
        let Some(original) = original else { return false };
        let group = group_kind(original);
        if group != group_kind(kind) || (0x48..=0x4a).contains(&group) {
            return false;
        }
        cycle_exclusion(Some(original), original_solo, cycle_solo) == Some(group_kind(kind))
    };
    // The fork's `role_cycle_exclusion` (app.rs:690-703): the Player's exclusion reads its own
    // original Solo Popo preference; the CPU's original solo is its cycle solo.
    let excluded = match use_ {
        RosterUse::Cycles => cycle_exclusion(original, match role {
            Role::Player => original_solo,
            Role::Cpu => cycle_solo,
        }, cycle_solo),
        RosterUse::Hats => Some(0x06),
        _ => None,
    };
    let all = values.first() == Some(&1);
    let mut order: Vec<u32> = if all {
        css_order(&kinds, values, excluded)
    } else if use_ == RosterUse::Cycles {
        // Stored ranks retain choices across scenes; display the numbered order after this role's
        // excluded cells drop out, just as the cycle does (the fork's app.rs:759-768).
        let mut ranked: Vec<_> = kinds.iter().copied().enumerate()
            .filter_map(|(index, kind)| rank(values, index).map(|rank| (rank, kind)))
            .collect();
        ranked.sort_unstable_by_key(|(rank, _)| *rank);
        ranked.into_iter().map(|(_, kind)| kind).collect()
    } else {
        Vec::new()
    };
    order.retain(|kind| !hat_cell_disabled(*kind) && !not_a_cycle_member(*kind) && !slot_zero(*kind));
    kinds.iter().copied()
        .map(|kind| {
            if hat_cell_disabled(kind) {
                None
            } else if slot_zero(kind) {
                Some(0)
            } else if not_a_cycle_member(kind) {
                None
            } else if all || use_ == RosterUse::Cycles {
                order.iter().position(|k| group_kind(*k) == group_kind(kind)).map(|i| i as u8 + 1)
            } else {
                group_rank(&kinds, values, kind)
            }
        })
        .collect()
}

fn number_text(badge: Option<u8>) -> Text<4> {
    text_of(badge.map(|n| n.to_string()).unwrap_or_default().as_str())
}

fn text_of(text: &str) -> Text<4> {
    let mut out = Text::<4>::new();
    for unit in text.encode_utf16() {
        out.push_unit(unit);
    }
    out
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
        let player_badges = roster_badges_from_values(RosterUse::Cycles, Role::Player, &player, Some(0x00), false, false);
        let cpu_badges = roster_badges_from_values(RosterUse::Cycles, Role::Cpu, &cpu, Some(0x0b), false, false);
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
        let badges = roster_badges_from_values(RosterUse::Cycles, Role::Player, &values, Some(0x00), false, false);
        assert_eq!(badge_of(&badges, 0x00), Some(0), "the excluded original carries the slot-0 badge");
        assert_eq!(badge_of(&badges, 0x01), Some(1), "the css order starts at the first non-excluded member");
        assert_eq!(badge_of(&badges, 0x07), Some(7), "the css order is the grid's: DK 1, Link 2, Samus 3, Dark Samus 4, Yoshi 5, Kirby 6, Fox 7");
        // The All cell's badge carries the text "All" while the order is All (decorate_badges).
        let mut frame = Frame::new();
        let editor = RosterEditor::new(RosterUse::Cycles);
        editor.decorate(&mut frame, &values, false);
        editor.decorate_badges(&mut frame, &menu_with_kept(&[("player_roster", &values)]), &values);
        assert_eq!(frame.tile_badges_p[(0..CELLS).find(|c| matches!(grid_info(&editor, *c), Some(CellContent::All))).expect("the All cell")].string_lossy(), "All");
    }

    #[test]
    fn the_slot_zero_badge_greys_the_roles_original_group() {
        // The player's original is Steve; with Steve's Solo Popo off, the cycle exclusion excludes
        // Steve's group and the badge reads 0.
        let player = cycles_values(&[0x07]);
        let badges = roster_badges_from_values(RosterUse::Cycles, Role::Player, &player, Some(0x58), false, false);
        assert_eq!(badge_of(&badges, 0x58), Some(0));
        assert_eq!(badge_of(&badges, 0x07), Some(1), "the exclusion does not renumber the others");
        // The solo flags only spare the Ice Climbers: an IC original with the original Solo Popo on
        // and the cycle solo off stays a cycle member (no 0 badge), the reverse excludes it.
        let ic = cycles_values(&[0x4b]);
        let badges = roster_badges_from_values(RosterUse::Cycles, Role::Player, &ic, Some(0x4b), true, false);
        assert_eq!(badge_of(&badges, 0x4b), Some(1));
        let badges = roster_badges_from_values(RosterUse::Cycles, Role::Player, &ic, Some(0x4b), false, false);
        assert_eq!(badge_of(&badges, 0x4b), Some(0));
    }

    #[test]
    fn hats_badge_the_hat_order_and_hide_the_solo_markers() {
        let hats = cycles_values(&[0x0b, 0x07]); // Captain Falcon 1, Fox 2
        let badges = roster_badges_from_values(RosterUse::Hats, Role::Player, &hats, Some(0x00), false, false);
        assert_eq!(badge_of(&badges, 0x0b), Some(1));
        assert_eq!(badge_of(&badges, 0x07), Some(2));
        assert_eq!(badge_of(&badges, 0x06), None, "Kirby cannot be a hat");
        assert_eq!(badge_of(&badges, 0x49), None, "no Mii helper hats in this build");
        // The Solo Popo markers are cycles-only: the hats view hides them (decorate_badges).
        let mut frame = Frame::new();
        let editor = RosterEditor::new(RosterUse::Hats);
        editor.decorate(&mut frame, &hats, false);
        editor.decorate_badges(&mut frame, &menu_with_kept(&[("player_kirby_hats", &hats), ("solo_popo_player_cycle", &[1u8, 0])]), &hats);
        assert!(!frame.tile_solo_p);
    }

    #[test]
    fn picks_keep_no_badges_and_the_name_labels_carry_the_name_only() {
        let editor = RosterEditor::new(RosterUse::PlayerPick);
        let mut values = editor.values(None);
        values[CHOICES.iter().position(|(k, _)| *k == 0x58).unwrap()] = 1;
        let mut frame = Frame::new();
        editor.decorate(&mut frame, &values, false);
        assert!(frame.tiles[cell_of(0x58)].string_lossy().contains("Steve"), "the pick's chosen marker stays");
        assert!(!frame.tiles[cell_of(0x58)].string_lossy().contains("1."), "no rank number in the pick's name label");
        let badges = roster_badges_from_values(RosterUse::PlayerPick, Role::Player, &values, Some(0x58), false, false);
        assert!(badges.iter().all(|b| b.is_none()));
    }

    #[test]
    fn cycles_name_labels_show_the_fighter_name_only() {
        let editor = RosterEditor::new(RosterUse::Cycles);
        let values = cycles_values(&[0x58]);
        let mut frame = Frame::new();
        editor.decorate(&mut frame, &values, false);
        let label = frame.tiles[cell_of(0x58)].string_lossy();
        assert_eq!(label, "Steve", "the order number moved to the corner badge");
        let solo_frame = {
            let mut f = Frame::new();
            editor.decorate(&mut f, &values, true);
            f
        };
        assert!(!solo_frame.tiles[cell_of(0x4b)].string_lossy().contains('S'), "the S is a corner marker now");
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
