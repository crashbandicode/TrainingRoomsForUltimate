//! Value-only contract for the native three-member fighter bank experiment.
//!
//! This is the native entry's physical body/context capacity, not the configurable
//! source-cache budget. A body stays with its original entry throughout a switch.
//! Observations are values sampled by the current native callback; none of the
//! numeric identities here grants permission to dereference a retained pointer.

use crate::engine::swap::character_forms::{EntryState, Forms};
use std::convert::TryInto;

pub const NATIVE_MEMBERS: usize = 3;
pub const BANK_FLAG: u8 = 4;
/// Window flags while the normal Ice Climbers pair is the active member of a
/// pair-lead bank: bit 1 (pair; the native partner readers address cell 1) plus
/// bit 2 (bank table and teardown). A parked pair and ordinary members use
/// [`BANK_FLAG`] (ICE-CLIMBERS-OPTIMIZATION addendum, "Hybrid layout").
pub const PAIR_ACTIVE_FLAG: u8 = 6;
pub const POPO: u32 = 0x4b;
pub const NANA: u32 = 0x4c;
pub const KIRBY: u32 = 0x06;

// The eligibility predicates live in `crate::engine::swap::swap_model::capabilities` (C116.1 refactor WP1,
// WORKPLAN §2.2): one capability row per fighter form, each gate a one-liner over the table, and
// the FACT every row encodes documented there. The names and signatures are unchanged for every
// caller (`body_bank::member_forms_in`, `body_bank::prefetch_admitted`, …).
pub use crate::engine::swap::swap_model::capabilities::{d_kirby1_applies, d_kirby1_live, group_member_kind, kirby_parkable, member_forms, member_forms_in, native_group_request, object_rearm_admitted, original_bankable, original_form_bankable, pair_lead_extra, pair_request, prefetch_admitted, single_member_request, swap_commit_index, swap_commit_index_in, window_swappable_with};
#[cfg(test)]
pub use crate::engine::swap::swap_model::capabilities::{key_release_admitted, special_group_kind, swappable_kind, window_swappable};
use crate::engine::swap::swap_model::capabilities::{bankable_kind, pins_cell0};
use crate::engine::preload::cycle_preload_capacity::MAX_TARGETS;
// WP4: the resource bookkeeping wrappers below are thin views of the ledger's data.
use crate::engine::swap::swap_model::ledger::pair_rule;
#[cfg(test)]
use crate::engine::swap::swap_model::ledger::{native_window_requests, orphan_keys, RFlags};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Idle, Building, Ready, Changing, Retired }


/// Native shape of a swap group beyond the ordinary one-to-three single bodies
/// (SPECIAL-GROUPS-DESIGN §2, §3, §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Special {
    /// Ordinary single-body members (or the pair-lead window).
    None,
    /// A complete native group (Pokémon Trainer `[0x24,0x25,0x26]`, flags `0x05`;
    /// Pyra/Mythra `[0x5a,0x5b]`, flags `0x64`) built as its own group. `654dc0`
    /// writes these flags itself; the bank leaves them. Carries the entry's
    /// `+0x5a88/+0x5a90/+0x5a98` object records with its saved window.
    Native(u8),
    /// The normal Ice Climbers pair `[0x4b,0x4c]` as a detached group: two actors,
    /// flags 6 active / 4 parked, one `R+0x50` reference per body before the
    /// synchronous build (C122 rule). Only Popo is selectable; Nana is the
    /// transient in-window commit index on the way out.
    Pair,
    /// One Mii (`0x48..=0x4a`, preset or exact) as a one-member group. The
    /// entry's shared resource owner carries the Mii identity (`R+0x58..0x80`:
    /// kind, costume, record index, outfit/hat/colour/voice tokens, descriptor
    /// flag bytes) exactly as loader state 6 writes it for a canonical Mii
    /// (`0x650904-0x65137c`); the adapter writes it from the group's staged
    /// descriptor around the prefetch and leaves it in place while the Mii is
    /// the window (MII-GROUPS-DESIGN §3). One Mii identity per owner at a time.
    Mii,
}

/// Pokémon Trainer object kind (`0x658bb4`: `0x17e8890(…, R, 0x72, 0xc)`).
pub const PT_OBJECT_KIND: u32 = 0x72;
/// Pyra/Mythra partner object kind (`0x17e5bc0` Aegis branch: keys `0x1332/0x1333`, NRO `0x75`).
pub const AEGIS_OBJECT_KIND: u32 = 0x75;

/// The holder `+0x5a88` native loader state 11 makes for a native group (SPECIAL-GROUPS §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HolderStep {
    /// Pokémon Trainer (`0x650f38-0x6512c8`): the holder and the 1-byte object `+0x5a98` on the game thread, the pool
    /// task (`trainer_task`) on the worker.
    Trainer,
    /// Pyra/Mythra (`0x6509e8-0x650abc`): the holder, the partner table's guards and the pool, on the game thread.
    Partner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Members {
    kinds: [u32; NATIVE_MEMBERS],
    count: u8,
    /// Normal Ice Climbers pair in cells 0/1 plus one ordinary kind in cell 2.
    /// Selectable members are the pair (index 0) and that kind (index 2); cell 1
    /// (Nana) is never requested and follows the pair natively.
    pair_lead: bool,
    special: Special,
}

impl Members {
    /// Distinct single bodies only, one to three of them (`0x4b` is Solo Popo).
    /// Canonical multi-body groups and exact Miis keep their loader. A one-member
    /// group is an extra swap group, or the target-first window of a rebuild whose
    /// other members build as groups ([`Members::target_only`]); a full window bank
    /// starts with two members ([`Members::from_forms`]). A kind whose row pins cell 0
    /// (`CellRule::PinCell0`: Kirby, whose copy registry assumes sub 0 of its
    /// entry, RE-KIRBY-UNLOAD §0–§3; Solo Popo, whose secondary resource
    /// registration `0x17e8410` skips, SOLO-STALL-RCA §3) is refused at cells 1–2:
    /// such an entry is one native never builds. Planners lay rotations out with
    /// [`Members::laid_out`]; `new` takes physical cells.
    pub fn new(kinds: &[u32]) -> Option<Self> {
        if !(1..=NATIVE_MEMBERS).contains(&kinds.len()) {
            return None;
        }
        for (i, &kind) in kinds.iter().enumerate() {
            if !bankable_kind(kind) || kinds[..i].contains(&kind) || (i != 0 && pins_cell0(kind)) {
                return None;
            }
        }
        let mut members = Self { kinds: [u32::MAX; NATIVE_MEMBERS], count: kinds.len() as u8, pair_lead: false, special: Special::None };
        members.kinds[..kinds.len()].copy_from_slice(kinds);
        Some(members)
    }

    /// A complete native group as its own swap group: Pokémon Trainer (any
    /// form) or Pyra/Mythra (either). The cells are `Forms::starting` of the
    /// group's lowest kind, which is the native descriptor expansion and the
    /// order of the native `64fb70` tables (`0x453c164` / `0x453c158`).
    pub fn native_group(kind: u32) -> Option<Self> {
        let lead = match kind { 0x24..=0x26 => 0x24, 0x5a | 0x5b => 0x5a, _ => return None };
        let forms = Forms::starting(lead);
        Some(Self { kinds: forms.kinds, count: forms.count as u8, pair_lead: false, special: Special::Native(forms.flags) })
    }

    /// The normal Ice Climbers pair as a detached two-body group.
    pub fn pair_group() -> Self {
        Self { kinds: [POPO, NANA, u32::MAX], count: 2, pair_lead: false, special: Special::Pair }
    }

    /// A Mii class as its own one-member group (bank flags, one actor). Which
    /// identity it carries is the adapter's record (`Group.identity`); the value
    /// contract here is the window shape only.
    pub fn mii_group(kind: u32) -> Option<Self> {
        (0x48..=0x4a).contains(&kind)
            .then_some(Self { kinds: [kind, u32::MAX, u32::MAX], count: 1, pair_lead: false, special: Special::Mii })
    }

    /// The special group a lane kind forms, if any (`policy` decides which kinds).
    pub fn special_group(kind: u32, policy: SpecialPolicy) -> Option<Self> {
        match kind {
            0x24..=0x26 | 0x5a | 0x5b if policy.native => Self::native_group(kind),
            POPO if policy.pair => Some(Self::pair_group()),
            0x48..=0x4a if policy.mii => Self::mii_group(kind),
            _ => None,
        }
    }

    pub fn special(self) -> Special { self.special }

    /// Popo and Nana occupy cells 0/1: the pair-lead window or a pair group.
    pub fn is_pair(self) -> bool { self.pair_lead || self.special == Special::Pair }

    /// Pair-lead bank `[Popo, Nana, extra]`, count 3, two actors. Built only when
    /// the requested target is the normal pair; the pair never sits in cells 1/2.
    pub fn pair_lead(extra: u32) -> Option<Self> {
        pair_lead_extra(extra)
            .then_some(Self { kinds: [POPO, NANA, extra], count: NATIVE_MEMBERS as u8, pair_lead: true, special: Special::None })
    }

    /// Pair target with the first eligible upcoming kind in cell 2. Nana and that
    /// kind are two extra bodies, so a smaller extra budget yields no pair bank.
    pub fn from_pair(upcoming: &[u32], extra_budget: usize) -> Option<Self> {
        if extra_budget < 2 {
            return None;
        }
        upcoming.iter().copied().find(|kind| pair_lead_extra(*kind)).and_then(Self::pair_lead)
    }

    pub fn is_pair_lead(self) -> bool { self.pair_lead }

    /// Descriptor word `+0x28`: two simultaneous actors for the pair bank and the pair group.
    pub fn actor_count(self) -> u32 { if self.is_pair() { 2 } else { 1 } }

    /// A member index a request may select (`+0x591a`) and readiness may report.
    /// A pair group selects Popo only; a native group selects every form.
    pub fn selectable(self, index: u8) -> bool {
        match self.special {
            Special::Pair => index == 0,
            _ if self.pair_lead => index == 0 || index == 2,
            _ => usize::from(index) < self.count(),
        }
    }

    /// Window flags `+0x591f` native must see while `index` is the active member.
    /// Native groups keep the flags `654dc0` wrote (`0x05` / `0x64`); the pair
    /// (lead or group) is 6 while active and 4 while parked.
    pub fn expected_flags(self, index: u8) -> u8 {
        match self.special {
            Special::Native(flags) => flags,
            _ if self.is_pair() && index == 0 => PAIR_ACTIVE_FLAG,
            _ => BANK_FLAG,
        }
    }

    /// Body state `+0x3a` of cell `index`: 4 active, 3 parked. `None` is a commit in
    /// progress with every cell parked. The pair's second cell is active with it.
    pub fn body_state(self, index: usize, active: Option<u8>) -> u8 {
        match active {
            Some(active) if usize::from(active) == index => 4,
            Some(0) if self.is_pair() && index == 1 => 4,
            _ => 3,
        }
    }

    /// Logical form of a member kind of this bank: the pair-lead (or pair-group)
    /// Popo is the normal pair, a native group member is its native group form,
    /// every other member its single form (Solo for a Solo bank).
    pub fn forms_of(self, kind: u32) -> Forms {
        match self.special {
            Special::Native(_) => Forms::starting(kind),
            _ if self.is_pair() && kind == POPO => Forms::starting(POPO),
            _ => member_forms(kind),
        }
    }

    /// Selectable kinds in cell order (the pair counted once, as Popo).
    pub fn logical_kinds(self) -> ([u32; NATIVE_MEMBERS], usize) {
        match self.special {
            Special::Pair => ([POPO, u32::MAX, u32::MAX], 1),
            _ if self.pair_lead => ([POPO, self.kinds[2], u32::MAX], 2),
            _ => (self.kinds, self.count()),
        }
    }

    /// In-window commit index that parks the current member before a cross-group
    /// swap. A one-member window (ordinary, Solo Popo or Mii) commits to its own
    /// cell (`ExitRule::SelfCell`: `6581f0` never compares the request with
    /// `+0x5918`); a native group behaves like an ordinary window; the pair-lead
    /// window commits between Popo (0) and its extra (2), never Nana's cell
    /// (EXCLUSIONS row 6); a pair group commits through Nana (1): `64fb70` resolves
    /// her context from the lent table (word `0x4c` at index 1, `+0x4190`), the
    /// commit parks Popo, N3 parks Nana, and the seam replaces the window before
    /// any activation (DESIGN §5).
    pub fn swap_commit_index_for(self, from: u8) -> Option<u8> {
        match self.special {
            _ if self.count() == 1 => (from == 0).then_some(0),
            Special::Pair => (from == 0).then_some(1),
            _ if self.pair_lead => match from { 0 => Some(2), 2 => Some(0), _ => None },
            _ => swap_commit_index(from, self.count()),
        }
    }

    /// Index the fallback path activates when a cross-group swap is refused after
    /// the commit: the committed in-window member, except for a pair group,
    /// whose only activatable member is the pair itself (N2 wakes both).
    pub fn fallback_index(self, via: u8) -> u8 {
        if self.special == Special::Pair { 0 } else { via }
    }

    /// Kinds the owned prefetch registers in R for this group: every member plus
    /// the object kind a native group's synchronous build looks up through R
    /// (PT `0x72` at `0x658bb4`; Aegis `0x75` from the `0x17e5bc0` Aegis branch).
    pub fn prefetch_kinds(self) -> ([u32; NATIVE_MEMBERS + 1], usize) {
        let mut kinds = [u32::MAX; NATIVE_MEMBERS + 1];
        let mut count = 0;
        for kind in self.kinds.iter().take(self.count()) { kinds[count] = *kind; count += 1; }
        if let Some(object) = self.object_kind() { kinds[count] = object; count += 1; }
        (kinds, count)
    }

    /// Object kind registered for the build and released by owned code right
    /// after it: no body construction tail releases it (DESIGN §2.3, §3).
    pub fn object_kind(self) -> Option<u32> {
        match self.special {
            Special::Native(flags) if flags & 1 != 0 => Some(PT_OBJECT_KIND),
            Special::Native(flags) if flags & 0x20 != 0 => Some(AEGIS_OBJECT_KIND),
            _ => None,
        }
    }

    /// The entry records `+0x5a88/+0x5a90/+0x5a98` belong to this group's window.
    pub fn carries_records(self) -> bool { matches!(self.special, Special::Native(_)) }

    /// SPECIAL-GROUPS §3: the holder step native loader state 11 runs once a native group's bodies settled
    /// (`0x650010` case 0xb), which its background build mirrors.
    pub fn holder_step(self) -> Option<HolderStep> {
        match self.special {
            Special::Native(flags) if flags & 1 != 0 => Some(HolderStep::Trainer),
            Special::Native(flags) if flags & 0x20 != 0 => Some(HolderStep::Partner),
            _ => None,
        }
    }

    /// SPECIAL-GROUPS §0 X5: the cell 1 body init reads (`FE+0x1c`, `0x607ba8`) for a Pyra/Mythra body. `-1` (a
    /// one-member window) turns the body's model kind into `0x73`/`0x74`, files the group never prefetched; the
    /// group's own cell 1 keeps its kind, as its synchronous build does.
    pub fn partner_cell(self) -> Option<u32> {
        (self.holder_step() == Some(HolderStep::Partner)).then_some(self.kinds[1])
    }

    /// The shared resource owner must hold this group's Mii identity while it is
    /// the window, and the previous window's words while it is detached.
    pub fn carries_identity(self) -> bool { self.special == Special::Mii }

    pub fn kinds(self) -> [u32; NATIVE_MEMBERS] { self.kinds }
    /// Borrow the immutable native table. The adapter must retain its backing
    /// allocation until the enclosing native function has returned.
    pub fn table(&self) -> &[u32; NATIVE_MEMBERS] { &self.kinds }
    pub fn count(self) -> usize { usize::from(self.count) }

    /// Target first, then upcoming kinds in the order supplied. Ineligible and
    /// duplicate kinds are skipped and do not consume a slot. `extra_budget` is
    /// the number of additional bodies, capped by `NATIVE_MEMBERS - 1`. It is not
    /// the source-cache size. `None` when the target is ineligible or fewer than
    /// two members remain. Padding stays `u32::MAX` through [`Members::new`].
    /// No allocation.
    #[cfg(test)]
    pub fn from_cycle(target: u32, upcoming: &[u32], extra_budget: usize) -> Option<Self> {
        Self::from_forms(Forms::starting(target), upcoming, extra_budget)
    }

    /// Configured form, including explicitly solo Popo. An upcoming `0x4b` is Solo
    /// Popo (the adapter passes it only when that slot's configured form is Solo;
    /// the pair is never a cell 1–2 member) and is laid out at cell 0 like Kirby
    /// (`CellRule::PinCell0`). One pinned member per window: an upcoming kind that
    /// pins cell 0 after the target or an earlier upcoming one did waits for the
    /// next window, and the next eligible kind takes its place.
    pub fn from_forms(forms: Forms, upcoming: &[u32], extra_budget: usize) -> Option<Self> {
        if single_member_request(forms.count, forms.kinds, forms.actor_count) != Some(forms) {
            return None;
        }
        let target = forms.kinds[0];
        let mut selected = [target, 0, 0];
        let mut count = 1usize;
        let mut pinned = pins_cell0(target);
        let extras = extra_budget.min(NATIVE_MEMBERS - 1);
        for &kind in upcoming {
            if count > extras {
                break;
            }
            if selected[..count].contains(&kind) || !bankable_kind(kind) {
                continue;
            }
            if pins_cell0(kind) {
                if pinned {
                    continue;
                }
                pinned = true;
            }
            selected[count] = kind;
            count += 1;
        }
        if count < 2 {
            return None;
        }
        Self::laid_out(&selected[..count])
    }

    /// rc.6.1 target-first: the window of the picked fighter alone, the one-member window a rebuild makes when its other
    /// members build as background groups ([`target_first`]). Only a single-body form has one (an ordinary kind, Kirby,
    /// Solo Popo). Its Reload words are native's own single form, and a count-1 window with the bank flag is the state
    /// every exchange into a one-member group leaves (`ExitRule::SelfCell`). No allocation.
    pub fn target_only(forms: Forms) -> Option<Self> {
        if single_member_request(forms.count, forms.kinds, forms.actor_count) != Some(forms) {
            return None;
        }
        Self::new(&forms.kinds[..1])
    }

    /// Physical layout of a rotation (`CellRule::PinCell0`): a member whose row pins cell 0
    /// (Kirby or Solo Popo) takes cell 0 and the others keep their rotation order in cells 1–2;
    /// otherwise the cells are the rotation itself. The rotation the player chose is unchanged;
    /// the target's cell is `index_of(target)` (the Reload's starting form, the warm path's active
    /// index). `None` for two pinned members (both need cell 0; the planners let the later one
    /// wait) or a layout [`Members::new`] refuses. No allocation.
    pub fn laid_out(rotation: &[u32]) -> Option<Self> {
        if rotation.len() > NATIVE_MEMBERS {
            return None;
        }
        let mut pinned = None;
        let mut pins = 0;
        for (i, &kind) in rotation.iter().enumerate() {
            if pins_cell0(kind) { pins += 1; pinned = Some(i); }
        }
        if pins > 1 {
            return None;
        }
        let mut cells = [u32::MAX; NATIVE_MEMBERS];
        let mut filled = 0;
        if let Some(p) = pinned { cells[0] = rotation[p]; filled = 1; }
        for (i, &kind) in rotation.iter().enumerate() {
            if pinned == Some(i) { continue; }
            cells[filled] = kind;
            filled += 1;
        }
        Self::new(&cells[..filled])
    }

    pub fn index_of(self, kind: u32) -> Option<u8> {
        self.kinds[..self.count()]
            .iter()
            .position(|&stored| stored == kind)
            .map(|index| index as u8)
            .filter(|index| self.selectable(*index))
    }

    /// Index and kind after [`Members::ready_index`]. The kind is the observed
    /// descriptor word at that index. Nothing here permits a pointer dereference.
    pub fn ready_live(self, state: EntryState) -> Option<(u8, u32)> {
        let index = self.ready_index(state)?;
        Some((index, state.kinds[usize::from(index)]))
    }

    /// Descriptor words and installed count, without the flags byte.
    fn words_match(self, state: EntryState) -> bool {
        state.count == u32::from(self.count)
            && state.kinds == self.kinds
            && state.actor_count == self.actor_count()
            && state.installed == u64::from(self.count)
    }

    /// Words plus the flags the active index requires (pair active: 6; else 4).
    pub fn descriptor_matches(self, state: EntryState) -> bool {
        self.words_match(state) && state.flags == self.expected_flags(state.index)
    }

    /// Structural readiness only. The adapter must additionally validate all
    /// current bodies' owner, kind, vtable, accessor and active/parked states.
    pub fn ready_index(self, state: EntryState) -> Option<u8> {
        (self.descriptor_matches(state)
            && self.selectable(state.index)
            && (state.previous_or_next == 0xff || state.previous_or_next == state.index)
            && state.pending == 0xff && state.outer == 6 && state.loader == 14
            && state.changing == 0 && state.guard == 0)
            .then_some(state.index)
    }

    /// A native commit has selected the requested bank member and is waiting
    /// for deactivation completion. This predicate alone must never acknowledge
    /// it: the adapter also proves both bodies are parked, same-owner, unchanged
    /// objects and the selected controller context is live. No timeout qualifies.
    /// A pair-lead commit always waits with [`BANK_FLAG`]: parking the pair sets
    /// it inside the same commit call, and a parked pair already carries it.
    /// A native group waits with its own flags (`0x05` / `0x64`), which the
    /// commit does not change; the pair (lead or group) always waits with
    /// [`BANK_FLAG`]. The committed index of a pair group is Nana (1), which is
    /// not selectable: it is accepted here only as the transient commit index.
    pub fn awaiting_completion(self, state: EntryState, target: u8) -> bool {
        let flags = match self.special { Special::Native(flags) => flags, _ => BANK_FLAG };
        let committed = if self.special == Special::Pair { target == 1 } else { self.selectable(target) };
        self.words_match(state) && state.flags == flags && committed
            && state.index == target && state.previous_or_next == target
            && state.pending == 0xff && state.outer == 3 && state.loader == 12
            && state.changing == 1 && state.guard == 0
    }
}

/// Row 4: the lane entries a window may take as members, in lane order, at most
/// `MAX_TARGETS`. A window member is a single body, so a lane `0x4b` stands for
/// Solo Popo and is kept only when `solo_popo_member` holds (the adapter's proof
/// from the menu: the role's numbered form is Solo and the entry is not the
/// canonical-only original); the pair is never a cell 1–2 member. Other kinds
/// pass unchanged ([`Members::from_forms`] filters them by row). No allocation.
pub fn member_lane(upcoming: &[u32], solo_popo_member: bool) -> ([u32; MAX_TARGETS], usize) {
    let mut lane = [u32::MAX; MAX_TARGETS];
    let kept = upcoming.iter().copied().filter(|kind| *kind != POPO || solo_popo_member);
    let count = lane.iter_mut().zip(kept).fold(0, |count, (slot, kind)| { *slot = kind; count + 1 });
    (lane, count)
}

/// Descriptor byte of the starting form a Reload activates (`+0x43`).
pub const STARTING_FORM: usize = 0x43;

/// What a Reload of a staged 0x70-byte descriptor delivers at Ready: the cell
/// native activates and the logical target kind the owner's transaction settles
/// on. FACT (owned 13.0.5 ELF): the Reload start stages the request at `+0x70`
/// and sets `+0x5918`/`+0x5919` from its starting form `+0x43`
/// (`0x6533a8..0x653400`), and loader state 4 copies the staged bytes to the
/// entry verbatim, so native is ready at that cell (Eden M4b `m4b2`:
/// `ready owner=0 index=1 kind=0x5` for `cells=[6, 5, 7] active=1`). A bank
/// window build's target is the member in that cell (`CellRule::PinCell0` puts
/// Kirby or Solo Popo at cell 0 and the target at its own cell, written as the
/// starting form by the planner); a canonical request's target is its group's
/// first kind (a PT/Aegis group is named by its expansion). Never cell 0 by
/// assumption: a transaction that waits for index 0 or names `+0x18[0]` stalls
/// or settles on the pinned member instead of the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReloadTarget {
    pub cell: u8,
    pub kind: u32,
}

impl ReloadTarget {
    /// `bank`: the descriptor is a body-bank window build (the planner's `Plan`), not a
    /// canonical request.
    pub fn of(descriptor: &[u8; 0x70], bank: bool) -> Self {
        let cell = descriptor[STARTING_FORM];
        let word = |at: usize| u32::from_le_bytes(descriptor[at..at + 4].try_into().unwrap());
        let named = if bank { usize::from(cell).min(NATIVE_MEMBERS - 1) } else { 0 };
        Self { cell, kind: word(0x18 + named * 4) }
    }

    /// Native reports this Reload ready: its active index `+0x5918` is the starting cell.
    pub fn ready_at(self, index: u8) -> bool { index == self.cell }
}

/// A single-Mii request (count 1, one class kind, one actor): a target a
/// prefetched or built Mii group of the same identity can serve. Window banks
/// never adopt it (`single_member_request` keeps refusing the Mii range).
pub fn mii_group_request(count: u32, kinds: [u32; 3], actors: u32) -> Option<Forms> {
    if !(0x48..=0x4a).contains(&kinds[0]) { return None; }
    let forms = Forms::starting(kinds[0]);
    (count == forms.count && kinds == forms.kinds && actors == forms.actor_count).then_some(forms)
}

/// Which special groups the planner may form (menu/INI: PT and Aegis default on,
/// the pair group and the Mii group default on since C116.1; `group_swap` off
/// yields `NONE`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpecialPolicy {
    pub native: bool,
    pub pair: bool,
    /// A lane Mii (configured target) forms a one-member group; at most one per
    /// role at a time (one Mii identity per resource owner).
    pub mii: bool,
}

impl SpecialPolicy {
    pub const NONE: Self = Self { native: false, pair: false, mii: false };
    #[cfg(test)]
    pub const NATIVE: Self = Self { native: true, pair: false, mii: false };
    #[cfg(test)]
    pub const ALL: Self = Self { native: true, pair: true, mii: false };
    #[cfg(test)]
    pub const MII: Self = Self { native: true, pair: false, mii: true };
}

/// Kinds the bank's resource hook must request in addition to what the native
/// `0x17e5bc0` already requested for this window group, as `(kinds, count)`.
///
/// For an ordinary primary the native routine requests only `R+0x58`, so the
/// hook owns members 1 and 2. For a Solo-Popo primary the native jump table
/// (`kind-0x24`, `0x17e5c7c-0x17e5c84`) already tail-requests `R+0x5c`, so the
/// hook owns member 2 only. Requesting a kind twice leaves a map reference that
/// the per-body construction release never balances; the resource owner then
/// never empties, resets or frees, and the next canonical Reload is refused at
/// loader state 7 (SOLO-STALL-RCA §2–§3).
///
/// WP4: derived from the ledger's model of `0x17e5bc0` (`ledger::native_window_requests`): every
/// window kind native did not request itself. The native adapter records those requests as
/// `Holder::Native` tokens and acquires the rest through the ledger, which refuses a second
/// construction acquisition structurally; this wrapper remains the value-only view.
#[cfg(test)]
pub fn supplemental_acquisitions(members: Members) -> ([u32; NATIVE_MEMBERS], usize) {
    let native: Vec<u32> = native_window_requests(members.kinds(), RFlags::TRAINING).iter().map(|r| r.kind).collect();
    let mut kinds = [u32::MAX; NATIVE_MEMBERS];
    let mut count = 0;
    for kind in members.kinds().iter().take(members.count()) {
        if !native.contains(kind) {
            kinds[count] = *kind;
            count += 1;
        }
    }
    (kinds, count)
}

