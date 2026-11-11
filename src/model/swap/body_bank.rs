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
    d_kirby1_applies, d_kirby1_live, group_member_kind, key_release_admitted, kirby_parkable, member_forms, member_forms_in,
    native_group_request, object_rearm_admitted, original_bankable, original_form_bankable, pair_lead_extra, pair_request,
    prefetch_admitted,
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

