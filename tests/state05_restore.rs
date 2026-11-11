//! SPEC-state-05 §5.1 (Pokemon Trainer decide mapping and wait) and §5.2 (Kirby's acceptance rule).

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::collections::HashMap;

use model::state::charge::{self, Joint, Op, Step, Work};
use model::state::ids::{Table, C};
use model::state::pokemon::{
    decide_index, kirby_accepts, sound_silenced, switch_effect_hidden, switching_frame_is_dead, Descriptor, Kinds, KirbyKinds, PtWait, Sound,
    SwitchEffect, PT_WAIT_FRAMES,
};
use model::state::record::Charge;

const KINDS: Kinds = Kinds { squirtle: 0x24, ivysaur: 0x25, charizard: 0x26 };

fn descriptor(kinds: [u32; 3]) -> Option<Descriptor> {
    Some(Descriptor { count: 3, kinds })
}

#[test]
fn state05_pt_decide_mapping() {
    // The trainer starts with Squirtle: the index is the kind's offset (what upstream wrote).
    assert_eq!(decide_index(0x25, descriptor([0x24, 0x25, 0x26]), KINDS), Some(1));
    // Starting with Ivysaur (Ivysaur, Charizard, Squirtle): Ivysaur is index 0, Squirtle 2.
    assert_eq!(decide_index(0x25, descriptor([0x25, 0x26, 0x24]), KINDS), Some(0));
    assert_eq!(decide_index(0x24, descriptor([0x25, 0x26, 0x24]), KINDS), Some(2));
    // A record not saved as a Pokemon leaves the register alone.
    assert_eq!(decide_index(0x07, descriptor([0x24, 0x25, 0x26]), KINDS), None);
    assert_eq!(decide_index(-1, descriptor([0x24, 0x25, 0x26]), KINDS), None);
    // A descriptor that is not exactly a rotation, or has another count, or is missing: untouched.
    assert_eq!(decide_index(0x25, descriptor([0x24, 0x26, 0x25]), KINDS), None);
    assert_eq!(decide_index(0x25, Some(Descriptor { count: 2, kinds: [0x24, 0x25, 0x26] }), KINDS), None);
    assert_eq!(decide_index(0x25, None, KINDS), None);
    assert_eq!(decide_index(0x25, descriptor([0x24, 0x24, 0x24]), KINDS), None);
}

#[test]
fn state05_pt_wait_sequence() {
    let mut wait = PtWait::new();
    // The trainer in RUN gets its rate back at once; the wait lasts five frames, then an airborne trainer is
    // grounded and forced to RUN.
    for frame in 0..PT_WAIT_FRAMES {
        let out = wait.step(frame >= 2, true);
        assert!(!out.done && !out.ground_and_run, "frame {frame}");
        assert_eq!(out.rate_one, frame >= 2);
    }
    let last = wait.step(true, true);
    assert!(last.done && last.ground_and_run && last.rate_one);
    // A grounded trainer is only released.
    let mut wait = PtWait::new();
    for _ in 0..PT_WAIT_FRAMES {
        wait.step(false, false);
    }
    let last = wait.step(true, false);
    assert!(last.done && !last.ground_and_run);
    // The counter restarts for the next load.
    assert!(!wait.step(false, false).done);
}

#[test]
fn state05_kirby_accept_rule() {
    let kinds = KirbyKinds { pokemon: KINDS, pyra: 0x5a, mythra: 0x5b, none: -1 };
    // Equal kinds.
    assert!(kirby_accepts(0x03, 0x03, false, &kinds));
    assert!(!kirby_accepts(0x03, 0x07, false, &kinds));
    // The Pokemon family: Squirtle hat while Charizard is out; an opponent that reads -1 counts when its
    // trainer link is valid.
    assert!(kirby_accepts(0x24, 0x26, false, &kinds));
    assert!(kirby_accepts(0x24, -1, true, &kinds));
    assert!(!kirby_accepts(0x24, -1, false, &kinds));
    // Pyra and Mythra.
    assert!(kirby_accepts(0x5a, 0x5b, false, &kinds));
    assert!(!kirby_accepts(0x5a, 0x07, false, &kinds));
}

