//! Buffs applied after a state is placed (SPEC-state-03): Hero's spells, Joker's Arsene, Wii Fit's Deep
//! Breathing, Cloud's Limit, Little Mac's KO Punch, Sephiroth's Winged form, a Shulk Monado Art, Min Min's
//! Power Dragon arm and a Wario waft level. Pure: one `BuffState` per save-state record (player, CPU)
//! steps on every `ApplyBuff` frame, reads facts the runtime took from the game and answers what to write.
//!
//! Decisions of the open questions (the specs' recommendations): one delay counter per record (OQ 5), a
//! Shulk art is drawn once per load (OQ 2), the Sephiroth flags are cleared whenever the step ends
//! without the wing activating (OQ 4). The bookkeeping belongs to the save-state record, never to "who
//! operates the fighter" (fork fix, folded).

use crate::model::cpu::rng::Rng;

use super::settings::Buffs;

/// The fighters a buff can apply to (SPEC-state-01 §5.3 step 8d).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuffKind {
    Hero,
    Cloud,
    Joker,
    LittleMac,
    Sephiroth,
    WiiFit,
    Shulk,
    MinMin,
    Wario,
}

/// The facts one `ApplyBuff` frame reads.
#[derive(Clone, Copy, Debug, Default)]
pub struct BuffFacts {
    /// Hero: the current status / the previous status is `SPECIAL_LW_START`.
    pub hero_cast_now: bool,
    pub hero_cast_before: bool,
    /// Shulk: the current / previous status is `SPECIAL_N_ACTION`.
    pub shulk_action_now: bool,
    pub shulk_action_before: bool,
    /// Wii Fit: the current / previous status is `SPECIAL_LW_SUCCESS`.
    pub wii_success_now: bool,
    pub wii_success_before: bool,
    /// Sephiroth: the `ONE_WINGED_ACTIVATED` flag is set.
    pub wing_activated: bool,
}

/// The Hero spells, in menu order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spell {
    Accelerate,
    Oomph,
    Psyche,
    Bounce,
}

/// The Monado Arts, in menu order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    Jump,
    Speed,
    Shield,
    Buster,
    Smash,
}

/// The Wario waft levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waft {
    Mini,
    Half,
    Full,
}

/// What one `ApplyBuff` frame does.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BuffOut {
    /// The record's buffing flag after this frame (the effect, sound, wing and heal hooks read it).
    pub buffing: bool,
    /// Hero: write this spell into the decide-command work int and force `SPECIAL_LW_START` (blocking
    /// shield).
    pub hero_cast: Option<Spell>,
    /// Hero or Shulk or Wii Fit: set the motion rate (50 / 40 / 40) while in the buff's status.
    pub motion_rate: Option<f32>,
    /// Cloud: add 100 limit. Joker: add 120 rebel gauge. Little Mac: KO gauge 100.
    pub add_limit: bool,
    pub add_rebel: bool,
    pub ko_gauge: bool,
    /// Sephiroth: raise / clear the two rule flags.
    pub wing_flags_on: bool,
    pub wing_flags_off: bool,
    /// Wario: write this waft level.
    pub waft: Option<Waft>,
    /// Shulk: write the art, the select timer 29 and the select flag, then force `SPECIAL_N_ACTION`.
    pub shulk_write: Option<Art>,
    /// Wii Fit: force `SPECIAL_LW_SUCCESS` (not blocking shield).
    pub wii_force: bool,
    /// Min Min: the whole Power Dragon sequence.
    pub minmin: bool,
    /// The step is complete: clear the flag, the marker goes to `NoAction`.
    pub done: bool,
}

/// One record's buff bookkeeping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BuffState {
    buffing: bool,
    started: bool,
    /// Hero: spells still to cast.
    remaining: u8,
    /// The delay counter's started flag and count (`should_delay`).
    delay_started: bool,
    delay_count: u8,
    /// Shulk: the art drawn for this load.
    art: Option<Art>,
    /// Sephiroth: the rule flags are currently raised by this step.
    wing_raised: bool,
}

impl BuffState {
    pub const fn new() -> Self {
        Self { buffing: false, started: false, remaining: 0, delay_started: false, delay_count: 0, art: None, wing_raised: false }
    }

    /// The buffing flag.
    pub fn buffing(&self) -> bool {
        self.buffing
    }

    /// Clear everything (session end, a new load request, a training reset).
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// `should_delay(n)`: the first call starts counting and says "still delaying"; the call at which `n`
    /// frames have elapsed says "done" and resets itself (SPEC-state-03 §3).
    fn still_delaying(&mut self, n: u8) -> bool {
        if !self.delay_started {
            self.delay_started = true;
            self.delay_count = 0;
            return true;
        }
        self.delay_count = self.delay_count.saturating_add(1);
        if self.delay_count >= n {
            self.delay_started = false;
            self.delay_count = 0;
            false
        } else {
            true
        }
    }

    /// The `k`-th selected Hero spell in menu order.
    fn hero_spell(buffs: Buffs, k: u8) -> Option<Spell> {
        [(Buffs::ACCELERATE, Spell::Accelerate), (Buffs::OOMPH, Spell::Oomph), (Buffs::PSYCHE, Spell::Psyche), (Buffs::BOUNCE, Spell::Bounce)]
            .into_iter()
            .filter(|(option, _)| buffs.has(*option))
            .map(|(_, spell)| spell)
            .nth(usize::from(k))
    }

    fn hero_count(buffs: Buffs) -> u8 {
        [Buffs::ACCELERATE, Buffs::OOMPH, Buffs::PSYCHE, Buffs::BOUNCE].iter().filter(|o| buffs.has(**o)).count() as u8
    }

