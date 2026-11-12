//! SPEC-cpu-02 (DI, SDI, air-dodge direction, clatter, dodge staling) and the decisions it carries:
//! B5 / N4 (the recording wins by default), B6 (one state per fighter), B7 (In / Out toward the
//! opponent on both sides), Q18 A (SDI rolls on a reset and on a row change, not per hitlag), N9 (SDI
//! not set means no SDI). Test names are the spec's §9 rows.

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use core::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use model::cpu::defence::{
    angle, clatter_period, direction, sdi_period, stick_vector, Defence, SdiAnswer, CLATTER_STEP_DEFAULT,
};
use model::cpu::facts::FrameFacts;
use model::cpu::ids::{Table, C};
use model::cpu::playback::PlaybackState;
use model::cpu::rng::Rng;
use model::cpu::role::FighterKey;
use model::cpu::settings::{CpuSettings, Strength};
use model::cpu::state::Cpu;
use model::cpu::status::Situation;
use model::lra_reset::HubStep;
use model::menu::reroll;
use model::settings::Snapshot;

fn settings(rows: &[(&str, &[u8])]) -> CpuSettings {
    let mut snap = Snapshot::factory();
    for (id, row) in rows {
        snap.set(id, row).unwrap();
    }
    CpuSettings::from_snapshot(&snap)
}

/// A direction row with weight 1 on each listed direction.
fn dirs(at: &[u8]) -> [u8; 11] {
    let mut row = [0u8; 11];
    for i in at {
        row[usize::from(*i)] = 1;
    }
    row
}

fn near(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 1e-5 && (a.1 - b.1).abs() < 1e-5
}

fn rng() -> Rng {
    Rng::new(0xfeed_beef)
}

const HITSTUN: i32 = 11;
const CLATTER: i32 = 80;

/// A table where a status of 10..14 is hitstun and 80 is a clatter status.
fn table() -> Table {
    Table::with(&[
        (C::StatusDamage, 10),
        (C::StatusDamageFall, 14),
        (C::ClatterBury, CLATTER),
        (C::StatusWait, 1),
    ])
}

/// Run a frame in which the body is in the air and in hitstun, so the air-dodge stick sees it.
fn into_aerial_hitstun(cpu: &mut Cpu, key: FighterKey) {
    cpu.frame(key, &FrameFacts { status: HITSTUN, situation: Situation::Air, ..FrameFacts::default() });
}

fn cpu_with(s: CpuSettings) -> (Cpu, FighterKey) {
    let mut cpu = Cpu::new(s, 11);
    cpu.ids = table();
    (cpu, FighterKey::new(1, false).unwrap())
}

fn cpu02_direction_angles() {
    assert_eq!(angle(direction::OUT), Some(0.0));
    assert_eq!(angle(direction::UP_OUT), Some(FRAC_PI_4));
    assert_eq!(angle(direction::UP), Some(FRAC_PI_2));
    assert_eq!(angle(direction::UP_IN), Some(3.0 * FRAC_PI_4));
    assert_eq!(angle(direction::IN), Some(PI));
    assert_eq!(angle(direction::DOWN_IN), Some(5.0 * FRAC_PI_4));
    assert_eq!(angle(direction::DOWN), Some(3.0 * FRAC_PI_2));
    assert_eq!(angle(direction::DOWN_OUT), Some(7.0 * FRAC_PI_4));
    assert_eq!(angle(direction::NEUTRAL), None, "Neutral is the stick at rest");
    assert_eq!(angle(direction::LEFT), Some(PI));
    assert_eq!(angle(direction::RIGHT), Some(0.0));
    assert!(near(stick_vector(direction::NEUTRAL, 0.0, 5.0), (0.0, 0.0)));
}

fn cpu02_unit_vectors_are_cos_and_sin_of_the_angle() {
    // The stick vector uses exact compass vectors (no libm in the image); it must equal (cos a, sin a),
    // and mirrored (cos (π − a), sin (π − a)).
    for dir in 0..=10u8 {
        let Some(a) = angle(dir) else { continue };
        assert!(near(stick_vector(dir, -10.0, 0.0), (a.cos(), a.sin())), "direction {dir}");
        let mirrored = if dir == direction::LEFT || dir == direction::RIGHT { a } else { PI - a };
        assert!(near(stick_vector(dir, 10.0, 0.0), (mirrored.cos(), mirrored.sin())), "mirrored direction {dir}");
    }
}