#[test]
fn state05_pt_dead_test() {
    assert!(switching_frame_is_dead(true, true, true));
    // The frame after an L+R+A reset: the trainer is not restarting.
    assert!(!switching_frame_is_dead(true, true, false));
    assert!(!switching_frame_is_dead(true, false, true));
    assert!(!switching_frame_is_dead(false, true, true));
}

#[test]
fn state05_pt_suppressions() {
    // The flying plate is hidden always; the change light only while a load runs.
    assert!(switch_effect_hidden(SwitchEffect::FlyingPlate, false));
    assert!(switch_effect_hidden(SwitchEffect::ChangeLight, true));
    assert!(!switch_effect_hidden(SwitchEffect::ChangeLight, false));
    assert!(!switch_effect_hidden(SwitchEffect::Other, true));
    // The trainer's switch sounds.
    assert!(sound_silenced(Sound::TrainerSwitch, true, false, false));
    assert!(!sound_silenced(Sound::TrainerSwitch, false, false, false));
}

#[test]
fn state05_sfx_status_rules() {
    // Kirby's copy sound is silent while loading unless he is really drinking.
    assert!(sound_silenced(Sound::KirbyCopy, true, false, false));
    assert!(!sound_silenced(Sound::KirbyCopy, true, true, false));
    assert!(!sound_silenced(Sound::KirbyCopy, false, false, false));
    // Sora's jingle is silent below his neutral special statuses only.
    assert!(sound_silenced(Sound::SoraSpell, true, false, false));
    assert!(!sound_silenced(Sound::SoraSpell, true, false, true));
    assert!(!sound_silenced(Sound::SoraSpell, false, false, false));
    assert!(!sound_silenced(Sound::Other, true, false, false));
}

#[derive(Default)]
struct Mock {
    ints: HashMap<i32, i32>,
    floats: HashMap<i32, f32>,
    flags: HashMap<i32, bool>,
}

impl Work for Mock {
    fn get_int(&self, id: i32) -> i32 {
        self.ints.get(&id).copied().unwrap_or(-12345)
    }
    fn get_float(&self, id: i32) -> f32 {
        self.floats.get(&id).copied().unwrap_or(-1.5)
    }
    fn get_flag(&self, id: i32) -> bool {
        self.flags.get(&id).copied().unwrap_or(false)
    }
}

fn ids() -> Table {
    Table::sequential(1000)
}

fn kind(c: C) -> i32 {
    ids().get(c)
}

