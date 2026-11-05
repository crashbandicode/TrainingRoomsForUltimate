//! SPEC-core-02 (menu model) with draft 9 (`MENU-DRAFT.md`), SPEC-core-03 §2.5 (paging, shortcut rows,
//! delay popup), SPEC-core-04 (`last_page`) and SPEC-input-04 (N7 chord editing). Test names are the
//! specs' §9 rows where a row exists; rows that the new page layout replaced (the 8 x 4 submenu grid
//! of upstream) are covered by the test of the same name on the new structure, as noted on each.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::factory::DEFS;
use model::menu::chord::Chord;
use model::menu::delay::{DelayRange, DelaySpec};
use model::menu::grid::{ConsoleMode, GridKind, GridLayout, Move, RosterUse, ITEM_TILES, STAGE_TILES};
use model::menu::input::{Button, HoldRepeat, HOLD_REPEAT_PERIOD};
use model::menu::table::{row_count, Control, Editor, Page, GROUPS, ROWS};
use model::menu::{
    reroll, DelayView, Entry, Env, Grey, Menu, RowValue, Screen, SliderState,
};
use model::settings::{self, Kind, Settings};

// ---- helpers ---------------------------------------------------------------------------------

fn menu() -> Menu {
    Menu::new(Settings::factory(), Env::default())
}

fn menu_with(env: Env) -> Menu {
    Menu::new(Settings::factory(), env)
}

fn row_of(key: &str) -> usize {
    ROWS.iter().position(|r| r.id == key).or_else(|| ROWS.iter().position(|r| r.title == key)).unwrap_or_else(|| panic!("no row {key}"))
}

fn get(m: &Menu, id: &str) -> Vec<u8> {
    m.settings().menu.get(id).unwrap().to_vec()
}

fn set(m: &mut Menu, id: &str, array: &[u8]) {
    let mut s = m.settings().clone();
    s.menu.set(id, array).unwrap();
    let env = *m.env();
    m.reopen(s, env);
}

fn goto_entry(m: &mut Menu, entry: Entry) {
    for _ in 0..=m.entry_count(m.page()) {
        if m.focus() == Some(entry) {
            return;
        }
        m.press(Button::Down);
    }
    panic!("could not focus {entry:?}");
}

/// Focus a row (by setting id or title): change page, open its advanced group, move down to it.
fn goto(m: &mut Menu, key: &str) -> usize {
    let r = row_of(key);
    let page = ROWS[r].page();
    for _ in 0..6 {
        if m.page() == page {
            break;
        }
        m.press(Button::R);
    }
    assert_eq!(m.page(), page, "page of {key} is in the ring");
    let g = usize::from(ROWS[r].group);
    if !m.group_is_open(g) {
        goto_entry(m, Entry::Group(g));
        m.press(Button::A);
    }
    goto_entry(m, Entry::Row(r));
    r
}

fn press(m: &mut Menu, buttons: &[Button]) {
    for b in buttons {
        m.press(*b);
    }
}

/// The page's last line, *Reset page to defaults*, confirmed with A.
fn reset_page(m: &mut Menu) {
    let last = m.entry(m.page(), m.entry_count(m.page()) - 1).unwrap();
    assert!(matches!(last, Entry::ResetPage(_)), "{last:?}");
    goto_entry(m, last);
    press(m, &[Button::A, Button::A]);
    assert!(m.screen().is_none());
}

// ---- the factory table (SPEC-core-02 §2.7, §2.8, SPEC-core-04 §3.1) ------------------------------

/// `(id, kind, len, indices set to 1 in the factory value)`, written from the spec tables.
const SPEC_TABLE: &[(&str, Kind, usize, &[usize])] = &[
    ("mash_state", Kind::Weighted, 30, &[]),
    ("follow_up", Kind::Weighted, 30, &[]),
    ("mash_triggers", Kind::Multiple, 17, &[0, 1, 3, 7]),
    ("attack_angle", Kind::Weighted, 3, &[]),
    ("throw_state", Kind::Weighted, 5, &[0]),
    ("throw_delay", Kind::Weighted, 31, &[]),
    ("pummel_delay", Kind::Weighted, 31, &[]),
    ("falling_aerials", Kind::Weighted, 2, &[1]),
    ("full_hop", Kind::Weighted, 2, &[0]),
    ("aerial_delay", Kind::Weighted, 31, &[]),
    ("fast_fall", Kind::Weighted, 2, &[1]),
    ("fast_fall_delay", Kind::Weighted, 31, &[]),
    ("oos_offset", Kind::Weighted, 31, &[]),
    ("reaction_time", Kind::Weighted, 31, &[]),
    ("ledge_neutral_override", Kind::Weighted, 30, &[]),
    ("ledge_roll_override", Kind::Weighted, 30, &[]),
    ("ledge_jump_override", Kind::Weighted, 30, &[]),
    ("ledge_attack_override", Kind::Weighted, 30, &[]),
    ("tech_action_override", Kind::Weighted, 30, &[]),
    ("clatter_override", Kind::Weighted, 30, &[]),
    ("tumble_override", Kind::Weighted, 30, &[]),
    ("hitstun_override", Kind::Weighted, 30, &[]),
    ("parry_override", Kind::Weighted, 30, &[]),
    ("shieldstun_override", Kind::Weighted, 30, &[]),
    ("footstool_override", Kind::Weighted, 30, &[]),
    ("landing_override", Kind::Weighted, 30, &[]),
    ("trump_override", Kind::Weighted, 30, &[]),
    ("air_dodge_dir", Kind::Weighted, 11, &[]),
    ("di_state", Kind::Weighted, 11, &[]),
    ("sdi_state", Kind::Weighted, 11, &[]),
    ("sdi_strength", Kind::Single, 4, &[0]),
    ("clatter_strength", Kind::Single, 4, &[0]),
    ("ledge_state", Kind::Weighted, 10, &[0, 1, 2, 3]),
    ("ledge_delay", Kind::Weighted, 31, &[]),
    ("tech_state", Kind::Weighted, 4, &[0, 1, 2, 3]),
    ("miss_tech_state", Kind::Weighted, 4, &[0, 1, 2, 3]),
    ("shield_state", Kind::Single, 4, &[0]),
    ("shield_tilt", Kind::Single, 11, &[]),
    ("crouch", Kind::Single, 2, &[1]),
    ("stale_dodges", Kind::Single, 2, &[0]),
    ("tech_hide", Kind::Single, 2, &[1]),
    ("recording_slot", Kind::Single, 5, &[0]),
    ("record_trigger", Kind::Single, 2, &[0]),
    ("recording_duration", Kind::Single, 19, &[3]),
    ("recording_crop", Kind::Single, 2, &[0]),
    ("playback_button_slots", Kind::Weighted, 5, &[0]),
    ("hitstun_playback", Kind::Single, 3, &[0]),
    ("save_state_playback", Kind::Weighted, 5, &[]),
    ("playback_mash", Kind::Single, 2, &[0]),
    ("playback_loop", Kind::Single, 2, &[1]),
    ("save_state_save", Kind::Multiple, 16, &[6, 9]),
    ("save_state_load", Kind::Multiple, 16, &[6, 8]),
    ("input_record", Kind::Multiple, 16, &[7, 9]),
    ("input_playback", Kind::Multiple, 16, &[7, 8]),
    ("cpu_cycle", Kind::Multiple, 16, &[6, 10]),
    ("player_cycle", Kind::Multiple, 16, &[6, 11]),
    ("save_state_mirroring", Kind::Single, 3, &[0]),
    ("save_state_autoload", Kind::Single, 2, &[1]),
    ("save_damage_cpu", Kind::Single, 3, &[0]),
    ("save_damage_player", Kind::Single, 3, &[0]),
    ("save_state_enable", Kind::Single, 2, &[0]),
    ("save_state_slot", Kind::Single, 5, &[0]),
    ("randomize_slots", Kind::Weighted, 5, &[]),
    ("buff_state", Kind::Multiple, 18, &[]),
    ("frame_advantage", Kind::Single, 2, &[1]),
    ("hitbox_vis", Kind::Single, 2, &[1]),
    ("input_display_strip", Kind::Single, 2, &[1]),
    ("input_delay", Kind::Single, 31, &[0]),
    ("stage_hazards", Kind::Single, 2, &[1]),
    ("hud", Kind::Single, 2, &[0]),
    ("lra_reset", Kind::Single, 2, &[0]),
    // Draft 9 and the fork's rows.
    ("respawn_invincibility", Kind::Single, 2, &[1]),
    ("settings_override_playback", Kind::Single, 2, &[1]),
    ("dev_reload_config", Kind::Single, 2, &[1]),
    ("dev_debug_hooks", Kind::Single, 2, &[1]),
    ("cycle_preload_measurements", Kind::Single, 2, &[1]),
    ("macro_source", Kind::Single, 2, &[0]),
    ("fighter_prompt", Kind::Single, 2, &[0]),
    ("trio_practice", Kind::Single, 2, &[1]),
    ("lobby_practice", Kind::Single, 2, &[1]),
    ("training_stage_rotation", Kind::Single, 2, &[1]),
    ("solo_popo_original", Kind::Single, 2, &[1]),
    ("p1_cpu_control", Kind::Single, 2, &[1]),
    ("p1_cpu_level", Kind::Single, 9, &[2]),
    ("cycle_preload_slots", Kind::Single, 6, &[5]),
    ("cycle_preload_sources", Kind::Single, 3, &[2]),
    ("cycle_preload_bank_members", Kind::Single, 3, &[2]),
    // P-HOT: Ready Fighters 2..=12 starts at 10 (index 8).
    ("cycle_preload_ready_fighters", Kind::Single, 11, &[8]),
    ("cycle_preload_group_swap", Kind::Single, 2, &[0]),
    ("cycle_preload_ic_pair_bank", Kind::Single, 2, &[0]),
    ("cycle_preload_original_bank", Kind::Single, 2, &[0]),
    ("cycle_preload_solo_popo_groups", Kind::Single, 2, &[0]),
    ("cycle_preload_ic_pair_groups", Kind::Single, 2, &[0]),
    ("cycle_preload_native_groups", Kind::Single, 2, &[0]),
    ("cycle_preload_pair_groups", Kind::Single, 2, &[0]),
    ("cycle_preload_mii_groups", Kind::Single, 2, &[0]),
    // SPEC-input-07 §6 (combo helper, 2026-11-05): Off, 1 frame, keep playing.
    ("combo_helper", Kind::Single, 2, &[1]),
    ("combo_tolerance", Kind::Single, 3, &[1]),
    ("combo_after", Kind::Single, 2, &[0]),
];

#[test]
fn core02_factory_table_matches_spec() {
    for (id, kind, len, ones) in SPEC_TABLE {
        let (_, def) = settings::def(id).unwrap_or_else(|| panic!("{id} has no row"));
        assert_eq!(def.kind, *kind, "{id}");
        assert_eq!(def.len, *len, "{id}");
        let mut want = vec![0u8; *len];
        for i in *ones {
            want[*i] = 1;
        }
        assert_eq!(def.default, want.as_slice(), "{id}");
    }
    // The two damage sliders, and the seven delay ranges (absent by default).
    for id in ["save_damage_limits_cpu", "save_damage_limits_player"] {
        let (_, def) = settings::def(id).unwrap();
        assert_eq!((def.kind, def.len, def.default), (Kind::Slider, 2, &[0u8, 150][..]), "{id}");
    }
    for id in ["aerial_delay", "fast_fall_delay", "throw_delay", "pummel_delay", "ledge_delay", "oos_offset", "reaction_time"] {
        let (_, range) = settings::def(&format!("{id}_range")).unwrap();
        assert_eq!((range.kind, range.len, range.default.len()), (Kind::Range, 0, 0), "{id}");
    }
    // Nothing else is in the table, and no id is there twice.
    assert_eq!(DEFS.len(), SPEC_TABLE.len() + 2 + 7);
    for (i, def) in DEFS.iter().enumerate() {
        assert!(!DEFS[..i].iter().any(|d| d.id == def.id), "{} twice", def.id);
    }
    // The retired row is not in the table.
    assert!(settings::def("character_item").is_none());
    assert!(settings::def("update_policy").is_none());
}

