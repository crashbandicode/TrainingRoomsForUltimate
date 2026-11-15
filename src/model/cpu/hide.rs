//! Hide CPU (rc.14, SPEC-eggsercise V2.6): while the rule says so, a CPU body is invisible, has no hurtbox and no
//! collision, and takes no action; when the rule stops saying so, it is shown again where it stood. No CPU setting is
//! ever written. The runtime asks once per body frame (`Cpu::frame`) and does what the step says
//! (`runtime::features::hide_cpu`).

use super::role::Scene;

/// One body's step: `Some(true)` hides it (written on the first frame and again on every later one, so a reset or a swap
/// that rebuilt the body cannot leave it showing), `Some(false)` shows it again (the first frame after), `None` leaves it.
///
/// The rule: the Hide CPU row is On, or a practice game asked for the CPU to be hidden (`request`); a waiting room never
/// hides it, because hiding writes the practice CPU's game state, which the other consoles could see before the LDN check
/// has passed. `away` is whether the body was hidden last frame.
pub fn step(away: &mut bool, scene: Scene, row: bool, request: bool) -> Option<bool> {
    let wanted = scene == Scene::Training && (row || request);
    (std::mem::replace(away, wanted) || wanted).then_some(wanted)
}

/// The body's player tag (the floating label and arrow): the game's two per-fighter cursor work flags,
/// `FIGHTER_INSTANCE_WORK_ID_FLAG_CURSOR` and `..._FLAG_NAME_CURSOR`, the pair the game's own cursor helper writes
/// (`main+0x67db00`). `Some(shown)` to write, from the body's `step`.
pub const fn tag(_step: Option<bool>) -> Option<bool> {
    None
}

/// The HUD panels to hide, one bit per entry, from each body's `away` (in `FighterKey::index` order: two per entry).
pub fn panels(_away: impl IntoIterator<Item = bool>) -> u8 {
    0
}

/// The part of the `info_melee` layout that is `entry`'s HUD panel (portrait, name, damage, stocks).
pub fn panel_part(_entry: usize) -> Option<[u8; 2]> {
    None
}

/// One `info_melee` tick for one entry's panel: `Some(visible)` to write. `hidden` is whether this rule hid it.
pub fn panel_write(_wanted: bool, _hidden: &mut bool) -> Option<bool> {
    None
}

/// The notice for Play on CPU while the CPU is hidden: by a practice game's request, or by the row.
pub const fn refusal(request: bool) -> &'static str {
    // restriction: hide-cpu-play
    match request {
        true => "The CPU is hidden during this game",
        false => "The CPU is hidden: turn Hide CPU Off first",
    }
}
