//! One capability table per fighter form, and the eligibility predicates derived from it.
//!
//! SYNTHESIS agree-1; Fable REDESIGN §2.6 (the table shape); Astra REDESIGN §8 (`qualification`);
//! `redesign/fable/HUD.md` §2.1 (the `presentation` row); WORKPLAN §2.2. The gates the body bank
//! and its adapters consult are one-liners over `capabilities(form_class(kind, forms))`. The
//! `body_bank` functions they replace are re-exported from there with their exact signatures, so no
//! caller changes; `swap_model::tests` keeps the old bodies as the oracle and proves old == new over
//! every kind, form, policy and window shape (behaviour-neutral by construction).
//!
//! Every row cites the FACT it encodes (RCA section, listing or Eden row). Values only: no
//! pointer, no native call, no lock. A `Qualification::Gated` row is the one-line fallback
//! WORKPLAN §5.3 names (Kirby canonical-only if the K-rows are red); the shipped rows are all
//! `Shipped` except the Mii row (Mii groups land with the M0 merge) and `Foreign`.

use crate::engine::swap::body_bank::{GroupPolicy, Members, Special, AEGIS_OBJECT_KIND, KIRBY, NANA, NATIVE_MEMBERS, POPO, PT_OBJECT_KIND};
#[cfg(test)]
use crate::engine::swap::body_bank::SpecialPolicy;
use crate::engine::swap::character_forms::Forms;
use crate::engine::mii::inventory::MiiClass;
use crate::engine::swap::trio::TrioFighter;

/// R.O.B.: the native costume selector keys on the primary kind `R+0x58` at eight sites, three
/// on the resource worker (D-KIRBY1-ROB1-RCA §4.1); kept fast by operator decision (C115 §264).
pub const ROB: u32 = 0x2d;

/// The form a request or a bank member belongs to. One row of [`capabilities`] per variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormClass {
    /// One native body in any window or group cell (`Members::new` admits it).
    Ordinary,
    /// Ordinary body whose *departure* is gated on its copy state (EXCLUSIONS-AUDIT §2) and whose
    /// resource owner holds no bank prefetch key while it is live (D-KIRBY1-ROB1-RCA §3).
    Kirby,
    /// Ordinary body whose costume follows the window's primary kind (D-ROB1; operator: keep fast).
    Rob,
    /// A Mii class (`0x48..=0x4a`): never a window member; its own one-member group once the Mii
    /// groups patch is merged (M0), one identity per resource owner (MII-GROUPS-DESIGN §1).
    Mii(MiiClass),
    /// Pokémon Trainer `[0x24, 0x25, 0x26]`, flags `0x05`, object kind `0x72` (SPECIAL-GROUPS §2).
    PokemonTrainer,
    /// Pyra/Mythra `[0x5a, 0x5b]`, flags `0x64`, object kind `0x75` (SPECIAL-GROUPS §3).
    Aegis,
    /// The normal Ice Climbers pair: the pair-lead window `[Popo, Nana, X]` (ICE-CLIMBERS
    /// addendum) or the detached pair group (`Special::Pair`, SPECIAL-GROUPS §5).
    IcPair,
    /// Solo Popo: one body at physical cell 0 of any window or group that holds it
    /// (`CellRule::PinCell0`: `0x17e8410` skips a secondary Popo, SOLO-STALL-RCA §3).
    SoloPopo,
    /// Not a roster fighter kind (Nana `0x4c`, `0x4e..=0x50`, `0x6e..=0x75`, `u32::MAX`): never a
    /// member of anything. (Ninth variant beyond WORKPLAN §2.2's eight so that every predicate
    /// answers for every `u32` the adapters pass.)
    Foreign,
}

/// The class of `kind` in `forms`: the pair and Solo Popo are two classes of one kind
/// (Astra §6.1; FINAL-INT §6), everything else is decided by the kind alone.
pub fn form_class(kind: u32, forms: Forms) -> FormClass {
    match kind {
        KIRBY => FormClass::Kirby,
        ROB => FormClass::Rob,
        0x24..=0x26 => FormClass::PokemonTrainer,
        0x5a | 0x5b => FormClass::Aegis,
        POPO => if forms == Forms::solo_popo() { FormClass::SoloPopo } else { FormClass::IcPair },
        _ => match MiiClass::from_kind(kind) {
            Some(class) => FormClass::Mii(class),
            None if TrioFighter::from_kind(kind).is_some() => FormClass::Ordinary,
            None => FormClass::Foreign,
        },
    }
}

