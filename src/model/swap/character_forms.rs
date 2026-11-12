//! Native entry descriptors and form indices. Values only; callers own entry lifetime.

/// Ready visible body; inactive transformation alternatives use state 3.
pub const ACTIVE_BODY_STATE: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Forms {
    pub count: u32,
    pub kinds: [u32; 3],
    pub actor_count: u32,
    pub flags: u8,
}

impl Forms {
    /// Matches the native descriptor expansion. The caller admits catalog kinds.
    pub fn starting(kind: u32) -> Self {
        let (count, kinds, flags) = match kind {
            0x24 => (3, [0x24, 0x25, 0x26], 0x05),
            0x25 => (3, [0x25, 0x26, 0x24], 0x05),
            0x26 => (3, [0x26, 0x24, 0x25], 0x05),
            0x5a => (2, [0x5a, 0x5b, u32::MAX], 0x64),
            0x5b => (2, [0x5b, 0x5a, u32::MAX], 0x64),
            0x4b => (2, [0x4b, 0x4c, u32::MAX], 0x02),
            _ => (1, [kind, u32::MAX, u32::MAX], 0),
        };
        Self {
            count,
            kinds,
            actor_count: if kind == 0x4b { 2 } else { 1 },
            flags,
        }
    }

    pub fn solo_popo() -> Self {
        Self {
            count: 1,
            kinds: [0x4b, u32::MAX, u32::MAX],
            actor_count: 1,
            flags: 0,
        }
    }

    /// Simultaneous partners stay active; transformation alternatives sleep.
    pub fn body_state(self, index: usize, active: u8) -> u8 {
        if self.actor_count == self.count || index == usize::from(active) {
            ACTIVE_BODY_STATE
        } else {
            3
        }
    }

    pub fn index_of(self, kind: u32) -> Option<u8> {
        self.kinds[..self.count as usize]
            .iter()
            .position(|k| *k == kind)
            .map(|i| i as u8)
    }

    pub fn same_group(self, kind: u32) -> bool {
        self.index_of(kind).is_some()
    }
}

/// Native resource members belong to the requested source's canonical group.
/// Partners need not be separate selectable choices; the request still must be.
pub fn is_source_member(requested: u32, member: u32) -> bool {
    if (0x48..=0x4a).contains(&requested) {
        // A Mii source has exactly its own class member. Generic archive,
        // typed-param and copy-file readiness remain mandatory at the caller.
        return cfg!(feature = "mii-managed-helper-alpha") && requested == member;
    }
    crate::engine::swap::trio::TrioFighter::from_kind(requested).is_some()
        && Forms::starting(requested).same_group(member)
}

