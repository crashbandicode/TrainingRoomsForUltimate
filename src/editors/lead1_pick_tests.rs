//! LEAD-1 (hardware ccd9befd): a Player / CPU Character pick of the fighter a role already stores was a silent no-op. These run
//! the production path a pick takes:
//! - the portrait editor (`RosterEditor::apply`) and the menu's outbox and close commit (`Menu::take_outbox`, `commit_onto`);
//! - the typed views a publish compares (`TrainingModpackMenu::from_snapshot`, `role_views`);
//! - the versioned dispatch transaction (`DispatchChannel`): `publish` is what `compat::menu::after_publish` calls under
//!   `PUBLISHING`, and `take` is what `compat::menu::service_roster_change` calls on the game frame before
//!   `character_roster::menu_dispatch` queues each due load;
//! - the Mii close's reload suppression (`close_picked` into `MiiList::close_for`) and the admission outcome (`over_pending`).
//!
//! [`Pipeline`] only sequences them as the runtime does: a close publishes once, and a frame takes once. Each test chooses when
//! frames run and when a game-side selection edit (a load completing: `Role::sync_selection`) lands. The runtime itself is
//! Switch-only.

lib_unit_tests! {
use crate::editors::roster::{MiiTiles, RosterEditor};
use crate::engine::compat::menu::TrainingModpackMenu;
use crate::engine::swap::cycle_members::{close_picked, over_pending, role_views, Admission, DispatchChannel};
use crate::engine::swap::roster_grid::{Role as GridRole, RosterGrid, CELLS};
use crate::model::menu::grid::RosterUse;
use crate::model::menu::input::Button;
use crate::model::menu::{Env, Menu};
use crate::model::settings::Settings;

const MARIO: u32 = 0x00;
const FOX: u32 = 0x07;
const PIKACHU: u32 = 0x08;
const GAME_AND_WATCH: u32 = 0x1c;
const STEVE: u32 = 0x58;

fn cell_of(kind: u32) -> usize {
    let grid = RosterGrid::new(GridRole::Player);
    (0..CELLS).find(|c| grid.info(*c).and_then(|i| i.kind()) == Some(kind)).expect("kind on the grid")
}

/// The kept JSON of a pick of `kind` (the editor's own one-hot form).
fn pick_value(kind: u32) -> serde_json::Value {
    let mut editor = RosterEditor::new(RosterUse::PlayerPick);
    editor.set_mii(MiiTiles { player: [true; 3], ..MiiTiles::default() });
    let mut values = editor.values(None);
    values.fill(0);
    editor.press(cell_of(kind), Button::A, &mut values);
    RosterEditor::value_of(&values)
}

fn settings_with(player: u32, cpu: u32, player_swapping: bool) -> Settings {
    let mut settings = Settings::factory();
    settings.menu.set_kept("player_character", pick_value(player)).unwrap();
    settings.menu.set_kept("cpu_character", pick_value(cpu)).unwrap();
    settings.menu.set("trio_practice", if player_swapping { &[1, 0] } else { &[0, 1] }).unwrap();
    settings
}

/// The publish → game frame path, sequenced as the runtime sequences it.
struct Pipeline {
    /// The settings the menu opens on and commits onto (`ui::worker::SETTINGS`).
    live: Settings,
    /// The typed view readers have: the last publish, or a game-side edit made since (`compat::menu::view_before_publish`).
    view: TrainingModpackMenu,
    channel: DispatchChannel,
    /// Every load the frames queued: `(role, kind, once)`.
    loads: Vec<(usize, u32, bool)>,
}

impl Pipeline {
    fn new(live: Settings) -> Self {
        let view = TrainingModpackMenu::from_snapshot(&live.menu);
        let channel = DispatchChannel::new();
        channel.publish(None, role_views(&view), [None; 2]); // boot: nothing to compare with
        Self { live, view, channel, loads: Vec::new() }
    }
    fn open(&self) -> Menu {
        Menu::new(self.live.clone(), Env::default())
    }
    /// A game-side selection change (a load completing: `Role::sync_selection` → `store` → the worker's writeback).
    fn game_selects(&mut self, cpu: bool, kind: u32) {
        self.live.menu.set_kept(if cpu { "cpu_character" } else { "player_character" }, pick_value(kind)).unwrap();
        self.view = TrainingModpackMenu::from_snapshot(&self.live.menu);
    }
    /// A publish without a menu close (the input display bind, `ui::worker::toggle_input_display`): `[None; 2]` picks.
    fn publish_unrelated(&mut self) {
        let previous = self.view;
        let (next, _) = crate::model::frame_strip::toggled(self.live.menu.get(crate::model::frame_strip::ROW));
        self.live.menu.set(crate::model::frame_strip::ROW, &next).unwrap();
        self.view = TrainingModpackMenu::from_snapshot(&self.live.menu);
        self.channel.publish(Some(role_views(&previous)), role_views(&self.view), [None; 2]);
    }
    /// `ui::worker::close` → `published::publish` → `compat::menu::after_publish`.
    fn close(&mut self, menu: &mut Menu) {
        let outbox = menu.take_outbox();
        let settings = menu.commit_onto(&self.live);
        let next = TrainingModpackMenu::from_snapshot(&settings.menu);
        let previous = self.view;
        self.live = settings;
        self.view = next;
        self.channel.publish(Some(role_views(&previous)), role_views(&next), outbox.picks);
    }
    /// `compat::menu::service_roster_change` → `character_roster::menu_dispatch`.
    fn frame(&mut self) {
        for (role, due) in self.channel.take().into_iter().enumerate() {
            if let Some(cause) = due {
                self.loads.push((role, cause.target, cause.once));
            }
        }
    }
}

fn pick(menu: &mut Menu, cpu: bool, kind: u32) {
    let mut editor = RosterEditor::new(if cpu { RosterUse::CpuPick } else { RosterUse::PlayerPick });
    editor.set_mii(MiiTiles { player: [true; 3], cpu: [true; 3], ..MiiTiles::default() });
    editor.apply(menu, cell_of(kind), Button::A);
}

fn an_idle_repick_loads_the_stored_fighter_once() {
    // Hardware run-9: P1 G&W, stored pick Fox, A on "> Fox". ccd9befd: no load at all.
    let mut p = Pipeline::new(settings_with(FOX, MARIO, false));
    let mut menu = p.open();
    pick(&mut menu, false, FOX);
    p.close(&mut menu);
    p.frame();
    p.frame();
    assert_eq!(p.loads, vec![(0, FOX, true)], "one load the user chose, with Player Swapping Off");
}

fn a_changed_pick_loads_exactly_once() {
    let mut p = Pipeline::new(settings_with(GAME_AND_WATCH, MARIO, true));
    let mut menu = p.open();
    pick(&mut menu, false, FOX);
    p.close(&mut menu);
    p.frame();
    p.frame();
    p.publish_unrelated();
    p.frame();
    assert_eq!(p.loads, vec![(0, FOX, true)]);
}

/// Review R1: an earlier load completes and writes its fighter after the close published the repick, before the frame took it.
/// ca50f990 decided against the live typed menu (now the completion's fighter) and loaded nothing for the pick.
fn a_completion_after_the_publish_keeps_the_pick_for_both_roles() {
    let mut p = Pipeline::new(settings_with(FOX, STEVE, true));
    let mut menu = p.open();
    pick(&mut menu, false, FOX); // a repick
    pick(&mut menu, true, STEVE); // a repick
    p.close(&mut menu);
    p.game_selects(false, PIKACHU); // the earlier Player load completes
    p.game_selects(true, MARIO); // and the CPU's
    p.frame();
    p.frame();
    assert_eq!(p.loads, vec![(0, FOX, true), (1, STEVE, true)], "the picks' own targets load, once each");
}

/// Review R2: an earlier publication is still unconsumed when a close publishes a changed pick, and the frame runs between them
/// (and again after). ca50f990 re-compared the re-noted older change against the live menu and loaded the pick twice.
fn an_overlapping_publication_loads_the_pick_once() {
    let mut p = Pipeline::new(settings_with(GAME_AND_WATCH, MARIO, true));
    p.publish_unrelated(); // A: the input display bind, no fighter cause
    let mut menu = p.open();
    pick(&mut menu, false, FOX);
    p.frame(); // the frame takes A before the close publishes B
    p.close(&mut menu); // B
    p.frame();
    p.publish_unrelated(); // C, after B, with the same previous selection already loaded
    p.frame();
    p.frame();
    assert_eq!(p.loads, vec![(0, FOX, true)]);
    // Two closes before one frame: the newer cause supersedes the older (the newest target loads, once).
    let mut menu = p.open();
    pick(&mut menu, false, STEVE);
    p.close(&mut menu);
    let mut menu = p.open();
    pick(&mut menu, false, MARIO);
    p.close(&mut menu);
    p.frame();
    p.frame();
    assert_eq!(p.loads, vec![(0, FOX, true), (0, MARIO, true)]);
}

fn player_swapping_turned_on_with_a_repick_loads_once() {
    // Review F2: Off→On loads the stored pick; with a repick in the same visit that is still one load.
    let mut p = Pipeline::new(settings_with(FOX, MARIO, false));
    let mut menu = p.open();
    menu.bulk_update(&[("trio_practice", &[1, 0])]);
    pick(&mut menu, false, FOX);
    p.close(&mut menu);
    p.frame();
    p.frame();
    assert_eq!(p.loads, vec![(0, FOX, true)]);
}

fn a_repick_survives_a_live_selection_writeback_before_the_close() {
    // Review F3: the menu opened on Fox; a swap finishing while it is open writes Pikachu back; A on "> Fox".
    let mut p = Pipeline::new(settings_with(FOX, MARIO, true));
    let mut menu = p.open();
    p.game_selects(false, PIKACHU);
    pick(&mut menu, false, FOX);
    p.close(&mut menu);
    assert_eq!(p.live.menu.extra().get("player_character"), Some(&pick_value(FOX)), "the player's pick wins the merge");
    p.frame();
    assert_eq!(p.loads, vec![(0, FOX, true)]);
}

fn two_presses_in_one_visit_load_the_last() {
    // Ruling F5: the last pick per role per visit wins.
    let mut p = Pipeline::new(settings_with(GAME_AND_WATCH, MARIO, true));
    let mut menu = p.open();
    pick(&mut menu, false, FOX);
    pick(&mut menu, false, STEVE);
    p.close(&mut menu);
    p.frame();
    assert_eq!(p.loads, vec![(0, STEVE, true)]);
}

fn both_roles_in_one_visit_load_once_each() {
    let mut p = Pipeline::new(settings_with(FOX, MARIO, true));
    let mut menu = p.open();
    pick(&mut menu, false, FOX); // a repick
    pick(&mut menu, true, STEVE); // a change
    p.close(&mut menu);
    p.frame();
    p.frame();
    assert_eq!(p.loads, vec![(0, FOX, true), (1, STEVE, true)]);
}

fn a_close_without_a_pick_loads_nothing() {
    let mut p = Pipeline::new(settings_with(FOX, MARIO, true));
    let mut menu = p.open();
    menu.bulk_update(&[("trio_practice", &[1, 0])]);
    p.close(&mut menu);
    p.frame();
    assert!(p.loads.is_empty());
    // A cycle-order edit or an X form preview is not a pick either.
    let mut menu = p.open();
    RosterEditor::new(RosterUse::Cycles).apply(&mut menu, cell_of(FOX), Button::A);
    RosterEditor::new(RosterUse::PlayerPick).apply(&mut menu, cell_of(FOX), Button::X);
    p.close(&mut menu);
    p.frame();
    assert!(p.loads.is_empty());
}

fn a_waiting_user_load_is_busy_and_an_automatic_one_is_superseded() {
    // Review F4: a pick meeting the role's own user load still waiting is refused (its site notifies); an automatic one is replaced.
    assert_eq!(over_pending(Some(true)), Admission::Busy);
    assert_eq!(over_pending(Some(false)), Admission::Superseded);
    assert_eq!(over_pending(None), Admission::Accepted);
}

fn a_mii_preference_reload_and_a_repick_have_one_owner() {
    use crate::engine::mii::inventory::{build_snapshot, MiiClass, RawSlot, TaggedId, CHARINFO_LEN, TAG_CHARA, TAG_HAT, TAG_OUTFIT};
    use crate::engine::mii::selection::{Action, Active, PlayerLive, PlayerOrigin};
    use crate::engine::mii_root::mii_menu::Context;
    let snapshot = build_snapshot((0..2).map(|i| RawSlot {
        index: i, slot_id: i as u32 + 1, name_len: 1,
        name_units: [65 + i as u16, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        colour: i as u8, voice: 37, specials: [0; 4], charinfo: [0; CHARINFO_LEN],
        tag_outfit: TaggedId(u64::from(TAG_OUTFIT) << 56 | 1),
        tag_hat: TaggedId(u64::from(TAG_HAT) << 56 | 2),
        tag_chara: TaggedId(u64::from(TAG_CHARA) << 56 | MiiClass::Brawler.chara_hash40()),
    }).collect());
    let first = snapshot.configs[0].canonical;
    let context = Context { epoch: 4, original: PlayerOrigin::NotMii,
        active: Active { player: PlayerLive::NotMii, cpu: Some((MiiClass::Brawler, first)) }, cpu_original: Some(first) };
    let mut p = Pipeline::new(settings_with(FOX, 0x48, true));
    let mut menu = p.open();
    let mut list = crate::editors::mii::MiiList::new(snapshot, Some(context), 4, &menu).unwrap();
    // A new saved CPU Brawler preference: on its own, the Mii close reloads the CPU.
    list.open(0);
    list.press(Button::ZR, &mut menu);
    list.press(Button::Down, &mut menu);
    list.press(Button::A, &mut menu);
    assert!(list.close_for(false, close_picked([false; 2], menu.picks())).unwrap().actions.iter()
        .any(|action| matches!(action, Action::ReloadCpu(_))));
    // A repick of the stored CPU Brawler and of P1's stored Fox in the same visit: the picks own both roles, so the Mii close
    // drops the CPU reload (one owner per role), and the frame loads each role once.
    pick(&mut menu, true, 0x48);
    pick(&mut menu, false, FOX);
    let picked = close_picked([false; 2], menu.picks());
    assert_eq!(picked, [true, true]);
    assert!(list.close_for(false, picked).unwrap().actions.is_empty(), "no Mii reload beside the picks");
    p.close(&mut menu);
    p.frame();
    p.frame();
    assert_eq!(p.loads, vec![(0, FOX, true), (1, 0x48, true)]);
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("an_idle_repick_loads_the_stored_fighter_once", an_idle_repick_loads_the_stored_fighter_once as fn()),
        ("a_changed_pick_loads_exactly_once", a_changed_pick_loads_exactly_once as fn()),
        ("a_completion_after_the_publish_keeps_the_pick_for_both_roles", a_completion_after_the_publish_keeps_the_pick_for_both_roles as fn()),
        ("an_overlapping_publication_loads_the_pick_once", an_overlapping_publication_loads_the_pick_once as fn()),
        ("player_swapping_turned_on_with_a_repick_loads_once", player_swapping_turned_on_with_a_repick_loads_once as fn()),
        ("a_repick_survives_a_live_selection_writeback_before_the_close", a_repick_survives_a_live_selection_writeback_before_the_close as fn()),
        ("two_presses_in_one_visit_load_the_last", two_presses_in_one_visit_load_the_last as fn()),
        ("both_roles_in_one_visit_load_once_each", both_roles_in_one_visit_load_once_each as fn()),
        ("a_close_without_a_pick_loads_nothing", a_close_without_a_pick_loads_nothing as fn()),
        ("a_waiting_user_load_is_busy_and_an_automatic_one_is_superseded", a_waiting_user_load_is_busy_and_an_automatic_one_is_superseded as fn()),
        ("a_mii_preference_reload_and_a_repick_have_one_owner", a_mii_preference_reload_and_a_repick_have_one_owner as fn()),
    ]);
}
}
