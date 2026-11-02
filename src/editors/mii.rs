//! The menu worker's view of package 6's owned Mii catalog. Selection, eligibility,
//! identity and native-action policy stay in `CatalogMenu`.

use core::fmt::Write;

use crate::engine::mii::inventory::{MiiClass, Snapshot};
use crate::engine::mii::selection::Preferences;
use crate::engine::mii_root::mii_menu::{CatalogMenu, Context, PendingSelection};
use crate::model::frame::{page_frame, Frame, Text, Tone, View};
use crate::model::menu::input::Button;
use crate::model::menu::Menu;

pub struct MiiList {
    catalog: CatalogMenu,
    preferences: Preferences,
    opened: Option<u8>,
}

impl MiiList {
    pub fn new(snapshot: Snapshot, context: Option<Context>, epoch: u64, menu: &Menu) -> Option<Self> {
        let preferences = menu.top_extra("mii_preferences").cloned()
            .and_then(|value| serde_json::from_value(value).ok()).unwrap_or_default();
        let catalog = match context {
            Some(context) if context.epoch == epoch => CatalogMenu::new(snapshot, context, &preferences)?,
            Some(_) => return None,
            #[cfg(feature = "mii-editor-alpha")]
            None => CatalogMenu::editor_only(snapshot, epoch, &preferences)?,
            #[cfg(not(feature = "mii-editor-alpha"))]
            None => return None,
        };
        Some(Self { catalog, preferences, opened: None })
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
                if let Ok(value) = serde_json::to_value(&self.preferences) {
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
    use super::*;
    use crate::engine::mii_root::mii_menu::EditorSeed;
    use crate::model::{menu::Env, settings::Settings};

    #[test]
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
    }

    #[test]
    fn waiting_catalog_opens_each_class_at_add_new_without_role_permissions() {
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
}