/// Which window cells a member of this form may occupy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellRule {
    /// Any of the three cells.
    Any,
    /// Any rotation position, but the planner lays every window or group that contains this form
    /// out with it at **physical cell 0** (`entry+0x18[0]`, `R+0x58[0]`); the other members fill
    /// cells 1–2 in rotation order and the target activates through its own cell (descriptor
    /// starting form `+0x43` at a Reload — the Reload start copies it to `+0x5919` at `0x6533f4` —,
    /// `set_active_index` warm). Two rows pin, each by its own FACT:
    /// * Kirby (`wp-k/RE-KIRBY-UNLOAD.md` §0–§3, K-R1 on Eden): native's copy registry assumes
    ///   Kirby is sub 0 of its entry — `prepare_entry` skips the own entry only when `k0 == 6`,
    ///   opponents' mirrors go to `cell[ke][0]` hard-coded, a copy ctx carries no own-sub field —
    ///   so a Kirby built at sub 1/2 registers self-mirrors and its unload spins (S-B, D-KIRBY1).
    /// * Solo Popo (SOLO-STALL-RCA §3, L116): the native extra registration `0x17e8410`/`0x17e8440`
    ///   registers the primary `R+0x58` unconditionally and skips a *secondary* `0x4b`/`0x4c`, and
    ///   `0x17e5bc0`'s Popo entry (`kind − 0x24` table `0x44f58f4` → `0x17e5c7c`) requests the
    ///   primary's partner word `R+0x5c`; a Popo at cell 0 is resourced whatever cell is active.
    ///
    /// One pinned form per window or group (both need cell 0): when a rotation holds two, the later
    /// one in rotation order waits for the next window ([`crate::engine::swap::body_bank::Members::from_forms`]) or
    /// takes its own group (`plan_extra_groups_with`). A pair-lead window has no cell 0 to give, so
    /// it never takes a pinned form as its extra (`pair_lead_extra`).
    PinCell0,
    /// Cells 0/1 are the pair itself; cell 2 holds one ordinary extra (pair-lead window).
    PairLead,
    /// 7a candidate (WORKPLAN §1.2, §2.14 step 6): built only as the active member (cell 0 at a
    /// window build), never parked in cells 1–2 nor in a detached group. No shipped row uses it;
    /// the K-rows decide whether the Kirby row flips to it or to `Qualification::Gated`.
    ActiveOnly,
}

/// The policy switch that admits a form's own detached group (`SpecialPolicy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecialSwitch {
    /// PT / Aegis (`native_groups`).
    Native,
    /// The pair group (`pair_groups`).
    Pair,
    /// The Mii one-member group (`mii_groups`, M0): no switch exists in this stage, so it is
    /// never admitted here until the merge adds `SpecialPolicy::mii`.
    Mii,
}

impl SpecialSwitch {
    #[cfg(test)]
    pub fn enabled(self, policy: SpecialPolicy) -> bool {
        match self {
            SpecialSwitch::Native => policy.native,
            SpecialSwitch::Pair => policy.pair,
            SpecialSwitch::Mii => policy.mii,
        }
    }
}

/// The `GroupPolicy` switch that admits a mixed member form to detached groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupSwitch {
    /// Solo Popo (`solo_popo_groups`, EXCLUSIONS §4b).
    SoloPopo,
}

impl GroupSwitch {
    pub fn enabled(self, policy: GroupPolicy) -> bool {
        match self {
            GroupSwitch::SoloPopo => policy.solo_popo_groups,
        }
    }
}

/// How a form takes part in the group swap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grouping {
    /// Mixes with other single bodies in windows and detached groups.
    MixedMember,
    /// Forms only its own complete group, admitted by one `SpecialPolicy` switch (the payload is a
    /// deviation from WORKPLAN §2.2's bare variant: the switch is table data, not a match).
    OwnGroupOnly(SpecialSwitch),
    /// A window shape only (or nothing at all): never a detached-group member.
    WindowOnly,
}

/// What the form reads from the shared resource owner `R` beyond its own kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RIdentity {
    /// Nothing R-wide: the body takes the window's `R` as it finds it.
    Inherits,
    /// Costume follows `R+0x58`/`R+0x64` of the primary (D-KIRBY1-ROB1-RCA §4.1); accepted visual
    /// deviation (Famicom colours as a non-primary), operator decision.
    ReadsPrimaryCostume,
    /// Needs its own `R+0x58..0x80` identity (MII-GROUPS-DESIGN §1): one Mii identity per R.
    RequiresOwn,
}

/// The gate a live member of this form passes before it is parked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParkGate {
    None,
    /// `FLAG_COPY` clear, no retained copy object groups, no hat lease on the body
    /// (EXCLUSIONS-AUDIT §2; `copy_free_departure`).
    CopyFree,
    /// Both Climbers state 4 and outside Dead/Rebirth/Entry/Standby (addendum gates); native
    /// reads, decided by the adapter (`pair_parkable`).
    PairAlive,
}

/// The in-window commit index a warm exit selects (GROUP-SWAP-FEASIBILITY §3; EXCLUSIONS row 6).
/// Every window shape has one: the commit only parks `cell[+0x5918]` and selects the requested
/// index's own context, whichever index that is (see [`ExitRule::SelfCell`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitRule {
    /// `(from + 1) % count`: ordinary windows and native groups.
    NextCell,
    /// Pair (0) ↔ extra (2), never Nana's cell: the pair-lead window.
    PairExtraOnly,
    /// Through Nana (1) as the transient commit index: the detached pair group (SPECIAL-GROUPS §5).
    ViaNana,
    /// A one-member window commits to its own cell (`+0x591a = +0x5918 = 0`, a self-commit): the
    /// commit parks the only body and re-selects that body's own context, and the exchange at the
    /// acknowledgement seam replaces the window as for every group swap. FACT (owned 13.0.5 ELF,
    /// `refactor/popo-oneexit/re/`): `6581f0` gates only on `+0x5920 == 6` (`0x65821c`) and
    /// `+0x591a != 0xff` (`0x658234`) and never compares the request with `+0x5918`; the fighter
    /// manager calls it for every entry each frame (`0x678da4..0x678e00`); `64fb70` resolves the
    /// kind word at the selected index through the lent table, i.e. cell 0's own context (never the
    /// empty padding cell whose null context `0x64fc1c` would store through); loader state 12
    /// (`0x655b80`) only waits for `+0x5933`; after the exchange, state 13's activation handler for
    /// a commit (`+0x591b == 7`, `0x65655c`) reads `cell[+0x5910]` and `cell[+0x5918]`, both cell 0
    /// of the new window once the exchange has rewritten `+0x5910` ([`previous_cell_index`]: the
    /// commit left the *window's* index there, which a smaller group lacks; the C116.1 console
    /// crash). Replaces GROUPS-OF-ONE §1.2's "the commit needs a second cell": that
    /// held only for the `(from + 1) % count` rule, whose second index is a padding word.
    SelfCell,
}