fn cpu02_mirror_rule() {
    // The player on the CPU's left (x smaller): no mirroring, Out points right (away from the player).
    assert!(near(stick_vector(direction::OUT, -10.0, 0.0), (1.0, 0.0)));
    assert!(near(stick_vector(direction::IN, -10.0, 0.0), (-1.0, 0.0)));
    // The player on the CPU's right: the angle becomes π − a, so Out points left, In points right,
    // Up stays up and Up Out leans left.
    assert!(near(stick_vector(direction::OUT, 10.0, 0.0), (-1.0, 0.0)));
    assert!(near(stick_vector(direction::IN, 10.0, 0.0), (1.0, 0.0)));
    assert!(near(stick_vector(direction::UP, 10.0, 0.0), (0.0, 1.0)));
    let up_out = stick_vector(direction::UP_OUT, 10.0, 0.0);
    assert!(up_out.0 < 0.0 && up_out.1 > 0.0);
    // Left and Right are absolute on both sides.
    for player_x in [-10.0, 10.0] {
        assert!(near(stick_vector(direction::LEFT, player_x, 0.0), (-1.0, 0.0)));
        assert!(near(stick_vector(direction::RIGHT, player_x, 0.0), (1.0, 0.0)));
    }
}

fn cpu02_di_case_lifetime() {
    let s = settings(&[("di_state", &dirs(&[direction::UP]))]);
    let mut d = Defence::new();
    let mut r = rng();
    // The first correction rolls a case; every later correction in the same hitstun keeps it, even
    // when the row changes in between.
    assert!(near(d.di_correction(&s, &mut r, 10.0, 0.0).unwrap(), (0.0, 1.0)));
    let changed = settings(&[("di_state", &dirs(&[direction::DOWN]))]);
    assert!(near(d.di_correction(&changed, &mut r, 10.0, 0.0).unwrap(), (0.0, 1.0)), "held for the episode");
    d.di_frame(true);
    assert_eq!(d.di_case, Some(direction::UP));
    // Outside hitstun the case clears and the next hit rolls anew from the current row.
    d.di_frame(false);
    assert_eq!(d.di_case, None);
    assert!(near(d.di_correction(&changed, &mut r, 10.0, 0.0).unwrap(), (0.0, -1.0)));
    // Neutral writes an explicit (0, 0); an empty row writes nothing.
    let neutral = settings(&[("di_state", &dirs(&[direction::NEUTRAL]))]);
    let mut d = Defence::new();
    assert_eq!(d.di_correction(&neutral, &mut r, 0.0, 0.0), Some((0.0, 0.0)));
    let empty = settings(&[]);
    assert_eq!(Defence::new().di_correction(&empty, &mut r, 0.0, 0.0), None);
}

fn cpu02_sdi_period() {
    assert_eq!(sdi_period(Strength::None), None);
    assert_eq!(sdi_period(Strength::Normal), Some(8));
    assert_eq!(sdi_period(Strength::Medium), Some(6));
    assert_eq!(sdi_period(Strength::High), Some(4));
    for (strength, row, period) in [(Strength::Normal, [0u8, 1, 0, 0], 8u32), (Strength::Medium, [0, 0, 1, 0], 6), (Strength::High, [0, 0, 0, 1], 4)] {
        let s = settings(&[("sdi_state", &dirs(&[direction::UP])), ("sdi_strength", &row)]);
        assert_eq!(s.sdi_strength, strength);
        let mut d = Defence::new();
        d.sdi_roll(&s, &mut rng());
        let mut inputs = Vec::new();
        for query in 1..=period * 3 {
            if let SdiAnswer::Input(x, y) = d.sdi_query(&s, 5.0, 0.0) {
                assert!(near((x, y), (0.0, 1.0)));
                inputs.push(query);
            }
        }
        // One input every `period` queries: the input frame is when the counter reads period − 1.
        assert_eq!(inputs, vec![period - 1, period * 2 - 1, period * 3 - 1], "period {period}");
    }
}

