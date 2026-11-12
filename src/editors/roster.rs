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

use crate::engine::swap::cycle_members::{cycle_members, number_of, numbered, roster_of};
use crate::engine::swap::roster_grid::{
    catalog_index, css_order, cycle_exclusion, group_kind, group_rank, select_all, CellContent, Outcome, Role, RosterGrid, CELLS,
};
use crate::engine::swap::trio::CHOICES;
use crate::model::frame::{Frame, Text};
use crate::model::screens::TILE_GREYED;
use crate::model::menu::grid::RosterUse;
use crate::model::menu::input::Button;
use crate::model::menu::Menu;

/// What a cycle grid numbers for one role (P-CYCLE, the rows' evidence): which catalog kinds carry a number (a mask over
/// `trio::CHOICES` indices), the highest number, and the slot-0 kind the 0 names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Numbers {
    pub mask: [u64; 3],
    pub count: u8,
    pub slot_zero: Option<u32>,
}

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
    mii: MiiTiles,
}

/// What this opening admits per Mii class tile (`[Brawler, Swordfighter, Gunner]`) and where each role's cycle returns
/// to. The picks and the hats come from `CatalogMenu`'s checks (`editors::mii::MiiList::tiles`): the fork's
/// `roster_excluded`, `mii_cycle_admitted`, `hat_cell_disabled` and `roster_slot_zero`
/// (`training_mod_tui/src/containers/app.rs` 563-685). Each cycle's Mii admission and both slot 0s are the runtime's
/// own, published (`character_roster::cycle_facts`), so the grid numbers exactly what cycles (P-CYCLE, P1-CYCLE). The
/// default admits no Mii tile and knows no slot 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MiiTiles {
    /// `alpha_player_cycle`: Player picks (a manual pick, not the cycle).
    pub player: [bool; 3],
    /// The Player cycle's admission as the Player Mii owner walks it (`cycle_members::player_admission`, published).
    pub player_cycle: [bool; 3],
    /// `alpha_cpu_cycle`: CPU picks (a manual pick, not the cycle).
    pub cpu: [bool; 3],
    /// The CPU cycle's admission as the CPU Mii owner walks it (`cycle_members::cpu_admission`, published).
    pub cpu_cycle: [bool; 3],
    /// `alpha_helper_hat`: Kirby hats.
    pub hat: [bool; 3],
    /// Each role's slot 0 as the runtime returns to it (`[Player, CPU]`: the scene's original kind), when a practice
    /// scene is up. The saved pick stands in otherwise.
    pub slot_zero: [Option<u32>; 2],
}

fn mii_class(kind: u32) -> Option<usize> {
    (0x48..=0x4a).contains(&kind).then(|| (kind - 0x48) as usize)
}

impl MiiTiles {
    /// Whether a Mii class tile may be chosen for this use and role; `None` for every other tile.
    pub fn admitted(&self, use_: RosterUse, role: Role, kind: u32) -> Option<bool> {
        let k = mii_class(kind)?;
        Some(match (use_, role) {
            (RosterUse::Hats, _) => self.hat[k],
            (RosterUse::PlayerPick, _) => self.player[k],
            (RosterUse::Cycles, Role::Player) => self.player_cycle[k],
            (RosterUse::CpuPick, _) => self.cpu[k],
            (RosterUse::Cycles, Role::Cpu) => self.cpu_cycle[k],
        })
    }

    /// The fork's `roster_slot_zero` for a Mii original: a class the role's cycle does not number (for the CPU, its
    /// captured Mii has no other configuration of the class). Any other original keeps its slot 0.
    fn original_slot_zero(&self, role: Role, kind: u32) -> bool {
        let Some(k) = mii_class(kind) else { return true };
        match role {
            Role::Cpu => !self.cpu_cycle[k],
            Role::Player => !self.player_cycle[k],
        }
    }
}

fn kinds() -> Vec<u32> {
    CHOICES.iter().map(|(kind, _)| *kind).collect()
}

impl RosterEditor {
    pub fn new(use_: RosterUse) -> Self {
        let role = if use_ == RosterUse::CpuPick { Role::Cpu } else { Role::Player };
        Self { grid: RosterGrid::new(role), use_, mii: MiiTiles::default() }
    }

    /// This opening's Mii admissions (the worker sets them from the mounted catalog before a press or a draw).
    pub fn set_mii(&mut self, mii: MiiTiles) {
        self.mii = mii;
    }

