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

#[cfg(all(test, not(target_os = "switch")))]
#[path = "../../../tests/support/mii_cycle_compact.rs"]
pub(crate) mod compact_regressions;

#[cfg(all(test, feature = "mii-saved-alpha"))]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::mii::inventory::{build_snapshot, RawSlot, TaggedId, CHARINFO_LEN};
    

    fn partner_refresh_preserves_the_slot_before_the_next_mii() {
        let catalog=example();
        let ordered=roster(&[0x4b,0x48,6]);
        let preferences=Preferences::default();
        let refreshed=refresh_partners(0x4b,0x4b,Some(0),true).unwrap();
        assert_eq!(refreshed,Step {target:Target::Fighter(0x4b),cursor:Some(0)});
        let next=next_qualified_player_with_ice_climbers(&ordered,&catalog,&preferences,
            Identity::Fighter(0x4b),applied(refreshed),true,Some).unwrap().0;
        assert!(matches!(next.target,Target::Saved(choice) if choice.canonical.class==MiiClass::Brawler));
        assert_eq!(next.cursor,Some(1));
        assert_eq!(refresh_partners(0x4b,0x4b,None,true),Some(Step {target:Target::Original,cursor:None}));
        assert_eq!(refresh_partners(0x4b,9,Some(4),true),Some(Step {target:Target::Fighter(0x4b),cursor:Some(4)}));
        assert_eq!(refresh_partners(0x4b,0x4b,None,false),Some(Step {target:Target::Fighter(0x4b),cursor:None}));
        assert!(refresh_partners(0x48,0x4b,Some(1),true).is_none());
        assert_ne!(crate::engine::mii::ordinary_player::Request::REFRESH_PARTNERS,crate::engine::mii::ordinary_player::Request::CYCLE);
        assert_ne!(crate::engine::mii::ordinary_cpu::Request::REFRESH_PARTNERS,crate::engine::mii::ordinary_cpu::Request::CYCLE);
        assert!(crate::engine::mii::ordinary_player::Request::REFRESH_PARTNERS.refresh_partners());
        assert!(crate::engine::mii::ordinary_cpu::Request::REFRESH_PARTNERS.refresh_partners());
    }

    fn raw(index: usize, id: u32, class: MiiClass, face: u8) -> RawSlot {
        let mut charinfo = [face; CHARINFO_LEN];
        charinfo[..0x10].fill(index as u8);
        charinfo[0x10..0x24].fill(b'N');
        charinfo[0x24..0x26].fill(0);
        RawSlot {
            index,
            slot_id: id,
            name_len: 1,
            name_units: [u16::from(face), 0, 0, 0, 0, 0, 0, 0, 0, 0],
            colour: 0,
            voice: 37,
            specials: [0, 0, 0, 0],
            tag_outfit: TaggedId(0x1300011bfef7a5f7),
            tag_hat: TaggedId(0x5a00000fa9efa5ea),
            tag_chara: TaggedId(match class {
                MiiClass::Brawler => 0xc1004c13fdfdfb2b,
                MiiClass::Swordfighter => 0xc1004d15d7061f77,
                MiiClass::Gunner => 0xc1004e1257ab1d87,
            }),
            charinfo,
        }
    }

    fn example() -> Snapshot {
        build_snapshot(vec![
            raw(0, 2622, MiiClass::Swordfighter, b'a'),
            raw(1, 2700, MiiClass::Swordfighter, b'b'),
            raw(2, 4155, MiiClass::Brawler, b'c'),
            raw(3, 4200, MiiClass::Brawler, b'd'),
            raw(4, 4300, MiiClass::Brawler, b'e'),
            raw(5, 3137, MiiClass::Gunner, b'f'),
        ])
    }

    fn direct_mixed_roster_requests_keep_role_identity_and_continue_the_cycle() {
        let catalog = example();
        let preferences = Preferences::default()
            .with(Role::Player, MiiClass::Brawler, Some(id(&catalog, 4200))).unwrap()
            .with(Role::Cpu, MiiClass::Brawler, Some(id(&catalog, 4300))).unwrap();
        let before = preferences;
        let original = Identity::Fighter(0);
        let ordered = roster(&[6, 0x48, 8]);
        let player_request = crate::engine::mii::ordinary_player::Request::direct(7, 0x48).unwrap();
        let cpu_request = crate::engine::mii::ordinary_cpu::Request::direct(7, 0x48).unwrap();
        assert!(player_request.current(7, None));
        assert!(cpu_request.current(7, None));
        assert!(!player_request.current(8, None));
        assert!(!cpu_request.current(8, None));
        assert_eq!(player_request.choice(), None);
        assert_eq!(cpu_request.choice(), None);
        for (role, kind, expected_id) in [
            (Role::Player, player_request.direct_kind().unwrap(), 4200),
            (Role::Cpu, cpu_request.direct_kind().unwrap(), 4300),
        ] {
            let picked = direct(kind, &catalog, &preferences, role, original).unwrap();
            assert_eq!(saved(picked).canonical, id(&catalog, expected_id));
            assert_eq!(picked.cursor, None);
            let next = next_cycle(&ordered, &catalog, &preferences, role, original, applied(picked)).unwrap();
            assert_eq!(next.target, Target::Fighter(8));
            assert_eq!(next.cursor, Some(2));
            let wrap = next_cycle(&ordered, &catalog, &preferences, role, original, applied(next)).unwrap();
            // Both roles restore their captured slot 0 between numbered loops.
            assert_eq!(wrap, Step { target: Target::Original, cursor: None });
            let restart = next_cycle(&ordered, &catalog, &preferences, role, original, applied(wrap)).unwrap();
            assert_eq!(restart, Step { target: Target::Fighter(6), cursor: Some(0) });
        }
        // Manual ordinary swaps remain available with a Mii in the cycle and
        // retain the requested group's starting form without adding a cycle slot.
        for kind in [6, 0x25, 0x5b] {
            assert_eq!(direct(kind, &catalog, &preferences, Role::Player, original),
                Some(Step { target: Target::Fighter(kind), cursor: None }));
        }
        assert_eq!(preferences, before);
        assert_eq!(ordered, roster(&[6, 0x48, 8]));
    }

    fn id(catalog: &Snapshot, slot: u32) -> Canonical {
        catalog
            .configs
            .iter()
            .find(|c| c.slot_id == slot)
            .unwrap()
            .canonical
    }

    fn roster(kinds: &[u32]) -> Vec<TrioFighter> {
        kinds
            .iter()
            .map(|kind| TrioFighter::from_kind(*kind).unwrap())
            .collect()
    }

    fn applied(step: Step) -> Live {
        match step.target {
            Target::Original => Live::Original,
            Target::Fighter(kind) => Live::Numbered {
                identity: Identity::Fighter(kind),
                cursor: step.cursor,
            },
            Target::Saved(choice) => Live::Numbered {
                identity: Identity::Saved(choice.canonical),
                cursor: step.cursor,
            },
        }
    }

    fn saved(step: Step) -> Choice {
        match step.target {
            Target::Saved(choice) => choice,
            other => panic!("expected saved choice, got {:?}", other),
        }
    }

    fn saved_slot_zero_restores_captured_payload_after_an_ordinary_numbered_target() {
        use {crate::engine::mii::lifetime::Configuration, crate::engine::mii::ordinary_player::{retained_record,staged_descriptor}};
        let catalog=example();
        let config=|slot|Configuration::from_saved(catalog.configs.iter().find(|c|c.slot_id==slot).unwrap()).unwrap();
        let origin=config(2622);let alternate=config(2700);
        let order=roster(&[0x49,0x0b]);let preferences=Preferences::default();
        let first=next_cycle(&order,&catalog,&preferences,Role::Player,
            Identity::Saved(origin.canonical()),Live::Original).unwrap();
        assert_eq!(first.target.publish(Some(origin),Some(alternate)),Ok(Some(alternate)));
        assert!(first.target.publish(Some(origin),Some(origin)).is_err());
        let second=next_cycle(&order,&catalog,&preferences,Role::Player,
            Identity::Saved(origin.canonical()),applied(first)).unwrap();
        assert_eq!(second.target,Target::Fighter(0x0b));
        assert_eq!(second.target.publish(Some(origin),None),Ok(None));
        let returned=next_cycle(&order,&catalog,&preferences,Role::Player,
            Identity::Saved(origin.canonical()),applied(second)).unwrap();
        assert_eq!(returned,Step{target:Target::Original,cursor:None});
        let payload=returned.target.publish(Some(origin),None).unwrap();
        assert_eq!(payload,Some(origin));
        let after_alternate=retained_record([0x7d;0x68],Some(alternate));
        let after_fighter=retained_record(after_alternate,None);
        let after_return=retained_record(after_fighter,payload);
        assert_eq!(&after_return[..88],origin.charinfo());
        assert_eq!(&after_return[88..],&after_alternate[88..]);
        assert_ne!(&after_fighter[..88],origin.charinfo());
        let descriptor=staged_descriptor([0xa5;0x70],Some(origin));
        assert_eq!(staged_descriptor(descriptor,None),descriptor);
        // The same class alternate also returns through the explicit original position.
        let same_class=next_cycle(&roster(&[0x49]),&catalog,&preferences,Role::Player,
            Identity::Saved(origin.canonical()),applied(first)).unwrap();
        assert_eq!(same_class,returned);
        assert_eq!(same_class.target.publish(Some(origin),None),Ok(Some(origin)));
    }


    fn cpu_qualification_returns_slot_zero_between_loops() {
        let catalog = example();
        let prefs = Preferences::default();
        let prefs_before = serde_json::to_vec(&prefs).unwrap();
        let order = roster(&[0x48, 0x49, 0x4a, 9]);
        let origin = Identity::Fighter(9);
        let live = Live::Original;
        let qualify = |step: Step| match step.target {
            Target::Saved(choice) if choice.canonical.class != MiiClass::Gunner => None,
            _ => Some(()),
        };
        let (first, ()) = next_qualified_cpu(&order, &catalog, &prefs, origin, live, qualify).unwrap();
        assert_eq!(saved(first).slot_id.unwrap(), 3137);
        assert_eq!(first.cursor, Some(2));
        // The policy returns a proposal; the caller's actual live value stays original.
        assert_eq!(live, Live::Original);
        let (returned, ()) = next_qualified_cpu(&order, &catalog, &prefs, origin,
            applied(first), qualify).unwrap();
        assert_eq!(returned, Step {target: Target::Original, cursor: None});
        assert_ne!(returned.target, Target::Fighter(9));
        let (again, ()) = next_qualified_cpu(&order, &catalog, &prefs, origin,
            Live::Original, qualify).unwrap();
        assert_eq!(again, first);
        assert_eq!(serde_json::to_vec(&prefs).unwrap(), prefs_before);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("partner_refresh_preserves_the_slot_before_the_next_mii", partner_refresh_preserves_the_slot_before_the_next_mii as fn()),
            ("direct_mixed_roster_requests_keep_role_identity_and_continue_the_cycle", direct_mixed_roster_requests_keep_role_identity_and_continue_the_cycle as fn()),
            ("saved_slot_zero_restores_captured_payload_after_an_ordinary_numbered_target", saved_slot_zero_restores_captured_payload_after_an_ordinary_numbered_target as fn()),
            ("cpu_qualification_returns_slot_zero_between_loops", cpu_qualification_returns_slot_zero_between_loops as fn()),
        ]);
    }
    }
}