fn cpu02_sdi_reroll() {
    // The direction is rolled by the reset bundle and held; the period counter restarts with it.
    let s = settings(&[("sdi_state", &dirs(&[direction::UP, direction::DOWN])), ("sdi_strength", &[0, 1, 0, 0])]);
    let mut d = Defence::new();
    let mut r = rng();
    d.sdi_roll(&s, &mut r);
    let first = d.sdi_dir.unwrap();
    for _ in 0..7 {
        d.sdi_query(&s, 5.0, 0.0);
    }
    assert_eq!(d.sdi_counter, 7);
    d.sdi_roll(&s, &mut r);
    assert_eq!(d.sdi_counter, 0, "rolling restarts the counter");
    assert!([direction::UP, direction::DOWN].contains(&first));
}

fn cpu02_sdi_rolls_on_row_change_and_reset() {
    // Q18 A: no reset is needed. The menu changes the row; the next CPU frame rolls the new direction.
    let up = settings(&[("sdi_state", &dirs(&[direction::UP])), ("sdi_strength", &[0, 0, 0, 1])]);
    let (mut cpu, key) = cpu_with(up);
    cpu.fighters[key.index()].pending = 0;
    cpu.hub_step(HubStep::Sdi);
    assert_eq!(cpu.fighters[key.index()].defence.sdi_dir, Some(direction::UP));
    let input = |cpu: &mut Cpu| {
        for _ in 0..4 {
            if let SdiAnswer::Input(x, y) = cpu.sdi_query(key, 5.0, 0.0) {
                return Some((x, y));
            }
        }
        None
    };
    assert!(near(input(&mut cpu).unwrap(), (0.0, 1.0)));
    // SDI In: the menu commit leaves the roll pending, the next frame performs it, no reset happened.
    let inward = settings(&[("sdi_state", &dirs(&[direction::IN])), ("sdi_strength", &[0, 0, 0, 1])]);
    cpu.publish(inward, reroll::SDI_DIRECTION);
    assert_eq!(cpu.fighters[key.index()].defence.sdi_dir, Some(direction::UP), "nothing changes until the next frame");
    cpu.frame(key, &FrameFacts { status: 1, ..FrameFacts::default() });
    assert_eq!(cpu.fighters[key.index()].defence.sdi_dir, Some(direction::IN));
    assert_eq!(cpu.fighters[key.index()].pending & reroll::SDI_DIRECTION, 0);
    // Player on the CPU's right: In means toward the player, to the right.
    let got = input(&mut cpu).unwrap();
    assert!(near(got, (1.0, 0.0)), "{got:?}");
    // A reset re-rolls too.
    cpu.hub_step(HubStep::Sdi);
    assert_eq!(cpu.fighters[key.index()].defence.sdi_dir, Some(direction::IN));
}

fn cpu02_sdi_roll_not_per_hitlag() {
    // Two directions selected: the CPU keeps the one rolled at the reset across many hitlag episodes
    // (the reset or a row change is the only roll), unlike a roll per episode.
    let s = settings(&[("sdi_state", &dirs(&[direction::UP, direction::DOWN])), ("sdi_strength", &[0, 0, 0, 1])]);
    let (mut cpu, key) = cpu_with(s);
    cpu.hub_step(HubStep::Sdi);
    let held = cpu.fighters[key.index()].defence.sdi_dir;
    for _episode in 0..40 {
        for _ in 0..12 {
            cpu.sdi_query(key, 5.0, 0.0);
        }
        // Between episodes the body has frames outside hitlag; nothing re-rolls.
        cpu.frame(key, &FrameFacts { status: 1, ..FrameFacts::default() });
        assert_eq!(cpu.fighters[key.index()].defence.sdi_dir, held);
    }
}

fn cpu02_sdi_unset_means_no_sdi() {
    // N9: an empty direction, or Strength None, answers "no input" and never "call the original".
    let none_dir = settings(&[("sdi_strength", &[0, 0, 0, 1])]);
    let (mut cpu, key) = cpu_with(none_dir);
    cpu.hub_step(HubStep::Sdi);
    for _ in 0..20 {
        assert_eq!(cpu.sdi_query(key, 5.0, 0.0), SdiAnswer::NoInput);
    }
    let none_strength = settings(&[("sdi_state", &dirs(&[direction::UP]))]);
    let (mut cpu, key) = cpu_with(none_strength);
    cpu.hub_step(HubStep::Sdi);
    for _ in 0..20 {
        assert_eq!(cpu.sdi_query(key, 5.0, 0.0), SdiAnswer::NoInput);
    }
    // Neutral never reports an input either.
    let neutral = settings(&[("sdi_state", &dirs(&[direction::NEUTRAL])), ("sdi_strength", &[0, 0, 0, 1])]);
    let (mut cpu, key) = cpu_with(neutral);
    cpu.hub_step(HubStep::Sdi);
    assert!((0..20).all(|_| cpu.sdi_query(key, 5.0, 0.0) == SdiAnswer::NoInput));
}

