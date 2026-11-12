//! The menu worker's view of package 6's owned Mii catalog. Selection, eligibility,
//! identity and native-action policy stay in `CatalogMenu`.

use core::fmt::Write;

use crate::engine::mii::inventory::{MiiClass, Snapshot};
use crate::engine::mii::selection::{Action, Preferences};
use crate::engine::mii_root::mii_menu::{CatalogMenu, Context, PendingSelection};
use crate::model::frame::{page_frame, Frame, Text, Tone, View};
use crate::model::menu::input::Button;
use crate::model::menu::Menu;

pub struct MiiList {
    catalog: CatalogMenu,
    preferences: Preferences,
    opened: Option<u8>,
    epoch: u64,
}

impl MiiList {
    pub fn new(snapshot: Snapshot, context: Option<Context>, epoch: u64, menu: &Menu) -> Option<Self> {
        let preferences = crate::engine::compat::menu::mii_preferences_of(menu.top_extra("mii_preferences"));
        let mut catalog = match context {
            Some(context) if context.epoch == epoch => CatalogMenu::new(snapshot, context, &preferences)?,
            Some(_) => return None,
            #[cfg(feature = "mii-editor-alpha")]
            None => CatalogMenu::editor_only(snapshot, epoch, &preferences)?,
            #[cfg(not(feature = "mii-editor-alpha"))]
            None => return None,
        };
        catalog.set_editor_available(crate::engine::mii_root::mii_menu::NATIVE_EDITOR_AVAILABLE);
        Some(Self { catalog, preferences, opened: None, epoch })
    }

    #[cfg(feature = "mii-ordinary-cpu-alpha")]
    pub fn qualify_cpu(&mut self, qualified: bool) {
        self.catalog.qualify_ordinary_cpu_cycle(qualified);
    }

    pub fn open(&mut self, class: u8) {
        if self.opened == Some(class) { return; }
        let Some(class_kind) = [MiiClass::Brawler, MiiClass::Swordfighter, MiiClass::Gunner].get(usize::from(class)) else { return };
        self.catalog.open_list(*class_kind, &self.preferences);
        self.opened = Some(class);
    }

    pub fn epoch(&self) -> u64 { self.epoch }

    /// The roster grids' Mii tiles for this opening (the fork's `app.rs` 563-685 asks the mounted catalog the same). The
    /// CPU cycle's admission and the slot 0s are the runtime's, set by the worker (`MiiTiles::cpu_cycle`, `slot_zero`).
    pub fn tiles(&self) -> crate::editors::roster::MiiTiles {
        #[cfg(feature = "mii-saved-alpha")]
        let player = [0x48u32, 0x49, 0x4a].map(|kind| self.catalog.alpha_player_cycle(kind));
        #[cfg(not(feature = "mii-saved-alpha"))]
        let player = [false; 3];
        #[cfg(feature = "mii-ordinary-cpu-alpha")]
        let cpu = [0x48u32, 0x49, 0x4a].map(|kind| self.catalog.alpha_cpu_cycle(kind));
        #[cfg(not(feature = "mii-ordinary-cpu-alpha"))]
        let cpu = [false; 3];
        #[cfg(feature = "mii-managed-helper-alpha")]
        let hat = [0x48u32, 0x49, 0x4a].map(|kind| self.catalog.alpha_helper_hat(kind));
        #[cfg(not(feature = "mii-managed-helper-alpha"))]
        let hat = [false; 3];
        crate::editors::roster::MiiTiles { player, cpu, hat, ..Default::default() }
    }

    pub fn leave(&mut self) { self.opened = None; }

    pub fn press(&mut self, button: Button, menu: &mut Menu) {
        match button {
            Button::Up => self.catalog.step(-1),
            Button::Down => self.catalog.step(1),
            Button::L => self.catalog.step(-(crate::model::panes::ROWS as isize)),
            Button::R => self.catalog.step(crate::model::panes::ROWS as isize),
            Button::ZR => self.catalog.switch_list_role(&self.preferences),
            Button::A => {
                self.catalog.confirm(&mut self.preferences);
                // Preferences has a canonical, infallible JSON representation.
                if let Ok(value) = serde_json::to_value(self.preferences) {
                    menu.set_top_extra("mii_preferences", value);
                }
            }
            _ => {}
        }
    }