#[test]
fn every_menu_row_with_an_id_binds_to_its_row_or_is_an_editor_key() {
    for row in &ROWS {
        if row.id.is_empty() {
            continue;
        }
        let in_table = settings::def(row.id).is_some();
        match row.control {
            // Rows that hold their value in the settings file have a table row.
            Control::Toggle | Control::Dropdown(_) | Control::YesNoRandom | Control::Number { .. } | Control::MultiSelect(_)
            | Control::MultiSelectRange { .. } | Control::Slider | Control::DirectionGrid { .. } => {
                assert!(in_table, "{} ({}) has no table row", row.title, row.id)
            }
            Control::Editor(_) => {}
            other => panic!("{} has an id but control {other:?}", row.title),
        }
        if let Control::MultiSelectRange { range_id, .. } = row.control {
            assert!(settings::def(range_id).is_some_and(|(_, d)| d.kind == Kind::Range), "{range_id}");
        }
    }
    // Option counts agree with the table rows.
    for row in &ROWS {
        let Some((_, def)) = settings::def(row.id) else { continue };
        match row.control {
            Control::Dropdown(labels) | Control::MultiSelect(labels) => assert_eq!(labels.len(), def.len, "{}", row.title),
            Control::YesNoRandom => assert_eq!(def.len, 2),
            Control::DirectionGrid { .. } => assert_eq!(def.len, 11),
            _ => {}
        }
    }
}

// ---- the layout (MENU-DRAFT §2 with the approved RC deferrals) -----------------------------------------------------------

#[test]
fn layout_row_counts_match_the_rc_freeze() {
    let counts = |pilot| {
        [Page::Cpu, Page::Saves, Page::Recording, Page::FightersStages, Page::DisplayControls, Page::Developer]
            .map(|page| row_count(page, pilot))
    };
    // Fighters & Stages has 21 since Change Stage (operator order via the primary, 2026-11-05 12:50Z); Display &
    // Controls lost "Group inputs by status" to the frame strip (SPEC-input-06 §3.1, 2026-11-05); Recording gained the
    // 7 rows of the Combos group (SPEC-input-07 §6, 2026-11-05).
    assert_eq!(counts(false), [31, 11, 22, 21, 10, 3]);
    assert_eq!(counts(true), [30, 11, 22, 21, 10, 3]);
    assert_eq!(counts(false)[..5].iter().sum::<usize>(), 95);
    assert_eq!(counts(true)[..5].iter().sum::<usize>(), 94);
    // Controls (MENU-DRAFT §2): 4 direction grids, 7 delay rows, 13 multi-selects (the 12 checklists,\n    // 3 of them slot sets, and Swap groups).
    let n = |f: &dyn Fn(&Control) -> bool| ROWS.iter().filter(|r| r.vis != model::menu::table::Vis::PilotPresent && f(&r.control)).count();
    assert_eq!(n(&|c| matches!(c, Control::DirectionGrid { .. })), 4);
    assert_eq!(n(&|c| matches!(c, Control::MultiSelectRange { .. })), 7);
    assert_eq!(n(&|c| matches!(c, Control::MultiSelect(_) | Control::SwapGroups)), 13, "12 checklists and Swap groups");
    assert_eq!(n(&|c| matches!(c, Control::Slider)), 2);
    // The RC defers four editors, retaining the empty Setup group index so
    // persisted group IDs do not move. Every other group still has rows.
    assert_eq!(ROWS.len(), 99);
    assert!(!ROWS.iter().any(|r| matches!(r.control,
        Control::Editor(Editor::SetupSave | Editor::SetupLibrary | Editor::EditPlacement | Editor::ReplayWindow))));
    assert_eq!(GROUPS[7].title, "Setup library");
    assert_eq!(GROUPS[7].page, Page::Saves);
    for (g, group) in GROUPS.iter().enumerate() {
        assert_eq!(ROWS.iter().any(|r| usize::from(r.group) == g), g != 7, "{}", group.title);
    }
    assert!(ROWS.iter().all(|r| usize::from(r.group) < GROUPS.len()));
    for pilot_present in [false, true] {
        let m = menu_with(Env { pilot_present, ..Env::default() });
        assert!((0..m.entry_count(Page::Saves)).all(|i| m.entry(Page::Saves, i) != Some(Entry::Group(7))));
    }
    for title in ["Named library", "Save slot to library...", "Replay capture"] {
        assert!(ROWS.iter().any(|r| r.title == title), "retained row {title}");
    }
}

#[test]
fn layout_the_pilot_swaps_the_fallback_rows_for_one_info_row() {
    let absent = menu_with(Env { pilot_present: false, ..Env::default() });
    let present = menu_with(Env { pilot_present: true, ..Env::default() });
    let titles = |m: &Menu| -> Vec<&'static str> {
        (0..m.entry_count(Page::Cpu))
            .filter_map(|i| match m.entry(Page::Cpu, i) {
                Some(Entry::Row(r)) => Some(ROWS[r].title),
                _ => None,
            })
            .collect()
    };
    assert!(titles(&absent).contains(&"P1 CPU Control") && titles(&absent).contains(&"P1 CPU Level"));
    assert!(!titles(&absent).contains(&"P1 control"));
    assert!(titles(&present).contains(&"P1 control"));
    assert!(!titles(&present).contains(&"P1 CPU Control"));
}

#[test]
fn layout_every_row_has_two_line_help() {
    for row in &ROWS {
        let (first, second) = row.help_lines();
        assert!(!first.is_empty(), "{} has no help", row.title);
        assert!(first.len() <= 80 && second.len() <= 80, "{}: a help line is too long", row.title);
        assert!(!row.help.contains("Shield+"), "{}: stale chord text", row.title);
    }
}

#[test]
fn layout_advanced_group_is_collapsed_until_opened() {
    let mut m = menu();
    let loading = GROUPS.iter().position(|g| g.title == "Loading").unwrap();
    assert!(GROUPS[loading].advanced && !m.group_is_open(loading));
    let closed = m.entry_count(Page::FightersStages);
    for _ in 0..3 {
        m.press(Button::R);
    }
    assert_eq!(m.page(), Page::FightersStages);
    goto_entry(&mut m, Entry::Group(loading));
    m.press(Button::A);
    assert!(m.group_is_open(loading));
    assert_eq!(m.entry_count(Page::FightersStages), closed + 6, "six Loading rows appear");
    m.press(Button::A);
    assert_eq!(m.entry_count(Page::FightersStages), closed);
    // Other groups are always open, and their headers are not focus stops.
    assert!(!GROUPS.iter().enumerate().any(|(g, group)| !group.advanced && !m.group_is_open(g)));
}

// ---- navigation, pages -------------------------------------------------------------------------

#[test]
fn core02_table_nav_wrap_skip() {
    // The page's list wraps in both directions and skips the headers of ordinary groups.
    let mut m = menu();
    let first = m.focused_row().expect("focus starts on a row, not a header");
    assert_eq!(ROWS[first].title, "Mash");
    m.press(Button::Up);
    assert!(matches!(m.focus(), Some(Entry::ResetPage(_))), "wraps to the page's last line, Reset page to defaults");
    m.press(Button::Up);
    let last = m.focused_row().unwrap();
    assert_eq!(ROWS[last].title, "P1 CPU Level", "the last row of the page is above it");
    m.press(Button::Down);
    m.press(Button::Down);
    assert_eq!(m.focused_row(), Some(first));
    // Down never rests on a header of an ordinary group.
    for _ in 0..m.entry_count(Page::Cpu) {
        m.press(Button::Down);
        assert!(!matches!(m.focus(), Some(Entry::Group(_)) | None));
    }
}

#[test]
fn core02_table_carriage_return() {
    // A grid page whose last row is short: moving right off the end of the row returns to its first
    // cell and skips the empty cells (the item grid docked: 38 of 40 cells).
    let layout = GridLayout::new(GridKind::Items, ConsoleMode::Docked);
    assert_eq!(layout.page_len(0), 38);
    assert_eq!(layout.step(0, 37, Move::Right), 35);
    assert_eq!(layout.step(0, 35, Move::Left), 37, "left from the first cell comes to the last filled one");
    // Down from the row above the short row, in an empty column, skips to the next filled cell below.
    assert_eq!(layout.step(0, 33, Move::Down), 3, "cell 38 is empty: the move goes on and wraps to the top");
    assert_eq!(layout.step(0, 3, Move::Up), 33, "up from the top wraps past the empty cell");
}

#[test]
fn core02_page_machine() {
    let mut m = menu();
    assert!(m.screen().is_none() && !m.is_closed());
    let r = goto(&mut m, "sdi_strength");
    // A opens the list popup, B closes it.
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::List { row, .. }) if *row == r));
    m.press(Button::B);
    assert!(m.screen().is_none() && !m.is_closed());
    // A on a direction grid opens it, and B backs out of it first; B on the page closes the menu.
    goto(&mut m, "di_state");
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::Directions { .. })));
    m.press(Button::B);
    assert!(m.screen().is_none());
    assert!(!m.is_closed());
    m.press(Button::B);
    assert!(m.is_closed());
    // Nested: Button chords -> a chord row -> the chord editor, and back out one at a time.
    let mut m = menu();
    goto(&mut m, "Button chords");
    m.press(Button::A);
    assert_eq!(m.depth(), 1);
    m.press(Button::A);
    assert_eq!(m.depth(), 2);
    assert!(matches!(m.screen(), Some(Screen::ChordEntry { .. })));
    m.press(Button::B);
    m.press(Button::B);
    assert_eq!(m.depth(), 0);
    // L / R go round the ring of pages.
    let mut m = menu();
    let mut seen = vec![m.page()];
    for _ in 0..5 {
        m.press(Button::R);
        seen.push(m.page());
    }
    assert_eq!(
        seen,
        [Page::Cpu, Page::Saves, Page::Recording, Page::FightersStages, Page::DisplayControls, Page::Cpu],
        "five pages; the sixth is hidden"
    );
    m.press(Button::L);
    assert_eq!(m.page(), Page::DisplayControls);
}

// ---- controls ------------------------------------------------------------------------------------

#[test]
fn core02_toggle_flips_on_a() {
    let mut m = menu();
    goto(&mut m, "hud");
    assert_eq!(m.value(row_of("hud")), RowValue::Toggle(true));
    m.press(Button::A);
    assert_eq!(get(&m, "hud"), [0, 1]);
    m.press(Button::A);
    assert_eq!(get(&m, "hud"), [1, 0]);
    // Left / Right do nothing on a toggle.
    m.press(Button::Right);
    assert_eq!(get(&m, "hud"), [1, 0]);
}

