//! Pure model of the shared Player/CPU portrait roster editor.
//!
//! The physical grid mirrors the observed all-unlocked CSS and is independent of
//! the stable catalog order, which only the caller's kind slice defines. Rank
//! arrays are `[all, ranks...]` in catalog order, the layout the ordered roster
//! serializes. This module edits them for the caller but never owns them.

pub const COLUMNS: usize = 13;
pub const ROWS: usize = 7;
const FULL_ROWS: usize = ROWS - 1;
const LAST_ROW_CELLS: usize = 9;
/// The final row is centered: eight fighters and the role control.
const LAST_ROW_START: usize = (COLUMNS - LAST_ROW_CELLS) / 2;
pub const CELLS: usize = FULL_ROWS * COLUMNS + LAST_ROW_CELLS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Player,
    Cpu,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellContent {
    /// Catalog kind currently displayed by the cell.
    Fighter(u32),
    /// Visible placeholder that keeps its position and can never be selected.
    Unsupported,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellInfo {
    pub row: usize,
    /// Physical column: the centered last row starts at column 2.
    pub column: usize,
    pub label: &'static str,
    pub content: CellContent,
}

impl CellInfo {
    pub fn kind(self) -> Option<u32> {
        match self.content {
            CellContent::Fighter(kind) => Some(kind),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Unchanged,
    RosterChanged,
}

/// Slots whose native entry holds several forms; X previews them independently.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Aegis,
    PokemonTrainer,
}

impl Group {
    const fn forms(self) -> &'static [(u32, &'static str)] {
        match self {
            Self::Aegis => &[(0x5a, "Pyra"), (0x5b, "Mythra")],
            Self::PokemonTrainer => &[(0x24, "Squirtle"), (0x25, "Ivysaur"), (0x26, "Charizard")],
        }
    }
}

