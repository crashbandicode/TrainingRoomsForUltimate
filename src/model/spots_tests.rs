//! Decisions SP1 and SP2 (rc.14): which hitbox is sweet, sour or late, which one a hit connected, and the *Frame data colours*.
//! Damages are drawn for the cases (a tip of 16 over blades of 13); the plugin reads them from the game.

lib_unit_tests! {
use super::*;
use crate::model::hitbox::{self, AttackReader, FrameFacts, HandleSet, Sphere, StatusIds};
use crate::model::hitbox_orbs::OrbSink;

/// The damages of slots 0.. (the rest inactive).
fn p(values: &[f32]) -> Powers {
    let mut out = [0.0; SLOTS];
    out[..values.len()].copy_from_slice(values);
    out
}

fn the_strongest_hitbox_of_a_frame_is_sweet_and_a_weaker_one_sour() {
    let active = p(&[16.0, 13.0, 13.0, 13.0]);
    assert_eq!(classify(16.0, &active, 0.0), Some(Spot::Sweet));
    assert_eq!(classify(13.0, &active, 0.0), Some(Spot::Sour));
}

fn ties_at_the_top_are_all_sweet() {
    let active = p(&[16.0, 16.0, 13.0]);
    assert_eq!([16.0, 16.0, 13.0].map(|d| classify(d, &active, 0.0)), [Some(Spot::Sweet), Some(Spot::Sweet), Some(Spot::Sour)]);
}

fn equal_hitboxes_and_a_lone_hitbox_have_no_label() {
    for active in [p(&[12.0, 12.0, 12.0]), p(&[12.0]), p(&[12.0, 12.02])] {
        assert_eq!(classify(12.0, &active, 0.0), None, "{active:?}");
    }
    assert_eq!(classify(12.0, &p(&[12.0]), 12.0), None, "an earlier hit as strong is not stronger");
    assert_eq!(classify(16.0, &p(&[16.0]), 13.0), None, "a stronger hit than the earlier ones, with nothing weaker beside it");
    assert_eq!(classify(0.0, &p(&[16.0, 0.0]), 0.0), None, "a slot with no damage has no label");
}

fn a_hitbox_weaker_than_an_earlier_one_of_the_action_is_late() {
    assert_eq!(classify(13.0, &p(&[13.0]), 16.0), Some(Spot::Late));
    // Late wins over Sour: both are weaker than the earlier tip.
    let active = p(&[13.0, 9.0]);
    assert_eq!([13.0, 9.0].map(|d| classify(d, &active, 16.0)), [Some(Spot::Late); 2]);
    // The sweet phase is not over while the tip is still active: the blade is Sour, whatever the earlier peak.
    assert_eq!(classify(13.0, &p(&[16.0, 13.0]), 16.0), Some(Spot::Sour));
    assert_eq!(classify(9.0, &p(&[13.0, 9.0]), 8.0), Some(Spot::Sour), "weaker than the frame's strongest, not than the earlier peak");
}

fn the_peak_starts_over_with_a_new_status_or_when_the_motion_goes_back() {
    let mut peak = Peak::new();
    assert_eq!(peak.earlier(10, 0.0), 0.0);
    peak.fold(&p(&[16.0]));
    assert_eq!(peak.earlier(10, 1.0), 16.0, "the same action");
    peak.fold(&p(&[13.0]));
    assert_eq!(peak.earlier(10, 1.0), 16.0, "a weaker frame keeps the peak, and hitlag holds the motion frame");
    assert_eq!(peak.earlier(10, 9.0), 16.0);
    assert_eq!(peak.earlier(10, 0.0), 0.0, "the same status entered again (the next jab): the motion went back");
    peak.fold(&p(&[8.0]));
    assert_eq!(peak.earlier(11, 1.0), 0.0, "a new status is a new action");
}

/// The measured multiplier: a fresh hit adds 1.26 times its hitbox's damage (Eden, 2026-11-14).
const M: f32 = 1.26;

/// Marth's forward smash as the game reports it: three blade hitboxes of 13 and a tip of 18.
fn marth() -> Powers {
    p(&[13.0, 13.0, 13.0, 18.0])
}

fn a_hit_is_attributed_by_the_damage_it_added() {
    assert_eq!(attribute(18.0 * M, &marth(), 1.0, 0.0), Some(Spot::Sweet));
    assert_eq!(attribute(13.0 * M, &marth(), 1.0, 0.0), Some(Spot::Sour));
    // The hit Eden logged: 16.926 added. The blade's ratio is 1.30; the tip's would be 0.94, below any multiplier the game uses.
    assert_eq!(attribute(16.926, &marth(), 1.0, 0.0), Some(Spot::Sour));
    assert_eq!(attribute(13.0 * M, &p(&[13.0]), 1.0, 16.0), Some(Spot::Late));
}

fn staling_and_multipliers_cancel_in_the_ratio() {
    for m in [1.07, 1.15, M, 1.35] {
        assert_eq!(attribute(13.0 * m, &marth(), 1.0, 0.0), Some(Spot::Sour), "a blade hit at {m}");
    }
    assert_eq!(attribute(18.0 * M, &marth(), 1.0, 0.0), Some(Spot::Sweet), "a fresh tip hit");
    assert_eq!(attribute(18.0 * 1.35, &marth(), 1.0, 0.0), Some(Spot::Sweet), "a slightly charged tip hit");
    // A stale tip hit could also be a blade hit: 20.7 is 1.59 times 13. No label.
    for m in [1.07, 1.15] {
        assert_eq!(attribute(18.0 * m, &marth(), 1.0, 0.0), None, "a tip hit at {m}");
    }
}

fn a_hit_that_cannot_be_told_has_no_label() {
    assert_eq!(attribute(15.5 * M, &p(&[16.0, 15.0]), 1.0, 0.0), None, "two candidates of different classes");
    assert_eq!(attribute(12.0 * M, &p(&[12.0, 12.0, 12.0]), 1.0, 0.0), None, "equal hitboxes");
    assert_eq!(attribute(30.0 * M, &p(&[16.0, 13.0]), 1.0, 0.0), None, "no hitbox could have dealt it");
    for gained in [0.0, -3.0, f32::NAN, f32::INFINITY] {
        assert_eq!(attribute(gained, &marth(), 1.0, 0.0), None, "{gained}");
    }
    assert_eq!(attribute(16.0, &p(&[]), 1.0, 0.0), None, "nothing active");
    // Candidates that agree are still a label: 13 and 12.8 are both Sour, and the tip's ratio (0.91) is below the window.
    assert_eq!(attribute(13.0 * M, &p(&[18.0, 13.0, 12.8]), 1.0, 0.0), Some(Spot::Sour));
}
/// Captain Falcon's forward air as the first Eden boot logged it (spots-4e9f91b5-1): the knee's three phases, damages 14 and 18, then 8 and 9,
/// then 3. The final build's window reads each hit the way the game made it (a fresh hit is 1.26 times the damage).
fn the_falcon_knee_hits_eden_logged() {
    let knee = p(&[14.0, 18.0, 18.0]);
    assert_eq!(attribute(22.68, &knee, 1.0, 0.0), Some(Spot::Sweet), "18 x 1.26");
    assert_eq!(attribute(17.64, &knee, 1.0, 0.0), Some(Spot::Sour), "14 x 1.26; the 18s' ratio is 0.98");
    let middle = p(&[8.0, 9.0, 8.0, 9.0]);
    assert_eq!(attribute(8.736, &middle, 1.0, 0.0), Some(Spot::Sour), "8 x 1.092 (stale); the 9s' ratio is 0.97");
    assert_eq!(attribute(10.08, &middle, 1.0, 0.0), None, "8 x 1.26 or 9 x 1.12: no label");
    // The last frames: 3 and 3 after the 18 of the same action are Late; a lone 3 on its own has no label.
    assert_eq!(attribute(3.213, &p(&[3.0, 3.0]), 1.0, 18.0), Some(Spot::Late));
    assert_eq!(attribute(3.213, &p(&[3.0, 3.0]), 1.0, 0.0), None);
}
/// A smash charged to `mul` (the game's `AttackModule::power_mul_status`, 1.0 uncharged): every slot's damage is multiplied by it, so a hit adds
/// the slot's damage x `mul` x 1.26 and the label follows the hitbox, not the charge (spots-2, 2026-11-14; the lane's known limit was a blade hit of a
/// nearly full charge read as the tip's).
fn a_charged_smash_is_attributed_by_the_hitbox_that_dealt_it() {
    for mul in [1.0, 1.1, 1.2, 1.3, 1.4] {
        assert_eq!(attribute(13.0 * mul * M, &marth(), mul, 0.0), Some(Spot::Sour), "a blade hit at charge {mul}");
        assert_eq!(attribute(18.0 * mul * M, &marth(), mul, 0.0), Some(Spot::Sweet), "a tip hit at charge {mul}");
    }
    // The hit that read as the tip's before: a blade of a full charge adds 22.9, 1.27 times the tip's 18.
    assert_eq!(attribute(13.0 * 1.4 * M, &marth(), 1.4, 0.0), Some(Spot::Sour));
    // Late follows the same rule: a weaker hitbox after the action's strongest, charged.
    assert_eq!(attribute(3.0 * 1.4 * M, &p(&[3.0, 3.0]), 1.4, 18.0), Some(Spot::Late));
}

/// Marth's forward smash held to different charges, as Eden logged it (spots2-d37c0734-1, Yoga, 2026-11-14, `sp-charge`): the game's multiplier
/// is 1 + frames held / 150 up to 1.4 at 60 frames, and each hit added the slot's damage x the multiplier x 1.26. A tip at the first spacing, the blades
/// at the second. Without the multiplier the full-charge blade hit (22.932, 1.27 times the tip's 18) read as the tip's.
fn the_charged_marth_hits_eden_logged() {
    for (gained, mul, expected) in [
        (23.436, 1.0333, Spot::Sweet),
        (16.817, 1.0267, Spot::Sour),
        (27.821, 1.2267, Spot::Sweet),
        (20.311, 1.2400, Spot::Sour),
        (28.123, 1.2400, Spot::Sweet),
        (20.093, 1.2267, Spot::Sour),
        (31.752, 1.4000, Spot::Sweet),
        (22.932, 1.4000, Spot::Sour),
    ] {
        assert_eq!(attribute(gained, &marth(), mul, 0.0), Some(expected), "{gained} added at charge {mul}");
    }
}

/// A charge does not make a hit tellable: a stale tip hit that a blade could also have made is still no label, charged or not.
fn a_charged_hit_that_cannot_be_told_has_no_label() {
    assert_eq!(attribute(18.0 * 1.4 * 1.07, &marth(), 1.4, 0.0), None, "a stale full-charge tip hit: the blade's ratio is 1.48");
}

/// The multiplier is the game's own number; one that was not read (0.0) or is not a usable number labels nothing.
fn an_unusable_multiplier_gives_no_label() {
    for mul in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(attribute(18.0 * M, &marth(), mul, 0.0), None, "multiplier {mul}");
        assert_eq!(attribute(13.0 * M, &marth(), mul, 0.0), None, "multiplier {mul}");
    }
}
/// Slot `i` is active when `0[i]` is `Some(damage)`; its `AttackData` is at `0x1000 * (i + 1)`. Reading an inactive slot panics.
struct Slots([Option<f32>; SLOTS]);