/// What proves the form's transition complete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion {
    /// The bank's acknowledgement byte `+0x5933`.
    BankAck,
    /// The native form-change script (PT `0x34cebe0`, Aegis `0x33f4920`).
    NativeScript,
}

/// Whether the row ships as it stands (Astra §8). `Gated` turns every derived admission off and
/// names the evidence that is missing; it is WORKPLAN §5.3's one-row fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qualification {
    Shipped,
    Gated(&'static str),
}

/// The HUD bust rule (`redesign/fable/HUD.md` §2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BustRule {
    /// The static kind portrait only.
    Never,
    /// The exact configured bust when a configuration is staged, the class portrait otherwise.
    ExactWhenConfigured,
}

/// The `presentation` row (`redesign/fable/HUD.md` §2.1; WORKPLAN §2.12 rule 1): what the one HUD
/// publisher derives from a transition `Target` of this form. Ordinary, Kirby, Solo Popo, PT and
/// Aegis present `Kind`; the pair presents the pair plate; a Mii presents the exact bust or the
/// class portrait; PT/Aegis follow the native Down+B form change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationRule {
    /// The synthetic kind event (`0x1d`) names the live kind.
    pub kind_event: bool,
    pub bust: BustRule,
    /// Both Climbers on the plate (`publish_partners`).
    pub pair_plate: bool,
    /// The waiting-room CPU4 card icon follows the kind (`refresh_waiting_icon`, owner 4 only).
    pub waiting_icon: bool,
    /// The native form change (`+0x5918` under a bank record, R-PT3) re-derives the presentation.
    pub native_form_follow: bool,
}

/// One row of the table. Fields are facts about the form, never about a scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    /// Native bodies of one request (`Forms::starting(kind).count`).
    pub bodies: u8,
    /// Simultaneous actors (`+0x28`): two for the pair, one otherwise.
    pub actors: u8,
    /// Flags `654dc0` writes for the form (`+0x591f`): `0x05` PT, `0x64` Aegis, `0x02` the pair.
    pub native_flags: u8,
    pub window_cell: CellRule,
    pub grouping: Grouping,
    /// The policy switch a `MixedMember` form needs to join a detached group; `None`: no switch.
    pub group_switch: Option<GroupSwitch>,
    pub r_identity: RIdentity,
    /// Object kind the synchronous build looks up through `R` (PT `0x72`, Aegis `0x75`), prefetched
    /// by the bank and released by owned code (no construction tail releases it).
    pub object_kind: Option<u32>,
    /// The entry records `+0x5a88/+0x5a90/+0x5a98` travel with the form's saved window.
    pub carries_records: bool,
    /// The construction tail takes the `R+0x50` pair rule for every body of the window (cells 0/1
    /// `[Popo, Nana]`, C122): one reference per body, whole-R reset after the last.
    pub pair_rule: bool,
    pub park_gate: ParkGate,
    pub exit_commit: ExitRule,
    pub completion: Completion,
    /// The slot-0 original of this form may be a bank/group member (EXCLUSIONS §9).
    pub original_bankable: bool,
    pub qualification: Qualification,
    /// A bank prefetch key may rest on the role's `R` while a member of this form is live. False
    /// for Kirby: a copy registers the copied kind on Kirby's own `R` and `0x17e8610(R, k)` also
    /// releases R's G68 mirror for `(k, costume)` (D-KIRBY1-ROB1-RCA §3.2–3.3; Astra §4.3 drain and
    /// suspend rule).
    pub prefetch_while_live: bool,
    pub presentation: PresentationRule,
}

const KIND: PresentationRule = PresentationRule {
    kind_event: true, bust: BustRule::Never, pair_plate: false, waiting_icon: true, native_form_follow: false,
};

const NO_PRESENTATION: PresentationRule = PresentationRule {
    kind_event: false, bust: BustRule::Never, pair_plate: false, waiting_icon: false, native_form_follow: false,
};

/// The ordinary row every other row is a delta of: one body, any cell, mixed member, bank
/// acknowledgement, bankable original (`body_bank::bankable_kind`/`swappable_kind` as of B123-rc).
const ORDINARY: Capabilities = Capabilities {
    bodies: 1, actors: 1, native_flags: 0, window_cell: CellRule::Any, grouping: Grouping::MixedMember,
    group_switch: None, r_identity: RIdentity::Inherits, object_kind: None, carries_records: false, pair_rule: false,
    park_gate: ParkGate::None, exit_commit: ExitRule::NextCell, completion: Completion::BankAck,
    original_bankable: true, qualification: Qualification::Shipped, prefetch_while_live: true,
    presentation: KIND,
};