/// Pair-rule reference count the bank's resource hook must leave in `R+0x50`, or
/// `None` when native already holds the right value.
///
/// Native state 6 (`0x650010`, `0x651384-0x6513a4`) sets `R+0x50 = 2` for a Popo
/// primary with a secondary kind: one reference per body of a vanilla pair. The
/// construction tail `0x60de60-0x60deb8` takes that pair rule whenever the entry's
/// cells 0/1 are `[0x4b, 0x4c]`: it decrements `R+0x50` under the `R+0x28` lock
/// and, when the value before the decrement was at most 1, resets the whole R
/// (`vtable+0x20 = 0x17e45f0`: key map, `R+0x4c`, `R+0x50`). A pair-lead window
/// constructs three bodies under that rule, so with the vanilla 2 the second body
/// to finish discards the resources the third is still loading with (B119: member
/// 2's construction record never completes, loader state 11 forever). One
/// reference per window body lets the native reset fire after the last of them,
/// exactly as it fires after the second of a vanilla pair. Every other layout
/// releases per kind (`0x17e8610`) and never reads `R+0x50`.
///
/// WP4: the pair rule is the ledger's (`ledger::pair_rule`: cells 0/1 exactly `[Popo, Nana]`);
/// the write itself goes through `r_handle::set_pair_refs` under an exclusive reset batch.
pub fn pair_resource_references(members: Members) -> Option<u32> {
    pair_rule(members.kinds()).then_some(members.count() as u32)
}

/// Resource-map keys the owned prefetch registers for a native group's object
/// kind that native's per-kind release never removes; the bank releases them
/// itself, before `0x17e8610(R, kind)`, wherever it releases prefetched kinds
/// (D-SG1-RCA; SPECIAL-GROUPS-DESIGN risk R-AG1 resolved).
///
/// `0x17e64d0(R, k)` registers `k*42 + 0` and `k*42 + 6` for every non-Mii kind
/// (`+1` as well when the `0x65dd70` table says so: it does for `0x75`, not for
/// `0x72`; `+7` only for Byleth), each only when its file exists (`0x17e2df0`
/// inserts no node for the `0xffffff` handle). Native's own Aegis branch of
/// `0x17e5bc0` registers only `0x1332/0x1333` (`0x75*42 + {0, 1}`), and the
/// release `0x17e8610(R, 0x75)` mirrors exactly that pair (`0x17e8770-0x17e8780`,
/// then the empty-map check at `0x17e883c`) instead of the generic
/// `+0/+1/+6/+7` loop every fighter kind and `0x72` take (`0x17e86f4` releases
/// `0x12b4` and falls through to `0x17e8708`). The `+6` key `0x1338` therefore
/// survives the Aegis bodies' construction tails, the partner weapon's own
/// `0x17e8610(R, 0x75)` (`0x33f45e4`) and the bank's post-build release:
/// `R+0x18` never returns to zero, state 6 of the role's next canonical Reload
/// finds no free resource owner (`0x17e4700`: `R+0x4c0 == 0 && R+0x18 == 0`)
/// and ends unpublished in state 7 (B123-pre seq 79, both laps: `map=9` after
/// the Aegis build with the PT group's eight keys pending, `map=1` after the PT
/// build, `native_return … phase=Refused loader=7`).
///
/// WP4: computed, not hard-coded — `ledger::orphan_keys` = registration set minus native release
/// set under the Training flags (`0x1338` for `0x75`, nothing for any other kind); the release
/// itself is issued by `r_handle::release` in that order.
#[cfg(test)]
pub fn orphan_object_keys(kind: u32) -> ([u32; 2], usize) {
    let mut keys = [u32::MAX; 2];
    let mut count = 0;
    for key in orphan_keys(kind, RFlags::TRAINING).iter(kind) {
        if count < keys.len() {
            keys[count] = key.value();
            count += 1;
        }
    }
    (keys, count)
}

/// Reserve spare bodies fairly between the two visible roles. Existing native
/// banks keep their budget until a real native replacement/teardown; changing a
/// setting never frees live game objects. New plans consume only the remainder.
/// Returns per-role reservations (excluding each active body), without allocation.
pub fn reserve_extras(wanted: [usize; 2], existing: [usize; 2], slots: usize) -> [usize; 2] {
    let mut reserved = existing;
    let mut available = slots.saturating_sub(existing.iter().sum());
    for depth in 0..NATIVE_MEMBERS - 1 {
        for role in 0..2 {
            if reserved[role] <= depth && wanted[role].min(NATIVE_MEMBERS - 1) > depth
                && available != 0 {
                reserved[role] += 1;
                available -= 1;
            }
        }
    }
    reserved
}

// ---------------------------------------------------------------------------
// Same-R group swap: more than one built group per visible entry.
//
// The native entry window holds exactly three cells. A role may keep further
// groups of one to three built bodies detached from that window and swap a
// whole group in or out at the existing bank acknowledgement seam. Everything
// below is value-only planning and state-machine policy; the native adapter
// owns every pointer, write and proof (GROUP-SWAP-FEASIBILITY §6).
// ---------------------------------------------------------------------------

/// Process-memory admission of extra groups (GS-PANIC): what a group costs per phase and what admitted groups
/// still hold. The measured table is generated by `tools/group_memory_costs.py`.
pub mod memory;
pub mod memory_table;
/// C116.2 mii-fix: the retry policy of a planned Mii group's record publication (log once, fail fast on a permanent
/// refusal, count only races).
pub mod mii_publish;
/// CPU-WINDOW-REFORM: with group swap off, the windows' share of the slot count comes from the whole cycle and is
/// reserved before the helpers'.
pub mod window_share;

/// Groups one role may hold beyond its native window.
pub const MAX_EXTRA_GROUPS: usize = 3;
/// Fighter-pool bodies kept free for visible Reloads and helper creation.
pub const POOL_RESERVE: usize = 6;

// `swappable_kind` / `original_bankable`: `swap_model::capabilities` (re-exported above). R.O.B.
// (`0x2d`) stays admitted (EXCLUSIONS-AUDIT §1): the native costume selector inlined at
// `0x17e64f0`, `0x607dc0`, `0x17e9a24`, `0x611ac8` and `0x17e8410` only *reads* `R+0x58`/`R+0x64`
// to pick a costume argument and never stores either word, so a group swap, which leaves
// `R+0x54..` as it found it, keeps the same-R invariant for R.O.B. exactly as for any ordinary
// kind. Visual caveat (pre-existing in the window bank, not caused by group swap): the selector
// keys on the *primary* kind, so with costume slot 0/1 and a Western language index a R.O.B.
// primary builds its co-members with slots 0 and 1 exchanged, and a R.O.B. built under another
// primary is not swapped. Eden verifies the colour shots (ROWS1-6-IMPL.md matrix).

/// Runtime switches of the group-swap planner that widen Ice Climbers handling
/// (EXCLUSIONS-AUDIT §4b/§4c). The planner's baseline [`GroupPolicy::DEFAULT`]
/// keeps both off; the adapter derives the live policy from
/// `cycle_preload_config::Settings` (`solo_popo_groups_enabled`,
/// `ic_pair_groups_enabled`), whose menu switches default on since B123-pre
/// rows 3 and 4 passed, and whose `group_swap` master switch turns both off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupPolicy {
    /// Solo Popo may be a member of a detached group (laid out at its cell 0,
    /// `CellRule::PinCell0`; the Solo Popo row's `GroupSwitch`).
    pub solo_popo_groups: bool,
    /// A pair-lead window `[Popo, Nana, X]` may own detached groups; the pair is
    /// parked by the commit (N3) before the group swaps in, and the pair group
    /// itself is still never built detached (`R+0x50` pair rule, N1).
    pub ic_pair_groups: bool,
}

impl GroupPolicy {
    #[cfg(test)]
    pub const DEFAULT: Self = Self { solo_popo_groups: false, ic_pair_groups: false };
}

// `group_member_kind`, `window_swappable`, `window_swappable_with`: `swap_model::capabilities`
// (re-exported above), derived from each member's row and cell (EXCLUSIONS-AUDIT §4a/§4b/§4c;
// M0: a Mii window owns nothing).

/// Lifecycle of one extra group. Only the native adapter advances it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupState {
    /// Chosen by the planner; nothing native exists yet.
    Planned,
    /// Its kinds are registered in the entry's resource owner; the load is in flight.
    Prefetching,
    /// The resource owner reports ready: the group can be built at the first switch.
    Prefetched,
    /// Bodies exist (in the window, or detached with saved window fields).
    Built,
    /// Prefetch or build failed; keys released; never retried in this scene.
    Failed,
    /// Planned, but its prefetch start found too little process headroom or too few free pool
    /// bodies; nothing native was touched. It keeps its place in the plan (no helper duplicates
    /// its kinds) and its start is retried when the capacity returns.
    Held(Hold),
    /// rc.6 mid-session cycle change (inbox 19:21Z): a member left the role's cycle (`group_left_cycle`) and the group
    /// was released (keys back, bodies destroyed). It is no longer part of the plan; its slot takes the next planned
    /// group (`reuse_slot`).
    Released,
}

/// rc.6 mid-session cycle change (operator requirement 2026-11-08, inbox 19:21Z): a detached group holds a member the
/// role's cycle no longer has. `wanted` is the cycle's kinds and the role's original. A pair group's Nana follows its
/// Popo; a native group (Pokémon Trainer, Pyra/Mythra) is one cycle member, kept while any of its forms is wanted.
pub fn group_left_cycle(members: Members, wanted: &[u32]) -> bool {
    let kinds = &members.kinds()[..members.count()];
    match members.special() {
        Special::Native(_) => !kinds.iter().any(|kind| wanted.contains(kind)),
        _ => kinds.iter().any(|kind| !(wanted.contains(kind) || (*kind == NANA && wanted.contains(&POPO)))),
    }
}

/// Operator decision 2026-11-08 23:42Z ("free under the pause menu"): a Built group that left the cycle stays parked,
/// out of the plan, and is destroyed when Training's pause screen opens, because native's body destroy holds the game
/// thread 75-134 ms per body (Eden `rc6-hot-xps16`). Its still-wanted members stay reachable until then. A Prefetched
/// group returns its keys at once (it costs nothing); every other state holds nothing native. Operator rule 2026-11-09
/// 13:21Z: a parked group the new members need the room of goes now ([`leaves_now`]).
pub fn leaves_at_pause(state: GroupState) -> bool {
    state == GroupState::Built
}

/// The slot a newly planned group takes: the first Released one (its native objects and keys are gone), else none and
/// the group is appended.
pub fn reuse_slot(states: impl Iterator<Item = GroupState>) -> Option<usize> {
    states.enumerate().find(|(_, state)| *state == GroupState::Released).map(|(i, _)| i)
}

/// The group whose prefetch starts next, from `(state, publishable)` in plan order: the first
/// planned one, else a held one (a held group is retried but never blocks a group the admission
/// has not seen yet). Held retries rotate: the first held group at or after `turn` (modulo the
/// group count), so a smaller held group is not starved by a larger one before it.
/// `publishable` is false for a Mii group whose record is not out.
pub fn next_start(groups: impl Iterator<Item = (GroupState, bool)> + Clone, turn: usize) -> Option<usize> {
    if let Some(planned) = groups.clone().position(|(state, publishable)| publishable && state == GroupState::Planned) {
        return Some(planned);
    }
    let count = groups.clone().count();
    (0..count).map(|k| (turn + k) % count).find(|i| groups.clone().nth(*i)
        .is_some_and(|(state, publishable)| publishable && matches!(state, GroupState::Held(_))))
}

/// What a held group waits for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    Memory,
    Pool,
}

impl GroupState {
    /// The start of its prefetch is still owed (planned, or held for capacity).
    pub fn startable(self) -> bool {
        matches!(self, Self::Planned | Self::Held(_))
    }
    /// A request may switch into this group without a Reload.
    pub fn reachable(self) -> bool {
        matches!(self, Self::Prefetched | Self::Built)
    }
    /// Resource-owner keys are registered and must be released before a destroyer.
    pub fn holds_keys(self) -> bool {
        matches!(self, Self::Prefetching | Self::Prefetched)
    }
    /// Still part of the plan denominator and the body budget.
    pub fn counts_bodies(self) -> bool {
        !matches!(self, Self::Failed | Self::Released)
    }
}

/// Phase B′ (operator decision 2026-11-02, W20 FINDINGS §2b′): an ordinary group's bodies are
/// constructed the way the game's own Reload constructs them at loader state 10, on the task
/// worker (`0x3ae5f0` allocation on the game thread, then init `0x6079d0` + construct `0x60b6b0`
/// as a queued task), instead of the synchronous `0x658af0` loop that stops the game thread for
/// the whole construction (console C116.2: 1.01 s). Two classes keep the synchronous build for
/// now, an operator-approved restriction dated 2026-11-02 (a follow-up covers them): a native
/// group (`0x658af0`'s PT/Aegis branches create the `+0x5a88` object) and the pair group or a
/// pair-lead window (the `R+0x50` batch rule under an exclusive reset batch). A Mii group left
/// that restriction by the operator's decision of 2026-11-08 (rc.6 group-speed item 2; console
/// F6: its synchronous build held the game thread 141 ms): its identity is written into `R`, the
/// entry and the D mirror before either mode starts and stays while it is the window, and the
/// Phase B Mii setup runs before the construction is queued, as state 6 runs before native's own
/// queued states 8-10 for a canonical Mii. `Err` names the reason the adapter logs with `mode=sync`.
pub fn async_build_eligible(members: Members, identity: bool) -> Result<(), &'static str> {
    match members.special() {
        Special::Mii => Ok(()),
        Special::Native(_) => Err("native group"),
        Special::Pair => Err("pair group"),
        Special::None if members.is_pair() => Err("pair window"),
        Special::None if identity => Err("identity"),
        Special::None if members.actor_count() != 1 => Err("actors"),
        Special::None => Ok(()),
    }
}

/// rc.6 item 3 (operator decision 2026-11-08): a Prefetched group's bodies built in the background, beside the live
/// window, so its first switch is a warm exchange. The construction binds to the entry by index (body init's
/// prepare_entry `0x17f1a60` at `0x607f30` writes the copy-registry slot of its cell from the worker), so only what the
/// queued build already serves (`async_build_eligible`): an ordinary single-actor group or a Mii group, while the
/// window's live member is not Kirby (D-KIRBY1: no bank key rests on its R). The live window (`window`, the attached
/// group): the body's construction tail reads the live cells 0/1 from the worker and takes the pair rule on R for `[Popo, Nana]`, which may
/// reset the whole R mid-play (SPECIAL-GROUPS §0 X1, `0x60de60-0x60de78`), and construct picks its info wrapper from the
/// live window's actor count (X4, `0x60d21c-0x60d270`: in re6 one-actor windows gave every body `FE+0xf0`; only the
/// pair's Nana took `FE+0x100`).
///
/// The live window (`window`, the attached group):
/// - a pair window (`window.is_pair()`: the pair-lead window or the pair group attached; a session that starts on Ice
///   Climbers) takes part when the construction tail's pair test is redirected for background bodies (`pair_tail`,
///   `0x60de68`, SPECIAL-GROUPS §0 X1), so their tails take the per-kind release instead of the pair rule on R (which may
///   reset the whole R mid-play); otherwise it defers the build ("pair window").
/// - a native window (Pokémon Trainer, Pyra/Mythra; a session that starts on one) takes part: one actor, its cells 0/1
///   never `[Popo, Nana]`; only an Ice Climbers body picks its info wrapper from the live kinds (X4, `0x60d1f0`), so the
///   pair group needs the X4 hook there (the cross cells below).
/// - a Mii window (`LiveIdentity::Mii`: P1 or the CPU on a Mii group; X6, primary 2026-11-09 04:22Z) takes part once the
///   owner's ordinary baseline is known (`baseline`, `Bank::ordinary_identity`): every body's costume and paths key on
///   `R+0x58`/`+0x64` (SPECIAL-GROUPS §0 X3: a Mii class forces costume 0), so the group's tick, key requests and worker
///   take the baseline words with its own header kinds (`own_words`; the worker through the shadow `R`). Without a
///   baseline it defers the build ("mii window"). A Mii group beside it waits ("mii beside a mii window"): an owner has
///   one Mii record (`staged_descriptor_for_owner`), and the group's native setup would publish its pair into the
///   record the live Mii reads.
///
/// The groups:
/// - a native group (SPECIAL-GROUPS §3) builds as an ordinary group does; its holder records come from native loader
///   state 11's step after its bodies settled (the adapter's settlement), straight into its saved window, and its
///   switch-time build stays `0x658af0`'s (`async_build_eligible`).
/// - the pair group (SPECIAL-GROUPS §2) builds here too. Beside a window that is not `[Popo, Nana]` its two tails take
///   the per-kind release `0x17e8610` on the live cells, which returns exactly the keys its prefetch registered, so no
///   `R+0x50` reference and no reset-batch permit are involved. N1 gives Nana Popo's context in the tick (the bank lends
///   the group's members there), the control init serves flags 6, and its warm attach is today's (N2). The switch-time
///   build stays synchronous (`async_build_eligible`).
/// - a Mii group (SPECIAL-GROUPS §1) builds when it carries its identity: the tick brackets `R`'s words, the entry's Mii
///   bytes and the D mirror and runs the native Mii setup, and the worker reads a shadow `R` (`native_body_bank`
///   `SHADOW_R`). A live Mii window defers it (above). A live native or pair window takes part (X5, primary 2026-11-09 02:42Z): of §0's live reads, the Mii body's costume
///   and paths read the shadow (X3), its cell byte is the group's (X2), its tail is the shared hook's (X1), and X4 and X5
///   touch only Ice Climbers and Aegis bodies; a pair window still needs the redirected tail, as every group does.
///
/// The two cross cells (primary 2026-11-09 00:52Z: "All groups in rc.6", so both must work; SPECIAL-GROUPS §0):
/// - a native group beside a live pair window needs only the redirected tail above (X1). X4 never applies to its kinds
///   (construct `0x60d1f0`: only a Popo or Nana body chooses an info wrapper), and X5 reads the group's own cell 1
///   (`background_partner`).
/// - the pair group beside a live native window needs the X4 hook (`info_x4`, `native_body_bank::background_info`): the
///   wrapper a background Popo or Nana body takes comes from its building group's words (`info_wrapper`), so Nana gets
///   her own `+0x100` as in her synchronous build; without it the build waits for another window.
///
/// Every other group keeps today's build at its first switch.
pub fn background_build_eligible(members: Members, identity: bool, window: Members, live: LiveIdentity, live_kirby: bool,
    pair_tail: bool, info_x4: bool) -> Result<(), &'static str> {
    let native = |m: Members| matches!(m.special(), Special::Native(_));
    if members.special() == Special::Mii && !identity { return Err("mii without identity"); }
    if members.special() != Special::Pair && (identity || !native(members)) { async_build_eligible(members, identity)?; }
    if let LiveIdentity::Mii { baseline } = live {
        if members.special() == Special::Mii { return Err("mii beside a mii window"); }
        if !baseline { return Err("mii window"); }
    }
    if live_kirby { return Err("live kirby"); }
    if window.is_pair() && !pair_tail { return Err("pair window"); }
    if native(window) && members.special() == Special::Pair && !info_x4 { return Err("pair group beside a native window without X4"); }
    Ok(())
}

/// The group a background build starts next: the first Prefetched group in plan order that is eligible
/// (`background_build_eligible`) beside the live `window`, from `(state, members, identity)`.
pub fn next_background(groups: impl Iterator<Item = (GroupState, Members, bool)>, window: Members, live: LiveIdentity,
    live_kirby: bool, pair_tail: bool, info_x4: bool) -> Option<usize> {
    groups.enumerate().find(|(_, (state, members, identity))| *state == GroupState::Prefetched
        && background_build_eligible(*members, *identity, window, live, live_kirby, pair_tail, info_x4).is_ok())
        .map(|(i, _)| i)
}

/// What a role's group lifecycle starts next (`native_body_bank::group_lifecycle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextStep {
    Prefetch(usize),
    Background(usize),
}

/// rc.6 item 3: every planned group's prefetch first, then the background builds of the prefetched ones; a group held
/// for capacity is retried only when no background build is owed. `start` is [`next_start`]'s group with its state and
/// whether its last memory admission deferred it ([`memory::Admission::Defer`]); `background` is [`next_background`]'s.
/// A deferred group waits for what admitted groups still hold, which their builds spend, so it ranks like a held one.
/// Ranked first, it kept the owed background builds from starting, and so waited on them for good: rc.7
/// `final-surface-i3-nt-7d720c68` (integration FLAG 3, 2026-11-10 21:53Z), both roles' second prefetch deferred and their
/// prefetched pair groups unbuilt for 170 s.
pub fn next_step(start: Option<(usize, GroupState, bool)>, background: Option<usize>) -> Option<NextStep> {
    match (start, background) {
        (Some((group, GroupState::Planned, false)), _) => Some(NextStep::Prefetch(group)),
        (_, Some(group)) => Some(NextStep::Background(group)),
        (Some((group, _, _)), None) => Some(NextStep::Prefetch(group)),
        (None, None) => None,
    }
}

/// Primary 2026-11-09 07:17Z (item 3 is about fast switching, and right after Training loads is when the builds run)
/// and 07:34Z (no timeout: exactly the steps that serve the press go first): a foreground request that waits for INSTALL
/// and for nothing else (a P1 or CPU press with the roster or a Mii owner, a hat request's source load) is served next.
/// A speculative step that would take INSTALL (a prefetch, a background build, an object re-arm, a pool preparation)
/// does not start while one waits, except the one that serves it ([`serving_step`]); the step in flight keeps INSTALL
/// until it ends, and the polls that end it still run. A request its owner keeps for anything else (the menu open, a
/// recording, a Kirby not yet idle) is no foreground request, so nothing waits on it.
pub fn speculative_step_admitted(foreground: bool) -> Result<(), &'static str> {
    if foreground { Err("a foreground request is pending") } else { Ok(()) }
}

/// The speculative step a waiting press needs on its target's group ([`serving_step`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serving {
    Prefetch,
    ObjectRearm,
}

/// [`Serving`] for the group holding the press's target, in `state`. Its prefetch while it is planned or held for
/// capacity and publishable (a Mii group's CharInfo is on the owner's record): a target in no reachable group takes the
/// canonical Reload, which destroys the window and every group. Its object re-arm while a built Pokémon Trainer group
/// lacks its object kind (`rearm_due`, [`object_rearm_due`]): without it [`object_ready`] fails and so does the warm
/// attach. Nothing for a prefetching group (its poll ends it) or a prefetched or built one (reachable already: its
/// background build or pool preparation would only make the press wait longer).
pub fn serving_step(state: GroupState, publishable: bool, rearm_due: bool) -> Option<Serving> {
    match state {
        _ if state.startable() => publishable.then_some(Serving::Prefetch),
        GroupState::Built if rearm_due => Some(Serving::ObjectRearm),
        _ => None,
    }
}

/// The live window's Mii identity as a background build sees it (X6, `background_build_eligible`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveIdentity {
    /// An ordinary, native or pair window: `R` holds the owner's ordinary words.
    Ordinary,
    /// A Mii group's window; `baseline`: the owner's ordinary words are known (`Bank::ordinary_identity`).
    Mii { baseline: bool },
}

impl LiveIdentity {
    /// From the window's identity (`Bank::identity.is_some()`) and the baseline's (`Bank::ordinary_identity.is_some()`).
    pub fn of(window_identity: bool, baseline: bool) -> Self {
        if window_identity { Self::Mii { baseline } } else { Self::Ordinary }
    }
}

/// Inbox 00:18Z (a console-crash class, at the base too): a Pokémon Trainer group's article pool is retired at its detach
/// (`swap_model::special_articles`) and rebuilt at its next warm attach (`0x3a6270`), and the trainer article reads its
/// file through the object kind `0x72` (`0x355fea0(…, 0x372, …)`; Eden rc6-bg-pt-base1: Unmapped Read8 @ 0 at
/// main+0x35609c0), which the bank returned after the build. A Built PT group whose pool (`pool` articles in its saved
/// holder) was retired takes the kind again while it is detached, unless it holds it already (`held`).
pub fn object_rearm_due(state: GroupState, members: Members, pool: u64, held: bool) -> bool {
    state == GroupState::Built && members.holder_step() == Some(HolderStep::Trainer) && pool == 0 && !held
}

/// [`object_rearm_due`]'s other half: such a group is reachable warm only while it holds its object kind (its attach
/// rebuilds the pool); a filled pool, an Aegis holder (never retired) and every other group need nothing.
pub fn object_ready(members: Members, pool: u64, held: bool) -> bool {
    members.holder_step() != Some(HolderStep::Trainer) || pool > 0 || held
}

/// How the notice counts a group: a Prefetched group a background build still owes (`owed`) is pending work, as a
/// prefetch in flight is, so the notice reads "Loading x/y" until it is built; every other state is shown as it is.
pub fn shown_state(state: GroupState, owed: bool) -> GroupState {
    if state == GroupState::Prefetched && owed { GroupState::Prefetching } else { state }
}

/// The entry-info bytes a background build's entry-level third rewrites (`0x654dc0`: the dispatcher at `+0x8..+0x20`,
/// the descriptor copies `+0x28..+0x92`; body init's `+0x3a0..+0x3ac`), saved before it and put back after it, as
/// `(offset, length)`. The window's own `Window` record covers every entry field.
pub const BACKGROUND_INFO_SPANS: [(usize, usize); 2] = [(0x08, 0x90), (0x3a0, 0x10)];

/// The game's construction task object (`native_entry_dispatch_6556b0.c:215-250`, loader state
/// 10): 0x90 bytes from the heap `0x392e290(0x10, 0x90)`; `+0x00` the body `0x3ae5f0` returned;
/// `+0x10` the scheduler handle passed to `0x3549720(task+0x10, 0x3cb190, 2, 1)`; `+0x20` the
/// pending count the loader polls at state 0xb (`*(int*)(task+0x20) == 0`); `+0x24` the queue id
/// (`1`, the index into `0x5333f58` the wait `0x354ccd0` uses); `+0x50` the functor's in-place
/// storage (`0x3cb050` swaps `{vtable 0x4f69bb8, task}` into it); `+0x70` the functor pointer the
/// callback reads (`task+0x10+0x60`); `+0x80` the cancel byte (`task+0x10+0x70`). State 10 zeroes
/// `+0x10`, `+0x18`, `+0x20..+0x27` (then `+0x24 = 1`), `+0x70` and `+0x80` before the swap.
pub mod construction_task {
    pub const SIZE: usize = 0x90;
    pub const ALIGN: usize = 0x10;
    pub const BODY: usize = 0x00;
    pub const HANDLE: usize = 0x10;
    pub const PENDING: usize = 0x20;
    pub const QUEUE: usize = 0x24;
    pub const QUEUE_ID: u32 = 1;
    pub const FUNCTOR: usize = 0x50;
    pub const FUNCTOR_PTR: usize = 0x70;
    #[cfg(test)]
    pub const CANCEL: usize = 0x80;
    /// The two scheduler arguments state 10 passes after the callback.
    pub const SCHEDULE_ARGS: (u32, u32) = (2, 1);
    /// Every queued construction ran its init and construct: the callback cleared each pending count.
    pub fn settled(pending: &[i32]) -> bool {
        pending.iter().all(|p| *p <= 0)
    }
}