#[derive(Clone, Copy)]
enum Slot {
    Fighter(u32, &'static str),
    Shared(Group),
    All,
}

const fn f(kind: u32, name: &'static str) -> Slot {
    Slot::Fighter(kind, name)
}

/// Row-major physical cells, six rows of 13 then the 9-cell last row.
const GRID: [Slot; CELLS] = [
    f(0x00, "Mario"),
    f(0x01, "Donkey Kong"),
    f(0x02, "Link"),
    f(0x03, "Samus"),
    f(0x04, "Dark Samus"),
    f(0x05, "Yoshi"),
    f(0x06, "Kirby"),
    f(0x07, "Fox"),
    f(0x08, "Pikachu"),
    f(0x09, "Luigi"),
    f(0x0a, "Ness"),
    f(0x0b, "Captain Falcon"),
    f(0x0c, "Jigglypuff"),
    f(0x0d, "Peach"),
    f(0x0e, "Daisy"),
    f(0x0f, "Bowser"),
    f(0x4b, "Ice Climbers"),
    f(0x10, "Sheik"),
    f(0x11, "Zelda"),
    f(0x12, "Dr. Mario"),
    f(0x13, "Pichu"),
    f(0x14, "Falco"),
    f(0x15, "Marth"),
    f(0x16, "Lucina"),
    f(0x17, "Young Link"),
    f(0x18, "Ganondorf"),
    f(0x19, "Mewtwo"),
    f(0x1a, "Roy"),
    f(0x1b, "Chrom"),
    f(0x1c, "Mr. Game & Watch"),
    f(0x1d, "Meta Knight"),
    f(0x1e, "Pit"),
    f(0x1f, "Dark Pit"),
    f(0x20, "Zero Suit Samus"),
    f(0x21, "Wario"),
    f(0x22, "Snake"),
    f(0x23, "Ike"),
    Slot::Shared(Group::PokemonTrainer),
    f(0x27, "Diddy Kong"),
    f(0x28, "Lucas"),
    f(0x29, "Sonic"),
    f(0x2a, "King Dedede"),
    f(0x2b, "Olimar"),
    f(0x2c, "Lucario"),
    f(0x2d, "R.O.B."),
    f(0x2e, "Toon Link"),
    f(0x2f, "Wolf"),
    f(0x30, "Villager"),
    f(0x31, "Mega Man"),
    f(0x32, "Wii Fit Trainer"),
    f(0x33, "Rosalina & Luma"),
    f(0x34, "Little Mac"),
    f(0x35, "Greninja"),
    f(0x36, "Palutena"),
    f(0x37, "PAC-MAN"),
    f(0x38, "Robin"),
    f(0x39, "Shulk"),
    f(0x3a, "Bowser Jr."),
    f(0x3b, "Duck Hunt"),
    f(0x3c, "Ryu"),
    f(0x3d, "Ken"),
    f(0x3e, "Cloud"),
    f(0x3f, "Corrin"),
    f(0x40, "Bayonetta"),
    f(0x41, "Inkling"),
    f(0x42, "Ridley"),
    f(0x43, "Simon"),
    f(0x44, "Richter"),
    f(0x45, "King K. Rool"),
    f(0x46, "Isabelle"),
    f(0x47, "Incineroar"),
    f(0x51, "Piranha Plant"),
    f(0x52, "Joker"),
    f(0x53, "Hero"),
    f(0x54, "Banjo & Kazooie"),
    f(0x55, "Terry"),
    f(0x56, "Byleth"),
    f(0x57, "Min Min"),
    f(0x58, "Steve"),
    f(0x59, "Sephiroth"),
    Slot::Shared(Group::Aegis),
    f(0x5c, "Kazuya"),
    f(0x5d, "Sora"),
    f(0x48, "Mii Brawler"),
    f(0x49,"Mii Swordfighter"),
    f(0x4a, "Mii Gunner"),
    Slot::All,
];

#[cfg(test)]
const fn row_len(row: usize) -> usize {
    if row < FULL_ROWS {
        COLUMNS
    } else {
        LAST_ROW_CELLS
    }
}

const fn row_start(row: usize) -> usize {
    if row < FULL_ROWS {
        0
    } else {
        LAST_ROW_START
    }
}

/// Cell index to (row, physical column). Rows hold `COLUMNS` cell indices each,
/// so the last row's offset is simply shifted by its start column.
const fn position(cell: usize) -> (usize, usize) {
    let row = cell / COLUMNS;
    (row, row_start(row) + cell % COLUMNS)
}

#[cfg(test)]
const fn cell_at(row: usize, column: usize) -> usize {
    row * COLUMNS + column - row_start(row)
}

/// Editor state: cursor cell, active role, and the previewed form of each group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RosterGrid {
    cursor: usize,
    role: Role,
    forms: [u8; 2],
}

impl RosterGrid {
    pub const fn new(role: Role) -> Self {
        Self {
            cursor: 0,
            role,
            forms: [0; 2],
        }
    }

    pub const fn role(&self) -> Role {
        self.role
    }