#[test]
fn core02_toggle_single_exclusive() {
    // A dropdown keeps exactly one option set, however it is changed.
    let mut m = menu();
    goto(&mut m, "sdi_strength");
    assert_eq!(get(&m, "sdi_strength"), [1, 0, 0, 0]);
    press(&mut m, &[Button::A, Button::Down, Button::Down, Button::A]);
    assert!(m.screen().is_none());
    assert_eq!(get(&m, "sdi_strength"), [0, 0, 1, 0]);
    m.press(Button::Right);
    assert_eq!(get(&m, "sdi_strength"), [0, 0, 0, 1]);
    // B leaves the list without a change.
    press(&mut m, &[Button::A, Button::Up, Button::B]);
    assert_eq!(get(&m, "sdi_strength"), [0, 0, 0, 1]);
}

#[test]
fn core02_toggle_wrap() {
    // Left / Right cycle a dropdown in place and wrap (Q2); the list shows the current option.
    let mut m = menu();
    goto(&mut m, "sdi_strength");
    m.press(Button::Left);
    assert_eq!(m.value(row_of("sdi_strength")), RowValue::Choice(Some(3)), "wraps from the first to the last");
    m.press(Button::Right);
    assert_eq!(m.value(row_of("sdi_strength")), RowValue::Choice(Some(0)), "and back");
    m.press(Button::Right);
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::List { cursor: 1, .. })), "the list opens on the current option");
    // A direction chip's weight wraps from the largest back to 0.
    let mut m = menu();
    goto(&mut m, "di_state");
    m.press(Button::A);
    for expected in [1, 2, 3, 4, 5, 0] {
        m.press(Button::A);
        assert_eq!(get(&m, "di_state")[8], expected);
    }
}

#[test]
fn core02_y_clears_weighted_only() {
    // Y clears a weighted chip; on a plain checklist and a list it does nothing.
    let mut m = menu();
    goto(&mut m, "di_state");
    press(&mut m, &[Button::A, Button::A, Button::A, Button::A]);
    assert_eq!(get(&m, "di_state")[8], 3);
    m.press(Button::Y);
    assert_eq!(get(&m, "di_state")[8], 0);
    let mut m = menu();
    goto(&mut m, "mash_state");
    press(&mut m, &[Button::A, Button::A, Button::Y]);
    assert_eq!(get(&m, "mash_state")[0], 1, "Y does not clear a plain checklist");
}

#[test]
fn core02_clear_hint_rule() {
    let mut m = menu();
    assert!(!m.clear_hint(), "not on the page");
    goto(&mut m, "mash_state");
    m.press(Button::A);
    assert!(!m.clear_hint(), "not on a plain checklist");
    m.press(Button::B);
    goto(&mut m, "di_state");
    m.press(Button::A);
    assert!(m.clear_hint());
    m.press(Button::B);
    goto(&mut m, "aerial_delay");
    m.press(Button::A);
    assert!(m.clear_hint());
}

#[test]
fn dropdown_yes_no_random_maps_to_the_weighted_row() {
    let mut m = menu();
    goto(&mut m, "falling_aerials");
    assert_eq!(get(&m, "falling_aerials"), [0, 1]);
    assert_eq!(m.value(row_of("falling_aerials")), RowValue::Choice(Some(1)), "No");
    m.press(Button::Right);
    assert_eq!(get(&m, "falling_aerials"), [1, 1], "Random is both set");
    m.press(Button::Right);
    assert_eq!(get(&m, "falling_aerials"), [1, 0], "Yes");
    m.press(Button::Left);
    m.press(Button::Left);
    assert_eq!(get(&m, "falling_aerials"), [0, 1]);
    // The list popup offers the same three.
    press(&mut m, &[Button::A, Button::Down]);
    m.press(Button::A);
    assert_eq!(get(&m, "falling_aerials"), [1, 1], "the list opened on No; one down is Random");
    // Full Hop defaults to Yes.
    assert_eq!(m.value(row_of("full_hop")), RowValue::Choice(Some(0)));
    // A weighted file value of both reads as Random; nothing set reads as no choice.
    set(&mut m, "fast_fall", &[3, 2]);
    assert_eq!(m.value(row_of("fast_fall")), RowValue::Choice(Some(2)));
    set(&mut m, "fast_fall", &[0, 0]);
    assert_eq!(m.value(row_of("fast_fall")), RowValue::Choice(None));
}

#[test]
fn number_row_steps_and_clamps() {
    let mut m = menu();
    goto(&mut m, "input_delay");
    assert_eq!(m.value(row_of("input_delay")), RowValue::Number(Some(0)));
    m.press(Button::Left);
    assert_eq!(m.value(row_of("input_delay")), RowValue::Number(Some(0)), "stops at the lowest");
    for _ in 0..40 {
        m.press(Button::Right);
    }
    assert_eq!(m.value(row_of("input_delay")), RowValue::Number(Some(30)), "stops at the highest");
    assert_eq!(m.settings().menu.input_delay(), 30);
    // A opens the list on the current value; picking sets it.
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::List { cursor: 30, .. })));
    for _ in 0..25 {
        m.press(Button::Up);
    }
    m.press(Button::A);
    assert_eq!(m.settings().menu.input_delay(), 5);
    // Ready Fighters runs 2..=12 and starts at 10 (P-HOT).
    goto(&mut m, "cycle_preload_ready_fighters");
    assert_eq!(m.value(row_of("cycle_preload_ready_fighters")), RowValue::Number(Some(10)));
    for _ in 0..10 {
        m.press(Button::Right);
    }
    assert_eq!(m.value(row_of("cycle_preload_ready_fighters")), RowValue::Number(Some(12)));
}

#[test]
fn multi_select_ticks_and_all_none() {
    let mut m = menu();
    goto(&mut m, "mash_state");
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::Checklist { cursor: 0, .. })));
    press(&mut m, &[Button::A, Button::Down, Button::Down, Button::A]);
    let row = get(&m, "mash_state");
    assert_eq!((row[0], row[1], row[2]), (1, 0, 1));
    assert_eq!(m.value(row_of("mash_state")), RowValue::Checklist { ticked: 2, total: 30 });
    m.press(Button::A);
    assert_eq!(get(&m, "mash_state")[2], 0, "A on a ticked entry clears it");
    m.press(Button::ZL);
    assert!(get(&m, "mash_state").iter().all(|v| *v == 1), "ZL ticks all");
    m.press(Button::ZL);
    assert!(get(&m, "mash_state").iter().all(|v| *v == 0), "ZL again clears all");
    // The cursor wraps in the popup (it is on entry 2).
    press(&mut m, &[Button::Up, Button::Up, Button::Up]);
    assert!(matches!(m.screen(), Some(Screen::Checklist { cursor: 29, .. })));
}

#[test]
fn situation_overrides_editor_picks_a_situation_then_actions() {
    let mut m = menu();
    let r = goto(&mut m, "Situation overrides");
    assert_eq!(m.value(r), RowValue::OverridesSet(0));
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::Overrides { situation: 0, cursor: 0 })));
    // Right on the situation picker moves to Ledge Roll; the actions then belong to it.
    m.press(Button::Right);
    press(&mut m, &[Button::Down, Button::A]);
    assert_eq!(get(&m, "ledge_roll_override")[0], 1);
    assert_eq!(get(&m, "ledge_neutral_override")[0], 0);
    m.press(Button::B);
    assert_eq!(m.value(r), RowValue::OverridesSet(1));
    // Left from the first situation wraps to Ledge Trump.
    m.press(Button::A);
    m.press(Button::Left);
    assert!(matches!(m.screen(), Some(Screen::Overrides { situation: 12, .. })));
}

// ---- sliders (damage ranges) ---------------------------------------------------------------------

#[test]
fn damage_range_is_greyed_unless_random() {
    let mut m = menu();
    let range = goto(&mut m, "save_damage_limits_player");
    assert_eq!(m.grey(range), Some(Grey::DamageNotRandom));
    m.press(Button::A);
    assert!(m.screen().is_none(), "A on a greyed row does nothing");
    set(&mut m, "save_damage_player", &[0, 0, 1]);
    assert_eq!(m.grey(range), None);
    // The external reload has no saved page, so Q5 now opens CPU. Navigate
    // back to the range before exercising its unchanged activation assertion.
    goto(&mut m, "save_damage_limits_player");
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::Slider { state: SliderState::LowerHover, .. })));
    // The CPU's range follows the CPU's own dropdown, not the player's.
    assert_eq!(m.grey(row_of("save_damage_limits_cpu")), Some(Grey::DamageNotRandom));
}

#[test]
fn core02_slider_states() {
    let mut m = menu();
    set(&mut m, "save_damage_cpu", &[0, 0, 1]);
    goto(&mut m, "save_damage_limits_cpu");
    m.press(Button::A);
    let state = |m: &Menu| match m.screen() {
        Some(Screen::Slider { state, .. }) => *state,
        other => panic!("{other:?}"),
    };
    assert_eq!(state(&m), SliderState::LowerHover);
    m.press(Button::Right);
    assert_eq!(state(&m), SliderState::UpperHover, "Left / Right switch the hovered handle");
    m.press(Button::Left);
    assert_eq!(state(&m), SliderState::LowerHover);
    m.press(Button::A);
    assert_eq!(state(&m), SliderState::LowerSelected);
    m.press(Button::A);
    assert_eq!(state(&m), SliderState::LowerHover, "A again deselects");
    m.press(Button::A);
    m.press(Button::B);
    assert_eq!(state(&m), SliderState::LowerHover, "B deselects first");
    m.press(Button::B);
    assert!(m.screen().is_none(), "and then leaves the page");
}

#[test]
fn core02_slider_clamp_steps() {
    let mut m = menu();
    set(&mut m, "save_damage_player", &[0, 0, 1]);
    goto(&mut m, "save_damage_limits_player");
    m.press(Button::A);
    m.press(Button::A); // lower selected
    // Raising the lower handle: +1 slow, +10 fast, never past the upper handle.
    press(&mut m, &[Button::Right]);
    assert_eq!(get(&m, "save_damage_limits_player"), [1, 150]);
    press(&mut m, &[Button::Up]);
    assert_eq!(get(&m, "save_damage_limits_player"), [11, 150]);
    press(&mut m, &[Button::Down, Button::Down]);
    assert_eq!(get(&m, "save_damage_limits_player"), [0, 150], "clamped at the minimum");
    // Lower the upper handle to 5, then try to push the lower one past it.
    press(&mut m, &[Button::A, Button::Right, Button::A]);
    for _ in 0..15 {
        m.press(Button::Down);
    }
    assert_eq!(get(&m, "save_damage_limits_player"), [0, 0], "the upper handle stops at the lower one");
    for _ in 0..16 {
        m.press(Button::Up);
    }
    assert_eq!(get(&m, "save_damage_limits_player"), [0, 150], "and at the maximum");
    m.press(Button::Left);
    assert_eq!(get(&m, "save_damage_limits_player"), [0, 149]);
    // Lower handle again: it cannot pass the upper one.
    press(&mut m, &[Button::A, Button::Left, Button::A]);
    for _ in 0..20 {
        m.press(Button::Up);
    }
    assert_eq!(get(&m, "save_damage_limits_player"), [149, 149]);
}

// ---- greyed rows ---------------------------------------------------------------------------------

