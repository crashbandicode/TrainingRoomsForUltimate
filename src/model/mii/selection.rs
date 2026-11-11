//! Identity-aware selection over an owned catalog; no native actions or persistence writes.
use crate::engine::mii::inventory::{Canonical, CatalogEntry, MiiClass, Snapshot};

/// Character-select tile order.
pub const CLASSES: [MiiClass; 3] = [MiiClass::Brawler, MiiClass::Swordfighter, MiiClass::Gunner];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Player,
    Cpu,
}

/// P1's exact character-select configuration (slot 0), captured once per session by the roster
/// adapter when the entry is first ready. Never changed by this module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Origin(pub Canonical);

/// Unknown is distinct from a known ordinary CSS fighter. The adapter must bind the original
/// once per scene before Player selections are actionable. CPU never excludes P1's original.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerOrigin {
    #[default]
    Unresolved,
    NotMii,
    Mii(Origin),
}

impl PlayerOrigin {
    fn canonical(self) -> Option<Canonical> {
        match self { Self::Mii(origin) => Some(origin.0), _ => None }
    }
}


fn class_index(class: MiiClass) -> usize {
    match class {
        MiiClass::Brawler => 0,
        MiiClass::Swordfighter => 1,
        MiiClass::Gunner => 2,
    }
}

/// Persisted per-role, per-class preferences (identity = `Canonical`, stored by the menu layer as
/// its hex form). This module only reads them; `with` exists for the menu layer and tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "StoredPreferences")]
pub struct Preferences {
    player: [Option<Canonical>; 3],
    cpu: [Option<Canonical>; 3],
}

impl Preferences {
    pub fn get(&self, role: Role, class: MiiClass) -> Option<Canonical> {
        match role {
            Role::Player => self.player[class_index(class)],
            Role::Cpu => self.cpu[class_index(class)],
        }
    }

    pub fn with(mut self, role: Role, class: MiiClass, value: Option<Canonical>) -> Result<Self, String> {
        if let Some(value) = value {
            if value.class != class || Canonical::from_bytes(&value.to_bytes()) != Ok(value) {
                return Err("Mii preference does not match its class or canonical layout".into());
            }
        }
        match role {
            Role::Player => self.player[class_index(class)] = value,
            Role::Cpu => self.cpu[class_index(class)] = value,
        }
        Ok(self)
    }
}

#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
struct StoredPreferences {
    player: [Option<Canonical>; 3],
    cpu: [Option<Canonical>; 3],
}

impl TryFrom<StoredPreferences> for Preferences {
    type Error = String;
    fn try_from(value: StoredPreferences) -> Result<Self, Self::Error> {
        let mut prefs = Self::default();
        for (role, values) in [(Role::Player, value.player), (Role::Cpu, value.cpu)] {
            for (class, value) in CLASSES.into_iter().zip(values) {
                prefs = prefs.with(role, class, value)?;
            }
        }
        Ok(prefs)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
    NoPreference,
    PreferenceIsOrigin,
    PreferenceUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Preference,
    FirstEligible(Fallback),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub canonical: Canonical,
    pub slot_id: Option<u32>,
    pub source: Source,
}

/// Eligible configurations of `class` for `role` in snapshot order: every deduplicated configuration
/// of that class, minus slot 0 for the player. Incomplete catalogs and an unresolved Player origin
/// produce no actionable choice (unresolved origin blocks Player only); the adapter re-evaluates
/// after binding the origin or obtaining a complete catalog.
pub fn eligible<'a>(
    snapshot: &'a Snapshot,
    role: Role,
    class: MiiClass,
    origin: PlayerOrigin,
) -> impl Iterator<Item = CatalogEntry<'a>> + 'a {
    snapshot.entries(class).filter(move |cfg| snapshot.is_complete() && match (role, origin) {
        (Role::Player, PlayerOrigin::Unresolved) => false,
        (Role::Player, PlayerOrigin::Mii(origin)) => cfg.canonical != origin.0,
        _ => true,
    })
}

/// A class tile is selectable when at least one eligible configuration exists. For P1's original
/// class this is exactly "another distinct configuration exists"; copies never count.
pub fn class_selectable(snapshot: &Snapshot, role: Role, class: MiiClass, origin: PlayerOrigin) -> bool {
    eligible(snapshot, role, class, origin).next().is_some()
}

