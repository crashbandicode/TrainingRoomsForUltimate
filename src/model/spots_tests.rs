//! Decisions SP1 and SP2 (rc.14), ranked by launch power (spots-3): which hitbox is sweet, sour or late, which one a hit connected, and the
//! *Frame data colours*. Numbers are drawn for the classification cases (a tip of 16 over blades of 13, a hit and a gust of equal damage);
//! the attribution cases replay what Eden logged (spots3-bade7ea6-1) where they say so. The plugin reads every number from the game.

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
    Launch { mul: 1.0, percent, weight: 98.0, attacker: 0.0 }
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
fn the_strongest_hitbox_of_a_frame_is_sweet_and_a_weaker_one_sour() {
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
    let tipping = |weight| classify(&active, Launch { mul: 1.0, percent: 70.0, weight, attacker: 0.0 }, &NONE)[1];
    assert_eq!([tipping(60.0), tipping(140.0)], [Some(Spot::Sweet), Some(Spot::Sour)], "a light and a heavy defender at the same percent");
}

/// Ties: equal knockback ranks by damage; equal on both, no label.
fn equal_hitboxes_and_a_lone_hitbox_have_no_label() {
    // Fixed knockback: both launch 54.4 at any damage, so damage ranks them.
    let fixed = hits(&[(8.0, 100, 50, 0), (5.0, 100, 50, 0)]);
    assert_eq!(classify(&fixed, at(40.0), &NONE)[..2], [Some(Spot::Sweet), Some(Spot::Sour)]);
    let jab = hits(&[(3.0, 100, 0, 20), (3.0, 100, 0, 20), (3.0, 100, 0, 20)]);
    assert_eq!(classify(&jab, at(40.0), &NONE), [None; SLOTS], "equal on both");
    assert_eq!(classify(&hits(&[(12.0, 90, 0, 40)]), at(40.0), &NONE), [None; SLOTS], "a lone hitbox");
    assert_eq!(classify(&p(&[16.0, 0.0]), at(0.0), &NONE)[1], None, "a slot with no damage has no label");
}

fn ties_at_the_top_are_all_sweet() {
    let top = hits(&[(16.0, 90, 0, 40), (16.0, 90, 0, 40), (13.0, 90, 0, 40)]);
    assert_eq!(classify(&top, at(0.0), &NONE)[..3], [Some(Spot::Sweet), Some(Spot::Sweet), Some(Spot::Sour)]);
}