impl AttackReader for Slots {
    fn active(&mut self, slot: i32) -> bool {
        self.0[slot as usize].is_some()
    }

    fn data(&mut self, slot: i32) -> usize {
        0x1000 * (slot as usize + 1)
    }

    fn read_u64(&mut self, at: usize) -> u64 {
        // Ordinary hitboxes: the no-reaction flag (`legend::NO_REACTION`) clear, so none is a windbox.
        if at % 0x1000 == crate::model::legend::NO_REACTION { 0 } else { 7 }
    }

    fn read_f32(&mut self, at: usize) -> f32 {
        match at % 0x1000 {
            0x20 => self.0[at / 0x1000 - 1].expect("an active slot"),
            0x24 => 2.0,
            _ => 0.0,
        }
    }
}

fn the_powers_of_a_fighter_are_read_from_its_active_slots_only() {
    let mut reader = Slots([None, Some(16.0), None, None, None, Some(13.0), None, None]);
    assert_eq!(hitbox::powers(&mut reader), (true, p(&[0.0, 16.0, 0.0, 0.0, 0.0, 13.0])));
    assert_eq!(hitbox::powers(&mut Slots([None; SLOTS])), (false, p(&[])));
}

fn frame(set: &mut HandleSet, slots: [Option<f32>; SLOTS], status: i32) {
    let mut wants = [Sphere::NONE; hitbox::WANT_SCRATCH];
    let ids = StatusIds::from_values([100, 101, 102, 103, 104, 105]);
    let facts = FrameFacts { visualizes: true, enabled: true, slept: false, status, ids };
    hitbox::reconcile_frame(set, facts, &mut wants, &mut Slots(slots), None, &mut OrbSink { status });
}

