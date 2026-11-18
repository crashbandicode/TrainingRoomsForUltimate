lib_unit_tests! {
use super::*;

// rc15-sora116-tick-trace-xps-1 gen 10 (DECODE.json): seed remaining 18.648, auto 1, manual 4.
fn record(remaining: f32) -> Record {
    Record { remaining, automatic: 1.0, manual: 4.0, button_mul: 1.8, threshold: 0.5, sign: [0, 0],
        source: -1, stick_block: 0, button_block: 0, accepted: 0, other_block: 0,
        cross_reload: 4, other_reload: 3 }
}

const D_MAX: f32 = 120.0;

fn set(seq: u32, old: f32, new: f32) -> ClatterWrite {
    ClatterWrite { generation: 7, seq, slot: Slot::Set, tag: TAG_ICE_MAGIC_116, old: old.to_bits(),
        operand: new.to_bits(), new: new.to_bits(), frame: 14395, d_max: D_MAX.to_bits() }
}

fn held_to_first_set() -> Clock {
    let mut clock = Clock::new(record(18.648)).unwrap();
    for remaining in [17.648, 16.648, 15.648_001] {
        clock.observe(record(remaining), Input::default(), false).unwrap();
    }
    clock
}

#[test]
fn the_recorded_116_refreeze_is_the_current_mismatch_without_writes() {
    // RED receipt: `mismatch gen=10 t=3 … remaining: 15.648001 … live … 23.09` → Reinitialized today.
    let mut clock = held_to_first_set();
    assert_eq!(clock.observe(record(23.09), Input::default(), false), Err(ClockError::Reinitialized));
}

#[test]
fn an_observed_set_before_the_tick_rebases_bit_exactly() {
    let mut clock = held_to_first_set();
    let write = set(1, 15.648_001, 24.09);
    assert_eq!(clock.observe_writes(record(23.09), Input::default(), false, &[write]), Ok(()));
    assert_eq!((clock.elapsed(), clock.last().remaining.to_bits()), (4, 23.09f32.to_bits()));
}

#[test]
fn an_observed_set_after_the_tick_rebases_and_five_shards_chain() {
    let mut clock = held_to_first_set();
    clock.observe_writes(record(23.09), Input::default(), false, &[set(1, 15.648_001, 24.09)]).unwrap();
    // The next four shards of the same Blizzard (DECODE.json gen 10 new values), each after one tick.
    let mut model = 23.09f32;
    for (seq, new) in [(2, 28.085_51f32), (3, 29.329_878), (4, 30.325_947), (5, 29.435_383)] {
        let old = model - 1.0;
        clock.observe_writes(record(new), Input::default(), false, &[set(seq, old, new)]).unwrap();
        model = new;
    }
    assert_eq!((clock.elapsed(), clock.last().remaining.to_bits()), (8, 29.435_383f32.to_bits()));
}

#[test]
fn a_write_whose_old_is_not_bit_equal_is_a_mismatch_never_an_inferred_write() {
    for old in [15.648f32 + 0.001, 14.0, f32::from_bits(15.648_001f32.to_bits() + 1)] {
        let mut clock = held_to_first_set();
        let before = clock.last();
        assert_eq!(clock.observe_writes(record(23.09), Input::default(), false, &[set(1, old, 24.09)]),
            Err(ClockError::WriteRefused), "{old}");
        assert_eq!((clock.last(), clock.elapsed()), (before, 3), "no state change on refusal");
    }
    // A bit-exact write that does not explain the live value is still a transition mismatch.
    let mut clock = held_to_first_set();
    assert_eq!(clock.observe_writes(record(23.5), Input::default(), false, &[set(1, 15.648_001, 24.09)]),
        Err(ClockError::TransitionMismatch));
}

#[test]
fn every_poison_and_every_foreign_event_is_refused() {
    for poison in [Poison::Overflow, Poison::NewMismatch, Poison::Nested, Poison::WrongSlot, Poison::Stale, Poison::Lost] {
        let mut writes = Writes::new(7, TAG_ICE_MAGIC_116).unwrap();
        assert_eq!(writes.accept(Poll::Poisoned(poison)), Err(ClockError::WriteRefused), "{poison:?}");
    }
    let good = set(1, 15.648_001, 24.09);
    let mut writes = Writes::new(7, TAG_ICE_MAGIC_116).unwrap();
    assert_eq!(writes.accept(Poll::Empty), Ok(None));
    // v1.1: an event of another generation is stale: refused (ignored), the current attempt is not poisoned.
    assert_eq!(writes.accept(Poll::Event(ClatterWrite { generation: 6, ..good })), Ok(None));
    let cases = [
        ClatterWrite { tag: TAG_FROZEN_113, ..good },
        ClatterWrite { slot: Slot::Add, ..good },
        ClatterWrite { seq: 2, ..good },
        ClatterWrite { new: 24.0f32.to_bits(), ..good },
        ClatterWrite { operand: (-1.0f32).to_bits(), new: (-1.0f32).to_bits(), ..good },
    ];
    for bad in cases {
        let mut writes = Writes::new(7, TAG_ICE_MAGIC_116).unwrap();
        assert_eq!(writes.accept(Poll::Event(bad)), Err(ClockError::WriteRefused), "{bad:?}");
    }
    assert_eq!(writes.accept(Poll::Event(good)), Ok(Some(good)));
    assert_eq!(writes.accept(Poll::Event(good)), Err(ClockError::WriteRefused), "a replayed seq is a gap");
    assert_eq!(writes.accept(Poll::Event(ClatterWrite { seq: 2, ..good })), Ok(Some(ClatterWrite { seq: 2, ..good })));
    // Set clamps at zero (`new == max(operand, 0)`); Add is old + operand.
    let mut writes = Writes::new(7, TAG_ICE_MAGIC_116).unwrap();
    let clamped = ClatterWrite { operand: (-1.0f32).to_bits(), new: 0.0f32.to_bits(), ..good };
    assert_eq!(writes.accept(Poll::Event(clamped)), Ok(Some(clamped)));
    let mut adds = Writes::new(7, TAG_EGG_117).unwrap();
    let add = ClatterWrite { slot: Slot::Add, tag: TAG_EGG_117, old: 98.0f32.to_bits(), operand: 15.0f32.to_bits(),
        new: 113.0f32.to_bits(), ..good };
    assert_eq!(adds.accept(Poll::Event(add)), Ok(Some(add)));
}

#[test]
fn new_mismatch_uses_arm_fmax_exactly() {
    assert_eq!(fmax_zero(-0.0).to_bits(), 0.0f32.to_bits(), "max(-0, +0) = +0");
    assert_eq!(fmax_zero(-3.0).to_bits(), 0.0f32.to_bits());
    assert!(fmax_zero(f32::NAN).is_nan());
    let good = set(1, 15.648_001, 24.09);
    for (operand, new, ok) in [(-0.0f32, 0.0f32, true), (-0.0, -0.0, false), (f32::NAN, 0.0, false), (f32::NAN, f32::NAN, false)] {
        let mut writes = Writes::new(7, TAG_ICE_MAGIC_116).unwrap();
        let write = ClatterWrite { operand: operand.to_bits(), new: new.to_bits(), ..good };
        assert_eq!(writes.accept(Poll::Event(write)).is_ok(), ok, "{operand} -> {new}");
    }
    // Add: binary32 round-to-nearest fadd, then fmax.
    let add = ClatterWrite { slot: Slot::Add, tag: TAG_EGG_117, old: 0.1f32.to_bits(), operand: 0.2f32.to_bits(),
        new: (0.1f32 + 0.2f32).to_bits(), ..good };
    assert_eq!(Writes::new(7, TAG_EGG_117).unwrap().accept(Poll::Event(add)), Ok(Some(add)));
    assert_eq!([Slot::Set as u16, Slot::Add as u16], [0x90, 0x98]);
}

#[test]
fn tags_map_to_exactly_one_slot() {
    assert_eq!([113, 116, 117, 40].map(slot_for), [Some(Slot::Set), Some(Slot::Set), Some(Slot::Add), None]);
    assert_eq!(Writes::new(1, 40), None);
}

#[test]
fn d_max_must_be_a_valid_constant_cap_aux_invalid() {
    let good = set(1, 15.648_001, 24.09);
    assert_eq!(Writes::new(7, TAG_ICE_MAGIC_116).unwrap().accept(Poll::Poisoned(Poison::AuxInvalid)), Err(ClockError::WriteRefused));
    for d_max in [f32::NAN, f32::INFINITY, 0.0, -0.0, -1.0, 24.0] {
        let mut writes = Writes::new(7, TAG_ICE_MAGIC_116).unwrap();
        assert_eq!(writes.accept(Poll::Event(ClatterWrite { d_max: d_max.to_bits(), ..good })), Err(ClockError::WriteRefused), "{d_max}");
    }
    let mut writes = Writes::new(7, TAG_ICE_MAGIC_116).unwrap();
    assert!(writes.accept(Poll::Event(good)).is_ok());
    let second = ClatterWrite { seq: 2, old: 22.09f32.to_bits(), operand: 28.085_51f32.to_bits(), new: 28.085_51f32.to_bits(), ..good };
    assert_eq!(writes.accept(Poll::Event(ClatterWrite { d_max: 121.0f32.to_bits(), ..second })), Err(ClockError::WriteRefused),
        "d_max changing within an attempt");
    assert!(writes.accept(Poll::Event(second)).is_ok());
    // An operand at the cap is a real native write for G1, but G2 cannot bracket it: UNSCORED.
    let capped = ClatterWrite { operand: D_MAX.to_bits(), new: D_MAX.to_bits(), ..good };
    assert!(Writes::new(7, TAG_ICE_MAGIC_116).unwrap().accept(Poll::Event(capped)).is_ok());
    assert_eq!(g2::Schedule::new(&[capped]), None);
}

#[test]
fn recorded_sets_bracket_their_native_ice_frames() {
    // DECODE.json gen 10 (116): (old, operand, get_ice_frame) per Set, and 113's E-A Set (C1-RESULT −3/−5).
    let sets = [(15.648_001f32, 24.09f32, 8.442_f32), (22.09, 28.085_51, 5.995_509), (25.085_51, 29.329_878, 4.244_367_6),
        (27.329_878, 30.325_947, 2.996_069), (27.325_947, 29.435_383, 2.109_435_8), (34.278_004, 57.555_145, 23.277_143)];
    for (old, operand, ice) in sets {
        assert_eq!((old + ice).to_bits(), operand.to_bits(), "fixture: binary32 old + ice == operand");
        let (lo, hi) = g2::admissible(old, operand).unwrap();
        assert!(lo <= ice && ice <= hi, "{old} {operand}: [{lo}, {hi}] brackets {ice}");
        for f in [lo, hi] {
            assert_eq!((old + f).to_bits(), operand.to_bits());
        }
        assert_ne!((old + f32::from_bits(lo.to_bits() - 1)).to_bits(), operand.to_bits(), "lo is the lowest");
        assert_ne!((old + f32::from_bits(hi.to_bits() + 1)).to_bits(), operand.to_bits(), "hi is the highest");
    }
    assert_eq!(g2::admissible(24.0, 24.0), None, "no positive F leaves the value unchanged");
    assert_eq!(g2::admissible(24.0, 20.0), None);
    let writes: Vec<_> = sets[..5].iter().enumerate()
        .map(|(k, &(old, operand, _))| ClatterWrite { seq: k as u32 + 1, frame: 14395 + k as u32, ..set(1, old, operand) })
        .collect();
    let schedule = g2::Schedule::new(&writes).unwrap();
    assert_eq!((schedule.len, schedule.frames[4]), (5, 14399));
}

#[test]
fn g2_grades_only_when_both_range_ends_agree_and_no_cap_binds() {
    let writes: Vec<_> = [(15.648_001f32, 24.09f32), (22.09, 28.085_51)].iter().enumerate()
        .map(|(k, &(old, operand))| ClatterWrite { seq: k as u32 + 1, ..set(1, old, operand) }).collect();
    let schedule = g2::Schedule::new(&writes).unwrap();
    // A cadence with 1/frame automatic decay that receives every Set: its tick count is insensitive to F's ulps.
    let ticks = |s: &g2::Schedule, f: &[f32]| {
        let mut cf = 18.648f32;
        let mut n = 0u32;
        for &fk in f {
            cf = s.apply(fk, cf)?;
        }
        while cf > 0.0 {
            cf = (cf - 1.0).max(0.0);
            n += 1;
        }
        Some(n)
    };
    assert!(schedule.replay(ticks).is_some(), "insensitive -> graded");
    assert_eq!(schedule.replay(|_, f: &[f32]| Some(f[0].to_bits())), None, "sensitive -> unscored");
    // Counterfactual cap: a sum at or above d_max is UNSCORED; an escaped cadence (<= 0) is never extended.
    assert_eq!(schedule.apply(schedule.lo[0], D_MAX - 1.0), None);
    assert_eq!(schedule.apply(schedule.lo[0], 0.0), Some(0.0));
    assert_eq!(schedule.replay(|s, f: &[f32]| s.apply(f[0], D_MAX - 1.0)), None);
    assert_eq!(g2::Schedule::new(&[]), None);
    let mismatched = [writes[0], ClatterWrite { d_max: 121.0f32.to_bits(), ..writes[1] }];
    assert_eq!(g2::Schedule::new(&mismatched), None);
}
}