    /// A Mii class tile the catalog does not admit for the editing role: it cannot be chosen and draws greyed.
    fn mii_blocked(&self, kind: u32) -> bool {
        self.mii.admitted(self.use_, self.grid.role(), kind) == Some(false)
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
            (Button::A, CellContent::Fighter(kind)) if self.mii_blocked(kind) => Effect::Nothing,
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
            if info.kind().is_some_and(|kind| self.mii_blocked(kind)) && frame.tile_state[cell] == 0 {
                frame.tile_state[cell] = TILE_GREYED;
            }
        }
        if !self.pick() {
            frame.help[1].clear();
            let _ = write!(frame.help[1], "Editing the {} list.", if self.grid.role() == Role::Player { "Player" } else { "CPU" });
        }
        // The fork's " - unavailable" (`training/ui/roster_grid.rs` 304-308) for the focused Mii tile.
        let focus = usize::from(frame.tile_focus);
        if let Some(info) = self.grid.info(focus).filter(|info| info.kind().is_some_and(|kind| self.mii_blocked(kind))) {
            frame.help[1].clear();
            let _ = write!(frame.help[1], "{} - unavailable", info.label); // restriction: mii-class-unavailable
        }
    }

    /// Fill the fork's per-role corner badges (`roster_grid.rs:227-251`: the order number; "All" on the All cell),
    /// All-stripes (`:213-225`: the cell is in that role's All order) and Solo Popo "S" flags (`:253-271`, cycles
    /// only) from the menu's kept keys, for both roles at once. Picks keep none.
    pub fn decorate_badges(&self, frame: &mut Frame, menu: &Menu) -> [Numbers; 2] {
        let (kinds, hats) = (kinds(), self.use_ == RosterUse::Hats);
        let original_solo = flag_on(menu, "solo_popo_original");
        let mut numbers = [Numbers::default(); 2];
        if self.pick() {
            return numbers;
        }
        for (r, role) in [Role::Player, Role::Cpu].into_iter().enumerate() {
            let pick = Self::new([RosterUse::PlayerPick, RosterUse::CpuPick][r]);
            let mut editor = *self;
            editor.grid = RosterGrid::new(role);
            let values = editor.values(menu.kept(editor.key()));
            let picked = pick.values(menu.kept(pick.key()));
            // The runtime's slot 0 (the scene's original), not the saved pick the runtime overwrites with the fighter it
            // loaded: the 0 names the fighter the cycle returns to.
            let original = self.mii.slot_zero[r].or_else(|| picked.iter().position(|v| *v == 1).map(|i| kinds[i]));
            let solo = flag_on(menu, ["solo_popo_player_cycle", "solo_popo_cpu"][r]);
            // The fork's `role_cycle_exclusion` (app.rs:690-703): the CPU has one Solo setting for both.
            let excluded = cycle_exclusion(original, if r == 0 { original_solo } else { solo }, solo);
            let badges = roster_badges(self.use_, &values, excluded, role, &self.mii);
            numbers[r].slot_zero = original;
            for (i, badge) in badges.iter().enumerate() {
                if let Some(n) = badge.filter(|n| *n > 0) {
                    numbers[r].mask[i / 64] |= 1 << (i % 64);
                    numbers[r].count = numbers[r].count.max(n);
                }
            }
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
        numbers
    }
}

/// A kept `[on, off]` toggle reads On.
fn flag_on(menu: &Menu, key: &str) -> bool {
    menu.kept(key) == Some(&serde_json::json!([1, 0]))
}

/// `roster_badges` with no Mii admitted (the host tests' form for the non-Mii rules).
#[cfg(test)]
pub fn roster_badges_from_values(use_: RosterUse, values: &[u8], excluded: Option<u32>) -> Vec<Option<u8>> {
    roster_badges(use_, values, excluded, Role::Player, &MiiTiles::default())
}