fn tints(set: &HandleSet, slots: &[u32]) -> Vec<[f32; 3]> {
    let mut by_key: Vec<_> = set.spheres().map(|s| (s.key, s.tint)).collect();
    by_key.sort_by_key(|(key, _)| *key);
    slots.iter().filter_map(|slot| by_key.iter().find(|(key, _)| (key >> 8) & 0xffff == *slot).map(|(_, t)| *t)).collect()
}

fn the_hit_spheres_take_the_frame_data_colours() {
    let (strong, weak) = (hitbox::STRONG_TINT, hitbox::WEAK_TINT);
    let mut set = HandleSet::new();
    set.frame_data = true;
    // The swing's early frame: the tip (slot 0) is sweet, red; the blade (slots 1 and 2) is sour, pink.
    frame(&mut set, [Some(16.0), Some(13.0), Some(13.0), None, None, None, None, None], 20);
    assert_eq!(tints(&set, &[0, 1, 2]), [strong, weak, weak]);
    // A later frame of the same action: only the blade, weaker than the tip seen earlier: late, pink too.
    frame(&mut set, [None, Some(13.0), Some(13.0), None, None, None, None, None], 20);
    assert_eq!(tints(&set, &[1, 2]), [weak, weak]);
    // A new action, the blade alone: it cannot be ranked, and Ultimate Frame Data draws a lone hitbox as strong.
    frame(&mut set, [None, Some(13.0), Some(13.0), None, None, None, None, None], 21);
    assert_eq!(tints(&set, &[1, 2]), [strong, strong]);
}

