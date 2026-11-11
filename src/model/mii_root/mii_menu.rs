//! Owned catalog UI. No native pointers, file access, save edits or reload calls.
//! A caller must mount a fresh catalog/context after opening the ordinary App.
use crate::engine::mii::inventory::{Canonical, Finding, MiiClass, Snapshot};
use crate::engine::mii::selection::{self, Action, Active, DropdownRow, PlayerOrigin, Preferences, Role};

use crate::engine::compat::panel_view as macro_panel;

/// Add New opens the game's native Mii editor (the fork's editor, carried; operator 2026-11-05: bring it over). Its Save
/// makes a new native Smash Mii only (`mii_editor_bridge::edit_decide`: a fresh draft through the game's allocator; a
/// saved ID never enters, so no saved or protected original is overwritten). The capability stays one switch: with it
/// false the list hides Add New and the bridge refuses before any native work (`tests/mii_install_boundary.rs`).
pub const NATIVE_EDITOR_AVAILABLE: bool = true;

pub const PAGES: [(&str, &str, MiiClass); 3] = [
    ("mii_brawler_config", "Mii Brawler", MiiClass::Brawler),
    ("mii_sword_config", "Mii Swordfighter", MiiClass::Swordfighter),
    ("mii_gunner_config", "Mii Gunner", MiiClass::Gunner),
];

pub fn class_for_id(id: &str) -> Option<MiiClass> {
    PAGES.iter().find(|page| page.0 == id).map(|page| page.2)
}


/// Supplied once at menu open by the future native owner adapter, never inferred
/// from a name, current class, or selected dropdown. Epoch is comparison-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub epoch: u64,
    pub original: PlayerOrigin,
    pub active: Active,
    /// Captured CPU saved original for numbered-cycle exposure.
    /// None is unresolved. It is never inferred from the player origin,
    /// the live class, or the current dropdown.
    pub cpu_original: Option<Canonical>,
}

/// Exposure for one menu opening, separate from the adapters' protected originals.
/// A failed role cannot suppress a different qualified role or lend it stale context.
pub struct MenuCapture {
    pending: u8,
    open: bool,
    saved_valid: [bool; 2],
}

impl MenuCapture {
    pub const fn closed() -> Self {
        Self { pending: 0, open: false, saved_valid: [false; 2] }
    }

    pub fn begin(&mut self) {
        *self = Self { pending: 3, open: true, saved_valid: [false; 2] };
    }

    pub fn cancel(&mut self) { *self = Self::closed(); }
    pub fn waiting(&self) -> bool { self.open }
    pub fn needs(&self, role: Role) -> bool {
        self.open && self.pending & (1 << usize::from(role == Role::Cpu)) != 0
    }
    pub fn validated(&self, role: Role) -> bool {
        self.saved_valid[usize::from(role == Role::Cpu)]
    }

    pub fn finish(&mut self, role: Role, valid: bool) {
        if self.needs(role) {
            let i = usize::from(role == Role::Cpu);
            self.saved_valid[i] = valid;
            self.pending &= !(1 << i);
        }
    }

    /// `ordinary` comes only from the independent owned ordinary-Player adapter.
    /// It must never replace a protected saved origin, even after failed recapture.
    pub fn take_context(&mut self, context: Context, ordinary: Option<selection::PlayerLive>) -> Option<Context> {
        self.take_context_with_cpu(context,ordinary,false)
    }
    pub fn take_context_with_cpu(&mut self, context: Context,
        ordinary: Option<selection::PlayerLive>, ordinary_cpu:bool) -> Option<Context> {
        let ordinary=ordinary.filter(|_|context.original==PlayerOrigin::NotMii)
            .filter(|live|*live!=selection::PlayerLive::NotMii)
            .map(|live|(PlayerOrigin::NotMii,live));
        self.take_owned_context(context,ordinary,ordinary_cpu.then_some(None))
    }