#[test]
fn state05_kirby_charge_table() {
    let ids = ids();
    let id = |c: C| ids.get(c);
    let mut work = Mock::default();
    work.ints.insert(id(C::SamusShotCount), 112);
    work.ints.insert(id(C::MewtwoCharge), 40);
    work.ints.insert(id(C::MewtwoPrevCharge), 120);
    work.flags.insert(id(C::MewtwoHad), true);
    work.ints.insert(id(C::RobinThunderKind), 3);
    work.floats.insert(id(C::RobBeam), 0.75);
    work.ints.insert(id(C::OlimarPreVariation), 2);
    work.ints.insert(id(C::OlimarBeforePreVariation), 1);

    // No copy ability: only the flag, no fields, nothing to write.
    let (none, none_copy) = charge::read_kirby(&ids, false, kind(C::KindSamus), &work);
    assert_eq!(none, Charge { has_charge: Some(false), ..Charge::default() });
    assert_eq!(none_copy, None);
    assert!(charge::plan_kirby(&ids, &none, None).is_empty());

    // Samus: the shot count and the flag and copied kind; 112 shows the flash on handr. Samus has no
    // flag of her own, so no `copy_charge`.
    let (samus, samus_copy) = charge::read_kirby(&ids, true, kind(C::KindSamus), &work);
    assert_eq!((samus.int_x, samus.int_z, samus.has_charge), (Some(112), Some(kind(C::KindSamus)), Some(true)));
    assert_eq!(samus_copy, None);
    let steps: Vec<Step> = charge::plan_kirby(&ids, &samus, None).iter().collect();
    assert_eq!(steps[0], Step::SetInt(C::SamusShotCount, 112));
    assert_eq!(steps[1], Step::Visual(Op::ChargeMax));
    match steps[2] {
        Step::Visual(Op::Follow { effect, joint, pos, handle, .. }) => {
            assert_eq!((effect, joint, pos, handle), ("samus_cshot_max", Joint::Name("handr"), [0.0; 3], Some(C::SamusEffectHandle)));
        }
        other => panic!("unexpected {other:?}"),
    }

    // Mewtwo: the two counters, the raw effects at 120. Kirby's `has_charge` is the hat; Mewtwo's own HAD flag
    // is saved apart in `copy_charge` and written back from it.
    let (mewtwo, mewtwo_copy) = charge::read_kirby(&ids, true, kind(C::KindMewtwo), &work);
    assert_eq!((mewtwo.int_x, mewtwo.int_y, mewtwo.has_charge, mewtwo.int_z), (Some(40), Some(120), Some(true), Some(kind(C::KindMewtwo))));
    assert_eq!(mewtwo_copy, Some(Charge { has_charge: Some(true), ..Charge::default() }));
    let steps: Vec<Step> = charge::plan_kirby(&ids, &mewtwo, mewtwo_copy.as_ref()).iter().collect();
    assert!(steps.contains(&Step::SetFlag(C::MewtwoHad, true)));
    // An older record has no `copy_charge`: the flag is not written.
    let older: Vec<Step> = charge::plan_kirby(&ids, &mewtwo, None).iter().collect();
    assert!(!older.iter().any(|s| matches!(s, Step::SetFlag(..))));
    // A flag read as false is saved and written as false.
    let mut no_had = Mock::default();
    no_had.ints.insert(id(C::MewtwoCharge), 40);
    let (_, off_copy) = charge::read_kirby(&ids, true, kind(C::KindMewtwo), &no_had);
    assert_eq!(off_copy, Some(Charge { has_charge: Some(false), ..Charge::default() }));
    let raws: Vec<(u64, u64)> = steps
        .iter()
        .filter_map(|s| match s {
            Step::Visual(Op::FollowRaw { effect, joint, .. }) => Some((*effect, *joint)),
            _ => None,
        })
        .collect();
    assert_eq!(raws, vec![(0x1ac6d446d8, 0x5e008fd84), (0x1ac6d446d8, 0x51a07c0e7)]);

    // Robin: the thunder kind alone; 3 shows the Levin flash.
    let (robin, _) = charge::read_kirby(&ids, true, kind(C::KindRobin), &work);
    assert_eq!((robin.int_x, robin.robin), (Some(3), None));
    let steps: Vec<Step> = charge::plan_kirby(&ids, &robin, None).iter().collect();
    assert_eq!(steps[0], Step::SetInt(C::RobinThunderKind, 3));
    assert!(steps.iter().any(|s| matches!(s, Step::Visual(Op::FollowRaw { effect: 0x12db3e4172, joint: 0x5eb263e0d, lifetime: -1, .. }))));

    // R.O.B.: one float; Olimar: the two colour ints; no troops manager.
    let (rob, _) = charge::read_kirby(&ids, true, kind(C::KindRob), &work);
    assert_eq!((rob.float_x, rob.float_y, rob.float_z), (Some(0.75), None, None));
    let (olimar, _) = charge::read_kirby(&ids, true, kind(C::KindOlimar), &work);
    assert_eq!((olimar.int_x, olimar.int_y, olimar.int_z), (Some(2), Some(1), Some(kind(C::KindOlimar))));
    let steps: Vec<Step> = charge::plan_kirby(&ids, &olimar, None).iter().collect();
    assert_eq!(steps, vec![Step::SetInt(C::OlimarPreVariation, 2), Step::SetInt(C::OlimarBeforePreVariation, 1)]);

    // A fighter without a Kirby entry (Mario) has the flag and the kind only.
    let (mario, mario_copy) = charge::read_kirby(&ids, true, kind(C::KindMario), &work);
    assert_eq!((mario.int_x, mario.int_z, mario_copy), (None, Some(kind(C::KindMario)), None));
    assert!(charge::plan_kirby(&ids, &mario, None).is_empty());
    // A fighter that shares its own entry: Squirtle's 45 shows the flash.
    let mut work = Mock::default();
    work.ints.insert(id(C::SquirtleCharge), 45);
    let (squirtle, _) = charge::read_kirby(&ids, true, kind(C::KindPokemonSquirtle), &work);
    let steps: Vec<Step> = charge::plan_kirby(&ids, &squirtle, None).iter().collect();
    assert_eq!(steps, vec![Step::SetInt(C::SquirtleCharge, 45), Step::Visual(Op::ChargeMax)]);
}