/// ONE table. Each row cites the FACT it encodes.
pub const fn capabilities(class: FormClass) -> Capabilities {
    match class {
        FormClass::Ordinary => ORDINARY,
        // EXCLUSIONS-AUDIT §2: native has no Kirby case on the resource side; departure gated on
        // the copy state. D-KIRBY1 (FINAL-INT §3): no prefetch key on R while Kirby is live.
        FormClass::Kirby => Capabilities {
            window_cell: CellRule::PinCell0, park_gate: ParkGate::CopyFree, prefetch_while_live: false, ..ORDINARY
        },
        // D-KIRBY1-ROB1-RCA §4.1 (eight selector sites); operator: keep R.O.B. fast (FINAL-INT §3).
        FormClass::Rob => Capabilities { r_identity: RIdentity::ReadsPrimaryCostume, ..ORDINARY },
        // MII-GROUPS-DESIGN §1–§3: body params/hat from `R+0x58..0x80`, one identity per R, a
        // one-member group, left by its self-commit like every one-member window (`ExitRule::SelfCell`;
        // the exchange's identity move-out, DESIGN §2.2 "warm attach/detach"); `Members::new`
        // refuses the Mii range.
        FormClass::Mii(_) => Capabilities {
            grouping: Grouping::OwnGroupOnly(SpecialSwitch::Mii), r_identity: RIdentity::RequiresOwn,
            exit_commit: ExitRule::SelfCell, original_bankable: false,
            // M0 merged the Mii groups (`mii_groups` switch; default decided at M5 by M1–M13, WORKPLAN §5.3).
            qualification: Qualification::Shipped,
            presentation: PresentationRule { bust: BustRule::ExactWhenConfigured, ..KIND }, ..ORDINARY
        },
        // SPECIAL-GROUPS §2: flags 0x05, trainer object `+0x5a88/+0x5a98`, kind 0x72 through R
        // (`0x658bb4`), completion by the native script.
        // EXCLUSIONS §9 lifted for the native-group original (INBOX-opus-rc6-hotslots 01:09Z: "All groups in rc.6"; a
        // session that starts on PT/Aegis keeps its bank): the start-up window or the record holding it serves it, while
        // native groups are on (`original_form_bankable`).
        FormClass::PokemonTrainer => Capabilities {
            bodies: 3, native_flags: 0x05, grouping: Grouping::OwnGroupOnly(SpecialSwitch::Native),
            object_kind: Some(PT_OBJECT_KIND), carries_records: true, completion: Completion::NativeScript,
            original_bankable: true, presentation: PresentationRule { native_form_follow: true, ..KIND }, ..ORDINARY
        },
        // SPECIAL-GROUPS §3: flags 0x64, no third cell (Down+B modulo `+0x4150`), kind 0x75 from the
        // `0x17e5bc0` Aegis branch; D-SG1: its `0x1338` key is the bank's to release.
        FormClass::Aegis => Capabilities {
            bodies: 2, native_flags: 0x64, grouping: Grouping::OwnGroupOnly(SpecialSwitch::Native),
            object_kind: Some(AEGIS_OBJECT_KIND), carries_records: true, completion: Completion::NativeScript,
            original_bankable: true, presentation: PresentationRule { native_form_follow: true, ..KIND }, ..ORDINARY
        },
        // C122: `R+0x50` pair rule on cells 0/1 (one reference per body); ICE-CLIMBERS addendum:
        // pair-lead window, commit pair ↔ extra (never Nana), N1/N2/N3; SPECIAL-GROUPS §5: the pair
        // group exits through Nana. Shipped: `pair_groups` default on since C116.1 (E6 is the row).
        // EXCLUSIONS §9 lifted for the pair original (operator decisions 2026-11-08 "All groups in rc.6" and "a
        // session that starts on IC/PT still gets its groups built"; primary 2026-11-09): a pair-lead window or a pair
        // group serves it, while the pair bank is on (`original_form_bankable`).
        FormClass::IcPair => Capabilities {
            bodies: 2, actors: 2, native_flags: 0x02, window_cell: CellRule::PairLead,
            grouping: Grouping::OwnGroupOnly(SpecialSwitch::Pair), pair_rule: true, park_gate: ParkGate::PairAlive,
            exit_commit: ExitRule::PairExtraOnly, original_bankable: true,
            presentation: PresentationRule { pair_plate: true, ..KIND }, ..ORDINARY
        },
        // SOLO-STALL-RCA §3: `0x17e8410` skips a secondary Popo, so the layout pins it to cell 0
        // (`CellRule::PinCell0`) at any rotation position; joins detached groups under
        // `solo_popo_groups` (EXCLUSIONS §4b).
        FormClass::SoloPopo => Capabilities {
            window_cell: CellRule::PinCell0, group_switch: Some(GroupSwitch::SoloPopo), original_bankable: false, ..ORDINARY
        },
        FormClass::Foreign => Capabilities {
            bodies: 0, actors: 0, grouping: Grouping::WindowOnly, original_bankable: false,
            qualification: Qualification::Gated("not a roster fighter kind"), presentation: NO_PRESENTATION, ..ORDINARY
        },
    }
}

// ----- derived predicates over a row ----------------------------------------------------------

/// An ordinary single body at any rotation position: the `bankable_kind` set (Ordinary, Kirby,
/// R.O.B., Solo Popo). Kirby's and Solo Popo's rows pin their *physical* cell; their rotation
/// position is free.
pub fn single_body_member_of(row: &Capabilities) -> bool {
    row.bodies == 1 && row.grouping == Grouping::MixedMember
        && matches!(row.window_cell, CellRule::Any | CellRule::PinCell0)
        && row.qualification == Qualification::Shipped
}

/// May a member of this row occupy window cell `cell`? For [`CellRule::PinCell0`] the answer is
/// about the rotation position (any): the layout moves the form to cell 0 ([`pins_cell0`]).
pub fn can_window_member_of(row: &Capabilities, cell: usize) -> bool {
    row.qualification == Qualification::Shipped
        && match row.window_cell {
            CellRule::Any | CellRule::PinCell0 => row.bodies == 1 && row.grouping == Grouping::MixedMember,
            CellRule::ActiveOnly => cell == 0 && row.grouping == Grouping::MixedMember,
            CellRule::PairLead => cell == 0,
        }
}