    fn draw_art(buffs: Buffs, rng: &mut Rng) -> Option<Art> {
        let arts = [
            (Buffs::MONAD_JUMP, Art::Jump),
            (Buffs::MONAD_SPEED, Art::Speed),
            (Buffs::MONAD_SHIELD, Art::Shield),
            (Buffs::MONAD_BUSTER, Art::Buster),
            (Buffs::MONAD_SMASH, Art::Smash),
        ];
        let count = arts.iter().filter(|(o, _)| buffs.has(*o)).count() as u32;
        if count == 0 {
            return None;
        }
        let pick = rng.below(count) as usize;
        arts.iter().filter(|(o, _)| buffs.has(*o)).nth(pick).map(|(_, a)| *a)
    }

    fn draw_waft(buffs: Buffs, rng: &mut Rng) -> Option<Waft> {
        let levels = [(Buffs::WAFT_MINI, Waft::Mini), (Buffs::WAFT_HALF, Waft::Half), (Buffs::WAFT_FULL, Waft::Full)];
        let count = levels.iter().filter(|(o, _)| buffs.has(*o)).count() as u32;
        if count == 0 {
            return None;
        }
        let pick = rng.below(count) as usize;
        levels.iter().filter(|(o, _)| buffs.has(*o)).nth(pick).map(|(_, l)| *l)
    }

    /// One `ApplyBuff` frame for a fighter of `kind`.
    pub fn step(&mut self, kind: BuffKind, buffs: Buffs, facts: &BuffFacts, rng: &mut Rng) -> BuffOut {
        let mut out = BuffOut::default();
        let first = !self.started;
        match kind {
            BuffKind::Hero => {
                if first {
                    self.started = true;
                    self.buffing = true;
                    self.remaining = Self::hero_count(buffs);
                }
                if facts.hero_cast_before && !facts.hero_cast_now {
                    self.remaining = self.remaining.saturating_sub(1);
                }
                if self.remaining == 0 {
                    // The spell's sound is late: a 3-frame tail, still silenced.
                    out.done = !self.still_delaying(3);
                } else if let Some(spell) = Self::hero_spell(buffs, self.remaining - 1) {
                    if facts.hero_cast_now {
                        out.motion_rate = Some(50.0);
                    } else {
                        out.hero_cast = Some(spell);
                    }
                } else {
                    // The list shrank under the index (a setting changed mid-cast).
                    self.remaining = Self::hero_count(buffs);
                }
            }
            BuffKind::Cloud | BuffKind::Joker | BuffKind::LittleMac => {
                let option = match kind {
                    BuffKind::Cloud => Buffs::LIMIT,
                    BuffKind::Joker => Buffs::ARSENE,
                    _ => Buffs::KO,
                };
                if !buffs.has(option) {
                    out.done = true;
                } else {
                    if first {
                        self.started = true;
                        self.buffing = true;
                        out.add_limit = kind == BuffKind::Cloud;
                        out.add_rebel = kind == BuffKind::Joker;
                        out.ko_gauge = kind == BuffKind::LittleMac;
                    }
                    // The voice or KO sound is late: a 2-frame tail.
                    out.done = !self.still_delaying(2);
                }
            }
            BuffKind::Sephiroth => {
                if !buffs.has(Buffs::WING) {
                    out.wing_flags_off = self.wing_raised;
                    self.wing_raised = false;
                    out.done = true;
                } else {
                    if first {
                        self.started = true;
                        self.wing_raised = true;
                        out.wing_flags_on = true;
                    }
                    self.buffing = true;
                    if facts.wing_activated {
                        // Later deaths must not respawn with the wing.
                        out.wing_flags_off = true;
                        self.wing_raised = false;
                        out.done = true;
                    }
                }
            }
            BuffKind::Wario => {
                if first {
                    self.started = true;
                    if let Some(level) = Self::draw_waft(buffs, rng) {
                        out.waft = Some(level);
                        self.buffing = true;
                    }
                }
                out.done = true;
            }
            BuffKind::Shulk => {
                if first {
                    self.started = true;
                    self.art = Self::draw_art(buffs, rng);
                }
                match self.art {
                    None => out.done = true,
                    Some(art) => {
                        self.buffing = true;
                        if facts.shulk_action_before && !facts.shulk_action_now {
                            // The "JUMP!" voice line is late: a 3-frame tail.
                            out.done = !self.still_delaying(3);
                        } else if !facts.shulk_action_now {
                            out.shulk_write = Some(art);
                        } else {
                            out.motion_rate = Some(40.0);
                        }
                    }
                }
            }
            BuffKind::WiiFit => {
                if !buffs.has(Buffs::BREATHING) {
                    out.done = true;
                } else {
                    self.started = true;
                    self.buffing = true;
                    if facts.wii_success_before && !facts.wii_success_now {
                        out.done = true;
                    } else if !facts.wii_success_now {
                        out.wii_force = true;
                    } else {
                        out.motion_rate = Some(40.0);
                    }
                }
            }
            BuffKind::MinMin => {
                if !buffs.has(Buffs::POWER_DRAGON) {
                    out.done = true;
                } else {
                    // Upstream never sets the buffing flag for Min Min.
                    self.started = true;
                    out.minmin = true;
                    out.done = true;
                }
            }
        }
        out.buffing = self.buffing;
        if out.done {
            let raised = self.wing_raised;
            self.clear();
            self.wing_raised = false;
            out.buffing = false;
            if raised && kind == BuffKind::Sephiroth {
                out.wing_flags_off = true;
            }
        }
        out
    }
}