/// Late is the timing case, measured in knockback: this frame's strongest launches less than the action's earlier peak, even when it deals more.
fn a_hitbox_weaker_than_an_earlier_one_of_the_action_is_late() {
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

fn the_peak_starts_over_with_a_new_status_or_when_the_motion_goes_back() {
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

/// The hit of `slot` as the game records it: the defender (weight `w`, at `before`) gained `gained` and received that slot's knockback times `r`,
/// by the game's percent (the percent before plus the hit's damage before the 1-on-1 ratio, `gained / TARGET_MUL`).
fn took(active: &Powers, slot: usize, gained: f32, before: f32, w: f32, r: f32) -> Landed {
    Landed { gained, launched: r * knockback(&active[slot], gained / TARGET_MUL, before + gained / TARGET_MUL, w), slot: Some(slot) }
}

/// A defender of weight `w` at `percent`.
fn on(w: f32, percent: f32) -> Launch {
    Launch { mul: 1.0, percent, weight: w, attacker: 0.0 }
}

/// Marth's forward smash as Eden logged it (spots3-bade7ea6-1): three blades (13, growth 75, base 48) and the tip (18, 80, 80).
fn marth() -> Powers {
    hits(&[(13.0, 75, 0, 48), (13.0, 75, 0, 48), (13.0, 75, 0, 48), (18.0, 80, 0, 80)])
}

/// The hard case: hitboxes of equal damage. The damage cannot tell them apart; the game records which slot hit, and the knockback the
/// defender received agrees. Mario's jab as Eden logged it: two slots of (2.2, 30, 20) beat two of (2.2, 20, 15).
fn a_hit_of_equal_damage_is_attributed_by_the_slot_the_game_recorded() {
    let gut_check = hits(&[(14.0, 90, 0, 70), (14.0, 50, 0, 40)]);
    for before in [0.0, 60.0, 120.0] {
        assert_eq!(attribute(took(&gut_check, 0, 14.0 * M, before, 98.0, 1.0), &gut_check, at(before), &NONE), Some(Spot::Sweet), "the hit at {before}%");
        assert_eq!(attribute(took(&gut_check, 1, 14.0 * M, before, 98.0, 1.0), &gut_check, at(before), &NONE), Some(Spot::Sour), "the gust at {before}%");
    }
    let jab = hits(&[(2.2, 30, 0, 20), (2.2, 30, 0, 20), (2.2, 20, 0, 15), (2.2, 20, 0, 15)]);
    // Eden: the CPU Mario's jab on P1 Marth (weight 90) at 29.4%, fully staled (gained 1.239, 0.563 of 2.2): 27.987 received from slot 1.
    assert_eq!(attribute(Landed { gained: 1.239, launched: 27.987, slot: Some(1) }, &jab, on(90.0, 29.362), &NONE), Some(Spot::Sweet));
    assert_eq!(attribute(Landed { gained: 1.239, launched: 20.395, slot: Some(2) }, &jab, on(90.0, 30.601), &NONE), Some(Spot::Sour));
    let unread = Landed { slot: None, ..took(&gut_check, 1, 14.0 * M, 60.0, 98.0, 1.0) };
    assert_eq!(attribute(unread, &gut_check, at(60.0), &NONE), None, "no record (not read, or a projectile's): never a guess");
}

/// Both cross-checks hold the recorded slot to what the defender took; the knockback window is 0.96..=1.05 (Eden's spread 0.990..1.020 plus 0.03).
fn a_hit_both_readings_cannot_place_has_no_label() {
    let blade = took(&marth(), 0, 13.0 * M, 30.0, 98.0, 1.0);
    assert_eq!(attribute(blade, &marth(), at(30.0), &NONE), Some(Spot::Sour));
    for r in [0.97, 1.04] {
        assert_eq!(attribute(Landed { launched: blade.launched * r, ..blade }, &marth(), at(30.0), &NONE), Some(Spot::Sour), "inside the window: x{r}");
    }
    for r in [0.95, 1.06] {
        assert_eq!(attribute(Landed { launched: blade.launched * r, ..blade }, &marth(), at(30.0), &NONE), None, "a knockback the slot does not give: x{r}");
    }
    assert_eq!(attribute(Landed { gained: 30.0 * M, ..blade }, &marth(), at(30.0), &NONE), None, "a damage the slot does not deal");
    assert_eq!(attribute(Landed { launched: 0.0, ..blade }, &marth(), at(30.0), &NONE), None, "no knockback read");
}

fn a_hit_is_attributed_by_the_damage_it_added() {
    // Eden (spots3-bade7ea6-1, Marth's forward smash on Mario at 0%, the C-stick held 6 frames: charge 1.0267): the tip, then a blade.
    let launch = Launch { mul: 1.0267, ..at(0.0) };
    assert_eq!(attribute(Landed { gained: 23.285, launched: 117.183, slot: Some(3) }, &marth(), launch, &NONE), Some(Spot::Sweet));
    assert_eq!(attribute(Landed { gained: 16.817, launched: 73.054, slot: Some(0) }, &marth(), launch, &NONE), Some(Spot::Sour));
    let late = hits(&[(13.0, 75, 0, 48)]);
    assert_eq!(attribute(took(&late, 0, 13.0 * M, 0.0, 98.0, 1.0), &late, at(0.0), &marth()[3]), Some(Spot::Late), "a blade after the tip");
}

/// Staling lowers the damage the slot adds, down to 0.563 measured; the window keeps it. Marth's jab tip on K. Rool, stale (Eden): the game's
/// slot 2, Sweet, where the damage alone looked like a fresh 3 (the defect of the first boot).
fn staling_and_multipliers_cancel_in_the_ratio() {
    for m in [0.563, 0.82, 1.07, M, 1.35] {
        assert_eq!(attribute(took(&marth(), 0, 13.0 * m, 0.0, 98.0, 1.0), &marth(), at(0.0), &NONE), Some(Spot::Sour), "a blade hit at {m}");
    }
    let jab = hits(&[(3.0, 15, 0, 30), (3.0, 12, 0, 30), (5.0, 12, 0, 30), (5.0, 12, 0, 30)]);
    assert_eq!(attribute(Landed { gained: 4.082, launched: 32.918, slot: Some(2) }, &jab, on(133.0, 12.718), &NONE), Some(Spot::Sweet));
    assert_eq!(attribute(Landed { gained: 2.968, launched: 32.493, slot: Some(1) }, &jab, on(133.0, 7.056), &NONE), Some(Spot::Sour));
    assert_eq!(attribute(took(&marth(), 0, 13.0 * 0.45, 0.0, 98.0, 1.0), &marth(), at(0.0), &NONE), None, "below the measured floor");
}

fn a_hit_that_cannot_be_told_has_no_label() {
    let equal = hits(&[(12.0, 90, 0, 40), (12.0, 90, 0, 40), (12.0, 90, 0, 40)]);
    assert_eq!(attribute(took(&equal, 1, 12.0 * M, 0.0, 98.0, 1.0), &equal, at(0.0), &NONE), None, "identical hitboxes");
    let blade = took(&marth(), 0, 13.0 * M, 0.0, 98.0, 1.0);
    for slot in [4, 7, 8, 200] {
        assert_eq!(attribute(Landed { slot: Some(slot), ..blade }, &marth(), at(0.0), &NONE), None, "slot {slot}: not active or not a slot");
    }
    for gained in [0.0, -3.0, f32::NAN, f32::INFINITY] {
        assert_eq!(attribute(Landed { gained, ..blade }, &marth(), at(0.0), &NONE), None, "{gained}");
    }
    assert_eq!(attribute(blade, &p(&[]), at(0.0), &NONE), None, "nothing active");
}

/// Captain Falcon's forward air: its late hitboxes as Eden logged them (spots3-bade7ea6-1: two of 3, growth 80, base 35; 50.164 received at 0%)
/// after the knee of the same action are Late; on their own they are identical, no label.
fn the_falcon_knee_hits_eden_logged() {
    let late = hits(&[(3.0, 80, 0, 35), (3.0, 80, 0, 35)]);
    let knee = Hit { damage: 18.0, growth: 100, fixed: 0, base: 30 };
    let hit = Landed { gained: 3.213, launched: 50.164, slot: Some(0) };
    assert_eq!(attribute(hit, &late, at(0.0), &knee), Some(Spot::Late));
    assert_eq!(attribute(hit, &late, at(0.0), &NONE), None);
}

/// A smash charged to `mul` (the game's `AttackModule::power_mul_status`, 1.0 uncharged): every slot's damage is multiplied by it, and the label
/// follows the slot the game recorded, not the charge (spots-2, 2026-11-14).
fn a_charged_smash_is_attributed_by_the_hitbox_that_dealt_it() {
    for mul in [1.0, 1.2, 1.4] {
        let launch = Launch { mul, ..at(0.0) };
        assert_eq!(attribute(took(&marth(), 0, 13.0 * mul * M, 0.0, 98.0, 0.99), &marth(), launch, &NONE), Some(Spot::Sour), "a blade hit at charge {mul}");
        assert_eq!(attribute(took(&marth(), 3, 18.0 * mul * M, 0.0, 98.0, 0.99), &marth(), launch, &NONE), Some(Spot::Sweet), "a tip hit at charge {mul}");
    }
}

/// A charged hit whose record is not this attack's (not read, or another object's) has no label, charged or not.
fn a_charged_hit_that_cannot_be_told_has_no_label() {
    let hit = Landed { slot: None, ..took(&marth(), 3, 18.0 * 1.4 * M, 0.0, 98.0, 0.99) };
    assert_eq!(attribute(hit, &marth(), Launch { mul: 1.4, ..at(0.0) }, &NONE), None);
}

/// Marth's forward smash held to different charges, as Eden logged it (spots3-bade7ea6-1, `sp-charge`): the received knockback over the formula's
/// is 0.990 to 0.995 for these (the lowest the boots measured), inside the window.
fn the_charged_marth_hits_eden_logged() {
    for (gained, mul, launched, slot, expected) in [
        (23.285, 1.0267, 117.183, 3, Spot::Sweet),
        (16.817, 1.0267, 73.054, 0, Spot::Sour),
        (28.123, 1.2400, 127.084, 3, Spot::Sweet),
        (20.202, 1.2333, 77.814, 0, Spot::Sour),
        (31.752, 1.4000, 135.677, 3, Spot::Sweet),
        (22.932, 1.4000, 82.248, 0, Spot::Sour),
    ] {
        let hit = Landed { gained, launched, slot: Some(slot) };
        assert_eq!(attribute(hit, &marth(), Launch { mul, ..at(0.0) }, &NONE), Some(expected), "{gained} added at charge {mul}");
    }
}

/// The percent the game feeds its knockback function is the percent before the hit plus the hit's damage before the 1-on-1 ratio (the damage
/// record's `+0x0` + `+0x8`, `main+0x402e94`), not the percent after the hit. Two hits as Eden recorded them (spots3-bade7ea6-1, with the record's
/// own `rec08`): the knockback the game gave over the formula's is 0.9904 and 1.0200, the ends of the 93 recorded hits' spread. The percent after
/// the hit, the formula before this change, puts the first at 0.9324, outside the window, and a knockback by it is not this slot's.
fn the_knockback_percent_is_the_games_on_recorded_hits() {
    let jab = hits(&[(2.2, 30, 0, 20), (2.2, 30, 0, 20), (2.2, 20, 0, 15), (2.2, 20, 0, 15)]);
    // (slot's powers, the hit's slot, charge, gained, the record's +0x8, the percent before, weight, received, received over the game's formula):
    // `sp-charge`, Marth's forward smash held to 1.4, its tip on Mario at 0%; `rage-stand-lines.txt`, the CPU Mario's fully staled jab (0.563 of 2.2)
    // on Marth at 29.362%.
    let recorded = [(marth(), 3, 1.4, 31.752, 26.46, 0.0, 98.0, 135.677, 0.9904), (jab, 1, 1.0, 1.239, 1.033, 29.362, 90.0, 27.987, 1.0200)];
    for (powers, slot, mul, gained, rec08, before, w, received, ratio) in recorded {
        assert!((gained / TARGET_MUL - rec08).abs() < 2e-3, "the percent's rise is 1.2 times the record's damage: {gained} over {rec08}");
        let game = knockback(&powers[slot], rec08, before + rec08, w);
        assert!((received / game - ratio).abs() < 5e-4, "{received} over {game}: {}", received / game);
        assert!((KB_LOW..=KB_HIGH).contains(&(received / game)));
        let launch = Launch { mul, percent: before, weight: w, attacker: 0.0 };
        assert_eq!(attribute(Landed { gained, launched: received, slot: Some(slot) }, &powers, launch, &NONE), Some(Spot::Sweet), "the hit at {before}%");
    }
    let after = knockback(&marth()[3], 26.46, 0.0 + 31.752, 98.0);
    assert!(135.677 / after < KB_LOW, "by the percent after the hit the tip's ratio is {}", 135.677 / after);
    let launch = Launch { mul: 1.4, ..on(98.0, 0.0) };
    assert_eq!(attribute(Landed { gained: 31.752, launched: after, slot: Some(3) }, &marth(), launch, &NONE), None);
}

/// Rage (the attacker's percent) scales what the defender receives; the expected knockback carries the game's multiplier, which stops at
/// 1.1, so a hit stays labelled at any attacker percent: below the cap (Eden: P1 Marth at 60.3% landed a blade hit that received 74.664,
/// x1.022 the same hit's 73.054 from 0%; spots3-bedc9a69-1, `sp-rage`), at it (150%) and far above it (999%).
fn an_uncharged_hit_at_full_rage_stays_labelled() {
    assert_eq!([0.0, 35.0, 150.0, 999.0].map(rage), [1.0, 1.0, RAGE_MAX, RAGE_MAX]);
    assert!((rage(60.349) - 1.02204).abs() < 1e-4, "{}", rage(60.349));
    let raged = |attacker| Launch { mul: 1.0267, attacker, ..at(0.0) };
    assert_eq!(attribute(Landed { gained: 16.817, launched: 74.664, slot: Some(0) }, &marth(), raged(60.349), &NONE), Some(Spot::Sour));
    let full = took(&marth(), 3, 18.0 * M, 0.0, 98.0, 1.1);
    for attacker in [150.0, 999.0] {
        assert_eq!(attribute(full, &marth(), Launch { attacker, ..at(0.0) }, &NONE), Some(Spot::Sweet), "{attacker}%: x1.1 expected");
    }
    assert_eq!(attribute(full, &marth(), at(0.0), &NONE), None, "the same knockback from an attacker at 0% is not this slot's");
}

/// Fixed knockback takes no rage (the game's knockback function multiplies rage on its growth path only): a set-knockback hitbox launches
/// alike from any attacker percent while a growth one gains rage, so the attacker's percent can turn the ranking.
fn fixed_knockback_takes_no_rage() {
    let pair = hits(&[(6.0, 100, 60, 30), (8.0, 90, 0, 34)]);
    let (calm, raged) = (on(98.0, 50.0), Launch { attacker: 999.0, ..on(98.0, 50.0) });
    assert_eq!(classify(&pair, calm, &NONE)[..2], [Some(Spot::Sweet), Some(Spot::Sour)]);
    assert_eq!(classify(&pair, raged, &NONE)[..2], [Some(Spot::Sour), Some(Spot::Sweet)], "rage lifts the growth hitbox only");
    let set = took(&pair, 0, 6.0 * M, 50.0, 98.0, 0.99);
    assert_eq!(attribute(set, &pair, raged, &NONE), Some(Spot::Sour), "the set knockback expected without rage");
}

/// The game caps any knockback at 2500 (`reaction_max`): a hit past it on a defender at 900% is received as 2500 and still placed, and two
/// hitboxes that both pass it launch alike, so their damage ranks them.
fn the_knockback_ceiling_is_the_games() {
    let smash = hits(&[(30.0, 100, 0, 40), (20.0, 100, 0, 30)]);
    let hit = Landed { gained: 30.0 * M, launched: REACTION_MAX, slot: Some(0) };
    assert_eq!(attribute(hit, &smash, on(60.0, 900.0), &NONE), Some(Spot::Sweet));
    let capped = hits(&[(30.0, 100, 0, 40), (28.0, 100, 0, 400)]);
    assert_eq!(classify(&capped, on(60.0, 950.0), &NONE)[..2], [Some(Spot::Sweet), Some(Spot::Sour)], "both past 2500: the damage decides");
}

/// The multiplier is the game's own number; one that was not read (0.0) or is not a usable number labels nothing.
fn an_unusable_multiplier_gives_no_label() {
    for mul in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        for slot in [0, 3] {
            let hit = took(&marth(), slot, marth()[slot].damage * M, 0.0, 98.0, 1.0);
            assert_eq!(attribute(hit, &marth(), Launch { mul, ..at(0.0) }, &NONE), None, "multiplier {mul}");
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
    set.launch = Launch { mul: 1.0, percent: 80.0, weight: 98.0, attacker: 0.0 };
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
        ("the_strongest_hitbox_of_a_frame_is_sweet_and_a_weaker_one_sour", the_strongest_hitbox_of_a_frame_is_sweet_and_a_weaker_one_sour as fn()),
        ("gut_checks_gust_is_sour_beside_the_hit_of_equal_damage", gut_checks_gust_is_sour_beside_the_hit_of_equal_damage as fn()),
        ("the_ranking_follows_the_defenders_percent_and_weight", the_ranking_follows_the_defenders_percent_and_weight as fn()),
        ("equal_hitboxes_and_a_lone_hitbox_have_no_label", equal_hitboxes_and_a_lone_hitbox_have_no_label as fn()),
        ("ties_at_the_top_are_all_sweet", ties_at_the_top_are_all_sweet as fn()),
        ("a_hitbox_weaker_than_an_earlier_one_of_the_action_is_late", a_hitbox_weaker_than_an_earlier_one_of_the_action_is_late as fn()),
        ("the_peak_starts_over_with_a_new_status_or_when_the_motion_goes_back", the_peak_starts_over_with_a_new_status_or_when_the_motion_goes_back as fn()),
        ("a_hit_of_equal_damage_is_attributed_by_the_slot_the_game_recorded", a_hit_of_equal_damage_is_attributed_by_the_slot_the_game_recorded as fn()),
        ("a_hit_both_readings_cannot_place_has_no_label", a_hit_both_readings_cannot_place_has_no_label as fn()),
        ("a_hit_is_attributed_by_the_damage_it_added", a_hit_is_attributed_by_the_damage_it_added as fn()),
        ("staling_and_multipliers_cancel_in_the_ratio", staling_and_multipliers_cancel_in_the_ratio as fn()),
        ("a_hit_that_cannot_be_told_has_no_label", a_hit_that_cannot_be_told_has_no_label as fn()),
        ("the_falcon_knee_hits_eden_logged", the_falcon_knee_hits_eden_logged as fn()),
        ("a_charged_smash_is_attributed_by_the_hitbox_that_dealt_it", a_charged_smash_is_attributed_by_the_hitbox_that_dealt_it as fn()),
        ("the_charged_marth_hits_eden_logged", the_charged_marth_hits_eden_logged as fn()),
        ("a_charged_hit_that_cannot_be_told_has_no_label", a_charged_hit_that_cannot_be_told_has_no_label as fn()),
        ("the_knockback_percent_is_the_games_on_recorded_hits", the_knockback_percent_is_the_games_on_recorded_hits as fn()),
        ("an_uncharged_hit_at_full_rage_stays_labelled", an_uncharged_hit_at_full_rage_stays_labelled as fn()),
        ("fixed_knockback_takes_no_rage", fixed_knockback_takes_no_rage as fn()),
        ("the_knockback_ceiling_is_the_games", the_knockback_ceiling_is_the_games as fn()),
        ("an_unusable_multiplier_gives_no_label", an_unusable_multiplier_gives_no_label as fn()),
        ("the_powers_of_a_fighter_are_read_from_its_active_slots_only", the_powers_of_a_fighter_are_read_from_its_active_slots_only as fn()),
        ("the_hit_spheres_take_the_frame_data_colours", the_hit_spheres_take_the_frame_data_colours as fn()),
        ("one_function_chooses_every_tint", one_function_chooses_every_tint as fn()),
        ("off_is_exactly_the_old_drawing", off_is_exactly_the_old_drawing as fn()),
        ("the_spot_words_and_rows", the_spot_words_and_rows as fn()),
    ]);
}
}