#[test]
fn respawn_invincibility_is_greyed_while_auto_load_is_on() {
    let mut m = menu();
    let respawn = goto(&mut m, "respawn_invincibility");
    assert_eq!(m.grey(respawn), None);
    m.press(Button::A);
    assert_eq!(get(&m, "respawn_invincibility"), [1, 0]);
    goto(&mut m, "save_state_autoload");
    m.press(Button::A);
    assert_eq!(get(&m, "save_state_autoload"), [1, 0]);
    assert_eq!(m.grey(respawn), Some(Grey::AutoLoadOn));
    assert!(Grey::AutoLoadOn.text().contains("Auto-load"));
    goto(&mut m, "respawn_invincibility");
    m.press(Button::A);
    assert_eq!(get(&m, "respawn_invincibility"), [1, 0], "A on a greyed row changes nothing");
    // The stored value is kept; turning Auto-load off frees the row again.
    goto(&mut m, "save_state_autoload");
    m.press(Button::A);
    assert_eq!(m.grey(respawn), None);
}

#[test]
fn swap_group_entries_grey_with_the_reason() {
    let mut m = menu();
    let r = goto(&mut m, "Swap groups");
    assert_eq!(m.value(r), RowValue::Checklist { ticked: 7, total: 7 });
    for entry in 0..7 {
        assert_eq!(m.swap_entry_grey(entry), None, "factory: everything on, three bodies");
    }
    // Group Swap off: every group entry greys (the banks do not).
    goto(&mut m, "cycle_preload_group_swap");
    m.press(Button::A);
    assert_eq!(m.swap_entry_grey(0), None);
    assert_eq!(m.swap_entry_grey(1), None);
    for entry in 2..=6 {
        assert_eq!(m.swap_entry_grey(entry), Some(Grey::GroupSwapOff), "entry {entry}");
    }
    m.press(Button::A);
    // Pair bank off: the two pair-group entries grey; fewer than three bodies greys the pair bank.
    goto(&mut m, "Swap groups");
    m.press(Button::A);
    m.press(Button::A); // entry 0, the pair bank, off
    assert_eq!(get(&m, "cycle_preload_ic_pair_bank"), [0, 1]);
    assert_eq!(m.swap_entry_grey(3), Some(Grey::PairBankOff));
    assert_eq!(m.swap_entry_grey(5), Some(Grey::PairBankOff));
    assert_eq!(m.swap_entry_grey(2), None, "Solo Popo groups do not need the pair bank");
    // A on a greyed entry changes nothing.
    press(&mut m, &[Button::Down, Button::Down, Button::Down, Button::A]);
    assert_eq!(get(&m, "cycle_preload_ic_pair_groups"), [1, 0]);
    m.press(Button::B);
    set(&mut m, "cycle_preload_bank_members", &[0, 1, 0]);
    assert_eq!(m.swap_entry_grey(0), Some(Grey::NeedsThreeBodies));
}

// ---- direction grids -----------------------------------------------------------------------------

#[test]
fn direction_grid_navigation_follows_the_chip_layout() {
    // In . Neutral . Out across, Up . - . Down down, Left / Right in a column of their own.
    let mut m = menu();
    goto(&mut m, "di_state");
    m.press(Button::A);
    let chip = |m: &Menu| match m.screen() {
        Some(Screen::Directions { chip, .. }) => *chip,
        other => panic!("{other:?}"),
    };
    assert_eq!(chip(&m), 8, "starts on Neutral");
    m.press(Button::Up);
    assert_eq!(chip(&m), 2, "Up");
    m.press(Button::Left);
    assert_eq!(chip(&m), 3, "Up In");
    m.press(Button::Down);
    assert_eq!(chip(&m), 4, "In");
    m.press(Button::Left);
    assert_eq!(chip(&m), 10, "Left wraps to the absolute column (Right chip is on the same row)");
    m.press(Button::Right);
    assert_eq!(chip(&m), 4);
    m.press(Button::Right);
    m.press(Button::Right);
    m.press(Button::Right);
    assert_eq!(chip(&m), 10, "Out -> Right (absolute)");
    m.press(Button::Up);
    assert_eq!(chip(&m), 9, "Left (absolute)");
    // Every chip is reachable and the column of two skips its empty cell.
    m.press(Button::Down);
    m.press(Button::Down);
    assert_eq!(chip(&m), 9, "down from Right skips the empty cell and wraps to Left");
}

#[test]
fn direction_grid_weights_all_none_and_preview() {
    let mut m = menu();
    let r = goto(&mut m, "sdi_state");
    assert_eq!(m.value(r), RowValue::Chips(0));
    m.press(Button::A);
    m.press(Button::A); // Neutral +1
    m.press(Button::Up);
    press(&mut m, &[Button::A, Button::A]); // Up +2
    assert_eq!(get(&m, "sdi_state"), [0, 0, 2, 0, 0, 0, 0, 0, 1, 0, 0]);
    assert_eq!(m.value(r), RowValue::Chips((1 << 2) | (1 << 8)));
    m.press(Button::ZL);
    assert!(get(&m, "sdi_state").iter().all(|v| *v >= 1), "ZL ticks every chip, keeping weights");
    assert_eq!(get(&m, "sdi_state")[2], 2);
    m.press(Button::ZL);
    assert!(get(&m, "sdi_state").iter().all(|v| *v == 0), "ZL again clears them all");
}

#[test]
fn shield_tilt_grid_holds_exactly_one_chip() {
    let mut m = menu();
    goto(&mut m, "shield_tilt");
    assert_eq!(get(&m, "shield_tilt"), [0; 11]);
    m.press(Button::A);
    press(&mut m, &[Button::A]);
    assert_eq!(get(&m, "shield_tilt")[8], 1, "A selects the chip under the cursor");
    press(&mut m, &[Button::Up, Button::A]);
    assert_eq!(get(&m, "shield_tilt").iter().filter(|v| **v != 0).count(), 1, "one chip only");
    assert_eq!(get(&m, "shield_tilt")[2], 1);
    m.press(Button::A);
    assert_eq!(get(&m, "shield_tilt"), [0; 11], "A on the selected chip clears it");
    press(&mut m, &[Button::A, Button::Y]);
    assert_eq!(get(&m, "shield_tilt"), [0; 11]);
    press(&mut m, &[Button::A, Button::ZL]);
    assert_eq!(get(&m, "shield_tilt"), [0; 11], "ZL is none for a single-chip grid");
}

// ---- N3: delay rows (checklist or range) ---------------------------------------------------------

const DELAY_IDS: [&str; 7] =
    ["aerial_delay", "fast_fall_delay", "throw_delay", "pummel_delay", "ledge_delay", "oos_offset", "reaction_time"];

#[test]
fn core02_delay_row_exclusion() {
    for id in DELAY_IDS {
        let mut m = menu();
        let r = goto(&mut m, id);
        let range_id = format!("{id}_range");
        m.press(Button::A);
        assert!(matches!(m.screen(), Some(Screen::Delay { .. })), "{id}");
        assert_eq!(m.delay_grey(r), (None, None), "{id}: both empty, both usable");
        // Tick an entry: the range greys, and A on a range handle does nothing.
        m.press(Button::A);
        assert_eq!(m.delay_grey(r), (None, Some(Grey::ChecklistSet)), "{id}");
        m.press(Button::Up);
        m.press(Button::Up);
        m.press(Button::A);
        assert!(get(&m, &range_id).is_empty(), "{id}: a greyed handle ignores A");
        assert!(matches!(m.screen(), Some(Screen::Delay { editing: false, .. })));
        // Y clears the checklist; the range is usable; setting it greys the checklist.
        m.press(Button::Y);
        assert_eq!(m.delay_grey(r), (None, None));
        m.press(Button::A);
        assert_eq!(get(&m, &range_id), [0, 0], "{id}: A on a handle sets a range");
        assert_eq!(m.delay_grey(r), (Some(Grey::RangeSet), None));
        press(&mut m, &[Button::B]);
        // Cursor back to the first entry: a tick is ignored while the range is set.
        press(&mut m, &[Button::Down, Button::Down]);
        m.press(Button::A);
        assert!(get(&m, id).iter().all(|v| *v == 0), "{id}: a greyed checklist ignores A");
        // ZL (all) is ignored on the greyed checklist too.
        m.press(Button::ZL);
        assert!(get(&m, id).iter().all(|v| *v == 0));
    }
}

#[test]
fn core02_delay_row_y_clear() {
    let mut m = menu();
    let r = goto(&mut m, "aerial_delay");
    m.press(Button::A);
    press(&mut m, &[Button::A, Button::Down, Button::A]);
    assert_eq!(m.value(r), RowValue::Delay(DelayView::Ticked(2)));
    m.press(Button::Y);
    assert_eq!(m.value(r), RowValue::Delay(DelayView::Empty), "Y clears the checklist");
    // Move to the Min handle (cursor 31), select it, and raise Max; Y removes the range.
    for _ in 0..31 {
        m.press(Button::Up);
    }
    // From cursor 1 going up 31 times: 1 -> 0 -> 32 -> ... lands on 2 (33 positions); use the screen.
    let Some(Screen::Delay { cursor, .. }) = m.screen().copied() else { panic!() };
    assert_eq!(cursor, (1 + 33 * 31 - 31) % 33);
    goto_cursor(&mut m, 31);
    m.press(Button::A);
    press(&mut m, &[Button::Up, Button::Up]);
    assert_eq!(get(&m, "aerial_delay_range"), [0, 0], "Min is held at Max");
    m.press(Button::B);
    goto_cursor(&mut m, 32);
    m.press(Button::A);
    m.press(Button::Up);
    assert_eq!(get(&m, "aerial_delay_range"), [0, 10]);
    m.press(Button::B);
    assert_eq!(m.value(r), RowValue::Delay(DelayView::Range(DelayRange { min: 0, max: 10, step: 1 })));
    m.press(Button::Y);
    assert!(get(&m, "aerial_delay_range").is_empty(), "Y removes the active range");
    assert_eq!(m.value(r), RowValue::Delay(DelayView::Empty));
    assert!(!m.serialized().contains("aerial_delay_range"), "an empty range is absent from the file");
}

fn goto_cursor(m: &mut Menu, target: usize) {
    for _ in 0..40 {
        match m.screen() {
            Some(Screen::Delay { cursor, .. }) if *cursor == target => return,
            Some(Screen::Delay { .. }) => m.press(Button::Down),
            other => panic!("{other:?}"),
        }
    }
    panic!("cursor {target} not reached");
}

#[test]
fn core02_delay_range_bounds() {
    // Min <= max, both inside the row's value set, at the row's own step (1 / 5 / 10).
    for (id, first_step, values) in [("aerial_delay", 1u16, 31u16), ("throw_delay", 5, 31), ("ledge_delay", 10, 31)] {
        let mut m = menu();
        let r = goto(&mut m, id);
        m.press(Button::A);
        goto_cursor(&mut m, 32);
        m.press(Button::A);
        // Push Max to the top of the set, then past it: it stops at the last value.
        for _ in 0..5 {
            m.press(Button::Up);
        }
        let range = get(&m, &format!("{id}_range"));
        assert_eq!(range, [0, 30], "{id}: Max stops at the last option");
        assert_eq!(
            m.value(r),
            RowValue::Delay(DelayView::Range(DelayRange { min: 0, max: first_step * (values - 1), step: first_step })),
            "{id}: the range reads in the row's units"
        );
        // Min cannot pass Max, Max cannot pass Min.
        m.press(Button::B);
        goto_cursor(&mut m, 31);
        m.press(Button::A);
        for _ in 0..5 {
            m.press(Button::Up);
        }
        assert_eq!(get(&m, &format!("{id}_range")), [30, 30], "{id}: Min stops at Max");
        for _ in 0..5 {
            m.press(Button::Down);
        }
        assert_eq!(get(&m, &format!("{id}_range")), [0, 30]);
        // The slow step is one option, and Left at the lowest stays.
        m.press(Button::Right);
        m.press(Button::Right);
        m.press(Button::Left);
        assert_eq!(get(&m, &format!("{id}_range")), [1, 30]);
        m.press(Button::B);
        goto_cursor(&mut m, 32);
        m.press(Button::A);
        for _ in 0..40 {
            m.press(Button::Left);
        }
        assert_eq!(get(&m, &format!("{id}_range")), [1, 1], "Max stops at Min");
    }
    // The range table row refuses a reversed or oversized pair.
    let mut s = Settings::factory();
    assert!(s.menu.set("aerial_delay_range", &[5, 3]).is_err());
    assert!(s.menu.set("aerial_delay_range", &[0, 31]).is_err());
    assert!(s.menu.set("aerial_delay_range", &[3, 5]).is_ok());
}

