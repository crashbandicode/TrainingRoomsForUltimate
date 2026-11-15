//! Sweet, sour and late spots (rc.14, operator decisions SP1 to SP5, 2026-11-14; ranked by launch power since spots-3, 2026-11-15), pure and
//! host-tested. SmashWiki's definition: the sweet spot is the part of an attack with the most desirable knockback, usually the most
//! powerful; a sour spot the least powerful. So a hitbox ranks by the knockback it gives the defender (`knockback`, the game's formula), from
//! its own `AttackData` (damage, base knockback, growth, fixed knockback: `hitbox::powers`) and the defender's percent and weight at that
//! moment, never from a per-fighter table; damage decides only between equal knockbacks. The launch angle (the "most desirable" direction)
//! is a matter of taste and plays no part.
//!
//! * among the hitboxes active on one frame the one that launches hardest is **Sweet** (ties at the top are all Sweet) and a weaker one **Sour**;
//! * when the frame's strongest is weaker than the strongest seen earlier in the same action (the sweet phase is over), its hitboxes are **Late**;
//! * hitboxes equal in knockback and damage, and a lone hitbox with nothing stronger before it, have no label.
//!
//! A hit is attributed (`attribute`) by what the defender took: the damage it gained, divided by each slot's damage and the attacker's
//! `AttackModule::power_mul_status` (the smash charge), must lie in `K_LOW..=K_HIGH` (the defender's side adds 1.26 for a fresh hit, about
//! 1.07 stalest: Eden, 2026-11-14), and the knockback it received (`DamageModule::reaction`, the value the game's hit handler stores for the
//! strongest hit of the frame) divided by the slot's knockback must lie in `KB_LOW..=KB_HIGH`. The label shows only when every slot left
//! gets the same class; without the knockback reading, slots equal in damage but of different classes give no label.

/// The rows (decision SP5): the strip's labels and the combo's match, toggles Off by default. The hitbox visualizer's *Frame data colours*
/// (`hitbox::legend_colour`) is an item of its checklist (`legend::COLOURS`).
pub const ROW_STRIP: &str = "spots_strip";
pub const ROW_COMBO: &str = "spots_combo";

/// Whether toggle row id is On (its first option is set).
pub fn row_on(snapshot: &super::settings::Snapshot, id: &str) -> bool {
    snapshot.get(id).is_some_and(|row| row.first() == Some(&1))
}

/// Attack slots a fighter has (`hitbox::SLOTS`).
pub const SLOTS: usize = 8;

/// One active attack slot's launch numbers, from its `AttackData` (`hitbox::layout::POWER`, `legend::KNOCKBACK_QUADS`); all zero for a slot
/// that is not active.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hit {
    pub damage: f32,
    pub growth: i32,
    pub fixed: i32,
    pub base: i32,
}

/// Every attack slot's `Hit`.
pub type Powers = [Hit; SLOTS];

/// What a hitbox's knockback depends on besides its own numbers: the attacker's damage multiplier (`AttackModule::power_mul_status`: 1.0, or
/// the smash charge), the defender's percent before the hit and its weight (`WorkModule` parameter `weight`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub mul: f32,
    pub percent: f32,
    pub weight: f32,
}

impl Launch {
    /// Where there is no defender to read (a lone fighter): weight 100 at 0%.
    pub const STANDARD: Launch = Launch { mul: 1.0, percent: 0.0, weight: 100.0 };

    /// The weight, the standard 100 when it was not read.
    fn weight(self) -> f32 {
        if self.weight.is_finite() && self.weight > 0.0 { self.weight } else { 100.0 }
    }
}

/// Damages closer than this are equal; so are knockbacks closer than `KB_EPS`.
pub const EPS: f32 = 0.05;
pub const KB_EPS: f32 = 0.5;
/// The window of added damage over the hitbox's own damage times the attacker's charge (decision SP1's attribution), set from the
/// `[Spots]` lines of Eden (the module notes): the remaining multiplier is 1.26 for a fresh hit and about 1.07 stalest.
pub const K_LOW: f32 = 1.0;
pub const K_HIGH: f32 = 1.6;
/// The defender's side of a fresh hit's damage (1.26, Eden 2026-11-14) and the part of it that is the 1-on-1 ratio (1.2; the rest, 1.05, is
/// the attacker's freshness, which the formula's damage keeps).
pub const DEALT: f32 = 1.26;
pub const TARGET_MUL: f32 = 1.2;
/// The window of received knockback over a slot's knockback: the multipliers every hitbox of a move shares and the formula leaves out
/// (rage up to 1.1, crouching 0.85).
pub const KB_LOW: f32 = 0.8;
pub const KB_HIGH: f32 = 1.2;

/// The knockback of `hit` dealing damage `d` to a defender of weight `w` at percent `p` after the hit, by the game's formula (SmashWiki,
/// *Knockback*): ((p/10 + p·d/20) · 200/(w+100) · 1.4 + 18) · growth/100 + base; a fixed knockback f takes the place of d, with p = 10.
pub fn knockback(hit: &Hit, d: f32, p: f32, w: f32) -> f32 {
    let (p, d) = if hit.fixed != 0 { (10.0, hit.fixed as f32) } else { (p, d) };
    ((p / 10.0 + p * d / 20.0) * 200.0 / (w + 100.0) * 1.4 + 18.0) * hit.growth as f32 / 100.0 + hit.base as f32
}