/// May the planner place a member of this row in a detached group under `policy`? A row with a
/// [`GroupSwitch`] needs that switch on (Solo Popo: `solo_popo_groups`).
pub fn can_group_member_of(row: &Capabilities, policy: GroupPolicy) -> bool {
    row.qualification == Qualification::Shipped && row.grouping == Grouping::MixedMember
        && row.group_switch.is_none_or(|switch| switch.enabled(policy))
        && match row.window_cell {
            CellRule::Any | CellRule::PinCell0 => row.bodies == 1,
            CellRule::ActiveOnly | CellRule::PairLead => false,
        }
}

/// The form's row pins it to physical cell 0 of any window or group that holds it (Kirby, Solo
/// Popo).
pub fn pins_cell0_of(row: &Capabilities) -> bool { row.window_cell == CellRule::PinCell0 }

/// [`pins_cell0_of`] for a single-body member kind (`0x06`, and `0x4b`, whose member form is Solo).
pub fn pins_cell0(kind: u32) -> bool { pins_cell0_of(&capabilities(single_member_class(kind))) }

/// A layout native never builds: a kind whose row pins cell 0 sitting at sub 1/2 of a multi-kind
/// entry. For Kirby that is RE-KIRBY-UNLOAD §5's self-mirror (a mirror into its own cell and one
/// into the cell-0 member's cell; `unprepare` `0x17f26c0` unlinks the same node twice, §3); for Solo
/// Popo the secondary registration `0x17e8410` skips. A planned layout is valid only when this is
/// false; `Members::new` refuses such a layout.
#[cfg(test)]
pub fn self_mirror_hazard(cells: &[u32]) -> bool {
    cells.iter().enumerate().any(|(i, kind)| i != 0 && *kind != u32::MAX && pins_cell0(*kind))
}

/// Does this row form its own complete group under `policy`?
#[cfg(test)]
pub fn forms_own_group_of(row: &Capabilities, policy: SpecialPolicy) -> bool {
    row.qualification == Qualification::Shipped
        && matches!(row.grouping, Grouping::OwnGroupOnly(switch) if switch.enabled(policy))
}

pub fn single_body_member(class: FormClass) -> bool { single_body_member_of(&capabilities(class)) }
pub fn can_window_member(class: FormClass, cell: usize) -> bool { can_window_member_of(&capabilities(class), cell) }
pub fn can_group_member(class: FormClass, policy: GroupPolicy) -> bool { can_group_member_of(&capabilities(class), policy) }
#[cfg(test)]
pub fn forms_own_group(class: FormClass, policy: SpecialPolicy) -> bool { forms_own_group_of(&capabilities(class), policy) }

/// The class a bare kind denotes: its native starting form (the pair for `0x4b`).
fn kind_class(kind: u32) -> FormClass { form_class(kind, Forms::starting(kind)) }

/// The class of a kind as a single-body bank member: its [`member_forms`] (Solo for `0x4b`: the pair
/// is two bodies and never a member, `CellRule::PairLead`; every other kind its starting form).
fn single_member_class(kind: u32) -> FormClass { form_class(kind, member_forms(kind)) }

// ----- the `body_bank` predicates, same names and signatures ----------------------------------

/// A kind with a single-body bank member form (Ordinary, Kirby, R.O.B., and `0x4b` as Solo Popo):
/// a window member at any rotation position, laid out by its row's cell rule. Canonical groups and
/// exact Miis are other rows; what stays unqualified for Kirby is parking a copy, gated at
/// departure by [`kirby_parkable`]. An upcoming `0x4b` reaches the planner only when that slot's
/// configured form is Solo (the adapter resolves it with `requested_forms_at`).
pub fn bankable_kind(kind: u32) -> bool { single_body_member(single_member_class(kind)) }

/// A kind that may occupy any cell of an extra group: [`bankable_kind`] minus both Ice Climbers,
/// `0x4e..=0x50` and `0x6e..=0x75` (GROUP-SWAP-FEASIBILITY §6.1; the ranges are not roster kinds:
/// defence in depth). R.O.B. is admitted (EXCLUSIONS-AUDIT §1: the costume selector only *reads*
/// `R+0x58`/`R+0x64`; visual caveat disclosed in `docs/cycle-preload.md`). Solo Popo joins groups
/// through its own switch ([`group_member_kind`]).
pub fn swappable_kind(kind: u32) -> bool {
    bankable_kind(kind) && !matches!(kind, 0x4b..=0x4c | 0x4e..=0x50 | 0x6e..=0x75)
}

/// The role's slot-0 original may be an ordinary bank/group member (EXCLUSIONS §9, `original_bank`
/// switch): the row's `original_bankable` over the swappable set. Ice Climbers, Miis and the native
/// groups keep the canonical original Reload here; the pair original's warm return is
/// [`original_form_bankable`] (rc.6).
pub fn original_bankable(kind: u32) -> bool {
    swappable_kind(kind) && capabilities(kind_class(kind)).original_bankable
}

/// The role's slot-0 original in its requested form `forms` may be a bank/group member, so its return is a warm switch:
/// an ordinary original ([`original_bankable`]), the normal Ice Climbers pair while the pair bank is on (`pair_bank`),
/// or a native group (Pokémon Trainer, Pyra/Mythra) while native groups are on (`native_bank`; rc.6, a session that
/// starts on one). Those rows' `original_bankable` cite the operator decisions; the return is served by the window
/// or by a group (`group_serves`) and never by a single-body cell, which is why they stay out of
/// [`original_bankable`]. Solo Popo and the Miis keep the canonical original Reload.
pub fn original_form_bankable(kind: u32, forms: Forms, pair_bank: bool, native_bank: bool) -> bool {
    match form_class(kind, forms) {
        FormClass::IcPair => forms == Forms::starting(POPO) && pair_bank && capabilities(FormClass::IcPair).original_bankable,
        class @ (FormClass::PokemonTrainer | FormClass::Aegis) => native_bank && capabilities(class).original_bankable,
        _ => original_bankable(kind),
    }
}

