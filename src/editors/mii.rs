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