    #[cfg(test)]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn toggle_role(&mut self) {
        self.role = match self.role {
            Role::Player => Role::Cpu,
            Role::Cpu => Role::Player,
        };
    }

    /// Open the one-shot picker on its configured fighter and exact starting form.
    pub fn focus_kind(&mut self, kind: u32) {
        for group in [Group::Aegis, Group::PokemonTrainer] {
            if let Some(index) = group.forms().iter().position(|(form, _)| *form == kind) {
                self.forms[group as usize] = index as u8;
            }
        }
        if let Some(cell) = (0..CELLS).find(|cell| self.describe(*cell).kind() == Some(kind)) {
            self.cursor = cell;
        }
    }

    /// Left/Right wrap within the physical row. Up/Down wrap through the rows and
    /// land on the nearest physical column, so the centered last row clamps.
    #[cfg(test)]
    pub fn step(&mut self, direction: Direction) {
        let (row, column) = position(self.cursor);
        let (row, column) = match direction {
            Direction::Left | Direction::Right => {
                let len = row_len(row);
                let offset = column - row_start(row);
                let next = match direction {
                    Direction::Right => (offset + 1) % len,
                    _ => (offset + len - 1) % len,
                };
                (row, row_start(row) + next)
            }
            Direction::Up | Direction::Down => {
                let next = match direction {
                    Direction::Down => (row + 1) % ROWS,
                    _ => (row + ROWS - 1) % ROWS,
                };
                let first = row_start(next);
                (next, column.clamp(first, first + row_len(next) - 1))
            }
        };
        self.cursor = cell_at(row, column);
    }

    /// X: advance the previewed form of a shared slot. Returns whether the cursor
    /// was on one. Never touches a roster.
    pub fn cycle_form(&mut self) -> bool {
        let Slot::Shared(group) = GRID[self.cursor] else {
            return false;
        };
        let form = &mut self.forms[group as usize];
        *form = (*form + 1) % group.forms().len() as u8;
        true
    }

    /// Geometry plus displayed kind and label of any cell; `None` past the grid.
    pub fn info(&self, cell: usize) -> Option<CellInfo> {
        (cell < CELLS).then(|| self.describe(cell))
    }

    pub fn cursor_info(&self) -> CellInfo {
        self.describe(self.cursor)
    }

    fn describe(&self, cell: usize) -> CellInfo {
        let (row, column) = position(cell);
        let (label, content) = match GRID[cell] {
            Slot::Fighter(kind, name) => (name, CellContent::Fighter(kind)),
            Slot::Shared(group) => {
                let (kind, name) = group.forms()[usize::from(self.forms[group as usize])];
                (name, CellContent::Fighter(kind))
            }
            Slot::All => ("All", CellContent::All),
        };
        CellInfo {
            row,
            column,
            label,
            content,
        }
    }

    /// A: select All on the control cell, otherwise append the displayed
    /// kind to `values`, the active role's `[all, ranks...]`. Unsupported cells
    /// and kinds missing from `kinds` change nothing.
    pub fn confirm(&mut self, kinds: &[u32], values: &mut [u8]) -> Outcome {
        match self.cursor_info().content {
            CellContent::All => {
                select_all(kinds, values);
                Outcome::RosterChanged
            }
            CellContent::Fighter(kind) => {
                let changed = select_group(kinds, values, kind);
                if changed {
                    Outcome::RosterChanged
                } else {
                    Outcome::Unchanged
                }
            }
            CellContent::Unsupported => Outcome::Unchanged,
        }
    }

    /// Separate removal control for the displayed kind in the active role's `values`.
    pub fn remove(&self, kinds: &[u32], values: &mut [u8]) -> bool {
        self.cursor_info()
            .kind()
            .and_then(|kind| selected_group_index(kinds, values, kind))
            .is_some_and(|i| deselect_kind(values, i))
    }
}

/// Stable identity of a native CSS group. IC's 0x4b descriptor stays distinct
/// from the Popo/Nana actor kinds; the runtime owns that boundary.
pub const fn group_kind(kind: u32) -> u32 {
    match kind {
        0x24..=0x26 => 0x24,
        0x5a | 0x5b => 0x5a,
        _ => kind,
    }
}

/// Native group the Player cycle skips because the original CSS fighter is slot 0.
/// Ice Climbers is the one exception: when its Solo Popo preference differs between
/// the original and the cycle, they load different bodies, so it may repeat as a
/// numbered slot. Equal preferences keep the usual exclusion.
pub fn cycle_exclusion(original: Option<u32>, original_solo: bool, cycle_solo: bool) -> Option<u32> {
    original.filter(|kind| *kind != 0x4b || original_solo == cycle_solo)
}

pub fn selected_group_index(kinds: &[u32], values: &[u8], kind: u32) -> Option<usize> {
    kinds
        .iter()
        .enumerate()
        .filter(|(_, k)| group_kind(**k) == group_kind(kind))
        .find(|(i, _)| rank(values, *i).is_some())
        .map(|(i, _)| i)
}