fn off_is_exactly_the_old_drawing() {
    let mut set = HandleSet::new();
    frame(&mut set, [Some(16.0), Some(13.0), Some(13.0), None, None, None, None, None], 20);
    assert_eq!(tints(&set, &[0, 1, 2]), [0, 1, 2].map(hitbox::hitbox_tint));
}

fn one_function_chooses_every_tint() {
    let usual = [0.1, 0.2, 0.3];
    for kind in [hitbox::KIND_HIT, hitbox::KIND_HURT, hitbox::KIND_GRAB] {
        for spot in [None, Some(Spot::Sweet), Some(Spot::Sour), Some(Spot::Late)] {
            assert_eq!(hitbox::legend_colour(kind, spot, 0, usual, false), usual, "Off is today's tint");
        }
    }
    let on = |spot| hitbox::legend_colour(hitbox::KIND_HIT, spot, 0, usual, true);
    assert_eq!([None, Some(Spot::Sweet)].map(on), [hitbox::STRONG_TINT; 2]);
    assert_eq!([Some(Spot::Sour), Some(Spot::Late)].map(on), [hitbox::WEAK_TINT; 2]);
    assert_ne!(hitbox::WEAK_TINT, hitbox::GRAB_TINT, "pink is not the grab boxes' magenta");
    // The rest of the legend (hb-legend): `model::legend`'s tests.
}

