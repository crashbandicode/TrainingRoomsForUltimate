//! Decisions SP1 and SP2 (rc.14), ranked by launch power (spots-3): which hitbox is sweet, sour or late, which one a hit connected, and the
//! *Frame data colours*. Numbers are drawn for the cases (a tip of 16 over blades of 13, a kick and a gust of equal damage); the plugin reads
//! them from the game.

lib_unit_tests! {
use super::*;
use crate::model::hitbox::{self, AttackReader, FrameFacts, HandleSet, Sphere, StatusIds};
use crate::model::hitbox_orbs::OrbSink;

/// Hitboxes of damage only (no knockback numbers: every one launches the same, so damage ranks them), slots 0.. (the rest inactive).
fn p(values: &[f32]) -> Powers {
    let mut out = [Hit::default(); SLOTS];
    for (slot, damage) in values.iter().enumerate() {
        out[slot].damage = *damage;
    }
    out
}

/// Hitboxes with their launch numbers, `(damage, growth, fixed, base)`.
fn hits(values: &[(f32, i32, i32, i32)]) -> Powers {
    let mut out = [Hit::default(); SLOTS];
    for (slot, &(damage, growth, fixed, base)) in values.iter().enumerate() {
        out[slot] = Hit { damage, growth, fixed, base };
    }
    out
}

const NONE: Hit = Hit { damage: 0.0, growth: 0, fixed: 0, base: 0 };
/// A defender at `percent` of weight 98 (a middleweight), the attacker uncharged.
fn at(percent: f32) -> Launch {
    Launch { mul: 1.0, percent, weight: 98.0 }
}

fn the_knockback_is_the_games_formula() {
    let hit = |growth, fixed, base| Hit { damage: 10.0, growth, fixed, base };
    // ((10/10 + 10x10/20) x 200/200 x 1.4 + 18) x 1.0 + 0 = 26.4 (SmashWiki, Knockback).
    assert!((knockback(&hit(100, 0, 0), 10.0, 10.0, 100.0) - 26.4).abs() < 1e-4);
    assert!((knockback(&hit(100, 0, 40), 10.0, 10.0, 100.0) - 66.4).abs() < 1e-4, "base knockback adds");
    assert!((knockback(&hit(50, 0, 0), 10.0, 10.0, 100.0) - 13.2).abs() < 1e-4, "growth scales");
    assert!((knockback(&hit(100, 0, 0), 10.0, 10.0, 50.0) - 29.2).abs() < 1e-4, "a lighter defender flies further");
    // A fixed knockback of 50 stands for the damage, at 10%: (1 + 25) x 1.4 + 18 = 54.4, whatever the damage or the percent.
    for (d, p) in [(10.0, 10.0), (3.0, 150.0)] {
        assert!((knockback(&hit(100, 50, 0), d, p, 100.0) - 54.4).abs() < 1e-4, "{d} at {p}");
    }
}

/// The definition (SmashWiki): the sweet spot launches hardest; a hitbox dealing more damage but launching less is the sour one.
fn the_hitbox_that_launches_hardest_is_sweet_not_the_one_that_deals_most() {
    let active = hits(&[(10.0, 70, 0, 80), (12.0, 50, 0, 20)]);
    assert_eq!(classify(&active, at(20.0), &NONE)[..2], [Some(Spot::Sweet), Some(Spot::Sour)]);
}

/// King K. Rool's Gut Check (SmashWiki): the gust deals the same damage as the hit but launches noticeably less, so it is the sour spot.
/// Its numbers here are drawn for the case.
fn gut_checks_gust_is_sour_beside_the_hit_of_equal_damage() {
    let gut_check = hits(&[(14.0, 90, 0, 70), (14.0, 50, 0, 40)]);
    for percent in [0.0, 60.0, 150.0] {
        assert_eq!(classify(&gut_check, at(percent), &NONE)[..2], [Some(Spot::Sweet), Some(Spot::Sour)], "at {percent}%");
    }
}

/// The defender as it is now decides: a hitbox of high base knockback launches harder at low percent, one of high growth at high percent.
fn the_ranking_follows_the_defenders_percent_and_weight() {
    let active = hits(&[(8.0, 40, 0, 90), (8.0, 120, 0, 30)]);
    assert_eq!(classify(&active, at(0.0), &NONE)[..2], [Some(Spot::Sweet), Some(Spot::Sour)], "at 0% the base wins");
    assert_eq!(classify(&active, at(150.0), &NONE)[..2], [Some(Spot::Sour), Some(Spot::Sweet)], "at 150% the growth wins");
    let tipping = |weight| classify(&active, Launch { mul: 1.0, percent: 50.0, weight }, &NONE)[1];
    assert_ne!(tipping(60.0), tipping(140.0), "a light and a heavy defender at the same percent");
}

fn ties_rank_by_damage_and_equal_hitboxes_have_no_label() {
    // Fixed knockback: both launch 54.4 at any damage, so damage ranks them.
    let fixed = hits(&[(8.0, 100, 50, 0), (5.0, 100, 50, 0)]);
    assert_eq!(classify(&fixed, at(40.0), &NONE)[..2], [Some(Spot::Sweet), Some(Spot::Sour)]);
    let jab = hits(&[(3.0, 100, 0, 20), (3.0, 100, 0, 20), (3.0, 100, 0, 20)]);
    assert_eq!(classify(&jab, at(40.0), &NONE), [None; SLOTS], "equal on both");
    assert_eq!(classify(&hits(&[(12.0, 90, 0, 40)]), at(40.0), &NONE), [None; SLOTS], "a lone hitbox");
    let top = hits(&[(16.0, 90, 0, 40), (16.0, 90, 0, 40), (13.0, 90, 0, 40)]);
    assert_eq!(classify(&top, at(0.0), &NONE)[..3], [Some(Spot::Sweet), Some(Spot::Sweet), Some(Spot::Sour)], "ties at the top are all Sweet");
    assert_eq!(classify(&p(&[16.0, 0.0]), at(0.0), &NONE)[1], None, "a slot with no damage has no label");
}

/// Late is the timing case, measured in knockback: this frame's strongest launches less than the action's earlier peak, even when it deals more.
fn a_hitbox_launching_less_than_an_earlier_one_of_the_action_is_late() {
    let knee = Hit { damage: 18.0, growth: 100, fixed: 0, base: 30 };
    let after = hits(&[(19.0, 50, 0, 10), (19.0, 50, 0, 10)]);
    assert_eq!(classify(&after, at(30.0), &knee)[..2], [Some(Spot::Late); 2]);
    assert_eq!(classify(&after, at(30.0), &NONE)[..2], [None; 2], "no earlier peak: equal hitboxes, no label");
    // The sweet phase is not over while the peak's hitbox is still active.
    let both = hits(&[(18.0, 100, 0, 30), (19.0, 50, 0, 10)]);
    assert_eq!(classify(&both, at(30.0), &knee)[..2], [Some(Spot::Sweet), Some(Spot::Sour)]);
    // By damage alone (no knockback numbers) as before: 13 after a 16 is Late.
    assert_eq!(classify(&p(&[13.0]), at(0.0), &p(&[16.0])[0])[0], Some(Spot::Late));
}

fn the_peak_keeps_the_strongest_hitbox_and_starts_over_with_a_new_action() {
    let mut peak = Peak::new();
    assert_eq!(peak.earlier(10, 0.0), NONE);
    let strong = Hit { damage: 10.0, growth: 100, fixed: 0, base: 60 };
    peak.fold(&hits(&[(10.0, 100, 0, 60), (12.0, 40, 0, 10)]), at(0.0));
    assert_eq!(peak.earlier(10, 1.0), strong, "the hardest launch, not the most damage");
    peak.fold(&hits(&[(12.0, 40, 0, 10)]), at(0.0));
    assert_eq!(peak.earlier(10, 9.0), strong, "a weaker frame keeps it, and hitlag holds the motion frame");
    assert_eq!(peak.earlier(10, 0.0), NONE, "the same status entered again (the next jab): the motion went back");
    peak.fold(&p(&[8.0]), at(0.0));
    assert_eq!(peak.earlier(11, 1.0), NONE, "a new status is a new action");
}

/// The measured multiplier: a fresh hit adds 1.26 times its hitbox's damage (Eden, 2026-11-14).
const M: f32 = 1.26;

/// What the defender received from `hit` when it gained `gained` at `before` percent: the formula's knockback, as the game stores it.
fn took(hit: (f32, i32, i32, i32), gained: f32, before: f32) -> Landed {
    let h = Hit { damage: hit.0, growth: hit.1, fixed: hit.2, base: hit.3 };
    Landed { gained, launched: knockback(&h, gained / TARGET_MUL, before + gained, 98.0) }
}

fn by_damage(gained: f32) -> Landed {
    Landed { gained, launched: 0.0 }
}

/// The hard case: hitboxes of equal damage. The damage the defender gained cannot tell them apart; the knockback it received can.
fn a_hit_of_equal_damage_is_attributed_by_the_knockback_received() {
    let (hit, gust) = ((14.0, 90, 0, 70), (14.0, 50, 0, 40));
    let gut_check = hits(&[hit, gust]);
    for before in [0.0, 60.0, 120.0] {
        assert_eq!(attribute(took(hit, 14.0 * M, before), &gut_check, at(before), &NONE), Some(Spot::Sweet), "the hit at {before}%");
        assert_eq!(attribute(took(gust, 14.0 * M, before), &gut_check, at(before), &NONE), Some(Spot::Sour), "the gust at {before}%");
    }
    // Rage and crouching scale what the defender receives, for every hitbox alike.
    for r in [0.85, 1.1] {
        let mut landed = took(gust, 14.0 * M, 60.0);
        landed.launched *= r;
        assert_eq!(attribute(landed, &gut_check, at(60.0), &NONE), Some(Spot::Sour), "the gust at x{r}");
    }
    assert_eq!(attribute(by_damage(14.0 * M), &gut_check, at(60.0), &NONE), None, "without the knockback reading: equal damage, never a guess");
}

/// The damage window stays as a cross-check: both readings must fit the same hitbox.
fn a_hit_both_readings_cannot_place_has_no_label() {
    let marth = hits(&[(13.0, 70, 0, 40), (18.0, 80, 0, 50)]);
    let blade = took((13.0, 70, 0, 40), 13.0 * M, 30.0);
    assert_eq!(attribute(blade, &marth, at(30.0), &NONE), Some(Spot::Sour));
    let wrong = Landed { launched: took((18.0, 80, 0, 50), 13.0 * M, 30.0).launched * 3.0, ..blade };
    assert_eq!(attribute(wrong, &marth, at(30.0), &NONE), None, "a knockback no hitbox gives");
    assert_eq!(attribute(Landed { gained: 30.0 * M, ..blade }, &marth, at(30.0), &NONE), None, "a damage no hitbox deals");
}

/// Marth's forward smash as the game reports its damages: three blade hitboxes of 13 and a tip of 18.
fn marth() -> Powers {
    p(&[13.0, 13.0, 13.0, 18.0])
}

fn a_hit_is_attributed_by_the_damage_it_added() {
    assert_eq!(attribute(by_damage(18.0 * M), &marth(), at(0.0), &NONE), Some(Spot::Sweet));
    assert_eq!(attribute(by_damage(13.0 * M), &marth(), at(0.0), &NONE), Some(Spot::Sour));
    // The hit Eden logged: 16.926 added. The blade's ratio is 1.30; the tip's would be 0.94, below any multiplier the game uses.
    assert_eq!(attribute(by_damage(16.926), &marth(), at(0.0), &NONE), Some(Spot::Sour));
    assert_eq!(attribute(by_damage(13.0 * M), &p(&[13.0]), at(0.0), &p(&[16.0])[0]), Some(Spot::Late));
}

fn staling_and_multipliers_cancel_in_the_ratio() {
    for m in [1.07, 1.15, M, 1.35] {
        assert_eq!(attribute(by_damage(13.0 * m), &marth(), at(0.0), &NONE), Some(Spot::Sour), "a blade hit at {m}");
    }
    assert_eq!(attribute(by_damage(18.0 * 1.35), &marth(), at(0.0), &NONE), Some(Spot::Sweet), "a slightly charged tip hit");
    // A stale tip hit could also be a blade hit: 20.7 is 1.59 times 13. No label.
    for m in [1.07, 1.15] {
        assert_eq!(attribute(by_damage(18.0 * m), &marth(), at(0.0), &NONE), None, "a tip hit at {m}");
    }
}

fn a_hit_that_cannot_be_told_has_no_label() {
    assert_eq!(attribute(by_damage(15.5 * M), &p(&[16.0, 15.0]), at(0.0), &NONE), None, "two candidates of different classes");
    assert_eq!(attribute(by_damage(12.0 * M), &p(&[12.0, 12.0, 12.0]), at(0.0), &NONE), None, "equal hitboxes");
    assert_eq!(attribute(by_damage(30.0 * M), &p(&[16.0, 13.0]), at(0.0), &NONE), None, "no hitbox could have dealt it");
    for gained in [0.0, -3.0, f32::NAN, f32::INFINITY] {
        assert_eq!(attribute(by_damage(gained), &marth(), at(0.0), &NONE), None, "{gained}");
    }
    assert_eq!(attribute(by_damage(16.0), &p(&[]), at(0.0), &NONE), None, "nothing active");
    // Candidates that agree are still a label: 13 and 12.8 are both Sour, and the tip's ratio (0.91) is below the window.
    assert_eq!(attribute(by_damage(13.0 * M), &p(&[18.0, 13.0, 12.8]), at(0.0), &NONE), Some(Spot::Sour));
}

/// Captain Falcon's forward air as the first Eden boot logged it (spots-4e9f91b5-1): the knee's three phases, damages 14 and 18, then 8 and 9,
/// then 3. The window reads each hit the way the game made it (a fresh hit is 1.26 times the damage).
fn the_falcon_knee_hits_eden_logged() {
    let knee = p(&[14.0, 18.0, 18.0]);
    assert_eq!(attribute(by_damage(22.68), &knee, at(0.0), &NONE), Some(Spot::Sweet), "18 x 1.26");
    assert_eq!(attribute(by_damage(17.64), &knee, at(0.0), &NONE), Some(Spot::Sour), "14 x 1.26; the 18s' ratio is 0.98");
    let middle = p(&[8.0, 9.0, 8.0, 9.0]);
    assert_eq!(attribute(by_damage(8.736), &middle, at(0.0), &NONE), Some(Spot::Sour), "8 x 1.092 (stale); the 9s' ratio is 0.97");
    assert_eq!(attribute(by_damage(10.08), &middle, at(0.0), &NONE), None, "8 x 1.26 or 9 x 1.12: no label");
    // The last frames: 3 and 3 after the 18 of the same action are Late; a lone 3 on its own has no label.
    assert_eq!(attribute(by_damage(3.213), &p(&[3.0, 3.0]), at(0.0), &p(&[18.0])[0]), Some(Spot::Late));
    assert_eq!(attribute(by_damage(3.213), &p(&[3.0, 3.0]), at(0.0), &NONE), None);
}

/// A smash charged to `mul` (the game's `AttackModule::power_mul_status`, 1.0 uncharged): every slot's damage is multiplied by it, so a hit adds
/// the slot's damage x `mul` x 1.26 and the label follows the hitbox, not the charge (spots-2, 2026-11-14).
fn a_charged_smash_is_attributed_by_the_hitbox_that_dealt_it() {
    let charged = |mul| Launch { mul, ..at(0.0) };
    for mul in [1.0, 1.1, 1.2, 1.3, 1.4] {
        assert_eq!(attribute(by_damage(13.0 * mul * M), &marth(), charged(mul), &NONE), Some(Spot::Sour), "a blade hit at charge {mul}");
        assert_eq!(attribute(by_damage(18.0 * mul * M), &marth(), charged(mul), &NONE), Some(Spot::Sweet), "a tip hit at charge {mul}");
    }
    assert_eq!(attribute(by_damage(3.0 * 1.4 * M), &p(&[3.0, 3.0]), charged(1.4), &p(&[18.0])[0]), Some(Spot::Late));
    // A charge does not make a hit tellable: a stale tip hit that a blade could also have made is still no label.
    assert_eq!(attribute(by_damage(18.0 * 1.4 * 1.07), &marth(), charged(1.4), &NONE), None, "the blade's ratio is 1.48");
}

/// Marth's forward smash held to different charges, as Eden logged it (spots2-d37c0734-1, Yoga, 2026-11-14, `sp-charge`).
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
        assert_eq!(attribute(by_damage(gained), &marth(), Launch { mul, ..at(0.0) }, &NONE), Some(expected), "{gained} added at charge {mul}");
    }
}