pub fn group_rank(kinds: &[u32], values: &[u8], kind: u32) -> Option<u8> {
    if values.first() == Some(&1) {
        return None;
    }
    selected_group_index(kinds, values, kind).and_then(|i| rank(values, i))
}

fn select_group(kinds: &[u32], values: &mut [u8], kind: u32) -> bool {
    let Some(index) = catalog_index(kinds, kind) else {
        return false;
    };
    if values.len() != kinds.len() + 1 {
        return false;
    }
    if values[0] == 0 {
        if let Some(previous) = selected_group_index(kinds, values, kind) {
            if previous == index {
                return false;
            }
            values[index + 1] = values[previous + 1];
            values[previous + 1] = 0;
            return true;
        }
    }
    select_kind(values, index)
}

/// All retains only non-default group starts. Those entries are preferences,
/// not explicit ranks, and are interpreted only while the All flag is set.
pub fn select_all(kinds: &[u32], values: &mut [u8]) {
    let starts: Vec<_> = [0x24, 0x5a]
        .iter()
        .filter_map(|kind| selected_group_index(kinds, values, *kind))
        .filter(|i| kinds[*i] != group_kind(kinds[*i]))
        .collect();
    values.fill(0);
    if let Some(all) = values.first_mut() {
        *all = 1;
    }
    for i in starts {
        values[i + 1] = 1;
    }
}

/// CSS row-major order, once per native group, with selected starting forms.
pub fn css_order(kinds: &[u32], values: &[u8], excluded: Option<u32>) -> Vec<u32> {
    let mut out = Vec::new();
    css_order_each(kinds, values, excluded, |kind| out.push(kind));
    out
}

/// `css_order` without heap: each kind in CSS order is handed to `each` (the planner's per-tick form).
pub fn css_order_each(kinds: &[u32], values: &[u8], excluded: Option<u32>, mut each: impl FnMut(u32)) {
    GRID.iter()
        .filter_map(|slot| match slot {
            Slot::Fighter(kind, _) => Some(*kind),
            Slot::Shared(group) => {
                let default = group.forms()[0].0;
                Some(
                    selected_group_index(kinds, values, default)
                        .map(|i| kinds[i])
                        .unwrap_or(default),
                )
            }
            _ => None,
        })
        .filter(|kind| kinds.contains(kind) && excluded.map(group_kind) != Some(group_kind(*kind)))
        .for_each(&mut each);
}

impl RosterGrid {
}

/// Position of `kind` in the caller's stable catalog. The catalog lives in the
/// consts crate, which depends on this one, so it is passed in.
pub fn catalog_index(kinds: &[u32], kind: u32) -> Option<usize> {
    kinds.iter().position(|candidate| *candidate == kind)
}

/// Cycle position (1-based) of a catalog entry, `None` when unselected or in All mode.
pub fn rank(values: &[u8], catalog_index: usize) -> Option<u8> {
    values
        .get(1..)?
        .get(catalog_index)
        .copied()
        .filter(|rank| *rank != 0)
}

/// Append a catalog entry to the cycle. A selected entry keeps its rank, and
/// choosing from All leaves exactly that one explicit member.
pub fn select_kind(values: &mut [u8], catalog_index: usize) -> bool {
    let Some((all, ranks)) = values.split_first_mut() else {
        return false;
    };
    let Some(&current) = ranks.get(catalog_index) else {
        return false;
    };
    let all_mode = *all != 0;
    if !all_mode && current != 0 {
        return false;
    }
    let count = if all_mode {
        0
    } else {
        ranks.iter().filter(|rank| **rank != 0).count()
    };
    let Ok(next) = u8::try_from(count + 1) else {
        return false;
    };
    if all_mode {
        *all = 0;
        ranks.fill(0);
    }
    ranks[catalog_index] = next;
    true
}

