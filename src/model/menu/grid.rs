//! Paging of the item and stage grids by console mode (N5, Q14 B). The mode is an input: the
//! layout is chosen when the grid opens and stays fixed until it closes (SPEC-core-03 §2.5).
//!
//! * Items (38): docked, one page of 5 columns x 8 rows (38 used, short names); handheld, three
//!   pages of 4 x 4 (16, 16, 6, full names).
//! * Stages (All and the catalog's stages, 116 tiles for the carried catalog of 115): docked, three pages of\n//!   5 x 8 (40, 40, 36, short names); handheld, eight pages of 4 x 4 (seven of 16, one of 4, full names).

/// The console's operation mode when a grid opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleMode {
    Docked,
    Handheld,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridKind {
    Items,
    Stages,
    /// The portrait roster (Player/CPU Character, Character Cycles, Kirby Hats): the approved one-page 13 x 7 grid of the character
    /// select screen in both console modes, the last row of 9 centred (`engine::swap::roster_grid`).
    Roster(RosterUse),
}

/// Which roster the portrait grid edits (one approved grid, four rows of the menu).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RosterUse {
    /// Player Character / CPU Character: one fighter (`player_character`, `cpu_character`).
    PlayerPick,
    CpuPick,
    /// Character Cycles: the ordered rosters (`player_roster`, `cpu_roster`; ZR switches).
    Cycles,
    /// Kirby Hats: the ordered hat lists (`player_kirby_hats`, `cpu_kirby_hats`; ZR switches).
    Hats,
}

/// The give-item grid's tiles (`ITEM_NAMES`: 38 items, turnip faces included).
pub const ITEM_TILES: usize = 38;
/// The stage grid's tiles: the All tile and every stage of the catalog (never a hard-coded count: lead ruling
/// 2026-11-02; the carried catalog has 115 stages, package 6 supplies the authoritative list).
pub const STAGE_TILES: usize = 118;
/// The roster grid's cells: six rows of 13 and a centred last row of 9 (`engine::swap::roster_grid::CELLS`, checked in lib.rs).
pub const ROSTER_TILES: usize = 87;
/// The roster grid's columns and rows.
pub const ROSTER_COLS: u8 = 13;
pub const ROSTER_ROWS: u8 = 7;

/// A direction of cursor movement in a grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Up,
    Down,
    Left,
    Right,
}

/// The geometry of one grid, fixed at open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridLayout {
    pub kind: GridKind,
    pub mode: ConsoleMode,
    pub cols: u8,
    pub rows: u8,
    /// Docked grids use the short names (SPEC-core-03 §2.5).
    pub short_names: bool,
}

impl GridLayout {
    pub const fn new(kind: GridKind, mode: ConsoleMode) -> GridLayout {
        if let GridKind::Roster(_) = kind {
            return GridLayout { kind, mode, cols: ROSTER_COLS, rows: ROSTER_ROWS, short_names: true };
        }
        match mode {
            ConsoleMode::Docked => GridLayout { kind, mode, cols: 5, rows: 8, short_names: true },
            ConsoleMode::Handheld => GridLayout { kind, mode, cols: 4, rows: 4, short_names: false },
        }
    }

    /// Tiles in the whole grid.
    pub const fn tiles(&self) -> usize {
        match self.kind {
            GridKind::Items => ITEM_TILES,
            GridKind::Stages => STAGE_TILES,
            GridKind::Roster(_) => ROSTER_TILES,
        }
    }

    /// Cells on a full page.
    pub const fn per_page(&self) -> usize {
        self.cols as usize * self.rows as usize
    }

    pub const fn pages(&self) -> usize {
        self.tiles().div_ceil(self.per_page())
    }

    /// Tiles on `page` (`0` past the last page).
    pub const fn page_len(&self, page: usize) -> usize {
        let start = page * self.per_page();
        if start >= self.tiles() {
            0
        } else if self.tiles() - start < self.per_page() {
            self.tiles() - start
        } else {
            self.per_page()
        }
    }

    /// The tile number (in the grid's own order) at `cell` (row-major) of `page`.
    pub const fn tile(&self, page: usize, cell: usize) -> Option<usize> {
        if cell < self.page_len(page) {
            Some(page * self.per_page() + cell)
        } else {
            None
        }
    }

    /// Where the cursor goes from `cell` on `page`: rows and columns wrap, and empty cells of the last
    /// page are skipped in the direction of the move.
    pub fn step(&self, page: usize, cell: usize, direction: Move) -> usize {
        let len = self.page_len(page);
        if len == 0 {
            return 0;
        }
        let (cols, rows) = (usize::from(self.cols), usize::from(self.rows));
        let (mut col, mut row) = (cell % cols, cell / cols);
        for _ in 0..cols.max(rows) {
            match direction {
                Move::Left => col = (col + cols - 1) % cols,
                Move::Right => col = (col + 1) % cols,
                Move::Up => row = (row + rows - 1) % rows,
                Move::Down => row = (row + 1) % rows,
            }
            let next = row * cols + col;
            if next < len {
                return next;
            }
        }
        cell
    }
}
