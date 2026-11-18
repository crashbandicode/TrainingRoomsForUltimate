lib_unit_tests! {
use super::*;
use crate::model::mashout::native::Record;

const D_MAX: f32 = 120.0;

// rc15-sora116-tick-trace-xps-1 gen 10: seed remaining 18.648, automatic 1, manual 4, button_mul 1.8.
fn seed(remaining: f32) -> Seed {
    Seed { record: Record { remaining, automatic: 1.0, manual: 4.0, button_mul: 1.8, threshold: 0.5, sign: [0, 0],
        source: -1, stick_block: 0, button_block: 0, accepted: 0, other_block: 0, cross_reload: 4, other_reload: 3 },
        held: 0, deadline: None }
}

fn applied(boundary: u32, old: f32, operand: f32) -> Applied {
    Applied { boundary, old: old.to_bits(), operand: operand.to_bits(), new: fmax_zero(operand).to_bits(), d_max: D_MAX.to_bits() }
}

fn solve_plain(seed: Seed) -> Bounds {
    let mut solver = Solver::new(seed).unwrap();
    loop {
        if let Some(result) = solver.advance(64) {
            return result.unwrap();
        }
    }
}

// The 5 recorded gen-10 Sets (DECODE.json) at the G1 boundaries of their frames (14395 + 3 ticks before the first).
fn gen10() -> [Applied; 5] {
    [applied(3, 15.648_001, 24.09), applied(6, 22.09, 28.085_51), applied(13, 25.085_51, 29.329_878),
        applied(15, 27.329_878, 30.325_947), applied(19, 27.325_947, 29.435_383)]
}

#[test]
fn no_observed_set_is_the_plain_solver() {
    for remaining in [18.648f32, 31.5, 52.920_006] {
        assert_eq!(bounds(seed(remaining), &[]), Ok(solve_plain(seed(remaining))), "C2 bypass");
        let empty = Solver::scheduled_capped(seed(remaining), &[], D_MAX).and_then(solve).unwrap();
        assert_eq!(empty, solve_plain(seed(remaining)), "empty-schedule parity of the capped solver");
    }
}

#[test]
fn the_recorded_blizzard_is_insensitive_to_its_unpublished_ice_frames() {
    let b = bounds(seed(18.648), &gen10()).unwrap();
    let plain = solve_plain(seed(18.648));
    assert!(b.fastest > plain.fastest && b.no_input > plain.no_input, "{b:?} vs {plain:?}: re-freezes extend both");
    // Oracle for the no-input cadence: 1 per tick, each Set adds its F at its boundary (any F in range: insensitive).
    let mut cf = 18.648f32;
    let mut sets = gen10().into_iter().peekable();
    let mut ticks = 0;
    while cf > 0.0 {
        while let Some(w) = sets.next_if(|w| w.boundary == ticks) {
            let (f, _) = g2::admissible(f32::from_bits(w.old), f32::from_bits(w.operand)).unwrap();
            cf = fmax_zero(cf + f);
        }
        cf = (cf - 1.0).max(0.0);
        ticks += 1;
    }
    assert_eq!(b.no_input, Some(ticks), "oracle parity");
    assert!(matches!(judge(Ok(b), b.fastest, true), Ok(Outcome::Graded(_))));
    assert_eq!(judge(Ok(b), b.fastest, false), Err(Unscored::ProviderOpen), "S1");
}

#[test]
fn a_grade_that_depends_on_the_ice_frame_is_unscored() {
    // No-input baseline at boundary 3 (37.648); the user's actual old is lower after real presses. Search a write whose
    // bracket [F_lo, F_hi] puts the baseline sum on both sides of an integer, so the no-input tick count differs.
    let base = 40.648f32 - 3.0;
    let mut found = None;
    'search: for old in [base - 7.2, base - 14.4, base - 21.6, base - 28.8, base - 36.0] {
        let mut operand = f32::from_bits((old + (46.0 - base)).to_bits() - 4096);
        for _ in 0..8192 {
            if let Some((lo, hi)) = g2::admissible(old, operand) {
                let edge = (base + lo).ceil();
                if base + lo <= edge && base + hi > edge {
                    found = Some((old, operand));
                    break 'search;
                }
            }
            operand = f32::from_bits(operand.to_bits() + 1);
        }
    }
    let (old, operand) = found.expect("a straddling write exists");
    assert_eq!(bounds(seed(40.648), &[applied(3, old, operand)]), Err(Unscored::Sensitive));
}

#[test]
fn caps_and_aux_invalid_are_unscored() {
    let w = applied(3, 15.648_001, 24.09);
    // Actual cap: the operand is d_max itself.
    assert_eq!(bounds(seed(18.648), &[Applied { d_max: 24.09f32.to_bits(), ..w }]), Err(Unscored::Cap));
    // Counterfactual cap: the actual write is below d_max, a cadence's replayed sum is not.
    let low_actual = applied(3, 11.648, 20.09);
    assert_eq!(bounds(seed(18.648), &[Applied { d_max: 22.0f32.to_bits(), ..low_actual }]), Err(Unscored::Cap));
    assert!(bounds(seed(18.648), &[low_actual]).is_ok(), "uncapped path grades");
    for d_max in [f32::NAN, f32::INFINITY, 0.0, -1.0, 20.0] {
        assert_eq!(bounds(seed(18.648), &[Applied { d_max: d_max.to_bits(), ..w }]), Err(Unscored::Aux), "{d_max}");
    }
    let changing = [w, Applied { d_max: 121.0f32.to_bits(), ..applied(6, 22.09, 28.085_51) }];
    assert_eq!(bounds(seed(18.648), &changing), Err(Unscored::Aux), "d_max changing within the attempt");
    assert_eq!(bounds(seed(18.648), &[Applied { new: 24.0f32.to_bits(), ..w }]), Err(Unscored::Writes));
    assert_eq!(bounds(seed(18.648), &[applied(3, 24.0, 20.0)]), Err(Unscored::Range));
    assert_eq!(bounds(seed(18.648), &[w; 9]), Err(Unscored::Writes));
    let unordered = [applied(6, 22.09, 28.085_51), w];
    assert_eq!(bounds(seed(18.648), &unordered), Err(Unscored::Writes));
}
}