/// Native loader state 11's Pokémon Trainer task (`0x650fc4-0x6512c8`): `construction_task`'s layout (handle, pending
/// count, queue id, functor, cancel byte) in 0xa0 bytes, with the holder at `+0x00`, the owner at `+0x08`, the role's
/// costume byte `FE+0x41` at `+0x90`, [`trainer_task::flag`] at `+0x91` and `+0x92 = 1` (the build; 0 is the teardown
/// variant). Its functor (`0x651a30`, vtable `0x4f81498`) reads only the task, `R` by owner and globals.
pub mod trainer_task {
    pub const SIZE: usize = 0xa0;
    pub const HOLDER: usize = 0x00;
    pub const OWNER: usize = 0x08;
    pub const COSTUME: usize = 0x90;
    pub const FLAG: usize = 0x91;
    pub const BUILD: usize = 0x92;
    /// The 1-byte object state 11 allocates for `+0x5a98` (`0x650f74`).
    pub const OBJECT_SIZE: usize = 1;
    /// `+0x91`: `FE+0x52 != 0 || FE+0x53 == 0` (`0x651028-0x651040`).
    pub fn flag(fe52: u8, fe53: u8) -> u8 {
        u8::from(fe52 != 0 || fe53 == 0)
    }
}

/// P3-0 tick-gap meter over one swap (request → the bank's Ready): the largest gap between two
/// consecutive entry pre-service ticks is the stall the game thread took. A synchronous build
/// shows its whole `us=` as one gap; the asynchronous build must show about one frame. The
/// console's `[PreloadMetrics] … frames=0 max_gap_us=na` cannot see it (frame callbacks are not
/// counted while a roster request suspends practice), so the meter lives on the entry tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TickGap {
    pub last_us: u64,
    pub max_gap_us: u64,
    pub ticks: u32,
    pub armed: bool,
}

impl TickGap {
    /// Arm at the request; the first tick after it already counts.
    pub fn arm(now_us: u64) -> Self {
        Self { last_us: now_us, max_gap_us: 0, ticks: 0, armed: true }
    }

    pub fn observe(&mut self, now_us: u64) {
        if !self.armed {
            return;
        }
        let gap = now_us.saturating_sub(self.last_us);
        if gap > self.max_gap_us {
            self.max_gap_us = gap;
        }
        self.last_us = now_us;
        self.ticks = self.ticks.saturating_add(1);
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

// `swap_commit_index`, `swap_commit_index_in`: `swap_model::capabilities` (re-exported above),
// derived from the window's `ExitRule` (a one-member window commits to its own cell);
// `Members::swap_commit_index_for` is the same rule seen from the window.

/// Sizes of the groups that `bodies` newly chosen kinds form, in cycle order:
/// triples, then a pair; a one-member group only when exactly one body is left.
/// Four left over (or `3k+1`) become two pairs instead of a triple and a single:
/// every group keeps an in-window neighbour, so the lap's exits stay next-cell
/// commits and the one-member self-commit (`ExitRule::SelfCell`) is taken only
/// by a single that no pair can use. Zero: no group.
pub fn group_sizes(bodies: usize) -> [usize; MAX_EXTRA_GROUPS] {
    let mut sizes = [0; MAX_EXTRA_GROUPS];
    let mut remaining = bodies.min(MAX_EXTRA_GROUPS * NATIVE_MEMBERS);
    for size in sizes.iter_mut() {
        if remaining == 0 {
            break;
        }
        *size = if remaining == NATIVE_MEMBERS + 1 { NATIVE_MEMBERS - 1 } else { remaining.min(NATIVE_MEMBERS) };
        remaining -= *size;
    }
    sizes
}

/// rc.6.1 target-first (primary 2026-11-10 12:16Z, the operator's console play test): the window a switch's Reload builds
/// when group swap is on and the shared pool has room for any other body (`pool`: `ready_fighters` less the two active
/// bodies). It is the picked fighter alone ([`Members::target_only`]), so the Reload waits for one body. The members a full
/// window would have taken are the role's nearest group candidates ([`plan_extra_groups`] plans a one-member window's role
/// as usual) and build in the background under the same guards. `None` keeps today's window: group swap off (nothing
/// would build the rest), no pool, the normal pair (its pair-lead window), and any form without a one-member window.
pub fn target_first(forms: Forms, groups: bool, pool: usize) -> Option<Members> {
    if !groups || pool == 0 {
        return None;
    }
    Members::target_only(forms)
}

/// The window a switch's Reload builds (`native_body_bank::proposed`): target-first applies to a window rebuild only. When
/// the lane gives the target no `full` window (no cycle neighbour, or only special-group ones such as Pokémon Trainer beside
/// Kirby) the switch keeps its canonical Reload, with no bank, as before target-first. A one-member bank there let the
/// planner add groups no window had brought: integration FLAG 2 2026-11-10 17:56Z, P1's PT group planned beside a live
/// Kirby and deferred by D-KIRBY1, so the notice stayed at "Loading 4/7".
pub fn reload_window(full: Option<Members>, forms: Forms, groups: bool, pool: usize, live_gate: Option<u32>) -> Option<Members> {
    let full = full?;
    Some(target_first(forms, groups && groups_load_beside(live_gate), pool).unwrap_or(full))
}

/// Whether a target's other members can load as groups beside it once it is live, so that [`target_first`] may leave
/// them to the background. `live_gate` is the live kind D-KIRBY1's gates see for the target: the target itself, or
/// with rc.7 F1 `d_kirby1_live(Some(target), ..)`, which is `None` once the bank owns the G68 step. D-KIRBY1
/// ([`prefetch_admitted`]) defers every group prefetch beside a live Kirby. So while it is in force, a Kirby target keeps
/// its full window: neighbours planned after him could never load while he is on screen, and the notice would read
/// "Loading" until P1 left him (integration 2026-11-10 18:16Z; A's `tf-direct-after-3f7dca0c` #443–#446).
pub fn groups_load_beside(live_gate: Option<u32>) -> bool {
    prefetch_admitted(live_gate).is_ok()
}

/// The group a [`target_first`] Reload's neighbours build as: the members the `full` window would have held besides
/// `target`, laid out like any group (Kirby at cell 0). The planner plans it first, ahead of the rest of the lane, so it
/// is admitted on the same cost the full window's supplements were (`memory::window_extras` over those members is
/// `admit(group_cost(..))`). A kind the planner would not group (`group_member_kind`, the excluded original) is left out;
/// `None` when nothing is left.
pub fn neighbour_group(full: Members, target: u32, policy: GroupPolicy, exclude: Option<u32>) -> Option<Members> {
    let mut kinds = [u32::MAX; NATIVE_MEMBERS];
    let mut count = 0;
    for &kind in &full.kinds()[..full.count()] {
        if kind != target && exclude != Some(kind) && group_member_kind(kind, policy) {
            kinds[count] = kind;
            count += 1;
        }
    }
    if count == 0 { None } else { Members::laid_out(&kinds[..count]) }
}

/// rc.12, the CPU-cycle thrash, option (a′) (primary 2026-11-13 17:05Z; boot 3 `practice-diagnostics.previous.log`
/// #163–#199: each lap's target-first Reload tore down the role's Mii group `[4a]` and built it again, a 0.46 s prefetch
/// and a 0.22 s build): a detached group survives its role's target-first Reload (`target_first` is that Reload's
/// target) when it is Built, holds no key (a Built group's tails ran; a re-acquired Pokémon Trainer object kind is a key),
/// is not leaving its cycle and does not hold the target. Loader state 6 needs only `R+0x18 == 0`, which such a group
/// leaves true, and the native Reload destroys the attached window alone. Every other group is released first, as
/// before, and a canonical or full-window Reload (`None`) keeps nothing. A Mii group is released first too: Eden
/// rc12-int2-f44fac2f-1 crashed on the kept `[4a]`'s warm attach after the Reload (rc.13 item).
pub fn kept_across_reload(state: GroupState, members: Members, leaving: bool, holds_object: bool, target_first: Option<u32>) -> bool {
    let Some(target) = target_first else { return false; };
    state == GroupState::Built && !leaving && !holds_object && members.special() != Special::Mii && members.index_of(target).is_none()
}

/// The target-first neighbours' admission (`native_body_bank::plan_groups`): the seed fits in the free pool and beside
/// the bodies already charged, the other role's and this role's own groups kept across the Reload
/// ([`kept_across_reload`]).
pub fn neighbours_fit(seed: usize, own: usize, other: usize, pool_free: usize, group_pool: usize) -> bool {
    seed <= pool_free && seed + own + other <= group_pool
}

/// A [`neighbour_group`] seed with its window's target as one word, which the planner reads without a lock
/// (`native_body_bank::NEIGHBOURS`): 16 bits each for the target and up to two kinds, the count above. `u64::MAX`: none.
pub fn pack_neighbours(target: u32, seed: Option<Members>) -> u64 {
    let Some(seed) = seed.filter(|s| s.count() < NATIVE_MEMBERS && target < 0xffff) else { return u64::MAX; };
    let kinds = seed.kinds();
    u64::from(target) | (u64::from(kinds[0] & 0xffff) << 16) | (u64::from(kinds[1] & 0xffff) << 32) | ((seed.count() as u64) << 48)
}

/// The seed [`pack_neighbours`] stored, when `window` is its target's one-member window; `None` for any other window.
pub fn unpack_neighbours(word: u64, window: Members) -> Option<Members> {
    let field = |shift: u32| ((word >> shift) & 0xffff) as u32;
    let count = (word >> 48) as usize;
    if word == u64::MAX || window.count() != 1 || window.special() != Special::None || field(0) != window.kinds()[0]
        || !(1..NATIVE_MEMBERS).contains(&count) {
        return None;
    }
    Members::new(&[field(16), field(32)][..count])
}

/// The extra bodies the next Reload's window builds for `forms`, out of the `window` the lane would give it: none under
/// [`target_first`] (its neighbours are charged as group bodies), otherwise the window's members after the target. The
/// planner reserves these before the helpers (`plan_budget`).
pub fn reload_extras(window: Members, forms: Forms, groups: bool, pool: usize, live_gate: Option<u32>) -> usize {
    if reload_window(Some(window), forms, groups, pool, live_gate) != Some(window) { 0 } else { window.count() - 1 }
}

/// The members the Fighters notice counts for the window a role's next swap prepares (`plan_budget`): every one besides
/// the target. Under [`target_first`] that Reload builds the target alone ([`reload_extras`] is 0, the physical
/// reservation), but the members become its first background group once it is Ready ([`neighbour_group`]), so the swap
/// still starts them. Counting the physical extras instead left a role without a bank planning nothing, and the parked
/// seats with it (`hat_slots::with_seats`): `[PreloadStatus] state=Hidden planned=0` on `889919e1` (`of-seat-notice`).
pub fn notice_extras(window: Members) -> usize {
    window.count() - 1
}

/// The planned targets of the window a role's next swap prepares, as the notice counts them (`plan_budget`'s `next`):
/// the target, then the first `extras` other members (`notice_extras`, reserved against the window pool; a pair-lead
/// window lists all of its own), counted by [`next_window_targets`].
pub fn next_window_plan(window: Members, target: u32, extras: usize, original: Option<u32>, grouped: impl Fn(u32) -> bool) -> u8 {
    let (kinds, count) = window.logical_kinds();
    let extras = if window.is_pair_lead() { count } else { extras };
    let mut shown = [u32::MAX; NATIVE_MEMBERS + 1];
    let mut len = 0;
    for kind in std::iter::once(target).chain(kinds[..count].iter().copied().filter(|kind| *kind != target).take(extras)) {
        shown[len] = kind;
        len += 1;
    }
    next_window_targets(&shown[..len], target, original, grouped)
}

/// rc.6.1 target-first: a canonical Reload replaces a Ready one-member ordinary window whose words it repeats (count 1,
/// the same kind, one actor). The bank's descriptor check still matches such a Reload, so the bank is retired before
/// native loads it; otherwise the construct seam would write the bank flag onto the canonical entry and the next tick would
/// take its bodies as the window's. A bank Reload (its incoming bank already Building), a multi-body or Mii window and
/// any other shape are left to the existing replacement check.
pub fn canonical_replaces_window(window: Members, phase: Phase, count: u32, kinds: [u32; NATIVE_MEMBERS], actors: u32) -> bool {
    phase == Phase::Ready && window.special() == Special::None && window.count() == 1
        && count == 1 && kinds == window.kinds() && actors == window.actor_count()
}

/// Prefetch poll: ready wins; a timeout fails the group; otherwise keep waiting.
pub fn prefetch_outcome(ready: bool, elapsed_us: u64, timeout_us: u64) -> GroupState {
    if ready {
        GroupState::Prefetched
    } else if elapsed_us >= timeout_us {
        GroupState::Failed
    } else {
        GroupState::Prefetching
    }
}

/// One shared ready-fighter budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupBudget {
    /// Total built bodies across both visible roles, including the active fighters. Kirby hats are not charged here:
    /// they have their own budget (P-HOT, `preload::hat_slots`, "Ready Hats").
    pub ready_fighters: usize,
    /// Fighter-pool bodies available after the reserve (capacity − live − reserve).
    pub pool_free: usize,
    /// Special groups the planner may form; each is placed whole (PT 3, Aegis 2,
    /// pair 2 bodies) at its lane depth, never merged with ordinary kinds.
    pub special: SpecialPolicy,
}

/// Plan new extra groups for both roles from one shared budget.
///
/// `windows` are the current window groups (`None`: that role has no bank and
/// gets nothing). `existing` are extra groups already planned or built; they stay
/// and are charged. `upcoming` is each role's ordered cycle lookahead; `exclude`
/// is a per-role kind never placed (the canonical original restores canonically).
/// Priority is the next neighbour of each role first, interleaved by depth.
/// Groups hold one to three members. A leftover single body first completes the
/// other role's pair or single; only a body no pair can use forms a one-member
/// group, and four leftover bodies form two pairs ([`group_sizes`]). A one-member
/// window is left by its self-commit like any window is left by its commit
/// ([`swap_commit_index_in`]), so a role in a one-member window plans as usual.
/// This is the default policy; see [`plan_extra_groups_with`].
#[cfg(test)]
pub fn plan_extra_groups(
    windows: [Option<Members>; 2],
    existing: [&[Members]; 2],
    upcoming: [&[u32]; 2],
    exclude: [Option<u32>; 2],
    budget: GroupBudget,
) -> [[Option<Members>; MAX_EXTRA_GROUPS]; 2] {
    plan_extra_groups_with(GroupPolicy::DEFAULT, windows, existing, upcoming, exclude, budget)
}

/// [`plan_extra_groups`] under `policy`: a pair-lead window may own groups
/// (`ic_pair_groups`), and Solo Popo may be chosen as a member
/// (`solo_popo_groups`). Each group is laid out by [`Members::laid_out`]: a member
/// whose row pins cell 0 (Kirby, Solo Popo) takes cell 0 and the others keep
/// their lane order; when a chunk holds both, the later one in lane order forms
/// its own one-member group in the next slot.
pub fn plan_extra_groups_with(
    policy: GroupPolicy,
    windows: [Option<Members>; 2],
    existing: [&[Members]; 2],
    upcoming: [&[u32]; 2],
    exclude: [Option<u32>; 2],
    budget: GroupBudget,
) -> [[Option<Members>; MAX_EXTRA_GROUPS]; 2] {
    let mut result = [[None; MAX_EXTRA_GROUPS]; 2];
    // Every visible role has one active body even without a bank (Kirby, a Mii,
    // a native group): count it so hats and helper slots agree with the planner.
    let used: usize = windows.iter().map(|window| window.map_or(1, |members| members.count())).sum::<usize>()
        + existing.iter().flat_map(|groups| groups.iter()).map(|members| members.count()).sum::<usize>();
    let mut remaining = budget.ready_fighters.saturating_sub(used).min(budget.pool_free);
    // Borrow the complete lanes instead of allocating temporary candidate lists.
    // Prefix checks preserve first-occurrence order without imposing a lane cap.
    let eligible = |role: usize, kind: u32| {
        windows[role].filter(|window| window_swappable_with(*window, policy))
            .is_some_and(|window| exclude[role] != Some(kind)
                && window.index_of(kind).is_none()
                && !existing[role].iter().any(|group| group.index_of(kind).is_some()))
    };
    let candidates = |role: usize| upcoming[role].iter().copied().enumerate()
        .filter(move |&(depth, kind)| eligible(role, kind)
            && Members::special_group(kind, budget.special).is_none()
            && group_member_kind(kind, policy)
            && !upcoming[role][..depth].contains(&kind));
    let specials = |role: usize| upcoming[role].iter().copied().enumerate()
        .filter_map(move |(depth, kind)| {
            if !eligible(role, kind) { return None; }
            let group = Members::special_group(kind, budget.special)?;
            let is_mii = group.special() == Special::Mii;
            // One Mii group per role, the live Mii window included (inbox 06:16Z).
            if (is_mii && (existing[role].iter().any(|held| held.special() == Special::Mii)
                    || windows[role].is_some_and(|window| window.special() == Special::Mii)))
                || upcoming[role][..depth].iter().copied()
                    .filter(|&prior| eligible(role, prior))
                    .filter_map(|prior| Members::special_group(prior, budget.special))
                    .any(|held| held.kinds()[0] == group.kinds()[0]
                        || (is_mii && held.special() == Special::Mii))
            { return None; }
            Some((depth, group))
        });
    let mut room = [0usize; 2];
    let mut group_slots = [0usize; 2];
    for role in 0..2 {
        if !windows[role].is_some_and(|window| window_swappable_with(window, policy)) { continue; }
        group_slots[role] = MAX_EXTRA_GROUPS.saturating_sub(existing[role].len());
        room[role] = group_slots[role] * NATIVE_MEMBERS;
    }
    // Special groups are taken whole at their own lane depth, interleaved with
    // the ordinary candidates; one that no longer fits the remaining bodies or a
    // free group slot is skipped, never split.
    use crate::engine::compat::fixed::FixedList;
    let mut chosen_special = [FixedList::<Members, MAX_EXTRA_GROUPS>::new(Members::pair_group()); 2];
    let mut scursor = [0usize; 2];
    // These are output storage bounds: at most three native cells per free
    // group, already enforced by room/group_slots below; inputs remain uncapped.
    let mut chosen = [FixedList::<u32, { MAX_EXTRA_GROUPS * NATIVE_MEMBERS }>::new(0); 2];
    let mut cursor = [0usize; 2];
    let take = |role: usize, chosen: &mut [FixedList<u32, { MAX_EXTRA_GROUPS * NATIVE_MEMBERS }>; 2], cursor: &mut [usize; 2], remaining: &mut usize| -> bool {
        if *remaining == 0 || chosen[role].len() >= room[role] {
            return false;
        }
        let Some((_, kind)) = candidates(role).nth(cursor[role]) else { return false; };
        chosen[role].push(kind);
        cursor[role] += 1;
        *remaining -= 1;
        true
    };
    let deepest = upcoming.iter().map(|lane| lane.len()).max().unwrap_or(0);
    for depth in 0..deepest {
        for role in 0..2 {
            if candidates(role).nth(cursor[role]).is_some_and(|(d, _)| d == depth) {
                take(role, &mut chosen, &mut cursor, &mut remaining);
            } else if let Some((_, group)) = specials(role).nth(scursor[role]).filter(|(d, _)| *d == depth) {
                scursor[role] += 1;
                let slots_used = chosen_special[role].len() + chosen[role].len().div_ceil(NATIVE_MEMBERS);
                if remaining >= group.count() && slots_used < group_slots[role] {
                    chosen_special[role].push(group);
                    remaining -= group.count();
                }
            }
        }
    }
    // A leftover single body is worth more as the third member of the other
    // role's pair, or as a pair with that role's single: return it first.
    for role in 0..2 {
        if chosen[role].len() % NATIVE_MEMBERS == 1 {
            chosen[role].pop();
            cursor[role] -= 1;
            remaining += 1;
        }
    }
    for role in 0..2 {
        if chosen[role].len() % NATIVE_MEMBERS == 2 {
            take(role, &mut chosen, &mut cursor, &mut remaining);
        }
    }
    for role in 0..2 {
        while remaining >= 2
            && candidates(role).nth(cursor[role] + 1).is_some()
            && chosen[role].len() + 2 <= room[role]
            && chosen[role].len().is_multiple_of(NATIVE_MEMBERS)
        {
            take(role, &mut chosen, &mut cursor, &mut remaining);
            take(role, &mut chosen, &mut cursor, &mut remaining);
        }
    }
    // What no pair can use still buys a one-member group, nearest neighbour first.
    while remaining > 0 {
        let Some(role) = (0..2)
            .filter(|&role| candidates(role).nth(cursor[role]).is_some() && chosen[role].len() < room[role])
            .min_by_key(|&role| candidates(role).nth(cursor[role]).map(|(depth, _)| depth)) else { break; };
        take(role, &mut chosen, &mut cursor, &mut remaining);
    }
    for role in 0..2 {
        let mut slot = 0;
        for group in chosen_special[role].iter() {
            if slot >= MAX_EXTRA_GROUPS { break; }
            result[role][slot] = Some(*group);
            slot += 1;
        }
        let mut start = 0;
        for size in group_sizes(chosen[role].len()).iter() {
            if *size == 0 || slot >= MAX_EXTRA_GROUPS {
                break;
            }
            // A member whose row pins cell 0 (Kirby, Solo Popo) is laid out there by
            // `Members::laid_out`; the others keep lane order. When the chunk holds a second
            // pinned member, the later one in lane order forms its own one-member group in the
            // next slot rather than an entry native never builds (`CellRule::PinCell0`).
            let mut group = [u32::MAX; NATIVE_MEMBERS];
            let mut filled = 0;
            let mut deferred = None;
            let mut pinned = false;
            for &kind in &chosen[role][start..start + size] {
                if pins_cell0(kind) {
                    if pinned {
                        deferred = Some(kind);
                        continue;
                    }
                    pinned = true;
                }
                group[filled] = kind;
                filled += 1;
            }
            result[role][slot] = Members::laid_out(&group[..filled]);
            start += size;
            slot += 1;
            if let Some(kind) = deferred {
                if slot < MAX_EXTRA_GROUPS {
                    result[role][slot] = Members::new(&[kind]);
                    slot += 1;
                }
            }
        }
    }
    result
}

/// Ready-UI contribution of the extra groups: every planned member stays in the
/// denominator; only a built group's members are ready; failed groups stay counted.
/// A prefetched group is loaded but not built: its first switch needs no reload (it is
/// reachable) yet is not instant, so it is counted apart from `ready` and from the
/// groups whose load is still owed (`pending`). A held group waits for memory or pool bodies
/// and is counted apart (`held`, with the reason; memory outranks the pool). `planned == ready +
/// prefetched + failed + pending + held`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupCounts {
    pub planned: u8,
    pub ready: u8,
    pub prefetched: u8,
    pub failed: u8,
    pub pending: u8,
    pub held: u8,
    pub hold: Option<Hold>,
}

/// The reason a count of held fighters shows: memory when any group waits for it.
pub fn merge_hold(a: Option<Hold>, b: Option<Hold>) -> Option<Hold> {
    match (a, b) {
        (Some(Hold::Memory), _) | (_, Some(Hold::Memory)) => Some(Hold::Memory),
        (Some(Hold::Pool), _) | (_, Some(Hold::Pool)) => Some(Hold::Pool),
        (None, None) => None,
    }
}

#[cfg(test)]
pub fn count_extra_groups(groups: &[(Members, GroupState)]) -> GroupCounts {
    count_extra_groups_iter(groups.iter().copied())
}

#[cfg(test)]
pub fn count_extra_groups_iter(groups: impl IntoIterator<Item = (Members, GroupState)>) -> GroupCounts {
    count_group_members(groups.into_iter().map(|(members, state)| (members.count() as u8, state)))
}

/// Operator decision 2026-11-08 23:42Z: the members of a parked group (left the cycle, freed at the next pause) that its
/// role's cycle still has. They stay warm until the pause, so the notice and the planner count them; the ones that left
/// count nowhere (rc6-hot-xps18). The [`group_left_cycle`] rules: Nana follows Popo, a native group is whole while any
/// of its forms is wanted.
pub fn kept_members(members: Members, wanted: &[u32]) -> u8 {
    let kinds = &members.kinds()[..members.count()];
    match members.special() {
        Special::Native(_) => if kinds.iter().any(|kind| wanted.contains(kind)) { members.count() as u8 } else { 0 },
        _ => kinds.iter().filter(|kind| wanted.contains(kind) || (**kind == NANA && wanted.contains(&POPO))).count() as u8,
    }
}

/// Operator decision 2026-11-08 23:42Z: the new members the parked groups' room is for. `after_pause` is what the planner
/// places with the parked groups gone; `held` says whether any group (parked ones included) holds a kind now. Every
/// member it places that nothing holds is new. Since the operator rule of 2026-11-09 13:21Z they never wait for the
/// pause: [`release_for_new_members`] frees parked groups now when the plan beside them leaves these out.
pub fn waiting_for_pause(after_pause: &[Option<Members>], held: impl Fn(u32) -> bool) -> u8 {
    after_pause.iter().flatten()
        .flat_map(|members| { let kinds = members.kinds(); (0..members.count()).map(move |i| kinds[i]) })
        .filter(|kind| !held(*kind)).count().min(usize::from(u8::MAX)) as u8
}

impl GroupCounts {
    /// Operator rule 2026-11-09 (inbox 13:21Z): `members` new members that load once the parked groups marked to go now
    /// are freed ([`release_for_new_members`]): planned, and shown loading, never waiting.
    pub fn with_release(mut self, members: u8) -> GroupCounts {
        self.planned = self.planned.saturating_add(members);
        self.pending = self.pending.saturating_add(members);
        self
    }
}

/// Operator rule 2026-11-09 (inbox 13:21Z, "If it's already full it should just release so that it takes effect
/// accepting the hitch"; it replaces the pause wait of the 2026-11-08 23:42Z decision): after a cycle change the new
/// members load at once. The groups parked for the pause stay until it, unless the new members need their room (the
/// shared Ready Fighters budget, the fighter pool, a group slot).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReleaseNow {
    /// Per role, one bit per `parked` index: the parked groups to free now.
    pub groups: [u8; 2],
    /// Per role, the new members that load once they are freed (shown loading).
    pub members: [u8; 2],
}

