//! Stage tech menu contract and the shared practice-game exclusion boundary.
lib_unit_tests! {
use super::*;
use crate::model::tech_settings::{self, Mode};
fn row(id: &str) -> usize { ROWS.iter().position(|r| r.id == id).unwrap() }
fn menu() -> Menu { Menu::new(Settings::factory(), Env::default()) }

#[test]
fn tech_rows_follow_break_the_targets() {
    let layouts = row("btt_layouts");
    assert_eq!(ROWS[layouts + 1].id, "tech_practice");
    assert_eq!(ROWS[layouts + 1].page(), Page::FightersStages);
    assert_eq!(ROWS[layouts + 2].id, "tech_wizard");
    assert_eq!(ROWS[layouts + 2].page(), Page::FightersStages);
    let group = GROUPS.last().unwrap();
    assert_eq!((group.title, group.page), ("Tech practice", Page::FightersStages));
    let expected = ["tech_surface", "tech_point", "tech_damage", "tech_damage_range", "tech_speed", "tech_speed_range", "tech_hit_damage", "tech_hit_damage_range", "tech_angle", "tech_angle_range", "tech_start", "tech_start_range", "tech_time", "tech_time_range", "tech_di", "tech_di_direction", "tech_facing", "tech_delay", "tech_delay_range", "tech_camera", "tech_wizard_skip"];
    // rc15/mashout integration: Mash out's four Practice games rows follow Tech practice's rows; rc.15 Parry practice's
    // three rows are appended after them (artifacts/rc15-parry/DESIGN.md §2.1).
    let mash = ["mashout", "mashout_state", "mashout_attacker", "mashout_grab"];
    let parry = ["parry_practice", "parry_saved", "parry_save"];
    assert_eq!(&ROWS[layouts + 3..layouts + 3 + expected.len()].iter().map(|r| r.id).collect::<Vec<_>>(), &expected);
    let after = layouts + 3 + expected.len();
    assert_eq!(&ROWS[after..after + mash.len()].iter().map(|r| r.id).collect::<Vec<_>>(), &mash);
    assert_eq!(&ROWS[after + mash.len()..].iter().map(|r| r.id).collect::<Vec<_>>(), &parry);
    for r in &ROWS[layouts + 1..] { assert!(r.help.contains('\n')); }
}

#[test]
fn one_practice_game_at_a_time() {
    let ids = ["eggsercise", "btt", "tech_practice", "parry_practice"];
    for first in ids { for second in ids {
        let mut m = menu();
        m.put_one_hot(first, 1);
        m.put_one_hot(second, 1);
        assert_eq!(tech_settings::practice(&m.settings.menu).iter().filter(|v| **v).count(), 1);
        assert_eq!(tech_settings::selected(&m.settings.menu, second), 1);
        let committed = m.commit_onto(&Settings::factory());
        assert_eq!(tech_settings::practice(&committed.menu).iter().filter(|v| **v).count(), 1);
    } }
    let mut m = menu();
    m.cycle(row("tech_practice"), true);
    assert_eq!(tech_settings::read(&m.settings.menu).session, 1);
    m.put("tech_practice", &[0,0,0,0,0,1]);
    m.put("btt", &[0,1]);
    assert_eq!(tech_settings::read(&m.settings.menu).session, 0);
}

#[test]
fn tech_point_closes_into_surface_picker() {
    let mut m = menu();
    m.put_one_hot("eggsercise", 1);
    m.activate(row("tech_point"));
    assert!(m.is_closed());
    assert_eq!(tech_settings::practice(&m.settings.menu), [false, false, true, false]);
    assert_eq!(tech_settings::read(&m.settings.menu).surface, 4);
    assert_eq!(tech_settings::read(&m.settings.menu).session, 1);
    let out=m.take_outbox();
    assert!(out.tech_point);
    assert!(!out.tech_wizard);
    assert!(!m.take_outbox().tech_point);
}

#[test]
fn tech_wizard_off_starts_ten_attempts_and_excludes_other_games() {
    for game in ["eggsercise", "btt"] {
        let mut m=menu();
        m.put_one_hot(game,1);
        m.activate(row("tech_wizard"));
        assert!(m.is_closed());
        let effective=tech_settings::read(&m.settings.menu);
        assert_eq!((effective.session,effective.surface),(2,4));
        assert_eq!(tech_settings::practice(&m.settings.menu),[false,false,true,false]);
        let out=m.take_outbox();
        assert!(out.tech_wizard && out.changed);
        assert!(!out.tech_point && !out.place_targets);
        assert!(!m.take_outbox().tech_wizard);
        let committed=m.commit_onto(&Settings::factory());
        assert_eq!(tech_settings::read(&committed.menu).session,2);
        assert_eq!(tech_settings::read(&committed.menu).surface,4);
    }
}

#[test]
fn tech_wizard_preserves_active_session_and_advanced_variables() {
    for session in 1..6 {
        let mut m=menu();
        m.put_one_hot("tech_practice",session);
        m.put("tech_damage_range",&[4,8]);
        m.put_one_hot("tech_di",3);
        let mut expected=tech_settings::read(&m.settings.menu);
        expected.surface=4;
        m.activate(row("tech_wizard"));
        assert_eq!(tech_settings::read(&m.settings.menu),expected);
        assert!(m.is_closed());
        let out=m.take_outbox();
        assert!(out.tech_wizard);
        assert!(!out.tech_point);
    }
}

#[test]
fn tech_wizard_skip_defaults_off_and_persists_without_opening_wizard() {
    let mut m=menu();
    assert_eq!(m.get("tech_wizard_skip"),&[0,1]);
    m.activate(row("tech_wizard_skip"));
    assert_eq!(m.get("tech_wizard_skip"),&[1,0]);
    assert!(!m.is_closed());
    let out=m.take_outbox();
    assert!(out.changed);
    assert!(!out.tech_point && !out.tech_wizard);
    let committed=m.commit_onto(&Settings::factory());
    let loaded=settings::parse(&committed.to_json()).settings;
    assert_eq!(loaded.menu.get("tech_wizard_skip"),Some(&[1,0][..]));
    assert_eq!(tech_settings::read(&loaded.menu).session,0);
}

#[test]
fn tech_ranges_use_public_units_and_native_kb() {
    let mut s = settings::Snapshot::factory();
    let c = tech_settings::read(&s);
    assert_eq!((c.damage.lo, c.speed.lo, c.hit_damage.lo, c.time.lo, c.time.hi, c.delay.lo), (60.0,100.0,12.0,20.0,40.0,30.0));
    assert_eq!((c.angle.mode,c.start.mode,c.time.mode), (Mode::Auto,Mode::Auto,Mode::Random));
    for (id,max,lo,hi) in [("tech_damage_range",30,0.0,150.0),("tech_speed_range",17,80.0,250.0),("tech_hit_damage_range",29,1.0,30.0),("tech_angle_range",71,0.0,355.0),("tech_start_range",240,-120.0,120.0),("tech_time_range",50,10.0,60.0),("tech_delay_range",80,10.0,90.0)] {
        s.set(id,&[0,max]).unwrap();
        let n = tech_settings::numeric(&s,id);
        assert_eq!((n.lo,n.hi),(lo,hi));
        assert!(s.set(id,&[0,max+1]).is_err());
    }
    // Round-trip through the supported writer, including start's signed full range.
    let written = settings::Settings { menu:s.clone(), ..Settings::factory() }.to_json();
    assert_eq!(settings::parse(&written).settings.menu.get("tech_start_range"), Some(&[0,240][..]));
}

#[test]
fn tech_hit_damage_help_uses_visible_added_percent() {
    let help = ROWS[row("tech_hit_damage")].help;
    assert_eq!(help, "The percent the hit adds;\nit also sets how long the freeze lasts.");
    assert!(ROWS[row("tech_damage")].help.starts_with("Your percent before the hit."));
    let range = ROWS[row("tech_hit_damage_range")].help;
    assert!(range.contains("Added percent from 1 to 30, default 12."));
    assert!(range.contains("Set uses the low handle; Random uses both."));
    for text in [help, range] {
        for internal in ["not added", "multiplier", "resolver", "hitlag"] {
            assert!(!text.contains(internal), "unexpected help wording: {text}");
        }
    }
}

#[test]
fn tech_set_random_auto_controls_ranges() {
    let mut m=menu();
    for id in ["tech_angle_range","tech_start_range"] { assert_eq!(m.grey(row(id)),Some(Grey::TechAuto)); }
    m.put("tech_angle", &[0,1,0]);
    assert_eq!(m.grey(row("tech_angle_range")),None);
    assert_eq!(tech_settings::read(&m.settings.menu).angle.mode,Mode::Set);
    m.put("tech_angle", &[0,0,1]);
    assert_eq!(m.grey(row("tech_angle_range")),None);
    assert_eq!(tech_settings::read(&m.settings.menu).angle.mode,Mode::Random);
    m.put("tech_di", &[0,0,1,0]);
    assert_eq!(m.grey(row("tech_di_direction")),None);
}

#[test]
fn tech_file_conflicts_normalize_current_and_defaults() {
    let loaded=settings::parse(r#"{"menu":{"eggsercise":[0,1,0,0,0],"btt":[0,1],"tech_practice":[0,1,0,0,0,0]},"defaults_menu":{"btt":[0,1],"tech_practice":[0,0,0,0,0,1]}}"#);
    assert_eq!(tech_settings::practice(&loaded.settings.menu),[true,false,false,false]);
    assert_eq!(tech_settings::practice(&loaded.settings.defaults),[false,true,false,false]);
}
#[test]
fn tech_slider_edits_and_draws_public_units() {
    let mut m=menu();
    m.put("tech_start", &[0,1,0]);
    m.put("tech_start_range", &[0,240]);
    m.activate(row("tech_start_range"));
    m.press(Button::Right);
    m.press(Button::A);
    m.press(Button::Right);
    assert_eq!(m.get("tech_start_range"), &[0,240]);
    m.press(Button::Left);
    assert_eq!(m.get("tech_start_range"), &[0,239]);
    let mut f=crate::model::frame::Frame::new();
    crate::model::screens::frame_of(&m,&mut f,|_| "Stage");
    assert_eq!(f.rows[0].value.string_lossy(), "-120 units");
    assert_eq!(f.rows[1].value.string_lossy(), "119 units");
    let mut label=crate::model::frame::Text::<72>::new();
    crate::model::frame::write_value(&m,row("tech_start_range"),&mut label);
    assert_eq!(label.string_lossy(), "-120 units - 119 units  >");
}

#[test]
fn tech_di_grid_reuses_canonical_vectors() {
    for i in 0..11 {
        let actual=tech_settings::di_vector(i);
        let expected=crate::model::cpu::defence::stick_vector(i as u8,-1.0,0.0);
        assert_eq!((actual.x,actual.y),expected);
    }
}}