    #[cfg(feature = "mii-editor-alpha")]
    pub fn editor_request(&self) -> Option<crate::engine::mii_root::mii_menu::EditorRequest> {
        self.catalog.editor_seed(&self.preferences)
    }

    pub fn close(&self) -> Option<PendingSelection> { self.catalog.close(&self.preferences) }

    /// The selection a close hands over (fork `training_mod_tui/src/containers/app.rs`). Add New opens the editor
    /// alone (923-931): the choices made in this opening stay saved in `mii_preferences`, but their reload actions
    /// are dropped, so no Training reload races the editor's quit. A portrait pick drops its role's reload (522-531):
    /// that pick resolves the role's freshly saved choice itself. `picked` is `[Player, CPU]`.
    pub fn close_for(&self, editor: bool, picked: [bool; 2]) -> Option<PendingSelection> {
        if editor { return None; }
        let mut pending = self.close()?;
        pending.actions.retain(|action| match action {
            Action::ReloadPlayer(_) => !picked[0],
            Action::ReloadCpu(_) => !picked[1],
        });
        Some(pending)
    }

    pub fn frame(&self, menu: &Menu, frame: &mut Frame) {
        let view = self.catalog.view(&self.preferences);
        page_frame(menu, frame);
        frame.view = View::List;
        frame.group_count = 0;
        frame.rows.fill(crate::model::frame::RowLine::default());
        frame.row_count = view.panel.labels.len().min(frame.rows.len()) as u8;
        frame.focus = Some(view.panel.selected as u8);
        frame.crumb.clear();
        let _ = write!(frame.crumb, "{} > {}", menu.page().title(), view.panel.title);
        for (i, label) in view.panel.labels.iter().take(frame.rows.len()).enumerate() {
            let _ = frame.rows[i].label.write_str(label);
            if view.disabled.get(i) == Some(&true) { frame.rows[i].label_tone = Tone::Off; }
        }
        frame.help = [Text::new(); 2];
        let _ = frame.help[0].write_str(&view.panel.footer);
        frame.hint.clear();
        let _ = frame.hint.write_str("D-pad move   A select   L/R page   ZR Player/CPU   B back");
    }
}