fn group_kinds(groups: &[Option<Members>]) -> impl Iterator<Item = u32> + '_ {
    groups.iter().flatten().flat_map(|members| { let kinds = members.kinds(); (0..members.count()).map(move |i| kinds[i]) })
}

/// [`ReleaseNow`] for `parked` (each role's parked groups with their [`kept_members`]) beside `kept` (the groups that
/// stay, the ones this pass planned included); `held(role, kind)`: a group that is not parked holds `kind`.
/// `plan(existing, freed)` is the planner's [`plan_extra_groups_with`] with `existing` for the groups already there and
/// `freed` parked bodies added to the pool it has while they are live.
///
/// The members to load are the ones the plan with every parked body free places that no group holds now
/// ([`waiting_for_pause`]). When the plan beside the parked groups leaves some of them out, the parked groups to free
/// are the set beside which the plan places them all, with the fewest members the new cycle still has (served warm
/// until the pause), then the fewest bodies (native's destroy holds the game thread 75-134 ms per body), then the
/// fewest groups. Freeing them all always does, so nothing waits for a pause.
pub fn release_for_new_members(
    kept: [&[Members]; 2],
    parked: [&[(Members, u8)]; 2],
    held: impl Fn(usize, u32) -> bool,
    plan_with: impl Fn([&[Members]; 2], usize) -> [[Option<Members>; MAX_EXTRA_GROUPS]; 2],
) -> ReleaseNow {
    use crate::engine::compat::fixed::FixedList;
    let freed = |mask: [u8; 2], role: usize, index: usize| mask[role] & (1 << index) != 0;
    let plan = |mask: [u8; 2]| {
        let mut existing = [FixedList::<Members, { 2 * MAX_EXTRA_GROUPS }>::new(Members::pair_group()); 2];
        let mut bodies = 0usize;
        for role in 0..2 {
            for members in kept[role] { existing[role].push(*members); }
            for (index, (members, _)) in parked[role].iter().enumerate() {
                if freed(mask, role, index) { bodies += members.count(); } else { existing[role].push(*members); }
            }
        }
        plan_with([&existing[0], &existing[1]], bodies)
    };
    let all = [0, 1].map(|role: usize| ((1u16 << parked[role].len().min(8)) - 1) as u8);
    if all == [0, 0] { return ReleaseNow::default(); }
    let after = plan(all);
    let holds = |mask: [u8; 2], role: usize, kind: u32| held(role, kind)
        || parked[role].iter().enumerate().any(|(index, (members, _))| !freed(mask, role, index) && members.index_of(kind).is_some());
    let waiting = |mask: [u8; 2]| -> usize {
        let placed = plan(mask);
        (0..2).map(|role| group_kinds(&after[role])
            .filter(|kind| !holds(mask, role, *kind) && !group_kinds(&placed[role]).any(|k| k == *kind)).count()).sum()
    };
    let members = [0, 1].map(|role: usize| waiting_for_pause(&after[role], |kind| holds([0, 0], role, kind)));
    if members == [0, 0] || waiting([0, 0]) == 0 { return ReleaseNow::default(); }
    let mut best: Option<((usize, usize, u32), [u8; 2])> = None;
    for m0 in 0..=all[0] {
        for m1 in 0..=all[1] {
            let mask = [m0, m1];
            if mask == [0, 0] { continue; }
            let cost = (0..2).flat_map(|role| parked[role].iter().enumerate().filter(move |(index, _)| freed(mask, role, *index)))
                .fold((0usize, 0usize, 0u32), |(keeps, bodies, groups), (_, (members, kept))|
                    (keeps + usize::from(*kept), bodies + members.count(), groups + 1));
            if best.is_some_and(|(least, _)| least <= cost) || waiting(mask) != 0 { continue; }
            best = Some((cost, mask));
        }
    }
    ReleaseNow { groups: best.map_or(all, |(_, mask)| mask), members }
}

/// Operator rule 2026-11-09 (inbox 13:21Z): whether the entry tick frees a leaving group now. A parked one
/// ([`leaves_at_pause`]) waits for the pause unless `release_now` (the new members need its room,
/// [`release_for_new_members`], or a prefetch was refused for the memory or pool bodies it holds, [`capacity_release`]);
/// a prefetch in flight settles first; the rest hold keys at most and go at once.
pub fn leaves_now(state: GroupState, release_now: bool) -> bool {
    state != GroupState::Prefetching && (!leaves_at_pause(state) || release_now)
}

/// Operator rule 2026-11-09 (inbox 13:21Z): a prefetch refused for process memory or fighter-pool bodies while groups are
/// parked frees one of them now, the one with the fewest members its cycle keeps, then the fewest bodies, the first of
/// equals. `parked` yields `(key, members, kept)`. None: the refusal is a real shortage, shown "waiting for memory".
/// Operator rule 2026-11-09 16:06Z: nothing is freed while an earlier free's room is still arriving (`settled` false,
/// [`ReleaseWatch`]), so a group that keeps members of the new cycle goes only when nothing unneeded is parked and the
/// room still lacks after the earlier releases settled.
pub fn capacity_release<K: Copy>(settled: bool, parked: impl Iterator<Item = (K, Members, u8)>) -> Option<K> {
    if !settled { return None; }
    parked.min_by_key(|(_, members, kept)| (*kept, members.count())).map(|(key, _, _)| key)
}

/// Operator rule 2026-11-09 16:06Z (Eden `final-surface-i3-pf-1261f56c` `x6-cc`): a parked group freed now gives its
/// memory back over the next second or two (Eden `final-surface-i3-pf-b04856b2`: about 113 MB 2 s after
/// `[Samus, Yoshi]`'s free). F6: the game's resource service unloads the files within 0.5 s, and jemalloc returns their
/// pages to the kernel only when an arena's decay ticker fires. Until then the room is on its way: nothing more is
/// freed for capacity, and a prefetch held for memory shows as loading ([`arriving_room`]). Operator option C (17:57Z)
/// with the primary's purge (19:47Z, F6): the
/// watch ends exactly when the game has unloaded what the frees queued (`res_unload`, read by each planner pass) and a
/// jemalloc purge requested after that has run on MainThread, so the committed charge is the room; never after a
/// number of passes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReleaseWatch {
    /// Nothing freed is still on its way.
    #[default]
    Settled,
    /// Freed; the game has not unloaded all the free queued.
    Unloading,
    /// Unloaded; waiting for the MainThread hook's purge number `ticket` (purges counted since boot, wrapping).
    Purging { ticket: u32 },
}

impl ReleaseWatch {
    /// A parked group was freed: its room is on its way (a free during another starts over at its unload).
    pub fn freed(self) -> Self {
        ReleaseWatch::Unloading
    }
    /// A planner pass. `waiting`: the read of what the frees queued, `Some(true)` while any is still to be unloaded,
    /// None when the table is unreadable (it never holds a release). `purges`: the purges the MainThread hook has
    /// completed. `purge`: whether the purge is available (its rows passed); without it the unload settles the watch.
    pub fn pass(self, waiting: Option<bool>, purges: u32, purge: bool) -> Self {
        match self {
            ReleaseWatch::Settled => ReleaseWatch::Settled,
            ReleaseWatch::Unloading if waiting == Some(true) => ReleaseWatch::Unloading,
            ReleaseWatch::Unloading if purge => ReleaseWatch::Purging { ticket: purges.wrapping_add(1) },
            ReleaseWatch::Unloading => ReleaseWatch::Settled,
            ReleaseWatch::Purging { ticket } if purges.wrapping_sub(ticket) as i32 >= 0 => ReleaseWatch::Settled,
            ReleaseWatch::Purging { .. } => self,
        }
    }
    /// The purge this watch waits for; the MainThread hook runs one while a request is newer than its last.
    pub fn requested(self) -> Option<u32> {
        match self {
            ReleaseWatch::Purging { ticket } => Some(ticket),
            _ => None,
        }
    }
    pub fn settled(self) -> bool {
        self == ReleaseWatch::Settled
    }
}

/// The state the notice shows for a group while a free's room is `arriving` ([`ReleaseWatch`]): a prefetch held for
/// memory is loading, not waiting. A pool hold is not memory and shows as it is.
pub fn arriving_room(state: GroupState, arriving: bool) -> GroupState {
    if arriving && state == GroupState::Held(Hold::Memory) { GroupState::Planned } else { state }
}

/// Operator rule 2026-11-09 17:13Z (13:21Z point 4): whether a held group's room is coming, so the notice shows it as
/// loading ([`arriving_room`]): a free's room is still arriving (`settled` false), or a release decided for it is in
/// progress (`releasing`: a parked group marked to go now). Otherwise its hold shows as it is: "waiting for memory"
/// means nothing more can be freed.
pub fn room_coming(settled: bool, releasing: bool) -> bool {
    !settled || releasing
}

/// Operator rule 2026-11-09 17:13Z: the planner's check, in the pass where the watch settles, of a held group: the
/// reason a capacity release is still needed (`memory_refused`: the admission refuses its cost now; or fewer free pool
/// bodies than its members), or None when it fits now (its next start is admitted) or it isn't held.
pub fn still_short(state: GroupState, memory_refused: bool, pool_free: usize, members: usize) -> Option<&'static str> {
    match state {
        GroupState::Held(Hold::Memory) if memory_refused => Some("process headroom"),
        GroupState::Held(Hold::Pool) if pool_free < members => Some("fighter pool"),
        _ => None,
    }
}

/// The lane window's planned targets (`next` for [`window_targets`]): the reusable extras of the window the planner wants
/// for its next lane kind `target` (`shown`, target first). A group that holds the target already is that window (an exchange left it detached, e.g.
/// while a Mii group is live) and counts with the groups; a member a group holds counts there too (rc6-hot-xps19).
pub fn next_window_targets(shown: &[u32], target: u32, original: Option<u32>, grouped: impl Fn(u32) -> bool) -> u8 {
    if grouped(target) { return 0; }
    shown.iter().filter(|&&kind| kind != target && Some(kind) != original && !grouped(kind)).count() as u8
}

/// [`count_extra_groups_iter`] over `(member count, state)`, so a parked group can count only its kept members.
pub fn count_group_members(groups: impl IntoIterator<Item = (u8, GroupState)>) -> GroupCounts {
    let mut counts = GroupCounts::default();
    for (count, state) in groups {
        if state == GroupState::Released { continue; }
        counts.planned += count;
        match state {
            GroupState::Released => {}
            GroupState::Built => counts.ready += count,
            GroupState::Prefetched => counts.prefetched += count,
            GroupState::Failed => counts.failed += count,
            GroupState::Planned | GroupState::Prefetching => counts.pending += count,
            GroupState::Held(hold) => {
                counts.held += count;
                counts.hold = merge_hold(counts.hold, Some(hold));
            }
        }
    }
    counts
}

/// What the status sees of a role's window bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowStatus {
    /// No bank: the first swap that needs the window builds it (loads are demand-driven), so
    /// nothing runs and nothing is owed until then.
    Absent,
    /// The bank is Ready and its active member observed: the window extras are built.
    Built,
    /// A bank exists and is building, switching or not yet observed: its extras are owed.
    Busy,
    /// The bank failed.
    Failed,
}

/// One role's contribution to the fighter status. `planned == ready + prefetched + failed +
/// pending + held + unstarted`, where `unstarted` (planned members of a role without a bank) is
/// what remains of `planned`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoleCounts {
    pub planned: u8,
    pub ready: u8,
    pub prefetched: u8,
    pub failed: u8,
    pub pending: u8,
    pub held: u8,
    pub hold: Option<Hold>,
}

/// The Fighters notice's `swap` mask: the roles whose own next swap builds their unstarted fighters.
pub const SWAP_PLAYER: u8 = 1;
pub const SWAP_CPU: u8 = 2;

impl RoleCounts {
    /// Planned fighters that neither the role's window nor its groups account for (a role
    /// without a bank, or a plan beyond its window): only that role's next swap builds them.
    pub fn unstarted(self) -> u8 {
        let accounted = u16::from(self.ready) + u16::from(self.prefetched)
            + u16::from(self.failed) + u16::from(self.pending) + u16::from(self.held);
        u16::from(self.planned).saturating_sub(accounted) as u8
    }

    /// This role's bit of the notice's `swap` mask (role 0 Player, 1 CPU), or 0 when its own
    /// swap is not what its fighters wait for.
    pub fn swap_owed(self, role: usize) -> u8 {
        if self.unstarted() == 0 { 0 } else if role == 0 { SWAP_PLAYER } else { SWAP_CPU }
    }
}

/// `planned_targets` is the planner's receipt of what the role should hold, `extra` the window
/// members that are reusable swap targets (not the active body, not the canonical original), and
/// `groups` the extra groups' counts. A role with nothing planned contributes nothing.
pub fn role_counts(planned_targets: u8, extra: u8, window: WindowStatus, groups: GroupCounts) -> RoleCounts {
    if window == WindowStatus::Absent {
        return RoleCounts { planned: planned_targets, ..RoleCounts::default() };
    }
    let mut counts = RoleCounts {
        planned: planned_targets.max(extra.saturating_add(groups.planned)),
        ready: groups.ready,
        prefetched: groups.prefetched,
        failed: groups.failed,
        pending: groups.pending,
        held: groups.held,
        hold: groups.hold,
    };
    match window {
        WindowStatus::Built => counts.ready = counts.ready.saturating_add(extra),
        WindowStatus::Failed => counts.failed = counts.failed.saturating_add(extra),
        WindowStatus::Busy => counts.pending = counts.pending.saturating_add(extra),
        WindowStatus::Absent => {}
    }
    counts
}

/// SPECIAL-GROUPS §0 X4 (construct `0x60d1ec-0x60d26c`): the entry info wrapper (`entry+0xf0`, or Nana's own
/// `entry+0x100`) whose `info+0x94` gate a body's construction reads, from the entry words `count` (`+0x14`) and cells
/// 0/1 (`+0x18`, `+0x1c`). Only a Popo or Nana body chooses: with two cells, the cell-1 kind gets `+0x100`; every other
/// case is `+0xf0`. Native reads the live entry's words; a background body is given its building group's
/// (`native_body_bank::background_info`), as its synchronous build would read them.
pub fn info_wrapper(kind: u32, count: u32, cells: [u32; 2]) -> usize {
    let [cell0, cell1] = cells;
    if matches!(kind, POPO | NANA) && count >= 2 && cell1 != u32::MAX && cell0 != kind && cell1 == kind { 0x100 } else { 0xf0 }
}

/// The window part of a role's planned targets (`plan_budget`). With groups on, the live attached window (`live`: its
/// members and active kind; the start-up or a Reload-built window, a pair-lead window or any group switched in) stays
/// while the role's other groups build around it, so it plans what it serves: its reusable extras, the count the notice
/// reads (`role_counts`' `extra`). The window the lane's next kind would get (`next`) counted two extras where a
/// one-member or pair group serves none ("swap P1 to load 2 more" with everything built; primary 2026-11-09 02:00Z,
/// every kind). With groups off, or without a live window, the next swap builds the lane's window (`next`).
pub fn window_targets(grouped: bool, live: Option<(Members, u32)>, original: Option<u32>, next: u8) -> u8 {
    match live {
        Some((members, active)) if grouped => {
            let (kinds, count) = members.logical_kinds();
            crate::engine::preload::cycle_preload_capacity::reusable_extras(&kinds[..count], active, original)
        }
        _ => next,
    }
}

/// Entry-info observer dispatcher words (`info+8`, `+0x20`, `+0x24`) that must hold
/// between dispatches, exactly as `654dc0` leaves them: one list type, no dispatch
/// in progress, no deferred removal pending (GROUP-SWAP-IMPL §14.1). A swap
/// exchanges the two array pointers only in that state.
pub fn dispatcher_idle_words(list_types: u32, depth: u32, pending_removals: u32) -> bool {
    list_types == 1 && depth == 0 && pending_removals == 0
}

/// Largest listener ring the native module walks when validating a detached
/// group's dispatcher array before handing it to the native destructor.
pub const DISPATCHER_WALK_LIMIT: u64 = 64;

/// Header of a dispatcher array `654dc0` allocated (`0x654e38-0x654e54`): exactly
/// one `{size, next, prev}` list head, with a ring short enough to validate.
pub fn dispatcher_array_header_ok(element_count: u64, list_size: u64) -> bool {
    element_count == 1 && list_size <= DISPATCHER_WALK_LIMIT
}

/// Value-only identity of a bank-owned group exchange. Native callers must prove
/// the current bodies' kinds, owners, states and contexts before supplying it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupOwner {
    pub entry: usize,
    pub owner: u32,
    pub epoch: u64,
    pub resource: usize,
    pub header: [u32; 5],
    pub descriptor: [u8; 0x70],
    pub bodies: [(usize, u32); NATIVE_MEMBERS],
}

/// A `GroupOwner` projected to the exact receipt (`crate::engine::mii::identity::Identity`): the
/// fields a bank receipt cannot observe (object, O index/tokens, active member) are zero on both
/// sides of every compare and therefore equal.
impl From<GroupOwner> for crate::engine::mii::identity::Identity {
    fn from(owner: GroupOwner) -> Self {
        crate::engine::mii::identity::Identity {
            entry: owner.entry, owner: owner.owner, epoch: owner.epoch, resource: owner.resource, object: 0,
            header: owner.header, descriptor: owner.descriptor, index: 0, tokens: [0; 4], bodies: owner.bodies, active: 0,
        }
    }
}

impl Members {
    /// The entry words a bank exchange writes for these members: count (`FE+0x14`), kinds
    /// (`FE+0x18..0x24`) and one actor for ordinary and native groups, two for the pair-lead window
    /// and the pair group (`write_group_words`/`restore_window` publish the same).
    pub fn layout(self) -> crate::engine::mii::identity::GroupLayout {
        crate::engine::mii::identity::GroupLayout { count: self.count() as u32, kinds: self.kinds(), actors: self.actor_count() }
    }
}

/// Minted only at a verified exchange, never from an arbitrary Ready callback.
/// The shared resource keeps its header; only the entry's group words change
/// (`ExpectedMutation::GroupExchange`, WP5a: one `verify` for every compare site).
#[derive(Clone, Copy, Debug)]
pub struct GroupExchange {
    before: [u8; 0x70],
    after: GroupOwner,
}
impl GroupExchange {
    #[cfg(test)]
    pub fn new(before: GroupOwner, after: GroupOwner, members: Members) -> Option<Self> {
        Self::new_with_header(before, after, members, before.header)
    }

    /// `header` is the shared resource header the exchange must leave behind:
    /// the window's own for ordinary, native and pair groups (unchanged across the
    /// exchange, D-PTA1), the Mii identity header `[owner, mii, MAX, MAX, costume]`
    /// once a Mii group is built or attached, and the restored previous header
    /// when a Mii window is left (MII-GROUPS-DESIGN §3).
    #[cfg(test)]
    pub fn new_with_header(before: GroupOwner, after: GroupOwner, members: Members, header: [u32; 5]) -> Option<Self> {
        Self::new_with_entry(before, after, members, header, None)
    }

    /// [`GroupExchange::new_with_header`] when the exchange also rewrote the
    /// entry's Mii identity bytes (`MII_ENTRY_START..MII_ENTRY_END`): a Mii group
    /// moving in writes its identity's bytes, a Mii window moving out gets the
    /// bytes its saved window carried. `None` keeps today's rule that an
    /// exchange rewrites only the group words.
    pub fn new_with_entry(before: GroupOwner, after: GroupOwner, members: Members, header: [u32; 5],
        entry: Option<MiiEntryBytes>) -> Option<Self> {
        use crate::engine::mii::identity::{verify, ExpectedMutation};
        if before.entry == 0 || before.resource == 0 { return None; }
        let ok = if header == before.header && entry.is_none() {
            // WORKAROUND-DECISIONS row 8 (WP5a): an exchange is recognised by the exact receipt — every
            // identity field, the index/tokens and the bodies equal the origin's, the descriptor equal
            // except the exchanged group words (`GroupLayout::apply` = count, kinds, actor count).
            verify(&before.into(), &after.into(),
                ExpectedMutation::GroupExchange { layout: members.layout(), bodies: after.bodies }).is_ok()
        } else {
            // Mii groups (M0, MII-GROUPS-DESIGN §3): the exchange also installs the Mii identity header
            // and, when a Mii moves in or out, the entry's Mii bytes; both are compared byte-exact here.
            let mut descriptor = before.descriptor;
            descriptor[0x14..0x18].copy_from_slice(&(members.count() as u32).to_le_bytes());
            for (i, kind) in members.kinds().iter().enumerate() {
                descriptor[0x18 + i * 4..0x1c + i * 4].copy_from_slice(&kind.to_le_bytes());
            }
            // One actor for ordinary and native groups, two for the pair-lead window
            // and the pair group (`write_group_words`/`restore_window` publish the same).
            descriptor[0x28..0x2c].copy_from_slice(&members.actor_count().to_le_bytes());
            if let Some(bytes) = entry {
                apply_entry_bytes(&mut descriptor, &bytes);
            }
            before.entry == after.entry && before.owner == after.owner
                && before.epoch == after.epoch && before.resource == after.resource
                && header == after.header && descriptor == after.descriptor
        };
        ok.then_some(Self { before: before.descriptor, after })
    }

    /// Entry, owner, epoch, shared resource header and bodies must equal the
    /// minted receipt exactly. The descriptor must too, except for the entry's
    /// natively owned live behavior bytes: Training CPU Behavior (Stop, Attack,
    /// level, Control) rewrites FE+0x10..0x14 and FE+0x34..=0x37 on the live entry
    /// while the group stays exchanged (C118 D5). Each is accepted only inside
    /// its native domain (`ExpectedMutation::NativeBehavior`); group words, flags,
    /// index and tokens stay byte-exact, so every foreign identity, header or body
    /// change still refuses.
    pub fn permits(self, current: GroupOwner) -> bool {
        use crate::engine::mii::identity::{verify, ExpectedMutation};
        verify(&self.after.into(), &current.into(), ExpectedMutation::NativeBehavior).is_ok()
    }

    pub fn rebaseline(self, before: &[u8; 0x70], current: GroupOwner) -> bool {
        self.before == *before && self.permits(current)
    }
}

// ----- Mii one-member groups: the identity the shared resource owner carries -----

/// Entry bytes a Mii identity owns inside the 0x70-byte descriptor: `FE+0x41`
/// (costume, `R+0x64`), `FE+0x43` (cleared by every Mii staging), `FE+0x46..0x50`
/// (record index and the outfit/hat/colour/voice tokens, `R+0x68..0x7c`) and
/// `FE+0x52..0x55` (the descriptor flag bytes state 6 folds into `R+0x7d`/`R+0x7f`).
/// Everything else (group words, options, live behavior bytes) stays the window's.
pub const MII_ENTRY_START: usize = 0x41;
pub const MII_ENTRY_END: usize = 0x56;
pub const MII_ENTRY_LEN: usize = MII_ENTRY_END - MII_ENTRY_START;
pub type MiiEntryBytes = [u8; MII_ENTRY_LEN];

/// `R+0x58..0x84` as eleven words: the header `R+0x58/5c/60/64`, the Mii identity
/// `R+0x68` (u16 record index, upper half unused), `R+0x6c/70/74/78` tokens, the
/// flag bytes `R+0x7c..0x7f` and `R+0x80` (state 6, `0x651334-0x65137c`).
pub const RESOURCE_WORDS: usize = 11;

/// The bank flag the one-member Mii window shows at `+0x591f` (`prepare_models`
/// writes `expected_flags(0)` = [`BANK_FLAG`]); a canonical Mii entry shows 0.
#[cfg(test)]
pub const MII_WINDOW_FLAGS: u8 = BANK_FLAG;

/// The resource-owner words a Mii one-member group needs in `R` while its key
/// requests, its construction and its unload run, derived from the group's staged
/// descriptor exactly as loader state 6 derives them from the entry
/// (`0x650904-0x650970`, `0x6510ac-0x6510bc`, `0x651334-0x65137c`, MII-GROUPS-DESIGN
/// §1.1): `R+0x58..0x60 = FE+0x18..0x20`, `R+0x64 = FE+0x41`, `R+0x68 (u16) = FE+0x46`,
/// `R+0x6c/70/74/78 = FE+0x48/4a/4c/4e`, `R+0x7d = FE+0x52 ? 1 : FE+0x53 ^ 1`,
/// `R+0x7f = FE+0x54`. `R+0x54` (owner), `R+0x7c` (PT flag), `R+0x7e` and `R+0x80`
/// are the same for a Mii and for the ordinary window and are left alone. Value
/// type only: the adapter performs the writes under INSTALL in the entry callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MiiIdentity {
    pub kinds: [u32; NATIVE_MEMBERS],
    pub costume: u8,
    pub index: u16,
    /// outfit, hat, colour, voice (`FE+0x48..0x50`).
    pub tokens: [u16; 4],
    /// `FE+0x52`, `FE+0x53`, `FE+0x54`.
    pub flags: [u8; 3],
}

impl MiiIdentity {
    /// The identity a staged exact-Mii descriptor names: one Mii class, count 1,
    /// empty padding cells, one actor. Anything else is not a Mii identity.
    pub fn from_descriptor(descriptor: &[u8; 0x70]) -> Option<Self> {
        let word = |offset: usize| u32::from_le_bytes(descriptor[offset..offset + 4].try_into().unwrap());
        let half = |offset: usize| u16::from_le_bytes(descriptor[offset..offset + 2].try_into().unwrap());
        let kinds = [word(0x18), word(0x1c), word(0x20)];
        if !(0x48..=0x4a).contains(&kinds[0]) || kinds[1] != u32::MAX || kinds[2] != u32::MAX
            || word(0x14) != 1 || word(0x28) != 1 { return None; }
        Some(Self {
            kinds, costume: descriptor[0x41], index: half(0x46),
            tokens: [half(0x48), half(0x4a), half(0x4c), half(0x4e)],
            flags: [descriptor[0x52], descriptor[0x53], descriptor[0x54]],
        })
    }

    pub fn kind(self) -> u32 { self.kinds[0] }
    pub fn outfit(self) -> u16 { self.tokens[0] }
    pub fn hat(self) -> u16 { self.tokens[1] }
    #[cfg(test)]
    pub fn colour(self) -> u16 { self.tokens[2] }
    #[cfg(test)]
    pub fn voice(self) -> u16 { self.tokens[3] }

    /// `R+0x54..0x68` as the adapters and the exchange receipt read it.
    pub fn header(self, owner: u32) -> [u32; 5] {
        [owner, self.kinds[0], self.kinds[1], self.kinds[2], u32::from(self.costume)]
    }

    /// `R+0x7d`: `FE+0x52 ? 1 : FE+0x53 ^ 1` (state 6, `0x651334-0x65137c`).
    pub fn flag_7d(self) -> u8 { if self.flags[0] != 0 { 1 } else { self.flags[1] ^ 1 } }
    /// `R+0x7f = FE+0x54`.
    pub fn flag_7f(self) -> u8 { self.flags[2] }

    /// `D+0x72..0x7a`: the information block's mirror of index, outfit, hat, colour
    /// (`completed` compares it with `FE+0x46..0x4e`; the HUD portrait reads it).
    pub fn d_mirror(self) -> [u16; 4] { [self.index, self.tokens[0], self.tokens[1], self.tokens[2]] }

