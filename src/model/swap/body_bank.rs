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
pub use crate::engine::swap::swap_model::capabilities::{
    group_member_kind, key_release_admitted, kirby_parkable, member_forms, member_forms_in,
    native_group_request, original_bankable, original_form_bankable, pair_lead_extra, pair_request, prefetch_admitted,
    single_member_request, special_group_kind, swap_commit_index, swap_commit_index_in,
    swappable_kind, window_swappable, window_swappable_with,
};
use crate::engine::swap::swap_model::capabilities::{bankable_kind, pins_cell0};
use crate::engine::preload::cycle_preload_capacity::MAX_TARGETS;
// WP4: the resource bookkeeping wrappers below are thin views of the ledger's data.
use crate::engine::swap::swap_model::ledger::{native_window_requests, orphan_keys, pair_rule, RFlags};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Idle, Building, Ready, Changing, Retired }

impl Phase {
    /// A press during construction/switching is never queued as another change.
    /// A retired private probe must leave the mapped controller frame alone.
    pub fn accepts_input(self) -> bool { matches!(self, Self::Idle | Self::Ready) }
}

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
    /// group is an extra swap group only: a window bank always starts with two
    /// members ([`Members::from_forms`]). A kind whose row pins cell 0
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
    pub const NATIVE: Self = Self { native: true, pair: false, mii: false };
    pub const ALL: Self = Self { native: true, pair: true, mii: false };
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
/// group returns its keys at once (it costs nothing); every other state holds nothing native.
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
/// window holds no Mii identity (the costume selectors at `0x607dfc`/`0x17e9ae0` key on `R+0x58`) and its live member
/// is not Kirby (D-KIRBY1: no bank key rests on its R).
///
/// The live window (`window`, the attached group):
/// - a pair window (`window.is_pair()`: the pair-lead window or the pair group attached; a session that starts on Ice
///   Climbers) takes part when the construction tail's pair test is redirected for background bodies (`pair_tail`,
///   `0x60de68`, SPECIAL-GROUPS §0 X1), so their tails take the per-kind release instead of the pair rule on R (which may
///   reset the whole R mid-play); otherwise it defers the build ("pair window").
/// - a native window (Pokémon Trainer, Pyra/Mythra; a session that starts on one) takes part: one actor, its cells 0/1
///   never `[Popo, Nana]`; only an Ice Climbers body picks its info wrapper from the live kinds (X4, `0x60d1f0`), so the
///   pair group waits for another window there.
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
///   `SHADOW_R`). A live Mii window defers it (that window's faces and resource object in `R` are a case not built yet),
///   and so do a live native or pair window until a Mii body beside one is proven.
///
/// Every other group keeps today's build at its first switch.
pub fn background_build_eligible(members: Members, identity: bool, window: Members, window_identity: bool, live_kirby: bool,
    pair_tail: bool) -> Result<(), &'static str> {
    if members.special() == Special::Mii && !identity { return Err("mii without identity"); }
    if members.special() == Special::Mii && (window.is_pair() || matches!(window.special(), Special::Native(_))) {
        return Err("special window");
    }
    if members.special() == Special::Pair && matches!(window.special(), Special::Native(_)) { return Err("special window"); }
    if identity || !matches!(members.special(), Special::Native(_) | Special::Pair) { async_build_eligible(members, identity)?; }
    if window_identity { return Err("mii window"); }
    if live_kirby { return Err("live kirby"); }
    if window.is_pair() && !pair_tail { return Err("pair window"); }
    Ok(())
}

