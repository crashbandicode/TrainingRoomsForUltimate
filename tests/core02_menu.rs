//! SPEC-core-02 (menu model) with draft 9 (`MENU-DRAFT.md`), SPEC-core-03 §2.5 (paging, shortcut rows,
//! delay popup), SPEC-core-04 (`last_page`) and SPEC-input-04 (N7 chord editing). Test names are the
//! specs' §9 rows where a row exists; rows that the new page layout replaced (the 8 x 4 submenu grid
//! of upstream) are covered by the test of the same name on the new structure, as noted on each.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::menu::chord::Chord;
use model::menu::delay::{DelayRange, DelaySpec};
use model::menu::grid::{ConsoleMode, GridKind, GridLayout, Move, RosterUse};
use model::menu::input::Button;
use model::menu::table::{row_count, Control, Editor, Page, GROUPS, ROWS};
use model::menu::{
    reroll, DelayView, Entry, Env, Grey, Menu, RowValue, Screen, SliderState,
};
use model::settings::{self, Settings};

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

// ---- the layout (MENU-DRAFT §2 with the approved RC deferrals) -----------------------------------------------------------

fn layout_row_counts_match_the_rc_freeze() {
    let counts = |pilot| {
        [Page::Cpu, Page::Saves, Page::Recording, Page::FightersStages, Page::DisplayControls, Page::Developer]
            .map(|page| row_count(page, pilot))
    };
    // Fighters & Stages has 21 since Change Stage (operator order via the primary, 2026-11-05 12:50Z); Display &
    // Controls lost "Group inputs by status" to the frame strip (SPEC-input-06 §3.1, 2026-11-05); Recording gained the
    // 7 rows of the Combos group (SPEC-input-07 §6, 2026-11-05). The RC's four deferred editors ship again (operator
    // 2026-11-05): Saves has its Setup library rows, Recording Edit placement and the Replay window. Fighters & Stages
    // gained Lobby Items after Stage Hazards (Phase 2 N11, the primary's P-N11, 2026-11-05), then Ready Hats in Loading
    // (P-HOT, LANE-HOT-SLOTS-REPORT.md §5.1). Respawn on platform moved from Saves to Fighters & Stages › Stages
    // (2026-11-06): Saves is one row shorter and Fighters & Stages is one row longer.
    // Recording redesign revision 8.2: Slot and Randomize leave Saves (12 → 10). Fourteen recording
    // rows leave the menu (24 → 12). The two library groups have no setting rows.
    // rc.6: the CPU page gained CPU Level in its own group, Level (31 → 32).
    assert_eq!(counts(false), [32, 10, 12, 24, 10, 3]);
    assert_eq!(counts(true), [31, 10, 12, 24, 10, 3]);
    assert_eq!(counts(false)[..5].iter().sum::<usize>(), 88);
    assert_eq!(counts(true)[..5].iter().sum::<usize>(), 87);
    // Controls (MENU-DRAFT §2): 4 direction grids, 7 delay rows, 13 multi-selects (the 12 checklists,\n    // 3 of them slot sets, and Swap groups).
    let n = |f: &dyn Fn(&Control) -> bool| ROWS.iter().filter(|r| r.vis != model::menu::table::Vis::PilotPresent && f(&r.control)).count();
    assert_eq!(n(&|c| matches!(c, Control::DirectionGrid { .. })), 4);
    assert_eq!(n(&|c| matches!(c, Control::MultiSelectRange { .. })), 7);
    // Randomize slots, Button slots and Save-state slots left the menu (revision 8.2).
    assert_eq!(n(&|c| matches!(c, Control::MultiSelect(_) | Control::SwapGroups)), 10, "the checklists that stayed, and Swap groups");
    assert_eq!(n(&|c| matches!(c, Control::Slider)), 2);
    // Nothing is deferred any more (operator 2026-11-05): every editor has its row, and every group has rows.
    // (104: Lobby Items, Phase 2 N11; 105: Ready Hats, P-HOT; rc.6: CPU Level.)
    assert_eq!(ROWS.len(), 92);
    for editor in [Editor::SetupSave, Editor::SetupLibrary, Editor::EditPlacement, Editor::ReplayWindow] {
        assert_eq!(ROWS.iter().filter(|r| matches!(r.control, Control::Editor(e) if e == editor)).count(), 1, "{editor:?}");
    }
    let replay = ROWS.iter().position(|r| matches!(r.control, Control::Editor(Editor::ReplayWindow))).expect("the Replay window row");
    let macros: Vec<&str> = ROWS.iter().filter(|r| r.group == ROWS[replay].group).map(|r| r.title).collect();
    assert_eq!(macros, ["Ask before switching fighter", "Edit placement", "Replay window", "Replay capture"], "Replays keeps the capture rows");
    assert_eq!(GROUPS[7].title, "Setup library");
    assert_eq!(GROUPS[7].page, Page::Saves);
    let setups: Vec<&str> = ROWS.iter().filter(|r| r.group == 7).map(|r| r.title).collect();
    assert_eq!(setups, ["Save setup to library...", "Setup library"]);
    for (g, group) in GROUPS.iter().enumerate() {
        if g == usize::from(model::menu::table::SAVES_MACROS) || g == usize::from(model::menu::table::COMBOS_SEQ) {
            continue;
        }
        assert!(ROWS.iter().any(|r| usize::from(r.group) == g), "{}", group.title);
    }
    assert!(ROWS.iter().all(|r| usize::from(r.group) < GROUPS.len()));
    for pilot_present in [false, true] {
        let m = menu_with(Env { pilot_present, ..Env::default() });
        assert!((0..m.entry_count(Page::Saves)).any(|i| m.entry(Page::Saves, i) == Some(Entry::Group(7))), "the Setup library section shows");
    }
    assert!(ROWS.iter().any(|r| r.title == "Replay capture"), "retained row Replay capture");
    for title in ["Named library", "Save slot to library...", "Record Slot"] {
        assert!(ROWS.iter().all(|r| r.title != title), "{title} left the menu");
    }
}