/// The multiplier is the game's own number; one that was not read (0.0) or is not a usable number labels nothing.
fn an_unusable_multiplier_gives_no_label() {
    for mul in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        for gained in [18.0 * M, 13.0 * M] {
            assert_eq!(attribute(by_damage(gained), &marth(), Launch { mul, ..at(0.0) }, &NONE), None, "multiplier {mul}");
        }
    }
}

/// Slot `i` is active when `0[i]` is `Some(hit)`; its `AttackData` is at `0x1000 * (i + 1)`. Reading an inactive slot panics.
struct Slots([Option<(f32, i32, i32, i32)>; SLOTS]);

impl AttackReader for Slots {
    fn active(&mut self, slot: i32) -> bool {
        self.0[slot as usize].is_some()
    }

    fn data(&mut self, slot: i32) -> usize {
        0x1000 * (slot as usize + 1)
    }

    fn read_u64(&mut self, at: usize) -> u64 {
        let (_, growth, fixed, base) = self.0[at / 0x1000 - 1].expect("an active slot");
        // The two knockback quads (angle 45, then growth; fixed, then base); the no-reaction flag clear, so none is a windbox.
        match at % 0x1000 {
            0x28 => (growth as u32 as u64) << 32 | 45,
            0x30 => (base as u32 as u64) << 32 | fixed as u32 as u64,
            0x70 => 0,
            _ => 7,
        }
    }