/// What a hitbox ranks by: its knockback against the defender, then its damage.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Power {
    knockback: f32,
    damage: f32,
}

impl Power {
    fn of(hit: &Hit, launch: Launch) -> Power {
        let d = hit.damage * launch.mul;
        Power { knockback: knockback(hit, d, launch.percent + d * DEALT, launch.weight()), damage: hit.damage }
    }

    /// Weaker than `other`: by more than `KB_EPS` in knockback, or equal in knockback and by more than `EPS` in damage.
    fn below(self, other: Power) -> bool {
        if (self.knockback - other.knockback).abs() > KB_EPS { self.knockback < other.knockback } else { self.damage < other.damage - EPS }
    }
}

/// The class of every slot of `active` against `launch`, `earlier` being the strongest hitbox seen before this frame in the same action
/// (`Hit::default()` when none); `None` for a slot that is not active and for one with no label.
pub fn classify(active: &Powers, launch: Launch, earlier: &Hit) -> [Option<Spot>; SLOTS] {
    let mut out = [None; SLOTS];
    let live = |h: &&Hit| h.damage > 0.0;
    let Some(strongest) = active.iter().filter(live).map(|h| Power::of(h, launch)).reduce(|a, b| if a.below(b) { b } else { a }) else { return out };
    let late = earlier.damage > 0.0 && strongest.below(Power::of(earlier, launch));
    for (slot, hit) in active.iter().enumerate().filter(|(_, h)| live(h)) {
        let power = Power::of(hit, launch);
        out[slot] = if late {
            Some(Spot::Late)
        } else if power.below(strongest) {
            Some(Spot::Sour)
        } else if active.iter().filter(live).any(|h| Power::of(h, launch).below(power)) {
            Some(Spot::Sweet)
        } else {
            None
        };
    }
    out
}

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

/// The strongest hitbox of one fighter's current action: what `classify` takes as `earlier`. An action is one status; a new one starts when
/// the status changes or the motion goes back (the same status entered again, as a jab's next hit does). It keeps the hitbox, not a number,
/// so it is measured again against the defender as it is now.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Peak {
    status: i32,
    frame: f32,
    hit: Hit,
}

impl Peak {
    pub const fn new() -> Peak {
        Peak { status: i32::MIN, frame: 0.0, hit: Hit { damage: 0.0, growth: 0, fixed: 0, base: 0 } }
    }

    /// The strongest hitbox seen earlier in this action, for a frame in `status` at motion frame `frame`.
    pub fn earlier(&mut self, status: i32, frame: f32) -> Hit {
        if status != self.status || frame < self.frame {
            self.hit = Hit::default();
        }
        (self.status, self.frame) = (status, frame);
        self.hit
    }

    /// Fold this frame's active hitboxes in, after the frame was classified.
    pub fn fold(&mut self, active: &Powers, launch: Launch) {
        for hit in active.iter().filter(|h| h.damage > 0.0) {
            if self.hit.damage <= 0.0 || Power::of(&self.hit, launch).below(Power::of(hit, launch)) {
                self.hit = *hit;
            }
        }
    }
}

/// A hit as the defender took it: the damage it gained and the knockback it received (`DamageModule::reaction`; 0.0 when not read).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Landed {
    pub gained: f32,
    pub launched: f32,
}

/// The spot of the hit `landed` from the attacker's `active` hitboxes, `launch` (its multiplier and the defender before the hit) and the
/// action's `earlier` peak. `None` when it cannot be told, which includes a multiplier that is not a positive number (0.0: a read that
/// was not made).
pub fn attribute(landed: Landed, active: &Powers, launch: Launch, earlier: &Hit) -> Option<Spot> {
    let Landed { gained, launched } = landed;
    if !gained.is_finite() || gained <= 0.0 || !launch.mul.is_finite() || launch.mul <= 0.0 {
        return None;
    }
    let classes = classify(active, launch, earlier);
    let read = launched.is_finite() && launched > 0.0 && launch.weight.is_finite() && launch.weight > 0.0;
    let mut found: Option<Option<Spot>> = None;
    for (slot, hit) in active.iter().enumerate().filter(|(_, h)| h.damage > 0.0) {
        if !(K_LOW..=K_HIGH).contains(&(gained / (hit.damage * launch.mul))) {
            continue;
        }
        let k = knockback(hit, gained / TARGET_MUL, launch.percent + gained, launch.weight);
        if read && !(k > 0.0 && (KB_LOW..=KB_HIGH).contains(&(launched / k))) {
            continue;
        }
        match found {
            Some(other) if other != classes[slot] => return None,
            _ => found = Some(classes[slot]),
        }
    }
    found.flatten()
}

#[cfg(test)]
#[path = "spots_tests.rs"]
mod tests;