// ---- navigation, pages -------------------------------------------------------------------------

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

fn respawn_greys_only_while_a_death_loads_a_state() {
    // Operator 2026-11-13 (TR5): Respawn on platform works with Save States Off. It greys only while a death loads a
    // save state: Save States On and Auto-load on death On. With Save States Off nothing loads, so the row applies.
    let mut m = menu();
    let r = row_of("respawn_invincibility");
    assert_eq!(m.grey(r), None, "factory: Auto-load on death Off");
    set(&mut m, "save_state_autoload", &[1, 0]);
    assert_eq!(m.grey(r), Some(Grey::AutoLoadOn));
    set(&mut m, "save_state_enable", &[0, 1]);
    assert_eq!(m.grey(r), None, "Save States Off: nothing loads, the row applies");
}

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

// ---- N1: the Developer page ----------------------------------------------------------------------

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

fn chords_permanent_chord_fixed() {
    // Nothing the menu offers edits or disables the permanent chord: it has no editor row, no
    // setting and no outbox request.
    assert!(!ROWS.iter().any(|r| matches!(r.control, Control::Editor(Editor::ShortcutSlot(n)) if n > 1)));
    assert!(settings::DEFS.iter().all(|d| !d.id.contains("permanent")));
}

fn chords_shortcut_rows_open_editor() {
    core03_shortcut_rows_open_editor();
}

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
    // The last row is Game speed (`game_speed`, rc.8: R + D-pad Up); cancelling leaves it.
    m.press(Button::Up);
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::ChordEntry { captured, .. }) if *captured == chord(&[5, 8])));
    m.chord_entry(chord(&[0]));
    m.press(Button::B);
    assert_eq!(get(&m, "game_speed")[0], 0);
    // A capture outside the editor is ignored.
    m.press(Button::B);
    m.chord_entry(chord(&[1]));
    assert!(m.screen().is_none());
}

