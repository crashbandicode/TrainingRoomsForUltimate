//! Special-move charges saved with a state and written back on a load (SPEC-state-02): which game fields
//! each fighter's charge lives in, the `Charge` a Save reads, and the plan of writes and visuals a Load
//! performs. Table-driven and pure: the fields are named constants (`ids::C`, resolved at run time), the
//! reads and writes go through a `Work` trait the runtime implements, and a plan is a list of `Step`s
//! the runtime executes. Olimar's Pikmin line, Sora's spell and Kirby's copied ability are in
//! `restore.rs` (SPEC-state-05).

use super::ids::{Table, C};
use super::record::{Charge, Robin};

/// Reading and writing a fighter's work fields (the runtime's `WorkModule` calls; tests use a map).
pub trait Work {
    /// Olimar: the colours of the Pikmin in line order, held first (SPEC-state-02 §5.1); the runtime
    /// reads the troops manager. Any other implementation has none.
    fn olimar_colours(&self) -> [Option<i32>; 3] {
        [None; 3]
    }
    fn get_int(&self, id: i32) -> i32;
    fn get_float(&self, id: i32) -> f32;
    fn get_flag(&self, id: i32) -> bool;
}

/// Which `Charge` slot a field fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    IntX,
    IntY,
    IntZ,
    FloatX,
    FloatY,
    FloatZ,
    Has,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    Int,
    Float,
    Flag,
}

/// One game field and the slot it is saved in.
#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub slot: Slot,
    pub ty: Ty,
    pub work: C,
}

const fn int(slot: Slot, work: C) -> Field {
    Field { slot, ty: Ty::Int, work }
}

const fn float(slot: Slot, work: C) -> Field {
    Field { slot, ty: Ty::Float, work }
}

const fn flag(work: C) -> Field {
    Field { slot: Slot::Has, ty: Ty::Flag, work }
}

/// A joint to attach an effect to: a name, or a raw hash40 the spec gives only as a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Joint {
    Name(&'static str),
    Raw(u64),
}

/// What a visual does (SPEC-state-02 §4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    /// `EffectModule::req_common(fighter, "charge_max", 0.0)`.
    ChargeMax,
    /// `EffectModule::remove_common(fighter, "charge_max")`.
    RemoveChargeMax,
    /// `EffectModule::req_follow(fighter, effect, joint, pos, rot, 1.0, false, attribute, 0, lifetime, 0,
    /// 0, false, false)`; the returned handle is stored in the work int `handle`, if any.
    Follow { effect: &'static str, joint: Joint, pos: [f32; 3], rot: [f32; 3], attribute: u32, lifetime: i32, handle: Option<C> },
    /// Kirby's copied effects, given only as raw hash40 values: `req_follow` with zero offsets, attribute 0.
    FollowRaw { effect: u64, joint: u64, lifetime: i32, handle: Option<C> },
    /// Donkey Kong's `set_face_motion_by_priority(FIGHTER_FACIAL_SPECIAL, "special_n_max_face")`.
    FaceMotion,
}

/// When a rule's visuals apply.
#[derive(Clone, Copy, Debug)]
pub enum When {
    IntIs(Slot, i32),
    FloatIs(Slot, f32),
    FloatIsNot(Slot, f32),
}

#[derive(Clone, Copy, Debug)]
pub struct Rule {
    pub when: When,
    pub ops: &'static [Op],
}

/// A fighter-specific step the generic table cannot express.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Special {
    None,
    /// Sheik: the needle article's visibility group follows the needle count.
    Sheik,
    /// Pac-Man: at rank 12 with the pull/throw flag present, the max-have-item flag is raised.
    PacMan,
    /// Robin: eight values in `robin_charges`.
    Robin,
    /// Incineroar: Revenge with a fresh full timer and the aura on five joints.
    Incineroar,
    /// Hero: the max effect is cleared before the hold frame is written.
    Hero,
    /// Olimar: the Pikmin line (three colours in `int_x`, `int_y`, `int_z`).
    Olimar,
    /// Sora: the current spell (`int_x`), rotated back by the spell-change routine.
    Sora,
}

struct Entry {
    kinds: &'static [C],
    fields: &'static [Field],
    rules: &'static [Rule],
    special: Special,
}

