//! The damage display helper (SPEC-input-05 §2.3, §5.2): while a save state kills and respawns the
//! fighters the game's percentage digits are forced visible and correct at once and the death
//! explosion graphics are hidden. Pure: the digits, the pane rules and the animation-frame values; the
//! HUD tick (package 5's anchor) writes the panes by name.
//!
//! Decision applied (SPEC-input-05 question 4, recommendation): the helper runs only while a save-state
//! load or a lobby quick reset is in progress, and writes nothing when a fighter's accessor is
//! missing.

/// The four digits of a damage percent: hundreds, tens, ones and tenths.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Digits {
    pub hundreds: u8,
    pub tens: u8,
    pub ones: u8,
    pub tenths: u8,
}

#[cfg(test)]
impl Digits {
    pub const ZERO: Digits = Digits { hundreds: 0, tens: 0, ones: 0, tenths: 0 };
}

/// The digits of `damage` (123.4 gives 1, 2, 3, 4; 7.0 gives 0, 0, 7, 0). Negative and NaN read as
/// zero; above 999.9 the hundreds saturate at 9.
#[cfg(test)]
pub fn digits(damage: f32) -> Digits {
    if damage.is_nan() || damage < 0.0 {
        return Digits::ZERO;
    }
    // Tenths are taken from the value in tenths, rounded to the nearest, so 7.0 never reads 6.9.
    let tenths_total = ((f64::from(damage) * 10.0).round() as u32).min(9999);
    Digits {
        hundreds: (tenths_total / 1000) as u8,
        tens: (tenths_total / 100 % 10) as u8,
        ones: (tenths_total / 10 % 10) as u8,
        tenths: (tenths_total % 10) as u8,
    }
}

/// One pane rule of the percent display: the pane's name under the fighter's `p1`/`p2` part and the
/// `pos_y` it is forced to (`None`: only alpha is forced).
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneRule {
    pub name: &'static str,
    pub pos_y: Option<f32>,
}

/// The percent panes, in the spec's order (§4). `set_dmg_p` is 0, `set_dmg_num_p` -4, `dig_dec` -16 and
/// every other pane 0.
#[cfg(test)]
pub const PERCENT_PANES: [PaneRule; 16] = [
    PaneRule { name: "set_dmg_num_3", pos_y: Some(0.0) },
    PaneRule { name: "dig_3", pos_y: Some(0.0) },
    PaneRule { name: "dig_3_anim", pos_y: Some(0.0) },
    PaneRule { name: "set_dmg_num_2", pos_y: Some(0.0) },
    PaneRule { name: "dig_2", pos_y: Some(0.0) },
    PaneRule { name: "dig_2_anim", pos_y: Some(0.0) },
    PaneRule { name: "set_dmg_num_1", pos_y: Some(0.0) },
    PaneRule { name: "dig_1", pos_y: Some(0.0) },
    PaneRule { name: "dig_1_anim", pos_y: Some(0.0) },
    PaneRule { name: "set_dmg_num_p", pos_y: Some(-4.0) },
    PaneRule { name: "dig_dec", pos_y: Some(-16.0) },
    PaneRule { name: "dig_dec_anim_00", pos_y: Some(0.0) },
    PaneRule { name: "set_dmg_num_dec", pos_y: Some(0.0) },
    PaneRule { name: "dig_dec_anim_01", pos_y: Some(0.0) },
    PaneRule { name: "dig_0_anim", pos_y: Some(0.0) },
    PaneRule { name: "set_dmg_p", pos_y: Some(0.0) },
];

/// The death-effect panes whose alpha and global alpha are set to 0.
#[cfg(test)]
pub const DEATH_EFFECT_PANES: [&str; 4] = ["set_fxui_dead1", "set_fxui_dead2", "set_fxui_dead3", "set_fxui_fire"];

/// Whether `rule` is skipped for these digits: the hundreds panes (names containing `3`) when the
/// hundreds are 0, the tens panes (names containing `2`) when hundreds and tens are 0.
#[cfg(test)]
pub fn skipped(rule: &PaneRule, d: Digits) -> bool {
    (rule.name.contains('3') && d.hundreds == 0) || (rule.name.contains('2') && d.hundreds == 0 && d.tens == 0)
}

/// The material animations whose current frame is set to a digit: `set_dmg_num_3` the hundreds,
/// `_2` the tens, `_1` the ones, `_dec` the tenths.
#[cfg(test)]
pub fn animation_frame(animation: &str, d: Digits) -> Option<f32> {
    Some(f32::from(match animation {
        "set_dmg_num_3" => d.hundreds,
        "set_dmg_num_2" => d.tens,
        "set_dmg_num_1" => d.ones,
        "set_dmg_num_dec" => d.tenths,
        _ => return None,
    }))
}

/// What a pane's alpha says about the animation frames: they are rewritten once per fighter per tick,
/// only when some digit pane's alpha was not 255.
#[cfg(test)]
pub fn needs_animation_rewrite(alpha_was: u8, global_alpha_was: u8) -> bool {
    alpha_was != 255 || global_alpha_was != 255
}

/// Whether the helper runs this HUD tick: in Training with ready-go, and only while a save-state load
/// or a lobby quick reset is in progress.
#[cfg(test)]
pub const fn runs(training: bool, ready_go: bool, load_or_quick_reset_in_progress: bool) -> bool {
    training && ready_go && load_or_quick_reset_in_progress
}

/// A fighter whose accessor is missing: the digits read as zeros and nothing else is written
/// (background matchmaking unloads the fighters; SPEC-input-05 §6.8).
#[cfg(test)]
pub fn digits_or_none(damage: Option<f32>) -> Option<Digits> {
    damage.map(digits)
}
