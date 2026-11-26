//! Regression for Loop list confirmation, cancellation and its existing shared cycling setter.
use crate::model;
use crate::engine;

use engine::library::{LoopCount, RecordingIndex, Role};
use engine::macros::{InputFrame, MacroRecording, StartState};
use engine::playback::{Book, Playback};
use model::menu::input::Button;
use model::menu::table::{COMBOS_SEQ, ROWS, SAVES_MACROS};
use model::menu::{Entry, Env, Menu, Screen, LOOP_ROW};
use model::recording_menu::{LoopMark, ViewRole};
use model::settings::Settings;

const COUNTS: [(LoopMark, Option<usize>); 6] = [
    (LoopMark::Infinite, None),
    (LoopMark::Finite(1), Some(1)),
    (LoopMark::Finite(2), Some(2)),
    (LoopMark::Finite(3), Some(3)),
    (LoopMark::Finite(5), Some(5)),
    (LoopMark::Finite(10), Some(10)),
];

static LOOP_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn focus(menu: &mut Menu, target: Entry) {
    for _ in 0..=menu.entry_count(menu.page()) {
        if menu.focus() == Some(target) { return; }
        menu.press(if matches!(menu.focus(), Some(Entry::Group(g)) if g == usize::from(SAVES_MACROS) || g == usize::from(COMBOS_SEQ)) {
            Button::JumpDown
        } else {
            Button::Down
        });
    }
    panic!("cannot focus {target:?}");
}

fn menu(role: ViewRole) -> Menu {
    let mut menu = Menu::new(Settings::factory(), Env::default());
    let row = ROWS.iter().position(|row| row.id == LOOP_ROW).unwrap();
    for _ in 0..6 {
        if menu.page() == ROWS[row].page() { break; }
        menu.press(Button::R);
    }
    assert_eq!(menu.page(), ROWS[row].page());
    menu.recording_mut().adopt(
        None,
        [Some("macro-a".into()), Some("macro-a".into())],
        [LoopMark::Finite(3), LoopMark::Finite(5)],
        None,
    );
    if role == ViewRole::Cpu { menu.press(Button::ZR); }
    let group = usize::from(ROWS[row].group);
    if !menu.group_is_open(group) {
        focus(&mut menu, Entry::Group(group));
        menu.press(Button::A);
    }
    focus(&mut menu, Entry::Row(row));
    assert_eq!(menu.recording().page_role(), role);
    menu
}

fn pick(menu: &mut Menu, index: usize) {
    menu.press(Button::A);
    for _ in 0..6 {
        if matches!(menu.screen(), Some(Screen::List { cursor, .. }) if *cursor == index) { break; }
        menu.press(Button::Down);
    }
    assert!(matches!(menu.screen(), Some(Screen::List { cursor, .. }) if *cursor == index));
    menu.press(Button::A);
    assert!(menu.screen().is_none());
}

struct Scratch(crate::test_scratch::Scratch);
impl Scratch {
    fn new() -> Self {
        Self(crate::test_scratch::Scratch::new("tr-loop-picker"))
    }
}
/// Use the worker's real publication seam, reload its index, then execute the real playback machine.
fn assert_playback(root: &Scratch, role: ViewRole, expected: Option<usize>) {
    let index = RecordingIndex::load(&root.0).unwrap();
    let role = if role == ViewRole::Player { Role::Player } else { Role::Cpu };
    assert_eq!(
        index.loop_count(role),
        expected.map_or(LoopCount::Infinite, |n| LoopCount::Finite(n as u8)),
    );
    let mut book = Book::new(index);
    let frames = [0x10, 0x20].map(|buttons| InputFrame { buttons, ..InputFrame::default() }).to_vec();
    book.add_recording("macro-a", MacroRecording::new("Two frames", "13.0.5", StartState::neutral(0, 0, 1.0), frames).unwrap());
    let mut playback = Playback::new();
    assert!(playback.play_chord(role, &book).started);
    for i in 0..expected.map_or(24, |n| n * 2) {
        assert_eq!(playback.step(role, &book).expect("selected pass").frame.buttons, [0x10, 0x20][i % 2]);
    }
    if expected.is_none() {
        assert!(playback.playing(role), "infinite continues beyond the finite test window");
        assert!(playback.play_chord(role, &book).stopped);
    }
    assert!(playback.step(role, &book).is_none(), "no extra pass after a finite count or explicit stop");
    assert!(!playback.playing(role));
}