    fn read_f32(&mut self, at: usize) -> f32 {
        match at % 0x1000 {
            0x20 => self.0[at / 0x1000 - 1].expect("an active slot").0,
            0x24 => 2.0,
            _ => 0.0,
        }
    }
}

fn the_powers_of_a_fighter_are_read_from_its_active_slots_only() {
    let mut reader = Slots([None, Some((16.0, 90, 0, 40)), None, None, None, Some((13.0, 100, 30, 0)), None, None]);
    let mut expected = [Hit::default(); SLOTS];
    expected[1] = Hit { damage: 16.0, growth: 90, fixed: 0, base: 40 };
    expected[5] = Hit { damage: 13.0, growth: 100, fixed: 30, base: 0 };
    assert_eq!(hitbox::powers(&mut reader), (true, expected));
    assert_eq!(hitbox::powers(&mut Slots([None; SLOTS])), (false, p(&[])));
}

fn frame(set: &mut HandleSet, slots: [Option<(f32, i32, i32, i32)>; SLOTS], status: i32) {
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
    let (tip, blade) = (Some((16.0, 90, 0, 40)), Some((13.0, 90, 0, 40)));
    let mut set = HandleSet::new();
    set.frame_data = true;
    // The swing's early frame: the tip (slot 0) is sweet, red; the blade (slots 1 and 2) is sour, pink.
    frame(&mut set, [tip, blade, blade, None, None, None, None, None], 20);
    assert_eq!(tints(&set, &[0, 1, 2]), [strong, weak, weak]);
    // A later frame of the same action: only the blade, launching less than the tip seen earlier: late, pink too.
    frame(&mut set, [None, blade, blade, None, None, None, None, None], 20);
    assert_eq!(tints(&set, &[1, 2]), [weak, weak]);
    // A new action, the blade alone: it cannot be ranked, and Ultimate Frame Data draws a lone hitbox as strong.
    frame(&mut set, [None, blade, blade, None, None, None, None, None], 21);
    assert_eq!(tints(&set, &[1, 2]), [strong, strong]);
    // Gut Check: the hit and the gust deal the same damage; the gust launches less, so it is pink. Against the defender the pass set.
    set.launch = Launch { mul: 1.0, percent: 80.0, weight: 98.0 };
    frame(&mut set, [Some((14.0, 90, 0, 70)), Some((14.0, 50, 0, 40)), None, None, None, None, None, None], 22);
    assert_eq!(tints(&set, &[0, 1]), [strong, weak]);
}

