lib_unit_tests! {
use super::*;
use crate::editors::roster::RosterEditor;
use crate::model::edge_juggle::picker::{drills, DrillId, Mode};
use crate::model::settings::parse;

const GAMES: [Drill; 2] = [Drill::GuardEdge, Drill::Juggler];
const HERO: u32 = 0x53;

fn open(menu: &mut Menu, editor: Editor, use_: RosterUse) -> RosterEditor {
    menu.open_editor(editor);
    let (cell, button) = menu.take_roster_press().unwrap();
    let mut grid = RosterEditor::new(use_);
    grid.apply(menu, usize::from(cell), button);
    grid
}

fn select(menu: &mut Menu, game: Drill, kind: u32) {
    use crate::engine::swap::roster_grid::{Role, RosterGrid, CELLS};
    let mut editor = open(menu, Editor::PracticeCpu(game), RosterUse::PracticeCpu(game));
    let grid = RosterGrid::new(Role::Cpu);
    let cell = (0..CELLS).find(|&i| grid.info(i).and_then(|c| c.kind()) == Some(kind)).unwrap();
    editor.apply(menu, cell, Button::A);
    assert_eq!(menu.practice_cpu_popup(), Some(cell));
}

#[test]
fn practice_pickers_share_mashout_grid() {
    for game in GAMES {
        let mut m = Menu::new(Settings::factory(), Env::default());
        let mut p = Menu::new(Settings::factory(), Env::default());
        let mash = open(&mut m, Editor::MashoutPicker, RosterUse::Mashout);
        let pick = open(&mut p, Editor::PracticeCpu(game), RosterUse::PracticeCpu(game));
        let (Some(Screen::Grid { layout: a, .. }), Some(Screen::Grid { layout: b, .. })) =
            (m.screen(), p.screen()) else { panic!("expected both portrait grids") };
        assert_eq!((a.cols, a.rows, a.tiles()), (b.cols, b.rows, b.tiles()));
        let mut a = crate::model::frame::Frame::new();
        let mut b = crate::model::frame::Frame::new();
        mash.decorate_mashout(&mut a, &m);
        pick.decorate_mashout(&mut b, &p);
        assert_eq!(a.tile_n, b.tile_n);
        assert!(a.tile_roster_grid && b.tile_roster_grid);
        assert!(a.tile_cpu && b.tile_cpu);
    }
}

#[test]
fn fighter_click_cycles_modes() {
    for game in GAMES {
        for (kind, modes) in [
            (HERO, &[Mode::Cpu(9), Mode::Perfect, Mode::Drill(DrillId::Zoom)][..]),
            (0, &[Mode::Cpu(9), Mode::Perfect][..]),
        ] {
            let mut m = Menu::new(Settings::factory(), Env::default());
            select(&mut m, game, kind);
            for &mode in modes {
                assert_eq!(m.practice_cpu_pick(game).mode(), mode);
                m.press(Button::A);
            }
            assert_eq!(m.practice_cpu_pick(game).mode(), Mode::Cpu(9));
            m.press(Button::B);
            assert_eq!(m.practice_cpu_popup(), None);
            assert_eq!(m.practice_cpu_pick(game).fighter, Some(kind));
        }
    }
}

#[test]
fn x_cycles_all_stock_cpu_levels() {
    use crate::model::cpu::level::{training_drill_target, CpuLevel, Where, RANK_TABLE_13_0_5};
    use crate::model::edge_juggle::CpuBehavior;
    for game in GAMES {
        let mut m = Menu::new(Settings::factory(), Env::default());
        select(&mut m, game, HERO);
        for (expected, rank) in (1..=9).zip([0, 15, 21, 31, 42, 48, 60, 75, 100]) {
            m.press(Button::X);
            let mode = m.practice_cpu_pick(game).mode();
            assert_eq!(mode, Mode::Cpu(expected));
            let level = mode.stock_level();
            let cpu = CpuBehavior::stock(level);
            assert_eq!(cpu.level, u32::from(expected - 1));
            assert_eq!(training_drill_target(
                Some(level), CpuLevel::Off, Where::Training { behavior: cpu.menu }, 1, RANK_TABLE_13_0_5,
            ), Some(rank));
        }
        m.press(Button::X);
        assert_eq!(m.practice_cpu_pick(game).mode(), Mode::Cpu(1));
        m.press(Button::A);
        m.press(Button::X);
        m.press(Button::A);
        m.press(Button::X);
        m.press(Button::A);
        assert_eq!(m.practice_cpu_pick(game).mode(), Mode::Cpu(1));
    }
}

#[test]
fn hero_drill_table() {
    for &(kind, _) in crate::engine::swap::trio::CHOICES {
        let expected: &[DrillId] = if kind == HERO { &[DrillId::Zoom] } else { &[] };
        assert_eq!(drills(Some(kind)), expected);
    }
    assert!(drills(None).is_empty());
}

#[test]
fn picker_settings_round_trip_and_migrate() {
    let old = parse(r#"{"menu":{"edge_difficulty":[0,1],"edge_reaction":[0,0,1],"juggle_difficulty":[1,0],"juggle_reaction":[1,0,0]}}"#);
    let mut m = Menu::new(old.settings, Env::default());
    assert_eq!(m.practice_cpu_pick(Drill::GuardEdge).mode(), Mode::Perfect);
    assert_eq!(m.practice_cpu_pick(Drill::Juggler).mode(), Mode::Cpu(9));
    assert_eq!(m.practice_cpu_pick(Drill::GuardEdge).fighter, None);
    select(&mut m, Drill::GuardEdge, HERO);
    m.press(Button::A);
    select(&mut m, Drill::Juggler, 0);
    m.press(Button::X);
    m.put_one_hot("edge_zoom_window", 29);
    let saved = parse(&m.settings().to_json());
    let back = Menu::new(saved.settings, Env::default());
    assert_eq!(back.practice_cpu_pick(Drill::GuardEdge).mode(), Mode::Drill(DrillId::Zoom));
    assert_eq!(back.practice_cpu_pick(Drill::GuardEdge).fighter, Some(HERO));
    assert_eq!(back.practice_cpu_pick(Drill::Juggler).mode(), Mode::Cpu(1));
    assert_eq!(back.practice_cpu_pick(Drill::Juggler).fighter, Some(0));
    assert_eq!(back.get("edge_reaction"), &[0, 0, 1]);
    assert_eq!(back.get("juggle_reaction"), &[1, 0, 0]);
    assert_eq!(crate::model::tech_settings::selected(&back.settings().menu, "edge_zoom_window"), 29);
}
}