    /// `R+0x6c..0x7c` as the completion proofs read it (`O+0x6c == tokens[1..5]`).
    pub fn o_tokens(self) -> [u32; 4] { self.tokens.map(u32::from) }

    /// `R+0x58..0x84` for this identity over the window's current words: the
    /// header kinds and costume, the record index (upper half kept), the four
    /// tokens, `R+0x7d`/`R+0x7f` inside the flag word (`R+0x7c`/`R+0x7e` kept) and
    /// `R+0x80` kept.
    pub fn resource_words(self, current: [u32; RESOURCE_WORDS]) -> [u32; RESOURCE_WORDS] {
        let mut words = current;
        words[0] = self.kinds[0]; words[1] = self.kinds[1]; words[2] = self.kinds[2];
        words[3] = u32::from(self.costume);
        words[4] = (current[4] & 0xffff_0000) | u32::from(self.index);
        for (i, token) in self.tokens.iter().enumerate() { words[5 + i] = u32::from(*token); }
        words[9] = (current[9] & 0x00ff_00ff) | (u32::from(self.flag_7d()) << 8) | (u32::from(self.flag_7f()) << 24);
        words
    }

    /// Write the identity's own bytes onto a descriptor (the group words are the
    /// window's business): `FE+0x41`, `FE+0x43 = 0`, `FE+0x46..0x50`, `FE+0x52..0x55`.
    pub fn apply(self, descriptor: &mut [u8; 0x70]) {
        descriptor[0x41] = self.costume;
        descriptor[0x43] = 0;
        descriptor[0x46..0x48].copy_from_slice(&self.index.to_le_bytes());
        for (i, token) in self.tokens.iter().enumerate() {
            descriptor[0x48 + i * 2..0x4a + i * 2].copy_from_slice(&token.to_le_bytes());
        }
        descriptor[0x52..0x55].copy_from_slice(&self.flags);
    }

    /// The entry bytes a build or attach of this identity leaves in `window`.
    pub fn entry_bytes(self, window: &[u8; 0x70]) -> MiiEntryBytes {
        let mut descriptor = *window;
        self.apply(&mut descriptor);
        entry_bytes_of(&descriptor)
    }
}

/// What a warm attach writes of the Mii identity (`R` words, entry bytes, D mirror).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityMove<T> {
    /// The target is a Mii group: its own identity.
    Mii,
    /// Out of a Mii window into an ordinary group: these ordinary values.
    Ordinary(T),
    /// Between two ordinary windows: nothing (`R` and the entry hold the ordinary values already).
    None,
}

/// rc.6 (Eden `rc6-hot-xps15`): a warm attach writes an identity only when a Mii is involved. A Mii target writes its own;
/// out of a Mii window an ordinary target writes the move it saved when a Mii displaced it, else the owner's ordinary
/// baseline; between two ordinary windows nothing, even when the target still holds a saved move from an earlier Mii
/// (writing it changed what the adapters own).
pub fn identity_move<T>(target_mii: bool, leaving_mii: bool, saved: Option<T>, baseline: Option<T>) -> IdentityMove<T> {
    if target_mii { return IdentityMove::Mii; }
    if !leaving_mii { return IdentityMove::None; }
    saved.or(baseline).map_or(IdentityMove::None, IdentityMove::Ordinary)
}

/// rc.6 warm exit to any Built group (inbox 21:27Z): the `R+0x58..` words an ordinary `window` takes when it attaches out
/// of a Mii window without its own saved identity move: the owner's ordinary `baseline` (what the first ordinary window a
/// Mii displaced left in `R`) with that window's header kinds.
pub fn baseline_words(baseline: [u32; RESOURCE_WORDS], window: Members) -> [u32; RESOURCE_WORDS] {
    let mut words = baseline;
    words[..NATIVE_MEMBERS].copy_from_slice(&window.kinds());
    words
}

/// The `R+0x58..` words a group's native work takes when they are not the live window's (its key requests and releases,
/// its background tick and the worker's shadow `R`): a Mii group its identity's over `current`; X6 (primary 2026-11-09
/// 04:22Z), beside a live Mii window (`window_identity`) any other group the owner's ordinary `baseline` with its own
/// header kinds (`baseline_words`, what its warm exit writes), not the Mii's costume, record and flags. `None` when `R`
/// holds them already (an ordinary window), or when no baseline is known (its background build waits).
pub fn own_words(members: Members, identity: Option<MiiIdentity>, window_identity: bool, baseline: Option<[u32; RESOURCE_WORDS]>,
    current: [u32; RESOURCE_WORDS]) -> Option<[u32; RESOURCE_WORDS]> {
    match identity {
        Some(identity) => Some(identity.resource_words(current)),
        None if window_identity => baseline.map(|baseline| baseline_words(baseline, members)),
        None => None,
    }
}

/// The values the worker hooks give a background body that reads a shadow `R` (`native_body_bank` `MII_VALUES`), from
/// the identity bytes its tick wrote into the entry: the record index `FE+0x46` (bits 0-15), the costume `FE+0x41`
/// (16-23, construct's `0x17e99c0` argument) and `FE+0x52` (24-31, body init's `0x607d7c`).
pub fn shadow_values(entry: &MiiEntryBytes) -> u32 {
    let at = |offset: usize| entry[offset - MII_ENTRY_START];
    u32::from(u16::from_le_bytes([at(0x46), at(0x47)])) | u32::from(at(0x41)) << 16 | u32::from(at(0x52)) << 24
}

/// The identity-owned slice of a descriptor (what a Mii window must give back).
pub fn entry_bytes_of(descriptor: &[u8; 0x70]) -> MiiEntryBytes {
    descriptor[MII_ENTRY_START..MII_ENTRY_END].try_into().unwrap()
}

pub fn apply_entry_bytes(descriptor: &mut [u8; 0x70], bytes: &MiiEntryBytes) {
    descriptor[MII_ENTRY_START..MII_ENTRY_END].copy_from_slice(bytes);
}

/// A Mii request's bytes equal the live entry once the identity the request
/// names is applied to it, the group words aside (they stay the window's until
/// the build): the reuse test for a Mii group swap. The ordinary test is byte
/// equality; a Mii build rewrites exactly the identity bytes. Which identity the
/// reachable group carries is the separate `group_identity_matches` gate.
pub fn mii_reuse_bytes_match(live: &[u8; 0x70], request: &[u8; 0x70]) -> bool {
    let Some(identity) = MiiIdentity::from_descriptor(request) else { return false; };
    let mut expected = *live;
    identity.apply(&mut expected);
    (0..0x70)
        .filter(|i| !(0x14..0x24).contains(i) && !(0x28..0x2c).contains(i))
        .all(|i| expected[i] == request[i])
}

/// Warm exit from a Mii group (operator decision 2026-11-08, rc.6): the window the live Mii group displaced comes back
/// with the entry bytes it saved at that exchange (`saved`). An ordinary request out of the Mii window matches the live
/// entry with those bytes applied, the group words aside (the window's until the exchange) and `+0x43`, the restored
/// window's own starting-form byte (an in-window reuse keeps the live one). Every other byte is exact.
pub fn mii_exit_bytes_match(live: &[u8; 0x70], request: &[u8; 0x70], saved: &MiiEntryBytes) -> bool {
    let mut expected = *live;
    apply_entry_bytes(&mut expected, saved);
    (0..0x70)
        .filter(|i| !(0x14..0x24).contains(i) && !(0x28..0x2c).contains(i) && *i != 0x43)
        .all(|i| expected[i] == request[i])
}