    /// Unified adapters supply both immutable origin and current live identity after a
    /// fresh menu recapture. Some(None) is a qualified CPU currently on an ordinary fighter.
    /// `cpu_original` is preserved as supplied. Live CPU identity and the player origin
    /// do not replace or invent it.
    pub fn take_owned_context(&mut self, mut context:Context,
        player:Option<(PlayerOrigin,selection::PlayerLive)>,
        cpu:Option<Option<(MiiClass,crate::engine::mii::inventory::Canonical)>>)->Option<Context> {
        if !self.open || self.pending!=0 {return None;}
        self.open=false;
        if !self.saved_valid[0] {
            if matches!(context.original,PlayerOrigin::Mii(_)) {context.original=PlayerOrigin::Unresolved;}
            context.active.player=selection::PlayerLive::NotMii;
        }
        if !self.saved_valid[1] {context.active.cpu=None;}
        if let Some((origin,live))=player {
            context.original=origin;context.active.player=live;
        }
        if let Some(live)=cpu {context.active.cpu=live;}
        (self.saved_valid.iter().any(|v|*v) || player.is_some() || cpu.is_some()).then_some(context)
    }

}

/// Intent only. A consumer must revalidate scene/owner/current configuration and
/// resolve fresh saved values before using lifetime::MiiOverride/native_values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingSelection {
    pub context: Context,
    pub actions: Vec<Action>,
}

#[cfg(feature = "mii-editor-alpha")]
#[derive(Clone)]
pub enum EditorSeed {
    Saved(Box<crate::engine::mii::inventory::MiiConfig>),
    /// Intent only: the scene adapter must obtain actual native default values.
    NativeDefault(MiiClass),
}

#[cfg(feature = "mii-editor-alpha")]
#[derive(Clone)]
pub struct EditorRequest {
    pub seed: EditorSeed,
    pub catalog: std::sync::Arc<Snapshot>,
}

#[derive(Clone)]
pub struct CatalogMenu {
    snapshot: Snapshot,
    context: Context,
    editor_only: bool,
    editor_available: bool,
    before: Preferences,
    class: MiiClass,
    role: Option<Role>,
    list_mode: bool,
    cursor: usize,
    rows: Vec<DropdownRow>,
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    ordinary_cpu:bool,
}

pub struct View {
    pub panel: macro_panel::View,
    pub disabled: Vec<bool>,
}

impl CatalogMenu {
    pub fn new(snapshot: Snapshot, context: Context, preferences: &Preferences) -> Option<Self> {
        (context.epoch != 0).then(|| Self {
            snapshot,
            context,
            editor_only: false,
            editor_available: true,
            before: *preferences,
            class: MiiClass::Brawler,
            role: None,
            list_mode: false,
            cursor: 0,
            rows: Vec::new(),
            #[cfg(feature="mii-ordinary-cpu-alpha")]
            ordinary_cpu:false,
        })
    }

    /// Adapter capability, independent of saved-catalog and role qualification.
    pub fn set_editor_available(&mut self, available: bool) {
        self.editor_available = available;
        if !available { self.cursor = 0; }
    }

    /// A waiting-room editor needs a catalog, but has no qualified fighter-swap
    /// context. Do not grant either role permission by inventing a live original.
    #[cfg(feature = "mii-editor-alpha")]
    pub fn editor_only(snapshot: Snapshot, epoch: u64, preferences: &Preferences) -> Option<Self> {
        let context = Context { epoch, original: PlayerOrigin::Unresolved,
            active: Active { player: selection::PlayerLive::NotMii, cpu: None },
            cpu_original: None };
        Self::new(snapshot, context, preferences).map(|mut menu| {
            menu.editor_only = true;
            menu.cursor = 2;
            menu
        })
    }

