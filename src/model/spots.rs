//! Sweet, sour and late spots (rc.14, operator decisions SP1 to SP5, 2026-11-14), pure and host-tested. A hitbox is a spot by its
//! damage, read from the game's own `AttackData` (`hitbox::layout::POWER`), never from a per-fighter table:
//!
//! * among the hitboxes active on one frame the strongest is **Sweet** (ties at the top are all Sweet) and a weaker one is **Sour**;
//! * when the frame's strongest hitbox is weaker than the strongest seen earlier in the same action (the sweet phase is over), its hitboxes are **Late**;
//! * when every active hitbox has the same damage and nothing stronger came earlier there is no label.
//!
//! The game does not say which slot connected (the only native is `AttackModule::is_infliction_status`, one flag for the whole
//! module), so a hit is attributed by the damage it added to the defender (`attribute`): a slot is a candidate when the added
//! damage divided by the slot's own damage lies in the multiplier window (staling and the Training multiplier scale every slot
//! alike, so the ratio to one slot is the same number, and a slot whose ratio is outside the window cannot have connected). The
//! label shows only when every candidate gets the same class; no candidate, or candidates of different classes, is no label.

/// The rows (decision SP5, colours reworded 2026-11-14): the hitbox visualizer's *Frame data colours* (hitbox::legend_colour), the strip's labels, and the combo's match. Each is a toggle, Off by default.
pub const ROW_FRAME_DATA: &str = "frame_data_colours";
pub const ROW_STRIP: &str = "spots_strip";
pub const ROW_COMBO: &str = "spots_combo";

/// Whether toggle row id is On (its first option is set).
pub fn row_on(snapshot: &super::settings::Snapshot, id: &str) -> bool {
    snapshot.get(id).is_some_and(|row| row.first() == Some(&1))
}

/// Attack slots a fighter has (`hitbox::SLOTS`).
pub const SLOTS: usize = 8;

/// The damage of each attack slot that is active, 0.0 for one that is not (or that deals none).
pub type Powers = [f32; SLOTS];

/// Damages closer than this are equal.
pub const EPS: f32 = 0.05;
/// The window of added damage over the hitbox's own damage (decision SP1's attribution). Fresh hits measure 1.0; staling only
/// lowers it. The window is the one tuned number here: set from the ratios the diag build logs on Eden.
pub const K_LOW: f32 = 0.9;
pub const K_HIGH: f32 = 1.1;
/// Frames with no active hitbox after which the next hitbox starts a new action (a repeated jab, a second move).
pub const GAP: u8 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    Sweet,
    Sour,
    Late,
}

impl Spot {
    /// The label (the strip shows it, the combo bar says it).
    pub const fn name(self) -> &'static str {
        match self {
            Spot::Sweet => "Sweet",
            Spot::Sour => "Sour",
            Spot::Late => "Late",
        }
    }

    /// The word a combo file stores (`"spot"`); anything else reads as no spot.
    pub fn from_key(key: &str) -> Option<Spot> {
        [Spot::Sweet, Spot::Sour, Spot::Late].into_iter().find(|s| s.name().eq_ignore_ascii_case(key))
    }
}

/// The class of a hitbox of damage `p` among the `active` ones, `earlier` being the strongest damage seen before this frame in the
/// same action (0.0 when none). `None`: no label.
pub fn classify(p: f32, active: &Powers, earlier: f32) -> Option<Spot> {
    if p.is_nan() || p <= 0.0 {
        return None;
    }
    let strongest = active.iter().copied().fold(0.0, f32::max);
    if strongest < earlier - EPS {
        Some(Spot::Late)
    } else if p < strongest - EPS {
        Some(Spot::Sour)
    } else if active.iter().any(|&q| q > 0.0 && q < p - EPS) {
        Some(Spot::Sweet)
    } else {
        None
    }
}

/// The strongest damage of one fighter's current action: what `classify` takes as `earlier`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Peak {
    status: i32,
    value: f32,
    idle: u8,
}

impl Peak {
    pub const fn new() -> Peak {
        Peak { status: i32::MIN, value: 0.0, idle: 0 }
    }

    /// The strongest damage seen earlier in this action, for a frame in `status`: a new status is a new action.
    pub fn earlier(&mut self, status: i32) -> f32 {
        if status != self.status {
            *self = Peak { status, value: 0.0, idle: 0 };
        }
        self.value
    }

    /// Fold this frame's active damages in, after the frame was classified.
    pub fn fold(&mut self, active: &Powers) {
        let top = active.iter().copied().fold(0.0, f32::max);
        if top > 0.0 {
            self.value = self.value.max(top);
            self.idle = 0;
        } else {
            self.idle = self.idle.saturating_add(1);
            if self.idle >= GAP {
                self.value = 0.0;
            }
        }
    }
}

/// The spot of the hit that added `gained` damage to the defender, from the attacker's `active` hitboxes and the action's
/// `earlier` peak. `None` when it cannot be told.
pub fn attribute(gained: f32, active: &Powers, earlier: f32) -> Option<Spot> {
    if !gained.is_finite() || gained <= 0.0 {
        return None;
    }
    let mut found: Option<Option<Spot>> = None;
    for &p in active.iter().filter(|&&p| p > 0.0 && (K_LOW..=K_HIGH).contains(&(gained / p))) {
        let class = classify(p, active, earlier);
        match found {
            Some(other) if other != class => return None,
            _ => found = Some(class),
        }
    }
    found.flatten()
}

#[cfg(test)]
#[path = "spots_tests.rs"]
mod tests;