const SAMUS_POS: [f32; 3] = [7.98004, -0.50584, -0.25092];
const SAMUS_ROT: [f32; 3] = [-91.2728, -1.7974, 176.373];
const ZERO: [f32; 3] = [0.0; 3];

const fn follow(effect: &'static str, joint: &'static str, pos: [f32; 3], rot: [f32; 3], attribute: u32, lifetime: i32, handle: Option<C>) -> Op {
    Op::Follow { effect, joint: Joint::Name(joint), pos, rot, attribute, lifetime, handle }
}

/// The effect attribute 32768 the spec gives for Incineroar's aura and Piranha Plant's smoke.
const ATTR_32768: u32 = 32768;

static ENTRIES: [Entry; 21] = [
    Entry {
        kinds: &[C::KindMario],
        fields: &[int(Slot::IntX, C::MarioFluddCharge)],
        rules: &[Rule { when: When::IntIs(Slot::IntX, 80), ops: &[Op::ChargeMax] }],
        special: Special::None,
    },
    Entry {
        kinds: &[C::KindDonkey],
        fields: &[int(Slot::IntX, C::DonkeyPunchCount)],
        rules: &[Rule { when: When::IntIs(Slot::IntX, 110), ops: &[Op::FaceMotion] }],
        special: Special::None,
    },
    Entry {
        kinds: &[C::KindSamus],
        fields: &[int(Slot::IntX, C::SamusShotCount)],
        rules: &[Rule {
            when: When::IntIs(Slot::IntX, 112),
            ops: &[Op::ChargeMax, follow("samus_cshot_max", "armr", SAMUS_POS, SAMUS_ROT, 0, 0, Some(C::SamusEffectHandle))],
        }],
        special: Special::None,
    },
    Entry {
        kinds: &[C::KindSamusD],
        fields: &[int(Slot::IntX, C::SamusShotCount)],
        rules: &[Rule {
            when: When::IntIs(Slot::IntX, 112),
            ops: &[Op::ChargeMax, follow("samusd_cshot_max", "armr", SAMUS_POS, SAMUS_ROT, 0, 0, Some(C::SamusEffectHandle))],
        }],
        special: Special::None,
    },
    Entry {
        kinds: &[C::KindSheik],
        fields: &[int(Slot::IntX, C::SheikNeedleCount)],
        rules: &[Rule { when: When::IntIs(Slot::IntX, 6), ops: &[Op::ChargeMax] }],
        special: Special::Sheik,
    },
    Entry {
        kinds: &[C::KindMewtwo],
        fields: &[int(Slot::IntX, C::MewtwoCharge), int(Slot::IntY, C::MewtwoPrevCharge), flag(C::MewtwoHad)],
        rules: &[Rule {
            when: When::IntIs(Slot::IntY, 120),
            ops: &[
                Op::ChargeMax,
                follow("mewtwo_shadowball_max_hand", "handl", [1.0, 0.5, 0.0], ZERO, 0, -1, Some(C::MewtwoEffectLeft)),
                follow("mewtwo_shadowball_max_hand", "handr", [1.0, 0.5, 0.0], ZERO, 0, -1, Some(C::MewtwoEffectRight)),
            ],
        }],
        special: Special::None,
    },
    Entry {
        kinds: &[C::KindGameWatch],
        fields: &[float(Slot::FloatX, C::GameWatchGauge), float(Slot::FloatY, C::GameWatchAttack)],
        rules: &[
            Rule { when: When::FloatIs(Slot::FloatX, 3.0), ops: &[Op::ChargeMax] },
            // A flash left from a bucket catch is cleared when the gauge is anything else.
            Rule { when: When::FloatIsNot(Slot::FloatX, 3.0), ops: &[Op::RemoveChargeMax] },
        ],
        special: Special::None,
    },
    Entry { kinds: &[C::KindWario], fields: &[int(Slot::IntX, C::WarioGassCount)], rules: &[], special: Special::None },
    Entry {
        kinds: &[C::KindPokemonSquirtle],
        fields: &[int(Slot::IntX, C::SquirtleCharge)],
        rules: &[Rule { when: When::IntIs(Slot::IntX, 45), ops: &[Op::ChargeMax] }],
        special: Special::None,
    },
    Entry {
        kinds: &[C::KindLucario],
        fields: &[int(Slot::IntX, C::LucarioCharge), int(Slot::IntY, C::LucarioPrevCharge), flag(C::LucarioHad)],
        rules: &[Rule {
            when: When::IntIs(Slot::IntY, 90),
            ops: &[
                Op::ChargeMax,
                follow("lucario_hadoudan_max_l", "handl", ZERO, ZERO, 0, -1, Some(C::LucarioEffectLeft)),
                follow("lucario_hadoudan_max_r", "handr", ZERO, ZERO, 0, -1, Some(C::LucarioEffectRight)),
            ],
        }],
        special: Special::None,
    },
    Entry {
        kinds: &[C::KindRob],
        fields: &[float(Slot::FloatX, C::RobBeam), float(Slot::FloatY, C::RobGyro), float(Slot::FloatZ, C::RobBurner)],
        rules: &[Rule { when: When::FloatIs(Slot::FloatY, 90.0), ops: &[Op::ChargeMax] }],
        special: Special::None,
    },
    Entry { kinds: &[C::KindWiiFit], fields: &[float(Slot::FloatX, C::WiiFitBreath)], rules: &[], special: Special::None },
    Entry {
        kinds: &[C::KindPacMan],
        fields: &[int(Slot::IntX, C::PacManRank), flag(C::PacManPullThrow)],
        rules: &[Rule { when: When::IntIs(Slot::IntX, 12), ops: &[Op::ChargeMax] }],
        special: Special::PacMan,
    },
    Entry {
        kinds: &[C::KindRobin],
        fields: &[],
        rules: &[],
        special: Special::Robin,
    },
    Entry {
        kinds: &[C::KindIncineroar],
        fields: &[
            int(Slot::IntX, C::IncineroarRevengeNo),
            float(Slot::FloatX, C::IncineroarRevengeTotal),
            float(Slot::FloatY, C::IncineroarRevengeRate),
            flag(C::IncineroarIsRevenge),
        ],
        rules: &[],
        special: Special::Incineroar,
    },
    Entry {
        kinds: &[C::KindPiranha],
        fields: &[int(Slot::IntX, C::PiranhaCount)],
        rules: &[Rule {
            when: When::IntIs(Slot::IntX, 75),
            ops: &[Op::ChargeMax, follow("packun_poison_max_smoke", "hip", ZERO, ZERO, ATTR_32768, -1, Some(C::PiranhaEffectHandle))],
        }],
        special: Special::None,
    },
    Entry { kinds: &[C::KindHero], fields: &[int(Slot::IntX, C::HeroHoldFrame)], rules: &[], special: Special::Hero },
    Entry { kinds: &[C::KindBanjo], fields: &[int(Slot::IntX, C::BanjoFeathers)], rules: &[], special: Special::None },
    Entry { kinds: &[C::KindOlimar], fields: &[], rules: &[], special: Special::Olimar },
    Entry { kinds: &[C::KindSora], fields: &[int(Slot::IntX, C::SoraMagicKind)], rules: &[], special: Special::Sora },
    Entry {
        kinds: &[C::KindMiiGunner],
        fields: &[int(Slot::IntX, C::GunnerCharge)],
        rules: &[Rule {
            when: When::IntIs(Slot::IntX, 120),
            ops: &[Op::ChargeMax, follow("miigunner_cshot_max", "armr", [6.0, 0.0, 0.0], ZERO, 0, 0, Some(C::GunnerEffectHandle))],
        }],
        special: Special::None,
    },
];