#[test]
fn core02_delay_range_roundtrip() {
    let mut m = menu();
    set(&mut m, "ledge_delay_range", &[2, 9]);
    let r = row_of("ledge_delay");
    assert_eq!(m.value(r), RowValue::Delay(DelayView::Range(DelayRange { min: 20, max: 90, step: 10 })));
    let text = m.serialized();
    let loaded = settings::parse(&text);
    assert!(!loaded.corrupt(), "{:?}", loaded.issues);
    assert_eq!(loaded.settings.menu.get("ledge_delay_range"), Some(&[2u8, 9][..]));
    let reopened = Menu::new(loaded.settings, Env::default());
    assert_eq!(reopened.value(r), m.value(r));
    assert_eq!(reopened.delay_spec("ledge_delay"), Some(DelaySpec::Range(DelayRange { min: 20, max: 90, step: 10 })));
}

#[test]
fn core02_delay_no_range_loads_checklist() {
    // A file with no range field loads every delay row as a checklist.
    let loaded = settings::parse(r#"{"menu": {"throw_delay": [0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}}"#);
    assert!(!loaded.corrupt());
    let m = Menu::new(loaded.settings, Env::default());
    assert_eq!(m.value(row_of("throw_delay")), RowValue::Delay(DelayView::Ticked(1)));
    match m.delay_spec("throw_delay") {
        Some(DelaySpec::Set { weights, first: 0, step: 5 }) => assert_eq!(weights[1], 1),
        other => panic!("{other:?}"),
    }
    for id in DELAY_IDS {
        let m = menu();
        assert_eq!(m.delay_spec(id), Some(DelaySpec::Empty), "{id}: empty is the empty row");
    }
    // A hand-edited file with both a checklist and a range keeps the checklist.
    let mut s = Settings::factory();
    s.menu.set("aerial_delay", &[1]).unwrap();
    s.menu.set("aerial_delay_range", &[1, 4]).unwrap();
    let m = Menu::new(s, Env::default());
    assert!(m.settings().menu.get("aerial_delay_range").unwrap().is_empty());
    assert_eq!(m.value(row_of("aerial_delay")), RowValue::Delay(DelayView::Ticked(1)));
}

#[test]
fn delay_range_pick_is_uniform_over_the_step_values() {
    let range = DelayRange { min: 10, max: 40, step: 10 };
    assert_eq!(range.count(), 4);
    let mut hits = std::collections::BTreeMap::new();
    for random in 0..400u32 {
        *hits.entry(range.pick(random)).or_insert(0u32) += 1;
    }
    assert_eq!(hits.keys().copied().collect::<Vec<_>>(), [10, 20, 30, 40]);
    assert!(hits.values().all(|n| *n == 100), "{hits:?}");
    // min = max is a fixed delay.
    assert_eq!(DelayRange { min: 7, max: 7, step: 1 }.pick(12345), 7);
}

#[test]
fn core02_slot_sets_have_no_range() {
    for id in ["randomize_slots", "playback_button_slots", "save_state_playback"] {
        let r = row_of(id);
        assert!(matches!(ROWS[r].control, Control::MultiSelect(_)), "{id} is a plain checklist");
        assert!(settings::def(&format!("{id}_range")).is_none(), "{id} has no range row");
        assert!(menu().delay_spec(id).is_none(), "{id} is not a delay row");
    }
    // Exactly the seven delay rows carry a range.
    let ranged: Vec<&str> = ROWS.iter().filter(|r| matches!(r.control, Control::MultiSelectRange { .. })).map(|r| r.id).collect();
    assert_eq!(ranged, ["ledge_delay", "oos_offset", "reaction_time", "throw_delay", "pummel_delay", "aerial_delay", "fast_fall_delay"]);
}

#[test]
fn core03_delay_popup_render_plan() {
    // The popup shows both controls; the inactive one is greyed, with the reason, and ignores A.
    let mut m = menu();
    let r = goto(&mut m, "oos_offset");
    m.press(Button::A);
    let (checklist, range) = m.delay_grey(r);
    assert!(checklist.is_none() && range.is_none());
    m.press(Button::A);
    let (_, range) = m.delay_grey(r);
    assert_eq!(range.map(Grey::text), Some(Grey::ChecklistSet.text()));
    assert!(m.help_row().is_some_and(|row| row.title == "OoS Offset"));
}

#[test]
fn core02_delay_row_reset_and_save_defaults() {
    // Reset and save treat the checklist and the range as one row: both are restored together.
    let mut m = menu();
    set(&mut m, "aerial_delay_range", &[3, 8]);
    m.press(Button::X);
    // Change the row to a checklist (clear the range first), then reset the page: back to the range.
    let mut s = m.settings().clone();
    s.menu.set("aerial_delay_range", &[]).unwrap();
    s.menu.set("aerial_delay", &[0, 1]).unwrap();
    let env = *m.env();
    m.reopen(s, env);
    goto(&mut m, "mash_state");
    reset_page(&mut m);
    assert_eq!(get(&m, "aerial_delay_range"), [3, 8]);
    assert!(get(&m, "aerial_delay").iter().all(|v| *v == 0), "the checklist is restored with it");
    // Factory defaults: the range is empty after a reset.
    let mut m = menu();
    set(&mut m, "pummel_delay_range", &[0, 2]);
    goto(&mut m, "mash_state");
    reset_page(&mut m);
    assert!(get(&m, "pummel_delay_range").is_empty());
}

#[test]
fn core02_delay_row_bulk_update_clears_range() {
    let mut m = menu();
    set(&mut m, "ledge_delay_range", &[1, 3]);
    // Updating the checklist clears the range first.
    m.bulk_update(&[("ledge_delay", &[0, 0, 1])]);
    assert!(get(&m, "ledge_delay_range").is_empty());
    assert_eq!(&get(&m, "ledge_delay")[..4], [0, 0, 1, 0]);
    // Updating the range clears the checklist first.
    m.bulk_update(&[("ledge_delay_range", &[0, 4])]);
    assert!(get(&m, "ledge_delay").iter().all(|v| *v == 0));
    assert_eq!(get(&m, "ledge_delay_range"), [0, 4]);
}

// ---- defaults, reset, serialization --------------------------------------------------------------

#[test]
fn core02_defaults_and_reset() {
    let mut m = menu();
    // Change several rows of the CPU page, then the confirmed reset line returns the page to the defaults.
    goto(&mut m, "sdi_strength");
    m.press(Button::Right);
    goto(&mut m, "stale_dodges");
    m.press(Button::A);
    goto(&mut m, "mash_state");
    press(&mut m, &[Button::A, Button::A, Button::B]);
    goto(&mut m, "di_state");
    press(&mut m, &[Button::A, Button::A, Button::B]);
    // A row of another page stays.
    m.press(Button::R);
    goto(&mut m, "hud");
    m.press(Button::A);
    m.press(Button::L);
    m.press(Button::L);
    m.press(Button::L);
    m.press(Button::L);
    assert_eq!(m.page(), Page::Cpu);
    reset_page(&mut m);
    let factory = Settings::factory();
    for id in ["sdi_strength", "stale_dodges", "mash_state", "di_state"] {
        assert_eq!(m.settings().menu.get(id), factory.menu.get(id), "{id}");
    }
    assert_eq!(get(&m, "hud"), [0, 1], "other pages are untouched");
    // Saved defaults replace the factory ones for the next reset.
    goto(&mut m, "stale_dodges");
    m.press(Button::A);
    m.press(Button::X);
    m.press(Button::A);
    assert_eq!(get(&m, "stale_dodges"), [1, 0]);
    reset_page(&mut m);
    assert_eq!(get(&m, "stale_dodges"), [0, 1], "reset restores the saved default");
}

#[test]
fn core02_serialize_roundtrip() {
    let mut m = menu();
    goto(&mut m, "sdi_state");
    press(&mut m, &[Button::A, Button::A, Button::B]);
    goto(&mut m, "aerial_delay");
    press(&mut m, &[Button::A, Button::A, Button::B]);
    goto(&mut m, "Replay capture");
    let text = m.serialized();
    let loaded = settings::parse(&text);
    assert!(!loaded.corrupt(), "{:?}", loaded.issues);
    // Every settings array, sliders as [lower, upper], survives the round trip.
    assert_eq!(loaded.settings.menu, m.settings().menu);
    assert_eq!(loaded.settings.defaults, m.settings().defaults);
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["menu"]["save_damage_limits_cpu"], serde_json::json!([0, 150]));
    assert_eq!(value["menu"]["sdi_state"][8], 1);
    assert_eq!(value["last_page"], "recording", "the page the menu was left on");
    // The two snapshots are independent.
    assert_eq!(value["defaults_menu"]["sdi_state"][8], 0);
}

#[test]
fn core02_update_from_shorter_array() {
    // The fork's fix: a shorter array leaves the appended options off, not stale.
    let mut m = menu();
    set(&mut m, "mash_state", &[1; 30]);
    m.bulk_update(&[("mash_state", &[0, 1, 1])]);
    let row = get(&m, "mash_state");
    assert_eq!(row.len(), 30);
    assert_eq!(&row[..4], [0, 1, 1, 0]);
    assert!(row[3..].iter().all(|v| *v == 0));
    // A longer array loses its extra elements; a missing id is unchanged; an unknown id is ignored.
    m.bulk_update(&[("sdi_strength", &[0, 1, 0, 0, 1, 1]), ("not_a_row", &[1])]);
    assert_eq!(get(&m, "sdi_strength"), [0, 1, 0, 0]);
    assert_eq!(&get(&m, "mash_state")[..3], [0, 1, 1]);
    // A slider array must have exactly two elements; a bad one is refused and the row stays.
    m.bulk_update(&[("save_damage_limits_cpu", &[5])]);
    assert_eq!(get(&m, "save_damage_limits_cpu"), [0, 150]);
    m.bulk_update(&[("save_damage_limits_cpu", &[5, 20])]);
    assert_eq!(get(&m, "save_damage_limits_cpu"), [5, 20]);
}

#[test]
fn core02_hold_repeat_period() {
    assert_eq!(HOLD_REPEAT_PERIOD, 21);
    let mut hold = HoldRepeat::default();
    let mut at = Vec::new();
    for frame in 1..=70u32 {
        if hold.frame(true) {
            at.push(frame);
        }
    }
    assert_eq!(at, [21, 42, 63], "a held stick repeats every 21 frames");
    // Releasing restarts the count; an accepted input resets it too.
    assert!(!hold.frame(false));
    for _ in 0..20 {
        assert!(!hold.frame(true));
    }
    hold.reset();
    for _ in 0..20 {
        assert!(!hold.frame(true));
    }
    assert!(hold.frame(true));
}

// ---- N1: the Developer page ----------------------------------------------------------------------