fn cpu02_airdodge_direction_consistency() {
    let s = settings(&[("air_dodge_dir", &dirs(&[direction::UP, direction::DOWN, direction::OUT, direction::IN]))]);
    let mut d = Defence::new();
    let mut r = rng();
    for episode in 0..200 {
        // Every x and y read of one aerial-hitstun episode comes from the same pick.
        let first = d.air_dodge_stick(&s, &mut r, true, 5.0, 0.0).unwrap();
        for _ in 0..5 {
            assert_eq!(d.air_dodge_stick(&s, &mut r, true, 5.0, 0.0), Some(first), "episode {episode}");
        }
        // A read outside aerial hitstun ends the episode and answers nothing.
        assert_eq!(d.air_dodge_stick(&s, &mut r, false, 5.0, 0.0), None);
    }
    // An empty row answers nothing; Neutral is the stick at rest.
    assert_eq!(Defence::new().air_dodge_stick(&settings(&[]), &mut r, true, 5.0, 0.0), None);
    let neutral = settings(&[("air_dodge_dir", &dirs(&[direction::NEUTRAL]))]);
    assert_eq!(Defence::new().air_dodge_stick(&neutral, &mut r, true, 5.0, 0.0), Some((0.0, 0.0)));
    // Over many episodes every selected direction is rolled.
    let mut seen = std::collections::BTreeSet::new();
    let mut d = Defence::new();
    for _ in 0..200 {
        d.air_dodge_stick(&s, &mut r, false, 0.0, 0.0);
        let v = d.air_dodge_stick(&s, &mut r, true, -5.0, 0.0).unwrap();
        seen.insert(((v.0 * 100.0).round() as i32, (v.1 * 100.0).round() as i32));
    }
    assert_eq!(seen.len(), 4);
}

fn cpu02_airdodge_opponent_relative_both_sides() {
    // B7: Out is away from the opponent and In toward it, whichever side the player stands on.
    let out = settings(&[("air_dodge_dir", &dirs(&[direction::OUT]))]);
    let inward = settings(&[("air_dodge_dir", &dirs(&[direction::IN]))]);
    let stick = |s: &CpuSettings, player_x: f32| Defence::new().air_dodge_stick(s, &mut Rng::new(1), true, player_x, 0.0).unwrap();
    assert!(near(stick(&out, -8.0), (1.0, 0.0)), "player on the left: out is right");
    assert!(near(stick(&out, 8.0), (-1.0, 0.0)), "player on the right: out is left");
    assert!(near(stick(&inward, -8.0), (-1.0, 0.0)));
    assert!(near(stick(&inward, 8.0), (1.0, 0.0)));
}