/// Incineroar's five aura joints (hash40 values the spec gives as numbers).
pub const INCINEROAR_AURA_JOINTS: [u64; 5] = [0x90fa00879, 0x9f5af351a, 0x51ba4b748, 0x5e1ab8a2b, 0x8813bc317];
/// A fresh full Revenge timer (the remaining time is deliberately not saved).
pub const REVENGE_TIMER: i32 = 3500;

fn entry_for(ids: &Table, kind: i32) -> Option<&'static Entry> {
    ENTRIES.iter().find(|e| e.kinds.iter().any(|c| ids.is(*c, kind)))
}

const fn kirby_follow_raw(effect: u64, joint: u64, lifetime: i32, handle: Option<C>) -> Op {
    Op::FollowRaw { effect, joint, lifetime, handle }
}

/// Kirby's own copied-ability table (SPEC-state-05 §4.2) for the fighters whose copied charge differs from
/// the fighter's own: other joints and effects, fewer fields. Kirby's `has_charge` slot says whether he has a
/// copy ability, so the copied fighter's own flag (HAD, pull/throw) lives in the record's `copy_charge`.
static KIRBY_ENTRIES: [Entry; 8] = [
    Entry {
        kinds: &[C::KindSamus],
        fields: &[int(Slot::IntX, C::SamusShotCount)],
        rules: &[Rule {
            when: When::IntIs(Slot::IntX, 112),
            ops: &[Op::ChargeMax, follow("samus_cshot_max", "handr", ZERO, ZERO, 0, 0, Some(C::SamusEffectHandle))],
        }],
        special: Special::None,
    },
    Entry {
        kinds: &[C::KindSamusD],
        fields: &[int(Slot::IntX, C::SamusShotCount)],
        rules: &[Rule {
            when: When::IntIs(Slot::IntX, 112),
            ops: &[Op::ChargeMax, follow("samusd_cshot_max", "handr", ZERO, ZERO, 0, 0, Some(C::SamusEffectHandle))],
        }],
        special: Special::None,
    },
    // Kirby has no needle article: the count alone, and the flash at six.
    Entry {
        kinds: &[C::KindSheik],
        fields: &[int(Slot::IntX, C::SheikNeedleCount)],
        rules: &[Rule { when: When::IntIs(Slot::IntX, 6), ops: &[Op::ChargeMax] }],
        special: Special::None,
    },
    Entry {
        kinds: &[C::KindMewtwo],
        fields: &[int(Slot::IntX, C::MewtwoCharge), int(Slot::IntY, C::MewtwoPrevCharge), flag(C::MewtwoHad)],
        rules: &[Rule {
            when: When::IntIs(Slot::IntY, 120),
            ops: &[
                Op::ChargeMax,
                kirby_follow_raw(0x1ac6d446d8, 0x5e008fd84, -1, Some(C::MewtwoEffectLeft)),
                kirby_follow_raw(0x1ac6d446d8, 0x51a07c0e7, -1, Some(C::MewtwoEffectRight)),
            ],
        }],
        special: Special::None,
    },
    Entry {
        kinds: &[C::KindLucario],
        fields: &[int(Slot::IntX, C::LucarioCharge), int(Slot::IntY, C::LucarioPrevCharge), flag(C::LucarioHad)],
        rules: &[Rule {
            when: When::IntIs(Slot::IntY, 90),
            ops: &[
                Op::ChargeMax,
                kirby_follow_raw(0x164bf96ca1, 0x5e008fd84, -1, Some(C::LucarioEffectLeft)),
                kirby_follow_raw(0x16b1f651c2, 0x51a07c0e7, -1, Some(C::LucarioEffectRight)),
            ],
        }],
        special: Special::None,
    },
    // Kirby's next-Pikmin colour state (no troops manager).
    Entry {
        kinds: &[C::KindOlimar],
        fields: &[int(Slot::IntX, C::OlimarPreVariation), int(Slot::IntY, C::OlimarBeforePreVariation)],
        rules: &[],
        special: Special::None,
    },
    Entry { kinds: &[C::KindRob], fields: &[float(Slot::FloatX, C::RobBeam)], rules: &[], special: Special::None },
    Entry {
        kinds: &[C::KindPacMan],
        fields: &[int(Slot::IntX, C::PacManRank), flag(C::PacManPullThrow)],
        rules: &[Rule { when: When::IntIs(Slot::IntX, 12), ops: &[Op::ChargeMax] }],
        special: Special::PacMan,
    },
];

