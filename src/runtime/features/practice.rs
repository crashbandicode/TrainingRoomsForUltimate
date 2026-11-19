//! rc.15 practice menu (`model::practice_games`): which practice game runs. Start and Stop are menu actions; the menu worker
//! sets the one value here, and the games read it on their own threads (an atomic only, never a lock). A game that is not
//! the running one reads its mode as Off and stops through its own Off path, so one game runs at a time.

use std::sync::atomic::{AtomicU8, Ordering};

use crate::model::practice_games::Game;
use crate::runtime::diag_log::diagnostic;

/// The running game's code (`Game::code`), 0 for none.
static ACTIVE: AtomicU8 = AtomicU8::new(0);

/// The game Start started last and Stop has not ended.
pub fn active() -> Option<Game> {
    Game::from_code(ACTIVE.load(Ordering::Acquire))
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
}

/// Start `game` (the menu worker, at the close): it becomes the running game before its start trigger, so the game thread
/// that takes the trigger reads it on. Any other game stops on its next frame.
pub fn start(game: Game, how: How) {
    // A running game's settings apply at its next Start: each game latches its own here or when the Start reaches it.
    super::hitbox::practice_started();
    if game != Game::Parry {
        super::parry::stopped();
    }
    ACTIVE.store(game.code(), Ordering::Release);
    match game {
        Game::Eggs => super::hitbox::eggs_start(),
        Game::Btt => super::btt::start(how.place_targets),
        Game::Mashout => super::mashout::start(),
        Game::Parry => super::parry::start(how.resume_parry),
        Game::Tech => {
            super::tech::start(how.tech_point);
            if how.tech_wizard {
                super::tech::open_wizard();
            }
        }
    }
    diagnostic(format_args!("[Practice] start game={game:?} how={how:?}"));
}

/// Stop the running game (the menu worker, Stop with the menu open).
pub fn stop() {
    let was = Game::from_code(ACTIVE.swap(0, Ordering::AcqRel));
    super::parry::stopped();
    diagnostic(format_args!("[Practice] stop game={was:?}"));
}

/// The game the menu shows as running: the running game while its engine runs it or a start is still on its way.
pub fn running() -> Option<Game> {
    let game = active()?;
    let live = match game {
        Game::Eggs => super::hitbox::eggs_running(),
        Game::Btt => super::btt::running() || super::btt::starting(),
        Game::Tech => super::tech::running() || super::tech::starting(),
        // Mash out has no end of its own: each result re-arms the next attempt until Stop.
        Game::Mashout => true,
        Game::Parry => super::parry::on(),
    };
    live.then_some(game)
}