/// FighterParamAccessor2 record key for a validated native source member.
/// Native acquire (main+607b34) and release (main+720ea4) map Nana to Popo.
/// This does not change descriptor, body, NRO, archive or copy-resource kinds.
pub fn typed_record_kind(requested: u32, member: u32) -> Option<u32> {
    if !is_source_member(requested, member) {
        return None;
    }
    Some(if member == 0x4c { 0x4b } else { member })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryState {
    pub count: u32,
    pub kinds: [u32; 3],
    pub actor_count: u32,
    pub installed: u64,
    pub index: u8,
    pub previous_or_next: u8,
    pub pending: u8,
    pub flags: u8,
    pub outer: u8,
    pub loader: u8,
    pub changing: u8,
    pub guard: u8,
}

impl EntryState {
    /// Initial loads use ff in +5919; completed form changes use the live index.
    /// Pointer/owner/body checks remain the native callback's responsibility.
    pub fn ready(self) -> Option<(Forms, u32)> {
        self.ready_with_install_marker(false)
    }

    /// Count activation calls native 656ee0 directly, retaining the completed
    /// parked install's +5934 marker. Only a current, sealed promotion may use
    /// this shape; it is not load completion or ownership authority. An active
    /// Reload still requires ready(), including marker zero.
    pub fn ready_count_promoted(self, parked_install: bool) -> Option<(Forms, u32)> {
        self.ready_with_install_marker(parked_install)
    }

    fn ready_with_install_marker(self, parked_install: bool) -> Option<(Forms, u32)> {
        let forms = if self.kinds[0] == 0x4b && self.count == 1 {
            Forms::solo_popo()
        } else {
            Forms::starting(self.kinds[0])
        };
        (self.count == forms.count
            && self.kinds == forms.kinds
            && self.actor_count == forms.actor_count
            && self.installed == u64::from(forms.count)
            && u32::from(self.index) < forms.count
            && (self.previous_or_next == 0xff || self.previous_or_next == self.index)
            && self.pending == 0xff
            && self.flags == forms.flags
            && self.outer == 6
            && self.loader == 14
            && (self.changing == 0 || (parked_install && self.changing == 1))
            && self.guard == 0)
            .then(|| (forms, self.kinds[self.index as usize]))
    }
}

/// Ice Climbers body for a slot. Player slot 0 of an Ice Climbers original (`numbered`
/// false, which includes a direct pick) follows the original-select preference; every
/// numbered slot, and a direct pick over another original, follows the cycle's. CPU has
/// one preference for its original and its cycle.
pub fn configured_forms(
    menu: &crate::engine::compat::menu::TrainingModpackMenu,
    cpu: bool,
    original: Option<u32>,
    kind: u32,
    numbered: bool,
) -> Forms {
    let solo = if cpu {
        menu.solo_popo_cpu
    } else if !numbered && original == Some(0x4b) {
        menu.solo_popo_original
    } else {
        menu.solo_popo_player_cycle
    };
    if kind == 0x4b && solo == crate::engine::compat::menu::OnOff::ON {
        Forms::solo_popo()
    } else {
        Forms::starting(kind)
    }
}

/// Zero-based numbered slot holding `current`'s native group, if any.
pub fn cycle_position(choices: &[u32], current: u32) -> Option<usize> {
    choices
        .iter()
        .position(|kind| Forms::starting(*kind).same_group(current))
}

/// The step after Player slot 0. `next_cycle` finds a live fighter by group, so it cannot
/// tell slot 0 from a numbered slot of the same group (Ice Climbers, when its Solo Popo
/// preference differs between the two); the caller knows and asks for this instead.
pub fn next_from_original(
    choices: &[u32],
    original: Option<u32>,
) -> Option<(u32, Option<usize>)> {
    let original = catalog_original(original);
    choices
        .first()
        .map(|kind| (*kind, Some(0)))
        .or_else(|| original.map(|kind| (kind, None)))
}

/// Slot 0 is a catalog fighter. `u32::MAX` and any other non-catalog kind are
/// missing, not a selected identity.
fn catalog_original(original: Option<u32>) -> Option<u32> {
    original.filter(|kind| crate::engine::swap::trio::TrioFighter::from_kind(*kind).is_some())
}

/// Ordered slots are zero-based internally; None is the original CSS slot 0.
/// A live transformed partner belongs to its group's existing numbered slot.
pub fn next_cycle(
    current: u32,
    cursor: Option<usize>,
    choices: &[u32],
    original: Option<u32>,
) -> Option<(u32, Option<usize>)> {
    let original = catalog_original(original);
    let position = cursor
        .filter(|i| *i < choices.len() && Forms::starting(choices[*i]).same_group(current))
        .or_else(|| cycle_position(choices, current));
    if let Some(index) = position {
        if index + 1 < choices.len() {
            return Some((choices[index + 1], Some(index + 1)));
        }
        if let Some(kind) = original {
            return Some((kind, None));
        }
    }
    next_from_original(choices, original)
}

/// Entry identity, owner and kind of a settled practice body. The identity is
/// compared only, never dereferenced.
pub type Receipt = (usize, u32, u32);

/// What a native presentation rebuild (Training reset) does for the live body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rebuild {
    /// Native reset presents the live ordinary fighter itself.
    Admit(Receipt),
    /// Native reset keeps its own rebuild. The next service callback republishes
    /// this Mii publication, and the exact-portrait lifecycle restores its texture.
    Republish(Receipt),
    /// Not a plugin publication: native reset rebuilds the original CSS portrait.
    Native,
}

fn mii_class(kind: u32) -> bool {
    (0x48..=0x4a).contains(&kind)
}

/// Native reset rebuilds the original CSS portrait unless the plugin restores the live
/// body. The ordinary loader's `loaded` receipt also gates saved-CPU idleness, so an
/// accepted Mii-adapter publication (named or replay fighter switch, saved or exact
/// Mii) never writes it. This presentation-only receipt records that publication
/// instead. An ordinary fighter is admitted into native reset. A Mii class body is
/// never admitted, because its bust belongs to the exact-portrait lifecycle; it is
/// republished after native reset instead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentedReceipt(Option<Receipt>);