#[test]
fn core02_hidden_page_ring() {
    let off = menu();
    assert_eq!(off.ring().count(), 5);
    assert!(!off.ring().any(|p| p == Page::Developer));
    let mut m = menu();
    for _ in 0..8 {
        m.press(Button::R);
        assert_ne!(m.page(), Page::Developer, "R skips the hidden page");
    }
    for _ in 0..8 {
        m.press(Button::L);
        assert_ne!(m.page(), Page::Developer, "and so does L");
    }
    let on = menu_with(Env { developer_page: true, ..Env::default() });
    assert_eq!(on.ring().count(), 6);
    let mut m = menu_with(Env { developer_page: true, ..Env::default() });
    for _ in 0..4 {
        m.press(Button::R);
    }
    assert_eq!(m.page(), Page::DisplayControls);
    m.press(Button::R);
    assert_eq!(m.page(), Page::Developer, "the Developer page follows Display & Controls");
    assert_eq!(m.entry_count(Page::Developer), 1 + 3 + 1, "a header, three rows and the reset line");
    m.press(Button::R);
    assert_eq!(m.page(), Page::Cpu);
}

#[test]
fn core02_hidden_page_rows_serialized() {
    // With the flag off the rows exist, are stored and survive a round trip; the consumers read
    // `stored AND enabled`, so they are inert.
    let mut s = Settings::factory();
    for id in ["cycle_preload_measurements", "dev_reload_config", "dev_debug_hooks"] {
        s.menu.set(id, &[1, 0]).unwrap();
    }
    let hidden = Menu::new(s.clone(), Env { developer_page: false, ..Env::default() });
    let text = hidden.settings().to_json();
    for id in ["cycle_preload_measurements", "dev_reload_config", "dev_debug_hooks"] {
        assert!(text.contains(id), "{id} is serialized");
        assert!(!hidden.developer_effective(id), "{id} is inert while the page is hidden");
    }
    assert_eq!(settings::parse(&text).settings.menu, s.menu);
    let shown = Menu::new(s, Env { developer_page: true, ..Env::default() });
    for id in ["cycle_preload_measurements", "dev_reload_config", "dev_debug_hooks"] {
        assert!(shown.developer_effective(id));
    }
    // Bulk update and reset reach the hidden rows like any row.
    let mut m = menu();
    m.bulk_update(&[("dev_debug_hooks", &[1, 0])]);
    assert_eq!(get(&m, "dev_debug_hooks"), [1, 0]);
    // A row of the page is not reachable from the other pages.
    assert!(!m.ring().any(|p| p == Page::Developer));
}

// ---- Q5: the page the menu opens on --------------------------------------------------------------

#[test]
fn core02_first_open_is_cpu() {
    let m = menu();
    assert_eq!(m.page(), Page::Cpu);
    assert!(m.settings().last_page().is_none());
    assert_eq!(ROWS[m.focused_row().unwrap()].page(), Page::Cpu);
}

#[test]
fn core02_open_on_last_page() {
    let mut m = menu();
    m.press(Button::R);
    m.press(Button::R);
    assert_eq!(m.page(), Page::Recording);
    let settings = m.commit().clone();
    assert_eq!(settings.last_page(), Some("recording"));
    // Reopening, here or after a restart (through the file), lands on the same page.
    let mut m2 = Menu::new(settings.clone(), Env::default());
    assert_eq!(m2.page(), Page::Recording);
    assert_eq!(ROWS[m2.focused_row().unwrap()].page(), Page::Recording);
    let text = m2.serialized();
    let restarted = Menu::new(settings::parse(&text).settings, Env::default());
    assert_eq!(restarted.page(), Page::Recording);
    // Reopen the same instance too (the cursor of the page is kept).
    let mut m3 = menu();
    for _ in 0..3 {
        m3.press(Button::R);
    }
    m3.press(Button::Down);
    let cursor = m3.cursor();
    let s = m3.commit().clone();
    m3.press(Button::B);
    m3.reopen(s, Env::default());
    assert_eq!(m3.page(), Page::FightersStages);
    assert_eq!(m3.cursor(), cursor);
    assert!(!m3.is_closed() && m3.screen().is_none());
}

#[test]
fn core02_last_page_missing_falls_back() {
    // A saved Developer page with the flag off, an unknown id, a non-string value: CPU.
    for saved in [serde_json::json!("developer"), serde_json::json!("nowhere"), serde_json::json!(7), serde_json::json!(null)] {
        let mut s = Settings::factory();
        s.extra.insert("last_page".to_owned(), saved.clone());
        let m = Menu::new(s, Env::default());
        assert_eq!(m.page(), Page::Cpu, "{saved}");
    }
    // The same saved page opens when the flag is on.
    let mut s = Settings::factory();
    s.set_last_page("developer");
    assert_eq!(Menu::new(s.clone(), Env { developer_page: true, ..Env::default() }).page(), Page::Developer);
    assert_eq!(Menu::new(s, Env::default()).page(), Page::Cpu);
}

#[test]
fn q5_reopen_uses_loaded_page_or_cpu_not_the_previous_visible_page() {
    let invalid = [None, Some(serde_json::json!("developer")), Some(serde_json::json!("nowhere")),
        Some(serde_json::json!(7)), Some(serde_json::json!(null))];
    for previous in Page::ALL {
        for saved in &invalid {
            let mut original = Settings::factory();
            original.set_last_page(previous.id());
            let mut m = Menu::new(original, Env { developer_page: true, ..Env::default() });
            assert_eq!(m.page(), previous);
            let mut incoming = Settings::factory();
            incoming.menu.set("stale_dodges", &[0, 1]).unwrap();
            incoming.defaults.set("stale_dodges", &[0, 1]).unwrap();
            incoming.extra.insert("foreign_config".into(), serde_json::json!({"kept": [4, 2]}));
            if let Some(value) = saved { incoming.extra.insert("last_page".into(), value.clone()); }
            let loaded = settings::parse(&incoming.to_json());
            assert!(!loaded.corrupt());
            let expected = loaded.settings.to_json();
            m.reopen(loaded.settings, Env::default());
            assert_eq!(m.page(), Page::Cpu, "previous={previous:?}, saved={saved:?}");
            assert_eq!(m.settings().to_json(), expected, "fallback is not a settings rewrite");
            assert_eq!(m.take_outbox(), Default::default(), "loading emits no user effect");
        }
    }
    // Every visible saved page wins over a different old page; Developer needs the flag.
    for developer_page in [false, true] {
        for page in Page::ALL {
            let mut m = menu();
            m.press(Button::R);
            let mut incoming = Settings::factory();
            incoming.set_last_page(page.id());
            m.reopen(incoming, Env { developer_page, ..Env::default() });
            let expected = if page == Page::Developer && !developer_page { Page::Cpu } else { page };
            assert_eq!(m.page(), expected);
        }
    }
}

#[test]
fn q5_saved_developer_page_restores_in_a_fresh_process_then_continues_on_cpu() {
    use std::{fs, process::Command, time::{SystemTime, UNIX_EPOCH}};
    const MODE: &str = "TR_UI_Q5_PROCESS_MODE";
    const ROOT: &str = "TR_UI_Q5_PROCESS_ROOT";
    const NAME: &str = "q5_saved_developer_page_restores_in_a_fresh_process_then_continues_on_cpu";
    let now = || SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    if let Ok(mode) = std::env::var(MODE) {
        let root = std::path::PathBuf::from(std::env::var_os(ROOT).unwrap());
        let path = root.join("settings.json");
        match mode.as_str() {
            "save" => {
                let mut s = Settings::factory();
                s.menu.set("stale_dodges", &[0, 1]).unwrap();
                s.defaults.set("stale_dodges", &[0, 1]).unwrap();
                s.extra.insert("foreign_config".into(), serde_json::json!({"kept": [4, 2]}));
                let mut m = Menu::new(s, Env { developer_page: true, ..Env::default() });
                for _ in 0..5 { m.press(Button::R); }
                assert_eq!(m.page(), Page::Developer);
                m.press(Button::B);
                assert!(m.is_closed());
                fs::write(&path, m.serialized()).unwrap();
                assert_eq!(settings::parse(&fs::read_to_string(&path).unwrap()).settings.last_page(), Some("developer"));
                fs::write(root.join("saver.json"), format!("{{\"pid\":{},\"saved_after_close\":true,\"time_ns\":{}}}", std::process::id(), now())).unwrap();
            }
            "restore" => {
                let loaded = settings::parse(&fs::read_to_string(&path).unwrap());
                assert!(!loaded.corrupt());
                assert_eq!(loaded.settings.last_page(), Some("developer"));
                let expected = loaded.settings.to_json();
                let mut m = menu();
                for _ in 0..4 { m.press(Button::R); }
                assert_eq!(m.page(), Page::DisplayControls);
                m.reopen(loaded.settings, Env::default());
                assert_eq!(m.page(), Page::Cpu, "a fresh process may reuse its worker Menu too");
                assert_eq!(m.settings().to_json(), expected);
                assert_eq!(m.take_outbox(), Default::default());
                // Continue with actual navigation and close, replacing only the saved page.
                m.press(Button::R);
                assert_eq!(m.page(), Page::Saves);
                m.press(Button::B);
                let continued = settings::parse(&m.serialized()).settings;
                assert_eq!(continued.last_page(), Some("saves"));
                let mut wanted = settings::parse(&expected).settings;
                wanted.set_last_page("saves");
                assert_eq!(continued.to_json(), wanted.to_json());
                fs::write(root.join("restorer.json"), format!("{{\"pid\":{},\"nondefault_restored\":true,\"continued_after_fallback\":true,\"time_ns\":{}}}", std::process::id(), now())).unwrap();
            }
            _ => panic!("unknown process mode"),
        }
        return;
    }
    let root = std::env::temp_dir().join(format!("tr-ui-q5-{}-{}", std::process::id(), now()));
    fs::create_dir(&root).unwrap();
    let exe = std::env::current_exe().unwrap();
    let run = |mode| Command::new(&exe).args(["--exact", NAME, "--nocapture"])
        .env(MODE, mode).env(ROOT, &root).output().unwrap();
    let saved = run("save");
    assert!(saved.status.success(), "saver failed: {} {}", String::from_utf8_lossy(&saved.stdout), String::from_utf8_lossy(&saved.stderr));
    let saver_exit = now();
    let restore_launch = now();
    let restored = run("restore");
    assert!(restored.status.success(), "restorer failed: {} {}", String::from_utf8_lossy(&restored.stdout), String::from_utf8_lossy(&restored.stderr));
    println!("Q5_CONTINUATION saver_exit_ns={saver_exit} restore_launch_ns={restore_launch} saver={} restorer={}",
        fs::read_to_string(root.join("saver.json")).unwrap(), fs::read_to_string(root.join("restorer.json")).unwrap());
    assert!(saver_exit <= restore_launch);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn core04_last_page_roundtrip() {
    for page in Page::ALL {
        let mut s = Settings::factory();
        s.set_last_page(page.id());
        let loaded = settings::parse(&s.to_json());
        assert!(!loaded.corrupt());
        assert_eq!(loaded.settings.last_page(), Some(page.id()));
        assert_eq!(Page::from_id(page.id()), Some(page));
    }
    // It is a top-level key, not a settings row, and a file without it loads.
    let text = Settings::factory().to_json();
    assert!(!text.contains("last_page"), "absent until a page is saved");
    assert!(settings::parse(&text).settings.last_page().is_none());
}

#[test]
fn mii_preferences_is_the_preserved_extra_as_json() {
    assert!(Settings::factory().mii_preferences().is_none());
    let text = r#"{"menu":{},"mii_preferences":{"order":[3,1,2],"name":"x"}}"#;
    let loaded = settings::parse(text);
    assert!(!loaded.corrupt());
    let prefs = loaded.settings.mii_preferences().expect("kept as it is");
    assert_eq!(prefs["order"], serde_json::json!([3, 1, 2]));
    assert_eq!(prefs["name"], "x");
    // It survives a save unchanged.
    let again = settings::parse(&loaded.settings.to_json());
    assert_eq!(again.settings.mii_preferences(), Some(prefs));
}

// ---- Q18, B11: re-roll events --------------------------------------------------------------------

#[test]
fn menu_change_to_cpu_choices_emits_reroll_events() {
    let cases = [
        ("sdi_state", reroll::SDI_DIRECTION),
        ("shield_tilt", reroll::SHIELD_TILT),
        ("ledge_state", reroll::LEDGE_OPTIONS),
        ("throw_state", reroll::THROW_OPTIONS),
    ];
    for (id, bit) in cases {
        let mut m = menu();
        goto(&mut m, id);
        assert_eq!(m.outbox().rerolls, 0);
        m.press(Button::A);
        m.press(Button::A);
        let out = m.take_outbox();
        assert_eq!(out.rerolls, bit, "{id}");
        assert!(out.changed);
        assert_eq!(m.take_outbox().rerolls, 0, "taken once");
    }
    // Rows that do not re-roll: DI (read live), the delay rows (per-grab rule), SDI strength.
    for id in ["di_state", "air_dodge_dir", "ledge_delay", "throw_delay", "pummel_delay", "sdi_strength", "mash_state"] {
        let mut m = menu();
        goto(&mut m, id);
        m.press(Button::A);
        m.press(Button::A);
        m.press(Button::Right);
        let out = m.take_outbox();
        assert_eq!(out.rerolls, 0, "{id}");
        assert!(out.changed, "{id} still changed");
    }
    // Several changes before a take merge; a press that changes nothing emits nothing.
    let mut m = menu();
    goto(&mut m, "throw_state");
    press(&mut m, &[Button::A, Button::B]);
    assert!(!m.take_outbox().changed);
    goto(&mut m, "throw_state");
    press(&mut m, &[Button::A, Button::Down, Button::A, Button::B]);
    goto(&mut m, "ledge_state");
    press(&mut m, &[Button::A, Button::Down, Button::A, Button::B]);
    assert_eq!(m.take_outbox().rerolls, reroll::THROW_OPTIONS | reroll::LEDGE_OPTIONS);
    // Resetting the page re-rolls what it changes back: SDI now, and the ledge and throw rows above.
    goto(&mut m, "sdi_state");
    press(&mut m, &[Button::A, Button::A, Button::B]);
    let _ = m.take_outbox();
    reset_page(&mut m);
    assert_eq!(m.take_outbox().rerolls, reroll::SDI_DIRECTION | reroll::LEDGE_OPTIONS | reroll::THROW_OPTIONS);
    reset_page(&mut m);
    assert_eq!(m.take_outbox().rerolls, 0, "a reset that changes nothing re-rolls nothing");
}

// ---- N7: chord editing, shortcut rows ------------------------------------------------------------

fn chord(bits: &[u8]) -> Chord {
    Chord(bits.iter().fold(0u16, |mask, b| mask | (1 << b)))
}

#[test]
fn core03_shortcut_rows_open_editor() {
    let mut m = menu();
    let one = goto(&mut m, "Menu shortcut 1");
    assert_eq!(m.value(one), RowValue::Chord(Chord::DEFAULT_SHORTCUT_1));
    assert_eq!(Chord::DEFAULT_SHORTCUT_1.to_string(), "Minus");
    assert_eq!(m.value(row_of("Menu shortcut 2")), RowValue::Chord(Chord::NONE));
    assert_eq!(Chord::NONE.to_string(), "disabled");
    assert_eq!(chord(&[6, 8]).to_string(), "ZL + D-pad Up");
    m.press(Button::A);
    assert!(matches!(
        m.screen(),
        Some(Screen::ChordEntry { target: model::menu::ChordTarget::Shortcut(0), captured }) if *captured == Chord::DEFAULT_SHORTCUT_1
    ));
    m.press(Button::B);
    goto(&mut m, "Menu shortcut 2");
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::ChordEntry { target: model::menu::ChordTarget::Shortcut(1), .. })));
    assert!(matches!(ROWS[one].control, Control::Editor(Editor::ShortcutSlot(0))));
}