/// Robin's Kirby copy: the thunder kind alone; 3 is the Levin flash on `handl`.
static KIRBY_ROBIN: Entry = Entry {
    kinds: &[C::KindRobin],
    fields: &[int(Slot::IntX, C::RobinThunderKind)],
    rules: &[Rule {
        when: When::IntIs(Slot::IntX, 3),
        ops: &[Op::ChargeMax, kirby_follow_raw(0x12db3e4172, 0x5eb263e0d, -1, None)],
    }],
    special: Special::None,
};

/// The fighters whose own entry is also Kirby's (same fields, same visuals, no flag).
const KIRBY_SHARED: [C; 4] = [C::KindPokemonSquirtle, C::KindWiiFit, C::KindHero, C::KindSora];

fn kirby_entry_for(ids: &Table, copied: i32) -> Option<&'static Entry> {
    if ids.is(C::KindRobin, copied) {
        return Some(&KIRBY_ROBIN);
    }
    KIRBY_ENTRIES
        .iter()
        .find(|e| e.kinds.iter().any(|c| ids.is(*c, copied)))
        .or_else(|| ENTRIES.iter().find(|e| e.kinds.iter().any(|c| KIRBY_SHARED.contains(c) && ids.is(*c, copied))))
}

/// Whether a copied fighter has a charge entry in Kirby's table.
pub fn has_kirby_entry(ids: &Table, copied: i32) -> bool {
    kirby_entry_for(ids, copied).is_some()
}

