//! rc.15 practice menu (`docs/DECISIONS.md`, "rc.15 practice menu", 2026-11-18): the practice games of Fighters & Stages ›
//! Practice games, each with its own sub-screen and Start first. Which game runs is not a setting: Start and Stop are menu
//! actions, and the runtime keeps the one running game (`runtime::features::practice`). A game's settings row holds only
//! its mode now; the key and the stored length are unchanged, and the Off an older file stored reads as the first mode.
//! Also the games' bounded results (`best_time`, `record_time`, `result_tick`).

/// The practice games, in the order of the Practice games list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    Eggs,
    Btt,
    Tech,
    Mashout,
    Parry,
    GuardEdge,
    Juggler,
}

impl Game {
    pub const ALL: [Game; 7] = [
        Game::Eggs, Game::Btt, Game::Tech, Game::Mashout, Game::Parry, Game::GuardEdge, Game::Juggler,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Game::Eggs => "Eggs-ercise",
            Game::Btt => "Break the Targets",
            Game::Tech => "Tech practice",
            Game::Mashout => "Mash out",
            Game::Parry => "Parry practice",
            Game::GuardEdge => "Guard the Edge",
            Game::Juggler => "Juggler",
        }
    }

    /// The game in an atomic: 0 is none.
    pub const fn code(self) -> u8 {
        self as u8 + 1
    }

    pub fn from_code(code: u8) -> Option<Game> {
        Game::ALL.into_iter().find(|game| game.code() == code)
    }

    /// The settings row of the game's mode, whose option 0 is the Off older files stored. Break the Targets and Mash out
    /// have none: their Off / On rows are Start / Stop now, and their `btt` / `mashout` keys stay in the file unread.
    pub const fn mode_row(self) -> Option<&'static str> {
        match self {
            Game::Eggs => Some("eggsercise"),
            Game::Btt | Game::Mashout | Game::GuardEdge | Game::Juggler => None,
            Game::Tech => Some("tech_practice"),
            Game::Parry => Some("parry_practice"),
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

/// A session's minimum elapsed frames for this exact key; vacant slots are separate from zero keys/times.
pub fn best_time<K: Copy + Eq, const N: usize>(pool: &[Option<(K, u32)>; N], key: K) -> Option<u32> {
    pool.iter().flatten().find(|(saved, _)| *saved == key).map(|(_, time)| *time)
}

/// Record a scored attempt. Updates stay in place; a new key evicts the oldest (FIFO, not LRU).
/// The caller owns scoring eligibility. A zero-capacity pool returns the time without retaining it.
pub fn record_time<K: Copy + Eq, const N: usize>(pool: &mut [Option<(K, u32)>; N], key: K, time: u32) -> u32 {
    record_best(pool, key, time, u32::min)
}

/// Record a scored attempt for a game where a higher value wins.
pub fn record_max<K: Copy + Eq, const N: usize>(pool: &mut [Option<(K, u32)>; N], key: K, score: u32) -> u32 {
    record_best(pool, key, score, u32::max)
}

fn record_best<K: Copy + Eq, const N: usize>(
    pool: &mut [Option<(K, u32)>; N],
    key: K,
    time: u32,
    better: impl FnOnce(u32, u32) -> u32,
) -> u32 {
    if let Some((_, best)) = pool.iter_mut().flatten().find(|(saved, _)| *saved == key) {
        *best = better(*best, time);
        return *best;
    }
    if N > 0 {
        pool.rotate_left(1);
        pool[N - 1] = Some((key, time));
    }
    time
}

/// Advance the result's eligible-frame countdown. Its owner leaves the result phase when this is true.
pub fn result_tick(left: &mut u16, tick: bool) -> bool {
    *left = left.saturating_sub(u16::from(tick));
    *left == 0
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn edge_juggle_are_independent_practice_games() {
        let edge = Game::ALL.iter().copied().find(|game| game.title() == "Guard the Edge")
            .expect("Guard the Edge has its own practice game entry");
        let juggle = Game::ALL.iter().copied().find(|game| game.title() == "Juggler")
            .expect("Juggler has its own practice game entry");
        assert_eq!(Game::from_code(edge.code()), Some(edge));
        assert_eq!(Game::from_code(juggle.code()), Some(juggle));
        assert_eq!(effective(edge, 0, Some(juggle)), 0);
        assert_eq!(effective(juggle, 0, Some(edge)), 0);
        assert_eq!(Game::from_code(5), Some(Game::Parry), "existing codes stay stable");
    }

    #[test]
    fn one_game_runs_at_a_time() {
        let mut every = vec![None];
        every.extend(Game::ALL.map(Some));
        for active in every {
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
        assert_eq!((retired("eggsercise"), retired("tech_practice"), retired("btt"), retired("mashout")), (1, 1, 0, 0));
        assert_eq!(retired("eggsercise_break"), 0);
        assert_eq!((retired("parry_practice"), effective(Game::Parry, 0, Some(Game::Parry)), effective(Game::Parry, 2, None)), (1, 1, 0));
    }

    #[test]
    fn best_time_pool_keeps_exact_keys_and_fifo() {
        // Keys share their first component; the complete identity must remain distinct.
        let mut pool = [None; 3];
        assert_eq!(best_time(&pool, (0, 0)), None);
        assert_eq!(record_time(&mut pool, (0, 0), 40), 40);
        assert_eq!(record_time(&mut pool, (0, 1), 30), 30);
        assert_eq!(record_time(&mut pool, (0, 2), 20), 20);
        let order = pool.map(|entry| entry.map(|(key, _)| key));
        assert_eq!(record_time(&mut pool, (0, 0), 0), 0);
        assert_eq!(record_time(&mut pool, (0, 0), u32::MAX), 0);
        assert_eq!(pool.map(|entry| entry.map(|(key, _)| key)), order, "updates never refresh eviction order");
        assert_eq!([best_time(&pool, (0, 0)), best_time(&pool, (0, 1)), best_time(&pool, (0, 2))], [Some(0), Some(30), Some(20)]);
        record_time(&mut pool, (0, 3), 10);
        assert_eq!(best_time(&pool, (0, 0)), None, "even an improved key is evicted in insertion order");
        assert_eq!(pool, [Some(((0, 1), 30)), Some(((0, 2), 20)), Some(((0, 3), 10))]);
        let mut empty = [None; 0];
        assert_eq!(record_time(&mut empty, 0u8, 5), 5);
        assert_eq!(best_time(&empty, 0), None);
    }
    }
}