/// The one-member Mii bank window readiness rule beside the canonical
/// [`EntryState::ready`]: count 1, `[mii, MAX, MAX]`, one actor, bank flags and
/// the bank's ready index 0 (MII-GROUPS-DESIGN §4). The caller still proves the
/// bank's own descriptor and body receipts (`ready_forms`) and the body checks.
pub fn single_mii_bank_window(state: EntryState) -> Option<Forms> {
    let members = Members::mii_group(state.kinds[0])?;
    if state.count != 1 || state.kinds[1] != u32::MAX || state.kinds[2] != u32::MAX || state.actor_count != 1 {
        return None;
    }
    (members.ready_index(state) == Some(0)).then(|| Forms::starting(state.kinds[0]))
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn exchanged_owner(owner: u32) -> (GroupOwner, GroupOwner, Members) {
        let mut descriptor = [0u8; 0x70];
        descriptor[0x14..0x18].copy_from_slice(&3u32.to_le_bytes());
        descriptor[0x18..0x24].copy_from_slice(&[3,0,0,0,7,0,0,0,8,0,0,0]);
        descriptor[0x28] = 1;
        descriptor[0x41] = 2;
        let before = GroupOwner { entry: 0x1000, owner, epoch: 9, resource: 0x2000,
            header: [owner,3,7,8,2], descriptor, bodies: [(0x3000,1),(0x4000,262144),(0x5000,327680)] };
        let mut after = before;
        after.descriptor[0x18..0x24].copy_from_slice(&[10,0,0,0,11,0,0,0,21,0,0,0]);
        after.bodies = [(0x6000,393216),(0x7000,458752),(0x8000,524288)];
        (before, after, Members::new(&[10,11,21]).unwrap())
    }

    /// rc.6 item 3: only an ordinary single-actor group builds beside the live window, and only while the window holds
    /// no Mii identity and its live member is not Kirby; the first Prefetched eligible group in plan order goes first.
    /// rc.6 mid-session cycle change (inbox 19:21Z): a detached group leaves when one of its members is no longer in the
    /// role's cycle (its kinds plus the original); a pair group's Nana follows Popo, and a native group is one cycle
    /// member, kept while any of its forms is wanted. A Released group is nothing: not planned, not counted, no keys, no
    /// bodies, never a target; its slot takes the next planned group.
    fn a_group_leaves_when_a_member_left_the_cycle_and_its_slot_is_reused() {
        let wanted = [0x00, 0x01, 0x02, 0x03, 0x07, POPO, 0x25, 0x49];
        assert!(!group_left_cycle(Members::new(&[0x03, 0x07]).unwrap(), &wanted));
        assert!(group_left_cycle(Members::new(&[0x03, 0x05, 0x07]).unwrap(), &wanted), "Yoshi left");
        assert!(group_left_cycle(Members::new(&[0x08]).unwrap(), &wanted));
        assert!(!group_left_cycle(Members::pair_group(), &wanted), "Nana follows Popo");
        assert!(group_left_cycle(Members::pair_group(), &wanted[..5]));
        assert!(!group_left_cycle(Members::native_group(0x24).unwrap(), &wanted), "Ivysaur keeps the trainer");
        assert!(group_left_cycle(Members::native_group(0x5a).unwrap(), &wanted));
        assert!(!group_left_cycle(Members::mii_group(0x49).unwrap(), &wanted));
        assert!(group_left_cycle(Members::mii_group(0x48).unwrap(), &wanted));
        let released = GroupState::Released;
        assert!(!released.startable() && !released.reachable() && !released.holds_keys() && !released.counts_bodies());
        let counts = count_extra_groups_iter([(Members::new(&[0x08]).unwrap(), released), (Members::new(&[0x03]).unwrap(), GroupState::Built)]);
        assert_eq!((counts.planned, counts.ready, counts.failed, counts.pending), (1, 1, 0, 0));
        assert_eq!(memory::outstanding(released, memory::GroupCost { prefetch: 5, build: 7 }), 0);
        assert_eq!(next_start([(released, true), (GroupState::Planned, true)].into_iter(), 0), Some(1));
        assert_eq!(reuse_slot([GroupState::Built, released, GroupState::Released].into_iter()), Some(1));
        assert_eq!(reuse_slot([GroupState::Built, GroupState::Failed].into_iter()), None);
    }

    /// Eden rc6-hot-xps15: after the exit from a Mii went to Mario's window through the baseline, [Samus, Fox] still held
    /// the move it saved when the Mii displaced it, and its later attach out of Mario's (ordinary) window wrote it: the
    /// Player adapter's owned state changed and the switch settled unowned. A warm attach writes an identity only when a
    /// Mii is involved: a Mii target its own; out of a Mii window an ordinary target its saved move, else the baseline;
    /// between two ordinary windows nothing.
    fn an_identity_moves_only_into_or_out_of_a_mii_window() {
        let (saved, baseline) = (Some(1), Some(2));
        assert_eq!(identity_move(true, false, saved, baseline), IdentityMove::Mii);
        assert_eq!(identity_move::<u8>(true, true, None, None), IdentityMove::Mii);
        assert_eq!(identity_move(false, true, saved, baseline), IdentityMove::Ordinary(1));
        assert_eq!(identity_move(false, true, None, baseline), IdentityMove::Ordinary(2));
        assert_eq!(identity_move::<u8>(false, true, None, None), IdentityMove::None);
        assert_eq!(identity_move(false, false, saved, baseline), IdentityMove::None, "two ordinary windows");
        assert_eq!(identity_move(false, false, saved, None), IdentityMove::None);
    }

    /// rc6-hot-xps18 `mb-p1-change`: P1 dropped Fox, so `[Samus, Fox]` is parked until the pause. Samus is still served
    /// warm from it and counts as built; Fox left and counts nowhere (counting the whole group out showed Samus as
    /// "swap P1 to load", counting it whole showed Fox as planned). A native group stays whole while any form is wanted.
    fn a_parked_group_counts_only_the_members_its_cycle_keeps() {
        let wanted = [0x01, 0x02, 0x03, 0x05, 0x08];
        assert_eq!(kept_members(Members::new(&[0x03, 0x07]).unwrap(), &wanted), 1);
        assert_eq!(kept_members(Members::new(&[0x07]).unwrap(), &wanted), 0);
        assert_eq!(kept_members(Members::new(&[0x03, 0x05]).unwrap(), &wanted), 2);
        assert_eq!(kept_members(Members::mii_group(0x4a).unwrap(), &wanted), 0);
        let counts = count_group_members([(1, GroupState::Built), (2, GroupState::Planned), (3, GroupState::Released),
            (1, GroupState::Held(Hold::Pool))]);
        assert_eq!((counts.planned, counts.ready, counts.pending, counts.held, counts.hold), (4, 1, 2, 1, Some(Hold::Pool)));
        assert_eq!(count_extra_groups_iter([(Members::new(&[0x03, 0x07]).unwrap(), GroupState::Built)]),
            count_group_members([(2, GroupState::Built)]));
    }

    fn choose(windows: [Option<Members>; 2], kept: [&[Members]; 2], parked: [&[(Members, u8)]; 2], lanes: [&[u32]; 2],
        budget: GroupBudget) -> ReleaseNow {
        release_for_new_members(kept, parked, held_by(kept), |existing, freed| plan_extra_groups_with(GroupPolicy::DEFAULT,
            windows, existing, lanes, [None, None], GroupBudget { pool_free: budget.pool_free + freed, ..budget }))
    }

    fn held_by<'a>(groups: [&'a [Members]; 2]) -> impl Fn(usize, u32) -> bool + 'a {
        move |role: usize, kind: u32| groups[role].iter().any(|group| group.index_of(kind).is_some())
    }

    /// Operator rule 2026-11-09 (inbox 13:21Z), Eden `final-surface-i3-pf-r2` `x6-cc`: P1 on the Gunner (a Mii window)
    /// changes its cycle to DK, Link, PT and the Ice Climbers. PT fits beside the two parked groups (`[Samus, Yoshi]`,
    /// which the cycle no longer has, and `[Mario, DK, Link]`, which still serves DK and Link), the pair does not: the
    /// shared budget and P1's last group slot are theirs. Freeing `[Samus, Yoshi]` alone (2 bodies, no member the cycle
    /// keeps) lets the pair load now; `[Mario, DK, Link]` stays parked for the pause, serving DK and Link warm.
    fn a_change_that_does_not_fit_frees_just_enough_parked_bodies() {
        let mii = Members::mii_group(0x4a).unwrap();
        let cpu = Members::new(&[0x0b, 0x0d]).unwrap();
        let pt = Members::native_group(0x24).unwrap();
        let samus_yoshi = Members::new(&[0x03, 0x05]).unwrap();
        let mario_dk_link = Members::new(&[0x00, 0x01, 0x02]).unwrap();
        let kept: [&[Members]; 2] = [&[pt], &[]];
        let parked: [&[(Members, u8)]; 2] = [&[(samus_yoshi, 0), (mario_dk_link, 2)], &[]];
        let lanes: [&[u32]; 2] = [&[0x01, 0x02, 0x24, POPO], &[0x0b, 0x0d]];
        let budget = GroupBudget { ready_fighters: 12, pool_free: 7, special: SpecialPolicy::ALL };
        let release = choose([Some(mii), Some(cpu)], kept, parked, lanes, budget);
        assert_eq!(release, ReleaseNow { groups: [0b01, 0], members: [2, 0] });

        // Equal bodies: the group whose members the new cycle no longer has goes first.
        let p1 = Members::new(&[0x01]).unwrap();
        let unserved = Members::new(&[0x03, 0x05]).unwrap();
        let serves_pikachu = Members::new(&[0x07, 0x08]).unwrap();
        let tight = GroupBudget { ready_fighters: 6, pool_free: 8, special: SpecialPolicy::NONE };
        let release = choose([Some(p1), None], [&[], &[]],
            [&[(serves_pikachu, 1), (unserved, 0)], &[]], [&[0x07, 0x16], &[]], tight);
        assert_eq!(release, ReleaseNow { groups: [0b10, 0], members: [1, 0] });

        // Equally unserved: the fewest bodies.
        let one = Members::new(&[0x09]).unwrap();
        let five = GroupBudget { ready_fighters: 5, pool_free: 8, special: SpecialPolicy::NONE };
        let release = choose([Some(p1), None], [&[], &[]],
            [&[(unserved, 0), (one, 0)], &[]], [&[0x16], &[]], five);
        assert_eq!(release, ReleaseNow { groups: [0b10, 0], members: [1, 0] });

        // Two new members need both parked singles.
        let samus = Members::new(&[0x03]).unwrap();
        let four = GroupBudget { ready_fighters: 4, pool_free: 8, special: SpecialPolicy::NONE };
        let release = choose([Some(p1), None], [&[], &[]],
            [&[(samus, 0), (one, 0)], &[]], [&[0x16, 0x17], &[]], four);
        assert_eq!(release, ReleaseNow { groups: [0b11, 0], members: [2, 0] });

        // The budget is shared: a CPU group parked for the pause makes room for a new P1 member.
        let falcon = Members::new(&[0x0b]).unwrap();
        let ganon = Members::new(&[0x0c]).unwrap();
        let three = GroupBudget { ready_fighters: 3, pool_free: 8, special: SpecialPolicy::NONE };
        let release = choose([Some(p1), Some(falcon)], [&[], &[]],
            [&[], &[(ganon, 0)]], [&[0x16], &[0x0b]], three);
        assert_eq!(release, ReleaseNow { groups: [0, 0b1], members: [1, 0] });
    }

    /// Operator rule 2026-11-09 (inbox 13:21Z), point 4: with nothing parked a real shortage keeps "waiting for memory";
    /// the members a release makes room for show as loading, never as waiting.
    fn nothing_parked_and_a_real_shortage_still_waits_for_memory() {
        let p1 = Members::new(&[0x01]).unwrap();
        let none = GroupBudget { ready_fighters: 2, pool_free: 0, special: SpecialPolicy::NONE };
        assert_eq!(choose([Some(p1), None], [&[], &[]], [&[], &[]],
            [&[0x16], &[]], none), ReleaseNow::default());
        assert_eq!(capacity_release(true, std::iter::empty::<(usize, Members, u8)>()), None);
        let held = count_group_members([(2, GroupState::Built), (3, GroupState::Held(Hold::Memory))]).with_release(0);
        assert_eq!((held.planned, held.ready, held.pending, held.held, held.hold), (5, 2, 0, 3, Some(Hold::Memory)));
        let freeing = count_group_members([(2, GroupState::Built)]).with_release(2);
        assert_eq!((freeing.planned, freeing.ready, freeing.pending, freeing.held, freeing.hold), (4, 2, 2, 0, None));
        // A prefetch refused for memory or pool bodies with parked groups present frees one now (the one the cycle
        // serves least, then the smallest), and the refused group is retried before another goes.
        let samus_yoshi = Members::new(&[0x03, 0x05]).unwrap();
        let mario_dk_link = Members::new(&[0x00, 0x01, 0x02]).unwrap();
        let one = Members::new(&[0x09]).unwrap();
        assert_eq!(capacity_release(true, [((0, 2), mario_dk_link, 2), ((0, 0), samus_yoshi, 0), ((1, 1), one, 0)].into_iter()),
            Some((1, 1)));
        assert_eq!(capacity_release(true, [((0, 2), mario_dk_link, 2), ((0, 0), samus_yoshi, 0)].into_iter()), Some((0, 0)));
    }

    /// Operator rule 2026-11-09 17:13Z (Eden `final-surface-i3-pf-b04856b2` `x6-cc`, #798–#804): the watch settled under
    /// the pair's need, and for the 17 ms before the last-resort release the notice read "Loaded 10/12 (2 waiting for
    /// memory)", raised for 3 s. The planner decides that release in the pass that settles, before the notice is
    /// published; a held prefetch shows "Loading" while its room is arriving or being freed, and "waiting for memory"
    /// only when nothing can be freed.
    fn a_release_in_progress_shows_loading_never_waiting() {
        assert_eq!(still_short(GroupState::Held(Hold::Memory), true, 9, 2), Some("process headroom"));
        assert_eq!(still_short(GroupState::Held(Hold::Memory), false, 9, 2), None, "it fits now");
        assert_eq!(still_short(GroupState::Held(Hold::Pool), false, 1, 2), Some("fighter pool"));
        assert_eq!(still_short(GroupState::Held(Hold::Pool), false, 2, 2), None);
        for state in [GroupState::Planned, GroupState::Prefetching, GroupState::Prefetched, GroupState::Built] {
            assert_eq!(still_short(state, true, 0, 2), None, "{state:?}");
        }
        let shown = |settled: bool, releasing: bool| {
            let held = arriving_room(GroupState::Held(Hold::Memory), room_coming(settled, releasing));
            let counts = count_group_members([(10, GroupState::Built), (2, held)]);
            (counts.pending, counts.held, counts.hold)
        };
        assert_eq!(shown(false, false), (2, 0, None), "the room is arriving");
        assert_eq!(shown(true, true), (2, 0, None), "a release decided for it is in progress");
        assert_eq!(shown(false, true), (2, 0, None));
        assert_eq!(shown(true, false), (0, 2, Some(Hold::Memory)), "nothing can be freed: waiting for memory");
    }

    /// The entry tick frees a parked group only when the planner (or a refused prefetch) marked it to go now; any other
    /// leaving group goes at once, a prefetch in flight settles first.
    fn a_parked_group_leaves_on_the_entry_tick_only_when_marked() {
        assert!(!leaves_now(GroupState::Built, false));
        assert!(leaves_now(GroupState::Built, true));
        assert!(!leaves_now(GroupState::Prefetching, false) && !leaves_now(GroupState::Prefetching, true));
        for state in [GroupState::Planned, GroupState::Prefetched, GroupState::Failed, GroupState::Held(Hold::Memory)] {
            assert!(leaves_now(state, false) && leaves_now(state, true), "{state:?}");
        }
    }

    /// rc.6 item 3, SPECIAL-GROUPS §3 (operator 2026-11-08 "All groups in rc.6"): a Pokémon Trainer or Pyra/Mythra group
    /// builds in the background beside an ordinary window, and every group builds beside a live native window, so a
    /// session that starts on Pokémon Trainer still gets its groups built (inbox 21:27Z). §0: a native window is one
    /// actor whose cells 0/1 are never `[Popo, Nana]` (X1), and only an Ice Climbers body picks its info wrapper from
    /// the live cells (X4, `0x60d1f0`). The holder step each native group takes after its bodies, and the X5 cell.
    fn a_background_build_takes_native_groups_and_builds_beside_a_live_native_window() {
        let ordinary = Members::new(&[0x2d, 0x2c, 0x1c]).unwrap();
        let window = Members::new(&[0x00, 0x01, 0x02]).unwrap();
        let pt = Members::native_group(0x25).unwrap();
        let aegis = Members::native_group(0x5b).unwrap();
        for native in [pt, aegis] {
            assert_eq!(background_build_eligible(native, false, window, LiveIdentity::Ordinary, false, false, false), Ok(()));
            assert_eq!(background_build_eligible(ordinary, false, native, LiveIdentity::Ordinary, false, false, false), Ok(()));
            assert_eq!(background_build_eligible(Members::new(&[0x1c]).unwrap(), false, native, LiveIdentity::Ordinary, false, false, false), Ok(()));
            assert_eq!(background_build_eligible(native, false, window, LiveIdentity::Mii { baseline: false }, false, false, false), Err("mii window"));
            assert_eq!(background_build_eligible(native, false, window, LiveIdentity::Ordinary, true, false, false), Err("live kirby"));
            assert_eq!(background_build_eligible(native, false, Members::pair_group(), LiveIdentity::Ordinary, false, false, false), Err("pair window"));
            // Cross cell (a): beside a live pair window with the redirected tail (X1; X4 does not apply to its kinds).
            assert_eq!(background_build_eligible(native, false, Members::pair_group(), LiveIdentity::Ordinary, false, true, false),
                Ok(()));
            assert_eq!(background_build_eligible(native, true, window, LiveIdentity::Ordinary, false, false, false), Err("native group"));
            // Its switch-time build stays `0x658af0`'s.
            assert_eq!(async_build_eligible(native, false), Err("native group"));
        }
        assert_eq!(background_build_eligible(pt, false, aegis, LiveIdentity::Ordinary, false, false, false), Ok(()));
        assert_eq!(background_build_eligible(aegis, false, pt, LiveIdentity::Ordinary, false, false, false), Ok(()));
        // A Mii group (with its identity) builds beside an ordinary window and, with X5, beside a live native window.
        assert_eq!(background_build_eligible(Members::mii_group(0x48).unwrap(), true, pt, LiveIdentity::Ordinary, false, false, false), Ok(()));
        assert_eq!(background_build_eligible(Members::mii_group(0x48).unwrap(), true, window, LiveIdentity::Ordinary, false, false, false), Ok(()));
        let groups = [(GroupState::Built, ordinary, false), (GroupState::Prefetched, pt, false),
            (GroupState::Prefetched, aegis, false)];
        assert_eq!(next_background(groups.iter().copied(), window, LiveIdentity::Ordinary, false, false, false), Some(1));
        assert_eq!(next_background(groups[2..].iter().copied(), pt, LiveIdentity::Ordinary, false, false, false), Some(0));
        assert_eq!(next_background([(GroupState::Prefetched, ordinary, false)].into_iter(), pt, LiveIdentity::Ordinary, false, false, false), Some(0));
        assert_eq!(next_background(groups.iter().copied(), Members::pair_group(), LiveIdentity::Ordinary, false, false, false), None);
        // Native loader state 11's step: PT's pool task on the worker, Aegis's pool on the game thread.
        assert_eq!((pt.holder_step(), aegis.holder_step()), (Some(HolderStep::Trainer), Some(HolderStep::Partner)));
        for other in [ordinary, Members::pair_group(), Members::mii_group(0x4a).unwrap()] {
            assert_eq!((other.holder_step(), other.partner_cell()), (None, None));
        }
        // X5: the group's cell 1, as its synchronous build reads it (Eden re6: `FE+0x1c = 0x5b` for both bodies).
        assert_eq!((aegis.partner_cell(), pt.partner_cell()), (Some(0x5b), None));
        assert_eq!(Members::native_group(0x5a).unwrap().partner_cell(), Some(0x5b));
        // State 11's PT task (`0x651020-0x651080`).
        assert_eq!([(0, 0), (0, 1), (1, 0), (1, 1)].map(|(a, b)| trainer_task::flag(a, b)), [1, 0, 1, 1]);
        assert_eq!([trainer_task::SIZE, trainer_task::HOLDER, trainer_task::OWNER, trainer_task::COSTUME, trainer_task::FLAG,
            trainer_task::BUILD, trainer_task::OBJECT_SIZE], [0xa0, 0, 8, 0x90, 0x91, 0x92, 1]);
        // It keeps the construction task's handle, pending count, queue id and functor offsets.
        const { assert!(construction_task::CANCEL < trainer_task::COSTUME && construction_task::SIZE <= trainer_task::COSTUME) };
    }

    /// Inbox 00:18Z (a console-crash class, at the base too): a later warm attach of a Pokémon Trainer group rebuilds its
    /// article pool (`special_articles::rebuild_current`, `0x3a6270`), and the trainer article reads its file through the
    /// object kind `0x72`, which the bank returned after the build (Eden rc6-bg-pt-base1: Unmapped Read8 @ 0 at
    /// main+0x35609c0 on 2 of 2 rebuilds). A detached PT group whose pool was retired takes the kind again, and it is
    /// reachable warm only while it holds it; nothing else needs it.
    fn a_retired_trainer_group_takes_its_object_kind_again_before_its_rebuild() {
        let pt = Members::native_group(0x24).unwrap();
        let aegis = Members::native_group(0x5a).unwrap();
        let ordinary = Members::new(&[0x2d, 0x2c]).unwrap();
        assert!(object_rearm_due(GroupState::Built, pt, 0, false));
        assert!(!object_rearm_due(GroupState::Built, pt, 1, false), "a filled pool (the first attach) needs nothing");
        assert!(!object_rearm_due(GroupState::Built, pt, 0, true), "already held");
        for state in [GroupState::Planned, GroupState::Prefetching, GroupState::Prefetched, GroupState::Failed, GroupState::Released] {
            assert!(!object_rearm_due(state, pt, 0, false), "{state:?}");
        }
        assert!(!object_rearm_due(GroupState::Built, aegis, 0, false), "an Aegis holder is never retired");
        assert!(!object_rearm_due(GroupState::Built, ordinary, 0, false));
        assert!(!object_ready(pt, 0, false), "its attach would rebuild without the files");
        assert!(object_ready(pt, 0, true) && object_ready(pt, 1, false));
        assert!(object_ready(aegis, 0, false) && object_ready(ordinary, 0, false));
    }

    /// Primary 2026-11-09 05:38Z: a detached PT group's object re-arm runs under a live Kirby. D-KIRBY1 is about the keys
    /// a Kirby copy can share on `R`, and a copy registers a fighter kind; the trainer's object kind `0x72` is none. So
    /// PT > … > Kirby > PT is warm again. Every fighter-kind prefetch still waits for the live Kirby (D-KIRBY1 stays for
    /// fighter keys), and every other live member admits the re-arm as before.
    fn a_trainer_object_rearm_is_admitted_under_a_live_kirby_and_a_fighter_prefetch_is_not() {
        let pt = Members::native_group(0x24).unwrap();
        let kind = pt.object_kind().unwrap();
        assert_eq!(kind, PT_OBJECT_KIND);
        assert_eq!(object_rearm_admitted(Some(KIRBY), kind), Ok(()), "the trainer's object kind under a live Kirby");
        for live in [None, Some(0x01), Some(0x24), Some(0x4b), Some(0x48)] {
            assert_eq!(object_rearm_admitted(live, kind), Ok(()), "{live:?}");
        }
        // D-KIRBY1 is unchanged for fighter keys.
        assert!(prefetch_admitted(Some(KIRBY)).is_err());
        for fighter in [0x24, 0x25, 0x26, 0x01, 0x4b] {
            assert!(object_rearm_admitted(Some(KIRBY), fighter).is_err(), "{fighter:#x} is a fighter kind a copy takes");
        }
        // A copy never names the object kind, so the ledger's last-line guard admits its release under any copy.
        for copy in [None, Some(0x24), Some(0x25), Some(0x26), Some(0x00)] {
            assert_eq!(key_release_admitted(kind, copy), Ok(()), "{copy:?}");
        }
    }

    /// The tick-gap meter reports the one stalled tick of a synchronous build and stays quiet
    /// for an asynchronous one; disarmed it ignores later ticks.
    fn tick_gap_reports_the_largest_stall_between_ticks() {
        let mut gap = TickGap::arm(1_000);
        gap.observe(17_000);
        gap.observe(34_000);
        gap.observe(1_050_000);
        gap.observe(1_067_000);
        assert_eq!(gap.max_gap_us, 1_016_000);
        assert_eq!(gap.ticks, 4);
        let mut quiet = TickGap::arm(0);
        for tick in 1..=60u64 {
            quiet.observe(tick * 16_667);
        }
        assert_eq!(quiet.max_gap_us, 16_667);
        assert_eq!(quiet.ticks, 60);
        quiet.disarm();
        quiet.observe(10_000_000);
        assert_eq!(quiet.max_gap_us, 16_667);
        assert_eq!(quiet.ticks, 60);
        assert!(!TickGap::default().armed);
    }

    fn dispatcher_words_gate_the_array_exchange() {
        assert!(dispatcher_idle_words(1, 0, 0));
        assert!(!dispatcher_idle_words(0, 0, 0));
        assert!(!dispatcher_idle_words(2, 0, 0));
        assert!(!dispatcher_idle_words(1, 1, 0));
        assert!(!dispatcher_idle_words(1, 0, 3));
        assert!(dispatcher_array_header_ok(1, 0));
        assert!(dispatcher_array_header_ok(1, 2));
        assert!(dispatcher_array_header_ok(1, DISPATCHER_WALK_LIMIT));
        assert!(!dispatcher_array_header_ok(0, 2));
        assert!(!dispatcher_array_header_ok(2, 2));
        assert!(!dispatcher_array_header_ok(1, DISPATCHER_WALK_LIMIT + 1));
    }

    fn snapshot(kinds: &[u32], active: u8) -> EntryState {
        let mut fields = [u32::MAX; 3];
        fields[..kinds.len()].copy_from_slice(kinds);
        EntryState { count: kinds.len() as u32, kinds: fields, actor_count: 1,
            installed: kinds.len() as u64, index: active, previous_or_next: 0xff,
            pending: 0xff, flags: 4, outer: 6, loader: 14, changing: 0, guard: 0 }
    }

    /// MII-PHASEB: the Mii setup's two seams are this module's own predicates. Every state
    /// `ready_index` admits is the plan seam's Ready phase, and every state
    /// `awaiting_completion` admits (with `+0x5933 == 0`, which `service_locked` requires
    /// before `Action::Swap`) is the Phase B seam's parked change; neither admits the other.
    fn the_mii_setup_seams_are_the_bank_s_ready_and_awaiting_predicates() {
        use crate::engine::mii::mii_setup::{EntryPhase, Seam};
        let phase = |s: EntryState, acknowledged: u8| EntryPhase { outer: s.outer, loader: s.loader, pending: s.pending,
            changing: s.changing, guard: s.guard, acknowledged };
        let bank = Members::new(&[3, 7, 8]).unwrap();
        let ready = snapshot(&[3, 7, 8], 2);
        assert_eq!(bank.ready_index(ready), Some(2));
        assert!(phase(ready, 1).admits(Seam::Plan) && phase(ready, 0).admits(Seam::Plan));
        assert!(!phase(ready, 1).admits(Seam::PhaseB) && !phase(ready, 0).admits(Seam::PhaseB));
        // M3 (Surface s2 #168 `request … via=0 from=2`): Pikachu parks, the Mii group builds.
        let waiting = EntryState { index: 0, previous_or_next: 0, outer: 3, loader: 12, changing: 1, ..ready };
        assert!(bank.awaiting_completion(waiting, 0));
        assert_eq!(phase(waiting, 0), EntryPhase::BANK_PARKED);
        assert!(phase(waiting, 0).admits(Seam::PhaseB) && !phase(waiting, 0).admits(Seam::Plan));
        assert!(!phase(waiting, 1).admits(Seam::PhaseB), "acknowledged: native activation may run");
        for bad in [EntryState { outer: 6, ..waiting }, EntryState { loader: 6, ..waiting }, EntryState { pending: 1, ..waiting }] {
            assert!(!bank.awaiting_completion(bad, 0) && !phase(bad, 0).admits(Seam::PhaseB), "{:?}", bad);
        }
        // A Mii setup refused before the window detaches touched nothing: the group keeps its
        // prefetch keys, so it must stay a key holder that teardown releases under its identity
        // (`Failed` means "keys released" and is never walked for keys).
        assert!(GroupState::Prefetched.holds_keys() && GroupState::Prefetched.reachable());
        assert!(!GroupState::Failed.holds_keys());
    }

    fn solo_popo_is_an_upcoming_member_pinned_to_cell_0_and_the_later_pinned_kind_waits() {
        // Row 4: an upcoming Solo Popo is a window member at any rotation position; the layout
        // pins it to physical cell 0 (`0x17e8410` registers only the primary's Popo) and the
        // target activates through its own cell. The Eden row's shape: [4b, 5, 7], Yoshi active.
        const KIRBY: u32 = 0x06;
        let solo = Forms::solo_popo();
        for upcoming in [[POPO, FOX], [FOX, POPO]] {
            let window = Members::from_cycle(YOSHI, &upcoming, 2).unwrap();
            assert_eq!(window.kinds(), [POPO, YOSHI, FOX], "{:x?}", upcoming);
            assert_eq!((window.index_of(YOSHI), window.index_of(POPO), window.index_of(FOX)), (Some(1), Some(0), Some(2)));
            assert_eq!((window.forms_of(POPO), member_forms_in(3, window.kinds(), 1, POPO)), (solo, solo));
            assert_eq!(window.actor_count(), 1);
        }
        let window = Members::from_cycle(YOSHI, &[POPO, FOX], 2).unwrap();
        // Resources as for the Solo-led window: native requests Popo and `R+0x5c` (Yoshi), the
        // hook requests Fox once; no pair rule (`R+0x50`) for a Popo without Nana in cell 1.
        assert_eq!(supplemental_acquisitions(window), ([FOX, u32::MAX, u32::MAX], 1));
        assert_eq!(pair_resource_references(window), None);
        assert!(window_swappable(window));
        assert_eq!((swap_commit_index_in(window, 1), swap_commit_index_in(window, 0)), (Some(2), Some(1)));
        // Ready with Yoshi active (the Reload's starting form `+0x43 = 1`), then Solo Popo warm.
        assert_eq!(window.ready_live(snapshot(&[POPO, YOSHI, FOX], 1)), Some((1, YOSHI)));
        assert_eq!(window.ready_live(snapshot(&[POPO, YOSHI, FOX], 0)), Some((0, POPO)));
        assert_eq!(crate::engine::swap::swap_model::group_serves(window, POPO, solo), Some(0));
        assert_eq!(crate::engine::swap::swap_model::group_serves(window, POPO, Forms::starting(POPO)), None, "never the pair");
        assert_eq!(Members::from_cycle(YOSHI, &[POPO], 1).unwrap().kinds(), [POPO, YOSHI, u32::MAX]);
        // Physical cells: Popo at 1/2 is an entry native never builds.
        assert!(Members::new(&[YOSHI, POPO]).is_none() && Members::new(&[YOSHI, FOX, POPO]).is_none());
        assert_eq!(Members::laid_out(&[YOSHI, POPO]).map(|m| m.kinds()), Some([POPO, YOSHI, u32::MAX]));
        // One pinned member per window: the later pinned kind in rotation order waits for the next
        // window and the next eligible kind takes its place.
        for (target, upcoming, cells) in [
            (POPO, &[KIRBY, LINK, FOX][..], [POPO, LINK, FOX]),
            (KIRBY, &[POPO, LINK, FOX][..], [KIRBY, LINK, FOX]),
            (LINK, &[KIRBY, POPO, FOX][..], [KIRBY, LINK, FOX]),
            (LINK, &[POPO, KIRBY, FOX][..], [POPO, LINK, FOX]),
            (LINK, &[POPO, KIRBY][..], [POPO, LINK, u32::MAX]),
        ] {
            let forms = if target == POPO { solo } else { Forms::starting(target) };
            let window = Members::from_forms(forms, upcoming, 2).unwrap();
            assert_eq!(window.kinds(), cells, "{:#x} {:x?}", target, upcoming);
            assert!(!crate::engine::swap::swap_model::self_mirror_hazard(&window.kinds()), "{:x?}", window.kinds());
        }
        // The pair is never an upcoming member: a pair-lead window never takes Popo as its extra.
        assert!(!pair_lead_extra(POPO));
        assert_eq!(Members::from_pair(&[POPO, LINK], 2).unwrap().kinds(), [POPO, NANA, LINK]);
        // Groups: the first pinned kind in lane order takes cell 0, the later one its own group.
        let solo_policy = GroupPolicy { solo_popo_groups: true, ..GroupPolicy::DEFAULT };
        let window = members(&[MARIO, LINK, SAMUS]);
        let plan = plan_extra_groups_with(solo_policy, [Some(window), None], [&[], &[]], [&[KIRBY, POPO, FOX], &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![KIRBY, FOX], vec![POPO]]);
        let plan = plan_extra_groups_with(solo_policy, [Some(window), None], [&[], &[]], [&[POPO, KIRBY, FOX], &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![POPO, FOX], vec![KIRBY]]);
    }

    fn a_pinned_window_reload_settles_on_its_target_cell_not_cell_0() {
        // Eden M4b `m4b2` (K-PIN1): `layout … cells=[6, 5, 7] active=1`, native ready at index 1 with
        // Yoshi; the Player transaction waited for index 0 and named Kirby (`+0x18[0]`), and stalled.
        // Surface `s5` (shipped M4b, `group_swap=off`): the window Reload into `[6, 4, 5]` for Dark
        // Samus submitted, published and waited as Kirby the same way.
        const KIRBY: u32 = 0x06;
        const DARK_SAMUS: u32 = 0x04;
        // The staged bank descriptor the planner writes (`request_group_starting(group, active)`).
        let staged = |members: Members, target: u32| {
            let mut d = [0u8; 0x70];
            d[0x14..0x18].copy_from_slice(&(members.count() as u32).to_le_bytes());
            for (i, kind) in members.kinds().iter().enumerate() {
                d[0x18 + i * 4..0x1c + i * 4].copy_from_slice(&kind.to_le_bytes());
            }
            d[0x28..0x2c].copy_from_slice(&members.actor_count().to_le_bytes());
            d[STARTING_FORM] = members.index_of(target).unwrap();
            d
        };
        for (target, upcoming, cells, cell) in [
            (YOSHI, &[KIRBY, FOX][..], [KIRBY, YOSHI, FOX], 1u8),
            (DARK_SAMUS, &[KIRBY, YOSHI][..], [KIRBY, DARK_SAMUS, YOSHI], 1),
            (YOSHI, &[POPO, FOX][..], [POPO, YOSHI, FOX], 1),
            (FOX, &[YOSHI, KIRBY][..], [KIRBY, FOX, YOSHI], 1),
            (KIRBY, &[YOSHI, FOX][..], [KIRBY, YOSHI, FOX], 0),
        ] {
            let members = Members::from_cycle(target, upcoming, 2).unwrap();
            assert_eq!(members.kinds(), cells, "{:#x}", target);
            let reload = ReloadTarget::of(&staged(members, target), true);
            assert_eq!(reload, ReloadTarget { cell, kind: target });
            assert!(reload.ready_at(cell) && (cell == 0 || !reload.ready_at(0)), "{:#x}", target);
            // What the bank reports at that Ready: the target, at its own cell.
            assert_eq!(members.ready_live(snapshot(&cells, cell)), Some((cell, target)));
        }
        // Solo Popo pinned as the target settles at cell 0; as a member it is never the target.
        let solo = Members::from_forms(Forms::solo_popo(), &[YOSHI, FOX], 2).unwrap();
        assert_eq!(ReloadTarget::of(&staged(solo, POPO), true), ReloadTarget { cell: 0, kind: POPO });
        // Canonical requests: the group's first kind, ready at the starting form (0 for a fighter).
        let mut canonical = [0u8; 0x70];
        canonical[0x14] = 1;
        canonical[0x18..0x1c].copy_from_slice(&MARIO.to_le_bytes());
        assert_eq!(ReloadTarget::of(&canonical, false), ReloadTarget { cell: 0, kind: MARIO });
        let mut pt = canonical;
        pt[0x14] = 3;
        pt[0x18..0x24].copy_from_slice(&[0x24, 0, 0, 0, 0x25, 0, 0, 0, 0x26, 0, 0, 0]);
        pt[STARTING_FORM] = 2;
        assert_eq!(ReloadTarget::of(&pt, false), ReloadTarget { cell: 2, kind: 0x24 }, "a PT group is named by its expansion");
    }

    // ----- group swap policy -----

    const MARIO: u32 = 0x00;
    const DK: u32 = 0x01;
    const LINK: u32 = 0x02;
    const SAMUS: u32 = 0x03;
    const YOSHI: u32 = 0x05;
    const FOX: u32 = 0x07;
    const PIKACHU: u32 = 0x08;
    const LUIGI: u32 = 0x09;
    const NESS: u32 = 0x0a;
    const FALCON: u32 = 0x0b;
    const PEACH: u32 = 0x0d;

    fn members(kinds: &[u32]) -> Members {
        Members::new(kinds).unwrap()
    }

    fn budget(ready_fighters: usize) -> GroupBudget {
        GroupBudget { ready_fighters, pool_free: 16, special: SpecialPolicy::NONE }
    }

    fn budget_special(ready_fighters: usize, special: SpecialPolicy) -> GroupBudget {
        GroupBudget { special, ..budget(ready_fighters) }
    }

    fn kinds_of(plan: &[Option<Members>; MAX_EXTRA_GROUPS]) -> Vec<Vec<u32>> {
        plan.iter().flatten().map(|m| m.kinds()[..m.count()].to_vec()).collect()
    }

    // ----- rc.6.1 target-first (the console play test's P1: Mr. Game & Watch original; cycle Pikachu, Kirby, Palutena) -----

    const GAME_WATCH: u32 = 0x1c;
    const PALUTENA: u32 = 0x36;
    const NONE: u32 = u32::MAX;

    fn the_rest_of_the_lane_is_planned_as_groups_beside_the_target() {
        let window = target_first(Forms::starting(GAME_WATCH), true, 6).unwrap();
        let lane: &[u32] = &[PIKACHU, KIRBY, PALUTENA];
        // The members today's window held are the first group (Kirby laid out at cell 0)...
        let full = Members::from_forms(Forms::starting(GAME_WATCH), lane, 2).unwrap();
        let seed = neighbour_group(full, GAME_WATCH, GroupPolicy::DEFAULT, None).unwrap();
        assert_eq!(seed.kinds(), [KIRBY, PIKACHU, NONE]);
        // ...and the planner takes the rest of the lane beside it: with the console's Ready Fighters (8), Palutena alone.
        // Without the seed it would have made one triple, whose admission needs more than the pair the window charged.
        let plan = plan_extra_groups([Some(window), None], [&[seed], &[]], [lane, &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![PALUTENA]]);
        let unseeded = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&unseeded[0]), vec![vec![KIRBY, PIKACHU, PALUTENA]]);
        // Both build in the background beside the one-member window, the neighbours first.
        let palutena = plan[0][0].unwrap();
        for group in [seed, palutena] {
            assert_eq!(background_build_eligible(group, false, window, LiveIdentity::Ordinary, false, false, false), Ok(()));
        }
        assert_eq!(next_background([(GroupState::Prefetched, seed, false), (GroupState::Prefetched, palutena, false)].into_iter(),
            window, LiveIdentity::Ordinary, false, false, false), Some(0));
        // Kinds the planner would not group stay out of the seed (the excluded original; Solo Popo without its policy).
        assert_eq!(neighbour_group(full, GAME_WATCH, GroupPolicy::DEFAULT, Some(KIRBY)).map(Members::kinds), Some([PIKACHU, NONE, NONE]));
        let solo = Members::from_forms(Forms::starting(GAME_WATCH), &[POPO], 1).unwrap();
        assert_eq!(neighbour_group(solo, GAME_WATCH, GroupPolicy::DEFAULT, None), None);
    }

    fn the_neighbours_reach_the_planner_only_for_their_own_window() {
        let window = target_first(Forms::starting(GAME_WATCH), true, 6).unwrap();
        let seed = members(&[KIRBY, PIKACHU]);
        let word = pack_neighbours(GAME_WATCH, Some(seed));
        assert_eq!(unpack_neighbours(word, window), Some(seed));
        assert_eq!(unpack_neighbours(pack_neighbours(GAME_WATCH, Some(members(&[PIKACHU]))), window), Some(members(&[PIKACHU])));
        // Another target's window, a full window, a Mii window and no seed get nothing.
        assert_eq!(unpack_neighbours(word, target_first(Forms::starting(PALUTENA), true, 6).unwrap()), None);
        assert_eq!(unpack_neighbours(word, Members::from_forms(Forms::starting(GAME_WATCH), &[KIRBY, PIKACHU], 2).unwrap()), None);
        assert_eq!(unpack_neighbours(word, Members::mii_group(0x48).unwrap()), None);
        assert_eq!(pack_neighbours(GAME_WATCH, None), u64::MAX);
        assert_eq!(unpack_neighbours(u64::MAX, window), None);
    }

    fn a_target_first_reload_keeps_the_built_groups_it_does_not_displace() {
        const GANONDORF: u32 = 0x18;
        const PICHU: u32 = 0x13;
        const DIDDY: u32 = 0x27;
        const KROOL: u32 = 0x45;
        // Boot 3 #163–#169: the CPU's lane [18, 13, 27, 45] and Mii Gunner, the window [18, 13, 27] attached, the Mii
        // group [4a] Built and detached, a cycle onto 0x45. Before rc.12 the Mii group was torn down and built again.
        // Eden rc12-int2-f44fac2f-1 (#797, #1000): the kept Mii group's warm attach after the Reload crashed the guest
        // (Unmapped Read64 in the Mii's fighter module): a Mii group is released first and built again, as before.
        let mii = Members::mii_group(0x4a).unwrap();
        assert!(!kept_across_reload(GroupState::Built, mii, false, false, Some(KROOL)), "a Mii group is released first");
        // The next lap (#242, target 0x27): the one-member groups the cycle left behind survive too.
        assert!(kept_across_reload(GroupState::Built, members(&[KROOL]), false, false, Some(DIDDY)));
        assert!(kept_across_reload(GroupState::Built, members(&[GANONDORF, PICHU]), false, false, Some(DIDDY)));
        // Released first, as before: a group holding the target, one holding keys, one leaving its cycle, one holding the
        // trainer's object kind, and every group at a canonical or full-window Reload.
        assert!(!kept_across_reload(GroupState::Built, members(&[DIDDY, PICHU]), false, false, Some(DIDDY)));
        for state in [GroupState::Prefetching, GroupState::Prefetched, GroupState::Planned] {
            assert!(!kept_across_reload(state, members(&[KROOL]), false, false, Some(DIDDY)), "{state:?}");
        }
        assert!(!kept_across_reload(GroupState::Built, members(&[KROOL]), true, false, Some(DIDDY)));
        assert!(!kept_across_reload(GroupState::Built, members(&[KROOL]), false, true, Some(DIDDY)));
        assert!(!kept_across_reload(GroupState::Built, members(&[KROOL]), false, false, None));
        // Ready Fighters 8: a pool of 6, P1 holding 3. The first lap's neighbours [18, 13] (the bodies the native Reload
        // destroyed) fit beside a kept one-member group; the next lap's two kept groups leave no room for a seed of two, which the
        // planner then places as far as the budget goes.
        assert!(neighbours_fit(2, 1, 3, 16, 6));
        assert!(!neighbours_fit(2, 2, 3, 16, 6), "the kept groups are charged first");
        assert!(neighbours_fit(1, 2, 3, 16, 6));
    }

    fn a_role_without_a_bank_still_counts_the_window_its_next_swap_prepares() {
        use crate::engine::preload::cycle_preload_policy::{BuiltFighters, CacheStatus, Progress, Readiness};
        use crate::engine::preload::hat_slots::with_seats;
        use crate::model::parked_entries::SeatState;
        // `final-surface-of-889919e1` `of-seat-notice` (integration FLAG 2026-11-10 17:53Z): the preloader turned on with
        // no bank on either role, the CPU (original Mario since its canonical switch) given the cycle [1, 2, 3], Ready
        // Fighters 12 (a window pool of 10), and Training's two parked seats loading.
        const CPU_ORIGINAL: u32 = 0x0;
        let lane = [0x1u32, 0x2, 0x3];
        let forms = Forms::starting(lane[0]);
        let window = Members::from_forms(forms, &lane[1..], 2).unwrap();
        assert_eq!(window.kinds(), lane);
        let pool = 10;
        let seats = [Some(SeatState::Loading), Some(SeatState::Loading)];
        // The next swap's Reload builds the target alone: nothing is reserved for it physically.
        assert_eq!(reload_extras(window, forms, true, pool, Some(lane[0])), 0);
        // The notice counts the window that swap prepares: its other members load right after it, as the first group.
        let notice = reserve_extras([0, notice_extras(window)], [0, 0], pool);
        let next = next_window_plan(window, lane[0], notice[1], Some(CPU_ORIGINAL), |_| false);
        assert_eq!(next, 2);
        let cpu = role_counts(window_targets(true, None, Some(CPU_ORIGINAL), next), 0, WindowStatus::Absent,
            GroupCounts::default());
        let fighters = with_seats(BuiltFighters { planned: cpu.planned, swap: cpu.swap_owed(1), ..BuiltFighters::default() },
            seats);
        assert_eq!((fighters.planned, fighters.pending, fighters.unstarted(), fighters.swap), (4, 2, 2, SWAP_CPU));
        let assets = Readiness { progress: Progress::Loading, ready: 0, planned: 5, configured: 5, limit: 5, failed: 0 };
        assert_eq!(CacheStatus::new(assets, fighters, false).progress(), Progress::Loading);
        // `889919e1` counted the physical reservation: nothing planned, so the seats were dropped and the status hidden.
        let physical = reserve_extras([0, reload_extras(window, forms, true, pool, Some(lane[0]))], [0, 0], pool);
        let hidden = next_window_plan(window, lane[0], physical[1], Some(CPU_ORIGINAL), |_| false);
        assert_eq!(hidden, 0);
        let none = with_seats(BuiltFighters::default(), seats);
        assert_eq!((none.planned, CacheStatus::new(assets, none, false).progress()), (0, Progress::Hidden));
        // With groups off the window is built whole, and the notice counts what is reserved, as before target-first.
        assert_eq!(reload_extras(window, forms, false, pool, Some(lane[0])), notice_extras(window));
    }

    fn a_switch_with_no_window_to_rebuild_keeps_its_canonical_reload() {
        // `final-surface-i3-kr-889919e1-r2` `kr-after` (integration FLAG 2 2026-11-10 17:56Z): P1's cycle is Pokémon Trainer
        // and Kirby. After `kr-copy`'s canonical Reload to PT, the switch to Kirby has no full window: PT is a native group,
        // never a window member. `732663e2` Reloaded Kirby canonically, with no bank and so no group planned beside him.
        const SQUIRTLE: u32 = 0x24;
        let lane = [SQUIRTLE, KIRBY, SQUIRTLE, KIRBY, SQUIRTLE];
        let kirby = Forms::starting(KIRBY);
        let full = Members::from_forms(kirby, &lane, 2);
        assert_eq!(full, None);
        assert_eq!(reload_window(full, kirby, true, 10, Some(KIRBY)), None);
        // A role with no cycle neighbour at all (the menu's CPU pick with no cycle): the same.
        assert_eq!(reload_window(Members::from_forms(Forms::starting(MARIO), &[], 2), Forms::starting(MARIO), true, 10, Some(MARIO)),
            None);
        // A window to rebuild still loads its target first; with groups off, or no pool, the full window as before.
        let gw = Forms::starting(GAME_WATCH);
        let full = Members::from_forms(gw, &[PIKACHU, KIRBY], 2);
        assert!(full.is_some());
        assert_eq!(reload_window(full, gw, true, 10, Some(GAME_WATCH)), target_first(gw, true, 10));
        assert_eq!(reload_window(full, gw, true, 10, Some(GAME_WATCH)).map(|m| m.count()), Some(1));
        assert_eq!(reload_window(full, gw, false, 10, Some(GAME_WATCH)), full);
        assert_eq!(reload_window(full, gw, true, 0, Some(GAME_WATCH)), full);
    }

    /// rc.7 FLAG 3: steps two banks (`groups`, from `states`) through `next_step`, `memory::admit` and
    /// `memory::outstanding`, each step spending at most what `final-surface-i3-nt-7d720c68` measured (a prefetch 57 MB at
    /// #331, a build 18 MB at #709). Returns the tick at which each bank's group 1 was admitted, and the final states.
    /// `ranked` false is `7d720c68`'s pick, which ignored the deferral.
    fn step_banks(groups: [[Members; 2]; 2], mut states: [[GroupState; 2]; 2], mut headroom: u64, ranked: bool)
        -> ([Option<usize>; 2], [[GroupState; 2]; 2]) {
        use super::memory::{admit, group_cost, outstanding, Admission};
        const PREFETCH_SPEND: u64 = 57_147_392;
        const BUILD_SPEND: u64 = 18_153_472;
        let window = Members::new(&[0x00, 0x01, 0x02]).unwrap();
        let cost = |m: Members| group_cost(&m.kinds()[..m.count()]);
        let held = |states: &[[GroupState; 2]; 2]| (0..2).flat_map(|r| (0..2).map(move |g| (r, g)))
            .map(|(r, g)| outstanding(states[r][g], cost(groups[r][g]))).sum::<u64>();
        let mut deferred = [false; 2];
        let mut admitted = [None; 2];
        for tick in 0..8 {
            for role in 0..2 {
                let start = next_start(states[role].iter().map(|state| (*state, true)), 0);
                let background = next_background(states[role].iter().zip(groups[role]).map(|(state, members)| (*state, members, false)),
                    window, LiveIdentity::Ordinary, false, false, false);
                match next_step(start.map(|g| (g, states[role][g], ranked && g == 1 && deferred[role])), background) {
                    Some(NextStep::Background(g)) => {
                        states[role][g] = GroupState::Built;
                        headroom -= BUILD_SPEND;
                    }
                    Some(NextStep::Prefetch(g)) => match admit(Some(headroom), held(&states), cost(groups[role][g])).admission {
                        Admission::Admit => {
                            states[role][g] = GroupState::Prefetched;
                            deferred[role] = false;
                            headroom -= PREFETCH_SPEND;
                            admitted[role].get_or_insert(tick);
                        }
                        Admission::Defer => deferred[role] = true,
                        Admission::Refuse => states[role][g] = GroupState::Held(Hold::Memory),
                    },
                    None => {}
                }
            }
        }
        (admitted, states)
    }

    fn both_banks_with_a_deferred_group_build_then_admit_it() {
        use super::memory::{group_cost, outstanding};
        // Primary 2026-11-10 22:15Z: both banks hold a deferred Planned group behind a Prefetched one, and both must make
        // progress. `final-surface-i3-nt-7d720c68` at #346: each bank's pair group Prefetched and one member Planned (P1
        // [Samus], the CPU [0xf]), 565 MB of headroom, 478 MB held by the two pair groups' builds.
        let groups = [[Members::pair_group(), Members::new(&[0x03]).unwrap()], [Members::pair_group(), Members::new(&[0x0f]).unwrap()]];
        let at_346 = [[GroupState::Prefetched, GroupState::Planned]; 2];
        assert_eq!(2 * outstanding(GroupState::Prefetched, group_cost(&[POPO, NANA])), 478_642_176);
        // Tick 0 defers both (#345/#346), tick 1 builds both pair groups, tick 2 admits both deferred groups.
        assert_eq!(step_banks(groups, at_346, 565_465_088, true), ([Some(2), Some(2)], [[GroupState::Built; 2]; 2]));
        // `7d720c68`'s pick re-picked both deferred prefetches on every tick, and nothing moved.
        assert_eq!(step_banks(groups, at_346, 565_465_088, false), ([None, None], at_346));
        // One bank (the docs lane, 22:24Z: a heavy draw with a reserve), the other with nothing left to do. 300 MB covers
        // [Samus] (173 MB) but not beside the pair group's build (239 MB held): it waits for its own bank's build.
        let one = [at_346[0], [GroupState::Built; 2]];
        assert_eq!(step_banks(groups, one, 300_000_000, true), ([Some(2), None], [[GroupState::Built; 2]; 2]));
        assert_eq!(step_banks(groups, one, 300_000_000, false), ([None, None], one));
    }

    fn a_canonical_reload_of_the_window_shape_retires_a_one_member_bank() {
        let one = target_first(Forms::starting(PALUTENA), true, 6).unwrap();
        assert!(canonical_replaces_window(one, Phase::Ready, 1, [PALUTENA, NONE, NONE], 1));
        // The bank's own Reload (Building), another kind (the descriptor check retires it) and other windows: unchanged.
        assert!(!canonical_replaces_window(one, Phase::Building, 1, [PALUTENA, NONE, NONE], 1));
        assert!(!canonical_replaces_window(one, Phase::Ready, 1, [GAME_WATCH, NONE, NONE], 1));
        let three = Members::from_forms(Forms::starting(GAME_WATCH), &[PIKACHU, KIRBY], 2).unwrap();
        assert!(!canonical_replaces_window(three, Phase::Ready, 3, three.kinds(), 1));
        let mii = Members::mii_group(0x48).unwrap();
        assert!(!canonical_replaces_window(mii, Phase::Ready, 1, mii.kinds(), 1));
    }

    fn eight_ready_fighters_default_gives_five_and_three() {
        let windows = [Some(members(&[MARIO, LINK, SAMUS])), Some(members(&[LUIGI, DK, YOSHI]))];
        let upcoming: [&[u32]; 2] = [&[FOX, PIKACHU, NESS, FALCON, PEACH], &[PEACH, FALCON, NESS, PIKACHU, FOX]];
        let plan = plan_extra_groups(windows, [&[], &[]], upcoming, [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        assert!(plan[1].iter().all(Option::is_none), "{:?}", plan[1]);
        // Nine: the third body extends P1's pair instead of opening a CPU singleton.
        let plan = plan_extra_groups(windows, [&[], &[]], upcoming, [None, None], budget(9));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU, NESS]]);
        assert!(plan[1].iter().all(Option::is_none));
        // Ten: a pair each, next neighbours first.
        let plan = plan_extra_groups(windows, [&[], &[]], upcoming, [None, None], budget(10));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH, FALCON]]);
        // Twelve with a five-deep lookahead: three each.
        let plan = plan_extra_groups(windows, [&[], &[]], upcoming, [None, None], budget(12));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU, NESS]]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH, FALCON, NESS]]);
        // Six or fewer: the windows already consume the budget.
        for ready in [0, 2, 6] {
            let plan = plan_extra_groups(windows, [&[], &[]], upcoming, [None, None], budget(ready));
            assert!(plan.iter().flatten().all(Option::is_none), "{ready}: {:?}", plan);
        }
        // Seven leaves one body that no pair can use: P1's nearest neighbour alone.
        let plan = plan_extra_groups(windows, [&[], &[]], upcoming, [None, None], budget(7));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX]]);
        assert!(plan[1].iter().all(Option::is_none), "{:?}", plan[1]);
    }

    fn group_state_machine_commit_index_and_counts() {
        assert!(GroupState::Prefetched.reachable() && GroupState::Built.reachable());
        for state in [GroupState::Planned, GroupState::Prefetching, GroupState::Failed] {
            assert!(!state.reachable(), "{:?}", state);
        }
        assert!(GroupState::Prefetching.holds_keys() && GroupState::Prefetched.holds_keys());
        assert!(!GroupState::Built.holds_keys() && !GroupState::Planned.holds_keys() && !GroupState::Failed.holds_keys());
        assert!(!GroupState::Failed.counts_bodies() && GroupState::Built.counts_bodies());
        assert_eq!(prefetch_outcome(true, 0, 10), GroupState::Prefetched);
        assert_eq!(prefetch_outcome(true, 99, 10), GroupState::Prefetched);
        assert_eq!(prefetch_outcome(false, 9, 10), GroupState::Prefetching);
        assert_eq!(prefetch_outcome(false, 10, 10), GroupState::Failed);
        assert_eq!(swap_commit_index(0, 3), Some(1));
        assert_eq!(swap_commit_index(2, 3), Some(0));
        assert_eq!(swap_commit_index(1, 2), Some(0));
        assert_eq!(swap_commit_index(0, 1), None);
        assert_eq!(swap_commit_index(3, 3), None);
        // Every window has an exit commit: a one-member window its own cell (self-commit).
        assert_eq!(swap_commit_index_in(members(&[FOX]), 0), Some(0));
        assert_eq!(members(&[FOX]).swap_commit_index_for(0), Some(0));
        assert_eq!(swap_commit_index_in(members(&[FOX]), 1), None);
        assert_eq!(swap_commit_index_in(members(&[FOX, PIKACHU]), 0), Some(1));
        assert_eq!(swap_commit_index_in(members(&[FOX, PIKACHU, NESS]), 0), Some(1));
        assert_eq!(members(&[FOX, PIKACHU, NESS]).swap_commit_index_for(2), Some(0));
        assert_eq!(members(&[FOX]).fallback_index(0), 0);
        let counts = count_extra_groups(&[
            (members(&[FOX, PIKACHU]), GroupState::Built),
            (members(&[NESS, FALCON, PEACH]), GroupState::Prefetching),
            (members(&[0x1c, 0x2c]), GroupState::Failed),
        ]);
        assert_eq!(counts, GroupCounts { planned: 7, ready: 2, prefetched: 0, failed: 2, pending: 3, ..Default::default() });
        let settled = count_extra_groups(&[(members(&[FOX, PIKACHU]), GroupState::Built)]);
        assert_eq!(settled, GroupCounts { planned: 2, ready: 2, prefetched: 0, failed: 0, pending: 0, ..Default::default() });
        assert_eq!(count_extra_groups(&[]), GroupCounts::default());
        // Split of the old `building`: a prefetched group is reachable (no reload) and owes no work;
        // a planned or prefetching group still owes its load.
        let split = count_extra_groups(&[
            (members(&[FOX, PIKACHU]), GroupState::Built),
            (members(&[NESS, FALCON, PEACH]), GroupState::Prefetched),
            (members(&[0x1c]), GroupState::Planned),
            (members(&[0x2c]), GroupState::Prefetching),
        ]);
        assert_eq!(split, GroupCounts { planned: 7, ready: 2, prefetched: 3, failed: 0, pending: 2, ..Default::default() });
        assert_eq!(split.planned, split.ready + split.prefetched + split.failed + split.pending);
        // A role's contribution (what the status sums over the two roles).
        let none = GroupCounts::default();
        // No bank yet: planned from the planner's receipt only, nothing built or owed.
        assert_eq!(role_counts(3, 0, WindowStatus::Absent, none),
            RoleCounts { planned: 3, ..RoleCounts::default() });
        // A role with nothing planned (a CPU sitting on its Mii with no bankable cycle) is zero.
        assert_eq!(role_counts(0, 0, WindowStatus::Absent, none), RoleCounts::default());
        // Built window with two extras and a prefetched group.
        let groups = count_extra_groups(&[(members(&[FOX, PIKACHU]), GroupState::Prefetched)]);
        assert_eq!(role_counts(4, 2, WindowStatus::Built, groups),
            RoleCounts { planned: 4, ready: 2, prefetched: 2, failed: 0, pending: 0, ..Default::default() });
        // The same window while the bank builds or switches: the extras are owed work.
        assert_eq!(role_counts(4, 2, WindowStatus::Busy, groups),
            RoleCounts { planned: 4, ready: 0, prefetched: 2, failed: 0, pending: 2, ..Default::default() });
        // A failed bank keeps its extras in the denominator, as failed.
        assert_eq!(role_counts(2, 2, WindowStatus::Failed, none),
            RoleCounts { planned: 2, ready: 0, prefetched: 0, failed: 2, pending: 0, ..Default::default() });
        // The receipt may lag the bank: the bank's own members win, never fewer than they are.
        assert_eq!(role_counts(1, 2, WindowStatus::Built, groups).planned, 4);
        // The planner's receipt may be ahead of the bank (groups planned, not yet in the bank).
        assert_eq!(role_counts(6, 2, WindowStatus::Built, groups),
            RoleCounts { planned: 6, ready: 2, prefetched: 2, failed: 0, pending: 0, ..Default::default() });
    }

    fn mii_groups_are_one_member_bank_windows_with_one_identity_per_role() {
        for kind in [0x48u32, 0x49, 0x4a] {
            let mii = Members::mii_group(kind).unwrap();
            assert_eq!(mii.kinds(), [kind, u32::MAX, u32::MAX]);
            assert_eq!((mii.count(), mii.actor_count(), mii.is_pair(), mii.is_pair_lead()), (1, 1, false, false));
            assert_eq!(mii.special(), Special::Mii);
            assert!(mii.carries_identity() && !mii.carries_records());
            assert_eq!(mii.expected_flags(0), BANK_FLAG);
            assert!(mii.selectable(0) && !mii.selectable(1));
            assert_eq!(mii.forms_of(kind), Forms::starting(kind));
            assert_eq!(mii.prefetch_kinds(), ([kind, u32::MAX, u32::MAX, u32::MAX], 1));
            assert_eq!(mii.object_kind(), None);
            // A one-member window: entered warm, left by its self-commit; it plans its role's new groups like any
            // window (inbox 06:16Z, `a_live_mii_window_plans_its_roles_new_groups`).
            assert!(window_swappable(mii));
            assert_eq!((swap_commit_index_in(mii, 0), mii.swap_commit_index_for(0)), (Some(0), Some(0)));
            assert!(mii.awaiting_completion(EntryState { count: 1, kinds: [kind, u32::MAX, u32::MAX], actor_count: 1,
                installed: 1, index: 0, previous_or_next: 0, pending: 0xff, flags: BANK_FLAG, outer: 3, loader: 12,
                changing: 1, guard: 0 }, 0), "the self-commit's seam: index == next == 0, loader 12");
            assert_eq!(pair_resource_references(mii), None);
            // Ready: bank flags on a count-1 window; canonical readiness still refuses it.
            let state = EntryState { count: 1, kinds: [kind, u32::MAX, u32::MAX], actor_count: 1, installed: 1, index: 0,
                previous_or_next: 0xff, pending: 0xff, flags: BANK_FLAG, outer: 6, loader: 14, changing: 0, guard: 0 };
            assert_eq!(mii.ready_index(state), Some(0));
            assert!(state.ready().is_none());
            assert_eq!(mii.ready_index(EntryState { flags: 0, ..state }), None);
            assert_eq!(mii_group_request(1, [kind, u32::MAX, u32::MAX], 1), Some(Forms::starting(kind)));
            assert_eq!(mii_group_request(2, [kind, u32::MAX, u32::MAX], 1), None);
            assert_eq!(mii_group_request(1, [kind, u32::MAX, u32::MAX], 2), None);
            // Policy gates it; window banks never adopt it.
            assert!(Members::special_group(kind, SpecialPolicy::NATIVE).is_none());
            assert_eq!(Members::special_group(kind, SpecialPolicy::MII), Some(mii));
            assert!(Members::new(&[kind]).is_none() && Members::from_cycle(kind, &[LINK], 2).is_none());
            assert!(!special_group_kind(kind, SpecialPolicy::ALL) && special_group_kind(kind, SpecialPolicy::MII));
        }
        assert_eq!(mii_group_request(1, [LINK, u32::MAX, u32::MAX], 1), None);
        // Planner: the Mii is placed whole at its lane depth under its policy only,
        // and a second Mii class in the lane (or an existing Mii group) is skipped.
        let windows = [Some(members(&[MARIO, LINK, SAMUS])), Some(members(&[LUIGI, DK, YOSHI]))];
        let lane: &[u32] = &[0x4a, FOX, 0x48, PIKACHU];
        let plan = plan_extra_groups(windows, [&[], &[]], [lane, &[]], [None, None], budget_special(10, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        let plan = plan_extra_groups(windows, [&[], &[]], [lane, &[]], [None, None], budget_special(10, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), vec![vec![0x4a], vec![FOX, PIKACHU]]);
        assert_eq!(plan[0][0].unwrap().special(), Special::Mii);
        let gunner = Members::mii_group(0x4a).unwrap();
        let plan = plan_extra_groups(windows, [&[gunner], &[]], [&[0x48, FOX, PIKACHU], &[]], [None, None], budget_special(11, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        // A Mii window plans its role's new groups like any one-member window (inbox 06:16Z; it planned nothing before);
        // its budget charge is one body, and a second Mii group stays out beside it.
        let plan = plan_extra_groups([Some(gunner), windows[1]], [&[], &[]], [&[FOX, PIKACHU], &[FALCON, 0x15]], [None, None],
            budget_special(10, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![FALCON, 0x15]]);
        let plan = plan_extra_groups([Some(gunner), windows[1]], [&[], &[]], [&[0x48, FOX, PIKACHU], &[]], [None, None],
            budget_special(10, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        // The exchange receipt: a Mii build leaves the Mii header, not the window's.
        let (before, mut after, _) = exchanged_owner(0);
        after.descriptor[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
        after.descriptor[0x18..0x24].copy_from_slice(&[0x4a, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        after.descriptor[0x28..0x2c].copy_from_slice(&1u32.to_le_bytes());
        let mii_header = [0, 0x4a, u32::MAX, u32::MAX, 0];
        after.header = mii_header;
        assert!(GroupExchange::new(before, after, gunner).is_none(), "the window header is no longer the shared header");
        let exchange = GroupExchange::new_with_header(before, after, gunner, mii_header).unwrap();
        assert!(exchange.permits(after));
        assert!(!exchange.permits(GroupOwner { header: before.header, ..after }));
    }

    fn a_mii_group_identity_is_the_staged_descriptor_and_changes_with_its_hat_or_class() {
        use crate::engine::mii::{backend::Layout, inventory::MiiClass, lifetime::{Configuration, Tokens},
            native_values::RowIds, ordinary_cpu::{staged_descriptor_for_owner, staged_descriptor_in}};
        // Row ids carry their token in bits 40..56 (`RowIds::validate_tokens`).
        let config = |class, hat: u16| Configuration::from_observed(class,
            Tokens { outfit: 75, hat, colour: 0, voice: 37, specials: [2, 2, 2, 1] }, [0x5a; 88],
            RowIds { outfit: (0x13u64 << 56) | (75u64 << 40) | 0x1bfef7a5f7, hat: (0x5au64 << 56) | (u64::from(hat) << 40) | 0x0fa9efa5ea }).unwrap();
        let gunner = config(MiiClass::Gunner, 125);
        // An ordinary three-body window descriptor with option and flag bytes set.
        let mut window = [0u8; 0x70];
        window[0x14..0x18].copy_from_slice(&3u32.to_le_bytes());
        window[0x18..0x24].copy_from_slice(&[8, 0, 0, 0, 3, 0, 0, 0, 7, 0, 0, 0]);
        window[0x28] = 1; window[0x41] = 2; window[0x43] = 9; window[0x53] = 1; window[0x54] = 0x30;
        window[0x30] = 0x11;
        // Player0 is not a selector: its identity keeps the window's record index.
        assert_eq!(staged_descriptor_in(Layout::Training, 0, window, Some(gunner)), None);
        for owner in [0usize, 1] {
            let staged = staged_descriptor_for_owner(Layout::Training, owner, window, Some(gunner)).unwrap();
            let identity = MiiIdentity::from_descriptor(&staged).unwrap();
            assert_eq!(identity.kinds, [0x4a, u32::MAX, u32::MAX]);
            assert_eq!((identity.costume, identity.index, identity.tokens, identity.flags),
                (2, owner as u16, [75, 125, 0, 37], [0, 1, 0x30]));
            assert_eq!((identity.kind(), identity.outfit(), identity.hat(), identity.colour(), identity.voice()), (0x4a, 75, 125, 0, 37));
            assert_eq!(identity.header(owner as u32), [owner as u32, 0x4a, u32::MAX, u32::MAX, 2]);
            assert_eq!((identity.flag_7d(), identity.flag_7f()), (0, 0x30));
            assert_eq!(identity.d_mirror(), [owner as u16, 75, 125, 0]);
            assert_eq!(identity.o_tokens(), [75, 125, 0, 37]);
            // Applying the identity to the live window yields exactly the staged bytes
            // beyond the group words: the reuse test and the build's entry write.
            let mut applied = window;
            identity.apply(&mut applied);
            assert_eq!(applied[MII_ENTRY_START..MII_ENTRY_END], staged[MII_ENTRY_START..MII_ENTRY_END]);
            assert_eq!(identity.entry_bytes(&window), entry_bytes_of(&staged));
            assert!(mii_reuse_bytes_match(&window, &staged));
            // The same configuration again names the same identity (a reachable group serves it).
            assert_eq!(MiiIdentity::from_descriptor(&staged_descriptor_for_owner(Layout::Training, owner, window, Some(gunner)).unwrap()), Some(identity));
            // Another hat of the same class, or another class, is another identity.
            let hat = staged_descriptor_for_owner(Layout::Training, owner, window, Some(config(MiiClass::Gunner, 0))).unwrap();
            assert_ne!(MiiIdentity::from_descriptor(&hat), Some(identity));
            assert_eq!(MiiIdentity::from_descriptor(&hat).map(|i| i.hat()), Some(0));
            let brawler = staged_descriptor_for_owner(Layout::Training, owner, window, Some(config(MiiClass::Brawler, 125))).unwrap();
            assert_eq!(MiiIdentity::from_descriptor(&brawler).map(|i| i.kind()), Some(0x48));
            assert_ne!(MiiIdentity::from_descriptor(&brawler), Some(identity));
            // The reuse bytes test ignores the window's group words and the identity
            // bytes it will rewrite, and nothing else: an option byte must agree.
            assert!(mii_reuse_bytes_match(&applied, &staged) && mii_reuse_bytes_match(&window, &brawler));
            let mut option = staged;
            option[0x30] ^= 1;
            assert!(!mii_reuse_bytes_match(&window, &option));
        }
        // Not a Mii identity: an ordinary descriptor, a Mii with a second cell, two actors.
        assert_eq!(MiiIdentity::from_descriptor(&window), None);
        assert!(!mii_reuse_bytes_match(&window, &window));
        let staged = staged_descriptor_for_owner(Layout::Training, 1, window, Some(gunner)).unwrap();
        let mut two = staged;
        two[0x1c..0x20].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(MiiIdentity::from_descriptor(&two), None);
        let mut actors = staged;
        actors[0x28] = 2;
        assert_eq!(MiiIdentity::from_descriptor(&actors), None);
        // A live behavior byte change on the window never disturbs the identity match.
        let mut behavior = window;
        behavior[0x37] = 50;
        assert!(mii_reuse_bytes_match(&behavior, &{ let mut s = staged; s[0x37] = 50; s }));
        assert!(!mii_reuse_bytes_match(&behavior, &staged), "the reuse test is byte-exact outside the identity");
    }

    /// Warm exit from a Mii group (operator decision 2026-11-08, rc.6; console `#106`: Mii Gunner → Pichu took a 5.9 s
    /// Reload with `descriptor bytes differ at [46, 48, 4a, 4e]`): an ordinary request out of the live Mii window is
    /// served by the window the Mii displaced, whose exchange writes that window's saved entry bytes back.
    fn an_ordinary_request_leaves_a_mii_window_for_the_window_it_displaced() {
        use crate::engine::mii::{backend::Layout, inventory::MiiClass, lifetime::{Configuration, Tokens},
            native_values::RowIds, ordinary_cpu::staged_descriptor_for_owner};
        let gunner = Configuration::from_observed(MiiClass::Gunner,
            Tokens { outfit: 75, hat: 125, colour: 0, voice: 37, specials: [2, 2, 2, 1] }, [0x5a; 88],
            RowIds { outfit: (0x13u64 << 56) | (75u64 << 40) | 0x1bfef7a5f7, hat: (0x5au64 << 56) | (125u64 << 40) | 0x0fa9efa5ea }).unwrap();
        // The CPU's window [Fox, Pichu, Diddy] with option and flag bytes, started on cell 0.
        let mut window = [0u8; 0x70];
        window[0x14..0x18].copy_from_slice(&3u32.to_le_bytes());
        window[0x18..0x24].copy_from_slice(&[7, 0, 0, 0, 0x13, 0, 0, 0, 0x27, 0, 0, 0]);
        window[0x28] = 1; window[0x41] = 2; window[0x53] = 1; window[0x54] = 0x30; window[0x30] = 0x11;
        let saved = entry_bytes_of(&window);
        // The live entry after the Mii exchange: the Mii group words and the identity's entry bytes.
        let staged = staged_descriptor_for_owner(Layout::Training, 1, window, Some(gunner)).unwrap();
        let mut live = window;
        live[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
        live[0x18..0x24].copy_from_slice(&[0x4a, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        MiiIdentity::from_descriptor(&staged).unwrap().apply(&mut live);
        // The ordinary request the adapter stages for Pichu, with the live group words `plan` gives every reuse.
        let mut request = window;
        request[0x14..0x24].copy_from_slice(&live[0x14..0x24]);
        request[0x18..0x1c].copy_from_slice(&0x13u32.to_le_bytes());
        request[0x43] = 0;
        assert!(!(0..0x70).filter(|i| !(0x14..0x24).contains(i)).all(|i| live[i] == request[i]),
            "byte-exact against the Mii's identity bytes, as today, it never matches");
        assert!(mii_exit_bytes_match(&live, &request, &saved), "the displaced window's bytes come back");
        // `+0x43` is the restored window's own starting-form byte, as an in-window reuse keeps the live one.
        assert!(mii_exit_bytes_match(&live, &{ let mut r = request; r[0x43] = 9; r }, &saved));
        // Anything else the request names must be exactly what comes back.
        for at in [0x30usize, 0x41, 0x46, 0x48, 0x4e, 0x53, 0x60] {
            let mut other = request;
            other[at] ^= 1;
            assert!(!mii_exit_bytes_match(&live, &other, &saved), "byte {at:#x}");
        }
        // A live behavior byte (CPU Behavior, FE+0x37) agrees on both sides or the request is not this exit.
        let mut behavior = live;
        behavior[0x37] = 50;
        assert!(mii_exit_bytes_match(&behavior, &{ let mut r = request; r[0x37] = 50; r }, &saved));
        assert!(!mii_exit_bytes_match(&behavior, &request, &saved));
    }

    fn a_one_member_mii_bank_window_is_ready_only_with_bank_flags_at_index_zero() {
        let state = EntryState { count: 1, kinds: [0x49, u32::MAX, u32::MAX], actor_count: 1, installed: 1, index: 0,
            previous_or_next: 0xff, pending: 0xff, flags: MII_WINDOW_FLAGS, outer: 6, loader: 14, changing: 0, guard: 0 };
        assert_eq!(single_mii_bank_window(state), Some(Forms::starting(0x49)));
        assert!(state.ready().is_none(), "the canonical rule still refuses bank flags");
        assert_eq!(single_mii_bank_window(EntryState { flags: 0, ..state }), None, "canonical flags belong to the canonical rule");
        assert_eq!(single_mii_bank_window(EntryState { previous_or_next: 0, ..state }), Some(Forms::starting(0x49)));
        assert_eq!(single_mii_bank_window(EntryState { kinds: [0x49, 3, u32::MAX], count: 2, installed: 2, ..state }), None);
        assert_eq!(single_mii_bank_window(EntryState { actor_count: 2, ..state }), None);
        assert_eq!(single_mii_bank_window(EntryState { kinds: [3, u32::MAX, u32::MAX], ..state }), None);
        assert_eq!(single_mii_bank_window(EntryState { index: 1, ..state }), None);
        assert_eq!(single_mii_bank_window(EntryState { loader: 6, ..state }), None);
        assert_eq!(single_mii_bank_window(EntryState { pending: 0, ..state }), None);
        assert_eq!(single_mii_bank_window(EntryState { changing: 1, ..state }), None);
        assert_eq!(single_mii_bank_window(EntryState { installed: 0, ..state }), None);
    }

    fn one_member_groups_are_valid_members_with_empty_padding_cells() {
        let one = Members::new(&[LINK]).unwrap();
        assert_eq!(one.kinds(), [LINK, u32::MAX, u32::MAX]);
        assert_eq!((one.count(), one.actor_count(), one.is_pair_lead()), (1, 1, false));
        assert_eq!(one.logical_kinds(), ([LINK, u32::MAX, u32::MAX], 1));
        assert_eq!(one.index_of(LINK), Some(0));
        assert_eq!(one.index_of(u32::MAX), None);
        assert!(one.selectable(0) && !one.selectable(1) && !one.selectable(2));
        assert_eq!(one.expected_flags(0), BANK_FLAG);
        assert_eq!([0, 1, 2].map(|i| one.body_state(i, Some(0))), [4, 3, 3]);
        assert_eq!([0, 1, 2].map(|i| one.body_state(i, None)), [3, 3, 3]);
        // The window form of a one-member group: count 1, installed 1, bank flags,
        // padding words `u32::MAX` exactly as `write_group_words` publishes them.
        let state = snapshot(&[LINK], 0);
        assert_eq!(one.ready_index(state), Some(0));
        assert_eq!(one.ready_live(state), Some((0, LINK)));
        assert!(state.ready().is_none(), "bank flags keep a one-member window off the canonical path");
        assert_eq!(one.ready_index(snapshot(&[LINK], 1)), None);
        assert_eq!(one.ready_index(EntryState { installed: 2, ..state }), None);
        assert_eq!(one.ready_index(EntryState { count: 2, ..state }), None);
        assert_eq!(one.ready_index(EntryState { flags: 0, ..state }), None);
        assert_eq!(one.ready_index(EntryState { kinds: [LINK, LINK, u32::MAX], ..state }), None);
        assert!(!one.awaiting_completion(state, 0));
        // The self-commit (`ExitRule::SelfCell`): the request selects cell 0 itself, and the
        // seam is the ordinary one (`+0x5918 == +0x5919 == 0`, loader 12, the body parked).
        assert!(one.awaiting_completion(EntryState { previous_or_next: 0, outer: 3, loader: 12, changing: 1, ..state }, 0));
        assert_eq!(swap_commit_index(0, one.count()), None, "the next-cell rule has no second cell");
        assert_eq!(swap_commit_index_in(one, 0), Some(0));
        assert_eq!(one.fallback_index(0), 0, "a refused exchange re-activates the outgoing body itself");
        assert_eq!(supplemental_acquisitions(one), ([u32::MAX; 3], 0));
        assert_eq!(pair_resource_references(one), None);
        assert!(swappable_kind(LINK) && window_swappable(one));
        assert!(Members::new(&[]).is_none());
        assert!(Members::new(&[0x48]).is_none() && Members::new(&[0x24]).is_none() && Members::new(&[u32::MAX]).is_none());
        assert_eq!(Members::new(&[0x06]).map(|m| m.kinds()), Some([0x06, u32::MAX, u32::MAX]));
        assert_eq!(Members::new(&[POPO]).map(|m| m.kinds()), Some([POPO, u32::MAX, u32::MAX]));
        // Window banks still start with two members.
        assert!(Members::from_cycle(MARIO, &[], 2).is_none());
        assert!(Members::from_forms(Forms::starting(MARIO), &[0x48], 2).is_none());
        assert!(Members::from_forms(Forms::solo_popo(), &[], 2).is_none());
        // Exchange receipts: a one-member group has count 1, padding words and
        // empty body cells 1/2; swapping back to a three-member record stays exact.
        let (before, mut after, _) = exchanged_owner(0);
        after.descriptor[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
        after.descriptor[0x18..0x24].copy_from_slice(&[10, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        after.bodies = [(0x6000, 393216), (0, 0), (0, 0)];
        let receipt = GroupExchange::new(before, after, Members::new(&[10]).unwrap()).unwrap();
        assert!(receipt.rebaseline(&before.descriptor, after));
        assert!(receipt.permits(after));
        let mut foreign = after;
        foreign.bodies[1] = (0x7000, 458752);
        assert!(!receipt.permits(foreign), "an occupied empty cell is a foreign change");
        assert!(GroupExchange::new(before, after, Members::new(&[10, 11]).unwrap()).is_none(), "count must match");
        let back = GroupExchange::new(after, before, Members::new(&[3, 7, 8]).unwrap()).unwrap();
        assert!(back.rebaseline(&after.descriptor, before));
        assert_eq!(count_extra_groups(&[(one, GroupState::Built)]),
            GroupCounts { planned: 1, ready: 1, prefetched: 0, failed: 0, pending: 0, ..Default::default() });
        assert_eq!(count_extra_groups(&[(one, GroupState::Prefetched), (members(&[FOX, PIKACHU]), GroupState::Built)]),
            GroupCounts { planned: 3, ready: 2, prefetched: 1, failed: 0, pending: 0, ..Default::default() });
        assert_eq!(count_extra_groups(&[(one, GroupState::Failed)]),
            GroupCounts { planned: 1, ready: 0, prefetched: 0, failed: 1, pending: 0, ..Default::default() });
    }

    /// X6: the words a group's native work takes in `R` (its key requests and releases, its background tick, the worker's
    /// shadow `R`). A Mii group takes its identity's over the current words, as `with_mii_identity` always gave it. Beside
    /// a live Mii window every other group takes the owner's ordinary baseline with its own header kinds, the words its
    /// warm exit writes (`baseline_words`): the owner's costume and ordinary flags, not the Mii's. Beside an ordinary
    /// window nothing: `R` holds the owner's ordinary words already.
    fn each_group_takes_its_own_words_in_r() {
        let identity = MiiIdentity { kinds: [0x4a, u32::MAX, u32::MAX], costume: 2, index: 0, tokens: [75, 125, 0, 37], flags: [0, 1, 0] };
        let baseline: [u32; RESOURCE_WORDS] = [0x00, 0x01, 0x02, 3, 0xbeef_0000, 0, 0, 0, 0, 0x0001_0100, u32::MAX];
        let live_mii = identity.resource_words(baseline);
        assert_ne!(live_mii[3], baseline[3]);
        assert_ne!(live_mii[9], baseline[9]);
        let mii = Members::mii_group(0x4a).unwrap();
        assert_eq!(own_words(mii, Some(identity), false, None, baseline), Some(live_mii));
        assert_eq!(own_words(mii, Some(identity), true, Some(baseline), live_mii), Some(live_mii));
        for group in [Members::new(&[0x2d, 0x1c]).unwrap(), Members::native_group(0x24).unwrap(), Members::pair_group()] {
            let own = own_words(group, None, true, Some(baseline), live_mii);
            assert_eq!(own, Some(baseline_words(baseline, group)), "{group:?}");
            let own = own.unwrap();
            assert_eq!(&own[..3], &group.kinds());
            assert_eq!((own[3], own[4], own[9], own[10]), (3, 0xbeef_0000, 0x0001_0100, u32::MAX),
                "the owner's costume, record word and ordinary flags, not the Mii's");
            assert_eq!(own_words(group, None, false, Some(baseline), baseline), None, "{group:?}: an ordinary window");
            assert_eq!(own_words(group, None, true, None, live_mii), None, "{group:?}: no baseline (the build waits)");
        }
        // The worker's values from the identity bytes the tick wrote into the entry.
        let mut entry = [0u8; MII_ENTRY_LEN];
        entry[0] = 5;
        entry[0x46 - MII_ENTRY_START..0x48 - MII_ENTRY_START].copy_from_slice(&0x1234u16.to_le_bytes());
        entry[0x52 - MII_ENTRY_START] = 1;
        assert_eq!(shadow_values(&entry), 0x0105_1234);
        let window = [0u8; 0x70];
        assert_eq!(shadow_values(&identity.entry_bytes(&window)), u32::from(identity.index) | 2 << 16);
    }

    /// Primary 2026-11-09 04:22Z item 3 (the CPU on Pokémon Trainer with a Mii group): the eligibility, the plan order, the
    /// shadow words and the worker values take no owner. The same calls for owner 0 (P1) and owner 1 (the CPU) give the
    /// same answers; only the record index the planner stages differs (Player0 keeps the window's, the CPU takes its
    /// selector slot), as for every Mii group of that role.
    fn the_background_mii_path_beside_pokemon_trainer_is_the_same_for_both_roles() {
        use crate::engine::mii::backend::Layout;
        use crate::engine::mii::inventory::{Appearance, Canonical, MiiClass, MiiConfig, TaggedId, CHARINFO_LEN, CHARINFO_MASK_END};
        use crate::engine::mii::ordinary_cpu::staged_descriptor_for_owner;
        // `identity.rs`'s sample configuration (a Swordfighter preset).
        let mut info = [7u8; CHARINFO_LEN];
        info[..CHARINFO_MASK_END].fill(1);
        let mut masked = info;
        masked[..CHARINFO_MASK_END].fill(0);
        let canonical = Canonical { class: MiiClass::Swordfighter, appearance: Appearance {
            charinfo_masked: masked, specials: [1, 0, 0, 0], colour: 4, voice: 37,
            outfit: 0x14_7bc5_9435, hat: 0x0f_a9ef_a5ea, chara: MiiClass::Swordfighter.chara_hash40() } };
        let configuration = crate::engine::mii::lifetime::Configuration::from_saved(&MiiConfig {
            slot_id: 107, slot_index: 0, name: String::new(), charinfo: info,
            raw_tags: [TaggedId(0x13002b147bc59435), TaggedId(0x5a00000fa9efa5ea), TaggedId(0xc1004d15d7061f77)],
            canonical, copies: vec![] }).unwrap();
        let pt = Members::native_group(0x24).unwrap();
        let mut window = [0u8; 0x70];
        window[0x14..0x18].copy_from_slice(&3u32.to_le_bytes());
        for (i, kind) in pt.kinds().iter().enumerate() { window[0x18 + 4 * i..0x1c + 4 * i].copy_from_slice(&kind.to_le_bytes()); }
        window[0x28..0x2c].copy_from_slice(&1u32.to_le_bytes());
        window[0x41] = 2;
        let pt_words: [u32; RESOURCE_WORDS] = [0x24, 0x25, 0x26, 2, 0, 0, 0, 0, 0, 0x0001_0100, 0x24];
        let answers = [0usize, 1].map(|owner| {
            let staged = staged_descriptor_for_owner(Layout::Training, owner, window, Some(configuration)).unwrap();
            let identity = MiiIdentity::from_descriptor(&staged).unwrap();
            let mii = Members::mii_group(identity.kind()).unwrap();
            let eligible = background_build_eligible(mii, true, pt, LiveIdentity::Ordinary, false, false, false);
            let next = next_background([(GroupState::Built, Members::new(&[0x1c]).unwrap(), false),
                (GroupState::Prefetched, mii, true)].into_iter(), pt, LiveIdentity::Ordinary, false, false, false);
            let words = own_words(mii, Some(identity), false, None, pt_words).unwrap();
            let values = shadow_values(&identity.entry_bytes(&window));
            (eligible, next, words, values, identity.index)
        });
        assert_eq!((answers[0].0, answers[0].1), (Ok(()), Some(1)));
        assert_eq!((answers[0].0, answers[0].1), (answers[1].0, answers[1].1));
        let strip = |words: [u32; RESOURCE_WORDS], values: u32| {
            let mut w = words;
            w[4] &= 0xffff_0000;
            (w, values & 0xffff_0000)
        };
        assert_eq!(strip(answers[0].2, answers[0].3), strip(answers[1].2, answers[1].3), "only the record index differs");
        assert_eq!((answers[0].4, answers[1].4), (0, 1), "Player0 keeps the window's index, the CPU takes slot 1");
        assert_eq!((answers[0].2[4] & 0xffff, answers[1].2[4] & 0xffff), (0, 1));
        assert_eq!((answers[0].3 & 0xffff, answers[1].3 & 0xffff), (0, 1));
        assert_eq!(&answers[0].2[..4], &[0x49, u32::MAX, u32::MAX, 2], "the Swordfighter header with the role's costume byte");
    }

    /// Primary 2026-11-09 07:34Z (the 5 s release was a timeout guard; exactly the steps that serve the press go first):
    /// a press made during the prefetches, whose target's group is still planned, is served warm after that group's own
    /// prefetch, before the other groups'. Held back as well, the target would be in no reachable group: the canonical
    /// Reload, which destroys the window and every group. Each state of the target group, and the PT re-arm.
    fn a_waiting_press_lets_its_targets_prefetch_through() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum Work { Prefetch(usize), Press { warm: bool } }
        // Groups 0..3 planned; group 0's prefetch in flight at tick 0 (3 ticks); a press for group 2 at tick 1. The
        // lifecycle runs before the press each tick (`character_roster::service`), the worst case.
        let run = |serving: &dyn Fn(GroupState) -> Option<Serving>| {
            let mut states = [GroupState::Prefetching, GroupState::Planned, GroupState::Planned, GroupState::Planned];
            let (mut held_until, mut in_flight, mut press, mut served) = (3, Some(0), false, Vec::new());
            for tick in 0..40 {
                press |= tick == 1;
                if tick < held_until { continue; }
                if let Some(g) = in_flight.take() { states[g] = GroupState::Prefetched; }
                let next = states.iter().position(|s| *s == GroupState::Planned);
                let step = match speculative_step_admitted(press) {
                    Ok(()) => next,
                    Err(_) => (serving(states[2]) == Some(Serving::Prefetch)).then_some(2),
                };
                if let Some(g) = step {
                    states[g] = GroupState::Prefetching;
                    served.push((tick, Work::Prefetch(g)));
                    (held_until, in_flight) = (tick + 3, Some(g));
                } else if press {
                    served.push((tick, Work::Press { warm: states[2].reachable() }));
                    (press, held_until) = (false, tick + 1);
                }
            }
            served
        };
        let served = run(&|state| serving_step(state, true, false));
        assert_eq!(&served[..2], &[(3, Work::Prefetch(2)), (6, Work::Press { warm: true })], "{served:?}");
        assert_eq!(served[2..].iter().map(|(_, w)| *w).collect::<Vec<_>>(), [Work::Prefetch(1), Work::Prefetch(3)]);
        // Nothing let through (the cap-era rule): the press is served at once into no reachable group.
        assert_eq!(run(&|_| None)[0], (3, Work::Press { warm: false }));
        // Each state of the target's group.
        assert_eq!(serving_step(GroupState::Planned, true, false), Some(Serving::Prefetch));
        assert_eq!(serving_step(GroupState::Held(Hold::Memory), true, false), Some(Serving::Prefetch));
        assert_eq!(serving_step(GroupState::Planned, false, false), None, "a Mii group's CharInfo is published first");
        for state in [GroupState::Prefetching, GroupState::Prefetched, GroupState::Built, GroupState::Failed] {
            assert_eq!(serving_step(state, true, false), None, "{state:?}");
        }
        // A built PT group without its object kind: its re-arm makes it reachable (`object_ready`).
        assert_eq!(serving_step(GroupState::Built, true, true), Some(Serving::ObjectRearm));
    }

    /// The notice rule (primary 2026-11-09 02:00Z, approved for every kind): with groups on, the live attached window
    /// stays while the role's other groups are built around it, so it plans its own reusable extras. Eden rc6-bg-ic5 and
    /// rc6-xc2/3 showed "swap P1 to load 2 more" on a one-member group (Samus: "Loaded 10/12"), on the pair group
    /// ("Loaded 8/10") and for the CPU on its small groups, with everything planned built: the planner counted the
    /// window the lane's next kind would get (two extras) where the attached group serves none.
    fn a_live_attached_group_plans_what_it_serves() {
        use crate::engine::preload::cycle_preload_capacity::reusable_extras;
        let next = reusable_extras(&[DK, LINK, SAMUS], DK, None);
        assert_eq!(next, 2);
        // P1 on Samus (one-member group) with the window [Mario, DK, Link] and the pair group detached and built.
        let detached = count_extra_groups(&[(members(&[MARIO, DK, LINK]), GroupState::Built),
            (Members::pair_group(), GroupState::Built)]);
        for (live, active) in [(members(&[SAMUS]), SAMUS), (Members::pair_group(), POPO), (members(&[0x0f]), 0x0f)] {
            let (kinds, count) = live.logical_kinds();
            let extra = reusable_extras(&kinds[..count], active, None);
            assert_eq!(extra, 0, "{active:#x}: a one-member group serves only its active member");
            let planned = window_targets(true, Some((live, active)), None, next) + detached.planned;
            let role = role_counts(planned, extra, WindowStatus::Built, detached);
            assert_eq!((role.planned, role.ready, role.unstarted(), role.swap_owed(0)), (detached.planned, detached.ready, 0, 0),
                "{active:#x}: the notice reads Ready");
        }
        // A three-member group or window serves its two extras either way.
        assert_eq!(window_targets(true, Some((members(&[LINK, SAMUS, YOSHI]), LINK)), None, next), 2);
        // Groups off: the next swap builds the lane's window, as before.
        assert_eq!(window_targets(false, Some((members(&[SAMUS]), SAMUS)), None, next), next);
    }

    /// SPECIAL-GROUPS §0 X4 (primary 2026-11-09 00:52Z, the cross cells), from the disassembly
    /// `rc6-item3-re/special/asm/6079d0-60df60.s` `0x60d1ec-0x60d26c`. The construction picks the entry info wrapper whose
    /// `info+0x94` gate it reads. Only a Popo or Nana body looks at the entry's words: with two cells (count ≥ 2, cell 1
    /// not −1) cell 0 == kind gives `+0xf0`, cell 1 == kind gives `+0x100` (Nana's own), anything else `+0xf0`. Every
    /// other kind gives `+0xf0`. Native reads the live entry; a background body is given its building group's words, so
    /// Nana of a pair group built beside a one-actor window (an ordinary or native window) gets her own wrapper, as in her
    /// synchronous build (Eden re6: only the pair's Nana used `+0x100`).
    fn the_info_wrapper_follows_the_words_it_is_given() {
        let pair = [POPO, NANA];
        assert_eq!((info_wrapper(POPO, 2, pair), info_wrapper(NANA, 2, pair)), (0xf0, 0x100), "the pair group");
        assert_eq!((info_wrapper(POPO, 3, pair), info_wrapper(NANA, 3, pair)), (0xf0, 0x100), "a pair-lead window");
        for (count, cells) in [(3, [MARIO, DK]), (3, [0x24, 0x25]), (2, [0x5a, 0x5b]), (1, [MARIO, u32::MAX])] {
            assert_eq!((info_wrapper(POPO, count, cells), info_wrapper(NANA, count, cells)), (0xf0, 0xf0),
                "a live one-actor window {cells:x?}: Nana is in neither cell");
        }
        assert_eq!(info_wrapper(NANA, 1, pair), 0xf0, "one cell");
        assert_eq!(info_wrapper(NANA, 2, [POPO, u32::MAX]), 0xf0, "cell 1 empty");
        assert_eq!(info_wrapper(POPO, 1, [POPO, u32::MAX]), 0xf0, "Solo Popo");
        for kind in [MARIO, 0x24, 0x25, 0x5b, 0x48] {
            assert_eq!(info_wrapper(kind, 2, [POPO, kind]), 0xf0, "{kind:#x}: only Ice Climbers bodies choose");
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_group_leaves_when_a_member_left_the_cycle_and_its_slot_is_reused", a_group_leaves_when_a_member_left_the_cycle_and_its_slot_is_reused as fn()),
            ("an_identity_moves_only_into_or_out_of_a_mii_window", an_identity_moves_only_into_or_out_of_a_mii_window as fn()),
            ("a_parked_group_counts_only_the_members_its_cycle_keeps", a_parked_group_counts_only_the_members_its_cycle_keeps as fn()),
            ("a_change_that_does_not_fit_frees_just_enough_parked_bodies", a_change_that_does_not_fit_frees_just_enough_parked_bodies as fn()),
            ("nothing_parked_and_a_real_shortage_still_waits_for_memory", nothing_parked_and_a_real_shortage_still_waits_for_memory as fn()),
            ("a_release_in_progress_shows_loading_never_waiting", a_release_in_progress_shows_loading_never_waiting as fn()),
            ("a_parked_group_leaves_on_the_entry_tick_only_when_marked", a_parked_group_leaves_on_the_entry_tick_only_when_marked as fn()),
            ("a_background_build_takes_native_groups_and_builds_beside_a_live_native_window", a_background_build_takes_native_groups_and_builds_beside_a_live_native_window as fn()),
            ("a_retired_trainer_group_takes_its_object_kind_again_before_its_rebuild", a_retired_trainer_group_takes_its_object_kind_again_before_its_rebuild as fn()),
            ("a_trainer_object_rearm_is_admitted_under_a_live_kirby_and_a_fighter_prefetch_is_not", a_trainer_object_rearm_is_admitted_under_a_live_kirby_and_a_fighter_prefetch_is_not as fn()),
            ("tick_gap_reports_the_largest_stall_between_ticks", tick_gap_reports_the_largest_stall_between_ticks as fn()),
            ("dispatcher_words_gate_the_array_exchange", dispatcher_words_gate_the_array_exchange as fn()),
            ("the_mii_setup_seams_are_the_bank_s_ready_and_awaiting_predicates", the_mii_setup_seams_are_the_bank_s_ready_and_awaiting_predicates as fn()),
            ("solo_popo_is_an_upcoming_member_pinned_to_cell_0_and_the_later_pinned_kind_waits", solo_popo_is_an_upcoming_member_pinned_to_cell_0_and_the_later_pinned_kind_waits as fn()),
            ("a_pinned_window_reload_settles_on_its_target_cell_not_cell_0", a_pinned_window_reload_settles_on_its_target_cell_not_cell_0 as fn()),
            ("the_rest_of_the_lane_is_planned_as_groups_beside_the_target", the_rest_of_the_lane_is_planned_as_groups_beside_the_target as fn()),
            ("the_neighbours_reach_the_planner_only_for_their_own_window", the_neighbours_reach_the_planner_only_for_their_own_window as fn()),
            ("a_target_first_reload_keeps_the_built_groups_it_does_not_displace", a_target_first_reload_keeps_the_built_groups_it_does_not_displace as fn()),
            ("a_role_without_a_bank_still_counts_the_window_its_next_swap_prepares", a_role_without_a_bank_still_counts_the_window_its_next_swap_prepares as fn()),
            ("a_switch_with_no_window_to_rebuild_keeps_its_canonical_reload", a_switch_with_no_window_to_rebuild_keeps_its_canonical_reload as fn()),
            ("both_banks_with_a_deferred_group_build_then_admit_it", both_banks_with_a_deferred_group_build_then_admit_it as fn()),
            ("a_canonical_reload_of_the_window_shape_retires_a_one_member_bank", a_canonical_reload_of_the_window_shape_retires_a_one_member_bank as fn()),
            ("eight_ready_fighters_default_gives_five_and_three", eight_ready_fighters_default_gives_five_and_three as fn()),
            ("group_state_machine_commit_index_and_counts", group_state_machine_commit_index_and_counts as fn()),
            ("mii_groups_are_one_member_bank_windows_with_one_identity_per_role", mii_groups_are_one_member_bank_windows_with_one_identity_per_role as fn()),
            ("a_mii_group_identity_is_the_staged_descriptor_and_changes_with_its_hat_or_class", a_mii_group_identity_is_the_staged_descriptor_and_changes_with_its_hat_or_class as fn()),
            ("an_ordinary_request_leaves_a_mii_window_for_the_window_it_displaced", an_ordinary_request_leaves_a_mii_window_for_the_window_it_displaced as fn()),
            ("a_one_member_mii_bank_window_is_ready_only_with_bank_flags_at_index_zero", a_one_member_mii_bank_window_is_ready_only_with_bank_flags_at_index_zero as fn()),
            ("one_member_groups_are_valid_members_with_empty_padding_cells", one_member_groups_are_valid_members_with_empty_padding_cells as fn()),
            ("each_group_takes_its_own_words_in_r", each_group_takes_its_own_words_in_r as fn()),
            ("the_background_mii_path_beside_pokemon_trainer_is_the_same_for_both_roles", the_background_mii_path_beside_pokemon_trainer_is_the_same_for_both_roles as fn()),
            ("a_waiting_press_lets_its_targets_prefetch_through", a_waiting_press_lets_its_targets_prefetch_through as fn()),
            ("a_live_attached_group_plans_what_it_serves", a_live_attached_group_plans_what_it_serves as fn()),
            ("the_info_wrapper_follows_the_words_it_is_given", the_info_wrapper_follows_the_words_it_is_given as fn()),
        ]);
    }
    }
}