fn chord_text_and_row_conversion() {
    assert_eq!(chord(&[6, 9]).to_string(), "ZL + D-pad Down");
    assert_eq!(Chord::from_row(&Chord(0b1010).to_row()), Chord(0b1010));
    assert_eq!(Chord::from_row(&[0, 3, 0, 5]), Chord(0b1010), "any weight counts as a member");
    assert_eq!(Chord(0).count(), 0);
}

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

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("layout_row_counts_match_the_rc_freeze", layout_row_counts_match_the_rc_freeze as fn()),
        ("core02_table_carriage_return", core02_table_carriage_return as fn()),
        ("core02_page_machine", core02_page_machine as fn()),
        ("core02_y_clears_weighted_only", core02_y_clears_weighted_only as fn()),
        ("core02_clear_hint_rule", core02_clear_hint_rule as fn()),
        ("dropdown_yes_no_random_maps_to_the_weighted_row", dropdown_yes_no_random_maps_to_the_weighted_row as fn()),
        ("number_row_steps_and_clamps", number_row_steps_and_clamps as fn()),
        ("multi_select_ticks_and_all_none", multi_select_ticks_and_all_none as fn()),
        ("situation_overrides_editor_picks_a_situation_then_actions", situation_overrides_editor_picks_a_situation_then_actions as fn()),
        ("core02_slider_states", core02_slider_states as fn()),
        ("core02_slider_clamp_steps", core02_slider_clamp_steps as fn()),
        ("respawn_greys_only_while_a_death_loads_a_state", respawn_greys_only_while_a_death_loads_a_state as fn()),
        ("swap_group_entries_grey_with_the_reason", swap_group_entries_grey_with_the_reason as fn()),
        ("direction_grid_navigation_follows_the_chip_layout", direction_grid_navigation_follows_the_chip_layout as fn()),
        ("shield_tilt_grid_holds_exactly_one_chip", shield_tilt_grid_holds_exactly_one_chip as fn()),
        ("core02_delay_row_exclusion", core02_delay_row_exclusion as fn()),
        ("core02_delay_row_y_clear", core02_delay_row_y_clear as fn()),
        ("core02_delay_no_range_loads_checklist", core02_delay_no_range_loads_checklist as fn()),
        ("delay_range_pick_is_uniform_over_the_step_values", delay_range_pick_is_uniform_over_the_step_values as fn()),
        ("core02_delay_row_bulk_update_clears_range", core02_delay_row_bulk_update_clears_range as fn()),
        ("core02_hidden_page_rows_serialized", core02_hidden_page_rows_serialized as fn()),
        ("mii_preferences_is_the_preserved_extra_as_json", mii_preferences_is_the_preserved_extra_as_json as fn()),
        ("menu_change_to_cpu_choices_emits_reroll_events", menu_change_to_cpu_choices_emits_reroll_events as fn()),
        ("core03_permanent_shortcut_row_info", core03_permanent_shortcut_row_info as fn()),
        ("chords_permanent_chord_fixed", chords_permanent_chord_fixed as fn()),
        ("chords_shortcut_rows_open_editor", chords_shortcut_rows_open_editor as fn()),
        ("chords_shortcut_change_applies_immediately", chords_shortcut_change_applies_immediately as fn()),
        ("chords_ini_written_only_on_confirmed_change", chords_ini_written_only_on_confirmed_change as fn()),
        ("button_chords_editor_writes_the_chord_setting", button_chords_editor_writes_the_chord_setting as fn()),
        ("chord_text_and_row_conversion", chord_text_and_row_conversion as fn()),
        ("replay_capture_toggle_writes_its_ini", replay_capture_toggle_writes_its_ini as fn()),
        ("change_stage_comes_before_the_rotation_rows_and_opens_the_stages_without_all", change_stage_comes_before_the_rotation_rows_and_opens_the_stages_without_all as fn()),
        ("grid_screen_pages_with_l_r_and_keeps_the_cursor_on_a_tile", grid_screen_pages_with_l_r_and_keeps_the_cursor_on_a_tile as fn()),
    ]);
}