fn the_spot_words_and_rows() {
    for spot in [Spot::Sweet, Spot::Sour, Spot::Late] {
        assert_eq!(Spot::from_key(&spot.name().to_ascii_lowercase()), Some(spot));
    }
    assert_eq!(Spot::from_key("tip"), None);
    assert_eq!((Spot::Sweet.name(), Spot::Sour.name(), Spot::Late.name()), ("Sweet", "Sour", "Late"));
    let mut snapshot = crate::model::settings::Snapshot::factory();
    for row in [ROW_STRIP, ROW_COMBO] {
        assert!(!row_on(&snapshot, row), "{row}: Off by default");
        snapshot.set(row, &[1, 0]).unwrap();
        assert!(row_on(&snapshot, row));
    }
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_strongest_hitbox_of_a_frame_is_sweet_and_a_weaker_one_sour", the_strongest_hitbox_of_a_frame_is_sweet_and_a_weaker_one_sour as fn()),
        ("ties_at_the_top_are_all_sweet", ties_at_the_top_are_all_sweet as fn()),
        ("equal_hitboxes_and_a_lone_hitbox_have_no_label", equal_hitboxes_and_a_lone_hitbox_have_no_label as fn()),
        ("a_hitbox_weaker_than_an_earlier_one_of_the_action_is_late", a_hitbox_weaker_than_an_earlier_one_of_the_action_is_late as fn()),
        ("the_peak_starts_over_with_a_new_status_or_when_the_motion_goes_back", the_peak_starts_over_with_a_new_status_or_when_the_motion_goes_back as fn()),
        ("a_hit_is_attributed_by_the_damage_it_added", a_hit_is_attributed_by_the_damage_it_added as fn()),
        ("staling_and_multipliers_cancel_in_the_ratio", staling_and_multipliers_cancel_in_the_ratio as fn()),
        ("a_hit_that_cannot_be_told_has_no_label", a_hit_that_cannot_be_told_has_no_label as fn()),
        ("the_falcon_knee_hits_eden_logged", the_falcon_knee_hits_eden_logged as fn()),
        ("a_charged_smash_is_attributed_by_the_hitbox_that_dealt_it", a_charged_smash_is_attributed_by_the_hitbox_that_dealt_it as fn()),
        ("the_charged_marth_hits_eden_logged", the_charged_marth_hits_eden_logged as fn()),
        ("a_charged_hit_that_cannot_be_told_has_no_label", a_charged_hit_that_cannot_be_told_has_no_label as fn()),
        ("an_unusable_multiplier_gives_no_label", an_unusable_multiplier_gives_no_label as fn()),
        ("the_powers_of_a_fighter_are_read_from_its_active_slots_only", the_powers_of_a_fighter_are_read_from_its_active_slots_only as fn()),
        ("the_hit_spheres_take_the_frame_data_colours", the_hit_spheres_take_the_frame_data_colours as fn()),
        ("one_function_chooses_every_tint", one_function_chooses_every_tint as fn()),
        ("off_is_exactly_the_old_drawing", off_is_exactly_the_old_drawing as fn()),
        ("the_spot_words_and_rows", the_spot_words_and_rows as fn()),
    ]);
}
}