fn cpu02_clatter_period_and_step() {
    assert_eq!(clatter_period(Strength::None), None);
    assert_eq!(clatter_period(Strength::Normal), Some(8));
    assert_eq!(clatter_period(Strength::Medium), Some(5), "the code's rates, not the README's");
    assert_eq!(clatter_period(Strength::High), Some(2));
    let s = settings(&[("clatter_strength", &[0, 0, 0, 1])]);
    let mut d = Defence::new();
    assert_eq!(d.clatter_step, CLATTER_STEP_DEFAULT);
    // The step is the manual recovery rate of the start call (freeze is 4); an input frame removes it.
    d.clatter_start(4.0);
    let got: Vec<_> = (0..6).map(|_| d.clatter_frame(&s)).collect();
    // Period 2: the counter reads 1 = period − 1 on the first call, so the input frames are the odd calls.
    assert_eq!(got, vec![Some(-4.0), None, Some(-4.0), None, Some(-4.0), None]);
    // Normal (period 8): the counter reads 7 = period − 1 after the seventh call, which is therefore
    // the first input frame, and then every eighth.
    let normal = settings(&[("clatter_strength", &[0, 1, 0, 0])]);
    let mut d = Defence::new();
    let got: Vec<_> = (0..16).map(|_| d.clatter_frame(&normal)).collect();
    assert_eq!(got.iter().position(Option::is_some), Some(6));
    assert_eq!(got.iter().filter(|g| g.is_some()).count(), 2);
    // Strength None never inputs.
    let mut d = Defence::new();
    assert!((0..30).all(|_| d.clatter_frame(&settings(&[])).is_none()));
    // The frame dispatch calls it only in a clatter status, and with the effect on the same frame.
    let (mut cpu, key) = cpu_with(s);
    let outs: Vec<_> = (0..4).map(|_| cpu.frame(key, &FrameFacts { status: CLATTER, ..FrameFacts::default() }).clatter).collect();
    assert_eq!(outs, vec![Some(-8.0), None, Some(-8.0), None]);
    let outside = cpu.frame(key, &FrameFacts { status: 1, ..FrameFacts::default() });
    assert_eq!(outside.clatter, None, "outside clatter the counter is left alone");
}

fn cpu02_stale_flag_follows_setting() {
    // The flag equals !stale_dodges; it is written on the first frame and on any change, never every
    // frame, and a forgotten write (a reset) is repeated.
    let staling_on = settings(&[]);
    let (mut cpu, key) = cpu_with(staling_on);
    let wait = FrameFacts { status: 1, ..FrameFacts::default() };
    assert_eq!(cpu.frame(key, &wait).stale_flag, Some(false), "staling On: the penalty flag is false");
    assert_eq!(cpu.frame(key, &wait).stale_flag, None);
    // The menu turns staling Off: the next frame writes true, with no reset.
    cpu.publish(settings(&[("stale_dodges", &[0, 1])]), 0);
    assert_eq!(cpu.frame(key, &wait).stale_flag, Some(true));
    assert_eq!(cpu.frame(key, &wait).stale_flag, None);
    assert!(Defence::stale_flag_wanted(&settings(&[("stale_dodges", &[0, 1])])));
    assert!(!Defence::stale_flag_wanted(&staling_on));
    cpu.fighters[key.index()].defence.stale_forget();
    assert_eq!(cpu.frame(key, &wait).stale_flag, Some(true));
}

fn cpu02_per_fighter_state() {
    // B6: Popo's SDI counter, DI case, air-dodge case and clatter step do not leak into Nana's.
    let s = settings(&[
        ("sdi_state", &dirs(&[direction::UP])),
        ("sdi_strength", &[0, 0, 0, 1]),
        ("di_state", &dirs(&[direction::DOWN])),
    ]);
    let (mut cpu, popo) = cpu_with(s);
    let nana = FighterKey::new(1, true).unwrap();
    cpu.hub_step(HubStep::Sdi);
    for _ in 0..3 {
        cpu.sdi_query(popo, 5.0, 0.0);
    }
    cpu.clatter_start(popo, 4.0);
    cpu.di_correction(popo, 5.0, 0.0);
    assert_eq!(cpu.fighters[popo.index()].defence.sdi_counter, 3);
    assert_eq!(cpu.fighters[nana.index()].defence.sdi_counter, 0);
    assert_eq!(cpu.fighters[nana.index()].defence.clatter_step, CLATTER_STEP_DEFAULT);
    assert_eq!(cpu.fighters[nana.index()].defence.di_case, None);
    // Nana runs her own period: her first two queries are not input frames (Popo's third was), her
    // third is.
    assert_eq!(cpu.sdi_query(nana, 5.0, 0.0), SdiAnswer::NoInput);
    assert_eq!(cpu.sdi_query(nana, 5.0, 0.0), SdiAnswer::NoInput);
    assert!(matches!(cpu.sdi_query(nana, 5.0, 0.0), SdiAnswer::Input(..)));
    assert_eq!(cpu.fighters[popo.index()].defence.sdi_counter, 3, "Nana's queries did not move Popo's counter");
}

