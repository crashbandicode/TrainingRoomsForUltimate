//! Pokemon Trainer and Kirby restores (SPEC-state-05 §5.1, §5.2): which Pokemon the respawn brings out,
//! the trainer's five-frame wait after the switch, and which opponent lets Kirby get a hat back. Pure.

/// The three Pokemon kinds in the trainer's starting order, as the game's constants resolve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kinds {
    pub squirtle: i32,
    pub ivysaur: i32,
    pub charizard: i32,
}

impl Kinds {
    fn cycle(&self) -> [i32; 3] {
        [self.squirtle, self.ivysaur, self.charizard]
    }

    /// Whether `kind` is one of the three.
    pub fn is_pokemon(&self, kind: i32) -> bool {
        self.cycle().contains(&kind)
    }
}

/// The Pokemon descriptor at the game's decide site: the count of Pokemon and their three kinds
/// (SPEC-state-05 §5.1 step 3; the layout is the fork's observation, open question 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub count: u32,
    pub kinds: [u32; 3],
}

/// The index the game's decide routine is given for the saved Pokemon: `w8 = index of saved_kind` among
/// the descriptor's three kinds, only when the descriptor is exactly a rotation of (Squirtle, Ivysaur,
/// Charizard) with `count == 3`. `None` leaves the register alone (the record was not saved as a
/// Pokemon, or the descriptor does not check out). Upstream wrote `saved_kind - Squirtle`, which is right
/// only when the trainer starts with Squirtle.
pub fn decide_index(saved_kind: i32, descriptor: Option<Descriptor>, kinds: Kinds) -> Option<usize> {
    let value = saved_kind.checked_sub(kinds.squirtle)?;
    if !(0..=2).contains(&value) || !kinds.is_pokemon(saved_kind) {
        return None;
    }
    let descriptor = descriptor?;
    if descriptor.count != 3 {
        return None;
    }
    let cycle = kinds.cycle();
    // An array, not a `Vec`: this runs inside a game hook, which must not allocate.
    let found = [descriptor.kinds[0] as i32, descriptor.kinds[1] as i32, descriptor.kinds[2] as i32];
    let rotation = (0..3).any(|start| (0..3).all(|i| found[i] == cycle[(start + i) % 3]));
    if !rotation {
        return None;
    }
    found.iter().position(|k| *k == saved_kind)
}

/// What the trainer wait asks for on one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PtOut {
    /// Set the trainer's motion rate back to 1.0 (it is in `RUN`).
    pub rate_one: bool,
    /// The trainer is airborne after the switch: set its situation to GROUND and force `RUN`.
    pub ground_and_run: bool,
    pub done: bool,
}

/// The trainer's wait (`WaitForPokemonSwitch`): once the trainer is in `RUN` its rate returns to 1.0; five
/// in-game frames later (the switch flash is late) an airborne trainer is grounded; then the wait is over.
/// One counter per role (open question 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PtWait {
    frames: u8,
}

pub const PT_WAIT_FRAMES: u8 = 5;

impl PtWait {
    pub const fn new() -> Self {
        Self { frames: 0 }
    }

    pub fn clear(&mut self) {
        self.frames = 0;
    }

    pub fn step(&mut self, trainer_in_run: bool, trainer_airborne: bool) -> PtOut {
        let mut out = PtOut { rate_one: trainer_in_run, ..PtOut::default() };
        if self.frames < PT_WAIT_FRAMES {
            self.frames += 1;
            return out;
        }
        out.ground_and_run = trainer_airborne;
        out.done = true;
        self.frames = 0;
        out
    }
}

/// The Pokemon dead test (SPEC-state-01 §3, SPEC-state-05 §5.1 step 6): a Pokemon in `WAIT` whose previous
/// status is `NONE` while its trainer is in `RESTART_CHANGE` is dead, so the one switching frame the plain
/// test misses does not end the wait early.
pub fn switching_frame_is_dead(in_wait: bool, previous_is_none: bool, trainer_restarting: bool) -> bool {
    in_wait && previous_is_none && trainer_restarting
}

/// Which switch effect a request is (compared by the runtime against the game's hash40 values).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchEffect {
    /// `sys_flying_plate`: hidden on every spawn in Training (SPEC-state-05 open question 3).
    FlyingPlate,
    /// `ptrainer_change_light` and the effect hash `0x10e3fac8d9`: hidden while a load runs.
    ChangeLight,
    Other,
}

/// The effect hash the spec gives only as a number (the second change-light effect).
pub const CHANGE_LIGHT_RAW: u64 = 0x10e3fac8d9;

/// Whether a trainer-family object's effect is made size 0 (SPEC-state-05 §4.1, "Pokémon switch effects").
pub fn switch_effect_hidden(effect: SwitchEffect, loading: bool) -> bool {
    match effect {
        SwitchEffect::FlyingPlate => true,
        SwitchEffect::ChangeLight => loading,
        SwitchEffect::Other => false,
    }
}

/// Which sound a fighter `play_se` request is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// `se_ptrainer_change_appear`, `se_ptrainer_ball_open`, `se_ptrainer_ball_swing`.
    TrainerSwitch,
    /// Kirby's copy sounds (`0x1453dd86e4`, `0x14bdd3e7c8`).
    KirbyCopy,
    /// Sora's spell-change jingle (`0x156fdf29ba`).
    SoraSpell,
    Other,
}

pub const KIRBY_COPY_SE: [u64; 2] = [0x1453dd86e4, 0x14bdd3e7c8];
pub const SORA_SPELL_SE: u64 = 0x156fdf29ba;

/// Whether the sound becomes `se_silent` (SPEC-state-05 §4.1). The Kirby and Sora silences follow the spec's
/// open question 4 recommendation and apply only while a load runs; the status rules are the real copy
/// (`SPECIAL_N_DRINK`) and a real spell change (a status at or above Trail's `SPECIAL_N1`).
pub fn sound_silenced(sound: Sound, loading: bool, kirby_drinking: bool, at_or_above_trail_n1: bool) -> bool {
    match sound {
        Sound::TrainerSwitch => loading,
        Sound::KirbyCopy => loading && !kirby_drinking,
        Sound::SoraSpell => loading && !at_or_above_trail_n1,
        Sound::Other => false,
    }
}

/// The fighter kinds Kirby's hat acceptance compares (resolved constants).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KirbyKinds {
    pub pokemon: Kinds,
    pub pyra: i32,
    pub mythra: i32,
    /// The kind that reads as "none" (-1): a trainer's Pokemon whose own kind cannot be read.
    pub none: i32,
}

/// Whether Kirby gets his hat back: the opponent's kind equals the copied kind (`int_z`), or both are
/// in the Pokemon family (an opponent whose kind reads -1 counts as a Pokemon when its trainer parent
/// link is a valid active object, `opponent_trainer_valid`), or both are Pyra / Mythra
/// (SPEC-state-05 §5.2 step 2).
pub fn kirby_accepts(copied: i32, opponent: i32, opponent_trainer_valid: bool, kinds: &KirbyKinds) -> bool {
    if opponent == copied {
        return true;
    }
    let pokemon_like = |kind: i32, valid: bool| kinds.pokemon.is_pokemon(kind) || (kind == kinds.none && valid);
    if pokemon_like(copied, true) && pokemon_like(opponent, opponent_trainer_valid) {
        return true;
    }
    let aegis = |kind: i32| kind == kinds.pyra || kind == kinds.mythra;
    aegis(copied) && aegis(opponent)
}