fn card() -> Scratch {
    let root = Scratch::new();
    let mut index = RecordingIndex::fresh();
    for role in [Role::Player, Role::Cpu] {
        index.set_active(role, Some("macro-a"), 1).unwrap();
    }
    index.set_loop(Role::Player, LoopCount::Finite(3));
    index.set_loop(Role::Cpu, LoopCount::Finite(5));
    index.save(&root.0).unwrap();
    root
}

fn publish_if_changed(root: &Scratch, screen: &mut Menu) -> bool {
    use engine::rr_host::{recording_version, take_menu_effect};
    let version = recording_version();
    let _ = take_menu_effect(&root.0, screen.recording_mut(), Some(version));
    recording_version() != version
}

fn close_reopen(root: &Scratch, mut screen: Menu, role: ViewRole, mark: LoopMark) {
    screen.press(Button::B);
    assert!(screen.is_closed());
    drop(screen);
    let mut reopened = menu(role);
    engine::rr_runtime::apply_index(&RecordingIndex::load(&root.0).unwrap(), reopened.recording_mut());
    assert_eq!(reopened.recording().loop_of(role), mark, "the reopened Options row reflects the index");
}

#[test]
fn loop_list_selection_is_published_and_used_by_playback() {
    let _guard = LOOP_TEST.lock().unwrap_or_else(|err| err.into_inner());
    for role in [ViewRole::Player, ViewRole::Cpu] {
        for (index, (mark, passes)) in COUNTS.into_iter().enumerate() {
            let mut menu = menu(role);
            let other = menu.recording().loop_of(role.other());
            pick(&mut menu, index);
            assert_eq!(menu.recording().loop_of(role), mark);
            assert_eq!(menu.recording().loop_of(role.other()), other);
            let root = card();
            assert_eq!(publish_if_changed(&root, &mut menu), mark != if role == ViewRole::Player { LoopMark::Finite(3) } else { LoopMark::Finite(5) });
            assert_playback(&root, role, passes);
            close_reopen(&root, menu, role, mark);
        }
    }
}

#[test]
fn loop_list_cancel_keeps_the_previous_playback_count() {
    let _guard = LOOP_TEST.lock().unwrap_or_else(|err| err.into_inner());
    for (role, passes) in [(ViewRole::Player, 3), (ViewRole::Cpu, 5)] {
        let mut menu = menu(role);
        let before = menu.recording().selection();
        menu.press(Button::A);
        menu.press(Button::Down);
        menu.press(Button::B);
        assert!(menu.screen().is_none());
        assert_eq!(menu.recording().selection(), before);
        let root = card();
        assert!(!publish_if_changed(&root, &mut menu), "cancel does not write the index");
        assert_playback(&root, role, Some(passes));
    }
}

#[test]
fn loop_list_and_left_right_choose_the_same_counts() {
    let _guard = LOOP_TEST.lock().unwrap_or_else(|err| err.into_inner());
    for role in [ViewRole::Player, ViewRole::Cpu] {
        for button in [Button::Left, Button::Right] {
            for (target, (mark, passes)) in COUNTS.into_iter().enumerate() {
                let mut listed = menu(role);
                let mut cycled = menu(role);
                pick(&mut listed, target);
                for _ in 0..6 {
                    if cycled.recording().loop_row_index() == target { break; }
                    cycled.press(button);
                }
                assert_eq!(cycled.recording().loop_of(role), mark);
                assert_eq!(cycled.recording().selection(), listed.recording().selection());
                let root = card();
                let _ = publish_if_changed(&root, &mut cycled);
                assert_playback(&root, role, passes);
                close_reopen(&root, cycled, role, mark);
            }
        }
    }
}

#[test]
fn resetting_loop_publishes_both_roles_once() {
    let _guard = LOOP_TEST.lock().unwrap_or_else(|err| err.into_inner());
    let root = card();
    let mut screen = menu(ViewRole::Cpu);
    let page = screen.page();
    let reset = screen.entry(page, screen.entry_count(page) - 1).unwrap();
    assert!(matches!(reset, Entry::ResetPage(_)));
    focus(&mut screen, reset);
    for changed in [true, false] {
        screen.press(Button::A);
        assert!(matches!(screen.screen(), Some(Screen::ConfirmReset)));
        screen.press(Button::A);
        assert_eq!(publish_if_changed(&root, &mut screen), changed, "only a changed reset writes");
    }
    screen.press(Button::B);
    assert!(screen.is_closed());
    for role in [ViewRole::Player, ViewRole::Cpu] {
        assert_playback(&root, role, Some(1));
        let mut reopened = menu(role);
        engine::rr_runtime::apply_index(&RecordingIndex::load(&root.0).unwrap(), reopened.recording_mut());
        assert_eq!(reopened.recording().loop_of(role), LoopMark::Finite(1));
    }
}

