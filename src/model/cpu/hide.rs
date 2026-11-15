//! Hide CPU (rc.14, SPEC-eggsercise V2.6): while the rule says so, a CPU body is invisible, has no hurtbox and no
//! collision, and takes no action; when the rule stops saying so, it is shown again where it stood. Its player tag and
//! its HUD panel go and come back with it. No CPU setting is ever written. The runtime asks once per body frame
//! (`Cpu::frame`) and does what the step says (`runtime::features::hide_cpu`; the panel in `runtime::ui::hud`).


/// One body's step: `Some(true)` hides it (written on the first frame and again on every later one, so a reset or a swap
/// that rebuilt the body cannot leave it showing), `Some(false)` shows it again (the first frame after), `None` leaves it.
///
/// The rule: the Hide CPU row is On, or a practice game asked for the CPU to be hidden (`request`), in Training and in the
/// waiting room's practice alike (rc.14, 11-15: the room's practice is local to each console). `away` is whether the body was
/// hidden last frame.
pub fn step(away: &mut bool, row: bool, request: bool) -> Option<bool> {
    let wanted = row || request;
    (std::mem::replace(away, wanted) || wanted).then_some(wanted)
}

/// The body's player tag (the floating label and arrow): the game's two per-fighter cursor work flags,
/// `FIGHTER_INSTANCE_WORK_ID_FLAG_CURSOR` and `..._FLAG_NAME_CURSOR`, the pair the game's own cursor helper writes
/// (`main+0x67db00`; a Final Smash turns the other fighters' tags off with it and back on with `true`). `Some(shown)` to
/// write: off on every hidden frame, on the frame the body is shown, nothing otherwise.
pub fn tag(step: Option<bool>) -> Option<bool> {
    step.map(|hidden| !hidden)
}

/// The HUD panels to hide, one bit per entry: the entries with a body hidden on its last frame (`away`, in
/// `FighterKey::index` order, two bodies per entry). A rebuilt body starts shown and its first frame hides it again.
pub fn panels(away: impl IntoIterator<Item = bool>) -> u8 {
    away.into_iter().take(16).enumerate().filter(|(_, away)| *away).fold(0, |bits, (body, _)| bits | (1 << (body / 2)))
}

/// The part of the `info_melee` layout that is `entry`'s HUD panel (portrait, name, damage, stocks): `p1` for entry 0
/// (Player 1) up to `p8`.
pub fn panel_part(entry: usize) -> Option<[u8; 2]> {
    (entry < 8).then(|| [b'p', b'1' + entry as u8])
}

/// One `info_melee` tick for one entry's panel: `Some(visible)` to write. Hidden on every tick while `wanted` (a rebuilt
/// layout cannot show it), visible on the first tick after, and nothing for a panel this rule did not hide (`hidden`).
pub fn panel_write(wanted: bool, hidden: &mut bool) -> Option<bool> {
    (std::mem::replace(hidden, wanted) || wanted).then_some(!wanted)
}

/// The notice for Play on CPU while the CPU is hidden: by a practice game's request, or by the row.
pub const fn refusal(request: bool) -> &'static str {
    // restriction: hide-cpu-play
    match request {
        true => "The CPU is hidden during this game",
        false => "The CPU is hidden: turn Hide CPU Off first",
    }
}