impl PresentedReceipt {
    pub const NONE: Self = Self(None);

    /// An accepted adapter publication replaces the previous receipt.
    pub fn publish(&mut self, entry: usize, owner: u32, kind: u32) {
        self.0 = Some((entry, owner, kind));
    }

    /// A newly observed ready kind. The same entry and owner carry the receipt to
    /// it (a native transformation); a different identity retires it.
    pub fn observe(&mut self, entry: usize, owner: u32, kind: u32) {
        if self.0.is_some_and(|(identity, presented, _)| identity == entry && presented == owner) {
            self.publish(entry, owner, kind);
        } else {
            self.0 = None;
        }
    }

    /// Scene change, or an ordinary loader replacement of the body.
    pub fn retire(&mut self) {
        self.0 = None;
    }

    /// Before that entry's native destruction.
    pub fn retire_entry(&mut self, entry: usize) {
        if self.0.is_some_and(|(identity, _, _)| identity == entry) {
            self.0 = None;
        }
    }

    /// The receipt native reset may admit for `entry` presenting `kind`. A matching
    /// loader receipt wins; a stale one cannot hide a later adapter publication,
    /// which never updates `loaded`. A Mii class publication is never admitted.
    /// The caller still verifies the live body.
    pub fn admit(self, loaded: Option<Receipt>, entry: usize, kind: u32) -> Option<Receipt> {
        let matches = move |receipt: &Receipt| receipt.0 == entry && receipt.2 == kind;
        loaded
            .filter(matches)
            .or_else(|| self.0.filter(|receipt| matches(receipt) && !mii_class(receipt.2)))
    }