/// A kind the planner may choose as a member of a new extra group under `policy`: every
/// [`swappable_kind`], plus Solo Popo when `solo_popo_groups` is on (its row's [`GroupSwitch`]; the
/// adapter admits a `0x4b` lane entry only when that slot's configured form is Solo and the role's
/// original is not IC).
pub fn group_member_kind(kind: u32, policy: GroupPolicy) -> bool { can_group_member(single_member_class(kind), policy) }

/// A lane kind that forms its own group under `policy` (PT/Aegis with `native`, the pair with `pair`).
#[cfg(test)]
pub fn special_group_kind(kind: u32, policy: SpecialPolicy) -> bool { forms_own_group(kind_class(kind), policy) }

/// Ordinary single-body kind allowed in cell 2 of a pair-lead bank. Nana is not a catalog kind,
/// the pair already occupies cell 0 (so Solo Popo is not an extra either), and a kind pinned to
/// cell 0 (Kirby) cannot sit in cell 2 of an entry whose cell 0 is Popo (`CellRule::PinCell0`): it
/// waits for the next window.
pub fn pair_lead_extra(kind: u32) -> bool { bankable_kind(kind) && kind != POPO && kind != NANA && !pins_cell0(kind) }

/// Logical form of an already qualified bank member. Popo in a bank is always solo: the normal
/// pair needs two active bodies and two partner controllers (`bodies: 2, actors: 2`). This does not
/// qualify an arbitrary requested Ice Climbers configuration.
pub fn member_forms(kind: u32) -> Forms {
    if kind_class(kind) == FormClass::IcPair { Forms::solo_popo() } else { Forms::starting(kind) }
}

/// Logical form of `kind` inside the physical window described by these descriptor words: Popo of
/// a pair-lead window (`[Popo, Nana, X]`, two actors) is the normal pair; every other member keeps
/// [`member_forms`].
pub fn member_forms_in(count: u32, kinds: [u32; 3], actor_count: u32, kind: u32) -> Forms {
    let pair_lead = count == NATIVE_MEMBERS as u32 && kinds[0] == POPO && kinds[1] == NANA && actor_count == 2;
    if pair_lead && kind == POPO { Forms::starting(POPO) } else { member_forms(kind) }
}

/// The canonical form the descriptor words `(count, kinds, actors)` spell: the native expansion of
/// `kinds[0]` (`Forms::starting`) or Solo Popo; `None` when they spell neither. Flags are not part
/// of the words.
pub fn request_forms(count: u32, kinds: [u32; 3], actors: u32) -> Option<Forms> {
    let candidates = [Forms::starting(kinds[0]), Forms::solo_popo()];
    candidates.iter().copied().find(|f| count == f.count && kinds == f.kinds && actors == f.actor_count)
}

/// The normal Ice Climbers pair request: count 2, Popo then Nana, two actors.
pub fn pair_request(count: u32, kinds: [u32; 3], actors: u32) -> bool {
    request_forms(count, kinds, actors) == Some(Forms::starting(POPO))
}

/// A canonical native-group request (count, kinds, actors as `Forms::starting` expands them) for
/// a form completed by the native script (PT, Aegis): a target an existing or prefetched native
/// group can serve. Window banks never adopt it.
pub fn native_group_request(count: u32, kinds: [u32; 3], actors: u32) -> Option<Forms> {
    request_forms(count, kinds, actors)
        .filter(|forms| capabilities(form_class(kinds[0], *forms)).completion == Completion::NativeScript)
}

/// A request must describe the single body itself, not merely share its kind: the words spell
/// exactly [`member_forms`] of the kind, and that form is a single-body member (an ordinary single
/// body or Solo Popo). In particular, a normal-pair request never reuses a solo Popo bank.
pub fn single_member_request(count: u32, kinds: [u32; 3], actors: u32) -> Option<Forms> {
    let forms = request_forms(count, kinds, actors)?;
    (forms == member_forms(kinds[0]) && single_body_member(form_class(kinds[0], forms))).then_some(forms)
}

/// `ParkGate::CopyFree`: the copy-free departure gate for a live Kirby about to be parked
/// (in-window member change or group swap). Inputs are values the adapter reads from the outgoing
/// body in the current callback: the `FLAG_COPY` work flag, the retained copy object groups
/// (`None`: unreadable layout) and whether a hat lease names that body. A parked copy-source lease
/// is unqualified (resource-audit §2.2), so any copy state refuses and the request takes the
/// canonical Reload. A hatless Kirby parks and wakes like any ordinary member; a Reloaded Kirby is
/// hatless, so the parked one is never behind the Reload it replaces.
pub fn copy_free_departure(copy_flag: bool, retained_copy_kinds: Option<u128>, hat_lease_on_body: bool)
    -> Result<(), &'static str> {
    if copy_flag {
        return Err("Kirby holds a copy ability");
    }
    match retained_copy_kinds {
        None => return Err("Kirby copy resources are not readable"),
        Some(0) => {}
        Some(_) => return Err("Kirby still retains copy resources"),
    }
    if hat_lease_on_body {
        return Err("a hat lease names this Kirby");
    }
    Ok(())
}

/// The row's `park_gate` applied to a live member about to be parked. `PairAlive` is a native read
/// (`pair_parkable` in the adapter) and passes here.
pub fn departure_admitted(class: FormClass, copy_flag: bool, retained_copy_kinds: Option<u128>,
    hat_lease_on_body: bool) -> Result<(), &'static str> {
    match capabilities(class).park_gate {
        ParkGate::CopyFree => copy_free_departure(copy_flag, retained_copy_kinds, hat_lease_on_body),
        ParkGate::None | ParkGate::PairAlive => Ok(()),
    }
}