#[test]
fn both_loop_edits_persist_during_a_warm_catalog_check() {
    use engine::rr_host::{publish_selection_at, recording_version, take_menu_effect};
    use engine::rr_runtime::{apply_catalog, apply_index, CatalogCache, MenuIndex};
    use model::menu::PlaceKeeper;

    let _guard = LOOP_TEST.lock().unwrap_or_else(|err| err.into_inner());
    let root = card();
    RecordingIndex::update(&root.0, |index| {
        index.set_loop(Role::Player, LoopCount::Finite(1));
        Ok(true)
    }).unwrap();
    let macros = engine::macros::Library::new(root.0.join(engine::library::MACROS_DIR));
    for n in 0..60 {
        macros.save_new(&MacroRecording::new(
            format!("Take {n}"), "13.0.5", StartState::neutral(0, 0, 1.0), vec![InputFrame::default()],
        ).unwrap()).unwrap();
    }
    let saves = model::store::Store::new(root.0.join(engine::library::SAVES_DIR));
    let state = model::state::record::SavedState::never_saved();
    for n in 0..30 {
        saves.save_new(&engine::library::NamedSave::new(format!("Save {n}"), state.clone(), state.clone()).unwrap()).unwrap();
    }
    let observed = recording_version();
    let mut catalog = CatalogCache::new(&root.0);
    catalog.want(observed.0);
    while catalog.step() {}
    let mut index = MenuIndex::default();
    assert!(index.step(&root.0, observed));
    let cached_index = index.ready(observed).unwrap();

    let mut keeper = PlaceKeeper::new();
    let mut open = Some(menu(ViewRole::Player));
    open.as_mut().unwrap().press(Button::B);
    keeper.drop_model(&mut open);
    let screen = keeper.open(&mut open, Settings::factory(), Env::default());
    let warm = catalog.ready(observed.0).unwrap();
    assert_eq!((warm.macros.len(), warm.saves.len()), (60, 30));
    apply_catalog(warm, cached_index, screen.recording_mut());
    apply_index(cached_index, screen.recording_mut());
    assert_eq!(screen.recording().loop_of(ViewRole::Player), LoopMark::Finite(1));
    catalog.check();
    for wanted in [2, 3] {
        if wanted == 3 {
            assert!(index.ready(recording_version()).is_none(), "the first Right superseded the cached index");
            RecordingIndex::update(&root.0, |index| {
                index.set_slots(&[Some("take-0".into()), None, None, None, None])?;
                index.set_loop(Role::Cpu, LoopCount::Finite(10));
                Ok(true)
            }).unwrap();
        }
        assert!(catalog.step(), "the directory check is still pending");
        if index.ready(recording_version()).is_none() {
            screen.recording_mut().set_loading();
        }
        screen.press(Button::Right);
        assert_eq!(screen.recording().loop_of(ViewRole::Player), LoopMark::Finite(wanted));
        if let Some((_, true, version)) = take_menu_effect(&root.0, screen.recording_mut(), Some(observed)) {
            assert!(publish_selection_at(&root.0, &screen.recording().selection(), None, Some(version)));
        }
    }
    assert!(catalog.step(), "both Right presses happened before discovery finished");
    let saved = RecordingIndex::load(&root.0).unwrap();
    assert_eq!(saved.loop_count(Role::Player), LoopCount::Finite(3),
        "the second Right must persist while the original index snapshot is stale");
    assert_eq!(saved.slots()[0].as_deref(), Some("take-0"), "the Loop edit preserves an intervening slot write");
    assert_eq!(saved.loop_count(Role::Cpu), LoopCount::Finite(10), "the edit changes only the selected role");
    assert_playback(&root, ViewRole::Player, Some(3));
    close_reopen(&root, open.take().unwrap(), ViewRole::Player, LoopMark::Finite(3));
}