#[test]
fn core03_permanent_shortcut_row_info() {
    let mut m = menu();
    let r = goto(&mut m, "Permanent shortcut");
    assert!(matches!(ROWS[r].control, Control::Info(model::menu::table::Info::PermanentShortcut)));
    assert_eq!(m.value(r), RowValue::None);
    for b in [Button::A, Button::Left, Button::Right, Button::Y, Button::ZL] {
        m.press(b);
        assert!(m.screen().is_none(), "{b:?} opens nothing on an info row");
    }
    assert_eq!(m.outbox(), &Default::default());
    // Likewise the pilot's info row.
    let mut m = menu_with(Env { pilot_present: true, ..Env::default() });
    let r = goto(&mut m, "P1 control");
    m.press(Button::A);
    assert!(m.screen().is_none());
    assert_eq!(m.grey(r), None);
}

#[test]
fn chords_permanent_chord_fixed() {
    // Nothing the menu offers edits or disables the permanent chord: it has no editor row, no
    // setting and no outbox request.
    assert!(!ROWS.iter().any(|r| matches!(r.control, Control::Editor(Editor::ShortcutSlot(n)) if n > 1)));
    assert!(settings::DEFS.iter().all(|d| !d.id.contains("permanent")));
}

#[test]
fn chords_shortcut_rows_open_editor() {
    core03_shortcut_rows_open_editor();
}

#[test]
fn chords_shortcut_change_applies_immediately() {
    let mut m = menu();
    goto(&mut m, "Menu shortcut 1");
    m.press(Button::A);
    m.chord_entry(chord(&[6, 8])); // ZL + D-pad Up
    // Nothing is live until the editor confirms.
    assert_eq!(m.env().shortcuts[0], Chord::DEFAULT_SHORTCUT_1);
    m.press(Button::A);
    assert_eq!(m.env().shortcuts[0], chord(&[6, 8]), "the new chord is live at once");
    assert_eq!(m.value(row_of("Menu shortcut 1")), RowValue::Chord(chord(&[6, 8])));
    assert!(m.screen().is_none());
    let out = m.take_outbox();
    assert_eq!(out.button_config, Some([chord(&[6, 8]), Chord::NONE]), "the menu asks for button_config.ini");
    assert!(!out.changed, "the shortcut is not a menu setting");
    // Slot 2 is set the same way; the request carries both slots.
    goto(&mut m, "Menu shortcut 2");
    m.press(Button::A);
    m.chord_entry(chord(&[7, 9]));
    m.press(Button::A);
    assert_eq!(m.take_outbox().button_config, Some([chord(&[6, 8]), chord(&[7, 9])]));
}

#[test]
fn chords_ini_written_only_on_confirmed_change() {
    let mut m = menu();
    // Cancelled, unchanged, and unrelated changes write nothing.
    goto(&mut m, "Menu shortcut 1");
    m.press(Button::A);
    m.chord_entry(chord(&[6, 8]));
    m.press(Button::B);
    assert_eq!(m.take_outbox().button_config, None);
    assert_eq!(m.env().shortcuts[0], Chord::DEFAULT_SHORTCUT_1);
    m.press(Button::A);
    m.press(Button::A); // confirm the chord it already has
    assert_eq!(m.take_outbox().button_config, None);
    goto(&mut m, "L+R+A Reset");
    m.press(Button::A);
    assert_eq!(m.take_outbox().button_config, None);
    // Y empties the chord; confirming disables the slot and asks for the write once.
    goto(&mut m, "Menu shortcut 1");
    press(&mut m, &[Button::A, Button::Y]);
    assert!(matches!(m.screen(), Some(Screen::ChordEntry { captured, .. }) if captured.is_empty()));
    m.press(Button::A);
    assert_eq!(m.take_outbox().button_config, Some([Chord::NONE, Chord::NONE]));
    assert_eq!(m.value(row_of("Menu shortcut 1")), RowValue::Chord(Chord::NONE));
    // Opening and pages written nothing (boot or any other event).
    m.press(Button::R);
    assert_eq!(m.take_outbox(), Default::default());
    // Resetting the page does not touch the INI-held shortcuts.
    goto(&mut m, "Menu shortcut 1");
    reset_page(&mut m);
    assert_eq!(m.env().shortcuts[0], Chord::NONE);
    assert_eq!(m.take_outbox().button_config, None);
}

#[test]
fn button_chords_editor_writes_the_chord_setting() {
    let mut m = menu();
    goto(&mut m, "Button chords");
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::Chords { cursor: 0 })));
    m.press(Button::A);
    // The editor starts with the current chord: Save State Save = ZL + D-pad Down.
    assert!(matches!(m.screen(), Some(Screen::ChordEntry { captured, .. }) if *captured == chord(&[6, 9])));
    m.chord_entry(chord(&[4, 5]));
    m.press(Button::A);
    assert_eq!(get(&m, "save_state_save"), [0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert!(matches!(m.screen(), Some(Screen::Chords { cursor: 0 })), "back on the list");
    assert!(m.take_outbox().changed);
    // The sixth row is Cycle CPU (`cpu_cycle`); cancelling leaves it.
    m.press(Button::Up);
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::ChordEntry { captured, .. }) if *captured == chord(&[6, 10])));
    m.chord_entry(chord(&[0]));
    m.press(Button::B);
    assert_eq!(get(&m, "cpu_cycle")[0], 0);
    // A capture outside the editor is ignored.
    m.press(Button::B);
    m.chord_entry(chord(&[1]));
    assert!(m.screen().is_none());
}

#[test]
fn chord_text_and_row_conversion() {
    assert_eq!(chord(&[6, 9]).to_string(), "ZL + D-pad Down");
    assert_eq!(Chord::from_row(&Chord(0b1010).to_row()), Chord(0b1010));
    assert_eq!(Chord::from_row(&[0, 3, 0, 5]), Chord(0b1010), "any weight counts as a member");
    assert_eq!(Chord(0).count(), 0);
}

#[test]
fn replay_capture_toggle_writes_its_ini() {
    // The INI is the file of record and defaults to off (lead ruling 2026-11-02): so does the env.
    let mut m = menu();
    let r = goto(&mut m, "Replay capture");
    assert_eq!(m.value(r), RowValue::Toggle(false));
    m.press(Button::A);
    assert_eq!(m.value(r), RowValue::Toggle(true));
    assert_eq!(m.take_outbox().replay_capture, Some(true));
    m.press(Button::A);
    assert_eq!(m.take_outbox().replay_capture, Some(false));
    // The file value is the input; the toggle is not a settings row.
    let m = menu_with(Env { replay_capture: true, ..Env::default() });
    assert_eq!(m.value(r), RowValue::Toggle(true));
    assert!(m.settings().menu.get("replay_capture").is_none());
}

// ---- N5: paging by console mode ------------------------------------------------------------------

fn pages_of(layout: &GridLayout) -> Vec<usize> {
    (0..layout.pages()).map(|p| layout.page_len(p)).collect()
}

