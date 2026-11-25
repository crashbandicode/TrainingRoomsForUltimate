//! Wire contract of the optional tr.practice state provider.
use super::practice_games::Game;

pub const NAME: &[u8] = b"tr.practice";
pub const SCHEMA_ID: u32 = 2;
pub const SCHEMA_VERSION: u32 = 1;
pub const CPU_ENTRIES: usize = 8;

/// Slots of the game-to-worker sample mailbox. The worker drains every 33 ms (two game frames). The busiest case is
/// Break the Targets with Ice Climbers: two fighter passes and one P1 input poll post a sample each frame, six between
/// two drains. A full mailbox drops the newest sample; the per-frame producers post again on their next pass.
pub const SAMPLE_SLOTS: usize = 6;

/// The activation the poll publishes: the live practice layout's own, none without one.
pub fn published_activation(scene: &str, activation: impl FnOnce() -> (u64, Option<Game>)) -> (u64, Option<Game>) {
    if matches!(scene, "training" | "lobby") { activation() } else { (0, None) }
}

/// Keep the picker's three states inline in the sample, not a pointer and length for their wire text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CpuView {
    #[default]
    NotHeld,
    InView,
    Magnifier,
}

impl CpuView {
    fn wire(self) -> &'static str {
        match self {
            Self::NotHeld => "",
            Self::InView => "in_view",
            Self::Magnifier => "magnifier",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fields {
    pub phase: &'static str,
    pub step: &'static str,
    pub wizard_step: &'static str,
    pub wizard_selection: &'static str,
    pub wizard_point: Option<[f32; 2]>,
    /// How a picker's CPU point is shown (`crosshair::cpu_view`): `in_view` or `magnifier`; empty when the CPU's
    /// body is not held there.
    pub wizard_cpu: CpuView,
    /// Guard the Edge's start options as the running drill took them at Start (`edge_juggle::StartOptions`): under
    /// the stage allowed, midair jumps used. The settings, not what the game did with them; none for every other game.
    pub start_options: Option<[bool; 2]>,
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

/// Outcomes live with the run that produced them.
pub fn adopt_sample(current: &mut Fields, sample: Fields) {
    *current = sample;
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
            ("wizard.cpu", fields.wizard_cpu.wire()),
            ("last.outcome", fields.last_outcome),
        ] {
            if !value.is_empty() {
                let _ = writeln!(out, "{key}={value}");
            }
        }
        if let Some([under, used]) = fields.start_options {
            let _ = writeln!(out, "under={}\njumps={}", if under { "on" } else { "off" }, if used { "used" } else { "kept" });
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
