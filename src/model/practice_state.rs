//! Wire contract of the optional tr.practice state provider.
use super::practice_games::Game;

pub const NAME: &[u8] = b"tr.practice";
pub const SCHEMA_ID: u32 = 2;
pub const SCHEMA_VERSION: u32 = 1;
pub const CPU_ENTRIES: usize = 8;

/// The activation the poll publishes: the live practice layout's own, none without one.
pub fn published_activation(scene: &str, activation: impl FnOnce() -> (u64, Option<Game>)) -> (u64, Option<Game>) {
    if scene == "training" { activation() } else { (0, None) }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fields {
    pub phase: &'static str,
    pub step: &'static str,
    pub wizard_step: &'static str,
    pub wizard_selection: &'static str,
    pub wizard_point: Option<[f32; 2]>,
    pub wizard_player_point: Option<[f32; 2]>,
    pub wizard_value: Option<u32>,
    pub score: Option<u32>,
    pub attempt_frames: Option<u32>,
    pub session_frames: Option<u32>,
    pub remaining_frames: Option<u32>,
    pub streak: Option<u32>,
    pub best: Option<u32>,
    pub last_frames: Option<u32>,
    pub fastest_frames: Option<u32>,
    pub baseline_frames: Option<u32>,
    pub attempts: Option<u32>,
    pub hits: Option<u32>,
    pub target_count: Option<u32>,
    pub last_outcome: &'static str,
}

pub fn snapshot(game: Option<Game>, fields: Fields, thinks: [u64; CPU_ENTRIES], out: &mut String) {
    use core::fmt::Write as _;

    out.clear();
    let _ = writeln!(out, "schema=tr.practice/{SCHEMA_VERSION}");
    let _ = writeln!(out, "game={}", game.map_or("none", game_name));
    let phase = match (game, fields.phase) {
        (None, _) => "idle",
        (Some(_), "" | "off") => "starting",
        (_, phase) => phase,
    };
    let _ = writeln!(out, "phase={phase}");
    if game.is_some() {
        for (key, value) in [
            ("step", fields.step),
            ("wizard.step", fields.wizard_step),
            ("wizard.selection", fields.wizard_selection),
            ("last.outcome", fields.last_outcome),
        ] {
            if !value.is_empty() {
                let _ = writeln!(out, "{key}={value}");
            }
        }
        if let Some([x, y]) = fields.wizard_point {
            let _ = writeln!(out, "wizard.x={x}\nwizard.y={y}");
        }
        if let Some([x, y]) = fields.wizard_player_point {
            let _ = writeln!(out, "wizard.player.x={x}\nwizard.player.y={y}");
        }
        for (key, value) in [
            ("wizard.value", fields.wizard_value),
            ("score", fields.score),
            ("attempt.frames", fields.attempt_frames),
            ("session.frames", fields.session_frames),
            ("remaining.frames", fields.remaining_frames),
            ("streak", fields.streak),
            ("best", fields.best),
            ("last.frames", fields.last_frames),
            ("fastest.frames", fields.fastest_frames),
            ("baseline.frames", fields.baseline_frames),
            ("attempts", fields.attempts),
            ("hits", fields.hits),
            ("target.count", fields.target_count),
        ] {
            if let Some(value) = value {
                let _ = writeln!(out, "{key}={value}");
            }
        }
    }
    for (entry, count) in thinks.into_iter().enumerate().skip(1) {
        let _ = writeln!(out, "cpu.{entry}.ai_thinks={count}");
    }
}

pub const fn game_name(game: Game) -> &'static str {
    match game {
        Game::Eggs => "eggs",
        Game::Btt => "btt",
        Game::Tech => "tech",
        Game::Mashout => "mashout",
        Game::Parry => "parry",
        Game::GuardEdge => "guard_edge",
        Game::Juggler => "juggler",
    }
}

pub fn think_entry(admission: u64, primary_match: Option<i32>, mut matches: impl FnMut(i32) -> bool) -> Option<usize> {
    if admission == 0 {
        return None;
    }
    let entries = 1..CPU_ENTRIES as i32;
    primary_match.filter(|entry| entries.contains(entry))
        .or_else(|| entries.clone().find(|&entry| matches(entry)))
        .map(|entry| entry as usize)
}


#[cfg(test)]
#[path = "practice_state_tests.rs"]
mod tests;