/// [`departure_admitted`] for the Kirby row (the adapter's `kirby_park_gate` decision).
pub fn kirby_parkable(copy_flag: bool, retained_copy_kinds: Option<u128>, hat_lease_on_body: bool)
    -> Result<(), &'static str> {
    departure_admitted(FormClass::Kirby, copy_flag, retained_copy_kinds, hat_lease_on_body)
}

/// D-KIRBY1 (D-KIRBY1-ROB1-RCA §3): while a role's live member is of a row with
/// `prefetch_while_live: false` (Kirby), its resource owner holds no bank prefetch key. A Kirby copy
/// registers the copied kind on Kirby's own R (`[KirbyProbe] request owner=R kind=…`) and the
/// per-kind release `0x17e8610(R, k)` also drops R's G68 mirror for `(k, costume)`, so a prefetch
/// key and a live copy of the same kind on one R let the key's release corrupt the registry the
/// following Reload walks (Ally S2: endless `Read32@0x10`/`Read64@0x8` in `0x17f0cc0`/`0x17f165c`).
/// Planned groups wait, Prefetched groups hand their keys back; Built groups hold no keys.
/// `live_kind` is the window's ready member, `None` if unknown. The reason text is read by the
/// E4a row (`prefetch deferred … reason=live Kirby may copy a group kind (D-KIRBY1)`).
/// rc.7: the runtime passes [`d_kirby1_live`], so once the bank owns the G68 step (F1) a live Kirby defers nothing.
pub fn prefetch_admitted(live_kind: Option<u32>) -> Result<(), &'static str> {
    match live_kind {
        Some(kind) if !capabilities(kind_class(kind)).prefetch_while_live => Err("live Kirby may copy a group kind"),
        _ => Ok(()),
    }
}

/// rc.7 F1 (the D-KIRBY1 root fix): whether D-KIRBY1's live-Kirby rules are in force. A row that suspends prefetch while
/// live (Kirby) restricts the bank only while the G68 step of the bank's releases is native's: once the bank owns it
/// (`g68_owned`, `runtime::roster::g68_step`; `request_bit::Policy::Owned`) no release of the bank's keys reaches a copy's
/// package, and the rules have nothing left to guard.
pub fn d_kirby1_applies(row_suspends: bool, g68_owned: bool) -> bool {
    row_suspends && !g68_owned
}

/// The live kind D-KIRBY1's gates ([`prefetch_admitted`], [`object_rearm_admitted`], the background build's
/// `live_kirby`) see: the live kind, or `None` once the bank owns the G68 step ([`d_kirby1_applies`]).
pub fn d_kirby1_live(live_kind: Option<u32>, g68_owned: bool) -> Option<u32> {
    live_kind.filter(|_| !g68_owned)
}

/// A detached Pokémon Trainer group's object re-arm (`object_rearm_due`: its object kind taken again before its next
/// attach) under the role's live member. D-KIRBY1 ([`prefetch_admitted`]) is about the keys a live Kirby copy can share
/// on its R, and a copy registers a fighter kind there (`[KirbyProbe] request owner=R kind=…`; a copy of a Pokémon takes
/// the Pokémon, `0x24..=0x26`). The object kinds the bank holds keys for (the trainer's `PT_OBJECT_KIND`, the Aegis
/// partner's `AEGIS_OBJECT_KIND`) are no fighter a copy takes, so their re-arm runs under a live Kirby too, through the
/// same ledger path (primary 2026-11-09 05:38Z: Kirby > PT warm again). Any other kind waits as a prefetch does:
/// D-KIRBY1 stays for every fighter key.
pub fn object_rearm_admitted(live_kind: Option<u32>, kind: u32) -> Result<(), &'static str> {
    if bank_object_kind(kind) { Ok(()) } else { prefetch_admitted(live_kind) }
}

/// The object kinds the bank holds keys for (no fighter kind, so no Kirby copy registers one on R).
pub fn bank_object_kind(kind: u32) -> bool {
    matches!(kind, PT_OBJECT_KIND | AEGIS_OBJECT_KIND)
}

/// A prefetch key may be released unless the live Kirby copies exactly that kind
/// (`live_copy_kind`: `INT_COPY_CHARA` while `FLAG_COPY`, else `None`): the D-KIRBY1 last-line
/// guard (`Conflict::LiveCopySharesRegistry` in WP4's occupancy terms). The reason text is read by
/// the E4 rows (`key kept … reason=live Kirby copies this kind`).
#[cfg(test)]
pub fn key_release_admitted(kind: u32, live_copy_kind: Option<u32>) -> Result<(), &'static str> {
    if live_copy_kind == Some(kind) { Err("live Kirby copies this kind") } else { Ok(()) }
}

// ----- window-shaped predicates ---------------------------------------------------------------

/// The class of `kind` as a member of `window`: the window's own `forms_of` decides (the pair for
/// the Popo of a pair-lead window or a pair group, Solo for a Solo-led window's Popo).
fn member_class(window: Members, kind: u32) -> FormClass { form_class(kind, window.forms_of(kind)) }

/// The exit rule of a concrete window: a one-member window (ordinary, Solo Popo or Mii) commits to
/// its own cell, the pair group leaves through Nana, the pair-lead window through its extra, every
/// other window (ordinary, Solo-led, native group) through the next cell.
pub fn exit_rule(window: Members) -> ExitRule {
    if window.count() == 1 {
        return ExitRule::SelfCell;
    }
    match window.special() {
        Special::Pair => ExitRule::ViaNana,
        // A Mii window has one cell (handled above; the Mii row's `exit_commit` says the same).
        Special::Mii => capabilities(member_class(window, window.kinds()[0])).exit_commit,
        Special::None if window.is_pair_lead() => capabilities(FormClass::IcPair).exit_commit,
        Special::Native(_) | Special::None => capabilities(member_class(window, window.kinds()[0])).exit_commit,
    }
}