/// The fighter kinds that have a charge entry here.
pub fn has_entry(ids: &Table, kind: i32) -> bool {
    entry_for(ids, kind).is_some()
}

/// What a Save reads for a fighter of `kind` (an empty `Charge` for any other kind). A field whose
/// constant did not resolve is left absent.
pub fn read(ids: &Table, kind: i32, work: &dyn Work) -> Charge {
    match entry_for(ids, kind) {
        Some(entry) => read_entry(ids, entry, work, false),
        None => Charge::default(),
    }
}

/// What a Save reads for a Kirby (SPEC-state-05 §5.2 step 1): the record's `charge` (`has_charge` is whether
/// he has a copy ability, `int_z` the copied fighter kind, then the copied fighter's counters, §4.2) and,
/// when he has a copy, the record's `copy_charge`: the copied fighter's own flag (HAD, pull/throw) in
/// `has_charge`, present only for the copied fighters that have one.
pub fn read_kirby(ids: &Table, has_copy: bool, copied: i32, work: &dyn Work) -> (Charge, Option<Charge>) {
    let entry = has_copy.then(|| kirby_entry_for(ids, copied)).flatten();
    let mut charge = match entry {
        Some(entry) => read_entry(ids, entry, work, true),
        None => Charge::default(),
    };
    charge.has_charge = Some(has_copy);
    if has_copy {
        charge.int_z = Some(copied);
    }
    let copy_charge = entry.and_then(|entry| {
        let flag = entry.fields.iter().find(|f| f.slot == Slot::Has)?;
        ids.resolved(flag.work).then(|| Charge { has_charge: Some(work.get_flag(ids.get(flag.work))), ..Charge::default() })
    });
    (charge, copy_charge)
}

fn read_entry(ids: &Table, entry: &'static Entry, work: &dyn Work, skip_flags: bool) -> Charge {
    let mut charge = Charge::default();
    for field in entry.fields {
        if skip_flags && field.slot == Slot::Has {
            continue;
        }
        let id = ids.get(field.work);
        if !ids.resolved(field.work) {
            continue;
        }
        match (field.slot, field.ty) {
            (Slot::IntX, Ty::Int) => charge.int_x = Some(work.get_int(id)),
            (Slot::IntY, Ty::Int) => charge.int_y = Some(work.get_int(id)),
            (Slot::IntZ, Ty::Int) => charge.int_z = Some(work.get_int(id)),
            (Slot::FloatX, Ty::Float) => charge.float_x = Some(work.get_float(id)),
            (Slot::FloatY, Ty::Float) => charge.float_y = Some(work.get_float(id)),
            (Slot::FloatZ, Ty::Float) => charge.float_z = Some(work.get_float(id)),
            (Slot::Has, Ty::Flag) => charge.has_charge = Some(work.get_flag(id)),
            _ => {}
        }
    }
    if entry.special == Special::Olimar {
        let [first, second, third] = work.olimar_colours();
        charge.int_x = first;
        charge.int_y = second;
        charge.int_z = third;
    }
    if entry.special == Special::Robin {
        let g = |c: C| ids.get(c);
        if [
            C::RobinThunderKind,
            C::RobinYellow,
            C::RobinRed,
            C::RobinGreen,
            C::RobinPurple,
            C::RobinLevinCharge,
            C::RobinLevinRecharge,
            C::RobinLevinOn,
        ]
        .iter()
        .all(|c| ids.resolved(*c))
        {
            charge.robin = Some(Robin {
                thunder_kind: work.get_int(g(C::RobinThunderKind)),
                yellow_book: work.get_int(g(C::RobinYellow)),
                red_book: work.get_float(g(C::RobinRed)),
                green_book: work.get_int(g(C::RobinGreen)),
                purple_book: work.get_int(g(C::RobinPurple)),
                levin_charge: work.get_int(g(C::RobinLevinCharge)),
                levin_recharge: work.get_int(g(C::RobinLevinRecharge)),
                has_levin: work.get_flag(g(C::RobinLevinOn)),
            });
        }
    }
    charge
}

