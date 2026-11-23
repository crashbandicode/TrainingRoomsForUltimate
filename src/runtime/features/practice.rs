//! rc.15 practice menu (`model::practice_games`): which practice game runs. Start and Stop are menu actions; the menu worker
//! sets the one value here, and the games read it on their own threads (an atomic only, never a lock). A game that is not
//! the running one reads its mode as Off and stops through its own Off path, so one game runs at a time.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::model::practice_games::Game;
use crate::runtime::diag_log::diagnostic;

/// The running game's code in the low byte, plus an activation number. Stop preserves the number.
static ACTIVE: AtomicU64 = AtomicU64::new(0);
pub const STARTING: u64 = 0x100;
const GAME_MASK: u64 = 0xff;

pub fn activation() -> (u64, Option<Game>) {
    let word = ACTIVE.load(Ordering::Acquire);
    (word, Game::from_code(word as u8))
}

/// The game Start started last and Stop has not ended.
pub fn active() -> Option<Game> {
    activation().1
}

pub fn is(game: Game) -> bool {
    active() == Some(game)
}

/// How a game starts: Break the Targets in its placing view, Tech practice in its point picker or its wizard.
#[derive(Clone, Copy, Debug, Default)]
pub struct How {
    pub place_targets: bool,
    pub tech_point: bool,
    pub tech_wizard: bool,
    pub resume_parry: bool,
    pub edge_juggle_setup: bool,
}

/// Start `game` (the menu worker, at the close): it becomes the running game before its start trigger, so the game thread
/// that takes the trigger reads it on. Any other game stops on its next frame.
pub fn start(game: Game, how: How) {
    if crate::model::edge_juggle::Drill::from_game(game).is_some_and(|drill| !super::edge_juggle::allowed(drill)) {
        return;
    }
    // A running game's settings apply at its next Start: each game latches its own here or when the Start reaches it.
    super::hitbox::practice_started();
    if game != Game::Parry {
        super::parry::stopped();
    }
    let next = (ACTIVE.load(Ordering::Relaxed) & !(GAME_MASK | STARTING)).wrapping_add(0x200);
    ACTIVE.store(next | STARTING | u64::from(game.code()), Ordering::Release);
    match game {
        Game::Eggs => super::hitbox::eggs_start(),
        Game::Btt => super::btt::start(how.place_targets),
        Game::Mashout => super::mashout::start(),
        Game::Parry => super::parry::start(how.resume_parry),
        Game::GuardEdge | Game::Juggler => super::edge_juggle::start(how.edge_juggle_setup),
        Game::Tech => {
            super::tech::start(how.tech_point);
            if how.tech_wizard {
                super::tech::open_wizard();
            }
        }
    }
    ACTIVE.fetch_and(!STARTING, Ordering::Release);
    diagnostic(format_args!("[Practice] start game={game:?} how={how:?}"));
}

/// Stop the running game (the menu worker, Stop with the menu open).
pub fn stop() {
    let was = Game::from_code(ACTIVE.fetch_and(!GAME_MASK, Ordering::AcqRel) as u8);
    super::edge_juggle::stopped();
    super::parry::stopped();
    diagnostic(format_args!("[Practice] stop game={was:?}"));
}

/// End the named game from its frame or input path, if the menu still owns that game.
pub fn finish(game: Game) {
    let word = ACTIVE.load(Ordering::Acquire);
    if word as u8 == game.code() {
        let _ = ACTIVE.compare_exchange(word, word & !GAME_MASK, Ordering::AcqRel, Ordering::Acquire);
    }
}

/// The game the menu can stop, including an Eggs-ercise result that can still be restarted.
pub fn running() -> Option<Game> {
    let game = active()?;
    let live = match game {
        Game::Eggs => super::hitbox::eggs_running(),
        Game::Btt => super::btt::running() || super::btt::starting(),
        Game::Tech => super::tech::running() || super::tech::starting(),
        // Mash out has no end of its own: each result re-arms the next attempt until Stop.
        Game::Mashout => true,
        Game::Parry => super::parry::on(),
        Game::GuardEdge | Game::Juggler => true,
    };
    live.then_some(game)
}