/// The next-cell commit index (`ExitRule::NextCell`): `(from + 1) % count` inside a window of two
/// or more cells. `6581f0` parks `cell[+0x5918]` when `+0x591a != 0xff`, and `64fb70` resolves the
/// context from the kind word at the selected index, storing through null for an empty padding cell
/// (GROUP-SWAP-FEASIBILITY §3); for one cell the next index would be padding, so a one-member window
/// uses its own cell instead ([`ExitRule::SelfCell`], [`swap_commit_index_in`]).
pub fn swap_commit_index(from: u8, count: usize) -> Option<u8> {
    (count >= 2 && usize::from(from) < count).then(|| ((usize::from(from) + 1) % count) as u8)
}

/// In-window commit index that parks the current body before a group swap, by the window's
/// [`exit_rule`]: a one-member window commits to its own cell 0 (self-commit); a pair-lead window
/// has two selectable members, the pair (0) and its extra (2); Nana's cell 1 is never a request
/// target (`6581f0` would park Popo alone and `64fb70` would route her context), so the commit goes
/// pair → extra (N3 parks Nana inside the commit) or extra → pair; a pair group commits through
/// Nana (1) (`64fb70` resolves her context from the lent table, the commit parks Popo, N3 parks
/// Nana, and the seam replaces the window before any activation, SPECIAL-GROUPS §5); every other
/// window uses the next cell. Every window shape can be left warm: entering, leaving and tearing
/// down a one-member window use only its one cell.
pub fn swap_commit_index_in(window: Members, from: u8) -> Option<u8> {
    match exit_rule(window) {
        ExitRule::NextCell => swap_commit_index(from, window.count()),
        ExitRule::PairExtraOnly => match from { 0 => Some(2), 2 => Some(0), _ => None },
        ExitRule::ViaNana => (from == 0).then_some(1),
        ExitRule::SelfCell => (from == 0).then_some(0),
    }
}

/// The previous-cell index (`+0x5910`) the exchange leaves for the attached group.
///
/// FACT (owned 13.0.5 ELF, `refactor/popo-oneexit/re/entry_640000_660000.asm` `0x65655c..0x656670`;
/// Atmosphère report 01793544752, the first C116.1 console boot): loader state 13's activation
/// handler for a commit (`0x655f60` case 7, `+0x5934 != 0`, `+0x5910 != 0xff`) loads `cell[+0x5910]`
/// and `cell[+0x5918]` from `+0x4160` and reads each body's id (`+0x8`) for body event `0x1e`, with
/// no count check. The commit stored the *window's* active index at `+0x5910`; after an exchange into
/// a group of `count <= from` cells that cell is null and the console faults (PC main+0x656668, Data
/// Abort at 0x8). Eden only logs `Unmapped Read32 @ 0x8` and goes on, which is how 878 such swaps
/// passed its lanes. The window's previous cell stays when the group has it; otherwise the attached
/// member's own cell stands in, the self-commit shape ([`ExitRule::SelfCell`]: old id == new id,
/// exercised by every one-member window and every `from == member` swap).
pub fn previous_cell_index(from: u8, member: u8, count: usize) -> u8 {
    if usize::from(from) < count { from } else if usize::from(member) < count { member } else { 0 }
}

/// A window group may own extra groups when every member may occupy its cell (EXCLUSIONS-AUDIT
/// §4a: a detached group is sync-built with the entry cells equal to its own ordinary kinds, so the
/// `0x17e8410` secondary-cell skip and the `R+0x50` pair rule never see Popo there, and the Solo
/// window's `R+0x50 = 2` has no consumer outside that rule); a pair-lead bank never participates
/// under the default policy: see [`window_swappable_with`].
#[cfg(test)]
pub fn window_swappable(window: Members) -> bool { window_swappable_with(window, GroupPolicy::DEFAULT) }

/// [`window_swappable`] under `policy`: a native group or a pair group in the window is a complete
/// native window for the commit and may own (and be exchanged with) groups; with `ic_pair_groups`
/// a pair-lead window owns groups when its extra cell is an ordinary single body (§4c): the window
/// is left only through [`swap_commit_index_in`] (pair ↔ cell 2, never Nana's cell), so the pair is
/// parked by the commit's own N3 sequence before the exchange, and the warm return re-activates it
/// through N2 as for an in-window return; an ordinary or Solo-led window owns groups when every
/// cell holds a member its row admits there (`can_window_member`).
pub fn window_swappable_with(window: Members, policy: GroupPolicy) -> bool {
    match window.special() {
        Special::Native(_) | Special::Pair => true,
        // M0 lifted (primary inbox 2026-11-09 06:16Z; hot rc6-hot-xps26: a cycle change while P1 sat on
        // the Gunner planned nothing until P1 left it). A Mii window plans its role's groups like any
        // one-member window (`ExitRule::SelfCell` leaves it warm). Beside the live Mii, a group's keys
        // and build take the owner's ordinary words, not the Mii's header (X6). A second Mii group
        // stays out while a Mii is the window (`plan_extra_groups_with`: one Mii record per owner).
        Special::Mii => true,
        Special::None if window.is_pair_lead() => {
            policy.ic_pair_groups && single_body_member(member_class(window, window.kinds()[2]))
        }
        Special::None => (0..window.count()).all(|cell| can_window_member(member_class(window, window.kinds()[cell]), cell)),
    }
}