fn cpu02_playback_wins_default() {
    let s = settings(&[
        ("di_state", &dirs(&[direction::UP])),
        ("sdi_state", &dirs(&[direction::UP])),
        ("sdi_strength", &[0, 0, 0, 1]),
        ("air_dodge_dir", &dirs(&[direction::UP])),
    ]);
    let (mut cpu, key) = cpu_with(s);
    cpu.hub_step(HubStep::Sdi);
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    // While a playback possesses the CPU and the override row is Off, none of the three overrides acts.
    assert_eq!(cpu.di_correction(key, 5.0, 0.0), None);
    assert_eq!(cpu.sdi_query(key, 5.0, 0.0), SdiAnswer::Native, "the SDI hook calls the original");
    into_aerial_hitstun(&mut cpu, key);
    assert_eq!(cpu.air_dodge_stick(key, 5.0, 0.0), None);
    // They apply again when the playback ends.
    cpu.playback = PlaybackState::default();
    assert!(cpu.di_correction(key, 5.0, 0.0).is_some());
    assert!(cpu.air_dodge_stick(key, 5.0, 0.0).is_some());
}

fn cpu02_playback_override_on() {
    let s = settings(&[
        ("di_state", &dirs(&[direction::UP])),
        ("sdi_state", &dirs(&[direction::UP])),
        ("sdi_strength", &[0, 0, 0, 1]),
        ("air_dodge_dir", &dirs(&[direction::UP])),
        ("settings_override_playback", &[1, 0]),
    ]);
    let (mut cpu, key) = cpu_with(s);
    cpu.hub_step(HubStep::Sdi);
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    into_aerial_hitstun(&mut cpu, key);
    assert!(cpu.di_correction(key, 5.0, 0.0).is_some(), "Settings override playback On: the settings win");
    assert!(cpu.air_dodge_stick(key, 5.0, 0.0).is_some());
    assert_ne!(cpu.sdi_query(key, 5.0, 0.0), SdiAnswer::Native);
}

fn cpu02_di_case_clears_in_the_frame_dispatch() {
    // The frame dispatch clears the DI case whenever the body is not in hitstun.
    let s = settings(&[("di_state", &dirs(&[direction::UP]))]);
    let (mut cpu, key) = cpu_with(s);
    cpu.di_correction(key, 5.0, 0.0);
    cpu.frame(key, &FrameFacts { status: HITSTUN, ..FrameFacts::default() });
    assert!(cpu.fighters[key.index()].defence.di_case.is_some());
    cpu.frame(key, &FrameFacts { status: 1, ..FrameFacts::default() });
    assert!(cpu.fighters[key.index()].defence.di_case.is_none());
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("cpu02_direction_angles", cpu02_direction_angles as fn()),
        ("cpu02_unit_vectors_are_cos_and_sin_of_the_angle", cpu02_unit_vectors_are_cos_and_sin_of_the_angle as fn()),
        ("cpu02_mirror_rule", cpu02_mirror_rule as fn()),
        ("cpu02_di_case_lifetime", cpu02_di_case_lifetime as fn()),
        ("cpu02_sdi_period", cpu02_sdi_period as fn()),
        ("cpu02_sdi_reroll", cpu02_sdi_reroll as fn()),
        ("cpu02_sdi_rolls_on_row_change_and_reset", cpu02_sdi_rolls_on_row_change_and_reset as fn()),
        ("cpu02_sdi_roll_not_per_hitlag", cpu02_sdi_roll_not_per_hitlag as fn()),
        ("cpu02_sdi_unset_means_no_sdi", cpu02_sdi_unset_means_no_sdi as fn()),
        ("cpu02_airdodge_direction_consistency", cpu02_airdodge_direction_consistency as fn()),
        ("cpu02_airdodge_opponent_relative_both_sides", cpu02_airdodge_opponent_relative_both_sides as fn()),
        ("cpu02_clatter_period_and_step", cpu02_clatter_period_and_step as fn()),
        ("cpu02_stale_flag_follows_setting", cpu02_stale_flag_follows_setting as fn()),
        ("cpu02_per_fighter_state", cpu02_per_fighter_state as fn()),
        ("cpu02_playback_wins_default", cpu02_playback_wins_default as fn()),
        ("cpu02_playback_override_on", cpu02_playback_override_on as fn()),
        ("cpu02_di_case_clears_in_the_frame_dispatch", cpu02_di_case_clears_in_the_frame_dispatch as fn()),
    ]);
}