    /// The single native-rebuild decision for `entry` presenting `kind`, from both
    /// native reset hooks. The caller verifies the live body before acting on it.
    pub fn rebuild(self, loaded: Option<Receipt>, entry: usize, kind: u32) -> Rebuild {
        if let Some(receipt) = self.admit(loaded, entry, kind) {
            return Rebuild::Admit(receipt);
        }
        match self.0 {
            Some(receipt) if receipt.0 == entry && receipt.2 == kind && mii_class(kind) => {
                Rebuild::Republish(receipt)
            }
            _ => Rebuild::Native,
        }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn source_members_include_native_partners_without_making_them_selectable() {
        // A native Ice Climbers source owns both Popo and Nana resources.
        assert!(is_source_member(0x4b, 0x4b));
        assert!(is_source_member(0x4b, 0x4c));
        assert!(crate::engine::swap::trio::TrioFighter::from_kind(0x4c).is_none());
        // Membership is scoped to a valid requested source, not a kind whitelist.
        assert!(!is_source_member(0x00, 0x4c));
        assert!(!is_source_member(0x4b, u32::MAX));
        assert!(!is_source_member(0x4c, 0x4c));
        assert_eq!(is_source_member(0x48, 0x48), cfg!(feature = "mii-managed-helper-alpha"));
        assert!(is_source_member(0x5a, 0x5b));
        assert!(is_source_member(0x24, 0x26));

        // Native acquire/release share one typed record for the two IC bodies.
        assert_eq!(typed_record_kind(0x4b, 0x4b), Some(0x4b));
        assert_eq!(typed_record_kind(0x4b, 0x4c), Some(0x4b));
        assert_eq!(Forms::starting(0x4b).kinds, [0x4b, 0x4c, u32::MAX]);
        // Mapping cannot admit an unrelated member or a nonselectable request.
        for (requested, member) in [
            (0x00, 0x4c),
            (0x4c, 0x4c),
            (0x4b, u32::MAX),
        ] {
            assert_eq!(typed_record_kind(requested, member), None);
        }
        // Ordinary and transforming members retain their own parameter records.
        for (requested, member) in [
            (0x00, 0x00),
            (0x24, 0x26),
            (0x25, 0x24),
            (0x26, 0x25),
            (0x5a, 0x5b),
            (0x5b, 0x5a),
        ] {
            assert_eq!(typed_record_kind(requested, member), Some(member));
        }
    }

    // Facts captured from unmodified native CSS loads and Down+B transformations.
    fn native_ice_climbers_keep_both_bodies_active() {
        // Captured native CSS load on 13.0.5, separate from requested descriptors.
        let state = EntryState {
            count: 2,
            kinds: [0x4b, 0x4c, u32::MAX],
            actor_count: 2,
            installed: 2,
            index: 0,
            previous_or_next: 0xff,
            pending: 0xff,
            flags: 2,
            outer: 6,
            loader: 14,
            changing: 0,
            guard: 0,
        };
        let (forms, kind) = state.ready().unwrap();
        assert_eq!(kind, 0x4b);
        assert_eq!([forms.body_state(0, 0), forms.body_state(1, 0)], [4, 4]);
        assert!(EntryState {
            actor_count: 1,
            ..state
        }
        .ready()
        .is_none());
        assert!(EntryState {
            installed: 1,
            ..state
        }
        .ready()
        .is_none());
        assert_eq!(Forms::starting(0x5a).body_state(1, 0), 3);
    }
    fn player_visits_original_zero_and_cpu_returns_when_original_is_known() {
        let selected = [0x0b, 0x5b, 0x25];
        assert_eq!(
            next_cycle(6, None, &selected, Some(6)),
            Some((0x0b, Some(0)))
        );
        assert_eq!(
            next_cycle(0x0b, Some(0), &selected, Some(6)),
            Some((0x5b, Some(1)))
        );
        // Pyra transformed from selected Mythra and Charizard from Ivysaur.
        assert_eq!(
            next_cycle(0x5a, Some(1), &selected, Some(6)),
            Some((0x25, Some(2)))
        );
        assert_eq!(
            next_cycle(0x26, Some(2), &selected, Some(6)),
            Some((6, None))
        );
        assert_eq!(
            next_cycle(0x26, Some(2), &selected, None),
            Some((0x0b, Some(0)))
        );
        // A known CPU original is the return between loops, not another numbered copy.
        assert_eq!(
            next_cycle(0x26, Some(2), &selected, Some(0x0a)),
            Some((0x0a, None))
        );
        assert_eq!(
            next_cycle(0x0a, None, &selected, Some(0x0a)),
            Some((0x0b, Some(0)))
        );
        // Missing original does not become Mario, Ness, or a Mii class.
        assert_eq!(next_cycle(0x0a, None, &selected, None), Some((0x0b, Some(0))));
        assert_eq!(next_cycle(0x26, Some(2), &selected, Some(u32::MAX)), Some((0x0b, Some(0))));
        // Edited/reordered menu invalidates a stale cursor; actual group wins.
        assert_eq!(
            next_cycle(0x5a, Some(99), &selected, Some(6)),
            Some((0x25, Some(2)))
        );
        assert_eq!(next_cycle(6, None, &[], Some(6)), Some((6, None)));
        assert_eq!(next_cycle(6, None, &[], None), None);
    }

    fn solo_preference_follows_the_slot_not_just_the_kind() {
        let (off, on) = (crate::engine::compat::menu::OnOff::OFF, crate::engine::compat::menu::OnOff::ON);
        const IC: u32 = 0x4b;
        let menu = |original, cycle, cpu| {
            let mut saved = crate::engine::compat::menu::BASE_MENU;
            saved.solo_popo_original = original;
            saved.solo_popo_player_cycle = cycle;
            saved.solo_popo_cpu = cpu;
            saved
        };
        let (solo, pair) = (Forms::solo_popo(), Forms::starting(IC));
        // Solo original, ordinary cycle: slot 0 and a direct pick are Popo alone, numbered are not.
        let m = menu(on, off, off);
        assert_eq!(configured_forms(&m, false, Some(IC), IC, false), solo);
        assert_eq!(configured_forms(&m, false, Some(IC), IC, true), pair);
        // Ordinary original, Solo cycle: the reverse.
        let m = menu(off, on, off);
        assert_eq!(configured_forms(&m, false, Some(IC), IC, false), pair);
        assert_eq!(configured_forms(&m, false, Some(IC), IC, true), solo);
        // Another original has no slot-0 Ice Climbers: its cycle preference covers a direct pick too.
        for numbered in [false, true] {
            assert_eq!(configured_forms(&m, false, Some(0x00), IC, numbered), solo);
            assert_eq!(configured_forms(&m, false, None, IC, numbered), solo);
            let m = menu(on, off, on);
            assert_eq!(configured_forms(&m, false, Some(0x00), IC, numbered), pair);
        }
        // CPU reads its one preference for the original and every numbered slot.
        for (cpu, expected) in [(on, solo), (off, pair)] {
            let m = menu(cpu_flip(cpu), cpu_flip(cpu), cpu);
            for numbered in [false, true] {
                assert_eq!(configured_forms(&m, true, Some(IC), IC, numbered), expected);
            }
        }
        // Only Ice Climbers is ever solo.
        let m = menu(on, on, on);
        for kind in [0x00, 0x24, 0x5a] {
            for cpu in [false, true] {
                assert_eq!(configured_forms(&m, cpu, Some(IC), kind, false), Forms::starting(kind));
            }
        }
    }

    fn cpu_flip(value: crate::engine::compat::menu::OnOff) -> crate::engine::compat::menu::OnOff {
        if value == crate::engine::compat::menu::OnOff::ON { crate::engine::compat::menu::OnOff::OFF } else { crate::engine::compat::menu::OnOff::ON }
    }

    // Training CPU entry identity from a console log; compared only.
    const CPU_ENTRY: usize = 0x11aed3e000;
    const OTHER_ENTRY: usize = 0x11aed44000;

    fn css_dr_mario_to_exact_gunner_republishes_on_every_reset_until_retired() {
        const DR_MARIO: u32 = 0x12;
        const GUNNER: u32 = 0x4a;
        let mut receipt = PresentedReceipt::NONE;
        // CSS (Random) Dr. Mario: nothing published; native reset keeps its own path.
        assert_eq!(receipt.rebuild(None, CPU_ENTRY, DR_MARIO), Rebuild::Native);
        // A replay load publishes the exact Gunner through the Mii adapter; `loaded` stays empty.
        receipt.publish(CPU_ENTRY, 1, GUNNER);
        // Every native reset of that live body republishes; the decision has no one-shot state.
        for _ in 0..3 {
            assert_eq!(
                receipt.rebuild(None, CPU_ENTRY, GUNNER),
                Rebuild::Republish((CPU_ENTRY, 1, GUNNER))
            );
        }
        // A rebuild of another live kind or another entry is not this publication.
        assert_eq!(receipt.rebuild(None, CPU_ENTRY, DR_MARIO), Rebuild::Native);
        assert_eq!(receipt.rebuild(None, OTHER_ENTRY, GUNNER), Rebuild::Native);
        // A matching loader receipt is still the ordinary admission.
        let loaded = Some((CPU_ENTRY, 1, DR_MARIO));
        assert_eq!(receipt.rebuild(loaded, CPU_ENTRY, DR_MARIO), Rebuild::Admit((CPU_ENTRY, 1, DR_MARIO)));
        assert_eq!(receipt.rebuild(loaded, CPU_ENTRY, GUNNER), Rebuild::Republish((CPU_ENTRY, 1, GUNNER)));
        // Entry destruction, a new identity, another owner and scene exit retire it.
        receipt.retire_entry(CPU_ENTRY);
        assert_eq!(receipt.rebuild(None, CPU_ENTRY, GUNNER), Rebuild::Native);
        receipt.publish(CPU_ENTRY, 1, GUNNER);
        receipt.observe(OTHER_ENTRY, 1, GUNNER);
        assert_eq!(receipt.rebuild(None, OTHER_ENTRY, GUNNER), Rebuild::Native);
        receipt.publish(CPU_ENTRY, 1, GUNNER);
        receipt.observe(CPU_ENTRY, 4, GUNNER);
        assert_eq!(receipt, PresentedReceipt::NONE);
        receipt.publish(CPU_ENTRY, 1, GUNNER);
        receipt.retire();
        assert_eq!(receipt.rebuild(None, CPU_ENTRY, GUNNER), Rebuild::Native);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("source_members_include_native_partners_without_making_them_selectable", source_members_include_native_partners_without_making_them_selectable as fn()),
            ("native_ice_climbers_keep_both_bodies_active", native_ice_climbers_keep_both_bodies_active as fn()),
            ("player_visits_original_zero_and_cpu_returns_when_original_is_known", player_visits_original_zero_and_cpu_returns_when_original_is_known as fn()),
            ("solo_preference_follows_the_slot_not_just_the_kind", solo_preference_follows_the_slot_not_just_the_kind as fn()),
            ("css_dr_mario_to_exact_gunner_republishes_on_every_reset_until_retired", css_dr_mario_to_exact_gunner_republishes_on_every_reset_until_retired as fn()),
        ]);
    }
    }
}