/// Remove a catalog entry and compact later ranks. The final explicit member
/// stays, and All mode has nothing to remove.
pub fn deselect_kind(values: &mut [u8], catalog_index: usize) -> bool {
    let Some((all, ranks)) = values.split_first_mut() else {
        return false;
    };
    let Some(&removed) = ranks.get(catalog_index) else {
        return false;
    };
    if *all != 0 || removed == 0 || ranks.iter().filter(|rank| **rank != 0).count() == 1 {
        return false;
    }
    ranks[catalog_index] = 0;
    for rank in ranks.iter_mut().filter(|rank| **rank > removed) {
        *rank -= 1;
    }
    true
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    /// Mirrors the consts crate's `trio::CHOICES` kind order (85 stable positions).
    const CATALOG: [u32; 89] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
        0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f,
        0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e,
        0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56,
        0x57, 0x58, 0x59, 0x5c, 0x5d, // legacy 80 end here; forms are appended below
        0x5a, 0x5b, 0x24, 0x25, 0x26, 0x4b,
        0x49,
        0x48,
        0x4a,
    ];

    fn named(grid: &RosterGrid, label: &str) -> usize {
        (0..CELLS)
            .find(|cell| grid.info(*cell).unwrap().label == label)
            .unwrap_or_else(|| panic!("no cell labeled {label}"))
    }

    fn go_to(grid: &mut RosterGrid, label: &str) {
        grid.cursor = named(grid, label);
    }

    fn all_mode() -> Vec<u8> {
        let mut values = vec![0; CATALOG.len() + 1];
        values[0] = 1;
        values
    }

    fn explicit_mode(catalog_indexes: &[usize]) -> Vec<u8> {
        let mut values = vec![0; CATALOG.len() + 1];
        for (rank, index) in catalog_indexes.iter().enumerate() {
            values[index + 1] = rank as u8 + 1;
        }
        values
    }

    fn all_cell_changes_only_the_active_roster_and_role_switch_is_separate() {
        let mut rosters = [explicit_mode(&[0, 1]), explicit_mode(&[7])];
        let snapshot = rosters.clone();
        let mut grid = RosterGrid::new(Role::Player);
        go_to(&mut grid, "All");
        for expected in [Role::Cpu, Role::Player, Role::Cpu] {
            grid.toggle_role();
            assert_eq!(grid.role(), expected);
        }
        assert_eq!(rosters, snapshot);
        assert_eq!(
            grid.confirm(&CATALOG, &mut rosters[1]),
            Outcome::RosterChanged
        );
        assert_eq!(rosters[1], all_mode());
        assert_eq!(rosters[0], snapshot[0]);
        rosters = snapshot.clone();
        assert_eq!(grid.cursor_info().kind(), None);
        assert!(!grid.remove(&CATALOG, &mut rosters[0]));
        assert_eq!(rosters, snapshot);
        // Roles keep separate arrays: the same shared cell can hold different forms.
        let mut grid = RosterGrid::new(Role::Player);
        go_to(&mut grid, "Pyra");
        let active = grid.role() as usize;
        assert_eq!(
            grid.confirm(&CATALOG, &mut rosters[active]),
            Outcome::RosterChanged
        );
        grid.toggle_role();
        grid.cycle_form();
        let active = grid.role() as usize;
        assert_eq!(
            grid.confirm(&CATALOG, &mut rosters[active]),
            Outcome::RosterChanged
        );
        // Both badges read the shown kind's ranks, whichever role is active.
        assert_eq!(
            (rank(&rosters[0], 80), rank(&rosters[1], 80)),
            (Some(3), None)
        );
        assert_eq!(
            (rank(&rosters[0], 81), rank(&rosters[1], 81)),
            (None, Some(2))
        );
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("all_cell_changes_only_the_active_roster_and_role_switch_is_separate", all_cell_changes_only_the_active_roster_and_role_switch_is_separate as fn()),
        ]);
    }
    }
}