/// One thing a Load does for a charge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    SetInt(C, i32),
    SetFloat(C, f32),
    SetFlag(C, bool),
    /// A visual.
    Visual(Op),
    /// Sheik: enable the needle article and show the visibility group for this count.
    SheikNeedles(i32),
    /// Incineroar's aura on one joint.
    RevengeAura(u64),
    /// Olimar: respawn the Pikmin in this colour order (each present colour spawns in turn).
    OlimarLine([Option<i32>; 3]),
    /// Sora: make this the current spell (writes `(saved + 1) mod 3`, raises the change flag, calls the
    /// game's spell-change routine, which advances one spell and lands on the saved one).
    SoraSpell(i32),
}

/// A plan: at most `CAPACITY` steps, no allocation.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    steps: [Option<Step>; Plan::CAPACITY],
    len: usize,
}

impl Default for Plan {
    fn default() -> Self {
        Self { steps: [None; Plan::CAPACITY], len: 0 }
    }
}

impl Plan {
    pub const CAPACITY: usize = 24;

    fn push(&mut self, step: Step) {
        if self.len < Self::CAPACITY {
            self.steps[self.len] = Some(step);
            self.len += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Step> + '_ {
        self.steps[..self.len].iter().flatten().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

fn when_holds(when: When, charge: &Charge) -> bool {
    match when {
        When::IntIs(slot, value) => int_of(charge, slot) == Some(value),
        When::FloatIs(slot, value) => float_of(charge, slot).is_some_and(|v| v == value),
        When::FloatIsNot(slot, value) => float_of(charge, slot).is_some_and(|v| v != value),
    }
}

fn int_of(charge: &Charge, slot: Slot) -> Option<i32> {
    match slot {
        Slot::IntX => charge.int_x,
        Slot::IntY => charge.int_y,
        Slot::IntZ => charge.int_z,
        _ => None,
    }
}

fn float_of(charge: &Charge, slot: Slot) -> Option<f32> {
    match slot {
        Slot::FloatX => charge.float_x,
        Slot::FloatY => charge.float_y,
        Slot::FloatZ => charge.float_z,
        _ => None,
    }
}

/// The writes of a Load for a fighter of `kind`, only when the saved kind equals the current kind (the
/// caller checks that, SPEC-state-01 §5.3 step 8c). A field the record does not have is skipped
/// independently (an older save).
pub fn plan_write(ids: &Table, kind: i32, charge: &Charge) -> Plan {
    match entry_for(ids, kind) {
        Some(entry) => plan_entry(entry, charge),
        None => Plan::default(),
    }
}

/// The copied charge a Kirby gets when his copy starts (SPEC-state-05 §5.2 step 4): the record's `int_z`
/// kind is the copied fighter; a record without a hat, or whose copied fighter has no entry, writes nothing.
/// The copied fighter's own flag comes from `copy_charge` (absent in an older record: the flag is not written).
pub fn plan_kirby(ids: &Table, charge: &Charge, copy_charge: Option<&Charge>) -> Plan {
    let Some(copied) = charge.int_z.filter(|_| charge.has_charge == Some(true)) else { return Plan::default() };
    match kirby_entry_for(ids, copied) {
        Some(entry) => {
            let merged = Charge { has_charge: copy_charge.and_then(|c| c.has_charge), ..*charge };
            plan_entry(entry, &merged)
        }
        None => Plan::default(),
    }
}

fn plan_entry(entry: &'static Entry, charge: &Charge) -> Plan {
    let mut plan = Plan::default();
    if entry.special == Special::Hero {
        // The game redraws its own effect from the hold frame: clear the old one first.
        plan.push(Step::Visual(Op::RemoveChargeMax));
        plan.push(Step::SetFlag(C::HeroMaxEffect, false));
    }
    for field in entry.fields {
        match (field.slot, field.ty) {
            (Slot::Has, Ty::Flag) => {
                if let Some(value) = charge.has_charge {
                    plan.push(Step::SetFlag(field.work, value));
                }
            }
            (slot, Ty::Int) => {
                if let Some(value) = int_of(charge, slot) {
                    plan.push(Step::SetInt(field.work, value));
                }
            }
            (slot, Ty::Float) => {
                if let Some(value) = float_of(charge, slot) {
                    plan.push(Step::SetFloat(field.work, value));
                }
            }
            _ => {}
        }
    }
    match entry.special {
        Special::Sheik => {
            if let Some(count) = charge.int_x {
                plan.push(Step::SheikNeedles(count));
            }
        }
        Special::PacMan => {
            if charge.int_x == Some(12) && charge.has_charge.is_some() {
                plan.push(Step::SetFlag(C::PacManMaxHaveItem, true));
            }
        }
        Special::Robin => {
            if let Some(robin) = &charge.robin {
                plan.push(Step::SetInt(C::RobinThunderKind, robin.thunder_kind));
                plan.push(Step::SetInt(C::RobinYellow, robin.yellow_book));
                plan.push(Step::SetFloat(C::RobinRed, robin.red_book));
                plan.push(Step::SetInt(C::RobinGreen, robin.green_book));
                plan.push(Step::SetInt(C::RobinPurple, robin.purple_book));
                plan.push(Step::SetInt(C::RobinLevinCharge, robin.levin_charge));
                plan.push(Step::SetInt(C::RobinLevinRecharge, robin.levin_recharge));
                plan.push(Step::SetFlag(C::RobinLevinOn, robin.has_levin));
                if robin.thunder_kind == 3 {
                    plan.push(Step::Visual(Op::ChargeMax));
                    plan.push(Step::Visual(follow("reflet_thunder_max", "handl", [1.0, 2.0, 0.0], ZERO, 0, -1, None)));
                }
            }
        }
        Special::Incineroar => {
            if charge.has_charge == Some(true) {
                plan.push(Step::SetInt(C::IncineroarRevengeTimer, REVENGE_TIMER));
                for joint in INCINEROAR_AURA_JOINTS {
                    plan.push(Step::RevengeAura(joint));
                }
            }
        }
        Special::Olimar => {
            if charge.int_x.is_some() || charge.int_y.is_some() || charge.int_z.is_some() {
                plan.push(Step::OlimarLine([charge.int_x, charge.int_y, charge.int_z]));
            }
        }
        Special::Sora => {
            if let Some(saved) = charge.int_x {
                plan.push(Step::SoraSpell(saved));
            }
        }
        Special::Hero | Special::None => {}
    }
    for rule in entry.rules {
        if when_holds(rule.when, charge) {
            for op in rule.ops {
                plan.push(Step::Visual(*op));
            }
        }
    }
    plan
}

/// The visibility group of Sheik's needle article for a needle count (SPEC-state-02 §4): 6 is
/// `group_main_default`, 5 to 1 are `group_main_5` to `group_main_1`, anything else `group_main_0`.
pub fn sheik_group(count: i32) -> &'static str {
    match count {
        6 => "group_main_default",
        5 => "group_main_5",
        4 => "group_main_4",
        3 => "group_main_3",
        2 => "group_main_2",
        1 => "group_main_1",
        _ => "group_main_0",
    }
}

/// Sora's `MAGIC_KIND` to write so that the game's change routine, which advances one spell, lands on
/// `saved` (SPEC-state-02 §4): `(saved + 1) mod 3`.
pub fn sora_written_kind(saved: i32) -> i32 {
    (saved + 1).rem_euclid(3)
}

/// The previous Pikmin colour the game chooses the next one from: `v - 1`, and 4 before 0
/// (SPEC-state-02 §5.1).
pub fn prev_colour(v: i32) -> i32 {
    if v > 0 {
        v - 1
    } else {
        4
    }
}

/// The three colours in line order: held Pikmin first, then the following ones, at most three; absent
/// when fewer exist.
pub fn line_order(held: &[Option<i32>], following: &[Option<i32>]) -> [Option<i32>; 3] {
    let mut out = [None; 3];
    for (slot, colour) in out.iter_mut().zip(held.iter().chain(following.iter()).flatten()) {
        *slot = Some(*colour);
    }
    out
}

/// What the Pikmin respawn does for a line: for the active count 0, 1, 2 in turn, the colour to spawn
/// (its two previous colours are written first). A colour that is absent leaves that position empty.
pub fn spawn_colour(active: i32, line: &[Option<i32>; 3]) -> Option<i32> {
    usize::try_from(active).ok().and_then(|i| line.get(i).copied().flatten())
}