#[test]
fn core03_item_grid_layout_docked() {
    let l = GridLayout::new(GridKind::Items, ConsoleMode::Docked);
    assert_eq!((l.cols, l.rows, l.short_names), (5, 8, true));
    assert_eq!(pages_of(&l), [38]);
    assert_eq!(l.pages(), 1);
}

#[test]
fn core03_item_grid_layout_handheld() {
    let l = GridLayout::new(GridKind::Items, ConsoleMode::Handheld);
    assert_eq!((l.cols, l.rows, l.short_names), (4, 4, false));
    assert_eq!(pages_of(&l), [16, 16, 6]);
}

#[test]
fn core03_stage_grid_layout_docked() {
    let l = GridLayout::new(GridKind::Stages, ConsoleMode::Docked);
    assert_eq!((l.cols, l.rows, l.short_names), (5, 8, true));
    assert_eq!(l.tiles(), STAGE_TILES, "All + the catalog");
}

#[test]
fn core03_stage_grid_pages_docked_40_40_38() {
    let l = GridLayout::new(GridKind::Stages, ConsoleMode::Docked);
    assert_eq!(pages_of(&l), [40, 40, STAGE_TILES - 80]);
}

#[test]
fn core03_stage_grid_layout_handheld() {
    let l = GridLayout::new(GridKind::Stages, ConsoleMode::Handheld);
    assert_eq!((l.cols, l.rows, l.short_names), (4, 4, false));
}

#[test]
fn core03_stage_grid_pages_handheld_8() {
    let l = GridLayout::new(GridKind::Stages, ConsoleMode::Handheld);
    assert_eq!(l.pages(), 8);
    assert_eq!(pages_of(&l), [16, 16, 16, 16, 16, 16, 16, STAGE_TILES - 112]);
}

#[test]
fn core03_item_grid_pages_cover_38_items_once() {
    for mode in [ConsoleMode::Docked, ConsoleMode::Handheld] {
        let l = GridLayout::new(GridKind::Items, mode);
        let tiles: Vec<usize> = (0..l.pages()).flat_map(|p| (0..l.per_page()).filter_map(move |c| l.tile(p, c))).collect();
        assert_eq!(tiles, (0..ITEM_TILES).collect::<Vec<_>>(), "{mode:?}: every item once, in order");
    }
}

#[test]
fn core03_stage_grid_pages_cover_every_tile_once() {
    for mode in [ConsoleMode::Docked, ConsoleMode::Handheld] {
        let l = GridLayout::new(GridKind::Stages, mode);
        let tiles: Vec<usize> = (0..l.pages()).flat_map(|p| (0..l.per_page()).filter_map(move |c| l.tile(p, c))).collect();
        assert_eq!(tiles, (0..STAGE_TILES).collect::<Vec<_>>(), "{mode:?}: All first, every tile once, in order");
        assert_eq!(l.tile(l.pages(), 0), None);
    }
}

#[test]
fn core03_stage_grid_rotation_and_form_same_in_both_layouts() {
    // The model's part: the tile order is the same list in both layouts (the badges and the form
    // tag are drawn per tile, SPEC-core-03), and the layout change never reorders tiles.
    let docked = GridLayout::new(GridKind::Stages, ConsoleMode::Docked);
    let handheld = GridLayout::new(GridKind::Stages, ConsoleMode::Handheld);
    let order = |l: &GridLayout| -> Vec<usize> { (0..l.pages()).flat_map(|p| (0..l.page_len(p)).filter_map(move |c| l.tile(p, c))).collect() };
    assert_eq!(order(&docked), order(&handheld));
}

#[test]
fn change_stage_comes_before_the_rotation_rows_and_opens_the_stages_without_all() {
    let mut m = menu_with(Env { console: ConsoleMode::Docked, ..Env::default() });
    let titles: Vec<&str> = (0..m.entry_count(Page::FightersStages))
        .filter_map(|i| match m.entry(Page::FightersStages, i) { Some(Entry::Row(r)) => Some(ROWS[r].title), _ => None })
        .collect();
    let at = |t: &str| titles.iter().position(|x| *x == t).unwrap_or_else(|| panic!("{t} in {titles:?}"));
    assert!(at("Change Stage") < at("Lobby Stages") && at("Lobby Stages") < at("Training Stage Rotation"), "{titles:?}");
    goto(&mut m, "Change Stage");
    m.press(Button::A);
    let Some(Screen::Grid { layout, page: 0, cell: 0 }) = m.screen().copied() else { panic!("{:?}", m.screen()) };
    assert_eq!(layout, GridLayout::new(GridKind::StageChange, ConsoleMode::Docked));
    m.press(Button::Y);
    assert_eq!(m.take_stage_press(), Some((1, Button::Y)), "the first tile is the first stage (tile 1), not All");
    m.press(Button::A);
    assert_eq!(m.take_stage_press(), Some((1, Button::A)));
    m.press(Button::B);
    assert!(m.screen().is_none(), "B goes back to the page: {:?}", m.screen());
}

#[test]
fn change_stage_grid_pages_cover_every_stage_once_and_no_all_tile() {
    for mode in [ConsoleMode::Docked, ConsoleMode::Handheld] {
        let l = GridLayout::new(GridKind::StageChange, mode);
        let tiles: Vec<usize> = (0..l.pages()).flat_map(|p| (0..l.page_len(p)).filter_map(move |c| l.tile(p, c))).collect();
        assert_eq!(tiles, (1..STAGE_TILES).collect::<Vec<_>>(), "{mode:?}: the stage grid's numbers, All left out");
        assert_eq!(l.tile(l.pages(), 0), None);
    }
}

#[test]
fn core03_item_grid_layout_fixed_at_open() {
    let mut m = menu_with(Env { console: ConsoleMode::Docked, ..Env::default() });
    goto(&mut m, "Give item");
    m.press(Button::A);
    let layout = |m: &Menu| match m.screen() {
        Some(Screen::Grid { layout, .. }) => *layout,
        other => panic!("{other:?}"),
    };
    assert_eq!(layout(&m), GridLayout::new(GridKind::Items, ConsoleMode::Docked));
    // The console is undocked while the grid is open: nothing changes until it is opened again.
    m.set_console(ConsoleMode::Handheld);
    m.press(Button::Down);
    assert_eq!(layout(&m), GridLayout::new(GridKind::Items, ConsoleMode::Docked));
    m.press(Button::B);
    m.press(Button::A);
    assert_eq!(layout(&m), GridLayout::new(GridKind::Items, ConsoleMode::Handheld));
    // The stage grid reads the mode the same way.
    m.press(Button::B);
    goto(&mut m, "Lobby Stages");
    m.press(Button::A);
    assert_eq!(layout(&m), GridLayout::new(GridKind::Stages, ConsoleMode::Handheld));
}

#[test]
fn grid_screen_pages_with_l_r_and_keeps_the_cursor_on_a_tile() {
    let mut m = menu_with(Env { console: ConsoleMode::Handheld, ..Env::default() });
    goto(&mut m, "Give item");
    m.press(Button::A);
    let at = |m: &Menu| match m.screen() {
        Some(Screen::Grid { page, cell, .. }) => (*page, *cell),
        other => panic!("{other:?}"),
    };
    assert_eq!(at(&m), (0, 0));
    m.press(Button::R);
    m.press(Button::R);
    assert_eq!(at(&m), (2, 0));
    m.press(Button::R);
    assert_eq!(at(&m), (0, 0), "pages wrap");
    // The last page has six tiles: moving to it from a later cell lands on its last tile.
    for _ in 0..3 {
        m.press(Button::Down);
    }
    m.press(Button::Right);
    assert_eq!(at(&m), (0, 13));
    m.press(Button::L);
    assert_eq!(at(&m), (2, 5));
    // Docked, the item grid is one page: L / R stay on it.
    let mut m = menu();
    goto(&mut m, "Give item");
    m.press(Button::A);
    m.press(Button::R);
    assert_eq!(at(&m), (0, 0));
    // The roster rows open the one-page 13 x 7 portrait grid (the same in both console modes) and close with B.
    m.press(Button::B);
    goto(&mut m, "Kirby Hats");
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::Grid { layout, page: 0, cell: 0 }) if layout.kind == GridKind::Roster(RosterUse::Hats) && layout.cols == 13 && layout.rows == 7 && layout.pages() == 1));
    // A on a roster tile goes to the roster editor (the worker), with the tile and the button.
    m.press(Button::A);
    assert_eq!(m.take_roster_press(), Some((0, Button::A)));
    for button in [Button::X, Button::Y, Button::R, Button::ZR] {
        m.press(button);
        assert_eq!(m.take_roster_press(), Some((0, button)), "roster input belongs to its editor");
        assert!(matches!(m.screen(), Some(Screen::Grid { page: 0, cell: 0, .. })));
    }
    m.press(Button::B);
    assert!(m.screen().is_none());
}

// ---- cross-checks --------------------------------------------------------------------------------

#[test]
fn dropdown_options_are_unique_and_row_values_read_back() {
    // Every dropdown row's factory choice reads back as a valid option of its list.
    let m = menu();
    for (i, row) in ROWS.iter().enumerate() {
        if let Control::Dropdown(labels) = row.control {
            match m.value(i) {
                RowValue::Choice(Some(at)) => assert!(at < labels.len(), "{}", row.title),
                other => panic!("{}: {other:?}", row.title),
            }
            let mut sorted = labels.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), labels.len(), "{} has a repeated label", row.title);
        }
    }
    assert_eq!(m.value(row_of("input_display_strip")), RowValue::Choice(Some(1)), "Frame strip");
    assert_eq!(m.value(row_of("recording_duration")), RowValue::Choice(Some(3)), "150 frames");
}


#[test]
fn page_defaults_controls_roundtrip_and_reset_only_the_focused_complete_row() {
    let mut m = menu();
    goto(&mut m, "aerial_delay");
    set(&mut m, "aerial_delay_range", &[3, 8]);
    m.press(Button::X);
    let defaults = m.settings().defaults.clone();
    m.bulk_update(&[("aerial_delay", &[0, 1])]);
    set(&mut m, "stale_dodges", &[0, 1]);
    m.press(Button::Y);
    assert_eq!(get(&m, "aerial_delay_range"), [3, 8]);
    assert!(get(&m, "aerial_delay").iter().all(|v| *v == 0));
    assert_eq!(get(&m, "stale_dodges"), [0, 1], "row reset does not reset its page");
    assert_eq!(m.settings().defaults, defaults, "reset never overwrites saved defaults");
    let json = m.serialized();
    let reloaded = settings::parse(&json);
    assert_eq!(reloaded.settings.defaults, defaults);
}


#[test]
fn saving_committed_current_values_alone_requests_persistence_without_rerolls() {
    let mut settings = Settings::factory();
    settings.menu.set("stale_dodges", &[0, 1]).unwrap();
    let mut m = Menu::new(settings, Env::default());
    assert_eq!(m.take_outbox(), Default::default());
    m.press(Button::X);
    m.press(Button::B);
    let intent = m.take_outbox();
    assert!(intent.changed, "worker close must request the settings write for an X-only save");
    assert_eq!(intent.rerolls, 0, "saving defaults does not change current CPU choices");
    let loaded = settings::parse(&m.serialized());
    assert!(!loaded.corrupt());
    assert_eq!(loaded.settings.defaults.get("stale_dodges"), Some(&[0, 1][..]));
    assert_eq!(loaded.settings.defaults, loaded.settings.menu);
    m.reopen(loaded.settings, Env::default());
    m.press(Button::X);
    assert_eq!(m.take_outbox(), Default::default(), "saving identical defaults is a no-op");
}