    #[cfg(feature="mii-saved-alpha")]
    pub fn alpha_player_cycle(&self, kind: u32)->bool {
        match self.context.original {
            #[cfg(feature="mii-ordinary-player-alpha")]
            PlayerOrigin::Mii(origin) if self.context.active.player!=selection::PlayerLive::NotMii =>
                MiiClass::from_kind(kind).is_some_and(|class|
                    selection::eligible(&self.snapshot,Role::Player,class,self.context.original)
                        .any(|c|c.canonical.appearance.hat==origin.0.appearance.hat)),
            #[cfg(not(feature="mii-ordinary-player-alpha"))]
            PlayerOrigin::Mii(origin) if origin.0.class as u32 == kind =>
                selection::eligible(&self.snapshot,Role::Player,origin.0.class,self.context.original)
                    .any(|c|c.canonical.appearance.hat==origin.0.appearance.hat),
            #[cfg(feature="mii-ordinary-player-alpha")]
            PlayerOrigin::NotMii if self.context.active.player != selection::PlayerLive::NotMii =>
                MiiClass::from_kind(kind).is_some_and(|class|
                    selection::eligible(&self.snapshot,Role::Player,class,self.context.original).next().is_some()),
            _=>false,
        }
    }

    #[cfg(feature="mii-ordinary-cpu-alpha")]
    pub fn qualify_ordinary_cpu_cycle(&mut self, qualified:bool) {self.ordinary_cpu=qualified;}
    /// Class admission for a manual same-class choice. A saved CPU original with no
    /// other configuration of its class remains selectable here.
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    pub fn alpha_cpu_cycle(&self,kind:u32)->bool {
        self.ordinary_cpu && crate::engine::mii::ordinary_cpu::supported_kind(kind)
            && MiiClass::from_kind(kind).is_some_and(|class|
                selection::eligible(&self.snapshot,Role::Cpu,class,self.context.original).next().is_some())
    }
    /// Numbered-cycle choice for one class. If the resolved row is the captured CPU
    /// original (slot 0), choose the first different configuration, or none.
    /// A missing `cpu_original` does not borrow the player origin or invent a row.
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    pub fn cpu_numbered_choice(&self, kind: u32, preferences: &Preferences) -> Option<selection::Choice> {
        if !self.alpha_cpu_cycle(kind) {
            return None;
        }
        let class = MiiClass::from_kind(kind)?;
        let neutral = PlayerOrigin::NotMii;
        let resolved = selection::resolve(&self.snapshot, preferences, Role::Cpu, class, neutral)?;
        if let Some(origin) = self.context.cpu_original.filter(|origin| origin.class == class) {
            if resolved.canonical == origin {
                return selection::eligible(&self.snapshot, Role::Cpu, class, neutral)
                    .find(|cfg| cfg.canonical != origin)
                    .map(|cfg| selection::Choice {
                        canonical: cfg.canonical,
                        slot_id: cfg.slot_id,
                        source: selection::Source::FirstEligible(
                            selection::Fallback::PreferenceIsOrigin,
                        ),
                    });
            }
        }
        Some(resolved)
    }
    /// Grey badge 0 only for the captured CPU configuration when the numbered cycle
    /// has no other configuration of its class.
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    pub fn cpu_unnumbered_original(&self, kind: u32, preferences: &Preferences) -> bool {
        if !self.ordinary_cpu {
            return false;
        }
        let Some(origin) = self.context.cpu_original.filter(|origin| origin.class as u32 == kind) else {
            return false;
        };
        let present = selection::eligible(
            &self.snapshot, Role::Cpu, origin.class, PlayerOrigin::NotMii,
        ).any(|cfg| cfg.canonical == origin);
        present && self.cpu_numbered_choice(kind, preferences).is_none()
    }
    #[cfg(feature="mii-managed-helper-alpha")]
    pub fn alpha_helper_hat(&self,kind:u32)->bool {
        self.ordinary_cpu && self.context.active.player!=selection::PlayerLive::NotMii
            && MiiClass::from_kind(kind).is_some_and(|class|
                selection::eligible(&self.snapshot,Role::Cpu,class,PlayerOrigin::NotMii).next().is_some())
    }
    pub fn open_class(&mut self, class: MiiClass) {
        self.class = class;
        self.list_mode = false;
        self.role = None;
        self.cursor = if self.editor_only && self.editor_available { 2 } else { 0 };
        self.rows.clear();
    }

    /// The approved class list: saved/replay entries and Add New, with one active role.
    pub fn open_list(&mut self, class: MiiClass, preferences: &Preferences) {
        self.open_class(class);
        self.list_mode = true;
        self.set_list_role(Role::Player, preferences);
    }