/// Resolve the one configuration a role uses for a class this session. Never mutates `prefs`.
pub fn resolve(
    snapshot: &Snapshot,
    prefs: &Preferences,
    role: Role,
    class: MiiClass,
    origin: PlayerOrigin,
) -> Option<Choice> {
    let fallback = match prefs.get(role, class) {
        None => Fallback::NoPreference,
        Some(wanted) => {
            if let Some(hit) = eligible(snapshot, role, class, origin).find(|cfg| cfg.canonical == wanted) {
                return Some(Choice { canonical: hit.canonical, slot_id: hit.slot_id, source: Source::Preference });
            }
            if origin.canonical().is_some_and(|o| o == wanted) && role == Role::Player {
                Fallback::PreferenceIsOrigin
            } else {
                Fallback::PreferenceUnavailable
            }
        }
    };
    eligible(snapshot, role, class, origin)
        .next()
        .map(|cfg| Choice { canonical: cfg.canonical, slot_id: cfg.slot_id, source: Source::FirstEligible(fallback) })
}

/// One cycle entry per selectable class, in tile order. Slot 0 itself is not an entry; the roster
/// keeps it as the original.
pub fn cycle_entries(snapshot: &Snapshot, prefs: &Preferences, origin: PlayerOrigin) -> Vec<(MiiClass, Choice)> {
    CLASSES
        .iter()
        .filter_map(|class| resolve(snapshot, prefs, Role::Player, *class, origin).map(|choice| (*class, choice)))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropdownRow {
    pub slot_id: Option<u32>,
    pub name: String,
    pub copies: usize,
    pub canonical: Canonical,
    /// Not selectable: P1's slot 0, incomplete catalog, or unresolved Player origin.
    pub greyed: bool,
    /// The configuration this role currently resolves to.
    pub selected: bool,
}

/// Dropdown rows for one class page and role: all deduplicated configurations of the class in
/// snapshot order, with slot 0 greyed for the player and the resolved choice marked.
pub fn dropdown(
    snapshot: &Snapshot,
    prefs: &Preferences,
    role: Role,
    class: MiiClass,
    origin: PlayerOrigin,
) -> Vec<DropdownRow> {
    let chosen = resolve(snapshot, prefs, role, class, origin).map(|c| c.canonical);
    snapshot
        .entries(class)
        .map(|cfg| DropdownRow {
            slot_id: cfg.slot_id,
            name: cfg.name.to_owned(),
            copies: cfg.copies,
            canonical: cfg.canonical,
            greyed: !snapshot.is_complete()
                || (role == Role::Player && (origin == PlayerOrigin::Unresolved
                    || origin.canonical().is_some_and(|o| o == cfg.canonical))),
            selected: chosen == Some(cfg.canonical),
        })
        .collect()
}

/// What is live now, as the roster adapter reports it. `Origin` is P1 at slot 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerLive {
    Origin,
    Cycled { class: MiiClass, canonical: Canonical },
    NotMii,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Active {
    pub player: PlayerLive,
    /// Active CPU Mii configuration, if the CPU currently is a Mii.
    pub cpu: Option<(MiiClass, Canonical)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    ReloadPlayer(Choice),
    ReloadCpu(Choice),
}

/// Menu close: at most one action per role, only for a changed selector of the live class.
/// Unrelated menu edits preserve native CSS choices and current cycled configurations.
/// P1 at slot 0 never produces an action; unresolved or unchanged configurations stay intact.
pub fn on_menu_close(snapshot: &Snapshot, before: &Preferences, prefs: &Preferences, origin: PlayerOrigin, active: Active) -> Vec<Action> {
    let mut actions = Vec::with_capacity(2);
    if let PlayerLive::Cycled { class, canonical } = active.player {
        if let Some(choice) = resolve(snapshot, prefs, Role::Player, class, origin)
            .filter(|_| before.get(Role::Player, class) != prefs.get(Role::Player, class)) {
            if choice.canonical != canonical {
                actions.push(Action::ReloadPlayer(choice));
            }
        }
    }
    if let Some((class, canonical)) = active.cpu {
        if let Some(choice) = resolve(snapshot, prefs, Role::Cpu, class, origin)
            .filter(|_| before.get(Role::Cpu, class) != prefs.get(Role::Cpu, class)) {
            if choice.canonical != canonical {
                actions.push(Action::ReloadCpu(choice));
            }
        }
    }
    actions
}

// ------------------------------------------------------------------------------------------ tests
// Behavioral regression cases adapted from the accepted selection policy.