/// The group a background build starts next: the first Prefetched group in plan order that is eligible
/// (`background_build_eligible`) beside the live `window`, from `(state, members, identity)`.
pub fn next_background(groups: impl Iterator<Item = (GroupState, Members, bool)>, window: Members, window_identity: bool,
    live_kirby: bool, pair_tail: bool) -> Option<usize> {
    groups.enumerate().find(|(_, (state, members, identity))| *state == GroupState::Prefetched
        && background_build_eligible(*members, *identity, window, window_identity, live_kirby, pair_tail).is_ok())
        .map(|(i, _)| i)
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
            if (is_mii && existing[role].iter().any(|held| held.special() == Special::Mii))
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

pub fn count_extra_groups(groups: &[(Members, GroupState)]) -> GroupCounts {
    count_extra_groups_iter(groups.iter().copied())
}

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

/// Operator decision 2026-11-08 23:42Z: the new members waiting for the pause. `after_pause` is what the planner places
/// with the parked groups gone; `held` says whether any group (parked ones included) holds a kind now. Every member it
/// places that nothing holds is planned nowhere until the pause frees the parked bodies.
pub fn waiting_for_pause(after_pause: &[Option<Members>], held: impl Fn(u32) -> bool) -> u8 {
    after_pause.iter().flatten()
        .flat_map(|members| { let kinds = members.kinds(); (0..members.count()).map(move |i| kinds[i]) })
        .filter(|kind| !held(*kind)).count().min(usize::from(u8::MAX)) as u8
}

impl GroupCounts {
    /// `waiting` members held until the pause frees the parked groups' bodies: planned, and shown as waiting for memory.
    pub fn with_waiting(mut self, waiting: u8) -> GroupCounts {
        if waiting > 0 {
            self.planned = self.planned.saturating_add(waiting);
            self.held = self.held.saturating_add(waiting);
            self.hold = merge_hold(self.hold, Some(Hold::Memory));
        }
        self
    }
}

/// The lane window's planned targets (`next` for [`window_targets`]): the reusable extras of the window the planner wants
/// for its next lane kind `target` (`shown`, target first). A group that holds the target already is that window (an exchange left it detached, e.g.
/// while a Mii group is live) and counts with the groups; a member a group holds counts there too (rc6-hot-xps19).
pub fn lane_window_targets(shown: &[u32], target: u32, original: Option<u32>, grouped: impl Fn(u32) -> bool) -> u8 {
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

/// The window part of a role's planned targets (`plan_budget`): `next`, the reusable extras of the window the lane's
/// next kind would get, except for a live pair-lead window (`live`: its members and active kind) with groups on. That
/// window stays while the role's groups build around it (rc.6 item 3, a session that starts on Ice Climbers), so it
/// plans what it serves: its reusable extras, the count the notice reads (`role_counts`' `extra`), not the lane
/// window's (which has one more member than the pair's single extra cell).
pub fn window_targets(grouped: bool, live: Option<(Members, u32)>, original: Option<u32>, next: u8) -> u8 {
    match live {
        Some((members, active)) if grouped && members.is_pair_lead() => {
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
    pub fn new(before: GroupOwner, after: GroupOwner, members: Members) -> Option<Self> {
        Self::new_with_header(before, after, members, before.header)
    }

    /// `header` is the shared resource header the exchange must leave behind:
    /// the window's own for ordinary, native and pair groups (unchanged across the
    /// exchange, D-PTA1), the Mii identity header `[owner, mii, MAX, MAX, costume]`
    /// once a Mii group is built or attached, and the restored previous header
    /// when a Mii window is left (MII-GROUPS-DESIGN §3).
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
    pub fn colour(self) -> u16 { self.tokens[2] }
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
    #[test]
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

    /// rc.6 warm exit to any Built group (inbox 21:27Z): leaving a Mii group for an ordinary window that saved no identity
    /// move writes the owner's ordinary baseline (the words the first ordinary window a Mii displaced left in `R`) with
    /// that window's own header kinds; the costume, the Mii words and the flags stay the baseline's.
    #[test]
    fn the_ordinary_baseline_takes_the_target_windows_header() {
        let baseline: [u32; RESOURCE_WORDS] = std::array::from_fn(|i| 0x100 + i as u32);
        let window = Members::new(&[0x00, 0x01, 0x02]).unwrap();
        let group = Members::new(&[0x03, 0x07]).unwrap();
        let words = baseline_words(baseline, group);
        assert_eq!(&words[..3], &[0x03, 0x07, u32::MAX]);
        assert_eq!(&words[3..], &baseline[3..], "costume, record index, tokens and flags stay the baseline's");
        assert_eq!(&baseline_words(baseline, window)[..3], &[0x00, 0x01, 0x02]);
        let trainer = Members::native_group(0x24).unwrap();
        assert_eq!(&baseline_words(baseline, trainer)[..3], &trainer.kinds());
    }

    /// Eden rc6-hot-xps15: after the exit from a Mii went to Mario's window through the baseline, [Samus, Fox] still held
    /// the move it saved when the Mii displaced it, and its later attach out of Mario's (ordinary) window wrote it: the
    /// Player adapter's owned state changed and the switch settled unowned. A warm attach writes an identity only when a
    /// Mii is involved: a Mii target its own; out of a Mii window an ordinary target its saved move, else the baseline;
    /// between two ordinary windows nothing.
    #[test]
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

    /// Operator decision 2026-11-08 23:42Z (inbox): a Built group that left the cycle stays parked until Training's pause
    /// screen opens (native's body destroy costs 75-134 ms per body on the game thread, rc6-hot-xps16); a Prefetched one
    /// returns its keys at once; a prefetch in flight settles first; nothing native exists for the rest.
    #[test]
    fn a_built_group_that_left_the_cycle_is_freed_under_the_pause() {
        assert!(leaves_at_pause(GroupState::Built));
        for state in [GroupState::Planned, GroupState::Prefetching, GroupState::Prefetched, GroupState::Failed,
            GroupState::Held(Hold::Memory), GroupState::Released] {
            assert!(!leaves_at_pause(state), "{state:?}");
        }
    }

    /// rc6-hot-xps18 `mb-p1-change`: P1 dropped Fox, so `[Samus, Fox]` is parked until the pause. Samus is still served
    /// warm from it and counts as built; Fox left and counts nowhere (counting the whole group out showed Samus as
    /// "swap P1 to load", counting it whole showed Fox as planned). A native group stays whole while any form is wanted.
    #[test]
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

    /// rc6-hot-xps19 `mb-align` (and xps16/xps17 laps): with the CPU's Mii group live, the planner's window target is the
    /// next lane kind's window `[Peach, Falcon]`, which the exchange left in a detached record; counted once as the
    /// window's target and once with the groups, the notice read "Loaded 7/8 · swap the CPU to load 1 more" with every
    /// member built. A window target a group already holds is that group's, not the window's.
    #[test]
    fn a_window_target_a_group_holds_is_counted_once() {
        let none = |_: u32| false;
        assert_eq!(lane_window_targets(&[0x0d, 0x0b], 0x0d, None, none), 1);
        assert_eq!(lane_window_targets(&[0x0d, 0x0b], 0x0d, Some(0x0b), none), 0);
        let displaced = |kind: u32| [0x0d, 0x0b].contains(&kind);
        assert_eq!(lane_window_targets(&[0x0d, 0x0b], 0x0d, None, displaced), 0);
        assert_eq!(lane_window_targets(&[0x01, 0x00, 0x02], 0x01, None, |kind| [0x00, 0x01, 0x02].contains(&kind)), 0);
        assert_eq!(lane_window_targets(&[0x01, 0x00, 0x02], 0x01, None, |kind| kind == 0x02), 1);
    }

    /// Operator decision 2026-11-08 23:42Z: a new member the shared budget or a free group slot cannot take while a
    /// parked group still holds them waits for the pause, shown as "(N waiting for memory)"; it is planned and built
    /// in the background once the pause has freed the parked bodies. rc6-hot-xps18 `mb-cpu-change`: the CPU's Marth
    /// was planned nowhere while its parked Mii group and P1's parked pair held the budget, and the notice never
    /// named it. The count is the members the plan without the parked groups places that no group holds now.
    #[test]
    fn a_new_member_waits_for_the_pause_when_parked_groups_hold_the_budget() {
        let newcomer = Members::new(&[0x16]).unwrap();
        let samus = Members::new(&[0x03]).unwrap();
        let held = |kind: u32| [0x03, 0x07, 0x4a].contains(&kind);
        assert_eq!(waiting_for_pause(&[Some(newcomer), None, None], held), 1);
        assert_eq!(waiting_for_pause(&[Some(samus), None, None], held), 0);
        assert_eq!(waiting_for_pause(&[Some(Members::new(&[0x03, 0x16]).unwrap()), None, None], held), 1);
        assert_eq!(waiting_for_pause(&[None, None, None], held), 0);
        let counts = count_group_members([(2, GroupState::Built)]).with_waiting(1);
        assert_eq!((counts.planned, counts.ready, counts.held, counts.hold), (3, 2, 1, Some(Hold::Memory)));
        assert_eq!(count_group_members([(2, GroupState::Built)]).with_waiting(0), count_group_members([(2, GroupState::Built)]));
    }

    /// rc.6 item 3: the notice counts a prefetched group a background build still owes as loading (pending), so it reads
    /// "Loading x/y" until that group is built; every other state is shown as it is.
    #[test]
    fn a_group_owed_a_background_build_is_shown_loading() {
        assert_eq!(shown_state(GroupState::Prefetched, true), GroupState::Prefetching);
        assert_eq!(shown_state(GroupState::Prefetched, false), GroupState::Prefetched);
        for state in [GroupState::Planned, GroupState::Prefetching, GroupState::Built, GroupState::Failed, GroupState::Held(Hold::Pool)] {
            assert_eq!(shown_state(state, true), state);
        }
        let ordinary = Members::new(&[0x2d, 0x2c, 0x1c]).unwrap();
        let owed = count_extra_groups_iter([(ordinary, shown_state(GroupState::Prefetched, true))]);
        assert_eq!((owed.planned, owed.prefetched, owed.pending), (3, 0, 3));
    }

    #[test]
    fn a_background_build_takes_an_ordinary_prefetched_group_beside_an_ordinary_window() {
        let ordinary = Members::new(&[0x2d, 0x2c, 0x1c]).unwrap();
        let window = Members::new(&[0x00, 0x01, 0x02]).unwrap();
        assert_eq!(background_build_eligible(ordinary, false, window, false, false, false), Ok(()));
        assert_eq!(background_build_eligible(Members::new(&[0x1c]).unwrap(), false, window, false, false, false), Ok(()));
        // SPECIAL-GROUPS §1 (operator 20:26Z, "All groups in rc.6"): a Mii group with its identity builds in the
        // background too (its R, entry and D words bracketed in the tick, shadow R on the worker); never without one.
        assert_eq!(background_build_eligible(Members::mii_group(0x4a).unwrap(), true, window, false, false, false), Ok(()));
        assert_eq!(background_build_eligible(Members::mii_group(0x4a).unwrap(), false, window, false, false, false), Err("mii without identity"));
        assert_eq!(background_build_eligible(Members::mii_group(0x4a).unwrap(), true, window, true, false, false), Err("mii window"));
        assert_eq!(background_build_eligible(ordinary, true, window, false, false, false), Err("identity"));
        // SPECIAL-GROUPS §2: the pair group builds in the background (`a_background_build_takes_the_pair_group_…`).
        assert_eq!(background_build_eligible(Members::pair_group(), false, window, false, false, false), Ok(()));
        assert_eq!(background_build_eligible(ordinary, false, window, true, false, false), Err("mii window"));
        assert_eq!(background_build_eligible(ordinary, false, window, false, true, false), Err("live kirby"));
        // SPECIAL-GROUPS §0 X1: a body's construction tail reads the live cells 0/1; `[Popo, Nana]` takes the pair rule
        // (and may reset the whole R) unless the tail is redirected (`pair_tail`).
        for pair in [Members::pair_group(), Members::pair_lead(0x02).unwrap()] {
            assert_eq!(background_build_eligible(ordinary, false, pair, false, false, false), Err("pair window"));
        }
        // bg-pt: every group but the pair group and a Mii group builds beside a live native window.
        assert_eq!(background_build_eligible(ordinary, false, Members::native_group(0x24).unwrap(), false, false, false), Ok(()));
        let groups = [(GroupState::Built, ordinary, false), (GroupState::Prefetched, Members::mii_group(0x48).unwrap(), true),
            (GroupState::Planned, ordinary, false), (GroupState::Prefetched, Members::new(&[0x1c]).unwrap(), false),
            (GroupState::Prefetched, ordinary, false)];
        assert_eq!(next_background(groups.iter().copied(), window, false, false, false), Some(1));
        assert_eq!(next_background(groups[2..].iter().copied(), window, false, false, false).map(|i| i + 2), Some(3));
        assert_eq!(next_background(groups.iter().copied(), window, true, false, false), None);
        assert_eq!(next_background(groups.iter().copied(), window, false, true, false), None);
        assert_eq!(next_background(groups.iter().copied(), Members::pair_group(), false, false, false), None);
        assert_eq!(next_background(groups[..3].iter().copied(), window, false, false, false), Some(1));
        assert_eq!(next_background([groups[0], groups[2]].into_iter(), window, false, false, false), None);
        // The saved info spans cover what `0x654dc0` writes: the dispatcher words, the descriptor copies, `+0x3ac`.
        let covered = |at: usize| BACKGROUND_INFO_SPANS.iter().any(|(start, len)| (*start..start + len).contains(&at));
        for at in [0x08, 0x10, 0x18, 0x20, 0x28, 0x2c, 0x30, 0x34, 0x50, 0x51, 0x6b, 0x72, 0x78, 0x84, 0x92, 0x3a0, 0x3aa, 0x3ac] {
            assert!(covered(at), "info+{at:#x}");
        }
    }

    /// rc.6 item 3, SPECIAL-GROUPS §3 (operator 2026-11-08 "All groups in rc.6"): a Pokémon Trainer or Pyra/Mythra group
    /// builds in the background beside an ordinary window, and every group builds beside a live native window, so a
    /// session that starts on Pokémon Trainer still gets its groups built (inbox 21:27Z). §0: a native window is one
    /// actor whose cells 0/1 are never `[Popo, Nana]` (X1), and only an Ice Climbers body picks its info wrapper from
    /// the live cells (X4, `0x60d1f0`). The holder step each native group takes after its bodies, and the X5 cell.
    #[test]
    fn a_background_build_takes_native_groups_and_builds_beside_a_live_native_window() {
        let ordinary = Members::new(&[0x2d, 0x2c, 0x1c]).unwrap();
        let window = Members::new(&[0x00, 0x01, 0x02]).unwrap();
        let pt = Members::native_group(0x25).unwrap();
        let aegis = Members::native_group(0x5b).unwrap();
        for native in [pt, aegis] {
            assert_eq!(background_build_eligible(native, false, window, false, false, false), Ok(()));
            assert_eq!(background_build_eligible(ordinary, false, native, false, false, false), Ok(()));
            assert_eq!(background_build_eligible(Members::new(&[0x1c]).unwrap(), false, native, false, false, false), Ok(()));
            assert_eq!(background_build_eligible(native, false, window, true, false, false), Err("mii window"));
            assert_eq!(background_build_eligible(native, false, window, false, true, false), Err("live kirby"));
            assert_eq!(background_build_eligible(native, false, Members::pair_group(), false, false, false), Err("pair window"));
            assert_eq!(background_build_eligible(native, true, window, false, false, false), Err("native group"));
            // Its switch-time build stays `0x658af0`'s.
            assert_eq!(async_build_eligible(native, false), Err("native group"));
        }
        assert_eq!(background_build_eligible(pt, false, aegis, false, false, false), Ok(()));
        assert_eq!(background_build_eligible(aegis, false, pt, false, false, false), Ok(()));
        // A Mii group (with its identity) builds in the background beside an ordinary window (p23/rc6-hot-slots-mii), and
        // still defers beside a live native window until a Mii body beside one is proven.
        assert_eq!(background_build_eligible(Members::mii_group(0x48).unwrap(), true, pt, false, false, false), Err("special window"));
        assert_eq!(background_build_eligible(Members::mii_group(0x48).unwrap(), true, window, false, false, false), Ok(()));
        let groups = [(GroupState::Built, ordinary, false), (GroupState::Prefetched, pt, false),
            (GroupState::Prefetched, aegis, false)];
        assert_eq!(next_background(groups.iter().copied(), window, false, false, false), Some(1));
        assert_eq!(next_background(groups[2..].iter().copied(), pt, false, false, false), Some(0));
        assert_eq!(next_background([(GroupState::Prefetched, ordinary, false)].into_iter(), pt, false, false, false), Some(0));
        assert_eq!(next_background(groups.iter().copied(), Members::pair_group(), false, false, false), None);
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

    /// rc.6 item 3, Ice Climbers (SPECIAL-GROUPS §2): the pair group builds in the background beside an ordinary window
    /// (its tails then take the per-kind release `0x17e8610` on the live cells, which returns exactly its own keys). A
    /// live `[Popo, Nana]` window (a session that starts on Ice Climbers, or the pair group attached) no longer defers
    /// every other group, once the construction tail's pair test is redirected for background bodies (`pair_tail`, X1).
    /// Beside a live native window the pair group waits (X4); every other group builds there (bg-pt).
    #[test]
    fn a_background_build_takes_the_pair_group_and_builds_beside_a_live_pair_window() {
        let ordinary = Members::new(&[0x2d, 0x2c, 0x1c]).unwrap();
        let window = Members::new(&[0x00, 0x01, 0x02]).unwrap();
        for pair_tail in [false, true] {
            assert_eq!(background_build_eligible(Members::pair_group(), false, window, false, false, pair_tail), Ok(()));
            assert_eq!(background_build_eligible(Members::pair_group(), false, window, true, false, pair_tail), Err("mii window"));
            assert_eq!(background_build_eligible(Members::pair_group(), false, window, false, true, pair_tail), Err("live kirby"));
            // bg-pt: a live native window takes every group but the pair group (X4) and a Mii group.
            assert_eq!(background_build_eligible(ordinary, false, Members::native_group(0x24).unwrap(), false, false, pair_tail), Ok(()));
            assert_eq!(background_build_eligible(Members::pair_group(), false, Members::native_group(0x24).unwrap(), false, false,
                pair_tail), Err("special window"));
        }
        for live_pair in [Members::pair_group(), Members::pair_lead(0x02).unwrap()] {
            assert!(live_pair.is_pair());
            assert_eq!(background_build_eligible(ordinary, false, live_pair, false, false, true), Ok(()));
            assert_eq!(background_build_eligible(Members::new(&[0x1c]).unwrap(), false, live_pair, false, false, true), Ok(()));
            assert_eq!(background_build_eligible(ordinary, false, live_pair, false, false, false), Err("pair window"));
        }
        // The switch-time build of the pair group stays synchronous (`mode=sync reason=pair group`).
        assert_eq!(async_build_eligible(Members::pair_group(), false), Err("pair group"));
        // A session that starts on Ice Climbers: its other groups get built.
        let groups = [(GroupState::Prefetched, ordinary, false), (GroupState::Prefetched, Members::new(&[0x1c]).unwrap(), false)];
        assert_eq!(next_background(groups.iter().copied(), Members::pair_lead(0x02).unwrap(), false, false, true), Some(0));
        assert_eq!(next_background(groups.iter().copied(), Members::pair_lead(0x02).unwrap(), false, false, false), None);
        // The pair group as the next build beside an ordinary window.
        let with_pair = [(GroupState::Built, ordinary, false), (GroupState::Prefetched, Members::pair_group(), false)];
        assert_eq!(next_background(with_pair.iter().copied(), window, false, false, false), Some(1));
    }

    /// Phase B′: an ordinary group without a Mii identity and a Mii group (operator decision
    /// 2026-11-08, rc.6 item 2: the 2026-11-02 restriction lifted for Mii groups; console F6: the
    /// CPU's synchronous Mii build held the game thread 141 ms) build asynchronously; the two
    /// remaining excluded classes (operator decision 2026-11-02) and a pair-lead window name their reason.
    #[test]
    fn async_build_admits_ordinary_and_mii_groups_and_names_each_exclusion() {
        let ordinary = Members::new(&[0x2d, 0x2c, 0x1c]).unwrap();
        assert_eq!(async_build_eligible(ordinary, false), Ok(()));
        assert_eq!(async_build_eligible(Members::new(&[0x2d, 0x2c]).unwrap(), false), Ok(()));
        assert_eq!(async_build_eligible(Members::new(&[0x1c]).unwrap(), false), Ok(()));
        assert_eq!(async_build_eligible(ordinary, true), Err("identity"));
        for class in [0x48, 0x49, 0x4a] {
            assert_eq!(async_build_eligible(Members::mii_group(class).unwrap(), true), Ok(()), "{class:#x}");
        }
        assert_eq!(async_build_eligible(Members::native_group(0x24).unwrap(), false), Err("native group"));
        assert_eq!(async_build_eligible(Members::native_group(0x5a).unwrap(), false), Err("native group"));
        assert_eq!(async_build_eligible(Members::pair_group(), false), Err("pair group"));
        assert_eq!(async_build_eligible(Members::pair_lead(0x08).unwrap(), false), Err("pair window"));
    }

    /// The construction task layout the adapter replicates and the settlement rule state 0xb applies.
    #[test]
    fn construction_task_layout_and_settlement_follow_state_10() {
        use construction_task::*;
        assert_eq!((SIZE, ALIGN), (0x90, 0x10));
        assert_eq!((BODY, HANDLE, PENDING, QUEUE, QUEUE_ID), (0x00, 0x10, 0x20, 0x24, 1));
        assert_eq!((FUNCTOR, FUNCTOR_PTR, CANCEL), (0x50, 0x70, 0x80));
        assert_eq!(FUNCTOR_PTR, HANDLE + 0x60);
        assert_eq!(CANCEL, HANDLE + 0x70);
        assert_eq!(SCHEDULE_ARGS, (2, 1));
        assert!(settled(&[0, 0, 0]));
        assert!(settled(&[]));
        assert!(!settled(&[0, 1, 0]));
        assert!(!settled(&[2]));
    }

    /// The tick-gap meter reports the one stalled tick of a synchronous build and stays quiet
    /// for an asynchronous one; disarmed it ignores later ticks.
    #[test]
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

    #[test]
    fn group_exchange_rebaselines_both_roles_and_warm_return() {
        for owner in [0,1,4] {
            let (before, after, members) = exchanged_owner(owner);
            let receipt = GroupExchange::new(before,after,members).unwrap();
            assert!(receipt.rebaseline(&before.descriptor,after));
            assert!(receipt.permits(after)); // later opposite-role owner walk
            assert!(!receipt.rebaseline(&after.descriptor,after));
            let warm = GroupExchange::new(after,before,Members::new(&[3,7,8]).unwrap()).unwrap();
            assert!(warm.rebaseline(&after.descriptor,before));
            assert!(!warm.permits(after));
        }
    }

    /// Native Training CPU Behavior=Control on the live entry (C104 #336..#342):
    /// controller word ffffffff>00000001, operation 01>00, control type 01>24,
    /// rank 100>21; FE+0x36 (AI+0xc10) is carried as written.
    fn control_toggled(mut owner: GroupOwner) -> GroupOwner {
        owner.descriptor[0x10..0x14].copy_from_slice(&1i32.to_le_bytes());
        owner.descriptor[0x34] = 0;
        owner.descriptor[0x35] = 0x24;
        owner.descriptor[0x36] ^= 0x5a;
        owner.descriptor[0x37] = 21;
        owner
    }

    #[test]
    fn group_exchange_permits_only_native_live_behavior_under_the_same_identity() {
        for owner in [0, 1, 4] {
            let (mut before, mut after, members) = exchanged_owner(owner);
            for descriptor in [&mut before.descriptor, &mut after.descriptor] {
                descriptor[0x10..0x14].copy_from_slice(&(-1i32).to_le_bytes());
                descriptor[0x34] = 1; descriptor[0x35] = 1; descriptor[0x37] = 100;
            }
            let receipt = GroupExchange::new(before, after, members).unwrap();
            let toggled = control_toggled(after);
            // C118 D5: the later CPU and Player owner walks and the next swap's
            // receipt check accept the toggled entry; releasing Control is live too.
            assert!(receipt.permits(toggled));
            assert!(receipt.rebaseline(&before.descriptor, toggled));
            assert!(receipt.permits(after));
            assert!(GroupExchange::new(toggled, control_toggled(before), Members::new(&[3,7,8]).unwrap()).is_some());
            // Foreign identity, header or body changes beside the toggle still refuse.
            let mut foreign = [toggled; 6];
            foreign[0].descriptor[0x18] ^= 1;
            foreign[1].descriptor[0x14] ^= 1;
            foreign[2].descriptor[0x41] ^= 1;
            foreign[3].descriptor[0x46] ^= 1;
            foreign[4].header[2] ^= 1;
            foreign[5].bodies[1].0 += 8;
            for current in foreign {
                assert!(!receipt.permits(current));
                assert!(!receipt.rebaseline(&before.descriptor, current));
            }
            // Live bytes outside their native domains refuse as well.
            let mut pad = toggled; pad.descriptor[0x10..0x14].copy_from_slice(&9i32.to_le_bytes());
            let mut control = toggled; control.descriptor[0x35] = 0x28;
            let mut rank = toggled; rank.descriptor[0x37] = 101;
            for current in [pad, control, rank] { assert!(!receipt.permits(current)); }
        }
    }

    #[test]
    fn group_exchange_refuses_foreign_identity_header_and_body_changes() {
        let (before, after, members) = exchanged_owner(0);
        let receipt = GroupExchange::new(before,after,members).unwrap();
        let mut foreign = [after; 7];
        foreign[0].entry += 8;
        foreign[1].owner = 4;
        foreign[2].epoch += 1;
        foreign[3].resource += 8;
        foreign[4].header[1] = 11;
        foreign[5].bodies[0].1 += 1;
        foreign[6].bodies[0].0 += 8;
        for current in foreign {
            assert!(!receipt.permits(current));
            assert!(!receipt.rebaseline(&before.descriptor,current));
        }
        for current in &foreign[..5] {
            assert!(GroupExchange::new(before,*current,members).is_none());
        }
    }

    #[test]
    fn group_exchange_never_adopts_foreign_descriptor_bytes() {
        let (before, after, members) = exchanged_owner(4);
        let receipt = GroupExchange::new(before,after,members).unwrap();
        // Only the entry's natively owned live behavior bytes pass the receipt, and
        // only inside their domains: a flipped low controller byte stays inside
        // -1..=8 while a flipped higher controller byte (0x11..0x14) does not;
        // operation, control type, AI+0xc10 and rank flip to 1. Minting stays
        // byte-exact for every byte.
        let live = [0x10, 0x34, 0x35, 0x36, 0x37];
        for offset in 0..0x70 {
            let mut foreign = after;
            foreign.descriptor[offset] ^= 1;
            assert_eq!(receipt.permits(foreign), live.contains(&offset), "byte {:#x}", offset);
            assert!(GroupExchange::new(before,foreign,members).is_none(), "byte {:#x}", offset);
        }
    }

    #[test]
    fn group_exchange_accepts_two_member_window_only_with_exact_group_words() {
        let (before, mut after, _) = exchanged_owner(1);
        after.descriptor[0x14] = 2;
        after.descriptor[0x20..0x24].copy_from_slice(&u32::MAX.to_le_bytes());
        after.bodies[2] = (0,0);
        let members = Members::new(&[10,11]).unwrap();
        assert!(GroupExchange::new(before,after,members).unwrap().rebaseline(&before.descriptor,after));
    }

    #[test]
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

    #[test]
    fn reservations_share_five_without_stealing_existing_native_bodies() {
        assert_eq!(reserve_extras([2, 2], [0, 0], 5), [2, 2]);
        assert_eq!(reserve_extras([2, 2], [0, 0], 2), [1, 1]);
        assert_eq!(reserve_extras([2, 2], [0, 0], 1), [1, 0]);
        assert_eq!(reserve_extras([0, 2], [0, 0], 2), [0, 2]);
        assert_eq!(reserve_extras([2, 2], [2, 0], 3), [2, 1]);
        // Reducing the setting does not pretend native bodies were freed.
        assert_eq!(reserve_extras([0, 0], [2, 2], 0), [2, 2]);
        for slots in 0..=5 {
            for a in 0..=2 {
                for b in 0..=2 {
                    let got = reserve_extras([2, 2], [a, b], slots);
                    assert!(got[0] >= a && got[1] >= b);
                    assert!(got[0] <= 2 && got[1] <= 2);
                    assert!(got.iter().sum::<usize>() <= slots.max(a + b));
                }
            }
        }
    }

    #[test]
    fn two_and_three_member_banks_preserve_order_and_native_sentinel() {
        let two = Members::new(&[0, 2]).unwrap();
        assert_eq!(two.kinds(), [0, 2, u32::MAX]);
        assert_eq!(two.count(), 2);
        let three = Members::new(&[0x1c, 0x36, 7]).unwrap();
        assert_eq!(three.kinds(), [0x1c, 0x36, 7]);
        assert_eq!(three.count(), 3);
        for bank in [two, three] {
            for i in 0..bank.count() {
                assert_eq!(bank.ready_index(snapshot(&bank.kinds()[..bank.count()], i as u8)), Some(i as u8));
            }
        }
    }

    #[test]
    fn native_capacity_cannot_be_confused_with_five_cache_slots() {
        for kinds in [&[][..], &[0, 2, 3, 7][..], &[0, 2, 3, 7, 8][..]] {
            assert!(Members::new(kinds).is_none());
        }
        for other in [0, u32::MAX, 0x48, 0x49, 0x4a, 0x4b, 0x24, 0x25, 0x26, 0x5a, 0x5b] {
            assert!(Members::new(&[0, other]).is_none(), "{:x}", other);
        }
        // Kirby is an ordinary single-body member (its departure is gated, not its membership),
        // laid out at physical cell 0 whatever its rotation position (CellRule::PinCell0,
        // RE-KIRBY-UNLOAD §0–§3): a layout with Kirby at cell 1 is an entry native never builds.
        assert!(Members::new(&[0, 0x06]).is_none());
        assert_eq!(Members::laid_out(&[0, 0x06]).map(|m| m.kinds()), Some([0x06, 0, u32::MAX]));
        assert_eq!(Members::laid_out(&[0, 0x06]).and_then(|m| m.index_of(0)), Some(1));
    }

    #[test]
    fn solo_popo_is_primary_only_and_pair_requests_cannot_reuse_it() {
        let solo = Forms::solo_popo();
        let bank = Members::from_forms(solo, &[0x4b, 0x4c, 0x02, 0x07], 2).unwrap();
        assert_eq!(bank.kinds(), [0x4b, 0x02, 0x07]);
        assert_eq!(member_forms(bank.kinds()[0]), solo);
        assert_eq!(member_forms(bank.kinds()[1]), Forms::starting(0x02));
        assert!(Members::new(&[0x02, 0x4b]).is_none());
        assert!(Members::new(&[0x02, 0x07, 0x4b]).is_none());
        assert!(Members::from_cycle(0x4b, &[0x02, 0x07], 2).is_none());
        assert!(Members::from_forms(Forms::starting(0x4b), &[0x02, 0x07], 2).is_none());
        assert_eq!(single_member_request(1, [0x4b, u32::MAX, u32::MAX], 1), Some(solo));
        // Same Popo kind, but normal Nana partner, malformed descriptor padding,
        // or incorrect actor count cannot pass the native request boundary.
        for (count, kinds, actors) in [
            (2, [0x4b, 0x4c, u32::MAX], 2),
            (1, [0x4b, 0x4c, u32::MAX], 1),
            (1, [0x4b, u32::MAX, u32::MAX], 2),
            (1, [0x4c, u32::MAX, u32::MAX], 1),
        ] {
            assert!(single_member_request(count, kinds, actors).is_none());
        }
        for active in 0..3 {
            let state = snapshot(&[0x4b, 0x02, 0x07], active);
            assert!(state.ready().is_none());
            assert_eq!(bank.ready_live(state), Some((active, [0x4b, 0x02, 0x07][active as usize])));
        }
        assert!(Members::from_forms(Forms { flags: 2, ..solo }, &[0x02], 1).is_none());
    }

    #[test]
    fn pair_lead_bank_fixes_the_pair_in_cells_zero_and_one_and_selects_only_pair_or_extra() {
        const MARIO: u32 = 0x00;
        const LINK: u32 = 0x02;
        const KIRBY: u32 = 0x06;
        let pair = Forms::starting(POPO);
        assert!(pair_request(pair.count, pair.kinds, pair.actor_count));
        for (count, kinds, actors) in [
            (1, [POPO, u32::MAX, u32::MAX], 1),
            (2, [POPO, NANA, u32::MAX], 1),
            (3, [POPO, NANA, MARIO], 2),
            (1, [MARIO, u32::MAX, u32::MAX], 1),
        ] {
            assert!(!pair_request(count, kinds, actors));
        }
        let bank = Members::pair_lead(MARIO).unwrap();
        assert!(bank.is_pair_lead());
        assert_eq!(bank.kinds(), [POPO, NANA, MARIO]);
        assert_eq!((bank.count(), bank.actor_count()), (3, 2));
        assert_eq!(bank.index_of(POPO), Some(0));
        assert_eq!(bank.index_of(NANA), None, "Nana is never a selectable member");
        assert_eq!(bank.index_of(MARIO), Some(2));
        assert!(bank.selectable(0) && !bank.selectable(1) && bank.selectable(2) && !bank.selectable(3));
        assert_eq!((bank.expected_flags(0), bank.expected_flags(2)), (PAIR_ACTIVE_FLAG, BANK_FLAG));
        assert_eq!([0, 1, 2].map(|i| bank.body_state(i, Some(0))), [4, 4, 3]);
        assert_eq!([0, 1, 2].map(|i| bank.body_state(i, Some(2))), [3, 3, 4]);
        assert_eq!([0, 1, 2].map(|i| bank.body_state(i, None)), [3, 3, 3]);
        assert_eq!(bank.forms_of(POPO), pair);
        assert_eq!(bank.forms_of(MARIO), Forms::starting(MARIO));
        assert_eq!(bank.logical_kinds(), ([POPO, MARIO, u32::MAX], 2));
        // Adapters hold only the physical descriptor words.
        assert_eq!(member_forms_in(3, [POPO, NANA, MARIO], 2, POPO), pair);
        assert_eq!(member_forms_in(3, [POPO, NANA, MARIO], 2, MARIO), Forms::starting(MARIO));
        assert_eq!(member_forms_in(3, [POPO, LINK, MARIO], 1, POPO), Forms::solo_popo());
        assert_eq!(member_forms_in(2, [POPO, NANA, u32::MAX], 2, POPO), Forms::solo_popo(), "not a bank window");
        assert_eq!(member_forms_in(3, [POPO, NANA, MARIO], 1, POPO), Forms::solo_popo());
        // Resources: native 0x17e5bc0 already requests Nana for a Popo primary.
        assert_eq!(supplemental_acquisitions(bank), ([MARIO, u32::MAX, u32::MAX], 1));
        assert!(!window_swappable(bank), "a pair group is never detached");
        // Construction: pair target only, first eligible extra, two extra bodies.
        for extra in [POPO, NANA, 0x48, 0x24, 0x5a] {
            assert!(Members::pair_lead(extra).is_none(), "{:x}", extra);
        }
        assert_eq!(Members::from_pair(&[NANA, 0x48, LINK, MARIO], 2).unwrap().kinds(), [POPO, NANA, LINK]);
        assert!(Members::from_pair(&[LINK], 1).is_none());
        // Kirby pins cell 0, which the pair holds: it is skipped as the extra and waits for the
        // next window (CellRule::PinCell0).
        assert_eq!(Members::from_pair(&[NANA, KIRBY, LINK], 2).unwrap().kinds(), [POPO, NANA, LINK]);
        assert!(Members::pair_lead(KIRBY).is_none());
        assert!(Members::new(&[POPO, NANA, MARIO]).is_none());
        let ordinary = Members::new(&[MARIO, LINK]).unwrap();
        assert!(!ordinary.is_pair_lead());
        assert_eq!(ordinary.actor_count(), 1);
        assert_eq!(ordinary.logical_kinds(), ([MARIO, LINK, u32::MAX], 2));
        assert_eq!([0, 1].map(|i| ordinary.body_state(i, Some(1))), [3, 4]);
        assert_eq!(ordinary.forms_of(MARIO), Forms::starting(MARIO));
        assert_eq!(Members::from_forms(Forms::solo_popo(), &[LINK], 1).unwrap().forms_of(POPO), Forms::solo_popo());
    }

    #[test]
    fn pair_lead_readiness_needs_pair_flags_while_active_and_bank_flags_while_parked() {
        const MARIO: u32 = 0x00;
        let bank = Members::pair_lead(MARIO).unwrap();
        let state = |index: u8, flags: u8| EntryState { count: 3, kinds: [POPO, NANA, MARIO], actor_count: 2,
            installed: 3, index, previous_or_next: 0xff, pending: 0xff, flags, outer: 6, loader: 14,
            changing: 0, guard: 0 };
        assert_eq!(bank.ready_index(state(0, PAIR_ACTIVE_FLAG)), Some(0));
        assert_eq!(bank.ready_live(state(0, PAIR_ACTIVE_FLAG)), Some((0, POPO)));
        assert_eq!(bank.ready_index(state(0, BANK_FLAG)), None, "pair active needs bit 1");
        assert_eq!(bank.ready_index(state(2, BANK_FLAG)), Some(2));
        assert_eq!(bank.ready_index(state(2, PAIR_ACTIVE_FLAG)), None, "extra active must clear bit 1");
        assert_eq!(bank.ready_index(state(1, PAIR_ACTIVE_FLAG)), None);
        assert_eq!(bank.ready_index(state(1, BANK_FLAG)), None);
        assert_eq!(bank.ready_index(EntryState { actor_count: 1, ..state(0, PAIR_ACTIVE_FLAG) }), None);
        assert!(state(0, PAIR_ACTIVE_FLAG).ready().is_none(), "never mistaken for a canonical pair");
        let waiting = |index: u8, flags: u8| EntryState { previous_or_next: index, outer: 3, loader: 12,
            changing: 1, ..state(index, flags) };
        assert!(bank.awaiting_completion(waiting(2, BANK_FLAG), 2), "pair parked by N3 inside the commit");
        assert!(!bank.awaiting_completion(waiting(2, PAIR_ACTIVE_FLAG), 2), "N3 did not run");
        assert!(bank.awaiting_completion(waiting(0, BANK_FLAG), 0), "extra parked, pair selected");
        assert!(!bank.awaiting_completion(waiting(1, BANK_FLAG), 1));
        assert!(!bank.awaiting_completion(waiting(2, BANK_FLAG), 0));
        // Ordinary banks are unchanged by the pair rules.
        let ordinary = Members::new(&[MARIO, 0x02, 0x07]).unwrap();
        for active in 0..3u8 {
            assert_eq!(ordinary.ready_index(snapshot(&[MARIO, 0x02, 0x07], active)), Some(active));
            assert_eq!(ordinary.expected_flags(active), BANK_FLAG);
        }
        assert_eq!(ordinary.ready_index(EntryState { flags: PAIR_ACTIVE_FLAG, ..snapshot(&[MARIO, 0x02, 0x07], 0) }), None);
    }

    #[test]
    fn ordinary_and_canonical_group_readiness_are_not_relaxed() {
        let bank = Members::new(&[0, 2, 3]).unwrap();
        let state = snapshot(&[0, 2, 3], 0);
        assert!(state.ready().is_none());
        assert_eq!(bank.ready_index(state), Some(0));
        for bad in [
            EntryState { installed: 2, ..state }, EntryState { flags: 0x64, ..state },
            EntryState { actor_count: 3, ..state }, EntryState { index: 3, ..state },
            EntryState { guard: 1, ..state }, EntryState { pending: 1, ..state },
            EntryState { kinds: [0, 3, 2], ..state }, EntryState { changing: 1, ..state },
        ] {
            assert_eq!(bank.ready_index(bad), None);
        }
    }

    #[test]
    fn completion_requires_the_exact_committed_transition() {
        let bank = Members::new(&[0, 2]).unwrap();
        let ready = snapshot(&[0, 2], 0);
        assert!(!bank.awaiting_completion(ready, 1));
        let waiting = EntryState { index: 1, previous_or_next: 1, outer: 3,
            loader: 12, changing: 1, ..ready };
        assert!(bank.awaiting_completion(waiting, 1));
        assert!(!bank.awaiting_completion(waiting, 0));
        assert!(!bank.awaiting_completion(waiting, 2));
        for bad in [EntryState { guard: 1, ..waiting }, EntryState { outer: 6, ..waiting },
            EntryState { loader: 0, ..waiting }, EntryState { loader: 13, ..waiting },
            EntryState { previous_or_next: 0, ..waiting }, EntryState { pending: 1, ..waiting }] {
            assert!(!bank.awaiting_completion(bad, 1));
        }
    }

    /// MII-PHASEB: the Mii setup's two seams are this module's own predicates. Every state
    /// `ready_index` admits is the plan seam's Ready phase, and every state
    /// `awaiting_completion` admits (with `+0x5933 == 0`, which `service_locked` requires
    /// before `Action::Swap`) is the Phase B seam's parked change; neither admits the other.
    #[test]
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

    #[test]
    fn cycle_keeps_target_first_after_skipping_unqualified_kinds() {
        const MARIO: u32 = 0;
        const LINK: u32 = 0x02;
        const SAMUS: u32 = 0x03;
        const KIRBY: u32 = 0x06;
        const FOX: u32 = 0x07;
        const PIKACHU: u32 = 0x08;
        // Target repeats, then three Miis, Nana, the PT trio and both Aegis forms,
        // then the ordinary kinds the cap must cut.
        let upcoming = [
            MARIO, MARIO, 0x48, 0x49, 0x4a, 0x4c, 0x24, 0x25, 0x26, 0x5a, 0x5b,
            PIKACHU, PIKACHU, LINK, FOX, SAMUS,
        ];
        // A lane `0x4b` the adapter keeps (`member_lane`: the role's numbered form is Solo) is a
        // qualified member since row 4, laid out at cell 0 with the target at its own cell.
        let mut with_popo = upcoming.to_vec();
        with_popo.insert(5, 0x4b);
        assert_eq!(Members::from_cycle(MARIO, &with_popo, 2).unwrap().kinds(), [0x4b, MARIO, PIKACHU]);
        // Kirby is kept like any ordinary upcoming kind and laid out at cell 0 (the target keeps
        // its own cell: Mario at 1); as the target it is cell 0 as before.
        let pinned = Members::from_cycle(MARIO, &[KIRBY, LINK], 2).unwrap();
        assert_eq!(pinned.kinds(), [KIRBY, MARIO, LINK]);
        assert_eq!((pinned.index_of(MARIO), pinned.index_of(KIRBY), pinned.index_of(LINK)), (Some(1), Some(0), Some(2)));
        assert_eq!(Members::from_cycle(KIRBY, &[LINK, FOX], 2).unwrap().kinds(), [KIRBY, LINK, FOX]);
        let full = Members::from_cycle(MARIO, &upcoming, 2).unwrap();
        assert_eq!(full.kinds(), [MARIO, PIKACHU, LINK]);
        assert_eq!(full.count(), 3);
        assert_eq!(Members::from_cycle(MARIO, &upcoming, 5).unwrap().kinds(), full.kinds());
        assert_eq!(Members::from_cycle(MARIO, &upcoming, 100).unwrap().kinds(), full.kinds());
        assert_eq!(
            Members::from_cycle(MARIO, &upcoming, 1).unwrap().kinds(),
            [MARIO, PIKACHU, u32::MAX]
        );
        assert!(Members::from_cycle(MARIO, &upcoming, 0).is_none());
        assert!(Members::from_cycle(MARIO, &[], 5).is_none());
        assert_eq!(full.index_of(MARIO), Some(0));
        assert_eq!(full.index_of(PIKACHU), Some(1));
        assert_eq!(full.index_of(LINK), Some(2));
        assert_eq!(full.index_of(FOX), None);
        assert_eq!(full.index_of(KIRBY), None);
        assert_eq!(full.index_of(u32::MAX), None);
        let pair = Members::from_cycle(MARIO, &upcoming, 1).unwrap();
        assert_eq!(pair.index_of(LINK), None);
        assert_eq!(pair.index_of(u32::MAX), None);
    }

    #[test]
    fn ineligible_target_or_no_second_member_is_absent() {
        let extras = [0x02u32, 0x03, 0x07];
        for target in [0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x24, 0x25, 0x26, 0x5a, 0x5b, u32::MAX] {
            assert!(Members::from_cycle(target, &extras, 2).is_none(), "{:x}", target);
        }
        assert!(Members::from_cycle(0, &[0x00, 0x48, 0x4c, 0x24, 0x5a], 2).is_none());
        // A lane `0x4b` is Solo Popo once the adapter keeps it (row 4), pinned to cell 0; when the
        // role's numbered form is the pair it never reaches the planner.
        assert_eq!(Members::from_cycle(0, &[0x00, 0x48, 0x4b, 0x24, 0x5a], 2).unwrap().kinds(), [0x4b, 0x00, u32::MAX]);
        let (pair_lane, n) = member_lane(&[0x00, 0x48, 0x4b, 0x24, 0x5a], false);
        assert!(Members::from_cycle(0, &pair_lane[..n], 2).is_none());
        assert_eq!(Members::from_cycle(0x06, &extras, 2).unwrap().kinds(), [0x06, 0x02, 0x03]);
        assert_eq!(Members::new(&[0x06, 0x02]).map(|m| m.count()), Some(2));
    }

    #[test]
    fn kirby_parks_only_copy_free_and_otherwise_keeps_the_reload() {
        const KIRBY: u32 = 0x06;
        // Membership: Kirby is an ordinary window member and group member.
        assert!(swappable_kind(KIRBY));
        assert!(window_swappable(members(&[KIRBY, LINK, FOX])));
        let laid = Members::laid_out(&[LINK, KIRBY]).unwrap();
        assert_eq!((laid.kinds(), laid.index_of(KIRBY), laid.index_of(LINK)), ([KIRBY, LINK, u32::MAX], Some(0), Some(1)));
        assert_eq!(member_forms(KIRBY), Forms::starting(KIRBY));
        assert_eq!(single_member_request(1, [KIRBY, u32::MAX, u32::MAX], 1), Some(Forms::starting(KIRBY)));
        let plan = plan_extra_groups([Some(members(&[MARIO, LINK, SAMUS])), None], [&[], &[]],
            [&[KIRBY, FOX], &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![KIRBY, FOX]]);
        // Departure gate: only a copy-free Kirby parks; every copy state keeps today's Reload.
        assert_eq!(kirby_parkable(false, Some(0), false), Ok(()));
        assert_eq!(kirby_parkable(true, Some(0), false), Err("Kirby holds a copy ability"));
        assert_eq!(kirby_parkable(true, Some(1 << 0x07), true), Err("Kirby holds a copy ability"));
        assert_eq!(kirby_parkable(false, None, false), Err("Kirby copy resources are not readable"));
        assert_eq!(kirby_parkable(false, Some(1 << 0x07), false), Err("Kirby still retains copy resources"));
        assert_eq!(kirby_parkable(false, Some(1), false), Err("Kirby still retains copy resources"));
        assert_eq!(kirby_parkable(false, Some(0), true), Err("a hat lease names this Kirby"));
        // Natural inhale copy (flag set, no modpack lease) refuses as well.
        assert_eq!(kirby_parkable(true, Some(1 << 0x2d), false), Err("Kirby holds a copy ability"));
    }

    #[test]
    fn solo_popo_leads_a_detached_group_only_under_its_policy() {
        let solo_policy = GroupPolicy { solo_popo_groups: true, ..GroupPolicy::DEFAULT };
        assert_eq!(GroupPolicy::DEFAULT, GroupPolicy::default());
        assert!(!group_member_kind(POPO, GroupPolicy::DEFAULT) && group_member_kind(POPO, solo_policy));
        assert!(!group_member_kind(NANA, solo_policy) && group_member_kind(FOX, GroupPolicy::DEFAULT));
        let window = members(&[MARIO, LINK, SAMUS]);
        // Default policy: Popo is skipped, exactly as before.
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [&[FOX, POPO, PIKACHU], &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        // Policy on: Popo is a member and leads its group; the others keep lane order.
        let plan = plan_extra_groups_with(solo_policy, [Some(window), None], [&[], &[]], [&[FOX, POPO, PIKACHU], &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![POPO, FOX, PIKACHU]]);
        let group = plan[0][0].unwrap();
        assert_eq!((group.index_of(POPO), group.forms_of(POPO), group.actor_count()), (Some(0), Forms::solo_popo(), 1));
        // The Reload-time supplemental rule (native requests member 1 for a Popo
        // primary) is unchanged for this shape; a detached group is prefetched
        // member by member through `0x17e64d0` instead, so it is not consulted there.
        assert_eq!(supplemental_acquisitions(group), ([PIKACHU, u32::MAX, u32::MAX], 1));
        assert!(swap_commit_index_in(group, 0) == Some(1) && group.selectable(0) && group.selectable(2) && !group.selectable(3));
        let plan = plan_extra_groups_with(solo_policy, [Some(window), None], [&[], &[]], [&[FOX, PIKACHU, NESS, POPO], &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU], vec![POPO, NESS]]);
        let plan = plan_extra_groups_with(solo_policy, [Some(window), None], [&[], &[]], [&[FOX, POPO], &[]], [None, None], budget(7));
        assert_eq!(kinds_of(&plan[0]), vec![vec![POPO, FOX]]);
        // Nana is never a member; a Solo window's own Popo is not re-planned; the
        // pair-lead window stays excluded under this policy alone.
        let plan = plan_extra_groups_with(solo_policy, [Some(window), None], [&[], &[]], [&[NANA, FOX], &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX]]);
        let solo = Members::from_forms(Forms::solo_popo(), &[LINK, FOX], 2).unwrap();
        let plan = plan_extra_groups_with(solo_policy, [Some(solo), None], [&[], &[]], [&[POPO, PIKACHU], &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![PIKACHU]]);
        assert!(!window_swappable_with(Members::pair_lead(LINK).unwrap(), solo_policy));
    }

    #[test]
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

    #[test]
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

    #[test]
    fn a_pinned_home_leaving_for_a_group_reports_its_live_member_not_cell_0() {
        // Eden M4b `m4b3`: home `[6, 5, 7]` with Fox (index 2) live, a Phase B request logged
        // `outgoing=0x6` (cell 0). The outgoing member is the bank's ready live member
        // (`ready_live`, what the adapters' `bank_live`/`live_kind` report), read before the request.
        const KIRBY: u32 = 0x06;
        for (home, pinned) in [
            (Members::from_cycle(YOSHI, &[KIRBY, FOX], 2).unwrap(), KIRBY),
            (Members::from_cycle(YOSHI, &[POPO, FOX], 2).unwrap(), POPO),
        ] {
            let cells = [pinned, YOSHI, FOX];
            assert_eq!(home.kinds(), cells);
            let state = snapshot(&cells, 2);
            assert_eq!(home.ready_live(state), Some((2, FOX)), "outgoing is Fox, not {:#x}", pinned);
            // The group request commits through the next cell (the pinned member's cell 0) and is
            // never activated there: the seam exchanges the window first.
            assert_eq!(swap_commit_index_in(home, 2), Some(0));
            let seam = EntryState { index: 0, previous_or_next: 0, outer: 3, loader: 12, changing: 1, ..state };
            assert!(home.awaiting_completion(seam, 0));
            // The group's own Reload-free entry names its member, whatever the home's cell 0.
            let group = Members::new(&[0x1b, 0x0a]).unwrap();
            assert_eq!(crate::engine::swap::swap_model::group_serves(group, 0x1b, Forms::starting(0x1b)), Some(0));
        }
    }

    #[test]
    fn a_lane_popo_reaches_the_window_planner_only_as_the_roles_solo_popo() {
        // The adapter proves from the menu that the role's numbered `0x4b` is Solo and not the
        // canonical-only original (`solo_popo_member`); otherwise the entry never reaches it.
        let lane = [FOX, POPO, NANA, LINK, u32::MAX];
        let (kept, n) = member_lane(&lane, true);
        assert_eq!(&kept[..n], &lane[..]);
        let (kept, n) = member_lane(&lane, false);
        assert_eq!(&kept[..n], &[FOX, NANA, LINK, u32::MAX][..]);
        let (kept, n) = member_lane(&[POPO, FOX, LINK], true);
        assert_eq!(Members::from_cycle(YOSHI, &kept[..n], 2).unwrap().kinds(), [POPO, YOSHI, FOX]);
        let (kept, n) = member_lane(&[POPO, FOX, LINK], false);
        assert_eq!(Members::from_cycle(YOSHI, &kept[..n], 2).unwrap().kinds(), [YOSHI, FOX, LINK]);
        // Bounded by the lane capacity, and empty in, empty out.
        assert_eq!(member_lane(&[FOX; MAX_TARGETS + 3], true).1, MAX_TARGETS);
        assert_eq!(member_lane(&[], true).1, 0);
    }

    #[test]
    fn a_pair_lead_window_owns_groups_only_under_its_policy_and_commits_to_selectable_cells() {
        let pair_policy = GroupPolicy { ic_pair_groups: true, ..GroupPolicy::DEFAULT };
        let pair = Members::pair_lead(MARIO).unwrap();
        assert!(!window_swappable(pair) && window_swappable_with(pair, pair_policy));
        assert!(window_swappable_with(Members::pair_lead(0x2d).unwrap(), pair_policy), "R.O.B. extra");
        assert!(window_swappable_with(members(&[MARIO, LINK]), pair_policy), "ordinary windows unchanged");
        // The commit index: pair (0) <-> extra (2); Nana's cell is never selected.
        assert_eq!(swap_commit_index_in(pair, 0), Some(2));
        assert_eq!(swap_commit_index_in(pair, 2), Some(0));
        assert_eq!(swap_commit_index_in(pair, 1), None);
        assert_eq!(swap_commit_index_in(pair, 3), None);
        assert!(pair.selectable(swap_commit_index_in(pair, 0).unwrap()) && pair.selectable(swap_commit_index_in(pair, 2).unwrap()));
        for (window, from, via) in [(members(&[FOX, PIKACHU, NESS]), 1, Some(2)), (members(&[FOX, PIKACHU, NESS]), 2, Some(0)),
            (members(&[FOX, PIKACHU]), 1, Some(0))] {
            assert_eq!(swap_commit_index_in(window, from), via);
            assert_eq!(swap_commit_index_in(window, from), swap_commit_index(from, window.count()));
        }
        // A one-member window commits to its own cell (`ExitRule::SelfCell`), not the next-cell rule.
        assert_eq!((swap_commit_index_in(members(&[FOX]), 0), swap_commit_index(0, 1)), (Some(0), None));
        // Planner: the pair-lead window owns ordinary groups only with the policy;
        // Popo and Nana are never members under it.
        let lane: &[u32] = &[LINK, FOX, POPO, NANA, PIKACHU, MARIO];
        let plan = plan_extra_groups([Some(pair), None], [&[], &[]], [lane, &[]], [None, None], budget(8));
        assert!(plan.iter().flatten().all(Option::is_none));
        let plan = plan_extra_groups_with(pair_policy, [Some(pair), None], [&[], &[]], [lane, &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![LINK, FOX, PIKACHU]]);
        assert!(plan[0][0].unwrap().kinds().iter().all(|k| *k != POPO && *k != NANA));
        // The pair window's three bodies are charged to the budget.
        let plan = plan_extra_groups_with(pair_policy, [Some(pair), None], [&[], &[]], [lane, &[]], [None, None], budget(5));
        assert_eq!(kinds_of(&plan[0]), vec![vec![LINK]]);
        let plan = plan_extra_groups_with(pair_policy, [Some(pair), None], [&[], &[]], [lane, &[]], [None, None], budget(4));
        assert!(plan.iter().flatten().all(Option::is_none));
    }

    #[test]
    fn the_original_joins_its_bank_only_as_an_ordinary_single_body_kind() {
        for kind in [MARIO, LINK, 0x06, 0x2d, 0x2c, 0x59] {
            assert!(original_bankable(kind), "{:x}", kind);
        }
        for kind in [0x4b, 0x4c, 0x48, 0x49, 0x4a, 0x24, 0x25, 0x26, 0x5a, 0x5b, 0x4e, 0x75, u32::MAX] {
            assert!(!original_bankable(kind), "{:x}", kind);
        }
        // With the original admitted (no `exclude`), the lane's return to it is
        // planned like any neighbour: the user's roster becomes a full window plus
        // [R.O.B., Lucario, Falcon] instead of a canonical Reload every lap.
        const DARK_PIT: u32 = 0x1f;
        const LUCARIO: u32 = 0x2c;
        const ROB: u32 = 0x2d;
        const TOON_LINK: u32 = 0x2e;
        let window = members(&[DARK_PIT, TOON_LINK, ROB]);
        let lane: &[u32] = &[TOON_LINK, ROB, LUCARIO, FALCON, DARK_PIT];
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![LUCARIO, FALCON]]);
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [Some(FALCON), None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![LUCARIO]], "the excluded original stays canonical");
    }

    #[test]
    fn ready_live_returns_the_observed_member_without_relaxing_canonical_ready() {
        let bank = Members::from_cycle(0, &[0x08, 0x02, 0x07], 2).unwrap();
        let active = snapshot(&[0, 0x08, 0x02], 2);
        assert!(active.ready().is_none());
        assert_eq!(bank.ready_live(active), Some((2, 0x02)));
        let steady = EntryState { previous_or_next: 2, ..active };
        assert_eq!(bank.ready_live(steady), Some((2, 0x02)));
        assert_eq!(bank.ready_live(snapshot(&[0, 0x02, 0x08], 2)), None);
        let pt = EntryState {
            count: 3, kinds: [0x24, 0x25, 0x26], actor_count: 1, installed: 3, index: 2,
            previous_or_next: 0xff, pending: 0xff, flags: 0x05, outer: 6, loader: 14,
            changing: 0, guard: 0,
        };
        assert_eq!(pt.ready().map(|(_, kind)| kind), Some(0x26));
        assert_eq!(bank.ready_live(pt), None);
        let aegis = EntryState {
            count: 2, kinds: [0x5b, 0x5a, u32::MAX], actor_count: 1, installed: 2, index: 1,
            previous_or_next: 1, pending: 0xff, flags: 0x64, outer: 6, loader: 14,
            changing: 0, guard: 0,
        };
        assert_eq!(aegis.ready().map(|(_, kind)| kind), Some(0x5a));
        assert_eq!(bank.ready_live(aegis), None);
        let climbers = EntryState {
            count: 2, kinds: [0x4b, 0x4c, u32::MAX], actor_count: 2, installed: 2, index: 0,
            previous_or_next: 0xff, pending: 0xff, flags: 0x02, outer: 6, loader: 14,
            changing: 0, guard: 0,
        };
        assert!(climbers.ready().is_some());
        assert_eq!(bank.ready_live(climbers), None);
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

    #[test]
    fn excluded_kinds_never_join_a_swap_group_and_solo_first_windows_own_groups() {
        for kind in [0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4e, 0x4f, 0x50, 0x24, 0x25, 0x26,
            0x5a, 0x5b, 0x6e, 0x70, 0x75, u32::MAX] {
            assert!(!swappable_kind(kind), "{:x}", kind);
        }
        // R.O.B. (0x2d) is an ordinary single-body kind for the group swap: the
        // native costume selector only reads R+0x58 (EXCLUSIONS-AUDIT §1). Kirby
        // (0x06) is too; its departure gate is `kirby_parkable`.
        for kind in [MARIO, LINK, SAMUS, 0x06, FOX, PIKACHU, 0x1c, 0x2c, 0x2d, 0x2e, 0x59] {
            assert!(swappable_kind(kind), "{:x}", kind);
        }
        assert!(window_swappable(members(&[MARIO, LINK, SAMUS])));
        assert!(window_swappable(members(&[0x2d, LINK])), "a R.O.B.-primary window owns groups");
        assert!(window_swappable(members(&[LINK, 0x2d, FOX])), "R.O.B. as a window member");
        // A Solo-Popo-first window owns ordinary groups (EXCLUSIONS row 2); Popo
        // itself is never placed in one, and a pair-lead window still never is.
        let solo = Members::from_forms(Forms::solo_popo(), &[LINK, FOX], 2).unwrap();
        assert!(window_swappable(solo) && swap_commit_index_in(solo, 0) == Some(1));
        assert!(window_swappable(Members::from_forms(Forms::solo_popo(), &[LINK], 1).unwrap()));
        assert!(!window_swappable(Members::pair_lead(LINK).unwrap()));
        let plan = plan_extra_groups([Some(solo), Some(members(&[LUIGI, DK, YOSHI]))], [&[], &[]],
            [&[PIKACHU, 0x4b, NESS, FALCON], &[PEACH, SAMUS, MARIO]], [None, None], budget(12));
        assert_eq!(kinds_of(&plan[0]), vec![vec![PIKACHU, NESS, FALCON]]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH, SAMUS, MARIO]]);
        // The Solo window's own Popo is a window member, never a group candidate.
        let plan = plan_extra_groups([Some(solo), None], [&[], &[]], [&[0x4b, 0x4c, PIKACHU], &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![PIKACHU]]);
        // R.O.B. is placed like any neighbour; the role's canonical original is
        // skipped, not placed.
        let plan = plan_extra_groups([Some(members(&[MARIO, LINK, SAMUS])), None], [&[], &[]],
            [&[0x2d, FOX, NESS, PIKACHU], &[]], [Some(NESS), None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![0x2d, FOX, PIKACHU]]);
        assert!(plan[1].iter().all(Option::is_none));
        // R.O.B. may also be a detached group's member in any cell.
        assert_eq!(Members::new(&[FOX, 0x2d]).map(|m| m.index_of(0x2d)), Some(Some(1)));
    }

    #[test]
    fn a_live_kirby_keeps_no_prefetch_key_on_its_resource_owner() {
        // D-KIRBY1: Planned groups wait and Prefetched groups return their keys
        // while the live member is Kirby; any other live member (or none) admits.
        assert_eq!(prefetch_admitted(Some(KIRBY)), Err("live Kirby may copy a group kind"));
        for live in [None, Some(MARIO), Some(FOX), Some(POPO), Some(0x2c)] {
            assert_eq!(prefetch_admitted(live), Ok(()), "{:?}", live);
        }
        // A key is released unless the live Kirby copies exactly that kind: the
        // Ally S2 stall released kind 0 (Mario) while Kirby wore Mario's hat.
        assert_eq!(key_release_admitted(MARIO, Some(MARIO)), Err("live Kirby copies this kind"));
        assert_eq!(key_release_admitted(MARIO, Some(FOX)), Ok(()));
        assert_eq!(key_release_admitted(MARIO, None), Ok(()));
        assert_eq!(key_release_admitted(FOX, Some(MARIO)), Ok(()));
    }

    #[test]
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

    #[test]
    fn budget_charges_existing_groups_hats_and_the_fighter_pool() {
        let windows = [Some(members(&[MARIO, LINK, SAMUS])), Some(members(&[LUIGI, DK, YOSHI]))];
        let upcoming: [&[u32]; 2] = [&[FOX, PIKACHU, NESS, FALCON, PEACH], &[PEACH, FALCON, NESS, PIKACHU, FOX]];
        let existing_p1 = [members(&[FOX, PIKACHU])];
        // Eight with P1's pair already built: nothing more, and the pair is not re-planned.
        let plan = plan_extra_groups(windows, [&existing_p1, &[]], upcoming, [None, None], budget(8));
        assert!(plan.iter().flatten().all(Option::is_none), "{:?}", plan);
        // Ten with P1's pair built: CPU's nearer neighbours outrank P1's deeper ones.
        let plan = plan_extra_groups(windows, [&existing_p1, &[]], upcoming, [None, None], budget(10));
        assert!(plan[0].iter().all(Option::is_none), "{:?}", plan[0]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH, FALCON]]);
        // The fighter pool caps bodies regardless of the setting.
        let plan = plan_extra_groups(windows, [&[], &[]], upcoming, [None, None],
            GroupBudget { ready_fighters: 12, pool_free: 3, special: SpecialPolicy::NONE });
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU, NESS]]);
        assert!(plan[1].iter().all(Option::is_none));
        // One free pool body buys the nearest neighbour as a one-member group.
        let plan = plan_extra_groups(windows, [&[], &[]], upcoming, [None, None],
            GroupBudget { ready_fighters: 12, pool_free: 1, special: SpecialPolicy::NONE });
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX]]);
        assert!(plan[1].iter().all(Option::is_none));
        let plan = plan_extra_groups(windows, [&[], &[]], upcoming, [None, None],
            GroupBudget { ready_fighters: 12, pool_free: 0, special: SpecialPolicy::NONE });
        assert!(plan.iter().flatten().all(Option::is_none));
        // A role without a window bank gets nothing even with budget to spare; the
        // other role may then hold several groups, the last one a pair.
        let plan = plan_extra_groups([None, windows[1]], [&[], &[]], upcoming, [None, None], budget(12));
        assert!(plan[0].iter().all(Option::is_none));
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH, FALCON, NESS], vec![PIKACHU, FOX]]);
        // Window members and duplicates in the lookahead are skipped; at most MAX_EXTRA_GROUPS.
        let long: [u32; 12] = [MARIO, LINK, FOX, FOX, PIKACHU, NESS, FALCON, PEACH, 0x1c, 0x2c, 0x2e, 0x59];
        let plan = plan_extra_groups([windows[0], None], [&[], &[]], [&long, &[]], [None, None],
            GroupBudget { ready_fighters: 40, pool_free: 40, special: SpecialPolicy::NONE });
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU, NESS], vec![FALCON, PEACH, 0x1c], vec![0x2c, 0x2e, 0x59]]);
        let full: [Members; 3] = [members(&[FOX, PIKACHU]), members(&[NESS, FALCON]), members(&[PEACH, 0x1c])];
        let plan = plan_extra_groups([windows[0], None], [&full, &[]], [&long, &[]], [None, None], budget(40));
        assert!(plan.iter().flatten().all(Option::is_none));
    }

    #[test]
    fn solo_first_banks_request_member_one_only_once() {
        // SOLO-STALL-RCA: native 0x17e5bc0 already requests member 1 for a Popo
        // primary; the hook may add member 2 only. Ordinary primaries own 1 and 2.
        let solo3 = Members::from_forms(Forms::solo_popo(), &[SAMUS, FOX], 2).unwrap();
        assert_eq!(solo3.kinds(), [0x4b, SAMUS, FOX]);
        assert_eq!(supplemental_acquisitions(solo3), ([FOX, u32::MAX, u32::MAX], 1));
        let solo2 = Members::from_forms(Forms::solo_popo(), &[SAMUS], 1).unwrap();
        assert_eq!(supplemental_acquisitions(solo2), ([u32::MAX; 3], 0));
        assert_eq!(supplemental_acquisitions(members(&[LINK, SAMUS, FOX])), ([SAMUS, FOX, u32::MAX], 2));
        assert_eq!(supplemental_acquisitions(members(&[LINK, SAMUS])), ([SAMUS, u32::MAX, u32::MAX], 1));
        // Every window member is accounted for exactly once across native and hook.
        for bank in [solo3, solo2, members(&[LINK, SAMUS, FOX]), members(&[LINK, SAMUS])] {
            let (kinds, count) = supplemental_acquisitions(bank);
            let native = if bank.kinds()[0] == 0x4b { 2.min(bank.count()) } else { 1 };
            assert_eq!(native + count, bank.count(), "{:?}", bank);
            for kind in &kinds[..count] {
                assert!(bank.kinds()[native..bank.count()].contains(kind));
                assert!(swappable_kind(*kind));
            }
            assert!(kinds[count..].iter().all(|k| *k == u32::MAX));
        }
    }

    #[test]
    fn pair_lead_banks_request_only_the_extra_and_carry_one_resource_reference_per_body() {
        // Native 0x17e5bc0 requests Popo and, through its kind table, Nana; the hook
        // adds the extra kind once. The construction tail then applies the pair rule
        // to all three bodies, so R+0x50 must count three, not the vanilla two.
        let pair = Members::pair_lead(LINK).unwrap();
        assert_eq!(pair.kinds(), [0x4b, 0x4c, LINK]);
        assert_eq!(supplemental_acquisitions(pair), ([LINK, u32::MAX, u32::MAX], 1));
        assert_eq!(pair_resource_references(pair), Some(3));
        assert_eq!(pair_resource_references(pair), Some(pair.count() as u32));
        // Ordinary and Solo-first windows release per kind and keep the native value.
        let solo3 = Members::from_forms(Forms::solo_popo(), &[SAMUS, FOX], 2).unwrap();
        assert_eq!(pair_resource_references(solo3), None);
        assert_eq!(pair_resource_references(members(&[LINK, SAMUS, FOX])), None);
        assert_eq!(pair_resource_references(members(&[LINK, SAMUS])), None);
    }

    #[test]
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

    #[test]
    fn native_groups_keep_their_shape_flags_and_count() {
        for kind in [0x24u32, 0x25, 0x26] {
            let pt = Members::native_group(kind).unwrap();
            assert_eq!(pt.kinds(), [0x24, 0x25, 0x26]);
            assert_eq!((pt.count(), pt.actor_count(), pt.is_pair(), pt.is_pair_lead()), (3, 1, false, false));
            assert_eq!(pt.special(), Special::Native(0x05));
            assert_eq!([0, 1, 2].map(|i| pt.expected_flags(i)), [5, 5, 5]);
            assert!(pt.selectable(0) && pt.selectable(1) && pt.selectable(2));
            assert_eq!(pt.index_of(0x26), Some(2));
            assert_eq!(pt.forms_of(0x25), Forms::starting(0x25));
            assert_eq!(pt.logical_kinds(), ([0x24, 0x25, 0x26], 3));
            assert_eq!(pt.prefetch_kinds(), ([0x24, 0x25, 0x26, PT_OBJECT_KIND], 4));
            assert_eq!(pt.object_kind(), Some(PT_OBJECT_KIND));
            assert!(pt.carries_records() && window_swappable(pt) && swap_commit_index_in(pt, 0) == Some(1));
            assert_eq!(pt.swap_commit_index_for(2), Some(0));
            assert_eq!(pair_resource_references(pt), None);
            // The native PT window: count 3, native flags 5, every form selectable.
            let state = EntryState { count: 3, kinds: [0x24, 0x25, 0x26], actor_count: 1, installed: 3, index: 1,
                previous_or_next: 1, pending: 0xff, flags: 0x05, outer: 6, loader: 14, changing: 0, guard: 0 };
            assert_eq!(pt.ready_index(state), Some(1));
            assert_eq!(pt.ready_index(EntryState { flags: BANK_FLAG, ..state }), None);
            assert!(pt.awaiting_completion(EntryState { index: 2, previous_or_next: 2, outer: 3, loader: 12, changing: 1, ..state }, 2));
            assert!(!pt.awaiting_completion(EntryState { index: 2, previous_or_next: 2, outer: 3, loader: 12, changing: 1, flags: BANK_FLAG, ..state }, 2));
            assert_eq!(native_group_request(3, [kind, Forms::starting(kind).kinds[1], Forms::starting(kind).kinds[2]], 1), Some(Forms::starting(kind)));
        }
        for kind in [0x5au32, 0x5b] {
            let aegis = Members::native_group(kind).unwrap();
            assert_eq!(aegis.kinds(), [0x5a, 0x5b, u32::MAX]);
            assert_eq!((aegis.count(), aegis.actor_count()), (2, 1));
            assert_eq!(aegis.special(), Special::Native(0x64));
            assert_eq!(aegis.expected_flags(1), 0x64);
            assert!(aegis.selectable(0) && aegis.selectable(1) && !aegis.selectable(2));
            assert_eq!(aegis.prefetch_kinds(), ([0x5a, 0x5b, AEGIS_OBJECT_KIND, u32::MAX], 3));
            assert_eq!(aegis.object_kind(), Some(AEGIS_OBJECT_KIND));
            assert!(aegis.carries_records() && swap_commit_index_in(aegis, 0) == Some(1));
            assert_eq!(native_group_request(2, [kind, Forms::starting(kind).kinds[1], u32::MAX], 1), Some(Forms::starting(kind)));
        }
        // Ordinary members carry none of it; window banks still refuse the native kinds.
        let plain = members(&[LINK, SAMUS]);
        assert_eq!((plain.special(), plain.object_kind(), plain.carries_records()), (Special::None, None, false));
        assert_eq!(plain.prefetch_kinds(), ([LINK, SAMUS, u32::MAX, u32::MAX], 2));
        assert!(Members::native_group(LINK).is_none() && Members::native_group(0x4b).is_none());
        assert!(Members::from_cycle(0x24, &[LINK], 2).is_none());
        assert_eq!(native_group_request(1, [LINK, u32::MAX, u32::MAX], 1), None);
        assert_eq!(native_group_request(3, [0x25, 0x24, 0x26], 1), None, "rotated order is not the native expansion of 0x25");
        // The exchange receipt of a native group keeps one actor and the native count.
        let (before, mut after, _) = exchanged_owner(0);
        after.descriptor[0x18..0x24].copy_from_slice(&[0x24, 0, 0, 0, 0x25, 0, 0, 0, 0x26, 0, 0, 0]);
        assert!(GroupExchange::new(before, after, Members::native_group(0x24).unwrap()).is_some());
    }

    #[test]
    fn the_aegis_object_prefetch_leaves_one_key_the_bank_must_release_itself() {
        // `0x17e64d0(R, 0x75)` registers `0x1332/0x1333/0x1338`; `0x17e8610(R, 0x75)`
        // releases only `0x1332/0x1333`, so `0x1338` is the bank's to release (D-SG1).
        assert_eq!(orphan_object_keys(AEGIS_OBJECT_KIND), ([0x1338, u32::MAX], 1));
        assert_eq!(AEGIS_OBJECT_KIND * 42 + 6, 0x1338);
        // The trainer branch falls through the generic loop; fighters take it directly.
        for kind in [PT_OBJECT_KIND, 0x5a, 0x5b, 0x24, 0x25, 0x26, LINK, FOX, POPO, NANA, 0x52] {
            assert_eq!(orphan_object_keys(kind), ([u32::MAX; 2], 0), "{:#x}", kind);
        }
        // Every prefetched kind of a native group is covered once the object kind's
        // orphan keys go first: an Aegis group has exactly one such kind, PT none.
        let (kinds, count) = Members::native_group(0x5b).unwrap().prefetch_kinds();
        assert_eq!(kinds[..count].iter().filter(|k| orphan_object_keys(**k).1 != 0).count(), 1);
        let (kinds, count) = Members::native_group(0x26).unwrap().prefetch_kinds();
        assert!(kinds[..count].iter().all(|k| orphan_object_keys(*k).1 == 0));
    }

    #[test]
    fn pair_group_commits_through_nana_and_counts_two_references() {
        let pair = Members::pair_group();
        assert_eq!(pair.kinds(), [POPO, NANA, u32::MAX]);
        assert_eq!((pair.count(), pair.actor_count(), pair.is_pair(), pair.is_pair_lead()), (2, 2, true, false));
        assert_eq!(pair.special(), Special::Pair);
        assert!(pair.selectable(0) && !pair.selectable(1) && !pair.selectable(2));
        assert_eq!(pair.index_of(NANA), None);
        assert_eq!((pair.expected_flags(0), pair.expected_flags(1)), (PAIR_ACTIVE_FLAG, BANK_FLAG));
        assert_eq!([0, 1, 2].map(|i| pair.body_state(i, Some(0))), [4, 4, 3]);
        assert_eq!([0, 1, 2].map(|i| pair.body_state(i, None)), [3, 3, 3]);
        assert_eq!(pair.forms_of(POPO), Forms::starting(POPO));
        assert_eq!(pair.logical_kinds(), ([POPO, u32::MAX, u32::MAX], 1));
        assert_eq!(pair_resource_references(pair), Some(2));
        assert_eq!(pair.prefetch_kinds(), ([POPO, NANA, u32::MAX, u32::MAX], 2));
        assert_eq!(pair.object_kind(), None);
        assert!(!pair.carries_records() && window_swappable(pair) && swap_commit_index_in(pair, 0) == Some(1));
        // Exit through Nana as the transient commit index; fallback wakes the pair.
        assert_eq!(pair.swap_commit_index_for(0), Some(1));
        assert_eq!(pair.swap_commit_index_for(1), None);
        assert_eq!(pair.fallback_index(1), 0);
        let state = EntryState { count: 2, kinds: [POPO, NANA, u32::MAX], actor_count: 2, installed: 2, index: 0,
            previous_or_next: 0xff, pending: 0xff, flags: PAIR_ACTIVE_FLAG, outer: 6, loader: 14, changing: 0, guard: 0 };
        assert_eq!(pair.ready_index(state), Some(0));
        assert!(state.ready().is_none(), "flags 6 keep a pair group off the canonical pair path (flags 2)");
        assert!(pair.awaiting_completion(EntryState { index: 1, previous_or_next: 1, flags: BANK_FLAG, outer: 3, loader: 12, changing: 1, ..state }, 1));
        assert!(!pair.awaiting_completion(EntryState { index: 1, previous_or_next: 1, flags: PAIR_ACTIVE_FLAG, outer: 3, loader: 12, changing: 1, ..state }, 1));
        // The pair-lead window keeps its own commit pair and three references.
        let lead = Members::pair_lead(LINK).unwrap();
        assert_eq!((lead.swap_commit_index_for(0), lead.swap_commit_index_for(2), lead.swap_commit_index_for(1)), (Some(2), Some(0), None));
        assert!(lead.is_pair() && !window_swappable(lead));
        assert_eq!(pair_resource_references(lead), Some(3));
        // Exchange receipts of two-actor windows carry `+0x28 = 2` (a pair-lead
        // return or a pair group would otherwise fail its own verification).
        let (before, mut after, _) = exchanged_owner(0);
        after.descriptor[0x14..0x18].copy_from_slice(&2u32.to_le_bytes());
        after.descriptor[0x18..0x24].copy_from_slice(&[0x4b, 0, 0, 0, 0x4c, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]);
        after.descriptor[0x28..0x2c].copy_from_slice(&2u32.to_le_bytes());
        after.bodies[2] = (0, 0);
        assert!(GroupExchange::new(before, after, pair).is_some());
        let mut one_actor = after;
        one_actor.descriptor[0x28] = 1;
        assert!(GroupExchange::new(before, one_actor, pair).is_none());
        let mut lead_after = after;
        lead_after.descriptor[0x14] = 3;
        lead_after.descriptor[0x20..0x24].copy_from_slice(&LINK.to_le_bytes());
        lead_after.bodies[2] = (0x8000, 524288);
        assert!(GroupExchange::new(before, lead_after, lead).is_some());
    }

    #[test]
    fn planner_places_pt_aegis_and_pair_as_whole_groups() {
        let windows = [Some(members(&[MARIO, LINK, SAMUS])), Some(members(&[LUIGI, DK, YOSHI]))];
        // PTA116T P1 lane after Samus: Fox, Pikachu, Ness, Falcon, Marth, Aegis, PT, Ridley (0x42).
        let lane: &[u32] = &[FOX, PIKACHU, NESS, FALCON, 0x15, 0x5a, 0x24, 0x42];
        // Policy off: the native kinds are skipped exactly as before.
        let plan = plan_extra_groups(windows, [&[], &[]], [lane, &[]], [None, None], budget(12));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU, NESS], vec![FALCON, 0x15, 0x42]]);
        // Policy on, 12 bodies: window 3+3 leaves 6, all spent on the five nearer
        // ordinaries and Ridley before the Aegis/PT depths: specials never jump the queue.
        let plan = plan_extra_groups(windows, [&[], &[]], [lane, &[]], [None, None], budget_special(12, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU, NESS], vec![FALCON, 0x15, 0x42]]);
        // 14 bodies: after the five ordinaries three remain; Aegis (depth 5, two bodies)
        // fits, PT (depth 6, three) does not, Ridley takes the last one.
        let plan = plan_extra_groups(windows, [&[], &[]], [lane, &[]], [None, None], budget_special(14, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![0x5a, 0x5b], vec![FOX, PIKACHU, NESS], vec![FALCON, 0x15, 0x42]]);
        assert_eq!(plan[0][0].unwrap().special(), Special::Native(0x64));
        // Nearest-first: PT right after the window gets its three bodies before anything deeper.
        let near: &[u32] = &[0x26, FOX, PIKACHU];
        let plan = plan_extra_groups(windows, [&[], &[]], [near, &[]], [None, None], budget_special(9, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![0x24, 0x25, 0x26]]);
        assert_eq!(plan[0][0].unwrap().special(), Special::Native(0x05));
        // Eight bodies: two windows leave 2, PT (3) is skipped whole, the pair of ordinaries is taken.
        let plan = plan_extra_groups(windows, [&[], &[]], [near, &[]], [None, None], budget_special(8, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        // An existing PT group is charged and not re-planned; a window holding PT owns groups.
        let pt = Members::native_group(0x24).unwrap();
        let plan = plan_extra_groups(windows, [&[pt], &[]], [near, &[]], [None, None], budget_special(11, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        let plan = plan_extra_groups([Some(pt), None], [&[], &[]], [&[FOX, PIKACHU], &[]], [None, None], budget_special(8, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        // The pair only under its own policy; Nana never appears as a lane kind.
        let ic: &[u32] = &[POPO, FOX, PIKACHU];
        let plan = plan_extra_groups(windows, [&[], &[]], [ic, &[]], [None, None], budget_special(10, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        let plan = plan_extra_groups(windows, [&[], &[]], [ic, &[]], [None, None], budget_special(10, SpecialPolicy::ALL));
        assert_eq!(kinds_of(&plan[0]), vec![vec![POPO, NANA], vec![FOX, PIKACHU]]);
        assert_eq!(plan[0][0].unwrap().special(), Special::Pair);
        // The role's canonical original is never placed, even as a native group.
        let plan = plan_extra_groups(windows, [&[], &[]], [near, &[]], [Some(0x26), None], budget_special(12, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        // Group slots: three specials fill a role's MAX_EXTRA_GROUPS and nothing ordinary follows.
        let many: &[u32] = &[0x24, 0x5a, POPO, FOX];
        let plan = plan_extra_groups([windows[0], None], [&[], &[]], [many, &[]], [None, None],
            GroupBudget { ready_fighters: 40, pool_free: 40, special: SpecialPolicy::ALL });
        assert_eq!(kinds_of(&plan[0]), vec![vec![0x24, 0x25, 0x26], vec![0x5a, 0x5b], vec![POPO, NANA]]);
        // The pair-group policy wins over Solo Popo membership for the same lane kind;
        // without it a Solo lane entry is an ordinary member (EXCLUSIONS row 5).
        let solo_policy = GroupPolicy { solo_popo_groups: true, ..GroupPolicy::DEFAULT };
        let plan = plan_extra_groups_with(solo_policy, windows, [&[], &[]], [ic, &[]], [None, None], budget_special(10, SpecialPolicy::ALL));
        assert_eq!(kinds_of(&plan[0]), vec![vec![POPO, NANA], vec![FOX, PIKACHU]]);
        let plan = plan_extra_groups_with(solo_policy, windows, [&[], &[]], [ic, &[]], [None, None], budget_special(10, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![POPO, FOX, PIKACHU]]);
    }

    #[test]
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
            // A one-member window: entered warm, left by its self-commit; owns nothing new.
            assert!(!window_swappable(mii));
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
        // A Mii window plans nothing new (one-member window); its budget charge is one body.
        let plan = plan_extra_groups([Some(gunner), windows[1]], [&[], &[]], [&[FOX, PIKACHU], &[FALCON, 0x15]], [None, None],
            budget_special(10, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), Vec::<Vec<u32>>::new());
        assert_eq!(kinds_of(&plan[1]), vec![vec![FALCON, 0x15]]);
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

    #[test]
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
    #[test]
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

    #[test]
    fn mii_identity_resource_words_follow_state_six_and_keep_the_window_words() {
        let identity = MiiIdentity { kinds: [0x49, u32::MAX, u32::MAX], costume: 3, index: 1, tokens: [10, 20, 30, 40], flags: [0, 0, 7] };
        // R+0x58.. as an ordinary window leaves it: kinds, costume, index 0 with the upper
        // half unused, zero tokens, the flag word 7c=0 7d=1 7e=1 7f=0, R+0x80 = -1.
        let current: [u32; RESOURCE_WORDS] = [8, 3, 7, 2, 0xbeef_0000, 0, 0, 0, 0, 0x0001_0100, u32::MAX];
        let words = identity.resource_words(current);
        assert_eq!(&words[..4], &[0x49, u32::MAX, u32::MAX, 3]);
        assert_eq!(words[4], 0xbeef_0001, "u16 index; upper half untouched");
        assert_eq!(&words[5..9], &[10, 20, 30, 40]);
        assert_eq!(words[9], 0x0701_0100, "R+0x7c and R+0x7e kept; R+0x7d = FE+0x53 ^ 1; R+0x7f = FE+0x54");
        assert_eq!(words[10], u32::MAX);
        // FE+0x52 set forces R+0x7d = 1 regardless of FE+0x53.
        assert_eq!(MiiIdentity { flags: [1, 1, 0], ..identity }.flag_7d(), 1);
        assert_eq!(MiiIdentity { flags: [0, 1, 0], ..identity }.flag_7d(), 0);
        // The window's own words written back over the identity are the inverse bracket.
        let ordinary = MiiIdentity { kinds: [8, 3, 7], costume: 2, index: 0, tokens: [0; 4], flags: [0, 0, 0] };
        assert_eq!(ordinary.resource_words(words), current);
    }

    #[test]
    fn a_mii_build_exchange_expects_the_identity_header_and_entry_bytes() {
        let (before, mut after, _) = exchanged_owner(0);
        let gunner = Members::mii_group(0x4a).unwrap();
        let identity = MiiIdentity { kinds: [0x4a, u32::MAX, u32::MAX], costume: 2, index: 0, tokens: [75, 125, 0, 37], flags: [0, 0, 0] };
        let mut expected = before.descriptor;
        expected[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
        expected[0x18..0x24].copy_from_slice(&[0x4a, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        expected[0x28..0x2c].copy_from_slice(&1u32.to_le_bytes());
        identity.apply(&mut expected);
        after.descriptor = expected;
        after.header = identity.header(0);
        after.bodies = [(0x6000, 393216), (0, 0), (0, 0)];
        let bytes = identity.entry_bytes(&before.descriptor);
        // Group words alone no longer describe the entry; the identity header and entry bytes do.
        assert!(GroupExchange::new_with_header(before, after, gunner, identity.header(0)).is_none());
        assert!(GroupExchange::new_with_entry(before, after, gunner, before.header, Some(bytes)).is_none());
        let exchange = GroupExchange::new_with_entry(before, after, gunner, identity.header(0), Some(bytes)).unwrap();
        assert!(exchange.permits(after) && exchange.rebaseline(&before.descriptor, after));
        // A different token in the entry, or the window's header, is refused.
        let mut other = after;
        other.descriptor[0x4a] ^= 1;
        assert!(!exchange.permits(other));
        assert!(!exchange.permits(GroupOwner { header: before.header, ..after }));
        // Live CPU behavior bytes stay tolerated as for every exchange.
        let mut behavior = after;
        behavior.descriptor[0x37] = 50;
        assert!(exchange.permits(behavior));
        // Leaving a Mii window warmly restores the saved bytes: the inverse exchange.
        let back = GroupOwner { descriptor: before.descriptor, header: before.header, bodies: before.bodies, ..after };
        let ordinary = Members::new(&[3, 7, 8]).unwrap();
        assert!(GroupExchange::new_with_entry(after, back, ordinary, before.header, Some(entry_bytes_of(&before.descriptor))).is_some());
        assert!(GroupExchange::new_with_header(after, back, ordinary, before.header).is_none());
    }

    #[test]
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

    #[test]
    fn a_mii_group_is_charged_one_body_and_yields_to_the_budget_like_other_specials() {
        let windows = [Some(members(&[MARIO, LINK, SAMUS])), Some(members(&[LUIGI, DK, YOSHI]))];
        // Six bodies in the windows; a budget of 7 leaves one body: the nearest Mii alone fits.
        let plan = plan_extra_groups(windows, [&[], &[]], [&[0x4a, FOX, PIKACHU], &[]], [None, None], budget_special(7, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), vec![vec![0x4a]]);
        // A budget of 6 fits nothing, the Mii included.
        let plan = plan_extra_groups(windows, [&[], &[]], [&[0x4a, FOX, PIKACHU], &[]], [None, None], budget_special(6, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), Vec::<Vec<u32>>::new());
        // A deeper Mii is taken at its depth after the nearer ordinary pair when both fit (6+2+1).
        let plan = plan_extra_groups(windows, [&[], &[]], [&[FOX, PIKACHU, 0x48], &[]], [None, None], budget_special(9, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), vec![vec![0x48], vec![FOX, PIKACHU]]);
        // With one body left the nearest ordinary neighbour wins its one-member group.
        let plan = plan_extra_groups(windows, [&[], &[]], [&[FOX, PIKACHU, 0x48], &[]], [None, None], budget_special(7, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX]]);
        // The pool bound applies to the Mii body too.
        let plan = plan_extra_groups(windows, [&[], &[]], [&[0x4a, FOX, PIKACHU], &[]], [None, None],
            GroupBudget { pool_free: 0, ..budget_special(9, SpecialPolicy::MII) });
        assert_eq!(kinds_of(&plan[0]), Vec::<Vec<u32>>::new());
        // Without the policy a lane Mii is skipped and ordinary planning is unchanged.
        let plan = plan_extra_groups(windows, [&[], &[]], [&[FOX, PIKACHU, 0x48], &[]], [None, None], budget_special(9, SpecialPolicy::NATIVE));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        // A Mii group already held is charged and blocks a second Mii of another class.
        let gunner = Members::mii_group(0x4a).unwrap();
        let plan = plan_extra_groups(windows, [&[gunner], &[]], [&[0x48, FOX, PIKACHU], &[]], [None, None], budget_special(9, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        let plan = plan_extra_groups(windows, [&[gunner], &[]], [&[0x48, FOX, PIKACHU], &[]], [None, None], budget_special(8, SpecialPolicy::MII));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX]]);
    }

    #[test]
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

    #[test]
    fn users_roster_forms_a_rob_lucario_pair_once_rob_is_admitted() {
        const DARK_PIT: u32 = 0x1f;
        const LUCARIO: u32 = 0x2c;
        const ROB: u32 = 0x2d;
        const TOON_LINK: u32 = 0x2e;
        // LOBBY116T: P1 Falcon, Dark Pit, Toon Link, R.O.B., Lucario with the
        // window [Falcon, Dark Pit, Toon Link]. With R.O.B. admitted (EXCLUSIONS
        // row 1) the two leftover bodies form the pair [R.O.B., Lucario], so the
        // lap no longer rebuilds the window at R.O.B. (the canonical original
        // Falcon is still excluded here; row 4 admits it behind its toggle).
        let window = members(&[FALCON, DARK_PIT, TOON_LINK]);
        let lane: &[u32] = &[DARK_PIT, TOON_LINK, ROB, LUCARIO, FALCON];
        assert!(swappable_kind(ROB) && swappable_kind(LUCARIO) && swappable_kind(DARK_PIT) && swappable_kind(TOON_LINK));
        assert!(window_swappable(window) && swap_commit_index_in(window, 0) == Some(1));
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![ROB, LUCARIO]]);
        assert!(plan[1].iter().all(Option::is_none));
        let pair = plan[0][0].unwrap();
        assert_eq!((pair.count(), pair.index_of(ROB), pair.index_of(LUCARIO)), (2, Some(0), Some(1)));
        assert_eq!(swap_commit_index_in(pair, 0), Some(1), "a two-member group leaves through its other cell");
        // Five: one spare body buys the nearest neighbour, R.O.B., as a one-member group.
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [None, None], budget(5));
        assert_eq!(kinds_of(&plan[0]), vec![vec![ROB]]);
        assert_eq!(swap_commit_index_in(plan[0][0].unwrap(), 0), Some(0), "a one-member window leaves by its self-commit");
        // Four bodies: the two windows (three and the CPU's one) use it all.
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [None, None], budget(4));
        assert!(plan.iter().flatten().all(Option::is_none));
        // An excluded canonical original is never placed; the other body is a single.
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [Some(LUCARIO), None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![ROB]]);
        // Both roles banked at the default eight: the two spare bodies make the
        // CPU's nearer pair rather than two one-member groups.
        let cpu = members(&[LUIGI, DK, YOSHI]);
        let cpu_lane: &[u32] = &[PEACH, SAMUS, MARIO];
        let plan = plan_extra_groups([Some(window), Some(cpu)], [&[], &[]], [lane, cpu_lane], [None, None], budget(8));
        assert!(plan[0].iter().all(Option::is_none), "{:?}", plan[0]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH, SAMUS]]);
        // Ten: the CPU triple by depth, then R.O.B. alone.
        let plan = plan_extra_groups([Some(window), Some(cpu)], [&[], &[]], [lane, cpu_lane], [None, None], budget(10));
        assert_eq!(kinds_of(&plan[0]), vec![vec![ROB]]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH, SAMUS, MARIO]]);
        // Eleven: the CPU triple and P1's pair.
        let plan = plan_extra_groups([Some(window), Some(cpu)], [&[], &[]], [lane, cpu_lane], [None, None], budget(11));
        assert_eq!(kinds_of(&plan[0]), vec![vec![ROB, LUCARIO]]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH, SAMUS, MARIO]]);
        // One spare body with the other role's only candidate deeper: nearest first.
        let plan = plan_extra_groups([Some(window), Some(cpu)], [&[], &[]], [lane, &[0x48, 0x4b, 0x24, PEACH]], [None, None], budget(7));
        assert_eq!(kinds_of(&plan[0]), vec![vec![ROB]]);
        assert!(plan[1].iter().all(Option::is_none));
    }

    #[test]
    fn leftover_bodies_form_pairs_before_singles_and_two_pairs_before_a_triple_and_a_single() {
        assert_eq!(group_sizes(0), [0, 0, 0]);
        assert_eq!(group_sizes(1), [1, 0, 0]);
        assert_eq!(group_sizes(2), [2, 0, 0]);
        assert_eq!(group_sizes(3), [3, 0, 0]);
        assert_eq!(group_sizes(4), [2, 2, 0]);
        assert_eq!(group_sizes(5), [3, 2, 0]);
        assert_eq!(group_sizes(6), [3, 3, 0]);
        assert_eq!(group_sizes(7), [3, 2, 2]);
        assert_eq!(group_sizes(8), [3, 3, 2]);
        assert_eq!(group_sizes(9), [3, 3, 3]);
        assert_eq!(group_sizes(10), [3, 3, 3]);
        for bodies in 0..=9 {
            let sizes = group_sizes(bodies);
            assert_eq!(sizes.iter().sum::<usize>(), bodies);
            assert_eq!(sizes.iter().filter(|s| **s == 1).count(), usize::from(bodies == 1));
            assert!(sizes.iter().all(|s| *s <= NATIVE_MEMBERS));
        }
        let window = members(&[MARIO, LINK, SAMUS]);
        let lane: &[u32] = &[FOX, PIKACHU, NESS, FALCON, PEACH, LUIGI, DK];
        // Four leftover bodies: two pairs, so every group keeps an in-window neighbour.
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU], vec![NESS, FALCON]]);
        // Seven: a triple and two pairs, at most MAX_EXTRA_GROUPS.
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [None, None], budget(11));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU, NESS], vec![FALCON, PEACH], vec![LUIGI, DK]]);
        // Five and six: a triple and a pair, two triples.
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [None, None], budget(9));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU, NESS], vec![FALCON, PEACH]]);
        let plan = plan_extra_groups([Some(window), None], [&[], &[]], [lane, &[]], [None, None], budget(10));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU, NESS], vec![FALCON, PEACH, LUIGI]]);
        // One body left beside an existing pair: groups never grow, so it is a single
        // in the middle of the lane (the pair's members are skipped).
        let existing = [members(&[FOX, PIKACHU])];
        let plan = plan_extra_groups([Some(window), None], [&existing, &[]], [lane, &[]], [None, None], budget(7));
        assert_eq!(kinds_of(&plan[0]), vec![vec![NESS]]);
        // Both roles down to one candidate each with two bodies: two singles, nearest first.
        let cpu = members(&[LUIGI, DK, YOSHI]);
        let plan = plan_extra_groups([Some(window), Some(cpu)], [&[], &[]], [&[FOX], &[PEACH]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX]]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH]]);
        // The same two bodies with a deeper P1 lane become P1's pair instead.
        let plan = plan_extra_groups([Some(window), Some(cpu)], [&[], &[]], [&[FOX, PIKACHU], &[PEACH]], [None, None], budget(8));
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        assert!(plan[1].iter().all(Option::is_none));
        // A role in a one-member window plans like any other (row 2: it leaves by its
        // self-commit); its detached groups stay charged, one group slot each, and the
        // other role plans as usual (12 − 1 − 3 − 3 = 5 bodies: P1's triple, the CPU pair).
        let detached = [members(&[MARIO, LINK, SAMUS])];
        let plan = plan_extra_groups([Some(members(&[FOX])), Some(cpu)], [&detached, &[]], [lane, &[PEACH, SAMUS]], [None, None], budget(12));
        assert_eq!(kinds_of(&plan[0]), vec![vec![PIKACHU, NESS, FALCON]]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH, SAMUS]]);
        // Every planned group is distinct, in lane order and within the body budget.
        for ready in 6..=16 {
            let plan = plan_extra_groups([Some(window), Some(cpu)], [&[], &[]], [lane, &[PEACH, SAMUS, FALCON, NESS]], [None, None], budget(ready));
            let mut bodies = 0;
            for (role, groups) in plan.iter().enumerate() {
                let mut seen = Vec::new();
                for group in groups.iter().flatten() {
                    for kind in &group.kinds()[..group.count()] {
                        assert!(!seen.contains(kind), "{}: duplicate {:#x} in role {}", ready, kind, role);
                        seen.push(*kind);
                        bodies += 1;
                    }
                }
            }
            assert!(bodies <= ready.saturating_sub(6), "{ready}: {:?}", plan);
        }
    }

    /// rc.6 item 3, a session that starts on Ice Climbers (Eden rc6-bg-ic2, `ic-start-loaded`): P1's live pair-lead
    /// window `[Popo, Nana, DK]` and its built group `[Link, Samus, Yoshi]` are every P1 fighter, yet the notice stayed
    /// "Loaded 9/10 · swap P1 to load 1 more". The planner counted the window the lane's next kind would get (`[DK, Link,
    /// Samus]`, two extras) where the live pair-lead window serves one. With groups on that window stays while the
    /// groups build around it, so it plans what it serves, the count the notice reads; with groups off, or for any
    /// other window, the next swap still builds the lane's window.
    #[test]
    fn a_live_pair_lead_window_plans_what_it_serves() {
        use crate::engine::preload::cycle_preload_capacity::reusable_extras;
        let window = Members::pair_lead(DK).unwrap();
        let group = count_extra_groups(&[(members(&[LINK, SAMUS, YOSHI]), GroupState::Built)]);
        let next = reusable_extras(&[DK, LINK, SAMUS], DK, Some(POPO));
        assert_eq!(next, 2);
        for (active, served) in [(POPO, 1), (DK, 0)] {
            let (kinds, count) = window.logical_kinds();
            let extra = reusable_extras(&kinds[..count], active, Some(POPO));
            assert_eq!(extra, served, "{active:#x}: Popo is the canonical original");
            let planned = window_targets(true, Some((window, active)), Some(POPO), next) + group.planned;
            let role = role_counts(planned, extra, WindowStatus::Built, group);
            assert_eq!((role.planned, role.ready, role.unstarted()), (served + 3, served + 3, 0), "{active:#x}");
            assert_eq!(role.swap_owed(0), 0);
        }
        assert_eq!(window_targets(true, Some((window, POPO)), None, next), 1, "an original that is not the pair");
        assert_eq!(window_targets(false, Some((window, POPO)), Some(POPO), next), next, "groups off: the next swap builds it");
        assert_eq!(window_targets(true, None, Some(POPO), next), next, "no live window");
        let ordinary = members(&[MARIO, DK, LINK]);
        assert_eq!(window_targets(true, Some((ordinary, MARIO)), None, next), next, "an ordinary window");
    }

    /// tr-p23-hot-measure-1, g03 frame 85 ("Loaded 2/5 (3 failed)"): the CPU window `[34,0,1]` built, its two
    /// reusable extras ready, and the next group `[2,3,4]` refused at its prefetch start for headroom. Held, the
    /// group is counted apart from failures and keeps its place in the plan and the body budget.
    #[test]
    fn held_group_counts_apart_from_failed() {
        let held = count_extra_groups(&[(members(&[0x2, 0x3, 0x4]), GroupState::Held(Hold::Memory))]);
        assert_eq!(held, GroupCounts { planned: 3, held: 3, hold: Some(Hold::Memory), ..Default::default() });
        let role = role_counts(5, 2, WindowStatus::Built, held);
        assert_eq!((role.planned, role.ready, role.failed, role.pending, role.held, role.unstarted()), (5, 2, 0, 0, 3, 0));
        assert_eq!(role.swap_owed(1), 0, "a held group does not wait for a swap");
        assert!(GroupState::Held(Hold::Memory).counts_bodies() && GroupState::Held(Hold::Pool).counts_bodies());
        assert!(!GroupState::Held(Hold::Memory).holds_keys() && !GroupState::Held(Hold::Memory).reachable());
        assert!(GroupState::Held(Hold::Pool).startable() && GroupState::Planned.startable());
        for state in [GroupState::Prefetching, GroupState::Prefetched, GroupState::Built, GroupState::Failed] {
            assert!(!state.startable(), "{state:?}");
        }
        let both = count_extra_groups(&[(members(&[0x7]), GroupState::Held(Hold::Pool)),
            (members(&[0x8, 0xb]), GroupState::Held(Hold::Memory))]);
        assert_eq!((both.held, both.hold), (3, Some(Hold::Memory)), "memory outranks the pool");
        assert_eq!(merge_hold(Some(Hold::Pool), None), Some(Hold::Pool));
        assert_eq!(merge_hold(None, None), None);
    }

    /// P-HOT (§3.2, DECISIONS 2026-11-02: "Hat sources are not charged against the fighter budget"): Ready Fighters 10
    /// with both windows built (6 bodies) leaves 4 bodies for groups, interleaved by depth, whatever hats a Kirby role
    /// wants; the old rule charged two hats first and left the CPU nothing.
    #[test]
    fn hats_are_not_charged_against_ready_fighters() {
        let windows = [Some(members(&[MARIO, LINK, SAMUS])), Some(members(&[LUIGI, DK, YOSHI]))];
        let upcoming: [&[u32]; 2] = [&[FOX, PIKACHU, NESS, FALCON, PEACH], &[PEACH, FALCON, NESS, PIKACHU, FOX]];
        let plan = plan_extra_groups(windows, [&[], &[]], upcoming, [None, None],
            GroupBudget { ready_fighters: 10, pool_free: 16, special: SpecialPolicy::NONE });
        assert_eq!(kinds_of(&plan[0]), vec![vec![FOX, PIKACHU]]);
        assert_eq!(kinds_of(&plan[1]), vec![vec![PEACH, FALCON]]);
    }

    /// The start order: a planned group first; held groups only when none is planned, rotating from `turn` so
    /// a large held group never starves a smaller one after it; a Mii group waits for its record.
    #[test]
    fn held_groups_retry_after_planned_and_rotate() {
        let held = GroupState::Held(Hold::Memory);
        let groups = [(held, true), (GroupState::Planned, true), (held, true)];
        assert_eq!(next_start(groups.iter().copied(), 0), Some(1), "a planned group goes before every held one");
        let groups = [(held, true), (GroupState::Built, true), (held, true)];
        assert_eq!(next_start(groups.iter().copied(), 0), Some(0));
        assert_eq!(next_start(groups.iter().copied(), 1), Some(2), "the turn passes to the next held group");
        assert_eq!(next_start(groups.iter().copied(), 3), Some(0), "and wraps");
        let groups = [(GroupState::Planned, false), (held, false), (GroupState::Prefetched, true)];
        assert_eq!(next_start(groups.iter().copied(), 0), None, "a Mii group waits for its record; nothing else starts");
        assert_eq!(next_start([].iter().copied(), 5), None);
    }
}