    fn set_list_role(&mut self, role: Role, preferences: &Preferences) {
        self.role = Some(role);
        self.rows = selection::dropdown(&self.snapshot, preferences, role, self.class, self.context.original);
        self.cursor = if self.editor_only && self.editor_available { self.rows.len() } else {
            self.rows.iter().position(|row| row.selected).unwrap_or(0)
        };
    }

    pub fn switch_list_role(&mut self, preferences: &Preferences) {
        if self.list_mode && !self.editor_only {
            let role = if self.role == Some(Role::Player) { Role::Cpu } else { Role::Player };
            self.set_list_role(role, preferences);
        }
    }

    pub fn step(&mut self, delta: isize) {
        if self.editor_only { return; }
        let count = if self.role.is_some() { self.rows.len() + usize::from(self.list_mode && self.editor_available) } else { 2 + usize::from(self.editor_available) };
        if count != 0 {
            self.cursor = self.cursor.saturating_add_signed(delta).min(count - 1);
        }
    }

    /// Seed is copied by value; selecting Add New never edits a native saved row.
    #[cfg(feature = "mii-editor-alpha")]
    pub fn editor_seed(&self, preferences: &Preferences) -> Option<EditorRequest> {
        let add_new = if self.list_mode { self.cursor == self.rows.len() } else { self.role.is_none() && self.cursor == 2 };
        if !self.editor_available || !add_new || !self.snapshot.is_complete() {
            return None;
        }
        let preferred = selection::resolve(&self.snapshot, preferences, Role::Player,
            self.class, self.context.original).map(|choice| choice.canonical);
        let seed = self.snapshot.of_class(self.class)
            .find(|config| Some(config.canonical) == preferred)
            .or_else(|| self.snapshot.of_class(self.class).next()).cloned()
            .map(|seed| EditorSeed::Saved(Box::new(seed)))
            .unwrap_or(EditorSeed::NativeDefault(self.class));
        Some(EditorRequest { seed, catalog: std::sync::Arc::new(self.snapshot.clone()) })
    }

    pub fn confirm(&mut self, preferences: &mut Preferences) {
        if self.editor_only { return; }
        if let Some(role) = self.role {
            if let Some(row) = self.rows.get(self.cursor).filter(|row| !row.greyed) {
                // Names and save IDs are never the selection identity.
                if let Ok(next) = (*preferences).with(role, self.class, Some(row.canonical)) {
                    *preferences = next;
                    if self.list_mode {
                        self.set_list_role(role, preferences);
                    } else {
                        self.role = None;
                        self.cursor = usize::from(role == Role::Cpu);
                        self.rows.clear();
                    }
                }
            }
        } else if self.cursor < 2 {
            let role = if self.cursor == 0 { Role::Player } else { Role::Cpu };
            self.rows = selection::dropdown(&self.snapshot, preferences, role, self.class, self.context.original);
            self.cursor = self.rows.iter().position(|row| row.selected).unwrap_or(0);
            self.role = Some(role);
        }
        // Add New is dispatched separately through editor_seed.
    }

    /// true means the class page remains open; false returns to Character Extras.
    pub fn back(&mut self) -> bool {
        if self.list_mode { return false; }
        if let Some(role) = self.role.take() {
            self.cursor = usize::from(role == Role::Cpu);
            self.rows.clear();
            true
        } else {
            false
        }
    }

    fn ordinary_player(&self) -> bool {
        cfg!(feature="mii-ordinary-player-alpha")
            && self.context.original == PlayerOrigin::NotMii
            && self.context.active.player != selection::PlayerLive::NotMii
    }

    pub fn close(&self, preferences: &Preferences) -> Option<PendingSelection> {
        if self.editor_only { return None; }
        let actions = selection::on_menu_close(&self.snapshot, &self.before, preferences,
            self.context.original, self.context.active);
        (!actions.is_empty()).then_some(PendingSelection { context: self.context, actions })
    }