#[cfg(all(test, feature = "mii-editor-alpha"))]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::{menu::Env, settings::Settings};

    fn saved_choices_keep_role_identity_and_close_actions() {
        use crate::engine::mii::inventory::{build_snapshot, RawSlot, TaggedId, CHARINFO_LEN, TAG_CHARA, TAG_HAT, TAG_OUTFIT};
        use crate::engine::mii::selection::{Active, PlayerLive, PlayerOrigin, Role, Action};
        let snapshot = build_snapshot((0..2).map(|i| RawSlot {
            index: i, slot_id: i as u32 + 1, name_len: 1,
            name_units: [65 + i as u16, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            colour: i as u8, voice: 37, specials: [0; 4], charinfo: [0; CHARINFO_LEN],
            tag_outfit: TaggedId(u64::from(TAG_OUTFIT) << 56 | 1),
            tag_hat: TaggedId(u64::from(TAG_HAT) << 56 | 2),
            tag_chara: TaggedId(u64::from(TAG_CHARA) << 56 | MiiClass::Brawler.chara_hash40()),
        }).collect());
        let first = snapshot.configs[0].canonical;
        let second = snapshot.configs[1].canonical;
        let context = Context { epoch: 4, original: PlayerOrigin::NotMii,
            active: Active { player: PlayerLive::NotMii, cpu: Some((MiiClass::Brawler, first)) }, cpu_original: Some(first) };
        let mut menu = Menu::new(Settings::factory(), Env::default());
        let mut list = MiiList::new(snapshot, Some(context), 4, &menu).unwrap();
        list.open(0);
        list.press(Button::ZR, &mut menu);
        list.press(Button::Down, &mut menu);
        list.press(Button::A, &mut menu);
        let prefs: Preferences = serde_json::from_value(menu.top_extra("mii_preferences").unwrap().clone()).unwrap();
        assert_eq!(prefs.get(Role::Cpu, MiiClass::Brawler), Some(second));
        assert_eq!(prefs.get(Role::Player, MiiClass::Brawler), None);
        let pending = list.close().unwrap();
        assert_eq!(pending.actions.len(), 1);
        assert!(matches!(pending.actions[0], Action::ReloadCpu(choice) if choice.canonical == second));
        list.press(Button::Down, &mut menu);
        assert!(list.editor_request().is_some(), "Add New follows the two saved choices");
        assert!(list.close_for(true, [false; 2]).is_none(), "Add New drops this opening's reload actions");
        assert!(matches!(list.close_for(false, [false; 2]).unwrap().actions[0], Action::ReloadCpu(choice) if choice.canonical == second));
        assert!(list.close_for(false, [false, true]).unwrap().actions.is_empty(), "a CPU portrait pick drops the CPU reload");
        assert_eq!(list.close_for(false, [true, false]).unwrap().actions.len(), 1, "a Player pick keeps the CPU reload");
        let prefs: Preferences = serde_json::from_value(menu.top_extra("mii_preferences").unwrap().clone()).unwrap();
        assert_eq!(prefs.get(Role::Cpu, MiiClass::Brawler), Some(second), "the choice itself stays saved");
        let mut frame = Frame::new();
        list.frame(&menu, &mut frame);
        assert!(frame.rows[..usize::from(frame.row_count)].iter().any(|row| row.label.string_lossy() == "Add New…"));
    }

    fn waiting_catalog_opens_each_class_at_add_new_without_role_permissions() {
        use crate::engine::mii_root::mii_menu::EditorSeed;
        let mut menu = Menu::new(Settings::factory(), Env::default());
        let mut list = MiiList::new(Snapshot::default(), None, 1, &menu).unwrap();
        for class in 0..3 {
            list.open(class);
            let mut frame = Frame::new();
            list.frame(&menu, &mut frame);
            assert_eq!(frame.row_count, 1);
            assert_eq!(frame.rows[0].label.string_lossy(), "Add New…");
            let request = list.editor_request().unwrap();
            assert!(matches!(request.seed, EditorSeed::NativeDefault(kind) if kind.kind() == 0x48 + u32::from(class)));
            list.press(Button::ZR, &mut menu);
            assert!(list.close().is_none());
        }
    }

    fn unavailable_capability_refuses_editor_without_granting_role_permissions() {
        let mut menu = Menu::new(Settings::factory(), Env::default());
        let mut list = MiiList::new(Snapshot::default(), None, 1, &menu).unwrap();
        list.catalog.set_editor_available(false);
        for class in 0..3 {
            list.open(class);
            let mut frame = Frame::new();
            list.frame(&menu, &mut frame);
            assert_eq!(frame.row_count, 1);
            assert_eq!(frame.rows[0].label.string_lossy(), "Unavailable");
            assert_eq!(frame.rows[0].label_tone, Tone::Off);
            assert!(list.editor_request().is_none());
            list.press(Button::A, &mut menu);
            assert!(list.editor_request().is_none());
            list.press(Button::ZR, &mut menu);
            assert!(list.close().is_none());
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("saved_choices_keep_role_identity_and_close_actions", saved_choices_keep_role_identity_and_close_actions as fn()),
            ("waiting_catalog_opens_each_class_at_add_new_without_role_permissions", waiting_catalog_opens_each_class_at_add_new_without_role_permissions as fn()),
            ("unavailable_capability_refuses_editor_without_granting_role_permissions", unavailable_capability_refuses_editor_without_granting_role_permissions as fn()),
        ]);
    }
    }
}