fn off_is_exactly_the_old_drawing() {
    let mut set = HandleSet::new();
    frame(&mut set, [Some((16.0, 90, 0, 40)), Some((13.0, 90, 0, 40)), Some((13.0, 90, 0, 40)), None, None, None, None, None], 20);
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
        ("the_knockback_is_the_games_formula", the_knockback_is_the_games_formula as fn()),
        ("the_hitbox_that_launches_hardest_is_sweet_not_the_one_that_deals_most", the_hitbox_that_launches_hardest_is_sweet_not_the_one_that_deals_most as fn()),
        ("gut_checks_gust_is_sour_beside_the_hit_of_equal_damage", gut_checks_gust_is_sour_beside_the_hit_of_equal_damage as fn()),
        ("the_ranking_follows_the_defenders_percent_and_weight", the_ranking_follows_the_defenders_percent_and_weight as fn()),
        ("ties_rank_by_damage_and_equal_hitboxes_have_no_label", ties_rank_by_damage_and_equal_hitboxes_have_no_label as fn()),
        ("a_hitbox_launching_less_than_an_earlier_one_of_the_action_is_late", a_hitbox_launching_less_than_an_earlier_one_of_the_action_is_late as fn()),
        ("the_peak_keeps_the_strongest_hitbox_and_starts_over_with_a_new_action", the_peak_keeps_the_strongest_hitbox_and_starts_over_with_a_new_action as fn()),
        ("a_hit_of_equal_damage_is_attributed_by_the_knockback_received", a_hit_of_equal_damage_is_attributed_by_the_knockback_received as fn()),
        ("a_hit_both_readings_cannot_place_has_no_label", a_hit_both_readings_cannot_place_has_no_label as fn()),
        ("a_hit_is_attributed_by_the_damage_it_added", a_hit_is_attributed_by_the_damage_it_added as fn()),
        ("staling_and_multipliers_cancel_in_the_ratio", staling_and_multipliers_cancel_in_the_ratio as fn()),
        ("a_hit_that_cannot_be_told_has_no_label", a_hit_that_cannot_be_told_has_no_label as fn()),
        ("the_falcon_knee_hits_eden_logged", the_falcon_knee_hits_eden_logged as fn()),
        ("a_charged_smash_is_attributed_by_the_hitbox_that_dealt_it", a_charged_smash_is_attributed_by_the_hitbox_that_dealt_it as fn()),
        ("the_charged_marth_hits_eden_logged", the_charged_marth_hits_eden_logged as fn()),
        ("an_unusable_multiplier_gives_no_label", an_unusable_multiplier_gives_no_label as fn()),
        ("the_powers_of_a_fighter_are_read_from_its_active_slots_only", the_powers_of_a_fighter_are_read_from_its_active_slots_only as fn()),
        ("the_hit_spheres_take_the_frame_data_colours", the_hit_spheres_take_the_frame_data_colours as fn()),
        ("one_function_chooses_every_tint", one_function_chooses_every_tint as fn()),
        ("off_is_exactly_the_old_drawing", off_is_exactly_the_old_drawing as fn()),
        ("the_spot_words_and_rows", the_spot_words_and_rows as fn()),
    ]);
}
}
