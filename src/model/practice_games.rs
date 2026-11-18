//! rc.15 practice menu (`docs/DECISIONS.md`, "rc.15 practice menu", 2026-11-18): the practice games of Fighters & Stages ›
//! Practice games, each with its own sub-screen and Start first. Which game runs is not a setting: Start and Stop are menu
//! actions, and the runtime keeps the one running game (`runtime::features::practice`). A game's settings row holds only
//! its mode now; the key and the stored length are unchanged, and the Off an older file stored reads as the first mode.

/// The practice games, in the order of the Practice games list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    Eggs,
    Btt,
    Tech,
}

impl Game {
    pub const ALL: [Game; 3] = [Game::Eggs, Game::Btt, Game::Tech];

    pub const fn title(self) -> &'static str {
        match self {
            Game::Eggs => "Eggs-ercise",
            Game::Btt => "Break the Targets",
            Game::Tech => "Tech practice",
        }
    }

    /// The game in an atomic: 0 is none.
    pub const fn code(self) -> u8 {
        self as u8 + 1
    }

    pub fn from_code(code: u8) -> Option<Game> {
        Game::ALL.into_iter().find(|game| game.code() == code)
    }

    /// The settings row of the game's mode, whose option 0 is the Off older files stored. Break the Targets has none: its
    /// `btt` key stays in the file, and nothing reads it.
    pub const fn mode_row(self) -> Option<&'static str> {
        match self {
            Game::Eggs => Some("eggsercise"),
            Game::Btt => None,
            Game::Tech => Some("tech_practice"),
        }
    }
}

/// How many stored options of row `id` come before the ones the menu lists: 1 for a game's mode (its retired Off).
pub fn retired(id: &str) -> usize {
    usize::from(Game::ALL.iter().any(|game| game.mode_row() == Some(id)))
}

/// The mode a game runs with, from its row's stored option: an older file's Off (0) is the first mode.
pub const fn mode(stored: u8) -> u8 {
    if stored == 0 {
        1
    } else {
        stored
    }
}

/// What a game's engine reads as its mode now: its mode while it is the running game (`active`), Off (0) for every other
/// game. One value names the running game, so one game runs at a time.
pub fn effective(game: Game, stored: u8, active: Option<Game>) -> u8 {
    if active == Some(game) {
        mode(stored)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn one_game_runs_at_a_time() {
        for active in [None, Some(Game::Eggs), Some(Game::Btt), Some(Game::Tech)] {
            for stored in 0..6 {
                let on: Vec<Game> = Game::ALL.into_iter().filter(|g| effective(*g, stored, active) != 0).collect();
                assert_eq!(on, active.into_iter().collect::<Vec<_>>(), "active {active:?} stored {stored}");
            }
        }
        for game in Game::ALL {
            assert_eq!(Game::from_code(game.code()), Some(game));
        }
        assert_eq!(Game::from_code(0), None);
    }

    #[test]
    fn a_stored_off_runs_the_first_mode() {
        assert_eq!((mode(0), mode(1), mode(4)), (1, 1, 4));
        assert_eq!(effective(Game::Eggs, 0, Some(Game::Eggs)), 1, "a file's Off plays 1 minute");
        assert_eq!(effective(Game::Tech, 3, Some(Game::Tech)), 3);
        assert_eq!((retired("eggsercise"), retired("tech_practice"), retired("btt"), retired("eggsercise_break")), (1, 1, 0, 0));
    }
    }
}
