//! Ordered mixed-roster selection only. No native admission, writes or persistence.
//! An original is an explicit position: a saved original and its numbered same-class
//! alternative must never be equated merely because their fighter kinds match.

use {crate::engine::swap::character_forms::Forms, crate::engine::swap::trio::TrioFighter};
use {crate::engine::mii::inventory::{Canonical, MiiClass, Snapshot}, crate::engine::mii::selection::{self, Choice, Fallback, Origin, PlayerOrigin, Preferences, Role, Source}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Identity {
    /// Actual non-Mii kind, including a live transformed member of a native group.
    Fighter(u32),
    Saved(Canonical),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Live {
    /// The caller has qualified its immutable native original, not just its kind.
    Original,
    Numbered {
        identity: Identity,
        /// Zero-based position in the last resolved cycle; only a hint after edits.
        cursor: Option<usize>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// Restore the caller's immutable original, including its saved configuration.
    Original,
    Fighter(u32),
    Saved(Choice),
}

impl Target {
    /// The committed cycle position and the bytes published are independent. Slot0
    /// restores the captured configuration even when the last numbered target was ordinary.
    pub fn publish(self, origin:Option<crate::engine::mii::lifetime::Configuration>,
        selected:Option<crate::engine::mii::lifetime::Configuration>)
        ->Result<Option<crate::engine::mii::lifetime::Configuration>,&'static str> {
        match self {
            Self::Original=>Ok(origin),
            Self::Fighter(_)=>Ok(None),
            Self::Saved(choice)=>selected.filter(|c|c.canonical()==choice.canonical)
                .map(Some).ok_or("selected configuration differs from committed target"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub target: Target,
    /// None is that role's slot 0; Some(0) is the first numbered entry.
    pub cursor: Option<usize>,
}

/// Partner refresh retains the current slot; it is never a cycle or saved-Mii choice.
/// Player slot 0 keeps its original setting, while CPU uses its shared Solo setting.
pub fn refresh_partners(kind:u32, original_kind:u32, cursor:Option<usize>, player:bool)->Option<Step> {
    (kind==0x4b).then_some(Step {cursor,target:
        if player && original_kind==0x4b && cursor.is_none() {Target::Original}
        else {Target::Fighter(kind)}})
}

/// A manual pick has no numbered cursor and never edits/traverses the cycle.
/// Mii class tiles use the same per-role saved identity resolver as cycling;
/// the protected Player original remains the separate slot-0 cycle target.
pub fn direct(
    kind: u32,
    catalog: &Snapshot,
    preferences: &Preferences,
    role: Role,
    original: Identity,
) -> Option<Step> {
    TrioFighter::from_kind(kind)?;
    let player_origin = match original {
        Identity::Fighter(kind) if MiiClass::from_kind(kind).is_some() => return None,
        Identity::Fighter(_) => PlayerOrigin::NotMii,
        Identity::Saved(canonical) => PlayerOrigin::Mii(Origin(canonical)),
    };
    let target = if let Some(class) = MiiClass::from_kind(kind) {
        Target::Saved(selection::resolve(catalog, preferences, role, class, player_origin)?)
    } else {
        // Preserve the explicitly requested starting form, even in the original group.
        Target::Fighter(kind)
    };
    Some(Step { target, cursor: None })
}

/// One numbered configuration for a class. The role's exact saved original is
/// slot 0, so a different configuration of that class is the numbered choice.
fn numbered_saved(
    catalog: &Snapshot,
    preferences: &Preferences,
    role: Role,
    class: MiiClass,
    player_origin: PlayerOrigin,
    original: Identity,
) -> Option<Choice> {
    let resolved = selection::resolve(catalog, preferences, role, class, player_origin)?;
    if let Identity::Saved(canonical) = original {
        if resolved.canonical == canonical {
            return selection::eligible(catalog, role, class, player_origin)
                .find(|cfg| cfg.canonical != canonical)
                .map(|cfg| Choice {
                    canonical: cfg.canonical,
                    slot_id: cfg.slot_id,
                    source: Source::FirstEligible(Fallback::PreferenceIsOrigin),
                });
        }
    }
    Some(resolved)
}

/// The ordered roster keeps every fighter position, but saved payloads exist
/// only for the three real Mii classes, already deduplicated by the traversal.
struct CycleChoices {
    kinds: crate::engine::compat::fixed::FixedList<u32, { crate::engine::swap::trio::CHOICES.len() }>,
    saved: [Option<Choice>; 3],
}
impl CycleChoices {
    fn new() -> Self {
        Self { kinds: crate::engine::compat::fixed::FixedList::new(0), saved: [None; 3] }
    }
    fn class_index(class: MiiClass) -> usize {
        (class.kind() - MiiClass::Brawler.kind()) as usize
    }
    fn has_class(&self, class: MiiClass) -> bool {
        self.saved[Self::class_index(class)].is_some()
    }
    fn has_group(&self, forms: Forms) -> bool {
        self.kinds.iter().any(|kind| MiiClass::from_kind(*kind).is_none() && forms.same_group(*kind))
    }
    fn push_fighter(&mut self, kind: u32) { self.kinds.push(kind); }
    fn push_saved(&mut self, choice: Choice) {
        let class = choice.canonical.class;
        if self.kinds.push(class.kind()) {
            self.saved[Self::class_index(class)] = Some(choice);
        }
    }
    fn matches_kind(&self, kind: u32, live: Identity) -> bool {
        match (MiiClass::from_kind(kind), live) {
            (None, Identity::Fighter(current)) => Forms::starting(kind).same_group(current),
            (Some(class), Identity::Saved(current)) => self.saved[Self::class_index(class)]
                .is_some_and(|choice| choice.canonical == current),
            _ => false,
        }
    }
    fn matches_at(&self, index: usize, live: Identity) -> bool {
        self.kinds.get(index).is_some_and(|kind| self.matches_kind(*kind, live))
    }
    fn position(&self, live: Identity) -> Option<usize> {
        self.kinds.iter().position(|kind| self.matches_kind(*kind, live))
    }
    /// Materialize only the selected result, never a roster-sized Target array.
    fn get(&self, index: usize) -> Option<Target> {
        let kind = *self.kinds.get(index)?;
        match MiiClass::from_kind(kind) {
            Some(class) => self.saved[Self::class_index(class)].map(Target::Saved),
            None => Some(Target::Fighter(kind)),
        }
    }
    fn first(&self) -> Option<Target> { self.get(0) }
}

/// Resolve one ordered step using the existing catalog and per-role selectors.
/// `ordered` is the existing roster's `fighters()` order, before original exclusion.
/// The caller must supply a qualified exact original/live identity and commit the
/// returned cursor only after native completion. A result grants no native authority.
///
/// Each role's slot 0 is outside the numbered order and is the return between
/// loops. A kind-only Mii or a non-catalog sentinel is not an original.
/// Missing/ineligible saved choices are skipped without rewriting preferences.
pub fn next_cycle(
    ordered: &[TrioFighter],
    catalog: &Snapshot,
    preferences: &Preferences,
    role: Role,
    original: Identity,
    live: Live,
) -> Option<Step> {
    next_cycle_with_ice_climbers(ordered, catalog, preferences, role, original, live, false)
}

/// The exception is limited to Ice Climbers with different original/cycle Solo
/// preferences. Saved-Mii identity and every other group keep their existing rule.
fn next_cycle_with_ice_climbers(
    ordered: &[TrioFighter], catalog: &Snapshot, preferences: &Preferences,
    role: Role, original: Identity, live: Live, separate_ice_climbers: bool,
) -> Option<Step> {
    // A kind-only Mii cannot establish a configuration. A non-catalog sentinel
    // is not slot 0 and must not be replaced with Mii Swordfighter.
    if matches!(original, Identity::Fighter(kind) if MiiClass::from_kind(kind).is_some())
        || matches!(live, Live::Numbered { identity: Identity::Fighter(kind), .. }
            if MiiClass::from_kind(kind).is_some())
    {
        return None;
    }
    let slot_zero = match original {
        Identity::Saved(_) => true,
        Identity::Fighter(kind) => TrioFighter::from_kind(kind).is_some(),
    };
    let player_origin = match original {
        Identity::Fighter(_) => PlayerOrigin::NotMii,
        Identity::Saved(canonical) => PlayerOrigin::Mii(Origin(canonical)),
    };
    let mut choices = CycleChoices::new();
    for fighter in ordered {
        let kind = fighter.kind();
        if let Some(class) = MiiClass::from_kind(kind) {
            if choices.has_class(class) {
                continue;
            }
            if let Some(choice) = numbered_saved(
                catalog, preferences, role, class, player_origin, original,
            ) {
                choices.push_saved(choice);
            }
        } else {
            let forms = Forms::starting(kind);
            if (matches!(original, Identity::Fighter(current) if forms.same_group(current))
                && !(separate_ice_climbers && kind == 0x4b))
                || choices.has_group(forms)
            {
                continue;
            }
            choices.push_fighter(kind);
        }
    }
    let position = match live {
        Live::Original => None,
        Live::Numbered { identity, cursor } => cursor
            .filter(|i| choices.matches_at(*i, identity))
            .or_else(|| choices.position(identity)),
    };
    if let Some(index) = position {
        if let Some(target) = choices.get(index + 1) {
            return Some(Step {
                target,
                cursor: Some(index + 1),
            });
        }
        if slot_zero {
            return Some(Step {
                target: Target::Original,
                cursor: None,
            });
        }
    }
    choices.first().map(|target| Step {
        target,
        cursor: Some(0),
    }).or_else(|| {
        slot_zero.then_some(Step {
            target: Target::Original,
            cursor: None,
        })
    })
}


/// CPU-only preflight filtering of one numbered traversal. Rejected choices move
/// only this local lookup cursor: no live cursor or preference is changed here.
/// The caller supplies read-only target qualification, never INSTALL/publication.
pub fn next_qualified_cpu<T>(
    ordered: &[TrioFighter],
    catalog: &Snapshot,
    preferences: &Preferences,
    original: Identity,
    live: Live,
    qualify: impl FnMut(Step) -> Option<T>,
) -> Option<(Step, T)> {
    let walk = Walk { ordered, catalog, preferences, role: Role::Cpu, original, separate_ice_climbers: false };
    next_qualified(walk, live, qualify)
}

/// Player traversal includes the protected original as its final, unnumbered
/// position. An unavailable numbered choice may be skipped before native work;
/// failure to qualify the original stops rather than wrapping past slot 0.
pub fn next_qualified_player<T>(
    ordered: &[TrioFighter],
    catalog: &Snapshot,
    preferences: &Preferences,
    original: Identity,
    live: Live,
    qualify: impl FnMut(Step) -> Option<T>,
) -> Option<(Step, T)> {
    let walk = Walk { ordered, catalog, preferences, role: Role::Player, original, separate_ice_climbers: false };
    next_qualified(walk, live, qualify)
}

/// Player traversal with the same opposite-Solo exception as the portrait grid.
pub fn next_qualified_player_with_ice_climbers<T>(
    ordered: &[TrioFighter], catalog: &Snapshot, preferences: &Preferences,
    original: Identity, live: Live, separate_ice_climbers: bool,
    qualify: impl FnMut(Step) -> Option<T>,
) -> Option<(Step, T)> {
    let walk = Walk { ordered, catalog, preferences, role: Role::Player, original, separate_ice_climbers };
    next_qualified(walk, live, qualify)
}

/// The fixed inputs of one qualified traversal: everything but the moving lookup cursor and the
/// caller's qualification.
struct Walk<'a> {
    ordered: &'a [TrioFighter],
    catalog: &'a Snapshot,
    preferences: &'a Preferences,
    role: Role,
    original: Identity,
    separate_ice_climbers: bool,
}

fn next_qualified<T>(
    walk: Walk<'_>,
    live: Live,
    mut qualify: impl FnMut(Step) -> Option<T>,
) -> Option<(Step, T)> {
    let Walk { ordered, catalog, preferences, role, original, separate_ice_climbers } = walk;
    let mut lookup = live;
    let mut first = None;
    // Numbered choices cannot outnumber roster entries. Slot 0 is one more step.
    let limit = ordered.len().saturating_add(1);
    for _ in 0..limit {
        let step = next_cycle_with_ice_climbers(ordered, catalog, preferences, role,
            original, lookup, separate_ice_climbers)?;
        if first == Some(step) {
            break;
        }
        first.get_or_insert(step);
        if let Some(value) = qualify(step) {
            return Some((step, value));
        }
        if step.target == Target::Original {
            return None;
        }
        lookup = match step.target {
            Target::Original => Live::Original,
            Target::Fighter(kind) => Live::Numbered {
                identity: Identity::Fighter(kind), cursor: step.cursor,
            },
            Target::Saved(choice) => Live::Numbered {
                identity: Identity::Saved(choice.canonical), cursor: step.cursor,
            },
        };
    }
    None
}