    pub fn view(&self, preferences: &Preferences) -> View {
        let title = PAGES.iter().find(|page| page.2 == self.class).unwrap().1;
        let defects = self.snapshot.defects().count();
        let status = match defects {
            count if count != 0 => format!("{count} saved Mii configuration(s) could not be read. Selection is unavailable."),
            _ if self.editor_only && !self.editor_available => "Saved Mii selection is unavailable in this context. B: Back".into(),
            _ if self.editor_only => "Create and test a Mii, including Headgear, Outfit, Color, Voice, and Random. An incoming match keeps that unfinished page. B: Back".into(),
            _ if self.snapshot.findings.iter().any(|finding| matches!(finding, Finding::PresetSource { .. })) =>
                "Imported presets unavailable. Native saved Miis remain selectable. B: Back".into(),
            _ if self.role == Some(Role::Player) && self.context.original == PlayerOrigin::Unresolved =>
                "Player original configuration is not yet available.".into(),
            _ if self.ordinary_player() && self.role != Some(Role::Cpu) =>
                if self.context.active.player == selection::PlayerLive::Origin {
                    "Player choice applies on the next cycle. B: Back".into()
                } else {
                    "The active Player class updates on menu close. Other class choices apply on a later cycle. B: Back".into()
                },
            #[cfg(feature="mii-ordinary-cpu-alpha")]
            _ if self.ordinary_cpu && self.role==Some(Role::Cpu) =>
                if self.context.active.cpu.is_some() {
                    "The active CPU class updates on menu close. Other class choices apply on a later cycle. B: Back".into()
                } else {
                    "CPU choice applies on the next cycle. B: Back".into()
                },
            _ => "Saved configurations are read-only. B: Back".into(),
        };
        if let Some(role) = self.role {
            let page_size = crate::model::panes::ROWS;
            let start = self.cursor / page_size * page_size;
            let visible = self.rows.iter().enumerate().skip(start).take(page_size);
            let mut labels: Vec<String> = visible.clone().map(|(index, row)| {
                let moves: String = row.canonical.appearance.specials.iter()
                    .map(|value| char::from(b'1' + *value)).collect();
                format!("{}. {}{} [{}]", index + 1, if row.selected { "> " } else { "" }, row.name, moves)
            }).collect();
            let mut disabled: Vec<bool> = visible.map(|(_, row)| self.editor_only || row.greyed).collect();
            if self.editor_available && self.list_mode && self.rows.len() >= start && self.rows.len() < start + page_size {
                labels.push("Add New…".into());
                disabled.push(!cfg!(feature = "mii-editor-alpha") || !self.snapshot.is_complete());
            }
            if labels.is_empty() && !self.editor_available {
                labels.push("Unavailable".into());
                disabled.push(true);
            }
            View { panel: macro_panel::View {
                title: format!("{title} — {}", if role == Role::Player { "Player" } else { "CPU" }),
                labels,
                selected: self.cursor - start,
                footer: if self.rows.is_empty() && defects == 0 && !(self.role == Some(Role::Player) && self.context.original == PlayerOrigin::Unresolved) { "No saved configurations in this class. B: Back".into() } else { status },
            }, disabled }
        } else {
            let mut labels = Vec::with_capacity(3);
            for (role, name) in [(Role::Player, "Player"), (Role::Cpu, "CPU")] {
                if self.editor_only {
                    labels.push(format!("{name} configuration (Training only)"));
                    continue;
                }
                let choice = selection::resolve(&self.snapshot, preferences, role, self.class, self.context.original);
                let selected = choice.and_then(|choice| self.snapshot.entries(self.class)
                    .find(|entry| entry.canonical == choice.canonical && entry.slot_id == choice.slot_id))
                    .map(|entry| entry.name);
                labels.push(format!("{name}: {}", selected.unwrap_or("Unavailable")));
            }
            let can_add = cfg!(feature = "mii-editor-alpha") && self.snapshot.is_complete();
            let mut disabled = vec![self.editor_only, self.editor_only];
            if self.editor_available {
                labels.push(if can_add { "Add New" } else { "Add New (unavailable)" }.into());
                disabled.push(!can_add);
            }
            View { panel: macro_panel::View { title: title.into(), labels, selected: self.cursor,
                footer: status }, disabled }
        }
    }
}