/// The fork's corner-badge model (`training_mod_tui/src/containers/app.rs:745-799`) on TrainingRooms' data: one
/// role's order number per catalog kind; `Some(0)` the slot-0 badge; `None` no badge. `excluded` is the role's
/// cycle exclusion (`cycle_exclusion`, the fork's `role_cycle_exclusion`); its group takes the slot-0 badge in
/// the cycles (`roster_slot_zero`, app.rs:621-643; a Mii original only by `MiiTiles::original_slot_zero`). Kirby
/// takes no hat badge (app.rs:564-573), and nor does a Mii tile the catalog does not admit for `role` (app.rs:785-789).
pub fn roster_badges(use_: RosterUse, values: &[u8], excluded: Option<u32>, role: Role, mii: &MiiTiles) -> Vec<Option<u8>> {
    let kinds = kinds();
    let slot_zero = |kind: u32| use_ == RosterUse::Cycles && excluded == Some(group_kind(kind))
        && mii.original_slot_zero(role, kind);
    let unbadged = |kind: u32| (use_ == RosterUse::Hats && group_kind(kind) == 0x06)
        || mii.admitted(use_, role, kind) == Some(false);
    let all = values.first() == Some(&1);
    if use_ == RosterUse::Cycles {
        // The cycle's own list (P-CYCLE): `cycle_members` with this role's Mii admission, numbered as the walkers step
        // (`cycle_members::numbered`), so a number is exactly a position the cycle loads and nothing else cycles.
        let admitted = [0x48, 0x49, 0x4a].map(|kind| mii.admitted(use_, role, kind) == Some(true));
        let members = cycle_members(roster_of(values), admitted);
        let order = numbered(&members, excluded);
        let badge = |kind| match () {
            _ if slot_zero(kind) => Some(0),
            _ if unbadged(kind) => None,
            _ => number_of(&order, kind).and_then(|n| u8::try_from(n).ok()),
        };
        return kinds.iter().map(|&kind| badge(kind)).collect();
    }
    let skip = (use_ == RosterUse::Hats).then_some(0x06).or(excluded.filter(|kind| slot_zero(*kind)));
    let mut order: Vec<u32> = if all { css_order(&kinds, values, skip) } else { Vec::new() };
    order.retain(|kind| !unbadged(*kind) && !slot_zero(*kind));
    let groups: Vec<u32> = order.into_iter().map(group_kind).collect();
    let number = |kind| groups.iter().position(|g| *g == group_kind(kind));
    let badge = |kind| match () {
        _ if slot_zero(kind) => Some(0),
        _ if unbadged(kind) => None,
        _ if all => number(kind).map(|i| i as u8 + 1),
        _ => group_rank(&kinds, values, kind),
    };
    kinds.iter().map(|&kind| badge(kind)).collect()
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn cell_of(kind: u32) -> usize {
        let grid = RosterGrid::new(Role::Player);
        (0..CELLS).find(|c| grid.info(*c).and_then(|i| i.kind()) == Some(kind)).expect("kind on the grid")
    }

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

    fn hats_badge_the_hat_order_and_hide_the_solo_markers() {
        let hats = cycles_values(&[0x0b, 0x07]); // Captain Falcon 1, Fox 2
        let badges = roster_badges_from_values(RosterUse::Hats, &hats, Some(0x00));
        assert_eq!(badge_of(&badges, 0x0b), Some(1));
        assert_eq!(badge_of(&badges, 0x07), Some(2));
        assert_eq!(badge_of(&badges, 0x06), None, "Kirby cannot be a hat");
        assert_eq!(badge_of(&badges, 0x49), None, "no Mii helper hat without an admitting catalog");
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

    fn mii_tiles_follow_the_catalog_admission() {
        // No catalog: no Mii tile can be chosen (the fork greyed them, " - unavailable") and none takes a badge.
        let brawler = cell_of(0x48);
        for use_ in [RosterUse::PlayerPick, RosterUse::CpuPick, RosterUse::Cycles, RosterUse::Hats] {
            let mut editor = RosterEditor::new(use_);
            let mut values = editor.values(None);
            assert_eq!(editor.press(brawler, Button::A, &mut values), Effect::Nothing, "{use_:?}");
            let mut frame = Frame::new();
            frame.tile_focus = brawler as u8;
            editor.decorate(&mut frame, &values);
            assert_eq!(frame.tile_state[brawler], TILE_GREYED, "{use_:?}");
            assert!(frame.help[1].string_lossy().ends_with(" - unavailable"), "{use_:?}");
            assert_eq!(frame.tile_state[cell_of(0x07)], 0, "other fighters stay as they are");
        }
        // The Player cycle admits the Brawler: it can be added and takes its number; the CPU cycle still does not.
        let mii = MiiTiles { player: [true, false, false], player_cycle: [true, false, false], ..MiiTiles::default() };
        let mut editor = RosterEditor::new(RosterUse::Cycles);
        editor.set_mii(mii);
        let mut values = cycles_values(&[0x07]);
        assert_eq!(editor.press(brawler, Button::A, &mut values), Effect::Changed);
        let badges = roster_badges(RosterUse::Cycles, &values, None, Role::Player, &mii);
        assert_eq!(badge_of(&badges, 0x07), Some(1));
        assert_eq!(badge_of(&badges, 0x48), Some(2), "an admitted Mii takes its number");
        assert_eq!(badge_of(&badges, 0x49), None, "a class with no admitted Mii takes none");
        assert_eq!(badge_of(&roster_badges(RosterUse::Cycles, &values, None, Role::Cpu, &mii), 0x48), None);
        assert_eq!(editor.press(0, Button::ZR, &mut values), Effect::Shown);
        assert_eq!(editor.press(brawler, Button::A, &mut values), Effect::Nothing, "the CPU cycle does not admit it");
        // Hats follow the helper admission only.
        let mut hats = RosterEditor::new(RosterUse::Hats);
        hats.set_mii(MiiTiles { hat: [true, false, false], ..MiiTiles::default() });
        let mut values = hats.values(None);
        values[0] = 0;
        assert_eq!(hats.press(brawler, Button::A, &mut values), Effect::Changed);
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

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_pick_is_one_hot_and_a_second_pick_moves_it", a_pick_is_one_hot_and_a_second_pick_moves_it as fn()),
            ("an_order_ranks_adds_removes_and_switches_role", an_order_ranks_adds_removes_and_switches_role as fn()),
            ("solo_popo_persists_independently_for_each_role", solo_popo_persists_independently_for_each_role as fn()),
            ("hats_badge_the_hat_order_and_hide_the_solo_markers", hats_badge_the_hat_order_and_hide_the_solo_markers as fn()),
            ("mii_tiles_follow_the_catalog_admission", mii_tiles_follow_the_catalog_admission as fn()),
        ]);
    }
    }
}
